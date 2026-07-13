use eframe::egui;
use crate::app::AxiomStudio;
use crate::render::object_viewport::sample_cell;



use crate::data::border::{BorderTemplate, ExtraBorder};
use crate::data::texture::{AxiomTexture, TextureLayer, BlendMode, HeightFunction, LayerGenMode, TileWrapMode};
use crate::data::texture_presets;
use crate::render::texture_composer::ComposedTexture;
use crate::ui::widgets::template_ui;

fn gen_mode_label(mode: &LayerGenMode) -> &'static str {
    match mode {
        LayerGenMode::Solid => "Solid — Düz Tekrar",
        LayerGenMode::Noise => "Noise — Rastgele Dağılım",
        LayerGenMode::Checker => "Checker — Dama Tahtası",
        LayerGenMode::Border => "Border — Basit Çerçeve",
        LayerGenMode::Fill => "Fill — İç Dolgu (Tuğla Aralığı)",
        LayerGenMode::DirectionalBorder => "Directional — Yönlü Kenarlar (Üst/Sol/Alt/Sağ)",
    }
}

pub fn show(app: &mut AxiomStudio, ctx: &egui::Context) {
    let mut to_add_layer: Option<usize> = None;
    let mut to_remove_layer: Option<(usize, usize)> = None;
    let mut to_remove_tex: Option<usize> = None;
    let mut to_add_extra_border: Option<(usize, usize)> = None;
    let mut to_remove_extra_border: Option<(usize, usize, usize)> = None;

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Materyal & Doku Editörü");
        ui.label(
            "Katmanlı ASCII materyal sistemi — yönlü kenarlar, tiling, katmanlar arası ek kenarlar ve height map.",
        );
        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("➕ Yeni Materyal").clicked() {
                let mut new_tex = AxiomTexture::default();
                new_tex.id = format!("Tex_{}", app.textures.len() + 1);
                app.textures.push(new_tex);
                app.active_texture_index = app.textures.len() - 1;
            }
            ui.separator();
            ui.label("Hazır Şablonlar:");
            if ui.button("🧱 Taş Tuğla").clicked() {
                let p = texture_presets::preset_stone_brick(&format!("Tex_{}", app.textures.len() + 1));
                app.textures.push(p);
                app.active_texture_index = app.textures.len() - 1;
            }
            if ui.button("🪨 Ham Taş").clicked() {
                let p = texture_presets::preset_rough_stone(&format!("Tex_{}", app.textures.len() + 1));
                app.textures.push(p);
                app.active_texture_index = app.textures.len() - 1;
            }
            if ui.button("🪵 Tahta").clicked() {
                let p = texture_presets::preset_wood_plank(&format!("Tex_{}", app.textures.len() + 1));
                app.textures.push(p);
                app.active_texture_index = app.textures.len() - 1;
            }
            if ui.button("🧱 Tuğla+Harç").clicked() {
                let p = texture_presets::preset_brick_with_mortar(&format!("Tex_{}", app.textures.len() + 1));
                app.textures.push(p);
                app.active_texture_index = app.textures.len() - 1;
            }
            if ui.button("🔩 Metal Plaka").clicked() {
                let p = texture_presets::preset_metal_plate(&format!("Tex_{}", app.textures.len() + 1));
                app.textures.push(p);
                app.active_texture_index = app.textures.len() - 1;
            }
            if ui.button("🧵 Kumaş").clicked() {
                let p = texture_presets::preset_fabric(&format!("Tex_{}", app.textures.len() + 1));
                app.textures.push(p);
                app.active_texture_index = app.textures.len() - 1;
            }
            if ui.button("🎨 Mozaik").clicked() {
                let p = texture_presets::preset_mosaic(&format!("Tex_{}", app.textures.len() + 1));
                app.textures.push(p);
                app.active_texture_index = app.textures.len() - 1;
            }
        });
        ui.separator();

        if app.textures.is_empty() {
            ui.label("Materyal ekleyin veya hazır şablondan birini seçin.");
            return;
        }

        if app.active_texture_index >= app.textures.len() {
            app.active_texture_index = app.textures.len() - 1;
        }

        ui.columns(2, |cols| {
            let (left_cols, right_cols) = cols.split_at_mut(1);
            let left_ui = &mut left_cols[0];
            let right_ui = &mut right_cols[0];

            egui::ScrollArea::vertical()
                .id_source("tex_scroll")
                .show(left_ui, |ui| {
                    for (t_idx, tex) in app.textures.iter_mut().enumerate() {
                        let selected = app.active_texture_index == t_idx;
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                let label = if selected {
                                    format!("▶ {}", tex.name)
                                } else {
                                    tex.name.clone()
                                };
                                if ui.selectable_label(selected, label).clicked() {
                                    app.active_texture_index = t_idx;
                                }
                                if ui.button("🗑").clicked() {
                                    to_remove_tex = Some(t_idx);
                                }
                            });

                            if !selected {
                                return;
                            }

                            ui.horizontal(|ui| {
                                ui.label("ID:");
                                ui.text_edit_singleline(&mut tex.id);
                                ui.label("İsim:");
                                ui.text_edit_singleline(&mut tex.name);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Grid:");
                                ui.add(egui::DragValue::new(&mut tex.resolution[0]).clamp_range(1..=128));
                                ui.label("×");
                                ui.add(egui::DragValue::new(&mut tex.resolution[1]).clamp_range(1..=128));
                                ui.label("Zemin:");
                                ui.color_edit_button_srgb(&mut tex.base_color);
                            });

                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.label("Katmanlar (alttan üste Z-Index):");
                                if ui.button("➕ Katman").clicked() {
                                    to_add_layer = Some(t_idx);
                                }
                            });

                            let mut sorted_layers: Vec<(usize, i32)> = tex
                                .layers
                                .iter()
                                .enumerate()
                                .map(|(i, l)| (i, l.z_index))
                                .collect();
                            sorted_layers.sort_by(|a, b| a.1.cmp(&b.1));

                            for (l_idx, _) in sorted_layers {
                                layer_panel(
                                    ui,
                                    t_idx,
                                    l_idx,
                                    &mut tex.layers[l_idx],
                                    &mut to_remove_layer,
                                    &mut to_add_extra_border,
                                    &mut to_remove_extra_border,
                                );
                            }
                        });
                    }
                });

            right_ui.heading("Önizleme");
            ui_preview_controls(right_ui, app);

            // PERFORMANS: Eskiden burada aktif dokunun TAMAMI her frame
            // klonlanıp compose() ile sıfırdan hesaplanıyordu — doku hiç
            // değişmemiş olsa bile. Artık app.texture_cache sadece içerik
            // gerçekten değiştiğinde yeniden hesaplıyor, aksi halde
            // önbellekten anında dönüyor.
            let composed = app
                .texture_cache
                .get_or_compose(&app.textures[app.active_texture_index])
                .clone();
            draw_preview(right_ui, &composed, &app.textures[app.active_texture_index], app.texture_preview_mode);
        });
    });

    if let Some(idx) = to_remove_tex {
        app.textures.remove(idx);
        if app.active_texture_index >= app.textures.len() && !app.textures.is_empty() {
            app.active_texture_index = app.textures.len() - 1;
        }
    }
    if let Some(t_idx) = to_add_layer {
        app.textures[t_idx].layers.push(TextureLayer::default());
    }
    if let Some((t_idx, l_idx)) = to_remove_layer {
        app.textures[t_idx].layers.remove(l_idx);
    }
    if let Some((t_idx, l_idx)) = to_add_extra_border {
        app.textures[t_idx].layers[l_idx]
            .extra_borders
            .push(ExtraBorder::default());
    }
    if let Some((t_idx, l_idx, eb_idx)) = to_remove_extra_border {
        app.textures[t_idx].layers[l_idx].extra_borders.remove(eb_idx);
    }
}

