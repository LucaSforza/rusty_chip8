use std::fmt;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;

use minifb::Key;
use rand::rngs::ThreadRng;
use rand::{thread_rng, Rng};

use crate::display::{Display, Sprite};
use crate::keyboard::DataKeys;
use crate::memory::Memory;
use crate::registers::Registers;

#[derive(Clone)]
struct Istruction {
    val: u16,
    opcode: u8,
    reg: u8,
    nibbles: u8,
    func_code: u8,
    addr: u16,
    byte: u8
}
impl Istruction {
    fn new(value: u16) -> Istruction {
        Istruction {
            val: value,
            opcode: (value >> 12) as u8,
            reg: ((value & 0x0F00) >> 8) as u8,
            nibbles: ((value & 0x00F0) >> 4) as u8,
            func_code: (value & 0x000F) as u8,
            addr: value & 0x0FFF,
            byte: value as u8,
        }
    }
}
impl fmt::Debug for Istruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:X}", self.val)
    }
}

pub fn convert_key_to_value(key: Key) -> Option<u8> {
    match key {
        Key::X => Some(0x0),
        Key::Key1 => Some(0x1),
        Key::Key2 => Some(0x2),
        Key::Key3 => Some(0x3),
        Key::Q => Some(0x4),
        Key::W => Some(0x5),
        Key::E => Some(0x6),
        Key::A => Some(0x7),
        Key::S => Some(0x8),
        Key::D => Some(0x9),
        Key::Z => Some(0xA),
        Key::C => Some(0xB),
        Key::Key4 => Some(0xC),
        Key::R => Some(0xD),
        Key::F => Some(0xE),
        Key::V => Some(0xF),
        _ => None,
    }
}

pub fn convert_num_to_key(key: u8) -> Key {
    match key {
        0x0 => Key::X,
        0x1 => Key::Key1,
        0x2 => Key::Key2,
        0x3 => Key::Key3,
        0x4 => Key::Q,
        0x5 => Key::W,
        0x6 => Key::E,
        0x7 => Key::A,
        0x8 => Key::S,
        0x9 => Key::D,
        0xA => Key::Z,
        0xB => Key::C,
        0xC => Key::Key4,
        0xD => Key::R,
        0xE => Key::F,
        0xF => Key::V,
        _ => panic!("key inesistent"),
    }
}

pub struct Interpreter {
    regs: Registers,
    mem: Memory,
    disp: Display,
    to_draw: bool,
    keyboard: Arc<DataKeys>,
    reg: u8,
    r_thread: ThreadRng,
}
impl Interpreter {

    pub fn new(keyboard: Arc<DataKeys>) -> Self {
        Self {
            regs: Default::default(),
            mem: Default::default(),
            disp: Default::default(),
            to_draw: Default::default(),
            keyboard: keyboard,
            reg: Default::default(),
            r_thread: thread_rng(),
        }
    }

    pub fn write_rom_on_mem(&mut self, file: File) {
        let mut data = Vec::new();
        for byte in file.bytes() {
            let byte = byte.unwrap();
            data.push(byte)
        }
        self.mem.write_slice(0x200, data.as_slice())
    }

    pub fn draw(&mut self, buf: &mut [u32]) {
        self.disp.draw(buf);
    }

    pub fn sound_is_playing(&self) -> bool {
        self.regs.get_sound() != 0
    }

    pub fn set_key(&mut self, key: Key) {
        if let Some(key) = convert_key_to_value(key) {
            self.regs.set_v(self.reg as usize, key);
        }
    }

    pub fn to_draw(&self) -> bool {
        self.to_draw
    }

    fn jump(&mut self, istro: Istruction) {
        self.regs.set_pc(istro.addr)
    }

    fn call_subroutine(&mut self, istro: Istruction) {
        self.regs.stack_push();
        self.regs.set_pc(istro.addr)
    }

    fn skip_if_equal_reg_byte(&mut self, istro: Istruction) {
        let x_value = self.regs.get_v(istro.reg as usize);
        if x_value == istro.byte {
            self.regs.increment_pc()
        }
    }

    fn skip_if_not_equal_reg_byte(&mut self, istro: Istruction) {
        let x_value = self.regs.get_v(istro.reg as usize);
        if x_value != istro.byte {
            self.regs.increment_pc()
        }
    }

    fn skip_if_equal_regs(&mut self, istro: Istruction) {
        let x = istro.reg as usize;
        let y = istro.nibbles as usize;
        if self.regs.get_v(x) == self.regs.get_v(y) {
            self.regs.increment_pc()
        }
    }

    fn load_byte(&mut self, istro: Istruction) {
        let x = istro.reg as usize;
        self.regs.set_v(x, istro.byte)
    }

    fn add_reg_byte(&mut self, istro: Istruction) {
        let x = istro.reg as usize;
        let new_val = istro.byte as u16 + self.regs.get_v(x) as u16;
        self.regs.set_v(x, new_val as u8)
    }

