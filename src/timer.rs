use crate::idt;
use crate::machine;
use crate::println;
use crate::smp;
use crate::thread;
use core::sync::atomic::Ordering;

pub static PIT_FREQ: u32 = 1193182;
pub static APIT_vector: usize = 40;
pub static mut APIT_counter: Option<u32> = None;

pub fn calibrate(hz: u32) {
    println!("Calibrating APIT...");
    let lapic = unsafe {
        match &smp::LAPIC {
            Some(lapic) => lapic,
            None => panic!("No LAPIC available"),
        }
    };
    let d = PIT_FREQ / 20;
    let mut initial = 0xffffffff;
    unsafe {
        core::ptr::write_volatile(lapic.apit_lvt_timer, 0x00010000);
        core::ptr::write_volatile(lapic.apit_divide, 0x00010000);
        core::ptr::write_volatile(lapic.apit_initial_count, initial);
        machine::outb(0x61, 1);
        machine::outb(0x43, 0b10110110);
        machine::outb(0x42, d);
        machine::outb(0x42, d >> 8);
    }
    let mut last = unsafe { machine::inb(0x61) & 0x20 };
    let mut changes = 0;

    while changes < 40 {
        let t = unsafe { machine::inb(0x61) & 0x20 };
        if t != last {
            changes += 1;
            last = t;
        }
    }
    let current_count = unsafe { core::ptr::read_volatile(lapic.apit_current_count) };
    println!("current count {:x}", current_count);
    let diff = initial - current_count;
    unsafe {
        machine::outb(0x61, 0);
    }
    println!("diff {:x}", diff);
    println!("APIT running at {} hz", diff);
    let counter = diff / hz;
    println!("apit counter: {}", counter);
    unsafe {
        APIT_counter = Some(counter);
    }
    idt::interrupt(APIT_vector, machine::_apit_handler);
}

pub fn init() {
    let lapic = unsafe {
        match &smp::LAPIC {
            Some(lapic) => lapic,
            None => panic!("No LAPIC available"),
        }
    };
    let counter = unsafe {
        match APIT_counter {
            Some(counter) => counter,
            None => panic!("APIT not initialized"),
        }
    };
    unsafe {
        core::ptr::write_volatile(lapic.apit_divide, 0x0000000B);
        core::ptr::write_volatile(
            lapic.apit_lvt_timer,
            (1 << 17) | (0 << 16) | (APIT_vector as u32),
        );
        core::ptr::write_volatile(lapic.apit_initial_count, counter);
    }
}

#[no_mangle]
pub extern "C" fn apit_handler() {
    //println!("timer interrupt");
    let lapic = unsafe {
        match &smp::LAPIC {
            Some(lapic) => lapic,
            None => panic!("No LAPIC available"),
        }
    };
    unsafe {
        core::ptr::write_volatile(lapic.eoi_reg, 0);
    }
    thread::surrender();
}
