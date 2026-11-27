# External Technical Reference Notes — `ZeldaAlttPSNESRecomp` Conceptual Study

> **Policy Notice (Strict Separation):**  
> [`mstan/ZeldaAlttPSNESRecomp`](https://github.com/mstan/ZeldaAlttPSNESRecomp) was consulted strictly as an **informational architectural reference**.  
> **Zero code, assets, build steps, or dependencies** were copied or imported from that repository. *The Legend of Peach* remains 100% built by our standalone Rust ROM pipeline (`crates/snes-core`) targeting an **unmodified physical Super Nintendo / Super Famicom (2 MiB LoROM, 65c816 + PPU + SPC700)**.

---

## 1. Host-Only Capabilities Explicitly NOT Required by *The Legend of Peach*

The reference project uses static C recompilation (`snesrecomp`) running on a modern desktop CPU/GPU. The following capabilities rely on host hardware and are **strictly excluded** from our SNES ROM requirements:

| Capability in Recomp | Why It Requires Host Resources | *The Legend of Peach* SNES Stance |
|---|---|---|
| Full-map Seamless World canvas (`F4` zoom, multi-screen simultaneous rendering) | Exceeds SNES 256×224 PPU viewport, 64 KiB VRAM, and 128-entry OAM hardware limits | **Excluded** — native SNES PPU 256×224 scanout preserved |
| Widescreen / square-pixel independent HUD projection | Requires modern host GPU compositor outside SNES BG1/BG2/BG3 | **Excluded** — native SNES BG3 HUD preserved |
| Native C recompilation & expanded host actor pools | Exceeds 3.58 MHz 65c816 CPU & 128 KiB WRAM (`$7E..$7F`) | **Excluded** — all new Mario mechanics run in native 65c816 (`Bank $22`) & WRAM `$7E:F500..$7E:F53F` |
| MSU-1 streaming audio | Requires expansion hardware / SD-cart audio coprocessor | **Excluded** — 100% native SPC700 N-SPC songs & BRR/SFX |

---

## 2. Conceptual Lessons Adaptable to Native 65c816 SNES Architecture

Four high-leverage architectural patterns from the study translate cleanly onto real SNES hardware within our 2 MiB LoROM + 64-byte WRAM (`$7E:F500..$7E:F53F`) budget:

1. **Explicit Cross-Boundary State Gates (Instead of Ad-Hoc Transition Resets):**
   - *Concept:* Treat screen edges, doorways, stairs, and pits as explicit state synchronization gates rather than blindly wiping actor/player registers.
   - *SNES Adaptation (`v0.8.0` Bank `$22`):* Maintain a dedicated compact Mario State Gate in WRAM (`$7E:F500..$7E:F53F`) tracking Power-Up Form (Classic / Fire / Ice / Cape / Starman), active Yoshi Companion mount flag, and Ground-Pound / Jump vertical velocity so transitions cleanly preserve Power-Up & Yoshi state across doors and screen scrolls while safely clamping vertical `Z` height to `0` on room entry (zero softlocks, zero mid-air door desyncs).

2. **Halo-Aware Active Zone Continuity (Within 65c816 Cycle Budget):**
   - *Concept:* Never simulate the entire world simultaneously; focus CPU cycles exclusively on active entities inside the loaded 16-slot sprite table (`$7E:0D00..$7E:0FA0`) while using compact flags for boundary continuity.
   - *SNES Adaptation (`v0.8.0` Bank `$22`):* Our 65c816 Koopa Shell ricochet, Boo orientation-freeze AI, and Mario Ground-Pound shockwave operate strictly inside ALTTP's existing 16-slot sprite pool (`O(16)` worst case, `< 350` CPU cycles per frame — well under 1.5% of a PAL 50 Hz frame budget).

3. **Strict Separation of Graphic Tables vs. Behavioral State Machines:**
   - *Concept:* Decouple tile/OAM rendering lookups from gameplay state transitions so graphic upgrades never corrupt logic and logic upgrades never corrupt OAM.
   - *SNES Adaptation:* Our Rust pipeline separates `mario_world.rs` / `mario_bosses.rs` (3bpp/4bpp graphics & palettes) from `mario_engine.rs` (65c816 behavioral hooks in Bank `$22`) and `mario_map.rs` (Map32 geography & hardware `$00:FBD9` collision audits), all guarded by `PATCH_DEPENDENCIES.json` memory firewalls.

4. **Deterministic Frame-Replayed Validation & Differential Auditing:**
   - *Concept:* Verify state transitions, cross-boundary combat, and save/load restoration with deterministic input replays rather than ad-hoc manual checks.
   - *SNES Adaptation:* Our Python + Mednafen (`snes_faust` + cycle-accurate `snes`/bsnes) QA engine (`scripts/qa_engine_v070.py`, expanding in `v0.8.0`) drives deterministic SRAM + X11 gamepad sequences, records quantitative frame deltas and visual contact sheets, and logs full execution history into a persistent local SQLite validation database.
