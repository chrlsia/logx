use crate::parser::{LogEntry, LogLevel};
use crate::watcher::Watcher;
use crate::parser::Parser as LogParser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode,
               EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::io;
use std::time::Instant;

// ─────────────────────────────────────────────
// App — holds all the state of the TUI
// ─────────────────────────────────────────────
struct App {
    filename:  String,
    entries:   Vec<LogEntry>,  // all parsed log entries
    errors:    usize,
    warns:     usize,
    scroll:    usize,          // how far down we've scrolled
}

impl App {
    fn new(filename: &str) -> Self {
        App {
            filename: filename.to_string(),
            entries:  Vec::new(),
            errors:   0,
            warns:    0,
            scroll:   0,
        }
    }

    fn push(&mut self, entry: LogEntry) {
        if entry.level.priority() >= 4 { self.errors += 1; }
        if entry.level.priority() == 3 { self.warns  += 1; }
        self.entries.push(entry);
    }

    // Always scroll to the bottom when new lines arrive
    fn scroll_to_bottom(&mut self, visible_lines: usize) {
        if self.entries.len() > visible_lines {
            self.scroll = self.entries.len() - visible_lines;
        } else {
            self.scroll = 0;
        }
    }
}

// ─────────────────────────────────────────────
// run_tui — the main entry point
// Called from main.rs when user runs `logx watch`
// ─────────────────────────────────────────────
pub fn run_tui(filename: Option<&str>) -> io::Result<()> {
    // Set up the terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend  = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app — if it crashes, still restore terminal
    let result = run_app(&mut terminal, filename.unwrap_or("stdin"));

    // Always restore the terminal before exiting
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    filename: &str,
) -> io::Result<()> {
    let parser  = LogParser::new();
    let mut watcher = Watcher::new(filename);
    let mut app     = App::new(filename);

    // Load all existing lines first
    for line in watcher.read_all() {
        app.push(parser.parse_line(&line));
    }

    let mut last_poll = Instant::now();

    loop {
        // Draw the current frame
        terminal.draw(|frame| {
            let area = frame.area();

            // Split screen: log view on top, stats bar on bottom
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(3),      // log view — takes all available space
                    Constraint::Length(3),   // stats bar — always 3 lines tall
                ])
                .split(area);

            // ── Log view ──────────────────────────────
            let log_height = chunks[0].height as usize - 2; // -2 for borders

            // Auto-scroll to bottom
            if app.entries.len() > log_height {
                app.scroll = app.entries.len() - log_height;
            }

            let visible: Vec<ListItem> = app.entries
                .iter()
                .skip(app.scroll)
                .take(log_height)
                .map(|e| entry_to_listitem(e))
                .collect();

            let log_block = List::new(visible)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(" logx watch — {} ", app.filename))
                );

            frame.render_widget(log_block, chunks[0]);

            // ── Stats bar ─────────────────────────────
            let stats = Line::from(vec![
                Span::raw("  Lines: "),
                Span::styled(
                    app.entries.len().to_string(),
                    Style::default().add_modifier(Modifier::BOLD)
                ),
                Span::raw("   Errors: "),
                Span::styled(
                    app.errors.to_string(),
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                ),
                Span::raw("   Warnings: "),
                Span::styled(
                    app.warns.to_string(),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                ),
                Span::styled(
                    "        [q] quit   [↑/↓] scroll",
                    Style::default().fg(Color::DarkGray)
                ),
            ]);

            let stats_block = Paragraph::new(stats)
                .block(Block::default().borders(Borders::ALL).title(" Stats "));

            frame.render_widget(stats_block, chunks[1]);
        })?;

        // Poll for keyboard events (non-blocking)
        if event::poll(Watcher::poll_interval())? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    // q or Ctrl+C → quit
                    KeyCode::Char('q') | KeyCode::Char('Q') => break,

                    // Scroll up
                    KeyCode::Up => {
                        if app.scroll > 0 { app.scroll -= 1; }
                    }

                    // Scroll down
                    KeyCode::Down => {
                        let log_height = terminal.size()?.height as usize - 5;
                        if app.scroll + log_height < app.entries.len() {
                            app.scroll += 1;
                        }
                    }

                    _ => {}
                }
            }
        }

        // Poll for new lines every 250ms
        if last_poll.elapsed() >= Watcher::poll_interval() {
            let new_lines = watcher.poll_new();
            if !new_lines.is_empty() {
                for line in new_lines {
                    app.push(parser.parse_line(&line));
                }
                let log_height = terminal.size()?.height as usize - 5;
                app.scroll_to_bottom(log_height);
            }
            last_poll = Instant::now();
        }
    }

    Ok(())
}

// ─────────────────────────────────────────────
// Convert a LogEntry into a colored ratatui ListItem
// ─────────────────────────────────────────────
fn entry_to_listitem(entry: &LogEntry) -> ListItem<'_> {
    let ts = entry.timestamp
        .map(|t| t.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| "        ".to_string());

    let (level_str, level_color) = match entry.level {
        LogLevel::Fatal   => (" FATAL", Color::Red),
        LogLevel::Error   => (" ERROR", Color::Red),
        LogLevel::Warn    => ("  WARN", Color::Yellow),
        LogLevel::Info    => ("  INFO", Color::Green),
        LogLevel::Debug   => (" DEBUG", Color::Cyan),
        LogLevel::Trace   => (" TRACE", Color::DarkGray),
        LogLevel::Unknown => ("   ???", Color::DarkGray),
    };

    let msg_color = match entry.level {
        LogLevel::Fatal | LogLevel::Error => Color::Red,
        LogLevel::Warn                    => Color::Yellow,
        LogLevel::Debug | LogLevel::Trace => Color::DarkGray,
        _                                 => Color::Reset,
    };

    let line = Line::from(vec![
        Span::styled(ts, Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled(level_str, Style::default()
            .fg(level_color)
            .add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(&entry.message, Style::default().fg(msg_color)),
    ]);

    ListItem::new(line)
}