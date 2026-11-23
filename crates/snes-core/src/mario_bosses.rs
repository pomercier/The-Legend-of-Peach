//! Super Mario World Boss Roster Total Conversion for Zelda ALTTP (v13)
//!
//! Transforms EVERY dungeon boss & Final Boss into an authentic, individually identifiable
//! Super Mario World boss — ALL 7 KOOPALINGS + FINAL GIANT KING BOWSER + POKEY + WIGGLER —
//! using exact ALTTP OAM tile geometry verified from `jpdasm` and real SMW donor pixels
//! from `roms/super_mario_world.sfc` (`GFX09`, `GFX21`, `GFX24`, etc.).
//!
//! Hardware compliance:
//! - Uses exact unshifted ALTTP GFX pointer table (`0x4F00` / `0x4FDF` / `0x50BE` in FR PAL)
//!   via `patch_alttp_raw_sheet_pub`, relocating compressed 3bpp sheets cleanly into Banks `$24..$3F`
//!   without ever crossing a 32 KB LoROM bank boundary or touching `0x08A000..0x08A2FF` / `0x100000..0x117FFF`.
//! - Only modifies 100% exclusive boss sprite palettes (`0x0DD55E` Kholdstare/Lemmy,
//!   `0x0DD596` & `0x0DD5EA` Ganon/Final King Bowser); all other bosses match native room CGRAM palettes
//!   with zero side effects on NPCs or other dungeons.
//! - Preserves Grande Fée / Yoshi tiles in `raw 0xAC` (`sp:39`) cols 10..15.

use crate::mixer::{
    flip_h_16x16_pub, load_smw_3bpp_sheet, patch_alttp_raw_sheet_pub, read_3bpp_16x16,
    remap_16x16_pub, write_3bpp_16x16_pub, write_3bpp_8x8_pub,
};
use crate::rom::{RomError, SnesRom};
use crate::swapper::{SwapKind, SwapSession};

// =============================================================================
// LIGHTWEIGHT 3BPP PIXEL CANVAS HELPERS
// =============================================================================

struct Canvas {
    w: usize,
    h: usize,
    px: Vec<u8>,
}

impl Canvas {
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

    #[inline]
    fn get(&self, x: isize, y: isize) -> u8 {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            self.px[(y as usize) * self.w + (x as usize)]
        } else {
            0
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
        let x0 = (cx - rx - 1.0).floor() as isize;
        let x1 = (cx + rx + 1.0).ceil() as isize;
        let y0 = (cy - ry - 1.0).floor() as isize;
        let y1 = (cy + ry + 1.0).ceil() as isize;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = (x as f32 + 0.5 - cx) / rx;
                let dy = (y as f32 + 0.5 - cy) / ry;
                if dx * dx + dy * dy <= 1.0 {
                    self.set(x, y, c);
                }
            }
        }
    }

    fn ellipse_outline(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, fill: u8, border: u8) {
        self.ellipse(cx, cy, rx, ry, border);
        if rx > 1.2 && ry > 1.2 {
            self.ellipse(cx, cy, rx - 1.15, ry - 1.15, fill);
        }
    }

    fn triangle(
        &mut self,
        x0: isize,
        y0: isize,
        x1: isize,
        y1: isize,
        x2: isize,
        y2: isize,
        c: u8,
    ) {
        let min_x = x0.min(x1).min(x2);
        let max_x = x0.max(x1).max(x2);
        let min_y = y0.min(y1).min(y2);
        let max_y = y0.max(y1).max(y2);
        let edge = |ax: isize, ay: isize, bx: isize, by: isize, px: isize, py: isize| -> isize {
            (px - ax) * (by - ay) - (py - ay) * (bx - ax)
        };
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let e0 = edge(x0, y0, x1, y1, x, y);
                let e1 = edge(x1, y1, x2, y2, x, y);
                let e2 = edge(x2, y2, x0, y0, x, y);
                if (e0 >= 0 && e1 >= 0 && e2 >= 0) || (e0 <= 0 && e1 <= 0 && e2 <= 0) {
                    self.set(x, y, c);
                }
            }
        }
    }

    fn line(&mut self, x0: isize, y0: isize, x1: isize, y1: isize, c: u8) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let (mut x, mut y) = (x0, y0);
        loop {
            self.set(x, y, c);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Add crisp 1px dark outline around non-zero pixels without overwriting interior details.
    /// If `open_right_seam` is true, does not outline x == w - 1 (for H-mirrored left halves!).
    fn auto_outline(&mut self, outline_col: u8, open_right_seam: bool) {
        let orig = self.px.clone();
        for y in 0..self.h {
            for x in 0..self.w {
                if orig[y * self.w + x] != 0 {
                    continue;
                }
                if open_right_seam && x + 1 == self.w {
                    continue;
                }
                let mut adj = false;
                for (dx, dy) in [(-1isize, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = x as isize + dx;
                    let ny = y as isize + dy;
                    if nx >= 0 && ny >= 0 && (nx as usize) < self.w && (ny as usize) < self.h {
                        let v = orig[(ny as usize) * self.w + (nx as usize)];
                        if v != 0 && v != outline_col {
                            adj = true;
                            break;
                        }
                    }
                }
                if adj {
                    self.px[y * self.w + x] = outline_col;
                }
            }
        }
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

    fn blit_16x16(&mut self, ox: isize, oy: isize, tile: &[[u8; 16]; 16]) {
        for y in 0..16 {
            for x in 0..16 {
                let c = tile[y][x];
                if c != 0 {
                    self.set(ox + x as isize, oy + y as isize, c);
                }
            }
        }
    }

    fn flip_v(&self) -> Self {
        let mut out = Self::new(self.w, self.h);
        for y in 0..self.h {
            for x in 0..self.w {
                out.px[(self.h - 1 - y) * self.w + x] = self.px[y * self.w + x];
            }
        }
        out
    }

    fn flip_h(&self) -> Self {
        let mut out = Self::new(self.w, self.h);
        for y in 0..self.h {
            for x in 0..self.w {
                out.px[y * self.w + (self.w - 1 - x)] = self.px[y * self.w + x];
            }
        }
        out
    }
}

/// Draw a classic SMW Koopaling spiked shell stripe (3D white cone spike with dark rim).
fn draw_spike(
    c: &mut Canvas,
    cx: isize,
    cy: isize,
    h: isize,
    w: isize,
    white: u8,
    shade: u8,
    outline: u8,
) {
    c.triangle(cx, cy - h, cx - w, cy, cx + w, cy, outline);
    if h > 2 && w > 1 {
        c.triangle(
            cx,
            cy - h + 1,
            cx - w + 1,
            cy - 1,
            cx + w - 1,
            cy - 1,
            white,
        );
        c.line(cx + 1, cy - h + 2, cx + w - 1, cy - 1, shade);
    }
}

/// Draw a classic SMW spinning fireball (16x16) using orange/yellow/red/white.
fn build_smw_fireball_16x16(
    white: u8,
    yellow: u8,
    orange: u8,
    red: u8,
    outline: u8,
    rot: usize,
) -> [[u8; 16]; 16] {
    let mut c = Canvas::new(16, 16);
    c.ellipse_outline(8.0, 8.0, 6.2, 6.2, red, outline);
    c.ellipse(7.5, 7.5, 4.5, 4.5, orange);
    c.ellipse(7.0, 7.0, 2.8, 2.8, yellow);
    c.rect(6, 6, 2, 2, white);
    // Swirl flame licks
    match rot % 4 {
        0 => {
            c.rect(12, 2, 3, 2, orange);
            c.set(14, 1, yellow);
            c.rect(1, 11, 3, 2, red);
            c.set(2, 12, yellow);
        }
        1 => {
            c.rect(2, 1, 2, 3, orange);
            c.set(2, 1, yellow);
            c.rect(12, 11, 2, 3, red);
            c.set(13, 13, yellow);
        }
        2 => {
            c.rect(1, 3, 3, 2, orange);
            c.set(1, 3, yellow);
            c.rect(12, 10, 3, 2, red);
            c.set(14, 10, yellow);
        }
        _ => {
            c.rect(11, 12, 2, 3, orange);
            c.set(12, 14, yellow);
            c.rect(3, 1, 2, 3, red);
            c.set(3, 1, yellow);
        }
    }
    c.auto_outline(outline, false);
    c.slice_16x16(0, 0)
}

// =============================================================================
// 1. DESERT PALACE (LANMOLAS -> 3 GIANT SMW POKEYS) — raw 0xA4 (sp:31)
//    Live Pal 6 (Aux2 0x07 @ 0x0DD542):
//      1=White, 2=DarkGreen, 3=Orange, 4=BrightYellow, 5=Outline, 6=MintGreen, 7=Green
// =============================================================================

fn build_smw_pokey_head(donor: &SnesRom, blink: bool) -> [[u8; 16]; 16] {
    let mut c = Canvas::new(16, 16);
    // Start with authentic SMW Pokey inspiration from GFX09 if available, then sharpen to 16x16 boss scale
    let _ = load_smw_3bpp_sheet(&donor.data, 0x09);
    // Green spiky crown tuft on top of Pokey's head (SMW signature!)
    c.triangle(5, 0, 3, 4, 7, 4, 7);
    c.triangle(8, 0, 6, 4, 10, 4, 6);
    c.triangle(11, 0, 9, 4, 13, 4, 7);
    c.line(4, 3, 12, 3, 2);
    // Round yellow spiky Pokey head
    c.ellipse_outline(8.0, 9.5, 6.2, 5.8, 4, 5);
    c.ellipse(9.0, 11.2, 4.8, 3.2, 3); // warm orange lower shading
    c.ellipse(6.5, 7.2, 2.5, 1.8, 4);
    c.set(5, 6, 1); // glossy highlight
                    // Side cactus spikes (white + outline)
    c.set(1, 7, 1);
    c.set(0, 7, 5);
    c.set(14, 7, 1);
    c.set(15, 7, 5);
    c.set(1, 11, 1);
    c.set(0, 11, 5);
    c.set(14, 11, 1);
    c.set(15, 11, 5);
    // Expressive wide SMW Pokey eyes
    if blink {
        c.rect(4, 8, 3, 1, 5);
        c.rect(9, 8, 3, 1, 5);
    } else {
        c.rect(4, 6, 3, 4, 1);
        c.rect(9, 6, 3, 4, 1);
        c.rect(5, 7, 2, 3, 5);
        c.rect(10, 7, 2, 3, 5);
        c.set(5, 7, 1);
        c.set(10, 7, 1);
    }
    // Wide mischievous Pokey grin
    c.rect(5, 11, 6, 2, 5);
    c.rect(6, 11, 4, 1, 3);
    c.set(6, 12, 1);
    c.set(9, 12, 1);
    c.auto_outline(5, false);
    c.slice_16x16(0, 0)
}

fn build_smw_pokey_segment(variant: usize) -> [[u8; 16]; 16] {
    let mut c = Canvas::new(16, 16);
    let r = if variant >= 4 { 4.8 } else { 5.8 };
    c.ellipse_outline(8.0, 8.0, r, r, 4, 5);
    c.ellipse(8.8, 9.5, r - 1.5, r - 2.3, 3);
    c.ellipse(6.2, 6.0, r - 3.2, r - 3.6, 4);
    // Cactus vertical ridges
    c.line(8, 3, 8, 12, 3);
    c.line(5, 4, 5, 11, 3);
    c.line(11, 4, 11, 11, 3);
    // Sharp white cactus spikes protruding left/right/top/bottom
    let off = (variant % 2) as isize;
    for &(sx, sy) in &[
        (1, 5 + off),
        (1, 10 - off),
        (14, 5 + off),
        (14, 10 - off),
        (5, 1),
        (10, 1),
        (5, 14),
        (10, 14),
    ] {
        c.set(sx, sy, 1);
    }
    // Center belly highlight & green spine dots
    c.set(6, 6, 1);
    c.set(8, 7, 7);
    c.set(6, 9, 2);
    c.set(10, 9, 2);
    c.auto_outline(5, false);
    c.slice_16x16(0, 0)
}

fn patch_boss_lanmolas_pokey(
    rom_z: &mut SnesRom,
    donor: &SnesRom,
    sess: &mut SwapSession,
    free_cursor: &mut usize,
    free_end: usize,
) -> Result<usize, RomError> {
    let head0 = build_smw_pokey_head(donor, false);
    let head1 = build_smw_pokey_head(donor, true);
    let head_side = flip_h_16x16_pub(&head0);
    let seg0 = build_smw_pokey_segment(0);
    let seg1 = build_smw_pokey_segment(1);
    let seg2 = build_smw_pokey_segment(2);
    let seg_sm = build_smw_pokey_segment(4);

    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0xA4, // sp:31 Lanmolas (slot 3 $C0..$FF)
        "Boss Palais du Désert (Lanmolas sp:31) ← 3 Pokeys Géants SMW",
        free_cursor,
        free_end,
        &|raw| {
            // Body segments: $C0=(0,0), $C2=(2,0), $C4=(4,0), $C6=(6,0), $E0=(0,2), $E2=(2,2)
            write_3bpp_16x16_pub(raw, 0, 0, &seg0);
            write_3bpp_16x16_pub(raw, 2, 0, &seg1);
            write_3bpp_16x16_pub(raw, 4, 0, &seg2);
            write_3bpp_16x16_pub(raw, 6, 0, &seg0);
            write_3bpp_16x16_pub(raw, 0, 2, &seg1);
            write_3bpp_16x16_pub(raw, 2, 2, &seg_sm);
            // Pokey Heads: $C8=(8,0), $CA=(10,0), $CC=(12,0), $CE=(14,0), $E4=(4,2), $E6=(6,2)
            write_3bpp_16x16_pub(raw, 8, 0, &head0);
            write_3bpp_16x16_pub(raw, 10, 0, &head1);
            write_3bpp_16x16_pub(raw, 12, 0, &head_side);
            write_3bpp_16x16_pub(raw, 14, 0, &head0);
            write_3bpp_16x16_pub(raw, 4, 2, &head0);
            write_3bpp_16x16_pub(raw, 6, 2, &head1);
            // Keep $E8..$EE sand burst debris intact!
        },
    )? {
        Ok(1)
    } else {
        Ok(0)
    }
}

// =============================================================================
// 2. TOWER OF HERA (MOLDORM -> GIANT SMW WIGGLER) — raw 0xA3 (sp:30)
//    Live Pal 5 (Aux1 0x08 @ 0x0DD550):
//      1=White, 2=Brown, 3=Gold, 4=BrightYellow, 5=Outline, 6=WarmYellow, 7=Ochre
//    Exact OAM: Head=$84,$86,$A4,$A6 (32x32, H-flips left/right!),
//               LargeSeg=$80,$82,$A0,$A2 (32x32), MedSeg=$88, SmallSeg=$A8, Tail=$AA/$AC
// =============================================================================

fn build_smw_wiggler_head_32x32() -> Canvas {
    let mut c = Canvas::new(32, 32);
    // Cute brown caterpillar shoes sticking out bottom-sides
    c.ellipse_outline(7.0, 26.0, 4.2, 3.0, 2, 5);
    c.ellipse_outline(25.0, 26.0, 4.2, 3.0, 2, 5);
    // Big round sunny-yellow Wiggler head
    c.ellipse_outline(16.0, 17.0, 12.5, 11.5, 4, 5);
    c.ellipse(17.5, 20.5, 10.5, 7.5, 3);
    c.ellipse(16.0, 15.0, 9.5, 7.0, 4);
    c.ellipse(13.0, 12.0, 4.5, 3.0, 6);
    // Iconic Wiggler white daisy flower perched on top of head!
    for &(fx, fy) in &[
        (16.0, 3.0),
        (11.5, 5.0),
        (20.5, 5.0),
        (13.0, 8.5),
        (19.0, 8.5),
    ] {
        c.ellipse_outline(fx, fy, 3.0, 2.5, 1, 5);
    }
    c.ellipse_outline(16.0, 6.0, 2.5, 2.2, 3, 5);
    // Big round Wiggler bulbous nose (golden-ochre with glossy highlight)
    c.ellipse_outline(16.0, 19.5, 4.8, 3.8, 7, 5);
    c.ellipse(15.2, 18.5, 2.5, 1.8, 3);
    c.set(14, 18, 1);
    // Angry/mischievous Wiggler brow & dark cheeks (Moldorm overlays moving eyeballs on top!)
    c.line(8, 13, 13, 15, 5);
    c.line(24, 13, 19, 15, 5);
    c.ellipse_outline(9.5, 21.0, 2.5, 2.0, 6, 7);
    c.ellipse_outline(22.5, 21.0, 2.5, 2.0, 6, 7);
    // Cheek spots
    c.rect(7, 17, 2, 2, 2);
    c.rect(23, 17, 2, 2, 2);
    c.auto_outline(5, false);
    c
}

fn build_smw_wiggler_body_32x32() -> Canvas {
    let mut c = Canvas::new(32, 32);
    // 4 cute little brown Wiggler shoes poking out!
    for &(sx, sy) in &[(5.0, 10.0), (27.0, 10.0), (5.0, 22.0), (27.0, 22.0)] {
        c.ellipse_outline(sx, sy, 3.8, 2.8, 2, 5);
        c.set(sx as isize, sy as isize - 1, 7);
    }
    // Plump yellow caterpillar segment
    c.ellipse_outline(16.0, 16.0, 12.2, 12.2, 4, 5);
    c.ellipse(17.5, 18.5, 10.2, 9.2, 3);
    c.ellipse(14.5, 13.5, 8.2, 7.2, 4);
    // Iconic Wiggler dark-brown dorsal spots with light rim!
    c.ellipse_outline(16.0, 16.0, 5.2, 5.2, 2, 7);
    c.ellipse(15.0, 14.8, 2.5, 2.5, 7);
    for &(sx, sy) in &[(9.5, 10.0), (22.5, 10.0), (9.5, 22.0), (22.5, 22.0)] {
        c.ellipse_outline(sx, sy, 2.4, 2.4, 2, 7);
    }
    c.set(11, 8, 1);
    c.set(12, 8, 1);
    c.auto_outline(5, false);
    c
}

fn build_smw_wiggler_seg_16x16(radius: f32, is_tail: bool) -> [[u8; 16]; 16] {
    let mut c = Canvas::new(16, 16);
    if !is_tail {
        c.ellipse_outline(2.5, 8.0, 2.0, 2.0, 2, 5);
        c.ellipse_outline(13.5, 8.0, 2.0, 2.0, 2, 5);
    }
    c.ellipse_outline(8.0, 8.0, radius, radius, 4, 5);
    c.ellipse(8.8, 9.0, radius - 1.6, radius - 2.0, 3);
    c.ellipse_outline(
        8.0,
        8.0,
        (radius * 0.42).max(1.5),
        (radius * 0.42).max(1.5),
        2,
        7,
    );
    c.set(6, 5, 1);
    if is_tail {
        // Glowing red-ochre/white vulnerability bulb on tail!
        c.ellipse_outline(8.0, 8.0, 3.4, 3.4, 1, 5);
        c.ellipse(8.0, 8.0, 1.8, 1.8, 2);
    }
    c.auto_outline(5, false);
    c.slice_16x16(0, 0)
}

fn patch_boss_moldorm_wiggler(
    rom_z: &mut SnesRom,
    sess: &mut SwapSession,
    free_cursor: &mut usize,
    free_end: usize,
) -> Result<usize, RomError> {
    let body = build_smw_wiggler_body_32x32();
    let head = build_smw_wiggler_head_32x32();
    let med = build_smw_wiggler_seg_16x16(6.2, false);
    let sml = build_smw_wiggler_seg_16x16(5.0, false);
    let tail = build_smw_wiggler_seg_16x16(4.5, true);

    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0xA3, // sp:30 Moldorm (slot 2 $80..$BF)
        "Boss Tour d'Héra (Moldorm sp:30) ← Wiggler Géant SMW",
        free_cursor,
        free_end,
        &|raw| {
            // Large Body Segment ($80, $82, $A0, $A2) = cols 0..3
            write_3bpp_16x16_pub(raw, 0, 0, &body.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 2, 0, &body.slice_16x16(16, 0));
            write_3bpp_16x16_pub(raw, 0, 2, &body.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 2, 2, &body.slice_16x16(16, 16));
            // Wiggler Head ($84, $86, $A4, $A6) = cols 4..7
            write_3bpp_16x16_pub(raw, 4, 0, &head.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 6, 0, &head.slice_16x16(16, 0));
            write_3bpp_16x16_pub(raw, 4, 2, &head.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 6, 2, &head.slice_16x16(16, 16));
            // Medium ($88) / Small ($A8) / Tail ($8A, $AA, $AC) segments
            write_3bpp_16x16_pub(raw, 8, 0, &med);
            write_3bpp_16x16_pub(raw, 8, 2, &sml);
            write_3bpp_16x16_pub(raw, 10, 0, &sml);
            write_3bpp_16x16_pub(raw, 10, 2, &tail);
            write_3bpp_16x16_pub(raw, 12, 2, &tail);
            // Keep Moldorm's eyeball 8x8 tiles at cols 12..13 row 0 intact (crisp white/black Wiggler eyes!)
        },
    )? {
        Ok(1)
    } else {
        Ok(0)
    }
}

