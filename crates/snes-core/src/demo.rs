use crate::graphics::{encode_tile, rgb_to_bgr555, TileFormat};
use crate::rom::SnesRom;

/// Generate two complete, valid 256 KB SNES LoROMs ("SUPER RUST HERO" & "CYBER DEMON X")
/// populated with 4bpp sprites, 2bpp HUD tiles, BGR555 palettes, and SPC700 BRR sound samples.
pub fn generate_demo_rom_pair() -> (SnesRom, SnesRom) {
    let rom_a_bytes = build_synthetic_rom(
        "SUPER RUST HERO      ",
        0x01, // North America
        true,
    );
    let rom_b_bytes = build_synthetic_rom(
        "CYBER DEMON X        ",
        0x00, // Japan
        false,
    );

    let mut rom_a =
        SnesRom::from_bytes("rom_a_super_rust_hero.sfc", &rom_a_bytes).expect("Valid demo ROM A");
    let mut rom_b =
        SnesRom::from_bytes("rom_b_cyber_demon_x.sfc", &rom_b_bytes).expect("Valid demo ROM B");

    rom_a.fix_checksum();
    rom_b.fix_checksum();

    (rom_a, rom_b)
}

fn build_synthetic_rom(title_21: &str, region: u8, is_hero: bool) -> Vec<u8> {
    let mut data = vec![0x00u8; 256 * 1024]; // 256 KB LoROM

    // 1. Write LoROM internal header at 0x007FC0
    let base = 0x007FC0;
    let title_bytes = title_21.as_bytes();
    for i in 0..21 {
        data[base + i] = *title_bytes.get(i).unwrap_or(&b' ');
    }
    data[base + 0x15] = if is_hero { 0x20 } else { 0x30 }; // LoROM / FastROM
    data[base + 0x16] = 0x02; // ROM + RAM + Battery
    data[base + 0x17] = 0x08; // 256 KB (1 << 8)
    data[base + 0x18] = 0x03; // 8 KB SRAM
    data[base + 0x19] = region;
    data[base + 0x1A] = 0x33;
    data[base + 0x1B] = 0x01;
    // NMI & Reset vectors
    data[base + 0x2A] = 0x80;
    data[base + 0x2B] = 0x80;
    data[base + 0x3C] = 0x00;
    data[base + 0x3D] = 0x80;

    // 2. Write 4bpp Sprite Sheet at 0x010000 (896 tiles = 28,672 bytes = 0x7000 bytes, matching full Zelda Link sprite bank 0x080000..0x087000)
    let gfx_base = 0x010000;
    for tile_idx in 0..896 {
        let pattern = make_demo_4bpp_tile(tile_idx, is_hero);
        let encoded = encode_tile(&pattern, TileFormat::Bpp4);
        let dst = gfx_base + tile_idx * 32;
        data[dst..dst + 32].copy_from_slice(&encoded);
    }

    // 3. Write 2bpp HUD/Font Sheet at 0x017000 (128 tiles = 2048 bytes)
    let hud_base = 0x017000;
    for tile_idx in 0..128 {
        let pattern = make_demo_2bpp_tile(tile_idx, is_hero);
        let encoded = encode_tile(&pattern, TileFormat::Bpp2);
        let dst = hud_base + tile_idx * 16;
        data[dst..dst + 16].copy_from_slice(&encoded);
    }

    // 4. Write BGR555 Palettes at 0x018000..0x019A00 (3,328 colors = 6,656 bytes)
    let pal1_rgb: [[u8; 3]; 16] = if is_hero {
        [
            [16, 20, 32],
            [40, 44, 68],
            [235, 65, 55],
            [165, 30, 35],
            [250, 205, 165],
            [210, 145, 100],
            [65, 145, 245],
            [35, 80, 175],
            [75, 210, 85],
            [35, 135, 50],
            [255, 225, 65],
            [215, 145, 30],
            [150, 95, 55],
            [95, 55, 30],
            [210, 220, 235],
            [255, 255, 255],
        ]
    } else {
        [
            [18, 6, 36],     // 0: Deep Cyber Violet
            [45, 12, 85],    // 1: Dark Neon Indigo
            [255, 20, 145],  // 2: Hot Neon Magenta
            [185, 10, 105],  // 3: Crimson Magenta
            [0, 255, 245],   // 4: Electric Cyan
            [0, 165, 210],   // 5: Deep Cyan
            [180, 70, 255],  // 6: Ultraviolet Purple
            [105, 30, 190],  // 7: Dark Violet
            [50, 255, 130],  // 8: Acid Cyber Green
            [15, 170, 80],   // 9: Matrix Green
            [255, 240, 50],  // 10: Laser Gold Yellow
            [255, 120, 20],  // 11: Plasma Orange
            [255, 60, 200],  // 12: Neon Pink
            [80, 200, 255],  // 13: Ice Blue
            [215, 240, 255], // 14: Chrome Silver
            [255, 255, 255], // 15: Pure White
        ]
    };

    for c_idx in 0..3328usize {
        let [r, g, b] = pal1_rgb[(c_idx + (c_idx / 16)) % 16];
        let word = rgb_to_bgr555(r, g, b);
        let pos = 0x018000 + c_idx * 2;
        data[pos..pos + 2].copy_from_slice(&word.to_le_bytes());
    }

    // 5. Write 3 SPC700 BRR Audio Samples at 0x020000, 0x020200, 0x020400
    let sfx1 = synthesize_brr_sample(36, false, if is_hero { 1 } else { 4 });
    let sfx2 = synthesize_brr_sample(40, false, if is_hero { 2 } else { 5 });
    let sfx3 = synthesize_brr_sample(48, true, if is_hero { 3 } else { 6 });

    data[0x020000..0x020000 + sfx1.len()].copy_from_slice(&sfx1);
    data[0x020200..0x020200 + sfx2.len()].copy_from_slice(&sfx2);
    data[0x020400..0x020400 + sfx3.len()].copy_from_slice(&sfx3);

    // 6. Write 65c816 Reset Routine at PC 0x000000 ($00:8000) with real Hardware DMA transfers
    //    Channel 0 -> VRAM ($2118) from $02:8000 (PC 0x010000, size 0x1000)
    //    Channel 1 -> CGRAM ($2122) from $03:8000 (PC 0x018000, size 0x0020)
    let dma_code: &[u8] = &[
        0x78, // SEI
        0x18, // CLC
        0xFB, // XCE
        0xE2, 0x20, // SEP #$20 (8-bit A)
        0xC2, 0x10, // REP #$10 (16-bit X/Y)
        0xA9, 0x18, // LDA #$18 (VRAM $2118)
        0x8D, 0x01, 0x43, // STA $4301
        0xA2, 0x00, 0x80, // LDX #$8000
        0x8E, 0x02, 0x43, // STX $4302
        0xA9, 0x02, // LDA #$02 (Bank $02 -> PC 0x010000)
        0x8D, 0x04, 0x43, // STA $4304
        0xA2, 0x00, 0x10, // LDX #$1000 (4096 bytes)
        0x8E, 0x05, 0x43, // STX $4305
        0xA9, 0x22, // LDA #$22 (CGRAM $2122)
        0x8D, 0x11, 0x43, // STA $4311
        0xA2, 0x00, 0x80, // LDX #$8000
        0x8E, 0x12, 0x43, // STX $4312
        0xA9, 0x03, // LDA #$03 (Bank $03 -> PC 0x018000)
        0x8D, 0x14, 0x43, // STA $4314
        0xA2, 0x20, 0x00, // LDX #$0020 (32 bytes)
        0x8E, 0x15, 0x43, // STX $4315
        0xA9, 0x03, // LDA #$03 (Enable DMA Ch 0 & 1)
        0x8D, 0x0B, 0x42, // STA $420B
        0x6B, // RTL
    ];
    data[0..dma_code.len()].copy_from_slice(dma_code);

    // 7. Write a Nintendo LC_LZ2 compressed graphics stream at 0x030000
    let compressed_lz2 = crate::disasm_dma::compress_nintendo_lz2(&data[0x010000..0x010400]);
    if 0x030000 + compressed_lz2.len() <= data.len() {
        data[0x030000..0x030000 + compressed_lz2.len()].copy_from_slice(&compressed_lz2);
    }

    data
}

fn make_demo_4bpp_tile(tile_idx: usize, is_hero: bool) -> [u8; 64] {
    let mut px = [0u8; 64];
    let kind = tile_idx % 8;

    for y in 0..8usize {
        for x in 0..8usize {
            let dx = (x as i32 - 3).abs();
            let dy = (y as i32 - 3).abs();
            let val = if is_hero {
                match kind {
                    0 => {
                        // Hero Helmet / Face
                        if x == 0 || x == 7 || y == 0 || y == 7 {
                            1
                        } else if y <= 2 {
                            6 // Blue helmet
                        } else if (x == 2 || x == 5) && y == 4 {
                            15 // Eyes
                        } else {
                            4 // Skin
                        }
                    }
                    1 => {
                        // Sword Diagonal
                        if x + y == 7 {
                            15
                        } else if (x + y == 6) || (x + y == 8) {
                            14
                        } else if x <= 2 && y >= 5 {
                            10 // Gold hilt
                        } else {
                            0
                        }
                    }
                    2 => {
                        // Shield Emblem
                        if x == 0 || x == 7 || y == 0 || y == 7 {
                            14
                        } else if x == 3 || x == 4 || y == 3 || y == 4 {
                            10 // Gold cross
                        } else {
                            2 // Red shield
                        }
                    }
                    3 => {
                        // Heart Icon
                        let heart_mask: [u8; 8] = [
                            0b01100110, 0b11111111, 0b11111111, 0b11111111, 0b01111110, 0b00111100,
                            0b00011000, 0b00000000,
                        ];
                        if (heart_mask[y] >> (7 - x)) & 1 != 0 {
                            if x <= 2 && y <= 2 {
                                15
                            } else {
                                2
                            }
                        } else {
                            0
                        }
                    }
                    4 => {
                        // Emerald Gem
                        if dx + dy <= 2 {
                            8
                        } else if dx + dy == 3 {
                            9
                        } else {
                            0
                        }
                    }
                    5 => {
                        // Gold Coin
                        if dx <= 2 && dy <= 3 {
                            if x == 3 && (2..=5).contains(&y) {
                                15
                            } else {
                                10
                            }
                        } else {
                            0
                        }
                    }
                    6 => {
                        // Treasure Chest
                        if (1..=6).contains(&x) && (2..=6).contains(&y) {
                            if y == 4 {
                                10
                            } else {
                                12
                            }
                        } else {
                            0
                        }
                    }
                    _ => {
                        // Armor / Brick Tile
                        if y % 4 == 0 || (x + (y / 4) * 4) % 8 == 0 {
                            1
                        } else {
                            ((tile_idx + x + y) % 14 + 2) as u8
                        }
                    }
                }
            } else {
                // Villain / Cyber Demon patterns
                match kind {
                    0 => {
                        // Cyber Skull / Visor
                        let visor_mask: [u8; 8] = [
                            0b00111100, 0b01111110, 0b11000011, 0b11111111, 0b01111110, 0b01011010,
                            0b00100100, 0b00000000,
                        ];
                        if y == 2 && (2..=5).contains(&x) {
                            4 // Cyan glowing visor
                        } else if (visor_mask[y] >> (7 - x)) & 1 != 0 {
                            2 // Neon magenta armor
                        } else {
                            0
                        }
                    }
                    1 => {
                        // Plasma Blaster
                        if (2..=5).contains(&y) && (1..=6).contains(&x) {
                            if x >= 5 {
                                4
                            } else {
                                6
                            }
                        } else {
                            0
                        }
                    }
                    2 => {
                        // Cyber Drone X
                        if x == y || x + y == 7 {
                            4
                        } else if dx <= 1 && dy <= 1 {
                            2
                        } else {
                            0
                        }
                    }
                    3 => {
                        // Energy Core
                        if dx <= 2 && dy <= 2 {
                            if dx == 0 && dy == 0 {
                                15
                            } else {
                                8
                            }
                        } else if x == 0 || x == 7 || y == 0 || y == 7 {
                            7
                        } else {
                            0
                        }
                    }
                    4 => {
                        // Flame / Lava Orb
                        if dx + dy <= 3 {
                            if dx + dy <= 1 {
                                10
                            } else {
                                11
                            }
                        } else {
                            0
                        }
                    }
                    5 => {
                        // Mecha Claw
                        if x % 2 == 1 && y >= 2 {
                            14
                        } else if y == 1 {
                            2
                        } else {
                            0
                        }
                    }
                    6 => {
                        // Cyber Hex Grid
                        if (x + y) % 2 == 0 {
                            6
                        } else {
                            1
                        }
                    }
                    _ => {
                        // Circuit Board pattern
                        if x == 3 || y == 3 {
                            4
                        } else if (x == 1 || x == 6) && (y == 1 || y == 6) {
                            10
                        } else {
                            1
                        }
                    }
                }
            };
            px[y * 8 + x] = val & 0x0F;
        }
    }
    px
}

fn make_demo_2bpp_tile(tile_idx: usize, is_hero: bool) -> [u8; 64] {
    let mut px = [0u8; 64];
    for y in 0..8usize {
        for x in 0..8usize {
            let border = x == 0 || x == 7 || y == 0 || y == 7;
            let diag = if is_hero { x == y } else { x + y == 7 };
            px[y * 8 + x] = if border {
                1
            } else if diag || ((tile_idx + x).is_multiple_of(5) && y % 2 == 1) {
                3
            } else if (x + y + tile_idx).is_multiple_of(3) {
                2
            } else {
                0
            };
        }
    }
    px
}

fn synthesize_brr_sample(num_blocks: usize, loops: bool, preset: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(num_blocks * 9);
    for b in 0..num_blocks {
        let is_last = b + 1 == num_blocks;
        // First block uses filter 0; subsequent blocks can use filter 0 or 1 with range 9..11
        let filter = if b == 0 { 0 } else { (preset % 2) & 0x03 };
        let range = if b < num_blocks / 2 { 10u8 } else { 8u8 };
        let mut hdr = (range << 4) | (filter << 2);
        if is_last {
            hdr |= 0x01; // End bit
            if loops {
                hdr |= 0x02; // Loop bit
            }
        }
        out.push(hdr);

        for i in 0..8usize {
            let step0 = b * 16 + i * 2;
            let step1 = step0 + 1;
            let n0 = synth_nibble(step0, preset) & 0x0F;
            let n1 = synth_nibble(step1, preset) & 0x0F;
            out.push((n0 << 4) | n1);
        }
    }
    out
}

fn synth_nibble(step: usize, preset: u8) -> u8 {
    let phase = (step as f32) * (0.22 + (preset as f32) * 0.09);
    let val: f32 = match preset {
        1 => phase.sin() * 6.5,                                     // Hero Jump rise
        2 => (phase * 1.7).sin() * 5.5 + (phase * 0.5).cos() * 2.0, // Crystal Sword chime
        3 => {
            if (step / 6).is_multiple_of(2) {
                5.0
            } else {
                -5.0
            }
        } // Synth Bass square wave
        4 => (phase * 2.4).sin() * 6.8,                             // Plasma Laser
        5 => (((step * 73 + 19) % 15) as f32 - 7.0) * 0.85,         // Explosion burst
        _ => (phase * 1.3).sin() * 6.0,                             // Siren alarm
    };
    let clamped = (val.round() as i8).clamp(-7, 7);
    (clamped as u8) & 0x0F
}

