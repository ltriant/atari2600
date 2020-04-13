mod palette;

use crate::bus::Bus;
use crate::tia::palette::NTSC_PALETTE;

use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

pub struct TIA {
    dot: u16,
    scanline: u16,

    // Vertical sync
    vsync: bool,
    vblank: bool,

    // Horizontal sync
    wsync: bool,

    // Colors
    colup0: u8,
    colup1: u8,
    colupf: u8,
    colubk: u8,

    // 20-bit playfield
    // .... | .... .... | .... ....
    // PF0  |    PF1    |    PF2
    pf0: u8,
    pf1: u8,
    pf2: u8,
    pf: [bool; 80],
    pf_horizontal_mirror: bool,
}

pub struct StepResult {
    pub end_of_frame: bool,
}

impl TIA {
    pub fn new_tia() -> Self {
        Self {
            dot: 0,
            scanline: 0,

            vsync: false,
            vblank: false,
            wsync: false,

            colup0: 0,
            colup1: 0,
            colupf: 0,
            colubk: 0,

            pf: [false; 80],
            pf0: 0,
            pf1: 0,
            pf2: 0,
            pf_horizontal_mirror: false,
        }
    }

    pub fn cpu_halt(&self) -> bool { self.wsync }

    fn tick(&mut self) {
        self.dot += 1;
        if self.dot == 228 {
            self.scanline += 1;
            self.dot = 0;

            if self.scanline >= 262 {
                self.scanline = 0;
            }
        }
    }

    fn update_playfield(&mut self) {
        // The playfield is a 20-bit set of dots, where each dot makes up four
        // pixels on the screen.

        // PF0 is the first 4 bits, in big-endian order
        for x in 0 .. 4 {
            let v = (self.pf0 >> (x + 4)) & 0x01 != 0;
            self.pf[x * 4] = v;
            self.pf[x * 4 + 1] = v;
            self.pf[x * 4 + 2] = v;
            self.pf[x * 4 + 3] = v;
        }

        // PF1 is the next 8 bits, in little-endian order
        for x in 0 .. 8 {
            let v = (self.pf1 >> (7 - x)) & 0x01 != 0;
            self.pf[x * 4 + 16] = v;
            self.pf[x * 4 + 16 + 1] = v;
            self.pf[x * 4 + 16 + 2] = v;
            self.pf[x * 4 + 16 + 3] = v;
        }

        // PF2 is the last 8 bits, in big-endian order
        for x in 0 .. 8 {
            let v = (self.pf2 >> x) & 0x01 != 0;
            self.pf[x * 4 + 48] = v;
            self.pf[x * 4 + 48 + 1] = v;
            self.pf[x * 4 + 48 + 2] = v;
            self.pf[x * 4 + 48 + 3] = v;
        }
    }

    pub fn clock(&mut self, canvas: &mut Canvas<Window>) -> StepResult {
        // https://www.randomterrain.com/atari-2600-memories-tutorial-andrew-davie-08.html
        //
        // There are 262 scanlines per frame
        //   3 vertical sync scanlines
        //   37 vertical blanking scanlines
        //   192 visible scanlines
        //   30 overscan scanlines
        //
        // Each scanline has 228 dots
        //   68 horizontal blanking dots
        //   160 visible dots

        let mut rv = StepResult {
            end_of_frame: false,
        };

        let visible_scanline = self.scanline >= 40 && self.scanline < 232;
        let visible_cycle = self.dot >= 68 && self.dot < 228;

        if visible_scanline && visible_cycle {
            let x = self.dot - 68;
            let y = self.scanline - 40;

            let rect = Rect::new(
                (x as i32) * 5,
                (y as i32) * 3,
                5,
                3
            );

            let color = if x < 80 {
                // The playfield makes up the left-most side of the screen.
                if self.pf[x as usize] {
                    self.colupf
                } else {
                    self.colubk
                }
            } else {
                // The playfield also makes up the right-most side of the
                // screen, optionally mirrored horizontally as denoted by the
                // CTRLPF register.

                let idx = if self.pf_horizontal_mirror {
                    159 - x as usize
                } else {
                    79 - (159 - x as usize)
                };

                if self.pf[idx] {
                    self.colupf
                } else {
                    self.colubk
                }
            };

            canvas.set_draw_color(NTSC_PALETTE[color as usize]);
            canvas.fill_rect(rect).unwrap();
        }

        if self.scanline == 0 && self.dot == 0 {
            self.vsync = false;
        }

        if self.scanline == 3 && self.dot == 0 {
            // VBlank start
            rv.end_of_frame = true;
        }

        if self.scanline == 40 && self.dot == 0 {
            // VBlank end
        }

        if self.dot == 0 {
            // HBlank start
        }

        if self.dot == 67 {
            // HBlank end
        }

        if self.dot == 227 {
            // Simply writing to the WSYNC causes the microprocessor to halt
            // until the electron beam reaches the right edge of the screen.
            self.wsync = false;
        }

        self.tick();

        rv
    }
}

impl Bus for TIA {
    // https://problemkaputt.de/2k6specs.htm#memoryandiomap

    fn read(&mut self, address: u16) -> u8 {
        0
    }

    fn write(&mut self, address: u16, val: u8) {
        match address {
            //
            // Frame timing and synchronisation
            //

            // VSYNC   ......1.  vertical sync set-clear
            0x0000 => {
                self.vsync = (val & 0x02) != 0;

                // TODO this feels hacky
                if self.vsync {
                    self.dot = 0;
                    self.scanline = 0;
                }
            },

            // VBLANK  11....1.  vertical blank set-clear
            0x0001 => {
                self.vblank = (val & 0x02) != 0;
            },

            // WSYNC   <strobe>  wait for leading edge of horizontal blank
            0x0002 => { self.wsync = true },

            // RSYNC   <strobe>  reset horizontal sync counter
            0x0003 => { },

            //
            // Missile sizes
            //

            // NUSIZ0  ..111111  number-size player-missile 0
            0x0004 => { },

            // NUSIZ1  ..111111  number-size player-missile 1
            0x0005 => { },

            //
            // Colors
            //

            // COLUP0  1111111.  color-lum player 0 and missile 0
            0x0006 => { self.colup0 = val & 0xfe },

            // COLUP1  1111111.  color-lum player 1 and missile 1
            0x0007 => { self.colup1 = val & 0xfe },

            // COLUPF  1111111.  color-lum playfield and ball
            0x0008 => { self.colupf = val & 0xfe },

            // COLUBK  1111111.  color-lum background
            0x0009 => { self.colubk = val & 0xfe },

            // CTRLPF  ..11.111  control playfield ball size & collisions
            0x000a => {
                self.pf_horizontal_mirror = (val & 0x01) != 0;
                // TODO the other bits
            },

            //
            // Playfield
            //

            // PF0     1111....  playfield register byte 0
            0x000d => {
                debug!("pf0: {:08b}", val);
                self.pf0 = val;
                self.update_playfield();
            },

            // PF1     11111111  playfield register byte 1
            0x000e => {
                debug!("pf1: {:08b}", val);
                self.pf1 = val;
                self.update_playfield();
            },

            // PF2     11111111  playfield register byte 2
            0x000f => {
                debug!("pf2: {:08b}", val);
                self.pf2 = val;
                self.update_playfield();
            },

            // TODO the rest of the registers...

            _ => { }, // unimplemented!("register: 0x{:04X}", address),
        }
    }
}