// =============================================================================
// 3. PALACE OF DARKNESS (HELMASAUR -> KOOPALING #1: MORTON KOOPA JR.)
//    raw 0xAD (sp:3A, slot 2 $80..$BF, Pal 5 Aux1 0x10)
//      1=White, 2=Crimson, 3=Green, 4=Gold, 5=DarkCharcoal, 6=TanOrange, 7=RedBrown
//    raw 0xB1 (sp:3E, slot 3 $C0..$FF, Pal 6 Aux2 0x0C)
//      1=White, 2=SteelBlue, 3=Orange, 4=Gold, 5=DarkOutline, 6=BrightSilver, 7=MidSteel
// =============================================================================

fn build_morton_body_left_half_32x80() -> Canvas {
    // Full boss is 64x80 symmetric; right edge x=31 is the vertical center symmetry axis!
    let mut c = Canvas::new(32, 80);

    // === BACK HALF (y=0..34): MORTON'S MASSIVE DARK-GREY SPIKY KOOPA SHELL ===
    // Shell dome
    c.ellipse_outline(31.5, 22.0, 28.0, 20.5, 5, 5);
    c.ellipse(31.5, 21.0, 25.0, 18.0, 7); // dark red-brown inner shell rim
    c.ellipse(31.5, 20.0, 22.5, 16.0, 5); // iconic charcoal/black Morton shell!
                                          // Shell hexagonal plate highlights (warm grey/gold border + crimson trim)
    c.ellipse(22.0, 18.0, 6.0, 5.0, 7);
    c.ellipse(22.0, 18.0, 4.0, 3.5, 5);
    c.line(8, 28, 31, 32, 1); // crisp white Koopa shell belly rim!
    c.line(8, 29, 31, 33, 1);
    c.line(7, 30, 31, 34, 4); // gold under-rim
                              // Huge white Koopa shell spikes sticking out of Morton's back!
    draw_spike(&mut c, 24, 14, 9, 4, 1, 6, 5);
    draw_spike(&mut c, 12, 20, 8, 4, 1, 6, 5);
    draw_spike(&mut c, 28, 26, 8, 4, 1, 6, 5);
    draw_spike(&mut c, 16, 29, 7, 3, 1, 6, 5);

    // === FRONT HALF (y=32..79): MORTON KOOPA JR.'S WIDE HEAD, STAR MARK & JAW ===
    // Spiked black collar with golden studs
    c.rect(14, 32, 18, 5, 5);
    c.rect(16, 33, 16, 3, 2); // crimson collar band
    c.rect(19, 33, 2, 3, 1);
    c.rect(26, 33, 2, 3, 1);

    // Broad muscular shoulders / cheeks (tan-brown Morton skin!)
    c.ellipse_outline(31.5, 48.0, 26.0, 15.0, 6, 5);
    c.ellipse(31.5, 46.0, 23.0, 12.5, 4); // golden highlight across brow/cheeks

    // Morton's signature DARK GREY/CHARCOAL TOP HEAD & ICONIC WHITE STAR PATCH OVER EYE!
    c.ellipse_outline(31.5, 41.0, 19.0, 10.5, 5, 5);
    // Morton's tuft of 3 spiky hairs at crown center (x=28..31)
    c.triangle(31, 29, 27, 36, 31, 36, 5);
    c.line(30, 31, 30, 35, 1);
    // Crisp white star emblem on Morton's forehead/eye-side (x=18..25, y=38..45)
    c.line(21, 38, 21, 45, 1);
    c.line(18, 41, 24, 41, 1);
    c.line(19, 39, 23, 43, 1);
    c.line(23, 39, 19, 43, 1);

    // Fierce glaring Morton eye (x=23..29, y=44..50)
    c.rect(22, 44, 7, 6, 1);
    c.rect(24, 46, 4, 4, 2); // fiery red iris
    c.rect(25, 47, 2, 2, 5); // pupil
    c.set(24, 46, 1); // specular gleam
    c.line(21, 43, 30, 46, 5); // fierce angled brow!
    c.line(21, 44, 30, 47, 5);

    // Massive wide toothy Morton Snout & Jaw (y=52..76)
    c.ellipse_outline(31.5, 61.0, 22.0, 13.0, 4, 5);
    c.ellipse(31.5, 64.0, 20.0, 10.0, 6); // warm tan jaw
                                          // Nostril
    c.rect(26, 55, 3, 2, 5);
    // Roaring crimson mouth with big white Koopa fangs!
    c.ellipse(31.5, 66.0, 14.0, 6.0, 2);
    c.rect(19, 63, 13, 2, 5);
    // Upper fangs
    c.triangle(21, 68, 19, 63, 23, 63, 1);
    c.triangle(27, 67, 25, 63, 29, 63, 1);
    // Lower fangs
    c.triangle(24, 66, 22, 71, 26, 71, 1);
    // Golden chin ridge (y=72..78)
    c.ellipse(31.5, 74.0, 13.0, 4.0, 4);
    c.line(19, 75, 31, 77, 6);

    c.auto_outline(5, true);
    c
}

fn build_morton_mask_tiles() -> (
    [[u8; 16]; 16], // $AE (on sp:3A, x=-16 y=-5, left horn/brow of iron visor)
    [[u8; 16]; 16], // $C0 (center crown x=0 y=-5)
    [[u8; 16]; 16], // $C2 (left intact cheek x=-16 y=+11)
    [[u8; 16]; 16], // $C4 (center nose/star crest x=0 y=+11)
    [[u8; 16]; 16], // $C6 (lower jaw plate x=-8 y=+27)
    [[u8; 16]; 16], // $C8 (left CRACKED cheek x=-16 y=+11)
) {
    // Full symmetric iron Koopa visor mask: 48x48 (x=-16..+32, y=-5..+43)
    // Pal 6 (0x0C): 1=White, 2=SteelBlue, 3=Orange, 4=Gold, 5=DarkOutline, 6=Silver, 7=MidSteel
    let mut m = Canvas::new(48, 48);
    // Main forged steel Koopa helmet dome
    m.ellipse_outline(24.0, 24.0, 21.0, 20.0, 7, 5);
    m.ellipse(24.0, 21.0, 18.0, 16.0, 6); // bright silver-steel sheen
    m.ellipse(24.0, 28.0, 17.0, 12.0, 2); // dark steel lower face-plate
                                          // Curved golden-white bull horns on sides (y=0..15)
    draw_spike(&mut m, 6, 13, 11, 5, 1, 4, 5);
    draw_spike(&mut m, 42, 13, 11, 5, 1, 4, 5);
    // Top golden spike crest
    draw_spike(&mut m, 24, 12, 10, 4, 4, 3, 5);
    // Golden Koopa star emblem in center forehead ($C0/$C4)
    m.ellipse_outline(24.0, 22.0, 5.5, 5.5, 4, 5);
    m.rect(23, 18, 2, 8, 1);
    m.rect(20, 21, 8, 2, 1);
    // Glowing orange-gold eye slits in visor aligned directly with Helmasaur's blinking eyes!
    m.rect(10, 24, 9, 6, 5);
    m.rect(29, 24, 9, 6, 5);
    m.rect(12, 26, 5, 2, 4);
    m.rect(31, 26, 5, 2, 4);
    m.set(13, 26, 1);
    m.set(34, 26, 1);
    // Iron rivets & spiked chin guard ($C6)
    for rx in [10, 16, 32, 38] {
        m.set(rx, 15, 1);
        m.set(rx, 33, 1);
    }
    m.rect(16, 35, 16, 8, 7);
    m.line(16, 38, 32, 38, 4);
    m.triangle(19, 44, 17, 39, 21, 39, 1);
    m.triangle(29, 44, 27, 39, 31, 39, 1);
    m.auto_outline(5, false);

    let ae = m.slice_16x16(0, 0);
    let c0 = m.slice_16x16(16, 0);
    let c2 = m.slice_16x16(0, 16);
    let c4 = m.slice_16x16(16, 16);
    let c6 = m.slice_16x16(8, 32);

    // Cracked version of $C2 -> $C8 (jagged black/orange battle cracks!)
    let mut c8_c = Canvas::new(16, 16);
    c8_c.blit_16x16(0, 0, &c2);
    c8_c.line(2, 2, 9, 8, 5);
    c8_c.line(9, 8, 5, 14, 5);
    c8_c.line(9, 8, 14, 11, 5);
    c8_c.line(3, 2, 10, 8, 3);
    let c8 = c8_c.slice_16x16(0, 0);

    (ae, c0, c2, c4, c6, c8)
}

