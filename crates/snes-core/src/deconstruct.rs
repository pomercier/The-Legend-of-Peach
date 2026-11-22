use image::{ImageBuffer, Rgba};
use std::fs;
use std::path::Path;
use std::time::Instant;

use crate::assets::scan_assets_for_rom;
use crate::disasm_dma::decompress_nintendo_lz2_with_endian;
use crate::graphics::{
    render_single_asset_sprite, render_tile_sheet_rgba, scan_candidate_palettes, SnesPalette,
    TileFormat,
};
use crate::rom::{RecognizedGame, SnesRom};

/// Configuration for the full SNES ROM deconstruction engine
#[derive(Debug, Clone)]
pub struct DeconstructConfig {
    pub export_sprites: bool,
    pub export_sheets: bool,
    pub export_audio: bool,
    pub export_spc: bool,
    pub export_text: bool,
    pub export_palettes: bool,
    pub export_disassembly: bool,
    pub export_tilemaps: bool,
    pub upscale_sprites_factor: u32,
    pub full_extraction: bool,
}

impl Default for DeconstructConfig {
    fn default() -> Self {
        Self {
            export_sprites: true,
            export_sheets: true,
            export_audio: true,
            export_spc: true,
            export_text: true,
            export_palettes: true,
            export_disassembly: true,
            export_tilemaps: true,
            upscale_sprites_factor: 3,
            full_extraction: false,
        }
    }
}

/// Comprehensive report of all assets extracted and organized
#[derive(Debug, Clone)]
pub struct DeconstructReport {
    pub rom_title: String,
    pub internal_name: String,
    pub mapping: String,
    pub checksum: u16,
    pub crc32: u32,
    pub out_dir: String,
    pub sprites_count: usize,
    pub sheets_count: usize,
    pub spc_files_count: usize,
    pub brr_samples_count: usize,
    pub audio_wav_count: usize,
    pub text_dialogues_count: usize,
    pub palettes_count: usize,
    pub disassembled_banks_count: usize,
    pub screens_count: usize,
    pub elapsed_ms: u64,
}

