use crate::data::scene::EditorScene;
use crate::data::element::UiElement;
use crate::data::layer::{AxiomLayer, LayerKind};
use crate::data::border::BorderTemplate;
use crate::core::types::{Anchor, AnimationType};
use crate::core::interaction::InteractionState;
use crate::render::object_viewport::ViewportCamera;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectData {
    pub scenes: Vec<crate::data::scene::EditorScene>,
    pub objects: Vec<crate::data::object::GameObject>,
    pub textures: Vec<crate::data::texture::AxiomTexture>,
    pub levels: Vec<crate::data::level::GameLevel>,
    pub settings: crate::data::settings::ProjectSettings,
    
    // Tam Kapsamlı UI ve Editör Durumu Kaydı
    pub active_scene: usize,
    pub active_level_index: Option<usize>,
    pub current_mode: EditorMode,
    pub selected_index: Option<usize>,
    pub selected_part_id: Option<String>,
    pub windowed_panels: std::collections::HashSet<String>,
    
    // Kamera Durumu
    pub camera_rot: [f32; 2],
    pub camera_pan: [f32; 2],
    pub camera_zoom: f32,
    pub camera_ortho: bool,
    pub show_gizmo: bool,
}

#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum EditorMode {
    UiDesigner,
    LevelEditor,
    ObjectEditor,
    TextureEditor,
}

pub struct AxiomStudio {
    pub scenes: Vec<EditorScene>,
    pub objects: Vec<crate::data::object::GameObject>,
    pub textures: Vec<crate::data::texture::AxiomTexture>,
    pub levels: Vec<crate::data::level::GameLevel>,
    pub active_level_index: Option<usize>,
    pub active_scene: usize,
    pub current_mode: EditorMode,
    pub selected_index: Option<usize>,
    pub last_selected: Option<usize>,
    pub selected_part_id: Option<String>,
    pub focused_index: Option<usize>,
    pub json_buffer: Option<String>,
    pub json_error: Option<String>,
    pub interaction_states: std::collections::HashMap<(usize, usize), InteractionState>,
    pub windowed_panels: std::collections::HashSet<String>,
    pub is_playing: bool,
    pub preview_stack: Vec<usize>,
    pub settings: crate::data::settings::ProjectSettings,
    pub event_queue: Vec<crate::core::events::AxiomEvent>,
    pub camera_rot: [f32; 2],
    pub camera_pan: [f32; 2],
    pub camera_zoom: f32,
    pub camera_ortho: bool,
    pub show_gizmo: bool,
    pub viewport_camera: ViewportCamera, // Object editor viewport kamerası
    pub show_fps: bool,
    pub last_frame_time: f64,
    pub current_fps: f32,

    /// Texture editor'da seçili materyal indeksi
    pub active_texture_index: usize,
    /// Önizleme modu: 0=Renk, 1=Height Map, 2=Tile tekrarı
    pub texture_preview_mode: u8,

    /// Kalıcı doku kompozisyon önbelleği (RUNTIME-ONLY).
    /// PERFORMANS: Bu cache sayesinde dokular sadece içerikleri değiştiğinde
    /// yeniden hesaplanır; obje viewport'unda ve doku editöründe her frame
    /// sıfırdan compose() çalıştırılması önlenir. Projeyle birlikte
    /// kaydedilmez (ProjectData'da yok) — proje yüklendiğinde otomatik
    /// olarak yeniden doldurulur.
    pub texture_cache: crate::render::texture_composer::TextureCache,
    
    // CSG Önbelleği: Objelerin parçalanmış 3D yüzeylerini tutar (Kamera hareket etse bile 3D veriyi önbellekten okuruz)
    // Objenin ID'si -> O objeye ait parçalanmış ve boyanmaya hazır 3D poligonlar (CSG sonrası)
    pub csg_cache: std::collections::HashMap<String, Vec<crate::render::csg::Cached3DFace>>,
    // Cache validasyon kontrolü (Hash veya Version takibi için)
    pub csg_cache_keys: std::collections::HashMap<String, u64>,

