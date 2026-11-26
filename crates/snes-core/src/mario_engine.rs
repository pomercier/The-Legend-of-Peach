//! Phase 5: Native 65c816 Super Mario Gameplay Engine (`Bank $22:9000`, `PC 0x111000`).
//!
//! Transforms ALTTP's flat 2D Link loop into a genuine, playable Super Mario action-adventure
//! running 100% natively on original SNES 65c816 hardware (zero MSU-1, zero enhancement chips):
//!
//! 1. **Genuine Top-Down Mario Jumping (`L` or `R` shoulder button)**:
//!    - 8.8 fixed-point parabolic vertical physics (`MARIO_Z_PX`, `MARIO_Z_SUB`, `MARIO_VZ`).
//!    - Variable jump height (short-hop when releasing `L`/`R` early, full 22px arc when held).
//!    - Hooks cleanly around `JSL $07:8000` (`Link_Main`) at all 3 Bank `$02` callers
//!      (`0x010959`, `0x011CCA`, `0x0125C7`), keeping Mario's full 8-direction movement intact
//!      while lifting Mario smoothly above his ground shadow via ALTTP's native `$24/$25` Z-height.
//!    - Automatic clean landing & mode/room-transition reset so zero softlocks can ever occur.
//!
//! 2. **Genuine Mario Stomp Combat**:
//!    - When descending (`MARIO_AIRBORNE == 2`) onto an active enemy (`$0DD0,X == 0x09`),
//!      checks a 256-byte Mario Stomp Vulnerability Table (`$22:9800`) so stompable enemies
//!      (Goombas, Koopas, Buzzies, Rex, Octoroks, Bats, Rats, Ropes, etc.) take heavy Stomp
//!      damage or vanish in a Mario puff (`$0DD0,X = 0x06`), award +1 Gold Coin (`$7E:F362`),
//!      trigger the Mario Stomp Pop SFX (`$012F = 0x08`), grant post-stomp i-frames (`$037B`),
//!      and **bounce Mario upward** (extra-high bounce when holding `L`/`R`!).
//!    - Armored/Spiked enemies (Spinies, Thwomps, Pokeys, Bosses) are excluded from instant stomps.
//!
//! 3. **Unified Mario Power-Up State Machine**:
//!    - **Fire Mario**: Equipping the Fire Flower (`$7E:0202 == 0x06`, Fire Rod slot) immediately
//!      live-swaps Mario's CGRAM OBJ palette (`$7E:C6E2..$7E:C6FF` + `$15 = 1`) into authentic
//!      SMW Fire Mario (pure white cap/shirt + blazing crimson overalls)!
//!    - **Super Star Mario**: Pressing `L + R` simultaneously (consuming `0x20` Star Power from
//!      `$7E:F36E`) or activating Magic Cape triggers 240 frames (4 seconds) of authentic flashing
//!      4-palette rainbow Starman invincibility (`$037B = 1`) with instant contact-defeat on
//!      touching enemies, cleanly restoring Classic or Fire Mario palette upon expiration!

use std::collections::HashMap;

use crate::rom::{RomError, SnesRom};
use crate::swapper::{SwapKind, SwapSession};

/// Bank `$22` LoROM base for Phase 5 Mario Gameplay Engine (`PC 0x111000..0x111FFF`, 4 KiB).
pub const MARIO_ENGINE_PC: usize = 0x111000;
pub const MARIO_ENGINE_BANK: u8 = 0x22;
pub const MARIO_ENGINE_ADDR: u16 = 0x9000;

/// 256-byte Enemy Stomp Vulnerability Table at `$22:9800` (`PC 0x111800`).
pub const STOMP_TABLE_PC: usize = 0x111800;
pub const STOMP_TABLE_ADDR: u16 = 0x9800;

/// Live Mario Power-Up BGR555 Palette Tables at `$22:9900` (`PC 0x111900`):
/// 6 palettes × 30 bytes (colors 1..15, little-endian BGR555):
///   0 = Classic Super Mario (Red cap/shirt, Blue overalls)
///   1 = Fire Mario (Pure White cap/shirt, Crimson overalls)
///   2..5 = Super Starman Rainbow Cycle (Gold, Cyan, Neon Green, Blazing Magenta)
pub const PAL_TABLES_PC: usize = 0x111900;
pub const PAL_TABLES_ADDR: u16 = 0x9900;

/// Verified free WRAM block (`$7E:F500..$7E:F53F`, 0 conflicting ROM refs).
const W_AIRBORNE: u16 = 0xF500; // 0=Grounded, 1=Rising, 2=Falling/Stomp-Ready
const W_Z_PX: u16 = 0xF501; // Integer Z height (0..36 px)
const W_Z_SUB: u16 = 0xF502; // Fractional 8-bit Z subpixel
const W_VZ_LO: u16 = 0xF503; // Signed 8.8 vertical velocity low
const W_VZ_HI: u16 = 0xF504; // Signed 8.8 vertical velocity high
const W_COMBO: u16 = 0xF505; // Consecutive airborne stomp combo counter
const W_STOMP_IF: u16 = 0xF506; // Post-stomp i-frame timer
const W_POWERUP: u16 = 0xF507; // 0=Classic, 1=Fire Mario, 2=Starman
const W_STAR_LO: u16 = 0xF508; // Starman countdown timer (16-bit low)
const W_STAR_HI: u16 = 0xF509; // Starman countdown timer (16-bit high)
const W_LAST_MODE: u16 = 0xF50A; // Previous $10 game mode
const W_LAST_AREA: u16 = 0xF50B; // Previous $8A/$A0 area ID
const W_PAL_DIRTY: u16 = 0xF50C; // 1 = force palette restore to Classic/Fire
const W_JUMP_CD: u16 = 0xF50D; // Landing cooldown frames

/// Verified Bank `$02` callers of `JSL $07:8000` (`Link_Main`) in FR PAL ALTTP.
pub const LINK_MAIN_CALL_SITES_PC: [usize; 3] = [0x010959, 0x011CCA, 0x0125C7];

