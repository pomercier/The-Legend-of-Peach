//! Super Mario World Total World Conversion: Toad Town NPCs, Full SMW Bestiary,
//! and Mushroom Kingdom / Dark World / Bowser's Castle Tilesets (v13).
//!
//! Uses exact unshifted ALTTP GFX table indices (`raw_idx = bg_id` for BG sheets `0x00..=0x72`,
//! `raw_idx = 0x73 + sp_id` for Sprite sheets) via `patch_alttp_raw_sheet_pub`, keeping
//! `0x08A000..0x08A2FF` and `0x100000..0x117FFF` 100% untouched and staying strictly within
//! 32 KB LoROM bank boundaries in Banks `$23..$3F`.

use crate::mixer::{
    flip_h_16x16_pub, load_smw_3bpp_sheet, patch_alttp_raw_sheet_pub, read_3bpp_16x16,
    remap_16x16_pub, write_3bpp_16x16_pub, write_3bpp_8x8_pub,
};
use crate::rom::{RomError, SnesRom};
use crate::swapper::SwapSession;

// =============================================================================
// MINI 3BPP CANVAS HELPER (16x16 & 32x32)
// =============================================================================

struct Grid {
    w: usize,
    h: usize,
    px: Vec<u8>,
}

impl Grid {
    fn new(w: usize, h: usize) -> Self {
        Self {
            w,
            h,
            px: vec![0u8; w * h],
        }
    }
    #[inline]
    fn set(&mut self, x: isize, y: isize, c: u8) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            self.px[(y as usize) * self.w + (x as usize)] = c & 7;
        }
    }
    fn rect(&mut self, x0: isize, y0: isize, w: isize, h: isize, c: u8) {
        for y in y0..(y0 + h) {
            for x in x0..(x0 + w) {
                self.set(x, y, c);
            }
        }
    }
    fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, c: u8) {
        for y in ((cy - ry - 1.0).floor() as isize)..=((cy + ry + 1.0).ceil() as isize) {
            for x in ((cx - rx - 1.0).floor() as isize)..=((cx + rx + 1.0).ceil() as isize) {
                let dx = (x as f32 + 0.5 - cx) / rx;
                let dy = (y as f32 + 0.5 - cy) / ry;
                if dx * dx + dy * dy <= 1.0 {
                    self.set(x, y, c);
                }
            }
        }
    }
    fn ellipse_o(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, fill: u8, b: u8) {
        self.ellipse(cx, cy, rx, ry, b);
        if rx > 1.2 && ry > 1.2 {
            self.ellipse(cx, cy, rx - 1.1, ry - 1.1, fill);
        }
    }
    fn tri(&mut self, x0: isize, y0: isize, x1: isize, y1: isize, x2: isize, y2: isize, c: u8) {
        let edge = |ax: isize, ay: isize, bx: isize, by: isize, px: isize, py: isize| {
            (px - ax) * (by - ay) - (py - ay) * (bx - ax)
        };
        for y in y0.min(y1).min(y2)..=y0.max(y1).max(y2) {
            for x in x0.min(x1).min(x2)..=x0.max(x1).max(x2) {
                let e0 = edge(x0, y0, x1, y1, x, y);
                let e1 = edge(x1, y1, x2, y2, x, y);
                let e2 = edge(x2, y2, x0, y0, x, y);
                if (e0 >= 0 && e1 >= 0 && e2 >= 0) || (e0 <= 0 && e1 <= 0 && e2 <= 0) {
                    self.set(x, y, c);
                }
            }
        }
    }
    fn outline(&mut self, col: u8) {
        let orig = self.px.clone();
        for y in 0..self.h {
            for x in 0..self.w {
                if orig[y * self.w + x] != 0 {
                    continue;
                }
                for (dx, dy) in [(-1isize, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = x as isize + dx;
                    let ny = y as isize + dy;
                    if nx >= 0 && ny >= 0 && (nx as usize) < self.w && (ny as usize) < self.h {
                        let v = orig[(ny as usize) * self.w + (nx as usize)];
                        if v != 0 && v != col {
                            self.px[y * self.w + x] = col;
                            break;
                        }
                    }
                }
            }
        }
    }
    fn to_16x16(&self) -> [[u8; 16]; 16] {
        let mut out = [[0u8; 16]; 16];
        for y in 0..16.min(self.h) {
            for x in 0..16.min(self.w) {
                out[y][x] = self.px[y * self.w + x];
            }
        }
        out
    }
    fn slice_16x16(&self, ox: usize, oy: usize) -> [[u8; 16]; 16] {
        let mut out = [[0u8; 16]; 16];
        for y in 0..16 {
            for x in 0..16 {
                if ox + x < self.w && oy + y < self.h {
                    out[y][x] = self.px[(oy + y) * self.w + (ox + x)];
                }
            }
        }
        out
    }
}

/// Standard SMW -> ALTTP 3bpp Sprite LUT (universal across Pal 1 Red, Pal 3 Blue, Pal 4/6 Green/Gold):
/// SMW: 0=trans, 1=white, 2=outline, 3=light, 4=mid, 5=accent, 6=dark, 7=highlight
/// ALTTP: 0=trans, 1=white, 2=accent/red/blue, 3=orange/shade, 4=skin/gold, 5=outline, 6=primary, 7=dark primary
const LUT_SMW_STD: [u8; 8] = [0, 1, 5, 4, 6, 2, 7, 3];
const LUT_SMW_ALT: [u8; 8] = [0, 1, 5, 6, 4, 3, 7, 2];

fn smw_tile(sheet: &[u8], tx: usize, ty: usize, lut: &[u8; 8]) -> [[u8; 16]; 16] {
    remap_16x16_pub(&read_3bpp_16x16(sheet, tx, ty), lut)
}

// =============================================================================
// PART A: TOAD TOWN (KAKARIKO VILLAGE & WORLDWIDE NPCs — 6 VANILLA SHEETS!)
//   VERIFIED LIVE ALTTP NPC CGRAM PALETTES (NEVER USE INDEX 2 ON TOAD FACES!):
//   - Pal 3 (`PalRow[02]` @ 0x0DD254, Red Toads):
//       1=Pure White (#F8F8F8), 2=Lime-Yellow (#D0D850 - AVOID ON FACE!),
//       3=Warm Tan/Boots (#D08048), 4=Peach Skin (#F8C088),
//       5=Charcoal Outline (#484848), 6=Vibrant Red (#F83030), 7=Deep Crimson (#B81010)
//   - Pal 4 (`PalRow[03]` @ 0x0DD272, Blue Toads & SMW Kamek!):
//       1=Pure White (#F8F8F8), 2=Pink Magic Gem (#F888B0),
//       3=Warm Tan (#D07840), 4=Golden-Yellow Snout (#F8D080),
//       5=Crisp Black Outline (#303030), 6=SMW Royal Blue (#4090F8), 7=Deep Royal Blue (#0048A8)
// =============================================================================

#[derive(Clone, Copy)]
enum ToadStyle {
    ClassicFront,
    ClassicSide,
    ClassicBack,
    ToadetteFront,
    ToadetteSide,
    ToadsworthFront,
    ShopkeeperFront,
}

/// Museum-grade 16-bit Nintendo SMAS / Super Mario RPG Toad Head (16x16, BUG 05 OVERHAUL!).
/// - Plump, genuinely rounded shaded mushroom dome (NOT blocky/flat-top!)
/// - 3D shaded red/blue cap spots (`6` bright + `7` shaded underside!)
/// - Cute rounded peach face with shiny vertical Nintendo oval eyes (`5` pupil + `1` top glint!)
/// - Warm subtle smile (`3` warm tan — ZERO lime-green `2` mouth artifact!)
fn build_toad_head_16x16(style: ToadStyle) -> [[u8; 16]; 16] {
    let (cap_bg, spot_hi, spot_lo, face_col) = match style {
        ToadStyle::ToadetteFront | ToadStyle::ToadetteSide => (6u8, 1u8, 1u8, 4u8),
        ToadStyle::ToadsworthFront => (4u8, 3u8, 7u8, 4u8),
        ToadStyle::ShopkeeperFront => (1u8, 6u8, 7u8, 4u8),
        _ => (1u8, 6u8, 7u8, 4u8),
    };
    let rows: [&str; 16] = match style {
        ToadStyle::ClassicBack => [
            ".....555555.....",
            "...55CCSSCC55...",
            "..5CCSSSSSSCC5..",
            ".5SCCSSSSSSCCS5.",
            "5SSCCCSSSSCCCSS5",
            "5SSCCCDDDDCCCSS5",
            "5SDCCCCCCCCCCDS5",
            "5DCCCCCCCCCCCCD5",
            ".5CCCCCCCCCCCC5.",
            ".53CCCCCCCCCC35.",
            "..55CCSSSSCC55..",
            "...5CCSDDSCC5...",
            "....55CCCC55....",
            ".....555555.....",
            "................",
            "................",
        ],
        ToadStyle::ClassicSide | ToadStyle::ToadetteSide => [
            ".....555555.....",
            "...55CCSSCC55...",
            "..5CCSSSSSSCC5..",
            ".5SSCCSSSSCCCS5.",
            "5SSCCCSSSSCCCSS5",
            "5SDCCCCDDCCCCDS5",
            "5DCCCCCCCCCCCCD5",
            ".5CCCCCCCCCCCC5.",
            ".535555555553C5.",
            "..53FFFFFFFF55..",
            "..5FF15FFFFF5...",
            "..5FF55FFFFF5...",
            "..5FFFFFFFFF5...",
            "...53FF33FF5....",
            "....55FFFF5.....",
            "......5555......",
        ],
        ToadStyle::ToadsworthFront => [
            ".....555555.....",
            "...55CCSSCC55...",
            "..5CCSSSSSSCC5..",
            ".5SCCSSSSSSCCS5.",
            "5SSCCCSSSSCCCSS5",
            "5SDCCCCDDCCCCDS5",
            "5DCCCCCCCCCCCCD5",
            ".5CCCCCCCCCCCC5.",
            ".53555555555535.",
            "..53FFFFFFFF35..",
            "..5F155FF551F5..",
            "..5F515FF515F5..",
            "..5FF111111FF5..",
            "...53F1111F35...",
            "....55FFFF55....",
            "......5555......",
        ],
        _ => [
            ".....555555.....",
            "...55CCSSCC55...",
            "..5CCSSSSSSCC5..",
            ".5SCCSSSSSSCCS5.",
            "5SSCCCSSSSCCCSS5",
            "5SDCCCCDDCCCCDS5",
            "5DCCCCCCCCCCCCD5",
            ".5CCCCCCCCCCCC5.",
            ".53555555555535.",
            "..53FFFFFFFF35..",
            "..5FF15FF51FF5..",
            "..5FF55FF55FF5..",
            "..5FFFFFFFFFF5..",
            "...53FF33FF35...",
            "....55FFFF55....",
            "......5555......",
        ],
    };
    let mut px = [[0u8; 16]; 16];
    for (y, row) in rows.iter().enumerate() {
        for (x, ch) in row.bytes().take(16).enumerate() {
            px[y][x] = match ch {
                b'.' => 0,
                b'C' => cap_bg,
                b'S' => spot_hi,
                b'D' => spot_lo,
                b'F' => face_col,
                b'1' => 1,
                b'3' => 3,
                b'5' => 5,
                _ => 0,
            };
        }
    }
    if matches!(style, ToadStyle::ToadetteFront | ToadStyle::ToadetteSide) {
        // Adorable shaded twin Toadette pigtails
        for &(cx, cy) in &[(1usize, 10usize), (14, 10)] {
            px[cy][cx] = 6;
            px[cy + 1][cx] = 7;
            px[cy.saturating_sub(1)][cx] = 5;
            px[cy + 2][cx] = 5;
            if cx > 0 {
                px[cy][cx - 1] = 5;
                px[cy + 1][cx - 1] = 5;
            }
            if cx + 1 < 16 {
                px[cy][cx + 1] = 5;
                px[cy + 1][cx + 1] = 5;
            }
        }
    }
    px
}

