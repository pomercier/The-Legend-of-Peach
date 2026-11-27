# Gameplay State, Mechanics & Temporal QA Coverage Report — v0.9.0

**Target ROM:** `build/legend_of_peach_v0.9.0.sfc`  
**Verified SHA-256:** `5546dfaec62b98dc8c91bbbc58a7b16df6de91b9d8e6f0d56a1b7f1b36693fbd`  
**Automated QA Harness:** [`scripts/qa_engine_v090.py`](../scripts/qa_engine_v090.py)  
**33-Table Mega SQLite Validation Database:** Persisted locally outside git (`33/33` tables populated, `980+` rows, exported to local JSON & CSV snapshots)  
**Visual Evidence Plates:**
- **v0.9.0 The Ultimate Mario Adventure 2×5 Gallery:** [`docs/media/screenshots/24_v090_ultimate_mario_adventure_gallery.png`](media/screenshots/24_v090_ultimate_mario_adventure_gallery.png)
- **v0.9.0 Updated bsnes Title Screen (Zero Master Sword!):** [`docs/media/screenshots/01_title_screen.png`](media/screenshots/01_title_screen.png)
- v0.8.0 Super Mario Universe Revolution 4×4 Gallery: [`docs/media/screenshots/23_v080_mario_revolution_mechanics_gallery.png`](media/screenshots/23_v080_mario_revolution_mechanics_gallery.png)
- v0.7.0 All 12 Boss Arenas & 16 Mario States: [`20_v070_all_12_bosses_gallery.png`](media/screenshots/20_v070_all_12_bosses_gallery.png), [`22_v070_mario_16_states_gallery.png`](media/screenshots/22_v070_mario_16_states_gallery.png)

---

## 1. All 10 `v0.9.0` Ultimate Mario Adventure Live Scenarios (`10/10 VISUAL_PASS`)

| # | Scenario | Core | Zone / Room | Visually Verified Pixel Proof | Status |
|---:|---|---|---|---|---|
| 1 | `v090_01_title_peach_scepter_bsnes` | **`snes` (`bsnes`)** | Title Screen (`raw 0xA5`) | **ZERO Master Sword!** Gleaming SMW Golden Super Star + Princess Peach Jeweled Crown + Paratroopa Wings + Ornate Golden Scepter + Golden Orb finial behind `PEACH` with original sparkles intact | `VISUAL_PASS` |
| 2 | `v090_02_overworld_warp_pipe_action` | `snes_faust` | World 1 Yoshi's Island (`0x2C`) | Crisp 3D Glossy Emerald Green SMW Warp Pipe at `(r09, c07)` right outside Mario's front door; Ground Pounding beside it triggers native Warp Pipe SFX (`$012E=0x15`), Full Star Power, and **`+5` Coins (`250 -> 255 -> 260`)** | `VISUAL_PASS` |
| 3 | `v090_03_boss_gp_stagger_morton` | `snes_faust` | Morton's Volcanic Fortress (`0x05A`) | `DungPal[8]` Volcanic Basalt Fortress + 65c816 Boss Ground Pound Stagger (`+3` Coins) & Boss Hit Coin/Star Synergy (`+2` Coins) | `VISUAL_PASS` |
| 4 | `v090_04_wendy_coral_palace` | `snes_faust` | Wendy's Coral Sea Palace (`0x006`) | `DungPal[1]` (`0x0DD7E8`) Turquoise & Coral Sea masonry framing live Wendy O. Koopa + Boss GP Synergy rolling Coins **`250 -> 261 -> 268`** | `VISUAL_PASS` |
| 5 | `v090_05_lemmy_ice_fortress` | `snes_faust` | Lemmy's Star-Ice Fortress (`0x0DE`) | `DungPal[4]` (`0x0DDA04`) Glacial Frost-Cyan Ice Circus walls & diamond floor + Fire Mario melting Lemmy's giant spinning Ice Crystal | `VISUAL_PASS` |
| 6 | `v090_06_larry_desert_pyramid` | `snes_faust` | Larry's Chocolate Pyramid (`0x033`) | `DungPal[5]` (`0x0DDAB8`) Sun-Baked SMW Chocolate Island sandstone pyramid + Boss GP Stagger rolling Coins **`250 -> 253 -> 261`** | `VISUAL_PASS` |
| 7 | `v090_07_roy_midnight_castle` | `snes_faust` | Roy's Midnight Castle (`0x0AC`) | `DungPal[10]` (`0x0DDE3C`) Octagonal Midnight Violet-Slate & Magenta Koopa Castle with skylight beam | `VISUAL_PASS` |
| 8 | `v090_08_bowser_obsidian_keep` | `snes_faust` | Bowser's Obsidian Keep (`0x00C`) | `DungPal[13]` (`0x0DE058`) Dark Volcanic Obsidian brick masonry, lava-amber glow, and Giant Carved Bowser Head Statues | `VISUAL_PASS` |
| 9 | `v090_09_dark_world_bowser_citadel` | `snes_faust` | Bowser's Obsidian Citadel (`0x5B`) | Dark World volcanic crimson sky, Obsidian Pyramid masonry, and twin Jade Bowser Guard Statues | `VISUAL_PASS` |
| 10 | `v090_10_bsnes_final_bowser_synergy` | **`snes` (`bsnes`)** | King Bowser Throne (`0x000`) | Cycle-accurate `bsnes` parity: Fire Mario vs Final King Bowser, clean French dialogue (`BUG 07` zero regression), Boss GP Stagger & Synergy rolling Coins **`250 -> 259 -> 263`** | `VISUAL_PASS` |

---

## 2. Iterative Pass 1 -> Pass 2 Real Defect Caught & Resolved by Pixel-by-Pixel Audit

- **Pass 1 Catch (`V090-PASS1-01`):** Visual inspection of `v090_01_title_peach_scepter_bsnes` immediately caught that the vanilla Master Sword still appeared on the Title Screen because `0x004F73` was already offset `+0x73` from FR PAL ALTTP's true GFX table base `0x004F00` — meaning the Title Screen Master Sword lives in sprite sheet **`raw 0xA5` (`SP[0x32]`, PC `0x0B3CAD`)**, NOT BG `raw 0x32`!
- **Pass 2 Fix & Proof:** Repointed `mario_intro.rs` to `raw 0xA5`, added `0xA5` & `0x3E` to `audit_legend_of_peach_output()` (`51` audited sheets), updated the structural invariant test (`lib.rs`), rebuilt bit-identically, and verified **0 Master Sword pixels** on live cycle-accurate `bsnes` (`docs/media/screenshots/01_title_screen.png`).
