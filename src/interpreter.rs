#![allow(dead_code)]
use std::fs::File;
use std::io::Read;

use crate::display::{Display, Sprite};
use crate::instruction::*;
use crate::memory::Memory;
use crate::registers::Registers;

use termion::event::Key;

pub struct Interpreter {
    regs: Registers,
    mem: Memory,
    disp: Display,
    interrupt: bool,
    debug: bool,
    to_draw: bool,
    keys_pressed: Vec<Key>,
    reg: u8,
}
impl Interpreter {
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
    pub fn add_sprite(&mut self, sprite: Sprite) {
        println!("{}", self.disp.add_sprite(sprite));
    }
    pub fn sound_is_playinf(&self) -> bool {
        self.regs.get_sound() != 0
    }
    pub fn delay_is_on(&self) -> bool {
        self.regs.get_delay() != 0
    }
    pub fn interrupt(&self) -> bool {
        self.interrupt
    }
    pub fn set_key(&mut self, key: Key) {
        if let Some(key) = convert_key_to_value(key) {
            self.regs.set_v(self.reg as usize, key);
            self.interrupt = false
        }
    }
    pub fn add_key(&mut self, key: Key) {
        if !self.keys_pressed.contains(&key) {
            self.keys_pressed.push(key);
        }
    }
    pub fn set_debug(&mut self, flag: bool) {
        self.debug = flag
    }
    pub fn to_draw(&self) -> bool {
        self.to_draw
    }
    pub fn release_key(&mut self, key: Key) {
        let mut new_keys: Vec<Key> = Vec::with_capacity(self.keys_pressed.capacity());
        for k in self.keys_pressed.iter() {
            let k = *k;
            if k != key {
                new_keys.push(k)
            }
        }
        self.keys_pressed = new_keys
    }
    pub fn get_last_key(&self) -> Option<&Key> {
        self.keys_pressed.last()
    }
    pub fn next(&mut self) {
        //Fect istruction
        let istro = Istruction::new(self.mem.read_16bit(self.regs.get_pc()).unwrap());
        self.regs.increment_pc();

        if self.debug {
            println!("{}", istro)
        }

        //decode and execute
        match istro.get_op_code() {
            0x0 => match istro.get_func_code() {
                0x0 => self.disp.clear_display(),
                0xE => self.regs.stack_pop(),
                _ => panic!("istruction non-existent"),
            },
            0x1 => jump(istro, &mut self.regs),
            0x2 => call_subroutine(istro, &mut self.regs),
            0x3 => skip_if_equal_reg_byte(istro, &mut self.regs),
            0x4 => skip_if_not_equal_reg_byte(istro, &mut self.regs),
            0x5 => skip_if_equal_regs(istro, &mut self.regs),
            0x6 => load_byte(istro, &mut self.regs),
            0x7 => add_reg_byte(istro, &mut self.regs),
            0x8 => match istro.get_func_code() {
                0x0 => move_regs(istro, &mut self.regs),
                0x1 => or_regs(istro, &mut self.regs),
                0x2 => and_regs(istro, &mut self.regs),
                0x3 => xor_regs(istro, &mut self.regs),
                0x4 => add_regs(istro, &mut self.regs),
                0x5 => sub_regs(istro, &mut self.regs),
                0x6 => shift_right_regs(istro, &mut self.regs),
                0x7 => subn_regs(istro, &mut self.regs),
                0xE => shift_left_regs(istro, &mut self.regs),
                _ => panic!("istruction non-existent"),
            },
            0x9 => skip_if_not_equal_regs(istro, &mut self.regs),
            0xA => load_addr(istro, &mut self.regs),
            0xB => jump_rel_to_0(istro, &mut self.regs),
            0xC => rand(istro, &mut self.regs),
            0xD => draw(
                istro,
                &mut self.regs,
                &self.mem,
                &mut self.disp,
                &mut self.to_draw,
            ),
            0xE => match istro.get_func_code() {
                0x1 => skip_not_pressed(istro, &mut self.regs, self.keys_pressed.as_slice()),
                0xE => skip_pressed(istro, &mut self.regs, self.keys_pressed.as_slice()),
                _ => panic!("istruction non-existent"),
            },
            0xF => match istro.get_byte() {
                0x07 => read_dalay(istro, &mut self.regs),
                0x0A => {
                    self.interrupt = true;
                    self.reg = istro.get_reg()
                } // read key
                0x15 => set_delay_timer(istro, &mut self.regs),
                0x18 => set_sound_timer(istro, &mut self.regs),
                0x1E => add_i_reg(istro, &mut self.regs),
                0x29 => get_location_sprite(istro, &mut self.regs),
                0x33 => convert_binary_to_dec(istro, &self.regs, &mut self.mem),
                0x55 => save_regs(istro, &mut self.regs, &mut self.mem),
                0x65 => load_regs(istro, &mut self.regs, &self.mem),
                _ => panic!("istruction non-existent"),
            },
            _ => panic!("op code non-existent"),
        }

        self.regs.decrement_delay();
        self.regs.decrement_sound();

        if self.debug {
            println!("{}", self.regs)
        }
    }
}
impl Default for Interpreter {
    fn default() -> Self {
        Self {
            regs: Default::default(),
            mem: Default::default(),
            disp: Default::default(),
            interrupt: Default::default(),
            debug: Default::default(),
            to_draw: Default::default(),
            keys_pressed: Default::default(),
            reg: Default::default(),
        }
    }
}
