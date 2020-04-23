#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(box_syntax)]
#![reexport_test_harness_main = "test_main"]
#![test_runner(crate::test_runner)]

extern crate alloc;

use core::str;
use core::slice;

use oxos::machine;
use oxos::ide;
use oxos::ide::{IDEImpl, IDE};
use oxos::sfs;
use oxos::{kernel_init};
use oxos::config::mb_info;
use oxos::{print, println, println_vga};

use alloc::{boxed::Box, vec, vec::Vec};



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
    let ide = ide::IDEImpl::new(1);
    let mut buf: Box<[u32]> = box [0; 512 / 4];
    println_vga!("Reading from file...");
    //ide.read_sector(0, &mut buf);
    ide.read(0, &mut buf, 406);
    let mut buf_u8 = box u32_as_u8_mut(&mut buf);
    let x = core::str::from_utf8(&buf_u8);
    println_vga!("{}", x.expect("uh oh"));
    println_vga!("File read complete!");

    let mut s = sfs::SFS::new(1);
    s.print_super_block();
    s.create_file("test", 5);
    
    loop {}
    machine::exit(machine::EXIT_QEMU_SUCCESS);
}

fn u32_as_u8_mut<'a>(src: &'a mut [u32]) -> &'a mut [u8] {
    let dst = unsafe {
        core::slice::from_raw_parts_mut(src.as_mut_ptr() as *mut u8, src.len() * 4)
    };
    dst
}
