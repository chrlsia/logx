use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::time::Duration;

// ─────────────────────────────────────────────
// Watcher — watches a file for new lines
//
// Analogy: like a security guard checking a door
// every few seconds to see if anyone new arrived.
// ─────────────────────────────────────────────
pub struct Watcher {
    path:     String,
    position: u64,   // how far we've read so far (in bytes)
}

impl Watcher {
    pub fn new(path: &str) -> Self {
        Watcher {
            path:     path.to_string(),
            position: 0,
        }
    }

    // Read ALL lines from the file from the beginning.
    // Called once at startup to load existing content.
    pub fn read_all(&mut self) -> Vec<String> {
        let file = match File::open(&self.path) {
            Ok(f)  => f,
            Err(_) => return vec![],
        };

        let reader = BufReader::new(file);
        let mut lines = Vec::new();
        let mut byte_count: u64 = 0;

        for line in reader.lines() {
            if let Ok(l) = line {
                byte_count += l.len() as u64 + 1; // +1 for newline
                if !l.is_empty() {
                    lines.push(l);
                }
            }
        }

        self.position = byte_count;
        lines
    }

    // Check if any NEW lines have been added since last read.
    // Returns only the new lines.
    // Called repeatedly in the watch loop.
    pub fn poll_new(&mut self) -> Vec<String> {
        let mut file = match File::open(&self.path) {
            Ok(f)  => f,
            Err(_) => return vec![],
        };

        // Jump to where we left off
        if file.seek(SeekFrom::Start(self.position)).is_err() {
            return vec![];
        }

        let reader = BufReader::new(&file);
        let mut new_lines = Vec::new();
        let mut byte_count = self.position;

        for line in reader.lines() {
            if let Ok(l) = line {
                byte_count += l.len() as u64 + 1;
                if !l.is_empty() {
                    new_lines.push(l);
                }
            }
        }

        self.position = byte_count;
        new_lines
    }

    // How long to wait between polls
    pub fn poll_interval() -> Duration {
        Duration::from_millis(250)
    }
}