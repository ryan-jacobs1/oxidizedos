#![no_std]
#![no_main]
#![reexport_test_harness_main = "test_main"]


use oxos::kernel_init;
use oxos::config::mb_info;
use oxos::{print, println};

#[no_mangle]
pub extern "C" fn _start(mb_config: &mb_info, end: u64) -> ! {
    kernel_init(mb_config, end);
    #[cfg(test)]
    test_main();
}

#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
fn test_runner(tests: &[&dyn Fn()]) {
    println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
}


#[test_case]
fn trivial_assertion() {
    print!("trivial assertion... ");
    assert_eq!(1, 1);
    println!("[ok]");
}