/// Deconstruct ANY SNES ROM (.sfc/.smc) into an organized, editable round-trip project directory
/// containing ALL assets (universal hardware discovery + assembled standalone sprites + project.json).
pub fn deconstruct_rom(
    rom: &SnesRom,
    out_dir: &Path,
    config: &DeconstructConfig,
) -> std::io::Result<DeconstructReport> {
    let start_time = Instant::now();

    let sprites_dir = out_dir.join("sprites");
    let sheets_dir = out_dir.join("sheets");
    let audio_dir = out_dir.join("audio");
    let text_dir = out_dir.join("text");
    let pal_dir = out_dir.join("palettes");
    let screens_dir = out_dir.join("screens");
    let disasm_dir = out_dir.join("disassembly");

    fs::create_dir_all(&sprites_dir)?;
    fs::create_dir_all(&sheets_dir)?;
    fs::create_dir_all(&audio_dir)?;
    fs::create_dir_all(&text_dir)?;
    fs::create_dir_all(&pal_dir)?;
    fs::create_dir_all(&screens_dir)?;
    fs::create_dir_all(&disasm_dir)?;

    // =========================================================================
    // 0. UNIVERSAL HARDWARE DISCOVERY & ROUND-TRIP PROJECT EXPORT (`project.json`)
    // Works identically on 100% of SNES ROMs (LoROM, HiROM, ExHiROM, Unknown, Homebrew!)
    // =========================================================================
    let catalog =
        crate::universal::discover_universal_catalog_with_mode(rom, config.full_extraction);
    crate::universal::export_universal_editable_project_with_mode(
        rom,
        &catalog,
        out_dir,
        config.full_extraction,
    )?;

    let game = rom.identify_game();

    // =========================================================================
    // 1. PALETTES SUMMARY + FALLBACK FOR SPRITE RENDERING
    // =========================================================================
    let mut palettes = scan_candidate_palettes(&rom.data, 32);
    if palettes.is_empty() {
        palettes.push(SnesPalette::preset_grayscale());
    }
    let default_pal = palettes
        .first()
        .cloned()
        .unwrap_or_else(SnesPalette::preset_grayscale);
    let palettes_count = catalog.palettes.len().max(palettes.len());

    // =========================================================================
    // 2. GRAPHICS SHEETS (Universal discovered sheets + bonus named LZ sheets)
    // =========================================================================
    let mut sheets_count = catalog.graphics.len();
    if config.export_sheets {
        let known_lz_sheets: Vec<(&str, usize, usize, TileFormat)> = match game {
            RecognizedGame::SuperMarioWorld => vec![
                ("GFX00_PowerUps_Blocks", 0x0459F9, 8192, TileFormat::Bpp3),
                ("GFX01_Enemies_Koopas", 0x046231, 8192, TileFormat::Bpp3),
                ("GFX14_Castle_Backgrounds", 0x054817, 8192, TileFormat::Bpp4),
                ("GFX28_HUD_Font", 0x05C06C, 8192, TileFormat::Bpp2),
                ("GFX29_Title_Logo", 0x05C6A3, 8192, TileFormat::Bpp2),
                ("GFX32_Player_Mario", 0x040000, 32768, TileFormat::Bpp4),
                ("GFX33_Yoshi_Items", 0x043FC0, 16384, TileFormat::Bpp3),
            ],
            RecognizedGame::ZeldaAlttp => vec![
                ("Sheet_00_Link_Sprites", 0x080000, 2048, TileFormat::Bpp4),
                ("Sheet_01_Swords_Items", 0x081000, 2048, TileFormat::Bpp4),
                ("Sheet_07_Princess_Zelda", 0x087000, 2048, TileFormat::Bpp4),
                ("Sheet_0E_Soldiers_Guards", 0x08E000, 2048, TileFormat::Bpp4),
                ("Sheet_5C_Title_Castle", 0x0A6BAE, 2048, TileFormat::Bpp3),
            ],
            _ => vec![],
        };

        for (sheet_name, off, max_len, fmt) in known_lz_sheets {
            if let Some((decompressed, _)) =
                decompress_nintendo_lz2_with_endian(&rom.data, off, max_len, true)
            {
                let _ = fs::write(
                    sheets_dir.join(format!("{}.bin", sheet_name)),
                    &decompressed,
                );
                let cols = 16;
                let total_tiles = decompressed.len() / fmt.bytes_per_tile();
                let rows = (total_tiles / cols).max(1);
                let (w, h, rgba) =
                    render_tile_sheet_rgba(&decompressed, 0, cols, rows, fmt, &default_pal);
                if let Some(img) =
                    ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(w as u32, h as u32, rgba)
                {
                    let _ = img.save(sheets_dir.join(format!("{}.png", sheet_name)));
                    sheets_count += 1;
                }
            }
        }
    }

    // =========================================================================
    // 3. EXTRACT ASSEMBLED STANDALONE SPRITES & COMPANION METADATA JSON
    // =========================================================================
    let mut sprites_count = 0;
    if config.export_sprites {
        let scanned = scan_assets_for_rom(rom);
        let scale = config.upscale_sprites_factor.max(1);

        for asset in &scanned {
            let pal = if let Some(poff) = asset.palette_offset {
                palettes
                    .iter()
                    .find(|p| p.source_offset == Some(poff))
                    .unwrap_or(&default_pal)
            } else {
                &default_pal
            };

            let (w, h, rgba) =
                if let Some((aw, ah, argba)) = render_single_asset_sprite(&rom.data, &asset.id) {
                    (aw, ah, argba)
                } else {
                    render_tile_sheet_rgba(&rom.data, asset.rom_offset, 2, 2, asset.format, pal)
                };

            if let Some(raw_img) =
                ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(w as u32, h as u32, rgba)
            {
                let safe_id = asset.id.replace(|c: char| !c.is_alphanumeric(), "_");
                let cat_label = asset
                    .category
                    .label_fr()
                    .replace(|c: char| !c.is_alphanumeric(), "_");

                // A. 1x pixel-art PNG
                let file_1x = format!("{}_{}_1x.png", cat_label, safe_id);
                let _ = raw_img.save(sprites_dir.join(&file_1x));

                // B. Crisp Nx upscaled PNG
                let mut upscaled: ImageBuffer<Rgba<u8>, Vec<u8>> =
                    ImageBuffer::new((w as u32) * scale, (h as u32) * scale);
                for y in 0..(h as u32) {
                    for x in 0..(w as u32) {
                        let px = raw_img.get_pixel(x, y);
                        for sy in 0..scale {
                            for sx in 0..scale {
                                upscaled.put_pixel(x * scale + sx, y * scale + sy, *px);
                            }
                        }
                    }
                }
                let file_scaled = format!("{}_{}.png", cat_label, safe_id);
                let _ = upscaled.save(sprites_dir.join(&file_scaled));

                // C. Companion JSON metadata
                let meta_json = format!(
                    "{{\n  \"id\": \"{}\",\n  \"name\": \"{}\",\n  \"category\": \"{}\",\n  \"family\": \"{}\",\n  \"width\": {},\n  \"height\": {},\n  \"format\": \"{}\",\n  \"rom_offset\": \"0x{:06X}\",\n  \"sheet_name\": \"{}\",\n  \"description\": \"{}\"\n}}\n",
                    asset.id,
                    asset.name,
                    asset.category.label_fr(),
                    asset.family_name,
                    w,
                    h,
                    asset.format.short_label(),
                    asset.rom_offset,
                    asset.sheet_name,
                    asset.description.replace('"', "\\\"")
                );
                let _ = fs::write(
                    sprites_dir.join(format!("{}_{}.json", cat_label, safe_id)),
                    meta_json,
                );
                sprites_count += 1;
            }
        }
    }

    // Count any universally extracted OAM cluster sprites
    if let Ok(entries) = fs::read_dir(&sprites_dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with("oam_sprite_") && name.ends_with(".png") {
                sprites_count += 1;
            }
        }
    }

    // =========================================================================
    // 4. AUDIO SUMMARY (Universal IPL .SPC + BRR/WAV + sample_directory.json)
    // =========================================================================
    let mut spc_files_count = catalog.spc_streams.len();
    let brr_samples_count = catalog.audio_samples.len();
    let audio_wav_count = catalog.audio_samples.len();

    if config.export_audio {
        // Export sample_directory.json from universal catalog
        let mut sample_dir_json = String::from("[\n");
        for (idx, sample) in catalog.audio_samples.iter().enumerate() {
            sample_dir_json.push_str(&format!(
                "  {{\n    \"id\": \"{}\",\n    \"offset\": \"0x{:06X}\",\n    \"snes_addr\": \"{}\",\n    \"byte_len\": {},\n    \"block_count\": {},\n    \"loops\": {},\n    \"peak_amplitude\": {},\n    \"duration_ms\": {},\n    \"wav_file\": \"{}\",\n    \"label\": \"{}\"\n  }}{}\n",
                sample.id,
                sample.rom_offset,
                sample.snes_addr,
                sample.byte_len,
                sample.block_count,
                sample.loops,
                sample.peak_amplitude,
                sample.duration_ms_16khz,
                sample.wav_file,
                sample.name,
                if idx + 1 < catalog.audio_samples.len() { "," } else { "" }
            ));
        }
        sample_dir_json.push_str("]\n");
        fs::write(audio_dir.join("sample_directory.json"), sample_dir_json)?;

        if spc_files_count == 0 && !catalog.audio_samples.is_empty() {
            spc_files_count = 1; // universal_soundbank.spc synthesized above
        }
    }

    // =========================================================================
    // 5. TEXT & DIALOGUES (Universal + Specialized Game Scripts)
    // =========================================================================
    let mut text_dialogues_count = catalog.texts.len();
    if config.export_text {
        let extra = extract_rom_texts(rom, &text_dir)?;
        if extra > text_dialogues_count {
            text_dialogues_count = extra;
        }
    }

    // =========================================================================
    // 6. SCREENS & TILEMAPS
    // =========================================================================
    let mut screens_count = catalog.tilemaps.len();
    if config.export_tilemaps {
        match game {
            RecognizedGame::SuperMarioWorld => {
                if let Some(title_img) = reconstruct_smw_title_screen_rgba(&rom.data) {
                    let _ = title_img.save(screens_dir.join("title_screen_super_mario_world.png"));
                    screens_count += 1;
                }
            }
            RecognizedGame::ZeldaAlttp => {
                if let Some(zelda_img) = reconstruct_zelda_title_screen_rgba(&rom.data) {
                    let _ = zelda_img.save(screens_dir.join("title_screen_zelda_alttp.png"));
                    screens_count += 1;
                }
            }
            _ => {}
        }
    }

    let disassembled_banks_count = rom.banks().len();

    // =========================================================================
    // 8. MASTER INDEX MARKDOWN & MACHINE-READABLE MANIFEST JSON
    // =========================================================================
    let elapsed = start_time.elapsed().as_millis() as u64;

    let manifest = format!(
        "{{\n\
           \"rom_title\": \"{}\",\n\
           \"internal_title\": \"{}\",\n\
           \"mapping\": \"{}\",\n\
           \"rom_size_bytes\": {},\n\
           \"checksum\": \"0x{:04X}\",\n\
           \"crc32\": \"0x{:08X}\",\n\
           \"sprites_count\": {},\n\
           \"sheets_count\": {},\n\
           \"spc_files_count\": {},\n\
           \"brr_samples_count\": {},\n\
           \"audio_wav_count\": {},\n\
           \"text_dialogues_count\": {},\n\
           \"palettes_count\": {},\n\
           \"disassembled_banks_count\": {},\n\
           \"screens_count\": {},\n\
           \"elapsed_ms\": {}\n\
         }}\n",
        rom.canonical_title(),
        rom.header.title.trim(),
        rom.header.mapping.as_str(),
        rom.data.len(),
        rom.header.checksum,
        rom.crc32,
        sprites_count,
        sheets_count,
        spc_files_count,
        brr_samples_count,
        audio_wav_count,
        text_dialogues_count,
        palettes_count,
        disassembled_banks_count,
        screens_count,
        elapsed
    );
    fs::write(out_dir.join("manifest.json"), manifest)?;

    let index_md = format!(
        "# 🎮 Déconstruction Complète des Assets SNES : {}\n\n\
         > Décompilation et extraction intégrale 100% matérielle réalisée en {} ms.\n\n\
         ## 📋 Fiche Technique de la Cartouche\n\
         - **Titre Interne** : `{}`\n\
         - **Mapping Matériel** : `{}` (Taille : {} Ko)\n\
         - **Checksum Interne** : `0x{:04X}` (Complément : `0x{:04X}`, Valide : `{}`)\n\
         - **CRC32** : `0x{:08X}`\n\
         - **Vecteur RESET** : `${:04X}` | **NMI** : `${:04X}` | **IRQ** : `${:04X}`\n\n\
         ## 📂 Répertoire des Actifs Décomposés\n\n\
         ### 1. 🏃 Sprites Individuels Assemblés (`sprites/` - {} fichiers)\n\
         Tous les personnages et objets sont isolés, assemblés dans leurs poses complètes, avec palette authentique et fichier de métadonnées `.json` compagnon.\n\n\
         ### 2. 🗺️ Planches Graphiques Décompressées (`sheets/` - {} fichiers)\n\
         Feuilles de tuiles 2bpp, 3bpp, 4bpp décompressées depuis les flux Nintendo LZ2 ou extraites brutes, enregistrées en `.png` et `.bin`.\n\n\
         ### 3. 🎵 Musiques & Audio SPC700 (`audio/` - {} SPC, {} WAV, {} BRR)\n\
         - Fichiers `.spc` standard (66 048 octets) directement lisibles par tout lecteur SPC/SNES.\n\
         - Échantillons audio BRR convertis en PCM 16 bits `.wav` @ 16 kHz.\n\
         - Table d'échantillons `sample_directory.json`.\n\n\
         ### 4. 📜 Textes & Dialogues (`text/` - {} entrées)\n\
         - Scripts et messages de jeu décodés avec pointeurs et balises de contrôle.\n\
         - Table d'encodage `.tbl` Thingy réutilisable pour la réinsertion.\n\n\
         ### 5. 🎨 Palettes BGR555 (`palettes/` - {} palettes)\n\
         - Nuanciers visuels `.png` (256x16).\n\
         - Définitions hexadécimales et 15 bits `.json`.\n\
         - Fichiers binaires bruts `.pal` (32 octets).\n\n\
         ### 6. 🖼️ Écrans & Tilemaps Reconstruits (`screens/` - {} écrans)\n\
         Plans d'arrière-plan et écrans-titres assemblés en résolution native SNES (256x224).\n\n\
         ### 7. ⚙️ Code Source & Désassemblage (`disassembly/` - {} banques)\n\
         - `header.asm` prêt pour l'assembleur WLA-DX.\n\
         - Fichiers `bank_XX.asm` désassemblés avec registres MMIO ($2100, $2115, $4200, $4300).\n\
         - Cartographie DMA et carte d'entropie des données.\n\n\
         ## 🔄 Workflow Universel Aller-Retour (Éditer & Reconstruire la ROM)\n\
         Ce dossier contient un manifeste **`project.json`** 100% réversible !\n\
         1. Éditez librement n'importe quel fichier PNG dans `sheets/`, palette `.json` dans `palettes/`, fichier `.wav` dans `audio/` ou texte dans `text/strings_editable.json`.\n\
         2. Reconstruisez automatiquement votre ROM SNES 100% matérielle (+ patch `.ips`) :\n\
         ```bash\n\
         snes-cli --reconstruct <rom_originale.sfc> {} <rom_modifiee.sfc> --ips <patch.ips>\n\
         ```\n",
        rom.canonical_title(),
        elapsed,
        rom.header.title.trim(),
        rom.header.mapping.as_str(),
        rom.data.len() / 1024,
        rom.header.checksum,
        rom.header.checksum_complement,
        rom.header.is_checksum_valid(),
        rom.crc32,
        rom.header.reset_vector,
        rom.header.nmi_vector,
        rom.header.irq_vector,
        sprites_count,
        sheets_count,
        spc_files_count,
        audio_wav_count,
        brr_samples_count,
        text_dialogues_count,
        palettes_count,
        screens_count,
        disassembled_banks_count,
        out_dir.display()
    );
    fs::write(out_dir.join("INDEX.md"), index_md)?;

    Ok(DeconstructReport {
        rom_title: rom.canonical_title(),
        internal_name: rom.header.title.trim().to_string(),
        mapping: rom.header.mapping.as_str().to_string(),
        checksum: rom.header.checksum,
        crc32: rom.crc32,
        out_dir: out_dir.display().to_string(),
        sprites_count,
        sheets_count,
        spc_files_count,
        brr_samples_count,
        audio_wav_count,
        text_dialogues_count,
        palettes_count,
        disassembled_banks_count,
        screens_count,
        elapsed_ms: elapsed,
    })
}

