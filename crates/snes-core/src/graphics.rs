#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileFormat {
    /// 2 bits per pixel (4 colors, 16 bytes per 8x8 tile) - Layer 3 HUD/fonts
    Bpp2,
    /// 3 bits per pixel (8 colors, 24 bytes per 8x8 tile) - Compact SNES sprites & BG (SMW, ALTTP, FF4/5/6)
    Bpp3,
    /// 4 bits per pixel (16 colors, 32 bytes per 8x8 tile) - Standard SNES sprites & BG1/BG2
    Bpp4,
    /// 8 bits per pixel planar (256 colors, 64 bytes per 8x8 tile) - Mode 3/4
    Bpp8,
    /// 8 bits per pixel linear (256 colors, 64 bytes per 8x8 tile) - Mode 7
    Mode7Linear,
}

impl TileFormat {
    pub fn bytes_per_tile(self) -> usize {
        match self {
            TileFormat::Bpp2 => 16,
            TileFormat::Bpp3 => 24,
            TileFormat::Bpp4 => 32,
            TileFormat::Bpp8 => 64,
            TileFormat::Mode7Linear => 64,
        }
    }

    pub fn color_count(self) -> usize {
        match self {
            TileFormat::Bpp2 => 4,
            TileFormat::Bpp3 => 8,
            TileFormat::Bpp4 => 16,
            TileFormat::Bpp8 | TileFormat::Mode7Linear => 256,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            TileFormat::Bpp2 => "2bpp (4 couleurs - HUD/Polices, 16 octets/tile)",
            TileFormat::Bpp3 => "3bpp (8 couleurs - Sprites/Décors compacts, 24 octets/tile)",
            TileFormat::Bpp4 => "4bpp (16 couleurs - Sprites/Décors, 32 octets/tile)",
            TileFormat::Bpp8 => "8bpp Planar (256 couleurs - Mode 3/4, 64 octets/tile)",
            TileFormat::Mode7Linear => "Mode 7 Linéaire (256 couleurs, 64 octets/tile)",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            TileFormat::Bpp2 => "2bpp",
            TileFormat::Bpp3 => "3bpp",
            TileFormat::Bpp4 => "4bpp",
            TileFormat::Bpp8 => "8bpp",
            TileFormat::Mode7Linear => "Mode7",
        }
    }

    pub fn from_short_label(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "2bpp" => Some(TileFormat::Bpp2),
            "3bpp" => Some(TileFormat::Bpp3),
            "4bpp" => Some(TileFormat::Bpp4),
            "8bpp" => Some(TileFormat::Bpp8),
            "mode7" | "mode7linear" => Some(TileFormat::Mode7Linear),
            _ => None,
        }
    }
}

/// Convert a 15-bit SNES BGR555 color (`0bbbbbgggggrrrrr`) to 8-bit `[R, G, B, A]`
pub fn bgr555_to_rgba(bgr: u16, transparent_zero: bool, is_index_zero: bool) -> [u8; 4] {
    if transparent_zero && is_index_zero {
        return [20, 22, 30, 255];
    }
    let r5 = (bgr & 0x1F) as u8;
    let g5 = ((bgr >> 5) & 0x1F) as u8;
    let b5 = ((bgr >> 10) & 0x1F) as u8;

    let r = (r5 << 3) | (r5 >> 2);
    let g = (g5 << 3) | (g5 >> 2);
    let b = (b5 << 3) | (b5 >> 2);
    [r, g, b, 255]
}

/// Convert 8-bit `[R, G, B]` to 15-bit SNES BGR555 (`0bbbbbgggggrrrrr`)
pub fn rgb_to_bgr555(r: u8, g: u8, b: u8) -> u16 {
    let r5 = ((r >> 3) & 0x1F) as u16;
    let g5 = ((g >> 3) & 0x1F) as u16;
    let b5 = ((b >> 3) & 0x1F) as u16;
    r5 | (g5 << 5) | (b5 << 10)
}

#[derive(Debug, Clone)]
pub struct SnesPalette {
    pub name: String,
    pub source_offset: Option<usize>,
    pub colors_rgba: Vec<[u8; 4]>,
}

impl SnesPalette {
    /// Extract a BGR555 palette directly from ROM bytes at `offset`
    pub fn from_rom_bgr555(
        name: impl Into<String>,
        data: &[u8],
        offset: usize,
        num_colors: usize,
    ) -> Self {
        let mut colors_rgba = Vec::with_capacity(256);
        for i in 0..256 {
            let byte_pos = offset + (i % num_colors.max(1)) * 2;
            if byte_pos + 1 < data.len() {
                let raw = u16::from_le_bytes([data[byte_pos], data[byte_pos + 1]]);
                colors_rgba.push(bgr555_to_rgba(raw, false, i == 0));
            } else {
                let v = ((i * 17) & 0xFF) as u8;
                colors_rgba.push([v, v, v, 255]);
            }
        }
        Self {
            name: name.into(),
            source_offset: Some(offset),
            colors_rgba,
        }
    }

    /// Construct a vibrant authentic Mario World 16-color CGRAM sprite palette
    pub fn mario_classic_full_16(rom_data: Option<&[u8]>) -> Self {
        let mut base_16: [[u8; 4]; 16] = [
            [18, 22, 32, 255],    // 0: Transparent backdrop
            [255, 255, 255, 255], // 1: White gloves / eyes
            [24, 24, 28, 255],    // 2: Dark outline
            [232, 40, 24, 255],   // 3: Mario Red Cap & Shirt
            [160, 16, 16, 255],   // 4: Deep Red Shadow
            [255, 208, 160, 255], // 5: Peach skin tone
            [216, 144, 96, 255],  // 6: Warm skin shadow
            [96, 48, 16, 255],    // 7: Hair & moustache brown
            [48, 112, 232, 255],  // 8: Overalls Bright Blue
            [16, 48, 144, 255],   // 9: Overalls Deep Blue Shadow
            [255, 224, 32, 255],  // 10: Golden Yellow buttons / cape
            [200, 152, 16, 255],  // 11: Dark Gold cape shadow
            [255, 232, 208, 255], // 12: Skin highlight
            [136, 72, 24, 255],   // 13: Brown shoes
            [240, 240, 255, 255], // 14: White glove highlight
            [0, 0, 0, 255],       // 15: Pure black
        ];

        // If ROM has been modified with custom palette bytes at 0x0032C8 (e.g. Link tunics), overlay them:
        if let Some(rom) = rom_data {
            if rom.len() >= 0x0032C8 + 20 {
                let first_word = u16::from_le_bytes([rom[0x0032C8], rom[0x0032C9]]);
                // Only overlay if non-standard or swapped (such as Link green mail)
                if first_word != 0x1F00 && first_word != 0x0000 && first_word != 0x0019 && first_word != 0x108C {
                    for i in 0..10 {
                        let bpos = 0x0032C8 + i * 2;
                        let raw = u16::from_le_bytes([rom[bpos], rom[bpos + 1]]);
                        let col = bgr555_to_rgba(raw, false, false);
                        base_16[6 + i] = col;
                    }
                }
            }
        }

        Self::expand_16_to_256("Palette Joueur Mario — Rouge & Bleu Authentique (Vibrant)", &base_16)
    }

    /// Construct an authentic Link 16-color tunic palette directly from Zelda ALTTP ROM
    pub fn zelda_link_full_16(rom_data: &[u8], tunic_offset: usize, name: &str, is_ai_blonde: bool) -> Self {
        let mut base_16: [[u8; 4]; 16] = [
            [18, 22, 32, 255],    // 0: Transparent
            [248, 248, 96, 255],  // 1: Yellow belt
            [216, 136, 64, 255],  // 2: Brown straps
            [248, 192, 136, 255], // 3: Peach skin
            [72, 72, 72, 255],    // 4: Dark outline
            [248, 152, 32, 255],  // 5: Orange highlight
            [224, 56, 64, 255],   // 6: Red shield
            [248, 128, 208, 255], // 7: Pink hair (or Blonde)
            [88, 176, 136, 255],  // 8: Green tunic highlight
            [96, 248, 144, 255],  // 9: Emerald Green tunic
            [112, 176, 48, 255],  // 10: Deep Green shadow
            [152, 216, 64, 255],  // 11: Forest Green shadow
            [248, 176, 112, 255], // 12: Skin highlight
            [168, 120, 72, 255],  // 13: Brown boots
            [224, 160, 248, 255], // 14: Light Purple / highlight
            [248, 248, 248, 255], // 15: White sword / eyes
        ];

        if tunic_offset + 30 <= rom_data.len() {
            for i in 1..16 {
                let pos = tunic_offset + (i - 1) * 2;
                let raw = u16::from_le_bytes([rom_data[pos], rom_data[pos + 1]]);
                base_16[i] = bgr555_to_rgba(raw, false, false);
            }
        }

        if is_ai_blonde {
            base_16[7] = [180, 156, 48, 255]; // Blond noble IA
            base_16[2] = [120, 90, 32, 255];  // Ombre dorée
        }

        let mut pal = Self::expand_16_to_256(name, &base_16);
        pal.source_offset = Some(tunic_offset);
        pal
    }

    pub fn preset_palettes() -> Vec<Self> {
        vec![
            Self::preset_snes_adventure(),
            Self::preset_cyber_neon(),
            Self::preset_grayscale(),
            Self::preset_fire_gold(),
        ]
    }

