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
use core::sync::atomic::{AtomicUsize, Ordering, AtomicU32};
use alloc::sync::Arc;

lazy_static! {
    pub static ref READY: Mutex<VecDeque<Box<dyn TCB>>> = Mutex::new(VecDeque::new());

    pub static ref ACTIVE: [Mutex<Option<Box<dyn TCB>>>; 16] = {
        let mut active: [MaybeUninit<Mutex<Option<Box<dyn TCB>>>>; 16] =
            unsafe { MaybeUninit::uninit().assume_init() };
        for i in 0..16 {
            active[i] = MaybeUninit::new(Mutex::new(Some(Box::new(BootstrapTCB::new()))));
        }
        unsafe { core::mem::transmute::<_, [Mutex<Option<Box<dyn TCB>>>; 16]>(active) }
    };

}

lazy_static! {
    pub static ref CLEANUP: [Mutex<Box<TaskHolder>>; 16] = {
        let mut cleanup: [MaybeUninit<Mutex<Box<TaskHolder>>>; 16] =
            unsafe { MaybeUninit::uninit().assume_init() };
        for i in 0..16 {
            cleanup[i] = MaybeUninit::new(Mutex::new(box TaskHolder::new()));
        }
        unsafe { core::mem::transmute::<_, [Mutex<Box<TaskHolder>>; 16]>(cleanup) }
    };
}

/// Swap the active thread with another thread
pub fn swap_active(swap_to: Option<Box<dyn TCB>>) -> Option<Box<dyn TCB>> {
    let mut result = swap_to;
    core::mem::swap(&mut result, &mut ACTIVE[smp::me()].lock());
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
}

impl TCB for BootstrapTCB {
    fn get_info(&mut self) -> *mut TCBInfo {
        &mut self.tcb_info as *mut TCBInfo
    }

    fn get_work(&mut self) -> Box<'static + FnOnce() + Send + Sync> {
        panic!("BootstrapTCB has no work to do!");
    }
}


#[repr(C)]
pub struct TCBImpl<T: 'static + FnOnce() + Send + Sync> {
    tcb_info: TCBInfo,
    stack: Box<[u64]>,
    work: Option<Box<T>>,
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

impl<T: 'static + FnOnce() + Send + Sync> TCBImpl<T> {
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

impl<T: 'static + FnOnce() + Send + Sync> TCB for TCBImpl<T> {
    fn get_info(&mut self) -> *mut TCBInfo {
        &mut self.tcb_info as *mut TCBInfo
    }

    fn get_work(&mut self) -> Box<'static + FnOnce() + Send + Sync> {
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

pub fn init() {
    println!("initializing threads...");
    lazy_static::initialize(&READY);
    lazy_static::initialize(&ACTIVE);
    lazy_static::initialize(&CLEANUP);
    println!("threads initialized");
}

pub fn surrender() {
    surrender_help(true);
}

pub fn stop() {
    surrender_help(false);
}

pub fn surrender_help(run_again: bool) {
    let mut current_thread: Box<dyn TCB> = match swap_active(None) {
        Some(mut tcb) => {tcb},
        None => {panic!("No active thread!")}
    };
    let current_thread_info = current_thread.get_info();
    let me = smp::me();
    if (run_again) {
        // Have the next thread add us back to the ready queue
        let add_to_ready = move || {
            READY.lock().push_back(current_thread);
        };
        CLEANUP[me].lock().add_task(Box::new(add_to_ready));
    } else {
        // Have the next thread free all the memory associated with the current TCB
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
            let busy_work_box = Box::new(TCBImpl::new(work));
            busy_work_box
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
    READY.lock().push_back(tcb);
}


pub fn cooperative_scheduler_test() {
    println!("running cooperative scheduler test");
    let counter = Arc::new(AtomicU32::new(0));
    for i in 0..10 {  
        let c = Arc::clone(&counter);
        let x = TCBImpl::new(move || {
            for i in 0..10 {
                c.fetch_add(1, Ordering::SeqCst);
                surrender();
            }
        });
        schedule(box x);
    }
    println!("scheduled all threads");
    while counter.load(Ordering::SeqCst) < 100 {
        surrender();
    }
    println!("counter: {}", counter.load(Ordering::SeqCst));
}
