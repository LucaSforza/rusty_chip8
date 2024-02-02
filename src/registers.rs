use std::{fmt, sync::{Arc, Mutex}, thread, time::Duration};

pub struct Registers {
    v: [u8; 16],
    i: u16,
    pc: u16,
    stack: Vec<u16>,
    sound_timer: Arc<Mutex<u8>>,
    delay_timer: Arc<Mutex<u8>>,
    _decrement_thread: thread::JoinHandle<()>,
}
impl Registers {
    pub fn get_v(&self, v_reg: usize) -> u8 {
        self.v[v_reg]
    }
    pub fn set_v(&mut self, v_reg: usize, value: u8) {
        self.v[v_reg] = value;
    }
    pub fn set_flag(&mut self, flag: bool) {
        self.v[15] = flag as u8;
    }
    pub fn get_i(&self) -> u16 {
        self.i
    }
    pub fn set_i(&mut self, address: u16) {
        self.i = address
    }
    pub fn get_pc(&self) -> u16 {
        self.pc
    }
    pub fn set_pc(&mut self, address: u16) {
        self.pc = address
    }
    pub fn increment_pc(&mut self) {
        self.pc += 2
    }
    pub fn stack_push(&mut self) {
        self.stack.push(self.pc)
    }
    pub fn stack_pop(&mut self) {
        match self.stack.pop() {
            Some(pc) => self.pc = pc,
            None => panic!("stack overflow ,the sp can't be < 0"),
        }
    }
    pub fn set_delay(&mut self, val: u8) {
        *self.delay_timer.lock().unwrap() = val
    }
    pub fn get_delay(&self) -> u8 {
        self.delay_timer.lock().unwrap().clone()
    }
    pub fn set_sound(&mut self, val: u8) {
        *self.sound_timer.lock().unwrap() = val
    }
    pub fn get_sound(&self) -> u8 {
        self.sound_timer.lock().unwrap().clone()
    }

}
impl Default for Registers {
    fn default() -> Self {

        let sound_timer: Arc<Mutex<u8>> = Default::default();
        let delay_timer: Arc<Mutex<u8>> = Default::default();
        let sound_t = sound_timer.clone();
        let delay_t = delay_timer.clone();

        let thread = thread::spawn(move || {
            let sound_t = sound_timer.clone();
            let delay_t = delay_timer.clone();
            loop {
                thread::sleep(Duration::from_secs_f64(1.0/60.0));
                {
                    let mut sound = sound_t.lock().unwrap();
                    let mut delay = delay_t.lock().unwrap();
                    if *sound != 0 {
                        *sound -= 1
                    }
                    if *delay != 0 {
                        *delay -= 1
                    }
                }
            }
        });

        Self {
            v: Default::default(),
            i: Default::default(),
            pc: 0x200,
            stack: Vec::with_capacity(16),
            sound_timer: sound_t,
            delay_timer: delay_t,
            _decrement_thread: thread,
        }
    }
}
impl fmt::Display for Registers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v regs: [")?;
        for v in self.v.iter() {
            write!(f, "0x{:X},", v)?
        }
        writeln!(f, "]")?;
        write!(f, "pc: {:X} i:0x{:X}", self.pc, self.i)
    }
}
