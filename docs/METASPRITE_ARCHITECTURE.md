# Metasprite & OAM Architecture (`v0.6.0`)

This document captures the verified SNES OAM, VRAM DMA, and CGRAM hardware contracts discovered and enforced in `v0.6.0`.

---

## 1. Player Sprite DMA & Bank `$0D` OAM Architecture (`demo.rs`, `mixer.rs`)

### Bank `$00` Link DMA Pointer Tables (`PC 0x001346..0x001802`)
- **Head Address Table:** `PC 0x001346` (`303 × u16`, LoROM offset into Bank `$10` starting at `$8000`).
- **Body Address Table:** `PC 0x0015A4` (`303 × u16`, LoROM offset into Bank `$10` starting at `$8000`).
- **Hardware Alignment Invariant:** Each 16×16 4bpp character cell occupies 2 top tiles (`64 B`) at `addr` plus 2 bottom tiles (`64 B`) at `addr + 0x200` (stride of 16 tiles = `512 B`). Therefore, any non-overlapping 16×16 cell MUST sit on an **even tile row**: `((addr - 0x8000) & 0x023F) == 0`.
- **Legacy Bug Fixed (BUG 01 & BUG 02):** Vanilla ALTTP packed 28 "24px tall" poses (including Hurt/Recoil `0x03B, 0x03F..0x042` and Fire Rod Cast `0x043, 0x046, 0x049, 0x04C`) with `Body = Head + 0x200` (overlapping 8px!) or `Body = 0x8000` (`R00C0`, Mario's idle head!). In `v0.6.0`, all 28 poses are remapped to dedicated even-row cells inside the 29 verified free Bank `$10` slots (`R22C0..7`, `R07C1..4`, `R17C3..5`, `R23C0`).

### Bank `$0D` Pose OAM Table (`PC 0x068000..0x06838D`, `303 × 3 bytes`)
- Byte 0: Head `dy`, Byte 1: Head `dx`, Byte 2: High/Low nibble flags (`0xF0` = hide Head, `0x0F` = hide Body, bit flags for H/V flip).
- Replaced the previous blanket overwrite with a **pose-aware table**:
  - Standard 2-piece poses: `dy = -8, dx = 0`.
  - Single-tile poses (`0x03C..0x03E`, `0x114`): preserved untouched.
  - Water ripples (`0x08E..0x098` at `dy = -2`; side swim `0x09C..0x0A1` at `dy = +16`): preserved for seamless swimming.
  - Horizontal KO (`0x111`): preserved.
  - Poses `0x12B..0x12E`: erroneous vertical flip cleared so Mario never renders upside-down.

---

## 2. `Sprite_DrawMultiple_quantity_preset` (`$05:DF6F`) XOR Palette Contract (BUG 04)

Disassembly of `PC 0x02DF6F` (`$05:DF6F`) proved a critical ALTTP engine contract:

```asm
LDA ($08),Y    ; 16-bit load: low = OAM tile, high = table attr flags
EOR $04        ; XORs high byte (`flags`) with `$05` (= sprite's `$0F50,X`!)
```

- **Consequence:** For `Shopkeeper 0xBB` (`$05:F8F5`, 32 bytes for 2 frames × 2 OAM sprites), setting `$0F50,X = 0x09` (Pal 4 Royal Blue `0x08` + Name Table `1`) AND setting table `flags = 0x08` caused `0x08 ^ 0x09 = 0x01` (cancelling Pal 4 down to OBJ Pal 0 Orange `0x0DD3D6`!).
- **Fix:** Table `flags` at `0x02F8F5` MUST be `0x00` so `0x00 ^ 0x09 = 0x09` selects **OBJ Pal 4 (`0x0DD272` SMW Royal Blue `#4194FF / #004AAC`) + NT1**.
- **Tile Separation:** Body frames moved from `$10` (overlapping Head row 1 and row 3 price digits `'0'`/`'1'`) to clean cells `$06` `(6,0)` and `$26` `(6,2)` in `raw 0xBE` (`Sheet 0xBC`), leaving digits `'0'..'9'` bit-identical to vanilla.

---

## 3. Room `0x0103` Toad `Sprite 0x35` Exact Slot Map (BUG 05)

Disassembly of `$05:E543` (`PC 0x02E543`) proved `Sprite 0x35` reads `raw 0xBD` (`sp:4A`) VRAM slot 2 (`$80..$BF`):
- `$80` `(0,0)` — Front Head (`dy` patched at `PC 0x02E544` from `-4 [0xFC]` to `-9 [0xF7]`)
- `$82` `(2,0)` — Body Frame 1 (`dy = +4`)
- `$84` `(4,0)` — Body Frame 2 (`dy = +4`)
- `$86` `(6,0)` — Mirrored bottom tile at `dy = +15` (cleared to transparent `0` so ALTTP's native ground shadow renders cleanly with zero giant grey lump).

---

## 4. Mothula / Winged Iggy Koopa OAM Table (`$1A:FCED`, BUG 06)

- **Location:** `PC 0x0D7CED` (`128` bytes, 2 flap frames × 8 OAM entries × 8 bytes).
- **Fix:** Preserved body tiles/offsets untouched; pulled Left Wing (`entries 0, 2`) from `dx = -24` to **`dx = -17`** and Right Wing (`entries 3, 5`) from `dx = +24` to **`dx = +17`**, with coordinated upstroke/downstroke `dy` (`-10/+6` vs `-6/+10`) so wings rooted at `x=15` sit flush against Iggy's shoulders.

---

## 5. Final Boss Ganon / King Bowser Cross-Sheet 16×24 Arm Architecture (BUG 07)

Disassembly of `Ganon_Draw` (`$1D:9AA0`, `PC 0x0E9AA0`) revealed:
- **Held Trident Call:** `PC 0x0E9AF5` (`20 1C 9C` = `JSR Ganon_DrawHeldTrident`) -> NOP'd (`EA EA EA`) eliminating 100% of held trident orb/stick sprites while preserving thrown projectiles (`Sprite 0xCE`, replaced with SMW `GFX24` Giant Fireballs).
- **OAM Tables (12 sub-sprites per frame):** `X @ 0x0E95CE`, `Y @ 0x0E969A`, `Tile @ 0x0E9766`, `Attr @ 0x0E9843`.
  - Standing/Dialogue Frames `2, 3, 4`: Head `s01/s02` lifted from `dy = -10 (0xF6)` to **`dy = -15 (0xF1)`** above upper shell/torso `s03/s04` (`dy = -13`).
- **Paired `dy = +8` Cross-Sheet 16×24 Arm Strips:** Every Bowser arm is drawn as two 16×16 OAM entries spaced by `dy = +8` (forming one continuous `16×24` canvas). In Frames 2, 3, 4, and 7, Left Arm uses `$38` (Row 3 of `sp:21` `raw 0x94` + Row 0 of `sp:41` `raw 0xB4`) paired with `$48` (Rows 0..1 of `sp:41`). Mapping every arm onto exact disjoint 8px horizontal bands (`write_arm_band_16x8`):
  - `sp:21` rows `0..2` -> same-sheet arms (`$06/$16`, `$08/$18`, `$0A/$1A`, `$0C/$1C`)
  - `sp:21` row `3` + `sp:41` rows `0..1` -> cross-sheet arms (`$36/$46`, `$38/$48`, `$3A/$4A`, `$3C/$4C`)
  eliminates 100% of mixed-arm claw forks and severed forearms across all 14 animation frames.
