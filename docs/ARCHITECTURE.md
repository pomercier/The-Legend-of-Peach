# Technical Architecture

**The Legend of Peach: A Mario to the Mushroom Kingdom** is built by a pure, headless Rust SNES binary transformation engine (`crates/snes-core`) with zero GUI runtime dependencies.

Instead of shipping opaque pre-baked binary patches containing copyrighted game tiles, the repository ships **clean-room Rust algorithms** that read two user-supplied original SNES ROMs, verify their cryptographic identities, transcode and assemble sprites, tilesets, palettes, 65c816 assembly routines, and SPC700 N-SPC music on the fly, and output a valid 2 MiB LoROM SNES cartridge image.

---

## Workspace Layout

```text
The-Legend-of-Peach/
├── Cargo.toml                     # Rust workspace manifest
├── crates/snes-core/              # Core SNES engine & CLI binary
│   └── src/
│       ├── rom.rs                 # LoROM/HiROM parser, copier header stripping, SHA-256 & strict input verification, SNES checksum
│       ├── graphics.rs            # 2bpp/3bpp/4bpp SNES planar tile codecs & BGR555 CGRAM palette engine
│       ├── audio.rs               # BRR sample decoding/encoding & SPC700 scanner
│       ├── disasm_dma.rs          # Nintendo LC_LZ2 compressor/decompressor, DMA scanner, 65c816 helpers
│       ├── swapper.rs             # Byte-accurate patch session tracking & rollback-safe patch log
│       ├── mixer.rs               # 18-rule canonical fusion orchestrator, bank allocator & post-build hardware audit
│       ├── mario_intro.rs         # 32-frame 3D Mario Trio intro renderer, giant PEACH logo, 65c816 DMA/OAM hooks & story text
│       ├── mario_ui.rs            # Mario HUD (coins, Bob-ombs, Bullet Bills, Mushroom hearts), inventory, blocks, NPCs & items
│       ├── mario_world.rs         # Toad Town NPCs, 16 SMW bestiary sheets, 9 worldwide BG tilesets (Bowser Statues, bricks, roofs)
│       ├── mario_bosses.rs        # All 7 Koopalings, Final Boss Giant King Bowser (4 sheets + 2 palettes), Giant Pokeys, Giant Wiggler
│       ├── mario_music.rs         # Native ALTTP N-SPC ($D000) music compiler (10 Mario themes in SPC700 hardware format)
│       ├── universal.rs           # Generic cross-ROM catalog & experimental omni-mixer utilities
│       ├── deconstruct.rs         # Full ROM deconstruct / round-trip reconstruct pipeline
│       ├── export.rs              # Asset inspection & PNG/WAV export utilities
│       ├── assets.rs              # Hardware region & sprite scanner
│       ├── profile.rs             # SNES game profile heuristics
│       ├── memory.rs              # Local session reporting helpers
│       ├── demo.rs                # Synthetic ROM pair generator for ROM-free CI unit tests
│       └── bin/snes-cli.rs        # Command-line interface (--build-peach & SNES toolkit commands)
├── recipes/legend_of_peach.json   # Canonical 18-rule fusion recipe
├── scripts/
│   ├── build.sh                   # One-command reproducible local builder
│   └── check_repo_safety.py       # Automated zero-ROM & secret-leak repository safety auditor
└── docs/                          # Documentation & curated visual showcase
```

---

## 2 MiB LoROM Memory Map

The original French PAL release of *The Legend of Zelda: A Link to the Past* is a **1 MiB LoROM** image occupying banks `$00..$1F` (`PC 0x000000..0x0FFFFF`).

During the build, `snes-core` expands the cartridge to a **2 MiB LoROM** image (`PC 0x000000..0x1FFFFF`, internal header byte `0x0B` at `0x7FD7`) while preserving strict 32 KiB LoROM bank boundaries (`$8000..$FFFF` per bank):

