# Global Sprite & Animation Audit (`v0.6.0`)

This audit catalogs player, NPC, enemy, item, and boss sprite assets in **The Legend of Peach `v0.6.0`**, explicitly separating **structurally verified**, **visually verified in live emulation**, and **not yet live-verified** states.

---

## 1. Player Character — Mario (Bank `$10`, `PC 0x080000..0x086FFF`, 303 Engine Poses)

| Metric | Count | Method | Status |
| :--- | ---: | :--- | :--- |
| Total ALTTP Player Poses (`0x000..0x12E`) | `303 / 303` | Exhaustive Rust structural audit (`test_v060_all_seven_blocking_bugs_structural_invariants`) + rendered contact sheet `/tmp/v060_diag/all_303_v060_step2.png` | **PASS** |
| Odd-row / overlapping 24px Head-Body pairs | `0 / 303` | All 28 legacy overlapping/odd-row poses remapped in `0x001346..0x001802` to verified free Bank `$10` cells (`R22C0..7`, `R07C1..4`, `R17C3..5`, `R23C0`) | **PASS** |
| Active unpatched Vanilla Link 16×16 cells | `0 / 303` | All 19+ previously unpatched cells (`R07..R11`, `R17..R18`, `R22..R23`) replaced with 4bpp Mario art | **PASS** |
| Same-cell `Head == Body` double-head poses | `0 / 303` | Enforced by unit assertion across all 2-piece poses | **PASS** |
| Live-emulated Mario states in `v0.6.0` | `10` | Idle, Walk (4-dir), Jump (`L`/`R`), Stomp bounce, Super Hammer swing, Koopa contact hurt/recoil (`12 fps`), Invincible blink, Fire Mario 4-dir fireball cast, Starman rainbow invincibility, Doorway transition | **PASS** |
| Structurally patched & sheet-verified, NOT yet live-captured in `v0.6.0` | `6` | Swimming / diving ripples (`0x08E..0x0A1`), Electric shock X-ray, Pushing/pulling statues, Kneeling/fall-in-pit, KO flat (`0x111`), Carry/lift heavy rock | **Structurally Verified Only** |

### Dedicated `v0.6.0` Mario Poses Added (`crates/snes-core/src/demo.rs`)
- **Hurt / Recoil (`0x03B`, `0x03F..0x042`):** `make_hurt_head_down/side/up` + `make_hurt_body_down/side/up` (eliminates BUG 01 stacked double heads).
- **Fireball Cast (`0x043`, `0x046`, `0x049`, `0x04C`):** `make_fire_throw_down/side/up` with clean upright OAM flags (eliminates BUG 02 inverted boots and unpatched Link head).
- **Special States:** `make_kneel_body`, `make_spark_overlay`, `make_shock_xray_head/body`, `make_swim_ripple`, `make_dive_splash`, `make_ko_flat_head/body`.
- **Pose-Aware Bank `$0D` OAM Table (`0x068000..0x06838D`):** Replaces the blanket overwrite with per-pose geometry (`y=-8` standard, single-tile preserved for `0x03C..0x03E`/`0x114`, swim `y=-2`/`+16`, KO `0x111` preserved, erroneous V-flip cleared on `0x12B..0x12E`).

---

## 2. NPCs & Town Cast (`mario_world.rs`, `mario_ui.rs`)

| Character | ROM Sheet(s) / OAM | Palette | `v0.6.0` Live Verification |
| :--- | :--- | :--- | :--- |
| **Kamek (Shopkeeper `Sprite 0xBB`)** | `raw 0xBE` (`(0,0)` Head, `(6,0)/(6,2)` Robe F0/F1), `$05:F8F5` OAM (`flags=0x00`), `0x030BFA=0x08`, `0x0F6F54=0x09` | OBJ Pal 4 (`0x0DD272`, SMW Royal Blue `#4194FF/#004AAC`) | **Visually Verified @ 4× zoom** (both wand idle & raised frames; `150/10/50` digits 100% intact) |
| **Toad (`Sprite 0x35`, Room `0x0103`)** | `raw 0xBD` (`$80` Head, `$82/$84` Body, `$86` transparent), `0x02E544 = 0xF7` (`dy=-9`) | OBJ Pal 3 (`0x0DD254`, Red Cap, zero index-2 lime-green on face) | **Visually Verified @ 4× zoom** (animated idle, zero lime-green mouth, zero grey blob) |
| **Toad Town Villagers / Toadette / Toadsworth / Baby Yoshi / Starman Fairy** | `raw 0xC2`, `0xC3`, `0xC0`, `0xC1`, `0xBF`; Innkeeper `0x0D7866/7E = 0xF5` (`dy=-11`) | Global OBJ Pal 2/3/4 | **Structurally Patched** (Room `0x0103` live-verified; outdoor Kakariko villagers & Flute Grove not re-captured in `v0.6.0`) |
| **Princess Peach & Luigi (Uncle)** | `raw 0xBC`, `raw 0xB9` (`mario_ui.rs`) | Dedicated Palettes | **Sanctuary verified (`05_bug05_toad_sanctuary`)** (Peach present on intro phase; Sanctuary post-rescue clear) |

