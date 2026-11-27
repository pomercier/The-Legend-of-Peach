# Next Actions & Honest Gates Towards `v1.0.0`

`v0.10.0 — Super Mario Adventure Beyond Limits` deliberately stops short of `v1.0.0` until all true `1.0` readiness gates are honestly satisfied:

## P0 — Mandatory Gates for `v1.0.0`
1. **Physical SNES / FXPAK Pro Hardware Validation (`14_physical_hardware_validation: 0% -> 100%`):**
   - Execute [`docs/REAL_HARDWARE_TEST_GUIDE.md`](REAL_HARDWARE_TEST_GUIDE.md) on a stock unmodified PAL SNES (`SNSP-001`) + FXPAK Pro / SD2SNES (and optionally an NTSC SNES / Super Famicom `60 Hz` check) confirming zero VRAM tearing, clean SPC700 audio, and battery-backed SRAM persistence across physical power cycles.
2. **Continuous Uninterrupted Blank-SRAM Start-to-Credits Full Playthrough (`13_playthrough_completeness: 58% -> 100%`):**
   - Complete an uninterrupted playthrough from 100% blank SRAM through all 3 Light World Relics, Peach's Castle Tower Bowser, all 7 Dark World Koopaling Fortresses, Final King Bowser, and the Ending Credits with zero softlocks and zero save-state/warp shortcuts.
3. **Broader Reference Emulator Matrix (`12_qa_and_differential_test_coverage`):**
   - Run cross-emulator spot checks on `Ares`, standalone `bsnes`, `Snes9x`, and `Mesen 2` in addition to `Mednafen snes_faust` + `Mednafen bsnes-accuracy`.

## P1 — High-Value `v1.0.0` Polish Opportunities
4. **Interactive Multi-Destination Warp Zone Selector:** Upgrade the 5-Hub ring cycle (`0x2C -> 0x18 -> 0x03 -> 0x35 -> 0x12 -> 0x2C`) with an optional Hub choice UI or D-Pad directional pipe selection, plus a water-edged tile frame for Lake Cheep-Cheep's pipe pad.
5. **Custom SMW BRR Instrument Samples:** Evaluate replacing selected ALTTP BRR samples with custom-tuned Mario brass/marimba/bass samples for even higher `05_audio_authenticity` fidelity.
6. **Deep Structural Residue (`ZRI-022`, `4/416`):** Replace the Toad Town Weathervane bird statue Map16 sprite/tile with a Mushroom Kingdom Golden Star Monument or Yoshi Egg Fountain.
