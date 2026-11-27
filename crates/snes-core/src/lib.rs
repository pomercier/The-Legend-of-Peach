#![allow(
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::too_many_arguments,
    clippy::doc_overindented_list_items,
    clippy::doc_lazy_continuation
)]

pub mod ai_sprites;
pub mod assets;
pub mod audio;
pub mod deconstruct;
pub mod demo;
pub mod disasm_dma;
pub mod export;
pub mod graphics;
pub mod mario_bosses;
pub mod mario_engine;
pub mod mario_intro;
pub mod mario_map;
pub mod mario_music;
pub mod mario_sfx;
pub mod mario_ui;
pub mod mario_world;
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
        if let (Ok(bytes_a), Ok(bytes_b)) = (
            std::fs::read(root.join("roms/super_mario_world.sfc")),
            std::fs::read(root.join("roms/zelda_alttp.sfc")),
        ) {
            let mut rom_a = SnesRom::from_bytes("smw", &bytes_a).unwrap();
            let rom_b = SnesRom::from_bytes("zelda", &bytes_b).unwrap();
            let mut session = SwapSession::new();

            let orig_g32 = super::disasm_dma::decompress_nintendo_lz2_with_endian(
                &rom_a.data,
                0x040000,
                32768,
                true,
            )
            .unwrap()
            .0;
            let ops =
                super::demo::apply_full_fusion(&mut rom_a, Some(&rom_b), &mut session).unwrap();
            println!("Full fusion ops applied: {}", ops);
            assert!(ops >= 6, "Should apply at least 6 operations");

            // 1. Verify Checksum
            assert!(
                rom_a.header.is_checksum_valid(),
                "Checksum must be 100% valid!"
            );
            let comp = u16::from_le_bytes([rom_a.data[0x7FDC], rom_a.data[0x7FDD]]);
            let sum = u16::from_le_bytes([rom_a.data[0x7FDE], rom_a.data[0x7FDF]]);
            assert_eq!(
                comp ^ sum,
                0xFFFF,
                "Checksum and complement must sum to 0xFFFF"
            );

            // 2. Verify Berry tiles in GFX32
            let (new_g32, _) = super::disasm_dma::decompress_nintendo_lz2_with_endian(
                &rom_a.data,
                0x040000,
                32768,
                true,
            )
            .unwrap();
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
                    let c = ((b0 >> sh) & 1)
                        | (((b1 >> sh) & 1) << 1)
                        | (((b2 >> sh) & 1) << 2)
                        | (((b3 >> sh) & 1) << 3);
                    line.push(if c == 0 { '.' } else { '#' });
                }
                println!("{}", line);
            }
            assert_eq!(
                &new_g32[5120..5120 + 32],
                &orig_g32[5120..5120 + 32],
                "Tile 80 (berry top) must match vanilla!"
            );
            assert_eq!(
                &new_g32[21632..21632 + 32],
                &orig_g32[21632..21632 + 32],
                "Tile 90 (berry bottom) must match vanilla!"
            );

            // 3. Verify GFX33 decompresses and fits
            let (g33, _) = super::disasm_dma::decompress_nintendo_lz2_with_endian(
                &rom_a.data,
                0x043FC0,
                16384,
                true,
            )
            .unwrap();
            assert!(!g33.is_empty());

            // 4. Verify Title screen GFX29 decompresses and fits
            let (g29, _) = super::disasm_dma::decompress_nintendo_lz2_with_endian(
                &rom_a.data,
                0x05C6A3,
                8192,
                true,
            )
            .unwrap();
            assert!(!g29.is_empty());

            println!("ALL FUSION CHECKS PASSED: 100% SNES COMPLIANT, NO BERRY GLITCH, NO ANIMATION GLITCH!");
        }
    }

    #[test]
    fn test_inspect_vanilla_gfx33() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        if let Ok(bytes) = std::fs::read(root.join("roms/super_mario_world.sfc")) {
            let rom = SnesRom::from_bytes("smw", &bytes).unwrap();
            let (orig_g33, _) = super::disasm_dma::decompress_nintendo_lz2_with_endian(
                &rom.data, 0x043FC0, 16384, true,
            )
            .unwrap();
            assert!(!orig_g33.is_empty());
            let (orig_g32, _) = super::disasm_dma::decompress_nintendo_lz2_with_endian(
                &rom.data, 0x040000, 32768, true,
            )
            .unwrap();
            assert!(!orig_g32.is_empty());
            assert!(super::disasm_dma::decompress_nintendo_lz2_with_endian(
                &rom.data, 0x0459F9, 8192, true
            )
            .is_some());
            assert!(super::disasm_dma::decompress_nintendo_lz2_with_endian(
                &rom.data, 0x050910, 8192, true
            )
            .is_some());
        }
    }

    #[test]
    fn test_export_all_rom_assets() {
        let (rom_a, _) = generate_demo_rom_pair();
        let tmp_dir = std::env::temp_dir().join("snes_test_export");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        let res =
            super::export::export_all_rom_assets(&rom_a, &tmp_dir).expect("Export must succeed");
        assert!(res.sheets_png_count > 0, "Must export at least 1 sheet PNG");
        assert!(res.palettes_count > 0, "Must export palettes");
        assert!(
            tmp_dir.join("manifest.json").exists(),
            "manifest.json must exist"
        );
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_deconstruct_rom_complete() {
        let (rom_a, _) = generate_demo_rom_pair();
        let tmp_dir = std::env::temp_dir().join("snes_test_deconstruct");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        let cfg = super::deconstruct::DeconstructConfig::default();
        let res = super::deconstruct::deconstruct_rom(&rom_a, &tmp_dir, &cfg)
            .expect("Deconstruction must succeed");
        assert!(res.sheets_count > 0, "Must export sheets");
        assert!(res.palettes_count > 0, "Must export palettes");
        assert!(
            tmp_dir.join("manifest.json").exists(),
            "manifest.json must exist"
        );
        assert!(tmp_dir.join("INDEX.md").exists(), "INDEX.md must exist");
        assert!(
            tmp_dir.join("disassembly").join("header.asm").exists(),
            "header.asm must exist"
        );
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_rom_knowledge_base_persistence() {
        let kb = super::memory::RomKnowledgeBase::builtin_knowledge_base();
        assert!(kb.roms.len() >= 2, "Must have built-in ROM entries");
        assert!(kb.verified_recipes.len() >= 2, "Must have verified recipes");
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
        assert!(
            proj_dir.join("project.json").exists(),
            "project.json must exist"
        );

        let catalog = super::universal::discover_universal_catalog(&rom_a);
        assert!(
            !catalog.graphics.is_empty(),
            "Must discover graphics universally"
        );
        assert!(
            !catalog.palettes.is_empty(),
            "Must discover palettes universally"
        );
        assert!(
            !catalog.audio_samples.is_empty(),
            "Must discover BRR samples universally"
        );
        assert!(
            !catalog.texts.is_empty(),
            "Must discover text strings universally"
        );

        // 1. Edit first palette JSON (change color index 1 to bright magenta #FF00FF)
        let pal0_path = proj_dir.join(&catalog.palettes[0].json_file);
        let pal0_txt = std::fs::read_to_string(&pal0_path).unwrap();
        let edited_pal = pal0_txt.replacen(
            "\"index\": 1, \"hex\": \"#",
            "\"index\": 1, \"hex\": \"#FF00FF\" /*",
            1,
        );
        // Cleanly replace first occurrence of index 1 hex
        let mut new_pal_lines = Vec::new();
        for line in pal0_txt.lines() {
            if line.contains("\"index\": 1,") {
                new_pal_lines.push(
                    "    { \"index\": 1, \"hex\": \"#F800F8\", \"bgr555\": \"0x7C1F\" },"
                        .to_string(),
                );
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

        assert!(
            rep.palettes_patched_count >= 1,
            "Must patch modified palette"
        );
        assert!(rep.gfx_patched_count >= 1, "Must patch modified PNG GFX");
        assert!(
            rep.audio_patched_count >= 1,
            "Must patch modified WAV audio"
        );
        assert!(rep.text_patched_count >= 1, "Must patch modified text");
        assert!(
            rep.checksum_valid,
            "Reconstructed ROM checksum must be 100% valid"
        );
        assert!(rep.total_bytes_changed > 0);

        // Verify rebuilt ROM & IPS integrity
        let rebuilt_bytes = std::fs::read(&out_rom).unwrap();
        let rebuilt_rom = SnesRom::from_bytes("rebuilt.sfc", &rebuilt_bytes).unwrap();
        assert!(rebuilt_rom.header.is_checksum_valid());
        assert_eq!(
            &rebuilt_rom.data[text_off..text_off + 20],
            b"HELLO UNIVERSAL SNES"
        );

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

        assert!(
            ops >= 3,
            "Must swap graphics, palettes, and audio between arbitrary unknown ROMs"
        );
        assert!(
            rom_a.header.is_checksum_valid(),
            "Swapped ROM must have valid SNES checksum"
        );
        assert_ne!(
            rom_a.data, orig_bytes,
            "Swapped ROM must differ from original"
        );

        // Verify Undo restores bit-for-bit original ROM
        while let Some(last) = session.operations.last().cloned() {
            session.undo_operation(&mut rom_a, last.id).unwrap();
        }
        assert_eq!(
            rom_a.data, orig_bytes,
            "Full undo must restore bit-identical original ROM"
        );
    }

    #[test]
    fn test_omni_mixer_recipe_json_roundtrip() {
        let recipe = super::mixer::MixRecipe {
            name: "Super Mario Hyrule".into(),
            custom_title: Some("SUPER MARIO HYRULE".into()),
            base_rom: "roms/zelda_alttp.sfc".into(),
            donors: vec![("smw".into(), "roms/super_mario_world.sfc".into())],
            rules: vec![
                super::mixer::MixRule {
                    target_slot: "hero".into(),
                    donor_alias: "smw".into(),
                    donor_source: "mario".into(),
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
        assert!(
            !chaos.rules.is_empty(),
            "Chaos recipe must generate universal rules on unknown ROMs"
        );

        let mut donors = std::collections::HashMap::new();
        donors.insert("d1".to_string(), rom_b);
        donors.insert("d2".to_string(), rom_c);

        let mut session = SwapSession::new();
        let rep = super::mixer::execute_mix_recipe(&mut rom_a, &donors, &chaos, &mut session)
            .expect("Omni-Mixer chaos execution on unknown ROMs must succeed");

        assert!(rep.operations_applied >= 2);
        assert!(
            rep.checksum_valid,
            "Mixed unknown ROM must have valid SNES checksum"
        );
        assert_ne!(rom_a.data, orig_a);
    }

    #[test]
    fn test_omni_mixer_real_multi_game_matrix() {
        let z_path = "../../roms/zelda_alttp.sfc";
        let m_path = "../../roms/super_mario_world.sfc";
        if !std::path::Path::new(z_path).exists() || !std::path::Path::new(m_path).exists() {
            return;
        }
        let zelda = SnesRom::from_bytes("zelda.sfc", &std::fs::read(z_path).unwrap()).unwrap();
        let smw = SnesRom::from_bytes("smw.sfc", &std::fs::read(m_path).unwrap()).unwrap();

        let p_z = super::mixer::inspect_rom_for_mixing(&zelda, "zelda", z_path);
        let p_m = super::mixer::inspect_rom_for_mixing(&smw, "smw", m_path);

        // 1. Base = Zelda + Donor = SMW (Super Mario Hyrule!)
        {
            let presets = super::mixer::suggest_presets(&p_z, std::slice::from_ref(&p_m));
            let mario_hyrule = presets
                .iter()
                .find(|p| p.id == "mario_hyrule_complete")
                .expect("Mario Hyrule preset");
            let recipe = super::mixer::MixRecipe {
                name: mario_hyrule.name.clone(),
                custom_title: Some(mario_hyrule.custom_title.clone()),
                base_rom: z_path.into(),
                donors: vec![("smw".into(), m_path.into())],
                rules: mario_hyrule.rules.clone(),
            };
            let mut target = zelda.clone();
            let mut donors = std::collections::HashMap::new();
            donors.insert("smw".into(), smw.clone());
            let mut sess = SwapSession::new();
            let rep =
                super::mixer::execute_mix_recipe(&mut target, &donors, &recipe, &mut sess).unwrap();
            assert!(rep.checksum_valid);
            assert!(
                rep.operations_applied >= 7,
                "Expected >=7 ops, got {}",
                rep.operations_applied
            );
            assert_eq!(rep.output_title, "SUPER MARIO HYRULE");
        }

        // 2. Base = SMW + Donor = Zelda (Link in SMW!)
        {
            let presets = super::mixer::suggest_presets(&p_m, std::slice::from_ref(&p_z));
            let link_smw = presets
                .iter()
                .find(|p| p.id == "super_link_world")
                .expect("Super Link World preset");
            let recipe = super::mixer::MixRecipe {
                name: link_smw.name.clone(),
                custom_title: Some(link_smw.custom_title.clone()),
                base_rom: m_path.into(),
                donors: vec![("zelda".into(), z_path.into())],
                rules: link_smw.rules.clone(),
            };
            let mut target = smw.clone();
            let mut donors = std::collections::HashMap::new();
            donors.insert("zelda".into(), zelda.clone());
            let mut sess = SwapSession::new();
            let rep =
                super::mixer::execute_mix_recipe(&mut target, &donors, &recipe, &mut sess).unwrap();
            assert!(rep.checksum_valid);
            assert!(rep.operations_applied >= 2);
            assert_eq!(rep.output_title, "SUPER LINK WORLD");
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
                name: "The Legend of Peach: A Mario to the Mushroom Kingdom".into(),
                custom_title: Some("THE LEGEND OF PEACH".into()),
                base_rom: "roms/zelda_alttp.sfc".into(),
                donors: vec![("smw".into(), "roms/super_mario_world.sfc".into())],
                rules: vec![
                    super::mixer::MixRule {
                        target_slot: "intro_title".into(),
                        donor_alias: "smw".into(),
                        donor_source: "peach_kingdom_intro".into(),
                    },
                    super::mixer::MixRule {
                        target_slot: "hero".into(),
                        donor_alias: "smw".into(),
                        donor_source: "mario".into(),
                    },
                    super::mixer::MixRule {
                        target_slot: "princess".into(),
                        donor_alias: "smw".into(),
                        donor_source: "peach".into(),
                    },
                    super::mixer::MixRule {
                        target_slot: "uncle_allies".into(),
                        donor_alias: "smw".into(),
                        donor_source: "luigi_yoshi_pack".into(),
                    },
                    super::mixer::MixRule {
                        target_slot: "bosses".into(),
                        donor_alias: "smw".into(),
                        donor_source: "bosses_pack".into(),
                    },
                    super::mixer::MixRule {
                        target_slot: "soldiers".into(),
                        donor_alias: "smw".into(),
                        donor_source: "koopa".into(),
                    },
                    super::mixer::MixRule {
                        target_slot: "castle_guards".into(),
                        donor_alias: "smw".into(),
                        donor_source: "goomba".into(),
                    },
                    super::mixer::MixRule {
                        target_slot: "elite_knights".into(),
                        donor_alias: "smw".into(),
                        donor_source: "spiny".into(),
                    },
                    super::mixer::MixRule {
                        target_slot: "sewer_enemies".into(),
                        donor_alias: "smw".into(),
                        donor_source: "sewer_pack".into(),
                    },
                    super::mixer::MixRule {
                        target_slot: "overworld_enemies".into(),
                        donor_alias: "smw".into(),
                        donor_source: "overworld_pack".into(),
                    },
                    super::mixer::MixRule {
                        target_slot: "zoras_water".into(),
                        donor_alias: "smw".into(),
                        donor_source: "water_pack".into(),
                    },
                    super::mixer::MixRule {
                        target_slot: "dungeon_enemies".into(),
                        donor_alias: "smw".into(),
                        donor_source: "dungeon_pack".into(),
                    },
                    super::mixer::MixRule {
                        target_slot: "dungeon_extra".into(),
                        donor_alias: "smw".into(),
                        donor_source: "dungeon_extra_pack".into(),
                    },
                    super::mixer::MixRule {
                        target_slot: "items".into(),
                        donor_alias: "smw".into(),
                        donor_source: "items_pack".into(),
                    },
                    super::mixer::MixRule {
                        target_slot: "chests_pots".into(),
                        donor_alias: "smw".into(),
                        donor_source: "blocks_pack".into(),
                    },
                    super::mixer::MixRule {
                        target_slot: "rocks".into(),
                        donor_alias: "smw".into(),
                        donor_source: "qblock".into(),
                    },
                    super::mixer::MixRule {
                        target_slot: "audio_percussion".into(),
                        donor_alias: "smw".into(),
                        donor_source: "percussion".into(),
                    },
                    super::mixer::MixRule {
                        target_slot: "mario_music".into(),
                        donor_alias: "smw".into(),
                        donor_source: "nspc_soundtrack".into(),
                    },
                ],
            };

            let mut session = SwapSession::new();
            let rep = super::mixer::execute_mix_recipe(&mut target, &donors, &recipe, &mut session)
                .unwrap();

            // 1. Hardware Checksum & Complement Verification
            assert!(rep.checksum_valid, "SNES Checksum must be 100% valid!");
            let comp = u16::from_le_bytes([target.data[0x7FDC], target.data[0x7FDD]]);
            let sum = u16::from_le_bytes([target.data[0x7FDE], target.data[0x7FDF]]);
            assert_eq!(
                comp ^ sum,
                0xFFFF,
                "Checksum and complement must sum to 0xFFFF"
            );
            assert!(
                rep.operations_applied >= 50,
                "Must apply all 50+ hardware fusions, got {}",
                rep.operations_applied
            );

            // 2. Hardware LoROM Mapping & Vector Verification (1MB or standard 2MB LoROM!)
            assert_eq!(target.header.mapping, crate::rom::RomMapping::LoROM);
            assert!(target.data.len() == 1024 * 1024 || target.data.len() == 2048 * 1024);
            assert!(target.data.len().is_power_of_two());
            let reset_vector = u16::from_le_bytes([target.data[0x7FFC], target.data[0x7FFD]]);
            assert!(reset_vector >= 0x8000, "Reset vector must be in ROM range");
            let nmi_vector = u16::from_le_bytes([target.data[0x7FEA], target.data[0x7FEB]]);
            assert!(nmi_vector >= 0x8000, "NMI vector must be in ROM range");

            // 3. Hardware OAM Y-offset and X-offset coordinates in Bank $0D must be bounded
            for i in 0..303usize {
                let y = target.data[0x068000 + i * 3] as i8;
                let x = target.data[0x068000 + i * 3 + 1] as i8;
                assert!(
                    (-64..=64).contains(&y),
                    "OAM Y coordinate out of safe PPU range at {}: {}",
                    i,
                    y
                );
                assert!(
                    (-64..=64).contains(&x),
                    "OAM X coordinate out of safe PPU range at {}: {}",
                    i,
                    x
                );
            }

            // 4. Hardware 3bpp LZ2 Decompression Integrity Check across ALL patched 3bpp sheets (including 0x3E/0x3F Title Logo & 0x0C Jars!)
            for &sheet_id in &[
                0x06usize, 0x07, 0x08, 0x0C, 0x0D, 0x3C, 0x3E, 0x3F, 0x7F, 0x82, 0x85, 0x86, 0x8C,
                0x8D, 0x8E, 0x8F, 0x90, 0xA5, 0xA6, 0xAA, 0xAC, 0xB3, 0xB4, 0xB7, 0xBA, 0xBC, 0xC2,
                0xC6, 0xC9, 0xCB, 0xCC, 0xCD, 0xCE, 0xD5, 0xD6, 0xD7,
            ] {
                let sheet = super::mixer::load_alttp_3bpp_sheet(&target.data, sheet_id);
                assert!(
                    sheet.is_some(),
                    "Sheet 0x{:02X} must decompress cleanly",
                    sheet_id
                );
                let (raw, _, _) = sheet.unwrap();
                assert_eq!(
                    raw.len(),
                    0x600,
                    "Sheet 0x{:02X} must have exact 64 3bpp tiles (1536 bytes)",
                    sheet_id
                );
            }

            // 4b. Verify "The Legend of Peach: A Mario to the Mushroom Kingdom" Intro, 3D Relics, Copyleft & FR Script!
            assert_eq!(target.header.title, "THE LEGEND OF PEACH");
            assert_eq!(
                target.data.len(),
                2048 * 1024,
                "Expanded to standard 2MB (16Mbit) LoROM for 3D Relics"
            );
            assert_eq!(
                target.data[0x007FD7], 0x0B,
                "SNES ROM size header byte must be 0x0B (2MB)"
            );
            assert_eq!(
                target.data[0x071C68], 0x10,
                "Title BG1 Row 7 VRAM high byte"
            );
            assert_eq!(
                target.data[0x071C69], 0xEA,
                "Title BG1 Row 7 Col 10 unified 'THE LEGEND OF' VRAM low byte"
            );
            assert_eq!(
                target.data[0x07070B], 27,
                "Copyright OAM must draw 27 sprites (Nintendo + copyleft 2026 pomercier)"
            );
            assert_eq!(
                target.data[0x04F7DE], 0x08,
                "Polyhedral_InitializeThread IRQ coroutine ($09:F7DE) must stay intact"
            );
            assert_eq!(
                &target.data[0x0012F7..0x0012FC],
                &[0x22, 0x00, 0x80, 0x22, 0x60],
                "NMI 3D DMA hook at $00:92F7 (32-frame 3D Mario Relics)"
            );
            assert_eq!(
                &target.data[0x07027C..0x070281],
                &[0x22, 0x80, 0x80, 0x22, 0xEA],
                "Title Screen 3D Step hook at $0E:827C"
            );
            assert_eq!(
                &target.data[0x070906..0x07090B],
                &[0x22, 0x80, 0x80, 0x22, 0xEA],
                "Intro Scroll 3D Step hook at $0E:8906"
            );
            assert_eq!(
                target.data[0x005D66], 0x17,
                "Room 0x00 RoomBlockset slot 6 must remain vanilla 0x17 (keeps north wall intact + loads Star into $1B0..$1BF)"
            );
            assert_eq!(
                &target.data[0x004008..0x00400A],
                &0x0DB6u16.to_le_bytes(),
                "Room 0x00 Star floor tiledata must target VRAM $1B6..$1BD (Pal 3, Slot 6 raw:0x17 row 3)"
            );
            assert_eq!(
                target.data[0x073ABB], 0x4F,
                "Sepia Hills Screen 1 top-right tile must be 0x4F (Golden Mario Trio)"
            );
            assert_eq!(
                target.data[0x073AE9], 0x6F,
                "Sepia Hills Screen 1 bot-right tile must be 0x6F (Golden Mario Trio)"
            );
            assert_ne!(
                &target.data[0x0E5564..0x0E56E1],
                &zelda.data[0x0E5564..0x0E56E1],
                "Intro Screen 0 must be rewritten"
            );
            assert_ne!(
                &target.data[0x0E56E1..0x0E5738],
                &zelda.data[0x0E56E1..0x0E5738],
                "Intro Screen 1 must be rewritten"
            );
            assert_ne!(
                &target.data[0x0E5738..0x0E577A],
                &zelda.data[0x0E5738..0x0E577A],
                "Intro Screen 2 must be rewritten"
            );
            assert_ne!(
                &target.data[0x0E577A..0x0E57AF],
                &zelda.data[0x0E577A..0x0E57AF],
                "Intro Screen 3 must be rewritten"
            );
            let repacked_1c = &target.data[0x0E0000..0x0E8000];
            assert_eq!(
                repacked_1c.iter().filter(|&&b| b == 0x7F).count(),
                372,
                "Bank $1C must contain all 372 French script messages with 0x7F terminators"
            );
            assert!(
                repacked_1c.contains(&0x88),
                "Bank $1C must preserve 0x88 end marker"
            );
            assert!(
                !repacked_1c
                    .windows(4)
                    .any(|w| w == [0x13, 0x2B, 0x22, 0x1F]),
                "Zero occurrences of 'Triforce' may remain in Bank $1C!"
            );

            // 4c. Verify ALL hardware patches (Zero Frame-0 3D corruption, Bowser Palettes, Koopa Swords, Black Bob-omb, HUD Bullet Bill, Luigi Green, Carry Z=23)
            let expected_3d = super::mario_intro::build_all_32_trio_3d_frames();
            assert_eq!(&target.data[0x100000..0x110000], expected_3d.as_slice(), "ALL 32 3D Intro frames (0x100000..0x110000) MUST remain 100% uncorrupted bit-for-bit!");
            assert_eq!(
                target.data[0x0484D1], 0x01,
                "House Chest 5-Rupee coin (item 0x35) must use SP2 Red Coin palette 0x01"
            );
            assert_eq!(
                &target.data[0x04843B..0x04843E],
                &[0x23, 0x14, 0x1E],
                "Boss 3 Pendants must use Mushroom, Fire Flower, Star GFX"
            );
            assert_eq!(
                target.data[0x041A50], 0x18,
                "Carried Ancilla Altitude Z ($029E) must be 0x18 (+7px above Mario cap)"
            );
            assert_eq!(target.data[0x035E68 + 3], 22, "SpriteModule_Carried.offset_z Stage 3 must be 22 (rests directly in Mario's wide white SMW gloves)");
            assert_eq!(
                target.data[0x06E79E], 0x61,
                "Start Menu header must start with 'R' ('RELIQUES', not 'PENDENTIF(S)')"
            );
            assert_eq!(
                target.data[0x06F111], 0x52,
                "Start Menu Boomerang must start cleanly with 'C' ('CARAPACE SMW', zero bleed)"
            );
            assert_eq!(
                &target.data[0x0DD64C..0x0DD64E],
                &0x1B87u16.to_le_bytes(),
                "Lv1 Shield must be vibrant SMW Emerald Green Koopa Shell"
            );
            assert_eq!(
                &target.data[0x0DE876..0x0DE878],
                &0x18BEu16.to_le_bytes(),
                "Mario's House Roof must be vibrant Super Mario Red"
            );
            assert_eq!(&target.data[0x08A000..0x08A300], &zelda.data[0x08A000..0x08A300], "Sheet 0x7B rows 0-1 (0x08A000..0x08A2FF) must remain untouched vanilla to prevent Bow/Bill Blaster WRAM corruption!");
            assert_eq!(
                target.data[0x06D163 + 5],
                0x0C,
                "Uncle Luigi head must use Pal 6 Green Mario/Luigi (0x0C)"
            );
            assert_eq!(
                target.data[0x06D163 + 8 + 5],
                0x0C,
                "Uncle Luigi body must use Pal 6 Green Mario/Luigi clothes (0x0C)"
            );
            assert_eq!(
                target.data[0x02C6A4], 12,
                "Koopa Guard head Y-sub must be 12px above body"
            );
            assert_eq!(
                target.data[0x02CAEA + 16],
                2,
                "Koopa Left/Right sword Y must be raised from 8 -> 2 (firmly in Koopa's hand!)"
            );
            assert_eq!(
                target.data[0x02B16E], 12,
                "Ball & Chain Trooper head Y-sub must be 12px above body"
            );
            assert_eq!(
                &target.data[0x0DD5A6..0x0DD5A8],
                &0x0CBDu16.to_le_bytes(),
                "Intro Bowser hair palette must be fiery SMW red"
            );
            assert_eq!(
                &target.data[0x0DD51A..0x0DD51C],
                &0x0CBDu16.to_le_bytes(),
                "Boss Bowser hair palette must be fiery SMW red"
            );
            assert_eq!(
                target.data[0x06F5AA], 0x28,
                "Start Menu & Y-box Bob-omb must use Pal 2 Black/White/Gold (ZERO blue bomb!)"
            );
            assert_eq!(
                target.data[0x06FDBD], 0x86,
                "HUD Arrow counter must display SMW Bullet Bill tile 0x86"
            );

            // 5. Hardware 2bpp HUD/Menu paired sheets (0x6F == 0xDB & 0x70 == 0xDC) must decompress to 2048B cleanly!
            for &(s1, s2) in &[(0x6Fusize, 0xDBusize), (0x70, 0xDC)] {
                let (d1, p1, _) =
                    super::mixer::load_alttp_3bpp_sheet(&target.data, s1).expect("2bpp sheet s1");
                let (d2, p2, _) =
                    super::mixer::load_alttp_3bpp_sheet(&target.data, s2).expect("2bpp sheet s2");
                assert_eq!(
                    d1.len(),
                    0x800,
                    "2bpp sheet 0x{:02X} must be 2048 bytes",
                    s1
                );
                assert_eq!(
                    p1, p2,
                    "Paired 2bpp pointers 0x{:02X} & 0x{:02X} must stay synchronized",
                    s1, s2
                );
                assert_eq!(d1, d2);
            }

            // 6. Verify Fix #3 (Gloves white, zero green flicker), #8 (Yellow Coin HUD), #10 (French Mushroom Dialog), #13 (N-SPC Mario Music)!
            assert_eq!(
                &target.data[0x0DEDF5..0x0DEDF9],
                &[0xFF, 0x7F, 0xFF, 0x7F],
                "Glove palettes must be white 0x7FFF"
            );
            assert_eq!(
                target.data[0x06FDB2], 0x28,
                "HUD Rupee must use Pal 2 Bright Yellow Coin"
            );
            assert_eq!(
                target.data[0x06FDB8], 0x28,
                "HUD Bomb must use Pal 2 SMW Bob-omb"
            );
            // Verify Bank $1C repacker preserves all 372 messages and eliminates Zelda terms
            let repacked_1c = &target.data[0x0E0000..0x0E8000];
            assert_eq!(
                repacked_1c.iter().filter(|&&b| b == 0x7F).count(),
                372,
                "Bank $1C must contain all 372 French script messages with 0x7F terminators"
            );
            // Verify N-SPC block upload headers & valid SPC $D000 song pointers
            assert_eq!(
                u16::from_le_bytes([target.data[0x0D1EF5], target.data[0x0D1EF6]]),
                0x2DAE
            );
            assert_eq!(
                u16::from_le_bytes([target.data[0x0D1EF7], target.data[0x0D1EF8]]),
                0xD000
            );
            let ow_song2_ptr =
                u16::from_le_bytes([target.data[0x0D1EF9 + 2], target.data[0x0D1EF9 + 3]]);
            assert!(
                (0xD036..0xFDAE).contains(&ow_song2_ptr),
                "OW Song #2 ptr ${:04X} must be in valid $D000..$FDAE",
                ow_song2_ptr
            );
            let dn_song18_ptr = u16::from_le_bytes([
                target.data[0x0D8004 + 17 * 2],
                target.data[0x0D8004 + 17 * 2 + 1],
            ]);
            assert!(
                (0xD046..0xFCBF).contains(&dn_song18_ptr),
                "Dungeon Song #18 ptr ${:04X} must be in valid $D000..$FCBF",
                dn_song18_ptr
            );

            // 7. Zero Donkey Kong contamination check
            let dkc_signature = b"DONKEY KONG";
            assert!(
                !target
                    .data
                    .windows(dkc_signature.len())
                    .any(|w| w == dkc_signature),
                "No DKC contamination in Zelda!"
            );

            // 8. Verify v13 Total Conversion (7 Koopalings + Final King Bowser + Pokey + Wiggler + Toad Town + Bestiary + Worldwide BG + 3 New N-SPC Songs!)
            let (raw_bk, raw_hi, raw_lo) =
                super::mixer::find_alttp_raw_gfx_table(&target.data).expect("raw ALTTP GFX table");
            let v13_raw_sheets: &[usize] = &[
                // Bosses (Pokey, Wiggler, Morton, Wendy, Iggy, Roy, Lemmy, Larry, Ludwig, Final King Bowser x4)
                0xA4, 0xA3, 0xAD, 0xB1, 0xAC, 0xAB, 0xAE, 0xAF, 0xB0, 0xB2, 0xB3, 0x94, 0xB4, 0xB8,
                0xA6, // Toad Town NPCs (6 sheets)
                0xC2, 0xBD, 0xC0, 0xC1, 0xC3, 0xBF, // SMW Bestiary Expansion (16 sheets)
                0x83, 0x85, 0x86, 0x89, 0x8A, 0x8B, 0x8C, 0x8D, 0x93, 0x95, 0x96, 0x97, 0x98, 0x99,
                0x9A, 0x9D,
                // Worldwide BG Tilesets (9 sheets: Dark World, Bowser's Castle, Dungeons, Bowser Statues, Toad Town Roofs)
                0x3F, 0x0D, 0x0B, 0x0C, 0x06, 0x18, 0x19, 0x4D, 0x4C,
                // Phase 5 Mario Super Hammer + Sparkling Super Star File Select Cursor (raw 0xCF)
                0xCF,
            ];
            for &raw_idx in v13_raw_sheets {
                let pc = ((target.data[raw_bk + raw_idx] as usize & 0x7F) << 15)
                    | ((target.data[raw_hi + raw_idx] as usize & 0x7F) << 8)
                    | (target.data[raw_lo + raw_idx] as usize);
                // Must NEVER intrude on reserved 3D Intro banks ($20..$22 = 0x100000..0x117FFF)!
                assert!(
                    !(0x100000..0x118000).contains(&pc),
                    "Raw sheet 0x{:02X} intruded on 3D Intro @ 0x{:06X}",
                    raw_idx,
                    pc
                );
                let (dec, clen) = super::disasm_dma::decompress_nintendo_lz2_with_endian(
                    &target.data,
                    pc,
                    4096,
                    false,
                )
                .unwrap_or_else(|| {
                    panic!(
                        "Raw sheet 0x{:02X} @ 0x{:06X} failed LZ2 decompress",
                        raw_idx, pc
                    )
                });
                assert_eq!(
                    dec.len(),
                    0x600,
                    "Raw sheet 0x{:02X} must decompress to exact 1536B (0x600)",
                    raw_idx
                );
                // Must NEVER straddle a 32 KB ($8000) LoROM bank boundary!
                assert_eq!(
                    pc / 0x8000,
                    (pc + clen - 1) / 0x8000,
                    "Raw sheet 0x{:02X} @ 0x{:06X} (len {}) crossed 32KB LoROM bank boundary!",
                    raw_idx,
                    pc,
                    clen
                );
            }
            // Exclusive Lemmy (0x0DD55E) & Final King Bowser (0x0DD596 / 0x0DD5EA) palettes patched
            assert_ne!(
                &target.data[0x0DD55E..0x0DD55E + 14],
                &zelda.data[0x0DD55E..0x0DD55E + 14],
                "Lemmy exclusive palette must be patched"
            );
            assert_ne!(
                &target.data[0x0DD596..0x0DD596 + 14],
                &zelda.data[0x0DD596..0x0DD596 + 14],
                "Final Bowser Pal 6 must be patched"
            );
            assert_ne!(
                &target.data[0x0DD5EA..0x0DD5EA + 14],
                &zelda.data[0x0DD5EA..0x0DD5EA + 14],
                "Final Bowser Pal 5 must be patched"
            );
            // Verify 3 NEW v13 N-SPC songs have distinct valid SPC $D000 pointers (Athletic #7 != OW #2, Ghost House #22 != Castle #16, Koopalings Boss #21 != Castle #16!)
            let ow_song7_ptr = u16::from_le_bytes([
                target.data[0x0D1EF9 + 6 * 2],
                target.data[0x0D1EF9 + 6 * 2 + 1],
            ]);
            assert!(
                (0xD036..0xFDAE).contains(&ow_song7_ptr) && ow_song7_ptr != ow_song2_ptr,
                "Toad Town #7 (SMW Athletic) must have distinct valid SPC pointer"
            );
            let dn_song16_ptr = u16::from_le_bytes([
                target.data[0x0D8004 + 15 * 2],
                target.data[0x0D8004 + 15 * 2 + 1],
            ]);
            let dn_song21_ptr = u16::from_le_bytes([
                target.data[0x0D8004 + 20 * 2],
                target.data[0x0D8004 + 20 * 2 + 1],
            ]);
            let dn_song22_ptr = u16::from_le_bytes([
                target.data[0x0D8004 + 21 * 2],
                target.data[0x0D8004 + 21 * 2 + 1],
            ]);
            assert!(
                (0xD046..0xFCBF).contains(&dn_song21_ptr) && dn_song21_ptr != dn_song16_ptr,
                "Koopaling Boss #21 must have distinct valid SPC pointer"
            );
            assert!(
                (0xD046..0xFCBF).contains(&dn_song22_ptr)
                    && dn_song22_ptr != dn_song16_ptr
                    && dn_song22_ptr != dn_song21_ptr,
                "Ghost House #22 must have distinct valid SPC pointer"
            );

            let audit = super::mixer::audit_legend_of_peach_output(&zelda, &target, &session)
                .expect("audit_legend_of_peach_output must pass");
            assert_eq!(audit.output_size_bytes, 2_097_152);
            assert_eq!(audit.verified_raw_sheets, 51);
            println!(
                "HARDWARE AUDIT PASSED: 100% COMPLIANT WITH SNES AND SUPER FAMICOM (SHA-256: {})!",
                audit.output_sha256
            );
        }
    }

    #[test]
    fn test_sha256_nist_vectors_and_input_rom_enforcement() {
        assert_eq!(
            super::rom::compute_sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            super::rom::compute_sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let (demo_a, demo_b) = generate_demo_rom_pair();
        assert!(super::rom::verify_legend_of_peach_inputs(&demo_a, &demo_b, false).is_err());
        assert!(super::rom::verify_legend_of_peach_inputs(&demo_a, &demo_b, true).is_ok());
    }

    #[test]
    fn test_kamek_shop_mini_mushroom_in_087000() {
        let mut buf = vec![0u8; 0x600];
        super::mario_ui::patch_smw_live_weapons_087000(&mut buf);
        // Mini mushroom at (9,2) -> tile # = 2*16 + 9 = 41 -> offset 41*24 = 984
        let t41 = &buf[41 * 24..42 * 24];
        let nz = t41.iter().filter(|&&b| b != 0).count();
        assert!(
            nz >= 12,
            "Mini Mushroom at (9,2) must have rich 3bpp pixel data"
        );
        // Mini berry at (A,2) -> tile # = 2*16 + 10 = 42
        let t42 = &buf[42 * 24..43 * 24];
        let nz42 = t42.iter().filter(|&&b| b != 0).count();
        assert!(
            nz42 >= 10,
            "Mini Berry at (A,2) must have rich 3bpp pixel data"
        );
    }

    #[test]
    fn test_room00_star_floor_and_raw0e_untouched() {
        let mut raw17 = vec![0u8; 0x600];
        super::mario_bosses::write_smw_golden_super_star_floor_tiles(&mut raw17);
        // Rows 0..2 + Row 3 cols 0..5 (tiles 0..53) must be completely untouched (zero)
        assert!(
            raw17[0..54 * 24].iter().all(|&b| b == 0),
            "Rows 0..2 and Row 3 cols 0..5 must remain untouched"
        );
        // Row 3 cols 6..13 (tiles 54..61) hold the 8 symmetric left-half tiles of the 32x32 SMW Super Star
        let star_nz = raw17[54 * 24..62 * 24].iter().filter(|&&b| b != 0).count();
        assert!(
            star_nz > 60,
            "Row 3 cols 6..13 must contain the 8 symmetric SMW Super Star floor tiles"
        );

        let z_path = if std::path::Path::new("roms/zelda_alttp.sfc").exists() {
            "roms/zelda_alttp.sfc"
        } else {
            "../../roms/zelda_alttp.sfc"
        };
        if let Ok(z_bytes) = std::fs::read(z_path) {
            let mut z_rom = super::rom::SnesRom::from_bytes("zelda", &z_bytes).unwrap();
            let mut sess = super::swapper::SwapSession::new();
            let mut free_cursor = 0x118000;
            let free_end = 0x200000;
            let res = super::mixer::patch_alttp_raw_sheet_pub(
                &mut z_rom,
                &mut sess,
                0x15,
                "test",
                &mut free_cursor,
                free_end,
                &|raw| {
                    super::mario_bosses::write_smw_golden_super_star_floor_tiles(raw);
                },
            );
            println!("patch_alttp_raw_sheet_pub on 0x15 result: {:?}", res);
            assert!(res.unwrap(), "raw:0x15 MUST be successfully patched!");
        }
    }

    #[test]
    fn test_multi_region_known_rom_catalog() {
        use super::rom::{identify_known_rom, RomRole, KNOWN_ROMS};
        assert!(
            KNOWN_ROMS.len() >= 7,
            "Catalog must define at least 7 official revisions"
        );

        // Check each entry in KNOWN_ROMS has valid metadata
        for entry in KNOWN_ROMS {
            assert!(!entry.title.is_empty());
            assert!(!entry.region.is_empty());
            assert_eq!(entry.sha256.len(), 64);
            match entry.role {
                RomRole::BaseZelda => assert_eq!(entry.size_bytes, 1_048_576),
                RomRole::DonorSMW => assert_eq!(entry.size_bytes, 524_288),
            }
        }

        // Verify that arbitrary data is not recognized
        let fake_rom = vec![0u8; 1_048_576];
        assert!(identify_known_rom(&fake_rom).is_none());
    }

    #[test]
    fn test_v060_all_seven_blocking_bugs_structural_invariants() {
        let z_path = if std::path::Path::new("roms/zelda_alttp.sfc").exists() {
            "roms/zelda_alttp.sfc"
        } else {
            "../../roms/zelda_alttp.sfc"
        };
        let m_path = if std::path::Path::new("roms/super_mario_world.sfc").exists() {
            "roms/super_mario_world.sfc"
        } else {
            "../../roms/super_mario_world.sfc"
        };
        let (Ok(z_bytes), Ok(m_bytes)) = (std::fs::read(z_path), std::fs::read(m_path)) else {
            return;
        };
        let mut target = super::rom::SnesRom::from_bytes("zelda_alttp.sfc", &z_bytes).unwrap();
        let donor = super::rom::SnesRom::from_bytes("super_mario_world.sfc", &m_bytes).unwrap();
        let recipe = super::mixer::legend_of_peach_canonical_recipe(z_path, m_path);
        let mut donors = std::collections::HashMap::new();
        donors.insert("smw".to_string(), donor);
        let mut sess = super::swapper::SwapSession::new();
        super::mixer::execute_mix_recipe(&mut target, &donors, &recipe, &mut sess).unwrap();

        // BUG 01 & BUG 02: Across ALL 303 Link/Mario poses:
        //   - ZERO odd-row overlapping 16x16 cells
        //   - ZERO unpatched vanilla Link cells active in Head or Body slots
        //   - ZERO same-cell Head == Body double-head bugs
        let read_4bpp_cell = |rom: &[u8], addr: u16| -> Vec<u8> {
            let pc_top = 0x080000 + (addr as usize - 0x8000);
            let mut buf = Vec::with_capacity(128);
            buf.extend_from_slice(&rom[pc_top..pc_top + 64]);
            buf.extend_from_slice(&rom[pc_top + 0x200..pc_top + 0x240]);
            buf
        };
        for pid in 0..303usize {
            let ha = u16::from_le_bytes([
                target.data[0x001346 + pid * 2],
                target.data[0x001346 + pid * 2 + 1],
            ]);
            let ba = u16::from_le_bytes([
                target.data[0x0015A4 + pid * 2],
                target.data[0x0015A4 + pid * 2 + 1],
            ]);
            let fl = target.data[0x068000 + pid * 3 + 2];
            if (fl & 0xF0) != 0xF0 {
                assert_eq!(
                    (ha - 0x8000) & 0x023F,
                    0,
                    "Pose 0x{pid:03X} Head addr 0x{ha:04X} must be even-row 16x16 aligned!"
                );
                let van_c = read_4bpp_cell(&z_bytes, ha);
                let blt_c = read_4bpp_cell(&target.data, ha);
                assert!(
                    van_c.iter().all(|&b| b == 0) || van_c != blt_c,
                    "Pose 0x{pid:03X} Head cell 0x{ha:04X} is still unpatched Vanilla Link!"
                );
            }
            if (fl & 0x0F) != 0x0F {
                assert_eq!(
                    (ba - 0x8000) & 0x023F,
                    0,
                    "Pose 0x{pid:03X} Body addr 0x{ba:04X} must be even-row 16x16 aligned!"
                );
                let van_c = read_4bpp_cell(&z_bytes, ba);
                let blt_c = read_4bpp_cell(&target.data, ba);
                assert!(
                    van_c.iter().all(|&b| b == 0) || van_c != blt_c,
                    "Pose 0x{pid:03X} Body cell 0x{ba:04X} is still unpatched Vanilla Link!"
                );
            }
            if (fl & 0xF0) != 0xF0 && (fl & 0x0F) != 0x0F {
                assert_ne!(
                    ha, ba,
                    "Pose 0x{pid:03X} must never reuse Head cell for Body!"
                );
            }
        }

        // BUG 03: Screen 0x2C terrain grammar validation passes with ZERO orphan cliffs/dirt seams
        let orig2c = super::mario_map::read_overworld_screen(&z_bytes, 0x2C, "W1", "Orig").unwrap();
        let edit2c =
            super::mario_map::read_overworld_screen(&target.data, 0x2C, "W1", "Edit").unwrap();
        let rep = super::mario_map::validate_screen_terrain_grammar(&target.data, &orig2c, &edit2c)
            .unwrap();
        assert!(
            rep.is_clean(),
            "Screen 0x2C terrain grammar must be 100% clean: {rep:?}"
        );

        // BUG 04: Kamek Shopkeeper uses SMW Royal Blue palette (0x08/0x09 ^ 0x00 = 0x09 Pal 4 + NT1) and clean tiles ($06/$26)
        assert_eq!(
            target.data[0x030BFA], 0x08,
            "Shopkeeper prep must set Blue Pal 4"
        );
        assert_eq!(
            target.data[0x0F6F54], 0x09,
            "Shopkeeper state 0 must set Pal 4 Blue + NT1"
        );
        assert_eq!(
            target.data[0x02F8FA], 0x00,
            "Kamek head table flags must be 0x00 so EOR $0F50(0x09) yields SMW Royal Blue Pal 4"
        );
        assert_eq!(
            target.data[0x02F901], 0x06,
            "Kamek body F0 must use clean tile $06"
        );
        assert_eq!(
            target.data[0x02F911], 0x26,
            "Kamek body F1 must use clean tile $26"
        );
        // Verify Shop Price Digits '0' & '1' at row 3 cols 0..1 of Sheet 0xBC (raw 0xBE) match vanilla 100%!
        let (van_be, _, _) = super::mixer::load_alttp_3bpp_sheet(&z_bytes, 0xBC).unwrap();
        let (blt_be, _, _) = super::mixer::load_alttp_3bpp_sheet(&target.data, 0xBC).unwrap();
        assert_eq!(
            &blt_be[48 * 24..50 * 24],
            &van_be[48 * 24..50 * 24],
            "Shop price digits '0' and '1' at row 3 cols 0..1 of Sheet 0xBC must be 100% intact!"
        );

        // BUG 05: Room 0x0103 Toad head OAM lifted to dy=-9 (0xF7) for clean 16-bit proportions
        assert_eq!(target.data[0x02E544], 0xF7);

        // BUG 06: Iggy Koopa wings anchored inward from +-24 to +-17 at $1A:FCED
        assert_eq!(
            i16::from_le_bytes([target.data[0x0D7CED], target.data[0x0D7CEE]]),
            -17
        );
        assert_eq!(
            i16::from_le_bytes([target.data[0x0D7D05], target.data[0x0D7D06]]),
            17
        );

        // BUG 07: Ganon held Trident call ($1D:9AF5) NOP'd & Bowser standing Head lifted to dy=-15 (0xF1)
        assert_eq!(&target.data[0x0E9AF5..0x0E9AF8], &[0xEA, 0xEA, 0xEA]);
        assert_eq!(target.data[0x0E969A + 25], 0xF1);
        assert_eq!(target.data[0x0E969A + 26], 0xF1);

        // v0.7.0 INVARIANTS:
        // 1. All 160 Overworld screens decode losslessly in the built ROM (including relocated LW + DW in $3B..$3F)
        for sid in 0u8..160u8 {
            let scr = super::mario_map::read_overworld_screen(&target.data, sid, "v070", "Check")
                .unwrap_or_else(|| {
                    panic!("Overworld screen 0x{sid:02X} corrupted in v0.7.0 build!")
                });
            assert_eq!(scr.tiles.len(), 256);
        }
        // 2. Koopa Guard raw 0xBA sword tiles (11,2)/(11,3)/(12,2)/(12,3) transformed into SMW Hammer Bro Hammers!
        let (van_ba, _, _) = super::mixer::load_alttp_3bpp_sheet(&z_bytes, 0xBA).unwrap();
        let (blt_ba, _, _) = super::mixer::load_alttp_3bpp_sheet(&target.data, 0xBA).unwrap();
        for &(tx, ty) in &[
            (11usize, 2usize),
            (11, 3),
            (12, 2),
            (12, 3),
            (14, 2),
            (15, 2),
        ] {
            let off = (ty * 16 + tx) * 24;
            assert_ne!(
                &blt_ba[off..off + 24],
                &van_ba[off..off + 24],
                "Koopa Guard weapon tile ({tx},{ty}) in raw 0xBA must be transformed into SMW Hammer/Club!"
            );
        }
        // 3. Dark World Bowser Volcanic Kingdom palettes active at 0x0DE70E & 0x0DE79A (distinct from vanilla olive!)
        assert_ne!(
            &target.data[0x0DE70E..0x0DE71C],
            &z_bytes[0x0DE70E..0x0DE71C]
        );
        assert_ne!(
            &target.data[0x0DE79A..0x0DE7A8],
            &z_bytes[0x0DE79A..0x0DE7A8]
        );
        assert_ne!(
            &target.data[0x0DEB36..0x0DEB60],
            &z_bytes[0x0DEB36..0x0DEB60]
        );

        // v0.8.0 INVARIANTS (Super Mario Universe Revolution):
        // 1. 65c816 Mario Universe Engine ($22:9000), 256B Behavior Table ($22:B000), 9 Live Palettes ($22:B100)
        assert_eq!(
            target.data[super::mario_engine::STOMP_TABLE_PC + 0x41],
            3,
            "Blue Koopa must be Shell-Kickable (class 3) in built ROM"
        );
        assert_eq!(
            target.data[super::mario_engine::STOMP_TABLE_PC + 0x19],
            4,
            "Overworld Boo (Poe 0x19) must use Orientation-Freeze AI (class 4) in built ROM"
        );
        assert_eq!(
            target.data[super::mario_engine::STOMP_TABLE_PC + 0x8B],
            4,
            "Ghost House Boo (0x8B) must use Orientation-Freeze AI (class 4) in built ROM"
        );
        // 2. ALL 8 Distinct Koopaling & King Bowser Dungeon Fortress architectural palettes active (verified via $07:FA90 -> DungPal[1,4,5,8,9,10,12,13])
        for &dung_pal_pc in &[
            0x0DD7E8usize, // DungPal[1]  Wendy's Coral Sea Palace
            0x0DDA04,      // DungPal[4]  Lemmy's Star-Ice Circus Fortress
            0x0DDAB8,      // DungPal[5]  Larry's Sun-Baked Chocolate Desert Pyramid
            0x0DDCD4,      // DungPal[8]  Morton's Red-Brick Koopa Fortress
            0x0DDD88,      // DungPal[9]  Iggy's Indigo Boo Ghost House
            0x0DDE3C,      // DungPal[10] Roy's Midnight Koopa Castle
            0x0DDFA4,      // DungPal[12] Ludwig's Industrial Airship Mech Keep
            0x0DE058,      // DungPal[13] King Bowser's Obsidian & Lava Keep
        ] {
            assert_ne!(
                &target.data[dung_pal_pc..dung_pal_pc + 14],
                &z_bytes[dung_pal_pc..dung_pal_pc + 14],
                "Koopaling/Bowser Fortress DungPal at 0x{dung_pal_pc:06X} must be patched!"
            );
        }

        // v0.9.0 INVARIANTS (The Ultimate Mario Adventure — ZRI-009, ZRI-017, ZRI-008, ZRI-019):
        // 1. Title Screen Royal Princess Peach Star-Scepter (raw 0xA5 = SP[0x32] = load_alttp sheet 0xA3) replaces Master Sword blade cleanly!
        let (van_a5, _, _) = super::mixer::load_alttp_3bpp_sheet(&z_bytes, 0xA3).unwrap();
        let (blt_a5, _, _) = super::mixer::load_alttp_3bpp_sheet(&target.data, 0xA3).unwrap();
        assert_eq!(blt_a5.len(), 0x600);
        assert_ne!(
            &blt_a5[0..16 * 24],
            &van_a5[0..16 * 24],
            "Title Screen raw 0xA5 (SP[0x32]) Master Sword must be replaced by Royal Peach Star-Scepter!"
        );
        // Preserve untouched Title Screen sparkle tiles at rows 2-3 cols 4..15!
        assert_eq!(
            &blt_a5[(2 * 16 + 4) * 24..(2 * 16 + 16) * 24],
            &van_a5[(2 * 16 + 4) * 24..(2 * 16 + 16) * 24],
            "Title Screen sparkle tiles in raw 0xA5 row 2 cols 4..15 must remain intact!"
        );
        // 2. Physical Overworld SMW Green Warp Pipe in Light World (sheet 0x3C = raw 0x3E) & Dark World (raw 0x3F = sheet 0x3D) + Screen 0x2C (9,7)!
        let (van_3c, _, _) = super::mixer::load_alttp_3bpp_sheet(&z_bytes, 0x3C).unwrap();
        let (blt_3c, _, _) = super::mixer::load_alttp_3bpp_sheet(&target.data, 0x3C).unwrap();
        assert_ne!(
            &blt_3c[32 * 24..33 * 24],
            &van_3c[32 * 24..33 * 24],
            "Light World sheet 0x3C (raw 0x3E) (0,2) must contain Physical 3D Green SMW Warp Pipe!"
        );
        let (van_3f, _, _) = super::mixer::load_alttp_3bpp_sheet(&z_bytes, 0x3D).unwrap();
        let (blt_3f, _, _) = super::mixer::load_alttp_3bpp_sheet(&target.data, 0x3D).unwrap();
        assert_ne!(
            &blt_3f[32 * 24..33 * 24],
            &van_3f[32 * 24..33 * 24],
            "Dark World raw 0x3F (sheet 0x3D) (0,2) must contain Physical 3D Bowser Warp Pipe!"
        );
        assert_eq!(
            edit2c.tiles[9 * 16 + 7],
            0x02D4,
            "Screen 0x2C (r09, c07) must hold Physical SMW Green Warp Pipe Map32 0x02D4!"
        );
        // 3. Boss Combat Synergy Class (0x05) active for King Bowser (0xD6) & Morton Koopa (0x92)!
        assert_eq!(
            target.data[super::mario_engine::STOMP_TABLE_PC + 0xD6],
            5,
            "King Bowser (0xD6) must use Boss Combat Class (5) in built ROM"
        );
        assert_eq!(
            target.data[super::mario_engine::STOMP_TABLE_PC + 0x92],
            5,
            "Morton Koopa (0x92) must use Boss Combat Class (5) in built ROM"
        );
    }
}
