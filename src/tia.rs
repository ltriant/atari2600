mod ball;
mod color;
mod counter;
mod palette;
mod playfield;

use std::rc::Rc;
use std::cell::RefCell;

use crate::bus::Bus;
use crate::tia::ball::Ball;
use crate::tia::color::Colors;
use crate::tia::counter::Counter;
use crate::tia::palette::NTSC_PALETTE;
use crate::tia::playfield::Playfield;

use sdl2::pixels::Color;

pub struct TIA {
    scanline: u16,

    ctr: Counter,

    // Vertical sync
    vsync: bool,
    vblank: bool,
    late_reset_hblank: usize,

    // Horizontal sync
    wsync: bool,

    colors: Rc<RefCell<Colors>>,
    pf: Playfield,

    // Player 0
    grp0: u8,
    refp0: bool,
    p0_x: usize,
    hmp0: usize,

    // Player 1
    grp1: u8,
    refp1: bool,
    p1_x: usize,
    hmp1: usize,

    // Missile 0
    m0_x: usize,
    enam0: bool,
    hmm0: usize,
    m0_size: usize,

    // Missile 1
    m1_x: usize,
    enam1: bool,
    hmm1: usize,
    m1_size: usize,

    // Ball
    bl: Ball,

    // Counters
    p0_ctr: usize,
    p1_ctr: usize,
    m0_ctr: usize,
    m1_ctr: usize,

    pixels: Vec<Vec<Color>>,
}

pub struct StepResult {
    pub end_of_frame: bool,
}

impl TIA {
    pub fn new_tia() -> Self {
        let colors = Rc::new(RefCell::new(Colors::new_colors()));
        let pf = Playfield::new_playfield(colors.clone());
        let bl = Ball::new_ball(colors.clone());

        Self {
            scanline: 0,

            // The horizontal sync counter has a period of 57
            ctr: Counter::new_counter(57),

            vsync: false,
            vblank: false,
            wsync: false,
            late_reset_hblank: 0,

            colors: colors,
            pf: pf,
            bl: bl,

            grp0: 0,
            grp1: 0,
            refp0: false,
            refp1: false,
            p0_x: 0,
            p1_x: 0,

            m0_x: 0,
            m1_x: 0,
            enam0: false,
            enam1: false,
            m0_size: 0,
            m1_size: 0,

            hmp0: 0,
            hmp1: 0,
            hmm0: 0,
            hmm1: 0,

            p0_ctr: 0,
            p1_ctr: 0,
            m0_ctr: 0,
            m1_ctr: 0,

            pixels: vec![vec![Color::RGB(0, 0, 0); 160]; 192],
        }
    }

    pub fn cpu_halt(&self) -> bool { self.wsync }

    fn in_hblank(&self) -> bool {
        self.ctr.internal_value < (68 + self.late_reset_hblank as u8)
    }

    pub fn get_pixels(&self) -> &Vec<Vec<Color>> { &self.pixels }

    fn tick(&mut self) {
        // If we hit the last scanline, we have to wait for the programmer to
        // signal a VSYNC to reset the gun.
        if self.scanline >= 262 {
            return;
        }

        self.ctr.internal_value += 1;
        if self.ctr.internal_value == 228 {
            self.scanline += 1;
            self.ctr.internal_value = 0;
        }
    }

    fn get_m0_color(&self, x: usize) -> Option<u8> {
        if x >= self.m0_x && x < self.m0_x + self.m0_size && self.enam0 {
            Some(self.colors.borrow().colup0())
        } else {
            None
        }
    }

    fn get_m1_color(&self, x: usize) -> Option<u8> {
        if x >= self.m1_x && x < self.m1_x + self.m1_size && self.enam1 {
            Some(self.colors.borrow().colup1())
        } else {
            None
        }
    }

    fn get_p0_color(&self, x: usize) -> Option<u8> {
        if x >= self.p0_x && x < self.p0_x + 8 {
            let x = x - self.p0_x;

            if self.refp0 {
                if (self.grp0 & (1 << x)) != 0 {
                    return Some(self.colors.borrow().colup0());
                }
            } else {
                if (self.grp0 & (1 << (7 - x))) != 0 {
                    return Some(self.colors.borrow().colup0());
                }
            }
        }

        return None;
    }

    fn get_p1_color(&self, x: usize) -> Option<u8> {
        if x >= self.p1_x && x < self.p1_x + 8 {
            let x = x - self.p1_x;

            if self.refp1 {
                if (self.grp1 & (1 << x)) != 0 {
                    return Some(self.colors.borrow().colup1());
                }
            } else {
                if (self.grp1 & (1 << (7 - x))) != 0 {
                    return Some(self.colors.borrow().colup1());
                }
            }
        }

        return None;
    }

