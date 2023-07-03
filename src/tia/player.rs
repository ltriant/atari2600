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

    // The REFPx register, for rendering backwards
    horizontal_mirror: bool,
    copies: u8,
    // 8 bit graphic to draw
    graphic: u8,
    // Delay - in TIA ticks - in rendering the graphic
    graphic_delay: u8,

    /*
     * Graphics Scan Counter
     */
    graphic_draw: bool,
    graphic_bit_idx: usize,
}

impl Player {
    pub fn new_player(colors: Rc<RefCell<Colors>>, player: PlayerType) -> Self {
        return Self {
            colors: colors,
            player: player,

            hmove_offset: 0,
            ctr: Counter::new_counter(40, 0),

            horizontal_mirror: false,
            copies: 0,
            graphic: 0,
            // Player sprites are delayed by 1 TIA tick
            graphic_delay: 5,

            graphic_draw: false,
            graphic_bit_idx: 0,
        }
    }

    pub fn size(&self) -> usize { 8 }

    pub fn set_hmove_value(&mut self, v: u8) { self.hmove_offset = v }
    pub fn set_graphic(&mut self, graphic: u8) { self.graphic = graphic }
    pub fn set_horizontal_mirror(&mut self, reflect: bool) { self.horizontal_mirror = reflect }
    pub fn set_copies(&mut self, v: u8) { self.copies = v }
    pub fn hmclr(&mut self) { self.hmove_offset = 0 }
    pub fn reset(&mut self) {
        self.ctr.reset();
        self.horizontal_mirror = false;
        self.graphic = 0;
        self.graphic_delay = 5;
        self.graphic_draw = false;
        self.graphic_bit_idx = 0;
    }

    pub fn start_hmove(&mut self) {
        // HMOVE causes rendering to be delayed by two cycles of the counter until LHRB, which is
        // eight ticks long.
        //self.graphic_delay += 8;
        self.ctr.start_hmove(self.hmove_offset);
    }

    pub fn tick_visible(&mut self) {
        if self.graphic_delay > 0 {
            self.graphic_delay -= 1;

            if self.graphic_delay == 0 {
                self.graphic_draw = true;
            }
        } else if self.graphic_draw {
            self.graphic_bit_idx += 1;
            self.graphic_bit_idx %= self.size();

            if self.graphic_bit_idx == 0 {
                self.graphic_draw = false;
            }
        }

        if self.ctr.clock() && self.ctr.value() == 39 {
            self.graphic_delay = 5;
        }
    }

    pub fn get_color(&self) -> Option<u8> {
        if self.graphic_draw {
            let color = match self.player {
                PlayerType::Player0 => self.colors.borrow().colup0(),
                PlayerType::Player1 => self.colors.borrow().colup1(),
            };

            let x = self.graphic_bit_idx;

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
