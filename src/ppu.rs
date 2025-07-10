// This module's primary goal is to draw the current state of a game on a TV Screen.

use sdl2::pixels::Color;

use crate::{bus::{self, Bus}, cpu, nes::{self, NES}, HEIGHT, WIDTH};

pub struct PPU {
    pub cycles: usize,
    pub scanline: isize,
    pub frame: usize,
    pub is_new_frame: bool,

    // Memory
    pub vram: [u8; 0x800],
    pub palette_table: [u8; 32],
    pub oam_data: [u8; 256],
    pub framebuffer: [u8; WIDTH * HEIGHT * 3],

    // Registers
    pub control: u8,
    pub mask: u8,
    pub status: u8,
    pub oam_addr: u8,
    pub addr: u16,
    pub addr_latch: bool,
    pub nmi_triggered: bool,
    pub buffered_read: u8,

    // Additional Registers for Scrolling
    pub vram_addr: u16,
    pub temp_addr: u16,
    pub fine_x: u8,
    pub write_toggle: bool,

    // Background Fetch Latches
    pub next_tile_id: u8,
    pub next_tile_attr: u8,
    pub next_tile_lsb: u8,
    pub next_tile_msb: u8,
    pub latch_attr: u8,

    // Background Shifters
    pub bg_pattern_shift_low: u16,
    pub bg_pattern_shift_high: u16,
    pub bg_attr_shift_low: u16,
    pub bg_attr_shift_high: u16,

}

impl PPU {
    pub fn new() -> Self {
        PPU {
            cycles: 0,
            scanline: 0,
            frame: 0,
            is_new_frame: false,
            vram: [0; 0x800],
            palette_table: [0; 32],
            oam_data: [0; 256],
            framebuffer: [0; (WIDTH * HEIGHT * 3)],
            control: 0,
            mask: 0,
            status: 0,
            oam_addr: 0,
            addr: 0,
            addr_latch: false,
            nmi_triggered: false,
            buffered_read: 0,
            vram_addr: 0,
            temp_addr: 0,
            fine_x: 0,
            write_toggle: false,
            next_tile_id: 0,
            next_tile_attr: 0,
            next_tile_lsb: 0,
            next_tile_msb: 0,
            latch_attr: 0,
            bg_pattern_shift_low: 0,
            bg_pattern_shift_high: 0,
            bg_attr_shift_low: 0,
            bg_attr_shift_high: 0,
        }
    }

    pub fn reset(&mut self) {
        self.vram_addr = 0;
        self.temp_addr = 0;
        self.write_toggle = false;
        self.addr_latch = false;
        self.scanline = 0;
        self.cycles = 0;
        self.frame = 0;
    }

