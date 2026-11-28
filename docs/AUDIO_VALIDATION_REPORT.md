# Real SPC700 Audio Validation Report (`v0.20.0`)

**Release:** `v0.20.0 — Project Nintendo Limit Break`  
**ROM SHA-256:** `5e9c06a5db1ffc6be4879e258fa914bfdd56db101455082c7d2fb711df9cb910`  
**Audio Score:** `94.0%` (`15 / 15 AUDIO_PASS` on real Mednafen SPC700 WAV captures)

---

## 1. `v0.20.0` Native SPC700 SFX Additions (`crates/snes-core/src/mario_sfx.rs`)

Building on 19 native SPC700 N-SPC Mario musical tracks (25 contexts) and 5 existing Mario SFX, `v0.20.0` adds 2 bespoke Bank `$19` native SPC700 sound effects (all byte-budget & terminator verified by `test_mario_sfx_budgets_and_terminators`):
- **`SFX2 0x14` — Ascending SMW Warp Pipe Emerge:** Complementary ascending triple-pulse sweep triggered on State 2 pipe pop-out emergence (`$012E = 0x14`).
- **`SFX2 0x2C` — Hero Swap / Big Coin Chime:** Crisp bright harmonic fanfare triggered when cycling Playable Heroes (`Down + L + R` across Mario, Luigi, Princess Peach, Captain Toad).

---

## 2. Live Measured Acoustic Results (`15 / 15 AUDIO_PASS`, 0% Severe Clipping)

| Scenario | Core | RMS (dBFS) | Active Windows (%) | Audio Tier | Soundtracks / SFX Exercised |
|---|---|---:|---:|---|---|
| `v0200_01_bug1_hedge_guard_and_star_monument` | `snes_faust` | `-26.5 dB` | `68.3%` | `AUDIO_PASS` | Toad Town + Jump & Spin Jump SFX |
| `v0200_02_smooth_bidirectional_pipes_faust` | `snes_faust` | `-25.4 dB` | `75.5%` | `AUDIO_PASS` | Multi-Biome Transitions + Pipe Descend (`0x15`) + Pipe Emerge (`0x14`) |
| `v0200_03_bidirectional_pipes_bsnes_parity` | **`bsnes`** | `-25.6 dB` | `78.4%` | `AUDIO_PASS` | Cycle-Accurate 4-Hop Multi-Biome SPC700 Transitions (0 lockups!) |
| `v0200_04_four_heroes_peach_hover_toad` | `snes_faust` | `-26.3 dB` | `67.5%` | `AUDIO_PASS` | Overworld + Hero Swap Chime (`0x2C`) + Jump + Turn-Block Coin |
| `v0200_05a_spin_jump_spiny_room017` | `snes_faust` | `-25.6 dB` | `58.0%` | `AUDIO_PASS` | Dungeon Theme + Spin Jump & GP Combat SFX |
| `v0200_05b_spin_jump_drybones_room026` | `snes_faust` | `-25.8 dB` | `58.0%` | `AUDIO_PASS` | Fortress Theme + Dry Bones Shatter Pop + Coin |
| `v0200_06_cape_360_spin_onoff_switch` | `snes_faust` | `-27.2 dB` | `49.5%` | `AUDIO_PASS` | Tower Theme + Cape 360° Whip + ON/OFF Switch Clank (`0x25`) |
| `v0200_07_koopa_shell_and_bobomb_room031` | `snes_faust` | `-25.3 dB` | `57.5%` | `AUDIO_PASS` | Dungeon Theme + Shell Kick & Combat SFX |
| `v0200_08_lake_cheepcheep_and_forest_hub4` | `snes_faust` | `-26.6 dB` | `73.0%` | `AUDIO_PASS` | Overworld -> Pipe Reverse -> River Hub 4 + Swim SFX |
| `v0200_09a_bestiary_swooper_mu061_room0d0` | `snes_faust` | `-25.7 dB` | `57.6%` | `AUDIO_PASS` | Cave Dungeon Theme + Combat SFX |
| `v0200_09b_bestiary_piranha_rex_dw06c` | `snes_faust` | `-28.2 dB` | `70.9%` | `AUDIO_PASS` | Valley of Bowser Dark World Theme + Jump/GP SFX |
| `v0200_10_sram_v2_3slot_migration_and_recovery` | `snes_faust` | `-27.8 dB` | `51.6%` | `AUDIO_PASS` | Title Calypso + File Select + House + Overworld Save |
| `v0200_11_bsnes_final_bowser_multiphase` | **`bsnes`** | `-24.4 dB` | `73.7%` | `AUDIO_PASS` | Final King Bowser Theme + Fireball + Heavy GP Multi-Phase Counter |
| `v0200_12_extended_blank_sram_playthrough` | `snes_faust` | `-24.4 dB` | `88.2%` | `AUDIO_PASS` | Title Calypso + Rainstorm + Uncle Luigi + Chest Fanfare + Save |
| `v0200_13_v0100_baseline_compare` | `snes_faust` | `-26.0 dB` | `72.3%` | `AUDIO_PASS` | `v0.10.0` baseline reference comparison |

## 3. Honest Remaining Audio Limitations

- All 19 N-SPC Mario themes still use ALTTP's original BRR instrument samples rather than custom imported SMW BRR instrument packs (`-6.0%` honest deduction in `05_audio_authenticity`).
- Physical analog RCA mix mastering awaits real SNES hardware validation.
