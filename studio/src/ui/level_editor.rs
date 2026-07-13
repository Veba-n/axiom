use eframe::egui;
use crate::app::AxiomStudio;
use crate::data::level::{GameLevel, ObjectInstance};

pub fn show(app: &mut AxiomStudio, ctx: &egui::Context) {
    // --- SOL PANEL: KÜTÜPHANE & HARİTALAR ---
    egui::SidePanel::left("level_library_panel").resizable(true).min_width(200.0).show(ctx, |ui| {
        ui.heading("Haritalar (Levels)");
        if ui.button("➕ Yeni Harita (Level) Ekle").clicked() {
            app.levels.push(GameLevel::default());
            app.active_level_index = Some(app.levels.len() - 1);
        }
        
        let mut to_remove_level = None;
        egui::ScrollArea::vertical().id_source("levels_scroll").max_height(150.0).show(ui, |ui| {
            for (i, lvl) in app.levels.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    let is_sel = app.active_level_index == Some(i);
                    if ui.selectable_label(is_sel, &lvl.name).clicked() { app.active_level_index = Some(i); }
                    if ui.button("🗑").clicked() { to_remove_level = Some(i); }
                });
            }
        });
        if let Some(idx) = to_remove_level { app.levels.remove(idx); app.active_level_index = None; }
        
        ui.separator();
        
        ui.heading("📦 Obje Kütüphanesi");
        ui.label("Sahneye (Grid) eklemek için aşağıdaki objelerden birine tıklayın.");
        
        egui::ScrollArea::vertical().id_source("library_scroll").show(ui, |ui| {
            for obj in &app.objects {
                ui.group(|ui| {
                    ui.label(format!("ID: {}", obj.id));
                    ui.label(format!("İsim: {}", obj.name));
                    if ui.button("Sahneye Ekle (Spawn)").clicked() {
                        if let Some(l_idx) = app.active_level_index {
                            let inst = ObjectInstance {
                                instance_id: format!("Inst_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()),
                                object_id: obj.id.clone(),
                                name_override: obj.name.clone(),
                                world_pos: [0.0, 0.0, 0.0],
                                world_rot: [0.0, 0.0, 0.0],
                                world_scale: [1.0, 1.0, 1.0],
                                param_overrides: std::collections::HashMap::new(),
                                health_override: None,
                                mass_override: None,
                                ai_behavior_override: None,
                                light_intensity_override: None,
                                light_color_override: None,
                            };
                            app.levels[l_idx].instances.push(inst);
                            // Seçimi bu objeye kaydır
                            app.selected_index = Some(app.levels[l_idx].instances.len() - 1);
                        }
                    }
                });
            }
            if app.objects.is_empty() {
                ui.label("Kütüphane boş. Önce 'Oyun Objeleri' sekmesinden obje üretin.");
            }
        });
    });

    // --- SAĞ PANEL: SAHNE HİYERARŞİSİ VE INSPECTOR ---
    egui::SidePanel::right("level_inspector_panel").resizable(true).min_width(250.0).show(ctx, |ui| {
        ui.heading("Sahne (Scene) Hiyerarşisi");
        
        if let Some(l_idx) = app.active_level_index {
            if l_idx >= app.levels.len() { return; }
            let mut lvl = app.levels[l_idx].clone();
            
            ui.horizontal(|ui| { ui.label("Harita Adı:"); ui.text_edit_singleline(&mut lvl.name); });
            ui.collapsing("Harita Ayarları", |ui| {
                ui.horizontal(|ui| { ui.label("Yerçekimi:"); ui.add(egui::DragValue::new(&mut lvl.gravity[0])); ui.add(egui::DragValue::new(&mut lvl.gravity[1])); ui.add(egui::DragValue::new(&mut lvl.gravity[2])); });
                ui.horizontal(|ui| { ui.label("Ortam Işığı:"); ui.color_edit_button_srgb(&mut lvl.ambient_light); });
            });
            
            ui.separator();
            ui.heading("Sahnedeki Objeler");
            
            let mut inst_to_rem = None;
            egui::ScrollArea::vertical().id_source("scene_hierarchy").max_height(200.0).show(ui, |ui| {
                for (i, inst) in lvl.instances.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let is_sel = app.selected_index == Some(i);
                        if ui.selectable_label(is_sel, format!("{} ({})", inst.name_override, inst.object_id)).clicked() {
                            app.selected_index = Some(i);
                        }
                        if ui.button("🗑").clicked() { inst_to_rem = Some(i); }
                    });
                }
            });
            if let Some(idx) = inst_to_rem { lvl.instances.remove(idx); app.selected_index = None; }
            
            ui.separator();
            
            if let Some(s_idx) = app.selected_index {
                if s_idx < lvl.instances.len() {
                        ui.heading("Seçili Instance (Yerleşim) Ayarları");
                        let inst = &mut lvl.instances[s_idx];
                        
                        ui.horizontal(|ui| { ui.label("İsim:"); ui.text_edit_singleline(&mut inst.name_override); });
                        ui.label(format!("Referans Obje ID: {}", inst.object_id));
                        
                        ui.group(|ui| {
                            ui.label("🌍 World Transform (Dünya Koordinatları)");
                            ui.horizontal(|ui| { ui.label("Pos:"); ui.add(egui::DragValue::new(&mut inst.world_pos[0]).speed(0.1)); ui.add(egui::DragValue::new(&mut inst.world_pos[1]).speed(0.1)); ui.add(egui::DragValue::new(&mut inst.world_pos[2]).speed(0.1)); });
                            ui.horizontal(|ui| { ui.label("Scl:"); ui.add(egui::DragValue::new(&mut inst.world_scale[0]).speed(0.1)); ui.add(egui::DragValue::new(&mut inst.world_scale[1]).speed(0.1)); ui.add(egui::DragValue::new(&mut inst.world_scale[2]).speed(0.1)); });
                            ui.horizontal(|ui| { ui.label("Rot:"); ui.add(egui::DragValue::new(&mut inst.world_rot[0]).speed(1.0)); ui.add(egui::DragValue::new(&mut inst.world_rot[1]).speed(1.0)); ui.add(egui::DragValue::new(&mut inst.world_rot[2]).speed(1.0)); });
                        });
                        
                        ui.group(|ui| {
                            ui.label("🧮 Obje Parametrelerini Ez (Param Overrides)");
                            ui.label("Bu spesifik kopya için base objenin matematiksel parametrelerini buradan ezebilirsiniz (Örn: bu kopyanın kapı uzunluğu farklı olsun).");
                            
                            // Find base object
                            let mut base_params = None;
                            for obj in &app.objects {
                                if obj.id == inst.object_id {
                                    base_params = Some(obj.parameters.clone());
                                    break;
                                }
                            }
                            
                            if let Some(params) = base_params {
                                if params.is_empty() {
                                    ui.label(egui::RichText::new("Bu objede tanımlanmış hiçbir parametre yok.").italics());
                                } else {
                                    let mut keys: Vec<_> = params.keys().cloned().collect();
                                    keys.sort();
                                    
                                    for key in keys {
                                        ui.horizontal(|ui| {
                                            ui.label(format!("{}:", key));
                                            
                                            // Check if it's overridden
                                            let mut is_overridden = inst.param_overrides.contains_key(&key);
                                            
                                            if ui.checkbox(&mut is_overridden, "Ez (Override)").changed() {
                                                if is_overridden {
                                                    inst.param_overrides.insert(key.clone(), *params.get(&key).unwrap());
                                                } else {
                                                    inst.param_overrides.remove(&key);
                                                }
                                            }
                                            
                                            if is_overridden {
                                                let mut val = *inst.param_overrides.get(&key).unwrap();
                                                ui.add(egui::DragValue::new(&mut val).speed(0.1));
                                                inst.param_overrides.insert(key.clone(), val);
                                            } else {
                                                let val = params.get(&key).unwrap();
                                                ui.label(format!("{} (Orijinal)", val));
                                            }
                                        });
                                    }
                                }
                            } else {
                                ui.label(egui::RichText::new("Uyarı: Orijinal obje kütüphanede bulunamadı!").color(egui::Color32::RED));
                            }

                        ui.group(|ui| {
                            ui.label("⚙️ Ana Özellikleri Ez (Core Overrides)");
                            ui.label("Sadece bu kopyanın (Instance) can, kütle, yapay zeka veya ışık ayarlarını değiştirebilirsiniz.");
                            
                            // Health
                            let mut has_health = inst.health_override.is_some();
                            ui.horizontal(|ui| {
                                if ui.checkbox(&mut has_health, "Can (Health)").changed() {
                                    if has_health { inst.health_override = Some(100.0); } else { inst.health_override = None; }
                                }
                                if let Some(ref mut h) = inst.health_override { ui.add(egui::DragValue::new(h)); }
                            });
                            
                            // Mass
                            let mut has_mass = inst.mass_override.is_some();
                            ui.horizontal(|ui| {
                                if ui.checkbox(&mut has_mass, "Kütle (Mass)").changed() {
                                    if has_mass { inst.mass_override = Some(1.0); } else { inst.mass_override = None; }
                                }
                                if let Some(ref mut m) = inst.mass_override { ui.add(egui::DragValue::new(m).speed(0.1)); }
                            });
                            
                            // Light
                            let mut has_light = inst.light_intensity_override.is_some();
                            ui.horizontal(|ui| {
                                if ui.checkbox(&mut has_light, "Işık Şiddeti (Intensity)").changed() {
                                    if has_light { inst.light_intensity_override = Some(5.0); } else { inst.light_intensity_override = None; }
                                }
                                if let Some(ref mut l) = inst.light_intensity_override { ui.add(egui::DragValue::new(l).speed(0.1)); }
                            });
                            
                            // Light Color
                            let mut has_color = inst.light_color_override.is_some();
                            ui.horizontal(|ui| {
                                if ui.checkbox(&mut has_color, "Işık Rengi (Color)").changed() {
                                    if has_color { inst.light_color_override = Some([255, 255, 255]); } else { inst.light_color_override = None; }
                                }
                                if let Some(ref mut c) = inst.light_color_override { ui.color_edit_button_srgb(c); }
                            });
                        });

                        });
                    }
                }
            app.levels[l_idx] = lvl;
        } else {
            ui.label("Önce sol menüden bir Harita (Level) seçin veya oluşturun.");
        }
    });

    // --- ORTA PANEL: 3D GRID & VOXEL EDITOR ---
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Level Editor (Voxel & 3D Harita Tasarımcısı)");
        ui.label("Sahnede kamerayı döndürmek için tıklayıp sürükleyin.");
        
        let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::drag());
        if response.dragged() {
            let drag = response.drag_delta();
            app.camera_rot[1] += drag.x * 0.5; // Yaw
            app.camera_rot[0] += drag.y * 0.5; // Pitch
        }
        
        let cam_rot_x = app.camera_rot[0];
        let cam_rot_y = app.camera_rot[1];
        let rect = response.rect;
        let center = rect.center();
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(15, 15, 20));
        
        let is_ortho = app.camera_ortho;
        let cx = if is_ortho { 30.0_f32.to_radians() } else { cam_rot_x.to_radians() };
        let cy = if is_ortho { 45.0_f32.to_radians() } else { cam_rot_y.to_radians() };
        
        let project_3d = |x: f32, y: f32, z: f32| -> egui::Pos2 {
            let x1 = x * cy.cos() + z * cy.sin();
            let z1 = -x * cy.sin() + z * cy.cos();
            let y2 = y * cx.cos() - z1 * cx.sin();
            center + egui::vec2(x1 * 30.0, y2 * -30.0) // Biraz daha uzak kamera (30.0 scale)
        };
        
        // Zemin Grid'i (Büyük Harita Grid'i)
        for i in -10..=10 {
            let offset = i as f32 * 2.0;
            let c = if i == 0 { egui::Color32::from_rgb(100, 100, 120) } else { egui::Color32::from_rgb(40, 40, 50) };
            painter.line_segment([project_3d(-20.0, 0.0, offset), project_3d(20.0, 0.0, offset)], (1.0, c));
            painter.line_segment([project_3d(offset, 0.0, -20.0), project_3d(offset, 0.0, 20.0)], (1.0, c));
        }

        // Sahnedeki obje örneklerini çiz
        if let Some(l_idx) = app.active_level_index {
            if l_idx < app.levels.len() {
                let lvl = &app.levels[l_idx];
                for (i, inst) in lvl.instances.iter().enumerate() {
                    let is_sel = app.selected_index == Some(i);
                    let color = if is_sel { egui::Color32::YELLOW } else { egui::Color32::from_rgb(100, 200, 255) };
                    
                    let p = project_3d(inst.world_pos[0], inst.world_pos[1], inst.world_pos[2]);
                    
                    // Simple Cube Representation for now
                    let size = inst.world_scale[0] * 5.0;
                    painter.circle_filled(p, size, color);
                    
                    if is_sel {
                        painter.circle_stroke(p, size + 2.0, (2.0, egui::Color32::WHITE));
                        // Draw XYZ Gizmo for selected instance in world space
                        painter.line_segment([p, project_3d(inst.world_pos[0] + 2.0, inst.world_pos[1], inst.world_pos[2])], (2.0, egui::Color32::RED));
                        painter.line_segment([p, project_3d(inst.world_pos[0], inst.world_pos[1] + 2.0, inst.world_pos[2])], (2.0, egui::Color32::GREEN));
                        painter.line_segment([p, project_3d(inst.world_pos[0], inst.world_pos[1], inst.world_pos[2] + 2.0)], (2.0, egui::Color32::BLUE));
                    }
                }
            }
        }
    });
}
