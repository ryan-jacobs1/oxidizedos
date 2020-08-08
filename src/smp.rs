use crate::config::CONFIG;
use crate::machine;
use crate::println;
use core::sync::atomic::AtomicPtr;
use core::sync::atomic::Ordering;

pub static mut LAPIC: Option<SMP> = None;

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
    init_ap();
}

pub fn init_ap() {
    unsafe {
        if let Some(ref lapic) = LAPIC {
            let x = &mut 0x1ff;
            core::ptr::write_volatile(0xfee000f0 as *mut u32, 0x1ff);
        }
        // Disable PIC
        machine::outb(0xa1, 0xff);
        machine::outb(0x21, 0xff);

        // Enable LAPIC
        let msr_val = machine::rdmsr(SMP::MSR);
        let to_write = msr_val | (SMP::ENABLE as u64);

        machine::wrmsr(msr_val | (SMP::ENABLE as u64), SMP::MSR);
    }
}

pub fn me() -> usize {
    unsafe {
        let result = core::ptr::read_volatile(0xfee00020 as *const u32);
        (result >> 24) as usize
    }
}

pub fn ipi(id: u32, mut num: u32) {
    let lapic = unsafe {
        match LAPIC {
            Some(ref x) => x,
            None => panic!("No LAPIC, unable to send IPI"),
        }
    };
    unsafe {
        let mut id_shifted = id << 24;

        core::ptr::write_volatile(0xfee00310 as *mut u32, id_shifted);

        core::ptr::write_volatile(0xfee00300 as *mut u32, num);
    }
}
