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
use crate::ismutex::ISMutex;

lazy_static! {
    pub static ref READY: ISMutex<VecDeque<Box<dyn TCB>>> = ISMutex::new(VecDeque::new());
}

lazy_static! {
    /// Invariant: When Active[i] == None, core i is guaranteed not to context switch due to a timer interrupt
    pub static ref ACTIVE: [ISMutex<Option<Box<dyn TCB>>>; 16] = {
        let mut active: [MaybeUninit<ISMutex<Option<Box<dyn TCB>>>>; 16] =
            unsafe { MaybeUninit::uninit().assume_init() };
        for i in 0..16 {
            active[i] = MaybeUninit::new(ISMutex::new(Some(BootstrapTCB::new_box())));
        }
        unsafe { core::mem::transmute::<_, [ISMutex<Option<Box<dyn TCB>>>; 16]>(active) }
    };
}

lazy_static! {
    pub static ref CLEANUP: [ISMutex<Box<TaskHolder>>; 16] = {
        let mut cleanup: [MaybeUninit<ISMutex<Box<TaskHolder>>>; 16] =
            unsafe { MaybeUninit::uninit().assume_init() };
        for i in 0..16 {
            cleanup[i] = MaybeUninit::new(ISMutex::new(box TaskHolder::new()));
        }
        unsafe { core::mem::transmute::<_, [ISMutex<Box<TaskHolder>>; 16]>(cleanup) }
    };
}

/// Swap the active thread with another thread. If swapped with None,
/// then all preemption attempts will be aborted, until Some(tcb) is swapped in.
pub fn swap_active(swap_to: Option<Box<dyn TCB>>) -> Option<Box<dyn TCB>> {
    let was = machine::disable();
    let mut result = swap_to;
    core::mem::swap(&mut result, &mut ACTIVE[smp::me()].lock());
    machine::enable(was);
    result
}


pub trait TCB: Send + Sync {
    fn get_info(&mut self) -> *mut TCBInfo;
    fn get_work(&mut self) -> Box<'static + FnOnce() + Send + Sync>;
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
    pub fn new_box() -> Box<BootstrapTCB> {
        box BootstrapTCB {
            tcb_info: TCBInfo::new(0),
            stack_frame_start: None
        }
    }
}

impl TCB for BootstrapTCB {
    fn get_info(&mut self) -> *mut TCBInfo {
        &mut self.tcb_info as *mut TCBInfo
    }

    fn get_work(&mut self) -> Box<Task> {
        panic!("BootstrapTCB has no work to do!");
    }
}

type Task = 'static + FnOnce() + Send + Sync;

#[repr(C)]
pub struct TCBImpl {
    tcb_info: TCBInfo,
    stack: Box<[u64]>,
    work: Option<Box<Task>>,
}

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

impl TCBImpl {
    const NUM_CALLEE_SAVED: usize = 6;

    pub fn new(work: Box<Task>) -> TCBImpl {
        let mut stack: Box<[u64]> = box [0; 4096];
        let end_of_stack = 4095;
        stack[end_of_stack] = thread_entry_point as *const () as u64;
        let index: usize = end_of_stack - TCBImpl::NUM_CALLEE_SAVED - 1;
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
            work: Some(work),
        }
    }
}

impl TCB for TCBImpl {
    fn get_info(&mut self) -> *mut TCBInfo {
        &mut self.tcb_info as *mut TCBInfo
    }

    fn get_work(&mut self) -> Box<Task> {
        let mut work = None;
        core::mem::swap(&mut work, &mut self.work);
        match work {
            Some(task) => task,
            None => panic!("TCBImpl had no work!")
        }
    }
}




type Cleanup = FnOnce() + Send + Sync;

/// Holds tasks to perform after context-switching.
/// No mutual exclusion needed as this is a per-core data structure
pub struct TaskHolder {
    tasks: VecDeque<Box<Cleanup>>,
}

impl TaskHolder {
    pub fn new() -> TaskHolder {
        TaskHolder {tasks: VecDeque::new()}
    }
    pub fn add_task(&mut self, task: Box<Cleanup>) {
        self.tasks.push_back(task);
    }
    pub fn get_task(&mut self) -> Option<Box<Cleanup>> {
        self.tasks.pop_front()
    }
}

