use eframe::egui;
use crate::app::AxiomStudio;

pub fn show(app: &mut AxiomStudio, ctx: &egui::Context) {
    let mut is_open = app.windowed_panels.contains("SettingsModal");
    if !is_open { return; }

    egui::Window::new("⚙ Genel Proje Ayarları")
        .collapsible(false)
        .resizable(true)
        .default_size(egui::vec2(400.0, 300.0))
        .open(&mut is_open)
        .show(ctx, |ui| {
            ui.heading("Ekran & Çözünürlük");
            ui.horizontal(|ui| {
                ui.label("Genişlik (W):");
                ui.add(egui::DragValue::new(&mut app.settings.resolution_w).speed(10.0));
                ui.label("Yükseklik (H):");
                ui.add(egui::DragValue::new(&mut app.settings.resolution_h).speed(10.0));
            });
            
            ui.separator();
            ui.heading("Kanvas (Tasarım Alanı)");
            ui.horizontal(|ui| {
                ui.label("Arka Plan Rengi:");
                ui.color_edit_button_srgb(&mut app.settings.canvas_bg_color);
            });
            ui.checkbox(&mut app.settings.show_grid_lines, "Hizalama Izgarasını (Grid) Göster");
            ui.horizontal(|ui| {
                ui.label("Hizalama (Snap) Boşluğu:");
                ui.add(egui::Slider::new(&mut app.settings.grid_snap, 1.0..=50.0).text("%"));
            });

            ui.separator();
            ui.heading("Varsayılan Değerler");
            ui.horizontal(|ui| {
                ui.label("Varsayılan Font Boyutu:");
                ui.add(egui::Slider::new(&mut app.settings.default_font_size, 5.0..=100.0));
            });
            
            egui::ComboBox::from_label("Varsayılan Yazı Tipi")
                .selected_text(&app.settings.default_font_family)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut app.settings.default_font_family, "Monospace".into(), "Monospace");
                    ui.selectable_value(&mut app.settings.default_font_family, "Proportional".into(), "Proportional");
                });
            
        });

    if !is_open {
        app.windowed_panels.remove("SettingsModal");
    }
}
