#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]
#![feature(box_syntax)]
#![feature(trait_alias)]
#![feature(alloc, allocator_api)]
#![feature(const_fn)]

mod machine;
mod u8250;
mod config;
mod heap;
mod vmm;
mod smp;
mod idt;
mod thread;
mod semaphore;
mod spinlock;
mod ismutex;
mod timer;
mod linked_list_allocator_2;

#[macro_use]
extern crate bitfield;
#[macro_use]
extern crate lazy_static;
extern crate alloc;
extern crate linked_list_allocator;

use alloc::{boxed::Box, vec, vec::Vec};

use core::fmt::Write;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicUsize, Ordering, AtomicU32};

use u8250::U8250;
use config::mb_info;
use config::CONFIG;
use heap::{Heap, Block};
//use heap::LockedHeap;
use linked_list_allocator::LockedHeap;
use linked_list_allocator_2::isheap::ISHeap;
use thread::TCBImpl;
use alloc::sync::Arc;



static HELLO: &[u8] = b"Off to the races!\n";

/*
#[global_allocator]
static mut ALLOCATOR: LockedHeap = LockedHeap::new();
*/

/*
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();
*/

#[global_allocator]
static ALLOCATOR: ISHeap = ISHeap::empty();

static mut STACK: Stack = Stack::new();
static APSTACK: AtomicUsize = AtomicUsize::new(0);
static CORES_ACTIVE: AtomicU32 = AtomicU32::new(0);

#[repr(C, align(4096))]
#[derive(Copy, Clone)]
pub struct Stack {
    pub stack: [u64; 512],
}

impl Stack {
    pub const fn new() -> Stack {
        Stack {stack: [0; 512]}
    }
    pub fn boxed_new() -> Box<Stack> {
        box Stack {stack: [0; 512]}
    }
}

pub struct BoxedStack {
    pub stack: Option<Box<[u64]>>
}

impl BoxedStack {
    pub fn new() -> BoxedStack {
        BoxedStack {stack: Some(box [0; 512])}
    }
}


pub fn main() {}

#[no_mangle]
pub extern "C" fn _ap_start() -> ! {
    unsafe {
        println!("rsp is {:x}", machine::get_rsp());
    }
    vmm::init_ap();
    idt::init_ap();
    smp::init_ap();
    timer::init();
    let me = smp::me();
    println!("AP {} reached _ap_start", me);
    CORES_ACTIVE.fetch_add(1, Ordering::SeqCst);
    let num_cores = unsafe {CONFIG.total_procs};
    while CORES_ACTIVE.load(Ordering::SeqCst) < num_cores {}
    loop {
        thread::stop();
        //panic!("thread::stop returned");
    }
}

#[no_mangle]
pub extern "C" fn pick_stack() -> usize {
    unsafe {(&STACK as *const Stack as usize) + (4096 - 8)}
}

#[no_mangle]
pub extern "C" fn ap_pick_stack() -> usize {
    let stack = APSTACK.load(Ordering::SeqCst) + (4096 - 8);
    println!("picked rsp 0x{:x}", stack);
    stack
}

#[no_mangle]
pub extern "C" fn _start(mb_config: &mb_info, end: u64) -> ! {
    CORES_ACTIVE.fetch_add(1, Ordering::SeqCst);
    println!("the kernel stack is at {:x}", unsafe {&STACK as *const Stack as usize});
    let mut uart = U8250 {};
    let hi = "Hello there!\n";
    uart.write_string(hi);
    write!(uart, "The numbers are {} and {}, {}\n", 42, 1.0 / 3.0, hi).unwrap();
    //println!("ooooweee, we're using println, {} {} {}", 42, 1.0 / 3.0, hi);
    println!("Kernel End Address {:x}", end);
    config::init(mb_config);
    config::memory_map_init();
    vmm::init();
    idt::init();
    idt::interrupt(0xff, machine::spurious_handler);
    smp::init_bsp();
    println!("smp::me(): {}", smp::me());
    unsafe {
        //ALLOCATOR.init(0x200000, 0x800000);
        ALLOCATOR.lock().init(0x200000, 0x800000);
    }    
    thread::init();
    timer::calibrate(1000);
    timer::init();
    
    let reset_eip = machine::ap_entry as *const () as u32;
    println!("reset eip 0x{:x}", reset_eip);
    println!("Booting up other cores...");
    let num_cores = unsafe {CONFIG.total_procs};
    
    for i in 1..num_cores {
        // First allocate a kernel stack
        // TODO: Put info about bootstrap stacks in a Bootstrap TCB
        APSTACK.store(vmm::alloc() as usize, Ordering::SeqCst);
        smp::ipi(i, 0x4500);
        smp::ipi(i, 0x4600 | (reset_eip >> 12));
        while (CORES_ACTIVE.load(Ordering::SeqCst) <= i) {}
    }
    println!("done with ipis");
    unsafe {
        machine::sti();
    }
    println!("scheduling threads");
    let counter = Arc::new(AtomicU32::new(0));
    for i in 0..100 {
        
        let c = Arc::clone(&counter);
        let x = TCBImpl::new(box move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
        thread::schedule(box x);
    }
    println!("scheduled all threads");
    while counter.load(Ordering::SeqCst) < 100 {}
    println!("counter: {}", counter.load(Ordering::SeqCst));
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    print!("Panic: ");
    if let Some(s) = _info.message() {
        u8250::_print(*s);
    }
    loop {}
}

#[alloc_error_handler]
fn alloc_panic(layout: alloc::alloc::Layout) -> ! {
    panic!("Core {}: Failure in alloc\n", smp::me());
}
