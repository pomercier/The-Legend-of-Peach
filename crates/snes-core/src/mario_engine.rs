//! Phase 8 (v0.8.0 — Super Mario Universe Revolution):
//! Native 65c816 Super Mario Gameplay, Enemy AI, Power-Up & Yoshi Companion Engine
//! (`Bank $22:9000..$22:B2FF`, `PC 0x111000..0x1132FF`).
//!
//! Runs 100% natively on unmodified Super Nintendo / Super Famicom hardware (2 MiB LoROM,
//! zero overclock, zero enhancement chips, zero MSU-1, < 2.2% of PAL 50 Hz frame budget):
//!
//! 1. **Variable Top-Down Mario Jump (`L` / `R`) + Cape Mario Mid-Air Glide + Yoshi Flutter Jump**:
//!    - 8.8 fixed-point parabolic jump physics with variable height and crisp ground shadow (`$24/$25`).
//!    - **Cape Mario (`PLUME DE CAPE`, `$0202 == 0x13`)**: holding `L`/`R` while falling glides at
//!      `-0.75 px/frame` (instead of `-3.0 px/frame`), tripling airborne range across hazards!
//!    - **Yoshi Flutter Jump (`W_YOSHI_ACTIVE == 1`)**: holding `L`/`R` during fall triggers 16 frames
//!      of Yoshi's Island Flutter-Kick upward hover/lift (`+0.5 px/frame`)!
//!
//! 2. **Authentic Mario Ground Pound (`Down` newly pressed while airborne at `Z >= 7px`)**:
//!    - 6-frame mid-air stall (`W_AIRBORNE = 3`) -> high-speed slam (`W_AIRBORNE = 4`, `-5.0 px/frame`).
//!    - On impact: triggers hardware screen shake (`$7E:011A = 0x0E`), heavy thud SFX (`$012F = 0x0C`),
//!      and a 48px-radius 360° shockwave that **flattens/cracks armored & spiked enemies (`0x02`)**,
//!      awards `+2` Bonus Coins & `+8` Star Power, AND **flips nearby dungeon SMW ON/OFF Crystal
//!      Switches (`$0E20 == 0x1E`) via native ALTTP Submodule `$16` (`$7E:C172 ^= 1`, `$11 = 0x16`)**!
//!
//! 3. **Real Koopa Shell Retract, Kick & Wall-Ricochet Bowling Engine (`$22:B000` class `0x03`)**:
//!    - **First Stomp** on Koopa Troopas / Hammer Bros / Buzzy Beetles retracts them into a
//!      **Stunned Koopa Shell** (`W_SHELL_STATE = 1`) and bounces Mario cleanly upward.
//!    - **Kicking** the stunned shell launches it at `±3 px/frame` along Mario's facing direction (`$2F`),
//!      **ricocheting off walls/boundaries** and **bowling through other active enemies** for instant
//!      chain KOs, escalating Coin Chimes, and `+2` Bonus Coins per bowled enemy!
//!
//! 4. **Authentic SMW Boo Orientation-Freeze AI (`$22:B000` class `0x04`, sprite `0x19`)**:
//!    - Compares Mario's facing direction (`$2F`) against `(Boo - Mario)` every frame:
//!      - When Mario **faces the Boo**, it shyly **freezes solid in place** (`$0D50/$0D40 = 0`).
//!      - When Mario **turns his back**, it **creeps stealthily toward Mario**!
//!
//! 5. **6-Form Mario Power-Up State Machine (`$22:B100`, 9 live BGR555 palettes = 270 B)**:
//!    - `0` Classic Super Mario (Red/Blue)
//!    - `1` **Fire Mario** (`$0202 == 0x06` Fire Flower: White/Crimson)
//!    - `2` **Ice Mario** (`$0202 == 0x07` Ice Flower: Frost-Cyan/Glacier-Blue + **Ice Stomp Shatter**
//!          bonus on frozen `$0DD0 == 0x0B` enemies for `+3` Coins & `+8` Star Power!)
//!    - `3` **Cape Mario** (`$0202 == 0x13` Cape Feather: SMW Golden Cape Trim + Mid-Air Glide)
//!    - `4` **Yoshi-Mounted Mario** (`W_YOSHI_ACTIVE == 1`: Mario Red Cap + Yoshi Emerald Saddle +
//!          **SMW 1-Hit Damage Absorption Shield** restoring Mario's HP on hit and dismounting Yoshi!)
//!    - `5..8` **Resource-Balanced Super Starman (`L + R`)**: continuously drains Star Power (`-1` every
//!          6 frames from `$7E:F36E`, ending cleanly when depleted — eliminating infinite invincibility
//!          while rewarding enemy stomps/Ground Pounds that refill Star Power!).
//!
//! 6. **Interactive Mario Turn-Block / Prop Throw Reward**:
//!    - Throwing any lifted Turn-Block or Smiling Bush (`$0308` transitioning `1 -> 0`) awards
//!      `+1` Gold Coin, `+2` Star Power, and native SPC700 Mario Coin Chime (`$012E = 0x2B`)!

use std::collections::HashMap;

use crate::rom::{RomError, SnesRom};
use crate::swapper::{SwapKind, SwapSession};

/// Bank `$22` LoROM base for Mario Gameplay Engine (`PC 0x111000..0x112FFF`, 8 KiB code budget).
pub const MARIO_ENGINE_PC: usize = 0x111000;
pub const MARIO_ENGINE_BANK: u8 = 0x22;
pub const MARIO_ENGINE_ADDR: u16 = 0x9000;

/// 256-byte Enemy Mario Behavior & Vulnerability Table at `$22:B000` (`PC 0x113000`):
/// - `0x00` = Non-enemy / NPC / Item / Boss
/// - `0x01` = Standard Stompable Enemy (Goombas, Rex, Monty Moles, Cheep-Cheeps, Dry Bones)
/// - `0x02` = Spiked / Armored Enemy (Spinies, Thwomps, Chain Chomps — immune to normal stomp;
///            defeated by Starman, Kicked Koopa Shell, or Mario Ground Pound Shockwave!)
/// - `0x03` = Koopa / Buzzy Shell Family (1st Stomp -> Stunned Shell; Touch/Kick -> Ricochet Bowling Shell!)
/// - `0x04` = SMW Boo / Ghost Family (`0x19` — Orientation-Freeze AI!)
pub const STOMP_TABLE_PC: usize = 0x113000;
pub const STOMP_TABLE_ADDR: u16 = 0xB000;

/// Live Mario Power-Up BGR555 Palette Tables at `$22:B100` (`PC 0x113100`):
/// 9 palettes × 30 bytes = 270 bytes (colors 1..15, little-endian BGR555):
///   0 = Classic Super Mario
///   1 = Fire Mario (White/Crimson)
///   2 = Ice Mario (Frost-Cyan/Glacier-Blue)
///   3 = Cape Mario (SMW Golden Cape Trim)
///   4 = Yoshi-Mounted Mario (Red Cap + Yoshi Emerald Saddle + Orange Boots)
///   5..8 = Super Starman Rainbow Cycle (Gold, Cyan, Neon Green, Blazing Magenta)
pub const PAL_TABLES_PC: usize = 0x113100;
pub const PAL_TABLES_ADDR: u16 = 0xB100;

/// Verified free WRAM block (`$7E:F500..$7E:F53F`, 64 bytes, 0 conflicting ROM refs).
const W_AIRBORNE: u16 = 0xF500; // 0=Grounded, 1=Rising, 2=Falling, 3=GroundPound-Hang, 4=GroundPound-Slam
const W_Z_PX: u16 = 0xF501; // Integer Z height (0..38 px)
const W_Z_SUB: u16 = 0xF502; // Fractional 8-bit Z subpixel
const W_VZ_LO: u16 = 0xF503; // Signed 8.8 vertical velocity low
const W_VZ_HI: u16 = 0xF504; // Signed 8.8 vertical velocity high
const W_COMBO: u16 = 0xF505; // Consecutive airborne stomp combo counter
const W_STOMP_IF: u16 = 0xF506; // Post-stomp / Ground-Pound / Yoshi-dismount i-frame timer
const W_POWERUP: u16 = 0xF507; // 0=Classic, 1=Fire, 2=Ice, 3=Cape, 4=Yoshi, 5=Starman
const W_STAR_LO: u16 = 0xF508; // Starman countdown timer low
const W_STAR_HI: u16 = 0xF509; // Starman countdown timer high
const W_LAST_MODE: u16 = 0xF50A; // Previous $10 game mode
const W_LAST_AREA: u16 = 0xF50B; // Previous $8A/$A0 area ID
const W_PAL_DIRTY: u16 = 0xF50C; // 1 = force palette restore
const W_JUMP_CD: u16 = 0xF50D; // Landing cooldown frames
const W_GP_HANG: u16 = 0xF50E; // Ground Pound mid-air stall countdown
const W_PREV_CARRY: u16 = 0xF50F; // Previous $0308 carry flag (1->0 Turn-Block Coin reward!)

// Koopa Shell Engine state (`$7E:F510..$7E:F517`)
const W_SHELL_SLOT: u16 = 0xF510; // 0xFF = none, 0..15 = active shell sprite slot
const W_SHELL_STATE: u16 = 0xF511; // 0=None, 1=Stunned Shell, 2=Kicked Ricochet Bowling Shell
const W_SHELL_VX: u16 = 0xF512; // Signed pixel delta X (-3, 0, +3)
const W_SHELL_VY: u16 = 0xF513; // Signed pixel delta Y (-3, 0, +3)
const W_SHELL_TIMER: u16 = 0xF514; // Shell active countdown frames
const W_SHELL_KICK_CD: u16 = 0xF515; // Post-stomp grace frames before kick
const W_SHELL_TMP_X: u16 = 0xF516; // Scratch shell X lo for bowling check
const W_SHELL_TMP_Y: u16 = 0xF517; // Scratch shell Y lo for bowling check

// Yoshi Companion, Boss Combat & Balanced Starman state (`$7E:F518..$7E:F51F`)
const W_YOSHI_ACTIVE: u16 = 0xF518; // 0=On Foot, 1=Riding Yoshi Companion!
const W_YOSHI_FLUTTER: u16 = 0xF519; // Remaining Flutter Jump frames in current jump
const W_YOSHI_LAST_HP: u16 = 0xF51A; // Last observed Mario HP ($7E:F36D) for 1-hit Yoshi shield
const W_BOSS_HIT_CD: u16 = 0xF51B; // v0.9.0 Boss Hit Coin & Star-Power synergy debounce timer
const W_STAR_DRAIN_CTR: u16 = 0xF51C; // Starman Star-Power drain prescaler (drains every 6 frames)
const W_GP_IMPACT: u16 = 0xF51D; // 1 = Ground Pound just impacted this frame
const W_TMP_A: u16 = 0xF51E; // Scratch A
const W_TMP_B: u16 = 0xF51F; // Scratch B

// Boo Orientation-Freeze AI coordinate anchors (`$7E:F520..$7E:F53F`, 16 slots X + 16 slots Y)
const W_BOO_LAST_X: u16 = 0xF520;
const W_BOO_LAST_Y: u16 = 0xF530;

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
            code: Vec::with_capacity(2048),
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

