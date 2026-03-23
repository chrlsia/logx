use crate::parser::{LogEntry, LogLevel};
use crate::filter::Filter;
use crate::reader::Reader;
use crate::parser::Parser as LogParser;
use colored::Colorize;
use std::path::Path;

// ─────────────────────────────────────────────
// One entry tagged with its service color index
// ─────────────────────────────────────────────
struct Tagged {
    entry:       LogEntry,
    color_index: usize,
}

// Colors assigned to services in order
const COLORS: &[&str] = &["cyan", "magenta", "yellow", "green", "blue"];

pub struct Correlator;

impl Correlator {
    pub fn new() -> Self { Correlator }

    pub fn run(&self, files: &[String], filter: &Filter) {
        let parser = LogParser::new();
        let reader = Reader::new();

        // Collect service names (filename without extension)
        let service_names: Vec<String> = files.iter()
            .map(|f| service_name(f))
            .collect();

        // Print legend
        self.print_legend(&service_names);

        // Read and parse all files, tagging each entry with
        // its source service name and color index
        let mut all: Vec<Tagged> = Vec::new();

        for (i, file) in files.iter().enumerate() {
            let lines: Vec<String> = match reader.read_lines(Path::new(file)) {
                Ok(l)  => l,
                Err(e) => {
                    eprintln!("Error reading {}: {}", file, e);
                    continue;
                }
            };

            for line in &lines {
                // Parse the line and set its source
                let mut entry = parser.parse_line(line);
                entry.source  = Some(service_names[i].clone());

                if filter.matches(&entry) {
                    all.push(Tagged { entry, color_index: i % COLORS.len() });
                }
            }
        }

        if all.is_empty() {
            println!("No entries matched.");
            return;
        }

        // Sort all entries by timestamp
        // Entries without timestamps go to the end
        all.sort_by(|a, b| {
            match (a.entry.timestamp, b.entry.timestamp) {
                (Some(ta), Some(tb)) => ta.cmp(&tb),
                (Some(_),  None)     => std::cmp::Ordering::Less,
                (None,     Some(_))  => std::cmp::Ordering::Greater,
                (None,     None)     => std::cmp::Ordering::Equal,
            }
        });

        // Print the unified timeline
        self.print_timeline(&all);
    }

    fn print_legend(&self, services: &[String]) {
        println!();
        println!("{}", "━".repeat(60).bold());
        println!("  {} {}", "⚡ logx correlate".bold().blue(), "— Multi-Service Timeline".dimmed());
        println!();
        print!("  Services: ");
        for (i, name) in services.iter().enumerate() {
            print!("{}  ", colorize(name, i % COLORS.len()).bold());
        }
        println!();
        println!("{}", "━".repeat(60).bold());
        println!();
        println!(
            "  {}  {}  {}  {}",
            "TIME    ".dimmed(),
            "SERVICE ".dimmed(),
            "LEVEL".dimmed(),
            "MESSAGE".dimmed()
        );
        println!("{}", "─".repeat(60).dimmed());
        println!();
    }

    fn print_timeline(&self, entries: &[Tagged]) {
        for tagged in entries {
            let entry = &tagged.entry;

            let ts = entry.timestamp
                .map(|t| t.format("%H:%M:%S").to_string())
                .unwrap_or_else(|| "        ".to_string());

            let service = entry.source.as_deref().unwrap_or("unknown");
            let service_tag = format!("[{:<8}]", &service[..service.len().min(8)]);
            let service_colored = colorize(&service_tag, tagged.color_index);

            let level = format_level(&entry.level);

            let message = match entry.level {
                LogLevel::Error | LogLevel::Fatal => entry.message.red().to_string(),
                LogLevel::Warn                    => entry.message.yellow().to_string(),
                LogLevel::Debug | LogLevel::Trace => entry.message.dimmed().to_string(),
                _                                 => entry.message.normal().to_string(),
            };

            println!(
                "  {}  {}  {}  {}",
                ts.dimmed(),
                service_colored,
                level,
                message
            );
        }

        println!();
        println!("{}", "─".repeat(60).dimmed());
        println!(
            "  {} entries shown",
            entries.len().to_string().bold()
        );
        println!();
    }
}

// ─────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────

// Derives a short service name from a file path
// "logs/api-service.log" → "api-service"
fn service_name(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn colorize(s: &str, index: usize) -> colored::ColoredString {
    match COLORS[index % COLORS.len()] {
        "cyan"    => s.cyan(),
        "magenta" => s.magenta(),
        "yellow"  => s.yellow(),
        "green"   => s.green(),
        "blue"    => s.blue(),
        _         => s.normal(),
    }
}

fn format_level(level: &LogLevel) -> String {
    match level {
        LogLevel::Trace   => " TRACE".dimmed().to_string(),
        LogLevel::Debug   => " DEBUG".cyan().to_string(),
        LogLevel::Info    => "  INFO".green().to_string(),
        LogLevel::Warn    => "  WARN".yellow().bold().to_string(),
        LogLevel::Error   => " ERROR".red().bold().to_string(),
        LogLevel::Fatal   => " FATAL".on_red().white().bold().to_string(),
        LogLevel::Unknown => "   ???".dimmed().to_string(),
    }
}