    pub history: Vec<ProjectData>,
    pub history_index: usize,
    pub last_auto_save_time: f64,
    pub last_history_check_time: f64,
    pub current_project_path: Option<String>,
    pub show_startup_modal: bool,
    pub new_project_name: String,
}

impl AxiomStudio {
    pub fn get_project_data(&self) -> ProjectData {
        ProjectData {
            scenes: self.scenes.clone(),
            objects: self.objects.clone(),
            textures: self.textures.clone(),
            levels: self.levels.clone(),
            settings: self.settings.clone(),
            
            active_scene: self.active_scene,
            active_level_index: self.active_level_index,
            current_mode: self.current_mode.clone(),
            selected_index: self.selected_index,
            selected_part_id: self.selected_part_id.clone(),
            windowed_panels: self.windowed_panels.clone(),
            
            camera_rot: self.camera_rot,
            camera_pan: self.camera_pan,
            camera_zoom: self.camera_zoom,
            camera_ortho: self.camera_ortho,
            show_gizmo: self.show_gizmo,
        }
    }

    pub fn load_project_data(&mut self, data: ProjectData) {
        self.scenes = data.scenes;
        self.objects = data.objects;
        self.textures = data.textures;
        self.levels = data.levels;
        self.settings = data.settings;
        // Önceki projeye/duruma ait doku önbelleğini temizle — id çakışması
        // olsa bile hash kontrolü güvenlidir, ama gereksiz belleği boşaltmak
        // ve net bir başlangıç durumu sağlamak için sıfırlıyoruz.
        self.texture_cache = crate::render::texture_composer::TextureCache::new();
        
        self.active_scene = data.active_scene;
        self.active_level_index = data.active_level_index;
        self.current_mode = data.current_mode;
        self.selected_index = data.selected_index;
        self.selected_part_id = data.selected_part_id;
        self.windowed_panels = data.windowed_panels;
        
        self.camera_rot = data.camera_rot;
        self.camera_pan = data.camera_pan;
        self.camera_zoom = data.camera_zoom;
        self.camera_ortho = data.camera_ortho;
        self.show_gizmo = data.show_gizmo;
    }

    pub fn push_history(&mut self) {
        let current_data = self.get_project_data();
        let should_push = if let Some(last) = self.history.get(self.history_index) {
            serde_json::to_string(last).unwrap_or_default() != serde_json::to_string(&current_data).unwrap_or_default()
        } else {
            true
        };
        
        if should_push {
            self.history.truncate(self.history_index + 1);
            self.history.push(current_data);
            if self.history.len() > 200 {
                self.history.remove(0);
            } else {
                self.history_index = self.history.len() - 1;
            }
        }
    }

    pub fn undo(&mut self) {
        if self.history_index > 0 {
            self.history_index -= 1;
            let data = self.history[self.history_index].clone();
            self.load_project_data(data);
        }
    }

    pub fn redo(&mut self) {
        if self.history_index + 1 < self.history.len() {
            self.history_index += 1;
            let data = self.history[self.history_index].clone();
            self.load_project_data(data);
        }
    }

    pub fn save_project(&mut self) {
        if let Some(path) = &self.current_project_path {
            std::fs::create_dir_all("projects").ok();
            let data = self.get_project_data();
            let json = serde_json::to_string_pretty(&data).unwrap();
            
            // Backup oluşturma: Eğer dosya zaten varsa .bak olarak kopyala (Veri Bozulmasını Önler)
            if std::path::Path::new(path).exists() {
                let bak_path = format!("{}.bak", path);
                std::fs::copy(path, &bak_path).ok();
            }
            
            // Atomic (Güvenli) Kaydetme: Önce gecici .tmp dosyasına yaz
            let tmp_path = format!("{}.tmp", path);
            if std::fs::write(&tmp_path, &json).is_ok() {
                // Yazma başarılıysa, ana dosyanın üstüne taşı (Böylece elektrik gitse bile orijinal dosya bozulmaz)
                std::fs::rename(&tmp_path, path).ok();
                // Olayı her saniye loglamamak için sadece SystemMessage'i sessizce geçebiliriz ya da console a yazabiliriz.
                // self.event_queue.push(crate::core::events::AxiomEvent::SystemMessage(format!("Proje kaydedildi: {}", path)));
            }
        }
    }
}

