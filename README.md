# Axiom Engine (Beta 0.1)

Axiom Engine, Rust dili ile geliştirilmiş yüksek performanslı, iki ana bileşenden (Studio ve Runtime) oluşan yeni nesil bir oyun motoru ve sahne düzenleyicisidir. Gelişmiş GPU hızlandırması, Özel CSG (Constructive Solid Geometry) altyapısı ve WGPU gücüyle donatılmıştır.

## 🚀 Teknolojiler ve Altyapı
- **Dil:** Rust (Güvenli, hızlı ve bellek dostu)
- **Grafik API:** `WGPU` (Vulkan, Metal, DX12 gibi modern grafik API'lerine çapraz platform (cross-platform) düşük seviyeli erişim)
- **Arayüz (GUI):** `egui` ve `eframe` (Geliştirici ortamı olan Studio'nun anında tepki veren panelleri için)
- **Veri Yönetimi:** `serde` (Oyun dünyası verilerinin JSON olarak işlenip kaydedilmesi)
- **Matematik & Geometri:** Özel CSG (Katı Cisim Geometrisi) algoritmaları, GPU tabanlı Painter's Algorithm (Ressam Algoritması) derinlik sıralaması.

---

## 📂 Proje Mimarisi ve Dosya Yapısı

Proje, temelde **Runtime** (Oyunun son kullanıcıda çalışacağı grafik motoru) ve **Studio** (Oyunun yapıldığı görsel editör) olmak üzere ikiye ayrılır.

### 1. `runtime/` (Çalışma Zamanı Motoru)
Oyunun derlenmiş son halinin çalıştığı, tamamen WGPU odaklı, pencere yönetimini ve ekran çizimini yapan saf motor kısmıdır.
* **`src/main.rs`**: Runtime'ın giriş noktasıdır. Konsol ayarlarını yapar, `winit` ile pencereyi oluşturur ve oyun döngüsünü (Event Loop) başlatır.
* **`src/engine.rs`**: WGPU kullanarak grafik kartı ile iletişime geçer. Ekranın temizlenmesi (clear color), swapchain yönetimi, adapter ve device işlemlerini yürüten ana grafik motorudur.
* **`src/data_parser.rs`**: Studio'da üretilen `.axiom` veya `.json` uzantılı harita/oyun dosyalarını okuyarak motorun anlayacağı Rust yapılarına (Struct) çevirir.

### 2. `studio/` (Oyun Geliştirme Editörü)
Oyunun tasarlandığı, haritaların yapıldığı, objelerin düzenlendiği zengin bir masaüstü uygulamasıdır. `egui` kullanır.

#### `studio/src/` Klasörleri ve Dosyaları:
* **`main.rs` & `lib.rs`**: Editör uygulamasının giriş noktasıdır. Uygulama ayarlarını yapılandırır ve başlatır.
* **`app.rs`**: Tüm editörün ana durum (state) yöneticisidir. Panelleri, sekmeleri ve genel veri akışını barındırır.

**A. `core/` (Çekirdek İşlemler):**
* **`events.rs`**: Klavyeden gelen tuş basımları, fare hareketleri gibi kullanıcı girdilerini yakalayan sistemdir.
* **`interaction.rs`**: Objeleri seçme, sürükleme, boyutlandırma gibi etkileşim mantıklarını yönetir.
* **`types.rs`**: Editörde genel olarak kullanılan temel veri tiplerini barındırır.
* **`mod.rs`**: Core klasöründeki dosyaları dışa aktarır.

**B. `data/` (Veri Yapıları):**
* **`level.rs` & `scene.rs`**: Oyun dünyasının hiyerarşik yapısını (Bölümler ve sahneler) barındıran veri modelleridir.
* **`layer.rs`**: Sahnelerdeki katman (Z-index veya mantıksal gruplama) sistemini tanımlar.
* **`object.rs` & `element.rs`**: Oyun dünyasındaki tekil varlıkları (objeler, kapılar, duvarlar vb.) ve onlara ait bileşenleri tanımlar.
* **`texture.rs` & `texture_presets.rs`**: Kaplama (Materyal) verilerini, renkleri ve ön tanımlı doku ayarlarını barındırır.
* **`border.rs`**: Arayüz ve objelerdeki sınır/kenarlık hesaplamalarının verisidir.
* **`settings.rs`**: Studio'nun kullanıcı ayarlarını (karanlık mod, grid boyutu vb.) tutar.

**C. `render/` (Çizim ve Grafik İşleme):**
* **`gpu.rs`**: Editör içerisindeki en kritik dosyalardan biridir. İşlemciden (CPU) bağımsız olarak WGPU üzerinden doğrudan ekran kartına çizim emirlerini yollayan yüksek performanslı render boru hattıdır.
* **`shaders/main.wgsl`**: GPU'da çalışan shader (gölgelendirici) kodudur. Objelerin ekrana nasıl yansıtılacağını matematiksel olarak belirler.
* **`csg.rs`**: (Constructive Solid Geometry) Objelerin birleşimi, kesişimi veya birbirinden çıkarılması gibi karmaşık 3D/2D katı cisim operasyonlarını yapan matematiksel motor.
* **`canvas.rs`**: Egui içerisinde oyun dünyasının çizildiği ana tuval ekranıdır.
* **`object_viewport.rs`**: Tek bir objenin detaylı incelendiği 3 boyutlu/2 boyutlu küçük izleme penceresinin çizim kodudur.
* **`texture_composer.rs`**: Kaplamaların, renklerin ve UV haritalamalarının bir araya getirilip GPU'ya hazır hale getirildiği yerdir.

**D. `ui/` (Kullanıcı Arayüzü - Paneller):**
* **`explorer.rs`**: Sol taraftaki proje dosyalarını ve sahne hiyerarşisini gösteren dosya gezginidir.
* **`inspector.rs`**: Sağ taraftaki özellikler panelidir. Seçilen objenin boyutunu, rengini, koordinatlarını değiştirmeyi sağlar.
* **`level_editor.rs`**: Ana harita tasarım ekranının arayüzüdür.
* **`object_editor.rs` & `object_editor_cache.rs`**: Bir objenin (örneğin bir duvarın veya karakterin) içine girip onu CSG ve poligon seviyesinde düzenlediğimiz derin editördür. İşlem yükünü azaltmak için önbellekleme (cache) kullanır.
* **`texture_editor.rs`**: Kaplama ve materyal oluşturma/düzenleme menüsüdür.
* **`settings_modal.rs`**: Editör ayarları için açılan pencere (Pop-up).
* **`toolbar.rs`**: Üst kısımdaki kaydetme, oynatma ve araç (seçim, çizim) çubuğudur.
* **`widgets.rs`**: Editör genelinde kullanılan özel yapım butonlar, slider'lar gibi küçük arayüz bileşenlerini içerir.

### 3. Diğer Klasörler
* **`data/`**: Deneme amaçlı oyun dosyalarını (örn: `room_01.json`) barındırır.
* **`analizler/`**: Projenin mimarisi, teknik planlamaları ve gelecekteki refaktör (kod iyileştirme) adımlarının yazılı olduğu Markdown (.md) belgeleridir.

---

## 🛠️ Nasıl Çalıştırılır?

Projeyi geliştirmeye devam etmek veya test etmek için ana dizinde terminalinizi açın:

**Studio (Oyun Editörü) için:**
```bash
cd studio
cargo run --release
```

**Runtime (Oyun Motoru) için:**
```bash
cd runtime
cargo run --release
```

*(Not: Performansın akıcı olması ve GPU hızlandırmasının tam verimli çalışması için her zaman `--release` bayrağı ile derlenmesi tavsiye edilir.)*
