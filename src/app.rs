use eframe::{egui, epi};
use hacker_news::{client::json_client::JsonClient, model::firebase::Item, model::firebase::Story};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use time_humanize::HumanTime;
use url::Url;

const WINDOW: usize = 25;

struct Auth {
    pub username: String,
    pub password: String,
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
    client: JsonClient,
    tab: Tab,
    top: HashMap<usize, Item>,
    top_ids: Vec<u32>,
    top_page: usize,
    new: HashMap<usize, Item>,
    new_ids: Vec<u32>,
    new_page: usize,
    show: HashMap<usize, Item>,
    show_ids: Vec<u32>,
    show_page: usize,
}

impl YReader {
    fn fetch_stories(&mut self) {
        match self.tab {
            Tab::Top => {
                let ids = self.client.top_stories();
                if let Ok(ids) = ids {
                    for (idx, id) in ids
                        .iter()
                        .skip(WINDOW * self.top_page)
                        .take(WINDOW)
                        .enumerate()
                    {
                        match self.client.item(*id) {
                            Ok(item) => {
                                self.top.insert(idx + (WINDOW * self.top_page), item);
                            }
                            _ => {}
                        }
                    }
                    self.top_ids = ids;
                    self.top_page += 1;
                };
            }
            Tab::New => {
                let ids = self.client.new_stories();
                if let Ok(ids) = ids {
                    for (idx, id) in ids
                        .iter()
                        .skip(WINDOW * self.new_page)
                        .take(WINDOW)
                        .enumerate()
                    {
                        match self.client.item(*id) {
                            Ok(item) => {
                                self.new.insert(idx + (WINDOW * self.new_page), item);
                            }
                            _ => {}
                        }
                    }
                    self.new_ids = ids;
                    self.new_page += 1;
                };
            }
            Tab::Show => {
                let ids = self.client.show_stories();
                if let Ok(ids) = ids {
                    for (idx, id) in ids
                        .iter()
                        .skip(WINDOW * self.show_page)
                        .take(WINDOW)
                        .enumerate()
                    {
                        match self.client.item(*id) {
                            Ok(item) => {
                                self.show.insert(idx + (WINDOW * self.show_page), item);
                            }
                            _ => {}
                        }
                    }
                    self.show_ids = ids;
                    self.show_page += 1;
                };
            }
        }
    }

    fn render_stories(&mut self, ui: &mut egui::Ui) {
        egui::containers::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let items = match self.tab {
                    Tab::Top => &self.top,
                    Tab::New => &self.new,
                    Tab::Show => &self.show,
                };

                let mut stories: Vec<(usize, &Story)> = items
                    .iter()
                    .filter_map(|(idx, i)| match i {
                        Item::Story(s) => Some((*idx as usize, s)),
                        _ => None,
                    })
                    .collect();
                stories.sort_by(|(a, _), (b, _)| a.cmp(b));

                stories.iter().for_each(|&(idx, s)| {
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
                            ui.label(format!("{:?}", s));
                        });

                        ui.separator();
                    }
                });
                ui.vertical_centered(|ui| {
                    let (page, count) = match self.tab {
                        Tab::Top => (self.top_page, self.top_ids.len()),
                        Tab::New => (self.new_page, self.new_ids.len()),
                        Tab::Show => (self.show_page, self.show_ids.len()),
                    };
                    if count > 0 && WINDOW * page >= count {
                        ui.label("All done!");
                    } else if ui.button("Load more").clicked() {
                        self.fetch_stories();
                    }
                });
            })
    }
}

impl Default for YReader {
    fn default() -> Self {
        Self {
            auth: None,
            authed: false,
            show_login: false,
            show_settings: false,
            client: JsonClient::new(),
            tab: Tab::Top,
            top: HashMap::new(),
            top_ids: Vec::new(),
            top_page: 0,
            new: HashMap::new(),
            new_ids: Vec::new(),
            new_page: 0,
            show: HashMap::new(),
            show_ids: Vec::new(),
            show_page: 0,
        }
    }
}

impl epi::App for YReader {
    fn name(&self) -> &str {
        "newsY"
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

        self.fetch_stories();
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
            client: _,
            tab,
            top: _,
            top_ids: _,
            top_page: _,
            new: _,
            new_ids: _,
            new_page: _,
            show: _,
            show_ids: _,
            show_page: _,
        } = self;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("Hacker News");

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
                let count = match self.tab {
                    Tab::Top => self.top.len(),
                    Tab::New => self.new.len(),
                    Tab::Show => self.show.len(),
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

#[derive(PartialEq)]
enum Tab {
    Top,
    New,
    Show,
}
