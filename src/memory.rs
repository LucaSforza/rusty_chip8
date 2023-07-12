use std::alloc::{alloc, dealloc, Layout};
use std::ptr::NonNull;

const CAPACITY: usize = 4096; // bytes

const FONT: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

pub struct Memory {
    ptr: NonNull<u8>,
}
impl Memory {
    pub fn new() -> Memory {
        let layout = Layout::array::<u8>(CAPACITY).expect("can't allocate");
        let ptr = unsafe { alloc(layout) };
        let ptr = NonNull::new(ptr).expect("the ptr is null");
        let mut mem = Memory { ptr: ptr };
        mem.write_slice(0x50, &FONT);
        mem
    }
    pub fn read_16bit(&self, address: u16) -> Result<u16, &str> {
        if address >= CAPACITY as u16 {
            return Err("overflow ,the address is too big for this memory");
        }
        let ptr = self.ptr.as_ptr();
        unsafe {
            let ptr = ptr.add(address as usize) as *mut u16;
            let val = ptr.read();
            if cfg!(target_endian = "little") {
                return Ok(val.to_be());
            }
            Ok(val)
        }
    }
    pub fn read_slice(&self, address: u16, buff: &mut [u8]) {
        if address as usize + buff.len() >= CAPACITY {
            panic!("overflow ,the slice is too big for this memory starting on this address")
        }
        let mut ptr = unsafe { self.ptr.as_ptr().add(address as usize) };
        buff.iter_mut().for_each(|val| unsafe {
            *val = ptr.read();
            ptr = ptr.add(1)
        })
    }
    pub fn write_slice(&mut self, address: u16, slice: &[u8]) {
        if address as usize + slice.len() >= CAPACITY {
            panic!("overflow ,the slice is too big for this memory starting on this address")
        }
        let mut ptr = unsafe { self.ptr.as_ptr().add(address as usize) };
        slice.iter().for_each(|val| unsafe {
            ptr.write(*val);
            ptr = ptr.add(1);
        })
    }
}
impl Drop for Memory {
    fn drop(&mut self) {
        let layout = Layout::array::<u8>(CAPACITY).expect("can't dellocate");
        let ptr = self.ptr.as_ptr();
        unsafe { dealloc(ptr, layout) };
    }
}
impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}