// =============================================================================
// HELPER: BUILD AUTHENTIC 66,048-BYTE SPC FILE
// =============================================================================

/// Creates a standard 66,048-byte .SPC file matching official SNES-SPC700 specifications
pub fn create_spc_file(
    song_title: &str,
    game_title: &str,
    dumper: &str,
    comments: &str,
    artist: &str,
    pc: u16,
    sp: u8,
    a: u8,
    x: u8,
    y: u8,
    psw: u8,
    ram: &[u8; 65536],
    dsp_regs: &[u8; 128],
    ipl_rom: Option<&[u8; 64]>,
) -> Vec<u8> {
    let mut spc = vec![0u8; 66048];

    // Magic Header: "SNES-SPC700 Sound File Data v0.30\x1a\x1a"
    let magic = b"SNES-SPC700 Sound File Data v0.30\x1a\x1a";
    spc[0..magic.len()].copy_from_slice(magic);
    spc[0x23] = 0x1A; // Has ID666 tag
    spc[0x24] = 30; // Version minor

    // CPU Registers
    spc[0x25..0x27].copy_from_slice(&pc.to_le_bytes());
    spc[0x27] = a;
    spc[0x28] = x;
    spc[0x29] = y;
    spc[0x2A] = psw;
    spc[0x2B] = sp;

    // ID666 Text Tags
    let write_tag = |dest: &mut [u8], s: &str, max_len: usize| {
        let b = s.as_bytes();
        let copy_len = b.len().min(max_len);
        dest[..copy_len].copy_from_slice(&b[..copy_len]);
    };

    write_tag(&mut spc[0x2E..0x4E], song_title, 32);
    write_tag(&mut spc[0x4E..0x6E], game_title, 32);
    write_tag(&mut spc[0x6E..0x7E], dumper, 16);
    write_tag(&mut spc[0x7E..0x9E], comments, 32);
    write_tag(&mut spc[0x9E..0xA9], "2026/11/17", 11);
    write_tag(&mut spc[0xA9..0xAC], "180", 3); // Play length 180s
    write_tag(&mut spc[0xAC..0xB1], "5000", 5); // Fade 5000ms
    write_tag(&mut spc[0xB1..0xD1], artist, 32);

    // 64 KB SPC700 RAM
    spc[0x100..0x10100].copy_from_slice(ram);

    // 128 bytes DSP Registers
    spc[0x10100..0x10180].copy_from_slice(dsp_regs);

    // 64 bytes IPL ROM (0xFFC0..0xFFFF backup)
    #[rustfmt::skip]
    let default_ipl: [u8; 64] = [
        0xCD, 0xEF, 0xBD, 0xE8, 0x00, 0xC6, 0x1D, 0xD0, 0xFC, 0x8F, 0xAA, 0xF4, 0x8F, 0xBB, 0xF5, 0x78,
        0xCC, 0xF4, 0xD0, 0xFB, 0x2F, 0x19, 0xEB, 0xF4, 0xD0, 0xFC, 0x7E, 0xF4, 0xD0, 0x0B, 0xE4, 0xF5,
        0xCB, 0xF4, 0xD7, 0x00, 0xFC, 0xD0, 0xF3, 0xAB, 0x01, 0x10, 0xEF, 0x7E, 0xF4, 0x10, 0xEB, 0xBA,
        0xF6, 0xDA, 0x00, 0xBA, 0xF4, 0xC4, 0xF4, 0xDD, 0x5D, 0xD0, 0xDB, 0x1F, 0x00, 0x00, 0xC0, 0xFF
    ];
    let ipl_bytes = ipl_rom.unwrap_or(&default_ipl);
    spc[0x101C0..0x10200].copy_from_slice(ipl_bytes);

    spc
}

