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
                // Return 0xFF instead of 0 to match expected default read behavior
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
            0x6000 => {
                match data {
                    0x00 => {
                        println!("blargg test PASSED!");
                        std::process::exit(0); // graceful exit
                    }
                    0x80 => {
                        println!("Running")
                    }
                    fail_code => {
                        println!("blargg test FAILED with code {:02X}", fail_code);
                        // Optionally read $6004..$60XX and print failure message
                        let mut msg = Vec::new();
                        let mut addr = 0x6004;
                        loop {
                            let byte = self.mem_read(addr);
                            if byte == 0 || addr > 0x60FF { break; }
                            msg.push(byte);
                            addr += 1;
                        }
                        if let Ok(message) = String::from_utf8(msg) {
                            println!("Failure reason: {}", message);
                        }
                        std::process::exit(1);
                    }
                }
            }
            0x6004..=0x7000 => {
                // Only print printable ASCII characters, skip nulls and control chars
                if data.is_ascii_graphic() || data == b' ' {
                    print!("{}", data as char);
                } else if data == b'\n' || data == b'\r' {
                    print!("{}", data as char);
                }
                // Do not print \x00 or other non-printable bytes
            }
            0x8000..=0xFFFF => panic!("Attmempt to write to cartridge ROM Space"),
            _ => {
                //println!("Ignoring memory access at {}", addr);
            }
        }
    }
}