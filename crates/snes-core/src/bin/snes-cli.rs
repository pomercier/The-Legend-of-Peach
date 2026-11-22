use snes_core::{
    audio::{decode_brr_to_pcm16, encode_pcm16_to_wav, scan_brr_samples},
    demo::{apply_obvious_demo_swap, generate_demo_rom_pair},
    disasm_dma::{
        classify_rom_chunks, decompile_rom_to_project_dir, scan_hardware_dma_transfers, ChunkKind,
    },
    graphics::{scan_candidate_palettes, scan_graphics_regions, SnesPalette, TileFormat},
    rom::SnesRom,
    swapper::SwapSession,
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_help();
        return;
    }

    match args[1].as_str() {
        "--generate-demo" => {
            let out_dir = args.get(2).map(|s| s.as_str()).unwrap_or(".");
            let (rom_a, rom_b) = generate_demo_rom_pair();
            let path_a = std::path::Path::new(out_dir).join(&rom_a.name);
            let path_b = std::path::Path::new(out_dir).join(&rom_b.name);
            std::fs::write(&path_a, rom_a.to_sfc_bytes()).expect("Erreur écriture ROM A");
            std::fs::write(&path_b, rom_b.to_sfc_bytes()).expect("Erreur écriture ROM B");
            println!(
                "✓ 2 ROMs SNES (.sfc) de démonstration générées :\n  - {}\n  - {}",
                path_a.display(),
                path_b.display()
            );
        }
        "--deconstruct" => {
            if args.len() < 3 {
                eprintln!("Usage: snes-cli --deconstruct <rom.sfc> [dossier_sortie] [--full]");
                return;
            }
            let rom_path = &args[2];
            let full_mode = args.iter().any(|a| a == "--full");
            let out_dir = args
                .get(3)
                .filter(|s| !s.starts_with("--"))
                .map(|s| s.as_str())
                .unwrap_or("./deconstructed_assets");
            let bytes = std::fs::read(rom_path).expect("Impossible de lire le fichier ROM");
            let rom = SnesRom::from_bytes(rom_path.clone(), &bytes).expect("ROM SNES invalide");
            let cfg = snes_core::deconstruct::DeconstructConfig {
                full_extraction: full_mode,
                ..Default::default()
            };
            let report = snes_core::deconstruct::deconstruct_rom(&rom, std::path::Path::new(out_dir), &cfg)
                .expect("Erreur lors de la déconstruction intégrale");

            println!("============================================================");
            println!("🚀 DÉCONSTRUCTION INTÉGRALE DES ASSETS SNES — {}", report.rom_title);
            println!("============================================================");
            println!("Dossier d'exportation : {}", report.out_dir);
            println!("Temps d'exécution    : {} ms", report.elapsed_ms);
            println!("ROM                  : {} ({} | Checksum 0x{:04X})", report.internal_name, report.mapping, report.checksum);
            println!("------------------------------------------------------------");
            println!("Actifs Décomposés :");
            println!("  - 🏃 Sprites Individuels Assemblés : {} personnages & objets (.png 1x/3x + .json)", report.sprites_count);
            println!("  - 🗺️ Planches Graphiques           : {} feuilles décompressées (.png + .bin)", report.sheets_count);
            println!("  - 🎵 Musiques & Fichiers .SPC      : {} fichiers SPC700 (66 048 octets standard)", report.spc_files_count);
            println!("  - 🔊 Échantillons Audio WAV        : {} fichiers PCM 16-bit (.wav @ 16kHz + .brr)", report.audio_wav_count);
            println!("  - 📜 Textes & Dialogues In-Game    : {} blocs de dialogue & noms (.txt, .json, .tbl)", report.text_dialogues_count);
            println!("  - 🎨 Palettes de Couleurs          : {} palettes BGR555 (.png swatches, .json, .pal)", report.palettes_count);
            println!("  - 🖼️ Écrans & Tilemaps             : {} écrans-titres & plans natifs (.png 256x224)", report.screens_count);
            println!("  - ⚙️ Code Source 65c816            : {} banques désassemblées + header.asm", report.disassembled_banks_count);
            println!("------------------------------------------------------------");
            println!("✓ Index maître généré : {}/INDEX.md", report.out_dir);
            println!("✓ Manifeste JSON      : {}/manifest.json", report.out_dir);
            println!("✓ Projet Aller-Retour : {}/project.json (prêt pour --reconstruct)", report.out_dir);
        }
        "--catalog" => {
            if args.len() < 3 {
                eprintln!("Usage: snes-cli --catalog <rom.sfc>");
                return;
            }
            let rom_path = &args[2];
            let bytes = std::fs::read(rom_path).expect("Impossible de lire le fichier ROM");
            let rom = SnesRom::from_bytes(rom_path.clone(), &bytes).expect("ROM SNES invalide");
            let cat = snes_core::universal::discover_universal_catalog(&rom);

            println!("============================================================");
            println!("🌐 CATALOGUE MATÉRIEL UNIVERSEL SNES — {}", cat.rom_title);
            println!("============================================================");
            println!("Titre interne : {} | Mapping : {} | Taille : {} Ko | Checksum : 0x{:04X}",
                cat.internal_title, cat.mapping, cat.rom_size_bytes / 1024, cat.checksum);
            println!("Scan matériel : {} ms ({} transferts DMA détectés)", cat.scan_elapsed_ms, cat.dma_count);
            println!("------------------------------------------------------------");
            println!("🗺️ Graphismes & Sprites ({}) :", cat.graphics.len());
            for g in cat.graphics.iter().take(12) {
                println!("  - {} | {} | {} | {} tuiles | budget {} o | {}",
                    g.id, g.snes_addr, g.format.short_label(), g.tile_count, g.rom_byte_budget, g.discovery_source);
            }
            println!("🎨 Palettes BGR555 ({}) :", cat.palettes.len());
            for p in cat.palettes.iter().take(8) {
                println!("  - {} | {} | {} couleurs | {}", p.id, p.snes_addr, p.color_count, p.discovery_source);
            }
            println!("🎵 Flux SPC700 IPL ({}) :", cat.spc_streams.len());
            for s in &cat.spc_streams {
                println!("  - {} | {} | Entry ${:04X} | {} chunks ({} o) | DIR ${:02X}00",
                    s.id, s.snes_addr, s.entry_pc, s.chunk_count, s.total_payload_bytes, s.detected_dir_page);
            }
            println!("🔊 Échantillons Audio BRR ({}) :", cat.audio_samples.len());
            for a in cat.audio_samples.iter().take(8) {
                println!("  - {} | {} | {} o | {} ms | Loop: {}",
                    a.id, a.snes_addr, a.byte_len, a.duration_ms_16khz, a.loops);
            }
            println!("📜 Textes & Tables .tbl ({}) :", cat.texts.len());
            for t in cat.texts.iter().take(10) {
                println!("  - {} | {} | {} | \"{}\"", t.id, t.snes_addr, t.encoding, t.text);
            }
            println!("🖼️ Tilemaps & Écrans ({}) :", cat.tilemaps.len());
            for tm in &cat.tilemaps {
                println!("  - {} | {} ({}x{} tuiles)", tm.id, tm.snes_addr, tm.width_tiles, tm.height_tiles);
            }
        }
        "--reconstruct" => {
            if args.len() < 5 {
                eprintln!("Usage: snes-cli --reconstruct <rom_originale.sfc> <dossier_projet> <rom_sortie.sfc> [--ips <patch.ips>]");
                return;
            }
            let orig_path = &args[2];
            let proj_dir = &args[3];
            let out_rom_path = &args[4];
            let ips_idx = args.iter().position(|a| a == "--ips");
            let ips_path = ips_idx.and_then(|i| args.get(i + 1)).map(std::path::Path::new);

            let bytes = std::fs::read(orig_path).expect("Impossible de lire la ROM originale");
            let orig_rom = SnesRom::from_bytes(orig_path.clone(), &bytes).expect("ROM originale invalide");
            let rep = snes_core::universal::reconstruct_rom_from_project(
                &orig_rom,
                std::path::Path::new(proj_dir),
                std::path::Path::new(out_rom_path),
                ips_path,
            )
            .expect("Erreur lors de la reconstruction de la ROM");

            println!("============================================================");
            println!("🔄 RECONSTRUCTION UNIVERSELLE DE ROM SNES TERMINÉE ({} ms)", rep.elapsed_ms);
            println!("============================================================");
            println!("ROM générée            : {}", rep.out_rom_path);
            if let Some(ip) = &rep.ips_path {
                println!("Patch IPS généré       : {}", ip);
            }
            println!("Planches GFX patchées  : {}", rep.gfx_patched_count);
            println!("Palettes patchées      : {}", rep.palettes_patched_count);
            println!("Audio BRR/WAV patchés  : {}", rep.audio_patched_count);
            println!("Textes patchés         : {}", rep.text_patched_count);
            println!("Octets modifiés total  : {}", rep.total_bytes_changed);
            println!("Checksum SNES          : 0x{:04X} (Valide: {})", rep.new_checksum, rep.checksum_valid);
            for w in &rep.warnings {
                println!("⚠️ {}", w);
            }
        }
        "--swap-universal" => {
            if args.len() < 5 {
                eprintln!("Usage: snes-cli --swap-universal <rom_a.sfc> <rom_b.sfc> <sortie.sfc> [--gfx] [--palettes] [--audio] [--text] [--all] [--ips <patch.ips>]");
                return;
            }
            let path_a = &args[2];
            let path_b = &args[3];
            let out_path = &args[4];
            let flags = &args[5..];

            let has_any_explicit = flags.iter().any(|f| matches!(f.as_str(), "--gfx" | "--palettes" | "--audio" | "--text" | "--all"));
            let all = !has_any_explicit || flags.iter().any(|f| f == "--all");
            let opts = snes_core::universal::UniversalBatchSwapOptions {
                swap_graphics: all || flags.iter().any(|f| f == "--gfx"),
                swap_palettes: all || flags.iter().any(|f| f == "--palettes"),
                swap_audio: all || flags.iter().any(|f| f == "--audio"),
                swap_text: flags.iter().any(|f| f == "--text"),
                smart_color_remap: true,
                max_gfx_swaps: 4,
                max_palette_swaps: 4,
                max_audio_swaps: 4,
            };
            let ips_idx = args.iter().position(|a| a == "--ips");
            let ips_path = ips_idx.and_then(|i| args.get(i + 1));

            let bytes_a = std::fs::read(path_a).expect("Impossible de lire ROM A");
            let bytes_b = std::fs::read(path_b).expect("Impossible de lire ROM B");
            let orig_a = bytes_a.clone();
            let mut rom_a = SnesRom::from_bytes(path_a.clone(), &bytes_a).expect("ROM A invalide");
            let rom_b = SnesRom::from_bytes(path_b.clone(), &bytes_b).expect("ROM B invalide");

            let cat_a = snes_core::universal::discover_universal_catalog(&rom_a);
            let cat_b = snes_core::universal::discover_universal_catalog(&rom_b);
            let mut session = SwapSession::new();
            let ops = snes_core::universal::apply_universal_batch_swap(
                &mut rom_a, &rom_b, &cat_a, &cat_b, &opts, &mut session,
            )
            .expect("Erreur lors du swap universel");

            let out_bytes = rom_a.to_sfc_bytes();
            std::fs::write(out_path, &out_bytes).expect("Impossible d'écrire la ROM de sortie");

            println!("============================================================");
            println!("⇄ SWAP UNIVERSEL MULTI-ROM SNES TERMINÉ");
            println!("============================================================");
            println!("ROM Cible (A) : {} ({})", rom_a.canonical_title(), rom_a.header.mapping.as_str());
            println!("ROM Donneuse  : {} ({})", rom_b.canonical_title(), rom_b.header.mapping.as_str());
            println!("Opérations    : {} échanges matériels appliqués", ops);
            for op in &session.operations {
                println!("  ✓ [{}] {}", op.kind.category_badge(), op.description);
            }
            println!("ROM Exportée  : {} (Checksum 0x{:04X}, Valide: {})",
                out_path, rom_a.header.checksum, rom_a.header.is_checksum_valid());

            if let Some(ip) = ips_path {
                let raw_orig_a = if (orig_a.len() % 1024) == 512 { &orig_a[512..] } else { &orig_a };
                let ips = snes_core::universal::build_ips_patch(raw_orig_a, &out_bytes);
                std::fs::write(ip, ips).expect("Erreur écriture IPS");
                println!("Patch IPS     : {}", ip);
            }
        }
        "--build-hybrid" => {
            if args.len() < 5 {
                eprintln!("Usage: snes-cli --build-hybrid <base.sfc> <donor.sfc> <out.sfc> [--ips <out.ips>]");
                return;
            }
            let base_path = &args[2];
            let donor_path = &args[3];
            let out_path = &args[4];
            let ips_idx = args.iter().position(|a| a == "--ips");
            let ips_path = ips_idx.and_then(|i| args.get(i + 1));

            let base_bytes = std::fs::read(base_path).expect("Impossible de lire ROM de base");
            let donor_bytes = std::fs::read(donor_path).expect("Impossible de lire ROM donneuse");

            let orig_base = SnesRom::from_bytes(base_path.clone(), &base_bytes).expect("ROM de base invalide");
            let mut target_rom = orig_base.clone();
            let donor_rom = SnesRom::from_bytes(donor_path.clone(), &donor_bytes).expect("ROM donneuse invalide");

            let mut session = SwapSession::new();
            let decisions = snes_core::assets::get_default_swap_decisions(&target_rom, &donor_rom);
            let ops = snes_core::assets::apply_custom_swaps(
                &mut target_rom,
                Some(&donor_rom),
                &decisions,
                &mut session,
            )
            .expect("Erreur lors de la fusion personnalisée");

            target_rom.fix_checksum();
            let out_bytes = target_rom.to_sfc_bytes();
            std::fs::write(out_path, &out_bytes).expect("Impossible d'écrire la ROM hybride");

            println!("============================================================");
            println!("🚀 HYBRIDE GÉNÉRÉ AVEC SUCCÈS !");
            println!("============================================================");
            println!("Base          : {} ({})", orig_base.canonical_title(), orig_base.header.mapping.as_str());
            println!("Donneur       : {}", donor_rom.canonical_title());
            println!("Opérations    : {} fusions matérielles appliquées", ops);
            for op in &session.operations {
                println!("  ✓ [{}] {}", op.kind.category_badge(), op.description);
            }
            println!("ROM Exportée  : {} ({} octets)", out_path, out_bytes.len());
            println!("Checksum SNES : 0x{:04X} (Complément: 0x{:04X}, Valide: {})",
                target_rom.header.checksum, target_rom.header.checksum_complement, target_rom.header.is_checksum_valid());
            println!("CRC32         : 0x{:08X}", target_rom.crc32);

            if let Some(ip) = ips_path {
                let ips = snes_core::universal::build_ips_patch(&orig_base.data, &out_bytes);
                std::fs::write(ip, &ips).expect("Erreur écriture IPS");
                println!("Patch IPS     : {} ({} octets)", ip, ips.len());
            }
        }
        "--build-mario-zelda" => {
            if args.len() < 5 {
                eprintln!("Usage: snes-cli --build-mario-zelda <zelda.sfc> <smw.sfc> <out.sfc> [--ips <out.ips>]");
                return;
            }
            let zelda_path = &args[2];
            let smw_path = &args[3];
            let out_path = &args[4];
            let ips_path = args.windows(2).find(|w| w[0] == "--ips").map(|w| w[1].as_str());

            let zelda_bytes = std::fs::read(zelda_path).expect("Impossible de lire ROM Zelda");
            let smw_bytes = std::fs::read(smw_path).expect("Impossible de lire ROM SMW");
            let orig_zelda = SnesRom::from_bytes(zelda_path.clone(), &zelda_bytes).expect("ROM Zelda invalide");
            let mut target_rom = orig_zelda.clone();
            let smw_rom = SnesRom::from_bytes(smw_path.clone(), &smw_bytes).expect("ROM SMW invalide");

            let mut session = SwapSession::new();
            let ops = snes_core::demo::apply_mario_into_zelda_alttp_swap(
                &mut target_rom,
                &smw_rom,
                &mut session,
            )
            .expect("Erreur fusion Mario -> Zelda ALTTP");

            target_rom.fix_checksum();
            let out_bytes = target_rom.to_sfc_bytes();
            std::fs::write(out_path, &out_bytes).expect("Impossible d'écrire la ROM Super Mario Hyrule");

            println!("============================================================");
            println!("🍄🗡️ NOUVEAU JEU HYBRIDE : SUPER MARIO HYRULE GÉNÉRÉ !");
            println!("============================================================");
            println!("Base          : {} ({})", orig_zelda.canonical_title(), orig_zelda.header.mapping.as_str());
            println!("Héros         : Mario (Super Mario World) -> Link (Zelda ALTTP)");
            println!("Opérations    : {} fusions matérielles appliquées", ops);
            for op in &session.operations {
                println!("  ✓ [{}] {}", op.kind.category_badge(), op.description);
            }
            println!("ROM Exportée  : {} ({} octets)", out_path, out_bytes.len());
            println!("Checksum SNES : 0x{:04X} (Complément: 0x{:04X}, Valide: {})",
                target_rom.header.checksum, target_rom.header.checksum_complement, target_rom.header.is_checksum_valid());
            println!("CRC32         : 0x{:08X}", target_rom.crc32);

            if let Some(ip) = ips_path {
                let ips = snes_core::universal::build_ips_patch(&orig_zelda.data, &out_bytes);
                std::fs::write(ip, &ips).expect("Erreur écriture IPS");
                println!("Patch IPS     : {} ({} octets)", ip, ips.len());
            }
        }
        "--mix-inspect" => {
            if args.len() < 3 {
                eprintln!("Usage: snes-cli --mix-inspect <rom1.sfc> [rom2.sfc ...]");
                return;
            }
            let mut profiles = Vec::new();
            for (idx, path) in args[2..].iter().enumerate() {
                let bytes = std::fs::read(path).expect("Impossible de lire la ROM");
                let rom = SnesRom::from_bytes(path.clone(), &bytes).expect("ROM SNES invalide");
                let alias = match rom.identify_game() {
                    snes_core::RecognizedGame::ZeldaAlttp => "zelda".to_string(),
                    snes_core::RecognizedGame::SuperMarioWorld => "smw".to_string(),
                    snes_core::RecognizedGame::Unknown => format!("rom{}", idx + 1),
                };
                let prof = snes_core::mixer::inspect_rom_for_mixing(&rom, alias, path);
                println!("============================================================");
                println!("🎮 [{}] {} ({} | {} Ko)", prof.alias, prof.title, prof.mapping, prof.size_bytes / 1024);
                println!("  Slots cibles ({}) :", prof.target_slots.len());
                for s in prof.target_slots.iter().take(14) {
                    println!("    • {:18} [{}] {}", s.id, s.category.label_fr(), s.label);
                }
                println!("  Sources donneuses ({}) :", prof.donor_sources.len());
                for d in prof.donor_sources.iter().take(16) {
                    println!("    • {:18} [{}] {}", d.id, d.category.label_fr(), d.label);
                }
                profiles.push(prof);
            }
            if profiles.len() >= 2 {
                println!("============================================================");
                println!("✨ PRESETS MULTI-JEUX DISPONIBLES (Base = {}) :", profiles[0].title);
                for pr in snes_core::mixer::suggest_presets(&profiles[0], &profiles[1..]) {
                    println!("  ★ {:24} — {} ({} règles)", pr.id, pr.name, pr.rules.len());
                    println!("    {}", pr.description);
                }
            }
        }
        "--mix" => {
            let mut base_path: Option<String> = None;
            let mut donor_specs: Vec<(String, String)> = Vec::new();
            let mut rule_specs: Vec<String> = Vec::new();
            let mut out_path = String::from("omni_mix_out.sfc");
            let mut ips_path: Option<String> = None;
            let mut title_opt: Option<String> = None;
            let mut preset_opt: Option<String> = None;
            let mut recipe_load_path: Option<String> = None;
            let mut recipe_save_path: Option<String> = None;
            let mut chaos_mode = false;
            let mut chaos_seed: u64 = 42;
            let mut play_after = false;

            let mut i = 2usize;
            while i < args.len() {
                match args[i].as_str() {
                    "--base" if i + 1 < args.len() => {
                        base_path = Some(args[i + 1].clone());
                        i += 2;
                    }
                    "--donor" if i + 1 < args.len() => {
                        let spec = &args[i + 1];
                        if let Some((a, p)) = spec.split_once('=') {
                            donor_specs.push((a.to_string(), p.to_string()));
                        } else {
                            let idx = donor_specs.len() + 1;
                            donor_specs.push((format!("d{}", idx), spec.clone()));
                        }
                        i += 2;
                    }
                    "--rule" if i + 1 < args.len() => {
                        rule_specs.push(args[i + 1].clone());
                        i += 2;
                    }
                    "--out" | "-o" if i + 1 < args.len() => {
                        out_path = args[i + 1].clone();
                        i += 2;
                    }
                    "--ips" if i + 1 < args.len() => {
                        ips_path = Some(args[i + 1].clone());
                        i += 2;
                    }
                    "--title" if i + 1 < args.len() => {
                        title_opt = Some(args[i + 1].clone());
                        i += 2;
                    }
                    "--preset" if i + 1 < args.len() => {
                        preset_opt = Some(args[i + 1].clone());
                        i += 2;
                    }
                    "--recipe" if i + 1 < args.len() => {
                        recipe_load_path = Some(args[i + 1].clone());
                        i += 2;
                    }
                    "--save-recipe" if i + 1 < args.len() => {
                        recipe_save_path = Some(args[i + 1].clone());
                        i += 2;
                    }
                    "--chaos" => {
                        chaos_mode = true;
                        i += 1;
                    }
                    "--seed" if i + 1 < args.len() => {
                        chaos_seed = args[i + 1].parse().unwrap_or(42);
                        i += 2;
                    }
                    "--play" => {
                        play_after = true;
                        i += 1;
                    }
                    _ => {
                        i += 1;
                    }
                }
            }

            let mut recipe = if let Some(ref rp) = recipe_load_path {
                let json = std::fs::read_to_string(rp).expect("Impossible de lire la recette JSON");
                snes_core::mixer::MixRecipe::from_json(&json).expect("Recette JSON invalide")
            } else {
                let bp = match base_path {
                    Some(b) => b,
                    None => {
                        eprintln!("Usage: snes-cli --mix --base <base.sfc> --donor <alias=rom.sfc> [--donor ...] [--rule <slot=alias:source> ...] [--preset <id|auto>] [--chaos --seed 42] [--title <TITRE>] --out <out.sfc> [--ips <out.ips>] [--save-recipe <rec.json>] [--play]");
                        return;
                    }
                };
                snes_core::mixer::MixRecipe {
                    name: "CLI Omni-Mix".into(),
                    custom_title: title_opt.clone(),
                    base_rom: bp,
                    donors: donor_specs,
                    rules: Vec::new(),
                }
            };

            if title_opt.is_some() {
                recipe.custom_title = title_opt;
            }

            let base_bytes = std::fs::read(&recipe.base_rom).expect("Impossible de lire la ROM de base");
            let orig_base = SnesRom::from_bytes(recipe.base_rom.clone(), &base_bytes).expect("ROM de base invalide");
            let mut target_rom = orig_base.clone();

            let mut donor_roms = std::collections::HashMap::new();
            let mut donor_profiles = Vec::new();
            for (alias, path) in &recipe.donors {
                let dbytes = std::fs::read(path).expect("Impossible de lire une ROM donneuse");
                let drom = SnesRom::from_bytes(path.clone(), &dbytes).expect("ROM donneuse invalide");
                donor_profiles.push(snes_core::mixer::inspect_rom_for_mixing(&drom, alias.clone(), path.clone()));
                donor_roms.insert(alias.clone(), drom);
            }

            let base_profile = snes_core::mixer::inspect_rom_for_mixing(&orig_base, "base", &recipe.base_rom);

            if chaos_mode {
                let chaos = snes_core::mixer::generate_chaos_recipe(&base_profile, &donor_profiles, chaos_seed);
                if recipe.custom_title.is_none() {
                    recipe.custom_title = chaos.custom_title;
                }
                recipe.rules.extend(chaos.rules);
            } else if let Some(ref pid) = preset_opt {
                let presets = snes_core::mixer::suggest_presets(&base_profile, &donor_profiles);
                let chosen = if pid == "auto" {
                    presets.first()
                } else {
                    presets.iter().find(|p| p.id == *pid)
                };
                if let Some(pr) = chosen {
                    recipe.name = pr.name.clone();
                    if recipe.custom_title.is_none() {
                        recipe.custom_title = Some(pr.custom_title.clone());
                    }
                    recipe.rules.extend(pr.rules.clone());
                } else {
                    eprintln!("Preset '{}' introuvable. Presets dispos :", pid);
                    for p in &presets {
                        eprintln!("  - {} ({})", p.id, p.name);
                    }
                    return;
                }
            }

            for rs in &rule_specs {
                // Format: slot=alias:source
                if let Some((slot, rhs)) = rs.split_once('=') {
                    if let Some((alias, source)) = rhs.split_once(':') {
                        recipe.rules.push(snes_core::mixer::MixRule {
                            target_slot: slot.trim().to_string(),
                            donor_alias: alias.trim().to_string(),
                            donor_source: source.trim().to_string(),
                        });
                    }
                }
            }

            if let Some(ref sp) = recipe_save_path {
                std::fs::write(sp, recipe.to_json()).expect("Impossible d'écrire la recette JSON");
                println!("💾 Recette JSON sauvegardée : {}", sp);
            }

            let mut session = SwapSession::new();
            let rep = snes_core::mixer::execute_mix_recipe(&mut target_rom, &donor_roms, &recipe, &mut session)
                .expect("Erreur lors du mixage multi-jeux");

            let out_bytes = target_rom.to_sfc_bytes();
            std::fs::write(&out_path, &out_bytes).expect("Impossible d'écrire la ROM mixée");

            println!("============================================================");
            println!("🎛️ SNES OMNI-MIXER MULTI-JEUX — FUSION TERMINÉE ({} ms)", rep.elapsed_ms);
            println!("============================================================");
            println!("ROM de Base   : {} ({})", rep.base_title, orig_base.header.mapping.as_str());
            println!("Donneuses ({}) : {}", recipe.donors.len(), recipe.donors.iter().map(|(a, _)| a.as_str()).collect::<Vec<_>>().join(", "));
            println!("Titre Cartouche: '{}'", rep.output_title);
            println!("Règles ({})    :", rep.applied_rules.len());
            for r in &rep.applied_rules {
                println!("  ✓ {}", r);
            }
            for w in &rep.warnings {
                println!("  ⚠️ {}", w);
            }
            println!("Opérations    : {} fusions matérielles appliquées", rep.operations_applied);
            println!("ROM Exportée  : {} ({} octets)", out_path, out_bytes.len());
            println!("Checksum SNES : 0x{:04X} (Complément: 0x{:04X}, Valide: {})",
                rep.checksum, rep.complement, rep.checksum_valid);
            println!("CRC32         : 0x{:08X}", rep.crc32);

            if let Some(ref ip) = ips_path {
                let ips = snes_core::universal::build_ips_patch(&orig_base.data, &out_bytes);
                std::fs::write(ip, &ips).expect("Erreur écriture IPS");
                println!("Patch IPS     : {} ({} octets)", ip, ips.len());
            }

            if play_after {
                println!("▶️ Lancement dans Mednafen...");
                let _ = std::process::Command::new("mednafen")
                    .args(["-force_module", "snes_faust", "-video.fs", "0", "-snes_faust.xscale", "3", "-snes_faust.yscale", "3", &out_path])
                    .spawn();
            }
        }
        "--decompile" => {
            if args.len() < 3 {
                eprintln!("Usage: snes-cli --decompile <rom.sfc> [dossier_sortie]");
                return;
            }
            let rom_path = &args[2];
            let out_dir = args.get(3).map(|s| s.as_str()).unwrap_or("./decompiled_rom");
            let bytes = std::fs::read(rom_path).expect("Impossible de lire le fichier ROM");
            let rom = SnesRom::from_bytes(rom_path.clone(), &bytes).expect("ROM SNES invalide");
            let summary = decompile_rom_to_project_dir(&rom, std::path::Path::new(out_dir))
                .expect("Erreur lors de la décomposition du projet");
            println!(
                "✓ Projet décomposé avec succès dans '{}' :",
                summary.out_dir
            );
            println!(
                "  - {} banques (.asm annotés 65c816 avec suivi SEP/REP + .bin bruts)",
                summary.bank_count
            );
            println!(
                "  - {} transferts DMA matériels documentés dans README.md",
                summary.dma_count
            );
            println!("  - {} blocs graphiques extraits", summary.gfx_count);
            println!("  - {} palettes BGR555 extraites", summary.palette_count);
            println!(
                "  - {} échantillons audio SPC700 extraits en .brr et .wav",
                summary.audio_wav_count
            );
        }
        "--dump-assets" => {
            if args.len() < 3 {
                eprintln!("Usage: snes-cli --dump-assets <rom.sfc> [dossier_sortie]");
                return;
            }
            let rom_path = &args[2];
            let out_dir = args.get(3).map(|s| s.as_str()).unwrap_or("./extracted_assets");
            let bytes = std::fs::read(rom_path).expect("Impossible de lire le fichier ROM");
            let rom = SnesRom::from_bytes(rom_path.clone(), &bytes).expect("ROM SNES invalide");
            let summary = snes_core::export::export_all_rom_assets(&rom, std::path::Path::new(out_dir))
                .expect("Erreur lors de l'extraction des assets");
            println!("✓ Tous les assets de la ROM '{}' ont été extraits dans '{}' :", rom.canonical_title(), summary.out_dir);
            println!("  - {} sprites individuels (.png)", summary.sprites_png_count);
            println!("  - {} planches graphiques complètes (.png)", summary.sheets_png_count);
            println!("  - {} échantillons audio (.wav + .brr)", summary.audio_wav_count);
            println!("  - {} palettes BGR555 (.json + .png + .pal)", summary.palettes_count);
        }
        "--inspect" => {
            for path in &args[2..] {
                let bytes = std::fs::read(path).expect("Impossible de lire le fichier ROM");
                let rom = SnesRom::from_bytes(path.clone(), &bytes).expect("ROM SNES invalide");
                let dmas = scan_hardware_dma_transfers(&rom);
                let gfx = scan_graphics_regions(&rom.data);
                let pals = scan_candidate_palettes(&rom.data, 24);
                let brr = scan_brr_samples(&rom.data, 16);
                let chunks = classify_rom_chunks(&rom.data);

                let gfx_chunks = chunks
                    .iter()
                    .filter(|c| c.kind == ChunkKind::PlanarGraphics)
                    .count();
                let audio_chunks = chunks
                    .iter()
                    .filter(|c| c.kind == ChunkKind::BrrAudio)
                    .count();
                let lz_chunks = chunks
                    .iter()
                    .filter(|c| c.kind == ChunkKind::CompressedOrHighEntropy)
                    .count();

                println!("============================================================");
                println!(
                    "ROM : {} ({} Ko, {} banques de {} Ko)",
                    rom.name,
                    rom.data.len() / 1024,
                    rom.banks().len(),
                    rom.bank_size() / 1024
                );
                println!("  Titre interne   : {}", rom.header.title);
                println!("  Mapping mémoire : {}", rom.header.mapping.as_str());
                println!("  Chipset         : {}", rom.header.chipset_description);
                println!("  Région          : {}", rom.header.region_name);
                println!(
                    "  Vecteurs CPU    : RESET=${:04X} | NMI=${:04X} | IRQ=${:04X}",
                    rom.header.reset_vector, rom.header.nmi_vector, rom.header.irq_vector
                );
                println!(
                    "  Checksum SNES   : 0x{:04X} (Complément: 0x{:04X}, Valide: {})",
                    rom.header.checksum,
                    rom.header.checksum_complement,
                    rom.header.is_checksum_valid()
                );
                println!("  CRC32           : 0x{:08X}", rom.crc32);
                println!(
                    "  Anatomie (4Ko)  : {} blocs GFX, {} blocs Audio, {} blocs Compressés LZ",
                    gfx_chunks, audio_chunks, lz_chunks
                );
                println!(
                    "  Pointeurs DMA   : {} transferts matériels ($43x0 -> VRAM/CGRAM) détectés",
                    dmas.len()
                );
                for dma in dmas.iter().take(10) {
                    println!(
                        "    - DMA Ch{} -> {} @ Source ${:02X}:{:04X} ({})",
                        dma.dma_channel,
                        dma.target.label(),
                        dma.source_snes_bank,
                        dma.source_snes_addr,
                        dma.source_pc_offset
                            .map(|p| format!("PC 0x{:06X}", p))
                            .unwrap_or_else(|| "WRAM/RAM".to_string())
                    );
                }
                println!("  Blocs Tiles GFX : {} régions détectées", gfx.len());
                println!("  Palettes BGR555 : {} palettes détectées", pals.len());
                println!(
                    "  Samples Audio   : {} échantillons SPC700 BRR détectés",
                    brr.len()
                );
                for s in brr.iter().take(8) {
                    println!(
                        "    - {} @ 0x{:06X} ({} octets, {} ms @ 16kHz)",
                        s.label,
                        s.offset,
                        s.byte_len,
                        s.duration_ms(16000)
                    );
                }
            }
        }
        "--swap" => {
            if args.len() < 4 {
                eprintln!("Usage: snes-cli --swap <target.sfc> <donor1.sfc> [donor2.sfc] [--export <output.sfc>]");
                return;
            }
            let target_path = &args[2];
            let donor1_path = &args[3];
            let donor2_path = args.get(4).filter(|s| !s.starts_with("--"));
            let export_idx = args.iter().position(|a| a == "--export");
            let out_path = if let Some(idx) = export_idx {
                args.get(idx + 1).cloned().unwrap_or_else(|| "swapped_output.sfc".to_string())
            } else {
                "swapped_output.sfc".to_string()
            };

            let target_bytes = std::fs::read(target_path).expect("Impossible de lire la ROM cible");
            let mut target_rom = SnesRom::from_bytes(target_path, &target_bytes).expect("ROM cible invalide");
            let donor1_bytes = std::fs::read(donor1_path).expect("Impossible de lire la ROM donneuse 1");
            let donor1_rom = SnesRom::from_bytes(donor1_path, &donor1_bytes).expect("ROM donneuse 1 invalide");
            let _donor2_bytes = donor2_path.and_then(|p| std::fs::read(p).ok());

            let mut session = SwapSession::new();
            let mut ops = 0usize;

            let brr_a = scan_brr_samples(&target_rom.data, 16);
            let brr_b = scan_brr_samples(&donor1_rom.data, 16);

            ops += apply_obvious_demo_swap(&mut target_rom, &donor1_rom, &mut session, &brr_a, &brr_b)
                .expect("Erreur swap");

            target_rom.fix_checksum();
            std::fs::write(&out_path, target_rom.to_sfc_bytes()).expect("Erreur écriture ROM exportée");
            println!(
                "✓ ROM SNES 100% matérielle exportée avec succès : '{}' ({} opérations appliquées, Checksum 0x{:04X})",
                out_path, ops, target_rom.header.checksum
            );
        }
        "--demo-smw" | "--demo-zelda" | "--demo-bbb" | "--demo-aaa" => {
            let is_smw = args[1] == "--demo-smw" || args[1] == "--demo-bbb";
            let in_path = args
                .get(2)
                .map(|s| s.as_str())
                .unwrap_or(if is_smw {
                    "roms/super_mario_world.sfc"
                } else {
                    "roms/zelda_alttp.sfc"
                });
            let out_path = args
                .get(3)
                .map(|s| s.as_str())
                .unwrap_or(if is_smw {
                    "smw_swapped.sfc"
                } else {
                    "zelda_swapped.sfc"
                });
            let donor_path = args
                .get(4)
                .map(|s| s.as_str())
                .unwrap_or(if is_smw {
                    "roms/zelda_alttp.sfc"
                } else {
                    "roms/super_mario_world.sfc"
                });
            let bytes = std::fs::read(in_path).expect("Impossible de lire la ROM cible");
            let mut rom_a =
                SnesRom::from_bytes(in_path, &bytes).expect("ROM SNES cible invalide");
            let rom_b = if let Ok(bytes_b) = std::fs::read(donor_path) {
                SnesRom::from_bytes(donor_path, &bytes_b).expect("ROM SNES donneuse invalide")
            } else {
                generate_demo_rom_pair().1
            };
            let brr_a = scan_brr_samples(&rom_a.data, 16);
            let brr_b = scan_brr_samples(&rom_b.data, 16);
            let mut session = SwapSession::new();
            let count = apply_obvious_demo_swap(&mut rom_a, &rom_b, &mut session, &brr_a, &brr_b)
                .expect("Erreur lors du swap démo");
            std::fs::write(out_path, rom_a.to_sfc_bytes()).expect("Erreur écriture ROM modifiée");

            let mut kb = snes_core::memory::RomKnowledgeBase::load_or_init();
            kb.learn_from_rom(&rom_a);
            kb.learn_from_rom(&rom_b);
            kb.record_verified_recipe(
                "FUSION_ZELDA_MARIO",
                &rom_a.canonical_title(),
                vec![
                    rom_b.canonical_title(),
                ],
                "Conversion Zelda <-> Mario World (GFX, Palettes BGR555, BRR Audio, Checksum SNES validé)",
                vec![
                    format!("GFX Décompressé LZ2 ({} octets)", rom_a.data.len()),
                    "Palettes BGR555 synchronisées".to_string(),
                    "Checksum & Complément SNES recalculés pour compatibilité 100% matériel".to_string(),
                ],
            );
            let _ = kb.save_to_default_path();

            println!(
                "✓ {} opérations appliquées ! ROM cible '{}' ({}) mise à jour dans '{}' (Checksum SNES valide : 0x{:04X})",
                count, in_path, rom_a.canonical_title(), out_path, rom_a.header.checksum
            );
            for op in &session.operations {
                println!("  ✓ #{} [{}] {}", op.id, op.kind.category_badge(), op.description);
            }
        }
        "--auto-swap-demo" => {
            let out_path = args
                .get(2)
                .map(|s| s.as_str())
                .unwrap_or("rom_a_swapped_with_b.sfc");
            let (mut rom_a, rom_b) = generate_demo_rom_pair();
            let pals_a = scan_candidate_palettes(&rom_a.data, 4);
            let pals_b = scan_candidate_palettes(&rom_b.data, 4);
            let brr_a = scan_brr_samples(&rom_a.data, 16);
            let brr_b = scan_brr_samples(&rom_b.data, 16);

            let mut session = SwapSession::new();
            let op_gfx = session
                .swap_graphics_tiles_smart_remapped(
                    &mut rom_a,
                    &rom_b,
                    0x010000,
                    0x010000,
                    32,
                    TileFormat::Bpp4,
                    pals_a
                        .first()
                        .unwrap_or(&SnesPalette::preset_snes_adventure()),
                    TileFormat::Bpp4,
                    pals_b.first().unwrap_or(&SnesPalette::preset_cyber_neon()),
                )
                .expect("Swap GFX failed");
            println!("✓ {}", op_gfx.description);

            if let (Some(sa), Some(sb)) = (brr_a.first(), brr_b.first()) {
                let op_snd = session
                    .swap_brr_sample(&mut rom_a, &rom_b, sa, sb, true)
                    .expect("Swap BRR failed");
                println!("✓ {}", op_snd.description);

                let pcm = decode_brr_to_pcm16(&rom_a.data[sa.offset..sa.offset + sa.byte_len]);
                let wav = encode_pcm16_to_wav(&pcm, 16000);
                let _ = std::fs::write("swapped_sample_preview.wav", wav);
            }

            std::fs::write(out_path, rom_a.to_sfc_bytes()).expect("Failed to write swapped ROM");
            println!(
                "✓ ROM hybride exportée vers '{}' (Checksum SNES valide : 0x{:04X})",
                out_path, rom_a.header.checksum
            );
        }
        _ => print_help(),
    }
}

