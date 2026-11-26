# Technical Architecture (`v0.5.0`)

**The Legend of Peach: A Mario to the Mushroom Kingdom** is built by a pure, headless Rust SNES binary transformation engine (`crates/snes-core`) with zero GUI runtime dependencies.

Instead of shipping opaque pre-baked binary patches containing copyrighted game tiles, the repository ships **clean-room Rust algorithms** that read two user-supplied original SNES ROMs, verify their cryptographic identities, transcode and assemble sprites, tilesets, palettes, native 65c816 gameplay assembly, Overworld Map32 screens, French dialogue, and SPC700 N-SPC music & SFX on the fly, and output a valid, bit-for-bit deterministic 2 MiB LoROM SNES cartridge image.

---

## Workspace Layout

```text
The-Legend-of-Peach/
├── Cargo.toml                     # Rust workspace manifest (v0.5.0)
├── crates/snes-core/              # Core SNES engine & CLI binary
│   └── src/
│       ├── rom.rs                 # LoROM/HiROM parser, copier header stripping, SHA-256 & strict input verification, SNES checksum
│       ├── graphics.rs            # 2bpp/3bpp/4bpp SNES planar tile codecs & BGR555 CGRAM palette engine
│       ├── audio.rs               # BRR sample decoding/encoding & SPC700 scanner
│       ├── disasm_dma.rs          # Nintendo LC_LZ2 compressor/decompressor, DMA scanner, 65c816 helpers
│       ├── swapper.rs             # Byte-accurate patch session tracking & rollback-safe patch log
│       ├── mixer.rs               # 18-rule canonical fusion orchestrator, bank allocator & post-build hardware audit (49 sheets)
│       ├── mario_engine.rs        # Phase 5 native 65c816 Mario Gameplay Engine ($22:9000): Jump (L/R), Stomp Combat, Fire Mario & Starman
│       ├── mario_sfx.rs           # Phase 5 native SPC700 Mario SFX compiler (Bank $19: Coin, Jump, Stomp, Warp Pipe, Hammer)
│       ├── mario_map.rs           # Phase 5 Overworld Map32 Big-Endian LC_LZ2 JSON editor & World 1 Yoshi's Island redesign (Bank $3B)
│       ├── mario_intro.rs         # 32-frame 3D Mario Trio intro, giant PEACH logo, Bank $0C FR dictionary & zero-hyphen 372-msg Bank $1C engine
│       ├── mario_ui.rs            # File Select (Golden Star cursor, Hammer), Start Menu (Fire/Ice Flowers, Green Warp Pipe, 7 Yoshi Eggs), HUD & items
│       ├── mario_world.rs         # OAM-aligned Toad Town NPCs, 16 SMW bestiary sheets, 9 worldwide BG tilesets (Bowser Statues, ON/OFF blocks)
│       ├── mario_bosses.rs        # All 7 Koopalings (incl. rebuilt Winged Iggy), Final King Bowser (4 sheets + 2 palettes), Pokeys, Wiggler
│       ├── mario_music.rs         # Native ALTTP N-SPC ($D000) music compiler (19 Mario themes across 25 contexts)
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
└── docs/                          # Persistent registries, maps JSON, architecture docs & curated 3x media
```

---

## 2 MiB LoROM Memory Map & Hardware Firewalls

The original French PAL release of *The Legend of Zelda: A Link to the Past* is a **1 MiB LoROM** image occupying banks `$00..$1F` (`PC 0x000000..0x0FFFFF`). During the build, `snes-core` expands the cartridge to a standard **2 MiB LoROM** image (`PC 0x000000..0x1FFFFF`, header ROM size byte `0x0B` at `0x7FD7`) with strict compile-time memory firewalls and single-bank 32 KiB alignment:

