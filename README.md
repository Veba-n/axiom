# Axiom Engine — Teknik Mimari ve Modül Spesifikasyonu (v0.1)

Bu doküman, Axiom Engine'in tam kaynak kodu analizine dayanan teknik bir referans belgesidir.
Bellek düzenleri, WGPU render boru hatları, CSG matematik algoritmaları, texture compositor
pipeline'ı ve çalışma zamanı (runtime) mimarisini doğrudan kaynak koddan elde edilen
spesifik değerlerle inceler.

---

## Bölüm 2.0 — Runtime Çekirdeği (`runtime/src/`)

### 2.1 Giriş Noktası (`main.rs`)

Runtime, terminale bağımlı bir başlatıcı arayüze sahiptir. `dialoguer` crate'i üzerinden
`ColorfulTheme` ile renklendirilmiş bir `Select` menüsü oluşturulur. Kullanıcı hangi
sahne (room) dosyasını yükleyeceğini bu menüden seçer. `crossterm` crate'i
`ClearType::All` komutuyla terminali temizler ve imleci `(0, 0)` konumuna taşır.
Seçimden sonra kontrol `engine::run(room_name)` async fonksiyonuna devredilir.

### 2.2 WGPU Donanım Başlatma Sekansı (`engine.rs`)

`engine::run()` fonksiyonu `async` olarak tanımlıdır. Donanım bağlantısı şu sırayla kurulur:

```
1. wgpu::Instance::new(Backends::all())
2. instance.create_surface(Arc<Window>)        → surface
3. instance.request_adapter(HighPerformance)   → adapter  [await]
4. adapter.request_device(Features::empty())   → (device, queue)  [await]
5. surface.get_default_config(adapter, w, h)   → config
6. surface.configure(device, config)
```

`PowerPreference::HighPerformance` ile birden fazla GPU varsa ayrık (discrete) GPU
seçilir. `force_fallback_adapter: false` olduğundan yazılım tabanlı SwiftShader devre
dışıdır. Başlangıç çözünürlüğü `1280x720` pikseldir.

**Render döngüsü** `EventLoop::run` closure'ı içinde çalışır:

- `WindowEvent::Resized` → `config.width/height` güncellenir, `surface.configure`
  ile swapchain yeniden oluşturulur.
