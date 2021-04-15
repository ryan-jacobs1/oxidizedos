use crate::machine;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

pub struct SpinLock {
    taken: AtomicBool,
}

impl SpinLock {
    pub const fn new() -> SpinLock {
        SpinLock {
            taken: AtomicBool::new(false),
        }
    }
    pub fn lock(&self) -> bool {
        let mut was = machine::disable();
        while self.taken.swap(true, Ordering::SeqCst) {
            machine::enable(was);
            was = machine::disable();
        }
        was
    }
    pub fn unlock(&self, was: bool) {
        self.taken.swap(false, Ordering::SeqCst);
        machine::enable(was);
    }
}

unsafe impl core::marker::Send for SpinLock {}
unsafe impl core::marker::Sync for SpinLock {}
