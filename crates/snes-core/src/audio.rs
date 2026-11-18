#[derive(Debug, Clone)]
pub struct BrrSampleInfo {
    pub index: usize,
    pub offset: usize,
    pub byte_len: usize,
    pub block_count: usize,
    pub pcm_sample_count: usize,
    pub loops: bool,
    pub peak_amplitude: i16,
    pub label: String,
}

impl BrrSampleInfo {
    pub fn duration_ms(&self, sample_rate: u32) -> u32 {
        if sample_rate == 0 {
            return 0;
        }
        ((self.pcm_sample_count as u64 * 1000) / sample_rate as u64) as u32
    }
}

/// Decode a SNES SPC700 BRR sample slice (multiple of 9 bytes) into 16-bit signed PCM samples
pub fn decode_brr_to_pcm16(brr_bytes: &[u8]) -> Vec<i16> {
    let num_blocks = brr_bytes.len() / 9;
    let mut pcm = Vec::with_capacity(num_blocks * 16);
    let mut prev1: i32 = 0;
    let mut prev2: i32 = 0;

    for block in brr_bytes.as_chunks::<9>().0 {
        let header = block[0];
        let range = (header >> 4) & 0x0F;
        let filter = (header >> 2) & 0x03;
        let end_flag = (header & 0x01) != 0;

        for &byte in &block[1..9] {
            let nibbles = [(byte >> 4) as i8, (byte & 0x0F) as i8];
            for raw_nib in nibbles {
                // Sign-extend 4-bit nibble (-8..=7)
                let signed_nib = if raw_nib >= 8 {
                    (raw_nib as i32) - 16
                } else {
                    raw_nib as i32
                };

                // Hardware shift behavior
                let mut s = if range <= 12 {
                    (signed_nib << range) >> 1
                } else {
                    if signed_nib < 0 {
                        -2048
                    } else {
                        0
                    }
                };

                // Apply SPC700 IIR prediction filter
                match filter {
                    0 => {}
                    1 => {
                        // s += p1 * 15/16
                        s += prev1 + ((-prev1) >> 4);
                    }
                    2 => {
                        // s += p1 * 61/32 - p2 * 15/16
                        s += (prev1 << 1) + ((-prev1 * 3) >> 5) - prev2 + (prev2 >> 4);
                    }
                    3 => {
                        // s += p1 * 115/64 - p2 * 13/16
                        s += (prev1 << 1) + ((-prev1 * 13) >> 6) - prev2 + ((prev2 * 3) >> 4);
                    }
                    _ => unreachable!(),
                }

                // Clamp to 16-bit signed range and wrap 15-bit hardware accumulator
                s = s.clamp(-32768, 32767);
                let clipped = (s as i16) as i32;
                prev2 = prev1;
                prev1 = clipped;
                pcm.push(clipped as i16);
            }
        }

        if end_flag {
            break;
        }
    }

    pcm
}

