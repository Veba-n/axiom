use eframe::egui;
use crate::app::AxiomStudio;
use crate::data::object::{GameObject, PrimitiveShape, BooleanOp, ModifierType, ObjectPart, FaceMaterial, Bone, ColliderType, PivotMode, ObjectSocket, ParticleEmitter, default_mesh_part};
use crate::render::object_viewport::{self, FACE_SLOTS, ShadingMode, parse_shading, sync_texture_cache};

pub fn draw_outliner_tree(
    ui: &mut egui::Ui,
    parts: &[ObjectPart],
    parent_id: Option<&String>,
    app_selected_part_id: &mut Option<String>,
    part_to_remove: &mut Option<String>,
    move_up: &mut Option<usize>,
    move_down: &mut Option<usize>,
    vis_changes: &mut Vec<(usize, bool)>,
    part_to_dup: &mut Option<String>,
    move_target: &mut Option<(String, Option<String>)>
) {
    for (p_idx, part) in parts.iter().enumerate() {
        if part.parent_part_id.as_ref() == parent_id {
            let has_children = parts.iter().any(|p| p.parent_part_id.as_ref() == Some(&part.id));
            
            ui.horizontal(|ui| {
                let c_id = ui.make_persistent_id(&part.id);
                let mut is_open = ui.data_mut(|d| d.get_temp(c_id).unwrap_or(true));
                
                if has_children {
                    let icon_btn = if is_open { "▼" } else { "▶" };
                    // Set font size for small button
                    if ui.button(egui::RichText::new(icon_btn).size(10.0)).clicked() {
                        is_open = !is_open;
                        ui.data_mut(|d| d.insert_temp(c_id, is_open));
                    }
                } else {
                    ui.label(egui::RichText::new("  ").size(10.0));
                }
                
                let icon = if part.shape == crate::data::object::PrimitiveShape::EmptyGroup { "📁" } else { "🧊" };
                let is_sel = app_selected_part_id.as_ref() == Some(&part.id);
                let op_icon = match part.boolean_op { crate::data::object::BooleanOp::Add => "+", crate::data::object::BooleanOp::Subtract => "-", crate::data::object::BooleanOp::Intersect => "∩" };
                
                // Rich Text for better readability
                let mut label_text = egui::RichText::new(format!("{} {} ({})", icon, part.name, op_icon)).size(14.0);
                if is_sel { label_text = label_text.strong().color(egui::Color32::WHITE); }
                
                let label_resp = ui.selectable_label(is_sel, label_text);
                if label_resp.clicked() {
                    *app_selected_part_id = Some(part.id.clone());
                }
                
                let drag_id = egui::Id::new("dragged_part");
                if label_resp.drag_started() {
                    ui.data_mut(|d| d.insert_temp(drag_id, part.id.clone()));
                }
                
                if let Some(dragged_id) = ui.data(|d| d.get_temp::<String>(drag_id)) {
                    if dragged_id != part.id {
                        if label_resp.hovered() {
                            ui.painter().rect_stroke(label_resp.rect, 2.0, (1.0, egui::Color32::YELLOW));
                            if ui.input(|i| i.pointer.any_released()) {
                                *move_target = Some((dragged_id.clone(), Some(part.id.clone())));
                            }
                        }
                    }
                }
                
                let mut vis = part.is_visible;
                if ui.checkbox(&mut vis, "👁").changed() {
                    vis_changes.push((p_idx, vis));
                }
                
                if ui.button("↑").clicked() && p_idx > 0 { *move_up = Some(p_idx); }
                if ui.button("↓").clicked() && p_idx < parts.len() - 1 { *move_down = Some(p_idx); }
                
                if ui.button("📄").on_hover_text("Kopyala (Duplicate)").clicked() {
                    *part_to_dup = Some(part.id.clone());
                }
                if ui.button("🗑").clicked() {
                    *part_to_remove = Some(part.id.clone());
                }
            });
            
            if has_children {
                let c_id = ui.make_persistent_id(&part.id);
                let is_open = ui.data_mut(|d| d.get_temp(c_id).unwrap_or(true));
                if is_open {
                    ui.indent(&part.id, |ui| {
                        draw_outliner_tree(ui, parts, Some(&part.id), app_selected_part_id, part_to_remove, move_up, move_down, vis_changes, part_to_dup, move_target);
                    });
                }
            }
        }
    }
}