impl Default for AxiomStudio {
    fn default() -> Self {
        let mut bg = UiElement::default();
        bg.id = "BG_Master".into();
        bg.width = 100.0; bg.height = 100.0;
        bg.z_index = -100;
        bg.layers.clear();
        let mut bg_fill = AxiomLayer::default();
        bg_fill.id = "BG_Noktalar".into();
        bg_fill.kind = LayerKind::Fill;
        bg_fill.content = ".".into();
        bg_fill.fg_color = [40, 40, 40];
        bg_fill.bg_color = [10, 10, 15];
        bg.layers.push(bg_fill);

        let mut btn = UiElement::default();
        btn.id = "Start_Button".into();
        btn.anchor = Anchor::TopCenter; btn.pos_y = 10.0;
        btn.width = 50.0; btn.height = 10.0;
        btn.action_binding = "START_GAME".into();
        
        btn.layers[0].fg_color = [255, 200, 0]; btn.layers[0].bg_color = [30, 20, 10]; // Fill
        btn.layers[1].fg_color = [255, 100, 50]; btn.layers[1].bg_color = [30, 20, 10]; // Border
        btn.layers[1].border = BorderTemplate::round();
        btn.layers[2].content = "AXIOM ENGINE".into(); // Text
        btn.layers[2].fg_color = [255, 255, 255];
        btn.layers[2].animation = AnimationType::PulseColor; btn.layers[2].anim_speed = 3.0;
        btn.layers[2].drop_shadow = true;
        
        // Hover states
        btn.layers[0].hover_state.enabled = true;
        btn.layers[0].hover_state.bg_color = [40, 40, 50];
        btn.layers[1].hover_state.enabled = true;
        btn.layers[1].hover_state.fg_color = [255, 255, 255]; btn.layers[1].hover_state.bg_color = [40, 40, 50];

        // FX Layer
        let mut fx = AxiomLayer::default();
        fx.id = "Yildizlar".into();
        fx.kind = LayerKind::Text;
        fx.content = "*  *  *  *  *".into();
        fx.offset_x = -10.0; fx.offset_y = 50.0;
        fx.fg_color = [100, 255, 100];
        fx.letter_spacing = 25.0;
        fx.animation = AnimationType::Wave; fx.anim_speed = 1.5; fx.anim_amplitude = 10.0;
        btn.layers.push(fx);

        let initial_scene = EditorScene {
            name: "Ana Menü".into(),
            elements: vec![bg, btn],
        };
        let settings_scene = EditorScene {
            name: "Ayarlar Menüsü".into(),
            elements: vec![],
        };
        
        Self {
            scenes: vec![initial_scene, settings_scene],
            objects: vec![],
            textures: vec![],
            levels: vec![],
            active_level_index: None,
            active_scene: 0,
            current_mode: EditorMode::UiDesigner,
            selected_index: Some(1),
            last_selected: Some(1),
            selected_part_id: None,
            focused_index: None,
            json_buffer: None,
            json_error: None,
            interaction_states: std::collections::HashMap::new(),
            windowed_panels: std::collections::HashSet::new(),
            is_playing: false,
            preview_stack: vec![0],
            settings: crate::data::settings::ProjectSettings::default(),
            event_queue: Vec::new(),
            camera_rot: [30.0, 45.0],
            camera_pan: [0.0, 0.0],
            camera_zoom: 40.0,
            camera_ortho: false,
            show_gizmo: true,
            active_texture_index: 0,
            texture_preview_mode: 0,
            texture_cache: crate::render::texture_composer::TextureCache::new(),
            csg_cache: std::collections::HashMap::new(),
            csg_cache_keys: std::collections::HashMap::new(),
            viewport_camera: ViewportCamera::default(),
            show_fps: false,
            last_frame_time: 0.0,
            current_fps: 0.0,
            history: vec![],
            history_index: 0,
            last_auto_save_time: 0.0,
            last_history_check_time: 0.0,
            current_project_path: None,
            show_startup_modal: true,
            new_project_name: "YeniProje".into(),
        }
    }
}

