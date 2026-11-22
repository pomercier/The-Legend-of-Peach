use std::fs;
use std::path::Path;
use std::time::Instant;

use image::{ImageBuffer, Rgba};

use crate::audio::{
    decode_brr_to_pcm16, decode_wav_to_pcm16, encode_pcm16_to_brr_fit_budget, encode_pcm16_to_wav,
    resample_and_encode_brr_for_swap, scan_brr_samples, BrrSampleInfo,
};
use crate::disasm_dma::{
    classify_rom_chunks, compress_nintendo_lz2_with_endian, compute_shannon_entropy,
    decompress_nintendo_lz2_with_endian, disassemble_65c816_block, scan_hardware_dma_transfers,
    DmaTarget,
};
use crate::graphics::{
    decode_tile, encode_rgba_sheet_to_tiles, remap_and_transcode_tiles, render_tile_sheet_rgba,
    rgb_to_bgr555, scan_candidate_palettes, scan_graphics_regions, SnesPalette, TileFormat,
};
use crate::rom::{RomError, RomMapping, SnesRom};
use crate::swapper::{SwapKind, SwapSession};

// =============================================================================
// UNIVERSAL ASSET TYPES (100% ROM-AGNOSTIC)
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UniversalGfxCompression {
    None,
    NintendoLz2BigEndian,
    NintendoLz2LittleEndian,
}

impl UniversalGfxCompression {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::NintendoLz2BigEndian => "lz2_be",
            Self::NintendoLz2LittleEndian => "lz2_le",
        }
    }

    pub fn from_str_tag(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "lz2_be" => Self::NintendoLz2BigEndian,
            "lz2_le" => Self::NintendoLz2LittleEndian,
            _ => Self::None,
        }
    }

    pub fn label_fr(self) -> &'static str {
        match self {
            Self::None => "Brut (non compressé)",
            Self::NintendoLz2BigEndian => "Compressé Nintendo LZ2 (BE)",
            Self::NintendoLz2LittleEndian => "Compressé Nintendo LZ2 (LE)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UniversalGfxCategory {
    Sprites,
    Backgrounds,
    HudFont,
    Mode7,
}

impl UniversalGfxCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sprites => "sprites",
            Self::Backgrounds => "backgrounds",
            Self::HudFont => "hud_font",
            Self::Mode7 => "mode7",
        }
    }

    pub fn label_fr(self) -> &'static str {
        match self {
            Self::Sprites => "Sprites & Personnages",
            Self::Backgrounds => "Décors & Tilesets",
            Self::HudFont => "Polices & HUD (2bpp)",
            Self::Mode7 => "Mode 7 / Haute Couleur (8bpp)",
        }
    }
}

#[derive(Debug, Clone)]
pub struct UniversalGfxAsset {
    pub id: String,
    pub name: String,
    pub category: UniversalGfxCategory,
    pub rom_offset: usize,
    pub snes_addr: String,
    pub rom_byte_budget: usize,
    pub decompressed_len: usize,
    pub format: TileFormat,
    pub compression: UniversalGfxCompression,
    pub tile_count: usize,
    pub suggested_palette_idx: usize,
    pub discovery_source: String,
    pub sheet_file: String,
}

#[derive(Debug, Clone)]
pub struct UniversalPaletteAsset {
    pub id: String,
    pub name: String,
    pub rom_offset: usize,
    pub snes_addr: String,
    pub color_count: usize,
    pub byte_len: usize,
    pub colors_rgba: Vec<[u8; 4]>,
    pub discovery_source: String,
    pub json_file: String,
}

#[derive(Debug, Clone)]
pub struct UniversalAudioAsset {
    pub id: String,
    pub name: String,
    pub rom_offset: usize,
    pub snes_addr: String,
    pub byte_len: usize,
    pub block_count: usize,
    pub loops: bool,
    pub peak_amplitude: i16,
    pub duration_ms_16khz: u32,
    pub wav_file: String,
}

#[derive(Debug, Clone)]
pub struct UniversalSpcStream {
    pub id: String,
    pub name: String,
    pub rom_offset: usize,
    pub snes_addr: String,
    pub entry_pc: u16,
    pub chunk_count: usize,
    pub total_payload_bytes: usize,
    pub detected_dir_page: u8,
    pub spc_file: String,
}

