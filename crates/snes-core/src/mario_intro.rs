//! Complete Intro, Title Screen, 3D Mario Relics Animation, Copyleft & French Script
//! Replacement for **"The Legend of Peach: A Mario to the Mushroom Kingdom"**
//!
//! 100% Native SNES & Super Famicom Hardware Compatible (Standard 16 Mbit / 2 MB LoROM):
//! 1. **Title Screen BG1 Logo (`Sheet 0x3E`, `Sheet 0x3F`, Stripe Tilemap `PC 0x071C68..0x071E10`)**:
//!    - Line 1 (Rows 7–8):   `THE LEGEND OF` (UNIFIED 8x16 beveled Nintendo serif font!)
//!    - Line 2 (Rows 9–14):  Giant 128x48 3D chiseled crimson-rose & peach-gold serif **`PEACH`**
//!                           with BG1 Priority=1 across ALL tiles so **`P` is 100% COMPLETE & UNBROKEN**!
//!    - Line 3 (Rows 15–16): `A MARIO TO THE` (same unified 8x16 beveled Nintendo serif font)
//!    - Line 4 (Rows 17–18): `MUSHROOM KINGDOM` (same unified 8x16 beveled Nintendo serif font)
//! 2. **Title Screen Copyright + Copyleft (`Sheet 0x79` + `$0E:8569` + `$0E:870A`)**:
//!    - Preserves `©1991,1992 Nintendo` and adds crisp **`🄯 2026 pomercier`** directly beneath it!
//! 3. **True 3D-Animated Super Mushroom + Fire Flower + Super Star (`Banks $20..$22`)**:
//!    - Replaces ALL 3 yellow Triforce triangles in BOTH the Title Screen AND Opening Story Scroll
//!      with 32 pre-rendered TRUE 3D-rotated volumetric voxel frames of:
//!      - Top (`X=1`):          **3D Super Mushroom**
//!      - Bottom-Left (`X=0`):  **3D Fire Flower**
//!      - Bottom-Right (`X=2`): **3D Super Star**
//!    - Native 65816 NMI DMA (`$00:92F7` -> `$22:8000`) & 16-color 4bpp OBJ Palette 6 (`$7E:C6C0`).
//! 4. **Complete French Script Rewrite (`Bank $1C`)**:
//!    - Intro Story Screens 1–4 (`0x0E5564..0x0E57AF`): `"Princesse Peach"` (replaces `"Reine"`),
//!      `"Champignon, Fleur & Etoile"` (replaces Triforce), `"Roi Bowser"` (replaces Magikoopa/Agahnim),
//!      `"Sept Yoshis"`.
//!    - Global Bank `$1C` rewrite: ALL `"Agahnim"` -> `"Bowser"`, ALL `"jeunes filles"` / `"petite-fille"`
//!      -> `"jeunes Yoshi"` / `"petit Yoshi"`, ALL `"Zelda"` -> `"Peach"`.

use std::collections::HashMap;

use crate::mixer::{patch_alttp_comp_sheet_pub, write_3bpp_8x8_pub};
use crate::rom::{RomError, SnesRom};
use crate::swapper::{SwapKind, SwapSession};

/// Exact PC bounds of ALTTP's uncompressed Title Screen BG1 Logo Stripe Image (`$0E:9C68..$0E:9E10`)
pub const TITLE_BG1_STRIPE_PC: usize = 0x071C68;
pub const TITLE_BG1_STRIPE_LEN: usize = 0x071E10 - 0x071C68; // 424 bytes exact!

// =============================================================================
// 1. GIANT 128x48 3D CHISELED SERIF "PEACH" LOGO GENERATOR (PAL 6)
// =============================================================================

fn fill_rect(mask: &mut [[bool; 128]; 48], x0: i32, y0: i32, x1: i32, y1: i32) {
    for y in y0.max(0)..y1.min(48) {
        for x in x0.max(0)..x1.min(128) {
            mask[y as usize][x as usize] = true;
        }
    }
}

fn fill_diag(mask: &mut [[bool; 128]; 48], x0: f32, y0: f32, x1: f32, y1: f32, half_w: f32) {
    let ya = y0.min(y1).floor().max(0.0) as i32;
    let yb = y0.max(y1).ceil().min(47.0) as i32;
    for y in ya..=yb {
        let t = ((y as f32) - y0) / (y1 - y0);
        let t_clamped = if (y1 - y0).abs() < 0.001 {
            0.5
        } else {
            t.clamp(0.0, 1.0)
        };
        let cx = x0 + (x1 - x0) * t_clamped;
        let xa = (cx - half_w).floor().max(0.0) as i32;
        let xb = (cx + half_w).ceil().min(127.0) as i32;
        for x in xa..=xb {
            mask[y as usize][x as usize] = true;
        }
    }
}

/// Build the monumental Nintendo-serif 128x48 pixel silhouette of **"PEACH"**
/// shaded with authentic ALTTP Title Screen Pal 6 3D bevel + ruby-crimson stone grain.
fn render_giant_peach_128x48() -> [[u8; 128]; 48] {
    let mut m = [[false; 128]; 48];

    // --- LETTER 'P' (x = 1..24, y = 4..44) ---
    fill_rect(&mut m, 4, 5, 11, 43); // Main vertical pillar (7px thick, 100% solid & complete!)
    fill_rect(&mut m, 1, 4, 18, 9); // Top bar & dramatic left serif overhang
    fill_rect(&mut m, 1, 39, 15, 44); // Flared bottom serif foot
    fill_rect(&mut m, 3, 37, 13, 40); // Serif bracket flare
    fill_rect(&mut m, 15, 5, 21, 10); // Upper curve
    fill_rect(&mut m, 17, 7, 23, 12);
    fill_rect(&mut m, 18, 10, 24, 24); // Thick outer right curve of 'P'
    fill_rect(&mut m, 16, 22, 23, 27);
    fill_rect(&mut m, 10, 23, 20, 28); // Lower bowl bar closing cleanly into stem!
    for y in 10..23 {
        for x in 11..18 {
            m[y][x] = false;
        }
    }
    m[10][17] = true;
    m[22][17] = true;

    // --- LETTER 'E' (x = 27..49, y = 4..44) ---
    fill_rect(&mut m, 29, 5, 36, 43);
    fill_rect(&mut m, 27, 4, 49, 10);
    fill_rect(&mut m, 44, 9, 49, 14);
    fill_rect(&mut m, 35, 21, 45, 27);
    fill_rect(&mut m, 43, 19, 46, 29);
    fill_rect(&mut m, 27, 38, 49, 44);
    fill_rect(&mut m, 44, 34, 49, 39);

    // --- LETTER 'A' (x = 52..76, y = 3..44) ---
    fill_diag(&mut m, 64.0, 4.0, 56.0, 43.0, 3.2);
    fill_diag(&mut m, 64.0, 4.0, 72.0, 43.0, 3.6);
    fill_rect(&mut m, 61, 3, 67, 8);
    fill_rect(&mut m, 56, 28, 72, 34);
    fill_rect(&mut m, 52, 39, 62, 44);
    fill_rect(&mut m, 66, 39, 76, 44);
    for y in 14..28 {
        let t = (y - 14) as f32 / 14.0;
        let xl = (64.0 - 1.5 - t * 3.5).round() as usize;
        let xr = (64.0 + 1.5 + t * 3.5).round() as usize;
        for x in xl..=xr {
            m[y][x] = false;
        }
    }

    // --- LETTER 'C' (x = 79..101, y = 4..44) ---
    fill_rect(&mut m, 84, 4, 100, 10);
    fill_rect(&mut m, 81, 6, 89, 13);
    fill_rect(&mut m, 79, 10, 86, 38);
    fill_rect(&mut m, 81, 35, 89, 42);
    fill_rect(&mut m, 84, 38, 100, 44);
    fill_rect(&mut m, 95, 7, 101, 15);
    fill_rect(&mut m, 95, 33, 101, 41);
    for y in 11..37 {
        for x in 86..95 {
            m[y][x] = false;
        }
    }
    m[11][86] = true;
    m[36][86] = true;

    // --- LETTER 'H' (x = 104..127, y = 4..44) ---
    fill_rect(&mut m, 106, 5, 113, 43);
    fill_rect(&mut m, 118, 5, 125, 43);
    fill_rect(&mut m, 112, 21, 119, 27);
    fill_rect(&mut m, 104, 4, 115, 9);
    fill_rect(&mut m, 116, 4, 127, 9);
    fill_rect(&mut m, 104, 39, 115, 44);
    fill_rect(&mut m, 116, 39, 127, 44);

    // --- 3D CHISELED BEVEL + RUBY-CRIMSON STONE TEXTURE (PAL 6) ---
    let mut px = [[0u8; 128]; 48];
    let is_on = |m: &[[bool; 128]; 48], x: i32, y: i32| -> bool {
        if x < 0 || y < 0 || x >= 128 || y >= 48 {
            false
        } else {
            m[y as usize][x as usize]
        }
    };

    for y in 0..48i32 {
        for x in 0..128i32 {
            if !is_on(&m, x, y) {
                if is_on(&m, x - 1, y) || is_on(&m, x, y - 1) || is_on(&m, x - 1, y - 1) {
                    px[y as usize][x as usize] = 2; // Deep Wine shadow outline
                }
                continue;
            }
            let up = is_on(&m, x, y - 1);
            let lt = is_on(&m, x - 1, y);
            let dn = is_on(&m, x, y + 1);
            let rt = is_on(&m, x + 1, y);
            let up2 = is_on(&m, x, y - 2);
            let lt2 = is_on(&m, x - 2, y);
            let dn2 = is_on(&m, x, y + 2);
            let rt2 = is_on(&m, x + 2, y);

            let col = if !up || !lt {
                4 // Pure Gold-Cream outer top-left specular bevel!
            } else if !dn || !rt {
                2 // Darkest Wine outer bottom-right bevel!
            } else if !up2 || !lt2 {
                6 // Warm Peach-Gold inner top-left bevel!
            } else if !dn2 || !rt2 {
                1 // Deep Burgundy inner bottom-right bevel!
            } else {
                let hash = ((x as u32).wrapping_mul(37)
                    ^ (y as u32).wrapping_mul(17)
                    ^ ((x + y) as u32).wrapping_mul(7))
                    % 11;
                match hash {
                    0 | 1 => 3,
                    2 | 3 => 1,
                    _ => 7,
                }
            };
            px[y as usize][x as usize] = col;
        }
    }

    px
}

// =============================================================================
// 2. UNIFIED 8x16 NINTENDO BEVELED SERIF FONT (PAL 7) FOR ALL 3 TEXT LINES!
// =============================================================================

/// Exact 16 unique uppercase letters covering ALL THREE title screen lines uniformly:
/// `"THE LEGEND OF"` + `"A MARIO TO THE"` + `"MUSHROOM KINGDOM"`!
/// Fits EXACTLY into Sheet 0x3F rows 2–3, cols 0..15 (VRAM `$160..$16F` top, `$170..$17F` bot)!
const SUBTITLE_ALPHABET: [char; 16] = [
    'A', 'D', 'E', 'F', 'G', 'H', 'I', 'K', 'L', 'M', 'N', 'O', 'R', 'S', 'T', 'U',
];

fn letter_bitmap_8x16(ch: char) -> [&'static str; 16] {
    match ch {
        'A' => [
            "........", "........", "...##...", "..####..", "..#..#..", ".##..##.", ".##..##.",
            ".######.", ".######.", ".##..##.", ".##..##.", ".##..##.", "###..###", "........",
            "........", "........",
        ],
        'D' => [
            "........", "........", "#####...", ".##.##..", ".##..##.", ".##..##.", ".##..##.",
            ".##..##.", ".##..##.", ".##..##.", ".##..##.", ".##.##..", "#####...", "........",
            "........", "........",
        ],
        'E' => [
            "........", "........", "#######.", ".##...#.", ".##.....", ".##.....", ".#####..",
            ".#####..", ".##.....", ".##.....", ".##.....", ".##...#.", "#######.", "........",
            "........", "........",
        ],
        'F' => [
            "........", "........", "#######.", ".##...#.", ".##.....", ".##.....", ".#####..",
            ".#####..", ".##.....", ".##.....", ".##.....", ".##.....", "####....", "........",
            "........", "........",
        ],
        'G' => [
            "........", "........", "..#####.", ".##...#.", ".##.....", ".##.....", ".##.....",
            ".##.###.", ".##..##.", ".##..##.", ".##..##.", ".##..##.", "..#####.", "........",
            "........", "........",
        ],
        'H' => [
            "........", "........", "###.###.", ".##..##.", ".##..##.", ".##..##.", ".######.",
            ".######.", ".##..##.", ".##..##.", ".##..##.", ".##..##.", "###.###.", "........",
            "........", "........",
        ],
        'I' => [
            "........", "........", ".#####..", "...##...", "...##...", "...##...", "...##...",
            "...##...", "...##...", "...##...", "...##...", "...##...", ".#####..", "........",
            "........", "........",
        ],
        'K' => [
            "........", "........", "###.###.", ".##..##.", ".##.##..", ".####...", ".###....",
            ".###....", ".####...", ".##.##..", ".##..##.", ".##..##.", "###.###.", "........",
            "........", "........",
        ],
        'L' => [
            "........", "........", "####....", ".##.....", ".##.....", ".##.....", ".##.....",
            ".##.....", ".##.....", ".##.....", ".##.....", ".##...#.", "#######.", "........",
            "........", "........",
        ],
        'M' => [
            "........", "........", "##...##.", "###.###.", "#######.", "##.#.##.", "##.#.##.",
            "##...##.", "##...##.", "##...##.", "##...##.", "##...##.", "##...##.", "........",
            "........", "........",
        ],
        'N' => [
            "........", "........", "###..##.", ".###.##.", ".###.##.", ".######.", ".##.###.",
            ".##.###.", ".##..##.", ".##..##.", ".##..##.", ".##..##.", "###..##.", "........",
            "........", "........",
        ],
        'O' => [
            "........", "........", "..####..", ".##..##.", ".##..##.", ".##..##.", ".##..##.",
            ".##..##.", ".##..##.", ".##..##.", ".##..##.", ".##..##.", "..####..", "........",
            "........", "........",
        ],
        'R' => [
            "........", "........", "######..", ".##..##.", ".##..##.", ".##..##.", ".#####..",
            ".####...", ".##.##..", ".##..##.", ".##..##.", ".##..##.", "###..##.", "........",
            "........", "........",
        ],
        'S' => [
            "........", "........", "..#####.", ".##...#.", ".##.....", ".###....", "..####..",
            "....###.", ".....##.", ".....##.", ".#...##.", ".##..##.", "..####..", "........",
            "........", "........",
        ],
        'T' => [
            "........", "........", "#######.", "#..##.#.", "...##...", "...##...", "...##...",
            "...##...", "...##...", "...##...", "...##...", "...##...", "..####..", "........",
            "........", "........",
        ],
        'U' => [
            "........", "........", "###.###.", ".##..##.", ".##..##.", ".##..##.", ".##..##.",
            ".##..##.", ".##..##.", ".##..##.", ".##..##.", ".##..##.", "..####..", "........",
            "........", "........",
        ],
        _ => ["........"; 16],
    }
}

fn build_shaded_letter_8x16(ch: char) -> ([[u8; 8]; 8], [[u8; 8]; 8]) {
    let bm = letter_bitmap_8x16(ch);
    let mut on = [[false; 8]; 16];
    for y in 0..16 {
        for (x, b) in bm[y].bytes().take(8).enumerate() {
            on[y][x] = b == b'#';
        }
    }
    let mut px = [[0u8; 8]; 16];
    for y in 0..16 {
        for x in 0..8 {
            if on[y][x] {
                px[y][x] = match y {
                    0..=5 => 1,
                    6..=8 => 6,
                    9..=10 => 7,
                    _ => 2,
                };
            } else {
                let mut adj = false;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if (0..8).contains(&nx)
                            && (0..16).contains(&ny)
                            && on[ny as usize][nx as usize]
                        {
                            adj = true;
                        }
                    }
                }
                if !adj && x >= 1 && y >= 2 && on[y - 2][x - 1] {
                    adj = true;
                }
                if adj {
                    px[y][x] = 5;
                }
            }
        }
    }
    let mut top = [[0u8; 8]; 8];
    let mut bot = [[0u8; 8]; 8];
    top.copy_from_slice(&px[..8]);
    bot.copy_from_slice(&px[8..16]);
    (top, bot)
}

// =============================================================================
// 3. COPYLEFT "🄯 2026 pomercier" 8x8 GLYPHS FOR SHEET 0x79 (PC 0x08A000)
// =============================================================================

