# Mushroom Kingdom World Transformation Report (`v0.20.0`)

**Release:** `v0.20.0 — Project Nintendo Limit Break`  
**ROM SHA-256:** `5e9c06a5db1ffc6be4879e258fa914bfdd56db101455082c7d2fb711df9cb910`  
**Master World Map Telemetry:** [`docs/WORLD_MASTER_MAP.json`](WORLD_MASTER_MAP.json)  
**Overworld Coverage:** **`128 / 128` Light & Dark World Screens (`100.0%`) + 3 Special Areas (`0x80, 0x81, 0x82`) = `131 / 160` total Overworld slots transformed**

---

## 1. Key `v0.20.0` World & Environmental Art Breakthroughs

1. **Global Bank `$09` Enemy-Spawn Exclusion Across ALL 128 Overworld Screens (`USER-BUG-02` Fixed Permanently):**
   - `mario_map.rs` now parses ALL 3 FR PAL Overworld sprite pointer tables (`0x04C8A6` Rain, `0x04C926` Pre-Agahnim, `0x04CA46` Post-Agahnim) with Large-Area quadrant coordinate mapping and a 1-tile protective halo around every spawn.
   - Automated Rust test `test_v0200_zero_enemy_spawn_overlap_all_128_overworld_screens` proves **`0` placed Warp Pipes or solid props overlap ANY enemy spawn across all 128 Light + Dark World screens**.
   - Relocated Vanilla Dome Hub 2 pipe off Deadrock/Spiny `0x27`'s spawn `(r10, c05)` to open plateau **`0x0B (r09, c02)`**, and Toad Town Hub 1 pipe out from behind the hedge to **Open Star Plaza `0x19 (r14, c01)`**.
2. **Vanilla Dome Clean Terracotta Boulder Palettes (Zero Olive Halos!):**
   - Patched `OW_BGColor0` Death Mountain (`0x0DE654 -> 0x36BB`), `OWMainPal[2]` Rows 3–4, and `OWAuxPal` Death Mountain (`0x0DE972/98E/99A/99C`) so dark brown SMW mountain boulders blend seamlessly into sunlit terracotta sandstone (`docs/media/screenshots/28_v0200_bugfixes_before_after.png` Row 2).
3. **Toad Town Golden Super Star Monument (`ZRI-022`: `4 -> 2`!):**
   - Replaced Kakariko's iconic red Weathervane Bird Spire (`raw 0x53` cols `14..15`, rows `0..2`, verified exclusively loaded by Toad Town areas `0x10..0x29`) with a gleaming 16-bit **Mushroom Kingdom Golden Super Star atop a Royal Red-Velvet Pedestal Cushion**, and cleared the 3 weathervane-exclusive Map16 rooster head/tail entries (`0x41A, 0x41D, 0x437 -> 0x0CED` grass) at `0x078000` (`docs/media/screenshots/28_v0200_bugfixes_before_after.png` Row 1).
4. **Expanded Special Areas (`0x80..0x82`) & Zero-Overlap Bank Allocation:**
   - Transformed Special Overworld Areas `0x80` (Star Road Celestial Grove), `0x81` (Star Sanctuary Clearing), and `0x82` (Bridge Overlook) alongside all 128 Light+Dark World screens (`0` grammar/collision/border/spawn breaks, `272` relocated blocks, `0` overlaps).
