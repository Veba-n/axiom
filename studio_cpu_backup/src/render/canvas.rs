use eframe::egui;
use crate::app::AxiomStudio;
use crate::core::types::{Anchor, GradientDirection, TextAlign, TextValign, AnimationType};
use crate::data::layer::LayerKind;
use crate::data::border::BorderPiece;
use crate::core::interaction::InteractionState;
use crate::data::element::UiElement;



fn compute_absolute_rect(
    idx: usize, 
    scene_idx: usize,
    elements: &[UiElement],
    cache: &mut std::collections::HashMap<(usize, usize), (f32, f32, isize, isize, f32, f32, usize, usize)>,
    render_w: f32, render_h: f32, grid_cols: usize, grid_rows: usize, char_width: f32, char_height: f32, rect_min_x: f32, rect_min_y: f32
) -> (f32, f32, isize, isize, f32, f32, usize, usize) {
    if let Some(&val) = cache.get(&(scene_idx, idx)) { return val; }
    
    let el = &elements[idx];
    let (p_start_x, p_start_y, p_w, p_h) = if let Some(pid) = &el.parent_id {
        if let Some(p_idx) = elements.iter().position(|e| &e.id == pid) {
            let p_val = compute_absolute_rect(p_idx, scene_idx, elements, cache, render_w, render_h, grid_cols, grid_rows, char_width, char_height, rect_min_x, rect_min_y);
            (p_val.0, p_val.1, p_val.4, p_val.5)
        } else {
            (rect_min_x, rect_min_y, render_w, render_h)
        }
    } else {
        (rect_min_x, rect_min_y, render_w, render_h)
    };
    
    let anchor_x = match el.anchor {
        Anchor::TopLeft | Anchor::BottomLeft => 0.0,
        Anchor::TopCenter | Anchor::Center | Anchor::BottomCenter => p_w / 2.0,
        Anchor::TopRight | Anchor::BottomRight => p_w,
    };
    let anchor_y = match el.anchor {
        Anchor::TopLeft | Anchor::TopCenter | Anchor::TopRight => 0.0,
        Anchor::Center => p_h / 2.0,
        Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight => p_h,
    };

    let offset_x = (el.pos_x / 100.0) * render_w;
    let offset_y = (el.pos_y / 100.0) * render_h;
    
    let w_chars = ((el.width / 100.0) * grid_cols as f32).max(2.0) as usize;
    let h_chars = ((el.height / 100.0) * grid_rows as f32).max(2.0) as usize;
    let b_width = w_chars as f32 * char_width;
    let b_height = h_chars as f32 * char_height;
    
    let center_x = p_start_x + anchor_x + offset_x;
    let center_y = p_start_y + anchor_y + offset_y;
    
    let start_c = ((center_x - rect_min_x) / char_width) as isize - (w_chars as isize / 2);
    let start_r = ((center_y - rect_min_y) / char_height) as isize - (h_chars as isize / 2);
    
    let start_x = rect_min_x + (start_c as f32 * char_width);
    let start_y = rect_min_y + (start_r as f32 * char_height);
    
    let val = (start_x, start_y, start_c, start_r, b_width, b_height, w_chars, h_chars);
    cache.insert((scene_idx, idx), val);
    val
}

