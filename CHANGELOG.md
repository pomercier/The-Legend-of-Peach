# Changelog

All notable changes to **The Legend of Peach: A Mario to the Mushroom Kingdom** are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.6.0] — 2026-11-27 — Super Mario World Authenticity

### Complete Structural Resolution of All 7 Blocking Visual Defects
- **BUG 01 — Mario Enemy Contact & All 303 Player DMA Poses (`demo.rs`, `mixer.rs`)**:
  - Disassembled Bank `$00` Link DMA tables (`PC 0x001346..0x001802`) and Bank `$0D` pose OAM table (`PC 0x068000..0x06838D`), remapping all 28 odd-row/overlapping/`Body==R00C0` poses (including Hurt/Recoil `0x03B, 0x03F..0x042`) onto verified free even-row `16×16` cells in Bank `$10` (`R22C0..7`, `R07C1..4`, `R17C3..5`, `R23C0`).
  - Patched all 19+ previously unpatched Vanilla Link cells with dedicated 4bpp Mario art (`make_hurt_head/body`, `make_kneel_body`, `make_shock_xray`, `make_swim_ripple`, `make_dive_splash`, `make_ko_flat`) so **0/303** poses have odd-row overlaps, active Vanilla Link tiles, or duplicate `Head == Body` slots.
  - Replaced blanket Bank `$0D` OAM overwrites with a pose-aware table preserving single-tile, swim, and KO states. Verified live at `12 fps` during Blue Koopa collision (`0` double heads).
- **BUG 02 — Fire Mario 4-Direction Fireball Cast & Zelda Rod Removal (`demo.rs`, `mario_ui.rs`)**:
  - Added dedicated 4-direction `make_fire_throw_down/side/up` poses cleanly aligned in Bank `$10`, cleared erroneous vertical-flip inversion flags on `0x12B..0x12E`, and replaced the 6 residual vanilla Zelda Fire/Ice Rod staff tiles in `0x087000` with blazing Mario fireball cast flames and sparks. Verified live on both `snes_faust` and cycle-accurate `bsnes`.
- **BUG 03 — Bit-Exact Map32→Map16 Terrain Grammar Validator & Screen `0x2C` Cleanse (`mario_map.rs`)**:
  - Added ALTTP's packed 12-bit Map32 quadrant decoder (`0x018000`, `0x01B410`, `0x020000`, `0x023410`) and automated `validate_screen_terrain_grammar` enforcing continuous cliff tops and zero orphan dirt seams.
  - Replaced buggy Screen `0x2C` tiles `0x1B76` / `0x06A0` with pure-grass SMW Turn-Blocks (`0x06C7` at `r09 c4 & c7`), pure grass above continuous cliff top (`0x0208` at `r10 c5`), and pure daisy grass (`0x0166` at `r15 c12`). Guarded by unit test `test_world1_screen2c_terrain_grammar_zero_seams`.
