extern crate sdl2;

mod util;
mod cpu;
mod joypad;
mod mmu;
mod cart;
mod ppu;
mod nes;

fn main() {
    use std::env::args;

    let cmdline_args : Vec<String> = args().skip(1).collect();
    
    if cmdline_args.len() == 0 {
        println!("Usage: rustynes <filename>");
        return;
    }
    
    let use_debug = (cmdline_args.len() == 2) && (cmdline_args[1] == "--debug"); 
    
    //println!("Loading: {}", &cmdline_args[0]);
    let result = nes::run_cart(&cmdline_args[0], use_debug);
    match result {
        Ok(_) => {},
        Err(e) => println!("Error loading: {}.  {}", cmdline_args[0], e)
    }
}
