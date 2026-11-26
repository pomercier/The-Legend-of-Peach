<p align="center">
  <img src="docs/media/banner/hero_banner.png" alt="The Legend of Peach: A Mario to the Mushroom Kingdom — Promotional 16-bit Pixel Art Banner" width="100%" />
</p>

<p align="center">
  <sub><em>Promotional 16-bit key artwork created for the repository. Authentic live SNES emulator screenshots are featured immediately below.</em></sub>
</p>

<h1 align="center">THE LEGEND OF PEACH</h1>
<h3 align="center">A Mario to the Mushroom Kingdom — v0.5.0 Super Mario Revolution</h3>

<p align="center">
  <strong>Two legendary SNES worlds. One genuine Super Mario action-adventure built on native 65c816 hardware.</strong>
</p>

<p align="center">
  <a href="https://github.com/pomercier/The-Legend-of-Peach/actions/workflows/ci.yml">
    <img src="https://github.com/pomercier/The-Legend-of-Peach/actions/workflows/ci.yml/badge.svg" alt="CI Status" />
  </a>
  <a href="https://github.com/pomercier/The-Legend-of-Peach/releases">
    <img src="https://img.shields.io/github/v/release/pomercier/The-Legend-of-Peach?color=e52521&label=Release" alt="Latest Release" />
  </a>
  <img src="https://img.shields.io/badge/Platform-SNES%20%2F%202%20MiB%20LoROM-5b459e" alt="Platform: SNES 2 MiB LoROM" />
  <img src="https://img.shields.io/badge/Engine-Pure%20Rust%20%2B%2065c816-dea584?logo=rust&logoColor=white" alt="Built with Rust + 65c816" />
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/License-MIT%20(Code)-2ea44f" alt="License: MIT" />
  </a>
  <img src="https://img.shields.io/badge/ROMs%20Included-Zero-critical" alt="Zero ROMs Included" />
</p>

---

**The Legend of Peach: A Mario to the Mushroom Kingdom** transforms *The Legend of Zelda: A Link to the Past* into a genuine top-down **Super Mario** SNES action-adventure — complete with native 65c816 **Mario Jumping**, **Mario Stomp Combat**, **Super Mario Hammer**, **Fire Mario live palette transformation**, **Super Starman rainbow invincibility**, **native SPC700 Mario Sound Effects**, **19 native N-SPC Mario musical tracks**, redesigned **World 1 Yoshi's Island** Map32 geography, and all **12 Super Mario World bosses**.

Powered by a pure, headless **Rust** binary transformation engine, the project reads your own legally obtained original backups of *A Link to the Past* and *Super Mario World*, applies **220 verified SNES hardware patches** across **18 rules** (`49` audited compressed graphics sheets, custom 65c816 gameplay assembly in Bank `$22`, Map32 world screens in Bank `$3B`, rewritten zero-hyphen 372-message French narrative, 19 native N-SPC musical themes across 25 contexts, and native SPC700 Mario SFX), and outputs a **bit-for-bit deterministic 2 MiB SNES LoROM cartridge image**.

