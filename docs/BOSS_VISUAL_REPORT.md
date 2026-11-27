# Boss Visual & Live Arena Audit Report — v0.7.0

**Target ROM:** `build/legend_of_peach_v0.7.0.sfc`  
**Verified SHA-256:** `f975131b5eaeee99ee7215ae636a380946ddc48832d3e7be95a11ab5d344dfee`  
**Visual Evidence Plate:** [`docs/media/screenshots/20_v070_all_12_bosses_gallery.png`](media/screenshots/20_v070_all_12_bosses_gallery.png)

In `v0.6.0`, only 2 of the 12 bosses (Winged Iggy and Final King Bowser) were live-verified on the final ROM. In **`v0.7.0`**, **ALL 12 / 12 BOSSES (`100%`)** have been live-emulated in their real Underworld arenas (extracted from `PC 0x04D653`), captured across multi-frame combat/dialogue sequences, and visually audited pixel-by-pixel with **zero visual corruption and zero regressions of `v0.6.0` BUG 06 / BUG 07**.

---

## Complete 12-Boss Live Verification Matrix

| # | Boss Name (Mario Universe) | ALTTP Room | Base Boss | Emulator Core(s) | Live Observed Behavior & Visual Verification | Status |
|---:|---|---|---|---|---|---|
| 1 | **6 Giant Chargin' Chucks** | `0x0C8` | Armos Knights | `mednafen:snes_faust` | Full 6-Chuck formation crisp in Eastern Palace arena; zero OAM clipping; Mario combat active | `VISUAL_PASS` |
| 2 | **3 Giant Pokeys** | `0x033` | Lanmolas | `mednafen:snes_faust` | All 3 segmented yellow-orange cactus Pokeys erupt out of desert sand with rock shards; clean shadows | `VISUAL_PASS` |
| 3 | **Giant Wiggler** | `0x007` | Moldorm | `mednafen:snes_faust` | Segmented yellow Wiggler with head flower crawling around Tower of Hera arena; pitfall drop to `-1` verified | `VISUAL_PASS` |
| 4 | **Castle Tower King Bowser** | `0x020` | Agahnim | `mednafen:snes_faust` | Crisp horned green/gold Bowser + **NEW v0.7.0 Mario dialogue (`#318`: "BWAHAHAHA ! Tu oses défier le grand Roi Bowser dans sa tour ?")** + clean warp vortex | `VISUAL_PASS` |
| 5 | **Morton Koopa Jr.** | `0x05A` | Helmasaur King | `mednafen:snes_faust` | Giant armored spiked shell boss with angry eyes and fiery whipping tail; crisp hitboxes & animation | `VISUAL_PASS` |
| 6 | **Wendy O. Koopa** | `0x006` | Arrghus | `mednafen:snes_faust` | Wendy core surrounded by orbiting ring swarm over shallow water; Mario wading ripples clean | `VISUAL_PASS` |
| 7 | **Winged Iggy Koopa** *(v0.6.0 BUG 06 Guard)* | `0x029` | Mothula | `mednafen:snes_faust` | **Zero regression:** crisp orange body, glasses, spiked hair, symmetrical anchored white/blue wings (`±17px` at `$1A:FCED`) | `VISUAL_PASS` |
| 8 | **Roy Koopa** | `0x0AC` | Blind the Thief | `mednafen:snes_faust` | **Live SRAM trigger (`$7E:F3CC=0x06` + Room `0x065` sunlit bit `0x0100`):** Maiden steps into light -> full transformation cutscene -> Roy Koopa materializes cleanly with sunglasses & outstretched arms | `VISUAL_PASS` |
| 9 | **Lemmy Koopa** | `0x0DE` | Kholdstare | `mednafen:snes_faust` | Lemmy visible inside crystalline ice shell; Mario opens Start Menu, equips Fire Flower, transforms into Fire Mario, casts fireballs onto ice shell | `VISUAL_PASS` |
| 10 | **Larry Koopa** | `0x090` | Vitreous | `mednafen:snes_faust` | Larry surrounded by crisp Mini-Koopa hatchlings detaching and swarming Mario across the slime pool | `VISUAL_PASS` |
| 11 | **Ludwig von Koopa** | `0x0A4` | Trinexx | `mednafen:snes_faust` | 3-headed Mecha-Bowser/Ludwig boss: central blue-crested Ludwig + extending fiery red head + icy purple head | `VISUAL_PASS` |
| 12 | **Final Boss King Bowser** *(v0.6.0 BUG 07 Guard)* | `0x000` | Ganon | `mednafen:snes` (**cycle-accurate bsnes**) | **Zero regression on cycle-accurate bsnes:** SMW Bowser silhouette (`dy=-15`), 0 held Trident (`$1D:9AF5` NOP), clean French Mario speech (`#366`), Blue Spiny Shell toss & spin attack | `VISUAL_PASS` |

---

## Structural Non-Regression Guards (`crates/snes-core/src/lib.rs`)

- **BUG 06 (Iggy Wings):** Asserts `$1A:FCED == -17` and `$1A:FD05 == +17` (`test_v060_all_seven_blocking_bugs_structural_invariants`).
- **BUG 07 (Final King Bowser):** Asserts `$1D:9AF5..9AF7 == [0xEA, 0xEA, 0xEA]` (NOP held Trident) and `$1D:969A + 25..26 == 0xF1` (`dy = -15` standing head offset).
- **Castle Bowser Dialogue (`#316/#318/#319`):** Replaces vanilla Zelda FR `"occire"` / `"Héros"` text cleanly inside Bank `$1C` repacker (`test_bank1c_repacker_invariants`).
