#[derive(PartialEq, Debug)]
pub enum JoyButton {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    Select,
    Start,
}
pub struct Joypad {
    keys: Vec<JoyButton>,
    joypad_1_last_write: u8,
    joypad_1_read_ptr: u8,
}

impl Joypad {
    pub fn new() -> Joypad {
        Joypad {
            keys: Vec::new(),
            joypad_1_last_write: 0,
            joypad_1_read_ptr: 0,
        }
    }

    pub fn update_keys(&mut self, keys: Vec<JoyButton>) {
        self.keys = keys;
    }

    pub fn joypad_1_read(&mut self) -> u8 {
        let result = match self.joypad_1_read_ptr {
            1 => self.keys.contains(&JoyButton::A),      // A
            2 => self.keys.contains(&JoyButton::B),      // B
            3 => self.keys.contains(&JoyButton::Select), // Select
            4 => self.keys.contains(&JoyButton::Start),  // Start
            5 => self.keys.contains(&JoyButton::Up),
            6 => self.keys.contains(&JoyButton::Down) && !self.keys.contains(&JoyButton::Up),
            7 => self.keys.contains(&JoyButton::Left),
            8 => self.keys.contains(&JoyButton::Right) && !self.keys.contains(&JoyButton::Left),
            _ => false,
        };
        self.joypad_1_read_ptr += 1;
        return if result { 1 } else { 0 };
    }

    pub fn joypad_1_write(&mut self, data: u8) {
        if (data == 0) && (self.joypad_1_last_write == 1) {
            self.joypad_1_read_ptr = 1;
        }
        self.joypad_1_last_write = data;
    }

    pub fn joypad_2_read(&self) -> u8 {
        return 0;
    }

    pub fn joypad_2_write(&mut self, _: u8) {}
}