impl eframe::App for AxiomStudio {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        // PERFORMANS: Eskiden burada KOŞULSUZ ctx.request_repaint() çağrılıyordu.
        // Bu, egui'nin normal davranışını (sadece girdi/değişiklik olduğunda
        // yeniden çizmek) devre dışı bırakıp uygulamayı donanımın izin verdiği
        // en yüksek hızda, durmaksızın yeniden çiziyordu — CPU/GPU sürekli
        // %100 kullanımda kalıyor ve bu da Obje/Doku editöründe parça sayısı
        // arttıkça hissedilen FPS düşüşünün temel sebeplerinden biriydi.
        //
        // Artık sadece gerçekten sürekli animasyon gerektiren durumlarda
        // (Oynatma modu veya UI Designer'daki canlı sahne animasyonları)
        // maksimum hızda yeniden çiziyoruz. Diğer modlarda (Obje Editörü,
        // Doku Editörü, Seviye Editörü gibi statik düzenleme ekranlarında)
        // egui zaten fare/klavye girdisinde otomatik olarak yeniden çizer;
        // biz sadece düşük bir arka plan frekansında (saniyede ~10 kez)
        // tazeleme istiyoruz ki saat/auto-save gibi zaman bazlı kontroller
        // yine de çalışabilsin.
        let needs_continuous_repaint =
            self.show_fps || self.is_playing || self.current_mode == EditorMode::UiDesigner;
        if needs_continuous_repaint {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        let time = ctx.input(|i| i.time);
        
        if self.show_startup_modal {
            egui::Window::new("Axiom Proje Yöneticisi")
                .collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.heading("Axiom Studio'ya Hoşgeldiniz");
                    ui.label("Projelerinizi özel .axiom formatında kaydedin veya yükleyin.");
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        ui.label("Yeni Proje Adı:");
                        ui.text_edit_singleline(&mut self.new_project_name);
                        if ui.button("Yeni Proje Oluştur").clicked() {
                            std::fs::create_dir_all("projects").ok();
                            let path = format!("projects/{}.axiom", self.new_project_name);
                            self.current_project_path = Some(path.clone());
                            self.show_startup_modal = false;
                            self.push_history(); // Initial state
                            self.save_project();
                        }
                    });
                    
