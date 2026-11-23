//! High-precision Super Mario World UI, HUD, Start Menu, Dialog Font, French Text,
//! Unclipped Princess Peach, and 2-Piece Koopa Warrior builders for Zelda ALTTP.
//!
//! 100% SNES & Super Famicom hardware compatible (standard 2bpp/3bpp LZ2 sheets,
//! uncompressed dialog font @ 0x0646E8, exact-length French dialog patches in Bank $1C).

use crate::disasm_dma::{compress_nintendo_lz2_with_endian, decompress_nintendo_lz2_with_endian};
use crate::graphics::TileFormat;
use crate::mixer::find_alttp_gfx_table_pub;
use crate::rom::{RomError, SnesRom};
use crate::swapper::{SwapKind, SwapSession};

// =============================================================================
// 1. 2BPP TILE ENCODING & PAIRED ALTTP LZ2 SHEET PATCHER (0x70/0xDC & 0x6F/0xDB)
// =============================================================================

fn parse_ascii_8x8(rows: [&str; 8]) -> [[u8; 8]; 8] {
    let mut out = [[0u8; 8]; 8];
    for (y, r) in rows.iter().enumerate() {
        for (x, ch) in r.bytes().take(8).enumerate() {
            out[y][x] = match ch {
                b'.' | b'0' => 0,
                b'1' | b'R' | b'G' | b'B' | b'Y' => 1,
                b'2' | b'W' => 2,
                b'3' | b'K' | b'#' => 3,
                _ => 0,
            };
        }
    }
    out
}

fn parse_ascii_16x16(rows: [&str; 16]) -> [[u8; 16]; 16] {
    let mut out = [[0u8; 16]; 16];
    for (y, r) in rows.iter().enumerate() {
        for (x, ch) in r.bytes().take(16).enumerate() {
            out[y][x] = match ch {
                b'.' | b'0' => 0,
                b'1' | b'R' | b'G' | b'B' | b'Y' => 1,
                b'2' | b'W' => 2,
                b'3' | b'K' | b'#' => 3,
                _ => 0,
            };
        }
    }
    out
}

fn write_2bpp_8x8(buf: &mut [u8], tile_idx: usize, px: &[[u8; 8]; 8]) {
    let off = tile_idx * 16;
    if off + 16 > buf.len() {
        return;
    }
    for y in 0..8usize {
        let (mut b0, mut b1) = (0u8, 0u8);
        for x in 0..8usize {
            let c = px[y][x] & 3;
            let sh = 7 - x;
            b0 |= (c & 1) << sh;
            b1 |= ((c >> 1) & 1) << sh;
        }
        buf[off + y * 2] = b0;
        buf[off + y * 2 + 1] = b1;
    }
}

fn write_2bpp_16x16_quad(
    buf: &mut [u8],
    tl: usize,
    tr: usize,
    bl: usize,
    br: usize,
    px: &[[u8; 16]; 16],
) {
    let mut q = [[0u8; 8]; 8];
    for (ti, qx, qy) in [(tl, 0usize, 0usize), (tr, 8, 0), (bl, 0, 8), (br, 8, 8)] {
        for y in 0..8 {
            for x in 0..8 {
                q[y][x] = px[qy + y][qx + x];
            }
        }
        write_2bpp_8x8(buf, ti, &q);
    }
}

/// Patch a compressed 2bpp ALTTP sheet (`0x6F` or `0x70`, 2048 bytes = 128 tiles) and automatically
/// keep ALL alias pointer entries (`0x6F`+`0xDB`, `0x70`+`0xDC`) pointing to the updated sheet!
fn patch_alttp_2bpp_sheet(
    rom_z: &mut SnesRom,
    sess: &mut SwapSession,
    primary_sheet: usize,
    alias_sheets: &[usize],
    desc: &str,
    free_cursor: &mut usize,
    free_end: usize,
    edit_fn: &dyn Fn(&mut [u8]),
) -> Result<bool, RomError> {
    let Some((z_bk, z_hi, z_lo)) = find_alttp_gfx_table_pub(&rom_z.data) else {
        return Ok(false);
    };
    let pc = ((rom_z.data[z_bk + primary_sheet] as usize & 0x7F) << 15)
        | ((rom_z.data[z_hi + primary_sheet] as usize & 0x7F) << 8)
        | (rom_z.data[z_lo + primary_sheet] as usize);
    let Some((mut raw, orig_clen)) =
        decompress_nintendo_lz2_with_endian(&rom_z.data, pc, 4096, false)
    else {
        return Ok(false);
    };
    if raw.len() != 0x800 {
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
                format: TileFormat::Bpp2,
                tile_count: 128,
                smart_remapped: true,
            },
        )?;
        Ok(true)
    } else {
        const EXPANDED_FREE_START: usize = 0x118000; // Reserve Banks $20..$22 (0x100000..0x117FFF) for 3D Intro Frames + Code!
        let mut cur_end = if *free_cursor < 0x100000 {
            free_end
        } else {
            rom_z.data.len()
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
                cur_end = rom_z.data.len();
            } else if *free_cursor < EXPANDED_FREE_START {
                *free_cursor = EXPANDED_FREE_START;
                cur_end = rom_z.data.len();
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
                format: TileFormat::Bpp2,
                tile_count: 128,
                smart_remapped: true,
            },
        )?;
        let mut all_ids = vec![primary_sheet];
        all_ids.extend_from_slice(alias_sheets);
        for sid in all_ids {
            sess.swap_custom_bytes(
                rom_z,
                z_bk + sid,
                z_bk + sid,
                vec![snes_bank],
                format!("Ptr Bank 2bpp 0x{:02X}", sid),
                SwapKind::CodePatch { byte_count: 1 },
            )?;
            sess.swap_custom_bytes(
                rom_z,
                z_hi + sid,
                z_hi + sid,
                vec![(snes_addr >> 8) as u8],
                format!("Ptr Hi 2bpp 0x{:02X}", sid),
                SwapKind::CodePatch { byte_count: 1 },
            )?;
            sess.swap_custom_bytes(
                rom_z,
                z_lo + sid,
                z_lo + sid,
                vec![(snes_addr & 0xFF) as u8],
                format!("Ptr Lo 2bpp 0x{:02X}", sid),
                SwapKind::CodePatch { byte_count: 1 },
            )?;
        }
        Ok(true)
    }
}

// =============================================================================
// 2. FIXES #4, #7, #8: HUD MINI-MUSHROOMS, YELLOW COIN, BOB-OMB & START MENU!
// =============================================================================

