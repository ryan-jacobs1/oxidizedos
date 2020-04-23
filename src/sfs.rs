use crate::ide::{IDEImpl, IDE};
use crate::{println, panic};

use alloc::{boxed::Box, vec, vec::Vec};

static mut SYSTEM_INCREMENTER: u64 = 1;

// Hex value 0x0194 to 0x01BD. Size = 42 Bytes
#[repr(C, packed)]
struct SuperBlock {
    timestamp: u64,
    data_area_size: u64,
    index_area_size: u64,
    magicnum_and_sfs_version: u32,
    total_blocks: u64,
    reserved_block: u32,
    block_size: u8,
    checksum: u8
}

impl SuperBlock {
    fn print(&self) {
        println!("Timestamp: 0x{:x}", self.timestamp);
        println!("Date area size: {} blocks", self.data_area_size);
        println!("Index area size: {} bytes", self.index_area_size);
        println!("Magic number: 0x{:x}", self.magicnum_and_sfs_version & 0x00FFFFFF);
        println!("SFS version: {}", self.magicnum_and_sfs_version >> 24);
        println!("Total blocks: {}", self.total_blocks);
        println!("Reserved blocks: {}", self.reserved_block);
        println!("Block size: {} bytes", self.block_size_bytes());
        println!("Checksum: 0x{:x}", self.checksum);
    }

    fn block_size_bytes(&self) -> u64 {
        1 << (self.block_size + 7)
    }

    fn get_media_size(&self) -> u64 {
        self.total_blocks * (1 << self.block_size + 7)
    }

    fn data_start_location(&self) -> u64 {
        self.reserved_block as u64 * self.block_size_bytes()
    }

    fn index_start_location(&self) -> u64 {
        self.get_media_size() - self.index_area_size
    }

    fn increment_data_area_size(&mut self, inc: u64) {
        self.data_area_size += inc;
        self.timestamp = get_timestamp();
    }

    fn increment_index_area_size(&mut self, inc: u64) {
        self.index_area_size += inc;
        self.timestamp = get_timestamp();
    }
}

pub struct SFS {
    ide: IDEImpl,
    super_block: Box<SuperBlock>
}

impl SFS {
    pub fn new(drive: u32) -> SFS {
        let ide = IDEImpl::new(drive);
        // Only require 42 bytes, but need to increase to 44 for alignment
        let mut buf: Box<[u32]> = box [0; 44 / 4];
        // Taking the SuperBlock plus 2 bytes
        ide.read(404, &mut buf, 44);
        let buf_raw = Box::into_raw(buf);
        SFS { 
            ide: ide,
            super_block: unsafe { Box::from_raw(buf_raw as *mut SuperBlock) }
        }
    }

    pub fn create_file(&mut self, filename: &str, blocks: u64) {
        let filename_u8: &[u8] = filename.as_bytes();
        if filename_u8.len() <= 30 {
            match self.get_file_entry(filename) {
                Ok((file_entry, pos)) => {
                    println!("File already exists");
                },
                Err(e) => {
                    let mut filename_padded: [u8; 30] = [0; 30];
                    unsafe { core::ptr::copy(&filename_u8[0] as *const u8, &mut filename_padded[0] as *mut u8, filename_u8.len() as usize); }
                    // ! Check the bounds of the data area size
                    let starting_block = self.super_block.data_area_size;
                    let ending_block = starting_block + blocks;
                    let file_entry: FileEntry = FileEntry::new(filename_padded, starting_block, ending_block, 0, 0);
                    let buf: &[u32] = unsafe { 
                        core::slice::from_raw_parts((&file_entry as *const FileEntry) as *const u32, 64)
                    };
                    self.ide.write(self.super_block.index_start_location() as u32, buf, 64);
                    self.move_starting_marker_entry(1);
        
                    self.super_block.increment_data_area_size(blocks);
                    self.update_super_block();
                }
            }
        }
        else {
            panic!("Filelength > 30 is not supported yet");
        }
    }

    pub fn append_to_file(&mut self, filename: &str, content: &[u32]) {
        let filename_u8: &[u8] = filename.as_bytes();
        if filename_u8.len() <= 30 {
            match self.get_file_entry(filename) {
                Ok((file_entry, position)) => {
                    if file_entry.length + (content.len() * 4) as u64 <= ((file_entry.ending_block - file_entry.starting_block) * self.super_block.block_size_bytes()) {
                        let appendLocation = self.super_block.data_start_location() + (file_entry.starting_block * self.super_block.block_size_bytes()) + file_entry.length;
                        println!("Append Location: {}", appendLocation);
                        self.ide.write(appendLocation as u32, content, content.len() as u32 * 4);
                        self.update_file_length(position, file_entry.length + (content.len() as u64 * 4));
                    }
                    else {
                        panic!("File size too small. Cant append");
                    }
                },
                Err(e) => {
                    println!("File doesnt exist");
                }
            };
        }
        else {
            panic!("Filelength > 30 is not supported yet");
        }
    }

