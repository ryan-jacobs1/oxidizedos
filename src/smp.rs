use core::sync::atomic::AtomicPtr;
use core::sync::atomic::Ordering;
use crate::config::CONFIG;
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
        LAPIC = Some(SMP::new(CONFIG.local_apic));
    }
    init_ap();
}

pub fn init_ap() {
    unsafe {
        if let Some(ref lapic) = LAPIC {
            let x = &mut 0x1ff;
            //lapic.spurious.store(x, Ordering::SeqCst);
            core::ptr::write_volatile(0xfee000f0 as *mut u32, 0x1ff);
        }
        // Disable PIC
        machine::outb(0xa1, 0xff);
        machine::outb(0x21, 0xff);

        // Enable LAPIC
        let msr_val = machine::rdmsr(SMP::MSR);
        //println!("msr {:x}", msr_val);
        let to_write = msr_val | (SMP::ENABLE as u64);
        //println!("writing {:x}", to_write);
        machine::wrmsr(msr_val | (SMP::ENABLE as u64), SMP::MSR);
        //println!("reread msr {:x}", machine::rdmsr(SMP::MSR));
        
    }
}

pub fn me() -> u32 {
    unsafe {
        /*
        if let Some(ref lapic) = LAPIC {
            let result = lapic.id.load(Ordering::SeqCst);
            println!("result: {}", *result);
            (*(result) >> 24)
        }
        else {
            panic!("smp::me() failed");
        }
        */
        let result = core::ptr::read_volatile(0xfee00020 as *const u32);
        println!("result {}", result);
        result >> 24
    }
}

pub fn ipi(id: u32, mut num: u32) {
    let lapic = unsafe {
        match LAPIC {
            Some(ref x) => x,
            None => panic!("No LAPIC, unable to send IPI")
        }
    };
    unsafe {
    //println!("num 0x{:x}", num);
    let mut id_shifted = id << 24;
    //println!("id {} storing {:b}", id, id_shifted);
    core::ptr::write_volatile(0xfee00310 as *mut u32, id_shifted);
    //lapic.icr_high.store(&mut id_shifted as *mut u32, Ordering::SeqCst);
    //let x = lapic.icr_high.load(Ordering::SeqCst);
    //println!("stored {:b}", unsafe{*x});
    //println!("sending ipi {:b}", num);
    //lapic.icr_low.store(&mut num, Ordering::SeqCst);
    core::ptr::write_volatile(0xfee00300 as *mut u32, num);
    unsafe {
        //while (*(lapic.icr_low.load(Ordering::SeqCst)) & (1 << 12)) != 0 {}
    }

}
}