    pub fn step(&mut self, bus: &Bus) {
        // Increment Cycles
        self.cycles += 1;

        // Clear Framebuffer at the start of each frame
        if self.scanline == -1 && self.cycles == 1 {
            self.framebuffer.fill(0);
            self.is_new_frame = true;
        }

        // Every 8 PPU cycles, fetch data for background rendering
        if self.scanline >= 0 && self.scanline < 240 && (self.cycles >= 1 && self.cycles <= 256) {
            // Shift background registers every cycle
            self.bg_pattern_shift_low <<= 1;
            self.bg_pattern_shift_high <<= 1;
            self.bg_attr_shift_low <<= 1;
            self.bg_attr_shift_high <<= 1;

            // Pixel X, Y coordinates
            let x = (self.cycles - 1) as usize;
            let y = self.scanline as usize;

            // Select bit from shifters using fine_x
            let bit = 0x8000 >> self.fine_x;
            let p0 = if self.bg_pattern_shift_low & bit != 0 { 1 } else { 0 };
            let p1 = if self.bg_pattern_shift_high & bit != 0 { 1 } else { 0 };
            let palette_low = if self.bg_attr_shift_low & bit != 0 { 1 } else { 0 };
            let palette_high = if self.bg_attr_shift_high & bit != 0 { 1 } else { 0 };

            let palette_index = (palette_high << 2) | (palette_low << 1) | (p1 << 1) | p0;

            // Store Pixel to Framebuffer
            // TODO: Update Colors
            if x < WIDTH && y < HEIGHT {
                let offset = (y * WIDTH + x) * 3;
                if palette_index != 0 {
                    println!("Drawing pixel at ({}, {}) with color index {}", x, y, palette_index);
                    self.framebuffer[offset] = 0xFF;       // R
                    self.framebuffer[offset + 1] = 0;      // G
                    self.framebuffer[offset + 2] = 0;      // B
                }
            }

            match self.cycles % 8 {
                0 => {  
                    let addr = 0x2000 | (self.vram_addr & 0x0FFF);
                    self.next_tile_id = self.read_vram(bus, addr);
                }
                1 => {  
                    let attr_addr = 0x23C0
                        | (self.vram_addr & 0x0C00)
                        | ((self.vram_addr >> 4) & 0x38)
                        | ((self.vram_addr >> 2) & 0x07);
                    self.next_tile_attr = self.read_vram(bus, attr_addr);
                }
                3 => {
                    let fine_y = (self.vram_addr >> 12) & 0x7;
                    let table = if self.control & 0x10 != 0 { 0x1000 } else { 0 };
                    let pattern_addr = table + (self.next_tile_id as u16 * 16) + fine_y;
                    let lsb = self.read_vram(bus, pattern_addr);
                    let msb = self.read_vram(bus, pattern_addr + 8);
                    println!("Tile ID: {:02X}, LSB: {:02X}, MSB: {:02X}", self.next_tile_id, lsb, msb);
                    self.next_tile_lsb = lsb;
                    self.next_tile_msb = msb;
                }
                5 => {  
                    let fine_y = (self.vram_addr >> 12) & 0x7;
                    let table = if self.control & 0x10 != 0 { 0x1000 } else { 0 };
                    let pattern_addr = table + (self.next_tile_id as u16 * 16) + fine_y + 8;
                    let lsb = self.read_vram(bus, pattern_addr);
                    let msb = self.read_vram(bus, pattern_addr + 8);
                    println!("Tile ID: {:02X}, LSB: {:02X}, MSB: {:02X}", self.next_tile_id, lsb, msb);
                    self.next_tile_lsb = lsb;
                    self.next_tile_msb = msb;
                }
                7 => {  
                    // Reload Shifters
                    self.bg_pattern_shift_low = (self.bg_pattern_shift_low & 0xFF00) | (self.next_tile_lsb as u16);
                    self.bg_pattern_shift_high = (self.bg_pattern_shift_high & 0xFF00) | (self.next_tile_msb as u16);

                    // Attribute shift reload
                    let coarse_x = (self.vram_addr >> 0) & 0x1F;
                    let coarse_y = (self.vram_addr >> 5) & 0x1F;
                    let quadrant = ((coarse_y >> 1) & 0x02) | ((coarse_x >> 1) & 0x01);
                    let attr_bits = match quadrant {
                        0 => self.next_tile_attr & 0b11,
                        1 => (self.next_tile_attr >> 2) & 0b11,
                        2 => (self.next_tile_attr >> 4) & 0b11,
                        3 => (self.next_tile_attr >> 6) & 0b11,
                        _ => 0,
                    };

                    self.bg_attr_shift_low = (self.bg_attr_shift_low & 0xFF00) | if attr_bits & 0x01 != 0 { 0xFF } else { 0 };
                    self.bg_attr_shift_high = (self.bg_attr_shift_high & 0xFF00) | if attr_bits & 0x02 != 0 { 0xFF } else { 0 };
                }
                _ => {}
            }

            if self.cycles % 8 == 0 {
                self.increment_x();
            }

            if self.cycles == 256 {
                self.increment_y();
            }

            if self.cycles == 257 {
                self.transfer_horizontal();
            }
        }

        // Finish scanline
        if self.cycles > 340 {
            self.cycles = 0;
            self.scanline += 1;

            if self.scanline > 261 {
                self.scanline = -1;
                self.frame += 1;
            }
        }

        // VBlank begin
        if self.scanline == 241 && self.cycles == 1 {
            self.status |= 0x80;
            if self.control & 0x80 != 0 {
                self.nmi_triggered = true;
            }
        }

        // VBlank end
        if self.scanline == -1 && self.cycles == 1 {
            self.status &= 0x7F;
        }
    }

    fn read_vram(&self, bus: &Bus, addr: u16) -> u8 {
        match addr {
            0x000..=0x1FFF => bus.read_chr(addr),
            0x2000..=0x2FFF => self.vram[(addr - 0x2000) as usize & 0x7FF],
            0x3000..=0x3EFF => self.vram[(addr - 0x3000) as usize & 0x7FF],
            0x3F00..=0x3FFF => self.palette_table[(addr - 0x3F00) as usize & 0x1F],
            _ => 0,
        }
    }

