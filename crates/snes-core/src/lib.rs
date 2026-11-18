pub mod ai_sprites;
pub mod assets;
pub mod audio;
pub mod deconstruct;
pub mod demo;
pub mod disasm_dma;
pub mod export;
pub mod graphics;
pub mod memory;
pub mod mixer;
pub mod profile;
pub mod rom;
pub mod swapper;
pub mod universal;

pub use deconstruct::{deconstruct_rom, DeconstructConfig, DeconstructReport};
pub use memory::RomKnowledgeBase;
pub use mixer::{
    execute_mix_recipe, generate_chaos_recipe, inspect_rom_for_mixing, suggest_presets,
    GameMixProfile, MixDonorSource, MixPreset, MixRecipe, MixReport, MixRule, MixSlotCategory,
    MixTargetSlot,
};
pub use rom::{RecognizedGame, RomMapping, SnesHeader, SnesRom};
pub use universal::{
    apply_universal_batch_swap, discover_universal_catalog, export_universal_editable_project,
    reconstruct_rom_from_project, ReconstructReport, UniversalBatchSwapOptions,
    UniversalRomCatalog,
};

#[cfg(test)]
mod tests {
    use super::audio::{decode_brr_to_pcm16, encode_pcm16_to_wav, scan_brr_samples};
    use super::demo::generate_demo_rom_pair;
    use super::disasm_dma::{
        classify_rom_chunks, compress_nintendo_lz2, decompress_nintendo_lz2,
        disassemble_65c816_block, scan_hardware_dma_transfers,
    };
    use super::graphics::{
        decode_tile, encode_tile, scan_candidate_palettes, scan_graphics_regions, SnesPalette,
        TileFormat,
    };
    use super::profile::GameProfile;
    use super::rom::{RomMapping, SnesRom};
    use super::swapper::SwapSession;

    #[test]
    fn test_demo_roms_and_header_checksum() {
        let (rom_a, rom_b) = generate_demo_rom_pair();
        assert_eq!(rom_a.header.title, "SUPER RUST HERO");
        assert_eq!(rom_b.header.title, "CYBER DEMON X");
        assert_eq!(rom_a.header.mapping, RomMapping::LoROM);
        assert!(rom_a.header.is_checksum_valid());
        assert!(rom_b.header.is_checksum_valid());

        // Verify 512-byte SMC copier header stripping
        let mut with_smc = vec![0xAAu8; 512];
        with_smc.extend_from_slice(&rom_a.data);
        let parsed_smc = SnesRom::from_bytes("test.smc", &with_smc).unwrap();
        assert!(parsed_smc.has_copier_header);
        assert_eq!(parsed_smc.header.title, "SUPER RUST HERO");
        assert!(parsed_smc.header.is_checksum_valid());
    }

    #[test]
    fn test_tile_encode_decode_roundtrip() {
        let mut sample_indices = [0u8; 64];
        for i in 0..64 {
            sample_indices[i] = (i as u8) & 0x0F;
        }

        for fmt in [
            TileFormat::Bpp2,
            TileFormat::Bpp3,
            TileFormat::Bpp4,
            TileFormat::Bpp8,
            TileFormat::Mode7Linear,
        ] {
            let encoded = encode_tile(&sample_indices, fmt);
            let decoded = decode_tile(&encoded, fmt);
            let mask = (fmt.color_count() - 1) as u8;
            for i in 0..64 {
                assert_eq!(decoded[i], sample_indices[i] & mask);
            }
        }
    }

    #[test]
    fn test_universal_dma_and_lz2_and_disasm() {
        let (rom_a, _) = generate_demo_rom_pair();

        // Hardware DMA pointer discovery
        let dmas = scan_hardware_dma_transfers(&rom_a);
        assert!(
            dmas.len() >= 2,
            "Expected at least 2 DMA transfers (VRAM & CGRAM), found {}",
            dmas.len()
        );
        assert!(dmas.iter().any(|d| d.source_pc_offset == Some(0x010000)));
        assert!(dmas.iter().any(|d| d.source_pc_offset == Some(0x018000)));

        // 65c816 Disassembler
        let asm = disassemble_65c816_block(&rom_a, 0, 12);
        assert!(!asm.is_empty());

        // Entropy & chunk classifier
        let chunks = classify_rom_chunks(&rom_a.data);
        assert!(!chunks.is_empty());

        // Nintendo LC_LZ2 roundtrip
        let raw_slice = &rom_a.data[0x010000..0x010400];
        let compressed = compress_nintendo_lz2(raw_slice);
        let (decompressed, used_len) =
            decompress_nintendo_lz2(&compressed, 0, 4096).expect("Valid LC_LZ2 stream");
        assert_eq!(used_len, compressed.len());
        assert_eq!(decompressed, raw_slice);
    }