    pub fn preset_snes_adventure() -> Self {
        let base_16: [[u8; 4]; 16] = [
            [18, 20, 28, 255],    // 0: Backdrop dark
            [40, 44, 68, 255],    // 1: Outline dark blue-black
            [235, 65, 55, 255],   // 2: Tunic / Cap Red
            [165, 30, 35, 255],   // 3: Dark Red shadow
            [250, 205, 165, 255], // 4: Skin light
            [210, 145, 100, 255], // 5: Skin warm shadow
            [65, 145, 245, 255],  // 6: Bright Blue
            [35, 80, 175, 255],   // 7: Deep Blue
            [75, 210, 85, 255],   // 8: Emerald Green
            [35, 135, 50, 255],   // 9: Forest Green
            [255, 225, 65, 255],  // 10: Coin / Gold Yellow
            [215, 145, 30, 255],  // 11: Ochre / Bronze
            [150, 95, 55, 255],   // 12: Brown leather
            [95, 55, 30, 255],    // 13: Dark Brown
            [210, 220, 235, 255], // 14: Silver / Steel
            [255, 255, 255, 255], // 15: Pure White highlight
        ];
        Self::expand_16_to_256("SNES Classique (Aventure 16c)", &base_16)
    }

    pub fn preset_cyber_neon() -> Self {
        let base_16: [[u8; 4]; 16] = [
            [12, 10, 24, 255],
            [32, 20, 58, 255],
            [255, 45, 145, 255],
            [175, 20, 95, 255],
            [0, 240, 255, 255],
            [0, 150, 185, 255],
            [165, 85, 255, 255],
            [95, 40, 175, 255],
            [80, 255, 155, 255],
            [25, 165, 95, 255],
            [255, 235, 75, 255],
            [245, 130, 40, 255],
            [140, 150, 185, 255],
            [75, 85, 120, 255],
            [215, 235, 255, 255],
            [255, 255, 255, 255],
        ];
        Self::expand_16_to_256("Cyber Néon (Sci-Fi 16c)", &base_16)
    }

    pub fn preset_fire_gold() -> Self {
        let base_16: [[u8; 4]; 16] = [
            [20, 12, 12, 255],
            [50, 22, 18, 255],
            [110, 28, 20, 255],
            [165, 42, 25, 255],
            [220, 68, 28, 255],
            [245, 110, 30, 255],
            [255, 160, 45, 255],
            [255, 210, 75, 255],
            [255, 245, 150, 255],
            [255, 255, 240, 255],
            [125, 105, 90, 255],
            [85, 70, 62, 255],
            [60, 135, 195, 255],
            [100, 195, 245, 255],
            [180, 150, 115, 255],
            [235, 215, 185, 255],
        ];
        Self::expand_16_to_256("Feu & Or (Boss / Dragon)", &base_16)
    }

    pub fn preset_grayscale() -> Self {
        let mut colors_rgba = Vec::with_capacity(256);
        for i in 0..256 {
            let v = if i < 16 {
                ((i * 17) & 0xFF) as u8
            } else {
                (i & 0xFF) as u8
            };
            colors_rgba.push([v, v, v, 255]);
        }
        Self {
            name: "Niveaux de gris ( Contraste 4bpp/8bpp )".to_string(),
            source_offset: None,
            colors_rgba,
        }
    }

    fn expand_16_to_256(name: &str, base_16: &[[u8; 4]; 16]) -> Self {
        let mut colors_rgba = Vec::with_capacity(256);
        for i in 0..256 {
            colors_rgba.push(base_16[i % 16]);
        }
        Self {
            name: name.to_string(),
            source_offset: None,
            colors_rgba,
        }
    }
}

/// Decode a single 8x8 SNES tile into 64 palette indices (row-major, 8x8)
pub fn decode_tile(tile_bytes: &[u8], format: TileFormat) -> [u8; 64] {
    let mut out = [0u8; 64];
    let needed = format.bytes_per_tile();
    if tile_bytes.len() < needed {
        return out;
    }

    match format {
        TileFormat::Bpp2 => {
            for y in 0..8 {
                let bp0 = tile_bytes[y * 2];
                let bp1 = tile_bytes[y * 2 + 1];
                for x in 0..8 {
                    let shift = 7 - x;
                    let c = ((bp0 >> shift) & 1) | (((bp1 >> shift) & 1) << 1);
                    out[y * 8 + x] = c;
                }
            }
        }
        TileFormat::Bpp3 => {
            for y in 0..8 {
                let bp0 = tile_bytes[y * 2];
                let bp1 = tile_bytes[y * 2 + 1];
                let bp2 = tile_bytes[16 + y];
                for x in 0..8 {
                    let shift = 7 - x;
                    let c = ((bp0 >> shift) & 1)
                        | (((bp1 >> shift) & 1) << 1)
                        | (((bp2 >> shift) & 1) << 2);
                    out[y * 8 + x] = c;
                }
            }
        }
        TileFormat::Bpp4 => {
            for y in 0..8 {
                let bp0 = tile_bytes[y * 2];
                let bp1 = tile_bytes[y * 2 + 1];
                let bp2 = tile_bytes[16 + y * 2];
                let bp3 = tile_bytes[16 + y * 2 + 1];
                for x in 0..8 {
                    let shift = 7 - x;
                    let c = ((bp0 >> shift) & 1)
                        | (((bp1 >> shift) & 1) << 1)
                        | (((bp2 >> shift) & 1) << 2)
                        | (((bp3 >> shift) & 1) << 3);
                    out[y * 8 + x] = c;
                }
            }
        }
        TileFormat::Bpp8 => {
            for y in 0..8 {
                let bp0 = tile_bytes[y * 2];
                let bp1 = tile_bytes[y * 2 + 1];
                let bp2 = tile_bytes[16 + y * 2];
                let bp3 = tile_bytes[16 + y * 2 + 1];
                let bp4 = tile_bytes[32 + y * 2];
                let bp5 = tile_bytes[32 + y * 2 + 1];
                let bp6 = tile_bytes[48 + y * 2];
                let bp7 = tile_bytes[48 + y * 2 + 1];
                for x in 0..8 {
                    let shift = 7 - x;
                    let c = ((bp0 >> shift) & 1)
                        | (((bp1 >> shift) & 1) << 1)
                        | (((bp2 >> shift) & 1) << 2)
                        | (((bp3 >> shift) & 1) << 3)
                        | (((bp4 >> shift) & 1) << 4)
                        | (((bp5 >> shift) & 1) << 5)
                        | (((bp6 >> shift) & 1) << 6)
                        | (((bp7 >> shift) & 1) << 7);
                    out[y * 8 + x] = c;
                }
            }
        }
        TileFormat::Mode7Linear => {
            out.copy_from_slice(&tile_bytes[..64]);
        }
    }

    out
}

/// Encode 64 palette indices (8x8 row-major) into SNES planar tile bytes
pub fn encode_tile(indices: &[u8; 64], format: TileFormat) -> Vec<u8> {
    let mut out = vec![0u8; format.bytes_per_tile()];
    match format {
        TileFormat::Bpp2 => {
            for y in 0..8 {
                for x in 0..8 {
                    let shift = 7 - x;
                    let c = indices[y * 8 + x] & 0x03;
                    out[y * 2] |= (c & 1) << shift;
                    out[y * 2 + 1] |= ((c >> 1) & 1) << shift;
                }
            }
        }
        TileFormat::Bpp3 => {
            for y in 0..8 {
                for x in 0..8 {
                    let shift = 7 - x;
                    let c = indices[y * 8 + x] & 0x07;
                    out[y * 2] |= (c & 1) << shift;
                    out[y * 2 + 1] |= ((c >> 1) & 1) << shift;
                    out[16 + y] |= ((c >> 2) & 1) << shift;
                }
            }
        }
        TileFormat::Bpp4 => {
            for y in 0..8 {
                for x in 0..8 {
                    let shift = 7 - x;
                    let c = indices[y * 8 + x] & 0x0F;
                    out[y * 2] |= (c & 1) << shift;
                    out[y * 2 + 1] |= ((c >> 1) & 1) << shift;
                    out[16 + y * 2] |= ((c >> 2) & 1) << shift;
                    out[16 + y * 2 + 1] |= ((c >> 3) & 1) << shift;
                }
            }
        }
        TileFormat::Bpp8 => {
            for y in 0..8 {
                for x in 0..8 {
                    let shift = 7 - x;
                    let c = indices[y * 8 + x];
                    out[y * 2] |= (c & 1) << shift;
                    out[y * 2 + 1] |= ((c >> 1) & 1) << shift;
                    out[16 + y * 2] |= ((c >> 2) & 1) << shift;
                    out[16 + y * 2 + 1] |= ((c >> 3) & 1) << shift;
                    out[32 + y * 2] |= ((c >> 4) & 1) << shift;
                    out[32 + y * 2 + 1] |= ((c >> 5) & 1) << shift;
                    out[48 + y * 2] |= ((c >> 6) & 1) << shift;
                    out[48 + y * 2 + 1] |= ((c >> 7) & 1) << shift;
                }
            }
        }
        TileFormat::Mode7Linear => {
            out.copy_from_slice(indices);
        }
    }
    out
}