fn ui_preview_controls(ui: &mut egui::Ui, app: &mut AxiomStudio) {
    ui.horizontal(|ui| {
        ui.label("Görünüm:");
        ui.selectable_value(&mut app.texture_preview_mode, 0, "2D Renk + ASCII");
        ui.selectable_value(&mut app.texture_preview_mode, 1, "Height Map");
        ui.selectable_value(&mut app.texture_preview_mode, 2, "Tile Izgarası");
        ui.selectable_value(&mut app.texture_preview_mode, 3, "3D Önizleme");
    });
}

fn layer_panel(
    ui: &mut egui::Ui,
    t_idx: usize,
    l_idx: usize,
    layer: &mut TextureLayer,
    to_remove: &mut Option<(usize, usize)>,
    to_add_eb: &mut Option<(usize, usize)>,
    to_remove_eb: &mut Option<(usize, usize, usize)>,
) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.checkbox(&mut layer.is_visible, "");
            ui.text_edit_singleline(&mut layer.name);
            ui.label("Z:");
            ui.add(egui::DragValue::new(&mut layer.z_index));
            if ui.button("🗑").clicked() {
                *to_remove = Some((t_idx, l_idx));
            }
        });

        if !layer.is_visible {
            return;
        }

        ui.push_id(format!("gen_mode_{}_{}", t_idx, l_idx), |ui| {
            ui.collapsing("1. Üretim Modu", |ui| {
            ui.horizontal(|ui| {
                ui.label("Mod:");
                egui::ComboBox::from_id_source(format!("gen_{}_{}", t_idx, l_idx))
                    .selected_text(gen_mode_label(&layer.gen_mode))
                    .width(280.0)
                    .show_ui(ui, |ui| {
                        for mode in [
                            LayerGenMode::Solid,
                            LayerGenMode::Fill,
                            LayerGenMode::Noise,
                            LayerGenMode::Checker,
                            LayerGenMode::Border,
                            LayerGenMode::DirectionalBorder,
                        ] {
                            let label = gen_mode_label(&mode);
                            ui.selectable_value(&mut layer.gen_mode, mode, label);
                        }
                    });
            });

            match layer.gen_mode {
                LayerGenMode::Noise => {
                    ui.horizontal(|ui| {
                        ui.label("Karakter seti:");
                        ui.text_edit_singleline(&mut layer.pattern);
                    });
                    ui.add(egui::Slider::new(&mut layer.noise_density, 0.0..=1.0).text("Yoğunluk"));
                }
                LayerGenMode::DirectionalBorder => {
                    ui.label("Ana kenar şablonu — her kenara ayrı desen:");
                    template_ui(ui, &format!("main_{}_{}", t_idx, l_idx), &mut layer.border);
                    ui.horizontal(|ui| {
                        if ui.button("Tuğla").clicked() {
                            layer.border = BorderTemplate::brick();
                        }
                        if ui.button("Taş").clicked() {
                            layer.border = BorderTemplate::stone();
                        }
                        if ui.button("Yuvarlak").clicked() {
                            layer.border = BorderTemplate::round();
                        }
                        if ui.button("Örgü").clicked() {
                            layer.border = BorderTemplate::interwoven();
                        }
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Katmanlar arası ek kenarlar:");
                        if ui.button("➕ Ek Kenar").clicked() {
                            *to_add_eb = Some((t_idx, l_idx));
                        }
                    });
                    for (eb_i, eb) in layer.extra_borders.iter_mut().enumerate() {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(format!("Ek Kenar #{}", eb_i + 1));
                                ui.label("Z:");
                                ui.add(egui::DragValue::new(&mut eb.z_index));
                                if ui.button("🗑").clicked() {
                                    *to_remove_eb = Some((t_idx, l_idx, eb_i));
                                }
                            });
                            template_ui(
                                ui,
                                &format!("eb_{}_{}_{}", t_idx, l_idx, eb_i),
                                &mut eb.template,
                            );
                            ui.horizontal(|ui| {
                                ui.label("Global offset:");
                                ui.add(egui::DragValue::new(&mut eb.global_offset_x).speed(0.5));
                                ui.add(egui::DragValue::new(&mut eb.global_offset_y).speed(0.5));
                            });
                        });
                    }
                    ui.checkbox(&mut layer.border_composite, "Çoklu karakter (composite)");
                    if layer.border_composite {
                        ui.horizontal(|ui| {
                            ui.label("Composite aralık:");
                            ui.add(egui::DragValue::new(&mut layer.composite_spacing_x).speed(0.5));
                            ui.add(egui::DragValue::new(&mut layer.composite_spacing_y).speed(0.5));
                        });
                    }
                    ui.add(
                        egui::Slider::new(&mut layer.pattern_spacing, 1..=8)
                            .text("Tuğla aralığı (pattern spacing)"),
                    );
                }
                _ => {
                    ui.horizontal(|ui| {
                        ui.label("Desen / karakter seti:");
                        ui.text_edit_singleline(&mut layer.pattern);
                    });
                    if layer.gen_mode == LayerGenMode::Fill {
                        ui.add(
                            egui::Slider::new(&mut layer.pattern_spacing, 1..=8)
                                .text("Dolgu aralığı (tuğla boşluğu)"),
                        );
                    }
                }
            }
        });
        });

        ui.push_id(format!("tiling_uv_{}_{}", t_idx, l_idx), |ui| {
            ui.collapsing("2. Tiling & UV", |ui| {
            ui.label("Tekrar aralığı — küçük değer = sık tekrar, büyük = geniş tile:");
            ui.horizontal(|ui| {
                ui.label("Tile X:");
                ui.add(egui::DragValue::new(&mut layer.uv_scale[0]).speed(0.1).clamp_range(0.1..=64.0));
                ui.label("Tile Y:");
                ui.add(egui::DragValue::new(&mut layer.uv_scale[1]).speed(0.1).clamp_range(0.1..=64.0));
            });
            ui.horizontal(|ui| {
                ui.label("Offset X/Y:");
                ui.add(egui::DragValue::new(&mut layer.uv_offset[0]).speed(0.1));
                ui.add(egui::DragValue::new(&mut layer.uv_offset[1]).speed(0.1));
            });
            ui.horizontal(|ui| {
                ui.label("Wrap:");
                egui::ComboBox::from_id_source(format!("wrap_{}_{}", t_idx, l_idx))
                    .selected_text(format!("{:?}", layer.tile_wrap))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut layer.tile_wrap, TileWrapMode::Repeat, "Repeat — Tekrar");
                        ui.selectable_value(&mut layer.tile_wrap, TileWrapMode::Mirror, "Mirror — Ayna");
                        ui.selectable_value(&mut layer.tile_wrap, TileWrapMode::Clamp, "Clamp — Kenarda kes");
                    });
            });
            ui.add(
                egui::Slider::new(&mut layer.rotation, -180.0..=180.0)
                    .text("Desen Açısı (°) — çapraz tuğla / açılı tahta damarı için"),
            );
            if layer.gen_mode == LayerGenMode::DirectionalBorder && layer.rotation.abs() > f32::EPSILON {
                ui.colored_label(
                    egui::Color32::from_rgb(230, 180, 90),
                    "⚠ Desen Açısı, Directional Border kenar hizalamasını bozabilir.",
                );
            }
        });
        });

