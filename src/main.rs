#![no_std]
#![no_main]


mod u8250;
mod machine;

use core::panic::PanicInfo;
use core::fmt::Write;  

use u8250::U8250;


static HELLO: &[u8] = b"Off to the races!\n";

pub fn main() {}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut uart = U8250{};
    let hi = "Hello there!\n";
    uart.write_string(hi);
    write!(uart, "The numbers are {} and {}, {}\n", 42, 1.0/3.0, hi).unwrap();
    println!("ooooweee, we're using println, {} {} {}", 42, 1.0/3.0, hi);
    for (i, &byte) in HELLO.iter().enumerate() {
        uart.put(byte as u8);
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