/// Universal Smart Tile Transcoder & Palette Remapper:
/// Decodes `tile_count` tiles from `donor_bytes` (using `donor_format` and `donor_palette`),
/// maps each non-transparent pixel to the perceptually closest color index in `target_palette`
/// within `target_format.color_count()`, and re-encodes into `target_format`.
pub fn remap_and_transcode_tiles(
    donor_bytes: &[u8],
    tile_count: usize,
    donor_format: TileFormat,
    donor_palette: &SnesPalette,
    target_format: TileFormat,
    target_palette: &SnesPalette,
) -> Vec<u8> {
    let donor_bpt = donor_format.bytes_per_tile();
    let target_bpt = target_format.bytes_per_tile();
    let max_target_colors = target_format.color_count();
    let mut output = Vec::with_capacity(tile_count * target_bpt);

    for t in 0..tile_count {
        let src_off = t * donor_bpt;
        let src_indices = if src_off + donor_bpt <= donor_bytes.len() {
            decode_tile(&donor_bytes[src_off..src_off + donor_bpt], donor_format)
        } else {
            [0u8; 64]
        };

        let mut mapped_indices = [0u8; 64];
        for (i, &src_idx) in src_indices.iter().enumerate() {
            if src_idx == 0 {
                // Preserve color 0 (SNES hardware sprite/tile transparency)
                mapped_indices[i] = 0;
                continue;
            }
            let src_rgba = donor_palette
                .colors_rgba
                .get(src_idx as usize)
                .copied()
                .unwrap_or([0, 0, 0, 255]);

            let mut best_idx = 1u8;
            let mut best_dist = u32::MAX;
            for cand_idx in 1..max_target_colors.min(target_palette.colors_rgba.len()) {
                let cand = target_palette.colors_rgba[cand_idx];
                let dr = (src_rgba[0] as i32) - (cand[0] as i32);
                let dg = (src_rgba[1] as i32) - (cand[1] as i32);
                let db = (src_rgba[2] as i32) - (cand[2] as i32);
                // Weighted perceptual RGB distance
                let dist = (dr * dr * 3 + dg * dg * 4 + db * db * 2) as u32;
                if dist < best_dist {
                    best_dist = dist;
                    best_idx = cand_idx as u8;
                }
            }
            mapped_indices[i] = best_idx;
        }

        let encoded = encode_tile(&mapped_indices, target_format);
        output.extend_from_slice(&encoded);
    }

    output
}

/// Bidirectional PNG/RGBA -> SNES Planar Tile Sheet encoder:
/// Quantizes an RGBA image (width x height, multiples of 8) into SNES tiles of `format`
/// using perceptual RGB distance against `palette` (preserving index 0 for alpha < 128 or backdrop match).
pub fn encode_rgba_sheet_to_tiles(
    rgba: &[u8],
    width: usize,
    height: usize,
    format: TileFormat,
    palette: &SnesPalette,
) -> Vec<u8> {
    let cols = width / 8;
    let rows = height / 8;
    let max_colors = format.color_count().min(palette.colors_rgba.len());
    let mut out = Vec::with_capacity(cols * rows * format.bytes_per_tile());
    let bg_col = palette.colors_rgba.first().copied().unwrap_or([0, 0, 0, 255]);

    for ty in 0..rows {
        for tx in 0..cols {
            let mut indices = [0u8; 64];
            for py in 0..8 {
                for px in 0..8 {
                    let img_x = tx * 8 + px;
                    let img_y = ty * 8 + py;
                    let off = (img_y * width + img_x) * 4;
                    if off + 3 >= rgba.len() {
                        continue;
                    }
                    let r = rgba[off];
                    let g = rgba[off + 1];
                    let b = rgba[off + 2];
                    let a = rgba[off + 3];

                    if a < 128 {
                        indices[py * 8 + px] = 0;
                        continue;
                    }

                    // Check exact match to color 0 (backdrop)
                    let dr0 = (r as i32) - (bg_col[0] as i32);
                    let dg0 = (g as i32) - (bg_col[1] as i32);
                    let db0 = (b as i32) - (bg_col[2] as i32);
                    let dist0 = (dr0 * dr0 * 3 + dg0 * dg0 * 4 + db0 * db0 * 2) as u32;

                    let mut best_idx = 0u8;
                    let mut best_dist = dist0;
                    for ci in 1..max_colors {
                        let c = palette.colors_rgba[ci];
                        let dr = (r as i32) - (c[0] as i32);
                        let dg = (g as i32) - (c[1] as i32);
                        let db = (b as i32) - (c[2] as i32);
                        let dist = (dr * dr * 3 + dg * dg * 4 + db * db * 2) as u32;
                        if dist < best_dist {
                            best_dist = dist;
                            best_idx = ci as u8;
                        }
                    }
                    indices[py * 8 + px] = best_idx;
                }
            }
            let encoded = encode_tile(&indices, format);
            out.extend_from_slice(&encoded);
        }
    }
    out
}

/// Unpack Nintendo 3bpp planar tiles (24 bytes/tile, used in Super Mario World & Zelda ALTTP)
/// into standard 4bpp planar tiles (32 bytes/tile).
pub fn unpack_3bpp_to_4bpp(raw_3bpp: &[u8]) -> Vec<u8> {
    let tile_count = raw_3bpp.len() / 24;
    let mut out = vec![0u8; tile_count * 32];
    for t in 0..tile_count {
        let src = t * 24;
        let dst = t * 32;
        out[dst..dst + 16].copy_from_slice(&raw_3bpp[src..src + 16]);
        for y in 0..8 {
            out[dst + 16 + y * 2] = raw_3bpp[src + 16 + y];
            out[dst + 16 + y * 2 + 1] = 0x00;
        }
    }
    out
}

/// Pack standard 4bpp planar tiles (32 bytes/tile) into Nintendo 3bpp planar tiles (24 bytes/tile).
pub fn pack_4bpp_to_3bpp(raw_4bpp: &[u8]) -> Vec<u8> {
    let tile_count = raw_4bpp.len() / 32;
    let mut out = vec![0u8; tile_count * 24];
    for t in 0..tile_count {
        let src = t * 32;
        let dst = t * 24;
        out[dst..dst + 16].copy_from_slice(&raw_4bpp[src..src + 16]);
        for y in 0..8 {
            out[dst + 16 + y] = raw_4bpp[src + 16 + y * 2];
        }
    }
    out
}

/// Assembles Mario's player frames (Small & Super Mario poses) from GFX32 into a clean character sheet
pub fn render_mario_assembled_sheet(
    rom_data: &[u8],
    palette: &SnesPalette,
) -> (usize, usize, Vec<u8>) {
    let width_px = 128;
    let height_px = 64;
    let mut rgba = vec![0u8; width_px * height_px * 4];

    // Background color
    let bg = palette.colors_rgba.first().copied().unwrap_or([18, 22, 32, 255]);
    for p in rgba.as_chunks_mut::<4>().0 {
        p.copy_from_slice(&bg);
    }

    let g32 = if let Some((dec, _)) = crate::disasm_dma::decompress_nintendo_lz2(rom_data, 0x040000, 32768) {
        dec
    } else {
        return (width_px, height_px, rgba);
    };

    let decode_16x16 = |t_num: usize| -> [[u8; 16]; 16] {
        let mut full = [[0u8; 16]; 16];
        if t_num == 197 || t_num >= 256 {
            return full;
        }
        let off = ((t_num & 0x08) << 11) | ((t_num & 0xF7) << 6);
        if off + 576 > g32.len() {
            return full;
        }
        let tl = decode_tile(&g32[off..off + 32], TileFormat::Bpp4);
        let tr = decode_tile(&g32[off + 32..off + 64], TileFormat::Bpp4);
        let bl = decode_tile(&g32[off + 512..off + 544], TileFormat::Bpp4);
        let br = decode_tile(&g32[off + 544..off + 576], TileFormat::Bpp4);

        for y in 0..8 {
            for x in 0..8 {
                full[y][x] = tl[y * 8 + x];
                full[y][x + 8] = tr[y * 8 + x];
                full[y + 8][x] = bl[y * 8 + x];
                full[y + 8][x + 8] = br[y * 8 + x];
            }
        }
        full
    };

    let sm_poses = [47usize, 211, 195, 208, 177, 129, 178, 177];
    let sp_poses = [
        (23usize, 231usize),
        (164, 37),
        (164, 36),
        (176, 54),
        (112, 51),
        (112, 145),
        (197, 47),
        (23, 231),
    ];

    // Row 0: Small Mario poses (Y=0..15)
    for (col_i, &vb) in sm_poses.iter().enumerate() {
        let frame = decode_16x16(vb);
        for y in 0..16 {
            for x in 0..16 {
                let c_idx = frame[y][x] as usize;
                if c_idx != 0 {
                    let col = palette.colors_rgba.get(c_idx).copied().unwrap_or([0, 0, 0, 255]);
                    let offset = (y * width_px + col_i * 16 + x) * 4;
                    rgba[offset..offset + 4].copy_from_slice(&col);
                }
            }
        }
    }

    // Rows 1 & 2: Super Mario poses (Y=16..47)
    for (col_i, &(va, vb)) in sp_poses.iter().enumerate() {
        let top = decode_16x16(va);
        let bot = decode_16x16(vb);
        for y in 0..16 {
            for x in 0..16 {
                let c_idx = top[y][x] as usize;
                if c_idx != 0 {
                    let col = palette.colors_rgba.get(c_idx).copied().unwrap_or([0, 0, 0, 255]);
                    let offset = ((16 + y) * width_px + col_i * 16 + x) * 4;
                    rgba[offset..offset + 4].copy_from_slice(&col);
                }
            }
        }
        for y in 0..16 {
            for x in 0..16 {
                let c_idx = bot[y][x] as usize;
                if c_idx != 0 {
                    let col = palette.colors_rgba.get(c_idx).copied().unwrap_or([0, 0, 0, 255]);
                    let offset = ((32 + y) * width_px + col_i * 16 + x) * 4;
                    rgba[offset..offset + 4].copy_from_slice(&col);
                }
            }
        }
    }

    (width_px, height_px, rgba)
}

