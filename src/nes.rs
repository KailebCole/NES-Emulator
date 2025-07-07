use crate::{cpu::CPU, ppu::PPU, rom::Rom};

pub struct NES {
    pub cpu: CPU,
    pub ppu: PPU,
    pub rom: Rom,

    ram: [u8; 0x800],
}

impl NES {
    pub fn new(rom: Rom) -> Self {        
        Self {
            cpu: CPU::new(),
            ppu: PPU::new(),
            rom,
            ram: [0; 0x800],
        }
    }

    pub fn reset(&mut self) {
        self.cpu.reset(self.mem_read_16(0xFFFC));
    }

    pub fn step(&mut self) {
        self.cpu.step(self);
    }

    pub fn mem_read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.ram[(addr as usize) & 0x7FF],
            0x2000..=0x3FFF => self.ppu.read_register(0x2000 + (addr & 0x7)),
            0x8000..=0xFFFF => self.read_prg_rom(addr),
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

    pub fn step_ppu(&mut self) { self.ppu.step(self); }

    fn read_prg_rom(&self, addr: u16) -> u8 {
        let mut addr = addr - 0x8000;
        if self.rom.p_rom.len() == 0x4000 && addr >= 0x4000 {
            addr %= 0x4000;
        }
        self.rom.p_rom[addr as usize]
    }
}