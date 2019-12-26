extern crate spin;


use spin::Mutex;

use alloc::alloc::{GlobalAlloc, Layout};


pub struct Heap {
    head: usize,
    size: usize,
}

pub struct LockedHeap {
    heap: Mutex<Heap>,
}


pub struct Block {
    is_free: bool,
    size: usize,
    prev_block_size: usize,
}

static OVERHEAD_SIZE: usize = 8 + core::mem::size_of::<Block>() as usize;

impl Block {
    fn find_free_mem(&mut self, layout: Layout, last_block_addr: *mut Block) -> *mut usize {
        let offset = match ((self as *mut Block) as usize + OVERHEAD_SIZE) % layout.align() {
            0 => 0,
            _ => layout.align() - ((self as *mut Block) as usize + OVERHEAD_SIZE) % layout.align(),
        };
        let size = layout.size() + offset;
        if self.is_free && size <= self.size {
            unsafe {
                if size + OVERHEAD_SIZE + core::mem::size_of::<usize>() <= self.size {
                    let new_block_addr = ((self as *mut Block) as usize + OVERHEAD_SIZE + size) as *mut Block;
                    *new_block_addr = Block { 
                        is_free: true, 
                        size: self.size - size - OVERHEAD_SIZE,
                        prev_block_size: size + OVERHEAD_SIZE,
                    };

                    (*self).size = size;
                }

                (*self).is_free = false;
                (*(((self as *mut Block) as usize + OVERHEAD_SIZE + offset - 8) as *mut usize)) = (self as *mut Block) as usize;
                ((self as *mut Block) as usize + OVERHEAD_SIZE + offset) as *mut usize
            }
        }
        else {
            let next_block_addr = (self as *mut Block as usize + (self.size + OVERHEAD_SIZE) as usize) as *mut Block;
            if next_block_addr != last_block_addr {
                unsafe { (*next_block_addr).find_free_mem(layout, last_block_addr) }
            }
            else {
                panic!("Uh Oh Sisters: Out Of Memory!!! :(");
            }
        }
    }

    fn free(&mut self, last_block_addr: *mut Block) {
        self.is_free = true;
        self.coalesce(last_block_addr);
    }
    
    fn coalesce(&mut self, last_block_addr: *mut Block) {
        unsafe {
            let prev_block_addr = (self as *mut Block as usize - self.prev_block_size) as *mut Block;
            let next_block_addr = (self as *mut Block as usize + (self.size + OVERHEAD_SIZE) as usize) as *mut Block;
            let next_next_block_addr = (next_block_addr as usize + (*next_block_addr).size + OVERHEAD_SIZE) as *mut Block;

            if self.prev_block_size != 0 && (*prev_block_addr).is_free {
                (*prev_block_addr).size += self.size + OVERHEAD_SIZE;
                (*next_block_addr).prev_block_size = (*prev_block_addr).size + OVERHEAD_SIZE; // Must update next block's previous size to allow for future coalensces
            }

            if next_block_addr != last_block_addr && (*next_block_addr).is_free  {
                if self.prev_block_size != 0 && (*prev_block_addr).is_free {            // If the previous block was coalesced, we must update the address size instead of self
                    (*prev_block_addr).size += (*next_block_addr).size + OVERHEAD_SIZE;
                    if next_next_block_addr != last_block_addr {
                        (*next_next_block_addr).prev_block_size = (*prev_block_addr).size + OVERHEAD_SIZE;
                    }
                }
                else {                                                                  // If only self and next are coalesced, update self size and next_next prev size
                    self.size += (*next_block_addr).size + OVERHEAD_SIZE;
                    if next_next_block_addr != last_block_addr {
                        (*next_next_block_addr).prev_block_size = self.size + OVERHEAD_SIZE;
                    }
                }
            }
        }
    }
}

unsafe impl GlobalAlloc for LockedHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut heap = self.heap.lock();
        let mut block: &mut Block = &mut *(heap.head as *mut Block);
        block.find_free_mem(layout, (heap.head + heap.size) as *mut Block) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        let heap = self.heap.lock();
        let deallocated_block = (*((ptr as usize - 8) as *mut usize)) as *mut Block;
        (*deallocated_block).free((heap.head as *mut Block as usize + heap.size) as *mut Block);
    }
}

unsafe impl core::marker::Send for LockedHeap {}
unsafe impl core::marker::Send for Block {}

impl Heap {
    pub const fn new() -> Self {
        Self {head: 0 as usize, size: 0 as usize}
    }
    pub fn init(&mut self, addr: usize, size: usize) {
        unsafe {
            let head = addr as *mut Block;
            *head = Block {
                is_free: true, 
                size: size - OVERHEAD_SIZE,
                prev_block_size: 0,
            };
            self.head = head as usize;
            self.size = size;
        }
    }
}

impl LockedHeap {
    pub const fn new() -> Self {
        let heap = spin::Mutex::new(Heap::new());
        Self {heap}
    }
    pub fn init(&mut self, addr: usize, size: usize) {
        let mut heap = self.heap.lock();
        heap.init(addr, size);
    }
}