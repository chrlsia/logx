use crate::parser::{LogEntry, LogLevel};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// ─────────────────────────────────────────────
// ErrorGroup — a repeated error message
// and how many times it occurred
// ─────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct ErrorGroup {
    pub message:    String,
    pub count:      usize,
    pub first_seen: Option<DateTime<Utc>>,
    pub last_seen:  Option<DateTime<Utc>>,
}

// ─────────────────────────────────────────────
// TimeBucket — how many events in one minute
// ─────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct TimeBucket {
    pub start:  DateTime<Utc>,
    pub errors: usize,
    pub warns:  usize,
    pub total:  usize,
}

// ─────────────────────────────────────────────
// AnalysisSummary — the final result
// everything the reporter needs to display
// ─────────────────────────────────────────────
#[derive(Debug)]
pub struct AnalysisSummary {
    pub total_lines:  usize,
    pub error_count:  usize,
    pub warn_count:   usize,
    pub info_count:   usize,
    pub debug_count:  usize,
    pub earliest:     Option<DateTime<Utc>>,
    pub latest:       Option<DateTime<Utc>>,
    pub top_errors:   Vec<ErrorGroup>,
    pub top_warns:    Vec<ErrorGroup>,
    pub time_buckets: Vec<TimeBucket>,
    pub spike_at:     Option<DateTime<Utc>>,
    pub spike_count:  usize,
}

// ─────────────────────────────────────────────
// Aggregator
// ─────────────────────────────────────────────
pub struct Aggregator {
    bucket_minutes: i64,
    top_n:          usize,
}

impl Aggregator {
    pub fn new() -> Self {
        Aggregator { bucket_minutes: 1, top_n: 5 }
    }

    // One pass through all entries — update every counter simultaneously
    // Analogy: reading receipts once while updating multiple running totals
    pub fn analyze(&self, entries: &[LogEntry]) -> AnalysisSummary {
        let mut total   = 0usize;
        let mut errors  = 0usize;
        let mut warns   = 0usize;
        let mut infos   = 0usize;
        let mut debugs  = 0usize;
        let mut earliest: Option<DateTime<Utc>> = None;
        let mut latest:   Option<DateTime<Utc>> = None;

        // HashMap<message, ErrorGroup>
        let mut error_groups: HashMap<String, ErrorGroup> = HashMap::new();
        let mut warn_groups:  HashMap<String, ErrorGroup> = HashMap::new();

        // HashMap<bucket_start_unix, TimeBucket>
        let mut bucket_map: HashMap<i64, TimeBucket> = HashMap::new();

        for entry in entries {
            total += 1;

            // Update level counters
            match entry.level {
                LogLevel::Error | LogLevel::Fatal => errors += 1,
                LogLevel::Warn                    => warns  += 1,
                LogLevel::Info                    => infos  += 1,
                LogLevel::Debug | LogLevel::Trace => debugs += 1,
                LogLevel::Unknown                 => {}
            }

            // Update time span
            if let Some(ts) = entry.timestamp {
                earliest = Some(match earliest {
                    None    => ts,
                    Some(e) => if ts < e { ts } else { e },
                });
                latest = Some(match latest {
                    None    => ts,
                    Some(l) => if ts > l { ts } else { l },
                });

                // Place into time bucket
                self.update_bucket(&mut bucket_map, ts, &entry.level);
            }

            // Group errors and warnings by message
            match entry.level {
                LogLevel::Error | LogLevel::Fatal => {
                    self.update_group(&mut error_groups, entry);
                }
                LogLevel::Warn => {
                    self.update_group(&mut warn_groups, entry);
                }
                _ => {}
            }
        }

        // Sort error groups by count descending, take top N
        let mut top_errors: Vec<ErrorGroup> = error_groups.into_values().collect();
        top_errors.sort_by(|a, b| b.count.cmp(&a.count));
        top_errors.truncate(self.top_n);

        let mut top_warns: Vec<ErrorGroup> = warn_groups.into_values().collect();
        top_warns.sort_by(|a, b| b.count.cmp(&a.count));
        top_warns.truncate(self.top_n);

        // Sort buckets by time
        let mut time_buckets: Vec<TimeBucket> = bucket_map.into_values().collect();
        time_buckets.sort_by_key(|b| b.start);

        // Find the spike — bucket with most errors
        let (spike_at, spike_count) = time_buckets
            .iter()
            .max_by_key(|b| b.errors)
            .map(|b| (Some(b.start), b.errors))
            .unwrap_or((None, 0));

        AnalysisSummary {
            total_lines: total,
            error_count: errors,
            warn_count:  warns,
            info_count:  infos,
            debug_count: debugs,
            earliest,
            latest,
            top_errors,
            top_warns,
            time_buckets,
            spike_at,
            spike_count,
        }
    }

    // Snap a timestamp to its bucket boundary
    // e.g. 10:32:45 → bucket starting at 10:32:00
    fn update_bucket(
        &self,
        buckets: &mut HashMap<i64, TimeBucket>,
        ts:      DateTime<Utc>,
        level:   &LogLevel,
    ) {
        let bucket_secs      = self.bucket_minutes * 60;
        let ts_unix          = ts.timestamp();
        let bucket_start_unix = (ts_unix / bucket_secs) * bucket_secs;

        let bucket = buckets.entry(bucket_start_unix).or_insert_with(|| {
            let start = DateTime::from_timestamp(bucket_start_unix, 0).unwrap_or(ts);
            TimeBucket { start, errors: 0, warns: 0, total: 0 }
        });

        bucket.total += 1;
        match level {
            LogLevel::Error | LogLevel::Fatal => bucket.errors += 1,
            LogLevel::Warn                    => bucket.warns  += 1,
            _                                 => {}
        }
    }

    // Insert or update a group in the HashMap
    fn update_group(&self, groups: &mut HashMap<String, ErrorGroup>, entry: &LogEntry) {
        let group = groups.entry(entry.message.clone()).or_insert_with(|| {
            ErrorGroup {
                message:    entry.message.clone(),
                count:      0,
                first_seen: entry.timestamp,
                last_seen:  entry.timestamp,
            }
        });

        group.count += 1;

        if let Some(ts) = entry.timestamp {
            if group.first_seen.map_or(true, |f| ts < f) {
                group.first_seen = Some(ts);
            }
            if group.last_seen.map_or(true, |l| ts > l) {
                group.last_seen = Some(ts);
            }
        }
    }
}