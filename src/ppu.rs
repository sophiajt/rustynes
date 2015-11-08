use mmu::{Mmu, mirroring};

/*
const NES_PALETTE : [u16; 64] = [
	    0x8410, 0x17, 0x3017, 0x8014, 0xb80d, 0xb003, 0xb000, 0x9120,
	    0x7940, 0x1e0, 0x241, 0x1e4, 0x16c, 0x0, 0x20, 0x20,
	    0xce59, 0x2df, 0x41ff, 0xb199, 0xf995, 0xf9ab, 0xf9a3, 0xd240,
	    0xc300, 0x3bc0, 0x1c22, 0x4ac, 0x438, 0x1082, 0x841, 0x841,
	    0xffff, 0x4bf, 0x6c3f, 0xd37f, 0xfbb9, 0xfb73, 0xfbcb, 0xfc8b,
	    0xfd06, 0xa5e0, 0x56cd, 0x4eb5, 0x6df, 0x632c, 0x861, 0x861,
	    0xffff, 0x85ff, 0xbddf, 0xd5df, 0xfdfd, 0xfdf9, 0xfe36, 0xfe75,
	    0xfed4, 0xcf13, 0xaf76, 0xafbd, 0xb77f, 0xdefb, 0x1082, 0x1082
   ];
*/

const NES_PALETTE : [u32; 64] = [
    0x808080, 0x0000BB, 0x3700BF, 0x8400A6, 0xBB006A, 0xB7001E, 0xB30000, 0x912600,
    0x7B2B00, 0x003E00, 0x00480D, 0x003C22, 0x002F66, 0x000000, 0x050505, 0x050505, 
    0xC8C8C8, 0x0059FF, 0x443CFF, 0xB733CC, 0xFF33AA, 0xFF375E, 0xFF371A, 0xD54B00,
    0xC46200, 0x3C7B00, 0x1E8415, 0x009566, 0x0084C4, 0x111111, 0x090909, 0x090909, 
    0xFFFFFF, 0x0095FF, 0x6F84FF, 0xD56FFF, 0xFF77CC, 0xFF6F99, 0xFF7B59, 0xFF915F, 
    0xFFA233, 0xA6BF00, 0x51D96A, 0x4DD5AE, 0x00D9FF, 0x666666, 0x0D0D0D, 0x0D0D0D,
    0xFFFFFF, 0x84BFFF, 0xBBBBFF, 0xD0BBFF, 0xFFBFEA, 0xFFBFCC, 0xFFC4B7, 0xFFCCAE, 
    0xFFD9A2, 0xCCE199, 0xAEEEB7, 0xAAF7EE, 0xB3EEFF, 0xDDDDDD, 0x111111, 0x111111
];

pub type BitsPerPixel = u32;
 
pub struct Ppu {
    execute_nmi_on_vblank: bool,
    ppu_master: u8,
    sprite_size: usize,
    background_address: usize,
    sprite_address: usize,
    ppu_address_increment: usize,
    name_table_address: usize,
    
    monochrome_display: bool,
    no_background_clipping: bool,
    no_sprite_clipping: bool,
    background_visible: bool,
    sprites_visible: bool,
    
    ppu_color: i32,
    
    sprite_0_hit: bool,
    sprite_0_buffer: Vec<i32>,
    
    vram_rw_addr: usize,
    prev_vram_rw_addr: usize,
    vram_hi_lo_toggle: u8,
    vram_read_buffer: u8,
    scroll_v: u8,
    scroll_h: u8,
    
    //FIXME: this is public for debugging purposes
    pub current_scanline: usize,
    name_tables: Vec<u8>,
    
    sprite_ram: Vec<u8>,
    sprite_ram_address: usize,
    sprites_crossed: i32,
    
    pub offscreen_buffer: Vec<BitsPerPixel>,

    //workarounds
    pub fix_scroll_offset_1: bool,
    pub fix_scroll_offset_2: bool,
    pub fix_scroll_offset_3: bool,
    pub fix_bg_change: bool            
}

impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            execute_nmi_on_vblank: false,
            ppu_master: 0xff,
            sprite_size: 8,
            background_address: 0x0000,
            sprite_address: 0x0000,
            ppu_address_increment: 1,
            name_table_address: 0x2000,
            current_scanline: 0,
            vram_hi_lo_toggle: 1,
            vram_read_buffer: 0,
            prev_vram_rw_addr: 0,
            vram_rw_addr: 0,
            sprite_ram_address: 0,
            scroll_v: 0,
            scroll_h: 0,
            ppu_color: 0,
            sprites_crossed: 0,
            sprite_0_hit: false,
            monochrome_display: false,
            no_background_clipping: false,
            no_sprite_clipping: false,
            background_visible: false,
            sprites_visible: false,
            fix_scroll_offset_1: false,
            fix_scroll_offset_2: false,
            fix_scroll_offset_3: false,
            fix_bg_change: false,
            name_tables: vec![0; 0x2000],
            sprite_ram: vec![0; 0x100],
            offscreen_buffer: vec![0; 256*240],
            sprite_0_buffer: vec![0; 256]            
        }
    }
    
    pub fn control_reg_1_write(&mut self, data: u8) {
        self.execute_nmi_on_vblank = (data & 0x80) == 0x80;
        self.sprite_size = if (data & 0x20) == 0x20 {16} else {8};
        self.background_address = if (data & 0x10) == 0x10 {0x1000} else {0};
        self.sprite_address = if (data & 0x8) == 0x8 {0x1000} else {0};
        self.ppu_address_increment = if (data & 0x4) == 0x4 {32} else {1};
        
        if self.background_visible || (self.ppu_master == 0xff) || 
            (self.ppu_master == 1) {
            
            match data & 0x3 {
                0 => self.name_table_address = 0x2000,
                1 => self.name_table_address = 0x2400,
                2 => self.name_table_address = 0x2800,
                3 => self.name_table_address = 0x2c00,
                _ => {}
            }
        }
        
        if self.fix_bg_change && self.current_scanline == 241 {
            self.name_table_address = 0x2000;
        }
        
        if self.ppu_master == 0xff {
            if (data & 0x40) == 0x40 {
                self.ppu_master = 0;
            }
            else {
                self.ppu_master = 1;
            }
        }
    }
    
    pub fn control_reg_2_write(&mut self, data: u8) {
        self.monochrome_display = (data & 0x1) == 0x1;
        self.no_background_clipping = (data & 0x2) == 0x2;
        self.no_sprite_clipping = (data & 0x4) == 0x4;
        self.sprites_visible = (data & 0x8) == 0x8;
        self.ppu_color = (data >> 5) as i32;
    }
    
    pub fn status_reg_read(&mut self) -> u8 {
        let mut result: u8 = 0;
        
        if self.current_scanline >= 240 {
            result += 0x80;
        }
        
        if self.sprite_0_hit {
            result += 0x40;
        }
        
        if self.sprites_crossed > 8 {
            result += 0x20;
        }
        
        self.vram_hi_lo_toggle = 1;
        
        result
    }
    
    pub fn vram_addr_reg_1_write(&mut self, data: u8) {
        if self.vram_hi_lo_toggle == 1 {
            self.scroll_v = data;
            self.vram_hi_lo_toggle = 0;
        }
        else {
            self.scroll_h = data;
            if self.scroll_h > 239 {
                self.scroll_h = 0;
            }
            
            // FIXME: these workarounds are from original emu
            // Unsure if there are unimplemented
            // parts of the PPU that should be added
            if self.fix_scroll_offset_1 &&
                self.current_scanline < 240 {
                
                self.scroll_h = ((self.scroll_h as i32) - self.current_scanline as i32) as u8;
            }
            if self.fix_scroll_offset_2 && 
                self.current_scanline < 240 {
                
                self.scroll_h = ((self.scroll_h as i32) - (self.current_scanline as i32) + 8) as u8;
            }
            if self.fix_scroll_offset_3 && 
                self.current_scanline < 240 {
                
                self.scroll_h = 238;
            }
        }
        
        self.vram_hi_lo_toggle = 1;
    }
    
    pub fn vram_addr_reg_2_write(&mut self, data: u8) {
        if self.vram_hi_lo_toggle == 1 {
            self.prev_vram_rw_addr = 
                self.vram_rw_addr;
            self.vram_rw_addr = (data as usize) << 8;
            self.vram_hi_lo_toggle = 0;
        }
        else {
            self.vram_rw_addr += data as usize;
            
            if (self.prev_vram_rw_addr == 0) && (self.current_scanline < 240) {
                //check for scrolling trick
                if (self.vram_rw_addr >= 0x2000) && (self.vram_rw_addr <= 0x2400) {
                    self.scroll_h = (((self.vram_rw_addr - 0x2000) / 0x20) * 8 - self.current_scanline) as u8;
                }
            }
            self.vram_hi_lo_toggle = 1;
        }
    }
    
    pub fn vram_io_reg_write(&mut self, mmu: &mut Mmu, data: u8) {
        if self.vram_rw_addr < 0x2000 {
            mmu.write_chr_rom(self.vram_rw_addr, data);
        }
        else if (self.vram_rw_addr >= 0x2000) && (self.vram_rw_addr < 0x3f00) {
            match mmu.mirroring() {
                mirroring::HORIZONTAL => {
                    match self.vram_rw_addr & 0x2c00 {
                        0x2000 => self.name_tables[self.vram_rw_addr - 0x2000] = data,
                        0x2400 => self.name_tables[self.vram_rw_addr - 0x2400] = data,
                        0x2800 => self.name_tables[self.vram_rw_addr - 0x2400] = data,
                        0x2C00 => self.name_tables[self.vram_rw_addr - 0x2800] = data,
                        _ => {}
                    }
                },
                mirroring::VERTICAL => {
                    match self.vram_rw_addr & 0x2c00 {
                        0x2000 => self.name_tables[self.vram_rw_addr - 0x2000] = data,
                        0x2400 => self.name_tables[self.vram_rw_addr - 0x2000] = data,
                        0x2800 => self.name_tables[self.vram_rw_addr - 0x2800] = data,
                        0x2C00 => self.name_tables[self.vram_rw_addr - 0x2800] = data,
                        _ => {}
                    }
                },
                mirroring::ONE_SCREEN => {
                    if mmu.mirroring_base() == 0x2000 {
                        match self.vram_rw_addr & 0x2c00 {
                            0x2000 => self.name_tables[self.vram_rw_addr - 0x2000] = data,
                            0x2400 => self.name_tables[self.vram_rw_addr - 0x2400] = data,
                            0x2800 => self.name_tables[self.vram_rw_addr - 0x2800] = data,
                            0x2C00 => self.name_tables[self.vram_rw_addr - 0x2c00] = data,
                            _ => {}
                        }
                    }
                    else if mmu.mirroring_base() == 0x2400 {
                        match self.vram_rw_addr & 0x2c00 {
                            0x2000 => self.name_tables[self.vram_rw_addr + 0x400 - 0x2000] = data,
                            0x2400 => self.name_tables[self.vram_rw_addr - 0x2000] = data,
                            0x2800 => self.name_tables[self.vram_rw_addr - 0x2400] = data,
                            0x2C00 => self.name_tables[self.vram_rw_addr - 0x2800] = data,
                            _ => {}
                        }
                    }
                },
                _ =>self.name_tables[self.vram_rw_addr - 0x2000] = data
            }
        }
        else if (self.vram_rw_addr >= 0x3f00) && (self.vram_rw_addr < 0x3f20) {
            self.name_tables[self.vram_rw_addr - 0x2000] = data;
            if (self.vram_rw_addr & 0x7) == 0 {
                self.name_tables[(self.vram_rw_addr - 0x2000) ^ 0x10] = data;
            }
        }
        self.vram_rw_addr += self.ppu_address_increment;
    }
    
    pub fn vram_io_reg_read(&mut self, mmu: &Mmu) -> u8 {
        let mut result = 0;
        
        if self.vram_rw_addr < 0x3f00 {
            result = self.vram_read_buffer;
            
            if self.vram_rw_addr >= 0x2000 {
                self.vram_read_buffer = self.name_tables[self.vram_rw_addr - 0x2000];                
            }
            else {
                self.vram_read_buffer = mmu.read_chr_rom(self.vram_rw_addr);
            }
        }
        else if self.vram_rw_addr >= 0x4000 {
            println!("Error: Need VRAM mirroring!");
        }
        else {
            result = self.name_tables[self.vram_rw_addr - 0x2000];
        }
        
        //FIXME: This is not entirely accurate, the 'buffered' read
        //should not increment the address the first time
        self.vram_rw_addr += self.ppu_address_increment;
        
        result
    }
    
    pub fn sprite_ram_addr_reg_write(&mut self, data: u8) {
        self.sprite_ram_address = data as usize;
    }
    
    pub fn sprite_ram_io_reg_write(&mut self, data: u8) {
        self.sprite_ram[self.sprite_ram_address] = data;
        self.sprite_ram_address += 1;
    }
    
    pub fn sprite_ram_io_reg_read(&self) -> u8 {
        self.sprite_ram[self.sprite_ram_address]
    }
    
    pub fn sprite_ram_dma_begin(&mut self, mmu: &Mmu, data: u8) {
        println!("Sprite RAM DMA from 0x{0:x}", (data as u16) * 0x100);
        for i in 0..256 {
            self.sprite_ram[i] = mmu.read_u8(self, (data as u16) * 0x100 + i as u16);
        }
        println!("{:?}", self.sprite_ram);
    }
    
    fn render_background(&mut self, mmu: &mut Mmu) {
        let mut start_column;
        let mut end_column;
        
        for v_scroll_side in 0..2 {
            let mut virtual_scanline = self.current_scanline + self.scroll_h as usize;
            let mut name_table_base = self.name_table_address;
            
            if v_scroll_side == 0 {
                if virtual_scanline >= 240 {
                    match self.name_table_address {
                        0x2000 => name_table_base = 0x2800,
                        0x2400 => name_table_base = 0x2c00,
                        0x2800 => name_table_base = 0x2000,
                        0x2c00 => name_table_base = 0x2400,
                        _ => {}
                    }
                    
                    virtual_scanline -= 240;
                }
                
                start_column = self.scroll_v / 8;
                end_column = 32;
            }
            else {
                if virtual_scanline >= 240 {
                    match self.name_table_address {
                        0x2000 => name_table_base = 0x2c00,
                        0x2400 => name_table_base = 0x2800,
                        0x2800 => name_table_base = 0x2400,
                        0x2c00 => name_table_base = 0x2000,
                        _ => {}
                    }
                    
                    virtual_scanline -= 240;
                }
                else {
                    match self.name_table_address {
                        0x2000 => name_table_base = 0x2400,
                        0x2400 => name_table_base = 0x2000,
                        0x2800 => name_table_base = 0x2c00,
                        0x2c00 => name_table_base = 0x2800,
                        _ => {}
                    }         
                }
                
                start_column = 0;
                end_column = self.scroll_v / 8 + 1;
            }
            
            match mmu.mirroring() {
                mirroring::HORIZONTAL => {
                    match name_table_base {
                        0x2400 => name_table_base = 0x2000,
                        0x2800 => name_table_base = 0x2400,
                        0x2c00 => name_table_base = 0x2400,
                        _ => {}
                    }
                },
                mirroring::VERTICAL => {
                    match name_table_base {
                        0x2800 => name_table_base = 0x2000,
                        0x2c00 => name_table_base = 0x2400,
                        _ => {}
                    }
                },
                mirroring::ONE_SCREEN => {
                    name_table_base = mmu.mirroring_base();
                },
                _ => {}
            }
            
            for current_col in start_column..end_column {
                // grab the bg tile for the given column and scanline

                let tile_num = self.name_tables[name_table_base - 0x2000 + 
                    ((virtual_scanline / 8) * 32) + current_col as usize];
                
                let tile_data_offset = self.background_address + (tile_num as usize) * 16;
                
                let tile_data_1 = mmu.read_chr_rom(tile_data_offset + 
                    (virtual_scanline % 8));
                let tile_data_2 = mmu.read_chr_rom(tile_data_offset + 
                    (virtual_scanline % 8) + 8);
                    
                // next, calculate where to go in the palette table
                
                let mut palette_high_bits = self.name_tables[((name_table_base - 0x2000 + 
                    0x3c0 + (((virtual_scanline / 8) / 4) * 8) + ((current_col / 4) as usize)))];
                palette_high_bits = palette_high_bits >> ((4 * (((virtual_scanline / 8 ) % 4) / 2)) + 
                    (2 * ((current_col % 4) / 2)) as usize);
                palette_high_bits = (palette_high_bits & 0x3) << 2;
                
                // that was fun, now we have enough to render the tile
                
                let mut start_tile_pixel = 0;
                let mut end_tile_pixel = 8;
                
                if (v_scroll_side == 0) && (current_col == start_column) {
                    start_tile_pixel = self.scroll_v % 8;
                }
                else if (v_scroll_side == 1) && (current_col == end_column) {
                    end_tile_pixel = self.scroll_v % 8;
                }
                
                for i in start_tile_pixel..end_tile_pixel {
                    let pixel_color = palette_high_bits + (((tile_data_2 & (1 << (7 - i))) >> (7 - i)) << 1) + 
                        ((tile_data_1 & (1 << (7 - i))) >> (7 - i)); 
                    
                    if (pixel_color % 4) != 0 {
                        if v_scroll_side == 0 {
                            self.offscreen_buffer[(self.current_scanline * 256) + ((8 * current_col) as usize) - (self.scroll_v as usize) + i as usize] = 
                                NES_PALETTE[(0x3f & self.name_tables[0x1f00 + pixel_color as usize]) as usize];
                            
                            if !self.sprite_0_hit {
                                self.sprite_0_buffer[((8 * current_col) - self.scroll_v + (i as u8)) as usize] += 4;
                            }
                        }
                        else {
                            if ((8 * current_col as usize) + (256 - self.scroll_v as usize) + i as usize) < 256 {
                                self.offscreen_buffer[((self.current_scanline * 256) + ((8 * current_col) as usize) + ((256usize - self.scroll_v as usize) as usize) + i as usize) as usize] = 
                                    NES_PALETTE[(0x3f & (self.name_tables[0x1f00 + pixel_color as usize] as usize))];
                            
                                //Console.WriteLine("Greater than: {0}", ((8 * currentTileColumn) + (256-scrollV) + i));
                                if !self.sprite_0_hit {
                                    self.sprite_0_buffer[((8 * current_col) + ((256 - self.scroll_v as usize) as u8) + i) as usize] += 4;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    fn render_sprites(&mut self, mmu: &mut Mmu, behind: u8) {
        let mut i : usize = 252;
        
        loop {
            let actual_y : usize = (self.sprite_ram[i] as usize) + 1;
            
            if ((self.sprite_ram[i+2] & 0x20) == behind) && 
                (actual_y <= self.current_scanline) &&
                ((actual_y + self.sprite_size) > self.current_scanline) {
            
                self.sprites_crossed += 1;
                
                if self.sprite_size == 8 {
                    //sprite is 8x8
                    
                    let sprite_line_to_draw : usize = 
                        if (self.sprite_ram[i+2] & 0x80) != 0x80 {
                            self.current_scanline - actual_y
                        }
                        else {
                            actual_y + 7 - self.current_scanline
                        };
                    let offset_to_sprite : usize = self.sprite_address + 
                        (((self.sprite_ram[i+1] as usize) * 16) as usize);
                    
                    let tile_data_1 = mmu.read_chr_rom(offset_to_sprite + sprite_line_to_draw);
                    let tile_data_2 = mmu.read_chr_rom(offset_to_sprite + sprite_line_to_draw + 8);
                    
                    let palette_high_bits = (self.sprite_ram[i+2] & 0x3) << 2;
                    
                    for j in 0..8 {
                        // Calculate pixel color, we'll also check the horizontal flip bit
                        let pixel_color = 
                            if (self.sprite_ram[i+2] & 0x40) == 0x40 {
                                palette_high_bits + (((tile_data_2 & (1 << (j))) >> (j)) << 1) + ((tile_data_1 & (1 << (j))) >> (j)) 
                            }
                            else {
                                palette_high_bits + (((tile_data_2 & (1 << (7 - j))) >> (7 - j)) << 1) + 
                                    ((tile_data_1 & (1 << (7 - j))) >> (7 - j))
                            };
                        if (pixel_color % 4) != 0 {
                            if ((self.sprite_ram[i+3] as usize) + j) < 256 {
                                self.offscreen_buffer[(self.current_scanline * 256) + (self.sprite_ram[i+3] as usize) + j] = 
                                    NES_PALETTE[(0x3f & self.name_tables[0x1f10 + (pixel_color as usize)]) as usize];
                            
                                if i == 0 {
                                    self.sprite_0_buffer[(self.sprite_ram[i+3] as usize) + j] += 1;
                                }
                            }
                        }
                    }
                }
                else {
                    // If they aren't 8x8, they're 8x16
                    
                    let sprite_id = self.sprite_ram[i+1] as usize;
                    
                    let mut sprite_line_to_draw : usize = 
                        if (self.sprite_ram[i+2] & 0x80) != 0x80 {
                            self.current_scanline - actual_y
                        }
                        else {
                            actual_y + 16 - self.current_scanline
                        };
                    
                    let mut offset_to_sprite : usize = 0;
                    
                    if sprite_line_to_draw < 8 {
                        //top sprite
                        
                        if (sprite_id % 2) == 0 {
                            offset_to_sprite = sprite_id * 16
                        }
                        else if sprite_line_to_draw < 8 {
                            offset_to_sprite = 0x1000 + (sprite_id - 1) * 16
                        }
                    }
                    else {
                        //bottom sprite
                        sprite_line_to_draw -= 8;
                        
                        if (sprite_id % 2) == 0 {
                            offset_to_sprite = (sprite_id + 1) * 16;
                        }
                        else {
                            offset_to_sprite = 0x1000 + sprite_id * 16;
                        }
                    }
                    
                    let tile_data_1 = mmu.read_chr_rom(offset_to_sprite + sprite_line_to_draw);
                    let tile_data_2 = mmu.read_chr_rom(offset_to_sprite + sprite_line_to_draw + 8);
                                        
                    let palette_high_bits = (self.sprite_ram[i+2] & 0x3) << 2;
                    
                    for j in 0..8 {
                        let pixel_color = 
                            if (self.sprite_ram[i+2] & 0x40) == 0x40 {
                                palette_high_bits + (((tile_data_2 & (1 << (j))) >> (j)) << 1) + ((tile_data_1 & (1 << (j))) >> (j)) 
                            }
                            else {
                                palette_high_bits + (((tile_data_2 & (1 << (7 - j))) >> (7 - j)) << 1) + 
                                    ((tile_data_1 & (1 << (7 - j))) >> (7 - j))
                            };
                            
                        if (pixel_color % 4) != 0 {
                            if ((self.sprite_ram[i+3] as usize) + j) < 256 {
                                self.offscreen_buffer[(self.current_scanline * 256) + (self.sprite_ram[i+3] as usize) + j] = 
                                    NES_PALETTE[(0x3f & self.name_tables[0x1f10 + (pixel_color as usize)]) as usize];
                            
                                if i == 0 {
                                    self.sprite_0_buffer[(self.sprite_ram[i+3] as usize) + j] += 1;
                                }
                            }
                        }
                    }                                        
                }
            }
            
            if i == 0 { break; }
            i -= 4;
        }
    }
    
    pub fn render_scanline(&mut self, mmu: &mut Mmu) -> bool {
        if self.current_scanline < 234 {
            if self.name_tables[0x1f00] > 63 {
                for i in 0..256 {
                    self.offscreen_buffer[self.current_scanline * 256 + i] = 0;
                    self.sprite_0_buffer[i] = 0;
                }
            }
            else {
                for i in 0..256 {
                    self.offscreen_buffer[self.current_scanline * 256 + i] = 
                        NES_PALETTE[self.name_tables[0x1f00] as usize];
                    self.sprite_0_buffer[i] = 0;                    
                } 
            }
            self.sprites_crossed = 0;
            
            if self.sprites_visible {
                self.render_sprites(mmu, 0x20);
            }
            
            if self.background_visible {
                self.render_background(mmu);
            }
            
            if self.sprites_visible {
                self.render_sprites(mmu, 0);
            }
            
            if !self.sprite_0_hit {            
                for i in 0..256 {
                    if self.sprite_0_buffer[i] > 4 {
                        self.sprite_0_hit = true;
                    }
                }
            }
            
            /*
            if self.current_scanline == 32 {
                self.sprite_0_hit = true;
            }
            else {
                self.sprite_0_hit = false;
            }
            */
            
            if !self.no_background_clipping {
                for i in 0..8 {
                    self.offscreen_buffer[self.current_scanline * 256 + i] = 0;
                }
            }
        }

        if self.current_scanline == 240 {            
            // do nothing, but later, add video and events?
        }
                
        self.current_scanline += 1;

/*
        if self.current_scanline == 240 {
            println!("--------------------------------------");
            println!("             VBLANK");
            println!("--------------------------------------");
        }
*/
        
        if self.fix_scroll_offset_1 {
            if self.current_scanline > 244 {
                self.sprite_0_hit = false;
            }
        }
        
        if self.current_scanline > 262 {
            self.current_scanline = 0;
            self.sprite_0_hit = false;
        }
        
        if (self.current_scanline == 240) && self.execute_nmi_on_vblank {
            return true;
        }
        else {
            return false;
        }
    }
} 