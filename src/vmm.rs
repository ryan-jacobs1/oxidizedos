extern crate spin;

use spin::Mutex;

use crate::config::mb_info_memory;
use crate::config::mb_info_memory_entry;
use crate::config::mb_memory_map;
use crate::println;

/*
 * Not a particularly impressive allocator, but works fine in QEMU.
 * Assumes all physical pages are available from start_phys_mem to end_phys_mem
 * and allocates them in order. Then uses next as an intrusive linked list, where
 * next is a page aligned value representing the next available physical page.
 */
pub struct VMAllocator {
    next: u64,
    start_phys_mem: u64,
    end_phys_mem: u64,
}

#[repr(C, align(4096))]
pub struct AddressSpace {
    entries: [AddressSpaceEntry; 512],
}

impl AddressSpace {
    pub fn new() -> *mut AddressSpace {
        alloc() as *mut AddressSpace
    }
    pub fn create_mapping(&mut self, vpn: u64, ppn: u64) {
        self.create_mapping_helper(Address{0: vpn}, ppn, 4);
    }
    fn create_mapping_helper(&mut self, vpn: Address, ppn: u64, level: u32) {
        let index = match level {
            4 => vpn.pml4_index(),
            3 => vpn.pdpt_index(),
            2 => vpn.pd_index(),
            1 => vpn.pt_index(),
            _ => {panic!("Invalid paging structure level");}
        } as usize;
        let mut entry = &mut self.entries[index];
        match level {
            1 => {
                entry.set_present(1);
                entry.set_physical_addr(ppn);
            }
            2..=4 => {
                if (entry.present() == 0) {
                    entry.set_present(1);
                    entry.set_physical_addr(alloc() / PAGE_SIZE);
                }
                entry.get_address_space().create_mapping_helper(vpn, ppn, level - 1);
            }
            _ => {panic!("Invalid paging structure level");}
        }
    }
    pub fn create_huge_mapping(&mut self, vpn: u64, ppn: u64) {
        self.create_huge_mapping_helper(Address{0: vpn}, ppn, 4);
    }
    fn create_huge_mapping_helper(&mut self, vpn: Address, ppn: u64, level: u32) {
        let index = match level {
            4 => vpn.pml4_index(),
            3 => vpn.pdpt_index(),
            2 => vpn.pd_index(),
            _ => {panic!("Invalid paging structure level");}
        } as usize;
        let mut entry = &mut self.entries[index];
        match level {
            2 => {
                entry.set_huge(1);
                entry.set_present(1);
                entry.set_physical_addr(ppn);
            }
            3..=4 => {
                if (entry.present() == 0) {
                    entry.set_present(1);
                    entry.set_physical_addr(alloc() / PAGE_SIZE);
                }
                entry.get_address_space().create_mapping_helper(vpn, ppn, level - 1);
            }
            _ => {panic!("Invalid paging structure level");}
        }
    }
}

/**
 * Represents a memory address in terms of its indices into
 * the paging structure
 */
bitfield! {
    #[repr(transparent)]
    pub struct Address(u64);
    u64;
    pt_index, _: 8, 0;
    pd_index, _: 17, 9;
    pdpt_index, _: 26, 18;
    pml4_index, _: 35, 27;
}

/**
 * Represents an entry in the PML4/PDPT/PD/PT
 */
bitfield! {
    #[repr(transparent)]
    pub struct AddressSpaceEntry(u64);
    present, set_present: 0, 0;
    writable, set_writable: 1, 1;
    user_supervisor, set_user_supervisor: 2, 2;
    huge, set_huge: 7, 7;
    u64;
    physical_addr, set_physical_addr: 51, 12;

}

impl AddressSpaceEntry {
    pub fn get_address_space(&self) -> &mut AddressSpace {
        unsafe {
            &mut *((self.physical_addr() + PAGE_SIZE) as *mut AddressSpace)
        }
    }
}

static VMM_ALLOCATOR: Mutex<VMAllocator> = spin::Mutex::new(VMAllocator {
    next: 0,
    start_phys_mem: 0x150000,
    end_phys_mem: 0,
});
static PAGE_SIZE: u64 = 0x1000;

pub fn init() {
    let mut vmm_allocator = VMM_ALLOCATOR.lock();
    unsafe {
        if let Some(ref memory_map) = mb_memory_map {
            let mut entry = memory_map.first_entry();
            for i in 0..memory_map.num_entries() {
                if entry.mem_type == 1 {
                    let highAddr = entry.base_addr + entry.length;
                    if (vmm_allocator.end_phys_mem < highAddr) {
                        vmm_allocator.end_phys_mem = highAddr;
                    }
                }
                if i != memory_map.num_entries() - 1 {
                    entry = entry.get_next(memory_map.entry_size as usize);
                }
            }
        }
    }
    println!("end_phys_mem: {:x}", vmm_allocator.end_phys_mem);
    let x = AddressSpaceEntry { 0: 1 };
    let y = core::mem::size_of::<AddressSpaceEntry>();
    println!("sizeof entry is {}", y);
    println!(
        "sizeof address space is 0x{:x}",
        core::mem::size_of::<AddressSpace>()
    );
    x.present();
}

pub fn alloc() -> u64 {
    let mut vmm_allocator = VMM_ALLOCATOR.lock();
    let result: u64;
    if (vmm_allocator.start_phys_mem != vmm_allocator.end_phys_mem) {
        result = vmm_allocator.start_phys_mem;
        vmm_allocator.start_phys_mem += PAGE_SIZE;
    } else {
        // TODO: Demand paging
        if (vmm_allocator.next == 0) {
            panic!("Out of physical frames.");
        }
        result = vmm_allocator.next;
        let ptr = result as *mut u64;
        unsafe {
            vmm_allocator.next = *ptr;
        }
    }
    unsafe {
        core::ptr::write_bytes(result as *mut u8, 0, PAGE_SIZE as usize);
    }
    result
}