/// Apply an immediately obvious, full-scale swap between two SNES ROMs (`rom_a` Target and `rom_b` Donor),
/// such as booting Super Mario World with Link from Zelda ALTTP replacing Mario!
pub fn apply_obvious_demo_swap(
    rom_a: &mut SnesRom,
    rom_b: &SnesRom,
    session: &mut crate::swapper::SwapSession,
    brr_samples_a: &[crate::audio::BrrSampleInfo],
    brr_samples_b: &[crate::audio::BrrSampleInfo],
) -> Result<usize, crate::rom::RomError> {
    let mut ops_applied = 0usize;

    let game_a = rom_a.identify_game();
    let game_b = rom_b.identify_game();

    // Case 0: Target is Super Mario World
    if game_a == crate::rom::RecognizedGame::SuperMarioWorld || rom_a.data.len() == 512 * 1024 {
        if game_b == crate::rom::RecognizedGame::ZeldaAlttp || rom_b.data.len() == 1024 * 1024 {
            ops_applied += apply_link_into_smw_swap(rom_a, rom_b, session)?;
        }
    } else if (game_a == crate::rom::RecognizedGame::ZeldaAlttp || rom_a.data.len() >= 0x0E0000)
        && (game_b == crate::rom::RecognizedGame::SuperMarioWorld || rom_b.data.len() == 512 * 1024)
    {
        // Case 1: ROM A is 1MB Zelda ALTTP and ROM B is 512KB Super Mario World
        // 1) Decompress Super Mario World's real LC_LZ2 3bpp sheets (GFX00 @ 0x0459F9, GFX01 @ 0x046231, GFX33 @ 0x043FC0, GFX32 @ 0x040000)
        //    and unpack 896 tiles (28,672 bytes) into 4bpp for Link's sprite bank (0x080000..0x087000)
        let gfx00 = crate::disasm_dma::decompress_nintendo_lz2_with_endian(
            &rom_b.data,
            0x0459F9,
            16384,
            true,
        )
        .map(|(d, _)| d)
        .unwrap_or_default();
        let gfx01 = crate::disasm_dma::decompress_nintendo_lz2_with_endian(
            &rom_b.data,
            0x046231,
            16384,
            true,
        )
        .map(|(d, _)| d)
        .unwrap_or_default();
        let gfx33 = crate::disasm_dma::decompress_nintendo_lz2_with_endian(
            &rom_b.data,
            0x043FC0,
            16384,
            true,
        )
        .map(|(d, _)| d)
        .unwrap_or_default();
        let gfx32 = crate::disasm_dma::decompress_nintendo_lz2_with_endian(
            &rom_b.data,
            0x040000,
            32768,
            true,
        )
        .map(|(d, _)| d)
        .unwrap_or_default();
        let gfx28 = crate::disasm_dma::decompress_nintendo_lz2_with_endian(
            &rom_b.data,
            0x05C06C,
            8192,
            true,
        )
        .map(|(d, _)| d)
        .unwrap_or_default();

        let mut combined_3bpp = Vec::with_capacity(896 * 24);
        combined_3bpp.extend_from_slice(&gfx00);
        combined_3bpp.extend_from_slice(&gfx01);
        combined_3bpp.extend_from_slice(&gfx33);
        combined_3bpp.extend_from_slice(&gfx32);
        combined_3bpp.resize(896 * 24, 0x00);

        let unpacked_4bpp = crate::graphics::unpack_3bpp_to_4bpp(&combined_3bpp);
        session.swap_custom_bytes(
            rom_a,
            0x080000,
            0x0459F9,
            unpacked_4bpp,
            "Swap Décompressé LC_LZ2 -> 4bpp : 896 tiles Super Mario World (GFX00 Champignon/Bloc ?, GFX01 Koopa/Goomba, GFX33 Yoshi, GFX32 Mario -> Link 0x080000)".to_string(),
            crate::swapper::SwapKind::GraphicsTiles {
                format: TileFormat::Bpp4,
                tile_count: 896,
                smart_remapped: true,
            },
        )?;
        ops_applied += 1;

        // 2) Recompress SMW GFX00 (Mushroom, Fire Flower, ? Block, P-Switch, Star, 1UP) into Zelda's Title Screen Castle/Mountains BG Sheet 0x5C (0x0A6BAE, budget 928B)
        if gfx00.len() >= 1152 {
            let mut title_bg_3bpp = Vec::with_capacity(1536);
            title_bg_3bpp.extend_from_slice(&gfx00[384..1152]);
            title_bg_3bpp.extend_from_slice(&gfx00[384..1152]);
            let recomp_bg =
                crate::disasm_dma::compress_nintendo_lz2_with_endian(&title_bg_3bpp, false);
            if recomp_bg.len() <= 928 {
                session.swap_custom_bytes(
                    rom_a,
                    0x0A6BAE,
                    0x0459F9,
                    recomp_bg,
                    "Swap Écran Titre LC_LZ2 : GFX00 Super Mario World (Bloc ?, Champignon, Fleur, Étoile, 1UP -> Château Écran Titre 0x0A6BAE)".to_string(),
                    crate::swapper::SwapKind::GraphicsTiles {
                        format: TileFormat::Bpp4,
                        tile_count: 64,
                        smart_remapped: true,
                    },
                )?;
                ops_applied += 1;
            }
        }

        // 3) Recompress SMW GFX01 (Koopa, Bullet Bill, Goomba, Piranha Plant, Key) into Zelda's File Select / Sword Sheet 0x5E (0x0C09A1, budget 963B)
        if gfx01.len() >= 768 {
            let mut select_spr_3bpp = Vec::with_capacity(1536);
            select_spr_3bpp.extend_from_slice(&gfx01[..768]);
            select_spr_3bpp.extend_from_slice(&gfx01[..768]);
            let recomp_spr =
                crate::disasm_dma::compress_nintendo_lz2_with_endian(&select_spr_3bpp, false);
            if recomp_spr.len() <= 963 {
                session.swap_custom_bytes(
                    rom_a,
                    0x0C09A1,
                    0x046231,
                    recomp_spr,
                    "Swap Sélection Joueur LC_LZ2 : GFX01 Super Mario World (Koopa, Goomba, Bullet Bill -> Fée & Épée 0x0C09A1)".to_string(),
                    crate::swapper::SwapKind::GraphicsTiles {
                        format: TileFormat::Bpp4,
                        tile_count: 64,
                        smart_remapped: true,
                    },
                )?;
                ops_applied += 1;
            }
        }

        // 4) Swap SMW GFX28 (2bpp HUD/Title Font @ 0x05C06C) into Zelda's 2bpp Font/HUD (0x070000)
        if gfx28.len() >= 2048 {
            session.swap_custom_bytes(
                rom_a,
                0x070000,
                0x05C06C,
                gfx28[..2048].to_vec(),
                "Swap Police HUD 2bpp Décompressée : GFX28 Super Mario World (0x05C06C -> 0x070000)"
                    .to_string(),
                crate::swapper::SwapKind::GraphicsTiles {
                    format: TileFormat::Bpp2,
                    tile_count: 128,
                    smart_remapped: false,
                },
            )?;
            ops_applied += 1;
        }

        // 5) Swap 3D Spinning Triforce Intro Palette (0x064425, 8 BGR555 colors) <- SMW Mario & Luigi Palette (0x0032CC)
        session.swap_palette(rom_a, rom_b, 0x064425, 0x0032CC, 8)?;
        ops_applied += 1;

        // 6) Swap Zelda's Master BGR555 Palette Bank (0x0DD218..0x0DEBB4 = 3,278 colors = 6,556 bytes)
        //    using Super Mario World's real Master Palette Bank (0x003100..0x003800 = 896 colors = 1,792 bytes)
        let smw_pal_slice = &rom_b.data[0x003100..0x003800];
        let mut master_pal_bytes = vec![0u8; 3278 * 2];
        for i in 0..master_pal_bytes.len() {
            master_pal_bytes[i] = smw_pal_slice[(i + 0x180) % smw_pal_slice.len()];
        }
        // Align Link's Tunic Palette (0x0DD308, offset 0xF0 inside master_pal_bytes) to Mario's exact palette (0x0032C8)
        let link_rel = 0x0DD308 - 0x0DD218;
        if link_rel + 90 <= master_pal_bytes.len() {
            master_pal_bytes[link_rel..link_rel + 90]
                .copy_from_slice(&rom_b.data[0x0032B4..0x0032B4 + 90]);
        }
        session.swap_custom_bytes(
            rom_a,
            0x0DD218,
            0x003100,
            master_pal_bytes,
            "Swap Banque Maîtresse 3 278 Couleurs BGR555 : Palettes Super Mario World (Mario 0x0032C8, Luigi 0x0032DC, Yoshi & Niveaux 0x003100 -> Zelda 0x0DD218)".to_string(),
            crate::swapper::SwapKind::PaletteBgr555 { color_count: 3278 },
        )?;
        ops_applied += 1;

        // Also update the 4 hardcoded Title Screen / Triforce Intro sky backdrop immediate pairs using SMW color from 0x0032E6 (0x7CAE Royal Luigi/Star Indigo)
        let smw_sky_lo = rom_b.data[0x0032E6];
        let smw_sky_hi = rom_b.data[0x0032E7];
        for &(lo_off, hi_off) in &[
            (0x066E3Eusize, 0x066E44usize),
            (0x066F97usize, 0x066F9Dusize),
            (0x06701Cusize, 0x067022usize),
            (0x067092usize, 0x06709Cusize),
        ] {
            if hi_off < rom_a.data.len()
                && rom_a.data[lo_off - 1] == 0xA9
                && rom_a.data[hi_off - 1] == 0xA9
            {
                rom_a.data[lo_off] = smw_sky_lo;
                rom_a.data[hi_off] = smw_sky_hi;
            }
        }
        rom_a.fix_checksum();
    } else if rom_a.data.len() >= 0x0E0000 && rom_b.data.len() >= 0x019A00 {
        session.swap_graphics_tiles(rom_a, rom_b, 0x080000, 0x010000, 896, TileFormat::Bpp4)?;
        session.swap_graphics_tiles(rom_a, rom_b, 0x070000, 0x017000, 128, TileFormat::Bpp2)?;
        session.swap_palette(rom_a, rom_b, 0x064425, 0x018004, 8)?;
        session.swap_palette(rom_a, rom_b, 0x0DD218, 0x018000, 3278)?;
        ops_applied += 4;

        // Also swap the 4 hardcoded Title Screen / Triforce Intro backdrop color immediate bytes ($3800 Dark Blue -> $4816 Cyber Crimson-Magenta)
        for &(lo_off, hi_off) in &[
            (0x066E3Eusize, 0x066E44usize),
            (0x066F97usize, 0x066F9Dusize),
            (0x06701Cusize, 0x067022usize),
            (0x067092usize, 0x06709Cusize),
        ] {
            if hi_off < rom_a.data.len()
                && rom_a.data[lo_off - 1] == 0xA9
                && rom_a.data[lo_off] == 0x00
                && rom_a.data[hi_off - 1] == 0xA9
                && rom_a.data[hi_off] == 0x38
            {
                rom_a.data[lo_off] = 0x16;
                rom_a.data[hi_off] = 0x48;
            }
        }
        rom_a.fix_checksum();
    } else if rom_a.data.len() >= 0x019000 && rom_b.data.len() >= 0x019000 {
        session.swap_graphics_tiles(rom_a, rom_b, 0x010000, 0x010000, 128, TileFormat::Bpp4)?;
        session.swap_palette(rom_a, rom_b, 0x018000, 0x018000, 16)?;
        ops_applied += 2;
    }

    // Also swap up to 6 BRR audio samples using time-stretch resampling ONLY when not targeting N-SPC music
    // (Blindly replacing N-SPC music instruments causes severe out-of-tune buzzing and audio glitches)
    if !brr_samples_b.is_empty() && rom_a.data.len() != 512 * 1024 {
        for (idx, sa) in brr_samples_a.iter().take(6).enumerate() {
            let sb = &brr_samples_b[idx % brr_samples_b.len()];
            session.swap_brr_sample(rom_a, rom_b, sa, sb, true)?;
            ops_applied += 1;
        }
    }

    Ok(ops_applied)
}

/// Autonomous full fusion function: injects Link into Super Mario World,
/// applies title screen "SUPER LINK WORLD", and validates the SNES hardware checksum.
pub fn apply_full_fusion(
    rom_target: &mut SnesRom,
    donor_zelda: Option<&SnesRom>,
    session: &mut crate::swapper::SwapSession,
) -> Result<usize, crate::rom::RomError> {
    let mut ops_applied = 0usize;
    if let Some(zelda) = donor_zelda {
        ops_applied += apply_link_into_smw_swap(rom_target, zelda, session)?;
    }
    if let Ok(title_ops) = apply_link_world_title_screen(rom_target, session) {
        ops_applied += title_ops;
    }
    rom_target.fix_checksum();
    Ok(ops_applied)
}

/// Dynamically locate SMW's Mario OAM Pose tables (`top_table`, `bot_table`) across all regions
/// (USA: `0x00600C` / `0x0060CC`, PAL Europe: `0x005FFC` / `0x0060BC`, Japan, etc.)
/// using the invariant Super Mario pose signature at `top_table + 0x46`.
pub(crate) fn find_smw_mario_pose_tables(rom: &[u8]) -> (usize, usize) {
    const SUPER_POSE_SIG: [u8; 14] = [
        0x70, 0x70, 0x70, 0xA0, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0xA0, 0x74, 0x70, 0x80,
    ];
    let end = rom.len().min(0x008000);
    if let Some(pos) = rom[..end]
        .windows(SUPER_POSE_SIG.len())
        .position(|w| w == SUPER_POSE_SIG)
    {
        if pos >= 0x46 {
            let top = pos - 0x46;
            return (top, top + 0xC0);
        }
    }
    (0x00600C, 0x0060CC)
}

/// Dynamically locate SMW's Yoshi (4×6 BGR555 colors) and Player (Mario/Luigi/Fire 4×10 BGR555 colors)
/// palette tables across all regions (USA: `0x003298`/`0x0032C8`, PAL Europe: `0x0032AB`/`0x0032DB`).
fn find_smw_yoshi_and_mario_palettes(rom: &[u8]) -> (usize, usize) {
    // Green Yoshi palette (4th entry, +36 bytes from start of Yoshi table):
    // 0000 (outline), 01E0, 02E0, 03E0 (greens), 00B7 (saddle/boots), 023F (orange)
    const GREEN_YOSHI_SIG: [u8; 12] = [
        0x00, 0x00, 0xE0, 0x01, 0xE0, 0x02, 0xE0, 0x03, 0xB7, 0x00, 0x3F, 0x02,
    ];
    // Mario palette first 4 words: 635F (skin), 581D (dark pink), 000A (dark red), 391F (red hat)
    const MARIO_PAL_SIG: [u8; 8] = [0x5F, 0x63, 0x1D, 0x58, 0x0A, 0x00, 0x1F, 0x39];

    let end = rom.len().min(0x008000);
    if let Some(pos) = rom[..end]
        .windows(GREEN_YOSHI_SIG.len())
        .position(|w| w == GREEN_YOSHI_SIG)
    {
        if pos >= 36 {
            let yoshi_off = pos - 36;
            return (yoshi_off, yoshi_off + 48);
        }
    }
    if let Some(pos) = rom[..end]
        .windows(MARIO_PAL_SIG.len())
        .position(|w| w == MARIO_PAL_SIG)
    {
        if pos >= 48 {
            return (pos - 48, pos);
        }
    }
    // Fallback by region byte ($007FD9: 0x02 = Europe/PAL)
    if rom.get(0x7FD9).copied() == Some(0x02) {
        (0x0032AB, 0x0032DB)
    } else {
        (0x003298, 0x0032C8)
    }
}

