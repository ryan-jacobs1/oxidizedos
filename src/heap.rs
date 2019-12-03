use std::alloc::{GlobalAlloc, Layout};

pub struct Heap {
    head: *mut Block,
    size: usize,
}

pub struct Block {
    is_free: bool,
    size: usize,
    prev_block_size: usize,
}

static BLOCK_SIZE: usize = std::mem::size_of::<Block>() as usize;

impl Block {
    fn find_free_mem(&mut self, size: usize, last_block_addr: *mut Block) -> *mut usize {
        if self.is_free && size <= self.size {
            unsafe {
                if size + BLOCK_SIZE + std::mem::size_of::<usize>() <= self.size {
                    let new_block_addr = ((self as *mut Block) as usize + BLOCK_SIZE + size) as *mut Block;
                    *new_block_addr = Block { 
                        is_free: true, 
                        size: self.size - size - BLOCK_SIZE,
                        prev_block_size: size + BLOCK_SIZE,
                    };

                    (*self).size = size;
                }

                (*self).is_free = false;
                ((self as *mut Block) as usize + BLOCK_SIZE) as *mut usize
            }
        }
        else {
            let next_block_addr = (self as *mut Block as usize + (self.size + BLOCK_SIZE) as usize) as *mut Block;
            if next_block_addr != last_block_addr {
                unsafe { (*next_block_addr).find_free_mem(size, last_block_addr) }
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
            let next_block_addr = (self as *mut Block as usize + (self.size + BLOCK_SIZE) as usize) as *mut Block;
            let next_next_block_addr = (next_block_addr as usize + (*next_block_addr).size + BLOCK_SIZE) as *mut Block;

            if self.prev_block_size != 0 && (*prev_block_addr).is_free {
                (*prev_block_addr).size += self.size + BLOCK_SIZE;
                (*next_block_addr).prev_block_size = (*prev_block_addr).size + BLOCK_SIZE; // Must update next block's previous size to allow for future coalensces
            }

            if next_block_addr != last_block_addr && (*next_block_addr).is_free  {
                if self.prev_block_size != 0 && (*prev_block_addr).is_free {            // If the previous block was coalesced, we must update the address size instead of self
                    (*prev_block_addr).size += (*next_block_addr).size + BLOCK_SIZE;
                    if next_next_block_addr != last_block_addr {
                        (*next_next_block_addr).prev_block_size = (*prev_block_addr).size + BLOCK_SIZE;
                    }
                }
                else {                                                                  // If only self and next are coalesced, update self size and next_next prev size
                    self.size += (*next_block_addr).size + BLOCK_SIZE;
                    if next_next_block_addr != last_block_addr {
                        (*next_next_block_addr).prev_block_size = self.size + BLOCK_SIZE;
                    }
                }
            }
        }
    }
}

unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        (*self.head).find_free_mem(layout.size(), (self.head as usize + self.size) as *mut Block) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        let deallocated_block = (ptr as usize - BLOCK_SIZE) as *mut Block;
        (*deallocated_block).free((self.head as usize + self.size) as *mut Block);
    }
}

impl Heap {
    fn new(addr: *mut usize, size: usize) -> Self {
        unsafe {
            let head = addr as *mut Block;
            *head = Block {
                is_free: true, 
                size: size - BLOCK_SIZE,
                prev_block_size: 0,
            };
            Self { head, size }
        }
    }
}