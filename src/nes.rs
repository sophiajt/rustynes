use std::io;
use std::io::Error;
use std::io::prelude::*;
use std::fs::File;

use cpu::{Cpu, BreakCondition};
use cart::Cart;
use ppu::Ppu;
use mmu::Mmu;

#[derive(Clone)]
enum DebuggerCommand {
    RunCpuUntil(BreakCondition),
    ShowCpu,
    ToggleDebug,
    Nop,
    Ppm,
    Quit
}

// Everything we need to read and write from memory
pub struct Memory {
    pub ppu: Ppu,
    pub mmu: Mmu
}

impl Memory {
    pub fn new(cart: Cart) -> Memory {
        let mut ppu = Ppu::new();

        //Add in our workarounds first
        configure_ppu_for_cart(&mut ppu, &cart);
    
        //Then hand off cart, after this the MMU is the only one
        //who can access program memory
        let mmu = Mmu::new(cart);
        
        Memory { ppu: ppu, mmu: mmu }
    }
}

pub const TICKS_PER_SCANLINE : u32 = 113;

fn configure_ppu_for_cart(ppu: &mut Ppu, cart: &Cart) {
    //Check for workarounds
    ppu.fix_bg_change =
        (cart.prg_rom[cart.num_prg_pages - 1][0xfeb] == b'Z') &&
        (cart.prg_rom[cart.num_prg_pages - 1][0xfec] == b'E') &&
        (cart.prg_rom[cart.num_prg_pages - 1][0xfed] == b'L') &&
        (cart.prg_rom[cart.num_prg_pages - 1][0xfee] == b'D') &&
        (cart.prg_rom[cart.num_prg_pages - 1][0xfef] == b'A');
    
    ppu.fix_scroll_offset_1 =
        (cart.prg_rom[cart.num_prg_pages - 1][0xfe0] == b'B') &&
        (cart.prg_rom[cart.num_prg_pages - 1][0xfe1] == b'B') &&
        (cart.prg_rom[cart.num_prg_pages - 1][0xfe2] == b'4') &&
        (cart.prg_rom[cart.num_prg_pages - 1][0xfe3] == b'7') &&
        (cart.prg_rom[cart.num_prg_pages - 1][0xfe4] == b'9') &&
        (cart.prg_rom[cart.num_prg_pages - 1][0xfe5] == b'5') &&
        (cart.prg_rom[cart.num_prg_pages - 1][0xfe6] == b'6') &&
        (cart.prg_rom[cart.num_prg_pages - 1][0xfe7] == b'-') &&
        (cart.prg_rom[cart.num_prg_pages - 1][0xfe8] == b'1') &&
        (cart.prg_rom[cart.num_prg_pages - 1][0xfe9] == b'5') &&
        (cart.prg_rom[cart.num_prg_pages - 1][0xfea] == b'4') &&
        (cart.prg_rom[cart.num_prg_pages - 1][0xfeb] == b'4') &&
        (cart.prg_rom[cart.num_prg_pages - 1][0xfec] == b'0');
        
    ppu.fix_scroll_offset_2 = 
        (cart.prg_rom[0][0x9] == 0xfc) &&
        (cart.prg_rom[0][0xa] == 0xfc) &&
        (cart.prg_rom[0][0xb] == 0xfc) &&
        (cart.prg_rom[0][0xc] == 0x40) &&
        (cart.prg_rom[0][0xd] == 0x40) &&
        (cart.prg_rom[0][0xe] == 0x40) &&
        (cart.prg_rom[0][0xf] == 0x40);
        
    ppu.fix_scroll_offset_3 = 
        (cart.prg_rom[0][0x75] == 0x11) &&
        (cart.prg_rom[0][0x76] == 0x12) &&
        (cart.prg_rom[0][0x77] == 0x13) &&
        (cart.prg_rom[0][0x78] == 0x14) &&
        (cart.prg_rom[0][0x79] == 0x07) && 
        (cart.prg_rom[0][0x7a] == 0x03) && 
        (cart.prg_rom[0][0x7b] == 0x03) && 
        (cart.prg_rom[0][0x7c] == 0x03) && 
        (cart.prg_rom[0][0x7d] == 0x03);  
    
}

