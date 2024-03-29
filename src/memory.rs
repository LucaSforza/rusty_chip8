use std::process::exit;

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
    buf: Vec<u8>,
}
impl Memory {
    pub fn new() -> Memory {
        let mut mem = Memory {
            buf: vec![0;CAPACITY]
        };
        mem.write_slice(0x50, &FONT);
        mem
    }
    pub fn read_16bit(&self, address: u16) -> u16 {
        let address = address as usize;
        u16::from_be_bytes([self.buf[address], self.buf[address + 1]])
    }
    pub fn read_slice(&self, address: u16, buff: &mut [u8]) {
        let start = address as usize;
        let end = start + buff.len();
        buff.copy_from_slice(&self.buf[start..end]);
    }
    pub fn write_slice(&mut self, address: u16, slice: &[u8]) {
        let address = address as usize;
        let len = slice.len();
        if address + len >= self.buf.capacity() {
            eprintln!("[PANIC] memory overflow");
            exit(1);
        }
        self.buf[address..address+len].copy_from_slice(slice);
    }
}
impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}
