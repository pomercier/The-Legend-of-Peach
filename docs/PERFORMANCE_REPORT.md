# Hardware Performance & Resource Budget Report — v0.8.0

**Target Hardware:** Unmodified Physical SNES / Super Famicom (2 MiB LoROM, 65c816 @ 2.68/3.58 MHz, PPU, SPC700, 128 KiB WRAM, 64 KiB VRAM, 512+32 B OAM, 512 B CGRAM, 64 KiB ARAM, 8 KiB SRAM)  
**ROM Built:** `build/legend_of_peach_v0.8.0.sfc` (`2,097,152` bytes, SHA-256 `2270d55f8f4d09a164a939213531be80e4237a94488f535749708801ba925fd8`, Checksum `0x1B55 / 0xE4AA`)

---

## 1. ROM Bank Utilization (2 MiB LoROM — Strict 32 KiB Bank Boundaries)

| Region | LoROM Banks | PC Range | Capacity | v0.7.0 Used | v0.8.0 Used | Headroom | Status |
|---|---|---|---:|---:|---:|---:|---|
| Base ALTTP Core + Audio + Dialogues | `$00..$1F` | `0x000000..0x0FFFFF` | 1,048,576 B | In-place | In-place (`0x08A000..0x08A2FF` untouched; `DungPal[8/9/12]` patched) | Protected | `STATIC_PASS` |
| 3D Mario Trio Intro Buffers | `$20..$21` | `0x100000..0x10FFFF` | 65,536 B | ~65 KiB | ~65 KiB | Reserved | `STATIC_PASS` |
| Custom 65c816 Mario Engine & Tables | `$22` | `0x110000..0x117FFF` | 32,768 B | 2,560 B | **~2,520 B** (`$22:8000` intro + `$22:9000` ~1.5 KiB engine + `$22:B000` 256 B table + `$22:B100` 270 B 9-palettes) | **~30,248 B free** | `STATIC_PASS` |
| Relocated LC_LZ2 Graphics Sheets | `$23..$3A` | `0x118000..0x1D7FFF` | 786,432 B | ~17.5 KiB | ~17.5 KiB (`11` relocated sheets) | **>768 KiB free** | `STATIC_PASS` |
| Relocated Overworld Map32 Streams | `$3B..$3F` | `0x1D8000..0x1FFFFF` | 163,840 B | ~48.5 KiB | ~48.5 KiB (`108` screens, `216` streams) | **~115 KiB free** | `STATIC_PASS` |

**Memory Overlap Audit:** `0` overlaps across all `664` hardware patches and `225` relocated blocks (`audit_legend_of_peach_output()` enforced at build time).

---

## 2. CPU, VBlank, DMA, OAM & WRAM Budgets (`v0.8.0`)

| Subsystem | Hardware Limit (PAL 50 Hz) | Measured / Analytical Usage | Safety Margin |
|---|---|---|---|
| **65c816 Engine Idle Frame (`$22:9000`)** | ~71,000 cycles / frame | ~210 cycles (grounded fast-path: mode/area gate + Yoshi/Jump/Boo/PowerUp checks) | **`0.30%`** of frame budget |
| **65c816 Engine Worst-Case Frame** | ~71,000 cycles / frame | ~1,180 cycles (simultaneous Ground Pound 16-slot shockwave + active Koopa Shell bowling scan + 3 active Ghost House Boos + 30 B CGRAM refresh) | **`1.66%`** of frame budget |
| **Live 6-Form CGRAM Palette Swap** | ~2,600 B DMA / VBlank (PAL) | 30 B WRAM CGRAM copy (`$7E:C6E2..$7E:C6FF`, `$15 = 1`) on form change or every 8th frame for non-Classic forms (~180 cycles = `0.25%`) | `0` extra DMA channels |
| **Dungeon ON/OFF Switch Flip** | Native ALTTP VBlank DMA | Delegates 100% to ALTTP hardware Submodule `$16` (`$7E:C172 ^= 1 ; $11 = $16`) | `0` custom VBlank risk |
| **WRAM Allocation** | 128 KiB (`$7E..$7F`) | `64` bytes in verified conflict-free block `$7E:F500..$7E:F53F` (Ground Pound, Koopa Shell, Yoshi Companion, Balanced Starman, 16-slot Boo freeze anchors) | `0` conflicts |
| **OAM Entries / Scanline** | 128 sprites / frame, 32 / scanline | Zero extra OAM tiles spawned; Koopa Shell reuses original enemy slot `X`, Yoshi/Ice/Cape/Fire swap live CGRAM | `VISUAL_PASS` (`bsnes`) |
| **SPC700 ARAM & SRAM** | 64 KiB ARAM / 8 KiB SRAM | 19 N-SPC tracks + 5 resident SFX in-place; 100% vanilla-compatible SRAM (`0x5A5A` checksum) | `STATIC_PASS` |

---

## 3. PAL 50 Hz vs. NTSC 60 Hz Assessment

- **Verified Target (`FR PAL` 50 Hz):** All `12/12` `v0.8.0` scenarios (plus `23/23` `v0.7.0` scenarios) run cleanly with zero slowdown, zero VBlank overrun, and crisp SPC700 audio on `snes_faust` and cycle-accurate `bsnes`.
- **NTSC 60 Hz Headroom:** Even under tighter 60 Hz timing (~59,500 cycles/frame), worst-case `v0.8.0` 65c816 execution (~1,180 cycles = `< 2.0%`) leaves >98% CPU margin and uses `0` custom VBlank DMA channels.
