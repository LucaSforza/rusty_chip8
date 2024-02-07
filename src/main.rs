mod display;
mod keyboard;
mod interpreter;
mod memory;
mod registers;

use std::process::exit;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use std::{env, fs::File, path::Path};

use crate::interpreter::Interpreter;

use keyboard::{DataKeys, KeyboardState};
use minifb::{Key, Window, WindowOptions};
use rodio::source::{SineWave, Source};
use rodio::{OutputStream, Sink};

const WIDTH: usize = 640;
const HEIGHT: usize = 320;
const FRAME4FRAME: usize = 50;

fn main() {
    let mut args = env::args();
    let program = args.next().unwrap();
    let path = args.next().unwrap_or_else(|| {
        eprintln!("[ERROR] no path provided");
        eprintln!("[ERROR] Usage: {program} <path>");
        std::process::exit(1);
    });
    let path = Path::new(&path);
    if !path.exists() {
        eprintln!("[ERROR] file '{}' not found", path.to_str().unwrap());
        std::process::exit(2);
    }

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let source = SineWave::new(700.0).amplify(0.20);
    sink.append(source);

    let file = File::open(path).unwrap();

    //Creates vector for initializing window with values of each pixel being 0
    let mut buffer = vec![0; WIDTH * HEIGHT];
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
        eprintln!("[MINIFB ERROR] {}", e);
        exit(1)
    });

    let new_key_press: Arc<Mutex<bool>> = Default::default();

    let data_keys = Arc::new(DataKeys::new(new_key_press.clone()));
    let keyboard = KeyboardState::new(data_keys.clone());

    let mut interpreter = Interpreter::new(data_keys);
    window.set_input_callback(keyboard);

    interpreter.write_rom_on_mem(file);

    let mut cycles_count = 0;

    let mut fps = 0;
    let mut last_time = SystemTime::now();

    //While loop for when the window is open and the escape key is not pressed
    while window.is_open() && !window.is_key_down(Key::Escape) {
        cycles_count += 1;
        interpreter.next_istr();

        if interpreter.sound_is_playing() {
            sink.play()
        } else {
            sink.pause()
        }


        if cycles_count == FRAME4FRAME {
            if interpreter.to_draw() {
                interpreter.draw(&mut buffer);
                window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
            }
            let now = SystemTime::now();
            let duration = now.duration_since(last_time).unwrap();
            fps += 1;
            if duration.as_secs_f64() >= 1.0 {
                println!("FPS: {fps}");
                last_time = now;
                fps = 0;
            }
            cycles_count = 0;
        }
    }
}