/// Museum-grade 16-bit Nintendo SMAS / Super Mario RPG Toad Body (16x16, BUG 05 OVERHAUL!).
/// Tailored shaded open vest (`6`/`7`) with golden-tan trim (`3`) over crisp white shirt (`1`),
/// adorable peach arms (`4`), plump white Toad shorts (`1`), and tidy chestnut boots (`3`).
fn build_toad_body_16x16(vest_hi: u8, vest_lo: u8, stride: isize, dress: bool) -> [[u8; 16]; 16] {
    let rows: [&str; 16] = if dress {
        [
            "................",
            "....55555555....",
            "...5VV3113VV5...",
            "..54VV3113VV45..",
            "..545V3113V545..",
            "...55VD11DV55...",
            "...5VVD11DVV5...",
            "..5VVVD11DVVV5..",
            "..5VDD1111DDV5..",
            "..511111111115..",
            "...5555555555...",
            "...5335..5335...",
            "..53335..53335..",
            "..55555..55555..",
            "................",
            "................",
        ]
    } else if stride != 0 {
        [
            "................",
            "....55555555....",
            "...5VV3113VV5...",
            "..54VV3113VV45..",
            "..545V3113V545..",
            "...55VD11DV55...",
            "...5VVD11DVV5...",
            "...5111111115...",
            "...5111111115...",
            "...5111551115...",
            "..53335..5335...",
            "..53335...5335..",
            "..55555...5555..",
            "................",
            "................",
            "................",
        ]
    } else {
        [
            "................",
            "....55555555....",
            "...5VV3113VV5...",
            "..54VV3113VV45..",
            "..545V3113V545..",
            "...55VD11DV55...",
            "...5VVD11DVV5...",
            "...5111111115...",
            "...5111111115...",
            "...5111551115...",
            "..53335..53335..",
            "..53335..53335..",
            "..55555..55555..",
            "................",
            "................",
            "................",
        ]
    };
    let mut px = [[0u8; 16]; 16];
    for (y, row) in rows.iter().enumerate() {
        for (x, ch) in row.bytes().take(16).enumerate() {
            px[y][x] = match ch {
                b'.' => 0,
                b'V' => vest_hi,
                b'D' => vest_lo,
                b'1' => 1,
                b'3' => 3,
                b'4' => 4,
                b'5' => 5,
                _ => 0,
            };
        }
    }
    px
}

/// Transparent ground tile for `Sprite 0x35` (`raw 0xBD (6,0)` = `$86`, drawn mirrored at `dy=+15`).
/// Replaces the v0.5.0 bug where Toad's Back Head sat in `(6,0)` and rendered as a massive dark-grey lump,
/// letting ALTTP's clean native sprite ground shadow render beneath Toad's shoes with zero double-shadowing!
fn build_toad_ground_shadow_16x16() -> [[u8; 16]; 16] {
    [[0u8; 16]; 16]
}

/// Authentic Super Mario World Blue Magikoopa **Kamek** (BUG 04 OVERHAUL!).
/// Calibrated strictly to Global Sprite Pal 4 (`PalRow[03]` @ 0x0DD272):
/// - `1` = Pure White (`#F8F8F8`): Round glasses lenses, crisp wizard robe trim, magic sparkle core
/// - `2` = Magic Pink (`#F888B0`): Glowing wand crystal & floating magic sparkles!
/// - `3` = Warm Tan (`#D07840`): Beak underside shade & wooden wand staff
/// - `4` = Golden Yellow (`#F8D080`): Iconic SMW Koopa snout & collar clasp
/// - `5` = Crisp Black (`#303030`): Bold 16-bit Nintendo outline, spectacles frame & pupils
/// - `6` = Vibrant SMW Royal Blue (`#4090F8`): Pointed Magikoopa wizard hat & flowing robe!
/// - `7` = Deep SMW Royal Blue (`#0048A8`): Hat brim shadow & deep robe pleats!
pub fn build_kamek_magikoopa_smw_16x16(frame: usize) -> ([[u8; 16]; 16], [[u8; 16]; 16]) {
    // 1. Kamek Head (16x16): Authentic SMW GFX03 tilted pointed blue wizard hat, big round white
    //    spectacles, expressive dark pupils, and friendly-wicked yellow Koopa snout!
    let head_rows: [&str; 16] = [
        "........555.....",
        ".......56665....",
        "......566675....",
        ".....5666675....",
        "....56666775....",
        "...566666775....",
        "..55777777755...",
        ".5666666666775..",
        ".5555555555555..",
        "..55115551155...",
        "..511555511545..",
        "..5115555115445.",
        "..5555444444445.",
        "...54444444335..",
        "....533333335...",
        ".....5555555....",
    ];
    // 2. Kamek Robe & Magic Wand (16x16): Flowing royal-blue wizard cloak with crisp white trim,
    //    golden chest clasp, cute boots, and glowing SMW magic wand (Frame 0 ready, Frame 1 raised + sparkle!)
    let body_rows: [&str; 16] = if frame == 0 {
        [
            ".....555555.....",
            "....51144115....",
            "...5661661665...",
            "..566616616665..",
            ".56676167167665.",
            ".54566177166545.",
            "..5566177166552.",
            "...5661771665315",
            "...566177166532.",
            "..5667177176635.",
            "..566717717665..",
            "..511111111115..",
            "...5555555555...",
            "...5335..5335...",
            "...5555..5555...",
            "................",
        ]
    } else {
        [
            ".....555555..1..",
            "....51144115121.",
            "...566166166525.",
            "..5666166166635.",
            ".56676167167635.",
            ".54566177166545.",
            "..556617716655..",
            "...5661771665...",
            "...5661771665...",
            "..566717717665..",
            "..566717717665..",
            "..511111111115..",
            "...5555555555...",
            "...5335..5335...",
            "...5555..5555...",
            "................",
        ]
    };
    let parse = |rows: [&str; 16]| -> [[u8; 16]; 16] {
        let mut px = [[0u8; 16]; 16];
        for (y, row) in rows.iter().enumerate() {
            for (x, ch) in row.bytes().take(16).enumerate() {
                px[y][x] = match ch {
                    b'.' => 0,
                    b'1' => 1,
                    b'2' => 2,
                    b'3' => 3,
                    b'4' => 4,
                    b'5' => 5,
                    b'6' => 6,
                    b'7' => 7,
                    _ => 0,
                };
            }
        }
        px
    };
    (parse(head_rows), parse(body_rows))
}

fn build_kamek_magikoopa_16x16(frame: usize) -> ([[u8; 16]; 16], [[u8; 16]; 16]) {
    build_kamek_magikoopa_smw_16x16(frame)
}

fn build_starman_fairy_16x16(frame: usize) -> [[u8; 16]; 16] {
    let mut g = Grid::new(16, 16);
    // Gleaming SMW Super Star Fairy with cute vertical eyes & wing sparkles!
    g.tri(8, 1, 5, 8, 11, 8, 4);
    g.tri(1, 6, 15, 6, 8, 11, 4);
    g.tri(8, 7, 3, 14, 8, 11, 4);
    g.tri(8, 7, 13, 14, 8, 11, 4);
    g.ellipse(8.0, 8.0, 3.2, 3.0, if frame == 0 { 1 } else { 4 });
    g.rect(6, 6, 1, 3, 5);
    g.rect(9, 6, 1, 3, 5);
    g.outline(5);
    g.to_16x16()
}

fn build_baby_yoshi_cute_16x16(hop: isize) -> [[u8; 16]; 16] {
    let mut g = Grid::new(16, 16);
    g.ellipse_o(8.0, (8 - hop) as f32, 5.5, 5.2, 6, 5);
    g.ellipse(10.5, (7 - hop) as f32, 3.5, 3.2, 6); // round cute snout
    g.ellipse(6.5, (10 - hop) as f32, 3.2, 2.8, 1); // white belly
                                                    // Big cute Yoshi eyes on top of head!
    g.rect(6, 2 - hop, 3, 4, 1);
    g.rect(7, 3 - hop, 2, 3, 5);
    g.set(7, 3 - hop, 1);
    // Tiny red back crests & boots
    g.rect(2, 5 - hop, 2, 2, 2);
    g.rect(2, 8 - hop, 2, 2, 2);
    g.ellipse_o(6.0, 13.5, 2.2, 1.8, 2, 5);
    g.ellipse_o(10.5, 13.5, 2.2, 1.8, 2, 5);
    g.outline(5);
    g.to_16x16()
}

