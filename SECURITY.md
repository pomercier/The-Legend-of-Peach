# Security Policy

## Scope

**The Legend of Peach** is an offline, local Rust command-line tool that reads user-supplied SNES ROM files and writes a transformed `.sfc` file locally. It performs no network requests, runs no background services, and stores no credentials.

## Reporting a Vulnerability or Legal / Copyright Concern

If you discover:
- A security issue (for example, an out-of-bounds panic, unsafe file write, or path traversal in `snes-cli`), or
- Any accidental inclusion of copyrighted ROM data, sensitive paths, or credentials in the repository

please open an issue or contact the maintainer (`@pomercier`) directly via GitHub so it can be remediated immediately.

## Integrity Guarantees

- Original input ROMs are opened strictly read-only and are never modified in place.
- Input ROMs are verified against published No-Intro SHA-256 hashes before transformation.
- Every pull request and commit on `main` is scanned automatically by `scripts/check_repo_safety.py` in GitHub Actions to enforce the zero-ROM and zero-secret policy.
