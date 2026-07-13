use crate::data::border::{BorderPiece, BorderTemplate, ExtraBorder};
use crate::data::texture::{
    AxiomTexture, BlendMode, HeightFunction, LayerGenMode, TextureLayer, TileWrapMode,
};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Tek hücrenin kompozisyon sonucu
#[derive(Clone, Debug, Default)]
pub struct TextureCell {
    pub ch: char,
    pub fg: [u8; 3],
    pub alpha: f32,
    pub height: f32,
    pub emission: f32,
    pub visible: bool,
}

/// Tam kompozisyon çıktısı — renk + height map
#[derive(Clone, Debug)]
pub struct ComposedTexture {
    pub width: u32,
    pub height: u32,
    pub base_color: [u8; 3],
    pub cells: Vec<Vec<TextureCell>>,
    pub height_map: Vec<Vec<f32>>,
    pub has_border: bool,
}

impl ComposedTexture {
    pub fn cell(&self, x: u32, y: u32) -> &TextureCell {
        &self.cells[y as usize][x as usize]
    }
}

/// Doku kompozisyon önbelleği (cache).
///
/// PERFORMANS NOTU: `compose()` katmanları, kenarlıkları ve blend modlarını
/// tarayıp her seferinde sıfırdan hesaplayan pahalı bir fonksiyondur. Eskiden
/// hem doku editöründe hem de obje viewport'unda bu fonksiyon HER FRAME,
/// içerik değişmemiş olsa bile, projedeki dokular için yeniden çağrılıyordu.
/// Bu, parça/doku sayısı arttıkça FPS'in çökmesinin başlıca sebebiydi.
///
/// Bu cache, her dokunun içerik özetini (hash) saklar ve sadece hash
/// değiştiğinde yeniden `compose()` çalıştırır; aksi halde anında önceki
/// sonucu döner.
#[derive(Default)]
pub struct TextureCache {
    entries: HashMap<String, (u64, ComposedTexture)>,
}

impl TextureCache {
    pub fn new() -> Self {
        Self { entries: HashMap::new() }
    }

    /// 3D dönüşümler için yeni fonksiyon
    pub fn apply_3d_transform(&mut self, texture_id: &str, _rotation: [f32; 3], scale: [f32; 3]) {
        if let Some((hash, tex)) = self.entries.get_mut(texture_id) {
            // Dokunun her hücresine dönüşüm uygula
            for y in 0..tex.height {
                for x in 0..tex.width {
                    let cell = &mut tex.cells[y as usize][x as usize];
                    // Yükseklik haritasını dönüşümlerle güncelle
                    cell.height *= scale[2]; // Z ekseni ölçekleme
                    // Görünürlük kontrolü
                    cell.visible = cell.visible && cell.height > 0.0;
                }
            }
            // Height map'i de güncelle
            for y in 0..tex.height {
                for x in 0..tex.width {
                    let cell = &tex.cells[y as usize][x as usize];
                    tex.height_map[y as usize][x as usize] = cell.height;
                }
            }
            // Hash'i güncelle
            *hash = hash.wrapping_add(1);
        }
    }

    /// Dokunun içerik hash'i. JSON serileştirme `compose()`'a göre
    /// kıyaslanamayacak kadar ucuzdur (hücre hücre tarama / kenarlık
    /// hesaplama yapmaz), bu yüzden "değişti mi?" kontrolü için güvenli ve
    /// pratik bir yöntemdir.
    fn content_hash(texture: &AxiomTexture) -> u64 {
        let bytes = serde_json::to_vec(texture).unwrap_or_default();
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        hasher.finish()
    }

    /// Tek bir dokuyu önbellekten döner; içerik değiştiyse önce yeniden
    /// hesaplar. Doku editöründeki canlı önizleme için kullanılır.
    pub fn get_or_compose(&mut self, texture: &AxiomTexture) -> &ComposedTexture {
        let h = Self::content_hash(texture);
        let needs_recompute = match self.entries.get(&texture.id) {
            Some((cached_hash, _)) => *cached_hash != h,
            None => true,
        };
        if needs_recompute {
            self.entries.insert(texture.id.clone(), (h, compose(texture)));
        }
        &self.entries.get(&texture.id).unwrap().1
    }