    fn move_regs(&mut self, istro: Istruction) {
        let x = istro.reg;
        let y = istro.nibbles;

        self.regs.set_v(x as usize, self.regs.get_v(y as usize))
    }

    fn or_regs(&mut self, istro: Istruction) {
        let x = istro.reg as usize;
        let y = istro.nibbles as usize;

        let new_val = self.regs.get_v(x) | self.regs.get_v(y);

        self.regs.set_v(x, new_val);
        self.regs.set_flag(false)
    }

    fn and_regs(&mut self, istro: Istruction) {
        let x = istro.reg as usize;
        let y = istro.nibbles as usize;

        let new_val = self.regs.get_v(x) & self.regs.get_v(y);

        self.regs.set_v(x, new_val);
        self.regs.set_flag(false)
    }

    fn xor_regs(&mut self, istro: Istruction) {
        let x = istro.reg as usize;
        let y = istro.nibbles as usize;

        let new_val = self.regs.get_v(x) ^ self.regs.get_v(y);

        self.regs.set_v(x, new_val);
        self.regs.set_flag(false)
    }

    fn add_regs(&mut self, istro: Istruction) {
        let x = istro.reg as usize;
        let y = istro.nibbles as usize;

        let x_value = self.regs.get_v(x);
        let y_value = self.regs.get_v(y);

        self.regs.set_v(x, (x_value as u16 + y_value as u16) as u8);
        self.regs.set_flag(x_value.checked_add(y_value).is_none());
    }

    fn sub_regs(&mut self, istro: Istruction) {
        let x = istro.reg as usize;
        let y = istro.nibbles as usize;

        let x_value = self.regs.get_v(x);
        let y_value = self.regs.get_v(y);
        if x_value >= y_value {
            self.regs.set_v(x, x_value - y_value);
            self.regs.set_flag(true);
        } else {
            let x_with_underflow = x_value as u16 + 0b1_0000_0000;
            let result = x_with_underflow - y_value as u16;
            self.regs.set_v(x, result as u8);
            self.regs.set_flag(false);
        }
    }

    fn shift_right_regs(&mut self, istro: Istruction) {
        let x = istro.reg as usize;
        self.regs
            .set_v(x, self.regs.get_v(istro.nibbles as usize));
        let x_value = self.regs.get_v(x);
        self.regs.set_v(x, x_value >> 1);
        self.regs.set_flag((x_value & 0x01) != 0);
    }

    fn subn_regs(&mut self, istro: Istruction) {
        let x = istro.reg as usize;
        let y = istro.nibbles as usize;

        let x_value = self.regs.get_v(x);
        let y_value = self.regs.get_v(y);
        if y_value >= x_value {
            self.regs.set_v(x, y_value - x_value);
            self.regs.set_flag(true);
        } else {
            let y_with_underflow = y_value as u16 + 0b1_0000_0000;
            let result = y_with_underflow - x_value as u16;
            self.regs.set_v(x, result as u8);
            self.regs.set_flag(false);
        }
    }

    fn shift_left_regs(&mut self, istro: Istruction) {
        let x = istro.reg as usize;
        self.regs.set_v(x, self.regs.get_v(istro.nibbles as usize));
        let x_value = self.regs.get_v(x);
        self.regs.set_v(x, x_value << 1);
        self.regs.set_flag((x_value & 0x80) != 0);
    }

    fn skip_if_not_equal_regs(&mut self, istro: Istruction) {
        let x = istro.reg as usize;
        let y = istro.nibbles as usize;
        if self.regs.get_v(x) != self.regs.get_v(y) {
            self.regs.increment_pc()
        }
    }
    fn load_addr(&mut self, istro: Istruction) {
        self.regs.set_i(istro.addr)
    }

    fn jump_rel_to_0(&mut self, istro: Istruction) {
        self.regs.set_pc(istro.addr + self.regs.get_v(0) as u16)
    }

    fn rand(&mut self, istro: Istruction) {
        let random_byte = self.r_thread.gen_range(0..256) as u8;
        let bit_mask = istro.byte;
        let x = istro.reg;
        self.regs.set_v(x as usize, random_byte & bit_mask)
    }

    //TODO: sistemare problema che ho 2 metodo draw
    fn todo_draw(&mut self, istro: Istruction) {
        let mut buff: Vec<u8> = vec![0; istro.func_code as usize];
        self.mem.read_slice(self.regs.get_i(), buff.as_mut_slice());
        let x = self.regs.get_v(istro.reg as usize);
        let y = self.regs.get_v(istro.nibbles as usize);
        let collision = self.disp.add_sprite(Sprite::from_slice(buff.as_slice(), x, y));
        self.to_draw = true;
        self.regs.set_flag(collision)
    }

    //TODO: c'è un bug qua da qualche parte
    fn skip_pressed(&mut self, istro: Istruction) {
        let key = convert_num_to_key(self.regs.get_v(istro.reg as usize));
        let pressed = self.keyboard.key_pressed(key);
        println!("key: {key:?}, pressed: {pressed}");
        if pressed {
            self.regs.increment_pc()
        }
    }

