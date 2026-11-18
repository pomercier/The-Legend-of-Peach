use std::fs;
use std::path::Path;
use image::{ImageBuffer, Rgba};
use crate::assets::scan_assets_for_rom;
use crate::audio::{decode_brr_to_pcm16, encode_pcm16_to_wav, scan_brr_samples};
use crate::graphics::{
    render_single_asset_sprite, render_tile_sheet_rgba, scan_candidate_palettes, scan_graphics_regions, SnesPalette,
};
use crate::rom::SnesRom;

/// Summary of all assets exported from a SNES ROM
#[derive(Debug, Clone)]
pub struct RomExportSummary {
    pub out_dir: String,
    pub sprites_png_count: usize,
    pub sheets_png_count: usize,
    pub audio_wav_count: usize,
    pub palettes_count: usize,
}

/// Exports all recognized sprites, graphic sheets, audio samples and palettes from a ROM into an organized folder
pub fn export_all_rom_assets(rom: &SnesRom, out_dir: &Path) -> std::io::Result<RomExportSummary> {
    let sprites_dir = out_dir.join("sprites");
    let sheets_dir = out_dir.join("sheets");
    let audio_dir = out_dir.join("audio");
    let pal_dir = out_dir.join("palettes");

    fs::create_dir_all(&sprites_dir)?;
    fs::create_dir_all(&sheets_dir)?;
    fs::create_dir_all(&audio_dir)?;
    fs::create_dir_all(&pal_dir)?;

    // 1. Scan Palettes
    let mut palettes = scan_candidate_palettes(&rom.data, 24);
    if palettes.is_empty() {
        palettes.push(SnesPalette::preset_grayscale());
    }
    let default_pal = palettes.first().cloned().unwrap_or_else(SnesPalette::preset_grayscale);

    let mut palettes_count = 0;
    for (idx, pal) in palettes.iter().enumerate() {
        // A. JSON color list
        let mut json = String::from("{\n  \"name\": \"");
        json.push_str(&pal.name);
        json.push_str("\",\n  \"colors\": [\n");
        for (c_idx, c) in pal.colors_rgba.iter().enumerate() {
            let hex = format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]);
            json.push_str(&format!("    \"{}\"{}", hex, if c_idx + 1 < pal.colors_rgba.len() { "," } else { "" }));
            json.push('\n');
        }
        json.push_str("  ]\n}\n");
        let safe_name = pal.name.replace(|c: char| !c.is_alphanumeric(), "_");
        fs::write(pal_dir.join(format!("palette_{:02}_{}.json", idx + 1, safe_name)), json)?;

        // B. PNG Palette Swatch (16 colors x 16x16 block = 256x16 image)
        let swatch_w = (pal.colors_rgba.len() * 16) as u32;
        let swatch_h = 16u32;
        let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(swatch_w, swatch_h);
        for (c_idx, c) in pal.colors_rgba.iter().enumerate() {
            let col = Rgba([c[0], c[1], c[2], c[3]]);
            let x_start = (c_idx * 16) as u32;
            for y in 0..16 {
                for x in 0..16 {
                    img.put_pixel(x_start + x, y, col);
                }
            }
        }
        let _ = img.save(pal_dir.join(format!("palette_{:02}_{}.png", idx + 1, safe_name)));

        // C. Binary BGR555 if offset known
        if let Some(off) = pal.source_offset {
            if off + 32 <= rom.data.len() {
                fs::write(pal_dir.join(format!("palette_{:02}_{}.pal", idx + 1, safe_name)), &rom.data[off..off + 32])?;
            }
        }
        palettes_count += 1;
    }

    // 2. Export Graphic Sheets as PNG
    let gfx_regions = scan_graphics_regions(&rom.data);
    let mut sheets_png_count = 0;
    for (idx, reg) in gfx_regions.iter().enumerate() {
        let cols = 16;
        let total_tiles = reg.length / reg.suggested_format.bytes_per_tile();
        let rows = (total_tiles / cols).max(1).min(32);
        let (w, h, rgba) = render_tile_sheet_rgba(
            &rom.data,
            reg.offset,
            cols,
            rows,
            reg.suggested_format,
            &default_pal,
        );
        if let Some(img) = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(w as u32, h as u32, rgba) {
            let path = sheets_dir.join(format!(
                "sheet_{:02}_0x{:06X}_{}.png",
                idx + 1,
                reg.offset,
                reg.suggested_format.short_label()
            ));
            let _ = img.save(path);
            sheets_png_count += 1;
        }
    }

    // 3. Export Recognized Scanned Assets as Individual Sprites
    let scanned_assets = scan_assets_for_rom(rom);
    let mut sprites_png_count = 0;
    for asset in &scanned_assets {
        let pal = if let Some(poff) = asset.palette_offset {
            palettes.iter().find(|p| p.source_offset == Some(poff)).unwrap_or(&default_pal)
        } else {
            &default_pal
        };

        // Render asset as single isolated authentic sprite if supported, or fallback to 2x2 tiles
        let (w, h, rgba) = if let Some((aw, ah, argba)) = render_single_asset_sprite(&rom.data, &asset.id) {
            (aw, ah, argba)
        } else {
            let tiles_x = 2;
            let tiles_y = 2;
            render_tile_sheet_rgba(
                &rom.data,
                asset.rom_offset,
                tiles_x,
                tiles_y,
                asset.format,
                pal,
            )
        };

        if let Some(img) = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(w as u32, h as u32, rgba) {
            // Upscale 3x with crisp nearest-neighbor for high visibility
            let scale = 3u32;
            let mut upscaled: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new((w as u32) * scale, (h as u32) * scale);
            for y in 0..(h as u32) {
                for x in 0..(w as u32) {
                    let pixel = img.get_pixel(x, y);
                    for sy in 0..scale {
                        for sx in 0..scale {
                            upscaled.put_pixel(x * scale + sx, y * scale + sy, *pixel);
                        }
                    }
                }
            }

            let cat_label = asset.category.label_fr().replace(|c: char| !c.is_alphanumeric(), "_");
            let safe_asset_name = asset.name.replace(|c: char| !c.is_alphanumeric(), "_");
            let file_name = format!("{}_{}_0x{:06X}.png", cat_label, safe_asset_name, asset.rom_offset);
            let _ = upscaled.save(sprites_dir.join(file_name));
            sprites_png_count += 1;
        }
    }

    // 4. Export Audio BRR Samples (.brr + .wav @ 16kHz)
    let brr_samples = scan_brr_samples(&rom.data, 24);
    let mut audio_wav_count = 0;
    for sample in &brr_samples {
        let end = (sample.offset + sample.byte_len).min(rom.data.len());
        let brr_slice = &rom.data[sample.offset..end];
        let safe_label = sample.label.replace(|c: char| !c.is_alphanumeric(), "_");

        let brr_file = audio_dir.join(format!("sample_{:02}_0x{:06X}_{}.brr", sample.index, sample.offset, safe_label));
        fs::write(brr_file, brr_slice)?;

        let pcm = decode_brr_to_pcm16(brr_slice);
        let wav_bytes = encode_pcm16_to_wav(&pcm, 16000);
        let wav_file = audio_dir.join(format!("sample_{:02}_0x{:06X}_{}.wav", sample.index, sample.offset, safe_label));
        fs::write(wav_file, wav_bytes)?;
        audio_wav_count += 1;
    }

    // 5. Generate Comprehensive Manifest JSON
    let mut manifest = String::from("{\n");
    manifest.push_str(&format!("  \"rom_title\": \"{}\",\n", rom.canonical_title()));
    manifest.push_str(&format!("  \"mapping\": \"{}\",\n", rom.header.mapping.as_str()));
    manifest.push_str(&format!("  \"checksum\": \"0x{:04X}\",\n", rom.header.checksum));
    manifest.push_str(&format!("  \"sprites_count\": {},\n", sprites_png_count));
    manifest.push_str(&format!("  \"sheets_count\": {},\n", sheets_png_count));
    manifest.push_str(&format!("  \"audio_wav_count\": {},\n", audio_wav_count));
    manifest.push_str(&format!("  \"palettes_count\": {}\n", palettes_count));
    manifest.push_str("}\n");
    fs::write(out_dir.join("manifest.json"), manifest)?;

    Ok(RomExportSummary {
        out_dir: out_dir.display().to_string(),
        sprites_png_count,
        sheets_png_count,
        audio_wav_count,
        palettes_count,
    })
}