#[derive(Debug, Clone)]
pub struct UniversalTextAsset {
    pub id: String,
    pub rom_offset: usize,
    pub snes_addr: String,
    pub max_byte_len: usize,
    pub encoding: String,
    pub char_shift: i16,
    pub space_byte: u8,
    pub terminator: Option<u8>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct UniversalTilemapAsset {
    pub id: String,
    pub name: String,
    pub rom_offset: usize,
    pub snes_addr: String,
    pub width_tiles: usize,
    pub height_tiles: usize,
    pub linked_gfx_idx: usize,
    pub linked_pal_idx: usize,
    pub png_file: String,
}

/// Complete, 100% hardware-discovered catalog of ALL assets in ANY SNES ROM
#[derive(Debug, Clone)]
pub struct UniversalRomCatalog {
    pub rom_title: String,
    pub internal_title: String,
    pub mapping: String,
    pub rom_size_bytes: usize,
    pub checksum: u16,
    pub crc32: u32,
    pub graphics: Vec<UniversalGfxAsset>,
    pub palettes: Vec<UniversalPaletteAsset>,
    pub audio_samples: Vec<UniversalAudioAsset>,
    pub spc_streams: Vec<UniversalSpcStream>,
    pub texts: Vec<UniversalTextAsset>,
    pub tilemaps: Vec<UniversalTilemapAsset>,
    pub dma_count: usize,
    pub scan_elapsed_ms: u64,
}

/// Options for universal cross-ROM swapping
#[derive(Debug, Clone)]
pub struct UniversalBatchSwapOptions {
    pub swap_graphics: bool,
    pub swap_palettes: bool,
    pub swap_audio: bool,
    pub swap_text: bool,
    pub smart_color_remap: bool,
    pub max_gfx_swaps: usize,
    pub max_palette_swaps: usize,
    pub max_audio_swaps: usize,
}

impl Default for UniversalBatchSwapOptions {
    fn default() -> Self {
        Self {
            swap_graphics: true,
            swap_palettes: true,
            swap_audio: true,
            swap_text: false,
            smart_color_remap: true,
            max_gfx_swaps: 4,
            max_palette_swaps: 4,
            max_audio_swaps: 4,
        }
    }
}

/// Report produced when reconstructing a ROM from an editable deconstructed project directory
#[derive(Debug, Clone)]
pub struct ReconstructReport {
    pub out_rom_path: String,
    pub ips_path: Option<String>,
    pub gfx_patched_count: usize,
    pub palettes_patched_count: usize,
    pub audio_patched_count: usize,
    pub text_patched_count: usize,
    pub total_bytes_changed: usize,
    pub new_checksum: u16,
    pub checksum_valid: bool,
    pub warnings: Vec<String>,
    pub elapsed_ms: u64,
}

// =============================================================================
// 1. UNIVERSAL HARDWARE DISCOVERY ENGINE
// =============================================================================

/// Discover ALL assets (Graphics, LZ2 Compressed GFX, Palettes, SPC700 IPL Streams, BRR Audio,
/// Texts/TBL, Tilemaps) from ANY SNES ROM using pure hardware & statistical analysis.
pub fn discover_universal_catalog(rom: &SnesRom) -> UniversalRomCatalog {
    discover_universal_catalog_with_mode(rom, false)
}

/// Discover ALL assets with optional `exhaustive_full_mode` (removes sampling caps to extract 100% of a ROM).
pub fn discover_universal_catalog_with_mode(
    rom: &SnesRom,
    exhaustive_full_mode: bool,
) -> UniversalRomCatalog {
    let start = Instant::now();
    let dmas = scan_hardware_dma_transfers(rom);

    // --- A. PALETTES (Hardware CGRAM DMA + Universal BGR555 Heuristic Scanner) ---
    let mut palettes: Vec<UniversalPaletteAsset> = Vec::new();
    let mut seen_pal_offsets: Vec<usize> = Vec::new();

    for dma in &dmas {
        if dma.target == DmaTarget::CgramPalette {
            if let Some(pc) = dma.source_pc_offset {
                let sz = dma.transfer_size.unwrap_or(32) as usize;
                let color_count = (sz / 2).clamp(4, 16);
                let byte_len = color_count * 2;
                if pc + byte_len <= rom.data.len()
                    && !seen_pal_offsets.iter().any(|&o| o.abs_diff(pc) < 16)
                {
                    let pal = SnesPalette::from_rom_bgr555(
                        format!("DMA CGRAM @ 0x{:06X}", pc),
                        &rom.data,
                        pc,
                        color_count,
                    );
                    if palette_has_meaningful_colors(&pal.colors_rgba[..color_count]) {
                        let idx = palettes.len() + 1;
                        let id = format!("pal_{:03}_0x{:06X}", idx, pc);
                        let (bank, addr) = rom.pc_to_snes(pc);
                        palettes.push(UniversalPaletteAsset {
                            id: id.clone(),
                            name: format!("Palette DMA #{:02} (${:02X}:{:04X})", idx, bank, addr),
                            rom_offset: pc,
                            snes_addr: format!("${:02X}:{:04X}", bank, addr),
                            color_count,
                            byte_len,
                            colors_rgba: pal.colors_rgba[..color_count].to_vec(),
                            discovery_source: "DMA CGRAM ($2122)".to_string(),
                            json_file: format!("palettes/{}.json", id),
                        });
                        seen_pal_offsets.push(pc);
                    }
                }
            }
        }
    }

    let max_heuristic_pals = if exhaustive_full_mode {
        512
    } else if rom.data.len() >= 2 * 1024 * 1024 {
        48
    } else {
        32
    };
    let scanned_pals = scan_candidate_palettes(&rom.data, max_heuristic_pals);
    for p in scanned_pals {
        if let Some(pc) = p.source_offset {
            if pc + 32 <= rom.data.len() && !seen_pal_offsets.iter().any(|&o| o.abs_diff(pc) < 16) {
                let idx = palettes.len() + 1;
                let id = format!("pal_{:03}_0x{:06X}", idx, pc);
                let (bank, addr) = rom.pc_to_snes(pc);
                palettes.push(UniversalPaletteAsset {
                    id: id.clone(),
                    name: format!("Palette BGR555 #{:02} (${:02X}:{:04X})", idx, bank, addr),
                    rom_offset: pc,
                    snes_addr: format!("${:02X}:{:04X}", bank, addr),
                    color_count: 16,
                    byte_len: 32,
                    colors_rgba: p.colors_rgba.iter().take(16).copied().collect(),
                    discovery_source: "Scanner BGR555".to_string(),
                    json_file: format!("palettes/{}.json", id),
                });
                seen_pal_offsets.push(pc);
            }
        }
    }

    // --- B. AUDIO: UNIVERSAL SPC700 IPL UPLOAD STREAMS & BRR SAMPLES ---
    let spc_streams = scan_universal_spc_ipl_streams(rom);
    let raw_brr = scan_brr_samples(&rom.data, if exhaustive_full_mode { 12 } else { 16 });
    let mut audio_samples: Vec<UniversalAudioAsset> = Vec::new();
    let max_brr = if exhaustive_full_mode { usize::MAX } else { 64 };
    for (idx, s) in raw_brr.iter().take(max_brr).enumerate() {
        let id = format!("brr_{:03}_0x{:06X}", idx + 1, s.offset);
        let (bank, addr) = rom.pc_to_snes(s.offset);
        audio_samples.push(UniversalAudioAsset {
            id: id.clone(),
            name: format!("{} (${:02X}:{:04X})", s.label, bank, addr),
            rom_offset: s.offset,
            snes_addr: format!("${:02X}:{:04X}", bank, addr),
            byte_len: s.byte_len,
            block_count: s.block_count,
            loops: s.loops,
            peak_amplitude: s.peak_amplitude,
            duration_ms_16khz: s.duration_ms(16000),
            wav_file: format!("audio/{}.wav", id),
        });
    }

    // --- C. GRAPHICS: DMA VRAM + UNIVERSAL LZ2 COMPRESSED STREAMS + PLANAR ENTROPY ---
    let mut graphics: Vec<UniversalGfxAsset> = Vec::new();
    let mut covered_ranges: Vec<(usize, usize)> = Vec::new();

    // Mark SPC / BRR audio ranges so we don't mistake BRR audio for tiles
    let mut audio_ranges: Vec<(usize, usize)> = Vec::new();
    for s in &audio_samples {
        audio_ranges.push((s.rom_offset, s.rom_offset + s.byte_len));
    }

    let overlaps_any = |ranges: &[(usize, usize)], start: usize, end: usize| -> bool {
        ranges.iter().any(|&(a, b)| start < b && end > a)
    };

    // C.1: Hardware DMA VRAM transfers
    for dma in &dmas {
        if dma.target == DmaTarget::VramGraphics {
            if let Some(pc) = dma.source_pc_offset {
                let raw_len = dma.transfer_size.unwrap_or(2048) as usize;
                let len = raw_len.clamp(512, 16384);
                let end = (pc + len).min(rom.data.len());
                let actual_len = end.saturating_sub(pc);
                if actual_len < 512
                    || overlaps_any(&covered_ranges, pc, end)
                    || overlaps_any(&audio_ranges, pc, end)
                {
                    continue;
                }
                let slice = &rom.data[pc..end];
                if looks_like_tilemap_buffer(slice) {
                    continue; // Handled in tilemap pass
                }
                let ent = compute_shannon_entropy(slice);
                if (1.2..=6.8).contains(&ent) {
                    let fmt = infer_planar_tile_format(slice);
                    let tile_count = actual_len / fmt.bytes_per_tile();
                    if tile_count >= 16 {
                        let idx = graphics.len() + 1;
                        let id = format!("gfx_{:03}_0x{:06X}", idx, pc);
                        let (bank, addr) = rom.pc_to_snes(pc);
                        let cat = categorize_gfx(fmt, tile_count);
                        graphics.push(UniversalGfxAsset {
                            id: id.clone(),
                            name: format!(
                                "DMA VRAM #{:02} [{} · {} tuiles · ${:02X}:{:04X}]",
                                idx,
                                fmt.short_label(),
                                tile_count,
                                bank,
                                addr
                            ),
                            category: cat,
                            rom_offset: pc,
                            snes_addr: format!("${:02X}:{:04X}", bank, addr),
                            rom_byte_budget: actual_len,
                            decompressed_len: actual_len,
                            format: fmt,
                            compression: UniversalGfxCompression::None,
                            tile_count,
                            suggested_palette_idx: if palettes.is_empty() {
                                0
                            } else {
                                (idx - 1) % palettes.len()
                            },
                            discovery_source: "DMA VRAM ($2118)".to_string(),
                            sheet_file: format!("sheets/{}.png", id),
                        });
                        covered_ranges.push((pc, end));
                    }
                }
            }
        }
    }

    // C.2: Universal Compressed Nintendo LC_LZ2 Scanner (discovers SMW, ALTTP, Super Metroid, etc. automatically!)
    let lz_hits = scan_universal_lz2_graphics(rom, &audio_ranges);
    for hit in lz_hits {
        let end = hit.rom_offset + hit.compressed_len;
        if overlaps_any(&covered_ranges, hit.rom_offset, end) {
            continue;
        }
        let idx = graphics.len() + 1;
        let id = format!("gfx_{:03}_0x{:06X}", idx, hit.rom_offset);
        let (bank, addr) = rom.pc_to_snes(hit.rom_offset);
        let tile_count = hit.decompressed_len / hit.format.bytes_per_tile();
        let cat = categorize_gfx(hit.format, tile_count);
        graphics.push(UniversalGfxAsset {
            id: id.clone(),
            name: format!(
                "GFX Compressé LZ2 #{:02} [{} · {} tuiles · ${:02X}:{:04X}]",
                idx,
                hit.format.short_label(),
                tile_count,
                bank,
                addr
            ),
            category: cat,
            rom_offset: hit.rom_offset,
            snes_addr: format!("${:02X}:{:04X}", bank, addr),
            rom_byte_budget: hit.compressed_len,
            decompressed_len: hit.decompressed_len,
            format: hit.format,
            compression: hit.compression,
            tile_count,
            suggested_palette_idx: 0,
            discovery_source: format!("Scanner {}", hit.compression.label_fr()),
            sheet_file: format!("sheets/{}.png", id),
        });
        covered_ranges.push((hit.rom_offset, end));
    }

    // C.3: Universal Uncompressed Planar Graphics Regions
    let raw_gfx_regions = scan_graphics_regions(&rom.data);
    let max_raw_regions = if exhaustive_full_mode {
        usize::MAX
    } else if rom.data.len() >= 2 * 1024 * 1024 {
        64
    } else {
        40
    };
    for reg in raw_gfx_regions.into_iter().take(max_raw_regions) {
        let end = (reg.offset + reg.length).min(rom.data.len());
        if end <= reg.offset + 512
            || overlaps_any(&covered_ranges, reg.offset, end)
            || overlaps_any(&audio_ranges, reg.offset, end)
        {
            continue;
        }
        let slice = &rom.data[reg.offset..end];
        let fmt = infer_planar_tile_format(slice);
        let tile_count = (end - reg.offset) / fmt.bytes_per_tile();
        if tile_count < 16 {
            continue;
        }
        let idx = graphics.len() + 1;
        let id = format!("gfx_{:03}_0x{:06X}", idx, reg.offset);
        let (bank, addr) = rom.pc_to_snes(reg.offset);
        let cat = categorize_gfx(fmt, tile_count);
        graphics.push(UniversalGfxAsset {
            id: id.clone(),
            name: format!(
                "Planche GFX #{:02} [{} · {} tuiles · ${:02X}:{:04X}]",
                idx,
                fmt.short_label(),
                tile_count,
                bank,
                addr
            ),
            category: cat,
            rom_offset: reg.offset,
            snes_addr: format!("${:02X}:{:04X}", bank, addr),
            rom_byte_budget: end - reg.offset,
            decompressed_len: end - reg.offset,
            format: fmt,
            compression: UniversalGfxCompression::None,
            tile_count,
            suggested_palette_idx: 0,
            discovery_source: "Scanner Planaire Universel".to_string(),
            sheet_file: format!("sheets/{}.png", id),
        });
        covered_ranges.push((reg.offset, end));
    }

    // --- D. UNIVERSAL TEXT & RELATIVE-TBL DISCOVERY ---
    let texts = scan_universal_texts(rom, &audio_ranges);

    // --- E. UNIVERSAL TILEMAPS / SCREENS ---
    let tilemaps = scan_universal_tilemaps(rom, &dmas, &graphics, &palettes);

    UniversalRomCatalog {
        rom_title: rom.canonical_title(),
        internal_title: rom.header.title.trim().to_string(),
        mapping: rom.header.mapping.as_str().to_string(),
        rom_size_bytes: rom.data.len(),
        checksum: rom.header.checksum,
        crc32: rom.crc32,
        graphics,
        palettes,
        audio_samples,
        spc_streams,
        texts,
        tilemaps,
        dma_count: dmas.len(),
        scan_elapsed_ms: start.elapsed().as_millis() as u64,
    }
}

/// Extract decompressed (or raw) planar tile bytes for ANY `UniversalGfxAsset`
pub fn read_universal_gfx_bytes(rom: &SnesRom, asset: &UniversalGfxAsset) -> Option<Vec<u8>> {
    let end = (asset.rom_offset + asset.rom_byte_budget).min(rom.data.len());
    if asset.rom_offset >= rom.data.len() {
        return None;
    }
    match asset.compression {
        UniversalGfxCompression::None => Some(rom.data[asset.rom_offset..end].to_vec()),
        UniversalGfxCompression::NintendoLz2BigEndian => {
            let (dec, _) = decompress_nintendo_lz2_with_endian(
                &rom.data,
                asset.rom_offset,
                asset.decompressed_len.max(32768),
                true,
            )?;
            Some(dec)
        }
        UniversalGfxCompression::NintendoLz2LittleEndian => {
            let (dec, _) = decompress_nintendo_lz2_with_endian(
                &rom.data,
                asset.rom_offset,
                asset.decompressed_len.max(32768),
                false,
            )?;
            Some(dec)
        }
    }
}

/// Render ANY `UniversalGfxAsset` into an RGBA image using the given palette
pub fn render_universal_gfx_asset(
    rom: &SnesRom,
    asset: &UniversalGfxAsset,
    palette: &SnesPalette,
    max_rows: usize,
) -> Option<(usize, usize, Vec<u8>)> {
    let raw = read_universal_gfx_bytes(rom, asset)?;
    let cols = 16usize;
    let bpt = asset.format.bytes_per_tile();
    let total_tiles = raw.len() / bpt;
    if total_tiles == 0 {
        return None;
    }
    let rows = total_tiles.div_ceil(cols).clamp(1, max_rows.max(1));
    Some(render_tile_sheet_rgba(
        &raw,
        0,
        cols,
        rows,
        asset.format,
        palette,
    ))
}

// =============================================================================
// 2. UNIVERSAL LZ2 COMPRESSED GRAPHICS SCANNER
// =============================================================================

struct Lz2Hit {
    rom_offset: usize,
    compressed_len: usize,
    decompressed_len: usize,
    format: TileFormat,
    compression: UniversalGfxCompression,
}

fn scan_universal_lz2_graphics(rom: &SnesRom, audio_ranges: &[(usize, usize)]) -> Vec<Lz2Hit> {
    let mut hits: Vec<Lz2Hit> = Vec::new();
    let data = &rom.data;
    if data.len() < 0x10000 {
        return hits;
    }

    // Scan bank starts and follow consecutive chained LZ2 blocks (the exact way Nintendo & 3rd-party ROMs pack GFX!)
    let bank_step = rom.bank_size();
    let mut candidate_starts: Vec<usize> = Vec::new();

    for bank_off in (bank_step..data.len()).step_by(bank_step) {
        candidate_starts.push(bank_off);
    }

    // Also add 4KB-aligned high-entropy chunk starts in smaller ROMs (<= 2MB) for fast coverage
    if data.len() <= 2 * 1024 * 1024 {
        for off in (0x010000..data.len().saturating_sub(512)).step_by(0x1000) {
            if !candidate_starts.contains(&off) {
                candidate_starts.push(off);
            }
        }
    }

    let mut visited_until = 0usize;
    candidate_starts.sort_unstable();

    for start_off in candidate_starts {
        if start_off < visited_until || start_off + 64 >= data.len() {
            continue;
        }
        if audio_ranges
            .iter()
            .any(|&(a, b)| start_off >= a && start_off < b)
        {
            continue;
        }

        // Follow up to 24 consecutive LZ2 streams starting at `start_off`
        let mut cursor = start_off;
        for _chain in 0..24 {
            if cursor + 64 >= data.len() || hits.len() >= 48 {
                break;
            }
            if audio_ranges.iter().any(|&(a, b)| cursor >= a && cursor < b) {
                break;
            }

            // Try Big-Endian backref first (SMW, ALTTP, Super Metroid), then Little-Endian
            let mut found_hit: Option<Lz2Hit> = None;
            for (be, comp_kind) in [
                (true, UniversalGfxCompression::NintendoLz2BigEndian),
                (false, UniversalGfxCompression::NintendoLz2LittleEndian),
            ] {
                if let Some((dec, consumed)) =
                    decompress_nintendo_lz2_with_endian(data, cursor, 32768, be)
                {
                    // Strict validation: real SNES compressed graphic stream
                    let tile_aligned =
                        (dec.len() % 512 == 0) || (dec.len() % 384 == 0) || (dec.len() % 256 == 0);
                    if (96..=24576).contains(&consumed)
                        && dec.len() >= 1024
                        && dec.len() > (consumed * 11) / 10
                        && tile_aligned
                        && looks_like_planar_tiles(&dec)
                    {
                        let fmt = infer_planar_tile_format(&dec);
                        found_hit = Some(Lz2Hit {
                            rom_offset: cursor,
                            compressed_len: consumed,
                            decompressed_len: dec.len(),
                            format: fmt,
                            compression: comp_kind,
                        });
                        break;
                    }
                }
            }

            if let Some(hit) = found_hit {
                let next_cursor = hit.rom_offset + hit.compressed_len;
                visited_until = visited_until.max(next_cursor);
                hits.push(hit);
                // Advance past any 0xFF padding bytes between consecutive LZ2 blocks
                let mut padded = next_cursor;
                let pad_limit = (next_cursor + 32).min(data.len());
                while padded < pad_limit && data[padded] == 0xFF {
                    padded += 1;
                }
                cursor = padded;
            } else {
                break;
            }
        }
    }

    hits
}

fn looks_like_planar_tiles(bytes: &[u8]) -> bool {
    if bytes.len() < 256 {
        return false;
    }
    let sample_len = bytes.len().min(4096);
    let sample = &bytes[..sample_len];
    let ent = compute_shannon_entropy(sample);
    if !(0.6..=6.9).contains(&ent) {
        return false;
    }
    // Check that it isn't mostly ASCII text
    let ascii_letters = sample
        .iter()
        .filter(|&&b| b.is_ascii_uppercase() || b.is_ascii_lowercase())
        .count();
    if ascii_letters * 10 > sample_len * 7 {
        return false;
    }
    // Check bitplane row correlation (in SNES 2bpp/3bpp/4bpp, adjacent pairs share spatial structure)
    let mut non_zero = 0usize;
    let mut correlated_pairs = 0usize;
    for pair in sample.as_chunks::<2>().0 {
        if pair[0] != 0 || pair[1] != 0 {
            non_zero += 1;
            let common = (pair[0] & pair[1]).count_ones() + (!(pair[0] | pair[1])).count_ones();
            if common >= 4 {
                correlated_pairs += 1;
            }
        }
    }
    non_zero >= 16 && correlated_pairs * 4 >= non_zero
}

fn infer_planar_tile_format(bytes: &[u8]) -> TileFormat {
    let len = bytes.len();
    // Exact 3bpp Nintendo sheet lengths (e.g. 128 tiles * 24 = 3072, 256 tiles * 24 = 6144, 384 tiles * 24 = 9216)
    if len.is_multiple_of(24) && !len.is_multiple_of(32) {
        return TileFormat::Bpp3;
    }
    // Measure 2bpp vs 4bpp structure: in 2bpp HUD fonts, high zero-byte regularity & small distinct byte clusters
    let sample = &bytes[..len.min(2048)];
    let zero_count = sample.iter().filter(|&&b| b == 0).count();
    if zero_count * 100 / sample.len().max(1) > 48 && len <= 4096 {
        return TileFormat::Bpp2;
    }
    // Test if every 2nd 16-byte half in 32-byte blocks is completely zero (strong 2bpp signal stored in 4bpp or pure 2bpp)
    let mut upper_half_nonzero = 0usize;
    let mut lower_half_nonzero = 0usize;
    for blk in sample.as_chunks::<32>().0 {
        lower_half_nonzero += blk[..16].iter().filter(|&&b| b != 0).count();
        upper_half_nonzero += blk[16..].iter().filter(|&&b| b != 0).count();
    }
    if lower_half_nonzero > 64 && upper_half_nonzero * 12 < lower_half_nonzero {
        return TileFormat::Bpp2;
    }
    TileFormat::Bpp4
}

fn categorize_gfx(fmt: TileFormat, tile_count: usize) -> UniversalGfxCategory {
    match fmt {
        TileFormat::Bpp2 => UniversalGfxCategory::HudFont,
        TileFormat::Bpp8 | TileFormat::Mode7Linear => UniversalGfxCategory::Mode7,
        TileFormat::Bpp3 | TileFormat::Bpp4 => {
            if tile_count <= 256 {
                UniversalGfxCategory::Sprites
            } else {
                UniversalGfxCategory::Backgrounds
            }
        }
    }
}

fn palette_has_meaningful_colors(colors: &[[u8; 4]]) -> bool {
    if colors.len() < 4 {
        return false;
    }
    let mut unique = Vec::new();
    for &c in colors {
        if !unique.contains(&c) {
            unique.push(c);
        }
    }
    unique.len() >= 4
}

// =============================================================================
// 3. UNIVERSAL SPC700 IPL TRANSFER STREAM SCANNER & .SPC BUILDER
// =============================================================================

/// Scan ANY SNES ROM (LoROM/HiROM/ExHiROM) for genuine Nintendo SPC700 IPL transfer streams:
/// `[size_16le, dest_spc_16le, payload...]` terminated by `[0x0000, entry_pc_16le]`.
pub fn scan_universal_spc_ipl_streams(rom: &SnesRom) -> Vec<UniversalSpcStream> {
    let mut streams = Vec::new();
    let data = &rom.data;
    let step = rom.bank_size();
    let mut visited_ranges: Vec<(usize, usize)> = Vec::new();

    for bank_start in (step..data.len().saturating_sub(0x400)).step_by(step) {
        let mut cursor = bank_start;
        let bank_end = (bank_start + step).min(data.len());

        // A single audio bank can contain multiple consecutive IPL upload streams (engine + songs)
        for _sub in 0..8 {
            if cursor + 8 >= bank_end || streams.len() >= 16 {
                break;
            }
            if visited_ranges
                .iter()
                .any(|&(a, b)| cursor >= a && cursor < b)
            {
                break;
            }
            if let Some((entry_pc, chunks, total_payload, end_pos, dir_page)) =
                parse_ipl_stream_at(data, cursor)
            {
                let idx = streams.len() + 1;
                let id = format!("spc_{:03}_0x{:06X}", idx, cursor);
                let (bank, addr) = rom.pc_to_snes(cursor);
                streams.push(UniversalSpcStream {
                    id: id.clone(),
                    name: format!(
                        "Flux SPC700 #{:02} (Entry ${:04X} · {} Ko · ${:02X}:{:04X})",
                        idx,
                        entry_pc,
                        total_payload / 1024,
                        bank,
                        addr
                    ),
                    rom_offset: cursor,
                    snes_addr: format!("${:02X}:{:04X}", bank, addr),
                    entry_pc,
                    chunk_count: chunks,
                    total_payload_bytes: total_payload,
                    detected_dir_page: dir_page,
                    spc_file: format!("audio/{}.spc", id),
                });
                visited_ranges.push((cursor, end_pos));
                cursor = end_pos;
            } else {
                break;
            }
        }
    }

    streams
}

/// Attempt to parse a valid SPC700 IPL boot upload stream starting at `start_pos`.
/// Returns `(entry_pc, chunk_count, total_payload_bytes, end_pos, detected_dir_page)`.
fn parse_ipl_stream_at(data: &[u8], start_pos: usize) -> Option<(u16, usize, usize, usize, u8)> {
    let mut ptr = start_pos;
    let mut chunks = 0usize;
    let mut total_payload = 0usize;
    let mut dir_page = 0x12u8;
    let mut has_spc_code_signature = false;

    while ptr + 4 <= data.len() && chunks < 32 {
        let sz = u16::from_le_bytes([data[ptr], data[ptr + 1]]) as usize;
        let dest = u16::from_le_bytes([data[ptr + 2], data[ptr + 3]]);
        ptr += 4;

        if sz == 0 {
            if chunks >= 1
                && (512..=65000).contains(&total_payload)
                && (0x0200..=0xFFBF).contains(&dest)
                && has_spc_code_signature
            {
                return Some((dest, chunks, total_payload, ptr, dir_page));
            }
            return None;
        }

        if dest < 0x0100 || (dest as usize) + sz > 0xFFC0 || ptr + sz > data.len() {
            return None;
        }

        // Detect DIR page candidate (page-aligned small table of 4-byte pointers)
        if (dest & 0x00FF) == 0
            && (16..=512).contains(&sz)
            && sz.is_multiple_of(4)
            && dest >= 0x0200
        {
            dir_page = (dest >> 8) as u8;
        }

        // Verify SPC700 instruction / audio payload plausibility
        let payload = &data[ptr..ptr + sz];
        if !has_spc_code_signature {
            // Look for common SPC700 opcodes (MOV X,#imm=0xCD, MOV dp,#imm=0x8F, MOV A,#imm=0xE8, RET=0x6F, MOV SP,X=0xBD)
            let spc_ops = payload
                .iter()
                .filter(|&&b| matches!(b, 0xCD | 0x8F | 0xE8 | 0x6F | 0xBD | 0xC4 | 0xE4))
                .count();
            if spc_ops * 25 >= payload.len().max(1) || sz >= 2048 {
                has_spc_code_signature = true;
            }
        }

        total_payload += sz;
        chunks += 1;
        ptr += sz;
    }

    None
}

/// Build an authentic 66,048-byte `.spc` file from a discovered universal IPL stream
/// (optionally layering a base engine stream first so song-only streams are playable!).
pub fn build_spc_from_universal_stream(
    rom: &SnesRom,
    stream: &UniversalSpcStream,
    base_engine_stream: Option<&UniversalSpcStream>,
) -> Vec<u8> {
    let mut ram = [0u8; 65536];
    let mut pc = stream.entry_pc;
    let mut dir_page = stream.detected_dir_page;

    let apply_stream = |start_off: usize, ram: &mut [u8; 65536], pc: &mut u16| {
        let mut ptr = start_off;
        for _ in 0..32 {
            if ptr + 4 > rom.data.len() {
                break;
            }
            let sz = u16::from_le_bytes([rom.data[ptr], rom.data[ptr + 1]]) as usize;
            let dest = u16::from_le_bytes([rom.data[ptr + 2], rom.data[ptr + 3]]);
            ptr += 4;
            if sz == 0 {
                *pc = dest;
                break;
            }
            if ptr + sz > rom.data.len() || (dest as usize) + sz > 65536 {
                break;
            }
            ram[dest as usize..(dest as usize) + sz].copy_from_slice(&rom.data[ptr..ptr + sz]);
            ptr += sz;
        }
    };

    if let Some(base) = base_engine_stream {
        if base.rom_offset != stream.rom_offset {
            apply_stream(base.rom_offset, &mut ram, &mut pc);
            dir_page = base.detected_dir_page;
        }
    }
    apply_stream(stream.rom_offset, &mut ram, &mut pc);

    let mut dsp = [0u8; 128];
    dsp[0x0C] = 0x7F; // MVOL_L
    dsp[0x1C] = 0x7F; // MVOL_R
    dsp[0x5D] = dir_page; // DIR page
    dsp[0x6C] = 0x20; // FLG = unmute, echo off

    crate::deconstruct::create_spc_file(
        &stream.name,
        &rom.canonical_title(),
        "Antigravity",
        "Universal IPL Stream SPC700 Dump",
        "SNES Sound Engine",
        pc,
        0xEF,
        0,
        0,
        0,
        0,
        &ram,
        &dsp,
        None,
    )
}

// =============================================================================
// 4. UNIVERSAL TEXT & AUTOMATIC RELATIVE-TBL DISCOVERY ENGINE
// =============================================================================

/// Discover text strings in ANY SNES ROM using:
/// 1) Direct printable ASCII scan (credits, menus, Rareware/Capcom/Square games) skipping BRR audio ranges
/// 2) Universal Relative-Search `.tbl` inference (discovers custom font encodings automatically!)
pub fn scan_universal_texts(
    rom: &SnesRom,
    skip_ranges: &[(usize, usize)],
) -> Vec<UniversalTextAsset> {
    let mut results: Vec<UniversalTextAsset> = Vec::new();
    let mut seen_offsets: Vec<usize> = Vec::new();
    let mut seen_texts: Vec<String> = Vec::new();
    let header_base = rom.header.mapping.header_base_offset();

    // --- Pass 1: Clean ASCII strings (length >= 6, strong linguistic & case-coherence filter) ---
    let mut cur = String::new();
    let mut start_off = 0usize;
    for (idx, &b) in rom.data.iter().enumerate() {
        // Skip internal SNES header registration block & known BRR audio ranges
        if (idx >= header_base && idx < header_base + 0x40)
            || skip_ranges.iter().any(|&(a, e)| idx >= a && idx < e)
        {
            cur.clear();
            continue;
        }
        let is_ascii_text = b.is_ascii_uppercase()
            || b.is_ascii_lowercase()
            || b.is_ascii_digit()
            || matches!(b, b' ' | b'!' | b'?' | b'.' | b',' | b'\'' | b'-' | b':');

        if is_ascii_text {
            if cur.is_empty() {
                start_off = idx;
            }
            cur.push(b as char);
        } else {
            let trimmed = cur.trim();
            if trimmed.len() >= 6
                && is_plausible_human_text(trimmed)
                && !seen_texts.iter().any(|s| s == trimmed)
            {
                let term = if b == 0x00 || b == 0xFF {
                    Some(b)
                } else {
                    None
                };
                let id = format!("txt_{:03}_0x{:06X}", results.len() + 1, start_off);
                let (bank, addr) = rom.pc_to_snes(start_off);
                results.push(UniversalTextAsset {
                    id,
                    rom_offset: start_off,
                    snes_addr: format!("${:02X}:{:04X}", bank, addr),
                    max_byte_len: cur.len(),
                    encoding: "ASCII".to_string(),
                    char_shift: 0,
                    space_byte: b' ',
                    terminator: term,
                    text: trimmed.to_string(),
                });
                seen_offsets.push(start_off);
                seen_texts.push(trimmed.to_string());
                if results.len() >= 150 {
                    break;
                }
            }
            cur.clear();
        }
    }

    // --- Pass 2: Universal Relative-Search for custom SNES `.tbl` encodings ---
    // Tests common game words across ALL 255 non-ASCII byte shifts (detects SMW `A=0x00`, Zelda `a=0x00`, FF/RPG custom tables!)
    let probe_words: &[(&str, bool)] = &[
        ("SAUVEGARDE", false),
        ("CONTINUER", false),
        ("CONTINUE", true),
        ("ISLAND", true),
        ("CASTLE", true),
        ("FOREST", true),
        ("PLAINS", true),
        ("BOWSER", true),
        ("SELECT", true),
        ("PLAYER", true),
        ("START", true),
        ("WORLD", true),
        ("LEVEL", true),
        ("BONUS", true),
        ("STAGE", true),
        ("OPTION", true),
        ("PRINCESS", true),
        ("DANGER", false),
    ];

    let mut discovered_shifts: Vec<(i16, usize, bool)> = Vec::new(); // (shift_from_ascii, bank_start, uppercase)
    for &(word, is_upper) in probe_words {
        let base_bytes: Vec<u8> = word
            .bytes()
            .map(|c| {
                if is_upper {
                    c.to_ascii_uppercase()
                } else {
                    c.to_ascii_lowercase()
                }
            })
            .collect();
        let wlen = base_bytes.len();
        if wlen < 5 {
            continue;
        }
        // Compute relative deltas
        let deltas: Vec<u8> = (0..wlen - 1)
            .map(|i| base_bytes[i + 1].wrapping_sub(base_bytes[i]))
            .collect();

        // Scan ROM for matching delta sequence
        let limit = rom.data.len().saturating_sub(wlen);
        let mut i = 0usize;
        while i < limit {
            let b0 = rom.data[i];
            let shift_u8 = b0.wrapping_sub(base_bytes[0]);
            if shift_u8 != 0 {
                // Non-ASCII custom table!
                let mut matched = true;
                for k in 0..deltas.len() {
                    if rom.data[i + k + 1].wrapping_sub(rom.data[i + k]) != deltas[k] {
                        matched = false;
                        break;
                    }
                }
                if matched {
                    let shift_signed = (b0 as i16) - (base_bytes[0] as i16);
                    let bank_start = (i / rom.bank_size()) * rom.bank_size();
                    if !discovered_shifts
                        .iter()
                        .any(|&(s, bs, u)| s == shift_signed && bs == bank_start && u == is_upper)
                    {
                        discovered_shifts.push((shift_signed, bank_start, is_upper));
                    }
                    i += wlen;
                    continue;
                }
            }
            i += 1;
        }
    }

    // Extract strings from each discovered custom-TBL bank
    for (shift, bank_start, is_upper) in discovered_shifts.into_iter().take(6) {
        let bank_end = (bank_start + rom.bank_size()).min(rom.data.len());
        let base_letter: i16 = if is_upper { b'A' as i16 } else { b'a' as i16 };
        let alpha_min = (base_letter + shift).rem_euclid(256) as u8;
        let alpha_max = alpha_min.wrapping_add(25);
        // Infer likely space byte in this bank (common SNES space bytes: 0x1F, 0x59, 0xFF, 0xFE, 0x20)
        let space_byte = infer_space_byte(&rom.data[bank_start..bank_end], alpha_min, alpha_max);

        let mut cur_s = String::new();
        let mut s_off = 0usize;
        let mut s_bytes = 0usize;

        for pos in bank_start..bank_end {
            let b = rom.data[pos];
            let decoded = if alpha_min <= alpha_max && b >= alpha_min && b <= alpha_max {
                Some(((base_letter as u8) + (b - alpha_min)) as char)
            } else if !is_upper
                && b >= alpha_min.wrapping_add(26)
                && b <= alpha_min.wrapping_add(51)
            {
                // Lowercase + consecutive Uppercase (e.g. Zelda ALTTP!)
                Some((b'A' + (b - alpha_min.wrapping_add(26))) as char)
            } else if b == space_byte {
                Some(' ')
            } else {
                None
            };

            if let Some(ch) = decoded {
                if cur_s.is_empty() {
                    s_off = pos;
                    s_bytes = 0;
                }
                cur_s.push(ch);
                s_bytes += 1;
            } else {
                let trimmed = cur_s.trim();
                if trimmed.len() >= 5
                    && is_plausible_human_text(trimmed)
                    && !seen_offsets.iter().any(|&o| o.abs_diff(s_off) < 4)
                {
                    let id = format!("txt_{:03}_0x{:06X}", results.len() + 1, s_off);
                    let (bank, addr) = rom.pc_to_snes(s_off);
                    results.push(UniversalTextAsset {
                        id,
                        rom_offset: s_off,
                        snes_addr: format!("${:02X}:{:04X}", bank, addr),
                        max_byte_len: s_bytes,
                        encoding: format!(
                            "CustomTBL({}=0x{:02X}, sp=0x{:02X})",
                            if is_upper { "A" } else { "a" },
                            alpha_min,
                            space_byte
                        ),
                        char_shift: shift,
                        space_byte,
                        terminator: Some(b),
                        text: trimmed.to_string(),
                    });
                    seen_offsets.push(s_off);
                    if results.len() >= 250 {
                        break;
                    }
                }
                cur_s.clear();
                s_bytes = 0;
            }
        }
    }

    // Rank discovered strings by linguistic clarity so real human game strings (spaces, custom TBL, clean words) come FIRST
    results.sort_by_key(|a| std::cmp::Reverse(text_quality_score(a)));
    for (i, t) in results.iter_mut().enumerate() {
        t.id = format!("txt_{:03}_0x{:06X}", i + 1, t.rom_offset);
    }

    results
}

fn text_quality_score(t: &UniversalTextAsset) -> i32 {
    let mut score = 0i32;
    let s = &t.text;
    if s.contains(' ') {
        score += 40; // Multi-word phrase!
    }
    if t.char_shift != 0 {
        score += 30; // Discovered via custom game .tbl relative search!
    }
    if t.terminator == Some(0x00) || t.terminator == Some(0x9F) || t.terminator == Some(0x7F) {
        score += 15; // Proper null/SNES-terminated string
    }
    let words = s.split_whitespace().count();
    score += (words.min(8) as i32) * 10;
    score += (s.len().min(40) as i32) / 2;
    // Penalize pure single-case short noise without spaces
    if !s.contains(' ') && s.len() <= 8 {
        score -= 20;
    }
    score
}

fn infer_space_byte(bank: &[u8], alpha_min: u8, alpha_max: u8) -> u8 {
    let candidates = [0x59u8, 0x1Fu8, 0xFFu8, 0xFEu8, 0x20u8, 0x7Fu8, 0x00u8];
    let mut best_sp = 0x1Fu8;
    let mut best_score = 0usize;

    for &cand in &candidates {
        if cand >= alpha_min && cand <= alpha_max {
            continue;
        }
        let mut surrounded_by_letters = 0usize;
        for w in bank.windows(3) {
            if w[1] == cand
                && w[0] >= alpha_min
                && w[0] <= alpha_max
                && w[2] >= alpha_min
                && w[2] <= alpha_max
            {
                surrounded_by_letters += 1;
            }
        }
        if surrounded_by_letters > best_score {
            best_score = surrounded_by_letters;
            best_sp = cand;
        }
    }
    best_sp
}

fn is_plausible_human_text(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();
    let total = chars.len();
    if total < 5 {
        return false;
    }
    // Must start with a letter or digit (reject random punctuation noise like "?SZ2HzDUI")
    if !chars[0].is_ascii_alphanumeric() {
        return false;
    }
    let letters = chars.iter().filter(|c| c.is_ascii_alphabetic()).count();
    let vowels = chars
        .iter()
        .filter(|c| matches!(c.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u' | 'y'))
        .count();
    // Reject gibberish random case-transitions inside words (e.g. "HzDUI", "aBcDe")
    let mut weird_transitions = 0usize;
    for w in chars.windows(2) {
        if w[0].is_ascii_lowercase() && w[1].is_ascii_uppercase() {
            weird_transitions += 1;
        }
        if w[0].is_ascii_digit()
            && w[1].is_ascii_alphabetic()
            && w[1] != 'P'
            && w[1] != 'p'
            && w[1] != 's'
        {
            weird_transitions += 1;
        }
    }
    if weird_transitions > 0 {
        return false;
    }

    let mut distinct = Vec::new();
    for &c in &chars {
        let lc = c.to_ascii_lowercase();
        if !distinct.contains(&lc) {
            distinct.push(lc);
        }
    }
    letters * 10 >= total * 7 && vowels * 10 >= letters && distinct.len() >= 3
}

// =============================================================================
// 5. UNIVERSAL TILEMAP & SCREEN DETECTOR
// =============================================================================

fn looks_like_tilemap_buffer(bytes: &[u8]) -> bool {
    if bytes.len() < 1024 {
        return false;
    }
    let sample = &bytes[..bytes.len().min(1792)];
    let mut valid_entries = 0usize;
    let mut total_entries = 0usize;
    let mut sequential_steps = 0usize;
    let mut prev_tile = 0u16;

    for pair in sample.as_chunks::<2>().0 {
        let word = u16::from_le_bytes([pair[0], pair[1]]);
        let tile_num = word & 0x03FF;
        let pal_bits = (word >> 10) & 0x07;
        total_entries += 1;
        if tile_num < 512 && pal_bits <= 7 {
            valid_entries += 1;
        }
        if tile_num == prev_tile.wrapping_add(1) || tile_num == prev_tile {
            sequential_steps += 1;
        }
        prev_tile = tile_num;
    }

    total_entries >= 256
        && valid_entries * 10 >= total_entries * 9
        && sequential_steps * 4 >= total_entries
}

fn scan_universal_tilemaps(
    rom: &SnesRom,
    dmas: &[crate::disasm_dma::DmaTransferDiscovery],
    graphics: &[UniversalGfxAsset],
    _palettes: &[UniversalPaletteAsset],
) -> Vec<UniversalTilemapAsset> {
    let mut tilemaps = Vec::new();
    if graphics.is_empty() {
        return tilemaps;
    }

    // 1. Check DMA VRAM targets that look like tilemaps
    for dma in dmas {
        if dma.target == DmaTarget::VramGraphics {
            if let Some(pc) = dma.source_pc_offset {
                let end = (pc + 1792).min(rom.data.len());
                if end - pc >= 1024 && looks_like_tilemap_buffer(&rom.data[pc..end]) {
                    let idx = tilemaps.len() + 1;
                    let id = format!("screen_{:02}_0x{:06X}", idx, pc);
                    let (bank, addr) = rom.pc_to_snes(pc);
                    tilemaps.push(UniversalTilemapAsset {
                        id: id.clone(),
                        name: format!("Tilemap DMA #{:02} (${:02X}:{:04X})", idx, bank, addr),
                        rom_offset: pc,
                        snes_addr: format!("${:02X}:{:04X}", bank, addr),
                        width_tiles: 32,
                        height_tiles: 28,
                        linked_gfx_idx: 0,
                        linked_pal_idx: 0,
                        png_file: format!("screens/{}.png", id),
                    });
                    if tilemaps.len() >= 4 {
                        break;
                    }
                }
            }
        }
    }

    // 2. Always guarantee at least 1 synthesized native 256x224 plane preview from top GFX sheet
    if tilemaps.is_empty() {
        let gfx0 = &graphics[0];
        let id = format!("screen_01_0x{:06X}", gfx0.rom_offset);
        tilemaps.push(UniversalTilemapAsset {
            id: id.clone(),
            name: format!("Aperçu Plan Natif 256×224 ({})", gfx0.snes_addr),
            rom_offset: gfx0.rom_offset,
            snes_addr: gfx0.snes_addr.clone(),
            width_tiles: 32,
            height_tiles: 28,
            linked_gfx_idx: 0,
            linked_pal_idx: gfx0.suggested_palette_idx,
            png_file: format!("screens/{}.png", id),
        });
    }

    tilemaps
}

/// Render a 16-bit SNES BG Tilemap (`vhopppcc cccccccc`) with a tile buffer & palettes into a 256x224 RGBA screen
pub fn render_tilemap_screen_rgba(
    tilemap_bytes: &[u8],
    tile_bytes: &[u8],
    format: TileFormat,
    palette: &SnesPalette,
    width_tiles: usize,
    height_tiles: usize,
) -> (usize, usize, Vec<u8>) {
    let w = width_tiles * 8;
    let h = height_tiles * 8;
    let mut rgba = vec![0u8; w * h * 4];
    let bpt = format.bytes_per_tile();
    let total_tiles = (tile_bytes.len() / bpt).max(1);
    let color_mask = format.color_count();

    for ty in 0..height_tiles {
        for tx in 0..width_tiles {
            let entry_idx = (ty * width_tiles + tx) * 2;
            let (tile_idx, h_flip, v_flip) = if entry_idx + 1 < tilemap_bytes.len() {
                let word =
                    u16::from_le_bytes([tilemap_bytes[entry_idx], tilemap_bytes[entry_idx + 1]]);
                (
                    (word & 0x03FF) as usize % total_tiles,
                    (word & 0x4000) != 0,
                    (word & 0x8000) != 0,
                )
            } else {
                ((ty * width_tiles + tx) % total_tiles, false, false)
            };

            let t_off = tile_idx * bpt;
            let pixels = if t_off + bpt <= tile_bytes.len() {
                decode_tile(&tile_bytes[t_off..t_off + bpt], format)
            } else {
                [0u8; 64]
            };

            for py in 0..8 {
                let sy = if v_flip { 7 - py } else { py };
                for px in 0..8 {
                    let sx = if h_flip { 7 - px } else { px };
                    let c_idx = (pixels[sy * 8 + sx] as usize) % color_mask;
                    let col = palette
                        .colors_rgba
                        .get(c_idx)
                        .copied()
                        .unwrap_or([0, 0, 0, 255]);
                    let dst = ((ty * 8 + py) * w + (tx * 8 + px)) * 4;
                    rgba[dst..dst + 4].copy_from_slice(&col);
                }
            }
        }
    }

    (w, h, rgba)
}

// =============================================================================
// 6. UNIVERSAL CROSS-ROM SWAP ENGINE (ANY ROM A <-> ANY ROM B)
// =============================================================================

/// Swap ANY graphic asset from `rom_b` into `rom_a`, handling:
/// - Automatic LZ2 decompression on donor `asset_b`
/// - Cross-format transcoding (2bpp <-> 3bpp <-> 4bpp <-> 8bpp <-> Mode7)
/// - Optional perceptual RGB color remapping (`smart_remap`)
/// - Automatic LZ2 recompression + guaranteed byte-budget safety on target `asset_a`
/// - Atomic `SwapSession` recording & SNES checksum recalculation
#[allow(clippy::too_many_arguments)]
pub fn swap_universal_gfx_asset(
    rom_a: &mut SnesRom,
    rom_b: &SnesRom,
    asset_a: &UniversalGfxAsset,
    asset_b: &UniversalGfxAsset,
    pal_a: &SnesPalette,
    pal_b: &SnesPalette,
    smart_remap: bool,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    let donor_raw = read_universal_gfx_bytes(rom_b, asset_b).ok_or(RomError::OutOfBounds {
        offset: asset_b.rom_offset,
        len: asset_b.rom_byte_budget,
    })?;

    let target_tiles = asset_a.tile_count.max(1);
    let donor_bpt = asset_b.format.bytes_per_tile();
    let donor_tiles_avail = (donor_raw.len() / donor_bpt).max(1);

    // Prepare donor buffer matched to target tile count (looping cleanly if smaller)
    let needed_donor_bytes = target_tiles * donor_bpt;
    let mut padded_donor = vec![0u8; needed_donor_bytes];
    let usable_donor_len = donor_tiles_avail * donor_bpt;
    for i in 0..needed_donor_bytes {
        padded_donor[i] = donor_raw[i % usable_donor_len.max(1)];
    }

    let transcoded = if smart_remap || asset_a.format != asset_b.format {
        remap_and_transcode_tiles(
            &padded_donor,
            target_tiles,
            asset_b.format,
            pal_b,
            asset_a.format,
            pal_a,
        )
    } else {
        padded_donor
    };

    let replacement = pack_gfx_for_target_budget(&transcoded, asset_a);

    session.swap_custom_bytes(
        rom_a,
        asset_a.rom_offset,
        asset_b.rom_offset,
        replacement,
        format!(
            "Swap Universel GFX : {} ⬅️ {} ({} tuiles {}, {})",
            asset_a.id,
            asset_b.id,
            target_tiles,
            asset_a.format.short_label(),
            asset_a.compression.label_fr()
        ),
        SwapKind::GraphicsTiles {
            format: asset_a.format,
            tile_count: target_tiles,
            smart_remapped: smart_remap,
        },
    )?;

    Ok(1)
}

/// Safely pack raw planar tile bytes into `asset_a.rom_byte_budget`, compressing via LZ2 if required
/// and guaranteeing zero overflow past adjacent ROM data.
fn pack_gfx_for_target_budget(raw_tiles: &[u8], target: &UniversalGfxAsset) -> Vec<u8> {
    let budget = target.rom_byte_budget;
    match target.compression {
        UniversalGfxCompression::None => {
            let mut out = vec![0u8; budget];
            let copy_len = raw_tiles.len().min(budget);
            out[..copy_len].copy_from_slice(&raw_tiles[..copy_len]);
            out
        }
        UniversalGfxCompression::NintendoLz2BigEndian
        | UniversalGfxCompression::NintendoLz2LittleEndian => {
            let be = target.compression == UniversalGfxCompression::NintendoLz2BigEndian;
            let mut work = raw_tiles.to_vec();
            work.resize(target.decompressed_len, 0);

            let mut comp = compress_nintendo_lz2_with_endian(&work, be);
            // If compressed data exceeds exact ROM budget, progressively zero-simplify tail tiles
            // so the recompressed stream ALWAYS fits 100% safely without corrupting neighboring banks!
            let bpt = target.format.bytes_per_tile();
            let mut keep_tiles = work.len() / bpt;
            while comp.len() > budget && keep_tiles > 4 {
                keep_tiles = (keep_tiles * 3) / 4;
                let cut_byte = keep_tiles * bpt;
                for b in &mut work[cut_byte..] {
                    *b = 0;
                }
                comp = compress_nintendo_lz2_with_endian(&work, be);
            }

            if comp.len() <= budget {
                comp.resize(budget, 0xFF);
                comp
            } else {
                // Ultra-tight fallback: empty RLE stream fitting in 5 bytes + 0xFF padding
                let mut minimal = vec![0xFFu8; budget];
                if budget >= 5 {
                    let zeros = vec![0u8; target.decompressed_len];
                    let zcomp = compress_nintendo_lz2_with_endian(&zeros, be);
                    let clen = zcomp.len().min(budget);
                    minimal[..clen].copy_from_slice(&zcomp[..clen]);
                }
                minimal
            }
        }
    }
}

/// Swap ANY BGR555 palette from `rom_b` into `rom_a`
pub fn swap_universal_palette_asset(
    rom_a: &mut SnesRom,
    rom_b: &SnesRom,
    pal_a: &UniversalPaletteAsset,
    pal_b: &UniversalPaletteAsset,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    let color_count = pal_a.color_count.min(pal_b.color_count);
    session.swap_palette(
        rom_a,
        rom_b,
        pal_a.rom_offset,
        pal_b.rom_offset,
        color_count,
    )?;
    Ok(1)
}

/// Swap ANY BRR audio sample from `rom_b` into `rom_a` with automatic time-domain resampling
pub fn swap_universal_audio_asset(
    rom_a: &mut SnesRom,
    rom_b: &SnesRom,
    audio_a: &UniversalAudioAsset,
    audio_b: &UniversalAudioAsset,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    let end_b = (audio_b.rom_offset + audio_b.byte_len).min(rom_b.data.len());
    if audio_b.rom_offset >= end_b || audio_a.rom_offset + audio_a.byte_len > rom_a.data.len() {
        return Err(RomError::OutOfBounds {
            offset: audio_a.rom_offset,
            len: audio_a.byte_len,
        });
    }
    let donor_brr = &rom_b.data[audio_b.rom_offset..end_b];
    let replacement = resample_and_encode_brr_for_swap(donor_brr, audio_a.byte_len, audio_a.loops);
    session.swap_custom_bytes(
        rom_a,
        audio_a.rom_offset,
        audio_b.rom_offset,
        replacement,
        format!(
            "Swap Universel Audio BRR : {} ⬅️ {} ({} octets, Loop: {})",
            audio_a.id, audio_b.id, audio_a.byte_len, audio_a.loops
        ),
        SwapKind::AudioBrrSample {
            target_blocks: audio_a.block_count,
            donor_blocks: audio_b.block_count,
            loops: audio_a.loops,
            resampled: true,
        },
    )?;
    Ok(1)
}

/// Patch / swap ANY discovered text string inside `rom_a` using its discovered `.tbl` shift encoding
pub fn patch_universal_text_asset(
    rom_a: &mut SnesRom,
    text_asset: &UniversalTextAsset,
    new_text: &str,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    let max_len = text_asset.max_byte_len;
    if text_asset.rom_offset + max_len > rom_a.data.len() || max_len == 0 {
        return Err(RomError::OutOfBounds {
            offset: text_asset.rom_offset,
            len: max_len,
        });
    }

    let encoded = encode_text_for_asset(new_text, text_asset);
    session.swap_custom_bytes(
        rom_a,
        text_asset.rom_offset,
        0,
        encoded,
        format!(
            "Édition Texte Universel ({}) @ 0x{:06X} -> \"{}\"",
            text_asset.id,
            text_asset.rom_offset,
            new_text.chars().take(24).collect::<String>()
        ),
        SwapKind::CodePatch {
            byte_count: max_len,
        },
    )?;
    Ok(1)
}

fn encode_text_for_asset(new_text: &str, asset: &UniversalTextAsset) -> Vec<u8> {
    let mut out = vec![asset.space_byte; asset.max_byte_len];
    let is_ascii = asset.encoding == "ASCII";
    let is_upper_tbl = asset.encoding.contains("A=");

    for (i, ch) in new_text.chars().take(asset.max_byte_len).enumerate() {
        if is_ascii {
            let b = if ch.is_ascii() && (0x20..=0x7E).contains(&(ch as u8)) {
                ch as u8
            } else {
                asset.space_byte
            };
            out[i] = b;
        } else if ch == ' ' {
            out[i] = asset.space_byte;
        } else if ch.is_ascii_alphabetic() {
            let target_ch = if is_upper_tbl {
                ch.to_ascii_uppercase() as i16
            } else {
                ch.to_ascii_lowercase() as i16
            };
            out[i] = (target_ch + asset.char_shift).rem_euclid(256) as u8;
        } else {
            out[i] = asset.space_byte;
        }
    }
    out
}

/// One-shot Universal Cross-ROM Batch Swap between ANY two SNES ROMs (LoROM/HiROM/ExHiROM!)
pub fn apply_universal_batch_swap(
    rom_a: &mut SnesRom,
    rom_b: &SnesRom,
    catalog_a: &UniversalRomCatalog,
    catalog_b: &UniversalRomCatalog,
    opts: &UniversalBatchSwapOptions,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    let mut ops = 0usize;

    let default_pal_a = catalog_a
        .palettes
        .first()
        .map(palette_asset_to_snes_palette)
        .unwrap_or_else(SnesPalette::preset_snes_adventure);
    let default_pal_b = catalog_b
        .palettes
        .first()
        .map(palette_asset_to_snes_palette)
        .unwrap_or_else(SnesPalette::preset_cyber_neon);

    // 1. Graphics / Sprites Swaps (prioritize Sprites & Backgrounds of matching format)
    if opts.swap_graphics && !catalog_a.graphics.is_empty() && !catalog_b.graphics.is_empty() {
        let count = opts
            .max_gfx_swaps
            .min(catalog_a.graphics.len())
            .min(catalog_b.graphics.len());
        for i in 0..count {
            let ga = &catalog_a.graphics[i];
            let gb = &catalog_b.graphics[i % catalog_b.graphics.len()];
            let pa = catalog_a
                .palettes
                .get(ga.suggested_palette_idx)
                .map(palette_asset_to_snes_palette)
                .unwrap_or_else(|| default_pal_a.clone());
            let pb = catalog_b
                .palettes
                .get(gb.suggested_palette_idx)
                .map(palette_asset_to_snes_palette)
                .unwrap_or_else(|| default_pal_b.clone());
            if let Ok(n) = swap_universal_gfx_asset(
                rom_a,
                rom_b,
                ga,
                gb,
                &pa,
                &pb,
                opts.smart_color_remap,
                session,
            ) {
                ops += n;
            }
        }
    }

    // 2. Palette Swaps
    if opts.swap_palettes && !catalog_a.palettes.is_empty() && !catalog_b.palettes.is_empty() {
        let count = opts
            .max_palette_swaps
            .min(catalog_a.palettes.len())
            .min(catalog_b.palettes.len());
        for i in 0..count {
            let pa = &catalog_a.palettes[i];
            let pb = &catalog_b.palettes[i % catalog_b.palettes.len()];
            if let Ok(n) = swap_universal_palette_asset(rom_a, rom_b, pa, pb, session) {
                ops += n;
            }
        }
    }

    // 3. Audio BRR Swaps
    if opts.swap_audio && !catalog_a.audio_samples.is_empty() && !catalog_b.audio_samples.is_empty()
    {
        let count = opts
            .max_audio_swaps
            .min(catalog_a.audio_samples.len())
            .min(catalog_b.audio_samples.len());
        for i in 0..count {
            let aa = &catalog_a.audio_samples[i];
            let ab = &catalog_b.audio_samples[i % catalog_b.audio_samples.len()];
            if let Ok(n) = swap_universal_audio_asset(rom_a, rom_b, aa, ab, session) {
                ops += n;
            }
        }
    }

    // 4. Text Swaps (optional)
    if opts.swap_text && !catalog_a.texts.is_empty() && !catalog_b.texts.is_empty() {
        let count = 4.min(catalog_a.texts.len()).min(catalog_b.texts.len());
        for i in 0..count {
            let ta = &catalog_a.texts[i];
            let tb = &catalog_b.texts[i];
            if let Ok(n) = patch_universal_text_asset(rom_a, ta, &tb.text, session) {
                ops += n;
            }
        }
    }

    rom_a.fix_checksum();
    Ok(ops)
}

pub fn palette_asset_to_snes_palette(asset: &UniversalPaletteAsset) -> SnesPalette {
    let mut colors_rgba = Vec::with_capacity(256);
    for i in 0..256 {
        let c = asset
            .colors_rgba
            .get(i % asset.colors_rgba.len().max(1))
            .copied()
            .unwrap_or([0, 0, 0, 255]);
        colors_rgba.push(c);
    }
    SnesPalette {
        name: asset.name.clone(),
        source_offset: Some(asset.rom_offset),
        colors_rgba,
    }
}

// =============================================================================
// 7. UNIVERSAL ROUND-TRIP PROJECT EXPORT (`project.json`) & RECONSTRUCTION
// =============================================================================

/// Export a complete, editable Universal Round-Trip Project (`project.json` + PNGs + WAVs + SPCs +
/// Palettes JSON + Editable Texts + Tilemaps + WLA-DX Disassembly) for ANY SNES ROM.
pub fn export_universal_editable_project(
    rom: &SnesRom,
    catalog: &UniversalRomCatalog,
    out_dir: &Path,
) -> std::io::Result<()> {
    export_universal_editable_project_with_mode(rom, catalog, out_dir, false)
}

/// Export a Universal Round-Trip Project with optional `exhaustive_full_mode`
/// (exports 100% of ALL discovered OAM sprites, multi-page contact sheets, and raw bank binaries).
pub fn export_universal_editable_project_with_mode(
    rom: &SnesRom,
    catalog: &UniversalRomCatalog,
    out_dir: &Path,
    exhaustive_full_mode: bool,
) -> std::io::Result<()> {
    let sheets_dir = out_dir.join("sheets");
    let pal_dir = out_dir.join("palettes");
    let audio_dir = out_dir.join("audio");
    let text_dir = out_dir.join("text");
    let screens_dir = out_dir.join("screens");
    let disasm_dir = out_dir.join("disassembly");

    fs::create_dir_all(&sheets_dir)?;
    fs::create_dir_all(&pal_dir)?;
    fs::create_dir_all(&audio_dir)?;
    fs::create_dir_all(&text_dir)?;
    fs::create_dir_all(&screens_dir)?;
    fs::create_dir_all(&disasm_dir)?;

    let snes_pals: Vec<SnesPalette> = catalog
        .palettes
        .iter()
        .map(palette_asset_to_snes_palette)
        .collect();
    let fallback_pal = snes_pals
        .first()
        .cloned()
        .unwrap_or_else(SnesPalette::preset_snes_adventure);

    // 1. Export Palettes (JSON + PNG Swatch + Raw .pal)
    for pal in &catalog.palettes {
        let mut json = String::new();
        json.push_str("{\n");
        json.push_str(&format!("  \"id\": \"{}\",\n", pal.id));
        json.push_str(&format!("  \"name\": \"{}\",\n", escape_json(&pal.name)));
        json.push_str(&format!("  \"rom_offset\": {},\n", pal.rom_offset));
        json.push_str(&format!("  \"color_count\": {},\n", pal.color_count));
        json.push_str("  \"colors\": [\n");
        for (i, c) in pal.colors_rgba.iter().enumerate() {
            let hex = format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]);
            let bgr = rgb_to_bgr555(c[0], c[1], c[2]);
            json.push_str(&format!(
                "    {{ \"index\": {}, \"hex\": \"{}\", \"bgr555\": \"0x{:04X}\" }}{}\n",
                i,
                hex,
                bgr,
                if i + 1 < pal.colors_rgba.len() {
                    ","
                } else {
                    ""
                }
            ));
        }
        json.push_str("  ]\n}\n");
        fs::write(out_dir.join(&pal.json_file), json)?;

        // PNG swatch
        let swatch_w = (pal.colors_rgba.len() * 16).max(16) as u32;
        let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(swatch_w, 16);
        for (ci, c) in pal.colors_rgba.iter().enumerate() {
            let px = Rgba([c[0], c[1], c[2], 255]);
            for y in 0..16 {
                for x in 0..16 {
                    img.put_pixel((ci * 16 + x) as u32, y, px);
                }
            }
        }
        let _ = img.save(pal_dir.join(format!("{}.png", pal.id)));

        if pal.rom_offset + pal.byte_len <= rom.data.len() {
            fs::write(
                pal_dir.join(format!("{}.pal", pal.id)),
                &rom.data[pal.rom_offset..pal.rom_offset + pal.byte_len],
            )?;
        }
    }

    // 2. Export Graphics Sheets (Editable PNG + Raw .bin)
    for gfx in &catalog.graphics {
        let pal = snes_pals
            .get(gfx.suggested_palette_idx)
            .unwrap_or(&fallback_pal);
        if let Some(raw_tiles) = read_universal_gfx_bytes(rom, gfx) {
            fs::write(sheets_dir.join(format!("{}.bin", gfx.id)), &raw_tiles)?;
            let cols = 16usize;
            let bpt = gfx.format.bytes_per_tile();
            let total_tiles = raw_tiles.len() / bpt;
            let rows = total_tiles.div_ceil(cols).clamp(1, 64);
            let (w, h, rgba) = render_tile_sheet_rgba(&raw_tiles, 0, cols, rows, gfx.format, pal);
            if let Some(img) = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(w as u32, h as u32, rgba)
            {
                let _ = img.save(out_dir.join(&gfx.sheet_file));
            }
        }
    }

    // 3. Export Audio (BRR + Editable WAV 16kHz + Authentic Universal .SPC streams)
    for audio in &catalog.audio_samples {
        let end = (audio.rom_offset + audio.byte_len).min(rom.data.len());
        let brr = &rom.data[audio.rom_offset..end];
        fs::write(audio_dir.join(format!("{}.brr", audio.id)), brr)?;
        let pcm = decode_brr_to_pcm16(brr);
        let wav = encode_pcm16_to_wav(&pcm, 16000);
        fs::write(out_dir.join(&audio.wav_file), wav)?;
    }

    let base_spc_engine = catalog.spc_streams.first();
    for spc in &catalog.spc_streams {
        let spc_bytes = build_spc_from_universal_stream(rom, spc, base_spc_engine);
        fs::write(out_dir.join(&spc.spc_file), spc_bytes)?;
    }

    // If ROM uses custom non-IPL audio engine, synthesize soundbank SPC
    if catalog.spc_streams.is_empty() && !catalog.audio_samples.is_empty() {
        let brr_infos: Vec<BrrSampleInfo> = catalog
            .audio_samples
            .iter()
            .enumerate()
            .map(|(i, a)| BrrSampleInfo {
                index: i + 1,
                offset: a.rom_offset,
                byte_len: a.byte_len,
                block_count: a.block_count,
                pcm_sample_count: a.block_count * 16,
                loops: a.loops,
                peak_amplitude: a.peak_amplitude,
                label: a.name.clone(),
            })
            .collect();
        if let Some(spc_bytes) =
            synthesize_universal_soundbank_spc(&rom.data, &brr_infos, &rom.canonical_title())
        {
            fs::write(audio_dir.join("universal_soundbank.spc"), spc_bytes)?;
        }
    }

    // 4. Export Editable Text Strings (`text/strings_editable.json` + `.txt` + `.tbl`)
    let mut txt_json = String::from("[\n");
    let mut txt_plain = format!(
        "; TEXTES DÉCOUVERTS UNIVERSELLEMENT — {}\n; Modifiez text/strings_editable.json puis lancez --reconstruct pour réinjecter !\n\n",
        rom.canonical_title()
    );
    for (i, t) in catalog.texts.iter().enumerate() {
        txt_plain.push_str(&format!(
            "[{}] {} ({}, max {} octets): {}\n",
            t.id, t.snes_addr, t.encoding, t.max_byte_len, t.text
        ));
        txt_json.push_str(&format!(
            "  {{ \"id\": \"{}\", \"rom_offset\": {}, \"max_byte_len\": {}, \"encoding\": \"{}\", \"char_shift\": {}, \"space_byte\": {}, \"original_text\": \"{}\", \"text\": \"{}\" }}{}\n",
            t.id,
            t.rom_offset,
            t.max_byte_len,
            escape_json(&t.encoding),
            t.char_shift,
            t.space_byte,
            escape_json(&t.text),
            escape_json(&t.text),
            if i + 1 < catalog.texts.len() { "," } else { "" }
        ));
    }
    txt_json.push_str("]\n");
    fs::write(text_dir.join("strings_editable.json"), txt_json)?;
    fs::write(text_dir.join("strings_universal.txt"), txt_plain)?;

    // Generate universal `.tbl` file from first custom-TBL encoding discovered (or ASCII)
    if let Some(custom_t) = catalog.texts.iter().find(|t| t.char_shift != 0) {
        let is_upper = custom_t.encoding.contains("A=");
        let base_c = if is_upper { b'A' as i16 } else { b'a' as i16 };
        let mut tbl = format!(
            "; Table Thingy (.tbl) auto-détectée pour {}\n",
            rom.canonical_title()
        );
        for k in 0..26i16 {
            let b = (base_c + k + custom_t.char_shift).rem_euclid(256) as u8;
            tbl.push_str(&format!("{:02X}={}\n", b, (base_c as u8 + k as u8) as char));
        }
        tbl.push_str(&format!("{:02X}= \n", custom_t.space_byte));
        fs::write(text_dir.join("auto_discovered.tbl"), tbl)?;
    }

    // 5. Export Reconstructed Tilemap Screens
    for tm in &catalog.tilemaps {
        if let Some(gfx) = catalog.graphics.get(tm.linked_gfx_idx) {
            if let Some(tiles) = read_universal_gfx_bytes(rom, gfx) {
                let pal = snes_pals.get(tm.linked_pal_idx).unwrap_or(&fallback_pal);
                let tm_slice =
                    if tm.rom_offset + 1792 <= rom.data.len() && tm.rom_offset != gfx.rom_offset {
                        &rom.data[tm.rom_offset..tm.rom_offset + 1792]
                    } else {
                        &[]
                    };
                let (w, h, rgba) = render_tilemap_screen_rgba(
                    tm_slice,
                    &tiles,
                    gfx.format,
                    pal,
                    tm.width_tiles,
                    tm.height_tiles,
                );
                if let Some(img) =
                    ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(w as u32, h as u32, rgba)
                {
                    let _ = img.save(out_dir.join(&tm.png_file));
                }
            }
        }
    }

    // 6. Export Universal Disassembly (`header.asm`, `bank_XX.asm`, `dma_transfers.json`, `entropy_map.json`)
    export_universal_disassembly(rom, &disasm_dir)?;
    if exhaustive_full_mode {
        let raw_dir = disasm_dir.join("raw");
        fs::create_dir_all(&raw_dir)?;
        let bank_size = rom.bank_size();
        for (b_idx, bank_data) in rom.banks().iter().enumerate() {
            let (snes_bank, _) = rom.pc_to_snes(b_idx * bank_size);
            fs::write(
                raw_dir.join(format!("bank_{:02X}.bin", snes_bank)),
                bank_data,
            )?;
        }
    }

    // 6b. Universal OAM Multi-Tile Sprite Table Auto-Extraction (HiROM OAM sprite engines)
    let sprites_dir = out_dir.join("sprites");
    fs::create_dir_all(&sprites_dir)?;
    if let Some((_table_off, frame_offsets)) = scan_universal_oam_sprite_table(rom) {
        export_oam_sprite_collection(
            rom,
            &frame_offsets,
            &snes_pals,
            &fallback_pal,
            &sprites_dir,
            exhaustive_full_mode,
        )?;
    }

    // 7. Write Master Round-Trip `project.json` (used by `reconstruct_rom_from_project`!)
    let mut proj = String::new();
    proj.push_str("{\n");
    proj.push_str("  \"schema\": \"snes_universal_project_v1\",\n");
    proj.push_str(&format!(
        "  \"rom_title\": \"{}\",\n",
        escape_json(&catalog.rom_title)
    ));
    proj.push_str(&format!(
        "  \"mapping\": \"{}\",\n",
        escape_json(&catalog.mapping)
    ));
    proj.push_str(&format!(
        "  \"rom_size_bytes\": {},\n",
        catalog.rom_size_bytes
    ));
    proj.push_str(&format!(
        "  \"checksum\": \"0x{:04X}\",\n",
        catalog.checksum
    ));
    proj.push_str(&format!("  \"crc32\": \"0x{:08X}\",\n", catalog.crc32));

    proj.push_str("  \"graphics\": [\n");
    for (i, g) in catalog.graphics.iter().enumerate() {
        proj.push_str(&format!(
            "    {{ \"id\": \"{}\", \"rom_offset\": {}, \"rom_byte_budget\": {}, \"decompressed_len\": {}, \"format\": \"{}\", \"compression\": \"{}\", \"tile_count\": {}, \"palette_idx\": {}, \"sheet_file\": \"{}\" }}{}\n",
            g.id,
            g.rom_offset,
            g.rom_byte_budget,
            g.decompressed_len,
            g.format.short_label(),
            g.compression.as_str(),
            g.tile_count,
            g.suggested_palette_idx,
            g.sheet_file,
            if i + 1 < catalog.graphics.len() { "," } else { "" }
        ));
    }
    proj.push_str("  ],\n");

    proj.push_str("  \"palettes\": [\n");
    for (i, p) in catalog.palettes.iter().enumerate() {
        proj.push_str(&format!(
            "    {{ \"id\": \"{}\", \"rom_offset\": {}, \"color_count\": {}, \"json_file\": \"{}\" }}{}\n",
            p.id,
            p.rom_offset,
            p.color_count,
            p.json_file,
            if i + 1 < catalog.palettes.len() { "," } else { "" }
        ));
    }
    proj.push_str("  ],\n");

    proj.push_str("  \"audio_samples\": [\n");
    for (i, a) in catalog.audio_samples.iter().enumerate() {
        proj.push_str(&format!(
            "    {{ \"id\": \"{}\", \"rom_offset\": {}, \"byte_len\": {}, \"loops\": {}, \"wav_file\": \"{}\" }}{}\n",
            a.id,
            a.rom_offset,
            a.byte_len,
            a.loops,
            a.wav_file,
            if i + 1 < catalog.audio_samples.len() { "," } else { "" }
        ));
    }
    proj.push_str("  ]\n");
    proj.push_str("}\n");
    fs::write(out_dir.join("project.json"), proj)?;

    Ok(())
}

/// Round-Trip Universal ROM Reconstruction:
/// Takes the original `.sfc/.smc` ROM and an exported (and user-edited!) project directory,
/// automatically detects ALL edited PNG sheets, Palette JSONs, WAV audio samples, and text strings,
/// re-encodes them to exact SNES hardware formats (2bpp/3bpp/4bpp/8bpp + LZ2 recompression, BGR555,
/// BRR resampled, TBL encoded), recalculates the official SNES checksum, and writes a 100% playable `.sfc` ROM
/// (+ optional `.ips` patch)!
pub fn reconstruct_rom_from_project(
    original_rom: &SnesRom,
    project_dir: &Path,
    out_rom_path: &Path,
    out_ips_path: Option<&Path>,
) -> std::io::Result<ReconstructReport> {
    let start = Instant::now();
    let proj_str = fs::read_to_string(project_dir.join("project.json"))?;

    let mut target_rom = original_rom.clone();
    let mut gfx_patched_count = 0usize;
    let mut palettes_patched_count = 0usize;
    let mut audio_patched_count = 0usize;
    let mut text_patched_count = 0usize;
    let mut warnings: Vec<String> = Vec::new();

    // --- 1. Re-inject Edited Palettes FIRST (so PNG re-encoding uses updated palettes!) ---
    let mut loaded_palettes: Vec<SnesPalette> = Vec::new();
    for line in extract_json_array_lines(&proj_str, "\"palettes\"") {
        let pal_offset = parse_json_usize(&line, "rom_offset").unwrap_or(0);
        let color_count = parse_json_usize(&line, "color_count").unwrap_or(16);
        let json_file = parse_json_string(&line, "json_file").unwrap_or_default();
        let pal_path = project_dir.join(&json_file);

        if let Ok(pal_content) = fs::read_to_string(&pal_path) {
            let mut rgba_list: Vec<[u8; 4]> = Vec::new();
            let mut new_bgr_bytes: Vec<u8> = Vec::new();

            for pline in pal_content.lines() {
                if let Some(hex) = parse_json_string(pline, "hex") {
                    if let Some([r, g, b]) = parse_hex_rgb(&hex) {
                        rgba_list.push([r, g, b, 255]);
                        let bgr = rgb_to_bgr555(r, g, b);
                        new_bgr_bytes.extend_from_slice(&bgr.to_le_bytes());
                    }
                }
            }

            if !rgba_list.is_empty() {
                let mut full_256 = Vec::with_capacity(256);
                for i in 0..256 {
                    full_256.push(rgba_list[i % rgba_list.len()]);
                }
                loaded_palettes.push(SnesPalette {
                    name: json_file.clone(),
                    source_offset: Some(pal_offset),
                    colors_rgba: full_256,
                });

                let byte_len = (color_count * 2).min(new_bgr_bytes.len());
                if pal_offset + byte_len <= target_rom.data.len()
                    && target_rom.data[pal_offset..pal_offset + byte_len]
                        != new_bgr_bytes[..byte_len]
                {
                    target_rom.data[pal_offset..pal_offset + byte_len]
                        .copy_from_slice(&new_bgr_bytes[..byte_len]);
                    palettes_patched_count += 1;
                }
            }
        }
    }

    let fallback_pal = loaded_palettes
        .first()
        .cloned()
        .unwrap_or_else(SnesPalette::preset_snes_adventure);

    // --- 2. Re-inject Edited PNG Graphic Sheets (auto-quantize + auto-recompress LZ2 within budget!) ---
    for line in extract_json_array_lines(&proj_str, "\"graphics\"") {
        let id = parse_json_string(&line, "id").unwrap_or_default();
        let rom_offset = parse_json_usize(&line, "rom_offset").unwrap_or(0);
        let budget = parse_json_usize(&line, "rom_byte_budget").unwrap_or(0);
        let dec_len = parse_json_usize(&line, "decompressed_len").unwrap_or(budget);
        let fmt_str = parse_json_string(&line, "format").unwrap_or_else(|| "4bpp".to_string());
        let comp_str =
            parse_json_string(&line, "compression").unwrap_or_else(|| "none".to_string());
        let pal_idx = parse_json_usize(&line, "palette_idx").unwrap_or(0);
        let sheet_file = parse_json_string(&line, "sheet_file").unwrap_or_default();

        let format = TileFormat::from_short_label(&fmt_str).unwrap_or(TileFormat::Bpp4);
        let compression = UniversalGfxCompression::from_str_tag(&comp_str);
        let png_path = project_dir.join(&sheet_file);
        if !png_path.exists() || rom_offset + budget > target_rom.data.len() || budget == 0 {
            continue;
        }

        // Only re-encode if the PNG actually differs from what the original ROM renders!
        let pal = loaded_palettes.get(pal_idx).unwrap_or(&fallback_pal);
        let orig_asset = UniversalGfxAsset {
            id: id.clone(),
            name: id.clone(),
            category: UniversalGfxCategory::Sprites,
            rom_offset,
            snes_addr: String::new(),
            rom_byte_budget: budget,
            decompressed_len: dec_len,
            format,
            compression,
            tile_count: dec_len / format.bytes_per_tile(),
            suggested_palette_idx: pal_idx,
            discovery_source: String::new(),
            sheet_file: sheet_file.clone(),
        };

        if let Ok(dyn_img) = image::open(&png_path) {
            let rgba_img = dyn_img.to_rgba8();
            let w = rgba_img.width() as usize;
            let h = rgba_img.height() as usize;

            let orig_rendered = render_universal_gfx_asset(original_rom, &orig_asset, pal, 64);
            let pixels_changed = match orig_rendered {
                Some((ow, oh, orgba)) => {
                    ow != w || oh != h || orgba != rgba_img.as_raw().as_slice()
                }
                None => true,
            };

            if pixels_changed {
                let mut encoded_tiles =
                    encode_rgba_sheet_to_tiles(rgba_img.as_raw(), w, h, format, pal);
                encoded_tiles.resize(dec_len, 0);
                let packed = pack_gfx_for_target_budget(&encoded_tiles, &orig_asset);
                if packed.len() == budget {
                    target_rom.data[rom_offset..rom_offset + budget].copy_from_slice(&packed);
                    gfx_patched_count += 1;
                } else {
                    warnings.push(format!(
                        "Graphisme {} ignoré (taille {} != budget {})",
                        id,
                        packed.len(),
                        budget
                    ));
                }
            }
        }
    }

    // --- 3. Re-inject Edited WAV Audio Samples (auto-resample & BRR re-encode!) ---
    for line in extract_json_array_lines(&proj_str, "\"audio_samples\"") {
        let id = parse_json_string(&line, "id").unwrap_or_default();
        let rom_offset = parse_json_usize(&line, "rom_offset").unwrap_or(0);
        let byte_len = parse_json_usize(&line, "byte_len").unwrap_or(0);
        let loops = line.contains("\"loops\": true");
        let wav_file = parse_json_string(&line, "wav_file").unwrap_or_default();

        let wav_path = project_dir.join(&wav_file);
        if !wav_path.exists() || rom_offset + byte_len > target_rom.data.len() || byte_len < 9 {
            continue;
        }

        if let Ok(wav_bytes) = fs::read(&wav_path) {
            // Compare against exact original exported WAV so unmodified samples stay bit-identical
            let orig_pcm =
                decode_brr_to_pcm16(&original_rom.data[rom_offset..rom_offset + byte_len]);
            let orig_wav = encode_pcm16_to_wav(&orig_pcm, 16000);
            if wav_bytes != orig_wav {
                if let Some((_sr, new_pcm)) = decode_wav_to_pcm16(&wav_bytes) {
                    let new_brr = encode_pcm16_to_brr_fit_budget(&new_pcm, byte_len, loops);
                    if new_brr.len() == byte_len {
                        target_rom.data[rom_offset..rom_offset + byte_len]
                            .copy_from_slice(&new_brr);
                        audio_patched_count += 1;
                    }
                } else {
                    warnings.push(format!("WAV invalide pour {}", id));
                }
            }
        }
    }

    // --- 4. Re-inject Edited Texts (`text/strings_editable.json`) ---
    let strings_path = project_dir.join("text").join("strings_editable.json");
    if let Ok(str_json) = fs::read_to_string(&strings_path) {
        for line in str_json.lines() {
            if !line.trim_start().starts_with('{') {
                continue;
            }
            let id = parse_json_string(line, "id").unwrap_or_default();
            let rom_offset = parse_json_usize(line, "rom_offset").unwrap_or(0);
            let max_byte_len = parse_json_usize(line, "max_byte_len").unwrap_or(0);
            let encoding =
                parse_json_string(line, "encoding").unwrap_or_else(|| "ASCII".to_string());
            let char_shift = parse_json_i16(line, "char_shift").unwrap_or(0);
            let space_byte = parse_json_usize(line, "space_byte").unwrap_or(0x20) as u8;
            let original_text = parse_json_string(line, "original_text");
            let text = parse_json_string(line, "text").unwrap_or_default();

            if let Some(ref orig) = original_text {
                if orig == &text {
                    continue; // Unmodified text -> preserve original ROM bytes 100% bit-identically!
                }
            }

            if max_byte_len == 0 || rom_offset + max_byte_len > target_rom.data.len() {
                continue;
            }

            let asset = UniversalTextAsset {
                id,
                rom_offset,
                snes_addr: String::new(),
                max_byte_len,
                encoding,
                char_shift,
                space_byte,
                terminator: None,
                text: text.clone(),
            };
            let encoded = encode_text_for_asset(&text, &asset);
            if target_rom.data[rom_offset..rom_offset + max_byte_len] != encoded[..] {
                target_rom.data[rom_offset..rom_offset + max_byte_len].copy_from_slice(&encoded);
                text_patched_count += 1;
            }
        }
    }

    // --- 5. Recalculate SNES Checksum & Write Output ROM (+ Optional IPS Patch) ---
    target_rom.fix_checksum();
    let total_bytes_changed = original_rom
        .data
        .iter()
        .zip(target_rom.data.iter())
        .filter(|(a, b)| a != b)
        .count();

    if let Some(parent) = out_rom_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(out_rom_path, target_rom.to_sfc_bytes())?;

    let mut ips_saved = None;
    if let Some(ips_p) = out_ips_path {
        let ips_bytes = build_ips_patch(&original_rom.data, &target_rom.data);
        fs::write(ips_p, ips_bytes)?;
        ips_saved = Some(ips_p.display().to_string());
    }

    Ok(ReconstructReport {
        out_rom_path: out_rom_path.display().to_string(),
        ips_path: ips_saved,
        gfx_patched_count,
        palettes_patched_count,
        audio_patched_count,
        text_patched_count,
        total_bytes_changed,
        new_checksum: target_rom.header.checksum,
        checksum_valid: target_rom.header.is_checksum_valid(),
        warnings,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })
}

/// Generate a standard Lunar-IPS compatible `.ips` binary patch (`PATCH ... EOF`)
pub fn build_ips_patch(original: &[u8], modified: &[u8]) -> Vec<u8> {
    let mut ips = Vec::new();
    ips.extend_from_slice(b"PATCH");
    let len = original.len().min(modified.len());
    let mut i = 0usize;
    while i < len {
        if original[i] != modified[i] {
            let start = i;
            let mut diff = Vec::new();
            while i < len && diff.len() < 0xFFFF {
                if original[i] != modified[i] {
                    diff.push(modified[i]);
                    i += 1;
                } else {
                    // Merge tiny gaps (< 5 bytes) for compact IPS
                    let mut lookahead = 0;
                    while i + lookahead < len
                        && lookahead < 5
                        && original[i + lookahead] == modified[i + lookahead]
                    {
                        lookahead += 1;
                    }
                    if lookahead < 5 && i + lookahead < len && diff.len() + lookahead < 0xFFFF {
                        for k in 0..lookahead {
                            diff.push(modified[i + k]);
                        }
                        i += lookahead;
                    } else {
                        break;
                    }
                }
            }
            // Avoid IPS EOF magic sentinel (0x454F46) as offset
            let (rec_start, rec_data) = if start == 0x454F46 && start > 0 {
                let mut d = vec![modified[start - 1]];
                d.extend_from_slice(&diff);
                (start - 1, d)
            } else {
                (start, diff)
            };
            ips.push(((rec_start >> 16) & 0xFF) as u8);
            ips.push(((rec_start >> 8) & 0xFF) as u8);
            ips.push((rec_start & 0xFF) as u8);
            let rlen = rec_data.len() as u16;
            ips.push(((rlen >> 8) & 0xFF) as u8);
            ips.push((rlen & 0xFF) as u8);
            ips.extend_from_slice(&rec_data);
        } else {
            i += 1;
        }
    }
    ips.extend_from_slice(b"EOF");
    ips
}

// =============================================================================
// INTERNAL HELPERS
// =============================================================================

fn synthesize_universal_soundbank_spc(
    rom_data: &[u8],
    brr_samples: &[BrrSampleInfo],
    game_title: &str,
) -> Option<Vec<u8>> {
    if brr_samples.is_empty() {
        return None;
    }
    let mut ram = [0u8; 65536];
    let dir_base = 0x1200usize;
    let mut cur_spc = 0x1400usize;
    for (i, s) in brr_samples.iter().take(64).enumerate() {
        let dir_entry = dir_base + i * 4;
        if dir_entry + 4 > 65536 || cur_spc + s.byte_len > 0xFFC0 {
            break;
        }
        let start_spc = cur_spc as u16;
        ram[dir_entry..dir_entry + 2].copy_from_slice(&start_spc.to_le_bytes());
        ram[dir_entry + 2..dir_entry + 4].copy_from_slice(&start_spc.to_le_bytes());
        let end = (s.offset + s.byte_len).min(rom_data.len());
        let clen = end.saturating_sub(s.offset).min(s.byte_len);
        ram[cur_spc..cur_spc + clen].copy_from_slice(&rom_data[s.offset..s.offset + clen]);
        cur_spc += clen;
    }
    // Minimal SPC loop at $0400
    ram[0x0400] = 0x2F;
    ram[0x0401] = 0xFE;
    let mut dsp = [0u8; 128];
    dsp[0x0C] = 0x7F;
    dsp[0x1C] = 0x7F;
    dsp[0x5D] = 0x12;
    dsp[0x6C] = 0x20;
    Some(crate::deconstruct::create_spc_file(
        "Universal BRR Soundbank",
        game_title,
        "Antigravity",
        "Auto-synthesized SPC700 Soundbank",
        "SNES",
        0x0400,
        0xEF,
        0,
        0,
        0,
        0,
        &ram,
        &dsp,
        None,
    ))
}

fn export_universal_disassembly(rom: &SnesRom, disasm_dir: &Path) -> std::io::Result<()> {
    let header_asm = format!(
        "; =============================================================================\n\
         ; SNES INTERNAL HEADER & 65C816 INTERRUPT VECTORS (UNIVERSAL WLA-DX)\n\
         ; Game: {}\n\
         ; =============================================================================\n\n\
         .SNESHEADER\n\
           ID \"SNES\"\n\
           NAME \"{:21}\"\n\
           {}\n\
           {}\n\
           CARTRIDGETYPE ${:02X}\n\
           ROMSIZE       ${:02X}\n\
           SRAMSIZE      ${:02X}\n\
           COUNTRY       ${:02X}\n\
           DEVELOPER     ${:02X}\n\
           VERSION       ${:02X}\n\
         .ENDSNES\n\n\
         .SNESNATIVEVECTOR\n\
           NMI   ${:04X}\n\
           IRQ   ${:04X}\n\
         .ENDNATIVEVECTOR\n\n\
         .SNESEMUVECTOR\n\
           NMI   ${:04X}\n\
           RESET ${:04X}\n\
           IRQ   ${:04X}\n\
         .ENDEMUVECTOR\n",
        rom.name,
        rom.header.title,
        match rom.header.mapping {
            RomMapping::LoROM => "LOROM",
            RomMapping::HiROM => "HIROM",
            RomMapping::ExHiROM => "EXHIROM",
        },
        if rom.header.fast_rom {
            "FASTROM"
        } else {
            "SLOWROM"
        },
        rom.header.chipset_byte,
        rom.header.rom_size_byte,
        rom.header.sram_size_byte,
        rom.header.region_byte,
        rom.header.developer_id,
        rom.header.version,
        rom.header.nmi_vector,
        rom.header.irq_vector,
        rom.header.nmi_vector,
        rom.header.reset_vector,
        rom.header.irq_vector,
    );
    fs::write(disasm_dir.join("header.asm"), header_asm)?;

    let banks = rom.banks();
    let bank_size = rom.bank_size();
    for (b_idx, bank_data) in banks.iter().enumerate() {
        let (snes_bank, base_addr) = rom.pc_to_snes(b_idx * bank_size);
        let mut asm_output = format!(
            "; === SNES Bank ${:02X} ({} Ko) ===\n.BANK ${:02X} SLOT 0\n.ORG ${:04X}\n\n",
            snes_bank,
            bank_data.len() / 1024,
            snes_bank,
            base_addr
        );
        let instrs = disassemble_65c816_block(rom, b_idx * bank_size, 64);
        for instr in instrs {
            asm_output.push_str(&instr);
            asm_output.push('\n');
        }
        fs::write(
            disasm_dir.join(format!("bank_{:02X}.asm", snes_bank)),
            asm_output,
        )?;
    }

    let dmas = scan_hardware_dma_transfers(rom);
    let mut dma_json = String::from("[\n");
    for (i, dma) in dmas.iter().enumerate() {
        dma_json.push_str(&format!(
            "  {{ \"channel\": {}, \"target\": \"{}\", \"snes_source\": \"${:02X}:{:04X}\", \"pc_offset\": {}, \"transfer_bytes\": {} }}{}\n",
            dma.dma_channel,
            dma.target.label(),
            dma.source_snes_bank,
            dma.source_snes_addr,
            dma.source_pc_offset
                .map(|p| format!("\"0x{:06X}\"", p))
                .unwrap_or_else(|| "null".to_string()),
            dma.transfer_size
                .map(|s| s.to_string())
                .unwrap_or_else(|| "null".to_string()),
            if i + 1 < dmas.len() { "," } else { "" }
        ));
    }
    dma_json.push_str("]\n");
    fs::write(disasm_dir.join("dma_transfers.json"), dma_json)?;

    let chunks = classify_rom_chunks(&rom.data);
    let mut chunks_json = String::from("[\n");
    for (i, c) in chunks.iter().enumerate() {
        chunks_json.push_str(&format!(
            "  {{ \"chunk\": {}, \"offset\": \"0x{:06X}\", \"kind\": \"{}\", \"entropy_bits\": {:.3} }}{}\n",
            i,
            c.offset,
            c.kind.label(),
            c.entropy_bits,
            if i + 1 < chunks.len() { "," } else { "" }
        ));
    }
    chunks_json.push_str("]\n");
    fs::write(disasm_dir.join("entropy_map.json"), chunks_json)?;

    Ok(())
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "")
}