- **BUG 04 — Authentic SMW Royal Blue Magikoopa Kamek (`mario_world.rs`, `mixer.rs`)**:
  - Rebuilt Kamek modeled on official SMW `GFX03` (pointed royal-blue wizard hat, round white spectacles, yellow Koopa snout, white-trimmed blue wizard robe, animated pink-tipped wand).
  - Relocated Robe F0/F1 to non-digit cells `(6,0)` (`$06`) & `(6,2)` (`$26`) in `raw 0xBE` leaving shop price digits `'0'..'9'` bit-identical to vanilla.
  - Patched Shopkeeper prep (`0x030BFA = 0x08`), state 0 (`0x0F6F54 = 0x09`), and OAM table (`$05:F8F5`, `flags = 0x00` respecting `$05:DF6F`'s `EOR $05` contract so `0x00 ^ 0x09 = 0x09` activates OBJ Pal 4 `0x0DD272` Royal Blue `#4194FF / #004AAC`).
- **BUG 05 — 16-Bit SMAS/SMRPG Toad Proportions & Clean Face/Shadow (`mario_world.rs`)**:
  - Rebuilt Toad eliminating all `PalRow[02]` index-2 lime-green pixels from the mouth/face, cleared `raw 0xBD (6,0)` (`$86`) so ALTTP's native ground shadow renders without the giant grey head lump, and lifted `Sprite 0x35` head `dy` at `0x02E544` (`-4 -> -9`) and Innkeeper `0x0D7866/7E` (`-9 -> -11`).
- **BUG 06 — Winged Iggy Koopa Anchored SMW Paratroopa Wings (`mario_bosses.rs`)**:
  - Preserved Iggy Koopa's body (`cols 16..31`) 100% untouched while sculpting SMW `GFX01` Paratroopa feathered wings rooted at `x = 15` and pulling Mothula's OAM wing offsets at `PC 0x0D7CED` (`$1A:FCED`) inward from `dx = ±24` to **`dx = ±17`** with coordinated upstroke/downstroke flap `dy`.
- **BUG 07 — Authentic SMW Final Boss King Bowser (`mario_bosses.rs`)**:
  - NOP'd `JSR Ganon_DrawHeldTrident` at `PC 0x0E9AF5` (`20 1C 9C -> EA EA EA`), eliminating 100% of Ganon's blue/beige orb and diagonal stick clutter while preserving thrown attacks.
  - Modeled Final King Bowser on SMW `GFX21` (wild crimson mane, sweeping ivory bull horns, fierce red brows, golden snout & fanged roar, hunched emerald spiked shell, spiked black leather collar tucked cleanly under the chin, wide horizontal-ribbed gold belly), lifted standing/dialogue Head `dy` in `PC 0x0E969A` (`-10 -> -15`), engineered exact disjoint `16×24` continuous arm strips across `sp:21` and `sp:41` (`$06/$16`, `$36/$46`, `$08/$18`, `$38/$48`, `$0A/$1A`), and extracted real SMW `GFX24` Giant Fireballs from `roms/super_mario_world.sfc`. Verified identically on both `snes_faust` and cycle-accurate `bsnes`.

### Technical Documentation & Regression Suite
- Added `docs/SPRITE_ANIMATION_AUDIT.md`, `docs/ART_DIRECTION.md`, `docs/METASPRITE_ARCHITECTURE.md`, `docs/TERRAIN_VALIDATION_REPORT.md`, `docs/CONTACT_COLLISION_REPORT.md`, `docs/POWERUP_ANIMATION_REPORT.md`, `docs/BOSS_VISUAL_REPORT.md`, `docs/HARDWARE_VALIDATION.md`, and `docs/TEST_COVERAGE.json`.
- Added automated invariant guard `tests::test_v060_all_seven_blocking_bugs_structural_invariants` and `mario_map::tests::test_world1_screen2c_terrain_grammar_zero_seams` (`31/31` passing Rust tests, `0` clippy warnings).

### Build & Verification (`v0.6.0`)
- Deterministic 2 MiB LoROM build: `230` hardware patches across `18` rules (`~1.0 s`), `49` LZ2 sheets verified, `11` relocated in `$23..$3A` + Map32 in `$3B..$3F` (0 overlaps).
- **SNES Checksum / Complement:** `0xC419` / `0x3BE6` (Valid: `true`)
- **Output CRC32:** `D5E69FF4`
- **Output MD5:** `947e35e7a81406693829eb88c5962e41`
- **Output SHA-1:** `5f78310b0a85046ec016f3822d5b3b5e21016632`
- **Output SHA-256:** `2004714a74f35fbf6c6c581f25b86d0b40d6729dd38c089f65e7b69e68a202ca`

---

## [0.5.0] — 2026-11-26 — Super Mario Revolution

### Native 65c816 Super Mario Gameplay Engine (`mario_engine.rs`, Bank `$22:9000`)
- **Genuine Top-Down Mario Jumping (`L` or `R`)**:
  - Hooks all 3 verified Bank `$02` callers of `Link_Main` (`PC 0x010959`, `0x011CCA`, `0x0125C7`) using collision-free WRAM `$7E:F500..$7E:F53F`.
  - Smooth 8.8 fixed-point parabolic jump physics with variable jump height (short-hop vs. full ~22px arc), crisp dark oval ground shadow beneath Mario, full 8-direction air control, airborne hazard clearance, and automatic mode/room transition reset.
- **Genuine Mario Stomp Combat (`$22:9800` Vulnerability Table)**:
  - Descending onto stompable Mario enemies (Koopas, Goombas, Buzzies, Rex, Monty Moles, Dry Bones, Cheep Cheeps) deals heavy stomp damage / instant defeat puff (`$0DD0 = 6`), awards **+1 Gold Coin**, triggers post-stomp i-frames, plays the native SPC700 Mario Stomp SFX, and bounces Mario upward (extra height when holding `L`/`R`).
  - Spiked/armored enemies (Spinies, Thwomps, Chain Chomps) are safely immune to ordinary stomps and can be defeated via Super Starman.
- **Unified Mario Power-Up State Machine (`$22:9900` Live CGRAM Palettes)**:
  - **Fire Mario**: Equipping `FLEUR DE FEU` (`$0202 == 0x06`) immediately live-swaps Mario's CGRAM OBJ palette into pure white cap/shirt & blazing crimson overalls while launching Fireballs.
  - **Super Starman (`L + R`)**: Consumes Star Power from `$7E:F36E` to trigger 4 seconds of flashing 4-palette rainbow CGRAM invincibility with instant contact-kill across both regular and spiked enemies.

### Native SPC700 Mario Sound Effects (`mario_sfx.rs`)
- In-place Bank `$19` (`PC 0x0D0B70..0x0D1BFF` -> `ARAM $17C0..$284F`) native SPC700 SFX compiler delivering authentic **Mario Coin chime (`B5 -> E6`)**, **SMW rising Jump sweep**, **Mario Stomp pop**, **descending triple-pulse Warp Pipe**, and **Super Mario Hammer bonk**.

### Overworld Map32 World Engine & World 1 Redesign (`mario_map.rs`)
- Zero-dependency Rust Big-Endian LC_LZ2 Overworld Map32 JSON editor (`OverworldScreenMap::to_json_pretty` / `from_json`) verified lossless across all 64 Light World screens.
- Redesigned **World 1 — Yoshi's Island (`Screen 0x2C`, Mario's House Hub)** relocated cleanly into LoROM Bank `$3B` (`0x1D8000+`) with an enforced compile-time memory firewall isolating GFX relocation (`$23..$3A`) from Map32 screens (`$3B..$3F`).

