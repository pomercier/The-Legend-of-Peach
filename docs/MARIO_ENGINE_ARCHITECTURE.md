# Super Mario Engine v2.0 — Native 65c816 Architecture (`v0.20.0`)

**Release:** `v0.20.0 — Project Nintendo Limit Break`  
**Output ROM:** `build/legend_of_peach_v0.20.0.sfc` (`2,097,152` bytes, 2 MiB LoROM Mode `$20`)  
**Verified SHA-256:** `5e9c06a5db1ffc6be4879e258fa914bfdd56db101455082c7d2fb711df9cb910`  
**Source Module:** [`crates/snes-core/src/mario_engine.rs`](../crates/snes-core/src/mario_engine.rs)

---

## 1. Design Philosophy: Stock SNES 65c816 First

Mario Engine v2.0 runs **100% natively on the stock Ricoh 5A22 (65c816 @ 2.68/3.58 MHz)** with **zero enhancement chips** (no SA-1, no SuperFX, no MSU-1, no overclock). All engine code and data live inside standard LoROM Bank `$22`:

| SNES Address | PC Offset | Region | Contents |
|---|---|---|---|
| `$22:8000..$22:8FFF` | `0x110000..0x110FFF` | Intro 3D DMA | 32-frame 3D Mario Relics NMI DMA handler (`mario_intro.rs`) |
| `$22:9000..$22:AFFF` | `0x111000..0x112FFF` | **Mario Engine v2.0** | Native 65c816 `Mario_MainHook` + 15 subsystems |
| `$22:B000..$22:B0FF` | `0x113000..0x1130FF` | Stomp Vulnerability LUT | 256-byte sprite class table (`0=immune, 1=stomp, 2=spiked/spin-bounce, 3=shell, 5=boss, 6=drybones`) |
| `$22:B100..$22:B2A3` | `0x113100..0x1132A3` | **14 BGR555 Hero/Form Palettes** | `420` bytes (`14 × 15 × 2 B`: Classic, Fire, Ice, Cape, Yoshi, Star×4, Luigi, Frog Suit, **Princess Peach**, **Captain Toad**, **Spin Gleam**) |

All 3 ALTTP Bank `$02` callers of `JSL $07:8000` (`Link_Main`) are hooked via `JSL $22:9000`, which runs a pre-hook guard, calls intact `JSL $07:8000`, and executes the post-hook Mario Engine v2.0 pipeline whenever `$7E:0010` is `0x07` (Dungeon) or `0x09` (Overworld).

---

## 2. Critical Collision & Animation Architecture Fixes (`v0.20.0`)

### 2.1 Universal Solid-Tile / Hedge Collision Guard (USER BUG #1 Fix)
Disassembly of ALTTP's `Link_Move` (`$07:86CD..$07:86D5`) proved:
```asm
$07:86CD  A5 24     LDA $24        ; Player Z-altitude
$07:86CF  10 40     BPL +$40       ; Skip solid tile collision if $24 >= 0!
$07:86D1  A5 29     LDA $29        ; Secondary Z / jump offset
$07:86D3  10 3C     BPL +$3C       ; Skip solid tile collision if $29 >= 0!
$07:86D5  A0 05     LDY #$05
$07:86D7  20 77 D0  JSR $D077      ; Hardware solid-tile / hedge / wall collision!
```
In `v0.10.0`, `mario_engine.rs` executed `STZ $29` (`0x00`, positive!) while airborne, inadvertently causing `BPL +$3C` to skip `JSR $D077` so Mario could jump onto solid hedges and walls.  
In **`v0.20.0`**, every airborne pre-hook and jump update writes **`LDA #$FF; STA $29`** (negative), guaranteeing `JSR $D077` runs on **every single airborne frame**. An automated Rust unit test (`test_mario_engine_65816_assembles_cleanly`) now statically scans the assembled 65c816 bytecode and **fails the build if opcode `64 29` (`STZ $29`) ever appears**.

### 2.2 Locked Warp Pipe Descent & Coordinate-Safe Pop-Out Emergence (USER BUG #1b Fix)
When `W_PIPE_STATE != 0`, `Mario_MainHook`'s pre-hook immediately freezes player control via ALTTP's native `$02E4 = 1`, zeroes joypad inputs (`$F0..$F6`), and locks facing Down (`$2F = 2`):
- **State 1 (Descent, 22 frames):** Plays native descending Warp Pipe SFX (`$012E = 0x15`), slides Mario smoothly downward `7px` (`+1px` every 2 frames) while fading hardware brightness (`$13`: `0x0E -> 0x00`), then enters **hardware-legal Forced Blank** (`$00:4200 = 0`, `$00:420C = 0`, `JSL $00:8919 -> $0D:F988 -> $0A:B8A5 -> $0A:B928`) and holds 1 NMI frame in Forced Blank for `$17 = 4` full tilemap DMA.
- **State 2 (Emergence, 16 frames):** Plays ascending SMW Pipe Emerge SFX (`$012E = 0x14`), keeps `(Y, X)` locked on the exact safe `$02:EB67` arrival coordinate (zero clipping into whirlpools or walls — catching and resolving Pass-1 defect `V0200-PASS1-02`), and animates an authentic **SMW Pipe Pop-Out Parabolic Hop on `$24` altitude (`0 -> 14px -> 0`)** while fading brightness `0x01 -> 0x0F`, before unlocking `$02E4 = 0`.

