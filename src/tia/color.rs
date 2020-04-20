pub struct Colors {
    colup0: u8,
    colup1: u8,
    colupf: u8,
    colubk: u8,
}

impl Colors {
    pub fn new_colors() -> Self {
        Self {
            colup0: 0,
            colup1: 0,
            colupf: 0,
            colubk: 0,
        }
    }

    pub fn set_colup0(&mut self, val: u8) {
        self.colup0 = val;
    }

    pub fn set_colup1(&mut self, val: u8) {
        self.colup1 = val;
    }

    pub fn set_colupf(&mut self, val: u8) {
        self.colupf = val;
    }

    pub fn set_colubk(&mut self, val: u8) {
        self.colubk = val;
    }

    pub fn colup0(&self) -> u8 {
        self.colup0
    }

    pub fn colup1(&self) -> u8 {
        self.colup1
    }

    pub fn colupf(&self) -> u8 {
        self.colupf
    }

    pub fn colubk(&self) -> u8 {
        self.colubk
    }
}