pub fn output_ppm(ppu: &Ppu, frame: usize) -> Result<(), io::Error> {
    let fname = format!("screens\\outputfile_{}.ppm", frame);
    let mut f = try!(File::create(fname));

    try!(write!(f, "P3\n"));
    try!(write!(f, "256 240\n"));
    try!(write!(f, "255\n"));
    
    for row in 0..240 {
        for col in 0..256 {
            let pixel = ppu.offscreen_buffer[row * 256 + col];
            
            try!(write!(f, "{} {} {} ", pixel >> 16, (pixel >> 8) & 0xff, pixel & 0xff));
        }
        try!(write!(f, "\n"));
    }
    
    Ok(())
}

fn prompt(prev_command: DebuggerCommand) -> Result<DebuggerCommand, io::Error> {
    loop {
        print!("> ");
        try!(io::stdout().flush());
        
        let mut buffer = String::new();
        try!(io::stdin().read_line(&mut buffer));
        
        for line in buffer.lines() {
            let parts : Vec<&str> = line.split_whitespace().collect();
            
            if parts.len() == 0 {
                return Ok(prev_command);
            }
            
            match parts[0] {
                "quit" | "q" => return Ok(DebuggerCommand::Quit),
                "cpu" => return Ok(DebuggerCommand::ShowCpu),
                "debug" => return Ok(DebuggerCommand::ToggleDebug),
                "ppm" => return Ok(DebuggerCommand::Ppm),
                "fr" => return Ok(DebuggerCommand::RunCpuUntil(BreakCondition::RunFrame)),
                "sl" => return Ok(DebuggerCommand::RunCpuUntil(BreakCondition::RunToScanline)),
                "n" => return Ok(DebuggerCommand::RunCpuUntil(BreakCondition::RunNext)),
                "break" | "br" => {
                        if parts.len() < 2 {
                            println!("Supply a PC to break on. Eg: break fffc");
                        }
                        else {
                            let pc = parts[1];
                            match u16::from_str_radix(pc, 16) {
                                Ok(val) => return Ok(DebuggerCommand::RunCpuUntil(BreakCondition::RunToPc(val))),
                                _ => println!("Supply a PC to break on. Eg: break fffc")
                            }
                        }
                    },
                "help" => {
                    println!("Commands available:");
                    println!("  q:   leave debugger");
                    println!("  cpu: print cpu contents");
                    println!("  debug: toggle cpu verbose debug");
                    println!("  fr:  run until next video frame");
                    println!("  br <addr>: run until pc == addr");
                    println!("  sl:  run until next scanline");
                    println!("  n:   run until next instruction");
                    println!("  ppm: save ppm of current video frame to 'screens'");
                },
                _ => println!("Use 'help' to see commands")
            }
        }
    }
}

pub fn run_cart(fname: &String) -> Result<(), io::Error> {        
    let cart = try!(Cart::load_cart(fname));
    let mut cpu = Cpu::new();
    let mut frame_count = 0;
    let mut prev_command = DebuggerCommand::Nop;

    //Create all our memory handlers, and hand off ownership
    //of the cart to contained mmu
    let mut mem = Memory::new(cart);
    
    cpu.reset(&mut mem);
    
    let mut cond_met;
    loop {
        if cpu.is_debugging {
            print!("[{:?}] ", cpu);
        }
        let command = try!(prompt(prev_command));
        prev_command = command.clone();
        match command {
            DebuggerCommand::Quit => break,
            DebuggerCommand::Nop => {},
            DebuggerCommand::Ppm => try!(output_ppm(&mem.ppu, frame_count)),
            DebuggerCommand::ShowCpu => println!("{:?}", cpu),
            DebuggerCommand::ToggleDebug => cpu.is_debugging = !cpu.is_debugging,
            DebuggerCommand::RunCpuUntil(cond) => {
                cond_met = false;
                while !cond_met {
                    cond_met = cpu.run_until_condition(&mut mem, &cond);
                    
                    if cpu.tick_count >= TICKS_PER_SCANLINE {
                        cpu.tick_count -= TICKS_PER_SCANLINE;
                        
                        let execute_interrupt = mem.ppu.render_scanline(&mut mem.mmu);
                        if execute_interrupt {
                            let pc = cpu.pc;
                            cpu.push_u16(&mut mem, pc);
                            cpu.push_status(&mut mem);
                            cpu.pc = mem.mmu.read_u16(&mut mem.ppu, 0xfffa);
                        }
                        
                        if mem.ppu.current_scanline == 240 {
                            //output_ppm(&mem.ppu, frame_count);
                            match cond {
                                BreakCondition::RunFrame => cond_met = true,
                                _ => {}
                            }
                            
                            println!("Frame: {}", frame_count);
                            frame_count += 1;
                        }
                        
                        mem.mmu.tick_timer();
                    }
                }
            }
        }
    }
        
    Ok(())
}

