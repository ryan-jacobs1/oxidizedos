use alloc::boxed::Box;
use crate::Stack;
use crate::println;
use crate::machine;

#[repr(C)]
struct TCB<T> where T: Fn() {
    tcb_info: TCBInfo,
    stack: Box<Stack>,
    work: T,
}

#[repr(C)]
pub struct TCBInfo {
    stack_pointer: usize,
    leave_me_alone: bool
}

impl TCBInfo {
    pub fn new(stack_pointer: usize) -> TCBInfo {
        TCBInfo {stack_pointer: stack_pointer, leave_me_alone: false}
    }
}

impl<T: Fn()> TCB<T> {
    const num_callee_saved: usize = 6;

    pub fn new(work: T) -> TCB<T> {
        let mut stack = Box::new(Stack::new());
        let end_of_stack = 511;
        stack.stack[end_of_stack] = thread_entry_point as *const () as u64;
        let index: usize = end_of_stack - TCB::<T>::num_callee_saved - 1;
        stack.stack[index] = 0; // Flags
        stack.stack[index - 1] = 0; // CR2
        let stack_ptr = Box::into_raw(stack);
        let stack_ptr_as_usize = stack_ptr as usize;
        println!("loaded return at 0x{:x}", stack_ptr_as_usize + (end_of_stack * core::mem::size_of::<usize>()));
        let x = stack_ptr_as_usize + ((index - 1) * core::mem::size_of::<usize>());
        println!("initial rsp 0x{:x}", x);
        let tcb_info = TCBInfo::new(x);
        stack = unsafe {Box::from_raw(stack_ptr)};
        TCB {tcb_info: tcb_info, stack: stack, work: work}
    }

    pub fn get_info(&mut self) -> *mut TCBInfo {
        &mut self.tcb_info as *mut TCBInfo
    }
}

#[no_mangle]
pub extern "C" fn thread_entry_point() -> ! {
    println!("thread arrived at entry point");
    //work();
    println!("thread finished work");
    loop {}
}

pub fn r#yield() {
    let mut test1 = Box::new(TCB::new(|| ()));
    let mut test2 = Box::new(TCB::new(|| ()));
    println!("attempting to context switch");
    let x = test2.get_info();
    unsafe {
        println!("switching to rsp {:x}", unsafe {*(x as *mut usize)});
    }
    unsafe {
        machine::context_switch(test1.get_info(), test2.get_info());
    }
}

