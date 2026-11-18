//! Universal Multi-Game SNES Omni-Mixer Engine (`N` Donor ROMs -> 1 Base ROM)
//!
//! Enables mixing **anything with anything** across any combination of SNES ROMs:
//! - Semantic high-fidelity cross-game mixing for recognized games (Zelda ALTTP, Super Mario World,
//!   Donkey Kong Country): heroes, mounts, soldiers/enemies, collectibles/items, world tiles, palettes,
//!   and unpitched BRR percussion audio.
//! - 100% generic ROM-agnostic mixing (`gfx:N`, `pal:N`, `audio:N`) powered by hardware DMA/LZ2/BRR
//!   discovery for ANY unknown LoROM / HiROM / ExHiROM cartridge.
//! - Reproducible JSON `MixRecipe` serialization/deserialization + deterministic Chaos Randomizer.

use std::collections::HashMap;
use std::time::Instant;

use crate::audio::{resample_and_encode_brr_for_swap, scan_brr_samples};
use crate::demo::{
    apply_donkey_kong_into_smw_yoshi, apply_link_into_smw_swap, apply_link_world_title_screen,
    apply_mario_into_zelda_alttp_swap, apply_swap_into_dkc,
};
use crate::disasm_dma::{compress_nintendo_lz2_with_endian, decompress_nintendo_lz2_with_endian};
use crate::graphics::{remap_and_transcode_tiles, SnesPalette, TileFormat};
use crate::rom::{RecognizedGame, RomError, SnesRom};
use crate::swapper::{SwapKind, SwapSession};
use crate::universal::{
    discover_universal_catalog, UniversalGfxCompression, UniversalRomCatalog,
};

// =============================================================================
// DATA MODEL: SLOTS, DONOR SOURCES, RECIPES & REPORTS
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MixSlotCategory {
    Hero,
    Mount,
    Soldiers,
    Enemies,
    Items,
    WorldTiles,
    Palette,
    Audio,
    RawGfx,
    RawPalette,
    RawAudio,
}

