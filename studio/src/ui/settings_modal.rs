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
            
            ui.separator();
            ui.heading("💻 Donanım Hızlandırma (WGPU)");
            ui.label(format!("Aktif Ekran Kartı (GPU): {}", app.active_gpu_name));
            
            ui.horizontal(|ui| {
                ui.label("GPU Tercihi (Laptoplar İçin):");
                let mut current_pref = app.gpu_preference.clone();
                egui::ComboBox::from_id_source("gpu_pref")
                    .selected_text(if current_pref == "HighPerformance" { "Harici GPU (Yüksek Performans)" } else { "Dahili GPU (Güç Tasarrufu)" })
                    .show_ui(ui, |ui| {
                        if ui.selectable_value(&mut current_pref, "HighPerformance".to_string(), "Harici GPU (Yüksek Performans)").clicked() {
                            std::fs::write("gpu_preference.txt", "HighPerformance").ok();
                            app.gpu_preference = current_pref.clone();
                        }
                        if ui.selectable_value(&mut current_pref, "LowPower".to_string(), "Dahili GPU (Güç Tasarrufu)").clicked() {
                            std::fs::write("gpu_preference.txt", "LowPower").ok();
                            app.gpu_preference = current_pref.clone();
                        }
                    });
            });
            ui.label(egui::RichText::new("Uyarı: GPU değişikliklerinin uygulanması için uygulamayı yeniden başlatmanız gerekmektedir.").color(egui::Color32::YELLOW).small());
            
        });

    if !is_open {
        app.windowed_panels.remove("SettingsModal");
    }
}
