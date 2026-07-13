use eframe::egui;
use crate::app::AxiomStudio;
use crate::data::element::UiElement;
use crate::data::layer::{AxiomLayer, LayerKind};
use crate::data::border::ExtraBorder;
use crate::core::types::{Anchor, GradientDirection, TextAlign, TextValign, AnimationType};
use crate::data::border::BorderTemplate;
use crate::panel_section;
use crate::ui::widgets::template_ui;


pub fn show(app: &mut AxiomStudio, ctx: &egui::Context) {
        egui::SidePanel::right("inspector_panel").resizable(true).default_width(350.0).show(ctx, |ui| {
            ui.heading("Axiom Studio | Inspector");
            ui.separator();
            
            ui.horizontal(|ui| {
                if ui.button("➕ Yeni Obje Ekle").clicked() {
                    let mut new_el = UiElement::default();
                    new_el.id = format!("Obje_{}", app.scenes[app.active_scene].elements.len());
                    app.scenes[app.active_scene].elements.push(new_el);
                    app.selected_index = Some(app.scenes[app.active_scene].elements.len() - 1);
                    app.last_selected = app.selected_index;
                }
            });

            ui.separator();
            ui.label("Sahnedeki Objeler:");
            egui::ScrollArea::vertical().id_source("elements_list").max_height(150.0).show(ui, |ui| {
                let mut to_delete = None;
                for (i, el) in app.scenes[app.active_scene].elements.iter().enumerate() {
                    ui.horizontal(|ui| {
                        if ui.selectable_label(app.selected_index == Some(i), &el.id).clicked() {
                            app.selected_index = Some(i);
                            app.last_selected = Some(i);
                            app.json_buffer = None; // Reset buffer on selection change
                        }
                        if ui.button("🗑").clicked() {
                            to_delete = Some(i);
                        }
                    });
                }
                if let Some(i) = to_delete {
                    app.scenes[app.active_scene].elements.remove(i);
                    if app.selected_index == Some(i) { app.selected_index = None; }
                }
            });

            ui.separator();

            if let Some(idx) = app.selected_index {
                if let Some(el) = app.scenes[app.active_scene].elements.get_mut(idx) {
                    panel_section!(app, ui, format!("🎯 Obje Ayarları ({})", el.id), |ui| {
                    egui::ScrollArea::vertical().id_source("element_properties").show(ui, |ui| {
                        ui.horizontal(|ui| { ui.label("ID:"); ui.text_edit_singleline(&mut el.id); });
                        ui.horizontal(|ui| { 
                            ui.label("Parent ID:");
                            let mut pid_str = el.parent_id.clone().unwrap_or_default();
                            if ui.text_edit_singleline(&mut pid_str).changed() {
                                el.parent_id = if pid_str.trim().is_empty() { None } else { Some(pid_str) };
                            }
                        });
                        
                        // MASTER JSON
                        panel_section!(app, ui, "🔥 Master Element JSON (Tüm Ayarlar)", |ui| {
                            if app.json_buffer.is_none() {
                                app.json_buffer = Some(serde_json::to_string_pretty(el).unwrap());
                            }
                            if let Some(buf) = &mut app.json_buffer {
                                if ui.add(egui::TextEdit::multiline(buf).font(egui::FontId::monospace(14.0)).code_editor().desired_width(f32::INFINITY)).changed() {
                                    match serde_json::from_str::<UiElement>(buf) {
                                        Ok(parsed_el) => {
                                            *el = parsed_el;
                                            app.json_error = None;
                                        }
                                        Err(e) => {
                                            app.json_error = Some(e.to_string());
                                        }
                                    }
                                }
                            } else {
                                app.json_buffer = Some(serde_json::to_string_pretty(el).unwrap());
                                app.json_error = None;
                            }
                            if let Some(err) = &app.json_error {
                                ui.colored_label(egui::Color32::RED, format!("JSON Hatası: {}", err));
                            } else {
                                ui.colored_label(egui::Color32::GREEN, "✔ Kod Geçerli (Anlık Eşitleniyor).");
                            }
                        });

                        egui::CollapsingHeader::new("📌 Master Konum (Hitbox & Container)")
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.add(egui::Slider::new(&mut el.z_index, -100..=100).text("Z-Index"));
                                ui.add(egui::Slider::new(&mut el.pos_x, -100.0..=100.0).text("X Offset %"));
                                ui.add(egui::Slider::new(&mut el.pos_y, -100.0..=100.0).text("Y Offset %"));
                                ui.add(egui::Slider::new(&mut el.width, 1.0..=100.0).text("Genişlik %"));
                                ui.add(egui::Slider::new(&mut el.height, 1.0..=100.0).text("Yükseklik %"));
                                ui.horizontal(|ui| {
                                    ui.label("Anchor:");
                                    egui::ComboBox::from_id_source("anchor_box")
                                        .selected_text(format!("{:?}", el.anchor))
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut el.anchor, Anchor::TopLeft, "TopLeft");
                                            ui.selectable_value(&mut el.anchor, Anchor::TopCenter, "TopCenter");
                                            ui.selectable_value(&mut el.anchor, Anchor::TopRight, "TopRight");
                                            ui.selectable_value(&mut el.anchor, Anchor::Center, "Center");
                                            ui.selectable_value(&mut el.anchor, Anchor::BottomLeft, "BottomLeft");
                                            ui.selectable_value(&mut el.anchor, Anchor::BottomCenter, "BottomCenter");
                                            ui.selectable_value(&mut el.anchor, Anchor::BottomRight, "BottomRight");
                                        });
                                });
                                ui.horizontal(|ui| { ui.label("Bağlı Aksiyon:"); ui.text_edit_singleline(&mut el.action_binding); });
                            });

                        ui.separator();
                        ui.heading("🧩 Modüler Katmanlar (Layers)");
                        if ui.button("➕ Yeni Katman Ekle").clicked() {
                            let mut nl = AxiomLayer::default();
                            nl.id = format!("Katman_{}", el.layers.len());
                            el.layers.push(nl);
                            app.json_buffer = None;
                        }

                        let mut layer_to_del = None;
                        let mut layers_to_swap = None;
                        let layers_count = el.layers.len();

                        for (l_idx, layer) in el.layers.iter_mut().enumerate() {
                            panel_section!(app, ui, format!("Katman: {} [{:?}] ({})", layer.id, layer.kind, el.id), |ui| {
                                    ui.horizontal(|ui| {
                                        ui.checkbox(&mut layer.enabled, "Aktif");
                                        if ui.button("⬆").clicked() && l_idx > 0 { layers_to_swap = Some((l_idx, l_idx - 1)); }
                                        if ui.button("⬇").clicked() && l_idx < layers_count - 1 { layers_to_swap = Some((l_idx, l_idx + 1)); }
                                        if ui.button("🗑 Sil").clicked() { layer_to_del = Some(l_idx); }
                                    });
                                    ui.horizontal(|ui| { ui.label("ID:"); ui.text_edit_singleline(&mut layer.id); });
                                    ui.horizontal(|ui| {
                                        ui.label("Tür:");
                                        ui.selectable_value(&mut layer.kind, LayerKind::Fill, "Zemin");
                                        ui.selectable_value(&mut layer.kind, LayerKind::Border, "Kenarlık");
                                        ui.selectable_value(&mut layer.kind, LayerKind::Text, "Metin");
                                    });

                                    panel_section!(app, ui, format!("📐 Konum ({}-{})", el.id, l_idx), |ui| {
                                        ui.add(egui::Slider::new(&mut layer.z_index, -100..=100).text("Yerel Z-Index"));
                                        ui.add(egui::Slider::new(&mut layer.offset_x, -100.0..=100.0).text("X Kayması %"));
                                        ui.add(egui::Slider::new(&mut layer.offset_y, -100.0..=100.0).text("Y Kayması %"));
                                        ui.add(egui::Slider::new(&mut layer.fine_offset_x, -50.0..=50.0).text("Sub-Pixel X (px)"));
                                        ui.add(egui::Slider::new(&mut layer.fine_offset_y, -50.0..=50.0).text("Sub-Pixel Y (px)"));
                                        ui.add(egui::Slider::new(&mut layer.scale_x, 0.1..=3.0).text("Yatay Sıklık/Ölçek"));
                                        ui.add(egui::Slider::new(&mut layer.scale_y, 0.1..=3.0).text("Dikey Sıklık/Ölçek"));
                                        ui.add(egui::Slider::new(&mut layer.font_size, 5.0..=100.0).text("Yazı Boyutu (PX)"));
                                        egui::ComboBox::from_label("Yazı Tipi")
                                            .selected_text(&layer.font_family)
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(&mut layer.font_family, "Monospace".into(), "Monospace");
                                                ui.selectable_value(&mut layer.font_family, "Proportional".into(), "Proportional");
                                            });
                                        ui.add(egui::Slider::new(&mut layer.font_scale, 0.1..=5.0).text("Ölçek Çarpanı"));
                                        ui.add(egui::Slider::new(&mut layer.zigzag_x, -20.0..=20.0).text("Çapraz X (Zigzag Offset)"));
                                        ui.add(egui::Slider::new(&mut layer.zigzag_y, -20.0..=20.0).text("Çapraz Y (Zigzag Offset)"));
                                    });
                                    
                                    if layer.kind == LayerKind::Fill || layer.kind == LayerKind::Border {
                                        ui.add(egui::Slider::new(&mut layer.width, 0.0..=150.0).text("Genişlik (Parent %)"));
                                        ui.add(egui::Slider::new(&mut layer.height, 0.0..=150.0).text("Yükseklik (Parent %)"));
                                        ui.add(egui::Slider::new(&mut layer.padding_x, -50.0..=50.0).text("Padding X (İçe Daralma)"));
                                        ui.add(egui::Slider::new(&mut layer.padding_y, -50.0..=50.0).text("Padding Y (İçe Daralma)"));
                                        ui.add(egui::Slider::new(&mut layer.shear_x, -5.0..=5.0).text("Eğme (Shear X)"));
                                        ui.add(egui::Slider::new(&mut layer.pattern_spacing, 1..=10).text("Desen Boşluğu"));
                                    }
                                    if layer.kind == LayerKind::Border {
                                        ui.checkbox(&mut layer.border_composite, "Çoklu-Karakter Bindirme (Composite)");
                                        if layer.border_composite {
                                            ui.add(egui::Slider::new(&mut layer.composite_spacing_x, -20.0..=20.0).text("Bindirme X Aralığı"));
                                            ui.add(egui::Slider::new(&mut layer.composite_spacing_y, -20.0..=20.0).text("Bindirme Y Aralığı"));
                                        }
                                        ui.separator();
                                        ui.horizontal(|ui| {
                                            ui.heading("Ekstra Kenarlıklar");
                                            if ui.button("+").clicked() { layer.extra_borders.push(ExtraBorder::default()); }
                                        });
                                        let mut to_remove = None;
                                        for (i, eb) in layer.extra_borders.iter_mut().enumerate() {
                                            ui.group(|ui| {
                                                ui.horizontal(|ui| {
                                                    ui.heading(format!("Ek Kenarlık {}", i + 1));
                                                    if ui.button("🗑 Sil").clicked() { to_remove = Some(i); }
                                                });
                                                ui.horizontal(|ui| {
                                                    ui.add(egui::Slider::new(&mut eb.global_offset_x, -50.0..=50.0).text("Genel X"));
                                                    ui.add(egui::Slider::new(&mut eb.global_offset_y, -50.0..=50.0).text("Genel Y"));
                                                    ui.add(egui::DragValue::new(&mut eb.z_index).prefix("Z-Index: "));
                                                });
                                                template_ui(ui, &format!("Ek {}", i + 1), &mut eb.template);
                                            });
                                        }
                                        if let Some(i) = to_remove { layer.extra_borders.remove(i); }

                                    }

                                    ui.add(egui::Slider::new(&mut layer.alpha, 0.0..=1.0).text("Genel Saydamlık (Alpha)"));
                                    ui.add(egui::Slider::new(&mut layer.bg_alpha, 0.0..=1.0).text("Arkaplan Saydamlığı (BG Alpha)"));

                                    panel_section!(app, ui, format!("🎨 Renk ({}-{})", el.id, l_idx), |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label("Ana FG:"); ui.color_edit_button_srgb(&mut layer.fg_color);
                                            ui.label("Ana BG:"); ui.color_edit_button_srgb(&mut layer.bg_color);
                                        });
                                        ui.checkbox(&mut layer.use_gradient, "Gradient (Geçiş) Kullan");
                                        if layer.use_gradient {
                                            ui.horizontal(|ui| {
                                                ui.label("Hedef Renk:"); ui.color_edit_button_srgb(&mut layer.gradient_target);
                                            });
                                            ui.horizontal(|ui| {
                                                ui.selectable_value(&mut layer.gradient_dir, GradientDirection::Horizontal, "Yatay");
                                                ui.selectable_value(&mut layer.gradient_dir, GradientDirection::Vertical, "Dikey");
                                                ui.selectable_value(&mut layer.gradient_dir, GradientDirection::Diagonal, "Çapraz");
                                            });
                                        }
                                    });

                                    if layer.kind == LayerKind::Fill || layer.kind == LayerKind::Text {
                                        ui.horizontal(|ui| { ui.label("İçerik:"); ui.text_edit_singleline(&mut layer.content); });
                                    }

                                    if layer.kind == LayerKind::Text {
                                        ui.checkbox(&mut layer.repeat_content, "Metni Alan Boyunca Tekrarla");
                                        ui.horizontal(|ui| {
                                            ui.selectable_value(&mut layer.text_align, TextAlign::Left, "Sol");
                                            ui.selectable_value(&mut layer.text_align, TextAlign::Center, "Orta");
                                            ui.selectable_value(&mut layer.text_align, TextAlign::Right, "Sağ");
                                        });
                                        ui.horizontal(|ui| {
                                            ui.selectable_value(&mut layer.text_valign, TextValign::Top, "Üst");
                                            ui.selectable_value(&mut layer.text_valign, TextValign::Middle, "Orta");
                                            ui.selectable_value(&mut layer.text_valign, TextValign::Bottom, "Alt");
                                        });
                                        ui.add(egui::Slider::new(&mut layer.letter_spacing, 1.0..=50.0).text("Harf Boşluğu"));
                                        ui.add(egui::Slider::new(&mut layer.line_spacing, 1.0..=50.0).text("Satır Boşluğu"));
                                        ui.checkbox(&mut layer.wrap_text, "Metni Kutuda Kaydır (Wrap)");
                                        ui.horizontal(|ui| {
                                            ui.checkbox(&mut layer.text_outline, "Dış Çizgi (Outline)");
                                            if layer.text_outline {
                                                ui.color_edit_button_srgb(&mut layer.text_outline_color);
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            ui.checkbox(&mut layer.is_bold, "Kalın (Bold)");
                                            ui.checkbox(&mut layer.is_italic, "İtalik");
                                            ui.checkbox(&mut layer.is_underline, "Altı Çizili");
                                            ui.checkbox(&mut layer.is_strikethrough, "Üstü Çizili");
                                        });
                                        ui.add(egui::Slider::new(&mut layer.text_rotation, -360.0..=360.0).text("Yazı Dönüşü (Rotation)"));
                                    }

                                    if layer.kind == LayerKind::Border {
                                        ui.horizontal(|ui| {
                                            if ui.button(egui::RichText::new("╔═╗").font(egui::FontId::monospace(14.0))).clicked() { layer.border = BorderTemplate::default(); }
                                            if ui.button(egui::RichText::new("=*=").font(egui::FontId::monospace(14.0))).clicked() { layer.border = BorderTemplate::interwoven(); }
                                            if ui.button(egui::RichText::new("╭─╮").font(egui::FontId::monospace(14.0))).clicked() { layer.border = BorderTemplate::round(); }
                                            if ui.button(egui::RichText::new("██").font(egui::FontId::monospace(14.0))).clicked() { layer.border = BorderTemplate::solid(); }
                                        });
                                        ui.group(|ui| {
                                            template_ui(ui, "Ana Kenarlık", &mut layer.border);
                                        });
                                    }

                                    panel_section!(app, ui, format!("Hover / Pressed ({}-{})", el.id, l_idx), |ui| {
                                        ui.checkbox(&mut layer.hover_state.enabled, "Hover Aktif");
                                        if layer.hover_state.enabled {
                                            ui.horizontal(|ui| {
                                                ui.color_edit_button_srgb(&mut layer.hover_state.fg_color);
                                                ui.color_edit_button_srgb(&mut layer.hover_state.bg_color);
                                            });
                                        }
                                        ui.checkbox(&mut layer.pressed_state.enabled, "Pressed Aktif");
                                        if layer.pressed_state.enabled {
                                            ui.horizontal(|ui| {
                                                ui.color_edit_button_srgb(&mut layer.pressed_state.fg_color);
                                                ui.color_edit_button_srgb(&mut layer.pressed_state.bg_color);
                                            });
                                        }
                                    });

                                    panel_section!(app, ui, format!("VFX & Animasyon ({}-{})", el.id, l_idx), |ui| {
                                        ui.checkbox(&mut layer.drop_shadow, "Gölge Ekle");
                                        if layer.drop_shadow {
                                            ui.color_edit_button_srgb(&mut layer.shadow_color);
                                            ui.add(egui::Slider::new(&mut layer.shadow_offset_x, -20.0..=20.0).text("Gölge X"));
                                            ui.add(egui::Slider::new(&mut layer.shadow_offset_y, -20.0..=20.0).text("Gölge Y"));
                                        }
                                        egui::ComboBox::from_id_source(format!("anim_{}", l_idx))
                                            .selected_text(format!("{:?}", layer.animation))
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(&mut layer.animation, AnimationType::None, "Yok");
                                                ui.selectable_value(&mut layer.animation, AnimationType::Blink, "Blink");
                                                ui.selectable_value(&mut layer.animation, AnimationType::Wave, "Wave");
                                                ui.selectable_value(&mut layer.animation, AnimationType::PulseColor, "Pulse");
                                                ui.selectable_value(&mut layer.animation, AnimationType::Typewriter, "Typewriter");
                                                ui.selectable_value(&mut layer.animation, AnimationType::MatrixRain, "Matrix Rain");
                                            });
                                        ui.add(egui::Slider::new(&mut layer.anim_speed, 0.1..=10.0).text("Anim Hız"));
                                        ui.add(egui::Slider::new(&mut layer.anim_amplitude, 1.0..=50.0).text("Dalga Gücü"));
                                    });
                                });
                        }
                        if let Some(idx) = layer_to_del { el.layers.remove(idx); app.json_buffer = None; }
                        if let Some((i, j)) = layers_to_swap { el.layers.swap(i, j); app.json_buffer = None; }
                    });
                    });
                }
            }
        });
}