                    ui.separator();
                    ui.heading("Kayıtlı Projeler (projects/ klasörü)");
                    if let Ok(entries) = std::fs::read_dir("projects") {
                        let mut found = false;
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.extension().and_then(|e| e.to_str()) == Some("axiom") {
                                found = true;
                                let filename = path.file_name().unwrap().to_string_lossy();
                                ui.horizontal(|ui| {
                                    ui.label(filename.as_ref());
                                    if ui.button("Yükle").clicked() {
                                        match std::fs::read_to_string(&path) {
                                            Ok(content) => {
                                                match serde_json::from_str::<ProjectData>(&content) {
                                                    Ok(data) => {
                                                        self.load_project_data(data);
                                                        self.current_project_path = Some(path.to_string_lossy().to_string());
                                                        self.show_startup_modal = false;
                                                        self.history.clear();
                                                        self.history_index = 0;
                                                        self.push_history();
                                                    }
                                                    Err(e) => {
                                                        eprintln!("JSON parse hatası: {}", e);
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("Dosya okuma hatası: {}", e);
                                            }
                                        }
                                    }
                                });
                            }
                        }
                        if !found { ui.label("Kayıtlı .axiom projesi bulunamadı."); }
                    }
                });
            return;
        }

        // Auto-save & push history check - Kesin Veri Güvenliği (Zero-Data Loss)
        let time_f64 = time;
        // Her 1 saniyede bir History değişimi var mı diye kontrol et (Kayıt geçmişi yakalama)
        if time_f64 - self.last_history_check_time > 1.0 {
            self.push_history(); // Yalnızca değişiklik varsa belleğe history kaydı atar
            self.last_history_check_time = time_f64;
        }

        // Her 10 saniyede bir diske güvenli kaydet (Atomic Save & Backup)
        if time_f64 - self.last_auto_save_time > 10.0 {
            self.save_project();
            self.last_auto_save_time = time_f64;
        }

        let time_f32 = time as f32;

        crate::ui::toolbar::show(self, ctx);
        
        match self.current_mode {
            EditorMode::UiDesigner => {
                crate::ui::inspector::show(self, ctx);
                crate::ui::explorer::show(self, ctx);
                crate::render::canvas::show(self, ctx, time_f32);
            },
            EditorMode::LevelEditor => {
                crate::ui::level_editor::show(self, ctx);
            },
            EditorMode::ObjectEditor => {
                crate::ui::object_editor::show(self, ctx);
            },
            EditorMode::TextureEditor => {
                crate::ui::texture_editor::show(self, ctx);
            }
        }
        
        crate::ui::settings_modal::show(self, ctx);

        let events: Vec<_> = self.event_queue.drain(..).collect();
        for event in events {
            match event {
                crate::core::events::AxiomEvent::ActionTriggered(action) => {
                    println!("[AxiomStudio] Aksiyon Tetiklendi: {}", action);
                    // Automatic routing for builtin actions
                    if action.starts_with("PUSH:") {
                        let target = action.replace("PUSH:", "").trim().to_string();
                        self.event_queue.push(crate::core::events::AxiomEvent::PushScene(target));
                    } else if action == "POP" {
                        self.event_queue.push(crate::core::events::AxiomEvent::PopScene);
                    }
                }
                crate::core::events::AxiomEvent::SceneChangeRequested(scene_name) => {
                    if let Some(idx) = self.scenes.iter().position(|s| s.name == scene_name) {
                        self.active_scene = idx;
                        self.preview_stack = vec![idx];
                    }
                }
                crate::core::events::AxiomEvent::PushScene(scene_name) => {
                    if let Some(idx) = self.scenes.iter().position(|s| s.name == scene_name) {
                        if !self.preview_stack.contains(&idx) {
                            self.preview_stack.push(idx);
                        }
                    }
                }
                crate::core::events::AxiomEvent::PopScene => {
                    if self.preview_stack.len() > 1 {
                        self.preview_stack.pop();
                    }
                }
                crate::core::events::AxiomEvent::SystemMessage(msg) => {
                    println!("[AxiomStudio] Sistem: {}", msg);
                }
            }
        }

        let time_f64 = time;
        if self.last_frame_time > 0.0 {
            let dt = time_f64 - self.last_frame_time;
            if dt > 0.0 {
                self.current_fps = (1.0 / dt) as f32;
            }
        }
        self.last_frame_time = time_f64;

        if self.show_fps {
            egui::Area::new(egui::Id::new("fps_counter_area"))
                .fixed_pos(egui::pos2(10.0, 40.0))
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new(format!("FPS: {:.1}", self.current_fps))
                            .color(if self.current_fps > 50.0 { egui::Color32::GREEN } else { egui::Color32::RED })
                            .background_color(egui::Color32::from_black_alpha(200))
                            .strong()
                            .heading()
                    );
                });
        }
    }
}