- `WindowEvent::RedrawRequested` → `surface.get_current_texture()` ile sonraki frame
  tamponu alınır. `LoadOp::Clear` ile yüzey `wgpu::Color { r:0.1, g:0.1, b:0.1, a:1.0 }`
  değeri (sRGB'de yaklaşık `#1A1A1A` koyu gri) ile temizlenir. `StoreOp::Store` ile
  tampon kaydedilir. `queue.submit` → `frame.present()` ile frame GPU'ya gönderilir.
- `Event::AboutToWait` → `window.request_redraw()` bir sonraki frame'i tetikler
  (vsync veya hız limiti olmaksızın aktif döngü).
- `KeyCode::Escape` → `control_flow.exit()` ile döngü sonlandırılır.

### 2.3 Sahne Dosyası Deserialization (`data_parser.rs`)

Runtime, Studio'dan dışa aktarılan JSON sahne dosyalarını şu yapıya deserialize eder:

```rust
RoomData {
    room: RoomInfo {
        id: String,
        name: String,
        dimensions: RoomDimensions { width: u32, length: u32, height: u32 },
    },
    lighting: Lighting {
        ambient: f32,           // 0.0–1.0 dünya ışık yoğunluğu
        player_fov: f32,        // Derece cinsinden görüş alanı
        flashlight_range: f32,
    },
    player_start: PlayerStart { x: f32, y: f32, angle: f32 },
    objects: Vec<GameObject> {
        object_type: String,    // "enemy", "pickup", "trigger" vb.
        x: f32, y: f32, z: f32,
    },
}
```

`load_room_data(path)` fonksiyonu `fs::read_to_string` ile dosyayı belleğe alır,
ardından `serde_json::from_str` ile tek seferde parse eder. Hata yönetimi
`Box<dyn std::error::Error>` ile caller'a iletilir.

---

## Bölüm 3.0 — Studio GPU Render Pipeline (`studio/src/render/gpu.rs`)

### 3.1 Vertex Bellek Düzeni (Memory Layout)

Köşeler GPU'ya `#[repr(C)]` ve `bytemuck::{Pod, Zeroable}` trait'leriyle işaretlenmiş
`GpuVertex` struct'ı aracılığıyla iletilir:

```rust
#[repr(C)]
pub struct GpuVertex {
    pub position: [f32; 3],  // offset:  0 byte, boyut: 12 byte
    pub uv:       [f32; 2],  // offset: 12 byte, boyut:  8 byte
    pub color:    [f32; 4],  // offset: 20 byte, boyut: 16 byte
}                            // toplam stride: 36 byte/vertex
```

`VertexBufferLayout` içinde her alan için ayrı `VertexAttribute` tanımlanmıştır:

| Shader Location | Alan | Format | Byte Offset |
|---|---|---|---|
| `@location(0)` | position | `Float32x3` | 0 |
| `@location(1)` | uv | `Float32x2` | 12 |
| `@location(2)` | color | `Float32x4` | 20 |

Kamera verisi `CameraUniform { view_proj: [[f32;4];4] }` olarak ayrı bir uniform
buffer'da tutulur ve `@group(0) @binding(0)` konumuna bağlanır.

### 3.2 Dinamik Buffer Yönetimi

`Custom3dCallback::prepare()` aşamasında iki ayrı vertex buffer yönetilir:

- **`SolidBuffer`**: Katı yüzey (TriangleList) geometrisi.
- **`LineBuffer`**: Wireframe/tel kafes (LineList) geometrisi.

Her buffer, kapasite yetersiz kaldığında `next_power_of_two().max(1024)` formülüyle
2'nin bir sonraki katına büyütülür (minimum 1024 vertex). Bu sayede frame başına
sürekli bellek tahsisi (allocation) gerçekleşmez; yalnızca kapasite aşımında
`device.create_buffer` çağrılır. Veri, `queue.write_buffer` ile sıfır kopyalama
(zero-copy) yaklaşımıyla VRAM'e yazılır.

### 3.3 İki Pipeline Konfigürasyonu

`Custom3dPipeline::new()` içinde aynı WGSL shader modülünden iki ayrı
`wgpu::RenderPipeline` oluşturulur:

#### Solid Pipeline (Katı Yüzeyler)
```
topology:            TriangleList
cull_mode:           None          ← back-face yok; editörde her yön görünür
depth_write_enabled: true
depth_compare:       LessEqual     ← yakın pikseller uzaktakileri örter
depth_format:        Depth24Plus
blend:               ALPHA_BLENDING
depth_bias_constant: 0
```

#### Line Pipeline (Tel Kafes)
```
topology:            LineList
cull_mode:           None
depth_write_enabled: false         ← çizgiler depth buffer'a yazmaz
depth_compare:       LessEqual     ← solid'in arkasındaki çizgiler gizlenir
depth_format:        Depth24Plus
blend:               ALPHA_BLENDING
depth_bias_constant: -2            ← Z-fighting engeli; çizgileri öne çeker
depth_bias_slope:    -1.0
```

`depth_write_enabled: false` konfigürasyonu, katı yüzeyler tamamen çizildikten
sonra wireframe'in depth buffer'ı kirletmesini engeller; böylece çizgiler kendi
arkalarındaki solid geometriyi örtmez.

---

## Bölüm 4.0 — WGSL Gölgelendirici (`render/shaders/main.wgsl`)

### 4.1 Shader Yapısı

```wgsl
struct CameraUniform { view_proj: mat4x4<f32> };
@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv:       vec2<f32>,
    @location(2) color:    vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv:    vec2<f32>,
    @location(1) color: vec4<f32>,
};
```

**Vertex shader** (`vs_main`): Her köşe için dünya koordinatı
`camera.view_proj * vec4<f32>(position, 1.0)` çarpımıyla clip-space'e
dönüştürülür. UV ve renk değerleri doğrudan fragment aşamasına aktarılır.

**Fragment shader** (`fs_main`): Doku örnekleme (texture sampling) yapılmaz;
fragment çıktısı doğrudan interpolasyon renkidir (`return in.color`). Bu yaklaşım,
Axiom'un ASCII/karakter tabanlı render paradigmasıyla uyumludur — renk verisi
CPU'da (texture composer'da) hesaplanır ve vertex rengine gömülür.

