use eframe::egui;
use crate::data::object::{FaceMaterial, ObjectPart};
use crate::data::texture::AxiomTexture;
use crate::render::texture_composer::{ComposedTexture, TextureCache};

/// Yüzey slot adları — küp/prisma için standart isimler
pub const FACE_SLOTS: &[&str] = &["All", "+X", "-X", "+Y", "-Y", "+Z", "-Z"];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShadingMode {
    Wireframe,
    Solid,
    Textured,
}

pub fn parse_shading(model: &str) -> ShadingMode {
    match model {
        "Wireframe" => ShadingMode::Wireframe,
        "Solid" | "Flat" | "Smooth" => ShadingMode::Solid,
        "Textured" | "Lit" => ShadingMode::Textured,
        _ => ShadingMode::Textured,
    }
}

/// PERFORMANS: Eskiden bu fonksiyon HER FRAME projedeki TÜM dokuları
/// `compose()` ile sıfırdan hesaplıyordu (her katman, her kenarlık, her
/// blend işlemi yeniden taranıyordu) — parça/doku sayısı arttıkça FPS'in
/// asıl çöküş sebebi buydu.
///
/// Artık kalıcı bir `TextureCache` (AxiomStudio::texture_cache) üzerinde
/// çalışıyor: sadece içeriği gerçekten değişen dokular yeniden hesaplanıyor,
/// değişmeyenler için ucuz bir hash kontrolü yeterli. `app` her frame
/// `sync_texture_cache(&mut app.texture_cache, &app.textures)` çağırmalı,
/// ardından çizim sırasında `&app.texture_cache` salt-okunur olarak
/// `draw_face_quad`'a verilmeli.
pub fn sync_texture_cache(cache: &mut TextureCache, textures: &[AxiomTexture]) {
    cache.sync(textures);
}

pub fn resolve_face_material<'a>(part: &'a ObjectPart, face: &str) -> Option<&'a FaceMaterial> {
    if let Some(m) = part.faces.get(face) {
        return Some(m);
    }
    if face != "All" {
        if let Some(m) = part.faces.get("All") {
            return Some(m);
        }
    }
    part.faces.values().next()
}

pub fn material_tint(mat: &FaceMaterial) -> egui::Color32 {
    egui::Color32::from_rgb(mat.tint[0], mat.tint[1], mat.tint[2])
}

pub fn sample_cell(
    composed: &ComposedTexture,
    tx: u32,
    ty: u32,
    grid_cols: u32,
    grid_rows: u32,
    uv_scale: [f32; 2],
    uv_offset: [f32; 2],
    _manual_paint: Option<[u8; 3]>, // Elle boyama rengi
) -> Option<(char, [u8; 3], f32, f32)> {
    let w = composed.width.max(1);
    let h = composed.height.max(1);
    
    let cx;
    let cy;

    if composed.has_border {
        // 9-Slice Border Preservation Logic (Akıllı Çerçeve Koruma)
        // Sol kenar -> 0. indekse, Sağ kenar -> Son indekse çivilenir. 
        // Ara değerler Tile (Döşeme) edilir.
        cx = if tx == 0 {
            0
        } else if tx >= grid_cols.saturating_sub(1) {
            w - 1
        } else {
            let inner_w = w.saturating_sub(2).max(1);
            let inner_tx = tx.saturating_sub(1);
            let scaled_u = (inner_tx as f32 / inner_w as f32) * uv_scale[0] + uv_offset[0];
            1 + (scaled_u * inner_w as f32).rem_euclid(inner_w as f32) as u32
        };

        // Aynı işlemi dikey (Y) eksen için yap.
        cy = if ty == 0 {
            0
        } else if ty >= grid_rows.saturating_sub(1) {
            h - 1
        } else {
            let inner_h = h.saturating_sub(2).max(1);
            let inner_ty = ty.saturating_sub(1);
            let scaled_v = (inner_ty as f32 / inner_h as f32) * uv_scale[1] + uv_offset[1];
            1 + (scaled_v * inner_h as f32).rem_euclid(inner_h as f32) as u32
        };
    } else {
        // Normal (Düz) Tiling Mantığı (Çerçeve olmayan dokular için)
        let u = tx as f32 / grid_cols as f32;
        let v = ty as f32 / grid_rows as f32;
        let scaled_u = u * uv_scale[0] + uv_offset[0];
        let scaled_v = v * uv_scale[1] + uv_offset[1];
        cx = (scaled_u * w as f32).rem_euclid(w as f32) as u32;
        cy = (scaled_v * h as f32).rem_euclid(h as f32) as u32;
    }

    
    let cell = composed.cell(cx.min(w - 1), cy.min(h - 1));
    if cell.visible && cell.ch != ' ' {
        Some((cell.ch, cell.fg, cell.alpha, cell.height))
    } else {
        None
    }
}