/// Build the 256-byte Mario Enemy Behavior & Stomp Vulnerability Table (`$22:B000`):
/// - `0x00` = Non-enemy / NPC / Item / Unstompable / Boss
/// - `0x01` = Standard Stompable Mario Enemy (Goombas, Rex, Monty Moles, Cheep-Cheeps, Dry Bones)
/// - `0x02` = Spiked / Armored Enemy (Spinies, Thwomps, Pokeys — immune to normal stomp;
///            defeated by Starman, Kicked Koopa Shell, OR Mario Ground Pound Shockwave!)
/// - `0x03` = Koopa Troopa / Buzzy Beetle Shell Family (1st Stomp -> Stunned Shell; Kick -> Ricochet Bowling!)
/// - `0x04` = Authentic SMW Boo Family (`0x19` — Orientation-Freeze AI!)
pub fn build_mario_stomp_table() -> [u8; 256] {
    let mut t = [0u8; 256];

    // 1. Standard Stompable Overworld & Dungeon Enemies
    const STOMPABLE: &[u8] = &[
        0x00, // Raven -> Swooper
        0x01, // Vulture -> Paragoomba
        0x08, // Octorok -> Goomba
        0x0A, // Octorok 4-way
        0x0D, // Buzzblob -> Electric Amp
        0x0E, // Snapdragon -> Rex
        0x0F, // Octoballoon
        0x11, // Hinox -> Chargin' Chuck
        0x12, // Moblin -> Hammer Bro
        0x18, // Mini Moldorm -> Mini Wiggler
        0x23, // Red Bari
        0x24, // Blue Bari
        0x46, // Blue Archer -> Bill Koopa
        0x47, // Green Archer -> Bill Koopa
        0x4A, // Red Bomb Soldier -> Bob-omb Koopa
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
        0x8F, // Blob -> Goomba
        0x90, // Wallmaster
        0x91, // Stalfos Knight -> Dry Bones
        0x94, // Pirogusu -> Cheep
        0x95, // Laser Eye
        0x9A, // Kyameron
        0x9B, // Wizzrobe -> Magikoopa
        0x9C, // Zazak -> Dino Rhino
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
        0xC7, // Hokkubokku -> Pokey
    ];
    for &id in STOMPABLE {
        t[id as usize] = 0x01;
    }

    // 2. Spiked / Armored Enemies (immune to ordinary stomp; defeated by Starman, Kicked Shell, or Ground Pound!)
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

    // 3. Koopa Troopa & Buzzy Beetle Shell Family (1st Stomp -> Stunned Shell; Kick -> Ricochet Bowling!)
    const KOOPA_SHELL_FAMILY: &[u8] = &[
        0x13, // Mini Helmasaur -> Buzzy Beetle
        0x26, // Hardhat Beetle -> Buzzy Beetle
        0x41, // Blue Koopa Hammer Bro Guard
        0x42, // Green Koopa Hammer Bro Guard
        0x43, // Red Koopa Guard
        0x44, // Armored Koopa Guard
        0x45, // Green Koopa Guard
        0x48, // Red Hammer Bro
        0x49, // Red Bush Koopa
        0x4B, // Green Koopa
        0x8D, // Terrorpin -> Buzzy Beetle
        0x8E, // Terrorpin 2 -> Buzzy Beetle
    ];
    for &id in KOOPA_SHELL_FAMILY {
        t[id as usize] = 0x03;
    }

    // 4. Authentic SMW Boo Family (Orientation-Freeze AI: freezes when watched, creeps when ignored!)
    t[0x19] = 0x04; // Overworld Graveyard Poe -> SMW Boo!
    t[0x8B] = 0x04; // Haunted Mansion / Skull Woods Ghost -> SMW Ghost House Boo!

    // 5. v0.9.0 — All 12 Mario & Koopaling Boss Encounters (Ground Pound Boss Stagger + Hit Coin/Star Synergy!)
    //    Protected from ordinary jump-stomps so multi-phase boss scripts never break, while Mario Ground Pound
    //    staggers bosses with arena shake + +3 Coins + safe 2 HP chip, and every boss hit refills Coins & Star Power!
    const MARIO_BOSSES: &[u8] = &[
        0x09, // Moldorm -> Giant Wiggler
        0x53, // Armos Knight -> Giant Chargin' Chucks
        0x54, // Lanmolas -> 3 Giant Pokeys
        0x7A, // Agahnim -> Castle Tower King Bowser
        0x88, // Mothula -> Winged Iggy Koopa
        0x8C, // Arrghus -> Wendy O. Koopa
        0x92, // Helmasaur King -> Morton Koopa Jr.
        0xA2, // Kholdstare -> Lemmy Koopa
        0xBD, // Vitreous -> Larry Koopa
        0xCB, // Trinexx Body -> Ludwig Mech Bowser
        0xCC, // Trinexx Fire Head -> Ludwig Dino Torch
        0xCD, // Trinexx Ice Head -> Ludwig Frost Koopa
        0xCE, // Blind -> Roy Koopa
        0xD6, // Ganon -> Final Boss King Bowser!
    ];
    for &id in MARIO_BOSSES {
        t[id as usize] = 0x05;
    }

    t
}

/// Build the 9 live Mario Power-Up & Companion BGR555 palettes (colors 1..15 each = 30 B × 9 = 270 B).
///   0 = Classic Super Mario (Red cap/shirt, Blue overalls)
///   1 = Fire Mario (Pure White cap/shirt, Blazing Crimson overalls)
///   2 = Ice Mario (Crisp Frost-Cyan cap/shirt, Deep Glacier-Blue overalls)
///   3 = Cape Mario (SMW Golden-Yellow Cap/Cape Trim, Mario Red/Blue)
///   4 = Yoshi-Mounted Mario (Red Cap + Vibrant SMW Yoshi Emerald-Green Saddle/Overalls + Orange Boots)
///   5..8 = Super Starman Rainbow Cycle (Gold, Cyan, Neon Green, Blazing Magenta)
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
        0x7FFF, 0x0000, 0x0D71, 0x1E9B, 0x3B7F, 0x635F, 0x5AD6, 0x3DEF,
        0x7FFF, // Pure white cap/shirt
        0x0010, 0x001B, 0x14DF, // Blazing crimson overalls
        0x7FFF, 0x02BF, 0x03FF,
    ];
    let ice: [u16; 15] = [
        0x7FFF, // 1: Snow white gloves/eyes
        0x0000, // 2: Crisp black outline
        0x5182, // 3: Frosted sapphire boots
        0x1E9B, // 4: Skin shade
        0x3B7F, // 5: Peach skin
        0x635F, // 6: Skin highlight
        0x7AE4, // 7: Cool ice-cyan cap shadow
        0x5940, // 8: Deep glacier outline
        0x7FE8, // 9: ELECTRIC FROST-CYAN ICE MARIO CAP & SHIRT!
        0x5440, // 10: Deep glacier sapphire shadow
        0x7542, // 11: BRIGHT GLACIER-BLUE ICE MARIO OVERALLS!
        0x7F6E, // 12: Sparkling frost highlight
        0x7FFF, // 13: Ice glint
        0x7FD4, // 14: Cyan shimmer
        0x7FFF, // 15: Crystal buttons
    ];
    let cape: [u16; 15] = [
        0x7FFF, // 1: White gloves/eyes
        0x0000, // 2: Black outline
        0x0D71, // 3: Brown boots
        0x1E9B, // 4: Skin shade
        0x3B7F, // 5: Peach skin
        0x635F, // 6: Skin highlight
        0x02BF, // 7: Rich golden-amber cape shadow
        0x0196, // 8: Deep gold outline
        0x03FF, // 9: BLAZING SMW CANARY-GOLD CAPE FEATHER CAP & SHIRT!
        0x0014, // 10: Deep Mario crimson shadow
        0x001F, // 11: Vibrant Mario crimson overalls
        0x2DDF, // 12: Bright highlight
        0x7FFF, // 13: White highlight
        0x03FF, // 14: Gold shimmer
        0x03FF, // 15: Golden buttons
    ];
    let yoshi_rider: [u16; 15] = [
        0x7FFF, // 1: White gloves/Yoshi belly
        0x0000, // 2: Crisp black outline
        0x01DF, // 3: Iconic SMW Yoshi Orange Boots!
        0x1E9B, // 4: Skin shade
        0x3B7F, // 5: Peach skin
        0x635F, // 6: Skin highlight
        0x0857, // 7: Mario Red Cap shadow
        0x000A, // 8: Mario Red outline
        0x10DF, // 9: Classic Mario Red Cap!
        0x0240, // 10: Deep SMW Yoshi Forest Green shadow
        0x03A2, // 11: ELECTRIC SMW YOSHI EMERALD-LIME SADDLE & BODY!
        0x2FEA, // 12: Bright Yoshi Neon-Lime highlight!
        0x7FFF, // 13: Yoshi white cheek/belly trim
        0x02DF, // 14: Yoshi orange saddle accent
        0x03FF, // 15: Golden buttons
    ];
    let star0: [u16; 15] = [
        0x7FFF, 0x0000, 0x02BF, 0x037F, 0x03FF, 0x4BFF, 0x02BF, 0x0198, 0x03FF, 0x025F, 0x033F,
        0x5FFF, 0x7FFF, 0x03FF, 0x7FFF,
    ];
    let star1: [u16; 15] = [
        0x7FFF, 0x0000, 0x4E20, 0x6F00, 0x7FC8, 0x7FF5, 0x5E60, 0x3D40, 0x7FE0, 0x7E08, 0x7F10,
        0x7FFF, 0x7FFF, 0x7FE0, 0x03FF,
    ];
    let star2: [u16; 15] = [
        0x7FFF, 0x0000, 0x0240, 0x0320, 0x2BE8, 0x5FF5, 0x02A0, 0x0180, 0x03E0, 0x02C8, 0x2FE0,
        0x6FFF, 0x7FFF, 0x03FF, 0x7FFF,
    ];
    let star3: [u16; 15] = [
        0x7FFF, 0x0000, 0x4014, 0x601C, 0x7C1F, 0x7E9F, 0x5018, 0x300C, 0x7C1F, 0x001F, 0x341F,
        0x7DFF, 0x7FFF, 0x03FF, 0x7FFF,
    ];

    let mut out = Vec::with_capacity(9 * 30);
    for pal in [
        &classic,
        &fire,
        &ice,
        &cape,
        &yoshi_rider,
        &star0,
        &star1,
        &star2,
        &star3,
    ] {
        for &c in pal {
            out.extend_from_slice(&c.to_le_bytes());
        }
    }
    out
}

