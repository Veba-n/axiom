use serde::{Deserialize, Serialize};
use crate::data::border::{BorderTemplate, ExtraBorder};

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug, Default)]
pub enum BlendMode {
    #[default]
    Normal,
    Additive,
    Multiply,
    Subtractive,
    Overlay,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum LayerGenMode {
    /// Karakter setini tüm ızgarada tekrarlar
    Solid,
    /// Karakter setini rastgele dağıtır
    Noise,
    /// Dama tahtası deseni
    Checker,
    /// Dış çerçeve (basit)
    Border,
    /// İç dolgu — desen tüm yüzeye yayılır, tuğla aralığı destekler
    Fill,
    /// Yönlü kenarlar: üst, alt, sol, sağ, köşeler ayrı desenler
    DirectionalBorder,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum TileWrapMode {
    /// Desen hücre hücre tekrarlanır
    Repeat,
    /// Ayna yansıması ile tekrar
    Mirror,
    /// Kenarda kesilir, tekrar yok
    Clamp,
}

impl Default for TileWrapMode {
    fn default() -> Self {
        Self::Repeat
    }
}

/// Bir katmanın yüksekliğinin (height map) hücre hücre nasıl değişeceğini
/// belirler. `height_val` taban (düz) yüksekliği verir, bu fonksiyon ise
/// `height_amplitude` ile ölçeklenmiş bir kabartma/relief deseni ekler.
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum HeightFunction {
    /// Eski davranış: tüm katman boyunca sabit (düz) yükseklik.
    Flat,
    /// Pseudo-rastgele kabartma — kaba taş, sıva gibi düzensiz yüzeyler için.
    Noise,
    /// Periyodik dalga — tahta damarı / yaşlılık çizgileri, dokuma çizgileri
    /// gibi tekrarlayan kabartma çizgileri için. `height_frequency` dalganın
    /// ne kadar sık tekrar edeceğini belirler.
    Wave,
    /// `pattern_spacing` aralığının merkezine yakın hücreler kabarık, aralık
    /// sınırlarına (örn. harç çizgisi) yakın hücreler çukur olur —
    /// kabartmalı tuğla/karo efekti için idealdir.
    CellBulge,
}

impl Default for HeightFunction {
    fn default() -> Self {
        Self::Flat
    }
}

fn default_height_frequency() -> f32 {
    1.0
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct TextureLayer {
    #[serde(default = "default_layer_name")]
    pub name: String,
    #[serde(default = "default_true")]
    pub is_visible: bool,
    #[serde(default)]
    pub z_index: i32,
    #[serde(default)]
    pub blend_mode: BlendMode,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub manual_painting: bool, // Elle boyama özelliği
    #[serde(default)]
    pub pattern_lock: bool,    // Pattern kilitleme
    #[serde(default = "default_rotation_3d")]
    pub rotation_3d: [f32; 3], // X,Y,Z dönüş açıları
    #[serde(default = "default_scale_3d")]
    pub scale_3d: [f32; 3],    // X,Y,Z ölçek faktörleri

    pub gen_mode: LayerGenMode,
    /// ASCII karakter veya karakter seti (Fill/Solid/Noise için)
    pub pattern: String,
    pub noise_density: f32,

    pub font_family: String,
    pub font_size: f32,
    pub rotation: f32,

    /// Yüzey yüksekliği — height map'e katkı (taban / düz değer)
    pub height_val: f32,
    pub emission_val: f32,

    /// Kabartma (relief) fonksiyonu — height_val'in üzerine hücre hücre
    /// değişen bir yükseklik deseni ekler (örn. kabartmalı tuğla, tahta
    /// damarı). Varsayılan: Flat (eski davranış, etkisiz).
    #[serde(default)]
    pub height_function: HeightFunction,
    /// Kabartma fonksiyonunun genliği — ne kadar yukarı/aşağı oynayacağı.
    /// 0.0 ise kabartma fonksiyonu hiç etkisizdir (geriye dönük uyumlu).
    #[serde(default)]
    pub height_amplitude: f32,
    /// Kabartma fonksiyonunun tekrar sıklığı (Wave/Noise modunda kaç
    /// dalga/dalgalanma sığacağını etkiler).
    #[serde(default = "default_height_frequency")]
    pub height_frequency: f32,

    pub fg_color: [u8; 3],
    pub fg_gradient_end: Option<[u8; 3]>,
    #[serde(default = "default_bg_color")]
    pub bg_color: [u8; 3],
    #[serde(default)]
    pub bg_alpha: f32,

    /// Tekrar aralığı (1 = her hücre, 2 = tuğla aralığı gibi)
    pub uv_scale: [f32; 2],
    pub uv_offset: [f32; 2],
    #[serde(default)]
    pub tile_wrap: TileWrapMode,

    /// Yönlü kenar şablonu (DirectionalBorder modu)
    #[serde(default)]
    pub border: BorderTemplate,
    /// Katmanlar arası ek kenar katmanları
    #[serde(default)]
    pub extra_borders: Vec<ExtraBorder>,
    /// Tuğla/taş aralığı (1 = kapalı, 2+ = boşluk)
    #[serde(default = "default_pattern_spacing")]
    pub pattern_spacing: usize,
    #[serde(default)]
    pub border_composite: bool,
    #[serde(default = "default_composite_spacing")]
    pub composite_spacing_x: f32,
    #[serde(default = "default_composite_spacing")]
    pub composite_spacing_y: f32,
}

fn default_layer_name() -> String {
    "Yeni Katman".into()
}
fn default_true() -> bool {
    true
}
fn default_opacity() -> f32 {
    1.0
}
fn default_rotation_3d() -> [f32; 3] {
    [0.0, 0.0, 0.0]
}
fn default_scale_3d() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}
fn default_pattern_spacing() -> usize {
    1
}
fn default_composite_spacing() -> f32 {
    5.0
}
fn default_bg_color() -> [u8; 3] {
    [0, 0, 0]
}

impl Default for TextureLayer {
    fn default() -> Self {
        Self {
            manual_painting: false,
            pattern_lock: false,
            rotation_3d: [0.0, 0.0, 0.0],
            scale_3d: [1.0, 1.0, 1.0],
            name: "Yeni Katman".into(),
            is_visible: true,
            z_index: 0,
            blend_mode: BlendMode::Normal,
            opacity: 1.0,

            gen_mode: LayerGenMode::Solid,
            pattern: "▓".into(),
            noise_density: 0.5,

            font_family: "Monospace".into(),
            font_size: 16.0,
            rotation: 0.0,

            height_val: 1.5,
            emission_val: 0.0,
            height_function: HeightFunction::Flat,
            height_amplitude: 0.0,
            height_frequency: 1.0,

            fg_color: [200, 200, 200],
            fg_gradient_end: None,
            bg_color: [0, 0, 0],
            bg_alpha: 0.0,

            uv_scale: [1.0, 1.0],
            uv_offset: [0.0, 0.0],
            tile_wrap: TileWrapMode::Repeat,

            border: BorderTemplate::default(),
            extra_borders: vec![],
            pattern_spacing: 1,
            border_composite: false,
            composite_spacing_x: 5.0,
            composite_spacing_y: 5.0,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct AxiomTexture {
    pub id: String,
    pub name: String,
    pub resolution: [u32; 2],
    pub layers: Vec<TextureLayer>,
    pub base_color: [u8; 3],
}

impl Default for AxiomTexture {
    fn default() -> Self {
        Self {
            id: "Tex_1".into(),
            name: "Gelişmiş Materyal".into(),
            resolution: [8, 8],
            layers: vec![TextureLayer::default()],
            base_color: [20, 20, 25],
        }
    }
}