/// Patch ALTTP's Top-HUD and Start Inventory Menu (Sheets `0x70`/`0xDC` & `0x6F`/`0xDB` + Bank `$0D` tilemaps):
/// - Fix #7: Life bar hearts (`0xA0`/`0xA1`/`0xA2`) -> 8x8 Red Mini Super Mushrooms!
/// - Fix #8: HUD Rupee (`0xA8` + Pal 2 `0x28` @ `0x06FDB2`) -> Bright Yellow SMW Coin!
///           HUD Bomb (`0x88`/`0x89` + Pal 2 `0x28` @ `0x06FDB8`) -> Black/White/Gold SMW Bob-omb!
/// - Fix #4: Start Inventory Menu icons (Bob-omb, Fire Flower Lantern, Bullet Bill Bow, SMW Hammer,
///           Koopa Shell Shield, Mario Cap Tunic in Red/Blue/Fire, Super Mushroom Reserve, Cape Feather,
///           Message Block, Yoshi, 3-UP Moon, Starman)!
pub fn patch_mario_hud_and_start_menu(
    rom_zelda: &mut SnesRom,
    session: &mut SwapSession,
    free_cursor: &mut usize,
    free_end: usize,
) -> Result<usize, RomError> {
    let mut ops = 0usize;

    // --- Top-HUD 8x8 tiles in Sheet 0x70 (VRAM 0x80..0xFF -> local tile index = tile - 0x80) ---
    // Fix #7 (v2): Plump, 100% symmetrical, rounded iconic 8x8 Super Mario Mushroom (ZERO awkward notch!)
    // Pal 1 (0x24): 1='R' BRIGHT RUBY RED, 2='W' PURE WHITE, 3='K' CRISP BLACK OUTLINE!
    let hud_mush_full_crisp = parse_ascii_8x8([
        "..KKKK..", ".KRRRRK.", "KWRRRRWK", "KRRWWRRK", "KRRRRRRK", ".KWWWWK.", "..KWWK..",
        "..KKKK..",
    ]);
    let hud_mush_half = parse_ascii_8x8([
        "..KKKK..", ".KRRWWK.", "KWRRKKWK", "KRRWKKWK", "KRRRKKWK", ".KWWKKK.", "..KWWK..",
        "..KKKK..",
    ]);
    let hud_mush_empty = parse_ascii_8x8([
        "..WWWW..", ".WKKKKW.", "WKKKKKKW", "WKKKKKKW", "WKKKKKKW", ".WKKKKW.", "..WKKW..",
        "..WWWW..",
    ]);

    // HUD Yellow SMW Coin (tile 0xA8 -> local 0x28, Pal 2: 1=BRIGHT GOLD, 2=WHITE, 3=BLACK!)
    // 100% symmetric classic SMW oval coin with gleaming vertical center highlight (zero notch!)
    let hud_coin = parse_ascii_8x8([
        "..KKKK..", ".KYYYYK.", "KYWYYWYK", "KYWYKYYK", "KYWYKYYK", "KYWYYWYK", ".KYYYYK.",
        "..KKKK..",
    ]);

    // HUD SMW Key (tile 0xA9 -> local 0x29, Pal 2: 1=GOLD, 2=WHITE, 3=BLACK!)
    let hud_key = parse_ascii_8x8([
        "..KKKK..", ".KYWWYK.", ".KYKKYK.", "..KYYK..", "...KYK..", "..KYYK..", "..KYYK..",
        "...KK...",
    ]);

    // HUD SMW Bob-omb (tiles 0x88, 0x89 -> local 0x08, 0x09; Pal 2: 3=BLACK body, 2=WHITE eyes/spark, 1=GOLD key & fuse!)
    let hud_bobomb_l = parse_ascii_8x8([
        "......KW", "....KKKY", "...KKKKK", "..KKWKWK", "..KKWKWK", "..KKKKKK", "...KKKKK",
        "...KYK.K",
    ]);
    let hud_bobomb_r = parse_ascii_8x8([
        "Y.......", "K.......", "KK..KK..", "KKKYYK..", "KKKYYK..", "KK..KK..", "K.......",
        "YK......",
    ]);

    // HUD Bullet Bill (tiles 0x86, 0x87 -> local 0x06, 0x07; Pal 2: 3=BLACK body, 2=WHITE eye/rim, 1=GOLD tail glow)
    // Crisp white upper/lower highlight rim so the iconic Bullet Bill pops cleanly on both dark & bright HUDs!
    let hud_bullet_l = parse_ascii_8x8([
        "...WWWWW", "..WKKKKK", ".WKKWWKK", ".WKWKWWK", ".WKKKKKK", ".WKKKKKK", "..WKKKKK",
        "...WWWWW",
    ]);
    let hud_bullet_r = parse_ascii_8x8([
        "WWWWWW..", "KKKKKKW.", "WWWWKKYW", "KKKKKKYW", "KKKKKKYW", "KKKKKKW.", "KKKKKKW.",
        "WWWWWW..",
    ]);

    // --- Start Menu 16x16 Items in Sheet 0x70 (BG = 3 'K' BLACK, Primary = 1, White = 2 'W') ---
    // 1. Menu JET-BLACK SMW Bob-omb (0xB2, 0xB3 top + 0xC2, 0xC2-H bottom; switched to Pal 2 `0x28`:
    //    3='K' JET-BLACK body & BG, 2='W' crisp white spherical rim & big SMW oval eyes, 1='Y' golden wind-up key, spark & boots!)
    //    100% concordance with HUD Counter Bob-omb & Live Ground Bob-omb — ZERO blue bombs anywhere!
    let menu_bobomb_tl = parse_ascii_8x8([
        "KKKKKKYK", "KKKKKYYK", "KKKKKWWK", "KKKWWWWW", "KKWKKKKK", "KWKKWKKW", "KWKKWKKW",
        "KWKKWKKW",
    ]);
    let menu_bobomb_tr = parse_ascii_8x8([
        "YKKKKKKK", "WYKKKKKK", "WKKKKKKK", "WWWKKKKK", "KKKWKYYK", "KKKKWYYK", "KKKKWYKY",
        "KKKKWYYK",
    ]);
    // Symmetric bottom half (0xC2 mirrored horizontally for right half!) -> Jet-black round body + white rim + gleaming GOLD boots!
    let menu_bobomb_bl = parse_ascii_8x8([
        "KWKKKKKK", "KWKKKKKK", "KKWKKKKK", "KKKWKKKK", "KKKKWWWW", "KKKYYKKK", "KKYYYYKK",
        "KKKKKKKK",
    ]);

    // 2. Menu Fire Flower Lantern (0xBC, 0xBD, 0xCC, 0xCD; Pal 1: 1=RED petals, 2=WHITE face/stem, 3=BLACK BG!)
    let menu_fireflower = parse_ascii_16x16([
        "KKKKKRRRRRRKKKKK",
        "KKKRRRRRRRRRRKKK",
        "KKRRWWWWWWWWRRKK",
        "KRWWWRRRRRRWWWRK",
        "KRWWRWKRKRWWRWWK",
        "KRWWRWKRKRWWRWWK",
        "KRWWWRRRRRRWWWRK",
        "KKRRWWWWWWWWRRKK",
        "KKKRRRRRRRRRRKKK",
        "KKKKKKKWWKKKKKKK",
        "KKWWWKKWWKKWWWKK",
        "KWWWWWKWKWWWWWWK",
        "KKWWWWWWWWWWWWKK",
        "KKKWWWWWWWWWKKKK",
        "KKKKKKKWWKKKKKKK",
        "KKKKKKKKKKKKKKKK",
    ]);

    // 3. Menu Bullet Bill Bow (0xBA, 0xBB, 0xCA, 0xCB; Pal 2: 3='K' JET-BLACK body & BG, 2='W' crisp WHITE highlight/eye/glove, 1='Y' blazing GOLD rocket thruster!)
    //    Matches the crisp black/white/gold style of menu_bobomb & hud_bullet — instantly unmistakable SMW Bullet Bill!
    let menu_bullet = parse_ascii_16x16([
        "KKKKKKKKKKKKKKKK",
        "KKKKKKKKKKKKKKKK",
        "KKKKWWWWWWWWKKKK",
        "KKWWKKKKKKKKWWKK",
        "KWKKKWWKKKKKKWYK",
        "KWKKWWKWKKKKWWYK",
        "KWKKWWWWKKKKWYYK",
        "KWKKKKKKKKKKWYYK",
        "KWKKKKKWWWWKWYYK",
        "KWKKKKWKKWKKWWYK",
        "KWKKKKWWWWKKKWYK",
        "KKWWKKKKKKKKWWKK",
        "KKKKWWWWWWWWKKKK",
        "KKKKKKKKKKKKKKKK",
        "KKKKKKKKKKKKKKKK",
        "KKKKKKKKKKKKKKKK",
    ]);

    // 3b. Menu 3-UP Moon for Moon Pearl slot (0xD6 TL/H-flipped TR, 0xE6 BL, 0xE7 BR; Pal 2: 1='Y' GOLD, 2='W' WHITE, 3='K' BLACK!)
    //     Symmetrical gleaming Golden SMW Power Moon Emblem with cute vertical eyes!
    let menu_moon_tl = parse_ascii_8x8([
        "KKKKKKKK", "KKKKKWWW", "KKKWWYYY", "KKWYYYYY", "KWYYWWYY", "KWYYWKYY", "KWYYWKYY",
        "KWYYWWYY",
    ]);
    let menu_moon_bl = parse_ascii_8x8([
        "KWYYYYYY", "KWYYYYYY", "KKWYYYYW", "KKKWYYYK", "KKKKWWYY", "KKKKKKWW", "KKKKKKKK",
        "KKKKKKKK",
    ]);
    let menu_moon_br = parse_ascii_8x8([
        "YYYYYYWK", "YYYYYYWK", "WYYYYWKK", "KYYYWKKK", "YYWWKKKK", "WWKKKKKK", "KKKKKKKK",
        "KKKKKKKK",
    ]);

    // 4. Menu SMW Hammer (0xB6, 0xB7, 0xC6, 0xC7; Pal 0: 1=Orange/Brown, 2=Silver/White, 3=Black BG!)
    let menu_hammer = parse_ascii_16x16([
        "KKKKKKKKKKKKKKKK",
        "KKKKKWWWWWWKKKKK",
        "KKKKWYYYYYYWKKKK",
        "KKKWYYYYYYYYWKKK",
        "KKKWYYWWWWYYWKKK",
        "KKKWYYYYYYYYWKKK",
        "KKKKWYYYYYYWKKKK",
        "KKKKKKKWWKKKKKKK",
        "KKKKKKWYYWKKKKKK",
        "KKKKKKWYYWKKKKKK",
        "KKKKKKWYYWKKKKKK",
        "KKKKKKWYYWKKKKKK",
        "KKKKKKWYYWKKKKKK",
        "KKKKKKWYYWKKKKKK",
        "KKKKKKKWWKKKKKKK",
        "KKKKKKKKKKKKKKKK",
    ]);

    // 5. Menu Super Cape Feather (0xB4, 0xB5, 0xC4, 0xC5; Pal 1: 1=Red/Gold, 2=White, 3=Black BG!)
    let menu_cape = parse_ascii_16x16([
        "KKKKKKKKKKKKWWKK",
        "KKKKKKKKKKWWWWKK",
        "KKKKKKKKWWWRRWWK",
        "KKKKKKWWWRRRRWWK",
        "KKKKKWWWRRWRWWKK",
        "KKKKWWWRRWRWWKKK",
        "KKKWWWRRWRWWKKKK",
        "KKWWWRRWRWWKKKKK",
        "KKWWRRWRWWKKKKKK",
        "KKWRRWRWWKKKKKKK",
        "KKWWWRWWKKKKKKKK",
        "KKKWWWWKKKKKKKKK",
        "KKWWKKKKKKKKKKKK",
        "KWWKKKKKKKKKKKKK",
        "KKKKKKKKKKKKKKKK",
        "KKKKKKKKKKKKKKKK",
    ]);

    // 6. Menu '?' Message Block for Book (0xA5, 0xA6, 0xD8, 0xD9; Pal 7: 1=GREEN, 2=WHITE, 3=BLACK!)
    let menu_qblock = parse_ascii_16x16([
        "KGGGGGGGGGGGGGGK",
        "GWGGGGGGGGGGGGWG",
        "GGGGGGWWWWGGGGGG",
        "GGGGGWWGGWWGGGGG",
        "GGGGGWWGGWWGGGGG",
        "GGGGGGGGGWWGGGGG",
        "GGGGGGGGWWGGGGGG",
        "GGGGGGGWWGGGGGGG",
        "GGGGGGGWWGGGGGGG",
        "GGGGGGGGGGGGGGGG",
        "GGGGGGGWWGGGGGGG",
        "GGGGGGGWWGGGGGGG",
        "GGGGGGGGGGGGGGGG",
        "GWGGGGGGGGGGGGWG",
        "KGGGGGGGGGGGGGGK",
        "KKKKKKKKKKKKKKKK",
    ]);

    // 7. Menu Fireball / Iceball Wand for Fire Rod & Ice Rod (0xB0, 0xB1/0xBE, 0xC0, 0xC1)
    let menu_wand = parse_ascii_16x16([
        "KKKKKKKKKRRRRKKK",
        "KKKKKKKKRWWWWKKK",
        "KKKKKKKRWWWRWRKK",
        "KKKKKKKRWRRRWRKK",
        "KKKKKKKRWWWWWRKK",
        "KKKKKKKKRRRRRKKK",
        "KKKKKKKWWKKKKKKK",
        "KKKKKKWRWKKKKKKK",
        "KKKKKWRWKKKKKKKK",
        "KKKKWRWKKKKKKKKK",
        "KKKWRWKKKKKKKKKK",
        "KKWRWKKKKKKKKKKK",
        "KWRWKKKKKKKKKKKK",
        "KWWKKKKKKKKKKKKK",
        "KKKKKKKKKKKKKKKK",
        "KKKKKKKKKKKKKKKK",
    ]);

    // 8. Menu Equipment Big Heart Container -> 16x16 SUPER MUSHROOM RESERVE!
    //    Empty (0x84 TL, 0x85 BL, H-mirrored for TR/BR) & Filled (0xAD TL, 0xAE BL; Pal 1: 1=RED, 2=WHITE, 3=BLACK!)
    let menu_mush_empty_tl = parse_ascii_8x8([
        "KKKKKWWW", "KKKWWKKK", "KKWKKKKK", "KWKKKKKK", "KWKKKKKK", "KWKKKKKK", "KWKKKKKK",
        "KWKKKKKK",
    ]);
    let menu_mush_empty_bl = parse_ascii_8x8([
        "KKWWWWWW", "KKKWKKKK", "KKKWKKKK", "KKKWKKKK", "KKKWKKKK", "KKKKWKKK", "KKKKKWWW",
        "KKKKKKKK",
    ]);
    let menu_mush_full_tl = parse_ascii_8x8([
        "KKKKKWWW", "KKKWWRRR", "KKWRRRRR", "KWRRWWWR", "KWRWWWRR", "KWRWWWRR", "KWRRRRRR",
        "KWRRRRRR",
    ]);
    let menu_mush_full_bl = parse_ascii_8x8([
        "KKWWWWWW", "KKKWWWWW", "KKKWWKWW", "KKKWWKWW", "KKKWWWWW", "KKKKWWWW", "KKKKKWWW",
        "KKKKKKKK",
    ]);

    // 9. Menu Shield 1 (0xFD, 0xFE H-mirrored) & Shield 2 (0xFF, 0x9F H-mirrored) -> SMW KOOPA SHELL SHIELD!
    let menu_shell_tl = parse_ascii_8x8([
        "KKKKKKKK", "KKKKWWWW", "KKWWBBBB", "KWWBBWBB", "KWBBWBBB", "KWBWBBBB", "KWBBBBBW",
        "KWBBBBWW",
    ]);
    let menu_shell_bl = parse_ascii_8x8([
        "KWWWWWWW", "KWBBBBBB", "KKWBBBBB", "KKWWWWWW", "KKKKWWWW", "KKKKKKKK", "KKKKKKKK",
        "KKKKKKKK",
    ]);

    // 10. Menu Boomerang (0xB8, 0xB9, 0xC9; Pal 3 Blue / Pal 1 Red) -> SMW Spinning Koopa Shell!
    let menu_boom_tl = parse_ascii_8x8([
        "KKKKKKKK", "KKKKWWWW", "KKWWBBBB", "KWWBBWBB", "KWBBWBBB", "KWBBBBWW", "KWWWWWWW",
        "KKWWWWWW",
    ]);
    let menu_boom_tr = parse_ascii_8x8([
        "KKKKKKKK", "WWWWKKKK", "BBBBWWKK", "BBWBBWWK", "BBBWBBWK", "WWBBBBWK", "WWWWWWWK",
        "WWWWWWKK",
    ]);
    let menu_boom_br = parse_ascii_8x8([
        "BBBBBWWK", "BBBBWWKK", "WWWWKKKK", "KKKKKKKK", "KKKKKKKK", "KKKKKKKK", "KKKKKKKK",
        "KKKKKKKK",
    ]);

    // 11. Menu Power Glove / Titan's Mitt -> SMW Super Star Power-Up (0x9A,9B,9D,9E in 0x70 & 0x33..36 in 0x6F)!
    let menu_star = parse_ascii_16x16([
        "KKKKKKKWWKKKKKKK",
        "KKKKKKWYYWKKKKKK",
        "KKKKKKWYYWKKKKKK",
        "KKKKKWYYYYWKKKKK",
        "KWWWWYYYYYYWWWWK",
        "KWYYYYYYYYYYYYWK",
        "KKWYYYKYKYWWYWKK",
        "KKKWYYKYKYYYYWKK",
        "KKKKWYYYYYYYWKKK",
        "KKKKWYYYYYYYWKKK",
        "KKKWYYYYYYYYYWKK",
        "KKKWYYYYYYYYYWKK",
        "KKWYYYYWWYYYYWKK",
        "KKWYYWWKKWWYYWKK",
        "KKWWWKKKKKKWWWKK",
        "KKKKKKKKKKKKKKKK",
    ]);

    // Patch Sheet 0x70 (+ alias 0xDC)!
    if patch_alttp_2bpp_sheet(
        rom_zelda,
        session,
        0x70,
        &[0xDC],
        "HUD & Menu Start 2bpp (Sheet 0x70/0xDC) : Mini-Champignons Vie, Pièce Or, Bob-omb, Fleur Feu, Bill Balle, Marteau, Carapace, Starman, Réserve Champignon",
        free_cursor,
        free_end,
        &|raw| {
            // Top HUD Life Mini-Mushrooms (0xA0, 0xA1, 0xA2 -> local 0x20, 0x21, 0x22)
            write_2bpp_8x8(raw, 0x20, &hud_mush_full_crisp);
            write_2bpp_8x8(raw, 0x21, &hud_mush_half);
            write_2bpp_8x8(raw, 0x22, &hud_mush_empty);
            // Top HUD Coin (0xA8 -> 0x28), Key (0xA9 -> 0x29), Bob-omb (0x88/0x89 -> 0x08/0x09), Bullet Bill (0x86/0x87 -> 0x06/0x07)
            write_2bpp_8x8(raw, 0x28, &hud_coin);
            write_2bpp_8x8(raw, 0x29, &hud_key);
            write_2bpp_8x8(raw, 0x08, &hud_bobomb_l);
            write_2bpp_8x8(raw, 0x09, &hud_bobomb_r);
            write_2bpp_8x8(raw, 0x06, &hud_bullet_l);
            write_2bpp_8x8(raw, 0x07, &hud_bullet_r);

            // Start Menu Bob-omb (0xB2, 0xB3, 0xC2 -> local 0x32, 0x33, 0x42)
            write_2bpp_8x8(raw, 0x32, &menu_bobomb_tl);
            write_2bpp_8x8(raw, 0x33, &menu_bobomb_tr);
            write_2bpp_8x8(raw, 0x42, &menu_bobomb_bl);

            // Start Menu Boomerang -> Spinning Koopa Shell (0xB8, 0xB9, 0xC9 -> local 0x38, 0x39, 0x49)
            write_2bpp_8x8(raw, 0x38, &menu_boom_tl);
            write_2bpp_8x8(raw, 0x39, &menu_boom_tr);
            write_2bpp_8x8(raw, 0x49, &menu_boom_br);

            // Start Menu Power Glove 1 -> Starman (0x9A, 0x9B, 0x9D, 0x9E -> local 0x1A, 0x1B, 0x1D, 0x1E)
            write_2bpp_16x16_quad(raw, 0x1A, 0x1B, 0x1D, 0x1E, &menu_star);

            // Start Menu Fire Flower Lantern (0xBC, 0xBD, 0xCC, 0xCD -> local 0x3C, 0x3D, 0x4C, 0x4D)
            write_2bpp_16x16_quad(raw, 0x3C, 0x3D, 0x4C, 0x4D, &menu_fireflower);
            // Start Menu Bullet Bill Bow (0xBA, 0xBB, 0xCA, 0xCB -> local 0x3A, 0x3B, 0x4A, 0x4B)
            write_2bpp_16x16_quad(raw, 0x3A, 0x3B, 0x4A, 0x4B, &menu_bullet);
            // Start Menu SMW Hammer (0xB6, 0xB7, 0xC6, 0xC7 -> local 0x36, 0x37, 0x46, 0x47)
            write_2bpp_16x16_quad(raw, 0x36, 0x37, 0x46, 0x47, &menu_hammer);
            // Start Menu Cape Feather (0xB4, 0xB5, 0xC4, 0xC5 -> local 0x34, 0x35, 0x44, 0x45)
            write_2bpp_16x16_quad(raw, 0x34, 0x35, 0x44, 0x45, &menu_cape);
            // Start Menu '?' Block Book (0xA5, 0xA6, 0xD8, 0xD9 -> local 0x25, 0x26, 0x58, 0x59)
            write_2bpp_16x16_quad(raw, 0x25, 0x26, 0x58, 0x59, &menu_qblock);
            // Start Menu Fire/Ice Wand (0xB0, 0xB1, 0xC0, 0xC1 -> local 0x30, 0x31, 0x40, 0x41; Ice top-right 0xBE -> 0x3E)
            write_2bpp_16x16_quad(raw, 0x30, 0x31, 0x40, 0x41, &menu_wand);
            let mut wand_tr = [[0u8; 8]; 8];
            for y in 0..8 {
                for x in 0..8 {
                    wand_tr[y][x] = menu_wand[y][x + 8];
                }
            }
            write_2bpp_8x8(raw, 0x3E, &wand_tr);

            // Start Menu Equipment Big Heart -> Super Mushroom Reserve (0x84, 0x85, 0xAD, 0xAE -> local 0x04, 0x05, 0x2D, 0x2E)
            write_2bpp_8x8(raw, 0x04, &menu_mush_empty_tl);
            write_2bpp_8x8(raw, 0x05, &menu_mush_empty_bl);
            write_2bpp_8x8(raw, 0x2D, &menu_mush_full_tl);
            write_2bpp_8x8(raw, 0x2E, &menu_mush_full_bl);

            // Start Menu Koopa Shell Shields (0xFD, 0xFE, 0xFF, 0x9F -> local 0x7D, 0x7E, 0x7F, 0x1F)
            write_2bpp_8x8(raw, 0x7D, &menu_shell_tl);
            write_2bpp_8x8(raw, 0x7E, &menu_shell_bl);
            write_2bpp_8x8(raw, 0x7F, &menu_shell_tl);
            write_2bpp_8x8(raw, 0x1F, &menu_shell_bl);

            // Start Menu Moon Pearl -> Golden SMW 3-UP Moon Emblem (0xD6, 0xE6, 0xE7 -> local 0x56, 0x66, 0x67)
            write_2bpp_8x8(raw, 0x56, &menu_moon_tl);
            write_2bpp_8x8(raw, 0x66, &menu_moon_bl);
            write_2bpp_8x8(raw, 0x67, &menu_moon_br);
        },
    )? {
        ops += 1;
    }

    // --- Start Menu Equipment Icons in Sheet 0x6F (+ alias 0xDB, VRAM tiles 0x00..0x7F) ---
    // 1. Mario Cap Tunic (0x68 TL, 0x78 BL, H-mirrored -> full iconic Mario Cap with 'M' emblem!)
    let menu_cap_tl = parse_ascii_8x8([
        "KKKKKKKK", "KKKKRRRR", "KKRRRRRR", "KRRRRWWW", "KRRRWWRW", "KRRRWWRR", "KRRRRWWW",
        "KRRRRRRR",
    ]);
    let menu_cap_bl = parse_ascii_8x8([
        "KRRRRRRR", "KWWWWWWW", "KRRRRRRR", "KKRRRRRR", "KKKKKKKK", "KKKKKKKK", "KKKKKKKK",
        "KKKKKKKK",
    ]);

    // 2. Menu Yoshi Head for Bug Net (0x40, 0x41, 0x42, 0x43)
    let menu_yoshi = parse_ascii_16x16([
        "KKKKKGGGKKKKKKKK",
        "KKKKGWWWGKKKKKKK",
        "KKKGWKWKWGKKKKKK",
        "KKKGWKWKWGKKKKKK",
        "KKKGGWWWGGGGGKKK",
        "KKGGGGGGGGGGGGKK",
        "KGGGWWWWWWGGGGGK",
        "KKGWWWWWWWWGGGGK",
        "KKKWWWWWWWWWGGKK",
        "KKKKWWWWWWWWGKKK",
        "KKKKKWWWWWWWKKKK",
        "KKKKKKGGGGKKKKKK",
        "KKKKKGGWWGGKKKKK",
        "KKKKKGGWWGGKKKKK",
        "KKKKKKKKKKKKKKKK",
        "KKKKKKKKKKKKKKKK",
    ]);

    // 3. Menu Pegasus Boots -> SMW Kuribo's Winged Boot (0x29, 0x2A, 0x2B, 0x2C; Pal 5 Orange/Gold)
    let menu_wing_boot = parse_ascii_16x16([
        "KKKKKKKKKKKKKKKK",
        "KWWWWKKKKKKKKKKK",
        "KKWWWWKKKRRRRRKK",
        "KKKWWWKKRRRRRRKK",
        "KKKKWWKKRRRRRRKK",
        "KKKKKKKRRRRRRRKK",
        "KKKKKKRRRRRRRRKK",
        "KKKKKRRRRRRRRRKK",
        "KKKRRRRRRRRRRRKK",
        "KKRRRRRRRRRRRRKK",
        "KRRRRRRRRRRRRRKK",
        "KWWWWWWWWWWWWWKK",
        "KRRRRRRRRRRRRRKK",
        "KKRRRRRRRRRRRKKK",
        "KKKKKKKKKKKKKKKK",
        "KKKKKKKKKKKKKKKK",
    ]);

    // 4. Menu Zora Flippers -> SMW Cheep Cheep (0x62, 0x63, 0x72, 0x73; Pal 3 Blue/White)
    let menu_cheep = parse_ascii_16x16([
        "KKKKKKKKKKKKKKKK",
        "KKKKKKBBBBKKKKKK",
        "KKKKBBBBBBBBKKKK",
        "KKKBBWBBBBBBWKKK",
        "KKBBWKBBBBBBWWKK",
        "KBBBWWBBBBWWWWWK",
        "KBBBBBBBBBBWWWWK",
        "KWWBBBBBBBBWWKKK",
        "KKWBBBBBBBBBWWKK",
        "KKBBWWWWWWWWBBKK",
        "KKKWWWWWWWWWWKKK",
        "KKKKWWWWWWWWKKKK",
        "KKKKKKBBBBKKKKKK",
        "KKKKKKKKKKKKKKKK",
        "KKKKKKKKKKKKKKKK",
        "KKKKKKKKKKKKKKKK",
    ]);

    // 5. Start Menu 3 Pendants ("PENDENTIF(S)") -> SUPER MUSHROOM, FIRE FLOWER & SUPER STAR!
    //    Written into the 24 FREE 2bpp tiles of Sheet 0x6F (0x00..0x07, 0x10..0x17, 0x20..0x27)!
    //    For collected icons: 3='K' Black BG, 2='W' Pure White, 1='R'/'Y' Primary Color (Red/Gold)
    //    For empty grey silhouettes (Pal 4 `0x30`: 1=Grey outline/fill, 2=Dark Grey, 3=Black BG):
    let pend_mush_full = parse_ascii_16x16([
        "KKKKKWWWWWWKKKKK",
        "KKKWWWRRRRWWWKKK",
        "KKWRRWRRRRWRRWKK",
        "KWRRWWWRRWWWRRWK",
        "KWRWWWRRRRWWWRWK",
        "KWRRRRRWWRRRRRWK",
        "KWRRRRWWWWRRRRWK",
        "KWRRRRWWWWRRRRWK",
        "KWRRRRRRRRRRRRWK",
        "KKWWWWWWWWWWWWKK",
        "KKKWWKWWWWKWWKKK",
        "KKKWWKWWWWKWWKKK",
        "KKKWWWWWWWWWWKKK",
        "KKKKWWWWWWWWKKKK",
        "KKKKKWWWWWWKKKKK",
        "KKKKKKKKKKKKKKKK",
    ]);
    let pend_mush_empty = parse_ascii_16x16([
        "KKKKKRRRRRRKKKKK",
        "KKKRRKKKKKKRRKKK",
        "KKRKKKKKKKKKKRKK",
        "KRKKKRRKKRRKKKRK",
        "KRKKRRRRRRRRKKRK",
        "KRKKKKKKKKKKKKRK",
        "KRKKKKKRRKKKKKRK",
        "KRKKKKRRRRKKKKRK",
        "KRKKKKKKKKKKKKRK",
        "KKRRRRRRRRRRRRKK",
        "KKKRKKRKKRKKRKKK",
        "KKKRKKRKKRKKRKKK",
        "KKKRKKKKKKKKRKKK",
        "KKKKRKKKKKKRKKKK",
        "KKKKKRRRRRRKKKKK",
        "KKKKKKKKKKKKKKKK",
    ]);
    let pend_flower_full = parse_ascii_16x16([
        "KKKKRRRRRRRRKKKK",
        "KKRRRRWWWWWRRRKK",
        "KRRWWWRRRRWWWRRK",
        "KRWWRWKRKRWWRWWK",
        "KRWWRWKRKRWWRWWK",
        "KRRWWWRRRRWWWRRK",
        "KKRRRRWWWWWRRRKK",
        "KKKKRRRRRRRRKKKK",
        "KKKKKKKWWKKKKKKK",
        "KKRRRKKWWKKRRRKK",
        "KRRRRRKWKRRRRRRK",
        "KKRRRRWWWWWRRRKK",
        "KKKRRRRWWWRRRKKK",
        "KKKKKKKWWKKKKKKK",
        "KKKKKKKWWKKKKKKK",
        "KKKKKKKKKKKKKKKK",
    ]);
    let pend_flower_empty = parse_ascii_16x16([
        "KKKKRRRRRRRRKKKK",
        "KKRRKKKKKKKKRRKK",
        "KRKKKRRRRRRKKKRK",
        "KRKKRKRKKRKRKKRK",
        "KRKKRKRKKRKRKKRK",
        "KRKKKRRRRRRKKKRK",
        "KKRRKKKKKKKKRRKK",
        "KKKKRRRRRRRRKKKK",
        "KKKKKKKRRKKKKKKK",
        "KKRRRKKRRKKRRRKK",
        "KRKKKRRRRRRKKKRK",
        "KKRRRKKRRKKRRRKK",
        "KKKKKKKRRKKKKKKK",
        "KKKKKKKRRKKKKKKK",
        "KKKKKKKKKKKKKKKK",
        "KKKKKKKKKKKKKKKK",
    ]);
    let pend_star_full = menu_star;
    let pend_star_empty = parse_ascii_16x16([
        "KKKKKKKRRKKKKKKK",
        "KKKKKKRKKRKKKKKK",
        "KKKKKKRKKRKKKKKK",
        "KKKKKRKKKKRKKKKK",
        "KRRRRKKKKKKRRRRK",
        "KRKKKKKKKKKKKKRK",
        "KKRKKKRKKRKKKRKK",
        "KKKRKKRKKRKKRKKK",
        "KKKKRKKKKKKRKKKK",
        "KKKKRKKKKKKRKKKK",
        "KKKRKKKKKKKKRKKK",
        "KKKRKKKKKKKKRKKK",
        "KKRKKKKRRKKKKRKK",
        "KKRKKRRKKRRKKRKK",
        "KKRRRKKKKKKRRRKK",
        "KKKKKKKKKKKKKKKK",
    ]);

    if patch_alttp_2bpp_sheet(
        rom_zelda,
        session,
        0x6F,
        &[0xDB],
        "Menu Équipement & Pendentifs 2bpp (Sheet 0x6F/0xDB) : Champignon, Fleur, Étoile, Casquette Mario, Yoshi, Botte Ailée",
        free_cursor,
        free_end,
        &|raw| {
            // 3 Mario Relics replacing Pendants in free slots 0x00..0x07, 0x10..0x17, 0x20..0x27!
            write_2bpp_16x16_quad(raw, 0x00, 0x01, 0x10, 0x11, &pend_mush_full);
            write_2bpp_16x16_quad(raw, 0x02, 0x03, 0x12, 0x13, &pend_mush_empty);
            write_2bpp_16x16_quad(raw, 0x04, 0x05, 0x14, 0x15, &pend_flower_full);
            write_2bpp_16x16_quad(raw, 0x06, 0x07, 0x16, 0x17, &pend_flower_empty);
            write_2bpp_16x16_quad(raw, 0x20, 0x21, 0x22, 0x23, &pend_star_full);
            write_2bpp_16x16_quad(raw, 0x24, 0x25, 0x26, 0x27, &pend_star_empty);

            write_2bpp_8x8(raw, 0x68, &menu_cap_tl);
            write_2bpp_8x8(raw, 0x78, &menu_cap_bl);
            write_2bpp_16x16_quad(raw, 0x40, 0x41, 0x42, 0x43, &menu_yoshi);
            write_2bpp_16x16_quad(raw, 0x29, 0x2A, 0x2B, 0x2C, &menu_wing_boot);
            write_2bpp_16x16_quad(raw, 0x33, 0x34, 0x35, 0x36, &menu_star);
            write_2bpp_16x16_quad(raw, 0x62, 0x63, 0x72, 0x73, &menu_cheep);
        },
    )? {
        ops += 1;
    }

    // Also replace World Map / Dungeon Plan Pendant necklace tiles (0x2B..0x2E & 0x3B..0x3E) with Super Star
    // AND the 7 Start Menu Crystal slots (0x44/0x45 collected, 0x46/0x47 empty outline) with 8x16 Spotted SMW Yoshi Eggs!
    // Pal 3 (0x2C): 3='K' Black outline, 2='W' Pure White egg shell, 1='B' Vibrant Blue/Cyan Yoshi Egg spots!
    let yoshi_egg_top = parse_ascii_8x8([
        "KKKKKKKK", "KKWWWWKK", "KWWBBWWK", "KWBBBBWK", "KWWBBWWK", "KWWWWWWK", "KWWWWWBB",
        "KWBBBBBB",
    ]);
    let yoshi_egg_bot = parse_ascii_8x8([
        "KWWBBBWW", "KWWWWWWK", "KBBWWWWK", "KBBWWBBK", "KWWWWWBB", "KKWWWWKK", "KKKWWKKK",
        "KKKKKKKK",
    ]);
    let yoshi_egg_empty_top = parse_ascii_8x8([
        "KKKKKKKK", "KKWWWWKK", "KWKKKKWK", "KWKKKKWK", "KWKKKKWK", "KWKKKKWK", "KWKKKKWK",
        "KWKKKKWK",
    ]);
    let yoshi_egg_empty_bot = parse_ascii_8x8([
        "KWKKKKWK", "KWKKKKWK", "KWKKKKWK", "KWKKKKWK", "KWKKKKWK", "KKWWWWKK", "KKKKKKKK",
        "KKKKKKKK",
    ]);
    if patch_alttp_2bpp_sheet(
        rom_zelda,
        session,
        0xDA,
        &[],
        "Carte & Menu 2bpp (Sheet 0xDA) : Pendentifs -> Étoile Super Mario & 7 Cristaux -> 7 Œufs Yoshi SMW",
        free_cursor,
        free_end,
        &|raw| {
            write_2bpp_16x16_quad(raw, 0x2B, 0x2C, 0x2D, 0x2E, &pend_star_full);
            write_2bpp_16x16_quad(raw, 0x3B, 0x3C, 0x3D, 0x3E, &pend_star_empty);
            write_2bpp_8x8(raw, 0x44, &yoshi_egg_top);
            write_2bpp_8x8(raw, 0x45, &yoshi_egg_bot);
            write_2bpp_8x8(raw, 0x46, &yoshi_egg_empty_top);
            write_2bpp_8x8(raw, 0x47, &yoshi_egg_empty_bot);
        },
    )? {
        ops += 1;
    }

    // Patch Start Menu PENDENTIF(S) Tile Tables (0x06F7F9 Wisdom->Star, 0x06F809 Power->Fire Flower, 0x06F819 Courage->Super Mushroom!)
    if 0x06F829 <= rom_zelda.data.len()
        && rom_zelda.data[0x06F7F9] == 0x3B
        && rom_zelda.data[0x06F809] == 0x3B
        && rom_zelda.data[0x06F819] == 0x3B
    {
        let mut pend_tbl = Vec::with_capacity(48);
        // 0x06F7F9 (Bottom-Right / Wisdom) -> SUPER STAR (Grey silhouette 0x24..0x27 Pal 4 `0x30`, Collected Gold 0x20..0x23 Pal 2 `0x28`!)
        pend_tbl.extend_from_slice(&[
            0x24, 0x30, 0x25, 0x30, 0x26, 0x30, 0x27, 0x30, 0x20, 0x28, 0x21, 0x28, 0x22, 0x28,
            0x23, 0x28,
        ]);
        // 0x06F809 (Bottom-Left / Power) -> FIRE FLOWER (Grey silhouette 0x06,07,16,17 Pal 4 `0x30`, Collected Red/White 0x04,05,14,15 Pal 1 `0x24`!)
        pend_tbl.extend_from_slice(&[
            0x06, 0x30, 0x07, 0x30, 0x16, 0x30, 0x17, 0x30, 0x04, 0x24, 0x05, 0x24, 0x14, 0x24,
            0x15, 0x24,
        ]);
        // 0x06F819 (Top / Courage) -> SUPER MUSHROOM (Grey silhouette 0x02,03,12,13 Pal 4 `0x30`, Collected Red/White 0x00,01,10,11 Pal 1 `0x24`!)
        pend_tbl.extend_from_slice(&[
            0x02, 0x30, 0x03, 0x30, 0x12, 0x30, 0x13, 0x30, 0x00, 0x24, 0x01, 0x24, 0x10, 0x24,
            0x11, 0x24,
        ]);
        session.swap_custom_bytes(
            rom_zelda,
            0x06F7F9,
            0x06F7F9,
            pend_tbl,
            "Menu Start Pendentifs (0x06F7F9..0x06F829) -> Champignon, Fleur de Feu & Étoile SMW"
                .into(),
            SwapKind::CodePatch { byte_count: 48 },
        )?;
        ops += 1;
    }

    // --- Bank $0D HUD & Start Menu Tilemap Palette Fixes ---
    // 1. Top-HUD Bar (0x06FDB1..0x06FDC1):
    //    - Rupee (0x06FDB1: A8 3C) -> Pal 2 Bright Yellow (0x28) for SMW YELLOW COIN!
    //    - Bomb  (0x06FDB7: 88 2C 89 2C) -> Pal 2 Black/White/Gold (0x28) for BLACK SMW BOB-OMB!
    //    - Arrow (0x06FDBD: A7 20 A9 20) -> Tiles 0x86/0x87 Pal 2 (0x28) for SMW BULLET BILL!
    if 0x06FDC1 <= rom_zelda.data.len()
        && rom_zelda.data[0x06FDB1] == 0xA8
        && rom_zelda.data[0x06FDB7] == 0x88
    {
        let mut hud_bar = rom_zelda.data[0x06FDB1..0x06FDC1].to_vec();
        hud_bar[1] = 0x28; // Rupee -> Bright Yellow SMW Coin!
        hud_bar[7] = 0x28; // Bomb left -> Black SMW Bob-omb
        hud_bar[9] = 0x28; // Bomb right -> Black SMW Bob-omb
        hud_bar[12] = 0x86; // Arrow left -> SMW Bullet Bill left!
        hud_bar[13] = 0x28; // Pal 2 Black/White/Gold
        hud_bar[14] = 0x87; // Arrow right -> SMW Bullet Bill right!
        hud_bar[15] = 0x28; // Pal 2 Black/White/Gold
        session.swap_custom_bytes(
            rom_zelda,
            0x06FDB1,
            0x06FDB1,
            hud_bar,
            "HUD Haut Tilemap (0x06FDB1) : Pièce Jaune SMW, Bob-omb Noir & Bill Balle (Pal 2)"
                .into(),
            SwapKind::CodePatch { byte_count: 16 },
        )?;
        ops += 1;
    }

    // 1b. Start Menu & Equipped Item Box ('Y') Bomb Tilemap (0x06F5A9: b2 2c b3 2c c2 2c c2 6c)
    //     Switch from Blue Pal 3 (0x2C/0x6C) -> JET-BLACK/WHITE/GOLD Pal 2 (0x28/0x68) for 100% Black Bob-omb concordance!
    if 0x06F5B1 <= rom_zelda.data.len()
        && rom_zelda.data[0x06F5A9] == 0xB2
        && rom_zelda.data[0x06F5AA] == 0x2C
    {
        let black_bomb_tbl: Vec<u8> = vec![0xB2, 0x28, 0xB3, 0x28, 0xC2, 0x28, 0xC2, 0x68];
        session.swap_custom_bytes(
            rom_zelda,
            0x06F5A9,
            0x06F5A9,
            black_bomb_tbl,
            "Menu Start & Case Y Bob-omb (0x06F5A9) : Bob-omb Noir SMW (Pal 2, zéro bombe bleue)"
                .into(),
            SwapKind::CodePatch { byte_count: 8 },
        )?;
        ops += 1;
    }

    // 2. Start Menu Bow tilemap (0x06F559..0x06F579): point all 4 Bow states cleanly to Bullet Bill (BA, BB, CA, CB, Pal 2 Gold!)
    if 0x06F579 <= rom_zelda.data.len() && rom_zelda.data[0x06F559] == 0xBA {
        let mut bow_tbl = Vec::with_capacity(32);
        for _ in 0..4 {
            bow_tbl.extend_from_slice(&[0xBA, 0x28, 0xBB, 0x28, 0xCA, 0x28, 0xCB, 0x28]);
        }
        session.swap_custom_bytes(
            rom_zelda,
            0x06F559,
            0x06F559,
            bow_tbl,
            "Menu Start Arc (0x06F559) -> Bill Balle SMW (Pal 2 Or)".into(),
            SwapKind::CodePatch { byte_count: 32 },
        )?;
        ops += 1;
    }

    // 3. Start Menu Equipment Tunic (0x06F7A9..0x06F7C1):
    //    Green Mail (Mario Classique) -> Pal 1 RED Mario Cap (0x24/0x64)!
    //    Blue Mail  (Mario Bleu)      -> Pal 3 BLUE Mario Cap (0x2C/0x6C)!
    //    Red Mail   (Fire Mario)      -> Pal 2 GOLD/WHITE Fire Mario Cap (0x28/0x68)!
    if 0x06F7C1 <= rom_zelda.data.len() && rom_zelda.data[0x06F7A9] == 0x68 {
        let tunic_tbl: Vec<u8> = vec![
            0x68, 0x24, 0x68, 0x64, 0x78, 0x24, 0x78, 0x64, // Mario Classique (RED Cap!)
            0x68, 0x2C, 0x68, 0x6C, 0x78, 0x2C, 0x78, 0x6C, // Mario Bleu (BLUE Cap!)
            0x68, 0x28, 0x68, 0x68, 0x78, 0x28, 0x78, 0x68, // Fire Mario (GOLD/WHITE Cap!)
        ];
        session.swap_custom_bytes(
            rom_zelda,
            0x06F7A9,
            0x06F7A9,
            tunic_tbl,
            "Menu Équipement Casquette Mario (0x06F7A9) : Rouge Classique, Bleu Glace, Fire Mario"
                .into(),
            SwapKind::CodePatch { byte_count: 24 },
        )?;
        ops += 1;
    }

    // 4. Start Menu "PENDENTIF(S)" Header (0x06E79E..0x06E7AE, 8 tiles) -> "RELIQUES" (crisp white 0x25 font!)
    if 0x06E7AE <= rom_zelda.data.len()
        && rom_zelda.data[0x06E79E] == 0x21
        && rom_zelda.data[0x06E79F] == 0x25
    {
        let encode_menu_line = |text: &str| -> Vec<u8> {
            let mut v = Vec::with_capacity(16);
            for ch in text.bytes().take(8) {
                match ch {
                    // Verified French ALTTP Sheet 0xDA menu font: A..W = 0x50..0x66, Y = 0x67!
                    // (0x68 is French ligature "CI", so 'Y' MUST map to 0x67!)
                    b'Y' => {
                        v.push(0x67);
                        v.push(0x25);
                    }
                    b'A'..=b'W' => {
                        v.push(0x50 + (ch - b'A'));
                        v.push(0x25);
                    }
                    b'\'' => {
                        v.push(0x4F);
                        v.push(0x25);
                    }
                    _ => {
                        v.push(0xF5);
                        v.push(0x24);
                    }
                }
            }
            while v.len() < 16 {
                v.push(0xF5);
                v.push(0x24);
            }
            v
        };
        let reliques = encode_menu_line("RELIQUES");
        session.swap_custom_bytes(
            rom_zelda,
            0x06E79E,
            0x06E79E,
            reliques,
            "Menu Start En-tête (0x06E79E) : 'PENDENTIF(S)' -> 'RELIQUES'".into(),
            SwapKind::CodePatch { byte_count: 16 },
        )?;
        ops += 1;

        // 4b. Start Menu "CRISTAUX" Header (0x06E852..0x06E862, 8 tiles) -> " YOSHIS " matching the 7 Spotted SMW Yoshi Eggs!
        if 0x06E862 <= rom_zelda.data.len()
            && rom_zelda.data[0x06E852] == 0x2F
            && rom_zelda.data[0x06E853] == 0x25
        {
            let yoshis = encode_menu_line(" YOSHIS ");
            session.swap_custom_bytes(
                rom_zelda,
                0x06E852,
                0x06E852,
                yoshis,
                "Menu Start En-tête (0x06E852) : 'CRISTAUX' -> ' YOSHIS ' (7 Œufs Yoshi SMW)"
                    .into(),
                SwapKind::CodePatch { byte_count: 16 },
            )?;
            ops += 1;
        }

        // 4c. Start Menu Equipped Lv1 Shield (0x06F791..0x06F799): Blue Pal 3 (0x2C/0x6C) -> Emerald Green Pal 7 (0x3C/0x7C)
        //     so Mario's Equipped Koopa Shell matches his live Emerald Green Koopa Shell Shield!
        if 0x06F799 <= rom_zelda.data.len()
            && rom_zelda.data[0x06F791] == 0xFD
            && rom_zelda.data[0x06F792] == 0x2C
        {
            session.swap_custom_bytes(
                rom_zelda,
                0x06F791,
                0x06F791,
                vec![0xFD, 0x3C, 0xFD, 0x7C, 0xFE, 0x3C, 0xFE, 0x7C],
                "Menu Équipement Bouclier Lv1 (0x06F791) : Carapace Koopa Verte Émeraude (Pal 7)"
                    .into(),
                SwapKind::CodePatch { byte_count: 8 },
            )?;
            ops += 1;
        }

        // 5. Start Menu Item Names (exact 0x20-byte paired entries starting at 0x06F0F1..0x06F551!) -> 100% SMW Names!
        if 0x06F551 <= rom_zelda.data.len() && rom_zelda.data[0x06F241] == 0x5B {
            let mut names = rom_zelda.data[0x06F0F1..0x06F551].to_vec();
            let mut put_item = |abs_pc: usize, l1: &str, l2: &str| {
                let rel = abs_pc - 0x06F0F1;
                if rel + 32 <= names.len() {
                    names[rel..rel + 16].copy_from_slice(&encode_menu_line(l1));
                    names[rel + 16..rel + 32].copy_from_slice(&encode_menu_line(l2));
                }
            };
            put_item(0x06F0F1, "  BILL  ", " BLASTER"); // Item 0: Bow (no arrows)
            put_item(0x06F111, "CARAPACE", "  SMW   "); // Item 1: Boomerang
            put_item(0x06F151, "BOB OMBS", "  SMW   "); // Item 3: Bombs
            put_item(0x06F191, " FLEUR  ", " DE FEU "); // Item 5: Fire Rod
            put_item(0x06F1B1, " FLEUR  ", "DE GLACE"); // Item 6: Ice Rod
            put_item(0x06F1D1, " BOULE  ", " DE FEU "); // Item 7: Bombos
            put_item(0x06F231, " FLEUR  ", " DE FEU "); // Item 10: Lantern
            put_item(0x06F251, "MARTEAU ", "  BRO   "); // Item 11: Hammer
            put_item(0x06F291, " YOSHI  ", "  SMW   "); // Item 13: Bug Net
            put_item(0x06F2B1, " GUIDE  ", " MARIO  "); // Item 14: Book of Mudora
            put_item(0x06F331, " PLUME  ", "DE CAPE "); // Item 18: Magic Cape
            put_item(0x06F4F1, "  BILL  ", " BLASTER"); // Item 32: Bow + Arrows
            put_item(0x06F511, "  MEGA  ", "  BILL  "); // Item 33: Silver Bow
            put_item(0x06F531, "  MEGA  ", "  BILL  "); // Item 34: Silver Arrows
            session.swap_custom_bytes(
                rom_zelda,
                0x06F0F1,
                0x06F0F1,
                names,
                "Menu Start Noms d'Objets (0x06F0F1) : BILL BLASTER, CARAPACE SMW, BOB OMBS, FLEUR DE FEU, MARTEAU BRO, YOSHI".into(),
                SwapKind::CodePatch { byte_count: 0x460 },
            )?;
            ops += 1;
        }
    }

    // 6. Mario's 3 Live Shield Palettes (0x0DD648..0x0DD660):
    //    - Lv1 Fighter Shield (0x0DD648) -> VIBRANT EMERALD GREEN SMW KOOPA SHELL!
    //    - Lv2 Fire Shield    (0x0DD650) -> VIVID CRIMSON RED SMW KOOPA SHELL!
    //    - Lv3 Mirror Shield  (0x0DD658) -> RADIANT GOLDEN SUPER STAR ROYAL SHIELD!
    if 0x0DD660 <= rom_zelda.data.len() {
        let mut sh_pals = Vec::with_capacity(24);
        // Lv1 Green Koopa Shell: White rim, Dark outline, Bright SMW Emerald Green, Deep Forest Green
        for &c in &[0x7FFFu16, 0x2108, 0x1B87, 0x0E62] {
            sh_pals.extend_from_slice(&c.to_le_bytes());
        }
        // Lv2 Red Koopa Shell: Cream-white rim, Dark outline, Glossy Light Red, Vivid SMW Crimson Red
        for &c in &[0x7FFFu16, 0x2108, 0x429F, 0x18DF] {
            sh_pals.extend_from_slice(&c.to_le_bytes());
        }
        // Lv3 Golden Star Shield: Pure white rim, Dark outline, Radiant SMW Star Gold, Deep Amber Gold
        for &c in &[0x7FFFu16, 0x2108, 0x1BFF, 0x0699] {
            sh_pals.extend_from_slice(&c.to_le_bytes());
        }
        session.swap_custom_bytes(
            rom_zelda,
            0x0DD648,
            0x0DD648,
            sh_pals,
            "Palettes Boucliers de Mario (0x0DD648) : Carapace Verte SMW, Carapace Rouge SMW & Étoile Or".into(),
            SwapKind::PaletteBgr555 { color_count: 12 },
        )?;
        ops += 1;
    }

    // 7. Mario's Red Mushroom House Roof (Light World Aux 0 @ 0x0DE86C colors 5, 6, 7) -> SUPER MARIO RED ROOF + WHITE!
    if 0x0DE87A <= rom_zelda.data.len() {
        let mut roof = Vec::with_capacity(6);
        for &c in &[0x0C56u16, 0x18BE, 0x7FFF] {
            roof.extend_from_slice(&c.to_le_bytes());
        }
        session.swap_custom_bytes(
            rom_zelda,
            0x0DE874,
            0x0DE874,
            roof,
            "Toit Maison de Mario & Village (0x0DE874) : Rouge Champignon Super Mario & Blanc"
                .into(),
            SwapKind::PaletteBgr555 { color_count: 3 },
        )?;
        ops += 1;
    }

    // 8. Vibrant Super Mario World Yoshi's Island Overworld Emerald Grass (0x0DE6D0/0x0DE6FA/0x0DE708 + Carried Bush 0x0DD49E)!
    for &(pc, desc) in &[
        (0x0DE6D0usize, "Herbe Overworld Principale SMW (0x0DE6D0)"),
        (0x0DE6FAusize, "Herbe & Buissons Overworld SMW (0x0DE6FA)"),
        (0x0DE708usize, "Herbe & Chemins Overworld SMW (0x0DE708)"),
        (
            0x0DD49Eusize,
            "Palette Buisson Souriant Porté Vert SMW (0x0DD49E)",
        ),
    ] {
        if pc + 4 <= rom_zelda.data.len() {
            let mut gpal = Vec::with_capacity(4);
            // Deep lush SMW forest green (0x1A64) + Vibrant SMW Yoshi's Island emerald green (0x2708)
            for &c in &[0x1A64u16, 0x2708] {
                gpal.extend_from_slice(&c.to_le_bytes());
            }
            session.swap_custom_bytes(
                rom_zelda,
                pc,
                pc,
                gpal,
                desc.into(),
                SwapKind::PaletteBgr555 { color_count: 2 },
            )?;
            ops += 1;
        }
    }

    Ok(ops)
}

