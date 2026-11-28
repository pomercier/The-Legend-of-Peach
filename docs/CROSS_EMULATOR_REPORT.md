# Cross-Emulator Differential Parity Report (`v0.20.0`)

**Release:** `v0.20.0 — Project Nintendo Limit Break`  
**ROM SHA-256:** `5e9c06a5db1ffc6be4879e258fa914bfdd56db101455082c7d2fb711df9cb910`  
**Machine-Readable Matrix:** [`docs/HARDWARE_COMPATIBILITY_MATRIX.json`](HARDWARE_COMPATIBILITY_MATRIX.json)

---

## 1. Installed vs Not-Installed Emulator Matrix (100% Honest)

| Emulator / Core | Local Status | Executed in `v0.20.0`? | Result |
|---|---|---|---|
| **Mednafen 1.32.1 (`snes_faust`)** | Installed | **YES** (13 scenarios) | `13 / 13 VISUAL_PASS + AUDIO_PASS` |
| **Mednafen 1.32.1 (`snes` / `bsnes-accuracy`)** | Installed | **YES** (2 cycle-accurate scenarios) | `2 / 2 VISUAL_PASS + AUDIO_PASS` |
| **Standalone `bsnes` GUI** | `NOT_INSTALLED` | No | `NOT_TESTED` |
| **Ares** | `NOT_INSTALLED` | No | `NOT_TESTED` |
| **Snes9x** | `NOT_INSTALLED` | No | `NOT_TESTED` |
| **Mesen 2** | `NOT_INSTALLED` | No | `NOT_TESTED` |
| **Physical SNES / FXPAK Pro** | No console attached | No | `HARDWARE_NOT_TESTED` (`0/1`) |

---

## 2. True Identical-ROM Differential Comparison (`snes_faust` vs `bsnes-accuracy`)

Both `v0200_02_smooth_bidirectional_pipes_faust` (`snes_faust`) and `v0200_03_bidirectional_pipes_bsnes_parity` (`bsnes-accuracy`) ran the exact unmodified `v0.20.0` ROM (`SHA-256: 5e9c06a5...df9cb910`) through the Bidirectional Smart Warp Pipe Network 2.0 + SRAM v2 Save:

| Observable Output | `mednafen:snes_faust` (`v0200_02`) | `mednafen:snes` / `bsnes-accuracy` (`v0200_03`) | Parity |
|---|---|---|---|
| Hub 0 -> Hub 1 (`0x2C -> 0x19` Toad Town Star Monument) | Smooth locked pipe + Z-hop, 0 seams, `250 -> 260c` | Smooth locked pipe + Z-hop, 0 seams, `250 -> 260c` | **IDENTICAL** |
| Hub 1 -> Hub 2 (`0x19 -> 0x0B` Vanilla Dome, 0 olive halos) | 0 halos, enemy-free pipe `(9,2)`, `260 -> 270c` | 0 halos, enemy-free pipe `(9,2)`, `260 -> 270c` | **IDENTICAL** |
| Bidirectional Return (`Reverse Left` / `Express Home Up`) | Express Home -> `0x2C`, `270 -> 280c` (`pipe_total=3`) | Reverse `0x19` -> Express Home `0x2C`, `270 -> 280c` (`pipe_total=4`) | **VERIFIED BOTH** |
| Saved `.srm` `alttp_sig` & Hardware Checksum | `0x55AA`, `alttp_checksum_valid: True` | `0x55AA`, `alttp_checksum_valid: True` | **IDENTICAL** |
| Saved `.srm` Mario SRAM v2 Header & CRC16 | `magic="MP"`, `version=2`, `sram_v2_crc_valid: True` | `magic="MP"`, `version=2`, `sram_v2_crc_valid: True` | **IDENTICAL** |
| Saved `.srm` Progress (`coins`, `dragon_cnt`, `pipe_disc`) | `coins=280`, `dragon_cnt=3`, `pipe_disc=0x07` | `coins=280`, `dragon_cnt=3`, `pipe_disc=0x07` | **BIT-IDENTICAL** |
| Real SPC700 WAV Audio Metrics | `-25.4 dBFS`, `75.5%` active (`AUDIO_PASS`) | `-25.6 dBFS`, `78.4%` active (`AUDIO_PASS`) | **PARITY PASS** |

Additionally, **`v0200_11_bsnes_final_bowser_multiphase`** verified Final King Bowser Multi-Phase Escalation (`250 -> 285` Coins, 0 BUG 07 regression, `AUDIO_PASS -24.4 dBFS`) on cycle-accurate `bsnes`.
