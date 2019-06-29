use joypad::Joypad;
use ppu::{mirroring, Ppu};

pub struct Mmu {
    active_prg_page: Vec<usize>,
    scratch_ram: Vec<u8>,
    pub save_ram: Vec<u8>,
    is_save_ram_readonly: bool,

    // Mapper-specific registers

    // Mapper 1
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

    // Mapper 4
    pub map4_command_number: u8,
    pub map4_prg_addr_select: u8,
    pub map4_chr_addr_select: u8,
    pub timer_irq_enabled: bool,
    pub timer_reload_next: bool,
    pub timer_irq_count: u8,
    pub timer_irq_reload: u8,
    pub timer_zero_pulse: bool, //the single pulse timer

    // From cart
    pub prg_rom: Vec<Vec<u8>>,
    pub save_ram_present: bool,
    pub num_prg_pages: usize,

    // Save ram-specific
    pub save_ram_file_name: String,

    // Subsystems
    pub joypad: Joypad,
    pub ppu: Ppu,
}

impl Mmu {
    pub fn new() -> Mmu {
        let mut active_prg_page: Vec<usize> = Vec::new();
        for x in 0..8 {
            active_prg_page.push(x);
        }

        let scratch_ram: Vec<u8> = vec![0; 0x800];
        let save_ram: Vec<u8> = vec![0; 0x2000];

        Mmu {
            active_prg_page: active_prg_page,
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

            map4_command_number: 0,
            map4_prg_addr_select: 0,
            map4_chr_addr_select: 0,
            timer_irq_enabled: false,
            timer_reload_next: false,
            timer_irq_count: 0,
            timer_irq_reload: 0,
            timer_zero_pulse: false,

            prg_rom: Vec::new(),
            save_ram_present: false,
            num_prg_pages: 0,
            save_ram_file_name: String::new(),

            joypad: Joypad::new(),
            ppu: Ppu::new(),
        }
    }

    pub fn setup_defaults(&mut self) {
        if self.ppu.mapper == 1 {
            self.map1_reg_8000_bit = 0;
            self.map1_reg_8000_val = 0;
            self.map1_mirroring_flag = 0;
            self.map1_one_page_mirroring = 1;
            self.map1_prg_switch_area = 1;
            self.map1_prg_switch_size = 1;
            self.map1_vrom_switch_size = 0;

            let num_prg = self.num_prg_pages;
            self.switch_16k_prg_page((num_prg - 1) * 4, 1);
        } else if self.ppu.mapper == 2 {
            let num_prg = self.num_prg_pages;
            self.switch_16k_prg_page((num_prg - 1) * 4, 1);
        } else if self.ppu.mapper == 4 {
            self.map4_prg_addr_select = 0;
            self.map4_chr_addr_select = 0;
            self.timer_zero_pulse = false;
            let num_prg = self.num_prg_pages;
            self.switch_16k_prg_page((num_prg - 1) * 4, 1);
        }
    }

    pub fn read_u8(&mut self, address: u16) -> u8 {
        match address {
            0x0000..=0x07FF => self.scratch_ram[address as usize],
            0x0800..=0x0FFF => self.scratch_ram[(address as usize) - 0x0800],
            0x1000..=0x17FF => self.scratch_ram[(address as usize) - 0x1000],
            0x1800..=0x1FFF => self.scratch_ram[(address as usize) - 0x1800],
            0x2002 => self.ppu.status_reg_read(),
            0x2004 => self.ppu.sprite_ram_io_reg_read(),
            0x2007 => self.ppu.vram_io_reg_read(),
            0x4015 => 0, //ignored read
            0x4016 => self.joypad.joypad_1_read(),
            0x4017 => self.joypad.joypad_2_read(),
            0x6000..=0x7FFF => self.save_ram[(address as usize) - 0x6000],
            0x8000..=0x8FFF => self.prg_rom[self.active_prg_page[0]][(address as usize) - 0x8000],
            0x9000..=0x9FFF => self.prg_rom[self.active_prg_page[1]][(address as usize) - 0x9000],
            0xA000..=0xAFFF => self.prg_rom[self.active_prg_page[2]][(address as usize) - 0xA000],
            0xB000..=0xBFFF => self.prg_rom[self.active_prg_page[3]][(address as usize) - 0xB000],
            0xC000..=0xCFFF => self.prg_rom[self.active_prg_page[4]][(address as usize) - 0xC000],
            0xD000..=0xDFFF => self.prg_rom[self.active_prg_page[5]][(address as usize) - 0xD000],
            0xE000..=0xEFFF => self.prg_rom[self.active_prg_page[6]][(address as usize) - 0xE000],
            0xF000..=0xFFFF => self.prg_rom[self.active_prg_page[7]][(address as usize) - 0xF000],
            _ => {
                println!("Unknown read: {0:x}", address);
                0
            }
        }
    }

