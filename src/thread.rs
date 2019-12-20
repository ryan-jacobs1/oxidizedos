use crate::machine;
use crate::println;
use crate::Stack;
use alloc::boxed::Box;

use crate::smp;
use alloc::collections::VecDeque;
use crate::config::CONFIG;
use core::mem::MaybeUninit;
use spin::Mutex;
use core::borrow::BorrowMut;

lazy_static! {
    pub static ref READY: Mutex<VecDeque<Box<dyn TCB>>> = spin::Mutex::new(VecDeque::new());
    pub static ref ACTIVE: Mutex<[Box<dyn TCB>; 16]> = {
        let mut active: [MaybeUninit<Box<dyn TCB>>; 16] =
            unsafe { MaybeUninit::uninit().assume_init() };
        for i in 0..16 {
            active[i] = MaybeUninit::new(Box::new(BootstrapTCB::new()));
        }
        Mutex::new(unsafe { core::mem::transmute::<_, [Box<dyn TCB>; 16]>(active) })
    };
}

pub fn get_active() -> &'static dyn TCB {
    let was = machine::disable();
    let active = ACTIVE.lock()[smp::me() as usize].borrow_mut();
    machine::enable(was);
    active
}

pub fn swap_active(swap_to: Box<dyn TCB>) -> Box<dyn TCB> {
    let was = machine::disable();
    let mut result = swap_to;
    core::mem::swap(&mut result, &mut ACTIVE.lock()[smp::me() as usize]);
    result
}


pub trait TCB: core::marker::Send {
    fn get_info(&mut self) -> *mut TCBInfo;
}

#[repr(C)]
struct BootstrapTCB {
    tcb_info: TCBInfo,
    stack_frame_start: Option<usize>,
}

impl BootstrapTCB {
    pub fn new() -> BootstrapTCB {
        BootstrapTCB {
            tcb_info: TCBInfo::new(0),
            stack_frame_start: None,
        }
    }
}

impl TCB for BootstrapTCB {
    fn get_info(&mut self) -> *mut TCBInfo {
        &mut self.tcb_info as *mut TCBInfo
    }
}

#[repr(C)]
struct TCBImpl<T: Fn() + core::marker::Send> {
    tcb_info: TCBInfo,
    stack: Box<Stack>,
    work: Box<T>,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TCBInfo {
    stack_pointer: usize,
    leave_me_alone: bool,
}

impl TCBInfo {
    pub fn new(stack_pointer: usize) -> TCBInfo {
        TCBInfo {
            stack_pointer: stack_pointer,
            leave_me_alone: false,
        }
    }
}

impl<T: Fn() + core::marker::Send> TCBImpl<T> {
    const NUM_CALLEE_SAVED: usize = 6;

    pub fn new(work: T) -> TCBImpl<T> {
        let mut stack = box Stack::new();
        let end_of_stack = 511;
        stack.stack[end_of_stack] = thread_entry_point as *const () as u64;
        let index: usize = end_of_stack - TCBImpl::<T>::NUM_CALLEE_SAVED - 1;
        stack.stack[index] = 0; // Flags
        stack.stack[index - 1] = 0; // CR2
        let stack_ptr = Box::into_raw(stack);
        let stack_ptr_as_usize = stack_ptr as usize;
        println!(
            "loaded return at 0x{:x}",
            stack_ptr_as_usize + (end_of_stack * core::mem::size_of::<usize>())
        );
        let x = stack_ptr_as_usize + ((index - 1) * core::mem::size_of::<usize>());
        println!("initial rsp 0x{:x}", x);
        let tcb_info = TCBInfo::new(x);
        stack = unsafe { Box::from_raw(stack_ptr) };
        TCBImpl {
            tcb_info: tcb_info,
            stack: stack,
            work: Box::new(work),
        }
    }
}

impl<T: Fn() + core::marker::Send> TCB for TCBImpl<T> {
    fn get_info(&mut self) -> *mut TCBInfo {
        &mut self.tcb_info as *mut TCBInfo
    }
}

#[no_mangle]
pub extern "C" fn thread_entry_point() -> ! {
    println!("thread arrived at entry point with rsp {:x}", unsafe {
        machine::get_rsp()
    });
    //work();
    println!("thread finished work");
    loop {}
}

/// Yield is a reserved word in Rust, so we use a synonym
pub fn surrender() {
    let mut test1 = Box::new(TCBImpl::new(|| ()));
    println!("{} in surrender after heap allocation", smp::me());
    let mut test2 = Box::new(TCBImpl::new(|| ()));
    println!("attempting to context switch");
    let x = test2.get_info();
    unsafe {
        println!("switching to rsp {:x}", unsafe { *(x as *mut usize) });
    }
    unsafe {
        machine::context_switch(test1.get_info(), test2.get_info());
    }
}
