# Next Actions & Roadmap (Post-v0.8.0 -> v0.9.0 The Ultimate Mario Adventure)

## Completed in v0.8.0 (`2270d55f8f4d09a164a939213531be80e4237a94488f535749708801ba925fd8`)
- [x] **Mario Universe Bible (`docs/MARIO_UNIVERSE_REGISTRY.json`)** cataloging `72` canonical Mario elements across `12` categories + **Transparent Multi-Dimensional Quality Scores (`docs/QUALITY_SCORES.json`)** raising **Mario Identity Score to `91.8%`** (`ZRI` reduced from `94.9% -> 19.2% -> 13.2% -> 8.2%`).
- [x] **33-Table Normalized Mega SQLite Validation Database** (`scripts/qa_engine_v080.py`) with `33/33` tables populated (`640` rows) and automated local CSV/JSON export.
- [x] **Native 65c816 Super Mario Universe Engine (`mario_engine.rs`, Bank `$22:9000..$22:B2FF`, WRAM `$7E:F500..$7E:F53F`):**
  - **Mario Ground Pound (`Down` mid-air):** 6f hang -> `-5 px/f` slam -> SNES hardware screen shake (`$011A`), armored enemy break, `+2` Coins (`$7E:F360`), AND **live hardware Dungeon ON/OFF Crystal Switch flip (`$7E:C172 ^= 1`, Submodule `$16`)** visually verified flipping Room `0x077` `RED <-> BLUE`!
  - **Real Koopa Shell Retract, Kick & Dual Wall-Ricochet Bowling (`class 0x03`):** Visually verified in Room `0x081` (`Stomp -> +1 Coin -> Kick -> Dual Wall Ricochet`).
  - **Authentic SMW Boo Orientation-Freeze AI (`class 0x04`, `0x19` & `0x8B`):** Visually verified in Room `0x059` (all 3 Ghost House Boos freeze solid when Mario faces them and creep stealthily toward Mario the instant he turns his back).
  - **6-Form Live CGRAM Power-Up State Machine (`$22:B100`, 270 B, 8f transition persistence guard):** Classic, Fire Mario, **Ice Mario** (Frost-Cyan + Ice Stomp Shatter), **Cape Mario** (SMW Golden Trim + `-0.75 px/f` Slow-Descent Glide), **Playable Yoshi Companion** (`FILET YOSHI 0x0E`, Emerald Saddle, Higher Jump, 16f Flutter Jump, and **SMW 1-Hit Damage Shield** absorbing 100% HP loss at `14/14` hearts!), and **Resource-Balanced Super Starman** (continuous Star-Power drain).
  - **Turn-Block / Bush Throw Reward:** Throwing any lifted Smiling Bush or Turn-Block increments Target Coins (`250 -> 251`) + refills Star Power.
- [x] **Distinct Koopaling Dungeon Fortress Architectural Palettes (`mario_world.rs`, verified via `$07:FA90`):** Morton's Volcanic Basalt Fortress (`DungPal[8]` @ `0x0DDCD4`), Iggy's Indigo SMW Ghost House (`DungPal[9]` @ `0x0DDD88`), and Ludwig's Molten Mech Fortress (`DungPal[12]` @ `0x0DDFA4`).
- [x] **12/12 Live `VISUAL_PASS`** on final deterministic ROM (`snes_faust` + cycle-accurate `bsnes`), `32/32` Rust tests passing, `0` clippy warnings, `0` regressions across all `v0.6.0` & `v0.7.0` guarantees.

## Next Milestone: **v0.9.0 — The Ultimate Mario Adventure**
1. **Title Screen BG2 Master Sword Silhouette Elimination (`ZRI-009`, 10 pts):** Replace residual BG2 Title Screen sword tiles with an authentic Super Mario World Castle / Peach Crown crest backdrop.
2. **Physical Overworld Warp Pipe Entrances & Network (`ZRI-017`, 10 pts):** Sculpt physical Map32 Green SMW Warp Pipes into key Light & Dark World portals connecting Toad Town, Yoshi's Island, and Bowser's Volcanic Kingdom.
3. **Custom 65c816 Koopaling & King Bowser Behavioral Phase Hooks (`ZRI-019`, 2 pts):** Add distinct Mario-flavored boss attack telegraphs and coin/star drops per Koopaling.
4. **Expanded Multi-Biome Dungeon Palettes & Optional 100% Overworld Polish (`ZRI-007` / `ZRI-008`):** Give Lemmy's Ice Fortress, Wendy's Aquatic Palace, and Roy's Castle dedicated SMW architectural accents.
5. **Physical SNES / FXPAK Pro & Multi-Emulator Matrix (`v1.0.0`):** Validate on real unmodified PAL 50 Hz SNES hardware and additional standalone emulators (`ares`, `Snes9x`, `Mesen 2`), plus study USA NTSC 60 Hz offset profile.
