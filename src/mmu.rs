use cart::Cart;
use ppu::Ppu;
use joypad::Joypad;

pub mod mirroring {
    pub const HORIZONTAL  : u8 = 1;
    pub const VERTICAL    : u8 = 2;
    pub const FOUR_SCREEN : u8 = 3;
    pub const ONE_SCREEN  : u8 = 4;
}

pub struct Mmu {
    active_prg_page: Vec<usize>,
    active_chr_page: Vec<usize>,
    scratch_ram: Vec<u8>,
    save_ram: Vec<u8>,
    is_save_ram_readonly: bool,
    
    //Mapper-specific registers
    map1_reg_8000_bit: usize,
    map1_reg_a000_bit: usize,
    map1_reg_c000_bit: usize,
    map1_reg_e000_bit: usize,
    map1_reg_8000_val: usize,
    map1_reg_a000_val: usize,
    map1_reg_c000_val: usize,
    map1_reg_e000_val: usize,
    
    map1_mirroring_flag: u8,
    map1_one_page_mirroring: u8,
    map1_prg_switch_area: u8,
    map1_prg_switch_size: u8,
    map1_vrom_switch_size: u8,
    
    pub joypad: Joypad,
    
    cart: Cart
}

impl Mmu {
    pub fn new(cart: Cart) -> Mmu {
        let mut active_prg_page: Vec<usize> = Vec::new();
        let mut active_chr_page: Vec<usize> = Vec::new();
        for x in 0..8 {
            active_prg_page.push(x);
            active_chr_page.push(x);
        }
        //println!("Switch 32k prg page: 0");
        //println!("Switch 8k chr page: 0");
        
        let scratch_ram : Vec<u8> = vec![0; 0x800];
        
        let save_ram : Vec<u8> = vec![0; 0x2000];
                
        // Start with the default mirroring loaded from the cart, 
        // but may change via the mapper

        Mmu { 
            active_prg_page: active_prg_page,
            active_chr_page: active_chr_page,
            scratch_ram: scratch_ram,
            save_ram: save_ram,
            is_save_ram_readonly: false,
            
            //Mapper-specific registers
            map1_reg_8000_bit: 0,
            map1_reg_a000_bit: 0,
            map1_reg_c000_bit: 0,
            map1_reg_e000_bit: 0,
            map1_reg_8000_val: 0,
            map1_reg_a000_val: 0,
            map1_reg_c000_val: 0,
            map1_reg_e000_val: 0,
            
            map1_mirroring_flag: 0,
            map1_one_page_mirroring: 0,
            map1_prg_switch_area: 0,
            map1_prg_switch_size: 0,
            map1_vrom_switch_size: 0,
            
            joypad: Joypad::new(),
            
            cart: cart
        }
    }
    
    pub fn setup_defaults(&mut self) {
        if self.cart.mapper == 1 {
            self.map1_reg_8000_bit = 0;
            self.map1_reg_8000_val = 0;
            self.map1_mirroring_flag = 0;
            self.map1_one_page_mirroring = 1;
            self.map1_prg_switch_area = 1;
            self.map1_prg_switch_size = 1;
            self.map1_vrom_switch_size = 0;
            
            let num_prg = self.cart.num_prg_pages;
            self.switch_16k_prg_page((num_prg - 1) * 4, 1);
        }
        else if self.cart.mapper == 2{
            let num_prg = self.cart.num_prg_pages;
            self.switch_16k_prg_page((num_prg - 1) * 4, 1);
        }
    }
    
    pub fn read_u8(&mut self, ppu: &mut Ppu, address: u16) -> u8 {
        match address {
            0x0000...0x07FF => self.scratch_ram[address as usize],
            0x0800...0x0FFF => self.scratch_ram[(address as usize) - 0x0800],
            0x1000...0x17FF => self.scratch_ram[(address as usize) - 0x1000],
            0x1800...0x1FFF => self.scratch_ram[(address as usize) - 0x1800],
            0x2002          => ppu.status_reg_read(),
            0x2004          => ppu.sprite_ram_io_reg_read(),
            0x2007          => ppu.vram_io_reg_read(self),
            0x4016          => self.joypad.joypad_1_read(),
            0x4017          => self.joypad.joypad_2_read(),
            0x6000...0x7FFF => self.save_ram[(address as usize) - 0x6000],
            0x8000...0x8FFF => self.cart.prg_rom[self.active_prg_page[0]][(address as usize) - 0x8000],
            0x9000...0x9FFF => self.cart.prg_rom[self.active_prg_page[1]][(address as usize) - 0x9000],
            0xA000...0xAFFF => self.cart.prg_rom[self.active_prg_page[2]][(address as usize) - 0xA000],
            0xB000...0xBFFF => self.cart.prg_rom[self.active_prg_page[3]][(address as usize) - 0xB000],
            0xC000...0xCFFF => self.cart.prg_rom[self.active_prg_page[4]][(address as usize) - 0xC000],
            0xD000...0xDFFF => self.cart.prg_rom[self.active_prg_page[5]][(address as usize) - 0xD000],
            0xE000...0xEFFF => self.cart.prg_rom[self.active_prg_page[6]][(address as usize) - 0xE000],
            0xF000...0xFFFF => self.cart.prg_rom[self.active_prg_page[7]][(address as usize) - 0xF000],
            _ => {println!("Unknown read: {0:x}", address); 0}
        }
    }
    
