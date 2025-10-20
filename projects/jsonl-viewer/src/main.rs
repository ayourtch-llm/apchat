use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use serde_json::Value;
use std::{
    fs,
        io::{stdout},
};

struct JsonlEntry {
    content: String,
    parsed: Option<Value>,
    valid: bool,
}

struct App {
    entries: Vec<JsonlEntry>,
    selected_index: usize,
    scroll_offset: usize,
    show_only_invalid: bool,
}

impl App {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            show_only_invalid: false,
        }
    }

    fn load_jsonl(&mut self, file_path: &str) -> Result<()> {
        let content = fs::read_to_string(file_path)?;
        self.entries = content
            .lines()
            .enumerate()
            .map(|(_, line)| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    JsonlEntry {
                        content: String::new(),
                        parsed: None,
                        valid: true,
                    }
                } else {
                    match serde_json::from_str::<Value>(trimmed) {
                        Ok(value) => JsonlEntry {
                            content: line.to_string(),
                            parsed: Some(value),
                            valid: true,
                        },
                        Err(_) => JsonlEntry {
                            content: line.to_string(),
                            parsed: None,
                            valid: false,
                        },
                    }
                }
            })
            .collect();
        
        if !self.entries.is_empty() {
            self.selected_index = 0;
        }
        Ok(())
    }

    fn next_entry(&mut self) {
        let filtered_entries = self.get_filtered_entries();
        if !filtered_entries.is_empty() {
            if let Some(pos) = filtered_entries.iter().position(|&i| i == self.selected_index) {
                if pos + 1 < filtered_entries.len() {
                    self.selected_index = filtered_entries[pos + 1];
                }
            }
        }
        self.scroll_offset = 0;
    }

    fn previous_entry(&mut self) {
        let filtered_entries = self.get_filtered_entries();
        if !filtered_entries.is_empty() {
            if let Some(pos) = filtered_entries.iter().position(|&i| i == self.selected_index) {
                if pos > 0 {
                    self.selected_index = filtered_entries[pos - 1];
                }
            }
        }
        self.scroll_offset = 0;
    }

    fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    fn toggle_invalid(&mut self) {
        self.show_only_invalid = !self.show_only_invalid;
        if self.show_only_invalid {
            if let Some(first_invalid) = self.get_filtered_entries().first() {
                self.selected_index = *first_invalid;
            }
        }
    }

    fn get_filtered_entries(&self) -> Vec<usize> {
        if self.show_only_invalid {
            self.entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| !entry.valid)
                .map(|(i, _)| i)
                .collect()
        } else {
            (0..self.entries.len()).collect()
        }
    }
}

fn draw_entry_list<'a>(entries: &[JsonlEntry], selected: usize, show_only_invalid: bool) -> Vec<Line<'a>> {
    let filtered_entries: Vec<usize> = if show_only_invalid {
        entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| !entry.valid)
            .map(|(i, _)| i)
            .collect()
    } else {
        (0..entries.len()).collect()
    };

    filtered_entries
        .iter()
        .map(|&i| {
            let entry = &entries[i];
            let prefix = if i == selected { "> " } else { "  " };
            let content = if entry.content.len() > 50 {
                format!("{}...", entry.content.chars().take(47).collect::<String>())
            } else {
                entry.content.clone()
            };
            
            let style = if entry.valid {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            };
            
            Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(content, style),
            ])
        })
        .collect()
}

fn draw_detail(entry: &JsonlEntry) -> String {
    if let Some(ref parsed) = entry.parsed {
        match serde_json::to_string_pretty(parsed) {
            Ok(formatted) => formatted,
            Err(_) => entry.content.clone(),
        }
    } else {
        entry.content.clone()
    }
}

fn draw_ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(f.size());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[1]);

    let entry_list = draw_entry_list(&app.entries, app.selected_index, app.show_only_invalid);
    let entry_list_widget = Paragraph::new(entry_list)
        .block(Block::default().borders(Borders::ALL).title("Entries"));
    f.render_widget(entry_list_widget, main_chunks[0]);

    let detail_content = if app.selected_index < app.entries.len() {
        draw_detail(&app.entries[app.selected_index])
    } else {
        String::new()
    };
    let detail_widget = Paragraph::new(detail_content)
        .block(Block::default().borders(Borders::ALL).title("Content"))
        .wrap(Wrap { trim: true })
        .scroll((app.scroll_offset as u16, 0));
    f.render_widget(detail_widget, main_chunks[1]);

    let keys = "Keys: j/k=navigate, d/u=scroll, i=toggle invalid, q=quit";
    let keys_widget = Paragraph::new(keys)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(keys_widget, chunks[2]);
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <jsonl_file>", args[0]);
        return Ok(());
    }

    let mut app = App::new();
    app.load_jsonl(&args[1])?;

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    loop {
        terminal.draw(|f| draw_ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Esc => break,
                KeyCode::Char('j') => app.next_entry(),
                KeyCode::Char('k') => app.previous_entry(),
                KeyCode::Down => app.next_entry(),
                KeyCode::Up => app.previous_entry(),
                KeyCode::Char('d') => app.scroll_down(),
                KeyCode::Char('u') => app.scroll_up(),
                KeyCode::Char('i') => app.toggle_invalid(),
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}