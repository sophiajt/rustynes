use sdl2;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use std::io;
use std::io::Error;
use std::io::prelude::*;
use std::fs::File;
use std::thread::sleep_ms;

use cpu::{Cpu, BreakCondition};
use cart::Cart;
use ppu::Ppu;
use mmu::Mmu;

#[derive(Clone)]
enum DebuggerCommand {
    RunCpuUntil(BreakCondition),
    ToggleShowCpu,
    ToggleShowMem,
    ToggleDebug,
    ShowPpu,
    PrintAddr(u16, u16),
    PrintPpuAddr(u16, u16),
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
        let mut mmu = Mmu::new(cart);
        
        //Before we're done, we also have to reset our mapper to 
        //its default
        mmu.setup_defaults();
        
        Memory { ppu: ppu, mmu: mmu }
    }
}

pub const TICKS_PER_SCANLINE : u32 = 113;

pub fn tick_timer(cpu: &mut Cpu, mem: &mut Memory) {
    if mem.ppu.current_scanline < 240 {
        if mem.mmu.timer_reload_next && mem.mmu.timer_irq_enabled {
            mem.mmu.timer_irq_count = mem.mmu.timer_irq_reload;
            mem.mmu.timer_reload_next = false;
        }
        else {
            if mem.mmu.timer_irq_enabled {
                if mem.mmu.timer_irq_count == 0 {
                    if mem.mmu.timer_irq_reload > 0 {
                        let pc = cpu.pc;
                        cpu.push_u16(mem, pc);
                        cpu.push_status(mem);
                        cpu.pc = mem.mmu.read_u16(&mut mem.ppu, 0xfffe);
                        cpu.interrupt = true;
                        mem.mmu.timer_irq_enabled = false;
                    }
                    else if mem.mmu.timer_zero_pulse {
                        let pc = cpu.pc;
                        cpu.push_u16(mem, pc);
                        cpu.push_status(mem);
                        cpu.pc = mem.mmu.read_u16(&mut mem.ppu, 0xfffe);
                        cpu.interrupt = true;
                        mem.mmu.timer_zero_pulse = false;
                    }
                    mem.mmu.timer_reload_next = true;
                }
                else {
                    if mem.ppu.background_visible || mem.ppu.sprites_visible {
                        mem.mmu.timer_irq_count -= 1;
                    }
                }
            }
        }
    }
}


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
    
    ppu.fix_scroll_reset = 
        (cart.prg_rom[0][0xfeb - 0x10] == 0xFA) &&
        (cart.prg_rom[0][0xfec - 0x10] == 0xA9) &&
        (cart.prg_rom[0][0xfed - 0x10] == 0x18);
        
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

