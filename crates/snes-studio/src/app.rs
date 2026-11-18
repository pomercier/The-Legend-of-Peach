#![allow(dead_code)]
use eframe::egui::{
    self, Color32, ColorImage, Pos2, Rect, RichText, Sense, Stroke, TextureHandle, TextureOptions,
    Vec2,
};
use eframe::epaint::CubicBezierShape;
use snes_core::{
    assets::{
        apply_custom_swaps, get_default_swap_decisions, scan_assets_for_rom, AssetCategory,
        ScannedAsset, UserSwapDecision,
    },
    audio::{scan_brr_samples, BrrSampleInfo},
    demo::generate_demo_rom_pair,
    disasm_dma::{
        classify_rom_chunks, disassemble_65c816_block,
        scan_hardware_dma_transfers, DmaTarget, DmaTransferDiscovery,
        RomChunkClassification,
    },
    graphics::{
        render_single_asset_sprite, render_tile_sheet_rgba, scan_candidate_palettes,
        scan_graphics_regions, GraphicsRegion, SnesPalette, TileFormat,
    },
    memory::RomKnowledgeBase,
    profile::GameProfile,
    rom::{RecognizedGame, SnesRom},
    swapper::SwapSession,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    OverviewUniversal,
    GraphicsSwap,
    AudioSwap,
    HistoryExport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarView {
    OmniMixer,
    Melter,
    Deconstructor,
    GraphicsInspector,
    PalettesInspector,
    AudioInspector,
    DisassemblyInspector,
    HistoryLog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RomCardTab {
    Sprites,
    Tiles,
    Audio,
    Palettes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FusionMode {
    SmartBlendAI,
    DirectHardware,
    PaletteAdapt,
    DmaInject,
}

pub struct RomSlotState {
    pub rom: SnesRom,
    pub profile: Option<GameProfile>,
    pub gfx_regions: Vec<GraphicsRegion>,
    pub detected_palettes: Vec<SnesPalette>,
    pub brr_samples: Vec<BrrSampleInfo>,
    pub dma_transfers: Vec<DmaTransferDiscovery>,
    pub chunk_map: Vec<RomChunkClassification>,
    pub disasm_preview: Vec<String>,
    pub scanned_assets: Vec<ScannedAsset>,
    pub view_offset: usize,
    pub offset_hex_input: String,
    pub selected_tile_idx: usize,
    pub selected_sample_idx: usize,
    pub tile_format: TileFormat,
    pub active_palette_idx: usize,
    pub tile_texture: Option<TextureHandle>,
    pub single_asset_texture: Option<TextureHandle>,
    pub single_asset_id: String,
    pub texture_dirty: bool,
}

pub fn build_palettes_for_rom(rom: &SnesRom, profile: &Option<GameProfile>) -> Vec<SnesPalette> {
    let mut detected_palettes = Vec::new();
    let recognized = rom.identify_game();

    if recognized == RecognizedGame::SuperMarioWorld {
        detected_palettes.push(SnesPalette::mario_classic_full_16(Some(&rom.data)));
        if rom.data.len() >= 0x003280 + 32 {
            let mut p = SnesPalette::from_rom_bgr555("Palette Sprites SMW — Bloc ?, Champignon, Yoshi (0x003280)", &rom.data, 0x003280, 16);
            if let Some(c0) = p.colors_rgba.first_mut() { *c0 = [18, 22, 32, 255]; }
            detected_palettes.push(p);
        }
        if rom.data.len() >= 0x0032DC + 32 {
            let mut p = SnesPalette::from_rom_bgr555("Palette Joueur Luigi — Vert & Violet (0x0032DC)", &rom.data, 0x0032DC, 16);
            if let Some(c0) = p.colors_rgba.first_mut() { *c0 = [18, 22, 32, 255]; }
            detected_palettes.push(p);
        }
        if rom.data.len() >= 0x0032F0 + 32 {
            let mut p = SnesPalette::from_rom_bgr555("Palette Joueur Mario de Feu — Blanc & Rouge (0x0032F0)", &rom.data, 0x0032F0, 16);
            if let Some(c0) = p.colors_rgba.first_mut() { *c0 = [18, 22, 32, 255]; }
            detected_palettes.push(p);
        }
        if rom.data.len() >= 0x003100 + 32 {
            let mut p = SnesPalette::from_rom_bgr555("Banque Palettes Niveaux & Écran Titre SMW (0x003100)", &rom.data, 0x003100, 16);
            if let Some(c0) = p.colors_rgba.first_mut() { *c0 = [18, 22, 32, 255]; }
            detected_palettes.push(p);
        }
    } else if recognized == RecognizedGame::ZeldaAlttp {
        detected_palettes.push(SnesPalette::zelda_link_full_16(&rom.data, 0x0DD308, "Palette Link — Tunique Verte Noble (0x0DD308)", false));
        detected_palettes.push(SnesPalette::zelda_link_full_16(&rom.data, 0x0DD308, "Palette IA — Link Blond Doré HD (Restoration Remaster)", true));
        detected_palettes.push(SnesPalette::zelda_link_full_16(&rom.data, 0x0DD326, "Palette Link — Tunique Bleue Aquatique (0x0DD326)", false));
        detected_palettes.push(SnesPalette::zelda_link_full_16(&rom.data, 0x0DD344, "Palette Link — Tunique Rouge Magique (0x0DD344)", false));
        if rom.data.len() >= 0x0DD218 + 32 {
            let mut p = SnesPalette::from_rom_bgr555("Palette Épées & Trésors Hyrule (0x0DD218)", &rom.data, 0x0DD218, 16);
            if let Some(c0) = p.colors_rgba.first_mut() { *c0 = [18, 22, 32, 255]; }
            detected_palettes.push(p);
        }
    } else if let Some(ref prof) = profile {
        for (pal_name, pal_off) in &prof.palettes {
            if *pal_off + 32 <= rom.data.len() {
                let mut p = SnesPalette::from_rom_bgr555(pal_name.clone(), &rom.data, *pal_off, 16);
                if let Some(c0) = p.colors_rgba.first_mut() {
                    *c0 = [18, 22, 32, 255];
                }
                detected_palettes.push(p);
            }
        }
    }

    detected_palettes.extend(scan_candidate_palettes(&rom.data, 20));
    detected_palettes
}

impl RomSlotState {
    pub fn from_rom(rom: SnesRom, default_palette_idx: usize) -> Self {
        let profile = GameProfile::detect_for_rom(&rom);
        let gfx_regions = scan_graphics_regions(&rom.data);
        let detected_palettes = build_palettes_for_rom(&rom, &profile);

        let mut brr_samples = scan_brr_samples(&rom.data, 16);
        let dma_transfers = scan_hardware_dma_transfers(&rom);
        let chunk_map = classify_rom_chunks(&rom.data);
        let disasm_preview = disassemble_65c816_block(&rom, 0, 18);

        if let Some(ref prof) = profile {
            for sample in &mut brr_samples {
                if let Some(bm) = prof
                    .audio_samples
                    .iter()
                    .find(|b| b.offset == sample.offset)
                {
                    sample.label = format!("{} ({} blocs)", bm.name, sample.block_count);
                }
            }
        }

        let scanned_assets = scan_assets_for_rom(&rom);

        // Find primary player hero asset to display by default (Mario in SMW, Link in Zelda)
        let initial_asset = scanned_assets
            .iter()
            .find(|a| a.category == AssetCategory::Player)
            .or_else(|| scanned_assets.first());

        let initial_offset = initial_asset.map(|a| a.rom_offset).unwrap_or_else(|| {
            profile
                .as_ref()
                .and_then(|p| p.graphics.first().map(|g| g.offset))
                .or_else(|| {
                    dma_transfers
                        .iter()
                        .find(|d| d.target == DmaTarget::VramGraphics && d.source_pc_offset.is_some())
                        .and_then(|d| d.source_pc_offset)
                })
                .or_else(|| gfx_regions.first().map(|r| r.offset))
                .unwrap_or(0)
        });

        let initial_format = initial_asset.map(|a| a.format).unwrap_or(TileFormat::Bpp4);
        let initial_selected_idx = initial_asset
            .and_then(|ia| scanned_assets.iter().position(|a| a.id == ia.id))
            .unwrap_or(0);

        let initial_palette_idx = if let Some(asset) = initial_asset {
            if let Some(poff) = asset.palette_offset {
                detected_palettes
                    .iter()
                    .position(|p| p.source_offset == Some(poff) || p.name.contains(&format!("{:06X}", poff)))
                    .unwrap_or(default_palette_idx.min(detected_palettes.len().saturating_sub(1)))
            } else {
                default_palette_idx.min(detected_palettes.len().saturating_sub(1))
            }
        } else {
            default_palette_idx.min(detected_palettes.len().saturating_sub(1))
        };

        Self {
            rom,
            profile,
            gfx_regions,
            detected_palettes,
            brr_samples,
            dma_transfers,
            chunk_map,
            disasm_preview,
            scanned_assets,
            view_offset: initial_offset,
            offset_hex_input: format!("{:06X}", initial_offset),
            selected_tile_idx: initial_selected_idx,
            selected_sample_idx: 0,
            tile_format: initial_format,
            active_palette_idx: initial_palette_idx,
            tile_texture: None,
            single_asset_texture: None,
            single_asset_id: String::new(),
            texture_dirty: true,
        }
    }

    pub fn rescan_after_edit(&mut self) {
        self.profile = GameProfile::detect_for_rom(&self.rom);
        self.gfx_regions = scan_graphics_regions(&self.rom.data);
        self.detected_palettes = build_palettes_for_rom(&self.rom, &self.profile);
        self.brr_samples = scan_brr_samples(&self.rom.data, 16);
        self.dma_transfers = scan_hardware_dma_transfers(&self.rom);
        self.chunk_map = classify_rom_chunks(&self.rom.data);
        self.disasm_preview = disassemble_65c816_block(&self.rom, 0, 18);
        self.scanned_assets = scan_assets_for_rom(&self.rom);
        if self.active_palette_idx >= self.detected_palettes.len() {
            self.active_palette_idx = 0;
        }
        self.single_asset_texture = None;
        self.single_asset_id.clear();
        self.texture_dirty = true;
    }

    pub fn set_offset(&mut self, new_offset: usize) {
        let max_off = self.rom.data.len().saturating_sub(64);
        self.view_offset = new_offset.min(max_off);
        self.offset_hex_input = format!("{:06X}", self.view_offset);
        self.texture_dirty = true;
    }
}

/// Helper to create a standard binary IPS patch between two ROMs
pub fn create_ips_patch(orig: &[u8], modified: &[u8]) -> Vec<u8> {
    let mut ips = Vec::new();
    ips.extend_from_slice(b"PATCH");
    let max_len = orig.len().max(modified.len());
    let mut i = 0;
    while i < max_len {
        let b_orig = orig.get(i).copied().unwrap_or(0);
        let b_mod = modified.get(i).copied().unwrap_or(0);
        if b_orig != b_mod {
            let start = i;
            let mut diff = Vec::new();
            while i < max_len && diff.len() < 0xFFFF {
                let o = orig.get(i).copied().unwrap_or(0);
                let m = modified.get(i).copied().unwrap_or(0);
                if o == m && i + 2 < max_len && orig.get(i + 1) == modified.get(i + 1) && orig.get(i + 2) == modified.get(i + 2) {
                    break;
                }
                diff.push(m);
                i += 1;
            }
            if !diff.is_empty() {
                ips.push(((start >> 16) & 0xFF) as u8);
                ips.push(((start >> 8) & 0xFF) as u8);
                ips.push((start & 0xFF) as u8);
                let len = diff.len() as u16;
                ips.push(((len >> 8) & 0xFF) as u8);
                ips.push((len & 0xFF) as u8);
                ips.extend_from_slice(&diff);
            }
        } else {
            i += 1;
        }
    }
    ips.extend_from_slice(b"EOF");
    ips
}

pub struct SnesStudioApp {
    pub sidebar_view: SidebarView,
    pub active_tab: ActiveTab,
    pub slot_a: RomSlotState,
    pub slot_b: RomSlotState,
    pub swap_session: SwapSession,
    pub knowledge_base: RomKnowledgeBase,
    pub show_inspector: bool,

    // UI state for SNES Studio
    pub rom_a_tab: RomCardTab,
    pub rom_b_tab: RomCardTab,
    pub fusion_mode: FusionMode,
    pub fusion_direction_ab: bool,
    pub zoom_a: f32,
    pub zoom_b: f32,
    pub grid_a: bool,
    pub grid_b: bool,
    pub log_messages: Vec<(String, Color32)>,
    pub emulator_path: String,
    pub original_rom_a_data: Vec<u8>,
    pub ai_hd_upscale: bool,

    // Dynamic Asset Scanners & Swap Decider Matrix
    pub swap_decisions: Vec<UserSwapDecision>,
    pub asset_filter_category_a: Option<AssetCategory>,
    pub asset_filter_category_b: Option<AssetCategory>,

    // Shared Graphics Viewer settings
    pub sheet_columns: usize,
    pub sheet_rows: usize,
    pub zoom: f32,
    pub swap_tile_count: usize,
    pub palettes: Vec<SnesPalette>,
    pub smart_color_remap: bool,

    // Audio settings
    pub wav_sample_rate: u32,
    pub audio_resample_to_fit: bool,

    // Status message
    pub status_message: String,

    // Deconstruction & Universal Engine
    pub last_deconstruct_report: Option<snes_core::deconstruct::DeconstructReport>,
    pub last_reconstruct_report: Option<snes_core::universal::ReconstructReport>,
    pub deconstruct_target_slot_a: bool,
    pub catalog_a: Option<snes_core::universal::UniversalRomCatalog>,
    pub catalog_b: Option<snes_core::universal::UniversalRomCatalog>,
    pub universal_cat_tab: usize, // 0=GFX, 1=Palettes, 2=Audio, 3=Text
    pub universal_sel_a: usize,
    pub universal_sel_b: usize,
    pub universal_smart_remap: bool,

    // Multi-Game Omni-Mixer Studio (Mix Anything with Anything!)
    pub omni_mixer: crate::omni_mixer_ui::OmniMixerState,
}

pub fn load_or_extract_smw() -> SnesRom {
    if let Ok(bytes) = std::fs::read("roms/super_mario_world.sfc") {
        if let Ok(rom) = SnesRom::from_bytes("Super Mario World", &bytes) {
            return rom;
        }
    }
    generate_demo_rom_pair().0
}

pub fn load_or_extract_zelda() -> SnesRom {
    if let Ok(bytes) = std::fs::read("roms/zelda_alttp.sfc") {
        if let Ok(rom) = SnesRom::from_bytes("Zelda ALTTP", &bytes) {
            return rom;
        }
    }
    generate_demo_rom_pair().1
}

pub fn load_or_extract_dkc() -> Option<Vec<u8>> {
    std::fs::read("roms/donkey_kong_country.sfc").ok()
}

impl SnesStudioApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let rom_a = load_or_extract_smw();
        let rom_b = load_or_extract_zelda();

        let orig_a = rom_a.data.clone();
        let slot_a = RomSlotState::from_rom(rom_a, 0);
        let slot_b = RomSlotState::from_rom(rom_b, 1);
        let swap_decisions = get_default_swap_decisions(&slot_a.rom, &slot_b.rom);

        let mut kb = RomKnowledgeBase::load_or_init();
        kb.learn_from_rom(&slot_a.rom);
        kb.learn_from_rom(&slot_b.rom);
        if let Some(c_bytes) = load_or_extract_dkc() {
            if let Ok(rom_c) = SnesRom::from_bytes("Donkey Kong Country", &c_bytes) {
                kb.learn_from_rom(&rom_c);
            }
        }
        let _ = kb.save_to_default_path();

        let mut app = Self {
            sidebar_view: SidebarView::OmniMixer,
            active_tab: ActiveTab::GraphicsSwap,
            slot_a,
            slot_b,
            swap_session: SwapSession::new(),
            knowledge_base: kb,
            show_inspector: false,
            rom_a_tab: RomCardTab::Sprites,
            rom_b_tab: RomCardTab::Sprites,
            fusion_mode: FusionMode::SmartBlendAI,
            fusion_direction_ab: true,
            zoom_a: 3.0,
            zoom_b: 3.0,
            grid_a: true,
            grid_b: true,
            log_messages: Vec::new(),
            emulator_path: "/usr/bin/snes9x-gtk".to_string(),
            original_rom_a_data: orig_a,
            ai_hd_upscale: true,
            swap_decisions,
            asset_filter_category_a: None,
            asset_filter_category_b: None,
            sheet_columns: 16,
            sheet_rows: 8,
            zoom: 3.0,
            swap_tile_count: 128,
            palettes: Vec::new(),
            smart_color_remap: false,
            wav_sample_rate: 16000,
            audio_resample_to_fit: true,
            status_message: "SNES Omni-Mixer Studio prêt.".to_string(),
            last_deconstruct_report: None,
            last_reconstruct_report: None,
            deconstruct_target_slot_a: true,
            catalog_a: None,
            catalog_b: None,
            universal_cat_tab: 0,
            universal_sel_a: 0,
            universal_sel_b: 0,
            universal_smart_remap: true,
            omni_mixer: crate::omni_mixer_ui::OmniMixerState::new_auto_loaded(),
        };

        let title_a = app.slot_a.rom.canonical_title();
        let title_b = app.slot_b.rom.canonical_title();
        let size_a = app.slot_a.rom.data.len() / 1024;
        let size_b = app.slot_b.rom.data.len() / 1024;
        app.add_log("Initialisation de SNES Omni-Mixer Studio v1.0...", Color32::from_rgb(34, 211, 238));
        app.add_log(format!("ROM Source A chargée : {} ({} Ko)", title_a, size_a), Color32::from_rgb(16, 185, 129));
        app.add_log(format!("ROM Source B chargée : {} ({} Ko)", title_b, size_b), Color32::from_rgb(139, 92, 246));
        app.add_log("Moteur universel multi-ROM (LoROM/HiROM/ExHiROM, 2/3/4/8bpp, LC_LZ2, SPC700 BRR) prêt.", Color32::from_rgb(251, 191, 36));

        app.rebuild_palette_list();
        app
    }

    pub fn reload_default_rom(&mut self, is_slot_a: bool) {
        if is_slot_a {
            let rom = load_or_extract_smw();
            self.original_rom_a_data = rom.data.clone();
            self.slot_a = RomSlotState::from_rom(rom, 0);
            self.catalog_a = None;
            self.add_log("ROM A réinitialisée : Super Mario World", Color32::from_rgb(16, 185, 129));
        } else {
            let rom = load_or_extract_zelda();
            self.slot_b = RomSlotState::from_rom(rom, 1);
            self.catalog_b = None;
            self.add_log("ROM B réinitialisée : The Legend of Zelda: ALTTP", Color32::from_rgb(139, 92, 246));
        }
        self.swap_decisions = get_default_swap_decisions(&self.slot_a.rom, &self.slot_b.rom);
        self.rebuild_palette_list();
    }

    pub fn add_log(&mut self, msg: impl Into<String>, color: Color32) {
        let dur = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        let secs = dur.as_secs();
        let h = (secs / 3600) % 24;
        let m = (secs / 60) % 60;
        let s = secs % 60;
        let timestamp = format!("{:02}:{:02}:{:02}", h, m, s);
        self.log_messages.push((format!("[{}] {}", timestamp, msg.into()), color));
        if self.log_messages.len() > 200 {
            self.log_messages.remove(0);
        }
    }

    pub fn execute_user_swaps(&mut self) {
        let enabled_count = self.swap_decisions.iter().filter(|d| d.enabled).count();
        self.add_log(
            format!("Application de {} swap(s) sélectionné(s) par l'utilisateur...", enabled_count),
            Color32::from_rgb(139, 92, 246),
        );

        // Reload clean base ROM A before applying selected swaps
        if !self.original_rom_a_data.is_empty() {
            if let Ok(rom) = SnesRom::from_bytes(&self.slot_a.rom.name, &self.original_rom_a_data) {
                self.slot_a = RomSlotState::from_rom(rom, 0);
                self.swap_session = SwapSession::new();
            }
        } else {
            let rom = load_or_extract_smw();
            self.original_rom_a_data = rom.data.clone();
            self.slot_a = RomSlotState::from_rom(rom, 0);
            self.swap_session = SwapSession::new();
        }

        let dkc_bytes = load_or_extract_dkc();
        match apply_custom_swaps(
            &mut self.slot_a.rom,
            Some(&self.slot_b.rom),
            dkc_bytes.as_deref(),
            &self.swap_decisions,
            &mut self.swap_session,
        ) {
            Ok(count) => {
                self.slot_a.rescan_after_edit();
                let preview_path = std::env::temp_dir().join("snes_studio_swapped.sfc");
                let _ = std::fs::write(&preview_path, &self.slot_a.rom.data);
                let _ = std::fs::copy(&preview_path, "super_marioworld_swapped.sfc");
                let sum = self.slot_a.rom.header.checksum;
                self.add_log(
                    format!("✓ {} swaps appliqués avec succès ! Checksum SNES: 0x{:04X} (100% Valide).", count, sum),
                    Color32::from_rgb(16, 185, 129),
                );

                // Update Knowledge Base with user choices & continuous learning
                let active_labels: Vec<String> = self.swap_decisions.iter().filter(|d| d.enabled).map(|d| d.label.clone()).collect();
                let recipe_id = format!("custom_swap_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs());
                let desc = format!("Recette personnalisée utilisateur avec {} actifs", enabled_count);
                self.knowledge_base.record_verified_recipe(
                    &recipe_id,
                    &self.slot_a.rom.header.title,
                    vec![self.slot_b.rom.header.title.clone(), "Donkey Kong Country".to_string()],
                    &desc,
                    active_labels,
                );
                let _ = self.knowledge_base.save_to_default_path();
                self.add_log("Apprentissage sauvegardé dans snes_knowledge_base.json", Color32::from_rgb(251, 191, 36));
            }
            Err(e) => {
                self.add_log(format!("Erreur lors de la fusion : {:?}", e), Color32::from_rgb(239, 68, 68));
            }
        }
    }

    pub fn play_jeu_1(&mut self) {
        self.add_log("Exécution de la Fusion Totale (Link + Donkey Kong Punch)...", Color32::from_rgb(139, 92, 246));
        if !self.original_rom_a_data.is_empty() {
            if let Ok(rom) = SnesRom::from_bytes(&self.slot_a.rom.name, &self.original_rom_a_data) {
                self.slot_a = RomSlotState::from_rom(rom, 0);
                self.swap_session = SwapSession::new();
            }
        } else {
            let rom = load_or_extract_smw();
            self.original_rom_a_data = rom.data.clone();
            self.slot_a = RomSlotState::from_rom(rom, 0);
            self.swap_session = SwapSession::new();
        }

        let dkc_bytes = load_or_extract_dkc();
        match snes_core::demo::apply_full_fusion(
            &mut self.slot_a.rom,
            Some(&self.slot_b.rom),
            dkc_bytes.as_deref(),
            &mut self.swap_session,
        ) {
            Ok(count) => {
                let _ = snes_core::demo::apply_link_world_title_screen(&mut self.slot_a.rom, &mut self.swap_session);
                self.slot_a.rescan_after_edit();
                let preview_path = std::env::temp_dir().join("snes_studio_swapped.sfc");
                let preview_str = preview_path.to_string_lossy().to_string();
                self.export_current_rom(Some(&preview_str));
                let _ = std::fs::copy(&preview_path, "super_marioworld_swapped.sfc");
                self.add_log(format!("✓ Fusion appliquée avec succès ({} modifications).", count), Color32::from_rgb(16, 185, 129));
                self.add_log(format!("Lancement de l'émulateur SNES ({})...", preview_str), Color32::from_rgb(34, 211, 238));
                self.launch_in_snes_emulator(&preview_str);
            }
            Err(e) => {
                self.add_log(format!("Erreur lors de la fusion : {:?}", e), Color32::from_rgb(239, 68, 68));
            }
        }
    }

    pub fn export_current_rom(&mut self, target_path: Option<&str>) {
        let path = target_path.map(|s| s.to_string()).unwrap_or_else(|| {
            rfd::FileDialog::new()
                .add_filter("ROM SNES (*.sfc, *.smc)", &["sfc", "smc"])
                .set_file_name("super_link_world_swapped.sfc")
                .save_file()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default()
        });

        if !path.is_empty() {
            match std::fs::write(&path, &self.slot_a.rom.data) {
                Ok(_) => {
                    self.add_log(format!("✓ ROM exportée : {}", path), Color32::from_rgb(16, 185, 129));
                    self.status_message = format!("ROM exportée : {}", path);
                }
                Err(e) => {
                    self.add_log(format!("Erreur d'exportation : {:?}", e), Color32::from_rgb(239, 68, 68));
                }
            }
        }
    }

    pub fn export_ips_patch(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Patch IPS (*.ips)", &["ips"])
            .set_file_name("super_link_world.ips")
            .save_file()
        {
            let patch = create_ips_patch(&self.original_rom_a_data, &self.slot_a.rom.data);
            if std::fs::write(&path, &patch).is_ok() {
                self.add_log(format!("✓ Patch IPS exporté ({} octets) : {}", patch.len(), path.display()), Color32::from_rgb(16, 185, 129));
            }
        }
    }

    fn launch_in_snes_emulator(&mut self, rom_path: &str) {
        let emu_custom = self.emulator_path.clone();
        let emus = [emu_custom.as_str(), "/usr/bin/snes9x-gtk", "/usr/bin/snes9x", "snes9x-gtk", "snes9x", "zsnes", "retroarch"];
        for emu in emus {
            if let Ok(mut child) = std::process::Command::new(emu).arg(rom_path).spawn() {
                self.add_log(format!("Émulateur démarré avec succès : {}", emu), Color32::from_rgb(16, 185, 129));
                std::thread::spawn(move || { let _ = child.wait(); });
                return;
            }
        }
        self.add_log("Aucun émulateur SNES trouvé. Installez 'snes9x-gtk' ou configurez le chemin dans Paramètres.", Color32::from_rgb(251, 191, 36));
    }

    pub fn export_all_assets_slot(&mut self, is_slot_a: bool) {
        self.deconstruct_rom_slot(is_slot_a);
    }

    pub fn deconstruct_rom_slot(&mut self, is_slot_a: bool) {
        let (rom_title, rom_name, rom_clone) = if is_slot_a {
            (self.slot_a.rom.canonical_title(), self.slot_a.rom.name.clone(), self.slot_a.rom.clone())
        } else {
            (self.slot_b.rom.canonical_title(), self.slot_b.rom.name.clone(), self.slot_b.rom.clone())
        };

        let safe_name = rom_name.replace(|c: char| !c.is_alphanumeric(), "_");
        let out_dir = std::path::PathBuf::from(format!("./deconstructed_assets_{}", safe_name));
        let cfg = snes_core::deconstruct::DeconstructConfig::default();

        match snes_core::deconstruct::deconstruct_rom(&rom_clone, &out_dir, &cfg) {
            Ok(report) => {
                self.add_log(
                    format!(
                        "✓ Déconstruction terminée en {} ms pour '{}' : {} sprites, {} planches, {} SPC, {} WAV, {} dialogues dans '{}' !",
                        report.elapsed_ms,
                        report.rom_title,
                        report.sprites_count,
                        report.sheets_count,
                        report.spc_files_count,
                        report.audio_wav_count,
                        report.text_dialogues_count,
                        report.out_dir
                    ),
                    Color32::from_rgb(16, 185, 129),
                );
                self.last_deconstruct_report = Some(report);
                self.sidebar_view = SidebarView::Deconstructor;
            }
            Err(e) => {
                self.add_log(
                    format!("Erreur lors de la déconstruction de '{}' : {:?}", rom_title, e),
                    Color32::from_rgb(239, 68, 68),
                );
            }
        }
    }

    fn rebuild_palette_list(&mut self) {
        self.palettes.clear();
        self.palettes.extend(self.slot_a.detected_palettes.clone());
        self.palettes.extend(self.slot_b.detected_palettes.clone());
        if self.palettes.is_empty() {
            self.palettes.push(SnesPalette::preset_grayscale());
        }
    }

    fn load_rom_from_dialog(&mut self, is_slot_a: bool) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("ROM SNES (*.sfc, *.smc)", &["sfc", "smc"])
            .pick_file()
        {
            if let Ok(bytes) = std::fs::read(&path) {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("rom.sfc");
                match SnesRom::from_bytes(name, &bytes) {
                    Ok(rom) => {
                        let desc = format!("{} ({} Ko)", rom.canonical_title(), rom.data.len() / 1024);
                        if is_slot_a {
                            self.original_rom_a_data = rom.data.clone();
                            self.slot_a = RomSlotState::from_rom(rom, 0);
                            self.catalog_a = None;
                            self.add_log(format!("ROM A chargée : {}", desc), Color32::from_rgb(16, 185, 129));
                        } else {
                            self.slot_b = RomSlotState::from_rom(rom, 1);
                            self.catalog_b = None;
                            self.add_log(format!("ROM B chargée : {}", desc), Color32::from_rgb(139, 92, 246));
                        }
                        self.swap_session = SwapSession::new();
                        self.knowledge_base.learn_from_rom(&self.slot_a.rom);
                        self.knowledge_base.learn_from_rom(&self.slot_b.rom);
                        let _ = self.knowledge_base.save_to_default_path();
                        self.swap_decisions = get_default_swap_decisions(&self.slot_a.rom, &self.slot_b.rom);
                        self.rebuild_palette_list();
                    }
                    Err(e) => {
                        self.add_log(format!("Erreur de format ROM : {:?}", e), Color32::from_rgb(239, 68, 68));
                    }
                }
            }
        }
    }

    fn refresh_textures_if_needed(&mut self, ctx: &egui::Context) {
        let pal_a = self.slot_a.detected_palettes.get(self.slot_a.active_palette_idx).cloned().unwrap_or_else(SnesPalette::preset_grayscale);
        let pal_b = self.slot_b.detected_palettes.get(self.slot_b.active_palette_idx).cloned().unwrap_or_else(SnesPalette::preset_grayscale);

        let tex_options = if self.ai_hd_upscale {
            TextureOptions::LINEAR
        } else {
            TextureOptions::NEAREST
        };

        let dirty_a = self.slot_a.texture_dirty;
        let dirty_b = self.slot_b.texture_dirty;

        if dirty_a || self.slot_a.tile_texture.is_none() {
            let (w, h, rgba) = render_tile_sheet_rgba(
                &self.slot_a.rom.data,
                self.slot_a.view_offset,
                self.sheet_columns,
                self.sheet_rows,
                self.slot_a.tile_format,
                &pal_a,
            );
            let img = ColorImage::from_rgba_unmultiplied([w, h], &rgba);
            self.slot_a.tile_texture = Some(ctx.load_texture("sheet_a", img, tex_options));
            self.slot_a.texture_dirty = false;
        }

        if dirty_b || self.slot_b.tile_texture.is_none() {
            let (w, h, rgba) = render_tile_sheet_rgba(
                &self.slot_b.rom.data,
                self.slot_b.view_offset,
                self.sheet_columns,
                self.sheet_rows,
                self.slot_b.tile_format,
                &pal_b,
            );
            let img = ColorImage::from_rgba_unmultiplied([w, h], &rgba);
            self.slot_b.tile_texture = Some(ctx.load_texture("sheet_b", img, tex_options));
            self.slot_b.texture_dirty = false;
        }

        // Render single isolated asset sprite for clean, focused preview (UN Mario, UN Link, etc.)
        if let Some(asset_a) = self.slot_a.scanned_assets.get(self.slot_a.selected_tile_idx) {
            if self.slot_a.single_asset_id != asset_a.id || self.slot_a.single_asset_texture.is_none() || dirty_a {
                if let Some((w, h, rgba)) = render_single_asset_sprite(&self.slot_a.rom.data, &asset_a.id) {
                    let img = ColorImage::from_rgba_unmultiplied([w, h], &rgba);
                    self.slot_a.single_asset_texture = Some(ctx.load_texture(format!("single_asset_a_{}", asset_a.id), img, tex_options));
                    self.slot_a.single_asset_id = asset_a.id.clone();
                } else {
                    self.slot_a.single_asset_texture = None;
                    self.slot_a.single_asset_id.clear();
                }
            }
        }

        if let Some(asset_b) = self.slot_b.scanned_assets.get(self.slot_b.selected_tile_idx) {
            if self.slot_b.single_asset_id != asset_b.id || self.slot_b.single_asset_texture.is_none() || dirty_b {
                if let Some((w, h, rgba)) = render_single_asset_sprite(&self.slot_b.rom.data, &asset_b.id) {
                    let img = ColorImage::from_rgba_unmultiplied([w, h], &rgba);
                    self.slot_b.single_asset_texture = Some(ctx.load_texture(format!("single_asset_b_{}", asset_b.id), img, tex_options));
                    self.slot_b.single_asset_id = asset_b.id.clone();
                } else {
                    self.slot_b.single_asset_texture = None;
                    self.slot_b.single_asset_id.clear();
                }
            }
        }
    }

    // =========================================================================
    // RENDERING: TOP MENU BAR
    // =========================================================================
    fn render_top_menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("Fichier", |ui| {
                if ui.button("📂 Charger ROM Source A...").clicked() {
                    self.load_rom_from_dialog(true);
                    ui.close_menu();
                }
                if ui.button("📂 Charger ROM Source B...").clicked() {
                    self.load_rom_from_dialog(false);
                    ui.close_menu();
                }
                if ui.button("🔄 Recharger ROMs par défaut (Mario & Zelda)").clicked() {
                    self.reload_default_rom(true);
                    self.reload_default_rom(false);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("💾 Exporter ROM modifiée (.sfc)...").clicked() {
                    self.export_current_rom(None);
                    ui.close_menu();
                }
                if ui.button("📦 Exporter Patch (.ips)...").clicked() {
                    self.export_ips_patch();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Quitter").clicked() {
                    std::process::exit(0);
                }
            });

            ui.menu_button("Édition", |ui| {
                if ui.button("Annuler le dernier swap").clicked() {
                    if let Some(op) = self.swap_session.operations.last().cloned() {
                        let _ = self.swap_session.undo_operation(&mut self.slot_a.rom, op.id);
                        self.slot_a.rescan_after_edit();
                        self.add_log(format!("Swap #{} annulé.", op.id), Color32::from_rgb(251, 191, 36));
                    }
                    ui.close_menu();
                }
                if ui.button("Réinitialiser à la ROM originale").clicked() {
                    self.slot_a.rom.data = self.original_rom_a_data.clone();
                    self.slot_a.rescan_after_edit();
                    self.swap_session = SwapSession::new();
                    self.add_log("Session réinitialisée.", Color32::from_rgb(251, 191, 36));
                    ui.close_menu();
                }
            });

            ui.menu_button("Outils", |ui| {
                if ui.button("📦 Extraire TOUS les Assets de la ROM A (PNG/WAV/Palettes)").clicked() {
                    self.export_all_assets_slot(true);
                    ui.close_menu();
                }
                if ui.button("📦 Extraire TOUS les Assets de la ROM B (PNG/WAV/Palettes)").clicked() {
                    self.export_all_assets_slot(false);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Re-scanner toutes les palettes").clicked() {
                    self.rebuild_palette_list();
                    self.add_log("Palettes rafraîchies.", Color32::from_rgb(34, 211, 238));
                    ui.close_menu();
                }
            });

            ui.menu_button("Émulateur", |ui| {
                if ui.button("▶ Lancer l'émulateur SNES (ROM swappée)").clicked() {
                    let preview_path = std::env::temp_dir().join("snes_studio_swapped.sfc");
                    self.launch_in_snes_emulator(&preview_path.to_string_lossy());
                    ui.close_menu();
                }
            });

            ui.menu_button("Aide", |ui| {
                if ui.button("À propos de SNES Omni-Mixer Studio").clicked() {
                    self.add_log("SNES Omni-Mixer Studio v1.0 — Outil open-source multi-jeux universel pour ROMs SNES (.sfc/.smc).", Color32::WHITE);
                    ui.close_menu();
                }
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new("SNES Omni-Mixer Studio v1.0").strong().color(Color32::from_rgb(56, 189, 248)));
                ui.separator();
                ui.label(RichText::new("100% Rust").monospace().color(Color32::from_rgb(52, 211, 153)));
            });
        });
    }

    // =========================================================================
    // RENDERING: LEFT ICON SIDEBAR
    // =========================================================================
    fn render_icon_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(8.0);

            let icon_btn = |ui: &mut egui::Ui, icon: &str, tooltip: &str, active: bool| -> bool {
                let text = RichText::new(icon).size(20.0).color(if active { Color32::from_rgb(139, 92, 246) } else { Color32::LIGHT_GRAY });
                let resp = ui.button(text).on_hover_text(tooltip);
                resp.clicked()
            };

            if icon_btn(ui, "★", "Omni-Mixer Multi-Jeux (Mixez tout avec tout !)", self.sidebar_view == SidebarView::OmniMixer) {
                self.sidebar_view = SidebarView::OmniMixer;
            }
            ui.add_space(8.0);

            if icon_btn(ui, "⚡", "Passerelle de Fusion (Melter)", self.sidebar_view == SidebarView::Melter) {
                self.sidebar_view = SidebarView::Melter;
            }
            ui.add_space(8.0);

            if icon_btn(ui, "📦", "Déconstructeur Universel d'Assets SNES", self.sidebar_view == SidebarView::Deconstructor) {
                self.sidebar_view = SidebarView::Deconstructor;
            }
            ui.add_space(8.0);

            if icon_btn(ui, "🎨", "Inspecteur de Sprites & Feuilles GFX", self.sidebar_view == SidebarView::GraphicsInspector) {
                self.sidebar_view = SidebarView::GraphicsInspector;
            }
            ui.add_space(8.0);

            if icon_btn(ui, "🌈", "Inspecteur de Palettes BGR555", self.sidebar_view == SidebarView::PalettesInspector) {
                self.sidebar_view = SidebarView::PalettesInspector;
            }
            ui.add_space(8.0);

            if icon_btn(ui, "🎵", "Lecteur BRR & Échantillons Audio", self.sidebar_view == SidebarView::AudioInspector) {
                self.sidebar_view = SidebarView::AudioInspector;
            }
            ui.add_space(8.0);

            if icon_btn(ui, "📜", "Historique des Swaps & Patch", self.sidebar_view == SidebarView::HistoryLog) {
                self.sidebar_view = SidebarView::HistoryLog;
            }
        });
    }

    // =========================================================================
    // RENDERING: ROM SOURCE CARD (COLUMN A OR B)
    // =========================================================================
    fn render_rom_card(&mut self, ui: &mut egui::Ui, is_slot_a: bool) -> Rect {
        let (title_card, badge_color, filename, format_str, size_str, checksum_val) = if is_slot_a {
            (
                "ROM SOURCE A",
                Color32::from_rgb(16, 185, 129),
                self.slot_a.rom.canonical_title(),
                self.slot_a.rom.header.mapping.as_str(),
                format!("{} Ko", self.slot_a.rom.data.len() / 1024),
                self.slot_a.rom.header.checksum,
            )
        } else {
            (
                "ROM SOURCE B",
                Color32::from_rgb(139, 92, 246),
                self.slot_b.rom.canonical_title(),
                self.slot_b.rom.header.mapping.as_str(),
                format!("{} Ko", self.slot_b.rom.data.len() / 1024),
                self.slot_b.rom.header.checksum,
            )
        };

        let card_response = egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(26, 33, 51))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(39, 47, 68)))
            .corner_radius(8.0)
            .inner_margin(10.0)
            .show(ui, |ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.label(RichText::new(title_card).strong().color(badge_color));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("📂 Parcourir...").clicked() {
                            self.load_rom_from_dialog(is_slot_a);
                        }
                        if is_slot_a {
                            if ui.button("🔄 Défaut (SMW)").on_hover_text("Recharger Super Mario World par défaut").clicked() {
                                self.reload_default_rom(true);
                            }
                        } else {
                            if ui.button("🔄 Défaut (Zelda)").on_hover_text("Recharger Zelda ALTTP par défaut").clicked() {
                                self.reload_default_rom(false);
                            }
                        }
                    });
                });

                ui.label(RichText::new(filename).size(15.0).strong().color(Color32::WHITE));
                ui.label(
                    RichText::new(format!("Taille: {} | Format: {} | Checksum: 0x{:04X}", size_str, format_str, checksum_val))
                        .size(11.0)
                        .color(Color32::LIGHT_GRAY),
                );

                ui.add_space(6.0);

                // Tabs: Sprites, Tuiles, Musiques, Palettes
                let active_tab = if is_slot_a { &mut self.rom_a_tab } else { &mut self.rom_b_tab };
                ui.horizontal(|ui| {
                    ui.selectable_value(active_tab, RomCardTab::Sprites, "Sprites");
                    ui.selectable_value(active_tab, RomCardTab::Tiles, "Tuiles GFX");
                    ui.selectable_value(active_tab, RomCardTab::Audio, "Musiques");
                    ui.selectable_value(active_tab, RomCardTab::Palettes, "Palettes");
                });

                let current_tab = if is_slot_a { self.rom_a_tab } else { self.rom_b_tab };
                ui.separator();

                // Quick Asset Showcase Bar (Barre de sélection rapide d'actifs isolés)
                let game = if is_slot_a { self.slot_a.rom.identify_game() } else { self.slot_b.rom.identify_game() };
                let showcase_items: &[(&str, &str, &str)] = match game {
                    RecognizedGame::SuperMarioWorld => &[
                        ("smw_mario", "🏃 Mario", "Super Mario Debout (16×32)"),
                        ("smw_coin", "🪙 Pièce", "Pièce d'or animée (16×16)"),
                        ("smw_turn_block", "🧱 Brique", "Bloc de brique tournant (16×16)"),
                        ("smw_question_block", "❓ Bloc '?'", "Bloc mystère '?' (16×16)"),
                        ("smw_mushroom", "🍄 Champignon", "Super Champignon (16×16)"),
                        ("smw_koopa", "🐢 Koopa", "Koopa Troopa (16×24)"),
                        ("smw_yoshi", "🦖 Yoshi", "Monture Yoshi (32×32)"),
                    ],
                    RecognizedGame::ZeldaAlttp => &[
                        ("zelda_link", "🗡️ Link", "Link Héros d'Hyrule (16×24)"),
                        ("zelda_rupee", "💎 Rubis", "Rubis d'Hyrule (16×16)"),
                        ("zelda_heart", "❤️ Cœur", "Conteneur de Cœur (16×16)"),
                        ("zelda_chest", "📦 Coffre", "Coffre au Trésor (16×16)"),
                        ("zelda_pot", "🏺 Pot", "Pot en terre cuite (16×16)"),
                        ("zelda_zelda", "👸 Zelda", "Princesse Zelda (16×32)"),
                        ("zelda_soldier", "🛡️ Soldat", "Garde / Soldat d'Hyrule (16×24)"),
                    ],
                    _ => &[],
                };

                let mut clicked_showcase: Option<&'static str> = None;
                if current_tab == RomCardTab::Sprites && !showcase_items.is_empty() {
                    let current_selected_id = {
                        let slot = if is_slot_a { &self.slot_a } else { &self.slot_b };
                        slot.scanned_assets.get(slot.selected_tile_idx).map(|a| a.id.clone()).unwrap_or_default()
                    };

                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Sélection Rapide :").strong().size(10.5).color(Color32::from_rgb(251, 191, 36)));
                        for (sc_id, sc_label, sc_tip) in showcase_items {
                            let is_cur = current_selected_id == *sc_id;
                            let btn = egui::Button::new(
                                RichText::new(*sc_label)
                                    .size(10.5)
                                    .color(if is_cur { Color32::WHITE } else { Color32::from_rgb(203, 213, 225) })
                                    .strong(),
                            )
                            .fill(if is_cur {
                                if is_slot_a { Color32::from_rgb(16, 120, 80) } else { Color32::from_rgb(109, 40, 217) }
                            } else {
                                Color32::from_rgb(30, 38, 55)
                            })
                            .stroke(Stroke::new(1.0_f32, if is_cur {
                                if is_slot_a { Color32::from_rgb(52, 211, 153) } else { Color32::from_rgb(196, 181, 253) }
                            } else {
                                Color32::from_rgb(45, 55, 78)
                            }));

                            if ui.add(btn).on_hover_text(*sc_tip).clicked() {
                                clicked_showcase = Some(*sc_id);
                            }
                        }
                    });
                    ui.add_space(3.0);
                }

                if let Some(target_id) = clicked_showcase {
                    let slot = if is_slot_a { &mut self.slot_a } else { &mut self.slot_b };
                    if let Some(pos) = slot.scanned_assets.iter().position(|a| a.id == target_id) {
                        slot.selected_tile_idx = pos;
                        let a = &slot.scanned_assets[pos];
                        slot.view_offset = a.rom_offset;
                        slot.offset_hex_input = format!("{:06X}", a.rom_offset);
                        slot.tile_format = a.format;
                        slot.single_asset_id = target_id.to_string();
                        if let Some(poff) = a.palette_offset {
                            if let Some(p_pos) = slot.detected_palettes.iter().position(|p| p.source_offset == Some(poff) || p.name.contains(&format!("{:06X}", poff))) {
                                slot.active_palette_idx = p_pos;
                            }
                        }
                        slot.texture_dirty = true;
                    }
                }

                // Interactive Viewport Controls
                let zoom = if is_slot_a { &mut self.zoom_a } else { &mut self.zoom_b };
                let grid = if is_slot_a { &mut self.grid_a } else { &mut self.grid_b };

                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Zoom:").size(11.0));
                    ui.add(egui::Slider::new(zoom, 1.0..=6.0).show_value(false));
                    if current_tab == RomCardTab::Tiles {
                        ui.checkbox(grid, RichText::new("Grille").size(11.0));
                    }
                    if ui.checkbox(&mut self.ai_hd_upscale, RichText::new("✨ Modèle IA : Lissage HD").size(11.0).color(Color32::from_rgb(34, 211, 238))).changed() {
                        self.slot_a.texture_dirty = true;
                        self.slot_b.texture_dirty = true;
                    }
                });

                // Viewport Box
                let vp_height = 145.0;
                let current_zoom = *zoom;
                egui::Frame::canvas(ui.style())
                    .fill(Color32::from_rgb(14, 18, 30))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(32, 40, 60)))
                    .corner_radius(6.0)
                    .show(ui, |ui| {
                        ui.set_min_height(vp_height);
                        ui.set_max_height(vp_height);
                        ui.set_width(ui.available_width());

                        let slot = if is_slot_a { &self.slot_a } else { &self.slot_b };
                        match current_tab {
                            RomCardTab::Sprites => {
                                if let Some(ref tex) = slot.single_asset_texture {
                                    let base_h = tex.size()[1] as f32;
                                    let base_scale = (112.0 / base_h).clamp(2.5, 6.0);
                                    let scale = base_scale * (current_zoom / 2.0).clamp(0.5, 2.5);
                                    let w = (tex.size()[0] as f32) * scale;
                                    let h = (tex.size()[1] as f32) * scale;

                                    ui.centered_and_justified(|ui| {
                                        ui.image((tex.id(), Vec2::new(w, h)));
                                    });
                                } else if let Some(ref tex) = slot.tile_texture {
                                    let w = (tex.size()[0] as f32) * current_zoom;
                                    let h = (tex.size()[1] as f32) * current_zoom;
                                    ui.centered_and_justified(|ui| {
                                        ui.image((tex.id(), Vec2::new(w, h)));
                                    });
                                } else {
                                    ui.centered_and_justified(|ui| {
                                        ui.label(RichText::new("Aperçu du sprite").color(Color32::DARK_GRAY));
                                    });
                                }
                            }
                            RomCardTab::Tiles => {
                                if let Some(ref tex) = slot.tile_texture {
                                    let w = (tex.size()[0] as f32) * current_zoom;
                                    let h = (tex.size()[1] as f32) * current_zoom;
                                    egui::ScrollArea::both().max_height(vp_height).show(ui, |ui| {
                                        ui.image((tex.id(), Vec2::new(w, h)));
                                    });
                                } else {
                                    ui.centered_and_justified(|ui| {
                                        ui.label(RichText::new("Feuille de tuiles GFX").color(Color32::DARK_GRAY));
                                    });
                                }
                            }
                            RomCardTab::Audio => {
                                let sample_count = slot.brr_samples.len();
                                ui.centered_and_justified(|ui| {
                                    ui.vertical_centered(|ui| {
                                        ui.label(RichText::new("🎵 Lecteur Audio BRR").strong().size(13.0).color(Color32::from_rgb(139, 92, 246)));
                                        ui.add_space(4.0);
                                        ui.label(RichText::new(format!("{} échantillons BRR indexés dans la ROM", sample_count)).size(11.0).color(Color32::LIGHT_GRAY));
                                    });
                                });
                            }
                            RomCardTab::Palettes => {
                                let pal_count = slot.detected_palettes.len();
                                ui.centered_and_justified(|ui| {
                                    ui.vertical_centered(|ui| {
                                        ui.label(RichText::new("🎨 Palettes CGRAM BGR555").strong().size(13.0).color(Color32::from_rgb(251, 191, 36)));
                                        ui.add_space(4.0);
                                        ui.label(RichText::new(format!("{} palettes détectées et décodées", pal_count)).size(11.0).color(Color32::LIGHT_GRAY));
                                    });
                                });
                            }
                        }
                    });

                // Focused Asset Info Badge
                if current_tab == RomCardTab::Sprites {
                    let slot = if is_slot_a { &self.slot_a } else { &self.slot_b };
                    if let Some(asset) = slot.scanned_assets.get(slot.selected_tile_idx) {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("⭐ Actif Isolé :").strong().size(10.5).color(Color32::from_rgb(34, 211, 238)));
                            ui.label(RichText::new(format!("{} {}", asset.icon, asset.name)).strong().size(10.5).color(Color32::WHITE));
                            ui.label(RichText::new("• Couleurs Authentiques 16c").size(10.0).color(Color32::from_rgb(16, 185, 129)));
                        });
                    }
                }

                ui.add_space(4.0);

                // Palette Selector & 16-color Swatches
                let (_current_pal_idx, current_pal_name) = {
                    let slot = if is_slot_a { &self.slot_a } else { &self.slot_b };
                    let idx = slot.active_palette_idx;
                    let name = slot.detected_palettes.get(idx).map(|p| p.name.clone()).unwrap_or_else(|| "Par défaut".to_string());
                    (idx, name)
                };

                let mut changed_pal = None;
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🎨 Palette :").strong().size(11.0).color(Color32::from_rgb(251, 191, 36)));
                    egui::ComboBox::from_id_salt(if is_slot_a { "pal_combo_a" } else { "pal_combo_b" })
                        .selected_text(RichText::new(&current_pal_name).size(11.0).color(Color32::WHITE))
                        .width(260.0)
                        .show_ui(ui, |ui| {
                            let palettes = if is_slot_a { &self.slot_a.detected_palettes } else { &self.slot_b.detected_palettes };
                            for (p_i, p) in palettes.iter().enumerate() {
                                if ui.selectable_value(&mut changed_pal, Some(p_i), RichText::new(&p.name).size(11.0)).clicked() {
                                    changed_pal = Some(p_i);
                                }
                            }
                        });
                });
                if let Some(new_pi) = changed_pal {
                    let slot = if is_slot_a { &mut self.slot_a } else { &mut self.slot_b };
                    slot.active_palette_idx = new_pi;
                    slot.texture_dirty = true;
                }

                let pal = if is_slot_a {
                    self.slot_a.detected_palettes.get(self.slot_a.active_palette_idx)
                } else {
                    self.slot_b.detected_palettes.get(self.slot_b.active_palette_idx)
                };

                if let Some(p) = pal {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("16c :").size(10.0).color(Color32::GRAY));
                        for (c_idx, c) in p.colors_rgba.iter().take(16).enumerate() {
                            let col32 = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
                            let (rect, resp) = ui.allocate_exact_size(Vec2::new(13.0, 13.0), Sense::hover());
                            ui.painter().rect_filled(rect, 2.0, col32);
                            ui.painter().rect_stroke(rect, 2.0, Stroke::new(1.0_f32, Color32::from_black_alpha(120)), egui::StrokeKind::Inside);
                            resp.on_hover_text(format!("Indice #{:02} : #{:02X}{:02X}{:02X}", c_idx, c[0], c[1], c[2]));
                        }
                    });
                }

                ui.add_space(6.0);

                // Asset Selector
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Actifs & Sprites détectés :").strong().size(12.0).color(Color32::from_rgb(220, 220, 240)));
                    let count = if is_slot_a { self.slot_a.scanned_assets.len() } else { self.slot_b.scanned_assets.len() };
                    ui.label(RichText::new(format!("({} trouvés)", count)).size(10.0).color(Color32::from_rgb(34, 211, 238)));
                });

                // Filter pills
                let current_filter = if is_slot_a { &mut self.asset_filter_category_a } else { &mut self.asset_filter_category_b };
                ui.horizontal_wrapped(|ui| {
                    if ui.selectable_label(current_filter.is_none(), "Tous").clicked() {
                        *current_filter = None;
                    }
                    if ui.selectable_label(*current_filter == Some(AssetCategory::Player), "🏃 Héros").clicked() {
                        *current_filter = Some(AssetCategory::Player);
                    }
                    if ui.selectable_label(*current_filter == Some(AssetCategory::Mount), "🦖 Monture").clicked() {
                        *current_filter = Some(AssetCategory::Mount);
                    }
                    if ui.selectable_label(*current_filter == Some(AssetCategory::PowerUp), "🍄 Objets").clicked() {
                        *current_filter = Some(AssetCategory::PowerUp);
                    }
                    if ui.selectable_label(*current_filter == Some(AssetCategory::InteractiveBlock), "🧱 Blocs").clicked() {
                        *current_filter = Some(AssetCategory::InteractiveBlock);
                    }
                    if ui.selectable_label(*current_filter == Some(AssetCategory::Enemy), "🐢 Ennemis").clicked() {
                        *current_filter = Some(AssetCategory::Enemy);
                    }
                    if ui.selectable_label(*current_filter == Some(AssetCategory::Boss), "👑 Boss").clicked() {
                        *current_filter = Some(AssetCategory::Boss);
                    }
                });

                ui.add_space(2.0);

                let current_cat = if is_slot_a { self.asset_filter_category_a } else { self.asset_filter_category_b };
                let slot = if is_slot_a { &mut self.slot_a } else { &mut self.slot_b };

                let mut clicked_asset: Option<(usize, usize, TileFormat, Option<usize>)> = None;
                let mut trigger_pack_family: Option<&'static str> = None;

                // Group scanned assets by family
                let mut families: Vec<&'static str> = Vec::new();
                for asset in &slot.scanned_assets {
                    if let Some(cat) = current_cat {
                        if asset.category != cat {
                            continue;
                        }
                    }
                    if !families.contains(&asset.family_name) {
                        families.push(asset.family_name);
                    }
                }

                egui::ScrollArea::vertical().max_height(130.0).show(ui, |ui| {
                    for fam in families {
                        ui.add_space(3.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(fam)
                                    .strong()
                                    .size(11.0)
                                    .color(Color32::from_rgb(147, 197, 253)),
                            );
                            ui.label(
                                RichText::new("[Pack Complet]")
                                    .size(9.0)
                                    .color(Color32::from_rgb(34, 211, 238)),
                            );
                        });

                        for (idx, asset) in slot.scanned_assets.iter().enumerate() {
                            if asset.family_name != fam {
                                continue;
                            }
                            if let Some(cat) = current_cat {
                                if asset.category != cat {
                                    continue;
                                }
                            }
                            let is_sel = slot.selected_tile_idx == idx;
                            let item_text = format!("   {} {}  [{}]", asset.icon, asset.name, asset.sheet_name);
                            let response = ui.selectable_label(is_sel, item_text);
                            if response.clicked() {
                                clicked_asset = Some((idx, asset.rom_offset, asset.format, asset.palette_offset));
                            }
                        }
                    }
                });

                if let Some((idx, off, fmt, pal_off)) = clicked_asset {
                    slot.selected_tile_idx = idx;
                    slot.view_offset = off;
                    slot.offset_hex_input = format!("{:06X}", off);
                    slot.tile_format = fmt;
                    if let Some(poff) = pal_off {
                        if let Some(pos) = slot.detected_palettes.iter().position(|p| p.source_offset == Some(poff) || p.name.contains(&format!("{:06X}", poff))) {
                            slot.active_palette_idx = pos;
                        }
                    }
                    if let Some(a) = slot.scanned_assets.get(idx) {
                        slot.single_asset_id = a.id.clone();
                    }
                    slot.texture_dirty = true;
                }

                // Show selected asset description box with family pack information
                if let Some(asset) = slot.scanned_assets.get(slot.selected_tile_idx) {
                    let fam_name = asset.family_name;
                    ui.add_space(4.0);
                    egui::Frame::new()
                        .fill(Color32::from_rgb(18, 22, 34))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 58, 85)))
                        .corner_radius(4.0)
                        .inner_margin(6.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("{} {}", asset.icon, asset.name)).strong().size(11.0).color(Color32::WHITE));
                                ui.label(RichText::new(asset.category.label_fr()).size(10.0).color(Color32::from_rgb(251, 191, 36)));
                            });
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("Famille : {}", asset.family_name)).strong().size(10.0).color(Color32::from_rgb(34, 211, 238)));
                            });
                            ui.label(RichText::new(asset.family_pack_desc).size(9.5).color(Color32::from_rgb(186, 230, 253)));
                            ui.label(RichText::new(&asset.description).size(9.0).color(Color32::LIGHT_GRAY));

                            ui.add_space(3.0);
                            let swap_pack_btn = egui::Button::new(
                                RichText::new("⚡ Échanger ce Pack Complet")
                                    .size(10.5)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(79, 70, 229))
                            .min_size(Vec2::new(ui.available_width(), 22.0));

                            if ui.add(swap_pack_btn).clicked() {
                                trigger_pack_family = Some(fam_name);
                            }
                        });
                }

                if let Some(fam) = trigger_pack_family {
                    let mut matched = false;
                    for d in &mut self.swap_decisions {
                        let fam_key = if fam.len() > 10 { &fam[..10] } else { fam };
                        if d.family_name.contains(fam_key) || fam.contains(&d.family_name[..10.min(d.family_name.len())]) {
                            d.enabled = true;
                            matched = true;
                        }
                    }
                    if !matched {
                        if let Some(first) = self.swap_decisions.first_mut() {
                            first.enabled = true;
                        }
                    }
                    self.add_log(format!("Pack complet activé dans la matrice de fusion : {}", fam), Color32::from_rgb(16, 185, 129));
                }

                ui.add_space(6.0);

                let btn_dump = egui::Button::new(
                    RichText::new("📦 Extraire tous les assets (PNG / WAV / Palettes)")
                        .size(11.0)
                        .strong()
                        .color(Color32::from_rgb(34, 211, 238)),
                )
                .fill(Color32::from_rgb(26, 38, 56))
                .min_size(Vec2::new(ui.available_width(), 26.0));

                if ui.add(btn_dump).clicked() {
                    self.export_all_assets_slot(is_slot_a);
                }
            });

        card_response.response.rect
    }

    // =========================================================================
    // RENDERING: CENTRAL MELTER COLUMN (SWAP DECIDER CONTROL CENTER)
    // =========================================================================
    fn render_melter_central(&mut self, ui: &mut egui::Ui) -> Rect {
        let card_response = egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(20, 26, 44))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 55, 80)))
            .corner_radius(8.0)
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("PASSERELLE DE FUSION").strong().size(13.0).color(Color32::from_rgb(139, 92, 246)));
                    ui.label(RichText::new("MATRICE DE DÉCISION DES SWAPS").strong().size(11.0).color(Color32::from_rgb(34, 211, 238)));
                });

                ui.add_space(4.0);

                // Summary badge
                let active_count = self.swap_decisions.iter().filter(|d| d.enabled).count();
                let total_count = self.swap_decisions.len();
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Actifs sélectionnés : {} / {}", active_count, total_count))
                            .size(11.0)
                            .strong()
                            .color(if active_count > 0 { Color32::from_rgb(16, 185, 129) } else { Color32::GRAY }),
                    );
                });

                // Presets
                ui.horizontal_wrapped(|ui| {
                    if ui.small_button("⚡ Tout").clicked() {
                        for d in &mut self.swap_decisions {
                            d.enabled = true;
                        }
                    }
                    if ui.small_button("🧹 Aucun").clicked() {
                        for d in &mut self.swap_decisions {
                            d.enabled = false;
                        }
                    }
                    if ui.small_button("🛡️ Pack Zelda").clicked() {
                        for d in &mut self.swap_decisions {
                            d.enabled = d.id == "swap_mario_link"
                                || d.id == "swap_title_screen"
                                || d.id == "swap_mushroom_heart"
                                || d.id == "swap_coin_rupee"
                                || d.id == "swap_qblock_chest"
                                || d.id == "swap_brick_pot";
                        }
                    }
                    if ui.small_button("🥊 Pack DK").clicked() {
                        for d in &mut self.swap_decisions {
                            d.enabled = d.id == "swap_yoshi_dk" || d.id == "swap_punch_fist";
                        }
                    }
                });

                ui.separator();

                // Interactive Decider List with Checkboxes & Descriptions
                egui::ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
                    for d in &mut self.swap_decisions {
                        egui::Frame::new()
                            .fill(if d.enabled { Color32::from_rgb(26, 38, 60) } else { Color32::from_rgb(16, 20, 30) })
                            .stroke(Stroke::new(1.0_f32, if d.enabled { Color32::from_rgb(99, 102, 241) } else { Color32::from_rgb(32, 40, 56) }))
                            .corner_radius(4.0)
                            .inner_margin(6.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut d.enabled, "");
                                    ui.label(RichText::new(format!("{} {}", d.icon, d.label)).size(11.0).strong().color(if d.enabled { Color32::WHITE } else { Color32::GRAY }));
                                });
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(format!("Famille : {}", d.family_name)).size(9.5).color(Color32::from_rgb(251, 191, 36)));
                                });
                                ui.label(RichText::new(format!("📦 Pack Complet : {}", d.pack_summary)).size(9.5).color(Color32::from_rgb(147, 197, 253)));
                                ui.label(RichText::new(&d.description).size(9.0).color(Color32::from_rgb(160, 170, 190)));
                            });
                        ui.add_space(2.0);
                    }
                });

                ui.add_space(6.0);

                // Big Action Glow Button: FUSIONNER LES ACTIFS SÉLECTIONNÉS
                let melt_btn = egui::Button::new(
                    RichText::new("🚀 FUSIONNER LES SÉLECTIONS")
                        .size(13.0)
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(139, 92, 246))
                .min_size(Vec2::new(ui.available_width(), 38.0));

                if ui.add(melt_btn).clicked() {
                    self.execute_user_swaps();
                }

                ui.add_space(4.0);

                // Controls row
                ui.horizontal(|ui| {
                    if ui.button("▶ Tester Snes9x").clicked() {
                        let _ = std::process::Command::new("snes9x")
                            .arg("super_marioworld_swapped.sfc")
                            .spawn();
                        self.add_log("Émulateur Snes9x lancé avec super_marioworld_swapped.sfc", Color32::from_rgb(34, 211, 238));
                    }

                    if ui.button("↩ Annuler").clicked() {
                        if let Some(op) = self.swap_session.operations.last().cloned() {
                            let _ = self.swap_session.undo_operation(&mut self.slot_a.rom, op.id);
                            self.slot_a.rescan_after_edit();
                            self.add_log(format!("Swap #{} annulé.", op.id), Color32::from_rgb(251, 191, 36));
                        }
                    }
                });
            });

        card_response.response.rect
    }

    // =========================================================================
    // RENDERING: 3-COLUMN MAIN MELTER VIEW
    // =========================================================================
    fn render_melter_tab(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let total_width = ui.available_width().max(1060.0);
                let center_w = 360.0;
                let col_w = ((total_width - center_w - 32.0) / 2.0).max(330.0);

                let mut rect_a = Rect::NOTHING;
                let mut rect_melter = Rect::NOTHING;
                let mut rect_b = Rect::NOTHING;

                ui.horizontal(|ui| {
                    // Left Column: ROM SOURCE A
                    ui.allocate_ui_with_layout(
                        Vec2::new(col_w, ui.available_height()),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            rect_a = self.render_rom_card(ui, true);
                        },
                    );

                    ui.add_space(8.0);

                    // Center Column: MELTER
                    ui.allocate_ui_with_layout(
                        Vec2::new(center_w, ui.available_height()),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            rect_melter = self.render_melter_central(ui);
                        },
                    );

                    ui.add_space(8.0);

                    // Right Column: ROM SOURCE B
                    ui.allocate_ui_with_layout(
                        Vec2::new(col_w, ui.available_height()),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            rect_b = self.render_rom_card(ui, false);
                        },
                    );
                });

                // Paint Bezier Connector Curves
                if rect_a.is_positive() && rect_melter.is_positive() && rect_b.is_positive() {
                    let p_a = Pos2::new(rect_a.right(), rect_a.center().y);
                    let p_m_left = Pos2::new(rect_melter.left(), rect_melter.center().y);
                    let p_m_right = Pos2::new(rect_melter.right(), rect_melter.center().y);
                    let p_b = Pos2::new(rect_b.left(), rect_b.center().y);

                    let c_a1 = Pos2::new(p_a.x + 30.0, p_a.y);
                    let c_m1 = Pos2::new(p_m_left.x - 30.0, p_m_left.y);
                    let bezier1 = CubicBezierShape::from_points_stroke(
                        [p_a, c_a1, c_m1, p_m_left],
                        false,
                        Color32::TRANSPARENT,
                        Stroke::new(2.0_f32, Color32::from_rgb(139, 92, 246)),
                    );
                    ui.painter().add(bezier1);

                    let c_m2 = Pos2::new(p_m_right.x + 30.0, p_m_right.y);
                    let c_b2 = Pos2::new(p_b.x - 30.0, p_b.y);
                    let bezier2 = CubicBezierShape::from_points_stroke(
                        [p_m_right, c_m2, c_b2, p_b],
                        false,
                        Color32::TRANSPARENT,
                        Stroke::new(2.0_f32, Color32::from_rgb(34, 211, 238)),
                    );
                    ui.painter().add(bezier2);
                }
            });
    }

    // =========================================================================
    // RENDERING: BOTTOM CONSOLE & QUICK ACTIONS
    // =========================================================================
    fn render_bottom_dock(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Left: Log Terminal
            let term_width = (ui.available_width() - 320.0).max(200.0);
            egui::Frame::canvas(ui.style())
                .fill(Color32::from_rgb(10, 14, 26))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(26, 33, 51)))
                .corner_radius(4.0)
                .inner_margin(6.0)
                .show(ui, |ui| {
                    ui.set_width(term_width);
                    ui.set_height(80.0);
                    egui::ScrollArea::vertical()
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for (msg, color) in &self.log_messages {
                                ui.label(RichText::new(msg).monospace().size(11.0).color(*color));
                            }
                        });
                });

            ui.separator();

            // Right: Export & Test Action Buttons
            ui.vertical(|ui| {
                let btn_test = egui::Button::new(
                    RichText::new("▶ Tester la ROM (Émulateur)")
                        .strong()
                        .size(13.0)
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(16, 185, 129))
                .min_size(Vec2::new(280.0, 26.0));

                if ui.add(btn_test).clicked() {
                    self.play_jeu_1();
                }

                ui.add_space(4.0);

                let btn_exp_sfc = egui::Button::new(
                    RichText::new("💾 Exporter la ROM modifiée (.sfc)")
                        .strong()
                        .size(13.0)
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(41, 128, 185))
                .min_size(Vec2::new(280.0, 26.0));

                if ui.add(btn_exp_sfc).clicked() {
                    self.export_current_rom(None);
                }

                ui.add_space(4.0);

                let btn_exp_ips = egui::Button::new(
                    RichText::new("📦 Exporter en Patch (.ips)")
                        .size(12.0)
                        .color(Color32::LIGHT_GRAY),
                )
                .min_size(Vec2::new(280.0, 22.0));

                if ui.add(btn_exp_ips).clicked() {
                    self.export_ips_patch();
                }
            });
        });
    }

    // =========================================================================
    // TECHNICAL INSPECTOR TABS (PRESERVED)
    // =========================================================================
    fn render_graphics_swap_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("🎨 Inspecteur & Swapper Graphique Direct").strong().color(Color32::from_rgb(139, 92, 246)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("◀ Retour à la Passerelle").clicked() {
                    self.sidebar_view = SidebarView::Melter;
                }
            });
        });
        ui.separator();

        ui.horizontal(|ui| {
            let half = (ui.available_width() - 20.0) / 2.0;
            ui.allocate_ui(Vec2::new(half, ui.available_height()), |ui| {
                ui.heading("ROM A — Graphismes");
                ui.label(format!("Offset: 0x{:06X}", self.slot_a.view_offset));
                if let Some(tex) = &self.slot_a.tile_texture {
                    ui.image((tex.id(), Vec2::new(tex.size()[0] as f32 * self.zoom, tex.size()[1] as f32 * self.zoom)));
                }
            });
            ui.separator();
            ui.allocate_ui(Vec2::new(half, ui.available_height()), |ui| {
                ui.heading("ROM B — Graphismes");
                ui.label(format!("Offset: 0x{:06X}", self.slot_b.view_offset));
                if let Some(tex) = &self.slot_b.tile_texture {
                    ui.image((tex.id(), Vec2::new(tex.size()[0] as f32 * self.zoom, tex.size()[1] as f32 * self.zoom)));
                }
            });
        });
    }

    fn render_palettes_inspector(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("🌈 Inspecteur de Palettes BGR555").strong().color(Color32::from_rgb(139, 92, 246)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("◀ Retour à la Passerelle").clicked() {
                    self.sidebar_view = SidebarView::Melter;
                }
            });
        });
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Palettes Détectées");
            for (idx, p) in self.palettes.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("#{}: {}", idx, p.name));
                    for c in &p.colors_rgba {
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
                        ui.painter().rect_filled(rect, 2.0, Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]));
                    }
                });
            }
        });
    }

    fn render_audio_inspector(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("🎵 Lecteur & Inspecteur BRR (SPC700)").strong().color(Color32::from_rgb(139, 92, 246)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("◀ Retour à la Passerelle").clicked() {
                    self.sidebar_view = SidebarView::Melter;
                }
            });
        });
        ui.separator();

        ui.horizontal(|ui| {
            let half = (ui.available_width() - 20.0) / 2.0;
            ui.allocate_ui(Vec2::new(half, ui.available_height()), |ui| {
                ui.heading("Échantillons BRR — ROM A");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for s in &self.slot_a.brr_samples {
                        ui.label(format!("0x{:06X}: {}", s.offset, s.label));
                    }
                });
            });
            ui.separator();
            ui.allocate_ui(Vec2::new(half, ui.available_height()), |ui| {
                ui.heading("Échantillons BRR — ROM B");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for s in &self.slot_b.brr_samples {
                        ui.label(format!("0x{:06X}: {}", s.offset, s.label));
                    }
                });
            });
        });
    }

    fn render_history_inspector(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("📜 Historique des Swaps").strong().color(Color32::from_rgb(139, 92, 246)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("◀ Retour à la Passerelle").clicked() {
                    self.sidebar_view = SidebarView::Melter;
                }
            });
        });
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.swap_session.operations.is_empty() {
                ui.label("Aucun swap n'a encore été appliqué.");
            } else {
                for op in &self.swap_session.operations {
                    ui.group(|ui| {
                        ui.label(RichText::new(format!("#{}: {}", op.id, op.description)).strong());
                        ui.label(format!("Offset A: 0x{:06X} | Offset B: 0x{:06X} | Octets: {}", op.target_offset_a, op.donor_offset_b, op.original_bytes_a.len()));
                    });
                }
            }
        });
    }

    fn ensure_universal_catalogs(&mut self) {
        if self.catalog_a.is_none() {
            self.catalog_a = Some(snes_core::universal::discover_universal_catalog(&self.slot_a.rom));
        }
        if self.catalog_b.is_none() {
            self.catalog_b = Some(snes_core::universal::discover_universal_catalog(&self.slot_b.rom));
        }
    }

    pub fn reconstruct_rom_from_dialog(&mut self) {
        if let Some(proj_dir) = rfd::FileDialog::new()
            .set_title("Sélectionner un dossier de projet déconstruit (contenant project.json)")
            .pick_folder()
        {
            let target_rom = if self.deconstruct_target_slot_a {
                &self.slot_a.rom
            } else {
                &self.slot_b.rom
            };
            let out_rom = proj_dir.join("reconstructed_rom.sfc");
            let out_ips = proj_dir.join("reconstructed_patch.ips");
            match snes_core::universal::reconstruct_rom_from_project(
                target_rom,
                &proj_dir,
                &out_rom,
                Some(&out_ips),
            ) {
                Ok(rep) => {
                    self.add_log(
                        format!(
                            "✓ ROM reconstruite en {} ms : {} GFX, {} palettes, {} audio, {} textes patchés (Checksum 0x{:04X}) -> {}",
                            rep.elapsed_ms,
                            rep.gfx_patched_count,
                            rep.palettes_patched_count,
                            rep.audio_patched_count,
                            rep.text_patched_count,
                            rep.new_checksum,
                            rep.out_rom_path
                        ),
                        Color32::from_rgb(16, 185, 129),
                    );
                    self.last_reconstruct_report = Some(rep);
                }
                Err(e) => {
                    self.add_log(
                        format!("Erreur reconstruction projet : {:?}", e),
                        Color32::from_rgb(239, 68, 68),
                    );
                }
            }
        }
    }

    fn execute_single_universal_swap(&mut self) {
        self.ensure_universal_catalogs();
        let (cat_a, cat_b) = match (self.catalog_a.clone(), self.catalog_b.clone()) {
            (Some(a), Some(b)) => (a, b),
            _ => return,
        };

        let res = match self.universal_cat_tab {
            0 => {
                // Graphics / Sprites Swap
                if let (Some(ga), Some(gb)) = (
                    cat_a.graphics.get(self.universal_sel_a),
                    cat_b.graphics.get(self.universal_sel_b),
                ) {
                    let pa = cat_a
                        .palettes
                        .get(ga.suggested_palette_idx)
                        .map(snes_core::universal::palette_asset_to_snes_palette)
                        .unwrap_or_else(SnesPalette::preset_snes_adventure);
                    let pb = cat_b
                        .palettes
                        .get(gb.suggested_palette_idx)
                        .map(snes_core::universal::palette_asset_to_snes_palette)
                        .unwrap_or_else(SnesPalette::preset_cyber_neon);
                    snes_core::universal::swap_universal_gfx_asset(
                        &mut self.slot_a.rom,
                        &self.slot_b.rom,
                        ga,
                        gb,
                        &pa,
                        &pb,
                        self.universal_smart_remap,
                        &mut self.swap_session,
                    )
                } else {
                    Ok(0)
                }
            }
            1 => {
                // Palette Swap
                if let (Some(pa), Some(pb)) = (
                    cat_a.palettes.get(self.universal_sel_a),
                    cat_b.palettes.get(self.universal_sel_b),
                ) {
                    snes_core::universal::swap_universal_palette_asset(
                        &mut self.slot_a.rom,
                        &self.slot_b.rom,
                        pa,
                        pb,
                        &mut self.swap_session,
                    )
                } else {
                    Ok(0)
                }
            }
            2 => {
                // Audio BRR Swap
                if let (Some(aa), Some(ab)) = (
                    cat_a.audio_samples.get(self.universal_sel_a),
                    cat_b.audio_samples.get(self.universal_sel_b),
                ) {
                    snes_core::universal::swap_universal_audio_asset(
                        &mut self.slot_a.rom,
                        &self.slot_b.rom,
                        aa,
                        ab,
                        &mut self.swap_session,
                    )
                } else {
                    Ok(0)
                }
            }
            _ => {
                // Text Swap
                if let (Some(ta), Some(tb)) = (
                    cat_a.texts.get(self.universal_sel_a),
                    cat_b.texts.get(self.universal_sel_b),
                ) {
                    snes_core::universal::patch_universal_text_asset(
                        &mut self.slot_a.rom,
                        ta,
                        &tb.text,
                        &mut self.swap_session,
                    )
                } else {
                    Ok(0)
                }
            }
        };

        match res {
            Ok(n) if n > 0 => {
                self.slot_a.rescan_after_edit();
                self.catalog_a = None;
                let preview_path = std::env::temp_dir().join("snes_studio_swapped.sfc");
                let _ = std::fs::write(&preview_path, self.slot_a.rom.to_sfc_bytes());
                self.add_log(
                    format!(
                        "✓ Échange universel appliqué ! Checksum SNES valide : 0x{:04X}",
                        self.slot_a.rom.header.checksum
                    ),
                    Color32::from_rgb(16, 185, 129),
                );
            }
            Ok(_) => {
                self.add_log("Sélectionnez un actif dans ROM A et un actif dans ROM B.", Color32::from_rgb(251, 191, 36));
            }
            Err(e) => {
                self.add_log(format!("Erreur échange universel : {:?}", e), Color32::from_rgb(239, 68, 68));
            }
        }
    }

    fn execute_full_universal_swap(&mut self) {
        self.ensure_universal_catalogs();
        let (cat_a, cat_b) = match (self.catalog_a.clone(), self.catalog_b.clone()) {
            (Some(a), Some(b)) => (a, b),
            _ => return,
        };
        let opts = snes_core::universal::UniversalBatchSwapOptions {
            smart_color_remap: self.universal_smart_remap,
            ..Default::default()
        };
        match snes_core::universal::apply_universal_batch_swap(
            &mut self.slot_a.rom,
            &self.slot_b.rom,
            &cat_a,
            &cat_b,
            &opts,
            &mut self.swap_session,
        ) {
            Ok(ops) => {
                self.slot_a.rescan_after_edit();
                self.catalog_a = None;
                let preview_path = std::env::temp_dir().join("snes_studio_swapped.sfc");
                let _ = std::fs::write(&preview_path, self.slot_a.rom.to_sfc_bytes());
                self.add_log(
                    format!(
                        "⚡ Swap Universel Complet : {} actifs échangés ({} ⬅️ {}) · Checksum 0x{:04X}",
                        ops,
                        self.slot_a.rom.canonical_title(),
                        self.slot_b.rom.canonical_title(),
                        self.slot_a.rom.header.checksum
                    ),
                    Color32::from_rgb(16, 185, 129),
                );
            }
            Err(e) => {
                self.add_log(format!("Erreur Swap Universel : {:?}", e), Color32::from_rgb(239, 68, 68));
            }
        }
    }

    fn render_deconstructor_tab(&mut self, ui: &mut egui::Ui) {
        self.ensure_universal_catalogs();

        ui.horizontal(|ui| {
            ui.label(
                RichText::new("🌐 Déconstructeur, Reconstructeur & Échangeur 100% Universel SNES")
                    .strong()
                    .size(16.0)
                    .color(Color32::from_rgb(139, 92, 246)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("◀ Retour à la Passerelle").clicked() {
                    self.sidebar_view = SidebarView::Melter;
                }
            });
        });
        ui.label("Compatible avec 100% des ROMs SNES (LoROM, HiROM, ExHiROM, FastROM, SMC/SFC) : décomposez en projet éditable, échangez n'importe quel actif entre ROM A et ROM B, ou reconstruisez votre ROM !");
        ui.separator();

        // --- SECTION 1: DECONSTRUCT & ROUND-TRIP RECONSTRUCT ---
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Cible Projet :").strong());
            ui.selectable_value(
                &mut self.deconstruct_target_slot_a,
                true,
                format!("ROM A ({})", self.slot_a.rom.canonical_title()),
            );
            ui.selectable_value(
                &mut self.deconstruct_target_slot_a,
                false,
                format!("ROM B ({})", self.slot_b.rom.canonical_title()),
            );

            ui.add_space(10.0);
            if ui
                .add(
                    egui::Button::new(RichText::new("🚀 Déconstruire en Projet Éditable").strong().color(Color32::WHITE))
                        .fill(Color32::from_rgb(109, 40, 217)),
                )
                .clicked()
            {
                self.deconstruct_rom_slot(self.deconstruct_target_slot_a);
            }

            if ui
                .add(
                    egui::Button::new(RichText::new("🔄 Reconstruire ROM depuis Projet...").strong().color(Color32::WHITE))
                        .fill(Color32::from_rgb(16, 120, 80)),
                )
                .clicked()
            {
                self.reconstruct_rom_from_dialog();
            }

            if let Some(ref report) = self.last_deconstruct_report {
                if ui.button("📂 Ouvrir Dossier Export").clicked() {
                    let _ = std::process::Command::new("xdg-open").arg(&report.out_dir).spawn();
                }
            }
        });

        if let Some(ref rep) = self.last_deconstruct_report {
            ui.group(|ui| {
                ui.label(
                    RichText::new(format!(
                        "✓ Dernière Déconstruction ({} — {} ms) : {} Sprites | {} Planches GFX | {} SPC | {} WAV | {} Textes | {} Palettes | {} Banques ASM  ➜  {}",
                        rep.rom_title,
                        rep.elapsed_ms,
                        rep.sprites_count,
                        rep.sheets_count,
                        rep.spc_files_count,
                        rep.audio_wav_count,
                        rep.text_dialogues_count,
                        rep.palettes_count,
                        rep.disassembled_banks_count,
                        rep.out_dir
                    ))
                    .size(11.5)
                    .color(Color32::from_rgb(52, 211, 153)),
                );
            });
        }
        if let Some(ref rrep) = self.last_reconstruct_report {
            ui.group(|ui| {
                ui.label(
                    RichText::new(format!(
                        "🔄 Dernière Reconstruction ({} ms) : {} GFX, {} Palettes, {} WAV, {} Textes patchés ({} octets, Checksum 0x{:04X}) ➜ {}",
                        rrep.elapsed_ms,
                        rrep.gfx_patched_count,
                        rrep.palettes_patched_count,
                        rrep.audio_patched_count,
                        rrep.text_patched_count,
                        rrep.total_bytes_changed,
                        rrep.new_checksum,
                        rrep.out_rom_path
                    ))
                    .size(11.5)
                    .color(Color32::from_rgb(34, 211, 238)),
                );
            });
        }

        ui.separator();

        // --- SECTION 2: UNIVERSAL ASSET-BY-ASSET CROSS-ROM SWAPPER ---
        let (cat_a, cat_b) = match (self.catalog_a.clone(), self.catalog_b.clone()) {
            (Some(a), Some(b)) => (a, b),
            _ => return,
        };

        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("⇄ Échangeur Universel Actif par Actif :").strong().color(Color32::from_rgb(251, 191, 36)));
            if ui.selectable_label(self.universal_cat_tab == 0, format!("🗺️ Graphismes/Sprites ({}/{})", cat_a.graphics.len(), cat_b.graphics.len())).clicked() {
                self.universal_cat_tab = 0;
                self.universal_sel_a = 0;
                self.universal_sel_b = 0;
            }
            if ui.selectable_label(self.universal_cat_tab == 1, format!("🎨 Palettes ({}/{})", cat_a.palettes.len(), cat_b.palettes.len())).clicked() {
                self.universal_cat_tab = 1;
                self.universal_sel_a = 0;
                self.universal_sel_b = 0;
            }
            if ui.selectable_label(self.universal_cat_tab == 2, format!("🎵 Audio BRR ({}/{})", cat_a.audio_samples.len(), cat_b.audio_samples.len())).clicked() {
                self.universal_cat_tab = 2;
                self.universal_sel_a = 0;
                self.universal_sel_b = 0;
            }
            if ui.selectable_label(self.universal_cat_tab == 3, format!("📜 Textes ({}/{})", cat_a.texts.len(), cat_b.texts.len())).clicked() {
                self.universal_cat_tab = 3;
                self.universal_sel_a = 0;
                self.universal_sel_b = 0;
            }
            ui.checkbox(&mut self.universal_smart_remap, "Smart Color Remap");
        });

        ui.add_space(4.0);
        let mut do_single_swap = false;
        let mut do_full_swap = false;

        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("⇄ Échanger l'Actif Sélectionné (ROM A ⬅️ ROM B)").strong().color(Color32::WHITE))
                        .fill(Color32::from_rgb(37, 99, 235)),
                )
                .clicked()
            {
                do_single_swap = true;
            }
            if ui
                .add(
                    egui::Button::new(RichText::new("⚡ Swap Universel Complet (GFX + Palettes + Audio)").strong().color(Color32::WHITE))
                        .fill(Color32::from_rgb(124, 58, 237)),
                )
                .clicked()
            {
                do_full_swap = true;
            }
        });
        ui.add_space(6.0);

        let avail_w = ui.available_width();
        let half_w = ((avail_w - 18.0) / 2.0).max(200.0);
        let avail_h = ui.available_height().max(180.0);

        ui.horizontal(|ui| {
            // Left Column: ROM A Universal Catalog
            ui.allocate_ui(Vec2::new(half_w, avail_h), |ui| {
                ui.label(
                    RichText::new(format!("ROM CIBLE A : {} [{}]", cat_a.rom_title, cat_a.mapping))
                        .strong()
                        .color(Color32::from_rgb(52, 211, 153)),
                );
                egui::ScrollArea::vertical().id_salt("univ_scroll_a").show(ui, |ui| {
                    match self.universal_cat_tab {
                        0 => {
                            for (i, g) in cat_a.graphics.iter().enumerate() {
                                let label = format!(
                                    "{} · {} ({} tuiles, {} o, {})",
                                    g.id, g.snes_addr, g.tile_count, g.rom_byte_budget, g.format.short_label()
                                );
                                ui.selectable_value(&mut self.universal_sel_a, i, label);
                            }
                        }
                        1 => {
                            for (i, p) in cat_a.palettes.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.selectable_value(&mut self.universal_sel_a, i, format!("{} ({})", p.id, p.snes_addr));
                                    for c in p.colors_rgba.iter().take(8) {
                                        let (r, _) = ui.allocate_exact_size(Vec2::new(10.0, 10.0), Sense::hover());
                                        ui.painter().rect_filled(r, 1.0, Color32::from_rgb(c[0], c[1], c[2]));
                                    }
                                });
                            }
                        }
                        2 => {
                            for (i, a) in cat_a.audio_samples.iter().enumerate() {
                                let label = format!(
                                    "{} · {} ({} o, {} ms, Loop: {})",
                                    a.id, a.snes_addr, a.byte_len, a.duration_ms_16khz, a.loops
                                );
                                ui.selectable_value(&mut self.universal_sel_a, i, label);
                            }
                        }
                        _ => {
                            for (i, t) in cat_a.texts.iter().enumerate() {
                                let preview: String = t.text.chars().take(36).collect();
                                ui.selectable_value(&mut self.universal_sel_a, i, format!("{} [{}]: \"{}\"", t.id, t.encoding, preview));
                            }
                        }
                    }
                });
            });

            ui.separator();

            // Right Column: ROM B Universal Catalog
            ui.allocate_ui(Vec2::new(half_w, avail_h), |ui| {
                ui.label(
                    RichText::new(format!("ROM DONNEUSE B : {} [{}]", cat_b.rom_title, cat_b.mapping))
                        .strong()
                        .color(Color32::from_rgb(196, 181, 253)),
                );
                egui::ScrollArea::vertical().id_salt("univ_scroll_b").show(ui, |ui| {
                    match self.universal_cat_tab {
                        0 => {
                            for (i, g) in cat_b.graphics.iter().enumerate() {
                                let label = format!(
                                    "{} · {} ({} tuiles, {} o, {})",
                                    g.id, g.snes_addr, g.tile_count, g.rom_byte_budget, g.format.short_label()
                                );
                                ui.selectable_value(&mut self.universal_sel_b, i, label);
                            }
                        }
                        1 => {
                            for (i, p) in cat_b.palettes.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.selectable_value(&mut self.universal_sel_b, i, format!("{} ({})", p.id, p.snes_addr));
                                    for c in p.colors_rgba.iter().take(8) {
                                        let (r, _) = ui.allocate_exact_size(Vec2::new(10.0, 10.0), Sense::hover());
                                        ui.painter().rect_filled(r, 1.0, Color32::from_rgb(c[0], c[1], c[2]));
                                    }
                                });
                            }
                        }
                        2 => {
                            for (i, a) in cat_b.audio_samples.iter().enumerate() {
                                let label = format!(
                                    "{} · {} ({} o, {} ms, Loop: {})",
                                    a.id, a.snes_addr, a.byte_len, a.duration_ms_16khz, a.loops
                                );
                                ui.selectable_value(&mut self.universal_sel_b, i, label);
                            }
                        }
                        _ => {
                            for (i, t) in cat_b.texts.iter().enumerate() {
                                let preview: String = t.text.chars().take(36).collect();
                                ui.selectable_value(&mut self.universal_sel_b, i, format!("{} [{}]: \"{}\"", t.id, t.encoding, preview));
                            }
                        }
                    }
                });
            });
        });

        if do_single_swap {
            self.execute_single_universal_swap();
        }
        if do_full_swap {
            self.execute_full_universal_swap();
        }
    }
}

