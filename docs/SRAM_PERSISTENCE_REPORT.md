# Versioned 32-Byte Checksummed SRAM Schema v2 Persistence Report (`v0.20.0`)

**Release:** `v0.20.0 — Project Nintendo Limit Break`  
**ROM SHA-256:** `5e9c06a5db1ffc6be4879e258fa914bfdd56db101455082c7d2fb711df9cb910`  
**Formal Schema Specification:** [`docs/SAVE_SCHEMA.json`](SAVE_SCHEMA.json)  
**Status:** `PLAYTHROUGH_PASS` (3-Slot Concurrent Stress Test + Lossless `v0.10.0` Migration + Corrupted CRC Recovery + Dual-Core `faust`/`bsnes` Parity)

---

## 1. Architecture: Dual-Layer Checksummed 32-Byte Schema v2 (`$7E:F400..$7E:F41F`)

`v0.20.0` expands `v0.10.0`'s 8-byte unversioned block into a **32-byte versioned, independently checksummed block** (`$7E:F400..$7E:F41F`) inside ALTTP's native 1,280-byte per-slot save mirror (`$70:0000/$0500/$0A00` + hardware mirrors at `+$1000`):
- **Outer Layer (Hardware Native):** ALTTP's native 16-bit additive complement checksum at `+$04FE` summing 639 u16 words to `0x5A5A` (`sig = 0x55AA`).
- **Inner Layer (Mario Schema v2):** Fixed magic `"MP"` (`$F408..$F409`), version byte `0x02` (`$F40A`), 30-byte payload (`Dragon Coins`, `S_CHAR_ID 0..3`, `S_PIPE_DISC`, backward-compatible `S_LUIGI_MODE`, `S_WORLD_RESTORE`, `S_QUEST_LO/HI`, `S_STOMP_TOTAL`, `S_PIPE_TOTAL`, `S_BOWSER_FLG`, 13 reserved expansion bytes), and seeded Fletcher/XOR 16-bit CRC at `$F41E..$F41F`.
- **Vanilla New-File Quirk Handled (`V0200-PASS1-03`):** Vanilla ALTTP File Select initializes `+$0405/+$0406` to `0xFF` on brand-new files; `Mario_InitOrVerifySramV2` sanitizes `S_PIPE_DISC >= 0x20` and `S_DRAGON_CNT >= 0x21` while preserving genuine `v0.10.0` saves.

---

## 2. Live On-Disk Hardware Verification Evidence (`v0.20.0`)

1. **3-Slot Concurrent Migration, Corruption Recovery & Isolation (`v0200_10_sram_v2_3slot_migration_and_recovery`):**
   - **Slot 1 (Legacy `v0.10.0` Luigi save):** Auto-migrated losslessly on first gameplay frame -> `sram_v2_magic = "MP"`, `version = 2`, `char_id = 1`, `luigi_mirror = 1`, `dragon_cnt = 3`, `pipe_disc = 0x03`, `world_restore = 0x7F`, `sram_v2_crc_valid = True`, `alttp_checksum_valid = True`.
   - **Slot 2 (Intentionally Corrupted `v2` save: bad CRC `0xBEEF`, `char_id = 0xFF`, `dragon_cnt = 99`):** Detected CRC mismatch, sanitized out-of-bounds fields (`char_id = 3`, `dragon_cnt = 0`), re-sealed valid CRC16 (`sram_v2_crc_valid = True`), exited into Overworld with zero crash, and saved cleanly.
   - **Slot 3 (Valid `v2` Princess Peach save):** Verified 100% isolated and untouched (`char_id = 2`, `sram_v2_crc_valid = True`, `alttp_checksum_valid = True`) across both Slot 1 and Slot 2 reboot cycles.
2. **4-Hero Selection Persistence (`v0200_04`):** Cycling Mario -> Luigi -> Peach -> Captain Toad and saving flushed `char_id = 3`, `quest_lo = 0x20` (Hero Swap milestone), `sram_v2_crc_valid = True`.
3. **Dual-Core `snes_faust` vs Cycle-Accurate `bsnes` Parity (`v0200_02` & `v0200_03`):** Both cores flushed valid ALTTP + Mario v2 checksums (`coins = 280`, `dragon_cnt = 3`, `pipe_disc = 0x07`, `magic = "MP"`, `version = 2`, `sram_v2_crc_valid = True`, `bsnes pipe_total = 4`).
4. **100% Blank-SRAM Cold Boot -> Rainy Overworld -> Save -> Cold Reboot (`v0200_12`):** Verified valid ALTTP hardware slot creation (`alttp_sig = 0x55AA`, `alttp_checksum_valid = True`, `hp_max = 24`) and clean cold-reboot reload (noting honestly that vanilla ALTTP purposefully resets rainstorm prologue progress prior to rescuing Peach `$7E:F3C5 < 2`).
