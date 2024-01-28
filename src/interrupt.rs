use minifb::Key;

use crate::interpreter::Interpreter;

pub trait Interrupt {
    fn handler(&self, inter: &mut Interpreter) -> Result<(),()>;
}

pub struct KeyInterrupt {
    key: Key,
    release: bool, // True: pressed, False: released
}

impl KeyInterrupt {
    pub fn new(key: Key, release: bool) -> Box<dyn Interrupt> {
        Box::new(KeyInterrupt {
            key: key,
            release: release,
        })
    }
}

impl Interrupt for KeyInterrupt {
    fn handler(&self, inter: &mut Interpreter) -> Result<(),()> {
        match self.release {
            false => inter.add_key(self.key),
            true => inter.release_key(self.key),
        }
        Ok(())
    }
}