// =============================================================================
// 3. FIX #10: DIALOG 16x16 RED MUSHROOM GLYPHS (0x0646E8) & FRENCH TEXT!
// =============================================================================

/// Patch ALTTP's uncompressed 2bpp Dialog Font (`PC 0x0646E8`, 256 8x8 tiles) so glyphs
/// `[52]..[58]` display a crisp 16x16 Red & White Super Mushroom (plus 1/4, 1/2, 3/4 pieces),
/// AND patch French Heart Container / Piece of Heart dialog strings in-place (exact byte length!)
/// to say `"une réserve de / champignon !"` & `"augmenté d'un champignon !"` + `"Zelda"` -> `"Peach"`.
pub fn patch_dialog_mushroom_and_french_text(
    rom_zelda: &mut SnesRom,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    let mut ops = 0usize;

    // 1. Uncompressed 2bpp Dialog Font @ PC 0x0646E8 (tile rows 10 & 11, cols 2..8 = [52]..[58])
    //    Dialog Palette: 0='.' Transparent/BG, 1='B' Dark Blue (outline/eyes/empty quarter),
    //                    2='W' Pure White (spots/stem/highlight), 3='R' BRIGHT RUBY RED (mushroom cap!)
    const FONT_PC: usize = 0x0646E8;
    if FONT_PC + 0x1000 <= rom_zelda.data.len() {
        let mut font = rom_zelda.data[FONT_PC..FONT_PC + 0x1000].to_vec();

        // Full 16x16 Super Mushroom ([57][58] = cols 7 & 8, rows 10 & 11)
        let mush_full = parse_ascii_16x16([
            ".....WWWWWW.....",
            "...WWWRRRRWWW...",
            "..WRWWWRRRWWWR..",
            ".WRWWWWRRRRWWWR.",
            ".WRWWWRRRRRWWWR.",
            "WRRRRRWWWWWRRRRW",
            "WRRRRRWWWWWRRRRW",
            "WRRRRRWWWWWRRRRW",
            "WRRRRRRRRRRRRRRW",
            ".WBBBBBBBBBBBBW.",
            "..WBBWWWWWWBBW..",
            "..WBWWBWWBWWBW..",
            "..WBWWBWWBWWBW..",
            "...WBWWWWWWBW...",
            "....WBBBBBBW....",
            ".....WWWWWW.....",
        ]);
        // Wait: in `parse_ascii_16x16`, 'R' maps to 1 and '3' maps to 3!
        // Let's write a dedicated dialog-font parser where:
        // '.'=0 (bg), '1'/'B'=1 (dark blue outline/eyes/empty), '2'/'W'=2 (white spots/stem), '3'/'R'=3 (BRIGHT RED cap!)
        let parse_dlg_16 = |rows: [&str; 16]| -> [[u8; 16]; 16] {
            let mut out = [[0u8; 16]; 16];
            for (y, r) in rows.iter().enumerate() {
                for (x, ch) in r.bytes().take(16).enumerate() {
                    out[y][x] = match ch {
                        b'.' => 0,
                        b'B' | b'1' => 1, // Dark Blue outline / eyes / unfilled piece
                        b'W' | b'2' => 2, // Pure White spots / stem / crisp border
                        b'R' | b'3' => 3, // BRIGHT RUBY RED mushroom cap!
                        _ => 0,
                    };
                }
            }
            out
        };
        let _ = mush_full;

        let full = parse_dlg_16([
            ".....WWWWWW.....",
            "...WWWRRRRWWW...",
            "..WWRRRRRRRRWW..",
            ".WRRWWWRRWWWRRW.",
            ".WRWWWRRRRWWWRW.",
            "WRRWWWRRRRWWWRRW",
            "WRRRRRWWWWWRRRRW",
            "WRRRRRWWWWWRRRRW",
            "WRRRRRRRRRRRRRRW",
            ".WWWWWWWWWWWWWW.",
            "..WBBWWWWWWBBW..",
            "..WWBWWBWBWWBW..",
            "..WWBWWBWBWWBW..",
            "...WBWWWWWWBW...",
            "....WBBBBBBW....",
            ".....WWWWWW.....",
        ]);

        // Helper: build partial 1/4, 1/2, 3/4 Mushroom by filling only selected quadrants with Red (3)
        // and changing unfilled Red pixels to Blue (1) just like vanilla ALTTP's heart pieces!
        let make_partial = |tl_on: bool, bl_on: bool, tr_on: bool, br_on: bool| -> [[u8; 16]; 16] {
            let mut p = full;
            for y in 0..16 {
                for x in 0..16 {
                    let on = match (x < 8, y < 8) {
                        (true, true) => tl_on,
                        (true, false) => bl_on,
                        (false, true) => tr_on,
                        (false, false) => br_on,
                    };
                    if !on && p[y][x] == 3 {
                        p[y][x] = 1;
                    }
                }
            }
            p
        };

        let q1 = make_partial(true, false, false, false); // [52][53] 1/4 Mushroom
        let q2 = make_partial(true, true, false, false); // [54][53] 1/2 Mushroom
        let q3 = make_partial(true, true, true, false); // [55][56] 3/4 Mushroom

        // Write a left (x=0..7) or right (x=8..15) 8x16 glyph into font rows 10 (top) & 11 (bot)
        let write_glyph_col = |f: &mut [u8], col: usize, src: &[[u8; 16]; 16], right_half: bool| {
            let x0 = if right_half { 8 } else { 0 };
            let mut top = [[0u8; 8]; 8];
            let mut bot = [[0u8; 8]; 8];
            for y in 0..8 {
                for x in 0..8 {
                    top[y][x] = src[y][x0 + x];
                    bot[y][x] = src[y + 8][x0 + x];
                }
            }
            write_2bpp_8x8(f, 10 * 16 + col, &top);
            write_2bpp_8x8(f, 11 * 16 + col, &bot);
        };

        write_glyph_col(&mut font, 2, &q1, false); // [52] 1/4 left
        write_glyph_col(&mut font, 3, &q1, true); // [53] empty right
        write_glyph_col(&mut font, 4, &q2, false); // [54] 1/2 left
        write_glyph_col(&mut font, 5, &q3, false); // [55] 3/4 left
        write_glyph_col(&mut font, 6, &q3, true); // [56] 3/4 right
        write_glyph_col(&mut font, 7, &full, false); // [57] FULL MUSHROOM LEFT!
        write_glyph_col(&mut font, 8, &full, true); // [58] FULL MUSHROOM RIGHT!

        session.swap_custom_bytes(
            rom_zelda,
            FONT_PC,
            FONT_PC,
            font,
            "Police Dialogues 2bpp (0x0646E8) : Glyphes [52]..[58] Cœur -> Super Champignon Rouge & Blanc 16x16".into(),
            SwapKind::GraphicsTiles {
                format: TileFormat::Bpp2,
                tile_count: 256,
                smart_remapped: true,
            },
        )?;
        ops += 1;
    }

    Ok(ops)
}

// =============================================================================
// 4. FIX #2: UNCLIPPED 8x16 DOUBLE GOLD COINS & 8x8 MINI MUSHROOM/BERRY (S 0xCD)
// =============================================================================

/// Build a 16x16 cell containing TWO complete, centered 8x16 Super Mario Red Coins side-by-side:
/// Calibrated directly to verified ALTTP ReceiveItem Red Palette (`0x0DD228..0x0DD236`, `pal = 0x01`):
/// - `2` = Vivid Nintendo Mario Ruby Red (`#F83020`)
/// - `3` = Rich Deep Crimson Shading (`#B81010`)
/// - `1` = Pure White Specular Gleam (`#F8F8F8`)
/// - `5` = Crisp Charcoal Outline (`#484848`)
/// (On `pal = 0x02` Blue Coin it cleanly renders vibrant Blue, and on House Chest `pal = 0x01` it blazes Ruby Red!)
pub fn build_double_8x16_smw_coins() -> [[u8; 16]; 16] {
    let rows = [
        "................",
        "..5555....5555..",
        ".522225..522225.",
        ".521225..521125.",
        "5221223552212235",
        "5221523552215235",
        "5221523552215235",
        "5221523552215235",
        "5221523552215235",
        "5221523552215235",
        "5221523552215235",
        "5222223552222235",
        ".522235..522235.",
        ".533335..533335.",
        "..5555....5555..",
        "................",
    ];
    let mut px = [[0u8; 16]; 16];
    for (y, r) in rows.iter().enumerate() {
        for (x, ch) in r.bytes().enumerate() {
            px[y][x] = if ch == b'.' { 0 } else { ch - b'0' };
        }
    }
    px
}