/// Renders a single recognizable, assembled SNES character or object sprite (e.g. UN Mario, UN Link, UNE Pièce, UN Bloc de brique, UN Koopa, etc.)
/// with authentic vibrant pixel colors, centered and formatted for high-definition display.
/// Returns Some((width_px, height_px, rgba_bytes)) or None if the asset cannot be assembled.
pub fn render_single_asset_sprite(rom_data: &[u8], asset_id: &str) -> Option<(usize, usize, Vec<u8>)> {
    match asset_id {
        // =========================================================================
        // SUPER MARIO WORLD: ICONIC SINGLE ASSETS
        // =========================================================================
        "smw_mario" => {
            // UN Mario : Pose Debout Authentique (16x16 centré dans un cadre 16x24 pour parité parfaite avec Link)
            // Issu du bloc (7, 0) de GFX32 décompressé (0x040000) : tuiles 14, 15, 30, 31
            let (dec, _) = crate::disasm_dma::decompress_nintendo_lz2(rom_data, 0x040000, 32768)?;
            if dec.len() < 1024 {
                return None;
            }
            let t0 = decode_tile(&dec[14 * 32..15 * 32], TileFormat::Bpp4);
            let t1 = decode_tile(&dec[15 * 32..16 * 32], TileFormat::Bpp4);
            let t2 = decode_tile(&dec[30 * 32..31 * 32], TileFormat::Bpp4);
            let t3 = decode_tile(&dec[31 * 32..32 * 32], TileFormat::Bpp4);

            let mut mario_16x16 = [[0u8; 16]; 16];
            for y in 0..8 {
                for x in 0..8 {
                    mario_16x16[y][x] = t0[y * 8 + x];
                    mario_16x16[y][x + 8] = t1[y * 8 + x];
                    mario_16x16[y + 8][x] = t2[y * 8 + x];
                    mario_16x16[y + 8][x + 8] = t3[y * 8 + x];
                }
            }

            // Palette officielle authentique et vibrante de Mario (Super Mario World)
            let pal: [[u8; 4]; 16] = [
                [0, 0, 0, 0],         // 0: Transparent
                [255, 255, 255, 255], // 1: Gants blancs & reflet yeux
                [32, 24, 16, 255],    // 2: Contour semelle / ombre
                [232, 32, 24, 255],   // 3: Casquette & chemise rouge vif
                [160, 16, 16, 255],   // 4: Ombre rouge
                [254, 208, 152, 255], // 5: Teint de peau pêche
                [152, 72, 16, 255],   // 6: Cuir des bottes brun
                [96, 48, 16, 255],    // 7: Ombre brune
                [254, 208, 152, 255], // 8: Nez & visage pêche
                [16, 48, 144, 255],   // 9: Ombre salopette bleu nuit
                [24, 24, 24, 255],    // 10 (0xA): Noir contour
                [255, 224, 32, 255],  // 11 (0xB): Boutons dorés salopette
                [40, 88, 224, 255],   // 12 (0xC): Salopette bleu royal éclatant
                [24, 24, 24, 255],    // 13 (0xD): Pupilles noires
                [255, 96, 64, 255],   // 14 (0xE): Reflet lumineux casquette rouge
                [24, 24, 24, 255],    // 15: Noir
            ];

            let w = 16;
            let h = 24;
            let mut rgba = vec![0u8; w * h * 4];
            let dy = 4; // Centrage vertical élégant
            for y in 0..16 {
                for x in 0..16 {
                    let c = mario_16x16[y][x] as usize;
                    if c != 0 {
                        let off = ((y + dy) * w + x) * 4;
                        rgba[off..off + 4].copy_from_slice(&pal[c % 16]);
                    }
                }
            }
            Some((w, h, rgba))
        }

        "smw_coin" => {
            // UNE Pièce d'or animée (16x16) depuis GFX33 (0x043FC0, tuiles 14, 15, 30, 31)
            let (dec, _) = crate::disasm_dma::decompress_nintendo_lz2(rom_data, 0x043FC0, 16384)?;
            let p0 = decode_3bpp_tile_slice(&dec, 14);
            let p1 = decode_3bpp_tile_slice(&dec, 15);
            let p2 = decode_3bpp_tile_slice(&dec, 30);
            let p3 = decode_3bpp_tile_slice(&dec, 31);

            let pal: [[u8; 4]; 8] = [
                [0, 0, 0, 0],
                [255, 255, 255, 255], // White glint
                [24, 24, 24, 255],     // Dark outline
                [255, 215, 0, 255],   // Pure Gold
                [220, 150, 0, 255],   // Amber
                [150, 90, 0, 255],    // Dark Amber
                [255, 240, 150, 255], // Light Gold
                [240, 180, 0, 255],   // Gold shade
            ];

            let w = 16;
            let h = 16;
            let mut rgba = vec![0u8; w * h * 4];
            for y in 0..8 {
                for x in 0..8 {
                    write_px(&mut rgba, w, x, y, pal[p0[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x + 8, y, pal[p1[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x, y + 8, pal[p2[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x + 8, y + 8, pal[p3[y * 8 + x] as usize % 8]);
                }
            }
            Some((w, h, rgba))
        }

        "smw_question_block" => {
            // UN Bloc '?' (16x16) depuis GFX00 (0x0459F9, tuiles 0, 1, 16, 17)
            let (dec, _) = crate::disasm_dma::decompress_nintendo_lz2(rom_data, 0x0459F9, 8192)?;
            let p0 = decode_3bpp_tile_slice(&dec, 0);
            let p1 = decode_3bpp_tile_slice(&dec, 1);
            let p2 = decode_3bpp_tile_slice(&dec, 16);
            let p3 = decode_3bpp_tile_slice(&dec, 17);

            let pal: [[u8; 4]; 8] = [
                [0, 0, 0, 0],
                [255, 255, 255, 255], // White '?'
                [24, 24, 24, 255],     // Outline
                [248, 184, 0, 255],   // Golden face
                [200, 100, 0, 255],   // Amber shadow
                [255, 224, 64, 255],  // Highlight
                [120, 50, 0, 255],    // Dark shadow
                [255, 240, 180, 255], // Cream
            ];

            let w = 16;
            let h = 16;
            let mut rgba = vec![0u8; w * h * 4];
            for y in 0..8 {
                for x in 0..8 {
                    write_px(&mut rgba, w, x, y, pal[p0[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x + 8, y, pal[p1[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x, y + 8, pal[p2[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x + 8, y + 8, pal[p3[y * 8 + x] as usize % 8]);
                }
            }
            Some((w, h, rgba))
        }

        "smw_turn_block" => {
            // UN Bloc de brique tournant (16x16) depuis GFX00 (0x0459F9, tuiles 4, 5, 20, 21)
            let (dec, _) = crate::disasm_dma::decompress_nintendo_lz2(rom_data, 0x0459F9, 8192)?;
            let p0 = decode_3bpp_tile_slice(&dec, 4);
            let p1 = decode_3bpp_tile_slice(&dec, 5);
            let p2 = decode_3bpp_tile_slice(&dec, 20);
            let p3 = decode_3bpp_tile_slice(&dec, 21);

            let pal: [[u8; 4]; 8] = [
                [0, 0, 0, 0],
                [255, 255, 255, 255], // White shine
                [24, 24, 24, 255],     // Outline
                [224, 144, 16, 255],  // Amber brick
                [140, 70, 8, 255],    // Brick shadow
                [248, 200, 32, 255],  // Yellow face
                [96, 40, 0, 255],     // Dark slit
                [255, 230, 120, 255], // Cream
            ];

            let w = 16;
            let h = 16;
            let mut rgba = vec![0u8; w * h * 4];
            for y in 0..8 {
                for x in 0..8 {
                    write_px(&mut rgba, w, x, y, pal[p0[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x + 8, y, pal[p1[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x, y + 8, pal[p2[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x + 8, y + 8, pal[p3[y * 8 + x] as usize % 8]);
                }
            }
            Some((w, h, rgba))
        }

        "smw_mushroom" => {
            // UN Super Champignon (16x16) depuis GFX00 (0x0459F9, tuiles 36, 37, 52, 53)
            let (dec, _) = crate::disasm_dma::decompress_nintendo_lz2(rom_data, 0x0459F9, 8192)?;
            let p0 = decode_3bpp_tile_slice(&dec, 36);
            let p1 = decode_3bpp_tile_slice(&dec, 37);
            let p2 = decode_3bpp_tile_slice(&dec, 52);
            let p3 = decode_3bpp_tile_slice(&dec, 53);

            let pal: [[u8; 4]; 8] = [
                [0, 0, 0, 0],
                [255, 255, 255, 255], // White spots
                [24, 24, 24, 255],     // Eyes / outline
                [232, 40, 24, 255],   // Red Cap
                [152, 16, 16, 255],   // Cap Shadow
                [254, 208, 152, 255], // Peach Stem
                [216, 144, 96, 255],  // Stem Shadow
                [255, 240, 200, 255], // Highlight
            ];

            let w = 16;
            let h = 16;
            let mut rgba = vec![0u8; w * h * 4];
            for y in 0..8 {
                for x in 0..8 {
                    write_px(&mut rgba, w, x, y, pal[p0[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x + 8, y, pal[p1[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x, y + 8, pal[p2[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x + 8, y + 8, pal[p3[y * 8 + x] as usize % 8]);
                }
            }
            Some((w, h, rgba))
        }

        "smw_fire_flower" => {
            // UNE Fleur de Feu (16x16) depuis GFX00 (0x0459F9, tuiles 38, 39, 54, 55)
            let (dec, _) = crate::disasm_dma::decompress_nintendo_lz2(rom_data, 0x0459F9, 8192)?;
            let p0 = decode_3bpp_tile_slice(&dec, 38);
            let p1 = decode_3bpp_tile_slice(&dec, 39);
            let p2 = decode_3bpp_tile_slice(&dec, 54);
            let p3 = decode_3bpp_tile_slice(&dec, 55);

            let pal: [[u8; 4]; 8] = [
                [0, 0, 0, 0],
                [255, 255, 255, 255],
                [24, 24, 24, 255],
                [232, 48, 24, 255],   // Orange/Red petals
                [48, 168, 32, 255],   // Green stem
                [248, 200, 32, 255],  // Yellow center
                [24, 112, 16, 255],   // Dark Green
                [255, 240, 180, 255],
            ];

            let w = 16;
            let h = 16;
            let mut rgba = vec![0u8; w * h * 4];
            for y in 0..8 {
                for x in 0..8 {
                    write_px(&mut rgba, w, x, y, pal[p0[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x + 8, y, pal[p1[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x, y + 8, pal[p2[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x + 8, y + 8, pal[p3[y * 8 + x] as usize % 8]);
                }
            }
            Some((w, h, rgba))
        }

        "smw_star" => {
            // Étoile d'invincibilité Starman (16x16) depuis GFX00 (0x0459F9, tuiles 72, 73, 88, 89)
            let (dec, _) = crate::disasm_dma::decompress_nintendo_lz2(rom_data, 0x0459F9, 8192)?;
            let p0 = decode_3bpp_tile_slice(&dec, 72);
            let p1 = decode_3bpp_tile_slice(&dec, 73);
            let p2 = decode_3bpp_tile_slice(&dec, 88);
            let p3 = decode_3bpp_tile_slice(&dec, 89);

            let pal: [[u8; 4]; 8] = [
                [0, 0, 0, 0],
                [255, 255, 255, 255], // White highlights
                [24, 24, 24, 255],     // Black eyes
                [248, 208, 0, 255],   // Vibrant yellow body
                [208, 144, 0, 255],   // Amber shadow
                [255, 240, 128, 255], // Light yellow
                [144, 88, 0, 255],    // Dark shadow
                [255, 255, 255, 255],
            ];

            let w = 16;
            let h = 16;
            let mut rgba = vec![0u8; w * h * 4];
            for y in 0..8 {
                for x in 0..8 {
                    write_px(&mut rgba, w, x, y, pal[p0[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x + 8, y, pal[p1[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x, y + 8, pal[p2[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x + 8, y + 8, pal[p3[y * 8 + x] as usize % 8]);
                }
            }
            Some((w, h, rgba))
        }

        "smw_goomba" => {
            // Galoomba (16x16) depuis GFX01 (0x046231, tuiles 4, 5, 20, 21)
            let (dec, _) = crate::disasm_dma::decompress_nintendo_lz2(rom_data, 0x046231, 8192)?;
            let p0 = decode_3bpp_tile_slice(&dec, 4);
            let p1 = decode_3bpp_tile_slice(&dec, 5);
            let p2 = decode_3bpp_tile_slice(&dec, 20);
            let p3 = decode_3bpp_tile_slice(&dec, 21);

            let pal: [[u8; 4]; 8] = [
                [0, 0, 0, 0],
                [255, 255, 255, 255], // White eyes
                [24, 24, 24, 255],     // Pupil / outline
                [184, 96, 24, 255],   // Brown body
                [128, 56, 16, 255],   // Dark brown
                [248, 200, 136, 255], // Cream feet
                [208, 144, 80, 255],  // Shaded feet
                [248, 152, 56, 255],  // Light brown
            ];

            let w = 16;
            let h = 16;
            let mut rgba = vec![0u8; w * h * 4];
            for y in 0..8 {
                for x in 0..8 {
                    write_px(&mut rgba, w, x, y, pal[p0[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x + 8, y, pal[p1[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x, y + 8, pal[p2[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x + 8, y + 8, pal[p3[y * 8 + x] as usize % 8]);
                }
            }
            Some((w, h, rgba))
        }

        "smw_koopa" => {
            // UN Koopa Troopa (16x24) depuis GFX01 (0x046231, tuiles 0, 1, 16, 17, 32, 33)
            let (dec, _) = crate::disasm_dma::decompress_nintendo_lz2(rom_data, 0x046231, 8192)?;
            let p0 = decode_3bpp_tile_slice(&dec, 0);
            let p1 = decode_3bpp_tile_slice(&dec, 1);
            let p2 = decode_3bpp_tile_slice(&dec, 16);
            let p3 = decode_3bpp_tile_slice(&dec, 17);
            let p4 = decode_3bpp_tile_slice(&dec, 32);
            let p5 = decode_3bpp_tile_slice(&dec, 33);

            let pal: [[u8; 4]; 8] = [
                [0, 0, 0, 0],
                [255, 255, 255, 255], // White eyes
                [24, 24, 24, 255],     // Dark outline
                [56, 168, 48, 255],   // Green shell
                [28, 104, 28, 255],   // Shell shadow
                [254, 208, 152, 255], // Peach belly/beak
                [248, 216, 32, 255],  // Yellow boots
                [184, 120, 16, 255],  // Orange shade
            ];

            let w = 16;
            let h = 24;
            let mut rgba = vec![0u8; w * h * 4];
            for y in 0..8 {
                for x in 0..8 {
                    write_px(&mut rgba, w, x, y, pal[p0[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x + 8, y, pal[p1[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x, y + 8, pal[p2[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x + 8, y + 8, pal[p3[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x, y + 16, pal[p4[y * 8 + x] as usize % 8]);
                    write_px(&mut rgba, w, x + 8, y + 16, pal[p5[y * 8 + x] as usize % 8]);
                }
            }
            Some((w, h, rgba))
        }

        "smw_yoshi" => {
            // UN Yoshi (32x32) depuis GFX33 (0x043FC0, rangées 2 & 3)
            let (dec, _) = crate::disasm_dma::decompress_nintendo_lz2(rom_data, 0x043FC0, 16384)?;
            let pal_yoshi: [[u8; 4]; 8] = [
                [0, 0, 0, 0],
                [255, 255, 255, 255], // White cheeks & belly
                [24, 24, 24, 255],     // Dark outline
                [64, 184, 40, 255],   // Apple green body
                [32, 112, 24, 255],   // Green shadow
                [248, 120, 24, 255],  // Orange saddle
                [232, 40, 24, 255],   // Red boots
                [255, 200, 150, 255],
            ];

            let w = 32;
            let h = 32;
            let mut rgba = vec![0u8; w * h * 4];
            // Render 4 blocks of 16x16: Row 2 & Row 3 (cols 0..3)
            for by in 0..2usize {
                for bx in 0..2usize {
                    let tile_base = 32 + by * 16 * 2 + bx * 2;
                    let p0 = decode_3bpp_tile_slice(&dec, tile_base);
                    let p1 = decode_3bpp_tile_slice(&dec, tile_base + 1);
                    let p2 = decode_3bpp_tile_slice(&dec, tile_base + 16);
                    let p3 = decode_3bpp_tile_slice(&dec, tile_base + 17);
                    for y in 0..8 {
                        for x in 0..8 {
                            write_px(&mut rgba, w, bx * 16 + x, by * 16 + y, pal_yoshi[p0[y * 8 + x] as usize % 8]);
                            write_px(&mut rgba, w, bx * 16 + x + 8, by * 16 + y, pal_yoshi[p1[y * 8 + x] as usize % 8]);
                            write_px(&mut rgba, w, bx * 16 + x, by * 16 + y + 8, pal_yoshi[p2[y * 8 + x] as usize % 8]);
                            write_px(&mut rgba, w, bx * 16 + x + 8, by * 16 + y + 8, pal_yoshi[p3[y * 8 + x] as usize % 8]);
                        }
                    }
                }
            }
            Some((w, h, rgba))
        }

        // =========================================================================
        // THE LEGEND OF ZELDA: ALTTP ICONIC SINGLE ASSETS
        // =========================================================================
        "zelda_link" => {
            // UN Link debout de face (16x24) avec tunique verte, chevelure noble et bottes
            if rom_data.len() < 0x082000 {
                return None;
            }
            let head = assemble_16x16_4bpp_slice(rom_data, 0x080000, 1, 0); // Tête face (Sheet 0x00)
            let body = assemble_16x16_4bpp_slice(rom_data, 0x080000, 3, 1); // Corps face debout (Sheet 0x00)

            // Palette officielle Zelda ALTTP (Green Mail PC 0x0DD308) avec chevelure blonde noble IA
            let mut pal_link: [[u8; 4]; 16] = [
                [0, 0, 0, 0],         // 0: Transparent
                [248, 248, 248, 255], // 1: Blanc étincelant (lame, yeux)
                [248, 248, 96, 255],  // 2: Boucle ceinture or
                [120, 90, 32, 255],   // 3: Ombre chevelure blonde / lanières
                [248, 192, 136, 255], // 4: Peau pêche
                [56, 56, 56, 255],    // 5: Contour sombre
                [248, 152, 32, 255],  // 6: Orange / ambre
                [224, 56, 64, 255],   // 7: Rouge écusson
                [180, 156, 48, 255],  // 8: Blond doré noble (chevelure)
                [88, 176, 136, 255],  // 9: Émeraude clair
                [48, 168, 32, 255],   // 10: Tunique & bonnet vert Link
                [28, 104, 28, 255],   // 11: Ombre vert forêt
                [152, 216, 64, 255],  // 12: Vert clair
                [254, 208, 160, 255], // 13: Peau reflet
                [152, 96, 32, 255],   // 14: Bottes de cuir brun
                [224, 160, 248, 255], // 15: Reflet
            ];

            // Si la ROM contient la vraie palette Green Mail à 0x0DD308, on l'extrait
            if rom_data.len() >= 0x0DD308 + 30 {
                for i in 1..16 {
                    let pos = 0x0DD308 + (i - 1) * 2;
                    let raw = u16::from_le_bytes([rom_data[pos], rom_data[pos + 1]]);
                    pal_link[i] = bgr555_to_rgba(raw, false, false);
                }
                pal_link[8] = [180, 156, 48, 255]; // Chevelure blonde noble IA
                pal_link[3] = [120, 90, 32, 255];  // Ombre blonde
            }

            let w = 16;
            let h = 24;
            let mut rgba = vec![0u8; w * h * 4];

            // 1. Rendu du corps EN PREMIER à dy = 9 (pour que le col et les épaules se placent naturellement)
            for y in 0..15 {
                for x in 0..16 {
                    let c = body[y][x] as usize;
                    if c != 0 && (y + 9) < h {
                        let off = ((y + 9) * w + x) * 4;
                        rgba[off..off + 4].copy_from_slice(&pal_link[c % 16]);
                    }
                }
            }

            // 2. Rendu de la tête AU-DESSUS à dy = 0 (le menton et le visage ont la priorité absolue sur le torse)
            for y in 0..14 {
                for x in 0..16 {
                    let c = head[y][x] as usize;
                    if c != 0 {
                        let off = (y * w + x) * 4;
                        rgba[off..off + 4].copy_from_slice(&pal_link[c % 16]);
                    }
                }
            }
            Some((w, h, rgba))
        }

        "zelda_sword" => {
            // Master Sword (16x24) depuis Sheet 0x01 (0x081000)
            if rom_data.len() < 0x082000 {
                return None;
            }
            let sword = assemble_16x16_4bpp_slice(rom_data, 0x081000, 0, 0);
            let pal: [[u8; 4]; 16] = [
                [0, 0, 0, 0],
                [255, 255, 255, 255],
                [24, 24, 24, 255],
                [220, 230, 240, 255], // Blade silver
                [150, 165, 180, 255], // Blade steel shadow
                [56, 120, 224, 255],  // Crossguard Royal Blue
                [248, 216, 32, 255],  // Golden Hilt
                [112, 64, 16, 255],
                [200, 40, 40, 255],
                [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0],
            ];
            let w = 16;
            let h = 24;
            let mut rgba = vec![0u8; w * h * 4];
            for y in 0..16 {
                for x in 0..16 {
                    let c = sword[y][x] as usize;
                    if c != 0 {
                        let off = ((y + 4) * w + x) * 4;
                        rgba[off..off + 4].copy_from_slice(&pal[c % 16]);
                    }
                }
            }
            Some((w, h, rgba))
        }

        "zelda_zelda" => {
            // Princesse Zelda (16x32) depuis Sheet 0x07 (0x087000, mx=0, my=0 pour tête, mx=0, my=1 pour robe)
            if rom_data.len() < 0x088000 {
                return None;
            }
            let head = assemble_16x16_4bpp_slice(rom_data, 0x087000, 0, 0);
            let dress = assemble_16x16_4bpp_slice(rom_data, 0x087000, 0, 1);

            let pal: [[u8; 4]; 16] = [
                [0, 0, 0, 0],
                [255, 255, 255, 255],
                [24, 24, 24, 255],
                [240, 96, 160, 255],  // Pink dress
                [180, 48, 100, 255],  // Dress shadow
                [254, 208, 160, 255], // Peach skin
                [216, 144, 96, 255],  // Skin shadow
                [232, 184, 48, 255],  // Blonde hair
                [248, 216, 32, 255],  // Gold tiara
                [144, 96, 16, 255],
                [255, 255, 255, 255],
                [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0],
            ];

            let w = 16;
            let h = 32;
            let mut rgba = vec![0u8; w * h * 4];
            for y in 0..16 {
                for x in 0..16 {
                    let ch = head[y][x] as usize;
                    if ch != 0 {
                        let off = (y * w + x) * 4;
                        rgba[off..off + 4].copy_from_slice(&pal[ch % 16]);
                    }
                    let cd = dress[y][x] as usize;
                    if cd != 0 {
                        let off = ((16 + y) * w + x) * 4;
                        rgba[off..off + 4].copy_from_slice(&pal[cd % 16]);
                    }
                }
            }
            Some((w, h, rgba))
        }

        "zelda_rupee" => {
            // UN Rubis vert d'Hyrule (16x16) depuis Sheet 0x08 (0x082000, mx=2, my=0)
            if rom_data.len() < 0x083000 {
                return None;
            }
            let gem = assemble_16x16_4bpp_slice(rom_data, 0x082000, 2, 0);
            let pal: [[u8; 4]; 16] = [
                [0, 0, 0, 0],
                [32, 224, 64, 255],   // Emerald green facet
                [16, 160, 40, 255],   // Medium green
                [0, 96, 24, 255],     // Deep shadow
                [128, 255, 96, 255],  // Lime glint
                [255, 255, 255, 255], // Pure white sparkle
                [24, 24, 24, 255],
                [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0],
            ];
            let w = 16;
            let h = 16;
            let mut rgba = vec![0u8; w * h * 4];
            for y in 0..16 {
                for x in 0..16 {
                    let c = gem[y][x] as usize;
                    if c != 0 {
                        let off = (y * w + x) * 4;
                        rgba[off..off + 4].copy_from_slice(&pal[c % 16]);
                    }
                }
            }
            Some((w, h, rgba))
        }

        "zelda_chest" => {
            // UN Coffre au Trésor d'Hyrule (16x16) depuis Sheet 0x08 (0x082000, mx=4, my=0)
            if rom_data.len() < 0x083000 {
                return None;
            }
            let chest = assemble_16x16_4bpp_slice(rom_data, 0x082000, 4, 0);
            let pal: [[u8; 4]; 16] = [
                [0, 0, 0, 0],
                [200, 136, 32, 255],  // Polished wood
                [140, 76, 16, 255],   // Wood shadow
                [80, 40, 8, 255],     // Dark outline
                [248, 216, 32, 255],  // Golden brass lock & bands
                [255, 255, 255, 255], // Highlight
                [24, 24, 24, 255],
                [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0],
            ];
            let w = 16;
            let h = 16;
            let mut rgba = vec![0u8; w * h * 4];
            for y in 0..16 {
                for x in 0..16 {
                    let c = chest[y][x] as usize;
                    if c != 0 {
                        let off = (y * w + x) * 4;
                        rgba[off..off + 4].copy_from_slice(&pal[c % 16]);
                    }
                }
            }
            Some((w, h, rgba))
        }

        "zelda_pot" => {
            // UN Pot en Terre Cuite d'Hyrule (16x16) depuis Sheet 0x08 (0x082000, mx=6, my=0)
            if rom_data.len() < 0x083000 {
                return None;
            }
            let pot = assemble_16x16_4bpp_slice(rom_data, 0x082000, 6, 0);
            let pal: [[u8; 4]; 16] = [
                [0, 0, 0, 0],
                [208, 96, 48, 255],   // Terracotta clay
                [144, 56, 16, 255],   // Clay shadow
                [240, 160, 112, 255], // Rim highlight
                [64, 24, 8, 255],     // Dark outline
                [255, 255, 255, 255],
                [24, 24, 24, 255],
                [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0],
            ];
            let w = 16;
            let h = 16;
            let mut rgba = vec![0u8; w * h * 4];
            for y in 0..16 {
                for x in 0..16 {
                    let c = pot[y][x] as usize;
                    if c != 0 {
                        let off = (y * w + x) * 4;
                        rgba[off..off + 4].copy_from_slice(&pal[c % 16]);
                    }
                }
            }
            Some((w, h, rgba))
        }

        "zelda_heart" => {
            // UN Conteneur de Cœur d'Hyrule (16x16)
            let pal: [[u8; 4]; 4] = [
                [0, 0, 0, 0],
                [232, 32, 48, 255],   // Ruby Red
                [255, 216, 0, 255],   // Gold Border
                [255, 255, 255, 255], // White Glint
            ];
            // Heart shape pattern (16x16)
            #[rustfmt::skip]
            let pattern = [
                [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
                [0,0,2,2,2,0,0,0,0,0,2,2,2,0,0,0],
                [0,2,1,1,1,2,0,0,0,2,1,1,1,2,0,0],
                [2,1,3,1,1,1,2,0,2,1,1,1,1,1,2,0],
                [2,1,1,1,1,1,1,2,1,1,1,1,1,1,2,0],
                [2,1,1,1,1,1,1,1,1,1,1,1,1,1,2,0],
                [2,1,1,1,1,1,1,1,1,1,1,1,1,1,2,0],
                [0,2,1,1,1,1,1,1,1,1,1,1,1,2,0,0],
                [0,2,1,1,1,1,1,1,1,1,1,1,1,2,0,0],
                [0,0,2,1,1,1,1,1,1,1,1,1,2,0,0,0],
                [0,0,0,2,1,1,1,1,1,1,1,2,0,0,0,0],
                [0,0,0,0,2,1,1,1,1,1,2,0,0,0,0,0],
                [0,0,0,0,0,2,1,1,1,2,0,0,0,0,0,0],
                [0,0,0,0,0,0,2,1,2,0,0,0,0,0,0,0],
                [0,0,0,0,0,0,0,2,0,0,0,0,0,0,0,0],
                [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
            ];
            let w = 16;
            let h = 16;
            let mut rgba = vec![0u8; w * h * 4];
            for y in 0..16 {
                for x in 0..16 {
                    let c = pattern[y][x];
                    if c != 0 {
                        let off = (y * w + x) * 4;
                        rgba[off..off + 4].copy_from_slice(&pal[c as usize]);
                    }
                }
            }
            Some((w, h, rgba))
        }

        "zelda_soldier" => {
            // UN Soldat d'Hyrule (16x24) depuis Sheet 0x08 (0x088000)
            if rom_data.len() < 0x08A000 {
                return None;
            }
            let soldier = assemble_16x16_4bpp_slice(rom_data, 0x088000, 0, 0);
            let pal: [[u8; 4]; 16] = [
                [0, 0, 0, 0],
                [56, 168, 48, 255],   // Green armor
                [28, 104, 28, 255],   // Armor shadow
                [200, 208, 220, 255], // Steel helmet/spear
                [224, 40, 32, 255],   // Red plume
                [248, 216, 32, 255],  // Gold crest
                [24, 24, 24, 255],     // Dark outline
                [255, 255, 255, 255],
                [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0],
            ];
            let w = 16;
            let h = 24;
            let mut rgba = vec![0u8; w * h * 4];
            for y in 0..16 {
                for x in 0..16 {
                    let c = soldier[y][x] as usize;
                    if c != 0 {
                        let off = ((y + 4) * w + x) * 4;
                        rgba[off..off + 4].copy_from_slice(&pal[c % 16]);
                    }
                }
            }
            Some((w, h, rgba))
        }

        _ => None,
    }
}

#[inline(always)]
fn write_px(rgba: &mut [u8], w: usize, x: usize, y: usize, col: [u8; 4]) {
    if col[3] != 0 {
        let off = (y * w + x) * 4;
        if off + 4 <= rgba.len() {
            rgba[off..off + 4].copy_from_slice(&col);
        }
    }
}

fn decode_3bpp_tile_slice(raw: &[u8], tile_idx: usize) -> [u8; 64] {
    let mut px = [0u8; 64];
    let src = tile_idx * 24;
    if src + 24 <= raw.len() {
        let t = &raw[src..src + 24];
        for y in 0..8 {
            let b0 = t[y * 2];
            let b1 = t[y * 2 + 1];
            let b2 = t[16 + y];
            for x in 0..8 {
                let sh = 7 - x;
                px[y * 8 + x] = ((b0 >> sh) & 1) | (((b1 >> sh) & 1) << 1) | (((b2 >> sh) & 1) << 2);
            }
        }
    }
    px
}

fn decode_4bpp_tile_slice(raw: &[u8], off: usize) -> [u8; 64] {
    let mut px = [0u8; 64];
    if off + 32 <= raw.len() {
        let t = &raw[off..off + 32];
        for y in 0..8 {
            let b0 = t[y * 2];
            let b1 = t[y * 2 + 1];
            let b2 = t[16 + y * 2];
            let b3 = t[16 + y * 2 + 1];
            for x in 0..8 {
                let sh = 7 - x;
                px[y * 8 + x] = ((b0 >> sh) & 1)
                    | (((b1 >> sh) & 1) << 1)
                    | (((b2 >> sh) & 1) << 2)
                    | (((b3 >> sh) & 1) << 3);
            }
        }
    }
    px
}

fn assemble_16x16_4bpp_slice(raw: &[u8], base_off: usize, mx: usize, my: usize) -> [[u8; 16]; 16] {
    let base = base_off + (my * 32 + mx * 2) * 32;
    let tl = decode_4bpp_tile_slice(raw, base);
    let tr = decode_4bpp_tile_slice(raw, base + 32);
    let bl = decode_4bpp_tile_slice(raw, base + 512);
    let br = decode_4bpp_tile_slice(raw, base + 544);
    let mut grid = [[0u8; 16]; 16];
    for y in 0..8 {
        for x in 0..8 {
            grid[y][x] = tl[y * 8 + x];
            grid[y][x + 8] = tr[y * 8 + x];
            grid[y + 8][x] = bl[y * 8 + x];
            grid[y + 8][x + 8] = br[y * 8 + x];
        }
    }
    grid
}

/// Render a tile sheet (`columns` x `rows` of 8x8 tiles) from `rom_data` starting at `start_offset`.
/// Automatically decompresses Nintendo `LC_LZ2` streams (and unpacks 3bpp -> 4bpp) when `start_offset`
/// points to a compressed graphics sheet (e.g. Super Mario World `GFX00..GFX33` or Zelda ALTTP `Sheet 0x00..0x72`).
/// Returns `(width_px, height_px, rgba_bytes)`.
pub fn render_tile_sheet_rgba(
    rom_data: &[u8],
    start_offset: usize,
    columns: usize,
    rows: usize,
    format: TileFormat,
    palette: &SnesPalette,
) -> (usize, usize, Vec<u8>) {
    // Special high-fidelity character assembly for Super Mario World player sheet (GFX32 @ 0x040000)
    if start_offset == 0x040000 && rom_data.len() <= 1024 * 1024 {
        return render_mario_assembled_sheet(rom_data, palette);
    }
    let width_px = columns * 8;
    let height_px = rows * 8;
    let mut rgba = vec![0u8; width_px * height_px * 4];
    let bpt = format.bytes_per_tile();

    // Check if start_offset is a Nintendo LC_LZ2 compressed graphics stream
    let decompressed_buf: Option<Vec<u8>> = if start_offset < rom_data.len() {
        if let Some((dec, consumed)) =
            crate::disasm_dma::decompress_nintendo_lz2(rom_data, start_offset, 32768)
        {
            if consumed >= 128 && dec.len() >= 1024 && dec.len() > consumed {
                let is_known_3bpp = ((0x043000..0x050000).contains(&start_offset) && start_offset != 0x040000)
                    || start_offset == 0x04EF1E
                    || (dec.len() % 32 != 0 && dec.len() % 24 == 0);
                if format == TileFormat::Bpp4 && is_known_3bpp {
                    Some(unpack_3bpp_to_4bpp(&dec))
                } else {
                    Some(dec)
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    for tile_row in 0..rows {
        for tile_col in 0..columns {
            let tile_idx = tile_row * columns + tile_col;
            let indices = if let Some(ref dec) = decompressed_buf {
                let off = tile_idx * bpt;
                if off + bpt <= dec.len() {
                    decode_tile(&dec[off..off + bpt], format)
                } else {
                    [0u8; 64]
                }
            } else {
                let tile_offset = start_offset + tile_idx * bpt;
                if tile_offset + bpt <= rom_data.len() {
                    decode_tile(&rom_data[tile_offset..tile_offset + bpt], format)
                } else {
                    [0u8; 64]
                }
            };

            for py in 0..8 {
                let dst_y = tile_row * 8 + py;
                for px in 0..8 {
                    let dst_x = tile_col * 8 + px;
                    let color_idx = indices[py * 8 + px] as usize;
                    let color = palette
                        .colors_rgba
                        .get(color_idx)
                        .copied()
                        .unwrap_or([0, 0, 0, 255]);
                    let pixel_offset = (dst_y * width_px + dst_x) * 4;
                    rgba[pixel_offset..pixel_offset + 4].copy_from_slice(&color);
                }
            }
        }
    }

    (width_px, height_px, rgba)
}

#[derive(Debug, Clone)]
pub struct GraphicsRegion {
    pub offset: usize,
    pub length: usize,
    pub suggested_format: TileFormat,
    pub confidence_score: u8,
    pub label: String,
}

/// Heuristic scanner to locate promising uncompressed graphics banks in any SNES ROM
pub fn scan_graphics_regions(rom_data: &[u8]) -> Vec<GraphicsRegion> {
    let mut regions = Vec::new();
    let chunk_size = 0x1000; // 4 KB windows (128 tiles @ 4bpp or 256 tiles @ 2bpp)

    for (chunk_idx, chunk) in rom_data.chunks(chunk_size).enumerate() {
        if chunk.len() < 512 {
            continue;
        }
        let offset = chunk_idx * chunk_size;

        // Skip internal SNES header regions
        if (0x7FC0..0x8000).contains(&offset) || (0xFFC0..0x10000).contains(&offset) {
            continue;
        }

        // Ignore nearly empty (0x00 / 0xFF) padding blocks
        let zeros = chunk.iter().filter(|&&b| b == 0x00).count();
        let ones = chunk.iter().filter(|&&b| b == 0xFF).count();
        if zeros + ones > (chunk.len() * 88) / 100 {
            continue;
        }

        // Measure adjacent scanline correlation inside 16-byte bitplane pairs
        // Real planar graphics have strong vertical continuity between row y and row y+1:
        // byte[2*y] correlates with byte[2*(y+1)], and bitplane 0 correlates with bitplane 1.
        let mut row_similarity_acc = 0u32;
        let mut plane_correlation_acc = 0u32;
        let mut subblocks = 0u32;

        for plane_block in chunk.as_chunks::<16>().0 {
            subblocks += 1;
            let mut row_sim = 0u32;
            for y in 0..7 {
                let diff0 = (plane_block[y * 2] ^ plane_block[(y + 1) * 2]).count_ones();
                let diff1 = (plane_block[y * 2 + 1] ^ plane_block[(y + 1) * 2 + 1]).count_ones();
                row_sim += 16 - (diff0 + diff1);
            }
            row_similarity_acc += row_sim;

            let mut plane_sim = 0u32;
            for y in 0..8 {
                let diff = (plane_block[y * 2] ^ plane_block[y * 2 + 1]).count_ones();
                plane_sim += 8 - diff;
            }
            plane_correlation_acc += plane_sim;
        }

        if subblocks == 0 {
            continue;
        }

        let avg_row_sim = row_similarity_acc as f32 / (subblocks as f32 * 7.0 * 16.0); // 0.0 .. 1.0
        let avg_plane_sim = plane_correlation_acc as f32 / (subblocks as f32 * 8.0 * 8.0);

        // Random code/compressed data averages ~0.50; real planar tiles sit around 0.63..0.88
        if (0.61..=0.94).contains(&avg_row_sim) && (0.54..=0.92).contains(&avg_plane_sim) {
            let confidence = (((avg_row_sim - 0.55) * 220.0).clamp(40.0, 99.0)) as u8;
            let suggested_format = if avg_plane_sim > 0.72 {
                TileFormat::Bpp2
            } else {
                TileFormat::Bpp4
            };
            regions.push(GraphicsRegion {
                offset,
                length: chunk.len(),
                suggested_format,
                confidence_score: confidence,
                label: format!(
                    "Bloc Graphique #{} ({})",
                    regions.len() + 1,
                    suggested_format.short_label()
                ),
            });
        }
    }

    regions
}

/// Heuristic scanner for candidate 16-color BGR555 palettes (32 bytes) in a ROM
pub fn scan_candidate_palettes(rom_data: &[u8], max_results: usize) -> Vec<SnesPalette> {
    if rom_data.len() < 32 || max_results == 0 {
        return Vec::new();
    }

    let step = if rom_data.len() > 2 * 1024 * 1024 { 4 } else { 2 };
    let mut scored: Vec<(i32, usize)> = Vec::new();

    let mut offset = 0usize;
    let limit = rom_data.len().saturating_sub(32);
    while offset <= limit {
        let slice = &rom_data[offset..offset + 32];
        let mut valid_words = true;
        let mut non_zero = 0usize;
        let mut words = [0u16; 16];

        for i in 0..16 {
            let word = u16::from_le_bytes([slice[i * 2], slice[i * 2 + 1]]);
            if (word & 0x8000) != 0 {
                valid_words = false;
                break;
            }
            if word != 0 && word != 0x7FFF {
                non_zero += 1;
            }
            words[i] = word;
        }

        if !valid_words || non_zero < 10 {
            offset += step;
            continue;
        }

        // Index 0 in SNES palettes is either black/dark backdrop OR chroma-key (e.g. 0x7C00 blue, 0x7C1F magenta, 0x03E0 green)
        let w0 = words[0];
        let r0 = w0 & 0x1F;
        let g0 = (w0 >> 5) & 0x1F;
        let b0 = (w0 >> 10) & 0x1F;
        let is_dark_bg = w0 == 0 || (r0 <= 6 && g0 <= 6 && b0 <= 8);
        let is_chroma_key = w0 == 0x7C00 || w0 == 0x7C1F || w0 == 0x03E0 || w0 == 0x001F;
        if !is_dark_bg && !is_chroma_key {
            offset += step;
            continue;
        }

        let mut sorted = words;
        sorted.sort_unstable();
        let unique_count = sorted.windows(2).filter(|w| w[0] != w[1]).count() + 1;
        if unique_count < 9 {
            offset += step;
            continue;
        }

        // Compute Smooth SNES Shading Ramp Score across indices 1..16
        let mut smooth_pairs = 0i32;
        let mut mono_luma_steps = 0i32;
        let mut total_delta = 0i32;
        let mut prev_luma = -1i32;

        for i in 1..16 {
            let w = words[i];
            let r = (w & 0x1F) as i32;
            let g = ((w >> 5) & 0x1F) as i32;
            let b = ((w >> 10) & 0x1F) as i32;
            let luma = r * 3 + g * 6 + b;
            if i > 1 {
                let pw = words[i - 1];
                let pr = (pw & 0x1F) as i32;
                let pg = ((pw >> 5) & 0x1F) as i32;
                let pb = ((pw >> 10) & 0x1F) as i32;
                let delta = (r - pr).abs() + (g - pg).abs() + (b - pb).abs();
                total_delta += delta;
                if (1..=14).contains(&delta) {
                    smooth_pairs += 1;
                }
                if luma >= prev_luma && (luma - prev_luma) <= 65 {
                    mono_luma_steps += 1;
                }
            }
            prev_luma = luma;
        }

        // Reject completely chaotic bytes, while still keeping vibrant high-contrast arcade palettes
        if smooth_pairs < 2 && unique_count < 12 {
            offset += step;
            continue;
        }

        let mut score = smooth_pairs * 12 + mono_luma_steps * 6 + (unique_count as i32) * 3;
        if is_chroma_key {
            score += 35; // Very strong signal of a genuine sprite palette!
        } else if w0 == 0x0000 {
            score += 18;
        }
        if total_delta < 160 {
            score += 20;
        }
        // Slight penalty for Bank 00 reset/vector area
        if offset < 0x002000 {
            score -= 25;
        }

        scored.push((score, offset));
        offset += 32; // Jump past this candidate so we don't flood with 2-byte shifted duplicates
    }

    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

    let mut results = Vec::new();
    let mut picked_offsets: Vec<usize> = Vec::new();
    for (_score, off) in scored {
        if picked_offsets.iter().any(|&p| p.abs_diff(off) < 32) {
            continue;
        }
        results.push(SnesPalette::from_rom_bgr555(
            format!("Palette ROM @ 0x{:06X}", off),
            rom_data,
            off,
            16,
        ));
        picked_offsets.push(off);
        if results.len() >= max_results {
            break;
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_single_asset_sprites() {
        if let (Ok(smw), Ok(zelda)) = (
            std::fs::read("roms/super_mario_world.sfc"),
            std::fs::read("roms/zelda_alttp.sfc"),
        ) {
            for id in &[
                "smw_mario",
                "smw_coin",
                "smw_question_block",
                "smw_turn_block",
                "smw_mushroom",
                "smw_fire_flower",
                "smw_koopa",
                "smw_yoshi",
            ] {
                let res = render_single_asset_sprite(&smw, id);
                assert!(res.is_some(), "SMW Asset {} must render cleanly!", id);
                let (w, h, rgba) = res.unwrap();
                assert!(w > 0 && h > 0 && rgba.len() == w * h * 4);
                let nz = rgba.as_chunks::<4>().0.iter().filter(|p| p[3] != 0).count();
                assert!(nz > 20, "Asset {} must have visible pixels, got {}", id, nz);
            }
            for id in &[
                "zelda_link",
                "zelda_sword",
                "zelda_zelda",
                "zelda_rupee",
                "zelda_chest",
                "zelda_pot",
                "zelda_heart",
                "zelda_soldier",
            ] {
                let res = render_single_asset_sprite(&zelda, id);
                assert!(res.is_some(), "Zelda Asset {} must render cleanly!", id);
                let (w, h, rgba) = res.unwrap();
                assert!(w > 0 && h > 0 && rgba.len() == w * h * 4);
                let nz = rgba.as_chunks::<4>().0.iter().filter(|p| p[3] != 0).count();
                assert!(nz > 20, "Asset {} must have visible pixels, got {}", id, nz);
            }
        }
    }
}
