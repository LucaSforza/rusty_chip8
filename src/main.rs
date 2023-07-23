mod args;
mod display;
mod instruction;
mod interpreter;
mod memory;
mod registers;

use crate::args::ChipArgs;
use crate::interpreter::Interpreter;

use clap::Parser;
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use rodio::source::{SineWave, Source};
use rodio::{OutputStream, Sink};

use std::fs::File;
use std::path::Path;

const WIDTH: usize = 640;
const HEIGHT: usize = 320;

fn main() {
    let args = ChipArgs::parse();
    let path = Path::new(&args.path);
    if !path.exists() {
        println!("the path don't exists");
        return;
    }

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    // Add a dummy source of the sake of the example.
    let source = SineWave::new(700.0).amplify(0.20);
    sink.append(source);

    let file = File::open(path).unwrap();

    //Creates vector for initializing window with values of each pixel being 0
    let mut buffer = vec![0; WIDTH * HEIGHT];
    let mut interpreter = Interpreter::default();

    interpreter.write_rom_on_mem(file);
    //interpreter.set_debug(true);

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
    while window.is_open() && !window.is_key_down(Key::Escape) {
        window
            .get_keys_released()
            .iter()
            .for_each(|k| interpreter.release_key(*k));

        window
            .get_keys_pressed(KeyRepeat::No)
            .iter()
            .for_each(|k| interpreter.add_key(k));

        if !interpreter.interrupt() {
            interpreter.next();
        } else if let Some(key) = interpreter.get_last_key() {
            interpreter.set_key(*key)
        }

        if interpreter.sound_is_playing() {
            sink.play()
        } else {
            sink.pause()
        }

        interpreter.update_timers();

        //TODO: add sound
        if interpreter.to_draw() {
            window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
            interpreter.draw(&mut buffer);
        }
    }
}
