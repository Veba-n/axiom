use serde::{Deserialize, Serialize};
use crate::core::types::Anchor;
use crate::data::layer::{AxiomLayer, LayerKind};

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct UiElement {
    pub id: String,
    pub z_index: i32,
    pub anchor: Anchor,
    pub pos_x: f32, pub pos_y: f32,
    pub width: f32, pub height: f32,
    pub parent_id: Option<String>,
    pub children: Vec<String>,
    pub action_binding: String,
    pub layers: Vec<AxiomLayer>,
}

impl Default for UiElement {
    fn default() -> Self {
        let mut fill_layer = AxiomLayer::default();
        fill_layer.id = "Arka_Plan".into();
        fill_layer.kind = LayerKind::Fill;
        fill_layer.content = " ".into();
        fill_layer.fg_color = [0, 255, 150];
        fill_layer.bg_color = [20, 20, 25];
        fill_layer.z_index = -2;

        let mut border_layer = AxiomLayer::default();
        border_layer.id = "Kenarlik".into();
        border_layer.kind = LayerKind::Border;
        border_layer.fg_color = [0, 255, 150];
        border_layer.bg_color = [20, 20, 25];
        border_layer.z_index = -1;

        let mut text_layer = AxiomLayer::default();
        text_layer.id = "Metin".into();
        text_layer.kind = LayerKind::Text;
        text_layer.content = "AXIOM".into();
        text_layer.z_index = 0;

        Self {
            id: "Yeni_Obje".into(),
            z_index: 0,
            anchor: Anchor::Center,
            pos_x: 0.0, pos_y: 0.0,
            width: 25.0, height: 15.0,
            parent_id: None,
            children: vec![],
            action_binding: "".into(),
            layers: vec![fill_layer, border_layer, text_layer],
        }
    }
}
