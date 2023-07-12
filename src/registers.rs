use std::fmt;

pub struct Registers {
    v: [u8; 16],
    i: u16,
    pc: u16,
    sp: u8,
    stack: [u16; 16],
    sound_timer: u8,
    delay_timer: u8,
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
        if self.sp == 16 {
            panic!("stack overflow ,the sp can't be >= 16");
        }
        self.stack[self.sp as usize] = self.pc;
        self.sp += 1;
    }
    pub fn stack_pop(&mut self) {
        match self.sp.checked_sub(1) {
            Some(new_sp) => {
                self.pc = self.stack[new_sp as usize];
                self.sp = new_sp;
            }
            None => panic!("stack overflow ,the sp can't be < 0"),
        }
    }
    pub fn set_delay(&mut self, val: u8) {
        self.delay_timer = val
    }
    pub fn get_delay(&self) -> u8 {
        self.delay_timer
    }
    pub fn set_sound(&mut self, val: u8) {
        self.sound_timer = val
    }
    pub fn get_sound(&self) -> u8 {
        self.sound_timer
    }
    pub fn decrement_sound(&mut self) {
        if self.sound_timer != 0 {
            self.sound_timer -= 1
        }
    }
    pub fn decrement_delay(&mut self) {
        if self.delay_timer != 0 {
            self.delay_timer -= 1
        }
    }
}
impl Default for Registers {
    fn default() -> Self {
        Self {
            v: Default::default(),
            i: Default::default(),
            pc: 0x200,
            sp: Default::default(),
            stack: Default::default(),
            sound_timer: Default::default(),
            delay_timer: Default::default(),
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

#[cfg(test)]
mod test {
    use super::Registers;

    #[test]
    fn test_v_regs() {
        let mut regs = Registers::default();
        for i in 0..16 {
            regs.set_v(i, i as u8);
        }
        for i in 0..16 {
            assert_eq!(regs.get_v(i), i as u8)
        }
    }
    #[test]
    fn test_flag_reg() {
        let mut regs = Registers::default();
        regs.set_flag(true);
        assert_eq!(regs.get_v(15), 1);
        regs.set_flag(false);
        assert_eq!(regs.get_v(15), 0);
    }
    #[test]
    fn test_pc() {
        let mut regs = Registers::default();
        regs.increment_pc();
        regs.increment_pc();
        assert_eq!(regs.get_pc(), 0x204)
    }
    #[test]
    fn test_stack() {
        let mut regs = Registers::default();
        (0..16).for_each(|_| {
            regs.stack_push();
            regs.increment_pc()
        });
        (256..272).rev().for_each(|n| {
            regs.stack_pop();
            assert_eq!(regs.get_pc(), n << 1)
        })
    }
}