    pub fn read_register(&mut self, bus: &Bus, addr: u16) -> u8 {
        match addr {
            0x2000 => self.control,
            0x2001 => self.mask,
            0x2002 => {
                let val = self.status;
                self.status &= !0x80;
                self.write_toggle = false;
                val
            }
            0x2003 => self.oam_addr,
            0x2004 => self.oam_data[self.oam_addr as usize],
            0x2007 => {
                let addr = self.addr & 0x3FFF;
                let value = self.read_vram(bus, addr);
                self.addr = self.addr.wrapping_add(self.vram_increment());

                if addr >= 0x3F00 {
                    value
                } 
                else {
                    let ret = self.buffered_read;
                    self.buffered_read = value;
                    ret
                }
            }
            _ => 0,
        }
    }

    pub fn write_register(&mut self, bus: &mut Bus, addr: u16, data: u8) {
        match addr & 0x2007 {
            0x2000 => self.control = data,
            0x2001 => self.mask = data,
            0x2003 => self.oam_addr = data,
            0x2004 => self.oam_data[self.oam_addr as usize] = data,
            0x2005 => {
                if !self.write_toggle {
                    self.fine_x = data & 0x07;
                    self.temp_addr = (self.temp_addr & 0xFFE0) | ((data as u16) >> 3);
                } else {
                    self.temp_addr = (self.temp_addr & 0x8FFF) | (((data as u16) & 0x07) << 12);
                    self.temp_addr = (self.temp_addr & 0xFC1F) | (((data as u16) & 0xF8) << 2);
                }
                self.write_toggle = !self.write_toggle;
            },
            0x2006 => {
                if !self.write_toggle {
                    self.temp_addr = (self.temp_addr & 0x00FF) | (((data & 0x3F) as u16) << 8);
                } else {
                    self.temp_addr = (self.temp_addr & 0xFF00) | (data as u16);
                    self.vram_addr = self.temp_addr;
                }
                self.write_toggle = !self.write_toggle;
            },
            0x2007 => {
                let addr = self.addr & 0x3FFF;
                if addr < 0x2000 {
                    bus.write_chr(addr, data);
                }
                else {
                    self.vram[(addr - 0x2000) as usize & 0x7FF] = data;
                }
                self.addr = self.addr.wrapping_add(self.vram_increment());
            },
            _ => {}
        }
    }

    fn vram_increment(&self) -> u16 {
        if self.control & 0x04 != 0 { 0x20 } else { 0x01 }
    }

    fn increment_x(&mut self) {
        if (self.vram_addr & 0x001F) == 31 {
            self.vram_addr &= !0x001F;           
            self.vram_addr ^= 0x0400;            
        } else {
            self.vram_addr += 1;                 
        }
    }

    fn increment_y(&mut self) {
        if (self.vram_addr & 0x7000) != 0x7000 {
            self.vram_addr += 0x1000;                           
        } else {
            self.vram_addr &= !0x7000;                         
            let mut y = (self.vram_addr & 0x03E0) >> 5;     
            if y == 29 {
                y = 0;
                self.vram_addr ^= 0x0800;                   
            } else if y == 31 {
                y = 0;                                       
            } else {
                y += 1;
            }
            self.vram_addr = (self.vram_addr & !0x03E0) | (y << 5);
        }
    }

    fn transfer_horizontal(&mut self) {
        self.vram_addr = (self.vram_addr & 0x7BE0) | (self.temp_addr & 0x041F);
    }
}

// Return a Color based on a bytye
fn color(byte: u8) -> Color {
    match byte {
        0 => sdl2::pixels::Color::BLACK,
        1 => sdl2::pixels::Color::WHITE,
        2 | 9 => sdl2::pixels::Color::GREY,
        3 | 10 => sdl2::pixels::Color::RED,
        4 | 11 => sdl2::pixels::Color::GREEN,
        5 | 12 => sdl2::pixels::Color::BLUE,
        6 | 13 => sdl2::pixels::Color::MAGENTA,
        7 | 14 => sdl2::pixels::Color::YELLOW,
        _ => sdl2::pixels::Color::CYAN,
    }
}