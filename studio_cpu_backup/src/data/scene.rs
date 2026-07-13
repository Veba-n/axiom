use crate::data::element::UiElement;

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct EditorScene {
    pub name: String,
    pub elements: Vec<UiElement>,
}
