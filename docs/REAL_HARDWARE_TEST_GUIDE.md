# Real SNES Hardware & FXPAK Pro Test Guide (`v0.10.0`)

**Target ROM:** `build/legend_of_peach_v0.10.0.sfc` (`2,097,152` bytes, Stock 2 MiB LoROM `0x20`, Checksum `0x930C / 0x6CF3`, SHA-256 `2da376f5c5cd6f760c07e465bdba252fe1aa1f03e69f17dcabcff1ecd6e072b1`)  
**Current Hardware Validation Status:** **`HARDWARE_NOT_TESTED` (`0 / 1` physical consoles run by maintainer so far — NEVER falsely claimed passed)**

---

## 1. Stock Hardware Compatibility Contract

*The Legend of Peach* is engineered to run on a **stock, unmodified Super Nintendo / Super Famicom**:
- **Mapping:** Standard 2 MiB LoROM (`$7FD5 = 0x20`, FastROM off, SlowROM compatible).
- **Cartridge Type:** `0x02` (ROM + 8 KiB battery-backed SRAM, `$7FD8 = 0x03`).
- **Enhancement Chips:** **None** (zero SA-1, zero Super FX, zero DSP-1, zero MSU-1, zero overclocking).
- **VRAM / DMA Safety:** All Overworld Warp Pipe reloads enter hardware Forced Blank (`$00:4200 = 0`, `$00:420C = 0`, `$00:2100 = $80`) before touching VRAM and hold Forced Blank for 1 NMI frame (`$17 = 0x04`) before fading brightness up.
- **Region / Video Timing:** Base ROM is **FR PAL (`50 Hz`, `$7FD9 = 0x06`)**.
  - **PAL SNES (SNSP-001) + FXPAK Pro / SD2SNES:** Native `50 Hz` timing (recommended primary test target).
  - **NTSC SNES / Super Famicom (SNS-001 / SHVC-001) + FXPAK Pro:** May require disabling region check / using FXPAK Pro PAL-on-NTSC auto-patch or testing `60 Hz` audio pitch & VBlank headroom (PAL has `~312` scanlines vs NTSC `~262` — verify zero NMI overflow on `60 Hz` NTSC consoles during heavy dungeon transitions!).

---

## 2. Step-by-Step FXPAK Pro / SD2SNES Verification Checklist

1. **Build & Verify SHA-256 Locally:**
   ```bash
   cargo run --release --bin snes-cli -- --build-peach \
     --zelda roms/zelda_alttp.sfc --smw roms/super_mario_world.sfc \
     --out build/legend_of_peach_v0.10.0.sfc
   sha256sum build/legend_of_peach_v0.10.0.sfc
   # Expected: 2da376f5c5cd6f760c07e465bdba252fe1aa1f03e69f17dcabcff1ecd6e072b1
   ```
2. **Copy to SD Card:** Place `legend_of_peach_v0.10.0.sfc` on your FXPAK Pro / SD2SNES microSD card with **in-game hooks disabled** on first test so raw stock bus timing is measured.
3. **Hardware Checklist (Report PASS / FAIL + Console Revision `1CHIP` / `2/1/3` / `PAL` / `NTSC`):**
   - [ ] Cold power-on -> Title Screen Royal Princess Peach Star-Scepter animates cleanly with SMW Title Calypso audio (zero glitches, zero audio crackle).
   - [ ] Blank SRAM Slot 1 creation -> Opening Rain Prologue -> Star-Torch Chest -> Save (`Select -> Sauvegarder et quitter`) -> **Physical Power Switch OFF for 10s -> Power ON** -> Slot 1 reloads cleanly with intact battery-backed SRAM.
   - [ ] Overworld Warp Pipe 5-Hub Circuit (`0x2C -> 0x18 -> 0x03 -> 0x35 -> 0x12 -> 0x2C`) -> Verify zero VRAM tearing/garbage on real PPU1/PPU2 CRT/OSSC output across all 5 transitions and confirm no infinite coin farming on re-entry.
   - [ ] Playable Luigi (`Down + L + R`), Timed P-Switch (`Up + L + R`), Starman (`L + R`), Yoshi Companion Mount + Tongue Swallow, SMW Frog Suit in Lake Cheep-Cheep -> Save -> Power-cycle -> Verify `S_LUIGI_MODE` & `S_DRAGON_BITS` persist.
   - [ ] Dungeon ON/OFF Ground Pound Flip (`Room 0x077`), Koopa Shell Ricochet (`Room 0x081`), Boo Freeze AI (`Room 0x059`), Dry Bones Crumble/Shatter (`Room 0x026`), and Final King Bowser Phase 2 (`Room 0x000`) -> Verify zero OAM flicker anomalies or NMI crashes.

Reporting a completed physical console run (with console revision & photo/video) directly unlocks the `14_physical_hardware_validation` gate required for **`v1.0.0`**.