    /// Projedeki tüm dokuları, sadece değişenleri yeniden hesaplayarak
    /// günceller. Obje viewport'unda her frame çağrılması güvenlidir —
    /// değişmeyen dokular için sadece ucuz bir hash kontrolü yapılır.
    pub fn sync(&mut self, textures: &[AxiomTexture]) {
        for tex in textures {
            self.get_or_compose(tex);
        }
        // Artık projede olmayan dokuların kaydını temizle (bellek sızıntısını önler).
        let valid_ids: std::collections::HashSet<&str> =
            textures.iter().map(|t| t.id.as_str()).collect();
        self.entries.retain(|id, _| valid_ids.contains(id.as_str()));
    }

    /// Salt-okunur bakış: yeniden hesaplama yapmaz, sadece önbellekten okur.
    /// `sync()` çağrıldıktan sonra çizim aşamasında kullanılır.
    pub fn get(&self, texture_id: &str) -> Option<&ComposedTexture> {
        self.entries.get(texture_id).map(|(_, c)| c)
    }
}

pub fn compose(texture: &AxiomTexture) -> ComposedTexture {
    let w = texture.resolution[0].max(1);
    let h = texture.resolution[1].max(1);

    let mut cells: Vec<Vec<TextureCell>> = (0..h)
        .map(|_| {
            (0..w)
                .map(|_| TextureCell {
                    fg: texture.base_color,
                    alpha: 1.0,
                    visible: false,
                    ..Default::default()
                })
                .collect()
        })
        .collect();

    let mut height_map: Vec<Vec<f32>> = vec![vec![0.0; w as usize]; h as usize];

    let mut layers: Vec<&TextureLayer> = texture.layers.iter().filter(|l| l.is_visible).collect();
    layers.sort_by_key(|l| l.z_index);

    // Akıllı arkaplan entegrasyonu - boş hücreleri base_color ile doldur
    for y in 0..h {
        for x in 0..w {
            let dst = &mut cells[y as usize][x as usize];
            if !dst.visible {
                dst.fg = texture.base_color;
                dst.visible = true;
                dst.alpha = 1.0;
            }
        }
    }

    for layer in layers {
        let layer_grid = generate_layer(w, h, layer);
        for y in 0..h {
            for x in 0..w {
                let src = &layer_grid[y as usize][x as usize];
                if !src.visible || src.ch == ' ' {
                    continue;
                }
                let dst = &mut cells[y as usize][x as usize];
                blend_cell(dst, src, layer);
                height_map[y as usize][x as usize] =
                    height_map[y as usize][x as usize].max(src.height * layer.opacity);
            }
        }
    }

    ComposedTexture {
        width: w,
        height: h,
        base_color: texture.base_color,
        cells,
        height_map,
        has_border: texture.layers.iter().any(|l| {
            matches!(
                l.gen_mode,
                LayerGenMode::Border | LayerGenMode::DirectionalBorder
            )
        }),
    }
}

