# World Connectivity & Complete Zone Discovery Report (`v0.7.0`)

Extracted directly from the verified FR PAL ROM binary (`roms/zelda_alttp.sfc`) with zero manual guesswork.

## 1. Global Discovered World Inventory (`100%` Inventoried)

| Domain | Discovered | Binary Source Offset (FR PAL) | Status |
| :--- | ---: | :--- | :--- |
| **Light World Overworld Screens (`0x00..0x3F`)** | `64 / 64` | Map32 HI `0x0179CF`, LO `0x017BAF`, AreaSize `0x012869` | `DISCOVERED` / `AUDITED` |
| **Dark World Overworld Screens (`0x40..0x7F`)** | `64 / 64` | Map32 HI `0x0179CF`, LO `0x017BAF` | `DISCOVERED` / `AUDITED` |
| **Special Overworld Areas (`0x80..0x9F`)** | `32 / 32` | Map32 HI `0x0179CF`, LO `0x017BAF` (`0x80` Star Glade, `0x81` Waterfall, etc.) | `DISCOVERED` / `AUDITED` |
| **Underworld / Dungeon / Interior Rooms (`0x000..0x127`)** | `296 / 296` | RoomHeaders `0x0274A2` (`$04:F4A2`), RoomSprites `0x04D653` (`$09:D653`) | `DISCOVERED` / `AUDITED` |
| **Overworld → Underworld Entrance Links** | `133` (`106` distinct target rooms) | `Entrance_Room` table `0x014895` (`133 × u16`) | `DISCOVERED` / `AUDITED` |
| **Overworld Hardware Collision Table (`TileAttr`)** | `512 / 512` Map8 entries | `PC 0x007BD9` (`$00:FBD9`) + Map16→Map8 `PC 0x078000` | `AUDITED` |
| **Boss Arenas Discovered** | `12 / 12` | Extracted from `RoomSpritesPtrs` (`0x04D653`) | `AUDITED` |

## 2. Overworld 8×8 Grid Connectivity & Biome Partition

- **Light World (Mushroom Kingdom, `0x00..0x3F`):** `8 × 8` contiguous grid (`112` internal cardinal screen boundaries: `56` East↔West + `56` South↔North). Large `1024×1024` 2×2 hubs verified from `0x012869`: `0x00` (Mushroom Forest), `0x03` & `0x05` (Vanilla Dome Mountains), `0x18` (Toad Town), `0x1B` (Peach's Castle), `0x1E` (Donut Plains / Chargin' Chuck Stadium), `0x30` (Dry Dry Desert), `0x35` (Cheep-Cheep Lake).
- **Dark World (Bowser's Conquered Kingdom, `0x40..0x7F`):** `8 × 8` contiguous grid (`112` internal cardinal screen boundaries). Distinct volcanic/obsidian biomes: `0x40` (Boo Haunted Woods), `0x43`/`0x45` (Bowser Magma Peaks), `0x58` (Koopa Shadow Village), `0x5B` (Bowser Volcanic Pyramid), `0x5E` (Morton Bastion), `0x70` (Larry Poison Swamp), `0x75` (Lemmy Ice Glacier).
- **Boundary Preservation Invariant:** All `224` cardinal screen boundaries (`112` Light World + `112` Dark World) are checked programmatically by `validate_full_world_integrity()` in `crates/snes-core/src/mario_map.rs` to ensure `0` border seams and `0` broken screen transitions.

## 3. All 12 Verified Boss Rooms & Key Hubs

| # | Boss (Mario Identity) | Room ID | Sprite ID | Header Pal / Blk / Spr | Dungeon World |
| ---: | :--- | :---: | :---: | :---: | :--- |
| 1 | 6 Giant Chargin' Chucks | `0x0C8` | `0x53` | `0B / 05 / 09` | World 2 — Donut Stadium (Eastern Palace) |
| 2 | 3 Giant Pokeys | `0x033` | `0x54` | `04 / 05 / 0B` | World 3 — Dry Dry Ruins (Desert Palace) |
| 3 | Giant Wiggler | `0x007` | `0x09` | `06 / 05 / 0C` | World 4 — Vanilla Summit (Tower of Hera) |
| 4 | Castle Tower King Bowser | `0x020` | `0x7A` | `0C / 02 / 12` | Peach's Castle Tower (Agahnim) |
| 5 | Koopaling #1 — Morton Koopa Jr. | `0x05A` | `0x92` | `10 / 07 / 15` | World 7A — Morton's Fortress (Palace of Darkness) |
| 6 | Koopaling #2 — Wendy O. Koopa | `0x006` | `0x8C` | `08 / 08 / 14` | World 7B — Wendy's Water Fort (Swamp Palace) |
| 7 | Koopaling #3 — Winged Iggy Koopa | `0x029` | `0x88` | `0E / 09 / 1A` | World 5 — Iggy's Boo Woods (Skull Woods) |
| 8 | Koopaling #4 — Roy Koopa | `0x0AC` | `0xCE` | `17 / 0A / 20` | World 7C — Roy's Hideout (Thieves' Town) |
| 9 | Koopaling #5 — Lemmy Koopa | `0x0DE` | `0xA2` | `14 / 0B / 16` | World 6 — Lemmy's Ice Circus (Ice Palace) |
| 10 | Koopaling #6 — Larry Koopa | `0x090` | `0xBD` | `12 / 0C / 16` | World 7D — Larry's Poison Mire (Misery Mire) |
| 11 | Koopaling #7 — Ludwig von Koopa | `0x0A4` | `0xCB` | `19 / 0D / 17` | World 7E — Ludwig's Lava Rock (Turtle Rock) |
| 12 | Final Boss — Giant King Bowser | `0x000` | `0xD6` | `21 / 13 / 22` | World 8 — Bowser's Volcanic Throne (Pyramid) |

## 4. Key Overworld Entrance Links (`Entrance_Room` @ `0x014895`)

- `0x00 / 0x01` → Room `0x104` (Mario's House on Screen `0x2C`)
- `0x02` → Room `0x012` (Peach & Toad Sanctuary)
- `0x03 / 0x04 / 0x05` → Rooms `0x060 / 0x061 / 0x062` (Peach's Castle West/Main/East)
- `0x12` → Room `0x103` (Toad Town House — 16-bit Toad)
- `0x1F` → Room `0x11F` (SMW Royal Blue Kamek's Shop)
- Total distinct dungeon/interior rooms reachable via `0x014895`: `106` entry rooms branching across all `296` Underworld rooms (`40` distinct room palettes).
