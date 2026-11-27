# Boss Visual Authenticity & OAM Verification Report (`v0.6.0`)

## Summary of `v0.6.0` Boss Overhauls

### 1. Koopaling #3 — Winged Iggy Koopa (Skull Woods / Mothula — BUG 06)
- **User Directive:** Preserve Iggy Koopa's body **100% untouched**; fix only the floating, detached wings (`media_1795753884747_0cd5b58a.jpg`).
- **Root Cause:** Mothula's OAM coordinate table at `PC 0x0D7CED` (`$1A:FCED`, 128 bytes) positioned left/right wings at `dx = ±24`, leaving an ~8-pixel open gap away from Iggy's shoulders.
- **Fix (`crates/snes-core/src/mario_bosses.rs`):**
  - Left Iggy's body pixel art (`cols 16..31` of `raw 0xAB`) completely untouched.
  - Sculpted authentic SMW `GFX01` Paratroopa feathered wings (`cols 0..15`) firmly anchored at `x = 15` (inner shoulder attachment edge).
  - Patched `PC 0x0D7CED` pulling Left/Right wings inward from `dx = ±24` to **`dx = ±17`** with coordinated upstroke (`dy = -10 / +6`) and downstroke (`dy = -6 / +10`) flap offsets.
- **Verification (`06_bug06_iggy_wings`, 4 fps):** Live frames `f_013..f_016` confirm Iggy's body is identical to `v0.5.0` while both feathered Paratroopa wings stay solidly connected to his shoulders throughout every flap cycle.

---

### 2. Final Boss — Giant King Bowser (Pyramid of Power Room `0x00` / Ganon — BUG 07)
- **Defects in `v0.5.0` (`media_1795753888717_b9eb3607.png`):**
  1. Ganon's held Zelda Trident (`JSR $9C1C` at `PC 0x0E9AF5`) rendered a blue/beige orb and diagonal spear stick directly over Bowser's chest and belly.
  2. In standing/dialogue Frames 2..4 (`$1D:969A`), Head `dy = -10 (0xF6)` sat 3px below upper torso `dy = -13`, putting the torso's black collar band on Bowser's forehead.
  3. Ganon's paired `dy = +8` arm sprites (`$06/$16`, `$36/$46`, `$08/$18`, `$38/$48`, `$0A/$1A`) cross VRAM sheet boundaries between `sp:21` (`raw 0x94`) row 3 and `sp:41` (`raw 0xB4`) row 0; writing 16×32 strips caused `$38` to draw upside-down white claws from `arm_down` on top of `arm_up`'s green forearm.
- **Fixes (`crates/snes-core/src/mario_bosses.rs`):**
  - **Held Trident Removal:** Patched `PC 0x0E9AF5` (`20 1C 9C` → `EA EA EA`, `NOP NOP NOP`), removing 100% of held Trident sprites while keeping Bowser's thrown projectile attacks (`Sprite 0xCE`) intact.
  - **Authentic SMW `GFX21` King Bowser Head & Shell:** Rebuilt `build_final_bowser_front_head_left_16x16` (sweeping curved ivory bull horns, wild crimson mane, fierce red eyebrows, green brow, wide arched golden snout & fanged roar) and `build_final_bowser_torso_left_16x32` (massive hunched emerald spiked shell rising behind his shoulders, black spiked collar tucked cleanly under his chin at `y=12..15`, wide horizontal-ribbed golden belly). Lifted Frame 2..4 Head `dy` in `PC 0x0E969A` from `-10` to **`-15 (0xF1)`**.
  - **Exact Disjoint 16×24 Continuous Arm Strips (`build_final_bowser_arm_24` + `write_arm_band_16x8`):** Mapped all 8 paired arm slots (`$06/$16`, `$36/$46`, `$08/$18`, `$38/$48`, `$0A/$1A`, `$3A/$4A`, `$0C/$1C`, `$3C/$4C`) across strictly non-overlapping 8px bands of `sp:21` and `sp:41`, producing muscular emerald arms with spiked black leather wrist cuffs and compact clawed fists across all 14 animation frames.
  - **Official SMW `GFX24` Giant Fireballs:** Extracted Nintendo's SMW Giant Bowser Fireball tiles directly from `roms/super_mario_world.sfc` (`GFX24`) for Bowser's projectile attacks.
- **Dual-Core Verification (`07_bug07_bowser_final` + `09_bsnes_bowser`):** Live frames across `snes_faust` and cycle-accurate `snes` (bsnes) show identical, glitch-free rendering across entrance, French dialogue (zero overlap, zero hyphens), roaring arm-raise (`f_012..f_013`), and active combat (`f_014..f_016`).

---

## Complete 12-Boss Audit Table

| # | Boss | Sheets Modified | `v0.6.0` Live Emulated? | Notes |
| ---: | :--- | :--- | :---: | :--- |
| 1 | 6 Chargin' Chucks | `raw 0xA8` | Verified in `v0.5.0` | Structurally audited in `v0.6.0` |
| 2 | 3 Giant Pokeys | `raw 0xA9` | Structurally audited | — |
| 3 | Giant Wiggler | `raw 0xAA` | Structurally audited | — |
| 4 | Castle Tower Bowser | `mario_ui.rs` | Structurally audited | — |
| 5 | Morton Koopa Jr. | `raw 0xAC` | Verified in `v0.5.0` | Structurally audited in `v0.6.0` |
| 6 | Wendy O. Koopa | `raw 0xAD` | Structurally audited | — |
| 7 | **Winged Iggy Koopa** | `raw 0xAB` + `$1A:FCED` | **YES (`v0.6.0` `snes_faust`)** | **BUG 06 FIXED** |
| 8 | Roy Koopa | `raw 0xAE` | Structurally audited | — |
| 9 | Lemmy Koopa | `raw 0xAF` + `0x0DD55E` | Verified in `v0.5.0` | Structurally audited in `v0.6.0` |
| 10 | Larry Koopa | `raw 0xB0` | Structurally audited | — |
| 11 | Ludwig von Koopa | `raw 0xB2`/`0xB3` | Verified in `v0.5.0` | Structurally audited in `v0.6.0` |
| 12 | **Final King Bowser** | `raw 0x94, 0xB4, 0xB8, 0xA6`, `0x0E9AF5`, `0x0E969A` | **YES (`v0.6.0` `snes_faust` + `bsnes`)** | **BUG 07 FIXED** |
