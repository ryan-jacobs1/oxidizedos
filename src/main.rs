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

use u8250::U8250;
use config::mb_info;
use heap::{Heap, LockedHeap, Block};




static HELLO: &[u8] = b"Off to the races!\n";


#[global_allocator]
static mut ALLOCATOR: LockedHeap = LockedHeap::new();

pub fn main() {}

#[no_mangle]
pub extern "C" fn _start(mb_config: &mb_info, end: u64) -> ! {
    let mut uart = U8250 {};
    let hi = "Hello there!\n";
    uart.write_string(hi);
    write!(uart, "The numbers are {} and {}, {}\n", 42, 1.0 / 3.0, hi).unwrap();
    println!("ooooweee, we're using println, {} {} {}", 42, 1.0 / 3.0, hi);
    println!("Kernel End Address {:x}", end);
    config::init_pre_paging(mb_config);
    config::memory_map_init();
    vmm::init();
    config::init_post_paging(mb_config);
    //smp::init_bsp();
    for (i, &byte) in HELLO.iter().enumerate() {
        uart.put(byte as u8);
    }
    unsafe {
        ALLOCATOR.init(0x200000, 0x800000);
    }
    let heap_val = Box::new(41);
    println!("value on heap {}", heap_val);
    let mut stuff = vec::Vec::new();
    for i in 0..500 {
        stuff.push(i);
    }
    println!("{:?}", stuff);
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
