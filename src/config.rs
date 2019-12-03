use crate::println;


#[repr(C)]
pub struct mb_info {
    mb_type: u32,
    size: u32,
}

impl mb_info {
    pub fn print(&self) {
        println!("type {} size {}", self.mb_type, self.size);
    }
}