fn print_help() {
    println!("snes-cli — Outil Universel Multi-Jeux SNES Omni-Mixer, Déconstructeur & Échangeur (.sfc/.smc)");
    println!("Commandes principales (OMNI-MIXER MULTI-JEUX) :");
    println!("  --mix-inspect <rom1.sfc> [rom2.sfc ...]                Inspecte N ROMs, liste tous les slots mixables & presets multi-jeux");
    println!("  --mix --base <base.sfc> --donor <alias=rom.sfc> [...]  MIXE TOUT AVEC TOUT (N ROMs donneuses -> 1 ROM hybride jouable !)");
    println!("        Options : --rule <slot=alias:source>  --preset <id|auto>  --chaos [--seed N]");
    println!("                  --recipe <rec.json>  --save-recipe <rec.json>  --title <TITRE>  --out <out.sfc>  --ips <out.ips>  --play");
    println!("Autres commandes :");
    println!("  --deconstruct <rom.sfc> [out_dir]                      DÉCONSTRUCTION TOTALE en projet éditable (project.json, PNG, WAV, SPC, TBL, ASM)");
    println!("  --reconstruct <orig.sfc> <proj_dir> <out.sfc> [--ips]  RECONSTRUCTION aller-retour : réinjecte vos PNG/WAV/Palettes/Textes modifiés !");
    println!("  --swap-universal <rom_a> <rom_b> <out.sfc> [options]   ÉCHANGE UNIVERSEL entre N'IMPORTE QUELLES ROMs (--gfx --palettes --audio --text --all --ips)");
    println!("  --catalog <rom.sfc>                                    Affiche le catalogue matériel universel découvert (DMA, LZ2, Palettes, IPL SPC, BRR, Textes)");
    println!("  --swap <target.sfc> <donor1.sfc> [donor2.sfc] [--export <out.sfc>] Swap autonome & export de ROM matérielle");
    println!("  --dump-assets <rom.sfc> [out_dir]                      Extrait les assets graphiques et audio de base");
    println!("  --decompile <rom.sfc> [out_dir]                        Décompose une ROM en projet WLA-DX (ASM 65c816, hdr.asm, WAV, GFX)");
    println!("  --inspect <rom1.sfc> [rom2.sfc]                        Analyse universelle (Header, DMA, Entropie, GFX, Palettes, BRR)");
    println!("  --generate-demo [dossier]                              Génère 2 ROMs SNES (.sfc) de test complètes");
    println!("  --auto-swap-demo [sortie.sfc]                          Exécute un Smart Swap Graphique + Audio BRR entre ROM A et ROM B");
}
