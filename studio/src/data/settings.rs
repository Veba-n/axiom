use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ProjectSettings {
    pub resolution_w: f32,
    pub resolution_h: f32,
    pub grid_snap: f32,
    pub show_grid_lines: bool,
    pub canvas_bg_color: [u8; 3],
    pub default_font_family: String,
    pub default_font_size: f32,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            resolution_w: 1280.0,
            resolution_h: 720.0,
            grid_snap: 5.0,
            show_grid_lines: false,
            canvas_bg_color: [10, 10, 15],
            default_font_family: "Monospace".into(),
            default_font_size: 20.0,
        }
    }
}
