# Mushroom Kingdom World Transformation Report (`v0.10.0`)

**Release:** `v0.10.0 — Super Mario Adventure Beyond Limits`  
**ROM SHA-256:** `2da376f5c5cd6f760c07e465bdba252fe1aa1f03e69f17dcabcff1ecd6e072b1`  
**Overworld Coverage:** **`128 / 128` Light & Dark World Screens (`100.0%`) + Star Road `0x80` (`129 / 160` total Overworld areas)**

---

## 1. Systemic 128/128 Overworld Transformation (`ZRI-007 -> 0`)

Building on `v0.7.0` (`108/128` screens), `v0.10.0` closes all remaining `20/128` untouched Light & Dark World screens (`0x1C, 0x1E, 0x1F, 0x21, 0x27, 0x2B, 0x34, 0x3F, 0x40, 0x41, 0x48, 0x49, 0x5B, 0x5C, 0x5E, 0x5F, 0x60, 0x61, 0x62, 0x6B`) PLUS Special Area **`0x80` Star Road Celestial Grove (`MU-067`)**, using strictly hardware-collision-identical uniform `TileAttr` (`$00:FBD9`) replacement pools:

- **Light World (`64 / 64` transformed):** `0` terrain grammar violations, `0` hardware collision regressions, `0` cross-screen border breaks across all `112` internal Light World cardinal edges.
- **Dark World (`64 / 64` transformed):** `0` terrain grammar violations, `0` hardware collision regressions, `0` cross-screen border breaks across all `112` internal Dark World cardinal edges, paired with Bowser's Volcanic Kingdom (`0x0DE70E` / `0x0DE79A`) and Obsidian Lava Citadel (`0x0DEB36` / `0x5B`).
- **Light World Vanilla Dome / Chocolate Mountain (`MU-045`):** Overhauled `OWMainPal[2]` (`PC 0x0DE754`) from cold grey Zelda mountain rock into sunlit warm **SMW Vanilla Dome terracotta-amber crags & sandstone plateaus**, live-verified via Real Warp Pipe arrival on both `snes_faust` and cycle-accurate `bsnes` (`docs/media/screenshots/25_v0100_beyond_limits_gallery.png` Panel 3).
- **Physical 5-Hub SMW Warp Pipe Network (`MU-041`):** Placed Physical 3D Glossy Green SMW Warp Pipes (`m32 = 0x02D4`) directly beside native `$02:EB67` landing coordinates on `0x2C (r09,c07)`, `0x19 (r12,c03)`, `0x0B (r10,c05)`, `0x3E (r04,c07)`, and `0x12 (r06,c05)`.
- **All 8 / 8 Bespoke Koopaling & King Bowser Fortresses:** `DungPal[1]` Wendy, `DungPal[4]` Lemmy, `DungPal[5]` Larry, `DungPal[8]` Morton, `DungPal[9]` Iggy, `DungPal[10]` Roy, `DungPal[12]` Ludwig, `DungPal[13]` King Bowser.
- **Relocated Map32 Budget:** All `129` transformed screens (`258` compressed HI/LO Big-Endian LC_LZ2 streams) fit cleanly inside LoROM Banks `$3B..$3F` (`0x1D8000..0x1FFFFF`) with `0` overlaps and 100% lossless decode verified across all `160/160` Overworld screens.
