---
name: Bug Report
about: Report a build error, graphical glitch, audio issue, or emulator problem
title: "[Bug]: "
labels: bug
---

### Summary
A clear, concise description of the bug.

### Environment
- **OS:** (e.g. Linux, macOS, Windows 11)
- **Rust version (`rustc --version`):**
- **Release tag / Commit SHA:** (e.g. `v0.1.0`)
- **Emulator & Core:** (e.g. Mednafen `snes_faust`, bsnes, RetroArch)

### Input ROM Verification
- Did `./scripts/build.sh --verify-only` succeed? (Yes / No)
- Output ROM SHA-256 (if built):

> **IMPORTANT:** Never attach ROM files, `.sfc`, `.smc`, `.ips`, or `.bps` files to GitHub issues.

### Steps to Reproduce
1. ...
2. ...

### Expected vs. Actual Behavior
Describe what you expected to see and what actually happened. If it is an in-game visual bug, please attach a screenshot of the exact location/boss.
