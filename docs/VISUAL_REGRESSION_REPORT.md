# Visual Regression & Hardware Verification Report (`v0.5.0`)

**Build Target:** `v0.5.0 — Super Mario Revolution`  
**Deterministic ROM SHA-256:** `93d3774f81119eb41d80b0c610d3b1bbcd1430c5f572f16b65b47b3ddd24975e`  
**SNES Checksum / Complement:** `0x87FF` / `0x7800` (Valid: `true`)  
**CRC32:** `9E8F84EB` | **MD5:** `74b5e46d58efa5f0f15cfe2b30383401` | **SHA-1:** `50db6eed1ab3495d877e0701c10974df4f252dd3`  
**Hardware Operations:** `220 patches across 18 rules` | **Bank Audit:** `49 compressed sheets verified, 11 relocated, 0 overlaps`

---

## 1. Resolution of All 5 Reported Visual Defects

| # | Defect Reported | Root Cause Diagnosed | v0.5.0 Fix Applied | Live Verification Evidence |
| :- | :--- | :--- | :--- | :--- |
| **1** | **File Select**: Garbled name `1. MA[U/n]IO`, vanilla pink Fairy cursor, blue Zelda sword, dull olive-green stone BG | Test SRAM used wrong FR char code `0x11` (`R = 0x21`); live File Select cursor (`$A8/$AA`) and sword (`$85/$A1`) live in compressed `raw 0xCF`, not `Sheet 0x76/0x78`; `$0E:96B2` used pink palette `0x7E`; BG palette `0x0DD9AA` was Zelda olive | Fixed FR SRAM encoding (`[0x0C,0x00,0x21,0x08,0x0E,0xA9]`), replaced `raw 0xCF` cursor with 2-frame Golden SMW Super Star (`$0E:96B2 = 0x30`), replaced sword with upright Super Mario Hammer, upgraded `0x0DD9AA` to SMW Royal Sky Blue | Visually verified in `docs/media/screenshots/14_file_select_mario.png` |
| **2** | **Toad NPC (Room `0x0103`)**: Distorted side face, red shoulder puffs, red belly dot | `mario_world.rs` assumed all 8 row-0 cells in `raw 0xC2/0xBD/0xC3` were heads; on `raw 0xC3`, col 10 row 0 is the sweeping villager's BODY, where `ClassicBack` mushroom cap had been placed | Audited vanilla `raw 0xC2/0xBD/0xC3` pixel-by-pixel, aligned every head/body cell to exact vanilla OAM slots, upgraded pixel art, and upgraded interior carpet (`0x0DD932`) to Royal Crimson & Gold | Visually verified in `docs/media/screenshots/15_toad_town_crimson.png` |
| **3** | **Kamek's Shop**: Broken digits, vanilla red heart, red potion, blue bomb | Previous showcase video mistakenly reused a stale pre-`v0.2.0` clip (`v13_clips/03_toad_shop`) | Upgraded shop carpet (`0x0DD932`) to Royal Crimson & Gold and recorded 100% fresh `v0.5.0` footage directly from `93d3774f...` showing crisp `150 / 10 / 50`, Super Mushroom, Mini-Mushroom, and 10 Jet-Black Bob-ombs | Visually verified in `docs/media/screenshots/03_magikoopa_shop.png` |
| **4** | **Iggy Koopa Boss (Room `0x0029`)**: Unreadable jumble of white rings & eyes | Mothula (`raw 0xAB`) OAM tile ordering did not match assumed grid | Rebuilt clean, crisp Winged Iggy Koopa + compact sparkle orb aligned to Mothula OAM layout | Visually verified in `docs/media/screenshots/18_boss_iggy_koopa.png` |
| **5** | **Bowser Room `0x0000` Dialogue & Arena**: Overwritten text (`¿ri·galet` / `ne ·auser`) + brown Zelda Pyramid stone + hyphenated line breaks | `v0.3.0` Bank `$1C` repacker emitted duplicate `0x75`/`0x76` line breaks (overwriting existing VRAM line pixels) and preserved vanilla syllable hyphens (`mot-<LB>suite`) | Fixed deterministic single-emission line-break state machine, eliminated 100% of hyphenated breaks across all 372 French messages (enforced by unit test), rewrote marquee Mario dialogues, and upgraded Room `0x0000` (`0x0DE2C2`) to Obsidian Dark-Charcoal & Lava Volcanic Stone | Visually verified in both `snes_faust` and cycle-accurate `bsnes`: `docs/media/screenshots/19_bowser_french_dialogue.png` & `09_boss_king_bowser.png` |

---

## 2. Honest Validation Matrix

| Subsystem | Implemented | Compiled | Unit-Tested | Runtime-Tested (`snes_faust`) | Cycle-Accurate (`bsnes`) | Visually Verified | Physical SNES Hardware |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| Top-Down Mario Jump (`L`/`R`, shadow, variable arc) | Yes | Yes | Yes | Yes | Yes | Yes | Untested |
| Mario Stomp Combat (`$22:9800`, puff, bounce, +1 coin) | Yes | Yes | Yes | Yes | Yes | Yes | Untested |
| Super Mario Hammer (melee, File Select, Start Menu) | Yes | Yes | Yes | Yes | Yes | Yes | Untested |
| Fire Mario Live CGRAM Swap & Fireballs | Yes | Yes | Yes | Yes | Yes | Yes | Untested |
| Super Starman Rainbow Invincibility (`L+R`) | Yes | Yes | Yes | Yes | Yes | Yes | Untested |
| Native SPC700 Mario SFX (Coin/Jump/Stomp/Pipe/Hammer) | Yes | Yes | Yes | Yes | Yes | Audio-active | Untested |
| Overworld Map32 JSON Engine & World 1 Screen `0x2C` | Yes | Yes | Yes (64 screens) | Yes | Yes | Yes | Untested |
| Bank `$1C` 372-Message FR Script (0 overlap, 0 hyphens) | Yes | Yes | Yes | Yes | Yes | Yes | Untested |
| Start Menu (Fire/Ice Flowers, Green Warp Pipe, Names) | Yes | Yes | Yes | Yes | Yes | Yes | Untested |
| Bit-for-Bit Build Determinism & Hardware Audit | Yes | Yes | Yes (29/29) | N/A | N/A | N/A | Compliant (Untested) |

---

## 3. De-Zeldafication Metrics (`v0.4.0` -> `v0.5.0`)

- **Zelda Residue Index (ZRI):** `94.9` (`371 / 391`) -> **`19.2`** (`75 / 391`) — **79.8% reduction**!
- **Mario Identity Index (MII):** `5.1` -> **`80.8`**!
- See [`docs/ZELDA_RESIDUE_REGISTRY.json`](ZELDA_RESIDUE_REGISTRY.json) for full item-by-item scoring and honest remaining v0.6.0 backlog.
