use crate::data::border::{BorderTemplate, ExtraBorder};
use crate::data::texture::{AxiomTexture, LayerGenMode, TextureLayer};

/// Hazır materyal şablonları — taş, tuğla, tahta vb.
pub fn preset_stone_brick(id: &str) -> AxiomTexture {
    AxiomTexture {
        id: id.into(),
        name: "Taş Tuğla".into(),
        resolution: [16, 16],
        base_color: [45, 42, 38],
        layers: vec![
            TextureLayer {
                name: "Tuğla Dolgu".into(),
                z_index: 0,
                gen_mode: LayerGenMode::Fill,
                pattern: "░▒".into(),
                fg_color: [90, 85, 78],
                pattern_spacing: 2,
                uv_scale: [4.0, 2.0],
                opacity: 0.85,
                height_val: 0.5,
                // Her tuğlanın merkezi hafifçe kabarık, harç aralığına
                // yakın kısımları çukur — gerçekçi kabartmalı tuğla hissi.
                height_function: crate::data::texture::HeightFunction::CellBulge,
                height_amplitude: 0.6,
                ..TextureLayer::default()
            },
            TextureLayer {
                name: "Tuğla Kenarları".into(),
                z_index: 10,
                gen_mode: LayerGenMode::DirectionalBorder,
                border: BorderTemplate::brick(),
                fg_color: [120, 110, 100],
                pattern_spacing: 2,
                height_val: 2.0,
                ..TextureLayer::default()
            },
            TextureLayer {
                name: "Derinlik Gölgesi".into(),
                z_index: 5,
                gen_mode: LayerGenMode::Noise,
                pattern: ".,".into(),
                noise_density: 0.15,
                fg_color: [30, 28, 25],
                blend_mode: crate::data::texture::BlendMode::Multiply,
                opacity: 0.4,
                height_val: 0.2,
                ..TextureLayer::default()
            },
        ],
    }
}

pub fn preset_rough_stone(id: &str) -> AxiomTexture {
    AxiomTexture {
        id: id.into(),
        name: "Ham Taş".into(),
        resolution: [12, 12],
        base_color: [55, 52, 48],
        layers: vec![
            TextureLayer {
                name: "Taş Yüzey".into(),
                z_index: 0,
                gen_mode: LayerGenMode::Noise,
                pattern: ".,-~:;=!*#".into(),
                noise_density: 0.7,
                fg_color: [110, 105, 95],
                uv_scale: [3.0, 3.0],
                height_val: 1.0,
                ..TextureLayer::default()
            },
            TextureLayer {
                name: "Taş Çerçeve".into(),
                z_index: 10,
                gen_mode: LayerGenMode::DirectionalBorder,
                border: BorderTemplate::stone(),
                fg_color: [70, 65, 60],
                height_val: 3.0,
                ..TextureLayer::default()
            },
        ],
    }
}

pub fn preset_wood_plank(id: &str) -> AxiomTexture {
    AxiomTexture {
        id: id.into(),
        name: "Tahta Plank".into(),
        resolution: [8, 16],
        base_color: [60, 40, 25],
        layers: vec![
            TextureLayer {
                name: "Tahta Dokusu".into(),
                z_index: 0,
                gen_mode: LayerGenMode::Solid,
                pattern: "|/|\\".into(),
                fg_color: [139, 90, 43],
                fg_gradient_end: Some([100, 65, 30]),
                uv_scale: [1.0, 4.0],
                height_val: 0.8,
                // Tahta damarı boyunca dalgalanan ince kabartma çizgileri —
                // "yaşlılık çizgileri" hissi. Desen tahta uzun eksenine
                // (Y) paralel olduğundan rotation 90° ile çizgileri damar
                // yönüne çeviriyoruz.
                height_function: crate::data::texture::HeightFunction::Wave,
                height_amplitude: 0.3,
                height_frequency: 2.5,
                rotation: 90.0,
                ..TextureLayer::default()
            },
            TextureLayer {
                name: "Plank Kenarları".into(),
                z_index: 5,
                gen_mode: LayerGenMode::DirectionalBorder,
                border: BorderTemplate::interwoven(),
                fg_color: [80, 50, 25],
                pattern_spacing: 4,
                height_val: 1.5,
                ..TextureLayer::default()
            },
        ],
    }
}

pub fn preset_brick_with_mortar(id: &str) -> AxiomTexture {
    let mut tex = preset_stone_brick(id);
    tex.name = "Tuğla + Harç".into();
    tex.layers.push(TextureLayer {
        name: "Harç Çizgisi (Ara Katman)".into(),
        z_index: 8,
        gen_mode: LayerGenMode::DirectionalBorder,
        border: BorderTemplate::empty(),
        extra_borders: vec![ExtraBorder {
            template: {
                let mut t = BorderTemplate::brick();
                t.top_pattern.pattern = "══".into();
                t.bottom_pattern.pattern = "══".into();
                t.left_pattern.pattern = "║".into();
                t.right_pattern.pattern = "║".into();
                t
            },
            global_offset_x: 0.0,
            global_offset_y: 0.0,
            z_index: 2,
        }],
        fg_color: [180, 175, 165],
        pattern_spacing: 2,
        opacity: 0.9,
        height_val: 0.3,
        ..TextureLayer::default()
    });
    tex
}