/// Inject Link (from Zelda ALTTP `rom_b` @ `0x080000` & `0x0DD308`) into Super Mario World (`rom_a` @ `0x040000` GFX32,
/// `0x04EF1E` GFX10 Overworld, and dynamically located Player CGRAM Palettes + Hardware OAM pose tables).
pub fn apply_link_into_smw_swap(
    rom_a: &mut SnesRom,
    rom_b: &SnesRom,
    session: &mut crate::swapper::SwapSession,
) -> Result<usize, crate::rom::RomError> {
    let mut ops = 0usize;
    let (pose_top_base, pose_bot_base) = find_smw_mario_pose_tables(&rom_a.data);
    let (_yoshi_pal_base, mario_pal_base) = find_smw_yoshi_and_mario_palettes(&rom_a.data);

    // 1. Decompress SMW GFX32 (PC 0x040000..0x043FC0, 23,808 bytes = 744 4bpp tiles)
    let orig_gfx32 =
        crate::disasm_dma::decompress_nintendo_lz2_with_endian(&rom_a.data, 0x040000, 32768, true)
            .map(|(d, _)| d)
            .unwrap_or_else(|| vec![0u8; 23808]);

    let mut gfx32 = orig_gfx32.clone();
    if gfx32.len() < 23808 {
        gfx32.resize(23808, 0);
    }

    // Color map from Zelda ALTTP Link 4bpp indices (0..15) to SMW Mario CGRAM ($80..$8F):
    // 0 = Transparent, 1 = White ($81 = 0x7FFF), 2 = Black Outline ($82 = 0x0000),
    // 6..15 = Loaded every VBlank from `mario_pal_base` (10 BGR555 colors)
    let cmap4: [u8; 16] = [0, 1, 6, 7, 13, 2, 6, 15, 8, 9, 10, 11, 12, 13, 14, 1];

    let small_frames: [[[u8; 16]; 16]; 9] = [
        make_small_link_16x16(&rom_b.data, (0, 0), (3, 2), true, &cmap4), // 0: Stand
        make_small_link_16x16(&rom_b.data, (0, 0), (2, 2), true, &cmap4), // 1: Walk 1
        make_small_link_16x16(&rom_b.data, (0, 0), (4, 2), true, &cmap4), // 2: Walk 2
        make_small_link_16x16(&rom_b.data, (0, 0), (5, 2), true, &cmap4), // 3: Run 1
        make_small_link_16x16(&rom_b.data, (0, 0), (6, 2), true, &cmap4), // 4: Run 2
        make_small_link_16x16(&rom_b.data, (0, 0), (7, 2), true, &cmap4), // 5: Jump
        make_small_link_16x16(&rom_b.data, (0, 0), (0, 3), true, &cmap4), // 6: Fall
        make_small_link_16x16(&rom_b.data, (1, 0), (3, 1), false, &cmap4), // 7: Front
        make_small_link_16x16(&rom_b.data, (2, 0), (6, 1), false, &cmap4), // 8: Back
    ];

    let super_pairs: [([[u8; 16]; 16], [[u8; 16]; 16]); 9] = [
        make_super_link_16x32(&rom_b.data, (0, 0), (3, 2), true, &cmap4), // 0: Stand
        make_super_link_16x32(&rom_b.data, (0, 0), (2, 2), true, &cmap4), // 1: Walk 1
        make_super_link_16x32(&rom_b.data, (0, 0), (4, 2), true, &cmap4), // 2: Walk 2
        make_super_link_16x32(&rom_b.data, (0, 0), (5, 2), true, &cmap4), // 3: Run 1
        make_super_link_16x32(&rom_b.data, (0, 0), (6, 2), true, &cmap4), // 4: Run 2
        make_super_link_16x32(&rom_b.data, (0, 0), (7, 2), true, &cmap4), // 5: Jump
        make_super_link_16x32(&rom_b.data, (0, 0), (0, 3), true, &cmap4), // 6: Fall
        make_super_link_16x32(&rom_b.data, (1, 0), (3, 1), false, &cmap4), // 7: Front
        make_super_link_16x32(&rom_b.data, (2, 0), (6, 1), false, &cmap4), // 8: Back
    ];

    // Redirect Super Mario's shared legs (pose 0x0F victory -> tile 0x7E, pose 0x25 net climbing -> tile 0x7F)
    // to prevent Super Mario from overwriting Small Mario's standing pose (tile 7) and walking pose (tile 16)
    let remaps: [(usize, u8); 4] = [
        (pose_bot_base + 0x55, 0x7E), // Super Mario Victory legs
        (pose_bot_base + 0x92, 0x7E), // Cape Mario Victory legs
        (pose_bot_base + 0x6B, 0x7F), // Super Mario Net Climbing legs
        (pose_bot_base + 0xA8, 0x7F), // Cape Mario Net Climbing legs
    ];
    for &(addr, new_tile) in &remaps {
        if addr < rom_a.data.len() {
            rom_a.data[addr] = new_tile;
        }
    }

    // Blank all Small Mario top tiles in ROM table `pose_top_base` (USA 0x00600C / PAL 0x005FFC).
    // Small Link is 16x16 and completely contained in vb.
    // By setting va = 197 (transparent blank tile) for all Small Mario poses (0x00..0x45),
    // Small Link is 100% clean with ZERO Mario cap or floating head artifacts!
    for y in 0x00..0x46usize {
        rom_a.data[pose_top_base + y] = 197;
    }

    // Small Mario slope sliding & dummy poses in `pose_bot_base` (USA 0x0060CC / PAL 0x0060BC):
    // In vanilla SMW, poses 0x2A..0x2F and 0x40, 0x41 had vb = 197 (blank).
    // Remap them to unused tile 5 so tile 197 stays 100% transparent and Link slides properly!
    for y in 0x2A..0x30usize {
        rom_a.data[pose_bot_base + y] = 5;
    }
    rom_a.data[pose_bot_base + 0x40] = 5;
    rom_a.data[pose_bot_base + 0x41] = 5;

    let blank16 = [[0u8; 16]; 16];
    write_smw_gfx32_16x16(&mut gfx32, 197, &blank16);

    // 1) First populate Small Mario poses (Y = 0x00..0x45 in `pose_bot_base`)
    for y in 0x00..0x46usize {
        let vb = rom_a.data[pose_bot_base + y];
        if vb != 197 {
            let pidx = smw_pose_to_link_idx(y);
            write_smw_gfx32_16x16(&mut gfx32, vb, &small_frames[pidx]);
        }
    }

    // 2) Then populate Super, Fire & Cape Mario poses (Y = 0x46..0xBF in `pose_top_base` / `pose_bot_base`)
    for y in 0x46..0xC0usize {
        let pose = if y < 0x83 { y - 0x46 } else { y - 0x83 };
        let va = rom_a.data[pose_top_base + y];
        let vb = rom_a.data[pose_bot_base + y];
        if pose == 0x3C {
            // Crouching Super Mario: top half is blanked (erasing Mario's cap/head), bottom half is crouching Link
            write_smw_gfx32_16x16(&mut gfx32, va, &blank16);
            write_smw_gfx32_16x16(&mut gfx32, vb, &small_frames[0]);
        } else {
            let pidx = smw_pose_to_link_idx(pose);
            let (ref top, ref bot) = super_pairs[pidx];
            write_smw_gfx32_16x16(&mut gfx32, va, top);
            write_smw_gfx32_16x16(&mut gfx32, vb, bot);
        }
    }

    let comp_gfx32 = crate::disasm_dma::compress_nintendo_lz2_with_endian(&gfx32, true);
    if comp_gfx32.len() <= 0x3FC0 {
        session.swap_custom_bytes(
            rom_a,
            0x040000,
            0x080000,
            comp_gfx32,
            "Swap Joueur Principal LC_LZ2 4bpp : Link (Zelda ALTTP 0x080000) -> Mario GFX32 (Super Mario World 0x040000, Petit/Super/Feu/Cape)".to_string(),
            crate::swapper::SwapKind::GraphicsTiles {
                format: TileFormat::Bpp4,
                tile_count: 744,
                smart_remapped: true,
            },
        )?;
        ops += 1;
    }

    // 2. Also replace Overworld Map Mario in GFX10 (PC 0x04EF1E..0x04F7AF, budget 2,193 bytes, 3bpp LC_LZ2)
    if let Some((mut g10, _)) =
        crate::disasm_dma::decompress_nintendo_lz2_with_endian(&rom_a.data, 0x04EF1E, 4096, true)
    {
        if g10.len() >= 3072 {
            let cmap3: [u8; 16] = [0, 1, 6, 7, 5, 2, 6, 3, 4, 4, 4, 4, 5, 5, 3, 1];
            let ow_walk1 = make_small_link_16x16(&rom_b.data, (0, 0), (2, 2), true, &cmap3);
            let ow_walk2 = make_small_link_16x16(&rom_b.data, (0, 0), (4, 2), true, &cmap3);
            let ow_front1 = make_small_link_16x16(&rom_b.data, (1, 0), (3, 1), false, &cmap3);
            let ow_front2 = make_small_link_16x16(&rom_b.data, (1, 0), (4, 1), false, &cmap3);
            let ow_back1 = make_small_link_16x16(&rom_b.data, (2, 0), (6, 1), false, &cmap3);
            let ow_back2 = make_small_link_16x16(&rom_b.data, (2, 0), (0, 2), false, &cmap3);

            for &((mx, my), px) in &[
                ((3usize, 0usize), &ow_walk1),
                ((4, 0), &ow_walk2),
                ((5, 0), &ow_front1),
                ((6, 0), &ow_front2),
                ((7, 0), &ow_back1),
                ((0, 1), &ow_back2),
                ((2, 1), &ow_front1),
                ((3, 2), &ow_back1),
                ((2, 3), &ow_walk1),
                ((3, 3), &ow_back2),
            ] {
                write_smw_gfx10_3bpp_16x16(&mut g10, mx, my, px);
            }

            let comp_g10 = crate::disasm_dma::compress_nintendo_lz2_with_endian(&g10, true);
            if comp_g10.len() <= (0x04F7AF - 0x04EF1E) {
                session.swap_custom_bytes(
                    rom_a,
                    0x04EF1E,
                    0x080000,
                    comp_g10,
                    "Swap Carte du Monde (Overworld) LC_LZ2 3bpp : Link (Zelda ALTTP) -> Mario Overworld GFX10 (0x04EF1E)".to_string(),
                    crate::swapper::SwapKind::GraphicsTiles {
                        format: TileFormat::Bpp4,
                        tile_count: 128,
                        smart_remapped: true,
                    },
                )?;
                ops += 1;
            }
        }
    }

    // 3. Replace Mario, Luigi, Fire Mario & Fire Luigi CGRAM Palettes at PC 0x0032C8..0x003318 (80 bytes = 4 x 10 BGR555 colors)
    //    with Link's Green Mail (0x0DD308), Blue Mail (0x0DD326), and Red Mail (0x0DD344) palettes from Zelda ALTTP!
    let extract_link_10_colors = |pal_off: usize| -> [u8; 20] {
        let mut out = [0u8; 20];
        // Map CGRAM $86..$8F (indices 6..15) to Link's ALTTP palette indices [2, 3, 8, 9, 10, 11, 12, 13, 14, 7]
        let src_indices: [usize; 10] = [2, 3, 8, 9, 10, 11, 12, 13, 14, 7];
        for (dst_i, &src_c) in src_indices.iter().enumerate() {
            let pos = pal_off + (src_c - 1) * 2;
            if pos + 1 < rom_b.data.len() {
                out[dst_i * 2] = rom_b.data[pos];
                out[dst_i * 2 + 1] = rom_b.data[pos + 1];
            }
        }
        // Harmonisation de la chevelure de Link :
        // Dans ALTTP, l'indice 8 (src_indices[2]) est un rose vif ($599D / #EE62B4).
        // On le remplace par un blond doré noble (BGR555 0x1A76: R=22, G=19, B=6 -> #B49C30).
        // L'ombre de la chevelure (src_indices[7] = 13) est un châtain doré (0x122F: R=15, G=17, B=4).
        let blond_hair = 0x1A76u16;
        out[2 * 2] = (blond_hair & 0xFF) as u8;
        out[2 * 2 + 1] = ((blond_hair >> 8) & 0xFF) as u8;
        let blond_shadow = 0x122Fu16;
        out[7 * 2] = (blond_shadow & 0xFF) as u8;
        out[7 * 2 + 1] = ((blond_shadow >> 8) & 0xFF) as u8;
        out
    };

    let green_mail = extract_link_10_colors(0x0DD308);
    let blue_mail = extract_link_10_colors(0x0DD326);
    let red_mail = extract_link_10_colors(0x0DD344);

    let mut player_pals = Vec::with_capacity(80);
    player_pals.extend_from_slice(&green_mail); // Mario -> Green Tunic Link
    player_pals.extend_from_slice(&blue_mail); // Luigi -> Blue Mail Link
    player_pals.extend_from_slice(&red_mail); // Fire Mario -> Red Mail Link
    player_pals.extend_from_slice(&blue_mail); // Fire Luigi -> Blue Mail Link

    session.swap_custom_bytes(
        rom_a,
        mario_pal_base,
        0x0DD308,
        player_pals,
        format!("Swap Palettes Joueur BGR555 (0x{:06X}..0x{:06X}) : Tunique Verte (0x0DD308), Bleue (0x0DD326) & Rouge (0x0DD344) de Link -> Mario/Luigi/Feu", mario_pal_base, mario_pal_base + 80),
        crate::swapper::SwapKind::PaletteBgr555 { color_count: 40 },
    )?;
    ops += 1;

    // 4. Also dynamically locate & update Overworld Mario's 3bpp Sprite Palette (7 colors: White, Black, DarkRed, BrightRed, Blue, DarkSkin, LightSkin)
    //    so Link wears his iconic Hyrule Green Tunic & Golden-Blond Hair on the Overworld Map too!
    const OW_MARIO_PAL_SIG: [u8; 14] = [
        0xFF, 0x7F, // 1: White
        0x00, 0x00, // 2: Black outline
        0x16, 0x00, // 3: Dark Red
        0x1F, 0x00, // 4: Bright Red
        0x08, 0x6D, // 5: Blue
        0xDD, 0x2D, // 6: Dark Skin
        0x5F, 0x63, // 7: Light Skin
    ];
    let ow_scan_end = rom_a.data.len().min(0x008000);
    if let Some(ow_pal_off) = rom_a.data[..ow_scan_end]
        .windows(OW_MARIO_PAL_SIG.len())
        .position(|w| w == OW_MARIO_PAL_SIG)
    {
        let link_ow_words: [u16; 7] = [
            0x7FFF, // 1: Pure White
            0x0000, // 2: Black Outline
            0x0184, // 3: Deep Hyrule Forest Green (tunic shadow)
            0x16C8, // 4: Bright Hyrule Green (tunic & cap)
            0x1A76, // 5: Golden-Blond Hair & Boots
            0x223B, // 6: Link Skin Shadow
            0x33FF, // 7: Link Bright Skin
        ];
        let mut ow_bytes = Vec::with_capacity(14);
        for w in link_ow_words {
            ow_bytes.extend_from_slice(&w.to_le_bytes());
        }
        session.swap_custom_bytes(
            rom_a,
            ow_pal_off,
            0x0DD308,
            ow_bytes,
            format!("Swap Palette Overworld BGR555 @ 0x{:06X} : Link Tunique Verte sur la Carte du Monde", ow_pal_off),
            crate::swapper::SwapKind::PaletteBgr555 { color_count: 7 },
        )?;
        ops += 1;
    }

    rom_a.fix_checksum();
    Ok(ops)
}

/// Extract a 16x16 tile block from decompressed SMW GFX32 (4bpp) using SMW's hardware DMA formula.
pub(crate) fn read_smw_gfx32_16x16(buf: &[u8], val: u8) -> [[u8; 16]; 16] {
    let off = (((val as usize) & 0x08) << 11) | (((val as usize) & 0xF7) << 6);
    let coords = [off, off + 32, off + 512, off + 544];
    let mut px = [[0u8; 16]; 16];
    for (q, &src) in coords.iter().enumerate() {
        if src + 32 > buf.len() {
            continue;
        }
        let qx = (q & 1) * 8;
        let qy = (q >> 1) * 8;
        let tile = &buf[src..src + 32];
        for y in 0..8usize {
            let b0 = tile[y * 2];
            let b1 = tile[y * 2 + 1];
            let b2 = tile[16 + y * 2];
            let b3 = tile[16 + y * 2 + 1];
            for x in 0..8usize {
                let sh = 7 - x;
                let c = ((b0 >> sh) & 1)
                    | (((b1 >> sh) & 1) << 1)
                    | (((b2 >> sh) & 1) << 2)
                    | (((b3 >> sh) & 1) << 3);
                px[qy + y][qx + x] = c;
            }
        }
    }
    px
}