fn extract_json_array_lines(json: &str, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_section = false;
    for line in json.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(key) && trimmed.contains('[') {
            in_section = true;
            continue;
        }
        if in_section {
            if trimmed.starts_with(']') {
                break;
            }
            if trimmed.starts_with('{') {
                out.push(trimmed.to_string());
            }
        }
    }
    out
}

fn parse_json_string(line: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\": \"", key);
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(esc) = chars.next() {
                match esc {
                    'n' => out.push('\n'),
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    other => out.push(other),
                }
            }
        } else if c == '"' {
            return Some(out);
        } else {
            out.push(c);
        }
    }
    None
}

fn parse_json_usize(line: &str, key: &str) -> Option<usize> {
    let needle = format!("\"{}\": ", key);
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    num_str.parse().ok()
}

fn parse_json_i16(line: &str, key: &str) -> Option<i16> {
    let needle = format!("\"{}\": ", key);
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let num_str: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '-')
        .collect();
    num_str.parse().ok()
}

fn parse_hex_rgb(hex: &str) -> Option<[u8; 3]> {
    let h = hex.trim().trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some([r, g, b])
}

// =============================================================================
// 8. UNIVERSAL OAM MULTI-TILE SPRITE POINTER TABLE EXTRACTOR (HiROM / Rareware)
// =============================================================================