fn patch_toad_town_npcs(
    rom_z: &mut SnesRom,
    sess: &mut SwapSession,
    free_cursor: &mut usize,
    free_end: usize,
) -> Result<usize, RomError> {
    let mut ops = 0usize;
    let th_f = build_toad_head_16x16(ToadStyle::ClassicFront);
    let th_s = build_toad_head_16x16(ToadStyle::ClassicSide);
    let th_b = build_toad_head_16x16(ToadStyle::ClassicBack);
    let tt_f = build_toad_head_16x16(ToadStyle::ToadetteFront);
    let tt_s = build_toad_head_16x16(ToadStyle::ToadetteSide);
    let tw_f = build_toad_head_16x16(ToadStyle::ToadsworthFront);
    let ts_f = build_toad_head_16x16(ToadStyle::ShopkeeperFront);

    let tb_1 = build_toad_body_16x16(6, 7, 0, false);
    let tb_2 = build_toad_body_16x16(6, 7, 1, false);
    let tb_alt = build_toad_body_16x16(7, 6, 0, false);
    let td_1 = build_toad_body_16x16(6, 7, 0, true);
    let td_2 = build_toad_body_16x16(7, 6, 1, true);
    let toad_shadow = build_toad_ground_shadow_16x16();
    let (km_h0, km_b0) = build_kamek_magikoopa_16x16(0);
    let (km_h1, km_b1) = build_kamek_magikoopa_16x16(1);
    let sf0 = build_starman_fairy_16x16(0);
    let sf1 = build_starman_fairy_16x16(1);
    let by0 = build_baby_yoshi_cute_16x16(0);
    let by1 = build_baby_yoshi_cute_16x16(1);

    // 1. Kakariko Villagers Main (sp:4F -> raw 0xC2): Exact verified vanilla Head/Body cell layout!
    //    Row 0: (0)=HeadF, (2)=HeadS, (4)=HeadF, (6)=BODY, (8)=HeadS, (10)=BODY, (12)=ToadetteF, (14)=ToadetteS
    //    Row 2: (0)=BODY, (2)=ToadsworthF, (4)=ToadetteF, (6)=HeadB, (8)=ToadetteS, (10)=HeadF, (12)=HeadB, (14)=HeadS
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0xC2,
        "Village Cocorico → TOAD TOWN 1/4 (sp:4F) : Toads, Toadettes & Papy Champi 16-bit SMAS/SMRPG",
        free_cursor,
        free_end,
        &|raw| {
            let row0 = [&th_f, &th_s, &th_f, &tb_1, &th_s, &tb_2, &tt_f, &tt_s];
            let row2 = [&tb_1, &tw_f, &tt_f, &th_b, &tt_s, &th_f, &th_b, &th_s];
            for i in 0..8 {
                write_3bpp_16x16_pub(raw, i * 2, 0, row0[i]);
                write_3bpp_16x16_pub(raw, i * 2, 2, row2[i]);
            }
        },
    )? {
        ops += 1;
    }

    // 2. Room 0x0103 Toad (`Sprite 0x35`) & Villagers 2 (sp:4A -> raw 0xBD):
    //    EXACT VERIFIED `Sprite 0x35` OAM MAP ($05:E543):
    //      Row 0: (0)=HeadF ($80), (2)=BodyF1 ($82), (4)=BodyF2 ($84), (6)=SubtleGroundShadow ($86!),
    //             (8)=HeadF ($88), (10)=Body1 ($8A), (12)=Body2 ($8C)
    //      Row 2: (0)=Body1, (2)=Body2, (4)=Body1, (6)=Body2, (8)=HeadS, (10)=BodyAlt, (12)=ShopHeadF
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0xBD,
        "Maison Toad 0x0103 & Villageois 2/4 (sp:4A) : Toad 16-bit exact ($80 tête, $82/$84 corps, $86 ombre douce)",
        free_cursor,
        free_end,
        &|raw| {
            let row0 = [&th_f, &tb_1, &tb_2, &toad_shadow, &th_f, &tb_1, &tb_2];
            let row2 = [&tb_1, &tb_2, &tb_1, &tb_2, &th_s, &tb_alt, &ts_f];
            for i in 0..7 {
                write_3bpp_16x16_pub(raw, i * 2, 0, row0[i]);
                write_3bpp_16x16_pub(raw, i * 2, 2, row2[i]);
            }
        },
    )? {
        ops += 1;
    }

    // 2b. Surgical OAM & Palette Patches for BUG 04 (SMW Blue Kamek Shopkeeper) & BUG 05 (Toad Alignment):
    //     - Shopkeeper (`Sprite 0xBB` @ 0x030BF9): ORA #$0C (Green) -> ORA #$08 (Global Pal 4 = SMW Royal Blue!)
    //     - Shopkeeper (`$1E:EF54` @ 0x0F6F54):    LDA #$07 -> LDA #$09 (Pal 4 + NT 1 = SMW Royal Blue!)
    //     - Shopkeeper OAM Table (`$05:F8F5` @ 0x02F8F5, 32B):
    //         Frame 0: Head tile $00 @ (0, -12, pal 4 Blue), Body tile $06 @ (0, +2, pal 4 Blue) — ZERO digit overlap!
    //         Frame 1: Head tile $00 @ (0, -11, pal 4 Blue), Body tile $26 @ (0, +2, pal 4 Blue) — wand raised!
    //     - Room 0x0103 Toad (`Sprite 0x35` @ 0x02E544): Head dy = -4 (0xFC) -> dy = -9 (0xF7) (clean 13px separation!)
    //     - Innkeeper Toad (`Sprite 0xBC` @ 0x0D7866 & 0x0D787E): Head dy = -9 (0xF7) -> dy = -11 (0xF5)!
    if 0x030BFA < rom_z.data.len() && rom_z.data[0x030BF9] == 0x09 && rom_z.data[0x030BFA] == 0x0C {
        sess.swap_custom_bytes(
            rom_z,
            0x030BFA,
            0x030BFA,
            vec![0x08],
            "Palette Marchand Kamek (0x030BFA) : Vert 0x0C -> Bleu Royal SMW 0x08 (PalRow 3)"
                .into(),
            crate::swapper::SwapKind::CodePatch { byte_count: 1 },
        )?;
        ops += 1;
    }
    if 0x0F6F55 < rom_z.data.len() && rom_z.data[0x0F6F53] == 0xA9 && rom_z.data[0x0F6F54] == 0x07 {
        sess.swap_custom_bytes(
            rom_z,
            0x0F6F54,
            0x0F6F54,
            vec![0x09],
            "Palette Marchand Kamek État 0 (0x0F6F54) : Bleu Royal SMW 0x09 (Pal 4 + NT1)".into(),
            crate::swapper::SwapKind::CodePatch { byte_count: 1 },
        )?;
        ops += 1;
    }
    if 0x02F915 <= rom_z.data.len() && rom_z.data[0x02F8F9] == 0x00 && rom_z.data[0x02F901] == 0x10
    {
        // NOTE: `Sprite_DrawMultiple_quantity_preset` ($05:DF6F) executes `EOR $05` (where $05 = $0F50,X = 0x09)!
        // Keeping table `flags = 0x00` computes `0x00 ^ 0x09 = 0x09` => OBJ Pal 4 (0x0DD272 SMW Royal Blue) + NT1!
        #[rustfmt::skip]
        let kamek_oam: Vec<u8> = vec![
            // Frame 0: Head (x=0, y=-12, tile=$00, flags=$00 -> ^$09 = Pal 4 Blue), Body (x=0, y=+2, tile=$06, flags=$00)
            0x00, 0x00, 0xF4, 0xFF, 0x00, 0x00, 0x00, 0x02,
            0x00, 0x00, 0x02, 0x00, 0x06, 0x00, 0x00, 0x02,
            // Frame 1: Head (x=0, y=-11, tile=$00, flags=$00 -> ^$09 = Pal 4 Blue), Body (x=0, y=+2, tile=$26 wand up, flags=$00)
            0x00, 0x00, 0xF5, 0xFF, 0x00, 0x00, 0x00, 0x02,
            0x00, 0x00, 0x02, 0x00, 0x26, 0x00, 0x00, 0x02,
        ];
        sess.swap_custom_bytes(
            rom_z,
            0x02F8F5,
            0x02F8F5,
            kamek_oam,
            "Table OAM Marchand Kamek ($05:F8F5, 32 B) : Robe propre $06/$26 (0 chiffres, flags=0x00 ^ 0x09 = Bleu Royal SMW)".into(),
            crate::swapper::SwapKind::CodePatch { byte_count: 32 },
        )?;
        ops += 1;
    }
    if 0x02E544 < rom_z.data.len() && rom_z.data[0x02E544] == 0xFC && rom_z.data[0x02E545] == 0x80 {
        sess.swap_custom_bytes(
            rom_z,
            0x02E544,
            0x02E544,
            vec![0xF7], // Head dy = -9px (Body at +4px -> crisp 13px separation!)
            "Alignement OAM Tête Toad Room 0x0103 ($05:E544) : dy=-4 -> dy=-9 (proportions 16-bit propres)".into(),
            crate::swapper::SwapKind::CodePatch { byte_count: 1 },
        )?;
        ops += 1;
    }
    for &inn_y in &[0x0D7866usize, 0x0D787Eusize] {
        if inn_y < rom_z.data.len() && rom_z.data[inn_y] == 0xF7 {
            sess.swap_custom_bytes(
                rom_z,
                inn_y,
                inn_y,
                vec![0xF5], // Head dy = -11px above body at 0px
                format!("Alignement OAM Tête Toad Aubergiste (0x{inn_y:06X}) : dy=-9 -> dy=-11"),
                crate::swapper::SwapKind::CodePatch { byte_count: 1 },
            )?;
            ops += 1;
        }
    }

    // 3. Shopkeeper / Fortune Teller / Chest Game (sp:4D -> raw 0xC0) — keep sign at col 2..3 intact!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0xC0,
        "Boutiques & Maisons Toad (sp:4D) ← Toad Marchand & Magikoopa Voyant SMW",
        free_cursor,
        free_end,
        &|raw| {
            write_3bpp_16x16_pub(raw, 0, 0, &ts_f);
            write_3bpp_16x16_pub(raw, 0, 2, &tb_alt);
            write_3bpp_16x16_pub(raw, 4, 0, &th_f);
            write_3bpp_16x16_pub(raw, 4, 2, &tb_1);
            write_3bpp_16x16_pub(raw, 6, 0, &km_h0);
            write_3bpp_16x16_pub(raw, 6, 2, &km_b0);
            write_3bpp_16x16_pub(raw, 8, 0, &km_h1);
            write_3bpp_16x16_pub(raw, 8, 2, &km_b1);
            write_3bpp_16x16_pub(raw, 10, 0, &tt_f);
            write_3bpp_16x16_pub(raw, 10, 2, &td_1);
            write_3bpp_16x16_pub(raw, 12, 0, &tw_f);
            write_3bpp_16x16_pub(raw, 12, 2, &tb_2);
            write_3bpp_16x16_pub(raw, 14, 0, &th_s);
            write_3bpp_16x16_pub(raw, 14, 2, &tb_1);
        },
    )? {
        ops += 1;
    }

    // 4. Flute Boy & Woodland Animals (sp:4E -> raw 0xC1) -> Flute Toad & Baby Yoshis!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0xC1,
        "Bosquet Flûte & Animaux (sp:4E) ← Toad Musicien & Bébés Yoshis SMW",
        free_cursor,
        free_end,
        &|raw| {
            write_3bpp_16x16_pub(raw, 0, 0, &th_f);
            write_3bpp_16x16_pub(raw, 0, 2, &tb_1);
            write_3bpp_16x16_pub(raw, 2, 0, &th_s);
            write_3bpp_16x16_pub(raw, 2, 2, &tb_2);
            write_3bpp_16x16_pub(raw, 4, 0, &tt_f);
            write_3bpp_16x16_pub(raw, 4, 2, &td_1);
            write_3bpp_16x16_pub(raw, 6, 0, &by0);
            write_3bpp_16x16_pub(raw, 6, 2, &by1);
            write_3bpp_16x16_pub(raw, 8, 0, &by0);
            write_3bpp_16x16_pub(raw, 10, 0, &by1);
            write_3bpp_16x16_pub(raw, 12, 0, &flip_h_16x16_pub(&by0));
            write_3bpp_16x16_pub(raw, 14, 0, &flip_h_16x16_pub(&by1));
            write_3bpp_16x16_pub(raw, 8, 2, &by1);
            write_3bpp_16x16_pub(raw, 10, 2, &by0);
            write_3bpp_16x16_pub(raw, 12, 2, &sf0);
            write_3bpp_16x16_pub(raw, 14, 2, &sf1);
        },
    )? {
        ops += 1;
    }

    // 5. Running Boy, Sweeping Lady (Room 0x0103!) & Town Scouts (sp:50 -> raw 0xC3): Exact verified vanilla layout!
    //    Row 0: (0)=HeadF, (2)=BODY, (4)=HeadF, (6)=HeadB, (8)=HeadS, (10)=BODY_SWEEP, (12)=BODY_S1, (14)=BODY_S2
    //    Row 2: (0)=HeadB, (2)=HeadS, (4)=BODY_W1, (6)=BODY_W2, (8)=BODY_S, (10)=BabyYoshi0, (12)=BabyYoshi1, (14)=BODY
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0xC3,
        "Village Cocorico → TOAD TOWN 3/4 (sp:50) : Toads Aubergistes & Éclaireurs SMW (layout OAM exact)",
        free_cursor,
        free_end,
        &|raw| {
            let row0 = [&tw_f, &tb_1, &th_f, &th_b, &th_s, &tb_1, &tb_2, &td_1];
            let row2 = [&th_b, &th_s, &tb_1, &tb_2, &tb_alt, &by0, &by1, &td_2];
            for i in 0..8 {
                write_3bpp_16x16_pub(raw, i * 2, 0, row0[i]);
                write_3bpp_16x16_pub(raw, i * 2, 2, row2[i]);
            }
        },
    )? {
        ops += 1;
    }

    // 6. Witch & Great Fairy Assistants (sp:4C -> raw 0xBF) -> Magikoopa Alchemist & Super Star Fairy!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0xBF,
        "Alchimiste & Fée Étoile (sp:4C) ← Magikoopa Kamek & Fée Super Étoile SMW",
        free_cursor,
        free_end,
        &|raw| {
            write_3bpp_16x16_pub(raw, 0, 0, &km_h0);
            write_3bpp_16x16_pub(raw, 0, 2, &km_b0);
            write_3bpp_16x16_pub(raw, 2, 0, &km_h1);
            write_3bpp_16x16_pub(raw, 2, 2, &km_b1);
            write_3bpp_16x16_pub(raw, 12, 0, &sf0);
            write_3bpp_16x16_pub(raw, 12, 2, &sf1);
        },
    )? {
        ops += 1;
    }

    Ok(ops)
}

// =============================================================================
// PART B: MASSIVE SMW BESTIARY EXPANSION (16 FULL VANILLA SHEETS!)
// =============================================================================

