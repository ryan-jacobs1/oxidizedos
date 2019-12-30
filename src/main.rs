#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]
#![feature(box_syntax)]
#![feature(trait_alias)]

mod machine;
mod u8250;
mod config;
mod heap;
mod vmm;
mod smp;
mod idt;
mod thread;
mod spinlock;
mod ismutex;
mod timer;

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
use linked_list_allocator::LockedHeap;
use thread::TCBImpl;
use alloc::sync::Arc;



static HELLO: &[u8] = b"Off to the races!\n";



#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();


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



#[no_mangle]
pub extern "C" fn _ap_start() -> ! {
    loop {}
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
    config::init(mb_config);
    config::memory_map_init();
    vmm::init();
    idt::init();
    idt::interrupt(0xff, machine::spurious_handler);
    smp::init_bsp();
    println!("smp::me(): {}", smp::me());
    unsafe {
        ALLOCATOR.lock().init(0x200000, 0x800000);
    }
    thread::context_switch_test();
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