pub fn show(app: &mut AxiomStudio, ctx: &egui::Context, time: f32) {

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("720p Saf Oyun Motoru Canvası (1280x720) - Live Preview");
            ui.separator();

            let available = ui.available_size();
            let mut scale = (available.x / 1280.0).min(available.y / 720.0);
            if scale > 1.5 { scale = 1.5; }

            let render_w = 1280.0 * scale;
            let render_h = 720.0 * scale;

            let (rect, _response) = ui.allocate_exact_size(egui::vec2(render_w, render_h), egui::Sense::hover());
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(10, 10, 10));

            let char_width = 12.0 * scale;
            let char_height = 20.0 * scale;
            let grid_cols = (app.settings.resolution_w / 12.0) as usize; 
            let grid_rows = (app.settings.resolution_h / 20.0) as usize;  

            if grid_cols == 0 || grid_rows == 0 { return; }
            let _font = egui::FontId::monospace(char_height);

            let mut drag_deltas = std::collections::HashMap::new();
            let mut resize_deltas = std::collections::HashMap::new();
            let mut clicked_idx = None;
            let mut next_interaction_states = std::collections::HashMap::new();

            let scenes_to_render = if app.is_playing { app.preview_stack.clone() } else { vec![app.active_scene] };
            
            let mut sorted_elements = Vec::new();
            for (_order, &s_idx) in scenes_to_render.iter().enumerate() {
                if s_idx < app.scenes.len() {
                    let mut s_elements: Vec<_> = app.scenes[s_idx].elements.iter().enumerate().map(|(i, e)| (s_idx, i, e)).collect();
                    s_elements.sort_by_key(|(_, _, el)| el.z_index);
                    sorted_elements.extend(s_elements);
                }
            }


            let rotate_point = |px: f32, py: f32, cx: f32, cy: f32, angle_deg: f32| -> (f32, f32) {
                if angle_deg == 0.0 { return (px, py); }
                let rad = angle_deg.to_radians();
                let cos_a = rad.cos();
                let sin_a = rad.sin();
                let dx = px - cx;
                let dy = py - cy;
                (cx + dx * cos_a - dy * sin_a, cy + dx * sin_a + dy * cos_a)
            };

            let mut rect_cache = std::collections::HashMap::new();

            for &(scene_idx, idx, el) in &sorted_elements {
                let (start_x, start_y, start_c, start_r, b_width, b_height, w_chars, h_chars) = compute_absolute_rect(
                    idx, scene_idx, &app.scenes[scene_idx].elements, &mut rect_cache, 
                    render_w, render_h, grid_cols, grid_rows, char_width, char_height, rect.min.x, rect.min.y
                );


                let element_rect = egui::Rect::from_min_size(
                    egui::pos2(start_x, start_y),
                    egui::vec2(b_width, b_height)
                );

                let id = ui.id().with(scene_idx).with(idx);
                let resize_rect = egui::Rect::from_min_size(
                    egui::pos2(start_x + b_width - 15.0, start_y + b_height - 15.0),
                    egui::vec2(15.0, 15.0)
                );
                
                let mut is_hovered = if app.is_playing && (app.focused_index == Some(idx) && app.active_scene == scene_idx) { true } else { false };
                let mut is_pressed = false;

                let resize_response = ui.interact(resize_rect, id.with("resize"), egui::Sense::drag());
                if resize_response.dragged() {
                    resize_deltas.insert((scene_idx, idx), resize_response.drag_delta());
                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeNwSe);
                    is_pressed = true;
                } else if resize_response.hovered() {
                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeNwSe);
                    is_hovered = true;
                } else {
                    let drag_response = ui.interact(element_rect, id, egui::Sense::click_and_drag());
                    if drag_response.dragged() {
                        drag_deltas.insert((scene_idx, idx), drag_response.drag_delta());
                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grab);
                        is_pressed = true;
                    } else if drag_response.hovered() {
                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grab);
                        painter.rect_stroke(element_rect, 0.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 200, 255)));
                        is_hovered = true;
                    }
                    if drag_response.is_pointer_button_down_on() { is_pressed = true; }
                    if drag_response.clicked() { clicked_idx = Some((scene_idx, idx)); }
                }

                if resize_response.hovered() || resize_response.dragged() {
                    painter.rect_filled(resize_rect, 0.0, egui::Color32::from_rgb(255, 200, 50));
                }

                next_interaction_states.insert((scene_idx, idx), InteractionState { is_hovered, is_pressed });

                // Render layers
                let mut layers_to_render = el.layers.clone();
                layers_to_render.sort_by_key(|l| l.z_index);

                let _center_x = start_x + (b_width / 2.0);
                let _center_y = start_y + (b_height / 2.0);

                for layer in layers_to_render {
                    if !layer.enabled { continue; }
                    
                    let font_size_px = layer.font_size * layer.font_scale * scale;
                    let layer_font = if layer.font_family == "Proportional" {
                        egui::FontId::proportional(font_size_px)
                    } else {
                        egui::FontId::monospace(font_size_px)
                    };
                    
                    let mut fg = egui::Color32::from_rgba_unmultiplied(layer.fg_color[0], layer.fg_color[1], layer.fg_color[2], (layer.alpha * 255.0) as u8);
                    let mut bg = egui::Color32::from_rgba_unmultiplied(layer.bg_color[0], layer.bg_color[1], layer.bg_color[2], (layer.alpha * layer.bg_alpha * 255.0) as u8);

                    if let Some(state) = app.interaction_states.get(&(scene_idx, idx)) {
                        if state.is_pressed && layer.pressed_state.enabled {
                            fg = egui::Color32::from_rgba_unmultiplied(layer.pressed_state.fg_color[0], layer.pressed_state.fg_color[1], layer.pressed_state.fg_color[2], (layer.alpha * 255.0) as u8);
                            bg = egui::Color32::from_rgba_unmultiplied(layer.pressed_state.bg_color[0], layer.pressed_state.bg_color[1], layer.pressed_state.bg_color[2], (layer.alpha * layer.bg_alpha * 255.0) as u8);
                        } else if state.is_hovered && layer.hover_state.enabled {
                            fg = egui::Color32::from_rgba_unmultiplied(layer.hover_state.fg_color[0], layer.hover_state.fg_color[1], layer.hover_state.fg_color[2], (layer.alpha * 255.0) as u8);
                            bg = egui::Color32::from_rgba_unmultiplied(layer.hover_state.bg_color[0], layer.hover_state.bg_color[1], layer.hover_state.bg_color[2], (layer.alpha * layer.bg_alpha * 255.0) as u8);
                        }
                    }

                    let mut layer_off_y = (layer.offset_y / 100.0) * render_h;
                    let layer_off_x = (layer.offset_x / 100.0) * render_w;

                    match layer.animation {
                        AnimationType::Wave => {
                            layer_off_y += (time * layer.anim_speed).sin() * layer.anim_amplitude * scale;
                        }
                        AnimationType::Blink => {
                            if (time * layer.anim_speed).sin() > 0.0 { continue; }
                        }
                        AnimationType::PulseColor => {
                            let intensity = ((time * layer.anim_speed).sin() * 0.5 + 0.5) * 255.0;
                            let apply_pulse = |c: &mut egui::Color32| {
                                *c = egui::Color32::from_rgb(
                                    (c.r() as f32 * intensity / 255.0) as u8,
                                    (c.g() as f32 * intensity / 255.0) as u8,
                                    (c.b() as f32 * intensity / 255.0) as u8,
                                );
                            };
                            apply_pulse(&mut fg);
                        }
                        _ => {}
                    }

                    let l_w_chars = ((layer.width / 100.0) * w_chars as f32).max(1.0) as usize;
                    let l_h_chars = ((layer.height / 100.0) * h_chars as f32).max(1.0) as usize;
                    
                    let l_start_c = start_c + (layer_off_x / char_width) as isize;
                    let l_start_r = start_r + (layer_off_y / char_height) as isize;

                    match layer.kind {
                        LayerKind::Fill | LayerKind::Border => {
                            let fill_chars: Vec<char> = if layer.content.is_empty() { vec![' '] } else { layer.content.chars().collect() };
                            let _top_pat: Vec<char> = layer.border.top_pattern.pattern.chars().collect();
                            let _bot_pat: Vec<char> = layer.border.bottom_pattern.pattern.chars().collect();
                            let _left_pat: Vec<char> = layer.border.left_pattern.pattern.chars().collect();
                            let _right_pat: Vec<char> = layer.border.right_pattern.pattern.chars().collect();

                            let safe_char = |pat: &Vec<char>, c_idx: usize| -> char {
                                if pat.is_empty() { ' ' } else { pat[c_idx % pat.len()] }
                            };

                            for r in 0..l_h_chars {
                                for c in 0..l_w_chars {
                                    let mut ch = if layer.kind == LayerKind::Fill {
                                        safe_char(&fill_chars, r * l_w_chars + c)
                                    } else {
                                        ' '
                                    };

                                    let mut cell_fg = fg;
                                    let mut cell_bg = bg;
                                    
                                    if layer.use_gradient {
                                        let t = match layer.gradient_dir {
                                            GradientDirection::Horizontal => c as f32 / l_w_chars.max(1) as f32,
                                            GradientDirection::Vertical => r as f32 / l_h_chars.max(1) as f32,
                                            GradientDirection::Diagonal => (c as f32 + r as f32) / (l_w_chars + l_h_chars).max(1) as f32,
                                        };
                                        let lerp = |a: u8, b: u8, t: f32| -> u8 { (a as f32 + (b as f32 - a as f32) * t) as u8 };
                                        cell_bg = egui::Color32::from_rgba_unmultiplied(
                                            lerp(layer.bg_color[0], layer.gradient_target[0], t),
                                            lerp(layer.bg_color[1], layer.gradient_target[1], t),
                                            lerp(layer.bg_color[2], layer.gradient_target[2], t),
                                            (layer.alpha * layer.bg_alpha * 255.0) as u8
                                        );
                                    }

                                    if layer.animation == AnimationType::MatrixRain {
                                        let drop_y = (time * layer.anim_speed * 10.0 + c as f32 * 3.7) as usize;
                                        if (drop_y % (l_h_chars + 5)) == r {
                                            cell_fg = egui::Color32::WHITE;
                                            ch = ['0', '1', 'A', 'X', 'I', 'O', 'M'][(r + c + drop_y) % 7];
                                        } else if (drop_y % (l_h_chars + 5)) > r && (drop_y % (l_h_chars + 5)) - r < 4 {
                                            cell_fg = egui::Color32::from_rgb(0, 255, 0);
                                            ch = ['*', '#', '$', '%', '&'][(r + c) % 5];
                                        }
                                    } else if layer.animation == AnimationType::Glitch {
                                        if (time * layer.anim_speed * 50.0 + c as f32 * 13.0 + r as f32 * 7.0).sin() > 0.95 {
                                            ch = ['░', '▒', '▓', '█'][(time * 100.0 + c as f32) as usize % 4];
                                            cell_fg = if (r + c) % 2 == 0 { egui::Color32::RED } else { egui::Color32::from_rgb(0, 255, 255) };
                                        }
                                    } else if layer.animation == AnimationType::Ripple {
                                        let dist = ((c as f32 - l_w_chars as f32 / 2.0).powi(2) + (r as f32 - l_h_chars as f32 / 2.0).powi(2)).sqrt();
                                        if (dist - time * layer.anim_speed * 5.0).sin() > 0.8 {
                                            cell_fg = egui::Color32::WHITE;
                                            cell_bg = egui::Color32::from_rgb(100, 150, 255);
                                        }
                                    }

                                     let mut zigzag_off_x = 0.0;
                                     let mut zigzag_off_y = 0.0;
                                     if (r + c) % 2 != 0 {
                                         zigzag_off_x = layer.zigzag_x * scale;
                                         zigzag_off_y = layer.zigzag_y * scale;
                                     }
                                     let mut is_border = false;
                                     let mut chars_to_draw: Vec<char> = Vec::new();
                                     let is_top = r == 0; let is_bot = r == l_h_chars - 1;
                                     let is_left = c == 0; let is_right = c == l_w_chars - 1;
                                     
                                     if layer.kind == LayerKind::Border {
                                         
                                         let get_chars = |s: &str| -> Vec<char> {
                                             if layer.border_composite && !s.is_empty() { s.chars().collect() }
                                             else if !s.is_empty() { vec![s.chars().next().unwrap()] }
                                             else { vec![' '] }
                                         };

                                         
                                         let mut active_piece: Option<&BorderPiece> = None;
                                         if is_top && is_left { active_piece = Some(&layer.border.top_left); chars_to_draw = get_chars(&layer.border.top_left.pattern); is_border=true; }
                                         else if is_top && is_right { active_piece = Some(&layer.border.top_right); chars_to_draw = get_chars(&layer.border.top_right.pattern); is_border=true; }
                                         else if is_bot && is_left { active_piece = Some(&layer.border.bottom_left); chars_to_draw = get_chars(&layer.border.bottom_left.pattern); is_border=true; }
                                         else if is_bot && is_right { active_piece = Some(&layer.border.bottom_right); chars_to_draw = get_chars(&layer.border.bottom_right.pattern); is_border=true; }
                                         else if is_top { 
                                             active_piece = Some(&layer.border.top_pattern);
                                             if layer.border_composite && !layer.border.top_pattern.pattern.is_empty() { chars_to_draw = layer.border.top_pattern.pattern.chars().collect(); }
                                             else { chars_to_draw = vec![safe_char(&layer.border.top_pattern.pattern.chars().collect(), c.saturating_sub(1))]; }
                                             is_border=true; 
                                         }
                                         else if is_bot { 
                                             active_piece = Some(&layer.border.bottom_pattern);
                                             if layer.border_composite && !layer.border.bottom_pattern.pattern.is_empty() { chars_to_draw = layer.border.bottom_pattern.pattern.chars().collect(); }
                                             else { chars_to_draw = vec![safe_char(&layer.border.bottom_pattern.pattern.chars().collect(), c.saturating_sub(1))]; }
                                             is_border=true; 
                                         }
                                         else if is_left { 
                                             active_piece = Some(&layer.border.left_pattern);
                                             if layer.border_composite && !layer.border.left_pattern.pattern.is_empty() { chars_to_draw = layer.border.left_pattern.pattern.chars().collect(); }
                                             else { chars_to_draw = vec![safe_char(&layer.border.left_pattern.pattern.chars().collect(), r.saturating_sub(1))]; }
                                             is_border=true; 
                                         }
                                         else if is_right { 
                                             active_piece = Some(&layer.border.right_pattern);
                                             if layer.border_composite && !layer.border.right_pattern.pattern.is_empty() { chars_to_draw = layer.border.right_pattern.pattern.chars().collect(); }
                                             else { chars_to_draw = vec![safe_char(&layer.border.right_pattern.pattern.chars().collect(), r.saturating_sub(1))]; }
                                             is_border=true; 
                                         }
                                         
                                         if let Some(p) = active_piece {
                                             if p.color_override { cell_fg = egui::Color32::from_rgb(p.fg_color[0], p.fg_color[1], p.fg_color[2]); }
                                             zigzag_off_x += p.offset_x * scale;
                                             zigzag_off_y += p.offset_y * scale;
                                         }
if !is_border { continue; } // Skip interior for border
                                     } else {
                                         chars_to_draw = vec![ch];
                                     }

                                     let is_corner = is_border && ((r==0 && c==0) || (r==0 && c==l_w_chars-1) || (r==l_h_chars-1 && c==0) || (r==l_h_chars-1 && c==l_w_chars-1));
                                     if !is_corner && layer.pattern_spacing > 1 {
                                         if layer.kind == LayerKind::Border {
                                            if (r == 0 || r == l_h_chars - 1) && c % layer.pattern_spacing != 0 { chars_to_draw.clear(); cell_bg = egui::Color32::TRANSPARENT; }
                                            if (c == 0 || c == l_w_chars - 1) && r % layer.pattern_spacing != 0 { chars_to_draw.clear(); cell_bg = egui::Color32::TRANSPARENT; }
                                         } else if layer.kind == LayerKind::Fill {
                                            if (r + c) % layer.pattern_spacing != 0 { chars_to_draw.clear(); cell_bg = egui::Color32::TRANSPARENT; }
                                         }
                                     }

                                     let actual_char_width = char_width * layer.scale_x;
                                     let actual_char_height = char_height * layer.scale_y;
                                     let shear_offset_c = r as f32 * layer.shear_x;
                                     

                                     
                                     let phys_x = rect.min.x + (l_start_c as f32 * char_width) + (c as f32 + shear_offset_c) * actual_char_width + layer.fine_offset_x * scale + zigzag_off_x;
                                     let phys_y = rect.min.y + (l_start_r as f32 * char_height) + (r as f32 * actual_char_height) + layer.fine_offset_y * scale + zigzag_off_y;

                                     let cell_rect = egui::Rect::from_min_size(egui::pos2(phys_x, phys_y), egui::vec2(char_width, char_height));
                                     if cell_bg != egui::Color32::TRANSPARENT {
                                         painter.rect_filled(cell_rect, 0.0, cell_bg);
                                     }

                                     
                                     struct DrawOp {
                                         z_index: i32,
                                         ch: char,
                                         fg: egui::Color32,
                                         px: f32,
                                         py: f32,
                                     }
                                     let mut draw_ops: Vec<DrawOp> = Vec::new();

                                     for (i, &draw_ch) in chars_to_draw.iter().enumerate() {
                                         if draw_ch == ' ' { continue; }
                                         let mut comp_dx = 0.0;
                                         let mut comp_dy = 0.0;
                                         if layer.border_composite && chars_to_draw.len() > 1 {
                                             comp_dx = i as f32 * layer.composite_spacing_x * scale;
                                             comp_dy = i as f32 * layer.composite_spacing_y * scale;
                                         }
                                         draw_ops.push(DrawOp {
                                             z_index: 0,
                                             ch: draw_ch,
                                             fg: cell_fg,
                                             px: phys_x + comp_dx,
                                             py: phys_y + comp_dy,
                                         });
                                     }

                                     if is_border {
                                         for eb in &layer.extra_borders {
                                             let mut active_eb_piece: Option<&BorderPiece> = None;
                                             let mut chars_to_draw_eb: Vec<char> = Vec::new();
                                             
                                             let get_chars_eb = |s: &str| -> Vec<char> {
                                                 if layer.border_composite && !s.is_empty() { s.chars().collect() }
                                                 else if !s.is_empty() { vec![s.chars().next().unwrap()] }
                                                 else { vec![' '] }
                                             };

                                             if is_top && is_left { active_eb_piece = Some(&eb.template.top_left); chars_to_draw_eb = get_chars_eb(&eb.template.top_left.pattern); }
                                             else if is_top && is_right { active_eb_piece = Some(&eb.template.top_right); chars_to_draw_eb = get_chars_eb(&eb.template.top_right.pattern); }
                                             else if is_bot && is_left { active_eb_piece = Some(&eb.template.bottom_left); chars_to_draw_eb = get_chars_eb(&eb.template.bottom_left.pattern); }
                                             else if is_bot && is_right { active_eb_piece = Some(&eb.template.bottom_right); chars_to_draw_eb = get_chars_eb(&eb.template.bottom_right.pattern); }
                                             else if is_top { 
                                                 active_eb_piece = Some(&eb.template.top_pattern);
                                                 if layer.border_composite && !eb.template.top_pattern.pattern.is_empty() { chars_to_draw_eb = eb.template.top_pattern.pattern.chars().collect(); }
                                                 else { chars_to_draw_eb = vec![safe_char(&eb.template.top_pattern.pattern.chars().collect(), c.saturating_sub(1))]; }
                                             }
                                             else if is_bot { 
                                                 active_eb_piece = Some(&eb.template.bottom_pattern);
                                                 if layer.border_composite && !eb.template.bottom_pattern.pattern.is_empty() { chars_to_draw_eb = eb.template.bottom_pattern.pattern.chars().collect(); }
                                                 else { chars_to_draw_eb = vec![safe_char(&eb.template.bottom_pattern.pattern.chars().collect(), c.saturating_sub(1))]; }
                                             }
                                             else if is_left { 
                                                 active_eb_piece = Some(&eb.template.left_pattern);
                                                 if layer.border_composite && !eb.template.left_pattern.pattern.is_empty() { chars_to_draw_eb = eb.template.left_pattern.pattern.chars().collect(); }
                                                 else { chars_to_draw_eb = vec![safe_char(&eb.template.left_pattern.pattern.chars().collect(), r.saturating_sub(1))]; }
                                             }
                                             else if is_right { 
                                                 active_eb_piece = Some(&eb.template.right_pattern);
                                                 if layer.border_composite && !eb.template.right_pattern.pattern.is_empty() { chars_to_draw_eb = eb.template.right_pattern.pattern.chars().collect(); }
                                                 else { chars_to_draw_eb = vec![safe_char(&eb.template.right_pattern.pattern.chars().collect(), r.saturating_sub(1))]; }
                                             }
                                             
                                             if let Some(p) = active_eb_piece {
                                                 let mut e_fg = cell_fg; // default from layer
                                                 if p.color_override { e_fg = egui::Color32::from_rgb(p.fg_color[0], p.fg_color[1], p.fg_color[2]); }
                                                 
                                                 for (i, &draw_ch) in chars_to_draw_eb.iter().enumerate() {
                                                     if draw_ch == ' ' { continue; }
                                                     let mut comp_dx = 0.0;
                                                     let mut comp_dy = 0.0;
                                                     if layer.border_composite && chars_to_draw_eb.len() > 1 {
                                                         comp_dx = i as f32 * layer.composite_spacing_x * scale;
                                                         comp_dy = i as f32 * layer.composite_spacing_y * scale;
                                                     }
                                                     
                                                     draw_ops.push(DrawOp {
                                                         z_index: eb.z_index,
                                                         ch: draw_ch,
                                                         fg: e_fg,
                                                         px: phys_x + comp_dx + (eb.global_offset_x + p.offset_x) * scale,
                                                         py: phys_y + comp_dy + (eb.global_offset_y + p.offset_y) * scale,
                                                     });
                                                 }
                                             }
                                         }
                                     }

                                     draw_ops.sort_by_key(|op| op.z_index);
                                     
                                     for op in draw_ops {
                                         if layer.drop_shadow {
                                             painter.text(egui::pos2(op.px + layer.shadow_offset_x*scale, op.py + layer.shadow_offset_y*scale), egui::Align2::LEFT_TOP, op.ch, layer_font.clone(), egui::Color32::from_rgba_unmultiplied(layer.shadow_color[0], layer.shadow_color[1], layer.shadow_color[2], (layer.alpha * 255.0) as u8));
                                         }
                                         painter.text(egui::pos2(op.px, op.py), egui::Align2::LEFT_TOP, op.ch, layer_font.clone(), op.fg);
                                     }

                                 }
                            }
                        }
                        LayerKind::Text => {
                            let content_to_use = if layer.repeat_content {
                                let mut repeated = String::new();
                                let chars_vec: Vec<char> = layer.content.chars().collect();
                                for _ in 0..l_h_chars {
                                    let mut line = String::new();
                                    for i in 0..l_w_chars {
                                        line.push(if chars_vec.is_empty() { ' ' } else { chars_vec[i % chars_vec.len()] });
                                    }
                                    repeated.push_str(&line);
                                    repeated.push('\n');
                                }
                                repeated
                            } else {
                                layer.content.clone()
                            };
                            let raw_lines: Vec<&str> = content_to_use.lines().collect();
                            let mut text_lines = Vec::new();
                            
                            if layer.wrap_text {
                                for line in raw_lines {
                                    let mut current = String::new();
                                    for ch in line.chars() {
                                        if current.chars().count() >= l_w_chars {
                                            text_lines.push(current.clone());
                                            current.clear();
                                        }
                                        current.push(ch);
                                    }
                                    if !current.is_empty() { text_lines.push(current); }
                                }
                            } else {
                                text_lines = raw_lines.iter().map(|s| s.to_string()).collect();
                            }
                            
                            let text_h = text_lines.len().max(1);

                            let total_chars: usize = text_lines.iter().map(|l| l.chars().count()).sum();
                            let reveal_count = if layer.animation == AnimationType::Typewriter {
                                (time * layer.anim_speed * 10.0) as usize % (total_chars + 10) // 10 chars pause
                            } else {
                                total_chars
                            };
                            let mut chars_drawn = 0;

                            for (line_idx, line) in text_lines.iter().enumerate() {
                                let line_w = line.chars().count();
                                
                                let txt_c = match layer.text_align {
                                    TextAlign::Left => 0,
                                    TextAlign::Center => (l_w_chars as isize / 2).saturating_sub(line_w as isize / 2),
                                    TextAlign::Right => (l_w_chars as isize).saturating_sub(line_w as isize),
                                };

                                let txt_r = match layer.text_valign {
                                    TextValign::Top => 0,
                                    TextValign::Middle => (l_h_chars as isize / 2).saturating_sub(text_h as isize / 2),
                                    TextValign::Bottom => (l_h_chars as isize).saturating_sub(text_h as isize),
                                };

                                let absolute_base_y = rect.min.y + (start_r + txt_r) as f32 * char_height + layer_off_y + layer.fine_offset_y * scale;
                                let absolute_y = absolute_base_y + (line_idx as f32 * layer.line_spacing * scale * layer.scale_y);

                                let absolute_base_x = rect.min.x + (start_c + txt_c) as f32 * char_width + layer_off_x + layer.fine_offset_x * scale;

                                for (char_idx, ch) in line.chars().enumerate() {
                                    if chars_drawn >= reveal_count { break; }
                                    chars_drawn += 1;
                                    
                                    let mut cell_fg = fg;
                                    if layer.use_gradient {
                                        let t = match layer.gradient_dir {
                                            GradientDirection::Horizontal => char_idx as f32 / line_w.max(1) as f32,
                                            GradientDirection::Vertical => line_idx as f32 / text_h.max(1) as f32,
                                            GradientDirection::Diagonal => (char_idx as f32 + line_idx as f32) / (line_w + text_h).max(1) as f32,
                                        };
                                        let lerp = |a: u8, b: u8, t: f32| -> u8 { (a as f32 + (b as f32 - a as f32) * t) as u8 };
                                        cell_fg = egui::Color32::from_rgba_unmultiplied(
                                            lerp(layer.fg_color[0], layer.gradient_target[0], t),
                                            lerp(layer.fg_color[1], layer.gradient_target[1], t),
                                            lerp(layer.fg_color[2], layer.gradient_target[2], t),
                                            (layer.alpha * 255.0) as u8
                                        );
                                    }

                                    let mut zigzag_off_x = 0.0;
                                    let mut zigzag_off_y = 0.0;
                                    if (line_idx + char_idx) % 2 != 0 {
                                        zigzag_off_x = layer.zigzag_x * scale;
                                        zigzag_off_y = layer.zigzag_y * scale;
                                    }

                                    let phys_x = absolute_base_x + (char_idx as f32 * layer.letter_spacing * scale * layer.scale_x) + zigzag_off_x;
                                    let phys_y = absolute_y + zigzag_off_y;
                                    
                                    let (rot_px, rot_py) = rotate_point(phys_x, phys_y, _center_x, _center_y, layer.text_rotation);
                                    let b_w = layer_font.size; // approximate

                                    if layer.drop_shadow && ch != ' ' {
                                        painter.text(egui::pos2(rot_px + layer.shadow_offset_x*scale, rot_py + layer.shadow_offset_y*scale), egui::Align2::LEFT_TOP, ch, layer_font.clone(), egui::Color32::from_rgba_unmultiplied(layer.shadow_color[0], layer.shadow_color[1], layer.shadow_color[2], (layer.alpha * 255.0) as u8));
                                    }
                                    if ch != ' ' {
                                        if layer.text_outline {
                                            let outline_c = egui::Color32::from_rgba_unmultiplied(layer.text_outline_color[0], layer.text_outline_color[1], layer.text_outline_color[2], (layer.alpha * 255.0) as u8);
                                            for dx in [-1.0, 0.0, 1.0_f32] {
                                                for dy in [-1.0, 0.0, 1.0_f32] {
                                                    if dx != 0.0 || dy != 0.0 {
                                                        painter.text(egui::pos2(rot_px + dx * 1.5 * scale, rot_py + dy * 1.5 * scale), egui::Align2::LEFT_TOP, ch, layer_font.clone(), outline_c);
                                                    }
                                                }
                                            }
                                        }
                                        painter.text(egui::pos2(rot_px, rot_py), egui::Align2::LEFT_TOP, ch, layer_font.clone(), cell_fg);
                                        
                                        if layer.is_bold {
                                            painter.text(egui::pos2(rot_px + 1.0, rot_py), egui::Align2::LEFT_TOP, ch, layer_font.clone(), cell_fg);
                                        }
                                        if layer.is_underline {
                                            painter.line_segment([egui::pos2(rot_px, rot_py + b_w), egui::pos2(rot_px + b_w * 0.7, rot_py + b_w)], egui::Stroke::new(1.0, cell_fg));
                                        }
                                        if layer.is_strikethrough {
                                            painter.line_segment([egui::pos2(rot_px, rot_py + b_w * 0.6), egui::pos2(rot_px + b_w * 0.7, rot_py + b_w * 0.6)], egui::Stroke::new(1.0, cell_fg));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            app.interaction_states = next_interaction_states;

            if let Some((s_idx, e_idx)) = clicked_idx {
                app.active_scene = s_idx;
                app.selected_index = Some(e_idx);
                app.last_selected = Some(e_idx);
                app.json_buffer = None;
                
                let action = app.scenes[s_idx].elements[e_idx].action_binding.clone();
                if !action.trim().is_empty() {
                    app.event_queue.push(crate::core::events::AxiomEvent::ActionTriggered(action));
                }
            }

            for ((s_idx, e_idx), delta) in drag_deltas {
                if let Some(el) = app.scenes[s_idx].elements.get_mut(e_idx) {
                    el.pos_x += (delta.x / render_w) * 100.0;
                    el.pos_y += (delta.y / render_h) * 100.0;
                }
            }
            for ((s_idx, e_idx), delta) in resize_deltas {
                if let Some(el) = app.scenes[s_idx].elements.get_mut(e_idx) {
                    el.width += (delta.x / render_w) * 100.0;
                    el.height += (delta.y / render_h) * 100.0;
                    el.width = el.width.max(1.0);
                    el.height = el.height.max(1.0);
                }
            }

            if app.is_playing {
                let mut move_dir = None;
                if ui.input(|i| i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::W)) { move_dir = Some((0.0, -1.0)); }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::S)) { move_dir = Some((0.0, 1.0)); }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::A)) { move_dir = Some((-1.0, 0.0)); }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::D)) { move_dir = Some((1.0, 0.0)); }

                if let Some((dx, dy)) = move_dir {
                    let focusable: Vec<usize> = app.scenes[app.active_scene].elements.iter().enumerate()
                        .filter(|(_, el)| !el.action_binding.trim().is_empty())
                        .map(|(i, _)| i)
                        .collect();

                    if !focusable.is_empty() {
                        if let Some(curr_idx) = app.focused_index {
                            if let Some(&(cx, cy, _, _, _, _, _, _)) = rect_cache.get(&(app.active_scene, curr_idx)) {
                                let mut best_idx = curr_idx;
                                let mut best_score = f32::MAX;

                                for &f_idx in &focusable {
                                    if f_idx == curr_idx { continue; }
                                    if let Some(&(fx, fy, _, _, _, _, _, _)) = rect_cache.get(&(app.active_scene, f_idx)) {
                                        let diff_x = fx - cx;
                                        let diff_y = fy - cy;
                                        let dot = diff_x * dx + diff_y * dy;
                                        if dot > 0.0 {
                                            let dist = (diff_x.powi(2) + diff_y.powi(2)).sqrt();
                                            let perp = (diff_x * dy - diff_y * dx).abs();
                                            let score = dist + perp * 2.0; 
                                            if score < best_score {
                                                best_score = score;
                                                best_idx = f_idx;
                                            }
                                        }
                                    }
                                }
                                app.focused_index = Some(best_idx);
                            }
                        } else {
                            app.focused_index = Some(focusable[0]);
                        }
                    }
                }

                if ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space)) {
                    if let Some(idx) = app.focused_index {
                        let action = app.scenes[app.active_scene].elements[idx].action_binding.clone();
                        if !action.trim().is_empty() {
                            app.event_queue.push(crate::core::events::AxiomEvent::ActionTriggered(action));
                        }
                    }
                }
            }
        });
}
