# Contributing to The Legend of Peach

Thank you for your interest in contributing to **The Legend of Peach: A Mario to the Mushroom Kingdom**!

---

## Absolute Rules

1. **NEVER commit, attach, or link to copyrighted ROM files or binary ROM patches.**
   - Do not commit `.sfc`, `.smc`, `.ips`, `.bps`, `.ups`, ROM archives, or extracted copyrighted tiles/audio.
   - Pull requests must pass `python3 scripts/check_repo_safety.py`.
2. **Keep SNES hardware compatibility sacred.**
   - Every modification must maintain valid 2 MiB LoROM mapping (`$00..$3F`), valid SNES checksum/complement, single-bank 32 KiB compressed graphics alignment, zero patch overlaps, and adherence to SNES PPU OAM/VRAM/CGRAM/SPC700 hardware limits.
3. **Visually verify every graphical or animation change in an accurate SNES emulator.**
   - Validate in Mednafen (`snes_faust` and/or `snes` bsnes-accuracy) or `bsnes` and inspect zoomed crops before opening a PR.

---

## Local Development Workflow

```bash
# 1. Fork and clone the repository
git clone https://github.com/pomercier/The-Legend-of-Peach.git
cd The-Legend-of-Peach

# 2. Place your legal backups in roms/ (git-ignored) or pass paths directly
./scripts/build.sh roms/zelda_alttp.sfc roms/super_mario_world.sfc build/legend_of_peach.sfc

# 3. Run all quality checks before opening a PR
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
python3 scripts/check_repo_safety.py
```

---

## Pull Request Checklist

- [ ] Code is formatted with `cargo fmt --all`
- [ ] Zero warnings under `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace --all-targets` passes both **without** ROMs (CI mode) and **with** verified ROMs (if modifying ROM patches)
- [ ] `python3 scripts/check_repo_safety.py` passes cleanly
- [ ] Commit messages and documentation are written in clear English
- [ ] Live emulator screenshots included in PR description for visual changes
