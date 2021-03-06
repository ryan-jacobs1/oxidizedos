extern crate spin;

use spin::Mutex;

use crate::config::CONFIG;
use crate::machine;
use crate::println;

lazy_static! {
    pub static ref IDENTITY_MAP: Mutex<&'static mut AddressSpace> = Mutex::new(
        create_identity_mappings(unsafe { CONFIG.high_phys_mem } / PAGE_SIZE)
    );
}

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
#[derive(Copy, Clone)]
pub struct AddressSpace {
    entries: [AddressSpaceEntry; 512],
}

impl AddressSpace {
    pub fn new() -> *mut AddressSpace {
        alloc() as *mut AddressSpace
    }
    pub fn new_with_identity() -> &'static mut AddressSpace {
        let address_space = alloc() as *mut AddressSpace;
        let address_space_ref = unsafe { &mut *(address_space) };
        let identity = (*IDENTITY_MAP).lock();
        for i in 0..512 {
            address_space_ref.entries[i] = identity.entries[i];
        }
        address_space_ref
    }
    pub fn create_mapping(&mut self, vpn: u64, ppn: u64) {
        self.create_mapping_helper(Address { 0: vpn }, ppn, 4);
    }
    fn create_mapping_helper(&mut self, vpn: Address, ppn: u64, level: u32) {
        let index = match level {
            4 => vpn.pml4_index(),
            3 => vpn.pdpt_index(),
            2 => vpn.pd_index(),
            1 => vpn.pt_index(),
            _ => {
                panic!("Invalid paging structure level");
            }
        } as usize;
        let mut entry = &mut self.entries[index];
        match level {
            1 => {
                entry.set_present(1);
                entry.set_writable(1);
                entry.set_physical_addr(ppn);
            }
            2..=4 => {
                if entry.present() == 0 {
                    entry.set_present(1);
                    entry.set_writable(1);
                    entry.set_physical_addr(alloc() / PAGE_SIZE);
                }
                entry
                    .get_address_space()
                    .create_mapping_helper(vpn, ppn, level - 1);
            }
            _ => {
                panic!("Invalid paging structure level");
            }
        }
    }
    pub fn create_huge_mapping(&mut self, vpn: u64, ppn: u64) {
        self.create_huge_mapping_helper(Address { 0: vpn }, ppn, 4);
    }
    fn create_huge_mapping_helper(&mut self, vpn: Address, ppn: u64, level: u32) {
        let index = match level {
            4 => vpn.pml4_index(),
            3 => vpn.pdpt_index(),
            2 => vpn.pd_index(),
            _ => {
                panic!("Invalid paging structure level");
            }
        } as usize;
        let mut entry = &mut self.entries[index];
        match level {
            2 => {
                entry.set_huge(1);
                entry.set_present(1);
                entry.set_writable(1);
                entry.set_physical_addr(ppn);
            }
            3..=4 => {
                if entry.present() == 0 {
                    println!("creating additional page table");
                    entry.set_present(1);
                    entry.set_writable(1);
                    entry.set_physical_addr(alloc() / PAGE_SIZE);
                }
                entry
                    .get_address_space()
                    .create_huge_mapping_helper(vpn, ppn, level - 1);
            }
            _ => {
                panic!("Invalid paging structure level");
            }
        }
    }
    /// Load this address space in CR3
    pub fn activate(&self) {
        unsafe {
            println!(
                "switching to address space at 0x{:x}",
                self as *const AddressSpace as usize
            );
            machine::load_cr3(self as *const AddressSpace as u64);
        }
    }
}

fn create_identity_mappings(high_page: u64) -> &'static mut AddressSpace {
    let address_space = AddressSpace::new();
    let mut address_space_ref = unsafe { &mut *address_space };
    // The first 2MB
    for i in 1..0x200 {
        address_space_ref.create_mapping(i, i);
    }
    // Huge mappings
    let boundary = high_page - (high_page % 0x200);
    for i in (0x200..boundary).step_by(0x200) {
        address_space_ref.create_huge_mapping(i, i);
    }
    for i in boundary..high_page {
        address_space_ref.create_mapping(i, i);
    }
    // Map MMIO
    unsafe {
        let lapic: u64 = (CONFIG.local_apic as u64) / PAGE_SIZE;
        println!("mapping {:x}", lapic);
        address_space_ref.create_mapping(lapic, lapic);
    }
    address_space_ref
}

bitfield! {
    /**
    * Represents a memory address in terms of its indices into
    * the paging structure
    */
    #[repr(transparent)]
    pub struct Address(u64);
    u64;
    pt_index, _: 8, 0;
    pd_index, _: 17, 9;
    pdpt_index, _: 26, 18;
    pml4_index, _: 35, 27;
}

bitfield! {
    /**
    * Represents an entry in the PML4/PDPT/PD/PT
    */
    #[repr(transparent)]
    #[derive(Copy, Clone)]
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
        unsafe { &mut *((self.physical_addr() * PAGE_SIZE) as *mut AddressSpace) }
    }
}

static VMM_ALLOCATOR: Mutex<VMAllocator> = spin::Mutex::new(VMAllocator {
    next: 0,
    start_phys_mem: 0x1000000,
    end_phys_mem: 0,
});
static PAGE_SIZE: u64 = 0x1000;

pub fn init() {
    {
        let mut vmm_allocator = VMM_ALLOCATOR.lock();
        vmm_allocator.end_phys_mem = unsafe { CONFIG.high_phys_mem };
        if vmm_allocator.end_phys_mem < vmm_allocator.start_phys_mem {
            panic!("Failed in VMM init: not enough physical memory on the system");
        }
        println!("end_phys_mem: {:x}", vmm_allocator.end_phys_mem);
    }
    lazy_static::initialize(&IDENTITY_MAP);
    println!("Creating new address space...");
    let new_address_space = AddressSpace::new_with_identity();
    println!("Switching to new address space...");
    new_address_space.activate();
    println!("Running with a new address space!");
}

pub fn init_ap() {
    println!("called vmm init ap");
    let new_address_space = AddressSpace::new_with_identity();
    //new_address_space.create_mapping(0xfee00, 0xfee00);
    new_address_space.activate();
}

pub fn alloc() -> u64 {
    let mut vmm_allocator = VMM_ALLOCATOR.lock();
    let result: u64;
    if vmm_allocator.start_phys_mem != vmm_allocator.end_phys_mem {
        result = vmm_allocator.start_phys_mem;
        vmm_allocator.start_phys_mem += PAGE_SIZE;
    } else {
        panic!("Ran out of fresh frames to allocate");
        // TODO: Demand paging
        if vmm_allocator.next == 0 {
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
    println!("allocated frame 0x{:x}", result);
    result
}
