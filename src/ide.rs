use crate::machine;
use crate::thread;


pub static ports: [u32; 2] = [0x1f0, 0x170];
pub static ERR: u8 = 0x01;
pub static DRQ: u8 = 0x08;
pub static SRV: u8 = 0x10;
pub static DF: u8 = 0x20;
pub static DRDY: u8 = 0x40;
pub static BSY: u8 = 0x80;

pub struct IDE {
    drive: u32
}

impl IDE {
    const SECTOR_SIZE: u32 = 512;

    pub fn read_sector(sector: u32, buffer: &mut [u32]) {
        if buffer.len() * 4 < IDE::SECTOR_SIZE as usize {
            panic!("Cannot read sector size bytes into buffer");
        }

    }
}

fn controller(drive: u32) -> u32 {
    (drive >> 1) & 1
}

fn channel(drive: u32) -> u32 {
    drive & 1
}

fn port(drive: u32) -> u32 {
    ports[controller(drive) as usize]
}

fn get_status(drive: u32) -> u8 {
    unsafe { machine::inb(port(drive) + 7) }
}

fn wait_for_drive(drive: u32) {
    let status = get_status(drive) as u8;
    if (status & (ERR | DF)) != 0 {
        panic!("drive error, device:{:x}, status:{:x}", drive, status);
    }
    if (status & DRDY) == 0 {
        panic!("drive {:x} is not ready, status: {:x}", drive, status);
    }
    // TODO Block instead of polling
    while (get_status(drive) & BSY) != 0 {
        thread::surrender();
    }
}
