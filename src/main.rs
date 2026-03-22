mod reader;
mod parser;
mod formatter;
mod filter;

use clap::{Parser, Subcommand};
use std::path::Path;
use reader::Reader;
use parser::{Parser as LogParser, LogLevel};   // ← added LogLevel
use formatter::Formatter;

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

        /// Only show lines at or above this level (trace/debug/info/warn/error/fatal)
        #[arg(short, long, value_name = "LEVEL")]
        level: Option<String>,                    // ← new

        /// Highlight lines containing this pattern
        #[arg(short, long, value_name = "PATTERN")]
        grep: Option<String>,                     // ← new

        /// Show only the last N lines
        #[arg(short, long, value_name = "N")]
        tail: Option<usize>,                      // ← new
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Read { file, level, grep, tail } => {   // ← added level, grep, tail
            run_read(file, level, grep, tail);
        }
    }
}

fn run_read(
    file:         String,
    level_filter: Option<String>,    // ← new parameter
    grep:         Option<String>,    // ← new parameter
    tail:         Option<usize>,     // ← new parameter
) {
    let reader    = Reader::new();
    let parser    = LogParser::new();
    let formatter = Formatter::new(grep.clone());   // ← pass grep to formatter

    let min_priority: u8 = level_filter
        .as_deref()
        .map(|l| LogLevel::from_str(l).priority())
        .unwrap_or(0);                              // ← new: 0 = show everything

    let lines: Vec<String> = match reader.read_lines(Path::new(&file)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Error reading {}: {}", file, e);
            std::process::exit(1);
        }
    };

    // Apply --tail before anything else
    let lines_to_show: Vec<&String> = match tail {
        Some(n) => lines.iter().rev().take(n)
                        .collect::<Vec<_>>()
                        .into_iter().rev().collect(),
        None    => lines.iter().collect(),
    };                                              // ← new

    let mut shown  = 0usize;
    let mut errors = 0usize;
    let mut warns  = 0usize;

    for line in &lines_to_show {
        let entry = parser.parse_line(line);

        if entry.level.priority() >= 4 { errors += 1; }
        if entry.level.priority() == 3 { warns  += 1; }

        // Apply --level filter
        if entry.level.priority() < min_priority { continue; }  // ← new

        // Apply --grep filter
        if let Some(ref pat) = grep {
            if !line.to_lowercase().contains(&pat.to_lowercase()) {
                continue;
            }
        }                                                        // ← new

        println!("{}", formatter.format(&entry));
        shown += 1;
    }

    println!();
    println!(
        "  {} lines shown  ·  {} errors  ·  {} warnings",
        shown, errors, warns
    );
}