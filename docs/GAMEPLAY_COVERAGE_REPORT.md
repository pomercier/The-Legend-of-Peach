# Gameplay State, Boss & Temporal QA Coverage Report — v0.7.0

**Target ROM:** `build/legend_of_peach_v0.7.0.sfc`  
**Verified SHA-256:** `f975131b5eaeee99ee7215ae636a380946ddc48832d3e7be95a11ab5d344dfee`  
**Automated QA Harness:** [`scripts/qa_engine_v070.py`](../scripts/qa_engine_v070.py)  
**Local SQLite Validation Database:** Persisted locally outside git (`qa_runs` + `qa_run_history`)  
**Visual Evidence Plates:**
- All 16 Mario States: [`docs/media/screenshots/22_v070_mario_16_states_gallery.png`](media/screenshots/22_v070_mario_16_states_gallery.png)
- All 12 Boss Arenas: [`docs/media/screenshots/20_v070_all_12_bosses_gallery.png`](media/screenshots/20_v070_all_12_bosses_gallery.png)
- Transformed Light & Dark Worlds: [`docs/media/screenshots/21_v070_world_transformation_gallery.png`](media/screenshots/21_v070_world_transformation_gallery.png)

---

## 1. All 16 Mario States — 100% Live `VISUAL_PASS` (Closing All 6 `v0.6.0` Gaps!)

| # | Mario State | Previously Un-Emulated in v0.6.0? | Verified Scenario & Core | Observed Visual Proof | Status |
|---:|---|---|---|---|---|
| 1 | Idle & Walk (4-Dir) | Verified in v0.6.0 | `mario_state_06_core_fire_starman_bsnes` (`bsnes`) | Crisp Mario cap/overalls across 4 directions | `VISUAL_PASS` |
| 2 | Top-Down Mario Jump & Shadow | Verified in v0.6.0 | `mario_state_06_core_fire_starman_bsnes` (`bsnes`) | 22px parabolic jump above clean ground shadow | `VISUAL_PASS` |
| 3 | Mario Stomp Combat | Verified in v0.6.0 | `mario_state_06_core_fire_starman_bsnes` (`bsnes`) | Stomp bounce + puff + coin increment | `VISUAL_PASS` |
| 4 | Super Mario Hammer Melee | Verified in v0.6.0 | `boss_01_chargin_chucks` + `mario_state_03` (`snes_faust`) | Clean Mario Hammer arc, 0 Zelda sword | `VISUAL_PASS` |
| 5 | Fire Mario 4-Dir Fireball | Verified in v0.6.0 | `mario_state_06` (`bsnes`) + `boss_09_lemmy` (`snes_faust`) | Live white cap / crimson overalls CGRAM swap + fireball cast | `VISUAL_PASS` |
| 6 | Super Starman Rainbow | Verified in v0.6.0 | `mario_state_06_core_fire_starman_bsnes` (`bsnes`) | Flashing 4-palette rainbow invincibility + clean restore | `VISUAL_PASS` |
| 7 | Pause Menu & Yoshi Eggs | Verified in v0.6.0 | `boss_09_lemmy` (`snes_faust`) | `BILL BLASTER`, `FLEUR DE FEU`, `YOSHIS`, `EQUIPEMENT` | `VISUAL_PASS` |
| 8 | Koopa Contact Hurt/Recoil | Verified in v0.6.0 | `mario_state_05_ko_flat` (`snes_faust`) | Clean contact recoil vs Hammer Bro Koopa (0 head duplication) | `VISUAL_PASS` |
| 9 | **Heavy Lift & Carry (4-Dir)** | **NEW in v0.7.0** | `mario_state_01_heavy_carry` (`snes_faust`) | Mario lifts SMW Smiling Bush overhead, walks Right/Up/Down/Left | `VISUAL_PASS` |
| 10 | **Prop Throw** | **NEW in v0.7.0** | `mario_state_01_heavy_carry` (`snes_faust`) | Clean throw animation + leaf burst (`f_013..f_014`) | `VISUAL_PASS` |
| 11 | **Deep Water Swim (4-Dir)** | **NEW in v0.7.0** | `mario_state_02_swim_dive` (`snes_faust`, Lake Hylia `0x3E`) | Crisp Mario cap/profile swimming Up/Left/Right/Down beside Cheep-Cheep | `VISUAL_PASS` |
| 12 | **Underwater Dive** | **NEW in v0.7.0** | `mario_state_02_swim_dive` (`snes_faust`, Lake Hylia `0x3E`) | Underwater dive submerge + surface splash ripple (`w_007..w_010`) | `VISUAL_PASS` |
| 13 | **Push Block Pose** | **NEW in v0.7.0** | `mario_state_04_push_pull` (`snes_faust`, Sanctuary) | Clean two-handed Mario push pose against Sanctuary Altar | `VISUAL_PASS` |
| 14 | **Electric Shock X-Ray** | **NEW in v0.7.0** | `mario_state_03_electric_shock` (`snes_faust`, Buzzblob `0x0D`) | PPU mosaic flash (`f_008`) + blue/black Mario X-Ray Shock (`f_009`) + clean recovery | `VISUAL_PASS` |
| 15 | **Pitfall Fall & Drop (`-1F`)** | **NEW in v0.7.0** | `boss_03_wiggler_and_pitfall` (`snes_faust`, Room `0x007`) | Pitfall shrink (`f_004..f_005`), ceiling drop (`-1`), clean landing (`f_008..f_010`) | `VISUAL_PASS` |
| 16 | **KO Spin & KO Flat** | **NEW in v0.7.0** | `mario_state_05_ko_flat` (`snes_faust`) | Full red Game Over spin (`f_012..f_013`) + flat KO Mario pose (`f_014`), 0 Link residue | `VISUAL_PASS` |

---

## 2. QA Run Summary (`23 / 23 VISUAL_PASS` on Final ROM `f975131b...dfee`)

- **Suite A — All 12 Bosses:** `12 / 12 VISUAL_PASS` (`11` on `snes_faust` + Final King Bowser on cycle-accurate `snes`/bsnes).
- **Suite B — Mario States & Non-Regression Guards:** `6 / 6 VISUAL_PASS` (including `mario_state_06` on `bsnes`).
- **Suite C — Transformed World & NPC Galleries:** `5 / 5 VISUAL_PASS` (`16-bit Toad 0x0103`, `Royal Blue Kamek 0x011F`, `Bowser Obsidian Citadel 0x5B` on BOTH `snes_faust` and `bsnes`, Overworld Sanctuary tour).
- **Zero Crashes, Zero Softlocks, Zero Regressions of `v0.6.0` BUG 01..07.**
