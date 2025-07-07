use crate::{bus::Bus, cpu::CPU, ppu::PPU, rom::Rom};

pub struct NES {
    pub cpu: CPU,
    pub ppu: PPU,
    pub bus: Bus,
}

impl NES {
    pub fn new(rom: Rom) -> Self {        
        Self {
            bus: Bus::new(rom),
            cpu: CPU::new(),
            ppu: PPU::new(),
        }
    }

    pub fn reset(&mut self) {
        let pc_start = self.bus.mem_read_16(0xFFFC);
        self.cpu.reset(pc_start);
        self.ppu.reset();
    }

    pub fn step(&mut self) {
        self.cpu.step(&mut self.bus, &mut self.ppu);
    }
}