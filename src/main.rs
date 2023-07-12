mod args;
mod display;
mod instruction;
mod interpreter;
mod memory;
mod registers;

use crate::args::ChipArgs;
use crate::instruction::{convert_key_to_value, convert_num_to_key};
use crate::interpreter::Interpreter;

use clap::Parser;
use minifb::{Window, WindowOptions};
use termion::event::Key;
use termion::input::TermRead;

use std::fs::File;
use std::io::{stdin, Stdin};
use std::path::Path;

const WIDTH: usize = 640;
const HEIGHT: usize = 320;

fn read_keys(keys: &mut [Option<Key>], stdin: Stdin) {
    keys.iter_mut().for_each(|key| *key = None);
    let handle = stdin.lock();
    let mut it = handle.keys();

    loop {
        let b = it.next();
        match b {
            Some(x) => match x {
                Ok(key) => {
                    let i = convert_key_to_value(key).unwrap();
                    keys[i as usize] = Some(key)
                }
                _ => {}
            },
            None => break,
        }
    }
}

fn main() {
    let args = ChipArgs::parse();
    let path = Path::new(&args.path);
    if !path.exists() {
        println!("the path don't exists");
        return;
    }

    let file = File::open(path).unwrap();

    //Creates vector for initializing window with values of each pixel being 0
    let mut buffer = vec![0; WIDTH * HEIGHT];
    let mut interpreter = Interpreter::default();

    interpreter.write_rom_on_mem(file);
    interpreter.set_debug(true);

    //Defines variable for window data
    let mut window = Window::new(
        "CHIP-8 interpreter in Rust", //Name of window
        WIDTH,                        //Sets width as width variable
        HEIGHT,                       //Sets height as height variable
        WindowOptions::default(),     //Sets the window options to defaults
    )
    //Panic method to handle errors when working with result object
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    window.limit_update_rate(Some(std::time::Duration::from_micros(1300)));

    //While loop for when the window is open and the escape key is not pressed
    while window.is_open() {
        let stdin = stdin();
        let mut keys_pressed: [Option<Key>; 16] = Default::default();
        read_keys(keys_pressed.as_mut_slice(), stdin);
        println!("{:?}", keys_pressed);
        keys_pressed
            .iter()
            .enumerate()
            .for_each(|(i, key)| match *key {
                Some(key) => interpreter.add_key(key),
                None => interpreter.release_key(convert_num_to_key(i as u8)),
            });

        if !interpreter.interrupt() {
            interpreter.next();
        } else if let Some(key) = interpreter.get_last_key() {
            interpreter.set_key(*key)
        }

        //TODO: add sound
        if interpreter.to_draw() {
            window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
            interpreter.draw(&mut buffer);
        }
    }
}