    // Resolve playfield/player/missile/ball priorities and return the color to
    // be rendered at the `x' position.
    fn get_pixel_color(&self, x: usize) -> u8 {
        if !self.pf.priority() {
            // When pixels of two or more objects overlap each other, only the
            // pixel of the object with topmost priority is drawn to the screen.
            // The normal priority ordering is:
            //
            //  Priority     Color    Objects
            //  1 (highest)  COLUP0   P0, M0  (and left side of PF in SCORE-mode)
            //  2            COLUP1   P1, M1  (and right side of PF in SCORE-mode)
            //  3            COLUPF   BL, PF  (only BL in SCORE-mode)
            //  4 (lowest)   COLUBK   BK

            self.get_p0_color(x)
                .or(self.get_m0_color(x))
                .or(self.get_p1_color(x))
                .or(self.get_m1_color(x))
                .or(self.pf.get_color(x))
                .or(self.bl.get_color())
                .unwrap_or(self.colors.borrow().colubk())
        } else {
            // Optionally, the playfield and ball may be assigned to have higher
            // priority (by setting CTRLPF.2). The priority ordering is then:
            //
            //  Priority     Color    Objects
            //  1 (highest)  COLUPF   PF, BL  (always, the SCORE-bit is ignored)
            //  2            COLUP0   P0, M0
            //  3            COLUP1   P1, M1
            //  4 (lowest)   COLUBK   BK

            self.pf.get_color(x)
                .or(self.bl.get_color())
                .or(self.get_p0_color(x))
                .or(self.get_m0_color(x))
                .or(self.get_p1_color(x))
                .or(self.get_m1_color(x))
                .unwrap_or(self.colors.borrow().colubk())
        }
    }