---
## Bölüm 1.0 — Genel Mimari ve Çalışma Alanı Yapısı

### 1.1 Workspace Düzeni

Proje kökü (`axiom/`), Cargo workspace olarak yapılandırılmıştır. `Cargo.toml` dosyası
iki üye (member) crate'i bir arada yönetir: `runtime` ve `studio`. Bu yapı sayesinde her
iki crate aynı `target/` dizinini paylaşır; ortak bağımlılıklar tek seferinde derlenir.

```text
axiom/
├── Cargo.toml                        # Workspace manifest — members = ["runtime", "studio"]
├── Cargo.lock                        # Bağımlılık kilidi (lock) — tekrarlanabilir build
│
├── runtime/                          # Oyun İstemcisi (Standalone Engine)
│   ├── Cargo.toml                    # winit, wgpu, serde_json, env_logger bağımlılıkları
│   └── src/
│       ├── main.rs                   # İstemci giriş noktası; dialoguer menü, konsol yönetimi
│       ├── engine.rs                 # WGPU EventLoop, adapter, device, swapchain
│       └── data_parser.rs            # JSON → Rust struct deserialization (serde)
│
├── studio/                           # Geliştirici Editörü (eframe/egui Uygulaması)
│   ├── Cargo.toml                    # eframe 0.27.2, egui, wgpu 0.19, bytemuck, glam, earcutr
│   └── src/
│       ├── main.rs                   # eframe::run_native başlatıcısı
│       ├── lib.rs                    # Crate kök modül bildirimleri
│       ├── app.rs                    # AxiomStudio state makinesi, update döngüsü
│       ├── core/
│       │   ├── mod.rs
│       │   ├── events.rs             # Ham I/O event soyutlaması
│       │   ├── interaction.rs        # Bounding box, seçim, transform hesapları
│       │   └── types.rs              # Anchor, TextAlign, AnimationType enumları
│       ├── data/
│       │   ├── mod.rs
│       │   ├── border.rs             # BorderPiece, BorderTemplate, ExtraBorder
│       │   ├── element.rs            # UiElement — UI sahne birimi
│       │   ├── layer.rs              # AxiomLayer, LayerKind, render Z-index
│       │   ├── level.rs              # GameLevel, ObjectInstance — dünya yerleşimi
│       │   ├── object.rs             # GameObject, ObjectPart, PrimitiveShape, BooleanOp
│       │   ├── scene.rs              # Sahne graph yönetimi
│       │   ├── settings.rs           # Studio kullanıcı yapılandırması
│       │   ├── texture.rs            # TextureLayer, AxiomTexture, BlendMode, HeightFunction
│       │   └── texture_presets.rs    # Statik ön tanımlı materyal profilleri
│       ├── render/
│       │   ├── mod.rs
│       │   ├── canvas.rs             # egui Painter köprüsü; sahne çizim ana döngüsü
│       │   ├── csg.rs                # BSP tabanlı konveks bölme (split_poly, subtract_convex)
│       │   ├── gpu.rs                # WGPU custom pipeline, GpuVertex, buffer yönetimi
│       │   ├── object_viewport.rs    # Tek obje 3D projeksiyon izleyicisi
│       │   ├── texture_composer.rs   # CPU texture baking, blend modları, height map
│       │   └── shaders/
│       │       └── main.wgsl         # WGSL vertex/fragment gölgelendirici
│       └── ui/
│           ├── mod.rs
│           ├── explorer.rs           # Sol panel — sahne hiyerarşi ağacı (TreeView)
│           ├── inspector.rs          # Sağ panel — özellik (property) mutatörü
│           ├── level_editor.rs       # Ana harita/grid tasarım ekranı
│           ├── object_editor.rs      # CSG yüzey ve part düzenleyicisi
│           ├── object_editor_cache.rs # CPU darboğazını önleyen yüzey önbelleği
│           ├── settings_modal.rs     # Yapılandırma popup penceresi
│           ├── texture_editor.rs     # Materyal ve katman düzenleyicisi
│           ├── toolbar.rs            # Save/Load/Play komut çubuğu
│           └── widgets.rs            # Özel egui widget/macro implementasyonları
│
├── data/                             # Örnek oyun sahne dosyaları
│   └── room_01.json                  # RoomData formatında test seviyesi
└── analizler/                        # Mimari teknik notlar (gitignore'd)
```