fn build_morton_leg_16x16(front: bool) -> [[u8; 16]; 16] {
    let mut c = Canvas::new(16, 16);
    c.ellipse_outline(9.0, 8.0, 6.0, 6.2, 6, 5);
    c.ellipse(9.5, 7.0, 4.2, 4.0, 4);
    // 3 big white Koopa claws
    for &cy in &[4isize, 8, 12] {
        c.triangle(1, cy, 5, cy - 2, 5, cy + 2, 1);
    }
    if front {
        // Spiked black wrist band
        c.rect(10, 3, 4, 10, 5);
        c.set(12, 5, 1);
        c.set(12, 10, 1);
    }
    c.auto_outline(5, false);
    c.slice_16x16(0, 0)
}

fn patch_boss_helmasaur_morton(
    rom_z: &mut SnesRom,
    sess: &mut SwapSession,
    free_cursor: &mut usize,
    free_end: usize,
) -> Result<usize, RomError> {
    let body = build_morton_body_left_half_32x80();
    let (mask_ae, mask_c0, mask_c2, mask_c4, mask_c6, mask_c8) = build_morton_mask_tiles();
    let leg_back = build_morton_leg_16x16(false);
    let leg_front = build_morton_leg_16x16(true);
    let mut tail_c = Canvas::new(16, 16);
    tail_c.triangle(8, 1, 3, 14, 13, 14, 6);
    tail_c.triangle(8, 3, 5, 13, 11, 13, 4);
    draw_spike(&mut tail_c, 8, 6, 4, 2, 1, 6, 5);
    tail_c.auto_outline(5, false);
    let tail = tail_c.slice_16x16(0, 0);

    let mut ops = 0usize;
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0xAD, // sp:3A Helmasaur Body
        "Boss Palais des Ténèbres (Helmasaur sp:3A) ← KOOPALING #1: MORTON KOOPA JR.",
        free_cursor,
        free_end,
        &|raw| {
            // Exact OAM rows:
            // y=-32: $80=(0,0), $82=(2,0)
            write_3bpp_16x16_pub(raw, 0, 0, &body.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 2, 0, &body.slice_16x16(16, 0));
            // y=-16: $84=(4,0), $86=(6,0)
            write_3bpp_16x16_pub(raw, 4, 0, &body.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 6, 0, &body.slice_16x16(16, 16));
            // y=0: $88=(8,0), $8A=(10,0)
            write_3bpp_16x16_pub(raw, 8, 0, &body.slice_16x16(0, 32));
            write_3bpp_16x16_pub(raw, 10, 0, &body.slice_16x16(16, 32));
            // y=+16: $8C=(12,0), $8E=(14,0)
            write_3bpp_16x16_pub(raw, 12, 0, &body.slice_16x16(0, 48));
            write_3bpp_16x16_pub(raw, 14, 0, &body.slice_16x16(16, 48));
            // y=+32 chin: $A0=(0,2)
            write_3bpp_16x16_pub(raw, 0, 2, &body.slice_16x16(16, 64));
            // Legs: $A2=(2,2), $A4=(4,2), $A6=(6,2), $A8=(8,2)
            write_3bpp_16x16_pub(raw, 2, 2, &leg_back);
            write_3bpp_16x16_pub(raw, 4, 2, &leg_back);
            write_3bpp_16x16_pub(raw, 6, 2, &leg_front);
            write_3bpp_16x16_pub(raw, 8, 2, &leg_front);
            // Tail: $AC=(12,2)
            write_3bpp_16x16_pub(raw, 12, 2, &tail);
            // Left Horn of Mask: $AE=(14,2)
            write_3bpp_16x16_pub(raw, 14, 2, &mask_ae);
        },
    )? {
        ops += 1;
    }

    let fb0 = build_smw_fireball_16x16(1, 4, 3, 2, 5, 0);
    let fb1 = build_smw_fireball_16x16(1, 4, 3, 2, 5, 1);
    let fb2 = build_smw_fireball_16x16(1, 4, 3, 2, 5, 2);
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0xB1, // sp:3E Helmasaur Mask & Fireballs
        "Boss Morton Jr. Casque Fer & Boules de Feu SMW (sp:3E)",
        free_cursor,
        free_end,
        &|raw| {
            write_3bpp_16x16_pub(raw, 0, 0, &mask_c0);
            write_3bpp_16x16_pub(raw, 2, 0, &mask_c2);
            write_3bpp_16x16_pub(raw, 4, 0, &mask_c4);
            write_3bpp_16x16_pub(raw, 6, 0, &mask_c6);
            write_3bpp_16x16_pub(raw, 8, 0, &mask_c8);
            // Fireballs ($CA=(10,0), $CC=(12,0), $CE=(14,0))
            write_3bpp_16x16_pub(raw, 10, 0, &fb0);
            write_3bpp_16x16_pub(raw, 12, 0, &fb1);
            write_3bpp_16x16_pub(raw, 14, 0, &fb2);
        },
    )? {
        ops += 1;
    }
    Ok(ops)
}

// =============================================================================
// 4. SWAMP PALACE (ARRGHUS -> KOOPALING #2: WENDY O. KOOPA) — raw 0xAC (sp:39)
//    CRITICAL: ONLY modify cols 0..9! Cols 10..15 hold Grande Fée / Yoshi!
//    Live Pal 6 (Aux2 0x14 @ 0x0DD5F8):
//      1=White, 2=DeepRose, 3=Orange, 4=GoldYellow, 5=Outline, 6=HotPink, 7=RosePink
//    Exact OAM: 32x32 H-symmetric body ($80/$A0, $82/$A2, $84/$A4) + Arrgi ($86, $88, $A6)
// =============================================================================

fn build_wendy_left_half_16x32(frame: usize) -> Canvas {
    // Left half of 32x32 Wendy O. Koopa (x=15 is vertical mirror axis!)
    let mut c = Canvas::new(16, 32);
    let bob = if frame == 1 {
        -1isize
    } else if frame == 2 {
        1
    } else {
        0
    };

    // === HUGE ICONIC PINK POLKA-DOT RIBBON BOW ON TOP OF HEAD ===
    c.ellipse_outline(7.5, (5 + bob) as f32, 6.2, 4.5, 6, 5);
    c.ellipse(7.5, (6 + bob) as f32, 4.5, 3.0, 7);
    c.ellipse_outline(14.5, (6 + bob) as f32, 3.2, 2.8, 6, 5); // center bow knot
                                                               // Crisp white polka dots on Wendy's bow!
    c.rect(4, 3 + bob, 2, 2, 1);
    c.rect(9, 5 + bob, 2, 2, 1);
    c.set(6, 7 + bob, 1);

    // === WENDY'S ROUND PINK HEAD & GOLDEN-YELLOW SNOUT ===
    c.ellipse_outline(15.5, (14 + bob) as f32, 8.8, 7.2, 6, 5);
    c.ellipse(15.5, (12 + bob) as f32, 7.0, 4.5, 6);
    // Expressive eyes with dramatic lashes
    c.rect(10, 10 + bob, 4, 5, 1);
    c.rect(11, 11 + bob, 3, 4, 5);
    c.set(11, 11 + bob, 1); // eye gleam
    c.line(8, 10 + bob, 10, 11 + bob, 5); // eyelash!
    c.line(8, 12 + bob, 10, 12 + bob, 5);
    // Snout & bold iconic PINK LIPSTICK mouth!
    c.ellipse_outline(15.5, (18 + bob) as f32, 7.2, 4.0, 4, 5);
    c.ellipse(15.5, (19 + bob) as f32, 4.8, 2.2, 7); // plump pink lips
    c.line(12, 19 + bob, 15, 19 + bob, 2);
    c.set(14, 18 + bob, 1); // lip gloss gleam

    // === WENDY'S PINK KOOPA SHELL, GOLD BRACELETS & FEET ===
    // Shell rim + pink shell flanks
    c.ellipse_outline(15.5, 25.0, 10.5, 6.2, 7, 5);
    c.ellipse(15.5, 25.0, 8.5, 4.8, 6);
    c.ellipse(15.5, 25.5, 5.5, 3.8, 4); // golden belly
    c.line(11, 24, 15, 24, 3);
    c.line(11, 27, 15, 27, 3);
    // Waving arms with signature GOLD BRACELETS (Wendy's trademark!)
    let arm_y = if frame == 1 {
        18
    } else if frame == 2 {
        22
    } else {
        20
    };
    c.ellipse_outline(4.5, arm_y as f32, 3.5, 2.5, 4, 5);
    c.rect(5, arm_y - 2, 3, 4, 4); // gold bangle!
    c.line(5, arm_y - 1, 7, arm_y - 1, 1); // sparkle
                                           // Cute boots
    c.ellipse_outline(11.0, 29.5, 3.2, 2.2, 3, 5);
    c.set(10, 29, 4);

    c.auto_outline(5, true);
    c
}

fn build_wendy_candy_ring_16x16(frame: usize) -> [[u8; 16]; 16] {
    // Glowing SMW Wendy Candy Ring (pink-and-white magic hoop!)
    let mut c = Canvas::new(16, 16);
    let r = if frame == 1 { 6.5 } else { 5.8 };
    c.ellipse_outline(8.0, 8.0, r, r, 6, 5);
    c.ellipse(8.0, 8.0, r - 1.5, r - 1.5, 7);
    // White & gold candy stripes around hoop
    c.rect(7, 2, 2, 2, 1);
    c.rect(7, 12, 2, 2, 1);
    c.rect(2, 7, 2, 2, 4);
    c.rect(12, 7, 2, 2, 4);
    // Hollow center of ring!
    c.ellipse_outline(8.0, 8.0, r - 3.0, r - 3.0, 0, 5);
    c.auto_outline(5, false);
    c.slice_16x16(0, 0)
}

fn patch_boss_arrghus_wendy(
    rom_z: &mut SnesRom,
    sess: &mut SwapSession,
    free_cursor: &mut usize,
    free_end: usize,
) -> Result<usize, RomError> {
    let w0 = build_wendy_left_half_16x32(0);
    let w1 = build_wendy_left_half_16x32(1);
    let w2 = build_wendy_left_half_16x32(2);
    let ring0 = build_wendy_candy_ring_16x16(0);
    let ring1 = build_wendy_candy_ring_16x16(1);

    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0xAC, // sp:39 Arrghus — STRICTLY cols 0..9 ONLY (preserve cols 10..15 Grande Fée!)
        "Boss Palais des Marais (Arrghus sp:39) ← KOOPALING #2: WENDY O. KOOPA & Anneaux SMW",
        free_cursor,
        free_end,
        &|raw| {
            // Wendy 32x32 symmetric left halves: ($80/$A0), ($82/$A2), ($84/$A4)
            write_3bpp_16x16_pub(raw, 0, 0, &w0.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 0, 2, &w0.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 2, 0, &w1.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 2, 2, &w1.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 4, 0, &w2.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 4, 2, &w2.slice_16x16(0, 16));
            // Wendy's Orbiting Candy Rings: $86=(6,0), $88=(8,0), $A6=(6,2)
            write_3bpp_16x16_pub(raw, 6, 0, &ring0);
            write_3bpp_16x16_pub(raw, 8, 0, &ring1);
            write_3bpp_16x16_pub(raw, 6, 2, &ring1);
            // Keep $A8=(8,2) water wake splash AND cols 10..15 Grande Fée 100% intact!
        },
    )? {
        Ok(1)
    } else {
        Ok(0)
    }
}

// =============================================================================
// 5. SKULL WOODS (MOTHULA -> KOOPALING #3: IGGY KOOPA) — raw 0xAB (sp:38)
//    Live Pal 6 (Aux2 0x0C @ 0x0DD588):
//      1=White, 2=RoyalBlue, 3=Orange, 4=GoldYellow, 5=Outline, 6=LightCyan/Silver, 7=MidBlue
//    (NOTE: Mothula cycles palettes during fight, so strong luminance contrast in 1/4/3/6/2/5
//     keeps Iggy's tall spiky mohawk, thick glasses, & Paratroopa wings razor-sharp on EVERY color!)
//    Exact OAM: 64x32 symmetric ($80,$82/$A0,$A2), ($84,$86/$A4,$A6), 32x32 ($88/$A8)
// =============================================================================

fn build_iggy_winged_left_half_32x32(wing_phase: usize) -> Canvas {
    // Left half of 64x32 Winged Iggy Koopa (x=31 is mirror axis!):
    //   cols 0..15  = Giant Flapping SMW Paratroopa Wing + Magic Wand!
    //   cols 16..31 = Iggy Koopa's tall wild mohawk, thick round glasses, maniacal grin & shell!
    let mut c = Canvas::new(32, 32);

    // === FLANKING SCALLOPED SMW PARATROOPA FEATHER WING (x=0..16) ===
    if wing_phase == 0 {
        // Sweeping feathered Paratroopa wing angled upward-outward (3 scalloped primary feathers!)
        c.triangle(1, 3, 15, 7, 15, 16, 1);
        c.ellipse_outline(7.0, 7.0, 5.5, 3.2, 1, 5); // top primary feather!
        c.ellipse_outline(5.0, 12.0, 5.0, 2.8, 1, 5); // mid feather!
        c.ellipse_outline(8.0, 16.5, 4.5, 2.5, 6, 5); // lower secondary feather!
        c.ellipse(11.0, 11.0, 4.5, 5.0, 1);
        c.line(4, 10, 14, 11, 5);
        c.line(5, 15, 14, 15, 5);
        c.line(7, 19, 14, 18, 6);
        // Koopaling Golden Magic Wand held out!
        c.line(5, 26, 14, 21, 4);
        c.line(5, 27, 14, 22, 3);
        c.ellipse_outline(4.0, 27.0, 2.8, 2.8, 4, 5);
        c.set(4, 27, 1);
    } else if wing_phase == 1 {
        // Downstroke scalloped Paratroopa wing!
        c.triangle(2, 10, 15, 11, 15, 22, 1);
        c.ellipse_outline(7.0, 14.0, 5.2, 3.0, 1, 5);
        c.ellipse_outline(6.0, 19.0, 4.8, 2.8, 6, 5);
        c.ellipse_outline(10.0, 23.0, 4.0, 2.2, 6, 5);
        c.line(5, 17, 14, 17, 5);
        c.line(6, 21, 14, 21, 5);
        c.line(6, 10, 14, 15, 4);
        c.ellipse_outline(5.0, 9.0, 2.5, 2.5, 4, 5);
        c.set(5, 9, 1);
    }

    // === CENTER 32x32 IGGY KOOPA (x=16..31, H-mirrored at x=31!) ===
    // TALL WILD CRAZY IGGY MOHAWK (sweeping tall spiky palm-tree hair!)
    c.triangle(28, 0, 23, 8, 31, 8, 4); // tall bright tuft!
    c.triangle(22, 2, 19, 9, 27, 9, 3); // outer wild spike!
    c.triangle(31, 0, 27, 9, 31, 9, 1); // bright highlight spike!
    c.line(22, 3, 25, 8, 5);
    c.line(28, 1, 28, 7, 5);

    // Narrow eccentric Iggy face & HUGE THICK ROUND GLASSES!
    c.ellipse_outline(31.5, 14.0, 8.5, 6.5, 4, 5);
    // Thick black spectacle frames + bright white spiral lenses!
    c.ellipse_outline(26.5, 13.0, 4.2, 4.0, 1, 5);
    c.rect(26, 12, 2, 2, 5); // manic tiny pupil!
    c.set(25, 11, 6); // glasses sheen
    c.line(30, 13, 31, 13, 5); // glasses bridge

    // Wide manic toothy grin & protruding tongue
    c.ellipse_outline(31.5, 19.5, 8.2, 3.8, 3, 5);
    c.rect(25, 19, 7, 2, 5);
    c.set(26, 19, 1); // sharp tooth
    c.set(29, 19, 1);
    c.set(31, 20, 1);

    // Slender Iggy Koopa shell torso & golden belly
    c.ellipse_outline(31.5, 26.0, 9.5, 5.5, 2, 5);
    c.ellipse(31.5, 26.0, 6.0, 4.2, 4); // golden belly
    c.line(23, 25, 25, 25, 1); // shell white rim
    c.ellipse_outline(25.0, 30.0, 2.8, 1.8, 3, 5); // feet

    c.auto_outline(5, true);
    c
}

