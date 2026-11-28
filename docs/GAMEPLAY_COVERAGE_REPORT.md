# Gameplay State, Mechanics & Temporal QA Coverage Report — v0.20.0

**Release:** `v0.20.0 — Project Nintendo Limit Break`  
**Target ROM:** `build/legend_of_peach_v0.20.0.sfc` (`2,097,152` bytes, 2 MiB LoROM)  
**Verified SHA-256:** `5e9c06a5db1ffc6be4879e258fa914bfdd56db101455082c7d2fb711df9cb910`  
**Output CRC32:** `0x7A5A145B` | **SNES Checksum:** `0x5547` / `0xAAB8` (`Valid: true`)  
**Automated QA Harness:** [`scripts/qa_engine_v0200.py`](../scripts/qa_engine_v0200.py)  
**33-Table Mega SQLite Validation Database:** Persisted locally outside git (`33/33` tables populated, `1,218` rows)  
**Visual Evidence Plates:**
- **v0.20.0 Project Nintendo Limit Break 12-Panel Live Gallery:** [`docs/media/screenshots/27_v0200_limit_break_gallery.png`](media/screenshots/27_v0200_limit_break_gallery.png)
- **v0.20.0 Direct Before (`v0.10.0`) vs After (`v0.20.0`) Comparison Plate:** [`docs/media/screenshots/28_v0200_bugfixes_before_after.png`](media/screenshots/28_v0200_bugfixes_before_after.png)
- Prior Milestone Galleries: [`25_v0100_beyond_limits_gallery.png`](media/screenshots/25_v0100_beyond_limits_gallery.png), [`26_v0100_before_after_comparison.png`](media/screenshots/26_v0100_before_after_comparison.png), [`24_v090_ultimate_mario_adventure_gallery.png`](media/screenshots/24_v090_ultimate_mario_adventure_gallery.png)

---

## 1. All 15 `v0.20.0` Limit Break Live Scenarios (`15/15 VISUAL_PASS + AUDIO_PASS`)