### 1.2 Bağımlılık Matrisi

| Crate | Versiyon | Amaç |
|---|---|---|
| `eframe` | 0.27.2 (+wgpu feature) | egui uygulama çerçevesi |
| `egui` | 0.27.2 | Anlık mod GUI (Immediate Mode) |
| `wgpu` | 0.19 | Cross-platform GPU API |
| `bytemuck` | 1.15 (+derive) | `GpuVertex` → ham byte (Pod/Zeroable) |
| `glam` | 0.27 | SIMD-destekli lineer cebir (mat4, vec3) |
| `earcutr` | 0.5.0 | Poligon triangülasyon (ear-clipping) |
| `fasteval` | 0.2.4 | Parametrik ifade değerlendirici |
| `serde` | 1.0 (+derive) | Serialization/Deserialization makroları |
| `serde_json` | 1.0 | JSON okuma/yazma |
| `bincode` | 1.3 | Binary serileştirme (ikili format) |
| `winit` | * | İşletim sistemi pencere ve event döngüsü |
| `env_logger` | * | Runtime loglama |

---

## Bölüm 5.0 — CSG Matematik Motoru (`studio/src/render/csg.rs`)

### 5.1 Temel Veri Yapıları

```rust
pub struct Vertex { pub pos: [f32; 3], pub uv: [f32; 2] }
pub struct Plane  { pub n: [f32; 3],   pub d: f32 }

pub struct Cached3DFace {
    pub verts:         Vec<[f32; 3]>,
    pub face_id:       String,
    pub is_inner_wall: bool,
    pub part_id:       String,
    pub part_index:    usize,
    pub destroyed_ratio: f32,
}
```

### 5.2 Düzlem Oluşturma (`make_plane`)

Üç noktadan (v0, v1, v2) kros çarpım ile yüzey normali hesaplanır:

```
dx1 = v1 - v0,  dx2 = v2 - v1
n   = dx1 × dx2            ← çapraz çarpım
n   = n / |n|              ← normalizasyon (|n| > 0.00001 koşuluyla)
d   = -(n · v0)            ← orijine uzaklık
```

### 5.3 Poligon Bölme Algoritması (`split_poly`)

Tolerans sabiti `EPSILON = 1e-4`. Her köşe düzleme göre sınıflandırılır:

| Koşul | Etiket | Anlam |
|---|---|---|
| `dist > +EPSILON` | `+1` | Front (dışarıda) |
| `dist < -EPSILON` | `-1` | Back (içeride) |
| Diğer | `0` | Coplanar (düzlem üzerinde) |

