use std::process;

use std::fmt; //for custom Debug

use crate::nes::{TICKS_PER_SCANLINE};
use crate::mmu::Mmu;

mod flag {
    pub const SIGN      : u8 = 0x80;
    pub const OVERFLOW  : u8 = 0x40;
    pub const BREAK     : u8 = 0x10;
    pub const DECIMAL   : u8 = 0x08;
    pub const INTERRUPT : u8 = 0x04;
    pub const ZERO      : u8 = 0x02;
    pub const CARRY     : u8 = 0x01;
}

pub struct Cpu {
    //registers
    a: u8,
    x: u8,
    y: u8,
    sp: u8,
    pub pc: u16,
    
    //flags
    carry: bool,
    zero: bool,
    pub interrupt: bool,
    decimal: bool,
    brk: bool,
    overflow: bool,
    sign: bool,
    
    //ticks and timers
    pub tick_count: u32,
    
    pub is_debugging: bool,
    
    //helper fields
    current_opcode: u8,
}

impl fmt::Debug for Cpu {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{6:04x}:{0}[{1:02x}] a:{2:02x} x:{3:02x} y:{4:02x} sp:{5:02x} flags:{7}{8}{9}{10}{11}{12} tick: {13}", 
            self.show_opcode(), self.current_opcode,
            self.a, self.x, self.y, self.sp, self.pc,
            if self.sign {'N'} else {'-'}, if self.zero { 'Z' } else {'-'}, if self.carry { 'C' } else {'-'},
            if self.interrupt {'I'} else {'-'}, if self.decimal {'D'} else {'-'}, if self.overflow {'V'} else {'-'},
            self.tick_count)
        /*
        write!(f, "{0:x} A: {1:x} X: {2:x} Y: {3:x} SP: {4:x} PC: {5:x} Flags:{6}{7}{8}{9}{10}{11} Ticks: {12}", 
            self.current_opcode,
            self.a, self.x, self.y, self.sp, self.pc, 
            if self.sign {'N'} else {'-'}, if self.zero { 'Z' } else {'-'}, if self.carry { 'C' } else {'-'}, 
            if self.interrupt {'I'} else {'-'}, if self.decimal {'D'} else {'-'}, if self.overflow {'V'} else {'-'},
            self.tick_count)
        */
    }
}

fn make_address(c: u8, d: u8) -> u16 {
    ((d as u16) << 8) + (c as u16)
}

impl Cpu {
    pub fn new() -> Cpu{
        Cpu {
            a: 0, 
            x: 0, 
            y: 0, 
            sp: 0xff,
            pc: 0xfffc,
            
            carry: false,
            zero: false,
            interrupt: false,
            decimal: false,
            brk: false,
            overflow: false,
            sign: false,
            
            tick_count: 0,
            
            is_debugging: false,
            
            current_opcode: 0,
        }
    }
    fn zero_page(&self, mmu: &mut Mmu,c: u8) -> u8 {
        mmu.read_u8(c as u16)
    }
    
    fn zero_page_x(&self, mmu: &mut Mmu,c: u8) -> u8 {
        let new_addr = 0xff & (c as u16 + self.x as u16);
        mmu.read_u8(new_addr)
    }
    
    fn zero_page_y(&self, mmu: &mut Mmu,c: u8) -> u8 {
        let new_addr = 0xff & (c as u16 + self.y as u16);
        mmu.read_u8(new_addr)
    }
    
    fn absolute(&self, mmu: &mut Mmu, c: u8, d: u8) -> u8 {
        mmu.read_u8(make_address(c, d))
    }
    
    fn absolute_x(&mut self, mmu: &mut Mmu,c: u8, d:u8, check_page: bool) -> u8 {
        if check_page {
            if (make_address(c, d) & 0xFF00) != 
                ((make_address(c, d) + self.x as u16) & 0xFF00) {
                
                self.tick_count += 1;
            }
        }
        
        mmu.read_u8(make_address(c, d) + self.x as u16)
    }
    
    fn absolute_y(&mut self, mmu: &mut Mmu,c: u8, d:u8, check_page: bool) -> u8 {
        if check_page {
            if (make_address(c, d) & 0xFF00) != 
                ((make_address(c, d) + self.y as u16) & 0xFF00) {
                
                self.tick_count += 1;
            }
        }
        
        mmu.read_u8(make_address(c, d) + self.y as u16)
    }
    
    fn indirect_x(&self, mmu: &mut Mmu,c: u8) -> u8 {
        let new_addr = mmu.read_u16(0xff & ((c as u16) + self.x as u16));        
        mmu.read_u8(new_addr)
    }
    
    fn indirect_y(&mut self, mmu: &mut Mmu,c: u8, check_page: bool) -> u8 {
        if check_page {
            if (mmu.read_u16(c as u16) & 0xFF00) !=
                ((mmu.read_u16(c as u16) + self.y as u16) & 0xFF00) {
                
                self.tick_count += 1;
            }
        }
        
        let addr = mmu.read_u16(c as u16) + self.y as u16;
        mmu.read_u8(addr)
    }
    
    fn zero_page_write(&mut self, mmu: &mut Mmu,c: u8, data: u8) {
        mmu.write_u8(c as u16, data);
    }
    
    fn zero_page_x_write(&mut self, mmu: &mut Mmu,c: u8, data: u8) {
        mmu.write_u8((c as u16 + self.x as u16) & 0xff, data);
    }

    fn zero_page_y_write(&mut self, mmu: &mut Mmu,c: u8, data: u8) {
        mmu.write_u8((c as u16 + self.y as u16) & 0xff, data);
    }
    
    fn absolute_write(&mut self, mmu: &mut Mmu,c: u8, d: u8, data: u8) {
        mmu.write_u8(make_address(c, d), data);
    }
    
    fn absolute_x_write(&mut self, mmu: &mut Mmu,c: u8, d: u8, data: u8) {
        mmu.write_u8(make_address(c, d) + self.x as u16, data);
    }
    
    fn absolute_y_write(&mut self, mmu: &mut Mmu,c: u8, d: u8, data: u8) {
        mmu.write_u8(make_address(c, d) + self.y as u16, data);
    }
    
    fn indirect_x_write(&mut self, mmu: &mut Mmu,c: u8, data: u8) {
        let new_addr = mmu.read_u16(0xff & (c as u16 + self.x as u16));
        mmu.write_u8(new_addr, data);
    }
    
    fn indirect_y_write(&mut self, mmu: &mut Mmu,c: u8, data: u8) {
        let new_addr = mmu.read_u16(c as u16) + self.y as u16;
        mmu.write_u8(new_addr, data);
    }
    
    fn push_u8(&mut self, mmu: &mut Mmu, data: u8) {
        mmu.write_u8(0x100 + self.sp as u16, data);
        if self.sp == 0 {
            self.sp = 0xff;
        }
        else {
            self.sp -= 1;
        }
    }
    
    pub fn push_u16(&mut self, mmu: &mut Mmu,data: u16) {
        self.push_u8(mmu, (data >> 8) as u8);
        self.push_u8(mmu, (data & 0xff) as u8);
    }
    
