use crate::audio::{decode_brr_to_pcm16, encode_pcm16_to_wav, scan_brr_samples};
use crate::graphics::{scan_candidate_palettes, scan_graphics_regions};
use crate::rom::{RomMapping, SnesRom};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaTarget {
    /// PPU VRAM ($2118 / $2119) - Tiles, Sprites, Tilemaps
    VramGraphics,
    /// PPU CGRAM ($2122) - BGR555 Palettes
    CgramPalette,
    /// PPU OAM ($2104) - Sprite Attribute Table
    OamSprites,
    /// WRAM ($2180)
    WramBuffer,
}

impl DmaTarget {
    pub fn label(self) -> &'static str {
        match self {
            DmaTarget::VramGraphics => "VRAM ($2118 - Tiles/Sprites)",
            DmaTarget::CgramPalette => "CGRAM ($2122 - Palette BGR555)",
            DmaTarget::OamSprites => "OAM ($2104 - Attributs Sprites)",
            DmaTarget::WramBuffer => "WRAM ($2180 - Buffer Mémoire)",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DmaTransferDiscovery {
    pub code_pc_offset: usize,
    pub dma_channel: u8,
    pub target: DmaTarget,
    pub source_snes_bank: u8,
    pub source_snes_addr: u16,
    pub source_pc_offset: Option<usize>,
    pub transfer_size: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkKind {
    EmptyPadding,
    CpuCode65c816,
    PlanarGraphics,
    BrrAudio,
    CompressedOrHighEntropy,
    DataTables,
}

impl ChunkKind {
    pub fn label(self) -> &'static str {
        match self {
            ChunkKind::EmptyPadding => "Vide / Padding",
            ChunkKind::CpuCode65c816 => "Code CPU 65c816",
            ChunkKind::PlanarGraphics => "Graphismes Tiles (Non-compressé)",
            ChunkKind::BrrAudio => "Audio SPC700 (BRR)",
            ChunkKind::CompressedOrHighEntropy => "Données Compressées (LZ/RLE)",
            ChunkKind::DataTables => "Tables / Pointeurs / Tilemaps",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RomChunkClassification {
    pub offset: usize,
    pub length: usize,
    pub kind: ChunkKind,
    pub entropy_bits: f32,
}

#[derive(Debug, Clone)]
pub struct DecompiledProjectSummary {
    pub out_dir: String,
    pub bank_count: usize,
    pub dma_count: usize,
    pub gfx_count: usize,
    pub palette_count: usize,
    pub audio_wav_count: usize,
}

/// Convert a SNES `Bank:Addr` pointer back into a PC ROM offset for any ROM mapping.
/// Returns `None` if the bank/address points to WRAM ($7E/$7F) or hardware registers rather than ROM.
pub fn snes_to_pc(mapping: RomMapping, bank: u8, addr: u16, rom_len: usize) -> Option<usize> {
    if bank == 0x7E || bank == 0x7F {
        return None;
    }
    let pc = match mapping {
        RomMapping::LoROM => {
            if addr < 0x8000 {
                return None;
            }
            let effective_bank = (bank & 0x7F) as usize;
            effective_bank * 0x8000 + ((addr - 0x8000) as usize)
        }
        RomMapping::HiROM => {
            if (bank & 0x7F) < 0x40 && addr < 0x8000 {
                return None;
            }
            let effective_bank = (bank & 0x3F) as usize;
            effective_bank * 0x10000 + (addr as usize)
        }
        RomMapping::ExHiROM => {
            let effective_bank = (bank & 0x7F) as usize;
            effective_bank * 0x10000 + (addr as usize)
        }
    };
    if pc < rom_len {
        Some(pc)
    } else {
        None
    }
}

/// Universal scanner that inspects 65c816 machine code across any SNES ROM to find
/// hardware DMA transfers (`$43x0-$43x5`) targeting VRAM (`$2118`), CGRAM (`$2122`), or OAM (`$2104`).
/// Supports both 8-bit writes (`STA $43x1`) and 16-bit combined writes (`STA/STX/STY $43x0` with `#$1801`, `#$2200`, etc.).
pub fn scan_hardware_dma_transfers(rom: &SnesRom) -> Vec<DmaTransferDiscovery> {
    let data = &rom.data;
    let mut discoveries = Vec::new();
    if data.len() < 32 {
        return discoveries;
    }

    for i in 0..data.len().saturating_sub(24) {
        // Match STA/STX/STY $43x0 or $43x1 (8D/8E/8C [00..71] 43)
        let op = data[i];
        if (op == 0x8D || op == 0x8E || op == 0x8C)
            && data[i + 2] == 0x43
            && data[i + 1] < 0x80
            && ((data[i + 1] & 0x0F) == 0x00 || (data[i + 1] & 0x0F) == 0x01)
        {
            let ch = (data[i + 1] >> 4) & 0x07;
            let reg_low = data[i + 1] & 0x0F;

            let window_start = i.saturating_sub(28);
            let window_end = (i + 48).min(data.len());
            let win = &data[window_start..window_end];

            let mut target_opt = None;
            let mut src_addr: Option<u16> = None;
            let mut src_bank: Option<u8> = None;
            let mut size_opt: Option<u16> = None;

            // 1) If writing to $43x0 (16-bit mode+B-bus write): look at the immediate load right before `i`
            if reg_low == 0x00
                && i >= 3
                && (data[i - 3] == 0xA9 || data[i - 3] == 0xA2 || data[i - 3] == 0xA0)
            {
                let b_reg = data[i - 1];
                let mode_byte = data[i - 2];
                if mode_byte <= 0x0F {
                    match b_reg {
                        0x18 | 0x19 => target_opt = Some(DmaTarget::VramGraphics),
                        0x22 => target_opt = Some(DmaTarget::CgramPalette),
                        0x04 => target_opt = Some(DmaTarget::OamSprites),
                        0x80 => target_opt = Some(DmaTarget::WramBuffer),
                        _ => {}
                    }
                }
            }

            // 2) If writing to $43x1 (8-bit B-bus write): look at the immediate load right before `i`
            if reg_low == 0x01
                && i >= 2
                && (data[i - 2] == 0xA9 || data[i - 2] == 0xA2 || data[i - 2] == 0xA0)
            {
                match data[i - 1] {
                    0x18 | 0x19 => target_opt = Some(DmaTarget::VramGraphics),
                    0x22 => target_opt = Some(DmaTarget::CgramPalette),
                    0x04 => target_opt = Some(DmaTarget::OamSprites),
                    0x80 => target_opt = Some(DmaTarget::WramBuffer),
                    _ => {}
                }
            }

            for k in 0..win.len().saturating_sub(4) {
                // 3) 16-bit immediate load followed by STA/STX/STY $43x2 (source addr) or $43x5 (size)
                if k + 5 < win.len()
                    && (win[k] == 0xA9 || win[k] == 0xA2 || win[k] == 0xA0)
                    && (win[k + 3] == 0x8D || win[k + 3] == 0x8E || win[k + 3] == 0x8C)
                    && win[k + 5] == 0x43
                {
                    let r_ch = (win[k + 4] >> 4) & 0x07;
                    let r_low = win[k + 4] & 0x0F;
                    let val16 = u16::from_le_bytes([win[k + 1], win[k + 2]]);
                    if r_low == 0x02 && (r_ch == ch || src_addr.is_none()) {
                        src_addr = Some(val16);
                    } else if r_low == 0x05 && (r_ch == ch || size_opt.is_none()) {
                        size_opt = Some(val16);
                    }
                }

                // 4) 8-bit immediate load followed by STA/STX/STY $43x4 (source bank)
                if k + 4 < win.len()
                    && (win[k] == 0xA9 || win[k] == 0xA2 || win[k] == 0xA0)
                    && (win[k + 2] == 0x8D || win[k + 2] == 0x8E || win[k + 2] == 0x8C)
                    && win[k + 4] == 0x43
                    && (win[k + 3] & 0x0F) == 0x04
                {
                    let r_ch = (win[k + 3] >> 4) & 0x07;
                    if r_ch == ch || src_bank.is_none() {
                        src_bank = Some(win[k + 1]);
                    }
                }

                // 5) STZ $43x4 (9C x4 43) sets bank to $00
                if k + 2 < win.len()
                    && win[k] == 0x9C
                    && win[k + 2] == 0x43
                    && (win[k + 1] & 0x0F) == 0x04
                {
                    src_bank = Some(0x00);
                }
            }

            if let Some(target) = target_opt {
                if let (Some(bank), Some(addr)) = (src_bank, src_addr) {
                    let pc_offset = snes_to_pc(rom.header.mapping, bank, addr, data.len());
                    let is_dup = discoveries.iter().any(|d: &DmaTransferDiscovery| {
                        d.source_snes_bank == bank
                            && d.source_snes_addr == addr
                            && d.target == target
                    });
                    if !is_dup {
                        discoveries.push(DmaTransferDiscovery {
                            code_pc_offset: i,
                            dma_channel: ch,
                            target,
                            source_snes_bank: bank,
                            source_snes_addr: addr,
                            source_pc_offset: pc_offset,
                            transfer_size: size_opt,
                        });
                    }
                }
            }
        }
    }

    discoveries
}

/// Classify all 4KB blocks of any SNES ROM by combining Shannon entropy, 65c816 opcode density,
/// planar bitplane correlation, and BRR block structure.
pub fn classify_rom_chunks(rom_data: &[u8]) -> Vec<RomChunkClassification> {
    let chunk_size = 0x1000;
    let mut out = Vec::with_capacity(rom_data.len() / chunk_size + 1);

    for (idx, chunk) in rom_data.chunks(chunk_size).enumerate() {
        let offset = idx * chunk_size;
        let entropy = compute_shannon_entropy(chunk);

        let zeros = chunk.iter().filter(|&&b| b == 0x00).count();
        let ones = chunk.iter().filter(|&&b| b == 0xFF).count();
        if zeros + ones > (chunk.len() * 90) / 100 {
            out.push(RomChunkClassification {
                offset,
                length: chunk.len(),
                kind: ChunkKind::EmptyPadding,
                entropy_bits: entropy,
            });
            continue;
        }

        let mut valid_brr_blocks = 0usize;
        let total_brr_blocks = chunk.len() / 9;
        for b in chunk.as_chunks::<9>().0 {
            if (b[0] >> 4) <= 12 {
                valid_brr_blocks += 1;
            }
        }
        if total_brr_blocks > 10
            && valid_brr_blocks > (total_brr_blocks * 88) / 100
            && entropy > 4.5
        {
            out.push(RomChunkClassification {
                offset,
                length: chunk.len(),
                kind: ChunkKind::BrrAudio,
                entropy_bits: entropy,
            });
            continue;
        }

        let mut cpu_markers = 0usize;
        for w in chunk.windows(2) {
            match w[0] {
                0xE2 | 0xC2 if (w[1] & 0xCF) == 0 => cpu_markers += 2,
                0x20 | 0x22 | 0x60 | 0x6B | 0x4C | 0x8D | 0xAD | 0xA9 | 0x85 | 0xA5 => {
                    cpu_markers += 1
                }
                _ => {}
            }
        }
        if cpu_markers > chunk.len() / 5 {
            out.push(RomChunkClassification {
                offset,
                length: chunk.len(),
                kind: ChunkKind::CpuCode65c816,
                entropy_bits: entropy,
            });
            continue;
        }

        if entropy > 7.35 {
            out.push(RomChunkClassification {
                offset,
                length: chunk.len(),
                kind: ChunkKind::CompressedOrHighEntropy,
                entropy_bits: entropy,
            });
        } else if (3.2..=6.8).contains(&entropy) {
            out.push(RomChunkClassification {
                offset,
                length: chunk.len(),
                kind: ChunkKind::PlanarGraphics,
                entropy_bits: entropy,
            });
        } else {
            out.push(RomChunkClassification {
                offset,
                length: chunk.len(),
                kind: ChunkKind::DataTables,
                entropy_bits: entropy,
            });
        }
    }

    out
}

pub fn compute_shannon_entropy(bytes: &[u8]) -> f32 {
    if bytes.is_empty() {
        return 0.0;
    }
    let mut counts = [0u32; 256];
    for &b in bytes {
        counts[b as usize] += 1;
    }
    let len = bytes.len() as f32;
    let mut entropy = 0.0f32;
    for &c in &counts {
        if c > 0 {
            let p = (c as f32) / len;
            entropy -= p * p.log2();
        }
    }
    entropy
}

/// Universal Nintendo LC_LZ2 / LZ5 Decompressor (supports both Big-Endian SMW and Little-Endian ALTTP back-references)
pub fn decompress_nintendo_lz2(
    rom_data: &[u8],
    start_offset: usize,
    max_output_len: usize,
) -> Option<(Vec<u8>, usize)> {
    // Super Mario World uses 512KB LoROM (big-endian cmd 4); Zelda ALTTP uses 1MB LoROM (little-endian cmd 4)
    let prefer_be = rom_data.len() <= 512 * 1024;
    if let Some(res) =
        decompress_nintendo_lz2_with_endian(rom_data, start_offset, max_output_len, prefer_be)
    {
        return Some(res);
    }
    decompress_nintendo_lz2_with_endian(rom_data, start_offset, max_output_len, !prefer_be)
}

pub fn decompress_nintendo_lz2_with_endian(
    rom_data: &[u8],
    start_offset: usize,
    max_output_len: usize,
    big_endian_backref: bool,
) -> Option<(Vec<u8>, usize)> {
    let mut pos = start_offset;
    let mut out = Vec::with_capacity(2048);

    while pos < rom_data.len() && out.len() < max_output_len {
        let cmd_byte = rom_data[pos];
        pos += 1;
        if cmd_byte == 0xFF {
            let consumed = pos - start_offset;
            if out.len() >= 64 {
                return Some((out, consumed));
            } else {
                return None;
            }
        }

        let (cmd, len) = if (cmd_byte & 0xE0) == 0xE0 {
            if pos >= rom_data.len() {
                return None;
            }
            let cmd = (cmd_byte >> 2) & 0x07;
            let len = (((cmd_byte & 0x03) as usize) << 8) + (rom_data[pos] as usize) + 1;
            pos += 1;
            (cmd, len)
        } else {
            let cmd = (cmd_byte >> 5) & 0x07;
            let len = ((cmd_byte & 0x1F) as usize) + 1;
            (cmd, len)
        };

        match cmd {
            0 => {
                if pos + len > rom_data.len() {
                    return None;
                }
                out.extend_from_slice(&rom_data[pos..pos + len]);
                pos += len;
            }
            1 => {
                if pos >= rom_data.len() {
                    return None;
                }
                let fill = rom_data[pos];
                pos += 1;
                out.resize(out.len() + len, fill);
            }
            2 => {
                if pos + 1 >= rom_data.len() {
                    return None;
                }
                let b0 = rom_data[pos];
                let b1 = rom_data[pos + 1];
                pos += 2;
                for i in 0..len {
                    out.push(if i % 2 == 0 { b0 } else { b1 });
                }
            }
            3 => {
                if pos >= rom_data.len() {
                    return None;
                }
                let mut b = rom_data[pos];
                pos += 1;
                for _ in 0..len {
                    out.push(b);
                    b = b.wrapping_add(1);
                }
            }
            4 => {
                if pos + 1 >= rom_data.len() {
                    return None;
                }
                let src_off = if big_endian_backref {
                    u16::from_be_bytes([rom_data[pos], rom_data[pos + 1]]) as usize
                } else {
                    u16::from_le_bytes([rom_data[pos], rom_data[pos + 1]]) as usize
                };
                pos += 2;
                if src_off >= out.len() {
                    return None;
                }
                for i in 0..len {
                    let b = out[src_off + i];
                    out.push(b);
                }
            }
            _ => return None,
        }
    }

    None
}

/// Compress arbitrary bytes using Nintendo LC_LZ2 (with RLE + LZ77 back-references)
pub fn compress_nintendo_lz2(raw: &[u8]) -> Vec<u8> {
    compress_nintendo_lz2_with_endian(raw, true)
}

pub fn compress_nintendo_lz2_with_endian(raw: &[u8], big_endian_backref: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len() + 16);
    let mut i = 0;
    let n = raw.len();
    let mut lit_buf: Vec<u8> = Vec::with_capacity(1024);

    let flush_literals = |out: &mut Vec<u8>, lit_buf: &mut Vec<u8>| {
        if lit_buf.is_empty() {
            return;
        }
        let len_m1 = lit_buf.len() - 1;
        if len_m1 < 32 {
            out.push(len_m1 as u8);
        } else {
            out.push(0xE0 | (((len_m1 >> 8) & 0x03) as u8));
            out.push((len_m1 & 0xFF) as u8);
        }
        out.extend_from_slice(lit_buf);
        lit_buf.clear();
    };

    while i < n {
        let mut run = 1usize;
        while i + run < n && raw[i + run] == raw[i] && run < 1024 {
            run += 1;
        }

        let mut run2 = 0usize;
        if i + 1 < n {
            let b0 = raw[i];
            let b1 = raw[i + 1];
            while i + run2 < n
                && raw[i + run2] == (if run2.is_multiple_of(2) { b0 } else { b1 })
                && run2 < 1024
            {
                run2 += 1;
            }
            if run2 < 4 {
                run2 = 0;
            }
        }

        let mut best_len = 0usize;
        let mut best_off = 0usize;
        if i > 0 {
            let max_l = (n - i).min(1024);
            if max_l >= 4 {
                for cand in 0..i {
                    if raw[cand] == raw[i]
                        && cand + 3 < n
                        && raw[cand + 1..cand + 4] == raw[i + 1..i + 4]
                    {
                        let mut m = 4usize;
                        while m < max_l && raw[cand + m] == raw[i + m] {
                            m += 1;
                        }
                        if m > best_len {
                            best_len = m;
                            best_off = cand;
                            if best_len == max_l {
                                break;
                            }
                        }
                    }
                }
            }
        }

        if run >= 3 && run >= run2 && run >= best_len {
            flush_literals(&mut out, &mut lit_buf);
            let len_m1 = run - 1;
            if len_m1 < 32 {
                out.push((1 << 5) | (len_m1 as u8));
            } else {
                out.push(0xE0 | (1 << 2) | (((len_m1 >> 8) & 0x03) as u8));
                out.push((len_m1 & 0xFF) as u8);
            }
            out.push(raw[i]);
            i += run;
            continue;
        } else if run2 >= 4 && run2 >= best_len {
            flush_literals(&mut out, &mut lit_buf);
            let len_m1 = run2 - 1;
            if len_m1 < 32 {
                out.push((2 << 5) | (len_m1 as u8));
            } else {
                out.push(0xE0 | (2 << 2) | (((len_m1 >> 8) & 0x03) as u8));
                out.push((len_m1 & 0xFF) as u8);
            }
            out.push(raw[i]);
            out.push(raw[i + 1]);
            i += run2;
            continue;
        } else if best_len >= 4 {
            flush_literals(&mut out, &mut lit_buf);
            let len_m1 = best_len - 1;
            if len_m1 < 32 {
                out.push((4 << 5) | (len_m1 as u8));
            } else {
                out.push(0xE0 | (4 << 2) | (((len_m1 >> 8) & 0x03) as u8));
                out.push((len_m1 & 0xFF) as u8);
            }
            let off_bytes = if big_endian_backref {
                (best_off as u16).to_be_bytes()
            } else {
                (best_off as u16).to_le_bytes()
            };
            out.extend_from_slice(&off_bytes);
            i += best_len;
            continue;
        }

        lit_buf.push(raw[i]);
        if lit_buf.len() == 1024 {
            flush_literals(&mut out, &mut lit_buf);
        }
        i += 1;
    }

    flush_literals(&mut out, &mut lit_buf);
    out.push(0xFF);
    out
}

/// Look up official SNES hardware register names ($2100..$2183, $4016..$4017, $4200..$421F, $4300..$437F)
pub fn snes_register_comment(addr16: u16) -> Option<&'static str> {
    match addr16 {
        0x2100 => Some("INIDISP - Screen Display & Brightness"),
        0x2101 => Some("OBSEL - Object Size & Sprite Tile Base"),
        0x2102 => Some("OAMADDL - OAM Address Low"),
        0x2103 => Some("OAMADDH - OAM Address High"),
        0x2104 => Some("OAMDATA - OAM Data Write"),
        0x2105 => Some("BGMODE - BG Mode & Tile Size"),
        0x2106 => Some("MOSAIC - Mosaic Size & Enable"),
        0x2107..=0x210A => Some("BGxSC - BG Tilemap Base & Size"),
        0x210B..=0x210C => Some("BGxxNBA - BG Character Tile Base"),
        0x210D..=0x2114 => Some("BGxHOFS/VOFS - BG Scroll Registers"),
        0x2115 => Some("VMAIN - Video Port Control"),
        0x2116 => Some("VMADDL - VRAM Word Address Low"),
        0x2117 => Some("VMADDH - VRAM Word Address High"),
        0x2118 => Some("VMDATAL - VRAM Data Write Low"),
        0x2119 => Some("VMDATAH - VRAM Data Write High"),
        0x211A..=0x2120 => Some("M7 - Mode 7 Matrix / Center Registers"),
        0x2121 => Some("CGADD - CGRAM Palette Address"),
        0x2122 => Some("CGDATA - CGRAM Palette Data Write"),
        0x2123..=0x212B => Some("WxxSEL - Window Mask Settings"),
        0x212C => Some("TM - Main Screen Layer Designation"),
        0x212D => Some("TS - Sub Screen Layer Designation"),
        0x2130 => Some("CGWSEL - Color Addition Select"),
        0x2131 => Some("CGADSUB - Color Math Designation"),
        0x2132 => Some("COLDATA - Fixed Color Data"),
        0x2133 => Some("SETINI - Screen Mode / Video Select"),
        0x2140..=0x2143 => Some("APUIO0..3 - SPC700 Audio I/O Ports"),
        0x2180 => Some("WMDATA - WRAM Data Read/Write"),
        0x2181..=0x2183 => Some("WMADD - WRAM Address Registers"),
        0x4016..=0x4017 => Some("JOYSER0/1 - Joypad Serial Ports"),
        0x4200 => Some("NMITIMEN - NMI, V/H-IRQ & Auto-Joypad Enable"),
        0x4201 => Some("WRIO - Programmable I/O Port"),
        0x4202..=0x4206 => Some("WRMPY/WRDIV - Hardware Multiply/Divide"),
        0x4207..=0x420A => Some("HTIME/VTIME - IRQ H/V Timer"),
        0x420B => Some("MDMAEN - General Purpose DMA Enable"),
        0x420C => Some("HDMAEN - H-Blank DMA Enable"),
        0x420D => Some("MEMSEL - FastROM Speed Select"),
        0x4210 => Some("RDNMI - NMI Flag & 5A22 Version"),
        0x4211 => Some("TIMEUP - IRQ Flag"),
        0x4212 => Some("HVBJOY - V-Blank / H-Blank / Joypad Busy Status"),
        0x4218..=0x421F => Some("JOY1..4 - Hardware Joypad Data"),
        0x4300..=0x437F => match addr16 & 0x0F {
            0x00 => Some("DMAPx - DMA Control (Direction, Mode)"),
            0x01 => Some("BBADx - DMA Destination B-Bus Register ($21xx)"),
            0x02 => Some("A1TxL - DMA Source Address Low"),
            0x03 => Some("A1TxH - DMA Source Address High"),
            0x04 => Some("A1Bx - DMA Source Bank"),
            0x05 => Some("DASxL - DMA Transfer Size Low"),
            0x06 => Some("DASxH - DMA Transfer Size High"),
            _ => Some("DMA/HDMA Channel Register"),
        },
        _ => None,
    }
}

/// Disassemble 65c816 instructions starting at `pc_offset` while dynamically tracking
/// `SEP` (#$20/#$10) and `REP` (#$20/#$10) register width changes (`M` and `X` flags)
/// and annotating SNES hardware registers ($21xx / $42xx / $43xx).
pub fn disassemble_65c816_block(
    rom: &SnesRom,
    pc_offset: usize,
    instruction_count: usize,
) -> Vec<String> {
    let mut lines = Vec::with_capacity(instruction_count);
    let mut pos = pc_offset;
    let data = &rom.data;
    // On Reset, 65c816 starts in 8-bit M=1, X=1 emulation mode
    let mut m_8bit = true;
    let mut x_8bit = true;

    for _ in 0..instruction_count {
        if pos >= data.len() {
            break;
        }
        let (bank, addr) = rom.pc_to_snes(pos);
        let op = data[pos];
        let (mnemonic, mode_kind) = decode_opcode_65c816_full(op);
        let size = mode_kind.byte_size(m_8bit, x_8bit);
        let end = (pos + size).min(data.len());
        let bytes = &data[pos..end];

        // Update dynamic M / X processor status flags on SEP ($E2) and REP ($C2)
        if op == 0xE2 && bytes.len() >= 2 {
            let imm = bytes[1];
            if (imm & 0x20) != 0 {
                m_8bit = true;
            }
            if (imm & 0x10) != 0 {
                x_8bit = true;
            }
        } else if op == 0xC2 && bytes.len() >= 2 {
            let imm = bytes[1];
            if (imm & 0x20) != 0 {
                m_8bit = false;
            }
            if (imm & 0x10) != 0 {
                x_8bit = false;
            }
        }

        let (operand_str, reg_comment) = format_65c816_operand(bytes, addr, mode_kind);
        let hex_bytes = bytes
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        let comment_suffix = match (op, reg_comment) {
            (0xE2, _) => format!(
                " ; SEP (M={}, X={})",
                if m_8bit { "8b" } else { "16b" },
                if x_8bit { "8b" } else { "16b" }
            ),
            (0xC2, _) => format!(
                " ; REP (M={}, X={})",
                if m_8bit { "8b" } else { "16b" },
                if x_8bit { "8b" } else { "16b" }
            ),
            (_, Some(c)) => format!(" ; {}", c),
            _ => String::new(),
        };

        lines.push(format!(
            "${:02X}:{:04X} (0x{:06X}) | {:<11} | {:<4} {}{}",
            bank, addr, pos, hex_bytes, mnemonic, operand_str, comment_suffix
        ));
        pos += size;
    }

    lines
}

#[derive(Debug, Clone, Copy)]
enum AddrMode65c816 {
    Implied,
    ImmM,
    ImmX,
    Imm8,
    Dp,
    DpX,
    DpY,
    DpInd,
    DpIndX,
    DpIndY,
    DpIndLong,
    DpIndLongY,
    StackRel,
    StackRelIndY,
    Abs,
    AbsX,
    AbsY,
    AbsLong,
    AbsLongX,
    AbsInd,
    AbsIndX,
    AbsIndLong,
    Rel8,
    Rel16,
    BlockMove,
}

impl AddrMode65c816 {
    fn byte_size(self, m_8bit: bool, x_8bit: bool) -> usize {
        match self {
            AddrMode65c816::Implied => 1,
            AddrMode65c816::ImmM => {
                if m_8bit {
                    2
                } else {
                    3
                }
            }
            AddrMode65c816::ImmX => {
                if x_8bit {
                    2
                } else {
                    3
                }
            }
            AddrMode65c816::Imm8
            | AddrMode65c816::Dp
            | AddrMode65c816::DpX
            | AddrMode65c816::DpY
            | AddrMode65c816::DpInd
            | AddrMode65c816::DpIndX
            | AddrMode65c816::DpIndY
            | AddrMode65c816::DpIndLong
            | AddrMode65c816::DpIndLongY
            | AddrMode65c816::StackRel
            | AddrMode65c816::StackRelIndY
            | AddrMode65c816::Rel8 => 2,
            AddrMode65c816::Abs
            | AddrMode65c816::AbsX
            | AddrMode65c816::AbsY
            | AddrMode65c816::AbsInd
            | AddrMode65c816::AbsIndX
            | AddrMode65c816::AbsIndLong
            | AddrMode65c816::Rel16
            | AddrMode65c816::BlockMove => 3,
            AddrMode65c816::AbsLong | AddrMode65c816::AbsLongX => 4,
        }
    }
}

fn format_65c816_operand(
    bytes: &[u8],
    pc_snes_addr: u16,
    mode: AddrMode65c816,
) -> (String, Option<&'static str>) {
    match (mode, bytes.len()) {
        (AddrMode65c816::Implied, _) => (String::new(), None),
        (AddrMode65c816::Imm8 | AddrMode65c816::ImmM | AddrMode65c816::ImmX, 2) => {
            (format!("#${:02X}", bytes[1]), None)
        }
        (AddrMode65c816::ImmM | AddrMode65c816::ImmX, 3) => {
            let v = u16::from_le_bytes([bytes[1], bytes[2]]);
            (format!("#${:04X}", v), None)
        }
        (AddrMode65c816::Dp, 2) => (format!("${:02X}", bytes[1]), None),
        (AddrMode65c816::DpX, 2) => (format!("${:02X},X", bytes[1]), None),
        (AddrMode65c816::DpY, 2) => (format!("${:02X},Y", bytes[1]), None),
        (AddrMode65c816::DpInd, 2) => (format!("(${:02X})", bytes[1]), None),
        (AddrMode65c816::DpIndX, 2) => (format!("(${:02X},X)", bytes[1]), None),
        (AddrMode65c816::DpIndY, 2) => (format!("(${:02X}),Y", bytes[1]), None),
        (AddrMode65c816::DpIndLong, 2) => (format!("[${:02X}]", bytes[1]), None),
        (AddrMode65c816::DpIndLongY, 2) => (format!("[${:02X}],Y", bytes[1]), None),
        (AddrMode65c816::StackRel, 2) => (format!("${:02X},S", bytes[1]), None),
        (AddrMode65c816::StackRelIndY, 2) => (format!("(${:02X},S),Y", bytes[1]), None),
        (AddrMode65c816::Rel8, 2) => {
            let rel = bytes[1] as i8 as i32;
            let dest = ((pc_snes_addr as i32) + 2 + rel) as u16;
            (format!("${:04X}", dest), None)
        }
        (AddrMode65c816::Rel16, 3) => {
            let rel = i16::from_le_bytes([bytes[1], bytes[2]]) as i32;
            let dest = ((pc_snes_addr as i32) + 3 + rel) as u16;
            (format!("${:04X}", dest), None)
        }
        (AddrMode65c816::Abs, 3) => {
            let a = u16::from_le_bytes([bytes[1], bytes[2]]);
            (format!("${:04X}", a), snes_register_comment(a))
        }
        (AddrMode65c816::AbsX, 3) => {
            let a = u16::from_le_bytes([bytes[1], bytes[2]]);
            (format!("${:04X},X", a), snes_register_comment(a))
        }
        (AddrMode65c816::AbsY, 3) => {
            let a = u16::from_le_bytes([bytes[1], bytes[2]]);
            (format!("${:04X},Y", a), snes_register_comment(a))
        }
        (AddrMode65c816::AbsInd, 3) => {
            let a = u16::from_le_bytes([bytes[1], bytes[2]]);
            (format!("(${:04X})", a), None)
        }
        (AddrMode65c816::AbsIndX, 3) => {
            let a = u16::from_le_bytes([bytes[1], bytes[2]]);
            (format!("(${:04X},X)", a), None)
        }
        (AddrMode65c816::AbsIndLong, 3) => {
            let a = u16::from_le_bytes([bytes[1], bytes[2]]);
            (format!("[${:04X}]", a), None)
        }
        (AddrMode65c816::BlockMove, 3) => (format!("#${:02X}, #${:02X}", bytes[1], bytes[2]), None),
        (AddrMode65c816::AbsLong, 4) => {
            let a = u16::from_le_bytes([bytes[1], bytes[2]]);
            (format!("${:02X}:{:04X}", bytes[3], a), None)
        }
        (AddrMode65c816::AbsLongX, 4) => {
            let a = u16::from_le_bytes([bytes[1], bytes[2]]);
            (format!("${:02X}:{:04X},X", bytes[3], a), None)
        }
        _ => (String::new(), None),
    }
}

fn decode_opcode_65c816_full(op: u8) -> (&'static str, AddrMode65c816) {
    use AddrMode65c816::*;
    match op {
        0x00 => ("BRK", Imm8),
        0x01 => ("ORA", DpIndX),
        0x02 => ("COP", Imm8),
        0x03 => ("ORA", StackRel),
        0x04 => ("TSB", Dp),
        0x05 => ("ORA", Dp),
        0x06 => ("ASL", Dp),
        0x07 => ("ORA", DpIndLong),
        0x08 => ("PHP", Implied),
        0x09 => ("ORA", ImmM),
        0x0A => ("ASL", Implied),
        0x0B => ("PHD", Implied),
        0x0C => ("TSB", Abs),
        0x0D => ("ORA", Abs),
        0x0E => ("ASL", Abs),
        0x0F => ("ORA", AbsLong),
        0x10 => ("BPL", Rel8),
        0x11 => ("ORA", DpIndY),
        0x12 => ("ORA", DpInd),
        0x13 => ("ORA", StackRelIndY),
        0x14 => ("TRB", Dp),
        0x15 => ("ORA", DpX),
        0x16 => ("ASL", DpX),
        0x17 => ("ORA", DpIndLongY),
        0x18 => ("CLC", Implied),
        0x19 => ("ORA", AbsY),
        0x1A => ("INC", Implied),
        0x1B => ("TCS", Implied),
        0x1C => ("TRB", Abs),
        0x1D => ("ORA", AbsX),
        0x1E => ("ASL", AbsX),
        0x1F => ("ORA", AbsLongX),
        0x20 => ("JSR", Abs),
        0x21 => ("AND", DpIndX),
        0x22 => ("JSL", AbsLong),
        0x23 => ("AND", StackRel),
        0x24 => ("BIT", Dp),
        0x25 => ("AND", Dp),
        0x26 => ("ROL", Dp),
        0x27 => ("AND", DpIndLong),
        0x28 => ("PLP", Implied),
        0x29 => ("AND", ImmM),
        0x2A => ("ROL", Implied),
        0x2B => ("PLD", Implied),
        0x2C => ("BIT", Abs),
        0x2D => ("AND", Abs),
        0x2E => ("ROL", Abs),
        0x2F => ("AND", AbsLong),
        0x30 => ("BMI", Rel8),
        0x31 => ("AND", DpIndY),
        0x32 => ("AND", DpInd),
        0x33 => ("AND", StackRelIndY),
        0x34 => ("BIT", DpX),
        0x35 => ("AND", DpX),
        0x36 => ("ROL", DpX),
        0x37 => ("AND", DpIndLongY),
        0x38 => ("SEC", Implied),
        0x39 => ("AND", AbsY),
        0x3A => ("DEC", Implied),
        0x3B => ("TSC", Implied),
        0x3C => ("BIT", AbsX),
        0x3D => ("AND", AbsX),
        0x3E => ("ROL", AbsX),
        0x3F => ("AND", AbsLongX),
        0x40 => ("RTI", Implied),
        0x41 => ("EOR", DpIndX),
        0x42 => ("WDM", Imm8),
        0x43 => ("EOR", StackRel),
        0x44 => ("MVP", BlockMove),
        0x45 => ("EOR", Dp),
        0x46 => ("LSR", Dp),
        0x47 => ("EOR", DpIndLong),
        0x48 => ("PHA", Implied),
        0x49 => ("EOR", ImmM),
        0x4A => ("LSR", Implied),
        0x4B => ("PHK", Implied),
        0x4C => ("JMP", Abs),
        0x4D => ("EOR", Abs),
        0x4E => ("LSR", Abs),
        0x4F => ("EOR", AbsLong),
        0x50 => ("BVC", Rel8),
        0x51 => ("EOR", DpIndY),
        0x52 => ("EOR", DpInd),
        0x53 => ("EOR", StackRelIndY),
        0x54 => ("MVN", BlockMove),
        0x55 => ("EOR", DpX),
        0x56 => ("LSR", DpX),
        0x57 => ("EOR", DpIndLongY),
        0x58 => ("CLI", Implied),
        0x59 => ("EOR", AbsY),
        0x5A => ("PHY", Implied),
        0x5B => ("TCD", Implied),
        0x5C => ("JML", AbsLong),
        0x5D => ("EOR", AbsX),
        0x5E => ("LSR", AbsX),
        0x5F => ("EOR", AbsLongX),
        0x60 => ("RTS", Implied),
        0x61 => ("ADC", DpIndX),
        0x62 => ("PER", Rel16),
        0x63 => ("ADC", StackRel),
        0x64 => ("STZ", Dp),
        0x65 => ("ADC", Dp),
        0x66 => ("ROR", Dp),
        0x67 => ("ADC", DpIndLong),
        0x68 => ("PLA", Implied),
        0x69 => ("ADC", ImmM),
        0x6A => ("ROR", Implied),
        0x6B => ("RTL", Implied),
        0x6C => ("JMP", AbsInd),
        0x6D => ("ADC", Abs),
        0x6E => ("ROR", Abs),
        0x6F => ("ADC", AbsLong),
        0x70 => ("BVS", Rel8),
        0x71 => ("ADC", DpIndY),
        0x72 => ("ADC", DpInd),
        0x73 => ("ADC", StackRelIndY),
        0x74 => ("STZ", DpX),
        0x75 => ("ADC", DpX),
        0x76 => ("ROR", DpX),
        0x77 => ("ADC", DpIndLongY),
        0x78 => ("SEI", Implied),
        0x79 => ("ADC", AbsY),
        0x7A => ("PLY", Implied),
        0x7B => ("TDC", Implied),
        0x7C => ("JMP", AbsIndX),
        0x7D => ("ADC", AbsX),
        0x7E => ("ROR", AbsX),
        0x7F => ("ADC", AbsLongX),
        0x80 => ("BRA", Rel8),
        0x81 => ("STA", DpIndX),
        0x82 => ("BRL", Rel16),
        0x83 => ("STA", StackRel),
        0x84 => ("STY", Dp),
        0x85 => ("STA", Dp),
        0x86 => ("STX", Dp),
        0x87 => ("STA", DpIndLong),
        0x88 => ("DEY", Implied),
        0x89 => ("BIT", ImmM),
        0x8A => ("TXA", Implied),
        0x8B => ("PHB", Implied),
        0x8C => ("STY", Abs),
        0x8D => ("STA", Abs),
        0x8E => ("STX", Abs),
        0x8F => ("STA", AbsLong),
        0x90 => ("BCC", Rel8),
        0x91 => ("STA", DpIndY),
        0x92 => ("STA", DpInd),
        0x93 => ("STA", StackRelIndY),
        0x94 => ("STY", DpX),
        0x95 => ("STA", DpX),
        0x96 => ("STX", DpY),
        0x97 => ("STA", DpIndLongY),
        0x98 => ("TYA", Implied),
        0x99 => ("STA", AbsY),
        0x9A => ("TXS", Implied),
        0x9B => ("TXY", Implied),
        0x9C => ("STZ", Abs),
        0x9D => ("STA", AbsX),
        0x9E => ("STZ", AbsX),
        0x9F => ("STA", AbsLongX),
        0xA0 => ("LDY", ImmX),
        0xA1 => ("LDA", DpIndX),
        0xA2 => ("LDX", ImmX),
        0xA3 => ("LDA", StackRel),
        0xA4 => ("LDY", Dp),
        0xA5 => ("LDA", Dp),
        0xA6 => ("LDX", Dp),
        0xA7 => ("LDA", DpIndLong),
        0xA8 => ("TAY", Implied),
        0xA9 => ("LDA", ImmM),
        0xAA => ("TAX", Implied),
        0xAB => ("PLB", Implied),
        0xAC => ("LDY", Abs),
        0xAD => ("LDA", Abs),
        0xAE => ("LDX", Abs),
        0xAF => ("LDA", AbsLong),
        0xB0 => ("BCS", Rel8),
        0xB1 => ("LDA", DpIndY),
        0xB2 => ("LDA", DpInd),
        0xB3 => ("LDA", StackRelIndY),
        0xB4 => ("LDY", DpX),
        0xB5 => ("LDA", DpX),
        0xB6 => ("LDX", DpY),
        0xB7 => ("LDA", DpIndLongY),
        0xB8 => ("CLV", Implied),
        0xB9 => ("LDA", AbsY),
        0xBA => ("TSX", Implied),
        0xBB => ("TYX", Implied),
        0xBC => ("LDY", AbsX),
        0xBD => ("LDA", AbsX),
        0xBE => ("LDX", AbsY),
        0xBF => ("LDA", AbsLongX),
        0xC0 => ("CPY", ImmX),
        0xC1 => ("CMP", DpIndX),
        0xC2 => ("REP", Imm8),
        0xC3 => ("CMP", StackRel),
        0xC4 => ("CPY", Dp),
        0xC5 => ("CMP", Dp),
        0xC6 => ("DEC", Dp),
        0xC7 => ("CMP", DpIndLong),
        0xC8 => ("INY", Implied),
        0xC9 => ("CMP", ImmM),
        0xCA => ("DEX", Implied),
        0xCB => ("WAI", Implied),
        0xCC => ("CPY", Abs),
        0xCD => ("CMP", Abs),
        0xCE => ("DEC", Abs),
        0xCF => ("CMP", AbsLong),
        0xD0 => ("BNE", Rel8),
        0xD1 => ("CMP", DpIndY),
        0xD2 => ("CMP", DpInd),
        0xD3 => ("CMP", StackRelIndY),
        0xD4 => ("PEI", Dp),
        0xD5 => ("CMP", DpX),
        0xD6 => ("DEC", DpX),
        0xD7 => ("CMP", DpIndLongY),
        0xD8 => ("CLD", Implied),
        0xD9 => ("CMP", AbsY),
        0xDA => ("PHX", Implied),
        0xDB => ("STP", Implied),
        0xDC => ("JML", AbsIndLong),
        0xDD => ("CMP", AbsX),
        0xDE => ("DEC", AbsX),
        0xDF => ("CMP", AbsLongX),
        0xE0 => ("CPX", ImmX),
        0xE1 => ("SBC", DpIndX),
        0xE2 => ("SEP", Imm8),
        0xE3 => ("SBC", StackRel),
        0xE4 => ("CPX", Dp),
        0xE5 => ("SBC", Dp),
        0xE6 => ("INC", Dp),
        0xE7 => ("SBC", DpIndLong),
        0xE8 => ("INX", Implied),
        0xE9 => ("SBC", ImmM),
        0xEA => ("NOP", Implied),
        0xEB => ("XBA", Implied),
        0xEC => ("CPX", Abs),
        0xED => ("SBC", Abs),
        0xEE => ("INC", Abs),
        0xEF => ("SBC", AbsLong),
        0xF0 => ("BEQ", Rel8),
        0xF1 => ("SBC", DpIndY),
        0xF2 => ("SBC", DpInd),
        0xF3 => ("SBC", StackRelIndY),
        0xF4 => ("PEA", Abs),
        0xF5 => ("SBC", DpX),
        0xF6 => ("INC", DpX),
        0xF7 => ("SBC", DpIndLongY),
        0xF8 => ("SED", Implied),
        0xF9 => ("SBC", AbsY),
        0xFA => ("PLX", Implied),
        0xFB => ("XCE", Implied),
        0xFC => ("JSR", AbsIndX),
        0xFD => ("SBC", AbsX),
        0xFE => ("INC", AbsX),
        0xFF => ("SBC", AbsLongX),
    }
}

/// Decompile any SNES ROM into a structured, WLA-DX & Antigravity-ready project directory (`snes2asm` style in pure Rust)
pub fn decompile_rom_to_project_dir(
    rom: &SnesRom,
    out_dir: &Path,
) -> std::io::Result<DecompiledProjectSummary> {
    let banks_dir = out_dir.join("banks");
    let raw_banks_dir = banks_dir.join("raw");
    let gfx_dir = out_dir.join("assets").join("graphics");
    let pal_dir = out_dir.join("assets").join("palettes");
    let audio_dir = out_dir.join("assets").join("audio");

    std::fs::create_dir_all(&raw_banks_dir)?;
    std::fs::create_dir_all(&gfx_dir)?;
    std::fs::create_dir_all(&pal_dir)?;
    std::fs::create_dir_all(&audio_dir)?;

    let banks = rom.banks();
    let dmas = scan_hardware_dma_transfers(rom);
    let gfx_regions = scan_graphics_regions(&rom.data);
    let palettes = scan_candidate_palettes(&rom.data, 32);
    let brr_samples = scan_brr_samples(&rom.data, 16);
    let chunks = classify_rom_chunks(&rom.data);

    // 1. Write hdr.asm (WLA-DX header & vectors)
    let hdr_asm = format!(
        "; === SNES Internal Header & Interrupt Vectors ({}) ===\n\
         .SNESHEADER\n\
         ID \"SNES\"\n\
         NAME \"{:21}\"\n\
         {}\n\
         {}\n\
         CARTRIDGETYPE ${:02X}\n\
         ROMSIZE ${:02X}\n\
         SRAMSIZE ${:02X}\n\
         COUNTRY ${:02X}\n\
         LICENSEECODE ${:02X}\n\
         VERSION ${:02X}\n\
         .ENDSNES\n\n\
         .SNESNATIVEVECTOR\n\
         NMI   ${:04X}\n\
         IRQ   ${:04X}\n\
         .ENDNATIVEVECTOR\n\n\
         .SNESEMUVECTOR\n\
         RESET ${:04X}\n\
         .ENDEMUVECTOR\n",
        rom.name,
        rom.header.title,
        match rom.header.mapping {
            RomMapping::LoROM => "LOROM",
            RomMapping::HiROM => "HIROM",
            RomMapping::ExHiROM => "EXHIROM",
        },
        if rom.header.fast_rom {
            "FASTROM"
        } else {
            "SLOWROM"
        },
        rom.header.chipset_byte,
        rom.header.rom_size_byte,
        rom.header.sram_size_byte,
        rom.header.region_byte,
        rom.header.developer_id,
        rom.header.version,
        rom.header.nmi_vector,
        rom.header.irq_vector,
        rom.header.reset_vector
    );
    std::fs::write(out_dir.join("hdr.asm"), hdr_asm)?;

    // 2. Write each bank (raw .bin + annotated 65c816 .asm)
    let bank_size = rom.bank_size();
    for (bank_idx, bank_bytes) in banks.iter().enumerate() {
        let raw_path = raw_banks_dir.join(format!("bank_{:02X}.bin", bank_idx));
        std::fs::write(&raw_path, bank_bytes)?;

        let bank_pc_start = bank_idx * bank_size;
        // Determine if bank contains predominantly CPU code or data/assets
        let first_chunk_kind = chunks
            .get(bank_pc_start / 0x1000)
            .map(|c| c.kind)
            .unwrap_or(ChunkKind::DataTables);

        let mut asm_content = format!(
            "; ============================================================================\n\
             ; BANK ${:02X} (PC Offset 0x{:06X}..0x{:06X}, {} Ko)\n\
             ; Classification dominante : {}\n\
             ; ============================================================================\n\
             .BANK {} SLOT 0\n\
             .ORG 0\n\n",
            bank_idx,
            bank_pc_start,
            bank_pc_start + bank_bytes.len(),
            bank_bytes.len() / 1024,
            first_chunk_kind.label(),
            bank_idx
        );

        // Disassemble first 256 instructions of every bank (or full vector routines in bank 00)
        let instr_lines = disassemble_65c816_block(rom, bank_pc_start, 220);
        for line in instr_lines {
            asm_content.push_str(&line);
            asm_content.push('\n');
        }
        std::fs::write(
            banks_dir.join(format!("bank_{:02X}.asm", bank_idx)),
            asm_content,
        )?;
    }

    // 3. Export uncompressed graphics regions
    for (idx, reg) in gfx_regions.iter().enumerate() {
        let end = (reg.offset + reg.length).min(rom.data.len());
        let path = gfx_dir.join(format!(
            "gfx_{:02}_0x{:06X}_{}.bin",
            idx + 1,
            reg.offset,
            reg.suggested_format.short_label()
        ));
        std::fs::write(path, &rom.data[reg.offset..end])?;
    }

    // 4. Export BGR555 Palettes
    for (idx, pal) in palettes.iter().enumerate() {
        if let Some(off) = pal.source_offset {
            let end = (off + 32).min(rom.data.len());
            let path = pal_dir.join(format!("palette_{:02}_0x{:06X}.pal", idx + 1, off));
            std::fs::write(path, &rom.data[off..end])?;
        }
    }

    // 5. Export SPC700 BRR Audio Samples (.brr + .wav @ 16kHz)
    for sample in &brr_samples {
        let end = (sample.offset + sample.byte_len).min(rom.data.len());
        let brr_slice = &rom.data[sample.offset..end];
        let brr_path = audio_dir.join(format!(
            "sample_{:02}_0x{:06X}.brr",
            sample.index, sample.offset
        ));
        std::fs::write(brr_path, brr_slice)?;

        let pcm = decode_brr_to_pcm16(brr_slice);
        let wav_bytes = encode_pcm16_to_wav(&pcm, 16000);
        let wav_path = audio_dir.join(format!(
            "sample_{:02}_0x{:06X}.wav",
            sample.index, sample.offset
        ));
        std::fs::write(wav_path, wav_bytes)?;
    }

    // 6. Write README.md context file tailored for AI agents (Antigravity) & developers
    let mut readme = format!(
        "# Projet Décomposé SNES (65c816) — `{}`\n\n\
         - **Titre Interne** : `{}`\n\
         - **Architecture** : Ricoh 5A22 (65c816) — `{}` ({})\n\
         - **Chipset** : {}\n\
         - **Taille ROM** : {} Ko ({} banques de {} Ko)\n\
         - **Vecteurs CPU** : `RESET = ${:04X}`, `NMI = ${:04X}`, `IRQ = ${:04X}`\n\
         - **Checksum** : `0x{:04X}` (Complément `0x{:04X}`, Valide : `{}`)\n\
         - **CRC32** : `0x{:08X}`\n\n\
         ## Structure générée\n\
         - `hdr.asm` : En-tête officiel SNES et vecteurs d'interruption pour WLA-DX.\n\
         - `banks/bank_XX.asm` : Code 65c816 désassemblé par banque avec suivi dynamique `SEP`/`REP` (8/16-bit) et registres matériels annotés (`$2100 INIDISP`, `$420B MDMAEN`, `$43x0 DMA`).\n\
         - `banks/raw/bank_XX.bin` : Tranches binaires brutes de chaque banque mémoire.\n\
         - `assets/graphics/` : {} régions de tiles planaires extraites.\n\
         - `assets/palettes/` : {} palettes CGRAM BGR555 extraites.\n\
         - `assets/audio/` : {} échantillons SPC700 extraits en `.brr` et convertis en `.wav` (16 kHz).\n\n\
         ## Transferts DMA Matériels Détectés ({})\n",
        rom.name,
        rom.header.title,
        rom.header.mapping.as_str(),
        if rom.header.fast_rom {
            "FastROM 3.58 MHz"
        } else {
            "SlowROM 2.68 MHz"
        },
        rom.header.chipset_description,
        rom.data.len() / 1024,
        banks.len(),
        bank_size / 1024,
        rom.header.reset_vector,
        rom.header.nmi_vector,
        rom.header.irq_vector,
        rom.header.checksum,
        rom.header.checksum_complement,
        rom.header.is_checksum_valid(),
        rom.crc32,
        gfx_regions.len(),
        palettes.len(),
        brr_samples.len(),
        dmas.len()
    );

    for dma in &dmas {
        readme.push_str(&format!(
            "- **Code @ `0x{:06X}`** : Canal DMA `{}` -> `{}` | Source `${:02X}:{:04X}` ({}) | Taille : `{}` octets\n",
            dma.code_pc_offset,
            dma.dma_channel,
            dma.target.label(),
            dma.source_snes_bank,
            dma.source_snes_addr,
            dma.source_pc_offset
                .map(|p| format!("PC 0x{:06X}", p))
                .unwrap_or_else(|| "WRAM/RAM".to_string()),
            dma.transfer_size.unwrap_or(0)
        ));
    }

    std::fs::write(out_dir.join("README.md"), readme)?;

    Ok(DecompiledProjectSummary {
        out_dir: out_dir.display().to_string(),
        bank_count: banks.len(),
        dma_count: dmas.len(),
        gfx_count: gfx_regions.len(),
        palette_count: palettes.len(),
        audio_wav_count: brr_samples.len(),
    })
}
