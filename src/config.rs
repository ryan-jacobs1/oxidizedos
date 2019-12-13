use crate::println;

use core::str::from_utf8;

// Lots of unsafe code in this file, but that's OK,
// since all of this code is run by one core (on one thread),
// so no race conditions here

pub static mut mb_memory_map: Option<&mb_info_memory> = None;
pub static mut rsdp: Option<&RSDP> = None;
pub static mut rsdt: Option<&ACPIHeader> = None;
pub static mut madt: Option<&MADT> = None;
pub static mut config: Option<Config> = None;

pub struct Config {
    local_apic: u32,
    io_apic: u32,
    num_other_procs: u32,
    total_procs: u32,
}


struct APICInfo {
    processor_id: u8,
    apic_id: u8,
    flags: u32,
}

#[repr(C, packed)]
struct MADTEntry {
    entry_type: u8,
    record_length: u8,  
}

impl MADTEntry {
    pub fn next_entry(&self) -> *const MADTEntry {
        (self as *const MADTEntry as usize + self.record_length as usize) as *const MADTEntry
    }
    pub fn print(&self) {
        println!("entry type {} record length {} ", self.entry_type, self.record_length);
    }
}

#[repr(C, packed)]
struct LAPICEntry {
    entry_type: u8,
    record_length: u8,
    acpi_processor_id: u8,
    apic_id: u32
}


#[repr(C)]
pub struct mb_info {
    mb_type: u32,
    size: u32,
}

#[repr(C)]
pub struct mb_info_memory {
    pub mb_type: u32,
    pub size: u32,
    pub entry_size: u32,
    pub entry_version: u32,
}

#[repr(C)]
pub struct mb_info_memory_entry {
    pub base_addr: u64,
    pub length: u64,
    pub mem_type: u32,
    pub reserved: u32,
}

#[repr(C, packed)]
pub struct ACPIHeader {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oemid: [u8; 6],
    oemtableid: [u8; 8],
    oemrevision: u32,
    creator_id: u32,
    creator_revision: u32
}


impl ACPIHeader {
    pub fn print(&self) {
        println!("ACPIHeader: signature: {} length {} revision {} checksum {} oemid {} oemtableid {} oemrevision {} creator_id {} creator_revision {}", from_utf8(&self.signature).unwrap(), self.length, self.revision, self.checksum, from_utf8(&self.oemid).unwrap(), from_utf8(&self.oemtableid).unwrap(), self.oemrevision, self.creator_id, self.creator_revision);
    }
    pub fn find_sdt(&self, signature: &[u8]) -> Result<&ACPIHeader, ()> {
        let num_entries = (self.length as usize - core::mem::size_of::<ACPIHeader>()) / 4;
        for i in 0..num_entries {
            let table_ptr = (self as *const ACPIHeader as usize + core::mem::size_of::<ACPIHeader>()) + (i * 4);
            let table = unsafe {&(*(*(table_ptr as usize as *const u32) as *const ACPIHeader))};
            if (table.signature == signature) {
                return Ok(table);
            }
            table.print();
        }
        Err(())
    }
}

#[repr(C, packed)]
pub struct MADT {
    header: ACPIHeader,
    local_apic_addr: u32,
    flags: u32
}

impl MADT {
    pub fn first_entry(&self) -> *const MADTEntry {
        (self as *const MADT as usize + core::mem::size_of::<MADT>()) as *const MADTEntry
    }
    pub fn length_of_entries(&self) -> usize {
        self.header.length as usize - core::mem::size_of::<MADT>()
    }
}

#[repr(C, packed)]
pub struct RSDP {
    mb_type: u32,
    size: u32,
    signature: [u8; 8],
    checksum: u8,
    oemid: [u8; 6],
    revision: u8,
    rsdt_address: u32,
}

impl RSDP {
    pub fn print(&self) {
        println!("signature: {}, checksum {} oemid {} revision {} rsdt_address 0x{:x}", from_utf8(&self.signature).unwrap(), self.checksum, from_utf8(&self.oemid).unwrap(), self.revision, self.rsdt_address);
    }
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
                14 => {
                    unsafe {
                        let rsdp_temp = &*(current as *const mb_info as *const RSDP);
                        rsdp_temp.print();
                        rsdp = Some(rsdp_temp);
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
            current = current.get_next(self.entry_size as usize);
        }
    }
    pub unsafe fn first_entry(&self) -> &mb_info_memory_entry {
        &*(((self as *const mb_info_memory as usize) + 16) as *const mb_info_memory_entry)
    }
    pub fn num_entries(&self) -> u32 {
        (self.size - 16) / self.entry_size
    }
}

impl mb_info_memory_entry {
    pub fn print(&self) {
        println!("Range 0x{:x}-0x{:x} length {} num pages {:x} mem_type {} reserved {}", self.base_addr, self.base_addr + self.length, self.length, self.length / 0x1000, self.mem_type, self.reserved);
    }
    pub fn get_next(&self, entry_size: usize) -> &mb_info_memory_entry {
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

pub fn initialize_rsdt() {
    println!("initialzing rsdt");
    unsafe {
        if let Some(ref rsdp_temp) = rsdp {
            let rsdt_temp = &*(rsdp_temp.rsdt_address as *const ACPIHeader);
            rsdt_temp.print();
            unsafe {
                rsdt = Some(rsdt_temp);
            }
        }
        else {
            panic!("RSDT cannot be initialized");
        }
    }
}

pub fn initialize_madt() {
    println!("Initializing MADT");
    unsafe {
        if let Some(ref rsdt_temp) = rsdt {
            let table = rsdt_temp.find_sdt(b"APIC");
            match table {
                Ok(x) => unsafe {madt = Some(&*(x as *const ACPIHeader as *const MADT));},
                Err(()) => {panic!("Failed to find MADT");},
            }
        }
    }
}

pub fn initialize_config() {
    unsafe {
        if let Some(ref madt_temp) = madt {
            println!("lapic base 0x{:x}", madt_temp.local_apic_addr);
            let mut total = 0;
            let length = madt_temp.length_of_entries();
            let mut entry = madt_temp.first_entry();
            while total < length {
                let entry_as_ref = unsafe {&*entry};
                entry_as_ref.print();
                entry = entry_as_ref.next_entry();
                total += entry_as_ref.record_length as usize;
            }
        }
    }
}

pub fn init_pre_paging(mb_config: &mb_info) {
    mb_config.find_all();
}

pub fn init_post_paging(mb_config: &mb_info) {
    initialize_rsdt();
    initialize_madt();
    initialize_config();
}