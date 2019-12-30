use crate::machine;
use crate::println;
use crate::Stack;
use crate::BoxedStack;
use alloc::boxed::Box;

use crate::smp;
use alloc::collections::VecDeque;
use crate::config::CONFIG;
use core::mem::MaybeUninit;
use spin::Mutex;
use core::borrow::BorrowMut;
use core::marker::{Send, Sync};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TCBInfo {
    stack_pointer: usize,
}

impl TCBInfo {
    pub fn new(stack_pointer: usize) -> TCBInfo {
        TCBInfo {
            stack_pointer: stack_pointer,
        }
    }
}

pub trait TCB: Send + Sync {
    fn get_info(&mut self) -> *mut TCBInfo;
    /* fn get_work(&mut self) -> ???; */
}

#[repr(C)]
pub struct TCBImpl<T: 'static + Fn() + Send + Sync> {
    tcb_info: TCBInfo,
    stack: Box<[u64]>,
    work: Option<Box<T>>,
}

impl<T: 'static + Fn() + Send + Sync> TCBImpl<T> {
    const NUM_CALLEE_SAVED: usize = 6;

    pub fn new(work: T) -> TCBImpl<T> {
        let mut stack: Box<[u64]> = box [0; 512];
        let end_of_stack = 511;
        stack[end_of_stack] = thread_entry_point as *const () as u64;
        let index: usize = end_of_stack - TCBImpl::<T>::NUM_CALLEE_SAVED - 1;
        stack[index] = 0; // Flags
        stack[index - 1] = 0; // CR2
        let stack_ptr = Box::into_raw(stack);
        let stack_ptr_as_usize = stack_ptr as *mut u64 as usize;
        stack = unsafe {Box::from_raw(stack_ptr)};
        let stack_ptr_start = stack_ptr_as_usize + ((index - 1) * core::mem::size_of::<usize>());
        let tcb_info = TCBInfo::new(stack_ptr_start);
        TCBImpl {
            tcb_info: tcb_info,
            stack: stack,
            work: Some(Box::new(work)),
        }
    }
}

impl<T: 'static + Fn() + Send + Sync> TCB for TCBImpl<T> {
    fn get_info(&mut self) -> *mut TCBInfo {
        &mut self.tcb_info as *mut TCBInfo
    }
}

pub fn context_switch_test() {
    let mut test1 = Box::new(TCBImpl::new(move || ()));
    let mut test2 = Box::new(TCBImpl::new(move || ()));
    unsafe {
        machine::context_switch(test1.get_info(), test2.get_info());
    }
}

#[no_mangle]
pub extern "C" fn thread_entry_point() -> ! {
    println!("Thread made it to entry point!");
    loop {}
}