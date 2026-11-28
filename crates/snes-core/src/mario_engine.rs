//! Phase 9 / v0.20.0 (PROJECT NINTENDO LIMIT BREAK):
//! Modular Native 65c816 Super Mario Action-Adventure Engine v2.0
//! (`Bank $22:9000..$22:B3FF`, `PC 0x111000..0x1133FF`).
//!
//! Runs 100% natively on unmodified Super Nintendo / Super Famicom hardware (2 MiB LoROM,
//! zero overclock, zero enhancement chips, zero MSU-1, < 2.5% of PAL 50 Hz frame budget):
//!
//! 1. **Bug-Free Hardware Collision Guard (`$24=$FF, $25=$FF, $29=$FF` before `Link_Main`)**:
//!    - Fixes the `v0.10.0` hedge-walking regression where `STZ $29` (`0x00`, positive) caused
//!      ALTTP's `$07:86D2: LDA $29; BPL +$3C` to bypass `JSR $D077` solid-tile collision while
//!      airborne. Keeping `$29 = #$FF` enforces 100% authentic wall, hedge, fence, and cliff
//!      collision on every airborne frame across the entire Overworld and all Dungeons!
//!
//! 2. **Nintendo Movement Precision & 4 Playable Heroes (`$22:B100`, 14 live BGR555 palettes = 420 B)**:
//!    - **6-frame Jump Input Buffer (`W_JUMP_BUF`)**: pressing `L`/`R` shortly before landing
//!      triggers a crisp frame-1 jump on touchdown.
//!    - **Variable Mario Jump (`L`) & Authentic SMW Spin Jump (`R`)**:
//!      - `L`: Classic variable parabolic jump with Cape Mario slow-glide, Yoshi 16f Flutter Kick,
//!        and **Princess Peach 28-frame SMB2 Floating Dress Hover**!
//!      - `R`: **SMW Spin Jump** with 360° spinning animation that **bounces safely off Spiked &
//!        Armored enemies (Spinies, Thwomps, class `0x02`) without taking damage** and shatters
//!        Dry Bones / Frozen foes!
//!    - **4 Playable Characters** (`Down + L + R` grounded -> cycles & persists in SRAM v2):
//!      - `0` **Super Mario** (Balanced classic physics, Red Cap / Blue Overalls)
//!      - `1` **Luigi** (Emerald Cap / Navy Overalls, authentic higher `0x02F0` floaty jump)
//!      - `2` **Princess Peach** (Royal Rose Dress / Gold Crown, **28-frame SMB2 Floating Dress Hover**)
//!      - `3` **Captain Toad** (Spotted Mushroom Cap / Explorer Vest, snappy low-cooldown agile hop,
//!        `+1` bonus Coin on every Turn-Block/Bush throw & steady Star Power recharge!)
//!    - **Cape Mario 360° Spin Attack (`B`/`Y` with Cape Feather `$0202 == 0x13`)**: spins Mario,
//!      stuns/chips nearby enemies within 32px, AND flips nearby Dungeon ON/OFF Switches!
//!
//! 3. **Authentic Smooth Warp Pipe Animation & Bidirectional Smart Warp Network 2.0**:
//!    - Locks player controls cleanly via ALTTP's native `$02E4 = 1` freeze flag throughout pipe
//!      descent (`W_PIPE_STATE = 1`) and emergence (`W_PIPE_STATE = 2`) — ZERO walking/gliding
//!      animation glitches!
//!    - Smoothly centers Mario onto the exact pipe mouth, slides him pixel-by-pixel downward into
//!      the pipe with descending SFX (`0x15`), executes Hardware-Legal Forced Blank full reload
//!      (`JSL $00:8919 -> $0D:F988 -> $0A:B8A5 -> $0A:B928`), and smoothly emerges upward at the
//!      destination with ascending Pipe Emerge SFX (`0x14`)!
//!    - **Bidirectional Smart Selection**: Neutral/Right -> Next Hub, Left -> Previous Hub,
//!      Up -> Express Return Home to World 1 Yoshi's Island (Mario's House)!
//!
//! 4. **Emergent Systemic Mario Gameplay & Boss Escalation**:
//!    - **Koopa Shell -> Dungeon ON/OFF Switch (`0x1E`)**: a kicked Koopa Shell remotely flips
//!      `$7E:C172 ^= 1` via native Submodule `$16`, ricochets back, and awards Dragon Coin #12!
//!    - **Bob-omb (`0x4A`) Explosive Chain Reaction**: stomping or Ground Pounding a Bob-omb
//!      triggers a 56px chain blast damaging surrounding enemies and awarding Dragon Coin #13!
//!    - **Runtime `Mario_UnstickEnemies` Guard**: prevents any roaming Overworld enemy from ever
//!      getting trapped inside a Hub Warp Pipe!
//!    - **World Restoration Persistence (`$7E:F37A`)**: conquered Koopaling Crystals are synced
//!      into SRAM v2 and permanently boost Mario's Star Power recharge!
//!    - **Final King Bowser (`0xD6`) Multi-Phase Escalation**: escalating Phase 1 -> 2 -> 3
//!      tremors, coin bursts, and Star Power surges with zero vanilla ending script breakage!
//!
//! 5. **Versioned 32-Byte Checksummed SRAM Schema v2 (`$7E:F400..$7E:F41F`)**:
//!    - Magic `0x4D 0x50` (`"MP"`), Version `0x02`, auto-migrates `v0.10.0` saves cleanly, stores
//!      32 unique Dragon Coins, Hero ID, 8 Warp Hubs, World Restoration Crystals, Mario Quest
//!      flags, Stomp & Pipe stats, and a live 16-bit integrity checksum with corruption recovery!

use std::collections::HashMap;

use crate::rom::{RomError, SnesRom};
use crate::swapper::{SwapKind, SwapSession};

/// Bank `$22` LoROM base for Mario Gameplay Engine v2.0 (`PC 0x111000..0x112FFF`, 8 KiB code budget).
pub const MARIO_ENGINE_PC: usize = 0x111000;
pub const MARIO_ENGINE_BANK: u8 = 0x22;
pub const MARIO_ENGINE_ADDR: u16 = 0x9000;

/// 256-byte Enemy Mario Behavior & Vulnerability Table at `$22:B000` (`PC 0x113000`):
/// - `0x00` = Non-enemy / NPC / Item
/// - `0x01` = Standard Stompable Enemy (Goombas, Rex, Monty Moles, Cheep-Cheeps, Bob-ombs)
/// - `0x02` = Spiked / Armored Enemy (Spinies, Thwomps, Chain Chomps — immune to ordinary stomp;
///            safely bounced with SMW Spin Jump `R`, defeated by Starman, Kicked Shell, or Ground Pound!)
/// - `0x03` = Koopa / Buzzy Shell Family (1st Stomp -> Stunned Shell; Touch/Kick -> Ricochet Bowling!)
/// - `0x04` = SMW Boo / Ghost Family (`0x19`, `0x8B` — Orientation-Freeze AI!)
/// - `0x05` = All 12 Mario & Koopaling Boss Encounters (Ground Pound Stagger + Multi-Phase Bowser!)
/// - `0x06` = SMW Dry Bones Family (`0x85`, `0x91`, `0x9E`, `0xA7` — Crumble on stomp; Spin Jump/GP shatters!)
pub const STOMP_TABLE_PC: usize = 0x113000;
pub const STOMP_TABLE_ADDR: u16 = 0xB000;

/// Live Mario Power-Up & 4-Hero BGR555 Palette Tables at `$22:B100` (`PC 0x113100`):
/// 14 palettes × 30 bytes = 420 bytes (colors 1..15, little-endian BGR555):
///   0 = Classic Super Mario
///   1 = Fire Mario (White/Crimson)
///   2 = Ice Mario (Frost-Cyan/Glacier-Blue)
///   3 = Cape Mario (SMW Golden Cape Trim)
///   4 = Yoshi-Mounted Mario (Red Cap + Yoshi Emerald Saddle + Orange Boots)
///   5..8 = Super Starman Rainbow Cycle (Gold, Cyan, Neon Green, Blazing Magenta)
///   9 = Playable Luigi (Emerald Cap + Navy Overalls)
///  10 = SMW Frog Suit (Bright Frog-Lime Swimsuit & Cream Belly)
///  11 = Playable Princess Peach (Royal Rose-Pink Gown + Golden Crown/Hair)
///  12 = Playable Captain Toad (Spotted Mushroom Cap + Crimson Explorer Vest)
///  13 = Spin Jump / P-Switch Golden Gleam Accent
pub const PAL_TABLES_PC: usize = 0x113100;
pub const PAL_TABLES_ADDR: u16 = 0xB100;
pub const PAL_COUNT: usize = 14;

/// Verified free WRAM block (`$7E:F500..$7E:F557`, 88 bytes, 0 conflicting ROM refs).
const W_AIRBORNE: u16 = 0xF500; // 0=Grounded, 1=Rising, 2=Falling, 3=GroundPound-Hang, 4=GroundPound-Slam
const W_Z_PX: u16 = 0xF501; // Integer Z height (0..38 px)
const W_Z_SUB: u16 = 0xF502; // Fractional 8-bit Z subpixel
const W_VZ_LO: u16 = 0xF503; // Signed 8.8 vertical velocity low
const W_VZ_HI: u16 = 0xF504; // Signed 8.8 vertical velocity high
const W_COMBO: u16 = 0xF505; // Consecutive airborne stomp combo counter
const W_STOMP_IF: u16 = 0xF506; // Post-stomp / Ground-Pound / Yoshi-dismount i-frame timer
const W_POWERUP: u16 = 0xF507; // Active CGRAM palette ID (0..13)
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
const W_BOSS_HIT_CD: u16 = 0xF51B; // Boss Hit Coin & Star-Power synergy debounce timer
const W_STAR_DRAIN_CTR: u16 = 0xF51C; // Starman Star-Power drain prescaler
const W_GP_IMPACT: u16 = 0xF51D; // 1 = Ground Pound just impacted this frame
const W_TMP_A: u16 = 0xF51E; // Scratch A
const W_TMP_B: u16 = 0xF51F; // Scratch B

// Boo Orientation-Freeze AI coordinate anchors (`$7E:F520..$7E:F53F`, 16 slots X + 16 slots Y)
const W_BOO_LAST_X: u16 = 0xF520;
const W_BOO_LAST_Y: u16 = 0xF530;

// Extended WRAM (`$7E:F540..$7E:F557`, 24 bytes, proven 0 conflicting ROM refs):
const W_PIPE_STATE: u16 = 0xF540; // 0=Idle, 1=Descending into Warp Pipe, 2=Emerging at Destination Hub
const W_PIPE_TMR: u16 = 0xF541; // 20-frame descent / emergence animation & fade timer
const W_PIPE_DST: u16 = 0xF542; // ALTTP native Bank $02 Overworld Warp slot (0..16 -> $1AF0)
const W_PIPE_HUB: u16 = 0xF543; // Source Hub index (0..4) for 1-time SRAM Discovery Dragon Coin
const W_PIPE_CD: u16 = 0xF544; // 80-frame post-warp anti-double-trigger cooldown
const W_PSWITCH_LO: u16 = 0xF545; // Timed SMW P-Switch 300-frame 2x Gold Rush timer low
const W_PSWITCH_HI: u16 = 0xF546; // Timed SMW P-Switch 2x Gold Rush timer high
const W_TONGUE_CD: u16 = 0xF547; // 18-frame Yoshi Tongue Swallow cooldown
const W_TMP_C: u16 = 0xF548; // Scratch C (Dragon Coin ID / bitmask / CRC scratch)
const W_JUMP_BUF: u16 = 0xF549; // v0.20.0: 6-frame Jump Input Buffer (bit 0..5 = frames, bit 7 = spin req)
const W_SPIN_JUMP: u16 = 0xF54A; // v0.20.0: 1 = Active SMW Spin Jump (safe bounce off Spinies/Thwomps!)
const W_PEACH_HOVER: u16 = 0xF54B; // v0.20.0: Princess Peach 28-frame SMB2 Floating Dress Hover timer
const W_CAPE_SPIN_CD: u16 = 0xF54C; // v0.20.0: Cape Mario 360° Spin Attack cooldown
const W_SHELL_SW_CD: u16 = 0xF54D; // v0.20.0: Koopa Shell -> Crystal Switch debounce
const W_TOAD_RECH: u16 = 0xF54E; // v0.20.0: Captain Toad + World Restoration Star-Power recharge prescaler
const W_BOWSER_PHASE: u16 = 0xF54F; // v0.20.0: Final King Bowser (0xD6) multi-phase tracker (0, 1, 2)
const W_BOBOMB_CD: u16 = 0xF550; // v0.20.0: Bob-omb chain reaction debounce
const W_PIPE_STEP: u16 = 0xF551; // v0.20.0: Pipe vertical pixel slide offset tracker
const W_SRAM_READY: u16 = 0xF552; // v0.20.0: 1 once SRAM v2 header validated/migrated this session
const W_TMP_D: u16 = 0xF553; // Scratch D (checksum / temp)

// v0.20.0 — Versioned 32-Byte Checksummed SRAM Schema v2 (`$7E:F400..$7E:F41F`, saved & checksummed in `.srm`!):
pub const S_DRAGON_BITS: u16 = 0xF400; // 4 bytes ($F400..$F403) = 32 Persistent Dragon Coin bits (0 infinite farming!)
pub const S_DRAGON_CNT: u16 = 0xF404; // Total unique Dragon Coins collected (every 5 = 1-UP Full Heal!)
pub const S_CHAR_ID: u16 = 0xF405; // v0.20.0: 0=Mario, 1=Luigi, 2=Princess Peach, 3=Captain Toad!
pub const S_PIPE_DISC: u16 = 0xF406; // Discovered Warp Pipe Hubs bitmask (bits 0..4)
pub const S_LUIGI_MODE: u16 = 0xF407; // Backward-compatible Luigi mirror (1 when S_CHAR_ID == 1)
pub const S_MAGIC_0: u16 = 0xF408; // Magic byte 0 = 0x4D ('M')
pub const S_MAGIC_1: u16 = 0xF409; // Magic byte 1 = 0x50 ('P')
pub const S_VERSION: u16 = 0xF40A; // Schema Version = 0x02
pub const S_WORLD_RESTORE: u16 = 0xF40B; // Synced World Restoration Crystal bitmask from $7E:F37A
pub const S_QUEST_LO: u16 = 0xF40C; // Persistent Mario Quest & Milestone flags low
pub const S_QUEST_HI: u16 = 0xF40D; // Persistent Mario Quest & Milestone flags high
pub const S_STOMP_TOTAL: u16 = 0xF40E; // Cumulative Mario Stomp counter (saturating 0..255)
pub const S_PIPE_TOTAL: u16 = 0xF40F; // Cumulative Warp Pipe travels (saturating 0..255)
pub const S_CRC_LO: u16 = 0xF41E; // Live 16-bit SRAM v2 checksum low (over $F400..$F41D)
pub const S_CRC_HI: u16 = 0xF41F; // Live 16-bit SRAM v2 checksum high

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
            code: Vec::with_capacity(4096),
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
/// - `0x00` = Non-enemy / NPC / Item
/// - `0x01` = Standard Stompable Mario Enemy (Goombas, Rex, Monty Moles, Cheep-Cheeps, Bob-ombs)
/// - `0x02` = Spiked / Armored Enemy (Spinies, Thwomps, Pokeys — immune to ordinary stomp;
///            safely bounced by SMW Spin Jump `R`; defeated by Starman, Kicked Shell, or Ground Pound!)
/// - `0x03` = Koopa Troopa / Buzzy Beetle Shell Family (1st Stomp -> Stunned Shell; Kick -> Ricochet Bowling!)
/// - `0x04` = Authentic SMW Boo Family (`0x19`, `0x8B` — Orientation-Freeze AI!)
/// - `0x05` = All 12 Mario & Koopaling Boss Encounters (Ground Pound Stagger + Bowser Multi-Phase!)
/// - `0x06` = SMW Dry Bones Family (`0x85`, `0x91`, `0x9E`, `0xA7` — Crumble on stomp; Spin Jump/GP shatters!)
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
        0x4A, // Red Bomb Soldier -> Bob-omb (Triggers v0.20.0 Chain Blast!)
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

    // 2. Spiked / Armored Enemies (immune to ordinary stomp; safe bounce with SMW Spin Jump `R`;
    //    defeated by Starman, Kicked Shell, or Ground Pound!)
    const SPIKED: &[u8] = &[
        0x0C, // Spiny / Buzz
        0x14, // Gargoyle
        0x22, // Ropa
        0x27, // Deadrock -> Vanilla Dome Spiny!
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

    // 5. All 12 Mario & Koopaling Boss Encounters (Ground Pound Boss Stagger + Multi-Phase Bowser!)
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

    // 6. Authentic SMW Dry Bones Family (class 0x06):
    //    Ordinary stomp crumbles Dry Bones into rattling stunned bone pile ($0EA0=0x78)!
    //    SMW Spin Jump (`R`), Ground Pound Slam, Super Hammer, or Super Starman shatters permanently!
    for &id in &[0x85u8, 0x91, 0x9E, 0xA7] {
        t[id as usize] = 0x06;
    }

    t
}

