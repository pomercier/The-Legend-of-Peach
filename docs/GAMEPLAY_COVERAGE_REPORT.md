# Gameplay State, Mechanics & Temporal QA Coverage Report — v0.8.0

**Target ROM:** `build/legend_of_peach_v0.8.0.sfc`  
**Verified SHA-256:** `2270d55f8f4d09a164a939213531be80e4237a94488f535749708801ba925fd8`  
**Automated QA Harness:** [`scripts/qa_engine_v080.py`](../scripts/qa_engine_v080.py)  
**33-Table Mega SQLite Validation Database:** Persisted locally outside git (`33/33` tables populated, `640` rows, exported to local JSON & CSV snapshots)  
**Visual Evidence Plates:**
- **v0.8.0 Super Mario Universe Revolution 4×4 Gallery:** [`docs/media/screenshots/23_v080_mario_revolution_mechanics_gallery.png`](media/screenshots/23_v080_mario_revolution_mechanics_gallery.png)
- v0.7.0 All 16 Mario States: [`docs/media/screenshots/22_v070_mario_16_states_gallery.png`](media/screenshots/22_v070_mario_16_states_gallery.png)
- v0.7.0 All 12 Boss Arenas: [`docs/media/screenshots/20_v070_all_12_bosses_gallery.png`](media/screenshots/20_v070_all_12_bosses_gallery.png)
- v0.7.0 Transformed Worlds: [`docs/media/screenshots/21_v070_world_transformation_gallery.png`](media/screenshots/21_v070_world_transformation_gallery.png)

---

## 1. All 12 `v0.8.0` Super Mario Universe Revolution Live Scenarios (`12/12 VISUAL_PASS`)

| # | Scenario | Core | Zone / Room | Visually Verified Pixel Proof | Status |
|---:|---|---|---|---|---|
| 1 | `v080_01_ground_pound_onoff` | `snes_faust` | Tower of Hera ON/OFF Hall (`0x077`) | Mid-air Ground Pound triggers native Submodule `$16`, snapping the entire SMW ON/OFF block matrix `RED -> BLUE` (`f_006..f_010`), `BLUE -> RED` (`f_012..f_015`), and back to `BLUE` (`f_016`) | `VISUAL_PASS` |
| 2 | `v080_02_koopa_shell_kick` | `snes_faust` | Bowser Outpost Hall (`0x081`) | Mario stomps Green Koopa #1 (`+1` Coin `250 -> 251`), retracts it into Stunned Shell, kicks it high-speed upward (`f_007..f_009`), dual wall ricochet (`f_010..f_013`) & puff (`f_014`) | `VISUAL_PASS` |
| 3 | `v080_03_boo_freeze_ai` | `snes_faust` | Iggy's Indigo Ghost House (`0x059`) | Mario faces UP (`f_004..f_008`) -> all 3 Ghost House Boos (`0x8B`) freeze motionless at exact pixel coordinates; Mario turns DOWN (`f_009..f_014`) -> all 3 Boos creep south toward Mario! | `VISUAL_PASS` |
| 4 | `v080_04_ice_mario` | `snes_faust` | World 1 Yoshi's Island (`0x2C`) | Auto-equipped `FLEUR DE GLACE` (`$0202=0x07`) persists Electric Frost-Cyan / Glacier-Blue CGRAM across Overworld exit + launches sparkling Ice Flurries | `VISUAL_PASS` |
| 5 | `v080_05_cape_mario_glide` | `snes_faust` | World 1 Yoshi's Island (`0x2C`) | Auto-equipped `PLUME DE CAPE` (`$0202=0x13`) persists Blazing SMW Golden Cape CGRAM + executes extended `-0.75 px/f` slow-descent mid-air glides | `VISUAL_PASS` |
| 6 | `v080_06_yoshi_companion_shield` | `snes_faust` | World 1 Yoshi's Island (`0x2C`) | Auto-equipped `FILET YOSHI` (`$0202=0x0E`) mounts Yoshi (Emerald Saddle + Orange Boots), executes Flutter Jump, drops into Blue Koopa -> **SMW 1-Hit Shield absorbs hit (`14/14` FULL HEARTS!)**, dismounts to Classic Red/Blue (`f_013..f_014`), then cleanly remounts (`f_015..f_016`) | `VISUAL_PASS` |
| 7 | `v080_07_balanced_starman` | `snes_faust` | World 1 Yoshi's Island (`0x2C`) | `L+R` Rainbow Starman cycles 4 palettes while steadily draining green Star Power meter to empty (`f_001 -> f_013`) and plowing through Blue Koopa (`+1` Coin `250 -> 251`) | `VISUAL_PASS` |
| 8 | `v080_08_turnblock_coin_reward` | `snes_faust` | World 1 Yoshi's Island (`0x2C`) | Mario lifts Smiling Bush (`f_006..f_008`) and throws it (`f_009`) -> Target Coins (`$7E:F360`) immediately roll up **`250 -> 251`** | `VISUAL_PASS` |
| 9 | `v080_09_morton_lava_fortress` | `snes_faust` | Morton's Volcanic Fortress (`0x05A`) | `DungPal[8]` (`0x0DDCD4`) deep volcanic basalt walls & glowing magma-amber trim framing Morton Koopa | `VISUAL_PASS` |
| 10 | `v080_10_iggy_ghost_house` | `snes_faust` | Iggy's Indigo Ghost House (`0x029`) | `DungPal[9]` (`0x0DDD88`) spooky SMW indigo timber & lavender parquet + razor-sharp anchored Iggy wings (`v0.6.0` BUG 06 zero regression) | `VISUAL_PASS` |
| 11 | `v080_11_ludwig_mech_fortress` | `snes_faust` | Ludwig's Mech Fortress (`0x0A4`) | `DungPal[12]` (`0x0DDFA4`) industrial forged iron & riveted bronze plating framing Ludwig Molten Mech Bowser | `VISUAL_PASS` |
| 12 | `v080_12_bsnes_final_bowser_gp` | **`snes` (`bsnes`)** | King Bowser Throne (`0x000`) | Cycle-accurate `bsnes` parity: Fire Mario vs King Bowser, crisp French dialogue (`v0.6.0` BUG 07 zero regression), Ground Pound & combat | `VISUAL_PASS` |