fn tint_rgb(base: [u8; 3], tint: [u8; 3]) -> [u8; 3] {
    [
        (base[0] as u16 * tint[0] as u16 / 255) as u8,
        (base[1] as u16 * tint[1] as u16 / 255) as u8,
        (base[2] as u16 * tint[2] as u16 / 255) as u8,
    ]
}

/// Dört köşeli yüzeye doku veya düz renk uygular (viewport içinde, ayrı pencere yok).
pub struct ViewportCamera {
    pub position: [f32; 3],
    pub rotation: [f32; 3], // X,Y,Z rotation angles
    pub zoom: f32,
    pub parallel_projection: bool,
}

impl ViewportCamera {
    pub fn apply_constraints(&mut self) {
        // Lock X rotation to exactly 10 degrees as required with no drift
        self.rotation[0] = 10.0f32.to_radians();
        
        // Keep Y and Z rotations at 0 with no drift
        self.rotation[1] = 0.0;
        self.rotation[2] = 0.0;
        
        // Maintain exactly 0.2 unit gap from surfaces with no floating point errors
        self.position[2] = (self.position[2] * 10.0).round().max(2.0) / 10.0;
        
        // Validate zoom to prevent invalid values
        self.zoom = self.zoom.clamp(0.1, 10.0);
    }
    
    /// Optimized version that checks if constraints are already met
    pub fn apply_constraints_if_needed(&mut self) {
        if self.rotation[0] != 10.0f32.to_radians() 
            || self.rotation[1] != 0.0 
            || self.rotation[2] != 0.0 
            || (self.position[2] * 10.0).round() < 2.0 {
            self.apply_constraints();
        }
    }
}

impl Default for ViewportCamera {
    fn default() -> Self {
        let mut camera = Self {
            position: [0.0, 0.0, 5.0],
            rotation: [10.0, 0.0, 0.0], // 10 degree X rotation as required
            zoom: 1.0,
            parallel_projection: true, // Default to parallel projection
        };
        camera.apply_constraints();
        camera
    }
}


