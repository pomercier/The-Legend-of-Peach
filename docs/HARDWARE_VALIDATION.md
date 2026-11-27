# SNES Hardware Validation Report (`v0.10.0`)

**Release:** `v0.10.0 — Super Mario Adventure Beyond Limits`  
**Output ROM:** `build/legend_of_peach_v0.10.0.sfc` (`2,097,152` bytes, 2 MiB LoROM)  
**SHA-256:** `2da376f5c5cd6f760c07e465bdba252fe1aa1f03e69f17dcabcff1ecd6e072b1`  
**CRC32:** `37CE4B03` | **SNES Checksum / Complement:** `0x930C` / `0x6CF3` (`XOR = 0xFFFF`, Valid: `true`)

---

## 1. Static Hardware & Cartridge Compliance (`STATIC_PASS`)

- **Mapping Mode:** Standard SlowROM 2 MiB LoROM (`$7FD5 = 0x20`, `$7FD6 = 0x02` ROM+RAM+Battery, `$7FD7 = 0x0B` 2 MiB, `$7FD8 = 0x03` 8 KiB SRAM, `$7FD9 = 0x06` FR PAL).
- **Enhancement Chips / Overclock:** **None** (`0` SA-1, `0` Super FX, `0` MSU-1, `0` overclock).
- **Bank Boundary & Firewall Audit:** `51` LZ2 graphics sheets verified, `268` relocated blocks cleanly allocated inside `$23..$3F` (`0` overlaps, `0` 32 KiB bank straddles).
- **VRAM / PPU Legality:** All Overworld Warp Pipe transitions disable NMI (`$00:4200 = 0`) and HDMA (`$00:420C = 0`), assert hardware Forced Blank (`$00:2100 = $80`) via `JSL $00:8919`, invoke ALTTP's native `JSL $0D:F988`, `JSL $0A:B8A5`, and `JSL $0A:B928`, and hold `$13 = $80` across 1 NMI frame for `$17 = 4` full tilemap DMA before fading brightness up.
- **Deterministic Build:** Dual independent builds compared via `cmp` and `sha256sum` -> **100% bit-identical**.
- **Automated Rust Tests:** **`32 / 32 PASS`**, `0` clippy warnings, `0` repo safety leaks (`scripts/check_repo_safety.py`).

## 2. Live Emulation & Differential Validation (`VISUAL_PASS` + `AUDIO_PASS` + `PLAYTHROUGH_PASS`)

- **`mednafen:snes_faust`:** `12 / 12 VISUAL_PASS + AUDIO_PASS` (incl. 4-biome Warp Pipe tour, Frog Suit, Playable Luigi, P-Switch 2×, Yoshi Tongue, Dry Bones, 5 newly verified bestiary rooms, and 100% blank-SRAM opening playthrough + cold reboot).
- **`mednafen:snes` (`bsnes-accuracy`):** `2 / 2 VISUAL_PASS + AUDIO_PASS` (bit-identical SRAM parity with `snes_faust` on 2-hop Warp Pipe tour + Final King Bowser Phase 2 Counterplay `250 -> 281` Coins).
- **Not Installed Locally (`NOT_TESTED`):** Standalone `bsnes` GUI, `Ares`, `Snes9x`, `Mesen 2`.

## 3. Physical SNES Hardware Status (`HARDWARE_NOT_TESTED`)

**Physical SNES / Super Famicom / FXPAK Pro console execution is `0 / 1` (`HARDWARE_NOT_TESTED`)** — no physical console is connected to this build environment, and `HARDWARE_PASS` is **never** claimed without real console evidence. See [`docs/REAL_HARDWARE_TEST_GUIDE.md`](REAL_HARDWARE_TEST_GUIDE.md) for the complete FXPAK Pro / SD2SNES hardware validation protocol.
