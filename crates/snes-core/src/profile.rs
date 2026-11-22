use crate::graphics::TileFormat;
use crate::rom::SnesRom;

#[derive(Debug, Clone)]
pub struct ProfileGraphicsBookmark {
    pub name: String,
    pub offset: usize,
    pub length: usize,
    pub format: TileFormat,
    pub palette_offset: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct ProfileAudioBookmark {
    pub name: String,
    pub offset: usize,
    pub byte_len: usize,
}

#[derive(Debug, Clone)]
pub struct GameProfile {
    pub profile_name: String,
    pub match_titles: Vec<String>,
    pub match_crc32: Vec<u32>,
    pub description: String,
    pub graphics: Vec<ProfileGraphicsBookmark>,
    pub palettes: Vec<(String, usize)>,
    pub audio_samples: Vec<ProfileAudioBookmark>,
}

impl GameProfile {
    pub fn matches_rom(&self, rom: &SnesRom) -> bool {
        if self.match_crc32.contains(&rom.crc32) {
            return true;
        }
        let recognized = rom.identify_game();
        if self.profile_name.contains("Zelda") && recognized == crate::rom::RecognizedGame::ZeldaAlttp {
            return true;
        }
        if self.profile_name.contains("Super Mario World") && recognized == crate::rom::RecognizedGame::SuperMarioWorld {
            return true;
        }
        let upper_title = rom.header.title.to_uppercase();
        self.match_titles
            .iter()
            .any(|t| upper_title.contains(&t.to_uppercase()) || t.to_uppercase().contains(&upper_title))
    }

