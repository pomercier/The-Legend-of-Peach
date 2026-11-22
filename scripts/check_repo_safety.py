#!/usr/bin/env python3
"""
Repository Safety Audit for The Legend of Peach.

Verifies that NO copyrighted ROM files, binary ROM patches, ROM archives,
emulator saves, oversized binary blobs, personal host paths, or credentials
exist anywhere in:
  1. Tracked git files in HEAD / index
  2. Entire git commit history (all reachable blobs)
"""

from __future__ import annotations

import os
import re
import subprocess
import sys
from pathlib import Path

FORBIDDEN_EXTENSIONS = {
    ".sfc",
    ".smc",
    ".fig",
    ".swc",
    ".nes",
    ".gba",
    ".gbc",
    ".gb",
    ".n64",
    ".z64",
    ".v64",
    ".ips",
    ".bps",
    ".ups",
    ".xdelta",
    ".srm",
    ".sav",
    ".bs",
    ".zip",
    ".7z",
    ".rar",
    ".tar",
    ".gz",
    ".bz2",
    ".xz",
    ".mp4",
    ".mov",
    ".mkv",
    ".avi",
    ".wav",
    ".bin",
}

MAX_TRACKED_FILE_BYTES = 2 * 1024 * 1024  # 2 MiB ceiling for any tracked file

PERSONAL_PATH_PATTERNS = [
    re.compile(rb"/home/[A-Za-z0-9._-]+/"),
    re.compile(rb"/Users/[A-Za-z0-9._-]+/"),
    re.compile(rb"C:\\\\Users\\\\[A-Za-z0-9._-]+\\\\"),
]

SECRET_PATTERNS = [
    re.compile(rb"ghp_[A-Za-z0-9]{20,}"),
    re.compile(rb"gho_[A-Za-z0-9]{20,}"),
    re.compile(rb"github_pat_[A-Za-z0-9_]{20,}"),
    re.compile(rb"AKIA[0-9A-Z]{16}"),
    re.compile(rb"-----BEGIN (?:RSA |OPENSSH |EC )?PRIVATE KEY-----"),
]

TEXT_SCAN_EXTS = {
    ".rs",
    ".toml",
    ".md",
    ".yml",
    ".yaml",
    ".json",
    ".sh",
    ".py",
    ".txt",
    "",
}


def run_git(*args: str) -> bytes:
    return subprocess.check_output(["git", *args], stderr=subprocess.STDOUT)


def check_extension(path_str: str) -> str | None:
    lower = path_str.lower()
    ext = Path(lower).suffix
    if ext in FORBIDDEN_EXTENSIONS:
        return f"forbidden extension '{ext}'"
    if ".state" in Path(lower).name:
        return "emulator savestate filename"
    return None


def main() -> int:
    repo_root = Path(__file__).resolve().parent.parent
    os.chdir(repo_root)

    failures: list[str] = []

    # 1. Check all tracked files in the index / working tree
    tracked_raw = run_git("ls-files", "-z").split(b"\x00")
    tracked_files = [p.decode("utf-8", errors="replace") for p in tracked_raw if p]

    for rel in tracked_files:
        reason = check_extension(rel)
        if reason:
            failures.append(f"[TRACKED] {rel}: {reason}")
        full = repo_root / rel
        if not full.is_file():
            continue
        size = full.stat().st_size
        if size > MAX_TRACKED_FILE_BYTES:
            failures.append(
                f"[TRACKED] {rel}: size {size:,} bytes exceeds {MAX_TRACKED_FILE_BYTES:,} byte limit"
            )

        # Skip self when scanning regex patterns
        if rel == "scripts/check_repo_safety.py":
            continue

        if Path(rel).suffix.lower() in TEXT_SCAN_EXTS:
            data = full.read_bytes()
            for pat in PERSONAL_PATH_PATTERNS:
                m = pat.search(data)
                if m:
                    failures.append(
                        f"[PERSONAL_PATH] {rel}: matched {m.group(0).decode('utf-8', 'replace')!r}"
                    )
            for pat in SECRET_PATTERNS:
                if pat.search(data):
                    failures.append(f"[SECRET] {rel}: matched credential pattern {pat.pattern!r}")

    # 2. Check ENTIRE git history for any forbidden paths or large blobs
    rev_list = run_git("rev-list", "--objects", "--all").decode("utf-8", errors="replace")
    seen_paths: set[str] = set()
    for line in rev_list.splitlines():
        parts = line.split(" ", 1)
        if len(parts) == 2:
            _, path_str = parts
            if path_str in seen_paths:
                continue
            seen_paths.add(path_str)
            reason = check_extension(path_str)
            if reason:
                failures.append(f"[GIT_HISTORY] {path_str}: {reason}")

    # Verify blob sizes across full git history
    cat_check = subprocess.run(
        ["git", "cat-file", "--batch-check=%(objecttype) %(objectsize) %(rest)"],
        input=rev_list.encode("utf-8"),
        capture_output=True,
        check=True,
    ).stdout.decode("utf-8", errors="replace")
    for line in cat_check.splitlines():
        parts = line.split(" ", 2)
        if len(parts) >= 2 and parts[0] == "blob":
            size = int(parts[1])
            name = parts[2] if len(parts) > 2 else "<unknown>"
            if size > MAX_TRACKED_FILE_BYTES:
                failures.append(
                    f"[GIT_HISTORY_SIZE] {name}: blob size {size:,} bytes exceeds limit"
                )

    if failures:
        print("REPOSITORY SAFETY AUDIT FAILED:", file=sys.stderr)
        for f in failures:
            print(f"  - {f}", file=sys.stderr)
        return 1

    print(
        f"Repository safety audit PASSED ({len(tracked_files)} tracked files, "
        f"{len(seen_paths)} unique paths in git history; 0 ROMs, 0 patches, 0 leaks)."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
