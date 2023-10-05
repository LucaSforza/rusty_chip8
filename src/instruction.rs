use std::fmt;

use crate::{display::Display, display::Sprite, memory::Memory, registers::Registers};
use minifb::Key;
use rand::Rng;

#[derive(Clone)]
pub struct Istruction {
    i: u16,
}
impl Istruction {
    pub fn new(value: u16) -> Istruction {
        return Istruction { i: value };
    }
    pub fn get_op_code(&self) -> u8 {
        (self.i >> 12) as u8
    }
    pub fn get_reg(&self) -> u8 {
        ((self.i & 0x0F00) >> 8) as u8
    }
    pub fn get_2_nibble(&self) -> u8 {
        ((self.i & 0x00F0) >> 4) as u8
    }
    pub fn get_func_code(&self) -> u8 {
        (self.i & 0x000F) as u8
    }
    pub fn get_byte(&self) -> u8 {
        self.i as u8
    }
    pub fn get_addr(&self) -> u16 {
        self.i & 0x0FFF
    }
}
impl Default for Istruction {
    fn default() -> Self {
        Self {
            i: Default::default(),
        }
    }
}
impl fmt::Display for Istruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:X}", self.i)
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

//TODO: move these functions inside 'impl Interpreter'
pub fn jump(istro: Istruction, regs: &mut Registers) {
    regs.set_pc(istro.get_addr())
}

pub fn call_subroutine(istro: Istruction, regs: &mut Registers) {
    regs.stack_push();
    regs.set_pc(istro.get_addr())
}

pub fn skip_if_equal_reg_byte(istro: Istruction, regs: &mut Registers) {
    let x_value = regs.get_v(istro.get_reg() as usize);
    if x_value == istro.get_byte() {
        regs.increment_pc()
    }
}
pub fn skip_if_not_equal_reg_byte(istro: Istruction, regs: &mut Registers) {
    let x_value = regs.get_v(istro.get_reg() as usize);
    if x_value != istro.get_byte() {
        regs.increment_pc()
    }
}
pub fn skip_if_equal_regs(istro: Istruction, regs: &mut Registers) {
    let x = istro.get_reg() as usize;
    let y = istro.get_2_nibble() as usize;
    if regs.get_v(x) == regs.get_v(y) {
        regs.increment_pc()
    }
}

pub fn load_byte(istro: Istruction, regs: &mut Registers) {
    let x = istro.get_reg() as usize;
    regs.set_v(x, istro.get_byte())
}
pub fn add_reg_byte(istro: Istruction, regs: &mut Registers) {
    let x = istro.get_reg() as usize;
    let new_val = istro.get_byte() as u16 + regs.get_v(x) as u16;
    regs.set_v(x, new_val as u8)
}
pub fn move_regs(istro: Istruction, regs: &mut Registers) {
    let x = istro.get_reg();
    let y = istro.get_2_nibble();

    regs.set_v(x as usize, regs.get_v(y as usize))
}

pub fn or_regs(istro: Istruction, regs: &mut Registers) {
    let x = istro.get_reg() as usize;
    let y = istro.get_2_nibble() as usize;

    let new_val = regs.get_v(x) | regs.get_v(y);

    regs.set_v(x, new_val);
    regs.set_flag(false)
}
pub fn and_regs(istro: Istruction, regs: &mut Registers) {
    let x = istro.get_reg() as usize;
    let y = istro.get_2_nibble() as usize;

    let new_val = regs.get_v(x) & regs.get_v(y);

    regs.set_v(x, new_val);
    regs.set_flag(false)
}
pub fn xor_regs(istro: Istruction, regs: &mut Registers) {
    let x = istro.get_reg() as usize;
    let y = istro.get_2_nibble() as usize;

    let new_val = regs.get_v(x) ^ regs.get_v(y);

    regs.set_v(x, new_val);
    regs.set_flag(false)
}
pub fn add_regs(istro: Istruction, regs: &mut Registers) {
    let x = istro.get_reg() as usize;
    let y = istro.get_2_nibble() as usize;

    let x_value = regs.get_v(x);
    let y_value = regs.get_v(y);

    regs.set_v(x, (x_value as u16 + y_value as u16) as u8);
    regs.set_flag(x_value.checked_add(y_value).is_none());
}

