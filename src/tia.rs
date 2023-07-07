mod ball;
mod color;
mod counter;
mod missile;
mod palette;
mod player;
mod playfield;

use std::rc::Rc;
use std::cell::RefCell;

use crate::bus::Bus;
use crate::tia::ball::Ball;
use crate::tia::color::Colors;
use crate::tia::counter::Counter;
use crate::tia::missile::Missile;
use crate::tia::palette::NTSC_PALETTE;
use crate::tia::player::Player;
use crate::tia::playfield::Playfield;

use sdl2::pixels::Color;

pub enum PlayerType {
    Player0,
    Player1,
}

// Set H-SYNC
const SHS: u8 = 4;

// Reset H-SYNC
const RHS: u8 = 8;

// ColourBurst
const RCB: u8 = 12;

// Reset H-BLANK
const RHB: u8 = 16;

// Late RHB
const LRHB: u8 = 18;

// Center
const CNT: u8 = 36;

// RESET, H-BLANK
const SHB: u8 = 56;

pub struct TIA {
    // The scanline we're currently processing
    scanline: u16,

    // HSYNC counter
    ctr: Rc<RefCell<Counter>>,

    // Vertical sync
    vsync: bool,
    vblank: u8,
    late_reset_hblank: bool,

    // Horizontal sync
    wsync: bool,

    // Input
    // I'm only implementing player 0 joystick controls, so only one input port
    inpt4_port: bool,
    inpt4_latch: bool,

    colors: Rc<RefCell<Colors>>,

    // Graphics
    pf: Playfield,
    p0: Player,
    p1: Player,
    m0: Missile,
    m1: Missile,
    bl: Ball,

    // Pixels to be rendered
    pixels: Vec<Vec<Color>>,
}

pub struct StepResult {
    pub end_of_frame: bool,
}

impl TIA {
    pub fn new_tia() -> Self {
        let colors = Rc::new(RefCell::new(Colors::new_colors()));
        let hsync_ctr = Rc::new(RefCell::new(Counter::new_counter(57, 0)));
        let pf = Playfield::new_playfield(colors.clone(), hsync_ctr.clone());
        let bl = Ball::new_ball(colors.clone());
        let m0 = Missile::new_missile(colors.clone(), PlayerType::Player0);
        let m1 = Missile::new_missile(colors.clone(), PlayerType::Player1);
        let p0 = Player::new_player(colors.clone(), PlayerType::Player0);
        let p1 = Player::new_player(colors.clone(), PlayerType::Player1);

        Self {
            scanline: 0,

            ctr: hsync_ctr,

            vsync: false,
            vblank: 0,
            late_reset_hblank: false,

            wsync: false,

            inpt4_port: false,
            inpt4_latch: true,

            colors: colors,

            pf: pf,
            bl: bl,
            m0: m0,
            m1: m1,
            p0: p0,
            p1: p1,

            pixels: vec![vec![Color::RGB(0, 0, 0); 160]; 192],
        }
    }

    pub fn cpu_halt(&self) -> bool { self.wsync }

    pub fn get_pixels(&self) -> &Vec<Vec<Color>> { &self.pixels }

    pub fn joystick_fire(&mut self, pressed: bool) {
        self.inpt4_port = !pressed;

        if pressed {
            debug!("INPT4 pressed");
            self.inpt4_latch = false;
        }
    }

