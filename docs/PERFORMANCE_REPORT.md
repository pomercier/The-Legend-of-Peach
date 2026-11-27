# Hardware Performance & Resource Budget Report — v0.7.0

**Target Hardware:** Unmodified Physical SNES / Super Famicom (2 MiB LoROM, 65c816 @ 2.68/3.58 MHz, PPU, SPC700, 128 KiB WRAM, 64 KiB VRAM, 512+32 B OAM, 512 B CGRAM, 64 KiB ARAM, 8 KiB SRAM)  
**ROM Built:** `build/legend_of_peach_v0.7.0.sfc` (`2,097,152` bytes, SHA-256 `f975131b5eaeee99ee7215ae636a380946ddc48832d3e7be95a11ab5d344dfee`)

---

## 1. ROM Bank Utilization (2 MiB LoROM — Strict 32 KiB Bank Boundaries)

| Region | LoROM Banks | PC Range | Capacity | v0.6.0 Used | v0.7.0 Used | Headroom | Status |
|---|---|---|---:|---:|---:|---:|---|
| Base ALTTP Core + Audio + Dialogues | `$00..$1F` | `0x000000..0x0FFFFF` | 1,048,576 B | In-place | In-place (`0x08A000..0x08A2FF` untouched) | Protected | `STATIC_PASS` |
| 3D Mario Trio Intro Buffers | `$20..$21` | `0x100000..0x10FFFF` | 65,536 B | ~65 KiB | ~65 KiB | Reserved | `STATIC_PASS` |
| Custom 65c816 Mario Engine & Tables | `$22` | `0x110000..0x117FFF` | 32,768 B | 2,560 B | 2,560 B (`$22:8000..$22:99FF`) | **30,208 B free** | `STATIC_PASS` |
| Relocated LC_LZ2 Graphics Sheets | `$23..$3A` | `0x118000..0x1D7FFF` | 786,432 B | ~17 KiB (11 sheets) | ~17.5 KiB (11 sheets) | **>768 KiB free** | `STATIC_PASS` |
| Relocated Overworld Map32 Streams | `$3B..$3F` | `0x1D8000..0x1FFFFF` | 163,840 B | ~460 B (1 screen) | **~48.5 KiB (108 screens, 216 streams)** | **~115 KiB free** | `STATIC_PASS` |

**Memory Overlap Audit:** `0` overlaps across all `661` hardware patches and `225` relocated blocks (`audit_legend_of_peach_output()` enforced at build time).

---

## 2. CPU, VBlank, DMA, OAM & WRAM Budgets

| Subsystem | Hardware Limit (PAL 50 Hz) | Measured / Analytical Usage | Safety Margin |
|---|---|---|---|
| **65c816 Mario Engine (`$22:9000`)** | ~71,000 cycles / frame | ~220 cycles grounded; ~580 cycles airborne + 16-slot stomp scan | `< 0.85%` of frame budget |
| **VBlank Palette Swap (Fire/Starman)** | ~2,600 B DMA / VBlank (PAL) | 30 B WRAM CGRAM buffer (`$7E:C6E2..$7E:C6FF`, `$15 = 1`) via vanilla NMI | `0` extra DMA channels |
| **WRAM Allocation** | 128 KiB (`$7E..$7F`) | `14` bytes active in verified free block `$7E:F500..$7E:F50D` (`$7E:F50E..$7E:F53F` reserved for `v0.8.0`) | `50` bytes free in block |
| **OAM Entries / Scanline** | 128 sprites / frame, 32 / scanline | Preserves native ALTTP OAM slot budgets; Winged Iggy (`0x029`) & Final Bowser (`0x000`) verified zero tile overflow in `bsnes` | `VISUAL_PASS` (`bsnes`) |
| **SPC700 ARAM** | 64 KiB | 19 N-SPC tracks + 5 resident Mario SFX patches fit strictly inside vanilla bank allocations (`$D000..$FCBF` & `$17C0..$284F`) | `STATIC_PASS` |
| **SRAM Save File** | 8 KiB (`0x2000` B) | 100% vanilla-compatible save checksum (`0x5A5A` complement) | `DYNAMIC_PASS` |

---

## 3. PAL 50 Hz vs. NTSC 60 Hz Compatibility Analysis

- **Current Target (`FR PAL` base ROM):** Runs natively at **50 Hz PAL** (312 scanlines, generous ~68-line VBlank window). Every single capture in `v0.7.0` (`23/23` scenarios on `snes_faust` and cycle-accurate `snes`/bsnes) executes cleanly with zero VBlank overrun or screen tearing.
- **NTSC 60 Hz Considerations (`v0.8.0+` Study):** Running a PAL-timed ROM at 60 Hz shortens VBlank from ~68 to ~38 scanlines and speeds up 65c816 tick loops by 20% (while SPC700 music pitch/tempo remains unaffected because the SPC700 has its own independent 24.576 MHz crystal). Because our Bank `$22` Mario engine adds `< 0.9%` CPU overhead and `0` custom VBlank DMA transfers, it is architecturally 60 Hz-safe; full NTSC USA base-ROM support requires remapping base ALTTP pointer tables (tracked for future milestones without breaking the verified FR PAL build).