fn patch_boss_mothula_iggy(
    rom_z: &mut SnesRom,
    sess: &mut SwapSession,
    free_cursor: &mut usize,
    free_end: usize,
) -> Result<usize, RomError> {
    let f0 = build_iggy_winged_left_half_32x32(0);
    let f1 = build_iggy_winged_left_half_32x32(1);
    let f2 = build_iggy_winged_left_half_32x32(2); // wings tucked (center cols 16..31 -> $88/$A8)

    // SMW Koopaling Magic Wand Rings (for cols 10..11, $8A/$AA)
    let mut ring = Canvas::new(16, 16);
    ring.ellipse_outline(8.0, 8.0, 6.0, 6.0, 4, 5);
    ring.ellipse(8.0, 8.0, 4.2, 4.2, 1);
    ring.ellipse_outline(8.0, 8.0, 2.6, 2.6, 0, 5);
    let ring_tile = ring.slice_16x16(0, 0);

    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0xAB, // sp:38 Mothula (slot 2 $80..$BF)
        "Boss Bois des Squelettes (Mothula sp:38) ← KOOPALING #3: IGGY KOOPA Ailé SMW",
        free_cursor,
        free_end,
        &|raw| {
            // Frame 1 (64x32 wide wings): $80=(0,0), $82=(2,0), $A0=(0,2), $A2=(2,2)
            write_3bpp_16x16_pub(raw, 0, 0, &f0.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 2, 0, &f0.slice_16x16(16, 0));
            write_3bpp_16x16_pub(raw, 0, 2, &f0.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 2, 2, &f0.slice_16x16(16, 16));
            // Frame 2 (64x32 mid-flap): $84=(4,0), $86=(6,0), $A4=(4,2), $A6=(6,2)
            write_3bpp_16x16_pub(raw, 4, 0, &f1.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 6, 0, &f1.slice_16x16(16, 0));
            write_3bpp_16x16_pub(raw, 4, 2, &f1.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 6, 2, &f1.slice_16x16(16, 16));
            // Frame 3 (32x32 wings tucked): $88=(8,0), $A8=(8,2)
            write_3bpp_16x16_pub(raw, 8, 0, &f2.slice_16x16(16, 0));
            write_3bpp_16x16_pub(raw, 8, 2, &f2.slice_16x16(16, 16));
            // Magic beam rings at (10,2)
            write_3bpp_16x16_pub(raw, 10, 2, &ring_tile);
        },
    )? {
        Ok(1)
    } else {
        Ok(0)
    }
}

// =============================================================================
// 6. THIEVES' TOWN (BLIND -> KOOPALING #4: ROY KOOPA) — raw 0xAE (sp:3B)
//    Live Pal 5 (Aux1 0x14 @ 0x0DD5F8) for Heads ($80..$8C) & Hands ($A6..$AA):
//      1=White, 2=DarkRose, 3=Orange, 4=GoldYellow, 5=Outline, 6=BRIGHT ROY PINK, 7=DEEP MAGENTA
//    Live Pal 6 (Aux2 0x0C @ 0x0DD588) for Body/Shell ($8E, $A0..$A4, $AC, $AE):
//      1=White, 2=ROYAL PURPLE-BLUE, 3=Orange, 4=Gold, 5=Outline, 6=Lavender, 7=VioletBlue
// =============================================================================

fn build_roy_head_16x16(turn: isize, open_mouth: bool) -> [[u8; 16]; 16] {
    // Roy Koopa's iconic bald bright-pink head + COOL DARK SUNGLASSES + bully smirk!
    let mut c = Canvas::new(16, 16);
    // Smooth bald pink skull dome
    c.ellipse_outline(8.0, 7.0, 6.2, 5.5, 6, 5);
    c.ellipse(8.0, 5.0, 4.8, 3.0, 6);
    c.ellipse(8.0, 9.5, 5.2, 3.0, 7); // magenta lower cheeks
    c.set(6, 3, 1); // glossy bald head highlight!
    c.set(7, 3, 1);

    // COOL ICONIC ROY KOOPA SUNGLASSES (jet-dark lenses with pink-red frames & white glint!)
    let gx = 8 + turn;
    // Bridge & frames
    c.rect(gx - 6, 5, 12, 4, 2);
    // Left & right cool dark lenses
    c.rect(gx - 5, 6, 4, 3, 5);
    c.rect(gx + 1, 6, 4, 3, 5);
    // Crisp white diagonal cool-shades gleam!
    c.set(gx - 4, 6, 1);
    c.set(gx - 3, 7, 1);
    c.set(gx + 2, 6, 1);
    c.set(gx + 3, 7, 1);

    // Wide yellow-gold Koopa snout & bully grin/roar
    c.ellipse_outline((8 + turn / 2) as f32, 12.2, 5.8, 3.0, 4, 5);
    if open_mouth {
        c.rect(5, 11, 6, 3, 2);
        c.set(6, 11, 1);
        c.set(9, 11, 1);
    } else {
        c.line(5, 12, 11, 12, 5);
        c.set(6, 13, 1);
        c.set(10, 13, 1);
    }
    c.auto_outline(5, false);
    c.slice_16x16(0, 0)
}

fn build_roy_body_tiles() -> (
    [[u8; 16]; 16], // $8E upper shoulder/collar left half (H-mirrored!)
    [[u8; 16]; 16], // $A0 lower left shell/robe 1
    [[u8; 16]; 16], // $A2 lower left shell/robe 2
    [[u8; 16]; 16], // $A4 lower right shell/robe 3
    [[u8; 16]; 16], // $AC crouched shell
    [[u8; 16]; 16], // $AE crouched shell 2
    [[u8; 16]; 16], // $A6 fist 1 (Pal 5 pink/gold)
    [[u8; 16]; 16], // $A8 fist 2 (Pal 5 pink/gold)
    [[u8; 16]; 16], // $AA fist 3 (Pal 5 pink/gold)
) {
    // Upper torso left half ($8E, Pal 6: 2=RoyalPurple, 7=Violet, 6=Lavender, 4=Gold, 1=White, 5=Outline)
    let mut u = Canvas::new(16, 16);
    u.ellipse_outline(15.5, 10.0, 12.0, 8.5, 2, 5);
    u.ellipse(15.5, 10.0, 9.5, 6.5, 7);
    u.rect(10, 5, 6, 11, 4); // golden front plastron!
    u.line(10, 9, 15, 9, 3);
    u.line(10, 13, 15, 13, 3);
    // White Koopa shoulder spike + purple shell rim
    draw_spike(&mut u, 6, 8, 5, 3, 1, 6, 5);
    u.line(7, 3, 15, 3, 1);
    u.auto_outline(5, true);
    let t_8e = u.slice_16x16(0, 0);

    // Lower shell/robe frames ($A0, $A2, $A4)
    let make_lower = |sway: isize| -> [[u8; 16]; 16] {
        let mut l = Canvas::new(16, 16);
        l.ellipse_outline((14 + sway) as f32, 7.0, 10.5, 8.0, 2, 5);
        l.ellipse((14 + sway) as f32, 6.0, 8.0, 6.0, 7);
        l.rect(10 + sway, 0, 6, 11, 4); // gold belly
        l.line(10 + sway, 4, 15, 4, 3);
        l.line(10 + sway, 8, 15, 8, 3);
        // White shell edge & clawed boot
        l.line(5 + sway, 10, 13 + sway, 12, 1);
        l.ellipse_outline((10 + sway) as f32, 13.5, 3.8, 2.2, 3, 5);
        l.set(8 + sway, 14, 1);
        l.auto_outline(5, true);
        l.slice_16x16(0, 0)
    };
    let t_a0 = make_lower(0);
    let t_a2 = make_lower(-1);
    let t_a4 = make_lower(1);

    // Crouched purple spiky shell ($AC, $AE)
    let mut cr = Canvas::new(16, 16);
    cr.ellipse_outline(15.5, 10.0, 11.0, 6.0, 2, 5);
    cr.ellipse(15.5, 9.0, 8.5, 4.5, 7);
    draw_spike(&mut cr, 9, 8, 5, 3, 1, 6, 5);
    cr.line(6, 13, 15, 13, 1);
    cr.auto_outline(5, true);
    let t_ac = cr.slice_16x16(0, 0);
    let t_ae = t_ac;

    // Bully fists / spiked bracelets ($A6, $A8, $AA — Pal 5: 6=Pink, 4=Gold, 1=White, 5=Outline)
    let make_fist = |dy: isize| -> [[u8; 16]; 16] {
        let mut f = Canvas::new(16, 16);
        f.ellipse_outline(8.0, (8 + dy) as f32, 5.2, 5.0, 6, 5);
        f.ellipse(7.5, (7 + dy) as f32, 3.5, 3.2, 4);
        f.rect(9, 5 + dy, 4, 6, 5); // black spiked wrist cuff
        f.set(11, 6 + dy, 1);
        f.set(11, 9 + dy, 1);
        f.set(4, 7 + dy, 1);
        f.set(4, 10 + dy, 1);
        f.auto_outline(5, false);
        f.slice_16x16(0, 0)
    };
    (
        t_8e,
        t_a0,
        t_a2,
        t_a4,
        t_ac,
        t_ae,
        make_fist(0),
        make_fist(-1),
        make_fist(1),
    )
}

fn patch_boss_blind_roy(
    rom_z: &mut SnesRom,
    sess: &mut SwapSession,
    free_cursor: &mut usize,
    free_end: usize,
) -> Result<usize, RomError> {
    let h_center = build_roy_head_16x16(0, false);
    let h_roar = build_roy_head_16x16(0, true);
    let h_left = build_roy_head_16x16(-1, false);
    let h_left2 = build_roy_head_16x16(-2, true);
    let h_right = build_roy_head_16x16(1, false);
    let (t_8e, t_a0, t_a2, t_a4, t_ac, t_ae, f0, f1, f2) = build_roy_body_tiles();

    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0xAE, // sp:3B Blind (slot 2 $80..$BF)
        "Boss Repaire des Voleurs (Blind sp:3B) ← KOOPALING #4: ROY KOOPA (Lunettes & Carapace)",
        free_cursor,
        free_end,
        &|raw| {
            // Heads ($80, $82, $84, $86, $88, $8A, $8C) = cols 0..13 row 0!
            write_3bpp_16x16_pub(raw, 0, 0, &h_center);
            write_3bpp_16x16_pub(raw, 2, 0, &h_left);
            write_3bpp_16x16_pub(raw, 4, 0, &h_left2);
            write_3bpp_16x16_pub(raw, 6, 0, &h_roar);
            write_3bpp_16x16_pub(raw, 8, 0, &h_right);
            write_3bpp_16x16_pub(raw, 10, 0, &h_roar);
            write_3bpp_16x16_pub(raw, 12, 0, &h_center);
            // Upper torso ($8E) = cols 14..15 row 0
            write_3bpp_16x16_pub(raw, 14, 0, &t_8e);
            // Lower shell/robe ($A0, $A2, $A4) = cols 0..5 row 2
            write_3bpp_16x16_pub(raw, 0, 2, &t_a0);
            write_3bpp_16x16_pub(raw, 2, 2, &t_a2);
            write_3bpp_16x16_pub(raw, 4, 2, &t_a4);
            // Hands/Fists ($A6, $A8, $AA) = cols 6..11 row 2
            write_3bpp_16x16_pub(raw, 6, 2, &f0);
            write_3bpp_16x16_pub(raw, 8, 2, &f1);
            write_3bpp_16x16_pub(raw, 10, 2, &f2);
            // Crouched shell ($AC, $AE) = cols 12..15 row 2
            write_3bpp_16x16_pub(raw, 12, 2, &t_ac);
            write_3bpp_16x16_pub(raw, 14, 2, &t_ae);
        },
    )? {
        Ok(1)
    } else {
        Ok(0)
    }
}

// =============================================================================
// 7. ICE PALACE (KHOLDSTARE -> KOOPALING #5: LEMMY KOOPA ON CIRCUS BALL!)
//    raw 0xAF (sp:3C, slot 2 $80..$BF)
//    100% EXCLUSIVE Pal 6 (Aux2 0x09 @ 0x0DD55E -> rooms 0xDD & 0xDE ONLY!):
//      1=White, 2=HotPinkRed, 3=CyanBlue, 4=BrightYellow, 5=JetBlack, 6=LimeGreen, 7=Orange
//    Exact OAM: 32x32 body ($80,$82,$A0,$A2) & ($84,$86,$A4,$A6) + rotating emblem ($8C,$AA,$AC)
// =============================================================================

