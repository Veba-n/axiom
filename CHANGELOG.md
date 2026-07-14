# Axiom Engine Güncelleme Notları (Changelog)

## v0.1.1 - GPU Render ve CSG İyileştirmeleri (14 Temmuz 2026)

Bu güncelleme, Axiom Engine'in WGPU tabanlı render motorunda ve CSG (Katı Modelleme) sistemlerinde kritik mantıksal düzeltmeler içermektedir.

### 🐛 Hata Düzeltmeleri (Bug Fixes)
- **CSG İç Duvar Dokulandırma (Hole Texturing):** Delik açmak (Subtract) amacıyla kullanılan objelerin, deldikleri ana objenin içinde oluşturduğu yeni "iç duvar" yüzeyleri artık görünmez delik objesinin değil, **deliğin açıldığı katı objenin dokusunu (texture) ve özelliklerini miras alıyor.** Bu sayede, "tuğla bir duvara delik açıldığında içinin de tuğla görünmesi" gibi mantıksal beklentiler tam olarak karşılandı.
- **Kafa Karıştıran "?" Simgesi Kaldırıldı:** 3D viewport üzerinde, dokusu olmayan veya silinmiş dokulara sahip yüzeylerin ortasında beliren ve geliştiriciyi rahatsız eden devasa sarı soru işareti (`?`) kaldırıldı. Sistem artık doğrudan sağlam bir fallback (varsayılan) renkle çizim yapmaya devam ediyor.
- **Render Z-Buffer (Pitch Black) Düzeltildi:** Dokuların saydamlık (alpha) değerleri artık WGSL Shader içinde doğru `mix` (harmanlama) fonksiyonuyla işleniyor. Zemin renklerinin tamamen siyah bir tabaka tarafından yutulması sorunu ortadan kaldırıldı.
- **Dinamik UV Tiling (Boyutla Orantılı Döşeme):** Yüzeylerin UV koordinatları, `w0`, `vec_x`, `vec_y` vektörleri eşliğinde 3D dünya uzayından dinamik olarak çıkarılacak şekilde yeniden bağlandı. CSG yüzey bölünmelerinde desenlerin kayması engellendi.

### ⚡ Performans & Mimari
- Kullanılmayan `mut` uyarıları giderilerek derleme süreci tertemiz (warning-free) hale getirildi.
- GPU'ya aktarılan boş fallback dokusu artık tamamen şeffaf (`TRANSPARENT`) olacak şekilde ayarlandı.