    pub fn clock(&mut self) -> StepResult {
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

        let clocked = self.ctr.clock();

        let visible_cycle = self.ctr.value() >= 17 && self.ctr.value() <= 56;
        let visible_scanline = self.scanline >= 40 && self.scanline < 232;

        if visible_scanline {
            if visible_cycle {
                let x = self.ctr.internal_value as usize - 68;
                let y = self.scanline as usize - 40;
                let color = self.get_pixel_color(x) as usize;
                self.pixels[y][x] = NTSC_PALETTE[color];
                self.bl.tick_visible();
            } else {
                //self.bl.tick_hblank();
            }
        }

        // If we've reset the counter back to 0, we've finished the scanline and started a new
        // scanline.
        if clocked && self.ctr.value() == 0 {
            // Simply writing to the WSYNC causes the microprocessor to halt until the electron
            // beam reaches the right edge of the screen.
            self.wsync = false;

            // If we hit the last scanline, we have to wait for the programmer to signal a
            // VSYNC to reset the gun.
            if self.scanline < 262 {
                self.scanline += 1;
            }

            if self.scanline == 3 {
                // VBlank started
                rv.end_of_frame = true;
            }
        }

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

                if self.vsync {
                    self.ctr.internal_value = 0;
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
            // Colors
            //

            // COLUP0  1111111.  color-lum player 0 and missile 0
            0x0006 => { self.colors.borrow_mut().set_colup0(val & 0xfe) },

            // COLUP1  1111111.  color-lum player 1 and missile 1
            0x0007 => { self.colors.borrow_mut().set_colup1(val & 0xfe) },

            // COLUPF  1111111.  color-lum playfield and ball
            0x0008 => { self.colors.borrow_mut().set_colupf(val & 0xfe) },

            // COLUBK  1111111.  color-lum background
            0x0009 => { self.colors.borrow_mut().set_colubk(val & 0xfe) },

            // CTRLPF  ..11.111  control playfield ball size & collisions
            0x000a => {
                self.pf.set_control(val);
                let ball_size = match (val & 0b0011_0000) >> 4 {
                    0 => 1,
                    1 => 2,
                    2 => 4,
                    3 => 8,
                    _ => unreachable!(),
                };
                self.bl.set_size(ball_size);

                // TODO the other bits
            },

            //
            // Playfield
            //

            // PF0     1111....  playfield register byte 0
            0x000d => { self.pf.set_pf0(val) },

            // PF1     11111111  playfield register byte 1
            0x000e => { self.pf.set_pf1(val) },

            // PF2     11111111  playfield register byte 2
            0x000f => { self.pf.set_pf2(val) },

            //
            // Sprites
            //

            // NUSIZ0  ..111111  number-size player-missile 0
            0x0004 => {
                // TODO the other flags
                self.m0_size = match (val & 0b0011_0000) >> 4 {
                    0 => 1,
                    1 => 2,
                    2 => 4,
                    3 => 8,
                    _ => unreachable!(),
                };
            },

            // NUSIZ1  ..111111  number-size player-missile 1
            0x0005 => {
                // TODO the other flags
                self.m1_size = match (val & 0b0011_0000) >> 4 {
                    0 => 1,
                    1 => 2,
                    2 => 4,
                    3 => 8,
                    _ => unreachable!(),
                };
            },

            // REFP0   ....1...  reflect player 0
            0x000b => { self.refp0 = (val & 0b0000_1000) != 0 },

            // REFP1   ....1...  reflect player 1
            0x000c => { self.refp1 = (val & 0b0000_1000) != 0 },

            // RESP0   <strobe>  reset player 0
            0x0010 => {
                // If the write takes place anywhere within horizontal blanking
                // then the position is set to the left edge of the screen (plus
                // a few pixels towards right: 3 pixels for P0/P1, and only 2
                // pixels for M0/M1/BL).
                self.p0_x = if self.in_hblank() {
                    3
                } else {
                    self.ctr.internal_value as usize - 68
                };
            },

            // RESP1   <strobe>  reset player 1
            0x0011 => {
                self.p1_x = if self.in_hblank() {
                    3
                } else {
                    self.ctr.internal_value as usize - 68
                };
            },

            // RESM0   <strobe>  reset missile 0
            0x0012 => {
                self.m0_x = if self.in_hblank() {
                    2
                } else {
                    self.ctr.internal_value as usize - 68
                };
            },

            // RESM1   <strobe>  reset missile 1
            0x0013 => {
                self.m1_x = if self.in_hblank() {
                    2
                } else {
                    self.ctr.internal_value as usize - 68
                };
            },

            // RESBL   <strobe>  reset ball
            0x0014 => { self.bl.reset() },

            // GRP0    11111111  graphics player 0
            0x001b => { self.grp0 = val },

            // GRP1    11111111  graphics player 1
            0x001c => { self.grp1 = val },

            // ENAM0   ......1.  graphics (enable) missile 0
            0x001d => { self.enam0 = (val & 0x02) != 0 },

            // ENAM1   ......1.  graphics (enable) missile 1
            0x001e => { self.enam1 = (val & 0x02) != 0 },

            // ENABL   ......1.  graphics (enable) ball
            0x001f => { self.bl.set_enabled((val & 0x02) != 0) },

            //
            // Horizontal motion
            //

            // HMP0    1111....  horizontal motion player 0
            0x0020 => { self.hmp0 = hmove_value(val >> 4) as usize },

            // HMP1    1111....  horizontal motion player 1
            0x0021 => { self.hmp1 = hmove_value(val >> 4) as usize },

            // HMM0    1111....  horizontal motion missile 0
            0x0022 => { self.hmm0 = hmove_value(val >> 4) as usize },

            // HMM1    1111....  horizontal motion missile 1
            0x0023 => { self.hmm1 = hmove_value(val >> 4) as usize },

            // HMBL    1111....  horizontal motion ball
            0x0024 => { self.bl.set_hmove_value(val >> 4) },

            // VDELP0  .......1  vertical delay player 0
            0x0025 => { debug!("VDELP0 {}", val & 0x01); }

            // VDELP1  .......1  vertical delay player 1
            0x0026 => { debug!("VDELP1 {}", val & 0x01); }

            // VDELBL  .......1  vertical delay ball
            0x0027 => { debug!("VDELBL {}", val & 0x01); }

            // RESMP0  ......1.  reset missile 0 to player 0
            0x0028 => {
                if (val & 0x02) != 0 {
                    // The centering offset is +3 for normal, +6 for double, and
                    // +10 quad sized player.
                    let offset = match self.m0_size {
                        1 => 3,
                        2 => 6,
                        4 => 10,
                        8 => 15, // TODO can't find this offset
                        _ => unreachable!(),
                    };
                    self.m0_x = self.p0_x + offset;
                }
            },

            // RESMP1  ......1.  reset missile 1 to player 1
            0x0029 => {
                if (val & 0x02) != 0 {
                    // The centering offset is +3 for normal, +6 for double, and
                    // +10 quad sized player.
                    let offset = match self.m0_size {
                        1 => 3,
                        2 => 6,
                        4 => 10,
                        8 => 15, // TODO can't find this offset
                        _ => unreachable!(),
                    };
                    self.m1_x = self.p1_x + offset;
                }
            },

            // HMOVE   <strobe>  apply horizontal motion
            0x002a => {
                self.p0_x = (self.p0_x + self.hmp0) % 160;
                self.p1_x = (self.p1_x + self.hmp1) % 160;
                self.m0_x = (self.m0_x + self.hmm0) % 160;
                self.m1_x = (self.m1_x + self.hmm1) % 160;

                //self.bl_x = (self.bl_x + self.hmbl) % 160;
                self.bl.start_hmove();

                self.late_reset_hblank = 8;
            },

            // HMCLR   <strobe>  clear horizontal motion registers
            0x002b => {
                self.hmp0 = 0;
                self.hmp1 = 0;
                self.hmm0 = 0;
                self.hmm1 = 0;
                self.bl.hmclr();
            },

            //
            // Audio
            //

            0x0015 ..= 0x001a => { },

            _ => debug!("register: 0x{:04X} 0x{:02X}", address, val), 
        }
    }
}

fn hmove_value(v: u8) -> u8 {
    // Signed Motion Value (-8..-1=Right, 0=No motion, +1..+7=Left)

    if v == 0 {
        0
    } else if v < 8 {
        v + 8
    } else {
        v - 8
    }
}
