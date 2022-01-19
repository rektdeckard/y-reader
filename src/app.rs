use eframe::{egui, epi};
use hacker_news::model::firebase::Comment;
use hacker_news::{client::json_client::JsonClient, model::firebase::Item, model::firebase::Story};
use html_escape;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use time_humanize::HumanTime;
use url::Url;

const REFETCH_DELAY_SECONDS: u64 = 60;
const WINDOW: usize = 50;

struct Auth {
    username: String,
    password: String,
}

#[derive(PartialEq)]
enum Tab {
    Top,
    New,
    Show,
}

#[derive(Debug)]
struct Data {
    top: HashMap<usize, Item>,
    top_ids: Vec<u32>,
    top_page: usize,
    new: HashMap<usize, Item>,
    new_ids: Vec<u32>,
    new_page: usize,
    show: HashMap<usize, Item>,
    show_ids: Vec<u32>,
    show_page: usize,
    comments: HashMap<u32, CommentState>,
}

impl Data {
    fn new() -> Self {
        Self {
            top: HashMap::new(),
            top_ids: Vec::new(),
            top_page: 0,
            new: HashMap::new(),
            new_ids: Vec::new(),
            new_page: 0,
            show: HashMap::new(),
            show_ids: Vec::new(),
            show_page: 0,
            comments: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
enum CommentState {
    Loading,
    Loaded(LocalComment),
    Errored,
}

struct LocalStory {
    id: hacker_news::model::Id,
    by: Option<String>,
    time: u64,
    kids: Option<Vec<hacker_news::model::Id>>,
    score: Option<hacker_news::model::Score>,
    title: Option<String>,
    url: Option<String>,
}

impl LocalStory {
    fn from_lib(story: &Story) -> Self {
        Self {
            id: story.id.clone(),
            by: story.by.clone(),
            time: story.time,
            kids: story.kids.clone(),
            score: story.score.clone(),
            title: story.title.clone(),
            url: story.url.clone(),
        }
    }
}

#[derive(Debug, Clone)]
struct LocalComment {
    id: hacker_news::model::Id,
    by: Option<String>,
    time: u64,
    kids: Option<Vec<hacker_news::model::Id>>,
    // parent: Option<hacker_news::model::Id>,
    text: Option<String>,
}

impl LocalComment {
    fn from_lib(comment: &Comment) -> Self {
        Self {
            id: comment.id.clone(),
            by: comment.by.clone(),
            time: comment.time,
            kids: comment.kids.clone(),
            // parent: comment.parent.clone(),
            text: comment.text.clone(),
        }
    }
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
pub struct YReader {
    // this how you opt-out of serialization of a member
    auth: Option<Auth>,
    authed: bool,
    show_login: bool,
    show_settings: bool,
    #[cfg_attr(feature = "persistence", serde(skip))]
    tab: Tab,
    data: Arc<Mutex<Data>>,
}

impl YReader {
    // fn fetch_top(&self) {
    //     let data_top = Arc::clone(&self.data);

    //     thread::spawn(move || {
    //         let client = JsonClient::new();
    //         let ids = client.top_stories();
    //         if let Ok(ids) = ids {
    //             let page;
    //             {
    //                 let data = data_top.lock().unwrap();
    //                 page = data.top_page;
    //             }
    //             for (idx, id) in ids.iter().take(WINDOW * (page + 1)).enumerate() {
    //                 if let Ok(item) = client.item(*id) {
    //                     let mut data = data_top.lock().unwrap();
    //                     data.top.insert(idx, item);
    //                 }
    //             }
    //             let mut data = data_top.lock().unwrap();
    //             data.top_ids = ids;
    //             // data.top_page = (data.top_page + 1) % 4;
    //         }
    //         println!("Fetched top");
    //     });
    // }

    // fn fetch_new(&self) {
    //     let data_new = Arc::clone(&self.data);

    //     thread::spawn(move || {
    //         let client = JsonClient::new();
    //         let ids = client.new_stories();
    //         if let Ok(ids) = ids {
    //             let page;
    //             {
    //                 let data = data_new.lock().unwrap();
    //                 page = data.new_page;
    //             }
    //             for (idx, id) in ids.iter().take(WINDOW * (page + 1)).enumerate() {
    //                 if let Ok(item) = client.item(*id) {
    //                     let mut data = data_new.lock().unwrap();
    //                     data.new.insert(idx, item);
    //                 }
    //             }
    //             let mut data = data_new.lock().unwrap();
    //             data.new_ids = ids;
    //             // data.new_page = (data.new_page + 1) % 4;
    //         }
    //         println!("Fetched new");
    //     });
    // }

    fn init(&self) {
        let data_top = Arc::clone(&self.data);
        thread::spawn(move || loop {
            let client = JsonClient::new();
            let ids = client.top_stories();
            if let Ok(ids) = ids {
                let page;
                {
                    let data = data_top.lock().unwrap();
                    page = data.top_page;
                }
                for (idx, id) in ids.iter().take(WINDOW * (page + 1)).enumerate() {
                    if let Ok(item) = client.item(*id) {
                        let mut data = data_top.lock().unwrap();
                        data.top.insert(idx, item);
                    }
                }
                let mut data = data_top.lock().unwrap();
                data.top_ids = ids;
                data.top_page = (data.top_page + 1) % 2;
            }
            thread::sleep(Duration::from_secs(REFETCH_DELAY_SECONDS));
        });

        let data_new = Arc::clone(&self.data);
        thread::spawn(move || loop {
            let client = JsonClient::new();
            let ids = client.new_stories();
            if let Ok(ids) = ids {
                let page;
                {
                    let data = data_new.lock().unwrap();
                    page = data.new_page;
                }
                for (idx, id) in ids.iter().take(WINDOW * (page + 1)).enumerate() {
                    if let Ok(item) = client.item(*id) {
                        let mut data = data_new.lock().unwrap();
                        data.new.insert(idx, item);
                    }
                }
                let mut data = data_new.lock().unwrap();
                data.new_ids = ids;
                data.new_page = (data.new_page + 1) % 2;
            }
            thread::sleep(Duration::from_secs(REFETCH_DELAY_SECONDS));
        });

        let data_show = Arc::clone(&self.data);
        thread::spawn(move || loop {
            let client = JsonClient::new();
            let ids = client.show_stories();
            if let Ok(ids) = ids {
                let page;
                {
                    let data = data_show.lock().unwrap();
                    page = data.show_page;
                }
                for (idx, id) in ids.iter().take(WINDOW * (page + 1)).enumerate() {
                    if let Ok(item) = client.item(*id) {
                        let mut data = data_show.lock().unwrap();
                        data.show.insert(idx, item);
                    }
                }
                let mut data = data_show.lock().unwrap();
                data.show_ids = ids;
                data.show_page = (data.show_page + 1) % 2;
            }
            thread::sleep(Duration::from_secs(REFETCH_DELAY_SECONDS));
        });
    }

    fn render_stories(&mut self, ui: &mut egui::Ui) {
        egui::containers::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let current = self.data.lock().unwrap();
                let items = match self.tab {
                    Tab::Top => &current.top,
                    Tab::New => &current.new,
                    Tab::Show => &current.show,
                };

                let mut stories: Vec<(usize, LocalStory)> = items
                    .iter()
                    .filter_map(|(idx, i)| match i {
                        Item::Story(s) => Some((*idx as usize, LocalStory::from_lib(s))),
                        _ => None,
                    })
                    .collect();
                stories.sort_by(|(a, _), (b, _)| a.cmp(b));
                std::mem::drop(current);

                stories.iter().for_each(|(idx, s)| {
                    if let Some(title) = &s.title {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(egui::RichText::new(title).strong());
                            if let Some(url) = &s.url {
                                if let Ok(u) = Url::parse(url) {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 0.0;
                                        ui.label("(");
                                        ui.hyperlink_to(
                                            u.domain()
                                                .and_then(|s| Some(s.to_string()))
                                                .unwrap_or(url.clone()),
                                            u.to_string(),
                                        );
                                        ui.label(")");
                                    });
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.5;
                            ui.label(format!("{} points", &s.score.unwrap_or(0)));
                            if let Some(by) = &s.by {
                                ui.add(egui::widgets::Separator::default().vertical());
                                ui.hyperlink_to(
                                    by,
                                    format!("https://news.ycombinator.com/user?id={}", by),
                                );
                            }

                            ui.add(egui::widgets::Separator::default().vertical());

                            let now = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .expect("oops")
                                .as_secs();

                            ui.label(format!(
                                "{}",
                                HumanTime::from_seconds((s.time as i64) - (now as i64))
                            ));
                        });

                        egui::containers::CollapsingHeader::new(format!(
                            "{} Comments",
                            &s.kids
                                .as_ref()
                                .and_then(|k| Some(k.len()))
                                .unwrap_or_default()
                        ))
                        .id_source(format!("{}-{}", idx, s.id))
                        .show(ui, |ui| {
                            if let Some(kids) = &s.kids {
                                self.render_comments(ui, kids);
                            }
                        });

                        ui.separator();
                    }
                });

                // let margin = ui.visuals().clip_rect_margin;
                // let current_scroll = ui.clip_rect().top() - ui.min_rect().top() + margin;
                // let max_scroll = ui.min_rect().height() - ui.clip_rect().height() + 2.0 * margin;

                // if current_scroll == max_scroll {
                //     println!("yuip");
                //     match self.tab {
                //         Tab::Top => {
                //             self.fetch_top();
                //             // DEADLOCK!
                //             let mut data = self.data.lock().unwrap();
                //             data.top_page = (data.top_page + 1) % 4;
                //         }
                //         Tab::New => {
                //             self.fetch_new();
                //             let mut data = self.data.lock().unwrap();
                //             data.new_page = (data.new_page + 1) % 4;
                //         }
                //         Tab::Show => {}
                //     };
                // }

                // ui.vertical_centered(|ui| {
                //     let (page, count) = match self.tab {
                //         Tab::Top => (self.top_page, self.top_ids.len()),
                //         Tab::New => (self.new_page, self.new_ids.len()),
                //         Tab::Show => (self.show_page, self.show_ids.len()),
                //     };
                //     if count > 0 && WINDOW * page >= count {
                //         ui.label("All done!");
                //     } else if ui.button("Load more").clicked() {
                //         self.fetch_stories();
                //     }
                // });
            })
    }

