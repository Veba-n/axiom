use eframe::egui;
use crate::app::AxiomStudio;
use crate::data::scene::EditorScene;

struct TreeNode {
    idx: usize,
    id: String,
    layers_count: usize,
    children: Vec<TreeNode>,
}

fn build_tree(elements: &[crate::data::element::UiElement], parent_id: Option<&str>) -> Vec<TreeNode> {
    let mut nodes = vec![];
    for (i, el) in elements.iter().enumerate() {
        if el.parent_id.as_deref() == parent_id {
            nodes.push(TreeNode {
                idx: i,
                id: el.id.clone(),
                layers_count: el.layers.len(),
                children: build_tree(elements, Some(&el.id)),
            });
        }
    }
    nodes
}

fn draw_tree_node(ui: &mut egui::Ui, node: &TreeNode, selected_index: &mut Option<usize>, last_selected: &mut Option<usize>) {
    let label = format!("{} ({})", node.id, node.layers_count);
    if node.children.is_empty() {
        if ui.selectable_label(*selected_index == Some(node.idx), label).clicked() {
            *selected_index = Some(node.idx);
            *last_selected = Some(node.idx);
        }
    } else {
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), ui.make_persistent_id(&node.id), true)
            .show_header(ui, |ui| {
                if ui.selectable_label(*selected_index == Some(node.idx), label).clicked() {
                    *selected_index = Some(node.idx);
                    *last_selected = Some(node.idx);
                }
            })
            .body(|ui| {
                for child in &node.children {
                    draw_tree_node(ui, child, selected_index, last_selected);
                }
            });
    }
}

pub fn show(app: &mut AxiomStudio, ctx: &egui::Context) {
    egui::SidePanel::left("explorer_panel").resizable(true).default_width(200.0).show(ctx, |ui| {
        ui.heading("📁 Explorer");
        ui.separator();
        ui.label("Menüler / Sahneler:");
        let mut scene_to_delete = None;
        
        egui::ScrollArea::vertical().id_source("scene_list").max_height(200.0).show(ui, |ui| {
            let scenes_len = app.scenes.len();
            for (s_idx, scene) in app.scenes.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    if ui.selectable_label(app.active_scene == s_idx, "").clicked() {
                        app.active_scene = s_idx;
                        app.selected_index = None;
                    }
                    ui.text_edit_singleline(&mut scene.name);
                    if scenes_len > 1 && ui.button("🗑").clicked() {
                        scene_to_delete = Some(s_idx);
                    }
                });
            }
            if ui.button("➕ Yeni Menü Ekle").clicked() {
                app.scenes.push(EditorScene {
                    name: format!("Yeni Menü {}", app.scenes.len() + 1),
                    elements: vec![],
                });
            }
        });
        
        if let Some(s_idx) = scene_to_delete {
            app.scenes.remove(s_idx);
            if app.active_scene >= app.scenes.len() {
                app.active_scene = app.scenes.len() - 1;
            }
        }

        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Aktif Sahne Objeleri:");
            if ui.button("➕ Obje Ekle").clicked() {
                let mut el = crate::data::element::UiElement::default();
                el.id = format!("Obje_{}", app.scenes[app.active_scene].elements.len() + 1);
                
                for layer in &mut el.layers {
                    layer.font_family = app.settings.default_font_family.clone();
                    layer.font_size = app.settings.default_font_size;
                }
                
                app.scenes[app.active_scene].elements.push(el);
            }
        });
        
        let tree_roots = build_tree(&app.scenes[app.active_scene].elements, None);
        
        egui::ScrollArea::vertical().id_source("left_element_list").show(ui, |ui| {
            for root in &tree_roots {
                draw_tree_node(ui, root, &mut app.selected_index, &mut app.last_selected);
            }
        });
    });
}
