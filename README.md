# SNES Omni-Mixer Studio

[![Rust](https://img.shields.io/badge/rust-2021%20edition-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![CI](https://img.shields.io/badge/CI-GitHub%20Actions-brightgreen.svg)](#)

Application multiplateforme (**Windows, macOS, Linux**) écrite 100 % en **Rust** permettant de **mixer tout avec tout** sur Super Nintendo (`.sfc` / `.smc`) : chargez **N ROMs SNES** (LoROM, HiROM, ExHiROM), choisissez n'importe quelle ROM de base, combinez librement **Héros, Montures, Soldats, Ennemis, Objets, Décors, Palettes, Planches GFX universelles et Audio SPC700 BRR** issus de plusieurs jeux différents, sauvegardez vos recettes en `.json`, et exportez une **ROM hybride jouable `.sfc` + patch `.ips`** avec checksum SNES recalculé.

> **Avertissement légal** : Ce dépôt ne contient **aucune ROM commerciale** ni aucun asset sous copyright. Vous devez fournir vos propres dumps légaux dans le dossier `roms/` (ignoré automatiquement par `.gitignore`). Les tests unitaires passent sans aucune ROM commerciale.

---

## Architecture du Projet

Le workspace Cargo est divisé en deux crates :

1. **`crates/snes-core`** *(moteur de ROM hacking universel, CLI `snes-cli`)* :
   - **`mixer.rs`** : Moteur universel multi-jeux (`N ROMs → 1 ROM hybride`) — découverte automatique des slots sémantiques haute fidélité et des slots universels bruts (`gfx:0..15`, `pal:0..11`, `audio:0..11`) sur **toute ROM SNES**, Presets 1-clic, Chaos Randomizer déterministe (`--chaos --seed N`), recettes `.json` portables.
   - **`rom.rs`** : Détection LoROM (`$007FC0`), HiROM (`$00FFC0`), ExHiROM (`$40FFC0`), suppression d'en-têtes SMC 512 octets, recalcul checksum/complément SNES officiel.
   - **`disasm_dma.rs`** : Scanner de transferts DMA matériels (`$43x0..$43x5` → VRAM/CGRAM/OAM), entropie de Shannon, compression/décompression `LC_LZ2` (BE & LE), désassembleur 65c816.
   - **`graphics.rs`** : Décodeur/encodeur `2bpp`/`3bpp`/`4bpp`/`8bpp` & Mode 7, palettes `BGR555` ↔ `RGBA8888`, Smart Swap perceptuel (`remap_and_transcode_tiles`).
   - **`audio.rs`** : Scanner de samples SPC700 BRR (répertoire N-SPC + heuristique), décodeur BRR → PCM/WAV, encodeur & ré-échantillonneur ADPCM respectant les budgets mémoire.
   - **`universal.rs` / `deconstruct.rs` / `swapper.rs`** : Catalogue universel, déconstruction/reconstruction complète, export `.ips` & historique transactionnel.

2. **`crates/snes-studio`** *(interface graphique native `eframe` / `egui`)* :
   - Vue **SNES Omni-Mixer Multi-Jeux** : auto-découverte de `roms/`, ajout de ROMs illimitées, bascule de ROM de Base en 1 clic, Presets 1-clic, Chaos Randomizer, matrice complète par slot avec aperçus pixel-art temps réel, import/export `.json`, export `.sfc` + `.ips`, lancement direct dans Mednafen.

---

## Démarrage Rapide

### 1. Compiler en mode release
```bash
cargo build --release --workspace
```

### 2. Lancer l'interface graphique (`snes-studio`)
Déposez vos propres ROMs `.sfc`/`.smc` dans `roms/`, puis lancez :
```bash
cargo run --release -p snes-studio
```

### 3. Utiliser le CLI Multi-Jeux (`snes-cli`)
```bash
# Inspecter les slots, sources donneuses et presets de N ROMs
./target/release/snes-cli --mix-inspect roms/zelda_alttp.sfc roms/super_mario_world.sfc roms/donkey_kong_country.sfc

# Rejouer une recette d'exemple fournie dans recipes/
./target/release/snes-cli --mix --recipe recipes/donkey_hyrule_world.json --out donkey_hyrule_world.sfc --ips donkey_hyrule_world.ips

# Mix personnalisé règle par règle (slots sémantiques ou bruts gfx:N / pal:N / audio:N)
./target/release/snes-cli --mix \
  --base roms/super_mario_world.sfc \
  --donor zelda=roms/zelda_alttp.sfc \
  --donor dkc=roms/donkey_kong_country.sfc \
  --rule hero=zelda:link \
  --rule enemies=zelda:zelda_soldier \
  --rule items=zelda:zelda_items_pack \
  --rule mount=dkc:dk \
  --title "HYRULE MARIO WORLD" \
  --out hyrule_mario_world.sfc --ips hyrule_mario_world.ips

# Mode Chaos Multi-Jeux déterministe (compatible avec n'importe quelles ROMs SNES)
./target/release/snes-cli --mix \
  --base rom_a.sfc --donor b=rom_b.sfc --donor c=rom_c.sfc \
  --chaos --seed 42 --out chaos_mix.sfc
```

### Recettes d'exemple incluses (`recipes/`)
- `recipes/donkey_hyrule_world.json` — Crossover 3 jeux (Base Zelda + Donkey Kong DKC + Goombas/Objets SMW + Percussions DKC)
- `recipes/hyrule_mario_world.json` — Crossover 3 jeux (Base SMW + Link & Soldats Zelda + Monture Donkey Kong)
- `recipes/dk_mario_world.json` — Base SMW + Donkey Kong jouable + Percussions DKC
- `recipes/mario_kong_country.json` — Base DKC (4 Mo HiROM) + Mario SMW + Percussions Zelda

---

## Tests

```bash
cargo test --workspace
```

Les tests fonctionnent entièrement sans ROM commerciale (en utilisant les ROMs de démonstration synthétiques intégrées), et activent automatiquement des tests d'intégration supplémentaires si vos propres dumps sont présents dans `roms/`.

---

## Licence

Distribué sous licence **MIT** — voir [LICENSE](LICENSE).
