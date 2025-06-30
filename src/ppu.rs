// This module's primary goal is to draw the current state of a game on a TV Screen.

use crate::cpu;

pub struct PPU {
    pub cycles: usize,
    pub scanline: isize,
    pub frame: usize,

    // Memory
    pub vram: [u8; 0x800],
    pub palette_table: [u8; 32],
    pub oam_data: [u8; 256],

    // Registers
    pub control: u8,
    pub mask: u8,
    pub status: u8,
    pub oam_addr: u8,
    pub scroll: (u8, u8),
    pub addr: u16,
    pub addr_latch: bool,
    pub nmi_triggered: bool,
}

impl PPU {
    pub fn new() -> Self {
        PPU {
            cycles: 0,
            scanline: 0,
            frame: 0,
            vram: [0; 0x800],
            palette_table: [0; 32],
            oam_data: [0; 256],
            control: 0,
            mask: 0,
            status: 0,
            oam_addr: 0,
            scroll: (0, 0),
            addr: 0,
            addr_latch: false,
            nmi_triggered: false,
        }
    }
    pub fn step(&mut self) {
        self.cycles += 1;
        if self.cycles > 340 {
            self.cycles = 0;
            self.scanline += 1;

            if self.scanline > 261 {
                self.scanline = -1; 
                self.frame += 1;
            }
        }

        if self.scanline == 241 && self.cycles == 1 {
            self.status |= 0x80;    // Set VBlank flag
            if self.control & 0x80 != 0 { self.nmi_triggered = true; } 
        }

        if self.scanline == -1 && self.cycles == 1 {
            self.status &= 0x7F;   // Clear VBlank flag
        }
    }
    pub fn read_register(&self, addr: u16) -> u8 {
        match addr {
            0x2000 => self.control,
            0x2001 => self.mask,
            0x2002 => self.status,
            0x2003 => self.oam_addr,
            0x2004 => self.oam_data[self.oam_addr as usize],
            0x2005 => {
                if !self.addr_latch { self.scroll.0 } else { self.scroll.1 }},
            0x2007 => self.vram[self.addr as usize & 0x7FF],
            _ => 0,
        }
    }
    pub fn write_register(&mut self, addr: u16, data: u8) {
        match addr & 0x2007 {
            0x2000 => self.control = data,
            0x2001 => self.mask = data,
            0x2003 => self.oam_addr = data,
            0x2004 => self.oam_data[self.oam_addr as usize] = data,
            0x2005 => {
                if !self.addr_latch { self.scroll.0 = data; }
                else { self.scroll.1 = data; }
                self.addr_latch = !self.addr_latch;
            },
            0x2006 => {
                if !self.addr_latch { self.addr = ((data as u16) << 8) | (self.addr & 0x00FF); }
                else { self.addr = (self.addr & 0x00FF) | (data as u16); }
                self.addr_latch = !self.addr_latch;
            },
            0x2007 => {
                self.vram[self.addr as usize & 0x7FF] = data;
                self.addr = self.addr.wrapping_add(self.vram_increment());
            },
            _ => {}
        }
    }

    fn vram_increment(&self) -> u16 {
        if self.control & 0b00000100 != 0 { 32 } else { 1 }
    }
}