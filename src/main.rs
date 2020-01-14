#![no_std]
#![no_main]

use oxos::kernel_init;
use oxos::config::mb_info;


#[no_mangle]
pub extern "C" fn _start(mb_config: &mb_info, end: u64) -> ! {
    kernel_init(mb_config, end);
}