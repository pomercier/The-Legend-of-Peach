# Building The Legend of Peach

This guide walks through building **The Legend of Peach: A Mario to the Mushroom Kingdom** locally from source using your own legally obtained original SNES ROM backups.

The entire build pipeline is written in pure headless Rust, compiles cleanly on Linux, macOS, and Windows, runs in under one second once compiled, and produces a **bit-for-bit deterministic** 2 MiB SNES LoROM image.

---

## Prerequisites

1. **Git** — to clone the repository.
2. **Rust toolchain (stable, 1.88 or newer)** — install via [rustup.rs](https://rustup.rs/).
   ```bash
   rustc --version
   cargo --version
   ```
3. **Your own legal backups of both required SNES ROMs** — see [`docs/ROM_REQUIREMENTS.md`](ROM_REQUIREMENTS.md) for the exact regions, revisions, and SHA-256 hashes:
   - `Legend of Zelda, The - A Link to the Past (France).sfc` (PAL, Rev 1.0)
   - `Super Mario World (Europe).sfc` (PAL, Rev 1.0)
4. **An accurate SNES emulator** (for example, Mednafen `snes_faust` / `snes`, bsnes, Ares, or RetroArch with a `bsnes` / `snes9x` core).

---

## Step 1: Clone the Repository

```bash
git clone https://github.com/pomercier/The-Legend-of-Peach.git
cd The-Legend-of-Peach
```

---

## Step 2: Verify Your Input ROMs

You can place your ROMs anywhere on your disk, or drop copies into the git-ignored `roms/` directory:

- `roms/zelda_alttp.sfc`
- `roms/super_mario_world.sfc`

Verify their SHA-256 hashes before building (no output file is written):

**Linux / macOS:**
```bash
./scripts/build.sh --verify-only /path/to/zelda_fr.sfc /path/to/smw_eu.sfc
```

**Windows (PowerShell) or direct Cargo invocation:**
```powershell
cargo run --release --bin snes-cli -- --build-peach `
  --zelda C:\path\to\zelda_fr.sfc `
  --smw C:\path\to\smw_eu.sfc `
  --verify-only
```

If either ROM does not match the verified No-Intro SHA-256 digest, the tool exits with a clear diagnostic error instead of producing a broken image.

---

## Step 3: Build the Playable ROM

### Using the Helper Script (Linux / macOS)

```bash
./scripts/build.sh /path/to/zelda_fr.sfc /path/to/smw_eu.sfc build/legend_of_peach.sfc
```

Or, if your files are placed at `roms/zelda_alttp.sfc` and `roms/super_mario_world.sfc`:

```bash
./scripts/build.sh
```

### Using Cargo / `snes-cli` Directly (All Platforms)

```bash
cargo build --release --bin snes-cli

./target/release/snes-cli --build-peach \
  --zelda /path/to/zelda_fr.sfc \
  --smw /path/to/smw_eu.sfc \
  --out build/legend_of_peach.sfc
```

Your original input ROM files are **never modified**.

---

## Step 4: Verify Output Integrity & Determinism

After building, `snes-cli` automatically runs the internal hardware integrity audit (`audit_legend_of_peach_output`) verifying:

- Exact 2 MiB LoROM expansion (`2,097,152` bytes, ROM size header byte `0x0B`)
- Valid SNES checksum (`0x7C48`) and complement (`0x83B7`)
- Zero patch overlaps across expanded banks `$23..$3F`
- Single-bank 32 KiB alignment and full LC_LZ2 decompression integrity across all 48 modified graphics sheets
- Protected world-map table integrity (`0x08A000..0x08A2FF`) and 32-frame 3D intro buffer (`$20..$22`)

Expected output summary for `v0.4.0`:

```text
Output ROM      : build/legend_of_peach_mario_kingdom.sfc (2097152 bytes, 2 MiB LoROM)
Internal Title  : 'THE LEGEND OF PEACH'
Hardware Ops    : 193 patches across 18 rules
Bank Audit      : 48 sheets verified, 9 relocated in $23..$3F (0 overlaps)
SNES Checksum   : 0x7C48 (Complement: 0x83B7, Valid: true)
Output CRC32    : 0x397E3E64
Output SHA-256  : eed0aac268c6523657ca083cc8cd40e620516757801bd69bbff38474f8b190ca
```

You can also confirm bit-for-bit determinism yourself by running the build twice into two paths and comparing `sha256sum`.

---

## Step 5: Play!

Launch the generated `build/legend_of_peach.sfc` in your SNES emulator of choice:

```bash
# Mednafen (snes_faust — fast & accurate)
mednafen -force_module snes_faust build/legend_of_peach.sfc

# Mednafen (bsnes-accuracy core)
mednafen -force_module snes build/legend_of_peach.sfc

# bsnes / Ares / RetroArch
# Open build/legend_of_peach.sfc via File -> Load ROM
```

---

## Running Automated Tests & Checks

The Rust test suite and repository safety audit run cleanly **without** any copyrighted ROM files present:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
python3 scripts/check_repo_safety.py
```

When verified local ROMs are present under `roms/`, additional end-to-end integration tests in `crates/snes-core/src/lib.rs` automatically exercise the full 177-patch fusion, OAM coordinate bounds, all 3bpp/4bpp LZ2 sheets, and hardware integrity checks.
