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

use sdl2::event::Event;
use sdl2::EventPump;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use std::time::Duration;
use std::io::Write;

#[macro_use]
extern crate lazy_static;

const WIDTH: u32 = 32;
const HEIGHT: u32 = 32;

fn main() {

    // Init SDL2
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("NES Test", WIDTH * 10 as u32, HEIGHT * 10 as u32)
        .position_centered()
        .build().unwrap();
    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(10.0, 10.0).unwrap();

    // Render Texture
    let creator = canvas.texture_creator();
    let mut texture = creator.create_texture_target(PixelFormatEnum::RGB24, WIDTH, HEIGHT).unwrap();

    // Load Game
    let bytes: Vec<u8> = std::fs::read("nestest.nes").unwrap();
    let rom = rom::Rom::new(&bytes).unwrap();

    let bus = bus::Bus::new(rom);
    let mut cpu = cpu::CPU::new(bus);
    cpu.reset();
    cpu.register_pc = 0xC000;

    let mut screen_state = [0 as u8; (WIDTH * 3 * HEIGHT) as usize];
    //let mut rng = rand::thread_rng();

    cpu.run_with_callback(move |cpu| {
        // Open the file in append mode, or create it if it doesn't exist
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("log.txt")
            .unwrap();
        writeln!(file, "{}", trace::trace(cpu)).unwrap();

        if cpu.register_pc == 0xC66E {
            println!("Test completed: PC reached 0xC66E, exiting emulation.");
            return false;
        }

        true

        /*handle_user_input(cpu, &mut event_pump);
        cpu.mem_write(0xFE, rng.gen_range(1..16));

        if read_screen_state(cpu, &mut screen_state) {
            texture.update(None, &screen_state, 32 * 3).unwrap();
            canvas.copy(&texture, None, None).unwrap();
            canvas.present();
        }*/

        //::std::thread::sleep(std::time::Duration::new(0, 70_000));
    });
    
}

// Wait for user input and provide it to sdl2
fn handle_user_input(cpu: &mut cpu::CPU, event_pump: &mut EventPump) {
    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => { std::process::exit(0) },
            Event::KeyDown { keycode: Some(Keycode::W), .. } => {  cpu.mem_write(0xFF, 0x77) },
            Event::KeyDown { keycode: Some(Keycode::A), .. } => {  cpu.mem_write(0xFF, 0x61) },
            Event::KeyDown { keycode: Some(Keycode::S), .. } => {  cpu.mem_write(0xFF, 0x73) },
            Event::KeyDown { keycode: Some(Keycode::D), .. } => {  cpu.mem_write(0xFF, 0x64) },
            _ => (),
        }
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

// Check whether the screen should be updated
fn read_screen_state(cpu: &cpu::CPU, frame: &mut [u8; 32*3*32]) -> bool {
    let mut frame_idx = 0;
    let mut update = false;
    for i in 0x200..0x600 {
        let color_idx = cpu.mem_read(i as u16);
        let (b1, b2, b3) = color(color_idx).rgb();
        if frame[frame_idx] != b1 || frame[frame_idx + 1] != b2 || frame[frame_idx + 2] != b3 {
            frame[frame_idx] = b1;
            frame[frame_idx + 1] = b2;
            frame[frame_idx + 2] = b3;
            update = true;
        }
        frame_idx += 3;
    }

    return update;
}