    pub fn read_file(&mut self, filename: &str) -> Result<Vec<u32>, &str> {
        let filename_u8: &[u8] = filename.as_bytes();
        if filename_u8.len() <= 30 {
            match self.get_file_entry(filename) {
                Ok((file_entry, position)) => {
                    let mut file_data: Vec<u32> = Vec::with_capacity(((file_entry.ending_block - file_entry.ending_block) * self.super_block.block_size_bytes()) as usize);
                    for i in file_entry.starting_block..file_entry.ending_block {
                        let mut buf: Vec<u32> = Vec::with_capacity(self.super_block.block_size_bytes() as usize);
                        for _ in 0..self.super_block.block_size_bytes() {
                            buf.push(0);
                        }
                        let readLocation = self.super_block.data_start_location() + (file_entry.starting_block * self.super_block.block_size_bytes()) + (i * self.super_block.block_size_bytes());
                        self.ide.read(readLocation as u32, buf.as_mut_slice(), self.super_block.block_size_bytes() as u32);
                        file_data.append(&mut buf);
                    }
                    file_data.resize(file_entry.length as usize / 4, 0);
                    Ok(file_data)
                },
                Err(e) => {
                    Err("File doesnt exist")
                }
            }
        }
        else {
            Err("Filelength > 30 is not supported yet")
        }
    }

    pub fn print_super_block(&self) {
        self.super_block.print();
        println!("{}", self.super_block.index_start_location());
    }

    fn get_file_entry(&self, filename: &str) -> Result<(FileEntry, u64), &str> {
        let filename_u8: &[u8] = filename.as_bytes();
        if filename_u8.len() <= 30 {
            let mut buf: &mut [u32] = &mut [0; 64 / 4];
            for i in (self.super_block.index_start_location()..self.get_media_size()).step_by(64) {
                self.ide.read(i as u32, buf, 64);
                if buf[0] & 0xFF == 0x12 {
                    let mut buf: &mut [u8] = u32_as_u8_mut(buf);
                    let file_entry_slice: &[FileEntry] = unsafe { 
                        core::slice::from_raw_parts((&buf[0] as *const u8) as *const FileEntry, 64)
                    };
                    let mut filename_padded: [u8; 30] = [0; 30];
                    unsafe { core::ptr::copy(&filename_u8[0] as *const u8, &mut filename_padded[0] as *mut u8, filename_u8.len() as usize); }
                    if file_entry_slice[0].filename == filename_padded {
                        println!("File Found!");
                        return Ok((FileEntry::new(
                            file_entry_slice[0].filename, 
                            file_entry_slice[0].starting_block,
                            file_entry_slice[0].ending_block,
                            file_entry_slice[0].length,
                            file_entry_slice[0].continuations     
                        ), i))
                    }
                }
            }
            Err("Uh Oh Sisters: File not found")
        }
        else {
            Err("Uh Oh Sisters: Filelength > 30 is not supported yet")
        }
    }

    fn update_file_length(&self, file_entry_position: u64, length: u64) {
        let length_u64: &[u64] = &[length];
        let buf: &[u32] = unsafe {
            core::slice::from_raw_parts(length_u64.as_ptr() as *const u32, 2)
        };
        let location = file_entry_position + 26;
        self.ide.write(location as u32, buf, 8);
    }

    fn get_media_size(&self) -> u64 {
        self.super_block.total_blocks * (1 << self.super_block.block_size + 7)
    }

    fn move_starting_marker_entry(&mut self, steps: u64) {
        let starting_marker_entry: StartingMarkerEntry = StartingMarkerEntry::new();
        let buf: &[u32] = unsafe { 
            core::slice::from_raw_parts((&starting_marker_entry as *const StartingMarkerEntry) as *const u32, 64)
        };
        self.ide.write((self.super_block.index_start_location() - (64 * steps)) as u32, buf, 64);
        self.super_block.increment_index_area_size(64 * steps);
    }

    fn update_super_block(&mut self) {
        let buf: &[u32] = unsafe { 
            core::slice::from_raw_parts((&*self.super_block as *const SuperBlock) as *const u32, 64)
        };
        self.ide.write(404, buf, 44);
    }
}

