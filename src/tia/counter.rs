pub struct Counter {
    period: u8,
    reset_value: u8,
    pub internal_value: u8,

    last_value: u8,
    clocks_to_add: u8,
}

fn hmove_value(v: u8) -> u8 {
    // Signed Motion Value (-8..-1=Right, 0=No motion, +1..+7=Left)
    if v < 8 {
        v + 8
    } else {
        v - 8
    }
}

impl Counter {
    pub fn new_counter(period: u8, reset_value: u8) -> Self {
        Self {
            period: period,
            reset_value: reset_value,
            internal_value: 0,

            last_value: 0,
            clocks_to_add: 0,
        }
    }

    pub fn reset(&mut self) {
        self.internal_value = self.reset_value * 4;
    }

    pub fn value(&self) -> u8 {
        self.internal_value / 4
    }

    pub fn set_value(&mut self, val: u8) {
        self.internal_value = val * 4;
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
        self.clocks_to_add = hmove_value(hm_val);
        if hm_val != 0 {
            debug!("adding clocks: {} ({})", self.clocks_to_add, hm_val >> 4);
        }
    }

    pub fn apply_hmove(&mut self) -> bool {
        if self.clocks_to_add != 0 {
            self.clocks_to_add -= 1;

            return self.clocks_to_add != 0;
        }

        return false;
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
}
