use crate::machine;
use crate::thread;
use crate::println;

pub static ports: [u32; 2] = [0x1f0, 0x170];
pub static ERR: u8 = 0x01;
pub static DRQ: u8 = 0x08;
pub static SRV: u8 = 0x10;
pub static DF: u8 = 0x20;
pub static DRDY: u8 = 0x40;
pub static BSY: u8 = 0x80;

pub trait IDE {
    fn read_sector(&self, sector: u32, buffer: &mut [u32]);
    fn write_sector(&self, sector: u32, buffer: &[u32]);
    // Reads up to n bytes. Returns the actual number of bytes read.
    fn read(&self, offset: u32, buffer: &mut [u32], n: u32) -> u32;
    // Reads n bytes. Returns the number of bytes read.
    fn read_all(&self, offset: u32, buffer: &mut [u32], n: u32) -> u32;
    fn write(&self, offset: u32, buffer: &[u32], n: u32) -> u32;
    fn write_all(&self, offset: u32, buffer: &[u32], n: u32) -> u32;
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

    fn write_sector(&self, sector: u32, buffer: &[u32]) {
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

    fn read(&self, offset: u32, buffer: &mut [u32], n: u32) -> u32 {
        let sector = offset / IDEImpl::SECTOR_SIZE;
        let start = offset % IDEImpl::SECTOR_SIZE;
        let mut end = start + n;
        if end > IDEImpl::SECTOR_SIZE {
            end = IDEImpl::SECTOR_SIZE;
        }
        let count = end - start;
        if buffer.len() * 4 < count as usize{
            panic!("Buffer too small");
        }
        if count == IDEImpl::SECTOR_SIZE {
            self.read_sector(sector, buffer);
        } else if count != 0 {
            let mut sector_buf: [u32; 512 / 4] = [0; 512 / 4];
            self.read_sector(sector, &mut sector_buf);
            let mut sector_buf_u8 = unsafe {
                core::mem::transmute::<&mut [u32], &mut [u8]>(&mut sector_buf)
            };
            let mut buffer_u8 = unsafe {
                core::mem::transmute::<&mut [u32], &mut [u8]>(buffer)
            };
            unsafe { core::ptr::copy(&sector_buf_u8[start as usize] as *const u8, &mut buffer_u8[0] as *mut u8, count as usize); }
        }
        count
    }

    fn read_all(&self, offset: u32, buffer: &mut [u32], n: u32) -> u32 {
        let mut buf_u8: &mut [u8] = unsafe {
            core::mem::transmute::<&mut [u32], &mut [u8]>(buffer)
        };
        let mut temp_buf: [u32; 512 / 4] = [0; 512 / 4];
        let mut current_offset = offset;
        let mut bytes_remaining = n;
        let mut index = 0;
        while bytes_remaining > 0 {
            let count = self.read(current_offset, &mut temp_buf, bytes_remaining);
            let mut temp_buf_u8 = unsafe {
                core::mem::transmute::<&mut [u32], &mut [u8]>(&mut temp_buf)
            };
            for i in index..index + count {
                buf_u8[i as usize] = temp_buf_u8[i as usize];
            }
            index += count;
            bytes_remaining -= count;
        }
        n
    }

    fn write(&self, offset: u32, buffer: &[u32], n: u32) -> u32 {
        let sector = offset / IDEImpl::SECTOR_SIZE;
        let start = offset % IDEImpl::SECTOR_SIZE;
        let mut end = start + n;
        if end > IDEImpl::SECTOR_SIZE {
            end = IDEImpl::SECTOR_SIZE;
        }
        let count = end - start;
        if count == IDEImpl::SECTOR_SIZE {
            self.write_sector(sector, buffer);
        } else if count != 0 {
            let mut temp_buf: [u32; 512 / 4] = [0; 512 / 4];
            self.read_sector(sector, &mut temp_buf);
            let mut temp_buf_u8 = unsafe {
                core::mem::transmute::<&mut [u32], &mut [u8]>(&mut temp_buf)
            };
            let buffer_u8 = unsafe {
                core::mem::transmute::<&[u32], &[u8]>(buffer)
            };
            unsafe {core::ptr::copy(&buffer_u8[0] as *const u8, &mut temp_buf_u8[start as usize] as *mut u8, count as usize);}
            self.write_sector(sector, &temp_buf);
        }
        count
    }
    
    fn write_all(&self, offset: u32, buffer: &[u32], n: u32) -> u32 {
        let buf_u8: &[u8] = unsafe {
            core::mem::transmute::<&[u32], &[u8]>(buffer)
        };
        let mut temp_buf: [u32; 512 / 4] = [0; 512 / 4];
        let mut current_offset = offset;
        let mut bytes_remaining = n;
        let mut index = 0;
        while bytes_remaining > 0 {
            let mut temp_buf_u8 = unsafe {
                core::mem::transmute::<&mut [u32], &mut [u8]>(&mut temp_buf)
            };
            let to_copy = {
                if bytes_remaining < IDEImpl::SECTOR_SIZE {
                    bytes_remaining
                } else {
                    IDEImpl::SECTOR_SIZE
                }
            };
            for i in index..index + to_copy {
                temp_buf_u8[i as usize] = buf_u8[i as usize];
            }
            let count = self.write(current_offset, &mut temp_buf, bytes_remaining);
            index += count;
            bytes_remaining -= count;
        }
        n
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
