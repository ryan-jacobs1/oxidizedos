#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]

mod machine;
mod u8250;
mod config;
mod heap;
mod vmm;
mod smp;
mod idt;

#[macro_use]
extern crate bitfield;
#[macro_use]
extern crate lazy_static;
extern crate alloc;

use alloc::{boxed::Box, vec, vec::Vec};

use core::fmt::Write;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicUsize, Ordering};

use u8250::U8250;
use config::mb_info;
use config::config;
use heap::{Heap, LockedHeap, Block};




static HELLO: &[u8] = b"Off to the races!\n";


#[global_allocator]
static mut ALLOCATOR: LockedHeap = LockedHeap::new();


static mut STACK: Stack = Stack::new();
static APSTACK: AtomicUsize = AtomicUsize::new(0);

#[repr(align(4096))]
#[derive(Copy, Clone)]
struct Stack {
    stack: [u8; 4096],
}

impl Stack {
    pub const fn new() -> Stack {
        Stack {stack: [0; 4096]}
    }
}


pub fn main() {}

#[no_mangle]
pub extern "C" fn _ap_start() -> ! {
    println!("AP reached _ap_start");
    loop {}
}

#[no_mangle]
pub extern "C" fn pick_stack() -> usize {
    unsafe {(&STACK as *const Stack as usize) + (4096 - 8)}
}

#[no_mangle]
pub extern "C" fn ap_pick_stack() -> usize {
    let stack = APSTACK.load(Ordering::SeqCst);
    stack
}

#[no_mangle]
pub extern "C" fn _start(mb_config: &mb_info, end: u64) -> ! {
    println!("the kernel stack is at {:x}", unsafe {&STACK as *const Stack as usize});
    let mut uart = U8250 {};
    let hi = "Hello there!\n";
    uart.write_string(hi);
    write!(uart, "The numbers are {} and {}, {}\n", 42, 1.0 / 3.0, hi).unwrap();
    println!("ooooweee, we're using println, {} {} {}", 42, 1.0 / 3.0, hi);
    println!("Kernel End Address {:x}", end);
    config::init(mb_config);
    config::memory_map_init();
    vmm::init();
    idt::init();
    idt::interrupt(0xff, machine::spurious_handler);
    smp::init_bsp();
    println!("smp::me(): {}", smp::me());
    let resetEIP = machine::ap_entry as *const () as u32;
    println!("reset eip 0x{:x}", resetEIP);
    println!("Booting up other cores...");
    let num_cores = unsafe {config.total_procs};
    for i in 1..num_cores {
        // First allocate a kernel stack
        // TODO: Put info about bootstrap stacks in a Bootstrap TCB
        APSTACK.store(vmm::alloc() as usize, Ordering::SeqCst);
        smp::ipi(i, 0x4500);
        smp::ipi(i, (0x4600 | (resetEIP >> 12)));
    }
    for (i, &byte) in HELLO.iter().enumerate() {
        uart.put(byte as u8);
    }
    unsafe {
        ALLOCATOR.init(0x200000, 0x800000);
    }
    let heap_val = Box::new(41);
    println!("value on heap {}", heap_val);
    /*
    let mut stuff = vec::Vec::new();
    for i in 0..499 {
        stuff.push(i);
    }
    println!("{:?}", stuff);
    */
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
    panic!("Failure in alloc");
}