fn build_chain_chomp_16x16(open: bool) -> [[u8; 16]; 16] {
    // Iconic jet-black SMW Chain Chomp with razor-sharp white teeth & fierce eyes!
    let mut g = Grid::new(16, 16);
    g.ellipse_o(8.0, 8.0, 6.8, 6.8, 5, 5);
    g.ellipse(5.5, 5.0, 2.2, 1.8, 7); // glossy dark-steel shine
    g.set(5, 4, 1);
    // Fierce wide eyes
    g.rect(4, 5, 3, 3, 1);
    g.rect(9, 5, 3, 3, 1);
    g.set(5, 6, 5);
    g.set(10, 6, 5);
    // Saw-toothed chomping jaws!
    if open {
        g.rect(2, 9, 12, 5, 2); // crimson mouth interior
        for &tx in &[3isize, 6, 9, 12] {
            g.set(tx, 9, 1);
            g.set(tx, 10, 1);
            g.set(tx + 1, 13, 1);
        }
    } else {
        g.rect(3, 10, 10, 2, 1);
        g.set(5, 10, 5);
        g.set(8, 11, 5);
        g.set(11, 10, 5);
    }
    g.outline(5);
    g.to_16x16()
}

fn build_bill_blaster_turret_16x16(angle: usize) -> [[u8; 16]; 16] {
    // Iconic SMW Bill Blaster Cannon Turret (replacing Beamos laser eye!)
    let mut g = Grid::new(16, 16);
    g.rect(2, 2, 12, 12, 5);
    g.rect(3, 3, 10, 10, 7);
    g.rect(4, 3, 2, 10, 1); // metallic sheen
                            // Round cannon muzzle pointing in direction `angle % 4`
    let (mx, my) = match angle % 4 {
        0 => (8.0, 10.0),
        1 => (10.5, 8.0),
        2 => (8.0, 5.5),
        _ => (5.5, 8.0),
    };
    g.ellipse_o(mx, my, 3.6, 3.6, 5, 1);
    g.set(mx as isize, my as isize, 2); // glowing fuse core!
                                        // Skull crest emblem on top
    g.rect(6, 2, 4, 2, 4);
    g.outline(5);
    g.to_16x16()
}

fn build_monty_mole_16x16(pop: bool) -> [[u8; 16]; 16] {
    let mut g = Grid::new(16, 16);
    if pop {
        g.ellipse_o(8.0, 9.0, 6.0, 5.8, 3, 5);
        g.ellipse(8.0, 11.0, 4.2, 3.5, 4); // tan belly & muzzle
        g.rect(5, 6, 2, 3, 1);
        g.set(6, 7, 5);
        g.rect(9, 6, 2, 3, 1);
        g.set(9, 7, 5);
        g.rect(7, 9, 2, 2, 2); // pink mole nose!
        g.rect(6, 12, 4, 1, 1); // buck teeth
        g.rect(2, 9, 2, 3, 4); // digging claws
        g.rect(12, 9, 2, 3, 4);
    } else {
        // Peeking out of dirt mound!
        g.ellipse_o(8.0, 11.5, 6.2, 3.5, 3, 5);
        g.ellipse_o(8.0, 7.0, 4.0, 3.2, 4, 5);
        g.set(6, 6, 5);
        g.set(10, 6, 5);
        g.set(8, 8, 2);
    }
    g.outline(5);
    g.to_16x16()
}

fn build_swooper_bat_16x16(flap: bool) -> [[u8; 16]; 16] {
    let mut g = Grid::new(16, 16);
    // Iconic cute round SMW Swooper cave bat!
    if flap {
        g.tri(1, 3, 7, 7, 3, 12, 6);
        g.tri(15, 3, 9, 7, 13, 12, 6);
    } else {
        g.tri(1, 10, 7, 7, 4, 14, 6);
        g.tri(15, 10, 9, 7, 12, 14, 6);
    }
    g.ellipse_o(8.0, 8.5, 4.0, 4.2, 6, 5);
    g.tri(5, 3, 4, 6, 7, 6, 6); // ears
    g.tri(11, 3, 9, 6, 12, 6, 6);
    g.rect(6, 7, 2, 2, 1);
    g.set(6, 8, 5);
    g.rect(9, 7, 2, 2, 1);
    g.set(10, 8, 5);
    g.set(7, 10, 1);
    g.set(9, 10, 1); // fangs
    g.outline(5);
    g.to_16x16()
}

fn build_sparky_firebar_16x16(frame: usize) -> [[u8; 16]; 16] {
    let mut g = Grid::new(16, 16);
    g.ellipse_o(8.0, 8.0, 5.5, 5.5, 2, 5);
    g.ellipse(8.0, 8.0, 3.8, 3.8, 4);
    // Electric/Flame spikes
    let off = if frame == 0 { 0isize } else { 2 };
    for &(x, y) in &[
        (8, 1),
        (8, 14),
        (1, 8),
        (14, 8),
        (3 + off, 3),
        (13 - off, 13),
    ] {
        g.set(x, y, 1);
    }
    // Cute SMW Hothead / Sparky eyes!
    g.rect(5, 6, 2, 3, 1);
    g.set(6, 7, 5);
    g.rect(9, 6, 2, 3, 1);
    g.set(9, 7, 5);
    g.outline(5);
    g.to_16x16()
}

fn build_ninji_16x16(jump: bool) -> [[u8; 16]; 16] {
    let mut g = Grid::new(16, 16);
    let dy = if jump { -1isize } else { 0 };
    // Classic jet-black star-shaped SMB2/SMW Ninji!
    g.tri(8, 1 + dy, 3, 11 + dy, 13, 11 + dy, 5);
    g.tri(2, 6 + dy, 14, 6 + dy, 8, 12 + dy, 5);
    g.rect(5, 6 + dy, 2, 2, 1);
    g.rect(9, 6 + dy, 2, 2, 1);
    g.rect(7, 9 + dy, 2, 2, 2); // red ninja buttons!
    g.rect(4, 12, 3, 2, 2);
    g.rect(9, 12, 3, 2, 2);
    g.outline(1);
    g.to_16x16()
}

fn build_ice_koopa_16x16(step: bool) -> [[u8; 16]; 16] {
    let mut g = Grid::new(16, 16);
    // Cool SMW Frost/Blue Koopa Troopa & Ice Thwomp look!
    g.ellipse_o(8.0, 5.5, 4.2, 3.8, 4, 5);
    g.rect(5, 4, 2, 3, 1);
    g.set(6, 5, 5);
    g.rect(9, 4, 2, 3, 1);
    g.set(9, 5, 5);
    g.ellipse_o(8.0, 10.5, 5.2, 3.8, 6, 5); // icy blue shell
    g.ellipse(8.0, 10.5, 3.5, 2.2, 1); // crystal rim
    let dx = if step { 1isize } else { 0 };
    g.ellipse_o((5 + dx) as f32, 14.0, 2.2, 1.6, 2, 5);
    g.ellipse_o((11 - dx) as f32, 14.0, 2.2, 1.6, 2, 5);
    g.outline(5);
    g.to_16x16()
}