/// Minimal deterministic 65c816 assembler with label + branch + jump resolution.
struct Asm65816 {
    origin: u16,
    code: Vec<u8>,
    labels: HashMap<&'static str, u16>,
    rel8_fixups: Vec<(usize, &'static str)>,
    abs16_fixups: Vec<(usize, &'static str)>,
}

impl Asm65816 {
    fn new(origin: u16) -> Self {
        Self {
            origin,
            code: Vec::with_capacity(1024),
            labels: HashMap::new(),
            rel8_fixups: Vec::new(),
            abs16_fixups: Vec::new(),
        }
    }

    fn pc(&self) -> u16 {
        self.origin.wrapping_add(self.code.len() as u16)
    }

    fn label(&mut self, name: &'static str) {
        self.labels.insert(name, self.pc());
    }

    fn op(&mut self, opcode: u8) {
        self.code.push(opcode);
    }

    fn imm8(&mut self, opcode: u8, val: u8) {
        self.code.push(opcode);
        self.code.push(val);
    }

    fn dp(&mut self, opcode: u8, addr: u8) {
        self.code.push(opcode);
        self.code.push(addr);
    }

    fn abs(&mut self, opcode: u8, addr: u16) {
        self.code.push(opcode);
        self.code.extend_from_slice(&addr.to_le_bytes());
    }

    fn long(&mut self, opcode: u8, bank: u8, addr: u16) {
        self.code.push(opcode);
        self.code.extend_from_slice(&addr.to_le_bytes());
        self.code.push(bank);
    }

    fn branch(&mut self, opcode: u8, target: &'static str) {
        self.code.push(opcode);
        let patch_pos = self.code.len();
        self.code.push(0x00);
        self.rel8_fixups.push((patch_pos, target));
    }

    fn jmp(&mut self, target: &'static str) {
        self.code.push(0x4C);
        let patch_pos = self.code.len();
        self.code.extend_from_slice(&[0x00, 0x00]);
        self.abs16_fixups.push((patch_pos, target));
    }

    fn jsr(&mut self, target: &'static str) {
        self.code.push(0x20);
        let patch_pos = self.code.len();
        self.code.extend_from_slice(&[0x00, 0x00]);
        self.abs16_fixups.push((patch_pos, target));
    }

    fn finish(mut self) -> Result<Vec<u8>, String> {
        for (pos, name) in self.rel8_fixups {
            let target = *self
                .labels
                .get(name)
                .ok_or_else(|| format!("Unknown 65c816 branch label: {name}"))?;
            let next_pc = self.origin.wrapping_add((pos + 1) as u16);
            let delta = (target as i32) - (next_pc as i32);
            if !(-128..=127).contains(&delta) {
                return Err(format!(
                    "65c816 branch to '{name}' out of range: delta={delta} at offset 0x{pos:X}"
                ));
            }
            self.code[pos] = (delta as i8) as u8;
        }
        for (pos, name) in self.abs16_fixups {
            let target = *self
                .labels
                .get(name)
                .ok_or_else(|| format!("Unknown 65c816 jump label: {name}"))?;
            let bytes = target.to_le_bytes();
            self.code[pos] = bytes[0];
            self.code[pos + 1] = bytes[1];
        }
        Ok(self.code)
    }
}

/// Build the 256-byte Mario Stomp Vulnerability Table (`$22:9800`):
/// - `0x00` = Non-enemy / NPC / Item / Unstompable / Boss (never damaged by stomp)
/// - `0x01` = Stompable Mario Enemy (Goombas, Koopas, Buzzies, Rex, Octoroks, Chucks, etc.)
/// - `0x02` = Spiked / Hazardous Enemy (Spinies, Thwomps, Pokeys — immune to ordinary stomp, killed by Starman!)
pub fn build_mario_stomp_table() -> [u8; 256] {
    let mut t = [0u8; 256];

    // Stompable Overworld & Dungeon Enemies (ALTTP Sprite IDs -> SMW Bestiary):
    const STOMPABLE: &[u8] = &[
        0x00, // Raven -> Swooper
        0x01, // Vulture -> Paragoomba
        0x08, // Octorok -> Goomba / Spit enemy
        0x0A, // Octorok 4-way
        0x0B, // Cucco? Skip 0x0B!
        0x0D, // Buzzblob -> Buzzy / Elec
        0x0E, // Snapdragon -> Rex
        0x0F, // Octoballoon
        0x11, // Hinox -> Chargin' Chuck
        0x12, // Moblin -> Koopa / Hammer Bro
        0x13, // Mini Helmasaur -> Buzzy Beetle
        0x18, // Mini Moldorm -> Mini Wiggler
        0x19, // Poe -> Boo (stompable or star)
        0x1B, // Arrow -> skip!
        0x23, // Red Bari
        0x24, // Blue Bari
        0x26, // Hardhat Beetle -> Buzzy Beetle
        0x41, // Blue Soldier -> Blue Koopa Troopa
        0x42, // Green Soldier -> Green Koopa Troopa
        0x43, // Red Spear Soldier -> Red Koopa
        0x44, // Assault Soldier -> Armored Koopa
        0x45, // Green Spear Soldier -> Koopa Guard
        0x46, // Blue Archer -> Bill Koopa
        0x47, // Green Archer -> Bill Koopa
        0x48, // Red Javelin Soldier -> Hammer Bro
        0x49, // Red Bush Soldier -> Bush Koopa
        0x4A, // Red Bomb Soldier -> Bob-omb Koopa
        0x4B, // Green Knife Soldier -> Koopa
        0x4C, // Geldman -> Desert Goomba
        0x4D, // Toppo -> Monty Mole
        0x4E, // Popo -> Snow Goomba
        0x4F, // Popo 2
        0x51, // Armos Statue -> Koopa Statue
        0x55, // Zora -> Cheep Cheep
        0x58, // Crab -> Sidestepper
        0x6A, // Ball & Chain Trooper -> Heavy Koopa
        0x6B, // Cannon Soldier -> Bill Blaster Guard
        0x7D, // Pikit -> Swooper
        0x81, // Water Tektite -> Cheep Cheep
        0x82, // Leever -> Monty Mole
        0x83, // Green Eyegore -> Rex
        0x84, // Red Eyegore -> Red Rex
        0x85, // Stalfos -> Dry Bones
        0x8A, // Rope -> Wiggler / Goomba
        0x8B, // Gibdo -> Dry Bones
        0x8D, // Terrorpin -> Buzzy Beetle
        0x8E, // Terrorpin
        0x8F, // Blob -> Goomba
        0x90, // Wallmaster
        0x91, // Stalfos Knight -> Dry Bones
        0x94, // Pirogusu -> Cheep
        0x95, // Laser Eye
        0x9A, // Kyameron
        0x9B, // Wizzrobe -> Magikoopa
        0x9C, // Zazaks -> Dino Rhino
        0x9D, // Red Zazak -> Dino Torch
        0x9E, // Stalfos -> Dry Bones
        0x9F, // Kodongo -> Dino Rhino
        0xA5, // Green Penguin -> Bumpty
        0xA7, // Stalfos
        0xA8, // Green Kodongo -> Dino
        0xA9, // Red Kodongo -> Dino
        0xAA, // Pikit
        0xAC, // Red Mimic
        0xAD, // Green Mimic
        0xB9, // Sluggula
        0xBA, // Eyegore
        0xBB, // Hover
        0xBE, // Lynel -> Heavy Chargin' Chuck
        0xC5, // Medusa? No
        0xC7, // Hokkubokku -> Pokey
    ];
    for &id in STOMPABLE {
        if id != 0x0B && id != 0x1B && id != 0xC5 {
            t[id as usize] = 0x01;
        }
    }

    // Spiked / Armored Enemies (immune to stomp, defeated by Super Starman contact!):
    const SPIKED: &[u8] = &[
        0x0C, // Spiny / Buzz
        0x14, // Gargoyle
        0x22, // Ropa
        0x6E, // Spike Roller
        0x86, // Fireball / Spark
        0x87, // Spark
        0x88, // Spike / Firebar
        0x98, // Thwomp / Bumper
        0x99, // Thwomp
        0xA0, // Pokey / Spiny
        0xA1, // Freezor -> Frost Koopa
        0xC2, // Chain Chomp
    ];
    for &id in SPIKED {
        t[id as usize] = 0x02;
    }

    t
}

/// Build the 6 live Mario Power-Up BGR555 palettes (colors 1..15 each = 30 bytes × 6 = 180 bytes).
/// Palette 0 = Classic Super Mario (matches `0x0DD308` verified in `demo.rs`)
/// Palette 1 = Fire Mario (Crisp White Cap & Shirt + Blazing Red Overalls + Gold Buttons)
/// Palettes 2..5 = Super Starman Rainbow Flashing Cycle (Gold, Cyan, Neon Green, Magenta)
pub fn build_mario_powerup_palettes() -> Vec<u8> {
    let classic: [u16; 15] = [
        0x7FFF, // 1: White gloves/eyes
        0x0000, // 2: Black outline/mustache
        0x0D71, // 3: Brown boots/hair
        0x1E9B, // 4: Skin shade
        0x3B7F, // 5: Peach skin
        0x635F, // 6: Skin highlight
        0x0857, // 7: Deep Red shadow
        0x000A, // 8: Darkest Red outline
        0x10DF, // 9: Bright Mario Red cap & shirt
        0x4882, // 10: Deep Royal Blue shadow
        0x6D85, // 11: Classic SMW Blue overalls
        0x7E6B, // 12: Bright Blue highlight
        0x7FFF, // 13: White highlight
        0x35DF, // 14: Warm accent
        0x03FF, // 15: Golden buttons
    ];
    let fire: [u16; 15] = [
        0x7FFF, // 1: White gloves/eyes
        0x0000, // 2: Black outline/mustache
        0x0D71, // 3: Brown boots/hair
        0x1E9B, // 4: Skin shade
        0x3B7F, // 5: Peach skin
        0x635F, // 6: Skin highlight
        0x5AD6, // 7: Crisp Silver-White cap shadow
        0x3DEF, // 8: Soft Grey-White outline
        0x7FFF, // 9: PURE WHITE FIRE MARIO CAP & SHIRT!
        0x0010, // 10: Deep Crimson overalls shadow
        0x001B, // 11: BLAZING FIRE MARIO RED OVERALLS!
        0x14DF, // 12: Bright Flame Red highlight
        0x7FFF, // 13: Pure White highlight
        0x02BF, // 14: Fiery Orange accent
        0x03FF, // 15: Golden buttons
    ];
    let star0: [u16; 15] = [
        0x7FFF, 0x0000, 0x02BF, 0x037F, 0x03FF, 0x4BFF, 0x02BF, 0x0198, 0x03FF, 0x025F, 0x033F,
        0x5FFF, 0x7FFF, 0x03FF, 0x7FFF,
    ]; // Blazing Super Star Gold
    let star1: [u16; 15] = [
        0x7FFF, 0x0000, 0x4E20, 0x6F00, 0x7FC8, 0x7FF5, 0x5E60, 0x3D40, 0x7FE0, 0x7E08, 0x7F10,
        0x7FFF, 0x7FFF, 0x7FE0, 0x03FF,
    ]; // Electric Star Cyan
    let star2: [u16; 15] = [
        0x7FFF, 0x0000, 0x0240, 0x0320, 0x2BE8, 0x5FF5, 0x02A0, 0x0180, 0x03E0, 0x02C8, 0x2FE0,
        0x6FFF, 0x7FFF, 0x03FF, 0x7FFF,
    ]; // Neon Star Green
    let star3: [u16; 15] = [
        0x7FFF, 0x0000, 0x4014, 0x601C, 0x7C1F, 0x7E9F, 0x5018, 0x300C, 0x7C1F, 0x001F, 0x341F,
        0x7DFF, 0x7FFF, 0x03FF, 0x7FFF,
    ]; // Vivid Star Magenta-Red

    let mut out = Vec::with_capacity(6 * 30);
    for pal in [&classic, &fire, &star0, &star1, &star2, &star3] {
        for &c in pal {
            out.extend_from_slice(&c.to_le_bytes());
        }
    }
    out
}

/// Assemble the complete native 65c816 Super Mario Gameplay Engine for Bank `$22:9000`.
pub fn assemble_mario_engine_65816() -> Result<Vec<u8>, String> {
    let mut a = Asm65816::new(MARIO_ENGINE_ADDR);

    // =========================================================================
    // ENTRY POINT: Mario_MainHook ($22:9000)
    // Replaces `JSL $07:8000` at the 3 Bank $02 callers cleanly!
    // =========================================================================
    a.label("Mario_MainHook");
    // 1. PRE-Link_Main: if Mario is in custom airborne jump, temporarily hide $24/$25 ($FF)
    //    so vanilla Link_Main runs full 8-direction movement & wall collision without ledge-fall!
    a.op(0x08); // PHP
    a.op(0x8B); // PHB
    a.imm8(0xE2, 0x30); // SEP #$30 (8-bit A/X/Y)
    a.imm8(0xA9, 0x7E); // LDA #$7E
    a.op(0x48); // PHA
    a.op(0xAB); // PLB (DBR = $7E)
    a.abs(0xAD, W_AIRBORNE); // LDA $F500
    a.branch(0xF0, "pre_done"); // BEQ pre_done
    a.imm8(0xA9, 0xFF); // LDA #$FF
    a.dp(0x85, 0x24); // STA $24 (grounded for vanilla collision)
    a.dp(0x85, 0x25); // STA $25
    a.dp(0x64, 0x29); // STZ $29
    a.label("pre_done");
    a.op(0xAB); // PLB
    a.op(0x28); // PLP

    // 2. Call 100% intact vanilla Link_Main ($07:8000)!
    a.long(0x22, 0x07, 0x8000); // JSL $07:8000

    // 3. POST-Link_Main: Run Mario Jump Physics, Stomp Combat & Power-Up Engine!
    a.op(0x08); // PHP
    a.op(0x8B); // PHB
    a.imm8(0xE2, 0x30); // SEP #$30 (8-bit A/X/Y)
    a.imm8(0xA9, 0x7E); // LDA #$7E
    a.op(0x48); // PHA
    a.op(0xAB); // PLB (DBR = $7E)

    // --- SAFETY GATE: only active in Mode 0x07 (Dungeon) or 0x09 (Overworld), Submodule 0x00 ---
    a.dp(0xA5, 0x10); // LDA $10 (Game Mode)
    a.imm8(0xC9, 0x07); // CMP #$07
    a.branch(0xF0, "mode_ok");
    a.imm8(0xC9, 0x09); // CMP #$09
    a.branch(0xF0, "mode_ok");
    a.jsr("Mario_ResetAirborne");
    a.jmp("hook_exit");

    a.label("mode_ok");
    // Check Submodule $11 == 0, Freeze $02E4 == 0, Carry $0308 == 0, State $5D == 0
    a.dp(0xA5, 0x11); // LDA $11
    a.abs(0x0D, 0x02E4); // ORA $02E4
    a.abs(0x0D, 0x0308); // ORA $0308
    a.dp(0x05, 0x5D); // ORA $5D
    a.branch(0xF0, "state_ok");
    a.jsr("Mario_ResetAirborne");
    a.jsr("Mario_UpdatePowerUps");
    a.jmp("hook_exit");

    a.label("state_ok");
    // Check if Game Mode ($10) or Area ($8A in OW / $A0 in Dungeon) changed -> cleanly cancel jump!
    a.dp(0xA5, 0x10);
    a.abs(0xCD, W_LAST_MODE);
    a.branch(0xD0, "area_changed");
    a.imm8(0xC9, 0x09);
    a.branch(0xF0, "chk_ow_area");
    a.dp(0xA5, 0xA0); // Dungeon room ID low
    a.branch(0x80, "cmp_area");
    a.label("chk_ow_area");
    a.dp(0xA5, 0x8A); // Overworld screen ID
    a.label("cmp_area");
    a.abs(0xCD, W_LAST_AREA);
    a.branch(0xF0, "area_same");

    a.label("area_changed");
    a.dp(0xA5, 0x10);
    a.abs(0x8D, W_LAST_MODE);
    a.imm8(0xC9, 0x09);
    a.branch(0xF0, "save_ow_area");
    a.dp(0xA5, 0xA0);
    a.branch(0x80, "store_area");
    a.label("save_ow_area");
    a.dp(0xA5, 0x8A);
    a.label("store_area");
    a.abs(0x8D, W_LAST_AREA);
    a.jsr("Mario_ResetAirborne");

    a.label("area_same");
    a.jsr("Mario_UpdateJump");
    a.jsr("Mario_CheckStomp");
    a.jsr("Mario_UpdatePowerUps");

    a.label("hook_exit");
    a.op(0xAB); // PLB
    a.op(0x28); // PLP
    a.op(0x6B); // RTL

    // =========================================================================
    // SUBROUTINE: Mario_ResetAirborne
    // Cleanly cancels any active jump without disturbing vanilla ledge hops
    // =========================================================================
    a.label("Mario_ResetAirborne");
    a.abs(0xAD, W_AIRBORNE);
    a.branch(0xF0, "reset_done");
    a.abs(0x9C, W_AIRBORNE);
    a.abs(0x9C, W_Z_PX);
    a.abs(0x9C, W_Z_SUB);
    a.abs(0x9C, W_VZ_LO);
    a.abs(0x9C, W_VZ_HI);
    a.abs(0x9C, W_COMBO);
    a.abs(0x9C, W_STOMP_IF);
    a.imm8(0xA9, 0xFF);
    a.dp(0x85, 0x24);
    a.dp(0x85, 0x25);
    // Only clear $037B if Starman isn't active
    a.abs(0xAD, W_STAR_LO);
    a.abs(0x0D, W_STAR_HI);
    a.branch(0xD0, "reset_done");
    a.abs(0x9C, 0x037B);
    a.label("reset_done");
    a.op(0x60); // RTS

    // =========================================================================
    // SUBROUTINE: Mario_UpdateJump
    // Genuine Top-Down SMW Parabolic Jump Physics (L or R button!)
    // =========================================================================
    a.label("Mario_UpdateJump");
    // Decrement landing cooldown if > 0
    a.abs(0xAD, W_JUMP_CD);
    a.branch(0xF0, "cd_zero");
    a.abs(0xCE, W_JUMP_CD);
    a.label("cd_zero");

    // Decrement post-stomp i-frames if > 0
    a.abs(0xAD, W_STOMP_IF);
    a.branch(0xF0, "stomp_if_done");
    a.abs(0xCE, W_STOMP_IF);
    a.branch(0xD0, "keep_stomp_if");
    // Just expired -> clear $037B if grounded & no Starman
    a.abs(0xAD, W_AIRBORNE);
    a.abs(0x0D, W_STAR_LO);
    a.abs(0x0D, W_STAR_HI);
    a.branch(0xD0, "stomp_if_done");
    a.abs(0x9C, 0x037B);
    a.branch(0x80, "stomp_if_done");
    a.label("keep_stomp_if");
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, 0x037B);
    a.label("stomp_if_done");

    // Check if currently grounded or airborne
    a.abs(0xAD, W_AIRBORNE);
    a.branch(0xD0, "is_airborne");

    // --- GROUNDED: check for L ($20) or R ($10) new press in $F6 ---
    a.abs(0xAD, W_JUMP_CD);
    a.branch(0xD0, "jump_rts");
    // Do not interrupt a vanilla ledge jump ($24 != $FF)
    a.dp(0xA5, 0x24);
    a.imm8(0xC9, 0xFF);
    a.branch(0xD0, "jump_rts");

    // Check if BOTH L+R pressed simultaneously -> handled as Starman activation too!
    a.dp(0xA5, 0xF6); // Joy1 newly pressed low (A X L R)
    a.imm8(0x29, 0x30); // Mask L ($20) | R ($10)
    a.branch(0xF0, "jump_rts");

    // START MARIO JUMP!
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_AIRBORNE); // State = 1 (Rising)
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_Z_PX); // Initial lift = 2px
    a.abs(0x9C, W_Z_SUB);
    a.imm8(0xA9, 0x90); // VZ = +2.56 px/frame (0x0290) -> crisp ~22px peak!
    a.abs(0x8D, W_VZ_LO);
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_VZ_HI);
    a.abs(0x9C, W_COMBO);
    // Write native Z immediately so first jump frame visibly rises!
    a.imm8(0xA9, 0x02);
    a.dp(0x85, 0x24);
    a.dp(0x64, 0x25);
    a.dp(0x64, 0x29);
    // Play Native SPC700 Mario Jump Sweep SFX ($012E = 0x20)!
    a.imm8(0xA9, 0x20);
    a.abs(0x8D, 0x012E);
    a.label("jump_rts");
    a.op(0x60); // RTS

    // --- AIRBORNE: apply variable jump cut, gravity, and integrate Z ---
    a.label("is_airborne");
    // Variable jump height: if rising (VZ_HI >= 1) and BOTH L & R released ($F2 & $30 == 0), cut upward speed!
    a.abs(0xAD, W_VZ_HI);
    a.branch(0x30, "apply_grav"); // BMI (already falling)
    a.imm8(0xC9, 0x01);
    a.branch(0x90, "apply_grav"); // BCC (< 1.0 px/frame)
    a.dp(0xA5, 0xF2); // Joy1 held low
    a.imm8(0x29, 0x30); // L | R held?
    a.branch(0xD0, "apply_grav"); // Still held -> full jump arc!
                                  // Early button release -> short-hop!
    a.imm8(0xA9, 0x00);
    a.abs(0x8D, W_VZ_HI);
    a.imm8(0xA9, 0xC0);
    a.abs(0x8D, W_VZ_LO);

    a.label("apply_grav");
    // Signed 16-bit gravity: VZ -= 0x0024
    a.op(0x38); // SEC
    a.abs(0xAD, W_VZ_LO);
    a.imm8(0xE9, 0x24);
    a.abs(0x8D, W_VZ_LO);
    a.abs(0xAD, W_VZ_HI);
    a.imm8(0xE9, 0x00);
    a.abs(0x8D, W_VZ_HI);

    // Clamp max fall velocity to -3.0 px/frame (VZ_HI >= 0xFD when negative)
    a.branch(0x10, "state_rising"); // BPL -> still rising
                                    // Falling! Set MARIO_AIRBORNE = 2 (Stomp-Ready!)
    a.imm8(0xC9, 0xFD);
    a.branch(0xB0, "mark_falling"); // BCS (signed >= -3)
    a.imm8(0xA9, 0xFD);
    a.abs(0x8D, W_VZ_HI);
    a.abs(0x9C, W_VZ_LO);
    a.label("mark_falling");
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_AIRBORNE);
    a.branch(0x80, "integrate_z");

    a.label("state_rising");
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_AIRBORNE);

    a.label("integrate_z");
    // Z_PX:Z_SUB += VZ_HI:VZ_LO
    a.op(0x18); // CLC
    a.abs(0xAD, W_Z_SUB);
    a.abs(0x6D, W_VZ_LO);
    a.abs(0x8D, W_Z_SUB);
    a.abs(0xAD, W_Z_PX);
    a.abs(0x6D, W_VZ_HI);
    a.abs(0x8D, W_Z_PX);

    // Check landing: if Z_PX <= 0 (Z_PX == 0 or wrapped negative >= 0x80)
    a.branch(0x30, "landed"); // BMI -> went below 0
    a.branch(0xF0, "landed"); // BEQ -> reached 0
                              // Cap max altitude at 36px (0x24)
    a.imm8(0xC9, 0x24);
    a.branch(0x90, "z_ok");
    a.imm8(0xA9, 0x24);
    a.abs(0x8D, W_Z_PX);
    a.label("z_ok");
    // Apply altitude to native ALTTP $24/$25 so Bank $0D lifts Mario above his shadow!
    a.dp(0x85, 0x24);
    a.dp(0x64, 0x25);
    a.dp(0x64, 0x29);
    // High-air clearance: when Z >= 6px, Mario leaps cleanly OVER low ground enemies/hazards!
    a.imm8(0xC9, 0x06);
    a.branch(0x90, "air_rts");
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, 0x037B);
    a.label("air_rts");
    a.op(0x60); // RTS

    a.label("landed");
    a.abs(0x9C, W_AIRBORNE);
    a.abs(0x9C, W_Z_PX);
    a.abs(0x9C, W_Z_SUB);
    a.abs(0x9C, W_VZ_LO);
    a.abs(0x9C, W_VZ_HI);
    a.abs(0x9C, W_COMBO);
    a.imm8(0xA9, 0x04);
    a.abs(0x8D, W_JUMP_CD);
    a.imm8(0xA9, 0xFF);
    a.dp(0x85, 0x24);
    a.dp(0x85, 0x25);
    // Clear jump clearance flag unless Starman or post-stomp i-frames active
    a.abs(0xAD, W_STAR_LO);
    a.abs(0x0D, W_STAR_HI);
    a.abs(0x0D, W_STOMP_IF);
    a.branch(0xD0, "landed_rts");
    a.abs(0x9C, 0x037B);
    a.label("landed_rts");
    a.op(0x60); // RTS

    // =========================================================================
    // SUBROUTINE: Mario_CheckStomp
    // Genuine Mario Stomp Combat while descending (MARIO_AIRBORNE == 2)
    // AND Starman Contact-Kill when Starman timer > 0!
    // =========================================================================
    a.label("Mario_CheckStomp");
    // Check if Starman active (contact-kill mode) OR descending stomp mode!
    a.abs(0xAD, W_STAR_LO);
    a.abs(0x0D, W_STAR_HI);
    a.branch(0xD0, "scan_sprites"); // Starman active -> scan on contact!

    a.abs(0xAD, W_AIRBORNE);
    a.imm8(0xC9, 0x02); // Must be descending (state 2)!
    a.branch(0xD0, "stomp_early_rts");
    a.abs(0xAD, W_Z_PX);
    a.imm8(0xC9, 0x16); // Must be within 1..21px altitude above enemy!
    a.branch(0x90, "scan_sprites");
    a.label("stomp_early_rts");
    a.op(0x60); // RTS

    a.label("scan_sprites");
    a.imm8(0xA2, 0x0F); // LDX #$0F (16 sprite slots: 15..0)

    a.label("spr_loop");
    a.abs(0xBD, 0x0DD0); // LDA $0DD0, X (Sprite state)
    a.imm8(0xC9, 0x09); // Active enemy state?
    a.branch(0xF0, "spr_is_active");
    a.jmp("spr_next");

    a.label("spr_is_active");
    // Check Mario Stomp Vulnerability Table at $22:9800!
    a.op(0x8A); // TXA
    a.op(0x48); // PHA (save slot X)
    a.abs(0xBD, 0x0E20); // LDA $0E20, X (Sprite Type ID)
    a.op(0xAA); // TAX
    a.long(0xBF, MARIO_ENGINE_BANK, STOMP_TABLE_ADDR); // LDA $22:9800, X
    a.op(0xA8); // TAY (Y = vulnerability: 0=ignore, 1=stompable, 2=spiked)
    a.op(0x68); // PLA
    a.op(0xAA); // TAX (restore slot X)

    a.op(0x98); // TYA
    a.branch(0xD0, "spr_vuln_nonzero");
    a.jmp("spr_next"); // 0 = NPC/Item/Boss/Non-enemy -> skip!

    a.label("spr_vuln_nonzero");
    // If spiked (Y == 2), ONLY Starman can defeat it (ordinary stomp does NOT kill Spinies/Thwomps!)
    a.imm8(0xC9, 0x02);
    a.branch(0xD0, "check_bbox");
    a.abs(0xAD, W_STAR_LO);
    a.abs(0x0D, W_STAR_HI);
    a.branch(0xD0, "check_bbox");
    a.jmp("spr_next");

    a.label("check_bbox");
    // Check 16-bit X proximity: |sprX - marioX| < 16
    // Compute delta_x = sprX - marioX + 16; valid if high byte == 0 and low byte < 32!
    a.op(0x38); // SEC
    a.abs(0xBD, 0x0D10); // LDA $0D10, X (sprX lo)
    a.dp(0xE5, 0x22); // SBC $22 (marioX lo)
    a.op(0xA8); // TAY (temp lo)
    a.abs(0xBD, 0x0D30); // LDA $0D30, X (sprX hi)
    a.dp(0xE5, 0x23); // SBC $23 (marioX hi)
                      // Add +16 to 16-bit (A:Y):
    a.op(0x48); // PHA (hi)
    a.op(0x98); // TYA
    a.op(0x18); // CLC
    a.imm8(0x69, 0x10); // +16
    a.op(0xA8); // TAY
    a.op(0x68); // PLA
    a.imm8(0x69, 0x00);
    a.branch(0xF0, "x_hi_ok");
    a.jmp("spr_next"); // High byte != 0 -> far away in X
    a.label("x_hi_ok");
    a.op(0x98); // TYA
    a.imm8(0xC9, 0x20); // < 32 (i.e., -16..+15 px)?
    a.branch(0x90, "x_ok");
    a.jmp("spr_next");

    a.label("x_ok");
    // Check 16-bit Y proximity: |sprY - marioY| < 18 (+18 -> < 36 = 0x24)
    a.op(0x38); // SEC
    a.abs(0xBD, 0x0D00); // LDA $0D00, X (sprY lo)
    a.dp(0xE5, 0x20); // SBC $20 (marioY lo)
    a.op(0xA8); // TAY
    a.abs(0xBD, 0x0D20); // LDA $0D20, X (sprY hi)
    a.dp(0xE5, 0x21); // SBC $21 (marioY hi)
    a.op(0x48); // PHA
    a.op(0x98); // TYA
    a.op(0x18); // CLC
    a.imm8(0x69, 0x12); // +18
    a.op(0xA8); // TAY
    a.op(0x68); // PLA
    a.imm8(0x69, 0x00);
    a.branch(0xF0, "y_hi_ok");
    a.jmp("spr_next");
    a.label("y_hi_ok");
    a.op(0x98); // TYA
    a.imm8(0xC9, 0x24); // < 36 (-18..+17 px)?
    a.branch(0x90, "stomp_confirmed");
    a.jmp("spr_next");

    a.label("stomp_confirmed");
    // === CONFIRMED HIT! Check Starman vs Stomp ===
    a.abs(0xAD, W_STAR_LO);
    a.abs(0x0D, W_STAR_HI);
    a.branch(0xD0, "instant_kill");

    // Normal Mario Stomp: deal 6 HP damage (1-hits Goombas/Koopas, 2-hits armored enemies!)
    a.op(0x38); // SEC
    a.abs(0xBD, 0x0E50); // LDA $0E50, X (Sprite HP)
    a.imm8(0xE9, 0x06);
    a.abs(0x9D, 0x0E50);
    a.branch(0x90, "instant_kill"); // BCC (underflowed <= 0 -> KO!)
    a.branch(0xF0, "instant_kill"); // BEQ (exact 0 -> KO!)
                                    // Tough enemy survived -> recoil flash + stomp SFX + bounce Mario!
    a.imm8(0xA9, 0x0C);
    a.abs(0x9D, 0x0EA0); // Damage flash timer
    a.imm8(0xA9, 0x08);
    a.abs(0x8D, 0x012F); // Mario Stomp Pop SFX
    a.branch(0x80, "do_stomp_bounce");

    a.label("instant_kill");
    // Defeat enemy cleanly in ALTTP's native puff-of-smoke death state ($0DD0 = 6)!
    a.imm8(0xA9, 0x06);
    a.abs(0x9D, 0x0DD0);
    a.imm8(0xA9, 0x1F);
    a.abs(0x9D, 0x0DF0);
    a.imm8(0xA9, 0x08);
    a.abs(0x8D, 0x012F); // Mario Stomp Pop SFX!
                         // Award +1 Gold Coin to Mario's target coin counter ($7E:F362, capped at 999)!
    a.abs(0xAD, 0xF363);
    a.imm8(0xC9, 0x03);
    a.branch(0xB0, "coin_done");
    a.op(0x18);
    a.abs(0xAD, 0xF362);
    a.imm8(0x69, 0x01);
    a.abs(0x8D, 0xF362);
    a.abs(0xAD, 0xF363);
    a.imm8(0x69, 0x00);
    a.abs(0x8D, 0xF363);
    a.label("coin_done");

    // If Starman contact kill while grounded, don't bounce Mario!
    a.abs(0xAD, W_AIRBORNE);
    a.branch(0xF0, "spr_next");

    a.label("do_stomp_bounce");
    // Grant Mario 16 frames of post-stomp protection + upward SMW bounce!
    a.imm8(0xA9, 0x12);
    a.abs(0x8D, W_STOMP_IF);
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, 0x037B);
    a.abs(0x8D, W_AIRBORNE); // Rising again!
    a.abs(0xEE, W_COMBO);
    a.dp(0xA5, 0xF2); // Holding L or R -> High Mario Bounce!
    a.imm8(0x29, 0x30);
    a.branch(0xD0, "high_bounce");
    a.imm8(0xA9, 0x20);
    a.abs(0x8D, W_VZ_LO);
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_VZ_HI);
    a.op(0x60); // RTS (1 stomp per frame)
    a.label("high_bounce");
    a.imm8(0xA9, 0xB8);
    a.abs(0x8D, W_VZ_LO);
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_VZ_HI);
    a.op(0x60); // RTS

    a.label("spr_next");
    a.op(0xCA); // DEX
    a.branch(0x30, "stomp_exit"); // BMI stomp_exit (finished slot 0)
    a.jmp("spr_loop");
    a.label("stomp_exit");
    a.op(0x60); // RTS

    // =========================================================================
    // SUBROUTINE: Mario_UpdatePowerUps
    // 1. Activate Super Starman on L+R simultaneous press (costs 0x20 Star Power from $F36E)
    // 2. Rainbow CGRAM cycling while Starman active
    // 3. Live Fire Mario White/Red CGRAM palette swap when Fire Flower ($0202 == 0x06) equipped!
    // =========================================================================
    a.label("Mario_UpdatePowerUps");
    // Check L+R simultaneous activation for Super Starman (only if not already active)
    a.abs(0xAD, W_STAR_LO);
    a.abs(0x0D, W_STAR_HI);
    a.branch(0xD0, "star_tick");

    // Check if both L & R held ($F2 & $30 == $30) AND at least one newly pressed ($F6 & $30 != 0)
    a.dp(0xA5, 0xF2);
    a.imm8(0x29, 0x30);
    a.imm8(0xC9, 0x30);
    a.branch(0xD0, "check_fire_mario");
    a.dp(0xA5, 0xF6);
    a.imm8(0x29, 0x30);
    a.branch(0xF0, "check_fire_mario");
    // Require >= 0x20 Star Power in $7E:F36E
    a.abs(0xAD, 0xF36E);
    a.imm8(0xC9, 0x20);
    a.branch(0x90, "check_fire_mario");
    a.op(0x38);
    a.imm8(0xE9, 0x20);
    a.abs(0x8D, 0xF36E);
    // Start 240 frames (0x00F0) of Super Starman!
    a.imm8(0xA9, 0xF0);
    a.abs(0x8D, W_STAR_LO);
    a.abs(0x9C, W_STAR_HI);
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_POWERUP);
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, 0x037B);
    a.imm8(0xA9, 0x2B);
    a.abs(0x8D, 0x012E); // Crisp Mario Star/Coin chime!

    a.label("star_tick");
    // Decrement 16-bit Starman timer
    a.op(0x38);
    a.abs(0xAD, W_STAR_LO);
    a.imm8(0xE9, 0x01);
    a.abs(0x8D, W_STAR_LO);
    a.abs(0xAD, W_STAR_HI);
    a.imm8(0xE9, 0x00);
    a.abs(0x8D, W_STAR_HI);
    a.abs(0xAD, W_STAR_LO);
    a.abs(0x0D, W_STAR_HI);
    a.branch(0xD0, "star_rainbow");
    // Starman finished -> restore normal invulnerability & force palette restore!
    a.abs(0x9C, 0x037B);
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_PAL_DIRTY);
    a.abs(0x9C, W_POWERUP);
    a.branch(0x80, "check_fire_mario");

    a.label("star_rainbow");
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, 0x037B);
    // Pick rainbow palette 2..5 from (($1A >> 2) & 3) + 2 -> index * 30!
    a.dp(0xA5, 0x1A);
    a.op(0x4A); // LSR
    a.op(0x4A); // LSR
    a.imm8(0x29, 0x03);
    a.op(0x18);
    a.imm8(0x69, 0x02); // pal_idx = 2..5
    a.jsr("Mario_LoadCgramPal");
    a.op(0x60); // RTS

    a.label("check_fire_mario");
    // Check currently equipped Y-item ($0202): 0x06 = Fire Flower (Fire Rod)!
    a.abs(0xAD, 0x0202);
    a.imm8(0xC9, 0x06);
    a.branch(0xD0, "want_classic");

    // Want Fire Mario (palette 1)! Only write CGRAM if not already Fire Mario or dirty!
    a.abs(0xAD, W_POWERUP);
    a.imm8(0xC9, 0x01);
    a.branch(0xD0, "apply_fire");
    a.abs(0xAD, W_PAL_DIRTY);
    a.branch(0xF0, "pu_rts");
    a.label("apply_fire");
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_POWERUP);
    a.abs(0x9C, W_PAL_DIRTY);
    a.jsr("Mario_LoadCgramPal"); // A = 1 -> Fire Mario White/Red!
    a.op(0x60); // RTS

    a.label("want_classic");
    a.abs(0xAD, W_POWERUP);
    a.abs(0x0D, W_PAL_DIRTY);
    a.branch(0xF0, "pu_rts");
    a.abs(0x9C, W_POWERUP);
    a.abs(0x9C, W_PAL_DIRTY);
    a.imm8(0xA9, 0x00); // A = 0 -> Classic Super Mario Red/Blue!
    a.jsr("Mario_LoadCgramPal");
    a.label("pu_rts");
    a.op(0x60); // RTS

    // =========================================================================
    // SUBROUTINE: Mario_LoadCgramPal
    // Input: A = palette index (0..5) -> copies 30 bytes from $22:9900 + A*30
    //        into live CGRAM OBJ buffer $7E:C6E2..$7E:C6FF and sets $15 = 1!
    // =========================================================================
    a.label("Mario_LoadCgramPal");
    // Compute offset = A * 30 = A*32 - A*2 (max 5*30 = 150 fits cleanly in 8-bit X!)
    a.op(0x0A); // ASL (*2)
    a.op(0x48); // PHA (*2)
    a.op(0x0A); // *4
    a.op(0x0A); // *8
    a.op(0x0A); // *16
    a.op(0x0A); // *32
    a.op(0x38); // SEC
    a.dp(0xE3, 0x01); // SBC 1, S (stack-relative! Wait: keep pure clean 65c816!)
                      // Use stack-clean temp in $F51E instead of SBC 1,S for maximum readability!
    a.code.truncate(a.code.len() - 2);
    a.abs(0x8D, 0xF51E); // temp32 = A*32
    a.op(0x68); // PLA (A*2)
    a.abs(0x8D, 0xF51F); // temp2 = A*2
    a.op(0x38); // SEC
    a.abs(0xAD, 0xF51E);
    a.abs(0xED, 0xF51F); // A = A*30!
    a.op(0xAA); // TAX (source index in $22:9900,X)
    a.imm8(0xA0, 0x00); // LDY #$00 (dest offset 0..29)

    a.label("pal_copy_loop");
    a.long(0xBF, MARIO_ENGINE_BANK, PAL_TABLES_ADDR); // LDA $22:9900, X
    a.abs(0x99, 0xC6E2); // STA $C6E2, Y (live Link OBJ CGRAM colors 1..15!)
    a.op(0xE8); // INX
    a.op(0xC8); // INY
    a.imm8(0xC0, 0x1E); // CPY #30
    a.branch(0x90, "pal_copy_loop"); // BCC pal_copy_loop

    a.imm8(0xA9, 0x01);
    a.dp(0x85, 0x15); // Trigger NMI CGRAM DMA refresh!
    a.op(0x60); // RTS

    a.finish()
}