    pub fn read_u16(&mut self, ppu: &mut Ppu, address: u16) -> u16 {
        let read_1 = self.read_u8(ppu, address);
        let read_2 = self.read_u8(ppu, address+1);
        
        ((read_2 as u16) << 8) + (read_1 as u16)
    }
    
    pub fn write_u8(&mut self, ppu: &mut Ppu, address: u16, data: u8) {
        match address {
            0x0000...0x07FF => self.scratch_ram[address as usize]            = data,
            0x0800...0x0FFF => self.scratch_ram[(address as usize) - 0x0800] = data,
            0x1000...0x17FF => self.scratch_ram[(address as usize) - 0x1000] = data,
            0x1800...0x1FFF => self.scratch_ram[(address as usize) - 0x1800] = data,
            0x2000          => ppu.control_reg_1_write(data),
            0x2001          => ppu.control_reg_2_write(data),
            0x2003          => ppu.sprite_ram_addr_reg_write(data),
            0x2004          => ppu.sprite_ram_io_reg_write(data),
            0x2005          => ppu.vram_addr_reg_1_write(data),
            0x2006          => ppu.vram_addr_reg_2_write(data),
            0x2007          => ppu.vram_io_reg_write(self, data),
            0x4000...0x4013 => {}, // Sound signal write 
            0x4014          => ppu.sprite_ram_dma_begin(self, data),
            0x4015          => {}, // Sound signal write
            0x4016          => self.joypad.joypad_1_write(data),
            0x4017          => self.joypad.joypad_2_write(data),
            0x6000...0x7FFF => 
                if !self.is_save_ram_readonly { 
                    self.save_ram[(address as usize) - 0x6000] = data;
                },
            0x8000...0xFFFF => self.write_prg_rom(address, data),
            _ => println!("Unknown write of {0:x} to {1:x}", data, address)
        }
    }
    
    pub fn write_chr_rom(&mut self, addr: usize, data:u8) {
        if self.cart.is_vram {
            match addr {
                0x0000...0x03ff => self.cart.chr_rom[self.active_chr_page[0]][addr] = data,
                0x0400...0x07ff => self.cart.chr_rom[self.active_chr_page[1]][addr - 0x400] = data,
                0x0800...0x0bff => self.cart.chr_rom[self.active_chr_page[2]][addr - 0x800] = data,
                0x0c00...0x0fff => self.cart.chr_rom[self.active_chr_page[3]][addr - 0xc00] = data,
                0x1000...0x13ff => self.cart.chr_rom[self.active_chr_page[4]][addr - 0x1000] = data,
                0x1400...0x17ff => self.cart.chr_rom[self.active_chr_page[5]][addr - 0x1400] = data,
                0x1800...0x1bff => self.cart.chr_rom[self.active_chr_page[6]][addr - 0x1800] = data,
                0x1c00...0x1fff => self.cart.chr_rom[self.active_chr_page[7]][addr - 0x1c00] = data,
                _ => {}
            }
        }
    }
    
    pub fn read_chr_rom(&self, addr: usize) -> u8 {
        if addr < 0x400 {
            return self.cart.chr_rom[self.active_chr_page[0]][addr];
        }
        else if addr < 0x800 {
            return self.cart.chr_rom[self.active_chr_page[1]][addr - 0x400];
        }
        else if addr < 0xc00 {
            return self.cart.chr_rom[self.active_chr_page[2]][addr - 0x800];
        }
        else if addr < 0x1000 {
            return self.cart.chr_rom[self.active_chr_page[3]][addr - 0xc00];
        }
        else if addr < 0x1400 {
            return self.cart.chr_rom[self.active_chr_page[4]][addr - 0x1000];
        }
        else if addr < 0x1800 {
            return self.cart.chr_rom[self.active_chr_page[5]][addr - 0x1400];
        }
        else if addr < 0x1c00 {
            return self.cart.chr_rom[self.active_chr_page[6]][addr - 0x1800];
        }
        else {
            return self.cart.chr_rom[self.active_chr_page[7]][addr - 0x1c00];
        }
    }
    