---

## 2. Iterative Pass 1 -> Pass 2 Real Bugs Caught & Resolved by Pixel-by-Pixel Audit

Live visual QA caught **5 real hardware bugs** during Pass 1 that `DYNAMIC_PASS` motion metrics alone would have missed — all 5 were root-caused via 65c816/ROM disassembly, fixed, and verified `VISUAL_PASS` on Pass 2:
1. **`$7E:F362` vs `$7E:F360` Coins:** ALTTP reconciles Display Rupees (`$F362`) back to Target Rupees (`$F360`) every frame -> fixed `Mario_AddCoinsAndStar` to mutate `$7E:F360/$F361`.
2. **Overworld Exit CGRAM Reset + `FILET YOSHI` ID (`0x0E`):** Overworld exit reloaded tunic CGRAM after fade-in and `$7E:F34D` maps to `$0202=0x0E` (not `0x0D`) -> added transition `W_PAL_DIRTY` + 8-frame non-Classic CGRAM guard + `0x0E` check + 60f Yoshi dismount escape.
3. **Ground Pound Double-Flip in 2-Switch Rooms:** Toggling `$7E:C172` per `0x1E` sprite flipped `0 -> 1 -> 0` in one frame -> added `64px` proximity + `W_TMP_B` once-per-impact guard.
4. **Dungeon Palette Mapping (`$07:FA90`):** Disassembled `$01:B609` -> `$07:FA90` to patch exact `DungPal[8]` (`0x0DDCD4`), `DungPal[9]` (`0x0DDD88`), and `DungPal[12]` (`0x0DDFA4`).
5. **Ghost House Boo ID (`0x8B`):** Extended `STOMP_TABLE` class `0x04` Orientation-Freeze AI to Skull Woods Ghost House Boos (`0x8B`) alongside Overworld Graveyard Boos (`0x19`).