    pub fn push_status(&mut self, mmu: &mut Mmu) {
        let mut status = 0;
        if self.sign {
            status += flag::SIGN;
        }
        if self.overflow {
            status += flag::OVERFLOW;
        }
        if self.brk {
            status += flag::BREAK;
        }
        if self.decimal {
            status += flag::DECIMAL;
        }
        if self.interrupt {
            status += flag::INTERRUPT;
        }
        if self.zero {
            status += flag::ZERO;
        }
        if self.carry {
            status += flag::CARRY;
        }
        
        self.push_u8(mmu, status);        
    }
    
    fn pull_u8(&mut self, mmu: &mut Mmu) -> u8 {
        if self.sp == 0xff {
            self.sp = 0;
        }
        else {
            self.sp += 1;
        }
        
        mmu.read_u8(0x100 + self.sp as u16)
    }
    
    fn pull_u16(&mut self, mmu: &mut Mmu) -> u16 {
        let data_1 = self.pull_u8(mmu);
        let data_2 = self.pull_u8(mmu);
        
        make_address(data_1, data_2)
    }
    
    fn pull_status(&mut self, mmu: &mut Mmu) {
        let status = self.pull_u8(mmu);
        
        self.sign = (status & flag::SIGN) == flag::SIGN;
        self.overflow = (status & flag::OVERFLOW) == flag::OVERFLOW;
        self.brk = (status & flag::BREAK) == flag::BREAK;
        self.decimal = (status & flag::DECIMAL) == flag::DECIMAL;
        self.interrupt = (status & flag::INTERRUPT) == flag::INTERRUPT;
        self.zero = (status & flag::ZERO) == flag::ZERO;
        self.carry = (status & flag::CARRY) == flag::CARRY;
    }
    
    fn adc(&mut self, mmu: &mut Mmu) {
        let arg1 = mmu.read_u8(self.pc + 1);
        let arg2 = mmu.read_u8(self.pc + 2);
        
        let value = 
            match self.current_opcode {
                0x69 => arg1,
                0x65 => self.zero_page(mmu, arg1),
                0x75 => self.zero_page_x(mmu, arg1),
                0x6d => self.absolute(mmu, arg1, arg2),
                0x7d => self.absolute_x(mmu, arg1, arg2, true),
                0x79 => self.absolute_y(mmu, arg1, arg2, true),
                0x61 => self.indirect_x(mmu, arg1),
                0x71 => self.indirect_y(mmu, arg1, true),
                _ => {println!("Unknown opcode"); 0}
            };
        let total : u16 = self.a as u16 + value as u16 + 
            if self.carry {1} else {0};
        
        self.carry = total > 0xff;
        self.overflow = (total & 0x80) != (self.a as u16 & 0x80);
        self.zero = (total & 0xff) == 0;
        self.sign = (total & 0x80) == 0x80;        
        self.a = (total & 0xff) as u8;
        
        match self.current_opcode {
            0x69 => {self.tick_count += 2; self.pc += 2},
            0x65 => {self.tick_count += 3; self.pc += 2},
            0x75 => {self.tick_count += 4; self.pc += 2},
            0x6d => {self.tick_count += 4; self.pc += 3},
            0x7d => {self.tick_count += 4; self.pc += 3},
            0x79 => {self.tick_count += 4; self.pc += 3},
            0x61 => {self.tick_count += 6; self.pc += 2},
            0x71 => {self.tick_count += 5; self.pc += 2},
            _ => println!("unknown opcode in adc")
        }            
    }

    fn and(&mut self, mmu: &mut Mmu) {
        let arg1 = mmu.read_u8(self.pc + 1);
        let arg2 = mmu.read_u8(self.pc + 2);
        
        let value = 
            match self.current_opcode {
                0x29 => arg1,
                0x25 => self.zero_page(mmu, arg1),
                0x35 => self.zero_page_x(mmu, arg1),
                0x2d => self.absolute(mmu, arg1, arg2),
                0x3d => self.absolute_x(mmu, arg1, arg2, true),
                0x39 => self.absolute_y(mmu, arg1, arg2, true),
                0x21 => self.indirect_x(mmu, arg1),
                0x31 => self.indirect_y(mmu, arg1, true),
                _ => {println!("Unknown opcode"); 0}
            };
        
        self.a = self.a & value;
        self.zero = (self.a & 0xff) == 0;
        self.sign = (self.a & 0x80) == 0x80;        
        
        match self.current_opcode {
            0x29 => {self.tick_count += 2; self.pc += 2},
            0x25 => {self.tick_count += 3; self.pc += 2},
            0x35 => {self.tick_count += 4; self.pc += 2},
            0x2d => {self.tick_count += 4; self.pc += 3},
            0x3d => {self.tick_count += 4; self.pc += 3},
            0x39 => {self.tick_count += 4; self.pc += 3},
            0x21 => {self.tick_count += 6; self.pc += 2},
            0x31 => {self.tick_count += 5; self.pc += 2},
            _ => println!("unknown opcode in and")
        }            
    }

    fn asl(&mut self, mmu: &mut Mmu) {
        let arg1 = mmu.read_u8(self.pc + 1);
        let arg2 = mmu.read_u8(self.pc + 2);
        
        let mut value : u8 = 
            match self.current_opcode {
                0x0a => self.a,
                0x06 => self.zero_page(mmu, arg1),
                0x16 => self.zero_page_x(mmu, arg1),
                0x0e => self.absolute(mmu, arg1, arg2),
                0x1e => self.absolute_x(mmu, arg1, arg2, false),
                _ => {println!("Unknown opcode"); 0}
            };
        
        self.carry = (value & 0x80) == 0x80;
        value = (0xff & ((value as u16) << 1)) as u8;
        self.zero = value == 0;
        self.sign = (value & 0x80) == 0x80;        
        
        match self.current_opcode {
            0x0a => {self.a = value; 
                self.tick_count += 2; self.pc += 1},
            0x06 => {self.zero_page_write(mmu, arg1, value); 
                self.tick_count += 5; self.pc += 2},
            0x16 => {self.zero_page_x_write(mmu, arg1, value); 
                self.tick_count += 6; self.pc += 2},
            0x0e => {self.absolute_write(mmu, arg1, arg2, value);
                self.tick_count += 6; self.pc += 3},
            0x1e => {self.absolute_x_write(mmu, arg1, arg2, value);
                self.tick_count += 7; self.pc += 3},
            _ => println!("unknown opcode in asl")
        }            
    }
    
    fn bcc(&mut self, mmu: &mut Mmu) {
        let arg1 : i8 = mmu.read_u8(self.pc + 1) as i8;
        
        self.pc += 2;
        
        if !self.carry {
            if (self.pc & 0xff00) != ((self.pc as i16 + 2i16 + arg1 as i16) as u16 & 0xff00) {
                self.tick_count += 1;
            }
            self.pc = (0xffff & (self.pc as i32 + arg1 as i32)) as u16;
            self.tick_count += 1;
        }
        
        self.tick_count += 2;
    }

    fn bcs(&mut self, mmu: &mut Mmu) {
        let arg1 : i8 = mmu.read_u8(self.pc + 1) as i8;
        
        self.pc += 2;
        
        if self.carry {
            if (self.pc & 0xff00) != ((self.pc as i16 + 2i16 + arg1 as i16) as u16 & 0xff00) {
                self.tick_count += 1;
            }
            self.pc = (0xffff & (self.pc as i32 + arg1 as i32)) as u16;
            self.tick_count += 1;
        }
        
        self.tick_count += 2;
    }

