use alloc::collections::VecDeque;
use alloc::boxed::Box;
use alloc::sync::{Arc, Weak};
use core::sync::atomic::{Ordering, AtomicU64};
use crate::thread::{TCB, READY, CLEANUP};
use crate::thread;
use crate::smp;
use spin::{Mutex, MutexGuard};
use core::cell::UnsafeCell;
use crate::spinlock::SpinLock;
use crate::machine;

/// A universal synchronization primitive. Blocks if count == 0.
struct Semaphore {
    control: SpinLock,
    internals: SemaphoreInternalWrapper
}

impl Semaphore {
    pub fn new() -> Arc<Semaphore> {
        let sem = box Semaphore {control: SpinLock::new(), internals: SemaphoreInternalWrapper::new()};
        let sem_arc: Arc<Semaphore> = Arc::from(sem);
        sem_arc.control.lock();
        unsafe {
            let internals = sem_arc.internals.data.get();
            (*internals).weak_self = Some(Arc::downgrade(&sem_arc));
        }
        sem_arc
    }
    
    pub fn up(&mut self) {
        let was = self.control.lock();
        let internals = self.internals.data.get();
        unsafe {
            match (*internals).blocked.pop_front() {
                Some(tcb) => {
                    READY.lock().push_back(tcb);
                },
                None => (*internals).count += 1,
            }
        }
        self.control.unlock(was);
    }

    pub fn down(&mut self) {
        let was = self.control.lock();
        let mut internals = unsafe {Box::from_raw(self.internals.data.get())};
        let count = unsafe {((*internals).count)};
        if (count == 0) {
            // Block
            let mut active = match thread::swap_active(None) {
                Some(tcb) => tcb,
                None => panic!("Called down on semaphore with no active thread."),
            };
            let current_state = active.get_info();
            let me: Arc<Semaphore> = match internals.weak_self {
                Some(ref ptr) => match ptr.upgrade() {
                    Some(ptr) => ptr,
                    None => panic!("Semaphore has been dropped and the weak pointer was invalid")
                }
                None => panic!("No weak pointer")
            };
            let add_to_blocked_queue = move || {
                // Move internals ownership to lambda and release lock
                internals.blocked.push_back(active);
                let ptr = Box::into_raw(internals);
                me.control.unlock(true);
            };
            CLEANUP[smp::me()].lock().add_task(box add_to_blocked_queue);
            machine::enable(was);
            thread::block(current_state);
        }
        else {
            unsafe {
                internals.count -= 1;
                let ptr = Box::into_raw(internals);
            }
            self.control.unlock(was);
        }
    }
    
}

/// Thread-safe as mutual exclusion is provided through holding the control semaphore
struct SemaphoreInternalWrapper {
    data: UnsafeCell<SemaphoreInternals>
}

impl SemaphoreInternalWrapper {
    pub fn new() -> SemaphoreInternalWrapper {
        SemaphoreInternalWrapper {data: UnsafeCell::new(SemaphoreInternals::new())}
    }
}

unsafe impl core::marker::Sync for SemaphoreInternalWrapper {}
unsafe impl core::marker::Send for SemaphoreInternalWrapper {}


struct SemaphoreInternals {
    count: u64,
    blocked: VecDeque<Box<dyn TCB>>,
    weak_self: Option<Weak<Semaphore>>,
}

impl SemaphoreInternals {
    pub fn new() -> SemaphoreInternals {
        SemaphoreInternals { count: 0, blocked: VecDeque::new(), weak_self: None}
    }
}


