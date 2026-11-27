# Power-Up & Special Action Animation Report (`v0.6.0`)

## Root Cause of BUG 02 (`v0.5.0` Fire Mario Cast Corruption)

Casting a Fireball while transformed into **Fire Mario** (`FLEUR DE FEU` equipped) previously displayed:
1. **Inverted Mario boots floating above an unpatched golden Link head** (`media_1795753871393_64bfa35b.png`) caused by odd-row/unpatched Bank `$10` pointers for rod-casting poses (`0x043, 0x046, 0x049, 0x04C`) combined with vertical-flip OAM bits on `0x12B..0x12E`.
2. **A residual vertical Zelda Fire Rod staff** drawn in front of Mario from uncompressed sprite sheet `0x087000` tiles `(4,0), (5,0), (6,0), (2,1), (5,1), (6,1)`.

---

## Structural Fixes (`demo.rs`, `mario_ui.rs`, `mario_engine.rs`)

1. **Dedicated 4-Direction Fireball Throw Poses (`demo.rs`):** Added `make_fire_throw_down`, `make_fire_throw_side`, and `make_fire_throw_up` written to clean even-row `16×16` cells in Bank `$10`, pointed directly by Bank `$00` DMA tables (`0x001346..0x001802`).
2. **Pose-Aware Bank `$0D` Flip Cleanse (`demo.rs`):** Cleared erroneous vertical-flip inversion bits on poses `0x12B..0x12E` so Mario's head and boots never invert during special casts.
3. **Zelda Fire/Ice Rod Removal (`mario_ui.rs`, `patch_smw_live_weapons_087000`):** Replaced the 6 vanilla Zelda Fire/Ice Rod staff tiles in `0x087000` with a blazing Mario Fireball Cast Core and Flame Sparks so Fire Mario casts fire directly from his outstretched hand.
4. **Live CGRAM Power-Up State Machine (`mario_engine.rs`, `$22:9900`):**
   - **Classic Mario:** Default Red Cap / Blue Overalls.
   - **Fire Mario (`$0202 == 0x06`):** Live-swaps CGRAM OBJ Pal 7 into Pure White Cap/Shirt + Blazing Crimson Overalls immediately upon equipping `FLEUR DE FEU`.
   - **Super Starman (`L + R`):** Cycles 4 flashing rainbow CGRAM palettes (`$22:9900`) for 4 seconds with instant enemy contact-kill and clean restoration upon expiry.

---

## Dual-Core Verification Results (`v0.6.0`)

| Power-Up / Action | `snes_faust` | `snes` (bsnes-accuracy) | Notes |
| :--- | :---: | :---: | :--- |
| **Classic Mario Walk + Jump (`L`/`R`) + Hammer (`B`)** | **PASS** | **PASS** | Smooth parabolic arc over oval ground shadow |
| **Fire Mario Transformation (Equip `FLEUR DE FEU`)** | **PASS** | **PASS** | Instant white/red CGRAM swap on menu exit |
| **Fire Mario 4-Direction Fireball Cast (`Y`)** | **PASS** | **PASS** | Crisp Fire Mario cast pose + blazing fireball; `0` inverted boots, `0` Link tiles, `0` Zelda Fire Rod |
| **Fire Mario Jump + Stomp** | **PASS** | **PASS** | Maintains Fire Mario palette seamlessly while airborne |
| **Super Starman (`L + R`) Rainbow Flash** | **PASS** | **PASS** | Clean 4-palette rainbow flash during jump and movement (`08_bsnes_verify` `f_013`) |
| **Ice Flower (`FLEUR DE GLACE`) Cast** | Structurally Patched | Not Live-Captured | Shares the cleaned `0x087000` cast flame/spark tiles and remapped Bank `$00` cast poses |