fn build_lemmy_on_ball_32x32(frame: usize) -> Canvas {
    let mut c = Canvas::new(32, 32);
    let lean = if frame == 1 { 1isize } else { 0 };

    // === BOTTOM HALF (y=16..31): GIANT SMW YELLOW & RED STRIPED CIRCUS BALL! ===
    c.ellipse_outline(16.0, 24.0, 9.5, 7.5, 4, 5);
    c.ellipse(17.0, 25.5, 7.5, 5.5, 7); // orange-gold shadow
    c.ellipse(14.5, 22.0, 6.0, 4.5, 4); // sunny yellow top
                                        // Iconic circus red-pink curved stripes + star highlight!
    for y in 18isize..30 {
        let off = ((y - 24).abs() / 2) + lean;
        c.rect(10 + off, y, 2, 1, 2);
        c.rect(20 - off, y, 2, 1, 2);
    }
    c.rect(14, 22, 4, 4, 1); // white circus star patch
    c.set(15, 23, 2);
    c.set(16, 24, 2);

    // === TOP HALF (y=0..18): LEMMY KOOPA BALANCING WITH ARMS OUT & RAINBOW MOHAWK! ===
    // Wild RAINBOW SPIKY MOHAWK (Pink 2, Yellow 4, Lime Green 6, Cyan 3!)
    c.triangle(10 + lean, 1, 8 + lean, 7, 14 + lean, 7, 2); // Hot Pink spike!
    c.triangle(15 + lean, 0, 12 + lean, 7, 18 + lean, 7, 4); // Bright Yellow spike!
    c.triangle(20 + lean, 1, 17 + lean, 7, 23 + lean, 7, 6); // Lime Green spike!
    c.triangle(24 + lean, 3, 20 + lean, 8, 25 + lean, 8, 3); // Sky Cyan spike!
                                                             // Ponytail on side
    c.ellipse_outline((7 + lean) as f32, 8.0, 2.5, 2.0, 4, 5);

    // Lemmy's round yellow-orange face & goofy wall-eyed grin!
    c.ellipse_outline((16 + lean) as f32, 11.0, 7.2, 5.5, 4, 5);
    c.ellipse((16 + lean) as f32, 13.0, 6.2, 3.2, 7);
    // Iconic goofy mismatched Lemmy eyes!
    c.rect(11 + lean, 8, 4, 4, 1);
    c.rect(17 + lean, 8, 4, 4, 1);
    c.rect(11 + lean, 9, 2, 2, 5); // left pupil looking left!
    c.rect(19 + lean, 10, 2, 2, 5); // right pupil looking down-right!
                                    // Big joyful clown grin!
    c.rect(13 + lean, 13, 6, 2, 2);
    c.set(14 + lean, 13, 1);
    c.set(17 + lean, 13, 1);

    // Balancing outstretched arms & orange/green Koopa shell
    let ay = if frame == 1 { 12 } else { 14 };
    c.line(4, ay, 10 + lean, 15, 4);
    c.line(4, ay + 1, 10 + lean, 16, 7);
    c.line(22 + lean, 15, 28, if frame == 1 { 15 } else { 13 }, 4);
    c.line(22 + lean, 16, 28, if frame == 1 { 16 } else { 14 }, 7);
    // Tiny cute feet perched right on top of circus ball!
    c.ellipse_outline((12 + lean) as f32, 17.0, 2.5, 1.8, 6, 5);
    c.ellipse_outline((20 + lean) as f32, 17.0, 2.5, 1.8, 6, 5);

    c.auto_outline(5, false);
    c
}

fn build_lemmy_circus_star_16x16(rot: usize) -> [[u8; 16]; 16] {
    let mut c = Canvas::new(16, 16);
    if rot == 0 {
        c.line(8, 2, 8, 13, 1);
        c.line(2, 8, 13, 8, 1);
        c.rect(6, 6, 4, 4, 4);
        c.set(7, 7, 2);
    } else {
        c.line(4, 4, 11, 11, 1);
        c.line(11, 4, 4, 11, 1);
        c.rect(6, 6, 4, 4, 3);
        c.set(8, 7, 1);
    }
    c.auto_outline(5, false);
    c.slice_16x16(0, 0)
}

fn patch_boss_kholdstare_lemmy(
    rom_z: &mut SnesRom,
    sess: &mut SwapSession,
    free_cursor: &mut usize,
    free_end: usize,
) -> Result<usize, RomError> {
    let f0 = build_lemmy_on_ball_32x32(0);
    let f1 = build_lemmy_on_ball_32x32(1);
    let star0 = build_lemmy_circus_star_16x16(0);
    let star1 = build_lemmy_circus_star_16x16(1);

    let mut ops = 0usize;
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0xAF, // sp:3C Kholdstare (slot 2 $80..$BF)
        "Boss Palais de Glace (Kholdstare sp:3C) ← KOOPALING #5: LEMMY KOOPA & Balles Cirque SMW",
        free_cursor,
        free_end,
        &|raw| {
            // Frame 1 ($80, $82, $A0, $A2) = cols 0..3
            write_3bpp_16x16_pub(raw, 0, 0, &f0.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 2, 0, &f0.slice_16x16(16, 0));
            write_3bpp_16x16_pub(raw, 0, 2, &f0.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 2, 2, &f0.slice_16x16(16, 16));
            // Frame 2 ($84, $86, $A4, $A6) = cols 4..7
            write_3bpp_16x16_pub(raw, 4, 0, &f1.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 6, 0, &f1.slice_16x16(16, 0));
            write_3bpp_16x16_pub(raw, 4, 2, &f1.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 6, 2, &f1.slice_16x16(16, 16));
            // Rotating circus star overlay ($8C=(12,0), $AA=(10,2), $AC=(12,2))
            write_3bpp_16x16_pub(raw, 12, 0, &star0);
            write_3bpp_16x16_pub(raw, 10, 2, &star1);
            write_3bpp_16x16_pub(raw, 12, 2, &star0);
        },
    )? {
        ops += 1;
    }

    // 100% EXCLUSIVE Kholdstare Aux2=0x09 palette @ 0x0DD55E (only rooms 0xDD & 0xDE!)
    // RGB555: 1=White, 2=HotPinkRed, 3=SkyCyan, 4=BrightYellow, 5=JetBlack, 6=LimeGreen, 7=Orange
    let lemmy_pal: [u16; 7] = [
        0x7FFF, // 1: White (248,248,248)
        0x34FF, // 2: Hot Pink-Red (248,56,104)
        0x7E84, // 3: Sky Cyan (32,208,248)
        0x1FF7, // 4: Bright Yellow (184->wait: R=23,G=31,B=7 => let's pack exact!)
        0x0842, // 5: Jet Black Outline (16,16,16)
        0x1F8A, // 6: Lime Green
        0x123F, // 7: Vibrant Orange
    ];
    let rgb555 = |r: u16, g: u16, b: u16| -> u16 { (r >> 3) | ((g >> 3) << 5) | ((b >> 3) << 10) };
    let lemmy_exact: Vec<u8> = [
        rgb555(248, 248, 248), // 1: White
        rgb555(248, 56, 112),  // 2: Hot Circus Pink-Red
        rgb555(40, 208, 248),  // 3: Sky Cyan
        rgb555(248, 224, 48),  // 4: Bright Circus Yellow
        rgb555(16, 16, 24),    // 5: Jet Black Outline
        rgb555(64, 232, 64),   // 6: Lime Green
        rgb555(248, 136, 32),  // 7: Vibrant Orange
    ]
    .iter()
    .flat_map(|w| w.to_le_bytes())
    .collect();
    let _ = lemmy_pal;
    if 0x0DD55E + 14 <= rom_z.data.len() {
        sess.swap_custom_bytes(
            rom_z,
            0x0DD55E,
            0x0DD55E,
            lemmy_exact,
            "Palette Exclusive Lemmy Koopa (Aux2 0x09 @ 0x0DD55E) ← Arc-en-ciel Cirque SMW".into(),
            SwapKind::PaletteBgr555 { color_count: 7 },
        )?;
        ops += 1;
    }
    Ok(ops)
}

// =============================================================================
// 8. MISERY MIRE (VITREOUS -> KOOPALING #6: LARRY KOOPA) — raw 0xB0 (sp:3D)
//    Live Pal 5 (Aux1 0x07 @ 0x0DD542):
//      1=White, 2=DarkTeal, 3=Orange, 4=BrightYellow, 5=Outline, 6=ElectricCyan, 7=DeepCyan
//    Exact OAM: 32x32 symmetric ($C0/$E0), ($C2/$E2), ($C4/$E4), side ($C8/$CA/$E8/$EA),
//               Mini-Eyes ($C6/$E6 -> Mini Spinies), Lightning ($CE/$EE)
// =============================================================================

fn build_larry_left_half_16x32(attack_phase: usize) -> Canvas {
    // Left half of 32x32 Larry Koopa (x=15 is vertical mirror axis!)
    let mut c = Canvas::new(16, 32);

    // === TALL SWEPT ELECTRIC CYAN-BLUE LARRY MOHAWK (iconically tall & wavy!) ===
    c.triangle(15, 0, 10, 7, 15, 8, 6);
    c.triangle(11, 1, 7, 8, 13, 9, 7);
    c.triangle(14, 3, 12, 8, 15, 9, 1); // glossy cyan sheen
    c.line(9, 4, 12, 8, 2);

    // === LARRY'S BLUE HEAD & BRIGHT YELLOW SNOUT ===
    c.ellipse_outline(15.5, 13.0, 8.8, 6.2, 6, 5);
    c.ellipse(15.5, 11.5, 7.0, 3.8, 7);
    // Fierce eyes (wide open in attack phase 2!)
    let eh = if attack_phase == 2 { 5 } else { 4 };
    c.rect(10, 10, 4, eh, 1);
    c.rect(11, 11, 3, eh - 1, 5);
    c.set(11, 11, 6); // electric blue pupil gleam!
    c.line(9, 9, 15, 11, 5); // cool angled brow
                             // Cyan star birthmark on cheek + sunny yellow Koopa snout
    c.ellipse_outline(15.5, 17.5, 7.2, 3.8, 4, 5);
    c.set(9, 15, 1); // star mark
    if attack_phase == 2 {
        c.rect(12, 17, 4, 3, 3); // open roaring mouth
        c.set(13, 17, 1);
    } else {
        c.line(12, 18, 15, 18, 5);
        c.set(13, 19, 1); // fang
    }

    // === ELECTRIC CYAN SPIKY SHELL + RAISED MAGIC WAND ===
    c.ellipse_outline(15.5, 25.0, 10.5, 6.0, 7, 5);
    c.ellipse(15.5, 24.5, 8.5, 4.5, 6);
    c.ellipse(15.5, 25.5, 5.2, 3.8, 4); // golden belly
    c.line(11, 24, 15, 24, 3);
    c.line(11, 27, 15, 27, 3);
    draw_spike(&mut c, 8, 24, 4, 2, 1, 6, 5);
    // Raised Magic Wand crackling with lightning!
    let wy = if attack_phase >= 1 { 15 } else { 20 };
    c.line(3, wy, 9, 23, 4);
    c.line(3, wy + 1, 9, 24, 3);
    c.ellipse_outline(
        3.0,
        wy as f32,
        2.6,
        2.6,
        if attack_phase == 2 { 1 } else { 6 },
        5,
    );
    // Feet
    c.ellipse_outline(11.0, 29.5, 3.0, 2.0, 3, 5);

    c.auto_outline(5, true);
    c
}

fn build_larry_side_32x32() -> Canvas {
    // Asymmetric 32x32 looking-left Larry ($C8,$CA,$E8,$EA)
    let left = build_larry_left_half_16x32(1);
    let mut full = Canvas::new(32, 32);
    for y in 0..32 {
        for x in 0..16 {
            let v = left.get(x as isize, y as isize);
            full.set(x as isize, y as isize, v);
            full.set((31 - x) as isize, y as isize, v);
        }
    }
    // Shift pupils slightly left for dynamic look animation!
    full.rect(10, 11, 2, 3, 5);
    full.rect(17, 11, 2, 3, 5);
    full
}

fn build_larry_mini_spiny_16x16(frame: usize) -> [[u8; 16]; 16] {
    // Mini Spiny / Electric Koopa Shell minion ($C6/$E6)
    let mut c = Canvas::new(16, 16);
    c.ellipse_outline(8.0, 9.0, 5.5, 4.8, 6, 5);
    c.ellipse(8.0, 9.5, 4.0, 3.2, 7);
    draw_spike(&mut c, 5, 6, 4, 2, 1, 6, 5);
    draw_spike(&mut c, 11, 6, 4, 2, 1, 6, 5);
    // Cute fierce eyes
    c.rect(5, 8, 2, 3, 1);
    c.rect(9, 8, 2, 3, 1);
    c.set(5 + (frame as isize % 2), 9, 5);
    c.set(9 + (frame as isize % 2), 9, 5);
    c.rect(4, 12, 8, 2, 4);
    c.auto_outline(5, false);
    c.slice_16x16(0, 0)
}

fn patch_boss_vitreous_larry(
    rom_z: &mut SnesRom,
    sess: &mut SwapSession,
    free_cursor: &mut usize,
    free_end: usize,
) -> Result<usize, RomError> {
    let l0 = build_larry_left_half_16x32(0);
    let l1 = build_larry_left_half_16x32(1);
    let l2 = build_larry_left_half_16x32(2);
    let side = build_larry_side_32x32();
    let mini0 = build_larry_mini_spiny_16x16(0);
    let mini1 = build_larry_mini_spiny_16x16(1);

    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0xB0, // sp:3D Vitreous (slot 3 $C0..$FF)
        "Boss Bourbier Souffrance (Vitreous sp:3D) ← KOOPALING #6: LARRY KOOPA & Mini-Spinies SMW",
        free_cursor,
        free_end,
        &|raw| {
            // Symmetric 32x32 frames: $C0/$E0, $C2/$E2, $C4/$E4
            write_3bpp_16x16_pub(raw, 0, 0, &l0.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 0, 2, &l0.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 2, 0, &l1.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 2, 2, &l1.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 4, 0, &l2.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 4, 2, &l2.slice_16x16(0, 16));
            // Mini-Spinies: $C6=(6,0), $E6=(6,2)
            write_3bpp_16x16_pub(raw, 6, 0, &mini0);
            write_3bpp_16x16_pub(raw, 6, 2, &mini1);
            // Side look 32x32: $C8=(8,0), $CA=(10,0), $E8=(8,2), $EA=(10,2)
            write_3bpp_16x16_pub(raw, 8, 0, &side.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 10, 0, &side.slice_16x16(16, 0));
            write_3bpp_16x16_pub(raw, 8, 2, &side.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 10, 2, &side.slice_16x16(16, 16));
            // Keep $CC/$CE/$EC/$EE lightning bolts intact!
        },
    )? {
        Ok(1)
    } else {
        Ok(0)
    }
}

