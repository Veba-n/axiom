pub mod core;
pub mod data;
pub mod render;
pub mod ui;
pub mod app;

use crate::app::AxiomStudio;
use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    // Linux üzerinde Wayland kaynaklı fare (pointer) çökmesini engellemek için X11'i zorluyoruz
    std::env::set_var("WINIT_UNIX_BACKEND", "x11");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 900.0])
            .with_title("Axiom UI Designer - Advanced Text Rendering Engine"),
        vsync: false,
        ..Default::default()
    };
    eframe::run_native(
        "Axiom Studio",
        options,
        Box::new(|cc| {
            let mut fonts = egui::FontDefinitions::default();
            if let Ok(font_data) = std::fs::read("FiraCode.ttf") {
                fonts.font_data.insert(
                    "FiraCode".to_owned(),
                    egui::FontData::from_owned(font_data),
                );
                fonts.families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "FiraCode".to_owned());
            }
            cc.egui_ctx.set_fonts(fonts);
            
            Box::<AxiomStudio>::default()
        }),
    )
}