use crate::config::mb_memory_map;
use crate::config::mb_info_memory;
use crate::config::mb_info_memory_entry;


pub struct VMAllocator {
    next: u64,
}


pub fn init() {
    let mut vmm_allocator = VMAllocator{next: 0};
    unsafe {
        if let Some(ref memory_map) = mb_memory_map {
            let entry = memory_map.first_entry();
            for i in 0..memory_map.num_entries() {
                if entry.mem_type == 1 {
                    let ptrBase = entry.base_addr;
                    for j in 0..entry.length / 0x1000 {
                        let mut ptr = ptrBase + (j * 0x1000);
                        *(ptr as *mut u64) = vmm_allocator.next;
                        vmm_allocator.next = ptr;
                    }
                }
            }
        }
    }
}

pub fn alloc() {

}