    fn beq(&mut self, mmu: &mut Mmu) {
        let arg1 : i8 = mmu.read_u8(self.pc + 1) as i8;
        
        self.pc += 2;
        
        if self.zero {
            if (self.pc & 0xff00) != ((self.pc as i16 + 2i16 + arg1 as i16) as u16 & 0xff00) {
                self.tick_count += 1;
            }
            self.pc = (0xffff & (self.pc as i32 + arg1 as i32)) as u16;
            self.tick_count += 1;
        }
        
        self.tick_count += 2;
    }

    fn bit(&mut self, mmu: &mut Mmu) {
        let arg1 = mmu.read_u8(self.pc + 1);
        let arg2 = mmu.read_u8(self.pc + 2);
        
        let value = 
            match self.current_opcode {
                0x24 => self.zero_page(mmu, arg1),
                0x2c => self.absolute(mmu, arg1, arg2),
                _ => {println!("Unknown opcode"); 0}
            };
        
        self.zero = (self.a & value) == 0;
        self.sign = (value & 0x80) == 0x80;        
        self.overflow = (value & 0x40) == 0x40;
        
        match self.current_opcode {
            0x24 => {self.tick_count += 3; self.pc += 2},
            0x2c => {self.tick_count += 4; self.pc += 3},
            _ => println!("unknown opcode in bit")
        }
    }
    
    fn bmi(&mut self, mmu: &mut Mmu) {
        let arg1 : i8 = mmu.read_u8(self.pc + 1) as i8;
        
        self.pc += 2;
        
        if self.sign {
            if (self.pc & 0xff00) != ((self.pc as i16 + 2i16 + arg1 as i16) as u16 & 0xff00) {
                self.tick_count += 1;
            }
            self.pc = (0xffff & (self.pc as i32 + arg1 as i32)) as u16;
            self.tick_count += 1;
        }
        
        self.tick_count += 2;
    }

    fn bne(&mut self, mmu: &mut Mmu) {
        let arg1 : i8 = mmu.read_u8(self.pc + 1) as i8;
        
        self.pc += 2;
        
        if !self.zero {
            if (self.pc & 0xff00) != ((self.pc as i16 + 2i16 + arg1 as i16) as u16 & 0xff00) {
                self.tick_count += 1;
            }
            self.pc = (0xffff & (self.pc as i32 + arg1 as i32)) as u16;
            self.tick_count += 1;
        }
        
        self.tick_count += 2;
    }

    fn bpl(&mut self, mmu: &mut Mmu) {
        let arg1 : i8 = mmu.read_u8(self.pc + 1) as i8;
        
        self.pc += 2;
        
        if !self.sign {
            if (self.pc & 0xff00) != ((self.pc as i16 + 2i16 + arg1 as i16) as u16 & 0xff00) {
                self.tick_count += 1;
            }
            self.pc = (0xffff & (self.pc as i32 + arg1 as i32)) as u16;
            self.tick_count += 1;
        }
        
        self.tick_count += 2;
    }
    
    fn brk(&mut self, mmu: &mut Mmu) {
        self.pc = 0xff & (self.pc as u16 + 2);
        let tmp_pc = self.pc;
        self.push_u16(mmu, tmp_pc);
        self.brk = true;
        self.push_status(mmu);
        self.interrupt = true;
        self.pc = mmu.read_u16(0xfffe);
        self.tick_count += 7;
    }
    
    fn bvc(&mut self, mmu: &mut Mmu) {
        let arg1 : i8 = mmu.read_u8(self.pc + 1) as i8;
        
        self.pc += 2;
        
        if !self.overflow {
            if (self.pc & 0xff00) != ((self.pc as i16 + 2i16 + arg1 as i16) as u16 & 0xff00) {
                self.tick_count += 1;
            }
            self.pc = (0xffff & (self.pc as i32 + arg1 as i32)) as u16;
            self.tick_count += 1;
        }
        
        self.tick_count += 2;
    }

    fn bvs(&mut self, mmu: &mut Mmu) {
        let arg1 : i8 = mmu.read_u8(self.pc + 1) as i8;
        
        self.pc += 2;
        
        if self.overflow {
            if (self.pc & 0xff00) != ((self.pc as i16 + 2i16 + arg1 as i16) as u16 & 0xff00) {
                self.tick_count += 1;
            }
            self.pc = (0xffff & (self.pc as i32 + arg1 as i32)) as u16;
            self.tick_count += 1;
        }
        
        self.tick_count += 2;
    }
    
    fn clc(&mut self) {
        self.carry = false;
        self.pc += 1;
        self.tick_count += 2;
    }
    
    fn cld(&mut self) {
        self.decimal = false;
        self.pc += 1;
        self.tick_count += 2;
    }
    
    fn cli(&mut self) {
        self.interrupt = false;
        self.pc += 1;
        self.tick_count += 2;
    }
    
    fn clv(&mut self) {
        self.overflow = false;
        self.pc += 1;
        self.tick_count += 2;
    }

    fn cmp(&mut self, mmu: &mut Mmu) {
        let arg1 = mmu.read_u8(self.pc + 1);
        let arg2 = mmu.read_u8(self.pc + 2);
        
        let mut value = 
            match self.current_opcode {
                0xc9 => arg1,
                0xc5 => self.zero_page(mmu, arg1),
                0xd5 => self.zero_page_x(mmu, arg1),
                0xcd => self.absolute(mmu, arg1, arg2),
                0xdd => self.absolute_x(mmu, arg1, arg2, true),
                0xd9 => self.absolute_y(mmu, arg1, arg2, true),
                0xc1 => self.indirect_x(mmu, arg1),
                0xd1 => self.indirect_y(mmu, arg1, true),
                _ => {println!("Unknown opcode"); 0}
            };
            
        self.carry = self.a >= value;
        value = (0xff & ((self.a as i16) - value as i16)) as u8;
        self.zero = value == 0;
        self.sign = (value & 0x80) == 0x80;
        
        match self.current_opcode {
            0xc9 => {self.tick_count += 2; self.pc += 2},
            0xc5 => {self.tick_count += 3; self.pc += 2},
            0xd5 => {self.tick_count += 4; self.pc += 2},
            0xcd => {self.tick_count += 4; self.pc += 3},
            0xdd => {self.tick_count += 4; self.pc += 3},
            0xd9 => {self.tick_count += 4; self.pc += 3},
            0xc1 => {self.tick_count += 6; self.pc += 2},
            0xd1 => {self.tick_count += 5; self.pc += 2},
            _ => println!("unknown opcode in cmp")
        }            
    }

    fn cpx(&mut self, mmu: &mut Mmu) {
        let arg1 = mmu.read_u8(self.pc + 1);
        let arg2 = mmu.read_u8(self.pc + 2);
        
        let mut value = 
            match self.current_opcode {
                0xe0 => arg1,
                0xe4 => self.zero_page(mmu, arg1),
                0xec => self.absolute(mmu, arg1, arg2),
                _ => {println!("Unknown opcode"); 0}
            };
            
        self.carry = self.x >= value;
        value = (0xff & ((self.x as i16) - value as i16)) as u8;
        self.zero = value == 0;
        self.sign = (value & 0x80) == 0x80;
        
        match self.current_opcode {
            0xe0 => {self.tick_count += 2; self.pc += 2},
            0xe4 => {self.tick_count += 3; self.pc += 2},
            0xec => {self.tick_count += 4; self.pc += 3},
            _ => println!("unknown opcode in cpx")
        }            
    }