/// Scan a SNES ROM for valid SPC700 BRR sample chains.
/// First checks for a hardware N-SPC Upload Stream & Sample Directory (`DIR`), then falls back to heuristic BRR block scanning.
pub fn scan_brr_samples(rom_data: &[u8], min_blocks: usize) -> Vec<BrrSampleInfo> {
    let nspc_samples = scan_nspc_directory_samples(rom_data);
    if nspc_samples.len() >= 4 {
        return nspc_samples;
    }

    let mut samples = Vec::new();
    let min_blocks = min_blocks.max(32);
    let max_blocks = 1024;
    // Skip Bank $00 (0x000000..0x008000) which holds the 65c816 CPU Reset/NMI/IRQ code & vectors
    let mut pos = if rom_data.len() > 0x008000 {
        0x008000
    } else {
        0
    };

    while pos + min_blocks * 9 <= rom_data.len() {
        // Skip internal SNES header areas
        if (0x7FC0..0x8000).contains(&pos) || (0xFFC0..0x10000).contains(&pos) {
            pos += 0x40;
            continue;
        }

        let first_hdr = rom_data[pos];
        let first_range = first_hdr >> 4;
        let first_filter = (first_hdr >> 2) & 0x03;
        let first_flags = first_hdr & 0x03;

        // Real BRR samples start with filter 0, range 1..=12, and neither end nor loop bit set on block 0
        if first_range == 0 || first_range > 12 || first_filter != 0 || first_flags != 0 {
            pos += 1;
            continue;
        }

        // Fast skip if the first block's 8 data bytes are mostly 0x00 or 0xFF (padding/sparse bitplanes)
        let first_data = &rom_data[pos + 1..pos + 9];
        let first_nz = first_data.iter().filter(|&&b| b != 0x00 && b != 0xFF).count();
        if first_nz < 4 {
            pos += 1;
            continue;
        }

        let mut block_count = 0usize;
        let mut non_zero_bytes = 0usize;
        let mut ff_bytes = 0usize;
        let mut smooth_range_steps = 0usize;
        let mut prev_range = first_range;
        let mut valid_chain = false;
        let mut loops = false;

        while pos + (block_count + 1) * 9 <= rom_data.len() && block_count < max_blocks {
            let b_start = pos + block_count * 9;
            let block = &rom_data[b_start..b_start + 9];
            let hdr = block[0];
            let range = hdr >> 4;
            let end_flag = (hdr & 0x01) != 0;
            let loop_flag = (hdr & 0x02) != 0;

            if range > 12 {
                break;
            }
            // Intermediate blocks cannot have loop_flag set without end_flag
            if !end_flag && loop_flag {
                break;
            }
            if range.abs_diff(prev_range) <= 4 {
                smooth_range_steps += 1;
            }
            prev_range = range;

            for &b in &block[1..9] {
                if b != 0x00 {
                    non_zero_bytes += 1;
                }
                if b == 0xFF {
                    ff_bytes += 1;
                }
            }

            block_count += 1;

            if end_flag {
                valid_chain = true;
                loops = loop_flag;
                break;
            }
        }

        let total_data_bytes = block_count * 8;
        if valid_chain
            && block_count >= min_blocks
            && smooth_range_steps >= (block_count * 4) / 5
            && non_zero_bytes >= (total_data_bytes * 5) / 8
            && ff_bytes < total_data_bytes / 2
        {
            let byte_len = block_count * 9;
            let pcm = decode_brr_to_pcm16(&rom_data[pos..pos + byte_len]);
            let peak = pcm
                .iter()
                .map(|&s| (s as i32).abs())
                .max()
                .unwrap_or(0)
                .min(32767) as i16;

            if peak >= 1024 {
                let index = samples.len() + 1;
                samples.push(BrrSampleInfo {
                    index,
                    offset: pos,
                    byte_len,
                    block_count,
                    pcm_sample_count: pcm.len(),
                    loops,
                    peak_amplitude: peak,
                    label: format!(
                        "Sample BRR #{:02} ({} blocs{})",
                        index,
                        block_count,
                        if loops { ", Loop" } else { "" }
                    ),
                });
                pos += byte_len;
                continue;
            }
        }

        pos += 1;
    }

    samples
}

