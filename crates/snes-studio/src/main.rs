mod app;
mod omni_mixer_ui;

use app::SnesStudioApp;
use snes_core::{
    audio::scan_brr_samples,
    demo::generate_demo_rom_pair,
    graphics::{scan_candidate_palettes, scan_graphics_regions},
    rom::SnesRom,
};

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "--generate-demo" => {
                let out_dir = args.get(2).map(|s| s.as_str()).unwrap_or(".");
                let (rom_a, rom_b) = generate_demo_rom_pair();
                let path_a = std::path::Path::new(out_dir).join(&rom_a.name);
                let path_b = std::path::Path::new(out_dir).join(&rom_b.name);
                std::fs::write(&path_a, rom_a.to_sfc_bytes()).expect("Failed to write demo ROM A");
                std::fs::write(&path_b, rom_b.to_sfc_bytes()).expect("Failed to write demo ROM B");
                println!(
                    "ROMs de démonstration générées :\n  - {}\n  - {}",
                    path_a.display(),
                    path_b.display()
                );
                return Ok(());
            }
            "--inspect" => {
                for path in &args[2..] {
                    let bytes = std::fs::read(path).expect("Impossible de lire le fichier ROM");
                    let rom = SnesRom::from_bytes(path.clone(), &bytes).expect("ROM SNES invalide");
                    let gfx = scan_graphics_regions(&rom.data);
                    let pals = scan_candidate_palettes(&rom.data, 20);
                    let brr = scan_brr_samples(&rom.data, 8);
                    println!("=== {} ===", rom.name);
                    println!("  Titre interne : {}", rom.header.title);
                    println!("  Mapping       : {}", rom.header.mapping.as_str());
                    println!("  Chipset       : {}", rom.header.chipset_description);
                    println!(
                        "  Checksum      : 0x{:04X} (Valide: {})",
                        rom.header.checksum,
                        rom.header.is_checksum_valid()
                    );
                    println!("  CRC32         : 0x{:08X}", rom.crc32);
                    println!(
                        "  Assets trouvés: {} blocs graphiques, {} palettes BGR555, {} samples audio BRR",
                        gfx.len(),
                        pals.len(),
                        brr.len()
                    );
                }
                return Ok(());
            }
            "--help" | "-h" => {
                println!("SNES Studio - Décompilateur & Asset Swapper de ROMs SNES (.sfc / .smc)");
                println!("Usage :");
                println!("  snes-studio                        Lance l'interface graphique multiplateforme (egui/eframe)");
                println!("  snes-studio --generate-demo [dir]  Génère 2 ROMs .sfc de démonstration");
                println!("  snes-studio --inspect <rom.sfc>    Inspecte une ou plusieurs ROMs en ligne de commande");
                return Ok(());
            }
            _ => {}
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 780.0])
            .with_min_inner_size([1024.0, 600.0])
            .with_title("SNES Omni-Mixer Studio — Outil Multi-Jeux Universel SNES"),
        ..Default::default()
    };

    eframe::run_native(
        "SNES Omni-Mixer Studio",
        native_options,
        Box::new(|cc| Ok(Box::new(SnesStudioApp::new(cc)))),
    )
}
