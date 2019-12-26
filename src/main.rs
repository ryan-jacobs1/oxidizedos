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
mod semaphore;
mod spinlock;

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



static HELLO: &[u8] = b"Off to the races!\n";

/*
#[global_allocator]
static mut ALLOCATOR: LockedHeap = LockedHeap::new();
*/

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
}

pub fn main() {}

#[no_mangle]
pub extern "C" fn _ap_start() -> ! {
    unsafe {
        println!("rsp is {:x}", machine::get_rsp());
    }
    vmm::init_ap();
    //idt::init_ap();
    smp::init_ap();
    let me = smp::me();
    println!("AP {} reached _ap_start", me);
    CORES_ACTIVE.fetch_add(1, Ordering::SeqCst);
    if (me == 1) {
        /*
        let x = TCBImpl::new(|| {println!("yay {}!", smp::me());});
        thread::schedule(box x);
        thread::surrender();
        */
    }
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
    unsafe {
        //ALLOCATOR.init(0x200000, 0x800000);
        ALLOCATOR.lock().init(0x200000, 0x800000);
    }    
    thread::init();
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

    let x = box TCBImpl::new(|| {
        println!("me: {}", smp::me());
    });
    thread::schedule(x);
    thread::surrender();
    /*
    for (i, &byte) in HELLO.iter().enumerate() {
        uart.put(byte as u8);
    }
    */
    println!("main thread doing some allocation");
    let heap_val = Box::<u8>::new(41);
    println!("value on heap {}", heap_val);
<<<<<<< HEAD
    let ptr = Box::into_raw(heap_val);
    println!("location on heap {:x}", ptr as usize);
    let aligned_heap_val = Box::<u64>::new(17);
    let aligned_ptr = Box::into_raw(aligned_heap_val);
    println!("aligned? val at {:x}", aligned_ptr as usize);
=======

    unsafe {
        let mut box2 = Box::<u16>::new(16);
        let box2_ptr = Box::into_raw(box2);
        println!("Is u16 aligned?: {}", match box2_ptr as usize % core::mem::size_of::<u16>() {
            0 => "TRUE",
            _ => "FALSE",
        });
        box2 = Box::from_raw(box2_ptr);
        let mut box3 = Box::<u64>::new(32);
        let box3_ptr = Box::into_raw(box3);
        println!("Is u32 aligned?: {}", match box3_ptr as usize % core::mem::size_of::<u64>() {
            0 => "TRUE",
            _ => "FALSE",
        });
        box3 = Box::from_raw(box3_ptr);
        let mut box4 = Box::<u64>::new(64);
        let box4_ptr = Box::into_raw(box4);
        println!("Is u64 aligned?: {}", match box4_ptr as usize % core::mem::size_of::<u64>() {
            0 => "TRUE",
            _ => "FALSE",
        });
        box4 = Box::from_raw(box4_ptr);
        let mut box5 = Box::<u64>::new(64);
        let box5_ptr = Box::into_raw(box5);
        println!("Is u64 aligned?: {}", match box5_ptr as usize % core::mem::size_of::<u64>() {
            0 => "TRUE",
            _ => "FALSE",
        });
        box5 = Box::from_raw(box5_ptr);
        let mut box6 = Box::<u64>::new(64);
        let box6_ptr = Box::into_raw(box6);
        println!("Is u64 aligned?: {}", match box6_ptr as usize % core::mem::size_of::<u64>() {
            0 => "TRUE",
            _ => "FALSE",
        });
        box6 = Box::from_raw(box6_ptr);
    }
>>>>>>> master
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