| # | Scenario | Core | Zone / Room | Visually Verified Pixel, Audio & SRAM v2 Proof | Status |
|---:|---|---|---|---|---|
| 1 | `v0200_01_bug1_hedge_guard_and_star_monument` | `snes_faust` | Toad Town Star Plaza (`0x19`) | **USER BUG #1 FIX (`$29=#$FF`):** Mario jumping & spin-jumping against solid east hedge is 100% blocked; **Golden Super Star Monument** replaces Weathervane Bird (0 rooster pixels); Open Green Warp Pipe at `(14,1)` | `VISUAL_PASS` + `AUDIO_PASS (-26.5 dB)` |
| 2 | `v0200_02_smooth_bidirectional_pipes_faust` | `snes_faust` | `0x2C -> 0x19 -> 0x0B -> 0x2C` | **USER BUG #1b & #2 FIX:** Locked smooth pipe descent + Z-hop emergence; clean Vanilla Dome boulders (**0 olive halos**); enemy-free pipe `(9,2)`; Express Return Home (`Up`); SRAM v2 valid (`coins=280, dragon_cnt=3, pipe_disc=0x07, magic="MP", v2, CRC OK`) | `VISUAL_PASS` + `AUDIO_PASS (-25.4 dB)` |
| 3 | `v0200_03_bidirectional_pipes_bsnes_parity` | **`snes` (`bsnes`)** | `0x2C -> 0x19 -> 0x0B -> 0x19 -> 0x2C` | **Cycle-accurate `bsnes` 4-Hop Bidirectional Tour:** Forward -> Forward -> **Reverse (`Left`)** -> **Express Home (`Up`)** (`pipe_total=4, coins=280, dragon_cnt=3, pipe_disc=0x07, SRAM v2 CRC OK`, 0 VRAM seams) | `VISUAL_PASS` + `AUDIO_PASS (-25.6 dB)` |
| 4 | `v0200_04_four_heroes_peach_hover_toad` | `snes_faust` | Yoshi's Island (`0x2C`) | Live cycle across **4 Playable Heroes**: Super Mario -> Luigi (Pal 9) -> **Princess Peach** (Pal 11 + **28f SMB2 Dress Hover**) -> **Captain Toad** (Pal 12 + agile hop + `+2` Throw Coin); SRAM v2 `char_id=3, quest_lo=0x20, CRC OK` | `VISUAL_PASS` + `AUDIO_PASS (-26.3 dB)` |
| 5 | `v0200_05a_spin_jump_spiny_room017` | `snes_faust` | Tower of Hera (`0x017`) | SMW Spin Jump (`R`) & Ground Pound vs Buzzy Beetle/Spiny (`0x26`) with zero crash and clean pit-safe respawn | `VISUAL_PASS` + `AUDIO_PASS (-25.6 dB)` |
| 6 | `v0200_05b_spin_jump_drybones_room026` | `snes_faust` | Dungeon Room `0x026` | SMW Spin Jump (`R`) permanently shatters **Dry Bones (`0xA7`)** with shatter puffs & Coin reward (`250 -> 252 -> 254`) | `VISUAL_PASS` + `AUDIO_PASS (-25.8 dB)` |
| 7 | `v0200_06_cape_360_spin_onoff_switch` | `snes_faust` | Tower Room `0x077` | **Cape Mario 360° Spin Attack (`B`/`Y`)** stuns foe (`250 -> 252c`) and **flips SMW ON/OFF Crystal Switch** (blue blocks rise, Coins `252 -> 268 -> 274`) | `VISUAL_PASS` + `AUDIO_PASS (-27.2 dB)` |
| 8 | `v0200_07_koopa_shell_and_bobomb_room031` | `snes_faust` | Tower Room `0x031` | Live room with `2×` SMW ON/OFF Crystal Switches (`0x1E`) & `10×` Buzzy Beetles (`0x26`): shell retract/kick, Spin Jump & Ground Pound | `VISUAL_PASS` + `AUDIO_PASS (-25.3 dB)` |
| 9 | `v0200_08_lake_cheepcheep_and_forest_hub4` | `snes_faust` | `0x2C -> Reverse -> Hub 4 (0x12)` | **Bidirectional Reverse Warp (`Left`)** directly from Yoshi's Island to River Hub 4 (`0x12`, `250 -> 260c`) + glossy green Warp Pipe + **SMW Frog Suit** auto-swim beside Koopa Guards | `VISUAL_PASS` + `AUDIO_PASS (-26.6 dB)` |
| 10 | `v0200_09a_bestiary_swooper_mu061_room0d0` | `snes_faust` | Dark Cavern (`0x0D0`) | Live verification of **`MU-061` SMW Swooper Bats (`0x6F`)** & Koopa Guards (`0x41/0x44`) (`250 -> 252c`) | `VISUAL_PASS` + `AUDIO_PASS (-25.7 dB)` |
| 11 | `v0200_09b_bestiary_piranha_rex_dw06c` | `snes_faust` | Bowser Obsidian Citadel (`0x5B`) | Live verification of **`MU-060` SMW Rex (`0x12`)** patrolling below Bowser's Obsidian Citadel & carved Bowser statues | `VISUAL_PASS` + `AUDIO_PASS (-28.2 dB)` |
| 12 | `v0200_10_sram_v2_3slot_migration_and_recovery` | `snes_faust` | 3-Slot Concurrent SRAM v2 | **Slot 1:** lossless `v0.10.0` Luigi migration (`char_id=1, dragon_cnt=3, CRC OK`); **Slot 2:** corrupted CRC `0xBEEF` + `char_id=0xFF` recovered cleanly (`char_id=3, dragon_cnt=0, CRC OK`); **Slot 3:** Princess Peach (`char_id=2`) 100% isolated | `PLAYTHROUGH_PASS` + `AUDIO_PASS (-27.8 dB)` |
| 13 | `v0200_11_bsnes_final_bowser_multiphase` | **`snes` (`bsnes`)** | King Bowser Throne (`0x000`) | Cycle-accurate `bsnes`: Fire Mario vs Final King Bowser Multi-Phase Escalation (`250 -> 277 -> 279 -> 282 -> 285c`), clean French dialogue (0 BUG 07 regression) | `VISUAL_PASS` + `AUDIO_PASS (-24.4 dB)` |
| 14 | `v0200_12_extended_blank_sram_playthrough` | `snes_faust` | 100% Blank 8 KiB SRAM Cold Boot | Full cold boot -> Title -> Slot 1 -> Legend -> Uncle Luigi -> Star-Torch Chest -> Rainy Overworld Jump & Exploration -> `Sauvegarder et quitter.` -> Cold Reboot Reload + deterministic [`PLAYTHROUGH_REPLAY_MANIFEST.json`](PLAYTHROUGH_REPLAY_MANIFEST.json) | `PLAYTHROUGH_PASS` + `AUDIO_PASS (-24.4 dB)` |
| 15 | `v0200_13_v0100_baseline_compare` | `snes_faust` | Immutable `v0.10.0` Reference ROM | Side-by-side empirical capture of `v0.10.0` baseline (hedge bug, Weathervane Bird, Vanilla Dome olive halos & stuck Deadrock) for [`28_v0200_bugfixes_before_after.png`](media/screenshots/28_v0200_bugfixes_before_after.png) | `VISUAL_PASS` + `AUDIO_PASS (-26.0 dB)` |

---

## 2. Iterative Pass 1 -> Pass 2 -> Final Pixel Audit Catches Resolved Before Release

Pixel-by-pixel inspection of Pass 1 caught **4 real defects** that were fixed and re-verified across all 15 scenarios:
1. **`V0200-PASS1-01` (Toad Town Star Monument):** Leftover rooster head (`m16 0x41A/0x41D`) and weathervane tail arrow (`m16 0x437`) still flanked our star, and `AuxRow 0` rendered indices `5/6` as red -> cleared those 3 weathervane-exclusive Map16 entries to grass `0x0CED` and repainted `raw 0x53` using pure gold/white (`4/7/3/2`) atop a royal Mario red velvet pedestal (`5/6/7`).
2. **`V0200-PASS1-02` (Lake Cheep-Cheep Whirlpool Clip):** Shifting `$20 += 7px` South during pipe emergence dipped Mario into `Slot 9`'s whirlpool -> replaced Y shift with a coordinate-safe `$24` Z-altitude SMW Pipe Pop-Out Parabolic Hop (`0 -> 14px -> 0`).
3. **`V0200-PASS1-03` (ALTTP New-File `$FF` Quirk):** Vanilla File Select writes `0xFF` to `$F405/$F406` -> sanitized `S_PIPE_DISC >= 0x20` and `S_DRAGON_CNT >= 0x21` in `Mario_InitOrVerifySramV2`.
4. **`V0200-PASS1-04` (Bidirectional Takeoff Latch):** Pressing `Down` mid-air for Ground Pound overwrote `$2F` before landing -> latched jump-takeoff direction `$2F` into `W_PIPE_STEP` and widened Hub 1 pipe X-zone to `$01E8..$0257`, delivering a flawless 4-hop bidirectional tour on cycle-accurate `bsnes`.
