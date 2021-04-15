#[macro_use]
use crate::println;

use crate::ismutex::ISMutex;
use core::alloc::{GlobalAlloc, Layout};
use core::ops::Deref;
use core::ptr::NonNull;
use linked_list_allocator::Heap;

/// A wrapper around Phil Opp's Heap to that uses an interrupt-safe Mutex
pub struct ISHeap(ISMutex<Heap>);

impl ISHeap {
    /// Creates an empty heap. All allocate calls will return `None`.
    pub const fn empty() -> ISHeap {
        ISHeap(ISMutex::new(Heap::empty()))
    }

    /// Creates a new heap with the given `bottom` and `size`. The bottom address must be valid
    /// and the memory in the `[heap_bottom, heap_bottom + heap_size)` range must not be used for
    /// anything else. This function is unsafe because it can cause undefined behavior if the
    /// given address is invalid.
    pub unsafe fn new(heap_bottom: usize, heap_size: usize) -> ISHeap {
        ISHeap(ISMutex::new(Heap::new(heap_bottom, heap_size)))
    }

    pub unsafe fn init(&self, heap_bottom: usize, heap_size: usize) {
        self.0.lock().init(heap_bottom, heap_size);
        println!("initialized the ISHeap");
    }
}

impl Deref for ISHeap {
    type Target = ISMutex<Heap>;

    fn deref(&self) -> &ISMutex<Heap> {
        &self.0
    }
}

unsafe impl GlobalAlloc for ISHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.0
            .lock()
            .allocate_first_fit(layout)
            .ok()
            .map_or(0 as *mut u8, |allocation| allocation.as_ptr())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0
            .lock()
            .deallocate(NonNull::new_unchecked(ptr), layout)
    }
}