/// Write a 16x16 4bpp metasprite into Zelda ALTTP's uncompressed Link sprite bank (`0x080000..0x087000`).
fn write_alttp_link_16x16(rom_data: &mut [u8], mx: usize, my: usize, px16: &[[u8; 16]; 16]) {
    let base = 0x080000 + (my * 32 + mx * 2) * 32;
    let coords = [base, base + 32, base + 512, base + 544];
    for (q, &dst) in coords.iter().enumerate() {
        if dst + 32 > rom_data.len() {
            continue;
        }
        let qx = (q & 1) * 8;
        let qy = (q >> 1) * 8;
        let mut tile = [0u8; 32];
        for y in 0..8usize {
            let (mut b0, mut b1, mut b2, mut b3) = (0u8, 0u8, 0u8, 0u8);
            for x in 0..8usize {
                let c = px16[qy + y][qx + x] & 0x0F;
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
        rom_data[dst..dst + 32].copy_from_slice(&tile);
    }
}

/// Build a brand-new, 100% playable hybrid SNES ROM based on **The Legend of Zelda: A Link to the Past**
/// where **Link** is completely replaced by **100% authentic pixel-for-pixel Super Mario World Mario**
/// extracted directly from `rom_smw` (`GFX32` 4bpp tiles + authentic SMW Mario / Fire Mario / Luigi
/// CGRAM palettes from `0x0032DB` / `0x003303` / `0x0032EF` + `SUPER MARIO HYRULE` header)!
pub fn apply_mario_into_zelda_alttp_swap(
    rom_zelda: &mut SnesRom,
    rom_smw: &SnesRom,
    session: &mut crate::swapper::SwapSession,
) -> Result<usize, crate::rom::RomError> {
    let mut ops = 0usize;

    // 1. Decompress Super Mario World's authentic GFX32 (PC 0x040000, 744 4bpp tiles)
    let g32 = crate::disasm_dma::decompress_nintendo_lz2_with_endian(
        &rom_smw.data,
        0x040000,
        32768,
        true,
    )
    .map(|(d, _)| d)
    .ok_or(crate::rom::RomError::OutOfBounds {
        offset: 0x040000,
        len: 23808,
    })?;

    let (pose_top_base, pose_bot_base) = find_smw_mario_pose_tables(&rom_smw.data);
    let (_yoshi_pal_base, mario_pal_base) = find_smw_yoshi_and_mario_palettes(&rom_smw.data);

    // 100% UNSQUISHED, PIXEL-FOR-PIXEL AUTHENTIC Super Mario World GFX32 (full 16x32 = va top + vb bottom)!
    // Paired with the surgical Bank $0D Link Head OAM Y-offset patch (0x068000..0x06838D) that raises
    // Link's Head OAM from Y+0 to Y-8 (so Body at Y+8 sits EXACTLY 16px below Head, matching SMW's
    // hardware 16x16+16x16 OAM stack with ZERO scanlines deleted, ZERO overlap, and ZERO squishing!).
    // Because we load SMW's exact 15-color CGRAM index layout ($81..$8F) 1-to-1 into Zelda 0x0DD308,
    // ZERO color remapping is needed: pixel index c in SMW == pixel index c in Zelda!
    let mirror_h = |src: &[[u8; 16]; 16]| -> [[u8; 16]; 16] {
        let mut out = [[0u8; 16]; 16];
        for y in 0..16 {
            for x in 0..16 {
                out[y][x] = src[y][15 - x];
            }
        }
        out
    };

    let remap_no_glove_idx = |mut px: [[u8; 16]; 16]| -> [[u8; 16]; 16] {
        // CRITICAL ALTTP HARDWARE FIX (#3):
        // Equipping Power Glove (0x354=1) or Titan's Mitt (0x354=2) overwrites Link CGRAM index 13
        // from ROM PC 0x0DEDF5/0x0DEDF7 (0x0376 = LIME GREEN!). Remapping SMW pixel index 13 -> 7
        // (an unused slot NEVER overwritten by ALTTP!) eliminates 100% of green/red cap flicker!
        for row in &mut px {
            for c in row.iter_mut() {
                if *c == 13 {
                    *c = 7;
                }
            }
        }
        px
    };

    let extract_smw_mario = |pose: usize, hflip: bool| -> ([[u8; 16]; 16], [[u8; 16]; 16]) {
        let va = rom_smw
            .data
            .get(pose_top_base + 0x46 + pose)
            .copied()
            .unwrap_or(0);
        let vb = rom_smw
            .data
            .get(pose_bot_base + 0x46 + pose)
            .copied()
            .unwrap_or(0);
        let top = remap_no_glove_idx(read_smw_gfx32_16x16(&g32, va));
        let bot = remap_no_glove_idx(read_smw_gfx32_16x16(&g32, vb));
        if hflip {
            (mirror_h(&top), mirror_h(&bot))
        } else {
            (top, bot)
        }
    };

    // Lift left or right boot cleanly by 2px (plus subtle opposite shoulder sway) for smooth authentic SMW strides
    let make_step_body = |base: &[[u8; 16]; 16], lift_left: bool| -> [[u8; 16]; 16] {
        let mut out = *base;
        let (x0, x1) = if lift_left {
            (0usize, 8usize)
        } else {
            (8usize, 16usize)
        };
        for y in 10..14usize {
            for x in x0..x1 {
                out[y][x] = base[y + 2][x];
            }
        }
        for y in 14..16usize {
            for x in x0..x1 {
                out[y][x] = 0;
            }
        }
        out
    };

    // Dedicated Overhead-Carrying Mario Heads & Bodies (calibrated pixel-for-pixel to LIVE Mednafen x4 zoom!):
    // With Z=22 carry height, the bottom edge of the Turn-Block / Emerald Smiling Bush rests right at Head y=3,
    // so wide rounded white SMW gloves at y=3..6 (x=0..3 & x=12..15) VISIBLY cradle the underside/sides of the block
    // with ZERO pointy "cat ears" above Mario's cap (y=0..2 stays 100% clear!), ZERO "rabbit ears" along his cheeks!
    let make_carry_front_head = |base: &[[u8; 16]; 16]| -> [[u8; 16]; 16] {
        let mut out = *base;
        // Left wide rounded white SMW glove cradling block underside (y=3..6, x=0..3) + compact red elbow (y=7..9)
        out[3][1] = 2;
        out[3][2] = 2;
        out[3][3] = 2;
        out[4][0] = 2;
        out[4][1] = 1;
        out[4][2] = 1;
        out[4][3] = 1;
        out[4][4] = 2;
        out[5][0] = 2;
        out[5][1] = 1;
        out[5][2] = 1;
        out[5][3] = 2;
        out[6][0] = 2;
        out[6][1] = 3;
        out[6][2] = 2;
        out[7][0] = 2;
        out[7][1] = 9;
        out[7][2] = 9;
        out[8][1] = 2;
        out[8][2] = 9;
        out[9][1] = 2;
        out[9][2] = 7;
        // Right wide rounded white SMW glove cradling block underside (y=3..6, x=12..15) + compact red elbow (y=7..9)
        out[3][12] = 2;
        out[3][13] = 2;
        out[3][14] = 2;
        out[4][11] = 2;
        out[4][12] = 1;
        out[4][13] = 1;
        out[4][14] = 1;
        out[4][15] = 2;
        out[5][12] = 2;
        out[5][13] = 1;
        out[5][14] = 1;
        out[5][15] = 2;
        out[6][13] = 2;
        out[6][14] = 3;
        out[6][15] = 2;
        out[7][13] = 9;
        out[7][14] = 9;
        out[7][15] = 2;
        out[8][13] = 9;
        out[8][14] = 2;
        out[9][13] = 7;
        out[9][14] = 2;
        out
    };

    let make_carry_side_head = |base: &[[u8; 16]; 16]| -> [[u8; 16]; 16] {
        // Keep Mario's authentic SMW side profile (cap, visor, nose, mustache) 100% clean and unobstructed
        // (ZERO floating detached gloves in front of his face/visor!), while raised arms connect from body up to held block.
        *base
    };

    // Clean Front Stand / Walk Mario body: replaces SMW's vine-climb/peace-sign shoulder white gloves
    // (y=0..4, x=0..4 & 11..15) with classic smooth red sleeves sloping down to crisp white gloves at hips!
    let make_clean_front_body = |base: &[[u8; 16]; 16]| -> [[u8; 16]; 16] {
        let mut out = *base;
        // Wipe vine-climbing / raised shoulder white glove pixels cleanly on y=0..5
        for y in 0..=5usize {
            for &x in &[0usize, 1, 2, 3, 4, 11, 12, 13, 14, 15] {
                if matches!(out[y][x], 1 | 2 | 3 | 7 | 8 | 9) {
                    out[y][x] = 0;
                }
            }
        }
        // Smooth classic Mario red sleeves sloping from collar down to hips (y=1..5)
        out[1][3] = 8;
        out[1][4] = 9;
        out[1][11] = 9;
        out[1][12] = 8;
        out[2][2] = 8;
        out[2][3] = 9;
        out[2][4] = 9;
        out[2][11] = 9;
        out[2][12] = 9;
        out[2][13] = 8;
        out[3][1] = 8;
        out[3][2] = 9;
        out[3][3] = 9;
        out[3][4] = 7;
        out[3][11] = 7;
        out[3][12] = 9;
        out[3][13] = 9;
        out[3][14] = 8;
        out[4][1] = 8;
        out[4][2] = 9;
        out[4][3] = 7;
        out[4][12] = 7;
        out[4][13] = 9;
        out[4][14] = 8;
        // Crisp rounded white SMW gloves naturally at sides (y=5..7)
        out[5][1] = 2;
        out[5][2] = 1;
        out[5][3] = 2;
        out[5][12] = 2;
        out[5][13] = 1;
        out[5][14] = 2;
        out[6][1] = 2;
        out[6][2] = 1;
        out[6][3] = 3;
        out[6][12] = 3;
        out[6][13] = 1;
        out[6][14] = 2;
        out[7][2] = 2;
        out[7][13] = 2;
        out
    };

    let make_carry_front_body = |base: &[[u8; 16]; 16]| -> [[u8; 16]; 16] {
        let mut out = *base;
        // Clear ALL old arm/hand/brown-spike pixels across y=0..=9 (including y=0..1!) on both sides!
        for y in 0..=9usize {
            for &x in &[0usize, 1, 2, 3, 12, 13, 14, 15] {
                if out[y][x] != 0xA && out[y][x] != 0xB && out[y][x] != 0xC {
                    out[y][x] = 0;
                }
            }
        }
        // Smooth compact raised red shoulders tucked cleanly into Head's curved sleeves (ZERO brown spikes!)
        out[0][1] = 2;
        out[0][2] = 9;
        out[0][3] = 9;
        out[0][12] = 9;
        out[0][13] = 9;
        out[0][14] = 2;
        out[1][1] = 2;
        out[1][2] = 9;
        out[1][3] = 9;
        out[1][12] = 9;
        out[1][13] = 9;
        out[1][14] = 2;
        out[2][1] = 2;
        out[2][2] = 9;
        out[2][3] = 7;
        out[2][12] = 7;
        out[2][13] = 9;
        out[2][14] = 2;
        out[3][2] = 2;
        out[3][3] = 7;
        out[3][12] = 7;
        out[3][13] = 2;
        out[4][2] = 2;
        out[4][13] = 2;
        out
    };

    let make_carry_side_body = |base: &[[u8; 16]; 16]| -> [[u8; 16]; 16] {
        let mut out = *base;
        // Erase lowered front/back white gloves at waist (NO arms hanging down while carrying a block!)
        for y in 0..=9usize {
            for x in 0..16usize {
                if out[y][x] == 1 {
                    out[y][x] = if (4..=11).contains(&x) { 9 } else { 0 };
                }
            }
            for x in 11..16usize {
                if matches!(out[y][x], 2 | 3 | 4 | 5 | 7 | 8 | 9) {
                    out[y][x] = 0;
                }
            }
        }
        // Clean raised red shoulder/upper arm angled upward toward the raised white gloves on (6,11)!
        for y in 0..=3usize {
            out[y][7] = 9;
            out[y][8] = 9;
            out[y][9] = 9;
            out[y][10] = 2;
        }
        out[0][6] = 2;
        out[1][6] = 9;
        out[2][10] = 7;
        out[3][9] = 2;
        out
    };

    // Close ONLY Mario's eye sclera/pupils (y=8..10, x=5..10) on genuine SMW pose 0x0F head for Bed Sleeping
    let make_sleep_head = |head: &[[u8; 16]; 16]| -> [[u8; 16]; 16] {
        let mut out = *head;
        for y in 8..=10usize {
            for x in 5..=10usize {
                if out[y][x] == 1 || out[y][x] == 2 {
                    out[y][x] = 6; // skin
                }
            }
        }
        for &x in &[6usize, 7, 9, 10] {
            out[10][x] = 2; // closed black eyelash
        }
        out
    };

    // Dedicated sword/bow action bodies calibrated to LIVE Mednafen SNES PPU frames (ZERO knee dots, ZERO white shoulder glitches!):
    // 1. body_sword_side: athletic side lunge with smooth SMW red sleeve and rounded white-gloved fist
    //    gripping ALTTP's sword / Bill Blaster cleanly at x=11..14, y=3..6!
    let make_sword_side_body = |base: &[[u8; 16]; 16]| -> [[u8; 16]; 16] {
        let mut out = *base;
        for y in 0..=7usize {
            for x in 8..16usize {
                if matches!(out[y][x], 1..=3) {
                    out[y][x] = 0;
                }
            }
        }
        for x in 6..=10usize {
            out[2][x] = 8;
            out[3][x] = 9;
            out[4][x] = 9;
            out[5][x] = 7;
        }
        out[5][10] = 3;
        out[2][12] = 2;
        out[2][13] = 2;
        out[3][11] = 3;
        out[3][12] = 1;
        out[3][13] = 1;
        out[3][14] = 2;
        out[4][11] = 3;
        out[4][12] = 1;
        out[4][13] = 1;
        out[4][14] = 1;
        out[5][11] = 3;
        out[5][12] = 1;
        out[5][13] = 1;
        out[5][14] = 2;
        out[6][12] = 2;
        out[6][13] = 2;
        out
    };

    // 2. body_sword_down: clean front stance (built from clean_front_body — NO vine-climb shoulders, NO floating knee circle!)
    //    with right red sleeve extended neatly down to side fist gripping the sword/tool!
    let make_sword_down_body = |clean_front: &[[u8; 16]; 16]| -> [[u8; 16]; 16] {
        let mut out = *clean_front;
        // Extend right arm neatly outward/downward for sword/item stance
        out[3][12] = 9;
        out[3][13] = 9;
        out[3][14] = 8;
        out[4][12] = 9;
        out[4][13] = 9;
        out[4][14] = 7;
        out[5][12] = 3;
        out[5][13] = 1;
        out[5][14] = 1;
        out[5][15] = 2;
        out[6][12] = 2;
        out[6][13] = 1;
        out[6][14] = 1;
        out[6][15] = 2;
        out[7][13] = 2;
        out[7][14] = 2;
        out
    };

    // 3. body_sword_up: back view with LEFT arm raised at x=0..4, y=0..6 holding upward sword hilt!
    let make_sword_up_body = |base: &[[u8; 16]; 16]| -> [[u8; 16]; 16] {
        let mut out = *base;
        for y in 3..=6usize {
            for x in 1..=4usize {
                out[y][x] = if x == 1 {
                    8
                } else if y == 6 {
                    7
                } else {
                    9
                };
            }
        }
        out[0][1] = 2;
        out[0][2] = 2;
        out[1][0] = 2;
        out[1][1] = 1;
        out[1][2] = 1;
        out[1][3] = 3;
        out[2][0] = 2;
        out[2][1] = 1;
        out[2][2] = 1;
        out[2][3] = 3;
        out[3][1] = 3;
        out[3][2] = 3;
        out
    };

    // Extract 100% GENUINE OFFICIAL NINTENDO Super Mario World Super Mario poses (ZERO hand-drawn sprites!)
    // Side (H-flipped to face Right for ALTTP; ALTTP mirrors horizontally for Left automatically!)
    let (head_side, body_side_stand) = extract_smw_mario(0x00, true);
    let (_, body_side_w1) = extract_smw_mario(0x01, true);
    let (_, body_side_w2) = extract_smw_mario(0x02, true);
    let body_side_r1 = make_step_body(&body_side_w2, false);
    let body_side_r2 = body_side_w1;
    let (_, body_side_fall) = extract_smw_mario(0x0D, true);

    // Front (Down): pose 0x0F & 0x22 cleaned of raised vine-climb shoulder gloves -> classic SMW front stand & stride!
    let (head_down, raw_body_down_stand) = extract_smw_mario(0x0F, false);
    let (_, raw_body_down_w1) = extract_smw_mario(0x22, false);
    let body_down_stand = make_clean_front_body(&raw_body_down_stand);
    let body_down_w1 = make_clean_front_body(&raw_body_down_w1);
    let body_down_w2 = mirror_h(&body_down_w1);
    let head_down_sleep = make_sleep_head(&head_down);

    // Back (Up): pose 0x39 (authentic neat back stand with arms at sides) + crisp alternating boot stride
    let (head_up, body_up_stand) = extract_smw_mario(0x39, false);
    let body_up_w1 = make_step_body(&body_up_stand, true);
    let body_up_w2 = make_step_body(&body_up_stand, false);

    // Overhead-carrying poses (verified via jpdasm bank_00.asm: Head=(5..7,11), Body=(7,13)+(0..7,14)!)
    let head_carry_down = make_carry_front_head(&head_down);
    let head_carry_side = make_carry_side_head(&head_side);
    let head_carry_up = make_carry_front_head(&head_up);
    let body_carry_down_stand = make_carry_front_body(&body_down_stand);
    let body_carry_down_w1 = make_carry_front_body(&body_down_w1);
    let body_carry_down_w2 = make_carry_front_body(&body_down_w2);
    let body_carry_up_stand = make_carry_front_body(&body_up_stand);
    let body_carry_up_w1 = make_carry_front_body(&body_up_w1);
    let body_carry_up_w2 = make_carry_front_body(&body_up_w2);
    let body_carry_side_stand = make_carry_side_body(&body_side_stand);
    let body_carry_side_w1 = make_carry_side_body(&body_side_w1);
    let body_carry_side_w2 = make_carry_side_body(&body_side_w2);

    // Dedicated sword-holding bodies:
    let body_sword_side = make_sword_side_body(&body_side_w1);
    let body_sword_down = make_sword_down_body(&body_down_w1);
    let body_sword_up = make_sword_up_body(&body_up_stand);

    // Clone the entire 28,672-byte Link sprite bank (0x080000..0x087000) so FX/sword-arcs/sparkles/bunny stay intact
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

    // Row 0: Primary Heads (Down, Up, Side, Talk, Sleep, Push)
    write_cell(0, 0, &head_side);
    write_cell(1, 0, &head_down);
    write_cell(2, 0, &head_up);
    write_cell(3, 0, &head_down);
    write_cell(4, 0, &head_down);
    write_cell(5, 0, &head_down_sleep);
    write_cell(6, 0, &head_down);
    write_cell(7, 0, &head_down);

    // Row 1: Side Bow/Action Bodies (0..1 -> verified via live Mednafen Image 1!), Down Action Body (2),
    //        Primary Down Bodies (3..5), Primary Up Stand Body (6), Alt Side Head (7)
    write_cell(0, 1, &body_sword_side);
    write_cell(1, 1, &body_sword_side);
    write_cell(2, 1, &body_sword_down);
    write_cell(3, 1, &body_down_stand);
    write_cell(4, 1, &body_down_w1);
    write_cell(5, 1, &body_down_w2);
    write_cell(6, 1, &body_up_stand);
    write_cell(7, 1, &head_side);

    // Row 2: Primary Up Walk Bodies + Primary Side Stand/Walk + Up Sword-Swing Bodies (5..6)
    write_cell(0, 2, &body_up_w1);
    write_cell(1, 2, &body_up_w2);
    write_cell(2, 2, &body_side_w1);
    write_cell(3, 2, &body_side_stand);
    write_cell(4, 2, &body_side_w2);
    write_cell(5, 2, &body_sword_up);
    write_cell(6, 2, &body_sword_up);
    write_cell(7, 2, &body_side_w2);

    // Row 3: Secondary Walk / Sword / Carry Bodies
    write_cell(0, 3, &body_side_fall);
    write_cell(1, 3, &body_down_w1);
    write_cell(2, 3, &body_down_w2);
    write_cell(3, 3, &body_down_stand);
    write_cell(4, 3, &body_down_w1);
    write_cell(5, 3, &body_up_w1);
    write_cell(6, 3, &body_up_w2);
    write_cell(7, 3, &body_side_w1);

    // Row 4: Lift / Overhead Carry Bodies & Secondary Heads + Mario Pit-Fall / Swim (cols 5..7)
    write_cell(0, 4, &body_carry_down_stand);
    write_cell(1, 4, &body_carry_down_w1);
    write_cell(2, 4, &head_side);
    write_cell(3, 4, &head_down);
    write_cell(4, 4, &head_up);
    write_cell(5, 4, &head_down); // Pit-fall shrinking Mario f0
    write_cell(6, 4, &head_up); // Pit-fall shrinking Mario f1
    write_cell(7, 4, &head_down); // Swimming Mario head front

    // Row 5-6: Bed Wake-Up / Sitting / Holding Item poses + Action Heads
    write_cell(0, 5, &head_side);
    write_cell(0, 6, &body_side_stand);
    write_cell(1, 5, &head_up);
    write_cell(1, 6, &body_up_stand);
    write_cell(2, 5, &head_down);
    write_cell(2, 6, &body_down_stand);
    write_cell(3, 5, &head_side);
    write_cell(3, 6, &body_side_w1);
    write_cell(4, 5, &head_up);
    write_cell(4, 6, &body_up_stand);
    write_cell(5, 5, &body_side_w1);
    write_cell(6, 5, &body_side_w2);
    write_cell(7, 5, &body_down_stand);
    write_cell(5, 6, &head_side);
    write_cell(6, 6, &head_up);
    write_cell(7, 6, &body_side_stand);

    // Row 7: Swimming Heads (cols 5..7)
    write_cell(5, 7, &head_down);
    write_cell(6, 7, &head_side);
    write_cell(7, 7, &head_up);

    // Row 8: Side Attack Bodies (cols 0..2 ONLY; cols 3..7 are sword spin arcs -> preserved!)
    write_cell(0, 8, &body_sword_side);
    write_cell(1, 8, &body_sword_side);
    write_cell(2, 8, &body_sword_side);

    // Row 9: Swimming Mario (cols 3..4 ONLY; cols 0..2 are sword spin arcs & col 5 is splash -> preserved!)
    write_cell(3, 9, &head_down);
    write_cell(4, 9, &head_side);

    // Row 10-11: Dash / Swim / Ledge Heads & Bodies (preserving sword arcs in Row 9 & (0,11)!)
    write_cell(1, 10, &body_up_stand);
    write_cell(2, 10, &body_down_stand);
    write_cell(3, 10, &head_side);
    write_cell(4, 10, &head_side);
    write_cell(5, 10, &head_up);
    write_cell(6, 10, &body_side_w1);
    write_cell(7, 10, &head_down);
    write_cell(1, 11, &head_side);
    write_cell(2, 11, &body_down_stand);
    write_cell(3, 11, &body_down_w1);
    write_cell(4, 11, &body_up_w1);
    // Verified via jpdasm bank_00.asm: (5,11)=Carry Down Head, (6,11)=Carry Side Head, (7,11)=Carry Up Head!
    write_cell(5, 11, &head_carry_down);
    write_cell(6, 11, &head_carry_side);
    write_cell(7, 11, &head_carry_up);

    // Row 12-14: Shield-walk & Overhead Carrying Bodies (verified via jpdasm bank_00.asm LinkOAM_BodyAddresses!)
    for mx in 0..3 {
        write_cell(
            mx,
            12,
            if mx % 2 == 0 {
                &body_down_w1
            } else {
                &body_down_w2
            },
        );
    }
    for mx in 3..6 {
        write_cell(
            mx,
            12,
            if mx % 2 == 0 {
                &body_up_w1
            } else {
                &body_up_w2
            },
        );
    }
    write_cell(6, 12, &body_side_w1);
    write_cell(7, 12, &body_side_w2);

    // Row 13-14: Sword/Action + VERIFIED Carrying Bodies:
    //   (7,13)=CarryDown0, (0,14)=CarryDown1, (1,14)=CarryDown2,
    //   (2,14)=CarrySide0, (3,14)=CarrySide1, (4,14)=CarrySide2,
    //   (5,14)=CarryUp0,   (6,14)=CarryUp1,   (7,14)=CarryUp2!
    write_cell(0, 13, &body_down_stand);
    write_cell(1, 13, &body_sword_down);
    write_cell(2, 13, &body_sword_down);
    write_cell(3, 13, &body_sword_down);
    write_cell(4, 13, &body_sword_down);
    write_cell(5, 13, &body_sword_up);
    write_cell(6, 13, &body_sword_up);
    write_cell(7, 13, &body_carry_down_stand);

    write_cell(0, 14, &body_carry_down_w1);
    write_cell(1, 14, &body_carry_down_w2);
    write_cell(2, 14, &body_carry_side_stand);
    write_cell(3, 14, &body_carry_side_w1);
    write_cell(4, 14, &body_carry_side_w2);
    write_cell(5, 14, &body_carry_up_stand);
    write_cell(6, 14, &body_carry_up_w1);
    write_cell(7, 14, &body_carry_up_w2);

    // Row 15-16: Shieldless Walk & Overhead Carry Bodies (preserving (0,15) dust FX!)
    // Note (verified jpdasm): (7,16) is Walk Left/Right pose 0x0BF -> MUST be body_side_w2 (zero carry flicker)!
    write_cell(1, 15, &body_side_w1);
    write_cell(2, 15, &body_down_stand);
    write_cell(3, 15, &body_down_w1);
    write_cell(4, 15, &body_down_w2);
    write_cell(5, 15, &body_up_stand);
    write_cell(6, 15, &body_side_w2);
    write_cell(7, 15, &body_carry_up_stand);

    write_cell(0, 16, &body_carry_up_w1);
    write_cell(1, 16, &body_carry_up_w2);
    write_cell(2, 16, &body_carry_up_stand);
    write_cell(3, 16, &body_carry_side_stand);
    write_cell(4, 16, &body_carry_side_w1);
    write_cell(5, 16, &body_carry_side_w2);
    write_cell(6, 16, &body_side_w1);
    write_cell(7, 16, &body_side_w2);

    // Row 17-19: Verified Walk Cycle Bodies (eliminating all jpdasm-audited direction-flicker bugs!):
    //   Walk Left/Right uses (6,17), (7,17), (3,18), (4,18) -> ALL body_side_*!
    //   Walk Down uses (5,18), (6,18) -> body_down_*!
    //   Walk Up uses (7,18), (3..7,19) -> ALL body_up_*!
    write_cell(1, 17, &head_down);
    write_cell(2, 17, &head_up);
    write_cell(3, 17, &body_side_r1);
    write_cell(4, 17, &body_side_r2);
    write_cell(5, 17, &body_down_w1);
    write_cell(6, 17, &body_side_w1);
    write_cell(7, 17, &body_side_stand);

    write_cell(1, 18, &head_side);
    write_cell(2, 18, &head_side);
    write_cell(3, 18, &body_side_w1);
    write_cell(4, 18, &body_side_w2);
    write_cell(5, 18, &body_down_w1);
    write_cell(6, 18, &body_down_w2);
    write_cell(7, 18, &body_up_stand);

    write_cell(0, 19, &head_down);
    write_cell(1, 19, &head_side);
    write_cell(2, 19, &head_up);
    write_cell(3, 19, &body_up_w1);
    write_cell(4, 19, &body_up_w2);
    write_cell(5, 19, &body_up_w1);
    write_cell(6, 19, &body_up_w2);
    write_cell(7, 19, &body_up_stand);

    write_cell(0, 20, &head_down);
    write_cell(1, 20, &head_side);
    write_cell(2, 20, &head_up);
    write_cell(3, 20, &body_carry_down_w1);
    write_cell(4, 20, &body_carry_down_w2);
    write_cell(5, 20, &body_carry_side_w1);
    write_cell(6, 20, &body_carry_side_w2);
    write_cell(7, 20, &body_carry_up_stand);

    write_cell(0, 21, &body_up_stand);
    write_cell(1, 21, &body_side_r1);
    write_cell(2, 21, &body_side_r2);
    write_cell(3, 21, &body_down_w1);
    write_cell(4, 21, &body_down_w2);
    write_cell(5, 21, &body_up_w1);
    write_cell(6, 21, &body_up_w2);
    write_cell(7, 21, &body_side_w1);

    // Row 22-25 & 27: Shieldless Side Walk / Dash / Carry / Climb / Chest Heads & Bodies (preserving Dark World Bunny in 25..26!)
    write_cell(0, 22, &body_carry_side_w1);
    write_cell(1, 22, &body_carry_side_w2);
    write_cell(2, 22, &body_carry_side_stand);
    write_cell(3, 22, &body_carry_side_w1);
    write_cell(4, 22, &body_side_r2);
    write_cell(5, 22, &body_side_w1);
    write_cell(6, 22, &body_side_w2);
    write_cell(7, 22, &body_side_stand);

    write_cell(0, 23, &body_side_w1);
    write_cell(1, 23, &head_side);
    write_cell(2, 23, &body_side_w2);
    write_cell(3, 23, &body_side_stand);
    write_cell(4, 23, &body_side_r1);
    write_cell(5, 23, &body_carry_down_w1);
    write_cell(6, 23, &body_carry_down_w2);
    write_cell(7, 23, &body_carry_down_stand);

    write_cell(0, 24, &body_side_w1);
    write_cell(1, 24, &body_side_w2);
    write_cell(2, 24, &body_up_w1);
    write_cell(3, 24, &body_up_w2);
    write_cell(4, 24, &body_side_r1);
    write_cell(5, 24, &body_down_w1);
    write_cell(6, 24, &head_down);
    write_cell(7, 24, &head_side);

    write_cell(0, 25, &head_up);
    write_cell(1, 25, &head_up);
    write_cell(2, 25, &head_down);
    write_cell(3, 25, &head_side);
    write_cell(4, 25, &body_side_stand);

    // Dark World transformation (Row 25 col 5 & 7, Row 26 cols 0..4) -> Adorable Tanooki-Eared Mario!
    let make_tanooki_head = |base: &[[u8; 16]; 16]| -> [[u8; 16]; 16] {
        let mut out = *base;
        // Cute brown Tanooki ears on Mario's cap!
        out[0][3] = 2;
        out[0][4] = 2;
        out[1][2] = 2;
        out[1][3] = 3;
        out[1][4] = 4;
        out[1][5] = 2;
        out[2][3] = 3;
        out[2][4] = 4;
        out[0][11] = 2;
        out[0][12] = 2;
        out[1][10] = 2;
        out[1][11] = 4;
        out[1][12] = 3;
        out[1][13] = 2;
        out[2][11] = 4;
        out[2][12] = 3;
        out
    };
    let head_tanooki_down = make_tanooki_head(&head_down);
    let head_tanooki_up = make_tanooki_head(&head_up);
    let head_tanooki_side = make_tanooki_head(&head_side);
    write_cell(5, 25, &head_tanooki_down);
    write_cell(7, 25, &body_down_stand);

    // Row 26 cols 0..4 = Tanooki Mario Dark World, cols 5..7 = Side Sword Thrust / Lunge bodies!
    write_cell(0, 26, &body_down_w1);
    write_cell(1, 26, &head_tanooki_up);
    write_cell(2, 26, &body_up_w1);
    write_cell(3, 26, &body_up_w2);
    write_cell(4, 26, &head_tanooki_side);
    write_cell(5, 26, &body_sword_side);
    write_cell(6, 26, &body_sword_side);
    write_cell(7, 26, &body_sword_side);

    write_cell(0, 27, &body_side_w1);
    write_cell(1, 27, &body_side_w2);
    write_cell(2, 27, &body_down_stand);
    write_cell(3, 27, &body_up_stand);
    write_cell(4, 27, &body_down_w1);
    drop(write_cell);

    // Cell (6, 25) bottom-left 8x8 quadrant (tile offset base + 512):
    // Zero out to guarantee NO horizontal crotch bar / tube!
    {
        let arm_tile_dst = (25 * 32 + 6 * 2) * 32 + 512;
        if arm_tile_dst + 32 <= link_bank.len() {
            link_bank[arm_tile_dst..arm_tile_dst + 32].fill(0);
        }
    }

    session.swap_custom_bytes(
        rom_zelda,
        0x080000,
        0x040000,
        link_bank,
        "Swap Héros 100% Authentique 4bpp (0x080000..0x087000) : Vrais sprites Super Mario World (GFX32) 16x32 complets non écrasés -> Link".to_string(),
        crate::swapper::SwapKind::GraphicsTiles {
            format: TileFormat::Bpp4,
            tile_count: 896,
            smart_remapped: true,
        },
    )?;
    ops += 1;

    // 1b. Surgical Bank $0D Link Head OAM Y-offset patch (PC 0x068000..0x06838D, 303 entries x 3 bytes):
    //     In vanilla ALTTP, Link_DrawOAM ALWAYS places Body at (Link_X, Link_Y + 8) ($0D:AA01 `ADC #$0800`),
    //     while Head sits at (Link_X + entry.x, Link_Y + entry.y) — including sword thrust poses where vanilla
    //     entry.y = +5/-5/+12 and entry.x = +-4! Patching ALL 299 Head+Body entries (fl & 0x0F != 0x0F) to
    //     entry.y = -8 (0xF8) and entry.x = 0 locks Body_Y - Head_Y to EXACTLY 16px across walking, running,
    //     AND every sword swing/thrust/spin direction with ZERO head-over-body overlap and ZERO neck gap!
    let oam_table_pc = 0x068000usize;
    let oam_table_len = 303 * 3;
    if oam_table_pc + oam_table_len <= rom_zelda.data.len() {
        let mut oam_table = rom_zelda.data[oam_table_pc..oam_table_pc + oam_table_len].to_vec();
        for i in 0..303usize {
            let fl = oam_table[i * 3 + 2];
            if (fl & 0x0F) != 0x0F {
                oam_table[i * 3] = (-8i8) as u8;
                oam_table[i * 3 + 1] = 0;
            }
        }
        session.swap_custom_bytes(
            rom_zelda,
            oam_table_pc,
            oam_table_pc,
            oam_table,
            "Patch OAM Bank $0D (0x068000..0x06838D) : Décalage tête Link Y=-8 (écart exact 16px tête/corps pour Super Mario 28px 100% fidèle SMW)".to_string(),
            crate::swapper::SwapKind::CodePatch {
                byte_count: oam_table_len,
            },
        )?;
        ops += 1;
    }

    // 1c. Surgical Bank $0D Sword OAM (X, Y) alignment patch (verified live in Mednafen!):
    //     Pulls the extended slash peak poses so the sword hilt locks DIRECTLY into Mario's white-gloved hand
    //     with ZERO gap and ZERO floating sword across ALL 4 directions (Down, Right, Up, Left)!
    //     Tables: Y1 @ 0x068EEF, X1 @ 0x0690EE (stride 0x1FF), Y2 @ 0x0698F3, X2 @ 0x069AF2 (stride 0x1FF).
    {
        const Y1_BASE: usize = 0x068EEF;
        const X1_BASE: usize = 0x0690EE;
        const Y2_BASE: usize = 0x0698F3;
        const X2_BASE: usize = 0x069AF2;
        if X2_BASE + 0x1FF <= rom_zelda.data.len()
            && rom_zelda.data[X1_BASE + 0x1FB] == 20
            && rom_zelda.data[Y1_BASE + 0x1FB] == 12
            && rom_zelda.data[X1_BASE + 0x1F2] == (-20i8) as u8
            && rom_zelda.data[Y1_BASE + 0x1E9] == 23
        {
            let mut yx1 = rom_zelda.data[Y1_BASE..X1_BASE + 0x1FF].to_vec();
            let mut yx2 = rom_zelda.data[Y2_BASE..X2_BASE + 0x1FF].to_vec();
            let set_pose = |yx1: &mut [u8], yx2: &mut [u8], pose: usize, dx: i8, dy: i8| {
                let y1_off = pose;
                let x1_off = (X1_BASE - Y1_BASE) + pose;
                let y2_off = pose;
                let x2_off = (X2_BASE - Y2_BASE) + pose;
                yx1[y1_off] = (yx1[y1_off] as i8).wrapping_add(dy) as u8;
                yx1[x1_off] = (yx1[x1_off] as i8).wrapping_add(dx) as u8;
                yx2[y2_off] = (yx2[y2_off] as i8).wrapping_add(dy) as u8;
                yx2[x2_off] = (yx2[x2_off] as i8).wrapping_add(dx) as u8;
            };
            // RIGHT slash peaks: pull 7px left & 1px up directly into Mario's white fist (x=11..14, y=3..5)!
            set_pose(&mut yx1, &mut yx2, 0x1FA, -4, 0);
            set_pose(&mut yx1, &mut yx2, 0x1FB, -7, -1);
            set_pose(&mut yx1, &mut yx2, 0x1FC, -4, -1);
            set_pose(&mut yx1, &mut yx2, 0x043, -3, -1);
            set_pose(&mut yx1, &mut yx2, 0x06A, -5, -1);
            // LEFT slash peaks: mirror right (+7px right & 1px up into Mario's mirrored white fist!)
            set_pose(&mut yx1, &mut yx2, 0x1F1, 4, 0);
            set_pose(&mut yx1, &mut yx2, 0x1F2, 7, -1);
            set_pose(&mut yx1, &mut yx2, 0x1F3, 4, -1);
            set_pose(&mut yx1, &mut yx2, 0x03A, 3, -1);
            set_pose(&mut yx1, &mut yx2, 0x067, 5, -1);
            // DOWN slash peaks: lift 7px up right into Mario's front white glove!
            set_pose(&mut yx1, &mut yx2, 0x1E8, 0, -4);
            set_pose(&mut yx1, &mut yx2, 0x1E9, 0, -7);
            set_pose(&mut yx1, &mut yx2, 0x1EA, 0, -3);
            set_pose(&mut yx1, &mut yx2, 0x031, 0, -3);
            // UP slash peaks: keep full shiny upward blade visible right above Mario's raised left white glove!
            set_pose(&mut yx1, &mut yx2, 0x1DF, -1, 1);
            set_pose(&mut yx1, &mut yx2, 0x1E0, -2, 1);
            set_pose(&mut yx1, &mut yx2, 0x1E1, -1, 1);

            session.swap_custom_bytes(
                rom_zelda,
                Y1_BASE,
                Y1_BASE,
                yx1,
                "Patch OAM Épée Bank $0D (0x068EEF) : Alignement garde d'épée 100% dans le gant blanc (4 directions)".to_string(),
                crate::swapper::SwapKind::CodePatch { byte_count: 0x3FE },
            )?;
            ops += 1;
            session.swap_custom_bytes(
                rom_zelda,
                Y2_BASE,
                Y2_BASE,
                yx2,
                "Patch OAM Lame Épée Bank $0D (0x0698F3) : Alignement lame 100% solidaire du gant (4 directions)".to_string(),
                crate::swapper::SwapKind::CodePatch { byte_count: 0x3FE },
            )?;
            ops += 1;
        }
    }

    // 2. Load 100% AUTHENTIC Super Mario World 15-color Mario Palettes (Fixes #3 & #12!):
    //    - ZERO green/red cap flicker: cap/shirt shade moved to index 7 (`words[6]`, never overwritten by ALTTP!)
    //      and Power Glove / Titan's Mitt override bytes (`0x0DEDF5..0x0DEDF9`) patched to pure white `0x7FFF`!
    //    - ZERO playable Luigi: Green Mail = Mario Classique, Blue Mail = Mario Bleu (Glace),
    //      Red Mail = Fire Mario, File Select = Mario Classique! Luigi remains strictly NPC Uncle (S 0x7F / S 0xC2)!
    let build_smw_mario_pal = |variant: u8| -> Vec<u8> {
        let mut words = [0u16; 15];
        words[0] = 0x7FFF; // 1: Pure White (eyes, big SMW gloves)
        words[1] = 0x0000; // 2: Pure Black (mustache, visor, hair, soles)
        words[2] = 0x0D71; // 3: Exact SMW Chestnut Brown (shoes, glove outlines)
        words[3] = 0x1E9B; // 4: Exact SMW Warm Gold (cap badge & shoe shine)
        words[4] = 0x3B7F; // 5: Exact SMW Light Gold (cap highlight)
                           // Load authentic SMW skin & buttons from mario_pal_base
        for i in 0..10 {
            let off = mario_pal_base + i * 2;
            if off + 1 < rom_smw.data.len() {
                words[5 + i] = u16::from_le_bytes([rom_smw.data[off], rom_smw.data[off + 1]]);
            }
        }
        match variant {
            0 => {
                // Mario Classique (Green Mail & File Select): Vibrant Red cap/shirt + Royal Blue overalls
                words[6] = 0x0857; // 7: Shaded Red cap/shirt (remapped from index 13 -> ZERO Glove flicker!)
                words[7] = 0x000A; // 8: Deep Crimson outline
                words[8] = 0x10DF; // 9: True SMW Bright Red Cap & Shirt
                words[9] = 0x4882; // 10: Deep Royal Blue Overalls outline
                words[10] = 0x6D85; // 11: Classic Blue Overalls mid
                words[11] = 0x7E6B; // 12: Sky Blue Overalls highlight
                words[12] = 0x7FFF; // 13: White (safe even if ALTTP glove slot reloads)
            }
            1 => {
                // Mario Bleu / Glace (Blue Mail): Vibrant Sky-Blue cap & shirt + Crisp Deep-Navy/White overalls!
                words[6] = 0x5D60; // 7: Shaded Royal Blue cap/shirt (index 7 -> ZERO Glove flicker!)
                words[7] = 0x3440; // 8: Deep Navy outline
                words[8] = 0x7E48; // 9: Bright Mario Ice/Sky Blue Cap & Shirt
                words[9] = 0x354B; // 10: Steel-Blue Overalls outline
                words[10] = 0x56B5; // 11: Crisp Ice-Silver Overalls mid
                words[11] = 0x77BD; // 12: Bright Ice-White Overalls highlight
                words[12] = 0x7FFF; // 13: White
            }
            _ => {
                // Fire Mario (Red Mail): Pure White cap & overalls + Fiery Crimson shirt!
                words[6] = 0x5AD6; // 7: Soft Silver-White cap shade (index 7 -> ZERO Glove flicker!)
                words[7] = 0x318C; // 8: Slate-Gray outline on white cap
                words[8] = 0x7FFF; // 9: Pure White Fire Mario Cap & Overalls
                words[9] = 0x000D; // 10: Deep Fire Crimson Shirt outline
                words[10] = 0x0017; // 11: Vibrant Fire Red Shirt mid
                words[11] = 0x111F; // 12: Bright Flame Orange-Red Shirt highlight
                words[12] = 0x7FFF; // 13: White
            }
        }
        let mut out = Vec::with_capacity(30);
        for w in words {
            out.extend_from_slice(&w.to_le_bytes());
        }
        out
    };

    let mario_classic_30 = build_smw_mario_pal(0);
    let mario_blue_30 = build_smw_mario_pal(1);
    let mario_fire_30 = build_smw_mario_pal(2);

    // Write all 4 Mail/File-Select Link palettes (NO playable Luigi! Luigi is strictly NPC Uncle!)
    let mut mail_pals = Vec::with_capacity(120);
    mail_pals.extend_from_slice(&mario_classic_30); // 0x0DD308 Green Mail -> Mario Classique
    mail_pals.extend_from_slice(&mario_blue_30); // 0x0DD326 Blue Mail  -> Mario Bleu (Glace)
    mail_pals.extend_from_slice(&mario_fire_30); // 0x0DD344 Red Mail   -> Fire Mario
    mail_pals.extend_from_slice(&mario_classic_30); // 0x0DD362 File Select -> Mario Classique

    session.swap_custom_bytes(
        rom_zelda,
        0x0DD308,
        mario_pal_base,
        mail_pals,
        format!("Swap Palettes 100% SMW BGR555 (0x{:06X} -> 0x0DD308..0x0DD380) : Mario Classique, Mario Bleu, Fire Mario (0 clignotement, 0 Luigi jouable)", mario_pal_base),
        crate::swapper::SwapKind::PaletteBgr555 { color_count: 60 },
    )?;
    ops += 1;

    // 2a. Patch ALTTP Power Glove (0x0DEDF5) & Titan's Mitt (0x0DEDF7) dynamic CGRAM index 13 overrides
    //     to pure white (0x7FFF) so picking up Power Glove or Titan's Mitt NEVER flashes lime-green on Mario!
    if 0x0DEDF9 <= rom_zelda.data.len() {
        session.swap_custom_bytes(
            rom_zelda,
            0x0DEDF5,
            0x0DEDF5,
            vec![0xFF, 0x7F, 0xFF, 0x7F],
            "Patch Gants Puissance / Titan (0x0DEDF5..0x0DEDF9 -> 0x7FFF) : Suppression totale du clignotement vert/rouge sur Mario".to_string(),
            crate::swapper::SwapKind::PaletteBgr555 { color_count: 2 },
        )?;
        ops += 1;
    }

    // 2b. MASSIVE SUPER MARIO WORLD -> ZELDA ALTTP WORLD EXPANSION!
    //     Extract authentic 3bpp sprites & tiles directly from `rom_smw` (GFX00, GFX01, GFX02, GFX03, GFX06)
    //     and inject them into Zelda ALTTP's uncompressed & LZ2-compressed 3bpp sheets:
    //       - Hyrule Castle Soldiers (S 0x8C) -> SMW Goombas (Galoombas) + Koopa Shell Shields!
    //       - Castle Sewer / Dungeon Enemies (S 0x8D) -> SMW Boos, Spiny & Goombas!
    //       - Overworld Enemies (S 0x7F, S 0x82) -> SMW Spiny & Piranha Plants!
    //       - Dungeon Stalfos (S 0x90) -> SMW Boos & Magikoopas!
    //       - Overworld/Dungeon Bombs, Keys & Slimes (S 0x72, S 0x7B) -> SMW Bob-ombs, SMW Keys & Goombas!
    //       - Rupees, Heart Containers & HUD Icons (S 0xCD, S 0x76) -> SMW Gold Coins, Super Mushrooms & Bob-ombs!
    //       - Overworld Liftable Rocks (S 0x05) -> SMW '?' Question Blocks!
    {
        // Locate SMW 3bpp GFX00..31 split pointer table (Lo, Hi, Bank, stride 0x32) via GFX10 @ 0x04EF1E ($09:EF1E)
        let mut smw_gfx_tbl: Option<(usize, usize, usize)> = None;
        for i in 0x003000..0x004400usize {
            if i + 0x64 < rom_smw.data.len()
                && rom_smw.data[i] == 0x1E
                && rom_smw.data[i + 0x32] == 0xEF
                && rom_smw.data[i + 0x64] == 0x09
            {
                let lo = i - 0x10;
                smw_gfx_tbl = Some((lo, lo + 0x32, lo + 0x64));
                break;
            }
        }

        // Locate ALTTP 223-sheet GFX pointer table (Bank, Hi, Lo, stride 223) via Sheet 0x71 = $10:F000 (0x087000)
        let mut alttp_gfx_tbl: Option<(usize, usize, usize)> = None;
        for b in 0x004E80..0x005000usize {
            let h = b + 223;
            let l = b + 446;
            if l + 223 <= rom_zelda.data.len()
                && rom_zelda.data[b + 0x71] == 0x10
                && rom_zelda.data[h + 0x71] == 0xF0
                && rom_zelda.data[l + 0x71] == 0x00
                && rom_zelda.data[b + 0x72] == 0x10
                && rom_zelda.data[h + 0x72] == 0xF6
            {
                alttp_gfx_tbl = Some((b, h, l));
                break;
            }
        }

        if let (Some((smw_lo, smw_hi, smw_bk)), Some((z_bk, z_hi, z_lo))) =
            (smw_gfx_tbl, alttp_gfx_tbl)
        {
            let load_smw_3bpp = |g: usize| -> Option<Vec<u8>> {
                let pc = ((rom_smw.data[smw_bk + g] as usize & 0x7F) << 15)
                    | ((rom_smw.data[smw_hi + g] as usize & 0x7F) << 8)
                    | (rom_smw.data[smw_lo + g] as usize);
                crate::disasm_dma::decompress_nintendo_lz2_with_endian(
                    &rom_smw.data,
                    pc,
                    4096,
                    true,
                )
                .map(|(d, _)| d)
            };

            let read_3bpp_16x16 = |buf: &[u8], tx: usize, ty: usize| -> [[u8; 16]; 16] {
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
            };

            let write_3bpp_16x16 = |buf: &mut [u8], tx: usize, ty: usize, px: &[[u8; 16]; 16]| {
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
            };

            let remap_16x16 = |src: &[[u8; 16]; 16], lut: &[u8; 8]| -> [[u8; 16]; 16] {
                let mut out = [[0u8; 16]; 16];
                for y in 0..16 {
                    for x in 0..16 {
                        out[y][x] = lut[(src[y][x] & 7) as usize];
                    }
                }
                out
            };

            let flip_h_16x16 = |src: &[[u8; 16]; 16]| -> [[u8; 16]; 16] {
                let mut out = [[0u8; 16]; 16];
                for y in 0..16 {
                    for x in 0..16 {
                        out[y][x] = src[y][15 - x];
                    }
                }
                out
            };

            // Universal SMW 3bpp -> Zelda ALTTP 3bpp color LUTs (verified visually in ALTTP real palettes!):
            // SMW: 0=trans, 1=white, 2=black, 3=dark skin/brown, 4=gold/tan, 5=cream/light, 6=bright primary, 7=dark primary
            // ALTTP: 0=trans, 1=white, 2=yellow/accent, 3=brown/dark, 4=tan/light, 5=dark outline, 6=bright primary, 7=dark primary
            const LUT_STD: [u8; 8] = [0, 1, 5, 3, 2, 4, 6, 7];
            // True chestnut-brown Goomba even in Green Soldier palette (PalRow 05: 4=tan, 3=chestnut brown, 5=black, 1=white)
            // and Blue Goomba in Blue Elite Soldier palette!
            const LUT_GOOMBA: [u8; 8] = [0, 1, 5, 3, 2, 4, 4, 3];
            // Vibrant bright Red cap + White spots Super Mushroom in Zelda Red/Heart palette (6=bright red/orange, 7=deep red)
            const LUT_MUSHROOM: [u8; 8] = [0, 1, 5, 7, 6, 6, 6, 7];
            // Gleaming Gold Coin & Question Block (2=yellow, 6=gold/orange, 3=shaded gold, 1=white shine, 5=dark outline)
            const LUT_GOLD: [u8; 8] = [0, 1, 5, 3, 2, 6, 6, 3];
            // Crisp Bob-omb (black body = 5, white eyes = 1, yellow fuse/feet = 2/6)
            const LUT_BOBOMB: [u8; 8] = [0, 1, 5, 6, 2, 1, 5, 5];

            // Overworld BG liftable rock -> crisp SMW '?' Question Block (1=white '?', 2=dark outline, 6/7=bright face, 3=shade)
            const LUT_QBLOCK_OW: [u8; 8] = [0, 1, 2, 3, 7, 6, 6, 3];

            if let (Some(g00), Some(g01), Some(g02), Some(g03), Some(g06)) = (
                load_smw_3bpp(0x00),
                load_smw_3bpp(0x01),
                load_smw_3bpp(0x02),
                load_smw_3bpp(0x03),
                load_smw_3bpp(0x06),
            ) {
                let blank16 = [[0u8; 16]; 16];
                let goomba1 = remap_16x16(&read_3bpp_16x16(&g01, 8, 2), &LUT_GOOMBA);
                let goomba2 = remap_16x16(&read_3bpp_16x16(&g01, 10, 2), &LUT_GOOMBA);
                let goomba1_r = flip_h_16x16(&goomba1);
                let goomba2_r = flip_h_16x16(&goomba2);
                // Classic two-tone chestnut-brown SMW Goomba on ALL Hyrule Soldiers (Green/Blue/Red S 0xBA):
                // 4=warm caramel tan top dome, 3=rich chestnut brown lower dome, 1=white eyes, 5=black brows, 6/7=rank-colored shoes!
                const LUT_GOOMBA_SOL: [u8; 8] = [0, 1, 5, 7, 6, 4, 4, 3];
                let goomba_sol1 = remap_16x16(&read_3bpp_16x16(&g01, 8, 2), &LUT_GOOMBA_SOL);
                let goomba_sol2 = remap_16x16(&read_3bpp_16x16(&g01, 10, 2), &LUT_GOOMBA_SOL);
                let goomba_sol1_r = flip_h_16x16(&goomba_sol1);
                let shell = remap_16x16(&read_3bpp_16x16(&g01, 10, 0), &LUT_STD);
                let bobomb = remap_16x16(&read_3bpp_16x16(&g02, 10, 4), &LUT_BOBOMB);
                let coin = remap_16x16(&read_3bpp_16x16(&g01, 8, 6), &LUT_GOLD);
                let smw_key = remap_16x16(&read_3bpp_16x16(&g01, 12, 6), &LUT_GOLD);
                let mushroom = remap_16x16(&read_3bpp_16x16(&g00, 4, 2), &LUT_MUSHROOM);
                let qblock = remap_16x16(&read_3bpp_16x16(&g00, 10, 2), &LUT_QBLOCK_OW);
                let piranha1 = remap_16x16(&read_3bpp_16x16(&g01, 12, 2), &LUT_STD);
                let piranha2 = remap_16x16(&read_3bpp_16x16(&g01, 14, 2), &LUT_STD);
                let spiny1 = remap_16x16(&read_3bpp_16x16(&g02, 0, 0), &LUT_STD);
                let spiny2 = remap_16x16(&read_3bpp_16x16(&g02, 2, 0), &LUT_STD);
                let magikoopa1 = remap_16x16(&read_3bpp_16x16(&g03, 0, 2), &LUT_STD);
                let magikoopa2 = remap_16x16(&read_3bpp_16x16(&g03, 2, 2), &LUT_STD);
                let boo1 = remap_16x16(&read_3bpp_16x16(&g06, 12, 0), &LUT_STD);
                let boo2 = remap_16x16(&read_3bpp_16x16(&g06, 14, 0), &LUT_STD);

                // Free-space cursor in verified 0xFF padding at PC 0x066F46..0x068000 ($0C:EF46, 4282 bytes)
                let mut free_cursor = 0x066F46usize;
                let free_end = 0x068000usize;

                // Helper to patch a compressed ALTTP 3bpp sheet safely (in-place if <= orig_len, else repoint into 0x066F46!)
                let mut patch_alttp_comp_sheet =
                    |rom_z: &mut SnesRom,
                     sess: &mut crate::swapper::SwapSession,
                     sheet_id: usize,
                     desc: &str,
                     edit_fn: &dyn Fn(&mut [u8])|
                     -> Result<bool, crate::rom::RomError> {
                        let pc = ((rom_z.data[z_bk + sheet_id] as usize & 0x7F) << 15)
                            | ((rom_z.data[z_hi + sheet_id] as usize & 0x7F) << 8)
                            | (rom_z.data[z_lo + sheet_id] as usize);
                        let Some((mut raw, orig_clen)) =
                            crate::disasm_dma::decompress_nintendo_lz2_with_endian(
                                &rom_z.data,
                                pc,
                                4096,
                                false,
                            )
                        else {
                            return Ok(false);
                        };
                        if raw.len() != 0x600 {
                            return Ok(false);
                        }
                        edit_fn(&mut raw);
                        let comp =
                            crate::disasm_dma::compress_nintendo_lz2_with_endian(&raw, false);
                        // Verify roundtrip decompression before writing a single byte!
                        let rt = crate::disasm_dma::decompress_nintendo_lz2_with_endian(
                            &comp, 0, 4096, false,
                        );
                        if rt.map(|(d, _)| d) != Some(raw) {
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
                                crate::swapper::SwapKind::GraphicsTiles {
                                    format: TileFormat::Bpp3,
                                    tile_count: 64,
                                    smart_remapped: true,
                                },
                            )?;
                            Ok(true)
                        } else if free_cursor + comp.len() <= free_end {
                            let dst_pc = free_cursor;
                            free_cursor += comp.len();
                            let snes_bank = (dst_pc / 0x8000) as u8;
                            let snes_addr = 0x8000 + (dst_pc % 0x8000);
                            let snes_hi = (snes_addr >> 8) as u8;
                            let snes_lo = (snes_addr & 0xFF) as u8;
                            sess.swap_custom_bytes(
                                rom_z,
                                dst_pc,
                                pc,
                                comp,
                                format!(
                                    "{} (repointé @ ${:02X}:{:04X})",
                                    desc, snes_bank, snes_addr
                                ),
                                crate::swapper::SwapKind::GraphicsTiles {
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
                                format!("Pointeur GFX Bank Sheet 0x{:02X}", sheet_id),
                                crate::swapper::SwapKind::CodePatch { byte_count: 1 },
                            )?;
                            sess.swap_custom_bytes(
                                rom_z,
                                z_hi + sheet_id,
                                z_hi + sheet_id,
                                vec![snes_hi],
                                format!("Pointeur GFX Hi Sheet 0x{:02X}", sheet_id),
                                crate::swapper::SwapKind::CodePatch { byte_count: 1 },
                            )?;
                            sess.swap_custom_bytes(
                                rom_z,
                                z_lo + sheet_id,
                                z_lo + sheet_id,
                                vec![snes_lo],
                                format!("Pointeur GFX Lo Sheet 0x{:02X}", sheet_id),
                                crate::swapper::SwapKind::CodePatch { byte_count: 1 },
                            )?;
                            Ok(true)
                        } else {
                            Ok(false)
                        }
                    };

                // FUSION 2A: ALL HYRULE CASTLE SWORD & SPEAR SOLDIERS (S 0xBA) -> 100% AUTHENTIC SMW GOOMBAS!
                // Architecture insight verified live in Mednafen: ALTTP Soldiers ALWAYS draw ONE single centered
                // 16x16 Head/Helmet OAM sprite from (0,0)=Front, (2,0)=Side, (4,0)=Back on EVERY frame (never doubled,
                // never blinking!), whereas the lower body cells (6..E,0 & 0..8,2) are stitched multi-part armor pieces.
                // Placing the full 16x16 chestnut-brown SMW Goomba into (0,0), (2,0), (4,0) and clearing the stitched
                // lower body cells produces ONE spotless, crisp, zero-artifact SMW Goomba on 100% of frames!
                if patch_alttp_comp_sheet(
                    rom_zelda,
                    session,
                    0xBA,
                    "Soldats Épée/Lance d'Hyrule (Sheet 0xBA) -> Vrais Goombas SMW (GFX01)",
                    &|raw| {
                        // Put 100% authentic 16x16 chestnut-brown SMW Goomba in the 3 single-OAM Head slots!
                        write_3bpp_16x16(raw, 0, 0, &goomba_sol1); // Front Goomba
                        write_3bpp_16x16(raw, 2, 0, &goomba_sol2); // Side Goomba
                        write_3bpp_16x16(raw, 4, 0, &goomba_sol1_r); // Back/Alt Goomba
                                                                     // Clear stitched multi-part lower armor cells so Goomba NEVER doubles or glitches!
                        write_3bpp_16x16(raw, 6, 0, &blank16);
                        write_3bpp_16x16(raw, 8, 0, &blank16);
                        write_3bpp_16x16(raw, 10, 0, &blank16);
                        write_3bpp_16x16(raw, 14, 0, &blank16);
                        write_3bpp_16x16(raw, 0, 2, &blank16);
                        write_3bpp_16x16(raw, 2, 2, &blank16);
                        write_3bpp_16x16(raw, 4, 2, &blank16);
                        write_3bpp_16x16(raw, 6, 2, &blank16);
                        write_3bpp_16x16(raw, 8, 2, &blank16);
                    },
                )? {
                    ops += 1;
                }

                // FUSION 2B: ELITE HORNED HYRULE KNIGHTS (S 0xB7) -> AUTHENTIC SMW SPINY KNIGHTS!
                if patch_alttp_comp_sheet(
                    rom_zelda,
                    session,
                    0xB7,
                    "Chevaliers d'Élite d'Hyrule (Sheet 0xB7) -> Vrais Spiny SMW (GFX02)",
                    &|raw| {
                        write_3bpp_16x16(raw, 0, 0, &spiny1);
                        write_3bpp_16x16(raw, 2, 0, &spiny2);
                        write_3bpp_16x16(raw, 4, 0, &spiny1);
                        write_3bpp_16x16(raw, 6, 0, &blank16);
                        write_3bpp_16x16(raw, 8, 0, &blank16);
                        write_3bpp_16x16(raw, 10, 0, &blank16);
                        write_3bpp_16x16(raw, 12, 0, &blank16);
                        write_3bpp_16x16(raw, 0, 2, &blank16);
                        write_3bpp_16x16(raw, 2, 2, &blank16);
                        write_3bpp_16x16(raw, 4, 2, &blank16);
                        write_3bpp_16x16(raw, 6, 2, &blank16);
                        write_3bpp_16x16(raw, 12, 2, &blank16);
                    },
                )? {
                    ops += 1;
                }

                // FUSION 2C: CASTLE GUARDS / ARCHERS (S 0x8C) -> AUTHENTIC SMW GOOMBAS + KOOPA SHELL SHIELDS!
                if patch_alttp_comp_sheet(
                    rom_zelda,
                    session,
                    0x8C,
                    "Soldats d'Hyrule (Sheet 0x8C) -> Vrais Goombas SMW (GFX01) + Boucliers Carapace Koopa",
                    &|raw| {
                        // Clear Soldier Helmets (8,0), (A,0), (C,0), (E,0) so Goomba isn't double-stacked!
                        write_3bpp_16x16(raw, 8, 0, &blank16);
                        write_3bpp_16x16(raw, 10, 0, &blank16);
                        write_3bpp_16x16(raw, 12, 0, &blank16);
                        write_3bpp_16x16(raw, 14, 0, &blank16);
                        // Replace all Soldier Walk/Chase bodies with crisp alternating SMW Goombas!
                        write_3bpp_16x16(raw, 0, 0, &goomba1);
                        write_3bpp_16x16(raw, 2, 0, &goomba2);
                        write_3bpp_16x16(raw, 4, 0, &goomba1);
                        write_3bpp_16x16(raw, 6, 0, &goomba1_r);
                        write_3bpp_16x16(raw, 0, 2, &goomba2);
                        write_3bpp_16x16(raw, 2, 2, &goomba2_r);
                        write_3bpp_16x16(raw, 4, 2, &goomba1);
                        write_3bpp_16x16(raw, 6, 2, &goomba2);
                        // Soldier Shield -> SMW Koopa Shell!
                        write_3bpp_16x16(raw, 10, 2, &shell);
                    },
                )? {
                    ops += 1;
                }

                // FUSION 3: CASTLE SEWER / DUNGEON ENEMIES (S 0x8D) -> SMW BOOS, SPINY & GOOMBAS!
                if patch_alttp_comp_sheet(
                    rom_zelda,
                    session,
                    0x8D,
                    "Égouts du Château (Sheet 0x8D) -> Boos SMW (Chauves-souris), Spiny (Rats) & Goombas (Serpents)",
                    &|raw| {
                        write_3bpp_16x16(raw, 0, 0, &boo1);
                        write_3bpp_16x16(raw, 2, 0, &boo2);
                        write_3bpp_16x16(raw, 4, 0, &boo1);
                        write_3bpp_16x16(raw, 8, 0, &spiny1);
                        write_3bpp_16x16(raw, 10, 0, &spiny2);
                        write_3bpp_16x16(raw, 12, 0, &spiny1);
                        write_3bpp_16x16(raw, 14, 0, &spiny2);
                        write_3bpp_16x16(raw, 2, 2, &spiny1);
                        write_3bpp_16x16(raw, 6, 2, &goomba1);
                        write_3bpp_16x16(raw, 8, 2, &goomba2);
                        write_3bpp_16x16(raw, 10, 2, &goomba1_r);
                    },
                )? {
                    ops += 1;
                }

                // FUSION 4: OVERWORLD ENEMIES (S 0x7F Octoroks + S 0x82 Buzzblobs/Tektites) -> SMW SPINY & PIRANHA PLANTS!
                if patch_alttp_comp_sheet(
                    rom_zelda,
                    session,
                    0x7F,
                    "Overworld Octoroks (Sheet 0x7F) -> Spiny SMW (GFX02)",
                    &|raw| {
                        write_3bpp_16x16(raw, 14, 0, &spiny1);
                    },
                )? {
                    ops += 1;
                }
                if patch_alttp_comp_sheet(
                    rom_zelda,
                    session,
                    0x82,
                    "Overworld Ennemis (Sheet 0x82) -> Plantes Piranha SMW (GFX01) & Spiny SMW (GFX02)",
                    &|raw| {
                        write_3bpp_16x16(raw, 10, 0, &piranha1);
                        write_3bpp_16x16(raw, 12, 0, &piranha2);
                        write_3bpp_16x16(raw, 14, 0, &piranha1);
                        write_3bpp_16x16(raw, 6, 2, &spiny1);
                        write_3bpp_16x16(raw, 8, 2, &spiny2);
                        write_3bpp_16x16(raw, 12, 2, &spiny1);
                    },
                )? {
                    ops += 1;
                }

                // FUSION 5: DUNGEON STALFOS SKELETONS (S 0x90) -> SMW BOOS & MAGIKOOPAS!
                if patch_alttp_comp_sheet(
                    rom_zelda,
                    session,
                    0x90,
                    "Squelettes Stalfos (Sheet 0x90) -> Boos SMW (GFX06) & Magikoopas SMW (GFX03)",
                    &|raw| {
                        write_3bpp_16x16(raw, 0, 0, &boo1);
                        write_3bpp_16x16(raw, 2, 0, &boo2);
                        write_3bpp_16x16(raw, 4, 0, &boo1);
                        write_3bpp_16x16(raw, 6, 0, &magikoopa1);
                        write_3bpp_16x16(raw, 8, 0, &magikoopa2);
                        write_3bpp_16x16(raw, 10, 0, &magikoopa1);
                        write_3bpp_16x16(raw, 12, 0, &magikoopa2);
                        write_3bpp_16x16(raw, 12, 2, &magikoopa1);
                    },
                )? {
                    ops += 1;
                }

                // FUSION 6: DROPPED RUPEES & HEART CONTAINERS (S 0xCD) -> SMW GOLD COINS & SUPER MUSHROOMS!
                if patch_alttp_comp_sheet(
                    rom_zelda,
                    session,
                    0xCD,
                    "Objets / Collectibles (Sheet 0xCD) -> Pièces d'or SMW (Rubis) & Super Champignons SMW (Cœurs)",
                    &|raw| {
                        write_3bpp_16x16(raw, 0, 0, &coin);
                        write_3bpp_16x16(raw, 2, 0, &coin);
                        write_3bpp_16x16(raw, 4, 0, &coin);
                        write_3bpp_16x16(raw, 0, 2, &mushroom);
                    },
                )? {
                    ops += 1;
                }

                // FUSION 7: UNCOMPRESSED SPRITES (S 0x72 @ 0x087600, S 0x7B @ 0x08AC00, S 0x76 @ 0x088E00)
                // -> SMW BOB-OMBS (Bombs), SMW KEYS, SMW GOOMBAS (Zols/Slimes), & HUD GOLD COIN / BOB-OMB!
                for &(sheet_pc, label) in &[
                    (
                        0x087600usize,
                        "Sprites 3bpp (Sheet 0x72) -> Bob-ombs SMW, Clés SMW & Goombas (Slimes)",
                    ),
                    (
                        0x08AC00usize,
                        "Sprites 3bpp (Sheet 0x7B) -> Bob-ombs SMW, Clés SMW & Goombas (Slimes)",
                    ),
                ] {
                    let mut raw = rom_zelda.data[sheet_pc..sheet_pc + 0x600].to_vec();
                    write_3bpp_16x16(&mut raw, 4, 2, &goomba1);
                    write_3bpp_16x16(&mut raw, 6, 2, &goomba2);
                    write_3bpp_16x16(&mut raw, 10, 2, &smw_key);
                    write_3bpp_16x16(&mut raw, 14, 2, &bobomb);
                    session.swap_custom_bytes(
                        rom_zelda,
                        sheet_pc,
                        sheet_pc,
                        raw,
                        label.to_string(),
                        crate::swapper::SwapKind::GraphicsTiles {
                            format: TileFormat::Bpp3,
                            tile_count: 64,
                            smart_remapped: true,
                        },
                    )?;
                    ops += 1;
                }
                {
                    let hud_pc = 0x088E00usize;
                    let mut raw = rom_zelda.data[hud_pc..hud_pc + 0x600].to_vec();
                    write_3bpp_16x16(&mut raw, 0, 2, &bobomb);
                    write_3bpp_16x16(&mut raw, 4, 2, &coin);
                    session.swap_custom_bytes(
                        rom_zelda,
                        hud_pc,
                        hud_pc,
                        raw,
                        "Icônes HUD 3bpp (Sheet 0x76) -> Pièce d'or SMW (Rubis) & Bob-omb SMW (Bombes)".to_string(),
                        crate::swapper::SwapKind::GraphicsTiles {
                            format: TileFormat::Bpp3,
                            tile_count: 64,
                            smart_remapped: true,
                        },
                    )?;
                    ops += 1;
                }

                // FUSION 8: ALL OVERWORLD LIFTABLE ROCKS (Sheets 0x06, 0x07, 0x08 cols C & E = Light & Dark Rocks!) -> SMW '?' QUESTION BLOCKS!
                for &ow_sheet in &[0x06usize, 0x07, 0x08] {
                    if patch_alttp_comp_sheet(
                        rom_zelda,
                        session,
                        ow_sheet,
                        &format!(
                            "Overworld Rochers (Sheet 0x{:02X}) -> Blocs '?' Question Block SMW (GFX00)",
                            ow_sheet
                        ),
                        &|raw| {
                            write_3bpp_16x16(raw, 12, 0, &qblock);
                            write_3bpp_16x16(raw, 14, 0, &qblock);
                        },
                    )? {
                        ops += 1;
                    }
                }
            }
        }
    }

    // 3. Brand-new internal SNES ROM Header title ("SUPER MARIO HYRULE   ")
    let title_bytes = b"SUPER MARIO HYRULE   ";
    session.swap_custom_bytes(
        rom_zelda,
        0x007FC0,
        0x007FC0,
        title_bytes.to_vec(),
        "En-tête Cartouche SNES (0x007FC0) : 'SUPER MARIO HYRULE'".to_string(),
        crate::swapper::SwapKind::CodePatch { byte_count: 21 },
    )?;
    ops += 1;

    let _ = write_alttp_link_16x16;

    rom_zelda.fix_checksum();
    Ok(ops)
}

fn smw_pose_to_link_idx(pose: usize) -> usize {
    match pose {
        0x00 | 0x08 | 0x32 => 0,               // Stand
        0x01 | 0x09 | 0x16 | 0x18 | 0x35 => 1, // Walk 1
        0x02 | 0x06 | 0x17 | 0x19 => 2,        // Walk 2
        0x04 | 0x1A | 0x1B => 3,               // Run 1
        0x05 | 0x0C | 0x1C => 4,               // Run 2
        0x0B | 0x03 | 0x20 | 0x29 => 5,        // Jump / Swim / Ride Yoshi
        0x0D | 0x0A | 0x0E | 0x21 => 6,        // Fall / Kick / Ride Yoshi 2
        0x0F | 0x10 | 0x1E | 0x22 | 0x3E => 7, // Front / Spin / Victory / Dying / Turn on Yoshi (0x1E)
        0x14 | 0x15 | 0x24 | 0x25 | 0x26 => 8, // Back / Net / Pipe
        0x1D | 0x38 => 0, // Ride Yoshi 1 & 2 (Stand side view, holding reins, facing right)
        0x39 => 3,        // Attack / Punch on Yoshi (Thrust arm forward)
        0x1F => 0,        // Duck on Yoshi
        _ => pose % 6,
    }
}

fn decode_alttp_link_16x16(
    rom_b_data: &[u8],
    mx: usize,
    my: usize,
    hflip: bool,
    cmap: &[u8; 16],
) -> [[u8; 16]; 16] {
    let base = 0x080000 + (my * 32 + mx * 2) * 32;
    let coords = [base, base + 32, base + 512, base + 544];
    let mut px = [[0u8; 16]; 16];
    for (q, &off) in coords.iter().enumerate() {
        if off + 32 > rom_b_data.len() {
            continue;
        }
        let qx = (q & 1) * 8;
        let qy = (q >> 1) * 8;
        let tile = &rom_b_data[off..off + 32];
        for y in 0..8usize {
            let b0 = tile[y * 2];
            let b1 = tile[y * 2 + 1];
            let b2 = tile[16 + y * 2];
            let b3 = tile[16 + y * 2 + 1];
            for x in 0..8usize {
                let sh = 7 - x;
                let c = ((b0 >> sh) & 1)
                    | (((b1 >> sh) & 1) << 1)
                    | (((b2 >> sh) & 1) << 2)
                    | (((b3 >> sh) & 1) << 3);
                px[qy + y][qx + x] = cmap[(c as usize) & 0x0F];
            }
        }
    }
    if hflip {
        for row in &mut px {
            row.reverse();
        }
    }
    px
}

fn make_small_link_16x16(
    rom_b_data: &[u8],
    head_xy: (usize, usize),
    body_xy: (usize, usize),
    hflip: bool,
    cmap: &[u8; 16],
) -> [[u8; 16]; 16] {
    let h = decode_alttp_link_16x16(rom_b_data, head_xy.0, head_xy.1, hflip, cmap);
    let b = decode_alttp_link_16x16(rom_b_data, body_xy.0, body_xy.1, hflip, cmap);
    let mut out = [[0u8; 16]; 16];
    // Le corps est tracé en premier à dy = 3 (épaules et jambes bien positionnées)
    for y in 0..13usize {
        let dy = y + 3;
        if dy < 16 {
            out[dy] = b[y];
        }
    }
    // La tête est tracée par-dessus (priorité absolue au visage et au menton)
    for y in 0..12usize {
        for x in 0..16usize {
            if h[y][x] != 0 {
                out[y][x] = h[y][x];
            }
        }
    }
    out
}

fn make_super_link_16x32(
    rom_b_data: &[u8],
    head_xy: (usize, usize),
    body_xy: (usize, usize),
    hflip: bool,
    cmap: &[u8; 16],
) -> ([[u8; 16]; 16], [[u8; 16]; 16]) {
    let h = decode_alttp_link_16x16(rom_b_data, head_xy.0, head_xy.1, hflip, cmap);
    let b = decode_alttp_link_16x16(rom_b_data, body_xy.0, body_xy.1, hflip, cmap);
    let mut canvas = [[0u8; 16]; 32];
    // 1. Corps positionné à y = 13..28 dans le canevas global 16x32 (épaules en 15..17, tunique 18..22, bottes 23..27)
    for y in 0..16usize {
        if 13 + y < 32 {
            canvas[13 + y] = b[y];
        }
    }
    // 2. Tête positionnée au-dessus à y = 2..15 (bonnet en 2..5, visage en 6..10, menton en 11..14)
    for y in 0..14usize {
        let cy = 2 + y;
        if cy < 32 {
            for x in 0..16usize {
                if h[y][x] != 0 {
                    canvas[cy][x] = h[y][x];
                }
            }
        }
    }
    let mut top = [[0u8; 16]; 16];
    let mut bot = [[0u8; 16]; 16];
    top.copy_from_slice(&canvas[0..16]);
    bot.copy_from_slice(&canvas[16..32]);
    (top, bot)
}

fn write_smw_gfx32_16x16(buf: &mut [u8], val: u8, px16: &[[u8; 16]; 16]) {
    // Replicates $00:F636 hardware DMA pointer formula into $7E:2000 (GFX32)
    let off = (((val as usize) & 0x08) << 11) | (((val as usize) & 0xF7) << 6);
    let coords = [off, off + 32, off + 512, off + 544];
    for (q, &dst) in coords.iter().enumerate() {
        if dst + 32 > buf.len() {
            continue;
        }
        let qx = (q & 1) * 8;
        let qy = (q >> 1) * 8;
        let mut tile = [0u8; 32];
        for y in 0..8usize {
            let mut b0 = 0u8;
            let mut b1 = 0u8;
            let mut b2 = 0u8;
            let mut b3 = 0u8;
            for x in 0..8usize {
                let c = px16[qy + y][qx + x] & 0x0F;
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
        buf[dst..dst + 32].copy_from_slice(&tile);
    }
}

fn write_smw_gfx10_3bpp_16x16(buf: &mut [u8], mx: usize, my: usize, px16: &[[u8; 16]; 16]) {
    let base = (my * 32 + mx * 2) * 24;
    let coords = [base, base + 24, base + 384, base + 408];
    for (q, &dst) in coords.iter().enumerate() {
        if dst + 24 > buf.len() {
            continue;
        }
        let qx = (q & 1) * 8;
        let qy = (q >> 1) * 8;
        let mut tile = [0u8; 24];
        for y in 0..8usize {
            let mut b0 = 0u8;
            let mut b1 = 0u8;
            let mut b2 = 0u8;
            for x in 0..8usize {
                let c = px16[qy + y][qx + x] & 0x07;
                let sh = 7 - x;
                b0 |= (c & 1) << sh;
                b1 |= ((c >> 1) & 1) << sh;
                b2 |= ((c >> 2) & 1) << sh;
            }
            tile[y * 2] = b0;
            tile[y * 2 + 1] = b1;
            tile[16 + y] = b2;
        }
        buf[dst..dst + 24].copy_from_slice(&tile);
    }
}

pub fn apply_link_world_title_screen(
    rom_a: &mut SnesRom,
    session: &mut crate::swapper::SwapSession,
) -> Result<usize, crate::rom::RomError> {
    let (mut gfx29, _) =
        crate::disasm_dma::decompress_nintendo_lz2_with_endian(&rom_a.data, 0x05C6A3, 8192, true)
            .ok_or(crate::rom::RomError::OutOfBounds {
            offset: 0x05C6A3,
            len: 8192,
        })?;

    let write_tile_29 = |t_idx: usize, tile: &[u8; 16], g29: &mut [u8]| {
        if t_idx >= 128 {
            let offset = (t_idx - 128) * 16;
            if offset + 16 <= g29.len() {
                g29[offset..offset + 16].copy_from_slice(tile);
            }
        }
    };

    // 1. Inject the 35 unique AI-synthesized 3D bubble letter tiles for 'LINK'
    // into safe MARIO-only slots in GFX29 (0 collisions with SUPER, 0 with WORLD)
    for &(slot, ref tile) in &crate::ai_sprites::TITLE_TILES_GFX29 {
        write_tile_29(slot, tile, &mut gfx29);
    }

    let comp29 = crate::disasm_dma::compress_nintendo_lz2_with_endian(&gfx29, true);
    let budget29 = 0x05CB7B - 0x05C6A3;
    if comp29.len() <= budget29 {
        let mut rep = comp29;
        rep.resize(budget29, 0xFF);
        session.swap_custom_bytes(
            rom_a,
            0x05C6A3,
            0x000000,
            rep,
            "Titre Écran d'accueil -> 'SUPER LINK WORLD' 3D Bubble Letters (GFX29)".to_string(),
            crate::swapper::SwapKind::GraphicsTiles {
                format: TileFormat::Bpp2,
                tile_count: 128,
                smart_remapped: false,
            },
        )?;
    }

    // 2. Patch Stripe Image 03: exactly 12 tiles (24 bytes) per row.
    // Replaces MARIO and leaves columns 12..24 (WORLD TM) 100% byte-for-byte vanilla and intact!
    let offsets: [usize; 5] = [
        0x02B5CD, // Row 0
        0x02B601, // Row 1
        0x02B637, // Row 2
        0x02B66D, // Row 3
        0x02B6A3, // Row 4
    ];

    for (r, &offset) in offsets.iter().enumerate() {
        let patch = crate::ai_sprites::TITLE_ROW_PATCHES[r].to_vec();
        session.swap_custom_bytes(
            rom_a,
            offset,
            offset,
            patch,
            format!("Stripe Image 03 Tilemap Row {} @ 0x{:06X}", r, offset),
            crate::swapper::SwapKind::GraphicsTiles {
                format: TileFormat::Bpp2,
                tile_count: 12,
                smart_remapped: false,
            },
        )?;
    }

    // 3. Replace In-Game HUD 'MARIO' header (GFX28 tiles 0x30..0x34 @ PC 0x05C06C) with crisp 'LINK '
    if let Some((mut gfx28, _)) =
        crate::disasm_dma::decompress_nintendo_lz2_with_endian(&rom_a.data, 0x05C06C, 8192, true)
    {
        for (i, tile) in crate::ai_sprites::HUD_LINK_TILES_GFX28.iter().enumerate() {
            let dst = (0x30 + i) * 16;
            if dst + 16 <= gfx28.len() {
                gfx28[dst..dst + 16].copy_from_slice(tile);
            }
        }
        let comp28 = crate::disasm_dma::compress_nintendo_lz2_with_endian(&gfx28, true);
        let budget28 = 0x05C6A3 - 0x05C06C;
        if comp28.len() <= budget28 {
            let mut rep28 = comp28;
            rep28.resize(budget28, 0xFF);
            session.swap_custom_bytes(
                rom_a,
                0x05C06C,
                0x05C06C,
                rep28,
                "HUD En Jeu (GFX28) : 'MARIO' -> 'LINK'".to_string(),
                crate::swapper::SwapKind::GraphicsTiles {
                    format: TileFormat::Bpp2,
                    tile_count: 5,
                    smart_remapped: false,
                },
            )?;
        }
    }

    // 4. Rename File Select & Erase Data 'MARIO A / B / C' -> 'LINK  A / B / C' (dynamically matched in Bank 05)
    const FILE_SEL_MARIO: [u8; 10] = [0x76, 0x31, 0x71, 0x31, 0x74, 0x31, 0x82, 0x30, 0x83, 0x30];
    const FILE_SEL_LINK: [u8; 10] = [0x15, 0x38, 0x12, 0x38, 0x17, 0x38, 0x14, 0x38, 0xFC, 0x38];
    let mut fs_hits = Vec::new();
    let fs_start = 0x02B6D0usize.min(rom_a.data.len());
    let fs_end = 0x02BA00usize.min(rom_a.data.len());
    if fs_end > fs_start + FILE_SEL_MARIO.len() {
        for off in fs_start..=(fs_end - FILE_SEL_MARIO.len()) {
            if rom_a.data[off..off + FILE_SEL_MARIO.len()] == FILE_SEL_MARIO {
                fs_hits.push(off);
            }
        }
    }
    for off in fs_hits {
        session.swap_custom_bytes(
            rom_a,
            off,
            off,
            FILE_SEL_LINK.to_vec(),
            format!("Sélection Fichier @ 0x{:06X} : 'MARIO' -> 'LINK '", off),
            crate::swapper::SwapKind::CodePatch { byte_count: 10 },
        )?;
    }

    // 5. Rewrite Intro Cutscene Message Box (141 bytes exact) for full narrative coherence
    //    (Link + Princess Zelda + Yoshi + Ganon in Hyrule Island!)
    let encode_smw_msg_line = |line: &str| -> Vec<u8> {
        let mut out: Vec<u8> = line
            .bytes()
            .map(|b| match b {
                b'A'..=b'Z' => b - b'A',
                b'a'..=b'z' => 0x40 + (b - b'a'),
                b'0'..=b'9' => 0x22 + (b - b'0'),
                b' ' => 0x1F,
                b'!' => 0x1A,
                b'.' => 0x1B,
                b'-' => 0x1C,
                b',' => 0x1D,
                b'?' => 0x1E,
                _ => 0x1F,
            })
            .collect();
        if let Some(last) = out.last_mut() {
            *last |= 0x80; // Bit 7 = SMW line-break / message-end marker
        }
        out
    };
    let intro_sig = encode_smw_msg_line("Welcome!   This is");
    if let Some(intro_off) = rom_a
        .data
        .windows(intro_sig.len())
        .position(|w| w == intro_sig.as_slice())
    {
        let lines = [
            "Welcome!   This is",
            "Hyrule  Island. In",
            "this strange  land",
            "we    find    that",
            "Princess  Zelda is",
            "missing  again! Go",
            "ride  green  Yoshi",
            "to  stop Ganon!",
        ];
        let mut msg_bytes = Vec::with_capacity(141);
        for l in &lines {
            msg_bytes.extend_from_slice(&encode_smw_msg_line(l));
        }
        if msg_bytes.len() == 141 && intro_off + 141 <= rom_a.data.len() {
            session.swap_custom_bytes(
                rom_a,
                intro_off,
                intro_off,
                msg_bytes,
                format!(
                    "Boîte d'Intro @ 0x{:06X} : Hyrule Island, Princesse Zelda & Ganon",
                    intro_off
                ),
                crate::swapper::SwapKind::CodePatch { byte_count: 141 },
            )?;
        }
    }

    // 6. Rename Overworld Region Prefix "YOSHI'S " -> "HYRULE  " (exact 8 bytes)
    const OW_YOSHIS: [u8; 8] = [0x18, 0x0E, 0x12, 0x07, 0x08, 0x5D, 0x12, 0x9F];
    const OW_HYRULE: [u8; 8] = [0x07, 0x18, 0x11, 0x14, 0x0B, 0x04, 0x1F, 0x9F];
    if let Some(ow_off) = rom_a
        .data
        .windows(OW_YOSHIS.len())
        .position(|w| w == OW_YOSHIS)
    {
        session.swap_custom_bytes(
            rom_a,
            ow_off,
            ow_off,
            OW_HYRULE.to_vec(),
            format!(
                "Nom Région Overworld @ 0x{:06X} : 'YOSHI'S' -> 'HYRULE'",
                ow_off
            ),
            crate::swapper::SwapKind::CodePatch { byte_count: 8 },
        )?;
    }

    Ok(2)
}