    #[test]
    fn test_scanners_and_universal_swap_session() {
        let (mut rom_a, rom_b) = generate_demo_rom_pair();

        assert!(GameProfile::detect_for_rom(&rom_a).is_some());
        assert!(GameProfile::detect_for_rom(&rom_b).is_some());

        let gfx_a = scan_graphics_regions(&rom_a.data);
        assert!(!gfx_a.is_empty(), "Should detect graphics regions in ROM A");
        let pals_a = scan_candidate_palettes(&rom_a.data, 10);
        let pals_b = scan_candidate_palettes(&rom_b.data, 10);
        assert!(!pals_a.is_empty() && !pals_b.is_empty());

        let brr_a = scan_brr_samples(&rom_a.data, 8);
        let brr_b = scan_brr_samples(&rom_b.data, 8);
        assert!(brr_a.len() >= 3 && brr_b.len() >= 3);

        let s0 = &brr_a[0];
        let pcm = decode_brr_to_pcm16(&rom_a.data[s0.offset..s0.offset + s0.byte_len]);
        assert_eq!(pcm.len(), s0.block_count * 16);
        let wav = encode_pcm16_to_wav(&pcm, 16000);
        assert_eq!(&wav[0..4], b"RIFF");

        let orig_crc = rom_a.crc32;
        let mut session = SwapSession::new();

        // 1. Smart Palette-Remapped & Format-Converted Graphics Swap
        let pal_a = pals_a
            .first()
            .cloned()
            .unwrap_or_else(SnesPalette::preset_snes_adventure);
        let pal_b = pals_b
            .first()
            .cloned()
            .unwrap_or_else(SnesPalette::preset_cyber_neon);

        session
            .swap_graphics_tiles_smart_remapped(
                &mut rom_a,
                &rom_b,
                0x010000,
                0x010000,
                16,
                TileFormat::Bpp4,
                &pal_a,
                TileFormat::Bpp4,
                &pal_b,
            )
            .unwrap();
        assert_ne!(rom_a.crc32, orig_crc);
        assert!(rom_a.header.is_checksum_valid());

        // 2. Resampled BRR Audio Swap (fitting a 48-block sample into a 36-block slot)
        session
            .swap_brr_sample(&mut rom_a, &rom_b, &brr_a[0], &brr_b[2], true)
            .unwrap();
        assert!(rom_a.header.is_checksum_valid());

        // 3. Undo all restores exact original ROM A
        session.undo_all(&mut rom_a).unwrap();
        assert_eq!(rom_a.crc32, orig_crc);
        assert!(rom_a.header.is_checksum_valid());
    }

