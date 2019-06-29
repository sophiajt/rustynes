use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::TextureAccess;

use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::thread::sleep;

use crate::cart::load_cart;
use crate::cpu::{BreakCondition, Cpu};
use crate::mmu::Mmu;
use crate::ppu::Ppu;

const VISIBLE_WIDTH: u32 = 256;
const VISIBLE_HEIGHT: u32 = 240;

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
    Quit,
}

pub const TICKS_PER_SCANLINE: u32 = 113;

pub fn tick_timer(cpu: &mut Cpu, mmu: &mut Mmu) {
    if mmu.ppu.current_scanline < 240 {
        if mmu.timer_reload_next && mmu.timer_irq_enabled {
            mmu.timer_irq_count = mmu.timer_irq_reload;
            mmu.timer_reload_next = false;
        } else {
            if mmu.timer_irq_enabled {
                if mmu.timer_irq_count == 0 {
                    if mmu.timer_irq_reload > 0 {
                        let pc = cpu.pc;
                        cpu.push_u16(mmu, pc);
                        cpu.push_status(mmu);
                        cpu.pc = mmu.read_u16(0xfffe);
                        cpu.interrupt = true;
                        mmu.timer_irq_enabled = false;
                    } else if mmu.timer_zero_pulse {
                        let pc = cpu.pc;
                        cpu.push_u16(mmu, pc);
                        cpu.push_status(mmu);
                        cpu.pc = mmu.read_u16(0xfffe);
                        cpu.interrupt = true;
                        mmu.timer_zero_pulse = false;
                    }
                    mmu.timer_reload_next = true;
                } else {
                    if mmu.ppu.background_visible || mmu.ppu.sprites_visible {
                        mmu.timer_irq_count -= 1;
                    }
                }
            }
        }
    }
}

pub fn output_ppm(ppu: &Ppu, frame: usize) -> Result<(), io::Error> {
    let fname = format!("screens\\outputfile_{}.ppm", frame);
    let mut f = File::create(fname)?;

    write!(f, "P3\n")?;
    write!(f, "256 240\n")?;
    write!(f, "255\n")?;

    for row in 0..240 {
        for col in 0..256 {
            let pixel = ppu.offscreen_buffer[row * 256 + col];

            write!(
                f,
                "{} {} {} ",
                pixel >> 16,
                (pixel >> 8) & 0xff,
                pixel & 0xff
            )?;
        }
        write!(f, "\n")?;
    }

    Ok(())
}

fn prompt(prev_command: DebuggerCommand, info: &String) -> Result<DebuggerCommand, io::Error> {
    loop {
        print!("{}> ", info);
        io::stdout().flush()?;

        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer)?;

        for line in buffer.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();

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
                    } else {
                        let toframe = parts[1];
                        match usize::from_str_radix(toframe, 10) {
                            Ok(val) => {
                                return Ok(DebuggerCommand::RunCpuUntil(
                                    BreakCondition::RunUntilFrame(val),
                                ))
                            }
                            _ => println!("Supply a frame to break on. Eg: frame 100"),
                        }
                    }
                }
                "sl" => return Ok(DebuggerCommand::RunCpuUntil(BreakCondition::RunToScanline)),
                "next" | "n" => return Ok(DebuggerCommand::RunCpuUntil(BreakCondition::RunNext)),
                "break" | "br" => {
                    if parts.len() < 2 {
                        println!("Supply a PC to break on. Eg: break fffc");
                    } else {
                        let pc = parts[1];
                        match u16::from_str_radix(pc, 16) {
                            Ok(val) => {
                                return Ok(DebuggerCommand::RunCpuUntil(BreakCondition::RunToPc(
                                    val,
                                )))
                            }
                            _ => println!("Supply a PC to break on. Eg: break fffc"),
                        }
                    }
                }
                "print" | "p" => {
                    if parts.len() < 2 {
                        println!("Supply an address to show. Eg: print fffc");
                    } else {
                        let start = parts[1];
                        match u16::from_str_radix(start, 16) {
                            Ok(val) => {
                                if parts.len() == 3 {
                                    match u16::from_str_radix(parts[2], 16) {
                                        Ok(val2) => {
                                            return Ok(DebuggerCommand::PrintAddr(val, val2))
                                        }
                                        _ => println!(
                                            "Supply an end address to show. Eg: print fffc fffe"
                                        ),
                                    }
                                } else if parts.len() == 2 {
                                    return Ok(DebuggerCommand::PrintAddr(val, val));
                                } else {
                                    println!("Too many arguments to print command");
                                }
                            }
                            _ => println!("Supply an address to show. Eg: print fffc"),
                        }
                    }
                }
                "printppu" | "pp" => {
                    if parts.len() < 2 {
                        println!("Supply an address to show. Eg: print fffc");
                    } else {
                        let start = parts[1];
                        match u16::from_str_radix(start, 16) {
                            Ok(val) => {
                                if parts.len() == 3 {
                                    match u16::from_str_radix(parts[2], 16) {
                                        Ok(val2) => {
                                            return Ok(DebuggerCommand::PrintPpuAddr(val, val2))
                                        }
                                        _ => println!(
                                            "Supply an end address to show. Eg: print fffc fffe"
                                        ),
                                    }
                                } else if parts.len() == 2 {
                                    return Ok(DebuggerCommand::PrintPpuAddr(val, val));
                                } else {
                                    println!("Too many arguments to print command");
                                }
                            }
                            _ => println!("Supply an address to show. Eg: print fffc"),
                        }
                    }
                }
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
                }
                _ => println!("Use 'help' to see commands"),
            }
        }
    }
}