    fn render_comments(&self, ui: &mut egui::Ui, kids: &Vec<u32>) {
        let data = Arc::clone(&self.data);
        lazy_static! {
            static ref RE: Regex =
                Regex::new(r#"<a\s+href=(?:"([^"]+)"|'([^']+)').*?>(.*?)</a>"#).unwrap();
        }

        for k in kids {
            let comment: Option<CommentState>;
            {
                let data = data.lock().unwrap();
                comment = data.comments.get(k).and_then(|c| Some(c.to_owned()));
            }

            match comment {
                Some(CommentState::Loading) => {
                    ui.label("Loading...");
                }
                Some(CommentState::Loaded(c)) => {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.5;
                        if let Some(by) = &c.by {
                            ui.hyperlink_to(
                                by,
                                format!("https://news.ycombinator.com/user?id={}", by),
                            );
                        }

                        ui.add(egui::widgets::Separator::default().vertical());

                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .expect("oops")
                            .as_secs();

                        ui.label(format!(
                            "{}",
                            HumanTime::from_seconds((c.time as i64) - (now as i64))
                        ));
                    });

                    let text = c.text.to_owned().unwrap_or_default();
                    let decoded = html_escape::decode_html_entities(&text).to_string();

                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.y = 10.;
                        decoded.split("<p>").into_iter().for_each(|part| {
                            if RE.is_match(part) {
                                for piece in RE.captures_iter(part) {
                                    // TODO: include non-link text from this line
                                    if let Some(url) = &piece.get(1) {
                                        if let Some(label) = &piece.get(3) {
                                            ui.hyperlink_to(label.as_str(), url.as_str());
                                        } else {
                                            ui.hyperlink(url.as_str());
                                        }
                                    }
                                }
                            } else {
                                ui.label(part);
                            }

                            ui.end_row();
                        });
                    });

                    if let Some(kids) = &c.kids {
                        egui::containers::CollapsingHeader::new(format!("{} Replies", kids.len()))
                            .id_source(c.id)
                            .show(ui, |ui| {
                                self.render_comments(ui, &kids);
                            });
                    }
                    ui.separator();
                }
                Some(CommentState::Errored) => {
                    ui.label("Errored.");
                }
                _ => {
                    ui.label("Starting load...");
                    let data = Arc::clone(&self.data);
                    let id = k.clone();

                    {
                        let mut data = data.lock().unwrap();
                        data.comments.insert(id, CommentState::Loading);
                    }

                    thread::spawn(move || {
                        let client = JsonClient::new();
                        let comment = client.item(id);

                        let mut data = data.lock().unwrap();
                        match comment {
                            Ok(Item::Comment(c)) => {
                                data.comments
                                    .insert(id, CommentState::Loaded(LocalComment::from_lib(&c)));
                            }
                            _ => {
                                data.comments.insert(id, CommentState::Errored);
                            }
                        }
                    });
                }
            }
        }
    }
}