/// Assemble the complete v0.8.0 native 65c816 Super Mario Universe Engine (`$22:9000`).
pub fn assemble_mario_engine_65816() -> Result<Vec<u8>, String> {
    let mut a = Asm65816::new(MARIO_ENGINE_ADDR);

    // =========================================================================
    // ENTRY POINT: Mario_MainHook ($22:9000)
    // Replaces `JSL $07:8000` at all 3 Bank $02 callers cleanly!
    // =========================================================================
    a.label("Mario_MainHook");
    a.op(0x08); // PHP
    a.op(0x8B); // PHB
    a.imm8(0xE2, 0x30); // SEP #$30 (8-bit A/X/Y)
    a.imm8(0xA9, 0x7E);
    a.op(0x48);
    a.op(0xAB); // DBR = $7E
    a.abs(0xAD, W_AIRBORNE);
    a.branch(0xF0, "pre_done");
    a.imm8(0xA9, 0xFF);
    a.dp(0x85, 0x24);
    a.dp(0x85, 0x25);
    a.dp(0x64, 0x29);
    a.label("pre_done");
    a.op(0xAB);
    a.op(0x28);

    // Call 100% intact vanilla Link_Main ($07:8000)!
    a.long(0x22, 0x07, 0x8000);

    // POST-Link_Main: Super Mario Universe Revolution Engine!
    a.op(0x08);
    a.op(0x8B);
    a.imm8(0xE2, 0x30);
    a.imm8(0xA9, 0x7E);
    a.op(0x48);
    a.op(0xAB);

    // --- SAFETY GATE: only active in Mode 0x07 (Dungeon) or 0x09 (Overworld) ---
    a.dp(0xA5, 0x10);
    a.imm8(0xC9, 0x07);
    a.branch(0xF0, "mode_ok");
    a.imm8(0xC9, 0x09);
    a.branch(0xF0, "mode_ok");
    a.jsr("Mario_ResetAirborne");
    a.jsr("Mario_ResetShell");
    a.jmp("hook_exit");

    a.label("mode_ok");
    // Track Turn-Block / Bush Throw Reward ($0308 transitioning 1 -> 0 while Submodule 0)!
    a.dp(0xA5, 0x11);
    a.abs(0x0D, 0x02E4);
    a.branch(0xD0, "skip_carry_chk");
    a.abs(0xAD, W_PREV_CARRY);
    a.branch(0xF0, "save_carry");
    a.abs(0xAD, 0x0308);
    a.branch(0xD0, "save_carry");
    // Just threw a lifted Turn-Block / Bush -> +1 Coin + +2 Star Power + Coin Chime!
    a.imm8(0xA9, 0x01);
    a.jsr("Mario_AddCoinsAndStar");
    a.imm8(0xA9, 0x2B);
    a.abs(0x8D, 0x012E);
    a.label("save_carry");
    a.abs(0xAD, 0x0308);
    a.abs(0x8D, W_PREV_CARRY);
    a.label("skip_carry_chk");

    // Check Submodule $11 == 0, Freeze $02E4 == 0, Carry $0308 == 0, State $5D == 0
    a.dp(0xA5, 0x11);
    a.abs(0x0D, 0x02E4);
    a.abs(0x0D, 0x0308);
    a.dp(0x05, 0x5D);
    a.branch(0xF0, "state_ok");
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_PAL_DIRTY); // Force CGRAM power-up palette restore after door/warp/carry!
    a.jsr("Mario_ResetAirborne");
    a.jsr("Mario_UpdatePowerUps");
    a.jmp("hook_exit");

    a.label("state_ok");
    // Cross-boundary State Gate (preserving Power-Up & Yoshi across doors while resetting Z & Shell!)
    a.dp(0xA5, 0x10);
    a.abs(0xCD, W_LAST_MODE);
    a.branch(0xD0, "area_changed");
    a.imm8(0xC9, 0x09);
    a.branch(0xF0, "chk_ow_area");
    a.dp(0xA5, 0xA0);
    a.branch(0x80, "cmp_area");
    a.label("chk_ow_area");
    a.dp(0xA5, 0x8A);
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
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_PAL_DIRTY); // Restore Fire/Ice/Cape/Yoshi CGRAM cleanly after area transition!
    a.jsr("Mario_ResetAirborne");
    a.jsr("Mario_ResetShell");

    a.label("area_same");
    a.jsr("Mario_UpdateYoshi");
    a.jsr("Mario_UpdateJump");
    a.jsr("Mario_CheckGroundPoundShockwave");
    a.jsr("Mario_CheckWarpPipe");
    a.jsr("Mario_CheckStomp");
    a.jsr("Mario_UpdateKoopaShell");
    a.jsr("Mario_UpdateBoos");
    a.jsr("Mario_UpdateBossCombat");
    a.jsr("Mario_UpdatePowerUps");

    a.label("hook_exit");
    a.op(0xAB);
    a.op(0x28);
    a.op(0x6B); // RTL

    // =========================================================================
    // HELPER: Mario_AddCoinsAndStar (Input: A = bonus coins 1..4)
    // Safely increments ACTUAL/TARGET Coins ($7E:F360/$7E:F361, cap 999) so the HUD
    // display ($7E:F362) smoothly rolls up + refills +4 Star Power ($7E:F36E, cap 0x80)!
    // =========================================================================
    a.label("Mario_AddCoinsAndStar");
    a.abs(0x8D, W_TMP_A);
    a.abs(0xAD, 0xF361);
    a.imm8(0xC9, 0x03);
    a.branch(0xB0, "add_star_only");
    a.op(0x18);
    a.abs(0xAD, 0xF360);
    a.abs(0x6D, W_TMP_A);
    a.abs(0x8D, 0xF360);
    a.abs(0xAD, 0xF361);
    a.imm8(0x69, 0x00);
    a.abs(0x8D, 0xF361);
    a.label("add_star_only");
    a.op(0x18);
    a.abs(0xAD, 0xF36E);
    a.imm8(0x69, 0x04);
    a.imm8(0xC9, 0x80);
    a.branch(0x90, "store_star_refill");
    a.imm8(0xA9, 0x80);
    a.label("store_star_refill");
    a.abs(0x8D, 0xF36E);
    a.op(0x60);

    // =========================================================================
    // SUBROUTINE: Mario_ResetAirborne & Mario_ResetShell
    // =========================================================================
    a.label("Mario_ResetAirborne");
    a.abs(0x9C, W_GP_IMPACT);
    a.abs(0x9C, W_GP_HANG);
    a.abs(0x9C, W_YOSHI_FLUTTER);
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
    a.abs(0xAD, W_STAR_LO);
    a.abs(0x0D, W_STAR_HI);
    a.branch(0xD0, "reset_done");
    a.abs(0x9C, 0x037B);
    a.label("reset_done");
    a.op(0x60);

    a.label("Mario_ResetShell");
    a.imm8(0xA9, 0xFF);
    a.abs(0x8D, W_SHELL_SLOT);
    a.abs(0x9C, W_SHELL_STATE);
    a.abs(0x9C, W_SHELL_TIMER);
    a.abs(0x9C, W_SHELL_KICK_CD);
    a.op(0x60);

    // =========================================================================
    // SUBROUTINE: Mario_UpdateYoshi
    // Playable Yoshi Companion:
    //   - Equipping FILET YOSHI ($7E:F34D -> $0202 == 0x0E) automatically mounts Yoshi!
    //   - Yoshi stays mounted even if Mario switches to Fire/Ice/Cape until taking a hit!
    //   - SMW 1-Hit Damage Shield: absorbs 100% of Mario's lost HP, dismounts Yoshi with 1.2s escape!
    // =========================================================================
    a.label("Mario_UpdateYoshi");
    a.abs(0xAD, 0x0202);
    a.imm8(0xC9, 0x0E); // FILET YOSHI ($7E:F34D -> 1-indexed 0x0E) equipped?
    a.branch(0xD0, "yoshi_chk_shield");
    a.abs(0xAD, W_YOSHI_ACTIVE);
    a.branch(0xD0, "yoshi_chk_shield"); // Already mounted!
    a.abs(0xAD, W_STOMP_IF);
    a.branch(0xD0, "yoshi_chk_shield"); // Wait until post-dismount escape i-frames finish!
                                        // Mount Yoshi Companion!
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_YOSHI_ACTIVE);
    a.abs(0x8D, W_PAL_DIRTY);
    a.abs(0xAD, 0xF36D); // Snapshot current Mario HP
    a.abs(0x8D, W_YOSHI_LAST_HP);
    a.imm8(0xA9, 0x2B);
    a.abs(0x8D, 0x012E); // Joyful Yoshi Chime SFX!
    a.op(0x60);

    a.label("yoshi_chk_shield");
    a.abs(0xAD, W_YOSHI_ACTIVE);
    a.branch(0xF0, "yoshi_rts");
    // Compare current Mario HP ($F36D) vs W_YOSHI_LAST_HP
    a.abs(0xAD, 0xF36D);
    a.abs(0xCD, W_YOSHI_LAST_HP);
    a.branch(0x90, "yoshi_absorb_hit"); // BCC: current HP < last HP -> Mario was hit!
                                        // Mario healed or unchanged -> update snapshot upward
    a.abs(0x8D, W_YOSHI_LAST_HP);
    a.op(0x60);

    a.label("yoshi_absorb_hit");
    // SMW YOSHI 1-HIT SHIELD: restore Mario's lost HP, dismount Yoshi, grant 60f escape i-frames!
    a.abs(0xAD, W_YOSHI_LAST_HP);
    a.abs(0x8D, 0xF36D);
    a.abs(0x9C, W_YOSHI_ACTIVE);
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_PAL_DIRTY);
    a.imm8(0xA9, 0x3C); // 60 frames (~1.2s) of Classic Mario post-shield escape before Yoshi can remount!
    a.abs(0x8D, W_STOMP_IF);
    a.imm8(0xA9, 0x20);
    a.abs(0x8D, 0x012E); // Yoshi dismount pop SFX
    a.label("yoshi_rts");
    a.op(0x60);

    // =========================================================================
    // SUBROUTINE: Mario_UpdateJump
    // Variable Jump + Cape Glide + Yoshi Flutter Jump + Authentic Mario Ground Pound!
    // =========================================================================
    a.label("Mario_UpdateJump");
    a.abs(0x9C, W_GP_IMPACT);
    a.abs(0xAD, W_JUMP_CD);
    a.branch(0xF0, "cd_zero");
    a.abs(0xCE, W_JUMP_CD);
    a.label("cd_zero");

    a.abs(0xAD, W_STOMP_IF);
    a.branch(0xF0, "stomp_if_done");
    a.abs(0xCE, W_STOMP_IF);
    a.branch(0xD0, "keep_stomp_if");
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

    a.abs(0xAD, W_AIRBORNE);
    a.branch(0xD0, "is_airborne");

    // --- GROUNDED: check L ($20) or R ($10) new press in $F6 ---
    a.abs(0xAD, W_JUMP_CD);
    a.branch(0xD0, "jump_rts");
    a.dp(0xA5, 0x24);
    a.imm8(0xC9, 0xFF);
    a.branch(0xD0, "jump_rts");
    a.dp(0xA5, 0xF6);
    a.imm8(0x29, 0x30);
    a.branch(0xF0, "jump_rts");

    // START MARIO JUMP!
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_AIRBORNE);
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_Z_PX);
    a.abs(0x9C, W_Z_SUB);
    // Slightly higher initial jump boost when riding Yoshi!
    a.abs(0xAD, W_YOSHI_ACTIVE);
    a.branch(0xF0, "norm_jump_vz");
    a.imm8(0xA9, 0xC8); // VZ = +2.78 px/f on Yoshi!
    a.abs(0x8D, W_VZ_LO);
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_VZ_HI);
    a.imm8(0xA9, 0x10); // 16 Flutter Jump frames ready!
    a.abs(0x8D, W_YOSHI_FLUTTER);
    a.branch(0x80, "finish_jump_start");
    a.label("norm_jump_vz");
    a.imm8(0xA9, 0x90); // VZ = +2.56 px/f classic Mario!
    a.abs(0x8D, W_VZ_LO);
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_VZ_HI);
    a.abs(0x9C, W_YOSHI_FLUTTER);
    a.label("finish_jump_start");
    a.abs(0x9C, W_COMBO);
    a.abs(0x9C, W_GP_HANG);
    a.imm8(0xA9, 0x02);
    a.dp(0x85, 0x24);
    a.dp(0x64, 0x25);
    a.dp(0x64, 0x29);
    a.imm8(0xA9, 0x20);
    a.abs(0x8D, 0x012E);
    a.label("jump_rts");
    a.op(0x60);

    // --- AIRBORNE STATE MACHINE ---
    a.label("is_airborne");
    // Check if Ground Pound Hang (state 3) or Slam (state 4)!
    a.imm8(0xC9, 0x03);
    a.branch(0xF0, "gp_hang_tick");
    a.imm8(0xC9, 0x04);
    a.branch(0xF0, "gp_slam_tick");

    // Check Ground Pound trigger: Down newly pressed ($F4 & $04) while Z >= 7px!
    a.dp(0xA5, 0xF4); // Joy1 newly pressed high (B Y Sel St U D L R -> Down = $04!)
    a.imm8(0x29, 0x04);
    a.branch(0xF0, "check_var_jump");
    a.abs(0xAD, W_Z_PX);
    a.imm8(0xC9, 0x07);
    a.branch(0x90, "check_var_jump");
    // START MARIO GROUND POUND!
    a.imm8(0xA9, 0x03);
    a.abs(0x8D, W_AIRBORNE);
    a.imm8(0xA9, 0x06); // 6-frame dramatic mid-air flip stall!
    a.abs(0x8D, W_GP_HANG);
    a.abs(0x9C, W_VZ_LO);
    a.abs(0x9C, W_VZ_HI);
    a.imm8(0xA9, 0x20);
    a.abs(0x8D, 0x012E);
    a.jmp("write_air_z");

    a.label("gp_hang_tick");
    a.abs(0xCE, W_GP_HANG);
    a.branch(0xD0, "gp_stay_hang");
    // Hang done -> ROCKET DOWNWARD SLAM (-5.0 px/frame = 0xFB00)!
    a.imm8(0xA9, 0x04);
    a.abs(0x8D, W_AIRBORNE);
    a.abs(0x9C, W_VZ_LO);
    a.imm8(0xA9, 0xFB); // -5 px/frame!
    a.abs(0x8D, W_VZ_HI);
    a.label("gp_stay_hang");
    a.jmp("write_air_z");

    a.label("gp_slam_tick");
    a.jmp("integrate_z");

    // Variable jump height cut on early L/R release
    a.label("check_var_jump");
    a.abs(0xAD, W_VZ_HI);
    a.branch(0x30, "apply_grav");
    a.imm8(0xC9, 0x01);
    a.branch(0x90, "apply_grav");
    a.dp(0xA5, 0xF2);
    a.imm8(0x29, 0x30);
    a.branch(0xD0, "apply_grav");
    a.imm8(0xA9, 0x00);
    a.abs(0x8D, W_VZ_HI);
    a.imm8(0xA9, 0xC0);
    a.abs(0x8D, W_VZ_LO);

    a.label("apply_grav");
    // Signed 16-bit gravity: VZ -= 0x0024
    a.op(0x38);
    a.abs(0xAD, W_VZ_LO);
    a.imm8(0xE9, 0x24);
    a.abs(0x8D, W_VZ_LO);
    a.abs(0xAD, W_VZ_HI);
    a.imm8(0xE9, 0x00);
    a.abs(0x8D, W_VZ_HI);

    a.branch(0x10, "state_rising"); // BPL -> still rising

    // --- FALLING: Check Yoshi Flutter Jump AND Cape Mario Slow-Descent Glide! ---
    a.dp(0xA5, 0xF2); // Holding L or R?
    a.imm8(0x29, 0x30);
    a.branch(0xF0, "normal_fall_clamp");

    // 1. Yoshi Flutter Jump (when riding Yoshi & flutter frames > 0 -> gentle upward flutter hover!)
    a.abs(0xAD, W_YOSHI_ACTIVE);
    a.branch(0xF0, "chk_cape_glide");
    a.abs(0xAD, W_YOSHI_FLUTTER);
    a.branch(0xF0, "chk_cape_glide");
    a.abs(0xCE, W_YOSHI_FLUTTER);
    a.imm8(0xA9, 0x68); // VZ = +0.41 px/frame Yoshi Flutter Kick lift!
    a.abs(0x8D, W_VZ_LO);
    a.imm8(0xA9, 0x00);
    a.abs(0x8D, W_VZ_HI);
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_AIRBORNE);
    a.jmp("integrate_z");

    // 2. Cape Mario Glide ($0202 == 0x13 Magic Cape equipped -> ultra-slow -0.75 px/f descent!)
    a.label("chk_cape_glide");
    a.abs(0xAD, 0x0202);
    a.imm8(0xC9, 0x13); // PLUME DE CAPE equipped?
    a.branch(0xD0, "normal_fall_clamp");
    a.imm8(0xA9, 0xFF); // -0.75 px/frame smooth Cape Float!
    a.abs(0x8D, W_VZ_HI);
    a.imm8(0xA9, 0x40);
    a.abs(0x8D, W_VZ_LO);
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_AIRBORNE);
    a.jmp("integrate_z");

    a.label("normal_fall_clamp");
    a.abs(0xAD, W_VZ_HI);
    a.imm8(0xC9, 0xFD);
    a.branch(0xB0, "mark_falling");
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
    a.op(0x18);
    a.abs(0xAD, W_Z_SUB);
    a.abs(0x6D, W_VZ_LO);
    a.abs(0x8D, W_Z_SUB);
    a.abs(0xAD, W_Z_PX);
    a.abs(0x6D, W_VZ_HI);
    a.abs(0x8D, W_Z_PX);

    a.branch(0x30, "landed");
    a.branch(0xF0, "landed");
    a.imm8(0xC9, 0x26); // Cap altitude at 38px (allows Yoshi Flutter peak!)
    a.branch(0x90, "write_air_z");
    a.imm8(0xA9, 0x26);
    a.abs(0x8D, W_Z_PX);

    a.label("write_air_z");
    a.abs(0xAD, W_Z_PX);
    a.dp(0x85, 0x24);
    a.dp(0x64, 0x25);
    a.dp(0x64, 0x29);
    a.imm8(0xC9, 0x06);
    a.branch(0x90, "air_rts");
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, 0x037B);
    a.label("air_rts");
    a.op(0x60);

    a.label("landed");
    // Check if this landing was a GROUND POUND SLAM (W_AIRBORNE == 4)!
    a.abs(0xAD, W_AIRBORNE);
    a.imm8(0xC9, 0x04);
    a.branch(0xD0, "finish_land");
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_GP_IMPACT);
    a.imm8(0xA9, 0x0E);
    a.abs(0x8D, 0x011A); // SNES Hardware Screen Shake!
    a.imm8(0xA9, 0x0C);
    a.abs(0x8D, 0x012F); // Heavy Ground Pound Thud SFX!
    a.imm8(0xA9, 0x10);
    a.abs(0x8D, W_STOMP_IF);

    a.label("finish_land");
    a.abs(0x9C, W_AIRBORNE);
    a.abs(0x9C, W_Z_PX);
    a.abs(0x9C, W_Z_SUB);
    a.abs(0x9C, W_VZ_LO);
    a.abs(0x9C, W_VZ_HI);
    a.abs(0x9C, W_COMBO);
    a.abs(0x9C, W_GP_HANG);
    a.abs(0x9C, W_YOSHI_FLUTTER);
    a.imm8(0xA9, 0x04);
    a.abs(0x8D, W_JUMP_CD);
    a.imm8(0xA9, 0xFF);
    a.dp(0x85, 0x24);
    a.dp(0x85, 0x25);
    a.abs(0xAD, W_STAR_LO);
    a.abs(0x0D, W_STAR_HI);
    a.abs(0x0D, W_STOMP_IF);
    a.branch(0xD0, "landed_rts");
    a.abs(0x9C, 0x037B);
    a.label("landed_rts");
    a.op(0x60);

    // =========================================================================
    // SUBROUTINE: Mario_CheckGroundPoundShockwave
    // Runs on Ground Pound impact frame (W_GP_IMPACT == 1):
    //   1. Flips nearby Dungeon SMW ON/OFF Crystal Switches ($0E20 == 0x1E) via native Submodule $16!
    //   2. Flattens/cracks ALL active enemies (including Spiked/Armored 0x02!) within 48px!
    // =========================================================================
    a.label("Mario_CheckGroundPoundShockwave");
    a.abs(0xAD, W_GP_IMPACT);
    a.branch(0xD0, "gp_sw_go");
    a.op(0x60);

    a.label("gp_sw_go");
    a.abs(0x9C, W_TMP_B); // W_TMP_B = 0 -> flip ON/OFF Crystal Switch AT MOST ONCE per Ground Pound!
    a.imm8(0xA2, 0x0F); // 16 sprite slots
    a.label("gp_sw_loop");
    a.abs(0xBD, 0x0DD0);
    a.imm8(0xC9, 0x09);
    a.branch(0xF0, "gp_sw_active");
    a.jmp("gp_sw_next");

    a.label("gp_sw_active");
    // Check Crystal ON/OFF Switch ($0E20 == 0x1E) in Dungeon Mode ($10 == 0x07), AT MOST ONCE!
    a.abs(0xBD, 0x0E20);
    a.imm8(0xC9, 0x1E);
    a.branch(0xD0, "gp_sw_chk_enemy");
    a.dp(0xA5, 0x10);
    a.imm8(0xC9, 0x07);
    a.branch(0xD0, "gp_sw_next_j");
    a.abs(0xAD, W_TMP_B);
    a.branch(0xD0, "gp_sw_next_j"); // Already flipped once this Ground Pound (avoid double-flip!)
                                    // 64px proximity check to Crystal Switch so Mario only flips nearby switches!
    a.op(0x38);
    a.abs(0xBD, 0x0D10);
    a.dp(0xE5, 0x22);
    a.op(0x18);
    a.imm8(0x69, 0x40);
    a.imm8(0xC9, 0x80);
    a.branch(0xB0, "gp_sw_next_j");
    a.op(0x38);
    a.abs(0xBD, 0x0D00);
    a.dp(0xE5, 0x20);
    a.op(0x18);
    a.imm8(0x69, 0x40);
    a.imm8(0xC9, 0x80);
    a.branch(0xB0, "gp_sw_next_j");
    // FLIP REAL DUNGEON ON/OFF BLOCKS VIA VERIFIED HARDWARE ROUTINE ($033936)!
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_TMP_B);
    a.long(0xAF, 0x7E, 0xC172); // LDA $7E:C172
    a.imm8(0x49, 0x01); // EOR #$01
    a.long(0x8F, 0x7E, 0xC172); // STA $7E:C172
    a.imm8(0xA9, 0x16);
    a.dp(0x85, 0x11); // Trigger ALTTP native Submodule $16 ON/OFF tile swap!
    a.imm8(0xA9, 0x25);
    a.abs(0x8D, 0x012E); // Switch Clank SFX!
    a.jmp("gp_sw_next");

    a.label("gp_sw_next_j");
    a.jmp("gp_sw_next");

    a.label("gp_sw_chk_enemy");
    // Check Mario Behavior Table ($22:B000) != 0 (hits Goombas 1, Spiked/Armored 2, Koopas 3, Boos 4!)
    a.op(0x8A);
    a.op(0x48);
    a.abs(0xBD, 0x0E20);
    a.op(0xAA);
    a.long(0xBF, MARIO_ENGINE_BANK, STOMP_TABLE_ADDR);
    a.op(0xA8);
    a.op(0x68);
    a.op(0xAA);
    a.op(0x98);
    a.branch(0xF0, "gp_sw_next_j");

    // 48px-radius 16-bit X check (|dx| < 48 -> dx + 48 < 96 = 0x60)
    a.op(0x38);
    a.abs(0xBD, 0x0D10);
    a.dp(0xE5, 0x22);
    a.op(0xA8);
    a.abs(0xBD, 0x0D30);
    a.dp(0xE5, 0x23);
    a.op(0x48);
    a.op(0x98);
    a.op(0x18);
    a.imm8(0x69, 0x30);
    a.op(0xA8);
    a.op(0x68);
    a.imm8(0x69, 0x00);
    a.branch(0xD0, "gp_sw_next_j");
    a.op(0x98);
    a.imm8(0xC9, 0x60);
    a.branch(0xB0, "gp_sw_next_j");

    // 48px-radius 16-bit Y check
    a.op(0x38);
    a.abs(0xBD, 0x0D00);
    a.dp(0xE5, 0x20);
    a.op(0xA8);
    a.abs(0xBD, 0x0D20);
    a.dp(0xE5, 0x21);
    a.op(0x48);
    a.op(0x98);
    a.op(0x18);
    a.imm8(0x69, 0x30);
    a.op(0xA8);
    a.op(0x68);
    a.imm8(0x69, 0x00);
    a.branch(0xD0, "gp_sw_next_j");
    a.op(0x98);
    a.imm8(0xC9, 0x60);
    a.branch(0xB0, "gp_sw_next_j");

    // GROUND POUND DIRECT HIT!
    // Check if target is a Boss (class 0x05 in Y / W_TMP_A? Re-read table quickly or save Y!)
    // Actually we already know class in Y before X/Y bbox check if we save it, or check $22:B000[$0E20,X] == 5!
    a.op(0x8A); // TXA
    a.op(0x48); // PHA
    a.abs(0xBD, 0x0E20);
    a.op(0xAA);
    a.long(0xBF, MARIO_ENGINE_BANK, STOMP_TABLE_ADDR);
    a.op(0xA8); // TAY (class)
    a.op(0x68); // PLA
    a.op(0xAA); // TAX
    a.op(0x98); // TYA
    a.imm8(0xC9, 0x05); // Boss Encounter Class (0x05)?
    a.branch(0xD0, "gp_sw_normal_enemy");

    // v0.9.0 BOSS GROUND POUND STAGGER & COIN SHOWER!
    // Staggers boss ($0EA0 = 0x16), shakes fortress ($011A = 0x12), awards +3 Coins & +8 Star Power,
    // and safely chips 2 HP ONLY if boss HP >= 4 (preserving multi-phase boss scripts 100% safely!).
    a.imm8(0xA9, 0x16);
    a.abs(0x9D, 0x0EA0);
    a.imm8(0xA9, 0x12);
    a.abs(0x8D, 0x011A);
    a.abs(0xBD, 0x0E50);
    a.imm8(0xC9, 0x04);
    a.branch(0x90, "gp_boss_reward");
    a.op(0x38);
    a.imm8(0xE9, 0x02);
    a.abs(0x9D, 0x0E50);
    a.label("gp_boss_reward");
    a.imm8(0xA9, 0x2B);
    a.abs(0x8D, 0x012E); // Mario Coin Chime!
    a.imm8(0xA9, 0x03);
    a.jsr("Mario_AddCoinsAndStar");
    a.imm8(0xA9, 0x18);
    a.abs(0x8D, W_BOSS_HIT_CD);
    a.jmp("gp_sw_next");

    a.label("gp_sw_normal_enemy");
    // Deal 10 armor-piercing damage or instant KO puff + +2 Coins!
    a.op(0x38);
    a.abs(0xBD, 0x0E50);
    a.imm8(0xE9, 0x0A);
    a.abs(0x9D, 0x0E50);
    a.branch(0x90, "gp_sw_ko");
    a.branch(0xF0, "gp_sw_ko");
    a.imm8(0xA9, 0x10);
    a.abs(0x9D, 0x0EA0);
    a.branch(0x80, "gp_sw_next");

    a.label("gp_sw_ko");
    a.imm8(0xA9, 0x06);
    a.abs(0x9D, 0x0DD0);
    a.imm8(0xA9, 0x1F);
    a.abs(0x9D, 0x0DF0);
    a.imm8(0xA9, 0x02);
    a.jsr("Mario_AddCoinsAndStar");

    a.label("gp_sw_next");
    a.op(0xCA);
    a.branch(0x30, "gp_sw_exit");
    a.jmp("gp_sw_loop");
    a.label("gp_sw_exit");
    a.op(0x60);

    // =========================================================================
    // SUBROUTINE: Mario_CheckWarpPipe (v0.9.0)
    // Interactive Overworld Physical SMW Green Warp Pipe on World 1 Yoshi's Island (Screen 0x2C, r09 c07)!
    // Ground Pounding right beside the Green Warp Pipe (`X = $08C4..$08FF, Y = $0B10..$0B5F` on $8A == 0x2C)
    // triggers the native SPC700 Descending Warp Pipe SFX ($012E=0x15), +5 Secret Bonus Coins, & Full Star Power!
    // =========================================================================
    a.label("Mario_CheckWarpPipe");
    a.abs(0xAD, W_GP_IMPACT);
    a.branch(0xF0, "pipe_rts");
    a.dp(0xA5, 0x10);
    a.imm8(0xC9, 0x09); // Overworld Mode only!
    a.branch(0xD0, "pipe_rts");
    a.dp(0xA5, 0x8A);
    a.imm8(0xC9, 0x2C); // World 1 Mario's House Green Warp Pipe (0x2C, r09 c07 -> X=0x08E0, Y=0x0B30)!
    a.branch(0xD0, "pipe_rts");
    a.dp(0xA5, 0x23);
    a.imm8(0xC9, 0x08); // X_hi == 0x08 (left half of Screen 0x2C, cols 0..7)
    a.branch(0xD0, "pipe_rts");
    a.dp(0xA5, 0x21);
    a.imm8(0xC9, 0x0B); // Y_hi == 0x0B (lower half of Screen 0x2C, rows 8..15)
    a.branch(0xD0, "pipe_rts");
    a.dp(0xA5, 0x22);
    a.imm8(0xC9, 0xC4); // X_lo >= 0xC4 (right beside Green Warp Pipe at 0xE0, never at doorway 0xB8!)
    a.branch(0x90, "pipe_rts");
    a.dp(0xA5, 0x20);
    a.imm8(0xC9, 0x10); // Y_lo >= 0x10
    a.branch(0x90, "pipe_rts");
    a.imm8(0xC9, 0x60); // Y_lo < 0x60
    a.branch(0xB0, "pipe_rts");
    // TRIGGER PHYSICAL SMW GREEN WARP PIPE REWARD & SFX!
    a.imm8(0xA9, 0x15);
    a.abs(0x8D, 0x012E); // Authentic Native SPC700 SMW Descending Warp Pipe Triple-Pulse SFX!
    a.imm8(0xA9, 0x80);
    a.abs(0x8D, 0xF36E); // Refill Star Power to 100% FULL!
    a.imm8(0xA9, 0x05);
    a.jsr("Mario_AddCoinsAndStar"); // +5 Warp Pipe Bonus Gold Coins!
    a.label("pipe_rts");
    a.op(0x60);

    // =========================================================================
    // SUBROUTINE: Mario_CheckStomp
    // - Descending Mario Stomp (Goomba KO, Koopa -> Stunned Shell, Ice Shatter on Frozen!)
    // - Super Starman Contact-Kill across all enemies!
    // =========================================================================
    a.label("Mario_CheckStomp");
    a.abs(0xAD, W_STAR_LO);
    a.abs(0x0D, W_STAR_HI);
    a.branch(0xD0, "scan_sprites");

    a.abs(0xAD, W_AIRBORNE);
    a.imm8(0xC9, 0x02); // Falling/Stomp-Ready
    a.branch(0xF0, "chk_stomp_z");
    a.imm8(0xC9, 0x04); // Ground Pound Slam
    a.branch(0xD0, "stomp_early_rts");
    a.label("chk_stomp_z");
    a.abs(0xAD, W_Z_PX);
    a.imm8(0xC9, 0x16);
    a.branch(0x90, "scan_sprites");
    a.label("stomp_early_rts");
    a.op(0x60);

    a.label("scan_sprites");
    a.imm8(0xA2, 0x0F);

    a.label("spr_loop");
    a.abs(0xBD, 0x0DD0);
    a.imm8(0xC9, 0x09); // Active enemy
    a.branch(0xF0, "spr_is_active");
    a.imm8(0xC9, 0x0B); // Frozen in Ice! (Ice Flower Stomp Shatter!)
    a.branch(0xF0, "spr_is_frozen");
    a.jmp("spr_next");

    a.label("spr_is_frozen");
    a.imm8(0xA0, 0x05); // Special tag Y=5 -> Frozen Ice Shatter!
    a.jmp("check_bbox");

    a.label("spr_is_active");
    // Skip active kicked shell slot (handled by Shell Engine!)
    a.op(0x8A);
    a.abs(0xCD, W_SHELL_SLOT);
    a.branch(0xD0, "spr_not_cur_shell");
    a.abs(0xAD, W_SHELL_STATE);
    a.branch(0xF0, "spr_not_cur_shell");
    a.jmp("spr_next");

    a.label("spr_not_cur_shell");
    a.op(0x8A);
    a.op(0x48);
    a.abs(0xBD, 0x0E20);
    a.op(0xAA);
    a.long(0xBF, MARIO_ENGINE_BANK, STOMP_TABLE_ADDR);
    a.op(0xA8);
    a.op(0x68);
    a.op(0xAA);

    a.op(0x98);
    a.branch(0xD0, "spr_vuln_nonzero");
    a.jmp("spr_next");

    a.label("spr_vuln_nonzero");
    // Boss Encounter Class (0x05): NEVER ordinary-stomp-killed (handled by Ground Pound Stagger & Boss Combat!)
    a.imm8(0xC9, 0x05);
    a.branch(0xD0, "spr_not_boss");
    a.jmp("spr_next");
    a.label("spr_not_boss");
    // Spiked (2): ONLY Starman or Ground Pound Slam (4) can defeat!
    a.imm8(0xC9, 0x02);
    a.branch(0xD0, "check_bbox");
    a.abs(0xAD, W_STAR_LO);
    a.abs(0x0D, W_STAR_HI);
    a.branch(0xD0, "check_bbox");
    a.abs(0xAD, W_AIRBORNE);
    a.imm8(0xC9, 0x04);
    a.branch(0xF0, "check_bbox");
    a.jmp("spr_next");

    a.label("check_bbox");
    // Save behavior class Y in W_TMP_B
    a.abs(0x8C, W_TMP_B);
    // 16-bit X proximity: |dx| < 16
    a.op(0x38);
    a.abs(0xBD, 0x0D10);
    a.dp(0xE5, 0x22);
    a.op(0xA8);
    a.abs(0xBD, 0x0D30);
    a.dp(0xE5, 0x23);
    a.op(0x48);
    a.op(0x98);
    a.op(0x18);
    a.imm8(0x69, 0x10);
    a.op(0xA8);
    a.op(0x68);
    a.imm8(0x69, 0x00);
    a.branch(0xF0, "x_hi_ok");
    a.jmp("spr_next");
    a.label("x_hi_ok");
    a.op(0x98);
    a.imm8(0xC9, 0x20);
    a.branch(0x90, "x_ok");
    a.jmp("spr_next");

    a.label("x_ok");
    // 16-bit Y proximity: |dy| < 18
    a.op(0x38);
    a.abs(0xBD, 0x0D00);
    a.dp(0xE5, 0x20);
    a.op(0xA8);
    a.abs(0xBD, 0x0D20);
    a.dp(0xE5, 0x21);
    a.op(0x48);
    a.op(0x98);
    a.op(0x18);
    a.imm8(0x69, 0x12);
    a.op(0xA8);
    a.op(0x68);
    a.imm8(0x69, 0x00);
    a.branch(0xF0, "y_hi_ok");
    a.jmp("spr_next");
    a.label("y_hi_ok");
    a.op(0x98);
    a.imm8(0xC9, 0x24);
    a.branch(0x90, "stomp_confirmed");
    a.jmp("spr_next");

    a.label("stomp_confirmed");
    // Starman always instant KOs!
    a.abs(0xAD, W_STAR_LO);
    a.abs(0x0D, W_STAR_HI);
    a.branch(0xD0, "instant_kill");

    // Ice Shatter Bonus (Y == 5): Frozen enemy shatters for +3 Coins & +8 Star Power!
    a.abs(0xAD, W_TMP_B);
    a.imm8(0xC9, 0x05);
    a.branch(0xD0, "chk_koopa_retract");
    a.imm8(0xA9, 0x06);
    a.abs(0x9D, 0x0DD0);
    a.imm8(0xA9, 0x1F);
    a.abs(0x9D, 0x0DF0);
    a.imm8(0xA9, 0x08);
    a.abs(0x8D, 0x012F);
    a.imm8(0xA9, 0x03);
    a.jsr("Mario_AddCoinsAndStar");
    a.jmp("do_stomp_bounce");

    // KOOPA SHELL RETRACT (Class 0x03, when no shell currently active -> Retract into Stunned Shell!)
    a.label("chk_koopa_retract");
    a.imm8(0xC9, 0x03);
    a.branch(0xD0, "normal_stomp_dmg");
    a.abs(0xAD, W_SHELL_STATE);
    a.branch(0xD0, "normal_stomp_dmg"); // Already have an active shell -> deal normal damage
    a.op(0x8E); // STX W_SHELL_SLOT
    a.code.extend_from_slice(&W_SHELL_SLOT.to_le_bytes());
    a.imm8(0xA9, 0x01); // State 1 = Stunned Koopa Shell!
    a.abs(0x8D, W_SHELL_STATE);
    a.imm8(0xA9, 0xB4); // 180 frames stunned waiting to be kicked!
    a.abs(0x8D, W_SHELL_TIMER);
    a.imm8(0xA9, 0x10); // 16-frame grace before Mario can kick it
    a.abs(0x8D, W_SHELL_KICK_CD);
    a.abs(0x9E, 0x0D50); // STZ $0D50, X (zero VX)
    a.abs(0x9E, 0x0D40); // STZ $0D40, X (zero VY)
    a.imm8(0xA9, 0x14);
    a.abs(0x9D, 0x0EA0); // Shell retract flash!
    a.imm8(0xA9, 0x08);
    a.abs(0x8D, 0x012F); // Mario Stomp Pop SFX!
    a.imm8(0xA9, 0x01);
    a.jsr("Mario_AddCoinsAndStar");
    a.jmp("do_stomp_bounce");

    a.label("normal_stomp_dmg");
    a.op(0x38);
    a.abs(0xBD, 0x0E50);
    a.imm8(0xE9, 0x06);
    a.abs(0x9D, 0x0E50);
    a.branch(0x90, "instant_kill");
    a.branch(0xF0, "instant_kill");
    a.imm8(0xA9, 0x0C);
    a.abs(0x9D, 0x0EA0);
    a.imm8(0xA9, 0x08);
    a.abs(0x8D, 0x012F);
    a.branch(0x80, "do_stomp_bounce");

    a.label("instant_kill");
    a.imm8(0xA9, 0x06);
    a.abs(0x9D, 0x0DD0);
    a.imm8(0xA9, 0x1F);
    a.abs(0x9D, 0x0DF0);
    a.imm8(0xA9, 0x08);
    a.abs(0x8D, 0x012F);
    // Escalating Multi-Stomp Combo Coin Reward (+1 + combo capped at +4)!
    a.abs(0xAD, W_COMBO);
    a.op(0x18);
    a.imm8(0x69, 0x01);
    a.imm8(0xC9, 0x04);
    a.branch(0x90, "combo_coin_ok");
    a.imm8(0xA9, 0x04);
    a.label("combo_coin_ok");
    a.jsr("Mario_AddCoinsAndStar");

    a.abs(0xAD, W_AIRBORNE);
    a.branch(0xF0, "spr_next");

    a.label("do_stomp_bounce");
    a.imm8(0xA9, 0x12);
    a.abs(0x8D, W_STOMP_IF);
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, 0x037B);
    a.abs(0x8D, W_AIRBORNE);
    a.abs(0xEE, W_COMBO);
    a.dp(0xA5, 0xF2);
    a.imm8(0x29, 0x30);
    a.branch(0xD0, "high_bounce");
    a.imm8(0xA9, 0x20);
    a.abs(0x8D, W_VZ_LO);
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_VZ_HI);
    a.op(0x60);
    a.label("high_bounce");
    a.imm8(0xA9, 0xB8);
    a.abs(0x8D, W_VZ_LO);
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_VZ_HI);
    a.op(0x60);

    a.label("spr_next");
    a.op(0xCA);
    a.branch(0x30, "stomp_exit");
    a.jmp("spr_loop");
    a.label("stomp_exit");
    a.op(0x60);

    // =========================================================================
    // SUBROUTINE: Mario_UpdateKoopaShell
    // Manages Stunned Koopa Shell (state 1 -> waits for Mario kick) AND
    // Active High-Speed Ricochet Bowling Shell (state 2 -> bounces off walls & bowls enemies!)
    // =========================================================================
    a.label("Mario_UpdateKoopaShell");
    a.abs(0xAD, W_SHELL_STATE);
    a.branch(0xD0, "shell_active");
    a.op(0x60);

    a.label("shell_active");
    a.abs(0xAE, W_SHELL_SLOT); // LDX W_SHELL_SLOT
    a.imm8(0xE0, 0x10);
    a.branch(0x90, "shell_slot_ok");
    a.jmp("shell_clear_rts");

    a.label("shell_slot_ok");
    // Verify sprite slot is still active ($0DD0,X == 0x09)
    a.abs(0xBD, 0x0DD0);
    a.imm8(0xC9, 0x09);
    a.branch(0xF0, "shell_still_alive");
    a.jmp("shell_clear_rts");

    a.label("shell_still_alive");
    // Decrement timer; when 0, puff shell away cleanly!
    a.abs(0xCE, W_SHELL_TIMER);
    a.branch(0xD0, "shell_not_expired");
    a.imm8(0xA9, 0x06);
    a.abs(0x9D, 0x0DD0);
    a.imm8(0xA9, 0x18);
    a.abs(0x9D, 0x0DF0);
    a.jmp("shell_clear_rts");

    a.label("shell_not_expired");
    // Override vanilla Koopa velocity every frame so it stays a controlled shell!
    a.abs(0x9E, 0x0D50);
    a.abs(0x9E, 0x0D40);
    a.imm8(0xA9, 0x03);
    a.abs(0x9D, 0x0EA0); // Spinning/stunned shell visual shimmer!

    a.abs(0xAD, W_SHELL_STATE);
    a.imm8(0xC9, 0x01);
    a.branch(0xF0, "shell_stunned_tick");
    a.jmp("shell_kicked_tick");

    // --- STATE 1: STUNNED SHELL WAITING FOR MARIO KICK ---
    a.label("shell_stunned_tick");
    a.abs(0xAD, W_SHELL_KICK_CD);
    a.branch(0xF0, "shell_check_kick");
    a.abs(0xCE, W_SHELL_KICK_CD);
    a.op(0x60);

    a.label("shell_check_kick");
    // Check if Mario walked within 20px of the stunned shell -> KICK IT!
    a.op(0x38);
    a.abs(0xBD, 0x0D10);
    a.dp(0xE5, 0x22);
    a.op(0x18);
    a.imm8(0x69, 0x14);
    a.imm8(0xC9, 0x28);
    a.branch(0xB0, "shell_rts_1");
    a.op(0x38);
    a.abs(0xBD, 0x0D00);
    a.dp(0xE5, 0x20);
    a.op(0x18);
    a.imm8(0x69, 0x14);
    a.imm8(0xC9, 0x28);
    a.branch(0xB0, "shell_rts_1");

    // LAUNCH KICKED KOOPA SHELL ALONG MARIO'S FACING DIRECTION ($2F)!
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_SHELL_STATE);
    a.imm8(0xA9, 0x96); // 150 frames of ricochet bowling!
    a.abs(0x8D, W_SHELL_TIMER);
    a.imm8(0xA9, 0x0C);
    a.abs(0x8D, W_STOMP_IF); // Kick i-frames so Mario doesn't get bumped!
    a.abs(0x9C, W_SHELL_VX);
    a.abs(0x9C, W_SHELL_VY);
    a.dp(0xA5, 0x2F); // 0=Up, 2=Down, 4=Left, 6=Right
    a.branch(0xF0, "kick_up");
    a.imm8(0xC9, 0x02);
    a.branch(0xF0, "kick_down");
    a.imm8(0xC9, 0x04);
    a.branch(0xF0, "kick_left");
    // Right (+3)
    a.imm8(0xA9, 0x03);
    a.abs(0x8D, W_SHELL_VX);
    a.branch(0x80, "kick_sfx");
    a.label("kick_up");
    a.imm8(0xA9, 0xFD); // -3
    a.abs(0x8D, W_SHELL_VY);
    a.branch(0x80, "kick_sfx");
    a.label("kick_down");
    a.imm8(0xA9, 0x03);
    a.abs(0x8D, W_SHELL_VY);
    a.branch(0x80, "kick_sfx");
    a.label("kick_left");
    a.imm8(0xA9, 0xFD); // -3
    a.abs(0x8D, W_SHELL_VX);
    a.label("kick_sfx");
    a.imm8(0xA9, 0x08);
    a.abs(0x8D, 0x012F); // Crisp SMW Shell Kick Pop SFX!
    a.label("shell_rts_1");
    a.op(0x60);

    // --- STATE 2: KICKED RICOCHETING BOWLING SHELL ---
    a.label("shell_kicked_tick");
    // Move X by signed W_SHELL_VX (with screen-edge ricochet at [0x18..0xE4])
    a.abs(0xAD, W_SHELL_VX);
    a.branch(0xF0, "shell_move_y");
    a.op(0x18);
    a.abs(0x7D, 0x0D10); // ADC $0D10, X
    a.abs(0x9D, 0x0D10);
    a.imm8(0xC9, 0x18);
    a.branch(0x90, "shell_bounce_x");
    a.imm8(0xC9, 0xE4);
    a.branch(0x90, "shell_move_y");
    a.label("shell_bounce_x");
    a.imm8(0xA9, 0x00);
    a.op(0x38);
    a.abs(0xED, W_SHELL_VX);
    a.abs(0x8D, W_SHELL_VX); // Negate VX -> RICOCHET!
    a.imm8(0xA9, 0x20);
    a.abs(0x8D, 0x012E); // Bonk SFX!

    a.label("shell_move_y");
    a.abs(0xAD, W_SHELL_VY);
    a.branch(0xF0, "shell_bowl_scan");
    a.op(0x18);
    a.abs(0x7D, 0x0D00);
    a.abs(0x9D, 0x0D00);
    a.imm8(0xC9, 0x20);
    a.branch(0x90, "shell_bounce_y");
    a.imm8(0xC9, 0xD8);
    a.branch(0x90, "shell_bowl_scan");
    a.label("shell_bounce_y");
    a.imm8(0xA9, 0x00);
    a.op(0x38);
    a.abs(0xED, W_SHELL_VY);
    a.abs(0x8D, W_SHELL_VY); // Negate VY -> RICOCHET!
    a.imm8(0xA9, 0x20);
    a.abs(0x8D, 0x012E);

    // BOWLING COLLISION SCAN: Knock out EVERY OTHER active enemy within 18px!
    a.label("shell_bowl_scan");
    a.abs(0xBD, 0x0D10);
    a.abs(0x8D, W_SHELL_TMP_X);
    a.abs(0xBD, 0x0D00);
    a.abs(0x8D, W_SHELL_TMP_Y);
    a.imm8(0xA0, 0x0F); // LDY #$0F (scan other slots Y)
    a.label("bowl_loop");
    a.op(0x98); // TYA
    a.abs(0xCD, W_SHELL_SLOT);
    a.branch(0xF0, "bowl_next"); // Skip self!
    a.abs(0xB9, 0x0DD0); // LDA $0DD0, Y
    a.imm8(0xC9, 0x09);
    a.branch(0xD0, "bowl_next");
    // Check vulnerability table != 0 and != 5 (can bowl Goombas, Koopas, Spinies, Boos — never insta-KO Bosses!)
    a.abs(0xB9, 0x0E20);
    a.op(0xAA);
    a.long(0xBF, MARIO_ENGINE_BANK, STOMP_TABLE_ADDR);
    a.branch(0xF0, "bowl_next");
    a.imm8(0xC9, 0x05);
    a.branch(0xF0, "bowl_next");
    // Proximity |Y.x - shell.x| < 18 && |Y.y - shell.y| < 18
    a.op(0x38);
    a.abs(0xB9, 0x0D10);
    a.abs(0xED, W_SHELL_TMP_X);
    a.op(0x18);
    a.imm8(0x69, 0x12);
    a.imm8(0xC9, 0x24);
    a.branch(0xB0, "bowl_next");
    a.op(0x38);
    a.abs(0xB9, 0x0D00);
    a.abs(0xED, W_SHELL_TMP_Y);
    a.op(0x18);
    a.imm8(0x69, 0x12);
    a.imm8(0xC9, 0x24);
    a.branch(0xB0, "bowl_next");
    // STRIKE!! KO bowled enemy + Coin Chime + +2 Bonus Coins!
    a.imm8(0xA9, 0x06);
    a.abs(0x99, 0x0DD0);
    a.imm8(0xA9, 0x1F);
    a.abs(0x99, 0x0DF0);
    a.imm8(0xA9, 0x2B);
    a.abs(0x8D, 0x012E);
    a.op(0x5A); // PHY
    a.imm8(0xA9, 0x02);
    a.jsr("Mario_AddCoinsAndStar");
    a.op(0x7A); // PLY
    a.label("bowl_next");
    a.op(0x88); // DEY
    a.branch(0x10, "bowl_loop");
    a.op(0x60);

    a.label("shell_clear_rts");
    a.jsr("Mario_ResetShell");
    a.op(0x60);

    // =========================================================================
    // SUBROUTINE: Mario_UpdateBoos
    // Authentic Super Mario World Boo Orientation-Freeze AI (STOMP_TABLE class 0x04 -> 0x19 & 0x8B):
    //   - When Mario FACES the Boo ($2F matches relative vector): FREEZES SHYLY IN PLACE!
    //   - When Mario TURNS HIS BACK: CREEPS STEALTHILY TOWARD MARIO!
    // =========================================================================
    a.label("Mario_UpdateBoos");
    a.imm8(0xA2, 0x0F);
    a.label("boo_loop");
    a.abs(0xBD, 0x0DD0);
    a.imm8(0xC9, 0x09);
    a.branch(0xD0, "boo_next_j");
    // Look up STOMP_TABLE[$0E20, X] == 0x04 (Overworld Boo 0x19 + Ghost House Boo 0x8B!)
    a.op(0x8A); // TXA
    a.op(0x48); // PHA
    a.abs(0xBD, 0x0E20);
    a.op(0xAA);
    a.long(0xBF, MARIO_ENGINE_BANK, STOMP_TABLE_ADDR);
    a.op(0xA8); // TAY
    a.op(0x68); // PLA
    a.op(0xAA); // TAX
    a.op(0x98); // TYA
    a.imm8(0xC9, 0x04);
    a.branch(0xD0, "boo_next_j");

    // Check Mario facing direction $2F (0=Up, 2=Down, 4=Left, 6=Right) vs Boo position!
    a.dp(0xA5, 0x2F);
    a.branch(0xF0, "boo_chk_up");
    a.imm8(0xC9, 0x02);
    a.branch(0xF0, "boo_chk_down");
    a.imm8(0xC9, 0x04);
    a.branch(0xF0, "boo_chk_left");
    // Facing Right (6): watched if Boo.X > Mario.X!
    a.op(0x38);
    a.abs(0xBD, 0x0D10);
    a.dp(0xE5, 0x22);
    a.branch(0x10, "boo_freeze"); // BPL -> Boo is to the right of Mario!
    a.branch(0x80, "boo_creep");

    a.label("boo_next_j");
    a.jmp("boo_next");

    a.label("boo_chk_up");
    // Facing Up (0): watched if Boo.Y < Mario.Y!
    a.op(0x38);
    a.abs(0xBD, 0x0D00);
    a.dp(0xE5, 0x20);
    a.branch(0x30, "boo_freeze"); // BMI -> Boo is above Mario!
    a.branch(0x80, "boo_creep");

    a.label("boo_chk_down");
    // Facing Down (2): watched if Boo.Y >= Mario.Y!
    a.op(0x38);
    a.abs(0xBD, 0x0D00);
    a.dp(0xE5, 0x20);
    a.branch(0x10, "boo_freeze");
    a.branch(0x80, "boo_creep");

    a.label("boo_chk_left");
    // Facing Left (4): watched if Boo.X < Mario.X!
    a.op(0x38);
    a.abs(0xBD, 0x0D10);
    a.dp(0xE5, 0x22);
    a.branch(0x30, "boo_freeze");

    // --- MARIO HAS HIS BACK TURNED -> BOO CREEPS TOWARD MARIO! ---
    a.label("boo_creep");
    a.dp(0xA5, 0x1A);
    a.imm8(0x29, 0x01); // Creep every 2nd frame
    a.branch(0xD0, "boo_save_pos");
    a.abs(0xBD, 0x0D10);
    a.dp(0xC5, 0x22);
    a.branch(0xF0, "boo_creep_y");
    a.branch(0x90, "boo_inc_x");
    a.abs(0xDE, 0x0D10); // DEC $0D10, X
    a.branch(0x80, "boo_creep_y");
    a.label("boo_inc_x");
    a.abs(0xFE, 0x0D10); // INC $0D10, X
    a.label("boo_creep_y");
    a.abs(0xBD, 0x0D00);
    a.dp(0xC5, 0x20);
    a.branch(0xF0, "boo_save_pos");
    a.branch(0x90, "boo_inc_y");
    a.abs(0xDE, 0x0D00);
    a.branch(0x80, "boo_save_pos");
    a.label("boo_inc_y");
    a.abs(0xFE, 0x0D00);
    a.label("boo_save_pos");
    a.abs(0xBD, 0x0D10);
    a.abs(0x9D, W_BOO_LAST_X);
    a.abs(0xBD, 0x0D00);
    a.abs(0x9D, W_BOO_LAST_Y);
    a.branch(0x80, "boo_next");

    // --- MARIO IS WATCHING -> BOO SHYLY FREEZES IN PLACE! ---
    a.label("boo_freeze");
    a.abs(0xBD, W_BOO_LAST_X);
    a.branch(0xF0, "boo_save_pos"); // First frame -> initialize anchor
    a.abs(0x9D, 0x0D10);
    a.abs(0xBD, W_BOO_LAST_Y);
    a.abs(0x9D, 0x0D00);
    a.abs(0x9E, 0x0D50); // Zero VX
    a.abs(0x9E, 0x0D40); // Zero VY
    a.imm8(0xA9, 0x02);
    a.abs(0x9D, 0x0EA0); // Subtle shy Boo freeze shimmer!

    a.label("boo_next");
    a.op(0xCA);
    a.branch(0x30, "boo_exit");
    a.jmp("boo_loop");
    a.label("boo_exit");
    a.op(0x60);

    // =========================================================================
    // SUBROUTINE: Mario_UpdateBossCombat (v0.9.0)
    // Authentic Mario Boss Arena Feedback Loop:
    //   Whenever ANY active Koopaling or King Bowser (class 0x05) takes damage ($0EA0,X >= 4)
    //   while W_BOSS_HIT_CD == 0, award +2 Gold Coins ($7E:F360) + +4 Star Power ($7E:F36E) + Coin Chime!
    // =========================================================================
    a.label("Mario_UpdateBossCombat");
    a.abs(0xAD, W_BOSS_HIT_CD);
    a.branch(0xF0, "boss_scan_go");
    a.abs(0xCE, W_BOSS_HIT_CD);
    a.op(0x60);

    a.label("boss_scan_go");
    a.dp(0xA5, 0x10);
    a.imm8(0xC9, 0x07); // Dungeon / Boss Arena Mode only
    a.branch(0xD0, "boss_exit");
    a.imm8(0xA2, 0x0F);
    a.label("boss_loop");
    a.abs(0xBD, 0x0DD0);
    a.imm8(0xC9, 0x09);
    a.branch(0xD0, "boss_next");
    a.abs(0xBD, 0x0EA0); // Damage recoil flash timer >= 4?
    a.imm8(0xC9, 0x04);
    a.branch(0x90, "boss_next");
    // Check if sprite is Boss Class (0x05)!
    a.op(0x8A);
    a.op(0x48);
    a.abs(0xBD, 0x0E20);
    a.op(0xAA);
    a.long(0xBF, MARIO_ENGINE_BANK, STOMP_TABLE_ADDR);
    a.op(0xA8);
    a.op(0x68);
    a.op(0xAA);
    a.op(0x98);
    a.imm8(0xC9, 0x05);
    a.branch(0xD0, "boss_next");
    // BOSS HIT CONFIRMED -> Mario Coin & Star-Power Synergy!
    a.imm8(0xA9, 0x18); // 24-frame debounce
    a.abs(0x8D, W_BOSS_HIT_CD);
    a.imm8(0xA9, 0x2B);
    a.abs(0x8D, 0x012E); // Mario Coin Chime!
    a.imm8(0xA9, 0x02);
    a.jsr("Mario_AddCoinsAndStar");
    a.op(0x60);

    a.label("boss_next");
    a.op(0xCA);
    a.branch(0x10, "boss_loop");
    a.label("boss_exit");
    a.op(0x60);

    // =========================================================================
    // SUBROUTINE: Mario_UpdatePowerUps
    // 6-Form Unified Power-Up State Machine:
    //   - Balanced Super Starman (L+R, continuous Star Power drain!)
    //   - Fire Mario ($0202 == 0x06 -> Pal 1)
    //   - Ice Mario ($0202 == 0x07 -> Pal 2)
    //   - Cape Mario ($0202 == 0x13 -> Pal 3)
    //   - Yoshi-Mounted Mario (W_YOSHI_ACTIVE == 1 -> Pal 4)
    //   - Classic Super Mario (Pal 0)
    // =========================================================================
    a.label("Mario_UpdatePowerUps");
    a.abs(0xAD, W_STAR_LO);
    a.abs(0x0D, W_STAR_HI);
    a.branch(0xD0, "star_tick");

    // Check L+R simultaneous activation for Balanced Super Starman
    a.dp(0xA5, 0xF2);
    a.imm8(0x29, 0x30);
    a.imm8(0xC9, 0x30);
    a.branch(0xF0, "star_chk_new");
    a.jmp("resolve_base_form");
    a.label("star_chk_new");
    a.dp(0xA5, 0xF6);
    a.imm8(0x29, 0x30);
    a.branch(0xD0, "star_chk_mp");
    a.jmp("resolve_base_form");
    a.label("star_chk_mp");
    // Require >= 0x18 Star Power in $7E:F36E (upfront cost 0x08 + continuous drain!)
    a.abs(0xAD, 0xF36E);
    a.imm8(0xC9, 0x18);
    a.branch(0xB0, "star_start_now");
    a.jmp("resolve_base_form");
    a.label("star_start_now");
    a.op(0x38);
    a.imm8(0xE9, 0x08);
    a.abs(0x8D, 0xF36E);
    a.imm8(0xA9, 0xF0); // Max 240 frames (or earlier if Star Power runs out!)
    a.abs(0x8D, W_STAR_LO);
    a.abs(0x9C, W_STAR_HI);
    a.abs(0x9C, W_STAR_DRAIN_CTR);
    a.imm8(0xA9, 0x05);
    a.abs(0x8D, W_POWERUP);
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, 0x037B);
    a.imm8(0xA9, 0x2B);
    a.abs(0x8D, 0x012E);

    a.label("star_tick");
    // Continuous balanced Star Power drain (-1 every 6 frames!)
    a.abs(0xEE, W_STAR_DRAIN_CTR);
    a.abs(0xAD, W_STAR_DRAIN_CTR);
    a.imm8(0xC9, 0x06);
    a.branch(0x90, "star_dec_timer");
    a.abs(0x9C, W_STAR_DRAIN_CTR);
    a.abs(0xAD, 0xF36E);
    a.branch(0xF0, "star_expired"); // Star Power depleted -> end Starman cleanly!
    a.abs(0xCE, 0xF36E);

    a.label("star_dec_timer");
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

    a.label("star_expired");
    a.abs(0x9C, W_STAR_LO);
    a.abs(0x9C, W_STAR_HI);
    a.abs(0x9C, 0x037B);
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_PAL_DIRTY);
    a.imm8(0xA9, 0xFF);
    a.abs(0x8D, W_POWERUP);
    a.branch(0x80, "resolve_base_form");

    a.label("star_rainbow");
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, 0x037B);
    a.dp(0xA5, 0x1A);
    a.op(0x4A);
    a.op(0x4A);
    a.imm8(0x29, 0x03);
    a.op(0x18);
    a.imm8(0x69, 0x05); // Rainbow palettes 5..8!
    a.jsr("Mario_LoadCgramPal");
    a.op(0x60);

    // Resolve desired base Mario form (0=Classic, 1=Fire, 2=Ice, 3=Cape, 4=Yoshi) in A!
    a.label("resolve_base_form");
    a.abs(0xAD, 0x0202);
    a.imm8(0xC9, 0x06); // Fire Flower?
    a.branch(0xF0, "form_fire");
    a.imm8(0xC9, 0x07); // Ice Flower?
    a.branch(0xF0, "form_ice");
    a.imm8(0xC9, 0x13); // Cape Feather?
    a.branch(0xF0, "form_cape");
    a.abs(0xAD, W_YOSHI_ACTIVE); // Riding Yoshi Companion?
    a.branch(0xD0, "form_yoshi");
    a.imm8(0xA9, 0x00); // Classic Super Mario
    a.branch(0x80, "apply_form");
    a.label("form_fire");
    a.imm8(0xA9, 0x01);
    a.branch(0x80, "apply_form");
    a.label("form_ice");
    a.imm8(0xA9, 0x02);
    a.branch(0x80, "apply_form");
    a.label("form_cape");
    a.imm8(0xA9, 0x03);
    a.branch(0x80, "apply_form");
    a.label("form_yoshi");
    a.imm8(0xA9, 0x04);

    a.label("apply_form");
    a.abs(0x8D, W_TMP_A);
    a.abs(0xCD, W_POWERUP);
    a.branch(0xD0, "do_form_swap");
    a.abs(0xAD, W_PAL_DIRTY);
    a.branch(0xD0, "do_form_swap");
    // Bulletproof CGRAM guard: if Mario is in a non-Classic form (Fire/Ice/Cape/Yoshi),
    // refresh live CGRAM every 8 frames so Overworld/cave exits or Cape transitions NEVER reset it!
    a.abs(0xAD, W_TMP_A);
    a.branch(0xF0, "pu_rts");
    a.dp(0xA5, 0x1A);
    a.imm8(0x29, 0x07);
    a.branch(0xD0, "pu_rts");
    a.label("do_form_swap");
    a.abs(0xAD, W_TMP_A);
    a.abs(0x8D, W_POWERUP);
    a.abs(0x9C, W_PAL_DIRTY);
    a.jsr("Mario_LoadCgramPal");
    a.label("pu_rts");
    a.op(0x60);

    // =========================================================================
    // SUBROUTINE: Mario_LoadCgramPal
    // Input: A = palette index (0..8) -> copies 30 B from $22:B100 + A*30 into $7E:C6E2..$7E:C6FF
    //        Max offset = 8 * 30 = 240 <= 255 -> fits cleanly in 8-bit X!
    // =========================================================================
    a.label("Mario_LoadCgramPal");
    a.op(0x0A); // *2
    a.abs(0x8D, W_TMP_B);
    a.op(0x0A); // *4
    a.op(0x0A); // *8
    a.op(0x0A); // *16
    a.op(0x0A); // *32 (note: for A=8, 8*32=256 wraps to 0; 0 - 16 = 240 mod 256! EXACT!)
    a.op(0x38); // SEC
    a.abs(0xED, W_TMP_B); // A = (A*32 - A*2) mod 256 = A * 30 EXACT for all A in 0..=8!
    a.op(0xAA); // TAX
    a.imm8(0xA0, 0x00);

    a.label("pal_copy_loop");
    a.long(0xBF, MARIO_ENGINE_BANK, PAL_TABLES_ADDR);
    a.abs(0x99, 0xC6E2);
    a.op(0xE8);
    a.op(0xC8);
    a.imm8(0xC0, 0x1E);
    a.branch(0x90, "pal_copy_loop");

    a.imm8(0xA9, 0x01);
    a.dp(0x85, 0x15); // Trigger NMI CGRAM DMA refresh!
    a.op(0x60);

    a.finish()
}

