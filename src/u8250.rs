extern crate spin;

use crate::machine;
use core::fmt;
use spin::Mutex;

pub struct U8250 {}

static mut WRITER: Mutex<U8250> = Mutex::new(U8250{});

impl U8250 {
    const COM_PORT: u32 = 0x3F8;
    const COM_READY: u32 = 0x3F8 + 5;

    pub fn put(&self, c: u8) {
        unsafe {
            while machine::inb(U8250::COM_READY) & 0x20 == 0 {}
            machine::outb(U8250::COM_PORT, c as u32);
        }
    }

    pub fn write_string(&self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' => self.put(byte),
                _ => self.put(0xfe),
            }
        }
    }
}

impl fmt::Write for U8250 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::u8250::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    unsafe {
        WRITER.lock().write_fmt(args).unwrap();
    }
}
