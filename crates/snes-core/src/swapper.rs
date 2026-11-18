use crate::audio::{adapt_brr_for_swap, resample_and_encode_brr_for_swap, BrrSampleInfo};
use crate::graphics::{remap_and_transcode_tiles, SnesPalette, TileFormat};
use crate::rom::{RomError, SnesRom};

#[derive(Debug, Clone)]
pub enum SwapKind {
    GraphicsTiles {
        format: TileFormat,
        tile_count: usize,
        smart_remapped: bool,
    },
    PaletteBgr555 {
        color_count: usize,
    },
    AudioBrrSample {
        target_blocks: usize,
        donor_blocks: usize,
        loops: bool,
        resampled: bool,
    },
    CodePatch {
        byte_count: usize,
    },
}

impl SwapKind {
    pub fn category_badge(&self) -> &'static str {
        match self {
            SwapKind::GraphicsTiles {
                smart_remapped: true,
                ..
            } => "SPRITES (SMART REMAP)",
            SwapKind::GraphicsTiles {
                smart_remapped: false,
                ..
            } => "SPRITES (BRUT)",
            SwapKind::PaletteBgr555 { .. } => "PALETTE BGR555",
            SwapKind::AudioBrrSample {
                resampled: true, ..
            } => "AUDIO BRR (RESAMPLED)",
            SwapKind::AudioBrrSample {
                resampled: false, ..
            } => "AUDIO BRR (BLOCS)",
            SwapKind::CodePatch { .. } => "PATCH CODE 65C816",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SwapOperation {
    pub id: usize,
    pub description: String,
    pub kind: SwapKind,
    pub target_offset_a: usize,
    pub donor_offset_b: usize,
    pub byte_len: usize,
    pub original_bytes_a: Vec<u8>,
    pub replacement_bytes: Vec<u8>,
}

#[derive(Debug, Default, Clone)]
pub struct SwapSession {
    next_id: usize,
    pub operations: Vec<SwapOperation>,
}

impl SwapSession {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            operations: Vec::new(),
        }
    }

