use std::rc::Rc;
use std::cell::RefCell;

use crate::tia::TIA;

pub struct Debugger {
    tia: Rc<RefCell<TIA>>,
    enabled: bool,

    next_frame: bool,
}

impl Debugger {
    pub fn new(tia: Rc<RefCell<TIA>>) -> Self {
        Self {
            tia: tia,
            enabled: false,

            next_frame: false,
        }
    }

    // Enable/disable the debugger
    pub fn toggle(&mut self) {
        self.enabled = ! self.enabled;

        println!("Debugging is now: {}", if self.enabled { "on" } else { "off" });
    }

    pub fn debug(&self) {
        if !self.enabled { return }
        self.tia.borrow().debug();
    }

    // Controlling frame stepping
    pub fn next_frame(&self) -> bool {
        if !self.enabled { return true }

        self.next_frame
    }

    pub fn step_frame(&mut self) {
        self.next_frame = true;
    }

    pub fn end_frame(&mut self) {
        self.next_frame = false;
    }
}