    fn cpy(&mut self, mmu: &mut Mmu) {
        let arg1 = mmu.read_u8(self.pc + 1);
        let arg2 = mmu.read_u8(self.pc + 2);
        
        let mut value = 
            match self.current_opcode {
                0xc0 => arg1,
                0xc4 => self.zero_page(mmu, arg1),
                0xcc => self.absolute(mmu, arg1, arg2),
                _ => {println!("Unknown opcode"); 0}
            };
            
        self.carry = self.y >= value;
        value = (0xff & ((self.y as i16) - value as i16)) as u8;
        self.zero = value == 0;
        self.sign = (value & 0x80) == 0x80;
        
        match self.current_opcode {
            0xc0 => {self.tick_count += 2; self.pc += 2},
            0xc4 => {self.tick_count += 3; self.pc += 2},
            0xcc => {self.tick_count += 4; self.pc += 3},
            _ => println!("unknown opcode in cpy")
        }            
    }    
    
    fn dec(&mut self, mmu: &mut Mmu) {
        let arg1 = mmu.read_u8(self.pc + 1);
        let arg2 = mmu.read_u8(self.pc + 2);
        
        let mut value : u8 = 
            match self.current_opcode {
                0xc6 => self.zero_page(mmu, arg1),
                0xd6 => self.zero_page_x(mmu, arg1),
                0xce => self.absolute(mmu, arg1, arg2),
                0xde => self.absolute_x(mmu, arg1, arg2, false),
                _ => {println!("Unknown opcode"); 0}
            };
        
        if value == 0 {
            value = 0xff;
        }
        else {
            value -= 1;
        }
        
        self.zero = value == 0;
        self.sign = (value & 0x80) == 0x80;        
        
        match self.current_opcode {
            0xc6 => {self.zero_page_write(mmu, arg1, value); 
                self.tick_count += 5; self.pc += 2},
            0xd6 => {self.zero_page_x_write(mmu, arg1, value); 
                self.tick_count += 6; self.pc += 2},
            0xce => {self.absolute_write(mmu, arg1, arg2, value);
                self.tick_count += 6; self.pc += 3},
            0xde => {self.absolute_x_write(mmu, arg1, arg2, value);
                self.tick_count += 7; self.pc += 3},
            _ => println!("unknown opcode in dec")
        }            
    }
    
    fn dex(&mut self) {
        if self.x == 0 {
            self.x = 0xff;
        }
        else {
            self.x -= 1;
        }
        
        self.zero = self.x == 0;
        self.sign = (self.x & 0x80) == 0x80;
        
        self.pc += 1;
        self.tick_count += 2;
    }

    fn dey(&mut self) {
        if self.y == 0 {
            self.y = 0xff;
        }
        else {
            self.y -= 1;
        }
        
        self.zero = self.y == 0;
        self.sign = (self.y & 0x80) == 0x80;
        
        self.pc += 1;
        self.tick_count += 2;
    }

    fn eor(&mut self, mmu: &mut Mmu) {
        let arg1 = mmu.read_u8(self.pc + 1);
        let arg2 = mmu.read_u8(self.pc + 2);
        
        let value = 
            match self.current_opcode {
                0x49 => arg1,
                0x45 => self.zero_page(mmu, arg1),
                0x55 => self.zero_page_x(mmu, arg1),
                0x4d => self.absolute(mmu, arg1, arg2),
                0x5d => self.absolute_x(mmu, arg1, arg2, true),
                0x59 => self.absolute_y(mmu, arg1, arg2, true),
                0x41 => self.indirect_x(mmu, arg1),
                0x51 => self.indirect_y(mmu, arg1, true),
                _ => {println!("Unknown opcode"); 0}
            };
 
        self.a = self.a ^ value;           
        self.zero = self.a == 0;
        self.sign = (self.a & 0x80) == 0x80;
        
        //FIXME: I think 4d tick is 4, but need to confirm
        match self.current_opcode {
            0x49 => {self.tick_count += 2; self.pc += 2},
            0x45 => {self.tick_count += 3; self.pc += 2},
            0x55 => {self.tick_count += 4; self.pc += 2},
            0x4d => {self.tick_count += 3; self.pc += 3},
            0x5d => {self.tick_count += 4; self.pc += 3},
            0x59 => {self.tick_count += 4; self.pc += 3},
            0x41 => {self.tick_count += 6; self.pc += 2},
            0x51 => {self.tick_count += 5; self.pc += 2},
            _ => println!("unknown opcode in cmp")
        }            
    }
    
    fn inc(&mut self, mmu: &mut Mmu) {
        let arg1 = mmu.read_u8(self.pc + 1);
        let arg2 = mmu.read_u8(self.pc + 2);
        
        let mut value : u8 = 
            match self.current_opcode {
                0xe6 => self.zero_page(mmu, arg1),
                0xf6 => self.zero_page_x(mmu, arg1),
                0xee => self.absolute(mmu, arg1, arg2),
                0xfe => self.absolute_x(mmu, arg1, arg2, false),
                _ => {println!("Unknown opcode"); 0}
            };
        
        if value == 0xff {
            value = 0;
        }
        else {
            value += 1;
        }
        
        self.zero = value == 0;
        self.sign = (value & 0x80) == 0x80;        
        
        match self.current_opcode {
            0xe6 => {self.zero_page_write(mmu, arg1, value); 
                self.tick_count += 5; self.pc += 2},
            0xf6 => {self.zero_page_x_write(mmu, arg1, value); 
                self.tick_count += 6; self.pc += 2},
            0xee => {self.absolute_write(mmu, arg1, arg2, value);
                self.tick_count += 6; self.pc += 3},
            0xfe => {self.absolute_x_write(mmu, arg1, arg2, value);
                self.tick_count += 7; self.pc += 3},
            _ => println!("unknown opcode in inc")
        }            
    }    
    
    fn inx(&mut self) {
        if self.x == 0xff {
            self.x = 0;
        }
        else {
            self.x += 1;
        }
        
        self.zero = self.x == 0;
        self.sign = (self.x & 0x80) == 0x80;
        
        self.pc += 1;
        self.tick_count += 2;
    }

    fn iny(&mut self) {
        if self.y == 0xff {
            self.y = 0;
        }
        else {
            self.y += 1;
        }
        
        self.zero = self.y == 0;
        self.sign = (self.y & 0x80) == 0x80;
        
        self.pc += 1;
        self.tick_count += 2;
    }

    fn jmp(&mut self, mmu: &mut Mmu) {
        let addr = mmu.read_u16(self.pc + 1);
        
        match self.current_opcode {
            0x4c => {self.pc = addr; self.tick_count += 3},
            0x6c => {self.pc = mmu.read_u16(addr); self.tick_count += 5},
            _ => println!("Unknown opcode in jmp")
        }
    }
    
