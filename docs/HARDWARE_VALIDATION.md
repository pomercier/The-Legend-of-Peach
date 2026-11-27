# Hardware & Multi-Emulator Validation Report — v0.9.0

**Target ROM:** `build/legend_of_peach_v0.9.0.sfc`  
**Verified SHA-256:** `5546dfaec62b98dc8c91bbbc58a7b16df6de91b9d8e6f0d56a1b7f1b36693fbd`  
**Format:** Standard 2 MiB LoROM (`2,097,152` bytes, Checksum `0xBC86`, Complement `0x4379`, Valid `true`, CRC32 `261F4214`, zero enhancement chips, zero MSU-1, zero overclock)

---

## 1. Honest Execution Matrix (Zero Assumed Passes)

| Target Platform / Emulator | Installed Locally? | Tested on v0.9.0? | Scenarios Executed | Result |
|---|---|---|---:|---|
| **Mednafen 1.32.1 — `snes_faust` (fast accuracy core)** | Yes | **Yes** | 8 / 10 `v0.9.0` scenarios (Physical Overworld SMW Green Warp Pipe + GP `+5` Coins, Morton Boss GP Stagger, Wendy Coral Sea Palace `DungPal[1]`, Lemmy Star-Ice Circus `DungPal[4]`, Larry Chocolate Pyramid `DungPal[5]`, Roy Midnight Koopa Castle `DungPal[10]`, Bowser Obsidian Keep `DungPal[13]`, Dark World Bowser Citadel `0x5B`) | `VISUAL_PASS` (zero crashes, zero softlocks, crisp SPC700 audio) |
| **Mednafen 1.32.1 — `snes` (cycle-accurate bsnes core)** | Yes | **Yes** | 2 / 10 `v0.9.0` scenarios (`v090_01_title_peach_scepter_bsnes` Title Screen Royal Peach Star-Scepter with 0 Master Sword + `v090_10_bsnes_final_bowser_synergy` Final King Bowser + Fire Mario + Boss GP Synergy `250 -> 263` Coins + BUG 07 guard) | `VISUAL_PASS` (`0` divergence vs `snes_faust`) |
| **Standalone `bsnes` GUI** | No | No | 0 | `NOT_TESTED` (Mednafen's `snes` module IS cycle-accurate bsnes) |
| **`ares`** | No | No | 0 | `NOT_TESTED` |
| **`Snes9x`** | No | No | 0 | `NOT_TESTED` |
| **`Mesen 2`** | No | No | 0 | `NOT_TESTED` |
| **Physical SNES / Super Famicom + FXPAK Pro / SD2SNES** | No (not attached to build host) | No | 0 | `NOT_TESTED` — **never claimed as `HARDWARE_PASS`** |

---

## 2. Automated Hardware Compliance Guarantees (`v0.9.0`)

1. **LoROM Header & Checksum:** Internal title `THE LEGEND OF PEACH`, Map Mode `0x20` (LoROM), ROM Size `0x0B` (2 MiB), SRAM `0x03` (8 KiB), Region `0x06` (FR PAL), `0xBC86 + 0x4379 = 0xFFFF`.
2. **Strict 32 KiB Single-Bank Guard:** All `226` relocated streams in `$23..$3F` and all `51` audited compressed LZ2 sheets (including `raw 0xA5` Royal Peach Star-Scepter & `raw 0x3E/0x3F` Overworld Warp Pipes) stay strictly within single 32 KiB banks.
3. **Zero Memory Overlaps:** `audit_legend_of_peach_output()` verifies `0` overlaps across all `671` hardware ops and enforces isolation of `0x08A000..0x08A2FF`, `$20..$21`, `$22`, `$23..$3A`, and `$3B..$3F`.
4. **Bit-Identical Deterministic Build:** Two independent clean builds verified via `cmp` (exit 0) and `sha256sum` (`5546dfaec62b98dc8c91bbbc58a7b16df6de91b9d8e6f0d56a1b7f1b36693fbd`).
5. **Real Hardware Protocol (FXPAK Pro / SD2SNES):** Build locally via `snes-cli --build-peach`, verify `sha256sum`, copy to SD card with zero enhancement-chip overrides, and test the Royal Princess Peach Star-Scepter Title Screen, Overworld Green Warp Pipe Ground Pound bonus outside Mario's House, all 8 Koopaling/Bowser Fortresses, Boss GP Stagger & Synergy, and SRAM persistence across hardware resets.