    fn switch_32k_prg_page(&mut self, start: usize) {
        let start_page = match self.cart.num_prg_pages {
            2 => start & 0x7,
            4 => start & 0xf,
            8 => start & 0x1f,
            16 => start & 0x3f,
            32 => start & 0x7f,
            _ => {println!("Error: bad 32k switch"); 0}
        };
        
        //println!("Switch 32k prg page: {}", start_page);

        for i in 0..8 {
            self.active_prg_page[i] = start_page + i;
        }
    }
    
    fn switch_16k_prg_page(&mut self, start: usize, area: usize) {
        let start_page = match self.cart.num_prg_pages {
            2 => start & 0x7,
            4 => start & 0xf,
            8 => start & 0x1f,
            16 => start & 0x3f,
            32 => start & 0x7f,
            _ => {println!("Error: bad 16k switch"); 0}
        };
        
        //println!("Switch 16k prg page: {} {}", start_page, area);
        
        for i in 0..4 {
            self.active_prg_page[4 * area + i] = start_page + i;
        }
    }
    
    /*
    fn switch_8k_prg_page(&mut self, start: usize, area: usize) {
        let start_page = match self.cart.num_prg_pages {
            2 => start & 0x7,
            4 => start & 0xf,
            8 => start & 0x1f,
            16 => start & 0x3f,
            32 => start & 0x7f,
            _ => 0
        };
        
        for i in 0..2 {
            self.active_prg_page[i + 2 * area] = start_page + i;
        }
    }
    */
    
    fn switch_8k_chr_page(&mut self, start: usize) {
        let start_page = match self.cart.num_chr_pages {
            2 => start & 0xf,
            4 => start & 0x1f,
            8 => start & 0x3f,
            16 => start & 0x7f,
            32 => start & 0xff,
            _ => {println!("Error: bad 8k chr switch"); 0}
        };

        //println!("Switch 8k chr page: {}", start);
        
        for i in 0..8 {
            self.active_chr_page[i] = start_page + i;
        }              
    }
        
    fn switch_4k_chr_page(&mut self, start: usize, area: usize) {
        let start_page = match self.cart.num_chr_pages {
            2 => start & 0xf,
            4 => start & 0x1f,
            8 => start & 0x3f,
            16 => start & 0x7f,
            32 => start & 0xff,
            _ => {println!("Error: bad 4k chr switch"); 0}
        };

        //println!("Switch 8k chr page: {} {}", start, area);
        
        for i in 0..4 {
            self.active_chr_page[i + 4 * area] = start_page + i;
        }              
    }
    
    /*
    fn switch_2k_chr_page(&mut self, start: usize, area: usize) {
        let start_page = match self.cart.num_chr_pages {
            2 => start & 0xf,
            4 => start & 0x1f,
            8 => start & 0x3f,
            16 => start & 0x7f,
            32 => start & 0xff,
            _ => 0
        };
        
        for i in 0..2 {
            self.active_chr_page[i + 2 * area] = start_page + i;
        }              
    }
        
    fn switch_1k_chr_page(&mut self, start: usize, area: usize) {
        let start_page = match self.cart.num_chr_pages {
            2 => start & 0xf,
            4 => start & 0x1f,
            8 => start & 0x3f,
            16 => start & 0x7f,
            32 => start & 0xff,
            _ => 0
        };
        
        self.active_chr_page[area] = start_page;              
    }
    */
    