| LoROM Banks | PC File Range | Purpose |
| :--- | :--- | :--- |
| `$00..$1F` | `0x000000..0x0FFFFF` | Original ALTTP code, palettes, Bank `$02` engine hooks, Bank `$0C` FR dictionary, Bank `$19` SFX/music, Bank `$1C` 372-msg script |
| `0x08A000..0x08A2FF` | *(Inside Bank `$11`)* | **Protected zone** — ALTTP world-map pointer tables (verified untouched byte-for-byte) |
| `$20..$21` | `0x100000..0x10FFFF` | Pre-rendered 32-frame 4bpp 3D Mario Trio intro tile buffers (2 KiB per frame, DMA-streamed in NMI) |
| `$22` (`$8000..$8FFF`) | `0x110000..0x110FFF` | Custom 65c816 intro DMA/OAM stepper (`$22:8000`, `$22:8080`) & OBJ Pal 6 (`$22:8100`) |
| `$22` (`$9000..$BFFF`) | `0x111000..0x113FFF` | **Native 65c816 Super Mario Gameplay Engine** (`Mario_MainHook`, Jump/Stomp/Fire/Starman, `$22:9800` Stomp Table, `$22:9900` Live Palettes) |
| `$22` (`$C000..$FFFF`) | `0x114000..0x117FFF` | Reserved for future `v0.6.0+` 65c816 gameplay extensions (Yoshi Mount, Boss AI) |
| `$23..$3A` | `0x118000..0x1D7FFF` | Relocated LC_LZ2 compressed graphics sheets (capped by `EXPANDED_FREE_END = 0x1D8000`, zero overlaps) |
| `$3B..$3F` | `0x1D8000..0x1FFFFF` | Relocated Big-Endian LC_LZ2 Overworld Map32 screens (World 1 Yoshi's Island & future worlds) |

For exact verified FR PAL offsets and WRAM map (`$7E:F500..$7E:F53F`), see [`docs/ROM_MEMORY.md`](ROM_MEMORY.md), [`docs/WRAM_ALLOCATION.json`](WRAM_ALLOCATION.json), and [`docs/PATCH_DEPENDENCIES.json`](PATCH_DEPENDENCIES.json).

---

## End-to-End Transformation Pipeline

1. **Input Parsing & Strict SHA-256 Verification (`rom.rs`)** — Strips optional 512-byte SMC copier headers in memory and enforces exact No-Intro SHA-256 hashes.
2. **Graphics Extraction & Planar Transcoding (`graphics.rs`, `disasm_dma.rs`, `mario_*.rs`)** — Transcodes SMW 4bpp/3bpp graphics and custom pixel art into ALTTP VRAM/OAM layouts across **49 audited compressed sheets** (`11` relocated cleanly inside `$23..$3A`).
3. **Native 65c816 Gameplay Engine (`mario_engine.rs`)** — Hooks all 3 Bank `$02` callers of `JSL $07:8000` (`0x010959`, `0x011CCA`, `0x0125C7`) -> `JSL $22:9000`, enabling parabolic Mario Jumping (`L`/`R`), Stomp Combat with bounce/+1 Coin, Fire Mario live CGRAM swap, and Super Starman rainbow invincibility (`L+R`).
4. **Overworld Map32 Engine (`mario_map.rs`)** — Decompresses, edits, and losslessly recompresses Big-Endian LC_LZ2 Map32 screens into Bank `$3B..$3F` and updates 24-bit pointers at `0x0179CF` / `0x017BAF`.
5. **French Dialogue & Dictionary Engine (`mario_intro.rs`)** — Patches Bank `$0C` dictionary in-place and repacks all 372 Bank `$1C` messages with deterministic single-emission line breaks (0 overlaps, 0 hyphenated word breaks).
6. **Native SPC700 Music & SFX (`mario_music.rs`, `mario_sfx.rs`)** — Compiles **19 native N-SPC musical tracks** across 25 contexts and patches **Bank `$19` resident SFX** (Coin, Jump, Stomp, Warp Pipe, Hammer bonk).
7. **SNES Header Checksum & Post-Build Hardware Audit (`rom.rs`, `mixer.rs`)** — Sets internal title (`THE LEGEND OF PEACH`), 2 MiB size byte (`0x0B`), valid checksum (`0x87FF` / `0x7800`), and verifies zero overlaps, single-bank alignment, lossless LC_LZ2 decompression across all 49 sheets, and deterministic SHA-256 (`93d3774f81119eb41d80b0c610d3b1bbcd1430c5f572f16b65b47b3ddd24975e`).

---

## Disassembly References

- **[SMWDisX](https://github.com/IsoFrieze/SMWDisX)** — *Super Mario World* disassembly reference
- **[jpdasm](https://github.com/spannerisms/jpdasm)** — *The Legend of Zelda: A Link to the Past* disassembly reference

No copyrighted disassembly assets or ROM dumps are bundled in this repository.