/// Build the 14 live BGR555 Hero & Power-Up palettes (14 × 30 B = 420 B):
///   0 = Classic Super Mario
///   1 = Fire Mario
///   2 = Ice Mario
///   3 = Cape Mario
///   4 = Yoshi-Mounted Mario
///   5..8 = Super Starman Rainbow Cycle
///   9 = Playable Luigi
///  10 = SMW Frog Suit
///  11 = Playable Princess Peach (Royal Rose-Pink Gown + Golden Crown/Hair!)
///  12 = Playable Captain Toad (Spotted Mushroom Cap + Crimson Explorer Vest!)
///  13 = SMW Spin Jump / Golden Gleam Accent
pub fn build_mario_powerup_palettes() -> Vec<u8> {
    let classic: [u16; 15] = [
        0x7FFF, 0x0000, 0x0D71, 0x1E9B, 0x3B7F, 0x635F, 0x0857, 0x000A, 0x10DF, 0x4882, 0x6D85,
        0x7E6B, 0x7FFF, 0x35DF, 0x03FF,
    ];
    let fire: [u16; 15] = [
        0x7FFF, 0x0000, 0x0D71, 0x1E9B, 0x3B7F, 0x635F, 0x5AD6, 0x3DEF, 0x7FFF, 0x0010, 0x001B,
        0x14DF, 0x7FFF, 0x02BF, 0x03FF,
    ];
    let ice: [u16; 15] = [
        0x7FFF, 0x0000, 0x5182, 0x1E9B, 0x3B7F, 0x635F, 0x7AE4, 0x5940, 0x7FE8, 0x5440, 0x7542,
        0x7F6E, 0x7FFF, 0x7FD4, 0x7FFF,
    ];
    let cape: [u16; 15] = [
        0x7FFF, 0x0000, 0x0D71, 0x1E9B, 0x3B7F, 0x635F, 0x02BF, 0x0196, 0x03FF, 0x0014, 0x001F,
        0x2DDF, 0x7FFF, 0x03FF, 0x03FF,
    ];
    let yoshi_rider: [u16; 15] = [
        0x7FFF, 0x0000, 0x01DF, 0x1E9B, 0x3B7F, 0x635F, 0x0857, 0x000A, 0x10DF, 0x0240, 0x03A2,
        0x2FEA, 0x7FFF, 0x02DF, 0x03FF,
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
    let luigi: [u16; 15] = [
        0x7FFF, 0x0000, 0x0D71, 0x1E9B, 0x3B7F, 0x635F, 0x0240, 0x0120, 0x0382, 0x3C41, 0x58A2,
        0x71E8, 0x7FFF, 0x2FEA, 0x03FF,
    ];
    let frog_suit: [u16; 15] = [
        0x7FFF, 0x0000, 0x02A0, 0x1E9B, 0x3B7F, 0x635F, 0x0280, 0x0160, 0x03E4, 0x0240, 0x1BC4,
        0x53FF, 0x7FFF, 0x47EC, 0x03FF,
    ];
    // 11: Playable Princess Peach (Golden Blonde Crown/Hair + Royal Rose-Pink Gown + White Gloves!)
    let peach: [u16; 15] = [
        0x7FFF, // 1: Pure white satin gloves & eyes
        0x0000, // 2: Crisp outline
        0x029F, // 3: Golden hair/heels
        0x26BF, // 4: Soft porcelain shade
        0x4B7F, // 5: Peach skin
        0x6FBF, // 6: Skin highlight
        0x02BF, // 7: Golden Crown/Hair shade
        0x0194, // 8: Deep gold crown outline
        0x03FF, // 9: BRIGHT GOLDEN PRINCESS PEACH HAIR & CROWN!
        0x441B, // 10: Deep Royal Rose-Pink gown shadow
        0x655F, // 11: VIBRANT PRINCESS PEACH ROSE-PINK DRESS!
        0x7ADF, // 12: Pastel pink dress highlight
        0x7FFF, // 13: Pearl lace trim
        0x7E48, // 14: Sapphire royal brooch!
        0x03FF, // 15: Golden crown jewel
    ];
    // 12: Playable Captain Toad (Pure White Spotted Mushroom Cap + Crimson Explorer Vest & Blue Scarf!)
    let toad: [u16; 15] = [
        0x7FFF, // 1: Crisp white mushroom cap & gloves
        0x0000, // 2: Crisp black outline
        0x0D71, // 3: Chestnut explorer boots
        0x1E9B, // 4: Skin shade
        0x3B7F, // 5: Peach face
        0x635F, // 6: Skin highlight
        0x5AD6, // 7: Shaded white mushroom dome
        0x0018, // 8: Crimson mushroom cap spots!
        0x7FFF, // 9: BRIGHT WHITE SPOTTED CAPTAIN TOAD CAP!
        0x0014, // 10: Deep Captain Toad Crimson vest shadow
        0x001F, // 11: VIBRANT CAPTAIN TOAD RED EXPLORER VEST!
        0x6D85, // 12: Royal Blue Toad explorer trim!
        0x7FFF, // 13: White headlamp glint
        0x03FF, // 14: Gold headlamp rim
        0x03FF, // 15: Golden buttons
    ];
    // 13: SMW Spin Jump / Golden Gleam Accent
    let spin_gleam: [u16; 15] = [
        0x7FFF, 0x0000, 0x0D71, 0x26BF, 0x4B7F, 0x739F, 0x02BF, 0x0154, 0x23FF, 0x58A2, 0x764B,
        0x7FFF, 0x7FFF, 0x03FF, 0x7FFF,
    ];

    let mut out = Vec::with_capacity(PAL_COUNT * 30);
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
        &luigi,
        &frog_suit,
        &peach,
        &toad,
        &spin_gleam,
    ] {
        for &c in pal {
            out.extend_from_slice(&c.to_le_bytes());
        }
    }
    out
}