### Complete Resolution of All 5 Visual & Dialogue Bugs
- **File Select Overhaul (`mario_ui.rs`, `mixer.rs`)**: Fixed FR PAL player name encoding (`1. MARIO`), replaced the vanilla pink Fairy cursor (`raw 0xCF $A8/$AA`, `$0E:96B2 = 0x30`) with a 2-frame blazing **Golden SMW Super Star**, replaced Mario's blue Zelda sword with a centered upright **Super Mario Hammer**, and transformed the olive-green cobblestone backdrop (`0x0DD9AA`) into **SMW Royal Sky Blue**.
- **Super Mario Hammer & Start Menu Revolution (`mario_ui.rs`)**: Replaced all melee sword swings (`raw 0xCF`) and Start Menu `EQUIPEMENT` weapons (`0x06F769`) with Super Mario Hammers; replaced Fire/Ice Rods (`0xB0..0xC1`) with iconic **SMW Fire & Ice Flowers**; replaced Magic Mirror (`0x06F709`) with a glossy **SMW Green Warp Pipe** (eliminating the HUD tile collision); and Mario-ized all remaining Start Menu item names (`TUYAU WARP`, `BLOC POW`, `SUPER CHAMPI`, `FLUTE WARP`, `POUDRE KAMEK`).
- **Toad Town NPC OAM Alignment (`mario_world.rs`)**: Mapped every head and body slot of `raw 0xC2`, `0xBD`, and `0xC3` to verified vanilla ALTTP OAM layouts with upgraded pixel-exact Nintendo Toad sprites (fixing the distorted side sprite and red belly dot in Room `0x0103`), paired with **Royal Crimson Red & Gold Mario House Carpets (`0x0DD932`)** in Toad Town and Kamek's Shop.
- **Winged Iggy Koopa Rebuild (`mario_bosses.rs`)**: Rebuilt `raw 0xAB` (`sp:38`) aligned cleanly to Mothula's OAM structure.
- **French Dialogue Engine & Volcanic Bowser Throne (`mario_intro.rs`, `mixer.rs`)**: Fixed duplicate `0x75`/`0x76` line-break emissions (eliminating text overwrite in Bowser Room `0x0000`), eliminated 100% of hyphenated word splits across line breaks across all 372 messages (enforced by automated unit test), rewrote marquee story dialogues into a playful original Mario narrative, and converted Final Bowser Room `0x0000` (`0x0DE2C2`) into an **Obsidian Dark-Charcoal & Lava Volcanic Throne Room**.