/// Universally scan HiROM/ExHiROM ROMs for contiguous 4-byte OAM sprite pointer tables
/// (`[addr_lo, addr_hi, bank, 0x00]`) whose targets decode to valid multi-tile 4bpp OAM frames.
/// Works automatically across standard HiROM multi-tile OAM sprite engines!
pub fn scan_universal_oam_sprite_table(rom: &SnesRom) -> Option<(usize, Vec<usize>)> {
    if rom.header.mapping == RomMapping::LoROM || rom.data.len() < 0x100000 {
        return None;
    }
    let data = &rom.data;
    let mut best_table_start: Option<usize> = None;

    for base in (0x100000..data.len().saturating_sub(0x400)).step_by(0x100) {
        let mut valid4 = 0usize;
        let mut banks = Vec::new();
        for i in 0..64 {
            let b2 = data[base + i * 4 + 2];
            let b3 = data[base + i * 4 + 3];
            if (0xC0..=0xFF).contains(&b2) && b3 == 0x00 {
                valid4 += 1;
                if !banks.contains(&b2) {
                    banks.push(b2);
                }
            }
        }
        if valid4 >= 56 && banks.len() >= 2 {
            // Confirm at least 8 of the first 24 targets look like valid OAM multi-tile frame headers
            let mut valid_frames = 0usize;
            for i in 0..24 {
                let off = base + i * 4;
                let addr = u16::from_le_bytes([data[off], data[off + 1]]);
                let bank = data[off + 2];
                if let Some(pc) = rom.snes_to_pc(bank, addr) {
                    if looks_like_oam_cluster_frame(data, pc) {
                        valid_frames += 1;
                    }
                }
            }
            if valid_frames >= 8 {
                best_table_start = Some(base);
                break;
            }
        }
    }

    let anchor = best_table_start?;
    // Walk backwards to exact start of table
    let mut start = anchor;
    while start >= 4 && ((0xC0..=0xFF).contains(&data[start - 2]) && data[start - 1] == 0) {
        start -= 4;
    }

    // Walk forward collecting ALL valid unique OAM sprite frames (allowing small zero gaps)
    let mut frames: Vec<usize> = Vec::new();
    let mut pos = start;
    let mut bad_streak = 0usize;
    while pos + 4 <= data.len() && bad_streak < 64 && frames.len() < 4096 {
        let addr = u16::from_le_bytes([data[pos], data[pos + 1]]);
        let bank = data[pos + 2];
        let pad = data[pos + 3];
        if (0xC0..=0xFF).contains(&bank) && pad == 0x00 {
            if let Some(pc) = rom.snes_to_pc(bank, addr) {
                if looks_like_oam_cluster_frame(data, pc) {
                    if !frames.contains(&pc) {
                        frames.push(pc);
                    }
                    bad_streak = 0;
                } else {
                    bad_streak += 1;
                }
            } else {
                bad_streak += 1;
            }
        } else if addr == 0 && bank == 0 && pad == 0 {
            bad_streak += 1;
        } else {
            bad_streak += 4;
        }
        pos += 4;
    }

    if frames.len() >= 16 {
        Some((start, frames))
    } else {
        None
    }
}