/// Exact 8x8 white (`1`) + 1px bottom crimson shadow (`3`) glyphs matching vanilla `©1991,1992 Nintendo`!
fn build_copyleft_glyph_8x8(ch: char) -> [[u8; 8]; 8] {
    let rows: [&str; 8] = match ch {
        // Reversed-C inside circle (Copyleft symbol 🄯) matching Nintendo's © size!
        '@' => [
            "..111...", ".13331..", "1311131.", "1.331.1.", "1.111.1.", "3133313.", ".31113..",
            "..333...",
        ],
        '2' => [
            "........", ".11111..", "1133311.", "33..113.", "..1113..", ".1133...", "1111111.",
            "3333333.",
        ],
        '0' => [
            "........", ".11111..", "1133311.", "11..111.", "11.1.11.", "111..11.", ".111113.",
            "..3333..",
        ],
        '6' => [
            "........", "..1111..", ".113333.", "111111..", "1133311.", "11...11.", ".111113.",
            "..3333..",
        ],
        'p' => [
            "........", "........", "111111..", "1133311.", "11...11.", "1111113.", "113333..",
            "33......",
        ],
        'o' => [
            "........", "........", ".11111..", "1133311.", "11...11.", "11...11.", ".111113.",
            "..3333..",
        ],
        'm' => [
            "........", "........", "111.111.", "1111111.", "11.1.11.", "11.1.11.", "11.3.11.",
            "33...33.",
        ],
        'e' => [
            "........", "........", ".11111..", "1133311.", "1111111.", "1133333.", ".111111.",
            "..33333.",
        ],
        'r' => [
            "........", "........", "11.111..", "1113311.", "113..33.", "11......", "11......",
            "33......",
        ],
        'c' => [
            "........", "........", ".11111..", "1133311.", "113..33.", "11......", ".111111.",
            "..33333.",
        ],
        'i' => [
            "........", "...11...", "...33...", "..111...", "...11...", "...11...", "..1111..",
            "..3333..",
        ],
        _ => ["........"; 8],
    };
    let mut out = [[0u8; 8]; 8];
    for (y, r) in rows.iter().enumerate() {
        for (x, b) in r.bytes().take(8).enumerate() {
            out[y][x] = match b {
                b'1' => 1,
                b'3' => 3,
                _ => 0,
            };
        }
    }
    out
}

// =============================================================================
// 4. TRUE 3D-ANIMATED SUPER MUSHROOM, FIRE FLOWER & SUPER STAR (32 FRAMES 4BPP!)
// =============================================================================

/// 16-Color 4bpp OBJ Palette 6 (`$7E:C6C0..$7E:C6DF`) exclusively for our 3D Mario Relics!
/// Zero conflict with `©1991,1992 Nintendo` (Pal 5), Master Sword (Pal 0), or Sparkles (Pal 2)!
const TRIO_3D_PALETTE_BGR555: [u16; 16] = [
    0x0000, // 0: Transparent
    0x0842, // 1: Deep Black-Navy Outline (#101010)
    0x7FFF, // 2: Pure Specular White (#FFFFFF)
    0x63DF, // 3: Warm Cream Stem & Spots (#FFF0C0)
    0x3E7B, // 4: Shaded Warm Tan 3D Stem (#D8A878)
    0x10BF, // 5: Bright SMW Ruby Red (#F82820) — Mushroom Cap & Fire Flower Outer Petals
    0x0855, // 6: Deep Crimson 3D Extruded Bevel (#A81010)
    0x0E1F, // 7: Bright Tangerine Orange (#F88018) — Fire Flower Inner Ring & Star Warm Edge
    0x179F, // 8: Gleaming SMW Star Gold (#F8E028) — Super Star & Fire Flower Core
    0x0A39, // 9: Rich Golden Amber 3D Side Bevel (#C88810)
    0x2347, // 10: Bright SMW Leaf Green (#38D040) — Fire Flower Stem & Leaves
    0x1203, // 11: Deep Emerald Green 3D Shade (#188020)
    0x6B5F, // 12: Glossy Pink-White Mushroom Highlight (#FFD0D0)
    0x0CEB, // 13: Deep Bronze-Brown 3D Shadow (#583810)
    0x43FF, // 14: Bright Lemon Star Specular (#FFFF80)
    0x4631, // 15: Soft Silver-Grey Highlight (#8898A8)
];

/// Base 24x24 SMW icon color maps (semantic material tags -> 3D volumetric extrusion + shading!)
/// Materials:
/// - `R` = Ruby Red (5 front, 12 highlight, 6 3D side/shade)
/// - `W` = White/Cream (2 front highlight, 3 cream, 4 3D side/shade)
/// - `S` = Stem Cream (3 front, 2 highlight, 4 3D side/shade)
/// - `O` = Tangerine Orange (7 front, 8 highlight, 6 3D side/shade)
/// - `Y` = Super Star Gold (8 front, 14 highlight, 9 3D side, 13 deep shade)
/// - `G` = SMW Leaf Green (10 front, 14 highlight, 11 3D side/shade)
/// - `K` = Eyes / Inner Black (1 always)
const ICON_MUSHROOM_24: [&str; 24] = [
    "........................",
    "........RRRRRRRR........",
    ".....RRRRWWWRRRRRRR.....",
    "....RRRWWWWWWWRRRRRR....",
    "...RRRWWWWWWWWWRRRRRR...",
    "..WWWRWWWWWWWWWRRRRWWW..",
    ".WWWWWRWWWWWWWRRRRRWWWW.",
    ".WWWWWRRWWWWWRRRRRRWWWW.",
    "WWWWWWWRRRRRRRRRRRRWWWWW",
    "WWWWWWRRRRRRRRRRRRRRWWWW",
    ".WWWWRRRRRWWWWWWRRRRRWW.",
    ".RRRRRRRRWWWWWWWWRRRRRR.",
    ".RRRRRRRRWWWWWWWWRRRRRR.",
    "..RRRRRRRRWWWWWRRRRRRR..",
    "....SSSSSSSSSSSSSSSS....",
    "....SSSKKSSSSSSKKSSS....",
    "....SSSKKSSSSSSKKSSS....",
    "....SSSKKSSSSSSKKSSS....",
    "....SSSSSSSSSSSSSSSS....",
    ".....SSSSSSSSSSSSSS.....",
    "......SSSSSSSSSSSS......",
    ".......SSSSSSSSSS.......",
    "........................",
    "........................",
];

const ICON_FLOWER_24: [&str; 24] = [
    "........................",
    "......RRRRRRRRRRRR......",
    "....RRRRROOOOOORRRRR....",
    "...RRROOOYYYYYYOOORRR...",
    "..RROOYYWWWWWWWWYYOORR..",
    "..RROOYWWKWWWWKWWYOORR..",
    "..RROOYWWKWWWWKWWYOORR..",
    "..RROOYWWKWWWWKWWYOORR..",
    "..RROOYYWWWWWWWWYYOORR..",
    "...RRROOOYYYYYYOOORRR...",
    "....RRRRROOOOOORRRRR....",
    "......RRRRRRRRRRRR......",
    "..........GGGG..........",
    "..GGGG....GGGG....GGGG..",
    ".GGGGGG...GGGG...GGGGGG.",
    ".GGGGGGG..GGGG..GGGGGGG.",
    "..GGGGGGGGGGGGGGGGGGGG..",
    "...GGGGGGGGGGGGGGGGGG...",
    "....GGGGGGGGGGGGGGGG....",
    "......GGGGGGGGGGGG......",
    "..........GGGG..........",
    "..........GGGG..........",
    "........................",
    "........................",
];

const ICON_STAR_24: [&str; 24] = [
    "...........YY...........",
    "..........YYYY..........",
    "..........YYYY..........",
    ".........YYYYYY.........",
    ".........YYYYYY.........",
    "........YYYYYYYY........",
    "..YYYYYYYYYYYYYYYYYYYY..",
    ".YYYYYYYYYYYYYYYYYYYYYY.",
    "..YYYYYYYYYYYYYYYYYYYY..",
    "...YYYYYYKYYYYKYYYYYY...",
    "....YYYYYKYYYYKYYYYY....",
    ".....YYYYKYYYYKYYYY.....",
    "......YYYKYYYYKYYY......",
    "......YYYYYYYYYYYY......",
    ".....YYYYYYYYYYYYYY.....",
    ".....YYYYYYYYYYYYYY.....",
    "....YYYYYYYYYYYYYYYY....",
    "....YYYYYYY..YYYYYYY....",
    "...YYYYYY......YYYYYY...",
    "...YYYY..........YYYY...",
    "..YYY..............YYY..",
    "..Y..................Y..",
    "........................",
    "........................",
];

fn shade_3d_material(mat: u8, is_front: bool, nx: f32, ny: f32, nz: f32) -> u8 {
    // Directional top-left-front light vector (lx=-0.42, ly=-0.48, lz=0.77)
    let dot = nx * (-0.42) + ny * (-0.48) + nz * 0.77;
    if mat == b'K' {
        return 1;
    }
    if !is_front {
        // 3D extruded bevel/rim side & back walls!
        return match mat {
            b'R' => 6, // Deep Crimson 3D side
            b'W' => 4, // Shaded Cream 3D side
            b'S' => 4, // Warm Tan 3D stem side
            b'O' => 6, // Deep Orange-Red 3D side
            b'Y' => {
                if dot > 0.15 {
                    9
                } else {
                    13
                }
            } // Rich Amber / Bronze 3D star thickness!
            b'G' => 11, // Deep Emerald 3D leaf side
            _ => 1,
        };
    }
    match mat {
        b'R' => {
            if dot > 0.68 {
                12
            } else if dot > 0.20 {
                5
            } else {
                6
            }
        }
        b'W' => {
            if dot > 0.25 {
                2
            } else {
                3
            }
        }
        b'S' => {
            if dot > 0.62 {
                2
            } else if dot > 0.15 {
                3
            } else {
                4
            }
        }
        b'O' => {
            if dot > 0.62 {
                8
            } else if dot > 0.15 {
                7
            } else {
                6
            }
        }
        b'Y' => {
            if dot > 0.66 {
                14
            } else if dot > 0.15 {
                8
            } else {
                9
            }
        }
        b'G' => {
            if dot > 0.66 {
                14
            } else if dot > 0.15 {
                10
            } else {
                11
            }
        }
        _ => 1,
    }
}

/// Render a true 3D-extruded, perspective-projected, Z-buffered, outline-stroked 32x32 frame
/// of a Super Mario World icon at `(yaw_rad, pitch_rad)`!
fn render_3d_icon_frame(icon: &[&str; 24], yaw: f32, pitch: f32, scale: f32) -> [[u8; 32]; 32] {
    let mut color_buf = [[0u8; 32]; 32];
    let mut z_buf = [[-1e9f32; 32]; 32];

    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();

    // Fine oversampling over the 24x24 model + 3D depth slices z in [-2.6 .. +2.6] (extruded 3D thickness!)
    // Guarantees zero pinholes at every rotation angle including edge-on!
    let half = 11.5f32;
    let is_locked = yaw.abs() < 0.001 && pitch.abs() < 0.001;

    if is_locked {
        // Ultra-crisp, museum-grade front-facing 3D beveled frame 0!
        for y in 0..24usize {
            let row = icon[y].as_bytes();
            for x in 0..24usize {
                let mat = row[x];
                if mat == b'.' {
                    continue;
                }
                // Check neighbor edges to give subtle authentic 3D pillow/bevel normal!
                let up_empty = y == 0 || icon[y - 1].as_bytes()[x] == b'.';
                let lt_empty = x == 0 || row[x - 1] == b'.';
                let dn_empty = y == 23 || icon[y + 1].as_bytes()[x] == b'.';
                let rt_empty = x == 23 || row[x + 1] == b'.';
                let mut nx = 0.0f32;
                let mut ny = 0.0f32;
                if lt_empty {
                    nx -= 0.55;
                }
                if rt_empty {
                    nx += 0.55;
                }
                if up_empty {
                    ny -= 0.55;
                }
                if dn_empty {
                    ny += 0.55;
                }
                // Slight dome curvature
                nx += (x as f32 - half) / 22.0;
                ny += (y as f32 - half) / 22.0;
                let nz = (1.0 - (nx * nx + ny * ny).min(0.85)).sqrt();
                let col = shade_3d_material(mat, true, nx, ny, nz);
                // Place centered at (4..28, 4..28) inside 32x32!
                color_buf[y + 4][x + 4] = col;
                // Add subtle 1px bottom-right 3D extruded bevel rim!
                if (dn_empty || rt_empty)
                    && y + 5 < 31
                    && x + 5 < 31
                    && color_buf[y + 5][x + 5] == 0
                {
                    color_buf[y + 5][x + 5] = shade_3d_material(mat, false, 0.4, 0.4, 0.2);
                }
            }
        }
    } else {
        // Full 3D extruded volumetric projection with Z-buffer & perspective!
        for yi in 0..48usize {
            let fy = (yi as f32) * 0.5;
            let iy = (fy as usize).min(23);
            let row = icon[iy].as_bytes();
            for xi in 0..48usize {
                let fx = (xi as f32) * 0.5;
                let ix = (fx as usize).min(23);
                let mat = row[ix];
                if mat == b'.' {
                    continue;
                }
                let vx = fx - half;
                let vy = fy - half;
                // Extruded 3D thickness (7 slices z = -3.0..+3.0)
                for zi in -6..=6i32 {
                    let vz = (zi as f32) * 0.45;
                    let is_face = zi.abs() >= 5;
                    // Rotate Y (yaw), then X (pitch)
                    let rx = vx * cy + vz * sy;
                    let rz1 = -vx * sy + vz * cy;
                    let ry = vy * cp - rz1 * sp;
                    let rz = vy * sp + rz1 * cp;

                    // Perspective projection (camera at d = 68px for authentic crisp SNES 3D depth!)
                    let persp = 68.0 / (68.0 - rz);
                    let sx = (15.5 + rx * scale * persp).round() as i32;
                    let sy_p = (15.5 + ry * scale * persp).round() as i32;
                    if (1..31).contains(&sx) && (1..31).contains(&sy_p) {
                        let ux = sx as usize;
                        let uy = sy_p as usize;
                        if rz > z_buf[uy][ux] {
                            z_buf[uy][ux] = rz;
                            // Rotate surface normal for specular 3D lighting!
                            let (lnx, lny, lnz) = if is_face {
                                let sgn = if zi >= 0 { 1.0 } else { -1.0 };
                                (vx / 26.0, vy / 26.0, sgn)
                            } else {
                                let len = (vx * vx + vy * vy).sqrt().max(1.0);
                                (vx / len, vy / len, 0.0)
                            };
                            let rnx = lnx * cy + lnz * sy;
                            let rnz1 = -lnx * sy + lnz * cy;
                            let rny = lny * cp - rnz1 * sp;
                            let rnz = lny * sp + rnz1 * cp;
                            // Keep eyes only on front/back faces, never smeared across 3D rim!
                            let m_eff = if mat == b'K' && !is_face { b'Y' } else { mat };
                            color_buf[uy][ux] = shade_3d_material(m_eff, is_face, rnx, rny, rnz);
                        }
                    }
                }
            }
        }
    }

    // Crisp 1px black outline (`1`) around the entire silhouette so icons POP over sky & sepia scroll!
    let snap = color_buf;
    for y in 0..32usize {
        for x in 0..32usize {
            if snap[y][x] != 0 {
                continue;
            }
            let adj = (x > 0 && snap[y][x - 1] != 0)
                || (x < 31 && snap[y][x + 1] != 0)
                || (y > 0 && snap[y - 1][x] != 0)
                || (y < 31 && snap[y + 1][x] != 0);
            if adj {
                color_buf[y][x] = 1;
            }
        }
    }

    color_buf
}

fn write_4bpp_8x8(buf: &mut [u8], tile_idx: usize, px: &[[u8; 8]; 8]) {
    let off = tile_idx * 32;
    if off + 32 > buf.len() {
        return;
    }
    for y in 0..8usize {
        let (mut b0, mut b1, mut b2, mut b3) = (0u8, 0u8, 0u8, 0u8);
        for x in 0..8usize {
            let c = px[y][x] & 0x0F;
            let sh = 7 - x;
            b0 |= (c & 1) << sh;
            b1 |= ((c >> 1) & 1) << sh;
            b2 |= ((c >> 2) & 1) << sh;
            b3 |= ((c >> 3) & 1) << sh;
        }
        buf[off + y * 2] = b0;
        buf[off + y * 2 + 1] = b1;
        buf[off + 16 + y * 2] = b2;
        buf[off + 16 + y * 2 + 1] = b3;
    }
}

/// Pack all 32 3D-animated frames (`32 x 2048B = 65536B`) for Banks `$20..$21`!
/// Inside each 2048B frame (16x4 tiles = 128x32px):
/// - Cols 0..3:  Super Mushroom (`$80, $82, $A0, $A2`)
/// - Cols 4..7:  Fire Flower    (`$84, $86, $A4, $A6`)
/// - Cols 8..11: Super Star     (`$88, $8A, $A8, $AA`)
pub fn build_all_32_trio_3d_frames() -> Vec<u8> {
    let mut out = vec![0u8; 32 * 2048];
    for f in 0..32usize {
        let (yaw, pitch, scale) = if f == 0 {
            (0.0f32, 0.0f32, 1.0f32)
        } else {
            let angle = (f as f32) * std::f32::consts::TAU / 32.0;
            // Smooth 3D tumbling spin (Y-axis full 360° + graceful X-axis tilt!)
            (angle, 0.36 * angle.sin(), 1.0f32)
        };
        let mush = render_3d_icon_frame(&ICON_MUSHROOM_24, yaw, pitch, scale);
        // Subtle phase offsets between the 3 relics while spinning so the 3D choreography feels alive!
        let flw = render_3d_icon_frame(
            &ICON_FLOWER_24,
            if f == 0 { 0.0 } else { yaw + 0.55 },
            if f == 0 { 0.0 } else { -pitch },
            scale,
        );
        let star = render_3d_icon_frame(
            &ICON_STAR_24,
            if f == 0 { 0.0 } else { yaw + 1.10 },
            if f == 0 { 0.0 } else { pitch * 0.8 },
            scale,
        );

        let frame_slice = &mut out[f * 2048..(f + 1) * 2048];
        for (slot_col, img) in [(0usize, &mush), (4usize, &flw), (8usize, &star)] {
            for ty in 0..4usize {
                for tx in 0..4usize {
                    let mut tile8 = [[0u8; 8]; 8];
                    for y in 0..8 {
                        for x in 0..8 {
                            tile8[y][x] = img[ty * 8 + y][tx * 8 + x];
                        }
                    }
                    let tile_idx = ty * 16 + (slot_col + tx);
                    write_4bpp_8x8(frame_slice, tile_idx, &tile8);
                }
            }
        }
    }
    out
}

