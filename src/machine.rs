extern "C" {
    pub fn outb(port: u32, val: u32);
    pub fn outw(port: u32, val: u32);
    pub fn inb(port: u32) -> u8;
    pub fn hlt();
    pub fn load_cr3(pml4: u64);
    pub fn rdmsr(msr: u32) -> u64;
    pub fn wrmsr(val: u64, msr: u32);
}