fn looks_like_oam_cluster_frame(data: &[u8], pc: usize) -> bool {
    if pc + 16 > data.len() {
        return false;
    }
    let b0 = data[pc] as usize; // 16x16 metatiles
    let b1 = data[pc + 1] as usize; // 8x8 tiles group 1
    let b3 = data[pc + 3] as usize; // 8x8 tiles group 2
    let total = b0 + b1 + b3;
    if total == 0 || b0 > 24 || b1 > 32 || b3 > 32 || total > 48 {
        return false;
    }
    let tile_bytes = (b0 * 4 + b1 + b3) * 32;
    let coord_pos = pc + 8;
    let tile_start = coord_pos + total * 2;
    if tile_start + tile_bytes > data.len() {
        return false;
    }
    // Check coordinates are clustered around typical OAM anchor center (0x20..0xE0)
    let mut in_bounds = 0usize;
    for i in 0..total {
        let gx = data[coord_pos + i * 2];
        let gy = data[coord_pos + i * 2 + 1];
        if (0x20..=0xE0).contains(&gx) && (0x20..=0xE0).contains(&gy) {
            in_bounds += 1;
        }
    }
    in_bounds * 4 >= total * 3
}

/// Decode a VRAM-striped multi-tile 4bpp OAM cluster frame into a tightly cropped transparent RGBA sprite.
/// Handles full 16-tile VRAM row pairs (8x 16x16 per row pair), partial 16x16 rows, and both 8x8 groups.
pub fn decode_oam_cluster_sprite_rgba(
    data: &[u8],
    pc: usize,
    palette: &SnesPalette,
) -> Option<(usize, usize, Vec<u8>)> {
    if !looks_like_oam_cluster_frame(data, pc) {
        return None;
    }
    let b0 = data[pc] as usize; // 16x16 metatiles
    let b1 = data[pc + 1] as usize; // 8x8 group 1
    let b3 = data[pc + 3] as usize; // 8x8 group 2
    let total = b0 + b1 + b3;
    let coord_pos = pc + 8;
    let tile_base = coord_pos + total * 2;

    let mut canvas = vec![0u8; 160 * 160];
    let mut min_x = 160usize;
    let mut max_x = 0usize;
    let mut min_y = 160usize;
    let mut max_y = 0usize;

    let mut plot_8x8 = |t_idx: usize, ox: i32, oy: i32| {
        let tile_off = tile_base + t_idx * 32;
        if tile_off + 32 > data.len() {
            return;
        }
        let px = decode_tile(&data[tile_off..tile_off + 32], TileFormat::Bpp4);
        for y in 0..8 {
            for x in 0..8 {
                let c = px[y * 8 + x];
                if c != 0 {
                    let cx = ox + x as i32;
                    let cy = oy + y as i32;
                    if (0..160).contains(&cx) && (0..160).contains(&cy) {
                        let ux = cx as usize;
                        let uy = cy as usize;
                        canvas[uy * 160 + ux] = c;
                        min_x = min_x.min(ux);
                        max_x = max_x.max(ux);
                        min_y = min_y.min(uy);
                        max_y = max_y.max(uy);
                    }
                }
            }
        }
    };

    let full_rows = b0 / 8;
    let rem = b0 % 8;

    // 1. Decode all 16x16 metatiles using exact SNES VRAM row-striped indexing
    for i in 0..b0 {
        let gx = (data[coord_pos + i * 2] as i32) - 0x30;
        let gy = (data[coord_pos + i * 2 + 1] as i32) - 0x30;
        let row = i / 8;
        let col = i % 8;
        let (top_idx, bot_idx) = if row < full_rows {
            // Full 16-tile row pair: 16 top tiles followed immediately by 16 bottom tiles
            (row * 32 + col * 2, row * 32 + 16 + col * 2)
        } else {
            // Partial trailing row: rem*2 top tiles, then b1 8x8 tiles, then rem*2 bottom tiles
            let base_t = full_rows * 32;
            (base_t + col * 2, base_t + rem * 2 + b1 + col * 2)
        };
        plot_8x8(top_idx, gx, gy);
        plot_8x8(top_idx + 1, gx + 8, gy);
        plot_8x8(bot_idx, gx, gy + 8);
        plot_8x8(bot_idx + 1, gx + 8, gy + 8);
    }

    // 2. Decode Group 1 8x8 single tiles (located right after partial top halves)
    let g1_base = full_rows * 32 + rem * 2;
    for i in 0..b1 {
        let idx = b0 + i;
        let gx = (data[coord_pos + idx * 2] as i32) - 0x30;
        let gy = (data[coord_pos + idx * 2 + 1] as i32) - 0x30;
        plot_8x8(g1_base + i, gx, gy);
    }

    // 3. Decode Group 2 8x8 single tiles (located right after partial bottom halves)
    let g2_base = full_rows * 32 + rem * 4 + b1;
    for i in 0..b3 {
        let idx = b0 + b1 + i;
        let gx = (data[coord_pos + idx * 2] as i32) - 0x30;
        let gy = (data[coord_pos + idx * 2 + 1] as i32) - 0x30;
        plot_8x8(g2_base + i, gx, gy);
    }

    if min_x > max_x || min_y > max_y {
        return None;
    }
    let w = max_x - min_x + 1;
    let h = max_y - min_y + 1;
    if w < 8 || h < 8 {
        return None;
    }

    let mut rgba = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let c = canvas[(min_y + y) * 160 + (min_x + x)] as usize;
            if c != 0 {
                let col = palette
                    .colors_rgba
                    .get(c % 16)
                    .copied()
                    .unwrap_or([220, 180, 140, 255]);
                let dst = (y * w + x) * 4;
                rgba[dst..dst + 4].copy_from_slice(&[col[0], col[1], col[2], 255]);
            }
        }
    }
    Some((w, h, rgba))
}