    fn jsr(&mut self, mmu: &mut Mmu) {
        let pc = self.pc;
        let arg1 = mmu.read_u8(pc + 1);
        let arg2 = mmu.read_u8(pc + 2);
        self.push_u16(mmu, pc + 2);
        self.pc = make_address(arg1, arg2);
        self.tick_count += 6;
    }
    
    fn lda(&mut self, mmu: &mut Mmu) {
        let arg1 = mmu.read_u8(self.pc + 1);
        
        match self.current_opcode {
            0xa9 => {self.a = arg1; 
                self.tick_count += 2; self.pc += 2},
            0xa5 => {self.a = self.zero_page(mmu, arg1); 
                self.tick_count += 3; self.pc += 2},
            0xb5 => {self.a = self.zero_page_x(mmu, arg1); 
                self.tick_count += 4; self.pc += 2},
            0xad => {let arg2 = mmu.read_u8(self.pc + 2); 
                self.a = self.absolute(mmu, arg1, arg2);
                self.tick_count += 4; self.pc += 3},
            0xbd => {let arg2 = mmu.read_u8(self.pc + 2);
                self.a = self.absolute_x(mmu, arg1, arg2, true); 
                self.tick_count += 4; self.pc += 3},
            0xb9 => {let arg2 = mmu.read_u8(self.pc + 2);
                self.a = self.absolute_y(mmu, arg1, arg2, true);
                self.tick_count += 4; self.pc += 3},
            0xa1 => {self.a = self.indirect_x(mmu, arg1); 
                self.tick_count += 6; self.pc += 2},
            0xb1 => {self.a = self.indirect_y(mmu, arg1, true); 
                self.tick_count += 5; self.pc += 2},
            _ => println!("Unknown opcode in lda")
        }
        
        self.zero = self.a == 0;
        self.sign = (self.a & 0x80) == 0x80;
    }
    
    fn ldx(&mut self, mmu: &mut Mmu) {
        let arg1 = mmu.read_u8(self.pc + 1);
        let arg2 = mmu.read_u8(self.pc + 2);
        
        match self.current_opcode {
            0xa2 => {self.x = arg1; 
                self.tick_count += 2; self.pc += 2},
            0xa6 => {self.x = self.zero_page(mmu, arg1); 
                self.tick_count += 3; self.pc += 2},
            0xb6 => {self.x = self.zero_page_y(mmu, arg1); 
                self.tick_count += 4; self.pc += 2},
            0xae => {self.x = self.absolute(mmu, arg1, arg2);
                self.tick_count += 4; self.pc += 3},
            0xbe => {self.x = self.absolute_y(mmu, arg1, arg2, true);
                self.tick_count += 4; self.pc += 3},
            _ => println!("Unknown opcode in ldx")
        }
        
        self.zero = self.x == 0;
        self.sign = (self.x & 0x80) == 0x80;
    }
    
    fn ldy(&mut self, mmu: &mut Mmu) {
        let arg1 = mmu.read_u8(self.pc + 1);
        let arg2 = mmu.read_u8(self.pc + 2);
        
        match self.current_opcode {
            0xa0 => {self.y = arg1; 
                self.tick_count += 2; self.pc += 2},
            0xa4 => {self.y = self.zero_page(mmu, arg1); 
                self.tick_count += 3; self.pc += 2},
            0xb4 => {self.y = self.zero_page_x(mmu, arg1); 
                self.tick_count += 4; self.pc += 2},
            0xac => {self.y = self.absolute(mmu, arg1, arg2);
                self.tick_count += 4; self.pc += 3},
            0xbc => {self.y = self.absolute_x(mmu, arg1, arg2, true);
                self.tick_count += 4; self.pc += 3},
            _ => println!("Unknown opcode in ldx")
        }
        
        self.zero = self.y == 0;
        self.sign = (self.y & 0x80) == 0x80;
    }

    fn lsr(&mut self, mmu: &mut Mmu) {
        let arg1 = mmu.read_u8(self.pc + 1);
        let arg2 = mmu.read_u8(self.pc + 2);
        
        let mut value : u8 = 
            match self.current_opcode {
                0x4a => self.a,
                0x46 => self.zero_page(mmu, arg1),
                0x56 => self.zero_page_x(mmu, arg1),
                0x4e => self.absolute(mmu, arg1, arg2),
                0x5e => self.absolute_x(mmu, arg1, arg2, true),
                _ => {println!("Unknown opcode"); 0}
            };
        
        self.carry = (value & 0x1) == 0x1;
        value = value >> 1;
        self.zero = value == 0;
        self.sign = (value & 0x80) == 0x80;        
        
        match self.current_opcode {
            0x4a => {self.a = value; 
                self.tick_count += 2; self.pc += 1},
            0x46 => {self.zero_page_write(mmu, arg1, value); 
                self.tick_count += 5; self.pc += 2},
            0x56 => {self.zero_page_x_write(mmu, arg1, value); 
                self.tick_count += 6; self.pc += 2},
            0x4e => {self.absolute_write(mmu, arg1, arg2, value);
                self.tick_count += 6; self.pc += 3},
            0x5e => {self.absolute_x_write(mmu, arg1, arg2, value);
                self.tick_count += 7; self.pc += 3},
            _ => println!("unknown opcode in lsr")
        }
    }
    
    fn nop(&mut self) {
        self.pc += 1;
        self.tick_count += 1;
    }

    fn ora(&mut self, mmu: &mut Mmu) {
        let arg1 = mmu.read_u8(self.pc + 1);
        let arg2 = mmu.read_u8(self.pc + 2);
        
        let value = 
            match self.current_opcode {
                0x09 => arg1,
                0x05 => self.zero_page(mmu, arg1),
                0x15 => self.zero_page_x(mmu, arg1),
                0x0d => self.absolute(mmu, arg1, arg2),
                0x1d => self.absolute_x(mmu, arg1, arg2, true),
                0x19 => self.absolute_y(mmu, arg1, arg2, true),
                0x01 => self.indirect_x(mmu, arg1),
                0x11 => self.indirect_y(mmu, arg1, false),
                _ => {println!("Unknown opcode"); 0}
            };
        
        self.a = self.a | value;
        self.zero = (self.a & 0xff) == 0;
        self.sign = (self.a & 0x80) == 0x80;        
        
        match self.current_opcode {
            0x09 => {self.tick_count += 2; self.pc += 2},
            0x05 => {self.tick_count += 3; self.pc += 2},
            0x15 => {self.tick_count += 4; self.pc += 2},
            0x0d => {self.tick_count += 4; self.pc += 3},
            0x1d => {self.tick_count += 4; self.pc += 3},
            0x19 => {self.tick_count += 4; self.pc += 3},
            0x01 => {self.tick_count += 6; self.pc += 2},
            0x11 => {self.tick_count += 5; self.pc += 2},
            _ => println!("unknown opcode in and")
        }
    }
    
    fn pha(&mut self, mmu: &mut Mmu) {
        let a = self.a;
        self.push_u8(mmu, a);
        self.pc += 1;
        self.tick_count += 3;
    }
    
    fn php(&mut self, mmu: &mut Mmu) {
        self.push_status(mmu);
        self.pc += 1;
        self.tick_count += 3;
    }
    
    fn pla(&mut self, mmu: &mut Mmu) {
        self.a = self.pull_u8(mmu);
        self.zero = self.a == 0;
        self.sign = (self.a & 0x80) == 0x80;
        self.pc += 1;
        self.tick_count += 4;
    }
    
