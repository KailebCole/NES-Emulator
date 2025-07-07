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


use core::panic;

use crate::{rom, ppu::PPU};

const RAM: u16 = 0x0000;
const RAM_MIRRORS_END: u16 = 0x1FFF;
const PPU_REGISTERS: u16 = 0x2000;
const PPU_REGISTERS_MIRRORS_END: u16 = 0x3FFF;

pub struct Bus {
    pub ram: [u8; 0x800],
    rom: rom::Rom,
}

impl Bus {
    pub fn new(rom: rom::Rom) -> Self {
        Bus {
            ram: [0; 0x800],
            rom,
        }
    }

    pub fn mem_read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.ram[(addr as usize) & 0x7FF],
            0x2000..=0x3FFF => self.ppu.read_register(0x2000 + (addr & 0x7)),
            0x8000..=0xFFFF => self.read_prom(addr),
            _ => 0xFF,
        }
    }

    pub fn mem_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=0x1FFF => self.ram[(addr as usize) & 0x7FF] = data,
            0x2000..=0x3FFF => self.ppu.write_register(0x2000 + (addr & 0x7), data),
            0x8000..=0xFFFF => panic!("Attempted to write to ROM at {:04X}", addr),
            _ => {},
        }
    }

    pub fn mem_read_16(&self, addr: u16) -> u16 {
        let lo = self.mem_read(addr) as u16;
        let hi = self.mem_read(addr + 1) as u16;
        (hi << 8) | lo
    }

    fn read_prom(&self, addr: u16) -> u8 {
        let mut addr = addr - 0x8000;
        if self.rom.p_rom.len() == 0x4000 && addr >= 0x4000 {
            addr %= 0x4000;
        }
        self.rom.p_rom[addr as usize]
    }
}