    fn skip_not_pressed(&mut self, istro: Istruction) {
        let key = convert_num_to_key(self.regs.get_v(istro.reg as usize));
        if self.keyboard.key_pressed(key) {
            self.regs.increment_pc()
        }
    }

    fn read_dalay(&mut self, istro: Istruction) {
        self.regs.set_v(istro.reg as usize, self.regs.get_delay())
    }

    fn set_sound_timer(&mut self, istro: Istruction) {
        self.regs.set_sound(self.regs.get_v(istro.reg as usize))
    }

    fn set_delay_timer(&mut self, istro: Istruction) {
        self.regs.set_delay(self.regs.get_v(istro.reg as usize))
    }

    fn add_i_reg(&mut self, istro: Istruction) {
        let x_value = self.regs.get_v(istro.reg as usize);
        self.regs.set_i(self.regs.get_i() + x_value as u16);
    }

    fn get_location_sprite(&mut self, istro: Istruction) {
        let x_value = self.regs.get_v(istro.reg as usize) as u16;
        self.regs.set_i(0x50 + x_value * 5);
    }

    fn convert_binary_to_dec(&mut self, istro: Istruction) {
        let mut buff: Vec<u8> = Vec::with_capacity(3);
        let x_value = self.regs.get_v(istro.reg as usize);
        buff.push(x_value / 100);
        buff.push(x_value / 10 - buff[0] * 10);
        buff.push(x_value - buff[1] * 10 - buff[0] * 100);
        self.mem.write_slice(self.regs.get_i(), buff.as_slice())
    }

    fn save_regs(&mut self, istro: Istruction) {
        let x = istro.reg as usize;
        let mut values: Vec<u8> = Vec::with_capacity(x + 1);
        for r in 0..=x {
            values.push(self.regs.get_v(r));
        }
        self.mem.write_slice(self.regs.get_i(), values.as_slice());
        self.regs.set_i(self.regs.get_i() + (x as u16) + 1)
    }

    fn load_regs(&mut self, istro: Istruction) {
        let x = istro.reg as usize;
        let mut buff: Vec<u8> = vec![0; x + 1];
        self.mem.read_slice(self.regs.get_i(), buff.as_mut_slice());
        for r in 0..=x {
            self.regs.set_v(r, buff[r])
        }
    }

    pub fn next_istr(&mut self) {
        // Fetch instruction

        let istro = Istruction::new(self.mem.read_16bit(self.regs.get_pc()));
        self.regs.increment_pc();
    
        // Decode and execute
        match istro.opcode {
            0x0 => match istro.func_code {
                0x0 => self.disp.clear_display(),
                0xE => self.regs.stack_pop(),
                _ => panic!("instruction non-existent"),
            },
            0x1 => self.jump(istro),
            0x2 => self.call_subroutine(istro),
            0x3 => self.skip_if_equal_reg_byte(istro),
            0x4 => self.skip_if_not_equal_reg_byte(istro),
            0x5 => self.skip_if_equal_regs(istro),
            0x6 => self.load_byte(istro),
            0x7 => self.add_reg_byte(istro),
            0x8 => match istro.func_code {
                0x0 => self.move_regs(istro),
                0x1 => self.or_regs(istro),
                0x2 => self.and_regs(istro),
                0x3 => self.xor_regs(istro),
                0x4 => self.add_regs(istro),
                0x5 => self.sub_regs(istro),
                0x6 => self.shift_right_regs(istro),
                0x7 => self.subn_regs(istro),
                0xE => self.shift_left_regs(istro),
                _ => panic!("instruction non-existent"),
            },
            0x9 => self.skip_if_not_equal_regs(istro),
            0xA => self.load_addr(istro),
            0xB => self.jump_rel_to_0(istro),
            0xC => self.rand(istro),
            0xD => self.todo_draw(istro),
            0xE => match istro.func_code {
                0x1 => self.skip_not_pressed(istro),
                0xE => self.skip_pressed(istro),
                _ => panic!("instruction non-existent"),
            },
            0xF => match istro.byte {
                0x07 => self.read_dalay(istro),
                0x0A => {
                    println!("qua");
                    //TODO: fare in modo che se non c'è nessun tasto premuto di aspettare il tasto premuto
                    self.set_key(self.keyboard.last_key().unwrap());
                    self.reg = istro.reg;
                } // read key
                0x15 => self.set_delay_timer(istro),
                0x18 => self.set_sound_timer(istro),
                0x1E => self.add_i_reg(istro),
                0x29 => self.get_location_sprite(istro),
                0x33 => self.convert_binary_to_dec(istro),
                0x55 => self.save_regs(istro),
                0x65 => self.load_regs(istro),
                _ => panic!("instruction non-existent"),
            },
            _ => panic!("op code non-existent"),
        };
    }
}
