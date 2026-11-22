# Changelog

All notable changes to **The Legend of Peach: A Mario to the Mushroom Kingdom** are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-11-22 — First Public Release

### Highlights
- **First public open-source release** of the pure headless Rust SNES ROM transformation engine (`snes-core` / `snes-cli`) that deterministically builds **The Legend of Peach: A Mario to the Mushroom Kingdom** from user-provided legal backups of *The Legend of Zelda: A Link to the Past (France, PAL)* and *Super Mario World (Europe, PAL)*.

### Gameplay, Visuals & Audio Implemented
- **Custom 3D Intro & Title Screen (`mario_intro.rs`)**:
  - 32-frame 4bpp rotating 3D Super Mushroom, Fire Flower, and Super Star relics DMA-streamed via custom 65c816 routines in LoROM banks `$20..$22`.
  - Giant crimson metallic **`THE LEGEND OF PEACH: A MARIO TO THE MUSHROOM KINGDOM`** title logo, rebuilt copyright notice, and French story rewrite.
- **Playable Hero, Allies & Toad Town (`mario_ui.rs`, `mario_world.rs`)**:
  - Full playable **Mario** sprite animations (walking, sword slash, Green Koopa Shell shield, spin attack, block/bush carry with fixed OAM carry-height offset).
  - **Princess Peach**, **Uncle Luigi** (rebuilt from Mario proportions in green), **Toad**, **Yoshi**, and **Kamek (Magikoopa)** shopkeeper selling Super Mushrooms, Yoshi Berries, and Black Bob-ombs.
  - Mario-themed HUD (Gold Coins, Black Bob-ombs, Bullet Bills, Bill Blaster, Super Mushroom life containers) and Start inventory relics.
- **Full Bestiary & Worldwide Tilesets (`mario_world.rs`)**:
  - 16 SMW enemy sheets including Koopa Troopas (with corrected weapon placement), Goombas, Spinies, Rex, Spike Tops, Piranha Plants, Boos, Dry Bones, Thwomps, and Cheep Cheeps.
  - 9 transformed overworld/dungeon background tilesets: Mario's red polka-dot Mushroom Roof house (`bg:4D`), SMW smiling bushes, Turn-Blocks, Question Blocks, SMW Castle Bricks (`bg:0D`), and carved Stone King Bowser Statues (`bg:18/19`).
- **Complete Boss Roster (`mario_bosses.rs`)**:
  - All **7 Koopalings**: Morton Koopa Jr., Wendy O. Koopa, Winged Iggy Koopa, Roy Koopa, Lemmy Koopa (circus ball & rainbow Mohawk), Larry Koopa (+12 Mini-Spinies), and Ludwig von Koopa (+Dino Torch & Frost Koopa).
  - **Final Boss Giant King Bowser** (4 custom sprite sheets + 2 exclusive BGR555 palettes).
  - **3 Giant Pokeys** (Desert Palace) and **Giant Wiggler** (Tower of Hera).
- **Native SPC700 Soundtrack (`mario_music.rs`)**:
  - 10 Super Mario melodies compiled directly to ALTTP's native N-SPC (`$D000`) format — including Overworld, SMW Athletic, Ghost House, Underground, Koopalings/Bowser Boss themes, and Fanfares — no MSU-1 required.

### Build System, Safety & Verification
- **Strict SHA-256 Input Verification (`rom.rs`)**: Automatically strips optional 512-byte SMC copier headers and strictly verifies exact No-Intro SHA-256 hashes before building.
- **Bit-for-Bit Reproducible Builds (`--build-peach`, `scripts/build.sh`)**: Deterministic 2 MiB LoROM output (`SHA-256: 32540c4629c6b797f6933c7966a419fd959a3f711f36347f4348ceb2fa85b99c`, SNES Checksum `0x464D` / Complement `0xB9B2`) with automated post-build hardware audit (`audit_legend_of_peach_output`).
- **Emulator Validation**: Live-verified across 22 gameplay/boss/title captures in Mednafen (`snes_faust`) and cycle-accurate `bsnes` (`snes`).
- **ROM-Free CI & Repository Safety Gate**: Automated `rustfmt`, `clippy -D warnings`, 21/21 ROM-free unit tests, release build, and `scripts/check_repo_safety.py`.