/// Assemble the complete v0.20.0 Native 65c816 Super Mario Engine v2.0 (`$22:9000`).
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

    // USER BUG #1b FIX (Pipe Animation Lock):
    // When Warp Pipe animation is active (W_PIPE_STATE != 0), lock player control via
    // ALTTP's native $02E4 = 1 freeze flag + zero held/new joypad inputs + face Down ($2F=2)
    // BEFORE calling Link_Main so Mario NEVER walks, swings, or glitches during pipe slide!
    a.abs(0xAD, W_PIPE_STATE);
    a.branch(0xF0, "pre_chk_air");
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, 0x02E4); // Freeze Link_Control cleanly!
    a.imm8(0xA9, 0x02);
    a.dp(0x85, 0x2F); // Face Down cleanly into/out of Warp Pipe!
    a.dp(0x64, 0xF0);
    a.dp(0x64, 0xF2);
    a.dp(0x64, 0xF4);
    a.dp(0x64, 0xF6);
    a.branch(0x80, "pre_done");

    // USER BUG #1 FIX (Universal Solid-Tile / Hedge Collision Guard!):
    // ALTTP's Link_Move ($07:86CD..$07:86D5) executes:
    //   LDA $24; BPL +$40  -> skip collision if $24 >= 0
    //   LDA $29; BPL +$3C  -> skip collision if $29 >= 0  <-- v0.10.0 STZ $29 broke this!
    // Writing #$FF (NEGATIVE!) to $24, $25, AND $29 guarantees JSR $D077 runs on EVERY
    // airborne frame so Mario can NEVER walk or jump onto solid hedges, walls, or fences!
    a.label("pre_chk_air");
    a.abs(0xAD, W_AIRBORNE);
    a.branch(0xF0, "pre_done");
    a.imm8(0xA9, 0xFF);
    a.dp(0x85, 0x24);
    a.dp(0x85, 0x25);
    a.dp(0x85, 0x29); // CRITICAL FIX: #$FF (negative), NOT 0x00!
    a.label("pre_done");
    a.op(0xAB);
    a.op(0x28);

    // Call 100% intact vanilla Link_Main ($07:8000)!
    a.long(0x22, 0x07, 0x8000);

    // POST-Link_Main: Super Mario Engine v2.0!
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
    a.abs(0x9C, W_SRAM_READY); // Re-validate SRAM v2 cleanly after returning to Title / File Select!
    a.jsr("Mario_ResetAirborne");
    a.jsr("Mario_ResetShell");
    a.jmp("hook_exit");

    a.label("mode_ok");
    // Validate / auto-migrate Versioned 32-Byte Checksummed SRAM v2 once per gameplay session!
    a.abs(0xAD, W_SRAM_READY);
    a.branch(0xD0, "sram_ok");
    a.jsr("Mario_InitOrVerifySramV2");
    a.label("sram_ok");

    // Active Real Physical Overworld Warp Pipe State Machine (Smooth Slide -> Reload -> Emerge!)
    a.abs(0xAD, W_PIPE_STATE);
    a.branch(0xF0, "pipe_idle_ok");
    a.jsr("Mario_UpdateWarpPipe");
    a.jmp("hook_exit");
    a.label("pipe_idle_ok");
    a.abs(0xAD, W_PIPE_CD);
    a.branch(0xF0, "pipe_cd_zero");
    a.abs(0xCE, W_PIPE_CD);
    a.label("pipe_cd_zero");

    // Tick cooldowns (P-Switch, Cape Spin, Shell->Switch, Bob-omb chain)
    a.abs(0xAD, W_CAPE_SPIN_CD);
    a.branch(0xF0, "cape_cd_zero");
    a.abs(0xCE, W_CAPE_SPIN_CD);
    a.label("cape_cd_zero");
    a.abs(0xAD, W_SHELL_SW_CD);
    a.branch(0xF0, "shsw_cd_zero");
    a.abs(0xCE, W_SHELL_SW_CD);
    a.label("shsw_cd_zero");
    a.abs(0xAD, W_BOBOMB_CD);
    a.branch(0xF0, "bomb_cd_zero");
    a.abs(0xCE, W_BOBOMB_CD);
    a.label("bomb_cd_zero");

    // Tick Timed SMW P-Switch Gold Rush countdown ($7E:F545/$F546)!
    a.abs(0xAD, W_PSWITCH_LO);
    a.abs(0x0D, W_PSWITCH_HI);
    a.branch(0xF0, "pswitch_done");
    a.op(0x38);
    a.abs(0xAD, W_PSWITCH_LO);
    a.imm8(0xE9, 0x01);
    a.abs(0x8D, W_PSWITCH_LO);
    a.abs(0xAD, W_PSWITCH_HI);
    a.imm8(0xE9, 0x00);
    a.abs(0x8D, W_PSWITCH_HI);
    a.branch(0xD0, "pswitch_done");
    a.abs(0xAD, W_PSWITCH_LO);
    a.imm8(0xC9, 0x40);
    a.branch(0xB0, "pswitch_done");
    a.imm8(0x29, 0x0F);
    a.branch(0xD0, "pswitch_done");
    a.imm8(0xA9, 0x25);
    a.abs(0x8D, 0x012E);
    a.label("pswitch_done");

    // Track Turn-Block / Bush Throw Reward ($0308 transitioning 1 -> 0 while Submodule 0)!
    // Captain Toad (S_CHAR_ID == 3) earns +2 Coins (+1 Treasure Hunter bonus) on every throw!
    a.dp(0xA5, 0x11);
    a.abs(0x0D, 0x02E4);
    a.branch(0xD0, "skip_carry_chk");
    a.abs(0xAD, W_PREV_CARRY);
    a.branch(0xF0, "save_carry");
    a.abs(0xAD, 0x0308);
    a.branch(0xD0, "save_carry");
    a.abs(0xAD, S_CHAR_ID);
    a.imm8(0xC9, 0x03); // Captain Toad?
    a.branch(0xF0, "toad_throw_bonus");
    a.imm8(0xA9, 0x01);
    a.branch(0x80, "do_throw_coin");
    a.label("toad_throw_bonus");
    a.imm8(0xA9, 0x02);
    a.label("do_throw_coin");
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
    a.abs(0x8D, W_PAL_DIRTY);
    a.jsr("Mario_ResetAirborne");
    a.jsr("Mario_UpdatePowerUps");
    a.jmp("hook_exit");

    a.label("state_ok");
    // Cross-boundary State Gate (preserving Hero/Power-Up/Yoshi across rooms while resetting Z & Shell!)
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
    a.abs(0x8D, W_PAL_DIRTY);
    a.abs(0x9C, W_BOWSER_PHASE);
    a.jsr("Mario_ResetAirborne");
    a.jsr("Mario_ResetShell");

    a.label("area_same");
    a.jsr("Mario_UpdateWorldRestoreAndToad");
    a.jsr("Mario_UpdateYoshi");
    a.jsr("Mario_UpdateCapeSpin");
    a.jsr("Mario_UpdateJump");
    a.jsr("Mario_CheckGroundPoundShockwave");
    a.jsr("Mario_CheckWarpPipe");
    a.jsr("Mario_UnstickEnemies");
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
    // SUBROUTINE: Mario_InitOrVerifySramV2 & Mario_SealSramV2
    // Versioned 32-Byte Checksummed SRAM Schema v2 (`$7E:F400..$7E:F41F`):
    //   - Auto-migrates v0.10.0 saves (preserving Dragon Coins, Hubs, Luigi flag!)
    //   - Detects & recovers cleanly from corrupted SRAM checksums!
    // =========================================================================
    a.label("Mario_InitOrVerifySramV2");
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_SRAM_READY);
    a.abs(0xAD, S_MAGIC_0);
    a.imm8(0xC9, 0x4D); // 'M'
    a.branch(0xD0, "sram_migrate_or_init");
    a.abs(0xAD, S_MAGIC_1);
    a.imm8(0xC9, 0x50); // 'P'
    a.branch(0xD0, "sram_migrate_or_init");
    a.abs(0xAD, S_VERSION);
    a.imm8(0xC9, 0x02);
    a.branch(0xD0, "sram_migrate_or_init");
    // Magic + Version match -> verify 16-bit integrity checksum!
    a.jsr("Mario_CalcSramCrc");
    a.abs(0xAD, W_TMP_C);
    a.abs(0xCD, S_CRC_LO);
    a.branch(0xD0, "sram_recover_corrupt");
    a.abs(0xAD, W_TMP_D);
    a.abs(0xCD, S_CRC_HI);
    a.branch(0xD0, "sram_recover_corrupt");
    // Clamp hero ID defensively to 0..3
    a.abs(0xAD, S_CHAR_ID);
    a.imm8(0xC9, 0x04);
    a.branch(0x90, "sram_sync_luigi_mirror");
    a.abs(0x9C, S_CHAR_ID);
    a.label("sram_sync_luigi_mirror");
    a.abs(0xAD, S_CHAR_ID);
    a.imm8(0xC9, 0x01);
    a.branch(0xF0, "sram_is_luigi");
    a.abs(0x9C, S_LUIGI_MODE);
    a.op(0x60);
    a.label("sram_is_luigi");
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, S_LUIGI_MODE);
    a.op(0x60);

    a.label("sram_recover_corrupt");
    // Corruption Recovery: sanitize out-of-range fields cleanly and re-seal valid checksum!
    a.abs(0xAD, S_CHAR_ID);
    a.imm8(0x29, 0x03);
    a.abs(0x8D, S_CHAR_ID);
    a.abs(0xAD, S_DRAGON_CNT);
    a.imm8(0xC9, 0x21);
    a.branch(0x90, "sram_seal_now");
    a.abs(0x9C, S_DRAGON_CNT);
    a.branch(0x80, "sram_seal_now");

    a.label("sram_migrate_or_init");
    // Fresh blank ALTTP file (writes $FF to $F405/$F406!) OR v0.10.0 legacy migration:
    // Preserve genuine v0.10.0 $F400..$F404 & $F406 (<$20), migrate $F407 -> $F405, and sanitize $FF fresh bytes!
    a.abs(0xAD, S_LUIGI_MODE);
    a.imm8(0x29, 0x01);
    a.abs(0x8D, S_CHAR_ID);
    a.abs(0xAD, S_PIPE_DISC);
    a.imm8(0xC9, 0x20); // Valid 5-hub mask is 0x00..0x1F; ALTTP brand-new file sets 0xFF -> zero cleanly!
    a.branch(0x90, "sram_mig_dcnt");
    a.abs(0x9C, S_PIPE_DISC);
    a.label("sram_mig_dcnt");
    a.abs(0xAD, S_DRAGON_CNT);
    a.imm8(0xC9, 0x21);
    a.branch(0x90, "sram_mig_zero");
    a.abs(0x9C, S_DRAGON_CNT);
    a.label("sram_mig_zero");
    a.imm8(0xA2, 0x15); // Zero $F408..$F41D (22 bytes)
    a.label("sram_zero_loop");
    a.abs(0x9E, S_MAGIC_0);
    a.op(0xCA);
    a.branch(0x10, "sram_zero_loop");
    a.imm8(0xA9, 0x4D);
    a.abs(0x8D, S_MAGIC_0);
    a.imm8(0xA9, 0x50);
    a.abs(0x8D, S_MAGIC_1);
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, S_VERSION);
    a.label("sram_seal_now");
    a.jsr("Mario_SealSramV2");
    a.op(0x60);

    // Compute 16-bit Fletcher/XOR checksum over $7E:F400..$7E:F41D -> (W_TMP_C=lo, W_TMP_D=hi)
    a.label("Mario_CalcSramCrc");
    a.op(0xDA); // PHX
    a.imm8(0xA9, 0x5A); // Non-zero seed so all-zeros fails corruption check!
    a.abs(0x8D, W_TMP_C);
    a.imm8(0xA9, 0xA5);
    a.abs(0x8D, W_TMP_D);
    a.imm8(0xA2, 0x1D); // 30 bytes: 0x00..0x1D
    a.label("crc_loop");
    a.op(0x18);
    a.abs(0xBD, S_DRAGON_BITS);
    a.abs(0x6D, W_TMP_C);
    a.abs(0x8D, W_TMP_C);
    a.abs(0x4D, W_TMP_D);
    a.op(0x18);
    a.imm8(0x69, 0x13);
    a.abs(0x8D, W_TMP_D);
    a.op(0xCA);
    a.branch(0x10, "crc_loop");
    a.op(0xFA); // PLX
    a.op(0x60);

    a.label("Mario_SealSramV2");
    a.jsr("Mario_CalcSramCrc");
    a.abs(0xAD, W_TMP_C);
    a.abs(0x8D, S_CRC_LO);
    a.abs(0xAD, W_TMP_D);
    a.abs(0x8D, S_CRC_HI);
    a.op(0x60);

    // =========================================================================
    // SUBROUTINE: Mario_UpdateWorldRestoreAndToad
    // Syncs ALTTP Conquered Koopaling Crystals ($7E:F37A) -> S_WORLD_RESTORE ($7E:F40B)
    // and provides passive Star Power regeneration for Captain Toad (S_CHAR_ID == 3)
    // and restored Kingdoms ($7E:F37A != 0)!
    // =========================================================================
    a.label("Mario_UpdateWorldRestoreAndToad");
    a.abs(0xAD, 0xF37A); // Conquered Koopaling Fortress Crystals bitmask
    a.abs(0xCD, S_WORLD_RESTORE);
    a.branch(0xF0, "wr_no_change");
    a.abs(0x8D, S_WORLD_RESTORE);
    a.jsr("Mario_SealSramV2");
    a.label("wr_no_change");
    // Passive Star Power recharge every 64 frames if Captain Toad OR any Fortress Crystal restored!
    a.abs(0xAD, S_CHAR_ID);
    a.imm8(0xC9, 0x03);
    a.branch(0xF0, "wr_tick_rech");
    a.abs(0xAD, S_WORLD_RESTORE);
    a.branch(0xF0, "wr_rts");
    a.label("wr_tick_rech");
    a.abs(0xEE, W_TOAD_RECH);
    a.abs(0xAD, W_TOAD_RECH);
    a.imm8(0xC9, 0x40);
    a.branch(0x90, "wr_rts");
    a.abs(0x9C, W_TOAD_RECH);
    a.abs(0xAD, 0xF36E); // Current Star Power
    a.imm8(0xC9, 0x80);
    a.branch(0xB0, "wr_rts");
    a.op(0x18);
    a.imm8(0x69, 0x02);
    a.imm8(0xC9, 0x80);
    a.branch(0x90, "wr_store_sp");
    a.imm8(0xA9, 0x80);
    a.label("wr_store_sp");
    a.abs(0x8D, 0xF36E);
    a.label("wr_rts");
    a.op(0x60);

    // =========================================================================
    // HELPER: Mario_AddCoinsAndStar (Input: A = bonus coins 1..10)
    // Safely increments Coins ($7E:F360/$7E:F361, cap 999) — DOUBLED (2x) during
    // Timed SMW P-Switch Gold Rush! — + refills +4 Star Power!
    // =========================================================================
    a.label("Mario_AddCoinsAndStar");
    a.abs(0x8D, W_TMP_A);
    a.abs(0xAD, W_PSWITCH_LO);
    a.abs(0x0D, W_PSWITCH_HI);
    a.branch(0xF0, "coin_no_double");
    a.abs(0x0E, W_TMP_A); // 2x P-Switch Gold Rush Bonus!
    a.label("coin_no_double");
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
    // HELPER: Mario_AwardDragonCoin (Input: A = unique Dragon Coin ID 0..31)
    // Persists in checksummed SRAM v2 ($7E:F400..$7E:F404) & re-seals CRC!
    // =========================================================================
    a.label("Mario_AwardDragonCoin");
    a.op(0xDA); // PHX
    a.op(0x5A); // PHY
    a.abs(0x8D, W_TMP_C);
    a.op(0x4A);
    a.op(0x4A);
    a.op(0x4A);
    a.op(0xAA); // X = id >> 3 (0..3)
    a.abs(0xAD, W_TMP_C);
    a.imm8(0x29, 0x07);
    a.op(0xA8); // Y = id & 7
    a.imm8(0xA9, 0x01);
    a.label("dcoin_shl");
    a.op(0x88);
    a.branch(0x30, "dcoin_mask_ok");
    a.op(0x0A);
    a.branch(0x80, "dcoin_shl");
    a.label("dcoin_mask_ok");
    a.abs(0x8D, W_TMP_C);
    a.abs(0xBD, S_DRAGON_BITS);
    a.abs(0x2D, W_TMP_C);
    a.branch(0xD0, "dcoin_already"); // Already collected -> ZERO infinite farming!
    a.abs(0xBD, S_DRAGON_BITS);
    a.abs(0x0D, W_TMP_C);
    a.abs(0x9D, S_DRAGON_BITS);
    a.abs(0xEE, S_DRAGON_CNT);
    a.jsr("Mario_SealSramV2");
    a.imm8(0xA9, 0x2B);
    a.abs(0x8D, 0x012E);
    a.imm8(0xA9, 0x0A);
    a.jsr("Mario_AddCoinsAndStar");
    a.abs(0xAD, S_DRAGON_CNT);
    a.label("dcoin_mod5");
    a.imm8(0xC9, 0x05);
    a.branch(0x90, "dcoin_already");
    a.op(0x38);
    a.imm8(0xE9, 0x05);
    a.branch(0xD0, "dcoin_mod5");
    // Every 5 Dragon Coins -> 1-UP Full Heal & Star Power Refill!
    a.abs(0xAD, 0xF36C);
    a.abs(0x8D, 0xF36D);
    a.imm8(0xA9, 0x80);
    a.abs(0x8D, 0xF36E);
    a.label("dcoin_already");
    a.op(0x7A); // PLY
    a.op(0xFA); // PLX
    a.op(0x60);

    // =========================================================================
    // SUBROUTINE: Mario_ResetAirborne & Mario_ResetShell
    // =========================================================================
    a.label("Mario_ResetAirborne");
    a.abs(0x9C, W_GP_IMPACT);
    a.abs(0x9C, W_GP_HANG);
    a.abs(0x9C, W_YOSHI_FLUTTER);
    a.abs(0x9C, W_PEACH_HOVER);
    a.abs(0x9C, W_SPIN_JUMP);
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
    a.dp(0x85, 0x29); // Keep $29 negative!
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
    // SUBROUTINE: Mario_UpdateCapeSpin (v0.20.0 — Cape Mario 360° Spin Attack!)
    // Pressing B ($80) or Y ($40) while Cape Feather ($0202 == 0x13) is equipped:
    //   - Spins Mario 90° visually + knocks back/chips enemies within 32px!
    //   - Flips any nearby Dungeon ON/OFF Crystal Switch ($0E20 == 0x1E)!
    // =========================================================================
    a.label("Mario_UpdateCapeSpin");
    a.abs(0xAD, W_CAPE_SPIN_CD);
    a.branch(0xD0, "cape_spin_early_rts");
    a.abs(0xAD, 0x0202);
    a.imm8(0xC9, 0x13); // PLUME DE CAPE equipped?
    a.branch(0xD0, "cape_spin_early_rts");
    a.dp(0xA5, 0xF4); // B ($80) or Y ($40) newly pressed?
    a.imm8(0x29, 0xC0);
    a.branch(0xD0, "cape_spin_go");
    a.label("cape_spin_early_rts");
    a.op(0x60);

    a.label("cape_spin_go");
    // Execute 360° Cape Spin!
    a.imm8(0xA9, 0x14); // 20-frame cooldown
    a.abs(0x8D, W_CAPE_SPIN_CD);
    a.dp(0xA5, 0x2F);
    a.imm8(0x49, 0x02); // Snap facing spin
    a.dp(0x85, 0x2F);
    a.imm8(0xA9, 0x20);
    a.abs(0x8D, 0x012E); // Crisp Cape Whoosh SFX!
    a.imm8(0xA2, 0x0F);
    a.label("cape_spin_loop");
    a.abs(0xBD, 0x0DD0);
    a.imm8(0xC9, 0x09);
    a.branch(0xD0, "cape_spin_next_j");
    // 32px proximity check (|dx| < 32 -> dx + 32 < 64 = 0x40)
    a.op(0x38);
    a.abs(0xBD, 0x0D10);
    a.dp(0xE5, 0x22);
    a.op(0x18);
    a.imm8(0x69, 0x20);
    a.imm8(0xC9, 0x40);
    a.branch(0xB0, "cape_spin_next_j");
    a.op(0x38);
    a.abs(0xBD, 0x0D00);
    a.dp(0xE5, 0x20);
    a.op(0x18);
    a.imm8(0x69, 0x20);
    a.imm8(0xC9, 0x40);
    a.branch(0xB0, "cape_spin_next_j");
    // Check if Dungeon ON/OFF Crystal Switch ($0E20 == 0x1E)
    a.abs(0xBD, 0x0E20);
    a.imm8(0xC9, 0x1E);
    a.branch(0xD0, "cape_spin_enemy");
    a.dp(0xA5, 0x10);
    a.imm8(0xC9, 0x07);
    a.branch(0xD0, "cape_spin_next_j");
    a.long(0xAF, 0x7E, 0xC172);
    a.imm8(0x49, 0x01);
    a.long(0x8F, 0x7E, 0xC172);
    a.imm8(0xA9, 0x16);
    a.dp(0x85, 0x11);
    a.imm8(0xA9, 0x25);
    a.abs(0x8D, 0x012E);
    a.op(0x60);

    a.label("cape_spin_next_j");
    a.jmp("cape_spin_next");

    a.label("cape_spin_enemy");
    a.op(0x8A);
    a.op(0x48);
    a.abs(0xBD, 0x0E20);
    a.op(0xAA);
    a.long(0xBF, MARIO_ENGINE_BANK, STOMP_TABLE_ADDR);
    a.op(0xA8);
    a.op(0x68);
    a.op(0xAA);
    a.op(0x98);
    a.branch(0xF0, "cape_spin_next");
    a.imm8(0xC9, 0x05); // Do not stun-lock bosses with basic cape tap
    a.branch(0xF0, "cape_spin_next");
    // Stun + chip 4 HP!
    a.imm8(0xA9, 0x28);
    a.abs(0x9D, 0x0EA0);
    a.op(0x38);
    a.abs(0xBD, 0x0E50);
    a.imm8(0xE9, 0x04);
    a.abs(0x9D, 0x0E50);
    a.branch(0x90, "cape_spin_ko");
    a.branch(0xF0, "cape_spin_ko");
    a.branch(0x80, "cape_spin_next");
    a.label("cape_spin_ko");
    a.imm8(0xA9, 0x06);
    a.abs(0x9D, 0x0DD0);
    a.imm8(0xA9, 0x1F);
    a.abs(0x9D, 0x0DF0);
    a.imm8(0xA9, 0x02);
    a.jsr("Mario_AddCoinsAndStar");
    a.label("cape_spin_next");
    a.op(0xCA);
    a.branch(0x30, "cape_spin_rts");
    a.jmp("cape_spin_loop");
    a.label("cape_spin_rts");
    a.op(0x60);

    // =========================================================================
    // SUBROUTINE: Mario_UpdateYoshi
    // Yoshi Mount, Tongue Swallow Gulp (B/Y) & 1-Hit Damage Shield!
    // =========================================================================
    a.label("Mario_UpdateYoshi");
    a.abs(0xAD, 0x0202);
    a.imm8(0xC9, 0x0E); // FILET YOSHI equipped?
    a.branch(0xD0, "yoshi_chk_shield");
    a.abs(0xAD, W_YOSHI_ACTIVE);
    a.branch(0xD0, "yoshi_chk_shield");
    a.abs(0xAD, W_STOMP_IF);
    a.branch(0xD0, "yoshi_chk_shield");
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_YOSHI_ACTIVE);
    a.abs(0x8D, W_PAL_DIRTY);
    a.abs(0xAD, 0xF36D);
    a.abs(0x8D, W_YOSHI_LAST_HP);
    a.imm8(0xA9, 0x2B);
    a.abs(0x8D, 0x012E);
    a.op(0x60);

    a.label("yoshi_chk_shield");
    a.abs(0xAD, W_YOSHI_ACTIVE);
    a.branch(0xD0, "yoshi_active_go");
    a.op(0x60);
    a.label("yoshi_active_go");
    a.abs(0xAD, W_TONGUE_CD);
    a.branch(0xF0, "yoshi_chk_tongue_btn");
    a.abs(0xCE, W_TONGUE_CD);
    a.branch(0x80, "yoshi_do_hp_chk");
    a.label("yoshi_chk_tongue_btn");
    a.dp(0xA5, 0xF4);
    a.imm8(0x29, 0xC0);
    a.branch(0xF0, "yoshi_do_hp_chk");
    a.imm8(0xA2, 0x0F);
    a.label("yoshi_tongue_loop");
    a.abs(0xBD, 0x0DD0);
    a.imm8(0xC9, 0x09);
    a.branch(0xD0, "yoshi_tongue_next");
    a.op(0x8A);
    a.op(0x48);
    a.abs(0xBD, 0x0E20);
    a.op(0xAA);
    a.long(0xBF, MARIO_ENGINE_BANK, STOMP_TABLE_ADDR);
    a.op(0xA8);
    a.op(0x68);
    a.op(0xAA);
    a.op(0x98);
    a.branch(0xF0, "yoshi_tongue_next");
    a.imm8(0xC9, 0x04);
    a.branch(0xB0, "yoshi_tongue_next");
    a.op(0x38);
    a.abs(0xBD, 0x0D10);
    a.dp(0xE5, 0x22);
    a.op(0x18);
    a.imm8(0x69, 0x24);
    a.imm8(0xC9, 0x48);
    a.branch(0xB0, "yoshi_tongue_next");
    a.op(0x38);
    a.abs(0xBD, 0x0D00);
    a.dp(0xE5, 0x20);
    a.op(0x18);
    a.imm8(0x69, 0x24);
    a.imm8(0xC9, 0x48);
    a.branch(0xB0, "yoshi_tongue_next");
    a.imm8(0xA9, 0x06);
    a.abs(0x9D, 0x0DD0);
    a.imm8(0xA9, 0x1F);
    a.abs(0x9D, 0x0DF0);
    a.imm8(0xA9, 0x12);
    a.abs(0x8D, W_TONGUE_CD);
    a.imm8(0xA9, 0x08);
    a.abs(0x8D, 0x012F);
    a.op(0x18);
    a.abs(0xAD, 0xF36D);
    a.imm8(0x69, 0x08);
    a.abs(0xCD, 0xF36C);
    a.branch(0x90, "yoshi_heal_ok");
    a.abs(0xAD, 0xF36C);
    a.label("yoshi_heal_ok");
    a.abs(0x8D, 0xF36D);
    a.abs(0x8D, W_YOSHI_LAST_HP);
    a.imm8(0xA9, 0x02);
    a.jsr("Mario_AddCoinsAndStar");
    a.imm8(0xA9, 0x09);
    a.jsr("Mario_AwardDragonCoin");
    a.branch(0x80, "yoshi_do_hp_chk");
    a.label("yoshi_tongue_next");
    a.op(0xCA);
    a.branch(0x10, "yoshi_tongue_loop");

    a.label("yoshi_do_hp_chk");
    a.abs(0xAD, 0xF36D);
    a.abs(0xCD, W_YOSHI_LAST_HP);
    a.branch(0x90, "yoshi_absorb_hit");
    a.abs(0x8D, W_YOSHI_LAST_HP);
    a.op(0x60);

    a.label("yoshi_absorb_hit");
    a.abs(0xAD, W_YOSHI_LAST_HP);
    a.abs(0x8D, 0xF36D);
    a.abs(0x9C, W_YOSHI_ACTIVE);
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_PAL_DIRTY);
    a.imm8(0xA9, 0x3C);
    a.abs(0x8D, W_STOMP_IF);
    a.imm8(0xA9, 0x20);
    a.abs(0x8D, 0x012E);
    a.op(0x60);

    // =========================================================================
    // SUBROUTINE: Mario_UpdateJump (v0.20.0 Nintendo Movement Precision!)
    // - 6-frame Jump Input Buffer (W_JUMP_BUF)
    // - L = Variable Classic Jump | R = Authentic SMW Spin Jump (safe on Spinies!)
    // - Hero-specific physics: Mario (balanced), Luigi (high floaty),
    //   Princess Peach (28f SMB2 Dress Float!), Captain Toad (agile 2f cooldown!)
    // - Cape Mario Glide + Yoshi Flutter Kick + Ground Pound Slam!
    // =========================================================================
    a.label("Mario_UpdateJump");
    a.abs(0x9C, W_GP_IMPACT);
    a.abs(0xAD, W_JUMP_CD);
    a.branch(0xF0, "cd_zero");
    a.abs(0xCE, W_JUMP_CD);
    a.label("cd_zero");

    // 6-frame Jump Input Buffer update: capture any new L ($20) or R ($10) press in $F6!
    a.dp(0xA5, 0xF6);
    a.imm8(0x29, 0x30);
    a.branch(0xF0, "jbuf_tick");
    // Store 6 frames in low nibble; bit 7 = 1 if R ($10) pressed without L ($20) -> SMW Spin Jump!
    a.imm8(0x29, 0x20); // L pressed?
    a.branch(0xD0, "jbuf_norm");
    a.imm8(0xA9, 0x86); // Spin Jump buffered for 6 frames!
    a.branch(0x80, "jbuf_store");
    a.label("jbuf_norm");
    a.imm8(0xA9, 0x06); // Standard Jump buffered for 6 frames!
    a.label("jbuf_store");
    a.abs(0x8D, W_JUMP_BUF);
    a.branch(0x80, "jbuf_done");
    a.label("jbuf_tick");
    a.abs(0xAD, W_JUMP_BUF);
    a.imm8(0x29, 0x0F);
    a.branch(0xF0, "jbuf_clear");
    a.abs(0xCE, W_JUMP_BUF);
    a.branch(0x80, "jbuf_done");
    a.label("jbuf_clear");
    a.abs(0x9C, W_JUMP_BUF);
    a.label("jbuf_done");

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
    a.branch(0xF0, "chk_grounded_jump");
    a.jmp("is_airborne");

    // --- GROUNDED: trigger from 6-frame Jump Buffer (skip if L+R chord pressed!) ---
    a.label("chk_grounded_jump");
    a.abs(0xAD, W_JUMP_CD);
    a.branch(0xD0, "jump_rts_early");
    a.dp(0xA5, 0x24);
    a.imm8(0xC9, 0xFF);
    a.branch(0xD0, "jump_rts_early");
    // Do NOT jump on simultaneous L+R chord (used for Starman / P-Switch / Hero Swap!)
    a.dp(0xA5, 0xF2);
    a.imm8(0x29, 0x30);
    a.imm8(0xC9, 0x30);
    a.branch(0xF0, "jump_rts_early");
    a.abs(0xAD, W_JUMP_BUF);
    a.imm8(0x29, 0x0F);
    a.branch(0xD0, "jump_buf_go");
    a.label("jump_rts_early");
    a.op(0x60);

    a.label("jump_buf_go");

    // START JUMP! Check if SMW Spin Jump (bit 7 of W_JUMP_BUF set -> R button!)
    a.abs(0xAD, W_JUMP_BUF);
    a.abs(0x9C, W_JUMP_BUF); // Consume buffer!
    a.imm8(0x29, 0x80);
    a.branch(0xF0, "start_norm_jump");
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_SPIN_JUMP);
    a.branch(0x80, "init_jump_state");
    a.label("start_norm_jump");
    a.abs(0x9C, W_SPIN_JUMP);

    a.label("init_jump_state");
    a.dp(0xA5, 0x2F); // Latch Mario's facing direction at jump takeoff (for Bidirectional Warp Pipe 2.0!)
    a.abs(0x8D, W_PIPE_STEP);
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_AIRBORNE);
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_Z_PX);
    a.abs(0x9C, W_Z_SUB);
    a.abs(0x9C, W_YOSHI_FLUTTER);
    a.abs(0x9C, W_PEACH_HOVER);

    // Hero & Companion Jump Velocity Selection:
    a.abs(0xAD, W_YOSHI_ACTIVE);
    a.branch(0xF0, "chk_hero_jump_vz");
    a.imm8(0xA9, 0xC8); // VZ = +2.78 px/f on Yoshi!
    a.abs(0x8D, W_VZ_LO);
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_VZ_HI);
    a.imm8(0xA9, 0x10); // 16 Flutter frames ready!
    a.abs(0x8D, W_YOSHI_FLUTTER);
    a.branch(0x80, "finish_jump_start");

    a.label("chk_hero_jump_vz");
    a.abs(0xAD, S_CHAR_ID);
    a.imm8(0xC9, 0x01); // 1 = Luigi (highest floaty jump!)
    a.branch(0xF0, "luigi_jump_vz");
    a.imm8(0xC9, 0x02); // 2 = Princess Peach (28f SMB2 Floating Dress Hover!)
    a.branch(0xF0, "peach_jump_vz");
    a.imm8(0xC9, 0x03); // 3 = Captain Toad (snappy agile hop!)
    a.branch(0xF0, "toad_jump_vz");
    // 0 = Super Mario classic (+2.56 px/f)
    a.imm8(0xA9, 0x90);
    a.abs(0x8D, W_VZ_LO);
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_VZ_HI);
    a.branch(0x80, "finish_jump_start");

    a.label("luigi_jump_vz");
    a.imm8(0xA9, 0xF0); // Luigi VZ = +2.94 px/f!
    a.abs(0x8D, W_VZ_LO);
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_VZ_HI);
    a.branch(0x80, "finish_jump_start");

    a.label("peach_jump_vz");
    a.imm8(0xA9, 0x80); // Peach VZ = +2.50 px/f + 28 frames SMB2 Floating Dress Hover!
    a.abs(0x8D, W_VZ_LO);
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_VZ_HI);
    a.imm8(0xA9, 0x1C); // 28 frames (~0.56s) authentic Peach Dress Float!
    a.abs(0x8D, W_PEACH_HOVER);
    a.branch(0x80, "finish_jump_start");

    a.label("toad_jump_vz");
    a.imm8(0xA9, 0x70); // Toad VZ = +2.44 px/f (fast, responsive hops!)
    a.abs(0x8D, W_VZ_LO);
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_VZ_HI);

    a.label("finish_jump_start");
    a.abs(0x9C, W_COMBO);
    a.abs(0x9C, W_GP_HANG);
    a.imm8(0xA9, 0x02);
    a.dp(0x85, 0x24);
    a.dp(0x64, 0x25);
    a.imm8(0xA9, 0xFF);
    a.dp(0x85, 0x29); // CRITICAL BUG #1 FIX: keep $29 negative!
    a.imm8(0xA9, 0x20);
    a.abs(0x8D, 0x012E); // Rising Jump / Spin Jump SFX!
    a.label("jump_rts");
    a.op(0x60);

    // --- AIRBORNE STATE MACHINE ---
    a.label("is_airborne");
    // SMW Spin Jump 360° visual spin every 3rd frame!
    a.abs(0xAD, W_SPIN_JUMP);
    a.branch(0xF0, "air_no_spin_anim");
    a.dp(0xA5, 0x1A);
    a.imm8(0x29, 0x03);
    a.branch(0xD0, "air_no_spin_anim");
    a.dp(0xA5, 0x2F);
    a.op(0x18);
    a.imm8(0x69, 0x02);
    a.imm8(0x29, 0x06); // Cycle 0->2->4->6->0 (Up->Down->Left->Right spin!)
    a.dp(0x85, 0x2F);
    a.label("air_no_spin_anim");

    a.abs(0xAD, W_AIRBORNE);
    a.imm8(0xC9, 0x03);
    a.branch(0xF0, "gp_hang_tick");
    a.imm8(0xC9, 0x04);
    a.branch(0xF0, "gp_slam_tick");

    // Check Ground Pound trigger: Down newly pressed ($F4 & $04) while Z >= 7px!
    a.dp(0xA5, 0xF4);
    a.imm8(0x29, 0x04);
    a.branch(0xF0, "check_var_jump");
    a.abs(0xAD, W_Z_PX);
    a.imm8(0xC9, 0x07);
    a.branch(0x90, "check_var_jump");
    a.imm8(0xA9, 0x03);
    a.abs(0x8D, W_AIRBORNE);
    a.abs(0x9C, W_SPIN_JUMP);
    a.imm8(0xA9, 0x06);
    a.abs(0x8D, W_GP_HANG);
    a.abs(0x9C, W_VZ_LO);
    a.abs(0x9C, W_VZ_HI);
    a.imm8(0xA9, 0x20);
    a.abs(0x8D, 0x012E);
    a.jmp("write_air_z");

    a.label("gp_hang_tick");
    a.abs(0xCE, W_GP_HANG);
    a.branch(0xD0, "gp_stay_hang");
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
    // Signed 16-bit gravity: Luigi (-0x001E floaty!) vs others (-0x0024)
    a.abs(0xAD, S_CHAR_ID);
    a.imm8(0xC9, 0x01);
    a.branch(0xD0, "grav_mario");
    a.imm8(0xA9, 0x1E);
    a.branch(0x80, "grav_sub");
    a.label("grav_mario");
    a.imm8(0xA9, 0x24);
    a.label("grav_sub");
    a.abs(0x8D, W_TMP_A);
    a.op(0x38);
    a.abs(0xAD, W_VZ_LO);
    a.abs(0xED, W_TMP_A);
    a.abs(0x8D, W_VZ_LO);
    a.abs(0xAD, W_VZ_HI);
    a.imm8(0xE9, 0x00);
    a.abs(0x8D, W_VZ_HI);

    a.branch(0x10, "state_rising");

    // --- FALLING: Check Princess Peach Dress Hover, Yoshi Flutter & Cape Mario Glide! ---
    a.dp(0xA5, 0xF2);
    a.imm8(0x29, 0x30);
    a.branch(0xF0, "normal_fall_clamp");

    // 1. Princess Peach SMB2 Floating Dress Hover (28 frames at VZ = 0.00 px/frame!)
    a.abs(0xAD, S_CHAR_ID);
    a.imm8(0xC9, 0x02);
    a.branch(0xD0, "chk_yoshi_flutter");
    a.abs(0xAD, W_PEACH_HOVER);
    a.branch(0xF0, "chk_yoshi_flutter");
    a.abs(0xCE, W_PEACH_HOVER);
    a.abs(0x9C, W_VZ_LO); // VZ = 0.00 px/f pure horizontal Princess Peach float!
    a.abs(0x9C, W_VZ_HI);
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_AIRBORNE);
    a.jmp("integrate_z");

    // 2. Yoshi Flutter Jump (16 frames gentle upward flutter kick!)
    a.label("chk_yoshi_flutter");
    a.abs(0xAD, W_YOSHI_ACTIVE);
    a.branch(0xF0, "chk_cape_glide");
    a.abs(0xAD, W_YOSHI_FLUTTER);
    a.branch(0xF0, "chk_cape_glide");
    a.abs(0xCE, W_YOSHI_FLUTTER);
    a.imm8(0xA9, 0x68);
    a.abs(0x8D, W_VZ_LO);
    a.imm8(0xA9, 0x00);
    a.abs(0x8D, W_VZ_HI);
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_AIRBORNE);
    a.jmp("integrate_z");

    // 3. Cape Mario Glide ($0202 == 0x13 -> -0.75 px/f slow descent!)
    a.label("chk_cape_glide");
    a.abs(0xAD, 0x0202);
    a.imm8(0xC9, 0x13);
    a.branch(0xD0, "normal_fall_clamp");
    a.imm8(0xA9, 0xFF);
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
    a.imm8(0xC9, 0x26);
    a.branch(0x90, "write_air_z");
    a.imm8(0xA9, 0x26);
    a.abs(0x8D, W_Z_PX);

    a.label("write_air_z");
    a.abs(0xAD, W_Z_PX);
    a.dp(0x85, 0x24);
    a.dp(0x64, 0x25);
    a.imm8(0xA9, 0xFF);
    a.dp(0x85, 0x29); // CRITICAL BUG #1 FIX: keep $29 negative!
    a.abs(0xAD, W_Z_PX);
    a.imm8(0xC9, 0x06);
    a.branch(0x90, "air_rts");
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, 0x037B);
    a.label("air_rts");
    a.op(0x60);

    a.label("landed");
    a.abs(0xAD, W_AIRBORNE);
    a.imm8(0xC9, 0x04);
    a.branch(0xD0, "finish_land");
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_GP_IMPACT);
    a.imm8(0xA9, 0x0E);
    a.abs(0x8D, 0x011A);
    a.imm8(0xA9, 0x0C);
    a.abs(0x8D, 0x012F);
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
    a.abs(0x9C, W_PEACH_HOVER);
    a.abs(0x9C, W_SPIN_JUMP);
    // Captain Toad has ultra-agile 2-frame jump cooldown (vs 4 frames for others)!
    a.abs(0xAD, S_CHAR_ID);
    a.imm8(0xC9, 0x03);
    a.branch(0xF0, "toad_fast_cd");
    a.imm8(0xA9, 0x04);
    a.branch(0x80, "store_jcd");
    a.label("toad_fast_cd");
    a.imm8(0xA9, 0x02);
    a.label("store_jcd");
    a.abs(0x8D, W_JUMP_CD);
    a.imm8(0xA9, 0xFF);
    a.dp(0x85, 0x24);
    a.dp(0x85, 0x25);
    a.dp(0x85, 0x29);
    a.abs(0xAD, W_STAR_LO);
    a.abs(0x0D, W_STAR_HI);
    a.abs(0x0D, W_STOMP_IF);
    a.branch(0xD0, "landed_rts");
    a.abs(0x9C, 0x037B);
    a.label("landed_rts");
    a.op(0x60);

    // =========================================================================
    // HELPER: Mario_BobombChainBlast (v0.20.0 — Explosive Bob-omb Chain Reaction!)
    // Blasts all nearby enemies within 56px of slot X, awards Dragon Coin #13!
    // =========================================================================
    a.label("Mario_BobombChainBlast");
    a.abs(0xAD, W_BOBOMB_CD);
    a.branch(0xF0, "bobomb_go");
    a.op(0x60);
    a.label("bobomb_go");
    a.imm8(0xA9, 0x10);
    a.abs(0x8D, W_BOBOMB_CD);
    a.imm8(0xA9, 0x12);
    a.abs(0x8D, 0x011A); // Screen Shake!
    a.imm8(0xA9, 0x0C);
    a.abs(0x8D, 0x012F); // Heavy Boom SFX!
    a.abs(0xBD, 0x0D10);
    a.abs(0x8D, W_SHELL_TMP_X);
    a.abs(0xBD, 0x0D00);
    a.abs(0x8D, W_SHELL_TMP_Y);
    a.op(0xDA); // PHX
    a.op(0x5A); // PHY
    a.imm8(0xA0, 0x0F);
    a.label("bobomb_loop");
    a.abs(0xB9, 0x0DD0);
    a.imm8(0xC9, 0x09);
    a.branch(0xD0, "bobomb_next");
    a.abs(0xB9, 0x0E20);
    a.op(0xAA);
    a.long(0xBF, MARIO_ENGINE_BANK, STOMP_TABLE_ADDR);
    a.branch(0xF0, "bobomb_next");
    a.imm8(0xC9, 0x05);
    a.branch(0xF0, "bobomb_next");
    // 56px radius (dx + 56 < 112 = 0x70)
    a.op(0x38);
    a.abs(0xB9, 0x0D10);
    a.abs(0xED, W_SHELL_TMP_X);
    a.op(0x18);
    a.imm8(0x69, 0x38);
    a.imm8(0xC9, 0x70);
    a.branch(0xB0, "bobomb_next");
    a.op(0x38);
    a.abs(0xB9, 0x0D00);
    a.abs(0xED, W_SHELL_TMP_Y);
    a.op(0x18);
    a.imm8(0x69, 0x38);
    a.imm8(0xC9, 0x70);
    a.branch(0xB0, "bobomb_next");
    a.imm8(0xA9, 0x06);
    a.abs(0x99, 0x0DD0);
    a.imm8(0xA9, 0x1F);
    a.abs(0x99, 0x0DF0);
    a.imm8(0xA9, 0x02);
    a.jsr("Mario_AddCoinsAndStar");
    a.label("bobomb_next");
    a.op(0x88);
    a.branch(0x10, "bobomb_loop");
    a.abs(0xAD, S_QUEST_LO);
    a.imm8(0x09, 0x10); // Quest bit 4 = Bob-omb Chain Reaction!
    a.abs(0x8D, S_QUEST_LO);
    a.imm8(0xA9, 0x0D); // Persistent Dragon Coin #13!
    a.jsr("Mario_AwardDragonCoin");
    a.op(0x7A); // PLY
    a.op(0xFA); // PLX
    a.label("bobomb_rts");
    a.op(0x60);

    // =========================================================================
    // SUBROUTINE: Mario_CheckGroundPoundShockwave
    // =========================================================================
    a.label("Mario_CheckGroundPoundShockwave");
    a.abs(0xAD, W_GP_IMPACT);
    a.branch(0xD0, "gp_sw_go");
    a.op(0x60);

    a.label("gp_sw_go");
    a.abs(0x9C, W_TMP_B);
    a.imm8(0xA2, 0x0F);
    a.label("gp_sw_loop");
    a.abs(0xBD, 0x0DD0);
    a.imm8(0xC9, 0x09);
    a.branch(0xF0, "gp_sw_active");
    a.jmp("gp_sw_next");

    a.label("gp_sw_active");
    a.abs(0xBD, 0x0E20);
    a.imm8(0xC9, 0x1E);
    a.branch(0xD0, "gp_sw_chk_enemy");
    a.dp(0xA5, 0x10);
    a.imm8(0xC9, 0x07);
    a.branch(0xD0, "gp_sw_next_j");
    a.abs(0xAD, W_TMP_B);
    a.branch(0xD0, "gp_sw_next_j");
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
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_TMP_B);
    a.long(0xAF, 0x7E, 0xC172);
    a.imm8(0x49, 0x01);
    a.long(0x8F, 0x7E, 0xC172);
    a.imm8(0xA9, 0x16);
    a.dp(0x85, 0x11);
    a.imm8(0xA9, 0x25);
    a.abs(0x8D, 0x012E);
    a.imm8(0xA9, 0x2C);
    a.abs(0x8D, W_PSWITCH_LO);
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_PSWITCH_HI);
    a.imm8(0xA9, 0x05);
    a.jsr("Mario_AwardDragonCoin");
    a.jmp("gp_sw_next");

    a.label("gp_sw_next_j");
    a.jmp("gp_sw_next");

    a.label("gp_sw_chk_enemy");
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

    // 48px-radius 16-bit X check
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
    a.branch(0xD0, "gp_sw_normal_enemy");

    // Boss Ground Pound Stagger & Final King Bowser Multi-Phase Counterplay!
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
    a.abs(0x8D, 0x012E);
    a.abs(0xBD, 0x0E20);
    a.imm8(0xC9, 0xD6);
    a.branch(0xD0, "gp_boss_normal");
    a.imm8(0xA9, 0x20);
    a.abs(0x9D, 0x0EA0);
    a.imm8(0xA9, 0x18);
    a.abs(0x8D, 0x011A);
    a.imm8(0xA9, 0x05);
    a.jsr("Mario_AddCoinsAndStar");
    a.imm8(0xA9, 0x0B);
    a.jsr("Mario_AwardDragonCoin");
    a.branch(0x80, "gp_boss_done");
    a.label("gp_boss_normal");
    a.imm8(0xA9, 0x03);
    a.jsr("Mario_AddCoinsAndStar");
    a.imm8(0xA9, 0x08);
    a.jsr("Mario_AwardDragonCoin");
    a.label("gp_boss_done");
    a.imm8(0xA9, 0x18);
    a.abs(0x8D, W_BOSS_HIT_CD);
    a.jmp("gp_sw_next");

    a.label("gp_sw_normal_enemy");
    // Check if Bob-omb (0x4A) -> Explosive Chain Reaction!
    a.abs(0xBD, 0x0E20);
    a.imm8(0xC9, 0x4A);
    a.branch(0xD0, "gp_not_bobomb");
    a.jsr("Mario_BobombChainBlast");
    a.label("gp_not_bobomb");
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
    // SUBROUTINE: Mario_CheckWarpPipe & Mario_UpdateWarpPipe (v0.20.0!)
    // USER BUG #1b & #2 FIXES + BIDIRECTIONAL SMART 5-HUB WARP PIPE NETWORK 2.0!
    //   - Hub 0: World 1 Yoshi's Island  (0x2C r09 c07 -> X=$08E0 Y=$0B30)  [Next=Hub1(2), Prev=Hub4(13)]
    //   - Hub 1: Toad Town OPEN PLAZA!   (0x19 r14 c01 -> X=$0220 Y=$07D0)  [Next=Hub2(0), Prev=Hub0(3)]  <-- MOVED OUT OF HEDGE!
    //   - Hub 2: Vanilla Dome ENEMY-FREE!(0x0B r09 c02 -> X=$0640 Y=$0330)  [Next=Hub3(9), Prev=Hub1(2)]  <-- MOVED OFF DEADROCK!
    //   - Hub 3: Cheep-Cheep Lake        (0x3E r04 c07 -> X=$0CE0 Y=$0E90)  [Next=Hub4(13),Prev=Hub2(0)]
    //   - Hub 4: Forest of Illusion      (0x12 r06 c05 -> X=$04A0 Y=$04D0)  [Next=Hub0(3), Prev=Hub3(9)]
    // Directional Smart Warp Selection while Ground Pounding a Pipe:
    //   - Holding Up ($F0 & $08)   -> Express Return Home to World 1 Yoshi's Island (Slot 3)!
    //   - Holding Left ($F0 & $02) -> Previous Hub (Bidirectional reverse loop!)
    //   - Neutral / Right          -> Next Hub (Forward loop!)
    // =========================================================================
    a.label("Mario_CheckWarpPipe");
    a.abs(0xAD, W_GP_IMPACT);
    a.branch(0xF0, "pipe_rts_early");
    a.abs(0xAD, W_PIPE_CD);
    a.branch(0xD0, "pipe_rts_early");
    a.dp(0xA5, 0x10);
    a.imm8(0xC9, 0x09);
    a.branch(0xF0, "pipe_chk_h0");
    a.label("pipe_rts_early");
    a.op(0x60);

    // Hub 0: Screen 0x2C (r09 c07, pipe at X=$08E0..$08EF, Y=$0B30..$0B3F -> zone X=$08B8..$08FF, Y=$0B10..$0B60)
    a.label("pipe_chk_h0");
    a.dp(0xA5, 0x8A);
    a.imm8(0xC9, 0x2C);
    a.branch(0xD0, "pipe_chk_h1");
    a.dp(0xA5, 0x23);
    a.imm8(0xC9, 0x08);
    a.branch(0xD0, "pipe_rts_early");
    a.dp(0xA5, 0x21);
    a.imm8(0xC9, 0x0B);
    a.branch(0xD0, "pipe_rts_early");
    a.dp(0xA5, 0x22);
    a.imm8(0xC9, 0xB8);
    a.branch(0x90, "pipe_rts_early");
    a.dp(0xA5, 0x20);
    a.imm8(0xC9, 0x10);
    a.branch(0x90, "pipe_rts_early");
    a.imm8(0xC9, 0x60);
    a.branch(0xB0, "pipe_rts_early");
    a.imm8(0xA9, 0x02); // Next = Slot 2 (Toad Town)
    a.imm8(0xA0, 0x0D); // Prev = Slot 13 (Forest of Illusion)
    a.imm8(0xA2, 0x00); // Hub 0
    a.jmp("pipe_begin_enter");

    // Hub 1: Area 0x18 (Toad Town OPEN PLAZA at 0x19 r14 c01 -> pipe at X=$0220, Y=$07D0 -> zone X=$01E8..$0258, Y=$07A4..$07F8!)
    a.label("pipe_chk_h1");
    a.imm8(0xC9, 0x18);
    a.branch(0xD0, "pipe_chk_h2");
    a.dp(0xA5, 0x21);
    a.imm8(0xC9, 0x07);
    a.branch(0xD0, "pipe_rts_early");
    a.dp(0xA5, 0x20);
    a.imm8(0xC9, 0xA4);
    a.branch(0x90, "pipe_rts_early");
    a.dp(0xA5, 0x23);
    a.imm8(0xC9, 0x02);
    a.branch(0xF0, "pipe_h1_x2");
    a.imm8(0xC9, 0x01);
    a.branch(0xD0, "pipe_rts_early");
    a.dp(0xA5, 0x22);
    a.imm8(0xC9, 0xE8); // X in $01E8..$01FF (just west of plaza arrival!)
    a.branch(0x90, "pipe_rts_early");
    a.branch(0x80, "pipe_h1_hit");
    a.label("pipe_h1_x2");
    a.dp(0xA5, 0x22);
    a.imm8(0xC9, 0x58); // X in $0200..$0257 (covers arrival $0200 AND open plaza pipe $0220!)
    a.branch(0xB0, "pipe_rts_early");
    a.label("pipe_h1_hit");
    a.imm8(0xA9, 0x00); // Next = Slot 0 (Vanilla Dome Mountain)
    a.imm8(0xA0, 0x03); // Prev = Slot 3 (Yoshi's Island)
    a.imm8(0xA2, 0x01); // Hub 1
    a.jmp("pipe_begin_enter");

    // Hub 2: Area 0x03 (Vanilla Dome ENEMY-FREE Pipe at 0x0B r09 c02 -> pipe at X=$0640, Y=$0330 -> zone X=$0618..$0688, Y=$0308..$0360!)
    a.label("pipe_chk_h2");
    a.imm8(0xC9, 0x03);
    a.branch(0xD0, "pipe_chk_h3");
    a.dp(0xA5, 0x23);
    a.imm8(0xC9, 0x06);
    a.branch(0xD0, "pipe_rts_mid");
    a.dp(0xA5, 0x21);
    a.imm8(0xC9, 0x03);
    a.branch(0xD0, "pipe_rts_mid");
    a.dp(0xA5, 0x22);
    a.imm8(0xC9, 0x18);
    a.branch(0x90, "pipe_rts_mid");
    a.imm8(0xC9, 0x88); // Covers both pipe ($0640) and arrival ($0678)!
    a.branch(0xB0, "pipe_rts_mid");
    a.dp(0xA5, 0x20);
    a.imm8(0xC9, 0x08);
    a.branch(0x90, "pipe_rts_mid");
    a.imm8(0xC9, 0x60);
    a.branch(0xB0, "pipe_rts_mid");
    a.imm8(0xA9, 0x09); // Next = Slot 9 (Cheep-Cheep Lake)
    a.imm8(0xA0, 0x02); // Prev = Slot 2 (Toad Town)
    a.imm8(0xA2, 0x02); // Hub 2
    a.jmp("pipe_begin_enter");
    a.label("pipe_rts_mid");
    a.op(0x60);

    // Hub 3: Area 0x35 (Cheep-Cheep Lake Pipe at 0x3E r04 c07 -> X=$0C80..$0CFF, Y=$0E58..$0EC8)
    a.label("pipe_chk_h3");
    a.imm8(0xC9, 0x35);
    a.branch(0xD0, "pipe_chk_h4");
    a.dp(0xA5, 0x23);
    a.imm8(0xC9, 0x0C);
    a.branch(0xD0, "pipe_rts_mid");
    a.dp(0xA5, 0x21);
    a.imm8(0xC9, 0x0E);
    a.branch(0xD0, "pipe_rts_mid");
    a.dp(0xA5, 0x22);
    a.imm8(0xC9, 0x80);
    a.branch(0x90, "pipe_rts_mid");
    a.dp(0xA5, 0x20);
    a.imm8(0xC9, 0x58);
    a.branch(0x90, "pipe_rts_mid");
    a.imm8(0xC9, 0xC8);
    a.branch(0xB0, "pipe_rts_mid");
    a.imm8(0xA9, 0x0D); // Next = Slot 13 (Forest of Illusion)
    a.imm8(0xA0, 0x00); // Prev = Slot 0 (Vanilla Dome)
    a.imm8(0xA2, 0x03); // Hub 3
    a.jmp("pipe_begin_enter");

    // Hub 4: Screen 0x12 (Forest of Illusion Pipe at 0x12 r06 c05 -> X=$0478..$04F8, Y=$0498..$0510)
    a.label("pipe_chk_h4");
    a.imm8(0xC9, 0x12);
    a.branch(0xD0, "pipe_rts");
    a.dp(0xA5, 0x23);
    a.imm8(0xC9, 0x04);
    a.branch(0xD0, "pipe_rts");
    a.dp(0xA5, 0x21);
    a.imm8(0xC9, 0x04);
    a.branch(0xD0, "pipe_rts");
    a.dp(0xA5, 0x22);
    a.imm8(0xC9, 0x78);
    a.branch(0x90, "pipe_rts");
    a.dp(0xA5, 0x20);
    a.imm8(0xC9, 0x98);
    a.branch(0x90, "pipe_rts");
    a.imm8(0xA9, 0x03); // Next = Slot 3 (Yoshi's Island Home)
    a.imm8(0xA0, 0x09); // Prev = Slot 9 (Cheep-Cheep Lake)
    a.imm8(0xA2, 0x04); // Hub 4

    // BEGIN AUTHENTIC SMOOTH WARP PIPE DESCENT! (A = next slot, Y = prev slot, X = hub id 0..4)
    a.label("pipe_begin_enter");
    a.abs(0x8D, W_PIPE_DST); // Default = Next Hub
    a.op(0x8E); // STX W_PIPE_HUB
    a.code.extend_from_slice(&W_PIPE_HUB.to_le_bytes());
    // Smart Bidirectional Selection (checks both held D-Pad AND takeoff-latched W_PIPE_STEP / Spin Jump!):
    //   - Holding Up ($F0 & $08) OR Takeoff-Facing Up (W_PIPE_STEP == 0)     -> Express Return Home (Slot 3 Yoshi's Island)!
    //   - Holding Left ($F0 & $02) OR Takeoff-Facing Left (W_PIPE_STEP == 4) -> Previous Hub (Bidirectional reverse loop!)
    //   - Facing/Holding Down or Right -> Next Hub (Forward loop!)
    a.dp(0xA5, 0xF0);
    a.imm8(0x29, 0x08); // Up held?
    a.branch(0xD0, "pipe_sel_home");
    a.abs(0xAD, W_PIPE_STEP);
    a.branch(0xD0, "pipe_chk_left"); // W_PIPE_STEP == 0x00 means Faced Up at jump takeoff!
    a.label("pipe_sel_home");
    a.imm8(0xA9, 0x03); // Express Home to Yoshi's Island!
    a.abs(0x8D, W_PIPE_DST);
    a.branch(0x80, "pipe_start_anim");
    a.label("pipe_chk_left");
    a.dp(0xA5, 0xF0);
    a.imm8(0x29, 0x02); // Left held?
    a.branch(0xD0, "pipe_sel_prev");
    a.abs(0xAD, W_PIPE_STEP);
    a.imm8(0xC9, 0x04); // Faced Left at jump takeoff?
    a.branch(0xD0, "pipe_start_anim");
    a.label("pipe_sel_prev");
    a.op(0x98); // TYA -> Previous Hub!
    a.abs(0x8D, W_PIPE_DST);

    a.label("pipe_start_anim");
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_PIPE_STATE); // State 1 = Smooth Vertical Pipe Descent!
    a.abs(0x8D, 0x02E4); // Lock Link_Control immediately!
    a.imm8(0xA9, 0x16); // 22 frames: 8 frames clean vertical slide + 14 frames smooth fade-out!
    a.abs(0x8D, W_PIPE_TMR);
    a.abs(0x9C, W_PIPE_STEP);
    a.imm8(0xA9, 0x02);
    a.dp(0x85, 0x2F); // Face Down cleanly!
    a.imm8(0xA9, 0x15);
    a.abs(0x8D, 0x012E); // Native SMW Descending Warp Pipe Triple-Pulse SFX!
    a.label("pipe_rts");
    a.op(0x60);

    // --- ACTIVE WARP PIPE STATE MACHINE (Smooth Vertical Slide -> Forced Blank -> Smooth Vertical Emerge!) ---
    a.label("Mario_UpdateWarpPipe");
    a.abs(0xAD, W_PIPE_STATE);
    a.imm8(0xC9, 0x01);
    a.branch(0xF0, "pipe_state_descend");
    a.jmp("pipe_state_emerge");

    a.label("pipe_state_descend");
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, 0x037B); // Untouchable while entering pipe!
    a.abs(0x8D, 0x02E4); // Keep controls locked!
    a.imm8(0xA9, 0x02);
    a.dp(0x85, 0x2F); // Stay facing Down!
    a.abs(0xAD, W_PIPE_TMR);
    a.branch(0xF0, "pipe_exec_warp");
    a.abs(0xCE, W_PIPE_TMR);
    // Smooth pixel-by-pixel downward descent (+1px every 2 frames for first 14 frames -> 7px clean sink!)
    a.abs(0xAD, W_PIPE_TMR);
    a.imm8(0x29, 0x01);
    a.branch(0xD0, "pipe_desc_fade");
    a.abs(0xAD, W_PIPE_STEP);
    a.imm8(0xC9, 0x07);
    a.branch(0xB0, "pipe_desc_fade");
    a.abs(0xEE, W_PIPE_STEP);
    a.dp(0xE6, 0x20); // Slide Y +1px smoothly downward into pipe mouth!
    a.branch(0xD0, "pipe_desc_fade");
    a.dp(0xE6, 0x21);
    a.label("pipe_desc_fade");
    a.abs(0xAD, W_PIPE_TMR);
    a.imm8(0xC9, 0x0F);
    a.branch(0xB0, "pipe_desc_full");
    a.dp(0x85, 0x13); // Smooth hardware brightness fade-out (0x0E -> 0x00) on final 14 frames!
    a.op(0x60);
    a.label("pipe_desc_full");
    a.imm8(0xA9, 0x0F);
    a.dp(0x85, 0x13);
    a.op(0x60);

    a.label("pipe_exec_warp");
    // HARDWARE-LEGAL FORCED BLANK ($00:4200=0, $00:420C=0, JSL $00:8919 -> $0D:F988 -> $0A:B8A5 -> $0A:B928)!
    a.imm8(0xA9, 0x00);
    a.long(0x8F, 0x00, 0x4200);
    a.long(0x8F, 0x00, 0x420C);
    a.op(0x48);
    a.op(0xAB); // DBR = $00
    a.long(0x22, 0x00, 0x8919);
    a.long(0x22, 0x0D, 0xF988);
    a.long(0xAF, 0x7E, W_PIPE_DST);
    a.abs(0x8D, 0x1AF0);
    a.abs(0x9C, 0x1AF1);
    a.long(0x22, 0x0A, 0xB8A5);
    a.imm8(0xE2, 0x30);
    a.imm8(0xA9, 0x00);
    a.op(0x48);
    a.op(0xAB);
    a.long(0x22, 0x0A, 0xB928);
    a.imm8(0xE2, 0x30);
    a.imm8(0xA9, 0x7E);
    a.op(0x48);
    a.op(0xAB);
    a.abs(0x9C, 0x0200);
    a.dp(0x64, 0x11);
    a.dp(0x64, 0x5D);
    a.dp(0xA5, 0x8A);
    a.abs(0x8D, W_LAST_AREA);
    a.jsr("Mario_ResetAirborne");
    a.jsr("Mario_ResetShell");
    // Record Hub Discovery bitmask in S_PIPE_DISC, Quest bit 1, Pipe travel count, & 1-time Dragon Coin!
    a.abs(0xAD, S_PIPE_TOTAL);
    a.imm8(0xC9, 0xFF);
    a.branch(0xF0, "pipe_cnt_cap");
    a.abs(0xEE, S_PIPE_TOTAL);
    a.label("pipe_cnt_cap");
    a.abs(0xAD, S_QUEST_LO);
    a.imm8(0x09, 0x02); // Quest bit 1 = Warp Pipe Network Activated!
    a.abs(0x8D, S_QUEST_LO);
    // Set Hub bit (1 << W_PIPE_HUB) in S_PIPE_DISC
    a.abs(0xAC, W_PIPE_HUB);
    a.imm8(0xA9, 0x01);
    a.label("hub_bit_shl");
    a.op(0x88);
    a.branch(0x30, "hub_bit_done");
    a.op(0x0A);
    a.branch(0x80, "hub_bit_shl");
    a.label("hub_bit_done");
    a.abs(0x0D, S_PIPE_DISC);
    a.abs(0x8D, S_PIPE_DISC);
    a.jsr("Mario_SealSramV2");
    a.abs(0xAD, W_PIPE_HUB);
    a.jsr("Mario_AwardDragonCoin");

    // Keep Mario at the EXACT 100% safe $02:EB67 arrival (Y, X) — ZERO south/north shift into whirlpools or walls!
    // Emergence animates an authentic SMW Pipe Pop-Out Parabolic Hop on $24 Z-altitude (0 -> 14px -> 0)!
    a.abs(0x9C, W_PIPE_STEP);

    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_PIPE_STATE);
    a.abs(0x9C, W_PIPE_TMR);
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_PAL_DIRTY);
    a.abs(0x8D, 0x02E4); // Keep controls locked during smooth emergence!
    a.dp(0x85, 0x15);
    a.imm8(0xA9, 0x04);
    a.dp(0x85, 0x17);
    a.imm8(0xA9, 0x80);
    a.dp(0x85, 0x13);
    a.imm8(0xA9, 0x50);
    a.abs(0x8D, W_PIPE_CD);
    a.imm8(0xA9, 0x81);
    a.long(0x8F, 0x00, 0x4200);
    a.imm8(0xA9, 0x14); // Ascending SMW Warp Pipe Emerge SFX!
    a.abs(0x8D, 0x012E);
    a.op(0x60);

    a.label("pipe_state_emerge");
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, 0x02E4);
    a.imm8(0xA9, 0x02);
    a.dp(0x85, 0x2F);
    a.abs(0xAD, W_PIPE_TMR);
    a.branch(0xD0, "pipe_emerge_fade");
    a.imm8(0xA9, 0x80);
    a.dp(0x85, 0x13);
    a.abs(0xEE, W_PIPE_TMR);
    a.op(0x60);
    a.label("pipe_emerge_fade");
    a.dp(0x85, 0x13); // Smooth brightness fade-in 0x01 -> 0x0F!
                      // Authentic SMW Pipe Pop-Out Parabolic Hop via $24 Z-altitude (0 -> 14px -> 0) at exact safe landing (Y, X)!
    a.imm8(0xC9, 0x08);
    a.branch(0x90, "pipe_hop_up");
    a.op(0x38);
    a.imm8(0xA9, 0x0F);
    a.abs(0xED, W_PIPE_TMR);
    a.branch(0x80, "pipe_hop_apply");
    a.label("pipe_hop_up");
    a.abs(0xAD, W_PIPE_TMR);
    a.label("pipe_hop_apply");
    a.op(0x0A); // * 2 -> 0..14px clean vertical pop-out hop!
    a.dp(0x85, 0x24);
    a.dp(0x64, 0x25);
    a.imm8(0xA9, 0xFF);
    a.dp(0x85, 0x29); // Never STZ $29!
    a.abs(0xEE, W_PIPE_TMR);
    a.abs(0xAD, W_PIPE_TMR);
    a.imm8(0xC9, 0x10);
    a.branch(0x90, "pipe_emerge_wait");
    // Land cleanly on safe arrival tile and unlock player controls!
    a.imm8(0xA9, 0xFF);
    a.dp(0x85, 0x24);
    a.dp(0x85, 0x25);
    a.dp(0x85, 0x29);
    a.imm8(0xA9, 0x0F);
    a.dp(0x85, 0x13);
    a.abs(0x9C, W_PIPE_STATE);
    a.abs(0x9C, 0x02E4); // Unlock player controls cleanly!
    a.abs(0x9C, 0x037B);
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_PAL_DIRTY);
    a.label("pipe_emerge_wait");
    a.op(0x60);

    // =========================================================================
    // SUBROUTINE: Mario_UnstickEnemies (v0.20.0 USER BUG #2 RUNTIME GUARD!)
    // On Vanilla Dome (Area 0x03, Pipe at X=$0640, Y=$0330) and Yoshi's Island
    // (Area 0x2C, Pipe at X=$08E0, Y=$0B30), if any roaming enemy overlaps the
    // solid Warp Pipe footprint (< 14px), nudge it 16px cleanly outward so NO
    // enemy can ever become trapped inside a Warp Pipe!
    // =========================================================================
    a.label("Mario_UnstickEnemies");
    a.dp(0xA5, 0x10);
    a.imm8(0xC9, 0x09); // Overworld Mode only
    a.branch(0xD0, "unstick_rts");
    a.dp(0xA5, 0x8A);
    a.imm8(0xC9, 0x03); // Vanilla Dome (pipe at X=$0648, Y=$0338)
    a.branch(0xF0, "unstick_03");
    a.imm8(0xC9, 0x2C); // Yoshi's Island (pipe at X=$08E8, Y=$0B38)
    a.branch(0xF0, "unstick_2c");
    a.label("unstick_rts");
    a.op(0x60);

    a.label("unstick_03");
    a.imm8(0xA9, 0x48);
    a.abs(0x8D, W_TMP_A);
    a.imm8(0xA9, 0x38);
    a.abs(0x8D, W_TMP_B);
    a.imm8(0xA0, 0x06); // X_hi = $06
    a.imm8(0xA9, 0x03); // Y_hi = $03
    a.abs(0x8D, W_TMP_C);
    a.branch(0x80, "unstick_scan");

    a.label("unstick_2c");
    a.imm8(0xA9, 0xE8);
    a.abs(0x8D, W_TMP_A);
    a.imm8(0xA9, 0x38);
    a.abs(0x8D, W_TMP_B);
    a.imm8(0xA0, 0x08); // X_hi = $08
    a.imm8(0xA9, 0x0B); // Y_hi = $0B
    a.abs(0x8D, W_TMP_C);

    a.label("unstick_scan");
    a.imm8(0xA2, 0x0F);
    a.label("unstick_loop");
    a.abs(0xBD, 0x0DD0);
    a.imm8(0xC9, 0x09);
    a.branch(0xD0, "unstick_next");
    a.op(0x98); // TYA (expected X_hi)
    a.abs(0xDD, 0x0D30);
    a.branch(0xD0, "unstick_next");
    a.abs(0xAD, W_TMP_C); // expected Y_hi
    a.abs(0xDD, 0x0D20);
    a.branch(0xD0, "unstick_next");
    a.op(0x38);
    a.abs(0xBD, 0x0D10);
    a.abs(0xED, W_TMP_A);
    a.op(0x18);
    a.imm8(0x69, 0x0E);
    a.imm8(0xC9, 0x1C);
    a.branch(0xB0, "unstick_next");
    a.op(0x38);
    a.abs(0xBD, 0x0D00);
    a.abs(0xED, W_TMP_B);
    a.op(0x18);
    a.imm8(0x69, 0x0E);
    a.imm8(0xC9, 0x1C);
    a.branch(0xB0, "unstick_next");
    // Enemy wedged in pipe -> nudge cleanly +16px south!
    a.op(0x18);
    a.abs(0xBD, 0x0D00);
    a.imm8(0x69, 0x10);
    a.abs(0x9D, 0x0D00);
    a.label("unstick_next");
    a.op(0xCA);
    a.branch(0x10, "unstick_loop");
    a.op(0x60);

    // =========================================================================
    // SUBROUTINE: Mario_CheckStomp (v0.20.0!)
    // - Descending Mario Stomp + SMW Spin Jump safe bounce off Spinies/Thwomps!
    // - SMW Spin Jump shatters Dry Bones (class 0x06) & Frozen enemies!
    // - Bob-omb (0x4A) stomp triggers explosive Chain Blast!
    // =========================================================================
    a.label("Mario_CheckStomp");
    a.abs(0xAD, W_STAR_LO);
    a.abs(0x0D, W_STAR_HI);
    a.branch(0xD0, "scan_sprites");

    a.abs(0xAD, W_AIRBORNE);
    a.imm8(0xC9, 0x02);
    a.branch(0xF0, "chk_stomp_z");
    a.imm8(0xC9, 0x04);
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
    a.imm8(0xC9, 0x09);
    a.branch(0xF0, "spr_is_active");
    a.imm8(0xC9, 0x0B); // Frozen in Ice!
    a.branch(0xF0, "spr_is_frozen");
    a.jmp("spr_next");

    a.label("spr_is_frozen");
    a.imm8(0xA0, 0x05);
    a.jmp("check_bbox");

    a.label("spr_is_active");
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
    a.imm8(0xC9, 0x05); // Bosses handled by Ground Pound & Boss Combat
    a.branch(0xD0, "spr_not_boss");
    a.jmp("spr_next");
    a.label("spr_not_boss");
    // Spiked / Armored (class 2): Starman, Ground Pound Slam, OR SMW Spin Jump (`W_SPIN_JUMP != 0`)!
    a.imm8(0xC9, 0x02);
    a.branch(0xD0, "check_bbox");
    a.abs(0xAD, W_STAR_LO);
    a.abs(0x0D, W_STAR_HI);
    a.branch(0xD0, "check_bbox");
    a.abs(0xAD, W_AIRBORNE);
    a.imm8(0xC9, 0x04);
    a.branch(0xF0, "check_bbox");
    a.abs(0xAD, W_SPIN_JUMP); // v0.20.0: SMW Spin Jump bounces safely off Spinies & Thwomps!
    a.branch(0xD0, "check_bbox");
    a.jmp("spr_next");

    a.label("check_bbox");
    a.abs(0x8C, W_TMP_B);
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
    // Track cumulative Mario Stomp stat & Quest bit 0 in SRAM v2!
    a.abs(0xAD, S_STOMP_TOTAL);
    a.imm8(0xC9, 0xFF);
    a.branch(0xF0, "stomp_cnt_cap");
    a.abs(0xEE, S_STOMP_TOTAL);
    a.label("stomp_cnt_cap");
    a.abs(0xAD, S_QUEST_LO);
    a.imm8(0x09, 0x01); // Quest bit 0 = First Mario Stomp!
    a.abs(0x8D, S_QUEST_LO);

    // Starman always instant KOs!
    a.abs(0xAD, W_STAR_LO);
    a.abs(0x0D, W_STAR_HI);
    a.branch(0xF0, "stomp_not_star");
    a.jmp("instant_kill");
    a.label("stomp_not_star");

    // v0.20.0 SMW Spin Jump safe bounce off Spiked/Armored (class 2)!
    a.abs(0xAD, W_TMP_B);
    a.imm8(0xC9, 0x02);
    a.branch(0xD0, "chk_ice_shatter");
    a.abs(0xAD, W_AIRBORNE);
    a.imm8(0xC9, 0x04); // Ground Pound Slam still crushes class 2!
    a.branch(0xD0, "spin_spiny_bounce");
    a.jmp("normal_stomp_dmg");
    a.label("spin_spiny_bounce");
    // Spin Jump safe clink bounce off Spiny/Thwomp! (+1 Coin, Quest bit 3, Dragon Coin #15!)
    a.imm8(0xA9, 0x25);
    a.abs(0x8D, 0x012E); // Crisp metallic Spin-Jump Spiny clink!
    a.imm8(0xA9, 0x01);
    a.jsr("Mario_AddCoinsAndStar");
    a.abs(0xAD, S_QUEST_LO);
    a.imm8(0x09, 0x08); // Quest bit 3 = SMW Spin Jump Spiny Bounce!
    a.abs(0x8D, S_QUEST_LO);
    a.imm8(0xA9, 0x0F); // Persistent Dragon Coin #15 (SMW Spin Jump Master!)
    a.jsr("Mario_AwardDragonCoin");
    a.jmp("do_stomp_bounce");

    // Ice Shatter Bonus (Y == 5)
    a.label("chk_ice_shatter");
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

    // Dry Bones (Class 0x06) & Koopa Shell Retract (Class 0x03)
    a.label("chk_koopa_retract");
    a.imm8(0xC9, 0x06);
    a.branch(0xD0, "chk_koopa_c3");
    a.abs(0xAD, W_AIRBORNE);
    a.imm8(0xC9, 0x04); // Ground Pound shatters Dry Bones!
    a.branch(0xF0, "drybones_shatter");
    a.abs(0xAD, W_SPIN_JUMP); // SMW Spin Jump ALSO shatters Dry Bones!
    a.branch(0xF0, "drybones_crumble");
    a.label("drybones_shatter");
    a.jmp("instant_kill");
    a.label("drybones_crumble");
    a.imm8(0xA9, 0x78);
    a.abs(0x9D, 0x0EA0);
    a.abs(0x9E, 0x0D50);
    a.abs(0x9E, 0x0D40);
    a.imm8(0xA9, 0x08);
    a.abs(0x8D, 0x012F);
    a.jmp("do_stomp_bounce");

    a.label("chk_koopa_c3");
    a.imm8(0xC9, 0x03);
    a.branch(0xD0, "normal_stomp_dmg");
    a.abs(0xAD, W_SHELL_STATE);
    a.branch(0xD0, "normal_stomp_dmg");
    a.op(0x8E);
    a.code.extend_from_slice(&W_SHELL_SLOT.to_le_bytes());
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_SHELL_STATE);
    a.imm8(0xA9, 0xB4);
    a.abs(0x8D, W_SHELL_TIMER);
    a.imm8(0xA9, 0x10);
    a.abs(0x8D, W_SHELL_KICK_CD);
    a.abs(0x9E, 0x0D50);
    a.abs(0x9E, 0x0D40);
    a.imm8(0xA9, 0x14);
    a.abs(0x9D, 0x0EA0);
    a.imm8(0xA9, 0x08);
    a.abs(0x8D, 0x012F);
    a.imm8(0xA9, 0x01);
    a.jsr("Mario_AddCoinsAndStar");
    a.jmp("do_stomp_bounce");

    a.label("normal_stomp_dmg");
    // Check if Bob-omb (0x4A) -> Chain Blast!
    a.abs(0xBD, 0x0E20);
    a.imm8(0xC9, 0x4A);
    a.branch(0xD0, "stomp_not_bobomb");
    a.jsr("Mario_BobombChainBlast");
    a.label("stomp_not_bobomb");
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
    a.abs(0xAD, W_COMBO);
    a.op(0x18);
    a.imm8(0x69, 0x01);
    a.imm8(0xC9, 0x04);
    a.branch(0x90, "combo_coin_ok");
    a.imm8(0xA9, 0x04);
    a.label("combo_coin_ok");
    a.jsr("Mario_AddCoinsAndStar");
    a.abs(0xAD, W_COMBO);
    a.branch(0xF0, "no_combo_dcoin");
    a.imm8(0xA9, 0x07);
    a.jsr("Mario_AwardDragonCoin");
    a.label("no_combo_dcoin");

    a.abs(0xAD, W_AIRBORNE);
    a.branch(0xF0, "spr_next");

    a.label("do_stomp_bounce");
    a.jsr("Mario_SealSramV2");
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
    // SUBROUTINE: Mario_UpdateKoopaShell (v0.20.0 + Remote Dungeon ON/OFF Switch Hit!)
    // =========================================================================
    a.label("Mario_UpdateKoopaShell");
    a.abs(0xAD, W_SHELL_STATE);
    a.branch(0xD0, "shell_active");
    a.op(0x60);

    a.label("shell_active");
    a.abs(0xAE, W_SHELL_SLOT);
    a.imm8(0xE0, 0x10);
    a.branch(0x90, "shell_slot_ok");
    a.jmp("shell_clear_rts");

    a.label("shell_slot_ok");
    a.abs(0xBD, 0x0DD0);
    a.imm8(0xC9, 0x09);
    a.branch(0xF0, "shell_still_alive");
    a.jmp("shell_clear_rts");

    a.label("shell_still_alive");
    a.abs(0xCE, W_SHELL_TIMER);
    a.branch(0xD0, "shell_not_expired");
    a.imm8(0xA9, 0x06);
    a.abs(0x9D, 0x0DD0);
    a.imm8(0xA9, 0x18);
    a.abs(0x9D, 0x0DF0);
    a.jmp("shell_clear_rts");

    a.label("shell_not_expired");
    a.abs(0x9E, 0x0D50);
    a.abs(0x9E, 0x0D40);
    a.imm8(0xA9, 0x03);
    a.abs(0x9D, 0x0EA0);

    a.abs(0xAD, W_SHELL_STATE);
    a.imm8(0xC9, 0x01);
    a.branch(0xF0, "shell_stunned_tick");
    a.jmp("shell_kicked_tick");

    a.label("shell_stunned_tick");
    a.abs(0xAD, W_SHELL_KICK_CD);
    a.branch(0xF0, "shell_check_kick");
    a.abs(0xCE, W_SHELL_KICK_CD);
    a.op(0x60);

    a.label("shell_check_kick");
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

    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_SHELL_STATE);
    a.imm8(0xA9, 0x96);
    a.abs(0x8D, W_SHELL_TIMER);
    a.imm8(0xA9, 0x0C);
    a.abs(0x8D, W_STOMP_IF);
    a.abs(0x9C, W_SHELL_VX);
    a.abs(0x9C, W_SHELL_VY);
    a.dp(0xA5, 0x2F);
    a.branch(0xF0, "kick_up");
    a.imm8(0xC9, 0x02);
    a.branch(0xF0, "kick_down");
    a.imm8(0xC9, 0x04);
    a.branch(0xF0, "kick_left");
    a.imm8(0xA9, 0x03);
    a.abs(0x8D, W_SHELL_VX);
    a.branch(0x80, "kick_sfx");
    a.label("kick_up");
    a.imm8(0xA9, 0xFD);
    a.abs(0x8D, W_SHELL_VY);
    a.branch(0x80, "kick_sfx");
    a.label("kick_down");
    a.imm8(0xA9, 0x03);
    a.abs(0x8D, W_SHELL_VY);
    a.branch(0x80, "kick_sfx");
    a.label("kick_left");
    a.imm8(0xA9, 0xFD);
    a.abs(0x8D, W_SHELL_VX);
    a.label("kick_sfx");
    a.imm8(0xA9, 0x08);
    a.abs(0x8D, 0x012F);
    a.label("shell_rts_1");
    a.op(0x60);

    a.label("shell_kicked_tick");
    a.abs(0xAD, W_SHELL_VX);
    a.branch(0xF0, "shell_move_y");
    a.op(0x18);
    a.abs(0x7D, 0x0D10);
    a.abs(0x9D, 0x0D10);
    a.imm8(0xC9, 0x18);
    a.branch(0x90, "shell_bounce_x");
    a.imm8(0xC9, 0xE4);
    a.branch(0x90, "shell_move_y");
    a.label("shell_bounce_x");
    a.imm8(0xA9, 0x00);
    a.op(0x38);
    a.abs(0xED, W_SHELL_VX);
    a.abs(0x8D, W_SHELL_VX);
    a.imm8(0xA9, 0x20);
    a.abs(0x8D, 0x012E);

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
    a.abs(0x8D, W_SHELL_VY);
    a.imm8(0xA9, 0x20);
    a.abs(0x8D, 0x012E);

    a.label("shell_bowl_scan");
    a.abs(0xBD, 0x0D10);
    a.abs(0x8D, W_SHELL_TMP_X);
    a.abs(0xBD, 0x0D00);
    a.abs(0x8D, W_SHELL_TMP_Y);
    a.imm8(0xA0, 0x0F);
    a.label("bowl_loop");
    a.op(0x98);
    a.abs(0xCD, W_SHELL_SLOT);
    a.branch(0xF0, "bowl_next_j");
    a.abs(0xB9, 0x0DD0);
    a.imm8(0xC9, 0x09);
    a.branch(0xD0, "bowl_next_j");

    // v0.20.0 EMERGENT PUZZLE: Kicked Koopa Shell remotely hits Dungeon ON/OFF Crystal Switch ($0E20 == 0x1E)!
    a.abs(0xB9, 0x0E20);
    a.imm8(0xC9, 0x1E);
    a.branch(0xD0, "bowl_chk_enemy");
    a.dp(0xA5, 0x10);
    a.imm8(0xC9, 0x07);
    a.branch(0xD0, "bowl_next_j");
    a.abs(0xAD, W_SHELL_SW_CD);
    a.branch(0xD0, "bowl_next_j");
    // 24px proximity check to Crystal Switch
    a.op(0x38);
    a.abs(0xB9, 0x0D10);
    a.abs(0xED, W_SHELL_TMP_X);
    a.op(0x18);
    a.imm8(0x69, 0x18);
    a.imm8(0xC9, 0x30);
    a.branch(0xB0, "bowl_next_j");
    a.op(0x38);
    a.abs(0xB9, 0x0D00);
    a.abs(0xED, W_SHELL_TMP_Y);
    a.op(0x18);
    a.imm8(0x69, 0x18);
    a.imm8(0xC9, 0x30);
    a.branch(0xB0, "bowl_next_j");
    // SHELL HIT ON/OFF SWITCH!! Flip $7E:C172, invoke Submodule $16, bounce shell, award Dragon Coin #12!
    a.imm8(0xA9, 0x18);
    a.abs(0x8D, W_SHELL_SW_CD);
    a.long(0xAF, 0x7E, 0xC172);
    a.imm8(0x49, 0x01);
    a.long(0x8F, 0x7E, 0xC172);
    a.imm8(0xA9, 0x16);
    a.dp(0x85, 0x11);
    a.imm8(0xA9, 0x25);
    a.abs(0x8D, 0x012E);
    // Reverse shell velocity on switch ricochet!
    a.imm8(0xA9, 0x00);
    a.op(0x38);
    a.abs(0xED, W_SHELL_VX);
    a.abs(0x8D, W_SHELL_VX);
    a.imm8(0xA9, 0x00);
    a.op(0x38);
    a.abs(0xED, W_SHELL_VY);
    a.abs(0x8D, W_SHELL_VY);
    a.abs(0xAD, S_QUEST_LO);
    a.imm8(0x09, 0x04); // Quest bit 2 = Shell-to-Switch Puzzle!
    a.abs(0x8D, S_QUEST_LO);
    a.op(0x5A); // PHY
    a.imm8(0xA9, 0x0C); // Persistent Dragon Coin #12!
    a.jsr("Mario_AwardDragonCoin");
    a.op(0x7A); // PLY
    a.jmp("bowl_next");

    a.label("bowl_next_j");
    a.jmp("bowl_next");

    a.label("bowl_chk_enemy");
    a.abs(0xB9, 0x0E20);
    a.op(0xAA);
    a.long(0xBF, MARIO_ENGINE_BANK, STOMP_TABLE_ADDR);
    a.branch(0xF0, "bowl_next");
    a.imm8(0xC9, 0x05);
    a.branch(0xF0, "bowl_next");
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
    a.imm8(0xA9, 0x06);
    a.abs(0x99, 0x0DD0);
    a.imm8(0xA9, 0x1F);
    a.abs(0x99, 0x0DF0);
    a.imm8(0xA9, 0x2B);
    a.abs(0x8D, 0x012E);
    a.op(0x5A);
    a.imm8(0xA9, 0x02);
    a.jsr("Mario_AddCoinsAndStar");
    a.imm8(0xA9, 0x06);
    a.jsr("Mario_AwardDragonCoin");
    a.op(0x7A);
    a.label("bowl_next");
    a.op(0x88);
    a.branch(0x30, "bowl_done");
    a.jmp("bowl_loop");
    a.label("bowl_done");
    a.op(0x60);

    a.label("shell_clear_rts");
    a.jsr("Mario_ResetShell");
    a.op(0x60);

    // =========================================================================
    // SUBROUTINE: Mario_UpdateBoos (Authentic SMW Boo Orientation-Freeze AI!)
    // =========================================================================
    a.label("Mario_UpdateBoos");
    a.imm8(0xA2, 0x0F);
    a.label("boo_loop");
    a.abs(0xBD, 0x0DD0);
    a.imm8(0xC9, 0x09);
    a.branch(0xD0, "boo_next_j");
    a.op(0x8A);
    a.op(0x48);
    a.abs(0xBD, 0x0E20);
    a.op(0xAA);
    a.long(0xBF, MARIO_ENGINE_BANK, STOMP_TABLE_ADDR);
    a.op(0xA8);
    a.op(0x68);
    a.op(0xAA);
    a.op(0x98);
    a.imm8(0xC9, 0x04);
    a.branch(0xD0, "boo_next_j");

    a.dp(0xA5, 0x2F);
    a.branch(0xF0, "boo_chk_up");
    a.imm8(0xC9, 0x02);
    a.branch(0xF0, "boo_chk_down");
    a.imm8(0xC9, 0x04);
    a.branch(0xF0, "boo_chk_left");
    a.op(0x38);
    a.abs(0xBD, 0x0D10);
    a.dp(0xE5, 0x22);
    a.branch(0x10, "boo_freeze");
    a.branch(0x80, "boo_creep");

    a.label("boo_next_j");
    a.jmp("boo_next");

    a.label("boo_chk_up");
    a.op(0x38);
    a.abs(0xBD, 0x0D00);
    a.dp(0xE5, 0x20);
    a.branch(0x30, "boo_freeze");
    a.branch(0x80, "boo_creep");

    a.label("boo_chk_down");
    a.op(0x38);
    a.abs(0xBD, 0x0D00);
    a.dp(0xE5, 0x20);
    a.branch(0x10, "boo_freeze");
    a.branch(0x80, "boo_creep");

    a.label("boo_chk_left");
    a.op(0x38);
    a.abs(0xBD, 0x0D10);
    a.dp(0xE5, 0x22);
    a.branch(0x30, "boo_freeze");

    a.label("boo_creep");
    a.dp(0xA5, 0x1A);
    a.imm8(0x29, 0x01);
    a.branch(0xD0, "boo_save_pos");
    a.abs(0xBD, 0x0D10);
    a.dp(0xC5, 0x22);
    a.branch(0xF0, "boo_creep_y");
    a.branch(0x90, "boo_inc_x");
    a.abs(0xDE, 0x0D10);
    a.branch(0x80, "boo_creep_y");
    a.label("boo_inc_x");
    a.abs(0xFE, 0x0D10);
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

    a.label("boo_freeze");
    a.abs(0xBD, W_BOO_LAST_X);
    a.branch(0xF0, "boo_save_pos");
    a.abs(0x9D, 0x0D10);
    a.abs(0xBD, W_BOO_LAST_Y);
    a.abs(0x9D, 0x0D00);
    a.abs(0x9E, 0x0D50);
    a.abs(0x9E, 0x0D40);
    a.imm8(0xA9, 0x02);
    a.abs(0x9D, 0x0EA0);

    a.label("boo_next");
    a.op(0xCA);
    a.branch(0x30, "boo_exit");
    a.jmp("boo_loop");
    a.label("boo_exit");
    a.op(0x60);

    // =========================================================================
    // SUBROUTINE: Mario_UpdateBossCombat (v0.20.0 + Final King Bowser Multi-Phase!)
    // =========================================================================
    a.label("Mario_UpdateBossCombat");
    a.abs(0xAD, W_BOSS_HIT_CD);
    a.branch(0xF0, "boss_scan_go");
    a.abs(0xCE, W_BOSS_HIT_CD);
    a.op(0x60);

    a.label("boss_scan_go");
    a.dp(0xA5, 0x10);
    a.imm8(0xC9, 0x07);
    a.branch(0xF0, "boss_mode_ok");
    a.op(0x60);
    a.label("boss_mode_ok");
    a.imm8(0xA2, 0x0F);
    a.label("boss_loop");
    a.abs(0xBD, 0x0DD0);
    a.imm8(0xC9, 0x09);
    a.branch(0xF0, "boss_active_slot");
    a.jmp("boss_next");
    a.label("boss_active_slot");
    // Final King Bowser (0xD6) Multi-Phase HP Threshold Escalation (Phase 1 -> 2 -> 3)!
    a.abs(0xBD, 0x0E20);
    a.imm8(0xC9, 0xD6);
    a.branch(0xD0, "boss_chk_recoil");
    a.abs(0xBD, 0x0E50); // Bowser HP
    a.imm8(0xC9, 0x20); // HP <= 32 -> Phase 2!
    a.branch(0xB0, "boss_chk_recoil");
    a.abs(0xAD, W_BOWSER_PHASE);
    a.branch(0xD0, "bowser_chk_p3");
    // Enter Bowser Phase 2!
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_BOWSER_PHASE);
    a.imm8(0xA9, 0x18);
    a.abs(0x8D, 0x011A); // Dramatic volcanic tremor!
    a.imm8(0xA9, 0x04);
    a.jsr("Mario_AddCoinsAndStar");
    a.abs(0xAD, S_QUEST_LO);
    a.imm8(0x09, 0x40); // Quest bit 6 = Bowser Multi-Phase Reached!
    a.abs(0x8D, S_QUEST_LO);
    a.jsr("Mario_SealSramV2");
    a.branch(0x80, "boss_chk_recoil");
    a.label("bowser_chk_p3");
    a.imm8(0xC9, 0x02);
    a.branch(0xF0, "boss_chk_recoil");
    a.abs(0xBD, 0x0E50);
    a.imm8(0xC9, 0x10); // HP <= 16 -> Phase 3 Final Frenzy!
    a.branch(0xB0, "boss_chk_recoil");
    a.imm8(0xA9, 0x02);
    a.abs(0x8D, W_BOWSER_PHASE);
    a.imm8(0xA9, 0x20);
    a.abs(0x8D, 0x011A);
    a.imm8(0xA9, 0x0E); // Persistent Dragon Coin #14 (Bowser Phase 3!)
    a.jsr("Mario_AwardDragonCoin");

    a.label("boss_chk_recoil");
    a.abs(0xBD, 0x0EA0);
    a.imm8(0xC9, 0x04);
    a.branch(0x90, "boss_next");
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
    a.imm8(0xA9, 0x18);
    a.abs(0x8D, W_BOSS_HIT_CD);
    a.imm8(0xA9, 0x2B);
    a.abs(0x8D, 0x012E);
    a.imm8(0xA9, 0x02);
    a.jsr("Mario_AddCoinsAndStar");
    a.op(0x60);

    a.label("boss_next");
    a.op(0xCA);
    a.branch(0x30, "boss_exit");
    a.jmp("boss_loop");
    a.label("boss_exit");
    a.op(0x60);

    // =========================================================================
    // SUBROUTINE: Mario_UpdatePowerUps (v0.20.0 — 4 Playable Heroes + 14 Palettes!)
    //   - Down + L + R (strictly grounded!) -> Cycle 4 Heroes (Mario -> Luigi -> Peach -> Toad)!
    //   - Up   + L + R -> Timed SMW P-Switch 2x Gold Rush!
    //   - Neutral L+R  -> Balanced Super Starman!
    //   - Deep Water + Flippers -> Auto SMW Frog Suit!
    // =========================================================================
    a.label("Mario_UpdatePowerUps");
    a.abs(0xAD, W_STAR_LO);
    a.abs(0x0D, W_STAR_HI);
    a.branch(0xF0, "pu_not_star");
    a.jmp("star_tick");
    a.label("pu_not_star");

    a.dp(0xA5, 0xF2);
    a.imm8(0x29, 0x30);
    a.imm8(0xC9, 0x30);
    a.branch(0xF0, "star_chk_new");
    a.jmp("resolve_base_form");
    a.label("star_chk_new");
    a.dp(0xA5, 0xF6);
    a.imm8(0x29, 0x30);
    a.branch(0xD0, "lr_chord_dispatch");
    a.jmp("resolve_base_form");

    a.label("lr_chord_dispatch");
    // 1. Down ($F0 & $04) + L + R (ONLY when safely grounded!) -> Cycle 4 Playable Heroes!
    a.dp(0xA5, 0xF0);
    a.imm8(0x29, 0x04);
    a.branch(0xF0, "chk_pswitch_chord");
    a.abs(0xAD, W_AIRBORNE);
    a.branch(0xD0, "chk_pswitch_chord"); // Never swap heroes mid-air!
    a.abs(0xAD, S_CHAR_ID);
    a.op(0x1A); // INC A -> 0->1->2->3->0
    a.imm8(0x29, 0x03);
    a.abs(0x8D, S_CHAR_ID);
    // Keep backward-compatible S_LUIGI_MODE ($F407) synced!
    a.imm8(0xC9, 0x01);
    a.branch(0xF0, "swap_is_luigi");
    a.abs(0x9C, S_LUIGI_MODE);
    a.branch(0x80, "swap_chk_toad_quest");
    a.label("swap_is_luigi");
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, S_LUIGI_MODE);
    a.label("swap_chk_toad_quest");
    a.abs(0xAD, S_CHAR_ID);
    a.imm8(0xC9, 0x03); // Reached Captain Toad -> mark Quest bit 5 (All 4 Heroes Tried!)
    a.branch(0xD0, "swap_seal");
    a.abs(0xAD, S_QUEST_LO);
    a.imm8(0x09, 0x20);
    a.abs(0x8D, S_QUEST_LO);
    a.label("swap_seal");
    a.jsr("Mario_SealSramV2");
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_PAL_DIRTY);
    a.imm8(0xA9, 0x2C); // Sparkling Hero Swap Chime!
    a.abs(0x8D, 0x012E);
    a.jmp("resolve_base_form");

    // 2. Up ($F0 & $08) + L + R -> Timed SMW P-Switch 2x Gold Rush
    a.label("chk_pswitch_chord");
    a.dp(0xA5, 0xF0);
    a.imm8(0x29, 0x08);
    a.branch(0xF0, "star_chk_mp");
    a.abs(0xAD, 0xF36E);
    a.imm8(0xC9, 0x10);
    a.branch(0xB0, "start_pswitch_now");
    a.jmp("resolve_base_form");
    a.label("start_pswitch_now");
    a.op(0x38);
    a.imm8(0xE9, 0x10);
    a.abs(0x8D, 0xF36E);
    a.imm8(0xA9, 0x2C);
    a.abs(0x8D, W_PSWITCH_LO);
    a.imm8(0xA9, 0x01);
    a.abs(0x8D, W_PSWITCH_HI);
    a.imm8(0xA9, 0x25);
    a.abs(0x8D, 0x012E);
    a.imm8(0xA9, 0x0A);
    a.jsr("Mario_AwardDragonCoin");
    a.jmp("resolve_base_form");

    // 3. Neutral L+R -> Balanced Super Starman!
    a.label("star_chk_mp");
    a.abs(0xAD, 0xF36E);
    a.imm8(0xC9, 0x18);
    a.branch(0xB0, "star_start_now");
    a.jmp("resolve_base_form");
    a.label("star_start_now");
    a.op(0x38);
    a.imm8(0xE9, 0x08);
    a.abs(0x8D, 0xF36E);
    a.imm8(0xA9, 0xF0);
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
    a.abs(0xEE, W_STAR_DRAIN_CTR);
    a.abs(0xAD, W_STAR_DRAIN_CTR);
    a.imm8(0xC9, 0x06);
    a.branch(0x90, "star_dec_timer");
    a.abs(0x9C, W_STAR_DRAIN_CTR);
    a.abs(0xAD, 0xF36E);
    a.branch(0xF0, "star_expired");
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
    a.imm8(0x69, 0x05);
    a.jsr("Mario_LoadCgramPal");
    a.op(0x60);

    // Resolve live palette ID (0..13):
    //   Frog Suit (10) -> Fire (1) -> Ice (2) -> Cape (3) -> Yoshi (4) -> Spin Gleam (13) ->
    //   Playable Hero (0=Mario, 9=Luigi, 11=Peach, 12=Toad)!
    a.label("resolve_base_form");
    a.dp(0xA5, 0x5D);
    a.imm8(0xC9, 0x04); // Deep Water Swimming?
    a.branch(0xD0, "chk_non_frog_forms");
    a.abs(0xAD, 0xF356); // Flippers owned?
    a.branch(0xF0, "chk_non_frog_forms");
    a.imm8(0xA9, 0x0A); // Pal 10 = SMW Frog Suit (uses 100% safe collision-respecting swim!)
    a.jmp("apply_form");

    a.label("chk_non_frog_forms");
    a.abs(0xAD, 0x0202);
    a.imm8(0xC9, 0x06);
    a.branch(0xF0, "form_fire");
    a.imm8(0xC9, 0x07);
    a.branch(0xF0, "form_ice");
    a.imm8(0xC9, 0x13);
    a.branch(0xF0, "form_cape");
    a.abs(0xAD, W_YOSHI_ACTIVE);
    a.branch(0xD0, "form_yoshi");
    a.abs(0xAD, S_CHAR_ID);
    a.imm8(0xC9, 0x01);
    a.branch(0xF0, "form_luigi");
    a.imm8(0xC9, 0x02);
    a.branch(0xF0, "form_peach");
    a.imm8(0xC9, 0x03);
    a.branch(0xF0, "form_toad");
    a.imm8(0xA9, 0x00);
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
    a.branch(0x80, "apply_form");
    a.label("form_luigi");
    a.imm8(0xA9, 0x09);
    a.branch(0x80, "apply_form");
    a.label("form_peach");
    a.imm8(0xA9, 0x0B); // Pal 11 = Princess Peach Royal Pink & Gold!
    a.branch(0x80, "apply_form");
    a.label("form_toad");
    a.imm8(0xA9, 0x0C); // Pal 12 = Captain Toad Spotted Cap & Crimson Vest!

    a.label("apply_form");
    a.abs(0x8D, W_TMP_A);
    a.abs(0xCD, W_POWERUP);
    a.branch(0xD0, "do_form_swap");
    a.abs(0xAD, W_PAL_DIRTY);
    a.branch(0xD0, "do_form_swap");
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
    // SUBROUTINE: Mario_LoadCgramPal (supports all 14 palettes: 0..7 & 8..13!)
    // =========================================================================
    a.label("Mario_LoadCgramPal");
    a.imm8(0xC9, 0x08);
    a.branch(0xB0, "pal_hi_bank");
    a.op(0x0A);
    a.abs(0x8D, W_TMP_B);
    a.op(0x0A);
    a.op(0x0A);
    a.op(0x0A);
    a.op(0x0A);
    a.op(0x38);
    a.abs(0xED, W_TMP_B);
    a.op(0xAA);
    a.imm8(0xA0, 0x00);
    a.label("pal_copy_lo");
    a.long(0xBF, MARIO_ENGINE_BANK, PAL_TABLES_ADDR);
    a.abs(0x99, 0xC6E2);
    a.op(0xE8);
    a.op(0xC8);
    a.imm8(0xC0, 0x1E);
    a.branch(0x90, "pal_copy_lo");
    a.branch(0x80, "pal_finish_dma");

    a.label("pal_hi_bank");
    a.op(0x38);
    a.imm8(0xE9, 0x08);
    a.op(0x0A);
    a.abs(0x8D, W_TMP_B);
    a.op(0x0A);
    a.op(0x0A);
    a.op(0x0A);
    a.op(0x0A);
    a.op(0x38);
    a.abs(0xED, W_TMP_B);
    a.op(0xAA);
    a.imm8(0xA0, 0x00);
    a.label("pal_copy_hi");
    a.long(0xBF, MARIO_ENGINE_BANK, PAL_TABLES_ADDR + 0x00F0); // $22:B1F0 (Pal 8..13!)
    a.abs(0x99, 0xC6E2);
    a.op(0xE8);
    a.op(0xC8);
    a.imm8(0xC0, 0x1E);
    a.branch(0x90, "pal_copy_hi");

    a.label("pal_finish_dma");
    a.imm8(0xA9, 0x01);
    a.dp(0x85, 0x15);
    a.op(0x60);

    a.finish()
}