pub fn draw_face_quad(
    painter: &egui::Painter,
    screen_poly: &[egui::Pos2],
    face_id: &str,
    part: &ObjectPart,
    cache: &TextureCache,
    shading: ShadingMode,
    wire_stroke: egui::Stroke,
    _camera: &ViewportCamera,
    show_uv_dots: bool,
    show_outer_shell: bool,
) {
    let is_point_in_polygon = |p: egui::Pos2, poly: &[egui::Pos2]| -> bool {
        if poly.len() < 3 { return false; }
        let mut inside = false;
        let mut j = poly.len() - 1;
        for i in 0..poly.len() {
            if (poly[i].y > p.y) != (poly[j].y > p.y) &&
                p.x < (poly[j].x - poly[i].x) * (p.y - poly[i].y) / (poly[j].y - poly[i].y) + poly[i].x
            {
                inside = !inside;
            }
            j = i;
        }
        inside
    };
    
    // Outer Shell (CSG Hata Ayıklama) Modu
    if show_outer_shell {
        if screen_poly.len() >= 3 {
            let mut hash = 0u32;
            for b in part.id.bytes() {
                hash = hash.wrapping_mul(31).wrapping_add(b as u32);
            }
            for b in face_id.bytes() {
                hash = hash.wrapping_mul(31).wrapping_add(b as u32);
            }
            // Use distinct prime multipliers for RGB channels for better color distribution
            let r = (hash.wrapping_mul(17) % 200 + 55) as u8;
            let g = (hash.wrapping_mul(37) % 200 + 55) as u8;
            let b = (hash.wrapping_mul(71) % 200 + 55) as u8;
            let color = egui::Color32::from_rgb(r, g, b);
            painter.add(egui::Shape::convex_polygon(screen_poly.to_vec(), color, egui::Stroke::NONE));
        }
        return; // Doku ve Wireframe çizimlerini atla
    }

    let mat = resolve_face_material(part, face_id);
    let default_mat = FaceMaterial::default();
    let mat = mat.unwrap_or(&default_mat);

    let base_fill = material_tint(mat);
    let composed = if mat.texture_id.is_empty() {
        None
    } else {
        cache.get(&mat.texture_id)
    };

    let final_bg_color = if mat.use_custom_bg || composed.is_none() {
        mat.background_color
    } else {
        composed.unwrap().base_color
    };

    match shading {
        ShadingMode::Wireframe => {}
        ShadingMode::Solid => {
            let bg = egui::Color32::from_rgb(final_bg_color[0], final_bg_color[1], final_bg_color[2]);
            let fill = if composed.is_some() {
                egui::Color32::from_rgba_premultiplied(
                    base_fill.r(),
                    base_fill.g(),
                    base_fill.b(),
                    (210.0 * mat.opacity) as u8,
                )
            } else {
                egui::Color32::from_rgba_premultiplied(
                    base_fill.r(),
                    base_fill.g(),
                    base_fill.b(),
                    (200.0 * mat.opacity) as u8,
                )
            };
            if screen_poly.len() >= 3 {
                // To support > 4 corners for solid CSG shape
                let mut mesh = egui::Mesh::default();
                let start_idx = mesh.vertices.len() as u32;
                for p in screen_poly {
                    mesh.vertices.push(egui::epaint::Vertex { pos: *p, uv: egui::Pos2::ZERO, color: bg });
                }
                for i in 1..(screen_poly.len() - 1) {
                    mesh.add_triangle(start_idx, start_idx + i as u32, start_idx + i as u32 + 1);
                }
                painter.add(egui::Shape::mesh(mesh));
                
                let mut mesh_fill = egui::Mesh::default();
                let start_idx_f = mesh_fill.vertices.len() as u32;
                for p in screen_poly {
                    mesh_fill.vertices.push(egui::epaint::Vertex { pos: *p, uv: egui::Pos2::ZERO, color: fill });
                }
                for i in 1..(screen_poly.len() - 1) {
                    mesh_fill.add_triangle(start_idx_f, start_idx_f + i as u32, start_idx_f + i as u32 + 1);
                }
                painter.add(egui::Shape::mesh(mesh_fill));
            }
        }
        ShadingMode::Textured => {
            let bg = egui::Color32::from_rgba_premultiplied(
                final_bg_color[0],
                final_bg_color[1],
                final_bg_color[2],
                (255.0 * mat.opacity) as u8,
            );
            if screen_poly.len() >= 3 {
                let mut mesh_bg = egui::Mesh::default();
                let start_idx_bg = mesh_bg.vertices.len() as u32;
                for p in screen_poly {
                    mesh_bg.vertices.push(egui::epaint::Vertex { pos: *p, uv: egui::Pos2::ZERO, color: bg });
                }
                for i in 1..(screen_poly.len() - 1) {
                    mesh_bg.add_triangle(start_idx_bg, start_idx_bg + i as u32, start_idx_bg + i as u32 + 1);
                }
                painter.add(egui::Shape::mesh(mesh_bg));
            }

            if let Some(comp) = composed {
                let tw = comp.width.max(1);
                let th = comp.height.max(1);
                let v0 = if screen_poly.len() > 0 { screen_poly[0] } else { egui::Pos2::ZERO };
                let v1 = if screen_poly.len() > 1 { screen_poly[1] } else { v0 };
                let v2 = if screen_poly.len() > 2 { screen_poly[2] } else { v0 };
                let v3 = if screen_poly.len() > 3 { screen_poly[3] } else { v0 };

                // Yüzey boyutlarını hesapla
                let face_width = (v1 - v0).length();
                let face_height = (v3 - v0).length();

                // TASARIM DÜZELTMESİ: Eskiden ızgara yoğunluğu "ekrandaki piksel 
                // boyutuna" (TARGET_CELL_PX) göre belirleniyordu. Bu durum, 
                // Texture Editor'de tasarlanan dokunun Obje Editöründe kameraya 
                // yakınlaşıp uzaklaştıkça rastgele harf sayılarına bölünmesine 
                // ve desenin tamamen bozulmasına yol açıyordu.
                //
                // Artık ızgara yoğunluğu doğrudan Texture Editor'deki gibi
                // Dokunun Çözünürlüğü (width/height) * Doku Tekrarı (uv_scale) 
                // formülüyle hesaplanıyor. Böylece bir duvara atanan doku, 
                // kameradan bağımsız olarak her zaman doğru sayıda harfle (tile) çizilir!
                
                // 3D Objenin Parça Boyutuna Göre Auto-Tile
                let (obj_scale_x, obj_scale_y) = if face_id.contains("Z") {
                    (part.scale[0], part.scale[1])
                } else if face_id.contains("X") {
                    (part.scale[2], part.scale[1])
                } else if face_id.contains("Y") {
                    (part.scale[0], part.scale[2])
                } else {
                    (1.0, 1.0)
                };

                let auto_scale_x = if mat.auto_tile { obj_scale_x.abs() } else { 1.0 };
                let auto_scale_y = if mat.auto_tile { obj_scale_y.abs() } else { 1.0 };
                
                // Güvenlik: Maksimum 128x128 ızgaraya (16,384 hücre) sınırla ki oyun çökmesin.
                let grid_cols = ((tw as f32 * mat.uv_scale[0] * auto_scale_x).max(1.0)) as u32;
                let grid_cols = grid_cols.min(128);
                
                let grid_rows = ((th as f32 * mat.uv_scale[1] * auto_scale_y).max(1.0)) as u32;
                let grid_rows = grid_rows.min(128);

                let mut mesh = egui::Mesh::with_texture(egui::TextureId::Managed(0));

                for ty in 0..grid_rows {
                    for tx in 0..grid_cols {
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

                        // Karakterin (Hücrenin) merkez noktası poligon dışındaysa komple atla!
                        // Bu sayede hem çok ciddi performans kazanırız, hem de Mesh Index Buffer bozulmaz.
                        let center_pos = p_tl.lerp(p_br, 0.5);
                        if !is_point_in_polygon(center_pos, &screen_poly) {
                            continue;
                        }

                        if let Some((ch, rgb, alpha, cell_height)) = sample_cell(comp, tx, ty, grid_cols, grid_rows, mat.uv_scale, mat.uv_offset, None) {
                            if ch == ' ' { continue; } // Boş karakterleri atla

                            let rgb = tint_rgb(rgb, mat.tint);
                            // Yüzeyin on-screen boyutlarına göre karakter büyüklüğü hesapla (Maksimum 64px)
                            let size = (face_width / grid_cols as f32).min(face_height / grid_rows as f32).clamp(2.0, 64.0);
                            let font = egui::FontId::new(size, egui::FontFamily::Monospace);
                            
                            let final_alpha = alpha * mat.opacity;
                            let mut col = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                            if final_alpha < 1.0 { col = col.linear_multiply(final_alpha); }

                            let galley = painter.ctx().fonts(|f| f.layout_no_wrap(ch.to_string(), font, col));
                            let cell_size = galley.rect.size();
                            let cell_min = galley.rect.min;

                            // Karakter yüksekliği için parallax offset (Hafif bir yükselti)
                            let height_offset = cell_height * 0.5;

                            let shape = egui::Shape::galley(egui::Pos2::ZERO, galley, col);

                            // Egui'nin gizli motorunu zorla çalıştır, bize kusursuz bir 2D Mesh versin!
                            let primitives = painter.ctx().tessellate(vec![egui::epaint::ClippedShape {
                                clip_rect: egui::Rect::EVERYTHING,
                                shape,
                            }], 1.0);

                            for primitive in primitives {
                                if let egui::epaint::Primitive::Mesh(cell_mesh) = primitive.primitive {
                                    mesh.texture_id = cell_mesh.texture_id;
                                    let start_idx = mesh.vertices.len() as u32;

                                    for mut vertex in cell_mesh.vertices {
                                        // Karakteri hücrenin merkezine hizalayarak oranla (Fontun en/boy oranı korunur, esneme yapmaz)
                                        let fx = (vertex.pos.x - cell_min.x + (size - cell_size.x) / 2.0) / size;
                                        let fy = (vertex.pos.y - cell_min.y + (size - cell_size.y) / 2.0) / size;

                                        // Gerçek 3D küp yüzeyinde bu % oranının denk geldiği koordinatı bul (Bilinear Shear)
                                        let top = p_tl.lerp(p_tr, fx);
                                        let bot = p_bl.lerp(p_br, fx);
                                        let mut final_pos = top.lerp(bot, fy);

                                        // 3D çıkıntı/girinti efekti - height map'e göre offset
                                        final_pos.x += height_offset;
                                        final_pos.y += height_offset;

                                        vertex.pos = final_pos;
                                        mesh.vertices.push(vertex);
                                    }

                                    mesh.indices.extend(cell_mesh.indices.into_iter().map(|i| i + start_idx));
                                }
                            }
                        }
                    }
                }
                
                // Tüm eğik karakterleri gerçek bir 3D Mesh olarak tek hamlede çiz!
                painter.add(egui::Shape::mesh(mesh));
            } else {
                let fill = egui::Color32::from_rgba_premultiplied(
                    base_fill.r(),
                    base_fill.g(),
                    base_fill.b(),
                    (200.0 * mat.opacity) as u8,
                );
                painter.add(egui::Shape::convex_polygon(
                    screen_poly.to_vec(),
                    fill,
                    egui::Stroke::NONE,
                ));
                if !mat.texture_id.is_empty() {
                    let center = egui::pos2(
                        (screen_poly[0].x + screen_poly[2].x) * 0.5,
                        (screen_poly[0].y + screen_poly[2].y) * 0.5,
                    );
                    painter.text(
                        center,
                        egui::Align2::CENTER_CENTER,
                        "?",
                        egui::FontId::proportional(12.0),
                        egui::Color32::YELLOW,
                    );
                }
            }
        }
    }

    if shading == ShadingMode::Wireframe || shading == ShadingMode::Solid || shading == ShadingMode::Textured {
        if screen_poly.len() >= 3 {
            let mut edge_points = screen_poly.to_vec();
            edge_points.push(screen_poly[0]);
            painter.add(egui::Shape::line(edge_points, wire_stroke));
        }

        if show_uv_dots {
            for p in screen_poly.iter() {
                // UV veya köşe noktalarını yeşil (CSG) olarak göster
                painter.circle_filled(*p, 4.0, egui::Color32::GREEN);
                
                // Metin çizimi FPS'i çok düşürdüğü için, sadece gerçekten ihtiyaç varsa
                // burayı kullan. Geçici olarak metni kaldırdık (veya istersen çok küçük çizebiliriz).
                // painter.text(
                //     *p, 
                //     egui::Align2::LEFT_TOP, 
                //     format!("{}", i), 
                //     egui::FontId::proportional(12.0), 
                //     egui::Color32::WHITE
                // );
            }
        }
    }
}

pub fn draw_wire_edges(painter: &egui::Painter, edges: &[(egui::Pos2, egui::Pos2)], stroke: egui::Stroke) {
    for (a, b) in edges {
        painter.line_segment([*a, *b], stroke);
    }
}