### 2.3 Runtime Enemy Unstick Guard (USER BUG #2 Defense-in-Depth)
Paired with `mario_map.rs`'s compile-time Bank `$09` enemy-spawn exclusion map across all 128 Overworld screens, `Mario_UnstickEnemies` monitors roaming sprites on Vanilla Dome (`0x03`) and Yoshi's Island (`0x2C`) and gently nudges any enemy overlapping a Warp Pipe footprint (`<14px`) cleanly outward by `16px`.

---

## 3. Complete `v0.20.0` Gameplay Subsystems

1. **6-Frame Jump Input Buffer (`W_JUMP_BUF`):** Pressing `L` or `R` up to 6 frames before touching the ground queues an immediate frame-1 re-jump on landing.
2. **SMW Spin Jump (`R` Button, `W_SPIN_JUMP`):** Rapidly cycles `$2F` facing every 3 frames with Pal 13 Golden Gleam, **safely bounces off Spiked/Armored Class `0x02` foes (Spinies, Thwomps)** with metallic clink (`$012E = 0x25`) + `+1` Coin + Dragon Coin `#15`, and **permanently shatters Dry Bones (Class `0x06`)**!
3. **4 Playable Heroes (`Down + L + R` grounded -> cycles `S_CHAR_ID = 0..3`, persisted in SRAM v2):**
   - `0 — Super Mario`: Balanced jump (`+2.56 px/f`), classic red/blue palette (`Pal 0`).
   - `1 — Luigi`: Highest floaty jump (`+2.94 px/f`), Emerald Cap & Navy Overalls (`Pal 9`), mirrors legacy `S_LUIGI_MODE`.
   - `2 — Princess Peach`: Royal Rose-Pink Gown & Golden Crown (`Pal 11`) + **28-frame (~0.56s) SMB2 Floating Dress Hover** (`VZ = 0.00 px/f` while holding `L`/`R` descending!).
   - `3 — Captain Toad`: White Spotted Mushroom Cap & Crimson Explorer Vest (`Pal 12`), agile snappy hop (`+2.44 px/f`), **`+2` Turn-Block Throw Coin bonus**, and passive Star Power regeneration (`+2` every 64 frames).
4. **Cape Mario 360° Spin Attack (`Mario_UpdateCapeSpin`):** With Cape Feather equipped (`$0202 == 0x13`), pressing `B`/`Y` performs a 12-frame 360° cape whip that stuns enemies within `40px` (`+2` Coins) and **flips Dungeon SMW ON/OFF Crystal Switches (`$7E:C172` + Submodule `0x16`)**!
5. **Koopa/Buzzy Shell -> Remote ON/OFF Switch Flip:** Kicked ricocheting shells (`W_SHELL_STATE == 2`) scan active sprites for Crystal Switches (`0x1E`) within `22px` and trigger hardware block-matrix swap (`$7E:C172 ^= 1, $11 = 0x16`) remotely!
6. **Bob-omb (`0x4A`) 56px Explosive Chain Reaction (`Mario_BobombChainBlast`):** Stomping or Ground Pounding a Bob-omb triggers heavy screen shake (`$011A = 0x16`), blast SFX, and damages all nearby enemies within `56px` (`+2` Coins + Quest bit `4`).
7. **Bidirectional Smart 5-Hub Warp Pipe Network 2.0:** Latches Mario's facing direction at jump takeoff (`W_PIPE_STEP <- $2F` before Ground Pound `Down` overwrites `$2F`):
   - **Right / Down (Default):** Next Hub (`0x2C -> 0x19 -> 0x0B -> 0x35 -> 0x12 -> 0x2C`).
   - **Left (`$F0 & 0x02` or Takeoff Left):** Previous Hub (bidirectional reverse loop!).
   - **Up (`$F0 & 0x08` or Takeoff Up):** Express Return Home to World 1 Yoshi's Island (`Slot 3`).
8. **World Restoration Persistence & Final Bowser Multi-Phase:** Mirrors conquered Koopaling Crystals (`$7E:F37A`) into `S_WORLD_RESTORE` (`$7E:F40B`) with passive Star recharge, and tracks escalating Final King Bowser (`0xD6`) Ground Pound counterplay in `S_BOWSER_FLG`.
9. **Versioned 32-Byte Checksummed SRAM Schema v2 (`$7E:F400..$7E:F41F`):** See [`docs/SAVE_SCHEMA.json`](SAVE_SCHEMA.json).

---

## 4. Analytical 5A22 CPU & Memory Budget (PAL 50 Hz = 71,364 cyc/frame)

| State | Mario Engine v2.0 Est. Cycles | % of 50 Hz Frame | Notes |
|---|---:|---:|---|
| Grounded Idle (No Active Shell/Star/Pipe) | ~310 cyc | `0.43%` | Early-out gates skip airborne, stomp, shell, and pipe loops |
| Airborne Jump + Stomp Scan + Hero Physics | ~980 cyc | `1.37%` | 16-slot sprite scan with immediate state `!= 0x09` early-out |
| Worst-Case Frame (GP Shockwave + Bob-omb Chain + 16 Sprites + CGRAM Refresh) | ~1,950 cyc | `2.73%` | Measured analytically separate from vanilla ALTTP engine; verified smooth on cycle-accurate `bsnes` |
| WRAM Allocated | `79` bytes | `$7E:F500..$7E:F54E` | `49` bytes headroom remaining before vanilla `$7E:F580` (`0` conflicts) |
| Checksummed SRAM Allocated | `32` bytes | `$7E:F400..$7E:F41F` | Inside ALTTP's native checksummed slot (`$7E:F000..$7E:F4FF`), `236` bytes remaining before `$7E:F4FE` |
