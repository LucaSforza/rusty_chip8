use core::fmt;
use std::sync::{Arc, Mutex};

use minifb::{InputCallback, Key};

pub struct DataKeys {
    buf: Mutex<Vec<Key>>,
    last_key_pressed: Mutex<Option<Key>>
}

impl DataKeys {
    pub fn new() -> Self {
        Self {
            buf: Mutex::new(Vec::with_capacity(16)), // because the keyboard has only 16 keys
            last_key_pressed: Mutex::new(None),
        }
    }

    pub fn key_pressed(&self, key: Key) -> bool {
        self.buf.lock().unwrap().iter().any(|k| *k == key)
    }

    pub fn wait_key_pressed(&self) {
        let len = self.buf.lock().unwrap().len();
        while len == self.buf.lock().unwrap().len() { }
    }

    pub fn last_key(&self) -> Option<Key> {
        self.last_key_pressed.lock().unwrap().clone()
    }

    fn push(&self, key: Key) {
        let mut buf = self.buf.lock().unwrap();
        if buf.iter().all(|k| *k != key) {
            buf.push(key);
            *self.last_key_pressed.lock().unwrap() = Some(key);
        }
    }

    fn remove(&self, key: Key) {
        let mut buf = self.buf.lock().unwrap();
        if let Some(index) = buf.iter().position(|x| *x == key) {
            buf.remove(index);
        }
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
            true => self.keys_pressed.push(key),
            false => self.keys_pressed.remove(key),
        }
    }
}