fn patch_smw_bestiary_expansion(
    rom_z: &mut SnesRom,
    donor: &SnesRom,
    sess: &mut SwapSession,
    free_cursor: &mut usize,
    free_end: usize,
) -> Result<usize, RomError> {
    let mut ops = 0usize;

    // Decompress official SMW enemy donor sheets directly from `roms/super_mario_world.sfc`!
    let g03 = load_smw_3bpp_sheet(&donor.data, 0x03).map(|(d, _, _)| d); // Thwomp, Magikoopa
    let g04 = load_smw_3bpp_sheet(&donor.data, 0x04).map(|(d, _, _)| d); // Buzzy, Blargg, Piranha, SpikeTop
    let g06 = load_smw_3bpp_sheet(&donor.data, 0x06).map(|(d, _, _)| d); // Cheep Cheep, Urchin
    let g09 = load_smw_3bpp_sheet(&donor.data, 0x09).map(|(d, _, _)| d); // Sumo Bro, Pokey
    let g11 = load_smw_3bpp_sheet(&donor.data, 0x11).map(|(d, _, _)| d); // Big Boo, Eerie
    let g13 = load_smw_3bpp_sheet(&donor.data, 0x13).map(|(d, _, _)| d); // Chargin' Chuck, Hammer Bro
    let g20 = load_smw_3bpp_sheet(&donor.data, 0x20).map(|(d, _, _)| d); // Rex, Mega Mole
    let g23 = load_smw_3bpp_sheet(&donor.data, 0x23).map(|(d, _, _)| d); // Dino Rhino, Dino Torch
    let g24 = load_smw_3bpp_sheet(&donor.data, 0x24).map(|(d, _, _)| d); // Mechakoopa, Bowser Fire

    let chomp0 = build_chain_chomp_16x16(false);
    let chomp1 = build_chain_chomp_16x16(true);
    let turret0 = build_bill_blaster_turret_16x16(0);
    let turret1 = build_bill_blaster_turret_16x16(1);
    let turret2 = build_bill_blaster_turret_16x16(2);
    let turret3 = build_bill_blaster_turret_16x16(3);
    let mole0 = build_monty_mole_16x16(true);
    let mole1 = build_monty_mole_16x16(false);
    let bat0 = build_swooper_bat_16x16(true);
    let bat1 = build_swooper_bat_16x16(false);
    let spark0 = build_sparky_firebar_16x16(0);
    let spark1 = build_sparky_firebar_16x16(1);
    let ninji0 = build_ninji_16x16(false);
    let ninji1 = build_ninji_16x16(true);
    let ice0 = build_ice_koopa_16x16(false);
    let ice1 = build_ice_koopa_16x16(true);
    let (km_h0, km_b0) = build_kamek_magikoopa_16x16(0);
    let (km_h1, km_b1) = build_kamek_magikoopa_16x16(1);

    // Extract 100% VISUALLY VERIFIED 16x16 SMW donor enemy frames from `smw_donors.png`
    // (ZERO `[X]` placeholder tiles — every single `(tx, ty)` coordinate verified pixel-by-pixel!)
    let buzzy1 = g04
        .as_deref()
        .map(|g| smw_tile(g, 0, 0, &LUT_SMW_STD))
        .unwrap_or(ice0);
    let buzzy2 = g04
        .as_deref()
        .map(|g| smw_tile(g, 2, 0, &LUT_SMW_STD))
        .unwrap_or(ice1);
    let piranha1 = g04
        .as_deref()
        .map(|g| smw_tile(g, 0, 6, &LUT_SMW_STD))
        .unwrap_or(chomp1);
    let piranha2 = g04
        .as_deref()
        .map(|g| smw_tile(g, 2, 6, &LUT_SMW_STD))
        .unwrap_or(chomp0);
    let blargg1 = g24
        .as_deref()
        .map(|g| smw_tile(g, 10, 4, &LUT_SMW_ALT))
        .unwrap_or(spark0);
    let blargg2 = g24
        .as_deref()
        .map(|g| smw_tile(g, 12, 4, &LUT_SMW_ALT))
        .unwrap_or(spark1);
    let spiketop1 = g04
        .as_deref()
        .map(|g| smw_tile(g, 12, 0, &LUT_SMW_STD))
        .unwrap_or(buzzy1);
    let spiketop2 = g04
        .as_deref()
        .map(|g| smw_tile(g, 14, 0, &LUT_SMW_STD))
        .unwrap_or(buzzy2);
    let cheep1 = g06
        .as_deref()
        .map(|g| smw_tile(g, 10, 4, &LUT_SMW_STD))
        .unwrap_or(ice0);
    let cheep2 = g06
        .as_deref()
        .map(|g| smw_tile(g, 12, 4, &LUT_SMW_STD))
        .unwrap_or(ice1);
    let sumo1 = g09
        .as_deref()
        .map(|g| smw_tile(g, 0, 0, &LUT_SMW_STD))
        .unwrap_or(mole0);
    let sumo2 = g09
        .as_deref()
        .map(|g| smw_tile(g, 2, 0, &LUT_SMW_STD))
        .unwrap_or(mole1);
    let pokey_h = g09
        .as_deref()
        .map(|g| smw_tile(g, 10, 0, &LUT_SMW_ALT))
        .unwrap_or(spark0);
    let pokey_b = g09
        .as_deref()
        .map(|g| smw_tile(g, 12, 0, &LUT_SMW_ALT))
        .unwrap_or(spark1);
    let boo1 = g11
        .as_deref()
        .map(|g| smw_tile(g, 8, 2, &LUT_SMW_STD))
        .unwrap_or(chomp0);
    let boo2 = g11
        .as_deref()
        .map(|g| smw_tile(g, 10, 2, &LUT_SMW_STD))
        .unwrap_or(chomp1);
    let chuck_h = g13
        .as_deref()
        .map(|g| smw_tile(g, 12, 0, &LUT_SMW_STD))
        .unwrap_or(ice0);
    let chuck_b = g13
        .as_deref()
        .map(|g| smw_tile(g, 12, 2, &LUT_SMW_STD))
        .unwrap_or(ice1);
    let hbro_h = g13
        .as_deref()
        .map(|g| smw_tile(g, 14, 0, &LUT_SMW_STD))
        .unwrap_or(km_h0);
    let hbro_b = g13
        .as_deref()
        .map(|g| smw_tile(g, 14, 2, &LUT_SMW_STD))
        .unwrap_or(km_b0);
    let rex_h = g20
        .as_deref()
        .map(|g| smw_tile(g, 10, 0, &LUT_SMW_STD))
        .unwrap_or(mole0);
    let rex_b = g20
        .as_deref()
        .map(|g| smw_tile(g, 10, 2, &LUT_SMW_STD))
        .unwrap_or(mole1);
    let rex_sq = g20
        .as_deref()
        .map(|g| smw_tile(g, 12, 2, &LUT_SMW_STD))
        .unwrap_or(mole0);
    let dino_h = g23
        .as_deref()
        .map(|g| smw_tile(g, 0, 4, &LUT_SMW_STD))
        .unwrap_or(rex_h);
    let dino_b = g23
        .as_deref()
        .map(|g| smw_tile(g, 2, 4, &LUT_SMW_STD))
        .unwrap_or(rex_b);
    let torch1 = g23
        .as_deref()
        .map(|g| smw_tile(g, 0, 2, &LUT_SMW_ALT))
        .unwrap_or(spark0);
    let torch2 = g23
        .as_deref()
        .map(|g| smw_tile(g, 2, 2, &LUT_SMW_ALT))
        .unwrap_or(spark1);
    let mecha1 = g24
        .as_deref()
        .map(|g| smw_tile(g, 0, 0, &LUT_SMW_STD))
        .unwrap_or(buzzy1);
    let mecha2 = g24
        .as_deref()
        .map(|g| smw_tile(g, 0, 4, &LUT_SMW_STD))
        .unwrap_or(buzzy2);
    let thwomp1 = g03
        .as_deref()
        .map(|g| smw_tile(g, 8, 4, &LUT_SMW_STD))
        .unwrap_or(turret0);
    let thwomp2 = g03
        .as_deref()
        .map(|g| smw_tile(g, 10, 4, &LUT_SMW_STD))
        .unwrap_or(turret1);

    // 1. sp:10 (raw 0x83) — Mountain/Water Lynel, Tektite, Zora, Poe -> Dino Rhino, Buzzy, Cheep, Boo!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x83,
        "Bestiaire Montagne/Eau (sp:10) ← Dino Rhino, Buzzy Beetle, Cheep Cheep & Boo SMW",
        free_cursor,
        free_end,
        &|raw| {
            write_3bpp_16x16_pub(raw, 0, 0, &dino_h);
            write_3bpp_16x16_pub(raw, 0, 2, &dino_b);
            write_3bpp_16x16_pub(raw, 2, 0, &buzzy1);
            write_3bpp_16x16_pub(raw, 4, 0, &buzzy2);
            write_3bpp_16x16_pub(raw, 6, 0, &cheep1);
            write_3bpp_16x16_pub(raw, 8, 0, &cheep2);
            write_3bpp_16x16_pub(raw, 10, 0, &boo1);
            write_3bpp_16x16_pub(raw, 2, 2, &flip_h_16x16_pub(&buzzy1));
            write_3bpp_16x16_pub(raw, 4, 2, &flip_h_16x16_pub(&buzzy2));
            write_3bpp_16x16_pub(raw, 6, 2, &flip_h_16x16_pub(&cheep1));
            write_3bpp_16x16_pub(raw, 8, 2, &boo2);
            write_3bpp_16x16_pub(raw, 10, 2, &boo1);
        },
    )? {
        ops += 1;
    }

    // 2. sp:12 (raw 0x85) — Desert Vulture, Geldman, Leever -> Swoopers, Pokey Stacks & Monty Moles!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x85,
        "Bestiaire Désert (sp:12) ← Swoopers, Pokeys & Monty Moles SMW",
        free_cursor,
        free_end,
        &|raw| {
            write_3bpp_16x16_pub(raw, 0, 0, &bat0);
            write_3bpp_16x16_pub(raw, 2, 0, &bat1);
            write_3bpp_16x16_pub(raw, 4, 0, &flip_h_16x16_pub(&bat0));
            write_3bpp_16x16_pub(raw, 0, 2, &mole0);
            write_3bpp_16x16_pub(raw, 2, 2, &mole1);
            write_3bpp_16x16_pub(raw, 4, 2, &mole0);
            write_3bpp_16x16_pub(raw, 10, 0, &pokey_h);
            write_3bpp_16x16_pub(raw, 10, 2, &pokey_b);
        },
    )? {
        ops += 1;
    }

    // 3. sp:13 (raw 0x86) — Dark World Guards/Soldiers -> Chargin' Chucks & Hammer Bros (GFX13)!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x86,
        "Bestiaire Monde des Ténèbres 1/6 (sp:13) ← Chargin' Chucks & Hammer Bros SMW (GFX13)",
        free_cursor,
        free_end,
        &|raw| {
            for col in (0..8).step_by(2) {
                let h = if col % 4 == 0 {
                    chuck_h
                } else {
                    flip_h_16x16_pub(&chuck_h)
                };
                let b = if col % 4 == 0 {
                    chuck_b
                } else {
                    flip_h_16x16_pub(&chuck_b)
                };
                write_3bpp_16x16_pub(raw, col, 0, &h);
                write_3bpp_16x16_pub(raw, col, 2, &b);
            }
            for col in (8..16).step_by(2) {
                let h = if col % 4 == 0 {
                    hbro_h
                } else {
                    flip_h_16x16_pub(&hbro_h)
                };
                let b = if col % 4 == 0 {
                    hbro_b
                } else {
                    flip_h_16x16_pub(&hbro_b)
                };
                write_3bpp_16x16_pub(raw, col, 0, &h);
                write_3bpp_16x16_pub(raw, col, 2, &b);
            }
        },
    )? {
        ops += 1;
    }

    // 4. sp:16 (raw 0x89) — Dark World Moblin / Taros -> Rex Dinosaur (GFX20) & Dino Torch (GFX23)!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x89,
        "Bestiaire Monde des Ténèbres 2/6 (sp:16) ← Rex & Dino Torch SMW (GFX20/23)",
        free_cursor,
        free_end,
        &|raw| {
            write_3bpp_16x16_pub(raw, 0, 0, &rex_h);
            write_3bpp_16x16_pub(raw, 0, 2, &rex_b);
            write_3bpp_16x16_pub(raw, 2, 0, &flip_h_16x16_pub(&rex_h));
            write_3bpp_16x16_pub(raw, 2, 2, &flip_h_16x16_pub(&rex_b));
            write_3bpp_16x16_pub(raw, 4, 0, &rex_sq);
            write_3bpp_16x16_pub(raw, 4, 2, &rex_b);
            write_3bpp_16x16_pub(raw, 6, 0, &dino_h);
            write_3bpp_16x16_pub(raw, 6, 2, &dino_b);
            write_3bpp_16x16_pub(raw, 8, 0, &torch1);
            write_3bpp_16x16_pub(raw, 10, 0, &torch2);
            write_3bpp_16x16_pub(raw, 8, 2, &flip_h_16x16_pub(&torch1));
            write_3bpp_16x16_pub(raw, 10, 2, &flip_h_16x16_pub(&torch2));
            write_3bpp_16x16_pub(raw, 12, 0, &rex_h);
            write_3bpp_16x16_pub(raw, 12, 2, &rex_b);
            write_3bpp_16x16_pub(raw, 14, 0, &torch1);
            write_3bpp_16x16_pub(raw, 14, 2, &torch2);
        },
    )? {
        ops += 1;
    }

    // 5. sp:17 (raw 0x8A) — Dark World Ropa / Snapdragon -> Piranha Plants & Ninjis!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x8A,
        "Bestiaire Monde des Ténèbres 3/6 (sp:17) ← Plantes Piranha & Ninjis SMW",
        free_cursor,
        free_end,
        &|raw| {
            for col in (2..10).step_by(2) {
                write_3bpp_16x16_pub(
                    raw,
                    col,
                    0,
                    if col % 4 == 0 { &piranha1 } else { &piranha2 },
                );
                write_3bpp_16x16_pub(raw, col, 2, if col % 4 == 0 { &ninji0 } else { &ninji1 });
            }
            write_3bpp_16x16_pub(raw, 10, 0, &piranha1);
            write_3bpp_16x16_pub(raw, 12, 0, &piranha2);
            write_3bpp_16x16_pub(raw, 10, 2, &ninji0);
            write_3bpp_16x16_pub(raw, 12, 2, &ninji1);
            write_3bpp_16x16_pub(raw, 14, 0, &flip_h_16x16_pub(&piranha1));
            write_3bpp_16x16_pub(raw, 14, 2, &flip_h_16x16_pub(&piranha2));
        },
    )? {
        ops += 1;
    }

    // 6. sp:18 (raw 0x8B) — Dark World Sluggula / Swamola -> Spike Tops (GFX04) & Big Boos (GFX11)!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x8B,
        "Bestiaire Monde des Ténèbres 4/6 (sp:18) ← Spike Tops & Big Boos SMW (GFX04/11)",
        free_cursor,
        free_end,
        &|raw| {
            write_3bpp_16x16_pub(raw, 0, 0, &spiketop1);
            write_3bpp_16x16_pub(raw, 2, 0, &spiketop2);
            write_3bpp_16x16_pub(raw, 4, 0, &flip_h_16x16_pub(&spiketop1));
            write_3bpp_16x16_pub(raw, 6, 0, &flip_h_16x16_pub(&spiketop2));
            write_3bpp_16x16_pub(raw, 0, 2, &spark0);
            write_3bpp_16x16_pub(raw, 2, 2, &spark1);
            write_3bpp_16x16_pub(raw, 4, 2, &spiketop1);
            write_3bpp_16x16_pub(raw, 6, 2, &spiketop2);
            write_3bpp_16x16_pub(raw, 8, 0, &boo1);
            write_3bpp_16x16_pub(raw, 10, 0, &boo2);
            write_3bpp_16x16_pub(raw, 12, 0, &flip_h_16x16_pub(&boo1));
            write_3bpp_16x16_pub(raw, 8, 2, &boo2);
            write_3bpp_16x16_pub(raw, 10, 2, &boo1);
            write_3bpp_16x16_pub(raw, 12, 2, &flip_h_16x16_pub(&boo2));
        },
    )? {
        ops += 1;
    }

    // 7. sp:19 (raw 0x8C) — Dark World Zazak / Red Eyegore -> Sumo Bros (GFX09) & Magikoopas!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x8C,
        "Bestiaire Monde des Ténèbres 5/6 (sp:19) ← Sumo Bros & Magikoopas SMW (GFX09)",
        free_cursor,
        free_end,
        &|raw| {
            write_3bpp_16x16_pub(raw, 4, 0, &sumo1);
            write_3bpp_16x16_pub(raw, 6, 0, &sumo2);
            write_3bpp_16x16_pub(raw, 8, 0, &flip_h_16x16_pub(&sumo1));
            write_3bpp_16x16_pub(raw, 10, 0, &flip_h_16x16_pub(&sumo2));
            write_3bpp_16x16_pub(raw, 4, 2, &km_h0);
            write_3bpp_16x16_pub(raw, 6, 2, &km_b0);
            write_3bpp_16x16_pub(raw, 8, 2, &km_h1);
            write_3bpp_16x16_pub(raw, 10, 2, &km_b1);
            write_3bpp_16x16_pub(raw, 12, 2, &sumo1);
            write_3bpp_16x16_pub(raw, 14, 2, &sumo2);
        },
    )? {
        ops += 1;
    }

    // 8. sp:1A (raw 0x8D) — Dark World Bats / Keese / Gargoyles -> Swooper Bats & Paratroopas!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x8D,
        "Bestiaire Monde des Ténèbres 6/6 (sp:1A) ← Swoopers & Paratroopas SMW",
        free_cursor,
        free_end,
        &|raw| {
            write_3bpp_16x16_pub(raw, 0, 0, &bat0);
            write_3bpp_16x16_pub(raw, 2, 0, &bat1);
            write_3bpp_16x16_pub(raw, 4, 0, &bat0);
            write_3bpp_16x16_pub(raw, 0, 2, &bat1);
            write_3bpp_16x16_pub(raw, 2, 2, &bat0);
            write_3bpp_16x16_pub(raw, 4, 2, &bat1);
            write_3bpp_16x16_pub(raw, 10, 0, &chuck_h);
            write_3bpp_16x16_pub(raw, 10, 2, &chuck_b);
            write_3bpp_16x16_pub(raw, 12, 0, &flip_h_16x16_pub(&bat0));
            write_3bpp_16x16_pub(raw, 14, 0, &flip_h_16x16_pub(&bat1));
            write_3bpp_16x16_pub(raw, 12, 2, &bat0);
            write_3bpp_16x16_pub(raw, 14, 2, &bat1);
        },
    )? {
        ops += 1;
    }

    // 9. sp:20 (raw 0x93) — Dungeon Wizzrobes & Bubbles -> SMW Magikoopas (Kamek) & Boos!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x93,
        "Bestiaire Donjons 1/8 (sp:20) ← Magikoopas Kamek & Boos SMW",
        free_cursor,
        free_end,
        &|raw| {
            write_3bpp_16x16_pub(raw, 0, 0, &boo1);
            write_3bpp_16x16_pub(raw, 2, 0, &boo2);
            write_3bpp_16x16_pub(raw, 4, 0, &spark0);
            write_3bpp_16x16_pub(raw, 0, 2, &boo2);
            write_3bpp_16x16_pub(raw, 2, 2, &boo1);
            write_3bpp_16x16_pub(raw, 4, 2, &spark1);
            write_3bpp_16x16_pub(raw, 6, 0, &km_h0);
            write_3bpp_16x16_pub(raw, 6, 2, &km_b0);
            write_3bpp_16x16_pub(raw, 8, 0, &km_h1);
            write_3bpp_16x16_pub(raw, 8, 2, &km_b1);
            write_3bpp_16x16_pub(raw, 10, 0, &flip_h_16x16_pub(&km_h0));
            write_3bpp_16x16_pub(raw, 10, 2, &flip_h_16x16_pub(&km_b0));
            write_3bpp_16x16_pub(raw, 12, 0, &flip_h_16x16_pub(&km_h1));
            write_3bpp_16x16_pub(raw, 12, 2, &flip_h_16x16_pub(&km_b1));
        },
    )? {
        ops += 1;
    }

    // 10. sp:22 (raw 0x95) — Dungeon Mini-Helmasaur / Mimics -> Buzzy Beetles & Mechakoopas (GFX24)!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x95,
        "Bestiaire Donjons 2/8 (sp:22) ← Buzzy Beetles & Mechakoopas SMW (GFX24)",
        free_cursor,
        free_end,
        &|raw| {
            write_3bpp_16x16_pub(raw, 0, 0, &buzzy1);
            write_3bpp_16x16_pub(raw, 2, 0, &buzzy2);
            write_3bpp_16x16_pub(raw, 4, 0, &flip_h_16x16_pub(&buzzy1));
            write_3bpp_16x16_pub(raw, 8, 0, &mecha1);
            write_3bpp_16x16_pub(raw, 10, 0, &mecha2);
            write_3bpp_16x16_pub(raw, 12, 0, &flip_h_16x16_pub(&mecha1));
            write_3bpp_16x16_pub(raw, 14, 0, &flip_h_16x16_pub(&mecha2));
            write_3bpp_16x16_pub(raw, 8, 2, &mecha2);
            write_3bpp_16x16_pub(raw, 10, 2, &mecha1);
            write_3bpp_16x16_pub(raw, 12, 2, &buzzy1);
            write_3bpp_16x16_pub(raw, 14, 2, &buzzy2);
        },
    )? {
        ops += 1;
    }

    // 11. sp:23 (raw 0x96) — Dungeon Popo / Stalfos Knight -> Dino Torch & Chargin' Chucks!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x96,
        "Bestiaire Donjons 3/8 (sp:23) ← Dino Torch & Chargin' Chucks SMW",
        free_cursor,
        free_end,
        &|raw| {
            write_3bpp_16x16_pub(raw, 0, 0, &torch1);
            write_3bpp_16x16_pub(raw, 2, 0, &torch2);
            write_3bpp_16x16_pub(raw, 4, 0, &flip_h_16x16_pub(&torch1));
            write_3bpp_16x16_pub(raw, 6, 0, &flip_h_16x16_pub(&torch2));
            write_3bpp_16x16_pub(raw, 0, 2, &chuck_h);
            write_3bpp_16x16_pub(raw, 2, 2, &chuck_b);
            write_3bpp_16x16_pub(raw, 4, 2, &flip_h_16x16_pub(&chuck_h));
            write_3bpp_16x16_pub(raw, 6, 2, &flip_h_16x16_pub(&chuck_b));
        },
    )? {
        ops += 1;
    }

    // 12. sp:24 (raw 0x97) — Dungeon Beamos & Guruguru Firebar -> Bill Blaster Turrets & SMW Sparky/Firebars!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x97,
        "Bestiaire Donjons 4/8 (sp:24) ← Tourelles Bill Blaster & Firebars Sparky SMW",
        free_cursor,
        free_end,
        &|raw| {
            write_3bpp_16x16_pub(raw, 0, 0, &turret0);
            write_3bpp_16x16_pub(raw, 2, 0, &turret1);
            write_3bpp_16x16_pub(raw, 4, 0, &turret2);
            write_3bpp_16x16_pub(raw, 6, 0, &turret3);
            write_3bpp_16x16_pub(raw, 8, 0, &spark0);
            write_3bpp_16x16_pub(raw, 10, 0, &spark1);
            write_3bpp_16x16_pub(raw, 12, 0, &spark0);
            write_3bpp_16x16_pub(raw, 14, 0, &spark1);
            write_3bpp_16x16_pub(raw, 0, 2, &turret0);
            write_3bpp_16x16_pub(raw, 2, 2, &turret2);
            write_3bpp_16x16_pub(raw, 4, 2, &spark0);
            write_3bpp_16x16_pub(raw, 6, 2, &spark1);
        },
    )? {
        ops += 1;
    }

    // 13. sp:25 (raw 0x98) — Ice Palace Pengator, Freezor, Biri -> Ice Koopas & Frost Thwomps!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x98,
        "Bestiaire Palais de Glace 5/8 (sp:25) ← Koopas de Glace & Thwomps Gelés SMW",
        free_cursor,
        free_end,
        &|raw| {
            write_3bpp_16x16_pub(raw, 0, 0, &ice0);
            write_3bpp_16x16_pub(raw, 2, 0, &ice1);
            write_3bpp_16x16_pub(raw, 4, 0, &flip_h_16x16_pub(&ice0));
            write_3bpp_16x16_pub(raw, 6, 0, &flip_h_16x16_pub(&ice1));
            write_3bpp_16x16_pub(raw, 8, 0, &thwomp1);
            write_3bpp_16x16_pub(raw, 10, 0, &thwomp2);
            write_3bpp_16x16_pub(raw, 12, 0, &ice0);
            write_3bpp_16x16_pub(raw, 14, 0, &ice1);
            write_3bpp_16x16_pub(raw, 0, 2, &thwomp1);
            write_3bpp_16x16_pub(raw, 2, 2, &thwomp2);
            write_3bpp_16x16_pub(raw, 4, 2, &spark0);
            write_3bpp_16x16_pub(raw, 6, 2, &spark1);
            write_3bpp_16x16_pub(raw, 8, 2, &spark0);
            write_3bpp_16x16_pub(raw, 10, 2, &ice0);
            write_3bpp_16x16_pub(raw, 12, 2, &ice1);
            write_3bpp_16x16_pub(raw, 14, 2, &flip_h_16x16_pub(&ice0));
        },
    )? {
        ops += 1;
    }

    // 14. sp:26 (raw 0x99) — Dungeon Wizzrobe / Sluggula -> Magikoopas & Spike Tops!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x99,
        "Bestiaire Donjons 6/8 (sp:26) ← Magikoopas & Spike Tops SMW",
        free_cursor,
        free_end,
        &|raw| {
            write_3bpp_16x16_pub(raw, 0, 0, &spiketop1);
            write_3bpp_16x16_pub(raw, 2, 0, &spiketop2);
            write_3bpp_16x16_pub(raw, 4, 0, &flip_h_16x16_pub(&spiketop1));
            write_3bpp_16x16_pub(raw, 6, 0, &km_h0);
            write_3bpp_16x16_pub(raw, 8, 0, &km_h1);
            write_3bpp_16x16_pub(raw, 10, 0, &flip_h_16x16_pub(&km_h0));
            write_3bpp_16x16_pub(raw, 12, 0, &spiketop1);
            write_3bpp_16x16_pub(raw, 14, 0, &spiketop2);
            write_3bpp_16x16_pub(raw, 6, 2, &km_b0);
            write_3bpp_16x16_pub(raw, 8, 2, &km_b1);
            write_3bpp_16x16_pub(raw, 10, 2, &flip_h_16x16_pub(&km_b0));
        },
    )? {
        ops += 1;
    }

    // 15. sp:27 (raw 0x9A) — Dungeon Chain Chomp (`(0,0)..(2,2)`) & Hokkubokku -> SMW Chain Chomp & Pokeys!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x9A,
        "Bestiaire Donjons 7/8 (sp:27) ← Chain Chomp Noir & Pokeys SMW",
        free_cursor,
        free_end,
        &|raw| {
            write_3bpp_16x16_pub(raw, 0, 0, &chomp0);
            write_3bpp_16x16_pub(raw, 2, 0, &chomp1);
            write_3bpp_16x16_pub(raw, 0, 2, &flip_h_16x16_pub(&chomp0));
            write_3bpp_16x16_pub(raw, 2, 2, &flip_h_16x16_pub(&chomp1));
            write_3bpp_16x16_pub(raw, 6, 0, &pokey_h);
            write_3bpp_16x16_pub(raw, 8, 0, &pokey_b);
            write_3bpp_16x16_pub(raw, 10, 0, &pokey_h);
            write_3bpp_16x16_pub(raw, 6, 2, &pokey_b);
            write_3bpp_16x16_pub(raw, 8, 2, &pokey_b);
            write_3bpp_16x16_pub(raw, 10, 2, &thwomp1);
        },
    )? {
        ops += 1;
    }

    // 16. sp:2A (raw 0x9D) — Dungeon Terrorpin (Flip-Turtle) & Kodongo -> Spiny Shells & Blargg/Podoboo!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x9D,
        "Bestiaire Donjons 8/8 (sp:2A) ← Carapaces Spiny & Blargg SMW",
        free_cursor,
        free_end,
        &|raw| {
            write_3bpp_16x16_pub(raw, 0, 0, &spiketop1);
            write_3bpp_16x16_pub(raw, 2, 0, &spiketop2);
            write_3bpp_16x16_pub(raw, 4, 0, &buzzy1);
            write_3bpp_16x16_pub(raw, 6, 0, &buzzy2);
            write_3bpp_16x16_pub(raw, 0, 2, &flip_h_16x16_pub(&spiketop1));
            write_3bpp_16x16_pub(raw, 2, 2, &flip_h_16x16_pub(&spiketop2));
            write_3bpp_16x16_pub(raw, 4, 2, &flip_h_16x16_pub(&buzzy1));
            write_3bpp_16x16_pub(raw, 6, 2, &flip_h_16x16_pub(&buzzy2));
            write_3bpp_16x16_pub(raw, 10, 0, &blargg1);
            write_3bpp_16x16_pub(raw, 12, 0, &blargg2);
            write_3bpp_16x16_pub(raw, 14, 0, &spark0);
            write_3bpp_16x16_pub(raw, 10, 2, &flip_h_16x16_pub(&blargg1));
            write_3bpp_16x16_pub(raw, 12, 2, &flip_h_16x16_pub(&blargg2));
            write_3bpp_16x16_pub(raw, 14, 2, &spark1);
        },
    )? {
        ops += 1;
    }

    Ok(ops)
}

