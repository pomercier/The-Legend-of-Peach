# Deterministic Build & Reproducibility Report (`v0.20.0`)

**Release:** `v0.20.0 — Project Nintendo Limit Break`  
**Build Pipeline:** [`scripts/build.sh`](../scripts/build.sh) -> `snes-cli --build-peach` (`crates/snes-core v0.20.0`)

---

## 1. Exact Input ROM Integrity (Read-Only, Never Committed)

| Input ROM | Role | Size | Verified SHA-256 |
|---|---|---:|---|
| Zelda: A Link to the Past (FR PAL 1.0) | Target Base Engine | `1,048,576` B | `29a0d5812ccbb4b11bdb55d8f751a2a797b4110bf402ca8ba15eb2bf85db7b39` |
| Super Mario World (US/EU) | Authentic SMW Donor Assets | `524,288` B | `b5be1dba3012b6811a5660fbf2981cb23cdd1e48f845a42df00f0f55b19f0392` |

Both input hashes are enforced at build time by `snes-core`'s NIST SHA-256 verifier. Neither ROM is ever modified or committed to git.

---

## 2. Bit-Identical Dual-Pass Build Proof

Two independent consecutive executions of `./scripts/build.sh` writing to `build/legend_of_peach_v0.20.0.sfc` and `/tmp/lop_v0200_det.sfc` were verified byte-for-byte with `cmp -s` (`exit code 0`):

| Metric | Pass A (`build/legend_of_peach_v0.20.0.sfc`) | Pass B (`/tmp/lop_v0200_det.sfc`) | Match |
|---|---|---|---|
| **Output Size** | `2,097,152` bytes (2 MiB LoROM) | `2,097,152` bytes (2 MiB LoROM) | **IDENTICAL** |
| **Internal SNES Title** | `THE LEGEND OF PEACH` | `THE LEGEND OF PEACH` | **IDENTICAL** |
| **Hardware Patch Ops** | `773` patches across `18` rules (`3,078 ms`) | `773` patches across `18` rules (`2,847 ms`) | **IDENTICAL** |
| **Bank & Overlap Audit** | `52` sheets verified, `272` relocated in `$23..$3F`, **`0` overlaps** | `52` sheets verified, `272` relocated in `$23..$3F`, **`0` overlaps** | **IDENTICAL** |
| **SNES Header Checksum** | `0x5547` (Complement `0xAAB8`, Valid: `true`) | `0x5547` (Complement `0xAAB8`, Valid: `true`) | **IDENTICAL** |
| **Output CRC32** | `0x7A5A145B` | `0x7A5A145B` | **IDENTICAL** |
| **Output SHA-256** | `5e9c06a5db1ffc6be4879e258fa914bfdd56db101455082c7d2fb711df9cb910` | `5e9c06a5db1ffc6be4879e258fa914bfdd56db101455082c7d2fb711df9cb910` | **BIT-IDENTICAL** |

---

## 3. Expansion ROM Budget & Studio Inspector Audit

Audited automatically by [`scripts/peach_studio_inspector.py`](../scripts/peach_studio_inspector.py) (zero external dependencies):
- **Total Expansion Space (Banks `$20..$3F`, 32 banks):** `1,048,576` bytes
- **Expansion Bytes Used:** `135,442` bytes (`12.92%`)
- **Expansion Bytes Free:** `913,134` bytes (`87.08%`)
- **4 MiB LoROM Expansion Required?** `false` — `87.08%` of our 2 MiB expansion space remains free, keeping maximum flashcart compatibility.
- **Generated Inspector Outputs:** [`docs/ROM_BANK_UTILIZATION.json`](ROM_BANK_UTILIZATION.json), [`docs/WORLD_MASTER_MAP.json`](WORLD_MASTER_MAP.json), and local HTML dashboard `build/peach_studio_dashboard.html`.
