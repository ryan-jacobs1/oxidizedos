use crate::thread::TCBInfo;

extern "C" {
    pub fn outb(port: u32, val: u32);
    pub fn outw(port: u32, val: u32);
    pub fn inb(port: u32) -> u8;
    pub fn hlt();
    pub fn load_cr3(pml4: u64);
    pub fn rdmsr(msr: u32) -> u64;
    pub fn wrmsr(val: u64, msr: u32);
    pub fn lidt(idt: u64);
    pub fn spurious_handler();
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
        cli();
        let flags = get_flags();
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
