use serde::{Deserialize, Serialize};
use crate::core::types::{GradientDirection, TextAlign, TextValign, AnimationType};
use crate::data::border::{BorderTemplate, ExtraBorder};

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum LayerKind {
    Fill,
    Border,
    Text,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct LayerStateOverride {
    pub enabled: bool,
    pub fg_color: [u8; 3],
    pub bg_color: [u8; 3],
}

impl Default for LayerStateOverride {
    fn default() -> Self {
        Self {
            enabled: false,
            fg_color: [255, 255, 255],
            bg_color: [50, 50, 50],
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct AxiomLayer {
    pub id: String,
    pub kind: LayerKind,
    pub enabled: bool,
    pub z_index: i32,

    pub offset_x: f32,
    pub offset_y: f32,
    pub width: f32,
    pub height: f32,
    
    pub fine_offset_x: f32, 
    pub fine_offset_y: f32, 
    pub scale_x: f32,       
    pub scale_y: f32,       
    pub pattern_spacing: usize, 
    pub font_size: f32,
    pub font_family: String,
    pub font_scale: f32, 
    pub zigzag_x: f32, 
    pub zigzag_y: f32, 
    
    pub shear_x: f32, 
    
    pub content: String,
    pub repeat_content: bool, 
    
    pub alpha: f32, 
    pub fg_color: [u8; 3],
    pub bg_color: [u8; 3],
    pub bg_alpha: f32, 

    pub use_gradient: bool,
    pub gradient_target: [u8; 3],
    pub gradient_dir: GradientDirection,
    
    pub border: BorderTemplate,
    pub extra_borders: Vec<ExtraBorder>,
    pub padding_x: f32,
    pub padding_y: f32,
    pub border_composite: bool, 
    pub composite_spacing_x: f32,
    pub composite_spacing_y: f32,
    
    pub text_align: TextAlign,
    pub text_valign: TextValign,
    pub letter_spacing: f32,
    pub line_spacing: f32,
    pub wrap_text: bool,
    pub text_outline: bool, 
    pub text_outline_color: [u8; 3],
    
    pub text_rotation: f32,
    pub is_bold: bool,
    pub is_italic: bool,
    pub is_underline: bool,
    pub is_strikethrough: bool,
    
    pub drop_shadow: bool,
    pub shadow_color: [u8; 3],
    pub shadow_offset_x: f32,
    pub shadow_offset_y: f32,
    pub animation: AnimationType,
    pub anim_speed: f32,
    pub anim_amplitude: f32,
    
    pub hover_state: LayerStateOverride,
    pub pressed_state: LayerStateOverride,
}

impl Default for AxiomLayer {
    fn default() -> Self {
        Self {
            id: "Yeni_Katman".into(),
            kind: LayerKind::Text,
            enabled: true,
            z_index: 0,
            offset_x: 0.0, offset_y: 0.0,
            width: 100.0, height: 100.0,
            fine_offset_x: 0.0, fine_offset_y: 0.0,
            scale_x: 1.0, scale_y: 1.0,
            pattern_spacing: 1,
            font_size: 20.0,
            font_family: "Monospace".into(),
            font_scale: 1.0, zigzag_x: 0.0, zigzag_y: 0.0,
            shear_x: 0.0,
            content: "TEXT".into(),
            repeat_content: false,
            alpha: 1.0,
            bg_alpha: 1.0,
            fg_color: [255, 255, 255], bg_color: [0, 0, 0],
            use_gradient: false, gradient_target: [0, 0, 0], gradient_dir: GradientDirection::Horizontal,
            border: BorderTemplate::default(),
            extra_borders: vec![], padding_x: 0.0, padding_y: 0.0, border_composite: false,
            composite_spacing_x: 5.0,
            composite_spacing_y: 5.0,
            text_align: TextAlign::Center, text_valign: TextValign::Middle,
            letter_spacing: 12.0, line_spacing: 20.0, wrap_text: false,
            text_outline: false, text_outline_color: [0, 0, 0],
            text_rotation: 0.0, is_bold: false, is_italic: false, is_underline: false, is_strikethrough: false,
            drop_shadow: false, shadow_color: [0, 0, 0], shadow_offset_x: 3.0, shadow_offset_y: 3.0,
            animation: AnimationType::None, anim_speed: 1.0, anim_amplitude: 10.0,
            hover_state: LayerStateOverride::default(),
            pressed_state: LayerStateOverride::default(),
        }
    }
}
