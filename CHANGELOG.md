# Changelog

All notable changes to **The Legend of Peach: A Mario to the Mushroom Kingdom** are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.10.0] — 2026-11-27 — Super Mario Adventure Beyond Limits

### Pillar 1 — Real Physical Inter-Regional 5-Hub Warp Pipe Network (`mario_engine.rs`, `mario_map.rs`, `MU-041`)
- Replaced `v0.9.0`'s stationary front-yard coin bonus with a **genuine 5-Hub Physical Inter-Regional Overworld Warp Pipe Network** connecting **World 1 Yoshi's Island (`0x2C r09 c07`) -> Toad Town Weathervane Plaza (`0x18` / `0x19 r12 c03`) -> Vanilla Dome Terracotta Mountain (`0x03` / `0x0B r10 c05`) -> Lake Cheep-Cheep (`0x35` / `0x3E r04 c07`) -> Forest of Illusion (`0x12 r06 c05`) -> Yoshi's Island (`0x2C`)**.
- Implemented a hardware-legal 3-state Warp Pipe engine (`W_PIPE_STATE` `$7E:F540`) invoking ALTTP's complete native Bank `$00/$02/$0A/$0D` Overworld loader inside Forced Blank: 15-frame brightness fade-out + native SPC700 Warp Pipe triple-pulse SFX (`$012E = 0x15`) -> `JSL $00:8919` + `JSL $0D:F988` + `JSL $0A:B8A5` (`$1AF0` slot) + `JSL $0A:B928` (`JSL $02:B215 -> JSR $EDDB` full BG1+BG2 tilemap buffer + `$17 = 0x04`) -> 1-frame NMI Forced Blank DMA hold -> 15-frame emergence fade-in + 80-frame anti-retrigger cooldown.
- Caught and fixed a Pass-1 half-screen VRAM tilemap seam by adding `JSL $0A:B928` + the 1-frame Forced Blank NMI DMA hold; Pass 2 verified **3 consecutive live warps across 4 biomes in a single uncut run (`0x2C -> 0x18 -> 0x03 -> 0x35`)** with zero seams and bit-identical SRAM parity on both `snes_faust` and cycle-accurate `bsnes` ([`docs/WARP_NETWORK_REPORT.md`](docs/WARP_NETWORK_REPORT.md)).

### Pillar 2 — Complete `128 / 128` Overworld + Star Road `0x80` + Vanilla Dome Terracotta Palette + SMW Gold-Riveted HUD (`mario_map.rs`, `mario_world.rs`, `mario_ui.rs`, `ZRI-007 -> 0`, `ZRI-014 -> 0`)
- Closed all remaining `20 / 128` untouched Overworld screens (`64/64` Light World + `64/64` Dark World = **`128 / 128`**) PLUS Special Area **`0x80` Star Road Celestial Grove** using collision-identical uniform `TileAttr` (`$00:FBD9`) pools (`0` grammar violations, `0` hardware collision regressions, `0` cross-screen border breaks across 224 edges; `ZRI-007: 4 -> 0`).
- Overhauled Light World **Vanilla Dome / Chocolate Mountain (`OWMainPal[2]` @ `0x0DE754`, `MU-045`)** into sunlit SMW terracotta-amber crags & warm sandstone plateaus (live-verified on `snes_faust` & `bsnes`).
- Redesigned vanilla HUD border tiles (`0x50..0x5D` in Sheet `0x6F`/`0xDB` and `0x8B/0x8F` in Sheet `0x70`/`0xDC`, **`ZRI-014: 4 -> 0`**) into an authentic **SMW Gold-Riveted Reserve Item Box**, **Golden-Capped Star-Power Capsule**, and **Mini-Star `- VIES -` accents**.