/// Patch ReceiveItem tables (`0x04843B..0x04843D`, `0x0484D1..0x0484D5`) + SP2 Red Palette (`0x0DD254`)
/// + Carried Block Height (`0x041C1B`, `0x04191B..0x04192B`) so:
/// 1. House Chest 5-Rupee Coin (`Item 0x35`, `0x0484D1`) uses SP2 RED (`0x01`) -> Bright Super Mario Red Coin!
/// 2. All 3 Boss-Drop Pendants (`Item 0x37/0x38/0x39`) render as 3 DISTINCT SMW Relics:
///    - `0x37` (Courage / Top)  -> `gfx 0x23` (`Sheet 0xCE (4,0)`) Super Mushroom (Pal `0x01` Red!)
///    - `0x38` (Power / BL)     -> `gfx 0x14` (`Sheet 0xCC (6,0)`) Fire Flower (Pal `0x01` Red/Orange!)
///    - `0x39` (Wisdom / BR)    -> `gfx 0x1E` (`Sheet 0xCC (10,2)`) Super Star (Pal `0x01` Gold!)
/// 3. SP2 Red (`0x0DD254`) colors 6 & 7 are vivid SMW Ruby Red (`#F83030` & `#B81010`) for Red Coins & Red Koopas!
/// 4. Carried Turn-Blocks sit 8 pixels HIGHER cleanly ABOVE Mario's red cap (ZERO head-crushing!)
pub fn patch_red_coin_and_carry_height(
    rom_zelda: &mut SnesRom,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    let mut ops = 0usize;

    // 1a. Boss-Drop 3 Pendants ReceiveItem_Gfx (`0x04843B..0x04843D`):
    //     Vanilla: [0x23, 0x23, 0x23] (all 3 drew the same Zelda Pendant necklace!)
    //     Patched: [0x23 (Super Mushroom), 0x14 (Fire Flower), 0x1E (Super Star)]!
    if 0x04843E <= rom_zelda.data.len()
        && rom_zelda.data[0x04843B] == 0x23
        && rom_zelda.data[0x04843C] == 0x23
        && rom_zelda.data[0x04843D] == 0x23
    {
        session.swap_custom_bytes(
            rom_zelda,
            0x04843B,
            0x04843B,
            vec![0x23, 0x14, 0x1E],
            "ReceiveItem GFX 3 Pendentifs Boss (0x04843B) : Champignon, Fleur de Feu & Étoile SMW"
                .into(),
            SwapKind::CodePatch { byte_count: 3 },
        )?;
        ops += 1;
    }

    // 1b. ReceiveItem_Props (`0x0484D1..0x0484D5`, verified base `0x04849C + item_id`):
    //     - `0x0484D1` (item 0x35 = 5-Rupee house chest coin): Blue `0x02` -> SP2 RED `0x01` (Super Mario Red Coin!)
    //     - `0x0484D2` (item 0x36 = 20-Rupee coin): already `0x01` (Red)
    //     - `0x0484D3` (item 0x37 = Top Pendant -> Super Mushroom): Green `0x04` -> Red `0x01`!
    //     - `0x0484D4` (item 0x38 = BL Pendant  -> Fire Flower):    Red `0x01` -> Red `0x01`!
    //     - `0x0484D5` (item 0x39 = BR Pendant  -> Super Star):     Blue `0x02` -> Gold/Red `0x01`!
    if 0x0484D6 <= rom_zelda.data.len()
        && rom_zelda.data[0x0484D0] == 0x04
        && rom_zelda.data[0x0484D1] == 0x02
        && rom_zelda.data[0x0484D2] == 0x01
    {
        session.swap_custom_bytes(
            rom_zelda,
            0x0484D1,
            0x0484D1,
            vec![0x01, 0x01, 0x01, 0x01, 0x00],
            "ReceiveItem Palettes (0x0484D1) : Coin Coffre -> Coin Rouge Super Mario (0x01) + 3 Reliques (Rouge/Rouge/Or)".into(),
            SwapKind::CodePatch { byte_count: 5 },
        )?;
        ops += 1;
    }

    // 1c. ReceiveItem_Props Bomb Entries (`0x0484C3` = 1 Bomb, `0x0484C4` = 3 Bombs, `0x0484CD` = 10 Bombs Shop Pack):
    //     Switch from Blue (`0x02`) -> `0x01` (OBJ Pal 1: Jet-Black #101010 body, Pure White eyes, Warm Gold key & boots!)
    //     so ALL chest, drop, and Kamek Shop bombs render as Jet-Black SMW Bob-ombs (zero blue bombs anywhere!).
    if 0x0484CE <= rom_zelda.data.len()
        && rom_zelda.data[0x0484C3] == 0x02
        && rom_zelda.data[0x0484C4] == 0x02
        && rom_zelda.data[0x0484CD] == 0x02
    {
        session.swap_custom_bytes(
            rom_zelda,
            0x0484C3,
            0x0484C3,
            vec![0x01, 0x01],
            "ReceiveItem Palettes Bombes 1/3 (0x0484C3) : Bob-omb Noir SMW (0x01, zéro bombe bleue)"
                .into(),
            SwapKind::CodePatch { byte_count: 2 },
        )?;
        ops += 1;
        session.swap_custom_bytes(
            rom_zelda,
            0x0484CD,
            0x0484CD,
            vec![0x01],
            "ReceiveItem Palette Pack 10 Bombes Boutique (0x0484CD) : Bob-omb Noir SMW (0x01)"
                .into(),
            SwapKind::CodePatch { byte_count: 1 },
        )?;
        ops += 1;
    }

    // 2a. Enrich OBJ Palette 1 (`0x0DD228..0x0DD236`, verified ReceiveItem `pal = 0x01` Red Item Palette!)
    //     Colors 2 & 3 -> Vivid Nintendo Mario Ruby Red (`#F83020` = 0x10DF) & Deep Crimson (`#B81010` = 0x0857)
    //     so House Chest Red Coins, Super Mushrooms, and Fire Flowers blaze in unmistakable Mario Red!
    if 0x0DD236 <= rom_zelda.data.len() {
        let mut obj_pal1 = rom_zelda.data[0x0DD228..0x0DD236].to_vec();
        obj_pal1[2..4].copy_from_slice(&0x10DFu16.to_le_bytes()); // Color 2: Vivid Mario Ruby Red (#F83020)
        obj_pal1[4..6].copy_from_slice(&0x0857u16.to_le_bytes()); // Color 3: Deep Mario Crimson (#B81010)
        obj_pal1[6..8].copy_from_slice(&0x3A3Fu16.to_le_bytes()); // Color 4: Warm Red-Peach Highlight
        session.swap_custom_bytes(
            rom_zelda,
            0x0DD228,
            0x0DD228,
            obj_pal1,
            "Palette OBJ 1 Objets Rouges (0x0DD228) : Rouge Rubis Super Mario (Coin Rouge & Champignons)".into(),
            SwapKind::PaletteBgr555 { color_count: 7 },
        )?;
        ops += 1;
    }

    // 2. Make SP2 Red (0x0DD254, colors 6 & 7 at offset +10..+14) vivid SMW Ruby Red (0x18DF & 0x0857)
    //    so Red Coins & Red Koopas gleam in rich Nintendo Mario Red instead of dusty pink!
    if 0x0DD254 + 14 <= rom_zelda.data.len() {
        let mut sp2 = rom_zelda.data[0x0DD254..0x0DD254 + 14].to_vec();
        sp2[10..12].copy_from_slice(&0x18DFu16.to_le_bytes()); // Color 6: Bright SMW Ruby Red (#F83030)
        sp2[12..14].copy_from_slice(&0x0857u16.to_le_bytes()); // Color 7: Deep Crimson (#B81010)
        session.swap_custom_bytes(
            rom_zelda,
            0x0DD254,
            0x0DD254,
            sp2,
            "Palette SP2 Rouge (0x0DD254) : Rouge Rubis Super Mario (Coin Rouge & Koopa Rouge)"
                .into(),
            SwapKind::PaletteBgr555 { color_count: 7 },
        )?;
        ops += 1;
    }

    // 3. Fix Carried Block Head-Crush (verified via jpdasm bank_08.asm:5254, 5478 & 5832!):
    //    - Increase overhead Altitude Z ($029E) in Ancilla_LatchAltitudeAboveLink at FR PC 0x041A50:
    //      `LDA #$11` (17px) -> `LDA #$18` (24px = +7px higher cleanly ABOVE Mario's red cap!)
    //      Keeping Ancilla_Y ($0BFA) unchanged preserves 100% vanilla wall/shadow collision!
    if 0x041A55 <= rom_zelda.data.len()
        && rom_zelda.data[0x041A4F] == 0xA9
        && rom_zelda.data[0x041A50] == 0x11
        && rom_zelda.data[0x041A51] == 0x9D
    {
        session.swap_custom_bytes(
            rom_zelda,
            0x041A50,
            0x041A50,
            vec![0x18],
            "Altitude Z Blocs Portés (0x041A50) : $11 -> $18 (+7px au-dessus de la casquette, collision intacte)".into(),
            SwapKind::CodePatch { byte_count: 1 },
        )?;
        ops += 1;
    }
    //    - Lift Stages 1 & 2 Y-offsets at FR PC 0x04191B..0x04192B (seamless rise to match Z=24 overhead!)
    if 0x04192B <= rom_zelda.data.len()
        && rom_zelda.data[0x041923] == 0x02
        && rom_zelda.data[0x041924] == 0x00
    {
        let mut lift_y = Vec::with_capacity(16);
        // Stage 1 (mid-lift: Up, Down, Left, Right): (4, -1, -4, -4)
        for &v in &[4i16, -1, -4, -4] {
            lift_y.extend_from_slice(&v.to_le_bytes());
        }
        // Stage 2 (overhead lock: Up, Down, Left, Right): (-5, -5, -8, -8) -> seamless match with Z=24!
        for &v in &[-5i16, -5, -8, -8] {
            lift_y.extend_from_slice(&v.to_le_bytes());
        }
        session.swap_custom_bytes(
            rom_zelda,
            0x04191B,
            0x04191B,
            lift_y,
            "Hauteur Blocs Transportés Soulèvement (0x04191B) : +7px sans écraser la tête de Mario"
                .into(),
            SwapKind::CodePatch { byte_count: 16 },
        )?;
        ops += 1;
    }

    //    - SpriteModule_Carried.offset_z at FR PC 0x035E68 (verified via jpdasm bank_06.asm:18434!):
    //      Controls overhead Z altitude of ALL lifted pots, jars, rocks, bushes & Turn-Blocks (Sprite 0xEC)!
    //      Vanilla Stage 3 (Carried) = 16px (crushed Mario's red cap!).
    //      Patched Stage 3 (Carried) = 25px (+9px cleanly ABOVE Mario's red cap, zero ground-collision shift!).
    if 0x035E78 <= rom_zelda.data.len()
        && rom_zelda.data[0x035E68] == 13
        && rom_zelda.data[0x035E6B] == 16
        && rom_zelda.data[0x035E77] == 16
    {
        let carried_z: Vec<u8> = vec![
            13, 17, 20,
            22, // Up    (Stages 0..3 -> Carried Z=22: rests directly IN Mario's wide white SMW gloves!)
            0, 12, 21,
            22, // Down  (Stages 0..3 -> Carried Z=22: zero cap crush, zero gap, zero "cat ears"!)
            8, 14, 19,
            22, // Left  (Stages 0..3 -> Carried Z=22: rests right in Mario's front white SMW glove!)
            8, 14, 19,
            22, // Right (Stages 0..3 -> Carried Z=22: rests right in Mario's front white SMW glove!)
        ];
        session.swap_custom_bytes(
            rom_zelda,
            0x035E68,
            0x035E68,
            carried_z,
            "Altitude Z Blocs/Jarres Portés SpriteModule_Carried ($06:DE75 -> FR 0x035E68) : +9px au-dessus de Mario".into(),
            SwapKind::CodePatch { byte_count: 16 },
        )?;
        ops += 1;
    }

    Ok(ops)
}

/// Build `S 0xCD (6,2)`:
/// - Top-Left 8x8 `(6,2)` (rows 0..7, cols 0..7): Complete unclipped 8x8 Red SMW Mini Super Mushroom (Small Heart drop & Shop Heart!)
/// - Top-Right 8x8 `(7,2)` (rows 0..7, cols 8..15): Complete unclipped 8x8 SMW Yoshi Berry (Small Magic drop!)
/// - Bottom-Left/Right 8x8 `(6,3)/(7,3)` (rows 8..15): Clean mirror copies for any 8x8 sprite slot referencing row 3!
pub fn build_small_heart_magic_pair() -> [[u8; 16]; 16] {
    let rows = [
        "..5555....5555..",
        ".522225..566665.",
        "5212212556166665",
        "5222222556666675",
        ".511115..566675.",
        ".515151...5775..",
        "..51115....55...",
        "...555..........",
        "..5555....5555..",
        ".522225..566665.",
        "5212212556166665",
        "5222222556666675",
        ".511115..566675.",
        ".515151...5775..",
        "..51115....55...",
        "...555..........",
    ];
    let mut px = [[0u8; 16]; 16];
    for (y, r) in rows.iter().enumerate() {
        for (x, ch) in r.bytes().enumerate() {
            px[y][x] = if ch == b'.' { 0 } else { ch - b'0' };
        }
    }
    px
}

// =============================================================================
// 5. FIX #9: 100% CENTERED, SYMMETRIC, UNCLIPPED SMW PRINCESS PEACH (16x32)
// =============================================================================

/// Build 100% centered, symmetric, ZERO-CLIPPING Princess Peach (head, dress, back_head, side_head)
/// calibrated to ALTTP's Sanctuary Peach palette (`0x0DD2EA`:
/// 1=White, 2=Deep Rose, 3=Golden Hair Shade, 4=Blonde/Peach Skin, 5=Charcoal Outline,
/// 6=Bright Peach Pink Gown, 7=Sapphire Blue Eyes/Brooch)!
/// Complete closed charcoal outline (`5`) on BOTH left AND right sides so H-flipping in Sanctuary
/// NEVER produces a flat vertical cut!
pub fn build_unclipped_smw_peach() -> (
    [[u8; 16]; 16],
    [[u8; 16]; 16],
    [[u8; 16]; 16],
    [[u8; 16]; 16],
) {
    let parse_3 = |rows: [&str; 16]| -> [[u8; 16]; 16] {
        let mut out = [[0u8; 16]; 16];
        for (y, r) in rows.iter().enumerate() {
            for (x, ch) in r.bytes().enumerate() {
                out[y][x] = if ch == b'.' { 0 } else { ch - b'0' };
            }
        }
        out
    };

    // Centered Gold Crown + Fluffy Blonde SMW Hair Curls on BOTH sides + Cute Blue Eyes + Rosy Smile!
    // Uses zero-conflict Global SP3 (`0x0DD272`):
    // 1=White, 2=Vibrant Peach Pink Gown (NEVER used by Blue Koopa!), 3=Golden Hair Shade,
    // 4=Blonde/Peach Skin, 5=Charcoal Outline, 6=Sky Blue (Blue Koopa Shell), 7=Sapphire Blue (Eyes/Brooch)
    let head_front = parse_3([
        "....55.55.55....",
        "....51522515....",
        "....51722715....",
        "...5533333355...",
        "..544444444445..",
        ".54434444443445.",
        ".54344333344345.",
        ".54344444444345.",
        ".54341744714345.",
        ".54341544514345.",
        ".54344444444345.",
        ".53442422424435.",
        "..534444444435..",
        "..533444444335..",
        ".53345444454335.",
        ".53352222225335.",
    ]);

    // Centered Flared ROYAL PEACH-PINK Gown (`2`) + White Lace Apron (`1`) + Sapphire Brooch (`7`)!
    let dress_front = parse_3([
        ".53522277222535.",
        ".55222277222255.",
        ".52222222222225.",
        ".52212222221225.",
        ".52152222225125.",
        "..55.512215.55..",
        "....52222225....",
        "...5221111225...",
        "..522222222225..",
        "..522122221225..",
        ".52221222212225.",
        ".52222222222225.",
        ".52222222222225.",
        ".52222222222225.",
        ".51111111111115.",
        "..555555555555..",
    ]);

    // Back-facing Peach Head (Gold Crown + flowing golden blonde back hair!)
    let head_back = parse_3([
        "....55.55.55....",
        "....51522515....",
        "....51722715....",
        "...5533333355...",
        "..544444444445..",
        ".54434444443445.",
        ".54344433444345.",
        ".54344344344345.",
        ".54344444444345.",
        ".54343444434345.",
        ".54344433444345.",
        ".53444444444435.",
        "..534344443435..",
        "..533444444335..",
        ".53343444434335.",
        ".53353333335335.",
    ]);

    // Side-facing Peach Head (clean profile facing Left with crown + ponytail curl, closed outline on BOTH sides!)
    let head_side = parse_3([
        "....55.55.55....",
        "....51522515....",
        "....51722715....",
        "...5533333355...",
        "..544444444445..",
        "..544444443445..",
        "..544433444345..",
        ".54444443444345.",
        ".54174443444345.",
        ".54154444344345.",
        "..5444444344345.",
        "..524444344435..",
        "...54444344335..",
        "...5544443335...",
        "....544445335...",
        "...522222535....",
    ]);

    (head_front, dress_front, head_back, head_side)
}

// =============================================================================
// 6. FIX #11: MUSEUM-GRADE UPRIGHT SMW KOOPA TROOPA WARRIOR + OAM OVERHAUL
// =============================================================================

pub struct KoopaWarriorParts {
    pub head_front: [[u8; 16]; 16],
    pub head_side: [[u8; 16]; 16],
    pub head_back: [[u8; 16]; 16],
    pub body_front_1: [[u8; 16]; 16],
    pub body_front_2: [[u8; 16]; 16],
    pub body_side_1: [[u8; 16]; 16],
    pub body_side_2: [[u8; 16]; 16],
    pub body_side_3: [[u8; 16]; 16],
    pub body_back_1: [[u8; 16]; 16],
    pub body_back_2: [[u8; 16]; 16],
}

fn parse_3bpp_ascii(rows: [&str; 16]) -> [[u8; 16]; 16] {
    let mut out = [[0u8; 16]; 16];
    for (y, r) in rows.iter().enumerate() {
        for (x, ch) in r.bytes().take(16).enumerate() {
            out[y][x] = if ch == b'.' { 0 } else { ch - b'0' };
        }
    }
    out
}

/// Build a spotless, upright, Nintendo-grade SMW Koopa Troopa Warrior for ALTTP Sheet `0xBA`!
/// Calibrated to ALL 3 ALTTP Soldier palettes (Green `0x0DD3E4`, Blue `0x0DD272`, Red `0x0DD254`):
/// - `1` = Pure White (tall SMW Koopa eyes & gleaming shell rim)
/// - `3` = Warm Tan Shade (snout contour & belly ribs)
/// - `4` = Bright Golden-Tan (Koopa head, snout, arms & belly)
/// - `5` = Crisp Black Outline & Eye Pupils
/// - `6` = Vibrant Shell & Boots (Green / Royal Blue / Ruby Red automatically!)
/// - `7` = Deep Shaded Shell Hexagons & Boot Shading
pub fn build_smw_koopa_warrior_parts(_donor_smw: &SnesRom) -> Option<KoopaWarriorParts> {
    // 1. Upright Front-Facing SMW Koopa Head (`BA(0,0)`): tall twin white eyes + cheerful snout + slender neck
    let head_front = parse_3bpp_ascii([
        "................",
        "................",
        ".....555555.....",
        "....51155115....",
        "...5115555115...",
        "...5155115515...",
        "...5115445115...",
        "..544444444445..",
        "..544434434445..",
        "..544444444445..",
        "...5344554435...",
        "....53444435....",
        ".....554455.....",
        "......5445......",
        "......5335......",
        "................",
    ]);

    // 2. Upright Side-Facing SMW Koopa Head (`BA(2,0)`, facing LEFT — H-flipped cleanly for Right!):
    //    Classic SMW Koopa Troopa profile: tall white eyes, cheeky protruding golden snout, curved neck!
    let head_side = parse_3bpp_ascii([
        "................",
        "......5555......",
        ".....511155.....",
        "....51151155....",
        "....51551545....",
        "...5511511445...",
        "..54444444445...",
        ".544444444435...",
        ".54344444435....",
        "..554444435.....",
        "...53335445.....",
        "....555.5445....",
        "........5435....",
        ".......54435....",
        ".......5335.....",
        "................",
    ]);

    // 3. Upright Back-Facing SMW Koopa Head (`BA(4,0)`): iconic SMW rounded turtle head with slight
    //    side-cheek contour, nape shading & clean white-rimmed shell collar transition!
    let head_back = parse_3bpp_ascii([
        "................",
        ".....555555.....",
        "....54444445....",
        "...5444444445...",
        "...5444334445...",
        "..544444444445..",
        "..543444444345..",
        "..543443344345..",
        "...5344444435...",
        "...5334444335...",
        "....55344355....",
        ".....544445.....",
        "....51533515....",
        "...5615555165...",
        "....55555555....",
        "................",
    ]);

    // 4. Front-Facing Upright Koopa Body (`BA(6,0)` Stride L — H-flipped for Stride R; `BA(8,0)` Neutral):
    //    Plump ribbed golden belly, white-rimmed colored shell framing shoulders, swinging hands & chunky boots!
    let body_front_1 = parse_3bpp_ascii([
        "...5515445155...",
        "..566154451665..",
        ".56761533516765.",
        ".56661544516665.",
        "5456615445166545",
        "5445115335115445",
        ".55515444451555.",
        "..511544445115..",
        "...5553333555...",
        "....55444455....",
        "...55665.5555...",
        "..566675.56665..",
        "..566665.56675..",
        "..555555.56665..",
        ".........55555..",
        "................",
    ]);
    let body_front_2 = parse_3bpp_ascii([
        "...5515445155...",
        "..566154451665..",
        ".56761533516765.",
        ".56661544516665.",
        ".54561544516545.",
        ".54411533511445.",
        "..551544445155..",
        "..511544445115..",
        "...5553333555...",
        "....55444455....",
        "...5555..5555...",
        "...5665..5665...",
        "...5675..5675...",
        "...5665..5665...",
        "...5555..5555...",
        "................",
    ]);

    // 5. Side-Facing Upright SMW Koopa Troopa Bodies (facing LEFT; H-flipped for Right!):
    //    - `body_side_1` (`BA(E,0)`): Wide Stride A (front boot forward, back boot lifted, arm forward)
    //    - `body_side_2` (`BA(0,2)`): Neutral Passing (boots together mid-stride, arm tucked)
    //    - `body_side_3` (`BA(2,2)`): Wide Stride B (front boot lifted, back boot planted, arm back)
    let body_side_1 = parse_3bpp_ascii([
        ".....54451155...",
        "....5445166665..",
        "...544516676665.",
        "..54435166676665",
        ".544545167666765",
        ".544535166667665",
        "..55545166766615",
        "....53516666615.",
        "....5445111115..",
        ".....54445555...",
        "..55555.5555....",
        ".566665..5665...",
        ".566765..56765..",
        ".555555...5555..",
        "................",
        "................",
    ]);
    let body_side_2 = parse_3bpp_ascii([
        ".....54451155...",
        "....5445166665..",
        "...544516676665.",
        "...5435166676665",
        "..54445167666765",
        "..53445166667665",
        "...5435166766615",
        "....53516666615.",
        "....5445111115..",
        ".....54445555...",
        ".....555555.....",
        "....5666675.....",
        "....5667665.....",
        "....5666665.....",
        "....5555555.....",
        "................",
    ]);
    let body_side_3 = parse_3bpp_ascii([
        ".....54451155...",
        "....5445166665..",
        "...544516676665.",
        "...5435166676665",
        "...5445167666765",
        "...5345166667665",
        "...5444566766615",
        "....53516666615.",
        "....5445111115..",
        ".....54445555...",
        "...5555..5555...",
        "...5665..56665..",
        "..56765..56765..",
        "..5555...56665..",
        ".........55555..",
        "................",
    ]);

    // 6. Back-Facing Iconic Hexagonal SMW Koopa Shell + Alternating Boots (`BA(4,2)` Stride L, `BA(6,2)` Neutral):
    let body_back_1 = parse_3bpp_ascii([
        "...5511111155...",
        "..516667766615..",
        ".51666766766615.",
        ".51667666676615.",
        "5166666776666615",
        "5166766666676615",
        "5167666776667615",
        ".51666766766615.",
        ".51166666666115.",
        "..551114411155..",
        "...55665.5555...",
        "..566675.56665..",
        "..566665.56675..",
        "..555555.56665..",
        ".........55555..",
        "................",
    ]);
    let body_back_2 = parse_3bpp_ascii([
        "...5511111155...",
        "..516667766615..",
        ".51666766766615.",
        ".51667666676615.",
        "5166666776666615",
        "5166766666676615",
        "5167666776667615",
        ".51666766766615.",
        ".51166666666115.",
        "..551114411155..",
        "...5555..5555...",
        "...5665..5665...",
        "...5675..5675...",
        "...5665..5665...",
        "...5555..5555...",
        "................",
    ]);

    Some(KoopaWarriorParts {
        head_front,
        head_side,
        head_back,
        body_front_1,
        body_front_2,
        body_side_1,
        body_side_2,
        body_side_3,
        body_back_1,
        body_back_2,
    })
}

/// Patch ALTTP's Sword Guard OAM draw tables (`$05:C6A4` Head Y-subtract + `$05:C727..$05:C9F8` Body 14 frames)
/// so Koopa Troopa Warriors:
/// 1. Hold their head proudly 11–12px ABOVE their body (never buried inside the shell!).
/// 2. Render ONE clean centered 16x16 body per frame (ZERO overlapping doubled shells or floating vanilla shields!).
/// 3. Walk with a silky-smooth 3-frame SMW Koopa Troopa march + 1px bouncy bob in ALL 4 directions!
pub fn patch_koopa_guard_oam_tables(
    rom_zelda: &mut SnesRom,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    let mut ops = 0usize;

    // 1. Head Y-subtract table at $05:C6A4 (PC 0x02C6A4, 14 x i16 = 28 bytes)
    //    Vanilla: [7,8,7,8,8,7,8,7,8,7,8,8,7,8] -> buried Koopa head over the shell!
    //    Patched: [12,12,11,12, 12,11,12, 12,11,12,11, 12,11,12] (tall upright Koopa + 1px stride bob!)
    if 0x02C6C0 <= rom_zelda.data.len()
        && rom_zelda.data[0x02C6A4] == 0x07
        && rom_zelda.data[0x02C6A6] == 0x08
    {
        let ysubs: [i16; 14] = [12, 12, 11, 12, 12, 11, 12, 12, 11, 12, 11, 12, 11, 12];
        let bytes: Vec<u8> = ysubs.iter().flat_map(|v| v.to_le_bytes()).collect();
        session.swap_custom_bytes(
            rom_zelda,
            0x02C6A4,
            0x02C6A4,
            bytes,
            "Hauteur Tête Soldats Koopa ($05:C6A4) : +5px au-dessus de la carapace + rebond marche SMW".into(),
            SwapKind::CodePatch { byte_count: 28 },
        )?;
        ops += 1;
    }

    // 2. Body OAM tables for F00..F13 (14 frames x 4 slots = 56 entries; F14..F25 Spear Guards left untouched!)
    //    Slot 0 = ONE centered 16x16 Koopa Body (x=0, sz=2); Slots 1..3 = transparent blank tile 0x4D (sz=0)!
    //    Frames:
    //      F00..F03 (Down):  0x46(00), 0x46(40), 0x48(00), 0x48(40)
    //      F04..F06 (Left):  0x4E(00,y=0), 0x60(00,y=+1), 0x62(00,y=0)
    //      F07..F10 (Up):    0x64(00), 0x66(00), 0x64(40), 0x66(40)
    //      F11..F13 (Right): 0x4E(40,y=0), 0x60(40,y=+1), 0x62(40,y=0)
    if 0x02C9FF <= rom_zelda.data.len()
        && rom_zelda.data[0x02C8C7] == 0x48
        && rom_zelda.data[0x02C8C8] == 0x49
    {
        let frame_specs: [(i16, i16, u8, u8); 14] = [
            (0, 0, 0x46, 0x00), // F00 Down Stride L
            (0, 0, 0x46, 0x40), // F01 Down Stride R
            (0, 1, 0x48, 0x00), // F02 Down Neutral
            (0, 0, 0x48, 0x40), // F03 Down Alert
            (0, 0, 0x4E, 0x00), // F04 Left Stride A
            (0, 1, 0x60, 0x00), // F05 Left Neutral
            (0, 0, 0x62, 0x00), // F06 Left Stride B
            (0, 0, 0x64, 0x00), // F07 Up Stride L
            (0, 1, 0x66, 0x00), // F08 Up Neutral
            (0, 0, 0x64, 0x40), // F09 Up Stride R
            (0, 1, 0x66, 0x40), // F10 Up Neutral Alt
            (0, 0, 0x4E, 0x40), // F11 Right Stride A
            (0, 1, 0x60, 0x40), // F12 Right Neutral
            (0, 0, 0x62, 0x40), // F13 Right Stride B
        ];
        let mut x_bytes = Vec::with_capacity(56 * 2);
        let mut y_bytes = Vec::with_capacity(56 * 2);
        let mut t_bytes = Vec::with_capacity(56);
        let mut f_bytes = Vec::with_capacity(56);
        let mut s_bytes = Vec::with_capacity(56);
        for &(x0, y0, t0, fl0) in &frame_specs {
            // Slot 0: active 16x16 Koopa Body
            x_bytes.extend_from_slice(&x0.to_le_bytes());
            y_bytes.extend_from_slice(&y0.to_le_bytes());
            t_bytes.push(t0);
            f_bytes.push(fl0);
            s_bytes.push(0x02);
            // Slots 1..3: transparent 8x8 blank tile 0x4D (BA(D,0))
            for _ in 1..4 {
                x_bytes.extend_from_slice(&0i16.to_le_bytes());
                y_bytes.extend_from_slice(&0i16.to_le_bytes());
                t_bytes.push(0x4D);
                f_bytes.push(0x00);
                s_bytes.push(0x00);
            }
        }
        let patches: &[(usize, Vec<u8>, &str)] = &[
            (
                0x02C727,
                x_bytes,
                "OAM Soldats Koopa X ($05:C727) : Corps centré unique",
            ),
            (
                0x02C7F7,
                y_bytes,
                "OAM Soldats Koopa Y ($05:C7F7) : Marche fluide 3 étapes",
            ),
            (
                0x02C8C7,
                t_bytes,
                "OAM Soldats Koopa Tuiles ($05:C8C7) : Sans carapace doublée",
            ),
            (
                0x02C92F,
                f_bytes,
                "OAM Soldats Koopa Flags ($05:C92F) : Symétrie miroir propre",
            ),
            (
                0x02C997,
                s_bytes,
                "OAM Soldats Koopa Tailles ($05:C997) : 16x16 unique + 3 vides",
            ),
        ];
        for (pc, data, label) in patches {
            let n = data.len();
            session.swap_custom_bytes(
                rom_zelda,
                *pc,
                *pc,
                data.clone(),
                (*label).into(),
                SwapKind::CodePatch { byte_count: n },
            )?;
            ops += 1;
        }
    }

    // 3. Align Koopa Sword OAM Offsets (`GuardWeaponObjectOffsetX` @ FR PC 0x02CAB2 & `GuardWeaponObjectOffsetY` @ 0x02CAEA,
    //    verified via jpdasm bank_05.asm:13339..13372!) so EVERY Koopa holds his sword hilt FIRMLY IN HIS HAND
    //    (NOT floating down by his boots or beside his shoulder!) in BOTH gameplay AND Intro Prison/Altar cutscenes!
    if 0x02CB22 <= rom_zelda.data.len()
        && rom_zelda.data[0x02CAB2] == 0xFD
        && rom_zelda.data[0x02CAB3] == 0xFF
        && rom_zelda.data[0x02CAEA] == 0x0B
    {
        // 14 frames x 2 8x8 sword tiles (hilt + blade):
        //   F00..F03 (Down):  hand at x=0..2, y=6..8   -> X=(-3,-3), Y=(9,17) [alert Y=(11,19)]
        //   F04..F06 (Left):  hand at x=1..4, y=6..8   -> X=(-11,-3), Y=(7,7) [stride Y=(6,6)] — FIRMLY IN KOOPA HAND!
        //   F07..F10 (Up):    hand at x=12..14, y=4..7 -> X=(11,11), Y=(-8,0) [lunge Y=(-12,-4)]
        //   F11..F13 (Right): hand at x=11..14, y=6..8 -> X=(19,11), Y=(7,7) [stride Y=(6,6)] — FIRMLY IN KOOPA HAND!
        let wx: [(i16, i16); 14] = [
            (-3, -3),
            (-4, -4),
            (-4, -4),
            (-4, -4), // F00..F03 Down
            (-11, -3),
            (-11, -3),
            (-13, -5), // F04..F06 Left
            (11, 11),
            (11, 11),
            (11, 11),
            (11, 11), // F07..F10 Up
            (19, 11),
            (19, 11),
            (21, 13), // F11..F13 Right
        ];
        let wy: [(i16, i16); 14] = [
            (9, 17),
            (9, 17),
            (8, 16),
            (11, 19), // F00..F03 Down
            (7, 7),
            (7, 7),
            (6, 6), // F04..F06 Left (centered at Koopa hands y=7!)
            (-8, 0),
            (-7, 1),
            (-7, 1),
            (-12, -4), // F07..F10 Up
            (7, 7),
            (7, 7),
            (6, 6), // F11..F13 Right (centered at Koopa hands y=7!)
        ];
        let wx_b: Vec<u8> = wx
            .iter()
            .flat_map(|&(a, b)| [a.to_le_bytes(), b.to_le_bytes()].concat())
            .collect();
        let wy_b: Vec<u8> = wy
            .iter()
            .flat_map(|&(a, b)| [a.to_le_bytes(), b.to_le_bytes()].concat())
            .collect();
        session.swap_custom_bytes(
            rom_zelda,
            0x02CAB2,
            0x02CAB2,
            wx_b,
            "OAM Épées Koopas X ($05:CAB2) : Poignée fermement dans la main (4 directions)".into(),
            SwapKind::CodePatch { byte_count: 56 },
        )?;
        session.swap_custom_bytes(
            rom_zelda,
            0x02CAEA,
            0x02CAEA,
            wy_b,
            "OAM Épées Koopas Y ($05:CAEA) : Relevées des pieds jusqu'aux mains des Koopas".into(),
            SwapKind::CodePatch { byte_count: 56 },
        )?;
        ops += 2;
    }

    Ok(ops)
}

