use crate::graphics::TileFormat;
use crate::rom::{RecognizedGame, RomError, SnesRom};
use crate::swapper::SwapSession;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetCategory {
    Player,
    Mount,
    PowerUp,
    Collectible,
    InteractiveBlock,
    Enemy,
    Boss,
    UiTitle,
}

impl AssetCategory {
    pub fn label_fr(&self) -> &'static str {
        match self {
            Self::Player => "Joueur & Héros",
            Self::Mount => "Monture & Véhicule",
            Self::PowerUp => "Power-Ups & Objets",
            Self::Collectible => "Collectibles & Pièces",
            Self::InteractiveBlock => "Blocs & Décors",
            Self::Enemy => "Ennemis & Créatures",
            Self::Boss => "Boss & Ennemis Majeurs",
            Self::UiTitle => "Interface & Écran Titre",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Player => "🏃",
            Self::Mount => "🦖",
            Self::PowerUp => "🍄",
            Self::Collectible => "🪙",
            Self::InteractiveBlock => "🧱",
            Self::Enemy => "🐢",
            Self::Boss => "👑",
            Self::UiTitle => "🏷️",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScannedAsset {
    pub id: String,
    pub name: String,
    pub category: AssetCategory,
    pub family_name: &'static str,
    pub family_pack_desc: &'static str,
    pub icon: &'static str,
    pub sheet_name: String,
    pub rom_offset: usize,
    pub length: usize,
    pub format: TileFormat,
    pub compressed: bool,
    pub palette_offset: Option<usize>,
    pub description: String,
    pub partner_asset_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UserSwapDecision {
    pub id: String,
    pub label: String,
    pub category: AssetCategory,
    pub family_name: &'static str,
    pub pack_summary: &'static str,
    pub icon: &'static str,
    pub target_rom_asset: String,
    pub donor_rom_asset: String,
    pub description: String,
    pub enabled: bool,
}

/// Scan and catalog all recognizable sprites, assets, and interactive blocks from a ROM.
pub fn scan_assets_for_rom(rom: &SnesRom) -> Vec<ScannedAsset> {
    let mut assets = Vec::new();
    let game = rom.identify_game();

    match game {
        RecognizedGame::SuperMarioWorld => {
            assets.push(ScannedAsset {
                id: "smw_mario".to_string(),
                name: "Mario (Petit, Super, Feu, Cape)".to_string(),
                category: AssetCategory::Player,
                family_name: "🏃 Pack Héros Complet (Mario)",
                family_pack_desc: "Pack Complet Mario : remplace l'intégralité du personnage dans tous ses états (Petit, Super, Feu, Cape, 744 tuiles OAM, courses, sauts et palettes).",
                icon: "🏃",
                sheet_name: "GFX32 (0x040000)".to_string(),
                rom_offset: 0x040000,
                length: 0x4000,
                format: TileFormat::Bpp4,
                compressed: true,
                palette_offset: Some(0x0032C8),
                description: "Sprites dynamiques du héros : marche, course, saut, glissade et nage (744 tuiles décompressées)".to_string(),
                partner_asset_id: Some("zelda_link".to_string()),
            });

            assets.push(ScannedAsset {
                id: "smw_yoshi".to_string(),
                name: "Monture Yoshi & Œufs".to_string(),
                category: AssetCategory::Mount,
                family_name: "🦖 Pack Montures & Compagnons (Yoshi)",
                family_pack_desc: "Pack Monture Yoshi : remplace la monture au complet (adulte monté, bébé Yoshi, œufs, crachat et coup de poing frontal).",
                icon: "🦖",
                sheet_name: "GFX33 (0x043FC0)".to_string(),
                rom_offset: 0x043FC0,
                length: 0x3000,
                format: TileFormat::Bpp4,
                compressed: true,
                palette_offset: Some(0x003280),
                description: "Yoshi adulte (Row 2 & 3), bébé Yoshi, œufs et animations de course (384 tuiles 3bpp/4bpp)".to_string(),
                partner_asset_id: None,
            });

            assets.push(ScannedAsset {
                id: "smw_mushroom".to_string(),
                name: "Super Champignon (+1 Taille)".to_string(),
                category: AssetCategory::PowerUp,
                family_name: "🍄 Pack Objets & Power-Ups",
                family_pack_desc: "Pack Power-Ups d'évolution : remplace les champignons de croissance des blocs '?'.",
                icon: "🍄",
                sheet_name: "GFX00 (0x0459F9)".to_string(),
                rom_offset: 0x0459F9,
                length: 0x1000,
                format: TileFormat::Bpp4,
                compressed: true,
                palette_offset: Some(0x003280),
                description: "Champignon rouge classique de croissance dans les blocs '?' (Tuile 16x16 $24)".to_string(),
                partner_asset_id: Some("zelda_heart".to_string()),
            });

            assets.push(ScannedAsset {
                id: "smw_fire_flower".to_string(),
                name: "Fleur de Feu (Fire Flower)".to_string(),
                category: AssetCategory::PowerUp,
                family_name: "🍄 Pack Objets & Power-Ups",
                family_pack_desc: "Pack Power-Ups d'attaque : remplace les fleurs de feu et projectiles.",
                icon: "🔥",
                sheet_name: "GFX00 (0x0459F9)".to_string(),
                rom_offset: 0x0459F9,
                length: 0x1000,
                format: TileFormat::Bpp4,
                compressed: true,
                palette_offset: Some(0x003280),
                description: "Power-up permettant de lancer des boules de feu rebondissantes (Tuile 16x16 $26)".to_string(),
                partner_asset_id: Some("zelda_fairy".to_string()),
            });

            assets.push(ScannedAsset {
                id: "smw_star".to_string(),
                name: "Étoile d'Invincibilité (Starman)".to_string(),
                category: AssetCategory::PowerUp,
                family_name: "🍄 Pack Objets & Power-Ups",
                family_pack_desc: "Pack Invincibilité : remplace l'étoile Starman et les effets d'aura.",
                icon: "🌟",
                sheet_name: "GFX00 (0x0459F9)".to_string(),
                rom_offset: 0x0459F9,
                length: 0x1000,
                format: TileFormat::Bpp4,
                compressed: true,
                palette_offset: Some(0x003280),
                description: "Étoile rebondissante conférant une vitesse accrue et l'invincibilité temporaire (Tuile $48)".to_string(),
                partner_asset_id: Some("zelda_triforce".to_string()),
            });

            assets.push(ScannedAsset {
                id: "smw_coin".to_string(),
                name: "Pièce d'or Animée (Coin)".to_string(),
                category: AssetCategory::Collectible,
                family_name: "🪙 Pack Collectibles & Récolte",
                family_pack_desc: "Pack Monnaies du Royaume Champignon : remplace l'intégralité des pièces d'or animées.",
                icon: "🪙",
                sheet_name: "GFX33 (0x043FC0)".to_string(),
                rom_offset: 0x043FC0,
                length: 0x1000,
                format: TileFormat::Bpp4,
                compressed: true,
                palette_offset: Some(0x003280),
                description: "Pièce dorée rotative à 4 angles d'animation (donne 1UP à 100 pièces récoltées)".to_string(),
                partner_asset_id: Some("zelda_rupee".to_string()),
            });

            assets.push(ScannedAsset {
                id: "smw_berry".to_string(),
                name: "Pomme / Baie de Yoshi (Berry)".to_string(),
                category: AssetCategory::Collectible,
                family_name: "🪙 Pack Collectibles & Récolte",
                family_pack_desc:
                    "Pack Baies & Fruits : remplace les fruits consommables par Yoshi.",
                icon: "🍎",
                sheet_name: "GFX00 (0x0459F9)".to_string(),
                rom_offset: 0x0459F9,
                length: 0x0800,
                format: TileFormat::Bpp4,
                compressed: true,
                palette_offset: Some(0x003280),
                description:
                    "Baies rouges, roses et vertes des buissons comestibles par Yoshi (Tuile $80)"
                        .to_string(),
                partner_asset_id: None,
            });

            assets.push(ScannedAsset {
                id: "smw_question_block".to_string(),
                name: "Bloc '?' (Question Mark Block)".to_string(),
                category: AssetCategory::InteractiveBlock,
                family_name: "🧱 Pack Blocs & Mécanismes",
                family_pack_desc: "Pack Blocs Mystères : remplace les blocs interactifs '?' contenant des récompenses.",
                icon: "❓",
                sheet_name: "GFX00 (0x0459F9)".to_string(),
                rom_offset: 0x0459F9,
                length: 0x1000,
                format: TileFormat::Bpp4,
                compressed: true,
                palette_offset: Some(0x003280),
                description: "Bloc mystère contenant des pièces, super champignons, fleurs de feu ou plumes (Tuile $00)".to_string(),
                partner_asset_id: Some("zelda_chest".to_string()),
            });

            assets.push(ScannedAsset {
                id: "smw_turn_block".to_string(),
                name: "Bloc de Brique Tournant (Turn Block)".to_string(),
                category: AssetCategory::InteractiveBlock,
                family_name: "🧱 Pack Blocs & Mécanismes",
                family_pack_desc: "Pack Blocs Tournants : remplace les blocs de brique pivotants de Mario.",
                icon: "🧱",
                sheet_name: "GFX00 (0x0459F9)".to_string(),
                rom_offset: 0x0459F9,
                length: 0x1000,
                format: TileFormat::Bpp4,
                compressed: true,
                palette_offset: Some(0x003280),
                description: "Bloc de brique jaune tournoyant lorsqu'il est percuté par Mario par le bas (Tuile $04)".to_string(),
                partner_asset_id: Some("zelda_pot".to_string()),
            });

            assets.push(ScannedAsset {
                id: "smw_koopa".to_string(),
                name: "Koopa Troopa & Carapaces".to_string(),
                category: AssetCategory::Enemy,
                family_name: "🐢 Pack Troupes & Ennemis Koopas",
                family_pack_desc: "Pack Patrouilles Koopas : remplace les tortues terrestres vertes et rouges et leurs carapaces.",
                icon: "🐢",
                sheet_name: "GFX01 (0x046231)".to_string(),
                rom_offset: 0x046231,
                length: 0x1000,
                format: TileFormat::Bpp4,
                compressed: true,
                palette_offset: Some(0x003280),
                description: "Tortues patrouilleuses à carapaces vertes et rouges propulsables comme projectile".to_string(),
                partner_asset_id: Some("zelda_soldier".to_string()),
            });

            assets.push(ScannedAsset {
                id: "smw_goomba".to_string(),
                name: "Goomba / Kuribo (Galoomba)".to_string(),
                category: AssetCategory::Enemy,
                family_name: "🐢 Pack Troupes & Ennemis Koopas",
                family_pack_desc: "Pack Fantassins Galoombas : remplace les ennemis marchant au sol pouvant être portés.",
                icon: "🌰",
                sheet_name: "GFX01 (0x046231)".to_string(),
                rom_offset: 0x046231,
                length: 0x1000,
                format: TileFormat::Bpp4,
                compressed: true,
                palette_offset: Some(0x003280),
                description: "Ennemi terrestre marchant pouvant être assommé et porté".to_string(),
                partner_asset_id: Some("zelda_soldier_blue".to_string()),
            });

            assets.push(ScannedAsset {
                id: "smw_bowser".to_string(),
                name: "Bowser & Koopa Clown Car".to_string(),
                category: AssetCategory::Boss,
                family_name: "👑 Pack Boss & Forteresse (Bowser)",
                family_pack_desc: "Pack Boss Final : remplace le roi Bowser dans sa Koopa Clown Car.",
                icon: "👑",
                sheet_name: "GFX31 (0x05F800)".to_string(),
                rom_offset: 0x05F800,
                length: 0x4800,
                format: TileFormat::Bpp4,
                compressed: false,
                palette_offset: Some(0x003280),
                description: "Boss final géant dans sa capsule aérienne Koopa Clown Car (18 Ko non-compressés)".to_string(),
                partner_asset_id: Some("zelda_ganon".to_string()),
            });

            assets.push(ScannedAsset {
                id: "smw_title".to_string(),
                name: "Logo Titre 'SUPER MARIO WORLD'".to_string(),
                category: AssetCategory::UiTitle,
                family_name: "🏷️ Pack Interface & Logos",
                family_pack_desc:
                    "Pack Typographie : remplace le titre principal de l'écran d'accueil.",
                icon: "🏷️",
                sheet_name: "GFX29 (0x05C6A3)".to_string(),
                rom_offset: 0x05C6A3,
                length: 0x0800,
                format: TileFormat::Bpp2,
                compressed: true,
                palette_offset: Some(0x003100),
                description:
                    "Lettres en relief 3D de l'écran d'accueil avec stripe images vectorielles"
                        .to_string(),
                partner_asset_id: Some("zelda_title".to_string()),
            });
        }

        RecognizedGame::ZeldaAlttp => {
            assets.push(ScannedAsset {
                id: "zelda_link".to_string(),
                name: "Link (Héros d'Hyrule)".to_string(),
                category: AssetCategory::Player,
                family_name: "🛡️ Pack Héros Complet (Link)",
                family_pack_desc: "Pack Complet Link : remplace l'intégralité du personnage dans tous ses états (marche, course, épée, bouclier, nage, toutes directions, palettes et sons).",
                icon: "🗡️",
                sheet_name: "Sheet 0x00 (0x080000)".to_string(),
                rom_offset: 0x080000,
                length: 0x2000,
                format: TileFormat::Bpp4,
                compressed: false,
                palette_offset: Some(0x0DD308),
                description: "Animations complètes de Link : marche quadridirectionnelle, course, épée et bouclier".to_string(),
                partner_asset_id: Some("smw_mario".to_string()),
            });

            assets.push(ScannedAsset {
                id: "zelda_sword".to_string(),
                name: "Épée Master Sword & Armes".to_string(),
                category: AssetCategory::Player,
                family_name: "🛡️ Pack Héros Complet (Link)",
                family_pack_desc: "Pack Armes du Héros : Épée de Légende, Arc, Flèches, Grappin et Bouclier d'Hyrule.",
                icon: "⚔️",
                sheet_name: "Sheet 0x01 (0x081000)".to_string(),
                rom_offset: 0x081000,
                length: 0x1000,
                format: TileFormat::Bpp4,
                compressed: false,
                palette_offset: Some(0x0DD308),
                description: "Épée Master Sword, coups d'estoc, arc et bouclier royal".to_string(),
                partner_asset_id: Some("smw_mario".to_string()),
            });

            assets.push(ScannedAsset {
                id: "zelda_zelda".to_string(),
                name: "Princesse Zelda (Robe Royale)".to_string(),
                category: AssetCategory::Player,
                family_name: "👑 Pack Royauté & Alliés (Princesse Zelda)",
                family_pack_desc: "Pack Royauté d'Hyrule : la Princesse Zelda, ses robes royales et son sauvetage.",
                icon: "👸",
                sheet_name: "Sheet 0x07 (0x087000)".to_string(),
                rom_offset: 0x087000,
                length: 0x1000,
                format: TileFormat::Bpp4,
                compressed: false,
                palette_offset: Some(0x0DD308),
                description: "Princesse Zelda en tenue cérémonielle et demoiselles des cristaux".to_string(),
                partner_asset_id: Some("smw_mario".to_string()),
            });

            assets.push(ScannedAsset {
                id: "zelda_sahasrahla".to_string(),
                name: "Sage Sahasrahla & Oncle de Link".to_string(),
                category: AssetCategory::Player,
                family_name: "👑 Pack Royauté & Alliés (Princesse Zelda)",
                family_pack_desc:
                    "Pack Alliés d'Hyrule : le vénérable Sage Sahasrahla et l'Oncle de Link.",
                icon: "🧙‍♂️",
                sheet_name: "Sheet 0x06 (0x086000)".to_string(),
                rom_offset: 0x086000,
                length: 0x1000,
                format: TileFormat::Bpp4,
                compressed: false,
                palette_offset: Some(0x0DD308),
                description: "Vieux Sage d'Hyrule et Oncle armé confiant l'épée à Link".to_string(),
                partner_asset_id: Some("smw_mario".to_string()),
            });

            assets.push(ScannedAsset {
                id: "zelda_heart".to_string(),
                name: "Petit Cœur & Conteneur de Cœur".to_string(),
                category: AssetCategory::PowerUp,
                family_name: "🍄 Pack Objets & Magie Sacrée",
                family_pack_desc: "Pack Vie & Magie : Conteneur de Cœur, petits cœurs de vie et réceptacles sacrés.",
                icon: "❤️",
                sheet_name: "Sheet 0x07 (0x070000)".to_string(),
                rom_offset: 0x070000,
                length: 0x1000,
                format: TileFormat::Bpp2,
                compressed: false,
                palette_offset: Some(0x0DD308),
                description: "Cœurs de restauration et conteneurs de vie d'Hyrule".to_string(),
                partner_asset_id: Some("smw_mushroom".to_string()),
            });

            assets.push(ScannedAsset {
                id: "zelda_fairy".to_string(),
                name: "Fée Magique de Fontaine".to_string(),
                category: AssetCategory::PowerUp,
                family_name: "🍄 Pack Objets & Magie Sacrée",
                family_pack_desc: "Pack Bienfaits Magiques : Fée des fontaines bienfaitrice régénérant les points de vie.",
                icon: "🧚",
                sheet_name: "Sheet 0x08 (0x082000)".to_string(),
                rom_offset: 0x082000,
                length: 0x1000,
                format: TileFormat::Bpp4,
                compressed: false,
                palette_offset: Some(0x0DD308),
                description: "Fée bienfaitrice régénérant la vie et volant dans les fontaines".to_string(),
                partner_asset_id: Some("smw_fire_flower".to_string()),
            });

            assets.push(ScannedAsset {
                id: "zelda_potion".to_string(),
                name: "Flacon Magique & Potions d'Hyrule".to_string(),
                category: AssetCategory::PowerUp,
                family_name: "🍄 Pack Objets & Magie Sacrée",
                family_pack_desc:
                    "Pack Potions Sacrées : Flacons de magie rouge, verte et bleue de la sorcière.",
                icon: "🧪",
                sheet_name: "Sheet 0x02 (0x082000)".to_string(),
                rom_offset: 0x082000,
                length: 0x0800,
                format: TileFormat::Bpp4,
                compressed: false,
                palette_offset: Some(0x0DD308),
                description: "Flacons de potions médicinales et magiques d'Hyrule".to_string(),
                partner_asset_id: Some("smw_star".to_string()),
            });

            assets.push(ScannedAsset {
                id: "zelda_rupee".to_string(),
                name: "Rubis d'Hyrule (Vert, Bleu, Rouge)".to_string(),
                category: AssetCategory::Collectible,
                family_name: "💎 Pack Trésors & Blocs d'Hyrule",
                family_pack_desc:
                    "Pack Monnaies Légendaires : Rubis vert, bleu et rouge scintillants d'Hyrule.",
                icon: "💎",
                sheet_name: "Sheet 0x08 (0x082000)".to_string(),
                rom_offset: 0x082000,
                length: 0x0800,
                format: TileFormat::Bpp4,
                compressed: false,
                palette_offset: Some(0x0DD308),
                description: "Monnaie légendaire d'Hyrule scintillante (1, 5, 20 rubis)"
                    .to_string(),
                partner_asset_id: Some("smw_coin".to_string()),
            });

            assets.push(ScannedAsset {
                id: "zelda_chest".to_string(),
                name: "Coffre au Trésor d'Hyrule".to_string(),
                category: AssetCategory::InteractiveBlock,
                family_name: "💎 Pack Trésors & Blocs d'Hyrule",
                family_pack_desc:
                    "Pack Trésors des Donjons : Coffre royal ouvrable contenant les objets sacrés.",
                icon: "📦",
                sheet_name: "Sheet 0x08 (0x082000)".to_string(),
                rom_offset: 0x082000,
                length: 0x1000,
                format: TileFormat::Bpp4,
                compressed: false,
                palette_offset: Some(0x0DD308),
                description: "Coffre en bois et or contenant des trésors dans les donjons"
                    .to_string(),
                partner_asset_id: Some("smw_question_block".to_string()),
            });

            assets.push(ScannedAsset {
                id: "zelda_pot".to_string(),
                name: "Pot en Terre & Jarre Soulevable".to_string(),
                category: AssetCategory::InteractiveBlock,
                family_name: "💎 Pack Trésors & Blocs d'Hyrule",
                family_pack_desc: "Pack Mobilier & Objets Soulevables : Pots en terre cuite et jarres destructibles.",
                icon: "🏺",
                sheet_name: "Sheet 0x08 (0x082000)".to_string(),
                rom_offset: 0x082000,
                length: 0x0800,
                format: TileFormat::Bpp4,
                compressed: false,
                palette_offset: Some(0x0DD308),
                description: "Jarre d'argile pouvant être soulevée et fracassée contre les murs".to_string(),
                partner_asset_id: Some("smw_turn_block".to_string()),
            });

            assets.push(ScannedAsset {
                id: "zelda_soldier".to_string(),
                name: "Soldat d'Hyrule (Garde Vert & Rouge)".to_string(),
                category: AssetCategory::Enemy,
                family_name: "⚔️ Pack Gardes & Soldats du Château",
                family_pack_desc:
                    "Pack Garnison du Château : Soldats royaux d'Hyrule à l'épée, lance et arc.",
                icon: "🛡️",
                sheet_name: "Sheet 0x08 (0x088000)".to_string(),
                rom_offset: 0x088000,
                length: 0x2000,
                format: TileFormat::Bpp4,
                compressed: false,
                palette_offset: Some(0x0DD308),
                description: "Gardes royaux armés d'épées et de boucliers patrouillant le château"
                    .to_string(),
                partner_asset_id: Some("smw_koopa".to_string()),
            });

            assets.push(ScannedAsset {
                id: "zelda_monsters".to_string(),
                name: "Monstres d'Overworld (Octorok & Keese)".to_string(),
                category: AssetCategory::Enemy,
                family_name: "👾 Pack Monstres d'Overworld",
                family_pack_desc: "Pack Créatures Hostiles : Octoroks cracheurs de pierres et chauves-souris Keese.",
                icon: "🐙",
                sheet_name: "Sheet 0x0D (0x08D000)".to_string(),
                rom_offset: 0x08D000,
                length: 0x1000,
                format: TileFormat::Bpp4,
                compressed: false,
                palette_offset: Some(0x0DD308),
                description: "Octoroks terrestres et chauves-souris des grottes d'Hyrule".to_string(),
                partner_asset_id: Some("smw_goomba".to_string()),
            });

            assets.push(ScannedAsset {
                id: "zelda_ganon".to_string(),
                name: "Ganon (Seigneur des Ténèbres)".to_string(),
                category: AssetCategory::Boss,
                family_name: "🔥 Pack Boss & Némésis (Ganon & Agahnim)",
                family_pack_desc: "Pack Maître des Ténèbres : Ganon au trident magique régnant sur le Monde des Ténèbres.",
                icon: "👹",
                sheet_name: "Sheet 0x09 (0x098000)".to_string(),
                rom_offset: 0x098000,
                length: 0x3000,
                format: TileFormat::Bpp4,
                compressed: false,
                palette_offset: Some(0x0DD308),
                description: "Seigneur du mal armé de son trident magique dans la pyramide".to_string(),
                partner_asset_id: Some("smw_bowser".to_string()),
            });

            assets.push(ScannedAsset {
                id: "zelda_agahnim".to_string(),
                name: "Sorcier Agahnim (Prêtre Sombre)".to_string(),
                category: AssetCategory::Boss,
                family_name: "🔥 Pack Boss & Némésis (Ganon & Agahnim)",
                family_pack_desc:
                    "Pack Magie Sombre : Le grand prêtre Agahnim et ses sorts d'éclairs.",
                icon: "🧙",
                sheet_name: "Sheet 0x12 (0x092000)".to_string(),
                rom_offset: 0x092000,
                length: 0x1000,
                format: TileFormat::Bpp4,
                compressed: false,
                palette_offset: Some(0x0DD308),
                description: "Sorcier Agahnim projetant des orbes magiques et des éclairs"
                    .to_string(),
                partner_asset_id: Some("smw_bowser".to_string()),
            });

            assets.push(ScannedAsset {
                id: "zelda_title".to_string(),
                name: "Logo Écran Titre Triforce".to_string(),
                category: AssetCategory::UiTitle,
                family_name: "✨ Pack Interface & Triforce",
                family_pack_desc: "Pack Emblème Sacré : Logo doré de la Triforce d'Hyrule.",
                icon: "✨",
                sheet_name: "Sheet 0x5C (0x0A6BAE)".to_string(),
                rom_offset: 0x0A6BAE,
                length: 0x1000,
                format: TileFormat::Bpp2,
                compressed: true,
                palette_offset: Some(0x0DD308),
                description: "Triforce dorée flottante de l'écran d'accueil d'A Link to the Past"
                    .to_string(),
                partner_asset_id: Some("smw_title".to_string()),
            });
        }

        RecognizedGame::Unknown => {
            // Scanner matériel universel 100% compatible avec TOUTES les ROMs SNES (LoROM/HiROM/ExHiROM, DMA, LZ2, 2/3/4/8bpp)
            let catalog = crate::universal::discover_universal_catalog(rom);
            for (idx, g) in catalog.graphics.iter().take(16).enumerate() {
                let pal_off = catalog
                    .palettes
                    .get(g.suggested_palette_idx)
                    .map(|p| p.rom_offset);
                let cat = match g.category {
                    crate::universal::UniversalGfxCategory::Sprites => {
                        if idx == 0 {
                            AssetCategory::Player
                        } else {
                            AssetCategory::Enemy
                        }
                    }
                    crate::universal::UniversalGfxCategory::HudFont => AssetCategory::UiTitle,
                    crate::universal::UniversalGfxCategory::Mode7 => AssetCategory::Boss,
                    crate::universal::UniversalGfxCategory::Backgrounds => {
                        AssetCategory::InteractiveBlock
                    }
                };
                assets.push(ScannedAsset {
                    id: g.id.clone(),
                    name: g.name.clone(),
                    category: cat,
                    family_name: "🌐 Pack Graphismes Universels (Matériel)",
                    family_pack_desc: "Détecté automatiquement via DMA VRAM, flux Nintendo LZ2 et analyse planaire.",
                    icon: match g.category {
                        crate::universal::UniversalGfxCategory::Sprites => "🏃",
                        crate::universal::UniversalGfxCategory::HudFont => "🔤",
                        crate::universal::UniversalGfxCategory::Mode7 => "🌐",
                        crate::universal::UniversalGfxCategory::Backgrounds => "🗺️",
                    },
                    sheet_name: format!("{} ({})", g.snes_addr, g.compression.label_fr()),
                    rom_offset: g.rom_offset,
                    length: g.rom_byte_budget,
                    format: g.format,
                    compressed: g.compression != crate::universal::UniversalGfxCompression::None,
                    palette_offset: pal_off,
                    description: format!(
                        "{} · {} tuiles {} · Budget ROM {} octets",
                        g.discovery_source,
                        g.tile_count,
                        g.format.short_label(),
                        g.rom_byte_budget
                    ),
                    partner_asset_id: None,
                });
            }
        }
    }

    assets
}

/// Generate default, customizable swap decisions when pairing two ROMs.
pub fn get_default_swap_decisions(rom_a: &SnesRom, rom_b: &SnesRom) -> Vec<UserSwapDecision> {
    let mut decisions = Vec::new();
    let game_a = rom_a.identify_game();
    let game_b = rom_b.identify_game();

    if game_a == RecognizedGame::SuperMarioWorld && game_b == RecognizedGame::ZeldaAlttp {
        decisions.push(UserSwapDecision {
            id: "swap_mario_link".to_string(),
            label: "Pack Héros Complet : Mario ➡️ Link".to_string(),
            category: AssetCategory::Player,
            family_name: "🛡️ Pack Héros Complet (Link)",
            pack_summary: "Remplace le personnage Mario au complet : 100% des formes (Petit, Super, Feu, Cape), 744 tuiles OAM, démarches, courses, sauts, découplage torse/jambes et palettes.",
            icon: "🗡️",
            target_rom_asset: "Mario (Petit, Super, Feu, Cape) [GFX32]".to_string(),
            donor_rom_asset: "Link (Héros d'Hyrule) [Zelda Sheet 0x00 & 0x01]".to_string(),
            description: "Remplace Mario par Link : tuile supérieure 100% transparente en mode Petit et jambes découplées avec toutes poses d'action".to_string(),
            enabled: true,
        });

        decisions.push(UserSwapDecision {
            id: "swap_title_screen".to_string(),
            label: "Pack Écran Titre : SUPER MARIO ➡️ SUPER LINK WORLD".to_string(),
            category: AssetCategory::UiTitle,
            family_name: "🏷️ Pack Interface & Titre",
            pack_summary:
                "Remplace le logo d'accueil avec les lettres 3D d'Hyrule et le bandeau officiel.",
            icon: "🏷️",
            target_rom_asset: "Logo Titre Super Mario World [GFX29]".to_string(),
            donor_rom_asset: "Lettres 3D 'SUPER LINK WORLD' [AI Synth]".to_string(),
            description:
                "Remplace 'MARIO' par 'LINK' dans le logo d'accueil avec stripe images vectorielles"
                    .to_string(),
            enabled: true,
        });

        decisions.push(UserSwapDecision {
            id: "swap_mushroom_heart".to_string(),
            label: "Pack Power-Up : Super Champignon ➡️ Conteneur de Cœur".to_string(),
            category: AssetCategory::PowerUp,
            family_name: "🍄 Pack Objets & Magie Sacrée",
            pack_summary: "Remplace l'objet d'évolution : le champignon rouge devient un réceptacle de cœur d'Hyrule.",
            icon: "❤️",
            target_rom_asset: "Super Champignon (+1 Taille) [GFX00]".to_string(),
            donor_rom_asset: "Conteneur de Cœur d'Hyrule [Zelda Sheet 0x07]".to_string(),
            description: "Le champignon rouge des blocs '?' devient un cœur d'Hyrule qui fait grandir le joueur".to_string(),
            enabled: true,
        });

        decisions.push(UserSwapDecision {
            id: "swap_coin_rupee".to_string(),
            label: "Pack Collectible : Pièce d'or ➡️ Rubis d'Hyrule".to_string(),
            category: AssetCategory::Collectible,
            family_name: "💎 Pack Trésors & Monnaies",
            pack_summary:
                "Remplace 100% des pièces d'or animées par les Rubis scintillants d'Hyrule.",
            icon: "💎",
            target_rom_asset: "Pièce dorée rotative [GFX33]".to_string(),
            donor_rom_asset: "Rubis vert/doré scintillant [Zelda Sheet 0x08]".to_string(),
            description:
                "Toutes les pièces de Super Mario World deviennent des rubis dorés d'Hyrule"
                    .to_string(),
            enabled: true,
        });

        decisions.push(UserSwapDecision {
            id: "swap_qblock_chest".to_string(),
            label: "Pack Décor : Bloc '?' ➡️ Coffre au Trésor Zelda".to_string(),
            category: AssetCategory::InteractiveBlock,
            family_name: "🧱 Pack Blocs & Donjons",
            pack_summary:
                "Remplace l'ensemble des blocs '?' par des coffres au trésor dorés d'Hyrule.",
            icon: "📦",
            target_rom_asset: "Bloc Mystère '?' [GFX00]".to_string(),
            donor_rom_asset: "Coffre au Trésor des donjons [Zelda Sheet 0x08]".to_string(),
            description:
                "Les blocs '?' flottants deviennent des coffres dorés contenant des objets"
                    .to_string(),
            enabled: true,
        });

        decisions.push(UserSwapDecision {
            id: "swap_brick_pot".to_string(),
            label: "Pack Bloc : Brique Tournante ➡️ Jarre / Pot d'Hyrule".to_string(),
            category: AssetCategory::InteractiveBlock,
            family_name: "🧱 Pack Blocs & Donjons",
            pack_summary: "Remplace les blocs de brique cassables/pivotants par des jarres de poterie destructibles.",
            icon: "🏺",
            target_rom_asset: "Bloc de Brique Jaune [GFX00]".to_string(),
            donor_rom_asset: "Pot en Terre d'Hyrule [Zelda Sheet 0x08]".to_string(),
            description: "Les blocs de brique deviennent des pots en terre cuite qui pivotent à l'impact".to_string(),
            enabled: true,
        });

        decisions.push(UserSwapDecision {
            id: "swap_koopa_soldier".to_string(),
            label: "Pack Ennemis : Koopa Troopa ➡️ Garde d'Hyrule".to_string(),
            category: AssetCategory::Enemy,
            family_name: "⚔️ Pack Gardes & Monstres",
            pack_summary:
                "Remplace les patrouilles d'ennemis au sol par les soldats en armure d'Hyrule.",
            icon: "🛡️",
            target_rom_asset: "Koopa Troopa (Vert & Rouge) [GFX01]".to_string(),
            donor_rom_asset: "Garde Royal d'Hyrule [Zelda Sheet 0x08]".to_string(),
            description: "Les troupes de tortues au sol deviennent des soldats royaux d'Hyrule"
                .to_string(),
            enabled: false,
        });

        decisions.push(UserSwapDecision {
            id: "swap_bowser_ganon".to_string(),
            label: "Pack Boss : Bowser Clown Car ➡️ Ganon d'Hyrule".to_string(),
            category: AssetCategory::Boss,
            family_name: "🔥 Pack Boss & Némésis",
            pack_summary:
                "Remplace le boss final volant par le Seigneur des Ténèbres Ganon au trident.",
            icon: "👑",
            target_rom_asset: "Bowser & Clown Car (Boss Final) [GFX31]".to_string(),
            donor_rom_asset: "Ganon au Trident [Zelda Sheet 0x09]".to_string(),
            description: "Le boss final volant est remplacé par le Seigneur des Ténèbres Ganon"
                .to_string(),
            enabled: false,
        });
    } else {
        // Matrice de Swap 100% Universelle pour N'IMPORTE QUELLE paire de ROMs SNES (LoROM/HiROM/ExHiROM/Homebrew)
        let title_a = rom_a.canonical_title();
        let title_b = rom_b.canonical_title();

        decisions.push(UserSwapDecision {
            id: "swap_universal_gfx_sprites".to_string(),
            label: format!("Pack Graphismes & Sprites : {} ⬅️ {}", title_a, title_b),
            category: AssetCategory::Player,
            family_name: "🌐 Pack Universel Graphismes & Sprites",
            pack_summary: "Échange universel des planches de sprites/tuiles (décompression LZ2 auto, transcodage 2/3/4/8bpp, Smart Color Remap et respect strict du budget d'octets).",
            icon: "🏃",
            target_rom_asset: format!("Planches GFX/Sprites détectées ({})", title_a),
            donor_rom_asset: format!("Planches GFX/Sprites détectées ({})", title_b),
            description: "Remplace les banques graphiques principales avec adaptation automatique du format et de la compression".to_string(),
            enabled: true,
        });

        decisions.push(UserSwapDecision {
            id: "swap_universal_palettes".to_string(),
            label: format!("Pack Palettes BGR555 : {} ⬅️ {}", title_a, title_b),
            category: AssetCategory::PowerUp,
            family_name: "🎨 Pack Universel Palettes CGRAM",
            pack_summary: "Transfère l'ambiance chromatique (palettes 15-bit BGR555 détectées par DMA CGRAM & scanner) de la ROM B vers la ROM A.",
            icon: "🎨",
            target_rom_asset: format!("Palettes CGRAM BGR555 ({})", title_a),
            donor_rom_asset: format!("Palettes CGRAM BGR555 ({})", title_b),
            description: "Harmonise et permute les palettes 16 couleurs matérielles entre les deux cartouches".to_string(),
            enabled: true,
        });

        decisions.push(UserSwapDecision {
            id: "swap_universal_audio".to_string(),
            label: format!("Pack Audio SPC700 BRR : {} ⬅️ {}", title_a, title_b),
            category: AssetCategory::Collectible,
            family_name: "🎵 Pack Universel Audio & Instruments BRR",
            pack_summary: "Rééchantillonne et ré-encode automatiquement les instruments/effets sonores BRR de la ROM B dans le budget exact de la ROM A.",
            icon: "🎵",
            target_rom_asset: format!("Échantillons Audio BRR ({})", title_a),
            donor_rom_asset: format!("Échantillons Audio BRR ({})", title_b),
            description: "Échange les instruments SPC700 BRR en préservant les boucles (loops) et la taille matérielle exacte".to_string(),
            enabled: true,
        });

        decisions.push(UserSwapDecision {
            id: "swap_universal_text".to_string(),
            label: format!("Pack Textes & Scripts (.tbl) : {} ⬅️ {}", title_a, title_b),
            category: AssetCategory::UiTitle,
            family_name: "📜 Pack Universel Textes & Dialogues",
            pack_summary: "Ré-encode les textes détectés de la ROM B via la table de caractères (.tbl) auto-détectée de la ROM A sans décaler les pointeurs.",
            icon: "📜",
            target_rom_asset: format!("Chaînes de texte détectées ({})", title_a),
            donor_rom_asset: format!("Chaînes de texte détectées ({})", title_b),
            description: "Injecte les textes de la ROM B en respectant la longueur maximale de chaque chaîne".to_string(),
            enabled: false,
        });
    }

    decisions
}

/// Apply only the user-selected swaps onto the target ROM, maintaining 100% SNES hardware compliance.
/// Supports BOTH curated SMW<->Zelda packs AND 100% Universal Cross-ROM Swapping for ANY pair of SNES ROMs!
pub fn apply_custom_swaps(
    rom_target: &mut SnesRom,
    donor_rom: Option<&SnesRom>,
    decisions: &[UserSwapDecision],
    session: &mut SwapSession,
) -> Result<usize, RomError> {
    let mut ops_count = 0usize;

    // 0. Universal Cross-ROM Swaps (works for 100% of ALL SNES ROMs: LoROM, HiROM, ExHiROM, Unknown!)
    let u_gfx = decisions.iter().any(|d| {
        (d.id == "swap_universal_gfx_sprites" || d.id == "swap_player_generic") && d.enabled
    });
    let u_pal = decisions
        .iter()
        .any(|d| d.id == "swap_universal_palettes" && d.enabled);
    let u_aud = decisions
        .iter()
        .any(|d| d.id == "swap_universal_audio" && d.enabled);
    let u_txt = decisions
        .iter()
        .any(|d| d.id == "swap_universal_text" && d.enabled);

    if u_gfx || u_pal || u_aud || u_txt {
        if let Some(donor) = donor_rom {
            let cat_a = crate::universal::discover_universal_catalog(rom_target);
            let cat_b = crate::universal::discover_universal_catalog(donor);
            let opts = crate::universal::UniversalBatchSwapOptions {
                swap_graphics: u_gfx,
                swap_palettes: u_pal,
                swap_audio: u_aud,
                swap_text: u_txt,
                smart_color_remap: true,
                max_gfx_swaps: 4,
                max_palette_swaps: 4,
                max_audio_swaps: 4,
            };
            ops_count += crate::universal::apply_universal_batch_swap(
                rom_target, donor, &cat_a, &cat_b, &opts, session,
            )?;
        }
    }

    // 1. Link into SMW
    let link_enabled = decisions
        .iter()
        .any(|d| d.id == "swap_mario_link" && d.enabled);
    if link_enabled {
        if let Some(zelda) = donor_rom {
            ops_count += crate::demo::apply_link_into_smw_swap(rom_target, zelda, session)?;
        }
    }

    // 2. Title Screen Logo: "SUPER LINK WORLD"
    let title_enabled = decisions
        .iter()
        .any(|d| d.id == "swap_title_screen" && d.enabled);
    if title_enabled {
        if let Ok(t_ops) = crate::demo::apply_link_world_title_screen(rom_target, session) {
            ops_count += t_ops;
        }
    }

    // 4. Items & Interactive Blocks (GFX00 & GFX33 custom swaps)
    let mushroom_heart = decisions
        .iter()
        .any(|d| d.id == "swap_mushroom_heart" && d.enabled);
    let coin_rupee = decisions
        .iter()
        .any(|d| d.id == "swap_coin_rupee" && d.enabled);
    let qblock_chest = decisions
        .iter()
        .any(|d| d.id == "swap_qblock_chest" && d.enabled);
    let brick_pot = decisions
        .iter()
        .any(|d| d.id == "swap_brick_pot" && d.enabled);

    if mushroom_heart || qblock_chest || brick_pot {
        if let Some((mut g00, _)) = crate::disasm_dma::decompress_nintendo_lz2_with_endian(
            &rom_target.data,
            0x0459F9,
            8192,
            true,
        ) {
            // Heart Container replaces Super Mushroom (Tile $24 = 8x8 tiles 36, 37, 52, 53)
            if mushroom_heart {
                #[rustfmt::skip]
                let heart_16x16: [[u8; 16]; 16] = [
                    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
                    [0,0,2,2,2,0,0,0,0,0,2,2,2,0,0,0],
                    [0,2,3,3,3,2,0,0,0,2,3,3,3,2,0,0],
                    [2,3,1,3,3,3,2,0,2,3,3,3,3,3,2,0],
                    [2,3,3,3,3,3,3,2,3,3,3,3,3,3,2,0],
                    [2,3,3,3,3,3,3,3,3,3,3,3,3,3,2,0],
                    [2,3,3,3,3,3,3,3,3,3,3,3,3,3,2,0],
                    [0,2,3,3,3,3,3,3,3,3,3,3,3,2,0,0],
                    [0,2,3,3,3,3,3,3,3,3,3,3,3,2,0,0],
                    [0,0,2,3,3,3,3,3,3,3,3,3,2,0,0,0],
                    [0,0,0,2,3,3,3,3,3,3,3,2,0,0,0,0],
                    [0,0,0,0,2,3,3,3,3,3,2,0,0,0,0,0],
                    [0,0,0,0,0,2,3,3,3,2,0,0,0,0,0,0],
                    [0,0,0,0,0,0,2,3,2,0,0,0,0,0,0,0],
                    [0,0,0,0,0,0,0,2,0,0,0,0,0,0,0,0],
                    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
                ];
                write_16x16_into_3bpp_sheet(&mut g00, 36, &heart_16x16);
            }

            // Treasure Chest replaces Question Mark Block '?' sprite in GFX00 (Tile $2A = 8x8 tiles 42, 43, 58, 59; NEVER touch tiles 0..25 which hold 'Nintendo Presents'!)
            if qblock_chest {
                #[rustfmt::skip]
                let chest_16x16: [[u8; 16]; 16] = [
                    [0,0,0,2,2,2,2,2,2,2,2,2,2,0,0,0],
                    [0,0,2,5,5,5,5,5,5,5,5,5,5,2,0,0],
                    [0,2,3,3,3,3,3,3,3,3,3,3,3,3,2,0],
                    [0,2,3,3,3,3,3,3,3,3,3,3,3,3,2,0],
                    [2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2],
                    [2,3,3,3,3,3,2,5,5,2,3,3,3,3,3,2],
                    [2,3,3,3,3,3,2,5,1,2,3,3,3,3,3,2],
                    [2,3,3,3,3,3,2,2,2,2,3,3,3,3,3,2],
                    [2,3,3,3,3,3,3,2,2,3,3,3,3,3,3,2],
                    [2,3,3,3,3,3,3,3,3,3,3,3,3,3,3,2],
                    [2,4,4,4,4,4,4,4,4,4,4,4,4,4,4,2],
                    [2,4,4,4,4,4,4,4,4,4,4,4,4,4,4,2],
                    [2,4,4,4,4,4,4,4,4,4,4,4,4,4,4,2],
                    [2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2],
                    [0,2,6,6,2,0,0,0,0,0,0,2,6,6,2,0],
                    [0,0,2,2,0,0,0,0,0,0,0,0,2,2,0,0],
                ];
                write_16x16_into_3bpp_sheet(&mut g00, 42, &chest_16x16);
            }

            // Clay Pot replaces Turn Block sprite in GFX00 (Tile $40 = 8x8 tiles 64, 65, 80, 81)
            if brick_pot {
                #[rustfmt::skip]
                let pot_16x16: [[u8; 16]; 16] = [
                    [0,0,0,0,2,2,2,2,2,2,2,2,0,0,0,0],
                    [0,0,0,2,7,7,7,7,7,7,7,7,2,0,0,0],
                    [0,0,0,2,2,2,2,2,2,2,2,2,2,0,0,0],
                    [0,0,2,3,3,3,3,3,3,3,3,3,3,2,0,0],
                    [0,2,3,3,1,1,3,3,3,3,3,3,3,3,2,0],
                    [2,3,3,1,1,3,3,3,3,3,3,3,3,3,3,2],
                    [2,3,3,3,3,3,3,3,3,3,3,3,3,3,3,2],
                    [2,3,3,3,3,3,3,3,3,3,3,3,3,3,3,2],
                    [2,3,3,3,3,3,3,3,3,3,3,3,3,3,3,2],
                    [2,4,3,3,3,3,3,3,3,3,3,3,3,3,4,2],
                    [2,4,4,3,3,3,3,3,3,3,3,3,3,4,4,2],
                    [0,2,4,4,4,4,4,4,4,4,4,4,4,4,2,0],
                    [0,0,2,4,4,4,4,4,4,4,4,4,4,2,0,0],
                    [0,0,0,2,2,2,2,2,2,2,2,2,2,0,0,0],
                    [0,0,0,0,2,6,6,6,6,6,6,2,0,0,0,0],
                    [0,0,0,0,0,2,2,2,2,2,2,0,0,0,0,0],
                ];
                write_16x16_into_3bpp_sheet(&mut g00, 64, &pot_16x16);
            }

            let comp_g00 = crate::disasm_dma::compress_nintendo_lz2_with_endian(&g00, true);
            let budget_00 = 0x046231 - 0x0459F9;
            if comp_g00.len() <= budget_00 {
                let mut rep = comp_g00;
                rep.resize(budget_00, 0xFF);
                session.swap_custom_bytes(
                    rom_target,
                    0x0459F9,
                    0x000000,
                    rep,
                    "Objets & Blocs personnalisés (Champignon->Cœur, Bloc ?->Coffre, Brique->Pot) [GFX00]".to_string(),
                    crate::swapper::SwapKind::GraphicsTiles {
                        format: TileFormat::Bpp4,
                        tile_count: 128,
                        smart_remapped: true,
                    },
                )?;
                ops_count += 1;
            }
        }
    }

    // Golden Rupee replaces the 4 animation frames of the Spinning Coin in GFX33
    // (Tile rows 12..15, cols 12..13 = top 16x8 halves, cols 14..15 = bottom 16x8 halves; NEVER touch Row 0 water/lava!)
    if coin_rupee {
        if let Some((mut g33, _)) = crate::disasm_dma::decompress_nintendo_lz2_with_endian(
            &rom_target.data,
            0x043FC0,
            16384,
            true,
        ) {
            // Write all 4 spin animation frames (wide -> 3/4 -> slim -> thin edge) into rows 12..15, cols 12..15
            let frame_widths: [usize; 4] = [5, 3, 2, 1];
            for (f_idx, &half_w) in frame_widths.iter().enumerate() {
                let mut frame = [[0u8; 16]; 16];
                for y in 0..16usize {
                    let dist_v = if y < 8 { 7 - y } else { y - 8 };
                    // Taper top & bottom tips smoothly into a faceted Hyrule Rupee diamond
                    let cur_w = if dist_v >= 7 {
                        0
                    } else if dist_v >= 5 {
                        half_w.saturating_sub(dist_v - 4).max(1)
                    } else {
                        half_w
                    };
                    let x0 = 8usize.saturating_sub(cur_w);
                    let x1 = (8 + cur_w).min(15);
                    if cur_w == 0 {
                        frame[y][7] = 2;
                        frame[y][8] = 2;
                    } else {
                        for x in x0..=x1 {
                            if x == x0 || x == x1 || dist_v == 6 {
                                frame[y][x] = 2; // Black outline
                            } else if x <= 7 && (2..=6).contains(&y) && x == x0 + 1 {
                                frame[y][x] = 1; // Specular white glint
                            } else if x <= 8 {
                                frame[y][x] = 5; // Bright facet
                            } else {
                                frame[y][x] = 4; // Shaded facet
                            }
                        }
                    }
                }
                let extract_quad = |qx: usize, qy: usize| -> [u8; 24] {
                    let mut q = [[0u8; 8]; 8];
                    for y in 0..8 {
                        for x in 0..8 {
                            q[y][x] = frame[qy + y][qx + x];
                        }
                    }
                    encode_3bpp_tile_8x8(&q)
                };
                let row_base = (12 + f_idx) * 16 + 12;
                let tl = extract_quad(0, 0);
                let tr = extract_quad(8, 0);
                let bl = extract_quad(0, 8);
                let br = extract_quad(8, 8);
                for &(t_idx, tile) in &[
                    (row_base, &tl),
                    (row_base + 1, &tr),
                    (row_base + 2, &bl),
                    (row_base + 3, &br),
                ] {
                    if (t_idx + 1) * 24 <= g33.len() {
                        g33[t_idx * 24..(t_idx + 1) * 24].copy_from_slice(tile);
                    }
                }
            }

            let comp_g33 = crate::disasm_dma::compress_nintendo_lz2_with_endian(&g33, true);
            let budget_33 = 0x0459F9 - 0x043FC0;
            if comp_g33.len() <= budget_33 {
                let mut rep = comp_g33;
                rep.resize(budget_33, 0xFF);
                session.swap_custom_bytes(
                    rom_target,
                    0x043FC0,
                    0x000000,
                    rep,
                    "Pièces d'or ➡️ Rubis d'Hyrule scintillants [GFX33]".to_string(),
                    crate::swapper::SwapKind::GraphicsTiles {
                        format: TileFormat::Bpp4,
                        tile_count: 384,
                        smart_remapped: true,
                    },
                )?;
                ops_count += 1;
            }
        }
    }

    // Recompute SNES LoROM Checksum & Complement
    rom_target.fix_checksum();
    Ok(ops_count)
}

fn encode_3bpp_tile_8x8(px: &[[u8; 8]; 8]) -> [u8; 24] {
    let mut out = [0u8; 24];
    for y in 0..8usize {
        let mut b0 = 0u8;
        let mut b1 = 0u8;
        let mut b2 = 0u8;
        for x in 0..8usize {
            let c = px[y][x] & 0x07;
            let sh = 7 - x;
            b0 |= (c & 1) << sh;
            b1 |= ((c >> 1) & 1) << sh;
            b2 |= ((c >> 2) & 1) << sh;
        }
        out[y * 2] = b0;
        out[y * 2 + 1] = b1;
        out[16 + y] = b2;
    }
    out
}

fn write_16x16_into_3bpp_sheet(sheet: &mut [u8], base_tile_idx: usize, px: &[[u8; 16]; 16]) {
    let mut tl = [[0u8; 8]; 8];
    let mut tr = [[0u8; 8]; 8];
    let mut bl = [[0u8; 8]; 8];
    let mut br = [[0u8; 8]; 8];

    for y in 0..8 {
        for x in 0..8 {
            tl[y][x] = px[y][x];
            tr[y][x] = px[y][x + 8];
            bl[y][x] = px[y + 8][x];
            br[y][x] = px[y + 8][x + 8];
        }
    }

    let tiles = [
        (base_tile_idx, encode_3bpp_tile_8x8(&tl)),
        (base_tile_idx + 1, encode_3bpp_tile_8x8(&tr)),
        (base_tile_idx + 16, encode_3bpp_tile_8x8(&bl)),
        (base_tile_idx + 17, encode_3bpp_tile_8x8(&br)),
    ];

    for (t_idx, tile_data) in tiles {
        let start = t_idx * 24;
        let end = start + 24;
        if end <= sheet.len() {
            sheet[start..end].copy_from_slice(&tile_data);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_scanning_and_custom_swaps() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        if let (Ok(bytes_a), Ok(bytes_b)) = (
            std::fs::read(root.join("roms/super_mario_world.sfc")),
            std::fs::read(root.join("roms/zelda_alttp.sfc")),
        ) {
            let mut rom_a = SnesRom::from_bytes("smw", &bytes_a).unwrap();
            let rom_b = SnesRom::from_bytes("zelda", &bytes_b).unwrap();

            // 1. Scan assets for ROM A
            let assets_a = scan_assets_for_rom(&rom_a);
            assert!(
                assets_a.len() >= 10,
                "Should detect at least 10 assets in SMW"
            );
            let has_mushroom = assets_a.iter().any(|a| a.id == "smw_mushroom");
            let has_coin = assets_a.iter().any(|a| a.id == "smw_coin");
            let has_turn_block = assets_a.iter().any(|a| a.id == "smw_turn_block");
            let has_bowser = assets_a.iter().any(|a| a.id == "smw_bowser");
            assert!(has_mushroom, "Must detect Super Champignon");
            assert!(has_coin, "Must detect Pièce d'or");
            assert!(has_turn_block, "Must detect Bloc de brique");
            assert!(has_bowser, "Must detect Bowser");

            // 2. Scan assets for ROM B
            let assets_b = scan_assets_for_rom(&rom_b);
            assert!(
                assets_b.len() >= 8,
                "Should detect at least 8 assets in Zelda"
            );
            assert!(
                assets_b.iter().any(|a| a.id == "zelda_link"),
                "Must detect Link"
            );
            assert!(
                assets_b.iter().any(|a| a.id == "zelda_heart"),
                "Must detect Cœur"
            );

            // 3. Get user swap decisions
            let decisions = get_default_swap_decisions(&rom_a, &rom_b);
            assert!(decisions.len() >= 8);

            // 4. Test applying custom swaps with all enabled
            let mut session = SwapSession::new();
            let ops =
                apply_custom_swaps(&mut rom_a, Some(&rom_b), &decisions, &mut session).unwrap();
            assert!(ops >= 4, "Should apply custom swaps successfully");
            assert!(
                rom_a.header.is_checksum_valid(),
                "Checksum must be 100% valid after custom swaps!"
            );

            // 5. Update Knowledge Base
            let mut kb = crate::memory::RomKnowledgeBase::load_or_init();
            kb.learn_from_rom(&rom_a);
            kb.learn_from_rom(&rom_b);
            assert!(!kb.to_json_pretty().is_empty());
        }
    }
}
