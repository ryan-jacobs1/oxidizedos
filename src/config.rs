use crate::println;

static mut mb_memory_map: Option<&mb_info_memory> = None;

#[repr(C)]
pub struct mb_info {
    mb_type: u32,
    size: u32,
}

#[repr(C)]
pub struct mb_info_memory {
    mb_type: u32,
    size: u32,
    entry_size: u32,
    entry_version: u32,
}

#[repr(C)]
pub struct mb_info_memory_entry {
    base_addr: u64,
    length: u64,
    mem_type: u32,
    reserved: u32,
}




impl mb_info {
    fn print(&self) {
        println!("type {} size {}", self.mb_type, self.size);
    }

    pub fn get_next(&self) -> &mb_info {
        unsafe {
            let current: usize = (self as *const mb_info) as usize;
            let next = self.align((current + self.size as usize)) as *const mb_info;
            &*next
        }
    }

    pub fn align(&self, addr: usize) -> usize {
        ((addr + 8 - 1) / 8) * 8
    }

    pub fn find_all(&self) {
        println!("doing find_all");
        let mut current: &mb_info = self;
        while current.mb_type != 0 {
            current.print();
            match current.mb_type {
                6 => {
                    unsafe {
                        mb_memory_map = Some(&*(current as *const mb_info as *const mb_info_memory))
                    }
                }
                _ => ()
            }
            current = current.get_next();
        }
    }
}

impl mb_info_memory {
    pub fn print(&self) {
        println!("type {} size {}, entry size {}, version {}", self.mb_type, self.size, self.entry_size, self.entry_version);
    }
    pub unsafe fn find_all(&self) {
        let mut current: &mb_info_memory_entry = &*(((self as *const mb_info_memory as usize) + 16) as *const mb_info_memory_entry);
        let num_entries = (self.size - 16) / self.entry_size;
        println!("Parsing {} entries in the memory map", num_entries);
        for i in (0..num_entries) {
            current.print();
            current = current.getNext(self.entry_size as usize);
        }
    }
}

impl mb_info_memory_entry {
    pub fn print(&self) {
        println!("Range 0x{:x}-0x{:x} length {} mem_type {} reserved {}", self.base_addr, self.base_addr + self.length, self.length, self.mem_type, self.reserved);
    }
    pub fn getNext(&self, entry_size: usize) -> &mb_info_memory_entry {
            let current: usize = (self as *const mb_info_memory_entry) as usize;
            let next= (current + entry_size) as *const mb_info_memory_entry;
        unsafe {
            &*next
        }
    }
}

pub fn memory_map_init() {
    println!("initializing memory map\n");
    unsafe {
        if let Some(ref memory) = mb_memory_map {
            memory.print();
            memory.find_all();
        }
        else {
            panic!("No memory map structure!");
        }
    }
}