fn generate_layer(w: u32, h: u32, layer: &TextureLayer) -> Vec<Vec<TextureCell>> {
    let mut grid = vec![vec![TextureCell::default(); w as usize]; h as usize];

    for y in 0..h {
        for x in 0..w {
            let (tx, ty) = tile_coords(x, y, w, h, layer);
            let y_pct = if h > 1 {
                y as f32 / (h - 1) as f32
            } else {
                0.0
            };

            let mut cell = TextureCell::default();
            let chars: Vec<char> = layer.pattern.chars().collect();

            match layer.gen_mode {
                LayerGenMode::Solid => {
                    if chars.is_empty() {
                        continue;
                    }
                    cell.ch = chars[((tx + ty) as usize) % chars.len()];
                    cell.visible = true;
                }
                LayerGenMode::Noise => {
                    if chars.is_empty() {
                        continue;
                    }
                    let seed =
                        (tx.wrapping_mul(13579) ^ ty.wrapping_mul(97531)) as f32 / u32::MAX as f32;
                    if seed <= layer.noise_density {
                        cell.ch = chars[(seed * 100.0) as usize % chars.len()];
                        cell.visible = true;
                    }
                }
                LayerGenMode::Checker => {
                    if chars.is_empty() {
                        continue;
                    }
                    if (tx + ty) % 2 == 0 {
                        cell.ch = chars[0];
                        cell.visible = true;
                    }
                }
                LayerGenMode::Border => {
                    if chars.is_empty() {
                        continue;
                    }
                    let on_edge = x == 0 || x == w - 1 || y == 0 || y == h - 1;
                    if on_edge {
                        cell.ch = chars[((tx + ty) as usize) % chars.len()];
                        cell.visible = true;
                    }
                }
                LayerGenMode::Fill => {
                    if chars.is_empty() {
                        continue;
                    }
                    if layer.pattern_spacing > 1 && (tx + ty) % layer.pattern_spacing as u32 != 0 {
                        continue;
                    }
                    cell.ch = chars[((tx + ty * w) as usize) % chars.len()];
                    cell.visible = true;
                }
                LayerGenMode::DirectionalBorder => {
                    let ops = border_cell_ops(
                        tx,
                        ty,
                        w,
                        h,
                        &layer.border,
                        &layer.extra_borders,
                        layer,
                    );
                    if let Some(op) = ops.first() {
                        cell.ch = op.ch;
                        cell.fg = op.fg;
                        cell.visible = op.ch != ' ';
                    }
                }
            }

            if !cell.visible {
                continue;
            }

            cell.fg = layer_color(layer, y_pct);
            cell.alpha = layer.opacity;
            // height_val: taban (düz) yükseklik. height_modulation: seçilen
            // height_function'a göre hücre hücre eklenen kabartma/relief.
            // amplitude 0 ise modulation her zaman 0 -> eski davranışla birebir aynı.
            cell.height = (layer.height_val + height_modulation(layer, tx, ty)).max(0.0);
            cell.emission = layer.emission_val;
            grid[y as usize][x as usize] = cell;
        }
    }

    grid
}

struct BorderDrawOp {
    ch: char,
    fg: [u8; 3],
    z_index: i32,
}

fn border_cell_ops(
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    border: &BorderTemplate,
    extra_borders: &[ExtraBorder],
    layer: &TextureLayer,
) -> Vec<BorderDrawOp> {
    let w_us = w as usize;
    let h_us = h as usize;
    let x_us = x as usize;
    let y_us = y as usize;

    let is_top = y_us == 0;
    let is_bot = y_us == h_us - 1;
    let is_left = x_us == 0;
    let is_right = x_us == w_us - 1;
    let is_corner = (is_top || is_bot) && (is_left || is_right);

    if !(is_top || is_bot || is_left || is_right) {
        return vec![];
    }

    if !is_corner && layer.pattern_spacing > 1 {
        if (is_top || is_bot) && x_us % layer.pattern_spacing != 0 {
            return vec![];
        }
        if (is_left || is_right) && y_us % layer.pattern_spacing != 0 {
            return vec![];
        }
    }

    let base_fg = layer.fg_color;
    let mut ops = collect_border_ops(
        x_us,
        y_us,
        w_us,
        h_us,
        border,
        layer.border_composite,
        base_fg,
        0,
        0.0,
        0.0,
    );

    for eb in extra_borders {
        ops.extend(collect_border_ops(
            x_us,
            y_us,
            w_us,
            h_us,
            &eb.template,
            layer.border_composite,
            base_fg,
            eb.z_index,
            eb.global_offset_x,
            eb.global_offset_y,
        ));
    }

    ops.sort_by_key(|op| op.z_index);
    ops
}