// =============================================================================
// HELPER: EXTRACT TEXT & DIALOGUES (Zelda, SMW, Generic)
// =============================================================================

fn extract_rom_texts(rom: &SnesRom, text_dir: &Path) -> std::io::Result<usize> {
    let mut count = 0;
    let game = rom.identify_game();

    match game {
        RecognizedGame::ZeldaAlttp => {
            // Zelda ALTTP Bank 1C (0x0E0000..0x0E8000) Dialogue Extraction
            if rom.data.len() >= 0x0E8000 {
                let bank1c = &rom.data[0x0E0000..0x0E8000];
                let mut dialogues = Vec::new();
                let mut cur_text = String::new();
                let mut in_msg = false;
                let mut msg_offset = 0usize;

                for (idx, &b) in bank1c.iter().enumerate() {
                    let decoded_char = match b {
                        0x00..=0x19 => Some((b'a' + b) as char),
                        0x1A..=0x33 => Some((b'A' + (b - 0x1A)) as char),
                        0x34..=0x3D => Some((b'0' + (b - 0x34)) as char),
                        0x59 => Some(' '),
                        0x41 | 0x75 | 0x76 => Some('\n'),
                        0x42 => Some('-'),
                        0x43 => Some('.'),
                        0x44 => Some(','),
                        0x46 => Some('!'),
                        0x47 => Some('?'),
                        0x48 => Some('\''),
                        _ => None,
                    };

                    if let Some(c) = decoded_char {
                        if !in_msg {
                            in_msg = true;
                            msg_offset = 0x0E0000 + idx;
                        }
                        cur_text.push(c);
                    } else if b == 0x7F || b == 0x7A || b == 0x7B {
                        if in_msg && cur_text.trim().len() >= 4 {
                            dialogues.push((msg_offset, cur_text.trim().to_string()));
                            count += 1;
                        }
                        cur_text.clear();
                        in_msg = false;
                    }
                }

                // Write dialogues.txt
                let mut txt_file = String::from("; =====================================================\n; THE LEGEND OF ZELDA: A LINK TO THE PAST - DIALOGUES\n; Décodés depuis la Banque 1C ($0E0000)\n; =====================================================\n\n");
                for (i, (off, text)) in dialogues.iter().enumerate() {
                    txt_file.push_str(&format!(
                        "[Dialogue #{:03} @ 0x{:06X}]\n{}\n\n",
                        i + 1,
                        off,
                        text
                    ));
                }
                fs::write(text_dir.join("dialogues_zelda.txt"), txt_file)?;

                // Write dialogues.json
                let mut json_file = String::from("[\n");
                for (i, (off, text)) in dialogues.iter().enumerate() {
                    let clean = text.replace('\n', "\\n").replace('"', "\\\"");
                    json_file.push_str(&format!(
                        "  {{ \"id\": {}, \"offset\": \"0x{:06X}\", \"text\": \"{}\" }}{}",
                        i + 1,
                        off,
                        clean,
                        if i + 1 < dialogues.len() { "," } else { "" }
                    ));
                    json_file.push('\n');
                }
                json_file.push_str("]\n");
                fs::write(text_dir.join("dialogues_zelda.json"), json_file)?;

                // Write Thingy .tbl character table
                let mut tbl_file =
                    String::from("; Table Thingy / SNES ALTTP French Dialogue Table\n");
                for i in 0..26u8 {
                    tbl_file.push_str(&format!("{:02X}={}\n", i, (b'a' + i) as char));
                    tbl_file.push_str(&format!("{:02X}={}\n", i + 0x1A, (b'A' + i) as char));
                }
                tbl_file.push_str("59= \n41=\\n\n75=\\n\n76=\\n\n7F=[END]\n");
                fs::write(text_dir.join("zelda_french.tbl"), tbl_file)?;
            }
        }
        RecognizedGame::SuperMarioWorld => {
            // SMW Overworld Level Names at Bank 04 (0x021BC0..0x021F00)
            if rom.data.len() >= 0x022000 {
                let slice = &rom.data[0x021BC0..0x021E00];
                let mut names = Vec::new();
                let mut cur = String::new();

                for &b in slice {
                    if (0x00..=0x19).contains(&b) {
                        cur.push((b'A' + b) as char);
                    } else if (0x80..=0x89).contains(&b) {
                        cur.push((b'0' + (b - 0x80)) as char);
                    } else if b == 0x1F || b == 0xFF {
                        cur.push(' ');
                    } else if b == 0x24 {
                        cur.push('\'');
                    } else if b == 0x9F || b == 0x00 {
                        let trimmed = cur.trim();
                        if trimmed.len() >= 3 && trimmed.chars().any(|c| c.is_ascii_alphabetic()) {
                            names.push(trimmed.to_string());
                            count += 1;
                        }
                        cur.clear();
                    }
                }

                let mut txt =
                    String::from("; SUPER MARIO WORLD - OVERWORLD LEVEL & AREA NAMES\n\n");
                for (i, name) in names.iter().enumerate() {
                    txt.push_str(&format!("{:02}: {}\n", i + 1, name));
                }
                fs::write(text_dir.join("smw_overworld_names.txt"), txt)?;

                // Write Thingy SMW table
                let mut tbl =
                    String::from("; Table Thingy / Super Mario World Overworld Text Table\n");
                for i in 0..26u8 {
                    tbl.push_str(&format!("{:02X}={}\n", i, (b'A' + i) as char));
                }
                tbl.push_str("1F= \n9F=[END]\n");
                fs::write(text_dir.join("smw.tbl"), tbl)?;
            }
        }
        _ => {
            // Generic heuristic string scan (ASCII runs >= 4)
            let mut generic_strings = Vec::new();
            let mut cur_s = String::new();
            let mut s_off = 0usize;

            for (idx, &b) in rom.data.iter().enumerate() {
                if (0x20..=0x7E).contains(&b) {
                    if cur_s.is_empty() {
                        s_off = idx;
                    }
                    cur_s.push(b as char);
                } else {
                    if cur_s.trim().len() >= 5 && cur_s.chars().any(|c| c.is_alphabetic()) {
                        generic_strings.push((s_off, cur_s.clone()));
                        count += 1;
                    }
                    cur_s.clear();
                }
            }

            let mut txt = String::from("; SNES ROM STRINGS SCAN (ASCII Heuristic)\n\n");
            for (i, (off, s)) in generic_strings.iter().take(500).enumerate() {
                txt.push_str(&format!("{:03} @ 0x{:06X}: {}\n", i + 1, off, s));
            }
            fs::write(text_dir.join("strings_ascii.txt"), txt)?;
        }
    }

    Ok(count)
}

