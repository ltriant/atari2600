pub struct Counter {
    period: u8,
    reset_value: u8,
    pub internal_value: u8,

    last_value: u8,
    ticks_added: u8,
    movement_required: bool,
}

fn hmove_value(v: u8) -> u8 {
    let nibble = v >> 4;

    if nibble < 8 {
        nibble + 8
    } else {
        nibble - 8
    }
}

impl Counter {
    pub fn new_counter(period: u8, reset_value: u8) -> Self {
        Self {
            period: period,
            reset_value: reset_value,
            internal_value: 0,

            last_value: 0,
            ticks_added: 0,
            movement_required: false,
        }
    }

    pub fn reset(&mut self) {
        self.internal_value = self.reset_value * 4;
    }

    pub fn value(&self) -> u8 {
        self.internal_value / 4
    }

    pub fn reset_to(&mut self, v: u8) {
        self.internal_value = v;
    }

    pub fn clock(&mut self) -> bool {
        self.internal_value += 1;
        self.internal_value %= self.period * 4;

        if self.last_value != self.value() {
            self.last_value = self.value();
            return true;
        } else {
            return false;
        }
    }

    pub fn start_hmove(&mut self, hm_val: u8) {
        self.ticks_added = 0;
        self.movement_required = hmove_value(hm_val) != 0;
    }

    pub fn apply_hmove(&mut self, hm_val: u8) -> (bool, bool) {
        if !self.movement_required {
            return (false, false);
        }

        let clocked = self.clock();
        self.ticks_added += 1;
        self.movement_required = self.ticks_added != hmove_value(hm_val);

        return (true, clocked);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clocking() {
        let mut ctr = Counter::new_counter(40, 0);

        assert_eq!(ctr.value(), 0);

        let mut clocked = ctr.clock();
        assert!(!clocked);
        assert_eq!(ctr.value(), 0);

        clocked = ctr.clock();
        assert!(!clocked);
        assert_eq!(ctr.value(), 0);

        clocked = ctr.clock();
        assert!(!clocked);
        assert_eq!(ctr.value(), 0);

        clocked = ctr.clock();
        assert!(clocked);
        assert_eq!(ctr.value(), 1);

        for i in 1 ..= 152 {
            clocked = ctr.clock();

            if i % 4 == 0 {
                assert!(clocked);
            }
            else {
                assert!(!clocked);
            }
        }

        assert_eq!(ctr.value(), 39);

        ctr.clock();
        assert_eq!(ctr.value(), 39);
        ctr.clock();
        assert_eq!(ctr.value(), 39);
        ctr.clock();
        assert_eq!(ctr.value(), 39);
        let clocked = ctr.clock();

        assert!(clocked);
        assert_eq!(ctr.value(), 0);
    }

    #[test]
    fn test_scanline_counting() {
        // p0, p0, m0, and m1 use a 40 clock counter, so they should reset back to 0 after a full
        // scanline has finished rendering.
        let mut ctr = Counter::new_counter(40, 0);

        assert_eq!(ctr.value(), 0);

        for _ in 0 .. 160 {
            ctr.clock();
        }

        assert_eq!(ctr.value(), 0);
    }
}
