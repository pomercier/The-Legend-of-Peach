# Playthrough Validation Report (`v0.10.0`)

**Release:** `v0.10.0 — Super Mario Adventure Beyond Limits`  
**ROM SHA-256:** `2da376f5c5cd6f760c07e465bdba252fe1aa1f03e69f17dcabcff1ecd6e072b1`  
**Playthrough Completeness Score (Honest):** `58.0%` (`PARTIAL_PLAYTHROUGH_PASS`)

---

## 1. What WAS Executed End-to-End From 100% Blank SRAM (`PLAYTHROUGH_PASS`)

Scenario `v0100_12_clean_sram_playthrough` booted the **unmodified `v0.10.0` ROM** with a **100% zeroed 8 KiB `.srm` file (`0x00 * 8192`)** — zero pre-built save files, zero room warps, zero memory pokes:

1. **Cold Boot & Title Screen:** Royal Princess Peach Golden Star-Scepter & Super Star Crown (`raw 0xA5`, zero Master Sword) + SMW Title Calypso (`AUDIO_PASS`).
2. **Fresh Save Slot Creation:** Created Save Slot 1 on the redesigned Mario File Select screen (`1. AB`, Golden Super Star cursor).
3. **Opening Story & Narrative (`MU-071`):** Played the full French legend scroll, Princess Peach's telepathy, and Uncle Luigi's bedside Mario dialogue (`"Salut Mario ! Je file inspecter les Tuyaux du Royaume Champi. Bowser prépare encore un mauvais coup ! Reste ici, je reviens très vite !"`).
4. **In-House Exploration & Treasure Chest:** Mario hopped out of bed cleanly, navigated around the central table, opened the treasure chest, and received the **Star-Torch** (Fire Flower icon rendered inside the new SMW Gold-Riveted Reserve Item Box, clean French item text `"sur le bouton Y pour te frayer un chemin dans l'obscurité."`).
5. **Rainy Mushroom Kingdom Overworld:** Exited Mario's House (`0x104`) onto World 1 Yoshi's Island (`0x2C`) under rainstorm effects with the Glossy Green SMW Warp Pipe, SMW Turn-Blocks, and Smiling Bushes intact.
6. **Hardware Save & Cold Emulator Reboot:** Triggered `Select -> Sauvegarder et quitter.`, terminated Mednafen cleanly, verified the written 8 KiB `.srm` file on disk (`slot1_magic = 0x55AA`, `checksum_stored = 0x6CEE == checksum_calc = 0x6CEE`, `hp_max = 24` = 3 full hearts), **cold-rebooted Mednafen from scratch** on that `.srm`, and reloaded Slot 1 with zero corruption.

---

## 2. Individually Verified Late-Game & Boss Content (`VISUAL_PASS`)

- **All 12 / 12 Boss Arenas** (`v0.7.0`–`v0.10.0`) including Final King Bowser Phase 2 Counterplay on cycle-accurate `bsnes` (`250 -> 281` Coins, zero BUG 07 regression).
- **All 8 / 8 Bespoke Koopaling & King Bowser Fortresses** (`DungPal[1,4,5,8,9,10,12,13]`).
- **4-Biome Real Overworld Warp Pipe Chain** (`0x2C -> 0x18 -> 0x03 -> 0x35`) + SMW Frog Suit swim + Playable Luigi + Timed P-Switch + Yoshi Tongue Swallow + Dry Bones & 5 newly visual-verified bestiary rooms (`0x017, 0x0B0, 0x0B2, 0x013, 0x056`).

---

## 3. Honest Remaining Playthrough Gap (Why `v0.10.0` Is Not Yet `v1.0.0`)

A **continuous, uninterrupted, 100% blank-SRAM start-to-credits playthrough across all 11 dungeons** (without harness room-warps between dungeons) has **not yet been completed**. Completing a full start-to-credits human or scripted playthrough and validating on physical SNES + FXPAK Pro hardware are the two primary gates reserved for **`v1.0.0`**.