**Coplanar çözümleme:** Eğer hiç Front ve Back yoksa poligon düzlemin tam üzerindedir.
Bu durumda poligonun kendi normali ile kesme düzleminin normali arasındaki dot product
hesaplanır. `dot > 0.0` ve işlem subtract ise yüzey imha edilir (Z-fighting önlemi);
aksi halde korunur.

**Kesişim noktası interpolasyonu:**
```
t = dist[i] / (dist[i] - dist[j])
pos = v1.pos + t * (v2.pos - v1.pos)
uv  = v1.uv  + t * (v2.uv  - v1.uv)
```

**NaN/Spike koruması:** Oluşturulan köşe `is_nan()` veya `!is_finite()` kontrolünden
geçirilir; hatalı veri pipeline'a girmez.

**Temizleyici (clean_poly):** Kesim sonrası birbirine `√(dx²+dy²+dz²) < 1e-6` mesafeden
yakın (üst üste binen) köşeler kaldırılır. Sonuç `< 3` köşe kalırsa poligon geçersiz
sayılır ve boş döndürülür.

### 5.4 Boolean Subtraction (`subtract_convex`)

```rust
pub fn subtract_convex(
    poly: &[Vertex],
    planes: &[Plane],
    is_subtract_vol: bool,
) -> (Vec<Vec<Vertex>>, Vec<Vertex>)
```

`planes` dizisindeki her düzlem sırayla uygulanır. `split_poly` ile dışarıda kalan
(`outside`) parçalar kesin sonuç listesine eklenir; içeride kalan (`inside`) parça
bir sonraki düzleme sokulur. `inside` boşalırsa döngü kırılır (`break`). İşlem sonunda
`(dış_parçalar, kalan_iç_parça)` tuple'ı döner.

---

## Bölüm 6.0 — Texture Compositor (`studio/src/render/texture_composer.rs`)

### 6.1 Hücre ve Kompozisyon Yapıları

```rust
pub struct TextureCell {
    pub ch:       char,
    pub fg:       [u8; 3],   // ön plan rengi (RGB)
    pub alpha:    f32,        // şeffaflık 0.0–1.0
    pub height:   f32,        // height map katkısı
    pub emission: f32,        // ışıma değeri
    pub visible:  bool,
}

pub struct ComposedTexture {
    pub width:      u32,
    pub height:     u32,
    pub base_color: [u8; 3],
    pub cells:      Vec<Vec<TextureCell>>,   // [y][x] indeksleme
    pub height_map: Vec<Vec<f32>>,
    pub has_border: bool,
}
```

### 6.2 `TextureCache` — Önbellekleme Mekanizması

CPU'da her frame çalıştırılan `compose()` fonksiyonu pahalıdır. `TextureCache`,
`HashMap<String, (AxiomTexture, ComposedTexture, Option<egui::TextureHandle>)>`
yapısında her dokunun önceki kopyasını saklar. `get_or_compose()` çağrısında
`cached_tex != texture` karşılaştırması ile değişim tespiti yapılır:

- **Değişim yoksa:** Önbellekteki `ComposedTexture` anında döner. Hiç hesaplama yapılmaz.
- **Değişim varsa:** `compose()` → `bake_texture_to_image()` → `ctx.load_texture()` zinciri
  çalışır; sonuç hem CPU cache'e hem GPU `TextureHandle`'a yazılır.

`sync()` metodu, projede artık olmayan dokuların cache'den silinmesini (`retain`) sağlar.

### 6.3 Texture Baking (`bake_texture_to_image`)

`egui::FontId::new(16.0, Monospace)` ile her karakter için font atlas'tan UV koordinatları
alınır. Her hücre için:

```
dest_x = tx * char_width   (piksel)
dest_y = ty * char_height  (piksel)
```

Font atlas'taki her piksel için `coverage` (atlas şeffaflık değeri) ile hücrenin
`cell.alpha` değeri çarpılır:

