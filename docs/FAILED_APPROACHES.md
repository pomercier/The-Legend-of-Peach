# Failed Approaches & Lessons Learned

This permanent engineering log records approaches that failed or caused regressions so future development cycles never repeat them.

---

## 1. Duplicate Line-Break Emission in Bank `$1C` Repacker (`v0.3.0` / `v0.4.0`)
- **Symptom:** Corrupted, overlapping text on the left side of lines 1 and 2 in Bowser's Room `0x00` dialogue (`¿ri·galet` / `ne ·auser`) and several other long French messages.
- **Root Cause:** Auto-wrapping inserted a `0x75` (Line 2) or `0x76` (Line 3) break when a text segment exceeded 24 columns, and then the next segment's original `0x75` or `0x76` was ALSO emitted verbatim. In ALTTP's BG3 text renderer, `0x74`/`0x75`/`0x76` reposition the cursor WITHOUT clearing the target line, causing the second string to overwrite the first string's pixels in place.
- **Fix (`v0.5.0`):** Merge contiguous text across soft line breaks (and strip trailing `-` word-split hyphens!), reflow whole dialogue pages between `<WAIT>` (`0x7E`) / `<CTRL>` tokens using a strict `line_slot` state machine (`0 -> 0x75`, `1 -> 0x76`, `>=2 -> 0x73`), and enforce automated invariant tests guaranteeing zero consecutive duplicate `0x75`/`0x76` codes and zero hyphenated line breaks across all 372 messages.

## 2. Writing File Select Cursor to Sheet `0x76` (`0x088E00`) Instead of the Live File Select OBJ Sheet (`v0.3.0` / `v0.4.0`)
- **Symptom:** The File Select screen continued showing the vanilla pink Fairy cursor even though `build_smw_sparkling_star_cursor_frames` patched `0x088E00` cols 8..15.
- **Root Cause:** `0x088E00` (`Sheet 0x76`) is the Game Over / World Map OBJ sheet. File Select's `DrawCursor` routine at `$0E:9696` (`PC 0x071696`) draws tiles `0xA8` and `0xAA` with attribute `0x7E` from the File Select OBJ VRAM bank (not `0x088E00`).
- **Fix (`v0.5.0`):** Trace the exact File Select VRAM sheet + `$0E:9696` tile indices and patch the live tiles directly, verified in Mednafen live capture.

## 3. Reusing Pre-v0.2.0 (`v13_clips/`) Recordings in Later Showcase Videos (`v0.4.0`)
- **Symptom:** The v0.4.0 master showcase video displayed pre-v0.2.0 broken Kamek's Shop digits (`[orange] 5 [orange]`), a vanilla red heart, and a blue Zelda bomb, contradicting the actual fixed v0.4.0 ROM.
- **Root Cause:** The assembly script mixed fresh `/tmp/v040_qa/` captures with stale `scratch/v13_clips/` recordings.
- **Fix (`v0.5.0`):** Strict rule — EVERY clip in any showcase video MUST be freshly captured from the exact candidate ROM being verified, tagged with its SHA-256.

## 4. Wrong French File Select Character Codes in Test SRAM (`qa_v040_live.py`)
- **Symptom:** Test save slot 1 displayed `1. MA[U/n]IO[garbage]` instead of `1. MARIO`.
- **Root Cause:** In ALTTP FR File Select (`$0E:9071`), SRAM character codes map directly to 8×8 2bpp font top-half tile numbers (`A..P = 0x00..0x0F`, bottom halves occupy `0x10..0x1F`, `Q..Z` starts at `0x20`, so `R = 0x21` and `SPACE = 0xA9`). Writing `0x11` indexed into the bottom half of `'B'`.
- **Fix (`v0.5.0`):** Use exact verified FR File Select tile codes `[0x0C, 0x00, 0x21, 0x08, 0x0E, 0xA9]` (`MARIO `) AND test live keyboard name input on File Select.

