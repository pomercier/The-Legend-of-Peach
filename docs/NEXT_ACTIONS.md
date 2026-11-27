# Next Actions & Release Queue (Post-v0.7.0 -> v0.8.0 Super Mario Universe Revolution)

## Completed in v0.7.0 (`f975131b5eaeee99ee7215ae636a380946ddc48832d3e7be95a11ab5d344dfee`)
- [x] Full binary discovery & audit of all `160/160` Overworld screens, `296/296` Underworld rooms, `210` unique placed sprite IDs, and `133` entrance links.
- [x] Systemic collision-verified Overworld transformation across `108/128` Light & Dark World screens (`56/64` LW + `52/64` DW, `322` Map32 tiles, `0` grammar violations, `0` collision regressions, `0` cross-screen border breaks).
- [x] Distinct Dark World artistic identity: Bowser's Volcanic Kingdom (`0x0DE70E` / `0x0DE79A`) + Bowser's Obsidian Lava Citadel (`0x0DEB36`, Screen `0x5B/0x5C/0x63`).
- [x] Koopa Guard weapon conversion (`raw 0xBA` -> SMW Hammer Bro Hammers & Spiked Clubs, `ZRI-016` eliminated) + Castle Tower Bowser dialogues (`#316/#318/#319`).
- [x] **100% Live `VISUAL_PASS` across ALL 12/12 Bosses** (including live Roy Koopa trigger in Room `0x0AC`) and **ALL 16/16 Mario States** (closing all 6 `v0.6.0` live gaps: Heavy Carry/Throw, Deep Water Swim, Dive, Push Block, Electric Shock X-Ray, Pitfall Drop, KO Spin/KO Flat).
- [x] Reusable multi-worker Mednafen QA engine (`scripts/qa_engine_v070.py`) + local persistent SQLite validation database (`94` historical runs).

## Immediate Next Milestone: **v0.8.0 — Super Mario Universe Revolution**
1. **Mario Universe Bible (`docs/MARIO_UNIVERSE_REGISTRY.json`) & Transparent Quality Scores (`docs/QUALITY_SCORES.json`)**.
2. **Mega Normalized SQLite Validation Database (`33` entities + JSON/Parquet exports)** + native frame-by-frame temporal verification harness.
3. **Deep 65c816 Mario Gameplay Mechanics (`Bank $22`, WRAM `$7E:F500..$7E:F53F`):**
   - **Koopa Shell Retract, Kick & Wall-Ricochet Bowling Engine** (stomp Koopa -> stun/shell -> kick shell into walls & other enemies!).
   - **Mario Ground Pound (`Down` in mid-air -> fast slam + screen shake + shockwave stun/armor break + switch activation!)**.
   - **Authentic SMW Boo Orientation-Freeze AI** (Boos freeze shyly when Mario faces them and creep closer when Mario turns his back!).
   - **Expanded Power-Up State Machine:** Balanced Starman Star-Power drain (eliminating infinite invincibility exploit), **Ice Mario** live palette (`$22:9900`) & enemy freeze, **Cape Mario** mid-air glide (`Hold R`).
   - **Playable Yoshi Companion System:** Mountable SMW Yoshi state (`$7E:F518`) with +speed, Flutter Jump apex boost, and 1-hit damage shield!
4. **Physical Overworld Warp Pipe Entrances & Distinct Koopaling Dungeon Fortress Palettes**.
5. **Physical SNES / FXPAK Pro & Additional Emulator Matrix** (requires physical hardware / standalone `ares`/`Snes9x`).
