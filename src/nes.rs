use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::TextureAccess;

use crossterm::{cursor, terminal, AsyncReader, Attribute, Color, Colored};
use std::error::Error;
use std::f32;

use crossterm::{Crossterm, InputEvent, KeyEvent, RawScreen};

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
}

impl Context {
    pub fn blank() -> Context {
        //TODO: Make this a constant struct
        Context {
            width: 0,
            height: 0,
            frame_buffer: vec![],
            z_buffer: vec![],
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
    pub fn update(&mut self) -> Result<(), Box<Error>> {
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
) -> Result<bool, Box<std::error::Error>> {
    //context.update(size, &mesh_queue)?; // This checks for if there needs to be a context update
    context.update();
    context.clear(); // This clears the z and frame buffer

    let col_mult: f32 = 256.0 / context.width as f32;
    let row_mult: f32 = 240.0 / context.height as f32;

    for row in 0..context.height {
        for col in 0..(context.width) {
            let pixel = mmu.ppu.offscreen_buffer
                [((row_mult * row as f32) as usize * 256 + (col as f32 * col_mult) as usize)];
            //let offset = row * pitch * 2 + col * 3 * 2;

            let red = (pixel >> 16) as u8;
            let green = ((pixel >> 8) & 0xff) as u8;
            let blue = (pixel & 0xff) as u8;

            context.frame_buffer[col + row * context.width] = ('@', (red, green, blue));
        }
    }
    // for row in 0..(VISIBLE_HEIGHT as usize) {
    //     for col in 0..(VISIBLE_WIDTH as usize) {
    //     }
    // }

    //println!("context.size: {:?}", context.console_size);
    context.flush()?; // This prints all framebuffer info (good for changing colors ;)

    if let Some(b) = stdin.next() {
        match b {
            InputEvent::Keyboard(event) => match event {
                KeyEvent::Char('q') => return Ok(true),
                _ => {}
            },
            _ => {}
        }
    }

    Ok(false)
}
/*
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
*/

pub fn run_cart(fname: &String, use_debug: bool) -> Result<(), Box<std::error::Error>> {
    use std::cmp;

    /*
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
    */
    let mut context: Context = Context::blank(); // The context holds the frame+z buffer, and the width and height
    let size: (u16, u16) = (0, 0); // This is the terminal size, it's used to check when a new context must be made

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
    let mut frame_count = 0;

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
            /*
            let exiting =
                draw_frame_and_pump_events(&mut mmu, &mut canvas, &mut texture, &mut event_pump);
            if exiting {
                break 'gameloop;
            }
            */
            /*
            curr_timer_ticks = timer.ticks() as u64;
            if (curr_timer_ticks - prev_timer_ticks) < TIMER_TICKS_PER_FRAME {
                sleep(std::time::Duration::from_millis(
                    TIMER_TICKS_PER_FRAME - (curr_timer_ticks - prev_timer_ticks),
                ));
            }
            prev_timer_ticks = curr_timer_ticks;
            */
            frame_count += 1;
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
