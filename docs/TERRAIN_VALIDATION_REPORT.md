# Terrain Grammar, Hardware Collision & Border Validation Report — v0.7.0

**Target ROM:** `build/legend_of_peach_v0.7.0.sfc` (`SHA-256: f975131b5eaeee99ee7215ae636a380946ddc48832d3e7be95a11ab5d344dfee`)  
**Automated Enforcement:** `crates/snes-core/src/mario_map.rs` (`test_world1_screen2c_terrain_grammar_zero_seams`, `test_v070_all_128_overworld_screens_collision_grammar_and_borders`, `test_v060_all_seven_blocking_bugs_structural_invariants`)

---

## 1. Quantitative Terrain Validation Summary

| Audit Dimension | Audited Scope | Transformed Scope | Violations | Status |
|---|---:|---:|---:|---|
| **Overworld Map32 Lossless Decode (All Screens)** | 160 / 160 | 108 / 128 LW+DW (`322` Map32 tiles) | **0** | `STATIC_PASS` |
| **World 1 Screen `0x2C` BUG 03 Seam Guard** | 1,024 Map16 cells | 10 curated Map32 courtyard cells | **0** orphan cliffs, **0** orphan dirt triangles | `STATIC_PASS` + `VISUAL_PASS` (`bsnes`) |
| **Terrain Grammar Validator (All 128 LW+DW)** | 131,072 Map16 cells | 1,288 edited Map16 cells | **0** newly orphaned cliff-tops or dirt seams | `STATIC_PASS` |
| **Exact Hardware Collision (`$00:FBD9` `TileAttr`)** | 512 Map8 entries | Every edited Map16 subtile | **0** collision regressions (`0x48->0x48/0x50/0x52`, `0x00->0x00`, `0x08->0x08`) | `STATIC_PASS` |
| **Cross-Screen Edge Continuity (224 Cardinal Borders)** | 112 LW + 112 DW edges | Perimeter cells (`r∈{0,15}, c∈{0,15}`) locked on general screens | **0** cross-screen border breaks | `STATIC_PASS` |
| **Doorway Walkability Guard (`0x2C` Mario's House)** | `r16..19, c10..13` | 100% pure walkable grass (`0x034`) | **0** obstructions | `STATIC_PASS` + `VISUAL_PASS` |

---

## 2. Verified Collision-Identical Biome Tile Palettes (`$00:FBD9`)

All systemic Overworld transformations in `v0.7.0` strictly preserve hardware `TileAttr` (`PC 0x007BD9`, verified via ROM disassembly at `0x000840` / `0x0DBEF5`):
- **Grass (`0x001D`, `TileAttr = 0x48`):** Enriched with pure-walkable meadow variants (`0x0166`, `0x066A`, `0x009F`, `0x00AA`, `0x001C`, all `0x48`) and strictly 3×3-open-interior liftable SMW Turn-Blocks (`0x06C7`, `0x52`) / Smiling Bushes (`0x07CC`, `0x50`).
- **Forest / Dark Courtyard Floor (`0x0106`, `TileAttr = 0x00`):** Enriched with `0x06A6`, `0x0B53`, `0x0CF5`, `0x010C`, `0x0B5E`, `0x0E48`, `0x06A7` (all strictly `0x00`).
- **Dry Dry Desert / Bowser Swamp Sand (`0x05E5`, `TileAttr = 0x08`):** Enriched with `0x073B`, `0x0724`, `0x08B2`, `0x093E`, `0x0790`, `0x0C54` (all strictly `0x08`).
- **Cheep-Cheep Aquatic Sea (`0x0E90`, `TileAttr = 0x00`):** Enriched with `0x0FFC`, `0x0EF7`, `0x0EFE` (all strictly `0x00`).