#[repr(C, packed)]
struct VolumeIdentifier {
    entry_type: u8,
    unused_reserved: [u8; 3],
    timestamp: u64,
    volume_name: [u8; 52]
}

#[repr(C, packed)]
struct StartingMarkerEntry {
    entry_type: u8,
    unused_reserved: [u8; 63]
}

impl StartingMarkerEntry {
    pub fn new() -> StartingMarkerEntry {
        StartingMarkerEntry {
            entry_type: 0x02,
            unused_reserved: [0; 63]
        }
    }
}

#[repr(C, packed)]
struct UnusedEntry {
    entry_type: u8,
    unused_reserved: [u8; 63]
}

impl UnusedEntry {
    pub fn new() -> UnusedEntry {
        UnusedEntry {
            entry_type: 0x10,
            unused_reserved: [0; 63]
        }
    }
}

#[repr(C, packed)]
struct DirectoryEntry {
    entry_type: u8,
    continuations: u8,
    timestamp: u64,
    directory_name: [u8; 54]
}

impl DirectoryEntry {
    pub fn new(directory_name: &mut [u8; 54], continuations: u8) -> DirectoryEntry {
        DirectoryEntry {
            entry_type: 0x11,
            continuations: continuations,
            timestamp: get_timestamp(),
            directory_name: *directory_name
        }
    }
}

#[repr(C, packed)]
struct FileEntry {
    entry_type: u8,
    continuations: u8,
    timestamp: u64,
    starting_block: u64,
    ending_block: u64,
    length: u64,
    filename: [u8; 30]
}

impl FileEntry {
    pub fn new(filename: [u8; 30], starting_block: u64, ending_block: u64, length: u64, continuations: u8) -> FileEntry {
        FileEntry {
            entry_type: 0x12,
            continuations: continuations,
            timestamp: get_timestamp(),
            starting_block: starting_block,
            ending_block: ending_block,
            length: length,
            filename: filename
        }
    }
}

#[repr(C, packed)]
struct UnusableEntry {
    entry_type: u8,
    unused_reserved: [u8; 9],
    starting_block: u64,
    ending_block: u64,
    unused_reserved2: [u8; 38]
}

impl UnusableEntry {
    pub fn new(starting_block: u64, ending_block: u64) -> UnusableEntry {
        UnusableEntry {
            entry_type: 0x18,
            unused_reserved: [0; 9],
            starting_block: starting_block,
            ending_block: ending_block,
            unused_reserved2: [0; 38]
        }
    }
}

#[repr(C, packed)]
struct DeletedDirectoryEntry {
    entry_type: u8,
    continuations: u8,
    timestamp: u64,
    directory_name: [u8; 54]
}

impl DeletedDirectoryEntry {
    pub fn new(directory_name: &mut [u8; 54], continuations: u8) -> DeletedDirectoryEntry {
        DeletedDirectoryEntry {
            entry_type: 0x19,
            continuations: continuations,
            timestamp: get_timestamp(),
            directory_name: *directory_name
        }
    }
}

#[repr(C, packed)]
struct DeletedFileEntry {
    entry_type: u8,
    continuations: u8,
    timestamp: u64,
    starting_block: u64,
    ending_block: u64,
    length: u64,
    filename: [u8; 30]
}

impl DeletedFileEntry {
    pub fn new(filename: &mut [u8; 30], starting_block: u64, ending_block: u64, length: u64, continuations: u8) -> DeletedFileEntry {
        DeletedFileEntry {
            entry_type: 0x1A,
            continuations: continuations,
            timestamp: get_timestamp(),
            starting_block: starting_block,
            ending_block: ending_block,
            length: length,
            filename: *filename
        }
    }
}

#[repr(C, packed)]
struct ContinuationEntry {
    entry_name: [u8; 64]
}

impl ContinuationEntry {
    pub fn new(entry_name: &mut [u8; 64]) -> ContinuationEntry {
        ContinuationEntry {
            entry_name: *entry_name
        }
    }
}

fn u32_as_u8_mut<'a>(src: &'a mut [u32]) -> &'a mut [u8] {
    let dst = unsafe {
        core::slice::from_raw_parts_mut(src.as_mut_ptr() as *mut u8, src.len() * 4)
    };
    dst
}
fn get_timestamp() -> u64 {
    // (SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos() / 15259 / 65536) as u64
    unsafe {
        let timestamp = SYSTEM_INCREMENTER;
        SYSTEM_INCREMENTER += 1;
        timestamp
    }
}