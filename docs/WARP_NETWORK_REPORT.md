# Bidirectional Smart Inter-Regional Warp Pipe Network 2.0 Report (`v0.20.0`)

**Release:** `v0.20.0 — Project Nintendo Limit Break`  
**ROM SHA-256:** `5e9c06a5db1ffc6be4879e258fa914bfdd56db101455082c7d2fb711df9cb910`  
**Status:** `VISUAL_PASS` + `AUDIO_PASS` (Dual-Core Verified: `mednafen:snes_faust` + cycle-accurate `mednafen:snes` / `bsnes-accuracy`)

---

## 1. Major `v0.20.0` Upgrades & User Bug Fixes (`USER-BUG-01B`, `USER-BUG-02`)

1. **Input-Locked Smooth Vertical Descent & Coordinate-Safe Pop-Out Emergence (`USER-BUG-01B`):**
   - `Mario_MainHook`'s pre-hook now freezes ALTTP's native `Link_Control` (`$02E4 = 1`), zeroes held/new joypad inputs (`$F0..$F6`), and locks Mario facing Down (`$2F = 2`) throughout both pipe states — completely eliminating walk/swing glitches during pipe travel.
   - **Descent (State 1, 22 frames):** Plays descending Warp Pipe SFX (`$012E = 0x15`), smoothly slides Mario `7px` downward into the pipe while fading hardware brightness `0x0E -> 0x00`, then executes ALTTP's complete hardware-legal Forced Blank warp (`JSL $00:8919 -> $0D:F988 -> $0A:B8A5 -> $0A:B928`) + 1-frame NMI Forced Blank DMA hold (`$17 = 4`, zero VRAM seams).
   - **Emergence (State 2, 16 frames):** Plays newly tuned ascending SMW Pipe Emerge SFX (`$012E = 0x14`), keeps `(Y, X)` locked at the exact safe `$02:EB67` landing pixel (eliminating Pass-1 whirlpool clip `V0200-PASS1-02`), and animates an authentic **SMW Pipe Pop-Out Parabolic Hop on `$24` altitude (`0 -> 14px -> 0`)** while fading brightness `0x01 -> 0x0F`.
2. **Bidirectional Smart Selection (Takeoff-Latched `W_PIPE_STEP <- $2F`):**
   - **Right / Down (Default):** Next Hub in ring.
   - **Left (`$F0 & 0x02` or Takeoff Facing Left):** Previous Hub (reverse loop).
   - **Up (`$F0 & 0x08` or Takeoff Facing Up):** Express Return Home to World 1 Yoshi's Island (`Slot 3`).
3. **Open & Enemy-Free Hub Placements (`USER-BUG-02`):**
   - **Hub 1 (Toad Town):** Relocated out from behind the hedge to **Open Star Plaza `0x19 (r14, c01)`** right below the new Golden Super Star Monument (`X=$01E8..$0257`).
   - **Hub 2 (Vanilla Dome):** Relocated off Deadrock/Spiny `0x27`'s Bank `$09` spawn `(r10, c05)` to **enemy-free plateau `0x0B (r09, c02)`**, enforced globally across all 128 OW screens by `build_ow_enemy_exclusion_map()` + runtime `Mario_UnstickEnemies`.

---

## 2. Complete 5-Hub Bidirectional Routing Table

| Hub | Biome / Landmark | Area ID | Physical Pipe Cell (`0x02D4`) | Forward (`Next`) | Reverse (`Left`) | Express (`Up`) | Enemy Conflict | Live Validation |
|---|---|---|---|---|---|---|---|---|
| **Hub 0** | World 1 — Yoshi's Island | `0x2C` | `0x2C (r09, c07)` | Hub 1 (`Slot 2`) | Hub 4 (`Slot 13`) | Home (`Slot 3`) | `0` | `VISUAL_PASS` (`faust` + `bsnes`) |
| **Hub 1** | Toad Town Open Star Plaza | `0x18` | `0x19 (r14, c01)` | Hub 2 (`Slot 0`) | Hub 0 (`Slot 3`) | Home (`Slot 3`) | `0` | `VISUAL_PASS` (`faust` + `bsnes`) |
| **Hub 2** | Vanilla Dome Mountain (0 Halos) | `0x03` | `0x0B (r09, c02)` | Hub 3 (`Slot 9`) | Hub 1 (`Slot 2`) | Home (`Slot 3`) | `0` | `VISUAL_PASS` (`faust` + `bsnes`) |
| **Hub 3** | Lake Cheep-Cheep (Frog Suit) | `0x35` | `0x3E (r04, c07)` | Hub 4 (`Slot 13`) | Hub 2 (`Slot 0`) | Home (`Slot 3`) | `0` | `VISUAL_PASS` (`v0.10.0` + safe hop) |
| **Hub 4** | Forest River Hub (`0x12`, Frog Suit) | `0x12` | `0x12 (r06, c05)` | Hub 0 (`Slot 3`) | Hub 3 (`Slot 9`) | Home (`Slot 3`) | `0` | `VISUAL_PASS` (`v0200_08` Reverse!) |

**Live Cycle-Accurate `bsnes` 4-Hop Proof (`v0200_03`):** Executed **4 consecutive bidirectional Overworld warps in a single uncut run** (`0x2C -> Forward -> 0x19 Toad Town Star Monument -> Forward -> 0x0B Clean Vanilla Dome -> Reverse Left -> 0x19 Toad Town -> Express Up -> 0x2C Yoshi's Island -> Save SRAM v2`) with `pipe_total = 4`, `dragon_cnt = 3`, `pipe_disc = 0x07`, `coins = 280`, `sram_v2_crc_valid = True`, zero VRAM seams, and `AUDIO_PASS (-25.6 dBFS)`.