fn collect_border_ops(
    c: usize,
    r: usize,
    w: usize,
    h: usize,
    template: &BorderTemplate,
    composite: bool,
    default_fg: [u8; 3],
    z_index: i32,
    _global_off_x: f32,
    _global_off_y: f32,
) -> Vec<BorderDrawOp> {
    let is_top = r == 0;
    let is_bot = r == h - 1;
    let is_left = c == 0;
    let is_right = c == w - 1;

    let safe_char = |pat: &[char], idx: usize| -> char {
        if pat.is_empty() {
            ' '
        } else {
            pat[idx % pat.len()]
        }
    };

    let get_chars = |s: &str| -> Vec<char> {
        if composite && !s.is_empty() {
            s.chars().collect()
        } else if !s.is_empty() {
            vec![s.chars().next().unwrap()]
        } else {
            vec![' ']
        }
    };

    let (piece, chars): (Option<&BorderPiece>, Vec<char>) =
        if is_top && is_left {
            (
                Some(&template.top_left),
                get_chars(&template.top_left.pattern),
            )
        } else if is_top && is_right {
            (
                Some(&template.top_right),
                get_chars(&template.top_right.pattern),
            )
        } else if is_bot && is_left {
            (
                Some(&template.bottom_left),
                get_chars(&template.bottom_left.pattern),
            )
        } else if is_bot && is_right {
            (
                Some(&template.bottom_right),
                get_chars(&template.bottom_right.pattern),
            )
        } else if is_top {
            (
                Some(&template.top_pattern),
                if composite && !template.top_pattern.pattern.is_empty() {
                    template.top_pattern.pattern.chars().collect()
                } else {
                    vec![safe_char(
                        &template.top_pattern.pattern.chars().collect::<Vec<_>>(),
                        c.saturating_sub(1),
                    )]
                },
            )
        } else if is_bot {
            (
                Some(&template.bottom_pattern),
                if composite && !template.bottom_pattern.pattern.is_empty() {
                    template.bottom_pattern.pattern.chars().collect()
                } else {
                    vec![safe_char(
                        &template.bottom_pattern.pattern.chars().collect::<Vec<_>>(),
                        c.saturating_sub(1),
                    )]
                },
            )
        } else if is_left {
            (
                Some(&template.left_pattern),
                if composite && !template.left_pattern.pattern.is_empty() {
                    template.left_pattern.pattern.chars().collect()
                } else {
                    vec![safe_char(
                        &template.left_pattern.pattern.chars().collect::<Vec<_>>(),
                        r.saturating_sub(1),
                    )]
                },
            )
        } else if is_right {
            (
                Some(&template.right_pattern),
                if composite && !template.right_pattern.pattern.is_empty() {
                    template.right_pattern.pattern.chars().collect()
                } else {
                    vec![safe_char(
                        &template.right_pattern.pattern.chars().collect::<Vec<_>>(),
                        r.saturating_sub(1),
                    )]
                },
            )
        } else {
            (None, vec![])
        };

    let mut fg = default_fg;
    if let Some(p) = piece {
        if p.color_override {
            fg = p.fg_color;
        }
    }

    chars
        .into_iter()
        .filter(|&ch| ch != ' ')
        .map(|ch| BorderDrawOp {
            ch,
            fg,
            z_index,
        })
        .collect()
}

