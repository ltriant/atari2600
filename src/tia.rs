mod color;
mod palette;
mod playfield;

use std::rc::Rc;
use std::cell::RefCell;

use crate::bus::Bus;
use crate::tia::color::Colors;
use crate::tia::palette::NTSC_PALETTE;
use crate::tia::playfield::Playfield;

use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

pub struct TIA {
    dot: u16,
    scanline: u16,

    // Vertical sync
    vsync: bool,
    vblank: bool,
    late_reset_hblank: bool,

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
    bl_x: usize,
    enabl: bool,
    bl_size: usize,
    hmbl: usize,
}

pub struct StepResult {
    pub end_of_frame: bool,
}

fn hmove_value(v: u8) -> u8 {
    // Signed Motion Value (-8..-1=Right, 0=No motion, +1..+7=Left)

    /*
    match v & 0x0f {
        0b0000 => 0,
        0b0001 => 1,
        0b0010 => 2,
        0b0011 => 3,
        0b0100 => 4,
        0b0101 => 5,
        0b0110 => 6,
        0b0111 => 7,

        0b1000 => -1,
        0b1001 => -2,
        0b1010 => -3,
        0b1011 => -4,
        0b1100 => -5,
        0b1101 => -6,
        0b1110 => -7,
        0b1111 => -8,
        _ => unreachable!(),
    }
    */

    if v == 0 {
        0
    } else if v < 8 {
        v + 8
    } else {
        v - 8
    }
}

impl TIA {
    pub fn new_tia() -> Self {
        let colors = Rc::new(RefCell::new(Colors::new_colors()));
        let pf = Playfield::new_playfield(colors.clone());

        Self {
            dot: 0,
            scanline: 0,

            vsync: false,
            vblank: false,
            wsync: false,
            late_reset_hblank: false,

            colors: colors,
            pf: pf,

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

            bl_x: 0,
            enabl: false,
            bl_size: 0,

            hmp0: 0,
            hmp1: 0,
            hmm0: 0,
            hmm1: 0,
            hmbl: 0,
        }
    }

    pub fn cpu_halt(&self) -> bool { self.wsync }

    fn in_hblank(&self) -> bool { self.dot < 68 }

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

    fn get_bl_color(&self, x: usize) -> Option<u8> {
        if x >= self.bl_x && x < self.bl_x + self.bl_size && self.enabl {
            Some(self.colors.borrow().colupf())
        } else {
            None
        }
    }

    fn get_m0_color(&self, x: usize) -> Option<u8> {
        if x == self.m0_x && self.enam0 {
            Some(self.colors.borrow().colup0())
        } else {
            None
        }
    }

    fn get_m1_color(&self, x: usize) -> Option<u8> {
        if x == self.m1_x && self.enam1 {
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
                .or(self.get_bl_color(x))
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
                .or(self.get_bl_color(x))
                .or(self.get_p0_color(x))
                .or(self.get_m0_color(x))
                .or(self.get_p1_color(x))
                .or(self.get_m1_color(x))
                .unwrap_or(self.colors.borrow().colubk())
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

            let color = if self.late_reset_hblank && x < 8 {
                0
            } else {
                self.get_pixel_color(x as usize)
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
            self.late_reset_hblank = false;
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
                self.bl_size = match (val & 0b0011_0000) >> 4 {
                    0 => 1,
                    1 => 2,
                    2 => 4,
                    3 => 8,
                    _ => unreachable!(),
                };

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
                    self.dot as usize - 68
                };
            },

            // RESP1   <strobe>  reset player 1
            0x0011 => {
                self.p1_x = if self.in_hblank() {
                    3
                } else {
                    self.dot as usize - 68
                };
            },

            // RESM0   <strobe>  reset missile 0
            0x0012 => {
                self.m0_x = if self.in_hblank() {
                    2
                } else {
                    self.dot as usize - 68
                };
            },

            // RESM1   <strobe>  reset missile 1
            0x0013 => {
                self.m1_x = if self.in_hblank() {
                    2
                } else {
                    self.dot as usize - 68
                };
            },

            // RESBL   <strobe>  reset ball
            0x0014 => {
                self.bl_x = if self.in_hblank() {
                    2
                } else {
                    self.dot as usize - 68
                };
            },

            // GRP0    11111111  graphics player 0
            0x001b => { self.grp0 = val },

            // GRP1    11111111  graphics player 1
            0x001c => { self.grp1 = val },

            // ENAM0   ......1.  graphics (enable) missile 0
            0x001d => { self.enam0 = (val & 0x02) != 0 },

            // ENAM1   ......1.  graphics (enable) missile 1
            0x001e => { self.enam1 = (val & 0x02) != 0 },

            // ENABL   ......1.  graphics (enable) ball
            0x001f => { self.enabl = (val & 0x02) != 0 },

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
            0x0024 => { self.hmbl = hmove_value(val >> 4) as usize },

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
                self.bl_x = (self.bl_x + self.hmbl) % 160;

                self.late_reset_hblank = true;
            },

            // HMCLR   <strobe>  clear horizontal motion registers
            0x002b => {
                self.hmp0 = 0;
                self.hmp1 = 0;
                self.hmm0 = 0;
                self.hmm1 = 0;
                self.hmbl = 0;
            },

            //
            // Audio
            //

            0x0015 ..= 0x001a => { },

            _ => debug!("register: 0x{:04X} 0x{:02X}", address, val), 
        }
    }
}
