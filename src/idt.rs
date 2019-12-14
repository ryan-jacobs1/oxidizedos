use crate::machine;

pub static mut IDT: IDT = IDT::new();
pub static mut IDTRecord: IDTRecord = IDTRecord {limit: 0, idt_addr: 0};


#[repr(C, align(4096))]
pub struct IDT {
    entries: [IDTEntryWrapper; 256],
}

impl IDT {
    pub const fn new() -> IDT {
        IDT {entries: [IDTEntryWrapper::new(); 256]}
    }
}

#[repr(C, packed)]
pub struct IDTRecord {
    limit: u16,
    idt_addr: u64
}

pub fn init() {
    let limit: u16 = core::mem::size_of::<IDT>() as u16 - 1;
    let idt_addr: u64 = unsafe {&IDT as *const IDT as u64};
    let idt_record_ptr: u64 = unsafe {&IDTRecord as *const IDTRecord as u64};
    unsafe {
        IDTRecord.limit = limit;
        IDTRecord.idt_addr = idt_addr;
        machine::lidt(idt_record_ptr);
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
struct IDTEntryWrapper {
    entry: IDTEntry<[u8; 16]>,
}

impl IDTEntryWrapper {
    pub const fn new() -> IDTEntryWrapper {
        IDTEntryWrapper {entry: IDTEntry([0; 16])}
    }
}

bitfield! {
    #[derive(Clone, Copy)]
    #[repr(transparent)]
    struct IDTEntry([u8]);
    u64;
    offset_low_bits, set_offset_low_bits: 15, 0;
    selector, set_selector: 31, 16;
    zero, _: 39, 32;
    type_and_attributes, set_type_and_attributes: 47, 40;
    offset_middle_bits, set_offset_middle_bits: 63, 48;
    offset_high_bits, set_offset_high_bits: 95, 64;
}