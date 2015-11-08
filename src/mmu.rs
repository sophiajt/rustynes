use cart::Cart;
use ppu::Ppu;

pub mod mirroring {
    pub const HORIZONTAL  : u8 = 1;
    pub const VERTICAL    : u8 = 2;
    pub const FOUR_SCREEN : u8 = 3;
    pub const ONE_SCREEN  : u8 = 4;
}

pub struct Mmu {
    active_prg_page: Vec<usize>,
    active_chr_page: Vec<usize>,
    scratch_ram: Vec<Vec<u8>>,
    save_ram: Vec<u8>,
    is_save_ram_readonly: bool,
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
        
        let mut scratch_ram : Vec<Vec<u8>> = Vec::new();
        for _ in 0..4 {
            scratch_ram.push(vec![0; 0x800]);
        }
        
        let save_ram : Vec<u8> = vec![0; 0x2000];
        
        // Start with the default mirroring loaded from the cart, 
        // but may change via the mapper

        Mmu { 
            active_prg_page: active_prg_page,
            active_chr_page: active_chr_page,
            scratch_ram: scratch_ram,
            save_ram: save_ram,
            is_save_ram_readonly: false,
            cart: cart
        }
    }
    
    pub fn read_u8(&self, ppu: &mut Ppu, address: u16) -> u8 {
        match address {
            0x0000...0x07FF => self.scratch_ram[0][address as usize],
            0x0800...0x0FFF => self.scratch_ram[1][(address as usize) - 0x0800],
            0x1000...0x17FF => self.scratch_ram[2][(address as usize) - 0x1000],
            0x1800...0x1FFF => self.scratch_ram[3][(address as usize) - 0x1800],
            0x2002          => ppu.status_reg_read(),
            0x2004          => ppu.sprite_ram_io_reg_read(),
            0x2007          => ppu.vram_io_reg_read(self),
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
    
    pub fn read_u16(&self, ppu: &mut Ppu, address: u16) -> u16 {
        let read_1 = self.read_u8(ppu, address);
        let read_2 = self.read_u8(ppu, address+1);
        
        ((read_2 as u16) << 8) + (read_1 as u16)
    }
    
    pub fn write_u8(&mut self, ppu: &mut Ppu, address: u16, data: u8) {
        match address {
            0x0000...0x07FF => self.scratch_ram[0][address as usize]            = data,
            0x0800...0x0FFF => self.scratch_ram[1][(address as usize) - 0x0800] = data,
            0x1000...0x17FF => self.scratch_ram[2][(address as usize) - 0x1000] = data,
            0x1800...0x1FFF => self.scratch_ram[3][(address as usize) - 0x1800] = data,
            0x2000          => ppu.control_reg_1_write(data),
            0x2001          => ppu.control_reg_2_write(data),
            0x2003          => ppu.sprite_ram_addr_reg_write(data),
            0x2004          => ppu.sprite_ram_io_reg_write(data),
            0x2005          => ppu.vram_addr_reg_1_write(data),
            0x2006          => ppu.vram_addr_reg_2_write(data),
            0x2007          => ppu.vram_io_reg_write(self, data),
            0x4014          => ppu.sprite_ram_dma_begin(self, data),
            0x6000...0x7FFF => 
                if self.is_save_ram_readonly { 
                    self.save_ram[(address as usize) - 0x6000] = data;
                },
            _ => println!("Unknown write of {0:x} to {1:x}", data, address)
        }
    }
    
    pub fn write_chr_rom(&mut self, addr: usize, data:u8) {
        println!("Write to chr rom of {0:x} at {1:x}", data, addr);
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