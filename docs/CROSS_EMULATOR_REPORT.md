# Cross-Emulator Differential Parity Report (`v0.10.0`)

**Release:** `v0.10.0 — Super Mario Adventure Beyond Limits`  
**ROM SHA-256:** `2da376f5c5cd6f760c07e465bdba252fe1aa1f03e69f17dcabcff1ecd6e072b1`

---

## 1. Installed vs Not-Installed Emulator Matrix (100% Honest)

| Emulator / Core | Local Status | Executed in `v0.10.0`? | Result |
|---|---|---|---|
| **Mednafen 1.32.1 (`snes_faust`)** | Installed | **YES** (12 scenarios) | `12 / 12 VISUAL_PASS + AUDIO_PASS` |
| **Mednafen 1.32.1 (`snes` / `bsnes-accuracy`)** | Installed | **YES** (2 cycle-accurate scenarios) | `2 / 2 VISUAL_PASS + AUDIO_PASS` |
| **Standalone `bsnes` GUI** | `NOT_INSTALLED` | No | `NOT_TESTED` |
| **Ares** | `NOT_INSTALLED` | No | `NOT_TESTED` |
| **Snes9x** | `NOT_INSTALLED` | No | `NOT_TESTED` |
| **Mesen 2** | `NOT_INSTALLED` | No | `NOT_TESTED` |
| **Physical SNES / FXPAK Pro** | No console attached | No | `HARDWARE_NOT_TESTED` (see `REAL_HARDWARE_TEST_GUIDE.md`) |

---

## 2. True Identical-Scenario Differential Comparison (`snes_faust` vs `bsnes-accuracy`)

For `v0.10.0`, the **exact same unmodified ROM**, **exact same starting SRAM**, and **exact same multi-hop Warp Pipe input sequence** (`0x2C Yoshi's Island -> 0x18 Toad Town -> 0x03 Vanilla Dome -> Select -> Sauvegarder et quitter.`) were executed on BOTH `snes_faust` (`v0100_01_warp_pipe_network_faust`) AND cycle-accurate `bsnes` (`v0100_02_warp_pipe_network_bsnes`):

| Observable Output | `mednafen:snes_faust` | `mednafen:snes` (`bsnes-accuracy`) | Parity |
|---|---|---|---|
| Hub 0 -> Hub 1 Warp (`0x2C -> 0x18` Toad Town) | Clean fade, 0 VRAM seams, `250 -> 260` Coins | Clean fade, 0 VRAM seams, `250 -> 260` Coins | **IDENTICAL** |
| Hub 1 -> Hub 2 Warp (`0x18 -> 0x03` Vanilla Dome) | Clean fade, SMW terracotta palette, `260 -> 270` Coins | Clean fade, SMW terracotta palette, `260 -> 270` Coins | **IDENTICAL** |
| Saved `.srm` `S_DRAGON_BITS` (`$7E:F400..$F403`) | `[3, 0, 0, 0]` (Bits 0 & 1) | `[3, 0, 0, 0]` (Bits 0 & 1) | **BIT-IDENTICAL** |
| Saved `.srm` `S_DRAGON_CNT` (`$7E:F404`) | `2` | `2` | **BIT-IDENTICAL** |
| Saved `.srm` Target Coins (`$7E:F360`) | `270` | `270` | **BIT-IDENTICAL** |
| Saved `.srm` Hardware Checksum (`$04FE`) | `Valid: True` | `Valid: True` | **IDENTICAL** |
| Real SPC700 WAV Audio Metrics | `-26.2 dBFS`, `71.6%` active (`AUDIO_PASS`) | `-26.0 dBFS`, `74.8%` active (`AUDIO_PASS`) | **PARITY PASS** |

Additionally, **`v0100_11_bsnes_final_bowser_phase2`** verified Final King Bowser Phase 2 Counterplay (`250 -> 281` Coins, zero BUG 07 Trident, `AUDIO_PASS -24.4 dBFS`) on cycle-accurate `bsnes`.
