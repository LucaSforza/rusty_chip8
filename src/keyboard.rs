use core::fmt;
use std::sync::{Arc, Mutex};

use minifb::{InputCallback, Key};

pub const ONEHERTZ: f64 = 1.0/60.0;

pub struct DataKeys {
    buf: Mutex<Vec<Key>>,
    last_key_pressed: Mutex<Option<Key>>,
    new_pressed: Arc<Mutex<bool>>,
    i: Mutex<usize>,
}

impl DataKeys {

    pub fn new(new_pressed: Arc<Mutex<bool>>) -> Self {
        Self {
            buf: Mutex::new(Vec::with_capacity(16)), // because the keyboard has only 16 keys
            last_key_pressed: Mutex::new(None),
            new_pressed: new_pressed,
            i: 0.into(),
        }
    }

    pub fn key_pressed(&self, key: Key) -> bool {
        let buf = self.buf.lock().unwrap();
        buf.iter().any(|k| *k == key)
    }

    pub fn new_press(&self) -> bool {
        *self.new_pressed.lock().unwrap()
    }

    pub fn _last_key(&self) -> Option<Key> {
        self.last_key_pressed.lock().unwrap().clone()
    }

    pub fn reset_new_pressed_flag(&self) {
        *self.new_pressed.lock().unwrap() = false
    }

    fn push(&self, key: Key) {
        let mut buf = self.buf.lock().unwrap();
        *self.new_pressed.lock().unwrap() = true;
        if buf.iter().all(|k| *k != key) {
            buf.push(key);
        }
    }

    fn remove(&self, key: Key) {
        let mut i = self.i.lock().unwrap();
        *i += 1;
        let mut buf = self.buf.lock().unwrap();
        if let Some(index) = buf.iter().position(|x| *x == key) {
            buf.remove(index);
        }
    }

    fn set_new_key(&self, key: Key) {
        *self.last_key_pressed.lock().unwrap() = Some(key)
    } 
}

impl fmt::Debug for DataKeys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{:?}",self.buf)
    }
}

pub struct KeyboardState {
    keys_pressed: Arc<DataKeys>,
}

impl KeyboardState {
    pub fn new(buf: Arc<DataKeys>) -> Box<Self> {
        Box::new(Self {
            keys_pressed: buf,
        })
    }

}

impl InputCallback for KeyboardState {
    fn add_char(&mut self, _uni_char: u32) { }

    fn set_key_state(&mut self, key: Key, state: bool) {
        match state {
            true => {
                if !self.keys_pressed.key_pressed(key) {
                    self.keys_pressed.push(key);
                    self.keys_pressed.set_new_key(key);
                }    
            },
            false => {
                self.keys_pressed.remove(key)
            },
        }
    }
}

