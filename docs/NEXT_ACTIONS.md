# Next Actions — Roadmap Toward v0.6.0

This file tracks concrete, verified next priorities after **`v0.5.0 — Super Mario Revolution`**.

## Highest-Impact Remaining Opportunities (From `docs/ZELDA_RESIDUE_REGISTRY.json`)

1. **Multi-World Map32 Geographic Redesign (`ZRI-007`, Weight 12 remaining)**
   - Expand `mario_map.rs` beyond World 1 (`Screen 0x2C`) to redesign Toad Town (`0x18`), Yoshi's Meadow (`0x29`), Eastern Courtyard (`0x1E`), and Dark World Valley of Bowser (`0x5B`) using structured JSON maps under `docs/maps/`.
   - Add visual green/yellow SMW Warp Pipe entrances at Overworld caves and secret bonus rooms (`ZRI-017`).

2. **Koopa Guard Equipment Overhaul (`ZRI-016`, Weight 12 remaining)**
   - Replace remaining medieval blades held by Overworld/Castle Koopa guards with SMW Hammer Bro Hammers / spiked clubs.

3. **Title Screen BG2 Master Sword Removal (`ZRI-009`, Weight 10 remaining)**
   - Cleanly remove the vanilla Master Sword BG2 silhouette behind the metallic `PEACH` logo and replace the horizon with Yoshi's Island hills and Warp Pipes while preserving the 32-frame 3D Mario Trio intro.

4. **Custom Boss AI State Machines (`ZRI-019`, Weight 8 remaining)**
   - Add dedicated Bank `$22` (`$22:C000..$22:FFFF`) 65c816 boss behavior hooks for Koopaling hops, bouncing fireballs, and multi-phase King Bowser shell slams.

5. **Rideable Yoshi Mount (`ZRI-020`, Weight 9 remaining)**
   - Implement mount/dismount via Select/Yoshi Whistle using reserved WRAM `$7E:F510..$7E:F51F` for faster Overworld travel and tongue item grab.

6. **Multi-Region & Physical Hardware Verification**
   - Build and test verified USA (NTSC 60 Hz) offset profiles alongside the primary French/PAL build.
   - Collect real SNES / Super Famicom + FXPAK Pro hardware test confirmations from the community.