/// Universal N-SPC Upload Block & Sample Directory (`DIR`) scanner:
/// Locates `[size_16le, dest_spc_16le]` N-SPC transfer streams in ROM and resolves the 4-byte
/// `[start_spc, loop_spc]` Sample Directory entries directly to their exact PC offsets in the BRR block.
pub fn scan_nspc_directory_samples(rom_data: &[u8]) -> Vec<BrrSampleInfo> {
    let mut results = Vec::new();
    // N-SPC audio banks are aligned on 0x8000 (LoROM bank) boundaries
    for bank_start in (0x008000..rom_data.len().saturating_sub(0x1000)).step_by(0x8000) {
        let dir_size = u16::from_le_bytes([rom_data[bank_start], rom_data[bank_start + 1]]) as usize;
        let dir_spc = u16::from_le_bytes([rom_data[bank_start + 2], rom_data[bank_start + 3]]);

        // Standard N-SPC DIR block is a multiple of 4 bytes (16..512 bytes) uploaded to page-aligned SPC RAM ($xx00)
        if !(16..=512).contains(&dir_size)
            || !dir_size.is_multiple_of(4)
            || (dir_spc & 0x00FF) != 0
            || dir_spc < 0x0200
        {
            continue;
        }

        let next_hdr = bank_start + 4 + dir_size;
        if next_hdr + 4 >= rom_data.len() {
            continue;
        }
        let brr_bank_size =
            u16::from_le_bytes([rom_data[next_hdr], rom_data[next_hdr + 1]]) as usize;
        let brr_base_spc = u16::from_le_bytes([rom_data[next_hdr + 2], rom_data[next_hdr + 3]]);
        let brr_pc_base = next_hdr + 4;

        if brr_bank_size < 512
            || brr_base_spc < 0x1000
            || brr_pc_base + brr_bank_size > rom_data.len()
        {
            continue;
        }

        let num_entries = dir_size / 4;
        for i in 0..num_entries {
            let entry_pos = bank_start + 4 + i * 4;
            let start_spc = u16::from_le_bytes([rom_data[entry_pos], rom_data[entry_pos + 1]]);
            let loop_spc = u16::from_le_bytes([rom_data[entry_pos + 2], rom_data[entry_pos + 3]]);
            if start_spc < brr_base_spc
                || (start_spc - brr_base_spc) as usize >= brr_bank_size
                || start_spc == 0xFFFF
            {
                continue;
            }

            let sample_pc = brr_pc_base + (start_spc - brr_base_spc) as usize;
            if results
                .iter()
                .any(|s: &BrrSampleInfo| s.offset == sample_pc)
            {
                continue;
            }

            // Walk 9-byte BRR blocks until the hardware end bit (hdr & 0x01) is set
            let mut block_count = 0usize;
            let mut loops = false;
            let mut valid = true;
            while sample_pc + (block_count + 1) * 9 <= brr_pc_base + brr_bank_size
                && block_count < 1024
            {
                let hdr = rom_data[sample_pc + block_count * 9];
                if (hdr >> 4) > 12 {
                    valid = false;
                    break;
                }
                block_count += 1;
                if (hdr & 0x01) != 0 {
                    loops = (hdr & 0x02) != 0 || (loop_spc >= start_spc && loop_spc != 0xFFFF);
                    break;
                }
            }

            if valid && block_count >= 6 {
                let byte_len = block_count * 9;
                let pcm = decode_brr_to_pcm16(&rom_data[sample_pc..sample_pc + byte_len]);
                let peak = pcm
                    .iter()
                    .map(|&s| (s as i32).abs())
                    .max()
                    .unwrap_or(0)
                    .min(32767) as i16;
                let index = results.len() + 1;
                results.push(BrrSampleInfo {
                    index,
                    offset: sample_pc,
                    byte_len,
                    block_count,
                    pcm_sample_count: pcm.len(),
                    loops,
                    peak_amplitude: peak,
                    label: format!(
                        "N-SPC Sample #{:02} [SPC ${:04X}] ({} blocs{})",
                        index,
                        start_spc,
                        block_count,
                        if loops { ", Loop" } else { "" }
                    ),
                });
            }
        }

        if results.len() >= 4 {
            return results;
        }
    }

    results
}

/// Adapt a donor BRR sample from ROM B so that it fits safely into a target BRR sample slot in ROM A
pub fn adapt_brr_for_swap(
    donor_brr: &[u8],
    target_byte_len: usize,
    target_loops: bool,
) -> Vec<u8> {
    let target_blocks = (target_byte_len / 9).max(1);
    let donor_blocks = (donor_brr.len() / 9).max(1);
    let mut out = vec![0u8; target_blocks * 9];

    let copy_blocks = target_blocks.min(donor_blocks);
    out[..copy_blocks * 9].copy_from_slice(&donor_brr[..copy_blocks * 9]);

    // Clear end flag on all blocks before the final block
    for b in 0..target_blocks.saturating_sub(1) {
        out[b * 9] &= !0x01;
    }

    // Set end flag (and optional loop flag) on the very last block of the target slot
    let last_hdr_idx = (target_blocks - 1) * 9;
    out[last_hdr_idx] |= 0x01;
    if target_loops {
        out[last_hdr_idx] |= 0x02;
    } else {
        out[last_hdr_idx] &= !0x02;
    }

    out
}