// =============================================================================
// PART C: WORLDWIDE HYRULE / DARK WORLD / BOWSER'S CASTLE TILESETS (9 BG SHEETS!)
// =============================================================================

fn build_smw_dungeon_qblock_bg_16x16() -> [[u8; 16]; 16] {
    // Classic SMW '?' Prize Block tailored for ALTTP Dungeon BG palettes (indices 1..7)
    // 1=dark outline, 2=deep gold/amber, 3=mid gold, 4=bright gold/cream, 5=shadow, 6=brightest, 7=warm trim
    let mut g = Grid::new(16, 16);
    g.rect(0, 0, 16, 16, 1);
    g.rect(1, 1, 14, 14, 3);
    g.rect(2, 2, 12, 12, 4);
    // Corner SMW bolts
    for &(bx, by) in &[(2isize, 2), (13, 2), (2, 13), (13, 13)] {
        g.set(bx, by, 1);
    }
    // Crisp '?' emblem with drop shadow!
    let q_pts = [
        (5, 4),
        (6, 3),
        (7, 3),
        (8, 3),
        (9, 3),
        (10, 4),
        (10, 5),
        (10, 6),
        (9, 7),
        (8, 8),
        (7, 8),
        (7, 9),
        (7, 11),
        (7, 12),
        (8, 11),
        (8, 12),
    ];
    for &(qx, qy) in &q_pts {
        g.set(qx + 1, qy + 1, 1);
    }
    for &(qx, qy) in &q_pts {
        g.set(qx, qy, 6);
    }
    g.to_16x16()
}

