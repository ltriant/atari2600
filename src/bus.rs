use std::cell::RefCell;
use std::io;
use std::fs::File;
use std::rc::Rc;

use crate::riot::RIOT;
use crate::tia::TIA;

pub trait Bus {
    fn read(&mut self, _address: u16) -> u8 { 0 }
    fn write(&mut self, _address: u16, _val: u8) { }
    fn save(&self, _output: &mut File) -> io::Result<()> { Ok(()) }
    fn load(&mut self, _input: &mut File) -> io::Result<()> { Ok(()) }
}

pub struct AtariBus {
    rom: Vec<u8>,
    tia: Rc<RefCell<TIA>>,
    riot: Rc<RefCell<RIOT>>,
}

impl AtariBus {
    pub fn new(tia: Rc<RefCell<TIA>>, riot: Rc<RefCell<RIOT>>, rom: Vec<u8>) -> Self {
        Self {
            rom: rom,
            tia: tia,
            riot: riot,
        }
    }
}

impl Bus for AtariBus {
    fn read(&mut self, address: u16) -> u8 {
        // https://problemkaputt.de/2k6specs.htm#memorymirrors

        let a12 = (address & 0b0001_0000_0000_0000) != 0;
        let a9  = (address & 0b0000_0010_0000_0000) != 0;
        let a7  = (address & 0b0000_0000_1000_0000) != 0;

        match (a12, a9, a7) {
            // Cartridge memory is selected by A12=1
            (true, _, _)         => self.rom[address as usize & 0xfff],
            // PIA I/O is selected by A12=0, A9=1, A7=1
            (false, true, true)  => self.riot.borrow_mut().read(address & 0x2ff),
            // PIA RAM is selected by A12=0, A9=0, A7=1
            (false, false, true) => self.riot.borrow_mut().read(address & 0x7f),
            // The TIA chip is addressed by A12=0, A7=0
            (false, _, false)    => self.tia.borrow_mut().read((address & 0x0f) | 0x30),
        }
    }

    fn write(&mut self, address: u16, val: u8) {
        // https://problemkaputt.de/2k6specs.htm#memorymirrors

        let a12 = (address & 0b0001_0000_0000_0000) != 0;
        let a9  = (address & 0b0000_0010_0000_0000) != 0;
        let a7  = (address & 0b0000_0000_1000_0000) != 0;

        match (a12, a9, a7) {
            // Cartridge memory is selected by A12=1
            (true, _, _)         => { self.rom[address as usize & 0xfff] = val },
            // PIA I/O is selected by A12=0, A9=1, A7=1
            (false, true, true)  => self.riot.borrow_mut().write(address & 0x2ff, val),
            // PIA RAM is selected by A12=0, A9=0, A7=1
            (false, false, true) => self.riot.borrow_mut().write(address & 0x7f, val),
            // The TIA chip is addressed by A12=0, A7=0
            (false, _, false)    => self.tia.borrow_mut().write(address & 0x3f, val),
        }
    }
}
