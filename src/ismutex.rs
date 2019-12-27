/// An interrupt safe version of the mutex from the spin-rs crate.
/// https://github.com/mvdnes/spin-rs/

/*
The MIT License (MIT)

Copyright (c) 2014 Mathijs van de Nes

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/


use spin::{Mutex, MutexGuard};
use crate::spinlock::SpinLock;
use core::marker::{Send, Sync, Sized};
use core::ops::{Deref, DerefMut};
use core::cell::UnsafeCell;

pub struct ISMutex<T: ?Sized> {
    lock: SpinLock,
    data: UnsafeCell<T>
}

impl<T> ISMutex<T> {
    pub const fn new(data: T) -> ISMutex<T> {
        ISMutex {lock: SpinLock::new(), data: UnsafeCell::new(data)}
    }

    pub fn lock(&self) -> ISMutexGuard<T> {
        let was = self.lock.lock();
        ISMutexGuard {lock: &self.lock, data: unsafe {&mut *self.data.get()}, was: was}
    }
}

unsafe impl<T: Send> Send for ISMutex<T> {}
unsafe impl<T: Send> Sync for ISMutex<T> {}


pub struct ISMutexGuard<'a, T: ?Sized + 'a> {
    lock: &'a SpinLock,
    data: &'a mut T,
    was: bool
}

impl<'a, T: ?Sized + 'a> Deref for ISMutexGuard<'a, T> {
    type Target = T;
    fn deref<'b>(&'b self) -> &'b T {
        unsafe {&*self.data}
    }
}

impl<'a, T: ?Sized + 'a> DerefMut for ISMutexGuard<'a, T> {
    fn deref_mut<'b>(&'b mut self) -> &'b mut T {
        unsafe {&mut *self.data}
    }
}

impl<'a, T: ?Sized + 'a> Drop for ISMutexGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.unlock(self.was);
    }
}