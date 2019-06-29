use crossterm::{cursor, terminal, AsyncReader, Attribute, Color, Colored};
use std::error::Error;
use std::f32;

use crossterm::{Crossterm, InputEvent, KeyEvent, RawScreen};

use crate::joypad::JoyButton;
use std::fs::File;
use std::io::prelude::*;

use crate::cart::load_cart;
use crate::cpu::Cpu;
use crate::mmu::Mmu;

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

pub struct Context {
    pub width: usize,
    pub height: usize,
    pub frame_buffer: Vec<(char, (u8, u8, u8))>,
    pub z_buffer: Vec<f32>,
    pub since_last_button: Vec<usize>,
}

impl Context {
    pub fn blank() -> Context {
        //TODO: Make this a constant struct
        Context {
            width: 0,
            height: 0,
            frame_buffer: vec![],
            z_buffer: vec![],
            since_last_button: vec![0; 8],
        }
    }
    pub fn clear(&mut self) {
        self.frame_buffer = vec![(' ', (0, 0, 0)); self.width * self.height as usize];
        self.z_buffer = vec![f32::MAX; self.width * self.height as usize]; //f32::MAX is written to the z-buffer as an infinite back-wall to render with
    }
    pub fn flush(&self) -> Result<(), Box<dyn std::error::Error>> {
        let cursor = cursor();
        cursor.goto(0, 0)?;

        let mut prev_color = None;

        for pixel in &self.frame_buffer {
            match prev_color {
                Some(c) if c == pixel.1 => {
                    print!("{}", pixel.0);
                }
                _ => {
                    prev_color = Some(pixel.1);
                    print!(
                        "{}{}{}",
                        Colored::Fg(Color::Rgb {
                            r: (pixel.1).0,
                            g: (pixel.1).1,
                            b: (pixel.1).2
                        }),
                        Colored::Bg(Color::Rgb {
                            r: 25,
                            g: 25,
                            b: 25
                        }),
                        pixel.0
                    )
                }
            }
        }

        println!("{}", Attribute::Reset);

        Ok(())
    }
    pub fn update(&mut self) -> Result<(), Box<dyn Error>> {
        let terminal = terminal();
        let terminal_size = terminal.terminal_size();

        //println!("terminal_size: {:?}", terminal_size);

        if (self.width != terminal_size.0 as usize) || (self.height != terminal_size.1 as usize) {
            // Check if the size changed
            let cursor = cursor();

            //re-hide the cursor
            cursor.hide()?;
            self.width = terminal_size.0 as usize + 1;
            self.height = terminal_size.1 as usize;
        }

        Ok(())
    }
}

fn draw_frame_and_pump_events(
    mmu: &mut Mmu,
    stdin: &mut AsyncReader,
    context: &mut Context,
) -> Result<bool, Box<dyn std::error::Error>> {
    let _ = context.update();
    context.clear(); // This clears the z and frame buffer

    let mut old_offscreen = vec![];
    for row in 0..240 {
        for col in 0..256 {
            let pixel = mmu.ppu.offscreen_buffer[row * 256 + col];

            let red = (pixel >> 16) as u8;
            let green = ((pixel >> 8) & 0xff) as u8;
            let blue = (pixel & 0xff) as u8;
            old_offscreen.push(red);
            old_offscreen.push(green);
            old_offscreen.push(blue);
        }
    }

    let mut new_offscreen = vec![0; context.height * context.width * 3];
    let mut resizer = resize::new(
        256,
        240,
        context.width,
        context.height,
        resize::Pixel::RGB24,
        resize::Type::Lanczos3,
    );
    resizer.resize(&old_offscreen, &mut new_offscreen);

    for row in 0..context.height {
        for col in 0..(context.width) {
            let red = new_offscreen[col * 3 + row * context.width * 3];
            let green = new_offscreen[col * 3 + 1 + row * context.width * 3];
            let blue = new_offscreen[col * 3 + 2 + row * context.width * 3];

            context.frame_buffer[col + row * context.width] = ('@', (red, green, blue));
        }
    }
    // for row in 0..(VISIBLE_HEIGHT as usize) {
    //     for col in 0..(VISIBLE_WIDTH as usize) {
    //     }
    // }

    //println!("context.size: {:?}", context.console_size);
    context.flush()?; // This prints all framebuffer info (good for changing colors ;)
    let mut buttons: Vec<JoyButton> = vec![];

    while let Some(b) = stdin.next() {
        match b {
            InputEvent::Keyboard(event) => match event {
                KeyEvent::Char('q') => return Ok(true),
                KeyEvent::Up => context.since_last_button[JoyButton::Up as usize] = 0,
                KeyEvent::Down => context.since_last_button[JoyButton::Down as usize] = 0,
                KeyEvent::Left => context.since_last_button[JoyButton::Left as usize] = 0,
                KeyEvent::Right => context.since_last_button[JoyButton::Right as usize] = 0,
                KeyEvent::Char('z') => context.since_last_button[JoyButton::A as usize] = 0,
                KeyEvent::Char('x') => context.since_last_button[JoyButton::B as usize] = 0,
                KeyEvent::Char('a') => context.since_last_button[JoyButton::Select as usize] = 0,
                KeyEvent::Char('s') => context.since_last_button[JoyButton::Start as usize] = 0,
                _ => {}
            },
            _ => {}
        }
    }
    for i in 0..8 {
        if context.since_last_button[i] < 15 {
            buttons.push(JoyButton::from_usize(i));
            context.since_last_button[i] += 1;
        }
    }

    mmu.joypad.update_keys(buttons);

    Ok(false)
}

pub fn run_cart(fname: &String) -> Result<(), Box<dyn std::error::Error>> {
    let mut context: Context = Context::blank(); // The context holds the frame+z buffer, and the width and height

    let crossterm = Crossterm::new();
    #[allow(unused)]
    let screen = RawScreen::into_raw_mode();
    let input = crossterm.input();
    let mut stdin = input.read_async();
    let cursor = cursor();

    cursor.hide()?;

    let mut mmu = Mmu::new();

    //Load the cart contents into the MMU and PPU
    load_cart(fname, &mut mmu)?;

    let mut cpu = Cpu::new();

    //Create all our memory handlers, and hand off ownership
    //of the cart to contained mmu

    cpu.reset(&mut mmu);

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
            let _ = context.update();
            let exiting = draw_frame_and_pump_events(&mut mmu, &mut stdin, &mut context)?;
            if exiting {
                break 'gameloop;
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

    //re-show the cursor
    cursor.show()?;

    Ok(())
}