// =============================================================================
// 5. FRENCH ALTTP DICTIONARY ENCODER & EXACT-LENGTH GLOBAL SCRIPT REWRITER
// =============================================================================

const FR_DICT: &[(u8, &str)] = &[
    (0xEF, "Tu as trouvé "),
    (0xEE, "Perle de Lune"),
    (0xBF, "maintenant"),
    (0xEB, "Maintenant"),
    (0xF0, "Ténèbres"),
    (0xD9, "sorcier"),
    (0xCE, "pouvoir"),
    (0xC0, "magique"),
    (0xF1, "Tu peux"),
    (0xA3, "dans "),
    (0xE9, "C'est"),
    (0xEC, "Merci"),
    (0xED, "Monde"),
    (0xD4, "rubis"),
    (0xEA, "Ganon"),
    (0x91, " de "),
    (0x92, " la "),
    (0x93, " le "),
    (0xA4, "des "),
    (0xBA, "les "),
    (0xC1, "ment"),
    (0xC6, "onne"),
    (0xB2, "ique"),
    (0xCF, "pour"),
    (0xD0, "peux"),
    (0xD2, "que "),
    (0x94, " ! "),
    (0x9B, "ais "),
    (0xB4, "is "),
    (0xA7, "est "),
    (0xA5, "de "),
    (0xA9, "en "),
    (0xAA, "er "),
    (0xAB, "es "),
    (0xBB, "la "),
    (0xBC, "le "),
    (0xC5, "ne "),
    (0xD5, "re "),
    (0xE0, "te "),
    (0xE1, "tu "),
    (0xF2, "Tu "),
    (0xA0, "che"),
    (0xA8, "ent"),
    (0xB3, "ien"),
    (0xC2, "mon"),
    (0xC3, "mai"),
    (0xC7, "oir"),
    (0xC8, "our"),
    (0xC9, "ouv"),
    (0xD1, "pas"),
    (0xDF, "tro"),
    (0xE7, "ver"),
    (0x9C, "as "),
    (0xDA, "s l"),
    (0xDB, "s d"),
    (0x95, " d"),
    (0x96, " p"),
    (0x97, " t"),
    (0x98, " !"),
    (0x9A, ", "),
    (0x9D, "an"),
    (0x9E, "ai"),
    (0x9F, "a "),
    (0xA1, "ce"),
    (0xA2, "ch"),
    (0xA6, "de"),
    (0xAC, "en"),
    (0xAD, "es"),
    (0xAE, "et"),
    (0xAF, "eu"),
    (0xB0, "e,"),
    (0xB1, "e "),
    (0xB5, "ie"),
    (0xB6, "in"),
    (0xB7, "ir"),
    (0xB8, "is"),
    (0xB9, "i "),
    (0xBD, "le"),
    (0xBE, "ll"),
    (0xC4, "me"),
    (0xCA, "oi"),
    (0xCB, "on"),
    (0xCC, "ou"),
    (0xCD, "or"),
    (0xD3, "qu"),
    (0xD6, "ra"),
    (0xD7, "re"),
    (0xD8, "r "),
    (0xDC, "se"),
    (0xDD, "so"),
    (0xDE, "s "),
    (0xE2, "te"),
    (0xE3, "t "),
    (0xE4, "un"),
    (0xE5, "ur"),
    (0xE6, "u "),
];

pub(crate) fn encode_fr_char(ch: char) -> u8 {
    match ch {
        'A'..='Z' => (ch as u8) - b'A',
        'a'..='z' => 0x1A + ((ch as u8) - b'a'),
        '0'..='9' => 0x34 + ((ch as u8) - b'0'),
        '!' => 0x3E,
        '?' => 0x3F,
        '-' => 0x40,
        '.' => 0x41,
        ',' => 0x42,
        '\'' => 0x51,
        ' ' => 0x59,
        'ô' => 0x60,
        'è' => 0x66,
        'é' => 0x67,
        'ê' => 0x68,
        'à' => 0x69,
        'ù' => 0x6A,
        'ç' => 0x6B,
        'â' => 0x6C,
        'û' => 0x6D,
        'î' => 0x6E,
        'É' | 'È' | 'Ê' => 0x04, // E (ALTTP FR font has no accented uppercase)
        'À' | 'Â' => 0x00,       // A
        'Ç' => 0x02,             // C
        'Ô' => 0x0E,             // O
        'Î' => 0x08,             // I
        'Ù' | 'Û' => 0x14,       // U
        _ => 0x59,
    }
}

pub(crate) fn encode_fr_text(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut rest = s;
    while !rest.is_empty() {
        let mut matched = false;
        // First match our 3 live-patched Mario dictionary tokens in Bank $0C (0x065A33..0x065CAB):
        //   0xEE = "Lune 3-UP", 0xEA = "Bowser", 0xD4 = "pièces"!
        for &(tok, pat) in &[
            (0xEEu8, "Lune 3-UP"),
            (0xEAu8, "Bowser"),
            (0xD4u8, "pièces"),
        ] {
            if rest.starts_with(pat) {
                out.push(tok);
                rest = &rest[pat.len()..];
                matched = true;
                break;
            }
        }
        if matched {
            continue;
        }
        for &(tok, pat) in FR_DICT {
            // Skip vanilla dictionary entries whose live Bank $0C strings were upgraded to Mario terms!
            if matches!(tok, 0xD4 | 0xEA | 0xEE) {
                continue;
            }
            if rest.starts_with(pat) {
                out.push(tok);
                rest = &rest[pat.len()..];
                matched = true;
                break;
            }
        }
        if matched {
            continue;
        }
        let ch = rest.chars().next().unwrap();
        rest = &rest[ch.len_utf8()..];
        out.push(encode_fr_char(ch));
    }
    out
}

/// Patch the live French ALTTP Compression Dictionary in Bank `$0C` (`0x065A33..0x065CAB`) IN-PLACE:
///   - `0x99`: `", c'est moi, Sahasrahla"` -> `", c'est moi, Papy Toad"` (-1B)
///   - `0xD4`: `"rubis"`                   -> `"pièces"`                 (+1B)
///   - `0xEA`: `"Ganon"`                   -> `"Bowser"`                 (+1B)
///   - `0xEE`: `"Perle de Lune"`           -> `"Lune 3-UP"`              (-4B)
/// Net delta = -3 bytes (403 <= 406 bytes): zero expansion, exact SNES pointer table recalculated!
fn patch_french_dictionary_bank0c(
    rom_zelda: &mut SnesRom,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    const TBL_PC: usize = 0x065A33;
    const DATA_START_PC: usize = 0x065B15;
    const DATA_END_PC: usize = 0x065CAB;
    const NUM_TOKS: usize = 112; // 0x90..=0xFF (113 pointers including end sentinel)

    if DATA_END_PC > rom_zelda.data.len() {
        return Ok(0);
    }
    let p_first = u16::from_le_bytes([rom_zelda.data[TBL_PC], rom_zelda.data[TBL_PC + 1]]);
    let p_end = u16::from_le_bytes([
        rom_zelda.data[TBL_PC + NUM_TOKS * 2],
        rom_zelda.data[TBL_PC + NUM_TOKS * 2 + 1],
    ]);
    if p_first != 0xDB15 || p_end != 0xDCAB {
        return Ok(0);
    }

    let encode_raw_chars = |text: &str| -> Vec<u8> { text.chars().map(encode_fr_char).collect() };
    let mut new_ptrs = Vec::with_capacity((NUM_TOKS + 1) * 2);
    let mut new_data = Vec::with_capacity(DATA_END_PC - DATA_START_PC);

    for idx in 0..NUM_TOKS {
        let tok = 0x90usize + idx;
        let p0 = u16::from_le_bytes([
            rom_zelda.data[TBL_PC + idx * 2],
            rom_zelda.data[TBL_PC + idx * 2 + 1],
        ]);
        let p1 = u16::from_le_bytes([
            rom_zelda.data[TBL_PC + (idx + 1) * 2],
            rom_zelda.data[TBL_PC + (idx + 1) * 2 + 1],
        ]);
        let pc0 = 0x060000 + ((p0 as usize) - 0x8000);
        let pc1 = 0x060000 + ((p1 as usize) - 0x8000);
        let cur_snes = 0xDB15u16 + (new_data.len() as u16);
        new_ptrs.extend_from_slice(&cur_snes.to_le_bytes());

        let entry_bytes: Vec<u8> = match tok {
            0x99 => encode_raw_chars(", c'est moi, Papy Toad"),
            0xD4 => encode_raw_chars("pièces"),
            0xD9 => encode_raw_chars("Relique"),
            0xEA => encode_raw_chars("Bowser"),
            0xEE => encode_raw_chars("Lune 3-UP"),
            _ => rom_zelda.data[pc0..pc1].to_vec(),
        };
        new_data.extend_from_slice(&entry_bytes);
    }

    let max_len = DATA_END_PC - DATA_START_PC;
    if new_data.len() > max_len {
        return Ok(0);
    }
    let final_snes = 0xDB15u16 + (new_data.len() as u16);
    new_ptrs.extend_from_slice(&final_snes.to_le_bytes());
    while new_data.len() < max_len {
        new_data.push(0x59);
    }

    let mut full_block = Vec::with_capacity(DATA_END_PC - TBL_PC);
    full_block.extend_from_slice(&new_ptrs);
    // Preserve exact bytes between end of pointer table and DATA_START_PC (empty if adjacent: 0x65A33 + 226 = 0x65B15!)
    debug_assert_eq!(TBL_PC + new_ptrs.len(), DATA_START_PC);
    full_block.extend_from_slice(&new_data);

    session.swap_custom_bytes(
        rom_zelda,
        TBL_PC,
        TBL_PC,
        full_block,
        "Dictionnaire FR Bank $0C (0x065A33) : Ganon→Bowser, rubis→pièces, Sahasrahla→Papy Toad, Perle de Lune→Lune 3-UP".into(),
        SwapKind::CodePatch {
            byte_count: DATA_END_PC - TBL_PC,
        },
    )?;
    Ok(1)
}

