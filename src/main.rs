mod reader;
mod parser;
mod formatter;
mod filter;
mod aggregator;
mod reporter;

use clap::{Parser, Subcommand};
use std::path::Path;
use reader::Reader;
use parser::Parser as LogParser;
use formatter::Formatter;
use filter::Filter;                // ← new

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
        level: Option<String>,

        /// Only show lines after this time (e.g. 30m, 2h, 1d, "2026-03-17 10:00:00")
        #[arg(long, value_name = "TIME")]
        since: Option<String>,             // ← new

        /// Only show lines before this time
        #[arg(long, value_name = "TIME")]
        until: Option<String>,             // ← new

        /// Show lines matching this pattern (regex supported)
        #[arg(short, long, value_name = "PATTERN")]
        grep: Option<String>,

        /// Invert grep — show lines that do NOT match
        #[arg(short = 'v', long)]
        invert: bool,                      // ← new

        /// Show only the last N lines
        #[arg(short, long, value_name = "N")]
        tail: Option<usize>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Read { file, level, since, until, grep, invert, tail } => {  // ← updated
            run_read(file, level, since, until, grep, invert, tail);
        }
    }
}

fn run_read(
    file:   String,
    level:  Option<String>,
    since:  Option<String>,
    until:  Option<String>,
    grep:   Option<String>,
    invert: bool,
    tail:   Option<usize>,
) {
    // Build the filter — validate all arguments up front
    // before opening any file
    let filter = match Filter::build(
        level.as_deref(),
        since.as_deref(),
        until.as_deref(),
        grep.as_deref(),
        invert,
    ) {
        Ok(f)  => f,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    // Print active filters as a header
    let desc = filter.describe();
    if !desc.is_empty() {
        println!("Filters: {}", desc.join("  ·  "));
        println!();
    }

    let reader    = Reader::new();
    let parser    = LogParser::new();
    let formatter = Formatter::new(grep.clone());

    let lines: Vec<String> = match reader.read_lines(Path::new(&file)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Error reading {}: {}", file, e);
            std::process::exit(1);
        }
    };

    // Apply --tail before filtering
    let lines_to_show: Vec<&String> = match tail {
        Some(n) => lines.iter().rev().take(n)
                        .collect::<Vec<_>>()
                        .into_iter().rev().collect(),
        None    => lines.iter().collect(),
    };

    let mut shown  = 0usize;
    let mut errors = 0usize;
    let mut warns  = 0usize;

    for line in &lines_to_show {
        let entry = parser.parse_line(line);

        if entry.level.priority() >= 4 { errors += 1; }
        if entry.level.priority() == 3 { warns  += 1; }

        // Use the filter instead of manual checks
        if !filter.matches(&entry) { continue; }   // ← new

        println!("{}", formatter.format(&entry));
        shown += 1;
    }

    println!();
    println!(
        "  {} lines shown  ·  {} errors  ·  {} warnings",
        shown, errors, warns
    );
}