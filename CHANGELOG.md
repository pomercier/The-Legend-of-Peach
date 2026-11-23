# Changelog

All notable changes to **The Legend of Peach: A Mario to the Mushroom Kingdom** are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