pub fn all_presets(next_id: usize) -> Vec<AxiomTexture> {
    let base = next_id;
    vec![
        preset_stone_brick(&format!("Tex_{}", base)),
        preset_rough_stone(&format!("Tex_{}", base + 1)),
        preset_wood_plank(&format!("Tex_{}", base + 2)),
        preset_brick_with_mortar(&format!("Tex_{}", base + 3)),
        preset_metal_plate(&format!("Tex_{}", base + 4)),
        preset_fabric(&format!("Tex_{}", base + 5)),
        preset_mosaic(&format!("Tex_{}", base + 6)),
        preset_decals(&format!("Tex_{}", base + 7)),
    ]
}

pub fn preset_metal_plate(id: &str) -> AxiomTexture {
    AxiomTexture {
        id: id.into(),
        name: "Metal Plaka".into(),
        resolution: [12, 12],
        base_color: [80, 85, 95],
        layers: vec![
            TextureLayer {
                name: "Metal Yüzey".into(),
                z_index: 0,
                gen_mode: LayerGenMode::Solid,
                pattern: "▓▒░".into(),
                fg_color: [140, 150, 160],
                uv_scale: [2.0, 2.0],
                opacity: 0.9,
                height_val: 0.3,
                ..TextureLayer::default()
            },
            TextureLayer {
                name: "Metal Kenarları".into(),
                z_index: 10,
                gen_mode: LayerGenMode::DirectionalBorder,
                border: BorderTemplate::round(),
                fg_color: [180, 190, 200],
                height_val: 1.5,
                ..TextureLayer::default()
            },
            TextureLayer {
                name: "Pas/Leke".into(),
                z_index: 5,
                gen_mode: LayerGenMode::Noise,
                pattern: ".,".into(),
                noise_density: 0.2,
                fg_color: [100, 80, 60],
                blend_mode: crate::data::texture::BlendMode::Multiply,
                opacity: 0.3,
                height_val: 0.1,
                ..TextureLayer::default()
            },
        ],
    }
}

pub fn preset_fabric(id: &str) -> AxiomTexture {
    AxiomTexture {
        id: id.into(),
        name: "Kumaş Dokusu".into(),
        resolution: [16, 16],
        base_color: [60, 50, 70],
        layers: vec![
            TextureLayer {
                name: "Kumaş Dokusu".into(),
                z_index: 0,
                gen_mode: LayerGenMode::Solid,
                pattern: "╱╲".into(),
                fg_color: [120, 100, 140],
                uv_scale: [3.0, 3.0],
                opacity: 0.85,
                height_val: 0.2,
                ..TextureLayer::default()
            },
            TextureLayer {
                name: "Dikiş Çizgileri".into(),
                z_index: 5,
                gen_mode: LayerGenMode::Fill,
                pattern: "·".into(),
                fg_color: [90, 80, 100],
                pattern_spacing: 4,
                opacity: 0.6,
                height_val: 0.1,
                ..TextureLayer::default()
            },
        ],
    }
}

pub fn preset_decals(id: &str) -> AxiomTexture {
    AxiomTexture {
        id: id.into(),
        name: "Dekaller".into(),
        resolution: [16, 16],
        base_color: [50, 50, 60],
        layers: vec![
            TextureLayer {
                name: "Temel Katman".into(),
                z_index: 0,
                gen_mode: LayerGenMode::Solid,
                pattern: "░▒▓".into(),
                fg_color: [120, 120, 130],
                uv_scale: [0.5, 0.5],
                opacity: 0.95,
                height_val: 0.3,
                manual_painting: true,
                ..TextureLayer::default()
            },
            TextureLayer {
                name: "Dekaller".into(),
                z_index: 10,
                gen_mode: LayerGenMode::Noise,
                pattern: "★✦◆◈".into(),
                noise_density: 0.15,
                fg_color: [200, 180, 150],
                uv_scale: [2.0, 2.0],
                opacity: 0.9,
                height_val: 0.5,
                manual_painting: true,
                ..TextureLayer::default()
            }
        ],
    }
}

pub fn preset_mosaic(id: &str) -> AxiomTexture {
    AxiomTexture {
        id: id.into(),
        name: "Mozaik Desen".into(),
        resolution: [8, 8],
        base_color: [40, 40, 50],
        layers: vec![
            TextureLayer {
                name: "Mozaik Karoları".into(),
                z_index: 0,
                gen_mode: LayerGenMode::Checker,
                pattern: "■□".into(),
                fg_color: [180, 140, 100],
                uv_scale: [2.0, 2.0],
                opacity: 0.9,
                height_val: 0.4,
                ..TextureLayer::default()
            },
            TextureLayer {
                name: "Mozaik Harcı".into(),
                z_index: 10,
                gen_mode: LayerGenMode::DirectionalBorder,
                border: BorderTemplate::stone(),
                fg_color: [120, 110, 100],
                pattern_spacing: 2,
                height_val: 0.2,
                ..TextureLayer::default()
            },
        ],
    }
}