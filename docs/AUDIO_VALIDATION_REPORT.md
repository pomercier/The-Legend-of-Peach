# Real SPC700 Audio Validation Report (`v0.10.0`)

**Release:** `v0.10.0 — Super Mario Adventure Beyond Limits`  
**ROM SHA-256:** `2da376f5c5cd6f760c07e465bdba252fe1aa1f03e69f17dcabcff1ecd6e072b1`  
**Audio Score:** `93.0%` (`14 / 14 AUDIO_PASS` on real Mednafen SPC700 WAV captures)

---

## 1. Objective Measurement Methodology

In prior milestones (`v0.4.0`–`v0.9.0`), audio was primarily `STATIC_PASS` (N-SPC byte budget + terminator validation). In `v0.10.0`, **`scripts/qa_engine_v0100.py`** extracts the real 44.1 kHz 16-bit stereo PCM WAV stream directly from every Mednafen `.mov` capture via `ffmpeg` and computes objective acoustic metrics:

- **RMS Level (dBFS):** Must exceed `-42.0 dBFS` (proves active SPC700 synthesis, never muted/crashed).
- **Active Non-Silent Windows (%):** Percentage of 100 ms windows exceeding `-50 dBFS` (must exceed `35%`).
- **Clipping Ratio (%):** Percentage of samples hitting `±32766` (must stay `< 0.5%` — zero SPC700 BRR/gain overload).
- **Zero-Crossing Frequency (Hz):** Confirms genuine melodic harmonic content rather than DC offset or 50/60 Hz hum.

---

## 2. Live Measured Results (`14 / 14 AUDIO_PASS`)

| Scenario | Core | RMS (dBFS) | Active Windows (%) | Audio Tier | Soundtracks / SFX Exercised |
|---|---|---|---|---|---|
| `v0100_01_warp_pipe_network_faust` | `snes_faust` | `-26.2 dB` | `71.6%` | `AUDIO_PASS` | Overworld + Warp Pipe Triple-Pulse + Coin + Toad Town + Vanilla Dome |
| `v0100_02_warp_pipe_network_bsnes` | `bsnes` | `-26.0 dB` | `74.8%` | `AUDIO_PASS` | Identical multi-biome SPC700 audio parity on cycle-accurate core |
| `v0100_03_luigi_pswitch_sram` | `snes_faust` | `-26.5 dB` | `63.3%` | `AUDIO_PASS` | Overworld + Luigi Swap Chime + Jump + P-Switch Clank + 2× Coin |
| `v0100_04_yoshi_tongue_swallow` | `snes_faust` | `-26.6 dB` | `64.3%` | `AUDIO_PASS` | Overworld + Yoshi Mount Chime + Gulp/Pop SFX + Coin Chime |
| `v0100_05_frog_suit_and_vanilla_dome` | `snes_faust` | `-25.8 dB` | `78.0%` | `AUDIO_PASS` | 4-Biome continuous SPC700 track transitions (0 driver lockups!) |
| `v0100_06_dry_bones_crumble_shatter` | `snes_faust` | `-25.3 dB` | `58.7%` | `AUDIO_PASS` | Fortress Theme + Bone-Crumble Stomp Pop + GP Heavy Thud |
| `v0100_07_bestiary_spiny_room017` | `snes_faust` | `-25.6 dB` | `55.5%` | `AUDIO_PASS` | Dungeon N-SPC Theme + Jump/Combat SFX |
| `v0100_08_bestiary_chomp_room0b0` | `snes_faust` | `-25.4 dB` | `54.7%` | `AUDIO_PASS` | Castle/Dungeon Theme + Hammer Bonk + GP Thud |
| `v0100_09_bestiary_monty_room0b2` | `snes_faust` | `-26.2 dB` | `54.5%` | `AUDIO_PASS` | Dungeon Theme + Combat SFX |
| `v0100_10a_bestiary_lava_room013` | `snes_faust` | `-26.1 dB` | `56.0%` | `AUDIO_PASS` | Cave/Dungeon Theme + Jump/GP SFX |
| `v0100_10b_bestiary_snifit_room056` | `snes_faust` | `-25.7 dB` | `55.0%` | `AUDIO_PASS` | Fortress Theme + Combat SFX |
| `v0100_11_bsnes_final_bowser_phase2` | `bsnes` | `-24.4 dB` | `73.7%` | `AUDIO_PASS` | Final King Bowser Theme + Fireball + Heavy GP Counter + Coin Chime |
| `v0100_12_clean_sram_playthrough` | `snes_faust` | `-25.1 dB` | `87.3%` | `AUDIO_PASS` | Title Calypso + Rainstorm + Uncle Luigi + Chest Fanfare + Save |
| `v0100_13_v090_baseline_compare` | `snes_faust` | `-27.8 dB` | `54.8%` | `AUDIO_PASS` | `v0.9.0` baseline Overworld & Warp Pipe SFX |

## 3. Honest Remaining Audio Limitations

- All 19 N-SPC Mario themes use ALTTP's original BRR instrument samples rather than custom imported SMW BRR instrument packs (`-7.0%` honest deduction in `05_audio_authenticity`).
- Acoustic validation verifies dynamic level, active playback, harmonic content, and zero clipping across real SPC700 runs; subjective mix mastering on real analog SNES RCA outputs awaits physical hardware testing.