### De-Zeldafication Engineering & Persistent Registries
- Added structured engineering registries: `docs/ROM_MEMORY.md`, `docs/WRAM_ALLOCATION.json`, `docs/ZELDA_RESIDUE_REGISTRY.json`, `docs/PATCH_DEPENDENCIES.json`, `docs/WORLD_MAP_REGISTRY.json`, `docs/MARIO_GAMEPLAY_REGISTRY.json`, `docs/FAILED_APPROACHES.md`, `docs/VISUAL_REGRESSION_REPORT.md`, `docs/NEXT_ACTIONS.md`, and `docs/maps/world1_yoshis_island.json`.
- Reduced the measured **Zelda Residue Index** from `94.9` (`v0.4.0`) to **`19.2`** (`v0.5.0`, **−79.8%**), raising the **Mario Identity Index** from `5.1` to **`80.8`**.

### Build & Verification (`v0.5.0`)
- Deterministic 2 MiB LoROM build: `220` hardware patches across `18` rules (`~1.0 s`), `49` LZ2 sheets verified, `11` relocated in `$23..$3A` + Map32 in `$3B..$3F` (0 overlaps).
- **SNES Checksum / Complement:** `0x87FF` / `0x7800` (Valid: `true`)
- **Output CRC32:** `9E8F84EB`
- **Output MD5:** `74b5e46d58efa5f0f15cfe2b30383401`
- **Output SHA-1:** `50db6eed1ab3495d877e0701c10974df4f252dd3`
- **Output SHA-256:** `93d3774f81119eb41d80b0c610d3b1bbcd1430c5f572f16b65b47b3ddd24975e`
- Full test suite: `29/29` passing Rust unit and integration tests; dual-core live verification in Mednafen `snes_faust` and cycle-accurate `bsnes` (`snes`).

---

## [0.4.0] — 2026-11-26 — The Mushroom Kingdom Reborn

### Visual Polish & Authentic Super Mario World Environments
- **Title Screen BG2 Landscape & Master Sword SMW Colors (`mario_intro.rs`)**:
  - Live-patched Title Screen BG2 palette with authentic Super Mario World values: SMW Sparkling Blue water (`0x7A18`), SMW Sky Blue atmosphere (`0x7F0E`), Yoshi's Island Green Mountains (`0x1EE7`), Peach Castle Pink (`0x5B5E`), pure white battlement highlights (`0x7FFF`), and Golden Star blade hilt (`0x275F`).
  - Preserved 100% fluid rotating 3D polygonal relics (Mushroom, Fire Flower, Super Star) with dual copyleft and copyright notices.
- **Bowser Final Pyramid Arena Floor Cleanse (`mario_bosses.rs`)**:
  - Eliminated the 4×4 residual floor square and grey stone scales in Room 0x00 by patching routine `$01:A7F0` (`0x00A7F0`) with a clean `RTS` bypass.
  - Room 0x00 now displays an unblemished, majestic emerald green stone dais where King Bowser stands in combat, with zero Triforce residue.
- **Dungeon ON/OFF Solid & Dotted Switch Blocks (`mario_world.rs`)**:
  - Converted dungeon crystal switches and solid/dotted barriers in `raw:0x0F` (`0x08FC21`):
    - Solid Red/Blue ON & OFF Switch Blocks with beveled metallic borders, corner rivets, and crisp "ON" / "OFF" text.
    - SMW Dotted Outline Blocks for inactive states with authentic dashed borders.
    - Mounted SMW ON/OFF Switch Block replacing the crystal switch orb.
- **Dark World Pyramid Carved Stone Bowser Statues (`mario_world.rs`)**:
  - Converted flanking gargoyle statues on the Dark World Pyramid exterior (`raw:0x29`, `0x0976C0`) into 32×32 Carved Stone King Bowser Statues and 16×32 Grimacing Thwomp Pillars.
- **Item & Sprite Concordance Polish (`mixer.rs`, `mario_ui.rs`)**:
  - Verified and finalized all shop collectibles, live weapons, carried turn blocks, message blocks, and SMW smiling bushes across sheets `0xCB`, `0xCC`, `0xCD`, `0xCE`, `0xD0`, and `0xDA`.

