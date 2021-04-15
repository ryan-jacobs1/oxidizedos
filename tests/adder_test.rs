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
    println!("Running adder test");
    adder_test();
}

pub fn adder_test() -> ! {
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
    let result = counter.load(Ordering::SeqCst);
    assert_eq!(result, 100);
    println!("Adder Test PASSED");
    machine::exit(machine::EXIT_QEMU_SUCCESS);
}
