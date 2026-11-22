# Required Original ROMs

**The Legend of Peach: A Mario to the Mushroom Kingdom** does **not** include, distribute, embed, or link to any copyrighted game ROMs.

To build the playable crossover locally, you must provide **legal personal backups** of two original Super Nintendo Entertainment System (SNES) games. Your original ROM files are read strictly as read-only inputs and are **never modified**.

---

## Verified Input ROMs (Mandatory)

Both input ROMs below have been verified locally and cross-referenced against **No-Intro** SNES catalog signatures. The builder (`snes-cli --build-peach`) enforces **strict SHA-256 verification** on the unheadered ROM payload before building and refuses mismatched or unsupported dumps.

### 1. Base ROM — *The Legend of Zelda: A Link to the Past* (France, PAL)

| Field | Verified Value |
| :--- | :--- |
| **Game Title** | *The Legend of Zelda: A Link to the Past* (*La Légende de Zelda*) |
| **Console** | Super Nintendo Entertainment System (SNES / PAL) |
| **Region** | **France (PAL)** — SNES Header Country Code `0x06` |
| **Revision** | Rev 1.0 (`0x00`) |
| **No-Intro Name** | `Legend of Zelda, The - A Link to the Past (France).sfc` |
| **Mapping** | LoROM (`0x20`) |
| **Internal Header Title** | `LA LEGENDE DE ZELDA` |
| **Expected Size (Unheadered)** | `1,048,576` bytes (`1 MiB`) |
| **SNES Internal Checksum** | `0x65AE` (Complement: `0x9A51`) |
| **CRC32 (Unheadered)** | `1F8C30E8` |
| **SHA-1 (Unheadered)** | `229364a1b92a05167cd38609b1aa98f7041987cc` |
| **SHA-256 (Unheadered)** | `29a0d5812ccbb4b11bdb55d8f751a2a797b4110bf402ca8ba15eb2bf85db7b39` |

### 2. Donor ROM — *Super Mario World* (Europe, PAL)

| Field | Verified Value |
| :--- | :--- |
| **Game Title** | *Super Mario World* |
| **Console** | Super Nintendo Entertainment System (SNES / PAL) |
| **Region** | **Europe (PAL)** — SNES Header Country Code `0x02` |
| **Revision** | Rev 1.0 (`0x00`) |
| **No-Intro Name** | `Super Mario World (Europe).sfc` |
| **Mapping** | LoROM (`0x20`) |
| **Internal Header Title** | `SUPER MARIOWORLD` |
| **Expected Size (Unheadered)** | `524,288` bytes (`512 KiB`) |
| **SNES Internal Checksum** | `0x09E9` (Complement: `0xF616`) |
| **CRC32 (Unheadered)** | `3C41070F` |
| **SHA-1 (Unheadered)** | `56265120a74b55260ff7cacc00da1f21cbcb64f4` |
| **SHA-256 (Unheadered)** | `b5be1dba3012b6811a5660fbf2981cb23cdd1e48f845a42df00f0f55b19f0392` |

---

## Headerless vs. 512-Byte SMC Copier Headers

- Modern No-Intro `.sfc` dumps are **unheadered** (`1,048,576` bytes for *Zelda* and `524,288` bytes for *Super Mario World*).
- Legacy `.smc` backups sometimes include an extra **512-byte copier header** (`1,049,088` bytes or `524,800` bytes).
- `snes-cli` automatically detects and strips any 512-byte copier header in memory **before** computing SHA-256, so headered backups of the exact verified revisions are accepted seamlessly without modifying your original files.
- Note that external tools like `sha256sum` hash the raw file on disk; if your file has a 512-byte header, its raw `sha256sum` will differ even though `snes-cli --build-peach --verify-only` will strip the header and accept it.

---

## How to Verify Your ROMs Locally

### Option A — Built-in Verifier (Recommended)

```bash
./scripts/build.sh --verify-only /path/to/zelda_fr.sfc /path/to/smw_eu.sfc
```

This strips any 512-byte SMC header in memory, checks exact unheadered byte lengths and SHA-256 hashes, and exits without writing any output file.

### Option B — OS Checksum Tools (for Unheadered `.sfc` Files)

**Linux:**
```bash
sha256sum /path/to/zelda_fr.sfc /path/to/smw_eu.sfc
```

**macOS:**
```bash
shasum -a 256 /path/to/zelda_fr.sfc /path/to/smw_eu.sfc
```

**Windows (PowerShell):**
```powershell
Get-FileHash -Algorithm SHA256 C:\path\to\zelda_fr.sfc, C:\path\to\smw_eu.sfc
```

---

## Why Only These Exact Revisions Are Currently Supported

*The Legend of Peach* applies 177 low-level SNES hardware patches directly to 65c816 routines, compressed Nintendo LC_LZ2 tile banks, OAM tables, CGRAM palettes, French dialogue tables, and N-SPC audio banks. Offsets in USA (NTSC), Japanese (NTSC-J), German, or other revisions differ by hundreds or thousands of bytes.

Applying these patches to an unverified revision would corrupt code or graphics, so unsupported hashes are strictly rejected by default. Multi-region support (including USA NTSC) is tracked on the project roadmap.

---

## Expected Reproducible Output ROM Signature

When built with release **`v0.2.0`** from the two verified ROMs above, the locally generated output ROM is **100% deterministic (bit-for-bit identical)** across machines and operating systems:

| Field | Deterministic Output Value (`v0.2.0`) |
| :--- | :--- |
| **Output Size** | `2,097,152` bytes (`2 MiB`, LoROM expanded from `1 MiB`) |
| **Internal Header Title** | `THE LEGEND OF PEACH` |
| **SNES Checksum / Complement** | `0xEE72` / `0x118D` (Valid) |
| **CRC32** | `DA56A034` |
| **SHA-256** | `c700d1d2b60f50e789e2214fa8528c0c3af132ebc8af35a2f7ced5aa16735edb` |

*(Previous release `v0.1.0` SHA-256: `32540c4629c6b797f6933c7966a419fd959a3f711f36347f4348ceb2fa85b99c`, Checksum `0x464D` / `0xB9B2`.)*

> **Never commit, upload, or share the generated ROM file.** Both `roms/` and `build/` as well as all `.sfc`/`.smc` files are ignored by `.gitignore`.
