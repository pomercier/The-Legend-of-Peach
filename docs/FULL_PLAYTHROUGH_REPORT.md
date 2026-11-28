# Playthrough Validation Report (`v0.20.0`)

**Release:** `v0.20.0 — Project Nintendo Limit Break`  
**ROM SHA-256:** `5e9c06a5db1ffc6be4879e258fa914bfdd56db101455082c7d2fb711df9cb910`  
**Deterministic Replay Manifest:** [`docs/PLAYTHROUGH_REPLAY_MANIFEST.json`](PLAYTHROUGH_REPLAY_MANIFEST.json)  
**Playthrough Completeness Score (Honest):** `62.0%` (`PARTIAL_PLAYTHROUGH_PASS`)

---

## 1. Extended 100% Blank-SRAM Continuous Cold-Boot Playthrough (`PLAYTHROUGH_PASS`)

Scenario `v0200_12_extended_blank_sram_playthrough` booted the **unmodified `v0.20.0` ROM** with a **100% zeroed 8 KiB `.srm` file (`0x00 * 8192`)** — zero pre-built saves, zero room warps, zero memory pokes — and logged every timed controller input to [`docs/PLAYTHROUGH_REPLAY_MANIFEST.json`](PLAYTHROUGH_REPLAY_MANIFEST.json):

1. **Cold Boot & Title Screen:** Royal Princess Peach Golden Star-Scepter & Super Star Crown (`raw 0xA5`, zero Master Sword) + SMW Title Calypso (`AUDIO_PASS -24.4 dBFS`, `88.2%` active energy).
2. **Fresh Save Slot Creation:** Created Save Slot 1 (`1. AB`) on Mario File Select with Golden Super Star cursor.
3. **Opening Narrative (`MU-071`, 0 Zelda terms):** Played the full French legend scroll, Princess Peach's telepathy (`"Au secours ! Au secours ! C'est moi, la Princesse Peach !"`), and Uncle Luigi's bedside Mario dialogue.
4. **In-House Exploration & Star-Torch Chest:** Mario navigated past the table, opened the treasure chest, and equipped the **Star-Torch** inside the SMW Gold-Riveted Reserve Item Box.
5. **Rainy Mushroom Kingdom Overworld Exploration:** Exited Mario's House (`0x104`) onto World 1 Yoshi's Island (`0x2C`) under rainstorm effects, tested live **Mario Jump** in the rain (`f_016`), and explored the East courtyard toward Peach's Castle road.
6. **Hardware Save Menu & Cold Emulator Reboot:** Opened `Select -> Sauvegarder et quitter.` cleanly on the Rainy Overworld (`f_023..f_024`), terminated Mednafen cleanly, verified the written 8 KiB `.srm` hardware checksum (`alttp_sig = 0x55AA`, `alttp_checksum_valid = True`, `hp_max = 24`), **cold-rebooted Mednafen from scratch**, and reloaded Slot 1 cleanly (`r_006..r_009`, confirming authentic vanilla ALTTP rainstorm prologue reset behavior prior to Peach's Sanctuary rescue).

---

## 2. Individually Verified Mid-Game, Late-Game & Multi-Slot Persistence Content (`VISUAL_PASS`)

- **3-Slot Concurrent SRAM v2 Stress Test (`v0200_10`):** Verified lossless `v0.10.0` migration, corrupted CRC recovery, and Slot 3 isolation across cold reboots.
- **4-Hop Uncut Bidirectional Warp Pipe Tour (`v0200_03` on cycle-accurate `bsnes` + `v0200_02` + `v0200_08`):** Verified `0x2C -> 0x19 (Toad Town Star Monument) -> 0x0B (Clean Vanilla Dome) -> Reverse 0x19 -> Express Home 0x2C` plus `Reverse -> Hub 4 (0x12) + SMW Frog Suit`.
- **4 Playable Heroes (`v0200_04`), SMW Spin Jump (`v0200_05a/05b`), Cape 360° -> ON/OFF Switch (`v0200_06`), Buzzy Shell Room (`v0200_07`), Swooper (`v0200_09a`) & SMW Rex (`v0200_09b`), and Final King Bowser Multi-Phase on `bsnes` (`v0200_11`).**

---

## 3. Honest Remaining Playthrough Gap (Why `v0.20.0` Is Not Yet `v1.0.0`)

A **continuous, uninterrupted, 100% blank-SRAM start-to-credits playthrough across all 11 dungeons** (without harness room-warps between dungeons) has **not yet been completed**. Alongside physical SNES + FXPAK Pro hardware execution (`0/1 HARDWARE_NOT_TESTED`), this remains one of the two explicit release gates reserved for **`v1.0.0`**.
