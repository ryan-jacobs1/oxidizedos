extern crate spin;



use spin::Mutex;


use crate::println;
use crate::config::mb_memory_map;
use crate::config::mb_info_memory;
use crate::config::mb_info_memory_entry;

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

#[repr(C, packed)]
pub struct AddressSpace {
    entries: [AddressSpaceEntry; 512],
}

bitfield! {
    #[repr(C, packed)]
    pub struct AddressSpaceEntry(u64);
    present, set_present: 0, 0;
    writable, set_writable: 1, 1;
    user_supervisor, set_user_supervisor: 2, 2;
    u64;
    physical_addr, set_physical_addr: 51, 12;
}
static VMM_ALLOCATOR: Mutex<VMAllocator> = spin::Mutex::new(VMAllocator{next: 0, start_phys_mem: 0x150000, end_phys_mem: 0});
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
    let x = AddressSpaceEntry{0: 1};
    let y = core::mem::size_of::<AddressSpaceEntry>();
    println!("sizeof entry is {}", y);
    println!("sizeof address space is 0x{:x}", core::mem::size_of::<AddressSpace>());
    x.present();
}

pub fn alloc() -> u64 {
    let mut vmm_allocator = VMM_ALLOCATOR.lock();
    if (vmm_allocator.start_phys_mem != vmm_allocator.end_phys_mem) {
        let result = vmm_allocator.start_phys_mem;
        vmm_allocator.start_phys_mem += PAGE_SIZE;
        result
    }
    else {
        // TODO: Demand paging
        if (vmm_allocator.next == 0) {
            panic!("Out of physical frames.");
        }
        let result = vmm_allocator.next;
        let ptr = result as *mut u64;
        unsafe {
            vmm_allocator.next = *ptr;
        }
        result
    }
}