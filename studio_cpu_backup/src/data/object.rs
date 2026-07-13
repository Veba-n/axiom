use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum PrimitiveShape {
    Cube, Sphere, Pyramid, Cylinder, HalfCylinder, TriangularPrism, PentagonPrism, HexagonPrism, Cone, Torus, CustomMesh, EmptyGroup,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum BooleanOp { Add, Subtract, Intersect, }

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum ModifierType {
    Shear([f32; 3]), Bend(f32), Taper(f32), Noise([f32; 2]), // scale, intensity
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct FaceMaterial {
    pub texture_id: String, pub tint: [u8; 3], pub uv_scale: [f32; 2], pub uv_offset: [f32; 2],
    #[serde(default = "default_bg_color")]
    pub background_color: [u8; 3],
    #[serde(default)]
    pub use_custom_bg: bool,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default = "default_auto_tile")]
    pub auto_tile: bool,
}

fn default_auto_tile() -> bool {
    true
}

fn default_bg_color() -> [u8; 3] {
    [50, 50, 60]
}

fn default_opacity() -> f32 {
    1.0
}
impl Default for FaceMaterial {
    fn default() -> Self {
        Self {
            texture_id: "".into(),
            tint: [200, 200, 210],
            uv_scale: [1.0, 1.0],
            uv_offset: [0.0, 0.0],
            background_color: [50, 50, 60],
            use_custom_bg: false,
            opacity: 1.0,
            auto_tile: true,
        }
    }
}

