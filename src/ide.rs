use crate::machine;
use crate::thread;


pub static ports: [u32; 2] = [0x1f0, 0x170];
pub static ERR: u8 = 0x01;
pub static DRQ: u8 = 0x08;
pub static SRV: u8 = 0x10;
pub static DF: u8 = 0x20;
pub static DRDY: u8 = 0x40;
pub static BSY: u8 = 0x80;

pub trait IDE {
    fn read_sector(&self, sector: u32, buffer: &mut [u32]);
    fn write_sector(&self, sector: u32, buffer: &mut [u32]);
}

pub struct IDEImpl {
    drive: u32
}

impl IDEImpl {
    const SECTOR_SIZE: u32 = 512;
    
    pub fn new(drive: u32) -> IDEImpl {
        IDEImpl {drive: drive}
    }
}

impl IDE for IDEImpl {
    fn read_sector(&self, sector: u32, buffer: &mut [u32]) {
        if buffer.len() * 4 < IDEImpl::SECTOR_SIZE as usize {
            panic!("Cannot read sector size bytes into buffer");
        }
        let base = port(self.drive);
        let ch = channel(self.drive);
        wait_for_drive(self.drive);
        unsafe {
            machine::outb(base + 2, 1);			// sector count
            machine::outb(base + 3, sector >> 0);	// bits 7 .. 0
            machine::outb(base + 4, sector >> 8);	// bits 15 .. 8
            machine::outb(base + 5, sector >> 16);	// bits 23 .. 16
            machine::outb(base + 6, 0xE0 | (ch << 4) | ((sector >> 24) & 0xf));
            machine::outb(base + 7, 0x20);		// read with retry
        }
        wait_for_drive(self.drive);

        while get_status(self.drive) & DRQ == 0 {
            thread::surrender();
        }
        
        // TODO use DMA (if supported)
        for i in 0..IDEImpl::SECTOR_SIZE as usize / core::mem::size_of::<u32>() {
            buffer[i as usize] = unsafe { machine::inl(base) }; 
        }
    }

    fn write_sector(&self, sector: u32, buffer: &mut [u32]) {
        if buffer.len() * 4 < IDEImpl::SECTOR_SIZE as usize {
            panic!("Cannot write sector size bytes to disk");
        }
        let base = port(self.drive);
        let ch = channel(self.drive);
        wait_for_drive(self.drive);
        unsafe {
            machine::outb(base + 2, 1);			// sector count
            machine::outb(base + 3, sector >> 0);	// bits 7 .. 0
            machine::outb(base + 4, sector >> 8);	// bits 15 .. 8
            machine::outb(base + 5, sector >> 16);	// bits 23 .. 16
            machine::outb(base + 6, 0xE0 | (ch << 4) | ((sector >> 24) & 0xf));
            machine::outb(base + 7, 0x30);		// write
        }
        wait_for_drive(self.drive);

        while get_status(self.drive) & DRQ == 0 {
            thread::surrender();
        }

        // TODO use DMA (if supported)
        for i in 0..IDEImpl::SECTOR_SIZE as usize / core::mem::size_of::<u32>() {
            unsafe {
                machine::outl(base, buffer[i]);
            }
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
