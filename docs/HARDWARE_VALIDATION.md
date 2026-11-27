# Hardware & Multi-Emulator Validation Report — v0.7.0

**Target ROM:** `build/legend_of_peach_v0.7.0.sfc`  
**Verified SHA-256:** `f975131b5eaeee99ee7215ae636a380946ddc48832d3e7be95a11ab5d344dfee`  
**Format:** Standard 2 MiB LoROM (`2,097,152` bytes, Checksum `0x1285`, Complement `0xED7A`, Valid `true`, zero enhancement chips, zero MSU-1, zero overclock)

---

## 1. Honest Execution Matrix (No Assumed Passes)

| Target Platform / Emulator | Installed Locally? | Tested on v0.7.0? | Scenarios Executed | Result |
|---|---|---|---:|---|
| **Mednafen 1.32.1 — `snes_faust` (fast accuracy core)** | Yes | **Yes** | 20 / 23 scenarios (11 Bosses, Mario states, Light/Dark Worlds, Toad, Kamek) | `VISUAL_PASS` (zero crashes, zero hangs, clean SPC700 audio) |
| **Mednafen 1.32.1 — `snes` (cycle-accurate bsnes core)** | Yes | **Yes** | 3 / 23 critical scenarios (`boss_12_final_bowser_bsnes`, `mario_state_06_core_fire_starman_bsnes`, `world_04_dw_volcanic_bsnes`) | `VISUAL_PASS` (`0` divergence vs `snes_faust`!) |
| **Standalone `bsnes` (higan/bsnes GUI)** | No | No | 0 | `NOT_TESTED` (Mednafen's built-in cycle-accurate `snes` module IS bsnes-accuracy) |
| **`ares` (cycle-accurate multi-system)** | No | No | 0 | `NOT_TESTED` |
| **`Snes9x`** | No | No | 0 | `NOT_TESTED` |
| **`Mesen 2`** | No | No | 0 | `NOT_TESTED` |
| **Physical SNES / Super Famicom + FXPAK Pro / SD2SNES** | No (no physical console attached to build host) | No | 0 | `NOT_TESTED` — **never claimed as `HARDWARE_PASS`** |

---

## 2. Structural Hardware Compliance Checks (Verified Automatically on Every Build)

1. **LoROM Header & Checksum:** Internal title `THE LEGEND OF PEACH`, Map Mode `0x20` (LoROM), ROM Size `0x0B` (2 MiB), SRAM `0x03` (8 KiB), Region `0x06` (FR PAL), Checksum + Complement = `0xFFFF` (`0x1285 + 0xED7A = 0xFFFF`).
2. **Strict 32 KiB Bank Boundary Guard:** All `225` relocated streams in `$23..$3F` stay strictly inside single 32 KiB LoROM banks (never crossing `$xx:FFFF -> $xx+1:8000`).
3. **Zero Memory Overlaps:** `audit_legend_of_peach_output()` verifies `0` conflicting writes across all `661` hardware ops, `0x08A000..0x08A2FF` world map table untouched, `$20..$21` intro isolated, `$22` 65c816 engine isolated, `$23..$3A` GFX isolated, `$3B..$3F` Map32 isolated.
4. **Deterministic Bit-for-Bit Build:** Two independent clean runs produced identical SHA-256 `f975131b5eaeee99ee7215ae636a380946ddc48832d3e7be95a11ab5d344dfee` (`cmp` exit code 0).

---

## 3. Step-by-Step Physical SNES / FXPAK Pro Validation Protocol (For Real Hardware Testing)

To validate `build/legend_of_peach_v0.7.0.sfc` on an unmodified PAL Super Nintendo with an FXPAK Pro / SD2SNES flashcart:
1. Build the ROM locally from your legally dumped unmodified ROMs (`snes-cli --build-peach --zelda roms/zelda_alttp.sfc --smw roms/super_mario_world.sfc --out build/legend_of_peach_v0.7.0.sfc`) and verify `sha256sum` matches `f975131b5eaeee99ee7215ae636a380946ddc48832d3e7be95a11ab5d344dfee`.
2. Copy `legend_of_peach_v0.7.0.sfc` to the FXPAK Pro SD card (disable any cheat codes, SuperFX/SA-1 overrides, or MSU-1 packs).
3. Boot on a stock 50 Hz PAL SNES and verify:
   - 3D Super Mushroom / Fire Flower / Super Star intro spins smoothly without tearing,
   - File Select displays crisp `1. MARIO`, Super Star cursor, and Super Mario Hammer,
   - World 1 Yoshi's Island (`0x2C`) displays seamless Turn-Blocks, `L`/`R` Mario Jump + Stomp, Fire Mario palette swap, Koopa Hammer Bro Guards (no swords), and Super Starman (`L+R`),
   - Dark World Pyramid (`0x5B`) displays Bowser's Obsidian Citadel palette and Carved Stone Bowser Statues,
   - Saving & power-cycling preserves SRAM cleanly across console resets.