```rust
final_alpha = coverage * cell.alpha
r = fg.r * final_alpha + bg.r * (1.0 - final_alpha)
```

Bu, yazı tipi antialiasing'ini koruyarak hücre saydamlığını doğru uygular.

### 6.4 Blend Modları (`blend_cell`)

| Mod | Formül |
|---|---|
| `Normal` | `lerp(dst, src, alpha)` |
| `Additive` | `dst + src * alpha` (saturating_add) |
| `Multiply` | `dst * src / 255 * alpha + dst * (1-alpha)` |
| `Subtractive` | `dst - src * alpha` (saturating_sub) |
| `Overlay` | `bf < 0.5 → 2*bf*sf ; ≥0.5 → 1-2*(1-bf)*(1-sf)` |

### 6.5 Katman Üretim Modları (`LayerGenMode`)

| Mod | Algoritma |
|---|---|
| `Solid` | `pattern[(tx+ty) % len]` — döngüsel karakter indexleme |
| `Noise` | `(tx*13579 XOR ty*97531) / u32::MAX` — deterministik hash; density eşiği |
| `Checker` | `(tx+ty) % 2 == 0` — dama deseni |
| `Border` | `x==0 || x==w-1 || y==0 || y==h-1` — kenar tespiti |
| `Fill` | `pattern_spacing > 1` aralıklı doldurma |
| `DirectionalBorder` | 8 ayrı `BorderPiece` (köşeler + 4 kenar) |

### 6.6 UV ve Tile Koordinatları (`tile_coords`)

Döndürme (`rotation`) sıfırdan farklıysa her hücre koordinatı merkez etrafında
döndürülür:
```
cx = w/2,  cy = h/2
dx = x - cx,  dy = y - cy
rx = dx*cos(θ) - dy*sin(θ) + cx
ry = dx*sin(θ) + dy*cos(θ) + cy
```

`uv_scale` büyük değer = sık tekrar, küçük değer = geniş tile anlamına gelir.
`tile_wrap` modu üç seçenekten biridir:

- `Repeat` → `v.rem_euclid(max)` — sonsuz döngüsel tekrar
- `Mirror` → `period = max*2; r<max → r ; r≥max → period-r-1` — ayna yansıması
- `Clamp` → `v.clamp(0, max-1)` — kenar piksel tekrarı yok

### 6.7 Height Modülasyonu (`height_modulation`)

`height_amplitude == 0` ise fonksiyon her zaman `0.0` döner (geriye dönük uyumluluk).

| HeightFunction | Formül |
|---|---|
| `Flat` | `0.0` — sabit yükseklik |
| `Noise` | `(hash-0.5)*2 * amplitude` — kaba taş/sıva efekti |
| `Wave` | `sin((tx+ty) * freq * 0.3) * amplitude` — tahta damarı |
| `CellBulge` | `(1 - max(|mx-0.5|, |my-0.5|)*2) * amplitude` — kabartmalı tuğla |

---

## Bölüm 7.0 — Veri Modelleri (`studio/src/data/`)

### 7.1 Texture Katman Modeli (`texture.rs`)

`TextureLayer` struct'ı 20'den fazla alan içerir; tüm alanlar `serde` ile
JSON'a serileştirilir. Kritik alanlar:

