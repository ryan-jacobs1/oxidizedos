#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![reexport_test_harness_main = "test_main"]
#![test_runner(crate::test_runner)]


use oxos::machine;
use oxos::{kernel_init, adder_test};
use oxos::config::mb_info;
use oxos::{print, println, println_vga};


#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start(mb_config: &mb_info, end: u64) -> ! {
    kernel_init(mb_config, end);
    test_main();
    machine::exit(machine::EXIT_QEMU_SUCCESS);
}

#[cfg(test)]
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

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn _start(mb_config: &mb_info, end: u64) -> ! {
    kernel_init(mb_config, end);
    adder_test();
    unsafe {machine::cli()};
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    println_vga!("Hello World{}", "!");
    loop {}
    machine::exit(machine::EXIT_QEMU_SUCCESS);
}