/// Universal SPC700 BRR Encoder: encodes 16-bit signed PCM samples into 9-byte BRR blocks,
/// choosing the optimal shift range (0..=12) and IIR filter (0..=3) for each 16-sample block.
pub fn encode_pcm16_to_brr(pcm: &[i16], loops: bool) -> Vec<u8> {
    let total_blocks = pcm.len().div_ceil(16).max(1);
    let mut out = Vec::with_capacity(total_blocks * 9);
    let mut hist1: i32 = 0;
    let mut hist2: i32 = 0;

    for blk_idx in 0..total_blocks {
        // Hardware N-SPC initialization block at block 0 to clear SPC700 Gaussian/IIR history
        if blk_idx == 0 && total_blocks >= 2 {
            out.extend_from_slice(&[if loops { 0x02 } else { 0x00 }, 0, 0, 0, 0, 0, 0, 0, 0]);
            continue;
        }
        let mut samples = [0i16; 16];
        for i in 0..16 {
            let src_idx = blk_idx * 16 + i;
            if src_idx < pcm.len() {
                samples[i] = pcm[src_idx];
            }
        }

        let max_filter = if blk_idx == 0 { 0u8 } else { 3u8 };
        let mut best_error = u64::MAX;
        let mut best_block = [0u8; 9];
        let mut best_p1 = hist1;
        let mut best_p2 = hist2;

        for filter in 0..=max_filter {
            for range in 0..=12u8 {
                let mut p1 = hist1;
                let mut p2 = hist2;
                let mut err_acc = 0u64;
                let mut nibbles = [0u8; 16];

                for (i, &target_s16) in samples.iter().enumerate() {
                    let target = target_s16 as i32;
                    let pred = match filter {
                        0 => 0,
                        1 => p1 + ((-p1) >> 4),
                        2 => (p1 << 1) + ((-p1 * 3) >> 5) - p2 + (p2 >> 4),
                        3 => (p1 << 1) + ((-p1 * 13) >> 6) - p2 + ((p2 * 3) >> 4),
                        _ => 0,
                    };

                    let diff = target - pred;
                    let step = (1i32 << range) >> 1;
                    let raw_nib = if step > 0 {
                        ((diff as f32) / (step as f32)).round() as i32
                    } else {
                        diff.clamp(-8, 7)
                    };
                    let nib = raw_nib.clamp(-8, 7);
                    nibbles[i] = (nib as i8 as u8) & 0x0F;

                    let recon_delta = (nib << range) >> 1;
                    let mut recon = (pred + recon_delta).clamp(-32768, 32767);
                    recon = (recon as i16) as i32;

                    let e = (target - recon).unsigned_abs() as u64;
                    err_acc += e * e;
                    p2 = p1;
                    p1 = recon;
                }

                if err_acc < best_error {
                    best_error = err_acc;
                    best_p1 = p1;
                    best_p2 = p2;
                    let is_last = blk_idx + 1 == total_blocks;
                    let mut hdr = (range << 4) | (filter << 2);
                    if is_last {
                        hdr |= 0x01;
                        if loops {
                            hdr |= 0x02;
                        }
                    }
                    best_block[0] = hdr;
                    for b in 0..8 {
                        best_block[1 + b] = (nibbles[b * 2] << 4) | (nibbles[b * 2 + 1] & 0x0F);
                    }
                }
            }
        }

        hist1 = best_p1;
        hist2 = best_p2;
        out.extend_from_slice(&best_block);
    }

    out
}

/// Resample a donor BRR sample in the time domain so 100% of its waveform fits inside `target_byte_len`
pub fn resample_and_encode_brr_for_swap(
    donor_brr: &[u8],
    target_byte_len: usize,
    target_loops: bool,
) -> Vec<u8> {
    let target_blocks = (target_byte_len / 9).max(1);
    let target_pcm_len = target_blocks * 16;
    let donor_pcm = decode_brr_to_pcm16(donor_brr);

    if donor_pcm.is_empty() {
        return adapt_brr_for_swap(donor_brr, target_byte_len, target_loops);
    }

    let mut resampled = Vec::with_capacity(target_pcm_len);
    let scale = (donor_pcm.len().saturating_sub(1) as f32) / (target_pcm_len.saturating_sub(1).max(1) as f32);

    for i in 0..target_pcm_len {
        let src_pos = (i as f32) * scale;
        let idx0 = (src_pos.floor() as usize).min(donor_pcm.len() - 1);
        let idx1 = (idx0 + 1).min(donor_pcm.len() - 1);
        let frac = src_pos - (idx0 as f32);
        let s0 = donor_pcm[idx0] as f32;
        let s1 = donor_pcm[idx1] as f32;
        resampled.push((s0 + (s1 - s0) * frac).round() as i16);
    }

    encode_pcm16_to_brr(&resampled, target_loops)
}

/// Encode 16-bit mono PCM samples into a standard RIFF WAVE (.wav) byte buffer
pub fn encode_pcm16_to_wav(samples: &[i16], sample_rate: u32) -> Vec<u8> {
    let data_size = (samples.len() * 2) as u32;
    let riff_size = 36 + data_size;
    let byte_rate = sample_rate * 2;
    let mut wav = Vec::with_capacity((44 + data_size) as usize);

    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&riff_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // PCM chunk size
    wav.extend_from_slice(&1u16.to_le_bytes()); // AudioFormat = 1 (PCM)
    wav.extend_from_slice(&1u16.to_le_bytes()); // NumChannels = 1 (Mono)
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes()); // BlockAlign = 2
    wav.extend_from_slice(&16u16.to_le_bytes()); // BitsPerSample = 16
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());

    for &s in samples {
        wav.extend_from_slice(&s.to_le_bytes());
    }

    wav
}

