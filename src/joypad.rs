use sdl2::keyboard::Keycode;

pub struct Joypad {
    keys: Vec<Keycode>,
    joypad_1_last_write: u8,
    joypad_1_read_ptr: u8
}

impl Joypad {
    pub fn new() -> Joypad { 
        Joypad { keys: Vec::new(), joypad_1_last_write: 0,
            joypad_1_read_ptr: 0 }
    }
    
    pub fn update_keys(&mut self, keys: Vec<Keycode>) {
        self.keys = keys;
    }
    
    pub fn joypad_1_read(&mut self) -> u8 {
        let result = 
            match self.joypad_1_read_ptr {
                1 => self.keys.contains(&Keycode::Z),  // A
                2 => self.keys.contains(&Keycode::X),  // B
                3 => self.keys.contains(&Keycode::A),  // Select
                4 => self.keys.contains(&Keycode::S),  // Start
                5 => self.keys.contains(&Keycode::Up),
                6 => self.keys.contains(&Keycode::Down),
                7 => self.keys.contains(&Keycode::Left),
                8 => self.keys.contains(&Keycode::Right),                
                _ => false
            };
        self.joypad_1_read_ptr += 1;
        return if result {1} else {0};
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
    
    pub fn joypad_2_write(&mut self, _: u8) {
    }
}