fn tile_coords(x: u32, y: u32, w: u32, h: u32, layer: &TextureLayer) -> (u32, u32) {
    let sx = layer.uv_scale[0].max(0.01);
    let sy = layer.uv_scale[1].max(0.01);
    let ox = layer.uv_offset[0];
    let oy = layer.uv_offset[1];

    // Desen Açısı (rotation): deseni katmanın merkezi etrafında döndürür —
    // örn. çapraz/diyagonal tuğla dizilimi, açılı tahta damarı gibi
    // efektler için. 0 derece iken davranış tamamen eskisiyle aynıdır.
    // NOT: Bu, DirectionalBorder modunda kenar tespitini (üst/alt/sol/sağ)
    // bozabileceğinden, en iyi sonucu Solid/Noise/Checker/Fill katmanlarında verir.
    let (rx, ry) = if layer.rotation.abs() > f32::EPSILON {
        let cx = w as f32 / 2.0;
        let cy = h as f32 / 2.0;
        let rad = layer.rotation.to_radians();
        let dx = x as f32 - cx;
        let dy = y as f32 - cy;
        (
            dx * rad.cos() - dy * rad.sin() + cx,
            dx * rad.sin() + dy * rad.cos() + cy,
        )
    } else {
        (x as f32, y as f32)
    };

    // uv_scale: büyük değer = sık tekrar, küçük değer = geniş tile
    let fx = rx * sx + ox;
    let fy = ry * sy + oy;

    let tx = wrap_coord(fx, w, layer.tile_wrap);
    let ty = wrap_coord(fy, h, layer.tile_wrap);
    (tx, ty)
}

/// Bir katmanın `height_function` ayarına göre hücre başına ek kabartma
/// (relief) miktarını hesaplar. `height_amplitude` 0 ise her zaman 0 döner
/// — yani eski projeler/dokular davranış değişikliği olmadan çalışmaya devam eder.
fn height_modulation(layer: &TextureLayer, tx: u32, ty: u32) -> f32 {
    if layer.height_amplitude.abs() < f32::EPSILON {
        return 0.0;
    }
    let freq = layer.height_frequency.max(0.01);
    match layer.height_function {
        HeightFunction::Flat => 0.0,
        HeightFunction::Noise => {
            // LayerGenMode::Noise ile aynı hash tabanlı pseudo-rastgele
            // üretici — kaba taş/sıva gibi düzensiz kabartma yüzeyleri için.
            let freq_bits = (freq * 1000.0) as u32;
            let seed = (tx.wrapping_mul(13579).wrapping_add(freq_bits)
                ^ ty.wrapping_mul(97531).wrapping_add(freq_bits))
                as f32
                / u32::MAX as f32;
            (seed - 0.5) * 2.0 * layer.height_amplitude
        }
        HeightFunction::Wave => {
            // Periyodik dalga — tahta damarı / yaşlılık çizgileri gibi
            // tekrarlayan kabartma çizgileri için.
            let phase = (tx as f32 + ty as f32) * freq * 0.3;
            phase.sin() * layer.height_amplitude
        }
        HeightFunction::CellBulge => {
            // pattern_spacing aralığının merkezine yakın hücreler kabarık,
            // sınırlarına (örn. harç çizgisi) yakın hücreler çukur —
            // kabartmalı tuğla/karo efekti.
            let spacing = layer.pattern_spacing.max(1) as f32;
            let mx = (tx as f32 % spacing) / spacing;
            let my = (ty as f32 % spacing) / spacing;
            let dx = (mx - 0.5).abs() * 2.0;
            let dy = (my - 0.5).abs() * 2.0;
            let edge_dist = dx.max(dy); // 0 = merkez (kabarık), 1 = kenar (çukur)
            (1.0 - edge_dist) * layer.height_amplitude
        }
    }
}

fn wrap_coord(v: f32, max: u32, mode: TileWrapMode) -> u32 {
    let m = max.max(1) as f32;
    match mode {
        TileWrapMode::Repeat => {
            let r = v.rem_euclid(m);
            r as u32
        }
        TileWrapMode::Mirror => {
            let period = m * 2.0;
            let r = v.rem_euclid(period);
            if r < m {
                r as u32
            } else {
                (period - r - 1.0) as u32
            }
        }
        TileWrapMode::Clamp => v.clamp(0.0, m - 1.0) as u32,
    }
}

fn layer_color(layer: &TextureLayer, y_pct: f32) -> [u8; 3] {
    let base = layer.fg_color;
    if let Some(grad) = layer.fg_gradient_end {
        [
            (base[0] as f32 * (1.0 - y_pct) + grad[0] as f32 * y_pct) as u8,
            (base[1] as f32 * (1.0 - y_pct) + grad[1] as f32 * y_pct) as u8,
            (base[2] as f32 * (1.0 - y_pct) + grad[2] as f32 * y_pct) as u8,
        ]
    } else {
        base
    }
}

