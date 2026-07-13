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
        ui.heading("3D Görünüm");
        ui.label("Dokular bu panelde canlı gösterilir. Sol tık: döndür · Orta/sağ tık: kaydır · Tekerlek: yakınlaştır");
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut app.camera_ortho, "İzometrik Sabit Kamera (Kilitli)");
            ui.checkbox(&mut app.show_gizmo, "XYZ Referans Kollarını (Gizmo) Göster");
            
            let mut show_uv_dots = ui.data_mut(|d| d.get_temp(egui::Id::new("show_uv_dots")).unwrap_or(false));
            if ui.checkbox(&mut show_uv_dots, "Hücre Noktalarını (UV Dots) Göster").changed() {
                ui.data_mut(|d| d.insert_temp(egui::Id::new("show_uv_dots"), show_uv_dots));
            }

            let mut show_outer_shell = ui.data_mut(|d| d.get_temp(egui::Id::new("show_outer_shell")).unwrap_or(false));
            if ui.checkbox(&mut show_outer_shell, "Dış Kabuğu (Air Contact) Göster").changed() {
                ui.data_mut(|d| d.insert_temp(egui::Id::new("show_outer_shell"), show_outer_shell));
            }

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
        let project_3d_with_z = |x: f32, y: f32, z: f32| -> (egui::Pos2, f32) {
            let is_ortho = app.camera_ortho;
            
            let cx = if is_ortho { 30.0_f32.to_radians() } else { cam_rot_x.to_radians() };
            let cy = if is_ortho { 45.0_f32.to_radians() } else { cam_rot_y.to_radians() };
            
            // Y ekseni etrafında (Yaw)
            let x1 = x * cy.cos() + z * cy.sin();
            let z1 = -x * cy.sin() + z * cy.cos();
            // X ekseni etrafında (Pitch)
            let y2 = y * cx.cos() - z1 * cx.sin();
            let z2 = y * cx.sin() + z1 * cx.cos(); // Derinlik (Z-sorting için aktif!)
            
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

        let sel_idx = app.selected_index.unwrap_or(0);
        if sel_idx < app.objects.len() {
            let obj = &app.objects[sel_idx];
            // PERFORMANS: Eskiden burada TÜM dokular her frame sıfırdan
            // compose() ediliyordu. Artık kalıcı app.texture_cache sadece
            // gerçekten değişen dokuları yeniden hesaplıyor; değişmeyenler
            // için ucuz bir hash kontrolüyle anında geçiliyor.
            sync_texture_cache(&mut app.texture_cache, &app.textures);
            let tex_cache = &app.texture_cache;
            
            
            // --- Hiyerarşi (Parenting) Çözümleyici ---
            // Parçaların pozisyonlarını ve rotasyonlarını üst parçalara (Parent) göre kümülatif hesapla.
            
            // --- Parametric Evaluation ---
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
            
            // Build evaluated values for all parts
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
                let mut local_map = eval_map.clone(); // Kopya oluştur
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
            
            let mut resolved_transforms: std::collections::HashMap<String, (f32, f32, f32, f32, f32, f32, f32, f32, f32)> = std::collections::HashMap::new(); // (px, py, pz, rx, ry, rz, sx, sy, sz)
            
            // Basit bir çözücü (Dependency Tree) - 10 derinliğe kadar iteratif çözüm
            for _ in 0..10 {
                for part in &obj.parts {
                    if resolved_transforms.contains_key(&part.id) { continue; }
                    
                    if let Some(parent_id) = &part.parent_part_id {
                        if let Some(parent_t) = resolved_transforms.get(parent_id) {
                            // Parent resolved, apply parent transform to child's local pos
                            let (ppx, ppy, ppz, prx, pry, prz, psx, psy, psz) = parent_t;
                            let px_rad = prx.to_radians(); let py_rad = pry.to_radians(); let pz_rad = prz.to_radians();
                            
                            // Child local pos offset from parent
                            let (lx, ly, lz, lsx, lsy, lsz, lrx, lry, lrz) = evaluated_parts.get(&part.id).unwrap();
                            
                            // 1. Scale local offset by parent scale
                            let lx = lx * psx; let ly = ly * psy; let lz = lz * psz;
                            
                            // 2. Rotate local pos by parent rotation
                            let y1 = ly * px_rad.cos() - lz * px_rad.sin(); let z1 = ly * px_rad.sin() + lz * px_rad.cos();
                            let x2 = lx * py_rad.cos() + z1 * py_rad.sin(); let z2 = -lx * py_rad.sin() + z1 * py_rad.cos();
                            let x3 = x2 * pz_rad.cos() - y1 * pz_rad.sin(); let y3 = x2 * pz_rad.sin() + y1 * pz_rad.cos();
                            
                            // 3. Final Accumulated Transforms
                            let final_x = ppx + x3; let final_y = ppy + y3; let final_z = ppz + z2;
                            let final_rx = prx + lrx; let final_ry = pry + lry; let final_rz = prz + lrz;
                            let final_sx = psx * lsx; let final_sy = psy * lsy; let final_sz = psz * lsz;
                            
                            resolved_transforms.insert(part.id.clone(), (final_x, final_y, final_z, final_rx, final_ry, final_rz, final_sx, final_sy, final_sz));
                        }
                    } else {
                        // Kök (Root) obje
                        let eval_v = evaluated_parts.get(&part.id).unwrap();
                        resolved_transforms.insert(part.id.clone(), (eval_v.0, eval_v.1, eval_v.2, eval_v.6, eval_v.7, eval_v.8, eval_v.3, eval_v.4, eval_v.5));
                    }
                }
            }

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

struct FaceToDraw<'a> {
                screen_poly: Vec<egui::Pos2>,
                z_depth: f32,
                face_id: &'static str,
                part_index: usize,
                part: &'a ObjectPart,
                shading: ShadingMode,
                wire_stroke: egui::Stroke,
                destroyed_ratio: f32,
            }
            let mut all_faces = Vec::new();
            
            struct Volume<'a> {
                planes: Vec<Plane>,
                part_id: String,
                target_id: Option<String>,
                is_subtract: bool,
                part_ref: &'a ObjectPart,
            }
            let mut volumes = Vec::new();

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

                let rotate = |vx: f32, vy: f32, vz: f32| -> (f32, f32, f32) {
                    let rx = rot_x.to_radians(); let ry = rot_y.to_radians(); let rz = rot_z.to_radians();
                    let ox = vx - actual_pivot[0]; let oy = vy - actual_pivot[1]; let oz = vz - actual_pivot[2];
                    
                    let y1 = oy * rx.cos() - oz * rx.sin(); let z1 = oy * rx.sin() + oz * rx.cos();
                    let x2 = ox * ry.cos() + z1 * ry.sin(); let z2 = -ox * ry.sin() + z1 * ry.cos();
                    let x3 = x2 * rz.cos() - y1 * rz.sin(); let y3 = x2 * rz.sin() + y1 * rz.cos();
                    
                    (x3 + actual_pivot[0], y3 + actual_pivot[1], z2 + actual_pivot[2])
                };

                let mut transform_world = |lx: f32, ly: f32, lz: f32| -> [f32; 3] {
                    let sx_mod = lx * sx + ly * shear_x;
                    let sy_mod = ly * sy + lx * shear_y;
                    let sz_mod = lz * sz + ly * shear_z;
                    let (rx, ry, rz) = rotate(sx_mod, sy_mod, sz_mod);
                    let mut fx = base_x + rx;
                    let mut fy = base_y + ry;
                    let mut fz = base_z + rz;
                    fx *= gs_x; fy *= gs_y; fz *= gs_z;
                    
                    let grx_rad = gr_x.to_radians(); let gry_rad = gr_y.to_radians(); let grz_rad = gr_z.to_radians();
                    let g_y1 = fy * grx_rad.cos() - fz * grx_rad.sin(); let g_z1 = fy * grx_rad.sin() + fz * grx_rad.cos();
                    let g_x2 = fx * gry_rad.cos() + g_z1 * gry_rad.sin(); let g_z2 = -fx * gry_rad.sin() + g_z1 * gry_rad.cos();
                    let g_x3 = g_x2 * grz_rad.cos() - g_y1 * grz_rad.sin(); let g_y3 = g_x2 * grz_rad.sin() + g_y1 * grz_rad.cos();
                    
                    [g_x3 + gx, g_y3 + gy, g_z2 + gz]
                };

                let faces = get_part_polygons(&part.shape, &mut transform_world);
                if !faces.is_empty() {
                    let mut planes = Vec::new();
                    for face in faces {
                        if face.verts.len() >= 3 {
                            planes.push(make_plane(face.verts[0], face.verts[1], face.verts[2]));
                        }
                    }
                    volumes.push(Volume { planes, part_id: part.id.clone(), target_id: part.csg_target_id.clone(), is_subtract: part.boolean_op == BooleanOp::Subtract, part_ref: part });
                }
            }

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

                let rotate = |vx: f32, vy: f32, vz: f32| -> (f32, f32, f32) {
                    let rx = rot_x.to_radians(); let ry = rot_y.to_radians(); let rz = rot_z.to_radians();
                    // Origin shift for pivot
                    let ox = vx - actual_pivot[0]; let oy = vy - actual_pivot[1]; let oz = vz - actual_pivot[2];
                    
                    let y1 = oy * rx.cos() - oz * rx.sin(); let z1 = oy * rx.sin() + oz * rx.cos();
                    let x2 = ox * ry.cos() + z1 * ry.sin(); let z2 = -ox * ry.sin() + z1 * ry.cos();
                    let x3 = x2 * rz.cos() - y1 * rz.sin(); let y3 = x2 * rz.sin() + y1 * rz.cos();
                    
                    // Shift back
                    (x3 + actual_pivot[0], y3 + actual_pivot[1], z2 + actual_pivot[2])
                };

                let transform_z = |lx: f32, ly: f32, lz: f32| -> (egui::Pos2, f32) {
                    let sx_mod = lx * sx + ly * shear_x;
                    let sy_mod = ly * sy + lx * shear_y;
                    let sz_mod = lz * sz + ly * shear_z;
                    
                    let (rx, ry, rz) = rotate(sx_mod, sy_mod, sz_mod);
                    
                    let mut fx = base_x + rx;
                    let mut fy = base_y + ry;
                    let mut fz = base_z + rz;
                    
                    fx *= gs_x; fy *= gs_y; fz *= gs_z;
                    
                    let grx_rad = gr_x.to_radians(); let gry_rad = gr_y.to_radians(); let grz_rad = gr_z.to_radians();
                    let g_y1 = fy * grx_rad.cos() - fz * grx_rad.sin(); let g_z1 = fy * grx_rad.sin() + fz * grx_rad.cos();
                    let g_x2 = fx * gry_rad.cos() + g_z1 * gry_rad.sin(); let g_z2 = -fx * gry_rad.sin() + g_z1 * gry_rad.cos();
                    let g_x3 = g_x2 * grz_rad.cos() - g_y1 * grz_rad.sin(); let g_y3 = g_x2 * grz_rad.sin() + g_y1 * grz_rad.cos();
                    
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
                let use_fill = actual_shading == ShadingMode::Solid || actual_shading == ShadingMode::Textured;
                
                if show_gizmo {
                    let g_center = project_3d(base_x, base_y, base_z);
                    painter.line_segment([g_center, project_3d(base_x + 1.0, base_y, base_z)], (1.0, egui::Color32::from_rgb(255, 100, 100)));
                    painter.line_segment([g_center, project_3d(base_x, base_y + 1.0, base_z)], (1.0, egui::Color32::from_rgb(100, 255, 100)));
                    painter.line_segment([g_center, project_3d(base_x, base_y, base_z + 1.0)], (1.0, egui::Color32::from_rgb(100, 150, 255)));
                }

                                let mut transform_world = |lx: f32, ly: f32, lz: f32| -> [f32; 3] {
                    let sx_mod = lx * sx + ly * shear_x;
                    let sy_mod = ly * sy + lx * shear_y;
                    let sz_mod = lz * sz + ly * shear_z;
                    let (rx, ry, rz) = rotate(sx_mod, sy_mod, sz_mod);
                    let mut fx = base_x + rx;
                    let mut fy = base_y + ry;
                    let mut fz = base_z + rz;
                    fx *= gs_x; fy *= gs_y; fz *= gs_z;
                    
                    let grx_rad = gr_x.to_radians(); let gry_rad = gr_y.to_radians(); let grz_rad = gr_z.to_radians();
                    let g_y1 = fy * grx_rad.cos() - fz * grx_rad.sin(); let g_z1 = fy * grx_rad.sin() + fz * grx_rad.cos();
                    let g_x2 = fx * gry_rad.cos() + g_z1 * gry_rad.sin(); let g_z2 = -fx * gry_rad.sin() + g_z1 * gry_rad.cos();
                    let g_x3 = g_x2 * grz_rad.cos() - g_y1 * grz_rad.sin(); let g_y3 = g_x2 * grz_rad.sin() + g_y1 * grz_rad.cos();
                    
                    [g_x3 + gx, g_y3 + gy, g_z2 + gz]
                };

                let mut push_face = |w_arr: &[[f32; 3]], face_id: &'static str| {
                    if w_arr.len() < 3 { return; }
                    let mut s_arr = Vec::with_capacity(w_arr.len());
                    for i in 0..w_arr.len() {
                        s_arr.push(project_3d_with_z(w_arr[i][0], w_arr[i][1], w_arr[i][2]).0);
                    }
                    
                    let cross = (s_arr[1].x - s_arr[0].x) * (s_arr[2].y - s_arr[1].y) - (s_arr[1].y - s_arr[0].y) * (s_arr[2].x - s_arr[1].x);
                    let mut is_facing = cross < -0.1;
                    
                    if part.boolean_op == BooleanOp::Subtract {
                        is_facing = true; // DELİK (-) OBJELERİNDE YÜZEY GİZLEME YAPMA! Bırak CSG motoru dışarıda kalanları budasın.
                    }
                    
                    // Sadece dolgu modunda arkaya bakanları gizle (Wireframe'de her şeyi çiziyoruz)
                    if use_fill && !is_facing { return; }
                    
                    // FRUSTUM CULLING (Ekran Dışı Eleme) - O(1) Optimizasyonu
                    // Eğer yüzeyin kapladığı maksimum/minimum 2D alan ekran sınırlarının tamamen dışındaysa 
                    // (kesişim/delinme hesaplarına hiç sokmadan) çöpe at!
                    let clip_rect = painter.clip_rect();
                    let mut min_x = f32::MAX;
                    let mut max_x = f32::MIN;
                    let mut min_y = f32::MAX;
                    let mut max_y = f32::MIN;
                    for p in &s_arr {
                        if p.x < min_x { min_x = p.x; }
                        if p.x > max_x { max_x = p.x; }
                        if p.y < min_y { min_y = p.y; }
                        if p.y > max_y { max_y = p.y; }
                    }
                    if max_x < clip_rect.min.x || min_x > clip_rect.max.x || max_y < clip_rect.min.y || min_y > clip_rect.max.y {
                        return;
                    }
                    
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
                    
                    let mut original_max_z = f32::MIN;
                    for w in w_arr {
                        let (_, z) = project_3d_with_z(w[0], w[1], w[2]);
                        if z > original_max_z { original_max_z = z; }
                    }
                    
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
                                let (_, inside) = crate::render::csg::split_poly(&current, plane, false);
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
                            
                            let mut inner_wall = intersect_convex(&initial_poly, &vol.planes);
                            if !inner_wall.is_empty() {
                                // İç duvarların görünmesi için yüzey yönünü TERSİNE çevir (Arka yüzey sorunu çözümü)
                                inner_wall.reverse();
                                
                                // Doğrudan all_faces'e ekle, böylece hedefin rengini (shading) miras alabilir
                                let mut sum_z = 0.0;
                                let mut valid_count = 0;
                                let mut screen_poly = Vec::with_capacity(inner_wall.len());
                                for v in &inner_wall {
                                    if !v.pos[0].is_finite() || !v.pos[1].is_finite() || !v.pos[2].is_finite() { continue; }
                                    let proj = project_3d_with_z(v.pos[0], v.pos[1], v.pos[2]);
                                    screen_poly.push(proj.0);
                                    sum_z += proj.1;
                                    valid_count += 1;
                                }
                                let max_z = if valid_count > 0 { sum_z / (valid_count as f32) } else { f32::MIN };
                                
                                if screen_poly.len() >= 3 {
                                    if use_fill {
                                        // Ters çevrilen iç duvarlar için yeniden Culling (yüzey eleme) yap.
                                        // Kameraya arkasını dönen iç duvarlar çizilmemeli!
                                        let cross = (screen_poly[1].x - screen_poly[0].x) * (screen_poly[2].y - screen_poly[1].y) - (screen_poly[1].y - screen_poly[0].y) * (screen_poly[2].x - screen_poly[1].x);
                                        if cross >= -0.1 { continue; }
                                    }
                                    // vol.part_ref.shading klonlanarak hedefin rengi alınır.
                                    // Seçim için (tıklama) hala orijinal 'part' referansını veriyoruz.
                                    all_faces.push(FaceToDraw { 
                                        screen_poly, 
                                        z_depth: max_z, 
                                        face_id, 
                                        part_index,
                                        part, 
                                        shading: parse_shading(&vol.part_ref.shading_model), 
                                        wire_stroke: egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(0, 0, 0, 50)), 
                                        destroyed_ratio: 0.0 
                                    });
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
                        
                        let mut screen_poly = Vec::with_capacity(poly.len());
                        for v in &poly {
                            if !v.pos[0].is_finite() || !v.pos[1].is_finite() || !v.pos[2].is_finite() { continue; }
                            let proj = project_3d_with_z(v.pos[0], v.pos[1], v.pos[2]);
                            screen_poly.push(proj.0);
                        }
                        // Z-fighting'i önlemek için objenin parçalanmış hali (pie slices vb.) değil,
                        // kesilmeden önceki orjinal yüzeyinin kameraya en yakın (Maximum Z) derinliği kullanılır.
                        let max_z = original_max_z;
                        if screen_poly.len() < 3 { continue; }
                        
                        // Delik objesinin iç duvarları ise tam Dolgu (Solid) çizilsin!
                        let mut final_shading = actual_shading;
                        if is_inner_wall { final_shading = ShadingMode::Solid; }
                        
                        all_faces.push(FaceToDraw { screen_poly, z_depth: max_z, face_id, part_index, part, shading: final_shading, wire_stroke, destroyed_ratio });
                    }
                };

                let faces = get_part_polygons(&part.shape, &mut transform_world);
                for face in faces {
                    push_face(&face.verts, face.face_id);
                            }

        }
        
        // Alan (Area) Hesaplama Algoritması (Shoelace Formula)
        let poly_area = |poly: &[egui::Pos2]| -> f32 {
            let mut area = 0.0;
            if poly.len() < 3 { return 0.0; }
            for i in 0..poly.len() {
                let p1 = poly[i];
                let p2 = poly[(i + 1) % poly.len()];
                area += p1.x * p2.y - p2.x * p1.y;
            }
            area.abs() * 0.5
        };

        // Yüzeyleri Uzaklıktan Yakına Doğru Sırala (Painter's Algorithm)
        // Z-Depth değerini 10000.0 ile çarparak yüksek hassasiyetli gruplama yapıyoruz.
        // max_z kullandığımız için coplanar sorunları çözüldü, arkada olan yüzeyler kesinlikle arkada kalacak!
        all_faces.sort_by(|a, b| {
            let z_a_key = (a.z_depth * 10000.0).round() as i32;
            let z_b_key = (b.z_depth * 10000.0).round() as i32;
            
            let cmp = z_a_key.cmp(&z_b_key);
            if cmp == std::cmp::Ordering::Equal {
                // TAM OLARAK ÇAKIŞAN YÜZEYLER (Coplanar Z-Fighting)
                // Çözüm: Outliner (app.objects) sırasına göre öncelik ver.
                // Sonradan eklenen (veya listede aşağıda olan) obje ÜSTTE çizilir (Z-Index mantığı).
                // Yani part_index'i BÜYÜK olan SONRA çizilmeli (sort_by'da Daha Büyük (Greater) döndürülür ki sona atılsın).
                let index_cmp = a.part_index.cmp(&b.part_index);
                if index_cmp == std::cmp::Ordering::Equal {
                    // Aynı objenin parçalarıysa eski yönteme dön
                    let r_cmp = a.destroyed_ratio.partial_cmp(&b.destroyed_ratio).unwrap_or(std::cmp::Ordering::Equal);
                    if r_cmp == std::cmp::Ordering::Equal {
                        let area_a = poly_area(&a.screen_poly);
                        let area_b = poly_area(&b.screen_poly);
                        area_b.partial_cmp(&area_a).unwrap_or(std::cmp::Ordering::Equal)
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
        
        let show_uv_dots = ui.data_mut(|d| d.get_temp(egui::Id::new("show_uv_dots")).unwrap_or(false));
        let show_outer_shell = ui.data_mut(|d| d.get_temp(egui::Id::new("show_outer_shell")).unwrap_or(false));
        
        for face in all_faces {
            crate::render::object_viewport::draw_face_quad(&painter, &face.screen_poly, face.face_id, face.part, &tex_cache, face.shading, face.wire_stroke, &app.viewport_camera, show_uv_dots, show_outer_shell);
        }
        
                }
    });
}