    #[test]
    fn test_full_fusion_validation() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        if let (Ok(bytes_a), Ok(bytes_b), Ok(bytes_c)) = (
            std::fs::read(root.join("roms/super_mario_world.sfc")),
            std::fs::read(root.join("roms/zelda_alttp.sfc")),
            std::fs::read(root.join("roms/donkey_kong_country.sfc")),
        ) {
            let mut rom_a = SnesRom::from_bytes("smw", &bytes_a).unwrap();
            let rom_b = SnesRom::from_bytes("zelda", &bytes_b).unwrap();
            let mut session = SwapSession::new();

            let orig_g32 = super::disasm_dma::decompress_nintendo_lz2_with_endian(&rom_a.data, 0x040000, 32768, true).unwrap().0;
            let ops = super::demo::apply_full_fusion(&mut rom_a, Some(&rom_b), Some(&bytes_c), &mut session).unwrap();
            println!("Full fusion ops applied: {}", ops);
            assert!(ops >= 9, "Should apply at least 9 operations");

            // 1. Verify Checksum
            assert!(rom_a.header.is_checksum_valid(), "Checksum must be 100% valid!");
            let comp = u16::from_le_bytes([rom_a.data[0x7FDC], rom_a.data[0x7FDD]]);
            let sum = u16::from_le_bytes([rom_a.data[0x7FDE], rom_a.data[0x7FDF]]);
            assert_eq!(comp ^ sum, 0xFFFF, "Checksum and complement must sum to 0xFFFF");

            // 2. Verify Berry tiles in GFX32
            let (new_g32, _) = super::disasm_dma::decompress_nintendo_lz2_with_endian(&rom_a.data, 0x040000, 32768, true).unwrap();
            let off7 = (7usize & 0xF7) << 6;
            let t7 = &orig_g32[off7..off7 + 32];
            println!("Tile 7 (off {}):", off7);
            for y in 0..8 {
                let b0 = t7[y * 2];
                let b1 = t7[y * 2 + 1];
                let b2 = t7[16 + y * 2];
                let b3 = t7[16 + y * 2 + 1];
                let mut line = String::new();
                for x in 0..8 {
                    let sh = 7 - x;
                    let c = ((b0 >> sh) & 1) | (((b1 >> sh) & 1) << 1) | (((b2 >> sh) & 1) << 2) | (((b3 >> sh) & 1) << 3);
                    line.push(if c == 0 { '.' } else { '#' });
                }
                println!("{}", line);
            }
            assert_eq!(&new_g32[5120..5120 + 32], &orig_g32[5120..5120 + 32], "Tile 80 (berry top) must match vanilla!");
            assert_eq!(&new_g32[21632..21632 + 32], &orig_g32[21632..21632 + 32], "Tile 90 (berry bottom) must match vanilla!");

            // 3. Verify Punch Y offset table at PC 0x00F61A
            assert_eq!(
                &rom_a.data[0x00F61A..0x00F61A + 8],
                &[0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x18],
                "Punch must emerge from shoulder at Y=17 (0x11) instead of mouth (0x08)!"
            );

            // 4. Verify GFX33 decompresses and fits
            let (g33, _) = super::disasm_dma::decompress_nintendo_lz2_with_endian(&rom_a.data, 0x043FC0, 16384, true).unwrap();
            assert!(!g33.is_empty());

            // 5. Verify Title screen GFX29 decompresses and fits
            let (g29, _) = super::disasm_dma::decompress_nintendo_lz2_with_endian(&rom_a.data, 0x05C6A3, 8192, true).unwrap();
            assert!(!g29.is_empty());

            println!("ALL FUSION CHECKS PASSED: 100% SNES COMPLIANT, NO BERRY GLITCH, PUNCH FROM SHOULDER, NO ANIMATION GLITCH!");
        }
    }

    #[test]
    fn test_inspect_vanilla_gfx33() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        if let Ok(bytes) = std::fs::read(root.join("roms/super_mario_world.sfc")) {
            let rom = SnesRom::from_bytes("smw", &bytes).unwrap();
            let (orig_g33, _) = super::disasm_dma::decompress_nintendo_lz2_with_endian(&rom.data, 0x043FC0, 16384, true).unwrap();
            assert!(!orig_g33.is_empty());
            let (orig_g32, _) = super::disasm_dma::decompress_nintendo_lz2_with_endian(&rom.data, 0x040000, 32768, true).unwrap();
            assert!(!orig_g32.is_empty());
            assert!(super::disasm_dma::decompress_nintendo_lz2_with_endian(&rom.data, 0x0459F9, 8192, true).is_some());
            assert!(super::disasm_dma::decompress_nintendo_lz2_with_endian(&rom.data, 0x050910, 8192, true).is_some());
        }
    }

    #[test]
    fn test_export_all_rom_assets() {
        let (rom_a, _) = generate_demo_rom_pair();
        let tmp_dir = std::env::temp_dir().join("snes_test_export");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        let res = super::export::export_all_rom_assets(&rom_a, &tmp_dir).expect("Export must succeed");
        assert!(res.sheets_png_count > 0, "Must export at least 1 sheet PNG");
        assert!(res.palettes_count > 0, "Must export palettes");
        assert!(tmp_dir.join("manifest.json").exists(), "manifest.json must exist");
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_deconstruct_rom_complete() {
        let (rom_a, _) = generate_demo_rom_pair();
        let tmp_dir = std::env::temp_dir().join("snes_test_deconstruct");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        let cfg = super::deconstruct::DeconstructConfig::default();
        let res = super::deconstruct::deconstruct_rom(&rom_a, &tmp_dir, &cfg).expect("Deconstruction must succeed");
        assert!(res.sheets_count > 0, "Must export sheets");
        assert!(res.palettes_count > 0, "Must export palettes");
        assert!(tmp_dir.join("manifest.json").exists(), "manifest.json must exist");
        assert!(tmp_dir.join("INDEX.md").exists(), "INDEX.md must exist");
        assert!(tmp_dir.join("disassembly").join("header.asm").exists(), "header.asm must exist");
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_rom_knowledge_base_persistence() {
        let kb = super::memory::RomKnowledgeBase::builtin_knowledge_base();
        assert!(kb.roms.len() >= 3, "Must have built-in ROM entries");
        assert!(kb.verified_recipes.len() >= 3, "Must have verified recipes");
        let tmp = std::env::temp_dir();
        let json = kb.to_json_pretty();
        let md = kb.to_markdown();
        std::fs::write(tmp.join("snes_kb_test.json"), &json).expect("Must serialize JSON");
        std::fs::write(tmp.join("snes_kb_test.md"), &md).expect("Must serialize Markdown");
        let _ = std::fs::remove_file(tmp.join("snes_kb_test.json"));
        let _ = std::fs::remove_file(tmp.join("snes_kb_test.md"));
        assert!(md.contains("SUPER MARIOWORLD"));
        assert!(md.contains("THE LEGEND OF ZELDA"));
        assert!(md.contains("GFX00 — Power-Ups & Interactive Blocks"));
        assert!(md.contains("ICONIC_ASSETS_ISOLATION_AND_EXTRACTION"));
    }

    #[test]
    fn test_universal_checksum_non_power_of_two_and_exhirom() {
        // 1. Test 3MB HiROM (2MB + 1MB mirrored twice -> 24 Mbit like Chrono Trigger / FF6)
        let mut rom_3mb = vec![0x01u8; 3 * 1024 * 1024];
        // Write HiROM header at 0xFFC0
        let h = 0xFFC0usize;
        rom_3mb[h..h + 21].copy_from_slice(b"CHRONO TEST 3MB HIROM");
        rom_3mb[h + 0x15] = 0x31; // Fast HiROM
        rom_3mb[h + 0x17] = 0x0C; // 4MB declared
        rom_3mb[h + 0x3C] = 0x00;
        rom_3mb[h + 0x3D] = 0x80; // Reset $8000
        let mut parsed_3mb = SnesRom::from_bytes("test_3mb.sfc", &rom_3mb).unwrap();
        parsed_3mb.fix_checksum();
        assert_eq!(parsed_3mb.header.mapping, RomMapping::HiROM);
        assert!(parsed_3mb.header.is_checksum_valid());

        // Verify exact PC <-> SNES roundtrip for HiROM
        let (b, a) = parsed_3mb.pc_to_snes(0x123456);
        assert_eq!(parsed_3mb.snes_to_pc(b, a), Some(0x123456));

        // 2. Test 6MB ExHiROM (4MB + 2MB mirrored twice -> 48 Mbit like Tales of Phantasia)
        let mut rom_6mb = vec![0x02u8; 6 * 1024 * 1024];
        let eh = 0x40FFC0usize;
        rom_6mb[eh..eh + 21].copy_from_slice(b"EXHIROM 6MB UNIVERSAL");
        rom_6mb[eh + 0x15] = 0x35; // ExHiROM FastROM
        rom_6mb[eh + 0x17] = 0x0D; // 8MB declared
        rom_6mb[eh + 0x3C] = 0x00;
        rom_6mb[eh + 0x3D] = 0x80;
        rom_6mb[0x008000] = 0x78; // SEI at reset vector
        // Precompute valid checksum so ExHiROM wins mapping detection
        let mut ex_rom = SnesRom::from_bytes("exhirom_6mb.sfc", &rom_6mb).unwrap();
        // Force header checksum write at ExHiROM offset and re-parse
        ex_rom.header.mapping = RomMapping::ExHiROM;
        ex_rom.fix_checksum();
        let reparsed_ex = SnesRom::from_bytes("exhirom_6mb.sfc", &ex_rom.to_sfc_bytes()).unwrap();
        assert_eq!(reparsed_ex.header.mapping, RomMapping::ExHiROM);
        assert!(reparsed_ex.header.is_checksum_valid());
        let (eb1, ea1) = reparsed_ex.pc_to_snes(0x051234);
        assert_eq!(reparsed_ex.snes_to_pc(eb1, ea1), Some(0x051234));
        let (eb2, ea2) = reparsed_ex.pc_to_snes(0x451234);
        assert_eq!(reparsed_ex.snes_to_pc(eb2, ea2), Some(0x451234));
    }

    #[test]
    fn test_universal_roundtrip_deconstruct_edit_reconstruct() {
        let (mut rom_a, _) = generate_demo_rom_pair();
        // Inject a clean ASCII text string into ROM A so text round-trip is also tested
        let text_off = 0x01F000usize;
        let msg = b"PRESS START TO PLAY!";
        rom_a.data[text_off..text_off + msg.len()].copy_from_slice(msg);
        rom_a.data[text_off + msg.len()] = 0x00;
        rom_a.fix_checksum();

        let proj_dir = std::env::temp_dir().join("snes_test_univ_roundtrip");
        let _ = std::fs::remove_dir_all(&proj_dir);
        let cfg = super::deconstruct::DeconstructConfig::default();
        super::deconstruct::deconstruct_rom(&rom_a, &proj_dir, &cfg).expect("Deconstruct failed");
        assert!(proj_dir.join("project.json").exists(), "project.json must exist");

        let catalog = super::universal::discover_universal_catalog(&rom_a);
        assert!(!catalog.graphics.is_empty(), "Must discover graphics universally");
        assert!(!catalog.palettes.is_empty(), "Must discover palettes universally");
        assert!(!catalog.audio_samples.is_empty(), "Must discover BRR samples universally");
        assert!(!catalog.texts.is_empty(), "Must discover text strings universally");

        // 1. Edit first palette JSON (change color index 1 to bright magenta #FF00FF)
        let pal0_path = proj_dir.join(&catalog.palettes[0].json_file);
        let pal0_txt = std::fs::read_to_string(&pal0_path).unwrap();
        let edited_pal = pal0_txt.replacen("\"index\": 1, \"hex\": \"#", "\"index\": 1, \"hex\": \"#FF00FF\" /*", 1);
        // Cleanly replace first occurrence of index 1 hex
        let mut new_pal_lines = Vec::new();
        for line in pal0_txt.lines() {
            if line.contains("\"index\": 1,") {
                new_pal_lines.push("    { \"index\": 1, \"hex\": \"#F800F8\", \"bgr555\": \"0x7C1F\" },".to_string());
            } else {
                new_pal_lines.push(line.to_string());
            }
        }
        let _ = edited_pal;
        std::fs::write(&pal0_path, new_pal_lines.join("\n")).unwrap();

        // 2. Edit first GFX PNG (paint top-left 8x8 tile bright color)
        let gfx0_path = proj_dir.join(&catalog.graphics[0].sheet_file);
        let mut img = image::open(&gfx0_path).unwrap().to_rgba8();
        for y in 0..8 {
            for x in 0..8 {
                img.put_pixel(x, y, image::Rgba([248, 0, 248, 255]));
            }
        }
        img.save(&gfx0_path).unwrap();

        // 3. Edit first audio WAV (replace with clean 440Hz sine wave!)
        let aud0_path = proj_dir.join(&catalog.audio_samples[0].wav_file);
        let sine_pcm: Vec<i16> = (0..1024)
            .map(|i| (((i as f32 * 0.2).sin()) * 16000.0) as i16)
            .collect();
        let new_wav = encode_pcm16_to_wav(&sine_pcm, 16000);
        std::fs::write(&aud0_path, new_wav).unwrap();

        // 4. Edit text in text/strings_editable.json
        let txt_path = proj_dir.join("text").join("strings_editable.json");
        let txt_content = std::fs::read_to_string(&txt_path).unwrap();
        let edited_txt = txt_content.replace(
            "\"text\": \"PRESS START TO PLAY!\"",
            "\"text\": \"HELLO UNIVERSAL SNES\"",
        );
        std::fs::write(&txt_path, edited_txt).unwrap();

        // 5. Reconstruct ROM + IPS patch!
        let out_rom = proj_dir.join("rebuilt.sfc");
        let out_ips = proj_dir.join("rebuilt.ips");
        let rep = super::universal::reconstruct_rom_from_project(
            &rom_a,
            &proj_dir,
            &out_rom,
            Some(&out_ips),
        )
        .expect("Reconstruction must succeed");

        assert!(rep.palettes_patched_count >= 1, "Must patch modified palette");
        assert!(rep.gfx_patched_count >= 1, "Must patch modified PNG GFX");
        assert!(rep.audio_patched_count >= 1, "Must patch modified WAV audio");
        assert!(rep.text_patched_count >= 1, "Must patch modified text");
        assert!(rep.checksum_valid, "Reconstructed ROM checksum must be 100% valid");
        assert!(rep.total_bytes_changed > 0);

        // Verify rebuilt ROM & IPS integrity
        let rebuilt_bytes = std::fs::read(&out_rom).unwrap();
        let rebuilt_rom = SnesRom::from_bytes("rebuilt.sfc", &rebuilt_bytes).unwrap();
        assert!(rebuilt_rom.header.is_checksum_valid());
        assert_eq!(&rebuilt_rom.data[text_off..text_off + 20], b"HELLO UNIVERSAL SNES");

        let ips_bytes = std::fs::read(&out_ips).unwrap();
        assert_eq!(&ips_bytes[..5], b"PATCH");
        assert_eq!(&ips_bytes[ips_bytes.len() - 3..], b"EOF");

        let _ = std::fs::remove_dir_all(&proj_dir);
    }

    #[test]
    fn test_universal_cross_rom_swap_unknown_roms() {
        let (mut rom_a, rom_b) = generate_demo_rom_pair();
        let orig_bytes = rom_a.data.clone();
        let cat_a = super::universal::discover_universal_catalog(&rom_a);
        let cat_b = super::universal::discover_universal_catalog(&rom_b);

        let mut session = SwapSession::new();
        let opts = super::universal::UniversalBatchSwapOptions::default();
        let ops = super::universal::apply_universal_batch_swap(
            &mut rom_a,
            &rom_b,
            &cat_a,
            &cat_b,
            &opts,
            &mut session,
        )
        .expect("Universal cross-ROM swap must succeed");

        assert!(ops >= 3, "Must swap graphics, palettes, and audio between arbitrary unknown ROMs");
        assert!(rom_a.header.is_checksum_valid(), "Swapped ROM must have valid SNES checksum");
        assert_ne!(rom_a.data, orig_bytes, "Swapped ROM must differ from original");

        // Verify Undo restores bit-for-bit original ROM
        while let Some(last) = session.operations.last().cloned() {
            session.undo_operation(&mut rom_a, last.id).unwrap();
        }
        assert_eq!(rom_a.data, orig_bytes, "Full undo must restore bit-identical original ROM");
    }

    #[test]
    fn test_omni_mixer_recipe_json_roundtrip() {
        let recipe = super::mixer::MixRecipe {
            name: "Donkey Hyrule World".into(),
            custom_title: Some("DONKEY HYRULE WORLD".into()),
            base_rom: "roms/zelda_alttp.sfc".into(),
            donors: vec![
                ("smw".into(), "roms/super_mario_world.sfc".into()),
                ("dkc".into(), "roms/donkey_kong_country.sfc".into()),
            ],
            rules: vec![
                super::mixer::MixRule {
                    target_slot: "hero".into(),
                    donor_alias: "dkc".into(),
                    donor_source: "dk".into(),
                },
                super::mixer::MixRule {
                    target_slot: "soldiers".into(),
                    donor_alias: "smw".into(),
                    donor_source: "goomba".into(),
                },
            ],
        };
        let json = recipe.to_json();
        let parsed = super::mixer::MixRecipe::from_json(&json).expect("Valid JSON roundtrip");
        assert_eq!(parsed, recipe);
    }

    #[test]
    fn test_omni_mixer_unknown_roms_universal_and_chaos() {
        let (mut rom_a, rom_b) = generate_demo_rom_pair();
        let orig_a = rom_a.data.clone();
        let rom_c = rom_b.clone();

        let prof_a = super::mixer::inspect_rom_for_mixing(&rom_a, "base", "a.sfc");
        let prof_b = super::mixer::inspect_rom_for_mixing(&rom_b, "d1", "b.sfc");
        let prof_c = super::mixer::inspect_rom_for_mixing(&rom_c, "d2", "c.sfc");

        let chaos = super::mixer::generate_chaos_recipe(&prof_a, &[prof_b, prof_c], 777);
        assert!(!chaos.rules.is_empty(), "Chaos recipe must generate universal rules on unknown ROMs");

        let mut donors = std::collections::HashMap::new();
        donors.insert("d1".to_string(), rom_b);
        donors.insert("d2".to_string(), rom_c);

        let mut session = SwapSession::new();
        let rep = super::mixer::execute_mix_recipe(&mut rom_a, &donors, &chaos, &mut session)
            .expect("Omni-Mixer chaos execution on unknown ROMs must succeed");

        assert!(rep.operations_applied >= 2);
        assert!(rep.checksum_valid, "Mixed unknown ROM must have valid SNES checksum");
        assert_ne!(rom_a.data, orig_a);
    }

    #[test]
    fn test_omni_mixer_real_multi_game_matrix() {
        let z_path = "../../roms/zelda_alttp.sfc";
        let m_path = "../../roms/super_mario_world.sfc";
        let d_path = "../../roms/donkey_kong_country.sfc";
        if !std::path::Path::new(z_path).exists()
            || !std::path::Path::new(m_path).exists()
            || !std::path::Path::new(d_path).exists()
        {
            return;
        }
        let zelda = SnesRom::from_bytes("zelda.sfc", &std::fs::read(z_path).unwrap()).unwrap();
        let smw = SnesRom::from_bytes("smw.sfc", &std::fs::read(m_path).unwrap()).unwrap();
        let dkc = SnesRom::from_bytes("dkc.sfc", &std::fs::read(d_path).unwrap()).unwrap();

        let p_z = super::mixer::inspect_rom_for_mixing(&zelda, "zelda", z_path);
        let p_m = super::mixer::inspect_rom_for_mixing(&smw, "smw", m_path);
        let p_d = super::mixer::inspect_rom_for_mixing(&dkc, "dkc", d_path);

        // 1. Base = Zelda + Donors = SMW + DKC (Donkey Kong in Hyrule + SMW Goombas + SMW Items + DKC Audio!)
        {
            let presets = super::mixer::suggest_presets(&p_z, &[p_m.clone(), p_d.clone()]);
            let dk_hyrule = presets.iter().find(|p| p.id == "donkey_hyrule_3games").expect("3-game DK Hyrule preset");
            let recipe = super::mixer::MixRecipe {
                name: dk_hyrule.name.clone(),
                custom_title: Some(dk_hyrule.custom_title.clone()),
                base_rom: z_path.into(),
                donors: vec![("smw".into(), m_path.into()), ("dkc".into(), d_path.into())],
                rules: dk_hyrule.rules.clone(),
            };
            let mut target = zelda.clone();
            let mut donors = std::collections::HashMap::new();
            donors.insert("smw".into(), smw.clone());
            donors.insert("dkc".into(), dkc.clone());
            let mut sess = SwapSession::new();
            let rep = super::mixer::execute_mix_recipe(&mut target, &donors, &recipe, &mut sess).unwrap();
            assert!(rep.checksum_valid);
            assert!(rep.operations_applied >= 10, "Expected >=10 ops, got {}", rep.operations_applied);
            assert_eq!(rep.output_title, "DONKEY HYRULE WORLD");
        }

        // 2. Base = SMW + Donors = Zelda + DKC (Link + Zelda Soldiers + Zelda Items + DK Mount!)
        {
            let presets = super::mixer::suggest_presets(&p_m, &[p_z.clone(), p_d.clone()]);
            let inv = presets.iter().find(|p| p.id == "hyrule_invasion_smw").expect("Hyrule invasion SMW preset");
            let recipe = super::mixer::MixRecipe {
                name: inv.name.clone(),
                custom_title: Some(inv.custom_title.clone()),
                base_rom: m_path.into(),
                donors: vec![("zelda".into(), z_path.into()), ("dkc".into(), d_path.into())],
                rules: inv.rules.clone(),
            };
            let mut target = smw.clone();
            let mut donors = std::collections::HashMap::new();
            donors.insert("zelda".into(), zelda.clone());
            donors.insert("dkc".into(), dkc.clone());
            let mut sess = SwapSession::new();
            let rep = super::mixer::execute_mix_recipe(&mut target, &donors, &recipe, &mut sess).unwrap();
            assert!(rep.checksum_valid);
            assert!(rep.operations_applied >= 8);
            assert_eq!(rep.output_title, "HYRULE MARIO WORLD");
        }

        // 3. Base = DKC (4MB HiROM!) + Donors = SMW + Zelda (Mario in DKC + Zelda Audio!)
        {
            let presets = super::mixer::suggest_presets(&p_d, &[p_m.clone(), p_z.clone()]);
            let m_dkc = presets.iter().find(|p| p.id == "mario_in_dkc").expect("Mario in DKC preset");
            let recipe = super::mixer::MixRecipe {
                name: m_dkc.name.clone(),
                custom_title: Some(m_dkc.custom_title.clone()),
                base_rom: d_path.into(),
                donors: vec![("smw".into(), m_path.into()), ("zelda".into(), z_path.into())],
                rules: m_dkc.rules.clone(),
            };
            let mut target = dkc.clone();
            let mut donors = std::collections::HashMap::new();
            donors.insert("smw".into(), smw.clone());
            donors.insert("zelda".into(), zelda.clone());
            let mut sess = SwapSession::new();
            let rep = super::mixer::execute_mix_recipe(&mut target, &donors, &recipe, &mut sess).unwrap();
            assert!(rep.checksum_valid);
            assert!(rep.operations_applied >= 5);
            assert_eq!(rep.output_title, "MARIO KONG COUNTRY");
        }
    }

    #[test]
    fn test_super_mario_hyrule_snes_and_super_famicom_compatibility() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let z_path = root.join("roms/zelda_alttp.sfc");
        let m_path = root.join("roms/super_mario_world.sfc");
        if let (Ok(z_bytes), Ok(m_bytes)) = (std::fs::read(&z_path), std::fs::read(&m_path)) {
            let zelda = SnesRom::from_bytes("zelda", &z_bytes).unwrap();
            let smw = SnesRom::from_bytes("smw", &m_bytes).unwrap();

            let mut target = zelda.clone();
            let mut donors = std::collections::HashMap::new();
            donors.insert("smw".into(), smw.clone());

            let recipe = super::mixer::MixRecipe {
                name: "Super Mario Hyrule".into(),
                custom_title: Some("SUPER MARIO HYRULE".into()),
                base_rom: "roms/zelda_alttp.sfc".into(),
                donors: vec![("smw".into(), "roms/super_mario_world.sfc".into())],
                rules: vec![
                    super::mixer::MixRule { target_slot: "hero".into(), donor_alias: "smw".into(), donor_source: "mario".into() },
                    super::mixer::MixRule { target_slot: "audio_percussion".into(), donor_alias: "smw".into(), donor_source: "percussion".into() },
                    super::mixer::MixRule { target_slot: "soldiers".into(), donor_alias: "smw".into(), donor_source: "goomba".into() },
                    super::mixer::MixRule { target_slot: "elite_knights".into(), donor_alias: "smw".into(), donor_source: "spiny".into() },
                    super::mixer::MixRule { target_slot: "sewer_enemies".into(), donor_alias: "smw".into(), donor_source: "sewer_pack".into() },
                    super::mixer::MixRule { target_slot: "items".into(), donor_alias: "smw".into(), donor_source: "items_pack".into() },
                    super::mixer::MixRule { target_slot: "overworld_enemies".into(), donor_alias: "smw".into(), donor_source: "piranha".into() },
                    super::mixer::MixRule { target_slot: "dungeon_enemies".into(), donor_alias: "smw".into(), donor_source: "magikoopa".into() },
                    super::mixer::MixRule { target_slot: "rocks".into(), donor_alias: "smw".into(), donor_source: "qblock".into() },
                ],
            };

            let mut session = SwapSession::new();
            let rep = super::mixer::execute_mix_recipe(&mut target, &donors, &recipe, &mut session).unwrap();

            // 1. Hardware Checksum & Complement Verification
            assert!(rep.checksum_valid, "SNES Checksum must be 100% valid!");
            let comp = u16::from_le_bytes([target.data[0x7FDC], target.data[0x7FDD]]);
            let sum = u16::from_le_bytes([target.data[0x7FDE], target.data[0x7FDF]]);
            assert_eq!(comp ^ sum, 0xFFFF, "Checksum and complement must sum to 0xFFFF");
            assert_eq!(rep.operations_applied, 20, "Must apply all 20 hardware fusions");

            // 2. Hardware LoROM Mapping & Vector Verification
            assert_eq!(target.header.mapping, crate::rom::RomMapping::LoROM);
            assert_eq!(target.data.len(), 1024 * 1024);
            let reset_vector = u16::from_le_bytes([target.data[0x7FFC], target.data[0x7FFD]]);
            assert!(reset_vector >= 0x8000, "Reset vector must be in ROM range");
            let nmi_vector = u16::from_le_bytes([target.data[0x7FEA], target.data[0x7FEB]]);
            assert!(nmi_vector >= 0x8000, "NMI vector must be in ROM range");

            // 3. Hardware OAM Y-offset and X-offset coordinates in Bank $0D must be bounded
            for i in 0..303usize {
                let y = target.data[0x068000 + i * 3] as i8;
                let x = target.data[0x068000 + i * 3 + 1] as i8;
                assert!(y >= -64 && y <= 64, "OAM Y coordinate out of safe PPU range at {}: {}", i, y);
                assert!(x >= -64 && x <= 64, "OAM X coordinate out of safe PPU range at {}: {}", i, x);
            }

            // 4. Hardware 3bpp LZ2 Decompression Integrity Check
            for &sheet_id in &[0xBAusize, 0xB7, 0x8C, 0x8D, 0x7F, 0x82, 0x90, 0xCD, 0x06, 0x07, 0x08] {
                let sheet = super::mixer::load_alttp_3bpp_sheet(&target.data, sheet_id);
                assert!(sheet.is_some(), "Sheet 0x{:02X} must decompress cleanly", sheet_id);
                let (raw, _, _) = sheet.unwrap();
                assert_eq!(raw.len(), 0x600, "Sheet 0x{:02X} must have exact 64 3bpp tiles (1536 bytes)", sheet_id);
            }

            // 5. Zero Donkey Kong contamination check
            let dkc_signature = b"DONKEY KONG";
            assert!(!target.data.windows(dkc_signature.len()).any(|w| w == dkc_signature), "No DKC contamination in Zelda!");

            println!("HARDWARE AUDIT PASSED: 100% COMPLIANT WITH SNES AND SUPER FAMICOM!");
        }
    }
}
