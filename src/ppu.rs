// This module's primary goal is to draw the current state of a game on a TV Screen.

use sdl2::pixels::Color;

use crate::{cpu, WIDTH, HEIGHT};

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
    pub scroll: (u8, u8),
    pub addr: u16,
    pub addr_latch: bool,
    pub nmi_triggered: bool,

    // Additional Registers for Scrolling
    pub  vram_addr: u16,
    pub temp_addr: u16,
    pub fine_x: u8,
    pub write_toggle: bool,

    // Background Fetch Latches
    pub next_tile_id: u8,
    pub next_tile_attr: u8,
    pub next_tile_lsb: u8,
    pub next_tile_msb: u8,
    
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
            scroll: (0, 0),
            addr: 0,
            addr_latch: false,
            nmi_triggered: false,
            vram_addr: 0,
            temp_addr: 0,
            fine_x: 0,
            write_toggle: false,
            next_tile_id: 0,
            next_tile_attr: 0,
            next_tile_lsb: 0,
            next_tile_msb: 0,
        }
    }

    pub fn step(&mut self) {
        // Increment Cycles
        self.cycles += 1;

        // Clear Framebuffer at the start of each frame
        if self.scanline == -1 && self.cycles == 1 {
            self.framebuffer.fill(0);
            self.is_new_frame = true;
        }

        // Every 8 PPU cycles, fetch data for background rendering
        if self.scanline >= 0 && self.scanline < 240 && (self.cycles >= 1 && self.cycles <= 256) {
            let cycle_in_tile = (self.cycles - 1) % 8;

            match cycle_in_tile {
                1 => { // Fetch tile ID
                    let nametable_addr = 0x2000 | (self.vram_addr & 0x0FFF);
                    self.next_tile_id = self.vram[nametable_addr as usize & 0x7FF];
                }
                3 => { // Fetch attribute byte
                    let attr_addr = 0x23C0 | (self.vram_addr & 0x0C00) | ((self.vram_addr >> 4) & 0x38) | ((self.vram_addr >> 2) & 0x07);
                    self.next_tile_attr = self.vram[attr_addr as usize & 0x7FF];
                }
                5 => { // Fetch low byte of pattern
                    let fine_y = (self.vram_addr >> 12) & 0x7;
                    let pattern_table_addr = ((self.control as u16 & 0x10) << 8) + (self.next_tile_id as u16 * 16) + fine_y;
                    self.next_tile_lsb = self.vram[pattern_table_addr as usize & 0x7FF];
                }
                7 => { // Fetch high byte of pattern
                    let fine_y = (self.vram_addr >> 12) & 0x7;
                    let pattern_table_addr = ((self.control as u16 & 0x10) << 8) + (self.next_tile_id as u16 * 16) + fine_y + 8;
                    self.next_tile_msb = self.vram[pattern_table_addr as usize & 0x7FF];
                }
                0 => { // Tile data shift: render one pixel column for current tile
                    let fine_x = self.fine_x as usize;

                    for bit in 0..8 {
                        let bit_index = 7 - bit;
                        let plane0 = (self.next_tile_lsb >> bit_index) & 1;
                        let plane1 = (self.next_tile_msb >> bit_index) & 1;
                        let color_idx = (plane1 << 1) | plane0;

                        let cycle_base = if self.cycles >= 8 { self.cycles - 8 } else { 0 };
                        let x = (cycle_base + bit) as usize;
                        let y = self.scanline as usize;

                        if x < WIDTH && y < HEIGHT {
                            let offset = (y * WIDTH + x) * 3;

                            // Force any non-zero color_idx to bright color
                            if color_idx != 0 {
                                self.framebuffer[offset] = 0xFF;          // R
                                self.framebuffer[offset + 1] = 0x00;      // G
                                self.framebuffer[offset + 2] = 0x00;      // B
                            }
                        }
                    }

                    self.increment_x();
                }
                _ => {}
            }

            // Increment X position
            if self.cycles == 256 {
                self.vram_addr = (self.vram_addr & 0xFBE0) | ((self.vram_addr + 1) & 0x041F);
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

        // Increments Y at the end of each scanline
        if (self.scanline >= 0 && self.scanline < 240) || self.scanline == -1 {
            if self.cycles == 256 {
                self.increment_y();
            }
        }

        // During pre-render or visible lines, reload horizontal bits at certain cycles
        if self.scanline == -1 || (self.scanline >= 0 && self.scanline < 240) {
            if self.cycles == 257 {
                self.transfer_horizontal();
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

    pub fn read_register(&self, addr: u16) -> u8 {
        match addr {
            0x2000 => self.control,
            0x2001 => self.mask,
            0x2002 => self.status,
            0x2003 => self.oam_addr,
            0x2004 => self.oam_data[self.oam_addr as usize],
            0x2005 => { if !self.addr_latch { self.scroll.0 } else { self.scroll.1 } },
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
                self.vram[self.addr as usize & 0x7FF] = data;
                self.addr = self.addr.wrapping_add(self.vram_increment());
            },
            _ => {}
        }
    }

    fn vram_increment(&self) -> u16 {
        if self.control & 0b00000100 != 0 { 32 } else { 1 }
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