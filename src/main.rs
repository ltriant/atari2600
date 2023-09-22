#[macro_use] extern crate log;
#[macro_use] extern crate lazy_static;

mod bus;
mod cpu6507;
mod debugger;
mod riot;
mod tia;

use std::cell::RefCell;
use std::env;
use std::fs::File;
use std::io::Read;
use std::rc::Rc;
use std::thread;
use std::time::{Duration, Instant};

use crate::bus::AtariBus;
use crate::cpu6507::CPU6507;
use crate::debugger::Debugger;
use crate::riot::RIOT;
use crate::tia::TIA;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};

const ATARI_FPS: f64 = 60.0;
const FRAME_DURATION: Duration = Duration::from_millis(((1.0 / ATARI_FPS) * 1000.0) as u64);
const CLOCKS_PER_SCANLINE: usize = 228;

fn main() {
    env_logger::init();

    let rom_path = env::args().skip(1).next()
        .expect("missing argument: rom file");

    let mut fh = File::open(&rom_path).expect("unable to open rom");

    let mut rom = vec![];
    let bytes = fh.read_to_end(&mut rom).expect("unable to read rom data");
    info!("ROM: {} ({} bytes)", rom_path, bytes);

    info!("RIOT: init");
    let riot = Rc::new(RefCell::new(RIOT::new()));
    riot.borrow_mut().up(false);
    riot.borrow_mut().down(false);
    riot.borrow_mut().left(false);
    riot.borrow_mut().right(false);
    riot.borrow_mut().select(false);
    riot.borrow_mut().reset(false);

    info!("TIA: init");
    let tia = Rc::new(RefCell::new(TIA::new()));
    tia.borrow_mut().joystick_fire(false);

    let bus = AtariBus::new(tia.clone(), riot.clone(), rom);

    info!("CPU: init");
    let mut cpu = CPU6507::new(Box::new(bus));
    cpu.reset();

    //
    // SDL-related stuffs
    //

    info!("Graphics: init");
    let width  = 160 * 5;
    let height = 200 * 3;

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    info!("  video driver: {}", video_subsystem.current_video_driver());

    let window = video_subsystem.window("atari2600", width, height)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas()
        .target_texture()
        .build()
        .unwrap();

    info!("  canvas driver: {}", canvas.info().name);

    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator.create_texture_streaming(PixelFormatEnum::RGB24, width, height)
        .unwrap();

    texture.with_lock(None, |buffer: &mut [u8], _pitch: usize| {
        // Initialise a black canvas
        for y in 0 .. height {
            for x in 0 .. width {
                let offset = (y * width) + x;
                buffer[offset as usize] = 0;
            }
        }
    }).unwrap();

    canvas.clear();
    canvas.copy(&texture, None, None).unwrap();
    canvas.present();

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut fps_start = Instant::now();

    let mut debugger = Rc::new(RefCell::new(Debugger::new(
        tia.clone(),
    )));

    let mut scanline = || {
        for c in 0 .. CLOCKS_PER_SCANLINE {
            if (c % 3) == 0 {
                riot.borrow_mut().clock();
            }

            tia.borrow_mut().clock();
            debugger.borrow_mut().debug();

            if !tia.borrow().cpu_halt() && (c % 3) == 2 {
                cpu.clock();
            }
        }

        return tia.borrow().get_scanline_pixels().clone();
    };

    let mut frames = 0;

    let mut vsync = 0;
    let mut vblank = 0;
    let mut visible = 0;
    let mut overscan = 0;

    let mut frame_pixels = vec![vec![Color::RGB(0, 0, 0); 160]; 200];

    'running: loop {
        if debugger.borrow().next_frame() {
            // Generate one full frame

            // VSync
            while tia.borrow().in_vsync() {
                scanline();
                vsync += 1;
            }

            // VBlank
            while tia.borrow().in_vblank() {
                scanline();
                vblank += 1;
            }

            // Picture
            let mut y = 0;
            while !tia.borrow().in_vblank() {
                let pixels = scanline();
                if y < frame_pixels.len() {
                    frame_pixels[y] = pixels;
                }
                y += 1;

                visible += 1;
            }

            // Overscan
            while !tia.borrow().in_vsync() {
                scanline();
                overscan += 1;
            }

            frames += 1;

            vsync = 0;
            vblank = 0;
            visible = 0;
            overscan = 0;

            texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
                for y in 0 .. 200 {
                    for x in 0 .. 160 {
                        let color  = frame_pixels[y][x];
                        let offset = 3 * (y * pitch) + 5 * (x * 3);

                        for y2 in 0 .. 3 {
                            let offset = offset + (y2 * pitch);

                            for x2 in 0 .. 5 {
                                let offset = offset + (x2 * 3);

                                buffer[offset]   = color.r;
                                buffer[offset+1] = color.g;
                                buffer[offset+2] = color.b;
                            }
                        }
                    }
                }
            }).unwrap();

            canvas.clear();
            canvas.copy(&texture, None, None).unwrap();
            canvas.present();

            debugger.borrow_mut().end_frame();
        }

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => { break 'running },
                Event::KeyDown { keycode: Some(key), .. } => {
                    match key {
                        // Joystick controls
                        Keycode::W => riot.borrow_mut().up(true),
                        Keycode::A => riot.borrow_mut().left(true),
                        Keycode::S => riot.borrow_mut().down(true),
                        Keycode::D => riot.borrow_mut().right(true),
                        Keycode::N => tia.borrow_mut().joystick_fire(true),

                        // Console switches
                        Keycode::F1 => riot.borrow_mut().select(true),
                        Keycode::F2 => riot.borrow_mut().reset(true),
                        Keycode::F3 => riot.borrow_mut().color(),

                        // Debugger
                        Keycode::Backquote => debugger.borrow_mut().toggle(),
                        Keycode::Space     => debugger.borrow_mut().step_frame(),

                        _ => {},
                    }
                },
                Event::KeyUp { keycode: Some(key), .. } => {
                    match key {
                        Keycode::W => riot.borrow_mut().up(false),
                        Keycode::A => riot.borrow_mut().left(false),
                        Keycode::S => riot.borrow_mut().down(false),
                        Keycode::D => riot.borrow_mut().right(false),
                        Keycode::N => tia.borrow_mut().joystick_fire(false),

                        Keycode::F1 => riot.borrow_mut().select(false),
                        Keycode::F2 => riot.borrow_mut().reset(false),

                        _ => {},
                    }
                },
                _ => { },
            }
        }

        if let Some(delay) = FRAME_DURATION.checked_sub(fps_start.elapsed()) {
            thread::sleep(delay);
        }

        fps_start = Instant::now();
    }
}
