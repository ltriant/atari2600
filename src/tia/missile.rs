use std::rc::Rc;
use std::cell::RefCell;

use crate::tia::color::Colors;

pub struct Missile {
    colors: Rc<RefCell<Colors>>,

    x: usize,
    enabled: bool,
    hmove: usize,
}

impl Missile {
    pub fn new_missile(colors: Rc<RefCell<Colors>>) -> Self {
        Self {
            colors: colors,

            x: 0,
            enabled: false,
            hmove: 0,
        }
    }

    pub fn set_enabled(&mut self, en: bool) {
        self.enabled = en;
    }
}