ui.push_id(format!("color_height_{}_{}", t_idx, l_idx), |ui| {
    ui.collapsing("3. Renk & Height", |ui| {
    ui.horizontal(|ui| {
        ui.label("Renk:");
        ui.color_edit_button_srgb(&mut layer.fg_color);
        let mut has_grad = layer.fg_gradient_end.is_some();
        if ui.checkbox(&mut has_grad, "Gradient").changed() {
            layer.fg_gradient_end = if has_grad {
                Some([255, 255, 255])
            } else {
                None
            };
        }
        if let Some(g) = &mut layer.fg_gradient_end {
            ui.color_edit_button_srgb(g);
        }
        ui.checkbox(&mut layer.manual_painting, "Elle Boyama");
        ui.checkbox(&mut layer.pattern_lock, "Pattern Kilitle");
    });
ui.horizontal(|ui| {
    ui.label("Height (Z-map):");
    ui.add(egui::DragValue::new(&mut layer.height_val).speed(0.1).clamp_range(0.0..=10.0));
    ui.label("Emission:");
    ui.add(egui::DragValue::new(&mut layer.emission_val).speed(0.1).clamp_range(0.0..=5.0));
    
    // 3D dönüş kontrolleri
    ui.label("Dönüş:");
    ui.add(egui::DragValue::new(&mut layer.rotation_3d[0]).speed(1.0).clamp_range(-180.0..=180.0).prefix("X:"));
    ui.add(egui::DragValue::new(&mut layer.rotation_3d[1]).speed(1.0).clamp_range(-180.0..=180.0).prefix("Y:"));
    ui.add(egui::DragValue::new(&mut layer.rotation_3d[2]).speed(1.0).clamp_range(-180.0..=180.0).prefix("Z:"));
    
    ui.label("Ölçek:");
    ui.add(egui::DragValue::new(&mut layer.scale_3d[0]).speed(0.1).clamp_range(0.1..=5.0).prefix("X:"));
    ui.add(egui::DragValue::new(&mut layer.scale_3d[1]).speed(0.1).clamp_range(0.1..=5.0).prefix("Y:"));
    ui.add(egui::DragValue::new(&mut layer.scale_3d[2]).speed(0.1).clamp_range(0.1..=5.0).prefix("Z:"));
});
            ui.horizontal(|ui| {
                ui.label("Blend:");
                egui::ComboBox::from_id_source(format!("blend_{}_{}", t_idx, l_idx))
                    .selected_text(format!("{:?}", layer.blend_mode))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut layer.blend_mode, BlendMode::Normal, "Normal");
                        ui.selectable_value(&mut layer.blend_mode, BlendMode::Additive, "Additive");
                        ui.selectable_value(&mut layer.blend_mode, BlendMode::Multiply, "Multiply");
                        ui.selectable_value(&mut layer.blend_mode, BlendMode::Subtractive, "Subtractive");
                        ui.selectable_value(&mut layer.blend_mode, BlendMode::Overlay, "Overlay");
                    });
                ui.label("Opaklık:");
                ui.add(egui::Slider::new(&mut layer.opacity, 0.0..=1.0));
            });
            ui.horizontal(|ui| {
                ui.label("Font boyutu:");
                ui.add(egui::DragValue::new(&mut layer.font_size).speed(0.5).clamp_range(4.0..=64.0));
            });
        });
        });

        ui.push_id(format!("relief_{}_{}", t_idx, l_idx), |ui| {
            ui.collapsing("4. Kabartma (Relief)", |ui| {
            ui.label("Height_val'in üzerine hücre hücre değişen bir kabartma deseni ekler.");
            ui.horizontal(|ui| {
                ui.label("Fonksiyon:");
                egui::ComboBox::from_id_source(format!("height_fn_{}_{}", t_idx, l_idx))
                    .selected_text(match layer.height_function {
                        HeightFunction::Flat => "Düz (kapalı)",
                        HeightFunction::Noise => "Noise — Kaba taş/sıva",
                        HeightFunction::Wave => "Wave — Tahta damarı/yaşlılık çizgisi",
                        HeightFunction::CellBulge => "Cell Bulge — Kabartmalı tuğla/karo",
                    })
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut layer.height_function, HeightFunction::Flat, "Düz (kapalı)");
                        ui.selectable_value(&mut layer.height_function, HeightFunction::Noise, "Noise — Kaba taş/sıva");
                        ui.selectable_value(&mut layer.height_function, HeightFunction::Wave, "Wave — Tahta damarı/yaşlılık çizgisi");
                        ui.selectable_value(&mut layer.height_function, HeightFunction::CellBulge, "Cell Bulge — Kabartmalı tuğla/karo");
                    });
            });
            ui.add(
                egui::Slider::new(&mut layer.height_amplitude, -5.0..=5.0)
                    .text("Genlik — ne kadar yukarı/aşağı (0 = etkisiz)"),
            );
            ui.add(
                egui::Slider::new(&mut layer.height_frequency, 0.1..=8.0)
                    .text("Sıklık (frequency)"),
            );
            ui.label(
                egui::RichText::new(
                    "İpucu: Tuğla/taş için height_function olarak Cell Bulge'ı \"Tuğla Kenarları\" veya \"Tuğla Dolgu\" katmanında \
                    pattern_spacing ile birlikte kullanın. Tahtadaki yaşlılık çizgileri için Wave'i, Desen Açısı'nı tahta damarı \
                    yönüne çevirip kullanın."
                ).size(11.0).italics(),
            );
        });
        });
    });
}