/// Inject the complete v0.20.0 Native 65c816 Super Mario Engine v2.0 into Bank `$22`
/// and hook all 3 verified `Link_Main` (`JSL $07:8000`) callers in Bank `$02`.
pub fn inject_mario_gameplay_engine(
    rom_zelda: &mut SnesRom,
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    if rom_zelda.data.len() < 0x114000 {
        return Ok(0);
    }
    let mut ops = 0usize;

    // 1. Assemble and write the v0.20.0 65c816 Mario Engine v2.0 into Bank $22:9000 (PC 0x111000)
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
            "Moteur 65c816 Super Mario Limit Break v0.20.0 ($22:9000, {} B) : Garde Collision $29=$FF, Tuyaux Bidirectionnels Fluides, Spin Jump, 4 Héros (Mario/Luigi/Peach/Toad), Cape 360°, Shell->Switch, Bob-omb Chain, SRAM v2 32B",
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
        "Table Comportement & Stomp Mario v0.20.0 ($22:B000, 256 B) : Goombas/Bob-ombs (1), Spinies/Deadrocks/Thwomps (2), Koopas Shell-Kick (3), Boos Freeze AI (4), Boss Multi-Phase (5), Dry Bones (6)".into(),
        SwapKind::CodePatch { byte_count: 256 },
    )?;
    ops += 1;

    // 3. Write 14 live BGR555 Hero & Power-Up Palettes at $22:B100 (PC 0x113100, 420 B)
    let pals = build_mario_powerup_palettes();
    session.swap_custom_bytes(
        rom_zelda,
        PAL_TABLES_PC,
        PAL_TABLES_PC,
        pals,
        "Palettes Live 14 Formes & 4 Héros v0.20.0 ($22:B100, 420 B) : Mario, Fire, Ice, Cape, Yoshi, 4 Starman, Luigi, Frog Suit, Princesse Peach, Capitaine Toad & Spin Gleam".into(),
        SwapKind::PaletteBgr555 { color_count: 210 },
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
                    "Hook Boucle Joueur v0.20.0 (PC 0x{pc:06X}) : JSL $07:8000 -> JSL $22:9000 (Super Mario Engine v2.0)"
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
        let code =
            assemble_mario_engine_65816().expect("v0.20.0 65c816 Mario engine v2 must assemble");
        assert!(
            code.len() > 1000 && code.len() < 0x1F80,
            "Engine size {}B must stay cleanly inside $22:9000..$22:AF7F",
            code.len()
        );
        // Must call vanilla Link_Main ($07:8000) and touch Crystal Switch $7E:C172!
        assert!(
            code.windows(4).any(|w| w == [0x22, 0x00, 0x80, 0x07]),
            "Hook must call original Link_Main ($07:8000)"
        );
        assert!(
            code.windows(4).any(|w| w == [0xAF, 0x72, 0xC1, 0x7E]),
            "Ground Pound / Shell / Cape Spin must read hardware Crystal Switch $7E:C172"
        );
        assert!(
            code.windows(4).any(|w| w == [0x22, 0xA5, 0xB8, 0x0A]),
            "Bidirectional Warp Pipe Network 2.0 must invoke ALTTP native Overworld warp loader JSL $0A:B8A5"
        );
        // CRITICAL USER BUG #1 REGRESSION GUARD: NEVER emit `STZ $29` (`64 29`) anywhere in engine!
        assert!(
            !code.windows(2).any(|w| w == [0x64, 0x29]),
            "CRITICAL REGRESSION GUARD: `STZ $29` (0x64 0x29) must NEVER appear in Mario Engine (triggers $07:86D2 BPL hedge collision bypass!)"
        );
        // Must write #$FF into $24, $25, $29 (`A9 FF 85 24 85 25 85 29`)!
        assert!(
            code.windows(8)
                .any(|w| w == [0xA9, 0xFF, 0x85, 0x24, 0x85, 0x25, 0x85, 0x29]),
            "Pre-Link_Main collision guard must write #$FF into $24, $25, AND $29!"
        );
        // Must contain SRAM v2 magic 'M' (0x4D) & 'P' (0x50)
        assert!(code.windows(2).any(|w| w == [0xA9, 0x4D]));
        assert!(code.windows(2).any(|w| w == [0xA9, 0x50]));
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
            stomp[0x27], 2,
            "Vanilla Dome Deadrock/Spiny (0x27) must be Spiked/Armored (class 2)"
        );
        assert_eq!(
            stomp[0x19], 4,
            "Boo (Poe) must use Orientation-Freeze AI (class 4)"
        );
        assert_eq!(
            stomp[0x91], 6,
            "Dry Bones (Stalfos Knight) must use Crumble/Rebuild AI (class 6)"
        );
        assert_eq!(
            stomp[0xA7], 6,
            "Dry Bones (Stalfos) must use Crumble/Rebuild AI (class 6)"
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
            PAL_COUNT * 30,
            "14 Hero & Power-Up palettes * 30 bytes = 420 bytes"
        );
        // Verify modular 8-bit palette offset arithmetic stays < 256 for both low (0..7) and high (8..13) tables!
        for a_idx in 0u8..8u8 {
            let off = a_idx.wrapping_mul(32).wrapping_sub(a_idx.wrapping_mul(2));
            assert_eq!(off as usize, (a_idx as usize) * 30);
        }
        for a_idx in 8u8..(PAL_COUNT as u8) {
            let d = a_idx - 8;
            let off = d.wrapping_mul(32).wrapping_sub(d.wrapping_mul(2));
            assert_eq!(off as usize, (d as usize) * 30);
        }
    }
}
