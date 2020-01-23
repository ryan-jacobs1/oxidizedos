use alloc::{vec, vec::Vec};
use lazy_static::lazy_static;
use crate::machine;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Ata_Status {
    // The Command/Status Porn returns a bit mask referring to the status of a channel when read.
    BSY  = 0x80,
    DRDY = 0x40,
    DF   = 0x20,
    DSC  = 0x10,
    DRQ  = 0x08,
    CORR = 0x04,
    IDX  = 0x02,
    ERR  = 0x01,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Ata_Errors {
    // The Features/Error Port, which returns the most recent error upon read, has these possible bit masks.
    BBK   = 0x80,
    UNC   = 0x40,
    MC    = 0x20,
    IDNF  = 0x10,
    MCR   = 0x08,
    ABRT  = 0x04,
    TK0NF = 0x02,
    AMNF  = 0x01,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Ata_Commands {
    // When you write to the Command/Status Port, you are executing one of the commands below.
    READ_PIO        = 0x20,
    READ_PIO_EXT    = 0x24,
    READ_DMA        = 0xC8,
    READ_DMA_EXT    = 0x25,
    WRITE_PIO       = 0x30,
    WRITE_PIO_EXT   = 0x34,
    WRITE_DMA       = 0xCA,
    WRITE_DMA_EXT   = 0x35,
    CACHE_FLUSH     = 0xE7,
    CACHE_FLUSH_EXT = 0xEA,
    PACKET          = 0xA0,
    IDENTIFY_PACKET = 0xA1,
    IDENTIFY        = 0xEC,
    ATAPI_CMD_READ  = 0xA8,
    ATAPI_CMD_EJECT = 0x1B,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Ata_Identity {
    DEVICETYPE    = 0,
    CYLINDERS     = 2,
    HEADS         = 6,
    SECTORS       = 12,
    SERIAL        = 20,
    MODEL         = 54,
    CAPABILITIES  = 98,
    FIELDVALID    = 106,
    MAX_LBA       = 120,
    COMMANDSETS   = 164,
    MAX_LBA_EXT   = 200,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Interface_Type {
    ATA     = 0x00,
    ATAPI   = 0x01,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Drive_Type {
    MASTER  = 0x00,
    SLAVE   = 0x01,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Channel {
    Primary     = 0x00,
    Secondary   = 0x01,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Direction {
    Read     = 0x00,
    Write    = 0x01,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
#[repr(u16)]
pub enum Register {
    DATA        = 0x00,
    ERROR_OR_FEATURES       = 0x01,
    SECCOUNT0   = 0x02,
    LBA0        = 0x03,
    LBA1        = 0x04,
    LBA2        = 0x05,
    HDDEVSEL    = 0x06,
    COMMAND_OR_STATUS     = 0x07,
    SECCOUTN1   = 0x08,
    LBA3        = 0x09,
    LBA4        = 0x0A,
    LBA5        = 0x0B,
    CONTROL_OR_ALTSTATUS     = 0x0C,
    DEVADDRESS  = 0x0D,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Ide_Channel_Registers {
    base: u16,
    ctrl: u16,
    bmide: u16,
    nIEN: u8,
}

impl Ide_Channel_Registers {
    fn read(&self, reg: u16) -> u8 {
        let mut result: u8 = 0;
        if reg > 0x07 && reg < 0x0C {
            self.write(Register::CONTROL_OR_ALTSTATUS as u16, 0x80 | self.nIEN);
        }
        if reg < 0x08 {
            result = unsafe { machine::inb((self.base + reg - 0x00) as u32) };
        }
        else if reg < 0x0C {
            result = unsafe { machine::inb((self.base + reg - 0x06) as u32) };
        }
        else if reg < 0x0E {
            result = unsafe { machine::inb((self.ctrl + reg - 0x0A) as u32) };
        }
        else if reg < 0x16 {
            result = unsafe { machine::inb((self.bmide + reg - 0x0E) as u32) };
        }
        if reg > 0x07 && reg < 0x0C {
            self.write(Register::CONTROL_OR_ALTSTATUS as u16, 0x80 | self.nIEN);
        }
        result
    }

    fn write(&self, reg: u16, data: u8) {
        if reg > 0x07 && reg < 0x0C {
            self.write(Register::CONTROL_OR_ALTSTATUS as u16, 0x80 | self.nIEN);
        }
        if reg < 0x08 {
            unsafe { machine::outb((self.base + reg - 0x00) as u32, data as u32) };
        }
        else if reg < 0x0C {
            unsafe { machine::outb((self.base + reg - 0x06) as u32, data as u32) };
        }
        else if reg < 0x0E {
            unsafe { machine::outb((self.ctrl + reg - 0x0A) as u32, data as u32) };
        }
        else if reg < 0x16 {
            unsafe { machine::outb((self.bmide + reg - 0x0E) as u32, data as u32) };
        }
        if reg > 0x07 && reg < 0x0C {
            self.write(Register::CONTROL_OR_ALTSTATUS as u16, 0x80 | self.nIEN);
        }
    }

    fn read_buffer(&self, reg: u16, buffer: u32, quads: u32) {
        if reg > 0x07 && reg < 0x0C {
            self.write(Register::CONTROL_OR_ALTSTATUS as u16, 0x80 | self.nIEN);
        }
        asm!();
        if reg < 0x08 {
            unsafe { machine::in((self.base + reg - 0x00) as u32, data as u32) };
        }
        else if reg < 0x0C {
            unsafe { machine::outb((self.base + reg - 0x06) as u32, data as u32) };
        }
        else if reg < 0x0E {
            unsafe { machine::outb((self.ctrl + reg - 0x0A) as u32, data as u32) };
        }
        else if reg < 0x16 {
            unsafe { machine::outb((self.bmide + reg - 0x0E) as u32, data as u32) };
        }
        if reg > 0x07 && reg < 0x0C {
            self.write(Register::CONTROL_OR_ALTSTATUS as u16, 0x80 | self.nIEN);
        } 
    }
}

static mut channels: [Ide_Channel_Registers; 2] = [Ide_Channel_Registers {base: 0, ctrl: 0, bmide: 0, nIEN: 0}; 2];
lazy_static! {
    static ref ide_devices: [Ide_Device; 4] = [Ide_Device::new(), Ide_Device::new(), Ide_Device::new(), Ide_Device::new()];
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Ide_Device {
    reserved: u8,
    channel: u8,
    drive: u8,
    device_type: u16,
    signature: u16,
    capabilities: u16,
    commandsets: u32,
    size: u32,
    model: Vec<u8>,
}

impl Ide_Device {
    fn new() -> Ide_Device {
        Ide_Device {
            reserved: 0,
            channel: 0,
            drive: 0,
            device_type: 0,
            signature: 0,
            capabilities: 0,
            commandsets: 0,
            size: 0,
            model: vec![0; 41],
        }
    }
}