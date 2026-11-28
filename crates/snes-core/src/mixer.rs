//! Universal Multi-Game SNES Omni-Mixer Engine (`N` Donor ROMs -> 1 Base ROM)
//!
//! Enables mixing **anything with anything** across any combination of SNES ROMs:
//! - Semantic high-fidelity cross-game mixing for recognized games (Zelda ALTTP, Super Mario World):
//!   heroes, mounts, soldiers/enemies, collectibles/items, world tiles, palettes,
//!   and unpitched BRR percussion audio.
//! - 100% generic ROM-agnostic mixing (`gfx:N`, `pal:N`, `audio:N`) powered by hardware DMA/LZ2/BRR
//!   discovery for ANY unknown LoROM / HiROM / ExHiROM cartridge.
//! - Reproducible JSON `MixRecipe` serialization/deserialization + deterministic Chaos Randomizer.

use std::collections::HashMap;
use std::time::Instant;

use crate::audio::{resample_and_encode_brr_for_swap, scan_brr_samples};
use crate::demo::{
    apply_link_into_smw_swap, apply_link_world_title_screen, apply_mario_into_zelda_alttp_swap,
};
use crate::disasm_dma::{compress_nintendo_lz2_with_endian, decompress_nintendo_lz2_with_endian};
use crate::graphics::{remap_and_transcode_tiles, SnesPalette, TileFormat};
use crate::rom::{RecognizedGame, RomError, SnesRom};
use crate::swapper::{SwapKind, SwapSession};
use crate::universal::{discover_universal_catalog, UniversalGfxCompression, UniversalRomCatalog};

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
        let base_rom = extract_str(input, "base_rom")
            .ok_or_else(|| "Champ 'base_rom' manquant".to_string())?;

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
// LOW-LEVEL 3BPP / 4BPP / POINTER-TABLE HELPERS (SMW, ZELDA, GENERIC)
// =============================================================================

fn find_smw_gfx_table(smw: &[u8]) -> Option<(usize, usize, usize)> {
    for i in 0x003000..0x004400usize {
        if i + 0x64 < smw.len() && smw[i] == 0x1E && smw[i + 0x32] == 0xEF && smw[i + 0x64] == 0x09
        {
            let lo = i - 0x10;
            return Some((lo, lo + 0x32, lo + 0x64));
        }
    }
    None
}

pub(crate) fn load_smw_3bpp_sheet(smw: &[u8], sheet: usize) -> Option<(Vec<u8>, usize, usize)> {
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

pub(crate) fn find_alttp_gfx_table_pub(zelda: &[u8]) -> Option<(usize, usize, usize)> {
    find_alttp_gfx_table(zelda)
}

/// Exact unshifted ALTTP GFX pointer table (`0x4F00` / `0x4FDF` / `0x50BE` in FR PAL):
/// - `raw_idx = 0x00..=0x72` -> BG tileset sheet `bg_id`
/// - `raw_idx = 0x73 + sp_id` -> Sprite sheet `sp_id` (`SP[0x00] = raw 0x73 = $10:F000`)
pub(crate) fn find_alttp_raw_gfx_table(zelda: &[u8]) -> Option<(usize, usize, usize)> {
    let (b, h, l) = find_alttp_gfx_table(zelda)?;
    if b >= 2 && h >= 2 && l >= 2 {
        Some((b - 2, h - 2, l - 2))
    } else {
        None
    }
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

pub(crate) fn read_3bpp_16x16(buf: &[u8], tx: usize, ty: usize) -> [[u8; 16]; 16] {
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
                    px[dy * 8 + y][dx * 8 + x] =
                        ((b0 >> sh) & 1) | (((b1 >> sh) & 1) << 1) | (((b2 >> sh) & 1) << 2);
                }
            }
        }
    }
    px
}