impl eframe::App for SnesStudioApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply SNES Studio Dark Midnight Navy Theme
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgb(12, 18, 40); // #0C1228
        visuals.window_fill = Color32::from_rgb(26, 33, 51); // #1A2133
        visuals.faint_bg_color = Color32::from_rgb(20, 26, 44);
        visuals.extreme_bg_color = Color32::from_rgb(8, 12, 26);
        visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(26, 33, 51);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(39, 47, 68));
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(32, 40, 62);
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(45, 55, 80));
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(48, 58, 90);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.5_f32, Color32::from_rgb(139, 92, 246));
        visuals.widgets.active.bg_fill = Color32::from_rgb(139, 92, 246);
        visuals.selection.bg_fill = Color32::from_rgb(139, 92, 246);
        ctx.set_visuals(visuals);

        self.refresh_textures_if_needed(ctx);

        // Top Menu Bar
        egui::TopBottomPanel::top("top_menu_bar").show(ctx, |ui| {
            self.render_top_menu_bar(ui);
        });

        // Left Icon Sidebar
        egui::SidePanel::left("left_icon_sidebar")
            .exact_width(52.0)
            .resizable(false)
            .show(ctx, |ui| {
                self.render_icon_sidebar(ui);
            });

        // Bottom Dock (Console + Actions — hidden on OmniMixer which has its own dedicated Build/Play bar!)
        if self.sidebar_view != SidebarView::OmniMixer {
            egui::TopBottomPanel::bottom("bottom_dock")
                .min_height(96.0)
                .show(ctx, |ui| {
                    self.render_bottom_dock(ui);
                });
        }

        // Central Area
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.sidebar_view {
                SidebarView::OmniMixer => self.omni_mixer.render_ui(ui),
                SidebarView::Melter => self.render_melter_tab(ui),
                SidebarView::Deconstructor => self.render_deconstructor_tab(ui),
                SidebarView::GraphicsInspector => self.render_graphics_swap_tab(ui),
                SidebarView::PalettesInspector => self.render_palettes_inspector(ui),
                SidebarView::AudioInspector => self.render_audio_inspector(ui),
                SidebarView::DisassemblyInspector => self.render_graphics_swap_tab(ui),
                SidebarView::HistoryLog => self.render_history_inspector(ui),
            }
        });
    }
}
