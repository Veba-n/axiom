mod data_parser;
mod engine;

use dialoguer::{theme::ColorfulTheme, Select};
use console::{style, Term};
use crossterm::{
    execute,
    terminal::{Clear, ClearType},
};
use std::io::stdout;

const ASCII_LOGO: &str = r#"
   ___             _
  / _ \ _  ____  (_)_________  ____ _____ _____ ____
 / ___/| |/_/ / / / ___/ __ \/ __ `/ __ `/ __ `/ _ \
/ /    >  </ /_/ / /__/ /_/ / /_/ / /_/ / /_/ /  __/
/_/    /_/|_|\__,_/\___/\____/\__, /\__, /\__,_/\___/
                             /____//____//____/
"#;

fn clear_console() {
    let mut out = stdout();
    execute!(out, Clear(ClearType::All)).unwrap();
    // Move cursor to top left
    execute!(out, crossterm::cursor::MoveTo(0, 0)).unwrap();
}

fn main() {
    let term = Term::stdout();
    let theme = ColorfulTheme::default();

    loop {
        clear_console();
        println!("{}", style(ASCII_LOGO).magenta().bold());
        println!("{}", style("AXIOM ENGINE - Voxel Raycasting Motoru").cyan());
        println!("{}\n", style("=========================================").black().bright());

        let selections = &[
            "Oyunu Başlat (Oda Verisini Yükle)",
            "Geliştirici Modu (Debug & Render Ayarları)",
            "Çıkış",
        ];

        let selection = Select::with_theme(&theme)
            .with_prompt("Lütfen bir işlem seçin")
            .default(0)
            .items(&selections[..])
            .interact_on_opt(&term)
            .unwrap();

        match selection {
            Some(0) => {
                println!("{}", style("\n[+] Oda verisi yükleniyor...").green());
                let room_data_path = "../data/room_01.json"; 
                match data_parser::load_room_data(room_data_path) {
                    Ok(room_data) => {
                        println!("{} {}", style("[OK]").green().bold(), room_data.room.name);
                        println!("{}", style("[+] WGPU Oyun Motoru başlatılıyor...\n").cyan());
                        
                        pollster::block_on(engine::run(&room_data.room.name));
                    }
                    Err(e) => {
                        println!("{} {}\n", style("[HATA]").red().bold(), e);
                        term.read_line().unwrap();
                    }
                }
            }
            Some(1) => {
                clear_console();
                println!("{}", style("Geliştirici Modu (Henüz uygulanmadı)").yellow());
                term.read_line().unwrap();
            }
            Some(2) | None => {
                println!("{}", style("\nAxiom Engine'den çıkılıyor...").blue());
                break;
            }
            _ => unreachable!(),
        }
    }
}