pub(crate) fn write_3bpp_16x16_pub(buf: &mut [u8], tx: usize, ty: usize, px: &[[u8; 16]; 16]) {
    write_3bpp_16x16(buf, tx, ty, px);
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

pub(crate) fn write_3bpp_8x8_pub(buf: &mut [u8], tx: usize, ty: usize, px: &[[u8; 8]; 8]) {
    write_3bpp_8x8(buf, tx, ty, px);
}

fn write_3bpp_8x8(buf: &mut [u8], tx: usize, ty: usize, px: &[[u8; 8]; 8]) {
    let ti = ty * 16 + tx;
    let off = ti * 24;
    if off + 24 > buf.len() {
        return;
    }
    for y in 0..8usize {
        let (mut b0, mut b1, mut b2) = (0u8, 0u8, 0u8);
        for x in 0..8usize {
            let c = px[y][x] & 7;
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

pub(crate) fn remap_16x16_pub(src: &[[u8; 16]; 16], lut: &[u8; 8]) -> [[u8; 16]; 16] {
    remap_16x16(src, lut)
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

pub(crate) fn flip_h_16x16_pub(src: &[[u8; 16]; 16]) -> [[u8; 16]; 16] {
    flip_h_16x16(src)
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
    const LUT_KOOPA_TRUE: [u8; 8] = [0, 1, 5, 7, 6, 6, 3, 4]; // Universal Green/Blue/Red Koopa shell + warm yellow-tan snout (ZERO pink!)
    const LUT_GOOMBA_SOL: [u8; 8] = [0, 1, 5, 7, 6, 4, 4, 3];
    const LUT_GOOMBA_BRN: [u8; 8] = [0, 1, 5, 7, 6, 1, 7, 2]; // RTX-3080 CIELAB optimal for PAL_ALTTP_BRN (S 0x8D)
    const LUT_MUSHROOM: [u8; 8] = [0, 1, 5, 3, 2, 2, 2, 3]; // Verified ALTTP 0x0DD228 Red Sprite Pal: 2/3=Vibrant Ruby Red cap, 1=White spots/stem, 5=Outline!
    const LUT_GOLD: [u8; 8] = [0, 1, 5, 2, 6, 6, 7, 2]; // RTX-3080 CIELAB optimal for PAL_ALTTP_GLD (pure gold, zero violet!)
    const LUT_BOBOMB: [u8; 8] = [0, 1, 5, 6, 2, 1, 5, 5];
    const LUT_QBLOCK_SPR: [u8; 8] = [0, 1, 5, 2, 6, 6, 7, 2]; // RTX-3080 CIELAB optimal (91.6% sim!)
    const LUT_MAGIK_AGA: [u8; 8] = [0, 1, 5, 7, 6, 1, 2, 3]; // RTX-3080 CIELAB optimal for PAL_ALTTP_AGA (blue/purple robe + gold hat!)
    const LUT_DRYBONES: [u8; 8] = [0, 1, 5, 7, 2, 1, 6, 7]; // RTX-3080 CIELAB optimal for PAL_ALTTP_CRM (96.5% sim, crisp white/silver bones!)
    const LUT_THWOMP: [u8; 8] = [0, 1, 5, 7, 6, 1, 6, 7]; // RTX-3080 CIELAB optimal for PAL_ALTTP_CRM (95.3% SSIM!)
    const LUT_BOO: [u8; 8] = [0, 1, 5, 7, 3, 1, 1, 6]; // RTX-3080 CIELAB optimal for PAL_ALTTP_CRM (90.4% sim, white + red tongue!)
    const LUT_YOSHI_EGG: [u8; 8] = [0, 1, 5, 7, 6, 1, 2, 3]; // RTX-3080 CIELAB optimal for PAL_ALTTP_GRN (91.4% sim, white + green spots!)
    const LUT_FIRE_FLW: [u8; 8] = [0, 1, 5, 3, 1, 4, 3, 2]; // Verified ALTTP 0x0DD228 Red Lantern Pal: 2/3=Fiery Red petals, 1/4=White/Cream stem, 5=Outline!

    let composite_koopa = |g01: &[u8], hx: usize, bx: usize| -> [[u8; 16]; 16] {
        let h = remap_16x16(&read_3bpp_16x16(g01, hx, 0), &LUT_KOOPA_TRUE);
        let b = remap_16x16(&read_3bpp_16x16(g01, bx, 2), &LUT_KOOPA_TRUE);
        let mut out = [[0u8; 16]; 16];
        for y in 4..16usize {
            for x in 0..16usize {
                if b[y][x] != 0 {
                    out[y][x] = b[y][x];
                }
            }
        }
        for y in 0..10usize {
            let sy = y + 5;
            if sy < 16 {
                for x in 0..16usize {
                    if h[sy][x] != 0 {
                        out[y][x] = h[sy][x];
                    }
                }
            }
        }
        out
    };

    match source_id {
        "koopa" => {
            let (g01, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x01)?;
            Some((composite_koopa(&g01, 2, 0), composite_koopa(&g01, 4, 2)))
        }
        "chargin_chuck" => {
            let (g13, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x13)?;
            let h1 = remap_16x16(&read_3bpp_16x16(&g13, 14, 0), &LUT_STD);
            let b1 = remap_16x16(&read_3bpp_16x16(&g13, 6, 2), &LUT_STD);
            let mut c1 = [[0u8; 16]; 16];
            for y in 6..16usize {
                for x in 0..16usize {
                    if b1[y - 1][x] != 0 {
                        c1[y][x] = b1[y - 1][x];
                    }
                }
            }
            for y in 0..9usize {
                for x in 0..16usize {
                    if h1[y + 3][x] != 0 {
                        c1[y][x] = h1[y + 3][x];
                    }
                }
            }
            let c2 = remap_16x16(&read_3bpp_16x16(&g13, 6, 4), &LUT_STD);
            Some((c1, c2))
        }
        "goomba" => {
            let (g01, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x01)?;
            let lut = if for_soldier {
                &LUT_GOOMBA_SOL
            } else {
                &LUT_GOOMBA_BRN
            };
            Some((
                remap_16x16(&read_3bpp_16x16(&g01, 8, 2), lut),
                remap_16x16(&read_3bpp_16x16(&g01, 10, 2), lut),
            ))
        }
        "koopa_shell" => {
            let (g01, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x01)?;
            Some((
                remap_16x16(&read_3bpp_16x16(&g01, 10, 0), &LUT_KOOPA_TRUE),
                remap_16x16(&read_3bpp_16x16(&g01, 12, 0), &LUT_KOOPA_TRUE),
            ))
        }
        "spiny" => {
            let (g02, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x02)?;
            Some((
                remap_16x16(&read_3bpp_16x16(&g02, 0, 0), &LUT_STD),
                remap_16x16(&read_3bpp_16x16(&g02, 2, 0), &LUT_STD),
            ))
        }
        "buzzy_beetle" => {
            let (g02, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x02)?;
            let bb = remap_16x16(&read_3bpp_16x16(&g02, 12, 0), &LUT_STD);
            Some((bb, flip_h_16x16(&bb)))
        }
        "dry_bones" => {
            let (g03, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x03)?;
            Some((
                remap_16x16(&read_3bpp_16x16(&g03, 4, 0), &LUT_DRYBONES),
                remap_16x16(&read_3bpp_16x16(&g03, 6, 0), &LUT_DRYBONES),
            ))
        }
        "thwomp" => {
            let (g03, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x03)?;
            Some((
                remap_16x16(&read_3bpp_16x16(&g03, 8, 4), &LUT_THWOMP),
                remap_16x16(&read_3bpp_16x16(&g03, 10, 4), &LUT_THWOMP),
            ))
        }
        "bowser_statue" => {
            let (g03, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x03)?;
            Some((
                remap_16x16(&read_3bpp_16x16(&g03, 0, 4), &LUT_THWOMP),
                remap_16x16(&read_3bpp_16x16(&g03, 2, 4), &LUT_THWOMP),
            ))
        }
        "wiggler" => {
            let (g04, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x04)?;
            Some((
                remap_16x16(&read_3bpp_16x16(&g04, 0, 0), &LUT_STD),
                remap_16x16(&read_3bpp_16x16(&g04, 2, 0), &LUT_STD),
            ))
        }
        "monty_mole" => {
            let (g04, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x04)?;
            Some((
                remap_16x16(&read_3bpp_16x16(&g04, 0, 6), &LUT_STD),
                remap_16x16(&read_3bpp_16x16(&g04, 2, 6), &LUT_STD),
            ))
        }
        "volcano_lotus" => {
            let (g04, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x04)?;
            let vl = remap_16x16(&read_3bpp_16x16(&g04, 14, 4), &LUT_STD);
            Some((vl, flip_h_16x16(&vl)))
        }
        "boo" | "sewer_pack" => {
            let (g06, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x06)?;
            Some((
                remap_16x16(&read_3bpp_16x16(&g06, 12, 0), &LUT_BOO),
                remap_16x16(&read_3bpp_16x16(&g06, 14, 0), &LUT_BOO),
            ))
        }
        "swooper" => {
            let (g06, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x06)?;
            let sw = remap_16x16(&read_3bpp_16x16(&g06, 14, 4), &LUT_STD);
            Some((sw, flip_h_16x16(&sw)))
        }
        "cheep_cheep" => {
            let (g06, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x06)?;
            Some((
                remap_16x16(&read_3bpp_16x16(&g06, 10, 6), &LUT_STD),
                remap_16x16(&read_3bpp_16x16(&g06, 12, 6), &LUT_STD),
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
                remap_16x16(&read_3bpp_16x16(&g03, 0, 2), &LUT_MAGIK_AGA),
                remap_16x16(&read_3bpp_16x16(&g03, 2, 2), &LUT_MAGIK_AGA),
            ))
        }
        "bobomb" => {
            let (g02, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x02)?;
            let b = remap_16x16(&read_3bpp_16x16(&g02, 10, 4), &LUT_BOBOMB);
            Some((b, flip_h_16x16(&b)))
        }
        "bullet_bill" => {
            let (g02, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x02)?;
            let bb = remap_16x16(&read_3bpp_16x16(&g02, 2, 4), &LUT_BOBOMB);
            Some((bb, flip_h_16x16(&bb)))
        }
        "coin" => {
            let (g01, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x01)?;
            let c = remap_16x16(&read_3bpp_16x16(&g01, 8, 6), &LUT_GOLD);
            Some((c, c))
        }
        "smw_key" => {
            let (g01, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x01)?;
            let k = remap_16x16(&read_3bpp_16x16(&g01, 12, 6), &LUT_GOLD);
            Some((k, k))
        }
        "berry" => {
            let (g01, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x01)?;
            let br = remap_16x16(&read_3bpp_16x16(&g01, 0, 0), &LUT_MUSHROOM);
            Some((br, br))
        }
        "mushroom" => {
            let (g00, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x00)?;
            let m = remap_16x16(&read_3bpp_16x16(&g00, 4, 2), &LUT_MUSHROOM);
            Some((m, m))
        }
        "fire_flower" => {
            let (g00, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x00)?;
            let ff = remap_16x16(&read_3bpp_16x16(&g00, 6, 2), &LUT_FIRE_FLW);
            Some((ff, ff))
        }
        "starman" => {
            let (g00, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x00)?;
            let st = remap_16x16(&read_3bpp_16x16(&g00, 8, 4), &LUT_GOLD);
            Some((st, st))
        }
        "cape_feather" => {
            let (g00, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x00)?;
            let cf = remap_16x16(&read_3bpp_16x16(&g00, 14, 0), &LUT_GOLD);
            Some((cf, cf))
        }
        "p_switch" => {
            let (g00, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x00)?;
            let ps = remap_16x16(&read_3bpp_16x16(&g00, 2, 4), &LUT_STD);
            Some((ps, ps))
        }
        "yoshi_egg" => {
            let (g13, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x13)?;
            Some((
                remap_16x16(&read_3bpp_16x16(&g13, 0, 0), &LUT_YOSHI_EGG),
                remap_16x16(&read_3bpp_16x16(&g13, 2, 0), &LUT_YOSHI_EGG),
            ))
        }
        "moon_3up" => {
            let (g13, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x13)?;
            let m = remap_16x16(&read_3bpp_16x16(&g13, 4, 0), &LUT_GOLD);
            Some((m, m))
        }
        "baby_yoshi" => {
            let (g00, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x00)?;
            let by = remap_16x16(&read_3bpp_16x16(&g00, 14, 4), &[0, 1, 5, 7, 6, 1, 6, 7]);
            Some((by, flip_h_16x16(&by)))
        }
        "qblock" => {
            let (g00, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x00)?;
            let q = remap_16x16(&read_3bpp_16x16(&g00, 10, 2), &[0, 1, 2, 3, 7, 6, 6, 3]);
            Some((q, q))
        }
        "qblock_spr" => {
            let (g00, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x00)?;
            let q = remap_16x16(&read_3bpp_16x16(&g00, 10, 2), &LUT_QBLOCK_SPR);
            let used = remap_16x16(&read_3bpp_16x16(&g00, 14, 2), &LUT_QBLOCK_SPR);
            Some((q, used))
        }
        "turn_block" => {
            let (g00, _, _) = load_smw_3bpp_sheet(&donor_rom.data, 0x00)?;
            let tb = remap_16x16(&read_3bpp_16x16(&g00, 0, 4), &[0, 1, 2, 3, 7, 6, 6, 3]);
            Some((tb, tb))
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
                    description:
                        "Remplace Link (0x080000..0x087000 + Bank $0D OAM Y=-8 + palettes)".into(),
                },
                MixTargetSlot {
                    id: "soldiers".into(),
                    label: "Soldats Épée & Lance d'Hyrule (Sheet 0xBA)".into(),
                    category: MixSlotCategory::Soldiers,
                    description:
                        "Tous les soldats verts/bleus/rouges d'Hyrule et du château (mono-OAM)"
                            .into(),
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
            let z_pal =
                SnesPalette::zelda_link_full_16(&rom.data, 0x0DD308, "Link", false).colors_rgba;
            let link_head = read_4bpp_alttp_cell(&rom.data, 1, 0);
            let link_body = read_4bpp_alttp_cell(&rom.data, 3, 1);
            donor_sources.push(MixDonorSource {
                id: "link".into(),
                label: "Link (Zelda ALTTP — 16x32 complet)".into(),
                category: MixSlotCategory::Hero,
                compatible_targets: vec![MixSlotCategory::Hero],
                description: "Vrais sprites 4bpp de Link + palettes Tunique Verte/Bleue/Rouge"
                    .into(),
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
                description: "Titre SUPER LINK WORLD, sélection LINK A/B/C et histoire d'Hyrule"
                    .into(),
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
                    description: "Remplace Mario dans tous les niveaux et sur la carte du monde"
                        .into(),
                },
                MixTargetSlot {
                    id: "mount".into(),
                    label: "Monture Yoshi (GFX33 + Bébé Yoshi GFX10 + Palette)".into(),
                    category: MixSlotCategory::Mount,
                    description: "Remplace Yoshi par un autre compagnon".into(),
                },
                MixTargetSlot {
                    id: "enemies".into(),
                    label: "Ennemis SMW (Goombas & Koopas GFX01)".into(),
                    category: MixSlotCategory::Enemies,
                    description: "Remplace les ennemis principaux de GFX01 dans les niveaux SMW"
                        .into(),
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
                description:
                    "Vrais sprites Super Mario World non écrasés + vraies couleurs Rouge/Bleu"
                        .into(),
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
                description: "Combo Boos (chauves-souris), Spiny (rats) et Goombas (serpents)"
                    .into(),
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
            label: format!(
                "Palette #{} — {} ({} coul.)",
                idx, p.snes_addr, p.color_count
            ),
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
            label: format!(
                "Palette #{} ({} coul. @ {})",
                idx, p.color_count, p.snes_addr
            ),
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
            label: format!(
                "BRR #{} ({} ms, Loop:{})",
                idx, a.duration_ms_16khz, a.loops
            ),
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
                    MixRule {
                        target_slot: "hero".into(),
                        donor_alias: smw.alias.clone(),
                        donor_source: "mario".into(),
                    },
                    MixRule {
                        target_slot: "soldiers".into(),
                        donor_alias: smw.alias.clone(),
                        donor_source: "goomba".into(),
                    },
                    MixRule {
                        target_slot: "elite_knights".into(),
                        donor_alias: smw.alias.clone(),
                        donor_source: "spiny".into(),
                    },
                    MixRule {
                        target_slot: "castle_guards".into(),
                        donor_alias: smw.alias.clone(),
                        donor_source: "goomba".into(),
                    },
                    MixRule {
                        target_slot: "sewer_enemies".into(),
                        donor_alias: smw.alias.clone(),
                        donor_source: "sewer_pack".into(),
                    },
                    MixRule {
                        target_slot: "overworld_enemies".into(),
                        donor_alias: smw.alias.clone(),
                        donor_source: "overworld_pack".into(),
                    },
                    MixRule {
                        target_slot: "dungeon_enemies".into(),
                        donor_alias: smw.alias.clone(),
                        donor_source: "dungeon_pack".into(),
                    },
                    MixRule {
                        target_slot: "items".into(),
                        donor_alias: smw.alias.clone(),
                        donor_source: "items_pack".into(),
                    },
                    MixRule {
                        target_slot: "world_rocks".into(),
                        donor_alias: smw.alias.clone(),
                        donor_source: "qblock".into(),
                    },
                ];
                rules.push(MixRule {
                    target_slot: "audio_percussion".into(),
                    donor_alias: smw.alias.clone(),
                    donor_source: "percussion".into(),
                });
                presets.push(MixPreset {
                    id: "mario_hyrule_complete".into(),
                    name: "Super Mario Hyrule (Complet)".into(),
                    description: "Mario 16x32 (SMW) + Soldats Goombas + Spiny + Boos + Piranha + Pièces/Champignons/Bob-ombs + Percussions SMW".into(),
                    custom_title: "SUPER MARIO HYRULE".into(),
                    rules,
                });
            }
        }
        RecognizedGame::SuperMarioWorld => {
            if let Some(zelda) = find_donor(RecognizedGame::ZeldaAlttp) {
                presets.push(MixPreset {
                    id: "super_link_world".into(),
                    name: "Super Link World (Link dans SMW)".into(),
                    description:
                        "Héros Link (Zelda) + Titre Super Link World dans Super Mario World !"
                            .into(),
                    custom_title: "SUPER LINK WORLD".into(),
                    rules: vec![
                        MixRule {
                            target_slot: "hero".into(),
                            donor_alias: zelda.alias.clone(),
                            donor_source: "link".into(),
                        },
                        MixRule {
                            target_slot: "title_story".into(),
                            donor_alias: zelda.alias.clone(),
                            donor_source: "hyrule_theme".into(),
                        },
                    ],
                });
                presets.push(MixPreset {
                    id: "hyrule_mario_world".into(),
                    name: "Crossover Hyrule Mario World".into(),
                    description: "Link + Ennemis Soldats d'Hyrule + Rubis & Cœurs dans SMW !"
                        .into(),
                    custom_title: "HYRULE MARIO WORLD".into(),
                    rules: vec![
                        MixRule {
                            target_slot: "hero".into(),
                            donor_alias: zelda.alias.clone(),
                            donor_source: "link".into(),
                        },
                        MixRule {
                            target_slot: "enemies".into(),
                            donor_alias: zelda.alias.clone(),
                            donor_source: "zelda_soldier".into(),
                        },
                        MixRule {
                            target_slot: "items".into(),
                            donor_alias: zelda.alias.clone(),
                            donor_source: "zelda_items_pack".into(),
                        },
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
                custom_title: chaos
                    .custom_title
                    .unwrap_or_else(|| "OMNI MIXER SNES".into()),
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
        Self(if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        })
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

/// Canonical 18-rule recipe for **The Legend of Peach: A Mario to the Mushroom Kingdom**.
pub fn legend_of_peach_canonical_recipe(base_path: &str, smw_path: &str) -> MixRecipe {
    MixRecipe {
        name: "The Legend of Peach: A Mario to the Mushroom Kingdom".into(),
        custom_title: Some("THE LEGEND OF PEACH".into()),
        base_rom: base_path.into(),
        donors: vec![("smw".into(), smw_path.into())],
        rules: vec![
            MixRule {
                target_slot: "intro_title".into(),
                donor_alias: "smw".into(),
                donor_source: "peach_kingdom_intro".into(),
            },
            MixRule {
                target_slot: "hero".into(),
                donor_alias: "smw".into(),
                donor_source: "mario".into(),
            },
            MixRule {
                target_slot: "princess".into(),
                donor_alias: "smw".into(),
                donor_source: "peach".into(),
            },
            MixRule {
                target_slot: "uncle_allies".into(),
                donor_alias: "smw".into(),
                donor_source: "luigi_yoshi_pack".into(),
            },
            MixRule {
                target_slot: "bosses".into(),
                donor_alias: "smw".into(),
                donor_source: "bosses_pack".into(),
            },
            MixRule {
                target_slot: "soldiers".into(),
                donor_alias: "smw".into(),
                donor_source: "koopa".into(),
            },
            MixRule {
                target_slot: "castle_guards".into(),
                donor_alias: "smw".into(),
                donor_source: "goomba".into(),
            },
            MixRule {
                target_slot: "elite_knights".into(),
                donor_alias: "smw".into(),
                donor_source: "spiny".into(),
            },
            MixRule {
                target_slot: "sewer_enemies".into(),
                donor_alias: "smw".into(),
                donor_source: "sewer_pack".into(),
            },
            MixRule {
                target_slot: "overworld_enemies".into(),
                donor_alias: "smw".into(),
                donor_source: "overworld_pack".into(),
            },
            MixRule {
                target_slot: "zoras_water".into(),
                donor_alias: "smw".into(),
                donor_source: "water_pack".into(),
            },
            MixRule {
                target_slot: "dungeon_enemies".into(),
                donor_alias: "smw".into(),
                donor_source: "dungeon_pack".into(),
            },
            MixRule {
                target_slot: "dungeon_extra".into(),
                donor_alias: "smw".into(),
                donor_source: "dungeon_extra_pack".into(),
            },
            MixRule {
                target_slot: "items".into(),
                donor_alias: "smw".into(),
                donor_source: "items_pack".into(),
            },
            MixRule {
                target_slot: "chests_pots".into(),
                donor_alias: "smw".into(),
                donor_source: "blocks_pack".into(),
            },
            MixRule {
                target_slot: "rocks".into(),
                donor_alias: "smw".into(),
                donor_source: "qblock".into(),
            },
            MixRule {
                target_slot: "audio_percussion".into(),
                donor_alias: "smw".into(),
                donor_source: "percussion".into(),
            },
            MixRule {
                target_slot: "mario_music".into(),
                donor_alias: "smw".into(),
                donor_source: "nspc_soundtrack".into(),
            },
        ],
    }
}

/// Hardware integrity & patch-overlap audit report for a built Legend of Peach ROM.
#[derive(Debug, Clone)]
pub struct PeachBuildAudit {
    pub output_size_bytes: usize,
    pub output_sha256: String,
    pub checksum: u16,
    pub complement: u16,
    pub relocated_sheet_count: usize,
    pub verified_raw_sheets: usize,
}

/// Verify SNES LoROM compliance, reserved memory invariants, non-overlapping relocated expansion
/// banks (`$23..$3F`), and lossless LZ2 integrity of a built Legend of Peach ROM.
pub fn audit_legend_of_peach_output(
    orig_zelda: &SnesRom,
    built: &SnesRom,
    session: &SwapSession,
) -> Result<PeachBuildAudit, String> {
    if built.data.len() != 2_097_152 {
        return Err(format!(
            "Expected 2,097,152-byte 2 MiB LoROM, got {} bytes",
            built.data.len()
        ));
    }
    if built.data[0x007FD7] != 0x0B {
        return Err(format!(
            "Expected SNES ROM size byte 0x0B (2 MiB), got 0x{:02X}",
            built.data[0x007FD7]
        ));
    }
    if !built.header.is_checksum_valid() {
        return Err(format!(
            "Invalid SNES checksum: 0x{:04X} / complement 0x{:04X}",
            built.header.checksum, built.header.checksum_complement
        ));
    }
    if built.data[0x08A000..0x08A300] != orig_zelda.data[0x08A000..0x08A300] {
        return Err("Reserved Bow/Bill Blaster WRAM safety region 0x08A000..0x08A2FF was unexpectedly modified".into());
    }
    let expected_3d = crate::mario_intro::build_all_32_trio_3d_frames();
    if built.data[0x100000..0x110000] != expected_3d[..] {
        return Err(
            "3D Intro frame banks 0x100000..0x110000 failed bit-for-bit integrity check".into(),
        );
    }

    // Check relocated writes in expansion banks ($23..$3F = 0x118000..0x200000) for overlapping patches!
    let mut relocated_ranges: Vec<(usize, usize, String)> = Vec::new();
    for op in &session.operations {
        let start = op.target_offset_a;
        let end = start + op.replacement_bytes.len();
        if start >= 0x118000 && !op.replacement_bytes.is_empty() {
            if start / 0x8000 != (end - 1) / 0x8000 {
                return Err(format!(
                    "Relocated patch '{}' (0x{:06X}..0x{:06X}) crosses a 32 KiB LoROM bank boundary",
                    op.description, start, end
                ));
            }
            relocated_ranges.push((start, end, op.description.clone()));
        }
    }
    relocated_ranges.sort_by_key(|r| r.0);
    for pair in relocated_ranges.windows(2) {
        if pair[0].1 > pair[1].0 {
            return Err(format!(
                "Overlapping binary patches in expansion space: '{}' (0x{:06X}..0x{:06X}) overlaps '{}' (0x{:06X}..0x{:06X})",
                pair[0].2, pair[0].0, pair[0].1, pair[1].2, pair[1].0, pair[1].1
            ));
        }
    }

    // Verify all 52 v13/v14/v15/v0.9.0/v0.20.0 raw sheets decompress cleanly inside a single 32 KiB LoROM bank
    let (raw_bk, raw_hi, raw_lo) =
        find_alttp_raw_gfx_table(&built.data).ok_or("Could not locate ALTTP raw GFX table")?;
    let v13_raw_sheets: &[usize] = &[
        0xA4, 0xA3, 0xAD, 0xB1, 0xAC, 0xAB, 0xAE, 0xAF, 0xB0, 0xB2, 0xB3, 0x94, 0xB4, 0xB8, 0xA6,
        0xC2, 0xBD, 0xC0, 0xC1, 0xC3, 0xBF, 0x83, 0x85, 0x86, 0x89, 0x8A, 0x8B, 0x8C, 0x8D, 0x93,
        0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0x9D, 0x3F, 0x0D, 0x0B, 0x0C, 0x06, 0x18, 0x19, 0x4D,
        0x4C, 0x0F, 0x29, 0xCF, 0xA5, 0x3E, 0x53,
    ];
    for &raw_idx in v13_raw_sheets {
        let pc = ((built.data[raw_bk + raw_idx] as usize & 0x7F) << 15)
            | ((built.data[raw_hi + raw_idx] as usize & 0x7F) << 8)
            | (built.data[raw_lo + raw_idx] as usize);
        if (0x100000..0x118000).contains(&pc) {
            return Err(format!(
                "Raw sheet 0x{raw_idx:02X} intrudes on reserved 3D Intro banks at 0x{pc:06X}"
            ));
        }
        let (dec, clen) = decompress_nintendo_lz2_with_endian(&built.data, pc, 4096, false)
            .ok_or_else(|| {
                format!("Raw sheet 0x{raw_idx:02X} at 0x{pc:06X} failed LC_LZ2 decompression")
            })?;
        if dec.len() != 0x600 {
            return Err(format!(
                "Raw sheet 0x{raw_idx:02X} decompressed to {} bytes instead of 1536",
                dec.len()
            ));
        }
        if pc / 0x8000 != (pc + clen - 1) / 0x8000 {
            return Err(format!(
                "Raw sheet 0x{raw_idx:02X} at 0x{pc:06X} crosses a 32 KiB LoROM bank boundary"
            ));
        }
    }

    Ok(PeachBuildAudit {
        output_size_bytes: built.data.len(),
        output_sha256: crate::rom::compute_sha256_hex(&built.data),
        checksum: built.header.checksum,
        complement: built.header.checksum_complement,
        relocated_sheet_count: relocated_ranges.len(),
        verified_raw_sheets: v13_raw_sheets.len(),
    })
}

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
            if rule.donor_source == "mario"
                && donor.identify_game() == RecognizedGame::SuperMarioWorld
            {
                // Inject Mario hero + Bank $0D OAM + Palettes (without overwriting other sheets yet;
                // use dedicated hero-only injector so user rules control soldiers/items independently!)
                ops += inject_smw_mario_hero_into_zelda(rom_zelda, donor, session)?;
            } else if rule.donor_source == "link"
                && donor.identify_game() == RecognizedGame::ZeldaAlttp
            {
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
            if rule.donor_source == "koopa" {
                // Fix #11: Museum-Grade Upright SMW Koopa Troopa Warrior (Sheet 0xBA) + Guard OAM Overhaul!
                // Patched OAM ($05:C6A4 + $05:C727..$05:C9F8) draws ONE centered 16x16 body per frame
                // with 3-step smooth walking animation in all 4 directions and head perched 11-12px above shell!
                if let Some(kp) = crate::mario_ui::build_smw_koopa_warrior_parts(donor) {
                    let blank8 = [[0u8; 8]; 8];
                    if patch_alttp_comp_sheet_helper(
                        rom_zelda,
                        session,
                        0xBA,
                        &format!(
                            "Soldats d'Hyrule (Sheet 0xBA) ← {}:Guerrier Koopa Troopa SMW complet (4 directions)",
                            rule.donor_alias
                        ),
                        free_cursor,
                        free_end,
                        &|raw| {
                            // Heads (16x16 Top OAM): Front (0,0), Side Left (2,0), Back (4,0)
                            write_3bpp_16x16(raw, 0, 0, &kp.head_front);
                            write_3bpp_16x16(raw, 2, 0, &kp.head_side);
                            write_3bpp_16x16(raw, 4, 0, &kp.head_back);
                            // Front Bodies (Down): Stride L/R (6,0), Neutral (8,0)
                            write_3bpp_16x16(raw, 6, 0, &kp.body_front_1);
                            write_3bpp_16x16(raw, 8, 0, &kp.body_front_2);
                            // Side Bodies (Left / H-flipped Right — 3-step walk!): (14,0), (0,2), (2,2)
                            write_3bpp_16x16(raw, 14, 0, &kp.body_side_1);
                            write_3bpp_16x16(raw, 0, 2, &kp.body_side_2);
                            write_3bpp_16x16(raw, 2, 2, &kp.body_side_3);
                            // Back Bodies (Up): Stride L/R (4,2), Neutral (6,2)
                            write_3bpp_16x16(raw, 4, 2, &kp.body_back_1);
                            write_3bpp_16x16(raw, 6, 2, &kp.body_back_2);
                            // Explicitly blank 0x4D (13,0) used by OAM slots 1..3 + unused vanilla overhangs/shields
                            for &(bx, by) in &[
                                (10, 0), (10, 1), (11, 0), (11, 1), (12, 0), (12, 1),
                                (13, 0), (13, 1), (8, 2), (8, 3), (13, 2), (13, 3),
                            ] {
                                write_3bpp_8x8(raw, bx, by, &blank8);
                            }
                            // v0.7.0 (ZRI-016 ELIMINATED!): Replace ALL Koopa Guard medieval Zelda swords & spears
                            // in raw 0xBA (`$6B/$7B` vert @ (11,2)/(11,3), `$6C/$7C` horiz @ (12,2)/(12,3),
                            // `$6E/$7E/$6F/$7F` spear guards @ cols 14..15 rows 2..3) with authentic
                            // SMW Hammer Bro Hammers (glossy black iron head + white shine + wooden handle)
                            // and SMW Spiked Koopa Clubs!
                            let parse8 = |rows: [&str; 8]| -> [[u8; 8]; 8] {
                                let mut px = [[0u8; 8]; 8];
                                for (y, r) in rows.iter().enumerate() {
                                    for (x, ch) in r.bytes().take(8).enumerate() {
                                        px[y][x] = if ch == b'.' { 0 } else { ch - b'0' };
                                    }
                                }
                                px
                            };
                            // Vertical SMW Hammer Head (`$6B` at (11,2) — outer striking end!)
                            let h_vert_head = parse8([
                                ".555555.",
                                "55115555",
                                "51155555",
                                "55555555",
                                "55555555",
                                ".555555.",
                                "..5435..",
                                "..5435..",
                            ]);
                            // Vertical Wooden Handle (`$7B` at (11,3) — gripped in Koopa hand!)
                            let h_vert_handle = parse8([
                                "..5435..",
                                "..5435..",
                                "..5435..",
                                "..5435..",
                                "..5435..",
                                "..5335..",
                                "...55...",
                                "........",
                            ]);
                            // Horizontal SMW Hammer Head (`$6C` at (12,2) — outer striking end!)
                            let h_horiz_head = parse8([
                                ".55555..",
                                "5511555.",
                                "51155555",
                                "55555435",
                                "55555435",
                                "55555555",
                                ".55555..",
                                "........",
                            ]);
                            // Horizontal Wooden Handle (`$7C` at (12,3) — gripped in Koopa hand!)
                            let h_horiz_handle = parse8([
                                "........",
                                "........",
                                "555555..",
                                "4444435.",
                                "3333335.",
                                "555555..",
                                "........",
                                "........",
                            ]);
                            // Spiked Koopa Club Head (`$6E` vert / `$6F` horiz — for Elite Red/Green Spear Koopas!)
                            let club_vert_head = parse8([
                                "...1....",
                                ".15551..",
                                "5551555.",
                                "5515555.",
                                "5555555.",
                                ".15551..",
                                "..5435..",
                                "..5435..",
                            ]);
                            let club_horiz_head = parse8([
                                "..1..1..",
                                ".55555..",
                                "15155555",
                                "55555435",
                                "55555435",
                                "15555555",
                                ".55555..",
                                "..1..1..",
                            ]);
                            write_3bpp_8x8(raw, 11, 2, &h_vert_head);
                            write_3bpp_8x8(raw, 11, 3, &h_vert_handle);
                            write_3bpp_8x8(raw, 12, 2, &h_horiz_head);
                            write_3bpp_8x8(raw, 12, 3, &h_horiz_handle);
                            write_3bpp_8x8(raw, 14, 2, &club_vert_head);
                            write_3bpp_8x8(raw, 14, 3, &h_vert_handle);
                            write_3bpp_8x8(raw, 15, 2, &club_horiz_head);
                            write_3bpp_8x8(raw, 15, 3, &h_horiz_handle);
                        },
                    )? {
                        ops += 1;
                    }
                    ops += crate::mario_ui::patch_koopa_guard_oam_tables(rom_zelda, session)?;
                }
            } else if let Some((sp1, sp2)) =
                extract_donor_16x16_for_zelda(donor, &rule.donor_source, true)
            {
                let sp1_r = flip_h_16x16(&sp1);
                let blank16 = [[0u8; 16]; 16];
                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0xBA,
                    &format!(
                        "Soldats d'Hyrule (Sheet 0xBA) ← {}:{}",
                        rule.donor_alias, rule.donor_source
                    ),
                    free_cursor,
                    free_end,
                    &|raw| {
                        // Single-OAM Head slots (never doubled, never blinking!)
                        write_3bpp_16x16(raw, 0, 0, &sp1);
                        write_3bpp_16x16(raw, 2, 0, &sp2);
                        write_3bpp_16x16(raw, 4, 0, &sp1_r);
                        for &(tx, ty) in &[
                            (6, 0),
                            (8, 0),
                            (10, 0),
                            (12, 0),
                            (14, 0),
                            (0, 2),
                            (2, 2),
                            (4, 2),
                            (6, 2),
                            (8, 2),
                            (10, 2),
                        ] {
                            write_3bpp_16x16(raw, tx, ty, &blank16);
                        }
                    },
                )? {
                    ops += 1;
                }
            }
        }
        "elite_knights" => {
            // Fix #3B: Dungeon Jailer Guarding Princess Peach (Ball & Chain Trooper `Sprite 0x6A`, Sheet 0xB7)
            // + Red Spear / Charging Blue Spear / Bomb Guards -> Armored Heavy Koopa Captain swinging a
            // fierce SMW Chain Chomp Flail (`tile $2A`), with clean single-centered OAM ($05:B16E..$05:B395)!
            let cap = crate::mario_ui::build_ball_and_chain_koopa_captain();
            let blank8 = [[0u8; 8]; 8];
            if patch_alttp_comp_sheet_helper(
                rom_zelda,
                session,
                0xB7,
                &format!(
                    "Boss Geôlier & Gardes Élite (Sheet 0xB7) ← {}:Capitaine Koopa Cuirassé + Chomp en Chaîne SMW",
                    rule.donor_alias
                ),
                free_cursor,
                free_end,
                &|raw| {
                    // Heads: Front (0,0), Side Left (2,0), Back (4,0)
                    write_3bpp_16x16(raw, 0, 0, &cap.head_front);
                    write_3bpp_16x16(raw, 2, 0, &cap.head_side);
                    write_3bpp_16x16(raw, 4, 0, &cap.head_back);
                    // Bodies strictly within Sheet 0xB7 ($00..$3F)
                    write_3bpp_16x16(raw, 6, 0, &cap.body_front_idle);
                    write_3bpp_16x16(raw, 8, 0, &cap.body_side_1);
                    write_3bpp_16x16(raw, 10, 0, &cap.body_back_idle);
                    write_3bpp_16x16(raw, 12, 0, &cap.body_front_stride);
                    write_3bpp_16x16(raw, 2, 2, &cap.body_side_2);
                    write_3bpp_16x16(raw, 12, 2, &cap.body_back_stride);
                    // Fierce SMW Chain Chomp flail ball (`tile $2A` = (10,2))! Keep iron chain link `tile $3F` = (15,3) intact!
                    write_3bpp_16x16(raw, 10, 2, &cap.chain_chomp);
                    // Transparent blank tile 0x0E at (14,0) used for unused OAM slots 1..2
                    write_3bpp_8x8(raw, 14, 0, &blank8);
                    write_3bpp_8x8(raw, 14, 1, &blank8);
                },
            )? {
                ops += 1;
            }
            ops += crate::mario_ui::patch_ball_and_chain_oam_tables(rom_zelda, session)?;
        }
        "castle_guards" => {
            if let Some((sp1, sp2)) = extract_donor_16x16_for_zelda(donor, &rule.donor_source, true)
            {
                let sp1_r = flip_h_16x16(&sp1);
                let sp2_r = flip_h_16x16(&sp2);
                let blank16 = [[0u8; 16]; 16];
                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0x8C,
                    &format!(
                        "Gardes du Château (Sheet 0x8C) ← {}:{}",
                        rule.donor_alias, rule.donor_source
                    ),
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
        "princess" => {
            // Fix #9: 100% Centered, Symmetric, Unclipped SMW Princess Peach (ZERO flat vertical cut!)
            let (peach_h, peach_d, peach_h_back, peach_h_side) =
                crate::mario_ui::build_unclipped_smw_peach();
            let _ = build_smw_peach_for_zelda;
            {
                let peach_d_alt = flip_h_16x16(&peach_d);
                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0xD5,
                    &format!(
                        "Princesse Zelda (Sheet 0xD5) ← {}:Princesse Peach 16x32 centrée complète",
                        rule.donor_alias
                    ),
                    free_cursor,
                    free_end,
                    &|raw| {
                        write_3bpp_16x16(raw, 0, 0, &peach_h); // Front Head
                        write_3bpp_16x16(raw, 2, 0, &peach_h_back); // Back Head
                        write_3bpp_16x16(raw, 4, 0, &peach_h_side); // Side Head
                        write_3bpp_16x16(raw, 6, 0, &peach_d); // Walk/Stand Dress 1
                        write_3bpp_16x16(raw, 8, 0, &peach_d_alt); // Walk/Stand Dress 2
                        write_3bpp_16x16(raw, 10, 0, &peach_d); // Front Dress (Sanctuary!)
                        write_3bpp_16x16(raw, 12, 0, &peach_d_alt); // Front Dress Alt
                    },
                )? {
                    ops += 1;
                }
                // Intro Scene 3 Prison Cell Princess Peach + Intro Scene 4 Altar Captured Green Yoshi!
                // Verified via jpdasm bank_0C.asm:13486 (`$0AA3 = $7F` -> `Sheet 0xC6`) + `Attract_DrawZelda` (`$28/$2A`)
                // + `AttractAltarMaidenOAM` (`$00/$01` levitating & `$03/$04` resting = 24x16 on Pal 6 GREEN!)
                let altar_yoshi = crate::mario_ui::build_captured_yoshi_altar_24x16();
                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0xC6,
                    &format!("Cellule Intro & Autel (Sheet 0xC6) ← {}:Princesse Peach & Yoshi Vert capturé", rule.donor_alias),
                    free_cursor,
                    free_end,
                    &|raw| {
                        write_3bpp_16x16(raw, 8, 2, &peach_h);
                        write_3bpp_16x16(raw, 10, 2, &peach_d);
                        // Write 24x16 Captured Green Yoshi across cols 0..2 (step1) & cols 3..5 (step0), rows 0..1!
                        for base_col in [0usize, 3usize] {
                            for ty in 0..2usize {
                                for tx in 0..3usize {
                                    let mut tile = [[0u8; 8]; 8];
                                    for y in 0..8 {
                                        for x in 0..8 {
                                            tile[y][x] = altar_yoshi[ty * 8 + y][tx * 8 + x];
                                        }
                                    }
                                    write_3bpp_8x8(raw, base_col + tx, ty, &tile);
                                }
                            }
                        }
                    },
                )? {
                    ops += 1;
                }
                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0xD6,
                    &format!(
                        "Demoiselles & Zelda (Sheet 0xD6) ← {}:Princesse Peach 16x32 complète",
                        rule.donor_alias
                    ),
                    free_cursor,
                    free_end,
                    &|raw| {
                        write_3bpp_16x16(raw, 0, 0, &peach_h);
                        write_3bpp_16x16(raw, 2, 0, &peach_h_back);
                        write_3bpp_16x16(raw, 4, 0, &peach_h_side);
                        write_3bpp_16x16(raw, 6, 0, &peach_d);
                        write_3bpp_16x16(raw, 8, 0, &peach_d_alt);
                        write_3bpp_16x16(raw, 10, 0, &peach_d);
                        write_3bpp_16x16(raw, 12, 0, &peach_d_alt);
                        write_3bpp_16x16(raw, 0, 2, &peach_h);
                        write_3bpp_16x16(raw, 2, 2, &peach_h_back);
                        write_3bpp_16x16(raw, 4, 2, &peach_h_side);
                        write_3bpp_16x16(raw, 6, 2, &peach_d);
                        write_3bpp_16x16(raw, 8, 2, &peach_d_alt);
                        write_3bpp_16x16(raw, 10, 2, &peach_d);
                        write_3bpp_16x16(raw, 12, 2, &peach_d_alt);
                    },
                )? {
                    ops += 1;
                }
                // Zero-Conflict SP3 Palette (`0x0DD272` & `0x0DD2EA`):
                //   1=White (0x7FFF)
                //   2=Vibrant Princess Peach Rose-Pink Gown (0x5A3F) -- NEVER used by Blue Koopa!
                //   3=Golden Hair / Koopa Snout Shade (0x21FA)
                //   4=Blonde / Peach Skin / Koopa Snout (0x435F)
                //   5=Charcoal Outline (0x18C6)
                //   6=Vibrant SMW Sky Blue (0x7E48) -- Blue Koopa Shell!
                //   7=Deep SMW Royal Blue (0x5520)  -- Blue Koopa Dark Shell & Peach Sapphire Eyes/Brooch!
                // Gives BOTH a Bright Pink Princess Peach AND Authentic Vibrant Blue Koopa Warriors!
                let sp3_pal_14: Vec<u8> =
                    [0x7FFFu16, 0x5A3F, 0x21FA, 0x435F, 0x18C6, 0x7E48, 0x5520]
                        .iter()
                        .flat_map(|w| w.to_le_bytes())
                        .collect();
                for &pal_pc in &[0x0DD272usize, 0x0DD2EA] {
                    if pal_pc + 14 <= rom_zelda.data.len() {
                        session.swap_custom_bytes(
                            rom_zelda,
                            pal_pc,
                            pal_pc,
                            sp3_pal_14.clone(),
                            format!("Palette SP3 0x{:06X} ← Rose Royal Peach (idx 2) + Bleu Royal Koopa SMW (idx 6/7)", pal_pc),
                            SwapKind::PaletteBgr555 { color_count: 7 },
                        )?;
                        ops += 1;
                    }
                }
            }
        }
        "uncle_allies" => {
            // 1. Fix #4/#5: Normal SINGLE-HEADED Uncle Luigi built pixel-for-pixel from MARIO'S EXACT 4bpp BANK
            //    (`0x080000..0x087000`) remapped into Luigi's iconic GREEN clothes (Pal 6 `0x0C` on Head & Body!)
            let _ = build_smw_luigi_3bpp;
            let luigi = if 0x087000 <= rom_zelda.data.len() {
                crate::mario_ui::build_mario_green_uncle_luigi_from_bank(
                    &rom_zelda.data[0x080000..0x087000],
                )
            } else {
                crate::mario_ui::build_normal_uncle_luigi()
            };
            let (baby_y1, _) = extract_donor_16x16_for_zelda(donor, "baby_yoshi", false)
                .unwrap_or((luigi.body_sit, luigi.body_sit));
            if patch_alttp_comp_sheet_helper(
                rom_zelda,
                session,
                0x7F,
                &format!(
                    "Oncle de Link & Poules (Sheet 0x7F) ← {}:Luigi normal & Bébé Yoshi",
                    rule.donor_alias
                ),
                free_cursor,
                free_end,
                &|raw| {
                    write_3bpp_16x16(raw, 0, 0, &luigi.head_front);
                    write_3bpp_16x16(raw, 2, 0, &luigi.head_side);
                    write_3bpp_16x16(raw, 4, 0, &luigi.head_hurt);
                    write_3bpp_16x16(raw, 0, 2, &luigi.body_sit);
                    write_3bpp_16x16(raw, 2, 2, &luigi.body_walk_side_1);
                    write_3bpp_16x16(raw, 4, 2, &luigi.body_walk_down);
                    write_3bpp_16x16(raw, 6, 0, &baby_y1); // Cucco -> Bébé Yoshi
                },
            )? {
                ops += 1;
            }
            if patch_alttp_comp_sheet_helper(
                rom_zelda,
                session,
                0xC2,
                &format!("Oncle Maison & Égouts (Sheet 0xC2) ← {}:Luigi SMW normal (1 tête, salopette bleue)", rule.donor_alias),
                free_cursor,
                free_end,
                &|raw| {
                    // Heads (Pal 6 GREEN CAP): (0,0) Front, (2,0) Side, (8,0) Sewers
                    write_3bpp_16x16(raw, 0, 0, &luigi.head_front);
                    write_3bpp_16x16(raw, 2, 0, &luigi.head_side);
                    write_3bpp_16x16(raw, 8, 0, &luigi.head_hurt);
                    // Bodies (Pal 7 ROYAL BLUE OVERALLS — ZERO SECOND HEAD!):
                    write_3bpp_16x16(raw, 4, 0, &luigi.body_walk_down);
                    write_3bpp_16x16(raw, 6, 0, &luigi.body_sit);
                    write_3bpp_16x16(raw, 0, 2, &luigi.body_walk_side_1);
                    write_3bpp_16x16(raw, 4, 2, &luigi.body_walk_side_2);
                    write_3bpp_16x16(raw, 8, 2, &luigi.body_sit);
                },
            )? {
                ops += 1;
            }
            ops += crate::mario_ui::patch_uncle_luigi_oam_palettes(rom_zelda, session)?;
            // 2. Sahasrahla (S 0xA5), Sage du Sanctuaire & Roi (S 0xA6), Grande Fée (S 0xAA) -> Authentiques SMW Yoshis!
            if let Some((y_fh, y_fb, y_sh, y_sb, y_bh, y_bb)) = build_smw_yoshi_for_zelda(donor) {
                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0xA5,
                    &format!(
                        "Sahasrahla (Sheet 0xA5) ← {}:Yoshi Vert (GFX26)",
                        rule.donor_alias
                    ),
                    free_cursor,
                    free_end,
                    &|raw| {
                        write_3bpp_16x16(raw, 0, 0, &y_fh);
                        write_3bpp_16x16(raw, 0, 2, &y_fb);
                        write_3bpp_16x16(raw, 2, 0, &y_sh);
                        write_3bpp_16x16(raw, 2, 2, &y_sb);
                        write_3bpp_16x16(raw, 4, 0, &y_bh);
                        write_3bpp_16x16(raw, 4, 2, &y_bb);
                    },
                )? {
                    ops += 1;
                }
                let toad = crate::mario_ui::build_smw_toad_sanctuary();
                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0xA6,
                    &format!(
                        "Sage Sanctuaire (Sheet 0xA6) ← {}:Toad (Kinoppio) complet 16x32",
                        rule.donor_alias
                    ),
                    free_cursor,
                    free_end,
                    &|raw| {
                        write_3bpp_16x16(raw, 0, 0, &toad.head_front);
                        write_3bpp_16x16(raw, 0, 2, &toad.body_front);
                        write_3bpp_16x16(raw, 2, 0, &toad.head_blink);
                        write_3bpp_16x16(raw, 2, 2, &toad.body_cheer);
                        write_3bpp_16x16(raw, 4, 0, &toad.head_side);
                        write_3bpp_16x16(raw, 4, 2, &toad.body_side);
                        write_3bpp_16x16(raw, 14, 0, &toad.head_side);
                        write_3bpp_16x16(raw, 14, 2, &toad.body_side);
                    },
                )? {
                    ops += 1;
                }
                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0xAA,
                    &format!(
                        "Grande Fée (Sheet 0xAA) ← {}:Yoshi Ailé (GFX26)",
                        rule.donor_alias
                    ),
                    free_cursor,
                    free_end,
                    &|raw| {
                        write_3bpp_16x16(raw, 10, 0, &y_fh);
                        write_3bpp_16x16(raw, 10, 2, &y_fb);
                        write_3bpp_16x16(raw, 12, 0, &y_sh);
                        write_3bpp_16x16(raw, 12, 2, &y_sb);
                        write_3bpp_16x16(raw, 14, 0, &y_fh);
                        write_3bpp_16x16(raw, 14, 2, &y_fb);
                    },
                )? {
                    ops += 1;
                }
            }
            // 3. Poules Cuccos (S 0x86) -> Bébés Yoshis SMW, Marchand Kamek (S 0xBC / raw 0xBE cols 0..1 rows 0..2 ONLY,
            //    preserving Shop Price Digits '0' & '1' at row 3 cols 0..1 intact!), Forgerons (S 0xD7) -> Koopas!
            if patch_alttp_comp_sheet_helper(
                rom_zelda,
                session,
                0x86,
                &format!(
                    "Poules Cuccos Cocorico (Sheet 0x86) ← {}:Bébés Yoshis SMW complets",
                    rule.donor_alias
                ),
                free_cursor,
                free_end,
                &|raw| {
                    crate::mario_ui::patch_smw_baby_yoshi_sheet_86(raw);
                },
            )? {
                ops += 1;
            }
            {
                let (km_head, km_body0) = crate::mario_world::build_kamek_magikoopa_smw_16x16(0);
                let (_, km_body1) = crate::mario_world::build_kamek_magikoopa_smw_16x16(1);
                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0xBC,
                    &format!(
                        "Marchand Kamek Boutique (Sheet 0xBC, Bleu SMW, chiffres '0'..'9' 100% isolés!) ← {}:Magikoopa SMW",
                        rule.donor_alias
                    ),
                    free_cursor,
                    free_end,
                    &|raw| {
                        // Head in cols 0..1 rows 0..1 (tile $00);
                        // Clean 16x16 Royal Blue Robe frames in cols 6..7 rows 0..1 (tile $06) & rows 2..3 (tile $26)!
                        // Shop Price Digits '0'..'9' at cols 0..1 row 3 & cols 2..3 rows 0..3 remain 100% UNTOUCHED!
                        write_3bpp_16x16(raw, 0, 0, &km_head);
                        write_3bpp_16x16(raw, 6, 0, &km_body0);
                        write_3bpp_16x16(raw, 6, 2, &km_body1);
                    },
                )? {
                    ops += 1;
                }
            }
            if let Some((k1, k2)) = extract_donor_16x16_for_zelda(donor, "koopa", false) {
                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0xD7,
                    &format!(
                        "Forgerons Nains (Sheet 0xD7) ← {}:Deux Koopas SMW",
                        rule.donor_alias
                    ),
                    free_cursor,
                    free_end,
                    &|raw| {
                        write_3bpp_16x16(raw, 6, 0, &k1);
                        write_3bpp_16x16(raw, 8, 0, &k2);
                        write_3bpp_16x16(raw, 10, 0, &flip_h_16x16(&k1));
                        write_3bpp_16x16(raw, 12, 0, &flip_h_16x16(&k2));
                        write_3bpp_16x16(raw, 6, 2, &k1);
                        write_3bpp_16x16(raw, 8, 2, &k2);
                        write_3bpp_16x16(raw, 10, 2, &flip_h_16x16(&k1));
                        write_3bpp_16x16(raw, 12, 2, &flip_h_16x16(&k2));
                    },
                )? {
                    ops += 1;
                }
            }
        }
        "bosses" => {
            // 1. Complete SMW Boss Roster (v13): ALL 7 KOOPALINGS (Morton Jr., Wendy, Iggy, Roy,
            //    Lemmy, Larry, Ludwig) + FINAL BOSS GIANT KING BOWSER (4 sheets!) + 3 Pokeys + Wiggler!
            ops += crate::mario_bosses::apply_all_mario_bosses(
                rom_zelda,
                donor,
                session,
                free_cursor,
                free_end,
            )?;
            // 2. Boss Guarding Princess Peach (Agahnim 1 & 2 -> Symmetric 32x32 King Bowser on Sheets 0xB3 & 0xB4!)
            //    Verified $1E:D77F OAM layout:
            //      F00 Front Idle (0xB3 (2,0)/(2,2) H-mirrored), F01 Front Cast ((0,0)/(0,2) H-mirrored),
            //      F02 Back Idle ((4,0)/(4,2) H-mirrored), F03 Back Cast ((6,0)/(6,2) H-mirrored),
            //      F04/F05 Side Left (0xB3 (8,0)..(F,3)), F06/F07 Side Cast (0xB4 (0,0)..(7,3))!
            let bow = crate::mario_ui::build_king_bowser_for_agahnim();
            if patch_alttp_comp_sheet_helper(
                rom_zelda,
                session,
                0xB3,
                &format!(
                    "Boss Gardien de Peach (Sheet 0xB3) ← {}:Roi Bowser 32x32 symétrique",
                    rule.donor_alias
                ),
                free_cursor,
                free_end,
                &|raw| {
                    // Front Cast (Left half H-mirrored to 32x32)
                    write_3bpp_16x16(raw, 0, 0, &bow.front_cast_tl);
                    write_3bpp_16x16(raw, 0, 2, &bow.front_cast_bl);
                    // Front Idle guarding Peach (Left half H-mirrored to 32x32!)
                    write_3bpp_16x16(raw, 2, 0, &bow.front_idle_tl);
                    write_3bpp_16x16(raw, 2, 2, &bow.front_idle_bl);
                    // Back Idle spiky shell (Left half H-mirrored to 32x32)
                    write_3bpp_16x16(raw, 4, 0, &bow.back_idle_tl);
                    write_3bpp_16x16(raw, 4, 2, &bow.back_idle_bl);
                    // Back Cast (Left half H-mirrored to 32x32)
                    write_3bpp_16x16(raw, 6, 0, &bow.back_cast_tl);
                    write_3bpp_16x16(raw, 6, 2, &bow.back_cast_bl);
                    // Side Profile 1 & 2 (Full 32x32)
                    write_3bpp_16x16(raw, 8, 0, &bow.side_tl);
                    write_3bpp_16x16(raw, 10, 0, &bow.side_tr);
                    write_3bpp_16x16(raw, 8, 2, &bow.side_bl);
                    write_3bpp_16x16(raw, 10, 2, &bow.side_br);
                    write_3bpp_16x16(raw, 12, 0, &bow.side_tl);
                    write_3bpp_16x16(raw, 14, 0, &bow.side_tr);
                    write_3bpp_16x16(raw, 12, 2, &bow.side_bl);
                    write_3bpp_16x16(raw, 14, 2, &bow.side_br);
                },
            )? {
                ops += 1;
            }
            if patch_alttp_comp_sheet_helper(
                rom_zelda,
                session,
                0xB4,
                &format!(
                    "Boss Gardien de Peach Attaque (Sheet 0xB4) ← {}:Roi Bowser 32x32",
                    rule.donor_alias
                ),
                free_cursor,
                free_end,
                &|raw| {
                    // Verified AttractAgahnimOAM .char_step1 ($C4=TL, $C2=TR, $E4=BL, $E6=BR) + .step2 ($C0/$C2/$E0/$E2)
                    // + Agahnim teleport/cast silhouette at cols 8..11 (zero vanilla Agahnim cape tiles left!):
                    write_3bpp_16x16(raw, 0, 0, &bow.side_tl);
                    write_3bpp_16x16(raw, 2, 0, &bow.side_tr);
                    write_3bpp_16x16(raw, 0, 2, &bow.side_bl);
                    write_3bpp_16x16(raw, 2, 2, &bow.side_br);
                    write_3bpp_16x16(raw, 4, 0, &bow.side_tl);
                    write_3bpp_16x16(raw, 6, 0, &bow.side_tr);
                    write_3bpp_16x16(raw, 4, 2, &bow.side_bl);
                    write_3bpp_16x16(raw, 6, 2, &bow.side_br);
                    write_3bpp_16x16(raw, 8, 0, &bow.side_tl);
                    write_3bpp_16x16(raw, 10, 0, &bow.side_tr);
                    write_3bpp_16x16(raw, 8, 2, &bow.side_bl);
                    write_3bpp_16x16(raw, 10, 2, &bow.side_br);
                },
            )? {
                ops += 1;
            }
            ops += crate::mario_ui::patch_bowser_authentic_palettes(rom_zelda, session)?;
        }
        "chests_pots" => {
            // Coffres (S 0xC9 & S 0xCE) -> Blocs '?' & Blocs vides + Fées -> Starman + Super Bombe -> Bob-omb!
            if let (Some((qb_spr, used_spr)), Some((star, _)), Some((bob, _))) = (
                extract_donor_16x16_for_zelda(donor, "qblock_spr", false),
                extract_donor_16x16_for_zelda(donor, "starman", false),
                extract_donor_16x16_for_zelda(donor, "bobomb", false),
            ) {
                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0xC9,
                    &format!(
                        "Coffres, Fées & Super Bombe (Sheet 0xC9) ← {}:Blocs ?, Starman & Bob-omb",
                        rule.donor_alias
                    ),
                    free_cursor,
                    free_end,
                    &|raw| {
                        write_3bpp_16x16(raw, 0, 0, &star); // Petite Fée 1 -> Starman
                        write_3bpp_16x16(raw, 2, 0, &star); // Petite Fée 2 -> Starman
                        write_3bpp_16x16(raw, 4, 0, &star); // Petite Fée 3 -> Starman
                        write_3bpp_16x16(raw, 6, 0, &qb_spr); // Coffre fermé 1 -> Bloc '?'
                        write_3bpp_16x16(raw, 8, 0, &qb_spr); // Coffre fermé 2 -> Bloc '?'
                        write_3bpp_16x16(raw, 10, 0, &used_spr); // Coffre ouvert 1 -> Bloc vidé SMW
                        write_3bpp_16x16(raw, 12, 0, &used_spr); // Coffre ouvert 2 -> Bloc vidé SMW
                        write_3bpp_16x16(raw, 6, 2, &bob); // Super Bombe -> Bob-omb
                        write_3bpp_16x16(raw, 8, 2, &bob);
                        write_3bpp_16x16(raw, 10, 2, &bob);
                        write_3bpp_16x16(raw, 12, 2, &bob);
                    },
                )? {
                    ops += 1;
                }
                let (ff, _) = extract_donor_16x16_for_zelda(donor, "fire_flower", false)
                    .unwrap_or((star, star));
                let mush = crate::mario_ui::build_crisp_smw_mushroom_16x16();
                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0xCE,
                    &format!("Coffres & Pendentifs Boss (Sheet 0xCE) ← {}:Blocs ?, Champignon, Fleur de Feu & Étoile", rule.donor_alias),
                    free_cursor,
                    free_end,
                    &|raw| {
                        write_3bpp_16x16(raw, 8, 0, &qb_spr);  // Grand Coffre -> Bloc '?'
                        write_3bpp_16x16(raw, 0, 2, &qb_spr);  // Coffre fermé -> Bloc '?'
                        write_3bpp_16x16(raw, 2, 2, &used_spr);// Coffre ouvert -> Bloc vidé
                        write_3bpp_16x16(raw, 4, 0, &mush);    // VRAI Pendentif Boss (gfx 0x23) -> Super Champignon SMW!
                        write_3bpp_16x16(raw, 6, 0, &ff);      // Fleur de Feu SMW
                        write_3bpp_16x16(raw, 6, 2, &star);    // Étoile Super Mario SMW!
                        write_3bpp_16x16(raw, 8, 2, &mush);    // Super Champignon SMW
                    },
                )? {
                    ops += 1;
                }
            }
            // Fix #1: VRAIES Jarres & Pots d'intérieur (Maison de Mario + TOUS les Donjons!)
            // Vérifié en VRAM live Mednafen : Sheet 0x0C tiles (8,2) haut-gauche & (8,3) bas-gauche (miroir H par ALTTP!)
            // (Sheet 0x21 (8,0)/(9,0) était les tabourets de la table -> laissés en bois intact!)
            if let Some((g00, _, _)) = load_smw_3bpp_sheet(&donor.data, 0x00) {
                // Live House/Dungeon Jar palette (S 0x0C (8,2)/(8,3)):
                // 1=dark outline(82,82,82), 2=shadow(123), 3=silver(198), 4=white(255), 5=clay(156,107,107), 6=BRIGHT SMW GOLD(247,206,82), 7=amber(206,156,115)
                const LUT_JAR_TURN: [u8; 8] = [0, 4, 1, 5, 7, 6, 6, 5];
                let tb_raw = remap_16x16(&read_3bpp_16x16(&g00, 0, 4), &LUT_JAR_TURN);
                let mut tb_top = [[0u8; 8]; 8];
                let mut tb_bot = [[0u8; 8]; 8];
                for y in 0..8 {
                    for x in 0..8 {
                        tb_top[y][x] = tb_raw[y][x];
                        tb_bot[y][x] = tb_raw[y + 8][x];
                    }
                }
                let q_push = crate::mario_ui::build_smw_dungeon_qblock_16x16();
                let (psw_top, psw_bot) = crate::mario_ui::build_smw_dungeon_pswitch_half_8x16();
                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0x0C,
                    &format!("Jarres, Blocs ? & P-Switches Donjons (Sheet 0x0C) ← {}:Turn Blocks Dorés, Blocs ? & P-Switches SMW", rule.donor_alias),
                    free_cursor,
                    free_end,
                    &|raw| {
                        write_3bpp_8x8(raw, 8, 2, &tb_top);
                        write_3bpp_8x8(raw, 8, 3, &tb_bot);
                        write_3bpp_16x16(raw, 2, 2, &q_push);   // Dungeon Pushable Stone Block -> SMW '?' Block!
                        write_3bpp_8x8(raw, 0xB, 0, &psw_top);  // Dungeon Floor Switch -> SMW 'P'-Switch!
                        write_3bpp_8x8(raw, 0xB, 1, &psw_bot);
                    },
                )? {
                    ops += 1;
                }

                // Coffres Intérieurs BG (Sheet 0x0D (1,2)/(1,3) fermé & (2,2)/(2,3) ouvert — Sanctuaire + Maison + Donjons!)
                // Live Sanctuary/House Chest palette: 1=dark outline, 2=amber-red shade, 3=salmon, 4=white, 6=bright SMW gold!
                const LUT_CHEST_BG: [u8; 8] = [0, 4, 1, 2, 6, 6, 2, 2];
                let used_raw = remap_16x16(&read_3bpp_16x16(&g00, 14, 2), &LUT_CHEST_BG);
                // Closed BG chest is H-mirrored by ALTTP's tilemap -> build a 100% H-symmetric SMW Golden '!' Prize Block
                // using SMW's authentic riveted block frame (GFX00 (14,2)) + bright white '!' emblem down the center!
                let mut prize_top = [[0u8; 8]; 8];
                let mut prize_bot = [[0u8; 8]; 8];
                let mut used_top = [[0u8; 8]; 8];
                let mut used_bot = [[0u8; 8]; 8];
                for y in 0..8 {
                    for x in 0..8 {
                        used_top[y][x] = used_raw[y][x];
                        used_bot[y][x] = used_raw[y + 8][x];
                        prize_top[y][x] = used_raw[y][x];
                        prize_bot[y][x] = used_raw[y + 8][x];
                    }
                }
                // Fill inner face of closed prize block with gleaming SMW gold (6) and crisp white '!' (4) with dark shadow (1)
                for y in 2..8 {
                    for x in 2..8 {
                        if prize_top[y][x] == 2 {
                            prize_top[y][x] = 6;
                        }
                    }
                }
                for y in 0..6 {
                    for x in 2..8 {
                        if prize_bot[y][x] == 2 {
                            prize_bot[y][x] = 6;
                        }
                    }
                }
                // Crisp symmetric '!' stem (rows y=3..7 top, y=0..1 bot) + dot (rows y=3..4 bot) at x=6..7 (mirrored -> 4px wide!)
                for y in 3..8 {
                    prize_top[y][6] = 1;
                    prize_top[y][7] = 4;
                }
                prize_top[2][6] = 1;
                prize_top[2][7] = 1;
                prize_bot[0][6] = 1;
                prize_bot[0][7] = 4;
                prize_bot[1][6] = 1;
                prize_bot[1][7] = 1;
                prize_bot[3][6] = 1;
                prize_bot[3][7] = 4;
                prize_bot[4][6] = 1;
                prize_bot[4][7] = 4;
                prize_bot[5][6] = 1;
                prize_bot[5][7] = 1;

                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0x0D,
                    &format!("Coffres Intérieurs Sanctuaire/Donjons (Sheet 0x0D) ← {}:Bloc « ! » Doré & Bloc Vidé SMW", rule.donor_alias),
                    free_cursor,
                    free_end,
                    &|raw| {
                        write_3bpp_8x8(raw, 1, 2, &prize_top);
                        write_3bpp_8x8(raw, 1, 3, &prize_bot);
                        write_3bpp_8x8(raw, 2, 2, &used_top);
                        write_3bpp_8x8(raw, 2, 3, &used_bot);
                    },
                )? {
                    ops += 1;
                }
            }
        }
        "zoras_water" => {
            if let Some((cc1, cc2)) = extract_donor_16x16_for_zelda(donor, "cheep_cheep", false) {
                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0x85,
                    &format!(
                        "Zoras & Roi Zora (Sheet 0x85) ← {}:Cheep Cheeps & Porcu-Puffer",
                        rule.donor_alias
                    ),
                    free_cursor,
                    free_end,
                    &|raw| {
                        write_3bpp_16x16(raw, 0, 0, &cc1);
                        write_3bpp_16x16(raw, 2, 0, &cc2);
                        write_3bpp_16x16(raw, 4, 0, &cc1);
                        write_3bpp_16x16(raw, 6, 0, &cc2);
                        write_3bpp_16x16(raw, 8, 0, &flip_h_16x16(&cc1));
                    },
                )? {
                    ops += 1;
                }
            }
        }
        "dungeon_extra" => {
            if patch_alttp_comp_sheet_helper(
                rom_zelda,
                session,
                0x8E,
                &format!(
                    "Armos & Statues (Sheet 0x8E) ← {}:Thwomps SMW 16x32 complets",
                    rule.donor_alias
                ),
                free_cursor,
                free_end,
                &|raw| {
                    crate::mario_ui::patch_smw_thwomp_sheet_8e(raw);
                },
            )? {
                ops += 1;
            }
            if let (Some((bb1, bb2)), Some((wg1, wg2))) = (
                extract_donor_16x16_for_zelda(donor, "buzzy_beetle", false),
                extract_donor_16x16_for_zelda(donor, "wiggler", false),
            ) {
                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0x8F,
                    &format!("Hardhat Beetles & Mini-Moldorms (Sheet 0x8F) ← {}:Buzzy Beetles & Wigglers", rule.donor_alias),
                    free_cursor,
                    free_end,
                    &|raw| {
                        write_3bpp_16x16(raw, 0, 0, &bb1);
                        write_3bpp_16x16(raw, 2, 0, &bb2);
                        write_3bpp_16x16(raw, 4, 0, &wg1);
                        write_3bpp_16x16(raw, 6, 0, &wg2);
                    },
                )? {
                    ops += 1;
                }
            }
        }
        "sewer_enemies" => {
            if patch_alttp_comp_sheet_helper(
                rom_zelda,
                session,
                0x8D,
                &format!("Égouts & Château (Sheet 0x8D) ← {}:Boos SMW (3 frames), Buzzy Beetles & Spinies", rule.donor_alias),
                free_cursor,
                free_end,
                &|raw| {
                    crate::mario_ui::patch_smw_sewers_sheet_8d(raw);
                },
            )? {
                ops += 1;
            }
        }
        "overworld_enemies" => {
            if let Some((vl1, _)) = extract_donor_16x16_for_zelda(donor, "volcano_lotus", false) {
                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0x7F,
                    &format!(
                        "Overworld Octoroks Alt (Sheet 0x7F) ← {}:{}",
                        rule.donor_alias, rule.donor_source
                    ),
                    free_cursor,
                    free_end,
                    &|raw| {
                        write_3bpp_16x16(raw, 14, 0, &vl1);
                    },
                )? {
                    ops += 1;
                }
            }
            if patch_alttp_comp_sheet_helper(
                rom_zelda,
                session,
                0x82,
                &format!("Overworld Ennemis (Sheet 0x82) ← {}:Galoombas SMW (4 dirs), Boules de Feu, Spinies & Paragoombas", rule.donor_alias),
                free_cursor,
                free_end,
                &|raw| {
                    crate::mario_ui::patch_smw_overworld_enemies_sheet_82(raw);
                },
            )? {
                ops += 1;
            }
            // v13 Total World Conversion: Toad Town NPCs (6 sheets) + Full SMW Bestiary (16 sheets)
            // + Dark World / Bowser's Castle / Bowser Statues / Toad Town Roofs BG Tilesets (9 sheets)!
            ops += crate::mario_world::apply_mario_world_total_conversion(
                rom_zelda,
                donor,
                session,
                free_cursor,
                free_end,
            )?;
        }
        "dungeon_enemies" => {
            if patch_alttp_comp_sheet_helper(
                rom_zelda,
                session,
                0x90,
                &format!(
                    "Stalfos Donjons (Sheet 0x90) ← {}:Skelerex (Dry Bones) SMW complets",
                    rule.donor_alias
                ),
                free_cursor,
                free_end,
                &|raw| {
                    crate::mario_ui::patch_smw_dry_bones_sheet_90(raw);
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
                    let (g1, g2) =
                        extract_donor_16x16_for_zelda(donor, "goomba", true).unwrap_or((bob, bob));
                    let (key, _) = extract_donor_16x16_for_zelda(donor, "smw_key", false)
                        .unwrap_or((coin, coin));
                    let (shell, _) = extract_donor_16x16_for_zelda(donor, "koopa_shell", false)
                        .unwrap_or((coin, coin));
                    let (bill, _) = extract_donor_16x16_for_zelda(donor, "bullet_bill", false)
                        .unwrap_or((bob, bob));
                    let (ff, _) = extract_donor_16x16_for_zelda(donor, "fire_flower", false)
                        .unwrap_or((mush, mush));
                    let (star, _) = extract_donor_16x16_for_zelda(donor, "starman", false)
                        .unwrap_or((coin, coin));
                    let (cf, _) = extract_donor_16x16_for_zelda(donor, "cape_feather", false)
                        .unwrap_or((coin, coin));
                    let (ps, _) = extract_donor_16x16_for_zelda(donor, "p_switch", false)
                        .unwrap_or((coin, coin));
                    let (moon, _) = extract_donor_16x16_for_zelda(donor, "moon_3up", false)
                        .unwrap_or((coin, coin));
                    let (egg, _) = extract_donor_16x16_for_zelda(donor, "yoshi_egg", false)
                        .unwrap_or((mush, mush));
                    let (_berry, _) = extract_donor_16x16_for_zelda(donor, "berry", false)
                        .unwrap_or((mush, mush));

                    // Fix #2 & #6: Double 8x16 complete SMW Red/Gold Coins + Crisp High-Contrast 16x16 Super Mushroom
                    //              + 8x8 Red Mini-Mushroom & Berry pair in S 0xCD (6,2)!
                    let crisp_mush = crate::mario_ui::build_crisp_smw_mushroom_16x16();
                    let crisp_bill = crate::mario_ui::build_smw_bullet_bill_drop_16x16();
                    let black_bob = crate::mario_ui::build_jet_black_smw_bobomb_16x16();
                    let _ = (mush, bill, bob);
                    let double_coins = crate::mario_ui::build_double_8x16_smw_coins();
                    let small_heart_magic = crate::mario_ui::build_small_heart_magic_pair();
                    if patch_alttp_comp_sheet_helper(
                        rom_zelda,
                        session,
                        0xCD,
                        &format!("Collectibles Zelda (Sheet 0xCD) ← {}:Pièces 8x16, Champignons Rouges, Bob-ombs Noirs Boutique/Coffre, Bill Balle, Œuf", rule.donor_alias),
                        free_cursor,
                        free_end,
                        &|raw| {
                            write_3bpp_16x16(raw, 0, 0, &double_coins);      // Rubis 8x16 -> Double Coin Rouge/Or SMW complet!
                            write_3bpp_16x16(raw, 2, 0, &double_coins);      // Rubis 8x16 -> Double Coin SMW complet!
                            write_3bpp_16x16(raw, 5, 0, &egg);               // VRAI Cristal (gfx 0x28) -> Œuf Yoshi!
                            write_3bpp_16x16(raw, 8, 0, &crisp_bill);        // Arc -> Bill Balle Noir contour blanc!
                            write_3bpp_16x16(raw, 11, 0, &black_bob);        // Bombes 1/3 (B..C, 0..1) -> 100% JET-BLACK SMW Bob-omb!
                            write_3bpp_16x16(raw, 13, 0, &black_bob);        // Pack 10 Bombes Boutique (D..E, 0..1) -> 100% JET-BLACK SMW Bob-omb!
                            write_3bpp_16x16(raw, 0, 2, &crisp_mush);        // Réceptacle Cœur -> Super Champignon ROUGE net!
                            write_3bpp_16x16(raw, 2, 2, &shell);             // Boomerang -> Carapace
                            write_3bpp_16x16(raw, 4, 2, &shell);             // Bouclier -> Carapace
                            write_3bpp_16x16(raw, 6, 2, &small_heart_magic); // Petit Cœur 8x8 (6,2) -> Mini-Champignon Rouge + Baie!
                            write_3bpp_16x16(raw, 8, 2, &crisp_bill);        // Flèche -> Bill Balle Noir contour blanc!
                        },
                    )? {
                        ops += 1;
                    }
                    // Build Jet-Black SMW Bob-omb with crisp white '10' badge in bottom-right corner
                    // for Kamek's Shop 10-Bomb pack (Sheet 0xCB / raw 0xCD cols 8..9 rows 2..3)!
                    // Uses ONLY index 5 (Jet-Black), 1 (Pure White), and 4/3 (Gold key/boots) -> ZERO blue pixels!
                    let mut shop_bob_10 = black_bob;
                    let badge_10: [&str; 6] = [
                        ".55.5555", "51551115", ".5551515", ".5551515", "51551115", ".55.5555",
                    ];
                    for (by, row_s) in badge_10.iter().enumerate() {
                        for (bx, ch) in row_s.bytes().enumerate() {
                            if ch != b'.' {
                                shop_bob_10[10 + by][8 + bx] = ch - b'0';
                            }
                        }
                    }
                    if patch_alttp_comp_sheet_helper(
                        rom_zelda,
                        session,
                        0xCB,
                        &format!("Objets & Boutique (Sheet 0xCB) ← {}:Super Champignon, Fleur Feu, Pack 10 Bob-ombs Noirs, Clé", rule.donor_alias),
                        free_cursor,
                        free_end,
                        &|raw| {
                            write_3bpp_16x16(raw, 2, 0, &crisp_mush);  // Potion Rouge Boutique (150 pièces) -> Super Champignon Rouge SMW!
                            write_3bpp_16x16(raw, 6, 0, &crisp_mush);  // Cœur / Réserve -> Super Champignon Rouge net!
                            write_3bpp_16x16(raw, 8, 0, &ff);          // Lanterne -> Fleur de Feu
                            write_3bpp_16x16(raw, 14, 0, &shell);      // Bouclier -> Carapace
                            write_3bpp_16x16(raw, 8, 2, &shop_bob_10); // Pack 10 Bombes Boutique (50 pièces) -> 100% JET-BLACK SMW Bob-omb '10'!
                            write_3bpp_16x16(raw, 10, 2, &black_bob);  // Bombes -> 100% JET-BLACK SMW Bob-omb!
                            write_3bpp_16x16(raw, 14, 2, &key);        // Clé -> Clé SMW
                        },
                    )? {
                        ops += 1;
                    }
                    if patch_alttp_comp_sheet_helper(
                        rom_zelda,
                        session,
                        0xCC,
                        &format!("Médaillons & Reliques (Sheet 0xCC) ← {}:Fleur Feu, P-Switch, Starman, Plume, Lune 3-UP", rule.donor_alias),
                        free_cursor,
                        free_end,
                        &|raw| {
                            write_3bpp_16x16(raw, 6, 0, &ff);    // VRAIE Lanterne (gfx 0x14) & Pendentif Puissance -> Fleur de Feu SMW!
                            write_3bpp_16x16(raw, 0, 2, &ff);    // Bombos -> Fleur de Feu
                            write_3bpp_16x16(raw, 2, 2, &ps);    // Éther -> Interrupteur P
                            write_3bpp_16x16(raw, 4, 2, &ps);    // Secousse -> Interrupteur P
                            write_3bpp_16x16(raw, 6, 2, &moon);  // Perle de Lune -> Lune 3-UP
                            write_3bpp_16x16(raw, 10, 2, &star); // Byrna & Pendentif Sagesse (gfx 0x1E) -> Étoile Super Mario!
                            write_3bpp_16x16(raw, 12, 2, &cf);   // Cape -> Plume de Cape
                        },
                    )? {
                        ops += 1;
                    }
                    // Fix #5: Carried Overworld Rock (VERIFIED: S 0x72 & S 0x7C cell (4,0)!) & Carried Interior Jar (S 0x7B cell (6,0)!)
                    //         -> SMW Turn Block matching exact ground/shelf palettes so blocks KEEP Mario look in Mario's hands!
                    let (carry_rock, carry_pot) =
                        if let Some((g00, _, _)) = load_smw_3bpp_sheet(&donor.data, 0x00) {
                            const LUT_CARRY_ROCK: [u8; 8] = [0, 6, 1, 2, 7, 6, 7, 2]; // Matches 0x0DD49A mint-green Turn Block!
                            const LUT_CARRY_POT: [u8; 8] = [0, 6, 1, 2, 3, 4, 4, 3]; // Matches 0x0DD4D2 golden Turn Block!
                            let raw_tb = read_3bpp_16x16(&g00, 0, 4);
                            (
                                Some(remap_16x16(&raw_tb, &LUT_CARRY_ROCK)),
                                Some(remap_16x16(&raw_tb, &LUT_CARRY_POT)),
                            )
                        } else {
                            (None, None)
                        };
                    let carry_bush = crate::mario_ui::build_smw_smiling_bush_16x16();
                    let carry_msg = crate::mario_ui::build_smw_message_block_16x16();
                    let black_bob_spark = crate::mario_ui::build_jet_black_smw_bobomb_spark_16x16();
                    for &(pc, is_interior) in &[
                        (0x087600usize, false),
                        (0x08AC00usize, true),
                        (0x08B200usize, false),
                    ] {
                        let mut raw = rom_zelda.data[pc..pc + 0x600].to_vec();
                        write_3bpp_16x16(&mut raw, 4, 2, &g1);
                        write_3bpp_16x16(&mut raw, 6, 2, &g2);
                        write_3bpp_16x16(&mut raw, 14, 2, &black_bob); // Live ground/thrown Bob-omb -> 100% JET-BLACK!
                        if is_interior {
                            if let Some(ref pot) = carry_pot {
                                write_3bpp_16x16(&mut raw, 6, 0, pot); // Carried House/Dungeon Jar (6,0) -> Golden Turn Block!
                            }
                        } else {
                            write_3bpp_16x16(&mut raw, 2, 0, &carry_msg); // Carried/Cut OW Signpost (2,0) -> SMW Message Block!
                            if let Some(ref rock) = carry_rock {
                                write_3bpp_16x16(&mut raw, 4, 0, rock); // Carried OW Rock (4,0) -> Mint-Green Turn Block!
                            }
                            write_3bpp_16x16(&mut raw, 6, 0, &carry_bush); // Carried OW Bush (6,0) -> SMW Smiling Hill Bush!
                        }
                        session.swap_custom_bytes(
                            rom_zelda,
                            pc,
                            pc,
                            raw,
                            format!("Sprites Objets & Blocs Portés 0x{:06X} ← {}:Turn Blocks, Buissons, Blocs Message & Bob-ombs Noirs", pc, rule.donor_alias),
                            SwapKind::GraphicsTiles {
                                format: TileFormat::Bpp3,
                                tile_count: 64,
                                smart_remapped: true,
                            },
                        )?;
                        ops += 1;
                    }
                    // Complete 6-frame animated Jet-Black SMW Bob-omb spark on remaining water/rain sheets (0x087C00/0x088200/0x088800 (14,2))!
                    for &(idx, pc) in &[
                        (0usize, 0x087C00usize),
                        (1, 0x088200usize),
                        (2, 0x088800usize),
                    ] {
                        let mut raw = rom_zelda.data[pc..pc + 0x600].to_vec();
                        let frame = if idx % 2 == 0 {
                            &black_bob_spark
                        } else {
                            &black_bob
                        };
                        write_3bpp_16x16(&mut raw, 14, 2, frame);
                        session.swap_custom_bytes(
                            rom_zelda,
                            pc,
                            pc,
                            raw,
                            format!("Bob-omb Noir Animé 0x{:06X} (14,2) ← {}:Étincelle mèche Bob-omb SMW", pc, rule.donor_alias),
                            SwapKind::GraphicsTiles {
                                format: TileFormat::Bpp3,
                                tile_count: 64,
                                smart_remapped: true,
                            },
                        )?;
                        ops += 1;
                    }
                    // Live Combat Weapons & Projectiles (0x087000): Flying SMW Bullet Bills, Mini Bill Blaster, Spinning Koopa Shell Boomerang, Hammer Bro Hammer!
                    {
                        let mut w70 = rom_zelda.data[0x087000..0x087600].to_vec();
                        crate::mario_ui::patch_smw_live_weapons_087000(&mut w70);
                        session.swap_custom_bytes(
                            rom_zelda,
                            0x087000,
                            0x087000,
                            w70,
                            format!("Armes & Projectiles Live 0x087000 ← {}:Bill Balles Volants, Canon Bill Blaster, Carapace Boomerang & Marteau Bro", rule.donor_alias),
                            SwapKind::GraphicsTiles {
                                format: TileFormat::Bpp3,
                                tile_count: 64,
                                smart_remapped: true,
                            },
                        )?;
                        ops += 1;
                    }
                    // Mario's 3 Live Shields Held in Hand (Sheet 0xD0) -> SMW Koopa Shell Shields + Super Star Shield + '?' Block Tome!
                    if patch_alttp_comp_sheet_helper(
                        rom_zelda,
                        session,
                        0xD0,
                        &format!("Boucliers de Mario (Sheet 0xD0) ← {}:Carapaces Koopa SMW, Bouclier Étoile & Livre Bloc ?", rule.donor_alias),
                        free_cursor,
                        free_end,
                        &|raw| {
                            crate::mario_ui::patch_mario_shields_sheet_d0(raw);
                        },
                    )? {
                        ops += 1;
                    }
                    // Phase 5: Replace ALL live Sword Blades & File Select Fairy Cursor ($A8/$AA) in Sheet 0xCF
                    // with SMW Super Mario Hammer Swings + 2-Frame Sparkling Golden Super Star File Select Cursor!
                    if patch_alttp_comp_sheet_helper(
                        rom_zelda,
                        session,
                        0xCF,
                        &format!("Marteau Super Mario & Curseur Étoile Sélection (Sheet 0xCF) ← {}:Marteau SMW & Super Étoile 2-frames", rule.donor_alias),
                        free_cursor,
                        free_end,
                        &|raw| {
                            crate::mario_ui::patch_mario_super_hammer_and_fileselect_sheet_cf(raw);
                        },
                    )? {
                        ops += 1;
                    }
                    // Dungeon Mini-Map Player Marker (Sheet 0xD2 (1,3)) -> Mini Mario Red Cap Head!
                    let dmap_mario = crate::mario_ui::build_smw_dungeon_map_mario_8x8();
                    if patch_alttp_comp_sheet_helper(
                        rom_zelda,
                        session,
                        0xD2,
                        &format!("Carte Donjon Joueur (Sheet 0xD2 (1,3)) ← {}:Mini Tête Mario Casquette Rouge", rule.donor_alias),
                        free_cursor,
                        free_end,
                        &|raw| {
                            write_3bpp_8x8(raw, 1, 3, &dmap_mario);
                        },
                    )? {
                        ops += 1;
                    }
                    // Verified World Map OBJ Sheet (0x088E00): (0,2)=Super Star, (4,2)=Yoshi Egg, (6,2)=Mario Head Red Cap!
                    // Also cols 8..15 in rows 0..1 = 4-frame sparkling SMW Super Star cursor (File Select & Game Over)!
                    let mut hud = rom_zelda.data[0x088E00..0x089400].to_vec();
                    let wmap_mario = crate::mario_ui::build_smw_world_map_mario_head_16x16();
                    let cursor_frames = crate::mario_ui::build_smw_sparkling_star_cursor_frames();
                    write_3bpp_16x16(&mut hud, 0, 2, &star);
                    write_3bpp_16x16(&mut hud, 4, 2, &egg);
                    write_3bpp_16x16(&mut hud, 6, 2, &wmap_mario);
                    write_3bpp_16x16(&mut hud, 8, 0, &cursor_frames[0]);
                    write_3bpp_16x16(&mut hud, 10, 0, &cursor_frames[1]);
                    write_3bpp_16x16(&mut hud, 12, 0, &cursor_frames[2]);
                    write_3bpp_16x16(&mut hud, 14, 0, &cursor_frames[3]);
                    session.swap_custom_bytes(
                        rom_zelda,
                        0x088E00,
                        0x088E00,
                        hud,
                        format!("Icônes Carte du Monde & Curseur Étoile Scintillante 0x088E00 ← {}:Étoile, Œuf, Tête Mario & Étoiles 4-frames", rule.donor_alias),
                        SwapKind::GraphicsTiles {
                            format: TileFormat::Bpp3,
                            tile_count: 64,
                            smart_remapped: true,
                        },
                    )?;
                    ops += 1;

                    // Phase 5 Fix: Sheet 0x78 (0x089A00) holds the LIVE File Select Fairy Cursor (cols 10..13, rows 2..3 = $AA/$AC)
                    // AND the Big Heart Container (cols 0..1, rows 2..3 = $A0)! Replace Fairy with Sparkling SMW Super Star
                    // and Heart Container with Giant Super Mushroom!
                    if 0x089A00 + 0x600 <= rom_zelda.data.len() {
                        let mut sh78 = rom_zelda.data[0x089A00..0x089A00 + 0x600].to_vec();
                        write_3bpp_16x16(&mut sh78, 0, 2, &mush);
                        write_3bpp_16x16(&mut sh78, 10, 2, &cursor_frames[0]);
                        write_3bpp_16x16(&mut sh78, 12, 2, &cursor_frames[1]);
                        session.swap_custom_bytes(
                            rom_zelda,
                            0x089A00,
                            0x089A00,
                            sh78,
                            format!(
                                "Curseur Écran Sélection & Réserve Vie (Sheet 0x78 @ 0x089A00) ← {}:Super Étoile 2-frames & Super Champignon",
                                rule.donor_alias
                            ),
                            SwapKind::GraphicsTiles {
                                format: TileFormat::Bpp3,
                                tile_count: 64,
                                smart_remapped: true,
                            },
                        )?;
                        ops += 1;
                    }
                    // Also fix File Select cursor OAM tile indices & palette at $0E:96A6..$0E:96B5 (PC 0x0716AC..0x0716B4):
                    // Point cursor palette from pink fairy (0x7E) to OBJ Pal 0 (0x30: color 6 = Blazing Super Star Gold!)
                    if 0x0716B6 <= rom_zelda.data.len()
                        && rom_zelda.data[0x0716B2] == 0xA9
                        && rom_zelda.data[0x0716B3] == 0x7E
                    {
                        session.swap_custom_bytes(
                            rom_zelda,
                            0x0716B2,
                            0x0716B2,
                            vec![0xA9, 0x30], // OBJ Pal 0 (Blazing Super Star Gold RGB 31,29,8 on index 6!)
                            "Palette Curseur Écran Sélection ($0E:96B2) -> Super Étoile Or (Pal 0)"
                                .into(),
                            SwapKind::CodePatch { byte_count: 2 },
                        )?;
                        ops += 1;
                    }
                    // Phase 5 File Select Background Palette Overhaul (PC 0x0DD9AA..0x0DD9B1):
                    // Replace dull Zelda olive-green cobblestone backdrop (0x2589, 0x31EC, 0x3E4F, 0x4ED3)
                    // with vibrant Super Mario World Royal Sky Blue!
                    if 0x0DD9B2 <= rom_zelda.data.len()
                        && u16::from_le_bytes([rom_zelda.data[0x0DD9AA], rom_zelda.data[0x0DD9AB]])
                            == 0x2589
                    {
                        let smw_fs_bg: [u16; 4] = [0x4480, 0x5944, 0x6E29, 0x7F0E];
                        let mut pal_bytes = Vec::with_capacity(8);
                        for c in smw_fs_bg {
                            pal_bytes.extend_from_slice(&c.to_le_bytes());
                        }
                        session.swap_custom_bytes(
                            rom_zelda,
                            0x0DD9AA,
                            0x0DD9AA,
                            pal_bytes,
                            "Fond Écran Sélection (0x0DD9AA) : Pierre Verte Zelda -> Bleu Ciel Royal Super Mario World".into(),
                            SwapKind::PaletteBgr555 { color_count: 4 },
                        )?;
                        ops += 1;
                    }
                    // Phase 5 Interior & Boss Arena Palettes Revolution:
                    // 1. Kamek's Shop & Toad Town Houses Carpet (PC 0x0DD932..0x0DD939):
                    //    Olive-green Zelda rug -> Royal Mushroom Crimson Red & Gold Toad House Carpet!
                    if 0x0DD93A <= rom_zelda.data.len()
                        && u16::from_le_bytes([rom_zelda.data[0x0DD932], rom_zelda.data[0x0DD933]])
                            == 0x252B
                    {
                        let toad_rug: [u16; 4] = [0x104E, 0x03BF, 0x1494, 0x253B];
                        let mut rb = Vec::with_capacity(8);
                        for c in toad_rug {
                            rb.extend_from_slice(&c.to_le_bytes());
                        }
                        session.swap_custom_bytes(
                            rom_zelda,
                            0x0DD932,
                            0x0DD932,
                            rb,
                            "Tapis Intérieurs Boutique Kamek & Toad Town (0x0DD932) : Vert Zelda -> Tapis Royal Rouge & Or Mario".into(),
                            SwapKind::PaletteBgr555 { color_count: 4 },
                        )?;
                        ops += 1;
                    }
                    // 2. Bowser Final Pyramid Throne Room 0x000 Floor & Walls (PC 0x0DE2C2..0x0DE2D1):
                    //    Brown Zelda sand-stone -> Menacing Obsidian Dark-Charcoal Bowser Castle Volcanic Stone!
                    if 0x0DE2D2 <= rom_zelda.data.len()
                        && u16::from_le_bytes([rom_zelda.data[0x0DE2C2], rom_zelda.data[0x0DE2C3]])
                            == 0x2DB0
                    {
                        let bowser_volc: [u16; 8] = [
                            0x18C6, // Dark volcanic charcoal stone
                            0x2529, // Mid obsidian slate
                            0x318C, // Crisp stone highlight
                            0x1487, // Shadow slate
                            0x1085, // Deep castle shadow
                            0x0C45, // Volcanic ash
                            0x0823, // Darkest abyss
                            0x0138, // Molten Bowser lava ember accent!
                        ];
                        let mut vb = Vec::with_capacity(16);
                        for c in bowser_volc {
                            vb.extend_from_slice(&c.to_le_bytes());
                        }
                        session.swap_custom_bytes(
                            rom_zelda,
                            0x0DE2C2,
                            0x0DE2C2,
                            vb,
                            "Salle du Trône Bowser Chambre 0x00 (0x0DE2C2) : Pierre Brune Zelda -> Obsidienne & Lave Château Bowser".into(),
                            SwapKind::PaletteBgr555 { color_count: 8 },
                        )?;
                        ops += 1;
                    }

                    // Fixes #4, #7, #8, #10: Top-HUD Mini-Mushrooms, Yellow Coin, Bob-omb, Start Menu Relics & Boss Pendants!
                    ops += crate::mario_ui::patch_mario_hud_and_start_menu(
                        rom_zelda,
                        session,
                        free_cursor,
                        free_end,
                    )?;
                    // Fixes #8, #9, #10: House Chest Red Coin, Boss-Drop 3 Relics & +8px Carried Block Height!
                    ops += crate::mario_ui::patch_red_coin_and_carry_height(rom_zelda, session)?;
                    // Fix #10: Dialog Box 16x16 Red & White Super Mushroom Icon + French "réserve de champignon" + "Peach"!
                    ops +=
                        crate::mario_ui::patch_dialog_mushroom_and_french_text(rom_zelda, session)?;
                }
            } else if let Some((it, _)) =
                extract_donor_16x16_for_zelda(donor, &rule.donor_source, false)
            {
                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0xCD,
                    &format!(
                        "Collectibles Zelda (Sheet 0xCD) ← {}:{}",
                        rule.donor_alias, rule.donor_source
                    ),
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
            if let Some((tile, _)) = extract_donor_16x16_for_zelda(donor, &rule.donor_source, false)
            {
                for &ow_sheet in &[0x06usize, 0x07, 0x08] {
                    if patch_alttp_comp_sheet_helper(
                        rom_zelda,
                        session,
                        ow_sheet,
                        &format!(
                            "Overworld Rochers (Sheet 0x{:02X}) ← {}:{}",
                            ow_sheet, rule.donor_alias, rule.donor_source
                        ),
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
            // Overworld Light World Sheet 0x3C (devant la maison de Mario & tout le Royaume Champignon!):
            // - (2,2)/(2,3) H-miroir: Rochers soulevables -> Blocs Tournants Verts SMW (GFX00 (0,4))!
            // - (2,0)/(2,1) H-miroir: Panneaux en bois -> Blocs Message SMW (Information Box)!
            // - (0,0)/(0,1) & (3,0)/(3,1) H-miroir: Buissons -> Buissons Souriants SMW (Hill Bushes aux yeux ovales)!
            if let Some((g00, _, _)) = load_smw_3bpp_sheet(&donor.data, 0x00) {
                // Live OW rock palette: 1=dark outline, 2=teal shade, 3=mint bright, 4=white shine, 5=deep green, 6=vibrant green
                const LUT_OW_ROCK: [u8; 8] = [0, 4, 1, 5, 2, 3, 6, 5];
                let tb = remap_16x16(&read_3bpp_16x16(&g00, 0, 4), &LUT_OW_ROCK);
                let mut top = [[0u8; 8]; 8];
                let mut bot = [[0u8; 8]; 8];
                for y in 0..8 {
                    for x in 0..8 {
                        top[y][x] = tb[y][x];
                        bot[y][x] = tb[y + 8][x];
                    }
                }
                let (msg_top, msg_bot) = crate::mario_ui::build_smw_message_block_half_8x16();
                let (bush_top, bush_bot) = crate::mario_ui::build_smw_smiling_bush_half_8x16();
                let (brick_top, brick_bot) = crate::mario_ui::build_smw_ow_brick_block_half_8x16();
                let (pipe_top, pipe_bot) = crate::mario_ui::build_smw_ow_warp_pipe_half_8x16();
                let (flw_top, flw_bot) = crate::mario_ui::build_smw_ow_smiling_flowers_8x16();
                if patch_alttp_comp_sheet_helper(
                    rom_zelda,
                    session,
                    0x3C,
                    &format!("Overworld Props (Sheet 0x3C) ← {}:Tuyaux Warp Verts SMW (0x21B), Blocs Tournants, Briques, Blocs Message, Buissons & Fleurs SMW", rule.donor_alias),
                    free_cursor,
                    free_end,
                    &|raw| {
                        write_3bpp_8x8(raw, 2, 2, &top);       // Liftable Rock (m16 0x20F) -> SMW Mint-Green Turn-Block
                        write_3bpp_8x8(raw, 2, 3, &bot);
                        write_3bpp_8x8(raw, 0, 2, &pipe_top);  // Green Pillar/Portal (m16 0x21B, Pal 6) -> Physical 3D SMW Green Warp Pipe!
                        write_3bpp_8x8(raw, 0, 3, &pipe_bot);
                        write_3bpp_8x8(raw, 3, 2, &brick_top); // Heavy Dark Rock (m16 0x239) -> SMW Classic Brick Block!
                        write_3bpp_8x8(raw, 3, 3, &brick_bot);
                        write_3bpp_8x8(raw, 1, 2, &flw_top);   // Grass Decorative Flowers -> SMW Daisy/Smiling Flowers!
                        write_3bpp_8x8(raw, 1, 3, &flw_bot);
                        write_3bpp_8x8(raw, 2, 0, &msg_top);   // Wooden Signpost -> SMW Message Block!
                        write_3bpp_8x8(raw, 2, 1, &msg_bot);
                        write_3bpp_8x8(raw, 0, 0, &bush_top);  // Overworld Bush -> SMW Smiling Hill Bush!
                        write_3bpp_8x8(raw, 0, 1, &bush_bot);
                        write_3bpp_8x8(raw, 3, 0, &bush_top);  // Overworld Bush Alt -> SMW Smiling Hill Bush!
                        write_3bpp_8x8(raw, 3, 1, &bush_bot);
                    },
                )? {
                    ops += 1;
                }
            }
        }
        "audio_percussion" => {
            ops +=
                inject_unpitched_brr_percussion(rom_zelda, donor, &rule.donor_alias, session, 3)?;
        }
        "mario_music" => {
            ops += crate::mario_music::inject_mario_nspc_soundtrack(rom_zelda, session)?;
            ops += crate::mario_sfx::patch_mario_native_sfx(rom_zelda, session)?;
        }
        "intro_title" => {
            ops += crate::mario_intro::inject_legend_of_peach_intro(
                rom_zelda,
                session,
                free_cursor,
                free_end,
            )?;
            // Phase 5/7: Inject Native 65c816 Super Mario Gameplay Engine (Jump, Stomp, Fire Mario, Starman)
            // + Systemic 128-Screen Mushroom Kingdom & Bowser Volcanic Kingdom Overworld Transformation!
            ops += crate::mario_engine::inject_mario_gameplay_engine(rom_zelda, session)?;
            let (map_ops, _world_rep) =
                crate::mario_map::patch_all_mushroom_kingdom_overworld(rom_zelda, session)?;
            ops += map_ops;
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
        0x001346,
        0x001346,
        scratch.data[0x001346..0x001802].to_vec(),
        "Patch Tables DMA Joueur Bank $00 (0x001346..0x001802) : 28 poses impaires/recul/feu remappées sans doublon".into(),
        SwapKind::CodePatch {
            byte_count: 303 * 4,
        },
    )?;
    ops += 1;
    session.swap_custom_bytes(
        rom_zelda,
        0x068000,
        0x068000,
        scratch.data[0x068000..0x06838D].to_vec(),
        "Patch OAM Bank $0D (pose-par-pose) pour Super Mario 16x32".into(),
        SwapKind::CodePatch {
            byte_count: 303 * 3,
        },
    )?;
    ops += 1;
    if 0x069CF1 <= scratch.data.len() {
        session.swap_custom_bytes(
            rom_zelda,
            0x068EEF,
            0x068EEF,
            scratch.data[0x068EEF..0x068EEF + 0x3FE].to_vec(),
            "Patch OAM Épée Bank $0D (Garde dans le gant blanc 4 dir)".into(),
            SwapKind::CodePatch { byte_count: 0x3FE },
        )?;
        ops += 1;
        session.swap_custom_bytes(
            rom_zelda,
            0x0698F3,
            0x0698F3,
            scratch.data[0x0698F3..0x0698F3 + 0x3FE].to_vec(),
            "Patch OAM Lame Épée Bank $0D (Lame alignée 4 dir)".into(),
            SwapKind::CodePatch { byte_count: 0x3FE },
        )?;
        ops += 1;
    }
    session.swap_custom_bytes(
        rom_zelda,
        0x0DD308,
        0x0032DB,
        scratch.data[0x0DD308..0x0DD380].to_vec(),
        "Palettes Link -> Mario SMW (Classique, Mario Bleu, Fire Mario, 0 Luigi jouable)".into(),
        SwapKind::PaletteBgr555 { color_count: 60 },
    )?;
    ops += 1;
    if 0x0DEDF9 <= scratch.data.len() {
        session.swap_custom_bytes(
            rom_zelda,
            0x0DEDF5,
            0x0DEDF5,
            scratch.data[0x0DEDF5..0x0DEDF9].to_vec(),
            "Patch Gants Puissance/Titan (0x0DEDF5 -> 0x7FFF) : 0 clignotement vert/rouge".into(),
            SwapKind::PaletteBgr555 { color_count: 2 },
        )?;
        ops += 1;
    }
    Ok(ops)
}

/// Build 100% authentic SMW Princess Peach (GFX26 (E,0) head + (E,2) dress) with symmetric closed outline
/// plus back-hair and side-facing head variants ready for ALTTP Sheet 0xD5 / 0xD6!
fn build_smw_peach_for_zelda(
    rom_smw: &SnesRom,
) -> Option<(
    [[u8; 16]; 16],
    [[u8; 16]; 16],
    [[u8; 16]; 16],
    [[u8; 16]; 16],
)> {
    let (g26, _, _) = load_smw_3bpp_sheet(&rom_smw.data, 0x26)?;
    const LUT_PH: [u8; 8] = [0, 1, 5, 3, 4, 2, 4, 2];
    const LUT_PD: [u8; 8] = [0, 1, 5, 3, 4, 1, 6, 2];
    let raw_h = remap_16x16(&read_3bpp_16x16(&g26, 14, 0), &LUT_PH);
    let raw_d = remap_16x16(&read_3bpp_16x16(&g26, 14, 2), &LUT_PD);

    let mut head = [[0u8; 16]; 16];
    for y in 0..16 {
        for x in 0..14 {
            head[y][x + 2] = raw_h[y][x];
        }
        if head[y][2] != 0 && head[y][2] != 5 {
            head[y][1] = if y <= 3 || head[y][3] == 5 {
                5
            } else {
                head[y][2]
            };
            if head[y][1] != 5 && head[y][1] != 0 {
                head[y][0] = 5;
            }
        }
    }
    head[1][1] = 5;
    head[2][1] = 5;
    head[3][1] = 5;
    head[4][1] = 3;
    head[4][0] = 5;
    for y in 5..16 {
        if head[y][2] != 0 {
            head[y][1] = if y == 5 || y == 12 || y == 15 { 3 } else { 4 };
            head[y][0] = 5;
        }
    }

    let mut dress = [[0u8; 16]; 16];
    for y in 0..16 {
        for x in 0..14 {
            dress[y][x + 2] = raw_d[y][x];
        }
        if dress[y][2] != 0 {
            dress[y][1] = if y < 6 {
                if y >= 2 {
                    6
                } else {
                    3
                }
            } else if y < 11 {
                1
            } else {
                2
            };
            dress[y][0] = 5;
        }
    }
    for x in 1..14 {
        if dress[14][x] != 0 {
            dress[15][x] = 5;
        }
    }

    // Back head: golden blonde curls covering face area while keeping authentic SMW crown on top!
    let mut head_back = head;
    for y in 5..15 {
        for x in 2..13 {
            if head_back[y][x] == 1 || head_back[y][x] == 2 || head_back[y][x] == 5 {
                head_back[y][x] = if (x + y) % 3 == 0 { 3 } else { 4 };
            }
        }
    }
    let head_side = flip_h_16x16(&head);
    Some((head, dress, head_back, head_side))
}

/// Extract 100% authentic SMW Luigi (GFX32 4bpp poses 0x0F Front, 0x01 Side, 0x39 Back)
/// perceptually quantized to ALTTP's 3bpp green Uncle palette (S 0x7F & S 0xC2)!
fn build_smw_luigi_3bpp(
    rom_smw: &SnesRom,
) -> Option<(
    [[u8; 16]; 16],
    [[u8; 16]; 16],
    [[u8; 16]; 16],
    [[u8; 16]; 16],
    [[u8; 16]; 16],
    [[u8; 16]; 16],
)> {
    let (g32, _) = decompress_nintendo_lz2_with_endian(&rom_smw.data, 0x040000, 32768, true)?;
    let (ptop, pbot) = crate::demo::find_smw_mario_pose_tables(&rom_smw.data);
    // SMW 4bpp Mario/Luigi -> ALTTP 3bpp Uncle Green palette (RTX-3080 CIELAB optimal: dark overalls, green cap/shirt, yellow buttons, peach face!)
    const LUT_4TO3: [u8; 16] = [0, 1, 5, 3, 2, 2, 4, 5, 7, 6, 5, 5, 5, 6, 3, 2];
    let extract = |pose: usize, hflip: bool| -> ([[u8; 16]; 16], [[u8; 16]; 16]) {
        let va = rom_smw.data.get(ptop + 0x46 + pose).copied().unwrap_or(0);
        let vb = rom_smw.data.get(pbot + 0x46 + pose).copied().unwrap_or(0);
        let t = crate::demo::read_smw_gfx32_16x16(&g32, va);
        let b = crate::demo::read_smw_gfx32_16x16(&g32, vb);
        let mut to = [[0u8; 16]; 16];
        let mut bo = [[0u8; 16]; 16];
        for y in 0..16 {
            for x in 0..16 {
                let sx = if hflip { 15 - x } else { x };
                to[y][x] = LUT_4TO3[(t[y][sx] & 0x0F) as usize];
                bo[y][x] = LUT_4TO3[(b[y][sx] & 0x0F) as usize];
            }
        }
        (to, bo)
    };
    let (hd, bd) = extract(0x0F, false);
    let (hs, bs) = extract(0x01, true);
    let (hu, bu) = extract(0x39, false);
    Some((hd, bd, hs, bs, hu, bu))
}

/// Extract 100% authentic SMW Yoshi (GFX26: Front (4,4)+(4,6), Side (0,0)+(0,2), Back (6,4)+(6,6))
/// with vibrant primary green/blue/yellow snout & body, pure white belly, and gold/orange boots!
fn build_smw_yoshi_for_zelda(
    rom_smw: &SnesRom,
) -> Option<(
    [[u8; 16]; 16],
    [[u8; 16]; 16],
    [[u8; 16]; 16],
    [[u8; 16]; 16],
    [[u8; 16]; 16],
    [[u8; 16]; 16],
)> {
    let (g26, _, _) = load_smw_3bpp_sheet(&rom_smw.data, 0x26)?;
    const LUT_YOSHI: [u8; 8] = [0, 1, 5, 7, 6, 1, 6, 7];
    let fh = remap_16x16(&read_3bpp_16x16(&g26, 4, 4), &LUT_YOSHI);
    let mut fb = remap_16x16(&read_3bpp_16x16(&g26, 4, 6), &LUT_YOSHI);
    let sh = remap_16x16(&read_3bpp_16x16(&g26, 0, 0), &LUT_YOSHI);
    let mut sb = remap_16x16(&read_3bpp_16x16(&g26, 0, 2), &LUT_YOSHI);
    let bh = remap_16x16(&read_3bpp_16x16(&g26, 6, 4), &LUT_YOSHI);
    let mut bb = remap_16x16(&read_3bpp_16x16(&g26, 6, 6), &LUT_YOSHI);
    for body in [&mut fb, &mut sb, &mut bb] {
        for y in 12..16 {
            for x in 0..16 {
                if body[y][x] == 6 {
                    body[y][x] = 2;
                } else if body[y][x] == 7 {
                    body[y][x] = 3;
                }
            }
        }
    }
    Some((fh, fb, sh, sb, bh, bb))
}

pub(crate) fn patch_alttp_comp_sheet_pub(
    rom_z: &mut SnesRom,
    sess: &mut SwapSession,
    sheet_id: usize,
    desc: &str,
    free_cursor: &mut usize,
    free_end: usize,
    edit_fn: &dyn Fn(&mut [u8]),
) -> Result<bool, RomError> {
    patch_alttp_comp_sheet_helper(rom_z, sess, sheet_id, desc, free_cursor, free_end, edit_fn)
}

/// Patch any compressed ALTTP 3bpp sheet using its EXACT hardware raw index on `0x4F00`/`0x4FDF`/`0x50BE`:
/// - `raw_idx = 0x00..=0x72` for BG tileset sheet `bg_id`
/// - `raw_idx = 0x73 + sp_id` for Sprite sheet `sp_id` (`SP[0x00] = raw 0x73 = $10:F000`).
/// Automatically relocates larger compressed streams into clean 2 MB LoROM expansion banks (`$23..$3F`)
/// with strict `$8000` LoROM bank-boundary protection and lossless LC_LZ2 roundtrip verification.
pub(crate) fn patch_alttp_raw_sheet_pub(
    rom_z: &mut SnesRom,
    sess: &mut SwapSession,
    raw_idx: usize,
    desc: &str,
    free_cursor: &mut usize,
    free_end: usize,
    edit_fn: &dyn Fn(&mut [u8]),
) -> Result<bool, RomError> {
    let Some((z_bk, z_hi, z_lo)) = find_alttp_raw_gfx_table(&rom_z.data) else {
        return Ok(false);
    };
    if raw_idx >= 223 {
        return Ok(false);
    }
    let pc = ((rom_z.data[z_bk + raw_idx] as usize & 0x7F) << 15)
        | ((rom_z.data[z_hi + raw_idx] as usize & 0x7F) << 8)
        | (rom_z.data[z_lo + raw_idx] as usize);
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
    let orig_in_single_bank = pc / 0x8000 == (pc + orig_clen.saturating_sub(1)) / 0x8000;
    if comp.len() <= orig_clen && orig_in_single_bank {
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
    } else {
        // Relocate safely into clean LoROM 2MB expansion space (Bank $23..$3A, PC 0x118000..0x1D7FFF),
        // keeping Banks $20..$22 (3D Intro + Mario Engine) and Banks $3B..$3F (Map32) 100% isolated!
        const EXPANDED_FREE_START: usize = 0x118000;
        const EXPANDED_FREE_END: usize = crate::mario_map::OW_MAP32_RELOC_START_PC;
        let mut cur_end = if *free_cursor < 0x100000 {
            free_end
        } else {
            rom_z.data.len().min(EXPANDED_FREE_END)
        };
        if *free_cursor + comp.len() > cur_end {
            if rom_z.data.len() < 0x200000 {
                rom_z.data.resize(0x200000, 0xFF);
                if 0x007FD7 < rom_z.data.len() {
                    rom_z.data[0x007FD7] = 0x0B; // 16 Mbit standard SNES LoROM
                }
                rom_z.header.rom_size_byte = 0x0B;
                rom_z.header.declared_rom_size_kb = 2048;
                *free_cursor = EXPANDED_FREE_START;
                cur_end = EXPANDED_FREE_END;
            } else if *free_cursor < EXPANDED_FREE_START {
                *free_cursor = EXPANDED_FREE_START;
                cur_end = EXPANDED_FREE_END;
            }
        }
        let bank_rem = 0x8000 - (*free_cursor % 0x8000);
        if comp.len() > bank_rem {
            *free_cursor += bank_rem;
        }
        if *free_cursor + comp.len() > cur_end {
            return Ok(false);
        }
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
        sess.swap_custom_bytes(
            rom_z,
            z_bk + raw_idx,
            z_bk + raw_idx,
            vec![snes_bank],
            format!("Ptr Raw Bank 0x{:02X}", raw_idx),
            SwapKind::CodePatch { byte_count: 1 },
        )?;
        sess.swap_custom_bytes(
            rom_z,
            z_hi + raw_idx,
            z_hi + raw_idx,
            vec![(snes_addr >> 8) as u8],
            format!("Ptr Raw Hi 0x{:02X}", raw_idx),
            SwapKind::CodePatch { byte_count: 1 },
        )?;
        sess.swap_custom_bytes(
            rom_z,
            z_lo + raw_idx,
            z_lo + raw_idx,
            vec![(snes_addr & 0xFF) as u8],
            format!("Ptr Raw Lo 0x{:02X}", raw_idx),
            SwapKind::CodePatch { byte_count: 1 },
        )?;
        Ok(true)
    }
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
    let orig_in_single_bank = pc / 0x8000 == (pc + orig_clen.saturating_sub(1)) / 0x8000;
    if comp.len() <= orig_clen && orig_in_single_bank {
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
    } else {
        // If 1MB free block (0x066F46..0x068000) is full, cleanly auto-expand to standard 2MB SNES LoROM
        // (0x200000 bytes, header ROM size 0x0B at $007FD7 — 100% native SNES/Super Famicom LoROM 16Mbit!)
        // CRITICAL: Banks $20..$22 (PC 0x100000..0x117FFF) are RESERVED for 32 3D Intro Frames + 65816 DMA Code!
        // Therefore compressed sheet overflow MUST start at Bank $23 (PC 0x118000..0x1FFFFF = 928 KB clean space)!
        const EXPANDED_FREE_START: usize = 0x118000;
        const EXPANDED_FREE_END: usize = crate::mario_map::OW_MAP32_RELOC_START_PC;
        let mut cur_end = if *free_cursor < 0x100000 {
            free_end
        } else {
            rom_z.data.len().min(EXPANDED_FREE_END)
        };
        if *free_cursor + comp.len() > cur_end {
            if rom_z.data.len() < 0x200000 {
                rom_z.data.resize(0x200000, 0xFF);
                if 0x007FD7 < rom_z.data.len() {
                    rom_z.data[0x007FD7] = 0x0B; // 16 Mbit (2048 KB) standard SNES LoROM
                }
                rom_z.header.rom_size_byte = 0x0B;
                rom_z.header.declared_rom_size_kb = 2048;
                *free_cursor = EXPANDED_FREE_START; // Bank $23:$8000 (ZERO collision with 3D Intro at $20..$22 or Map32 at $3B..$3F!)
                cur_end = EXPANDED_FREE_END;
            } else if *free_cursor < EXPANDED_FREE_START {
                *free_cursor = EXPANDED_FREE_START;
                cur_end = EXPANDED_FREE_END;
            }
        }
        // Never allow a compressed sheet to straddle a $8000 LoROM bank boundary!
        let bank_rem = 0x8000 - (*free_cursor % 0x8000);
        if comp.len() > bank_rem {
            *free_cursor += bank_rem;
        }
        if *free_cursor + comp.len() > cur_end {
            return Ok(false);
        }
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
        sess.swap_custom_bytes(
            rom_z,
            z_bk + sheet_id,
            z_bk + sheet_id,
            vec![snes_bank],
            format!("Ptr Bank 0x{:02X}", sheet_id),
            SwapKind::CodePatch { byte_count: 1 },
        )?;
        sess.swap_custom_bytes(
            rom_z,
            z_hi + sheet_id,
            z_hi + sheet_id,
            vec![(snes_addr >> 8) as u8],
            format!("Ptr Hi 0x{:02X}", sheet_id),
            SwapKind::CodePatch { byte_count: 1 },
        )?;
        sess.swap_custom_bytes(
            rom_z,
            z_lo + sheet_id,
            z_lo + sheet_id,
            vec![(snes_addr & 0xFF) as u8],
            format!("Ptr Lo 0x{:02X}", sheet_id),
            SwapKind::CodePatch { byte_count: 1 },
        )?;
        Ok(true)
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
            }
        }
        "mount" => {}
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
                            format!(
                                "Ennemis SMW (GFX01) ← {}:{}",
                                rule.donor_alias, rule.donor_source
                            ),
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
        .filter(|s| {
            is_hw_oneshot(&target_rom.data, s) && s.block_count >= 12 && s.block_count <= 320
        })
        .collect();
    let mut d_perc: Vec<_> = donor_samps
        .into_iter()
        .filter(|s| {
            is_hw_oneshot(&donor_rom.data, s) && s.block_count >= 12 && s.block_count <= 450
        })
        .collect();

    t_perc.sort_by_key(|a| std::cmp::Reverse(a.block_count));
    d_perc.sort_by_key(|a| std::cmp::Reverse(a.block_count));

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
            colors_rgba: (0..16u8)
                .map(|i| [i * 17, i * 17, i * 17, if i == 0 { 0 } else { 255 }])
                .collect(),
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
        let transcoded =
            remap_and_transcode_tiles(&raw_b, tiles_to_copy, gb.format, &pal_b, ga.format, &pal_a);
        let mut full_raw =
            decode_universal_gfx_bytes(rom_a, ga).unwrap_or_else(|| vec![0u8; ga.decompressed_len]);
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
        let (Some(aa), Some(ab)) = (cat_a.audio_samples.get(ti), cat_b.audio_samples.get(di))
        else {
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
        UniversalGfxCompression::NintendoLz2BigEndian => decompress_nintendo_lz2_with_endian(
            &rom.data,
            g.rom_offset,
            g.decompressed_len.max(4096),
            true,
        )
        .map(|(d, _)| d),
        UniversalGfxCompression::NintendoLz2LittleEndian => decompress_nintendo_lz2_with_endian(
            &rom.data,
            g.rom_offset,
            g.decompressed_len.max(4096),
            false,
        )
        .map(|(d, _)| d),
    }
}
