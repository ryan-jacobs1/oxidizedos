use crate::smp;
use crate::machine;
use crate::println;
use core::sync::atomic::Ordering;

pub static PIT_FREQ: u32 = 1193182;

pub fn calibrate(hz: u32) {
    let lapic = unsafe {
        match &smp::LAPIC {
            Some(lapic) => lapic,
            None => panic!("No LAPIC available")
        }
    };
    let d = PIT_FREQ / 20;
    let mut initial = 0xffffffff;
    unsafe {
        core::ptr::write_volatile(lapic.apit_lvl_timer, 0x00010000);
        core::ptr::write_volatile(lapic.apit_divide, 0x00010000);
        core::ptr::write_volatile(lapic.apit_initial_count, initial);
        machine::outb(0x61, 1);
        machine::outb(0x43, 0b10110110);
        machine::outb(0x42, d);
        machine::outb(0x42, d >> 8);
    }
    let mut last = unsafe {machine::inb(0x61) & 0x20};
    let mut changes = 0;

    while changes < 40 {
        let t = unsafe {machine::inb(0x61) & 0x20};
        if t != last {
            changes += 1;
            last = t;
        }
    }
    let current_count = unsafe {core::ptr::read_volatile(lapic.apit_current_count)};
    println!("current count {:x}", current_count);
    let diff = initial - current_count;
    unsafe {
        machine::outb(0x61, 0);
    }
    println!("diff {:x}", diff);
    println!("APIT running at {} hz", diff);
    let apit_counter = diff / hz;
    println!("apit counter: {}", apit_counter);
}