    /// Raw bitplane swap of `tile_count` tiles of `format` from `rom_b` at `offset_b` into `rom_a` at `offset_a`.
    /// Automatically decompresses Nintendo `LC_LZ2` streams (and unpacks 3bpp -> 4bpp) if `offset_b` points to compressed GFX.
    pub fn swap_graphics_tiles(
        &mut self,
        rom_a: &mut SnesRom,
        rom_b: &SnesRom,
        offset_a: usize,
        offset_b: usize,
        tile_count: usize,
        format: TileFormat,
    ) -> Result<SwapOperation, RomError> {
        let byte_len = tile_count * format.bytes_per_tile();
        if offset_a + byte_len > rom_a.data.len() {
            return Err(RomError::OutOfBounds {
                offset: offset_a,
                len: byte_len,
            });
        }

        let decompressed_b = if offset_b < rom_b.data.len() {
            if let Some((dec, consumed)) =
                crate::disasm_dma::decompress_nintendo_lz2(&rom_b.data, offset_b, 32768)
            {
                if consumed >= 128 && dec.len() >= 1024 && dec.len() > consumed {
                    if format == TileFormat::Bpp4 && dec.len() % 24 == 0 {
                        Some(crate::graphics::unpack_3bpp_to_4bpp(&dec))
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

        let replacement_bytes = if let Some(dec) = decompressed_b {
            let mut out = vec![0u8; byte_len];
            for i in 0..byte_len {
                out[i] = dec[i % dec.len()];
            }
            out
        } else {
            if offset_b + byte_len > rom_b.data.len() {
                return Err(RomError::OutOfBounds {
                    offset: offset_b,
                    len: byte_len,
                });
            }
            rom_b.data[offset_b..offset_b + byte_len].to_vec()
        };

        let original_bytes_a = rom_a.data[offset_a..offset_a + byte_len].to_vec();

        rom_a.write_bytes(offset_a, &replacement_bytes)?;
        rom_a.fix_checksum();

        let id = self.next_id;
        self.next_id += 1;

        let op = SwapOperation {
            id,
            description: format!(
                "Swap {} tiles {} (ROM B 0x{:06X} -> ROM A 0x{:06X})",
                tile_count,
                format.short_label(),
                offset_b,
                offset_a
            ),
            kind: SwapKind::GraphicsTiles {
                format,
                tile_count,
                smart_remapped: false,
            },
            target_offset_a: offset_a,
            donor_offset_b: offset_b,
            byte_len,
            original_bytes_a,
            replacement_bytes,
        };
        self.operations.push(op.clone());
        Ok(op)
    }

    /// Record and apply a custom pre-built byte replacement (e.g., LC_LZ2 recompressed sheet or composite bank swap)
    pub fn swap_custom_bytes(
        &mut self,
        rom_a: &mut SnesRom,
        offset_a: usize,
        donor_offset_b: usize,
        replacement_bytes: Vec<u8>,
        description: String,
        kind: SwapKind,
    ) -> Result<SwapOperation, RomError> {
        let byte_len = replacement_bytes.len();
        if offset_a + byte_len > rom_a.data.len() {
            return Err(RomError::OutOfBounds {
                offset: offset_a,
                len: byte_len,
            });
        }
        let original_bytes_a = rom_a.data[offset_a..offset_a + byte_len].to_vec();
        rom_a.write_bytes(offset_a, &replacement_bytes)?;
        rom_a.fix_checksum();

        let id = self.next_id;
        self.next_id += 1;

        let op = SwapOperation {
            id,
            description,
            kind,
            target_offset_a: offset_a,
            donor_offset_b,
            byte_len,
            original_bytes_a,
            replacement_bytes,
        };
        self.operations.push(op.clone());
        Ok(op)
    }

    /// Universal Smart Swap: Remaps pixels from ROM B's `donor_palette` + `donor_format` to
    /// the closest colors in ROM A's `target_palette` + `target_format`, preserving ROM A's palette!
    #[allow(clippy::too_many_arguments)]
    pub fn swap_graphics_tiles_smart_remapped(
        &mut self,
        rom_a: &mut SnesRom,
        rom_b: &SnesRom,
        offset_a: usize,
        offset_b: usize,
        tile_count: usize,
        target_format_a: TileFormat,
        target_palette_a: &SnesPalette,
        donor_format_b: TileFormat,
        donor_palette_b: &SnesPalette,
    ) -> Result<SwapOperation, RomError> {
        let target_byte_len = tile_count * target_format_a.bytes_per_tile();
        let donor_byte_len = tile_count * donor_format_b.bytes_per_tile();

        if offset_a + target_byte_len > rom_a.data.len() {
            return Err(RomError::OutOfBounds {
                offset: offset_a,
                len: target_byte_len,
            });
        }
        if offset_b + donor_byte_len > rom_b.data.len() {
            return Err(RomError::OutOfBounds {
                offset: offset_b,
                len: donor_byte_len,
            });
        }

        let original_bytes_a = rom_a.data[offset_a..offset_a + target_byte_len].to_vec();
        let donor_slice = &rom_b.data[offset_b..offset_b + donor_byte_len];
        let replacement_bytes = remap_and_transcode_tiles(
            donor_slice,
            tile_count,
            donor_format_b,
            donor_palette_b,
            target_format_a,
            target_palette_a,
        );

        rom_a.write_bytes(offset_a, &replacement_bytes)?;
        rom_a.fix_checksum();

        let id = self.next_id;
        self.next_id += 1;

        let op = SwapOperation {
            id,
            description: format!(
                "Smart Swap (Couleurs & Format adaptés) {} tiles ({} -> {}, 0x{:06X} -> 0x{:06X})",
                tile_count,
                donor_format_b.short_label(),
                target_format_a.short_label(),
                offset_b,
                offset_a
            ),
            kind: SwapKind::GraphicsTiles {
                format: target_format_a,
                tile_count,
                smart_remapped: true,
            },
            target_offset_a: offset_a,
            donor_offset_b: offset_b,
            byte_len: target_byte_len,
            original_bytes_a,
            replacement_bytes,
        };
        self.operations.push(op.clone());
        Ok(op)
    }

    /// Swap a BGR555 palette from `rom_b` at `offset_b` into `rom_a` at `offset_a`
    pub fn swap_palette(
        &mut self,
        rom_a: &mut SnesRom,
        rom_b: &SnesRom,
        offset_a: usize,
        offset_b: usize,
        color_count: usize,
    ) -> Result<SwapOperation, RomError> {
        let byte_len = color_count * 2;
        if offset_a + byte_len > rom_a.data.len() {
            return Err(RomError::OutOfBounds {
                offset: offset_a,
                len: byte_len,
            });
        }
        if offset_b + byte_len > rom_b.data.len() {
            return Err(RomError::OutOfBounds {
                offset: offset_b,
                len: byte_len,
            });
        }

        let original_bytes_a = rom_a.data[offset_a..offset_a + byte_len].to_vec();
        let replacement_bytes = rom_b.data[offset_b..offset_b + byte_len].to_vec();

        rom_a.write_bytes(offset_a, &replacement_bytes)?;
        rom_a.fix_checksum();

        let id = self.next_id;
        self.next_id += 1;

        let op = SwapOperation {
            id,
            description: format!(
                "Swap Palette {} couleurs (ROM B 0x{:06X} -> ROM A 0x{:06X})",
                color_count, offset_b, offset_a
            ),
            kind: SwapKind::PaletteBgr555 { color_count },
            target_offset_a: offset_a,
            donor_offset_b: offset_b,
            byte_len,
            original_bytes_a,
            replacement_bytes,
        };
        self.operations.push(op.clone());
        Ok(op)
    }

    /// Swap an SPC700 BRR audio sample from `rom_b` into `rom_a`'s sample slot.
    /// If `resample_to_fit` is true, time-stretches and re-encodes the entire waveform into BRR!
    pub fn swap_brr_sample(
        &mut self,
        rom_a: &mut SnesRom,
        rom_b: &SnesRom,
        sample_a: &BrrSampleInfo,
        sample_b: &BrrSampleInfo,
        resample_to_fit: bool,
    ) -> Result<SwapOperation, RomError> {
        if sample_a.offset + sample_a.byte_len > rom_a.data.len() {
            return Err(RomError::OutOfBounds {
                offset: sample_a.offset,
                len: sample_a.byte_len,
            });
        }
        if sample_b.offset + sample_b.byte_len > rom_b.data.len() {
            return Err(RomError::OutOfBounds {
                offset: sample_b.offset,
                len: sample_b.byte_len,
            });
        }

        let donor_slice = &rom_b.data[sample_b.offset..sample_b.offset + sample_b.byte_len];
        let replacement_bytes = if resample_to_fit {
            resample_and_encode_brr_for_swap(donor_slice, sample_a.byte_len, sample_a.loops)
        } else {
            adapt_brr_for_swap(donor_slice, sample_a.byte_len, sample_a.loops)
        };
        let original_bytes_a =
            rom_a.data[sample_a.offset..sample_a.offset + sample_a.byte_len].to_vec();

        rom_a.write_bytes(sample_a.offset, &replacement_bytes)?;
        rom_a.fix_checksum();

        let id = self.next_id;
        self.next_id += 1;

        let op = SwapOperation {
            id,
            description: format!(
                "Swap Audio {} '{}' ({} blocs) <- '{}' ({} blocs)",
                if resample_to_fit {
                    "[Resamplé]"
                } else {
                    "[Blocs]"
                },
                sample_a.label,
                sample_a.block_count,
                sample_b.label,
                sample_b.block_count
            ),
            kind: SwapKind::AudioBrrSample {
                target_blocks: sample_a.block_count,
                donor_blocks: sample_b.block_count,
                loops: sample_a.loops,
                resampled: resample_to_fit,
            },
            target_offset_a: sample_a.offset,
            donor_offset_b: sample_b.offset,
            byte_len: sample_a.byte_len,
            original_bytes_a,
            replacement_bytes,
        };
        self.operations.push(op.clone());
        Ok(op)
    }

    /// Undo a specific swap operation by ID and restore original bytes in `rom_a`
    pub fn undo_operation(&mut self, rom_a: &mut SnesRom, id: usize) -> Result<bool, RomError> {
        if let Some(pos) = self.operations.iter().position(|op| op.id == id) {
            let op = self.operations.remove(pos);
            rom_a.write_bytes(op.target_offset_a, &op.original_bytes_a)?;
            rom_a.fix_checksum();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Revert all swaps in reverse chronological order
    pub fn undo_all(&mut self, rom_a: &mut SnesRom) -> Result<(), RomError> {
        while let Some(op) = self.operations.pop() {
            rom_a.write_bytes(op.target_offset_a, &op.original_bytes_a)?;
        }
        rom_a.fix_checksum();
        Ok(())
    }
}