| LoROM Banks | PC File Range | Purpose |
| :--- | :--- | :--- |
| `$00..$1F` | `0x000000..0x0FFFFF` | Original ALTTP code, maps, palettes, text, and in-place compressed patches |
| `0x08A000..0x08A2FF` | *(Inside Bank `$11`)* | **Protected zone** — ALTTP world-map pointer tables (verified untouched byte-for-byte) |
| `$20..$21` | `0x100000..0x10FFFF` | Pre-rendered 32-frame 4bpp 3D Mario Trio intro tile buffers (2 KiB per frame, DMA-streamed in NMI) |
| `$22` | `0x110000..0x117FFF` | Custom 65c816 intro DMA/OAM stepper (`$22:8000`, `$22:8080`) and BGR555 4bpp OBJ palette 6 (`$22:8100`) |
| `$23..$3F` | `0x118000..0x1FFFFF` | Relocated LC_LZ2 compressed graphics sheets (strictly aligned inside single 32 KiB banks, zero cross-bank splits, zero overlaps) |

---

## End-to-End Transformation Pipeline

1. **Input Parsing & Strict SHA-256 Verification (`rom.rs`)**
   - Strips optional 512-byte SMC copier headers in memory.
   - Computes FIPS 180-4 SHA-256 digests and verifies exact No-Intro sizes and hashes for both input ROMs.
2. **Graphics Extraction & Planar Transcoding (`graphics.rs`, `disasm_dma.rs`, `mario_*.rs`)**
   - Extracts raw 4bpp sprite tiles from *Super Mario World* and decompresses Nintendo **LC_LZ2** 3bpp graphics sheets from *A Link to the Past*.
   - Remaps 15-bit **BGR555** SNES palettes and transcodes 4bpp ↔ 3bpp planar bitplanes to fit ALTTP's exact PPU VRAM layout and OAM sub-sprite assembly without exceeding SNES OAM/VRAM budgets.
   - Recompresses every modified sheet with a byte-exact LC_LZ2 encoder. If a sheet grows beyond its original compressed slot, the bank allocator relocates it cleanly into `$23..$3F` (guaranteed within a single 32 KiB LoROM bank) and updates ALTTP's 24-bit pointer table (`$00:CF80` / `$00:D05F` / `$00:D13E`).
3. **Custom 65c816 Assembly & OAM Fixes (`mario_intro.rs`, `mario_ui.rs`)**
   - Hooks the ALTTP intro/title sequence to stream 32 frames of rotating 3D Mushroom, Fire Flower, and Super Star relics over DMA during VBlank, alongside the rebuilt giant `THE LEGEND OF PEACH` metallic title logo and story crawl.
   - Adjusts OAM carry-height offsets and Uncle/Luigi multi-tile assembly based on verified ALTTP disassembly structures so carried blocks sit cleanly above Mario's cap and Luigi renders cleanly with Mario's proportions in green.
4. **Native SPC700 N-SPC Soundtrack (`mario_music.rs`)**
   - Compiles 10 Super Mario melodies directly into ALTTP's native **N-SPC** sequence bytecode format (using ALTTP's existing `$D000` SPC700 sound driver and instrument bank — no external MSU-1 hardware required!).
   - Themes include Title / Overworld, SMW Athletic, Ghost House, Underground / Caves, Koopalings & Bowser Boss battles, and Victory Fanfares.
5. **SNES Header Checksum & Post-Build Hardware Audit (`rom.rs`, `mixer.rs`)**
   - Updates the 21-byte internal SNES title (`THE LEGEND OF PEACH`), sets LoROM 2 MiB size byte (`0x0B`), and recalculates the 16-bit SNES checksum (`0x464D`) and XOR complement (`0xB9B2`).
   - Runs `audit_legend_of_peach_output` on every build to guarantee zero patch overlaps, valid bank alignment, lossless LC_LZ2 round-trip across all 46 modified sheets, untouched safety regions, and deterministic output SHA-256.

---

## Disassembly References

Hardware structures, OAM layouts, LZ2 pointer tables, and sprite indices were studied using open-source SNES reverse-engineering references:

- **[SMWDisX](https://github.com/IsoFrieze/SMWDisX)** — *Super Mario World* disassembly (graphics sheets, animations, and official palettes)
- **[jpdasm](https://github.com/spannerisms/jpdasm)** — *The Legend of Zelda: A Link to the Past* disassembly (intro polyhedron routines, OAM assembly, carry offsets, and graphics tables)

No copyrighted disassembly assets or ROM dumps are bundled in this repository.
