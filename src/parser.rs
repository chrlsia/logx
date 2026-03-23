use chrono::{DateTime, Utc};
use regex::Regex;
use std::sync::OnceLock;

// ─────────────────────────────────────────────
// LogLevel — represents the severity of a line
// ─────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
    Unknown,
}

impl LogLevel {
    // Converts a string like "ERROR" into LogLevel::Error
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "TRACE"                         => LogLevel::Trace,
            "DEBUG"                         => LogLevel::Debug,
            "INFO" | "INFORMATION"          => LogLevel::Info,
            "WARN" | "WARNING"              => LogLevel::Warn,
            "ERROR" | "ERR"                 => LogLevel::Error,
            "FATAL" | "CRITICAL" | "CRIT"  => LogLevel::Fatal,
            _                               => LogLevel::Unknown,
        }
    }

    // Higher number = more severe
    // Used for --level filtering: "show me warn and above"
    pub fn priority(&self) -> u8 {
        match self {
            LogLevel::Trace   => 0,
            LogLevel::Debug   => 1,
            LogLevel::Info    => 2,
            LogLevel::Warn    => 3,
            LogLevel::Error   => 4,
            LogLevel::Fatal   => 5,
            LogLevel::Unknown => 2,
        }
    }
}

// ─────────────────────────────────────────────
// LogEntry — one parsed log line
// ─────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level:     LogLevel,
    pub message:   String,
    pub timestamp: Option<DateTime<Utc>>,
    pub raw:       String,
    pub source:    Option<String>,    // ← new: which file this came from
}

// ─────────────────────────────────────────────
// Regex compiled once and reused
// ─────────────────────────────────────────────
fn standard_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?P<ts>\d{4}-\d{2}-\d{2}[T\s]\d{2}:\d{2}:\d{2})\s+(?P<level>\w+)\s+(?P<msg>.+)"
        ).unwrap()
    })
}

fn level_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\b(trace|debug|info|warn(?:ing)?|error|fatal|critical)\b").unwrap()
    })
}

// ─────────────────────────────────────────────
// Parser
// ─────────────────────────────────────────────
pub struct Parser;

impl Parser {
    pub fn new() -> Self { Parser }

    pub fn parse_line(&self, raw: &str) -> LogEntry {
        let trimmed = raw.trim();

        // Try JSON first
        if trimmed.starts_with('{') {
            if let Some(entry) = self.try_json(trimmed) {
                return entry;
            }
        }

        // Try standard format: 2026-03-17 10:32:01 ERROR message
        if let Some(entry) = self.try_standard(trimmed) {
            return entry;
        }

        // Fallback: scan for any level keyword
        self.try_plain(trimmed)
    }

    fn try_json(&self, raw: &str) -> Option<LogEntry> {
        let v: serde_json::Value = serde_json::from_str(raw).ok()?;

        let level_str = ["level", "severity", "lvl"]
            .iter()
            .find_map(|k| v[k].as_str())
            .unwrap_or("unknown");

        let message = ["msg", "message", "body"]
            .iter()
            .find_map(|k| v[k].as_str())
            .unwrap_or(raw)
            .to_string();

        let timestamp = ["time", "timestamp", "ts"]
            .iter()
            .find_map(|k| v[k].as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        Some(LogEntry {
            level: LogLevel::from_str(level_str),
            message,
            timestamp,
            raw: raw.to_string(),
            source: None,             // ← new
        })
    }

    fn try_standard(&self, raw: &str) -> Option<LogEntry> {
        let caps = standard_regex().captures(raw)?;
        let level = LogLevel::from_str(&caps["level"]);
        let message = caps["msg"].to_string();
        let timestamp = Self::parse_timestamp(&caps["ts"]);

        Some(LogEntry { level, message, timestamp, raw: raw.to_string(), source: None })
    }

    fn try_plain(&self, raw: &str) -> LogEntry {
        let level = level_regex()
            .captures(raw)
            .map(|c| LogLevel::from_str(&c[1]))
            .unwrap_or(LogLevel::Unknown);

        LogEntry {
            level,
            message: raw.to_string(),
            timestamp: None,
            raw: raw.to_string(),
            source: None,             // ← new
        }
    }

    fn parse_timestamp(s: &str) -> Option<DateTime<Utc>> {
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Some(dt.with_timezone(&Utc));
        }
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
            return Some(dt.and_utc());
        }
        None
    }
}