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
pub fn sync_texture_cache(ctx: &egui::Context, cache: &mut TextureCache, textures: &[AxiomTexture]) {
    cache.sync(ctx, textures);
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
    cx_override: Option<u32>,
    cy_override: Option<u32>,
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
    } else if let (Some(cx_o), Some(cy_o)) = (cx_override, cy_override) {
        cx = cx_o;
        cy = cy_o;
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
    original_screen_poly: Option<&[egui::Pos2]>,
    triangles: &[usize],
    face_id: &str,
    part: &ObjectPart,
    cache: &TextureCache,
    shading: ShadingMode,
    wire_stroke: egui::Stroke,
    _camera: &ViewportCamera,
    show_uv_dots: bool,
    show_outer_shell: bool,
    texture_debug: bool,
    debug_depth_color: Option<f32>,
) {
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

    let mat_opt = resolve_face_material(part, face_id);
    let default_mat = FaceMaterial::default();
    let mat = mat_opt.unwrap_or(&default_mat);

    let base_fill = material_tint(mat);
    let (composed, handle) = if mat.texture_id.is_empty() {
        (None, None)
    } else {
        (cache.get(&mat.texture_id), cache.get_handle(&mat.texture_id))
    };

    let final_bg_color = if mat.use_custom_bg || composed.is_none() {
        mat.background_color
    } else {
        composed.unwrap().base_color
    };

    match shading {
        ShadingMode::Wireframe => {}
        ShadingMode::Solid => {
            // GPU Katı Çizim Hattı (solid_pipeline) zaten depth-test ile bu yüzeyleri doldurdu!
            // CPU üzerinde tekrar yarı saydam bir dolgu çizmek, depth buffer'ı bypass ederek X-Ray ghosting hatalarına yol açar.
            // Bu yüzden CPU üzerinde Solid dolgusu çizmeyi bıraktık.
        }
        ShadingMode::Textured => {
            if let (Some(comp), Some(tex_handle)) = (composed, handle) {
                let tw = comp.width.max(1);
                let th = comp.height.max(1);
                let uv_poly = original_screen_poly.unwrap_or(screen_poly);
                let v0 = if uv_poly.len() > 0 { uv_poly[0] } else { egui::Pos2::ZERO };
                let v1 = if uv_poly.len() > 1 { uv_poly[1] } else { v0 };
                let _v2 = if uv_poly.len() > 2 { uv_poly[2] } else { v0 };
                let v3 = if uv_poly.len() > 3 { uv_poly[3] } else { v0 };

                let (obj_scale_x, obj_scale_y) = if face_id.contains("Z") {
                    (part.scale[0], part.scale[1])
                } else if face_id.contains("X") {
                    (part.scale[2], part.scale[1])
                } else if face_id.contains("Y") {
                    (part.scale[0], part.scale[2])
                } else {
                    (1.0, 1.0)
                };

                let base_density = tw.max(th) as f32;
                let uv_mul = mat.uv_scale[0].max(mat.uv_scale[1]).max(0.01);
                let density = base_density * uv_mul;

                let face_w = if mat.auto_tile { obj_scale_x.abs() } else { 1.0 };
                let face_h = if mat.auto_tile { obj_scale_y.abs() } else { 1.0 };

                let mut mesh = egui::Mesh::with_texture(tex_handle.id());
                
                let uv_scale_x = density * face_w / tw as f32;
                let uv_scale_y = density * face_h / th as f32;

                // ColorImage'in kendi arka plan rengi olduğu için sadece beyazla çarpıyoruz.
                // Eğer opacity varsa, unmultiplied olarak geçiyoruz.
                let mut color = egui::Color32::from_rgba_unmultiplied(
                    255,
                    255,
                    255,
                    (255.0 * mat.opacity) as u8,
                );
                
                if let Some(depth_ratio) = debug_depth_color {
                    let v = (depth_ratio.clamp(0.0, 1.0) * 255.0) as u8;
                    color = egui::Color32::from_rgba_unmultiplied(
                        v, v, v,
                        (255.0 * mat.opacity) as u8,
                    );
                }


                // CSG kesilmeleri (clipping) için 4 köşe yerine orijinal face üzerinden UV buluyoruz
                let vec_x = v1 - v0;
                let vec_y = v3 - v0;
                let det = vec_x.x * vec_y.y - vec_x.y * vec_y.x;

                let start_idx = mesh.vertices.len() as u32;

                for p in screen_poly.iter() {
                    let mut u = 0.0;
                    let mut v = 0.0;
                    if det.abs() > 1e-5 {
                        let px = p.x - v0.x;
                        let py = p.y - v0.y;
                        u = (px * vec_y.y - py * vec_y.x) / det;
                        v = (py * vec_x.x - px * vec_x.y) / det;
                    }
                    
                    let tex_u = mat.uv_offset[0] + u * uv_scale_x;
                    let tex_v = mat.uv_offset[1] + v * uv_scale_y;

                    mesh.vertices.push(egui::epaint::Vertex {
                        pos: *p,
                        uv: egui::pos2(tex_u, tex_v),
                        color,
                    });
                }

                // Egui Mesh'e Proper Triangulation (Ear-Clipping) Sonucunu Ekle
                if triangles.len() % 3 == 0 {
                    for i in (0..triangles.len()).step_by(3) {
                        let i1 = triangles[i];
                        let i2 = triangles[i + 1];
                        let i3 = triangles[i + 2];
                        
                        // start_idx, Egui Mesh'in içindeki bu poligonun ilk vertex'idir
                        // csg.rs, poligon vertexlerine 0'dan başlayarak indeks verir.
                        mesh.add_triangle(
                            start_idx + i1 as u32,
                            start_idx + i2 as u32,
                            start_idx + i3 as u32
                        );
                    }
                }

                painter.add(egui::Shape::mesh(mesh));

                // Texture Debug (Hücre ızgarasından kaçtık, sadece kırmızı sınırları çiziyoruz)
                if texture_debug {
                    let _debug_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 0, 0, 150));
                    let mut edges = screen_poly.to_vec();
                    edges.push(edges[0]);
                    painter.add(egui::Shape::line(edges, egui::Stroke::new(2.0, egui::Color32::RED)));
                }
            } else {
                let fill = egui::Color32::from_rgba_unmultiplied(
                    base_fill.r(),
                    base_fill.g(),
                    base_fill.b(),
                    (255.0 * mat.opacity) as u8,
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