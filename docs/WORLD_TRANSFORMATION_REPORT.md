# World Transformation Report — v0.7.0 (`Mushroom Kingdom: World Transformation`)

**Target ROM:** `build/legend_of_peach_v0.7.0.sfc` (local deterministic build, never committed)  
**Verified SHA-256:** `f975131b5eaeee99ee7215ae636a380946ddc48832d3e7be95a11ab5d344dfee`  
**SNES Checksum / Complement:** `0x1285 / 0xED7A` (`Valid: true`, 2 MiB LoROM)  
**Hardware Operations:** `661` patches across `18` canonical fusion rules (`225` relocated blocks in `$23..$3F`, `0` overlaps)

---

## 1. Full 160-Screen Overworld & 296-Room Underworld Discovery

All world structures were extracted directly from the unmodified FR PAL base ROM (`PC 0x0179CF` Map32 HI pointers, `PC 0x017BAF` Map32 LO pointers, `PC 0x007BD9` 512-byte hardware `TileAttr` collision table, `PC 0x0274A2` Underworld room headers, `PC 0x04D653` Underworld sprite lists, `PC 0x014895` 133 entrance links):

| Domain | Discovered | Audited | Transformed (Map32 / Palette / Tileset) | Validation Status |
|---|---:|---:|---:|---|
| **Light World Overworld (`0x00..0x3F`)** | 64 | 64 (`100%`) | **56 / 64** Map32 screens (`87.5%`) + global SMW tilesets | `STATIC_PASS` + `VISUAL_PASS` |
| **Dark World Overworld (`0x40..0x7F`)** | 64 | 64 (`100%`) | **52 / 64** Map32 screens (`81.3%`) + **Bowser Volcanic & Obsidian Citadel Palettes** (`100%` DW palette overhaul) | `STATIC_PASS` + `VISUAL_PASS` |
| **Special Overworld (`0x80..0x9F`)** | 32 | 32 (`100%`) | Lossless decode & relocation verified | `STATIC_PASS` |
| **Underworld Rooms (`0x000..0x127`)** | 296 | 296 (`100%`) | 12 Boss Arenas, Toad Houses, Kamek Shop, ON/OFF Dungeon Switches (`bg:0F`), Bowser Volcanic Throne (`0x000`) | `STATIC_PASS` + `VISUAL_PASS` (all 12 bosses + shops + NPCs) |

---

## 2. Biome Taxonomy & Map32 Transformation Summary (`108 / 128` Screens, `322` Map32 Tiles)

Every edited Map32 cell across all `108` relocated Light World and Dark World screens was verified by `crates/snes-core/src/mario_map.rs` against three mandatory invariants:
1. **Exact Hardware Collision Preservation (`$00:FBD9` `TileAttr`):** Every modified Map16 subtile either retains its exact original hardware collision attribute (`0x48 -> 0x48` grass, `0x00 -> 0x00` forest/courtyard/sea floor, `0x08 -> 0x08` desert/swamp sand) or upgrades strictly interior 3×3 pure grass (`0x48`) into a liftable SMW Turn-Block (`0x52`) or Smiling Bush (`0x50`) — **0 blocked doorways, 0 cliff regressions**.
2. **Terrain Grammar Validator:** **0** orphan cliff-top strips (`0x16C` without `0x520`), **0** newly created orphan brown dirt triangles (`0x0AC/0x0AD/0x09F`).
3. **Cross-Screen Edge Continuity:** All `224` cardinal boundaries (`112` LW + `112` DW) audited with **0** broken screen borders.

| Region / Biome | Screens | Artistic & Geographic Identity |
|---|---:|---|
| **Yoshi's Island Hub (`0x2B..0x2D, 0x34, 0x3C`)** | 5 | Seamless v0.6.0 Screen `0x2C` Turn-Block Plaza, Smiling Bushes, animated daisies, zero dirt seams |
| **Toad Town (`0x18..0x19, 0x20..0x22, 0x28..0x2A`)** | 8 | Mushroom-roof village palette, 16-bit Toads (`0x0103`), Royal Blue Kamek Shop (`0x011F`), meadow accents |
| **Mushroom Forest (`0x00..0x02, 0x08..0x0A, 0x10..0x12`)** | 9 | Collision-identical forest floor & tufted meadow variety (`0x0106 -> 0x06A6..0x06A7`, `0x001D -> 0x066A..0x0166`) |
| **Vanilla Dome Mountain (`0x03..0x07, 0x0B..0x0F`)** | 10 | Mountain rock & ridge continuity preserved; walkable grass/courtyard accents enriched |
| **Peach Castle Domain (`0x13..0x15, 0x1A..0x1D, 0x23..0x25`)** | 10 | SMW Castle battlement tileset (`bg:4C`), **Koopa Hammer Bro Guards** (zero medieval Zelda swords in `raw 0xBA`!) |
| **Chargin' Chuck Plains (`0x16..0x17, 0x1E..0x1F, 0x26..0x27, 0x2E..0x2F`)** | 8 | Open clearing Turn-Blocks & Smiling Bushes, Witch's Hut Kamek cauldron, Eastern Bowser Statues |
| **Dry Dry Desert (`0x30..0x33, 0x38..0x3A`)** | 7 | Collision-identical desert sand variants (`0x05E5 -> 0x073B..0x0C54`, strictly `TileAttr 0x08`) |
| **Cheep-Cheep Aquatic (`0x35..0x37, 0x3B, 0x3D..0x3F`)** | 7 | Lake Hylia deep-water wave variants (`0x0E90 -> 0x0FFC/0x0EF7/0x0EFE`), live-verified Cheep-Cheep & Mario Swim/Dive |
| **Dark World — Bowser's Conquered Volcanic Kingdom (`0x40..0x7F`)** | 64 | **Complete visual separation from Light World:** `OWMainPal[1]` (`0x0DE70E` obsidian basalt, dark-plum turf, magma-amber ledges), `OWMainPal[3]` (`0x0DE79A` charred lava peaks), AND **`OWAuxPal[17]` (`0x0DEB36` Bowser's Obsidian Lava Citadel on Screen `0x5B/0x5C/0x63` with Carved Stone Bowser Statues & magma sunset sky)**! |

*Note on the 20 non-Map32-edited screens (`0x1C, 0x1E, 0x1F, 0x21, 0x27, 0x2B, 0x34, 0x3F, 0x40, 0x41, 0x48, 0x49, 0x5B, 0x5C, 0x5E, 0x5F, 0x60, 0x61, 0x62, 0x6B`):* These screens consist exclusively of cliffs, castle walls, pyramid architecture, or mountain ledges without uniform interior ground tiles; all 20 were audited at `100%` and visually receive our global SMW tileset (`bg:29` Bowser Statues, `bg:4C` castle walls, `bg:3F` Smiling Bushes/Turn-Blocks) and Bowser Volcanic / Obsidian Citadel palettes (`0x5B`, `0x5C`, `0x6B`).

---

## 3. Koopa Guard Weapon Conversion (`ZRI-016` Eliminated)

All 8 Zelda medieval sword and spear tiles in compressed sprite sheet `raw 0xBA` (`$6B/$7B` vertical, `$6C/$7C` horizontal, `$6E/$7E/$6F/$7F` spear guards) were replaced with **SMW Hammer Bro Hammers** and **SMW Spiked Koopa Clubs**, verified both structurally in `test_v060_all_seven_blocking_bugs_structural_invariants` and visually in live `Mednafen` Overworld captures (`21_v070_world_transformation_gallery.png`).