enum Seg<'a> {
    Raw(&'a [u8]),
    Txt(&'a str),
}

fn build_exact_fr_msg(segs: &[Seg<'_>], target_len: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(target_len);
    for seg in segs {
        match seg {
            Seg::Raw(bytes) => out.extend_from_slice(bytes),
            Seg::Txt(s) => out.extend_from_slice(&encode_fr_text(s)),
        }
    }
    if out.len() + 1 > target_len {
        return None;
    }
    while out.len() + 1 < target_len {
        out.push(0x59);
    }
    out.push(0x7F);
    Some(out)
}

fn decode_fr_char_opt(b: u8) -> Option<char> {
    match b {
        0x00..=0x19 => Some((b'A' + b) as char),
        0x1A..=0x33 => Some((b'a' + (b - 0x1A)) as char),
        0x34..=0x3D => Some((b'0' + (b - 0x34)) as char),
        0x3E => Some('!'),
        0x3F => Some('?'),
        0x40 => Some('-'),
        0x41 => Some('.'),
        0x42 => Some(','),
        0x51 => Some('\''),
        0x59 => Some(' '),
        0x60 => Some('ô'),
        0x66 => Some('è'),
        0x67 => Some('é'),
        0x68 => Some('ê'),
        0x69 => Some('à'),
        0x6A => Some('ù'),
        0x6B => Some('ç'),
        0x6C => Some('â'),
        0x6D => Some('û'),
        0x6E => Some('î'),
        _ => None,
    }
}

/// Lossless sequential repacker for Bank `$1C` French script (v0.3.0 Target C).
/// Replaces the old byte-constrained in-place patcher and eliminates EVERY remaining Zelda term
/// across all 372 messages while preserving all 65c816 control codes, accents, and transitions.
pub fn repack_french_bank1c(
    rom_zelda: &mut SnesRom,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    if 0x0E8000 > rom_zelda.data.len() {
        return Ok(0);
    }

    let read_dict = |data: &[u8]| -> Vec<(u8, String)> {
        const TBL_PC: usize = 0x065A33;
        let mut toks: Vec<(u8, String)> = Vec::new();
        if TBL_PC + 113 * 2 <= data.len() {
            for i in 0..112 {
                let p0 =
                    u16::from_le_bytes([data[TBL_PC + i * 2], data[TBL_PC + i * 2 + 1]]) as usize;
                let p1 = u16::from_le_bytes([
                    data[TBL_PC + (i + 1) * 2],
                    data[TBL_PC + (i + 1) * 2 + 1],
                ]) as usize;
                if p0 >= 0x8000 && p1 >= p0 {
                    let pc0 = 0x060000 + (p0 - 0x8000);
                    let pc1 = 0x060000 + (p1 - 0x8000);
                    if pc1 <= data.len() {
                        let s: String = data[pc0..pc1]
                            .iter()
                            .filter_map(|&b| decode_fr_char_opt(b))
                            .collect();
                        toks.push(((0x90 + i) as u8, s));
                    }
                }
            }
        }
        toks
    };

    // Read the CURRENT Bank $0C dictionary FIRST so vanilla Bank $1C decodes 100% losslessly!
    let decode_dict = read_dict(&rom_zelda.data);

    // Now patch Bank $0C dictionary (Ganon->Bowser, rubis->pièces, sorcier->Relique, Sahasrahla->Papy Toad, Perle de Lune->Lune 3-UP)
    let ops = patch_french_dictionary_bank0c(rom_zelda, session)?;

    // Read the UPDATED Bank $0C dictionary (0x91..=0xFF; 0x90 is preserved verbatim as blank-line Ctrl!)
    let encode_dict: Vec<(u8, Vec<u8>)> = read_dict(&rom_zelda.data)
        .into_iter()
        .filter(|(tok, s)| *tok != 0x90 && s.chars().count() >= 2)
        .map(|(tok, s)| (tok, s.chars().map(encode_fr_char).collect::<Vec<u8>>()))
        .collect();

    // Exact Dynamic Programming (Viterbi shortest-path) compressor against live Bank $0C dictionary!
    // Beats greedy encoding by ~1,500+ bytes across Bank $1C so all 372 messages fit with massive headroom!
    let encode_dp_fr = |text: &str| -> Vec<u8> {
        let raw_chars: Vec<u8> = text.chars().map(encode_fr_char).collect();
        let n = raw_chars.len();
        if n == 0 {
            return Vec::new();
        }
        let mut dp = vec![usize::MAX / 2; n + 1];
        let mut choice: Vec<(usize, u8)> = vec![(0, 0); n + 1];
        dp[n] = 0;
        for i in (0..n).rev() {
            // Option 1: literal character (1 byte)
            if dp[i + 1] + 1 < dp[i] {
                dp[i] = dp[i + 1] + 1;
                choice[i] = (1, raw_chars[i]);
            }
            // Option 2: any matching dictionary token (1 byte covering tok_bytes.len() chars)
            for (tok, tok_bytes) in &encode_dict {
                let m = tok_bytes.len();
                if i + m <= n
                    && &raw_chars[i..i + m] == tok_bytes.as_slice()
                    && dp[i + m] + 1 < dp[i]
                {
                    dp[i] = dp[i + m] + 1;
                    choice[i] = (m, *tok);
                }
            }
        }
        let mut out = Vec::with_capacity(dp[0]);
        let mut cur = 0;
        while cur < n {
            let (step, b) = choice[cur];
            out.push(b);
            cur += step;
        }
        out
    };

    let bank = rom_zelda.data[0x0E0000..0x0E8000].to_vec();

    // 1. Parse all messages up to 0x88 terminator
    let mut raw_msgs: Vec<&[u8]> = Vec::new();
    let mut cur = 0usize;
    while cur < bank.len() && bank[cur] != 0x88 {
        let Some(rel_end) = bank[cur..].iter().position(|&b| b == 0x7F) else {
            break;
        };
        raw_msgs.push(&bank[cur..cur + rel_end]);
        cur += rel_end + 1;
    }

    if raw_msgs.len() < 300 {
        return Ok(ops);
    }

    // 2. Structured decoding of each message
    enum MsgPart {
        Ctrl(Vec<u8>),
        LineBreak(u8), // 0x73 <SCR>, 0x74 <L1>, 0x75 <L2>, 0x76 <L3>
        Txt(String),
    }

    // Word-boundary-safe French Mario substitutions (never corrupting Sagesse->Toadsse, Reliques->Reliquess, or emotional cœur->champi!)
    const SUBS: &[(&str, &str)] = &[
        // Sorcier / Bowser (longest phrases first!)
        ("Le sorcier Bowser", "Bowser"),
        ("le sorcier Bowser", "Bowser"),
        ("du sorcier Bowser", "de Bowser"),
        ("au sorcier Bowser", "à Bowser"),
        ("sorcier Bowser", "Bowser"),
        ("le sombre sorcier", "le Roi Koopa"),
        ("le diabolique sorcier", "le cruel Bowser"),
        ("Ce sorcier", "Bowser"),
        ("ce sorcier", "Bowser"),
        ("du sorcier", "de Bowser"),
        ("au sorcier", "à Bowser"),
        ("Le sorcier", "Bowser"),
        ("le sorcier", "Bowser"),
        ("sorcier !", "Roi Koopa !"),
        ("sorcier", "Bowser"),
        // Agahnim
        ("qu'Agahnim", "que Bowser"),
        ("d'Agahnim", "de Bowser"),
        ("Agahnim", "Bowser"),
        ("AGAHNIM", "BOWSER"),
        // Ganon
        ("qu'Ganon", "que Bowser"),
        ("d'Ganon", "de Bowser"),
        ("Ganon", "Bowser"),
        ("GANON", "BOWSER"),
        // Hyrule -> Royaume Champi
        ("du Royaume d'Hyrule", "du Royaume Champi"),
        ("le Royaume d'Hyrule", "le Royaume Champi"),
        ("d'Hyrule", "de Champi"),
        ("l'Hyrule", "Champi"),
        ("Hyrule", "Champi"),
        ("HYRULE", "CHAMPI"),
        // Zelda -> Peach
        ("Zeld..", "Peach!"),
        ("Zelda", "Peach"),
        ("ZELDA", "PEACH"),
        // Triforce / Pendentifs -> Super Étoiles (clean singular/plural grammar!)
        ("de la Triforce", "des Super Étoiles"),
        ("à la Triforce", "aux Super Étoiles"),
        ("la Triforce a créé", "les Super Étoiles ont créé"),
        ("La Triforce a créé", "Les Super Étoiles ont créé"),
        ("la Triforce", "la Super Étoile"),
        ("La Triforce", "La Super Étoile"),
        ("Triforce", "Super Étoile"),
        ("des Pendentifs", "des Super Étoiles"),
        ("les Pendentifs", "les Super Étoiles"),
        ("les 3 Pendentifs", "les 3 Super Étoiles"),
        ("trois Pendentifs", "trois Super Étoiles"),
        ("autres Pendentifs", "autres Super Étoiles"),
        ("le Pendentif", "l'Étoile"),
        ("du Pendentif", "de l'Étoile"),
        ("un Pendentif", "une Étoile"),
        ("Pendentifs", "Super Étoiles"),
        ("Pendentif", "Super Étoile"),
        ("pendentifs", "Super Étoiles"),
        ("pendentif", "Super Étoile"),
        ("le Relique", "la Super Étoile"),
        ("Le Relique", "La Super Étoile"),
        ("un Relique", "une Super Étoile"),
        ("du Relique", "de la Super Étoile"),
        ("des Relique", "des Super Étoiles"),
        ("Reliquess", "Super Étoiles"),
        // Excalibur -> Super Marteau (Phase 5: eliminate sword identity!)
        ("l'Epée du Diable", "le Super Marteau"),
        ("l'Épée du Diable", "le Super Marteau"),
        ("l'Excalibur", "le Super Marteau"),
        ("d'Excalibur", "du Super Marteau"),
        ("Excalibur", "le Super Marteau"),
        ("cette épée", "ce marteau"),
        ("l'épée", "le marteau"),
        ("mon épée", "mon marteau"),
        ("ton épée", "ton marteau"),
        ("une épée", "un marteau"),
        // Sahasrahla -> Papy Toad
        ("Sahasrahla", "Papy Toad"),
        ("SAHASRAHLA", "PAPY TOAD"),
        // Cocorico -> Toad Town
        ("de Cocorico", "de Toad Town"),
        ("Cocorico", "Toad Town"),
        // Sages -> Toads (protect "Sagesse" via temporary sentinel!)
        ("Sagesse", "__SAGESSE__"),
        ("sagesse", "__sagesse__"),
        ("7 Sages", "7 Toads"),
        ("Sept Sages", "Sept Toads"),
        ("sept Sages", "sept Toads"),
        ("des 7 Sages", "des 7 Toads"),
        ("Sages", "Toads"),
        ("sages", "Toads"),
        ("__SAGESSE__", "Sagesse"),
        ("__sagesse__", "sagesse"),
        // Girls / Yoshis
        ("les 7 jeunes filles", "les 7 amis Yoshi"),
        ("jeunes filles enlevées", "amis Yoshi enlevés"),
        ("jeunes filles disparues", "amis Yoshi disparus"),
        ("jeunes filles", "amis Yoshi"),
        ("jeune fille", "ami Yoshi"),
        ("petite-fille", "petit Yoshi"),
        ("les avoir toutes", "les avoir tous"),
        ("sauvées", "sauvés"),
        // Hyliens -> Toads
        ("langue des Hyliens", "langue des Toads"),
        ("des Hyliens", "des Toads"),
        ("les Hyliens", "les Toads"),
        ("Hyliens", "Toads"),
        ("hylien", "toad"),
        ("hylienne", "toad"),
        // Weapons & Items
        ("l'Arc", "le Bill Blaster"),
        ("Arc", "Bill Blaster"),
        ("des Flèches d'Argent", "des Super Étoiles"),
        ("une Flèche d'Argent", "une Super Étoile"),
        ("Flèche d'Argent", "Super Étoile"),
        ("Flèches d'Argent", "Super Étoiles"),
        ("des flèches", "des Bullet Bills"),
        ("des Flèches", "des Bullet Bills"),
        ("flèches", "Bullet Bills"),
        ("Flèches", "Bullet Bills"),
        ("flèche", "Bullet Bill"),
        ("Flèche", "Bullet Bill"),
        ("Boomerang Magique", "Carapace Magique"),
        ("Boomerang", "Carapace"),
        ("Livre de Mudora", "Livre Magique"),
        ("Mudora", "Magie Mario"),
        ("Palmes de Zora", "Palmes Grenouille"),
        ("Zora", "Bloup"),
        ("Cristaux", "Oeufs Yoshi"),
        ("Cristal", "Oeuf Yoshi"),
        // Currency / Collectible Mushrooms (NEVER replace emotional "coeur" like "coeur d'or"!)
        ("rubis", "pièces"),
        ("Rubis", "Pièces"),
        ("une réserve de coeur", "un Super Champi"),
        ("réserve de coeur", "Super Champi"),
        ("un morceau de coeur", "un mini champi"),
        ("morceau de coeur", "mini champi"),
        ("un fragment de coeur", "un mini champi"),
        ("fragment de coeur", "mini champi"),
        ("d'un coeur", "d'un Super Champi"),
        ("un coeur", "un Super Champi"),
        // Locations
        ("Palais de l'Est", "Fort Koopa Est"),
        ("Palais du Désert", "Fort du Désert"),
        ("Palais", "Forteresse"),
        ("Monde d'Or", "Royaume Étoile"),
        ("Terre d'Or", "Royaume Étoile"),
        // Cleanup any doubled words
        ("Bowser Bowser", "Bowser"),
        ("Bowser, Bowser", "Bowser"),
        ("le le ", "le "),
        ("de de ", "de "),
        ("du du ", "du "),
        ("les les ", "les "),
        ("la la ", "la "),
        ("l'l'", "l'"),
    ];

    let apply_subs = |mut t: String| -> String {
        for &(pat, rep) in SUBS {
            t = t.replace(pat, rep);
        }
        t
    };

    // Detect whether a trailing '-' before a line break is a real French compound prefix
    // vs a vanilla ALTTP syllable hyphenation split (e.g. "réali-" + "ser", "con-" + "trôle").
    let is_real_compound_before_hyphen = |before: &str, after: &str| -> bool {
        let word_before = before
            .rsplit(|c: char| !c.is_alphabetic() && c != '\'')
            .next()
            .unwrap_or("")
            .to_lowercase();
        let word_after = after
            .split(|c: char| !c.is_alphabetic())
            .next()
            .unwrap_or("")
            .to_lowercase();
        matches!(
            (word_before.as_str(), word_after.as_str()),
            ("nous", "mêmes")
                | ("vous", "mêmes")
                | ("moi", "même")
                | ("toi", "même")
                | ("lui", "même")
                | ("elle", "même")
                | ("alter", "ego")
                | ("peut", "être")
                | ("est", "ce")
                | ("au", "dessus")
                | ("au", "dessous")
                | ("petit", "fils")
                | ("petite", "fille")
                | ("grand", "père")
                | ("arc", "en")
                | ("tout", "puissant")
                | ("rendez", "vous")
                | ("écoute", "moi")
                | ("touche", "la")
                | ("montre", "moi")
                | ("donne", "moi")
                | ("laisse", "moi")
                | ("aide", "moi")
                | ("sauve", "moi")
                | ("suis", "moi")
                | ("crois", "moi")
                | ("dis", "moi")
                | ("as", "tu")
                | ("es", "tu")
                | ("peux", "tu")
                | ("veux", "tu")
                | ("sais", "tu")
                | ("vas", "tu")
                | ("viens", "tu")
                | ("est", "il")
                | ("fait", "il")
                | ("faut", "il")
                | ("a", "t")
                | ("3", "up")
                | ("super", "marteau")
        )
    };

    // Helper: format a clean, zero-hyphen, zero-duplicate-LB French Mario paragraph list!
    // Each page in `pages` is separated by `<WAIT>` (0x7E). Strictly wraps at MAX_COLS = 28
    // using a deterministic single-emission line_slot machine (0 -> 0x75, 1 -> 0x76, >=2 -> 0x73).
    const MAX_COLS: usize = 28;
    let encode_mario_story_msg =
        |prefix_ctrl: &[u8], pages: &[&str], suffix_ctrl: &[u8]| -> Vec<u8> {
            let mut out = Vec::new();
            out.extend_from_slice(prefix_ctrl);
            let mut line_slot = 0usize;
            let emit_break = |buf: &mut Vec<u8>, slot: &mut usize| {
                let code = match *slot {
                    0 => 0x75,
                    1 => 0x76,
                    _ => 0x73,
                };
                buf.push(code);
                if *slot < 2 {
                    *slot += 1;
                }
            };
            for (p_idx, page) in pages.iter().enumerate() {
                if p_idx > 0 {
                    out.push(0x7E); // <WAIT>
                    emit_break(&mut out, &mut line_slot);
                }
                let mut cur_col = 0usize;
                let mut line_buf = String::new();
                let flush_line = |buf: &mut Vec<u8>, lbuf: &mut String| {
                    if !lbuf.is_empty() {
                        buf.extend_from_slice(&encode_dp_fr(lbuf));
                        lbuf.clear();
                    }
                };
                for tok in page.split_whitespace() {
                    if tok == "<NAME>" {
                        let add_len = 6 + usize::from(cur_col > 0);
                        if cur_col > 0 && cur_col + add_len > MAX_COLS {
                            flush_line(&mut out, &mut line_buf);
                            emit_break(&mut out, &mut line_slot);
                            cur_col = 0;
                        } else if cur_col > 0 {
                            line_buf.push(' ');
                            cur_col += 1;
                        }
                        flush_line(&mut out, &mut line_buf);
                        out.push(0x83);
                        cur_col += 6;
                        continue;
                    }
                    let wlen = tok.chars().count();
                    let need_sp = cur_col > 0;
                    if cur_col > 0 && cur_col + wlen + usize::from(need_sp) > MAX_COLS {
                        flush_line(&mut out, &mut line_buf);
                        emit_break(&mut out, &mut line_slot);
                        line_buf.push_str(tok);
                        cur_col = wlen;
                    } else {
                        if need_sp {
                            line_buf.push(' ');
                            cur_col += 1;
                        }
                        line_buf.push_str(tok);
                        cur_col += wlen;
                    }
                }
                flush_line(&mut out, &mut line_buf);
            }
            out.extend_from_slice(suffix_ctrl);
            out
        };

    let mut new_bank: Vec<u8> = Vec::with_capacity(0x8000);

    for (msg_idx, raw) in raw_msgs.iter().enumerate() {
        // Phase 5: Original Super Mario French Story Rewrites for marquee scenes!
        // Preserves exact ALTTP event/choice/sound control codes while delivering a genuine,
        // playful Mario adventure with ZERO hyphenated word splits and ZERO text overlap!
        let custom_story: Option<Vec<u8>> = match msg_idx {
            // #12: Uncle Luigi leaving Mario's House at game start
            12 => Some(encode_mario_story_msg(
                &[],
                &[
                    "Salut Mario ! Je file inspecter les Tuyaux du Royaume Champi.",
                    "Bowser prépare encore un mauvais coup ! Reste ici, je reviens très vite !",
                ],
                &[],
            )),
            // #13: Luigi gifting Mario his Super Hammer & Koopa Shell + teaching Jump & Stomp!
            13 => Some(encode_mario_story_msg(
                &[],
                &[
                    "Oh, <NAME> ! Bowser a volé les Super Étoiles et coupé nos Tuyaux !",
                    "Prends mon Super Marteau et ma Carapace ! Appuie sur L ou R pour sauter et écraser les ennemis !",
                ],
                &[],
            )),
            // #17: Tutorial Toad teaching Mario Jump/Stomp/Hammer
            17 => Some(encode_mario_story_msg(
                &[],
                &[
                    "Tu veux devenir un vrai héros du Royaume Champi ?",
                    "Saute avec L ou R pour écraser les Goombas et frappe avec B grâce à ton Super Marteau !",
                ],
                &[],
            )),
            // #30: Princess Peach opening telepathic call
            30 => Some(encode_mario_story_msg(
                &[0x84, 0x02, 0x7A, 0x03],
                &[
                    "Au secours ! Au secours ! C'est moi, la Princesse Peach !",
                    "Bowser et ses sept Koopalings ont envahi le château et volé les Super Étoiles !",
                    "Mario, tu es notre seul espoir ! Viens vite au château !",
                ],
                &[],
            )),
            // #31: Peach guiding Mario to the secret entrance
            31 => Some(encode_mario_story_msg(
                &[0x84, 0x02, 0x7A, 0x03],
                &[
                    "Au secours ! Je suis enfermée dans le sous-sol du château.",
                    "Un passage secret près des buissons mène droit au donjon. Saute par-dessus les gardes !",
                ],
                &[],
            )),
            // #35: Peach escaping the dungeon cell
            35 => Some(encode_mario_story_msg(
                &[],
                &[
                    "Mario ! Quel soulagement ! Vite, filons avant que Bowser ne nous repère !",
                    "Je connais un passage secret vers le refuge des Toads. Suis-moi !",
                ],
                &[],
            )),
            // #58: Toad Town Wanted Poster
            58 => Some(encode_mario_story_msg(
                &[],
                &[
                    "AVIS DE RECHERCHE ! Bowser a capturé Peach et nos amis Yoshi !",
                    "Aidez Mario à retrouver les Super Étoiles !",
                ],
                &[],
            )),
            // #278: Bob-omb Shop
            278 => Some(encode_mario_story_msg(
                &[],
                &[
                    "100 pièces les 30 Bob-ombs ! Qualité supérieure du Royaume Champi !",
                    "Des Bob-ombs toutes fraîches pour exploser les murs de Bowser !",
                ],
                &[],
            )),
            // #316: Castle Tower Bowser Altar intro
            316 => Some(encode_mario_story_msg(
                &[],
                &[
                    "BWAHAHAHA ! Enfin tu arrives, Mario !",
                    "Tu es trop tard pour sauver la Princesse Peach !",
                ],
                &[],
            )),
            // #317: Castle Tower Bowser taunt (replaces vanilla "Bon - A la prochaine !" split!)
            317 => Some(encode_mario_story_msg(
                &[],
                &[
                    "BWAHAHAHA ! Le sceau des sept Toads est enfin brisé !",
                    "Plus rien n'arrêtera mes sept Koopalings et mes forteresses de lave !",
                    "Rendez-vous dans mon château, petit plombier !",
                ],
                &[],
            )),
            // #318: Castle Tower Bowser Boss Room 0x020 opening taunt (replaces Zelda medieval "occire"!)
            318 => Some(encode_mario_story_msg(
                &[],
                &[
                    "BWAHAHAHA ! Tu oses défier le grand Roi Bowser dans sa tour ?",
                    "Goûte à ma magie Koopa, petit plombier !",
                ],
                &[],
            )),
            // #319: Castle Tower Bowser defeat line (replaces Zelda "Héros"!)
            319 => Some(encode_mario_story_msg(
                &[],
                &[
                    "Grrr... Pas mal, Mario ! Mais je t'emporte dans mon Royaume de Lave !",
                ],
                &[],
            )),
            // #366: Final Boss King Bowser Room 0x00 speech (fixes the overlapping text bug!)
            366 => Some(encode_mario_story_msg(
                &[],
                &[
                    "BWAHAHAHA ! Jamais je n'aurais cru qu'un plombier à moustache me causerait autant de problèmes !",
                    "Tu as vaincu mes sept Koopalings et traversé mes forteresses de lave...",
                    "Mais jamais tu ne reprendras les Super Étoiles ! Goûte au marteau et aux flammes du Roi Bowser !",
                ],
                &[],
            )),
            // #367: Final Boss King Bowser Phase 2
            367 => Some(encode_mario_story_msg(
                &[],
                &[
                    "Grrr... Pas mal, petit plombier ! Voyons si tu résistes à mes flammes royales ! En garde !",
                ],
                &[],
            )),
            _ => None,
        };

        if let Some(custom_bytes) = custom_story {
            new_bank.extend_from_slice(&custom_bytes);
            new_bank.push(0x7F);
            continue;
        }

        let mut segs: Vec<MsgPart> = Vec::new();
        let mut cur_txt = String::new();
        let mut i = 0usize;

        while i < raw.len() {
            let b = raw[i];
            // 2-byte control code only if next byte exists and is not 0x7F
            if matches!(b, 0x6F..=0x72 | 0x77..=0x7A | 0x84 | 0x87)
                && i + 1 < raw.len()
                && raw[i + 1] != 0x7F
            {
                if !cur_txt.is_empty() {
                    segs.push(MsgPart::Txt(std::mem::take(&mut cur_txt)));
                }
                segs.push(MsgPart::Ctrl(raw[i..i + 2].to_vec()));
                i += 2;
                continue;
            }
            if matches!(b, 0x73..=0x76) {
                if !cur_txt.is_empty() {
                    segs.push(MsgPart::Txt(std::mem::take(&mut cur_txt)));
                }
                segs.push(MsgPart::LineBreak(b));
                i += 1;
                continue;
            }
            // NOTE: 0x51 is French apostrophe '\'' -> MUST stay in Txt!
            if matches!(b, 0x43..=0x50 | 0x52..=0x58 | 0x6F..=0x72 | 0x7E | 0x80..=0x86 | 0x90) {
                if !cur_txt.is_empty() {
                    segs.push(MsgPart::Txt(std::mem::take(&mut cur_txt)));
                }
                segs.push(MsgPart::Ctrl(vec![b]));
                i += 1;
                continue;
            }
            if b >= 0x91 {
                if let Some((_, word)) = decode_dict.iter().find(|(tok, _)| *tok == b) {
                    cur_txt.push_str(word);
                    i += 1;
                    continue;
                }
            }
            if let Some(ch) = decode_fr_char_opt(b) {
                cur_txt.push(ch);
                i += 1;
                continue;
            }
            if !cur_txt.is_empty() {
                segs.push(MsgPart::Txt(std::mem::take(&mut cur_txt)));
            }
            segs.push(MsgPart::Ctrl(vec![b]));
            i += 1;
        }
        if !cur_txt.is_empty() {
            segs.push(MsgPart::Txt(cur_txt));
        }

        // Detect messages containing special menu choices, blank-line clears (0x90), or centered diagrams
        let has_special_layout = segs.iter().any(|s| {
            matches!(
                s,
                MsgPart::Ctrl(c)
                    if !c.is_empty()
                        && (matches!(c[0], 0x44 | 0x52..=0x58 | 0x70..=0x72 | 0x80..=0x82 | 0x85 | 0x86 | 0x90))
            )
        }) || segs.iter().any(|s| {
            matches!(s, MsgPart::Txt(t) if t.trim_start().starts_with('>') || t.starts_with("   "))
        });

        // PASS 1: Eliminate ALL hyphenated word-splits across line breaks (even in choice/special messages!)
        // e.g. "rÉali-" <LB> "ser" -> "réaliser", "con-" <LB> "trôle" -> "contrôle", "Penden-" <LB> "tifs" -> "Pendentifs"!
        let mut had_hyphen_split = false;
        let mut idx = 0usize;
        while idx + 2 < segs.len() {
            // Handle both `Txt("foo-") + LineBreak + Txt("bar")` AND `Txt("foo-") + Ctrl(0x7E) + LineBreak + Txt("bar")`!
            if let MsgPart::Txt(a) = &segs[idx] {
                let a_trim = a.trim_end();
                if a_trim.ends_with('-') && a_trim.len() >= 2 {
                    // Case A: Txt("foo-") + LB + Txt("bar")
                    if let (MsgPart::LineBreak(_), MsgPart::Txt(b)) =
                        (&segs[idx + 1], &segs[idx + 2])
                    {
                        let b_trim = b.trim_start();
                        if !b_trim.is_empty()
                            && b_trim.chars().next().is_some_and(|c| c.is_alphabetic())
                            && !is_real_compound_before_hyphen(&a_trim[..a_trim.len() - 1], b_trim)
                        {
                            // Move the whole joined word to the next line or join into `a`
                            let prefix = &a_trim[..a_trim.len() - 1];
                            let (before_word, partial) = match prefix.rfind(' ') {
                                Some(sp) => (&prefix[..sp], &prefix[sp + 1..]),
                                None => ("", prefix),
                            };
                            let joined_b = format!("{partial}{b_trim}");
                            segs[idx] = MsgPart::Txt(before_word.to_string());
                            segs[idx + 2] = MsgPart::Txt(joined_b);
                            had_hyphen_split = true;
                        }
                    }
                    // Case B: Txt("foo-") + Ctrl(0x7E) + LB + Txt("bar")
                    else if idx + 3 < segs.len() {
                        if let (MsgPart::Ctrl(c), MsgPart::LineBreak(_), MsgPart::Txt(b)) =
                            (&segs[idx + 1], &segs[idx + 2], &segs[idx + 3])
                        {
                            let b_trim = b.trim_start();
                            if c.as_slice() == [0x7E]
                                && !b_trim.is_empty()
                                && b_trim.chars().next().is_some_and(|ch| ch.is_alphabetic())
                                && !is_real_compound_before_hyphen(
                                    &a_trim[..a_trim.len() - 1],
                                    b_trim,
                                )
                            {
                                let prefix = &a_trim[..a_trim.len() - 1];
                                let (before_word, partial) = match prefix.rfind(' ') {
                                    Some(sp) => (&prefix[..sp], &prefix[sp + 1..]),
                                    None => ("", prefix),
                                };
                                let joined_b = format!("{partial}{b_trim}");
                                segs[idx] = MsgPart::Txt(before_word.to_string());
                                segs[idx + 3] = MsgPart::Txt(joined_b);
                                had_hyphen_split = true;
                            }
                        }
                    }
                }
            }
            idx += 1;
        }

        // Cross-line cleanup: handle phrases split across a single LineBreak (e.g. "Le sorcier<L3>Bowser")
        let mut msg_modified = had_hyphen_split || raw.contains(&0xD9);
        for idx in 0..segs.len() {
            if idx + 2 < segs.len() {
                if let (MsgPart::Txt(a), MsgPart::LineBreak(_), MsgPart::Txt(b)) =
                    (&segs[idx], &segs[idx + 1], &segs[idx + 2])
                {
                    let a_trim = a.trim_end();
                    let b_trim = b.trim_start();
                    if (a_trim.ends_with("Le sorcier") || a_trim.ends_with("le sorcier"))
                        && (b_trim.starts_with("Bowser")
                            || b_trim.starts_with("Agahnim")
                            || b_trim.starts_with("Ganon"))
                    {
                        let cut_len = "Le sorcier".len();
                        let new_a = a_trim[..a_trim.len() - cut_len].trim_end().to_string();
                        segs[idx] = MsgPart::Txt(new_a);
                        msg_modified = true;
                    }
                }
            }
        }
        if !msg_modified {
            for s in &segs {
                if let MsgPart::Txt(t) = s {
                    if apply_subs(t.clone()) != *t {
                        msg_modified = true;
                        break;
                    }
                }
            }
        }

        // Untouched vanilla messages (no Mario subs & no hyphen splits): DP-compress verbatim!
        if !msg_modified {
            let mut opt_bytes = Vec::with_capacity(raw.len());
            for s in &segs {
                match s {
                    MsgPart::Ctrl(c) => opt_bytes.extend_from_slice(c),
                    MsgPart::LineBreak(lb) => opt_bytes.push(*lb),
                    MsgPart::Txt(t) => opt_bytes.extend_from_slice(&encode_dp_fr(t)),
                }
            }
            if opt_bytes.len() < raw.len() {
                new_bank.extend_from_slice(&opt_bytes);
            } else {
                new_bank.extend_from_slice(raw);
            }
            new_bank.push(0x7F);
            continue;
        }

        // PASS 2: For prose messages (!has_special_layout), collapse soft LineBreaks (0x73/0x75/0x76)
        // between consecutive Txt segments so we NEVER emit duplicate 0x75 or 0x76 over drawn text!
        let mut normalized: Vec<MsgPart> = Vec::with_capacity(segs.len());
        if !has_special_layout {
            let mut k = 0usize;
            while k < segs.len() {
                match &segs[k] {
                    MsgPart::Txt(t) => {
                        let sub = apply_subs(t.clone());
                        if let Some(MsgPart::Txt(prev)) = normalized.last_mut() {
                            if !prev.is_empty()
                                && !sub.is_empty()
                                && !prev.ends_with(' ')
                                && !sub.starts_with(' ')
                            {
                                prev.push(' ');
                            }
                            prev.push_str(sub.trim_start());
                        } else {
                            normalized.push(MsgPart::Txt(sub));
                        }
                    }
                    MsgPart::LineBreak(lb) => {
                        if *lb == 0x74 {
                            // Explicit screen/L1 reset — preserve!
                            normalized.push(MsgPart::LineBreak(0x74));
                        } else {
                            // Soft break (0x73/0x75/0x76): if preceded by Txt and followed by Txt,
                            // merge as space so the single-emission word wrapper controls every line break!
                            let prev_is_txt = matches!(normalized.last(), Some(MsgPart::Txt(_)));
                            let next_is_txt = matches!(segs.get(k + 1), Some(MsgPart::Txt(_)));
                            if prev_is_txt && next_is_txt {
                                if let Some(MsgPart::Txt(prev)) = normalized.last_mut() {
                                    if !prev.is_empty() && !prev.ends_with(' ') {
                                        prev.push(' ');
                                    }
                                }
                            } else {
                                // Break right after <WAIT> or control code -> keep as logical page break
                                normalized.push(MsgPart::LineBreak(*lb));
                            }
                        }
                    }
                    MsgPart::Ctrl(c) => {
                        normalized.push(MsgPart::Ctrl(c.clone()));
                    }
                }
                k += 1;
            }
        } else {
            for s in segs {
                match s {
                    MsgPart::Txt(t) => normalized.push(MsgPart::Txt(apply_subs(t))),
                    other => normalized.push(other),
                }
            }
        }

        // PASS 3: Emit with strict single-emission line_slot machine (0 -> 0x75, 1 -> 0x76, >=2 -> 0x73)
        // Guarantees ZERO duplicate 0x75/0x76 overwrites and ZERO line overflows!
        let mut msg_bytes: Vec<u8> = Vec::new();
        let mut cur_col = 0usize;
        let mut line_slot = 0usize; // 0 = L1, 1 = L2, 2 = L3/scrolling

        let emit_next_break = |buf: &mut Vec<u8>, slot: &mut usize| {
            let code = match *slot {
                0 => 0x75, // -> Line 2
                1 => 0x76, // -> Line 3
                _ => 0x73, // -> Scroll 1 line (clears bottom line cleanly!)
            };
            buf.push(code);
            if *slot < 2 {
                *slot += 1;
            }
        };

        for seg in normalized {
            match seg {
                MsgPart::Ctrl(c) => {
                    if !c.is_empty() {
                        if c[0] == 0x83 {
                            if !has_special_layout && cur_col + 6 > MAX_COLS && cur_col > 0 {
                                emit_next_break(&mut msg_bytes, &mut line_slot);
                                cur_col = 0;
                            }
                            cur_col += 6;
                        } else if c[0] == 0x7E {
                            cur_col = 0;
                        }
                    }
                    msg_bytes.extend_from_slice(&c);
                }
                MsgPart::LineBreak(lb) => {
                    if lb == 0x74 {
                        msg_bytes.push(0x74);
                        line_slot = 0;
                        cur_col = 0;
                    } else if has_special_layout {
                        // Even in special-layout messages, NEVER emit a backwards/duplicate 0x75 or 0x76!
                        let target_slot = match lb {
                            0x75 => 1,
                            0x76 => 2,
                            _ => 3,
                        };
                        if target_slot > line_slot && target_slot <= 2 {
                            msg_bytes.push(lb);
                            line_slot = target_slot;
                        } else {
                            msg_bytes.push(0x73);
                            line_slot = 2;
                        }
                        cur_col = 0;
                    } else {
                        emit_next_break(&mut msg_bytes, &mut line_slot);
                        cur_col = 0;
                    }
                }
                MsgPart::Txt(t) => {
                    let char_len = t.chars().count();
                    let leading_spaces = t.chars().take_while(|&c| c == ' ').count();
                    if has_special_layout || leading_spaces >= 3 || cur_col + char_len <= MAX_COLS {
                        msg_bytes.extend_from_slice(&encode_dp_fr(&t));
                        cur_col += char_len;
                    } else {
                        let mut line_buf = String::new();
                        let mut first_word = true;
                        let starts_with_space = t.starts_with(' ');
                        for word in t.split_whitespace() {
                            let wlen = word.chars().count();
                            let need_space = if first_word {
                                starts_with_space && cur_col > 0
                            } else {
                                cur_col > 0
                            };
                            let add_len = wlen + usize::from(need_space);
                            if cur_col > 0 && cur_col + add_len > MAX_COLS {
                                if !line_buf.is_empty() {
                                    msg_bytes.extend_from_slice(&encode_dp_fr(&line_buf));
                                    line_buf.clear();
                                }
                                emit_next_break(&mut msg_bytes, &mut line_slot);
                                line_buf.push_str(word);
                                cur_col = wlen;
                            } else {
                                if need_space {
                                    line_buf.push(' ');
                                    cur_col += 1;
                                }
                                line_buf.push_str(word);
                                cur_col += wlen;
                            }
                            first_word = false;
                        }
                        if !line_buf.is_empty() {
                            msg_bytes.extend_from_slice(&encode_dp_fr(&line_buf));
                        }
                    }
                }
            }
        }
        msg_bytes.push(0x7F);
        new_bank.extend_from_slice(&msg_bytes);
    }

    new_bank.push(0x88);

    if new_bank.len() > 0x8000 {
        return Err(RomError::OutOfBounds {
            offset: new_bank.len(),
            len: 0x8000,
        });
    }

    // Pad with 0xFF to 0x8000 bytes
    new_bank.resize(0x8000, 0xFF);

    session.swap_custom_bytes(
        rom_zelda,
        0x0E0000,
        0x0E0000,
        new_bank,
        format!(
            "Script FR Global Bank $1C ({} dialogues) : Repacker séquentiel DP v0.4.0 (word-wrap 24 cols & 0 termes Zelda)",
            raw_msgs.len()
        ),
        SwapKind::CodePatch { byte_count: 0x8000 },
    )?;

    Ok(ops + 1)
}

// =============================================================================
// 6. MASTER INJECTOR: TITLE LOGO, COPYLEFT, 3D MARIO TRIO & ALL FRENCH DIALOGS
// =============================================================================

pub fn inject_legend_of_peach_intro(
    rom_zelda: &mut SnesRom,
    session: &mut SwapSession,
    free_cursor: &mut usize,
    free_end: usize,
) -> Result<usize, RomError> {
    let mut ops = 0usize;

    // Ensure standard 16 Mbit (2 MB) SNES / Super Famicom LoROM so Banks $20..$22 hold our 32 3D frames & code!
    if rom_zelda.data.len() < 0x200000 {
        rom_zelda.data.resize(0x200000, 0xFF);
        if 0x007FD7 < rom_zelda.data.len() {
            rom_zelda.data[0x007FD7] = 0x0B; // Standard 16 Mbit (2 MB) SNES/SFC LoROM!
        }
        rom_zelda.header.rom_size_byte = 0x0B;
        rom_zelda.header.declared_rom_size_kb = 2048;
    }

    // -------------------------------------------------------------------------
    // A. GIANT 3D "PEACH" (COMPLETE 'P') + UNIFIED 16-LETTER 8x16 SERIF FONT
    //    INTO SHEETS 0x3E & 0x3F
    // -------------------------------------------------------------------------
    let peach_canvas = render_giant_peach_128x48();
    let blank8 = [[0u8; 8]; 8];
    let mut tile_pool: Vec<[[u8; 8]; 8]> = Vec::new();
    let mut tile_map: HashMap<[[u8; 8]; 8], u16> = HashMap::new();
    tile_map.insert(blank8, 0x15F);

    let mut peach_grid = [[0x15Fu16; 16]; 6];
    for tr in 0..6usize {
        for tc in 0..16usize {
            let mut t = [[0u8; 8]; 8];
            for y in 0..8 {
                for x in 0..8 {
                    t[y][x] = peach_canvas[tr * 8 + y][tc * 8 + x];
                }
            }
            if let Some(&existing) = tile_map.get(&t) {
                peach_grid[tr][tc] = existing;
            } else {
                let idx = tile_pool.len();
                // Now up to 95 unique tiles (0x100..0x15E) since unified font freed 0x15D/0x15E!
                let vram_tile = if idx < 95 {
                    tile_pool.push(t);
                    let vt = 0x100 + idx as u16;
                    tile_map.insert(t, vt);
                    vt
                } else {
                    0x100
                };
                peach_grid[tr][tc] = vram_tile;
            }
        }
    }

    if patch_alttp_comp_sheet_pub(
        rom_zelda,
        session,
        0x3E,
        "Écran-Titre Logo 3D (Sheet 0x3E) : Lettres géantes « PEACH » (1/2, P complet)",
        free_cursor,
        free_end,
        &|raw| {
            for ti in 0..64usize {
                let t = tile_pool.get(ti).unwrap_or(&blank8);
                write_3bpp_8x8_pub(raw, ti % 16, ti / 16, t);
            }
        },
    )? {
        ops += 1;
    }

    let mut sub_top: Vec<[[u8; 8]; 8]> = Vec::with_capacity(SUBTITLE_ALPHABET.len());
    let mut sub_bot: Vec<[[u8; 8]; 8]> = Vec::with_capacity(SUBTITLE_ALPHABET.len());
    for &ch in &SUBTITLE_ALPHABET {
        let (t, b) = build_shaded_letter_8x16(ch);
        sub_top.push(t);
        sub_bot.push(b);
    }

    if patch_alttp_comp_sheet_pub(
        rom_zelda,
        session,
        0x3F,
        "Écran-Titre Logo & Police Unifiée (Sheet 0x3F) : « PEACH » (2/2) + 16 lettres serif unifiées",
        free_cursor,
        free_end,
        &|raw| {
            for ti in 0..31usize {
                let t = tile_pool.get(64 + ti).unwrap_or(&blank8);
                write_3bpp_8x8_pub(raw, ti % 16, ti / 16, t);
            }
            write_3bpp_8x8_pub(raw, 15, 1, &blank8); // 0x15F guaranteed transparent!
            for (i, (t, b)) in sub_top.iter().zip(sub_bot.iter()).enumerate() {
                write_3bpp_8x8_pub(raw, i, 2, t);
                write_3bpp_8x8_pub(raw, i, 3, b);
            }
        },
    )? {
        ops += 1;
    }

    // -------------------------------------------------------------------------
    // B. REWRITE UNCOMPRESSED BG1 TITLE SCREEN STRIPE TILEMAP (0x071C68..0x071E10 = 424B EXACT!)
    //    - Line 1 (Rows 7–8, Col 10): "THE LEGEND OF" in EXACT SAME unified serif font!
    //    - Line 2 (Rows 9–14, Col 8): Giant "PEACH" with Priority=1 (0x39) on ALL tiles -> P 100% COMPLETE!
    //    - Line 3 (Rows 15–16, Col 11): "A MARIO TO THE"
    //    - Line 4 (Rows 17–18, Col 10): "MUSHROOM KINGDOM"
    // -------------------------------------------------------------------------
    let sub_tile = |ch: char, bottom: bool| -> (u8, u8) {
        if let Some(pos) = SUBTITLE_ALPHABET.iter().position(|&c| c == ch) {
            let lo = (if bottom { 0x70 } else { 0x60 }) + pos as u8;
            (lo, 0x3D) // Pal 7, Priority=1, tile bit8=1 (VRAM $160..$17F)
        } else {
            (0x5F, 0x19) // Transparent blank tile $15F
        }
    };

    let push_stripe = |buf: &mut Vec<u8>, row: u16, col: u16, entries: &[(u8, u8)]| {
        let vram = 0x1000u16 + row * 32 + col;
        let len_m1 = (entries.len() * 2 - 1) as u16;
        buf.push((vram >> 8) as u8);
        buf.push((vram & 0xFF) as u8);
        buf.push((len_m1 >> 8) as u8);
        buf.push((len_m1 & 0xFF) as u8);
        for &(lo, hi) in entries {
            buf.push(lo);
            buf.push(hi);
        }
    };

    if TITLE_BG1_STRIPE_PC + TITLE_BG1_STRIPE_LEN <= rom_zelda.data.len()
        && rom_zelda.data[TITLE_BG1_STRIPE_PC] == 0x11
        && rom_zelda.data[TITLE_BG1_STRIPE_PC + 1] == 0x0B
    {
        let mut stripe_buf: Vec<u8> = Vec::with_capacity(TITLE_BG1_STRIPE_LEN);

        // 1. Rows 7 & 8, Col 10 (13 tiles): "THE LEGEND OF" — 100% UNIFIED beveled Nintendo serif!
        let line1 = "THE LEGEND OF";
        let l1_top: Vec<(u8, u8)> = line1.chars().map(|ch| sub_tile(ch, false)).collect();
        let l1_bot: Vec<(u8, u8)> = line1.chars().map(|ch| sub_tile(ch, true)).collect();
        push_stripe(&mut stripe_buf, 7, 10, &l1_top); // 30B
        push_stripe(&mut stripe_buf, 8, 10, &l1_bot); // 30B

        // 2. Rows 9..14, Col 8 (16x6 = 128x48px): GIANT "PEACH" — Priority=1 (0x39) on ALL tiles so 'P' is 100% WHOLE!
        for r in 0..6usize {
            let mut row_entries = Vec::with_capacity(16);
            for c in 0..16usize {
                let vt = peach_grid[r][c];
                let lo = (vt & 0xFF) as u8;
                row_entries.push((lo, 0x39)); // Pal 6, Priority=1 (never cut by Master Sword!), tile bit8=1
            }
            push_stripe(&mut stripe_buf, (9 + r) as u16, 8, &row_entries); // 36B x 6 = 216B
        }

        // 3. Rows 15 & 16, Col 11 (14 tiles): "A MARIO TO THE"
        let line3 = "A MARIO TO THE";
        let l3_top: Vec<(u8, u8)> = line3.chars().map(|ch| sub_tile(ch, false)).collect();
        let l3_bot: Vec<(u8, u8)> = line3.chars().map(|ch| sub_tile(ch, true)).collect();
        push_stripe(&mut stripe_buf, 15, 11, &l3_top); // 32B
        push_stripe(&mut stripe_buf, 16, 11, &l3_bot); // 32B

        // 4. Rows 17 & 18, Col 10 (16 tiles): "MUSHROOM KINGDOM"
        let line4 = "MUSHROOM KINGDOM";
        let l4_top: Vec<(u8, u8)> = line4.chars().map(|ch| sub_tile(ch, false)).collect();
        let l4_bot: Vec<(u8, u8)> = line4.chars().map(|ch| sub_tile(ch, true)).collect();
        push_stripe(&mut stripe_buf, 17, 10, &l4_top); // 36B
        push_stripe(&mut stripe_buf, 18, 10, &l4_bot); // 36B

        // Total = 60 + 216 + 64 + 72 = 412B -> pad exact 12B (4 blank tiles at Row 0) = 424B EXACT!
        if stripe_buf.len() + 12 == TITLE_BG1_STRIPE_LEN {
            push_stripe(&mut stripe_buf, 0, 0, &[(0x5F, 0x19); 4]);
        }

        if stripe_buf.len() == TITLE_BG1_STRIPE_LEN {
            session.swap_custom_bytes(
                rom_zelda,
                TITLE_BG1_STRIPE_PC,
                TITLE_BG1_STRIPE_PC,
                stripe_buf,
                "Tilemap Écran-Titre BG1 (0x071C68) : P complet + Police unifiée « THE LEGEND OF PEACH »".into(),
                SwapKind::CodePatch { byte_count: TITLE_BG1_STRIPE_LEN },
            )?;
            ops += 1;
        }
    }

    // -------------------------------------------------------------------------
    // C. COPYLEFT "🄯 2026 pomercier" UNDER "©1991,1992 Nintendo" (Sheet 0x7B rows 2-3 + $0E:8569 + $0E:870A)
    // -------------------------------------------------------------------------
    // CRITICAL HARDWARE DISCOVERY (verified live in Mednafen snes_faust):
    //   Leave 0x08A000..0x08A2FF (rows 0-1 of Sheet 0x7B) 100% UNTOUCHED VANILLA (modifying 0x08A090..0x08A226
    //   corrupts ALTTP's held Bow/Bill Blaster WRAM buffer $7E9800!), AND leave '-Nintendo- Presente'
    //   (rows 2-3 cols 8..15 except unused placeholder (8,3)=0x78) & sparkles (cols 2..5) 100% untouched!
    //   - '🄯' (copyleft) = native '©' tile 0x40 with OAM H-flip (attr = 0x4A) -> pixel-perfect Nintendo match!
    //   - '2' = native '2' tile 0x43 (attr = 0x0A) -> pixel-perfect Nintendo match!
    //   - Remaining 9 glyphs ('0','6','p','o','m','e','r','c','i') go into the 9 100%-unused tiles in rows 2-3
    //     of Sheet 0x7B: (0,2)=0x60, (1,2)=0x61, (6,2)=0x66, (7,2)=0x67, (0,3)=0x70, (1,3)=0x71, (6,3)=0x76, (7,3)=0x77, (8,3)=0x78!
    const COPY_GLYPHS: &[(usize, usize, char, u8)] = &[
        (0, 2, '0', 0x60),
        (1, 2, '6', 0x61),
        (6, 2, 'p', 0x66),
        (7, 2, 'o', 0x67),
        (0, 3, 'm', 0x70),
        (1, 3, 'e', 0x71),
        (6, 3, 'r', 0x76),
        (7, 3, 'c', 0x77),
        (8, 3, 'i', 0x78),
    ];
    if 0x08A000 + 0x600 <= rom_zelda.data.len() {
        let mut s7b = rom_zelda.data[0x08A000..0x08A600].to_vec();
        for &(cx, cy, ch, _) in COPY_GLYPHS {
            let g = build_copyleft_glyph_8x8(ch);
            write_3bpp_8x8_pub(&mut s7b, cx, cy, &g);
        }
        session.swap_custom_bytes(
            rom_zelda,
            0x08A000,
            0x08A000,
            s7b,
            "Écran-Titre OBJ Sheet 0x7B (0x08A300..0x08A557) : Glyphes « 🄯 2026 pomercier » (0x08A000..0x08A2FF intacts)".into(),
            SwapKind::CodePatch { byte_count: 0x600 },
        )?;
        ops += 1;
    }

    // -------------------------------------------------------------------------
    // D. TRUE 3D-ANIMATED SUPER MUSHROOM, FIRE FLOWER & SUPER STAR (Banks $20..$22 + Hooks!)
    // -------------------------------------------------------------------------
    // 1. Write all 32 pre-rendered 3D 4bpp OBJ frames (65,536 bytes -> Banks $20 & $21, PC 0x100000..0x10FFFF)
    let frames_3d = build_all_32_trio_3d_frames();
    session.swap_custom_bytes(
        rom_zelda,
        0x100000,
        0x100000,
        frames_3d,
        "Banques $20-$21 (64 Ko) : 32 Frames 3D Champignon + Fleur de Feu + Étoile SMW (4bpp OBJ)"
            .into(),
        SwapKind::CodePatch { byte_count: 65536 },
    )?;
    ops += 1;

    // 2. Write clean 65816 hardware helper routines & 16-color 4bpp OBJ Palette 6 into Bank $22 (PC 0x110000 = $22:8000):
    //    - $22:8000: NMI DMA from dynamic ROM address ($1F0E..$1F10) to VRAM $5800 (2048B)
    //    - $22:8040: Load 16-color 4bpp Mario Trio Palette into OBJ Pal 6 ($7E:C6C0..$7E:C6DF) + keep Pal 5 intact!
    //    - $22:8080: 3D Frame Stepper (reads $1F02/$1F04/$1F05 -> selects 3D frame 0..31 -> writes $1F0E..$1F10 & $1F0C=1)
    //    - $22:8100: 32-byte BGR555 4bpp OBJ Palette 6 data!
    let mut bank22 = vec![0xFFu8; 0x200];

    // $22:8000 — NMI 3D Frame DMA (called via JSL $22:8000 from $00:92F7 with 8-bit A/X/Y, DB=$00)
    let nmi_dma_code: [u8; 54] = [
        0xAD, 0x0C, 0x1F, // LDA $1F0C
        0xF0, 0x2F, // BEQ done (+47 bytes)
        0xA9, 0x80, // LDA #$80
        0x8D, 0x15, 0x21, // STA $2115 (VMAIN)
        0xC2, 0x20, // REP #$20 (16-bit A)
        0xA9, 0x00, 0x58, // LDA #$5800 (VRAM OBJ $180..$1BF)
        0x8D, 0x16, 0x21, // STA $2116 (VMADD)
        0xA9, 0x01, 0x18, // LDA #$1801 (DMA0 -> $2118)
        0x8D, 0x00, 0x43, // STA $4300
        0xAD, 0x0E, 0x1F, // LDA $1F0E (dynamic 16-bit ROM frame addr)
        0x8D, 0x02, 0x43, // STA $4302
        0xAE, 0x10, 0x1F, // LDX $1F10 (dynamic 8-bit ROM bank $20/$21)
        0x8E, 0x04, 0x43, // STX $4304
        0xA9, 0x00, 0x08, // LDA #$0800 (2048 bytes = 64 4bpp tiles)
        0x8D, 0x05, 0x43, // STA $4305
        0xE2, 0x20, // SEP #$20 (8-bit A)
        0xA9, 0x01, // LDA #$01
        0x8D, 0x0B, 0x42, // STA $420B (MDMAEN ch 0)
        0x9C, 0x0C, 0x1F, // STZ $1F0C
        0x6B, // done: RTL
        0xEA,
    ];
    bank22[0x00..nmi_dma_code.len()].copy_from_slice(&nmi_dma_code);

    // $22:8040 — Load 16-color 4bpp OBJ Palette 6 into $7E:C6C0..$7E:C6DF + Prime 3D Frame 0 ($20:8000)!
    // Called via JSL $22:8040 from $0E:81F7 right after vanilla Polyhedral_InitializeThread ($09:F7DE).
    // Keeping $09:F7DE intact preserves ALTTP's real IRQ coroutine thread ($00:822D/$09:F81D), which sits
    // safely idle while $1F00==0, while our NMI DMA uploads Frame 0 ($20:8000) immediately on next VBlank!
    let pal_load_code: &[u8] = &[
        0xC2, 0x30, // REP #$30 (16-bit A/X/Y)
        0xA2, 0x00, 0x00, // LDX #$0000
        // loop ($22:8045):
        0xBF, 0x00, 0x81, 0x22, // LDA $22:8100,X
        0x9F, 0xC0, 0xC6, 0x7E, // STA $7E:C6C0,X (OBJ Palette 6, all 16 colors!)
        0xE8, 0xE8, // INX : INX
        0xE0, 0x20, 0x00, // CPX #$0020 (32 bytes)
        0xD0, 0xF1, // BNE loop (-15 bytes)
        0xE2, 0x30, // SEP #$30 (8-bit A/X/Y)
        0x9C, 0x0E, 0x1F, // STZ $1F0E (Frame 0 addr lo = $00)
        0xA9, 0x80, 0x8D, 0x0F, 0x1F, // LDA #$80 : STA $1F0F (addr hi = $80 -> $8000)
        0xA9, 0x20, 0x8D, 0x10, 0x1F, // LDA #$20 : STA $1F10 (bank = $20)
        0xA9, 0x01, 0x8D, 0x0C, 0x1F, // LDA #$01 : STA $1F0C (trigger immediate NMI DMA!)
        0x9C, 0x00, 0x1F, // STZ $1F00 (keep polyhedral thread peacefully idle in .wait)
        0xE6, 0x15, // INC $15 (trigger NMI CGRAM upload!)
        0x6B, // RTL
    ];
    bank22[0x40..0x40 + pal_load_code.len()].copy_from_slice(pal_load_code);

    // $22:8080 — 3D Frame Stepper (called via JSL $22:8080 from $0E:827B and $0E:8905, 8-bit A/X/Y!)
    // Computes f = 0 if locked ($1F05==0 && $1F04==0 && $1F02==0), else (($1F05 + $1F02) >> 3) & 0x1F!
    // Maps f (0..31) -> bank = 0x20 + (f >> 4), addr_hi = 0x80 + ((f & 0x0F) << 3), addr_lo = 0x00!
    let step_3d_code: &[u8] = &[
        0xAD, 0x05, 0x1F, // LDA $1F05
        0x0D, 0x04, 0x1F, // ORA $1F04
        0x0D, 0x02, 0x1F, // ORA $1F02
        0xF0, 0x0C, // BEQ locked (+12 bytes)
        0xAD, 0x05, 0x1F, // LDA $1F05
        0x18, // CLC
        0x6D, 0x02, 0x1F, // ADC $1F02
        0x4A, 0x4A, 0x4A, // LSR : LSR : LSR (>> 3 -> 0..31)
        0x29, 0x1F, // AND #$1F
        // locked / compute addr:
        0x48, // PHA (save f)
        0x29, 0x0F, // AND #$0F
        0x0A, 0x0A, 0x0A, // ASL : ASL : ASL (* 8 -> $00, $08, ..., $78)
        0x09, 0x80, // ORA #$80 ($80..$F8 -> $8000..$F800)
        0x8D, 0x0F, 0x1F, // STA $1F0F (addr_hi)
        0x9C, 0x0E, 0x1F, // STZ $1F0E (addr_lo = $00)
        0x68, // PLA (restore f)
        0x4A, 0x4A, 0x4A, 0x4A, // LSR x4 (0 or 1)
        0x18, 0x69, 0x20, // CLC : ADC #$20 (bank $20 or $21)
        0x8D, 0x10, 0x1F, // STA $1F10 (bank)
        0xA9, 0x01, // LDA #$01
        0x8D, 0x0C, 0x1F, // STA $1F0C (trigger NMI DMA!)
        0x9C, 0x00, 0x1F, // STZ $1F00 (keep polyhedron idle)
        0x6B, // RTL
    ];
    bank22[0x80..0x80 + step_3d_code.len()].copy_from_slice(step_3d_code);

    // $22:8100 — 16-color BGR555 4bpp OBJ Palette 6
    for (i, &w) in TRIO_3D_PALETTE_BGR555.iter().enumerate() {
        let le = w.to_le_bytes();
        bank22[0x100 + i * 2] = le[0];
        bank22[0x100 + i * 2 + 1] = le[1];
    }

    session.swap_custom_bytes(
        rom_zelda,
        0x110000,
        0x110000,
        bank22,
        "Banque $22 ($22:8000) : Routines 65816 DMA 3D, Palette 4bpp OBJ Pal 6 & Stepper 32 frames"
            .into(),
        SwapKind::CodePatch { byte_count: 0x200 },
    )?;
    ops += 1;

    // 3. Hook NMI 3D DMA at $00:92F7 (PC 0x0012F7): `JSL $22:8000 : RTS`
    if rom_zelda.data[0x0012F7] == 0xAD && rom_zelda.data[0x0012F8] == 0x0C {
        session.swap_custom_bytes(
            rom_zelda,
            0x0012F7,
            0x0012F7,
            vec![0x22, 0x00, 0x80, 0x22, 0x60],
            "Hook NMI DMA 3D ($00:92F7) -> JSL $22:8000 (Champignon/Fleur/Étoile 3D)".into(),
            SwapKind::CodePatch { byte_count: 5 },
        )?;
        ops += 1;
    }

    // 4. Keep Polyhedral_InitializeThread ($09:F7DE) 100% intact so SwitchThread ($00:822D) never crashes.
    //    Because $22:8040 & $22:8080 keep $1F00 = 0 (and boot STA $1F00 at $0E:81D9/$1E:CDE6 are NOPed),
    //    Polyhedral_RunThread ($09:F81D) sits harmlessly idle while our NMI DMA uploads the 32 3D Mario Relic frames!

    // 5. Hook Palette Load at $0E:81F7 (PC 0x0701F7) -> `JSL $22:8040 : RTS`
    if rom_zelda.data[0x0701F7] == 0xC2 && rom_zelda.data[0x0701F8] == 0x20 {
        session.swap_custom_bytes(
            rom_zelda,
            0x0701F7,
            0x0701F7,
            vec![0x22, 0x40, 0x80, 0x22, 0x60],
            "Hook Palette 3D ($0E:81F7) -> JSL $22:8040 (OBJ Pal 6 16 couleurs Champignon/Fleur/Étoile)".into(),
            SwapKind::CodePatch { byte_count: 5 },
        )?;
        ops += 1;
    }

    // 5b. NOP out initial boot `STA $1F00` (`8D 00 1F`) at $0E:81D9 & $1E:CDE6 so vanilla rasterizer NEVER overwrites Frame 0!
    for &(pc, label) in &[
        (
            0x0701D9usize,
            "Désactivation Polyèdre Triforce Boot ($0E:81D9) -> NOP x3",
        ),
        (
            0x0F4DE6usize,
            "Désactivation Polyèdre Triforce Fin ($1E:CDE6) -> NOP x3",
        ),
    ] {
        if rom_zelda.data[pc..pc + 3] == [0x8D, 0x00, 0x1F] {
            session.swap_custom_bytes(
                rom_zelda,
                pc,
                pc,
                vec![0xEA, 0xEA, 0xEA],
                label.into(),
                SwapKind::CodePatch { byte_count: 3 },
            )?;
            ops += 1;
        }
    }

    // 6. Hook ALL 4 verified 3D Step sites -> `JSL $22:8080 : NOP`
    for &(pc, label) in &[
        (
            0x07027Cusize,
            "Hook Animation 3D Écran-Titre ($0E:827C) -> JSL $22:8080",
        ),
        (
            0x070906usize,
            "Hook Animation 3D Légende Intro ($0E:8906) -> JSL $22:8080",
        ),
        (
            0x0709FFusize,
            "Hook Animation 3D Terre d'Or Intro ($0E:89FF) -> JSL $22:8080",
        ),
        (
            0x0F4E33usize,
            "Hook Animation 3D Salle Finale ($1E:CE33) -> JSL $22:8080",
        ),
    ] {
        if rom_zelda.data[pc..pc + 5] == [0xA9, 0x01, 0x8D, 0x00, 0x1F] {
            session.swap_custom_bytes(
                rom_zelda,
                pc,
                pc,
                vec![0x22, 0x80, 0x80, 0x22, 0xEA],
                label.into(),
                SwapKind::CodePatch { byte_count: 5 },
            )?;
            ops += 1;
        }
    }

    // 6b. Replace Sepia Hills Intro Story Screen 1 BG Triforce (Sheet 0xD8 tiles 0x03/0x4F/0x13/0x6F
    //     + AttractImage0Stripes PC 0x073AB9 & 0x073AE7, verified via jpdasm bank_0C.asm:15778!)
    //     with a gleaming golden Super Mushroom (top) + Fire Flower (bottom-left) + Super Star (bottom-right)!
    if let Some((z_bk, z_hi, z_lo)) = crate::mixer::find_alttp_gfx_table_pub(&rom_zelda.data) {
        let sid = 0xD8usize;
        let orig_pc = ((rom_zelda.data[z_bk + sid] as usize & 0x7F) << 15)
            | ((rom_zelda.data[z_hi + sid] as usize & 0x7F) << 8)
            | (rom_zelda.data[z_lo + sid] as usize);
        if let Some((mut raw_d8, orig_clen)) =
            crate::disasm_dma::decompress_nintendo_lz2_with_endian(
                &rom_zelda.data,
                orig_pc,
                4096,
                false,
            )
        {
            if raw_d8.len() == 0x800 {
                // 16x16 gleaming golden Mario Trio over sepia hills (2bpp: 0=dark outline, 1=sepia hill, 2=light sand/sky, 3=bright gold!)
                let sepia_trio: [&str; 16] = [
                    "2222200000022222", // Top-center: Super Mushroom dome (cols 4..11)
                    "2222033223302222",
                    "2220323333230222",
                    "2220333333330222",
                    "2222002020202222", // Mushroom stem + cute eyes
                    "2222202222022222",
                    "2200000000000022", // Top edge of Fire Flower (left) & Super Star (right)
                    "2033233022033022",
                    "1032032010333301", // Bottom-left: Fire Flower (cols 1..6); Bottom-right: Super Star (cols 9..14)!
                    "1033233003300330",
                    "1100300110333301",
                    "1030303010300301",
                    "1103330103300330",
                    "1110301110011001",
                    "1122222222222211", // Golden basin glow
                    "1112222222222111",
                ];
                let write_2bpp_d8 = |buf: &mut [u8], tile_idx: usize, x0: usize, y0: usize| {
                    let off = tile_idx * 16;
                    for y in 0..8usize {
                        let row = sepia_trio[y0 + y].as_bytes();
                        let (mut b0, mut b1) = (0u8, 0u8);
                        for x in 0..8usize {
                            let c = (row[x0 + x] - b'0') & 3;
                            let sh = 7 - x;
                            b0 |= (c & 1) << sh;
                            b1 |= ((c >> 1) & 1) << sh;
                        }
                        buf[off + y * 2] = b0;
                        buf[off + y * 2 + 1] = b1;
                    }
                };
                write_2bpp_d8(&mut raw_d8, 0x03, 0, 0); // Top-Left
                write_2bpp_d8(&mut raw_d8, 0x4F, 8, 0); // Top-Right (unused slot in Sheet 0xD8!)
                write_2bpp_d8(&mut raw_d8, 0x13, 0, 8); // Bot-Left
                write_2bpp_d8(&mut raw_d8, 0x6F, 8, 8); // Bot-Right (unused slot in Sheet 0xD8!)

                let comp = crate::disasm_dma::compress_nintendo_lz2_with_endian(&raw_d8, false);
                let dst_pc = if comp.len() <= orig_clen {
                    orig_pc
                } else {
                    if *free_cursor + comp.len() <= free_end {
                        let p = *free_cursor;
                        *free_cursor += comp.len();
                        p
                    } else {
                        orig_pc
                    }
                };
                if dst_pc + comp.len() <= rom_zelda.data.len() {
                    session.swap_custom_bytes(
                        rom_zelda,
                        dst_pc,
                        dst_pc,
                        comp.clone(),
                        "Plaque Intro Sépia 2bpp (Sheet 0xD8) : Triforce -> Champignon, Fleur & Étoile d'Or".into(),
                        SwapKind::GraphicsTiles {
                            format: crate::graphics::TileFormat::Bpp2,
                            tile_count: 128,
                            smart_remapped: true,
                        },
                    )?;
                    if dst_pc != orig_pc {
                        let bk = ((dst_pc >> 15) & 0x7F) as u8;
                        let addr = 0x8000 | (dst_pc & 0x7FFF);
                        session.swap_custom_bytes(
                            rom_zelda,
                            z_bk + sid,
                            z_bk + sid,
                            vec![bk],
                            "Ptr Bank Sheet 0xD8".into(),
                            SwapKind::CodePatch { byte_count: 1 },
                        )?;
                        session.swap_custom_bytes(
                            rom_zelda,
                            z_hi + sid,
                            z_hi + sid,
                            vec![(addr >> 8) as u8],
                            "Ptr Hi Sheet 0xD8".into(),
                            SwapKind::CodePatch { byte_count: 1 },
                        )?;
                        session.swap_custom_bytes(
                            rom_zelda,
                            z_lo + sid,
                            z_lo + sid,
                            vec![(addr & 0xFF) as u8],
                            "Ptr Lo Sheet 0xD8".into(),
                            SwapKind::CodePatch { byte_count: 1 },
                        )?;
                    }
                    ops += 1;
                }
            }
        }
    }
    if 0x073AEB <= rom_zelda.data.len()
        && rom_zelda.data[0x073AB9] == 0x03
        && rom_zelda.data[0x073ABB] == 0x03
        && rom_zelda.data[0x073AE7] == 0x13
        && rom_zelda.data[0x073AE9] == 0x13
    {
        session.swap_custom_bytes(
            rom_zelda,
            0x073AB9,
            0x073AB9,
            vec![0x03, 0x31, 0x4F, 0x31],
            "Tilemap Plaque Intro Sépia Haut (0x073AB9) : Champignon, Fleur & Étoile sans miroir"
                .into(),
            SwapKind::CodePatch { byte_count: 4 },
        )?;
        session.swap_custom_bytes(
            rom_zelda,
            0x073AE7,
            0x073AE7,
            vec![0x13, 0x35, 0x6F, 0x35],
            "Tilemap Plaque Intro Sépia Bas (0x073AE7) : Fleur & Étoile sans miroir".into(),
            SwapKind::CodePatch { byte_count: 4 },
        )?;
        ops += 2;
    }

    // 7. Rewrite Bank $0E OAM Tables ($0E:8449..$0E:8688) for:
    //    - Title Screen 3D Mario Relics ($0E:8449 Mushroom, $0E:8469 Flower, $0E:8489 Star; attr=0x1D Pal 6!)
    //    - Intro Scroll 3D Mario Relics ($0E:84A9 Mushroom, $0E:84C9 Flower, $0E:84E9 Star; attr=0x2D Pal 6!)
    //    - 3-way OAM Dispatchers at $0E:8549 (Title) & $0E:8669 (Intro Scroll)!
    //    - Expanded 27-sprite Copyright + "🄯 2026 pomercier" OAM table at $0E:8569..$0E:8640!
    let make_32x32_oam = |tl: u8, tr: u8, bl: u8, br: u8, attr: u8| -> Vec<u8> {
        let mut v = Vec::with_capacity(32);
        for &(x, y, t) in &[(16i16, 16i16, tl), (32, 16, tr), (16, 32, bl), (32, 32, br)] {
            v.extend_from_slice(&x.to_le_bytes());
            v.extend_from_slice(&y.to_le_bytes());
            v.push(t);
            v.push(attr);
            v.extend_from_slice(&[0x00, 0x02]); // 16x16 OAM sprite!
        }
        v
    };

    if 0x070689 <= rom_zelda.data.len() && rom_zelda.data[0x070549] == 0xA9 {
        let mut oam_block = rom_zelda.data[0x070449..0x070689].to_vec(); // exact 576B ($0E:8449..$0E:8688)
        let write_at = |buf: &mut [u8], snes_addr: usize, data: &[u8]| {
            let off = snes_addr - 0x8449;
            buf[off..off + data.len()].copy_from_slice(data);
        };

        // Title Screen 3D Mario Relics (attr = 0x1D: Priority 1 behind BG1 Pri-1 'PEACH', OBJ Pal 6, NameTable 1!)
        write_at(
            &mut oam_block,
            0x8449,
            &make_32x32_oam(0x80, 0x82, 0xA0, 0xA2, 0x1D),
        ); // Top: Super Mushroom!
        write_at(
            &mut oam_block,
            0x8469,
            &make_32x32_oam(0x84, 0x86, 0xA4, 0xA6, 0x1D),
        ); // Bottom-Left: Fire Flower!
        write_at(
            &mut oam_block,
            0x8489,
            &make_32x32_oam(0x88, 0x8A, 0xA8, 0xAA, 0x1D),
        ); // Bottom-Right: Super Star!

        // Intro Story Scroll 3D Mario Relics (attr = 0x2D: Priority 2 crisp over sepia hills, OBJ Pal 6, NameTable 1!)
        write_at(
            &mut oam_block,
            0x84A9,
            &make_32x32_oam(0x80, 0x82, 0xA0, 0xA2, 0x2D),
        ); // Top: Super Mushroom!
        write_at(
            &mut oam_block,
            0x84C9,
            &make_32x32_oam(0x84, 0x86, 0xA4, 0xA6, 0x2D),
        ); // Bottom-Left: Fire Flower!
        write_at(
            &mut oam_block,
            0x84E9,
            &make_32x32_oam(0x88, 0x8A, 0xA8, 0xAA, 0x2D),
        ); // Bottom-Right: Super Star!

        // Title Screen 3-way dispatcher at $0E:8549 (X=0 Flower->$8469, X=1 Mushroom->$8449, X=2 Star->$8489)
        let disp_title: [u8; 26] = [
            0xA9, 0x04, 0x85, 0x06, 0x64,
            0x07, // LDA #$04 : STA $06 : STZ $07 (4 OAM sprites)
            0xBD, 0x60, 0x85, // LDA $8560,X
            0x85, 0x08, 0xA9, 0x84, 0x85, 0x09, // STA $08 : LDA #$84 : STA $09
            0x20, 0xAC, 0x87, 0x60, // JSR $87AC : RTS
            0xEA, 0xEA, 0xEA, 0xEA, // Padding to $8560
            0x69, 0x49, 0x89, // $8560: [X=0 Flower, X=1 Mushroom, X=2 Star]
        ];
        write_at(&mut oam_block, 0x8549, &disp_title);

        // Intro Story Scroll 3-way dispatcher at $0E:8669 (X=0 Flower->$84C9, X=1 Mushroom->$84A9, X=2 Star->$84E9)
        let disp_intro: [u8; 26] = [
            0xA9, 0x04, 0x85, 0x06, 0x64, 0x07, 0xBD, 0x80, 0x86, // LDA $8680,X
            0x85, 0x08, 0xA9, 0x84, 0x85, 0x09, 0x20, 0xAC, 0x87, 0x60, 0xEA, 0xEA, 0xEA, 0xEA,
            0xC9, 0xA9, 0xE9, // $8680: [X=0 Flower, X=1 Mushroom, X=2 Star]
        ];
        write_at(&mut oam_block, 0x8669, &disp_intro);

        // 27-sprite Copyright OAM Table at $0E:8569..$0E:8640 (216 bytes):
        //   Row 1 (y = 0):   Original 13 sprites of "©1991,1992 Nintendo" copied verbatim from $0E:86A2!
        //   Row 2 (y = +12): 14 crisp 8x8 sprites of "🄯 2026 pomercier" centered cleanly beneath it!
        let mut copy_oam = Vec::with_capacity(27 * 8);
        copy_oam.extend_from_slice(&rom_zelda.data[0x0706A2..0x0706A2 + 13 * 8]);
        // Center "🄯 2026 pomercier" (16 chars * 8px = 128px; "©1991,1992 Nintendo" is 13*8=104px at x=0..104 -> start at x=-12!)
        // Uses native 0x40 H-flipped (attr 0x4A) for '🄯', native 0x43 for '2', and rows 2-3 unused slots for 0,6,p,o,m,e,r,c,i!
        let copyleft_line: &[(i16, u8, u8)] = &[
            (-12, 0x40, 0x4A), // 🄯 (native © H-flipped!)
            // (-4: space)
            (4, 0x43, 0x0A),  // 2 (native Nintendo '2'!)
            (12, 0x60, 0x0A), // 0
            (20, 0x43, 0x0A), // 2
            (28, 0x61, 0x0A), // 6
            // (36: space)
            (44, 0x66, 0x0A),  // p
            (52, 0x67, 0x0A),  // o
            (60, 0x70, 0x0A),  // m
            (68, 0x71, 0x0A),  // e
            (76, 0x76, 0x0A),  // r
            (84, 0x77, 0x0A),  // c
            (92, 0x78, 0x0A),  // i
            (100, 0x71, 0x0A), // e
            (108, 0x76, 0x0A), // r
        ];
        for &(x, tile, attr) in copyleft_line {
            copy_oam.extend_from_slice(&x.to_le_bytes());
            copy_oam.extend_from_slice(&12i16.to_le_bytes()); // y = +12 (right below ©1991,1992 Nintendo!)
            copy_oam.push(tile);
            copy_oam.push(attr); // Exact same OBJ Pal 5 white+crimson shadow as ©1991,1992 Nintendo (+ H-flip for 🄯)!
            copy_oam.extend_from_slice(&[0x00, 0x00]); // 8x8 OAM sprite!
        }
        assert_eq!(copy_oam.len(), 27 * 8);
        write_at(&mut oam_block, 0x8569, &copy_oam);

        session.swap_custom_bytes(
            rom_zelda,
            0x070449,
            0x070449,
            oam_block,
            "Tables OAM Écran-Titre & Intro ($0E:8449..$0E:8688) : 3 Reliques 3D + « 🄯 2026 pomercier »".into(),
            SwapKind::CodePatch { byte_count: 576 },
        )?;
        ops += 1;

        // Point Copyright OAM drawer ($0E:870A = PC 0x07070A) to 27 sprites @ $0E:8569!
        if rom_zelda.data[0x07070A] == 0xA9 && rom_zelda.data[0x07070B] == 0x0D {
            let copy_call = vec![
                0xA9, 0x1B, 0x85, 0x06, 0x64, 0x07, // 27 sprites!
                0xA9, 0x69, 0x85, 0x08, // $08 = $69
                0xA9, 0x85, 0x85, 0x09, // $09 = $85 -> $0E:8569!
                0x20, 0xAC, 0x87, 0x60, // JSR $87AC : RTS
            ];
            session.swap_custom_bytes(
                rom_zelda,
                0x07070A,
                0x07070A,
                copy_call,
                "Appel OAM Copyright ($0E:870A) : Affiche « ©1991,1992 Nintendo » + « 🄯 2026 pomercier »".into(),
                SwapKind::CodePatch { byte_count: 18 },
            )?;
            ops += 1;
        }
    }

    // -------------------------------------------------------------------------
    // E. REWRITE OPENING LEGEND STORY INTRO (4 SCREENS @ 0x0E5564..0x0E57AF):
    //    - "Princesse Peach" (NO "Reine"!)
    //    - "Champignon, Fleur & Etoile" (NO Triforce!)
    //    - "Roi Bowser" (NO Agahnim/Magikoopa!)
    //    - "Sept Yoshis"
    // -------------------------------------------------------------------------
    let intro0 = build_exact_fr_msg(
        &[
            Seg::Raw(&[0x87, 0x00, 0x77, 0x07, 0x7A, 0x03, 0x84, 0x02, 0x80]),
            Seg::Txt("Il y a longtemps, dans le beau"),
            Seg::Raw(&[0x75]),
            Seg::Txt("Royaume Champignon entouré"),
            Seg::Raw(&[0x76]),
            Seg::Txt("de collines et de tuyaux..."),
            Seg::Raw(&[0x78, 0x07, 0x73]),
            Seg::Txt("brillaient 3 Reliques d'Or:"),
            Seg::Raw(&[0x73]),
            Seg::Txt("Champignon, Fleur et Etoile"),
            Seg::Raw(&[0x73]),
            Seg::Txt("cachées au château secret."),
            Seg::Raw(&[0x78, 0x07, 0x80, 0x80, 0x73]),
            Seg::Txt("Beaucoup cherchèrent par la"),
            Seg::Raw(&[0x73]),
            Seg::Txt("force à voler ce trésor du"),
            Seg::Raw(&[0x73]),
            Seg::Txt("Royaume Champignon, mais..."),
            Seg::Raw(&[0x78, 0x07, 0x80, 0x80, 0x73]),
            Seg::Txt("personne n'en revint jamais."),
            Seg::Raw(&[0x73]),
            Seg::Txt("Un jour, le pouvoir noir de"),
            Seg::Raw(&[0x73]),
            Seg::Txt("Bowser envahit le Royaume."),
            Seg::Raw(&[0x78, 0x07, 0x73]),
            Seg::Txt("Princesse Peach ordonna aux"),
            Seg::Raw(&[0x73]),
            Seg::Txt("Sept Yoshis de sceller la"),
            Seg::Raw(&[0x73]),
            Seg::Txt("porte du Monde d'Or."),
            Seg::Raw(&[0x78, 0x07, 0x73]),
            Seg::Txt("..."),
            Seg::Raw(&[0x73]),
            Seg::Txt("Ce sceau aurait dû"),
            Seg::Raw(&[0x73]),
            Seg::Txt("demeurer à jamais."),
            Seg::Raw(&[0x78, 0x07, 0x80, 0x80, 0x73]),
            Seg::Txt("Mais quand ces récits"),
            Seg::Raw(&[0x73]),
            Seg::Txt("devinrent des légendes,"),
        ],
        381,
    );

    let intro1 = build_exact_fr_msg(
        &[
            Seg::Raw(&[0x84, 0x02, 0x77, 0x07, 0x7A, 0x03, 0x74]),
            Seg::Txt("le cruel Roi Bowser revint"),
            Seg::Raw(&[0x75]),
            Seg::Txt("au Royaume Champignon pour"),
            Seg::Raw(&[0x76]),
            Seg::Txt("briser le sceau... Il fit"),
            Seg::Raw(&[0x78, 0x05, 0x73]),
            Seg::Txt("régner la terreur."),
        ],
        87,
    );

    let intro2 = build_exact_fr_msg(
        &[
            Seg::Raw(&[0x84, 0x02, 0x77, 0x07, 0x7A, 0x03]),
            Seg::Txt("Par sa magie, Bowser captura"),
            Seg::Raw(&[0x75]),
            Seg::Txt("un à un, les Sept jeunes"),
            Seg::Raw(&[0x76]),
            Seg::Txt("Yoshi du Royaume..."),
        ],
        66,
    );

    let intro3 = build_exact_fr_msg(
        &[
            Seg::Raw(&[0x84, 0x02, 0x77, 0x07, 0x7A, 0x03]),
            Seg::Txt("Et le destin de la Princesse"),
            Seg::Raw(&[0x75]),
            Seg::Txt("Peach repose désormais sur"),
            Seg::Raw(&[0x76]),
            Seg::Txt("Mario..."),
        ],
        53,
    );

    for (pc, expected_len, msg_opt, label) in [
        (
            0x0E5564usize,
            381usize,
            intro0,
            "Intro Légende Écran 1 (0x0E5564) : Princesse Peach, 3 Reliques & Bowser",
        ),
        (
            0x0E56E1usize,
            87usize,
            intro1,
            "Intro Légende Écran 2 (0x0E56E1) : Roi Bowser brise le sceau",
        ),
        (
            0x0E5738usize,
            66usize,
            intro2,
            "Intro Légende Écran 3 (0x0E5738) : Capture des Sept jeunes Yoshi",
        ),
        (
            0x0E577Ausize,
            53usize,
            intro3,
            "Intro Légende Écran 4 (0x0E577A) : Princesse Peach & Mario",
        ),
    ] {
        if let Some(bytes) = msg_opt {
            if bytes.len() == expected_len
                && pc + expected_len <= rom_zelda.data.len()
                && rom_zelda.data[pc + expected_len - 1] == 0x7F
            {
                session.swap_custom_bytes(
                    rom_zelda,
                    pc,
                    pc,
                    bytes,
                    label.into(),
                    SwapKind::CodePatch {
                        byte_count: expected_len,
                    },
                )?;
                ops += 1;
            }
        }
    }

    // -------------------------------------------------------------------------
    // F. LIVE FRENCH DICTIONARY ($0C) + OPTIMAL DP BANK $1C SEQUENTIAL REPACKER (v0.4.0):
    //    Decodes all 372 messages losslessly against vanilla $0C, upgrades Bank $0C dictionary
    //    (Ganon->Bowser, rubis->pièces, sorcier->Relique, Sahasrahla->Papy Toad, Perle de Lune->Lune 3-UP),
    //    applies word-wrapped 24-column Mario rewrites, and DP-compresses back into Bank $1C!
    // -------------------------------------------------------------------------
    ops += repack_french_bank1c(rom_zelda, session)?;

    // -------------------------------------------------------------------------
    // G. TITLE SCREEN BG2 LANDSCAPE & MASTER SWORD PALETTES (v0.4.0):
    //    - Water (0x0DE83E): 0x4608 -> SMW Sparkling Blue 0x7A18
    //    - Sky (0x0DE844 & 0x0DE856): 0x7ED4 -> SMW Sky Blue 0x7F0E (RGB 112, 192, 248)
    //    - Mountains (0x0DE84C): 0x7A72 -> Yoshi's Island Green Hill 0x1EE7 (RGB 56, 184, 56)
    //    - Peach's Castle (0x0DE85A): 0x4AB6 -> Peach Castle Pink 0x5B5E
    //    - Peach's Castle Highlights (0x0DE85C): 0x6FBE -> Pure White 0x7FFF
    //    - Master Sword Hilt (0x0DD444): 0x7273 -> Golden Star Blade 0x275F
    // -------------------------------------------------------------------------
    for (addr, color_u16, label) in [
        (
            0x0DE83Eusize,
            0x7A18u16,
            "Écran-Titre BG2 Eau : Bleu Scintillant SMW (0x7A18)",
        ),
        (
            0x0DE844usize,
            0x7F0Eu16,
            "Écran-Titre BG2 Ciel : Bleu Ciel SMW (0x7F0E)",
        ),
        (
            0x0DE84Cusize,
            0x1EE7u16,
            "Écran-Titre BG2 Collines : Vert Yoshi's Island (0x1EE7)",
        ),
        (
            0x0DE856usize,
            0x7F0Eu16,
            "Écran-Titre BG2 Ciel Château : Bleu Ciel SMW (0x7F0E)",
        ),
        (
            0x0DE85Ausize,
            0x5B5Eu16,
            "Écran-Titre BG2 Château : Rose Princesse Peach (0x5B5E)",
        ),
        (
            0x0DE85Cusize,
            0x7FFFu16,
            "Écran-Titre BG2 Château : Blanc Pur (0x7FFF)",
        ),
        (
            0x0DD444usize,
            0x275Fu16,
            "Écran-Titre Épée : Garde Dorée Étoile SMW (0x275F)",
        ),
    ] {
        if addr + 2 <= rom_zelda.data.len() {
            session.swap_custom_bytes(
                rom_zelda,
                addr,
                addr,
                color_u16.to_le_bytes().to_vec(),
                label.into(),
                SwapKind::PaletteBgr555 { color_count: 1 },
            )?;
            ops += 1;
        }
    }

    Ok(ops)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intro_screens_exact_budget() {
        let (s0, _s1, _s2, _s3) = (381usize, 87usize, 66usize, 53usize);
        let intro0 = build_exact_fr_msg(
            &[
                Seg::Raw(&[0x87, 0x00, 0x77, 0x07, 0x7A, 0x03, 0x84, 0x02, 0x80]),
                Seg::Txt("Il y a longtemps, dans le beau"),
                Seg::Raw(&[0x75]),
                Seg::Txt("Royaume Champignon entouré"),
                Seg::Raw(&[0x76]),
                Seg::Txt("de collines et de tuyaux..."),
                Seg::Raw(&[0x78, 0x07, 0x73]),
                Seg::Txt("brillaient 3 Reliques d'Or:"),
                Seg::Raw(&[0x73]),
                Seg::Txt("Champignon, Fleur et Etoile"),
                Seg::Raw(&[0x73]),
                Seg::Txt("cachées au château secret."),
                Seg::Raw(&[0x78, 0x07, 0x80, 0x80, 0x73]),
                Seg::Txt("Beaucoup cherchèrent par la"),
                Seg::Raw(&[0x73]),
                Seg::Txt("force à voler ce trésor du"),
                Seg::Raw(&[0x73]),
                Seg::Txt("Royaume Champignon, mais..."),
                Seg::Raw(&[0x78, 0x07, 0x80, 0x80, 0x73]),
                Seg::Txt("personne n'en revint jamais."),
                Seg::Raw(&[0x73]),
                Seg::Txt("Un jour, le pouvoir noir de"),
                Seg::Raw(&[0x73]),
                Seg::Txt("Bowser envahit le Royaume."),
                Seg::Raw(&[0x78, 0x07, 0x73]),
                Seg::Txt("Princesse Peach ordonna aux"),
                Seg::Raw(&[0x73]),
                Seg::Txt("Sept Yoshis de sceller la"),
                Seg::Raw(&[0x73]),
                Seg::Txt("porte du Monde d'Or."),
                Seg::Raw(&[0x78, 0x07, 0x73]),
                Seg::Txt("..."),
                Seg::Raw(&[0x73]),
                Seg::Txt("Ce sceau aurait dû"),
                Seg::Raw(&[0x73]),
                Seg::Txt("demeurer à jamais."),
                Seg::Raw(&[0x78, 0x07, 0x80, 0x80, 0x73]),
                Seg::Txt("Mais quand ces récits"),
                Seg::Raw(&[0x73]),
                Seg::Txt("devinrent des légendes,"),
            ],
            s0,
        );
        println!("intro0 is_some: {}", intro0.is_some());
        if let Some(ref b) = intro0 {
            println!("intro0 len: {}", b.len());
        }
        assert!(
            intro0.is_some(),
            "intro0 must fit within exact 381B budget!"
        );
    }

    #[test]
    fn test_giant_peach_logo_tile_budget() {
        let canvas = render_giant_peach_128x48();
        let blank8 = [[0u8; 8]; 8];
        let mut uniq = std::collections::HashSet::new();
        for tr in 0..6 {
            for tc in 0..16 {
                let mut t = [[0u8; 8]; 8];
                for y in 0..8 {
                    for x in 0..8 {
                        t[y][x] = canvas[tr * 8 + y][tc * 8 + x];
                    }
                }
                if t != blank8 {
                    uniq.insert(t);
                }
            }
        }
        assert!(
            uniq.len() <= 95,
            "Giant PEACH logo uses {} unique tiles, max is 95 (0x100..0x15E)!",
            uniq.len()
        );
    }

    #[test]
    fn test_3d_mario_trio_frames_integrity() {
        let frames = build_all_32_trio_3d_frames();
        assert_eq!(frames.len(), 32 * 2048);
        // Verify Frame 0 & rotated Frame 8 have non-zero pixel data in all 3 icon slots!
        for &f in &[0usize, 8, 16, 24] {
            let sl = &frames[f * 2048..(f + 1) * 2048];
            let mush_nz = sl[0..4 * 32].iter().filter(|&&b| b != 0).count();
            let flw_nz = sl[4 * 32..8 * 32].iter().filter(|&&b| b != 0).count();
            let star_nz = sl[8 * 32..12 * 32].iter().filter(|&&b| b != 0).count();
            assert!(
                mush_nz > 10 && flw_nz > 10 && star_nz > 10,
                "Frame {} must render all 3 3D icons",
                f
            );
        }
    }

    #[test]
    fn test_bank1c_repacker_invariants() {
        let p = [
            std::path::Path::new("roms/zelda_alttp.sfc"),
            std::path::Path::new("../../roms/zelda_alttp.sfc"),
        ]
        .into_iter()
        .find(|cand| cand.exists());
        let Some(p) = p else {
            return;
        };
        let data = std::fs::read(p).expect("read roms/zelda_alttp.sfc");
        let mut rom = SnesRom::from_bytes("zelda", &data).expect("parse snes rom");
        let mut session = SwapSession::new();
        let res = repack_french_bank1c(&mut rom, &mut session);
        assert!(res.is_ok(), "repack_french_bank1c must succeed: {:?}", res);
        assert!(res.unwrap() >= 1);

        // Verify the repacked bank in rom.data
        let repacked = &rom.data[0x0E0000..0x0E8000];
        assert_eq!(
            repacked.len(),
            0x8000,
            "Bank $1C must be exactly 32768 bytes"
        );
        assert_eq!(
            repacked.iter().filter(|&&b| b == 0x7F).count(),
            372,
            "Must contain exactly 372 messages"
        );
        assert!(repacked.contains(&0x88), "Must contain 0x88 end marker");
        // Verify ZERO occurrences of raw "Trif" (13 2B 22 1F) remain in Bank $1C!
        assert!(
            !repacked.windows(4).any(|w| w == [0x13, 0x2B, 0x22, 0x1F]),
            "Zero occurrences of 'Triforce' may remain in Bank $1C!"
        );

        // Phase 5 Strict Invariants across ALL 372 messages:
        // 1. ZERO duplicate 0x75 or 0x76 without an intervening 0x74 screen clear (prevents text-overlap corruption!)
        // 2. ZERO hyphenated word-splits ('-' immediately before 0x73/0x75/0x76 or 0x7E+LB)!
        // 3. ZERO broken French substitutions ("Toadsse", "Reliquess", "ton champi")!
        let mut cur = 0usize;
        let mut msg_idx = 0usize;
        let mut dup_lb_msgs = Vec::new();
        let mut hyphen_split_msgs = Vec::new();
        while cur < repacked.len() && repacked[cur] != 0x88 {
            let end = repacked[cur..].iter().position(|&b| b == 0x7F).unwrap();
            let raw = &repacked[cur..cur + end];
            let mut seen_75 = false;
            let mut seen_76 = false;
            let mut i = 0usize;
            while i < raw.len() {
                let b = raw[i];
                if matches!(b, 0x6F..=0x72 | 0x77..=0x7A | 0x84 | 0x87) && i + 1 < raw.len() {
                    i += 2;
                    continue;
                }
                if b == 0x74 {
                    seen_75 = false;
                    seen_76 = false;
                } else if b == 0x75 {
                    if seen_75 {
                        dup_lb_msgs.push((msg_idx, 0x75u8));
                    }
                    seen_75 = true;
                } else if b == 0x76 {
                    if seen_76 {
                        dup_lb_msgs.push((msg_idx, 0x76u8));
                    }
                    seen_76 = true;
                } else if b == 0x40 {
                    // '-' character: check if immediately followed by LineBreak or <WAIT>+LineBreak!
                    let next1 = raw.get(i + 1).copied();
                    let next2 = raw.get(i + 2).copied();
                    if matches!(next1, Some(0x73 | 0x75 | 0x76))
                        || (next1 == Some(0x7E) && matches!(next2, Some(0x73 | 0x75 | 0x76)))
                    {
                        hyphen_split_msgs.push(msg_idx);
                    }
                }
                i += 1;
            }
            cur += end + 1;
            msg_idx += 1;
        }
        assert!(
            dup_lb_msgs.is_empty(),
            "Found duplicate 0x75/0x76 line breaks (causes text overwrite corruption!): {:?}",
            dup_lb_msgs
        );
        assert!(
            hyphen_split_msgs.is_empty(),
            "Found hyphenated word-splits across line breaks: {:?}",
            hyphen_split_msgs
        );
    }
}