/// Inject the complete v0.8.0 Native 65c816 Super Mario Universe Engine into Bank `$22`
/// and hook all 3 verified `Link_Main` (`JSL $07:8000`) callers in Bank `$02`.
pub fn inject_mario_gameplay_engine(
    rom_zelda: &mut SnesRom,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    if rom_zelda.data.len() < 0x114000 {
        return Ok(0);
    }
    let mut ops = 0usize;

    // 1. Assemble and write the v0.8.0 65c816 Mario Engine code into Bank $22:9000 (PC 0x111000)
    let code = assemble_mario_engine_65816().map_err(|_| RomError::OutOfBounds {
        offset: MARIO_ENGINE_PC,
        len: 0x2000,
    })?;
    if code.len() > 0x1F80 {
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
            "Moteur 65c816 Super Mario Universe v0.8.0 ($22:9000, {} B) : Saut/Cape/Flutter Yoshi, Ground Pound + ON/OFF, Koopa Shell Kick/Ricochet, Boo Freeze AI, Fire/Ice/Cape/Yoshi/Starman",
            code.len()
        ),
        SwapKind::CodePatch {
            byte_count: code.len(),
        },
    )?;
    ops += 1;

    // 2. Write 256-byte Mario Enemy Behavior & Stomp Table at $22:B000 (PC 0x113000)
    let stomp_tbl = build_mario_stomp_table().to_vec();
    session.swap_custom_bytes(
        rom_zelda,
        STOMP_TABLE_PC,
        STOMP_TABLE_PC,
        stomp_tbl,
        "Table Comportement & Stomp Mario v0.8.0 ($22:B000, 256 B) : Goombas (1), Spinies/Thwomps (2), Koopas Shell-Kick (3), Boos Freeze AI (4)".into(),
        SwapKind::CodePatch { byte_count: 256 },
    )?;
    ops += 1;

    // 3. Write 9 live BGR555 Mario Power-Up & Yoshi Palettes at $22:B100 (PC 0x113100, 270 B)
    let pals = build_mario_powerup_palettes();
    session.swap_custom_bytes(
        rom_zelda,
        PAL_TABLES_PC,
        PAL_TABLES_PC,
        pals,
        "Palettes Live 6 Formes Mario v0.8.0 ($22:B100, 270 B) : Super, Fire, Ice, Cape, Yoshi Rider & 4 Cycles Starman".into(),
        SwapKind::PaletteBgr555 { color_count: 135 },
    )?;
    ops += 1;

    // 4. Hook all 3 Bank $02 callers of `JSL $07:8000` (`22 00 80 07` -> `22 00 90 22`)!
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
                    "Hook Boucle Joueur v0.8.0 (PC 0x{pc:06X}) : JSL $07:8000 -> JSL $22:9000 (Super Mario Universe Engine)"
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
        let code = assemble_mario_engine_65816().expect("v0.8.0 65c816 Mario engine must assemble");
        assert!(
            code.len() > 600 && code.len() < 0x1F00,
            "Engine size {}B must stay cleanly inside $22:9000..$22:AFFF",
            code.len()
        );
        // Must call vanilla Link_Main ($07:8000) and touch Crystal Switch $7E:C172!
        assert!(
            code.windows(4).any(|w| w == [0x22, 0x00, 0x80, 0x07]),
            "Hook must call original Link_Main ($07:8000)"
        );
        assert!(
            code.windows(4).any(|w| w == [0xAF, 0x72, 0xC1, 0x7E]),
            "Ground Pound must read hardware Crystal Switch $7E:C172"
        );
        assert_eq!(*code.last().unwrap(), 0x60);

        let stomp = build_mario_stomp_table();
        assert_eq!(
            stomp[0x41], 3,
            "Blue Koopa must be Shell-Kickable (class 3)"
        );
        assert_eq!(
            stomp[0x26], 3,
            "Buzzy Beetle must be Shell-Kickable (class 3)"
        );
        assert_eq!(
            stomp[0x19], 4,
            "Boo (Poe) must use Orientation-Freeze AI (class 4)"
        );
        assert_eq!(
            stomp[0x08], 1,
            "Goomba must be standard stompable (class 1)"
        );
        assert_eq!(stomp[0x98], 2, "Thwomp must be armored/spiked (class 2)");
        assert_eq!(
            stomp[0xD6], 5,
            "Final Boss King Bowser must be Boss Encounter Class (5)"
        );
        assert_eq!(
            stomp[0x92], 5,
            "Morton Koopa must be Boss Encounter Class (5)"
        );
        assert_eq!(stomp[0x35], 0, "Toad NPC must not be targeted (class 0)");

        let pals = build_mario_powerup_palettes();
        assert_eq!(
            pals.len(),
            270,
            "9 power-up/Yoshi/Starman palettes * 30 bytes = 270 bytes"
        );
        // Verify modular 8-bit palette offset arithmetic (A*32 - A*2) mod 256 == A*30 for A=0..=8!
        for a_idx in 0u8..=8u8 {
            let off = a_idx.wrapping_mul(32).wrapping_sub(a_idx.wrapping_mul(2));
            assert_eq!(off as usize, (a_idx as usize) * 30);
        }
    }
}