    fn plp(&mut self, mmu: &mut Mmu) {
        self.pull_status(mmu);
        self.pc += 1;
        self.tick_count += 4;
    }

    fn rol(&mut self, mmu: &mut Mmu) {
        let arg1 = mmu.read_u8(self.pc + 1);
        let arg2 = mmu.read_u8(self.pc + 2);
        
        let mut value = 
            match self.current_opcode {
                0x2a => self.a,
                0x26 => self.zero_page(mmu, arg1),
                0x36 => self.zero_page_x(mmu, arg1),
                0x2e => self.absolute(mmu, arg1, arg2),
                0x3e => self.absolute_x(mmu, arg1, arg2, false),
                _ => {println!("Unknown opcode"); 0}
            };
        
        let bit = (value & 0x80) == 0x80;
        value = (value & 0x7f) << 1;
        value += if self.carry {1} else {0};
        self.carry = bit;
        self.zero = value == 0;
        self.sign = (value & 0x80) == 0x80;        
        
        match self.current_opcode {
            0x2a => {self.a = value; 
                self.tick_count += 2; self.pc += 1},
            0x26 => {self.zero_page_write(mmu, arg1, value); 
                self.tick_count += 5; self.pc += 2},
            0x36 => {self.zero_page_x_write(mmu, arg1, value); 
                self.tick_count += 6; self.pc += 2},
            0x2e => {self.absolute_write(mmu, arg1, arg2, value);
                self.tick_count += 6; self.pc += 3},
            0x3e => {self.absolute_x_write(mmu, arg1, arg2, value);
                self.tick_count += 7; self.pc += 3},
            _ => println!("unknown opcode in rol")
        }
    }

    fn ror(&mut self, mmu: &mut Mmu) {
        let arg1 = mmu.read_u8(self.pc + 1);
        let arg2 = mmu.read_u8(self.pc + 2);
        
        let mut value : u8 = 
            match self.current_opcode {
                0x6a => self.a,
                0x66 => self.zero_page(mmu, arg1),
                0x76 => self.zero_page_x(mmu, arg1),
                0x6e => self.absolute(mmu, arg1, arg2),
                0x7e => self.absolute_x(mmu, arg1, arg2, true),
                _ => {println!("Unknown opcode"); 0}
            };
        
        let bit = (value & 0x1) == 0x1;
        value = (value >> 1) & 0x7f;
        value += if self.carry {0x80} else {0};
        self.carry = bit;
        self.zero = value == 0;
        self.sign = (value & 0x80) == 0x80;        
        
        match self.current_opcode {
            0x6a => {self.a = value; 
                self.tick_count += 2; self.pc += 1},
            0x66 => {self.zero_page_write(mmu, arg1, value); 
                self.tick_count += 5; self.pc += 2},
            0x76 => {self.zero_page_x_write(mmu, arg1, value); 
                self.tick_count += 6; self.pc += 2},
            0x6e => {self.absolute_write(mmu, arg1, arg2, value);
                self.tick_count += 6; self.pc += 3},
            0x7e => {self.absolute_x_write(mmu, arg1, arg2, value);
                self.tick_count += 7; self.pc += 3},
            _ => println!("unknown opcode in ror")
        }
    }
    
    fn rti(&mut self, mmu: &mut Mmu) {
        self.pull_status(mmu);
        self.pc = self.pull_u16(mmu);
        self.tick_count += 6;
    }
    
    fn rts(&mut self, mmu: &mut Mmu) {
        self.pc = self.pull_u16(mmu) + 1;
        self.tick_count += 6;
    }
    
    fn sbc(&mut self, mmu: &mut Mmu) {
        let arg1 = mmu.read_u8(self.pc + 1);
        let arg2 = mmu.read_u8(self.pc + 2);
        
        let value = 
            match self.current_opcode {
                0xe9 => arg1,
                0xe5 => self.zero_page(mmu, arg1),
                0xf5 => self.zero_page_x(mmu, arg1),
                0xed => self.absolute(mmu, arg1, arg2),
                0xfd => self.absolute_x(mmu, arg1, arg2, true),
                0xf9 => self.absolute_y(mmu, arg1, arg2, true),
                0xe1 => self.indirect_x(mmu, arg1),
                0xf1 => self.indirect_y(mmu, arg1, false),
                _ => {println!("Unknown opcode"); 0}
            };
        let total : i16 = self.a as i16 - value as i16 - 
            if self.carry {0} else {1};
        
        self.carry = total >= 0;
        self.overflow = (total & 0x80) != (self.a as i16 & 0x80);
        self.zero = (total & 0xff) == 0;
        self.sign = (total & 0x80) == 0x80;        
        self.a = (total & 0xff) as u8;
        
        match self.current_opcode {
            0xe9 => {self.tick_count += 2; self.pc += 2},
            0xe5 => {self.tick_count += 3; self.pc += 2},
            0xf5 => {self.tick_count += 4; self.pc += 2},
            0xed => {self.tick_count += 4; self.pc += 3},
            0xfd => {self.tick_count += 4; self.pc += 3},
            0xf9 => {self.tick_count += 4; self.pc += 3},
            0xe1 => {self.tick_count += 6; self.pc += 2},
            0xf1 => {self.tick_count += 5; self.pc += 2},
            _ => println!("unknown opcode in sbc")
        }
    }
    
    fn sec(&mut self) {
        self.carry = true;
        self.tick_count += 2;
        self.pc += 1;
    }
    
    fn sed(&mut self) {
        self.decimal = true;
        self.tick_count += 2;
        self.pc += 1;
    }
    
    fn sei(&mut self) {
        self.interrupt = true;
        self.tick_count += 2;
        self.pc += 1;
    }

    fn sta(&mut self, mmu: &mut Mmu) {
        let arg1 = mmu.read_u8(self.pc + 1);
        let arg2 = mmu.read_u8(self.pc + 2);
        
        let a = self.a;
        match self.current_opcode {
            0x85 => {self.zero_page_write(mmu, arg1, a); 
                self.tick_count += 3; self.pc += 2},
            0x95 => {self.zero_page_x_write(mmu, arg1, a);
                self.tick_count += 4; self.pc += 2},
            0x8d => {self.absolute_write(mmu, arg1, arg2, a); 
                self.tick_count += 4; self.pc += 3},
            0x9d => {self.absolute_x_write(mmu, arg1, arg2, a);
                self.tick_count += 5; self.pc += 3},
            0x99 => {self.absolute_y_write(mmu, arg1, arg2, a);
                self.tick_count += 5; self.pc += 3},
            0x81 => {self.indirect_x_write(mmu, arg1, a);
                self.tick_count += 6; self.pc += 2},
            0x91 => {self.indirect_y_write(mmu, arg1, a);
                self.tick_count += 6; self.pc += 2},
            _ => println!("Unknown opcode in sta")
        }
    }
    
    fn stx(&mut self, mmu: &mut Mmu) {
        let arg1 = mmu.read_u8(self.pc + 1);
        let arg2 = mmu.read_u8(self.pc + 2);
        
        let x = self.x;
        match self.current_opcode {
            0x86 => {self.zero_page_write(mmu, arg1, x); 
                self.tick_count += 3; self.pc += 2},
            0x96 => {self.zero_page_y_write(mmu, arg1, x);
                self.tick_count += 4; self.pc += 2},
            0x8e => {self.absolute_write(mmu, arg1, arg2, x); 
                self.tick_count += 4; self.pc += 3},
            _ => println!("Unknown opcode in stx")
        }
    }
    