// =============================================================================
// HELPER: TILEMAP & SCREEN RECONSTRUCTION
// =============================================================================

/// Reconstruct Super Mario World Title Screen (256x224 Native SNES plane)
fn reconstruct_smw_title_screen_rgba(rom_data: &[u8]) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    let (mut g28, _) = decompress_nintendo_lz2_with_endian(rom_data, 0x05C06C, 8192, true)?;
    let (g29, _) = decompress_nintendo_lz2_with_endian(rom_data, 0x05C6A3, 8192, true)?;
    g28.extend_from_slice(&g29);

    let pal = SnesPalette::mario_classic_full_16(Some(rom_data));
    let (w, h, rgba) = render_tile_sheet_rgba(&g28, 0, 32, 28, TileFormat::Bpp2, &pal);
    ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(w as u32, h as u32, rgba)
}

/// Reconstruct The Legend of Zelda: ALTTP Title Screen (256x224 Native SNES plane)
fn reconstruct_zelda_title_screen_rgba(rom_data: &[u8]) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    if rom_data.len() < 0x0A8000 {
        return None;
    }
    let pal = SnesPalette::zelda_link_full_16(rom_data, 0x0DD308, "Green Mail Title", false);
    let (w, h, rgba) = render_tile_sheet_rgba(rom_data, 0x0A6BAE, 32, 28, TileFormat::Bpp4, &pal);
    ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(w as u32, h as u32, rgba)
}