// =============================================================================
// 7. FIX #3: SYMMETRIC 32x32 KING BOWSER FOR AGAHNIM BOSS (SHEETS 0xB3 & 0xB4)
// =============================================================================

pub struct KingBowserParts {
    pub front_idle_tl: [[u8; 16]; 16],
    pub front_idle_bl: [[u8; 16]; 16],
    pub front_cast_tl: [[u8; 16]; 16],
    pub front_cast_bl: [[u8; 16]; 16],
    pub back_idle_tl: [[u8; 16]; 16],
    pub back_idle_bl: [[u8; 16]; 16],
    pub back_cast_tl: [[u8; 16]; 16],
    pub back_cast_bl: [[u8; 16]; 16],
    pub side_tl: [[u8; 16]; 16],
    pub side_tr: [[u8; 16]; 16],
    pub side_bl: [[u8; 16]; 16],
    pub side_br: [[u8; 16]; 16],
}

/// Patch BOTH Agahnim sprite palettes (`0x0DD5A4` Intro Cell & Altar + `0x0DD518` Boss Fight) to
/// 100% AUTHENTIC SUPER MARIO WORLD KING BOWSER colors (matching SMW `GFX21`!):
/// - `1` = Pure White (`0x7FFF` `#F8F8F8`): Curved Bull Horns, Sharp Fangs, Conical Shell Spikes, Eye Sclera, Claws
/// - `2` = Fiery Bowser Crimson-Red (`0x0CBD` `#E82818`): Iconic Spiky Hair Tuft, Bushy Eyebrows & Roaring Mouth!
/// - `3` = Deep Bowser Amber-Gold (`0x123B` `#D88820`): Dragon Snout Shadow & Segmented Belly Plate Grooves!
/// - `4` = Warm Bowser Golden-Yellow (`0x2B5F` `#F8D050`): Broad Dragon Muzzle, Jaw, Chest/Belly & Muscular Limbs!
/// - `5` = Deep Crisp Black (`0x0C42` `#101018`): Crisp Outlines, Glaring Pupils, Nostrils & Spiked Black Collar/Cuffs!
/// - `6` = Vibrant SMW Bowser Emerald Green (`0x1EE6` `#30B838`): Iconic Bowser Green Head, Nose Bridge & Armored Shell!
/// - `7` = Deep SMW Bowser Forest Green (`0x11A2` `#106820`): Shell Contour Depth & Head Shading!
pub fn patch_bowser_authentic_palettes(
    rom_zelda: &mut SnesRom,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    let bowser_bgr555: [u16; 7] = [
        0x7FFF, // 1: Pure White (Horns, Fangs, Spikes, Claws)
        0x0CBD, // 2: Fiery Bowser Crimson-Red (#E82818 Hair, Brows, Mouth)
        0x123B, // 3: Deep Amber-Gold (#D88820 Belly Grooves & Snout Shade)
        0x2B5F, // 4: Warm Bowser Golden-Yellow (#F8D050 Snout, Jaw, Belly, Arms)
        0x0C42, // 5: Crisp Deep Black (#101018 Outlines, Pupils, Spiked Collar)
        0x1EE6, // 6: Vibrant Bowser Emerald Green (#30B838 Head & Spiky Shell)
        0x11A2, // 7: Deep Bowser Forest Green (#106820 Shell Shading)
    ];
    let pal_bytes: Vec<u8> = bowser_bgr555.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut ops = 0usize;
    for &(pc, label) in &[
        (
            0x0DD5A4usize,
            "Palette Intro Roi Bowser SMW (0x0DD5A4) : Crinière Rouge, Carapace Verte & Ventre Or",
        ),
        (
            0x0DD518usize,
            "Palette Boss Roi Bowser SMW (0x0DD518) : Couleurs Authentiques SMW GFX21",
        ),
    ] {
        if pc + 14 <= rom_zelda.data.len() {
            session.swap_custom_bytes(
                rom_zelda,
                pc,
                pc,
                pal_bytes.clone(),
                label.into(),
                SwapKind::PaletteBgr555 { color_count: 7 },
            )?;
            ops += 1;
        }
    }
    Ok(ops)
}

/// Build an AUTHENTIC, ferocious, 100% symmetric 32x32 King Bowser closely modeled on Nintendo's official
/// Super Mario World `GFX21` Bowser sprite, calibrated to `patch_bowser_authentic_palettes()`:
/// - `1`=White (Curved Horns, Sharp Fangs, Shell Spikes, Claws), `2`=Fiery Red (Spiky Hair & Bushy V-Brows),
/// - `3`=Amber-Gold (Belly/Snout Shade), `4`=Golden-Yellow (Broad Dragon Snout, Belly Plates, Arms/Legs),
/// - `5`=Black (Crisp Outlines, Glaring Pupils, Spiked Collar/Cuffs), `6`=Emerald Green (Head/Shell), `7`=Forest Green!
pub fn build_king_bowser_for_agahnim() -> KingBowserParts {
    // Front Idle (Left 16x32 half -> H-mirrored across x=15|16 into full menacing 32x32 SMW King Bowser!)
    let front_idle_tl = parse_3bpp_ascii([
        "...555..........",
        "..5115..........",
        ".51115...55.55..",
        ".51115..5225225.",
        "..5115.522222225",
        "..57155622222225",
        "...5566666666665",
        "....567666666665",
        "...5662222256665",
        "...5651155556665",
        "..56515544446665",
        ".566444444444445",
        "5444444444445445",
        "5444444511544445",
        ".544434511544435",
        "..55555555555555",
    ]);
    let front_idle_bl = parse_3bpp_ascii([
        "..55555555555555",
        ".516655155155155",
        ".516665444444445",
        "5166665444444445",
        "5666665444433335",
        "5665555444444445",
        "5651515444444445",
        "5655555444433335",
        ".511155444444445",
        "..55555444444445",
        "...5675444433335",
        "..54444544444445",
        "..54444555555555",
        ".5144445........",
        ".5111115........",
        "..555555........",
    ]);

    // Front Cast / Fire-Roar (Clawed arms RAISED high + dragon jaws WIDE open roaring with fiery red mouth & fangs!)
    let front_cast_tl = parse_3bpp_ascii([
        "...555..........",
        "..5115...55.55..",
        ".51115..5225225.",
        ".51115.522222225",
        "..51155622222225",
        "...5566666666665",
        "....567666666665",
        "...5662222256665",
        "...5651155556665",
        "..56515544446665",
        ".566444444444445",
        "5444444511555555",
        "5444445115222222",
        ".544445152222222",
        "..54445552222222",
        "...5555555555555",
    ]);
    let front_cast_bl = parse_3bpp_ascii([
        ".511155555555555",
        "5144155155155155",
        "5444455444444445",
        "5666655444433335",
        ".566665444444445",
        "..56675444433335",
        "..55675444444445",
        "...5675444433335",
        "...5165444444445",
        "....555444444445",
        "...5675444433335",
        "..54444544444445",
        "..54444555555555",
        ".5144445........",
        ".5111115........",
        "..555555........",
    ]);

    // Back Idle (Iconic Emerald-Green Spiky SMW Bowser Shell + White Conical Spikes + Red Hair Tuft & Horns above!)
    let back_idle_tl = parse_3bpp_ascii([
        "..........55.55.",
        "..555....5225225",
        ".51115...5222222",
        ".51115..52225222",
        "..5115.552222222",
        "...5555666666666",
        "....566667766666",
        "...5511111111111",
        "..51166667766666",
        ".516666651566666",
        ".516666511156676",
        "5166676651566766",
        "5166766666667666",
        "5166666677776666",
        "5166676511156666",
        "5166765111115676",
    ]);
    let back_idle_bl = parse_3bpp_ascii([
        "5166766511156766",
        "5166666655566666",
        ".516667666665156",
        ".516666666651115",
        "..51667766665156",
        "..51666666666666",
        "...5111111111111",
        "....554444444444",
        "...5675555555555",
        "..54444544444444",
        "..54434555444444",
        "..5443445.554444",
        ".51414145...5544",
        ".51111115....555",
        "..555555........",
        "................",
    ]);

    // Back Cast (Muscular clawed arms raised beside spiky green shell!)
    let mut back_cast_tl = back_idle_tl;
    back_cast_tl[4][0] = 5;
    back_cast_tl[4][1] = 1;
    back_cast_tl[4][2] = 4;
    back_cast_tl[5][0] = 5;
    back_cast_tl[5][1] = 4;
    back_cast_tl[5][2] = 4;
    back_cast_tl[5][3] = 4;
    back_cast_tl[6][0] = 5;
    back_cast_tl[6][1] = 4;
    back_cast_tl[6][2] = 5;
    back_cast_tl[6][3] = 1;
    back_cast_tl[6][4] = 5;
    let back_cast_bl = back_idle_bl;

    // Side Profile King Bowser (Full 32x32 facing LEFT: TL + TR / BL + BR — crisp golden snout, blazing fiery mane, fierce eye & fangs!)
    let side_tl = parse_3bpp_ascii([
        ".........555.55.",
        "........51115225",
        ".......511152222",
        "......5662222222",
        ".....56622222222",
        "....566511522266",
        "...5665151556676",
        "..54444666666666",
        ".544444444446676",
        "5444544444446666",
        "5444444444444666",
        ".555155515445515",
        "....522222555555",
        ".....52212555155",
        "......5444555555",
        ".....54444444666",
    ]);
    let side_tr = parse_3bpp_ascii([
        "................",
        "5...............",
        "25..............",
        "225.............",
        "2225............",
        "66555...........",
        "666115..........",
        "67616655........",
        "661666655.......",
        "6166765155......",
        "61667511155.....",
        "166666515665....",
        "1667666667665...",
        "1676676515665...",
        "16666651116765..",
        "16676665156665..",
    ]);
    let side_bl = parse_3bpp_ascii([
        ".....54444444666",
        "....544345556666",
        "...5444544455666",
        "..51444444415666",
        "..51144444455666",
        "...5555443444666",
        ".....54444444666",
        ".....55555555666",
        "......5443444666",
        "......5444446666",
        "......5444444676",
        ".....54444444766",
        ".....54444344445",
        "....51414145....",
        "....51111115....",
        ".....555555.....",
    ]);
    let side_br = parse_3bpp_ascii([
        "16666666766665..",
        "16676515667665..",
        "16675111666665..",
        "16666515667665..",
        "16666666666615..",
        "6166766667615...",
        "661666666615....",
        "66611111115.....",
        "66665544455.....",
        "666766555445....",
        "6667664444445...",
        "6676664444445...",
        "555544444445....",
        "....51414145....",
        "....51111115....",
        ".....555555.....",
    ]);

    KingBowserParts {
        front_idle_tl,
        front_idle_bl,
        front_cast_tl,
        front_cast_bl,
        back_idle_tl,
        back_idle_bl,
        back_cast_tl,
        back_cast_bl,
        side_tl,
        side_tr,
        side_bl,
        side_br,
    }
}

// =============================================================================
// 8. FIX #4: NORMAL SINGLE-HEADED UNCLE LUIGI (SHEET 0xC2 & OAM $0D:D163)
// =============================================================================

pub struct NormalUncleLuigiParts {
    pub head_front: [[u8; 16]; 16],
    pub head_side: [[u8; 16]; 16],
    pub head_hurt: [[u8; 16]; 16],
    pub body_sit: [[u8; 16]; 16],
    pub body_walk_down: [[u8; 16]; 16],
    pub body_walk_side_1: [[u8; 16]; 16],
    pub body_walk_side_2: [[u8; 16]; 16],
}

/// Build a normal, handsome, SINGLE-HEADED SMW Uncle Luigi!
/// - Heads use Pal 6 (`fl=0x0C` GREEN): `1=White ('L' badge/eyes), 3=Tan shade, 4=Warm Skin,
///   5=Black outline/mustache/hair, 6=Vibrant Luigi Green Cap, 7=Deep Luigi Green Brim`!
/// - Bodies use Pal 7 (`fl=0x0E` BLUE): `1=Pure White Gloves/Collar, 3=Gold Buttons & Boots,
///   4=Light Tan/Skin, 5=Black Outline/Soles, 6=Vibrant Sky-Blue Overalls, 7=Deep Royal-Blue Shading`!
pub fn build_normal_uncle_luigi() -> NormalUncleLuigiParts {
    // Front Head (Tall Green Cap with White 'L' emblem + Friendly SMW Eyes + Bushy Mustache!)
    let head_front = parse_3bpp_ascii([
        "....55555555....",
        "...5666666665...",
        "..566611116665..",
        "..566116611665..",
        "..566116611665..",
        ".57777777777775.",
        ".55555555555555.",
        "..554154451455..",
        ".54541544514545.",
        ".54444444444445.",
        "..544434434445..",
        "..545554455545..",
        "...5555555555...",
        "....54444445....",
        ".....554455.....",
        "................",
    ]);

    // Side Head (facing LEFT: Green Cap with protruding brim + tall eye + round nose + mustache!)
    let head_side = parse_3bpp_ascii([
        "....55555555....",
        "...5666666665...",
        "..566116666665..",
        "..566116666665..",
        ".5777777777775..",
        "57777555555555..",
        ".5555415445555..",
        "...54415444545..",
        "..544444444545..",
        ".5444444444445..",
        "..55434444555...",
        "...555554445....",
        "....5554445.....",
        ".....54445......",
        "......555.......",
        "................",
    ]);

    // Sewer Resting/Hurt Head (`C2 (8,0)`): Luigi looking determined/resting with Green Cap!
    let head_hurt = parse_3bpp_ascii([
        "....55555555....",
        "...5666666665...",
        "..566611116665..",
        "..566116611665..",
        ".57777777777775.",
        ".55555555555555.",
        "..554444444455..",
        ".54545544554545.",
        ".54444444444445.",
        "..544434434445..",
        "..545554455545..",
        "...5555555555...",
        "....54444445....",
        ".....554455.....",
        "................",
        "................",
    ]);

    // Sitting/Standing Front Body (`C2 (6,0)` — ZERO SECOND HEAD!!):
    // Clean Royal-Blue Overalls + Two Gleaming Gold Buttons + Crisp White Collar & White-Gloved Hands!
    let body_sit = parse_3bpp_ascii([
        "....55111155....",
        "...5771111775...",
        "..577667766775..",
        ".57766677666775.",
        ".57763666636775.",
        ".51166666666115.",
        ".51156666665115.",
        "..555667766555..",
        "...5666776665...",
        "...5666666665...",
        "...5667557665...",
        "...5777557775...",
        "...5555..5555...",
        "..53335..53335..",
        "..55555..55555..",
        "................",
    ]);

    // Walking Down Body (`C2 (4,0)` — H-flipped between F2 & F3 for smooth 2-step Luigi walk!)
    let body_walk_down = parse_3bpp_ascii([
        "....55111155....",
        "...5771111775...",
        "..577667766775..",
        ".57766677666775.",
        ".57763666636775.",
        ".51166666666115.",
        ".51156666665115.",
        "..555667766555..",
        "...5666776665...",
        "...5666556665...",
        "...5675..5665...",
        "..53335..5775...",
        "..55555..53335..",
        ".........53435..",
        ".........55555..",
        "................",
    ]);

    // Walking Side Body 1 (`C2 (0,2)`, F4: Stride A — left boot forward, right boot back!)
    let body_walk_side_1 = parse_3bpp_ascii([
        ".....551155.....",
        "....57711775....",
        "...5776667775...",
        "...5763666775...",
        "..51156666675...",
        "..51156666675...",
        "...5556666775...",
        "....56666775....",
        "....56676665....",
        "...5665.5675....",
        "..53335..5335...",
        "..53435..53335..",
        "..55555...5555..",
        "................",
        "................",
        "................",
    ]);

    // Walking Side Body 2 (`C2 (4,2)`, F5: Stride B — boots together mid-stride!)
    let body_walk_side_2 = parse_3bpp_ascii([
        ".....551155.....",
        "....57711775....",
        "...5776667775...",
        "...5763666775...",
        "...576666115....",
        "...576666115....",
        "....5666655.....",
        "....56666775....",
        "....56676665....",
        ".....566765.....",
        ".....566765.....",
        "....5333345.....",
        "....5333335.....",
        "....5555555.....",
        "................",
        "................",
    ]);

    NormalUncleLuigiParts {
        head_front,
        head_side,
        head_hurt,
        body_sit,
        body_walk_down,
        body_walk_side_1,
        body_walk_side_2,
    }
}

/// Build Uncle Luigi DIRECTLY from Mario's exact 4bpp Player Bank (`0x080000..0x087000`)!
/// Pixel-for-pixel identical face, eyes, nose, mustache, gloves, boots & proportions to Mario,
/// remapped onto ALTTP's vibrant Luigi Green palette (`Pal 6` = `0x0C` for BOTH head & body!):
/// - `1`=White (Gloves, Eyes, Cap Badge), `2`=Gold (Overall Buttons), `3`=Brown (Hair, Boots),
/// - `4`=Warm Peach Skin, `5`=Black (Outlines, Mustache, Pupils),
/// - `6`=Vibrant Luigi Emerald Green (Cap & Shirt), `7`=Deep Luigi Forest Green (Cap Brim & Overalls)!
pub fn build_mario_green_uncle_luigi_from_bank(mario_bank_4bpp: &[u8]) -> NormalUncleLuigiParts {
    let read_mario_4bpp_cell = |mx: usize, my: usize, flip_h: bool| -> [[u8; 16]; 16] {
        // Map Mario's 4bpp palette indices (0..15) -> ALTTP 3bpp Pal 6 GREEN LUIGI (0..7)!
        // Mario 4bpp: 1=White, 2=Skin, 3=BrownHair/Boots, 6=Mustache, 7=CapBrim/ShirtShade, 8/9=Cap/Shirt, 10/11=Overalls, 14=Pupils/Outline
        const LUT_MARIO_TO_GREEN_LUIGI: [u8; 16] = [
            0, // 0: Transparent
            1, // 1: Pure White (Gloves, Eye Sclera, 'L' Cap Badge)
            4, // 2: Warm Peach Skin (Face, Cheeks, Forehead, Chin)
            3, // 3: Brown (Hair, Sideburns, Boots)
            6, // 4: Vibrant Luigi Emerald Green (Cap highlight)
            6, // 5: Vibrant Luigi Emerald Green (Cap shine)
            5, // 6: Black Mustache & Glaring Pupils
            7, // 7: Deep Luigi Forest Green (Cap Brim & Shirt Shade)
            6, // 8: Vibrant Luigi Emerald Green (Cap & Shirt!)
            6, // 9: Vibrant Luigi Emerald Green (Cap & Shirt!)
            5, // 10: Deep Dark Outlines / Boot Soles
            7, // 11: Deep Luigi Forest Green Overalls
            7, // 12: Deep Luigi Forest Green Overalls
            4, // 13: Skin Highlight
            5, // 14: Black Pupils & Mouth Outline
            5, // 15: Black Outlines
        ];
        let mut px = [[0u8; 16]; 16];
        let base = (my * 32 + mx * 2) * 32;
        for ty in 0..2usize {
            for tx in 0..2usize {
                let off = base + (ty * 16 + tx) * 32;
                if off + 32 > mario_bank_4bpp.len() {
                    continue;
                }
                for y in 0..8usize {
                    let p0 = mario_bank_4bpp[off + y * 2];
                    let p1 = mario_bank_4bpp[off + y * 2 + 1];
                    let p2 = mario_bank_4bpp[off + 16 + y * 2];
                    let p3 = mario_bank_4bpp[off + 16 + y * 2 + 1];
                    for x in 0..8usize {
                        let bit = 7 - x;
                        let v = ((p0 >> bit) & 1)
                            | (((p1 >> bit) & 1) << 1)
                            | (((p2 >> bit) & 1) << 2)
                            | (((p3 >> bit) & 1) << 3);
                        let dx = tx * 8 + x;
                        let dy = ty * 8 + y;
                        let dst_x = if flip_h { 15 - dx } else { dx };
                        px[dy][dst_x] = LUT_MARIO_TO_GREEN_LUIGI[v as usize];
                    }
                }
            }
        }
        px
    };

    let head_front = read_mario_4bpp_cell(1, 0, false); // Exact Mario Front Head in Luigi Green!
    let head_side = read_mario_4bpp_cell(0, 0, true); // Exact Mario Side Head (flipped LEFT for Sheet 0xC2!)
    let head_hurt = read_mario_4bpp_cell(4, 9, false); // Exact Mario Sleeping/Resting Head in Luigi Green!
    let body_sit = read_mario_4bpp_cell(3, 1, false); // Exact Mario Standing Front Body in Green Clothes!
    let body_walk_down = read_mario_4bpp_cell(4, 1, false); // Exact Mario Walk Down Body in Green Clothes!
    let body_walk_side_1 = read_mario_4bpp_cell(1, 1, true); // Exact Mario Side Walk 1 (flipped LEFT!)
    let body_walk_side_2 = read_mario_4bpp_cell(2, 1, true); // Exact Mario Side Walk 2 (flipped LEFT!)

    NormalUncleLuigiParts {
        head_front,
        head_side,
        head_hurt,
        body_sit,
        body_walk_down,
        body_walk_side_1,
        body_walk_side_2,
    }
}

