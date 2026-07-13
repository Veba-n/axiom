# Axiom Engine Mimari ve Modül Spesifikasyon Dokümantasyonu (v0.1)

Bu doküman, Axiom Engine'in dosya hiyerarşisini, modül bağımlılıklarını, bellek düzenlerini, WGPU render boru hatlarını (pipelines) ve CSG (Constructive Solid Geometry) matematik algoritmalarını kaynak kod (source code) seviyesinde incelemektedir. 

Motor, `runtime` (bağımsız oyun çalıştırma ortamı) ve `studio` (egui tabanlı geliştirme editörü) olmak üzere iki monolitik çalışma alanına (workspace) bölünmüştür.

---

## 1. Dizin Hiyerarşisi ve Modül Ağacı

```text
axiom/
├── Cargo.toml                   # Workspace kök yapılandırması
├── runtime/                     # Çalışma Zamanı (Oyun İstemcisi) Modülü
│   ├── Cargo.toml               # Runtime bağımlılıkları (winit, wgpu, serde)
│   └── src/
│       ├── main.rs              # İstemci başlangıç (entry) noktası
│       ├── engine.rs            # WGPU EventLoop ve donanım başlatma sekansları
│       └── data_parser.rs       # Scene/JSON statik bellek deserialization işlemleri
├── studio/                      # Geliştirici Editörü Modülü
│   ├── Cargo.toml               # Studio bağımlılıkları (eframe, egui, bytemuck)
│   └── src/
│       ├── main.rs / lib.rs     # Uygulama başlatıcısı ve `AxiomStudio` state'i
│       ├── app.rs               # GUI döngüsü (Update frame) yöneticisi
│       ├── core/                # Çekirdek girdiler ve durum (state) makinesi
│       │   ├── events.rs        # Ham I/O event dinleyicileri
│       │   ├── interaction.rs   # Raycasting, Bounding Box kesişim hesaplamaları
│       │   └── types.rs         # Ortak veri tipleri (Enumlar ve structlar)
│       ├── data/                # Serileştirilebilir (Serializable) Bellek Modelleri
│       │   ├── border.rs        # BorderTemplate ve topoloji verileri
│       │   ├── element.rs       # UiElement base struct
│       │   ├── layer.rs         # Render Z-Index katman (Layer) durumu
│       │   ├── level.rs         # Sahne ağacı (Scene Graph) tepe nodu
│       │   ├── object.rs        # Tekil Entity/Obje modelleri
│       │   ├── scene.rs         # Sahne içi hiyerarşi
│       │   ├── settings.rs      # Studio yapılandırma (config) verileri
│       │   ├── texture.rs       # UV ve materyal referans verileri
│       │   └── texture_presets.rs # Ön tanımlı statik kaplama profilleri
│       ├── render/              # Düşük Seviye WGPU ve Çizim İşlemleri
│       │   ├── canvas.rs        # Egui üzerindeki ana render pass köprüsü
│       │   ├── csg.rs           # Matematiksel katı cisim ve BSP bölme algoritmaları
│       │   ├── gpu.rs           # WGPU custom pipeline ve bind group tanımları
│       │   ├── object_viewport.rs # Tekil model izleme kamerasının projeksiyonları
│       │   ├── texture_composer.rs # CPU tabanlı texture blending algoritmaları
│       │   └── shaders/
│       │       └── main.wgsl    # Paralel işlemci GPU Vertex/Fragment Gölgelendiricisi
│       └── ui/                  # Arayüz Panelleri ve Çizim Komutları
│           ├── explorer.rs      # Scene Graph ağaç dizinleyicisi (TreeView)
│           ├── inspector.rs     # Obje özellik (Property) mutatörü
│           ├── level_editor.rs  # Ana seviye tasarım ızgarası (Grid) ve koordinat matrisi
│           ├── object_editor.rs # CSG yüzey düzenleyicisi
│           ├── object_editor_cache.rs # CPU darboğazını (bottleneck) engellemek için yüzey önbellekleme
│           ├── settings_modal.rs # Konfigürasyon I/O paneli
│           ├── texture_editor.rs # Materyal ve renk uzayı manipülatörü
│           ├── toolbar.rs       # Yüksek seviye komut (Save/Load/Play) tetikleyicileri
│           └── widgets.rs       # Egui Custom Widget (Macro) implementasyonları
└── analizler/                   # Mimarinin ve gelecek eklentilerin yazılı olduğu teknik notlar
```

---

## 2. Modül Spesifikasyonları

### 2.1 Runtime Çekirdeği (`runtime/src/engine.rs`)
`winit` tabanlı işletim sistemi seviyesi pencere döngüsünü (EventLoop) uygular. 
* **Başlatma Fazı**: `wgpu::Instance` yapısını `wgpu::Backends::all()` ile başlatır, `RequestAdapterOptions` içerisinden `HighPerformance` profili talep edilir.
* **Swapchain Konfigürasyonu**: Hardcoded olarak `1280x720` çözünürlüğünde çalışır.
* **Render Frame**: Her bir donanım karesinde (frame), `CommandEncoder` `LoadOp::Clear` talimatıyla yüzeyi `[0.1, 0.1, 0.1, 1.0]` lineer RGBA değeri ile temizler. Veriler belleğe `StoreOp::Store` üzerinden yazılır.

