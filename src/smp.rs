use crate::{apic::Apic, config::CONFIG};
use crate::machine;
use crate::println;
use core::sync::atomic::Ordering;
use core::{ops::Range, sync::atomic::AtomicPtr};
use x86_64::instructions::port::{self, Port};


pub static mut LAPIC: Option<SMP> = None;
pub static mut APIC: Option<Apic> = None;

pub struct SMP {
    id: *mut u32,
    spurious: *mut u32,
    icr_low: *mut u32,
    icr_high: *mut u32,
    pub eoi_reg: *mut u32,
    pub apit_lvt_timer: *mut u32,
    pub apit_initial_count: *mut u32,
    pub apit_current_count: *mut u32,
    pub apit_divide: *mut u32,
}

impl SMP {
    const ENABLE: u32 = 1 << 11;
    const ISBSP: u32 = 1 << 8;
    const MSR: u32 = 0x1B;

    pub fn new(lapic_base: u32) -> SMP {
        SMP {
            id: (lapic_base + 0x20) as *mut u32,
            eoi_reg: (lapic_base + 0xb0) as *mut u32,
            spurious: (lapic_base + 0xf0) as *mut u32,
            icr_low: (lapic_base + 0x300) as *mut u32,
            icr_high: (lapic_base + 0x310) as *mut u32,
            apit_lvt_timer: (lapic_base + 0x320) as *mut u32,
            apit_initial_count: (lapic_base + 0x380) as *mut u32,
            apit_current_count: (lapic_base + 0x390) as *mut u32,
            apit_divide: (lapic_base + 0x3e0) as *mut u32,
        }
    }
}

pub fn init_bsp() {
    unsafe {
        LAPIC = Some(SMP::new(CONFIG.local_apic));
    }
}


pub fn me() -> usize {
    unsafe {
        let result = core::ptr::read_volatile(0xfee00020 as *const u32);
        (result >> 24) as usize
    }
}
