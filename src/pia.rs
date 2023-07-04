use crate::bus::Bus;

pub struct PIA {
}

impl PIA {
    pub fn new_pia() -> Self {
        Self{}
    }
}

impl Bus for PIA {
    fn read(&mut self, address: u16) -> u8 {
        match address {
            // SWCHA   11111111  Port A; input or output  (read or write)
            0x0280 => 0,

            // SWACNT  11111111  Port A DDR, 0= input, 1=output
            0x0281 => 0,

            // SWCHB   11111111  Port B; console switches (read only)
            0x0282 => 0b0000_1000,

            // SWBCNT  11111111  Port B DDR (hardwired as input)
            0x0283 => 0,

            // INTIM   11111111  Timer output (read only)
            0x0284 => 0,

            // INSTAT  11......  Timer Status (read only, undocumented)
            0x0285 => 0,

            _ => 0,
        }
    }

    fn write(&mut self, address: u16, val: u8) {
        match address {
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
