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
    }
    else {
        println!("Loading: {}", &cmdline_args[0]);
        let result = nes::run_cart(&cmdline_args[0]);
        match result {
            Ok(_) => {},
            Err(_) => println!("File {} is not in .nes ROM format", cmdline_args[0])
        }
    }
}