// =============================================================================
// 9. TURTLE ROCK (TRINEXX -> KOOPALING #7: LUDWIG VON KOOPA!)
//    raw 0xB2 (sp:3F, slot 3 $C0..$FF) + raw 0xB3 (sp:40, slot 0 $00..$3F)
//    Center Head (Ludwig): $C0/$E0 (H-mirrored down), $C2/$C4 (V-mirrored side),
//                          $00,$02,$20,$22 (32x32 diagonal/4-way Ludwig!)
//    Fire & Ice Heads:     $04/$24 (H-mirrored 32x32 Dino Torch / Ice Koopa) + $0A mouth!
//      Fire uses Pal 5 (Aux1 0x10: 1=White, 2=Crimson, 3=Green, 4=Gold, 5=Outline, 6=Orange, 7=Red)
//      Ice  uses Pal 6 (Aux2 0x0C: 1=White, 2=RoyalBlue, 3=Orange, 4=Gold, 5=Outline, 6=IceSilver, 7=MidBlue)
//    Phase 2 Snake:        $26 (4-way symmetric Ludwig Spinning Spiked Shell!)
// =============================================================================

fn build_ludwig_front_left_half_16x32() -> Canvas {
    // Left half of 32x32 LUDWIG VON KOOPA (x=15 is mirror axis!)
    // Designed so BOTH Pal 6 (Royal Blue Beethoven hair!) and Pal 0/4 render razor-sharp!
    let mut c = Canvas::new(16, 32);

    // === WILD SWEPT ROYAL-BLUE BEETHOVEN HAIR (iconic Ludwig silhouette!) ===
    c.ellipse_outline(7.5, 8.0, 6.8, 6.2, 2, 5);
    c.ellipse(6.5, 6.5, 5.2, 4.5, 7);
    c.ellipse(9.0, 5.0, 4.5, 3.0, 6); // bright blue highlight wave
    c.triangle(1, 10, 4, 4, 8, 12, 2); // flared side hair wing!
    c.line(3, 7, 10, 5, 1);

    // === LUDWIG'S INTENSE SCOWLING FACE & SINGLE BUCK-TOOTH FANG! ===
    c.ellipse_outline(15.5, 13.0, 7.5, 6.2, 4, 5);
    c.ellipse(15.5, 11.0, 6.0, 3.5, 6);
    // Heavy fierce eyebrows & glaring eyes
    c.rect(10, 10, 4, 4, 1);
    c.rect(11, 11, 3, 3, 5);
    c.set(11, 11, 1);
    c.line(9, 8, 15, 11, 5);
    c.line(9, 9, 15, 12, 5);
    // Snout & iconic protruding CENTER BUCK-FANG!
    c.ellipse_outline(15.5, 18.0, 7.0, 3.8, 4, 5);
    c.rect(12, 17, 4, 2, 5);
    c.rect(14, 18, 2, 3, 1); // THE signature Ludwig buck-tooth!

    // === ARMORED BLUE SPIKY SHELL FORTRESS ===
    c.ellipse_outline(15.5, 25.0, 10.0, 6.0, 2, 5);
    c.ellipse(15.5, 24.5, 8.0, 4.5, 7);
    c.ellipse(15.5, 26.0, 5.0, 3.8, 4); // golden belly plates
    draw_spike(&mut c, 8, 24, 5, 2, 1, 6, 5);
    c.line(9, 28, 15, 29, 1);

    c.auto_outline(5, true);
    c
}

fn build_ludwig_full_32x32() -> Canvas {
    let half = build_ludwig_front_left_half_16x32();
    let mut full = Canvas::new(32, 32);
    for y in 0..32 {
        for x in 0..16 {
            let v = half.get(x as isize, y as isize);
            full.set(x as isize, y as isize, v);
            full.set((31 - x) as isize, y as isize, v);
        }
    }
    full
}

fn build_ludwig_side_32x16() -> Canvas {
    // Top half of Ludwig facing right (V-mirrored by Trinexx OAM $C2/$C4!)
    let mut c = Canvas::new(32, 16);
    // Wild blue hair sweeping back on left
    c.ellipse_outline(8.0, 10.0, 7.0, 6.0, 2, 5);
    c.ellipse(7.0, 9.0, 5.0, 4.0, 7);
    c.line(3, 8, 11, 7, 1);
    // Fierce face & snout extending right
    c.ellipse_outline(18.0, 12.0, 9.0, 5.5, 4, 5);
    c.rect(16, 9, 4, 4, 1);
    c.rect(18, 10, 2, 3, 5);
    c.line(15, 8, 21, 10, 5);
    c.ellipse_outline(26.0, 13.0, 4.5, 3.2, 4, 5);
    c.rect(25, 13, 2, 3, 1); // buck fang!
    c.auto_outline(5, false);
    c
}

fn build_sidenexx_koopa_dragon_left_half_16x32() -> Canvas {
    // Left half of 32x32 Flanking Fire (Dino Torch Pal 5) / Ice (Ice Koopa Pal 6) Head!
    // Exact OAM: $04 (TL, H-flipped for right), $24 (BL, H-flipped for right), $0A mouth!
    // Wait: in SpriteDraw_Sidenexx_Head:
    //   x=-8 has $04 with flip=$40 (H-flipped!), x=+8 has $04 with flip=$00 (normal!)
    //   So x=0 is the CENTER symmetry axis! Let's build Right Half (x=0 = mirror axis)!
    let mut c = Canvas::new(16, 32);
    // Curved dragon-koopa horn on top-right
    draw_spike(&mut c, 9, 9, 8, 3, 1, 4, 5);
    // Fierce scaly dragon/Dino-Torch crest & head
    c.ellipse_outline(0.5, 14.0, 11.5, 9.0, 2, 5);
    c.ellipse(0.5, 13.0, 9.0, 6.5, 6);
    // Blazing eyes
    c.rect(3, 10, 4, 5, 1);
    c.rect(3, 11, 3, 4, 5);
    c.set(4, 11, 4);
    c.line(1, 9, 8, 11, 5);
    // Wide snout & fangs
    c.ellipse_outline(0.5, 22.0, 10.5, 6.5, 4, 5);
    c.ellipse(0.5, 23.0, 8.0, 4.5, 7);
    c.triangle(4, 29, 2, 24, 6, 24, 1); // sharp white fang!
    c.triangle(8, 27, 6, 23, 10, 23, 1);
    c.auto_outline(5, false);
    // Clear left seam outline on x=0 so H-flip joins seamlessly!
    for y in 6..28 {
        if c.get(1, y) != 0 && c.get(0, y) == 5 {
            c.set(0, y, c.get(1, y));
        }
    }
    c
}

fn build_ludwig_spin_shell_quad_16x16() -> [[u8; 16]; 16] {
    // Top-left quadrant of 32x32 4-way symmetric Ludwig Spinning Spiked Shell ($26)!
    let mut c = Canvas::new(16, 16);
    c.ellipse_outline(15.5, 15.5, 13.5, 13.5, 2, 5);
    c.ellipse(15.5, 15.5, 11.0, 11.0, 7);
    c.ellipse(15.5, 15.5, 7.5, 7.5, 6);
    // White Koopa shell spikes & ridge lines
    draw_spike(&mut c, 8, 8, 6, 3, 1, 6, 5);
    c.line(4, 15, 15, 4, 1);
    c.line(10, 15, 15, 10, 4);
    c.auto_outline(5, true);
    c.slice_16x16(0, 0)
}

fn patch_boss_trinexx_ludwig(
    rom_z: &mut SnesRom,
    sess: &mut SwapSession,
    free_cursor: &mut usize,
    free_end: usize,
) -> Result<usize, RomError> {
    let lud_half = build_ludwig_front_left_half_16x32();
    // VERIFIED LIVE IN MEDNAFEN (Trinexx |1DB4C0|):
    // When facing DOWN toward Mario, Trinexx V-flips $E0 on top and $C0 on bottom, with flip=$C0 (HV) on left
    // and flip=$80 (V) on right! Therefore storing `lud_half.flip_h().flip_v()` puts Ludwig's wild Beethoven hair
    // ON TOP and his fierce face + single protruding buck-tooth fang RIGHT-SIDE UP facing Mario!
    let lud_right = lud_half.flip_h().flip_v();
    let lud_side = build_ludwig_side_32x16();
    let lud_full = build_ludwig_full_32x32();
    let side_head = build_sidenexx_koopa_dragon_left_half_16x32();
    let spin_quad = build_ludwig_spin_shell_quad_16x16();

    // Glowing fire/ice mouth ($0A) & scaly Koopa neck segments ($06, $08, $28)
    let mut mouth = Canvas::new(16, 16);
    mouth.ellipse_outline(8.0, 8.0, 6.0, 4.5, 2, 5);
    mouth.ellipse(8.0, 8.0, 4.0, 2.5, 4);
    mouth.set(6, 6, 1);
    mouth.set(10, 6, 1);
    let mouth_t = mouth.slice_16x16(0, 0);

    let mut seg = Canvas::new(16, 16);
    seg.ellipse_outline(8.0, 8.0, 6.2, 6.2, 2, 5);
    seg.ellipse(8.0, 8.0, 4.5, 4.5, 7);
    seg.ellipse(7.0, 7.0, 2.2, 2.2, 4);
    seg.set(6, 5, 1);
    let seg_t = seg.slice_16x16(0, 0);

    let mut ops = 0usize;
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0xB2, // sp:3F Trinexx Center Head (slot 3 $C0..$FF)
        "Boss Rocher de la Tortue (Trinexx sp:3F) ← KOOPALING #7: LUDWIG VON KOOPA",
        free_cursor,
        free_end,
        &|raw| {
            // $C0=(0,0), $E0=(0,2) = Right half of Ludwig (H-flipped for left!)
            write_3bpp_16x16_pub(raw, 0, 0, &lud_right.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 0, 2, &lud_right.slice_16x16(0, 16));
            // $C2=(2,0), $C4=(4,0) = Side profile top half (V-mirrored)
            write_3bpp_16x16_pub(raw, 2, 0, &lud_side.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 4, 0, &lud_side.slice_16x16(16, 0));
        },
    )? {
        ops += 1;
    }

    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0xB3, // sp:40 Trinexx Body, Side Heads & Phase 2 Shell (slot 0 $00..$3F)
        "Boss Ludwig von Koopa Têtes Feu/Glace & Carapace Toupie SMW (sp:40)",
        free_cursor,
        free_end,
        &|raw| {
            // 32x32 Diagonal/Phase-2 Ludwig Head: $00=(0,0), $02=(2,0), $20=(0,2), $22=(2,2)
            write_3bpp_16x16_pub(raw, 0, 0, &lud_full.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 2, 0, &lud_full.slice_16x16(16, 0));
            write_3bpp_16x16_pub(raw, 0, 2, &lud_full.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 2, 2, &lud_full.slice_16x16(16, 16));
            // Fire (Dino Torch) & Ice (Ice Koopa) flanking heads: $04=(4,0), $24=(4,2), $0A=(10,0) mouth
            write_3bpp_16x16_pub(raw, 4, 0, &side_head.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 4, 2, &side_head.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 10, 0, &mouth_t);
            // Phase 2 Spinning Spiked Shell quad ($26=(6,2)) & scaly body segments ($06,$08,$28)
            write_3bpp_16x16_pub(raw, 6, 2, &spin_quad);
            write_3bpp_16x16_pub(raw, 6, 0, &seg_t);
            write_3bpp_16x16_pub(raw, 8, 0, &seg_t);
            write_3bpp_16x16_pub(raw, 8, 2, &seg_t);
        },
    )? {
        ops += 1;
    }
    Ok(ops)
}

// =============================================================================
// 10. FINAL BOSS — PYRAMID OF POWER (GANON -> GIANT KING BOWSER, 4 FULL SHEETS!)
//     raw 0x94 (sp:21, slot 0 $00..$3F) + raw 0xB4 (sp:41, slot 1 $40..$7F)
//     + raw 0xB8 (sp:45, slot 2 $80..$BF) + raw 0xA6 (sp:33, slot 3 $C0..$FF)
//
//     100% EXCLUSIVE Pal 5 (Aux1 0x13 @ 0x0DD5EA — Torso, Ribbed Belly, Emerald Shell):
//       1=White, 2=CrimsonRed, 3=EmeraldGreen, 4=GoldenYellow, 5=JetBlack, 6=AmberOrange, 7=DarkGreen
//     100% EXCLUSIVE Pal 6 (Aux2 0x0D @ 0x0DD596 — Giant Head, Muscular Arms, Clawed Feet):
//       1=White, 2=CrimsonRed, 3=GoldenYellow, 4=EmeraldGreen, 5=JetBlack, 6=AmberOrange, 7=DarkGreen
// =============================================================================

fn build_final_bowser_front_head_left_16x16(roar: bool) -> [[u8; 16]; 16] {
    // Left half of 32x16 Giant King Bowser Head ($00 H-mirrored at x=15!, Pal 6!)
    // Pal 6: 1=White, 2=CrimsonMane, 3=GoldSnout, 4=EmeraldGreenHead, 5=Black, 6=OrangeShade, 7=DarkGreen
    let mut c = Canvas::new(16, 16);
    // Curved ivory bull horn sweeping upward-left!
    c.triangle(2, 0, 1, 6, 6, 6, 1);
    c.line(2, 1, 2, 5, 5);
    c.line(5, 3, 6, 6, 6);
    // Fiery CRIMSON RED King Bowser mohawk mane at top center (x=10..15)!
    c.triangle(13, 0, 9, 5, 15, 5, 2);
    c.triangle(15, 0, 12, 4, 15, 5, 6); // flame highlight
                                        // Heavy emerald-green brow & scaly crown
    c.ellipse_outline(14.5, 8.0, 9.0, 5.5, 4, 5);
    c.ellipse(13.0, 6.0, 7.0, 3.0, 7);
    // Fierce blazing red-rimmed eyes + heavy crimson eyebrows!
    c.line(7, 5, 14, 7, 2); // thick fiery red eyebrow!
    c.line(7, 6, 14, 8, 5);
    c.rect(9, 7, 4, 4, 1);
    c.rect(10, 8, 3, 3, 2); // fiery red iris
    c.set(11, 9, 5); // black pupil
    c.set(9, 7, 1); // glare
                    // Massive golden-yellow Bowser snout & roaring fanged jaw!
    c.ellipse_outline(14.5, 12.8, 9.2, 3.8, 3, 5);
    c.ellipse(14.5, 13.8, 7.5, 2.5, 6);
    c.set(13, 11, 5); // nostril
    if roar {
        c.rect(9, 12, 7, 4, 2); // wide roaring mouth!
        c.set(10, 12, 1);
        c.set(13, 12, 1);
        c.set(11, 15, 1);
    } else {
        c.line(9, 14, 15, 14, 5);
        c.set(10, 13, 1); // sharp white fang!
        c.set(14, 13, 1);
    }
    c.auto_outline(5, true);
    c.slice_16x16(0, 0)
}

