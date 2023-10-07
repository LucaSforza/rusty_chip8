
# Chip 8 emulator

This is a simple emulator for the chip8


## Run Locally

Clone the project

```bash
  git clone https://github.com/LucaSforza/rusty_chip8
```

Go to the project directory

```bash
  cd rusty_chip8
```

Compile the emulator

```bash
  cargo build
```

Run the emulator

```bash
  cargo run <path to a bytecode file>
```


## TODO

- Move the functions in 'istruction.rs' inside 'impl Interpreter' in 'interpreter.rs' file
- Add 'cycles by frame' feature like [octo](http://johnearnest.github.io/Octo/).
