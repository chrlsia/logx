use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

pub struct Reader;

impl Reader {
    pub fn new() -> Self {
        Reader
    }

    // Opens a file and returns all non-empty lines.
    // BufReader reads the file in chunks (efficient),
    // then we collect the lines into a Vec.
    pub fn read_lines(&self, path: &Path) -> io::Result<Vec<String>> {
        let file = File::open(path)?;
        let buf  = BufReader::new(file);

        let lines = buf
            .lines()
            .filter_map(|l| l.ok())
            .filter(|l| !l.is_empty())
            .collect();

        Ok(lines)
    }
}