---

## 3. Enemies & Bestiary (`mario_world.rs`, `mixer.rs`)

| Enemy Group | ROM Sheets | Live Verification (`v0.6.0`) |
| :--- | :--- | :--- |
| **Blue/Green/Red Koopa Troopa Guards** | `raw 0x99`, `0x9A`, `0x9B` | **Visually Verified @ 12 fps** in World 1 (`01_bug01_contact_koopa`); clean walk, spear/blade guard, contact, and stomp bounce. *(Note: Koopa guards still hold guard blades — converting guard blades to Hammer Bro hammers remains in `NEXT_ACTIONS.md`.)* |
| **16 Expanded SMW Bestiary Sheets** (Goomba, Galoomba, Chain Chomp, Bill Blaster, Monty Mole, Swooper, Sparky, Ninji, Ice Koopa, Dry Bones, Thwomp, Boos, Cheep Cheep, Buzzy Beetle, Spiny, Pokey) | `16` compressed sheets in `mario_world.rs` | **Structurally Audited (LC_LZ2 lossless, 0 bank overlaps)**; only Overworld Koopa Troopa was live-captured in `v0.6.0` QA. |

---

## 4. Boss Roster (All 12 Bosses — `mario_bosses.rs`, `mario_ui.rs`)

| # | Boss | ALTTP Slot / Sheets | `v0.6.0` Status |
| ---: | :--- | :--- | :--- |
| 1 | **6 Giant Chargin' Chucks** | Armos Knights (`raw 0xA8`) | Unchanged from `v0.5.0` (verified in `v0.5.0`; structurally audited in `v0.6.0`) |
| 2 | **3 Giant Pokeys** | Lanmolas (`raw 0xA9`) | Unchanged from `v0.5.0` (structurally audited in `v0.6.0`) |
| 3 | **Giant Wiggler** | Moldorm (`raw 0xAA`) | Unchanged from `v0.5.0` (structurally audited in `v0.6.0`) |
| 4 | **Castle Tower King Bowser** | Agahnim (`mario_ui.rs`) | Unchanged from `v0.5.0` (structurally audited in `v0.6.0`) |
| 5 | **Koopaling #1 — Morton Koopa Jr.** | Helmasaur King (`raw 0xAC`) | Unchanged from `v0.5.0` (verified in `v0.5.0`; structurally audited in `v0.6.0`) |
| 6 | **Koopaling #2 — Wendy O. Koopa** | Arrghus (`raw 0xAD`) | Unchanged from `v0.5.0` (structurally audited in `v0.6.0`) |
| 7 | **Koopaling #3 — Winged Iggy Koopa** | Mothula (`raw 0xAB` + `$1A:FCED` OAM) | **Rebuilt & Visually Verified @ 4 fps (`v0.6.0`)** — Body preserved 100%, Paratroopa wings anchored flush at `dx=±17` |
| 8 | **Koopaling #4 — Roy Koopa** | Blind (`raw 0xAE`) | Unchanged from `v0.5.0` (structurally audited in `v0.6.0`) |
| 9 | **Koopaling #5 — Lemmy Koopa** | Kholdstare (`raw 0xAF` + `0x0DD55E`) | Unchanged from `v0.5.0` (verified in `v0.5.0`; structurally audited in `v0.6.0`) |
| 10 | **Koopaling #6 — Larry Koopa** | Vitreous (`raw 0xB0`) | Unchanged from `v0.5.0` (structurally audited in `v0.6.0`) |
| 11 | **Koopaling #7 — Ludwig von Koopa** | Trinexx (`raw 0xB2`/`0xB3`) | Unchanged from `v0.5.0` (verified in `v0.5.0`; structurally audited in `v0.6.0`) |
| 12 | **Final Boss — Giant King Bowser** | Ganon (`raw 0x94, 0xB4, 0xB8, 0xA6`, `0x0E9AF5` NOP, `0x0E969A` Head `dy=-15`, SMW `GFX24` Fireballs) | **Overhauled & Dual-Core Visually Verified (`v0.6.0`)** on both `snes_faust` and cycle-accurate `bsnes` across dialogue, roar, and active combat |