fn blend_cell(dst: &mut TextureCell, src: &TextureCell, layer: &TextureLayer) {
    let alpha = src.alpha;
    let src_rgb = src.fg;
    let dst_rgb = if dst.visible { dst.fg } else { dst.fg };

    let blended = match layer.blend_mode {
        BlendMode::Normal => lerp_rgb(dst_rgb, src_rgb, alpha),
        BlendMode::Additive => [
            (dst_rgb[0].saturating_add((src_rgb[0] as f32 * alpha) as u8)).min(255),
            (dst_rgb[1].saturating_add((src_rgb[1] as f32 * alpha) as u8)).min(255),
            (dst_rgb[2].saturating_add((src_rgb[2] as f32 * alpha) as u8)).min(255),
        ],
        BlendMode::Multiply => [
            ((dst_rgb[0] as f32 * src_rgb[0] as f32 / 255.0 * alpha
                + dst_rgb[0] as f32 * (1.0 - alpha)) as u8),
            ((dst_rgb[1] as f32 * src_rgb[1] as f32 / 255.0 * alpha
                + dst_rgb[1] as f32 * (1.0 - alpha)) as u8),
            ((dst_rgb[2] as f32 * src_rgb[2] as f32 / 255.0 * alpha
                + dst_rgb[2] as f32 * (1.0 - alpha)) as u8),
        ],
        BlendMode::Subtractive => [
            dst_rgb[0].saturating_sub((src_rgb[0] as f32 * alpha) as u8),
            dst_rgb[1].saturating_sub((src_rgb[1] as f32 * alpha) as u8),
            dst_rgb[2].saturating_sub((src_rgb[2] as f32 * alpha) as u8),
        ],
        BlendMode::Overlay => {
            let overlay = |b: u8, s: u8| -> u8 {
                let bf = b as f32 / 255.0;
                let sf = s as f32 / 255.0;
                let r = if bf < 0.5 {
                    2.0 * bf * sf
                } else {
                    1.0 - 2.0 * (1.0 - bf) * (1.0 - sf)
                };
                (r * 255.0) as u8
            };
            [
                lerp_u8(dst_rgb[0], overlay(dst_rgb[0], src_rgb[0]), alpha),
                lerp_u8(dst_rgb[1], overlay(dst_rgb[1], src_rgb[1]), alpha),
                lerp_u8(dst_rgb[2], overlay(dst_rgb[2], src_rgb[2]), alpha),
            ]
        }
    };

    dst.ch = src.ch;
    dst.fg = blended;
    dst.alpha = alpha.max(if dst.visible { dst.alpha } else { 0.0 });
    dst.visible = true;
    dst.height = src.height;
    dst.emission = src.emission;
}

fn lerp_rgb(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    [
        lerp_u8(a[0], b[0], t),
        lerp_u8(a[1], b[1], t),
        lerp_u8(a[2], b[2], t),
    ]
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t.clamp(0.0, 1.0)) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::border::BorderTemplate;

    #[test]
    fn compose_checker_has_cells() {
        let mut tex = AxiomTexture::default();
        tex.resolution = [4, 4];
        tex.layers[0].gen_mode = LayerGenMode::Checker;
        tex.layers[0].pattern = "#".into();
        let out = compose(&tex);
        assert!(out.cells.iter().flatten().any(|c| c.visible));
    }

    #[test]
    fn directional_border_draws_edges() {
        let mut tex = AxiomTexture::default();
        tex.resolution = [6, 6];
        tex.layers[0].gen_mode = LayerGenMode::DirectionalBorder;
        tex.layers[0].border = BorderTemplate::brick();
        let out = compose(&tex);
        assert!(out.cell(0, 0).visible);
        assert!(!out.cell(2, 2).visible);
    }
}