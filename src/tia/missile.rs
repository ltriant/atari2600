use std::rc::Rc;
use std::cell::RefCell;

use crate::tia::color::Colors;
use crate::tia::counter::Counter;

pub struct Missile {
    colors: Rc<RefCell<Colors>>,

    enabled: bool,
    hmove_offset: u8,
    size: usize,
    ctr: Counter,
}

impl Missile {
    pub fn new_missile(colors: Rc<RefCell<Colors>>) -> Self {
        Self {
            colors: colors,

            enabled: false,
            hmove_offset: 0,
            size: 0,
            ctr: Counter::new_counter(40, 0),
        }
    }

    pub fn set_enabled(&mut self, en: bool) { self.enabled = en }
    pub fn set_hmove_value(&mut self, v: u8) { self.hmove_offset = v }
    pub fn set_size(&mut self, size: usize) { self.size = size }
    pub fn hmclr(&mut self) { self.hmove_offset = 0 }
    pub fn reset(&mut self) {
        self.ctr.reset();
    }

    pub fn tick_visible(&mut self) {
        self.ctr.clock();
    }

    pub fn reset_to_player(&mut self) {
        // TODO take a Player parameter

        // The centering offset is +3 for normal, +6 for double, and
        // +10 quad sized player.
        let offset = match self.size {
            1 => 3,
            2 => 6,
            4 => 10,
            8 => 15, // TODO can't find this offset
            _ => unreachable!(),
        };

        // TODO set the counter to player x + offset
    }

    pub fn start_hmove(&mut self) {
        self.ctr.start_hmove(self.hmove_offset);
    }

    pub fn get_color(&self) -> Option<u8> {
        // TODO this is wrong
        //if x >= self.m0_x && x < self.m0_x + self.m0_size && self.enam0 {
        debug!("size: {}", self.size);
        if self.enabled && self.ctr.internal_value < self.size as u8 {
            return Some(self.colors.borrow().colup0()); // m1 gets p1
        }

        return None;
    }
}
