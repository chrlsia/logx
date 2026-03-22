use colored::Colorize;
use crate::parser::{LogEntry, LogLevel};

pub struct Formatter {
    pub grep: Option<String>,
}

impl Formatter {
    pub fn new(grep: Option<String>) -> Self {
        Formatter { grep }
    }

    pub fn format(&self, entry: &LogEntry) -> String {
        let ts = entry.timestamp
            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "                   ".to_string());

        let level = self.format_level(&entry.level);
        let message = self.format_message(&entry.message, &entry.level);

        format!("{}  {}  {}", ts.dimmed(), level, message)
    }

    fn format_level(&self, level: &LogLevel) -> String {
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

    fn format_message(&self, msg: &str, level: &LogLevel) -> String {
        let colored = match level {
            LogLevel::Error | LogLevel::Fatal => msg.red().to_string(),
            LogLevel::Warn                    => msg.yellow().to_string(),
            LogLevel::Debug | LogLevel::Trace => msg.dimmed().to_string(),
            _                                 => msg.normal().to_string(),
        };

        // Highlight grep matches if pattern is set
        match &self.grep {
            None => colored,
            Some(pat) => highlight(&colored, pat),
        }
    }
}

// Highlights all occurrences of pat inside text
fn highlight(text: &str, pat: &str) -> String {
    let lower_text = text.to_lowercase();
    let lower_pat  = pat.to_lowercase();

    if !lower_text.contains(&lower_pat) {
        return text.to_string();
    }

    let mut result    = String::new();
    let mut remaining = text;

    while let Some(pos) = remaining.to_lowercase().find(&lower_pat) {
        result.push_str(&remaining[..pos]);
        let matched = &remaining[pos..pos + pat.len()];
        result.push_str(&matched.on_yellow().black().to_string());
        remaining = &remaining[pos + pat.len()..];
    }

    result.push_str(remaining);
    result
}