fn prompt(prev_command: DebuggerCommand, info: &String) -> Result<DebuggerCommand, io::Error> {
    loop {
        print!("{}> ", info);
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
                "cpu" => return Ok(DebuggerCommand::ToggleShowCpu),
                "mem" => return Ok(DebuggerCommand::ToggleShowMem),
                "ppu" => return Ok(DebuggerCommand::ShowPpu),
                "debug" => return Ok(DebuggerCommand::ToggleDebug),
                "ppm" => return Ok(DebuggerCommand::Ppm),
                "frame" | "fr" => {
                    if parts.len() == 1 {
                        return Ok(DebuggerCommand::RunCpuUntil(BreakCondition::RunFrame));
                    }
                    else {
                        let toframe = parts[1];
                        match usize::from_str_radix(toframe, 10) {
                            Ok(val) => return Ok(DebuggerCommand::RunCpuUntil(BreakCondition::RunUntilFrame(val))),
                            _ => println!("Supply a frame to break on. Eg: frame 100")
                        }
                    }
                },
                "sl" => return Ok(DebuggerCommand::RunCpuUntil(BreakCondition::RunToScanline)),
                "next" | "n" => return Ok(DebuggerCommand::RunCpuUntil(BreakCondition::RunNext)),
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
                "print" | "p" => {
                        if parts.len() < 2 {
                            println!("Supply an address to show. Eg: print fffc");
                        }
                        else {
                            let start = parts[1];
                            match u16::from_str_radix(start, 16) {
                                Ok(val) => {
                                    if parts.len() == 3 {
                                        match u16::from_str_radix(parts[2], 16) {
                                            Ok(val2) => return Ok(DebuggerCommand::PrintAddr(val, val2)),
                                            _ => println!("Supply an end address to show. Eg: print fffc fffe")
                                        }
                                    }
                                    else if parts.len() == 2 {
                                        return Ok(DebuggerCommand::PrintAddr(val, val));                                    
                                    }
                                    else {
                                        println!("Too many arguments to print command");
                                    }
                                },
                                _ => println!("Supply an address to show. Eg: print fffc")
                            }                            
                        }
                    },
                "printppu" | "pp" => {
                        if parts.len() < 2 {
                            println!("Supply an address to show. Eg: print fffc");
                        }
                        else {
                            let start = parts[1];
                            match u16::from_str_radix(start, 16) {
                                Ok(val) => {
                                    if parts.len() == 3 {
                                        match u16::from_str_radix(parts[2], 16) {
                                            Ok(val2) => return Ok(DebuggerCommand::PrintPpuAddr(val, val2)),
                                            _ => println!("Supply an end address to show. Eg: print fffc fffe")
                                        }
                                    }
                                    else if parts.len() == 2 {
                                        return Ok(DebuggerCommand::PrintPpuAddr(val, val));                                    
                                    }
                                    else {
                                        println!("Too many arguments to print command");
                                    }
                                },
                                _ => println!("Supply an address to show. Eg: print fffc")
                            }                            
                        }
                    },
                "help" | "h" => {
                    println!("Commands available:");
                    println!("  q(uit): leave debugger");
                    println!("  cpu: toggle showing cpu contents");
                    println!("  mem: toggle showing mem contents");
                    println!("  debug: toggle cpu verbose debug");
                    println!("  ppu: show ppu contents");
                    println!("  fr(ame) (<num>): run until next video frame or #num");
                    println!("  br(eak) <addr>: run until pc == addr");
                    println!("  sl: run until next scanline");
                    println!("  n(ext): run until next instruction");
                    println!("  p(rint) <addr> (<end addr>): show memory at addr");
                    println!("  pp <addr> (<end addr>): show ppu memory at addr");
                    println!("  ppm: save ppm of current video frame to 'screens'");
                },
                _ => println!("Use 'help' to see commands")
            }
        }
    }
}

fn print_addr(mem: &mut Memory, addr1: u16, addr2: u16) {
    let mut idx = 0;
    
    loop {
        if idx % 16 == 0 {
            print!("{0:04x}: ", addr1 + idx);
        }
        print!("{0:02x} ", mem.mmu.read_u8(&mut mem.ppu, addr1 + idx));
        
        if (addr1 + idx) == addr2 {
            break;
        }
        
        idx += 1;
        
        if (idx % 16) == 0 {
            println!("");
        }
    }
    println!("");
}

fn print_ppu_addr(mem: &mut Memory, addr1: u16, addr2: u16) {
    let mut idx = 0;
    
    loop {
        if idx % 16 == 0 {
            print!("{0:04x}: ", addr1 + idx);
        }
        if ((addr1 + idx) >= 0x2000) && ((addr1 + idx) < 0x4000) {
            print!("{0:02x} ", mem.ppu.name_tables[(addr1 as usize) - 0x2000 + (idx as usize)]);
        }
        else if (addr1 + idx) < 0x2000 {
            print!("{0:02x} ", mem.mmu.read_chr_rom((addr1 as usize) + (idx as usize)));
        }
        if (addr1 + idx) == addr2 {
            break;
        }
        
        idx += 1;
        
        if (idx % 16) == 0 {
            println!("");
        }
    }
    println!("");
}

fn draw_frame_and_pump_events(mem: &mut Memory, renderer: &mut sdl2::render::Renderer, texture: &mut sdl2::render::Texture,
    event_pump: &mut sdl2::EventPump) -> bool {
    
    texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
        for row in 0..240 {
            for col in 0..256 {
                let pixel = mem.ppu.offscreen_buffer[row * 256 + col];
                let offset = row*pitch + col*3;
                buffer[offset + 0] = (pixel >> 16) as u8;
                buffer[offset + 1] = ((pixel >> 8) & 0xff) as u8;
                buffer[offset + 2] = (pixel & 0xff) as u8;
            }
        }
    }).unwrap();

    renderer.clear();
    renderer.copy(&texture, None, Some(Rect::new_unwrap(0, 0, 256, 240)));
    renderer.present();
    
    for event in event_pump.poll_iter() {
        match event {
            Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => 
                return true,
            _ => ()
        }
    }
    
    let keys = event_pump.keyboard_state().pressed_scancodes().
        filter_map(Keycode::from_scancode).collect();
    mem.mmu.joypad.update_keys(keys);
    
    false
}