    pub fn read_u16(&mut self, address: u16) -> u16 {
        let read_1 = self.read_u8(address);
        let read_2 = self.read_u8(address + 1);

        ((read_2 as u16) << 8) + (read_1 as u16)
    }

    pub fn write_u8(&mut self, address: u16, data: u8) {
        match address {
            0x0000..=0x07FF => self.scratch_ram[address as usize] = data,
            0x0800..=0x0FFF => self.scratch_ram[(address as usize) - 0x0800] = data,
            0x1000..=0x17FF => self.scratch_ram[(address as usize) - 0x1000] = data,
            0x1800..=0x1FFF => self.scratch_ram[(address as usize) - 0x1800] = data,
            0x2000 => self.ppu.control_reg_1_write(data),
            0x2001 => self.ppu.control_reg_2_write(data),
            0x2003 => self.ppu.sprite_ram_addr_reg_write(data),
            0x2004 => self.ppu.sprite_ram_io_reg_write(data),
            0x2005 => self.ppu.vram_addr_reg_1_write(data),
            0x2006 => self.ppu.vram_addr_reg_2_write(data),
            0x2007 => self.ppu.vram_io_reg_write(data),
            0x4000..=0x4013 => {} // Sound signal write
            0x4014 => self.sprite_ram_dma_begin(data),
            0x4015 => {} // Sound signal write
            0x4016 => self.joypad.joypad_1_write(data),
            0x4017 => self.joypad.joypad_2_write(data),
            0x6000..=0x7FFF => {
                if !self.is_save_ram_readonly {
                    self.save_ram[(address as usize) - 0x6000] = data;
                }
            }
            0x8000..=0xFFFF => self.write_prg_rom(address, data),
            _ => println!("Unknown write of {0:x} to {1:x}", data, address),
        }
    }

    pub fn sprite_ram_dma_begin(&mut self, data: u8) {
        //println!("Sprite RAM DMA from 0x{0:x}", (data as u16) * 0x100);
        for i in 0..256 {
            self.ppu.sprite_ram[i] = self.read_u8((data as u16) * 0x100 + i as u16);
        }
        //println!("{:?}", self.sprite_ram);
    }

    fn switch_32k_prg_page(&mut self, start: usize) {
        let start_page = match self.num_prg_pages {
            2 => start & 0x7,
            4 => start & 0xf,
            8 => start & 0x1f,
            16 => start & 0x3f,
            32 => start & 0x7f,
            _ => {
                println!("Error: bad 32k switch");
                0
            }
        };

        for i in 0..8 {
            self.active_prg_page[i] = start_page + i;
        }
    }

    fn switch_16k_prg_page(&mut self, start: usize, area: usize) {
        let start_page = match self.num_prg_pages {
            2 => start & 0x7,
            4 => start & 0xf,
            8 => start & 0x1f,
            16 => start & 0x3f,
            32 => start & 0x7f,
            _ => {
                println!("Error: bad 16k switch");
                0
            }
        };

        for i in 0..4 {
            self.active_prg_page[4 * area + i] = start_page + i;
        }
    }

    fn switch_8k_prg_page(&mut self, start: usize, area: usize) {
        let start_page = match self.num_prg_pages {
            2 => start & 0x7,
            4 => start & 0xf,
            8 => start & 0x1f,
            16 => start & 0x3f,
            32 => start & 0x7f,
            _ => 0,
        };

        for i in 0..2 {
            self.active_prg_page[i + 2 * area] = start_page + i;
        }
    }

    fn switch_8k_chr_page(&mut self, start: usize) {
        let start_page = match self.ppu.num_chr_pages {
            2 => start & 0xf,
            4 => start & 0x1f,
            8 => start & 0x3f,
            16 => start & 0x7f,
            32 => start & 0xff,
            _ => {
                println!("Error: bad 8k chr switch");
                0
            }
        };

        for i in 0..8 {
            self.ppu.active_chr_page[i] = start_page + i;
        }
    }