pub fn sub_regs(istro: Istruction, regs: &mut Registers) {
    let x = istro.get_reg() as usize;
    let y = istro.get_2_nibble() as usize;

    let x_value = regs.get_v(x);
    let y_value = regs.get_v(y);
    match x_value > y_value {
        true => {
            regs.set_v(x, x_value - y_value);
            regs.set_flag(true);
        }
        false => {
            let x_with_underflow = x_value as u16 + 0b1_0000_0000;
            let result = x_with_underflow - y_value as u16;
            regs.set_v(x, result as u8);
            regs.set_flag(false);
        }
    }
}

pub fn shift_right_regs(istro: Istruction, regs: &mut Registers) {
    let x = istro.get_reg() as usize;
    regs.set_v(x, regs.get_v(istro.get_2_nibble() as usize));
    let x_value = regs.get_v(x);
    regs.set_v(x, x_value >> 1);
    regs.set_flag((x_value & 0x01) != 0);
}

pub fn subn_regs(istro: Istruction, regs: &mut Registers) {
    let x = istro.get_reg() as usize;
    let y = istro.get_2_nibble() as usize;

    let x_value = regs.get_v(x);
    let y_value = regs.get_v(y);
    match y_value > x_value {
        true => {
            regs.set_v(x, y_value - x_value);
            regs.set_flag(true);
        }
        false => {
            let y_with_underflow = y_value as u16 + 0b1_0000_0000;
            let result = y_with_underflow - x_value as u16;
            regs.set_v(x, result as u8);
            regs.set_flag(false);
        }
    }
}

pub fn shift_left_regs(istro: Istruction, regs: &mut Registers) {
    let x = istro.get_reg() as usize;
    regs.set_v(x, regs.get_v(istro.get_2_nibble() as usize));
    let x_value = regs.get_v(x);
    regs.set_v(x, x_value << 1);
    regs.set_flag((x_value & 0x80) != 0);
}

pub fn skip_if_not_equal_regs(istro: Istruction, regs: &mut Registers) {
    let x = istro.get_reg() as usize;
    let y = istro.get_2_nibble() as usize;
    if regs.get_v(x) != regs.get_v(y) {
        regs.increment_pc()
    }
}

pub fn load_addr(istro: Istruction, regs: &mut Registers) {
    regs.set_i(istro.get_addr())
}

pub fn jump_rel_to_0(istro: Istruction, regs: &mut Registers) {
    regs.set_pc(istro.get_addr() + regs.get_v(0) as u16)
}

pub fn rand(istro: Istruction, regs: &mut Registers) {
    let mut rng = rand::thread_rng();
    let random_byte = rng.gen_range(0..256) as u8;
    let bit_mask = istro.get_byte();
    let x = istro.get_reg();
    regs.set_v(x as usize, random_byte & bit_mask)
}

pub fn draw(
    istro: Istruction,
    regs: &mut Registers,
    mem: &Memory,
    disp: &mut Display,
    to_draw: &mut bool,
) {
    let mut buff: Vec<u8> = vec![0; istro.get_func_code() as usize];
    mem.read_slice(regs.get_i(), buff.as_mut_slice());
    let x = regs.get_v(istro.get_reg() as usize);
    let y = regs.get_v(istro.get_2_nibble() as usize);
    let collision = disp.add_sprite(Sprite::from_slice(buff.as_slice(), x, y));
    *to_draw = true;
    regs.set_flag(collision)
}

pub fn skip_pressed(istro: Istruction, regs: &mut Registers, keys_pressed: &[Key]) {
    let key = convert_num_to_key(regs.get_v(istro.get_reg() as usize));
    if keys_pressed.into_iter().any(|k| *k == key) && !keys_pressed.is_empty() {
        regs.increment_pc()
    }
}