pub fn show(app: &mut AxiomStudio, ctx: &egui::Context) {
    // --- SOL PANEL: EXPLORER ---
    egui::SidePanel::left("object_explorer_panel").resizable(true).min_width(200.0).show(ctx, |ui| {
        ui.heading("Objeler (Explorer)");
        ui.separator();
        
        ui.horizontal(|ui| {
            if ui.button("➕ Yeni Obje Ekle").clicked() {
                app.objects.push(GameObject::default());
                app.selected_index = Some(app.objects.len() - 1);
            }
        });
        
        ui.separator();
        
        let mut to_remove = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, obj) in app.objects.iter().enumerate() {
                ui.horizontal(|ui| {
                    let is_selected = app.selected_index == Some(i);
                    if ui.selectable_label(is_selected, &obj.name).clicked() {
                        app.selected_index = Some(i);
                    }
                    if ui.button("🗑").clicked() { to_remove = Some(i); }
                });
            }
        });
        
        if let Some(idx) = to_remove {
            app.objects.remove(idx);
            app.selected_index = None;
        }

        ui.separator();
        
        if let Some(sel_idx) = app.selected_index {
            if sel_idx < app.objects.len() {
                ui.heading("Hiyerarşi (Outliner)");
                ui.label("Parçalar ve Gruplar:");
                let mut obj = app.objects[sel_idx].clone();
                
                ui.horizontal(|ui| {
                    if ui.button("➕ Grup").clicked() {
                        let count = obj.parts.len();
                        obj.parts.push(crate::data::object::ObjectPart {
                            id: format!("Group_{}", count + 1), name: format!("Group_{}", count + 1), local_parameters: std::collections::HashMap::new(),
                            shape: crate::data::object::PrimitiveShape::EmptyGroup, boolean_op: crate::data::object::BooleanOp::Add, csg_target_id: None, bone_id: None, parent_part_id: None,
                            pos: [0.0, 0.0, 0.0], scale: [1.0, 1.0, 1.0], rot: [0.0, 0.0, 0.0],
                            pos_expr: ["".into(), "".into(), "".into()], scale_expr: ["".into(), "".into(), "".into()], rot_expr: ["".into(), "".into(), "".into()],
                            array_count_expr: "".into(), array_offset_expr: ["".into(), "".into(), "".into()],
                            modifiers: vec![], faces: std::collections::HashMap::new(), is_visible: true, pivot_mode: crate::data::object::PivotMode::Center, pivot_offset: [0.0, 0.0, 0.0],
                            shading_model: "Textured".into(), mirror_x: false, mirror_y: false, mirror_z: false,
                            collider_type: crate::data::object::ColliderType::None, lod_hide_distance: 0.0,
                        });
                        app.selected_part_id = Some(format!("Group_{}", count + 1));
                    }
                    if ui.button("➕ Mesh").clicked() {
                        let count = obj.parts.len();
                        let id = format!("Part_{}", count + 1);
                        obj.parts.push(default_mesh_part(&id, &format!("Mesh_{}", count + 1), PrimitiveShape::Cube));
                        app.selected_part_id = Some(id);
                    }
                });
                
                let mut part_to_remove = None;
                let mut part_to_dup = None;
                let mut move_up = None;
                let mut move_down = None;
                let mut move_target = None;
                
                let outliner_resp = egui::ScrollArea::vertical().id_source("outliner_scroll").show(ui, |ui| {
                    let parts = obj.parts.clone();
                    let mut vis_changes = Vec::new();
                    draw_outliner_tree(ui, &parts, None, &mut app.selected_part_id, &mut part_to_remove, &mut move_up, &mut move_down, &mut vis_changes, &mut part_to_dup, &mut move_target);
                    for (idx, vis) in vis_changes {
                        obj.parts[idx].is_visible = vis;
                    }
                    ui.allocate_space(ui.available_size()); // fill remaining space to allow dropping at root
                });
                
                let drag_id = egui::Id::new("dragged_part");
                if let Some(dragged_id) = ui.data(|d| d.get_temp::<String>(drag_id)) {
                    if ui.rect_contains_pointer(outliner_resp.inner_rect) && ui.input(|i| i.pointer.any_released()) && move_target.is_none() {
                        move_target = Some((dragged_id, None));
                    }
                    if ui.input(|i| i.pointer.any_released()) {
                        ui.data_mut(|d| d.remove::<String>(drag_id));
                    }
                }
                
                if let Some((dragged_id, new_parent)) = move_target {
                    // Check cyclic dependency to prevent infinite loops (child cannot be parent of its parent)
                    let mut is_cyclic = false;
                    let mut curr_p = new_parent.clone();
                    while let Some(pid) = curr_p {
                        if pid == dragged_id { is_cyclic = true; break; }
                        if let Some(p) = obj.parts.iter().find(|x| x.id == pid) { curr_p = p.parent_part_id.clone(); } else { break; }
                    }
                    
                    if !is_cyclic {
                        if let Some(p) = obj.parts.iter_mut().find(|x| x.id == dragged_id) {
                            p.parent_part_id = new_parent;
                        }
                    }
                }
                
                if let Some(idx) = move_up { obj.parts.swap(idx, idx - 1); }
                if let Some(idx) = move_down { obj.parts.swap(idx, idx + 1); }
                
                if let Some(ref rem_id) = part_to_remove {
                    obj.parts.retain(|p| &p.id != rem_id);
                    if app.selected_part_id.as_ref() == Some(rem_id) { app.selected_part_id = None; }
                }
                
                if let Some(ref dup_id) = part_to_dup {
                    if let Some(part_to_clone) = obj.parts.iter().find(|p| &p.id == dup_id) {
                        let mut clone = part_to_clone.clone();
                        clone.id = format!("P_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
                        clone.name = format!("{}_Kopya", clone.name);
                        obj.parts.push(clone);
                    }
                }
                
                app.objects[sel_idx] = obj;
            }
        }
    });

    // --- SAĞ PANEL: INSPECTOR (Seçili Obje Ayarları) ---
    egui::SidePanel::right("object_inspector_panel").resizable(true).min_width(300.0).show(ctx, |ui| {
        ui.heading("Obje Denetleyici");
        ui.label("Parça geometrisi, yüzey dokuları ve fizik ayarları.");
        ui.separator();
        
        let sel_idx = app.selected_index.unwrap_or(0);
        if sel_idx >= app.objects.len() { return; }
        
        let mut obj = app.objects[sel_idx].clone(); 
        
        egui::ScrollArea::both().show(ui, |ui| {
            ui.group(|ui| {
                ui.horizontal(|ui| { ui.label("ID:"); ui.text_edit_singleline(&mut obj.id); ui.label("İsim:"); ui.text_edit_singleline(&mut obj.name); });
                

                // --- 0. PARAMETRİK DEĞİŞKENLER ---
                ui.collapsing("Parametreler", |ui| {
                    ui.label("Formüllerde kullanılacak sayısal değişkenler (ör. genişlik = 2.0)");
                    if ui.button("➕ Add Variable").clicked() {
                        obj.parameters.insert(format!("var_{}", obj.parameters.len()), 1.0);
                    }
                    let mut p_rem = None;
                    let keys: Vec<String> = obj.parameters.keys().cloned().collect();
                    for key in keys {
                        ui.horizontal(|ui| {
                            let mut new_key = key.clone();
                            ui.text_edit_singleline(&mut new_key);
                            
                            let mut val = *obj.parameters.get(&key).unwrap();
                            ui.add(egui::DragValue::new(&mut val).speed(0.1));
                            
                            if new_key != key {
                                obj.parameters.remove(&key);
                                obj.parameters.insert(new_key.clone(), val);
                                return;
                            } else {
                                obj.parameters.insert(key.clone(), val);
                            }
                            if ui.button("🗑").clicked() { p_rem = Some(key.clone()); }
                        });
                    }
                    if let Some(r) = p_rem { obj.parameters.remove(&r); }
                });


                ui.collapsing("🌍 Global Dönüşüm (Tüm Grubu Düzenle)", |ui| {
                    ui.label("Tüm parçaları aynı anda taşır, büyütür veya döndürür.");
                    ui.horizontal(|ui| {
                        ui.label("Global Pos:"); ui.add(egui::DragValue::new(&mut obj.global_pos[0]).speed(0.1)); ui.add(egui::DragValue::new(&mut obj.global_pos[1]).speed(0.1)); ui.add(egui::DragValue::new(&mut obj.global_pos[2]).speed(0.1));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Pos Expr:"); ui.add(egui::TextEdit::singleline(&mut obj.global_pos_expr[0]).hint_text("X")); ui.add(egui::TextEdit::singleline(&mut obj.global_pos_expr[1]).hint_text("Y")); ui.add(egui::TextEdit::singleline(&mut obj.global_pos_expr[2]).hint_text("Z"));
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Global Scl:"); ui.add(egui::DragValue::new(&mut obj.global_scale[0]).speed(0.1)); ui.add(egui::DragValue::new(&mut obj.global_scale[1]).speed(0.1)); ui.add(egui::DragValue::new(&mut obj.global_scale[2]).speed(0.1));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Scl Expr:"); ui.add(egui::TextEdit::singleline(&mut obj.global_scale_expr[0]).hint_text("X")); ui.add(egui::TextEdit::singleline(&mut obj.global_scale_expr[1]).hint_text("Y")); ui.add(egui::TextEdit::singleline(&mut obj.global_scale_expr[2]).hint_text("Z"));
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Global Rot:"); ui.add(egui::DragValue::new(&mut obj.global_rot[0]).speed(1.0)); ui.add(egui::DragValue::new(&mut obj.global_rot[1]).speed(1.0)); ui.add(egui::DragValue::new(&mut obj.global_rot[2]).speed(1.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Rot Expr:"); ui.add(egui::TextEdit::singleline(&mut obj.global_rot_expr[0]).hint_text("X")); ui.add(egui::TextEdit::singleline(&mut obj.global_rot_expr[1]).hint_text("Y")); ui.add(egui::TextEdit::singleline(&mut obj.global_rot_expr[2]).hint_text("Z"));
                    });
                });

                // --- 1. MESH & PARÇALAR ---

                ui.collapsing("3D Mesh", |ui| {
                    if ui.button("➕ Geometri Ekle").clicked() {
                        let id = format!("P_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
                        obj.parts.push(default_mesh_part(&id, &format!("Mesh_{}", obj.parts.len() + 1), PrimitiveShape::Cube));
                    }
                    
                    let parts_list = obj.parts.clone();
                    
                    let mut p_rem = None;
                    let mut p_dup = None; // Dublication Queue
                    // let mut to_clone_part = None;
                    if let Some(ref sel_part_id) = app.selected_part_id {
                        if let Some((p_idx, part)) = obj.parts.iter_mut().enumerate().find(|(_, p)| &p.id == sel_part_id) {
                            ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.text_edit_singleline(&mut part.name);
                                egui::ComboBox::from_id_source(format!("shape_{}", p_idx))
                                    .selected_text(format!("{:?}", part.shape))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut part.shape, PrimitiveShape::Cube, "Küp");
                                        ui.selectable_value(&mut part.shape, PrimitiveShape::Sphere, "Küre");
                                        ui.selectable_value(&mut part.shape, PrimitiveShape::Pyramid, "Piramit");
                                        ui.selectable_value(&mut part.shape, PrimitiveShape::Cylinder, "Silindir");
                                        ui.selectable_value(&mut part.shape, PrimitiveShape::HalfCylinder, "Yarım Silindir");
                                        ui.selectable_value(&mut part.shape, PrimitiveShape::TriangularPrism, "Üçgen Prizma");
                                        ui.selectable_value(&mut part.shape, PrimitiveShape::PentagonPrism, "Beşgen Prizma");
                                        ui.selectable_value(&mut part.shape, PrimitiveShape::Torus, "Torus (Halka)");
                                        ui.selectable_value(&mut part.shape, PrimitiveShape::HexagonPrism, "Altıgen Prizma");
                                        ui.selectable_value(&mut part.shape, PrimitiveShape::Cone, "Koni");
                                        ui.selectable_value(&mut part.shape, PrimitiveShape::CustomMesh, "Özel Mesh (OBJ)");
                                    });
                                egui::ComboBox::from_id_source(format!("csg_{}", p_idx))
                                    .selected_text(match part.boolean_op { BooleanOp::Add => "Ekle (+)", BooleanOp::Subtract => "Çıkar / Oyuk Aç (-)", BooleanOp::Intersect => "Kesişim", })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut part.boolean_op, BooleanOp::Add, "Ekle (+)");
                                        ui.selectable_value(&mut part.boolean_op, BooleanOp::Subtract, "Çıkar / Oyuk Aç (-)");
                                        ui.selectable_value(&mut part.boolean_op, BooleanOp::Intersect, "Kesişim");
                                    });
                                    
                                if part.boolean_op != BooleanOp::Add {
                                    ui.horizontal(|ui| {
                                        ui.label("CSG Hedefi:");
                                        let mut tgt = "Yok (Tüm Objeye Etki)".to_string();
                                        if let Some(tid) = &part.csg_target_id {
                                            tgt = tid.clone(); // Fallback if name not found
                                            for op in &parts_list {
                                                if &op.id == tid {
                                                    tgt = op.name.clone();
                                                    break;
                                                }
                                            }
                                        }
                                        egui::ComboBox::from_id_source(format!("csg_tgt_{}", p_idx))
                                            .selected_text(&tgt)
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(&mut part.csg_target_id, None, "Tümü (Tüm Objeye Etki)");
                                                for op in &parts_list {
                                                    if op.id != part.id {
                                                        ui.selectable_value(&mut part.csg_target_id, Some(op.id.clone()), op.name.clone());
                                                    }
                                                }
                                            });
                                    });
                                }
                                if ui.button("Kopyala").clicked() { 
                                    p_dup = Some(part.clone());
                                }
                                if ui.button("🗑").clicked() { p_rem = Some(p_idx); }
                            });
                            
                            ui.horizontal(|ui| {
                                ui.label("Bağlı Olduğu Üst Parça (Group Parent):");
                                let cur_p = part.parent_part_id.clone().unwrap_or("Yok".into());
                                egui::ComboBox::from_id_source(format!("parent_p_{}", p_idx))
                                    .selected_text(&cur_p)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut part.parent_part_id, None, "Yok");
                                        for op in &parts_list {
                                            if op.id != part.id {
                                                ui.selectable_value(&mut part.parent_part_id, Some(op.id.clone()), op.name.clone());
                                            }
                                        }
                                    });
                            });
                            ui.collapsing("📐 Dinamik Matematiksel Formüller (Expressions)", |ui| {
                                ui.label("Eğer kutular doluysa, normal X, Y, Z değerleri yok sayılır ve matematiksel denklem çalışır.");
                                ui.label("Örn: 'kapi_uzunluk * 2' veya '15.5 + kapi_uzunluk'");
                                ui.horizontal(|ui| { ui.label("Pos Expr:"); ui.add(egui::TextEdit::singleline(&mut part.pos_expr[0]).hint_text("X")); ui.add(egui::TextEdit::singleline(&mut part.pos_expr[1]).hint_text("Y")); ui.add(egui::TextEdit::singleline(&mut part.pos_expr[2]).hint_text("Z")); });
                                        ui.horizontal(|ui| { ui.label("Scl Expr:"); ui.add(egui::TextEdit::singleline(&mut part.scale_expr[0]).hint_text("X")); ui.add(egui::TextEdit::singleline(&mut part.scale_expr[1]).hint_text("Y")); ui.add(egui::TextEdit::singleline(&mut part.scale_expr[2]).hint_text("Z")); });
                                        ui.horizontal(|ui| { ui.label("Rot Expr:"); ui.add(egui::TextEdit::singleline(&mut part.rot_expr[0]).hint_text("X")); ui.add(egui::TextEdit::singleline(&mut part.rot_expr[1]).hint_text("Y")); ui.add(egui::TextEdit::singleline(&mut part.rot_expr[2]).hint_text("Z")); });
                            });
                            ui.collapsing("🔄 Çoklu Dizi (Array Modifier - Pattern)", |ui| {
                                ui.horizontal(|ui| { ui.label("Tekrar Sayısı (Formül):"); ui.text_edit_singleline(&mut part.array_count_expr); });
                                ui.horizontal(|ui| { ui.label("Kayma Expr (Offset):"); ui.text_edit_singleline(&mut part.array_offset_expr[0]); ui.text_edit_singleline(&mut part.array_offset_expr[1]); ui.text_edit_singleline(&mut part.array_offset_expr[2]); });
                            });
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut part.is_visible, "Görünür");
                                ui.checkbox(&mut part.mirror_x, "Ayna X");
                                ui.checkbox(&mut part.mirror_y, "Ayna Y");
                                ui.checkbox(&mut part.mirror_z, "Ayna Z");
                                egui::ComboBox::from_id_source(format!("collider_{}", p_idx))
                                    .selected_text(format!("{:?}", part.collider_type))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut part.collider_type, ColliderType::None, "Yok (Hayalet)");
                                        ui.selectable_value(&mut part.collider_type, ColliderType::Box, "Kutu Çarpışma");
                                        ui.selectable_value(&mut part.collider_type, ColliderType::Sphere, "Küre Çarpışma");
                                        ui.selectable_value(&mut part.collider_type, ColliderType::Capsule, "Kapsül");
                                        ui.selectable_value(&mut part.collider_type, ColliderType::Mesh, "Tam Mesh (Ağır)");
                                    });
                                ui.label("LOD (Kapanma Mesafesi):");
                                ui.add(egui::DragValue::new(&mut part.lod_hide_distance).speed(10.0));
                                egui::ComboBox::from_id_source(format!("shading_{}", p_idx))
                                    .selected_text(match part.shading_model.as_str() {
                                        "Wireframe" => "Tel kafes",
                                        "Solid" | "Flat" | "Smooth" => "Düz renk",
                                        "Textured" | "Lit" => "Dokulu",
                                        other => other,
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut part.shading_model, "Textured".into(), "Dokulu — materyal editöründeki doku görünür");
                                        ui.selectable_value(&mut part.shading_model, "Solid".into(), "Düz renk — sadece tint");
                                        ui.selectable_value(&mut part.shading_model, "Wireframe".into(), "Tel kafes — sadece kenarlar");
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.label("Boyut (Kalınlık/Hacim/Uzunluk):");
                                ui.add(egui::DragValue::new(&mut part.scale[0]).speed(0.1).prefix("X:"));
                                ui.add(egui::DragValue::new(&mut part.scale[1]).speed(0.1).prefix("Y:"));
                                ui.add(egui::DragValue::new(&mut part.scale[2]).speed(0.1).prefix("Z:"));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Konum:"); ui.add(egui::DragValue::new(&mut part.pos[0]).speed(0.1)); ui.add(egui::DragValue::new(&mut part.pos[1]).speed(0.1)); ui.add(egui::DragValue::new(&mut part.pos[2]).speed(0.1));
                                ui.label("Açı:"); ui.add(egui::DragValue::new(&mut part.rot[0]).speed(1.0)); ui.add(egui::DragValue::new(&mut part.rot[1]).speed(1.0)); ui.add(egui::DragValue::new(&mut part.rot[2]).speed(1.0));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Rotasyon Çapası (Pivot Anchor):");
                                egui::ComboBox::from_id_source(format!("pivot_{}", p_idx))
                                    .selected_text(match part.pivot_mode { PivotMode::Center => "Merkez (Center)", PivotMode::CustomOffset(_) => "Özel Nokta (Offset)", PivotMode::EdgeMinX => "Kenar: -X (Sol)", PivotMode::EdgeMaxX => "Kenar: +X (Sağ)", PivotMode::EdgeMinY => "Kenar: -Y (Alt)", PivotMode::EdgeMaxY => "Kenar: +Y (Üst)", PivotMode::EdgeMinZ => "Kenar: -Z (Arka)", PivotMode::EdgeMaxZ => "Kenar: +Z (Ön)", })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut part.pivot_mode, PivotMode::Center, "Merkez (Center)");
                                        ui.selectable_value(&mut part.pivot_mode, PivotMode::CustomOffset([0.0, 0.0, 0.0]), "Özel Nokta (Offset)");
                                        ui.selectable_value(&mut part.pivot_mode, PivotMode::EdgeMinX, "Kenar: -X (Sol)");
                                        ui.selectable_value(&mut part.pivot_mode, PivotMode::EdgeMaxX, "Kenar: +X (Sağ)");
                                        ui.selectable_value(&mut part.pivot_mode, PivotMode::EdgeMinY, "Kenar: -Y (Alt / Menteşe)");
                                        ui.selectable_value(&mut part.pivot_mode, PivotMode::EdgeMaxY, "Kenar: +Y (Üst / Tavan Kapak)");
                                        ui.selectable_value(&mut part.pivot_mode, PivotMode::EdgeMinZ, "Kenar: -Z (Arka)");
                                        ui.selectable_value(&mut part.pivot_mode, PivotMode::EdgeMaxZ, "Kenar: +Z (Ön)");
                                    });
                                
                                if let PivotMode::CustomOffset(ref mut off) = part.pivot_mode {
                                    ui.add(egui::DragValue::new(&mut off[0]).speed(0.1).prefix("X:"));
                                    ui.add(egui::DragValue::new(&mut off[1]).speed(0.1).prefix("Y:"));
                                    ui.add(egui::DragValue::new(&mut off[2]).speed(0.1).prefix("Z:"));
                                }
                            });
                            
                            // YÜZEY VE MATERYALLER
                            ui.collapsing("Yüzey Dokuları", |ui| {
                                ui.label("Her yüzeye ayrı materyal atayabilirsiniz. Ortadaki 3D görünümde anında yansır — ayrı pencere gerekmez.");
                                if app.textures.is_empty() {
                                    ui.colored_label(egui::Color32::YELLOW, "Henüz materyal yok. Üst menüden \"Doku & Materyal\" moduna geçip materyal oluşturun.");
                                }
                                ui.horizontal(|ui| {
                                    if ui.button("+ Yüzey Ekle").clicked() {
                                        let slot = FACE_SLOTS
                                            .iter()
                                            .find(|s| !part.faces.contains_key(**s))
                                            .unwrap_or(&"All");
                                        part.faces.insert(slot.to_string(), FaceMaterial::default());
                                    }
                                    if part.faces.is_empty() {
                                        if ui.button("Varsayılan (Tüm Yüzeyler)").clicked() {
                                            part.faces.insert("All".into(), FaceMaterial::default());
                                        }
                                    }
                                });
                                let mut face_to_rem = None;
                                let faces_keys: Vec<String> = part.faces.keys().cloned().collect();
                                for key in faces_keys {
                                    let mut mat = part.faces.get(&key).unwrap().clone();
                                    let mut effective_key = key.clone();
                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label("Yüzey:");
                                            let mut slot = effective_key.clone();
                                            egui::ComboBox::from_id_source(format!("fslot_{}_{}", p_idx, key))
                                                .selected_text(&slot)
                                                .show_ui(ui, |ui| {
                                                    for s in FACE_SLOTS {
                                                        ui.selectable_value(&mut slot, s.to_string(), *s);
                                                    }
                                                });
                                            if slot != effective_key {
                                                part.faces.remove(&effective_key);
                                                effective_key = slot;
                                            }
                                            if ui.button("🗑").clicked() {
                                                face_to_rem = Some(effective_key.clone());
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Materyal:");
                                            let sel_label = if mat.texture_id.is_empty() {
                                                "Seçilmedi".to_string()
                                            } else {
                                                app.textures
                                                    .iter()
                                                    .find(|t| t.id == mat.texture_id)
                                                    .map(|t| format!("{} ({})", t.name, t.id))
                                                    .unwrap_or_else(|| format!("{} (bulunamadı)", mat.texture_id))
                                            };
                                            egui::ComboBox::from_id_source(format!("ftex_{}_{}", p_idx, key))
                                                .selected_text(&sel_label)
                                                .width(220.0)
                                                .show_ui(ui, |ui| {
                                                    ui.selectable_value(&mut mat.texture_id, "".into(), "Seçilmedi");
                                                    for tex in &app.textures {
                                                        ui.selectable_value(
                                                            &mut mat.texture_id,
                                                            tex.id.clone(),
                                                            format!("{} — {}", tex.name, tex.id),
                                                        );
                                                    }
                                                });
                                            ui.label("Renk tonu:");
                                            ui.color_edit_button_srgb(&mut mat.tint);
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("UV kaydırma:");
                                            ui.add(egui::DragValue::new(&mut mat.uv_offset[0]).speed(0.1));
                                            ui.add(egui::DragValue::new(&mut mat.uv_offset[1]).speed(0.1));
                                            ui.label("UV ölçek:");
                                            ui.add(egui::DragValue::new(&mut mat.uv_scale[0]).speed(0.1).clamp_range(0.1..=16.0));
                                            ui.add(egui::DragValue::new(&mut mat.uv_scale[1]).speed(0.1).clamp_range(0.1..=16.0));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.checkbox(&mut mat.auto_tile, "Boyutla Orantılı Döşe (Auto-Tile)")
                                                .on_hover_text("Açıkken obje büyüdükçe dokunun sündürülmesini engeller ve döşemeyi artırır (World Space UV).");
                                        });
                                        ui.horizontal(|ui| {
                                            ui.checkbox(&mut mat.use_custom_bg, "Özel Arkaplan Rengi");
                                            if mat.use_custom_bg {
                                                ui.color_edit_button_srgb(&mut mat.background_color);
                                            }
                                            ui.label("Opaklık:");
                                            ui.add(egui::Slider::new(&mut mat.opacity, 0.0..=1.0));
                                        });
                                        if !mat.texture_id.is_empty() {
                                            if let Some(tex) = app.textures.iter().find(|t| t.id == mat.texture_id) {
                                                ui.label(format!(
                                                    "Önizleme: {}×{} ızgara, {} katman",
                                                    tex.resolution[0],
                                                    tex.resolution[1],
                                                    tex.layers.len()
                                                ));
                                            }
                                        }
                                    });
                                    part.faces.insert(effective_key, mat);
                                }
                                if let Some(rk) = face_to_rem {
                                    part.faces.remove(&rk);
                                }
                            });

                            // DEFORMASYONLAR (MODIFIERS)
                            ui.collapsing("🌀 Deformasyon Modifiers (Eğip Bükme)", |ui| {
                                ui.horizontal(|ui| {
                                    if ui.button("+ Eğme/Kaydırma (Shear)").clicked() { part.modifiers.push(ModifierType::Shear([0.0,0.0,0.0])); }
                                    if ui.button("+ Bükme (Bend)").clicked() { part.modifiers.push(ModifierType::Bend(0.0)); }
                                    if ui.button("+ Daraltma (Taper)").clicked() { part.modifiers.push(ModifierType::Taper(0.0)); }
                                    if ui.button("+ Gürültü (Noise)").clicked() { part.modifiers.push(ModifierType::Noise([1.0, 0.5])); }
                                });
                                let mut m_rem = None;
                                for (m_idx, modif) in part.modifiers.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        match modif {
                                            ModifierType::Shear(v) => { ui.label("Shear (Kaydırma):"); ui.add(egui::DragValue::new(&mut v[0]).prefix("X:")); ui.add(egui::DragValue::new(&mut v[1]).prefix("Y:")); ui.add(egui::DragValue::new(&mut v[2]).prefix("Z:")); },
                                            ModifierType::Bend(a) => { ui.label("Bend (Bükme Açısı):"); ui.add(egui::DragValue::new(a)); },
                                            ModifierType::Taper(t) => { ui.label("Taper (Daraltma Miktarı):"); ui.add(egui::DragValue::new(t)); },
                                            ModifierType::Noise(v) => { ui.label("Noise (Ölçek, Yoğunluk):"); ui.add(egui::DragValue::new(&mut v[0])); ui.add(egui::DragValue::new(&mut v[1])); },
                                        }
                                        if ui.button("🗑").clicked() { m_rem = Some(m_idx); }
                                    });
                                }
                                if let Some(r) = m_rem { part.modifiers.remove(r); }
                            });
                        });
                    }
                    }
                    if let Some(r) = p_rem { obj.parts.remove(r); }
                    if let Some(mut clone) = p_dup {
                        clone.id = format!("P_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
                        clone.name = format!("{}_Kopya", clone.name);
                        obj.parts.push(clone);
                    }
                });

                // --- 2. RIGGING (KEMİKLER) ---
                ui.collapsing("İskelet (Kemikler)", |ui| {
                    if ui.button("➕ Kemik (Bone) Ekle").clicked() {
                        obj.bones.push(Bone { id: format!("Bone_{}", obj.bones.len() + 1), parent_id: None, local_pos: [0.0, 0.0, 0.0], local_rot: [0.0, 0.0, 0.0], lock_x: false, lock_y: false, lock_z: false });
                    }
                    let mut b_rem = None;
                    for (b_idx, bone) in obj.bones.iter_mut().enumerate() {
                        ui.group(|ui| {
                            ui.horizontal(|ui| { ui.label("Kemik ID:"); ui.text_edit_singleline(&mut bone.id); if ui.button("🗑").clicked() { b_rem = Some(b_idx); }});
                            ui.horizontal(|ui| { ui.label("IK Constraint (Kilitle):"); ui.checkbox(&mut bone.lock_x, "X Kilit"); ui.checkbox(&mut bone.lock_y, "Y Kilit"); ui.checkbox(&mut bone.lock_z, "Z Kilit"); });
                        });
                    }
                    if let Some(r) = b_rem { obj.bones.remove(r); }
                });

                // --- 3. SOKETLER (Equipment) ---
                ui.collapsing("Bağlantı Noktaları (Soketler)", |ui| {
                    ui.label("Silah, ekipman veya alt parça takmak için bağlantı noktaları.");
                    if ui.button("➕ Yeni Soket Ekle").clicked() {
                        obj.sockets.push(ObjectSocket { id: format!("Socket_{}", obj.sockets.len() + 1), name: "Yeni Soket".into(), bone_id: None, local_pos: [0.0, 0.0, 0.0], local_rot: [0.0, 0.0, 0.0], local_scale: [1.0, 1.0, 1.0] });
                    }
                    let mut s_rem = None;
                    for (s_idx, s) in obj.sockets.iter_mut().enumerate() {
                        ui.horizontal(|ui| { ui.text_edit_singleline(&mut s.name); ui.label("Pos:"); ui.add(egui::DragValue::new(&mut s.local_pos[0])); ui.add(egui::DragValue::new(&mut s.local_pos[1])); if ui.button("🗑").clicked() { s_rem = Some(s_idx); } });
                    }
                    if let Some(r) = s_rem { obj.sockets.remove(r); }
                });

                // --- 4. VFX & PARTİKÜLLER ---
                ui.collapsing("Partikül Efektleri", |ui| {
                    if ui.button("➕ Ateş/Duman (Emitter) Ekle").clicked() {
                        obj.emitters.push(ParticleEmitter { id: format!("Vfx_{}", obj.emitters.len() + 1), name: "Ateş".into(), parent_part_id: None, local_pos: [0.0, 0.0, 0.0], emit_rate: 10.0, particle_color: [255, 100, 0] });
                    }
                    let mut v_rem = None;
                    for (v_idx, v) in obj.emitters.iter_mut().enumerate() {
                        ui.horizontal(|ui| { ui.text_edit_singleline(&mut v.name); ui.color_edit_button_srgb(&mut v.particle_color); if ui.button("🗑").clicked() { v_rem = Some(v_idx); } });
                    }
                    if let Some(r) = v_rem { obj.emitters.remove(r); }
                });
            });
        });
        
        app.objects[sel_idx] = obj; 
    });

    // --- ORTA PANEL: VIEWPORT VE KAMERA ---
    egui::CentralPanel::default().show(ctx, |ui| {
        let _frame_profiler_start = std::time::Instant::now();
        ui.heading("3D Görünüm");
        ui.label("Dokular bu panelde canlı gösterilir. Sol tık: döndür · Orta/sağ tık: kaydır · Tekerlek: yakınlaştır");
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut app.camera_ortho, "İzometrik Sabit Kamera (Kilitli)");
            ui.checkbox(&mut app.show_gizmo, "XYZ Referans Kollarını (Gizmo) Göster");
            
            let mut show_uv_dots = ui.data_mut(|d| d.get_temp(egui::Id::new("show_uv_dots")).unwrap_or(false));
            if ui.checkbox(&mut show_uv_dots, "Hücre Noktalarını (UV Dots) Göster").changed() {
                ui.data_mut(|d| d.insert_temp(egui::Id::new("show_uv_dots"), show_uv_dots));
            }

            ui.menu_button("🛠 Hata Ayıklama (Debug)", |ui| {
                let mut show_outer_shell = ui.data_mut(|d| d.get_temp(egui::Id::new("show_outer_shell")).unwrap_or(false));
                if ui.checkbox(&mut show_outer_shell, "Dış Kabuğu (Air Contact) Göster").changed() {
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("show_outer_shell"), show_outer_shell));
                }

                let mut debug_wireframe_only = ui.data_mut(|d| d.get_temp(egui::Id::new("debug_wireframe_only")).unwrap_or(false));
                if ui.checkbox(&mut debug_wireframe_only, "Sadece Tel Kafesleri Göster (Kaplamaları Gizle)").changed() {
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("debug_wireframe_only"), debug_wireframe_only));
                }

                let mut texture_debug = ui.data_mut(|d| d.get_temp(egui::Id::new("texture_debug")).unwrap_or(false));
                if ui.checkbox(&mut texture_debug, "Texture Debug Modu").changed() {
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("texture_debug"), texture_debug));
                }

                let mut show_gpu_triangles = ui.data_mut(|d| d.get_temp(egui::Id::new("show_gpu_triangles")).unwrap_or(false));
                if ui.checkbox(&mut show_gpu_triangles, "GPU Üçgenlerini (Triangulation) Göster").changed() {
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("show_gpu_triangles"), show_gpu_triangles));
                }

                let mut debug_culling = ui.data_mut(|d| d.get_temp(egui::Id::new("debug_culling")).unwrap_or(false));
                if ui.checkbox(&mut debug_culling, "Kırpılan Yüzeyleri (Culled) Kırmızı Göster").changed() {
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("debug_culling"), debug_culling));
                }

                let mut debug_normals = ui.data_mut(|d| d.get_temp(egui::Id::new("debug_normals")).unwrap_or(false));
                if ui.checkbox(&mut debug_normals, "Yüzey Normallerini Çiz").changed() {
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("debug_normals"), debug_normals));
                }

                let mut debug_inner_walls = ui.data_mut(|d| d.get_temp(egui::Id::new("debug_inner_walls")).unwrap_or(false));
                if ui.checkbox(&mut debug_inner_walls, "İç Duvarları (Inner Walls) Pembe Göster").changed() {
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("debug_inner_walls"), debug_inner_walls));
                }
                
                let mut debug_z_depth = ui.data_mut(|d| d.get_temp(egui::Id::new("debug_z_depth")).unwrap_or(false));
                if ui.checkbox(&mut debug_z_depth, "Z-Derinlik (Depth) Değerlerini Göster").changed() {
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("debug_z_depth"), debug_z_depth));
                }

                let mut debug_depth_color = ui.data_mut(|d| d.get_temp(egui::Id::new("debug_depth_color")).unwrap_or(false));
                if ui.checkbox(&mut debug_depth_color, "Kameraya Uzaklığa Göre Renklendir (Depth Shading)").changed() {
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("debug_depth_color"), debug_depth_color));
                }

                let mut debug_coplanar = ui.data_mut(|d| d.get_temp(egui::Id::new("debug_coplanar")).unwrap_or(false));
                if ui.checkbox(&mut debug_coplanar, "Coplanar (Aynı Düzlem) Yüzeyleri Renklendir").changed() {
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("debug_coplanar"), debug_coplanar));
                }
                
                let mut debug_draw_order = ui.data_mut(|d| d.get_temp(egui::Id::new("debug_draw_order")).unwrap_or(false));
                if ui.checkbox(&mut debug_draw_order, "Çizim Sırasını (Painter Index) Göster").changed() {
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("debug_draw_order"), debug_draw_order));
                }
                
                let mut debug_labels = ui.data_mut(|d| d.get_temp(egui::Id::new("debug_labels")).unwrap_or(false));
                if ui.checkbox(&mut debug_labels, "Parça ve Yüzey ID Etiketlerini Göster").changed() {
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("debug_labels"), debug_labels));
                }
                
                let mut debug_wireframe = ui.data_mut(|d| d.get_temp(egui::Id::new("debug_wireframe")).unwrap_or(false));
                if ui.checkbox(&mut debug_wireframe, "Tüm Yüzeylerin Tel Kafesini (Wireframe) Çiz").changed() {
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("debug_wireframe"), debug_wireframe));
                }
            });

            if ui.button("Kamerayı Sıfırla (Reset View)").clicked() {
                app.camera_rot[0] = 30.0;
                app.camera_rot[1] = 45.0;
                app.camera_pan = [0.0, 0.0];
                app.camera_zoom = 40.0;
            }
        });
        
        let sense = egui::Sense::drag();
        let (response, painter) = ui.allocate_painter(ui.available_size(), sense);
        
        if response.hovered() {
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll != 0.0 {
                app.camera_zoom = (app.camera_zoom + scroll * 0.1).clamp(5.0, 500.0);
            }
        }
        
        if response.dragged_by(egui::PointerButton::Primary) {
            let drag = response.drag_delta();
            app.camera_rot[1] += drag.x * 0.5; // Yaw
            app.camera_rot[0] += drag.y * 0.5; // Pitch
        } else if response.dragged_by(egui::PointerButton::Middle) || response.dragged_by(egui::PointerButton::Secondary) {
            let drag = response.drag_delta();
            app.camera_pan[0] += drag.x;
            app.camera_pan[1] += drag.y;
        }
        
        let cam_rot_x = app.camera_rot[0];
        let cam_rot_y = app.camera_rot[1];
        
        let rect = response.rect;
        let center = rect.center();
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(10, 10, 12));
        
        // Gelişmiş Kamera Projeksiyonu
        let is_ortho = app.camera_ortho;
        let cx = if is_ortho { 30.0_f32.to_radians() } else { cam_rot_x.to_radians() };
        let cy = if is_ortho { 45.0_f32.to_radians() } else { cam_rot_y.to_radians() };
        
        // ÖN-HESAPLAMA: Her vertex için sin/cos hesaplamak CPU'yu mahvediyordu!
        let cx_sin = cx.sin(); let cx_cos = cx.cos();
        let cy_sin = cy.sin(); let cy_cos = cy.cos();
        
        let project_3d_with_z = |x: f32, y: f32, z: f32| -> (egui::Pos2, f32) {
            // Y ekseni etrafında (Yaw)
            let x1 = x * cy_cos + z * cy_sin;
            let z1 = -x * cy_sin + z * cy_cos;
            // X ekseni etrafında (Pitch)
            let y2 = y * cx_cos - z1 * cx_sin;
            let z2 = y * cx_sin + z1 * cx_cos; // Derinlik (Z-sorting için aktif!)
            
            (center + egui::vec2(x1 * app.camera_zoom + app.camera_pan[0], y2 * -app.camera_zoom + app.camera_pan[1]), z2)
        };

        let project_3d = |x: f32, y: f32, z: f32| -> egui::Pos2 {
            project_3d_with_z(x, y, z).0
        };

        // Kırmızı (X), Yeşil (Y), Mavi (Z) Eksen Çizgileri
        let show_gizmo = app.show_gizmo;
        if show_gizmo {
            painter.line_segment([project_3d(0.0, 0.0, 0.0), project_3d(2.0, 0.0, 0.0)], (2.0, egui::Color32::RED));
            painter.line_segment([project_3d(0.0, 0.0, 0.0), project_3d(0.0, 2.0, 0.0)], (2.0, egui::Color32::GREEN));
            painter.line_segment([project_3d(0.0, 0.0, 0.0), project_3d(0.0, 0.0, 2.0)], (2.0, egui::Color32::from_rgb(100, 150, 255))); // Blue/Z
        }
        
        // Zemin Grid'i
        for i in -5..=5 {
            let offset = i as f32;
            painter.line_segment([project_3d(-5.0, 0.0, offset), project_3d(5.0, 0.0, offset)], (1.0, egui::Color32::from_rgb(40, 40, 50)));
            painter.line_segment([project_3d(offset, 0.0, -5.0), project_3d(offset, 0.0, 5.0)], (1.0, egui::Color32::from_rgb(40, 40, 50)));
        }

        let t0 = std::time::Instant::now();
        let sel_idx = app.selected_index.unwrap_or(0);
        if sel_idx < app.objects.len() {
            let obj = &app.objects[sel_idx];
            // PERFORMANS: Eskiden burada TÜM dokular her frame sıfırdan
            // compose() ediliyordu. Artık kalıcı app.texture_cache sadece
            // gerçekten değişen dokuları yeniden hesaplıyor; değişmeyenler
            // için ucuz bir hash kontrolüyle anında geçiliyor.
            sync_texture_cache(ui.ctx(), &mut app.texture_cache, &app.textures);
            
            
            // --- Hiyerarşi (Parenting) Çözümleyici ---
            // Parçaların pozisyonlarını ve rotasyonlarını üst parçalara (Parent) göre kümülatif hesapla.
            
            type EvalDepsType = (
                Vec<crate::data::object::ObjectPart>,
                std::collections::HashMap<String, f32>,
                [f32; 3], [f32; 3], [f32; 3],
                [String; 3], [String; 3], [String; 3]
            );
            let eval_deps: EvalDepsType = (
                obj.parts.clone(),
                obj.parameters.clone(),
                obj.global_pos, obj.global_rot, obj.global_scale,
                obj.global_pos_expr.clone(), obj.global_rot_expr.clone(), obj.global_scale_expr.clone(),
            );
            
            let eval_cache_id = ui.make_persistent_id(&format!("eval_cache_v2_{}", obj.id));
            let eval_deps_id = ui.make_persistent_id(&format!("eval_deps_{}", obj.id));
            
            type EvalCacheType = (f32, f32, f32, f32, f32, f32, f32, f32, f32, std::collections::HashMap<String, (f32, f32, f32, f32, f32, f32, f32, f32, f32)>, std::collections::HashMap<String, (f32, f32, f32, f32, f32, f32, f32, f32, f32)>);

            let (gx, gy, gz, gs_x, gs_y, gs_z, gr_x, gr_y, gr_z, evaluated_parts, resolved_transforms) = ui.data(|d| {
                if let (Some(old_deps), Some(cached_data)) = (
                    d.get_temp::<EvalDepsType>(eval_deps_id),
                    d.get_temp::<EvalCacheType>(eval_cache_id)
                ) {
                    if old_deps == eval_deps { return Some(cached_data); }
                }
                None
            }).unwrap_or_else(|| {
                let mut eval_map = std::collections::BTreeMap::new();
                for (k, v) in &obj.parameters {
                    eval_map.insert(k.clone(), *v as f64);
                }
                
                let eval_expr = |expr: &str, default_val: f32, map: &mut std::collections::BTreeMap<String, f64>| -> f32 {
                    if expr.trim().is_empty() { return default_val; }
                    match fasteval::ez_eval(expr, map) {
                        Ok(val) => val as f32,
                        Err(_) => default_val,
                    }
                };
                
                let gx = eval_expr(&obj.global_pos_expr[0], obj.global_pos[0], &mut eval_map);
                let gy = eval_expr(&obj.global_pos_expr[1], obj.global_pos[1], &mut eval_map);
                let gz = eval_expr(&obj.global_pos_expr[2], obj.global_pos[2], &mut eval_map);
                
                let gs_x = eval_expr(&obj.global_scale_expr[0], obj.global_scale[0], &mut eval_map);
                let gs_y = eval_expr(&obj.global_scale_expr[1], obj.global_scale[1], &mut eval_map);
                let gs_z = eval_expr(&obj.global_scale_expr[2], obj.global_scale[2], &mut eval_map);
                
                let gr_x = eval_expr(&obj.global_rot_expr[0], obj.global_rot[0], &mut eval_map);
                let gr_y = eval_expr(&obj.global_rot_expr[1], obj.global_rot[1], &mut eval_map);
                let gr_z = eval_expr(&obj.global_rot_expr[2], obj.global_rot[2], &mut eval_map);
                
                let mut evaluated_parts = std::collections::HashMap::new();
                for part in &obj.parts {
                    let mut local_map = eval_map.clone();
                    for (k, v) in &part.local_parameters { local_map.insert(k.clone(), *v as f64); }

                    let px = eval_expr(&part.pos_expr[0], part.pos[0], &mut local_map);
                    let py = eval_expr(&part.pos_expr[1], part.pos[1], &mut local_map);
                    let pz = eval_expr(&part.pos_expr[2], part.pos[2], &mut local_map);
                    
                    let sx = eval_expr(&part.scale_expr[0], part.scale[0], &mut local_map);
                    let sy = eval_expr(&part.scale_expr[1], part.scale[1], &mut local_map);
                    let sz = eval_expr(&part.scale_expr[2], part.scale[2], &mut local_map);
                    
                    let rx = eval_expr(&part.rot_expr[0], part.rot[0], &mut local_map);
                    let ry = eval_expr(&part.rot_expr[1], part.rot[1], &mut local_map);
                    let rz = eval_expr(&part.rot_expr[2], part.rot[2], &mut local_map);
                    
                    evaluated_parts.insert(part.id.clone(), (px, py, pz, sx, sy, sz, rx, ry, rz));
                }
                
                let mut resolved_transforms: std::collections::HashMap<String, (f32, f32, f32, f32, f32, f32, f32, f32, f32)> = std::collections::HashMap::new();
                
                for _ in 0..10 {
                    for part in &obj.parts {
                        if resolved_transforms.contains_key(&part.id) { continue; }
                        
                        if let Some(parent_id) = &part.parent_part_id {
                            if let Some(parent_t) = resolved_transforms.get(parent_id) {
                                let (ppx, ppy, ppz, prx, pry, prz, psx, psy, psz) = parent_t;
                                let px_rad = prx.to_radians(); let py_rad = pry.to_radians(); let pz_rad = prz.to_radians();
                                
                                let (lx, ly, lz, lsx, lsy, lsz, lrx, lry, lrz) = evaluated_parts.get(&part.id).unwrap();
                                
                                let lx = lx * psx; let ly = ly * psy; let lz = lz * psz;
                                
                                let y1 = ly * px_rad.cos() - lz * px_rad.sin(); let z1 = ly * px_rad.sin() + lz * px_rad.cos();
                                let x2 = lx * py_rad.cos() + z1 * py_rad.sin(); let z2 = -lx * py_rad.sin() + z1 * py_rad.cos();
                                let x3 = x2 * pz_rad.cos() - y1 * pz_rad.sin(); let y3 = x2 * pz_rad.sin() + y1 * pz_rad.cos();
                                
                                let final_x = ppx + x3; let final_y = ppy + y3; let final_z = ppz + z2;
                                let final_rx = prx + lrx; let final_ry = pry + lry; let final_rz = prz + lrz;
                                let final_sx = psx * lsx; let final_sy = psy * lsy; let final_sz = psz * lsz;
                                
                                resolved_transforms.insert(part.id.clone(), (final_x, final_y, final_z, final_rx, final_ry, final_rz, final_sx, final_sy, final_sz));
                            }
                        } else {
                            let eval_v = evaluated_parts.get(&part.id).unwrap();
                            resolved_transforms.insert(part.id.clone(), (eval_v.0, eval_v.1, eval_v.2, eval_v.6, eval_v.7, eval_v.8, eval_v.3, eval_v.4, eval_v.5));
                        }
                    }
                }
                
                ui.data_mut(|d| {
                    d.insert_temp(eval_deps_id, eval_deps);
                    d.insert_temp(eval_cache_id, (gx, gy, gz, gs_x, gs_y, gs_z, gr_x, gr_y, gr_z, evaluated_parts.clone(), resolved_transforms.clone()));
                });
                
                (gx, gy, gz, gs_x, gs_y, gs_z, gr_x, gr_y, gr_z, evaluated_parts, resolved_transforms)
            });
            let _t1 = std::time::Instant::now();

            use crate::render::csg::{Plane, make_plane};

struct PolyFace {
    verts: Vec<[f32; 3]>,
    face_id: &'static str,
}

fn get_part_polygons<F>(shape: &PrimitiveShape, mut transform_world: F) -> Vec<PolyFace>
where
    F: FnMut(f32, f32, f32) -> [f32; 3],
{
    let mut faces = Vec::new();
    match shape {
        PrimitiveShape::Cube => {
            let w0 = transform_world(-0.5, -0.5, -0.5);
            let w1 = transform_world(0.5, -0.5, -0.5);
            let w2 = transform_world(0.5, -0.5, 0.5);
            let w3 = transform_world(-0.5, -0.5, 0.5);
            let w4 = transform_world(-0.5, 0.5, -0.5);
            let w5 = transform_world(0.5, 0.5, -0.5);
            let w6 = transform_world(0.5, 0.5, 0.5);
            let w7 = transform_world(-0.5, 0.5, 0.5);
            faces.push(PolyFace { verts: vec![w0, w1, w2, w3], face_id: "-Y" });
            faces.push(PolyFace { verts: vec![w7, w6, w5, w4], face_id: "+Y" });
            faces.push(PolyFace { verts: vec![w4, w5, w1, w0], face_id: "-Z" });
            faces.push(PolyFace { verts: vec![w6, w7, w3, w2], face_id: "+Z" });
            faces.push(PolyFace { verts: vec![w7, w4, w0, w3], face_id: "-X" });
            faces.push(PolyFace { verts: vec![w5, w6, w2, w1], face_id: "+X" });
        },
        PrimitiveShape::Pyramid => {
            let w0 = transform_world(-0.5, -0.5, -0.5); // Bottom Left Back
            let w1 = transform_world(0.5, -0.5, -0.5);  // Bottom Right Back
            let w2 = transform_world(0.5, -0.5, 0.5);   // Bottom Right Front
            let w3 = transform_world(-0.5, -0.5, 0.5);  // Bottom Left Front
            let w4 = transform_world(0.0, 0.5, 0.0);    // Top
            faces.push(PolyFace { verts: vec![w0, w3, w2, w1], face_id: "-Y" });
            faces.push(PolyFace { verts: vec![w1, w0, w4], face_id: "-Z" });
            faces.push(PolyFace { verts: vec![w2, w1, w4], face_id: "+X" });
            faces.push(PolyFace { verts: vec![w3, w2, w4], face_id: "+Z" });
            faces.push(PolyFace { verts: vec![w0, w3, w4], face_id: "-X" });
        },
        PrimitiveShape::Cylinder => {
            let mut top = Vec::new(); let mut bot = Vec::new();
            for i in 0..8 {
                let angle = (i as f32) * std::f32::consts::PI / 4.0;
                let x = angle.cos() * 0.5; let z = angle.sin() * 0.5;
                top.push(transform_world(x, 0.5, z)); bot.push(transform_world(x, -0.5, z));
            }
            let mut top_ccw = top.clone(); top_ccw.reverse();
            faces.push(PolyFace { verts: top_ccw, face_id: "+Y" });
            faces.push(PolyFace { verts: bot.clone(), face_id: "-Y" });
            for i in 0..8 {
                let next = (i + 1) % 8;
                faces.push(PolyFace { verts: vec![top[i], top[next], bot[next], bot[i]], face_id: "Side" });
            }
        },
        PrimitiveShape::HalfCylinder => {
            let mut top = Vec::new(); let mut bot = Vec::new();
            for i in 0..17 {
                let angle = (i as f32) * std::f32::consts::PI / 16.0;
                let x = angle.cos() * 0.5; let z = angle.sin() * 0.5;
                top.push(transform_world(x, 0.5, z)); bot.push(transform_world(x, -0.5, z));
            }
            let mut top_ccw = top.clone(); top_ccw.reverse();
            faces.push(PolyFace { verts: top_ccw, face_id: "+Y" });
            faces.push(PolyFace { verts: bot.clone(), face_id: "-Y" });
            faces.push(PolyFace { verts: vec![bot[0], bot[16], top[16], top[0]], face_id: "FlatSide" });
            for i in 0..16 {
                faces.push(PolyFace { verts: vec![top[i], top[i+1], bot[i+1], bot[i]], face_id: "Curve" });
            }
        },
        PrimitiveShape::TriangularPrism => {
            let mut top = Vec::new(); let mut bot = Vec::new();
            for i in 0..3 {
                let angle = (i as f32) * std::f32::consts::PI * 2.0 / 3.0;
                let x = angle.cos() * 0.5; let z = angle.sin() * 0.5;
                top.push(transform_world(x, 0.5, z)); bot.push(transform_world(x, -0.5, z));
            }
            let mut top_ccw = top.clone(); top_ccw.reverse();
            faces.push(PolyFace { verts: top_ccw, face_id: "+Y" });
            faces.push(PolyFace { verts: bot.clone(), face_id: "-Y" });
            for i in 0..3 {
                let next = (i + 1) % 3;
                faces.push(PolyFace { verts: vec![top[i], top[next], bot[next], bot[i]], face_id: "Side" });
            }
        },
        PrimitiveShape::HexagonPrism => {
            let mut top = Vec::new(); let mut bot = Vec::new();
            for i in 0..6 {
                let angle = (i as f32) * std::f32::consts::PI * 2.0 / 6.0;
                let x = angle.cos() * 0.5; let z = angle.sin() * 0.5;
                top.push(transform_world(x, 0.5, z)); bot.push(transform_world(x, -0.5, z));
            }
            let mut top_ccw = top.clone(); top_ccw.reverse();
            faces.push(PolyFace { verts: top_ccw, face_id: "+Y" });
            faces.push(PolyFace { verts: bot.clone(), face_id: "-Y" });
            for i in 0..6 {
                let next = (i + 1) % 6;
                faces.push(PolyFace { verts: vec![top[i], top[next], bot[next], bot[i]], face_id: "Side" });
            }
        },
        PrimitiveShape::PentagonPrism => {
            let mut top = Vec::new(); let mut bot = Vec::new();
            for i in 0..5 {
                let angle = (i as f32) * std::f32::consts::PI * 2.0 / 5.0;
                let x = angle.cos() * 0.5; let z = angle.sin() * 0.5;
                top.push(transform_world(x, 0.5, z)); bot.push(transform_world(x, -0.5, z));
            }
            let mut top_ccw = top.clone(); top_ccw.reverse();
            faces.push(PolyFace { verts: top_ccw, face_id: "+Y" });
            faces.push(PolyFace { verts: bot.clone(), face_id: "-Y" });
            for i in 0..5 {
                let next = (i + 1) % 5;
                faces.push(PolyFace { verts: vec![top[i], top[next], bot[next], bot[i]], face_id: "Side" });
            }
        },
        PrimitiveShape::Cone => {
            let mut bot = Vec::new();
            let top_pt = transform_world(0.0, 0.5, 0.0);
            for i in 0..32 {
                let angle = (i as f32) * std::f32::consts::PI * 2.0 / 32.0;
                let x = angle.cos() * 0.5; let z = angle.sin() * 0.5;
                bot.push(transform_world(x, -0.5, z));
            }
            faces.push(PolyFace { verts: bot.clone(), face_id: "-Y" });
            for i in 0..32 {
                let next = (i + 1) % 32;
                faces.push(PolyFace { verts: vec![bot[i], bot[next], top_pt], face_id: "Side" });
            }
        },
        PrimitiveShape::Sphere => {
            let mut rings = Vec::new();
            for i in 0..=16 {
                let lat = std::f32::consts::PI / 2.0 - (i as f32) * std::f32::consts::PI / 16.0;
                let r = lat.cos() * 0.5; let y = lat.sin() * 0.5;
                let mut ring = Vec::new();
                for j in 0..32 {
                    let lon = (j as f32) * std::f32::consts::PI * 2.0 / 32.0;
                    ring.push(transform_world(lon.cos() * r, y, lon.sin() * r));
                }
                rings.push(ring);
            }
            for i in 0..16 {
                for j in 0..32 {
                    let nj = (j + 1) % 32;
                    let ni = i + 1;
                    faces.push(PolyFace { verts: vec![rings[i][j], rings[i][nj], rings[ni][nj], rings[ni][j]], face_id: "Sphere" });
                }
            }
        },
        PrimitiveShape::Torus => {
            let major_r = 0.4; let minor_r = 0.1;
            let mut rings = Vec::new();
            for i in 0..16 {
                let u = (i as f32) * std::f32::consts::PI * 2.0 / 16.0;
                let mut ring = Vec::new();
                for j in 0..16 {
                    let v = (j as f32) * std::f32::consts::PI * 2.0 / 16.0;
                    let x = (major_r + minor_r * v.cos()) * u.cos();
                    let y = minor_r * v.sin();
                    let z = (major_r + minor_r * v.cos()) * u.sin();
                    ring.push(transform_world(x, y, z));
                }
                rings.push(ring);
            }
            for i in 0..16 {
                for j in 0..16 {
                    let ni = (i + 1) % 16;
                    let nj = (j + 1) % 16;
                    faces.push(PolyFace { verts: vec![rings[i][j], rings[i][nj], rings[ni][nj], rings[ni][j]], face_id: "Torus" });
                }
            }
        },
        _ => {}
    }
    faces
}

#[derive(Clone)]
struct CachedCSGFace {
    poly_3d: Vec<[f32; 3]>,
    original_3d: Option<Vec<[f32; 3]>>,
    face_id: &'static str,
    part_index: usize,
    shading: ShadingMode,
    wire_stroke: egui::Stroke,
    destroyed_ratio: f32,
    triangles: Vec<usize>,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
struct PlaneKey {
    nx: i32,
    ny: i32,
    nz: i32,
    d: i32,
}

fn get_plane_key(normal: [f32; 3], d: f32) -> PlaneKey {
    let (mut nx, mut ny, mut nz, mut pd) = (normal[0], normal[1], normal[2], d);
    let epsilon = 1e-5;
    let flip = if nx.abs() > epsilon {
        nx < 0.0
    } else if ny.abs() > epsilon {
        ny < 0.0
    } else {
        nz < 0.0
    };
    if flip {
        nx = -nx;
        ny = -ny;
        nz = -nz;
        pd = -pd;
    }
    PlaneKey {
        nx: (nx * 100.0).round() as i32,
        ny: (ny * 100.0).round() as i32,
        nz: (nz * 100.0).round() as i32,
        d: (pd * 100.0).round() as i32,
    }
}

struct FaceToDraw<'a> {
    poly_3d: &'a [[f32; 3]], // SIFIR TAHSİS (Kopya yok)
    original_3d: Option<&'a [[f32; 3]]>,
    z_depth: f32,
    plane_key: Option<PlaneKey>,
    face_id: &'static str,
    part_index: usize,
    part: &'a ObjectPart,
    shading: &'a ShadingMode,
    wire_stroke: egui::Stroke,
    destroyed_ratio: f32,
    triangles: &'a [usize],
    is_culled: bool,
    normal: [f32; 3],
    is_inner_wall: bool,
}
let mut all_faces = Vec::new();
            
            struct Volume<'a> {
                planes: Vec<Plane>,
                part_id: String,
                target_id: Option<String>,
                is_subtract: bool,
                part_ref: &'a ObjectPart,
                aabb: ([f32; 3], [f32; 3]),
            }
            let mut volumes = Vec::new();
            let mut cached_part_faces = std::collections::HashMap::new();

            for part in &obj.parts {
                if !part.is_visible { continue; }
                
                let eval_v = evaluated_parts.get(&part.id).unwrap();
                let (base_x, base_y, base_z, rot_x, rot_y, rot_z, sx, sy, sz) = *resolved_transforms.get(&part.id).unwrap_or(&(eval_v.0, eval_v.1, eval_v.2, eval_v.6, eval_v.7, eval_v.8, eval_v.3, eval_v.4, eval_v.5));
                let mut shear_x = 0.0; let mut shear_y = 0.0; let mut shear_z = 0.0;
                for modif in &part.modifiers {
                    if let ModifierType::Shear(v) = modif { shear_x += v[0]; shear_y += v[1]; shear_z += v[2]; }
                }
                
                let actual_pivot = match part.pivot_mode {
                    PivotMode::Center => [0.0, 0.0, 0.0],
                    PivotMode::CustomOffset(off) => off,
                    PivotMode::EdgeMinX => [-sx / 2.0, 0.0, 0.0],
                    PivotMode::EdgeMaxX => [sx / 2.0, 0.0, 0.0],
                    PivotMode::EdgeMinY => [0.0, -sy / 2.0, 0.0],
                    PivotMode::EdgeMaxY => [0.0, sy / 2.0, 0.0],
                    PivotMode::EdgeMinZ => [0.0, 0.0, -sz / 2.0],
                    PivotMode::EdgeMaxZ => [0.0, 0.0, sz / 2.0],
                };

                let rx_rad = rot_x.to_radians(); let ry_rad = rot_y.to_radians(); let rz_rad = rot_z.to_radians();
                let rx_sin = rx_rad.sin(); let rx_cos = rx_rad.cos();
                let ry_sin = ry_rad.sin(); let ry_cos = ry_rad.cos();
                let rz_sin = rz_rad.sin(); let rz_cos = rz_rad.cos();

                let rotate = |vx: f32, vy: f32, vz: f32| -> (f32, f32, f32) {
                    let ox = vx - actual_pivot[0]; let oy = vy - actual_pivot[1]; let oz = vz - actual_pivot[2];
                    
                    let y1 = oy * rx_cos - oz * rx_sin; let z1 = oy * rx_sin + oz * rx_cos;
                    let x2 = ox * ry_cos + z1 * ry_sin; let z2 = -ox * ry_sin + z1 * ry_cos;
                    let x3 = x2 * rz_cos - y1 * rz_sin; let y3 = x2 * rz_sin + y1 * rz_cos;
                    
                    (x3 + actual_pivot[0], y3 + actual_pivot[1], z2 + actual_pivot[2])
                };

                let grx_rad = gr_x.to_radians(); let gry_rad = gr_y.to_radians(); let grz_rad = gr_z.to_radians();
                let grx_sin = grx_rad.sin(); let grx_cos = grx_rad.cos();
                let gry_sin = gry_rad.sin(); let gry_cos = gry_rad.cos();
                let grz_sin = grz_rad.sin(); let grz_cos = grz_rad.cos();

                let mut transform_world = |lx: f32, ly: f32, lz: f32| -> [f32; 3] {
                    let sx_mod = lx * sx + ly * shear_x;
                    let sy_mod = ly * sy + lx * shear_y;
                    let sz_mod = lz * sz + ly * shear_z;
                    let (rx, ry, rz) = rotate(sx_mod, sy_mod, sz_mod);
                    let mut fx = base_x + rx;
                    let mut fy = base_y + ry;
                    let mut fz = base_z + rz;
                    fx *= gs_x; fy *= gs_y; fz *= gs_z;
                    
                    let g_y1 = fy * grx_cos - fz * grx_sin; let g_z1 = fy * grx_sin + fz * grx_cos;
                    let g_x2 = fx * gry_cos + g_z1 * gry_sin; let g_z2 = -fx * gry_sin + g_z1 * gry_cos;
                    let g_x3 = g_x2 * grz_cos - g_y1 * grz_sin; let g_y3 = g_x2 * grz_sin + g_y1 * grz_cos;
                    
                    [g_x3 + gx, g_y3 + gy, g_z2 + gz]
                };

                let faces = get_part_polygons(&part.shape, &mut transform_world);
                if !faces.is_empty() {
                    let mut planes: Vec<crate::render::csg::Plane> = Vec::new();
                    let mut min = [f32::MAX; 3];
                    let mut max = [f32::MIN; 3];
                    for face in &faces {
                        for v in &face.verts {
                            if v[0] < min[0] { min[0] = v[0]; }
                            if v[1] < min[1] { min[1] = v[1]; }
                            if v[2] < min[2] { min[2] = v[2]; }
                            if v[0] > max[0] { max[0] = v[0]; }
                            if v[1] > max[1] { max[1] = v[1]; }
                            if v[2] > max[2] { max[2] = v[2]; }
                        }
                        if face.verts.len() >= 3 {
                            let p = make_plane(face.verts[0], face.verts[1], face.verts[2]);
                            let mut exists = false;
                            for ep in &planes {
                                let dot = ep.n[0]*p.n[0] + ep.n[1]*p.n[1] + ep.n[2]*p.n[2];
                                if dot > 0.999 && (ep.d - p.d).abs() < 0.001 {
                                    exists = true;
                                    break;
                                }
                            }
                            if !exists {
                                planes.push(p);
                            }
                        }
                    }
                    volumes.push(Volume { planes, part_id: part.id.clone(), target_id: part.csg_target_id.clone(), is_subtract: part.boolean_op == BooleanOp::Subtract, part_ref: part, aabb: (min, max) });
                }
                cached_part_faces.insert(part.id.clone(), faces);
            }
            let _t2 = std::time::Instant::now();

            let _iso = app.camera_ortho;
            let _cx_rad = if _iso { 30.0_f32.to_radians() } else { cam_rot_x.to_radians() };
            let _cy_rad = if _iso { 45.0_f32.to_radians() } else { cam_rot_y.to_radians() };
            let _view_dir: [f32; 3] = [
                -_cy_rad.sin() * _cx_rad.cos(),
                _cx_rad.sin(),
                _cy_rad.cos() * _cx_rad.cos(),
            ];
            let csg_parts_id = ui.make_persistent_id(&format!("csg_p_{}", obj.id));
            let csg_cache_id = ui.make_persistent_id(&format!("csg_c_{}", obj.id));
            let csg_transforms_id = ui.make_persistent_id(&format!("csg_t_{}", obj.id));
            let csg_params_id = ui.make_persistent_id(&format!("csg_param_{}", obj.id));
            
            let mut final_3d_faces: Vec<CachedCSGFace> = Vec::new();
            
            let is_cache_valid = ui.data(|d| {
                if let (Some(old_parts), Some(old_transforms)) = (
                    d.get_temp::<Vec<crate::data::object::ObjectPart>>(csg_parts_id),
                    d.get_temp::<std::collections::HashMap<String, (f32, f32, f32, f32, f32, f32, f32, f32, f32)>>(csg_transforms_id)
                ) {
                    old_parts == obj.parts && old_transforms == resolved_transforms
                } else {
                    false
                }
            });
            // HATA AYIKLAMA: Cache geçerli mi yazdır
            if !is_cache_valid {
                println!("CSG CACHE INVALIDATED for object: {}", obj.id);
            }
            
            if is_cache_valid {
                if let Some(cached) = ui.data(|d| d.get_temp::<Vec<CachedCSGFace>>(csg_cache_id)) {
                    final_3d_faces = cached;
                }
            }
            
            if !is_cache_valid {
                for (part_index, part) in obj.parts.iter().enumerate() {
                    if !part.is_visible { continue; }
                
                let eval_v = evaluated_parts.get(&part.id).unwrap();
                let _default_t = (eval_v.0, eval_v.1, eval_v.2, eval_v.6, eval_v.7, eval_v.8);
                let (base_x, base_y, base_z, rot_x, rot_y, rot_z, sx, sy, sz) = *resolved_transforms.get(&part.id).unwrap_or(&(eval_v.0, eval_v.1, eval_v.2, eval_v.6, eval_v.7, eval_v.8, eval_v.3, eval_v.4, eval_v.5));
                
                let mut shear_x = 0.0; let mut shear_y = 0.0; let mut shear_z = 0.0;
                for modif in &part.modifiers {
                    if let ModifierType::Shear(v) = modif { shear_x += v[0]; shear_y += v[1]; shear_z += v[2]; }
                }
                
                // Rotation pivot: applies rotation around a specific offset (part.pivot_offset)
                
                // Calculate dynamic pivot offset based on PivotMode and Scale
                let actual_pivot = match part.pivot_mode {
                    PivotMode::Center => [0.0, 0.0, 0.0],
                    PivotMode::CustomOffset(off) => off,
                    PivotMode::EdgeMinX => [-sx / 2.0, 0.0, 0.0],
                    PivotMode::EdgeMaxX => [sx / 2.0, 0.0, 0.0],
                    PivotMode::EdgeMinY => [0.0, -sy / 2.0, 0.0],
                    PivotMode::EdgeMaxY => [0.0, sy / 2.0, 0.0],
                    PivotMode::EdgeMinZ => [0.0, 0.0, -sz / 2.0],
                    PivotMode::EdgeMaxZ => [0.0, 0.0, sz / 2.0],
                };

                let rx_rad = rot_x.to_radians(); let ry_rad = rot_y.to_radians(); let rz_rad = rot_z.to_radians();
                let rx_sin = rx_rad.sin(); let rx_cos = rx_rad.cos();
                let ry_sin = ry_rad.sin(); let ry_cos = ry_rad.cos();
                let rz_sin = rz_rad.sin(); let rz_cos = rz_rad.cos();

                let rotate = |vx: f32, vy: f32, vz: f32| -> (f32, f32, f32) {
                    // Origin shift for pivot
                    let ox = vx - actual_pivot[0]; let oy = vy - actual_pivot[1]; let oz = vz - actual_pivot[2];
                    
                    let y1 = oy * rx_cos - oz * rx_sin; let z1 = oy * rx_sin + oz * rx_cos;
                    let x2 = ox * ry_cos + z1 * ry_sin; let z2 = -ox * ry_sin + z1 * ry_cos;
                    let x3 = x2 * rz_cos - y1 * rz_sin; let y3 = x2 * rz_sin + y1 * rz_cos;
                    
                    // Shift back
                    (x3 + actual_pivot[0], y3 + actual_pivot[1], z2 + actual_pivot[2])
                };

                let grx_rad = gr_x.to_radians(); let gry_rad = gr_y.to_radians(); let grz_rad = gr_z.to_radians();
                let grx_sin = grx_rad.sin(); let grx_cos = grx_rad.cos();
                let gry_sin = gry_rad.sin(); let gry_cos = gry_rad.cos();
                let grz_sin = grz_rad.sin(); let grz_cos = grz_rad.cos();

                let transform_z = |lx: f32, ly: f32, lz: f32| -> (egui::Pos2, f32) {
                    let sx_mod = lx * sx + ly * shear_x;
                    let sy_mod = ly * sy + lx * shear_y;
                    let sz_mod = lz * sz + ly * shear_z;
                    
                    let (rx, ry, rz) = rotate(sx_mod, sy_mod, sz_mod);
                    
                    let mut fx = base_x + rx;
                    let mut fy = base_y + ry;
                    let mut fz = base_z + rz;
                    
                    fx *= gs_x; fy *= gs_y; fz *= gs_z;
                    
                    let g_y1 = fy * grx_cos - fz * grx_sin; let g_z1 = fy * grx_sin + fz * grx_cos;
                    let g_x2 = fx * gry_cos + g_z1 * gry_sin; let g_z2 = -fx * gry_sin + g_z1 * gry_cos;
                    let g_x3 = g_x2 * grz_cos - g_y1 * grz_sin; let g_y3 = g_x2 * grz_sin + g_y1 * grz_cos;
                    
                    project_3d_with_z(g_x3 + gx, g_y3 + gy, g_z2 + gz)
                };

                let _transform = |lx: f32, ly: f32, lz: f32| -> egui::Pos2 {
                    transform_z(lx, ly, lz).0
                };

                let shading = parse_shading(&part.shading_model);
                let mut stroke_color = egui::Color32::from_rgb(100, 255, 100);
                if let Some(mat) = object_viewport::resolve_face_material(part, "All") {
                    stroke_color = object_viewport::material_tint(mat);
                } else if let Some(mat) = part.faces.values().next() {
                    stroke_color = object_viewport::material_tint(mat);
                }
                if part.boolean_op == BooleanOp::Subtract {
                    stroke_color = egui::Color32::from_rgb(255, 80, 80);
                }
                let wire_stroke = egui::Stroke::new(1.5, stroke_color);
                let _fill_color = egui::Color32::from_rgba_premultiplied(
                    stroke_color.r(),
                    stroke_color.g(),
                    stroke_color.b(),
                    180,
                );
                let actual_shading = if part.boolean_op == BooleanOp::Subtract {
                    ShadingMode::Wireframe
                } else {
                    shading
                };
                let _use_fill = actual_shading == ShadingMode::Solid || actual_shading == ShadingMode::Textured;
                
                if show_gizmo {
                    let g_center = project_3d(base_x, base_y, base_z);
                    painter.line_segment([g_center, project_3d(base_x + 1.0, base_y, base_z)], (1.0, egui::Color32::from_rgb(255, 100, 100)));
                    painter.line_segment([g_center, project_3d(base_x, base_y + 1.0, base_z)], (1.0, egui::Color32::from_rgb(100, 255, 100)));
                    painter.line_segment([g_center, project_3d(base_x, base_y, base_z + 1.0)], (1.0, egui::Color32::from_rgb(100, 150, 255)));
                }

                // ÖN-HESAPLAMA: Kamera bakış yönü FRAME BAŞINA TEK SEFER hesaplanır!
                // Her yüz için sin/cos/to_radians çağırmak 40 FPS'e düşürüyordu.
                let _iso = app.camera_ortho;
                let _cx_rad = if _iso { 30.0_f32.to_radians() } else { cam_rot_x.to_radians() };
                let _cy_rad = if _iso { 45.0_f32.to_radians() } else { cam_rot_y.to_radians() };
                let _clip_rect = painter.clip_rect();

                let mut push_face = |w_arr: &[[f32; 3]], face_id: &'static str| {
                    if w_arr.len() < 3 { return; }
                    
                    // ÖNCE BACK-FACE CULLING VE FRUSTUM CULLING İPTAL EDİLDİ!
                    // CSG objeleri 3D uzayda tam bir katı (solid) model oluşturmak zorundadır.
                    // Eğer burada kameraya bakmıyor diye yüzeyleri silersek, CSG kesme işlemi
                    // (subtract) çalışmaz çünkü objenin "arkası" açık kalır ve sonsuz uzay hatası verir.
                    // Ayrıca kamera döndüğünde objenin arkasındaki yüzeyler cache'den silinmiş olduğu için görünmez!
                    // Tüm culling işlemleri CSG'den sonra, ekrana çizim aşamasında yapılacaktır.
                    
                    let calc_area_3d = |verts: &[[f32; 3]]| -> f32 {
                        let mut area = 0.0;
                        if verts.len() < 3 { return 0.0; }
                        let v0 = verts[0];
                        for i in 1..(verts.len()-1) {
                            let v1 = verts[i];
                            let v2 = verts[i+1];
                            let d1 = [v1[0]-v0[0], v1[1]-v0[1], v1[2]-v0[2]];
                            let d2 = [v2[0]-v0[0], v2[1]-v0[1], v2[2]-v0[2]];
                            let cross = [
                                d1[1]*d2[2] - d1[2]*d2[1],
                                d1[2]*d2[0] - d1[0]*d2[2],
                                d1[0]*d2[1] - d1[1]*d2[0],
                            ];
                            area += (cross[0]*cross[0] + cross[1]*cross[1] + cross[2]*cross[2]).sqrt() * 0.5;
                        }
                        area
                    };
                    let original_area = calc_area_3d(w_arr);
                    
                    let mut initial_poly = Vec::new();
                    for i in 0..w_arr.len() {
                        let uv = match i % 4 { 0 => [0.0, 0.0], 1 => [1.0, 0.0], 2 => [1.0, 1.0], _ => [0.0, 1.0] };
                        initial_poly.push(crate::render::csg::Vertex { pos: w_arr[i], uv });
                    }
                    
                    let mut polys = Vec::new();
                    let mut is_inner_wall = false;
                    
                    if part.boolean_op != BooleanOp::Subtract {
                        polys.push(initial_poly);
                        for vol in &volumes {
                            if vol.part_id == part.id { continue; }
                            
                            if let Some(target) = &vol.target_id {
                                if target != &part.id { continue; }
                            }
                            
                            let mut next_polys = Vec::new();
                            for p in polys {
                                let mut p_min = [f32::MAX; 3];
                                let mut p_max = [f32::MIN; 3];
                                for v in &p {
                                    if v.pos[0] < p_min[0] { p_min[0] = v.pos[0]; }
                                    if v.pos[1] < p_min[1] { p_min[1] = v.pos[1]; }
                                    if v.pos[2] < p_min[2] { p_min[2] = v.pos[2]; }
                                    if v.pos[0] > p_max[0] { p_max[0] = v.pos[0]; }
                                    if v.pos[1] > p_max[1] { p_max[1] = v.pos[1]; }
                                    if v.pos[2] > p_max[2] { p_max[2] = v.pos[2]; }
                                }
                                
                                let overlap = 
                                    p_min[0] <= vol.aabb.1[0] + 0.01 && p_max[0] >= vol.aabb.0[0] - 0.01 &&
                                    p_min[1] <= vol.aabb.1[1] + 0.01 && p_max[1] >= vol.aabb.0[1] - 0.01 &&
                                    p_min[2] <= vol.aabb.1[2] + 0.01 && p_max[2] >= vol.aabb.0[2] - 0.01;
                                    
                                if !overlap {
                                    next_polys.push(p);
                                    continue;
                                }

                                let (outside_parts, inside_part) = crate::render::csg::subtract_convex(&p, &vol.planes, vol.is_subtract);
                                next_polys.extend(outside_parts);
                                
                                // KURTARMA ALGORİTMASI: Eğer bu Katı (+) obje, başka bir Katı (+) objenin içinde kaldığı için silindiyse...
                                if !vol.is_subtract && !inside_part.is_empty() {
                                    let intersect_convex = |poly: &[crate::render::csg::Vertex], planes: &[crate::render::csg::Plane]| -> Vec<crate::render::csg::Vertex> {
                                        let mut current = poly.to_vec();
                                        for plane in planes {
                                            let (_, inside) = crate::render::csg::split_poly(&current, plane, false);
                                            if inside.is_empty() { return Vec::new(); }
                                            current = inside;
                                        }
                                        current
                                    };
                                    
                                    for sub_vol in &volumes {
                                        if !sub_vol.is_subtract { continue; }
                                        
                                        // Bu delik, 'vol' objesini (Kemer) deliyor mu?
                                        let targets_vol = match &sub_vol.target_id {
                                            Some(tid) => tid == &vol.part_id,
                                            None => true,
                                        };
                                        if targets_vol {
                                            // Delik Kemeri deliyor. Peki asıl objemizi (Sütun) de deliyor mu?
                                            let targets_part = match &sub_vol.target_id {
                                                Some(tid) => tid == &part.id,
                                                None => true,
                                            };
                                            if !targets_part {
                                                // Delik sadece Kemeri deliyor, Sütuna dokunmuyor!
                                                // O zaman Sütunun kemer yüzünden silinen parçasını delik hizasında geri getir!
                                                let rescued_poly = intersect_convex(&inside_part, &sub_vol.planes);
                                                if !rescued_poly.is_empty() {
                                                    next_polys.push(rescued_poly);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            polys = next_polys;
                        }
                    } else {
                        // DELİK (-) OBJESİ: İç duvarları oluştur (Hole Wall Generation)
                        is_inner_wall = true;
                        
                        let intersect_convex = |poly: &[crate::render::csg::Vertex], planes: &[crate::render::csg::Plane]| -> Vec<crate::render::csg::Vertex> {
                            let mut current = poly.to_vec();
                            for plane in planes {
                                // inner wall testi: Solid plane'lerine karşı test ediyoruz ama coplanar 
                                // yüzeylerin dışarıda sayılması (silinmesi) için is_subtract_vol = true geçiyoruz.
                                let (_, inside) = crate::render::csg::split_poly(&current, plane, true);
                                if inside.is_empty() { return Vec::new(); }
                                current = inside;
                            }
                            current
                        };
                        
                        // Hangi Katı (+) objeleri deliyorsa, o objelerin İÇİNDE kalan yüzeyleri tut
                        for vol in &volumes {
                            if vol.is_subtract { continue; } // Sadece Katı objelerle kesişim
                            
                            // Eğer bu Delik objesi sadece belirli bir hedefi deliyorsa, duvarları da sadece o hedefte oluştur
                            if let Some(target) = &part.csg_target_id {
                                if target != &vol.part_id { continue; }
                            }
                            
                            let mut p_min = [f32::MAX; 3];
                            let mut p_max = [f32::MIN; 3];
                            for v in &initial_poly {
                                if v.pos[0] < p_min[0] { p_min[0] = v.pos[0]; }
                                if v.pos[1] < p_min[1] { p_min[1] = v.pos[1]; }
                                if v.pos[2] < p_min[2] { p_min[2] = v.pos[2]; }
                                if v.pos[0] > p_max[0] { p_max[0] = v.pos[0]; }
                                if v.pos[1] > p_max[1] { p_max[1] = v.pos[1]; }
                                if v.pos[2] > p_max[2] { p_max[2] = v.pos[2]; }
                            }
                            
                            let overlap = 
                                p_min[0] <= vol.aabb.1[0] + 0.01 && p_max[0] >= vol.aabb.0[0] - 0.01 &&
                                p_min[1] <= vol.aabb.1[1] + 0.01 && p_max[1] >= vol.aabb.0[1] - 0.01 &&
                                p_min[2] <= vol.aabb.1[2] + 0.01 && p_max[2] >= vol.aabb.0[2] - 0.01;
                                
                            if !overlap { continue; }
                            
                            let mut inner_wall = intersect_convex(&initial_poly, &vol.planes);
                            if !inner_wall.is_empty() {
                                // İç duvarları DİĞER deliklerden (Holes) çıkar (Crossing-wall hatası çözümü!)
                                let mut inner_polys = vec![inner_wall];
                                for other_vol in &volumes {
                                    if !other_vol.is_subtract { continue; }
                                    if other_vol.part_id == part.id { continue; } // Kendi deliğimizden çıkarma
                                    
                                    let mut next_inner = Vec::new();
                                    for p in inner_polys {
                                        let (outside, _) = crate::render::csg::subtract_convex(&p, &other_vol.planes, true);
                                        next_inner.extend(outside);
                                    }
                                    inner_polys = next_inner;
                                }
                                
                                for mut p_wall in inner_polys {
                                    // İç duvarların görünmesi için yüzey yönünü TERSİNE çevir (Arka yüzey sorunu çözümü)
                                    p_wall.reverse();
                                    if p_wall.len() >= 3 {
                                    // Inner Wall Caching
                                    let mut poly_3d = Vec::with_capacity(p_wall.len());
                                    for v in &p_wall {
                                        if !v.pos[0].is_finite() || !v.pos[1].is_finite() || !v.pos[2].is_finite() { continue; }
                                        poly_3d.push(v.pos);
                                    }
                                    
                                    if poly_3d.len() >= 3 {
                                        let final_shading = parse_shading(&vol.part_ref.shading_model);
                                        let triangles = if matches!(final_shading, ShadingMode::Wireframe) { vec![] } else { triangulate_3d(&poly_3d) };
                                        final_3d_faces.push(CachedCSGFace { 
                                            poly_3d,
                                            original_3d: None,
                                            face_id, 
                                            part_index,
                                            shading: final_shading, 
                                            wire_stroke: egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(0, 0, 0, 50)), 
                                            destroyed_ratio: 0.0,
                                            triangles,
                                        });
                                    }
                                }
                            }
                            }
                        }
                        
                        // Delik objesinin kendisi sahnede kaybolmasın diye sadece Tel Kafes (Wireframe) olarak ekle
                        // YENİ KARAR: Kullanıcı dışarıda kalan tel kafesleri görmek İSTEMİYOR! Seamless boolean!
                        // Sadece iç duvarlar oluşturulduysa görünür olacak. Wireframe eklemeyi kaldırdık.
                    }
                    
                    let mut visible_area = 0.0;
                    for p in &polys {
                        let mut p_verts = Vec::with_capacity(p.len());
                        for v in p { p_verts.push(v.pos); }
                        visible_area += calc_area_3d(&p_verts);
                    }
                    let destroyed_ratio = if original_area > 1e-5 {
                        (original_area - visible_area) / original_area
                    } else {
                        0.0
                    };
                    
                    for poly in polys {
                        if poly.len() < 3 { continue; }
                        
                        // Sıfır Hata Koruması: Noktalar çakışık mı (Degenerate Polygon) veya sonsuz (NaN) mu?
                        let mut valid = false;
                        for i in 1..poly.len() {
                            let d = (poly[i].pos[0] - poly[0].pos[0]).abs() + (poly[i].pos[1] - poly[0].pos[1]).abs() + (poly[i].pos[2] - poly[0].pos[2]).abs();
                            if d > 1e-4 { valid = true; break; }
                        }
                        if !valid { continue; }
                        
                        let mut poly_3d = Vec::with_capacity(poly.len());
                        for v in &poly {
                            if !v.pos[0].is_finite() || !v.pos[1].is_finite() || !v.pos[2].is_finite() { continue; }
                            poly_3d.push(v.pos);
                        }
                        if poly_3d.len() < 3 { continue; }
                        
                        let mut final_shading = actual_shading;
                        if is_inner_wall { final_shading = ShadingMode::Solid; }
                        
                        let triangles = if matches!(final_shading, ShadingMode::Wireframe) { vec![] } else { triangulate_3d(&poly_3d) };
                        
                        final_3d_faces.push(CachedCSGFace { 
                            poly_3d, 
                            original_3d: Some(w_arr.to_vec()),
                            face_id, 
                            part_index, 
                            shading: final_shading, 
                            wire_stroke, 
                            destroyed_ratio,
                            triangles,
                        });
                    }
                };

                if let Some(faces) = cached_part_faces.remove(&part.id) {
                    for face in faces {
                        push_face(&face.verts, face.face_id);
                    }
                }
            } // end of parts loop
            
            ui.data_mut(|d| {
                d.insert_temp(csg_parts_id, obj.parts.clone());
                d.insert_temp(csg_transforms_id, resolved_transforms.clone());
                d.insert_temp(csg_params_id, obj.parameters.clone());
                d.insert_temp(csg_cache_id, final_3d_faces.clone());
            });
        }
        
        // --- 1. GLOBAL EN DÜŞÜK Y KOORDİNATINI BUL (Zemin tespiti için) ---
        let mut global_min_y = f32::MAX;
        for cface in &final_3d_faces {
            let part = &obj.parts[cface.part_index];
            if !part.is_visible { continue; }
            for v in &cface.poly_3d {
                if v[1] < global_min_y {
                    global_min_y = v[1];
                }
            }
        }

        let debug_culling = ui.data_mut(|d| d.get_temp(egui::Id::new("debug_culling")).unwrap_or(false));
        
        // --- KAMERA OLUŞTURMA VE PROJEKSİYON DÖNGÜSÜ (CSG'DEN BAĞIMSIZ) ---
        for cface in &final_3d_faces {
            let part = &obj.parts[cface.part_index];
            if !part.is_visible { continue; }
            
            let use_fill = matches!(cface.shading, ShadingMode::Solid | ShadingMode::Textured);
            
            let mut is_culled = false;
            let mut face_normal = [0.0, 0.0, 1.0];
            
            // CSG SONRASI ARKA YÜZ KIRPMA (Back-Face Culling)
            // CSG kesme işlemi cface.poly_3d winding order'ını (saat yönünü) bozabilir.
            // Bu yüzden normali kararlı olan original_3d'den (orijinal kesilmemiş yüzey) hesaplıyoruz.
            if use_fill {
                let poly = cface.original_3d.as_deref().unwrap_or(&cface.poly_3d);
                if poly.len() >= 3 {
                    let mut nx = 0.0; let mut ny = 0.0; let mut nz = 0.0;
                    for i in 0..poly.len() {
                        let v1 = poly[i];
                        let v2 = poly[(i + 1) % poly.len()];
                        nx += (v1[1] - v2[1]) * (v1[2] + v2[2]);
                        ny += (v1[2] - v2[2]) * (v1[0] + v2[0]);
                        nz += (v1[0] - v2[0]) * (v1[1] + v2[1]);
                    }
                    let len = (nx*nx + ny*ny + nz*nz).sqrt();
                    if len > 1e-5 {
                        nx /= len; ny /= len; nz /= len;
                    }
                    
                    face_normal = [nx, ny, nz];
                    // Kameranın world space'deki bakış yönü (Kamera -Z'den +Z'ye bakar, dönmeleri uygularız)
                    // _cx_rad ve _cy_rad, kameranın Y ve X eksenindeki dönüşleridir (Euler)
                    // View vector = Camera Forward Vector
                    let view_x = -_cy_rad.sin() * _cx_rad.cos();
                    let view_y = _cx_rad.sin();
                    let view_z = _cy_rad.cos() * _cx_rad.cos();
                    
                    let dot = nx * view_x + ny * view_y + nz * view_z;
                    // Axiom'da küp yüzeyleri içeri bakacak şekilde (CW) tanımlandığı için normal vektörü İÇERİ doğru!
                    // İçe bakan bir normal, kameraya 'ters' (aynı yönde) ise aslında 'ÖN YÜZ'dür!
                    // İçe bakan bir normal, kameraya doğru bakıyorsa aslında 'ARKA YÜZ'dür!
                    // dot < 0 (Kameraya bakıyor) -> ARKA YÜZ -> CULL EDİLMELİ!
                    if dot < 0.0 {
                        is_culled = true;
                        if !debug_culling { continue; } // Arka yüzeyleri atla (CPU Painter'da ghosting'i önler)
                    }
                }
            }

            // Z-Derinlik: Poligonun en uzak noktası (min_z) ve ortalama Y değeri
            let mut min_z = f32::MAX;
            let mut sum_y = 0.0;
            for v in &cface.poly_3d {
                let (_, z) = project_3d_with_z(v[0], v[1], v[2]);
                if z < min_z { min_z = z; }
                sum_y += v[1];
            }
            let avg_y = if !cface.poly_3d.is_empty() { sum_y / cface.poly_3d.len() as f32 } else { 0.0 };
            let mut z_depth = if min_z == f32::MAX { f32::MIN } else { min_z };

            // Yüzeyin yatay olup olmadığını kontrol et (normal yönü Y aksına yakın mı?)
            let poly = cface.original_3d.as_deref().unwrap_or(&cface.poly_3d);
            let is_horizontal = if poly.len() >= 3 {
                let mut nx = 0.0; let mut ny = 0.0; let mut nz = 0.0;
                for i in 0..poly.len() {
                    let v1 = poly[i];
                    let v2 = poly[(i + 1) % poly.len()];
                    nx += (v1[1] - v2[1]) * (v1[2] + v2[2]);
                    ny += (v1[2] - v2[2]) * (v1[0] + v2[0]);
                    nz += (v1[0] - v2[0]) * (v1[1] + v2[1]);
                }
                let len = (nx*nx + ny*ny + nz*nz).sqrt();
                len > 1e-5 && ny.abs() > 0.9 * len
            } else {
                false
            };

            // Eğer yüzey yatay zemin/alt tabaka seviyesindeyse, onu Painter's Algorithm'de 
            // en arkaya göndermek için z_depth değerini ciddi şekilde azaltıyoruz.
            // Bu sayede sütunlar ve duvarlar zemin kaplamasının üstüne kusursuz şekilde çizilir.
            if is_horizontal && avg_y < global_min_y + 1.5 {
                z_depth -= 50.0;
            }

            let plane_key = if poly.len() >= 3 {
                let mut nx = 0.0;
                let mut ny = 0.0;
                let mut nz = 0.0;
                for i in 0..poly.len() {
                    let j = (i + 1) % poly.len();
                    let curr = poly[i];
                    let next = poly[j];
                    nx += (curr[1] - next[1]) * (curr[2] + next[2]);
                    ny += (curr[2] - next[2]) * (curr[0] + next[0]);
                    nz += (curr[0] - next[0]) * (curr[1] + next[1]);
                }
                let len = (nx*nx + ny*ny + nz*nz).sqrt();
                if len > 1e-5 {
                    let nx = nx / len;
                    let ny = ny / len;
                    let nz = nz / len;
                    let mut sum_d = 0.0;
                    for v in poly {
                        sum_d -= nx * v[0] + ny * v[1] + nz * v[2];
                    }
                    let d = sum_d / poly.len() as f32;
                    Some(get_plane_key([nx, ny, nz], d))
                } else {
                    None
                }
            } else {
                None
            };

            all_faces.push(FaceToDraw {
                poly_3d: &cface.poly_3d,
                original_3d: cface.original_3d.as_deref(),
                z_depth,
                plane_key,
                face_id: cface.face_id,
                part_index: cface.part_index,
                part,
                shading: &cface.shading,
                wire_stroke: cface.wire_stroke,
                destroyed_ratio: cface.destroyed_ratio,
                triangles: &cface.triangles,
                is_culled,
                normal: face_normal,
                is_inner_wall: cface.original_3d.is_none(),
            });
            
        }
        let _t3 = std::time::Instant::now();

        // COPLANAR YÜZEY GRUPLAMA (Painter's Algorithm İçin Derinlik Eşitleme)
        // Aynı 3D düzlemde yer alan (coplanar) yüzeylerin z_depth değerlerini
        // eşitleyerek Z-fighting ve yanlış sıralama (frame/cam) problemlerini çözüyoruz.
        let mut plane_depths: std::collections::HashMap<PlaneKey, f32> = std::collections::HashMap::new();
        for face in &all_faces {
            if let Some(key) = face.plane_key {
                let entry = plane_depths.entry(key).or_insert(face.z_depth);
                *entry = f32::min(*entry, face.z_depth);
            }
        }
        for face in &mut all_faces {
            if let Some(key) = face.plane_key {
                if let Some(&min_d) = plane_depths.get(&key) {
                    face.z_depth = min_d;
                }
            }
        }
        
        // WGPU DONANIMSAL Z-BUFFER YERİNE PAINTER'S ALGORITHM'A GERİ DÖNÜYORUZ!
        // Kullanıcının modelleri 2.5D katman (layer) mantığına dayanıyor. 
        // Çözüm: Sıralamayı CPU'da %100 kararlı ve panic-free yapacak şekilde total_cmp tabanlı yapıyoruz.
        all_faces.sort_by(|a, b| {
            let cmp = a.z_depth.total_cmp(&b.z_depth);
            if cmp == std::cmp::Ordering::Equal {
                // Aynı derinlikteki veya coplanar yüzeyler için, listede üstte olan (küçük part_index) 
                // parça en son çizilsin (yani üstte kalsın). 
                let index_cmp = b.part_index.cmp(&a.part_index);
                if index_cmp == std::cmp::Ordering::Equal {
                    let r_cmp = a.destroyed_ratio.partial_cmp(&b.destroyed_ratio).unwrap_or(std::cmp::Ordering::Equal);
                    if r_cmp == std::cmp::Ordering::Equal {
                        a.face_id.cmp(&b.face_id)
                    } else {
                        r_cmp
                    }
                } else {
                    index_cmp
                }
            } else {
                cmp
            }
        });
        let _t4 = std::time::Instant::now();
        
        let show_uv_dots = ui.data_mut(|d| d.get_temp(egui::Id::new("show_uv_dots")).unwrap_or(false));
        let show_outer_shell = ui.data_mut(|d| d.get_temp(egui::Id::new("show_outer_shell")).unwrap_or(false));
        let show_gpu_triangles = ui.data_mut(|d| d.get_temp(egui::Id::new("show_gpu_triangles")).unwrap_or(false));
        let debug_wireframe_only = ui.data_mut(|d| d.get_temp(egui::Id::new("debug_wireframe_only")).unwrap_or(false));
        let debug_depth_color = ui.data_mut(|d| d.get_temp(egui::Id::new("debug_depth_color")).unwrap_or(false));
        
        let mut min_z_all = f32::MAX;
        let mut max_z_all = f32::MIN;
        for face in &all_faces {
            if face.z_depth < min_z_all { min_z_all = face.z_depth; }
            if face.z_depth > max_z_all { max_z_all = face.z_depth; }
        }
        let z_range = (max_z_all - min_z_all).max(0.001);
        
        // ----------------------------------------------------
        // --- GPU (WGPU) ÇİZİM AKTARIMI (3000 FPS GEÇİŞİ) ---
        // ----------------------------------------------------
        let cx = if app.camera_ortho { 30.0_f32.to_radians() } else { app.camera_rot[0].to_radians() };
        let cy = if app.camera_ortho { 45.0_f32.to_radians() } else { app.camera_rot[1].to_radians() };
        
        let mut edge_counts = std::collections::HashMap::new();
        if !show_outer_shell {
            for face in &all_faces {
                if face.poly_3d.len() < 3 { continue; }
                let key_prefix = format!("{}_{}", face.part_index, face.face_id);
                for i in 0..face.poly_3d.len() {
                    let p1 = face.poly_3d[i];
                    let p2 = face.poly_3d[(i + 1) % face.poly_3d.len()];
                    let q1 = ((p1[0] * 1000.0).round() as i32, (p1[1] * 1000.0).round() as i32, (p1[2] * 1000.0).round() as i32);
                    let q2 = ((p2[0] * 1000.0).round() as i32, (p2[1] * 1000.0).round() as i32, (p2[2] * 1000.0).round() as i32);
                    let edge_key = if q1 < q2 { (q1, q2) } else { (q2, q1) };
                    *edge_counts.entry((key_prefix.clone(), edge_key)).or_insert(0) += 1;
                }
            }
        }
        
        let mut solid_vertices = Vec::with_capacity(all_faces.len() * 6);
        let mut textured_batches = Vec::new();
        let mut line_vertices = Vec::with_capacity(all_faces.len() * 8);
        
        let mut current_texture = String::new();
        let mut batch_start = 0;
        for face in &all_faces {
            if face.poly_3d.len() >= 3 {
                // Dinamik Renk ve Materyal Çözümleme
                // CPU renderer ile aynı materyal sistemini kullanıyoruz (FaceMaterial)
                let mut color = match &face.shading {
                    ShadingMode::Wireframe => [0.0, 1.0, 0.0, 0.3],
                    _ => {
                        // Materyal rengini çözümle (draw_face_quad ile aynı mantık)
                        let mat = crate::render::object_viewport::resolve_face_material(face.part, face.face_id);
                        let default_mat = crate::data::object::FaceMaterial::default();
                        let mat = mat.unwrap_or(&default_mat);
                        
                        let bg = mat.background_color;
                        let tint = crate::render::object_viewport::material_tint(mat);
                        
                        // Texture varsa arka plan rengini, yoksa tint rengini kullan
                        let tex_cache = &app.texture_cache;
                        let has_texture = !mat.texture_id.is_empty() && tex_cache.get(&mat.texture_id).is_some();
                        
                        if has_texture {
                            // Doku varsa in.color olarak BEYAZ veriyoruz!
                            // Çünkü TextureComposer zaten arka plan rengini ColorImage içine gömdü (bake etti).
                            // Eğer burada comp.base_color verirsek, Shader içinde (tex_color * in.color) 
                            // işlemi iki kez renk çarpımı (kare alma) yapar. Özellikle siyah arka planlarda
                            // sonuç PITCH BLACK (Simsiyah) çıkar!
                            [1.0, 1.0, 1.0, mat.opacity]
                        } else {
                            if mat.use_custom_bg {
                                [bg[0] as f32 / 255.0, bg[1] as f32 / 255.0, bg[2] as f32 / 255.0, mat.opacity]
                            } else {
                                [tint.r() as f32 / 255.0, tint.g() as f32 / 255.0, tint.b() as f32 / 255.0, mat.opacity]
                            }
                        }
                    }
                };

                // DIŞ KABUK (AIR CONTACT) MODU: Tüm yüzeyleri GPU ile benzersiz renkte çiz!
                // Daha önce CPU ile çiziliyordu ama CPU depth testi yapmadığı için hatalı görünüyordu.
                if show_outer_shell {
                    let mut hash = 0u32;
                    for b in face.part.id.bytes() { hash = hash.wrapping_mul(31).wrapping_add(b as u32); }
                    for b in face.face_id.bytes() { hash = hash.wrapping_mul(31).wrapping_add(b as u32); }
                    let r = (hash.wrapping_mul(17) % 200 + 55) as f32 / 255.0;
                    let g = (hash.wrapping_mul(37) % 200 + 55) as f32 / 255.0;
                    let b = (hash.wrapping_mul(71) % 200 + 55) as f32 / 255.0;
                    color = [r, g, b, 1.0];
                }

                if debug_depth_color {
                    let depth_ratio = (face.z_depth - min_z_all) / z_range;
                    let v = depth_ratio.clamp(0.0, 1.0);
                    color = [v, v, v, 1.0];
                }
                
                // --- 1. KATI ÇİZİM (Sadece Solid/Textured objeler için üçgenleştirme) ---
                if !matches!(face.shading, ShadingMode::Wireframe) && !debug_wireframe_only {
                    let mat = crate::render::object_viewport::resolve_face_material(face.part, face.face_id);
                    let default_mat = crate::data::object::FaceMaterial::default();
                    let mat = mat.unwrap_or(&default_mat);
                    
                    let has_texture = !mat.texture_id.is_empty() && app.texture_cache.entries.get(&mat.texture_id).is_some();
                    let new_texture = if has_texture && !show_outer_shell { mat.texture_id.clone() } else { "fallback".to_string() };
                    
                    if new_texture != current_texture {
                        if solid_vertices.len() as u32 > batch_start {
                            textured_batches.push(crate::render::gpu::TexturedBatch {
                                texture_id: current_texture.clone(),
                                range: batch_start..solid_vertices.len() as u32,
                            });
                        }
                        current_texture = new_texture;
                        batch_start = solid_vertices.len() as u32;
                    }

                    // 3D Dünya Uzayında (World Space) Planar UV Mapping
                    let mut w0 = [0.0; 3];
                    let mut vec_x = [1.0, 0.0, 0.0];
                    let mut vec_y = [0.0, 1.0, 0.0];
                    let mut len_x_sq = 1.0;
                    let mut len_y_sq = 1.0;

                    if let Some(orig) = &face.original_3d {
                        if orig.len() >= 3 {
                            w0 = orig[0];
                            let w1 = orig[1];
                            let w_last = *orig.last().unwrap();
                            vec_x = [w1[0]-w0[0], w1[1]-w0[1], w1[2]-w0[2]];
                            vec_y = [w_last[0]-w0[0], w_last[1]-w0[1], w_last[2]-w0[2]];
                            len_x_sq = vec_x[0]*vec_x[0] + vec_x[1]*vec_x[1] + vec_x[2]*vec_x[2];
                            len_y_sq = vec_y[0]*vec_y[0] + vec_y[1]*vec_y[1] + vec_y[2]*vec_y[2];
                            if len_x_sq < 1e-5 { len_x_sq = 1.0; }
                            if len_y_sq < 1e-5 { len_y_sq = 1.0; }
                        }
                    }
                    
                    let uv_scale_x = mat.uv_scale[0];
                    let uv_scale_y = mat.uv_scale[1];
                    let calc_uv = |pos: [f32; 3]| -> [f32; 2] {
                        let p_vec = [pos[0]-w0[0], pos[1]-w0[1], pos[2]-w0[2]];
                        let u = (p_vec[0]*vec_x[0] + p_vec[1]*vec_x[1] + p_vec[2]*vec_x[2]) / len_x_sq;
                        let v = (p_vec[0]*vec_y[0] + p_vec[1]*vec_y[1] + p_vec[2]*vec_y[2]) / len_y_sq;
                        [mat.uv_offset[0] + u * uv_scale_x, mat.uv_offset[1] + v * uv_scale_y]
                    };

                    let triangles = &face.triangles;
                    for i in (0..triangles.len()).step_by(3) {
                        if i + 2 < triangles.len() {
                            let idx0 = triangles[i];
                            let idx1 = triangles[i+1];
                            let idx2 = triangles[i+2];
                            
                            let v0 = face.poly_3d[idx0];
                            let v1 = face.poly_3d[idx1];
                            let v2 = face.poly_3d[idx2];
                            
                            solid_vertices.push(crate::render::gpu::GpuVertex { position: v0, uv: calc_uv(v0), color });
                            solid_vertices.push(crate::render::gpu::GpuVertex { position: v1, uv: calc_uv(v1), color });
                            solid_vertices.push(crate::render::gpu::GpuVertex { position: v2, uv: calc_uv(v2), color });
                            
                            if show_gpu_triangles {
                                let tri_color = [1.0, 1.0, 0.0, 0.8]; // Sarı üçgen çizgileri
                                line_vertices.push(crate::render::gpu::GpuVertex { position: v0, uv: [0.0, 0.0], color: tri_color });
                                line_vertices.push(crate::render::gpu::GpuVertex { position: v1, uv: [0.0, 0.0], color: tri_color });
                                
                                line_vertices.push(crate::render::gpu::GpuVertex { position: v1, uv: [0.0, 0.0], color: tri_color });
                                line_vertices.push(crate::render::gpu::GpuVertex { position: v2, uv: [0.0, 0.0], color: tri_color });
                                
                                line_vertices.push(crate::render::gpu::GpuVertex { position: v2, uv: [0.0, 0.0], color: tri_color });
                                line_vertices.push(crate::render::gpu::GpuVertex { position: v0, uv: [0.0, 0.0], color: tri_color });
                            }
                        }
                    }
                }
                
                // --- 2. ÇİZGİ ÇİZİM (Hem Solid hem Wireframe objeler için dış kenar hatları) ---
                if !show_outer_shell || debug_wireframe_only {
                    let sr = face.wire_stroke.color.r() as f32 / 255.0;
                    let sg = face.wire_stroke.color.g() as f32 / 255.0;
                    let sb = face.wire_stroke.color.b() as f32 / 255.0;
                    let sa = face.wire_stroke.color.a() as f32 / 255.0;
                    let line_color = [sr, sg, sb, sa];
                    
                    for i in 0..face.poly_3d.len() {
                        let p1_3d = face.poly_3d[i];
                        let p2_3d = face.poly_3d[(i + 1) % face.poly_3d.len()];
                        
                        let q1 = ((p1_3d[0] * 1000.0).round() as i32, (p1_3d[1] * 1000.0).round() as i32, (p1_3d[2] * 1000.0).round() as i32);
                        let q2 = ((p2_3d[0] * 1000.0).round() as i32, (p2_3d[1] * 1000.0).round() as i32, (p2_3d[2] * 1000.0).round() as i32);
                        let edge_key = if q1 < q2 { (q1, q2) } else { (q2, q1) };
                        let key_prefix = format!("{}_{}", face.part_index, face.face_id);
                        
                        if *edge_counts.get(&(key_prefix, edge_key)).unwrap_or(&0) <= 1 {
                            line_vertices.push(crate::render::gpu::GpuVertex { position: p1_3d, uv: [0.0, 0.0], color: line_color });
                            line_vertices.push(crate::render::gpu::GpuVertex { position: p2_3d, uv: [1.0, 1.0], color: line_color });
                        }
                    }
                }
            }
        }
        
        // Kalan son batch'i ekle
        if solid_vertices.len() as u32 > batch_start {
            textured_batches.push(crate::render::gpu::TexturedBatch {
                texture_id: current_texture,
                range: batch_start..solid_vertices.len() as u32,
            });
        }

        // KUSURSUZ EŞLEŞME: CPU zaten pikselleri (screen_poly) hesapladı!
        // Egui-wgpu, viewport'u response.rect'in fiziksel piksel alanına ayarlıyor.
        // NDC (-1,1) uzayı bu küçük viewport'a eşleniyor.
        // Bu yüzden screen_poly koordinatlarını (mutlak piksel) → rect-yerel koordinatlara → NDC'ye çevirmemiz lazım.
        let rect_w = response.rect.width();
        let rect_h = response.rect.height();
        
        // Screen-to-NDC Matrisi (Doğrudan World Space'den NDC'ye)
        let rot_y = glam::Mat4::from_rotation_y(cy);
        let rot_x = glam::Mat4::from_rotation_x(cx);
        let zoom = app.camera_zoom;
        let pan_x = app.camera_pan[0];
        let pan_y = app.camera_pan[1];
        
        let scale_trans = glam::Mat4::from_cols_array(&[
            zoom * 2.0 / rect_w, 0.0, 0.0, 0.0,
            0.0, zoom * 2.0 / rect_h, 0.0, 0.0,
            0.0, 0.0, zoom * (-1.0 / 20000.0), 0.0,
            pan_x * 2.0 / rect_w, -pan_y * 2.0 / rect_h, 0.5, 1.0
        ]);
        
        let proj = scale_trans * rot_x * rot_y;
        let camera_matrix = proj.to_cols_array_2d();

        // Ekran kartına komut verecek olan Orijinal Callback nesnemizi yaratıyoruz
        let custom_callback = crate::render::gpu::Custom3dCallback {
            solid_vertices,
            textured_batches,
            line_vertices,
            camera_matrix,
            textures_to_update: {
                // upload_queue'yu al ve boşalt
                let mut q = Vec::new();
                std::mem::swap(&mut q, &mut app.texture_cache.upload_queue);
                q
            },
        };

        // Egui_wgpu, PaintCallback objesini ve Arc sarmalamasını kendisi yapar!
        painter.add(eframe::egui_wgpu::Callback::new_paint_callback(
            response.rect,
            custom_callback,
        ));
        let _t5 = std::time::Instant::now();

        // =====================================================
        // HİBRİT RENDER: GPU mesh dolgusunu çizdikten sonra,
        // CPU painter ile üst katman özelliklerini ekle:
        //   1. Textured doku katmanı (ASCII karakter tessellasyonu)
        //   2. Kenar çizgileri (wireframe edges)
        //   3. UV Dot noktaları (show_uv_dots)
        //   4. Dış Kabuk modu (show_outer_shell)
        // =====================================================
        let tex_cache = &app.texture_cache;
        let debug_culling = ui.data_mut(|d| d.get_temp(egui::Id::new("debug_culling")).unwrap_or(false));
        let debug_normals = ui.data_mut(|d| d.get_temp(egui::Id::new("debug_normals")).unwrap_or(false));
        let debug_inner_walls = ui.data_mut(|d| d.get_temp(egui::Id::new("debug_inner_walls")).unwrap_or(false));
        let debug_z_depth = ui.data_mut(|d| d.get_temp(egui::Id::new("debug_z_depth")).unwrap_or(false));
        let debug_coplanar = ui.data_mut(|d| d.get_temp(egui::Id::new("debug_coplanar")).unwrap_or(false));
        let debug_labels = ui.data_mut(|d| d.get_temp(egui::Id::new("debug_labels")).unwrap_or(false));
        let debug_wireframe = ui.data_mut(|d| d.get_temp(egui::Id::new("debug_wireframe")).unwrap_or(false));

        for face in &all_faces {
            // Dış Kabuk (Outer Shell) Modu GPU'ya taşındı! 
            if show_outer_shell {
                continue; // GPU her şeyi benzersiz renklerle çizdi, CPU painter'a gerek yok.
            }

            let screen_poly: Vec<egui::Pos2> = face.poly_3d.iter().map(|v| project_3d_with_z(v[0], v[1], v[2]).0).collect();
            let mut cx = 0.0; let mut cy = 0.0; let mut cz = 0.0;
            if face.poly_3d.len() > 0 {
                for p in face.poly_3d {
                    cx += p[0]; cy += p[1]; cz += p[2];
                }
                let len = face.poly_3d.len() as f32;
                cx /= len; cy /= len; cz /= len;
            }
            let center_screen = project_3d_with_z(cx, cy, cz).0;

            // --- HATA AYIKLAMA (DEBUG) GÖRSELLEŞTİRMELERİ ---
            if debug_inner_walls && face.is_inner_wall {
                if screen_poly.len() >= 3 {
                    painter.add(egui::Shape::convex_polygon(screen_poly.clone(), egui::Color32::from_rgba_premultiplied(255, 0, 255, 50), egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 0, 255))));
                }
            }

            if debug_culling && face.is_culled {
                if screen_poly.len() >= 3 {
                    painter.add(egui::Shape::convex_polygon(screen_poly.clone(), egui::Color32::from_rgba_premultiplied(255, 0, 0, 30), egui::Stroke::new(1.0, egui::Color32::RED)));
                }
                // Culled yüzeyin başka bir şey çizmesine gerek yok
                continue;
            }

            // Yüzey Normallerini çiz
            if debug_normals && face.poly_3d.len() >= 3 {
                let n = face.normal;
                let p1 = center_screen;
                // Normalin ucunu kameraya doğru 1.5 birim uzat
                let p2 = project_3d_with_z(cx + n[0]*1.5, cy + n[1]*1.5, cz + n[2]*1.5).0;
                
                painter.line_segment([p1, p2], egui::Stroke::new(2.0, egui::Color32::YELLOW));
                painter.circle_filled(p1, 2.0, egui::Color32::RED);
            }

            if debug_coplanar {
                if let Some(key) = &face.plane_key {
                    let mut hash = 0u32;
                    hash = hash.wrapping_mul(31).wrapping_add(key.nx as u32);
                    hash = hash.wrapping_mul(31).wrapping_add(key.ny as u32);
                    hash = hash.wrapping_mul(31).wrapping_add(key.nz as u32);
                    hash = hash.wrapping_mul(31).wrapping_add(key.d as u32);
                    
                    let r = (hash & 0xFF) as u8;
                    let g = ((hash >> 8) & 0xFF) as u8;
                    let b = ((hash >> 16) & 0xFF) as u8;
                    
                    if screen_poly.len() >= 3 {
                        painter.add(egui::Shape::convex_polygon(screen_poly.clone(), egui::Color32::from_rgba_premultiplied(r, g, b, 80), egui::Stroke::new(2.0, egui::Color32::from_rgb(r, g, b))));
                    }
                }
            }

            if debug_wireframe && screen_poly.len() >= 3 {
                let color = if face.is_inner_wall { egui::Color32::from_rgb(255, 0, 255) } else { egui::Color32::WHITE };
                painter.add(egui::Shape::closed_line(screen_poly.clone(), egui::Stroke::new(1.0, color)));
            }

            if debug_labels {
                let _text = format!("P:{} | F:{}", face.part_index, face.face_id);
                let _label_pos = center_screen - egui::vec2(0.0, 15.0);
                // Draw label in the post-pass instead
            }

            if face.is_culled { continue; }

            // Solid ve Textured yüzeyler → Katmanlı sıralama (Painter's Algorithm) için CPU üst üste çizim
            if matches!(face.shading, ShadingMode::Textured | ShadingMode::Solid) && !debug_wireframe_only {
                let original_screen_poly: Option<Vec<egui::Pos2>> = face.original_3d.map(|orig| {
                    orig.iter().map(|v| project_3d_with_z(v[0], v[1], v[2]).0).collect()
                });
                let texture_debug = painter.ctx().data_mut(|d| d.get_temp(egui::Id::new("texture_debug")).unwrap_or(false));
                
                let depth_ratio_opt = if debug_depth_color {
                    Some((face.z_depth - min_z_all) / z_range)
                } else {
                    None
                };
                
                crate::render::object_viewport::draw_face_quad(
                    &painter, &screen_poly, original_screen_poly.as_deref(), &face.triangles, face.face_id, face.part,
                    tex_cache, *face.shading, face.wire_stroke,
                    &app.viewport_camera, show_uv_dots, false, texture_debug, depth_ratio_opt
                );
                // draw_face_quad zaten kenar çizgilerini ve UV dots çiziyor, devam et
                continue;
            }

            // Kenar çizgileri artık GPU Line Pipeline tarafından depth-testli çiziliyor.
            // CPU Painter'da çizersek, depth testi olmadığı için arka yüzlerin
            // kenar çizgileri de görünür olur → "hatlar çok" sorunu.

            // UV Dot Noktaları — sadece toggle açıksa
            if show_uv_dots {
                for p_3d in face.poly_3d.iter() {
                    let p = project_3d_with_z(p_3d[0], p_3d[1], p_3d[2]).0;
                    painter.circle_filled(p, 4.0, egui::Color32::GREEN);
                }
            }
        }
        
        // POST-PASS: Hata ayıklama (Debug) metinlerini en üst katmanda (z-index önceliği) çiz!
        let debug_draw_order = ui.data_mut(|d| d.get_temp(egui::Id::new("debug_draw_order")).unwrap_or(false));
        if debug_z_depth || debug_labels || debug_draw_order {
            for (draw_idx, face) in all_faces.iter().enumerate() {
                if show_outer_shell || (face.is_culled && !debug_culling) { continue; }
                
                let mut cx = 0.0; let mut cy = 0.0; let mut cz = 0.0;
                if face.poly_3d.len() > 0 {
                    for p in face.poly_3d.iter() {
                        cx += p[0]; cy += p[1]; cz += p[2];
                    }
                    let len = face.poly_3d.len() as f32;
                    cx /= len; cy /= len; cz /= len;
                }
                let center_screen = project_3d_with_z(cx, cy, cz).0;
                
                if debug_z_depth {
                    let bg_color = egui::Color32::from_black_alpha(150);
                    painter.rect_filled(
                        egui::Rect::from_center_size(center_screen, egui::vec2(40.0, 16.0)),
                        2.0, bg_color
                    );
                    painter.text(center_screen, egui::Align2::CENTER_CENTER, format!("{:.1}", face.z_depth), egui::FontId::proportional(12.0), egui::Color32::WHITE);
                }

                if debug_labels {
                    let text = format!("P:{} | F:{}", face.part_index, face.face_id);
                    let label_pos = center_screen - egui::vec2(0.0, 15.0);
                    painter.rect_filled(
                        egui::Rect::from_center_size(label_pos, egui::vec2(80.0, 16.0)),
                        2.0, egui::Color32::from_black_alpha(150)
                    );
                    painter.text(label_pos, egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(10.0), egui::Color32::GREEN);
                }

                if debug_draw_order {
                    let text = format!("#{}", draw_idx);
                    let label_pos = center_screen + egui::vec2(0.0, 15.0);
                    painter.rect_filled(
                        egui::Rect::from_center_size(label_pos, egui::vec2(40.0, 16.0)),
                        2.0, egui::Color32::from_black_alpha(150)
                    );
                    painter.text(label_pos, egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(12.0), egui::Color32::from_rgb(255, 200, 0));
                }
            }
        }
        
        let _t6 = std::time::Instant::now();
        
        let frame_time = t0.elapsed().as_secs_f64();
        app.last_engine_frame_time = frame_time;
        

                }
    });
}