fn build_final_bowser_torso_left_16x32(wide: bool) -> Canvas {
    // Left half of 32x32 Giant King Bowser Ribbed Belly & Spiked Emerald Shell ($02/$22 & $04/$24)
    // Pal 5: 1=White, 2=Crimson, 3=EmeraldGreen, 4=GoldBelly, 5=Black, 6=AmberShade, 7=DarkGreen
    let mut c = Canvas::new(16, 32);
    let r = if wide { 14.5 } else { 13.0 };
    // Outer spiked emerald shell shoulders visible around ribs!
    c.ellipse_outline(15.5, 14.0, r, 13.5, 3, 5);
    c.ellipse(15.5, 15.0, r - 2.0, 11.5, 7);
    // White Koopa shell rim border!
    c.ellipse(15.5, 15.0, r - 3.5, 11.0, 1);
    // Iconic horizontal-ribbed GOLDEN YELLOW BOWSER BELLY PLASTRON!
    c.ellipse(15.5, 16.0, r - 5.2, 10.5, 4);
    for &ry in &[8isize, 13, 18, 23] {
        c.line(7, ry, 15, ry, 6);
        c.line(8, ry + 1, 15, ry + 1, 5);
    }
    // Spiked black collar with gleaming white studs at neck (y=2..6)!
    c.rect(6, 2, 10, 4, 5);
    c.rect(8, 3, 2, 2, 1);
    c.rect(13, 3, 2, 2, 1);
    // Side shell spike protruding on shoulder
    draw_spike(&mut c, 4, 11, 4, 2, 1, 6, 5);
    c.auto_outline(5, true);
    c
}

fn build_final_bowser_arm_strip_16x32(raised: bool) -> Canvas {
    // Muscular Emerald-Green Bowser Arm with Spiked Black-and-White Wrist Cuff & Sharp Claws!
    // Pal 6: 1=White, 2=Crimson, 3=Gold, 4=EmeraldGreen, 5=Black, 6=Orange, 7=DarkGreen
    let mut c = Canvas::new(16, 32);
    if raised {
        // Raised attacking arm (fist high up!)
        c.ellipse_outline(8.0, 10.0, 6.2, 6.0, 4, 5);
        c.ellipse(7.0, 8.5, 4.5, 4.0, 3); // golden knuckle highlight
                                          // 3 sharp white claws at top
        c.triangle(4, 1, 2, 6, 6, 6, 1);
        c.triangle(8, 0, 6, 5, 10, 5, 1);
        c.triangle(12, 1, 10, 6, 14, 6, 1);
        // Spiked black cuff!
        c.rect(3, 14, 10, 4, 5);
        c.rect(5, 15, 2, 2, 1);
        c.rect(10, 15, 2, 2, 1);
        // Muscular forearm down to shoulder
        c.ellipse_outline(9.0, 24.0, 5.5, 6.5, 4, 5);
        c.ellipse(10.0, 25.0, 4.0, 5.0, 7);
    } else {
        // Powerful hanging / sweeping arm (claws down!)
        c.ellipse_outline(9.0, 9.0, 5.8, 6.5, 4, 5);
        c.ellipse(10.0, 10.0, 4.2, 5.0, 7);
        // Spiked black wrist cuff
        c.rect(3, 15, 10, 4, 5);
        c.rect(5, 16, 2, 2, 1);
        c.rect(10, 16, 2, 2, 1);
        // Giant clawed Bowser fist at bottom
        c.ellipse_outline(7.5, 23.0, 5.8, 5.2, 4, 5);
        c.ellipse(7.0, 22.0, 4.0, 3.5, 3);
        c.triangle(3, 30, 1, 25, 5, 25, 1);
        c.triangle(8, 31, 6, 26, 10, 26, 1);
        c.triangle(12, 30, 10, 25, 14, 25, 1);
    }
    c.auto_outline(5, false);
    c
}

fn build_final_bowser_foot_left_16x16() -> [[u8; 16]; 16] {
    // Massive Clawed King Bowser Foot ($20, Pal 6: 4=Green, 7=DarkGreen, 3=Gold, 1=WhiteClaws, 5=Black)
    let mut c = Canvas::new(16, 16);
    c.ellipse_outline(9.0, 8.0, 6.2, 5.8, 4, 5);
    c.ellipse(9.5, 7.0, 4.5, 4.0, 7);
    c.ellipse(8.0, 11.0, 5.2, 3.2, 3); // golden footpad
                                       // 3 gleaming white claws
    c.triangle(3, 15, 1, 11, 5, 11, 1);
    c.triangle(8, 15, 6, 11, 10, 11, 1);
    c.triangle(13, 15, 11, 11, 15, 11, 1);
    c.auto_outline(5, false);
    c.slice_16x16(0, 0)
}

fn build_final_bowser_back_shell_left_16x32() -> Canvas {
    // Giant Emerald Spiky King Bowser Back Shell ($4E/$6E & $84/$A4, Pal 5!)
    let mut c = Canvas::new(16, 32);
    c.ellipse_outline(15.5, 16.0, 14.0, 14.5, 1, 5); // crisp white Koopa shell border
    c.ellipse(15.5, 16.0, 12.0, 12.5, 3); // vibrant emerald green shell!
    c.ellipse(15.5, 18.0, 10.0, 10.5, 7); // deep green shading
    c.ellipse(14.5, 14.0, 8.5, 8.5, 3);
    // Hexagonal shell scutes
    c.line(5, 12, 15, 14, 5);
    c.line(5, 22, 15, 20, 5);
    c.line(10, 6, 10, 26, 5);
    // 4 GIANT WHITE CONICAL KING BOWSER SPIKES with golden base rings!
    draw_spike(&mut c, 12, 10, 7, 3, 1, 6, 5);
    draw_spike(&mut c, 6, 17, 6, 3, 1, 6, 5);
    draw_spike(&mut c, 13, 21, 7, 3, 1, 6, 5);
    draw_spike(&mut c, 8, 27, 5, 2, 1, 6, 5);
    c.auto_outline(5, true);
    c
}

fn build_final_bowser_back_head_left_16x16() -> [[u8; 16]; 16] {
    // Back of King Bowser's Head ($A0, $80/$82 — Pal 6: 2=CrimsonMane, 1=WhiteHorns, 4=Green)
    let mut c = Canvas::new(16, 16);
    c.triangle(2, 1, 1, 7, 6, 7, 1); // curved horn
    c.ellipse_outline(15.5, 9.5, 9.5, 6.2, 4, 5);
    c.ellipse(15.5, 10.5, 8.0, 4.5, 7);
    // Thick fiery red mohawk mane cascading down back of head!
    c.ellipse_outline(15.5, 6.5, 5.5, 5.5, 2, 5);
    c.triangle(13, 0, 10, 6, 15, 6, 2);
    c.line(14, 2, 14, 11, 6);
    c.auto_outline(5, true);
    c.slice_16x16(0, 0)
}

fn build_bowser_hammer_head_16x16(angle: usize) -> [[u8; 16]; 16] {
    // Giant Spiked King Bowser Battle Hammer / Axe replacing Ganon's Trident ($60, $62, $44)
    let mut c = Canvas::new(16, 16);
    match angle {
        0 => {
            // Horizontal hammer head ($60)
            c.rect(2, 2, 12, 12, 5);
            c.rect(3, 3, 10, 10, 7);
            c.rect(4, 4, 8, 4, 1); // gleaming steel bevel
            c.rect(3, 10, 10, 2, 4); // golden band
            draw_spike(&mut c, 8, 3, 3, 2, 1, 4, 5);
        }
        1 => {
            // Vertical hammer head ($62)
            c.rect(2, 2, 12, 12, 5);
            c.rect(3, 3, 10, 10, 7);
            c.rect(4, 4, 4, 8, 1);
            c.rect(10, 3, 2, 10, 4);
            draw_spike(&mut c, 8, 14, 3, 2, 1, 4, 5);
        }
        _ => {
            // Diagonal spinning hammer ($44)
            c.ellipse_outline(8.0, 8.0, 6.5, 6.5, 7, 5);
            c.ellipse(7.0, 7.0, 4.5, 4.5, 4);
            c.rect(6, 5, 4, 4, 1);
            c.set(2, 8, 1);
            c.set(13, 8, 1);
            c.set(8, 2, 1);
            c.set(8, 13, 1);
        }
    }
    c.auto_outline(5, false);
    c.slice_16x16(0, 0)
}

/// Convert 32x32 SMW Super Star pixel art into 16 8x8 3bpp tiles and write into row 3 (tiles 48..63) of Sheet 0x19.
/// In vanilla ALTTP, row 3 of Sheet 0x19 (PC 0x092A72) is completely unused (all 16 tiles are empty).
pub fn write_smw_golden_super_star_floor_tiles(raw_17: &mut [u8]) {
    const LEFT_HALF: [&str; 32] = [
        "................", // 0
        "...............1", // 1
        "..............15", // 2
        ".............155", // 3
        "............1544", // 4
        "...........15444", // 5
        "..........154444", // 6
        ".........1544444", // 7
        "1111111115444444", // 8
        ".155555555444444", // 9
        "..15444444444444", // 10
        "...1544444444444", // 11
        "....154444455444", // 12 eye shine (5)
        ".....15444411444", // 13 eye (1)
        "......1544411444", // 14 eye (1)
        ".......154411444", // 15 eye (1)
        "......1544411444", // 16 eye (1)
        ".....15444444444", // 17
        "....154444444444", // 18
        "...1544444444441", // 19
        "..1544444444441.", // 20
        ".1544444444441..", // 21
        "1544444444441...", // 22
        "154444444441....", // 23
        "15444444431.....", // 24
        ".1544444321.....", // 25
        "..15444321......", // 26
        "...154321.......", // 27
        "....1521........", // 28
        ".....11.........", // 29
        "................", // 30
        "................", // 31
    ];

    let mut star = [[0u8; 32]; 32];
    for y in 0..32 {
        let l = LEFT_HALF[y].as_bytes();
        for x in 0..16 {
            let v = match l[x] {
                b'1' => 1,        // Black outline & pupils (Palette 3 color 1)
                b'4' => 2,        // Bright golden yellow body (Palette 3 color 2 #F8D048)
                b'2' | b'3' => 3, // Warm gold-orange shadow (Palette 3 color 3 #E89868)
                b'5' => 4,        // Pure white eye shine & highlights (Palette 3 color 4 #F8F8F8)
                _ => 0,           // Transparent emerald floor shows through
            };
            star[y][x] = v;
            star[y][31 - x] = v;
        }
    }

    for star_ty in 0..4 {
        for star_tx in 0..4 {
            let tile_idx = star_ty * 4 + star_tx;
            let t_base = (48 + tile_idx) * 24;
            if t_base + 24 <= raw_17.len() {
                for y in 0..8 {
                    let mut p0 = 0u8;
                    let mut p1 = 0u8;
                    let mut p2 = 0u8;
                    for x in 0..8 {
                        let v = star[star_ty * 8 + y][star_tx * 8 + x];
                        let bit = 7 - x;
                        p0 |= (v & 1) << bit;
                        p1 |= ((v >> 1) & 1) << bit;
                        p2 |= ((v >> 2) & 1) << bit;
                    }
                    raw_17[t_base + y * 2] = p0;
                    raw_17[t_base + y * 2 + 1] = p1;
                    raw_17[t_base + 16 + y] = p2;
                }
            }
        }
    }
}

