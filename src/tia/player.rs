use std::rc::Rc;
use std::cell::RefCell;

use crate::tia::PlayerType;
use crate::tia::color::Colors;
use crate::tia::counter::Counter;

pub struct Player {
    colors: Rc<RefCell<Colors>>,
    player: PlayerType,

    hmove_offset: u8,
    ctr: Counter,

    graphic: u8,
    horizontal_mirror: bool,
}

impl Player {
    pub fn new_player(colors: Rc<RefCell<Colors>>, player: PlayerType) -> Self {
        return Self {
            colors: colors,
            player: player,

            hmove_offset: 0,
            ctr: Counter::new_counter(40, 0),

            graphic: 0,
            horizontal_mirror: false,
        }
    }

    pub fn set_hmove_value(&mut self, v: u8) { self.hmove_offset = v }
    pub fn set_graphic(&mut self, graphic: u8) { self.graphic = graphic }
    pub fn set_horizontal_mirror(&mut self, reflect: bool) { self.horizontal_mirror = reflect }
    pub fn hmclr(&mut self) { self.hmove_offset = 0 }
    pub fn reset(&mut self) {
        self.ctr.reset();
    }

    pub fn start_hmove(&mut self) {
        self.ctr.start_hmove(self.hmove_offset);
    }

    pub fn tick_visible(&mut self) {
        self.ctr.clock();
    }

    pub fn get_color(&self) -> Option<u8> {
        let color = match self.player {
            PlayerType::Player0 => self.colors.borrow().colup0(),
            PlayerType::Player1 => self.colors.borrow().colup1(),
        };

        let x = self.ctr.internal_value;
        if x < 8 {
            if self.horizontal_mirror {
                if (self.graphic & (1 << x)) != 0 {
                    return Some(color);
                }
            } else {
                if (self.graphic & (1 << (7 - x))) != 0 {
                    return Some(color);
                }
            }
        }

        return None;
    }
}