/// Yeni mesh parçası için varsayılan yüzey ve görünüm ayarları
pub fn default_mesh_part(id: &str, name: &str, shape: PrimitiveShape) -> ObjectPart {
    let mut faces = HashMap::new();
    faces.insert("All".into(), FaceMaterial::default());
    ObjectPart {
        id: id.into(),
        name: name.into(),
        local_parameters: HashMap::new(),
        shape,
        boolean_op: BooleanOp::Add,
        csg_target_id: None,
        bone_id: None,
        parent_part_id: None,
        pos: [0.0, 0.0, 0.0],
        scale: [1.0, 1.0, 1.0],
        rot: [0.0, 0.0, 0.0],
        pos_expr: ["".into(), "".into(), "".into()],
        scale_expr: ["".into(), "".into(), "".into()],
        rot_expr: ["".into(), "".into(), "".into()],
        array_count_expr: "".into(),
        array_offset_expr: ["".into(), "".into(), "".into()],
        modifiers: vec![],
        faces,
        is_visible: true,
        pivot_mode: PivotMode::Center,
        pivot_offset: [0.0, 0.0, 0.0],
        shading_model: "Textured".into(),
        mirror_x: false,
        mirror_y: false,
        mirror_z: false,
        collider_type: ColliderType::None,
        lod_hide_distance: 0.0,
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum PivotMode {
    Center,
    CustomOffset([f32; 3]),
    EdgeMinX, EdgeMaxX,
    EdgeMinY, EdgeMaxY,
    EdgeMinZ, EdgeMaxZ,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum ColliderType { None, Box, Sphere, Capsule, Mesh }

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct ObjectSocket {
    pub id: String, pub name: String, pub bone_id: Option<String>,
    pub local_pos: [f32; 3], pub local_rot: [f32; 3], pub local_scale: [f32; 3],
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct ParticleEmitter {
    pub id: String, pub name: String, pub parent_part_id: Option<String>,
    pub local_pos: [f32; 3], pub emit_rate: f32, pub particle_color: [u8; 3],
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct ObjectPart {
    pub id: String, // Grup ve hiyerarşi için Unique ID
    pub name: String,
    pub local_parameters: std::collections::HashMap<String, f32>, // Gurup / Parça bazlı değişkenler
    pub shape: PrimitiveShape,
    pub boolean_op: BooleanOp,
    pub csg_target_id: Option<String>, // Sadece belirli bir hedefe (veya gruba) etki etmesi için
    pub bone_id: Option<String>,
    pub parent_part_id: Option<String>, // Başka bir parçaya bağlanma (Gruplama/Hiyerarşi)
    
    pub pos: [f32; 3],
    pub scale: [f32; 3],
    pub rot: [f32; 3],
    
    // Parametrik Modelleme (Matematiksel Fonksiyonlar)
    // Boş değilse ("" değilse), `pos`/`scale`/`rot` değerlerini ezer. Örn: "width / 2"
    pub pos_expr: [String; 3],
    pub scale_expr: [String; 3],
    pub rot_expr: [String; 3],
    
    // Array Modifier (Tekrarlı Çoğaltma - Pattern)
    pub array_count_expr: String, // Kaç kere tekrar edeceği (Örn: "5" veya "length / 10")
    pub array_offset_expr: [String; 3], // Her kopyada ne kadar kayacağı
    
    pub modifiers: Vec<ModifierType>,
    pub faces: HashMap<String, FaceMaterial>,
    pub is_visible: bool,
    pub pivot_mode: PivotMode, pub pivot_offset: [f32; 3],
    pub shading_model: String,
    
    // QoL ve Modelleme Kolaylaştırıcıları
    pub mirror_x: bool,
    pub mirror_y: bool,
    pub mirror_z: bool,
    
    // Yüksek Seviye Oyun Motoru Özellikleri
    pub collider_type: ColliderType,
    pub lod_hide_distance: f32, // Mesafe bundan büyükse parçayı çizme
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum AiBehavior { None, Passive, Hostile, Fleeing, Patrol, }

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Bone {
    pub id: String, pub parent_id: Option<String>, pub local_pos: [f32; 3], pub local_rot: [f32; 3],
    // IK Limits / Constraints
    pub lock_x: bool, pub lock_y: bool, pub lock_z: bool,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Keyframe {
    pub time_ms: f32, pub pos: [f32; 3], pub rot: [f32; 3], pub scale: [f32; 3], pub easing: String,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct AnimationSequence {
    pub name: String, pub is_looping: bool, pub tracks: HashMap<String, Vec<Keyframe>>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct GameObject {
    pub id: String,
    pub name: String,
    pub tags: String,
    
    // Parametrik Değişkenler (Kullanıcının belirlediği Özel Ayarlar: "width" -> 10.0, "height" -> 50.0)
    pub parameters: HashMap<String, f32>,
    pub sockets: Vec<ObjectSocket>,
    pub emitters: Vec<ParticleEmitter>,
    
    pub parts: Vec<ObjectPart>,
    pub bones: Vec<Bone>,
    pub animations: Vec<AnimationSequence>,
    
    pub health: f32, pub bounding_box: [f32; 3], pub is_solid: bool,
    pub mass: f32, pub friction: f32, pub restitution: f32, pub gravity_scale: f32,
    
    pub cast_shadows: bool, pub light_emission_color: [u8; 3], pub light_radius: f32, pub light_intensity: f32,
    
    pub ai_behavior: AiBehavior, pub aggro_radius: f32, pub custom_scripts: String,
    
    // Obje Genel (Global) Dönüşüm Kontrolleri (Toplu Düzenleme)
    pub global_pos: [f32; 3],
    pub global_scale: [f32; 3],
    pub global_rot: [f32; 3],
    
    // Global Expressions
    pub global_pos_expr: [String; 3],
    pub global_scale_expr: [String; 3],
    pub global_rot_expr: [String; 3],
}

impl Default for GameObject {
    fn default() -> Self {
        Self {
            id: "Obj_1".into(), name: "Yeni 3D Obje".into(), tags: "".into(),
            parameters: HashMap::new(), sockets: vec![], emitters: vec![],
            parts: vec![], bones: vec![], animations: vec![],
            health: 100.0, bounding_box: [1.0, 1.0, 1.0], is_solid: true,
            mass: 1.0, friction: 0.5, restitution: 0.0, gravity_scale: 1.0,
            cast_shadows: true, light_emission_color: [0, 0, 0], light_radius: 0.0, light_intensity: 0.0,
            ai_behavior: AiBehavior::None, aggro_radius: 0.0, custom_scripts: "".into(),
            global_pos: [0.0, 0.0, 0.0], global_scale: [1.0, 1.0, 1.0], global_rot: [0.0, 0.0, 0.0],
            global_pos_expr: ["".into(), "".into(), "".into()], global_scale_expr: ["".into(), "".into(), "".into()], global_rot_expr: ["".into(), "".into(), "".into()],
        }
    }
}
