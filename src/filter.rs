use chrono::{DateTime, Utc};
use regex::Regex;
use crate::parser::{LogEntry, LogLevel};

// ─────────────────────────────────────────────
// Filter — built once, applied to every line
// Analogy: a sieve you shake once to set up,
// then pour all the logs through.
// ─────────────────────────────────────────────
pub struct Filter {
    pub min_level:    u8,
    pub since:        Option<DateTime<Utc>>,
    pub until:        Option<DateTime<Utc>>,
    pub pattern:      Option<Regex>,
    pub invert_match: bool,
}

impl Filter {
    // Build a Filter from CLI arguments.
    // Returns an error string if any argument is invalid.
    pub fn build(
        level:   Option<&str>,
        since:   Option<&str>,
        until:   Option<&str>,
        grep:    Option<&str>,
        invert:  bool,
    ) -> Result<Self, String> {

        // -- level --
        let min_level = level
            .map(|l| LogLevel::from_str(l).priority())
            .unwrap_or(0);

        // -- since / until --
        let since = since.map(parse_time).transpose()?;
        let until = until.map(parse_time).transpose()?;

        // -- grep (compiled as case-insensitive regex) --
        let pattern = grep
            .map(|p| {
                Regex::new(&format!("(?i){}", p))
                    .map_err(|e| format!("Invalid pattern '{}': {}", p, e))
            })
            .transpose()?;

        Ok(Filter { min_level, since, until, pattern, invert_match: invert })
    }

    // Returns true if this entry should be shown.
    pub fn matches(&self, entry: &LogEntry) -> bool {
        // Gate 1: level
        if entry.level.priority() < self.min_level {
            return false;
        }

        // Gate 2: time range
        if let Some(ts) = entry.timestamp {
            if let Some(since) = self.since {
                if ts < since { return false; }
            }
            if let Some(until) = self.until {
                if ts > until { return false; }
            }
        }

        // Gate 3: regex pattern
        if let Some(ref re) = self.pattern {
            let is_match = re.is_match(&entry.raw);
            if self.invert_match { return is_match == false; }
            if !is_match         { return false; }
        }

        true
    }

    // Human-readable summary of active filters
    pub fn describe(&self) -> Vec<String> {
        let mut parts = vec![];

        if self.min_level > 0 {
            let name = match self.min_level {
                1 => "DEBUG", 2 => "INFO",
                3 => "WARN",  4 => "ERROR",
                5 => "FATAL", _ => "TRACE",
            };
            parts.push(format!("level ≥ {}", name));
        }
        if let Some(s) = self.since {
            parts.push(format!("since {}", s.format("%Y-%m-%d %H:%M:%S")));
        }
        if let Some(u) = self.until {
            parts.push(format!("until {}", u.format("%Y-%m-%d %H:%M:%S")));
        }
        if let Some(ref re) = self.pattern {
            let prefix = if self.invert_match { "not matching" } else { "matching" };
            parts.push(format!("{} /{}/", prefix,
                re.as_str().trim_start_matches("(?i)")));
        }
        parts
    }
}

// ─────────────────────────────────────────────
// Time expression parser
// Handles:
//   "30m"            → 30 minutes ago
//   "2h"             → 2 hours ago
//   "1d"             → 1 day ago
//   "2026-03-17"     → midnight that day
//   "2026-03-17 10:30:00" → exact datetime
// ─────────────────────────────────────────────
fn parse_time(s: &str) -> Result<DateTime<Utc>, String> {
    let s = s.trim();

    // Try shorthand: 30m, 2h, 1d
    if let Some(dt) = try_relative(s) {
        return Ok(dt);
    }

    // Try exact datetime: "2026-03-17 10:30:00"
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Ok(dt.and_utc());
    }

    // Try date only: "2026-03-17"
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(d.and_hms_opt(0, 0, 0).unwrap().and_utc());
    }

    Err(format!(
        "Cannot parse time '{}'. Try: '30m', '2h', '1d', '2026-03-17', '2026-03-17 10:30:00'", s
    ))
}

fn try_relative(s: &str) -> Option<DateTime<Utc>> {
    // Find where digits end and unit begins
    let split = s.find(|c: char| c.is_alphabetic())?;
    let n: i64 = s[..split].parse().ok()?;
    let unit   = &s[split..];

    let duration = match unit {
        "s"              => chrono::Duration::seconds(n),
        "m" | "min"      => chrono::Duration::minutes(n),
        "h" | "hr"       => chrono::Duration::hours(n),
        "d"              => chrono::Duration::days(n),
        _                => return None,
    };

    Some(Utc::now() - duration)
}