pub fn run_cart(fname: &String, use_debug: bool) -> Result<(), io::Error> {
    use std::cmp;
    
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("rustynes", 256, 240)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut renderer = window.renderer().build().unwrap();

    let mut texture = renderer.create_texture_streaming(PixelFormatEnum::RGB24, (256, 240)).unwrap();
    
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut timer = sdl_context.timer().unwrap();
    
    let mut prev_timer_ticks : u32 = timer.ticks();
    let mut curr_timer_ticks : u32;
    const TIMER_TICKS_PER_FRAME : u32 = 1000 / 60;
       
    let cart = try!(Cart::load_cart(fname));
    let mut cpu = Cpu::new();
    let mut frame_count = 0;
    let mut debug_info : String;
    let mut show_cpu = true;
    let mut show_mem = false;
    let mut prev_command = DebuggerCommand::Nop;

    //Create all our memory handlers, and hand off ownership
    //of the cart to contained mmu
    let mut mem = Memory::new(cart);
    
    cpu.reset(&mut mem);
    
    if !use_debug {
        'gameloop: loop {
            cpu.run_for_scanline(&mut mem);
            cpu.tick_count -= TICKS_PER_SCANLINE;
            let execute_interrupt = mem.ppu.render_scanline(&mut mem.mmu);
            if execute_interrupt {
                let pc = cpu.pc;
                cpu.push_u16(&mut mem, pc);
                cpu.push_status(&mut mem);
                cpu.pc = mem.mmu.read_u16(&mut mem.ppu, 0xfffa);
            }
            
            if mem.mmu.cart.mapper == 4 {
                tick_timer(&mut cpu, &mut mem);
            }

            if mem.ppu.current_scanline == 240 {
                let exiting = draw_frame_and_pump_events(&mut mem, &mut renderer, &mut texture, &mut event_pump);
                if exiting { break 'gameloop }
                curr_timer_ticks = timer.ticks();
                if (curr_timer_ticks - prev_timer_ticks) < TIMER_TICKS_PER_FRAME {
                    sleep_ms(TIMER_TICKS_PER_FRAME - (curr_timer_ticks - prev_timer_ticks));
                }
                prev_timer_ticks = curr_timer_ticks;
    
                frame_count += 1;
            }

            /*
            if mem.ppu.current_scanline == 0 {
                println!("Frame count: {}", frame_count); 
            }
            */
        }
    }
    else {
        let mut cond_met;
        'gameloop_debug: loop {
            if show_cpu {
                cpu.fetch(&mut mem);
                debug_info = format!("[{:?}]", cpu);
            }
            else {
                debug_info = String::new();
            }
            
            if show_mem {
                print_addr(&mut mem, cpu.pc, cpu.pc + cmp::min(5, 0xffff - cpu.pc));
            }
            
            let command = try!(prompt(prev_command, &debug_info));
            prev_command = command.clone();
            match command {
                DebuggerCommand::Quit => break,
                DebuggerCommand::Nop => {},
                DebuggerCommand::Ppm => try!(output_ppm(&mem.ppu, frame_count)),
                DebuggerCommand::ShowPpu => println!("{:?}", mem.ppu),
                DebuggerCommand::ToggleShowCpu => show_cpu = !show_cpu,
                DebuggerCommand::ToggleShowMem => show_mem = !show_mem,
                DebuggerCommand::PrintAddr(addr1, addr2) => print_addr(&mut mem, addr1, addr2),
                DebuggerCommand::PrintPpuAddr(addr1, addr2) => print_ppu_addr(&mut mem, addr1, addr2),
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

                            if mem.mmu.cart.mapper == 4 {
                                tick_timer(&mut cpu, &mut mem);
                            }
                            
                            if mem.ppu.current_scanline == 240 {
                                let exiting = draw_frame_and_pump_events(&mut mem, &mut renderer, &mut texture, &mut event_pump);
                                if exiting { break 'gameloop_debug }
                                
                                curr_timer_ticks = timer.ticks();
                                if (curr_timer_ticks - prev_timer_ticks) < TIMER_TICKS_PER_FRAME {
                                    sleep_ms(TIMER_TICKS_PER_FRAME - (curr_timer_ticks - prev_timer_ticks));
                                }
                                prev_timer_ticks = curr_timer_ticks;
                                frame_count += 1;
    
                                match cond {
                                    BreakCondition::RunFrame => cond_met = true,
                                    BreakCondition::RunUntilFrame(f) => if frame_count == f { cond_met = true; },
                                    _ => {}
                                }                            
                            }
                        }
                    }
                }
            }
        }
    }
            
    Ok(())
}