fn print_addr(mmu: &mut Mmu, addr1: u16, addr2: u16) {
    let mut idx = 0;

    loop {
        if idx % 16 == 0 {
            print!("{0:04x}: ", addr1 + idx);
        }
        print!("{0:02x} ", mmu.read_u8(addr1 + idx));

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

fn print_ppu_addr(mmu: &mut Mmu, addr1: u16, addr2: u16) {
    let mut idx = 0;

    loop {
        if idx % 16 == 0 {
            print!("{0:04x}: ", addr1 + idx);
        }
        if ((addr1 + idx) >= 0x2000) && ((addr1 + idx) < 0x4000) {
            print!(
                "{0:02x} ",
                mmu.ppu.name_tables[(addr1 as usize) - 0x2000 + (idx as usize)]
            );
        } else if (addr1 + idx) < 0x2000 {
            print!(
                "{0:02x} ",
                mmu.ppu.read_chr_rom((addr1 as usize) + (idx as usize))
            );
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

fn draw_frame_and_pump_events(
    mmu: &mut Mmu,
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    texture: &mut sdl2::render::Texture,
    event_pump: &mut sdl2::EventPump,
) -> bool {
    texture
        .with_lock(None, |buffer: &mut [u8], pitch: usize| {
            for row in 0..(VISIBLE_HEIGHT as usize) {
                for col in 0..(VISIBLE_WIDTH as usize) {
                    let pixel = mmu.ppu.offscreen_buffer[row * 256 + col];
                    let offset = row * pitch * 2 + col * 3 * 2;

                    let red = (pixel >> 16) as u8;
                    let green = ((pixel >> 8) & 0xff) as u8;
                    let blue = (pixel & 0xff) as u8;

                    buffer[offset + 0] = red;
                    buffer[offset + 3] = red;
                    buffer[offset + pitch] = red;
                    buffer[offset + pitch + 3] = red;
                    buffer[offset + 1] = green;
                    buffer[offset + 4] = green;
                    buffer[offset + pitch + 1] = green;
                    buffer[offset + pitch + 4] = green;
                    buffer[offset + 2] = blue;
                    buffer[offset + 5] = blue;
                    buffer[offset + pitch + 2] = blue;
                    buffer[offset + pitch + 5] = blue;
                }
            }
        })
        .unwrap();

    canvas.clear();
    let _ = canvas.copy(
        &texture,
        None,
        Some(Rect::new(0, 0, VISIBLE_WIDTH * 2, VISIBLE_HEIGHT * 2)),
    );

    canvas.present();

    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => return true,
            _ => (),
        }
    }

    let keys = event_pump
        .keyboard_state()
        .pressed_scancodes()
        .filter_map(Keycode::from_scancode)
        .collect();

    mmu.joypad.update_keys(keys);

    false
}

pub fn run_cart(fname: &String, use_debug: bool) -> Result<(), io::Error> {
    use std::cmp;

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("rustynes", VISIBLE_WIDTH * 2, VISIBLE_HEIGHT * 2)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture(
            PixelFormatEnum::RGB24,
            TextureAccess::Streaming,
            VISIBLE_WIDTH * 2,
            VISIBLE_HEIGHT * 2,
        )
        .unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut timer = sdl_context.timer().unwrap();

    let mut prev_timer_ticks: u64 = timer.ticks() as u64;
    let mut curr_timer_ticks: u64;
    const TIMER_TICKS_PER_FRAME: u64 = 1000 / 60;

    let mut mmu = Mmu::new();

    //Load the cart contents into the MMU and PPU
    load_cart(fname, &mut mmu)?;

    let mut cpu = Cpu::new();
    let mut frame_count = 0;
    let mut debug_info: String;
    let mut show_cpu = true;
    let mut show_mem = false;
    let mut prev_command = DebuggerCommand::Nop;

    //Create all our memory handlers, and hand off ownership
    //of the cart to contained mmu

    cpu.reset(&mut mmu);

    if !use_debug {
        'gameloop: loop {
            cpu.run_for_scanline(&mut mmu);
            cpu.tick_count -= TICKS_PER_SCANLINE;
            let execute_interrupt = mmu.ppu.render_scanline();
            if execute_interrupt {
                let pc = cpu.pc;
                cpu.push_u16(&mut mmu, pc);
                cpu.push_status(&mut mmu);
                cpu.pc = mmu.read_u16(0xfffa);
            }

            if mmu.ppu.mapper == 4 {
                tick_timer(&mut cpu, &mut mmu);
            }

            if mmu.ppu.current_scanline == 240 {
                let exiting = draw_frame_and_pump_events(
                    &mut mmu,
                    &mut canvas,
                    &mut texture,
                    &mut event_pump,
                );
                if exiting {
                    break 'gameloop;
                }
                curr_timer_ticks = timer.ticks() as u64;
                if (curr_timer_ticks - prev_timer_ticks) < TIMER_TICKS_PER_FRAME {
                    sleep(std::time::Duration::from_millis(
                        TIMER_TICKS_PER_FRAME - (curr_timer_ticks - prev_timer_ticks),
                    ));
                }
                prev_timer_ticks = curr_timer_ticks;

                frame_count += 1;
            }
        }
    } else {
        let mut cond_met;
        'gameloop_debug: loop {
            if show_cpu {
                cpu.fetch(&mut mmu);
                debug_info = format!("[{:?}]", cpu);
            } else {
                debug_info = String::new();
            }

            if show_mem {
                print_addr(&mut mmu, cpu.pc, cpu.pc + cmp::min(5, 0xffff - cpu.pc));
            }

            let command = prompt(prev_command, &debug_info)?;
            prev_command = command.clone();
            match command {
                DebuggerCommand::Quit => break,
                DebuggerCommand::Nop => {}
                DebuggerCommand::Ppm => output_ppm(&mmu.ppu, frame_count)?,
                DebuggerCommand::ShowPpu => println!("{:?}", mmu.ppu),
                DebuggerCommand::ToggleShowCpu => show_cpu = !show_cpu,
                DebuggerCommand::ToggleShowMem => show_mem = !show_mem,
                DebuggerCommand::PrintAddr(addr1, addr2) => print_addr(&mut mmu, addr1, addr2),
                DebuggerCommand::PrintPpuAddr(addr1, addr2) => {
                    print_ppu_addr(&mut mmu, addr1, addr2)
                }
                DebuggerCommand::ToggleDebug => cpu.is_debugging = !cpu.is_debugging,
                DebuggerCommand::RunCpuUntil(cond) => {
                    cond_met = false;
                    while !cond_met {
                        cond_met = cpu.run_until_condition(&mut mmu, &cond);

                        if cpu.tick_count >= TICKS_PER_SCANLINE {
                            cpu.tick_count -= TICKS_PER_SCANLINE;

                            let execute_interrupt = mmu.ppu.render_scanline();
                            if execute_interrupt {
                                let pc = cpu.pc;
                                cpu.push_u16(&mut mmu, pc);
                                cpu.push_status(&mut mmu);
                                cpu.pc = mmu.read_u16(0xfffa);
                            }

                            if mmu.ppu.mapper == 4 {
                                tick_timer(&mut cpu, &mut mmu);
                            }

                            if mmu.ppu.current_scanline == 240 {
                                let exiting = draw_frame_and_pump_events(
                                    &mut mmu,
                                    &mut canvas,
                                    &mut texture,
                                    &mut event_pump,
                                );
                                if exiting {
                                    break 'gameloop_debug;
                                }

                                curr_timer_ticks = timer.ticks() as u64;
                                if (curr_timer_ticks - prev_timer_ticks) < TIMER_TICKS_PER_FRAME {
                                    sleep(std::time::Duration::from_millis(
                                        TIMER_TICKS_PER_FRAME
                                            - (curr_timer_ticks - prev_timer_ticks),
                                    ));
                                }
                                prev_timer_ticks = curr_timer_ticks;
                                frame_count += 1;

                                match cond {
                                    BreakCondition::RunFrame => cond_met = true,
                                    BreakCondition::RunUntilFrame(f) => {
                                        if frame_count == f {
                                            cond_met = true;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if mmu.save_ram_present {
        let mut out_save_file = File::create(mmu.save_ram_file_name);
        match out_save_file {
            Ok(ref mut f) => match f.write(&mmu.save_ram[..]) {
                _ => {}
            },
            _ => {}
        }
    }

    Ok(())
}