pub fn skip_not_pressed(istro: Istruction, regs: &mut Registers, keys_pressed: &[Key]) {
    let key = convert_num_to_key(regs.get_v(istro.get_reg() as usize));
    if keys_pressed.into_iter().all(|k| *k != key) || keys_pressed.is_empty() {
        regs.increment_pc()
    }
}

pub fn read_dalay(istro: Istruction, regs: &mut Registers) {
    regs.set_v(istro.get_reg() as usize, regs.get_delay())
}

pub fn set_sound_timer(istro: Istruction, regs: &mut Registers) {
    regs.set_sound(regs.get_v(istro.get_reg() as usize))
}
pub fn set_delay_timer(istro: Istruction, regs: &mut Registers) {
    regs.set_delay(regs.get_v(istro.get_reg() as usize))
}

pub fn add_i_reg(istro: Istruction, regs: &mut Registers) {
    let x_value = regs.get_v(istro.get_reg() as usize);
    regs.set_i(regs.get_i() + x_value as u16);
}

pub fn get_location_sprite(istro: Istruction, regs: &mut Registers) {
    let x_value = regs.get_v(istro.get_reg() as usize) as u16;
    regs.set_i(0x50 + x_value * 5);
}

pub fn convert_binary_to_dec(istro: Istruction, regs: &Registers, mem: &mut Memory) {
    let mut buff: Vec<u8> = Vec::with_capacity(3);
    let x_value = regs.get_v(istro.get_reg() as usize);
    buff.push(x_value / 100);
    buff.push(x_value / 10 - buff[0] * 10);
    buff.push(x_value - buff[1] * 10 - buff[0] * 100);
    mem.write_slice(regs.get_i(), buff.as_slice())
}

pub fn save_regs(istro: Istruction, regs: &mut Registers, mem: &mut Memory) {
    let x = istro.get_reg() as usize;
    let mut values: Vec<u8> = Vec::with_capacity(x + 1);
    for r in 0..=x {
        values.push(regs.get_v(r));
    }
    mem.write_slice(regs.get_i(), values.as_slice());
    regs.set_i(regs.get_i() + (x as u16) + 1)
}
pub fn load_regs(istro: Istruction, regs: &mut Registers, mem: &Memory) {
    let x = istro.get_reg() as usize;
    let mut buff: Vec<u8> = vec![0; x + 1];
    mem.read_slice(regs.get_i(), buff.as_mut_slice());
    for r in 0..=x {
        regs.set_v(r, buff[r])
    }
}

#[cfg(test)]
mod test {
    use crate::registers::Registers;

    use super::{jump, skip_if_equal_reg_byte, skip_if_equal_regs, Istruction};

    #[test]
    fn test_istruction() {
        let i = Istruction::new(0xABCD);
        assert_eq!(i.get_op_code(), 0x0A);
        assert_eq!(i.get_reg(), 0xB);
        assert_eq!(i.get_2_nibble(), 0xC);
        assert_eq!(i.get_func_code(), 0xD);
        assert_eq!(i.get_byte(), 0xCD);
        assert_eq!(i.get_addr(), 0xBCD)
    }
    #[test]
    fn test_jump() {
        let istro = Istruction::new(0xABCD);
        let mut regs = Registers::default();
        jump(istro, &mut regs);
        assert_eq!(0xBCD, regs.get_pc())
    }
    #[test]
    fn test_skip_if_equal_reg_byte() {
        let istro = Istruction::new(0xABCD);
        let mut regs = Registers::default();
        regs.set_v(0xB, 0xCD);
        skip_if_equal_reg_byte(istro, &mut regs);
        let istro = Istruction::new(0xABDD);
        skip_if_equal_reg_byte(istro, &mut regs);
        assert_eq!(regs.get_pc(), 0x202)
    }
    #[test]
    fn test_skip_if_equal_regs() {
        let istro = Istruction::new(0xABCD);
        let mut regs = Registers::default();
        regs.set_v(0xB, 0xCD);
        regs.set_v(0xC, 0xCD);
        skip_if_equal_regs(istro.clone(), &mut regs);
        regs.set_v(0xC, 0xDD);
        skip_if_equal_regs(istro, &mut regs);
        assert_eq!(regs.get_pc(), 0x202)
    }
}