    fn switch_4k_chr_page(&mut self, start: usize, area: usize) {
        let start_page = match self.ppu.num_chr_pages {
            2 => start & 0xf,
            4 => start & 0x1f,
            8 => start & 0x3f,
            16 => start & 0x7f,
            32 => start & 0xff,
            _ => {
                println!("Error: bad 4k chr switch");
                0
            }
        };

        for i in 0..4 {
            self.ppu.active_chr_page[i + 4 * area] = start_page + i;
        }
    }

    fn switch_2k_chr_page(&mut self, start: usize, area: usize) {
        let start_page = match self.ppu.num_chr_pages {
            2 => start & 0xf,
            4 => start & 0x1f,
            8 => start & 0x3f,
            16 => start & 0x7f,
            32 => start & 0xff,
            _ => 0,
        };

        for i in 0..2 {
            self.ppu.active_chr_page[i + 2 * area] = start_page + i;
        }
    }

    fn switch_1k_chr_page(&mut self, start: usize, area: usize) {
        let start_page = match self.ppu.num_chr_pages {
            2 => start & 0xf,
            4 => start & 0x1f,
            8 => start & 0x3f,
            16 => start & 0x7f,
            32 => start & 0xff,
            _ => 0,
        };

        self.ppu.active_chr_page[area] = start_page;
    }

