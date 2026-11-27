# Contact, Collision & Hurt-State Visual Report (`v0.6.0`)

## Root Cause of BUG 01 (`v0.5.0` Mario vs. Enemy Contact Corruption)

When Mario made direct contact with a Blue Koopa Troopa in `v0.5.0`, the player sprite displayed **two stacked Mario heads** on top of each other (`media_1795753865708_ae5eeb28.png`).

ROM disassembly of Bank `$00` Link DMA tables (`PC 0x001346..0x001802`) and Bank `$0D` Link OAM table (`PC 0x068000..0x06838D`) proved three compounding causes:
1. **Overlapping 24px DMA Pointers:** ALTTP's hurt/recoil poses (`0x03B`, `0x03F`, `0x040`, `0x041`, `0x042`) pointed `LinkOAM_BodyAddresses` to either `Head + 0x200` (an odd tile row sharing the lower 8 pixels of Mario's head!) or `$8000` (`R00C0`, Mario's idle head cell!).
2. **Unpatched Vanilla Link Cells:** 19+ secondary cells in Bank `$10` (`R07C0..4`, `R08C3..7`, `R09C0..2,5..7`, `R10C0`, `R11C0`, `R17C0,3..5`, `R18C0`) had never been converted from Vanilla Link.
3. **Blanket Bank `$0D` OAM Overwrite:** The earlier `v0.5.0` builder overwrote Bank `$0D` pose offsets uniformly without respecting single-tile, recoil, swim, or KO states.

---

## Structural Fix (`crates/snes-core/src/demo.rs`, `crates/snes-core/src/mixer.rs`)

1. **Dedicated 4bpp Hurt & Recoil Art:** Added `make_hurt_head_down/side/up` and `make_hurt_body_down/side/up`, plus `make_kneel_body`, `make_shock_xray_head/body`, `make_swim_ripple`, `make_dive_splash`, and `make_ko_flat_head/body` covering all 19+ previously unpatched Bank `$10` cells.
2. **Exact Bank `$00` DMA Remapping (`0x001346..0x001802`):** Remapped all 28 odd-row/overlapping/`Body==R00C0` poses across all 303 Link/Mario poses onto verified free even-row `16×16` slots in Bank `$10` (`R22C0..7`, `R07C1..4`, `R17C3..5`, `R23C0`), lifted from the scratch clone in `inject_smw_mario_hero_into_zelda`.
3. **Pose-Aware Bank `$0D` OAM Table (`0x068000..0x06838D`):** Preserves single-tile states (`0x03C..0x03E`, `0x114`), swim offsets (`0x08E..0x0A1`), and horizontal KO (`0x111`), while keeping standard 2-piece hurt/walk/cast poses cleanly aligned at `dy = -8, dx = 0`.
4. **Zero Conflict with Custom Jump/Stomp Engine (`$22:9000`):** Mario's airborne jump (`L`/`R`), stomp bounce, post-stomp i-frames, and ground-contact hurt recoil coexist cleanly with ALTTP's native collision and invincibility blink state machine.

---

## Verification Evidence

- **Automated Invariant (`lib.rs`):** `test_v060_all_seven_blocking_bugs_structural_invariants` audits all `303 / 303` player poses confirming `0` odd-row overlaps, `0` active unpatched Link cells, and `0` `Head == Body` duplicates.
- **Live 12 fps Temporal Capture (`01_bug01_contact_koopa`):** High-speed frames `imp_001..imp_012` captured Mario approaching the Blue Koopa (`imp_004`), colliding head-on (`imp_005`), entering the hurt/recoil animation (`imp_006`, heart lost in HUD), blinking during i-frames (`imp_007..009`), recoiling cleanly mid-air (`imp_010`), and recovering cleanly (`imp_011..012`) with **zero double heads and zero OAM corruption**.
- **Not Yet Live-Tested:** Electric shock (`Bumper`/`Bari`), deep-water diving, and pit-fall sequences are structurally patched and verified on the 303-pose contact sheet, but were not individually recorded in live gameplay in `v0.6.0`.