    pub fn builtin_profiles() -> Vec<Self> {
        vec![
            Self {
                profile_name: "The Legend of Zelda: A Link to the Past".to_string(),
                match_titles: vec![
                    "THE LEGEND OF ZELDA".to_string(),
                    "LA LEGENDE DE ZELDA".to_string(),
                    "ZELDA".to_string(),
                    "ZELDANODENSETSU".to_string(),
                ],
                match_crc32: vec![0x777AAC2F, 0xB63BFE15, 0x1F8C30E8],
                description:
                    "Profil complet pour The Legend of Zelda: A Link to the Past (Sprites Link 4bpp, Police 2bpp, Palettes Tuniques, Sons N-SPC)"
                        .to_string(),
                graphics: vec![
                    ProfileGraphicsBookmark {
                        name: "Link (Héros d'Hyrule) — Sprites Complets 4bpp".to_string(),
                        offset: 0x080000,
                        length: 0x7000,
                        format: TileFormat::Bpp4,
                        palette_offset: Some(0x0DD308),
                    },
                    ProfileGraphicsBookmark {
                        name: "Soldat d'Hyrule & Gardes (4bpp)".to_string(),
                        offset: 0x088000,
                        length: 0x3000,
                        format: TileFormat::Bpp4,
                        palette_offset: Some(0x0DD308),
                    },
                    ProfileGraphicsBookmark {
                        name: "Ganon au Trident — Boss des Ténèbres (4bpp)".to_string(),
                        offset: 0x098000,
                        length: 0x4000,
                        format: TileFormat::Bpp4,
                        palette_offset: Some(0x0DD308),
                    },
                    ProfileGraphicsBookmark {
                        name: "Objets & Trésors d'Hyrule (4bpp)".to_string(),
                        offset: 0x082000,
                        length: 0x3000,
                        format: TileFormat::Bpp4,
                        palette_offset: Some(0x0DD308),
                    },
                    ProfileGraphicsBookmark {
                        name: "Police Dialogues & HUD Cœurs (2bpp)".to_string(),
                        offset: 0x070000,
                        length: 0x1000,
                        format: TileFormat::Bpp2,
                        palette_offset: Some(0x0DD308),
                    },
                ],
                palettes: vec![
                    ("Palette Link — Tunique Verte Noble (0x0DD308)".to_string(), 0x0DD308),
                    ("Palette Link — Tunique Bleue Aquatique (0x0DD326)".to_string(), 0x0DD326),
                    ("Palette Link — Tunique Rouge Magique (0x0DD344)".to_string(), 0x0DD344),
                    ("Palette Épées & Trésors Hyrule (0x0DD218)".to_string(), 0x0DD218),
                ],
                audio_samples: vec![
                    ProfileAudioBookmark {
                        name: "Instrument / SFX N-SPC #1".to_string(),
                        offset: 0x0C8243,
                        byte_len: 175 * 9,
                    },
                    ProfileAudioBookmark {
                        name: "SFX Épée / Action N-SPC #2".to_string(),
                        offset: 0x0C886A,
                        byte_len: 331 * 9,
                    },
                    ProfileAudioBookmark {
                        name: "SFX Impact / Ennemi N-SPC #3".to_string(),
                        offset: 0x0CAD4B,
                        byte_len: 386 * 9,
                    },
                ],
            },
            Self {
                profile_name: "Demo Hero (SUPER RUST HERO)".to_string(),
                match_titles: vec!["SUPER RUST HERO".to_string()],
                match_crc32: vec![],
                description: "Profil officiel pour la ROM de démonstration A (Héros Chevalier)"
                    .to_string(),
                graphics: vec![
                    ProfileGraphicsBookmark {
                        name: "Sprites Joueur & Objets (4bpp)".to_string(),
                        offset: 0x010000,
                        length: 0x1000,
                        format: TileFormat::Bpp4,
                        palette_offset: Some(0x018000),
                    },
                    ProfileGraphicsBookmark {
                        name: "Police HUD & Icônes (2bpp)".to_string(),
                        offset: 0x014000,
                        length: 0x0800,
                        format: TileFormat::Bpp2,
                        palette_offset: Some(0x018000),
                    },
                ],
                palettes: vec![
                    ("Palette Héros Chevalier (BGR555)".to_string(), 0x018000),
                    ("Palette Forêt Émeraude (BGR555)".to_string(), 0x018020),
                ],
                audio_samples: vec![
                    ProfileAudioBookmark {
                        name: "SFX 01 - Saut Héroïque".to_string(),
                        offset: 0x020000,
                        byte_len: 36 * 9,
                    },
                    ProfileAudioBookmark {
                        name: "SFX 02 - Épée Cristal".to_string(),
                        offset: 0x020200,
                        byte_len: 40 * 9,
                    },
                    ProfileAudioBookmark {
                        name: "INSTR 03 - Basse Synthé (Loop)".to_string(),
                        offset: 0x020400,
                        byte_len: 48 * 9,
                    },
                ],
            },
            Self {
                profile_name: "Demo Villain (CYBER DEMON X)".to_string(),
                match_titles: vec!["CYBER DEMON X".to_string()],
                match_crc32: vec![],
                description: "Profil officiel pour la ROM de démonstration B (Boss Cyber Démon)"
                    .to_string(),
                graphics: vec![
                    ProfileGraphicsBookmark {
                        name: "Sprites Cyber Démon & Drones (4bpp)".to_string(),
                        offset: 0x010000,
                        length: 0x1000,
                        format: TileFormat::Bpp4,
                        palette_offset: Some(0x018000),
                    },
                    ProfileGraphicsBookmark {
                        name: "Police Cyber HUD (2bpp)".to_string(),
                        offset: 0x014000,
                        length: 0x0800,
                        format: TileFormat::Bpp2,
                        palette_offset: Some(0x018000),
                    },
                ],
                palettes: vec![
                    ("Palette Cyber Néon (BGR555)".to_string(), 0x018000),
                    ("Palette Magma Fusion (BGR555)".to_string(), 0x018020),
                ],
                audio_samples: vec![
                    ProfileAudioBookmark {
                        name: "SFX 01 - Rayon Laser Plasma".to_string(),
                        offset: 0x020000,
                        byte_len: 36 * 9,
                    },
                    ProfileAudioBookmark {
                        name: "SFX 02 - Explosion Lourde".to_string(),
                        offset: 0x020200,
                        byte_len: 40 * 9,
                    },
                    ProfileAudioBookmark {
                        name: "INSTR 03 - Sirène Alarme (Loop)".to_string(),
                        offset: 0x020400,
                        byte_len: 48 * 9,
                    },
                ],
            },
            Self {
                profile_name: "Super Mario World".to_string(),
                match_titles: vec!["SUPER MARIOWORLD".to_string(), "SUPER MARIO WORLD".to_string()],
                match_crc32: vec![0xB19ED489, 0x3C41070F, 0x896BF563],
                description: "Profil complet pour Super Mario World (GFX32 Mario, GFX33 Yoshi, GFX00 Champignon/Bloc ?, GFX01 Koopa/Goomba, Palettes BGR555 & 20 Sons N-SPC)".to_string(),
                graphics: vec![
                    ProfileGraphicsBookmark {
                        name: "GFX32 — Sprites Joueur Mario (Petit, Super, Feu, Cape 4bpp)".to_string(),
                        offset: 0x040000,
                        length: 0x4000,
                        format: TileFormat::Bpp4,
                        palette_offset: Some(0x0032C8),
                    },
                    ProfileGraphicsBookmark {
                        name: "GFX33 — Yoshi, Œufs, Pièces & Blocs Animés (LZ2 -> 4bpp)".to_string(),
                        offset: 0x043FC0,
                        length: 0x3000,
                        format: TileFormat::Bpp4,
                        palette_offset: Some(0x003280),
                    },
                    ProfileGraphicsBookmark {
                        name: "GFX00 — Champignon, Fleur, Bloc ?, Étoile, 1UP (LZ2 -> 4bpp)".to_string(),
                        offset: 0x0459F9,
                        length: 0x1000,
                        format: TileFormat::Bpp4,
                        palette_offset: Some(0x003280),
                    },
                    ProfileGraphicsBookmark {
                        name: "GFX01 — Koopa, Goomba, Bullet Bill, Plante, Clé (LZ2 -> 4bpp)".to_string(),
                        offset: 0x046231,
                        length: 0x1000,
                        format: TileFormat::Bpp4,
                        palette_offset: Some(0x003280),
                    },
                    ProfileGraphicsBookmark {
                        name: "GFX28 — Police HUD & Écran Titre SMW (LZ2 -> 2bpp)".to_string(),
                        offset: 0x05C06C,
                        length: 0x0800,
                        format: TileFormat::Bpp2,
                        palette_offset: Some(0x003100),
                    },
                    ProfileGraphicsBookmark {
                        name: "GFX31 — Bowser & Boss Non-Compressés (4bpp)".to_string(),
                        offset: 0x05F800,
                        length: 0x4800,
                        format: TileFormat::Bpp4,
                        palette_offset: Some(0x003280),
                    },
                ],
                palettes: vec![
                    ("Palette Joueur Mario — Rouge & Bleu (0x0032C8)".to_string(), 0x0032C8),
                    ("Palette Sprites SMW — Bloc ?, Champignon, Yoshi (0x003280)".to_string(), 0x003280),
                    ("Palette Joueur Luigi — Vert & Violet (0x0032DC)".to_string(), 0x0032DC),
                    ("Palette Joueur Mario de Feu — Blanc & Rouge (0x0032F0)".to_string(), 0x0032F0),
                    ("Banque Palettes Niveaux & Écran Titre SMW (0x003100)".to_string(), 0x003100),
                ],
                audio_samples: vec![],
            },
        ]
    }

    pub fn detect_for_rom(rom: &SnesRom) -> Option<Self> {
        Self::builtin_profiles()
            .into_iter()
            .find(|p| p.matches_rom(rom))
    }
}
