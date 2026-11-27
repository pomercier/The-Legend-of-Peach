# Checksummed SRAM Persistence Report (`v0.10.0`)

**Release:** `v0.10.0 — Super Mario Adventure Beyond Limits`  
**ROM SHA-256:** `2da376f5c5cd6f760c07e465bdba252fe1aa1f03e69f17dcabcff1ecd6e072b1`  
**Status:** `PLAYTHROUGH_PASS` (Verified on disk across `snes_faust`, cycle-accurate `bsnes`, and cold reboot)

---

## 1. Free Checksummed SRAM Discovery

A full 1 MiB binary scan of `roms/zelda_alttp.sfc` proved **zero** vanilla reads or writes in `$7E:F3E8..$7E:F4FD` — 268 bytes sitting **inside** ALTTP's native 1,280-byte per-slot save block (`$7E:F000..$7E:F4FF`, mirrored to `$70:0000/$0500/$0A00` with automatic additive complement checksum at `+$04FE` summing `639` 16-bit words to `0x5A5A`).

`v0.10.0` allocates **8 bytes** (`$7E:F400..$7E:F407`), automatically saved, mirrored, and checksum-verified by ALTTP's native hardware save routine (`$00:8929`):

| Address | Symbol | Purpose |
|---|---|---|
| `$7E:F400..$7E:F403` | `S_DRAGON_BITS` | 32-bit bitmask of unique Dragon Coins (`0..31`) — prevents infinite coin/magic farming |
| `$7E:F404` | `S_DRAGON_CNT` | Total unique Dragon Coins collected; every multiple of `5` grants a 1-UP full heal + Star refill |
| `$7E:F405` | `S_PIPE_DISC` | Discovered Overworld Warp Pipe Hubs bitmask |
| `$7E:F406` | `S_BOWSER_FLG` | Final King Bowser Phase 2 counterplay persistent flag |
| `$7E:F407` | `S_LUIGI_MODE` | `0 = Super Mario`, `1 = Playable Luigi` (`Down + L + R`) — persists across save/quit/reboot |

---

## 2. Live Disk Verification Evidence

1. **Multi-Hub Warp Pipe Save (`v0100_01` `faust` & `v0100_02` `bsnes`):**
   - After warping `0x2C -> 0x18 -> 0x03` and selecting `Sauvegarder et quitter.`, BOTH emulator cores flushed **bit-identical** `.srm` files: `dragon_bits = [3, 0, 0, 0]`, `dragon_cnt = 2`, `coins = 270`, `checksum_valid = True`.
2. **Playable Luigi & P-Switch Dragon Coin Save (`v0100_03`):**
   - After toggling Playable Luigi (`Down + L + R`), triggering Timed SMW P-Switch (`Up + L + R` -> Dragon Coin `#10`), throwing a Turn-Block (`+2` doubled coins -> `272`), and saving, `.srm` verified: `dragon_bits = [0, 4, 0, 0]` (bit `10` = `1 << 2` in byte 1), `dragon_cnt = 1`, `luigi_mode = 1`, `coins = 272`, `checksum_valid = True`.
3. **100% Blank-SRAM Cold Boot -> Save -> Cold Reboot (`v0100_12`):**
   - Booted zeroed 8 KiB `.srm`, created Slot 1, verified `slot1_magic = 0x55AA`, `checksum_stored = 0x6CEE == checksum_calc = 0x6CEE`, `hp_max = 24`, terminated emulator, **cold-rebooted**, and reloaded Slot 1 cleanly.
