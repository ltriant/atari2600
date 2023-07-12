use std::rc::Rc;
use std::cell::RefCell;

use crate::tia::color::Colors;
use crate::tia::counter::Counter;

pub struct Ball {
    colors: Rc<RefCell<Colors>>,

    enabled: bool,
    size: usize,
    hmove_offset: u8,
    ctr: Counter,
    vdel: bool,
    old_value: bool,

    bit_copies_written: usize,
    graphic_bit: Option<isize>,
    graphic_bit_value: bool,
}

impl Ball {
    pub fn new_ball(colors: Rc<RefCell<Colors>>) -> Self {
        Self {
            colors: colors,

            enabled: false,
            size: 0,
            hmove_offset: 0,
            ctr: Counter::new_counter(40, 39),
            vdel: false,
            old_value: false,

            bit_copies_written: 0,
            graphic_bit: None,
            graphic_bit_value: false,
        }
    }

    pub fn enabled(&self) -> bool { self.enabled }
    pub fn set_enabled(&mut self, v: bool) { self.enabled = v }
    pub fn set_hmove_value(&mut self, v: u8) { self.hmove_offset = v }
    pub fn set_vdel(&mut self, v: bool) { self.vdel = v }
    pub fn use_old_value(&mut self) { self.old_value = self.enabled }
    pub fn set_size(&mut self, size: usize) { self.size = size }
    pub fn hmclr(&mut self) { self.hmove_offset = 0 }
    pub fn reset(&mut self) {
        self.ctr.reset();
        self.graphic_bit = Some(-4);
        self.bit_copies_written = 0;
        self.graphic_bit_value = false;
    }

    pub fn tick_visible(&mut self) {
        self.tick_graphic_circuit();
        if self.ctr.clock() && self.ctr.value() == 39 {
            self.graphic_bit = Some(-4);
            self.bit_copies_written = 0;
        }
    }

    pub fn tick_hblank(&mut self) {
        let delayed = self.ctr.apply_hmove();

        if delayed {
            self.tick_graphic_circuit();
            if self.ctr.clock() && self.ctr.value() == 39 {
                self.graphic_bit = Some(-4);
                self.bit_copies_written = 0;
            }
        }
    }

    fn tick_graphic_circuit(&mut self) {
        if let Some(v) = self.graphic_bit {
            let mut new_v = v;

            if v >= 0 && v < 8 {
                self.graphic_bit_value = true;

                self.bit_copies_written += 1;
                if self.bit_copies_written == self.size {
                    new_v = v + 1;
                    self.bit_copies_written = 0;
                }
            } else {
                new_v = v + 1;
            }

            if new_v == 1 {
                self.graphic_bit = None;
            } else {
                self.graphic_bit = Some(new_v);
            }
        } else {
            self.graphic_bit_value = false;
        }
    }

    pub fn start_hmove(&mut self) {
        self.ctr.start_hmove(self.hmove_offset);
        self.tick_graphic_circuit();
    }

    pub fn get_color(&self) -> Option<u8> {
        if self.enabled && self.graphic_bit_value {
        //if self.enabled && self.ctr.internal_value < self.size as u8 {
            return Some(self.colors.borrow().colupf());
        }

        return None;
    }
}
