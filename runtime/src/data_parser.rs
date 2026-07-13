use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct RoomDimensions {
    pub width: u32,
    pub length: u32,
    pub height: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Lighting {
    pub ambient: f32,
    pub player_fov: f32,
    pub flashlight_range: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerStart {
    pub x: f32,
    pub y: f32,
    pub angle: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GameObject {
    #[serde(rename = "type")]
    pub object_type: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RoomData {
    pub room: RoomInfo,
    pub lighting: Lighting,
    pub player_start: PlayerStart,
    pub objects: Vec<GameObject>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RoomInfo {
    pub id: String,
    pub name: String,
    pub dimensions: RoomDimensions,
}

pub fn load_room_data<P: AsRef<Path>>(path: P) -> Result<RoomData, Box<dyn std::error::Error>> {
    let file_content = fs::read_to_string(path)?;
    let room_data: RoomData = serde_json::from_str(&file_content)?;
    Ok(room_data)
}