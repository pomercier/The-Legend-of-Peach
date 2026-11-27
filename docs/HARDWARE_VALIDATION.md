# Hardware & Multi-Emulator Validation Report — v0.8.0

**Target ROM:** `build/legend_of_peach_v0.8.0.sfc`  
**Verified SHA-256:** `2270d55f8f4d09a164a939213531be80e4237a94488f535749708801ba925fd8`  
**Format:** Standard 2 MiB LoROM (`2,097,152` bytes, Checksum `0x1B55`, Complement `0xE4AA`, Valid `true`, CRC32 `113947D1`, zero enhancement chips, zero MSU-1, zero overclock)

---

## 1. Honest Execution Matrix (Zero Assumed Passes)

| Target Platform / Emulator | Installed Locally? | Tested on v0.8.0? | Scenarios Executed | Result |
|---|---|---|---:|---|
| **Mednafen 1.32.1 — `snes_faust` (fast accuracy core)** | Yes | **Yes** | 11 / 12 `v0.8.0` revolution scenarios (Ground Pound ON/OFF, Koopa Shell Ricochet, SMW Boo AI, Ice Mario, Cape Glide, Yoshi Mount & 1-Hit Shield, Balanced Starman, Turn-Block Coin Reward, Morton/Iggy/Ludwig Fortresses) | `VISUAL_PASS` (zero crashes, zero softlocks, crisp SPC700 audio) |
| **Mednafen 1.32.1 — `snes` (cycle-accurate bsnes core)** | Yes | **Yes** | `v080_12_bsnes_final_bowser_gp` (Fire Mario + Ground Pound + Final King Bowser `0x000` + BUG 07 non-regression guard) | `VISUAL_PASS` (`0` divergence vs `snes_faust`) |
| **Standalone `bsnes` GUI** | No | No | 0 | `NOT_TESTED` (Mednafen's `snes` module IS cycle-accurate bsnes) |
| **`ares`** | No | No | 0 | `NOT_TESTED` |
| **`Snes9x`** | No | No | 0 | `NOT_TESTED` |
| **`Mesen 2`** | No | No | 0 | `NOT_TESTED` |
| **Physical SNES / Super Famicom + FXPAK Pro / SD2SNES** | No (not attached to build host) | No | 0 | `NOT_TESTED` — **never claimed as `HARDWARE_PASS`** |

---

## 2. Automated Hardware Compliance Guarantees (`v0.8.0`)

1. **LoROM Header & Checksum:** Internal title `THE LEGEND OF PEACH`, Map Mode `0x20` (LoROM), ROM Size `0x0B` (2 MiB), SRAM `0x03` (8 KiB), Region `0x06` (FR PAL), `0x1B55 + 0xE4AA = 0xFFFF`.
2. **Strict 32 KiB Single-Bank Guard:** All `225` relocated streams in `$23..$3F` stay strictly within single 32 KiB banks.
3. **Zero Memory Overlaps:** `audit_legend_of_peach_output()` verifies `0` overlaps across all `664` hardware ops and enforces isolation of `0x08A000..0x08A2FF`, `$20..$21`, `$22`, `$23..$3A`, and `$3B..$3F`.
4. **Bit-Identical Deterministic Build:** Two independent clean builds verified via `cmp` (exit 0) and `sha256sum` (`2270d55f8f4d09a164a939213531be80e4237a94488f535749708801ba925fd8`).
5. **Real Hardware Protocol (FXPAK Pro / SD2SNES):** Build locally via `snes-cli --build-peach`, verify `sha256sum`, copy to SD card with zero enhancement-chip overrides, and test all `v0.8.0` moves (Ground Pound `Down` mid-air near ON/OFF switch, Koopa Shell kick, Ghost House Boo stare/turn, Ice/Cape/Yoshi/Starman forms, and SRAM persistence across console resets).
