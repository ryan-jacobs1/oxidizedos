#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(box_syntax)]
#![reexport_test_harness_main = "test_main"]
#![test_runner(crate::test_runner)]

extern crate alloc;

use oxos::config::mb_info;
use oxos::kernel_init;
use oxos::machine;
use oxos::semaphore::Semaphore;
use oxos::thread;
use oxos::thread::TCBImpl;
use oxos::{print, println};

use alloc::sync::Arc;
use core::sync::atomic::{AtomicU32, Ordering};

fn test_runner(tests: &[&dyn Fn()]) {
    unimplemented!("test_runner not used so should never be called");
}

#[no_mangle]
pub extern "C" fn _start(mb_config: &mb_info, end: u64) -> ! {
    kernel_init(mb_config, end);
    semaphore_test();
}

pub fn semaphore_test() -> ! {
    println!("Running Semaphore test");
    let sem = Semaphore::new(1);
    println!("Created a semaphore");
    sem.up();
    println!("Called up on semaphore!");
    sem.down();
    println!("Called down on semaphore!");
    sem.down();
    println!("Called down on semaphore again!");
    machine::exit(machine::EXIT_QEMU_SUCCESS);
}
