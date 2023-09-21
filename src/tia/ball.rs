use std::rc::Rc;
use std::cell::RefCell;

use crate::tia::color::Colors;
use crate::tia::counter::Counter;

const INIT_DELAY: isize = 4;
const GRAPHIC_SIZE: isize = 1;

pub struct Ball {
    colors: Rc<RefCell<Colors>>,

    hmove_offset: u8,
    ctr: Counter,

    enabled: bool,
    // The ball sizee from the CTRLPF register
    nusiz: usize,

    // The VDELBL register
    vdel: bool,
    old_value: bool,

    // Graphics Scan Counter
    graphic_bit_idx: Option<isize>,
    graphic_bit_copies_written: usize,
    graphic_bit_value: Option<bool>,
}

impl Ball {
    pub fn new(colors: Rc<RefCell<Colors>>) -> Self {
        Self {
            colors: colors,

            hmove_offset: 0,
            ctr: Counter::new(40, 39),

            enabled: false,
            nusiz: 1,

            vdel: false,
            old_value: false,

            graphic_bit_idx: None,
            graphic_bit_copies_written: 0,
            graphic_bit_value: None,
        }
    }

    pub fn set_enabled(&mut self, v: bool) { self.enabled = v }
    pub fn set_hmove_value(&mut self, v: u8) { self.hmove_offset = v }
    pub fn set_vdel(&mut self, v: bool) { self.vdel = v }
    pub fn set_vdel_value(&mut self) { self.old_value = self.enabled }
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

    fn size(&self) -> usize { self.nusiz }
    fn pixel_bit(&self) -> bool {
        if self.vdel {
            self.old_value
        } else {
            self.enabled
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

    fn should_draw_copy(&self) -> bool { false }

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
            return Some(self.colors.borrow().colupf());
        }

        return None;
    }
}
