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
    pf: [bool; 20],
    pf_horizontal_mirror: bool,
    pf_score_mode: bool,
    pf_priority: bool,

    // Player sprites
    grp0: u8,
    grp1: u8,
    refp0: bool,
    refp1: bool,
    p0_x: usize,
    p1_x: usize,

    // Missile sprites
    m0_x: usize,
    m1_x: usize,
    enam0: bool,
    enam1: bool,

    // Ball sprite
    bl_x: usize,
    enabl: bool,
    bl_size: usize,
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

            pf: [false; 20],
            pf0: 0,
            pf1: 0,
            pf2: 0,
            pf_horizontal_mirror: false,
            pf_score_mode: false,
            pf_priority: false,

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

            bl_x: 0,
            enabl: false,
            bl_size: 0,
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

    fn update_playfield(&mut self) {
        // The playfield is a 20-bit set of dots, where each dot makes up four
        // pixels on the screen.

        // PF0 is the first 4 bits, in big-endian order
        for x in 0 .. 4 {
            self.pf[x] = (self.pf0 >> (x + 4)) & 0x01 != 0;
        }

        // PF1 is the next 8 bits, in little-endian order
        for x in 0 .. 8 {
            self.pf[x + 4] = (self.pf1 >> (7 - x)) & 0x01 != 0;
        }

        // PF2 is the last 8 bits, in big-endian order
        for x in 0 .. 8 {
            self.pf[x + 12] = (self.pf2 >> x) & 0x01 != 0;
        }
    }

    fn get_bl_color(&self, x: usize) -> Option<u8> {
        if x >= self.bl_x && x < self.bl_x + self.bl_size && self.enabl {
            Some(self.colupf)
        } else {
            None
        }
    }

    fn get_m0_color(&self, x: usize) -> Option<u8> {
        if x == self.m0_x && self.enam0 {
            Some(self.colup0)
        } else {
            None
        }
    }

    fn get_m1_color(&self, x: usize) -> Option<u8> {
        if x == self.m1_x && self.enam1 {
            Some(self.colup1)
        } else {
            None
        }
    }

    fn get_p0_color(&self, x: usize) -> Option<u8> {
        if x >= self.p0_x && x < self.p0_x + 8 {
            let x = x - self.p0_x;

            if self.refp0 {
                if (self.grp0 & (1 << x)) != 0 {
                    return Some(self.colup0);
                }
            } else {
                if (self.grp0 & (1 << (7 - x))) != 0 {
                    return Some(self.colup0);
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
                    return Some(self.colup1);
                }
            } else {
                if (self.grp1 & (1 << (7 - x))) != 0 {
                    return Some(self.colup1);
                }
            }
        }

        return None;
    }

    fn get_pf_color(&self, x: usize) -> Option<u8> {
        if x < 80 {
            // The playfield makes up the left-most side of the screen.

            let pf_x = x / 4;

            if self.pf[pf_x as usize] {
                return if self.pf_score_mode {
                    Some(self.colup0)
                } else {
                    Some(self.colupf)
                };
            }
        } else {
            // The playfield also makes up the right-most side of the
            // screen, optionally mirrored horizontally as denoted by the
            // CTRLPF register.

            let pf_x = (x - 80) / 4;

            let idx = if self.pf_horizontal_mirror {
                self.pf.len() - 1 - pf_x as usize
            } else {
                pf_x as usize
            };

            if self.pf[idx] {
                return if self.pf_score_mode {
                    Some(self.colup1)
                } else {
                    Some(self.colupf)
                };
            }
        }

        return None;
    }

    // Resolve playfield/player/missile/ball priorities and return the color to
    // be rendered at the `x' position.
    fn get_pixel_color(&self, x: usize) -> u8 {
        if !self.pf_priority {
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
                .or(self.get_pf_color(x))
                .or(self.get_bl_color(x))
                .unwrap_or(self.colubk)
        } else {
            // Optionally, the playfield and ball may be assigned to have higher
            // priority (by setting CTRLPF.2). The priority ordering is then:
            //
            //  Priority     Color    Objects
            //  1 (highest)  COLUPF   PF, BL  (always, the SCORE-bit is ignored)
            //  2            COLUP0   P0, M0
            //  3            COLUP1   P1, M1
            //  4 (lowest)   COLUBK   BK

            self.get_pf_color(x)
                .or(self.get_bl_color(x))
                .or(self.get_p0_color(x))
                .or(self.get_m0_color(x))
                .or(self.get_p1_color(x))
                .or(self.get_m1_color(x))
                .unwrap_or(self.colubk)
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

            let color = self.get_pixel_color(x as usize) as usize;
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
                self.pf_priority          = (val & 0x04) != 0;
                self.pf_score_mode        = (val & 0x02) != 0 && !self.pf_priority;
                self.bl_size              = match (val & 0b0011_0000) >> 4 {
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

            //
            // Sprites
            //

            // NUSIZ0  ..111111  number-size player-missile 0
            0x0004 => { },

            // NUSIZ1  ..111111  number-size player-missile 1
            0x0005 => { },

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
            0x001b => {
                debug!("grp0: {:08b}", val);
                self.grp0 = val;
            },

            // GRP1    11111111  graphics player 1
            0x001c => {
                debug!("grp1: {:08b}", val);
                self.grp1 = val;
            },

            // ENAM0   ......1.  graphics (enable) missile 0
            0x001d => {
                self.enam0 = (val & 0x02) != 0;
            },

            // ENAM1   ......1.  graphics (enable) missile 1
            0x001e => {
                self.enam1 = (val & 0x02) != 0;
            },

            // ENABL   ......1.  graphics (enable) ball
            0x001f => {
                self.enabl = (val & 0x02) != 0;
            },

            //
            // Audio
            //

            0x0015 ..= 0x001a => { },

            _ => { }, // unimplemented!("register: 0x{:04X} 0x{:02X}", address, val),
        }
    }
}
