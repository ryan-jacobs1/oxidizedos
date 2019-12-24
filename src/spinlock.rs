use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

pub struct SpinLock {
    taken: AtomicBool,
}

impl SpinLock {
    pub const fn new() -> SpinLock {
        SpinLock {taken: AtomicBool::new(false)}
    }
    pub fn lock(&self) {
        while self.taken.swap(true, Ordering::SeqCst) {}
    }
    pub fn unlock(&self) {
        self.taken.swap(false, Ordering::SeqCst);
    }
}