    fn write_prg_rom(&mut self, addr: u16, data: u8) {
        //println!("Write prg rom: {0:02x} <- {1:x}", addr, data);
        if self.cart.mapper == 1 {
            if (addr >= 0x8000) && (addr <= 0x9fff) {
                if (data & 0x80) == 0x80 {
                    //reset
                    self.map1_reg_8000_bit = 0;
                    self.map1_reg_8000_val = 0;
                    self.map1_mirroring_flag = 0;
                    self.map1_one_page_mirroring = 1;
                    self.map1_prg_switch_area = 1;
                    self.map1_prg_switch_size = 1;
                    self.map1_vrom_switch_size = 0;
                }
                else {
                    self.map1_reg_8000_val += ((data & 0x1) << self.map1_reg_8000_bit) as usize;
                    self.map1_reg_8000_bit += 1;
                    if self.map1_reg_8000_bit == 5 {
                        self.map1_mirroring_flag = (self.map1_reg_8000_val & 1) as u8;
                        if self.map1_mirroring_flag == 0 {
                            self.cart.mirroring = mirroring::VERTICAL;
                        }
                        else {
                            self.cart.mirroring = mirroring::HORIZONTAL;
                        }
                        self.map1_one_page_mirroring = ((self.map1_reg_8000_val >> 1) & 1) as u8;
                        
                        if self.map1_one_page_mirroring == 0 {
                            self.cart.mirroring = mirroring::ONE_SCREEN;
                            self.cart.mirroring_base = 0x2000;
                        }
                        
                        self.map1_prg_switch_area = ((self.map1_reg_8000_val >> 2) & 1) as u8;
                        self.map1_prg_switch_size = ((self.map1_reg_8000_val >> 3) & 1) as u8;
                        self.map1_vrom_switch_size = ((self.map1_reg_8000_val >> 4) & 1) as u8;
                        
                        self.map1_reg_8000_bit = 0;
                        self.map1_reg_8000_val = 0;
                        self.map1_reg_a000_bit = 0;
                        self.map1_reg_a000_val = 0;
                        self.map1_reg_c000_bit = 0;
                        self.map1_reg_c000_val = 0;
                        self.map1_reg_e000_bit = 0;
                        self.map1_reg_e000_val = 0;
                    }                    
                }
            }
            else if (addr >= 0xa000) && (addr <= 0xbfff) {
                if (data & 0x80) == 0x80 {
                    self.map1_reg_a000_bit = 0;
                    self.map1_reg_a000_val = 0;
                }
                else {
                    self.map1_reg_a000_val += ((data & 0x1) << self.map1_reg_a000_bit) as usize;
                    self.map1_reg_a000_bit += 1;
                    
                    if self.map1_reg_a000_bit == 5 {
                        let val = self.map1_reg_a000_val;
                        if self.cart.num_chr_pages > 0 {
                            if self.map1_vrom_switch_size == 1 {
                                self.switch_4k_chr_page(val * 4, 0);
                            }
                            else {
                                self.switch_8k_chr_page((val >> 1) * 8);
                            }
                        }
                        self.map1_reg_a000_bit = 0;
                        self.map1_reg_a000_val = 0;
                    }
                }
            }
            else if (addr >= 0xc000) && (addr <= 0xdfff) {
                if (data & 0x80) == 0x80 {
                    self.map1_reg_c000_bit = 0;
                    self.map1_reg_c000_val = 0;
                }
                else {
                    self.map1_reg_c000_val += ((data & 0x1) << self.map1_reg_c000_bit) as usize;
                    self.map1_reg_c000_bit += 1;
                    
                    if self.map1_reg_c000_bit == 5 {
                        let val = self.map1_reg_c000_val;
                        if self.cart.num_chr_pages > 0 {
                            if self.map1_vrom_switch_size == 1 {
                                self.switch_4k_chr_page(val * 4, 1);
                            }
                        }
                        self.map1_reg_c000_bit = 0;
                        self.map1_reg_c000_val = 0;
                    }
                }
            }
            else if addr >= 0xe000 {
                if (data & 0x80) == 0x80 {
                    self.map1_reg_8000_bit = 0;
                    self.map1_reg_8000_val = 0;
                    self.map1_reg_a000_bit = 0;
                    self.map1_reg_a000_val = 0;
                    self.map1_reg_c000_bit = 0;
                    self.map1_reg_c000_val = 0;
                    self.map1_reg_e000_bit = 0;
                    self.map1_reg_e000_val = 0;                
                }
                else {
                    self.map1_reg_e000_val += ((data & 0x1) << self.map1_reg_e000_bit) as usize;
                    self.map1_reg_e000_bit += 1;
                    
                    if self.map1_reg_e000_bit == 5 {
                        let val = self.map1_reg_e000_val;
                        let num_prg = self.cart.num_prg_pages;
                        if self.map1_prg_switch_size == 1 {
                            if self.map1_prg_switch_area == 1 {
                                self.switch_16k_prg_page(val * 4, 0);
                                self.switch_16k_prg_page((num_prg - 1) * 4, 1);
                            }
                            else {
                                self.switch_16k_prg_page(val * 4, 1);
                                self.switch_16k_prg_page(0, 0);
                            }
                        }
                        else {
                            self.switch_32k_prg_page((val >> 1) * 8);
                        }
                        self.map1_reg_e000_bit = 0;
                        self.map1_reg_e000_val = 0;
                    }
                }                    
            }
        }
        else if self.cart.mapper == 2 {
            if addr >= 0x8000 {
                self.switch_16k_prg_page(data as usize * 4, 0);
            }
        }
    }
        
    pub fn mirroring(&self) -> u8 {
        self.cart.mirroring
    }
    
    pub fn mirroring_base(&self) -> usize {
        self.cart.mirroring_base
    }
    
    pub fn tick_timer(&mut self) {
        //TODO
    }
}