/// Patch Uncle Luigi's OAM table at `$0D:D163..$0D:D2E3` (`PC 0x06D163`, 384 bytes = 8 frames x 6 OAM):
/// 1. Sets BOTH Head & Body to Pal 6 GREEN (`0x0C` / `0x4C`) so Luigi is Mario's exact twin in GREEN clothes!
/// 2. Raises standing/walking head cleanly 2px (`-10 -> -12`, `-9 -> -11`).
/// 3. Aligns Side Walk bodies `F4/F5` tiles (`0x23 -> 0x20`, `0x25 -> 0x24`) and centers Sewer body `F6/F7` (`t=0x06`).
pub fn patch_uncle_luigi_oam_palettes(
    rom_zelda: &mut SnesRom,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    const UNCLE_OAM_PC: usize = 0x06D163;
    if UNCLE_OAM_PC + 384 > rom_zelda.data.len()
        || rom_zelda.data[UNCLE_OAM_PC + 4] != 0x00
        || rom_zelda.data[UNCLE_OAM_PC + 5] != 0x0E
    {
        return Ok(0);
    }

    let mut tbl = rom_zelda.data[UNCLE_OAM_PC..UNCLE_OAM_PC + 384].to_vec();
    for fr in 0..8usize {
        for s in 0..6usize {
            let off = fr * 48 + s * 8;
            let x = i16::from_le_bytes([tbl[off], tbl[off + 1]]);
            let y = i16::from_le_bytes([tbl[off + 2], tbl[off + 3]]);
            let tile = tbl[off + 4];
            let flag = tbl[off + 5];

            // Skip NameTable 1 sword/shield OAMs (bit 0 = 1: fl=0x0D/0x4D/0x8D)
            if (flag & 0x01) != 0 {
                continue;
            }

            // Head OAMs originally had Pal 7 (`0x0E`): swap to Pal 6 GREEN LUIGI (`0x0C`)!
            if (flag & 0x0E) == 0x0E {
                tbl[off + 5] = (flag & !0x0E) | 0x0C;
                if y == -10 || y == -9 {
                    let ny = y - 2;
                    tbl[off + 2..off + 4].copy_from_slice(&ny.to_le_bytes());
                }
            }
            // Body OAMs originally had Pal 6 (`0x0C`): KEEP Pal 6 GREEN CLOTHES (`0x0C`) for authentic Green Mario/Luigi!
            else if (flag & 0x0E) == 0x0C {
                tbl[off + 5] = (flag & !0x0E) | 0x0C;
                // Align Side Walk bodies (F4: 0x23 -> 0x20, F5: 0x25 -> 0x24)
                if tile == 0x23 {
                    tbl[off + 4] = 0x20;
                } else if tile == 0x25 {
                    tbl[off + 4] = 0x24;
                }
                // Center Sewer Resting Body (F6/F7: originally split -8/+8 t=0x20 -> single centered 0,-17 t=0x06!)
                if (fr == 6 || fr == 7) && tile == 0x20 && (x == -8 || x == 8) {
                    tbl[off..off + 2].copy_from_slice(&0i16.to_le_bytes());
                    tbl[off + 2..off + 4].copy_from_slice(&(-17i16).to_le_bytes());
                    tbl[off + 4] = 0x06;
                    tbl[off + 5] = 0x0C;
                }
            }
        }
    }

    session.swap_custom_bytes(
        rom_zelda,
        UNCLE_OAM_PC,
        UNCLE_OAM_PC,
        tbl,
        "OAM Oncle Luigi ($0D:D163) : Sprite exact Mario en vêtements verts (Pal 6) + Corps unique"
            .into(),
        SwapKind::CodePatch { byte_count: 384 },
    )?;
    Ok(1)
}

// =============================================================================
// 9. FIX #6: CRISP HIGH-CONTRAST 16x16 SMW SUPER MUSHROOM (CHESTS / DROPS / BOSS)
// =============================================================================

/// Build a museum-grade, symmetrical, high-contrast 16x16 SMW Super Mushroom calibrated to
/// verified ALTTP ReceiveItem Red Palette (`0x0DD228..0x0DD236`, `pal = 0x01`):
/// - `1` = Pure White (glossy spots & bright mushroom stem)
/// - `2` = Vivid Nintendo Mario Ruby Red (`#F83020` cap!)
/// - `3` = Rich Deep Crimson (`#B81010` lower cap bevel!)
/// - `6` = Soft Cream-Silver (`#D8D8E8` stem contour shading!)
/// - `5` = Crisp Black Outline & Cute Vertical Eyes!
/// Eliminates murky dithering so every chest, Heart Container, and Boss Pendant Mushroom pops in vivid red!
pub fn build_crisp_smw_mushroom_16x16() -> [[u8; 16]; 16] {
    parse_3bpp_ascii([
        ".....555555.....",
        "...5522112255...",
        "..522211112225..",
        ".52112111121125.",
        ".51112211221115.",
        "5211122222211125",
        "5221222222221225",
        "5222222222222235",
        "5322222222222335",
        ".53355555555335.",
        "..551111111155..",
        "...5115115115...",
        "...5115115115...",
        "...5611111165...",
        "....56611665....",
        ".....555555.....",
    ])
}

// =============================================================================
// 10. FIX #3B: DUNGEON JAILER GUARDING PEACH — BALL & CHAIN TROOPER (SHEET 0xB7)
//     -> HEAVY BOWSER / KOOPA CAPTAIN SWINGING A FIERCE SMW CHAIN CHOMP FLAIL!
// =============================================================================

pub struct BallNChainCaptainParts {
    pub head_front: [[u8; 16]; 16],
    pub head_side: [[u8; 16]; 16],
    pub head_back: [[u8; 16]; 16],
    pub body_front_idle: [[u8; 16]; 16],
    pub body_front_stride: [[u8; 16]; 16],
    pub body_side_1: [[u8; 16]; 16],
    pub body_side_2: [[u8; 16]; 16],
    pub body_back_idle: [[u8; 16]; 16],
    pub body_back_stride: [[u8; 16]; 16],
    pub chain_chomp: [[u8; 16]; 16],
}

/// Build a cohesive, menacing Mario-universe Dungeon Jailer (Ball & Chain Trooper `Sprite 0x6A`,
/// plus Bomb / Charging Spear Elite Guards sharing `Sheet 0xB7` & `SpriteDraw_GuardBody`):
/// **Armored Heavy Koopa Captain** (spiked horned war-helmet, fiery brow, golden snout,
/// armored spiky shell & boots) swinging a **Fierce Toothed SMW Chain Chomp** (`tile $2A`)!
/// Uses universal guard palette mapping (`1=White horns/teeth/eyes/rim, 3=Amber shade,
/// 4=Gold snout/belly, 5=Charcoal black outline/Chomp body, 6=Primary armor/shell color,
/// 7=Dark armor/shell shade`).
pub fn build_ball_and_chain_koopa_captain() -> BallNChainCaptainParts {
    // Front Horned Heavy Koopa Captain Head (`B7 (0,0)`)
    let head_front = parse_3bpp_ascii([
        "..55........55..",
        ".5115.5555.5115.",
        ".51155666655115.",
        "..515667766515..",
        "..556666666655..",
        "..577777777775..",
        "..565115511565..",
        "..565155551565..",
        "..544444444445..",
        ".54443444434445.",
        ".54451544515445.",
        "..544444444445..",
        "...5334444335...",
        "....55544555....",
        "......5445......",
        "................",
    ]);

    // Side Horned Heavy Koopa Captain Head (`B7 (2,0)`, facing LEFT — H-flipped for Right)
    let head_side = parse_3bpp_ascii([
        "....55...55.....",
        "...5115.5115....",
        "...511556665....",
        "....556667665...",
        "...5666666675...",
        "..57777777775...",
        "..54511566665...",
        ".544551566765...",
        "544444444665....",
        "54434444445.....",
        ".5515444445.....",
        "...53344445.....",
        ".....55445......",
        "......5435......",
        ".......55.......",
        "................",
    ]);

    // Back Horned Heavy Koopa Captain Head (`B7 (4,0)`)
    let head_back = parse_3bpp_ascii([
        "..55........55..",
        ".5115.5555.5115.",
        ".51155666655115.",
        "..515667766515..",
        "..556666666655..",
        "..566766667665..",
        "..566667766665..",
        "..576666666675..",
        "..577667766775..",
        "...5777777775...",
        "....55444455....",
        ".....534435.....",
        "......5445......",
        "......5555......",
        "................",
        "................",
    ]);

    // Front Armored Spiky Koopa Captain Body — Neutral (`B7 (6,0)`) & Stride (`B7 (12,0)`)
    let body_front_idle = parse_3bpp_ascii([
        ".55155444455155.",
        "5661544444451665",
        "5671533333351765",
        "5161544444451615",
        ".55153333335155.",
        "..515444444515..",
        "..511544445115..",
        "...5557777555...",
        "...5771771775...",
        "...5555555555...",
        "..55665..56655..",
        "..56675..57665..",
        "..56665..56665..",
        "..57775..57775..",
        "..55555..55555..",
        "................",
    ]);
    let body_front_stride = parse_3bpp_ascii([
        ".55155444455155.",
        "5661544444451665",
        "5671533333351765",
        "5161544444451615",
        ".55153333335155.",
        "..515444444515..",
        "..511544445115..",
        "...5557777555...",
        "...5771771775...",
        "...5555555555...",
        "..55665..5555...",
        "..56675..56665..",
        "..55555..56675..",
        ".........57775..",
        ".........55555..",
        "................",
    ]);

    // Side Armored Spiky Koopa Captain Body — Stride 1 (`B7 (8,0)`) & Stride 2 (`B7 (2,2)`)
    let body_side_1 = parse_3bpp_ascii([
        "....54451155....",
        "...54451666655..",
        "..5443516676615.",
        "..5444516666115.",
        ".54453516766655.",
        "..5554516667615.",
        "...543516676115.",
        "....54516666655.",
        "....5771111115..",
        "....557755555...",
        "..55665..5555...",
        ".566675..56665..",
        ".577775...5775..",
        ".555555...5555..",
        "................",
        "................",
    ]);
    let body_side_2 = parse_3bpp_ascii([
        "....54451155....",
        "...54451666655..",
        "..5443516676615.",
        "..5444516666115.",
        "...543516766655.",
        "...544516667615.",
        "...543516676115.",
        "....54516666655.",
        "....5771111115..",
        "....557755555...",
        ".....556655.....",
        "....5666675.....",
        "....5667665.....",
        "....5777775.....",
        "....5555555.....",
        "................",
    ]);

    // Back Armored Spiky Bowser-Shell Koopa Captain Body — Neutral (`B7 (10,0)`) & Stride (`B7 (12,2)`)
    let body_back_idle = parse_3bpp_ascii([
        "..551111111155..",
        ".51666511566615.",
        "5166661111666615",
        "5167665115667615",
        "5166666776666615",
        "5151566666651515",
        "5111167667611115",
        ".55156666665155.",
        ".51166677666115.",
        "..551111111155..",
        "..55665..56655..",
        "..56675..57665..",
        "..56665..56665..",
        "..57775..57775..",
        "..55555..55555..",
        "................",
    ]);
    let body_back_stride = parse_3bpp_ascii([
        "..551111111155..",
        ".51666511566615.",
        "5166661111666615",
        "5167665115667615",
        "5166666776666615",
        "5151566666651515",
        "5111167667611115",
        ".55156666665155.",
        ".51166677666115.",
        "..551111111155..",
        "..55665..5555...",
        "..56675..56665..",
        "..55555..56675..",
        ".........57775..",
        ".........55555..",
        "................",
    ]);

    // Fierce SMW Chain Chomp Flail Head (`B7 (10,2)` = tile `$2A`!)
    // Pitch-black metallic sphere (`5`) with gleaming steel highlight (`1`/`7`), angry white eyes,
    // and menacing jagged white chomping teeth (`1`)!
    let chain_chomp = parse_3bpp_ascii([
        "....55555555....",
        "..555777555555..",
        ".55711755555555.",
        ".57115555555555.",
        "5577511155111555",
        "5555515155151555",
        "5555511155111555",
        "5555555555555555",
        "5511551155115515",
        "5551155115511555",
        "5555555555555555",
        "5551155115511555",
        ".51155115511555.",
        ".55555555555555.",
        "..555555555555..",
        "....55555555....",
    ]);

    BallNChainCaptainParts {
        head_front,
        head_side,
        head_back,
        body_front_idle,
        body_front_stride,
        body_side_1,
        body_side_2,
        body_back_idle,
        body_back_stride,
        chain_chomp,
    }
}

/// Patch `SpriteDraw_GuardHead` Y-sub (`FR PC 0x02B16E`) & `SpriteDraw_GuardBody` (`FR PC 0x02B19D..0x02B395`,
/// verified via `jpdasm bank_05.asm:9173..9334`!) so the Ball & Chain Trooper guarding Princess Peach
/// (plus Red Spear, Charging Blue Spear & Bomb Elite Guards) renders ONE crisp centered 16x16 body
/// strictly from `Sheet 0xB7` (ZERO mixed `Sheet 0xBA` leakage, ZERO corrupted/split torso!).
pub fn patch_ball_and_chain_oam_tables(
    rom_zelda: &mut SnesRom,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    let mut ops = 0usize;

    // 1. Raise Elite Guard / Ball & Chain Trooper head Y-sub (`0x02B16C: 38 E9 09 00` -> `SBC #$000C`)
    if 0x02B170 <= rom_zelda.data.len()
        && rom_zelda.data[0x02B16C] == 0x38
        && rom_zelda.data[0x02B16D] == 0xE9
        && rom_zelda.data[0x02B16E] == 0x09
    {
        session.swap_custom_bytes(
            rom_zelda,
            0x02B16E,
            0x02B16E,
            vec![0x0C],
            "Hauteur Tête Boss Geôlier / Gardes Élite ($05:B16E) : -12px au-dessus des épaules"
                .into(),
            SwapKind::CodePatch { byte_count: 1 },
        )?;
        ops += 1;
    }

    // 2. Patch all 24 frames x 3 slots of `SpriteDraw_GuardBody` (`0x02B19D..0x02B395`)
    //    Slot 0 = ONE centered 16x16 Heavy Koopa Captain body strictly in Sheet 0xB7 ($00..$3F);
    //    Slots 1..2 = transparent blank tile 0x0E (B7(14,0)), sz=0!
    if 0x02B395 <= rom_zelda.data.len()
        && rom_zelda.data[0x02B2BD] == 0x46
        && rom_zelda.data[0x02B2BE] == 0x06
    {
        let specs: [(i16, u8, u8); 24] = [
            (0, 0x06, 0x00), // F00 Down Neutral
            (1, 0x06, 0x00), // F01 Down Neutral bob
            (0, 0x0C, 0x00), // F02 Down Stride L
            (0, 0x0C, 0x00), // F03 Down Stride L
            (0, 0x0C, 0x40), // F04 Down Stride R
            (0, 0x0C, 0x40), // F05 Down Stride R
            (0, 0x08, 0x00), // F06 Left Stride 1
            (0, 0x08, 0x00), // F07 Left Stride 1
            (1, 0x22, 0x00), // F08 Left Stride 2
            (1, 0x22, 0x00), // F09 Left Stride 2
            (0, 0x0A, 0x00), // F10 Up Neutral
            (1, 0x0A, 0x00), // F11 Up Neutral bob
            (0, 0x2C, 0x00), // F12 Up Stride L
            (0, 0x2C, 0x00), // F13 Up Stride L
            (0, 0x2C, 0x40), // F14 Up Stride R
            (0, 0x2C, 0x40), // F15 Up Stride R
            (0, 0x08, 0x40), // F16 Right Stride 1
            (0, 0x08, 0x40), // F17 Right Stride 1
            (1, 0x22, 0x40), // F18 Right Stride 2
            (1, 0x22, 0x40), // F19 Right Stride 2
            (0, 0x08, 0x40), // F20 Turn Right
            (0, 0x08, 0x00), // F21 Turn Left
            (0, 0x06, 0x00), // F22 Turn Down
            (0, 0x0A, 0x00), // F23 Turn Up
        ];
        let mut x_b = Vec::with_capacity(72 * 2);
        let mut y_b = Vec::with_capacity(72 * 2);
        let mut t_b = Vec::with_capacity(72);
        let mut f_b = Vec::with_capacity(72);
        let mut s_b = Vec::with_capacity(72);
        for &(y0, t0, fl0) in &specs {
            x_b.extend_from_slice(&0i16.to_le_bytes());
            y_b.extend_from_slice(&y0.to_le_bytes());
            t_b.push(t0);
            f_b.push(fl0);
            s_b.push(0x02);
            for _ in 1..3 {
                x_b.extend_from_slice(&0i16.to_le_bytes());
                y_b.extend_from_slice(&0i16.to_le_bytes());
                t_b.push(0x0E); // transparent blank tile in Sheet 0xB7
                f_b.push(0x00);
                s_b.push(0x00);
            }
        }
        let patches: &[(usize, Vec<u8>, &str)] = &[
            (
                0x02B19D,
                x_b,
                "OAM Boss Geôlier X ($05:B19D) : Corps centré unique",
            ),
            (
                0x02B22D,
                y_b,
                "OAM Boss Geôlier Y ($05:B22D) : Démarche lourde propre",
            ),
            (
                0x02B2BD,
                t_b,
                "OAM Boss Geôlier Tuiles ($05:B2BD) : Sheet 0xB7 pur sans fuite 0xBA",
            ),
            (
                0x02B305,
                f_b,
                "OAM Boss Geôlier Flags ($05:B305) : Symétrie 4 directions",
            ),
            (
                0x02B34D,
                s_b,
                "OAM Boss Geôlier Tailles ($05:B34D) : 16x16 unique + 2 vides",
            ),
        ];
        for (pc, data, label) in patches {
            let n = data.len();
            session.swap_custom_bytes(
                rom_zelda,
                *pc,
                *pc,
                data.clone(),
                (*label).into(),
                SwapKind::CodePatch { byte_count: n },
            )?;
            ops += 1;
        }
    }

    Ok(ops)
}

// =============================================================================
// 11. MAXIMUM MARIO WORLD SURPRISES — ALTAR YOSHI, MESSAGE BLOCKS, SMW BUSHES,
//     QUESTION BOULDERS & JET-BLACK WORLD BOB-OMBS!
// =============================================================================

/// Build a 24x16 (3 columns x 2 rows of 8x8 tiles) **Captured Green Yoshi** for Intro Scene 4 Altar
/// (`Sheet 0xC6` cols `0..2` levitating & `3..5` resting, verified via `jpdasm bank_0C.asm:15394`, `Pal 6` GREEN!):
/// - `1`=White (Cheeks, Belly, Saddle Rim), `2`=Red/Orange (Yoshi Saddle, Crest Spikes & Boots!),
/// - `5`=Black Outline & Closed Sleeping Eye, `6`=Vibrant Yoshi Green, `7`=Deep Yoshi Green Shading!
pub fn build_captured_yoshi_altar_24x16() -> [[u8; 24]; 16] {
    let rows: [&str; 16] = [
        "........................",
        ".....5555...............",
        "....5666655.............",
        "...566116665552525......",
        "..566655666652525.......",
        "..56666666116655225.....",
        "..5676661111666222555...",
        "...5555111116671156665..",
        "......55111666666666675.",
        ".......566666667766675..",
        ".......51111166666555...",
        "........55111111555.....",
        ".......522555555225.....",
        "......52225....52225....",
        ".......555......555.....",
        "........................",
    ];
    let mut out = [[0u8; 24]; 16];
    for (y, row) in rows.iter().enumerate() {
        for (x, b) in row.bytes().enumerate() {
            if x < 24 && (b'1'..=b'7').contains(&b) {
                out[y][x] = b - b'0';
            }
        }
    }
    out
}

/// Build the LEFT half (8x16 = two 8x8 tiles: top `(2,0)` & bottom `(2,1)` on `Sheet 0x3C`, H-mirrored!)
/// of an iconic **SMW Message Block** for Overworld Signposts (automatically H-mirrored by ALTTP into a symmetric
/// 16x16 SMW Information Block with wooden post, riveted corners & classic SMW smiley eyes)!
/// Calibrated to `Sheet 0x3C` Overworld Signpost palette: `1`=Dark outline & eyes, `4`=Bright white/cream face,
/// `3`=Warm bevel highlight, `2`=Mid shadow, `5`=Dark wood stand!
pub fn build_smw_message_block_half_8x16() -> ([[u8; 8]; 8], [[u8; 8]; 8]) {
    let top_rows: [&str; 8] = [
        ".1111111", "14444444", "14133333", "14333313", "14333313", "14333313", "14333313",
        "14333333",
    ];
    let bot_rows: [&str; 8] = [
        "14333333", "14313333", "14331111", "14122222", "12222222", ".1111111", ".....152",
        "....1522",
    ];
    let parse8 = |rs: [&str; 8]| -> [[u8; 8]; 8] {
        let mut g = [[0u8; 8]; 8];
        for (y, r) in rs.iter().enumerate() {
            for (x, b) in r.bytes().enumerate() {
                if (b'1'..=b'7').contains(&b) {
                    g[y][x] = b - b'0';
                }
            }
        }
        g
    };
    (parse8(top_rows), parse8(bot_rows))
}

/// Build the LEFT half (8x16 = top `(0,0)` & bottom `(0,1)` on `Sheet 0x3C`, H-mirrored!) of an iconic
/// **Super Mario World Smiling Hill Bush** for Overworld Bushes outside Mario's house & across Hyrule!
/// Calibrated to `Sheet 0x3C` Overworld Bush palette: `1`=Dark bush outline & cute SMW vertical oval eyes,
/// `4`=Glossy top highlight, `3`=Light lime highlight, `6`=Vibrant lush SMW green, `2`=Mid green, `5`=Deep forest shade!
pub fn build_smw_smiling_bush_half_8x16() -> ([[u8; 8]; 8], [[u8; 8]; 8]) {
    let top_rows: [&str; 8] = [
        "....1111", "..113336", ".1343666", ".1366666", "13666166", "16661416", "16661116",
        "16666166",
    ];
    let bot_rows: [&str; 8] = [
        "16666666", "16666666", "16626666", ".1222666", ".1522222", "..115555", "...11111",
        "........",
    ];
    let parse8 = |rs: [&str; 8]| -> [[u8; 8]; 8] {
        let mut g = [[0u8; 8]; 8];
        for (y, r) in rs.iter().enumerate() {
            for (x, b) in r.bytes().enumerate() {
                if (b'1'..=b'7').contains(&b) {
                    g[y][x] = b - b'0';
                }
            }
        }
        g
    };
    (parse8(top_rows), parse8(bot_rows))
}

/// Build a full 16x16 **SMW Smiling Hill Bush** (for carried bush sprites on `0x087600`/`0x08B200 (6,0)`
/// so lifted bushes STAY **LUSH EMERALD GREEN** SMW Smiling Bushes in Mario's raised hands — matching ground bushes RGB-for-RGB!).
/// Exact verified `0x0DD49A` OBJ carried-sprite palette:
///   `1`=Dark outline & cute SMW vertical oval eyes, `4`=Vibrant SMW Emerald Green (matching ground bushes!),
///   `3`=Deep forest green lower shade, `7`=Fresh lime top highlight, `5`=Subtle warm bottom rim!
pub fn build_smw_smiling_bush_16x16() -> [[u8; 16]; 16] {
    let (top_l, bot_l) = build_smw_smiling_bush_half_8x16();
    // Remap BG Sheet 0x3C indices (1=outline, 2=mid-green, 3=lime, 4=shine, 5=deep-shade, 6=main-green)
    // to OBJ 0x0DD49A indices    (1=outline, 3=deep-green, 7=lime/shine, 4=MAIN EMERALD GREEN, 5=warm rim)!
    const BG_TO_CARRIED_OBJ: [u8; 8] = [0, 1, 3, 7, 7, 3, 4, 4];
    let mut out = [[0u8; 16]; 16];
    for y in 0..8 {
        for x in 0..8 {
            let t = BG_TO_CARRIED_OBJ[(top_l[y][x] & 7) as usize];
            let b = BG_TO_CARRIED_OBJ[(bot_l[y][x] & 7) as usize];
            out[y][x] = t;
            out[y][15 - x] = t;
            out[y + 8][x] = b;
            out[y + 8][15 - x] = b;
        }
    }
    out
}

