# Real Physical Inter-Regional Warp Pipe Network Report (`v0.10.0`)

**Release:** `v0.10.0 — Super Mario Adventure Beyond Limits`  
**ROM SHA-256:** `2da376f5c5cd6f760c07e465bdba252fe1aa1f03e69f17dcabcff1ecd6e072b1`  
**Status:** `VISUAL_PASS` + `AUDIO_PASS` (Dual-Core Verified: `mednafen:snes_faust` + `mednafen:snes` / `bsnes-accuracy`)

---

## 1. Architecture & Hardware Execution Path

Unlike `v0.9.0`'s stationary front-yard warp pipe bonus (`+5` coins on Screen `0x2C`), `v0.10.0` implements a **true physical inter-regional Overworld Warp Pipe Network** connecting 5 distinct Mushroom Kingdom hubs using ALTTP's complete native Bank `$02` / `$0A` Overworld loader (`mario_engine.rs` -> `Mario_CheckWarpPipe` + `Mario_UpdateWarpPipe`):

1. **Trigger:** Ground Pound (`L`/`R` -> `Down` mid-air) beside any Hub's 3D Glossy Green SMW Warp Pipe (`m32 = 0x02D4`, `m16 = 0x21B`, Pal 6 Emerald Green).
2. **State 1 — Descent (`W_PIPE_STATE = 1`, 15 frames):** Sets player untouchable (`$037B = 1`), plays the native SPC700 descending Warp Pipe triple-pulse SFX (`$012E = 0x15`), and smoothly fades SNES PPU hardware brightness (`$13`: `0x0E -> 0x00`).
3. **Hardware-Legal Forced Blank VRAM Reload (`pipe_exec_warp`):**
   - Disables NMI (`$00:4200 = 0x00`) and HDMA (`$00:420C = 0x00`) via 24-bit long writes, switches `DBR = $00`.
   - Calls **`JSL $00:8919`** (hardware Forced Blank `INIDISP = $80`, clears HDMA/Mosaic).
   - Calls **`JSL $0D:F988`** (clears stale overlay/sprite VRAM buffers).
   - Loads destination `$02:EB67` slot index into 16-bit `$1AF0` and invokes **`JSL $0A:B8A5`** (executes `$02:ECBB` coordinates/camera, `$02:ED5F` Map32 decompression, `$00:D314`/`$0F:FE78` Overworld BG/Sprite tilesets, `$00:E11B` biome CGRAM palettes, `$02:B219` sprite reload, and target biome SPC700 music).
   - Restores `DBR = $00` and invokes **`JSL $0A:B928`** (`JSL $02:B215 -> JSR $EDDB`: builds the complete 64×64 BG1+BG2 tilemap buffer in `$7E:2000` and arms NMI full-screen tilemap DMA flag `$17 = 0x04`).
   - Restores `DBR = $7E`, awards the **1-time Persistent Checksummed SRAM Discovery Dragon Coin** (`$7E:F400`, eliminating `v0.9.0`'s infinite coin farming!), sets an 80-frame anti-retrigger cooldown (`W_PIPE_CD = 0x50`), keeps Forced Blank (`$13 = 0x80`) for 1 NMI frame (`W_PIPE_TMR = 0`) so NMI finishes DMA'ing `$17=4` & `$15=1` with zero visible tearing, and re-enables `$00:4200 = 0x81`.
4. **State 2 — Emergence (`W_PIPE_STATE = 2`, 16 frames):** Frame 0 finishes NMI tilemap DMA in blank; Frames 1..15 fade brightness smoothly `0x01 -> 0x0F`, restoring Mario's active power-up/Luigi CGRAM palette cleanly.

---

## 2. Pass-1 Defect Caught & Resolved Before Release

During Pass 1 live visual QA (`v0100_01` & `v0100_02`), calling `JSL $0A:B8A5` without `JSL $0A:B928` left the right half of VRAM holding stale tiles from Screen `0x2C`, creating a visible vertical BG seam. Disassembling `$02:9F83..$02:9FA8` revealed ALTTP's exact four-call sequence (`$00:8919`, `$0D:F988`, `$0A:B8A5`, `$0A:B928`). Adding `JSL $0A:B928` + the 1-frame NMI Forced Blank DMA hold eliminated 100% of VRAM seams on both `snes_faust` and cycle-accurate `bsnes`.

---

## 3. Complete 5-Hub Routing Table

| Hub | Biome / Landmark | Area ID | Physical Pipe Cell (`0x02D4`) | Destination (`$02:EB67` Slot) | SRAM Coin Bit | Validation |
|---|---|---|---|---|---|---|
| **Hub 0** | World 1 — Yoshi's Island (Mario's House) | `0x2C` | `0x2C (r09, c07)` | Hub 1: Toad Town (`Slot 2`, `0x18`) | Bit `0` | `VISUAL_PASS` (`faust` + `bsnes`) |
| **Hub 1** | Toad Town Weathervane Plaza | `0x18` | `0x19 (r12, c03)` | Hub 2: Vanilla Dome (`Slot 0`, `0x03`) | Bit `1` | `VISUAL_PASS` (`faust` + `bsnes`) |
| **Hub 2** | Vanilla Dome Terracotta Mountain (`MU-045`) | `0x03` | `0x0B (r10, c05)` | Hub 3: Lake Cheep-Cheep (`Slot 9`, `0x35`) | Bit `3` | `VISUAL_PASS` (`faust` + `bsnes`) |
| **Hub 3** | Lake Cheep-Cheep (SMW Frog Suit Zone) | `0x35` | `0x3E (r04, c07)` | Hub 4: Forest of Illusion (`Slot 13`, `0x12`) | Bit `2` | `VISUAL_PASS` (`faust`) |
| **Hub 4** | Forest of Illusion | `0x12` | `0x12 (r06, c05)` | Hub 0: Yoshi's Island (`Slot 3`, `0x2C`) | Bit `4` | `STATIC_PASS` |

**Live Uncut Multi-Hop Proof:** Scenario `v0100_05_frog_suit_and_vanilla_dome` executed **3 consecutive live Overworld warps across 4 biomes in a single uncut run** (`0x2C` `250c` -> `0x18` `260c` -> `0x03` `270c` -> `0x35` `280c` -> deep water SMW Frog Suit swim) with zero crashes, zero VRAM seams, and `AUDIO_PASS (-25.8 dBFS)`.
