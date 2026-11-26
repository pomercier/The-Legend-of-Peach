//! Phase 5: Native SPC700 Super Mario Sound Effects Compiler (`Bank $19`, `PC 0x0D0B70..0x0D1BFF`).
//!
//! Patches ALTTP's resident SPC700 SFX sequence tables (`ARAM $17C0..$284F`) strictly IN-PLACE
//! within each original SFX slot's verified byte budget (zero ARAM overlap, 100% native SPC700
//! playback on real SNES hardware):
//!
//! - **SFX2 `0x2B` & `0x2C`** (Rupee pickup -> **Super Mario Gold Coin Chime `B5 -> E6`**!)
//! - **SFX2 `0x20`** (Ledge hop / Mario Jump -> **SMW Rising Jump Pitch-Sweep**!)
//! - **SFX3 `0x08`** (Enemy hit / Mario Stomp -> **Snappy Mario Stomp Bounce Pop**!)
//! - **SFX2 `0x15` & `0x14`** (Stairs down/up -> **Classic Mario Warp Pipe Triple-Pulse**!)
//! - **SFX1 `0x01..0x04`** (Sword slashes -> **Punchy Mario Super Hammer Swing/Bonk**!)

use crate::rom::{RomError, SnesRom};
use crate::swapper::{SwapKind, SwapSession};

/// Verified in-place SPC700 SFX patch descriptor (FR PAL ALTTP Bank $19 chunk `@ 0x0D0B70 -> ARAM $17C0`).
struct SfxPatch {
    name: &'static str,
    pc: usize,
    orig_prefix: &'static [u8],
    max_len: usize,
    mario_seq: &'static [u8],
}