### Expanded Native N-SPC Soundtrack (19 Tracks)
- **6 New Native Mario Compositions (`mario_music.rs`)**:
  - **SMW Title Screen Calypso Theme** (#1): Steel pan and bongo percussion groove for the game's title screen.
  - **SMW Forest of Illusion Theme** (#5): Mysterious, wandering woods melody in F minor for the Lost Woods.
  - **SMW Vanilla Dome / Cavern Theme** (#8): Subterranean echo theme with staccato harp and deep bass for Death Mountain.
  - **SMW Star Road Celestial Waltz** (#26): Sparkling harp arpeggios and soaring flute for the Star Road Chamber.
  - **SMB3 Airship Military March** (#27): Driving snare cadence, timpani kicks, and dramatic brass fanfares for Bowser's Tower.
  - **SMW Underwater Waltz** (#24): Gentle 3/4 waltz for Water Dungeons and Swamp Palace.
- Expanded native in-game music mappings to 25 distinct situations while maintaining strict single-bank N-SPC budget.

### Build & Verification (`v0.4.0`)
- Deterministic 2 MiB LoROM build: `193` hardware patches across `18` rules (`948 ms`), `48` LZ2 sheets audited, `9` relocated in `$23..$3F` (0 overlaps).
- **SNES Checksum / Complement:** `0x7C48` / `0x83B7` (Valid: true)
- **Output CRC32:** `397E3E64`
- **Output MD5:** `897400efdf2b13e556109b919e2604fc`
- **Output SHA-1:** `6bf9b8cbafae03c76c4227331c4595f42a5ae04a`
- **Output SHA-256:** `eed0aac268c6523657ca083cc8cd40e620516757801bd69bbff38474f8b190ca`
- Full test suite: `26/26` passing Rust unit and integration tests.

---

## [0.3.0] — 2026-11-22 — Definitive Mario Experience

### Bug Fixes & Visual Quality
- **Definitive King Bowser Overhaul (`mario_ui.rs`)**:
  - Completely redesigned King Bowser for castle cell and throne room encounters into an authentic, menacing 32×32 Super Mario World sprite: curved ivory bull horns, fiery crimson-red mohawk mane, emerald green armored scaly head with forest contour, fierce red-browed eyes, broad golden snout with dark nostrils and sharp white fangs, spiked black collar with silver studs, rounded golden segmented belly plastron flanked by emerald green muscular arms with spiked black wrist cuffs and white claws, spiky green shell with white rim, and clawed yellow feet.
  - Added full roaring front-cast animation with wide dragon jaws showing fiery throat and fangs.
- **Koopa Guard Sword Alignment Fix (`mario_ui.rs`)**:
  - Recalibrated sword OAM offsets (`wx = 19, 11`, `wy = 7, 7` for Right; `wx = -11, -3`, `wy = 7, 7` for Left). Swords are now held firmly and naturally in the Koopas' hands in both gameplay and cutscenes instead of floating in empty air.
- **Authentic Luigi Twin Sprite (`mario_ui.rs`)**:
  - Overhauled Uncle into Luigi using Mario's exact 4bpp sprite from the player bank (`0x080000`) with a corrected palette LUT into Pal 6: vibrant emerald green cap and shirt, deep forest green overalls, peach skin, black mustache and pupils, white gloves, and brown boots.
- **Mario Overhead Carry Pose Polish (`demo.rs`)**:
  - Eliminated the artificial detached floating glove in front of Mario's side visor in `make_carry_side_head`, restoring an authentic, clean Super Mario World side carry profile with arms naturally raised holding the block above his head.
- **Smooth, Glitch-Free 3D Intro & Attract Mode (`mario_intro.rs`)**:
  - Removed conflicting DMA and palette hooks that were overwriting the polyhedral engine with chopped-up noise tiles on the title screen, restoring 100% fluid native 3D polygon rendering for the iconic spinning triangles behind "THE LEGEND OF PEACH" logo and copyleft notice.

### New Content & Structural Upgrades
- **Complete Sequential 372-Message Bank $1C Repacker (`mario_intro.rs`)**:
  - Safely repacks all 372 in-game French messages with dynamic pointer relocation, replacing remaining Zelda terminology (including "Le sorcier Bowser" → "Bowser") without clipping accents or control codes.
- **Pyramid Room 0x00 Golden SMW Super Star Floor (`mario_bosses.rs`)**:
  - Replaced the Triforce floor emblem with a 32×32 Golden SMW Super Star using Sheet 0x19 and Palette 3 (`0x0DF0..0x0DFF`) and room blockset repointing (`0x005D66 = 0x19`), leaving globally shared dungeon tiles (`raw:0x0E`) 100% untouched.
- **Kamek's Shop 8×8 Mini Mushroom & Yoshi Berry (`mario_ui.rs`)**:
  - Replaced the last vanilla 8×8 heart in Kamek's Shop (`0x087000`) with a crisp 8×8 Red Mini Super Mushroom and Yoshi Berry.
- **Animated 4-Frame SMW Super Star Cursor (`mario_ui.rs`, `mixer.rs`)**:
  - Replaced the file select fairy cursor with an authentic 4-frame sparkling Super Mario World Super Star cursor in Sheet 0x76 (`0x088E00`).
- **Multi-Region Known ROM Catalog (`rom.rs`)**:
  - Added an official catalog of 7 verified ALTTP and SMW revisions (USA NTSC, European PAL, French, German, Japanese) with detailed compatibility status.

### Build & Verification (`v0.3.0`)
- Deterministic 2 MiB LoROM build: `156` hardware patches across `17` rules (`824 ms`), `46` LZ2 sheets audited, `8` relocated in `$23..$3F` (0 overlaps).
- **SNES Checksum / Complement:** `0x2EC1` / `0xD13E` (Valid)
- **Output CRC32:** `0x0DC79327`
- **Output MD5:** `2eb60aa2c2a6ef0e5ed484a839a9ad34`
- **Output SHA-1:** `8f5bfccf94ef528059f714f9a63447e5e108157e`
- **Output SHA-256:** `fe7390a1c4e58e219d8e6f023cc4f09ce6e7e6d5164bf1bb30b2dbf1c2828785`
- Full test suite: `25/25` passing Rust unit and integration tests.

---

## [0.2.0] — 2026-11-22 — Beyond the First Release

### Bug Fixes (Identified via Independent Phase 2 Live Audit)
- **Fixed Corrupted Shop Price Digits (`0` & `1`) Across All Shops (`mixer.rs`)**:
  - Previously, writing Kamek's 16×16 body tile into `Sheet 0xBC` (`raw:0xBE`) row `2..3` overwrote shop price digits `'0'` and `'1'` at `(0,3)` and `(1,3)`, garbling `150`, `10`, and `50` in Kamek's Shop.
  - Kamek's wizard robe now writes strictly to row `2` (`(0,2)` / `(1,2)`), keeping Kamek behind the counter while restoring crisp `150`, `10`, and `50` coin prices everywhere.
- **Eliminated the Last Blue Zelda Bombs (`mixer.rs`, `mario_ui.rs`)**:
  - Replaced the vanilla blue 10-Bomb pack in Kamek's Shop (`Sheet 0xCB` / `raw:0xCD` `(8,2)`) with a Jet-Black SMW Bob-omb sporting a crisp white `'10'` badge, replaced single/3/10 bomb drops on `Sheet 0xCD` (`(11,0)` and `(13,0)`), and switched `ReceiveItem` bomb palettes (`0x0484C3`, `0x0484C4`, `0x0484CD`) from Blue (`0x02`) to `OBJ Pal 1` (`0x01`).
- **Fixed Broken Twin-Pillar Eastern Palace Armos Knights (`mario_ui.rs`)**:
  - Live inspection revealed Eastern Palace Armos Knights assemble `Sheet 0x8E` (`raw:0x90`) cols `0..1` + `2..3` side-by-side without H-flipping, which caused the previous single-width Thwomp tiles to display as siamese twin pillars.
- **Fixed French Start Menu `'Y'` Glyph Mapping & Unmapped `'Z'` (`mario_ui.rs`)**:
  - Mapped `'Y'` to verified French `Sheet 0xDA` tile `0x67` (avoiding the French `"CI"` ligature at `0x68`) and updated Silver Bow name to `MEGA BILL`.
- **Fixed Mini-Mushroom Drop Tile Alignment (`mario_ui.rs`)**:
  - Centered the 8×8 Red Mini Super Mushroom and Yoshi Berry cleanly inside `y=0..7` of `Sheet 0xCD (6,2)/(7,2)` instead of straddling the tile boundary.

### New Content & Major Upgrades
- **Complete 100% Super Mario World Boss Roster — All 12 Bosses!**
  - **NEW Eastern Palace Boss — 6 Giant 32×32 Hopping SMW Chargin' Chucks (`mario_ui.rs`)**: Seamless symmetric 32×32 football-helmeted Chargin' Chucks in blue team gear (turning blazing red on the final enraged survivor).
  - **Upgraded Castle Tower King Bowser (`mario_ui.rs`, `mixer.rs`)**: Sharper golden-snout side profile with bold fangs, blazing mane, and full coverage of `Sheet 0xB4` cols `8..11` (removing the last vanilla Agahnim teleport cape tiles).
- **Start Menu & HUD Visual Overhaul (`mario_ui.rs`)**:
  - Redesigned the equipped **`BILL BLASTER`** icon (`0xBA/0xBB/0xCA/0xCB`) from a yellow oval into a sleek, unmistakable Jet-Black & White SMW Bullet Bill with angry eye, white boxing glove, and golden thruster.
  - Transformed the **`CRISTAUX`** box and its 7 blue Zelda diamonds into **`YOSHIS`** with **7 Spotted SMW Yoshi Eggs** (`Sheet 0xDA` `0x44..0x47`).
  - Switched Equipped Lv1 Shield (`0x06F791`) to Emerald Green (`Pal 7`, `0x3C/0x7C`) matching Mario's live Green Koopa Shell, and upgraded Moon Pearl tiles (`0x56/0x66/0x67`) to a golden 3-UP Moon emblem.
- **Full Live French Compression Dictionary & Script De-Zeldafication (`mario_intro.rs`)**:
  - Live-patched the Bank `$0C` French compression dictionary (`0x065A33..0x065CAB`, net `-3` bytes, exact SNES pointer recalculation): `Ganon` → `Bowser`, `rubis` → `pièces`, `Sahasrahla` → `Papy Toad`, `Perle de Lune` → `Lune 3-UP`.
  - Expanded Bank `$1C` script rewriter (`Hyrule` → `Champi`, `Cocorico` → `ToadTown`, `Excalibur` → `Épée Star`, `Sages` → `Toads`, `l'Arc`/`flèches` → `le Blaster`/`Bills`, `coeur` → `champi`, plus Final Boss speech polish), cutting over 100 remaining Zelda references down to near zero (`Ganon`: 0, `Zelda`: 0, `Agahnim`: 0, `Cocorico`: 0, `Perle de Lune`: 0).
- **3 New Native SPC700 N-SPC Compositions (`mario_music.rs`: 10 → 13 Tracks!)**:
  - **SMW Valley of Bowser** for Dark World Overworld (`#9`).
  - **SMW Bonus Room / Yoshi's House** for Toad & Kamek Shops (`#23`).
  - **SMW Final King Bowser Showdown** dedicated to Final Boss Giant King Bowser (`#28`, distinct from the 7 Koopalings theme `#21`).

### Build & Verification (`v0.2.0`)
- Deterministic 2 MiB LoROM build: `182` hardware patches across `18` rules (`856 ms`), `46` LZ2 sheets audited, `0` overlaps across `$23..$3F`.
- **SNES Checksum / Complement:** `0xEE72` / `0x118D` (Valid)
- **Output CRC32:** `0xDA56A034`
- **Output SHA-256:** `c700d1d2b60f50e789e2214fa8528c0c3af132ebc8af35a2f7ced5aa16735edb`
- Dual-core live verification in Mednafen `snes_faust` and cycle-accurate `bsnes` (`snes`), plus live SPC700 audio RMS verification across all new tracks and `21/21` Rust tests (both ROM-backed and ROM-free CI).

---

## [0.1.0] — 2026-11-22 — First Public Release

### Highlights
- **First public open-source release** of the pure headless Rust SNES ROM transformation engine (`snes-core` / `snes-cli`) that deterministically builds **The Legend of Peach: A Mario to the Mushroom Kingdom** from user-provided legal backups of *The Legend of Zelda: A Link to the Past (France, PAL)* and *Super Mario World (Europe, PAL)*.
- Custom 32-frame 3D Mario Trio intro & metallic `PEACH` title logo, playable Mario + Uncle Luigi + Peach + Toads + Kamek, 16-sheet SMW bestiary, 9 worldwide tilesets, all 7 Koopalings + Giant King Bowser + Pokeys + Wiggler, 10 native SPC700 N-SPC themes, strict No-Intro SHA-256 verification, hardware audit, and ROM-free CI (`SHA-256: 32540c4629c6b797f6933c7966a419fd959a3f711f36347f4348ceb2fa85b99c`, Checksum `0x464D` / `0xB9B2`).
