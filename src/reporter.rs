use crate::aggregator::AnalysisSummary;
use colored::Colorize;

pub struct Reporter;

impl Reporter {
    pub fn new() -> Self { Reporter }

    pub fn print(&self, summary: &AnalysisSummary, filename: &str) {
        self.print_header(filename);
        self.print_overview(summary);
        self.print_level_breakdown(summary);
        if !summary.time_buckets.is_empty() {
            self.print_timeline(summary);
        }
        if !summary.top_errors.is_empty() {
            self.print_top_errors(summary);
        }
        if !summary.top_warns.is_empty() {
            self.print_top_warns(summary);
        }
        self.print_footer(summary);
    }

    fn print_header(&self, filename: &str) {
        println!();
        println!("{}", "━".repeat(60).bold());
        println!("  {} {}", "⚡ logx analyze".bold().blue(), "— Log Analysis Report".dimmed());
        println!("  {} {}", "File:".dimmed(), filename.bold());
        println!("{}", "━".repeat(60).bold());
    }

    fn print_overview(&self, s: &AnalysisSummary) {
        println!();
        println!("  {}", "OVERVIEW".bold());
        println!();
        println!("  {:<20} {}", "Total lines:".dimmed(), s.total_lines.to_string().bold());

        match (s.earliest, s.latest) {
            (Some(start), Some(end)) => {
                let secs = (end - start).num_seconds().abs();
                let dur  = format_duration(secs);
                println!(
                    "  {:<20} {} → {}",
                    "Time span:".dimmed(),
                    start.format("%H:%M:%S").to_string().cyan(),
                    end.format("%H:%M:%S").to_string().cyan(),
                );
                println!("  {:<20} {}", "Duration:".dimmed(), dur.bold());
            }
            _ => {
                println!("  {:<20} {}", "Time span:".dimmed(), "no timestamps found".dimmed());
            }
        }
    }

    fn print_level_breakdown(&self, s: &AnalysisSummary) {
        println!();
        println!("  {}", "LEVEL BREAKDOWN".bold());
        println!();

        let total = s.total_lines.max(1);
        let max   = [s.error_count, s.warn_count, s.info_count, s.debug_count]
            .iter().copied().max().unwrap_or(1).max(1);

        let rows = [
            ("ERROR", s.error_count, "red"),
            ("WARN ", s.warn_count,  "yellow"),
            ("INFO ", s.info_count,  "green"),
            ("DEBUG", s.debug_count, "dimmed"),
        ];

        for (label, count, color) in &rows {
            if *count == 0 { continue; }
            let pct = (*count as f64 / total as f64) * 100.0;
            let bar = self.make_bar(*count, max, 30);

            let colored_label = match *color {
                "red"    => label.red().bold().to_string(),
                "yellow" => label.yellow().bold().to_string(),
                "green"  => label.green().to_string(),
                _        => label.dimmed().to_string(),
            };

            println!(
                "  {}  {}  {:>4}  ({:.0}%)",
                colored_label, bar, count, pct
            );
        }
    }

    fn print_timeline(&self, s: &AnalysisSummary) {
        println!();
        println!("  {}", "TIMELINE  (errors per minute)".bold());
        println!();

        let max_errors = s.time_buckets.iter()
            .map(|b| b.errors).max().unwrap_or(1).max(1);

        for bucket in &s.time_buckets {
            let bar      = self.make_bar(bucket.errors, max_errors, 20);
            let time     = bucket.start.format("%H:%M").to_string();
            let is_spike = Some(bucket.start) == s.spike_at && bucket.errors > 0;

            let suffix = if is_spike {
                format!("  ⚠ {} errors ← SPIKE", bucket.errors)
                    .red().bold().to_string()
            } else if bucket.errors > 0 {
                format!("  {} errors", bucket.errors).red().to_string()
            } else {
                "".to_string()
            };

            println!("  {}  {}{}", time.cyan(), bar, suffix);
        }

        if let (Some(spike_time), true) = (s.spike_at, s.spike_count > 0) {
            println!();
            println!(
                "  {} Peak at {} — {} errors in 1 minute",
                "⚠".red().bold(),
                spike_time.format("%H:%M:%S").to_string().bold(),
                s.spike_count.to_string().red().bold()
            );
        }
    }

    fn print_top_errors(&self, s: &AnalysisSummary) {
        println!();
        println!("  {}", "TOP ERRORS".bold().red());
        println!();

        for (i, group) in s.top_errors.iter().enumerate() {
            println!(
                "  {}  {}x  {}",
                format!("#{}", i + 1).dimmed(),
                group.count.to_string().red().bold(),
                group.message.bold()
            );
        }
    }

    fn print_top_warns(&self, s: &AnalysisSummary) {
        println!();
        println!("  {}", "TOP WARNINGS".bold().yellow());
        println!();

        for (i, group) in s.top_warns.iter().enumerate() {
            println!(
                "  {}  {}x  {}",
                format!("#{}", i + 1).dimmed(),
                group.count.to_string().yellow().bold(),
                group.message
            );
        }
    }

    fn print_footer(&self, s: &AnalysisSummary) {
        println!();
        println!("{}", "━".repeat(60).bold());

        let health = if s.error_count == 0 && s.warn_count == 0 {
            "✅  No errors or warnings".green().bold().to_string()
        } else if s.error_count == 0 {
            format!("🟡  No errors, {} warnings to review", s.warn_count)
                .yellow().bold().to_string()
        } else {
            format!("🔴  {} errors detected", s.error_count)
                .red().bold().to_string()
        };

        println!("  {}", health);
        println!("{}", "━".repeat(60).bold());
        println!();
    }

    // Draws a proportional bar: ████████░░░░
    fn make_bar(&self, value: usize, max: usize, width: usize) -> String {
        let filled = if max == 0 { 0 } else { (value * width) / max };
        let empty  = width - filled;
        format!("{}{}",
            "█".repeat(filled).blue(),
            "░".repeat(empty).dimmed()
        )
    }
}

fn format_duration(secs: i64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 { format!("{}h {}m {}s", h, m, s) }
    else if m > 0 { format!("{}m {}s", m, s) }
    else { format!("{}s", s) }
}