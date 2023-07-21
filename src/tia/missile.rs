use std::rc::Rc;
use std::cell::RefCell;

use crate::tia::PlayerType;
use crate::tia::color::Colors;
use crate::tia::counter::Counter;
use crate::tia::player::Player;

const INIT_DELAY: isize = 4;
const GRAPHIC_SIZE: isize = 1;

pub struct Missile {
    colors: Rc<RefCell<Colors>>,
    sibling_player: PlayerType,

    enabled: bool,
    hmove_offset: u8,
    nusiz: usize,
    ctr: Counter,

    // Graphics Scan Counter
    graphic_bit_idx: Option<isize>,
    graphic_bit_copies_written: usize,
    graphic_bit_value: Option<bool>,
}

impl Missile {
    pub fn new(colors: Rc<RefCell<Colors>>, sibling_player: PlayerType) -> Self {
        Self {
            colors: colors,
            sibling_player: sibling_player,

            enabled: false,
            hmove_offset: 0,
            nusiz: 0,
            ctr: Counter::new(40, 39),

            graphic_bit_idx: None,
            graphic_bit_copies_written: 0,
            graphic_bit_value: None,
        }
    }

    pub fn set_enabled(&mut self, en: bool) { self.enabled = en }
    pub fn set_hmove_value(&mut self, v: u8) { self.hmove_offset = v }
    pub fn set_nusiz(&mut self, size: usize) { self.nusiz = size }
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

    fn copies(&self) -> usize { self.nusiz }
    fn pixel_bit(&self) -> bool { self.enabled }

    fn tick_graphic_circuit(&mut self) {
        if let Some(mut idx) = self.graphic_bit_idx {
            if idx >= 0 && idx < 8 {
                self.graphic_bit_value = Some(self.pixel_bit());

                self.graphic_bit_copies_written += 1;
                if self.graphic_bit_copies_written == self.copies() {
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

    pub fn reset_to_player(&mut self, player: &Player) {
        self.ctr.reset_to(player.counter().internal_value);
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
            let color = match self.sibling_player {
                PlayerType::Player0 => self.colors.borrow().colup0(),
                PlayerType::Player1 => self.colors.borrow().colup1(),
            };

            return Some(color);
        }

        return None;
    }
}
