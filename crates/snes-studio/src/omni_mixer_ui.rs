use std::collections::HashMap;
use std::path::Path;

use eframe::egui::{
    self, Color32, ColorImage, RichText, Stroke, TextureHandle, TextureOptions, Vec2,
};
use snes_core::{
    mixer::{
        execute_mix_recipe, generate_chaos_recipe, inspect_rom_for_mixing, suggest_presets,
        GameMixProfile, MixRecipe, MixReport, MixRule, MixSlotCategory,
    },
    rom::{RecognizedGame, SnesRom},
    swapper::SwapSession,
    universal::build_ips_patch,
};

pub struct OmniMixerState {
    pub loaded_roms: Vec<(SnesRom, GameMixProfile)>,
    pub base_idx: usize,
    /// target_slot_id -> (donor_alias, donor_source_id)
    pub assignments: HashMap<String, (String, String)>,
    pub custom_title: String,
    pub output_filename: String,
    pub filter_mode: usize, // 0 = Sémantique, 1 = Universel Brut, 2 = Actifs uniquement
    pub chaos_seed: u64,
    pub preview_textures: HashMap<String, TextureHandle>,
    pub last_report: Option<MixReport>,
    pub last_output_path: Option<String>,
    pub last_ips_path: Option<String>,
    pub status_msg: String,
}