    fn sty(&mut self, mmu: &mut Mmu) {
        let arg1 = mmu.read_u8(self.pc + 1);
        let arg2 = mmu.read_u8(self.pc + 2);
        
        let y = self.y;
        match self.current_opcode {
            0x84 => {self.zero_page_write(mmu, arg1, y); 
                self.tick_count += 3; self.pc += 2},
            0x94 => {self.zero_page_x_write(mmu, arg1, y);
                self.tick_count += 4; self.pc += 2},
            0x8c => {self.absolute_write(mmu, arg1, arg2, y); 
                self.tick_count += 4; self.pc += 3},
            _ => println!("Unknown opcode in sty")
        }
    }
    
    fn tax(&mut self) {
        self.x = self.a;
        self.zero = self.x == 0;
        self.sign = (self.x & 0x80) == 0x80;
        self.pc += 1;
        self.tick_count += 2;
    }
    
    fn tay(&mut self) {
        self.y = self.a;
        self.zero = self.y == 0;
        self.sign = (self.y & 0x80) == 0x80;
        self.pc += 1;
        self.tick_count += 2;
    }
    
    fn tsx(&mut self) {
        self.x = self.sp;
        self.zero = self.x == 0;
        self.sign = (self.x & 0x80) == 0x80;
        self.pc += 1;
        self.tick_count += 2;
    }
    
    fn txa(&mut self) {
        self.a = self.x;
        self.zero = self.a == 0;
        self.sign = (self.a & 0x80) == 0x80;
        self.pc += 1;
        self.tick_count += 2;
    }
    
    fn txs(&mut self) {
        self.sp = self.x;
        
        self.pc += 1;
        self.tick_count += 2;
    }
    
    fn tya(&mut self) {
        self.a = self.y;
        self.zero = self.a == 0;
        self.sign = (self.a & 0x80) == 0x80;
        self.pc += 1;
        self.tick_count += 2;
    }
    
    pub fn reset(&mut self, mmu: &mut Mmu) {
        //reset pc using reset vector
        //println!("Reset vector: {0:x}", mmu.read_u16(&mut mem.ppu, 0xfffc));
        self.pc = mmu.read_u16(0xfffc);
    }
    
    fn show_opcode(&self) -> &str {
        match self.current_opcode {
            0x00 => "brk",
            0x01 => "ora", 
            0x05 => "ora",
            0x06 => "asl",
            0x08 => "php",
            0x09 => "ora",
            0x0a => "asl", 
            0x0d => "ora", 
            0x0e => "asl",
            0x10 => "bpl", 
            0x11 => "ora", 
            0x15 => "ora", 
            0x16 => "asl", 
            0x18 => "clc", 
            0x19 => "ora", 
            0x1d => "ora", 
            0x1e => "asl", 
            0x20 => "jsr",
            0x21 => "and", 
            0x24 => "bit", 
            0x25 => "and", 
            0x26 => "rol", 
            0x28 => "plp", 
            0x29 => "and",
            0x2a => "rol", 
            0x2c => "bit", 
            0x2d => "and", 
            0x2e => "rol", 
            0x30 => "bmi", 
            0x31 => "and", 
            0x32 => "nop",
            0x33 => "nop", 
            0x34 => "nop", 
            0x35 => "and", 
            0x36 => "rol", 
            0x38 => "sec", 
            0x39 => "and", 
            0x3d => "and", 
            0x3e => "rol", 
            0x40 => "rti", 
            0x41 => "eor", 
            0x45 => "eor", 
            0x46 => "lsr", 
            0x48 => "pha", 
            0x49 => "eor", 
            0x4a => "lsr", 
            0x4c => "jmp", 
            0x4d => "eor",
            0x4e => "lsr", 
            0x50 => "bvc", 
            0x51 => "eor", 
            0x55 => "eor", 
            0x56 => "lsr",
            0x58 => "cli", 
            0x59 => "eor", 
            0x5d => "eor", 
            0x5e => "lsr", 
            0x60 => "rts", 
            0x61 => "adc", 
            0x65 => "adc", 
            0x66 => "ror", 
            0x68 => "pla",
            0x69 => "adc", 
            0x6a => "ror", 
            0x6c => "jmp", 
            0x6d => "adc", 
            0x6e => "ror", 
            0x70 => "bvs", 
            0x71 => "adc",
            0x75 => "adc", 
            0x76 => "ror", 
            0x78 => "sei", 
            0x79 => "adc", 
            0x7d => "adc", 
            0x7e => "ror", 
            0x81 => "sta", 
            0x84 => "sty", 
            0x85 => "sta", 
            0x86 => "stx", 
            0x88 => "dey", 
            0x8a => "txa", 
            0x8c => "sty",
            0x8d => "sta", 
            0x8e => "stx", 
            0x90 => "bcc", 
            0x91 => "sta", 
            0x94 => "sty", 
            0x95 => "sta",
            0x96 => "stx", 
            0x98 => "tya", 
            0x99 => "sta", 
            0x9a => "txs", 
            0x9d => "sta", 
            0xa0 => "ldy", 
            0xa1 => "lda", 
            0xa2 => "ldx", 
            0xa4 => "ldy", 
            0xa5 => "lda", 
            0xa6 => "ldx", 
            0xa8 => "tay", 
            0xa9 => "lda", 
            0xaa => "tax", 
            0xac => "ldy", 
            0xad => "lda", 
            0xae => "ldx", 
            0xb0 => "bcs",
            0xb1 => "lda", 
            0xb4 => "ldy", 
            0xb5 => "lda", 
            0xb6 => "ldx", 
            0xb8 => "clv", 
            0xb9 => "lda",
            0xba => "tsx", 
            0xbc => "ldy", 
            0xbd => "lda", 
            0xbe => "ldx", 
            0xc0 => "cpy", 
            0xc1 => "cmp", 
            0xc4 => "cpy", 
            0xc5 => "cmp", 
            0xc6 => "dec", 
            0xc8 => "iny", 
            0xc9 => "cmp", 
            0xca => "dex",
            0xcb => "axs",
            0xcc => "cpy", 
            0xcd => "cmp", 
            0xce => "dec", 
            0xd0 => "bne", 
            0xd1 => "cmp", 
            0xd5 => "cmp", 
            0xd6 => "dec", 
            0xd8 => "cld", 
            0xd9 => "cmp", 
            0xdd => "cmp", 
            0xde => "dec", 
            0xe0 => "cpx", 
            0xe1 => "sbc", 
            0xe4 => "cpx", 
            0xe5 => "sbc", 
            0xe6 => "inc", 
            0xe8 => "inx", 
            0xe9 => "sbc",
            0xea => "nop",
            0xec => "cpx", 
            0xed => "sbc", 
            0xee => "inc", 
            0xf0 => "beq", 
            0xf1 => "sbc", 
            0xf5 => "sbc", 
            0xf6 => "inc", 
            0xf8 => "sed", 
            0xf9 => "sbc", 
            0xfd => "sbc", 
            0xfe => "inc",
            _ =>    "ERR"
        }        
    }
    
