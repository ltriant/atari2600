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

use crate::bus::AtariBus;
use crate::cpu6507::CPU6507;
use crate::tia::TIA;

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::rect::Rect;

fn main() {
    env_logger::init();

    let rom_path = env::args().skip(1).next()
        .unwrap_or(String::from("roms/kernel_01.bin"));

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

    let mut canvas = window.into_canvas().build().unwrap();

    for _ in 0 .. 2 {
        canvas.clear();
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.fill_rect(Rect::new(0, 0, width, height)).unwrap();
        canvas.present();
    }

    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => { break 'running },
                _ => { },
            }
        }

        let cpu_cycles = if tia.borrow().cpu_halt() {
            1
        } else {
            cpu.clock()
        };

        let tia_cycles = cpu_cycles * 3;

        for _ in 0 .. tia_cycles {
            let rv = tia.borrow_mut().clock(&mut canvas);

            if rv.end_of_frame {
                canvas.present();
                //std::thread::sleep(std::time::Duration::from_millis(2));
            }
        }

    }
}