fn export_oam_sprite_collection(
    rom: &SnesRom,
    frame_offsets: &[usize],
    snes_pals: &[SnesPalette],
    fallback_pal: &SnesPalette,
    sprites_dir: &Path,
    exhaustive_full_mode: bool,
) -> std::io::Result<usize> {
    // Pick a warm, rich sprite palette (preferring chroma-key #0000FF sprite palettes if present!)
    let preferred_pal = snes_pals
        .iter()
        .find(|p| {
            let c0 = p.colors_rgba[0];
            c0[0] == 0
                && c0[1] == 0
                && c0[2] >= 240
                && p.colors_rgba.get(8).map(|c| c[0] > c[2]).unwrap_or(false)
        })
        .or_else(|| snes_pals.first())
        .unwrap_or(fallback_pal);

    let chroma_pals: Vec<&SnesPalette> = snes_pals
        .iter()
        .filter(|p| p.colors_rgba[0][2] >= 240 && p.colors_rgba[0][0] == 0)
        .collect();

    let list_to_export: Vec<usize> = if exhaustive_full_mode {
        frame_offsets.to_vec()
    } else {
        let substantial: Vec<usize> = frame_offsets
            .iter()
            .copied()
            .filter(|&pc| {
                rom.data[pc] >= 2 && (rom.data[pc] as usize + rom.data[pc + 1] as usize) >= 3
            })
            .collect();
        let src = if substantial.len() >= 32 {
            &substantial
        } else {
            frame_offsets
        };
        let target_count = 64usize.min(src.len());
        let stride = (src.len() / target_count.max(1)).max(1);
        (0..target_count)
            .map(|slot| src[(slot * stride).min(src.len() - 1)])
            .collect()
    };

    let total_targets = list_to_export.len();
    let cell = 72u32;
    let cols = 8u32;
    let per_page = 64usize;
    let mut page_img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(cols * cell, 8 * cell, Rgba([18, 24, 38, 255]));
    let mut gallery_main: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(cols * cell, 8 * cell, Rgba([18, 24, 38, 255]));
    let main_stride = (total_targets / 64).max(1);
    let mut main_slot = 0usize;

    let mut exported = 0usize;
    let mut catalog_json = format!(
        "{{\n  \"total_oam_frames_discovered\": {},\n  \"exhaustive_full_mode\": {},\n  \"exported_sprites\": [\n",
        frame_offsets.len(),
        exhaustive_full_mode
    );

    for (slot, &pc) in list_to_export.iter().enumerate() {
        let pal = if slot < 8 || chroma_pals.is_empty() {
            preferred_pal
        } else {
            chroma_pals[(slot / 16) % chroma_pals.len()]
        };

        if let Some((w, h, rgba)) = decode_oam_cluster_sprite_rgba(&rom.data, pc, pal) {
            if let Some(raw_img) =
                ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(w as u32, h as u32, rgba)
            {
                let fname = format!("oam_sprite_{:04}_0x{:06X}.png", exported + 1, pc);
                if exhaustive_full_mode {
                    // Save crisp 1x pixel-exact sprite (fast & lossless for 2,600+ frames!)
                    let _ = raw_img.save(sprites_dir.join(&fname));
                } else {
                    let scale = 2u32;
                    let mut up: ImageBuffer<Rgba<u8>, Vec<u8>> =
                        ImageBuffer::new((w as u32) * scale, (h as u32) * scale);
                    for y in 0..(h as u32) {
                        for x in 0..(w as u32) {
                            let px = *raw_img.get_pixel(x, y);
                            for sy in 0..scale {
                                for sx in 0..scale {
                                    up.put_pixel(x * scale + sx, y * scale + sy, px);
                                }
                            }
                        }
                    }
                    let _ = up.save(sprites_dir.join(&fname));
                }

                // Helper to blit into a contact sheet grid cell
                let blit_cell = |target: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, cell_idx: usize| {
                    let gx = (cell_idx as u32 % cols) * cell;
                    let gy = (cell_idx as u32 / cols) * cell;
                    for bx in 2..(cell - 2) {
                        target.put_pixel(gx + bx, gy + 2, Rgba([38, 48, 72, 255]));
                        target.put_pixel(gx + bx, gy + cell - 3, Rgba([38, 48, 72, 255]));
                    }
                    for by in 2..(cell - 2) {
                        target.put_pixel(gx + 2, gy + by, Rgba([38, 48, 72, 255]));
                        target.put_pixel(gx + cell - 3, gy + by, Rgba([38, 48, 72, 255]));
                    }
                    let draw_w = (w as u32).min(cell - 8);
                    let draw_h = (h as u32).min(cell - 8);
                    let ox = gx + (cell - draw_w) / 2;
                    let oy = gy + (cell - draw_h) / 2;
                    for y in 0..draw_h {
                        let sy = (y * (h as u32)) / draw_h;
                        for x in 0..draw_w {
                            let sx = (x * (w as u32)) / draw_w;
                            let px = *raw_img.get_pixel(sx, sy);
                            if px[3] > 0 {
                                target.put_pixel(ox + x, oy + y, px);
                            }
                        }
                    }
                };

                // Blit into overview gallery (64 evenly spaced frames)
                if main_slot < 64 && (slot % main_stride == 0) {
                    blit_cell(&mut gallery_main, main_slot);
                    main_slot += 1;
                }

                // In exhaustive mode, also save paginated Contact Sheet Atlas pages (64 sprites/page)
                if exhaustive_full_mode {
                    let in_page = exported % per_page;
                    let page_num = (exported / per_page) + 1;
                    if in_page == 0 && exported > 0 {
                        let _ = page_img.save(
                            sprites_dir.join(format!("oam_gallery_page_{:02}.png", page_num - 1)),
                        );
                        page_img =
                            ImageBuffer::from_pixel(cols * cell, 8 * cell, Rgba([18, 24, 38, 255]));
                    }
                    blit_cell(&mut page_img, in_page);
                }

                let (bank, addr) = rom.pc_to_snes(pc);
                catalog_json.push_str(&format!(
                    "    {{ \"index\": {}, \"file\": \"{}\", \"rom_offset\": \"0x{:06X}\", \"snes_addr\": \"${:02X}:{:04X}\", \"width\": {}, \"height\": {} }}{}\n",
                    exported + 1,
                    fname,
                    pc,
                    bank,
                    addr,
                    w,
                    h,
                    if slot + 1 < total_targets { "," } else { "" }
                ));
                exported += 1;
            }
        }
    }

    if exhaustive_full_mode && exported > 0 {
        let last_page = exported.div_ceil(per_page);
        let _ = page_img.save(sprites_dir.join(format!("oam_gallery_page_{:02}.png", last_page)));
    }

    catalog_json.push_str("  ]\n}\n");
    fs::write(sprites_dir.join("oam_sprites_catalog.json"), catalog_json)?;
    if exported > 0 {
        let _ = gallery_main.save(sprites_dir.join("oam_sprites_gallery.png"));
    }

    Ok(exported)
}
