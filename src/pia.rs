use crate::bus::Bus;

// Actually a MOS6532 chip
pub struct PIA {
    ram: [u8; 128],

    swcha: u8,
    swacnt: u8,
    swchb: u8,
    swbcnt: u8,
    intim: u8,
    instat: u8,
}

impl PIA {
    pub fn new_pia() -> Self {
        Self {
            ram: [0; 128],

            swcha: 0,
            swacnt: 0,
            swchb: 0,
            swbcnt: 0,
            intim: 0,
            instat: 0,
        }
    }
}

impl Bus for PIA {
    fn read(&mut self, address: u16) -> u8 {
        match address {
            // RAM
            0x0080 ..= 0x00ff => self.ram[address as usize - 0x80],

            // SWCHA   11111111  Port A; input or output  (read or write)
            0x0280 => self.swcha,

            // SWACNT  11111111  Port A DDR, 0= input, 1=output
            0x0281 => self.swacnt,

            // SWCHB   11111111  Port B; console switches (read only)
            0x0282 => 0b0000_1000,

            // SWBCNT  11111111  Port B DDR (hardwired as input)
            0x0283 => self.swbcnt,

            // INTIM   11111111  Timer output (read only)
            0x0284 => self.intim,

            // INSTAT  11......  Timer Status (read only, undocumented)
            0x0285 => self.instat,

            _ => 0,
        }
    }

    fn write(&mut self, address: u16, val: u8) {
        match address {
            // RAM
            0x0080 ..= 0x00ff => { self.ram[address as usize - 0x80] = val },

            // TIM1T   11111111  set 1 clock interval (838 nsec/interval)
            0x0294 => {},

            // TIM8T   11111111  set 8 clock interval (6.7 usec/interval)
            0x0295 => {},

            // TIM64T  11111111  set 64 clock interval (53.6 usec/interval)
            0x0296 => {},

            // T1024T  11111111  set 1024 clock interval (858.2 usec/interval)
            0x0297 => {},

            _ => { },
        }
    }
}