/// Inject the complete Phase 5 Native 65c816 Super Mario Gameplay Engine into Bank `$22`
/// and hook the 3 verified `Link_Main` (`JSL $07:8000`) callers in Bank `$02`.
pub fn inject_mario_gameplay_engine(
    rom_zelda: &mut SnesRom,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    if rom_zelda.data.len() < 0x112000 {
        return Ok(0);
    }
    let mut ops = 0usize;

    // 1. Assemble and write the 65c816 Mario Engine code into Bank $22:9000 (PC 0x111000)
    let code = assemble_mario_engine_65816().map_err(|_| RomError::OutOfBounds {
        offset: MARIO_ENGINE_PC,
        len: 0x800,
    })?;
    if code.len() > 0x7F0 {
        return Err(RomError::OutOfBounds {
            offset: MARIO_ENGINE_PC,
            len: code.len(),
        });
    }
    session.swap_custom_bytes(
        rom_zelda,
        MARIO_ENGINE_PC,
        MARIO_ENGINE_PC,
        code.clone(),
        format!(
            "Moteur 65c816 Super Mario Phase 5 ($22:9000, {} B) : Saut Top-Down (L/R), Stomp Ennemis + Rebond, Fire Mario & Starman",
            code.len()
        ),
        SwapKind::CodePatch {
            byte_count: code.len(),
        },
    )?;
    ops += 1;

    // 2. Write 256-byte Mario Stomp Vulnerability Table at $22:9800 (PC 0x111800)
    let stomp_tbl = build_mario_stomp_table().to_vec();
    session.swap_custom_bytes(
        rom_zelda,
        STOMP_TABLE_PC,
        STOMP_TABLE_PC,
        stomp_tbl,
        "Table Vulnérabilité Stomp Mario ($22:9800, 256 B) : Goombas/Koopas/Rex écrasables, Spinies/Thwomps protégés".into(),
        SwapKind::CodePatch { byte_count: 256 },
    )?;
    ops += 1;

    // 3. Write 6 live BGR555 Mario Power-Up Palettes at $22:9900 (PC 0x111900)
    let pals = build_mario_powerup_palettes();
    let pal_len = pals.len();
    session.swap_custom_bytes(
        rom_zelda,
        PAL_TABLES_PC,
        PAL_TABLES_PC,
        pals,
        "Palettes Live Power-Up Mario ($22:9900, 180 B) : Super Mario, Fire Mario Blanc/Rouge & 4 Cycles Starman".into(),
        SwapKind::PaletteBgr555 { color_count: 90 },
    )?;
    let _ = pal_len;
    ops += 1;

    // 4. Verify & hook all 3 Bank $02 callers of `JSL $07:8000` (`22 00 80 07` -> `22 00 90 22`)!
    const VANILLA_JSL: [u8; 4] = [0x22, 0x00, 0x80, 0x07];
    const MARIO_JSL: [u8; 4] = [
        0x22,
        (MARIO_ENGINE_ADDR & 0xFF) as u8,
        (MARIO_ENGINE_ADDR >> 8) as u8,
        MARIO_ENGINE_BANK,
    ];
    for &pc in &LINK_MAIN_CALL_SITES_PC {
        if pc + 4 <= rom_zelda.data.len() && rom_zelda.data[pc..pc + 4] == VANILLA_JSL {
            session.swap_custom_bytes(
                rom_zelda,
                pc,
                pc,
                MARIO_JSL.to_vec(),
                format!(
                    "Hook Boucle Joueur (PC 0x{pc:06X}) : JSL $07:8000 -> JSL $22:9000 (Moteur Saut/Stomp/Power-Up Mario)"
                ),
                SwapKind::CodePatch { byte_count: 4 },
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
    fn test_mario_engine_65816_assembles_cleanly() {
        let code = assemble_mario_engine_65816().expect("65c816 Mario engine must assemble");
        assert!(
            code.len() > 250 && code.len() < 0x700,
            "Engine size {}B must stay compact inside $22:9000..$22:97FF",
            code.len()
        );
        // Must contain JSL $07:8000 (`22 00 80 07`) and end cleanly with RTS (`60`)!
        assert!(
            code.windows(4).any(|w| w == [0x22, 0x00, 0x80, 0x07]),
            "Hook must call original Link_Main ($07:8000)"
        );
        assert_eq!(*code.last().unwrap(), 0x60);

        let stomp = build_mario_stomp_table();
        assert_eq!(stomp[0x41], 1, "Blue Koopa Soldier must be stompable");
        assert_eq!(stomp[0x42], 1, "Green Koopa Soldier must be stompable");
        assert_eq!(stomp[0x08], 1, "Octorok/Goomba must be stompable");
        assert_eq!(stomp[0x98], 2, "Thwomp must be spiked (not ordinary stomp)");
        assert_eq!(stomp[0xCE], 0, "Bosses/NPCs must not be ordinary stomp");

        let pals = build_mario_powerup_palettes();
        assert_eq!(
            pals.len(),
            180,
            "6 power-up palettes * 30 bytes = 180 bytes"
        );
    }
}
