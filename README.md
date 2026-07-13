# AXIOM ENGINE - TEKNİK MİMARİ VE GELİŞTİRME DOKÜMANTASYONU (v0.1.0)

## 1. Vizyon ve Felsefe
Axiom Engine; 3 boyutlu, voxel tabanlı ve ışın izleme (Raycasting/Raymarching) matematiği ile çalışan bir terminal/konsol RPG oyun motorudur. 
Geleneksel oyun motorlarının aksine, CPU üzerindeki spagetti kodlar yerine GPU (wgpu/WebGPU) kullanılarak matematiksel formüllerin ASCII/Piksel dokulara dönüştürüldüğü iki parçalı bir mimariye sahiptir.

## 2. Sistem Mimarisi (İki Uygulamalı Yapı)

Axiom Engine, profesyonel oyun sektöründeki standartlara uygun olarak iki ana modülden oluşur:

### A. Axiom Studio (Editör)
* **Görev:** Dünyayı, odaları, voxel nesneleri (masa, kasa, duvar) ve ışıklandırmaları (fener kapsamı vb.) tasarlamak.
* **Teknoloji:** TypeScript, JSON, Web ortamı (veya Electron/Tauri).
* **Çıktı:** Oyun motorunun okuyacağı saf JSON yapılandırma dosyaları (`.axiom` veya `.json` uzantılı dünya verileri).

### B. Axiom Runtime (Çekirdek Motor)
* **Görev:** Studio'dan gelen veriyi okuyup, GPU üzerinde 720p çözünürlükte 60 FPS ile render eden ve oyuncu girdilerini (W-A-S-D, Envanter, Seçimler) işleyen oyunun kendisi.
* **Teknoloji:** Rust, `wgpu` (WebGPU API), GLSL/WGSL (Shader dili).
* **Çıktı:** Matematiksel olarak hesaplanmış, gölgelendirilmiş ASCII 3D dünyası.

---

## 3. Başlangıç Klasör Hiyerarşisi

Projeyi Windows PowerShell ortamında başlatmak için Workspace (Çalışma Alanı) yapısı kullanılacaktır.

```powershell
# PowerShell üzerinden ana klasörü ve alt projeleri oluşturmak için:
New-Item -ItemType Directory -Name "AxiomEngine"
cd AxiomEngine
cargo new runtime --bin
mkdir studio
```

**Klasör Ağacı:**

```text
AxiomEngine/
├── studio/                 # TypeScript Editör Uygulaması
│   ├── package.json
│   ├── src/
│   │   ├── ui/             # Menüler ve 3D Grid Arayüzü
│   │   └── exporter.ts     # JSON çıktı alma sistemi
│   └── public/
├── runtime/                # Rust Oyun Motoru
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs         # Giriş noktası ve CLI menüsü
│   │   ├── engine/         # wgpu render mekanikleri
│   │   └── data_parser.rs  # JSON okuyucu
│   └── shaders/
│       └── raycaster.wgsl  # GPU matematik formülleri
└── data/                   # İki uygulamanın konuştuğu ortak veri
    ├── materials.json
    └── room_01.json
```

---

## 4. Ortak Veri Şeması (Data Schema)

Motorun ve editörün birbiriyle iletişim kurduğu veri standartlarıdır. Başlangıç aşamasında sistem bu JSON dosyasını okuyarak ekranı çizer.

**`data/room_01.json`:**

```json
{
  "room": {
    "id": "start_dungeon_01",
    "name": "Karanlık Mahzen",
    "dimensions": { "width": 16, "length": 16, "height": 8 }
  },
  "lighting": {
    "ambient": 0.1,
    "player_fov": 60.0,
    "flashlight_range": 5.0
  },
  "player_start": { "x": 8.0, "y": 8.0, "angle": 0.0 },
  "objects": [
    { "type": "wooden_chest", "x": 4.0, "y": 2.0, "z": 0.0 }
  ]
}
```

---

## 5. Adım Adım Geliştirme Planı (Faz 1)

### Aşama 1.1: Runtime Terminal Menüleri (İlk Kodlanacak Kısım)

Rust `runtime` uygulaması PowerShell'den çalıştırıldığında doğrudan 3D ekranı açmaz. Önce bir "Yönetim/Giriş Menüsü" (CLI) sunar.

**Tasarlanacak Akış:**

1. Ekran temizlenir (`Clear-Host` mantığı).
2. Ekrana ASCII Axiom Logosu basılır.
3. Kullanıcıya seçenekler sunulur:
* `[1] Oyunu Başlat (Oda Verisini Yükle)`
* `[2] Geliştirici Modu (Debug & Render Ayarları)`
* `[3] Çıkış`

4. Kullanıcı 1'e basarsa `data/room_01.json` dosyası parse edilir ve `wgpu` penceresi tetiklenir.

### Aşama 1.2: Studio Menüleri (Editör Arayüzü)

TypeScript tarafında geliştirilecek ilk ekranlar:

1. **Oluşturma Ekranı (New Project):** Oda genişliği, yüksekliği ve ortam ışığı değerlerinin girildiği basit formlar.
2. **Voxel Grid Paneli:** Sol tarafta malzemelerin (Halı, Tuğla, Paslı Metal), sağ tarafta 2D bir ızgaranın olduğu, tıklayarak duvarların çizilebildiği ekran.
3. **Export Paneli:** Yapılan tasarımı JSON olarak `AxiomEngine/data/` klasörüne kaydeden fonksiyon.

### Aşama 1.3: İlk GPU Çıktısı (Raycasting V0)

Editörden alınan JSON verisi Rust tarafına geçer. `wgpu` boş bir pencere açar ve Shader kodu, sadece odanın dış sınırlarını (16x16) matematiksel mesafe ölçümüyle ekrana gri tonlamalı bloklar halinde çizer.

---

## 6. Teknik Standartlar ve Kurallar

* **Paralel İşleme:** Ağır hesaplamaların tamamı (Raycasting, FOV, Collision) Rust içinde GPU'ya (WGSL) devredilecektir.
* **Modülerlik:** Hiçbir eşya hard-coded (koda gömülü) olmayacaktır. Her şey TypeScript editöründen JSON olarak beslenecektir.
* **Performans Hedefi:** CPU yükü %10'u geçmeyecek, render işlemlerinin %90'ı wgpu üzerinden donanımsal hızlandırma ile çözülecektir.