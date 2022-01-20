#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use eframe::egui::Vec2;

    let icon = image::open("assets/y_icon.png")
        .expect("Could not load icon")
        .to_rgba8();
    let (icon_width, icon_height) = icon.dimensions();

    let app = y_reader::YReader::default();
    let mut native_options = eframe::NativeOptions::default();
    native_options.initial_window_size = Some(Vec2::new(540., 960.));
    // native_options.icon_data
    native_options.icon_data = Some(eframe::epi::IconData {
        width: icon_width,
        height: icon_height,
        rgba: icon.into_raw(),
    });
    eframe::run_native(Box::new(app), native_options);
}