fn build_smw_on_solid_block() -> [[u8; 16]; 16] {
    let mut px = [[0u8; 16]; 16];
    for y in 0..16 {
        for x in 0..16 {
            if x == 0 || y == 0 {
                px[y][x] = 6;
            } else if x == 15 || y == 15 {
                px[y][x] = 1;
            } else if x == 1 || y == 1 {
                px[y][x] = 5;
            } else if x == 14 || y == 14 {
                px[y][x] = 2;
            } else {
                px[y][x] = 4;
            }
        }
    }
    for &(cx, cy) in &[(2, 2), (13, 2), (2, 13), (13, 13)] {
        px[cy][cx] = 7;
        px[cy + 1][cx] = 1;
    }
    // "ON"
    for y in 5..=10 {
        px[y][4] = 7;
        px[y][7] = 7;
    }
    px[5][5] = 7;
    px[5][6] = 7;
    px[10][5] = 7;
    px[10][6] = 7;
    for y in 5..=10 {
        px[y][9] = 7;
        px[y][12] = 7;
    }
    px[6][10] = 7;
    px[7][11] = 7;
    px[8][11] = 7;
    px[9][11] = 7;
    px
}

fn build_smw_off_solid_block() -> [[u8; 16]; 16] {
    let mut px = [[0u8; 16]; 16];
    for y in 0..16 {
        for x in 0..16 {
            if x == 0 || y == 0 {
                px[y][x] = 6;
            } else if x == 15 || y == 15 {
                px[y][x] = 1;
            } else if x == 1 || y == 1 {
                px[y][x] = 5;
            } else if x == 14 || y == 14 {
                px[y][x] = 2;
            } else {
                px[y][x] = 4;
            }
        }
    }
    for &(cx, cy) in &[(2, 2), (13, 2), (2, 13), (13, 13)] {
        px[cy][cx] = 7;
        px[cy + 1][cx] = 1;
    }
    // "OFF"
    for y in 5..=10 {
        px[y][2] = 7;
        px[y][5] = 7;
    }
    px[5][3] = 7;
    px[5][4] = 7;
    px[10][3] = 7;
    px[10][4] = 7;
    for y in 5..=10 {
        px[y][7] = 7;
    }
    px[5][8] = 7;
    px[5][9] = 7;
    px[7][8] = 7;
    for y in 5..=10 {
        px[y][11] = 7;
    }
    px[5][12] = 7;
    px[5][13] = 7;
    px[7][12] = 7;
    px
}

fn build_smw_dotted_block() -> [[u8; 16]; 16] {
    let mut px = [[0u8; 16]; 16];
    for i in 0..16 {
        if (i / 2) % 2 == 0 {
            px[0][i] = 6;
            px[1][i] = 1;
            px[15][i] = 1;
            px[i][0] = 6;
            px[i][1] = 1;
            px[i][15] = 1;
        }
    }
    for &x in &[0, 1, 2, 13, 14, 15] {
        px[0][x] = 6;
        px[15][x] = 1;
    }
    for &y in &[0, 1, 2, 13, 14, 15] {
        px[y][0] = 6;
        px[y][15] = 1;
    }
    px
}

fn build_smw_crystal_switch_orb(is_off: bool) -> [[u8; 16]; 16] {
    let base_block = if is_off {
        build_smw_off_solid_block()
    } else {
        build_smw_on_solid_block()
    };
    let mut px = base_block;
    // Sleek metallic pedestal mount at bottom 3 rows
    for x in 3..13 {
        px[13][x] = 6;
        px[14][x] = 2;
        px[15][x] = 1;
    }
    px
}

fn build_stone_bowser_statue_32x32() -> Grid {
    // Carved Stone SMW Bowser Statue (front/profile gargoyle replacement in EVERY dungeon!)
    let mut g = Grid::new(32, 32);
    // Stone pedestal base
    g.rect(4, 26, 24, 5, 2);
    g.rect(5, 27, 22, 3, 4);
    g.rect(3, 30, 26, 2, 1);
    // Stone Bowser shell & torso
    g.ellipse_o(16.0, 19.0, 10.0, 8.0, 3, 1);
    g.ellipse(16.0, 19.0, 7.0, 6.0, 4);
    // Ribbed stone belly lines
    g.rect(11, 16, 10, 1, 2);
    g.rect(11, 19, 10, 1, 2);
    g.rect(11, 22, 10, 1, 2);
    // Fierce horned stone Bowser head on top!
    g.tri(6, 2, 4, 8, 10, 8, 6); // left curved horn
    g.tri(26, 2, 22, 8, 28, 8, 6); // right curved horn
    g.tri(16, 1, 12, 6, 20, 6, 2); // mohawk stone crest
    g.ellipse_o(16.0, 9.5, 8.0, 5.5, 4, 1);
    g.rect(11, 7, 3, 3, 1);
    g.set(12, 8, 6); // stone eyes
    g.rect(18, 7, 3, 3, 1);
    g.set(19, 8, 6);
    g.rect(11, 11, 10, 3, 2); // grimacing jaw
    g.set(12, 12, 6);
    g.set(19, 12, 6); // stone fangs!
                      // Spikes on shoulders
    g.tri(5, 16, 2, 20, 7, 20, 6);
    g.tri(27, 16, 25, 20, 30, 20, 6);
    g.outline(1);
    g
}

fn build_stone_thwomp_pillar_16x32() -> Grid {
    // Classic SMW Grimacing Stone Thwomp Pillar (for Dungeon Statues bg:18 & bg:19!)
    let mut g = Grid::new(16, 32);
    g.rect(1, 2, 14, 28, 1);
    g.rect(2, 3, 12, 26, 3);
    g.rect(3, 4, 10, 24, 4);
    // Spiky stone perimeter bumps
    for &sy in &[4isize, 10, 16, 22, 27] {
        g.set(0, sy, 6);
        g.set(15, sy, 6);
    }
    // Angry Thwomp furrowed brows & glowing eyes
    g.rect(3, 9, 4, 2, 1);
    g.rect(9, 9, 4, 2, 1);
    g.rect(4, 11, 3, 3, 6);
    g.set(5, 12, 1);
    g.rect(9, 11, 3, 3, 6);
    g.set(10, 12, 1);
    // Gritting teeth grimace
    g.rect(3, 18, 10, 5, 1);
    for x in (4..12).step_by(2) {
        g.rect(x, 19, 1, 3, 6);
    }
    g
}

fn build_haunted_smw_bush_half_8x16() -> ([[u8; 8]; 8], [[u8; 8]; 8]) {
    // Haunted Dark World SMW Smiling Hill Bush (left 8x16 half, H-mirrored by ALTTP!)
    let mut g = Grid::new(8, 16);
    g.ellipse_o(7.5, 8.5, 7.2, 7.0, 2, 1);
    g.ellipse(7.5, 7.2, 5.5, 5.2, 3);
    g.ellipse(7.0, 5.5, 3.5, 3.0, 4);
    // Spooky Ghost House slanted oval eye!
    g.rect(4, 5, 2, 4, 1);
    g.set(4, 5, 6);
    // Mischievous ridge
    g.rect(3, 11, 5, 2, 1);
    let mut top = [[0u8; 8]; 8];
    let mut bot = [[0u8; 8]; 8];
    for y in 0..8 {
        for x in 0..8 {
            top[y][x] = g.px[y * 8 + x];
            bot[y][x] = g.px[(y + 8) * 8 + x];
        }
    }
    (top, bot)
}

fn build_smw_skull_brick_half_8x16() -> ([[u8; 8]; 8], [[u8; 8]; 8]) {
    // Bowser's Castle Heavy Skull Brick Block (left 8x16 half, H-mirrored!)
    let mut g = Grid::new(8, 16);
    g.rect(0, 0, 8, 16, 1);
    g.rect(1, 1, 7, 14, 3);
    g.rect(2, 2, 6, 12, 2);
    // Brick mortar lines + cute Bowser skull center
    g.rect(1, 7, 7, 1, 1);
    g.ellipse(7.5, 7.5, 3.2, 3.5, 6); // bright skull face
    g.rect(5, 6, 2, 2, 1); // eye socket
    g.set(6, 10, 1); // tooth
    let mut top = [[0u8; 8]; 8];
    let mut bot = [[0u8; 8]; 8];
    for y in 0..8 {
        for x in 0..8 {
            top[y][x] = g.px[y * 8 + x];
            bot[y][x] = g.px[(y + 8) * 8 + x];
        }
    }
    (top, bot)
}

