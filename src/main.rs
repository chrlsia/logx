mod reader;
mod parser;
mod formatter;
mod filter;
mod aggregator;
mod reporter;
mod correlator;

use clap::{Parser, Subcommand};
use std::path::Path;
use reader::Reader;
use parser::Parser as LogParser;
use formatter::Formatter;
use filter::Filter;
use aggregator::Aggregator;
use reporter::Reporter;
use correlator::Correlator;    // ← new

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

        #[arg(short, long, value_name = "LEVEL")]
        level: Option<String>,

        #[arg(long, value_name = "TIME")]
        since: Option<String>,

        #[arg(long, value_name = "TIME")]
        until: Option<String>,

        #[arg(short, long, value_name = "PATTERN")]
        grep: Option<String>,

        #[arg(short = 'v', long)]
        invert: bool,

        #[arg(short, long, value_name = "N")]
        tail: Option<usize>,
    },

    /// Analyze a log file and show a statistical report
    Analyze {
        #[arg(required = true)]
        file: String,

        #[arg(short, long, value_name = "LEVEL")]
        level: Option<String>,

        #[arg(long, value_name = "TIME")]
        since: Option<String>,

        #[arg(long, value_name = "TIME")]
        until: Option<String>,
    },

    /// Correlate multiple log files into one unified timeline
    Correlate {                                    // ← new command
        /// Two or more log files to correlate
        #[arg(required = true, num_args = 2..)]
        files: Vec<String>,

        #[arg(short, long, value_name = "LEVEL")]
        level: Option<String>,

        #[arg(long, value_name = "TIME")]
        since: Option<String>,

        #[arg(long, value_name = "TIME")]
        until: Option<String>,

        #[arg(short, long, value_name = "PATTERN")]
        grep: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Read { file, level, since, until, grep, invert, tail } => {
            run_read(file, level, since, until, grep, invert, tail);
        }
        Commands::Analyze { file, level, since, until } => {
            run_analyze(file, level, since, until);
        }
        Commands::Correlate { files, level, since, until, grep } => {  // ← new
            run_correlate(files, level, since, until, grep);
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
    let filter = match Filter::build(
        level.as_deref(), since.as_deref(),
        until.as_deref(), grep.as_deref(), invert,
    ) {
        Ok(f)  => f,
        Err(e) => { eprintln!("Error: {}", e); std::process::exit(1); }
    };

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
        Err(e) => { eprintln!("Error reading {}: {}", file, e); std::process::exit(1); }
    };

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

        if !filter.matches(&entry) { continue; }

        println!("{}", formatter.format(&entry));
        shown += 1;
    }

    println!();
    println!("  {} lines shown  ·  {} errors  ·  {} warnings", shown, errors, warns);
}

fn run_analyze(
    file:  String,
    level: Option<String>,
    since: Option<String>,
    until: Option<String>,
) {
    let filter = match Filter::build(
        level.as_deref(), since.as_deref(),
        until.as_deref(), None, false,
    ) {
        Ok(f)  => f,
        Err(e) => { eprintln!("Error: {}", e); std::process::exit(1); }
    };

    let reader = Reader::new();
    let parser = LogParser::new();

    let lines: Vec<String> = match reader.read_lines(Path::new(&file)) {
        Ok(l) => l,
        Err(e) => { eprintln!("Error reading {}: {}", file, e); std::process::exit(1); }
    };

    let entries: Vec<_> = lines.iter()
        .map(|l| parser.parse_line(l))
        .filter(|e| filter.matches(e))
        .collect();

    if entries.is_empty() {
        println!("No entries matched.");
        return;
    }

    let summary  = Aggregator::new().analyze(&entries);
    let reporter = Reporter::new();
    reporter.print(&summary, &file);
}

// ← new function
fn run_correlate(
    files: Vec<String>,
    level: Option<String>,
    since: Option<String>,
    until: Option<String>,
    grep:  Option<String>,
) {
    let filter = match Filter::build(
        level.as_deref(), since.as_deref(),
        until.as_deref(), grep.as_deref(), false,
    ) {
        Ok(f)  => f,
        Err(e) => { eprintln!("Error: {}", e); std::process::exit(1); }
    };

    Correlator::new().run(&files, &filter);
}