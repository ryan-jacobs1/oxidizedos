extern "C" {
    pub fn outb(port: u32, val: u32);
    pub fn outw(port: u32, val: u32);
    pub fn inb(port: u32) -> u8;
    pub fn hlt();
}