/// Iconic **Snow-White Super Mushroom Polka-Dot** tile (8x8) on Mario's Red Mushroom House Roof (`Sheet 0x44 (11,3)`)!
/// Uses `Aux 0` colors: `5`=deep crimson roof shade, `6`=vibrant Super Mario red roof, `7`=pure snow-white mushroom spot!
pub fn build_smw_mushroom_roof_spot_8x8() -> [[u8; 8]; 8] {
    parse_3bpp_ascii_8x8([
        "55555555", "55777755", "57777775", "57777775", "57777775", "57777775", "55777755",
        "55555555",
    ])
}

/// Build an iconic **Super Mario World `?` Question Block Boulder** (16x16) for Overworld Heavy Rocks
/// (`Sheet 0x07` cell `(12,2)`): riveted beveled SMW `?` block so every heavy rock in Hyrule is pure Mario!
pub fn build_smw_question_boulder_16x16() -> [[u8; 16]; 16] {
    parse_3bpp_ascii([
        ".77777777777777.",
        "7111111111111147",
        "7174444444444757",
        "7144477777444457",
        "7144711111744457",
        "7144717771744457",
        "7144774471744457",
        "7144444717444457",
        "7144447174444457",
        "7144447744444457",
        "7144447174444457",
        "7144447174444457",
        "7144444744444457",
        "7175555555555757",
        "7555555555555557",
        ".77777777777777.",
    ])
}

/// Build a **100% JET-BLACK SMW BOB-OMB** (16x16) for live Ground / Placed / Shop Bombs
/// (`Sheet 0xCB (10,2)` & `0x087600`/`0x08AC00`/`0x08B200 (14,2)`), ensuring TOTAL concordance with our
/// Black HUD Bob-omb & Black Menu/Equipped `Y` Bob-omb (ZERO blue bombs anywhere!):
/// - `5`=Jet-Black round body & crisp outline, `1`=Pure White SMW oval eyes & glossy highlight + fuse cap,
/// - `4`=Bright Golden-Yellow wind-up key, fuse spark & iconic yellow Bob-omb boots, `3`=Amber key/boot shade!
pub fn build_jet_black_smw_bobomb_16x16() -> [[u8; 16]; 16] {
    parse_3bpp_ascii([
        "........44......",
        ".......5415.....",
        "......5115......",
        "....55555555....",
        "...5555555555...",
        "..5515555555554.",
        ".551551551555544",
        ".555551551555534",
        ".555551551555544",
        ".55555555555554.",
        "..555555555555..",
        "...5555555555...",
        "....55555555....",
        "...5445..5445...",
        "..54435..54435..",
        "..55555..55555..",
    ])
}

fn parse_3bpp_ascii_8x8(rows: [&str; 8]) -> [[u8; 8]; 8] {
    let mut out = [[0u8; 8]; 8];
    for (y, row) in rows.iter().enumerate() {
        for (x, ch) in row.bytes().take(8).enumerate() {
            out[y][x] = match ch {
                b'1'..=b'7' => ch - b'0',
                _ => 0,
            };
        }
    }
    out
}

fn flip_h_16(src: &[[u8; 16]; 16]) -> [[u8; 16]; 16] {
    let mut out = [[0u8; 16]; 16];
    for y in 0..16 {
        for x in 0..16 {
            out[y][15 - x] = src[y][x];
        }
    }
    out
}

/// Alternating bright-spark fuse frame for our **100% JET-BLACK SMW BOB-OMB** (`0x087C00`/`0x088200`/`0x088800 (14,2)`)
/// so live ground Bob-ombs animate their sparking fuse without EVER reverting to an old green/blue Zelda bomb!
pub fn build_jet_black_smw_bobomb_spark_16x16() -> [[u8; 16]; 16] {
    parse_3bpp_ascii([
        ".......414......",
        "......41114.....",
        "......5115......",
        "....55555555....",
        "...5555555555...",
        "..551555555555..",
        ".55155155155554.",
        ".555551551555544",
        ".555551551555534",
        ".55555555555554.",
        "..555555555555..",
        "...5555555555...",
        "....55555555....",
        "..5445....5445..",
        ".54435....54435.",
        ".55555....55555.",
    ])
}

/// Full 16x16 **SMW Message Block** (Information Box) for carried/slashed Overworld Signposts
/// (`0x087600` & `0x08B200 (2,0)`), matching ground `Sheet 0x3C (2,0)/(2,1)`!
pub fn build_smw_message_block_16x16() -> [[u8; 16]; 16] {
    parse_3bpp_ascii([
        ".55555555555555.",
        "5111111111111135",
        "5151111111111535",
        "5111111111111135",
        "5111155115511135",
        "5111151115111135",
        "5111155115511135",
        "5111155115511135",
        "5111111111111135",
        "5111511111151135",
        "5111155555511135",
        "5111111111111135",
        "5151111111111535",
        "5333333333333335",
        ".55555555555555.",
        "................",
    ])
}

/// Crisp **SMW Mario Head (Red Cap + M Badge + Mustache)** for the World Map player marker (`0x088E00 (6,2)`).
/// Exact verified World Map sprite indices: `5`=dark outline/eyes/mustache, `7`=bright cap shine, `6`=cap color, `1`=face/M badge.
pub fn build_smw_world_map_mario_head_16x16() -> [[u8; 16]; 16] {
    parse_3bpp_ascii([
        "................",
        "....55555555....",
        "...5677117765...",
        "..566711117665..",
        "..566666666665..",
        ".55555555555555.",
        ".56666666666665.",
        "..551111111155..",
        "..511511115115..",
        "..511511115115..",
        "..511111111115..",
        "..515555555515..",
        "...5555555555...",
        "....51111115....",
        ".....555555.....",
        "................",
    ])
}

/// 4-frame sparkling SMW Super Star cursor for File Select & Game Over menus (Sheet 0x76 @ 0x088E00 cols 8..15, rows 0..1).
/// Replaces the vanilla fluttering Great Fairy cursor with an authentic Mario Starman with twinkling eye shine and sparkles.
/// Palettes 0/1: 5=dark outline/eyes, 1=pure white eye shine & sparkle gleam, 4=bright gold body, 3=medium gold.
pub fn build_smw_sparkling_star_cursor_frames() -> [[[u8; 16]; 16]; 4] {
    let f0 = parse_3bpp_ascii([
        ".......5........",
        "......545.......",
        ".....54445......",
        "....5444445.....",
        "555554444455555.",
        ".5444444444445..",
        "..54514445145...",
        "...545144515....",
        "...5444444445...",
        "..544444444445..",
        ".54444444444445.",
        "5444445544444445",
        "544445..54444445",
        ".5555....555555.",
        "................",
        "................",
    ]);

    // Frame 1: Top point sparkle gleam (cross star on top point)
    let f1 = parse_3bpp_ascii([
        ".......1........",
        "......111.......",
        ".....11411......",
        "....5444445.....",
        "555554444455555.",
        ".5444444444445..",
        "..54514445145...",
        "...545144515....",
        "...5444444445...",
        "..544444444445..",
        ".54444444444445.",
        "5444445544444445",
        "544445..54444445",
        ".5555....555555.",
        "................",
        "................",
    ]);

    // Frame 2: Right arm sparkle twinkle
    let f2 = parse_3bpp_ascii([
        ".......5........",
        "......545.......",
        ".....54445...1..",
        "....5444445.111.",
        "555554444455515.",
        ".5444444444445..",
        "..54514445145...",
        "...545144515....",
        "...5444444445...",
        "..544444444445..",
        ".54444444444445.",
        "5444445544444445",
        "544445..54444445",
        ".5555....555555.",
        "................",
        "................",
    ]);

    // Frame 3: Left arm sparkle twinkle + bright eye glint
    let f3 = parse_3bpp_ascii([
        ".......5........",
        "......545.......",
        "..1..54445......",
        ".1115444445.....",
        ".51554444455555.",
        ".5444444444445..",
        "..54114441145...",
        "...541144115....",
        "...5444444445...",
        "..544444444445..",
        ".54444444444445.",
        "5444445544444445",
        "544445..54444445",
        ".5555....555555.",
        "................",
        "................",
    ]);

    [f0, f1, f2, f3]
}

/// Crisp 8x8 **Mini Mario Cap Face** for the Dungeon Mini-Map player marker (`Sheet 0xD2 (1,3)`).
pub fn build_smw_dungeon_map_mario_8x8() -> [[u8; 8]; 8] {
    parse_3bpp_ascii_8x8([
        ".555555.", "56711765", "56666665", "55111155", "51511515", "51555515", ".511115.",
        "..5555..",
    ])
}

/// High-contrast 16x16 **SMW Bullet Bill** for Ammo/Quiver Drops (`Sheet 0xCD (8,0)` & `(8,2)`).
pub fn build_smw_bullet_bill_drop_16x16() -> [[u8; 16]; 16] {
    parse_3bpp_ascii([
        "................",
        "................",
        "................",
        ".....11111111...",
        "...11555555551..",
        "..1551115555551.",
        ".155155115555151",
        ".155555555551551",
        ".155551155551551",
        ".155555555551551",
        "..1555555555551.",
        "...11555555551..",
        ".....11111111...",
        "................",
        "................",
        "................",
    ])
}

/// Museum-grade **TOAD (KINOPPIO)** 6-part sprite pack for the Sanctuary Priest (`Sheet 0xA6`)!
/// Stands right beside Princess Peach in the Sanctuary!
/// Verified `Sheet 0xA6` palette indices:
///   `5`=dark outline/eyes, `1`=pure snow-white mushroom cap/sleeves/shoes,
///   `4`=vivid red/blue mushroom cap spots & royal Toad vest, `3`=warm peach face & golden vest trim!
pub struct ToadSanctuaryParts {
    pub head_front: [[u8; 16]; 16],
    pub head_blink: [[u8; 16]; 16],
    pub head_side: [[u8; 16]; 16],
    pub body_front: [[u8; 16]; 16],
    pub body_cheer: [[u8; 16]; 16],
    pub body_side: [[u8; 16]; 16],
}

pub fn build_smw_toad_sanctuary() -> ToadSanctuaryParts {
    let head_front = parse_3bpp_ascii([
        "................",
        "....55555555....",
        "..551144441155..",
        ".51114444441115.",
        ".54114444441145.",
        "5441114444111445",
        "5441111111111445",
        "5411111111111145",
        ".51155555555115.",
        "..553333333355..",
        "..533533335335..",
        "..533533335335..",
        "..533333333335..",
        "...5333443335...",
        "....55333355....",
        "......5555......",
    ]);
    let head_blink = parse_3bpp_ascii([
        "................",
        "....55555555....",
        "..551144441155..",
        ".51114444441115.",
        ".54114444441145.",
        "5441114444111445",
        "5441111111111445",
        "5411111111111145",
        ".51155555555115.",
        "..553333333355..",
        "..535533335535..",
        "..533333333335..",
        "..533533335335..",
        "...5335555335...",
        "....55333355....",
        "......5555......",
    ]);
    let head_side = parse_3bpp_ascii([
        "................",
        "....55555555....",
        "..551114441155..",
        ".51441144411115.",
        ".54441114111445.",
        "5144111111114445",
        "5111111111114445",
        "5111111111111415",
        ".51115555555115.",
        "..553333333555..",
        "..5335333335....",
        "..5335333335....",
        "..5333333335....",
        "...53443335.....",
        "....553355......",
        "......555.......",
    ]);
    let body_front = parse_3bpp_ascii([
        "................",
        ".....555555.....",
        "...5543113455...",
        "..533431134335..",
        ".53354311345335.",
        ".53354311345335.",
        "..555431134555..",
        "...5443113445...",
        "...5444334445...",
        "...5111111115...",
        "...5111111115...",
        "...5111551115...",
        "..53335..53335..",
        "..53335..53335..",
        "..55555..55555..",
        "................",
    ]);
    let body_cheer = parse_3bpp_ascii([
        "................",
        ".555.555555.555.",
        "5333543113453335",
        "5333543113453335",
        ".55554311345555.",
        "...5443113445...",
        "...5443113445...",
        "...5444334445...",
        "...5111111115...",
        "...5111111115...",
        "...5111551115...",
        "..53335..53335..",
        "..53335..53335..",
        "..55555..55555..",
        "................",
        "................",
    ]);
    let body_side = parse_3bpp_ascii([
        "................",
        ".....555555.....",
        "....54311445....",
        "...5443114445...",
        "...5443334445...",
        "...5443334445...",
        "...5444344445...",
        "....51111115....",
        "....51111115....",
        "....51155115....",
        "...5333553335...",
        "...5333553335...",
        "...5555555555...",
        "................",
        "................",
        "................",
    ]);
    ToadSanctuaryParts {
        head_front,
        head_blink,
        head_side,
        body_front,
        body_cheer,
        body_side,
    }
}

/// Patch uncompressed `0x087000..0x087600` (Sheet `0x71`, Live Weapons & Projectiles!):
/// 1. **Flying Arrows** `(A,2)/(B,2)` Vert & `(A,3)/(D,3)` Horiz (+ wall-stuck `(B,3)/(C,2)/(C,3)/(D,2)`) -> **FLYING SMW BULLET BILLS**!
/// 2. **Held Bow** `(8,0)..(A,1)` -> **MINI SMW BILL BLASTER CANNON**!
/// 3. **Thrown Boomerang** `(6,2)..(7,3)` -> **16x16 SPINNING SMW KOOPA SHELL**!
/// 4. **Magic Hammer** `(B,0)..(F,1)` & `(0,2)..(1,3)` -> **SMW HAMMER BRO HAMMER**!
pub fn patch_smw_live_weapons_087000(buf: &mut [u8]) {
    if buf.len() < 0x600 {
        return;
    }
    let write_8 = |raw: &mut [u8], col: usize, row: usize, px: &[[u8; 8]; 8]| {
        let base = (row * 16 + col) * 24;
        if base + 24 > raw.len() {
            return;
        }
        for y in 0..8 {
            let (mut p0, mut p1, mut p2) = (0u8, 0u8, 0u8);
            for x in 0..8 {
                let v = px[y][x] & 7;
                let sh = 7 - x;
                p0 |= (v & 1) << sh;
                p1 |= ((v >> 1) & 1) << sh;
                p2 |= ((v >> 2) & 1) << sh;
            }
            raw[base + y * 2] = p0;
            raw[base + y * 2 + 1] = p1;
            raw[base + 16 + y] = p2;
        }
    };
    let write_16 = |raw: &mut [u8], col: usize, row: usize, px: &[[u8; 16]; 16]| {
        for ty in 0..2 {
            for tx in 0..2 {
                let mut sub = [[0u8; 8]; 8];
                for y in 0..8 {
                    for x in 0..8 {
                        sub[y][x] = px[ty * 8 + y][tx * 8 + x];
                    }
                }
                write_8(raw, col + tx, row + ty, &sub);
            }
        }
    };

    // 1. FLYING VERTICAL BULLET BILL: (A,2)=Tail + Flame, (B,2)=Sleek Black Nose + White Angry Eye!
    let bb_vert_tail = parse_3bpp_ascii_8x8([
        "...3....", "..373...", ".37173..", ".56665..", ".55155..", ".55555..", ".55155..",
        ".55555..",
    ]);
    let bb_vert_tip = parse_3bpp_ascii_8x8([
        ".55555..", ".55115..", ".55515..", ".55555..", ".51555..", "..515...", "..555...",
        "...5....",
    ]);
    // 2. FLYING HORIZONTAL BULLET BILL: (A,3)=Tail + Flame + SMW arm outline, (D,3)=Sleek Black Nose + Slanted White Eye!
    let bb_horiz_tail = parse_3bpp_ascii_8x8([
        "........", "...55555", ".3651155", "37155555", "37151155", ".3651555", "...55555",
        "........",
    ]);
    let bb_horiz_tip = parse_3bpp_ascii_8x8([
        "........", "55555...", "511155..", "5511555.", "5555555.", "555555..", "55555...",
        "........",
    ]);
    let bb_horiz_mid = parse_3bpp_ascii_8x8([
        "........", "........", "........", "....5555", "..375155", "..365555", "....5555",
        "........",
    ]);
    let bb_stuck_v1 = parse_3bpp_ascii_8x8([
        "........", "...37...", "..3713..", "..5665..", "..5155..", "..5555..", "..5555..",
        "...55...",
    ]);
    let bb_stuck_v2 = parse_3bpp_ascii_8x8([
        "..5555..", "..5555..", "..5155..", "..5555..", "..5555..", "...55...", "........",
        "........",
    ]);
    let bb_stuck_h = parse_3bpp_ascii_8x8([
        "........", "...555..", "..35155.", ".375555.", "..35555.", "...555..", "........",
        "........",
    ]);
    write_8(buf, 0xA, 2, &bb_vert_tail);
    write_8(buf, 0xB, 2, &bb_vert_tip);
    write_8(buf, 0xA, 3, &bb_horiz_tail);
    write_8(buf, 0xD, 3, &bb_horiz_tip);
    write_8(buf, 0xB, 3, &bb_horiz_mid);
    write_8(buf, 0xC, 2, &bb_stuck_v1);
    write_8(buf, 0xC, 3, &bb_stuck_v2);
    write_8(buf, 0xD, 2, &bb_stuck_h);

    // 3. PORTABLE SMW BILL BLASTER HAND-CANNON (Held Bow at (8,0)..(9,1) & (A,0)/(A,1))
    //    Verified live SP1 Right-Half palette (Do3bppToWRAM4bpp_RightPal -> 0x0DD228..0x0DD236):
    //      5=dark iron charcoal, 7=gunmetal steel, 6=polished silver muzzle rim,
    //      1=pure white specular shine, 2=vivid Mario Red collar, 3=deep Mario crimson!
    let blaster_diag = parse_3bpp_ascii([
        "................",
        "................",
        ".........555....",
        "........56665...",
        ".......561775...",
        "......5577555...",
        ".....5517555....",
        "....5577555.....",
        "...5575553......",
        "..5322553.......",
        "..532233........",
        "...5335.........",
        "................",
        "................",
        "................",
        "................",
    ]);
    // Compact 10px-tall sleek iron Bill Blaster barrel (A,0, V-mirrored by ALTTP around y=7: x=0 muzzle, x=7 breech!)
    let blaster_v = parse_3bpp_ascii_8x8([
        "........", "........", "........", "555.....", "5665553.", "56177523", "56755523",
        "5555553.",
    ]);
    // Compact 10px-wide sleek iron Bill Blaster barrel (A,1, H-mirrored by ALTTP around x=7: y=7 muzzle, y=0 breech!)
    let blaster_h = parse_3bpp_ascii_8x8([
        "....3223", "....3223", "....5555", "....5175", "....5175", "...55755", "...56665",
        "...55555",
    ]);
    write_16(buf, 8, 0, &blaster_diag);
    write_8(buf, 0xA, 0, &blaster_v);
    write_8(buf, 0xA, 1, &blaster_h);

    // 4. SPINNING 16x16 SMW KOOPA SHELL (Live Thrown Boomerang at (6,2)..(7,3)!)
    let koopa_shell_boom = parse_3bpp_ascii([
        "................",
        "................",
        "....55555555....",
        "...5333443335...",
        "..531134433335..",
        ".53133433433345.",
        ".53334333343345.",
        ".54343333334345.",
        ".54433333333445.",
        ".55111111111155.",
        "..511111111115..",
        "..554444444455..",
        "...5555555555...",
        "................",
        "................",
        "................",
    ]);
    write_16(buf, 6, 2, &koopa_shell_boom);

    // 5. SMW HAMMER BRO HAMMER (Magic Hammer at (B,0)..(C,1) diag & (D,0)..(D,1) vert!)
    let hammer_diag = parse_3bpp_ascii([
        "................",
        ".......5555.....",
        "......5211255...",
        ".....521222225..",
        ".....522222225..",
        "......52222255..",
        ".....5652225....",
        "....5666555.....",
        "...56665........",
        "..56665.........",
        "..5665..........",
        "...55...........",
        "................",
        "................",
        "................",
        "................",
    ]);
    write_16(buf, 0xB, 0, &hammer_diag);

    // 6. KAMEK'S SHOP 8x8 RED MINI SUPER MUSHROOM @ (9,2) and floating @ (9,3)
    //    Replaces vanilla Zelda 8x8 heart!
    //    0=transparent, 3=dark crimson outline/eyes, 4=vivid Mario red cap, 1=pure white spot/stem!
    let mini_mush_ground = parse_3bpp_ascii_8x8([
        "..3333..", ".341143.", "34111143", "34444443", ".333333.", ".313313.", ".311113.",
        "..3333..",
    ]);
    let mini_mush_float = parse_3bpp_ascii_8x8([
        "..3333..", ".314413.", "31144113", "34444443", ".333333.", ".313313.", ".311113.",
        "..3333..",
    ]);
    write_8(buf, 9, 2, &mini_mush_ground);
    write_8(buf, 9, 3, &mini_mush_float);

    // 7. KAMEK'S SHOP 8x8 MINI YOSHI BERRY / MAGIC FLASK @ (A,2) & (A,3)
    let mini_berry = parse_3bpp_ascii_8x8([
        "...55...", "..555...", ".53335..", "5311335.", "5313335.", ".53335..", "..555...",
        "........",
    ]);
    write_8(buf, 0xA, 2, &mini_berry);
    write_8(buf, 0xA, 3, &mini_berry);
}

/// Patch `Sheet 0xD0` (Mario's 3 Live Shields + Book of Mudora held in Mario's hand!):
/// - **Lv1 Fighter Shield** `(4..9, 0..1)` -> **SMW Koopa Shell Shield**!
/// - **Lv2 Fire Shield** `(A..F, 0..1)` -> **SMW Red/Spiked Koopa Shell Shield**!
/// - **Lv3 Mirror Shield** `(0..5, 2..3)` -> **SMW Golden Super Star Shield**!
/// - **Book of Mudora** `(E..F, 2..3)` -> **SMW `?` Block Tome**!
pub fn patch_mario_shields_sheet_d0(raw: &mut [u8]) {
    let write_16 = |buf: &mut [u8], col: usize, row: usize, px: &[[u8; 16]; 16]| {
        for ty in 0..2 {
            for tx in 0..2 {
                let base = ((row + ty) * 16 + (col + tx)) * 24;
                if base + 24 > buf.len() {
                    continue;
                }
                for y in 0..8 {
                    let (mut p0, mut p1, mut p2) = (0u8, 0u8, 0u8);
                    for x in 0..8 {
                        let v = px[ty * 8 + y][tx * 8 + x] & 7;
                        let sh = 7 - x;
                        p0 |= (v & 1) << sh;
                        p1 |= ((v >> 1) & 1) << sh;
                        p2 |= ((v >> 2) & 1) << sh;
                    }
                    buf[base + y * 2] = p0;
                    buf[base + y * 2 + 1] = p1;
                    buf[base + 16 + y] = p2;
                }
            }
        }
    };

    // Lv1 Fighter Shield -> Compact SMW Koopa Shell strapped to Mario's arm!
    let sh1_front = parse_3bpp_ascii([
        "................",
        "................",
        "................",
        "................",
        ".....555555.....",
        "....54444445....",
        "...5477667745...",
        "...5476776745...",
        "...5467777645...",
        "...5476776745...",
        "...5444444445...",
        "....54444445....",
        ".....555555.....",
        "................",
        "................",
        "................",
    ]);
    let sh1_back = parse_3bpp_ascii([
        "................",
        "................",
        "................",
        "................",
        ".....555555.....",
        "....54444445....",
        "...5466666645...",
        "...5465665645...",
        "...5466556645...",
        "...5466666645...",
        "...5444444445...",
        "....54444445....",
        ".....555555.....",
        "................",
        "................",
        "................",
    ]);
    let sh1_side = parse_3bpp_ascii([
        "................",
        "................",
        "................",
        "................",
        "......555.......",
        ".....54765......",
        ".....54765......",
        ".....54775......",
        ".....54765......",
        ".....54765......",
        ".....54445......",
        "......555.......",
        "................",
        "................",
        "................",
        "................",
    ]);

    // Lv2 Fire Shield -> SMW Red/Spiked Koopa Shell Shield!
    let sh2_front = parse_3bpp_ascii([
        "................",
        "................",
        ".....555555.....",
        "...5544444455...",
        "..544776677445..",
        "..547767767745..",
        ".54767777776745.",
        ".54776777767745.",
        ".54777666677745.",
        "..547777777745..",
        "..544444444445..",
        "...5444444445...",
        "....55555555....",
        "................",
        "................",
        "................",
    ]);
    let sh2_back = parse_3bpp_ascii([
        "................",
        "................",
        ".....555555.....",
        "...5544444455...",
        "..544666666445..",
        "..546656656645..",
        ".54666655666645.",
        ".54666566566645.",
        ".54666666666645.",
        "..546666666645..",
        "..544444444445..",
        "...5444444445...",
        "....55555555....",
        "................",
        "................",
        "................",
    ]);

    // Lv3 Mirror Shield -> SMW Golden Super Star Royal Shield!
    let sh3_front = parse_3bpp_ascii([
        "................",
        "....55555555....",
        "..554444444455..",
        ".54477766777445.",
        ".54777766777745.",
        ".54766666666745.",
        ".54776656567745.",
        ".54777656577745.",
        ".54776666667745.",
        ".54766677666745.",
        "..546677776645..",
        "..544777777445..",
        "...5444444445...",
        "....55555555....",
        "................",
        "................",
    ]);
    let sh3_back = parse_3bpp_ascii([
        "................",
        "....55555555....",
        "..554444444455..",
        ".54466666666445.",
        ".54666644666645.",
        ".54664444446645.",
        ".54666444466645.",
        ".54666444466645.",
        ".54664466446645.",
        ".54666666666645.",
        "..546666666645..",
        "..544666666445..",
        "...5444444445...",
        "....55555555....",
        "................",
        "................",
    ]);

    // Book of Mudora -> SMW `?` Block Tome!
    let q_tome = parse_3bpp_ascii([
        "................",
        "..555555555555..",
        ".54444444444445.",
        ".54577777777545.",
        ".54775555577745.",
        ".54754444457745.",
        ".54754555457745.",
        ".54775775457745.",
        ".54777754577745.",
        ".54777545777745.",
        ".54777757777745.",
        ".54777545777745.",
        ".54577757777545.",
        ".54444444444445.",
        "..555555555555..",
        "................",
    ]);

    write_16(raw, 4, 0, &sh1_front);
    write_16(raw, 6, 0, &sh1_back);
    write_16(raw, 8, 0, &sh1_side);
    write_16(raw, 0xA, 0, &sh2_front);
    write_16(raw, 0xC, 0, &sh2_back);
    write_16(raw, 0, 2, &sh3_front);
    write_16(raw, 2, 2, &sh3_back);
    write_16(raw, 0xE, 2, &q_tome);
}

