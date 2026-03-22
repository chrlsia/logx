mod reader;
mod parser;
mod formatter;

use clap::{Parser, Subcommand};
use std::path::Path;          // ← add
use reader::Reader;           // ← add
use parser::Parser as LogParser;   // ← new

#[derive(Parser)]
#[command(name = "logx", about = "⚡ Universal log analyzer CLI", version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Read and display a log file
    Read {
        #[arg(required = true)]
        file: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Read { file } => {
            run_read(file);   // ← change
        }
    }
}

// ← everything below is new
fn run_read(file: String) {
    let reader = Reader::new();
    let parser  = LogParser::new();   // ← new

    let lines: Vec<String> = match reader.read_lines(Path::new(&file)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Error reading {}: {}", file, e);
            std::process::exit(1);
        }
    };

    for line in &lines {
        let entry = parser.parse_line(line);   // ← new
        println!("{}", line);
        println!("{:?} | {}", entry.level, entry.message);   // ← new
    }

    println!("\n  {} lines read", lines.len());
}