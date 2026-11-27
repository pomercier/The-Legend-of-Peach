# Art Direction Guide — Super Mario World 16-Bit Authenticity (`v0.6.0`)

## Creative Objective

**The Legend of Peach** aims to look and read like an authentic Nintendo Super Famicom / SNES top-down Super Mario adventure crafted in 1991–1995 alongside *Super Mario World*, *Super Mario All-Stars*, and *Super Mario RPG*, rather than a palette-swapped Zelda hack.

---

## Visual Pillars Applied in `v0.6.0`

### 1. Authentic Nintendo Silhouettes & Proportions
- **Mario (`demo.rs`):** Bold red cap with bright highlight crest, expressive black mustache and vertical eyes, blue overalls with brass buttons, white gloves, and brown boots. Every pose (idle, walk, carry, jump, hammer, hurt/recoil, fireball throw, swim, shock, KO) respects clean `16×16` Head + `16×16` Body separation with zero vertical squashing or duplicate head stacking.
- **Kamek (`mario_world.rs`):** Modeled directly on official Nintendo *Super Mario World* `GFX03` Magikoopa: tilted pointed royal-blue wizard hat, large round white spectacles with bold frames, golden Koopa snout, flowing royal-blue wizard cloak with vertical white trim, and a wooden wand tipped with a glowing pink crystal that raises on frame 1.
- **Toad (`mario_world.rs`):** Modeled on 16-bit *Super Mario All-Stars* / *Super Mario RPG* proportions: plump white mushroom cap with vivid crimson spots, friendly warm-peach face with vertical dark eyes (strictly avoiding palette index `2` lime-green on facial pixels), tidy vest with dark trim, white shorts, and chestnut boots, lifted to `dy=-9` (`Sprite 0x35`) / `dy=-11` (`Sprite 0xBC`) so the cap never buries the body.
- **Winged Iggy Koopa (`mario_bosses.rs`):** Preserves Iggy's expressive green-tufted Koopaling body (`cols 16..31`) intact while anchoring curved white/periwinkle SMW `GFX01` Paratroopa feathered wings directly into his shoulders (`dx=±17`).
- **Final King Bowser (`mario_bosses.rs`):** Modeled on official *Super Mario World* `GFX21` Final Boss Bowser: wild crimson hair tuft, sweeping curved ivory bull horns, fierce red eyebrows over green scaly brow, massive arched golden snout with sharp white fangs, hunched emerald spiked shell framed by white conical spikes, spiked black leather collar cleanly tucked under the chin, horizontal-ribbed golden belly, muscular emerald arms with spiked black wrist cuffs and clenched clawed fists, and real *Super Mario World* `GFX24` Giant Fireballs.

### 2. SNES 3bpp / 4bpp Palette Discipline
- **Dark Outlines:** Consistent crisp 1px dark contour (`#303030` or `#101018`, never noisy dithering) separating foreground sprites from high-saturation Mario grass and stone floors.
- **CGRAM-Aware Pixel Indexing:** Every sprite builder documents the exact live CGRAM OBJ palette slot it renders under:
  - Kamek → Global OBJ Pal 4 (`0x0DD272`, `6=#4194FF` Royal Blue, `7=#004AAC` Deep Blue, `4=#FFD583` Gold Snout, `2=#FF8BB4` Wand Sparkle).
  - Toad → Global OBJ Pal 3 (`0x0DD254`, `6=#FF3131` Red Cap Spots, `4=#FFC58B` Peach Skin, `3=#D5834A` Chestnut Boots; index `2=#D5DE52` Lime Green banned from skin/mouth).
  - Final King Bowser → Exclusive Room `0x00` Pal 5 (`0x0DD5EA` Torso/Shell/Belly) + Pal 6 (`0x0DD596` Head/Mane/Arms/Feet).
  - Fire Mario & Starman → Live WRAM/CGRAM palette tables at `$22:9900` (`mario_engine.rs`).

### 3. Terrain Readability & Tile Continuity
- Overworld hub tiles (`Screen 0x2C`, World 1 Yoshi's Island) strictly pair SMW Turn-Blocks (`0x06C7`), SMW smiling bushes, daisies (`0x0166`), and continuous horizontal cliff rims (`0x0208`) on uniform lush green grass (`Map16 0x034`) with zero abrupt dirt transitions or orphaned cliff corners.
- All Zelda-only combat effects replaced in `v0.6.0`: Fire/Ice Rod staves in `0x087000` replaced with Mario fireball cast flames; Ganon's held Trident completely removed (`0x0E9AF5` NOP).
