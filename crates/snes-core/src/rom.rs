use std::fmt;

#[derive(Debug)]
pub enum RomError {
    FileTooSmall(usize),
    OutOfBounds { offset: usize, len: usize },
    Io(std::io::Error),
}

impl fmt::Display for RomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RomError::FileTooSmall(sz) => write!(
                f,
                "Le fichier ROM est trop petit ({} octets, minimum 32 Ko requis)",
                sz
            ),
            RomError::OutOfBounds { offset, len } => write!(
                f,
                "Plage d'octets hors limites (offset 0x{:X}, taille {})",
                offset, len
            ),
            RomError::Io(e) => write!(f, "Erreur d'entrée/sortie: {}", e),
        }
    }
}

impl std::error::Error for RomError {}

impl From<std::io::Error> for RomError {
    fn from(e: std::io::Error) -> Self {
        RomError::Io(e)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RomMapping {
    LoROM,
    HiROM,
    ExHiROM,
}

impl RomMapping {
    pub fn as_str(&self) -> &'static str {
        match self {
            RomMapping::LoROM => "LoROM ($007FC0)",
            RomMapping::HiROM => "HiROM ($00FFC0)",
            RomMapping::ExHiROM => "ExHiROM ($40FFC0)",
        }
    }

    pub fn header_base_offset(&self) -> usize {
        match self {
            RomMapping::LoROM => 0x007FC0,
            RomMapping::HiROM => 0x00FFC0,
            RomMapping::ExHiROM => 0x40FFC0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SnesHeader {
    pub title: String,
    pub mapping: RomMapping,
    pub map_mode_byte: u8,
    pub fast_rom: bool,
    pub chipset_byte: u8,
    pub chipset_description: String,
    pub rom_size_byte: u8,
    pub declared_rom_size_kb: usize,
    pub sram_size_byte: u8,
    pub sram_size_kb: usize,
    pub region_byte: u8,
    pub region_name: String,
    pub developer_id: u8,
    pub version: u8,
    pub checksum_complement: u16,
    pub checksum: u16,
    pub computed_checksum: u16,
    pub reset_vector: u16,
    pub nmi_vector: u16,
    pub irq_vector: u16,
}

impl SnesHeader {
    pub fn is_checksum_valid(&self) -> bool {
        (self.checksum ^ self.checksum_complement) == 0xFFFF
            && self.checksum == self.computed_checksum
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecognizedGame {
    SuperMarioWorld,
    ZeldaAlttp,
    Unknown,
}

impl RecognizedGame {
    pub fn canonical_title(&self) -> &'static str {
        match self {
            RecognizedGame::SuperMarioWorld => "SUPER MARIOWORLD",
            RecognizedGame::ZeldaAlttp => "THE LEGEND OF ZELDA",
            RecognizedGame::Unknown => "INCONNU / PERSONNALISÉ",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SnesRom {
    pub name: String,
    pub has_copier_header: bool,
    pub copier_header: Vec<u8>,
    pub data: Vec<u8>,
    pub header: SnesHeader,
    pub crc32: u32,
}

impl SnesRom {
    pub fn from_bytes(name: impl Into<String>, raw_bytes: &[u8]) -> Result<Self, RomError> {
        if raw_bytes.len() < 0x8000 {
            return Err(RomError::FileTooSmall(raw_bytes.len()));
        }

        // Detect 512-byte SMC/SWC copier header
        let has_copier_header = (raw_bytes.len() % 1024) == 512;
        let (copier_header, rom_slice) = if has_copier_header {
            (raw_bytes[..512].to_vec(), &raw_bytes[512..])
        } else {
            (Vec::new(), raw_bytes)
        };

        if rom_slice.len() < 0x8000 {
            return Err(RomError::FileTooSmall(rom_slice.len()));
        }

        let data = rom_slice.to_vec();
        let mapping = detect_best_mapping(&data);
        let computed_checksum = compute_snes_checksum(&data);
        let header = parse_header_at(&data, mapping, computed_checksum);
        let crc32 = compute_crc32(&data);

        let mut rom = Self {
            name: name.into(),
            has_copier_header,
            copier_header,
            data,
            header,
            crc32,
        };
        rom.normalize_identified_header();
        Ok(rom)
    }

    /// Identify the game based purely on internal SNES header metadata, vectors, CRC32, and hardware structure
    pub fn identify_game(&self) -> RecognizedGame {
        // 1. Super Mario World
        if self.header.title.to_uppercase().contains("MARIO")
            || self.crc32 == 0xB19ED489
            || (self.data.len() == 512 * 1024
                && self.header.mapping == RomMapping::LoROM
                && self.header.reset_vector == 0x8000
                && self.header.nmi_vector == 0x816A
                && self.header.irq_vector == 0x8374)
        {
            return RecognizedGame::SuperMarioWorld;
        }

        // 2. Zelda ALTTP (including cases where header title is "test" or modified)
        if self.header.title.to_uppercase().contains("ZELDA")
            || self.crc32 == 0xB63BFE15
            || (self.data.len() == 1024 * 1024
                && self.header.mapping == RomMapping::LoROM
                && self.header.reset_vector == 0x8000
                && self.header.nmi_vector == 0x80C9
                && self.header.irq_vector == 0x82D8)
            || (self.header.title.trim().eq_ignore_ascii_case("test")
                && self.data.len() >= 0x100000
                && self.header.reset_vector == 0x8000)
        {
            return RecognizedGame::ZeldaAlttp;
        }

        RecognizedGame::Unknown
    }

    /// Returns the authentic canonical game title (e.g. "SUPER MARIOWORLD", "THE LEGEND OF ZELDA")
    pub fn canonical_title(&self) -> String {
        let recognized = self.identify_game();
        if recognized != RecognizedGame::Unknown {
            recognized.canonical_title().to_string()
        } else {
            self.header.title.trim().to_string()
        }
    }

    /// If the ROM header has a dummy/test title (e.g. "test"), replace it with the true identified value
    pub fn normalize_identified_header(&mut self) {
        if self.identify_game() == RecognizedGame::ZeldaAlttp
            && self.header.title.trim().eq_ignore_ascii_case("test")
        {
            let canonical = "THE LEGEND OF ZELDA  ";
            let base = self.header.mapping.header_base_offset();
            if base + 21 <= self.data.len() {
                self.data[base..base + 21].copy_from_slice(canonical.as_bytes());
            }
            self.header.title = canonical.trim().to_string();
            self.fix_checksum();
        }
    }

    pub fn refresh_metadata(&mut self) {
        self.header.computed_checksum = compute_snes_checksum(&self.data);
        self.crc32 = compute_crc32(&self.data);
    }

    /// Recalculate and write the official SNES checksum and complement into the internal header
    pub fn fix_checksum(&mut self) {
        let base = self.header.mapping.header_base_offset();
        if base + 0x20 <= self.data.len() {
            // Set checksum + complement to 0x0000 and 0xFFFF before computing
            self.data[base + 0x1C] = 0xFF;
            self.data[base + 0x1D] = 0xFF;
            self.data[base + 0x1E] = 0x00;
            self.data[base + 0x1F] = 0x00;

            let sum = compute_snes_checksum(&self.data);
            let complement = sum ^ 0xFFFF;

            self.data[base + 0x1C] = (complement & 0xFF) as u8;
            self.data[base + 0x1D] = (complement >> 8) as u8;
            self.data[base + 0x1E] = (sum & 0xFF) as u8;
            self.data[base + 0x1F] = (sum >> 8) as u8;
        }
        let computed = compute_snes_checksum(&self.data);
        self.header = parse_header_at(&self.data, self.header.mapping, computed);
        self.crc32 = compute_crc32(&self.data);
    }

    /// Export ROM bytes (without the legacy 512-byte copier header, standard .sfc format)
    pub fn to_sfc_bytes(&self) -> Vec<u8> {
        self.data.clone()
    }

    /// Convert PC (headerless file) offset to SNES Bank:Address (`$BB:AAAA`)
    pub fn pc_to_snes(&self, pc_offset: usize) -> (u8, u16) {
        match self.header.mapping {
            RomMapping::LoROM => {
                let bank_raw = pc_offset / 0x8000;
                let bank = if bank_raw >= 0x7E {
                    // ExLoROM / FastROM upper mirror banks
                    (0x80 + (bank_raw & 0x7F)) as u8
                } else {
                    (bank_raw & 0x7F) as u8
                };
                let addr = 0x8000 + ((pc_offset % 0x8000) as u16);
                (bank, addr)
            }
            RomMapping::HiROM => {
                let bank = (0xC0 + ((pc_offset / 0x10000) & 0x3F)) as u8;
                let addr = (pc_offset & 0xFFFF) as u16;
                (bank, addr)
            }
            RomMapping::ExHiROM => {
                if pc_offset < 0x400000 {
                    let bank = (0xC0 + ((pc_offset / 0x10000) & 0x3F)) as u8;
                    let addr = (pc_offset & 0xFFFF) as u16;
                    (bank, addr)
                } else {
                    let bank = (0x40 + (((pc_offset - 0x400000) / 0x10000) & 0x3F)) as u8;
                    let addr = (pc_offset & 0xFFFF) as u16;
                    (bank, addr)
                }
            }
        }
    }

    /// Convert SNES Bank:Address (`$BB:AAAA`) to headerless PC file offset for ALL mapping modes
    pub fn snes_to_pc(&self, bank: u8, addr: u16) -> Option<usize> {
        let len = self.data.len();
        if len == 0 {
            return None;
        }
        let pc = match self.header.mapping {
            RomMapping::LoROM => {
                // Skip WRAM banks $7E/$7F
                if bank == 0x7E || bank == 0x7F {
                    return None;
                }
                if addr < 0x8000 {
                    // Banks $40..$7D or $C0..$FF can mirror lower half in some LoROM carts
                    if (0x40..=0x7D).contains(&bank) || bank >= 0xC0 {
                        let b = (bank & 0x7F) as usize;
                        b * 0x8000 + (addr as usize)
                    } else {
                        return None;
                    }
                } else {
                    let b = (bank & 0x7F) as usize;
                    b * 0x8000 + ((addr - 0x8000) as usize)
                }
            }
            RomMapping::HiROM => {
                if bank == 0x7E || bank == 0x7F {
                    return None;
                }
                if bank >= 0xC0 || (0x40..=0x7D).contains(&bank) || addr >= 0x8000 {
                    ((bank & 0x3F) as usize) * 0x10000 + (addr as usize)
                } else {
                    return None;
                }
            }
            RomMapping::ExHiROM => {
                if bank == 0x7E || bank == 0x7F {
                    return None;
                }
                if bank >= 0xC0 {
                    ((bank - 0xC0) as usize) * 0x10000 + (addr as usize)
                } else if (0x40..=0x7D).contains(&bank) {
                    0x400000 + ((bank - 0x40) as usize) * 0x10000 + (addr as usize)
                } else if (0x80..=0xBF).contains(&bank) && addr >= 0x8000 {
                    ((bank - 0x80) as usize) * 0x10000 + (addr as usize)
                } else if bank <= 0x3F && addr >= 0x8000 {
                    0x400000 + (bank as usize) * 0x10000 + (addr as usize)
                } else {
                    return None;
                }
            }
        };
        if pc < len {
            Some(pc)
        } else {
            None
        }
    }

    /// Format a PC offset as both PC hex and SNES `$BB:AAAA`
    pub fn format_address(&self, pc_offset: usize) -> String {
        let (bank, addr) = self.pc_to_snes(pc_offset);
        format!("0x{:06X} (${:02X}:{:04X})", pc_offset, bank, addr)
    }

    pub fn write_bytes(&mut self, offset: usize, bytes: &[u8]) -> Result<(), RomError> {
        if offset + bytes.len() > self.data.len() {
            return Err(RomError::OutOfBounds {
                offset,
                len: bytes.len(),
            });
        }
        self.data[offset..offset + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }

    /// Size in bytes of each memory bank according to the detected mapping (32 KB for LoROM, 64 KB for HiROM/ExHiROM)
    pub fn bank_size(&self) -> usize {
        match self.header.mapping {
            RomMapping::LoROM => 32 * 1024,
            RomMapping::HiROM | RomMapping::ExHiROM => 64 * 1024,
        }
    }

    /// Slice the pure ROM image into hardware memory banks (32 KB each in LoROM, 64 KB each in HiROM/ExHiROM)
    pub fn banks(&self) -> Vec<&[u8]> {
        self.data.chunks(self.bank_size()).collect()
    }
}

fn detect_best_mapping(data: &[u8]) -> RomMapping {
    let mut best_mapping = RomMapping::LoROM;
    let mut best_score = score_header_candidate(data, RomMapping::LoROM);

    if data.len() >= 0x10000 {
        let hi_score = score_header_candidate(data, RomMapping::HiROM);
        if hi_score > best_score {
            best_score = hi_score;
            best_mapping = RomMapping::HiROM;
        }
    }

    if data.len() >= 0x410000 {
        let ex_score = score_header_candidate(data, RomMapping::ExHiROM);
        if ex_score > best_score {
            best_mapping = RomMapping::ExHiROM;
        }
    }

    best_mapping
}

fn score_header_candidate(data: &[u8], mapping: RomMapping) -> i32 {
    let base = mapping.header_base_offset();
    if data.len() < base + 0x40 {
        return -1000;
    }

    let mut score = 0i32;

    // 1. Checksum + complement == 0xFFFF and exact computed checksum match
    let complement = u16::from_le_bytes([data[base + 0x1C], data[base + 0x1D]]);
    let checksum = u16::from_le_bytes([data[base + 0x1E], data[base + 0x1F]]);
    if (checksum ^ complement) == 0xFFFF && checksum != 0 && complement != 0 {
        score += 8;
        if checksum == compute_snes_checksum(data) {
            score += 10;
        }
    }

    // 2. Map mode byte plausibility
    let map_mode = data[base + 0x15];
    let mode_low = map_mode & 0x0F;
    match (mapping, mode_low) {
        (RomMapping::LoROM, 0x0 | 0x2 | 0x3) => score += 4,
        (RomMapping::HiROM, 0x1 | 0xA) => score += 4,
        (RomMapping::ExHiROM, 0x5) => score += 6,
        _ => {}
    }

    // 3. Reset vector >= 0x8000 and plausible first opcode (not BRK 0x00 / 0xFF)
    let reset_vec = u16::from_le_bytes([data[base + 0x3C], data[base + 0x3D]]);
    if reset_vec >= 0x8000 {
        score += 4;
        let reset_pc = match mapping {
            RomMapping::LoROM => (reset_vec - 0x8000) as usize,
            RomMapping::HiROM | RomMapping::ExHiROM => reset_vec as usize,
        };
        if reset_pc < data.len() {
            let op = data[reset_pc];
            if op == 0x78
                || op == 0x18
                || op == 0xC2
                || op == 0xE2
                || op == 0x4C
                || op == 0x5C
                || op == 0x9C
            {
                // Classic 65c816 reset entry opcodes: SEI, CLC, REP, SEP, JMP, JML, STZ
                score += 4;
            } else if op == 0x00 || op == 0xFF {
                score -= 2;
            }
        }
    } else {
        score -= 4;
    }

    // 4. Printable ASCII / JIS title characters (21 bytes)
    let title_bytes = &data[base..base + 21];
    let printable_count = title_bytes
        .iter()
        .filter(|&&b| (0x20..=0x7E).contains(&b) || (0xA1..=0xDF).contains(&b))
        .count();
    score += (printable_count as i32) / 3;

    // 5. Plausible ROM size byte (0x07..=0x0D -> 128KB..8MB)
    let rom_size = data[base + 0x17];
    if (0x05..=0x0E).contains(&rom_size) {
        score += 3;
    }

    // 6. Plausible SRAM size byte (0x00..=0x08)
    let sram_size = data[base + 0x18];
    if sram_size <= 0x08 {
        score += 2;
    }

    score
}

fn parse_header_at(data: &[u8], mapping: RomMapping, computed_checksum: u16) -> SnesHeader {
    let base = mapping.header_base_offset();
    if data.len() < base + 0x40 {
        return SnesHeader {
            title: "UNKNOWN".to_string(),
            mapping,
            map_mode_byte: 0,
            fast_rom: false,
            chipset_byte: 0,
            chipset_description: "Standard".to_string(),
            rom_size_byte: 0,
            declared_rom_size_kb: data.len() / 1024,
            sram_size_byte: 0,
            sram_size_kb: 0,
            region_byte: 0,
            region_name: "Unknown".to_string(),
            developer_id: 0,
            version: 0,
            checksum_complement: 0,
            checksum: 0,
            computed_checksum,
            reset_vector: 0,
            nmi_vector: 0,
            irq_vector: 0,
        };
    }

    let raw_title = &data[base..base + 21];
    let title: String = raw_title
        .iter()
        .map(|&b| {
            if (0x20..=0x7E).contains(&b) {
                b as char
            } else {
                ' '
            }
        })
        .collect::<String>()
        .trim()
        .to_string();

    let map_mode_byte = data[base + 0x15];
    let fast_rom = (map_mode_byte & 0x10) != 0;
    let chipset_byte = data[base + 0x16];
    let chipset_description = match chipset_byte {
        0x00 => "ROM uniquement",
        0x01 => "ROM + RAM",
        0x02 => "ROM + RAM + Batterie (SRAM)",
        0x03..=0x05 => "Co-processeur DSP (DSP-1/2/3/4)",
        0x13..=0x1A => "Super FX (GSU-1 / GSU-2)",
        0x25 => "OBC-1",
        0x34 | 0x35 => "SA-1",
        0x43 | 0x45 => "S-DD1",
        0xF3 => "CX4 (Capcom)",
        _ => "Chipset spécial / personnalisé",
    }
    .to_string();

    let rom_size_byte = data[base + 0x17];
    let declared_rom_size_kb = if rom_size_byte <= 16 {
        1usize << rom_size_byte
    } else {
        data.len() / 1024
    };

    let sram_size_byte = data[base + 0x18];
    let sram_size_kb = if sram_size_byte == 0 {
        0
    } else if sram_size_byte <= 12 {
        1usize << sram_size_byte
    } else {
        0
    };

    let region_byte = data[base + 0x19];
    let region_name = match region_byte {
        0x00 => "Japon (NTSC)",
        0x01 => "Amérique du Nord (NTSC)",
        0x02 => "Europe / Océanie (PAL)",
        0x03 => "Scandinavie (PAL)",
        0x06 => "France (SECAM/PAL)",
        0x07 => "Pays-Bas (PAL)",
        0x08 => "Espagne (PAL)",
        0x09 => "Allemagne (PAL)",
        0x0A => "Italie (PAL)",
        0x0D => "Corée du Sud (NTSC)",
        _ => "Autre / International",
    }
    .to_string();

    let developer_id = data[base + 0x1A];
    let version = data[base + 0x1B];
    let checksum_complement = u16::from_le_bytes([data[base + 0x1C], data[base + 0x1D]]);
    let checksum = u16::from_le_bytes([data[base + 0x1E], data[base + 0x1F]]);
    let nmi_vector = u16::from_le_bytes([data[base + 0x2A], data[base + 0x2B]]);
    let irq_vector = u16::from_le_bytes([data[base + 0x2E], data[base + 0x2F]]);
    let reset_vector = u16::from_le_bytes([data[base + 0x3C], data[base + 0x3D]]);

    SnesHeader {
        title: if title.is_empty() {
            "(Sans titre)".to_string()
        } else {
            title
        },
        mapping,
        map_mode_byte,
        fast_rom,
        chipset_byte,
        chipset_description,
        rom_size_byte,
        declared_rom_size_kb,
        sram_size_byte,
        sram_size_kb,
        region_byte,
        region_name,
        developer_id,
        version,
        checksum_complement,
        checksum,
        computed_checksum,
        reset_vector,
        nmi_vector,
        irq_vector,
    }
}

/// Compute official SNES 16-bit checksum (handling ALL non-power-of-two ROM sizes like 1.5MB, 2.5MB, 3MB, 6MB by exact hardware mirroring)
pub fn compute_snes_checksum(data: &[u8]) -> u16 {
    if data.is_empty() {
        return 0;
    }
    let len = data.len();
    let highest_pow2 = 1usize << len.ilog2();
    let mut sum: u32 = data[..highest_pow2].iter().map(|&b| b as u32).sum();

    let remainder = len - highest_pow2;
    if let Some(full_repeats) = highest_pow2.checked_div(remainder) {
        let rem_slice = &data[highest_pow2..];
        let rem_tail = highest_pow2 % remainder;
        let rem_sum: u32 = rem_slice.iter().map(|&b| b as u32).sum();
        sum = sum.wrapping_add(rem_sum.wrapping_mul(full_repeats as u32));
        if rem_tail > 0 {
            let tail_sum: u32 = rem_slice[..rem_tail].iter().map(|&b| b as u32).sum();
            sum = sum.wrapping_add(tail_sum);
        }
    }

    (sum & 0xFFFF) as u16
}

/// Fast standard IEEE 802.3 CRC32
pub fn compute_crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if (crc & 1) != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Verified unheadered No-Intro SHA-256 of `Legend of Zelda, The - A Link to the Past (France).sfc` (PAL, Rev 1.0, 1 MiB)
pub const VERIFIED_ZELDA_FR_PAL_SHA256: &str =
    "29a0d5812ccbb4b11bdb55d8f751a2a797b4110bf402ca8ba15eb2bf85db7b39";
pub const VERIFIED_ZELDA_FR_PAL_SIZE: usize = 1_048_576;

/// Verified unheadered No-Intro SHA-256 of `Super Mario World (Europe).sfc` (PAL, Rev 1.0, 512 KiB)
pub const VERIFIED_SMW_EU_PAL_SHA256: &str =
    "b5be1dba3012b6811a5660fbf2981cb23cdd1e48f845a42df00f0f55b19f0392";
pub const VERIFIED_SMW_EU_PAL_SIZE: usize = 524_288;

/// Standard zero-dependency FIPS 180-4 SHA-256 digest.
pub fn compute_sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut padded = Vec::with_capacity((data.len() + 9 + 63) & !63);
    padded.extend_from_slice(data);
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    let mut w = [0u32; 64];
    for chunk in padded.as_chunks::<64>().0 {
        for (i, word) in w.iter_mut().take(16).enumerate() {
            let o = i * 4;
            *word = u32::from_be_bytes([chunk[o], chunk[o + 1], chunk[o + 2], chunk[o + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// Lowercase 64-character hexadecimal SHA-256 digest.
pub fn compute_sha256_hex(data: &[u8]) -> String {
    let digest = compute_sha256(data);
    let mut s = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(&mut s, "{b:02x}");
    }
    s
}

/// Strictly verify that the supplied base Zelda ALTTP and donor Super Mario World ROMs match
/// the verified unheadered No-Intro hashes required by The Legend of Peach.
pub fn verify_legend_of_peach_inputs(
    zelda: &SnesRom,
    smw: &SnesRom,
    allow_unverified: bool,
) -> Result<(String, String), String> {
    let z_sha = compute_sha256_hex(&zelda.data);
    let m_sha = compute_sha256_hex(&smw.data);

    let z_ok =
        zelda.data.len() == VERIFIED_ZELDA_FR_PAL_SIZE && z_sha == VERIFIED_ZELDA_FR_PAL_SHA256;
    let m_ok = smw.data.len() == VERIFIED_SMW_EU_PAL_SIZE && m_sha == VERIFIED_SMW_EU_PAL_SHA256;

    if !z_ok && !allow_unverified {
        return Err(format!(
            "Unsupported base ROM for The Legend of Zelda: A Link to the Past.\n\
             Expected: Legend of Zelda, The - A Link to the Past (France).sfc (PAL, Rev 1.0, unheadered 1,048,576 bytes)\n\
             Expected SHA-256: {VERIFIED_ZELDA_FR_PAL_SHA256}\n\
             Actual   SHA-256: {z_sha} ({} bytes)\n\
             See docs/ROM_REQUIREMENTS.md for verification instructions.",
            zelda.data.len()
        ));
    }

    if !m_ok && !allow_unverified {
        return Err(format!(
            "Unsupported donor ROM for Super Mario World.\n\
             Expected: Super Mario World (Europe).sfc (PAL, Rev 1.0, unheadered 524,288 bytes)\n\
             Expected SHA-256: {VERIFIED_SMW_EU_PAL_SHA256}\n\
             Actual   SHA-256: {m_sha} ({} bytes)\n\
             See docs/ROM_REQUIREMENTS.md for verification instructions.",
            smw.data.len()
        ));
    }

    Ok((z_sha, m_sha))
}