    // Resolve playfield/player/missile/ball priorities and return the color to
    // be rendered.
    fn get_pixel_color(&self) -> u8 {
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

            self.p0.get_color()
                .or(self.m0.get_color())
                .or(self.p1.get_color())
                .or(self.m1.get_color())
                .or(self.pf.get_color())
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

            self.pf.get_color()
                .or(self.bl.get_color())
                .or(self.p0.get_color())
                .or(self.m0.get_color())
                .or(self.p1.get_color())
                .or(self.m1.get_color())
                .unwrap_or(self.colors.borrow().colubk())
        }
    }

    fn visible_cycle(&self) -> bool {
        self.ctr.borrow().value() > RHB && self.ctr.borrow().value() <= SHB
    }

    fn render_cycle(&self) -> bool {
        let hblank_ctr_value = if self.late_reset_hblank {
            LRHB
        } else {
            RHB
        };

        self.ctr.borrow().value() > hblank_ctr_value && self.ctr.borrow().value() <= SHB
    }

    fn visible_scanline(&self) -> bool { self.scanline >= 40 && self.scanline < 232 }

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

        // Clock the horizontal sync counter
        let clocked = self.ctr.borrow_mut().clock();

        if self.visible_scanline() {
            if self.visible_cycle() {
                // Player, missile, and ball counters only get clocked on visible cycles
                self.p0.tick_visible();
                self.p1.tick_visible();
                self.m0.tick_visible();
                self.m1.tick_visible();
                self.bl.tick_visible();

                let color = if self.render_cycle() {
                    self.get_pixel_color() as usize
                } else {
                    0 // default black
                };

                let x = self.ctr.borrow().internal_value as usize - 68;
                let y = self.scanline as usize - 40;
                self.pixels[y][x] = NTSC_PALETTE[color];
            }
        }

        if clocked {
            match self.ctr.borrow().value() {
                // If we've reset the counter back to 0, we've finished the scanline and started
                // a new scanline, in HBlank.
                0 => {
                    // If we hit the last scanline, we have to wait for the programmer to signal
                    // a VSYNC to reset the gun.
                    if self.scanline < 262 {
                        self.scanline += 1;
                    }

                    if self.scanline == 3 {
                        // VBlank started
                        rv.end_of_frame = true;
                    }

                    // Simply writing to the WSYNC causes the microprocessor to halt until the
                    // electron beam reaches the right edge of the screen.
                    self.wsync = false;

                    if self.late_reset_hblank {
                        //debug!("LRHB: scanline {}, dot {} RESET", self.scanline, self.ctr.internal_value);
                    }
                    self.late_reset_hblank = false;
                },

                // Reset HBlank
                RHB => {
                    if !self.late_reset_hblank {
                        //debug!("RHB: scanline {}, dot {}", self.scanline, self.ctr.internal_value);
                    }
                },

                // Late Reset HBlank
                LRHB => {
                    if self.late_reset_hblank {
                        //debug!("LRHB: scanline {}, dot {}", self.scanline, self.ctr.internal_value);
                    }
                },

                _ => { },
            }
        }

        rv
    }
}

impl Bus for TIA {
    // https://problemkaputt.de/2k6specs.htm#memoryandiomap