// 2D'ye izdüşüm yaparak earcutr ile poligonu (içbükey ve delikli dâhil) üçgenleştirir.
// Bu fonksiyon, özellikle CSG fark (difference) işlemleriyle oluşturulan boşluklu (hole) veya içbükey
// (concave) kemer gibi şekillerin ortasının hatalı boyanmasını engeller.
fn triangulate_3d(poly: &[[f32; 3]]) -> Vec<usize> {
    let n = poly.len();
    if n < 3 { return vec![]; }
    if n == 3 { return vec![0, 1, 2]; }
    
    // Poligon normalini (Newell's Method) hesapla
    let mut nx = 0.0; let mut ny = 0.0; let mut nz = 0.0;
    for i in 0..n {
        let v1 = poly[i];
        let v2 = poly[(i + 1) % n];
        nx += (v1[1] - v2[1]) * (v1[2] + v2[2]);
        ny += (v1[2] - v2[2]) * (v1[0] + v2[0]);
        nz += (v1[0] - v2[0]) * (v1[1] + v2[1]);
    }
    let len = (nx*nx + ny*ny + nz*nz).sqrt();
    if len > 0.0001 { nx /= len; ny /= len; nz /= len; } else { nx = 0.0; ny = 0.0; nz = 1.0; }
    
    // 3D'den 2D düzleme iz düşüm (Orthogonal basis projection)
    let ux = if nx.abs() > 0.5 { 0.0 } else { 1.0 };
    let uy = if nx.abs() > 0.5 { 1.0 } else { 0.0 };
    let uz = 0.0;
    let cross_x = uy*nz - uz*ny;
    let cross_y = uz*nx - ux*nz;
    let cross_z = ux*ny - uy*nx;
    let clen = (cross_x*cross_x + cross_y*cross_y + cross_z*cross_z).sqrt();
    let (vx, vy, vz) = (cross_x/clen, cross_y/clen, cross_z/clen);
    let (ux, uy, uz) = (ny*vz - nz*vy, nz*vx - nx*vz, nx*vy - ny*vx);
    
    let mut data = Vec::with_capacity(n * 2);
    for v in poly {
        let p2x = v[0]*ux + v[1]*uy + v[2]*uz;
        let p2y = v[0]*vx + v[1]*vy + v[2]*vz;
        data.push(p2x as f64);
        data.push(p2y as f64);
    }
    
    // earcutr kullanarak hatasız üçgenleştir!
    let triangles_result = earcutr::earcut(&data, &Vec::new(), 2);
    
    let triangles = match triangles_result {
        Ok(t) => t,
        Err(_) => Vec::new(),
    };
    
    if triangles.is_empty() {
        // Fallback: Earcut başarısız olursa Triangle Fan kullan
        let mut fallback = Vec::new();
        for i in 1..(n - 1) {
            fallback.push(0);
            fallback.push(i);
            fallback.push(i+1);
        }
        return fallback;
    }
    
    triangles
}
