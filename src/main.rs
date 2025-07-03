#![cfg_attr(debug_assertions, allow(dead_code))]
#![cfg_attr(debug_assertions, allow(unused_imports))]

pub mod apu;
pub mod bus;
pub mod rom;
pub mod cpu;
pub mod gamepad;
pub mod opcodes;
pub mod ppu;
pub mod trace;

use bus::Bus;
use cpu::CPU;
use cpu::Mem;
use rand::Rng;
use rom::Rom;
use ppu::PPU;

use sdl2::event::Event;
use sdl2::EventPump;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use std::io::Write;
use std::time::Instant;

#[macro_use]
extern crate lazy_static;

const WIDTH: usize = 256;
const HEIGHT: usize = 240;

fn main() {
    // Init SDL2
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("NES Test", WIDTH as u32, HEIGHT as u32)
        .position_centered()
        .build().unwrap();
    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(10.0, 10.0).unwrap();

    // Render Texture
    let creator = canvas.texture_creator();
    let mut texture = creator.create_texture_target(PixelFormatEnum::RGB24, WIDTH as u32, HEIGHT as u32).unwrap();

    // Load Game
    let bytes: Vec<u8> = std::fs::read("color_test.nes").unwrap();
    let rom = rom::Rom::new(&bytes).unwrap();

    let ppu = Rc::new(RefCell::new(PPU::new()));
    let bus = bus::Bus::new(ppu.clone(), rom);
    let mut cpu = cpu::CPU::new(bus);

    // Main Loop
    cpu.reset();
    let frame_time = Duration::from_millis(16); // 60 FPS

    loop {
        let start = Instant::now();

        // Handle events
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    ::std::process::exit(0);
                }
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    ::std::process::exit(0);
                }
                _ => {}
            }
        }

        // Set up frame count
        let frame_deadline = Instant::now() + frame_time;
        
        // Step CPU n times, can be corrected with a timer later
        while !ppu.borrow().is_new_frame && Instant::now() < frame_deadline {
            cpu.step();
            if cpu.bus.ppu.borrow().nmi_triggered {
                cpu.trigger_nmi();
                cpu.bus.ppu.borrow_mut().nmi_triggered = false;
            }
        }

        // On New Frame, Update SDL graphics
        if ppu.borrow().is_new_frame {
            texture.update(None, &ppu.borrow().framebuffer, WIDTH * 3).unwrap();
            canvas.copy(&texture, None, None).unwrap();
            canvas.present();
            ppu.borrow_mut().is_new_frame = false;
        }

        // Sleep to maintain frame rate
        let elapsed_time = start.elapsed();
        if elapsed_time < frame_time {
            ::std::thread::sleep(frame_time - elapsed_time);
        }
    }
}