    fn read(&mut self, address: u16) -> u8 {
        match address {
            // VBLANK
            0x0001 => self.vblank,

            // INPT4   1.......  read input
            // INPT5   1.......  read input
            0x003C | 0x003D => {
                // D6 of VBLANK specifies latched input for INPT4 and 5
                if (self.vblank & 0b0100_0000) != 0 {
                    0x80
                } else {
                    0x00
                }
            },

            _ => 0,
        }
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
                    self.ctr.borrow_mut().reset();
                    self.scanline = 0;
                }
            },

            // VBLANK  11....1.  vertical blank set-clear
            0x0001 => {
                self.vblank = val;

                if (val & 0x80) != 0 {
                    debug!("INPT4 latch reset");
                    self.inpt4_latch = true;
                }
            },

            // WSYNC   <strobe>  wait for leading edge of horizontal blank
            0x0002 => { self.wsync = true },

            // RSYNC   <strobe>  reset horizontal sync counter
            0x0003 => { self.ctr.borrow_mut().reset() },

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
                let missile_size = match (val & 0b0011_0000) >> 4 {
                    0 => 1,
                    1 => 2,
                    2 => 4,
                    3 => 8,
                    _ => unreachable!(),
                };
                let player_copies = val & 0b0000_0111;

                self.m0.set_size(missile_size);
                self.p0.set_copies(player_copies);
            },

            // NUSIZ1  ..111111  number-size player-missile 1
            0x0005 => {
                let missile_size = match (val & 0b0011_0000) >> 4 {
                    0 => 1,
                    1 => 2,
                    2 => 4,
                    3 => 8,
                    _ => unreachable!(),
                };
                let player_copies = val & 0b0000_0111;

                self.m1.set_size(missile_size);
                self.p1.set_copies(player_copies);
            },

            // REFP0   ....1...  reflect player 0
            0x000b => { self.p0.set_horizontal_mirror((val & 0b0000_1000) != 0) },

            // REFP1   ....1...  reflect player 1
            0x000c => { self.p1.set_horizontal_mirror((val & 0b0000_1000) != 0) },

            // RESP0   <strobe>  reset player 0
            0x0010 => {
                // If the write takes place anywhere within horizontal blanking
                // then the position is set to the left edge of the screen (plus
                // a few pixels towards right: 3 pixels for P0/P1, and only 2
                // pixels for M0/M1/BL).
                self.p0.reset();
            },

            // RESP1   <strobe>  reset player 1
            0x0011 => {
                self.p1.reset();
            },

            // RESM0   <strobe>  reset missile 0
            0x0012 => { self.m0.reset() },

            // RESM1   <strobe>  reset missile 1
            0x0013 => { self.m1.reset() },

            // RESBL   <strobe>  reset ball
            0x0014 => { self.bl.reset() },

            // AUDV0
            0x0015 => { debug!("AUDV0: {}", val) },

            // AUDV1
            0x0016 => { debug!("AUDV1: {}", val) },

            // AUDF0
            0x0017 => { debug!("AUDF0: {}", val) },

            // AUDF1
            0x0018 => { debug!("AUDF1: {}", val) },

            // AUDC0
            0x0019 => { debug!("AUDC0: {}", val) },

            // AUDC1
            0x001a => { debug!("AUDC1: {}", val) },

            // GRP0    11111111  graphics player 0
            0x001b => { self.p0.set_graphic(val) },

            // GRP1    11111111  graphics player 1
            0x001c => { self.p1.set_graphic(val) },

            // ENAM0   ......1.  graphics (enable) missile 0
            0x001d => { self.m0.set_enabled((val & 0x02) != 0) },

            // ENAM1   ......1.  graphics (enable) missile 1
            0x001e => { self.m1.set_enabled((val & 0x02) != 0) },

            // ENABL   ......1.  graphics (enable) ball
            0x001f => { self.bl.set_enabled((val & 0x02) != 0) },

            //
            // Horizontal motion
            //

            // HMP0    1111....  horizontal motion player 0
            0x0020 => { self.p0.set_hmove_value(val) },

            // HMP1    1111....  horizontal motion player 1
            0x0021 => { self.p1.set_hmove_value(val) },

            // HMM0    1111....  horizontal motion missile 0
            0x0022 => { self.m0.set_hmove_value(val) },

            // HMM1    1111....  horizontal motion missile 1
            0x0023 => { self.m1.set_hmove_value(val) },

            // HMBL    1111....  horizontal motion ball
            0x0024 => { self.bl.set_hmove_value(val) },

            // VDELP0  .......1  vertical delay player 0
            0x0025 => { debug!("VDELP0 {}", val & 0x01); }

            // VDELP1  .......1  vertical delay player 1
            0x0026 => { debug!("VDELP1 {}", val & 0x01); }

            // VDELBL  .......1  vertical delay ball
            0x0027 => { debug!("VDELBL {}", val & 0x01); }

            // RESMP0  ......1.  reset missile 0 to player 0
            0x0028 => {
                if (val & 0x02) != 0 {
                    //self.m0.reset_to_player(self.p0);
                    self.m0.reset_to_player();
                }
            },

            // RESMP1  ......1.  reset missile 1 to player 1
            0x0029 => {
                if (val & 0x02) != 0 {
                    //self.m1.reset_to_player(self.p1);
                    self.m1.reset_to_player();
                }
            },

            // HMOVE   <strobe>  apply horizontal motion
            0x002a => {
                self.bl.start_hmove();
                self.m0.start_hmove();
                self.m1.start_hmove();
                self.p0.start_hmove();
                self.p1.start_hmove();

                debug!("HMOVE: scanline {}, dot {}", self.scanline, self.ctr.borrow().internal_value);

                self.late_reset_hblank = true;
            },

            // HMCLR   <strobe>  clear horizontal motion registers
            0x002b => {
                self.bl.hmclr();
                self.m0.hmclr();
                self.m1.hmclr();
                self.p0.hmclr();
                self.p1.hmclr();
            },

            _ => debug!("register: 0x{:04X} 0x{:02X}", address, val), 
        }
    }
}

