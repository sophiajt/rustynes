use std::io;
use std::io::prelude::*;
use std::fs::File;
use util::BitReader;
use mmu::mirroring;

#[derive(Debug)]
pub struct Cart {
    pub prg_rom : Vec<Vec<u8>>,
    pub chr_rom : Vec<Vec<u8>>,
    pub mirroring: u8,
    pub mirroring_base: usize, 
    trainer_present: bool,
    save_ram_present: bool,
    pub is_vram: bool,
    pub mapper: u8,
    pub num_prg_pages: usize,
    pub num_chr_pages: usize
}

impl Cart {
    pub fn load_cart(fname: &String) -> Result<Cart, io::Error> {
        use std::io::{Error, ErrorKind};
        let mut f = try!(File::open(fname));
        
        let nes_header_id = try!(f.read_u32_be());
        
        //Check to see if the 'NES ' is there
        if nes_header_id != 0x4e45531A {
            return Err(Error::new(ErrorKind::InvalidInput, "File is not a compatible .nes file"));
        }
        
        let num_prg_pages = try!(f.read_u8());
        let num_chr_pages = try!(f.read_u8());
        let is_vram = num_chr_pages == 0;
        let cart_info = try!(f.read_u8());
        let mirroring = 
            if (cart_info & 0x8) == 0x8 {
                mirroring::FOUR_SCREEN
            } 
            else if (cart_info & 0x1) == 0x1 { 
                mirroring::VERTICAL 
            } 
            else { 
                mirroring::HORIZONTAL 
            };
        let save_ram_present = (cart_info & 0x2) == 0x2;
        let trainer_present = (cart_info & 0x4) == 0x4;
        let mapper_part = try!(f.read_u8());
        let mapper = 
            if mapper_part == 0x44 {
                // Disk dude garbage
                cart_info >> 4
            }
            else if (cart_info == 0x23) && (mapper_part == 0x64) {
                2
            }
            else {
                (cart_info >> 4) + (mapper_part & 0xf0)
            };
        let mut _unused_buffer = [0; 8];
        try!(f.read(&mut _unused_buffer));
        
        let mut prg_rom : Vec<Vec<u8>> = Vec::new();
        for _ in 0..(num_prg_pages*4) {
            let mut buffer = [0; 4096];
            try!(f.read(&mut buffer));
            prg_rom.push(buffer.iter().cloned().collect());
        }
        let mut chr_rom : Vec<Vec<u8>> = Vec::new();
        if num_chr_pages > 0 {            
            for _ in 0..(num_chr_pages*8) {
                let mut buffer = [0; 1024];
                try!(f.read(&mut buffer));
                chr_rom.push(buffer.iter().cloned().collect());
            }    
        }
        else {
            for _ in 0..8 {
                chr_rom.push(vec![0; 1024]);
            }
        }
        
        println!("Prg roms: {}", num_prg_pages * 4);
        println!("Chr roms: {}", num_chr_pages * 8);
        println!("Is_vram: {}", is_vram);
        println!("Mirroring: {:?}", mirroring);
        println!("Save ram present: {}", save_ram_present);
        println!("Trainer present: {}", trainer_present);
        println!("Mapper: {}", mapper);
        
        Ok(Cart {
            prg_rom: prg_rom,
            chr_rom: chr_rom,
            mirroring: mirroring,
            mirroring_base: 0,  // set later by the mapper
            trainer_present: trainer_present,
            save_ram_present: save_ram_present,
            is_vram: is_vram,
            mapper: mapper,
            num_prg_pages: num_prg_pages as usize,
            num_chr_pages: num_chr_pages as usize
        })
    }
}