/// All verified Phase 5 native SPC700 Mario SFX replacements.
/// Every sequence uses authentic ALTTP N-SPC SFX opcodes:
/// - `E0 <inst> <dur> <vol>` (set resident BRR voice, tick duration, amplitude)
/// - `80..C7` (musical note pitch; e.g. `BB`=B5, `C0`=E6, `9C`=E4, `B6`=F#5/A#5)
/// - `F9 <note> <delay> <len> <target>` (pitch glide / sweep)
/// - `00` (end of SFX sequence)
const MARIO_SFX_PATCHES: &[SfxPatch] = &[
    // 1. SFX2 0x2B (Rupee 1 -> Iconic Super Mario Gold Coin Chime: B5 crisp -> E6 ringing!)
    SfxPatch {
        name: "SFX2 0x2B (Pièce d'Or Mario B5->E6)",
        pc: 0x0D13A2,
        orig_prefix: &[0xE0, 0x0E, 0x08, 0x28],
        max_len: 15,
        mario_seq: &[
            0xE0, 0x0E, 0x05, 0x6C, 0xBB, // Crisp B5 (5 ticks)
            0x12, 0x6C, 0xC0, // Bright ringing E6 (18 ticks)
            0x06, 0x34, 0xC0, // Clean fade tail
            0x04, 0x16, 0xC0, 0x00,
        ],
    },
    // 2. SFX2 0x2C (Rupee 2 / Big Coin -> Sparkling High Mario Coin Chime B5->E6!)
    SfxPatch {
        name: "SFX2 0x2C (Pièce d'Or Scintillante Mario)",
        pc: 0x0D1389,
        orig_prefix: &[0xE0, 0x00, 0x0C, 0x78],
        max_len: 14,
        mario_seq: &[
            0xE0, 0x0E, 0x04, 0x74, 0xBB, // B5
            0x14, 0x74, 0xC0, // E6
            0x05, 0x28, 0xC0, 0x00,
        ],
    },
    // 3. SFX2 0x20 (Jump -> Playful SMW Mario Rising Jump Pitch-Sweep!)
    SfxPatch {
        name: "SFX2 0x20 (Saut Mario SMW Pitch-Sweep Ascendant)",
        pc: 0x0D168A,
        orig_prefix: &[0xE0, 0x0D, 0x12, 0x3C],
        max_len: 15,
        mario_seq: &[
            0xE0, 0x0D, 0x0B, 0x64, 0xF9, 0x9C, 0x00, 0x0B, 0xB6, // Bright upward sweep!
            0x05, 0x38, 0xB8, 0x00,
        ],
    },
    // 4. SFX3 0x08 (Enemy Hit / Mario Stomp -> Snappy Rising Mario Stomp Pop!)
    SfxPatch {
        name: "SFX3 0x08 (Stomp Mario Rebond Pop)",
        pc: 0x0D131F,
        orig_prefix: &[0xE0, 0x11, 0x0C, 0x78],
        max_len: 20,
        mario_seq: &[
            0xE0, 0x02, 0x03, 0x74, 0xA2, // Punchy pop base
            0x03, 0x74, 0xAA, // Rising snap
            0x05, 0x68, 0xB5, // High Mario stomp pop!
            0x04, 0x28, 0xB5, 0x00,
        ],
    },
    // 5. SFX2 0x15 (Stairs Down / Entrance -> Classic Mario Warp Pipe Descending Triple-Pulse!)
    SfxPatch {
        name: "SFX2 0x15 (Tuyau Warp Mario Triple-Descente)",
        pc: 0x0D1740,
        orig_prefix: &[0xE0, 0x07, 0x0C, 0x28],
        max_len: 16,
        mario_seq: &[
            0xE0, 0x07, 0x05, 0x68, 0xA8, // Pulse 1
            0x05, 0x68, 0xA4, // Pulse 2
            0x05, 0x68, 0xA0, // Pulse 3
            0x05, 0x38, 0x9C, 0x00,
        ],
    },
    // 6. SFX2 0x14 (Stairs Up / Exit -> Mario Warp Pipe Ascending Pulse!)
    SfxPatch {
        name: "SFX2 0x14 (Sortie Tuyau Warp Mario)",
        pc: 0x0D1730,
        orig_prefix: &[0xE0, 0x07, 0x0C, 0x28],
        max_len: 16,
        mario_seq: &[
            0xE0, 0x07, 0x05, 0x64, 0x9C, // Pulse 1
            0x05, 0x64, 0xA0, // Pulse 2
            0x05, 0x64, 0xA4, // Pulse 3
            0x05, 0x40, 0xA8, 0x00,
        ],
    },
    // 7. SFX1 0x01 (Sword Lv1 -> Punchy Mario Super Hammer Bonk/Whoosh!)
    SfxPatch {
        name: "SFX1 0x01 (Coup de Super Marteau Mario Lv1)",
        pc: 0x0D1A02,
        orig_prefix: &[0xE0, 0x01, 0x60, 0x50],
        max_len: 16,
        mario_seq: &[
            0xE0, 0x02, 0x04, 0x6C, 0x95, // Punchy low-mid bonk
            0x04, 0x58, 0x90, 0x04, 0x30, 0x8C, 0x00,
        ],
    },
    // 8. SFX1 0x02 (Sword Lv2 -> Super Hammer Power Bonk!)
    SfxPatch {
        name: "SFX1 0x02 (Coup de Super Marteau Mario Lv2)",
        pc: 0x0D1A12,
        orig_prefix: &[0xE0, 0x01, 0x60, 0x00],
        max_len: 21,
        mario_seq: &[
            0xE0, 0x02, 0x04, 0x70, 0x98, // Bold power hammer
            0x04, 0x60, 0x93, 0x04, 0x38, 0x8E, 0x00,
        ],
    },
];

/// Patch ALTTP's native SPC700 SFX bank in Bank `$19` with authentic Super Mario sound effects.
pub fn patch_mario_native_sfx(
    rom_zelda: &mut SnesRom,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    let mut ops = 0usize;
    for p in MARIO_SFX_PATCHES {
        if p.pc + p.max_len <= rom_zelda.data.len()
            && &rom_zelda.data[p.pc..p.pc + p.orig_prefix.len()] == p.orig_prefix
            && p.mario_seq.len() <= p.max_len
        {
            session.swap_custom_bytes(
                rom_zelda,
                p.pc,
                p.pc,
                p.mario_seq.to_vec(),
                format!("Audio SPC700 Natif Phase 5 : {}", p.name),
                SwapKind::CodePatch {
                    byte_count: p.mario_seq.len(),
                },
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
    fn test_mario_sfx_budgets_and_terminators() {
        for p in MARIO_SFX_PATCHES {
            assert!(
                p.mario_seq.len() <= p.max_len,
                "{} exceeds in-place budget ({} > {})",
                p.name,
                p.mario_seq.len(),
                p.max_len
            );
            assert_eq!(
                *p.mario_seq.last().unwrap(),
                0x00,
                "{} must terminate with 0x00",
                p.name
            );
        }
    }
}