### 2.2 Studio GPU Pipeline (`studio/src/render/gpu.rs`)
`egui` render hattını (pipeline) delerek doğrudan ekran kartına veri (Draw Call) yollayan `Custom3dCallback` (`egui_wgpu::CallbackTrait`) sistemini barındırır.
* **Bellek Düzeni (Struct Layout)**: `GpuVertex` `#[repr(C)]` formatındadır. Toplam köşe (vertex) adımı 36 byte'tır (`[f32; 3]` Position (12 byte), `[f32; 2]` UV (8 byte), `[f32; 4]` Color (16 byte)).
* **Tampon Bellek (Buffer Management)**: `SolidBuffer` ve `LineBuffer` kapasite ihtiyacına göre VRAM üzerinde 2'nin katları şeklinde dinamik olarak büyür (Min: 1024 vertex). Köşeler GPU'ya `queue.write_buffer` kullanılarak sıfır kopyalama (zero-copy) yaklaşımıyla yollanır.
* **Shader Yapılandırması (`Custom3dPipeline`)**:
  * **Solid Pipeline**: `TriangleList` topolojisi, `Depth24Plus` derinlik donanımı (write: true, compare: `LessEqual`), Culling kapalı (Back-face render serbest), Alpha Blending aktif.
  * **Wireframe Pipeline**: `LineList` topolojisi. Derinlik (Depth) yazımı kapalıdır ancak `LessEqual` karşılaştırma aktiftir. CSG objeleriyle çizgilerin `Z-fighting` yapmaması için donanımsal bazda `constant: -2` ve `slope_scale: -1.0` önyargı (Depth Bias) uygulanır.

### 2.3 Constructive Solid Geometry (`studio/src/render/csg.rs`)
Motor, BSP (Binary Space Partitioning) yaklaşımına benzeyen özel bir konveks bölme (Convex Splitting) matematik motoruna sahiptir. Gerçek zamanlı katı obje kesişimini (Intersection) ve çıkarılmasını (Subtraction) yönetir.
* **Düzlem Denklemi (`make_plane`)**: 3 noktadan (v0, v1, v2) kros (cross) vektör çarpımı yapılarak normalize edilmiş yüzey normali (`[nx, ny, nz]`) ve uzaklık (`d`) hesaplanır.
* **Tolerans Eşiği (`EPSILON`)**: Float (f32) kayıp hatalarını önlemek için kesin tolerans `1e-4` olarak sabitlenmiştir.
* **Poligon Ayrıştırma (`split_poly`)**: Yüzeydeki noktalar kesme düzleminin normaline göre (+1) Front, (-1) Back ve (0) Coplanar olarak işaretlenir. 
  * Eşdüzlemlilikte (Coplanar Intersection), objenin kendi normali ile düzlemin normali nokta çarpımına (dot product) sokulur. Eğer sonuç pozitifse ve obje delik objesiyse yüzey imha edilir.
  * Kesişim noktalarında `t = dist1 / (dist1 - dist2)` formülüyle interpolasyon köşeleri üretilir.
* **Nokta Birleştirici (Clean Poly)**: Sonsuz dikenli (spike) noktaları ve NaN çökmelerini engellemek için, kesim sonrasında aralarındaki uzaklık karesi `1e-6`'dan küçük olan köşeler otomatik olarak birleştirilir.

### 2.4 Hiyerarşik Veri İşleme (`studio/src/data/`)
JSON serileştirme işlemleri için `serde` paketleri (macros) kullanılır. `scene.rs` ve `level.rs` veri yapıları, ebeveyn-çocuk (parent-child) ağaç ilişkisini (Tree) simüle eden UUID (String) bazlı kimlik indeksleme sistemleri kullanır. İşlemler render hattında derinlik (Z-index) sırasına sokulmadan önce bu veriler parse edilir.

---

## 3. Derleme (Build) Hedefleri ve Bağımlılıklar
- `eframe / egui`: GUI sistemi ve State makinesi.
- `wgpu / winit`: Cross-platform donanım iletişimi, shader derlemesi, pencere döngüsü.
- `bytemuck`: `GpuVertex` verilerinin (Pod, Zeroable) memory-safe bir şekilde ham byte dizilerine (byte-cast) çevrilerek VRAM'e yazılması.
- `serde / serde_json`: Bellekteki objelerin sabit disk JSON formuna çevrilmesi (Serialization/Deserialization).

Motorun maksimum kapasite (frame-time, GPU overhead azaltımı) verebilmesi adına tüm geliştirme ve test evrelerinin optimize LLVM bitcode derlemesiyle (`cargo run --release`) başlatılması sistem mimarisinin bir gerekliliğidir.
