# Terrain Grammar & Map32 Validation Report (`v0.6.0`)

## Root Cause of BUG 03 (`v0.5.0` Screen `0x2C` Seams)

In `v0.5.0`, **World 1 — Yoshi's Island (`Screen 0x2C`, Mario's House Courtyard)** exhibited two visual terrain defects on live SNES output:
1. Two orphan brown dirt ledges (`Map16 0x0AC/0x0AD`) appearing directly behind the northern Turn-Blocks at `r09 c4` and `r09 c7` because Map32 `0x1B76` decodes to `(0AD, 0AC / 16C, 16C)` (dirt transition + cliff top), not Turn-Blocks on grass.
2. A floating brown dirt triangle at `r10 c5` breaking the horizontal cliff top, plus an orphan dirt-edge tile `0x06A0` (`09F, 09F / 0A5, 0A5`) at `r15 c12` in the southern meadow.

---

## Bit-Exact Map32 → Map16 Decoder & Validator (`crates/snes-core/src/mario_map.rs`)

To prevent blind Map32 ID guessing permanently, `v0.6.0` implements ALTTP's exact packed 12-bit Map32 quadrant decoder:

- **ROM Tables Read:**
  - Top-Left Quadrant (`TL`): `PC 0x018000`
  - Top-Right Quadrant (`TR`): `PC 0x01B410`
  - Bottom-Left Quadrant (`BL`): `PC 0x020000`
  - Bottom-Right Quadrant (`BR`): `PC 0x023410`
- **Functions Added:**
  - `decode_map32_quads(rom, map32_id) -> [u16; 4]`
  - `expand_screen_to_map16(rom, screen) -> [[u16; 32]; 32]`
  - `classify_map16_grammar(map16_id) -> Map16TerrainClass`
  - `validate_screen_terrain_grammar(rom, original, edited) -> TerrainGrammarReport`

### Automated Terrain Rules Enforced
1. **Continuous Cliff Top Continuity:** Every cliff-face tile (`0x520`) directly below a modified grass/courtyard row must be topped by a valid cliff-top cap (`0x16C`) with no floating dirt notch.
2. **Zero Orphan Dirt Seams:** Modified pure-grass hub tiles may not introduce orphan dirt edge/transition tiles (`0x09E, 0x09F, 0x0A4, 0x0A5, 0x0A9, 0x0AC, 0x0AD, 0x17C, 0x414, 0x415`) surrounded by pure grass (`0x034`).
3. **Doorway & Spawn Walkability:** Mario's House doorway (`r06..r07, c7..c8`) and south cliff drop-off remain 100% unobstructed and passable.

---

## Exact Screen `0x2C` Corrections (`v0.6.0`)

| Grid Cell `(row, col)` | `v0.5.0` Map32 (Buggy) | `v0.5.0` Decoded Map16 | `v0.6.0` Map32 (Fixed) | `v0.6.0` Decoded Map16 | Visual Effect |
| :--- | :---: | :--- | :---: | :--- | :--- |
| `(r09, c04)` | `0x1B76` | `0AD, 0AC / 16C, 16C` | **`0x06C7`** | `20F, 20F / 034, 034` | Pure SMW Turn-Blocks on seamless green grass |
| `(r09, c07)` | `0x1B76` | `0AD, 0AC / 16C, 16C` | **`0x06C7`** | `20F, 20F / 034, 034` | Pure SMW Turn-Blocks on seamless green grass |
| `(r10, c05)` | `0x1B76` / dirt | Floating brown dirt notch | **`0x0208`** | `034, 034 / 16C, 16C` | Pure grass above continuous horizontal cliff rim |
| `(r15, c12)` | `0x06A0` | `09F, 09F / 0A5, 0A5` | **`0x0166`** | `034, 034 / 034, 0DA` | Pure green daisy meadow (zero dirt edge) |

---

## Verification Results

- **Unit Test:** `mario_map::tests::test_world1_screen2c_terrain_grammar_zero_seams` explicitly proves that the validator **rejects** the `v0.5.0` buggy layout (`0x1B76` / `0x06A0`) and **passes with 0 violations** on `v0.6.0`.
- **Dual-Core Visual Verification:** Inspected live on both Mednafen `snes_faust` (`01_bug01_contact_koopa`, `02_bug02_03_fire_terrain`) and cycle-accurate `snes` / bsnes (`08_bsnes_verify`).
- **Scope Note:** `1 / 64` Light World screens (`Screen 0x2C`) has custom Map32 redesigns in `v0.6.0`; all `64 / 64` Light World screens pass lossless Map32 LC_LZ2 round-trip verification (`test_overworld_map32_lossless_roundtrip_all_64_light_world`).
