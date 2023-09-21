use std::rc::Rc;
use std::cell::RefCell;

use crate::tia::PlayerType;
use crate::tia::color::Colors;
use crate::tia::counter::Counter;

// Player sprites start 1 tick later than other sprites
const INIT_DELAY: isize = 5;

// How many bits to a graphic
const GRAPHIC_SIZE: isize= 8;

pub struct Player {
    colors: Rc<RefCell<Colors>>,
    player: PlayerType,

    hmove_offset: u8,
    ctr: Counter,

    // The REFPx register, for rendering backwards
    horizontal_mirror: bool,
    // The NUSIZx register
    nusiz: u8,
    // The 8-bit graphic to draw
    graphic: u8,

    // The VDELPx register
    vdel: bool,
    old_value: u8,

    // Graphics Scan Counter
    graphic_bit_idx: Option<isize>,
    graphic_bit_copies_written: usize,
    graphic_bit_value: Option<bool>,
}

impl Player {
    pub fn new(colors: Rc<RefCell<Colors>>, player: PlayerType) -> Self {
        return Self {
            colors: colors,
            player: player,

            hmove_offset: 0,
            ctr: Counter::new(40, 39),

            horizontal_mirror: false,
            nusiz: 0,
            graphic: 0,

            vdel: false,
            old_value: 0,

            graphic_bit_idx: None,
            graphic_bit_copies_written: 0,
            graphic_bit_value: None,
        }
    }

    pub fn size(&self) -> usize {
        match self.nusiz & 0x0f {
            0b0101 => 2,
            0b0111 => 4,
            _      => 1,
        }
    }

    pub fn counter(&self) -> &Counter { &self.ctr }
    pub fn set_hmove_value(&mut self, v: u8) { self.hmove_offset = v }
    pub fn set_graphic(&mut self, graphic: u8) { self.graphic = graphic }
    pub fn set_horizontal_mirror(&mut self, reflect: bool) { self.horizontal_mirror = reflect }
    pub fn set_nusiz(&mut self, v: u8) { self.nusiz = v & 0x0f }
    pub fn set_vdel(&mut self, v: bool) { self.vdel = v }
    pub fn set_vdel_value(&mut self) { self.old_value = self.graphic }
    pub fn hmclr(&mut self) { self.hmove_offset = 0 }
    pub fn reset(&mut self) {
        self.ctr.reset();

        if self.should_draw_graphic() || self.should_draw_copy() {
            self.graphic_bit_idx = Some(-1 * INIT_DELAY);
            self.graphic_bit_copies_written = 0;
        }
    }

    pub fn start_hmove(&mut self) {
        self.ctr.start_hmove(self.hmove_offset);
        self.tick_graphic_circuit();
    }

    // Based on current state, return whether or not we are rendering this sprite
    fn pixel_bit(&self) -> bool {
        if let Some(x) = self.graphic_bit_idx {
            let graphic = if self.vdel {
                self.old_value
            } else {
                self.graphic
            };

            return if self.horizontal_mirror {
                (graphic & (1 << x)) != 0
            } else {
                (graphic & (1 << (7 - x))) != 0
            };
        } else {
            return false;
        }
    }

    fn tick_graphic_circuit(&mut self) {
        if let Some(mut idx) = self.graphic_bit_idx {
            if idx >= 0 && idx < 8 {
                self.graphic_bit_value = Some(self.pixel_bit());

                self.graphic_bit_copies_written += 1;
                if self.graphic_bit_copies_written == self.size() {
                    self.graphic_bit_copies_written = 0;
                    idx += 1;
                }

                if idx == GRAPHIC_SIZE {
                    self.graphic_bit_idx = None;
                } else {
                    self.graphic_bit_idx = Some(idx);
                }
            } else {
                self.graphic_bit_idx = Some(idx + 1);
            }
        } else {
            self.graphic_bit_value = None;
        }
    }

    fn should_draw_graphic(&self) -> bool {
        self.ctr.value() == 39
    }

    fn should_draw_copy(&self) -> bool {
        let count = self.ctr.value();

           (count == 3  && (self.nusiz == 0b001 || self.nusiz == 0b011))
        || (count == 7  && (self.nusiz == 0b010 || self.nusiz == 0b011 || self.nusiz == 0b110))
        || (count == 15 && (self.nusiz == 0b100 || self.nusiz == 0b110))
    }

    pub fn clock(&mut self) {
        self.tick_graphic_circuit();

        if self.ctr.clock() && (self.should_draw_graphic() || self.should_draw_copy()) {
            self.graphic_bit_idx = Some(-1 * INIT_DELAY);
            self.graphic_bit_copies_written = 0;
        }
    }

    pub fn apply_hmove(&mut self) {
        let (moved, counter_clocked) = self.ctr.apply_hmove(self.hmove_offset);

        if counter_clocked && (self.should_draw_graphic() || self.should_draw_copy()) {
            self.graphic_bit_idx = Some(-1 * INIT_DELAY);
            self.graphic_bit_copies_written = 0;
        }

        if moved {
            self.tick_graphic_circuit();
        }
    }

    pub fn get_color(&self) -> Option<u8> {
        if let Some(true) = self.graphic_bit_value {
            let color = match self.player {
                PlayerType::Player0 => self.colors.borrow().colup0(),
                PlayerType::Player1 => self.colors.borrow().colup1(),
            };

            return Some(color);
        }

        return None;
    }
}