fn draw_preview(
    ui: &mut egui::Ui,
    composed: &ComposedTexture,
    tex: &AxiomTexture,
    preview_mode: u8,
) {
    let preview_size = ui.available_size();
    let (response, painter) = ui.allocate_painter(preview_size, egui::Sense::hover());
    let rect = response.rect;

    let grid_w = composed.width.max(1) as f32;
    let grid_h = composed.height.max(1) as f32;
    let cell_w = rect.width() / grid_w;
    let cell_h = rect.height() / grid_h;

    painter.rect_filled(
        rect,
        4.0,
        egui::Color32::from_rgb(tex.base_color[0], tex.base_color[1], tex.base_color[2]),
    );

    let max_height = composed
        .height_map
        .iter()
        .flatten()
        .cloned()
        .fold(0.01_f32, f32::max);

    if preview_mode == 3 {
        // 3D önizleme modu - Tam 3D kübik görünüm
        let mut cam_rot_x: f32 = ui.ctx().data_mut(|d| d.get_temp(egui::Id::new("tex_cam_x")).unwrap_or(20.0f32.to_radians()));
        let mut cam_rot_y: f32 = ui.ctx().data_mut(|d| d.get_temp(egui::Id::new("tex_cam_y")).unwrap_or(30.0f32.to_radians()));
        let mut cam_zoom: f32 = ui.ctx().data_mut(|d| d.get_temp(egui::Id::new("tex_cam_zoom")).unwrap_or(150.0));
        
        let mut cube_scale_x: f32 = ui.ctx().data_mut(|d| d.get_temp(egui::Id::new("tex_cube_x")).unwrap_or(1.0));
        let mut cube_scale_y: f32 = ui.ctx().data_mut(|d| d.get_temp(egui::Id::new("tex_cube_y")).unwrap_or(1.0));
        let mut cube_scale_z: f32 = ui.ctx().data_mut(|d| d.get_temp(egui::Id::new("tex_cube_z")).unwrap_or(1.0));
        
        let mut uv_scale_x: f32 = ui.ctx().data_mut(|d| d.get_temp(egui::Id::new("tex_uv_scale_x")).unwrap_or(1.0));
        let mut uv_scale_y: f32 = ui.ctx().data_mut(|d| d.get_temp(egui::Id::new("tex_uv_scale_y")).unwrap_or(1.0));
        
        let mut uv_offset_x: f32 = ui.ctx().data_mut(|d| d.get_temp(egui::Id::new("tex_uv_offset_x")).unwrap_or(0.0));
        let mut uv_offset_y: f32 = ui.ctx().data_mut(|d| d.get_temp(egui::Id::new("tex_uv_offset_y")).unwrap_or(0.0));

        let mut auto_tile: bool = ui.ctx().data_mut(|d| d.get_temp(egui::Id::new("tex_auto_tile")).unwrap_or(true));

        // 3D Test Ayarları Yüzen Penceresi
        egui::Window::new("3D Test Ayarları")
            .id(egui::Id::new("tex_3d_test_settings"))
            .default_pos(rect.min + egui::vec2(10.0, 10.0))
            .show(ui.ctx(), |ui| {
                ui.label("Obje Boyutu (Scale):");
                ui.horizontal(|ui| {
                    if ui.add(egui::DragValue::new(&mut cube_scale_x).speed(0.1).clamp_range(0.1..=10.0).prefix("X: ")).changed() {
                        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("tex_cube_x"), cube_scale_x));
                    }
                    if ui.add(egui::DragValue::new(&mut cube_scale_y).speed(0.1).clamp_range(0.1..=10.0).prefix("Y: ")).changed() {
                        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("tex_cube_y"), cube_scale_y));
                    }
                    if ui.add(egui::DragValue::new(&mut cube_scale_z).speed(0.1).clamp_range(0.1..=10.0).prefix("Z: ")).changed() {
                        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("tex_cube_z"), cube_scale_z));
                    }
                });
                
                ui.separator();
                ui.label("Doku Yerleşimi (Global Tiling/Offset):");
                if ui.checkbox(&mut auto_tile, "Boyutla Orantılı Döşe (Auto-Tile)").on_hover_text("Obje büyüdükçe dokunun sündürülmesini engeller, döşemeyi artırır.").changed() {
                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("tex_auto_tile"), auto_tile));
                }
                ui.horizontal(|ui| {
                    if ui.add(egui::DragValue::new(&mut uv_scale_x).speed(0.1).prefix("S-X: ")).changed() {
                        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("tex_uv_scale_x"), uv_scale_x));
                    }
                    if ui.add(egui::DragValue::new(&mut uv_scale_y).speed(0.1).prefix("S-Y: ")).changed() {
                        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("tex_uv_scale_y"), uv_scale_y));
                    }
                });
                ui.horizontal(|ui| {
                    if ui.add(egui::DragValue::new(&mut uv_offset_x).speed(0.1).prefix("O-X: ")).changed() {
                        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("tex_uv_offset_x"), uv_offset_x));
                    }
                    if ui.add(egui::DragValue::new(&mut uv_offset_y).speed(0.1).prefix("O-Y: ")).changed() {
                        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("tex_uv_offset_y"), uv_offset_y));
                    }
                });
                ui.label("Fare: Sol tık ile çevir, tekerlek ile yakınlaştır");
            });
        
        let response = ui.interact(rect, egui::Id::new("texture_camera_3d_drag"), egui::Sense::drag());
        
        if response.dragged() {
            let delta = response.drag_delta();
            cam_rot_y += delta.x * 0.01;
            cam_rot_x += delta.y * 0.01;
            ui.ctx().data_mut(|d| {
                d.insert_temp(egui::Id::new("tex_cam_x"), cam_rot_x);
                d.insert_temp(egui::Id::new("tex_cam_y"), cam_rot_y);
            });
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll != 0.0 {
                cam_zoom = (cam_zoom + scroll * 0.5).clamp(20.0, 500.0);
                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("tex_cam_zoom"), cam_zoom));
            }
        }

        let center_x = rect.center().x;
        let center_y = rect.center().y;

        let project_3d = |x: f32, y: f32, z: f32| -> (egui::Pos2, f32) {
            let y1 = y * cam_rot_x.cos() - z * cam_rot_x.sin();
            let z1 = y * cam_rot_x.sin() + z * cam_rot_x.cos();
            
            let x2 = x * cam_rot_y.cos() + z1 * cam_rot_y.sin();
            let z2 = -x * cam_rot_y.sin() + z1 * cam_rot_y.cos();
            
            // True Perspective Projection (Gerçek Perspektif Derinliği)
            let perspective_strength = 2.0; // Odak uzaklığı
            let z_dist = perspective_strength + z2; 
            let factor = if z_dist > 0.1 { perspective_strength / z_dist } else { 1.0 };
            
            let screen_pos = egui::pos2(center_x + x2 * cam_zoom * factor, center_y - y1 * cam_zoom * factor);
            (screen_pos, z2)
        };

        let s = 0.5;
        let verts = [
            project_3d(-s * cube_scale_x, -s * cube_scale_y, -s * cube_scale_z), project_3d( s * cube_scale_x, -s * cube_scale_y, -s * cube_scale_z),
            project_3d( s * cube_scale_x,  s * cube_scale_y, -s * cube_scale_z), project_3d(-s * cube_scale_x,  s * cube_scale_y, -s * cube_scale_z),
            project_3d(-s * cube_scale_x, -s * cube_scale_y,  s * cube_scale_z), project_3d( s * cube_scale_x, -s * cube_scale_y,  s * cube_scale_z),
            project_3d( s * cube_scale_x,  s * cube_scale_y,  s * cube_scale_z), project_3d(-s * cube_scale_x,  s * cube_scale_y,  s * cube_scale_z),
        ];

        let mut faces = vec![
            (vec![0, 1, 2, 3], "-Z Ön"),
            (vec![1, 5, 6, 2], "+X Sağ"),
            (vec![5, 4, 7, 6], "+Z Arka"),
            (vec![4, 0, 3, 7], "-X Sol"),
            (vec![3, 2, 6, 7], "+Y Üst"),
            (vec![4, 5, 1, 0], "-Y Alt"),
        ];
        
        faces.sort_by(|a, b| {
            let z_a = (verts[a.0[0]].1 + verts[a.0[1]].1 + verts[a.0[2]].1 + verts[a.0[3]].1) / 4.0;
            let z_b = (verts[b.0[0]].1 + verts[b.0[1]].1 + verts[b.0[2]].1 + verts[b.0[3]].1) / 4.0;
            z_a.partial_cmp(&z_b).unwrap()
        });

        for (face_indices, face_name) in faces {
            let v0 = verts[face_indices[0]].0;
            let v1 = verts[face_indices[1]].0;
            let v2 = verts[face_indices[2]].0;
            let v3 = verts[face_indices[3]].0;

            let cross = (v1.x - v0.x) * (v2.y - v1.y) - (v1.y - v0.y) * (v2.x - v1.x);
            if cross > 0.0 {
                continue;
            }

            let corners = [v0, v1, v2, v3];
            
            let bg_col = egui::Color32::from_rgb(tex.base_color[0], tex.base_color[1], tex.base_color[2]);
            painter.add(egui::Shape::convex_polygon(
                corners.to_vec(),
                bg_col,
                egui::Stroke::new(1.0, egui::Color32::from_gray(100)),
            ));

            let face_width = (v1 - v0).length().max(1.0);
            let face_height = (v3 - v0).length().max(1.0);
            
            // 3D Küpün Hangi Yüzünde Olduğumuza Göre Gerçek Scale Değerlerini Al
            // Böylece kamera yakınlaşması (zoom) tile miktarını etkilemez!
            let (obj_scale_x, obj_scale_y) = if face_name.contains("Z") {
                (cube_scale_x, cube_scale_y) // Ön/Arka yüzeyler X (genişlik) ve Y (yükseklik) eksenine yayılır
            } else if face_name.contains("X") {
                (cube_scale_z, cube_scale_y) // Sol/Sağ yüzeyler Z (genişlik) ve Y (yükseklik) eksenine yayılır
            } else if face_name.contains("Y") {
                (cube_scale_x, cube_scale_z) // Üst/Alt yüzeyler X (genişlik) ve Z (yükseklik) eksenine yayılır
            } else {
                (1.0, 1.0)
            };

            let auto_scale_x = if auto_tile { obj_scale_x } else { 1.0 };
            let auto_scale_y = if auto_tile { obj_scale_y } else { 1.0 };
            
            let final_uv_scale_x = uv_scale_x * auto_scale_x;
            let final_uv_scale_y = uv_scale_y * auto_scale_y;

            // 3D yüzeye döşenecek harf sayısını (Grid çözünürlüğünü) doku tekrar sayısıyla (uv_scale) çarp.
            let grid_cols = ((composed.width as f32 * final_uv_scale_x).max(1.0)) as usize;
            let grid_rows = ((composed.height as f32 * final_uv_scale_y).max(1.0)) as usize;

            // Gerçek 3D Mesh Kaplaması (Egui Font Atlas UV Mapping)
            let mut mesh = egui::Mesh::with_texture(egui::TextureId::Managed(0));

            for ty in 0..grid_rows {
                for tx in 0..grid_cols {
                    // Hücrenin poligon köşelerini hesapla (Bilinear Interpolation)
                    let u_min = tx as f32 / grid_cols as f32;
                    let u_max = (tx + 1) as f32 / grid_cols as f32;
                    let v_min = ty as f32 / grid_rows as f32;
                    let v_max = (ty + 1) as f32 / grid_rows as f32;

                    let top_min = v0.lerp(v1, u_min);
                    let top_max = v0.lerp(v1, u_max);
                    let bot_min = v3.lerp(v2, u_min);
                    let bot_max = v3.lerp(v2, u_max);

                    let p_tl = top_min.lerp(bot_min, v_min);
                    let p_tr = top_max.lerp(bot_max, v_min);
                    let p_bl = top_min.lerp(bot_min, v_max);
                    let p_br = top_max.lerp(bot_max, v_max);

                    // UV Ölçeklendirmesini de Auto-Tile oranında uygula ki doku sündürülmesin, REPEAT etsin!
                    if let Some((ch, rgb, alpha, _cell_height)) = sample_cell(composed, tx as u32, ty as u32, grid_cols as u32, grid_rows as u32, [final_uv_scale_x, final_uv_scale_y], [uv_offset_x, uv_offset_y], None) {
                        if ch == ' ' { continue; } // Boş karakterleri atla
                        
                        let size = (face_width / grid_cols as f32).min(face_height / grid_rows as f32).clamp(2.0, 64.0);
                        let font = egui::FontId::new(size, egui::FontFamily::Monospace);
                        let mut col = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                        if alpha < 1.0 { col = col.linear_multiply(alpha); }
                        
                        let galley = ui.ctx().fonts(|f| f.layout_no_wrap(ch.to_string(), font, col));
                        let cell_size = galley.rect.size();
                        let cell_min = galley.rect.min;
                        
                        let shape = egui::Shape::galley(egui::Pos2::ZERO, galley, col);
                        
                        // Egui'nin gizli motorunu zorla çalıştır, bize kusursuz bir 2D Mesh versin!
                        let primitives = ui.ctx().tessellate(vec![egui::epaint::ClippedShape {
                            clip_rect: egui::Rect::EVERYTHING,
                            shape,
                        }], 1.0);
                        
                        for primitive in primitives {
                            if let egui::epaint::Primitive::Mesh(cell_mesh) = primitive.primitive {
                                // Egui'nin o anki gerçek Font Atlas Texture ID'sini kopyala
                                mesh.texture_id = cell_mesh.texture_id;
                                
                                let start_idx = mesh.vertices.len() as u32;
                                
                                for mut vertex in cell_mesh.vertices {
                                    // Karakteri hücrenin merkezine hizalayarak oranla (Fontun en/boy oranı korunur, esneme yapmaz)
                                    let fx = (vertex.pos.x - cell_min.x + (size - cell_size.x) / 2.0) / size;
                                    let fy = (vertex.pos.y - cell_min.y + (size - cell_size.y) / 2.0) / size;
                                    
                                    // Gerçek 3D küp yüzeyinde bu % oranının denk geldiği koordinatı bul (Bilinear)
                                    let top = p_tl.lerp(p_tr, fx);
                                    let bot = p_bl.lerp(p_br, fx);
                                    let final_pos = top.lerp(bot, fy);
                                    
                                    vertex.pos = final_pos;
                                    mesh.vertices.push(vertex);
                                }
                                
                                mesh.indices.extend(cell_mesh.indices.into_iter().map(|i| i + start_idx));
                            }
                        }
                    }
                }
            }
            // Tüm eğik karakterleri gerçek bir 3D Mesh olarak çiz
            painter.add(egui::Shape::mesh(mesh));
        }
    } else {
        // 2D Modları (0, 1, 2)
        for y in 0..composed.height {
            for x in 0..composed.width {
                let cx = rect.min.x + x as f32 * cell_w;
                let cy = rect.min.y + y as f32 * cell_h;
                let cell_rect = egui::Rect::from_min_size(
                    egui::pos2(cx, cy),
                    egui::vec2(cell_w, cell_h),
                );

                if preview_mode == 1 {
                    let h = composed.height_map[y as usize][x as usize];
                    let t = (h / max_height).clamp(0.0, 1.0);
                    let v = (t * 255.0) as u8;
                    painter.rect_filled(cell_rect, 0.0, egui::Color32::from_gray(v));
                }
                
                if preview_mode == 2 {
                    let border = x == 0
                        || y == 0
                        || x == composed.width - 1
                        || y == composed.height - 1;
                    let bg = if border {
                        egui::Color32::from_rgba_unmultiplied(80, 80, 120, 40)
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    painter.rect_stroke(cell_rect, 0.0, egui::Stroke::new(0.5, bg));
                }

                if preview_mode == 0 {
                    let cell = composed.cell(x, y);
                    if cell.visible {
                        let cx_center = rect.min.x + (x as f32 + 0.5) * cell_w;
                        let cy_center = rect.min.y + (y as f32 + 0.5) * cell_h;
                        let font_size = (cell_w.min(cell_h) * 0.9).clamp(8.0, 48.0);
                        let font_id = egui::FontId::new(font_size, egui::FontFamily::Monospace);
                        let mut color = egui::Color32::from_rgb(cell.fg[0], cell.fg[1], cell.fg[2]);
                        if cell.alpha < 1.0 {
                            color = color.linear_multiply(cell.alpha);
                        }

                        if cell.emission > 0.0 {
                            let glow = color.linear_multiply(0.4);
                            painter.circle_filled(
                                egui::pos2(cx_center, cy_center),
                                cell.emission * 3.0,
                                glow,
                            );
                        }
                        if cell.height > 0.0 {
                            let off = cell.height * 0.8;
                            painter.text(
                                egui::pos2(cx_center + off, cy_center + off),
                                egui::Align2::CENTER_CENTER,
                                cell.ch.to_string(),
                                font_id.clone(),
                                egui::Color32::from_black_alpha(80),
                            );
                        }
                        painter.text(
                            egui::pos2(cx_center, cy_center),
                            egui::Align2::CENTER_CENTER,
                            cell.ch.to_string(),
                            font_id,
                            color,
                        );
                    }
                }
            }
        }

        if preview_mode == 2 {
            for layer in &tex.layers {
                if !layer.is_visible {
                    continue;
                }
                let tw = layer.uv_scale[0].max(1.0);
                let th = layer.uv_scale[1].max(1.0);
                let mut tx = 0.0_f32;
                while tx < grid_w {
                    let mut ty = 0.0_f32;
                    while ty < grid_h {
                        let r = egui::Rect::from_min_size(
                            egui::pos2(rect.min.x + tx * cell_w, rect.min.y + ty * cell_h),
                            egui::vec2(tw * cell_w, th * cell_h),
                        );
                        painter.rect_stroke(
                            r,
                            0.0,
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 180, 80)),
                        );
                        ty += th;
                    }
                    tx += tw;
                }
            }
        }
    }

    ui.separator();
    ui.horizontal(|ui| {
        ui.label(format!(
            "Çözünürlük: {}×{} | Katman: {} | Max Height: {:.1}",
            composed.width,
            composed.height,
            tex.layers.len(),
            max_height
        ));
    });
}