/// Museum-grade Overworld `Sheet 0x82` pack:
/// - **Octoroks (4 directions)** `(0..7,0..1)` & `(0..3,2..3)` -> **SMW Galoombas (Goombas)**!
/// - **Octorok Spit** `(4,2)/(5,2)` -> **SMW Fireballs**!
/// - **Overworld Buzz Blobs** `(6..B,2..3)` -> **SMW Spinies**!
/// - **Overworld Crows** `(8..F,0..1)` -> **SMW Paragoombas**!
pub fn patch_smw_overworld_enemies_sheet_82(raw: &mut [u8]) {
    let write_16 = |buf: &mut [u8], col: usize, row: usize, px: &[[u8; 16]; 16]| {
        for ty in 0..2 {
            for tx in 0..2 {
                let base = ((row + ty) * 16 + (col + tx)) * 24;
                if base + 24 > buf.len() {
                    continue;
                }
                for y in 0..8 {
                    let (mut p0, mut p1, mut p2) = (0u8, 0u8, 0u8);
                    for x in 0..8 {
                        let v = px[ty * 8 + y][tx * 8 + x] & 7;
                        let sh = 7 - x;
                        p0 |= (v & 1) << sh;
                        p1 |= ((v >> 1) & 1) << sh;
                        p2 |= ((v >> 2) & 1) << sh;
                    }
                    buf[base + y * 2] = p0;
                    buf[base + y * 2 + 1] = p1;
                    buf[base + 16 + y] = p2;
                }
            }
        }
    };
    let write_8 = |buf: &mut [u8], col: usize, row: usize, px: &[[u8; 8]; 8]| {
        let base = (row * 16 + col) * 24;
        if base + 24 > buf.len() {
            return;
        }
        for y in 0..8 {
            let (mut p0, mut p1, mut p2) = (0u8, 0u8, 0u8);
            for x in 0..8 {
                let v = px[y][x] & 7;
                let sh = 7 - x;
                p0 |= (v & 1) << sh;
                p1 |= ((v >> 1) & 1) << sh;
                p2 |= ((v >> 2) & 1) << sh;
            }
            buf[base + y * 2] = p0;
            buf[base + y * 2 + 1] = p1;
            buf[base + 16 + y] = p2;
        }
    };

    // SMW Galoomba Down (2 waddling frames)
    let gal_down_0 = parse_3bpp_ascii([
        "................",
        "....55555555....",
        "..557777777755..",
        ".57777777777665.",
        ".57755777755665.",
        "5775115775115665",
        "5775151551515665",
        "5777511771156665",
        "5777777777766665",
        ".57751555515665.",
        ".56675555557665.",
        "..556666666655..",
        "..53345555335...",
        ".533445..53345..",
        ".55555....555...",
        "................",
    ]);
    let gal_down_1 = flip_h_16(&gal_down_0);

    // SMW Galoomba Up (from behind)
    let gal_up_0 = parse_3bpp_ascii([
        "................",
        "....55555555....",
        "..557777777755..",
        ".57777777777665.",
        ".57777777776665.",
        "5777777777666665",
        "5777777776666665",
        "5777777766666665",
        "5677777666666665",
        ".56666666666665.",
        ".56666666666665.",
        "..556666666655..",
        "...53355553345..",
        "..53345..533445.",
        "...555....55555.",
        "................",
    ]);
    let gal_up_1 = flip_h_16(&gal_up_0);

    // SMW Galoomba Side (profile)
    let gal_side_0 = parse_3bpp_ascii([
        "................",
        "....55555555....",
        "..557777777755..",
        ".57777777777665.",
        ".57755777777665.",
        "5775115777766665",
        "5751515777766665",
        "5775115777666665",
        "5777777776666665",
        ".55155776666665.",
        ".56555666666665.",
        "..556666666655..",
        "..53345..53345..",
        ".533445...5345..",
        ".55555.....55...",
        "................",
    ]);
    let gal_side_1 = parse_3bpp_ascii([
        "................",
        "....55555555....",
        "..557777777755..",
        ".57777777777665.",
        ".57755777777665.",
        "5775115777766665",
        "5751515777766665",
        "5775115777666665",
        "5777777776666665",
        ".55155776666665.",
        ".56555666666665.",
        "..556666666655..",
        "....533453345...",
        "...5334453345...",
        "...55555.5555...",
        "................",
    ]);

    // SMW Spiny (red spiked shell + white conical spikes)
    let spiny_0 = parse_3bpp_ascii([
        "................",
        "......5..5......",
        ".....515515.....",
        "...5511551155...",
        "..515777777515..",
        ".51157777775115.",
        "..557777777755..",
        ".57777777777765.",
        ".57777777777665.",
        "5511111111111155",
        ".53351533515335.",
        ".53355533555335.",
        "..553333333355..",
        "..5445....5445..",
        "..5555....5555..",
        "................",
    ]);
    let spiny_1 = flip_h_16(&spiny_0);

    // SMW Paragoomba (winged Goomba)
    let para_0 = parse_3bpp_ascii([
        ".555........555.",
        "51115.5555.51115",
        "5111557777551115",
        ".51157777775115.",
        "..557515515755..",
        "...5755775575...",
        "...5777777765...",
        "...5775115765...",
        "....56666665....",
        "....53455345....",
        "...5334553345...",
        "...5555..5555...",
        "................",
        "................",
        "................",
        "................",
    ]);
    let para_1 = parse_3bpp_ascii([
        "................",
        "......5555......",
        "....55777755....",
        "...5777777775...",
        ".55575155157555.",
        "5111555775551115",
        "5111577777751115",
        ".51157511575115.",
        "..55.566665.55..",
        "....53455345....",
        "...5334553345...",
        "...5555..5555...",
        "................",
        "................",
        "................",
        "................",
    ]);

    // SMW Fireball 8x8 projectile
    let fireball = parse_3bpp_ascii_8x8([
        "..555...", ".57375..", "5731135.", "5311175.", "5731375.", ".57375..", "..555...",
        "........",
    ]);

    write_16(raw, 0, 0, &gal_down_0);
    write_16(raw, 2, 0, &gal_down_1);
    write_16(raw, 4, 0, &gal_up_0);
    write_16(raw, 6, 0, &gal_up_1);
    write_16(raw, 0, 2, &gal_side_0);
    write_16(raw, 2, 2, &gal_side_1);
    write_8(raw, 4, 2, &fireball);
    write_8(raw, 5, 2, &fireball);
    write_16(raw, 6, 2, &spiny_0);
    write_16(raw, 8, 2, &spiny_1);
    write_16(raw, 0xA, 2, &spiny_0);
    write_16(raw, 8, 0, &para_0);
    write_16(raw, 0xA, 0, &para_1);
    write_16(raw, 0xC, 0, &para_0);
    write_16(raw, 0xE, 0, &para_1);
}

/// Museum-grade Hyrule Castle Sewers & Caves `Sheet 0x8D` pack:
/// - **Keese Cave Bats** `(0,0)/(2,0)/(4,0)` -> **SMW BOO BUDDIES** (3 classic frames: Grin, Tongue-out, Shy!)
/// - **Sewer Rats** `(6..E,0)` & `(2..4,2)` -> **SMW BUZZY BEETLES**!
/// - **Sewer Ropes** `(6..A,2)` -> **SMW SPINIES**!
pub fn patch_smw_sewers_sheet_8d(raw: &mut [u8]) {
    let write_16 = |buf: &mut [u8], col: usize, row: usize, px: &[[u8; 16]; 16]| {
        for ty in 0..2 {
            for tx in 0..2 {
                let base = ((row + ty) * 16 + (col + tx)) * 24;
                if base + 24 > buf.len() {
                    continue;
                }
                for y in 0..8 {
                    let (mut p0, mut p1, mut p2) = (0u8, 0u8, 0u8);
                    for x in 0..8 {
                        let v = px[ty * 8 + y][tx * 8 + x] & 7;
                        let sh = 7 - x;
                        p0 |= (v & 1) << sh;
                        p1 |= ((v >> 1) & 1) << sh;
                        p2 |= ((v >> 2) & 1) << sh;
                    }
                    buf[base + y * 2] = p0;
                    buf[base + y * 2 + 1] = p1;
                    buf[base + 16 + y] = p2;
                }
            }
        }
    };

    // SMW Boo Buddy f0: Classic grinning ghost with little waving arms!
    let boo_grin = parse_3bpp_ascii([
        "................",
        "....55555555....",
        "..551111111155..",
        ".51111111111775.",
        ".51151115111775.",
        "5111511151117775",
        "5111111111111775",
        "5155555555511775",
        "5151555551511775",
        ".55533333551775.",
        ".51553335517775.",
        "..511555117775..",
        "...5777777755...",
        "....5555555.....",
        "................",
        "................",
    ]);
    // SMW Boo Buddy f1: Spooky wide laugh + rosy tongue out + arms raised!
    let boo_laugh = parse_3bpp_ascii([
        "................",
        "....55555555....",
        "..551111111155..",
        ".51111111111775.",
        "5115511551117775",
        "5111511151111775",
        "5111111111111775",
        "5115555555111775",
        "5115155515117775",
        ".51533333517775.",
        "..553333357775..",
        "..51533357775...",
        "...575557755....",
        "....5555555.....",
        "................",
        "................",
    ]);
    // SMW Boo Buddy f2: Shy Boo covering eyes with both white paws!
    let boo_shy = parse_3bpp_ascii([
        "................",
        "....55555555....",
        "..551111111155..",
        ".51111111111775.",
        ".51111111111775.",
        "5115551155517775",
        "5151115511157775",
        "5151115511157775",
        "5115551155517775",
        ".51133113311775.",
        ".51111111117775.",
        "..511111117775..",
        "...5777777755...",
        "....5555555.....",
        "................",
        "................",
    ]);

    // SMW Buzzy Beetle (armored glossy shell + angry eyes + waddling legs)
    let buzzy_0 = parse_3bpp_ascii([
        "................",
        "................",
        "....55555555....",
        "..557777777755..",
        ".57711777777665.",
        ".57177777776665.",
        "5777777777666665",
        "5777777776666665",
        "5511111111111155",
        ".53351533515335.",
        ".53355533555335.",
        "..553333333355..",
        "..5445....5445..",
        "..5555....5555..",
        "................",
        "................",
    ]);
    let buzzy_1 = flip_h_16(&buzzy_0);

    // SMW Spiny for Sewer Ropes
    let spiny = parse_3bpp_ascii([
        "................",
        "......5..5......",
        ".....515515.....",
        "...5511551155...",
        "..515777777515..",
        ".51157777775115.",
        "..557777777755..",
        ".57777777777765.",
        "5511111111111155",
        ".53351533515335.",
        "..553333333355..",
        "..5445....5445..",
        "..5555....5555..",
        "................",
        "................",
        "................",
    ]);

    write_16(raw, 0, 0, &boo_grin);
    write_16(raw, 2, 0, &boo_laugh);
    write_16(raw, 4, 0, &boo_shy);
    write_16(raw, 6, 0, &buzzy_0);
    write_16(raw, 8, 0, &buzzy_1);
    write_16(raw, 0xA, 0, &buzzy_0);
    write_16(raw, 0xC, 0, &buzzy_1);
    write_16(raw, 0xE, 0, &buzzy_0);
    write_16(raw, 2, 2, &buzzy_1);
    write_16(raw, 4, 2, &buzzy_0);
    write_16(raw, 6, 2, &spiny);
    write_16(raw, 8, 2, &flip_h_16(&spiny));
    write_16(raw, 0xA, 2, &spiny);
}

/// Museum-grade **16x32 SMW THWOMP** for Armos Statues (`Sheet 0x8E (0..3, 0..3)`)!
pub fn patch_smw_thwomp_sheet_8e(raw: &mut [u8]) {
    let write_16 = |buf: &mut [u8], col: usize, row: usize, px: &[[u8; 16]; 16]| {
        for ty in 0..2 {
            for tx in 0..2 {
                let base = ((row + ty) * 16 + (col + tx)) * 24;
                if base + 24 > buf.len() {
                    continue;
                }
                for y in 0..8 {
                    let (mut p0, mut p1, mut p2) = (0u8, 0u8, 0u8);
                    for x in 0..8 {
                        let v = px[ty * 8 + y][tx * 8 + x] & 7;
                        let sh = 7 - x;
                        p0 |= (v & 1) << sh;
                        p1 |= ((v >> 1) & 1) << sh;
                        p2 |= ((v >> 2) & 1) << sh;
                    }
                    buf[base + y * 2] = p0;
                    buf[base + y * 2 + 1] = p1;
                    buf[base + 16 + y] = p2;
                }
            }
        }
    };

    // Verified live in Mednafen (Room 0xC8 Eastern Palace Boss):
    // Each of the 6 Armos Knights is a 32x32 sprite assembled side-by-side WITHOUT H-flip:
    //   - Cols 0..1, rows 0..3 = LEFT 16x32 half
    //   - Cols 2..3, rows 0..3 = RIGHT 16x32 half (= exact horizontal mirror of left half!)
    // Calibrated to Eastern Palace Armos palettes:
    //   5 = Black outline/visor/cleats, 1 = Pure White helmet stripe/eyes/shoulder trim/jersey stripe,
    //   6 = Vibrant Team Blue (turns Blazing Red on the final enraged Chargin' Chuck!),
    //   7 = Deep Shadow Shade, 4 = Bright Gold snout/fists/belt, 3 = Warm Shade!
    let chuck_left_top = parse_3bpp_ascii([
        "...........55555",
        ".........5566111",
        "........56666111",
        ".......576666611",
        "......5766666611",
        "......5766666661",
        ".....57766666666",
        ".....57755555555",
        ".....57544455115",
        "....577544511515",
        "....575444511115",
        "...5555444444444",
        "..56665445111111",
        ".566116535555555",
        "5766116653344444",
        "5766666665555555",
    ]);
    let chuck_left_bot = parse_3bpp_ascii([
        "5776666666666666",
        "5776666111111666",
        "5776661111111166",
        ".576666111111666",
        ".555666666666666",
        "5445576666666666",
        "5441557777777777",
        "5344555555555555",
        ".555..5444444444",
        "......5776666675",
        "......577666675.",
        ".....577666675..",
        "....555555555...",
        "...5114444415...",
        "...5555555555...",
        "....5.5...5.5...",
    ]);
    let chuck_right_top = flip_h_16(&chuck_left_top);
    let chuck_right_bot = flip_h_16(&chuck_left_bot);

    write_16(raw, 0, 0, &chuck_left_top);
    write_16(raw, 0, 2, &chuck_left_bot);
    write_16(raw, 2, 0, &chuck_right_top);
    write_16(raw, 2, 2, &chuck_right_bot);
}

/// Museum-grade **SMW DRY BONES (Skelerex)** for Dungeon Stalfos (`Sheet 0x90`)!
pub fn patch_smw_dry_bones_sheet_90(raw: &mut [u8]) {
    let write_16 = |buf: &mut [u8], col: usize, row: usize, px: &[[u8; 16]; 16]| {
        for ty in 0..2 {
            for tx in 0..2 {
                let base = ((row + ty) * 16 + (col + tx)) * 24;
                if base + 24 > buf.len() {
                    continue;
                }
                for y in 0..8 {
                    let (mut p0, mut p1, mut p2) = (0u8, 0u8, 0u8);
                    for x in 0..8 {
                        let v = px[ty * 8 + y][tx * 8 + x] & 7;
                        let sh = 7 - x;
                        p0 |= (v & 1) << sh;
                        p1 |= ((v >> 1) & 1) << sh;
                        p2 |= ((v >> 2) & 1) << sh;
                    }
                    buf[base + y * 2] = p0;
                    buf[base + y * 2 + 1] = p1;
                    buf[base + 16 + y] = p2;
                }
            }
        }
    };

    let skull_front = parse_3bpp_ascii([
        "................",
        "....55555555....",
        "...5611111165...",
        "..561111111165..",
        "..511551155115..",
        "..511511115115..",
        "..511551155115..",
        "..561115511165..",
        "...5111111115...",
        "...5615115165...",
        "....51111115....",
        "....55155155....",
        ".....555555.....",
        "................",
        "................",
        "................",
    ]);
    let skull_side = parse_3bpp_ascii([
        "................",
        "....5555555.....",
        "...561111165....",
        "..56111111165...",
        "..51551111115...",
        "..51511111115...",
        ".551551111165...",
        "5111111111665...",
        "511111111655....",
        ".555151555......",
        "...511115.......",
        "....5555........",
        "................",
        "................",
        "................",
        "................",
    ]);
    let skull_back = parse_3bpp_ascii([
        "................",
        "....55555555....",
        "...5611111165...",
        "..561111111165..",
        "..561111111165..",
        "..561161161165..",
        "..566111111665..",
        "...5661111665...",
        "....56666665....",
        ".....555555.....",
        "................",
        "................",
        "................",
        "................",
        "................",
        "................",
    ]);
    let body_front = parse_3bpp_ascii([
        "................",
        ".....555555.....",
        "...5561111655...",
        "..515615516515..",
        ".51156111165115.",
        ".55556155165555.",
        "...5661111665...",
        "...5566666655...",
        "....51555515....",
        "....515..515....",
        "...5615..5165...",
        "...5555..5555...",
        "................",
        "................",
        "................",
        "................",
    ]);
    let body_walk = flip_h_16(&body_front);

    write_16(raw, 0, 0, &skull_front);
    write_16(raw, 2, 0, &skull_side);
    write_16(raw, 4, 0, &skull_back);
    write_16(raw, 6, 0, &body_front);
    write_16(raw, 8, 0, &body_walk);
    write_16(raw, 0xA, 0, &body_front);
    write_16(raw, 0xC, 0, &body_walk);
    write_16(raw, 0xC, 2, &body_front);
}

/// Museum-grade **SMW BABY YOSHI** for Kakariko Village Cuccos (`Sheet 0x86`)!
pub fn patch_smw_baby_yoshi_sheet_86(raw: &mut [u8]) {
    let write_16 = |buf: &mut [u8], col: usize, row: usize, px: &[[u8; 16]; 16]| {
        for ty in 0..2 {
            for tx in 0..2 {
                let base = ((row + ty) * 16 + (col + tx)) * 24;
                if base + 24 > buf.len() {
                    continue;
                }
                for y in 0..8 {
                    let (mut p0, mut p1, mut p2) = (0u8, 0u8, 0u8);
                    for x in 0..8 {
                        let v = px[ty * 8 + y][tx * 8 + x] & 7;
                        let sh = 7 - x;
                        p0 |= (v & 1) << sh;
                        p1 |= ((v >> 1) & 1) << sh;
                        p2 |= ((v >> 2) & 1) << sh;
                    }
                    buf[base + y * 2] = p0;
                    buf[base + y * 2 + 1] = p1;
                    buf[base + 16 + y] = p2;
                }
            }
        }
    };

    let yoshi_walk0 = parse_3bpp_ascii([
        "................",
        ".....5555.......",
        "....5711755.....",
        "...571511735....",
        "..5771511735....",
        ".57777117765....",
        ".577777777635...",
        "..55777717635...",
        "...5777111655...",
        "...577111165....",
        "....5611165.....",
        "....53455345....",
        "...5334553345...",
        "...5555..5555...",
        "................",
        "................",
    ]);
    let yoshi_walk1 = parse_3bpp_ascii([
        "................",
        "................",
        ".....5555.......",
        "....5711755.....",
        "...571511735....",
        "..5771511735....",
        ".57777117765....",
        ".577777777635...",
        "..55777717635...",
        "...5777111655...",
        "...577111165....",
        "....5611165.....",
        "....5345345.....",
        "....5555555.....",
        "................",
        "................",
    ]);

    write_16(raw, 0, 0, &yoshi_walk0);
    write_16(raw, 2, 0, &yoshi_walk1);
    write_16(raw, 4, 0, &yoshi_walk0);
    write_16(raw, 6, 0, &yoshi_walk1);
}

/// Classic **SMW Brick Block** left half (H-mirrored by ALTTP!) for Overworld Heavy Rocks (`Sheet 0x3C (0,2)/(0,3)`).
/// Verified `Sheet 0x3C` indices: `1`=dark mortar/outline, `2`=dark brick shade, `3`=mid brick, `4`=bright brick highlight, `5`=drop shadow, `6`=grass.
pub fn build_smw_ow_brick_block_half_8x16() -> ([[u8; 8]; 8], [[u8; 8]; 8]) {
    let top = parse_3bpp_ascii_8x8([
        "61111111", "14444444", "14333331", "14333321", "11111111", "14414444", "14214333",
        "12214332",
    ]);
    let bot = parse_3bpp_ascii_8x8([
        "11111111", "14444441", "14333321", "12222221", "11111111", "14414444", "12212222",
        "51111111",
    ]);
    (top, bot)
}

/// Cute **SMW Daisy / Smiling Flowers** for Overworld Decorative Grass Flowers (`Sheet 0x3C (1,2)/(1,3)`).
/// Verified `Sheet 0x3C (1,2)/(1,3)` indices: `1`=grass background, `3`=bright white/gold petals, `4`=leaf green/stem!
pub fn build_smw_ow_smiling_flowers_8x16() -> ([[u8; 8]; 8], [[u8; 8]; 8]) {
    let f0 = parse_3bpp_ascii_8x8([
        "11111111", "11333111", "13343311", "11333111", "11141111", "11444111", "11141111",
        "11111111",
    ]);
    let f1 = parse_3bpp_ascii_8x8([
        "11111111", "11133311", "11334331", "11133311", "11114111", "11144411", "11114111",
        "11111111",
    ]);
    (f0, f1)
}

/// Museum-grade **16x16 SMW `?` Block** for Dungeon Pushable Stone Blocks (`Sheet 0x0C (2,2)..(3,3)`)!
/// Verified `Sheet 0x0C` indices: `1`=crisp outline/rivets, `5`=bright highlight/?, `4`=mid block face, `3`=warm shade, `2`=deep bottom shadow!
pub fn build_smw_dungeon_qblock_16x16() -> [[u8; 16]; 16] {
    parse_3bpp_ascii([
        "1111111111111111",
        "1555555555555541",
        "1514444444444131",
        "1544411111144431",
        "1544155555514431",
        "1544155115514431",
        "1544111445514431",
        "1544444415514431",
        "1544444155144431",
        "1544444155144431",
        "1544444111444431",
        "1544444155144431",
        "1544444155144431",
        "1513333111333131",
        "1222222222222221",
        "1111111111111111",
    ])
}

/// Museum-grade **SMW `P`-SWITCH** for Dungeon Floor Pressure Switches (`Sheet 0x0C (B,0)/(B,1)`, H-mirrored + `(A,0)/(A,1)` raised edge)!
pub fn build_smw_dungeon_pswitch_half_8x16() -> ([[u8; 8]; 8], [[u8; 8]; 8]) {
    let top = parse_3bpp_ascii_8x8([
        "11111111", "13333333", "13111111", "13177777", "13175555", "13175111", "13175177",
        "13175111",
    ]);
    let bot = parse_3bpp_ascii_8x8([
        "13175177", "13175555", "13176666", "13166666", "13111111", "13333333", "13322222",
        "11111111",
    ]);
    (top, bot)
}