impl MixSlotCategory {
    pub fn label_fr(self) -> &'static str {
        match self {
            Self::Hero => "Héros jouable",
            Self::Mount => "Monture / Compagnon",
            Self::Soldiers => "Soldats / Gardes",
            Self::Enemies => "Ennemis",
            Self::Items => "Objets & Collectibles",
            Self::WorldTiles => "Décors & Blocs",
            Self::Palette => "Palettes Couleurs",
            Self::Audio => "Audio & Percussions BRR",
            Self::RawGfx => "Planche GFX Universelle",
            Self::RawPalette => "Palette BGR555 Universelle",
            Self::RawAudio => "Sample BRR Universel",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Hero => "★",
            Self::Mount => "⚡",
            Self::Soldiers => "🚩",
            Self::Enemies => "👾",
            Self::Items => "💎",
            Self::WorldTiles => "📦",
            Self::Palette => "🎨",
            Self::Audio => "🎵",
            Self::RawGfx => "🌐",
            Self::RawPalette => "🌈",
            Self::RawAudio => "🔊",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MixTargetSlot {
    pub id: String,
    pub label: String,
    pub category: MixSlotCategory,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct MixDonorSource {
    pub id: String,
    pub label: String,
    pub category: MixSlotCategory,
    pub compatible_targets: Vec<MixSlotCategory>,
    pub description: String,
    /// Optional RGBA preview (16x16 or 16x32) for GUI rendering
    pub preview_rgba: Vec<u8>,
    pub preview_w: u32,
    pub preview_h: u32,
}

#[derive(Debug, Clone)]
pub struct GameMixProfile {
    pub alias: String,
    pub rom_path: String,
    pub title: String,
    pub recognized: RecognizedGame,
    pub mapping: String,
    pub size_bytes: usize,
    pub target_slots: Vec<MixTargetSlot>,
    pub donor_sources: Vec<MixDonorSource>,
    pub universal_catalog: UniversalRomCatalog,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixRule {
    pub target_slot: String,
    pub donor_alias: String,
    pub donor_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixRecipe {
    pub name: String,
    pub custom_title: Option<String>,
    pub base_rom: String,
    pub donors: Vec<(String, String)>,
    pub rules: Vec<MixRule>,
}

#[derive(Debug, Clone)]
pub struct MixPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub custom_title: String,
    pub rules: Vec<MixRule>,
}

#[derive(Debug, Clone)]
pub struct MixReport {
    pub base_title: String,
    pub output_title: String,
    pub operations_applied: usize,
    pub applied_rules: Vec<String>,
    pub warnings: Vec<String>,
    pub checksum: u16,
    pub complement: u16,
    pub checksum_valid: bool,
    pub crc32: u32,
    pub elapsed_ms: u128,
}

// =============================================================================
// JSON SERIALIZATION FOR MIX RECIPES (ZERO EXTERNAL DEPS)
// =============================================================================

fn escape_json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

impl MixRecipe {
    pub fn to_json(&self) -> String {
        let mut s = String::from("{\n");
        s.push_str(&format!("  \"name\": {},\n", escape_json_str(&self.name)));
        match &self.custom_title {
            Some(t) => s.push_str(&format!("  \"custom_title\": {},\n", escape_json_str(t))),
            None => s.push_str("  \"custom_title\": null,\n"),
        }
        s.push_str(&format!(
            "  \"base_rom\": {},\n",
            escape_json_str(&self.base_rom)
        ));
        s.push_str("  \"donors\": [\n");
        for (i, (alias, path)) in self.donors.iter().enumerate() {
            let comma = if i + 1 < self.donors.len() { "," } else { "" };
            s.push_str(&format!(
                "    {{ \"alias\": {}, \"path\": {} }}{}\n",
                escape_json_str(alias),
                escape_json_str(path),
                comma
            ));
        }
        s.push_str("  ],\n");
        s.push_str("  \"rules\": [\n");
        for (i, r) in self.rules.iter().enumerate() {
            let comma = if i + 1 < self.rules.len() { "," } else { "" };
            s.push_str(&format!(
                "    {{ \"target\": {}, \"donor\": {}, \"source\": {} }}{}\n",
                escape_json_str(&r.target_slot),
                escape_json_str(&r.donor_alias),
                escape_json_str(&r.donor_source),
                comma
            ));
        }
        s.push_str("  ]\n");
        s.push_str("}\n");
        s
    }

    pub fn from_json(input: &str) -> Result<Self, String> {
        let extract_str = |src: &str, key: &str| -> Option<String> {
            let pat = format!("\"{}\"", key);
            let pos = src.find(&pat)?;
            let rest = &src[pos + pat.len()..];
            let colon = rest.find(':')?;
            let after = rest[colon + 1..].trim_start();
            if after.starts_with("null") {
                return None;
            }
            if !after.starts_with('"') {
                return None;
            }
            let mut out = String::new();
            let mut chars = after[1..].chars();
            while let Some(c) = chars.next() {
                if c == '\\' {
                    if let Some(esc) = chars.next() {
                        match esc {
                            'n' => out.push('\n'),
                            'r' => out.push('\r'),
                            't' => out.push('\t'),
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
        };

        let extract_array_objs = |src: &str, key: &str| -> Vec<String> {
            let pat = format!("\"{}\"", key);
            let Some(pos) = src.find(&pat) else {
                return Vec::new();
            };
            let rest = &src[pos + pat.len()..];
            let Some(lb) = rest.find('[') else {
                return Vec::new();
            };
            let mut depth = 0i32;
            let mut in_str = false;
            let mut esc = false;
            let mut objs = Vec::new();
            let mut cur_start: Option<usize> = None;
            for (idx, ch) in rest[lb + 1..].char_indices() {
                if in_str {
                    if esc {
                        esc = false;
                    } else if ch == '\\' {
                        esc = true;
                    } else if ch == '"' {
                        in_str = false;
                    }
                    continue;
                }
                match ch {
                    '"' => in_str = true,
                    '{' => {
                        if depth == 0 {
                            cur_start = Some(lb + 1 + idx);
                        }
                        depth += 1;
                    }
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            if let Some(st) = cur_start.take() {
                                objs.push(rest[st..=lb + 1 + idx].to_string());
                            }
                        }
                    }
                    ']' if depth == 0 => break,
                    _ => {}
                }
            }
            objs
        };

        let name = extract_str(input, "name").unwrap_or_else(|| "Custom Mix".to_string());
        let custom_title = extract_str(input, "custom_title");
        let base_rom =
            extract_str(input, "base_rom").ok_or_else(|| "Champ 'base_rom' manquant".to_string())?;

        let mut donors = Vec::new();
        for obj in extract_array_objs(input, "donors") {
            if let (Some(alias), Some(path)) =
                (extract_str(&obj, "alias"), extract_str(&obj, "path"))
            {
                donors.push((alias, path));
            }
        }

        let mut rules = Vec::new();
        for obj in extract_array_objs(input, "rules") {
            if let (Some(target), Some(donor), Some(source)) = (
                extract_str(&obj, "target"),
                extract_str(&obj, "donor"),
                extract_str(&obj, "source"),
            ) {
                rules.push(MixRule {
                    target_slot: target,
                    donor_alias: donor,
                    donor_source: source,
                });
            }
        }

        Ok(Self {
            name,
            custom_title,
            base_rom,
            donors,
            rules,
        })
    }
}

// =============================================================================
// LOW-LEVEL 3BPP / 4BPP / POINTER-TABLE HELPERS (SMW, ZELDA, DKC, GENERIC)
// =============================================================================

fn find_smw_gfx_table(smw: &[u8]) -> Option<(usize, usize, usize)> {
    for i in 0x003000..0x004400usize {
        if i + 0x64 < smw.len()
            && smw[i] == 0x1E
            && smw[i + 0x32] == 0xEF
            && smw[i + 0x64] == 0x09
        {
            let lo = i - 0x10;
            return Some((lo, lo + 0x32, lo + 0x64));
        }
    }
    None
}

fn load_smw_3bpp_sheet(smw: &[u8], sheet: usize) -> Option<(Vec<u8>, usize, usize)> {
    let (lo_b, hi_b, bk_b) = find_smw_gfx_table(smw)?;
    if sheet >= 0x32 {
        return None;
    }
    let pc = ((smw[bk_b + sheet] as usize & 0x7F) << 15)
        | ((smw[hi_b + sheet] as usize & 0x7F) << 8)
        | (smw[lo_b + sheet] as usize);
    let (data, clen) = decompress_nintendo_lz2_with_endian(smw, pc, 4096, true)?;
    Some((data, pc, clen))
}

fn find_alttp_gfx_table(zelda: &[u8]) -> Option<(usize, usize, usize)> {
    for b in 0x004E80..0x005000usize {
        let h = b + 223;
        let l = b + 446;
        if l + 223 <= zelda.len()
            && zelda[b + 0x71] == 0x10
            && zelda[h + 0x71] == 0xF0
            && zelda[l + 0x71] == 0x00
            && zelda[b + 0x72] == 0x10
            && zelda[h + 0x72] == 0xF6
        {
            return Some((b, h, l));
        }
    }
    None
}

pub(crate) fn load_alttp_3bpp_sheet(zelda: &[u8], sheet: usize) -> Option<(Vec<u8>, usize, usize)> {
    let (z_bk, z_hi, z_lo) = find_alttp_gfx_table(zelda)?;
    if sheet >= 223 {
        return None;
    }
    let pc = ((zelda[z_bk + sheet] as usize & 0x7F) << 15)
        | ((zelda[z_hi + sheet] as usize & 0x7F) << 8)
        | (zelda[z_lo + sheet] as usize);
    if (0x71..=0x7E).contains(&sheet) {
        if pc + 0x600 <= zelda.len() {
            return Some((zelda[pc..pc + 0x600].to_vec(), pc, 0x600));
        }
        return None;
    }
    let (data, clen) = decompress_nintendo_lz2_with_endian(zelda, pc, 4096, false)?;
    Some((data, pc, clen))
}

fn read_3bpp_16x16(buf: &[u8], tx: usize, ty: usize) -> [[u8; 16]; 16] {
    let mut px = [[0u8; 16]; 16];
    for dy in 0..2usize {
        for dx in 0..2usize {
            let ti = (ty + dy) * 16 + (tx + dx);
            let off = ti * 24;
            if off + 24 > buf.len() {
                continue;
            }
            for y in 0..8usize {
                let b0 = buf[off + y * 2];
                let b1 = buf[off + y * 2 + 1];
                let b2 = buf[off + 16 + y];
                for x in 0..8usize {
                    let sh = 7 - x;
                    px[dy * 8 + y][dx * 8 + x] = ((b0 >> sh) & 1)
                        | (((b1 >> sh) & 1) << 1)
                        | (((b2 >> sh) & 1) << 2);
                }
            }
        }
    }
    px
}

fn write_3bpp_16x16(buf: &mut [u8], tx: usize, ty: usize, px: &[[u8; 16]; 16]) {
    for dy in 0..2usize {
        for dx in 0..2usize {
            let ti = (ty + dy) * 16 + (tx + dx);
            let off = ti * 24;
            if off + 24 > buf.len() {
                continue;
            }
            for y in 0..8usize {
                let (mut b0, mut b1, mut b2) = (0u8, 0u8, 0u8);
                for x in 0..8usize {
                    let c = px[dy * 8 + y][dx * 8 + x] & 7;
                    let sh = 7 - x;
                    b0 |= (c & 1) << sh;
                    b1 |= ((c >> 1) & 1) << sh;
                    b2 |= ((c >> 2) & 1) << sh;
                }
                buf[off + y * 2] = b0;
                buf[off + y * 2 + 1] = b1;
                buf[off + 16 + y] = b2;
            }
        }
    }
}

fn remap_16x16(src: &[[u8; 16]; 16], lut: &[u8; 8]) -> [[u8; 16]; 16] {
    let mut out = [[0u8; 16]; 16];
    for y in 0..16 {
        for x in 0..16 {
            out[y][x] = lut[(src[y][x] & 7) as usize];
        }
    }
    out
}

fn flip_h_16x16(src: &[[u8; 16]; 16]) -> [[u8; 16]; 16] {
    let mut out = [[0u8; 16]; 16];
    for y in 0..16 {
        for x in 0..16 {
            out[y][x] = src[y][15 - x];
        }
    }
    out
}

fn solid_rgba(r: u8, g: u8, b: u8, w: usize, h: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(w * h * 4);
    for _ in 0..(w * h) {
        out.extend_from_slice(&[r, g, b, 255]);
    }
    out
}

/// Crisp 16x16 pixel-art audio equalizer / waveform badge for BRR sound donor previews.
fn audio_wave_rgba(r: u8, g: u8, b: u8) -> Vec<u8> {
    const BARS: [usize; 16] = [0, 2, 5, 3, 9, 13, 10, 6, 12, 14, 8, 5, 10, 6, 2, 0];
    let mut out = vec![0u8; 16 * 16 * 4];
    for y in 0..16 {
        for x in 0..16 {
            let idx = (y * 16 + x) * 4;
            let h = BARS[x];
            let top = 8usize.saturating_sub(h / 2);
            let bot = (8 + h.div_ceil(2)).min(15);
            if h > 0 && y >= top && y <= bot {
                let hi = y == top || y == bot;
                out[idx] = if hi { r.saturating_add(35) } else { r };
                out[idx + 1] = if hi { g.saturating_add(35) } else { g };
                out[idx + 2] = if hi { b.saturating_add(35) } else { b };
                out[idx + 3] = 255;
            } else {
                out[idx] = 18;
                out[idx + 1] = 24;
                out[idx + 2] = 38;
                out[idx + 3] = 255;
            }
        }
    }
    out
}

/// Convert a 3bpp/4bpp 16x16 indexed pixel grid + palette into a 16x16 RGBA preview buffer
fn render_16x16_preview(px: &[[u8; 16]; 16], pal: &[[u8; 4]]) -> Vec<u8> {
    let mut out = vec![0u8; 16 * 16 * 4];
    for y in 0..16 {
        for x in 0..16 {
            let c = px[y][x] as usize;
            let dst = (y * 16 + x) * 4;
            if c == 0 {
                out[dst..dst + 4].copy_from_slice(&[20, 26, 36, 255]);
            } else if c < pal.len() {
                out[dst..dst + 4].copy_from_slice(&pal[c]);
            }
        }
    }
    out
}

/// Convert two stacked 16x16 tiles into a 16x32 RGBA preview buffer
fn render_16x32_preview(top: &[[u8; 16]; 16], bot: &[[u8; 16]; 16], pal: &[[u8; 4]]) -> Vec<u8> {
    let mut out = vec![0u8; 16 * 32 * 4];
    for (half, src) in [(0usize, top), (16usize, bot)] {
        for y in 0..16 {
            for x in 0..16 {
                let c = src[y][x] as usize;
                let dst = ((half + y) * 16 + x) * 4;
                if c == 0 {
                    out[dst..dst + 4].copy_from_slice(&[20, 26, 36, 255]);
                } else if c < pal.len() {
                    out[dst..dst + 4].copy_from_slice(&pal[c]);
                }
            }
        }
    }
    out
}

fn default_dk_palette_rgba() -> Vec<[u8; 4]> {
    vec![
        [0, 0, 0, 0],
        [255, 255, 255, 255], // 1: White eyes
        [24, 16, 12, 255],    // 2: Dark outline
        [104, 48, 24, 255],   // 3: Deep fur brown
        [164, 84, 40, 255],   // 4: Mid fur brown
        [216, 136, 72, 255],  // 5: Fur highlight
        [236, 36, 36, 255],   // 6: Iconic Red Tie
        [248, 196, 136, 255], // 7: Warm tan muzzle/chest
    ]
}

fn default_smw_enemy_palette_rgba() -> Vec<[u8; 4]> {
    vec![
        [0, 0, 0, 0],
        [255, 255, 255, 255], // 1: White
        [24, 24, 24, 255],    // 2: Black
        [160, 88, 40, 255],   // 3: Brown
        [248, 200, 80, 255],  // 4: Gold/Yellow
        [255, 224, 168, 255], // 5: Cream/Tan
        [232, 112, 48, 255],  // 6: Bright Orange/Brown
        [184, 56, 32, 255],   // 7: Shaded Red/Brown
    ]
}

/// Extract a 16x16 3bpp sprite (already mapped to Zelda ALTTP's universal 3bpp index convention:
/// 0=trans, 1=white, 2=accent, 3=brown, 4=tan, 5=dark outline, 6=bright primary, 7=dark primary)
/// from ANY donor ROM + source id!
fn extract_donor_16x16_for_zelda(
    donor_rom: &SnesRom,
    source_id: &str,
    for_soldier: bool,
) -> Option<([[u8; 16]; 16], [[u8; 16]; 16])> {
    const LUT_STD: [u8; 8] = [0, 1, 5, 3, 2, 4, 6, 7];
    const LUT_GOOMBA_SOL: [u8; 8] = [0, 1, 5, 7, 6, 4, 4, 3];
    const LUT_MUSHROOM: [u8; 8] = [0, 1, 5, 7, 6, 6, 6, 7];
    const LUT_GOLD: [u8; 8] = [0, 1, 5, 3, 2, 6, 6, 3];
    const LUT_BOBOMB: [u8; 8] = [0, 1, 5, 6, 2, 1, 5, 5];
    const LUT_DK_TO_ZELDA: [u8; 8] = [0, 1, 5, 3, 3, 4, 7, 4];

    match source_id {
        "goomba" => {
            let (g01, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x01)?;
            let lut = if for_soldier { &LUT_GOOMBA_SOL } else { &LUT_STD };
            Some((
                remap_16x16(&read_3bpp_16x16(&g01, 8, 2), lut),
                remap_16x16(&read_3bpp_16x16(&g01, 10, 2), lut),
            ))
        }
        "koopa_shell" => {
            let (g01, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x01)?;
            Some((
                remap_16x16(&read_3bpp_16x16(&g01, 10, 0), &LUT_STD),
                remap_16x16(&read_3bpp_16x16(&g01, 12, 0), &LUT_STD),
            ))
        }
        "spiny" => {
            let (g02, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x02)?;
            Some((
                remap_16x16(&read_3bpp_16x16(&g02, 0, 0), &LUT_STD),
                remap_16x16(&read_3bpp_16x16(&g02, 2, 0), &LUT_STD),
            ))
        }
        "boo" | "sewer_pack" => {
            let (g06, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x06)?;
            Some((
                remap_16x16(&read_3bpp_16x16(&g06, 12, 0), &LUT_STD),
                remap_16x16(&read_3bpp_16x16(&g06, 14, 0), &LUT_STD),
            ))
        }
        "piranha" | "overworld_pack" => {
            let (g01, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x01)?;
            Some((
                remap_16x16(&read_3bpp_16x16(&g01, 12, 2), &LUT_STD),
                remap_16x16(&read_3bpp_16x16(&g01, 14, 2), &LUT_STD),
            ))
        }
        "magikoopa" | "dungeon_pack" => {
            let (g03, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x03)?;
            Some((
                remap_16x16(&read_3bpp_16x16(&g03, 0, 2), &LUT_STD),
                remap_16x16(&read_3bpp_16x16(&g03, 2, 2), &LUT_STD),
            ))
        }
        "bobomb" => {
            let (g02, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x02)?;
            let b = remap_16x16(&read_3bpp_16x16(&g02, 10, 4), &LUT_BOBOMB);
            Some((b, flip_h_16x16(&b)))
        }
        "coin" => {
            let (g01, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x01)?;
            let c = remap_16x16(&read_3bpp_16x16(&g01, 8, 6), &LUT_GOLD);
            Some((c, c))
        }
        "mushroom" => {
            let (g00, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x00)?;
            let m = remap_16x16(&read_3bpp_16x16(&g00, 4, 2), &LUT_MUSHROOM);
            Some((m, m))
        }
        "qblock" => {
            let (g00, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x00)?;
            let q = remap_16x16(&read_3bpp_16x16(&g00, 10, 2), &[0, 1, 2, 3, 7, 6, 6, 3]);
            Some((q, q))
        }
        "baby_dk" | "dk" => {
            let dk1 = remap_16x16(&crate::ai_sprites::DK_BABY_DK, &LUT_DK_TO_ZELDA);
            Some((dk1, flip_h_16x16(&dk1)))
        }
        "zelda_soldier" => {
            let (sba, _, _) = load_alttp_3bpp_sheet(&donor_rom.data, 0xBA)?;
            Some((read_3bpp_16x16(&sba, 0, 0), read_3bpp_16x16(&sba, 2, 0)))
        }
        "zelda_octorok" => {
            let (s7f, _, _) = load_alttp_3bpp_sheet(&donor_rom.data, 0x7F)?;
            let o = read_3bpp_16x16(&s7f, 14, 0);
            Some((o, flip_h_16x16(&o)))
        }
        "zelda_stalfos" => {
            let (s90, _, _) = load_alttp_3bpp_sheet(&donor_rom.data, 0x90)?;
            Some((read_3bpp_16x16(&s90, 0, 0), read_3bpp_16x16(&s90, 2, 0)))
        }
        _ => None,
    }
}

/// Extract a 16x16 3bpp sprite mapped to SMW's universal 3bpp index convention
/// (0=trans, 1=white, 2=black, 3=dark brown, 4=gold/tan, 5=cream, 6=bright primary, 7=dark primary)
fn extract_donor_16x16_for_smw(
    donor_rom: &SnesRom,
    source_id: &str,
) -> Option<([[u8; 16]; 16], [[u8; 16]; 16])> {
    // Inverse of LUT_STD: Zelda (0,1,2,3,4,5,6,7) -> SMW (0,1,4,3,5,2,6,7)
    const LUT_ZELDA_TO_SMW: [u8; 8] = [0, 1, 4, 3, 5, 2, 6, 7];

    match source_id {
        "zelda_soldier" => {
            let (sba, _, _) = load_alttp_3bpp_sheet(&donor_rom.data, 0xBA)?;
            Some((
                remap_16x16(&read_3bpp_16x16(&sba, 0, 0), &LUT_ZELDA_TO_SMW),
                remap_16x16(&read_3bpp_16x16(&sba, 2, 0), &LUT_ZELDA_TO_SMW),
            ))
        }
        "zelda_octorok" => {
            let (s7f, _, _) = load_alttp_3bpp_sheet(&donor_rom.data, 0x7F)?;
            let o = remap_16x16(&read_3bpp_16x16(&s7f, 14, 0), &LUT_ZELDA_TO_SMW);
            Some((o, flip_h_16x16(&o)))
        }
        "zelda_stalfos" => {
            let (s90, _, _) = load_alttp_3bpp_sheet(&donor_rom.data, 0x90)?;
            Some((
                remap_16x16(&read_3bpp_16x16(&s90, 0, 0), &LUT_ZELDA_TO_SMW),
                remap_16x16(&read_3bpp_16x16(&s90, 2, 0), &LUT_ZELDA_TO_SMW),
            ))
        }
        "zelda_rupee" => {
            let (scd, _, _) = load_alttp_3bpp_sheet(&donor_rom.data, 0xCD)?;
            let r = remap_16x16(&read_3bpp_16x16(&scd, 0, 0), &LUT_ZELDA_TO_SMW);
            Some((r, r))
        }
        "zelda_heart" => {
            let (scd, _, _) = load_alttp_3bpp_sheet(&donor_rom.data, 0xCD)?;
            let h = remap_16x16(&read_3bpp_16x16(&scd, 0, 2), &LUT_ZELDA_TO_SMW);
            Some((h, h))
        }
        "baby_dk" | "dk" => {
            let dk = crate::ai_sprites::DK_BABY_DK;
            Some((dk, flip_h_16x16(&dk)))
        }
        _ => None,
    }
}

// =============================================================================
// GAME PROFILE DISCOVERY (WORKS FOR ANY ROM: KNOWN OR COMPLETELY UNKNOWN)
// =============================================================================

pub fn inspect_rom_for_mixing(
    rom: &SnesRom,
    alias: impl Into<String>,
    rom_path: impl Into<String>,
) -> GameMixProfile {
    let alias = alias.into();
    let rom_path = rom_path.into();
    let recognized = rom.identify_game();
    let universal_catalog = discover_universal_catalog(rom);

    let mut target_slots = Vec::new();
    let mut donor_sources = Vec::new();

    match recognized {
        RecognizedGame::ZeldaAlttp => {
            target_slots.extend([
                MixTargetSlot {
                    id: "hero".into(),
                    label: "Héros jouable (Link -> 16x32 complet + épée 4 dir)".into(),
                    category: MixSlotCategory::Hero,
                    description: "Remplace Link (0x080000..0x087000 + Bank $0D OAM Y=-8 + palettes)".into(),
                },
                MixTargetSlot {
                    id: "soldiers".into(),
                    label: "Soldats Épée & Lance d'Hyrule (Sheet 0xBA)".into(),
                    category: MixSlotCategory::Soldiers,
                    description: "Tous les soldats verts/bleus/rouges d'Hyrule et du château (mono-OAM)".into(),
                },
                MixTargetSlot {
                    id: "elite_knights".into(),
                    label: "Chevaliers d'Élite à cornes (Sheet 0xB7)".into(),
                    category: MixSlotCategory::Soldiers,
                    description: "Soldats d'élite et capitaines d'Hyrule".into(),
                },
                MixTargetSlot {
                    id: "castle_guards".into(),
                    label: "Gardes & Archers du Château (Sheet 0x8C)".into(),
                    category: MixSlotCategory::Soldiers,
                    description: "Gardes spéciaux et boucliers".into(),
                },
                MixTargetSlot {
                    id: "sewer_enemies".into(),
                    label: "Ennemis Égouts du Château (Sheet 0x8D)".into(),
                    category: MixSlotCategory::Enemies,
                    description: "Chauves-souris, rats et serpents du passage secret".into(),
                },
                MixTargetSlot {
                    id: "overworld_enemies".into(),
                    label: "Ennemis Overworld (Sheets 0x7F & 0x82)".into(),
                    category: MixSlotCategory::Enemies,
                    description: "Octoroks, Buzzblobs et Tektites de la plaine d'Hyrule".into(),
                },
                MixTargetSlot {
                    id: "dungeon_enemies".into(),
                    label: "Squelettes Stalfos de Donjons (Sheet 0x90)".into(),
                    category: MixSlotCategory::Enemies,
                    description: "Squelettes et ennemis des donjons".into(),
                },
                MixTargetSlot {
                    id: "items".into(),
                    label: "Collectibles & Objets (Rubis, Cœurs, Bombes, Clés)".into(),
                    category: MixSlotCategory::Items,
                    description: "Sheets 0xCD, 0x72, 0x7B et 0x76".into(),
                },
                MixTargetSlot {
                    id: "world_rocks".into(),
                    label: "Rochers soulevables Overworld (Sheets 0x06..0x08)".into(),
                    category: MixSlotCategory::WorldTiles,
                    description: "Rochers clairs et sombres de l'Overworld d'Hyrule".into(),
                },
                MixTargetSlot {
                    id: "audio_percussion".into(),
                    label: "Percussions Audio BRR (sans altérer la mélodie)".into(),
                    category: MixSlotCategory::Audio,
                    description: "Échantillons BRR non bouclés (!loops) de Zelda".into(),
                },
            ]);

            // Zelda as Donor!
            let z_pal = SnesPalette::zelda_link_full_16(&rom.data, 0x0DD308, "Link", false).colors_rgba;
            let link_head = read_4bpp_alttp_cell(&rom.data, 1, 0);
            let link_body = read_4bpp_alttp_cell(&rom.data, 3, 1);
            donor_sources.push(MixDonorSource {
                id: "link".into(),
                label: "Link (Zelda ALTTP — 16x32 complet)".into(),
                category: MixSlotCategory::Hero,
                compatible_targets: vec![MixSlotCategory::Hero],
                description: "Vrais sprites 4bpp de Link + palettes Tunique Verte/Bleue/Rouge".into(),
                preview_rgba: render_16x32_preview(&link_head, &link_body, &z_pal),
                preview_w: 16,
                preview_h: 32,
            });
            if let Some((sba, _, _)) = load_alttp_3bpp_sheet(&rom.data, 0xBA) {
                let p = read_3bpp_16x16(&sba, 0, 0);
                let pal = alttp_sprite_pal_rgba(&rom.data, 0x0DD3E4);
                donor_sources.push(MixDonorSource {
                    id: "zelda_soldier".into(),
                    label: "Soldat d'Hyrule (Zelda ALTTP)".into(),
                    category: MixSlotCategory::Soldiers,
                    compatible_targets: vec![MixSlotCategory::Soldiers, MixSlotCategory::Enemies],
                    description: "Casque et garde d'Hyrule (Sheet 0xBA)".into(),
                    preview_rgba: render_16x16_preview(&p, &pal),
                    preview_w: 16,
                    preview_h: 16,
                });
            }
            if let Some((s7f, _, _)) = load_alttp_3bpp_sheet(&rom.data, 0x7F) {
                let p = read_3bpp_16x16(&s7f, 14, 0);
                let pal = alttp_sprite_pal_rgba(&rom.data, 0x0DD3D6);
                donor_sources.push(MixDonorSource {
                    id: "zelda_octorok".into(),
                    label: "Octorok (Zelda ALTTP)".into(),
                    category: MixSlotCategory::Enemies,
                    compatible_targets: vec![MixSlotCategory::Soldiers, MixSlotCategory::Enemies],
                    description: "Ennemi Octorok classique de la plaine d'Hyrule".into(),
                    preview_rgba: render_16x16_preview(&p, &pal),
                    preview_w: 16,
                    preview_h: 16,
                });
            }
            if let Some((s90, _, _)) = load_alttp_3bpp_sheet(&rom.data, 0x90) {
                let p = read_3bpp_16x16(&s90, 0, 0);
                let pal = alttp_sprite_pal_rgba(&rom.data, 0x0DD3D6);
                donor_sources.push(MixDonorSource {
                    id: "zelda_stalfos".into(),
                    label: "Squelette Stalfos (Zelda ALTTP)".into(),
                    category: MixSlotCategory::Enemies,
                    compatible_targets: vec![MixSlotCategory::Soldiers, MixSlotCategory::Enemies],
                    description: "Crâne Stalfos des donjons d'Hyrule".into(),
                    preview_rgba: render_16x16_preview(&p, &pal),
                    preview_w: 16,
                    preview_h: 16,
                });
            }
            if let Some((scd, _, _)) = load_alttp_3bpp_sheet(&rom.data, 0xCD) {
                let p = read_3bpp_16x16(&scd, 0, 0);
                let pal = alttp_sprite_pal_rgba(&rom.data, 0x0DD3E4);
                donor_sources.push(MixDonorSource {
                    id: "zelda_items_pack".into(),
                    label: "Pack Objets Zelda (Rubis & Cœurs)".into(),
                    category: MixSlotCategory::Items,
                    compatible_targets: vec![MixSlotCategory::Items],
                    description: "Rubis et Réceptacles de Cœur d'Hyrule".into(),
                    preview_rgba: render_16x16_preview(&p, &pal),
                    preview_w: 16,
                    preview_h: 16,
                });
            }
            donor_sources.push(MixDonorSource {
                id: "hyrule_theme".into(),
                label: "Thème Narratif Hyrule (Titre, HUD, Overworld)".into(),
                category: MixSlotCategory::WorldTiles,
                compatible_targets: vec![MixSlotCategory::WorldTiles],
                description: "Titre SUPER LINK WORLD, sélection LINK A/B/C et histoire d'Hyrule".into(),
                preview_rgba: solid_rgba(40, 140, 60, 16, 16),
                preview_w: 16,
                preview_h: 16,
            });
            donor_sources.push(MixDonorSource {
                id: "percussion".into(),
                label: "Percussions BRR Zelda ALTTP".into(),
                category: MixSlotCategory::Audio,
                compatible_targets: vec![MixSlotCategory::Audio],
                description: "Échantillons BRR percussifs non bouclés de Zelda".into(),
                preview_rgba: solid_rgba(60, 120, 200, 16, 16),
                preview_w: 16,
                preview_h: 16,
            });
        }
        RecognizedGame::SuperMarioWorld => {
            target_slots.extend([
                MixTargetSlot {
                    id: "hero".into(),
                    label: "Héros jouable (Mario GFX32 + Overworld GFX10 + Palette)".into(),
                    category: MixSlotCategory::Hero,
                    description: "Remplace Mario dans tous les niveaux et sur la carte du monde".into(),
                },
                MixTargetSlot {
                    id: "mount".into(),
                    label: "Monture Yoshi (GFX33 + Bébé Yoshi GFX10 + Palette)".into(),
                    category: MixSlotCategory::Mount,
                    description: "Remplace Yoshi par Donkey Kong ou un autre compagnon".into(),
                },
                MixTargetSlot {
                    id: "enemies".into(),
                    label: "Ennemis SMW (Goombas & Koopas GFX01)".into(),
                    category: MixSlotCategory::Enemies,
                    description: "Remplace les ennemis principaux de GFX01 dans les niveaux SMW".into(),
                },
                MixTargetSlot {
                    id: "items".into(),
                    label: "Objets SMW (Champignons & Pièces GFX00/GFX01)".into(),
                    category: MixSlotCategory::Items,
                    description: "Remplace les power-ups et collectibles de SMW".into(),
                },
                MixTargetSlot {
                    id: "title_story".into(),
                    label: "Thème Titre / Sélection Fichier / Histoire SMW".into(),
                    category: MixSlotCategory::WorldTiles,
                    description: "Personnalise l'écran-titre, le HUD et la boîte d'intro".into(),
                },
                MixTargetSlot {
                    id: "audio_percussion".into(),
                    label: "Percussions Audio BRR SMW (sans désaccorder la musique)".into(),
                    category: MixSlotCategory::Audio,
                    description: "Échantillons BRR non bouclés (!loops) de SMW".into(),
                },
            ]);

            // SMW as Donor!
            let smw_pal = SnesPalette::mario_classic_full_16(Some(&rom.data)).colors_rgba;
            let mario_preview = extract_smw_mario_preview(&rom.data, &smw_pal);
            donor_sources.push(MixDonorSource {
                id: "mario".into(),
                label: "Super Mario (SMW GFX32 — 16x32 authentique)".into(),
                category: MixSlotCategory::Hero,
                compatible_targets: vec![MixSlotCategory::Hero],
                description: "Vrais sprites Super Mario World non écrasés + vraies couleurs Rouge/Bleu".into(),
                preview_rgba: mario_preview,
                preview_w: 16,
                preview_h: 32,
            });
            let epal = default_smw_enemy_palette_rgba();
            if let Some((g01, _, _)) = load_smw_3bpp_sheet(&rom.data, 0x01) {
                donor_sources.push(MixDonorSource {
                    id: "goomba".into(),
                    label: "Goomba / Galoomba (SMW GFX01)".into(),
                    category: MixSlotCategory::Soldiers,
                    compatible_targets: vec![MixSlotCategory::Soldiers, MixSlotCategory::Enemies],
                    description: "Vrai Goomba SMW brun châtaigne avec sourcils froncés".into(),
                    preview_rgba: render_16x16_preview(&read_3bpp_16x16(&g01, 8, 2), &epal),
                    preview_w: 16,
                    preview_h: 16,
                });
                donor_sources.push(MixDonorSource {
                    id: "koopa_shell".into(),
                    label: "Carapace Koopa (SMW GFX01)".into(),
                    category: MixSlotCategory::Enemies,
                    compatible_targets: vec![MixSlotCategory::Soldiers, MixSlotCategory::Enemies],
                    description: "Carapace de Koopa Troopa SMW".into(),
                    preview_rgba: render_16x16_preview(&read_3bpp_16x16(&g01, 10, 0), &epal),
                    preview_w: 16,
                    preview_h: 16,
                });
                donor_sources.push(MixDonorSource {
                    id: "piranha".into(),
                    label: "Plante Piranha (SMW GFX01)".into(),
                    category: MixSlotCategory::Enemies,
                    compatible_targets: vec![MixSlotCategory::Soldiers, MixSlotCategory::Enemies],
                    description: "Plante Piranha carnivore de SMW".into(),
                    preview_rgba: render_16x16_preview(&read_3bpp_16x16(&g01, 12, 2), &epal),
                    preview_w: 16,
                    preview_h: 16,
                });
                donor_sources.push(MixDonorSource {
                    id: "items_pack".into(),
                    label: "Pack Objets SMW (Pièces, Champignons, Bob-ombs, Clés)".into(),
                    category: MixSlotCategory::Items,
                    compatible_targets: vec![MixSlotCategory::Items],
                    description: "Pièces d'or, Super Champignons, Bob-ombs et Clés SMW".into(),
                    preview_rgba: render_16x16_preview(&read_3bpp_16x16(&g01, 8, 6), &epal),
                    preview_w: 16,
                    preview_h: 16,
                });
            }
            if let Some((g02, _, _)) = load_smw_3bpp_sheet(&rom.data, 0x02) {
                donor_sources.push(MixDonorSource {
                    id: "spiny".into(),
                    label: "Spiny / Hériss (SMW GFX02)".into(),
                    category: MixSlotCategory::Enemies,
                    compatible_targets: vec![MixSlotCategory::Soldiers, MixSlotCategory::Enemies],
                    description: "Spiny à piquants de SMW".into(),
                    preview_rgba: render_16x16_preview(&read_3bpp_16x16(&g02, 0, 0), &epal),
                    preview_w: 16,
                    preview_h: 16,
                });
                donor_sources.push(MixDonorSource {
                    id: "bobomb".into(),
                    label: "Bob-omb (SMW GFX02)".into(),
                    category: MixSlotCategory::Items,
                    compatible_targets: vec![MixSlotCategory::Items, MixSlotCategory::Enemies],
                    description: "Bob-omb mécanique avec mèche allumée".into(),
                    preview_rgba: render_16x16_preview(&read_3bpp_16x16(&g02, 10, 4), &epal),
                    preview_w: 16,
                    preview_h: 16,
                });
            }
            if let Some((g06, _, _)) = load_smw_3bpp_sheet(&rom.data, 0x06) {
                donor_sources.push(MixDonorSource {
                    id: "boo".into(),
                    label: "Fantôme Boo (SMW GFX06)".into(),
                    category: MixSlotCategory::Enemies,
                    compatible_targets: vec![MixSlotCategory::Soldiers, MixSlotCategory::Enemies],
                    description: "Fantôme Boo de SMW (idéal donjons & égouts)".into(),
                    preview_rgba: render_16x16_preview(&read_3bpp_16x16(&g06, 12, 0), &epal),
                    preview_w: 16,
                    preview_h: 16,
                });
            }
            if let Some((g03, _, _)) = load_smw_3bpp_sheet(&rom.data, 0x03) {
                donor_sources.push(MixDonorSource {
                    id: "magikoopa".into(),
                    label: "Magikoopa (SMW GFX03)".into(),
                    category: MixSlotCategory::Enemies,
                    compatible_targets: vec![MixSlotCategory::Soldiers, MixSlotCategory::Enemies],
                    description: "Sorcier Magikoopa de SMW".into(),
                    preview_rgba: render_16x16_preview(&read_3bpp_16x16(&g03, 0, 2), &epal),
                    preview_w: 16,
                    preview_h: 16,
                });
            }
            if let Some((g00, _, _)) = load_smw_3bpp_sheet(&rom.data, 0x00) {
                donor_sources.push(MixDonorSource {
                    id: "qblock".into(),
                    label: "Bloc '?' Question Block (SMW GFX00)".into(),
                    category: MixSlotCategory::WorldTiles,
                    compatible_targets: vec![MixSlotCategory::WorldTiles],
                    description: "Bloc '?' iconique de Super Mario World".into(),
                    preview_rgba: render_16x16_preview(&read_3bpp_16x16(&g00, 10, 2), &epal),
                    preview_w: 16,
                    preview_h: 16,
                });
            }
            let boo_prev = load_smw_3bpp_sheet(&rom.data, 0x06)
                .map(|(g06, _, _)| render_16x16_preview(&read_3bpp_16x16(&g06, 12, 0), &epal))
                .unwrap_or_else(|| solid_rgba(220, 220, 235, 16, 16));
            let piranha_prev = load_smw_3bpp_sheet(&rom.data, 0x01)
                .map(|(g01, _, _)| render_16x16_preview(&read_3bpp_16x16(&g01, 12, 2), &epal))
                .unwrap_or_else(|| solid_rgba(60, 190, 80, 16, 16));
            let magi_prev = load_smw_3bpp_sheet(&rom.data, 0x03)
                .map(|(g03, _, _)| render_16x16_preview(&read_3bpp_16x16(&g03, 0, 2), &epal))
                .unwrap_or_else(|| solid_rgba(150, 90, 220, 16, 16));
            donor_sources.push(MixDonorSource {
                id: "sewer_pack".into(),
                label: "Pack Égouts SMW (Boos + Spiny + Goombas)".into(),
                category: MixSlotCategory::Enemies,
                compatible_targets: vec![MixSlotCategory::Enemies],
                description: "Combo Boos (chauves-souris), Spiny (rats) et Goombas (serpents)".into(),
                preview_rgba: boo_prev,
                preview_w: 16,
                preview_h: 16,
            });
            donor_sources.push(MixDonorSource {
                id: "overworld_pack".into(),
                label: "Pack Plaine SMW (Plantes Piranha + Spiny)".into(),
                category: MixSlotCategory::Enemies,
                compatible_targets: vec![MixSlotCategory::Enemies],
                description: "Combo Plantes Piranha et Spiny sur l'Overworld".into(),
                preview_rgba: piranha_prev,
                preview_w: 16,
                preview_h: 16,
            });
            donor_sources.push(MixDonorSource {
                id: "dungeon_pack".into(),
                label: "Pack Donjon SMW (Boos + Magikoopas)".into(),
                category: MixSlotCategory::Enemies,
                compatible_targets: vec![MixSlotCategory::Enemies],
                description: "Combo Boos et Magikoopas dans les donjons".into(),
                preview_rgba: magi_prev,
                preview_w: 16,
                preview_h: 16,
            });
            donor_sources.push(MixDonorSource {
                id: "percussion".into(),
                label: "Percussions BRR Super Mario World".into(),
                category: MixSlotCategory::Audio,
                compatible_targets: vec![MixSlotCategory::Audio],
                description: "Échantillons BRR non bouclés de SMW".into(),
                preview_rgba: audio_wave_rgba(245, 180, 50),
                preview_w: 16,
                preview_h: 16,
            });
        }
        RecognizedGame::DonkeyKongCountry => {
            target_slots.extend([
                MixTargetSlot {
                    id: "hero".into(),
                    label: "Héros Donkey Kong (Frames OAM 4bpp + Palette 0x39A068)".into(),
                    category: MixSlotCategory::Hero,
                    description: "Remplace Donkey Kong et sa palette par un autre héros (Mario/Link)".into(),
                },
                MixTargetSlot {
                    id: "palette".into(),
                    label: "Palette Principale DK (0x39A068 BGR555)".into(),
                    category: MixSlotCategory::Palette,
                    description: "Remplace la palette BGR555 de Donkey Kong".into(),
                },
                MixTargetSlot {
                    id: "audio_percussion".into(),
                    label: "Percussions Audio BRR DKC".into(),
                    category: MixSlotCategory::Audio,
                    description: "Échantillons BRR percussifs non bouclés de DKC".into(),
                },
            ]);

            let dk_pal = default_dk_palette_rgba();
            donor_sources.push(MixDonorSource {
                id: "dk".into(),
                label: "Donkey Kong (DKC — Héros 16x32 & Monture)".into(),
                category: MixSlotCategory::Hero,
                compatible_targets: vec![MixSlotCategory::Hero, MixSlotCategory::Mount],
                description: "Donkey Kong (fourrure brune, cravate rouge, museau tan) jouable en Héros ou Monture".into(),
                preview_rgba: render_16x32_preview(
                    &crate::ai_sprites::DK_HEAD_NORM,
                    &crate::ai_sprites::DK_BODY_ST,
                    &dk_pal,
                ),
                preview_w: 16,
                preview_h: 32,
            });
            donor_sources.push(MixDonorSource {
                id: "baby_dk".into(),
                label: "Mini Donkey Kong (DKC 16x16)".into(),
                category: MixSlotCategory::Enemies,
                compatible_targets: vec![MixSlotCategory::Soldiers, MixSlotCategory::Enemies],
                description: "Mini Donkey Kong 16x16 utilisable comme ennemi ou garde".into(),
                preview_rgba: render_16x16_preview(&crate::ai_sprites::DK_BABY_DK, &dk_pal),
                preview_w: 16,
                preview_h: 16,
            });
            donor_sources.push(MixDonorSource {
                id: "percussion".into(),
                label: "Percussions Jungle BRR Donkey Kong Country".into(),
                category: MixSlotCategory::Audio,
                compatible_targets: vec![MixSlotCategory::Audio],
                description: "Bongos et percussions haute fidélité BRR de David Wise (DKC)".into(),
                preview_rgba: audio_wave_rgba(235, 130, 50),
                preview_w: 16,
                preview_h: 16,
            });
        }
        RecognizedGame::Unknown => {}
    }

    // ALWAYS expose universal ROM-agnostic slots & sources (up to 16 GFX, 12 Palettes, 12 Audio)
    // so ANY ROM (known or 100% unknown!) can be mixed with ANY OTHER ROM!
    for (idx, g) in universal_catalog.graphics.iter().take(16).enumerate() {
        target_slots.push(MixTargetSlot {
            id: format!("gfx:{}", idx),
            label: format!(
                "GFX #{} — {} ({}, {} tuiles)",
                idx,
                g.snes_addr,
                g.format.short_label(),
                g.tile_count
            ),
            category: MixSlotCategory::RawGfx,
            description: format!("{} ({})", g.name, g.compression.label_fr()),
        });
        donor_sources.push(MixDonorSource {
            id: format!("gfx:{}", idx),
            label: format!(
                "GFX #{} ({}, {} tuiles)",
                idx,
                g.format.short_label(),
                g.tile_count
            ),
            category: MixSlotCategory::RawGfx,
            compatible_targets: vec![MixSlotCategory::RawGfx],
            description: format!("Planche universelle @ {}", g.snes_addr),
            preview_rgba: solid_rgba(70, 110, 170, 16, 16),
            preview_w: 16,
            preview_h: 16,
        });
    }

    for (idx, p) in universal_catalog.palettes.iter().take(12).enumerate() {
        target_slots.push(MixTargetSlot {
            id: format!("pal:{}", idx),
            label: format!("Palette #{} — {} ({} coul.)", idx, p.snes_addr, p.color_count),
            category: MixSlotCategory::RawPalette,
            description: p.name.clone(),
        });
        let mut sw = vec![0u8; 16 * 16 * 4];
        for y in 0..16 {
            for x in 0..16 {
                let ci = (x * p.colors_rgba.len().max(1)) / 16;
                let col = p.colors_rgba.get(ci).copied().unwrap_or([40, 40, 40, 255]);
                let dst = (y * 16 + x) * 4;
                sw[dst..dst + 4].copy_from_slice(&col);
            }
        }
        donor_sources.push(MixDonorSource {
            id: format!("pal:{}", idx),
            label: format!("Palette #{} ({} coul. @ {})", idx, p.color_count, p.snes_addr),
            category: MixSlotCategory::RawPalette,
            compatible_targets: vec![MixSlotCategory::RawPalette, MixSlotCategory::Palette],
            description: p.name.clone(),
            preview_rgba: sw,
            preview_w: 16,
            preview_h: 16,
        });
    }

    for (idx, a) in universal_catalog.audio_samples.iter().take(12).enumerate() {
        target_slots.push(MixTargetSlot {
            id: format!("audio:{}", idx),
            label: format!(
                "BRR #{} — {} ({} o, {} ms)",
                idx, a.snes_addr, a.byte_len, a.duration_ms_16khz
            ),
            category: MixSlotCategory::RawAudio,
            description: format!("Loop: {}", a.loops),
        });
        donor_sources.push(MixDonorSource {
            id: format!("audio:{}", idx),
            label: format!("BRR #{} ({} ms, Loop:{})", idx, a.duration_ms_16khz, a.loops),
            category: MixSlotCategory::RawAudio,
            compatible_targets: vec![MixSlotCategory::RawAudio],
            description: format!("Sample BRR @ {}", a.snes_addr),
            preview_rgba: audio_wave_rgba(56, 189, 248),
            preview_w: 16,
            preview_h: 16,
        });
    }

    GameMixProfile {
        alias,
        rom_path,
        title: rom.canonical_title(),
        recognized,
        mapping: rom.header.mapping.as_str().to_string(),
        size_bytes: rom.data.len(),
        target_slots,
        donor_sources,
        universal_catalog,
    }
}

fn read_4bpp_alttp_cell(zelda: &[u8], mx: usize, my: usize) -> [[u8; 16]; 16] {
    let mut px = [[0u8; 16]; 16];
    let base = 0x080000 + (my * 32 + mx * 2) * 32;
    let coords = [base, base + 32, base + 512, base + 544];
    for (q, &src) in coords.iter().enumerate() {
        if src + 32 > zelda.len() {
            continue;
        }
        let qx = (q & 1) * 8;
        let qy = (q >> 1) * 8;
        for y in 0..8 {
            let b0 = zelda[src + y * 2];
            let b1 = zelda[src + y * 2 + 1];
            let b2 = zelda[src + 16 + y * 2];
            let b3 = zelda[src + 16 + y * 2 + 1];
            for x in 0..8 {
                let sh = 7 - x;
                px[qy + y][qx + x] = ((b0 >> sh) & 1)
                    | (((b1 >> sh) & 1) << 1)
                    | (((b2 >> sh) & 1) << 2)
                    | (((b3 >> sh) & 1) << 3);
            }
        }
    }
    px
}

fn alttp_sprite_pal_rgba(zelda: &[u8], off: usize) -> Vec<[u8; 4]> {
    let mut cols = vec![[0, 0, 0, 0]];
    for i in 0..7 {
        if off + i * 2 + 1 < zelda.len() {
            let w = u16::from_le_bytes([zelda[off + i * 2], zelda[off + i * 2 + 1]]);
            let r = (((w & 31) << 3) | ((w & 31) >> 2)) as u8;
            let g = ((((w >> 5) & 31) << 3) | (((w >> 5) & 31) >> 2)) as u8;
            let b = ((((w >> 10) & 31) << 3) | (((w >> 10) & 31) >> 2)) as u8;
            cols.push([r, g, b, 255]);
        }
    }
    cols
}

fn extract_smw_mario_preview(smw: &[u8], pal: &[[u8; 4]]) -> Vec<u8> {
    let Some((g32, _)) = decompress_nintendo_lz2_with_endian(smw, 0x040000, 32768, true) else {
        return solid_rgba(220, 40, 40, 16, 32);
    };
    // Pose 0x0F (Front Super Mario: va=0x84, vb=0x07)
    let read_g32 = |val: u8| -> [[u8; 16]; 16] {
        let off = (((val as usize) & 0x08) << 11) | (((val as usize) & 0xF7) << 6);
        let coords = [off, off + 32, off + 512, off + 544];
        let mut px = [[0u8; 16]; 16];
        for (q, &src) in coords.iter().enumerate() {
            if src + 32 > g32.len() {
                continue;
            }
            let qx = (q & 1) * 8;
            let qy = (q >> 1) * 8;
            for y in 0..8 {
                let b0 = g32[src + y * 2];
                let b1 = g32[src + y * 2 + 1];
                let b2 = g32[src + 16 + y * 2];
                let b3 = g32[src + 16 + y * 2 + 1];
                for x in 0..8 {
                    let sh = 7 - x;
                    px[qy + y][qx + x] = ((b0 >> sh) & 1)
                        | (((b1 >> sh) & 1) << 1)
                        | (((b2 >> sh) & 1) << 2)
                        | (((b3 >> sh) & 1) << 3);
                }
            }
        }
        px
    };
    let top = read_g32(0x84);
    let bot = read_g32(0x07);
    render_16x32_preview(&top, &bot, pal)
}

// =============================================================================
// CURATED MULTI-GAME PRESETS & DETERMINISTIC CHAOS RANDOMIZER
// =============================================================================

pub fn suggest_presets(base: &GameMixProfile, donors: &[GameMixProfile]) -> Vec<MixPreset> {
    let mut presets = Vec::new();
    let find_donor = |game: RecognizedGame| donors.iter().find(|d| d.recognized == game);

    match base.recognized {
        RecognizedGame::ZeldaAlttp => {
            if let Some(smw) = find_donor(RecognizedGame::SuperMarioWorld) {
                let mut rules = vec![
                    MixRule { target_slot: "hero".into(), donor_alias: smw.alias.clone(), donor_source: "mario".into() },
                    MixRule { target_slot: "soldiers".into(), donor_alias: smw.alias.clone(), donor_source: "goomba".into() },
                    MixRule { target_slot: "elite_knights".into(), donor_alias: smw.alias.clone(), donor_source: "spiny".into() },
                    MixRule { target_slot: "castle_guards".into(), donor_alias: smw.alias.clone(), donor_source: "goomba".into() },
                    MixRule { target_slot: "sewer_enemies".into(), donor_alias: smw.alias.clone(), donor_source: "sewer_pack".into() },
                    MixRule { target_slot: "overworld_enemies".into(), donor_alias: smw.alias.clone(), donor_source: "overworld_pack".into() },
                    MixRule { target_slot: "dungeon_enemies".into(), donor_alias: smw.alias.clone(), donor_source: "dungeon_pack".into() },
                    MixRule { target_slot: "items".into(), donor_alias: smw.alias.clone(), donor_source: "items_pack".into() },
                    MixRule { target_slot: "world_rocks".into(), donor_alias: smw.alias.clone(), donor_source: "qblock".into() },
                ];
                if let Some(dkc) = find_donor(RecognizedGame::DonkeyKongCountry) {
                    rules.push(MixRule {
                        target_slot: "audio_percussion".into(),
                        donor_alias: dkc.alias.clone(),
                        donor_source: "percussion".into(),
                    });
                }
                presets.push(MixPreset {
                    id: "mario_hyrule_complete".into(),
                    name: "Super Mario Hyrule (Complet)".into(),
                    description: "Mario 16x32 (SMW) + Soldats Goombas + Spiny + Boos + Piranha + Pièces/Champignons/Bob-ombs + Percussions DKC".into(),
                    custom_title: "SUPER MARIO HYRULE".into(),
                    rules,
                });
            }
            if let Some(dkc) = find_donor(RecognizedGame::DonkeyKongCountry) {
                let mut rules = vec![
                    MixRule { target_slot: "hero".into(), donor_alias: dkc.alias.clone(), donor_source: "dk".into() },
                    MixRule { target_slot: "audio_percussion".into(), donor_alias: dkc.alias.clone(), donor_source: "percussion".into() },
                ];
                if let Some(smw) = find_donor(RecognizedGame::SuperMarioWorld) {
                    rules.push(MixRule { target_slot: "soldiers".into(), donor_alias: smw.alias.clone(), donor_source: "goomba".into() });
                    rules.push(MixRule { target_slot: "elite_knights".into(), donor_alias: smw.alias.clone(), donor_source: "spiny".into() });
                    rules.push(MixRule { target_slot: "sewer_enemies".into(), donor_alias: smw.alias.clone(), donor_source: "sewer_pack".into() });
                    rules.push(MixRule { target_slot: "items".into(), donor_alias: smw.alias.clone(), donor_source: "items_pack".into() });
                } else {
                    rules.push(MixRule { target_slot: "soldiers".into(), donor_alias: dkc.alias.clone(), donor_source: "baby_dk".into() });
                }
                presets.push(MixPreset {
                    id: "donkey_hyrule_3games".into(),
                    name: "Donkey Kong dans Hyrule (Crossover 3 Jeux !)".into(),
                    description: "Héros Donkey Kong (DKC) + Soldats Goombas & Objets (SMW) + Percussions Jungle (DKC) dans le monde de Zelda !".into(),
                    custom_title: "DONKEY HYRULE WORLD".into(),
                    rules,
                });
            }
        }
        RecognizedGame::SuperMarioWorld => {
            if let Some(zelda) = find_donor(RecognizedGame::ZeldaAlttp) {
                let mut rules = vec![
                    MixRule { target_slot: "hero".into(), donor_alias: zelda.alias.clone(), donor_source: "link".into() },
                    MixRule { target_slot: "title_story".into(), donor_alias: zelda.alias.clone(), donor_source: "hyrule_theme".into() },
                ];
                if let Some(dkc) = find_donor(RecognizedGame::DonkeyKongCountry) {
                    rules.push(MixRule { target_slot: "mount".into(), donor_alias: dkc.alias.clone(), donor_source: "dk".into() });
                    rules.push(MixRule { target_slot: "audio_percussion".into(), donor_alias: dkc.alias.clone(), donor_source: "percussion".into() });
                }
                presets.push(MixPreset {
                    id: "super_link_dk_world".into(),
                    name: "Super Link & DK World (Crossover 3 Jeux !)".into(),
                    description: "Héros Link (Zelda) chevauchant Donkey Kong (DKC) + Percussions DKC dans Super Mario World !".into(),
                    custom_title: "SUPER LINK WORLD".into(),
                    rules,
                });

                let mut inv_rules = vec![
                    MixRule { target_slot: "hero".into(), donor_alias: zelda.alias.clone(), donor_source: "link".into() },
                    MixRule { target_slot: "enemies".into(), donor_alias: zelda.alias.clone(), donor_source: "zelda_soldier".into() },
                    MixRule { target_slot: "items".into(), donor_alias: zelda.alias.clone(), donor_source: "zelda_items_pack".into() },
                ];
                if let Some(dkc) = find_donor(RecognizedGame::DonkeyKongCountry) {
                    inv_rules.push(MixRule { target_slot: "mount".into(), donor_alias: dkc.alias.clone(), donor_source: "dk".into() });
                }
                presets.push(MixPreset {
                    id: "hyrule_invasion_smw".into(),
                    name: "Invasion d'Hyrule dans Mario World".into(),
                    description: "Link + Ennemis Soldats d'Hyrule + Rubis & Cœurs + Monture Donkey Kong dans SMW !".into(),
                    custom_title: "HYRULE MARIO WORLD".into(),
                    rules: inv_rules,
                });
            }
            if let Some(dkc) = find_donor(RecognizedGame::DonkeyKongCountry) {
                presets.push(MixPreset {
                    id: "dk_mario_world".into(),
                    name: "Donkey Kong World (DK Jouable dans SMW !)".into(),
                    description: "Joue Donkey Kong directement comme héros dans Super Mario World + Percussions Jungle DKC !".into(),
                    custom_title: "DONKEY KONG WORLD".into(),
                    rules: vec![
                        MixRule { target_slot: "hero".into(), donor_alias: dkc.alias.clone(), donor_source: "dk".into() },
                        MixRule { target_slot: "audio_percussion".into(), donor_alias: dkc.alias.clone(), donor_source: "percussion".into() },
                    ],
                });
            }
        }
        RecognizedGame::DonkeyKongCountry => {
            if let Some(smw) = find_donor(RecognizedGame::SuperMarioWorld) {
                let mut rules = vec![
                    MixRule { target_slot: "hero".into(), donor_alias: smw.alias.clone(), donor_source: "mario".into() },
                ];
                if let Some(zelda) = find_donor(RecognizedGame::ZeldaAlttp) {
                    rules.push(MixRule { target_slot: "audio_percussion".into(), donor_alias: zelda.alias.clone(), donor_source: "percussion".into() });
                }
                presets.push(MixPreset {
                    id: "mario_in_dkc".into(),
                    name: "Mario dans Donkey Kong Country".into(),
                    description: "Injecte les tuiles 4bpp et la palette de Mario (SMW) + percussions Zelda dans DKC 4 Mo HiROM !".into(),
                    custom_title: "MARIO KONG COUNTRY".into(),
                    rules,
                });
            }
            if let Some(zelda) = find_donor(RecognizedGame::ZeldaAlttp) {
                presets.push(MixPreset {
                    id: "link_in_dkc".into(),
                    name: "Link dans Donkey Kong Country".into(),
                    description: "Injecte les tuiles 4bpp et la palette Tunique Verte de Link (Zelda) dans DKC 4 Mo HiROM !".into(),
                    custom_title: "LINK KONG COUNTRY".into(),
                    rules: vec![
                        MixRule { target_slot: "hero".into(), donor_alias: zelda.alias.clone(), donor_source: "link".into() },
                    ],
                });
            }
        }
        RecognizedGame::Unknown => {}
    }

    // Universal Multi-ROM Auto-Mix preset (works for ANY combination of known or unknown ROMs!)
    if !donors.is_empty() {
        let chaos = generate_chaos_recipe(base, donors, 1337);
        if !chaos.rules.is_empty() {
            presets.push(MixPreset {
                id: "chaos_multi_rom".into(),
                name: "Chaos Multi-Jeux Universel (Auto-Mix)".into(),
                description: format!(
                    "Mélange déterministe sûr depuis {} ROM(s) donneuse(s) vers {}",
                    donors.len(),
                    base.title
                ),
                custom_title: chaos.custom_title.unwrap_or_else(|| "OMNI MIXER SNES".into()),
                rules: chaos.rules,
            });
        }
    }

    presets
}

/// Deterministic xorshift64 PRNG for reproducible Chaos Randomizer mixes
struct SimpleRng(u64);
impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn pick(&mut self, len: usize) -> usize {
        (self.next_u64() as usize) % len.max(1)
    }
}

pub fn generate_chaos_recipe(
    base: &GameMixProfile,
    donors: &[GameMixProfile],
    seed: u64,
) -> MixRecipe {
    let mut rng = SimpleRng::new(seed);
    let mut rules = Vec::new();

    // 1. Try semantic slots first (excluding raw slots)
    let semantic_targets: Vec<&MixTargetSlot> = base
        .target_slots
        .iter()
        .filter(|s| {
            !matches!(
                s.category,
                MixSlotCategory::RawGfx | MixSlotCategory::RawPalette | MixSlotCategory::RawAudio
            )
        })
        .collect();

    for slot in semantic_targets {
        let mut candidates: Vec<(&str, &str)> = Vec::new();
        for d in donors {
            for src in &d.donor_sources {
                if src.compatible_targets.contains(&slot.category) {
                    candidates.push((d.alias.as_str(), src.id.as_str()));
                }
            }
        }
        if !candidates.is_empty() {
            let (da, ds) = candidates[rng.pick(candidates.len())];
            rules.push(MixRule {
                target_slot: slot.id.clone(),
                donor_alias: da.to_string(),
                donor_source: ds.to_string(),
            });
        }
    }

    // 2. If base is an unknown ROM (no semantic slots matched), mix universal GFX/Palettes/Audio safely!
    if rules.is_empty() && !donors.is_empty() {
        for i in 0..base.universal_catalog.graphics.len().min(3) {
            let d = &donors[rng.pick(donors.len())];
            if !d.universal_catalog.graphics.is_empty() {
                let di = rng.pick(d.universal_catalog.graphics.len().min(12));
                rules.push(MixRule {
                    target_slot: format!("gfx:{}", i),
                    donor_alias: d.alias.clone(),
                    donor_source: format!("gfx:{}", di),
                });
            }
        }
        for i in 0..base.universal_catalog.palettes.len().min(3) {
            let d = &donors[rng.pick(donors.len())];
            if !d.universal_catalog.palettes.is_empty() {
                let di = rng.pick(d.universal_catalog.palettes.len().min(10));
                rules.push(MixRule {
                    target_slot: format!("pal:{}", i),
                    donor_alias: d.alias.clone(),
                    donor_source: format!("pal:{}", di),
                });
            }
        }
        for i in 0..base.universal_catalog.audio_samples.len().min(2) {
            let d = &donors[rng.pick(donors.len())];
            if !d.universal_catalog.audio_samples.is_empty() {
                let di = rng.pick(d.universal_catalog.audio_samples.len().min(10));
                rules.push(MixRule {
                    target_slot: format!("audio:{}", i),
                    donor_alias: d.alias.clone(),
                    donor_source: format!("audio:{}", di),
                });
            }
        }
    }

    MixRecipe {
        name: format!("Chaos Mix #{}", seed),
        custom_title: Some(format!("OMNI MIX #{}", seed % 10000)),
        base_rom: base.rom_path.clone(),
        donors: donors
            .iter()
            .map(|d| (d.alias.clone(), d.rom_path.clone()))
            .collect(),
        rules,
    }
}

// =============================================================================
// EXECUTION ENGINE: APPLY ANY N-ROM MIX RECIPE TO ANY BASE ROM!
// =============================================================================

/// Execute a complete multi-game `MixRecipe` on `base_rom` using any number of loaded `donor_roms`.
pub fn execute_mix_recipe(
    base_rom: &mut SnesRom,
    donor_roms: &HashMap<String, SnesRom>,
    recipe: &MixRecipe,
    session: &mut SwapSession,
) -> Result<MixReport, RomError> {
    let t0 = Instant::now();
    let base_title = base_rom.canonical_title();
    let base_game = base_rom.identify_game();

    let mut total_ops = 0usize;
    let mut applied_rules = Vec::new();
    let mut warnings = Vec::new();

    // Precompute universal catalogs lazily only if a universal rule is requested
    let mut base_cat_opt: Option<UniversalRomCatalog> = None;
    let mut donor_cats: HashMap<String, UniversalRomCatalog> = HashMap::new();

    // Free-space cursor for Zelda ALTTP compressed sheet repointing ($0C:EF46..$0C:FFFF)
    let mut zelda_free_cursor = 0x066F46usize;
    let zelda_free_end = 0x068000usize;

    for rule in &recipe.rules {
        let Some(donor_rom) = donor_roms.get(&rule.donor_alias) else {
            warnings.push(format!(
                "Donneuse '{}' introuvable pour le slot '{}'",
                rule.donor_alias, rule.target_slot
            ));
            continue;
        };

        // 1. Universal raw slots (`gfx:N`, `pal:N`, `audio:N`) work on EVERY SNES ROM!
        if rule.target_slot.starts_with("gfx:")
            || rule.target_slot.starts_with("pal:")
            || rule.target_slot.starts_with("audio:")
        {
            let bcat = base_cat_opt.get_or_insert_with(|| discover_universal_catalog(base_rom));
            let dcat = donor_cats
                .entry(rule.donor_alias.clone())
                .or_insert_with(|| discover_universal_catalog(donor_rom));
            match apply_universal_single_rule(base_rom, donor_rom, bcat, dcat, rule, session) {
                Ok(n) if n > 0 => {
                    total_ops += n;
                    applied_rules.push(format!(
                        "{} ← {}:{} ({} op)",
                        rule.target_slot, rule.donor_alias, rule.donor_source, n
                    ));
                }
                Ok(_) => warnings.push(format!(
                    "Règle universelle ignorée (budget/format incompatible) : {} ← {}:{}",
                    rule.target_slot, rule.donor_alias, rule.donor_source
                )),
                Err(e) => warnings.push(format!(
                    "Erreur règle {} ← {}:{}: {}",
                    rule.target_slot, rule.donor_alias, rule.donor_source, e
                )),
            }
            continue;
        }

        // 2. Semantic game-specific slots (high-fidelity, playable, hardware-verified!)
        let res = match base_game {
            RecognizedGame::ZeldaAlttp => apply_rule_to_zelda(
                base_rom,
                donor_rom,
                rule,
                session,
                &mut zelda_free_cursor,
                zelda_free_end,
            ),
            RecognizedGame::SuperMarioWorld => {
                apply_rule_to_smw(base_rom, donor_rom, rule, session)
            }
            RecognizedGame::DonkeyKongCountry => {
                apply_rule_to_dkc(base_rom, donor_rom, rule, session)
            }
            RecognizedGame::Unknown => {
                warnings.push(format!(
                    "Slot sémantique '{}' non supporté sur ROM inconnue (utilisez gfx:N / pal:N / audio:N)",
                    rule.target_slot
                ));
                Ok(0)
            }
        };

        match res {
            Ok(n) if n > 0 => {
                total_ops += n;
                applied_rules.push(format!(
                    "{} ← {}:{} ({} op)",
                    rule.target_slot, rule.donor_alias, rule.donor_source, n
                ));
            }
            Ok(_) => warnings.push(format!(
                "Aucune opération appliquée pour {} ← {}:{}",
                rule.target_slot, rule.donor_alias, rule.donor_source
            )),
            Err(e) => warnings.push(format!(
                "Erreur {} ← {}:{}: {}",
                rule.target_slot, rule.donor_alias, rule.donor_source, e
            )),
        }
    }

    // Apply custom SNES cartridge internal title (21 ASCII chars padded with spaces)
    let output_title = if let Some(ref custom) = recipe.custom_title {
        let mut clean: String = custom
            .chars()
            .filter(|c| c.is_ascii_graphic() || *c == ' ')
            .map(|c| c.to_ascii_uppercase())
            .take(21)
            .collect();
        while clean.len() < 21 {
            clean.push(' ');
        }
        let header_off = base_rom.header.mapping.header_base_offset();
        if header_off + 21 <= base_rom.data.len() {
            session.swap_custom_bytes(
                base_rom,
                header_off,
                header_off,
                clean.as_bytes().to_vec(),
                format!("Titre Cartouche SNES : '{}'", clean.trim()),
                SwapKind::CodePatch { byte_count: 21 },
            )?;
            total_ops += 1;
        }
        clean.trim().to_string()
    } else {
        base_rom.header.title.clone()
    };

    base_rom.fix_checksum();

    Ok(MixReport {
        base_title,
        output_title,
        operations_applied: total_ops,
        applied_rules,
        warnings,
        checksum: base_rom.header.checksum,
        complement: base_rom.header.checksum_complement,
        checksum_valid: base_rom.header.is_checksum_valid(),
        crc32: base_rom.crc32,
        elapsed_ms: t0.elapsed().as_millis(),
    })
}

// =============================================================================
// TARGET 1: ZELDA ALTTP INJECTORS (HERO: MARIO / DK / ANY, ENEMIES, ITEMS, ROCKS, AUDIO)
// =============================================================================

fn apply_rule_to_zelda(
    rom_zelda: &mut SnesRom,
    donor: &SnesRom,
    rule: &MixRule,
    session: &mut SwapSession,
    free_cursor: &mut usize,
    free_end: usize,
) -> Result<usize, RomError> {
    let mut ops = 0usize;

    match rule.target_slot.as_str() {
        "hero" => {
            if rule.donor_source == "mario" && donor.identify_game() == RecognizedGame::SuperMarioWorld {
                // Inject Mario hero + Bank $0D OAM + Palettes (without overwriting other sheets yet;
                // use dedicated hero-only injector so user rules control soldiers/items independently!)
                ops += inject_smw_mario_hero_into_zelda(rom_zelda, donor, session)?;
            } else if rule.donor_source == "dk" {
                ops += inject_dk_hero_into_zelda(rom_zelda, session)?;
            } else if rule.donor_source == "link" && donor.identify_game() == RecognizedGame::ZeldaAlttp {
                // Restore vanilla Link from donor Zelda ROM
                let bank = donor.data[0x080000..0x087000].to_vec();
                session.swap_custom_bytes(
                    rom_zelda,
                    0x080000,
                    0x080000,
                    bank,
                    "Héros Link (Restauré depuis Zelda ALTTP)".into(),
                    SwapKind::GraphicsTiles {
                        format: TileFormat::Bpp4,
                        tile_count: 896,
                        smart_remapped: false,
                    },
                )?;
                ops += 1;
            }
        }
        "soldiers" => {
            if let Some((sp1, sp2)) = extract_donor_16x16_for_zelda(donor, &rule.donor_source, true) {
                let sp1_r = flip_h_16x16(&sp1);
                let blank16 = [[0u8; 16]; 16];
                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0xBA,
                    &format!("Soldats d'Hyrule (Sheet 0xBA) ← {}:{}", rule.donor_alias, rule.donor_source),
                    free_cursor,
                    free_end,
                    &|raw| {
                        // Single-OAM Head slots (never doubled, never blinking!)
                        write_3bpp_16x16(raw, 0, 0, &sp1);
                        write_3bpp_16x16(raw, 2, 0, &sp2);
                        write_3bpp_16x16(raw, 4, 0, &sp1_r);
                        for &(tx, ty) in &[(6, 0), (8, 0), (10, 0), (14, 0), (0, 2), (2, 2), (4, 2), (6, 2), (8, 2)] {
                            write_3bpp_16x16(raw, tx, ty, &blank16);
                        }
                    },
                )? {
                    ops += 1;
                }
            }
        }
        "elite_knights" => {
            if let Some((sp1, sp2)) = extract_donor_16x16_for_zelda(donor, &rule.donor_source, true) {
                let blank16 = [[0u8; 16]; 16];
                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0xB7,
                    &format!("Chevaliers d'Élite (Sheet 0xB7) ← {}:{}", rule.donor_alias, rule.donor_source),
                    free_cursor,
                    free_end,
                    &|raw| {
                        write_3bpp_16x16(raw, 0, 0, &sp1);
                        write_3bpp_16x16(raw, 2, 0, &sp2);
                        write_3bpp_16x16(raw, 4, 0, &sp1);
                        for &(tx, ty) in &[(6, 0), (8, 0), (10, 0), (12, 0), (0, 2), (2, 2), (4, 2), (6, 2), (12, 2)] {
                            write_3bpp_16x16(raw, tx, ty, &blank16);
                        }
                    },
                )? {
                    ops += 1;
                }
            }
        }
        "castle_guards" => {
            if let Some((sp1, sp2)) = extract_donor_16x16_for_zelda(donor, &rule.donor_source, true) {
                let sp1_r = flip_h_16x16(&sp1);
                let sp2_r = flip_h_16x16(&sp2);
                let blank16 = [[0u8; 16]; 16];
                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0x8C,
                    &format!("Gardes du Château (Sheet 0x8C) ← {}:{}", rule.donor_alias, rule.donor_source),
                    free_cursor,
                    free_end,
                    &|raw| {
                        for &(tx, ty) in &[(8, 0), (10, 0), (12, 0), (14, 0)] {
                            write_3bpp_16x16(raw, tx, ty, &blank16);
                        }
                        write_3bpp_16x16(raw, 0, 0, &sp1);
                        write_3bpp_16x16(raw, 2, 0, &sp2);
                        write_3bpp_16x16(raw, 4, 0, &sp1);
                        write_3bpp_16x16(raw, 6, 0, &sp1_r);
                        write_3bpp_16x16(raw, 0, 2, &sp2);
                        write_3bpp_16x16(raw, 2, 2, &sp2_r);
                        write_3bpp_16x16(raw, 4, 2, &sp1);
                        write_3bpp_16x16(raw, 6, 2, &sp2);
                    },
                )? {
                    ops += 1;
                }
            }
        }
        "sewer_enemies" => {
            let (a1, a2, b1, b2, c1, c2) = if rule.donor_source == "sewer_pack" {
                let (boo1, boo2) = extract_donor_16x16_for_zelda(donor, "boo", false)
                    .unwrap_or(([[0; 16]; 16], [[0; 16]; 16]));
                let (sp1, sp2) = extract_donor_16x16_for_zelda(donor, "spiny", false)
                    .unwrap_or((boo1, boo2));
                let (g1, g2) = extract_donor_16x16_for_zelda(donor, "goomba", true)
                    .unwrap_or((sp1, sp2));
                (boo1, boo2, sp1, sp2, g1, g2)
            } else if let Some((s1, s2)) = extract_donor_16x16_for_zelda(donor, &rule.donor_source, false) {
                (s1, s2, s1, s2, s1, s2)
            } else {
                return Ok(0);
            };
            if patch_alttp_comp_sheet_helper(
                rom_zelda,
                session,
                0x8D,
                &format!("Égouts du Château (Sheet 0x8D) ← {}:{}", rule.donor_alias, rule.donor_source),
                free_cursor,
                free_end,
                &|raw| {
                    write_3bpp_16x16(raw, 0, 0, &a1);
                    write_3bpp_16x16(raw, 2, 0, &a2);
                    write_3bpp_16x16(raw, 4, 0, &a1);
                    write_3bpp_16x16(raw, 8, 0, &b1);
                    write_3bpp_16x16(raw, 10, 0, &b2);
                    write_3bpp_16x16(raw, 12, 0, &b1);
                    write_3bpp_16x16(raw, 14, 0, &b2);
                    write_3bpp_16x16(raw, 2, 2, &b1);
                    write_3bpp_16x16(raw, 6, 2, &c1);
                    write_3bpp_16x16(raw, 8, 2, &c2);
                    write_3bpp_16x16(raw, 10, 2, &flip_h_16x16(&c1));
                },
            )? {
                ops += 1;
            }
        }
        "overworld_enemies" => {
            let (p1, p2, s1, s2) = if rule.donor_source == "overworld_pack" {
                let (pr1, pr2) = extract_donor_16x16_for_zelda(donor, "piranha", false)
                    .unwrap_or(([[0; 16]; 16], [[0; 16]; 16]));
                let (sp1, sp2) = extract_donor_16x16_for_zelda(donor, "spiny", false)
                    .unwrap_or((pr1, pr2));
                (pr1, pr2, sp1, sp2)
            } else if let Some((x1, x2)) = extract_donor_16x16_for_zelda(donor, &rule.donor_source, false) {
                (x1, x2, x1, x2)
            } else {
                return Ok(0);
            };
            if patch_alttp_comp_sheet_helper(
                rom_zelda,
                session,
                0x7F,
                &format!("Overworld Octoroks (Sheet 0x7F) ← {}:{}", rule.donor_alias, rule.donor_source),
                free_cursor,
                free_end,
                &|raw| {
                    write_3bpp_16x16(raw, 14, 0, &s1);
                },
            )? {
                ops += 1;
            }
            if patch_alttp_comp_sheet_helper(
                rom_zelda,
                session,
                0x82,
                &format!("Overworld Ennemis (Sheet 0x82) ← {}:{}", rule.donor_alias, rule.donor_source),
                free_cursor,
                free_end,
                &|raw| {
                    write_3bpp_16x16(raw, 10, 0, &p1);
                    write_3bpp_16x16(raw, 12, 0, &p2);
                    write_3bpp_16x16(raw, 14, 0, &p1);
                    write_3bpp_16x16(raw, 6, 2, &s1);
                    write_3bpp_16x16(raw, 8, 2, &s2);
                    write_3bpp_16x16(raw, 12, 2, &s1);
                },
            )? {
                ops += 1;
            }
        }
        "dungeon_enemies" => {
            let (b1, b2, m1, m2) = if rule.donor_source == "dungeon_pack" {
                let (bo1, bo2) = extract_donor_16x16_for_zelda(donor, "boo", false)
                    .unwrap_or(([[0; 16]; 16], [[0; 16]; 16]));
                let (mk1, mk2) = extract_donor_16x16_for_zelda(donor, "magikoopa", false)
                    .unwrap_or((bo1, bo2));
                (bo1, bo2, mk1, mk2)
            } else if let Some((x1, x2)) = extract_donor_16x16_for_zelda(donor, &rule.donor_source, false) {
                (x1, x2, x1, x2)
            } else {
                return Ok(0);
            };
            if patch_alttp_comp_sheet_helper(
                rom_zelda,
                session,
                0x90,
                &format!("Squelettes Stalfos (Sheet 0x90) ← {}:{}", rule.donor_alias, rule.donor_source),
                free_cursor,
                free_end,
                &|raw| {
                    write_3bpp_16x16(raw, 0, 0, &b1);
                    write_3bpp_16x16(raw, 2, 0, &b2);
                    write_3bpp_16x16(raw, 4, 0, &b1);
                    write_3bpp_16x16(raw, 6, 0, &m1);
                    write_3bpp_16x16(raw, 8, 0, &m2);
                    write_3bpp_16x16(raw, 10, 0, &m1);
                    write_3bpp_16x16(raw, 12, 0, &m2);
                    write_3bpp_16x16(raw, 12, 2, &m1);
                },
            )? {
                ops += 1;
            }
        }
        "items" => {
            if rule.donor_source == "items_pack" || rule.donor_source == "coin_mushroom" {
                if let (Some((coin, _)), Some((mush, _)), Some((bob, _))) = (
                    extract_donor_16x16_for_zelda(donor, "coin", false),
                    extract_donor_16x16_for_zelda(donor, "mushroom", false),
                    extract_donor_16x16_for_zelda(donor, "bobomb", false),
                ) {
                    let (g1, g2) = extract_donor_16x16_for_zelda(donor, "goomba", true)
                        .unwrap_or((bob, bob));
                    if patch_alttp_comp_sheet_helper(
                        rom_zelda,
                        session,
                        0xCD,
                        &format!("Collectibles Zelda (Sheet 0xCD) ← {}:Pièces & Champignons", rule.donor_alias),
                        free_cursor,
                        free_end,
                        &|raw| {
                            write_3bpp_16x16(raw, 0, 0, &coin);
                            write_3bpp_16x16(raw, 2, 0, &coin);
                            write_3bpp_16x16(raw, 4, 0, &coin);
                            write_3bpp_16x16(raw, 0, 2, &mush);
                        },
                    )? {
                        ops += 1;
                    }
                    for &pc in &[0x087600usize, 0x08AC00] {
                        let mut raw = rom_zelda.data[pc..pc + 0x600].to_vec();
                        write_3bpp_16x16(&mut raw, 4, 2, &g1);
                        write_3bpp_16x16(&mut raw, 6, 2, &g2);
                        write_3bpp_16x16(&mut raw, 14, 2, &bob);
                        session.swap_custom_bytes(
                            rom_zelda,
                            pc,
                            pc,
                            raw,
                            format!("Sprites Objets 0x{:06X} ← {}:Bob-ombs & Goombas", pc, rule.donor_alias),
                            SwapKind::GraphicsTiles {
                                format: TileFormat::Bpp3,
                                tile_count: 64,
                                smart_remapped: true,
                            },
                        )?;
                        ops += 1;
                    }
                    let mut hud = rom_zelda.data[0x088E00..0x089400].to_vec();
                    write_3bpp_16x16(&mut hud, 0, 2, &bob);
                    write_3bpp_16x16(&mut hud, 4, 2, &coin);
                    session.swap_custom_bytes(
                        rom_zelda,
                        0x088E00,
                        0x088E00,
                        hud,
                        format!("Icônes HUD 0x088E00 ← {}:Pièce & Bob-omb", rule.donor_alias),
                        SwapKind::GraphicsTiles {
                            format: TileFormat::Bpp3,
                            tile_count: 64,
                            smart_remapped: true,
                        },
                    )?;
                    ops += 1;
                }
            } else if let Some((it, _)) = extract_donor_16x16_for_zelda(donor, &rule.donor_source, false) {
                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0xCD,
                    &format!("Collectibles Zelda (Sheet 0xCD) ← {}:{}", rule.donor_alias, rule.donor_source),
                    free_cursor,
                    free_end,
                    &|raw| {
                        write_3bpp_16x16(raw, 0, 0, &it);
                        write_3bpp_16x16(raw, 2, 0, &it);
                        write_3bpp_16x16(raw, 0, 2, &it);
                    },
                )? {
                    ops += 1;
                }
            }
        }
        "world_rocks" | "rocks" => {
            if let Some((tile, _)) = extract_donor_16x16_for_zelda(donor, &rule.donor_source, false) {
                for &ow_sheet in &[0x06usize, 0x07, 0x08] {
                    if patch_alttp_comp_sheet_helper(
                        rom_zelda,
                        session,
                        ow_sheet,
                        &format!("Overworld Rochers (Sheet 0x{:02X}) ← {}:{}", ow_sheet, rule.donor_alias, rule.donor_source),
                        free_cursor,
                        free_end,
                        &|raw| {
                            write_3bpp_16x16(raw, 12, 0, &tile);
                            write_3bpp_16x16(raw, 14, 0, &tile);
                        },
                    )? {
                        ops += 1;
                    }
                }
            }
        }
        "audio_percussion" => {
            ops += inject_unpitched_brr_percussion(rom_zelda, donor, &rule.donor_alias, session, 3)?;
        }
        _ => {}
    }

    Ok(ops)
}

/// Hero-only SMW Mario -> Zelda ALTTP injector (delegates cleanly without overwriting enemies/items
/// when called via Omni-Mixer rules!)
fn inject_smw_mario_hero_into_zelda(
    rom_zelda: &mut SnesRom,
    rom_smw: &SnesRom,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    // Run full SMW->Zelda builder on a scratch clone, then lift ONLY the hero bank (0x080000..0x087000),
    // Bank $0D OAM table (0x068000..0x06838D), and 4 Mario palettes (0x0DD308..0x0DD380)!
    let mut scratch = rom_zelda.clone();
    let mut scratch_sess = SwapSession::new();
    apply_mario_into_zelda_alttp_swap(&mut scratch, rom_smw, &mut scratch_sess)?;

    let mut ops = 0usize;
    session.swap_custom_bytes(
        rom_zelda,
        0x080000,
        0x040000,
        scratch.data[0x080000..0x087000].to_vec(),
        "Héros Zelda -> Super Mario (SMW GFX32 16x32 complet + épée 4 dir)".into(),
        SwapKind::GraphicsTiles {
            format: TileFormat::Bpp4,
            tile_count: 896,
            smart_remapped: true,
        },
    )?;
    ops += 1;
    session.swap_custom_bytes(
        rom_zelda,
        0x068000,
        0x068000,
        scratch.data[0x068000..0x06838D].to_vec(),
        "Patch OAM Bank $0D (Y=-8, X=0) pour Super Mario 16x32".into(),
        SwapKind::CodePatch { byte_count: 303 * 3 },
    )?;
    ops += 1;
    session.swap_custom_bytes(
        rom_zelda,
        0x0DD308,
        0x0032DB,
        scratch.data[0x0DD308..0x0DD380].to_vec(),
        "Palettes Link -> Mario SMW (Classique, Feu, Luigi, Sélection)".into(),
        SwapKind::PaletteBgr555 { color_count: 60 },
    )?;
    ops += 1;
    Ok(ops)
}

/// Inject **Donkey Kong** (16x32 full unsquished Gorilla Hero with iconic Red Tie!) into Zelda ALTTP!
fn inject_dk_hero_into_zelda(
    rom_zelda: &mut SnesRom,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    let mut ops = 0usize;

    // Shift any 16x16 head down by `dy` pixels so DK's chin connects seamlessly to his shoulders (zero floating gap!)
    let shift_down = |src: &[[u8; 16]; 16], dy: usize| -> [[u8; 16]; 16] {
        let mut out = [[0u8; 16]; 16];
        for y in 0..16 {
            if y + dy < 16 {
                out[y + dy] = src[y];
            }
        }
        out
    };

    // DK sprites in ai_sprites face LEFT; mirror horizontally so Side faces RIGHT for ALTTP!
    let head_side = shift_down(&flip_h_16x16(&crate::ai_sprites::DK_HEAD_NORM), 2);
    let head_down = shift_down(&crate::ai_sprites::DK_HEAD_TURN, 2);
    let head_roar = shift_down(&flip_h_16x16(&crate::ai_sprites::DK_HEAD_ROAR), 2);
    // Clean Symmetrical Back Gorilla Head (brown fur dome + outline, zero eyes)
    let mut head_up = [[0u8; 16]; 16];
    for y in 0..16 {
        for x in 0..16 {
            let c = head_down[y][x];
            head_up[y][x] = match c {
                0 => 0,
                2 => 2,
                1 | 7 => 4, // replace eyes/muzzle with warm mid brown fur
                other => other,
            };
        }
    }
    let body_side_st = flip_h_16x16(&crate::ai_sprites::DK_BODY_ST);
    let body_side_w1 = flip_h_16x16(&crate::ai_sprites::DK_BODY_W1);
    let body_side_w2 = flip_h_16x16(&crate::ai_sprites::DK_BODY_W2);
    let body_side_cr = flip_h_16x16(&crate::ai_sprites::DK_BODY_CR);

    // Symmetrical Front-Facing Gorilla Torso (broad brown shoulders, tan chest, centered bright red tie!)
    // Indices: 0=trans, 2=outline, 3=deep brown, 4=mid brown, 5=fur highlight, 6=red tie, 7=tan chest
    let body_down_st: [[u8; 16]; 16] = [
        [0, 0, 2, 2, 2, 2, 2, 6, 6, 2, 2, 2, 2, 2, 0, 0],
        [0, 2, 4, 5, 4, 3, 2, 6, 6, 2, 3, 4, 5, 4, 2, 0],
        [0, 2, 4, 4, 3, 7, 7, 6, 6, 7, 7, 3, 4, 4, 2, 0],
        [2, 4, 4, 3, 2, 7, 7, 6, 6, 7, 7, 2, 3, 4, 4, 2],
        [2, 4, 3, 3, 2, 7, 7, 6, 6, 7, 7, 2, 3, 3, 4, 2],
        [2, 4, 3, 3, 2, 7, 7, 6, 6, 7, 7, 2, 3, 3, 4, 2],
        [2, 3, 3, 2, 3, 7, 7, 6, 6, 7, 7, 3, 2, 3, 3, 2],
        [2, 7, 7, 2, 3, 3, 7, 6, 6, 7, 3, 3, 2, 7, 7, 2],
        [0, 2, 2, 2, 3, 3, 3, 2, 2, 3, 3, 3, 2, 2, 2, 0],
        [0, 0, 2, 4, 4, 3, 3, 3, 3, 3, 3, 4, 4, 2, 0, 0],
        [0, 0, 2, 4, 3, 3, 2, 2, 2, 2, 3, 3, 4, 2, 0, 0],
        [0, 0, 2, 3, 3, 3, 2, 0, 0, 2, 3, 3, 3, 2, 0, 0],
        [0, 0, 2, 3, 3, 2, 2, 0, 0, 2, 2, 3, 3, 2, 0, 0],
        [0, 2, 7, 7, 7, 2, 0, 0, 0, 0, 2, 7, 7, 7, 2, 0],
        [0, 2, 7, 7, 7, 2, 0, 0, 0, 0, 2, 7, 7, 7, 2, 0],
        [0, 0, 2, 2, 2, 0, 0, 0, 0, 0, 0, 2, 2, 2, 0, 0],
    ];
    let mut body_down_w1 = body_down_st;
    // Stride 1: lift left foot 2px
    for y in 11..16 {
        for x in 1..7 {
            body_down_w1[y][x] = if y + 2 < 16 { body_down_st[y + 2][x] } else { 0 };
        }
    }
    let body_down_w2 = flip_h_16x16(&body_down_w1);

    // Symmetrical Back-Facing Gorilla Torso (broad brown fur back + shoulder shading)
    let mut body_up_st = body_down_st;
    for y in 0..10 {
        for x in 0..16 {
            if body_up_st[y][x] == 6 || body_up_st[y][x] == 7 {
                body_up_st[y][x] = if y <= 3 { 4 } else { 3 };
            }
        }
    }
    let mut body_up_w1 = body_up_st;
    for y in 11..16 {
        for x in 1..7 {
            body_up_w1[y][x] = if y + 2 < 16 { body_up_st[y + 2][x] } else { 0 };
        }
    }
    let body_up_w2 = flip_h_16x16(&body_up_w1);

    let mut link_bank = rom_zelda.data[0x080000..0x087000].to_vec();
    let mut write_cell = |mx: usize, my: usize, px: &[[u8; 16]; 16]| {
        let base = (my * 32 + mx * 2) * 32;
        let coords = [base, base + 32, base + 512, base + 544];
        for (q, &dst) in coords.iter().enumerate() {
            if dst + 32 > link_bank.len() {
                continue;
            }
            let qx = (q & 1) * 8;
            let qy = (q >> 1) * 8;
            let mut tile = [0u8; 32];
            for y in 0..8usize {
                let (mut b0, mut b1, mut b2, mut b3) = (0u8, 0u8, 0u8, 0u8);
                for x in 0..8usize {
                    let c = px[qy + y][qx + x] & 0x0F;
                    let sh = 7 - x;
                    b0 |= (c & 1) << sh;
                    b1 |= ((c >> 1) & 1) << sh;
                    b2 |= ((c >> 2) & 1) << sh;
                    b3 |= ((c >> 3) & 1) << sh;
                }
                tile[y * 2] = b0;
                tile[y * 2 + 1] = b1;
                tile[16 + y * 2] = b2;
                tile[16 + y * 2 + 1] = b3;
            }
            link_bank[dst..dst + 32].copy_from_slice(&tile);
        }
    };

    // Populate all Link Head & Body animation rows (0..27) cleanly while preserving sword arcs & FX!
    for mx in 0..8 {
        let h = match mx {
            0 => &head_side,
            2 => &head_up,
            _ => &head_down,
        };
        write_cell(mx, 0, h);
    }
    write_cell(0, 1, &body_side_w1);
    write_cell(1, 1, &body_side_cr);
    write_cell(2, 1, &body_side_w2);
    write_cell(3, 1, &body_down_st);
    write_cell(4, 1, &body_down_w1);
    write_cell(5, 1, &body_down_w2);
    write_cell(6, 1, &body_up_st);
    write_cell(7, 1, &head_side);

    write_cell(0, 2, &body_up_w1);
    write_cell(1, 2, &body_up_w2);
    write_cell(2, 2, &body_side_w1);
    write_cell(3, 2, &body_side_st);
    write_cell(4, 2, &body_side_w2);
    write_cell(5, 2, &body_side_cr);
    write_cell(6, 2, &body_side_w1);
    write_cell(7, 2, &body_side_w2);

    for mx in 0..8 {
        write_cell(mx, 3, if mx % 2 == 0 { &body_down_w1 } else { &body_side_w1 });
    }
    write_cell(0, 4, &body_down_st);
    write_cell(1, 4, &body_down_st);
    write_cell(2, 4, &head_side);
    write_cell(3, 4, &head_down);
    write_cell(4, 4, &head_up);

    // Rows 5..6, 7(5..7), 8(0..2), 10..25, 26(5..7), 27(0..4)
    for &(mx, my, is_head, dir) in &[
        (0, 5, true, 0), (1, 5, true, 2), (2, 5, true, 1), (3, 5, true, 0), (4, 5, true, 2),
        (5, 5, false, 0), (6, 5, false, 0), (7, 5, false, 1),
        (0, 6, false, 0), (1, 6, false, 2), (2, 6, false, 1), (3, 6, false, 0), (4, 6, false, 2),
        (5, 6, true, 0), (6, 6, true, 2), (7, 6, false, 0),
        (5, 7, true, 1), (6, 7, true, 0), (7, 7, true, 2),
        (0, 8, false, 0), (1, 8, false, 0), (2, 8, false, 0),
        (1, 10, false, 2), (2, 10, false, 1), (3, 10, true, 0), (4, 10, true, 0),
        (5, 10, true, 2), (6, 10, false, 0), (7, 10, true, 1),
        (1, 11, true, 0), (2, 11, false, 1), (3, 11, false, 1), (4, 11, false, 2),
        (5, 11, true, 1), (6, 11, true, 0), (7, 11, true, 2),
    ] {
        let px = match (is_head, dir) {
            (true, 0) => &head_side,
            (true, 1) => &head_down,
            (true, _) => &head_up,
            (false, 0) => &body_side_w1,
            (false, 1) => &body_down_st,
            (false, _) => &body_up_st,
        };
        write_cell(mx, my, px);
    }

    // Sword & shieldless walk rows 12..25, 26(5..7), 27(0..4)
    for my in 12..=24 {
        for mx in 0..8 {
            if (my == 15 || my == 17 || my == 18) && mx == 0 {
                continue; // preserve dust & zap skeleton FX
            }
            if (my == 17 && (mx == 1 || mx == 2))
                || (my == 18 && (mx == 1 || mx == 2))
                || ((my == 19 || my == 20) && mx <= 2)
                || (my == 23 && mx == 1)
                || (my == 24 && mx >= 6)
            {
                let h = match mx % 3 {
                    0 => &head_down,
                    1 => &head_roar,
                    _ => &head_up,
                };
                write_cell(mx, my, h);
            } else if (my == 13 && mx >= 5) || (my == 14 && (mx == 0 || mx >= 5)) {
                write_cell(mx, my, if mx % 2 == 0 { &body_up_w1 } else { &body_up_w2 });
            } else if my == 14 && (1..=4).contains(&mx) {
                write_cell(mx, my, if mx % 2 == 0 { &body_side_w1 } else { &body_side_w2 });
            } else if my == 22 || my == 23 {
                write_cell(mx, my, if mx % 2 == 0 { &body_side_w1 } else { &body_side_w2 });
            } else {
                write_cell(mx, my, if mx % 2 == 0 { &body_down_w1 } else { &body_down_w2 });
            }
        }
    }
    write_cell(0, 25, &head_up);
    write_cell(1, 25, &head_up);
    write_cell(2, 25, &head_down);
    write_cell(3, 25, &head_side);
    write_cell(4, 25, &body_side_st);
    write_cell(5, 26, &body_side_w1);
    write_cell(6, 26, &body_side_cr);
    write_cell(7, 26, &body_side_w2);
    write_cell(0, 27, &body_side_w1);
    write_cell(1, 27, &body_side_w2);
    write_cell(2, 27, &body_down_st);
    write_cell(3, 27, &body_up_st);
    write_cell(4, 27, &body_down_w1);
    drop(write_cell);

    // Muscular brown Donkey Kong forearm holding the sword hilt at (6, 25) bottom-left 8x8!
    let arm_dst = (25 * 32 + 6 * 2) * 32 + 512;
    if arm_dst + 32 <= link_bank.len() {
        const DK_ARM: [&str; 8] = [
            "........",
            "........",
            "222222..",
            "4444772.",
            "3333772.",
            "222222..",
            "........",
            "........",
        ];
        let mut tile = [0u8; 32];
        for (y, row) in DK_ARM.iter().enumerate() {
            let (mut b0, mut b1, mut b2, mut b3) = (0u8, 0u8, 0u8, 0u8);
            for (x, ch) in row.bytes().enumerate().take(8) {
                let c = if (b'1'..=b'9').contains(&ch) { ch - b'0' } else { 0 };
                let sh = 7 - x;
                b0 |= (c & 1) << sh;
                b1 |= ((c >> 1) & 1) << sh;
                b2 |= ((c >> 2) & 1) << sh;
                b3 |= ((c >> 3) & 1) << sh;
            }
            tile[y * 2] = b0;
            tile[y * 2 + 1] = b1;
            tile[16 + y * 2] = b2;
            tile[16 + y * 2 + 1] = b3;
        }
        link_bank[arm_dst..arm_dst + 32].copy_from_slice(&tile);
    }

    session.swap_custom_bytes(
        rom_zelda,
        0x080000,
        0x0D16C4,
        link_bank,
        "Héros Zelda -> Donkey Kong (DKC 16x32 complet + bras épée)".into(),
        SwapKind::GraphicsTiles {
            format: TileFormat::Bpp4,
            tile_count: 896,
            smart_remapped: true,
        },
    )?;
    ops += 1;

    // Bank $0D OAM Y=-8, X=0 so 32px Donkey Kong renders unsquished across ALL poses & 4 sword swings!
    let oam_pc = 0x068000usize;
    let oam_len = 303 * 3;
    let mut oam = rom_zelda.data[oam_pc..oam_pc + oam_len].to_vec();
    for i in 0..303 {
        if (oam[i * 3 + 2] & 0x0F) != 0x0F {
            oam[i * 3] = (-8i8) as u8;
            oam[i * 3 + 1] = 0;
        }
    }
    session.swap_custom_bytes(
        rom_zelda,
        oam_pc,
        oam_pc,
        oam,
        "Patch OAM Bank $0D (Y=-8, X=0) pour Donkey Kong 16x32".into(),
        SwapKind::CodePatch { byte_count: oam_len },
    )?;
    ops += 1;

    // Authentic Donkey Kong BGR555 palettes (Classic DK, Blue/Funky DK, Red/Cranky DK, File Select DK)
    let build_dk_pal = |tie_col: u16, fur_mid: u16, fur_dark: u16| -> Vec<u8> {
        let words: [u16; 15] = [
            0x7FFF,   // 1: White eyes
            0x0422,   // 2: Dark brown/black outline
            fur_dark, // 3: Deep fur brown
            fur_mid,  // 4: Mid fur brown
            0x263B,   // 5: Fur highlight
            tie_col,  // 6: Iconic Red Tie (or Blue/Gold variant!)
            0x431F,   // 7: Warm tan muzzle & chest
            0x0422, 0x001F, 0x1084, 0x263B, 0x431F, 0x0015, 0x219D, 0x03FF,
        ];
        let mut out = Vec::with_capacity(30);
        for w in words {
            out.extend_from_slice(&w.to_le_bytes());
        }
        out
    };
    let dk_classic = build_dk_pal(0x001F, 0x1154, 0x08AC); // Bright Red Tie + Rich Chestnut Fur
    let dk_funky = build_dk_pal(0x7E44, 0x1154, 0x08AC);   // Blue Mail -> Funky Cyan Tie!
    let dk_gold = build_dk_pal(0x03FF, 0x1997, 0x0CED);    // Red Mail -> Golden DK!
    let mut pals = Vec::with_capacity(120);
    pals.extend_from_slice(&dk_classic);
    pals.extend_from_slice(&dk_funky);
    pals.extend_from_slice(&dk_gold);
    pals.extend_from_slice(&dk_classic);

    session.swap_custom_bytes(
        rom_zelda,
        0x0DD308,
        0x39A068,
        pals,
        "Palettes Link -> Donkey Kong (Classique Cravate Rouge, Funky Bleu, Doré)".into(),
        SwapKind::PaletteBgr555 { color_count: 60 },
    )?;
    ops += 1;

    Ok(ops)
}

fn patch_alttp_comp_sheet_helper(
    rom_z: &mut SnesRom,
    sess: &mut SwapSession,
    sheet_id: usize,
    desc: &str,
    free_cursor: &mut usize,
    free_end: usize,
    edit_fn: &dyn Fn(&mut [u8]),
) -> Result<bool, RomError> {
    let Some((z_bk, z_hi, z_lo)) = find_alttp_gfx_table(&rom_z.data) else {
        return Ok(false);
    };
    let pc = ((rom_z.data[z_bk + sheet_id] as usize & 0x7F) << 15)
        | ((rom_z.data[z_hi + sheet_id] as usize & 0x7F) << 8)
        | (rom_z.data[z_lo + sheet_id] as usize);
    let Some((mut raw, orig_clen)) =
        decompress_nintendo_lz2_with_endian(&rom_z.data, pc, 4096, false)
    else {
        return Ok(false);
    };
    if raw.len() != 0x600 {
        return Ok(false);
    }
    edit_fn(&mut raw);
    let comp = compress_nintendo_lz2_with_endian(&raw, false);
    if decompress_nintendo_lz2_with_endian(&comp, 0, 4096, false).map(|(d, _)| d) != Some(raw) {
        return Ok(false);
    }
    if comp.len() <= orig_clen {
        let mut padded = comp;
        padded.resize(orig_clen, 0xFF);
        sess.swap_custom_bytes(
            rom_z,
            pc,
            pc,
            padded,
            desc.to_string(),
            SwapKind::GraphicsTiles {
                format: TileFormat::Bpp3,
                tile_count: 64,
                smart_remapped: true,
            },
        )?;
        Ok(true)
    } else if *free_cursor + comp.len() <= free_end {
        let dst_pc = *free_cursor;
        *free_cursor += comp.len();
        let snes_bank = (dst_pc / 0x8000) as u8;
        let snes_addr = 0x8000 + (dst_pc % 0x8000);
        sess.swap_custom_bytes(
            rom_z,
            dst_pc,
            pc,
            comp,
            format!("{} (repointé @ ${:02X}:{:04X})", desc, snes_bank, snes_addr),
            SwapKind::GraphicsTiles {
                format: TileFormat::Bpp3,
                tile_count: 64,
                smart_remapped: true,
            },
        )?;
        sess.swap_custom_bytes(rom_z, z_bk + sheet_id, z_bk + sheet_id, vec![snes_bank], format!("Ptr Bank 0x{:02X}", sheet_id), SwapKind::CodePatch { byte_count: 1 })?;
        sess.swap_custom_bytes(rom_z, z_hi + sheet_id, z_hi + sheet_id, vec![(snes_addr >> 8) as u8], format!("Ptr Hi 0x{:02X}", sheet_id), SwapKind::CodePatch { byte_count: 1 })?;
        sess.swap_custom_bytes(rom_z, z_lo + sheet_id, z_lo + sheet_id, vec![(snes_addr & 0xFF) as u8], format!("Ptr Lo 0x{:02X}", sheet_id), SwapKind::CodePatch { byte_count: 1 })?;
        Ok(true)
    } else {
        Ok(false)
    }
}

// =============================================================================
// TARGET 2: SUPER MARIO WORLD INJECTORS (HERO, MOUNT, ENEMIES, ITEMS, THEME, AUDIO)
// =============================================================================

fn apply_rule_to_smw(
    rom_smw: &mut SnesRom,
    donor: &SnesRom,
    rule: &MixRule,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    let mut ops = 0usize;
    match rule.target_slot.as_str() {
        "hero" => {
            if rule.donor_source == "link" && donor.identify_game() == RecognizedGame::ZeldaAlttp {
                ops += apply_link_into_smw_swap(rom_smw, donor, session)?;
            } else if rule.donor_source == "dk" {
                ops += inject_dk_as_player_in_smw(rom_smw, session)?;
            }
        }
        "mount" => {
            if rule.donor_source == "dk" {
                ops += apply_donkey_kong_into_smw_yoshi(rom_smw, &donor.data, session)?;
            }
        }
        "enemies" => {
            if let Some((e1, e2)) = extract_donor_16x16_for_smw(donor, &rule.donor_source) {
                if let Some((mut g01, pc, orig_len)) = load_smw_3bpp_sheet(&rom_smw.data, 0x01) {
                    // Replace SMW Goomba (8,2)/(10,2) & Koopa Shell (10,0)/(12,0) & Piranha (12,2)/(14,2)
                    write_3bpp_16x16(&mut g01, 8, 2, &e1);
                    write_3bpp_16x16(&mut g01, 10, 2, &e2);
                    write_3bpp_16x16(&mut g01, 10, 0, &e1);
                    write_3bpp_16x16(&mut g01, 12, 0, &e2);
                    let comp = compress_nintendo_lz2_with_endian(&g01, true);
                    if comp.len() <= orig_len
                        && decompress_nintendo_lz2_with_endian(&comp, 0, 4096, true).map(|(d, _)| d)
                            == Some(g01)
                    {
                        let mut padded = comp;
                        padded.resize(orig_len, 0xFF);
                        session.swap_custom_bytes(
                            rom_smw,
                            pc,
                            pc,
                            padded,
                            format!("Ennemis SMW (GFX01) ← {}:{}", rule.donor_alias, rule.donor_source),
                            SwapKind::GraphicsTiles {
                                format: TileFormat::Bpp3,
                                tile_count: 128,
                                smart_remapped: true,
                            },
                        )?;
                        ops += 1;
                    }
                }
            }
        }
        "items" => {
            if rule.donor_source == "zelda_items_pack" {
                if let (Some((rupee, _)), Some((heart, _))) = (
                    extract_donor_16x16_for_smw(donor, "zelda_rupee"),
                    extract_donor_16x16_for_smw(donor, "zelda_heart"),
                ) {
                    if let Some((mut g00, pc0, len0)) = load_smw_3bpp_sheet(&rom_smw.data, 0x00) {
                        write_3bpp_16x16(&mut g00, 4, 2, &heart); // Super Mushroom -> Zelda Heart!
                        let c0 = compress_nintendo_lz2_with_endian(&g00, true);
                        if c0.len() <= len0
                            && decompress_nintendo_lz2_with_endian(&c0, 0, 4096, true)
                                .map(|(d, _)| d)
                                == Some(g00)
                        {
                            let mut padded = c0;
                            padded.resize(len0, 0xFF);
                            session.swap_custom_bytes(
                                rom_smw,
                                pc0,
                                pc0,
                                padded,
                                format!("Objets SMW (GFX00) ← {}:Cœur Zelda", rule.donor_alias),
                                SwapKind::GraphicsTiles {
                                    format: TileFormat::Bpp3,
                                    tile_count: 128,
                                    smart_remapped: true,
                                },
                            )?;
                            ops += 1;
                        }
                    }
                    if let Some((mut g01, pc1, len1)) = load_smw_3bpp_sheet(&rom_smw.data, 0x01) {
                        write_3bpp_16x16(&mut g01, 8, 6, &rupee); // SMW Coin -> Zelda Rupee!
                        let c1 = compress_nintendo_lz2_with_endian(&g01, true);
                        if c1.len() <= len1
                            && decompress_nintendo_lz2_with_endian(&c1, 0, 4096, true)
                                .map(|(d, _)| d)
                                == Some(g01)
                        {
                            let mut padded = c1;
                            padded.resize(len1, 0xFF);
                            session.swap_custom_bytes(
                                rom_smw,
                                pc1,
                                pc1,
                                padded,
                                format!("Pièces SMW (GFX01) ← {}:Rubis Zelda", rule.donor_alias),
                                SwapKind::GraphicsTiles {
                                    format: TileFormat::Bpp3,
                                    tile_count: 128,
                                    smart_remapped: true,
                                },
                            )?;
                            ops += 1;
                        }
                    }
                }
            }
        }
        "title_story" => {
            if rule.donor_source == "hyrule_theme" {
                ops += apply_link_world_title_screen(rom_smw, session)?;
            }
        }
        "audio_percussion" => {
            ops += inject_unpitched_brr_percussion(rom_smw, donor, &rule.donor_alias, session, 3)?;
        }
        _ => {}
    }
    Ok(ops)
}

/// Inject **Donkey Kong** directly as the playable Player Hero inside Super Mario World (`GFX32` + Palette)!
fn inject_dk_as_player_in_smw(
    rom_smw: &mut SnesRom,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    let mut ops = 0usize;
    let Some((mut g32, _)) = decompress_nintendo_lz2_with_endian(&rom_smw.data, 0x040000, 32768, true) else {
        return Ok(0);
    };
    if g32.len() < 23808 {
        g32.resize(23808, 0);
    }

    // Locate SMW's pose tables dynamically using Super Mario signature
    const SUPER_SIG: [u8; 14] = [0x70, 0x70, 0x70, 0xA0, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0xA0, 0x74, 0x70, 0x80];
    let Some(sig_pos) = rom_smw.data.windows(SUPER_SIG.len()).position(|w| w == SUPER_SIG) else {
        return Ok(0);
    };
    let pose_top_base = sig_pos - 0x46;
    let pose_bot_base = pose_top_base + 0xC0;

    let write_g32_16x16 = |buf: &mut [u8], val: u8, px: &[[u8; 16]; 16]| {
        let off = (((val as usize) & 0x08) << 11) | (((val as usize) & 0xF7) << 6);
        let coords = [off, off + 32, off + 512, off + 544];
        for (q, &dst) in coords.iter().enumerate() {
            if dst + 32 > buf.len() {
                continue;
            }
            let qx = (q & 1) * 8;
            let qy = (q >> 1) * 8;
            for y in 0..8 {
                let (mut b0, mut b1, mut b2, mut b3) = (0u8, 0u8, 0u8, 0u8);
                for x in 0..8 {
                    let c = px[qy + y][qx + x] & 0x0F;
                    let sh = 7 - x;
                    b0 |= (c & 1) << sh;
                    b1 |= ((c >> 1) & 1) << sh;
                    b2 |= ((c >> 2) & 1) << sh;
                    b3 |= ((c >> 3) & 1) << sh;
                }
                buf[dst + y * 2] = b0;
                buf[dst + y * 2 + 1] = b1;
                buf[dst + 16 + y * 2] = b2;
                buf[dst + 16 + y * 2 + 1] = b3;
            }
        }
    };

    // Map DK 3bpp indices (1=white, 2=outline, 3=deep brown, 4=mid brown, 5=fur hi, 6=red tie, 7=tan)
    // onto SMW Mario CGRAM indices (1=white, 2=black, 3=brown, 6=tan, 9=red tie, 10=deep brown, 11=mid brown, 12=fur hi)
    const DK_TO_SMW_MARIO: [u8; 8] = [0, 1, 2, 10, 11, 12, 9, 6];
    let map_dk = |src: &[[u8; 16]; 16]| -> [[u8; 16]; 16] {
        let mut out = [[0u8; 16]; 16];
        for y in 0..16 {
            for x in 0..16 {
                out[y][x] = DK_TO_SMW_MARIO[(src[y][x] & 7) as usize];
            }
        }
        out
    };

    let dk_small = map_dk(&crate::ai_sprites::DK_BABY_DK);
    let dk_head = map_dk(&crate::ai_sprites::DK_HEAD_NORM);
    let dk_head_roar = map_dk(&crate::ai_sprites::DK_HEAD_ROAR);
    let dk_body_st = map_dk(&crate::ai_sprites::DK_BODY_ST);
    let dk_body_w1 = map_dk(&crate::ai_sprites::DK_BODY_W1);
    let dk_body_w2 = map_dk(&crate::ai_sprites::DK_BODY_W2);
    let blank16 = [[0u8; 16]; 16];

    // Patch Small Mario pose_top_base (0x00..0x46 -> blank tile 197) + Super/Cape shared-leg remaps via SwapSession
    // so both .sfc AND .ips patches are 100% consistent!
    let mut patched_top = rom_smw.data[pose_top_base..pose_top_base + 0x46].to_vec();
    patched_top.fill(197);
    session.swap_custom_bytes(
        rom_smw,
        pose_top_base,
        0x0D16C4,
        patched_top,
        "Table Poses Top Small Mario SMW -> Tile vierge 197 (Mini Donkey Kong propre)".into(),
        SwapKind::CodePatch { byte_count: 0x46 },
    )?;
    ops += 1;

    let mut patched_bot = rom_smw.data[pose_bot_base..pose_bot_base + 0xC0].to_vec();
    for &(rel, new_t) in &[(0x55usize, 0x7Eu8), (0x92, 0x7E), (0x6B, 0x7F), (0xA8, 0x7F)] {
        if rel < patched_bot.len() {
            patched_bot[rel] = new_t;
        }
    }
    session.swap_custom_bytes(
        rom_smw,
        pose_bot_base,
        0x0D16C4,
        patched_bot,
        "Table Poses Bot Super/Cape Mario SMW -> Remap jambes partagées".into(),
        SwapKind::CodePatch { byte_count: 0xC0 },
    )?;
    ops += 1;

    // Populate Super/Cape DK poses FIRST (skipping blank tile 197 so it NEVER receives a head!)
    for y in 0x46..0xC0 {
        let pose = if y < 0x83 { y - 0x46 } else { y - 0x83 };
        let va = rom_smw.data[pose_top_base + y];
        let vb = rom_smw.data[pose_bot_base + y];
        let h = if pose == 0x0B || pose == 0x0D { &dk_head_roar } else { &dk_head };
        let b = match pose % 3 {
            0 => &dk_body_st,
            1 => &dk_body_w1,
            _ => &dk_body_w2,
        };
        if va != 197 {
            write_g32_16x16(&mut g32, va, h);
        }
        if vb != 197 {
            write_g32_16x16(&mut g32, vb, b);
        }
    }

    // Populate Small DK (16x16 Mini Donkey Kong) + explicitly wipe blank tile 197 LAST so Small DK NEVER has a double head!
    for y in 0..0x46 {
        let vb = rom_smw.data[pose_bot_base + y];
        if vb != 197 {
            write_g32_16x16(&mut g32, vb, &dk_small);
        }
    }
    write_g32_16x16(&mut g32, 197, &blank16);

    let comp = compress_nintendo_lz2_with_endian(&g32, true);
    if comp.len() <= 0x3FC0 {
        let mut padded = comp;
        padded.resize(0x3FC0, 0xFF);
        session.swap_custom_bytes(
            rom_smw,
            0x040000,
            0x0D16C4,
            padded,
            "Héros SMW (GFX32) -> Donkey Kong Jouable !".into(),
            SwapKind::GraphicsTiles {
                format: TileFormat::Bpp4,
                tile_count: 744,
                smart_remapped: true,
            },
        )?;
        ops += 1;
    }

    // Patch SMW Mario Player Palette (USA 0x0032C8 / PAL 0x0032DB) with Donkey Kong Fur + Red Tie + Warm Tan!
    const MARIO_PAL_SIG: [u8; 8] = [0x5F, 0x63, 0x1D, 0x58, 0x0A, 0x00, 0x1F, 0x39];
    let m_pal = rom_smw.data[..rom_smw.data.len().min(0x008000)]
        .windows(MARIO_PAL_SIG.len())
        .position(|w| w == MARIO_PAL_SIG)
        .unwrap_or(if rom_smw.data.get(0x7FD9).copied() == Some(0x02) {
            0x0032DB
        } else {
            0x0032C8
        });
    if m_pal + 20 <= rom_smw.data.len() {
        let dk_10: [u16; 10] = [
            0x431F, // 6: Warm tan muzzle/chest
            0x35DF, // 7: Peach accent
            0x0012, // 8: Dark tie shadow
            0x001F, // 9: Bright Red Tie!
            0x08AC, // 10: Deep fur brown
            0x1154, // 11: Mid fur brown
            0x263B, // 12: Fur highlight
            0x0015, // 13: Red shadow
            0x219D, // 14: Skin shadow
            0x03FF, // 15: Gold accent
        ];
        let mut bytes = Vec::with_capacity(20);
        for w in dk_10 {
            bytes.extend_from_slice(&w.to_le_bytes());
        }
        session.swap_custom_bytes(
            rom_smw,
            m_pal,
            0x39A068,
            bytes,
            format!("Palette Joueur SMW @ 0x{:06X} -> Donkey Kong (Fourrure & Cravate Rouge)", m_pal),
            SwapKind::PaletteBgr555 { color_count: 10 },
        )?;
        ops += 1;
    }

    Ok(ops)
}

// =============================================================================
// TARGET 3: DONKEY KONG COUNTRY INJECTORS
// =============================================================================

fn apply_rule_to_dkc(
    rom_dkc: &mut SnesRom,
    donor: &SnesRom,
    rule: &MixRule,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    match rule.target_slot.as_str() {
        "hero" => apply_swap_into_dkc(rom_dkc, donor, session),
        "palette" => {
            if 0x39A068 + 30 <= rom_dkc.data.len() {
                let mut new_pal = rom_dkc.data[0x39A068..0x39A068 + 30].to_vec();
                if donor.identify_game() == RecognizedGame::SuperMarioWorld {
                    let smw_pal = SnesPalette::mario_classic_full_16(Some(&donor.data));
                    for (i, c) in smw_pal.colors_rgba.iter().skip(1).take(15).enumerate() {
                        let w = crate::graphics::rgb_to_bgr555(c[0], c[1], c[2]);
                        new_pal[i * 2..i * 2 + 2].copy_from_slice(&w.to_le_bytes());
                    }
                } else if donor.identify_game() == RecognizedGame::ZeldaAlttp {
                    new_pal.copy_from_slice(&donor.data[0x0DD308..0x0DD308 + 30]);
                }
                session.swap_custom_bytes(
                    rom_dkc,
                    0x39A068,
                    0x0DD308,
                    new_pal,
                    format!("Palette DKC (0x39A068) ← {}:{}", rule.donor_alias, rule.donor_source),
                    SwapKind::PaletteBgr555 { color_count: 15 },
                )?;
                return Ok(1);
            }
            Ok(0)
        }
        "audio_percussion" => {
            inject_unpitched_brr_percussion(rom_dkc, donor, &rule.donor_alias, session, 4)
        }
        _ => Ok(0),
    }
}

// =============================================================================
// UNIVERSAL CROSS-ROM AUDIO PERCUSSION & RAW SLOT SWAPPERS (WORKS ON ANY ROM!)
// =============================================================================

fn inject_unpitched_brr_percussion(
    target_rom: &mut SnesRom,
    donor_rom: &SnesRom,
    donor_alias: &str,
    session: &mut SwapSession,
    max_swaps: usize,
) -> Result<usize, RomError> {
    let target_samps = scan_brr_samples(&target_rom.data, 20);
    let donor_samps = scan_brr_samples(&donor_rom.data, 20);

    let is_hw_oneshot = |rom: &[u8], s: &crate::audio::BrrSampleInfo| -> bool {
        if !s.loops {
            return true;
        }
        if s.byte_len >= 9 && s.offset + s.byte_len <= rom.len() {
            let last_hdr = rom[s.offset + s.byte_len - 9];
            (last_hdr & 0x02) == 0
        } else {
            false
        }
    };

    let mut t_perc: Vec<_> = target_samps
        .into_iter()
        .filter(|s| is_hw_oneshot(&target_rom.data, s) && s.block_count >= 12 && s.block_count <= 320)
        .collect();
    let mut d_perc: Vec<_> = donor_samps
        .into_iter()
        .filter(|s| is_hw_oneshot(&donor_rom.data, s) && s.block_count >= 12 && s.block_count <= 450)
        .collect();

    t_perc.sort_by(|a, b| b.block_count.cmp(&a.block_count));
    d_perc.sort_by(|a, b| b.block_count.cmp(&a.block_count));

    let mut ops = 0usize;
    let count = t_perc.len().min(d_perc.len()).min(max_swaps);
    for i in 0..count {
        let ts = &t_perc[i];
        let ds = &d_perc[i];
        let raw_donor = &donor_rom.data[ds.offset..ds.offset + ds.byte_len];
        let adapted = resample_and_encode_brr_for_swap(raw_donor, ts.byte_len, false);
        if adapted.len() == ts.byte_len {
            session.swap_custom_bytes(
                target_rom,
                ts.offset,
                ds.offset,
                adapted,
                format!(
                    "Audio BRR Percussion #{} (0x{:06X}, {} blocs) ← {} (0x{:06X})",
                    i + 1,
                    ts.offset,
                    ts.block_count,
                    donor_alias,
                    ds.offset
                ),
                SwapKind::AudioBrrSample {
                    target_blocks: ts.block_count,
                    donor_blocks: ds.block_count,
                    loops: ts.loops,
                    resampled: true,
                },
            )?;
            ops += 1;
        }
    }
    Ok(ops)
}

fn apply_universal_single_rule(
    rom_a: &mut SnesRom,
    rom_b: &SnesRom,
    cat_a: &UniversalRomCatalog,
    cat_b: &UniversalRomCatalog,
    rule: &MixRule,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    if let (Some(ti_s), Some(di_s)) = (
        rule.target_slot.strip_prefix("gfx:"),
        rule.donor_source.strip_prefix("gfx:"),
    ) {
        let (Ok(ti), Ok(di)) = (ti_s.parse::<usize>(), di_s.parse::<usize>()) else {
            return Ok(0);
        };
        let (Some(ga), Some(gb)) = (cat_a.graphics.get(ti), cat_b.graphics.get(di)) else {
            return Ok(0);
        };
        let Some(raw_b) = decode_universal_gfx_bytes(rom_b, gb) else {
            return Ok(0);
        };
        let fallback_gray = || SnesPalette {
            name: "Grayscale".into(),
            colors_rgba: (0..16u8).map(|i| [i * 17, i * 17, i * 17, if i == 0 { 0 } else { 255 }]).collect(),
            source_offset: None,
        };
        let pal_a = cat_a
            .palettes
            .get(ga.suggested_palette_idx)
            .map(|p| SnesPalette {
                name: p.name.clone(),
                colors_rgba: p.colors_rgba.clone(),
                source_offset: Some(p.rom_offset),
            })
            .unwrap_or_else(fallback_gray);
        let pal_b = cat_b
            .palettes
            .get(gb.suggested_palette_idx)
            .map(|p| SnesPalette {
                name: p.name.clone(),
                colors_rgba: p.colors_rgba.clone(),
                source_offset: Some(p.rom_offset),
            })
            .unwrap_or_else(fallback_gray);

        let tiles_to_copy = ga.tile_count.min(gb.tile_count);
        let transcoded = remap_and_transcode_tiles(&raw_b, tiles_to_copy, gb.format, &pal_b, ga.format, &pal_a);
        let mut full_raw = decode_universal_gfx_bytes(rom_a, ga).unwrap_or_else(|| vec![0u8; ga.decompressed_len]);
        let copy_len = transcoded.len().min(full_raw.len());
        full_raw[..copy_len].copy_from_slice(&transcoded[..copy_len]);

        let payload = match ga.compression {
            UniversalGfxCompression::None => full_raw,
            UniversalGfxCompression::NintendoLz2BigEndian => {
                let c = compress_nintendo_lz2_with_endian(&full_raw, true);
                if c.len() > ga.rom_byte_budget {
                    return Ok(0);
                }
                let mut p = c;
                p.resize(ga.rom_byte_budget, 0xFF);
                p
            }
            UniversalGfxCompression::NintendoLz2LittleEndian => {
                let c = compress_nintendo_lz2_with_endian(&full_raw, false);
                if c.len() > ga.rom_byte_budget {
                    return Ok(0);
                }
                let mut p = c;
                p.resize(ga.rom_byte_budget, 0xFF);
                p
            }
        };

        if ga.rom_offset + payload.len() <= rom_a.data.len() {
            session.swap_custom_bytes(
                rom_a,
                ga.rom_offset,
                gb.rom_offset,
                payload,
                format!(
                    "Planche GFX Universelle {} ({}) ← {}:{} ({})",
                    ga.snes_addr,
                    ga.format.short_label(),
                    rule.donor_alias,
                    gb.snes_addr,
                    gb.format.short_label()
                ),
                SwapKind::GraphicsTiles {
                    format: ga.format,
                    tile_count: tiles_to_copy,
                    smart_remapped: true,
                },
            )?;
            return Ok(1);
        }
    } else if let (Some(ti_s), Some(di_s)) = (
        rule.target_slot.strip_prefix("pal:"),
        rule.donor_source.strip_prefix("pal:"),
    ) {
        let (Ok(ti), Ok(di)) = (ti_s.parse::<usize>(), di_s.parse::<usize>()) else {
            return Ok(0);
        };
        let (Some(pa), Some(pb)) = (cat_a.palettes.get(ti), cat_b.palettes.get(di)) else {
            return Ok(0);
        };
        let copy_bytes = pa.byte_len.min(pb.byte_len);
        if pa.rom_offset + copy_bytes <= rom_a.data.len()
            && pb.rom_offset + copy_bytes <= rom_b.data.len()
        {
            let slice = rom_b.data[pb.rom_offset..pb.rom_offset + copy_bytes].to_vec();
            session.swap_custom_bytes(
                rom_a,
                pa.rom_offset,
                pb.rom_offset,
                slice,
                format!(
                    "Palette BGR555 {} ← {}:{}",
                    pa.snes_addr, rule.donor_alias, pb.snes_addr
                ),
                SwapKind::PaletteBgr555 {
                    color_count: copy_bytes / 2,
                },
            )?;
            return Ok(1);
        }
    } else if let (Some(ti_s), Some(di_s)) = (
        rule.target_slot.strip_prefix("audio:"),
        rule.donor_source.strip_prefix("audio:"),
    ) {
        let (Ok(ti), Ok(di)) = (ti_s.parse::<usize>(), di_s.parse::<usize>()) else {
            return Ok(0);
        };
        let (Some(aa), Some(ab)) = (cat_a.audio_samples.get(ti), cat_b.audio_samples.get(di)) else {
            return Ok(0);
        };
        if aa.rom_offset + aa.byte_len <= rom_a.data.len()
            && ab.rom_offset + ab.byte_len <= rom_b.data.len()
        {
            let raw_b = &rom_b.data[ab.rom_offset..ab.rom_offset + ab.byte_len];
            let adapted = resample_and_encode_brr_for_swap(raw_b, aa.byte_len, aa.loops);
            if adapted.len() == aa.byte_len {
                session.swap_custom_bytes(
                    rom_a,
                    aa.rom_offset,
                    ab.rom_offset,
                    adapted,
                    format!(
                        "Audio BRR {} ({} blocs) ← {}:{}",
                        aa.snes_addr, aa.block_count, rule.donor_alias, ab.snes_addr
                    ),
                    SwapKind::AudioBrrSample {
                        target_blocks: aa.block_count,
                        donor_blocks: ab.block_count,
                        loops: aa.loops,
                        resampled: true,
                    },
                )?;
                return Ok(1);
            }
        }
    }

    Ok(0)
}

fn decode_universal_gfx_bytes(
    rom: &SnesRom,
    g: &crate::universal::UniversalGfxAsset,
) -> Option<Vec<u8>> {
    match g.compression {
        UniversalGfxCompression::None => {
            if g.rom_offset + g.rom_byte_budget <= rom.data.len() {
                Some(rom.data[g.rom_offset..g.rom_offset + g.rom_byte_budget].to_vec())
            } else {
                None
            }
        }
        UniversalGfxCompression::NintendoLz2BigEndian => {
            decompress_nintendo_lz2_with_endian(&rom.data, g.rom_offset, g.decompressed_len.max(4096), true)
                .map(|(d, _)| d)
        }
        UniversalGfxCompression::NintendoLz2LittleEndian => {
            decompress_nintendo_lz2_with_endian(&rom.data, g.rom_offset, g.decompressed_len.max(4096), false)
                .map(|(d, _)| d)
        }
    }
}