## 5. Blind Global Substring Replacement in French Script (`v0.2.0` / `v0.3.0`)
- **Symptom:** Nonsensical words like `"Toadsse"` (from `"Sagesse"` matching `"Sages"`), `"Reliquess"` (from `"Triforce"` -> `"Reliques"` before plural `s`), and `"dans ton champi"` (from `"dans ton cœur"` matching `"coeur"`).
- **Fix (`v0.5.0`):** Word-boundary-aware substitutions (`Sagesse` preserved, anatomical/emotional `cœur` preserved while collectible `fragment de cœur` -> `Super Champignon`), plus full original French Mario narrative rewrites for all major story messages.

## 6. Unverified Mothula OAM Layout Assumption for Iggy Koopa (`v13` / `v0.5.0`)
- **Symptom:** Iggy Koopa appeared as a jumbled pile of overlapping rings in `v13`, and after fixing his body in `v0.5.0`, his wings floated detached ~8px away in midair (`dx = ±24`).
- **Root Cause:** Mothula's live OAM table at `PC 0x0D7CED` (`$1A:FCED`) positions left/right wings at `dx = ±24`.
- **Fix (`v0.6.0`):** Preserved Iggy's body untouched, sculpted SMW Paratroopa feathered wings anchored at `x = 15`, and patched `PC 0x0D7CED` pulling wings inward to `dx = ±17` with dynamic upstroke/downstroke `dy`.

## 7. Setting Non-Zero Palette Bits in `Sprite_DrawMultiple` OAM Table (`v0.6.0` dev)
- **Symptom:** Kamek Shopkeeper rendered Orange/Gold (`OBJ Pal 0` `0x0DD3D6`) instead of SMW Royal Blue (`OBJ Pal 4` `0x0DD272`) even when `$0F50,X` was set to `0x09`.
- **Root Cause:** `Sprite_DrawMultiple_quantity_preset` (`$05:DF6F`) executes `EOR $04` (XORing the OAM table's `flags` byte against `$0F50,X`). Putting `0x08` in BOTH `$0F50,X` and the OAM table caused `0x08 ^ 0x09 = 0x01` (OBJ Pal 0!).
- **Fix (`v0.6.0`):** Keep `flags = 0x00` in `$05:F8F5` so `0x00 ^ 0x09 = 0x09` activates OBJ Pal 4 SMW Royal Blue (`#4194FF / #004AAC`).

## 8. Writing 16×32 Arm Strips Across Ganon's Cross-Sheet `$38/$48` Boundary (`v0.5.0`)
- **Symptom:** Final King Bowser displayed an upside-down white claw fork floating over his left shoulder and a severed forearm on his right side.
- **Root Cause:** Ganon's paired `dy = +8` arm OAM tiles (`$38/$48`, `$36/$46`, `$3A/$4A`, `$3C/$4C`) span Row 3 of `sp:21` (`raw 0x94`) into Rows 0..1 of `sp:41` (`raw 0xB4`). Writing `16×32` arms into `sp:21` put `arm_down`'s bottom white claws into Row 3 (`$38` top half).
- **Fix (`v0.6.0`):** Write strictly disjoint `16×24` arm strips using 8px horizontal bands (`sp:21` rows `0..2` for `$0X/$1X`; `sp:21` row `3` + `sp:41` rows `0..1` for `$3X/$4X`).

## 9. Guessing Map32 IDs Without Decoding Map16 Quadrants (`v0.5.0`)
- **Symptom:** Placing `0x1B76` and `0x06A0` on Screen `0x2C` introduced orphan brown dirt seams behind Turn-Blocks and a floating brown dirt triangle.
- **Fix (`v0.6.0`):** Implemented ALTTP's exact packed 12-bit Map32→Map16 decoder + `validate_screen_terrain_grammar` in `mario_map.rs` and replaced buggy IDs with verified pure-grass IDs (`0x06C7`, `0x0208`, `0x0166`).
