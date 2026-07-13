use eframe::egui;
use crate::data::border::{BorderPiece, BorderTemplate};

#[macro_export]
macro_rules! panel_section {
    ($self:expr, $ui:ident, $title:expr, |$inner_ui:ident| $body:expr) => {
        let title_str = $title.to_string();
        let is_window = $self.windowed_panels.contains(&title_str);
        if is_window {
            let mut open = true;
            egui::Window::new(&title_str)
                .id(egui::Id::new(&title_str))
                .open(&mut open)
                .show($ui.ctx(), |$inner_ui| {
                    $body
                });
            if !open {
                $self.windowed_panels.remove(&title_str);
            }
        } else {
            egui::collapsing_header::CollapsingState::load_with_default_open($ui.ctx(), $ui.make_persistent_id(&title_str), true)
                .show_header($ui, |header_ui| {
                    header_ui.label(&title_str);
                    if header_ui.button("⏏").on_hover_text("Pencere Olarak Ayır").clicked() {
                        $self.windowed_panels.insert(title_str.clone());
                    }
                })
                .body(|$inner_ui| {
                    $body
                });
        }
    };
}

pub fn json_editor(ui: &mut egui::Ui, title: &str, json_str: &mut String) {
    ui.collapsing(title, |ui| {
        ui.add(
            egui::TextEdit::multiline(json_str)
                .font(egui::FontId::monospace(14.0))
                .code_editor()
                .desired_width(f32::INFINITY)
                .desired_rows(10),
        );
    });
}

pub fn piece_ui(ui: &mut egui::Ui, id: &str, piece: &mut BorderPiece) {
    let label = if piece.pattern.is_empty() { " ".into() } else { piece.pattern.clone() };
    if ui.button(egui::RichText::new(label).font(egui::FontId::monospace(14.0))).clicked() {
        piece.is_editing = !piece.is_editing;
    }
    
    if piece.is_editing {
        let mut is_open = piece.is_editing;
        egui::Window::new(format!("{} Düzenle", id))
            .open(&mut is_open)
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Desen:");
                    ui.text_edit_singleline(&mut piece.pattern);
                });
                ui.checkbox(&mut piece.color_override, "Özel Renk Kullan");
                if piece.color_override {
                    ui.color_edit_button_srgb(&mut piece.fg_color);
                }
                ui.add(egui::Slider::new(&mut piece.offset_x, -50.0..=50.0).text("Offset X (px)"));
                ui.add(egui::Slider::new(&mut piece.offset_y, -50.0..=50.0).text("Offset Y (px)"));
            });
        piece.is_editing = is_open;
    }
}

pub fn template_ui(ui: &mut egui::Ui, id_prefix: &str, template: &mut BorderTemplate) {
    ui.horizontal(|ui| {
        piece_ui(ui, &format!("{} Sol Üst", id_prefix), &mut template.top_left);
        piece_ui(ui, &format!("{} Üst", id_prefix), &mut template.top_pattern);
        piece_ui(ui, &format!("{} Sağ Üst", id_prefix), &mut template.top_right);
    });
    ui.horizontal(|ui| {
        piece_ui(ui, &format!("{} Sol", id_prefix), &mut template.left_pattern);
        ui.label("      İç      ");
        piece_ui(ui, &format!("{} Sağ", id_prefix), &mut template.right_pattern);
    });
    ui.horizontal(|ui| {
        piece_ui(ui, &format!("{} Sol Alt", id_prefix), &mut template.bottom_left);
        piece_ui(ui, &format!("{} Alt", id_prefix), &mut template.bottom_pattern);
        piece_ui(ui, &format!("{} Sağ Alt", id_prefix), &mut template.bottom_right);
    });
}
