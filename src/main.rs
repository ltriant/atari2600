#[macro_use] extern crate log;
#[macro_use] extern crate lazy_static;

mod bus;
mod cpu6507;
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
use crate::tia::TIA;

use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::event::Event;
use sdl2::rect::Rect;

const ATARI_FPS: f64 = 60.0;
const FRAME_DURATION: Duration = Duration::from_millis(((1.0 / ATARI_FPS) * 1000.0) as u64);

fn main() {
    env_logger::init();

    let rom_path = env::args().skip(1).next()
        .expect("missing argument: rom file");

    let mut fh = File::open(rom_path).expect("unable to open rom");

    let mut rom = vec![];
    let bytes = fh.read_to_end(&mut rom).expect("unable to read rom data");
    info!("read {} bytes of ROM data", bytes);

    let tia = Rc::new(RefCell::new(TIA::new_tia()));
    let bus = AtariBus::new_bus(tia.clone(), rom);
    let mut cpu = CPU6507::new_cpu(Box::new(bus));
    cpu.reset();

    //
    // SDL-related stuffs
    //

    let width  = 160 * 5;
    let height = 192 * 3;

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("atari2600", width, height)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas()
        .target_texture()
        .build()
        .unwrap();

    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator.create_texture_streaming(PixelFormatEnum::RGB24, width, height)
        .unwrap();

    for _ in 0 .. 2 {
        canvas.clear();
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.fill_rect(Rect::new(0, 0, width, height)).unwrap();
        canvas.present();
    }

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut end_of_frame = false;
    let mut fps_start = Instant::now();

    'running: loop {
        let tia_cycles = if tia.borrow().cpu_halt() {
            1
        } else {
            3 * cpu.clock()
        };

        for _ in 0 .. tia_cycles {
            let rv = tia.borrow_mut().clock();

            if rv.end_of_frame {
                end_of_frame = true;
            }
        }

        if end_of_frame {
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => { break 'running },
                    _ => { },
                }
            }

            if let Some(delay) = FRAME_DURATION.checked_sub(fps_start.elapsed()) {
                thread::sleep(delay);
            }

            fps_start = Instant::now();

            let tia    = tia.borrow();
            let pixels = tia.get_pixels();

            texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
                for y in 0 .. 192 {
                    for x in 0 .. 160 {
                        let color  = pixels[y][x];
                        let offset = 3*(y*pitch) + 5*(x*3);

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

            end_of_frame = false;
        }
    }
}
