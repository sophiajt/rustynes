use std::io;
use std::io::prelude::*;
use std::fs::File;

pub trait BitReader {
    fn read_u16_be(&mut self) -> Result<u16, io::Error>;
    fn read_u32_be(&mut self) -> Result<u32, io::Error>;

    fn read_u16_le(&mut self) -> Result<u16, io::Error>;
    fn read_u32_le(&mut self) -> Result<u32, io::Error>;

    fn read_u8(&mut self) -> Result<u8, io::Error>;
}

impl BitReader for File {
    fn read_u32_be(&mut self) -> Result<u32, io::Error> {
        let mut buffer = [0; 4];
    
        try!(self.read(&mut buffer));
        
        Ok(buffer[3] as u32 + ((buffer[2] as u32) << 8) +
            ((buffer[1] as u32) << 16) + ((buffer[0] as u32) << 24))    
    }
    
    fn read_u16_be(&mut self) -> Result<u16, io::Error> {
        let mut buffer = [0; 2];
    
        try!(self.read(&mut buffer));
        
        Ok(buffer[1] as u16 + ((buffer[0] as u16) << 8))    
    }   

    fn read_u32_le(&mut self) -> Result<u32, io::Error> {
        let mut buffer = [0; 4];
    
        try!(self.read(&mut buffer));
        
        Ok(buffer[0] as u32 + ((buffer[1] as u32) << 8) +
            ((buffer[2] as u32) << 16) + ((buffer[3] as u32) << 24))    
    }
    
    fn read_u16_le(&mut self) -> Result<u16, io::Error> {
        let mut buffer = [0; 2];
    
        try!(self.read(&mut buffer));
        
        Ok(buffer[0] as u16 + ((buffer[1] as u16) << 8))    
    }    

    fn read_u8(&mut self) -> Result<u8, io::Error> {
        let mut buffer = [0; 1];
    
        try!(self.read(&mut buffer));
        
        Ok(buffer[0] as u8)    
    }    
}

