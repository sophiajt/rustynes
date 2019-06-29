mod cart;
mod cpu;
mod joypad;
mod mmu;
mod nes;
mod ppu;
mod util;

fn main() {
    use std::env::args;

    let cmdline_args: Vec<String> = args().skip(1).collect();

    if cmdline_args.len() == 0 {
        println!("Usage: rustynes <filename>");
        return;
    }

    let result = nes::run_cart(&cmdline_args[0]);
    match result {
        Ok(_) => {}
        Err(e) => println!("Error loading: {}.  {}", cmdline_args[0], e),
    }
}