```rust
pub struct TextureLayer {
    pub name:             String,
    pub is_visible:       bool,
    pub z_index:          i32,
    pub blend_mode:       BlendMode,          // Normal|Additive|Multiply|Subtractive|Overlay
    pub opacity:          f32,                // 1.0 default
    pub gen_mode:         LayerGenMode,       // Solid|Noise|Checker|Border|Fill|DirectionalBorder
    pub pattern:          String,             // ASCII karakter seti; default: "▓"
    pub noise_density:    f32,                // 0.0–1.0
    pub height_val:       f32,                // taban yükseklik; default: 1.5
    pub emission_val:     f32,
    pub height_function:  HeightFunction,     // Flat|Noise|Wave|CellBulge
    pub height_amplitude: f32,                // 0.0 = devre dışı (uyumluluk)
    pub height_frequency: f32,                // default: 1.0
    pub fg_color:         [u8; 3],
    pub fg_gradient_end:  Option<[u8; 3]>,    // dikey gradient uç rengi
    pub bg_color:         [u8; 3],            // default: [0,0,0]
    pub bg_alpha:         f32,
    pub uv_scale:         [f32; 2],           // default: [1.0, 1.0]
    pub uv_offset:        [f32; 2],
    pub tile_wrap:        TileWrapMode,       // Repeat|Mirror|Clamp
    pub border:           BorderTemplate,
    pub extra_borders:    Vec<ExtraBorder>,
    pub pattern_spacing:  usize,              // default: 1 (kapalı)
    pub rotation_3d:      [f32; 3],           // X,Y,Z dönüş açıları
    pub scale_3d:         [f32; 3],           // X,Y,Z ölçek; default: [1,1,1]
    pub manual_painting:  bool,
    pub pattern_lock:     bool,
}
```

### 7.2 Dünya Nesnesi Modeli (`object.rs`)

```rust
pub struct GameObject {
    pub id:           String,
    pub parts:        Vec<ObjectPart>,
    pub bones:        Vec<Bone>,
    pub animations:   Vec<AnimationSequence>,
    pub parameters:   HashMap<String, f32>,   // "width"→10.0, "height"→50.0 gibi
    pub sockets:      Vec<ObjectSocket>,
    pub emitters:     Vec<ParticleEmitter>,
    // Fizik
    pub health:       f32,    // default: 100.0
    pub mass:         f32,    // default: 1.0
    pub friction:     f32,    // default: 0.5
    pub restitution:  f32,    // default: 0.0
    pub gravity_scale: f32,   // default: 1.0
    pub bounding_box: [f32; 3],
    pub is_solid:     bool,
    // Işık
    pub cast_shadows:         bool,
    pub light_emission_color: [u8; 3],
    pub light_radius:         f32,
    pub light_intensity:      f32,
    // AI
    pub ai_behavior:  AiBehavior,  // None|Passive|Hostile|Fleeing|Patrol
    pub aggro_radius: f32,
    pub custom_scripts: String,
    // Global transform ifadeleri (parametrik override)
    pub global_pos_expr:   [String; 3],
    pub global_scale_expr: [String; 3],
    pub global_rot_expr:   [String; 3],
}
```

`ObjectPart` her geometri parçasını tanımlar ve şu özellikler içerir:

- `shape: PrimitiveShape` — `Cube|Sphere|Pyramid|Cylinder|HalfCylinder|TriangularPrism|PentagonPrism|HexagonPrism|Cone|Torus|CustomMesh|EmptyGroup`
- `boolean_op: BooleanOp` — `Add|Subtract|Intersect`
- `csg_target_id: Option<String>` — sadece belirli bir hedefe etki
- `parent_part_id: Option<String>` — parent-child hiyerarşi
- `pos_expr / scale_expr / rot_expr: [String; 3]` — `fasteval` ile değerlendirilen parametrik ifadeler; örn: `"width / 2"`
- `array_count_expr / array_offset_expr` — Array Modifier (tekrarlı geometri)
- `modifiers: Vec<ModifierType>` — `Shear([f32;3]) | Bend(f32) | Taper(f32) | Noise([f32;2])`
- `collider_type: ColliderType` — `None|Box|Sphere|Capsule|Mesh`
- `lod_hide_distance: f32` — mesafe aşılırsa parça çizilmez (LOD)
- `mirror_x/y/z: bool` — eksen aynalama
- `faces: HashMap<String, FaceMaterial>` — yüzey başına materyal ataması

