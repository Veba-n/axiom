use serde::{Deserialize, Serialize};
use crate::data::object::AiBehavior;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct ObjectInstance {
    pub instance_id: String,
    pub object_id: String, // Referans alınan GameObject ID'si
    pub name_override: String,
    
    // Dünyadaki Yeri
    pub world_pos: [f32; 3],
    pub world_rot: [f32; 3],
    pub world_scale: [f32; 3],
    
    // Objenin parametrelerine özel (instance-specific) override değerler
    pub param_overrides: std::collections::HashMap<String, f32>,
    
    // Core Object Özellikleri Ezme (Override)
    pub health_override: Option<f32>,
    pub mass_override: Option<f32>,
    pub ai_behavior_override: Option<AiBehavior>,
    pub light_intensity_override: Option<f32>,
    pub light_color_override: Option<[u8; 3]>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct GameLevel {
    pub id: String,
    pub name: String,
    
    // Seviyeye yerleştirilen objeler
    pub instances: Vec<ObjectInstance>,
    
    // Seviye (Oda) Özellikleri
    pub ambient_light: [u8; 3],
    pub gravity: [f32; 3],
    pub skybox_texture: String,
}

impl Default for GameLevel {
    fn default() -> Self {
        Self {
            id: "Level_1".into(),
            name: "Yeni Harita".into(),
            instances: vec![],
            ambient_light: [30, 30, 40],
            gravity: [0.0, -9.81, 0.0],
            skybox_texture: "".into(),
        }
    }
}
