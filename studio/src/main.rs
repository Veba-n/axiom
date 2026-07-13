pub mod core;
pub mod data;
pub mod render;
pub mod ui;
pub mod app;

use crate::app::AxiomStudio;
use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    // Linux üzerinde Wayland kaynaklı fare (pointer) çökmesini engellemek için X11'i zorluyoruz
    std::env::set_var("WINIT_UNIX_BACKEND", "x11");

    let mut power_pref = eframe::egui_wgpu::wgpu::PowerPreference::HighPerformance;
    let mut pref_str = "HighPerformance".to_string();
    if let Ok(pref) = std::fs::read_to_string("gpu_preference.txt") {
        if pref.trim() == "LowPower" {
            power_pref = eframe::egui_wgpu::wgpu::PowerPreference::LowPower;
            pref_str = "LowPower".to_string();
        }
    }
    
    let wgpu_options = eframe::egui_wgpu::WgpuConfiguration {
        power_preference: power_pref,
        // Kullanıcının en yüksek FPS değerini stabil şekilde aldığı AutoNoVsync moduna geri dönüyoruz.
        present_mode: eframe::egui_wgpu::wgpu::PresentMode::AutoNoVsync,
        ..Default::default()
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 900.0])
            .with_title("Axiom UI Designer - Advanced Text Rendering Engine"),
        vsync: false,
        depth_buffer: 24, // HİBRİT RENDER & KESİŞEN POLİGONLAR İÇİN GPU DEPTH BUFFER AKTİF!
        wgpu_options,
        ..Default::default()
    };
    eframe::run_native(
        "Axiom Studio",
        options,
        Box::new(move |cc| {
            let mut fonts = egui::FontDefinitions::default();
            if let Ok(font_data) = std::fs::read("FiraCode.ttf") {
                fonts.font_data.insert(
                    "FiraCode".to_owned(),
                    egui::FontData::from_owned(font_data),
                );
                fonts.families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "FiraCode".to_owned());
            }
            cc.egui_ctx.set_fonts(fonts);
            
            let mut gpu_name = "Bilinmeyen GPU (Yazılım veya Desteklenmiyor)".to_string();
            // WGPU (Ekran Kartı) Pipeline Başlatıcısı (Derleyici)
            if let Some(render_state) = &cc.wgpu_render_state {
                let info = render_state.adapter.get_info();
                gpu_name = format!("{} ({:?})", info.name, info.backend);
                eprintln!("\n=== AXIOM STUDIO GPU BAŞLATILDI ===");
                eprintln!("Kullanılan GPU: {}", gpu_name);
                eprintln!("=====================================\n");
                
                let format = render_state.target_format;
                let pipeline = crate::render::gpu::Custom3dPipeline::new(&render_state.device, format);
                // Pipeline'ı her frame derlememek için Egui'nin kalıcı kaynaklarına kaydediyoruz
                render_state.renderer.write().callback_resources.insert(pipeline);
            } else {
                eprintln!("\n!!! KRİTİK UYARI: WGPU BAŞLATILAMADI, OpenGL/Yazılım Moduna Düşüldü !!!\n");
            }
            
            let mut app = Box::<AxiomStudio>::default();
            app.active_gpu_name = gpu_name;
            app.gpu_preference = pref_str;
            app
        }),
    )
}