### 7.3 Seviye Yerleşim Modeli (`level.rs`)

```rust
pub struct GameLevel {
    pub id:             String,
    pub instances:      Vec<ObjectInstance>,
    pub ambient_light:  [u8; 3],   // default: [30, 30, 40]
    pub gravity:        [f32; 3],  // default: [0.0, -9.81, 0.0]
    pub skybox_texture: String,
}

pub struct ObjectInstance {
    pub instance_id:          String,
    pub object_id:            String,             // referans alınan GameObject ID
    pub world_pos:            [f32; 3],
    pub world_rot:            [f32; 3],
    pub world_scale:          [f32; 3],
    pub param_overrides:      HashMap<String, f32>, // instance-specific değişkenler
    pub health_override:      Option<f32>,
    pub mass_override:        Option<f32>,
    pub ai_behavior_override: Option<AiBehavior>,
    pub light_intensity_override: Option<f32>,
    pub light_color_override: Option<[u8; 3]>,
}
```

### 7.4 UI Element Modeli (`element.rs`)

```rust
pub struct UiElement {
    pub id:             String,
    pub z_index:        i32,
    pub anchor:         Anchor,         // Center, TopLeft, TopRight vb.
    pub pos_x:          f32,
    pub pos_y:          f32,
    pub width:          f32,            // default: 25.0
    pub height:         f32,            // default: 15.0
    pub parent_id:      Option<String>,
    pub children:       Vec<String>,    // child ID listesi
    pub action_binding: String,         // "START_GAME", "QUIT" vb.
    pub layers:         Vec<AxiomLayer>,
}
```

Varsayılan `UiElement` üç hazır katmanla gelir: `Fill` (arka plan), `Border` (kenarlık),
`Text` (içerik). Varsayılan renk `[0, 255, 150]` (parlak yeşil) ön plan, `[20, 20, 25]`
koyu arka plandır.

### 7.5 Kenarlık Sistemi (`border.rs`)

`BorderTemplate` sekiz ayrı `BorderPiece` içerir: dört köşe + dört kenar.
Her `BorderPiece` bağımsız desen (pattern), renk override ve offset değerlerine sahiptir.

Yerleşik şablonlar:

| Şablon | Köşeler | Kenarlar |
|---|---|---|
| `default()` | `╔╗╚╝` | `═ ║` |
| `round()` | `╭╮╰╯` | `─ │` |
| `brick()` | `┌┐└┘` | `── │` |
| `stone()` | `▛▜▙▟` | `▀▀ ▄▄ █` |
| `solid()` | `████` | `████` |
| `interwoven()` | `++++` | `=- :.` |

---

## Bölüm 8.0 — Derleme ve Çalıştırma

### 8.1 Geliştirici Ortamı

```bash
# Studio (Editör) başlatma
cd studio && cargo run --release

# Runtime (Oyun İstemcisi) başlatma
cd runtime && cargo run --release
```

`--release` bayrağı LLVM optimizasyonlarını (inlining, dead code elimination,
loop unrolling) etkinleştirir. GPU pipeline ve CSG hesaplamaları debug modda
10-50x daha yavaş çalışabilir; bu nedenle her zaman release derleme önerilir.

### 8.2 Yeni Sahne Dosyası Formatı

```json
{
  "room": {
    "id": "room_01",
    "name": "Test Odası",
    "dimensions": { "width": 20, "length": 20, "height": 5 }
  },
  "lighting": { "ambient": 0.3, "player_fov": 60.0, "flashlight_range": 8.0 },
  "player_start": { "x": 10.0, "y": 10.0, "angle": 0.0 },
  "objects": [
    { "type": "enemy", "x": 5.0, "y": 5.0, "z": 0.0 }
  ]
}
```

Sahne dosyaları `data/` dizinine `.json` uzantısıyla yerleştirilmeli ve runtime
menüsünde ilgili isimle çağrılmalıdır.

---
