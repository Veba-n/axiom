use eframe::egui;
use crate::app::AxiomStudio;


pub fn show(app: &mut AxiomStudio, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut app.current_mode, crate::app::EditorMode::UiDesigner, "UI Tasarımı");
                ui.selectable_value(&mut app.current_mode, crate::app::EditorMode::LevelEditor, "3D Seviye");
                ui.selectable_value(&mut app.current_mode, crate::app::EditorMode::ObjectEditor, "Oyun Objeleri");
                ui.selectable_value(&mut app.current_mode, crate::app::EditorMode::TextureEditor, "Doku & Materyal");
                ui.separator();
                if ui.button("▶ OYNA (Test)").clicked() {
                    app.is_playing = !app.is_playing;
                    if app.is_playing {
                        app.preview_stack = vec![app.active_scene];
                    }
                }
                if ui.button("⮪ Undo").clicked() {
                    app.undo();
                }
                if ui.button("⮫ Redo").clicked() {
                    app.redo();
                }
                if ui.button("💾 Kaydet (.axiom)").clicked() {
                    app.push_history();
                    app.save_project();
                }
                ui.label(format!("Geçmiş: {} / {}", app.history_index + 1, app.history.len()));
                if ui.button("⚙ Ayarlar").clicked() {
                    app.windowed_panels.insert("SettingsModal".into());
                }
                ui.separator();
                ui.checkbox(&mut app.show_fps, "FPS Göster");
                ui.separator();
                ui.label("Axiom Studio");
            });
        });
}
