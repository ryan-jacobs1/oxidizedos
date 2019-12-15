use core::sync::atomic::AtomicPtr;
use core::sync::atomic::Ordering;
use crate::config::config;
use crate::machine;
use crate::println;

pub static mut LAPIC: Option<SMP> = None;

pub struct SMP {
    id: AtomicPtr<u32>,
    spurious: AtomicPtr<u32>,
    icr_low: AtomicPtr<u32>,
    icr_high: AtomicPtr<u32>,
    pub eoi_reg: AtomicPtr<u32>,
    pub apit_lvl_timer: AtomicPtr<u32>,
    pub apit_initial_count: AtomicPtr<u32>,
    pub apit_current_count: AtomicPtr<u32>,
    pub apit_divide: AtomicPtr<u32>,
}

impl SMP {
    const ENABLE: u32 = 1 << 11;
    const ISBSP: u32 = 1 << 8;
    const MSR: u32 = 0x1B;

    pub fn new(lapic_base: u32) -> SMP {
        SMP {
            id: AtomicPtr::new((lapic_base + 0x20) as *mut u32),
            eoi_reg: AtomicPtr::new((lapic_base + 0xb0) as *mut u32),
            spurious: AtomicPtr::new((lapic_base + 0xf0) as *mut u32),
            icr_low: AtomicPtr::new((lapic_base + 0x300) as *mut u32),
            icr_high: AtomicPtr::new((lapic_base + 0x310) as *mut u32),
            apit_lvl_timer: AtomicPtr::new((lapic_base + 0x320) as *mut u32),
            apit_initial_count: AtomicPtr::new((lapic_base + 0x380) as *mut u32),
            apit_current_count: AtomicPtr::new((lapic_base + 0x390) as *mut u32),
            apit_divide: AtomicPtr::new((lapic_base + 0x3e0) as *mut u32),
        }
    }
}

pub fn init_bsp() {
    unsafe {
        LAPIC = Some(SMP::new(config.local_apic));
    }
    init_ap();
}

pub fn init_ap() {
    unsafe {
        // Disable PIC
        machine::outb(0x21, 0xff);
        machine::outb(0xa1, 0xff);

        // Enable LAPIC
        let msr_val = machine::rdmsr(SMP::MSR);
        machine::wrmsr(msr_val | (SMP::ENABLE as u64), SMP::MSR);
        if let Some(ref lapic) = LAPIC {
            let x = &mut 0x1ff;
            lapic.spurious.store(x, Ordering::SeqCst);
            let y = lapic.spurious.load(Ordering::SeqCst);
            unsafe {
                println!("spurious reg 0x{:x}", *y);
            }
        }
    }
}

pub fn me() -> u32 {
    unsafe {
        if let Some(ref lapic) = LAPIC {
            let result = lapic.id.load(Ordering::SeqCst);
            (*(result) >> 24)
            /*
            println!("attempting to read id");
            let id_reg = (0xfee00000 as u32 + 0x20) as *const u32;
            let result = unsafe {core::ptr::read_volatile(id_reg)};
            return result;
            */
        }
        else {
            panic!("smp::me() failed");
        }
    }
}