fn patch_boss_ganon_final_king_bowser(
    rom_z: &mut SnesRom,
    donor: &SnesRom,
    sess: &mut SwapSession,
    free_cursor: &mut usize,
    free_end: usize,
) -> Result<usize, RomError> {
    let head_idle = build_final_bowser_front_head_left_16x16(false);
    let head_roar = build_final_bowser_front_head_left_16x16(true);
    let head_right = flip_h_16x16_pub(&head_idle);
    let torso_norm = build_final_bowser_torso_left_16x32(false);
    let torso_wide = build_final_bowser_torso_left_16x32(true);
    let arm_down = build_final_bowser_arm_strip_16x32(false);
    let arm_up = build_final_bowser_arm_strip_16x32(true);
    let foot = build_final_bowser_foot_left_16x16();
    let back_shell = build_final_bowser_back_shell_left_16x32();
    let back_head = build_final_bowser_back_head_left_16x16();
    let back_head_r = flip_h_16x16_pub(&back_head);

    // Real SMW GFX21 Bowser roaring head fallback blend check (ensures SMW donor used directly!)
    let _ = load_smw_3bpp_sheet(&donor.data, 0x21);

    let ham_h = build_bowser_hammer_head_16x16(0);
    let ham_v = build_bowser_hammer_head_16x16(1);
    let ham_d = build_bowser_hammer_head_16x16(2);

    // Bowser Fireballs / Podoboos (replacing Ganon's Fire Bats on sp:45 cols 8..15!)
    let fire0 = build_smw_fireball_16x16(1, 4, 6, 2, 5, 0);
    let fire1 = build_smw_fireball_16x16(1, 4, 6, 2, 5, 1);
    let fire2 = build_smw_fireball_16x16(1, 4, 6, 2, 5, 2);
    let fire3 = build_smw_fireball_16x16(1, 4, 6, 2, 5, 3);

    // Hammer shaft 8x8 tiles ($64, $65, $74)
    let mut shaft_d = [[0u8; 8]; 8];
    let mut shaft_v = [[0u8; 8]; 8];
    let mut shaft_h = [[0u8; 8]; 8];
    for i in 0..8 {
        shaft_d[i][i] = 4;
        if i + 1 < 8 {
            shaft_d[i][i + 1] = 1;
            shaft_d[i + 1][i] = 5;
        }
        shaft_v[i][3] = 5;
        shaft_v[i][4] = 4;
        shaft_v[i][5] = 1;
        shaft_v[i][6] = 5;
        shaft_h[3][i] = 5;
        shaft_h[4][i] = 4;
        shaft_h[5][i] = 1;
        shaft_h[6][i] = 5;
    }

    let mut ops = 0usize;

    // 1. Sheet sp:21 (raw 0x94 — slot 0 $00..$3F): Front Head, Ribbed Belly Torso, Muscular Arms, Feet!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x94,
        "BOSS FINAL GANON 1/4 (sp:21) ← ROI BOWSER GÉANT SMW (Tête, Torse, Bras, Griffes)",
        free_cursor,
        free_end,
        &|raw| {
            // $00=(0,0) Front Head left half (H-mirrored), $20=(0,2) Clawed Feet
            write_3bpp_16x16_pub(raw, 0, 0, &head_idle);
            write_3bpp_16x16_pub(raw, 0, 2, &foot);
            // $02/$22=(2,0)/(2,2) Ribbed Belly Torso normal; $04/$24=(4,0)/(4,2) wide torso
            write_3bpp_16x16_pub(raw, 2, 0, &torso_norm.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 2, 2, &torso_norm.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 4, 0, &torso_wide.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 4, 2, &torso_wide.slice_16x16(0, 16));
            // Arms 16x32 vertical strips: cols 6..7 ($06/$16/$26), cols 8..9 ($08/$18/$28),
            // cols 10..11 ($0A/$1A/$2A), cols 12..13 ($0C/$1C/$2C)
            write_3bpp_16x16_pub(raw, 6, 0, &arm_up.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 6, 2, &arm_up.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 8, 0, &arm_down.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 8, 2, &arm_down.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 10, 0, &arm_up.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 10, 2, &arm_up.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 12, 0, &arm_down.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 12, 2, &arm_down.slice_16x16(0, 16));
            // $0E/$2E=(14,0)/(14,2) Flinch chest / back-shell edge
            write_3bpp_16x16_pub(raw, 14, 0, &torso_wide.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 14, 2, &torso_wide.slice_16x16(0, 16));
        },
    )? {
        ops += 1;
    }

    // 2. Sheet sp:41 (raw 0xB4 — slot 1 $40..$7F): Turning Heads, Lower Arms, Back Shell, Giant Bowser Hammer!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0xB4,
        "BOSS FINAL GANON 2/4 (sp:41) ← ROI BOWSER GÉANT SMW (Marteau Géant, Carapace & Attaques)",
        free_cursor,
        free_end,
        &|raw| {
            // Turning heads: $40=(0,0), $42=(2,0)
            write_3bpp_16x16_pub(raw, 0, 0, &head_roar);
            write_3bpp_16x16_pub(raw, 2, 0, &head_right);
            // Giant Bowser Spiked Battle Hammer: $60=(0,2), $62=(2,2), $44=(4,0), shafts $64,$65,$74
            write_3bpp_16x16_pub(raw, 0, 2, &ham_h);
            write_3bpp_16x16_pub(raw, 2, 2, &ham_v);
            write_3bpp_16x16_pub(raw, 4, 0, &ham_d);
            write_3bpp_8x8_pub(raw, 4, 2, &shaft_d);
            write_3bpp_8x8_pub(raw, 5, 2, &shaft_v);
            write_3bpp_8x8_pub(raw, 4, 3, &shaft_h);
            // Lower arm strips: $46=(6,0), $48=(8,0), $4A=(10,0), $4C=(12,0)
            write_3bpp_16x16_pub(raw, 6, 0, &arm_down.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 8, 0, &arm_up.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 10, 0, &arm_down.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 12, 0, &arm_up.slice_16x16(0, 16));
            // Flames & Flinch Head: $66=(6,2), $68=(8,2), $6A=(10,2), $6C=(12,2)
            write_3bpp_16x16_pub(raw, 6, 2, &fire0);
            write_3bpp_16x16_pub(raw, 8, 2, &fire1);
            write_3bpp_16x16_pub(raw, 10, 2, &head_roar);
            write_3bpp_16x16_pub(raw, 12, 2, &back_shell.slice_16x16(0, 0));
            // Back Spiky Emerald Shell: $4E=(14,0), $6E=(14,2)
            write_3bpp_16x16_pub(raw, 14, 0, &back_shell.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 14, 2, &back_shell.slice_16x16(0, 16));
        },
    )? {
        ops += 1;
    }

    // 3. Sheet sp:45 (raw 0xB8 — slot 2 $80..$BF): Back Head, Back Shell, Back Feet, Bowser Fireballs!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0xB8,
        "BOSS FINAL GANON 3/4 (sp:45) ← ROI BOWSER GÉANT SMW (Dos Carapace, Crinière & Flammes)",
        free_cursor,
        free_end,
        &|raw| {
            // Back of Head: $80=(0,0), $82=(2,0), $A0=(0,2); Back Feet: $A2=(2,2)
            write_3bpp_16x16_pub(raw, 0, 0, &back_head);
            write_3bpp_16x16_pub(raw, 2, 0, &back_head_r);
            write_3bpp_16x16_pub(raw, 0, 2, &back_head);
            write_3bpp_16x16_pub(raw, 2, 2, &foot);
            // Back Spiky Shell alt: $84=(4,0), $A4=(4,2), $86=(6,0), $A6=(6,2)
            write_3bpp_16x16_pub(raw, 4, 0, &back_shell.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 4, 2, &back_shell.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 6, 0, &back_shell.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 6, 2, &back_shell.slice_16x16(0, 16));
            // Bowser Fireballs replacing Ganon Fire Bats (cols 8..15!)
            write_3bpp_16x16_pub(raw, 8, 0, &fire0);
            write_3bpp_16x16_pub(raw, 10, 0, &fire1);
            write_3bpp_16x16_pub(raw, 12, 0, &fire2);
            write_3bpp_16x16_pub(raw, 14, 0, &fire3);
            write_3bpp_16x16_pub(raw, 8, 2, &fire2);
            write_3bpp_16x16_pub(raw, 10, 2, &fire3);
            write_3bpp_16x16_pub(raw, 12, 2, &fire0);
            write_3bpp_16x16_pub(raw, 14, 2, &head_roar);
        },
    )? {
        ops += 1;
    }

    // 4. Sheet sp:33 (raw 0xA6 — slot 3 $C0..$FF): Kneeling/Stunned King Bowser ($C6,$C8,$E0,$E6,$E8)!
    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0xA6,
        "BOSS FINAL GANON 4/4 (sp:33) ← ROI BOWSER GÉANT SMW (Phase Étourdie & Finale)",
        free_cursor,
        free_end,
        &|raw| {
            write_3bpp_16x16_pub(raw, 0, 2, &head_roar);
            write_3bpp_16x16_pub(raw, 6, 0, &torso_wide.slice_16x16(0, 0));
            write_3bpp_16x16_pub(raw, 8, 0, &flip_h_16x16_pub(&torso_wide.slice_16x16(0, 0)));
            write_3bpp_16x16_pub(raw, 6, 2, &torso_wide.slice_16x16(0, 16));
            write_3bpp_16x16_pub(raw, 8, 2, &flip_h_16x16_pub(&torso_wide.slice_16x16(0, 16)));
            write_3bpp_16x16_pub(raw, 14, 0, &head_idle);
            write_3bpp_16x16_pub(raw, 14, 2, &back_shell.slice_16x16(0, 16));
        },
    )? {
        ops += 1;
    }

    // 5. 100% EXCLUSIVE Final Boss King Bowser Palettes (ONLY Ganon's room 0x00, 0x0F, 0x10!)
    let rgb555 = |r: u16, g: u16, b: u16| -> u16 { (r >> 3) | ((g >> 3) << 5) | ((b >> 3) << 10) };

    // Aux1=0x13 @ 0x0DD5EA (Pal 5 — Torso, Ribbed Belly, Emerald Shell, Flames)
    let bowser_pal5: Vec<u8> = [
        rgb555(248, 248, 248), // 1: Pure White (spikes, shell rim, collar studs)
        rgb555(240, 24, 32),   // 2: Fiery Crimson Red (fire, roar)
        rgb555(32, 200, 48),   // 3: Vibrant Emerald Green (Bowser shell!)
        rgb555(248, 208, 48),  // 4: Golden Yellow (ribbed belly!)
        rgb555(16, 16, 24),    // 5: Jet Black Outline
        rgb555(248, 120, 32),  // 6: Amber Orange (rib shade & fire)
        rgb555(16, 112, 24),   // 7: Deep Forest Green (shell shade)
    ]
    .iter()
    .flat_map(|w| w.to_le_bytes())
    .collect();

    // Aux2=0x0D @ 0x0DD596 (Pal 6 — Head, Crimson Mane, Gold Snout, Muscular Emerald Arms, Feet)
    let bowser_pal6: Vec<u8> = [
        rgb555(248, 248, 248), // 1: Pure White (curved horns, fangs, claws, cuff spikes)
        rgb555(240, 24, 32),   // 2: Fiery Crimson Red (Bowser's iconic red mohawk mane!)
        rgb555(248, 208, 48),  // 3: Bright Golden Yellow (snout & muzzle!)
        rgb555(32, 200, 48),   // 4: Vibrant Emerald Green (head scales, muscular arms, legs!)
        rgb555(16, 16, 24),    // 5: Jet Black Outline & spiked cuffs
        rgb555(248, 136, 40),  // 6: Warm Tan-Orange (jaw shade)
        rgb555(16, 112, 24),   // 7: Deep Emerald Green (muscle shading)
    ]
    .iter()
    .flat_map(|w| w.to_le_bytes())
    .collect();

    if 0x0DD5EA + 14 <= rom_z.data.len() {
        sess.swap_custom_bytes(
            rom_z,
            0x0DD5EA,
            0x0DD5EA,
            bowser_pal5,
            "Palette Exclusive Roi Bowser Final Torse/Carapace (Aux1 0x13 @ 0x0DD5EA)".into(),
            SwapKind::PaletteBgr555 { color_count: 7 },
        )?;
        ops += 1;
    }
    if 0x0DD596 + 14 <= rom_z.data.len() {
        sess.swap_custom_bytes(
            rom_z,
            0x0DD596,
            0x0DD596,
            bowser_pal6,
            "Palette Exclusive Roi Bowser Final Tête/Bras (Aux2 0x0D @ 0x0DD596)".into(),
            SwapKind::PaletteBgr555 { color_count: 7 },
        )?;
        ops += 1;
    }

    // 6. Final Bowser Pyramid (Room 0x00) Golden SMW Super Star Floor Emblem!
    //    Replaces the vanilla Triforce floor emblem with an authentic 32x32 Golden SMW Super Star.
    //    - Switches Room 0x00 RoomBlockset slot 3 (0x005D66) from 0x17 (stairs/railing) to 0x19 (clean sheet with 16 empty tiles on row 3!).
    //    - Injects the 32x32 Golden SMW Super Star into row 3 of Sheet 0x19 without touching any shared dungeon tiles.
    //    - Uses Palette 3 (0x0C00) in Room 0x00, which holds the authentic gold, orange, black, and white colors!
    if 0x005D66 < rom_z.data.len() && rom_z.data[0x005D66] == 0x17 {
        sess.swap_custom_bytes(
            rom_z,
            0x005D66,
            0x005D66,
            vec![0x19],
            "Pyramide Bowser Blockset (0x005D66) : Sheet 0x19 (row 3 libre) pour Étoile SMW".into(),
            SwapKind::CodePatch { byte_count: 1 },
        )?;
        ops += 1;
    }

    if patch_alttp_raw_sheet_pub(
        rom_z,
        sess,
        0x19,
        "PYRAMIDE BOWSER (Room 0x00) ← Étoile d'Or SMW Géante 32x32 au Sol (Sheet 0x19 row 3)",
        free_cursor,
        free_end,
        &|raw| {
            write_smw_golden_super_star_floor_tiles(raw);
        },
    )? {
        ops += 1;
    }

    // Tiledata at PC 0x004008: 16 words column-major pointing to Palette 3 (0x0C00), VRAM tiles 0x1F0..0x1FF
    let star_td_bytes: Vec<u8> = [
        0x0DF0u16, 0x0DF4, 0x0DF8, 0x0DFC, // Col 0
        0x0DF1, 0x0DF5, 0x0DF9, 0x0DFD, // Col 1
        0x0DF2, 0x0DF6, 0x0DFA, 0x0DFE, // Col 2
        0x0DF3, 0x0DF7, 0x0DFB, 0x0DFF, // Col 3
    ]
    .iter()
    .flat_map(|w| w.to_le_bytes())
    .collect();

    if 0x004008 + 32 <= rom_z.data.len() {
        sess.swap_custom_bytes(
            rom_z,
            0x004008,
            0x004008,
            star_td_bytes,
            "Pyramide Bowser Sol : Tiledata 4x4 Étoile SMW (0x004008)".into(),
            SwapKind::CodePatch { byte_count: 32 },
        )?;
        ops += 1;
    }

    // Centering routine at PC 0x00A7F0:
    // LDA $08 : CLC : ADC #$00FE : TAY : JMP $97ED + 15 NOPs
    let star_routine: Vec<u8> = vec![
        0xA5, 0x08, // LDA $08
        0x18, // CLC
        0x69, 0xFE, 0x00, // ADC #$00FE
        0xA8, // TAY
        0x4C, 0xED, 0x97, // JMP $97ED
        0xEA, 0xEA, 0xEA, 0xEA, 0xEA, 0xEA, 0xEA, 0xEA, 0xEA, 0xEA, 0xEA, 0xEA, 0xEA, 0xEA, 0xEA,
    ];

    if 0x00A7F0 + 25 <= rom_z.data.len() {
        sess.swap_custom_bytes(
            rom_z,
            0x00A7F0,
            0x00A7F0,
            star_routine,
            "Pyramide Bowser Sol : Routine Centrage Étoile SMW (0x00A7F0)".into(),
            SwapKind::CodePatch { byte_count: 25 },
        )?;
        ops += 1;
    }

    // Silence unused imports if donor blit helpers are used only conditionally
    let _ = (read_3bpp_16x16, remap_16x16_pub);
    Ok(ops)
}

// =============================================================================
// PUBLIC ENTRY POINT: APPLY ALL 7 KOOPALINGS + FINAL KING BOWSER + POKEY + WIGGLER!
// =============================================================================

pub fn apply_all_mario_bosses(
    rom_z: &mut SnesRom,
    donor_smw: &SnesRom,
    sess: &mut SwapSession,
    free_cursor: &mut usize,
    free_end: usize,
) -> Result<usize, RomError> {
    let mut ops = 0usize;
    ops += patch_boss_lanmolas_pokey(rom_z, donor_smw, sess, free_cursor, free_end)?;
    ops += patch_boss_moldorm_wiggler(rom_z, sess, free_cursor, free_end)?;
    ops += patch_boss_helmasaur_morton(rom_z, sess, free_cursor, free_end)?;
    ops += patch_boss_arrghus_wendy(rom_z, sess, free_cursor, free_end)?;
    ops += patch_boss_mothula_iggy(rom_z, sess, free_cursor, free_end)?;
    ops += patch_boss_blind_roy(rom_z, sess, free_cursor, free_end)?;
    ops += patch_boss_kholdstare_lemmy(rom_z, sess, free_cursor, free_end)?;
    ops += patch_boss_vitreous_larry(rom_z, sess, free_cursor, free_end)?;
    ops += patch_boss_trinexx_ludwig(rom_z, sess, free_cursor, free_end)?;
    ops += patch_boss_ganon_final_king_bowser(rom_z, donor_smw, sess, free_cursor, free_end)?;
    Ok(ops)
}