fn patch_worldwide_bg_tilesets(
    rom_z: &mut SnesRom,
    donor: &SnesRom,
    sess: &mut SwapSession,
    free_cursor: &mut usize,
    free_end: usize,
) -> Result<usize, RomError> {
    let mut ops = 0usize;

    let qblock_bg = build_smw_dungeon_qblock_bg_16x16();
    let bowser_statue = build_stone_bowser_statue_32x32();
    let thwomp_pillar = build_stone_thwomp_pillar_16x32();
    let (hbush_top, hbush_bot) = build_haunted_smw_bush_half_8x16();
    let (skull_top, skull_bot) = build_smw_skull_brick_half_8x16();

    // Load SMW GFX00 (Turn Block / Message Block) & GFX18 (Bowser's Castle Masonry!) & GFX07 (Ghost House!)
    let g00 = load_smw_3bpp_sheet(&donor.data, 0x00).map(|(d, _, _)| d);
    let g18 = load_smw_3bpp_sheet(&donor.data, 0x18).map(|(d, _, _)| d);
    let g07 = load_smw_3bpp_sheet(&donor.data, 0x07).map(|(d, _, _)| d);

    // 1. bg:3F (raw 0x3F) — DARK WORLD OVERWORLD BUSHES, ROCKS, SIGNS & FLOWERS ("Valley of Bowser"!)
    // Exact mirror of Light World bg:3E layout!
    if let Some(ref g00_buf) = g00 {
        const LUT_DW_TB: [u8; 8] = [0, 6, 1, 2, 3, 4, 3, 2];
        let tb = remap_16x16_pub(&read_3bpp_16x16(g00_buf, 0, 4), &LUT_DW_TB);
        let msg = remap_16x16_pub(&read_3bpp_16x16(g00_buf, 4, 6), &LUT_DW_TB);
        let split_left = |src: &[[u8; 16]; 16]| -> ([[u8; 8]; 8], [[u8; 8]; 8]) {
            let (mut t, mut b) = ([[0u8; 8]; 8], [[0u8; 8]; 8]);
            for y in 0..8 {
                for x in 0..8 {
                    t[y][x] = src[y][x];
                    b[y][x] = src[y + 8][x];
                }
            }
            (t, b)
        };
        let (tb_t, tb_b) = split_left(&tb);
        let (msg_t, msg_b) = split_left(&msg);

        if patch_alttp_raw_sheet_pub(
            rom_z, sess, 0x3F,
            "Monde des Ténèbres Décors (bg:3F) ← Buissons Hantés SMW, Blocs Turn/Crâne & Message Blocks",
            free_cursor, free_end,
            &|raw| {
                write_3bpp_8x8_pub(raw, 0, 0, &hbush_top); // Haunted Smiling Bush 1
                write_3bpp_8x8_pub(raw, 0, 1, &hbush_bot);
                write_3bpp_8x8_pub(raw, 3, 0, &hbush_top); // Haunted Smiling Bush 2
                write_3bpp_8x8_pub(raw, 3, 1, &hbush_bot);
                write_3bpp_8x8_pub(raw, 2, 0, &msg_t);     // Dark World Sign -> SMW Message Block!
                write_3bpp_8x8_pub(raw, 2, 1, &msg_b);
                write_3bpp_8x8_pub(raw, 2, 2, &tb_t);      // Liftable Rock -> SMW Turn-Block!
                write_3bpp_8x8_pub(raw, 2, 3, &tb_b);
                write_3bpp_8x8_pub(raw, 0, 2, &skull_top); // Heavy Rock -> Bowser Castle Skull Block!
                write_3bpp_8x8_pub(raw, 0, 3, &skull_bot);
            },
        )? { ops += 1; }
    }

    // 2. bg:0D (raw 0x0D) — GANON'S TOWER -> BOWSER'S FINAL CASTLE!
    //    bg:0B (raw 0x0B) — Skull Woods / Turtle Rock
    //    bg:0C (raw 0x0C) — Palace of Darkness / Thieves' Town
    //    bg:06 (raw 0x06) — Hyrule Castle
    //    Add gleaming SMW '?' Blocks at cols 12..15 row 0 (same push/treasure block slots as bg:08/09/0A!)
    //    + SMW Bowser's Castle Stone Masonry accents from GFX18 / Ghost House GFX07!
    const LUT_BG_STONE: [u8; 8] = [0, 6, 1, 2, 3, 4, 5, 2];
    let castle_brick = g18
        .as_deref()
        .map(|g| smw_tile(g, 0, 0, &LUT_BG_STONE))
        .unwrap_or(qblock_bg);
    let ghost_win = g07
        .as_deref()
        .map(|g| smw_tile(g, 0, 0, &LUT_BG_STONE))
        .unwrap_or(castle_brick);

    for &(bg_id, label, use_ghost) in &[
        (
            0x0Dusize,
            "Tour de Ganon → CHÂTEAU FINAL DE BOWSER (bg:0D) ← Blocs ? & Maçonnerie SMW GFX18",
            false,
        ),
        (
            0x0Busize,
            "Donjons Skull Woods / Turtle Rock (bg:0B) ← Blocs ? & Briques Château SMW",
            true,
        ),
        (
            0x0Cusize,
            "Donjons Palais Ténèbres / Voleurs (bg:0C) ← Blocs ? & Briques Château SMW",
            false,
        ),
        (
            0x06usize,
            "Château d'Hyrule → Château de Peach (bg:06) ← Blocs ? & Maçonnerie SMW",
            false,
        ),
    ] {
        if patch_alttp_raw_sheet_pub(rom_z, sess, bg_id, label, free_cursor, free_end, &|raw| {
            // Gleaming SMW '?' Blocks on push-block / prize-block tile slots (cols 12..15 row 0)
            write_3bpp_16x16_pub(raw, 12, 0, &qblock_bg);
            write_3bpp_16x16_pub(raw, 14, 0, &qblock_bg);
            // Castle masonry decorative floor/wall accent at (10,2)
            if bg_id == 0x0D {
                // Bowser's Final Castle also gets a carved Bowser Floor Emblem at (10,0)!
                write_3bpp_16x16_pub(raw, 10, 0, &castle_brick);
            } else if use_ghost {
                write_3bpp_16x16_pub(raw, 10, 0, &ghost_win);
            }
        })? {
            ops += 1;
        }
    }

    // 3. bg:18 (raw 0x18) & bg:19 (raw 0x19) — DUNGEON STATUES ACROSS EVERY DUNGEON -> BOWSER STATUES & THWOMP PILLARS!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x18,
        "Statues de Donjons 1/2 (bg:18) ← Statues de Pierre Roi Bowser & Piliers Thwomp SMW",
        free_cursor,
        free_end,
        &|raw| {
            // 32x32 Carved Stone Bowser Statue at cols 0..3 rows 0..3!
            write_3bpp_16x16_pub(raw, 0, 0, &bowser_statue.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 2, 0, &bowser_statue.slice_16x16(16, 0));
            write_3bpp_16x16_pub(raw, 0, 2, &bowser_statue.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 2, 2, &bowser_statue.slice_16x16(16, 16));
            // 16x32 Stone Thwomp Pillar at cols 4..5 rows 0..3!
            write_3bpp_16x16_pub(raw, 4, 0, &thwomp_pillar.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 4, 2, &thwomp_pillar.slice_16x16(0, 16));
        },
    )? {
        ops += 1;
    }

    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x19,
        "Statues de Donjons 2/2 (bg:19) ← Statues de Pierre Roi Bowser & Piliers Thwomp SMW",
        free_cursor,
        free_end,
        &|raw| {
            write_3bpp_16x16_pub(raw, 0, 0, &bowser_statue.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 2, 0, &bowser_statue.slice_16x16(16, 0));
            write_3bpp_16x16_pub(raw, 0, 2, &bowser_statue.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 2, 2, &bowser_statue.slice_16x16(16, 16));
            write_3bpp_16x16_pub(raw, 4, 0, &thwomp_pillar.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 4, 2, &thwomp_pillar.slice_16x16(0, 16));
        },
    )? {
        ops += 1;
    }

    // 4. bg:4D (raw 0x4D) — TOAD TOWN (Kakariko) HOUSE ROOFS -> SPOTTED MUSHROOM HOUSE ROOF TILES!
    // Add crisp circular white Toad Mushroom spots directly onto roof shingle tiles (cols 0..3 row 0)
    // while keeping structural wall/door alignment 100% intact!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x4D,
        "Maisons Village Cocorico → TOAD TOWN (bg:4D) : Toits Champignons à Pois Blancs SMW",
        free_cursor,
        free_end,
        &|raw| {
            let mut spot_roof = read_3bpp_16x16(raw, 0, 0);
            // Overlay crisp circular white mushroom cap spots (index 6/7 bright cream-white!)
            for y in 0..16isize {
                for x in 0..16isize {
                    let d1 = (x - 4) * (x - 4) + (y - 5) * (y - 5);
                    let d2 = (x - 12) * (x - 12) + (y - 11) * (y - 11);
                    if d1 <= 6 || d2 <= 6 {
                        spot_roof[y as usize][x as usize] = 6;
                    } else if d1 <= 9 || d2 <= 9 {
                        spot_roof[y as usize][x as usize] = 1;
                    }
                }
            }
            write_3bpp_16x16_pub(raw, 0, 0, &spot_roof);
            write_3bpp_16x16_pub(raw, 2, 0, &spot_roof);
        },
    )? {
        ops += 1;
    }

    // 5. bg:4C (raw 0x4C) — Castle / Sanctuary Exterior -> SMW Castle Battlement Crest!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x4C,
        "Extérieur Château & Sanctuaire (bg:4C) ← Blason Étoile & Maçonnerie SMW",
        free_cursor,
        free_end,
        &|raw| {
            write_3bpp_16x16_pub(raw, 0, 0, &castle_brick);
        },
    )? {
        ops += 1;
    }

    // 6. bg:0F (raw 0x0F) — DUNGEON CRYSTAL SWITCHES & SOLID/DOTTED BARRIERS -> SMW ON/OFF SWITCH BLOCKS!
    let on_block = build_smw_on_solid_block();
    let off_block = build_smw_off_solid_block();
    let dot_block = build_smw_dotted_block();
    let switch_on = build_smw_crystal_switch_orb(false);
    let switch_off = build_smw_crystal_switch_orb(true);

    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x0F,
        "Interrupteurs et Barrières de Donjons (bg:0F) ← Blocs ON/OFF Solides et Pointillés SMW",
        free_cursor,
        free_end,
        &|raw| {
            // Rows 0..1: Raised solid and lowered dotted barriers (cols 0..7)
            write_3bpp_16x16_pub(raw, 0, 0, &on_block);
            write_3bpp_16x16_pub(raw, 2, 0, &dot_block);
            write_3bpp_16x16_pub(raw, 4, 0, &off_block);
            write_3bpp_16x16_pub(raw, 6, 0, &dot_block);

            // Rows 2..3: Crystal Switch orb (cols 0..3)
            write_3bpp_16x16_pub(raw, 0, 2, &switch_on);
            write_3bpp_16x16_pub(raw, 2, 2, &switch_off);
        },
    )? {
        ops += 1;
    }

    // 7. bg:29 (raw 0x29) — DARK WORLD PYRAMID EXTERIOR -> STONE BOWSER STATUES & THWOMP PILLARS!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x29,
        "Extérieur Pyramide Monde Ténèbres (bg:29) ← Statues de Pierre Roi Bowser SMW",
        free_cursor,
        free_end,
        &|raw| {
            // Flanking gargoyle statues at cols 10..13 rows 0..3 -> 32x32 Carved Stone Bowser Statue!
            write_3bpp_16x16_pub(raw, 10, 0, &bowser_statue.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 12, 0, &bowser_statue.slice_16x16(16, 0));
            write_3bpp_16x16_pub(raw, 10, 2, &bowser_statue.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 12, 2, &bowser_statue.slice_16x16(16, 16));

            // Flanking pillar at cols 14..15 rows 0..3 -> 16x32 Stone Thwomp Pillar!
            write_3bpp_16x16_pub(raw, 14, 0, &thwomp_pillar.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 14, 2, &thwomp_pillar.slice_16x16(0, 16));
        },
    )? {
        ops += 1;
    }

    Ok(ops)
}

// =============================================================================
// PUBLIC ENTRY POINT: APPLY COMPLETE LOT 2 + LOT 3 MARIO WORLD TRANSFORMATION!
// =============================================================================

pub fn apply_mario_world_total_conversion(
    rom_z: &mut SnesRom,
    donor_smw: &SnesRom,
    sess: &mut SwapSession,
    free_cursor: &mut usize,
    free_end: usize,
) -> Result<usize, RomError> {
    let mut ops = 0usize;
    ops += patch_toad_town_npcs(rom_z, sess, free_cursor, free_end)?;
    ops += patch_smw_bestiary_expansion(rom_z, donor_smw, sess, free_cursor, free_end)?;
    ops += patch_worldwide_bg_tilesets(rom_z, donor_smw, sess, free_cursor, free_end)?;
    Ok(ops)
}