    pub fn fetch(&mut self, mmu: &mut Mmu) {
        self.current_opcode = mmu.read_u8(self.pc);
    }
        
    pub fn execute(&mut self, mmu: &mut Mmu) {
        match self.current_opcode {
            0x00 => self.brk(mmu),
            0x01 => self.ora(mmu), 
            0x05 => self.ora(mmu),
            0x06 => self.asl(mmu),
            0x08 => self.php(mmu),
            0x09 => self.ora(mmu),
            0x0a => self.asl(mmu), 
            0x0d => self.ora(mmu), 
            0x0e => self.asl(mmu),
            0x10 => self.bpl(mmu), 
            0x11 => self.ora(mmu), 
            0x15 => self.ora(mmu), 
            0x16 => self.asl(mmu), 
            0x18 => self.clc(), 
            0x19 => self.ora(mmu), 
            0x1d => self.ora(mmu), 
            0x1e => self.asl(mmu), 
            0x20 => self.jsr(mmu),
            0x21 => self.and(mmu), 
            0x24 => self.bit(mmu), 
            0x25 => self.and(mmu), 
            0x26 => self.rol(mmu), 
            0x28 => self.plp(mmu), 
            0x29 => self.and(mmu),
            0x2a => self.rol(mmu), 
            0x2c => self.bit(mmu), 
            0x2d => self.and(mmu), 
            0x2e => self.rol(mmu), 
            0x30 => self.bmi(mmu), 
            0x31 => self.and(mmu), 
            0x32 => self.nop(),
            0x33 => self.nop(), 
            0x34 => self.nop(), 
            0x35 => self.and(mmu), 
            0x36 => self.rol(mmu), 
            0x38 => self.sec(), 
            0x39 => self.and(mmu), 
            0x3d => self.and(mmu), 
            0x3e => self.rol(mmu), 
            0x40 => self.rti(mmu), 
            0x41 => self.eor(mmu), 
            0x45 => self.eor(mmu), 
            0x46 => self.lsr(mmu), 
            0x48 => self.pha(mmu), 
            0x49 => self.eor(mmu), 
            0x4a => self.lsr(mmu), 
            0x4c => self.jmp(mmu), 
            0x4d => self.eor(mmu),
            0x4e => self.lsr(mmu), 
            0x50 => self.bvc(mmu), 
            0x51 => self.eor(mmu), 
            0x55 => self.eor(mmu), 
            0x56 => self.lsr(mmu),
            0x58 => self.cli(), 
            0x59 => self.eor(mmu), 
            0x5d => self.eor(mmu), 
            0x5e => self.lsr(mmu), 
            0x60 => self.rts(mmu), 
            0x61 => self.adc(mmu), 
            0x65 => self.adc(mmu), 
            0x66 => self.ror(mmu), 
            0x68 => self.pla(mmu),
            0x69 => self.adc(mmu), 
            0x6a => self.ror(mmu), 
            0x6c => self.jmp(mmu), 
            0x6d => self.adc(mmu), 
            0x6e => self.ror(mmu), 
            0x70 => self.bvs(mmu), 
            0x71 => self.adc(mmu),
            0x75 => self.adc(mmu), 
            0x76 => self.ror(mmu), 
            0x78 => self.sei(), 
            0x79 => self.adc(mmu), 
            0x7d => self.adc(mmu), 
            0x7e => self.ror(mmu), 
            0x81 => self.sta(mmu), 
            0x84 => self.sty(mmu), 
            0x85 => self.sta(mmu), 
            0x86 => self.stx(mmu), 
            0x88 => self.dey(), 
            0x8a => self.txa(), 
            0x8c => self.sty(mmu), 
            0x8d => self.sta(mmu), 
            0x8e => self.stx(mmu), 
            0x90 => self.bcc(mmu), 
            0x91 => self.sta(mmu), 
            0x94 => self.sty(mmu), 
            0x95 => self.sta(mmu), 
            0x96 => self.stx(mmu), 
            0x98 => self.tya(), 
            0x99 => self.sta(mmu), 
            0x9a => self.txs(), 
            0x9d => self.sta(mmu), 
            0xa0 => self.ldy(mmu), 
            0xa1 => self.lda(mmu), 
            0xa2 => self.ldx(mmu), 
            0xa4 => self.ldy(mmu), 
            0xa5 => self.lda(mmu), 
            0xa6 => self.ldx(mmu), 
            0xa8 => self.tay(), 
            0xa9 => self.lda(mmu), 
            0xaa => self.tax(), 
            0xac => self.ldy(mmu), 
            0xad => self.lda(mmu), 
            0xae => self.ldx(mmu), 
            0xb0 => self.bcs(mmu),
            0xb1 => self.lda(mmu), 
            0xb4 => self.ldy(mmu), 
            0xb5 => self.lda(mmu), 
            0xb6 => self.ldx(mmu), 
            0xb8 => self.clv(), 
            0xb9 => self.lda(mmu),
            0xba => self.tsx(), 
            0xbc => self.ldy(mmu), 
            0xbd => self.lda(mmu), 
            0xbe => self.ldx(mmu), 
            0xc0 => self.cpy(mmu), 
            0xc1 => self.cmp(mmu), 
            0xc4 => self.cpy(mmu), 
            0xc5 => self.cmp(mmu), 
            0xc6 => self.dec(mmu), 
            0xc8 => self.iny(), 
            0xc9 => self.cmp(mmu), 
            0xca => self.dex(), 
            0xcc => self.cpy(mmu), 
            0xcd => self.cmp(mmu), 
            0xce => self.dec(mmu), 
            0xd0 => self.bne(mmu), 
            0xd1 => self.cmp(mmu), 
            0xd5 => self.cmp(mmu), 
            0xd6 => self.dec(mmu), 
            0xd8 => self.cld(), 
            0xd9 => self.cmp(mmu), 
            0xdd => self.cmp(mmu),
            0xde => self.dec(mmu), 
            0xe0 => self.cpx(mmu), 
            0xe1 => self.sbc(mmu), 
            0xe4 => self.cpx(mmu), 
            0xe5 => self.sbc(mmu), 
            0xe6 => self.inc(mmu),
            0xe8 => self.inx(), 
            0xe9 => self.sbc(mmu),
            0xea => self.nop(),
            0xec => self.cpx(mmu), 
            0xed => self.sbc(mmu), 
            0xee => self.inc(mmu), 
            0xf0 => self.beq(mmu), 
            0xf1 => self.sbc(mmu), 
            0xf5 => self.sbc(mmu), 
            0xf6 => self.inc(mmu), 
            0xf8 => self.sed(),
            0xf9 => self.sbc(mmu), 
            0xfd => self.sbc(mmu), 
            0xfe => self.inc(mmu),
            _ => { println!("Error, bad opcode: {0:x} at {1:04x}", self.current_opcode, self.pc); 
                process::exit(1);}
        }    
    }
    
    pub fn run_for_scanline(&mut self, mmu: &mut Mmu) {        
        loop {
            self.fetch(mmu);
            if self.is_debugging {
                println!("{:?}", self)
            }                        
            self.execute(mmu);
            if self.tick_count > TICKS_PER_SCANLINE { break; }
        }
    }
}