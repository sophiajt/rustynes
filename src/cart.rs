use std::io;
use std::io::prelude::*;
use std::fs::File;
use util::{BitReader, Joiner};
use mmu::Mmu;
use ppu::mirroring;

fn configure_ppu_for_cart(mmu: &mut Mmu) {
    //Check for workarounds
    mmu.ppu.fix_bg_change =
        (mmu.prg_rom[mmu.num_prg_pages - 1][0xfeb] == b'Z') &&
        (mmu.prg_rom[mmu.num_prg_pages - 1][0xfec] == b'E') &&
        (mmu.prg_rom[mmu.num_prg_pages - 1][0xfed] == b'L') &&
        (mmu.prg_rom[mmu.num_prg_pages - 1][0xfee] == b'D') &&
        (mmu.prg_rom[mmu.num_prg_pages - 1][0xfef] == b'A');
    
    mmu.ppu.fix_scroll_offset_1 =
        (mmu.prg_rom[mmu.num_prg_pages - 1][0xfe0] == b'B') &&
        (mmu.prg_rom[mmu.num_prg_pages - 1][0xfe1] == b'B') &&
        (mmu.prg_rom[mmu.num_prg_pages - 1][0xfe2] == b'4') &&
        (mmu.prg_rom[mmu.num_prg_pages - 1][0xfe3] == b'7') &&
        (mmu.prg_rom[mmu.num_prg_pages - 1][0xfe4] == b'9') &&
        (mmu.prg_rom[mmu.num_prg_pages - 1][0xfe5] == b'5') &&
        (mmu.prg_rom[mmu.num_prg_pages - 1][0xfe6] == b'6') &&
        (mmu.prg_rom[mmu.num_prg_pages - 1][0xfe7] == b'-') &&
        (mmu.prg_rom[mmu.num_prg_pages - 1][0xfe8] == b'1') &&
        (mmu.prg_rom[mmu.num_prg_pages - 1][0xfe9] == b'5') &&
        (mmu.prg_rom[mmu.num_prg_pages - 1][0xfea] == b'4') &&
        (mmu.prg_rom[mmu.num_prg_pages - 1][0xfeb] == b'4') &&
        (mmu.prg_rom[mmu.num_prg_pages - 1][0xfec] == b'0');
        
    mmu.ppu.fix_scroll_offset_2 = 
        (mmu.prg_rom[0][0x9] == 0xfc) &&
        (mmu.prg_rom[0][0xa] == 0xfc) &&
        (mmu.prg_rom[0][0xb] == 0xfc) &&
        (mmu.prg_rom[0][0xc] == 0x40) &&
        (mmu.prg_rom[0][0xd] == 0x40) &&
        (mmu.prg_rom[0][0xe] == 0x40) &&
        (mmu.prg_rom[0][0xf] == 0x40);
        
    mmu.ppu.fix_scroll_offset_3 = 
        (mmu.prg_rom[0][0x75] == 0x11) &&
        (mmu.prg_rom[0][0x76] == 0x12) &&
        (mmu.prg_rom[0][0x77] == 0x13) &&
        (mmu.prg_rom[0][0x78] == 0x14) &&
        (mmu.prg_rom[0][0x79] == 0x07) && 
        (mmu.prg_rom[0][0x7a] == 0x03) && 
        (mmu.prg_rom[0][0x7b] == 0x03) && 
        (mmu.prg_rom[0][0x7c] == 0x03) && 
        (mmu.prg_rom[0][0x7d] == 0x03);  
    
    mmu.ppu.fix_scroll_reset = 
        (mmu.prg_rom[0][0xfeb - 0x10] == 0xFA) &&
        (mmu.prg_rom[0][0xfec - 0x10] == 0xA9) &&
        (mmu.prg_rom[0][0xfed - 0x10] == 0x18);
        
}


pub fn load_cart(fname: &String, mmu: &mut Mmu) -> Result<(), io::Error> {
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
    let _ = (cart_info & 0x4) == 0x4; //trainer_present
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

    if !(vec![0, 1, 2, 3, 4].contains(&mapper)) {
        return Err(Error::new(ErrorKind::InvalidInput, format!("Unsupport mapper: {}", mapper)));
    }

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

    mmu.prg_rom = prg_rom;
    mmu.ppu.chr_rom = chr_rom;
    mmu.ppu.mirroring = mirroring;
    mmu.ppu.mirroring_base = 0;
    mmu.save_ram_present = save_ram_present;
    mmu.ppu.is_vram = is_vram;
    mmu.ppu.mapper = mapper;
    mmu.num_prg_pages = num_prg_pages as usize;
    mmu.ppu.num_chr_pages = num_chr_pages as usize;

    mmu.setup_defaults();
    configure_ppu_for_cart(mmu);

    if save_ram_present {
        let mut fname_split: Vec<&str> = fname.split('.').collect();
        let save_file_name = 
            match fname_split.last() {
                Some(&"nes") => {fname_split.pop(); fname_split.push("sav"); fname_split.join('.')},
                _ => {fname_split.push("sav"); fname_split.join('.')}
            };

        let mut save_file_open = File::open(save_file_name.clone());
        match save_file_open {
            Ok(ref mut save_file) => {
                let mut buff = [0; 0x2000];
                let result = save_file.read(&mut buff);
                match result {
                    Ok(_) => mmu.save_ram = buff.iter().cloned().collect(),
                    _ => {}
                }
            },
            _ => {}
        }
        mmu.save_ram_file_name = save_file_name;
    }

    /*
    println!("Prg roms: {}", num_prg_pages * 4);
    println!("Chr roms: {}", num_chr_pages * 8);
    println!("Is_vram: {}", is_vram);
    println!("Mirroring: {:?}", mirroring);
    println!("Save ram present: {}", save_ram_present);
    println!("Trainer present: {}", trainer_present);
    println!("Mapper: {}", mapper);
    */

    Ok(())
}