    fn write_prg_rom(&mut self, addr: u16, data: u8) {
        if self.ppu.mapper == 1 {
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
                } else {
                    self.map1_reg_8000_val += ((data & 0x1) << self.map1_reg_8000_bit) as usize;
                    self.map1_reg_8000_bit += 1;
                    if self.map1_reg_8000_bit == 5 {
                        self.map1_mirroring_flag = (self.map1_reg_8000_val & 1) as u8;
                        if self.map1_mirroring_flag == 0 {
                            self.ppu.mirroring = mirroring::VERTICAL;
                        } else {
                            self.ppu.mirroring = mirroring::HORIZONTAL;
                        }
                        self.map1_one_page_mirroring = ((self.map1_reg_8000_val >> 1) & 1) as u8;

                        if self.map1_one_page_mirroring == 0 {
                            self.ppu.mirroring = mirroring::ONE_SCREEN;
                            self.ppu.mirroring_base = 0x2000;
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
            } else if (addr >= 0xa000) && (addr <= 0xbfff) {
                if (data & 0x80) == 0x80 {
                    self.map1_reg_a000_bit = 0;
                    self.map1_reg_a000_val = 0;
                } else {
                    self.map1_reg_a000_val += ((data & 0x1) << self.map1_reg_a000_bit) as usize;
                    self.map1_reg_a000_bit += 1;

                    if self.map1_reg_a000_bit == 5 {
                        let val = self.map1_reg_a000_val;
                        if self.ppu.num_chr_pages > 0 {
                            if self.map1_vrom_switch_size == 1 {
                                self.switch_4k_chr_page(val * 4, 0);
                            } else {
                                self.switch_8k_chr_page((val >> 1) * 8);
                            }
                        }
                        self.map1_reg_a000_bit = 0;
                        self.map1_reg_a000_val = 0;
                    }
                }
            } else if (addr >= 0xc000) && (addr <= 0xdfff) {
                if (data & 0x80) == 0x80 {
                    self.map1_reg_c000_bit = 0;
                    self.map1_reg_c000_val = 0;
                } else {
                    self.map1_reg_c000_val += ((data & 0x1) << self.map1_reg_c000_bit) as usize;
                    self.map1_reg_c000_bit += 1;

                    if self.map1_reg_c000_bit == 5 {
                        let val = self.map1_reg_c000_val;
                        if self.ppu.num_chr_pages > 0 {
                            if self.map1_vrom_switch_size == 1 {
                                self.switch_4k_chr_page(val * 4, 1);
                            }
                        }
                        self.map1_reg_c000_bit = 0;
                        self.map1_reg_c000_val = 0;
                    }
                }
            } else if addr >= 0xe000 {
                if (data & 0x80) == 0x80 {
                    self.map1_reg_8000_bit = 0;
                    self.map1_reg_8000_val = 0;
                    self.map1_reg_a000_bit = 0;
                    self.map1_reg_a000_val = 0;
                    self.map1_reg_c000_bit = 0;
                    self.map1_reg_c000_val = 0;
                    self.map1_reg_e000_bit = 0;
                    self.map1_reg_e000_val = 0;
                } else {
                    self.map1_reg_e000_val += ((data & 0x1) << self.map1_reg_e000_bit) as usize;
                    self.map1_reg_e000_bit += 1;

                    if self.map1_reg_e000_bit == 5 {
                        let val = self.map1_reg_e000_val;
                        let num_prg = self.num_prg_pages;
                        if self.map1_prg_switch_size == 1 {
                            if self.map1_prg_switch_area == 1 {
                                self.switch_16k_prg_page(val * 4, 0);
                                self.switch_16k_prg_page((num_prg - 1) * 4, 1);
                            } else {
                                self.switch_16k_prg_page(val * 4, 1);
                                self.switch_16k_prg_page(0, 0);
                            }
                        } else {
                            self.switch_32k_prg_page((val >> 1) * 8);
                        }
                        self.map1_reg_e000_bit = 0;
                        self.map1_reg_e000_val = 0;
                    }
                }
            }
        } else if self.ppu.mapper == 2 {
            if addr >= 0x8000 {
                self.switch_16k_prg_page(data as usize * 4, 0);
            }
        } else if self.ppu.mapper == 3 {
            if addr >= 0x8000 {
                self.switch_8k_chr_page(data as usize * 8);
            }
        } else if self.ppu.mapper == 4 {
            if addr == 0x8000 {
                self.map4_command_number = data & 0x7;
                self.map4_prg_addr_select = data & 0x40;
                self.map4_chr_addr_select = data & 0x80;
            } else if addr == 0x8001 {
                if self.map4_command_number == 0 {
                    let new_data = data - (data % 2);

                    if self.map4_chr_addr_select == 0 {
                        self.switch_2k_chr_page(new_data as usize, 0);
                    } else {
                        self.switch_2k_chr_page(new_data as usize, 2);
                    }
                } else if self.map4_command_number == 1 {
                    let new_data = data - (data % 2);

                    if self.map4_chr_addr_select == 0 {
                        self.switch_2k_chr_page(new_data as usize, 1);
                    } else {
                        self.switch_2k_chr_page(new_data as usize, 3);
                    }
                } else if self.map4_command_number == 2 {
                    let new_data = data & ((self.ppu.num_chr_pages * 8 - 1) as u8);
                    if self.map4_chr_addr_select == 0 {
                        self.switch_1k_chr_page(new_data as usize, 4);
                    } else {
                        self.switch_1k_chr_page(new_data as usize, 0);
                    }
                } else if self.map4_command_number == 3 {
                    if self.map4_chr_addr_select == 0 {
                        self.switch_1k_chr_page(data as usize, 5);
                    } else {
                        self.switch_1k_chr_page(data as usize, 1);
                    }
                } else if self.map4_command_number == 4 {
                    if self.map4_chr_addr_select == 0 {
                        self.switch_1k_chr_page(data as usize, 6);
                    } else {
                        self.switch_1k_chr_page(data as usize, 2);
                    }
                } else if self.map4_command_number == 5 {
                    if self.map4_chr_addr_select == 0 {
                        self.switch_1k_chr_page(data as usize, 7);
                    } else {
                        self.switch_1k_chr_page(data as usize, 3);
                    }
                } else if self.map4_command_number == 6 {
                    let num_pages = self.num_prg_pages;
                    if self.map4_prg_addr_select == 0 {
                        self.switch_8k_prg_page(data as usize * 2, 0);
                        self.switch_8k_prg_page(num_pages * 4 - 4, 2);
                    } else {
                        self.switch_8k_prg_page(data as usize * 2, 2);
                        self.switch_8k_prg_page(num_pages * 4 - 4, 0);
                    }
                } else if self.map4_command_number == 7 {
                    let num_pages = self.num_prg_pages;
                    self.switch_8k_prg_page(data as usize * 2, 1);
                    if self.map4_prg_addr_select == 0 {
                        self.switch_8k_prg_page(num_pages * 4 - 4, 2);
                    } else {
                        self.switch_8k_prg_page(num_pages * 4 - 4, 0);
                    }
                }
            } else if addr == 0xa000 {
                if (data & 1) == 1 {
                    self.ppu.mirroring = mirroring::HORIZONTAL;
                } else {
                    self.ppu.mirroring = mirroring::VERTICAL;
                }
            } else if addr == 0xa001 {
                //currently we ignore this
            } else if addr == 0xc000 {
                self.timer_irq_reload = data;
                if data == 0 {
                    self.timer_zero_pulse = true;
                }
                self.timer_reload_next = true;
            } else if addr == 0xc001 {
                self.timer_irq_count = 0;
            } else if addr == 0xe000 {
                self.timer_irq_enabled = false;
            } else if addr == 0xe001 {
                self.timer_irq_enabled = true;
            } else {
                println!("Unknown prg write: {0:04x}", addr);
            }
        }
    }
}