#[no_mangle]
pub extern "C" fn thread_entry_point() -> ! {
    cleanup();
    {
        let was = machine::disable();
        let mut active = match swap_active(None) {
            Some(active) => active,
            None => panic!("No thread available in thread entry point"),
        };
        let task = active.get_work();
        swap_active(Some(active));
        machine::enable(was);
        task();
    }
    stop();
    loop {}
}

pub fn yeet() -> [ISMutex<Option<Box<dyn TCB>>>; 16] {
    {
        let mut active: [MaybeUninit<ISMutex<Option<Box<dyn TCB>>>>; 16] =
            unsafe { MaybeUninit::uninit().assume_init() };
        for i in 0..16 {
            active[i] = MaybeUninit::new(ISMutex::new(Some(box BootstrapTCB::new())));
        }
        unsafe { core::mem::transmute::<_, [ISMutex<Option<Box<dyn TCB>>>; 16]>(active) }
    }
}

pub fn init() {
    println!("initializing threads...");
    lazy_static::initialize(&READY);
    //println!("ready complete");
    //println!("initializing active");
    lazy_static::initialize(&ACTIVE);
    //println!("active complete");
    lazy_static::initialize(&CLEANUP);
    println!("threads initialized");
}

pub fn surrender() {
    surrender_help(true);
}

pub fn stop() {
    surrender_help(false);
}

/// Yield is a reserved word in Rust, so we use a synonym
fn surrender_help(run_again: bool) {
    // If there's no active thread, return as we are currently surrendering
    let mut current_thread: Box<dyn TCB> = match swap_active(None) {
        Some(mut tcb) => {tcb},
        None => {return}
    };
    // Don't need to disable interrupts, as we will run on this core until we context switch
    let me = smp::me();
    let current_thread_info = current_thread.get_info();
    if (run_again) {
        let add_to_ready = move || {
            READY.lock().push_back(current_thread);
        };
        CLEANUP[me].lock().add_task(Box::new(add_to_ready));
    } else {
        let drop_current = move || {
            let x = current_thread;
            drop(x);
        };
        CLEANUP[me].lock().add_task(Box::new(drop_current));
    }
    block(current_thread_info);
}

pub fn block(current_thread_info: *mut TCBInfo) {
    // Find something to switch to
    let mut next_thread: Box<dyn TCB> = match READY.lock().pop_front() {
        Some(mut tcb) => tcb,
        None => {
            // Implementation Note: Potentially a trade off to switch to something that switches back,
            // but most of the time, there should be something in the ready q
            let work = move || {
                return
            };
            let busy_work = Box::new(TCBImpl::new(Box::new(work)));
            busy_work
        }
    };
    let next_thread_info = next_thread.get_info();
    let assert_as_active = move || {
        // The next thread will now assert itself as the active thread
        swap_active(Some(next_thread));
    };
    CLEANUP[smp::me()].lock().add_task(Box::new(assert_as_active));
    unsafe {
        machine::context_switch(current_thread_info, next_thread_info)
    }
    cleanup();
}

fn cleanup() {
    let was = machine::disable();
    let me = smp::me();
    let mut cleanup_work = CLEANUP[me].lock();
    machine::enable(was);
    loop {
        match cleanup_work.get_task() {
            Some(work) => {work()},
            None => {break}
        }
    }
}

pub fn schedule(tcb: Box<dyn TCB>) {
    unsafe {
        let was = machine::disable();
        READY.lock().push_back(tcb);
        machine::enable(was);
    }
}

pub fn surrender_test() {
    let mut test1 = Box::new(TCBImpl::new(box || ()));
    println!("{} in surrender after heap allocation", smp::me());
    let mut test2 = Box::new(TCBImpl::new(box || ()));
    println!("attempting to context switch");
    let x = test2.get_info();
    unsafe {
        println!("switching to rsp {:x}", unsafe { *(x as *mut usize) });
    }
    unsafe {
        machine::context_switch(test1.get_info(), test2.get_info());
    }
}
