# Next Actions & Honest Gates Towards `v1.0.0`

`v0.20.0 — Project Nintendo Limit Break` resolves all 3 user-reported gameplay/visual bugs, upgrades Mario Engine to v2.0 (4 Playable Heroes, Spin Jump, Cape 360° Spin, Bidirectional Warp Pipes 2.0, Versioned 32-Byte Checksummed SRAM v2), eliminates Toad Town's Weathervane Bird & Vanilla Dome's boulder halos, and achieves `15/15 VISUAL_PASS + AUDIO_PASS` across dual cores — yet **honestly stops short of `v1.0.0`** until the two remaining `1.0` readiness gates are satisfied:

## P0 — Mandatory Gates for `v1.0.0`
1. **Physical SNES / FXPAK Pro Hardware Validation (`14_physical_hardware_validation: 0% -> 100%`):**
   - Execute [`docs/REAL_HARDWARE_TEST_GUIDE.md`](REAL_HARDWARE_TEST_GUIDE.md) on a stock unmodified PAL SNES (`SNSP-001`) + FXPAK Pro / SD2SNES (plus optional NTSC 60 Hz check) verifying zero VRAM tearing, clean SPC700 audio, and battery-backed SRAM v2 retention across physical power-offs.
2. **Continuous Uninterrupted Blank-SRAM Start-to-Credits Full Playthrough (`13_playthrough_completeness: 62% -> 100%`):**
   - Complete an uninterrupted playthrough from 100% blank SRAM through all 3 Light World Relics, Peach's Castle Tower Bowser, all 7 Dark World Koopaling Fortresses, Final King Bowser, and the Ending Credits with zero softlocks and zero room-warp shortcuts.
3. **Broader Reference Emulator Matrix (`12_qa_and_differential_test_coverage`):**
   - Run verification checks on `Ares`, standalone `bsnes`, `Snes9x`, and `Mesen 2` in addition to `Mednafen snes_faust` + `Mednafen bsnes-accuracy`.

## P1 — Optional Polish Opportunities for `v1.0.0`
4. **Custom SMW BRR Instrument Samples:** Evaluate replacing selected ALTTP BRR samples with custom-tuned SMW brass/marimba/bass samples to raise `05_audio_authenticity` past `94%`.
5. **Dedicated Hero Sprite Geometry & Water-Framed Pipe Pads:** Explore custom Bank `$10` dress/cap silhouettes for Princess Peach & Captain Toad and blue-water-edged tile frames around River Hub 4 (`0x12`) & Lake Cheep-Cheep (`0x35`) Warp Pipes.
6. **Live Visual Proof of Piranha Plant (`MU-058`) on DW `0x6C`:** Add a quick Dark World `0x6C` walk-south scenario alongside `MU-060` (Rex) and `MU-061` (Swooper) already promoted to `VISUAL_PASS` in `v0.20.0`.
