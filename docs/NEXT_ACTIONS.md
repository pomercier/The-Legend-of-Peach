# Next Actions — Roadmap Toward `v0.7.0`

This file tracks concrete, verified next priorities after **`v0.6.0 — Super Mario World Authenticity`**.

## Completed in `v0.6.0`
- Resolved all 7 blocking visual regressions (BUG 01–07): all 303 Bank `$10` player DMA poses audited and cleaned (`0` odd-row/unpatched/duplicate cells), Fire Mario 4-dir cast + Zelda Fire/Ice Rod removal, bit-exact Map32→Map16 terrain grammar validator + seamless Screen `0x2C`, SMW Royal Blue Kamek + untouched shop digits, 16-bit SMAS/SMRPG Toad, anchored Winged Iggy Koopa wings (body preserved), and authentic SMW Final King Bowser (`16×24` arms, `GFX24` Fireballs, `0` held Trident).

## Highest-Impact Remaining Opportunities

1. **Multi-World Map32 Geographic Redesign (`ZRI-007`)**
   - Expand `mario_map.rs` + terrain grammar validator beyond Screen `0x2C` (`1/64`) to redesign Toad Town (`0x18`), Yoshi's Meadow (`0x29`), Eastern Courtyard (`0x1E`), and Dark World Valley of Bowser (`0x5B`).
   - Add visual green/yellow SMW Warp Pipe entrances at Overworld caves and secret bonus rooms (`ZRI-017`).

2. **Live Emulation Coverage of Secondary Player & Boss States**
   - Live-capture remaining 6 structurally verified Mario states (swim/dive, electric shock, push/pull, pitfall/kneel, KO flat, heavy carry) and remaining 6 structurally audited bosses (Pokeys, Wiggler, Castle Bowser, Wendy, Roy, Larry).

3. **Koopa Guard Equipment Overhaul (`ZRI-016`)**
   - Replace remaining medieval blades held by Overworld/Castle Koopa guards with SMW Hammer Bro Hammers / spiked clubs.

4. **Title Screen BG2 Master Sword Removal (`ZRI-009`)**
   - Cleanly remove the vanilla Master Sword BG2 silhouette behind the metallic `PEACH` logo while preserving the 32-frame 3D Mario Trio intro.

5. **Custom Boss AI State Machines & Rideable Yoshi Mount (`ZRI-019`, `ZRI-020`)**
   - Expand Bank `$22` (`$22:C000..$22:FFFF`) with Koopaling/Bowser AI hooks and a rideable Yoshi mount (`$7E:F510..$7E:F51F`).

6. **Multi-Region & Physical Hardware Verification**
   - Add verified USA (NTSC 60 Hz) offset profiles and test on standalone bsnes, Ares, Snes9x, and physical SNES / Super Famicom + FXPAK Pro hardware.