impl Default for YReader {
    fn default() -> Self {
        Self {
            auth: None,
            authed: false,
            show_login: false,
            show_settings: false,
            tab: Tab::Top,
            data: Arc::new(Mutex::new(Data::new())),
        }
    }
}

impl epi::App for YReader {
    fn name(&self) -> &str {
        "Y Reader"
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        _ctx: &egui::CtxRef,
        _frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        #[cfg(feature = "persistence")]
        if let Some(storage) = _storage {
            *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }

        self.init();
    }

    /// Called by the frame work to save state before shutdown.
    /// Note that you must enable the `persistence` feature for this to work.
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::CtxRef, _frame: &epi::Frame) {
        let Self {
            auth,
            authed,
            show_login,
            show_settings,
            tab,
            data: _,
        } = self;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("Y Reader");

                ui.add(egui::widgets::Separator::default().vertical());
                ui.selectable_value(tab, Tab::Top, "Top");
                ui.selectable_value(tab, Tab::New, "New");
                ui.selectable_value(tab, Tab::Show, "Show");
                egui::widgets::global_dark_light_mode_switch(ui);

                ui.with_layout(egui::Layout::right_to_left(), |ui| {
                    if ui.button("Settings").clicked() {
                        *show_settings = true;
                    }

                    egui::Window::new("Settings")
                        .open(show_settings)
                        .vscroll(true)
                        .show(ctx, |ui| {
                            ctx.settings_ui(ui);
                        });

                    if auth.is_none() || !*authed {
                        if ui.button("Sign in").clicked() {
                            *show_login = true;
                        }
                    } else {
                        if let Some(a) = auth {
                            let username = a.username.clone();
                            if ui.button("Sign out").clicked() {
                                *auth = None;
                                *authed = false;
                            }
                            ui.hyperlink_to(
                                username.as_str(),
                                format!("https://news.ycombinator.com/user?id={}", username),
                            );
                        }
                    }
                });

                if *show_login {
                    egui::Window::new("Sign in to Hacker News")
                        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::default())
                        .auto_sized()
                        .title_bar(false)
                        .show(ctx, |ui| {
                            if auth.is_none() {
                                *auth = Some(Auth {
                                    username: Default::default(),
                                    password: Default::default(),
                                })
                            }

                            let Auth { username, password } = auth.as_mut().unwrap();

                            ui.add(egui::TextEdit::singleline(username).hint_text("Username"));
                            ui.add(
                                egui::TextEdit::singleline(password)
                                    .password(true)
                                    .hint_text("Password"),
                            );
                            ui.horizontal(|ui| {
                                if ui
                                    .add_enabled(
                                        !username.is_empty() && !password.is_empty(),
                                        egui::Button::new("Sign in"),
                                    )
                                    .clicked()
                                {
                                    *authed = true;
                                    *show_login = false;
                                }
                                if ui.button("Cancel").clicked() {
                                    *show_login = false;
                                }
                            });
                        });
                }
            });
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                let data = &self.data.lock().unwrap();
                let count = match self.tab {
                    Tab::Top => data.top.len(),
                    Tab::New => data.new.len(),
                    Tab::Show => data.show.len(),
                };
                ui.small(format!("{} items", count));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            egui::warn_if_debug_build(ui);
            self.render_stories(ui);
        });
    }
}
