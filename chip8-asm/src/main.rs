use std::path::PathBuf;
use clap::Parser;

#[derive(Parser)]
#[command(name = "chip8-asm", about = "CHIP-8 assembler")]
struct Cli {
    input: PathBuf,
    #[arg(short = 'o', long, default_value = "a.out.ch8")]
    output: PathBuf,
    #[arg(short = 'l', long)]
    listing: Option<PathBuf>,
}

fn main() {
    let args = Cli::parse();

    let result = match chip8_asm::assemble_file(&args.input) {
        Ok(r) => r,
        Err(errs) => {
            for e in &errs {
                eprintln!("error: {}", e);
            }
            std::process::exit(1);
        }
    };

    if let Err(e) = std::fs::write(&args.output, &result.bytes) {
        eprintln!("error: writing {}: {}", args.output.display(), e);
        std::process::exit(1);
    }
    println!(
        "wrote {} bytes to {}",
        result.bytes.len(),
        args.output.display()
    );

    if let Some(list_path) = args.listing {
        if let Err(e) = std::fs::write(&list_path, result.listing.join("\n")) {
            eprintln!("error: writing {}: {}", list_path.display(), e);
        }
    }
}
