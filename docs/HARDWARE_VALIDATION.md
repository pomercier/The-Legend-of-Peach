# SNES Hardware & Emulator Validation Matrix (`v0.6.0`)

**Verified Build Artifact:** `build/legend_of_peach_v0.6.0.sfc`  
**Output Size & Mapping:** `2,097,152` bytes (`2 MiB`), Standard Slow LoROM (`0x20`, PAL 50 Hz, `0` enhancement chips, `0` MSU-1 dependencies)  
**SNES Checksum / Complement:** `0xC419` / `0x3BE6` (`Checksum ^ Complement == 0xFFFF`, Valid: `true`)  
**Output SHA-256:** `2004714a74f35fbf6c6c581f25b86d0b40d6729dd38c089f65e7b69e68a202ca`

---

## 1. Test Matrix (Strict Separation of Tested vs. Untested Targets)

| Target | Version / Core | Status | Scenes Directly Verified on `2004714a...` |
| :--- | :--- | :--- | :--- |
| **Mednafen (`snes_faust`)** | `1.32.1` | **TESTED — PASS** | 7 scenes: World 1 Koopa contact (`12 fps` impact), Fire Mario 4-dir cast + Screen `0x2C` terrain + Starman, Royal Blue Kamek Shop (`0x011F`), Room `0x0103` Toad, Sanctuary (`0x0012`), Winged Iggy Koopa (`0x0029`), Final King Bowser (`0x0000` dialogue + combat). |
| **Mednafen (`snes` / bsnes-accuracy)** | `1.32.1` (cycle-accurate PPU/CPU/SPC700) | **TESTED — PASS** | 2 scenes: World 1 Screen `0x2C` + Start Menu + Fire Mario + Starman Jump (`08_bsnes_verify`); Final King Bowser Room `0x0000` + French dialogue + combat (`09_bsnes_bowser`). Bit-for-bit visually consistent with `snes_faust`. |
| **Standalone bsnes / Ares** | — | **NOT TESTED LOCALLY** | Not installed in the local build environment. Expected compatible (same cycle-accurate architecture as Mednafen `snes`). |
| **Snes9x / RetroArch** | — | **NOT TESTED LOCALLY** | Not installed in the local build environment. Expected compatible. |
| **Physical SNES / Super Famicom / FXPAK Pro (SD2SNES)** | — | **NOT TESTED ON PHYSICAL HARDWARE** | No physical console or flashcart attached to this workstation. Architecturally compliant with original LoROM hardware rules listed below. |

---

## 2. Automated Hardware Architectural Audits (`audit_legend_of_peach_output`)

Every build executes deterministic hardware checks before writing the output `.sfc` file:

1. **Exact Cartridge Size & Header Integrity:** Strictly `2,097,152` bytes, LoROM header at `0x7FC0`, size byte `0x0B` (`2 MiB`), internal title `THE LEGEND OF PEACH`, valid complement + checksum (`0xC419` / `0x3BE6`).
2. **Zero Cross-Bank DMA / LZ2 Splits:** All `49` modified compressed sheets (`11` relocated into `$23..$3A`) and relocated Overworld Map32 screens (`$3B..$3F`) fit strictly within single 32 KiB LoROM bank boundaries (`0` bank boundary crossings, `0` byte overlaps).
3. **Memory Firewalls Preserved:**
   - Protected ALTTP world-map pointer table `0x08A000..0x08A2FF` untouched.
   - 3D Intro buffers (`$20..$21`), 65c816 Mario Engine (`$22:9000..$22:BFFF`), Relocated GFX (`$23..$3A`), and Relocated Map32 (`$3B..$3F`) strictly disjoint.
   - Custom WRAM variables confined to `$7E:F500..$7E:F53F` (unused scratch block).
4. **Native SPC700 ARAM Budget:** All 19 N-SPC musical tracks (`0x0D1EF5`) and 5 native Mario SFX patches (`0x0D0B70..0x0D1BFF`) fit strictly inside native SNES 64 KiB ARAM with zero external audio hardware.
5. **Determinism Verified:** Two independent clean builds to separate output paths produce identical SHA-256 (`2004714a74f35fbf6c6c581f25b86d0b40d6729dd38c089f65e7b69e68a202ca`) and `cmp` exit code `0`.
