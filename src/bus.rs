use std::cell::RefCell;
use std::io;
use std::fs::File;
use std::rc::Rc;

use crate::pia::PIA;
use crate::tia::TIA;

pub trait Bus {
    fn read(&mut self, _address: u16) -> u8 { 0 }
    fn write(&mut self, _address: u16, _val: u8) { }
    fn save(&self, _output: &mut File) -> io::Result<()> { Ok(()) }
    fn load(&mut self, _input: &mut File) -> io::Result<()> { Ok(()) }
}

pub struct AtariBus {
    ram: [u8; 128],
    rom: Vec<u8>,
    tia: Rc<RefCell<TIA>>,
    pia: Rc<RefCell<PIA>>,
}

impl AtariBus {
    pub fn new_bus(tia: Rc<RefCell<TIA>>, pia: Rc<RefCell<PIA>>, rom: Vec<u8>) -> Self {
        Self {
            ram: [0; 128],
            rom: rom,
            tia: tia,
            pia: pia,
        }
    }
}

impl Bus for AtariBus {
    fn read(&mut self, address: u16) -> u8 {
        match address {
            // TIA registers
            0x0000 ..= 0x007f => self.tia.borrow_mut().read(address),

            // RAM
            0x0080 ..= 0x00ff => self.ram[address as usize - 0x80],

            // PIA ports and timer
            0x0280 ..= 0x0297 => self.pia.borrow_mut().read(address),

            // Cartridge ROM
            0x1000 ..= 0x1fff => self.rom[address as usize & 0xfff],

            _ => 0,
        }
    }

    fn write(&mut self, address: u16, val: u8) {
        match address {
            // TIA registers
            0x0000 ..= 0x007f => self.tia.borrow_mut().write(address, val),

            // RAM
            0x0080 ..= 0x00ff => { self.ram[address as usize - 0x80] = val },

            // PIA ports and timer
            0x0280 ..= 0x0297 => self.pia.borrow_mut().write(address, val),

            _ => { },
        }
    }
}
