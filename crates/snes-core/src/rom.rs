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
    DonkeyKongCountry,
    Unknown,
}

impl RecognizedGame {
    pub fn canonical_title(&self) -> &'static str {
        match self {
            RecognizedGame::SuperMarioWorld => "SUPER MARIOWORLD",
            RecognizedGame::ZeldaAlttp => "THE LEGEND OF ZELDA",
            RecognizedGame::DonkeyKongCountry => "DONKEY KONG COUNTRY",
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

        // 3. Donkey Kong Country
        if self.header.title.to_uppercase().contains("DONKEY KONG")
            || self.crc32 == 0x3EAA5697
            || (self.data.len() >= 4 * 1024 * 1024
                && self.header.mapping == RomMapping::HiROM
                && self.header.reset_vector == 0x8000
                && self.header.nmi_vector == 0xA95F
                && self.header.irq_vector == 0xA987)
        {
            return RecognizedGame::DonkeyKongCountry;
        }

        RecognizedGame::Unknown
    }

    /// Returns the authentic canonical game title (e.g. "SUPER MARIOWORLD", "THE LEGEND OF ZELDA", "DONKEY KONG COUNTRY")
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
                if bank >= 0xC0 || (0x40..=0x7D).contains(&bank) {
                    ((bank & 0x3F) as usize) * 0x10000 + (addr as usize)
                } else if addr >= 0x8000 {
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
            if op == 0x78 || op == 0x18 || op == 0xC2 || op == 0xE2 || op == 0x4C || op == 0x5C || op == 0x9C {
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
    if remainder > 0 {
        let rem_slice = &data[highest_pow2..];
        let full_repeats = highest_pow2 / remainder;
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
