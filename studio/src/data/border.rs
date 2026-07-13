use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct BorderPiece {
    pub pattern: String,
    pub color_override: bool,
    pub fg_color: [u8; 3],
    pub offset_x: f32,
    pub offset_y: f32,
    #[serde(skip)] pub is_editing: bool,
}
impl BorderPiece {
    pub fn new(pat: &str) -> Self {
        Self { pattern: pat.into(), color_override: false, fg_color: [255, 255, 255], offset_x: 0.0, offset_y: 0.0, is_editing: false }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct BorderTemplate {
    pub top_left: BorderPiece, pub top_right: BorderPiece,
    pub bottom_left: BorderPiece, pub bottom_right: BorderPiece,
    pub top_pattern: BorderPiece, pub bottom_pattern: BorderPiece,
    pub left_pattern: BorderPiece, pub right_pattern: BorderPiece,
}

impl BorderTemplate {
    pub fn empty() -> Self {
        Self {
            top_left: BorderPiece::new(""), top_right: BorderPiece::new(""),
            bottom_left: BorderPiece::new(""), bottom_right: BorderPiece::new(""),
            top_pattern: BorderPiece::new(""), bottom_pattern: BorderPiece::new(""),
            left_pattern: BorderPiece::new(""), right_pattern: BorderPiece::new(""),
        }
    }
    
    pub fn interwoven() -> Self {
        Self {
            top_left: BorderPiece::new("+"), top_right: BorderPiece::new("+"),
            bottom_left: BorderPiece::new("+"), bottom_right: BorderPiece::new("+"),
            top_pattern: BorderPiece::new("=-"), bottom_pattern: BorderPiece::new("=-"),
            left_pattern: BorderPiece::new(":."), right_pattern: BorderPiece::new(":."),
        }
    }
    
    pub fn round() -> Self {
        Self {
            top_left: BorderPiece::new("╭"), top_right: BorderPiece::new("╮"),
            bottom_left: BorderPiece::new("╰"), bottom_right: BorderPiece::new("╯"),
            top_pattern: BorderPiece::new("─"), bottom_pattern: BorderPiece::new("─"),
            left_pattern: BorderPiece::new("│"), right_pattern: BorderPiece::new("│"),
        }
    }
    
    pub fn solid() -> Self {
        Self {
            top_left: BorderPiece::new("█"), top_right: BorderPiece::new("█"),
            bottom_left: BorderPiece::new("█"), bottom_right: BorderPiece::new("█"),
            top_pattern: BorderPiece::new("█"), bottom_pattern: BorderPiece::new("█"),
            left_pattern: BorderPiece::new("█"), right_pattern: BorderPiece::new("█"),
        }
    }

    /// Taş tuğla / tuğla duvar kenar deseni
    pub fn brick() -> Self {
        Self {
            top_left: BorderPiece::new("┌"), top_right: BorderPiece::new("┐"),
            bottom_left: BorderPiece::new("└"), bottom_right: BorderPiece::new("┘"),
            top_pattern: BorderPiece::new("──"), bottom_pattern: BorderPiece::new("──"),
            left_pattern: BorderPiece::new("│"), right_pattern: BorderPiece::new("│"),
        }
    }

    /// Ham taş / kaba duvar
    pub fn stone() -> Self {
        Self {
            top_left: BorderPiece::new("▛"), top_right: BorderPiece::new("▜"),
            bottom_left: BorderPiece::new("▙"), bottom_right: BorderPiece::new("▟"),
            top_pattern: BorderPiece::new("▀▀"), bottom_pattern: BorderPiece::new("▄▄"),
            left_pattern: BorderPiece::new("█"), right_pattern: BorderPiece::new("█"),
        }
    }
}

impl Default for BorderTemplate {
    fn default() -> Self {
        Self {
            top_left: BorderPiece::new("╔"), top_right: BorderPiece::new("╗"),
            bottom_left: BorderPiece::new("╚"), bottom_right: BorderPiece::new("╝"),
            top_pattern: BorderPiece::new("═"), bottom_pattern: BorderPiece::new("═"),
            left_pattern: BorderPiece::new("║"), right_pattern: BorderPiece::new("║"),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct ExtraBorder {
    pub template: BorderTemplate,
    pub global_offset_x: f32,
    pub global_offset_y: f32,
    pub z_index: i32,
}
impl Default for ExtraBorder {
    fn default() -> Self {
        Self {
            template: BorderTemplate::empty(),
            global_offset_x: 0.0,
            global_offset_y: 0.0,
            z_index: 1,
        }
    }
}