### Pillar 3 — Deep 65c816 Gameplay Expansion & Checksummed SRAM Persistence (`mario_engine.rs`, WRAM `$7E:F500..$7E:F548`, SRAM `$7E:F400..$7E:F407`)
- **Persistent Checksummed SRAM Dragon Coins (`$7E:F400..$7E:F404`):** 32 unique Dragon Coins stored inside ALTTP's native auto-checksummed `$7E:F000..$7E:F4FF` save slot — `+10` Coins 1-time (permanently eliminating `v0.9.0`'s infinite warp pipe coin farming!) + **1-UP Royal Full Heal & Star Refill** every 5 Dragon Coins ([`docs/SRAM_PERSISTENCE_REPORT.md`](docs/SRAM_PERSISTENCE_REPORT.md)).
- **Playable Luigi Mode (`Down + L + R` grounded, SRAM `$7E:F407`, Pal 9):** Live Emerald Cap & Navy Overalls CGRAM + higher (`0x02F0`) floatier (`-0x001E`) Luigi jump physics, persisting across save/quit/reboot!
- **SMW / SMB3 Frog Suit (`$5D == 0x04` + Flippers `$F356 != 0`, Pal 10):** Auto-activates Electric Lime Frog Suit CGRAM + 4-dir Swim Turbo Boost in deep water (live-verified swimming in Lake Cheep-Cheep & Zora's Waterfall!).
- **Timed SMW P-Switch 2× Gold Rush (`Up + L + R` grounded or Dungeon ON/OFF flip):** 300-frame timed Gold Rush doubling all Mario coin rewards (`2×`, verified `250 -> 270 -> 272` on Turn-Block throw + Dragon Coin `#10`).
- **Yoshi Companion Tongue Swallow (`B`/`Y` while riding Yoshi):** Gulps nearby edible enemies within 36 px into a puff for `+1` Full Heart (`+8` eighths), `+2` Coins, and Dragon Coin `#9` (live-verified healing `11 -> 12` hearts & `250 -> 262` coins!).
- **Authentic SMW Dry Bones Crumble/Rebuild AI (`STOMP_TABLE` class `0x06`, `0x85/0x91/0x9E/0xA7`) & Final King Bowser Phase 2 Counterplay (`0xD6`):** Ordinary stomp crumbles Dry Bones into a 120-frame stunned bone pile while Ground Pound Slam shatters them permanently; Ground Pounding Final King Bowser triggers heavy throne tremor (`$011A = 0x18`), `+5` Bonus Coins, and Dragon Coin `#11` (live-verified `250 -> 281` Coins on cycle-accurate `bsnes`!).
- **100% Bank `$1C` French Narrative Cleanse (`mario_intro.rs`, `MU-071`):** Eliminated all remaining `35` Zelda lore/item occurrences across all 372 Bank `$1C` French messages (`201` vanilla -> `35` `v0.9.0` -> **`0`** `v0.10.0`).

### Pillar 4 — Honest 14-Dimension Scoring, Real Audio QA, Differential Parity & Clean-SRAM Playthrough
- Replaced coupled quality scores with an **independent 14-dimension auditable rubric** ([`docs/QUALITY_SCORES.json`](docs/QUALITY_SCORES.json)), explicitly documenting why `v0.10.0` does not jump prematurely to `v1.0.0` (`14_physical_hardware_validation = 0% HARDWARE_NOT_TESTED`, `13_playthrough_completeness = 58%`).
- Built **`scripts/qa_engine_v0100.py`** verifying **`14 / 14 VISUAL_PASS + AUDIO_PASS`** (real WAV spectrum extraction `-24.4..-27.8 dBFS`, `54.5%..88.8%` active energy, `0%` clipping), true **identical-scenario differential parity** between `snes_faust` and cycle-accurate `bsnes` (`BIT_IDENTICAL` saved `.srm`!), **5 newly live-verified bestiary rooms** (`Spiny 0x26`, `Chain Chomp 0x6A`, `Monty Mole 0x20`, `Lava Bubble 0x15`, `Snifit 0x0A`), **100% blank-SRAM opening playthrough -> Star-Torch chest -> Rainy Overworld -> Save -> Cold Reboot**, and real `v0.9.0` vs `v0.10.0` Before/After comparison plates. Updated 33-table Mega SQLite DB (`1,104` rows).
- Added 6 dedicated engineering reports: [`WARP_NETWORK_REPORT.md`](docs/WARP_NETWORK_REPORT.md), [`FULL_PLAYTHROUGH_REPORT.md`](docs/FULL_PLAYTHROUGH_REPORT.md), [`PROGRESSION_GRAPH.json`](docs/PROGRESSION_GRAPH.json), [`AUDIO_VALIDATION_REPORT.md`](docs/AUDIO_VALIDATION_REPORT.md), [`CROSS_EMULATOR_REPORT.md`](docs/CROSS_EMULATOR_REPORT.md), [`SRAM_PERSISTENCE_REPORT.md`](docs/SRAM_PERSISTENCE_REPORT.md), and [`REAL_HARDWARE_TEST_GUIDE.md`](docs/REAL_HARDWARE_TEST_GUIDE.md).
- Deterministic 2 MiB LoROM build (dual independent runs bit-identical via `cmp` + `sha256sum`): `756` hardware patches across `18` rules (`~2.9 s`), `51` LZ2 sheets audited, `268` relocated blocks in `$23..$3F` (`0` overlaps), `32/32` passing Rust tests, `0` clippy warnings, `0` repo safety leaks.
- **SNES Checksum / Complement:** `0x930C` / `0x6CF3` (Valid: `true`)
- **Output CRC32:** `37CE4B03`
- **Output SHA-256:** `2da376f5c5cd6f760c07e465bdba252fe1aa1f03e69f17dcabcff1ecd6e072b1`

---

## [0.9.0] — 2026-11-27 — The Ultimate Mario Adventure

### Pillar 1 — Title Screen Master Sword 100% Eliminated (`mario_intro.rs`, `ZRI-009: 10 -> 0`)
- Disassembled ALTTP's Title Screen Master Sword OAM routine (`$0E:BE1A..$0E:BF84`, 10 `16×16` sprites `0x00..0x0E, 0x20, 0x22`, `attr = 0x21` OBJ Pal 0) and pinpointed its exact compressed sprite sheet in FR PAL ROM: **`raw 0xA5` (`SP[0x32]`, PC `0x0B3CAD`)**.
- Replaced all 10 Master Sword sprites (while preserving the original sparkle tiles in rows 2–3 cols `4..15` bit-identical!) with an authentic 16-bit **Royal Princess Peach Golden Star-Scepter & Super Star Crown**: gleaming SMW Super Star finial (`0x00`), Princess Peach's jeweled golden crown (`0x02`), feathered SMW Paratroopa royal wings (`0x20`/`0x22`), jeweled royal scepter collar (`0x04`), ornate golden-pearl scepter shaft (`0x06..0x0C`), and rounded golden orb finial (`0x0E` — zero sword blade/tip!). Tuned Title OBJ Pal 0 Sapphire (`0x0DD43A = 0x7544`) & Star Gold (`0x0DD444 = 0x039F`). Visually verified live on cycle-accurate `bsnes` (`docs/media/screenshots/01_title_screen.png`).

### Pillar 2 — Physical 3D Glossy Green SMW Overworld Warp Pipes (`mario_ui.rs`, `mixer.rs`, `mario_world.rs`, `mario_map.rs`, `mario_engine.rs`, `ZRI-017: 10 -> 0`)
- Crafted 3D glossy emerald-green SMW Warp Pipe graphics (`build_smw_ow_warp_pipe_half_8x16`) with rimmed lip, dark hollow mouth, white specular shine, and curved shading into slot `(0,2)/(0,3)` (`m16 = 0x21B`, Pal 6 Emerald Green, solid `TileAttr = 0x27`) of both Light World (`raw 0x3E` / sheet `0x3C`) and Dark World (`raw 0x3F`) Overworld prop sheets, relocating Heavy Rock Brick/Skull Blocks cleanly to `(3,2)/(3,3)` (`m16 = 0x239`).
- Placed Physical SMW Green Warp Pipe **`m32 = 0x02D4`** (`[0x034, 0x034, 0x21B, 0x034]`) directly in **World 1 Yoshi's Island Mario's House Front Yard (`Screen 0x2C r09 c07`)** and inside strictly verified open 3×3 `0x001D` clearings on curated Light & Dark World Overworld Hubs (`0` grammar violations, `0` collision regressions, `0` border breaks across all 128 screens).
- Added **`Mario_CheckWarpPipe`** (`$22:9000`): Ground Pounding beside Mario's Front-Yard Green Warp Pipe (`$8A == 0x2C`, `$23 == 0x08`, `$21 == 0x0B`, `X_lo >= 0xC4`, `Y_lo in 0x10..0x5F`) triggers the native SPC700 **Descending Warp Pipe Triple-Pulse SFX (`$012E = 0x15`)**, refills Star Power to 100% (`$7E:F36E = 0x80`), and awards **`+5` Secret Bonus Gold Coins** (visually verified rolling `250 -> 255 -> 260`).

### Pillar 3 — Complete 8 / 8 Bespoke Koopaling & King Bowser Fortress Palettes (`mario_world.rs`, `ZRI-008: 4 -> 0`)
- Completed ROM-disassembly-verified (`$07:FA90`) architectural wall & floor overhauls for ALL 5 remaining major dungeons (joining Morton `[8]`, Iggy `[9]`, and Ludwig `[12]` -> **8/8 bespoke SMW Fortresses!**):
  - **`DungPal[1]` (`0x0DD7E8`, Room `0x006`):** Wendy's Turquoise & Coral Sea Palace.
  - **`DungPal[4]` (`0x0DDA04`, Room `0x0DE`):** Lemmy's Glacial Frost-Cyan Star-Ice Circus Fortress.
  - **`DungPal[5]` (`0x0DDAB8`, Room `0x033`):** Larry's Sun-Baked SMW Chocolate Island Desert Pyramid.
  - **`DungPal[10]` (`0x0DDE3C`, Room `0x0AC`):** Roy's Midnight Violet-Slate & Magenta Koopa Castle.
  - **`DungPal[13]` (`0x0DE058`, Room `0x00C`/`0x00D`):** King Bowser's Obsidian & Lava Keep.

### Pillar 4 — 65c816 Boss Combat Mechanics (`mario_engine.rs`, `ZRI-019: 2 -> 0`)
- Added **Boss Encounter Class `0x05`** in `$22:B000` covering all 12 Boss sprite IDs (`0x09, 0x53, 0x54, 0x7A, 0x88, 0x8C, 0x92, 0xA2, 0xBD, 0xCB, 0xCC, 0xCD, 0xCE, 0xD6`), safely excluded from instant stomp/shell-bowling KOs so multi-phase boss scripts never break.
- Added **Boss Ground Pound Stagger & Coin Shower** inside `Mario_CheckGroundPoundShockwave` (`$0EA0 = 0x16` recoil flash, `$011A = 0x12` fortress shake, `+3` Gold Coins & `+6` Star Power, safe `-2` HP chip only if `HP >= 4`) and **`Mario_UpdateBossCombat`** (`W_BOSS_HIT_CD = $7E:F51B`, awarding `+2` Gold Coins + `+4` Star Power + Mario Coin Chime on every boss damage recoil, 24-frame debounce) — visually verified live on Morton (`0x05A`), Wendy (`0x006`, `250 -> 268`), Pokeys (`0x033`, `250 -> 261`), and cycle-accurate `bsnes` Final King Bowser (`0x000`, `250 -> 263`).

### Transparent Quality Scores, Dual-Emulator QA & Build Verification (`v0.9.0`)
- **Mario Identity Score raised to `98.1%`** (up from `91.8%` in `v0.8.0`), **Perceptible Zelda Residue reduced to `1.9%`** (`8 / 416` honest remaining weight), and **Gameplay Depth Score raised to `97.5%`** ([`docs/QUALITY_SCORES.json`](docs/QUALITY_SCORES.json)).
- Built **`scripts/qa_engine_v090.py`** verifying **10/10 live scenarios (`VISUAL_PASS`)** on `snes_faust` (8) and cycle-accurate `bsnes` (2), updating the 33-table Mega SQLite DB (`980+` rows). Pixel inspection caught and resolved 1 Pass-1 index mismatch (`raw 0x32` -> `raw 0xA5`) before release.
- Deterministic 2 MiB LoROM build (dual independent runs bit-identical via `cmp` + `sha256sum`): `671` hardware patches across `18` rules (`~2.6 s`), `51` LZ2 sheets audited (including `raw 0xA5` & `raw 0x3E`), `226` relocated blocks in `$23..$3F` (`0` overlaps), `32/32` passing Rust tests, `0` clippy warnings, `0` repo safety leaks.
- **SNES Checksum / Complement:** `0xBC86` / `0x4379` (Valid: `true`)
- **Output CRC32:** `261F4214`
- **Output SHA-256:** `5546dfaec62b98dc8c91bbbc58a7b16df6de91b9d8e6f0d56a1b7f1b36693fbd`

---

## [0.8.0] — 2026-11-27 — Super Mario Universe Revolution

### Native 65c816 Super Mario Universe Engine (`mario_engine.rs`, Bank `$22:9000..$22:B2FF`, WRAM `$7E:F500..$7E:F53F`)
- **Authentic Mario Ground Pound & Hardware ON/OFF Switch Flip:**
  - Press `Down` while airborne (`Z >= 7 px`) to execute a 6-frame dramatic mid-air stall (`W_AIRBORNE = 3`) followed by a `-5.0 px/frame` vertical slam (`W_AIRBORNE = 4`).
  - Impact triggers SNES hardware screen shake (`$011A = 0x0E`), heavy thud SFX (`$012F = 0x0C`), a 48px armor-piercing shockwave cracking Spiked/Armored enemies (`class 2`) for `+2` Gold Coins (`$7E:F360`) & `+4` Star Power, AND **flips nearby Dungeon SMW ON/OFF Crystal Switches (`$0E20 == 0x1E`, `<= 64px`, guarded once-per-impact via `W_TMP_B`) by invoking ALTTP's native hardware Submodule `$16` (`$7E:C172 ^= 1`, `$11 = 0x16`)** — visually verified flipping the entire Room `0x077` ON/OFF block matrix `RED <-> BLUE`!
- **Real Koopa Shell Retract, Kick & Wall-Ricochet Bowling Engine (`STOMP_TABLE` class `0x03`):**
  - First stomp on Koopa Troopas / Hammer Bros / Buzzy Beetles (`0x13/0x26/0x41..0x4B/0x8D/0x8E`) retracts them into a **Stunned Koopa Shell** (`W_SHELL_STATE = 1`), awards `+1` Coin (`$7E:F360`), and bounces Mario upward.
  - Walking into the stunned shell kicks it at `±3 px/frame` along Mario's facing direction (`$2F`), **ricocheting cleanly off walls/boundaries** (`[0x18..0xE4]`, `[0x20..0xD8]`) and **bowling through other active enemies** for instant KOs & `+2` Bonus Coins (visually verified in Room `0x081` with live `+1` Coin increment and dual wall ricochet!).
- **Authentic SMW Boo Orientation-Freeze AI (`STOMP_TABLE` class `0x04` — Overworld `0x19` & Ghost House `0x8B`):**
  - Compares Mario's facing vector (`$2F`) against each active Boo every frame using 16-slot WRAM coordinate anchors (`$7E:F520..$7E:F53F`): Boos shyly **freeze motionless** when Mario faces them and **creep stealthily toward Mario** the instant he turns his back (visually verified across 3 simultaneous Boos in Room `0x059`!).
- **6-Form Live CGRAM Power-Up State Machine (`$22:B100`, 9 BGR555 Palettes = 270 B) + Playable Yoshi Companion:**
  - `0` **Classic Super Mario**, `1` **Fire Mario** (`$0202 == 0x06`), `2` **Ice Mario** (`$0202 == 0x07` `FLEUR DE GLACE`: Electric Frost-Cyan / Glacier-Blue CGRAM + **Ice Stomp Shatter** bonus `+3` Coins on frozen `$0DD0 == 0x0B` enemies), `3` **Cape Mario** (`$0202 == 0x13` `PLUME DE CAPE`: Blazing SMW Golden Cape CGRAM + `-0.75 px/frame` **Mid-Air Slow-Descent Glide** while holding `L`/`R`), `4` **Playable Yoshi Companion** (`$0202 == 0x0E` `FILET YOSHI`: Vibrant SMW Yoshi Emerald Saddle & Orange Boots, higher jump `+2.78 px/f`, 16-frame **Yoshi's Island Flutter Jump**, and **SMW 1-Hit Damage Absorption Shield** restoring Mario's full HP `$7E:F36D` on enemy contact with 60-frame Classic escape before auto-remount!), and `5..8` **Resource-Balanced Super Starman (`L + R`)** continuously draining Star Power (`-1` every 6 frames from `$7E:F36E`, ending cleanly on depletion).
  - Protected across all Overworld/interior/warp transitions via `W_PAL_DIRTY` and an 8-frame non-Classic CGRAM guard.
- **Interactive Turn-Block / Bush Throw Reward & Multi-Stomp Combos:**
  - Throwing any lifted Smiling Bush or Turn-Block (`$0308` transitioning `1 -> 0`) increments Target Coins (`$7E:F360/$F361`, verified `250 -> 251`), refills `+4` Star Power, and plays the Mario Coin Chime (`$012E = 0x2B`).

### Distinct Koopaling Dungeon Fortress Architecture (`mario_world.rs`, Verified via ROM `$07:FA90`)
- Disassembled `$01:B609` -> `$07:FA90` (`PaletteSet` -> `DungPal` index) and transformed walls & main floors of three signature Koopaling Fortresses while preserving sprite/torch/switch readability:
  - **Morton's Volcanic Basalt Fortress (`DungPal[8]` @ `0x0DDCD4`, Room `0x05A`):** Charred volcanic basalt walls, glowing magma-amber mortar, and dark obsidian floor.
  - **Iggy & Big Boo's SMW Ghost House (`DungPal[9]` @ `0x0DDD88`, Rooms `0x029` & `0x059`):** Spooky deep SMW Ghost House indigo-violet timber walls and moonlit lavender parquet floor.
  - **Ludwig's Molten Mech Fortress (`DungPal[12]` @ `0x0DDFA4`, Room `0x0A4`):** Forged steel, riveted bronze plating, and ember-iron floor.

### Mario Universe Bible, Transparent Multi-Dimensional Quality Scores & 33-Table Mega QA DB
- Created **`docs/MARIO_UNIVERSE_REGISTRY.json`** (`72` Mario elements across `12` categories) and **`docs/QUALITY_SCORES.json`** documenting transparent scores: **Mario Identity Score `91.8%`** (up from `86.8%` in `v0.7.0`), **Perceptible Zelda Residue Score `8.2%`** (`34 / 416` remaining honest weight), and **Gameplay Depth Score `87.5%`**.
- Built **`scripts/qa_engine_v080.py`** with a **33-table normalized Mega SQLite Validation Database** (`33/33` tables populated, `640` rows, local JSON + CSV export) verifying **12/12 `v0.8.0` live Mednafen scenarios (`VISUAL_PASS`)** across `snes_faust` and cycle-accurate `snes` (`bsnes`) — catching and fixing 5 real hardware bugs between Pass 1 and Pass 2.
- Preserved 100% of all 7 `v0.6.0` fixes and all `v0.7.0` world/boss/state achievements with zero regressions.

### Build & Verification (`v0.8.0`)
- Deterministic 2 MiB LoROM build (dual independent runs bit-identical via `cmp` + `sha256sum`): `664` hardware patches across `18` rules (`~2.6 s`), `49` LZ2 sheets verified, `225` relocated blocks in `$23..$3F` (`0` overlaps), `32/32` passing Rust tests, `0` clippy warnings, `0` repo safety leaks.
- **SNES Checksum / Complement:** `0x1B55` / `0xE4AA` (Valid: `true`)
- **Output CRC32:** `113947D1`
- **Output SHA-256:** `2270d55f8f4d09a164a939213531be80e4237a94488f535749708801ba925fd8`

---

## [0.7.0] — 2026-11-27 — Mushroom Kingdom: World Transformation

### Full ROM World Discovery & Systemic 108/128 Overworld Transformation (`mario_map.rs`)
- **Complete Binary Inventory (`docs/WORLD_CONNECTIVITY_REPORT.md`, `docs/ENEMY_BEHAVIOR_REGISTRY.json`)**: Extracted all `160/160` Overworld screens (`64` Light World + `64` Dark World + `32` Special), all `296/296` Underworld room headers (`PC 0x0274A2`), all `247` populated room sprite lists (`PC 0x04D653`, `210` unique placed sprite IDs), `133` entrance links (`PC 0x014895`), and the exact 512-byte Overworld hardware collision table `TileAttr` at `PC 0x007BD9` (`$00:FBD9`).
- **Systemic Multi-Biome Overworld Transformation (`56/64` LW + `52/64` DW = `108/128` screens, `322` Map32 tiles):**
  - Classified all `160` screens into 15 Mario Light/Dark/Special biomes (`OverworldBiome`), enriching pure walkable grass (`0x001D`, `TileAttr 0x48`), forest/courtyard floors (`0x0106`, `TileAttr 0x00`), desert/swamp sand (`0x05E5`, `TileAttr 0x08`), and aquatic sea tiles (`0x0E90`, `TileAttr 0x00`) plus strictly 3×3-open-interior liftable SMW Turn-Blocks (`0x06C7`) and Smiling Bushes (`0x07CC`).
  - Guarded by automated unit test `test_v070_all_128_overworld_screens_collision_grammar_and_borders`: **`0` grammar violations, `0` hardware collision regressions, `0` cross-screen border breaks across all 224 cardinal edges**.
- **Distinct Dark World Artistic Identity — Bowser's Volcanic Kingdom & Obsidian Citadel (`mario_world.rs`):**
  - Overhauled Dark World main palettes (`OWMainPal[1]` @ `0x0DE70E` & `OWMainPal[3]` @ `0x0DE79A`) from muddy olive-brown to volcanic indigo-obsidian basalt, crimson-plum turf, and glowing magma-amber ridges.
  - Transformed Dark World Pyramid (`Screen 0x5B/0x5C/0x63`, `OWAuxPal[17]` @ `0x0DEB36`) from golden Egyptian sandstone into **Bowser's Obsidian Lava Citadel** with carved stone Bowser Statues and volcanic magma sunset skies (verified identically on `snes_faust` and cycle-accurate `bsnes`).

### Zelda Residue Elimination (`ZRI` Reduced from `19.2` -> `13.2`, `MII` Raised to `86.8`)
- **Koopa Guard Weapon Conversion (`mixer.rs`, `ZRI-016` Resolved):** Replaced all 8 medieval Zelda sword & spear tiles in compressed sprite sheet `raw 0xBA` (`$6B/$7B`, `$6C/$7C`, `$6E/$7E/$6F/$7F`) with authentic **SMW Hammer Bro Hammers** and **SMW Spiked Koopa Clubs** (verified live in Overworld combat).
- **Castle Tower King Bowser French Dialogues (`mario_intro.rs`):** Replaced vanilla Agahnim medieval French messages `#316`, `#318` (removing `"occire"`), and `#319` (removing `"Héros"`) with playful King Bowser dialogue (`"BWAHAHAHA ! Tu oses défier le grand Roi Bowser dans sa tour ?"`), verified live in Room `0x020`.

### 100% Live `VISUAL_PASS` Across All 12/12 Bosses & All 16/16 Mario States (`scripts/qa_engine_v070.py`)
- **Next-Gen Parallel QA Engine & Persistent Local SQLite DB:** Built reusable multi-worker Mednafen (`snes_faust` + cycle-accurate `snes`/bsnes) harness recording temporal metrics, contact sheets, and historical runs into local SQLite (`23/23 VISUAL_PASS`, `94` logged historical runs).
- **All 12 / 12 Bosses Live Verified in Real Arenas (`docs/BOSS_VISUAL_REPORT.md`):** Chargin' Chucks (`0x0C8`), 3 Giant Pokeys (`0x033`), Giant Wiggler (`0x007`), Castle Tower Bowser (`0x020`), Morton (`0x05A`), Wendy (`0x006`), Winged Iggy (`0x029`, `v0.6.0` guard), Roy Koopa (`0x0AC`, live Maiden sunlight transformation via `$7E:F3CC=0x06` & Room `0x065` bit `0x0100`), Lemmy (`0x0DE`, Fire Mario melting ice), Larry (`0x090`), Ludwig (`0x0A4`), and Final Boss King Bowser (`0x000` on cycle-accurate `bsnes`).
- **All 16 / 16 Mario States Live Verified (`docs/GAMEPLAY_COVERAGE_REPORT.md`):** Closed all 6 previously un-emulated states from `v0.6.0` — Heavy Lift & Carry (4-dir), Prop Throw, Lake Hylia Deep Water Swim (4-dir) & Dive alongside SMW Cheep-Cheep, Sanctuary Push Block, Buzzblob Electric Shock X-Ray (`$5D=0x13` + PPU mosaic), Pitfall Drop (`-1F`), and KO Spin / KO Flat.
- Preserved 100% of all 7 `v0.6.0` blocking fixes with zero regressions.

### Build & Verification (`v0.7.0`)
- Deterministic 2 MiB LoROM build (dual independent runs bit-identical via `cmp` + `sha256sum`): `661` hardware patches across `18` rules, `49` LZ2 sheets verified, `225` relocated blocks in `$23..$3F` (`0` overlaps), `32/32` passing Rust tests, `0` clippy warnings.
- **SNES Checksum / Complement:** `0x1285` / `0xED7A` (Valid: `true`)
- **Output CRC32:** `F5545171`
- **Output SHA-256:** `f975131b5eaeee99ee7215ae636a380946ddc48832d3e7be95a11ab5d344dfee`

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