> **Zero Copyrighted ROMs Included:** This repository contains **only original Rust source code, tooling, and documentation**. You must provide your own legal personal backups of both required SNES games to build the crossover locally. See [Required Original Games](#required-original-games).

---

## Gameplay Showcase (`v0.5.0 — Super Mario Revolution`)

All screenshots below are **100% authentic, unretouched live captures** from the locally built `v0.5.0` ROM (`SHA-256: 93d3774f...`) running in **Mednafen (`snes_faust` and cycle-accurate `bsnes`)**, integer-scaled `3×` nearest-neighbor (`768×672`) directly from native `256×224` SNES video output.

<table>
  <tr>
    <td align="center" width="50%">
      <img src="docs/media/screenshots/01_title_screen.png" alt="Title Screen captured in cycle-accurate bsnes" width="100%" /><br />
      <sub><strong>Custom Title Screen &amp; 3D Mario Trio (bsnes)</strong> — 32-frame 3D Super Mushroom, Fire Flower &amp; Super Star relics DMA-streamed behind the metallic <em>PEACH</em> logo.</sub>
    </td>
    <td align="center" width="50%">
      <img src="docs/media/screenshots/14_file_select_mario.png" alt="File Select showing 1. MARIO, Golden Super Star cursor, Mario Hammer, and SMW Royal Sky Blue backdrop" width="100%" /><br />
      <sub><strong>Redesigned Mario File Select</strong> — Crisp <code>1. MARIO</code>, blazing 2-frame Golden SMW Super Star cursor, Super Mario Hammer, 10 Mushroom lives, and SMW Royal Sky Blue backdrop.</sub>
    </td>
  </tr>
  <tr>
    <td align="center" width="50%">
      <img src="docs/media/screenshots/02_overworld_mario.png" alt="Fire Mario throwing a Fireball on redesigned World 1 Yoshi's Island" width="100%" /><br />
      <sub><strong>World 1 Yoshi's Island &amp; Fire Mario</strong> — Redesigned Map32 courtyard outside Mario's Mushroom House with live White/Red Fire Mario CGRAM transformation launching Fireballs.</sub>
    </td>
    <td align="center" width="50%">
      <img src="docs/media/screenshots/16_mario_jump_stomp.png" alt="Mario Jumping above his dark ground shadow and Stomping a Blue Koopa Troopa" width="100%" /><br />
      <sub><strong>Native 65c816 Mario Jump &amp; Stomp Combat</strong> — Press <code>L</code> or <code>R</code> to leap above Mario's ground shadow and stomp enemies for a bounce, defeat puff, and <code>+1</code> Gold Coin.</sub>
    </td>
  </tr>
  <tr>
    <td align="center" width="50%">
      <img src="docs/media/screenshots/17_mario_starman.png" alt="Super Starman Rainbow Invincibility active on Yoshi's Island" width="100%" /><br />
      <sub><strong>Super Starman Rainbow Invincibility (<code>L + R</code>)</strong> — 4-palette flashing rainbow CGRAM cycle with instant contact-kill across both regular and spiked enemies.</sub>
    </td>
    <td align="center" width="50%">
      <img src="docs/media/screenshots/11_start_menu_yoshis.png" alt="Start Menu with Fire/Ice Flowers, Green Warp Pipe, Super Mario Hammer, and 7 Spotted Yoshi Eggs" width="100%" /><br />
      <sub><strong>Mario Start Inventory Menu</strong> — Red Fire Flower, Ice Blue Flower, Glossy SMW Green Warp Pipe (<code>TUYAU WARP</code>), Super Mario Hammer, Emerald Koopa Shell, and <strong>7 Spotted Yoshi Eggs</strong> (<code>YOSHIS</code>).</sub>
    </td>
  </tr>
  <tr>
    <td align="center" width="50%">
      <img src="docs/media/screenshots/03_magikoopa_shop.png" alt="Kamek's Shop with Royal Crimson/Gold carpet, crisp 150/10/50 digits, Super Mushroom, Mini-Mushroom, and 10 Jet-Black Bob-ombs" width="100%" /><br />
      <sub><strong>Kamek's Toad Town Shop</strong> — Royal Mushroom Crimson &amp; Gold carpet, crisp <code>150 / 10 / 50</code> coin prices, Super Mushroom, Mini-Mushroom, and 10 Jet-Black SMW Bob-ombs.</sub>
    </td>
    <td align="center" width="50%">
      <img src="docs/media/screenshots/15_toad_town_crimson.png" alt="Toad Town House with rebuilt OAM-aligned Toad sprite on Crimson and Gold Mario carpet" width="100%" /><br />
      <sub><strong>Toad Town Interiors</strong> — Pixel-exact OAM-aligned Nintendo Toad NPCs on Royal Crimson &amp; Gold carpets.</sub>
    </td>
  </tr>
  <tr>
    <td align="center" width="50%">
      <img src="docs/media/screenshots/19_bowser_french_dialogue.png" alt="Bowser Volcanic Throne Room with crisp zero-overlap French dialogue in bsnes" width="100%" /><br />
      <sub><strong>Volcanic Bowser Throne &amp; Rewritten FR Script (bsnes)</strong> — Zero text overlap and zero hyphenated line breaks across all 372 French messages.</sub>
    </td>
    <td align="center" width="50%">
      <img src="docs/media/screenshots/09_boss_king_bowser.png" alt="Final Boss Giant King Bowser in Obsidian Volcanic Throne Room in bsnes" width="100%" /><br />
      <sub><strong>Final Showdown — Giant King Bowser (bsnes)</strong> — Obsidian volcanic stone throne room, 4-sheet Giant King Bowser, exclusive BGR555 palettes, and dedicated Final Bowser N-SPC theme.</sub>
    </td>
  </tr>
</table>

---

## Key Features

### Native 65c816 Super Mario Gameplay Engine (`mario_engine.rs`, Bank `$22:9000`)
- **Genuine Top-Down Mario Jumping (`L` or `R`):** Full 8.8 fixed-point parabolic jump physics with variable height (short-hop or ~22px full arc), crisp dark oval ground shadow underneath Mario, 8-direction air control, airborne hazard clearance, and automatic safe reset on room/mode transitions.
- **Genuine Mario Stomp Combat (`$22:9800` Vulnerability Table):** Descending onto stompable enemies (Koopas, Goombas, Buzzies, Rex, Monty Moles, Dry Bones, Cheep Cheeps) deals stomp damage or triggers a classic defeat puff, awards **+1 Gold Coin**, grants post-stomp i-frames, plays the Mario Stomp pop SFX, and bounces Mario upward (extra height when holding `L`/`R`). Spiked/armored enemies (Spinies, Thwomps, Chain Chomps) safely resist ordinary stomps.
- **Super Mario Hammer (`B`):** Replaces melee sword swings, File Select sword, and Start Menu `EQUIPEMENT` weapons with a glossy black-iron & wooden-handle Super Mario Hammer.
- **Unified Power-Up State Machine (`$22:9900` Live CGRAM Palettes):** Equipping `FLEUR DE FEU` live-swaps Mario into pure white-cap, blazing crimson-overalls **Fire Mario**; pressing `L + R` consumes Star Power to activate 4-palette flashing rainbow **Super Starman** invincibility with instant contact-kill.

### World, Cast, Bosses & French Narrative Overhaul
- **Structured Rust Map32 Overworld Editor (`mario_map.rs`):** Lossless Big-Endian LC_LZ2 JSON editor verified across all 64 Light World screens, featuring redesigned **World 1 — Yoshi's Island (`Screen 0x2C`)** relocated cleanly into LoROM Bank `$3B` ([`docs/maps/world1_yoshis_island.json`](docs/maps/world1_yoshis_island.json)).
- **Complete 100% Super Mario World Boss Roster (All 12 Bosses):** 6 Giant Chargin' Chucks, 3 Giant Pokeys, Giant Wiggler, Castle Tower King Bowser, all 7 Koopalings (Morton Jr., Wendy O., rebuilt Winged Iggy, Roy, Lemmy, Larry, Ludwig), and 4-sheet Final Boss **Giant King Bowser** in an Obsidian Volcanic Throne Room.
- **Zero-Overlap, Zero-Hyphen French Story (`mario_intro.rs`):** Live-patched Bank `$0C` compression dictionary plus deterministic single-emission Bank `$1C` 372-message repacker (verified by unit tests: `0` duplicate line breaks, `0` hyphenated word splits) with original playful Mario story rewrites.
- **De-Zeldafication Tracking:** Measured **Zelda Residue Index** reduced from `94.9` (`v0.4.0`) to **`19.2`** (`v0.5.0`, **−79.8%**), raising the **Mario Identity Index** to **`80.8`** ([`docs/ZELDA_RESIDUE_REGISTRY.json`](docs/ZELDA_RESIDUE_REGISTRY.json)).

### Native SNES Hardware Audio (19 N-SPC Tracks + Native Mario SFX — No MSU-1)
- **19 Native SPC700 N-SPC Musical Tracks across 25 Contexts (`mario_music.rs`):** SMW Title Calypso, Overworld, Athletic, Forest of Illusion, Vanilla Dome, Star Road Waltz, Valley of Bowser, Bonus Room / Yoshi's House, Underwater Waltz, Ghost House, Castle, SMB 1-1 / 1-2 Underground, SMB3 Airship March, Koopalings Boss, Final King Bowser Battle, Peach Sanctuary, Starman, and Course Clear Fanfare.
- **Native SPC700 Mario Sound Effects (`mario_sfx.rs`):** In-place Bank `$19` (`ARAM $17C0..$284F`) patches for Mario Coin chime (`B5 -> E6`), SMW Jump sweep, Mario Stomp pop, descending triple-pulse Warp Pipe, and Super Mario Hammer bonk.

### Pure Rust Reproducible Build Pipeline
- **Strict SHA-256 Input Enforcement:** Strips optional 512-byte SMC copier headers in memory and verifies exact No-Intro input ROM hashes before touching a byte.
- **100% Deterministic 2 MiB LoROM Output:** Builds in ~1 second, enforces compile-time memory firewalls (`$20..$22` Intro + Engine, `$23..$3A` Relocated GFX, `$3B..$3F` Map32), audits all `49` modified compressed sheets for single-bank alignment and lossless LC_LZ2 round-trips, and validates SNES checksum/complement (`0x87FF` / `0x7800`).

---

## Getting Started

### Prerequisites

- **Rust (stable, 1.88+)** — [rustup.rs](https://rustup.rs/)
- **Your own legal backups** of the two required original SNES ROMs (see [Required Original Games](#required-original-games))
- **An accurate SNES emulator** (Mednafen `snes_faust` / `snes`, bsnes, Ares, or RetroArch)

### 1. Clone & Verify Input ROMs

```bash
git clone https://github.com/pomercier/The-Legend-of-Peach.git
cd The-Legend-of-Peach
./scripts/build.sh --verify-only /path/to/zelda_fr.sfc /path/to/smw_eu.sfc
```

### 2. Build the Playable SNES ROM

**Linux / macOS:**
```bash
./scripts/build.sh /path/to/zelda_fr.sfc /path/to/smw_eu.sfc build/legend_of_peach.sfc
```

**Windows (PowerShell) or direct Cargo:**
```powershell
cargo run --release --bin snes-cli -- --build-peach `
  --zelda C:\path\to\zelda_fr.sfc `
  --smw C:\path\to\smw_eu.sfc `
  --out build\legend_of_peach.sfc
```

Your original ROM files are read-only and **never modified**. See [`docs/BUILDING.md`](docs/BUILDING.md) for full details.

### 3. Launch in an Emulator

```bash
mednafen -force_module snes_faust build/legend_of_peach.sfc
```

---

## Required Original Games

Both original ROMs below are verified locally against **No-Intro** SNES catalog digests. Optional 512-byte `.smc` copier headers are stripped automatically in memory before verification.

| Role | Game | Region / Rev | Unheadered Size | Verified SHA-256 (Unheadered) |
| :--- | :--- | :--- | ---: | :--- |
| **Base ROM** | *The Legend of Zelda: A Link to the Past* (`Legend of Zelda, The - A Link to the Past (France).sfc`) | France (PAL), Rev 1.0 | `1,048,576` B (1 MiB) | `29a0d5812ccbb4b11bdb55d8f751a2a797b4110bf402ca8ba15eb2bf85db7b39` |
| **Donor ROM** | *Super Mario World* (`Super Mario World (Europe).sfc`) | Europe (PAL), Rev 1.0 | `524,288` B (512 KiB) | `b5be1dba3012b6811a5660fbf2981cb23cdd1e48f845a42df00f0f55b19f0392` |

> Full verification details (expected `v0.5.0` output SHA-256 `93d3774f81119eb41d80b0c610d3b1bbcd1430c5f572f16b65b47b3ddd24975e`, Checksum `0x87FF` / `0x7800`, CRC32 `9E8F84EB`) are documented in [`docs/ROM_REQUIREMENTS.md`](docs/ROM_REQUIREMENTS.md).

---

## How It Works

```text
  [Zelda ALTTP (FR PAL)] ──┐
                           ├──► SHA-256 Verifier ──► Planar Tile & Palette Transcoder
  [Super Mario World (EU)] ┘        (rom.rs)          + LC_LZ2 Compressor (49 audited sheets)
                                                      + 65c816 Mario Engine (Jump/Stomp/Fire/Star)
                                                      + Overworld Map32 JSON Engine (Bank $3B)
                                                      + 372-Msg FR Script (0 overlap, 0 hyphens)
                                                      + Native SPC700 Music (19) & Mario SFX
                                                                   │
                                                                   ▼
                                                      LoROM 2 MiB Allocator ($00..$3F)
                                                      + Hardware Firewall & Integrity Audit
                                                      + SNES Checksum (0x87FF / 0x7800)
                                                                   │
                                                                   ▼
                                                   [build/legend_of_peach.sfc] (Local)
```

Read the full technical deep-dive in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md), [`docs/ROM_MEMORY.md`](docs/ROM_MEMORY.md), and [`docs/VISUAL_REGRESSION_REPORT.md`](docs/VISUAL_REGRESSION_REPORT.md).

---

## More Screenshots — Bosses & Worlds

<table>
  <tr>
    <td align="center" width="50%">
      <img src="docs/media/screenshots/18_boss_iggy_koopa.png" alt="Rebuilt Winged Iggy Koopa boss in Skull Woods" width="100%" /><br />
      <sub><strong>Koopaling #3 — Winged Iggy Koopa (Rebuilt)</strong> — Crisp OAM-aligned Iggy Koopa with green tuft, spiral glasses, and clean wings.</sub>
    </td>
    <td align="center" width="50%">
      <img src="docs/media/screenshots/12_boss_chargin_chucks.png" alt="Eastern Palace Boss — 6 Giant 32x32 Hopping SMW Chargin' Chucks" width="100%" /><br />
      <sub><strong>Eastern Palace Boss — 6 Chargin' Chucks</strong> — Seamless 32×32 hopping SMW Chargin' Chucks turning enraged blazing red on the final survivor.</sub>
    </td>
  </tr>
  <tr>
    <td align="center" width="50%">
      <img src="docs/media/screenshots/06_boss_morton_koopa.png" alt="Koopaling #1 Morton Koopa Jr." width="100%" /><br />
      <sub><strong>Koopaling #1 — Morton Koopa Jr.</strong> — Iron spiked visor, spiked shell, and flaming mace tail in the Palace of Darkness.</sub>
    </td>
    <td align="center" width="50%">
      <img src="docs/media/screenshots/07_boss_lemmy_circus.png" alt="Koopaling #5 Lemmy Koopa in Ice Palace" width="100%" /><br />
      <sub><strong>Koopaling #5 — Lemmy Koopa</strong> — Rainbow Mohawk and bouncing circus ball in the Ice Palace.</sub>
    </td>
  </tr>
  <tr>
    <td align="center" width="50%">
      <img src="docs/media/screenshots/10_boss_ludwig_trinity.png" alt="Koopaling #7 Ludwig von Koopa" width="100%" /><br />
      <sub><strong>Koopaling #7 — Ludwig von Koopa</strong> — Turtle Rock trinity with Dino Torch (fire) and Frost Koopa (ice) heads.</sub>
    </td>
    <td align="center" width="50%">
      <img src="docs/media/screenshots/05_dark_world_pyramid.png" alt="Dark World Mushroom Pyramid with Carved Stone King Bowser Statues" width="100%" /><br />
      <sub><strong>Valley of Bowser Pyramid</strong> — Carved Stone King Bowser Statues and Grimacing Thwomp Pillars paired with the <em>SMW Valley of Bowser</em> theme.</sub>
    </td>
  </tr>
</table>

---

## Compatibility & SNES Hardware Status

| Target | Status | Notes |
| :--- | :--- | :--- |
| **Mednafen (`snes_faust`)** | **Verified** | Live-tested across File Select, Start Menu, World 1 Yoshi's Island, Mario Jump (`L`/`R`), Mario Stomp Combat (`+1` Coin + bounce), Fire Mario, Starman (`L+R`), Kamek's Shop, Toad Town, Iggy Koopa, and Final King Bowser. |
| **Mednafen (`snes` / bsnes-accuracy)** | **Verified** | Cycle-accurate PPU/DMA/65c816/SPC700 verification of the 3D Title Screen, File Select, Yoshi's Island Overworld, Start Menu, Fire Mario, Starman, and Final Bowser Volcanic Throne Room + dialogue. |
| **bsnes / Ares / RetroArch (`bsnes`, `snes9x`)** | **Expected Compatible** | Standard 2 MiB LoROM (`0x20`, PAL 50 Hz, no enhancement chips, no MSU-1). |
| **Real SNES / Super Famicom / FXPAK Pro** | **Architecturally Compliant (Untested on Physical Hardware)** | Strictly respects LoROM 32 KiB single-bank boundaries, valid checksum/complement (`0x87FF` / `0x7800`), OAM/VRAM/CGRAM/SPC700 limits, and clean DMA timing. Community physical-hardware test reports are welcome! |

---

## Roadmap (`v0.6.0+`)

See [`docs/NEXT_ACTIONS.md`](docs/NEXT_ACTIONS.md) for the complete backlog:

- [ ] **Multi-World Map32 Redesign & Warp Pipe Entrances:** Extend `mario_map.rs` to Toad Town, Yoshi's Meadow, and Valley of Bowser with visual Overworld Warp Pipes.
- [ ] **Koopa Guard Weapon Overhaul & Title Screen BG2:** Replace Koopa guard blades with Hammer Bro hammers and remove the Title Screen Master Sword BG2 silhouette.
- [ ] **Custom Boss AI & Rideable Yoshi Mount:** Expand Bank `$22` (`$22:C000..$22:FFFF`) with dedicated Koopaling/Bowser AI patterns and a rideable Yoshi mount (`$7E:F510..$7E:F51F`).
- [ ] **Multi-Region & Physical Hardware Testing:** Add verified USA (NTSC 60 Hz) offset profiles and test on real SNES / FXPAK Pro consoles.

---

## Credits & Legal Notice

- **Author & Lead Developer:** [`pomercier`](https://github.com/pomercier)
- **SNES Reverse-Engineering References:** [`SMWDisX`](https://github.com/IsoFrieze/SMWDisX), [`jpdasm`](https://github.com/spannerisms/jpdasm), No-Intro SNES catalog

**The Legend of Peach: A Mario to the Mushroom Kingdom** is an unofficial, non-commercial fan-made project created for educational, reverse-engineering, and retro-gaming preservation purposes. All Nintendo trademarks, characters, artwork, and original game assets belong solely to **Nintendo Co., Ltd.** Original Rust source code, scripts, and documentation in this repository are licensed under the [MIT License](LICENSE). **Zero copyrighted ROM files or proprietary binary patches are hosted or distributed here.**