/// Decode a standard RIFF WAVE (.wav) file into mono 16-bit signed PCM samples (+ sample rate).
/// Supports 8-bit unsigned and 16-bit signed PCM, mono or stereo (downmixed to mono).
pub fn decode_wav_to_pcm16(wav_bytes: &[u8]) -> Option<(u32, Vec<i16>)> {
    if wav_bytes.len() < 44 || &wav_bytes[0..4] != b"RIFF" || &wav_bytes[8..12] != b"WAVE" {
        return None;
    }

    let mut pos = 12usize;
    let mut num_channels = 1u16;
    let mut sample_rate = 16000u32;
    let mut bits_per_sample = 16u16;
    let mut data_slice: Option<&[u8]> = None;

    while pos + 8 <= wav_bytes.len() {
        let chunk_id = &wav_bytes[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            wav_bytes[pos + 4],
            wav_bytes[pos + 5],
            wav_bytes[pos + 6],
            wav_bytes[pos + 7],
        ]) as usize;
        pos += 8;
        if pos + chunk_size > wav_bytes.len() {
            if chunk_id == b"data" {
                data_slice = Some(&wav_bytes[pos..]);
            }
            break;
        }

        if chunk_id == b"fmt " && chunk_size >= 16 {
            let audio_fmt = u16::from_le_bytes([wav_bytes[pos], wav_bytes[pos + 1]]);
            if audio_fmt != 1 {
                return None; // Only standard PCM supported
            }
            num_channels = u16::from_le_bytes([wav_bytes[pos + 2], wav_bytes[pos + 3]]).max(1);
            sample_rate = u32::from_le_bytes([
                wav_bytes[pos + 4],
                wav_bytes[pos + 5],
                wav_bytes[pos + 6],
                wav_bytes[pos + 7],
            ]);
            bits_per_sample = u16::from_le_bytes([wav_bytes[pos + 14], wav_bytes[pos + 15]]);
        } else if chunk_id == b"data" {
            data_slice = Some(&wav_bytes[pos..pos + chunk_size]);
        }

        pos += chunk_size + (chunk_size & 1);
    }

    let raw = data_slice?;
    let mut mono = Vec::new();
    let ch = num_channels as usize;

    if bits_per_sample == 16 {
        let frame_bytes = 2 * ch;
        for frame in raw.chunks_exact(frame_bytes) {
            let mut acc = 0i32;
            for c in 0..ch {
                let s = i16::from_le_bytes([frame[c * 2], frame[c * 2 + 1]]) as i32;
                acc += s;
            }
            mono.push((acc / ch as i32) as i16);
        }
    } else if bits_per_sample == 8 {
        for frame in raw.chunks_exact(ch) {
            let mut acc = 0i32;
            for &b in frame {
                acc += ((b as i32) - 128) << 8;
            }
            mono.push((acc / ch as i32) as i16);
        }
    } else {
        return None;
    }

    Some((sample_rate, mono))
}

/// Resample arbitrary 16-bit PCM samples to fit an exact SNES BRR byte budget (`target_byte_len`, multiple of 9)
pub fn encode_pcm16_to_brr_fit_budget(
    pcm: &[i16],
    target_byte_len: usize,
    target_loops: bool,
) -> Vec<u8> {
    let target_blocks = (target_byte_len / 9).max(1);
    let target_pcm_len = target_blocks * 16;
    if pcm.is_empty() {
        return encode_pcm16_to_brr(&vec![0i16; target_pcm_len], target_loops);
    }

    let mut resampled = Vec::with_capacity(target_pcm_len);
    let scale = (pcm.len().saturating_sub(1) as f32) / (target_pcm_len.saturating_sub(1).max(1) as f32);
    for i in 0..target_pcm_len {
        let src_pos = (i as f32) * scale;
        let idx0 = (src_pos.floor() as usize).min(pcm.len() - 1);
        let idx1 = (idx0 + 1).min(pcm.len() - 1);
        let frac = src_pos - (idx0 as f32);
        let s0 = pcm[idx0] as f32;
        let s1 = pcm[idx1] as f32;
        resampled.push((s0 + (s1 - s0) * frac).round() as i16);
    }

    let mut brr = encode_pcm16_to_brr(&resampled, target_loops);
    brr.resize(target_blocks * 9, 0);
    brr
}
