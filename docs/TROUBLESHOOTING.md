# Troubleshooting

Common issues and how to fix them when building or playing **The Legend of Peach: A Mario to the Mushroom Kingdom**.

---

## 1. "Unsupported base ROM" or "Unsupported donor ROM"

**Symptom:**
`snes-cli --build-peach` exits with `Input ROM verification failed`.

**Causes & Fixes:**
- **Wrong region:** Currently, the builder requires:
  - *The Legend of Zelda: A Link to the Past* — **France (PAL)** (`Legend of Zelda, The - A Link to the Past (France).sfc`)
  - *Super Mario World* — **Europe (PAL)** (`Super Mario World (Europe).sfc`)
  - USA (NTSC), Japanese (NTSC-J), German, or other regional dumps have different memory offsets and are intentionally rejected so the builder never outputs a corrupted ROM.
- **Wrong file argument order:** Double-check that `--zelda` points to *A Link to the Past* and `--smw` points to *Super Mario World*.
- **Modified or bad dump:** Make sure your input ROMs are clean, unmodified backups matching the exact hashes listed in [`docs/ROM_REQUIREMENTS.md`](ROM_REQUIREMENTS.md).

---

## 2. My ROM Is `.smc` and Has 512 Extra Bytes

If your backup is `1,049,088` bytes (Zelda) or `524,800` bytes (SMW), it contains a legacy 512-byte SMC copier header.

- `snes-cli --build-peach` automatically detects and strips the 512-byte header in memory before checking SHA-256 and building.
- Your original `.smc` file is left untouched.
- If you compare hashes manually using `sha256sum`, keep in mind `sha256sum` hashes the 512-byte header as well. Use `./scripts/build.sh --verify-only <zelda> <smw>` to verify the unheadered payload directly.

---

## 3. Rust Compilation Fails (`as_chunks` or MSRV Error)

**Symptom:**
Compiler error mentioning `no method named 'as_chunks'`.

**Fix:**
Update to a current stable Rust toolchain (Rust **1.88+**):

```bash
rustup update stable
rustc --version
```

---

## 4. Black Bars Top/Bottom or 50 Hz Timing in Emulator

Because the base ROM is the **France PAL** release of *A Link to the Past*, the generated ROM runs in **PAL 50 Hz** mode by default (256×224 active picture inside a 239-line PAL frame).

- **Mednafen (`snes_faust` / `snes`):** Works out of the box at native PAL timing.
- **bsnes / Ares / RetroArch:** Enable integer scaling and crop vertical overscan (`224` active lines) if you want edge-to-edge full-screen pixels without thin PAL borders.

---

## 5. Save Files / SRAM Between Versions

If you previously played an older development build of the ROM, delete or back up old `.srm` / savestate files before loading a newly built release so stale emulator savestates do not override updated VRAM or WRAM state.

---

## 6. Still Stuck?

Open an issue using the [Bug Report template](../.github/ISSUE_TEMPLATE/bug_report.md) and include:
- Your OS and `rustc --version`
- The exact command you ran and its terminal output (never attach ROM files!)
- The emulator name and version if reporting an in-game visual or audio issue
