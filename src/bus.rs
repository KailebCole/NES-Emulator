// Provide Interconnectibility between components
//  _______________ $10000  _______________
// | PRG-ROM       |       |               |
// | Upper Bank    |       |               |
// |_ _ _ _ _ _ _ _| $C000 | PRG-ROM       |
// | PRG-ROM       |       |               |
// | Lower Bank    |       |               |
// |_______________| $8000 |_______________|
// | SRAM          |       | SRAM          |
// |_______________| $6000 |_______________|
// | Expansion ROM |       | Expansion ROM |
// |_______________| $4020 |_______________|
// | I/O Registers |       |               |
// |_ _ _ _ _ _ _ _| $4000 |               |
// | Mirrors       |       | I/O Registers |
// | $2000-$2007   |       |               |
// |_ _ _ _ _ _ _ _| $2008 |               |
// | I/O Registers |       |               |
// |_______________| $2000 |_______________|
// | Mirrors       |       |               |
// | $0000-$07FF   |       |               |
// |_ _ _ _ _ _ _ _| $0800 |               |
// | RAM           |       | RAM           |
// |_ _ _ _ _ _ _ _| $0200 |               |
// | Stack         |       |               |
// |_ _ _ _ _ _ _ _| $0100 |               |
// | Zero Page     |       |               |
// |_______________| $0000 |_______________|


use crate::{cpu::Mem, rom};

const RAM: u16 = 0x0000;
const RAM_MIRRORS_END: u16 = 0x1FFF;
const PPU_REGISTERS: u16 = 0x2000;
const PPU_REGISTERS_MIRRORS_END: u16 = 0x3FFF;

pub struct Bus {
    cpu_vram: [u8; 2048],
    rom: rom::Rom,
}

impl Bus {
    pub fn new(rom: rom::Rom) -> Self {
        Bus {
            cpu_vram: [0; 2048],
            rom: rom,
        }
    }

    fn read_prom(&self, mut addr: u16) -> u8 {
        addr -= 0x8000;

        if self.rom.p_rom.len() == 0x4000 && addr >= 0x4000 {
            // Mirror if needed
            addr = addr % 0x4000;
        }

        return self.rom.p_rom[addr as usize];
    }
}

impl Mem for Bus {
    fn mem_read(&self, addr: u16) -> u8 {
        match addr {
            RAM ..= RAM_MIRRORS_END => {
                let mirror_down_addr = addr & 0b00000111_11111111;
                return self.cpu_vram[mirror_down_addr as usize]
            }
            // APU and I/O Registers ($4000–$401F)
            0x4000..=0x401F => {
                // Return 0xFF for unimplemented APU/I/O reads
                return 0xFF;
            }

            // ROM reads ($8000–$FFFF)
            0x8000..=0xFFFF => self.read_prom(addr),

            // All other regions (PPU registers, expansion ROM)
            _ => {
                return 0xFF;
            }
        }
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        match addr {
            RAM ..= RAM_MIRRORS_END => {
                let mirror_down_addr = addr & 0b11111111111;
                self.cpu_vram[mirror_down_addr as usize] = data;
            }
            /*PPU_REGISTERS ..= PPU_REGISTERS_MIRRORS_END => {
                let _mirror_down_addr = addr & 0b00100000_00000111;
                todo!("PPU Is not supported yet")
            }*/
            0x8000..=0xFFFF => {
                // Cartridge space: treat writes as mapper register writes
                // For now, just stub them out
                // In a real emulator, you'd pass (addr, data) to your Mapper object
                // e.g., self.mapper.write(addr, data);
                // TODO: Complete Mapper Regions
            }
            _ => {}
        }
    }
}