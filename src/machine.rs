use crate::println;
use crate::thread::TCBInfo;

pub static EXIT_QEMU_SUCCESS: u32 = 5;
pub static EXIT_QEMU_FAILURE: u32 = 3;

extern "C" {
    pub fn outb(port: u32, val: u32);
    pub fn outw(port: u32, val: u32);
    pub fn outl(port: u32, val: u32);
    pub fn inb(port: u32) -> u8;
    pub fn inw(port: u32) -> u16;
    pub fn inl(port: u32) -> u32;
    pub fn hlt();
    pub fn load_cr3(pml4: u64);
    pub fn rdmsr(msr: u32) -> u64;
    pub fn wrmsr(val: u64, msr: u32);
    pub fn lidt(idt: u64);
    pub fn spurious_handler();
    pub fn _apit_handler();
    pub fn software_int();
    pub fn ap_entry();
    pub fn context_switch(current: *mut TCBInfo, next: *mut TCBInfo);
    pub fn cli();
    pub fn sti();
    pub fn get_flags() -> u64;
    pub fn get_rsp() -> u64;
}

/// Disables interrupts, and returns whether or not interrupts were enabled
/// The result of this function should be passed to its companion function, enable
pub fn disable() -> bool {
    unsafe {
        let flags = get_flags();
        cli();
        (flags & (1 << 9)) > 0
    }
}

/// Enables interrupts only if was_enable is true
/// This provides composability of enable/disable
pub fn enable(was_enabled: bool) {
    unsafe {
        if was_enabled {
            sti();
        }
    }
}

pub fn are_interrupts_enabled() -> bool {
    let flags = unsafe { get_flags() };
    (flags & (1 << 9)) > 0
}

pub fn exit(exit_code: u32) -> ! {
    unsafe {
        outl(0xf4, exit_code);
    }
    loop {}
}
