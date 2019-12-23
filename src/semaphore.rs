use alloc::collections::VecDeque;
use alloc::boxed::Box;
use alloc::sync::{Arc, Weak};
use core::sync::atomic::{Ordering, AtomicU64};
use crate::thread::{TCB, READY, CLEANUP};
use crate::thread;
use crate::smp;
use spin::{Mutex, MutexGuard};
use core::cell::UnsafeCell;

/// A universal synchronization primitive. Blocks if count == 0.
struct Semaphore<'a> {
    control: Mutex<()>,
    internals: UnsafeCell<SemaphoreInternals<'a>>
}

impl<'a> Semaphore<'a> {
    pub fn new() -> Arc<Semaphore<'a>> {
        let sem = box Semaphore {control: Mutex::new(()), internals: UnsafeCell::new(SemaphoreInternals::new())};
        let sem_arc: Arc<Semaphore> = Arc::from(sem);
        sem_arc.control.lock();
        unsafe {
            let internals = sem_arc.internals.get();
            (*internals).weak_self = Some(Arc::downgrade(&sem_arc));
        }
        sem_arc
    }
    
    pub fn up(&mut self) {
        let lock = self.control.lock();
        let internals = self.internals.get();
        unsafe {
            match (*internals).blocked.pop_front() {
                Some(tcb) => {
                    READY.lock().push_back(tcb);
                },
                None => (*internals).count += 1,
            }
        }
    }

    pub fn down(&mut self) {
        let lock = self.control.lock();
        let mut internals = unsafe {Box::from_raw(self.internals.get())};
        let count = unsafe {((*internals).count)};
        if (count == 0) {
            // Block
            let mut active = match thread::swap_active(None) {
                Some(tcb) => tcb,
                None => panic!("Called down on semaphore with no active thread."),
            };
            let current_state = active.get_info();
            internals.held_lock = Some(lock);
            let add_to_blocked_queue = move || {
                // Move lock ownership to lambda
                internals.blocked.push_back(active);
                let lock = match internals.held_lock {
                    Some(lock) => lock,
                    None => panic!("Failed to hold lock during block"),
                };
                let ptr = Box::into_raw(internals);
                drop(lock);
            };
            let x = box add_to_blocked_queue;
            CLEANUP[smp::me()].lock().add_task(x);
        }
        else {
            unsafe {
                internals.count -= 1;
                let ptr = Box::into_raw(internals);
            }
        }
    }
    
}

struct SemaphoreInternals<'a> {
    count: u64,
    blocked: VecDeque<Box<dyn TCB>>,
    weak_self: Option<Weak<Semaphore<'a>>>,
    held_lock: Option<MutexGuard<'a, ()>>,
}

impl<'a> SemaphoreInternals<'a> {
    pub fn new() -> SemaphoreInternals<'a> {
        SemaphoreInternals { count: 0, blocked: VecDeque::new(), weak_self: None, held_lock: None}
    }
}

unsafe impl core::marker::Sync for SemaphoreInternals {}
unsafe impl core::marker::Send for SemaphoreInternals {}
