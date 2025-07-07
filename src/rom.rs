// Each cartridge carried at least two large ROM chips - the Character ROM (CHR ROM) and the Program ROM (PRG ROM). 
// The former stored a game's video graphics data, the latter stored CPU instructions - the game's code
// The later version of cartridges carried additional hardware (ROM and RAM) accessible through so-called mappers. 

const NES_TAG: [u8; 4] = [0x4E, 0x45, 0x53, 0x1A];
const PROM_PAGE_SIZE: usize = 16384;
const CROM_PAGE_SIZE: usize = 8192;

#[derive(Debug, PartialEq)]
pub enum Mirroring {
    VERTICAL,
    HORIZONTAL,
    FOURSCREEN,
}

pub struct Rom {
    pub p_rom: Vec<u8>,
    pub c_rom: Vec<u8>,
    pub mapper: u8,
    pub mirroring: Mirroring,
}

impl Rom {
    pub fn new(raw: &Vec<u8>) -> Result<Rom, String> {
        // First 4 bytes should be the NES Tag
        if &raw[0..4] != NES_TAG {
            return Err("File is not in iNES file format".to_string());
        }

        let mapper = (raw[6] >> 4) | (raw[7] & 0xF0);

        // Detect NES 2.0
        let nes2_bits = (raw[7] >> 2) & 0b11;
        let is_nes2 = nes2_bits == 0b10;

        // Set up mirroring type
        let four_screen = raw[6] & 0b1000 != 0;
        let vertical_mirroring = raw[6] & 0b1 != 0;
        let mirroring = match(four_screen, vertical_mirroring) {
            (true, _) => Mirroring::FOURSCREEN,
            (false, true) => Mirroring::VERTICAL,
            (false, false) => Mirroring::HORIZONTAL,
        };

        let skip_trainer = raw[6] & 0b100 != 0;
        let header_size = 16 + if skip_trainer { 512 } else { 0 };

        // Calculate PRG ROM size
        let (prg_rom_size, chr_rom_size) = if is_nes2 {
            let prg_lower = raw[4] as usize;
            let prg_upper = (raw[9] & 0x0F) as usize;  
            let chr_lower = raw[5] as usize;
            let chr_upper = (raw[9] >> 4) as usize;

            let prg_pages = (prg_upper << 8) | prg_lower;
            let chr_pages = (chr_upper << 8) | chr_lower;

            (
                prg_pages * PROM_PAGE_SIZE,
                chr_pages * CROM_PAGE_SIZE
            )
        } else {
            (
                raw[4] as usize * PROM_PAGE_SIZE,
                raw[5] as usize * CROM_PAGE_SIZE
            )
        };

        let prg_end = header_size + prg_rom_size;
        let chr_end = prg_end + chr_rom_size;

        // Safety checks
        if prg_end > raw.len() {
            return Err(format!("PRG-ROM size ({prg_rom_size} bytes) exceeds file length."));
        }
        if chr_end > raw.len() {
            return Err(format!("CHR-ROM size ({chr_rom_size} bytes) exceeds file length."));
        }

        Ok(Rom {
            p_rom: raw[header_size..prg_end].to_vec(),
            c_rom: if chr_rom_size > 0 {
                raw[prg_end..chr_end].to_vec()
            } else {
                Vec::new()
            },
            mapper,
            mirroring,
        })
    }
}