use alloc::boxed::Box;
use crate::Stack;
use crate::println;

#[repr(C)]
struct TCB<T> where T: Fn() {
    tcb_info: TCBInfo,
    stack: Box<Stack>,
    work: T,
}

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
        let index: usize = end_of_stack - TCB::<T>::num_callee_saved;
        stack.stack[index] = 0; // Flags
        stack.stack[index - 1] = 0; // CR2
        stack.stack[index - 2] = thread_entry_point as *const () as u64;
        let stack_ptr = Box::into_raw(stack);
        let tcb_info = TCBInfo::new(stack_ptr as usize + (end_of_stack * core::mem::size_of::<usize>()));
        stack = unsafe {Box::from_raw(stack_ptr)};
        TCB {tcb_info: tcb_info, stack: stack, work: work}
    }
}

#[no_mangle]
pub extern "C" fn thread_entry_point() {
    println!("thread arrived at entry point");
    //work();
    println!("thread finished work");
}