impl OmniMixerState {
    pub fn new_auto_loaded() -> Self {
        let mut st = Self {
            loaded_roms: Vec::new(),
            base_idx: 0,
            assignments: HashMap::new(),
            custom_title: "SUPER MARIO HYRULE".to_string(),
            output_filename: "omni_mix_hybride.sfc".to_string(),
            filter_mode: 0,
            chaos_seed: 42,
            preview_textures: HashMap::new(),
            last_report: None,
            last_output_path: None,
            last_ips_path: None,
            status_msg: "Prêt — sélectionnez une ROM de base et mixez n'importe quels éléments depuis toutes vos ROMs !".to_string(),
        };

        // Auto-discover all .sfc / .smc ROMs in local `roms/` directory
        let candidate_dirs = ["roms"];
        for dir in candidate_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                let mut paths: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| {
                        p.extension()
                            .and_then(|x| x.to_str())
                            .map(|x| matches!(x.to_ascii_lowercase().as_str(), "sfc" | "smc"))
                            .unwrap_or(false)
                    })
                    .collect();
                paths.sort();
                for p in paths {
                    let _ = st.add_rom_from_path(&p);
                }
                if !st.loaded_roms.is_empty() {
                    break;
                }
            }
        }

        // Default base = Zelda ALTTP if loaded (so user immediately sees Mario & DK mixing options!)
        if let Some(z_idx) = st
            .loaded_roms
            .iter()
            .position(|(_, p)| p.recognized == RecognizedGame::ZeldaAlttp)
        {
            st.base_idx = z_idx;
        }

        // Pre-populate the first suggested preset so the mixing board looks alive on first launch!
        if st.loaded_roms.len() >= 2 {
            let presets = st.current_presets();
            if let Some(first) = presets.first() {
                st.apply_preset_rules(&first.rules, &first.custom_title);
                st.status_msg = format!(
                    "Preset '{}' pré-chargé ({} ROMs détectées dans roms/). Modifiez n'importe quel slot ou cliquez MIXER !",
                    first.name,
                    st.loaded_roms.len()
                );
            }
        }

        st
    }

    pub fn add_rom_from_path(&mut self, path: &Path) -> Result<(), String> {
        let path_str = path.to_string_lossy().to_string();
        if self.loaded_roms.iter().any(|(_, p)| p.rom_path == path_str) {
            return Ok(());
        }
        let bytes = std::fs::read(path).map_err(|e| format!("Lecture impossible: {}", e))?;
        let rom = SnesRom::from_bytes(path_str.clone(), &bytes)
            .map_err(|e| format!("ROM SNES invalide: {}", e))?;
        let base_alias = match rom.identify_game() {
            RecognizedGame::ZeldaAlttp => "zelda".to_string(),
            RecognizedGame::SuperMarioWorld => "smw".to_string(),
            RecognizedGame::DonkeyKongCountry => "dkc".to_string(),
            RecognizedGame::Unknown => {
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("rom")
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric())
                    .take(8)
                    .collect::<String>()
                    .to_ascii_lowercase();
                if stem.is_empty() {
                    format!("rom{}", self.loaded_roms.len() + 1)
                } else {
                    stem
                }
            }
        };
        let mut alias = base_alias.clone();
        let mut suffix = 2usize;
        while self.loaded_roms.iter().any(|(_, p)| p.alias == alias) {
            alias = format!("{}{}", base_alias, suffix);
            suffix += 1;
        }
        let prof = inspect_rom_for_mixing(&rom, alias, path_str);
        self.loaded_roms.push((rom, prof));
        Ok(())
    }

    pub fn donor_profiles(&self) -> Vec<GameMixProfile> {
        self.loaded_roms
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != self.base_idx)
            .map(|(_, (_, p))| p.clone())
            .collect()
    }

    pub fn current_presets(&self) -> Vec<snes_core::mixer::MixPreset> {
        let Some((_, base_prof)) = self.loaded_roms.get(self.base_idx) else {
            return Vec::new();
        };
        let donors = self.donor_profiles();
        suggest_presets(base_prof, &donors)
    }

    pub fn apply_preset_rules(&mut self, rules: &[MixRule], title: &str) {
        self.assignments.clear();
        for r in rules {
            self.assignments.insert(
                r.target_slot.clone(),
                (r.donor_alias.clone(), r.donor_source.clone()),
            );
        }
        self.custom_title = title.to_string();
    }

    pub fn build_current_recipe(&self) -> Option<MixRecipe> {
        let (_, base_prof) = self.loaded_roms.get(self.base_idx)?;
        let donors: Vec<(String, String)> = self
            .loaded_roms
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != self.base_idx)
            .map(|(_, (_, p))| (p.alias.clone(), p.rom_path.clone()))
            .collect();

        // Keep deterministic ordering matching target_slots order
        let mut rules = Vec::new();
        for slot in &base_prof.target_slots {
            if let Some((da, ds)) = self.assignments.get(&slot.id) {
                rules.push(MixRule {
                    target_slot: slot.id.clone(),
                    donor_alias: da.clone(),
                    donor_source: ds.clone(),
                });
            }
        }

        Some(MixRecipe {
            name: self.custom_title.clone(),
            custom_title: if self.custom_title.trim().is_empty() {
                None
            } else {
                Some(self.custom_title.trim().to_string())
            },
            base_rom: base_prof.rom_path.clone(),
            donors,
            rules,
        })
    }

    pub fn execute_mix_and_export(&mut self) {
        let Some(recipe) = self.build_current_recipe() else {
            self.status_msg = "⚠️ Chargez au moins une ROM de base !".to_string();
            return;
        };
        if recipe.rules.is_empty() {
            self.status_msg = "⚠️ Choisissez au moins une règle de mixage (ou un Preset / Chaos) !".to_string();
            return;
        }
        let orig_base = self.loaded_roms[self.base_idx].0.clone();
        let mut target_rom = orig_base.clone();
        let mut donor_map = HashMap::new();
        for (i, (rom, prof)) in self.loaded_roms.iter().enumerate() {
            if i != self.base_idx {
                donor_map.insert(prof.alias.clone(), rom.clone());
            }
        }
        let mut session = SwapSession::new();
        match execute_mix_recipe(&mut target_rom, &donor_map, &recipe, &mut session) {
            Ok(report) => {
                let out_path = if self.output_filename.trim().is_empty() {
                    "omni_mix_hybride.sfc".to_string()
                } else {
                    self.output_filename.trim().to_string()
                };
                let ips_path = if out_path.ends_with(".sfc") || out_path.ends_with(".smc") {
                    format!("{}.ips", &out_path[..out_path.len() - 4])
                } else {
                    format!("{}.ips", out_path)
                };
                let out_bytes = target_rom.to_sfc_bytes();
                if let Err(e) = std::fs::write(&out_path, &out_bytes) {
                    self.status_msg = format!("❌ Erreur écriture '{}': {}", out_path, e);
                    return;
                }
                let ips_bytes = build_ips_patch(&orig_base.data, &out_bytes);
                let _ = std::fs::write(&ips_path, &ips_bytes);
                self.status_msg = format!(
                    "✅ ROM mixée générée : {} ({} fusions, Checksum 0x{:04X} valide, IPS {} o, {} ms)",
                    out_path,
                    report.operations_applied,
                    report.checksum,
                    ips_bytes.len(),
                    report.elapsed_ms
                );
                self.last_output_path = Some(out_path);
                self.last_ips_path = Some(ips_path);
                self.last_report = Some(report);
            }
            Err(e) => {
                self.status_msg = format!("❌ Erreur de mixage : {}", e);
            }
        }
    }

    pub fn render_ui(&mut self, ui: &mut egui::Ui) {
        let accent_cyan = Color32::from_rgb(56, 189, 248);
        let accent_emerald = Color32::from_rgb(52, 211, 153);
        let accent_amber = Color32::from_rgb(251, 191, 36);
        let accent_purple = Color32::from_rgb(168, 85, 247);
        let card_bg = Color32::from_rgb(22, 27, 38);
        let card_border = Color32::from_rgb(48, 60, 82);

        // 1. HEADER BAR
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("★ SNES OMNI-MIXER MULTI-JEUX")
                    .size(20.0)
                    .strong()
                    .color(accent_cyan),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(RichText::new("💾 Sauver Recette .json").strong())
                    .clicked()
                {
                    if let Some(recipe) = self.build_current_recipe() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name("ma_recette_mix.json")
                            .add_filter("Recette JSON", &["json"])
                            .save_file()
                        {
                            match std::fs::write(&path, recipe.to_json()) {
                                Ok(_) => {
                                    self.status_msg =
                                        format!("💾 Recette sauvegardée : {}", path.display());
                                }
                                Err(e) => {
                                    self.status_msg = format!("❌ Erreur sauvegarde : {}", e);
                                }
                            }
                        }
                    }
                }
                if ui
                    .button(RichText::new("📂 Charger Recette .json").strong())
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Recette JSON", &["json"])
                        .pick_file()
                    {
                        match std::fs::read_to_string(&path)
                            .map_err(|e| e.to_string())
                            .and_then(|s| MixRecipe::from_json(&s))
                        {
                            Ok(rec) => {
                                let _ = self.add_rom_from_path(Path::new(&rec.base_rom));
                                for (_, dp) in &rec.donors {
                                    let _ = self.add_rom_from_path(Path::new(dp));
                                }
                                if let Some(bi) = self
                                    .loaded_roms
                                    .iter()
                                    .position(|(_, p)| p.rom_path == rec.base_rom)
                                {
                                    self.base_idx = bi;
                                }
                                self.apply_preset_rules(
                                    &rec.rules,
                                    rec.custom_title.as_deref().unwrap_or("OMNI MIX"),
                                );
                                self.status_msg = format!(
                                    "📂 Recette '{}' chargée ({} règles)",
                                    rec.name,
                                    rec.rules.len()
                                );
                            }
                            Err(e) => {
                                self.status_msg = format!("❌ Recette invalide : {}", e);
                            }
                        }
                    }
                }
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("➕ Ajouter une ROM (.sfc/.smc)")
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(37, 99, 235)),
                    )
                    .clicked()
                {
                    if let Some(paths) = rfd::FileDialog::new()
                        .add_filter("ROM SNES", &["sfc", "smc", "fig"])
                        .pick_files()
                    {
                        for p in paths {
                            match self.add_rom_from_path(&p) {
                                Ok(_) => {
                                    self.status_msg = format!("✓ ROM ajoutée : {}", p.display());
                                }
                                Err(e) => {
                                    self.status_msg = format!("❌ {}", e);
                                }
                            }
                        }
                    }
                }
            });
        });

        ui.add_space(6.0);

        // 2. MULTI-ROM LIBRARY STRIP
        egui::Frame::NONE
            .fill(Color32::from_rgb(16, 20, 30))
            .stroke(Stroke::new(1.0_f32, card_border))
            .corner_radius(8.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "📚 BIBLIOTHÈQUE ({} ROMs) :",
                            self.loaded_roms.len()
                        ))
                        .strong()
                        .color(Color32::from_rgb(203, 213, 225)),
                    );

                    let mut new_base: Option<usize> = None;
                    let mut remove_idx: Option<usize> = None;

                    for (idx, (_, prof)) in self.loaded_roms.iter().enumerate() {
                        let is_base = idx == self.base_idx;
                        let border_col = if is_base { accent_emerald } else { card_border };
                        let bg_col = if is_base {
                            Color32::from_rgb(16, 46, 38)
                        } else {
                            card_bg
                        };

                        egui::Frame::NONE
                            .fill(bg_col)
                            .stroke(Stroke::new(if is_base { 2.0_f32 } else { 1.0_f32 }, border_col))
                            .corner_radius(6.0)
                            .inner_margin(egui::Margin::symmetric(8, 5))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let role_badge = if is_base {
                                        RichText::new("🎯 BASE").strong().color(accent_emerald)
                                    } else {
                                        RichText::new(format!("🎁 {}", prof.alias.to_uppercase()))
                                            .strong()
                                            .color(accent_amber)
                                    };
                                    ui.label(role_badge);
                                    ui.label(
                                        RichText::new(&prof.title)
                                            .strong()
                                            .color(Color32::WHITE),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "{}Ko",
                                            prof.size_bytes / 1024
                                        ))
                                        .small()
                                        .color(Color32::from_rgb(148, 163, 184)),
                                    );
                                    if !is_base && ui.small_button("Base").clicked() {
                                        new_base = Some(idx);
                                    }
                                    if self.loaded_roms.len() > 1 && ui.small_button("✖").clicked() {
                                        remove_idx = Some(idx);
                                    }
                                });
                            });
                    }

                    if let Some(nb) = new_base {
                        self.base_idx = nb;
                        self.assignments.clear();
                        let presets = self.current_presets();
                        if let Some(first) = presets.first() {
                            self.apply_preset_rules(&first.rules, &first.custom_title);
                        }
                        self.status_msg = format!(
                            "🎯 Nouvelle ROM de base : {}",
                            self.loaded_roms[nb].1.title
                        );
                    }
                    if let Some(ri) = remove_idx {
                        self.loaded_roms.remove(ri);
                        if self.base_idx >= self.loaded_roms.len() {
                            self.base_idx = self.loaded_roms.len().saturating_sub(1);
                        }
                        self.assignments.clear();
                    }
                });
            });

        ui.add_space(6.0);

        if self.loaded_roms.is_empty() {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                egui::Frame::NONE
                    .fill(card_bg)
                    .stroke(Stroke::new(1.0_f32, accent_cyan))
                    .corner_radius(10.0)
                    .inner_margin(24.0)
                    .show(ui, |ui| {
                        ui.set_max_width(580.0);
                        ui.label(
                            RichText::new("Bienvenue dans SNES Omni-Mixer Multi-Jeux")
                                .size(20.0)
                                .strong()
                                .color(accent_cyan),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(
                                "Aucune ROM trouvée dans le dossier local 'roms/'.\n\
                                 Ajoutez vos propres dumps SNES (.sfc / .smc) ou essayez immédiatement\n\
                                 l'outil avec nos 2 ROMs de démonstration synthétiques intégrées :",
                            )
                            .color(Color32::from_rgb(203, 213, 225)),
                        );
                        ui.add_space(16.0);
                        ui.horizontal(|ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("⚡ Charger les 2 ROMs de démo intégrées")
                                            .strong()
                                            .size(14.0)
                                            .color(Color32::BLACK),
                                    )
                                    .fill(accent_emerald)
                                    .min_size(Vec2::new(290.0, 36.0)),
                                )
                                .clicked()
                            {
                                let (demo_a, demo_b) = snes_core::demo::generate_demo_rom_pair();
                                let prof_a = inspect_rom_for_mixing(&demo_a, "demo_a", "demo_a.sfc");
                                let prof_b = inspect_rom_for_mixing(&demo_b, "demo_b", "demo_b.sfc");
                                self.loaded_roms.push((demo_a, prof_a));
                                self.loaded_roms.push((demo_b, prof_b));
                                self.base_idx = 0;
                                self.filter_mode = 1; // Universal slots tab for unknown/demo ROMs
                                let presets = self.current_presets();
                                if let Some(first) = presets.first() {
                                    self.apply_preset_rules(&first.rules, &first.custom_title);
                                }
                                self.status_msg = "2 ROMs de démonstration synthétiques chargées — cliquez MIXER ou explorez les slots universels !".to_string();
                            }
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("➕ Ajouter une ROM (.sfc/.smc)")
                                            .strong()
                                            .size(14.0)
                                            .color(Color32::WHITE),
                                    )
                                    .fill(Color32::from_rgb(37, 99, 235))
                                    .min_size(Vec2::new(230.0, 36.0)),
                                )
                                .clicked()
                            {
                                if let Some(paths) = rfd::FileDialog::new()
                                    .add_filter("ROM SNES", &["sfc", "smc", "fig"])
                                    .pick_files()
                                {
                                    for p in paths {
                                        let _ = self.add_rom_from_path(&p);
                                    }
                                }
                            }
                        });
                    });
            });
            return;
        }

        // 3. PRESETS 1-CLIC & CHAOS RANDOMIZER BAR
        let presets = self.current_presets();
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("✨ PRESETS 1-CLIC :")
                    .strong()
                    .color(accent_amber),
            );
            for pr in &presets {
                let btn = egui::Button::new(
                    RichText::new(format!("★ {}", pr.name))
                        .small()
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(40, 48, 72));
                if ui.add(btn).on_hover_text(&pr.description).clicked() {
                    self.apply_preset_rules(&pr.rules, &pr.custom_title);
                    self.status_msg = format!(
                        "★ Preset '{}' appliqué ({} règles prêtes) !",
                        pr.name,
                        pr.rules.len()
                    );
                }
            }
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("🎲 Chaos Randomizer")
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(accent_purple),
                )
                .clicked()
            {
                self.chaos_seed = self.chaos_seed.wrapping_add(137);
                let base_p = self.loaded_roms[self.base_idx].1.clone();
                let donors = self.donor_profiles();
                let chaos = generate_chaos_recipe(&base_p, &donors, self.chaos_seed);
                self.apply_preset_rules(
                    &chaos.rules,
                    chaos.custom_title.as_deref().unwrap_or("OMNI CHAOS"),
                );
                self.status_msg = format!(
                    "🎲 Chaos Randomizer #{} généré ({} règles multi-jeux) !",
                    self.chaos_seed,
                    chaos.rules.len()
                );
            }
            if ui.button("↺ Réinitialiser").clicked() {
                self.assignments.clear();
                self.status_msg = "Toutes les règles ont été effacées.".to_string();
            }
        });

        ui.add_space(6.0);

        // 4. FILTER TABS & SLOT-BY-SLOT MIXING BOARD
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.filter_mode,
                0,
                RichText::new("⭐ Slots Sémantiques (Héros, Soldats, Ennemis, Objets, Audio)").strong(),
            );
            ui.selectable_value(
                &mut self.filter_mode,
                1,
                RichText::new("🌐 Slots Universels Bruts (GFX, Palettes, BRR — Tout Jeu)").strong(),
            );
            ui.selectable_value(
                &mut self.filter_mode,
                2,
                RichText::new(format!("🔥 Actifs uniquement ({})", self.assignments.len())).strong(),
            );
        });

        ui.separator();

        // Collect base slots & donor choices without holding mutable borrow across UI closures
        let base_profile = self.loaded_roms[self.base_idx].1.clone();
        let donors = self.donor_profiles();

        let ctx = ui.ctx().clone();
        let avail_h = (ui.available_height() - 125.0).max(200.0);

        egui::ScrollArea::vertical()
            .max_height(avail_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("omni_mixer_grid")
                    .num_columns(4)
                    .min_col_width(120.0)
                    .spacing([14.0, 6.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.label(RichText::new("Slot Cible (Base)").strong().color(accent_cyan));
                        ui.label(RichText::new("Catégorie").strong().color(accent_cyan));
                        ui.label(RichText::new("Source Donneuse (Multi-ROM)").strong().color(accent_emerald));
                        ui.label(RichText::new("Aperçu").strong().color(accent_amber));
                        ui.end_row();

                        for slot in &base_profile.target_slots {
                            let is_raw = matches!(
                                slot.category,
                                MixSlotCategory::RawGfx
                                    | MixSlotCategory::RawPalette
                                    | MixSlotCategory::RawAudio
                            );
                            let is_assigned = self.assignments.contains_key(&slot.id);
                            if self.filter_mode == 0 && is_raw {
                                continue;
                            }
                            if self.filter_mode == 1 && !is_raw {
                                continue;
                            }
                            if self.filter_mode == 2 && !is_assigned {
                                continue;
                            }

                            // Column 1: Slot label + description (wide enough for crisp single/two-line display)
                            ui.allocate_ui(Vec2::new(360.0, 38.0), |ui| {
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(format!("{} {}", slot.category.icon(), slot.label))
                                            .strong()
                                            .color(if is_assigned {
                                                accent_emerald
                                            } else {
                                                Color32::WHITE
                                            }),
                                    );
                                    ui.label(
                                        RichText::new(&slot.description)
                                            .small()
                                            .color(Color32::from_rgb(148, 163, 184)),
                                    );
                                });
                            });

                            // Column 2: Category badge
                            ui.label(
                                RichText::new(slot.category.label_fr())
                                    .small()
                                    .color(Color32::from_rgb(180, 196, 220)),
                            );

                            // Column 3: Dropdown of ALL compatible donor sources across ALL loaded donor ROMs
                            let cur_val = self.assignments.get(&slot.id).cloned();
                            let selected_text = match &cur_val {
                                Some((da, ds)) => {
                                    let lbl = donors
                                        .iter()
                                        .find(|d| d.alias == *da)
                                        .and_then(|d| d.donor_sources.iter().find(|s| s.id == *ds))
                                        .map(|s| format!("[{}] {}", da, s.label))
                                        .unwrap_or_else(|| format!("{}:{}", da, ds));
                                    lbl
                                }
                                None => "— Original (Base) —".to_string(),
                            };

                            egui::ComboBox::from_id_salt(format!("mix_slot_{}", slot.id))
                                .width(340.0)
                                .selected_text(selected_text)
                                .show_ui(ui, |ui| {
                                    if ui
                                        .selectable_label(cur_val.is_none(), "— Original (Base) —")
                                        .clicked()
                                    {
                                        self.assignments.remove(&slot.id);
                                    }
                                    for d in &donors {
                                        for src in &d.donor_sources {
                                            if !src.compatible_targets.contains(&slot.category) {
                                                continue;
                                            }
                                            let is_sel = cur_val
                                                .as_ref()
                                                .map(|(a, s)| a == &d.alias && s == &src.id)
                                                .unwrap_or(false);
                                            let opt_text = format!("[{}] {}", d.alias, src.label);
                                            if ui.selectable_label(is_sel, opt_text).clicked() {
                                                self.assignments.insert(
                                                    slot.id.clone(),
                                                    (d.alias.clone(), src.id.clone()),
                                                );
                                            }
                                        }
                                    }
                                });

                            // Column 4: Live pixel-art preview thumbnail
                            if let Some((da, ds)) = &cur_val {
                                let tex_key = format!("{}:{}", da, ds);
                                if !self.preview_textures.contains_key(&tex_key) {
                                    if let Some(src) = donors
                                        .iter()
                                        .find(|d| d.alias == *da)
                                        .and_then(|d| d.donor_sources.iter().find(|s| s.id == *ds))
                                    {
                                        if src.preview_rgba.len()
                                            == (src.preview_w * src.preview_h * 4) as usize
                                        {
                                            let cimg = ColorImage::from_rgba_unmultiplied(
                                                [src.preview_w as usize, src.preview_h as usize],
                                                &src.preview_rgba,
                                            );
                                            let handle = ctx.load_texture(
                                                tex_key.clone(),
                                                cimg,
                                                TextureOptions::NEAREST,
                                            );
                                            self.preview_textures.insert(tex_key.clone(), handle);
                                        }
                                    }
                                }
                                if let Some(tex) = self.preview_textures.get(&tex_key) {
                                    let sz = tex.size();
                                    let scale = if sz[1] > 20 { 1.5 } else { 2.5 };
                                    ui.image((tex.id(), Vec2::new(sz[0] as f32 * scale, sz[1] as f32 * scale)));
                                } else {
                                    ui.label("✓");
                                }
                            } else {
                                ui.label(
                                    RichText::new("•").color(Color32::from_rgb(75, 85, 99)),
                                );
                            }

                            ui.end_row();
                        }
                    });
            });

        ui.add_space(6.0);

        // 5. BOTTOM BUILD, EXPORT & PLAY BAR
        egui::Frame::NONE
            .fill(Color32::from_rgb(16, 22, 34))
            .stroke(Stroke::new(1.0_f32, accent_cyan))
            .corner_radius(8.0)
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("★ Titre SNES :").strong());
                    ui.add(
                        egui::TextEdit::singleline(&mut self.custom_title)
                            .desired_width(175.0)
                            .char_limit(21),
                    );
                    ui.label(RichText::new("💾 Sortie :").strong());
                    ui.add(
                        egui::TextEdit::singleline(&mut self.output_filename)
                            .desired_width(195.0),
                    );

                    let mix_btn = egui::Button::new(
                        RichText::new(format!(
                            "🚀 MIXER & EXPORTER ({} règles)",
                            self.assignments.len()
                        ))
                        .strong()
                        .size(14.0)
                        .color(Color32::BLACK),
                    )
                    .fill(accent_emerald)
                    .min_size(Vec2::new(230.0, 32.0));
                    if ui.add(mix_btn).clicked() {
                        self.execute_mix_and_export();
                    }

                    if let Some(ref out_p) = self.last_output_path {
                        let play_btn = egui::Button::new(
                            RichText::new("▶️ JOUER DANS MEDNAFEN")
                                .strong()
                                .size(13.0)
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(220, 38, 38))
                        .min_size(Vec2::new(185.0, 32.0));
                        if ui.add(play_btn).clicked() {
                            let _ = std::process::Command::new("mednafen")
                                .args([
                                    "-force_module",
                                    "snes_faust",
                                    "-video.fs",
                                    "0",
                                    "-snes_faust.xscale",
                                    "3",
                                    "-snes_faust.yscale",
                                    "3",
                                    out_p,
                                ])
                                .spawn();
                            self.status_msg = format!("▶️ Mednafen lancé sur {}", out_p);
                        }
                    }
                });

                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(&self.status_msg)
                            .small()
                            .color(Color32::from_rgb(226, 232, 240)),
                    );
                    if let Some(ref rep) = self.last_report {
                        ui.label(
                            RichText::new(format!(
                                " | Checksum: 0x{:04X} ({}) | CRC32: 0x{:08X}",
                                rep.checksum,
                                if rep.checksum_valid { "VALIDE ✓" } else { "INVALIDE" },
                                rep.crc32
                            ))
                            .small()
                            .strong()
                            .color(accent_emerald),
                        );
                    }
                });
            });
    }
}
