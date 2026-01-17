use std::sync::mpsc::Sender;
use std::io::{self, Write};
use crossterm::event::{Event, KeyCode};
use crossterm::terminal;
use crossterm::ExecutableCommand;

/// Terminal input listener that handles user input and forwards it to a channel
pub struct TerminalInputListener {
    input_tx: Sender<String>,
    history: Vec<String>,
    history_index: usize,
    current_input: String,
}

impl TerminalInputListener {
    /// Create a new TerminalInputListener
    pub fn new(input_tx: Sender<String>) -> Self {
        TerminalInputListener {
            input_tx,
            history: Vec::new(),
            history_index: 0,
            current_input: String::new(),
        }
    }

    /// Run the input listener loop
    /// Handles terminal input, detects interruptions (! prefix), and forwards to channel
    pub fn run(&mut self) -> io::Result<()> {
        // Set up terminal for raw input
        let mut stdout = io::stdout();
        terminal::enable_raw_mode()?;
        stdout.execute(terminal::EnterAlternateScreen)?;

        let mut show_prompt = true;

        loop {
            if show_prompt {
                stdout.execute(terminal::Clear(terminal::ClearType::FromCursorDown))?;
                write!(stdout, "apchat> ")?;
                stdout.flush()?;
                show_prompt = false;
            }

            if let Event::Key(key_event) = crossterm::event::read()? {
                match key_event.code {
                    KeyCode::Char(c) => {
                        // Handle ! prefix for interruptions
                        if c == '!' && self.current_input.is_empty() {
                            self.current_input.push(c);
                        } else {
                            self.current_input.push(c);
                        }
                        write!(stdout, "{}", c)?;
                        stdout.flush()?;
                    },
                    KeyCode::Enter => {
                        if !self.current_input.is_empty() {
                            // Send to channel
                            let input = self.current_input.clone();
                            self.input_tx.send(input.clone()).map_err(|e| {
                                io::Error::new(io::ErrorKind::Other, e)
                            })?;

                            // Save to history if not empty
                            self.save_history();

                            // Reset for next input
                            self.current_input.clear();
                            self.history_index = self.history.len();
                            
                            // Show prompt again
                            show_prompt = true;
                        }
                    },
                    KeyCode::Backspace => {
                        self.current_input.pop();
                        write!(stdout, "{}", 8 as char)?; // Backspace
                        write!(stdout, " ")?; // Space
                        write!(stdout, "{}", 8 as char)?; // Backspace again
                        stdout.flush()?;
                    },
                    KeyCode::Up => {
                        // History navigation
                        if !self.history.is_empty() {
                            if self.history_index > 0 {
                                self.history_index -= 1;
                                self.current_input = self.history[self.history_index].clone();
                                
                                // Clear current line
                                let clear_len = format!("apchat> {}", self.current_input).len();
                                for _ in 0..clear_len {
                                    write!(stdout, "{}", 8 as char)?;
                                    write!(stdout, " ")?;
                                    write!(stdout, "{}", 8 as char)?;
                                }
                                
                                // Write new line
                                write!(stdout, "apchat> {}", self.current_input)?;
                                stdout.flush()?;
                            }
                        }
                    },
                    KeyCode::Down => {
                        // History navigation
                        if !self.history.is_empty() {
                            if self.history_index < self.history.len() - 1 {
                                self.history_index += 1;
                                self.current_input = self.history[self.history_index].clone();
                            } else if self.history_index == self.history.len() - 1 {
                                self.history_index = self.history.len();
                                self.current_input.clear();
                            }
                            
                            // Clear current line
                            let clear_len = format!("apchat> {}", self.current_input).len();
                            for _ in 0..clear_len {
                                write!(stdout, "{}", 8 as char)?;
                                write!(stdout, " ")?;
                                write!(stdout, "{}", 8 as char)?;
                            }
                            
                            // Write new line
                            write!(stdout, "apchat> {}", self.current_input)?;
                            stdout.flush()?;
                        }
                    },
                    KeyCode::Esc => {
                        // Exit on ESC
                        break;
                    },
                    _ => {
                        // Ignore other keys
                    }
                }
            }
        }

        // Cleanup
        stdout.execute(terminal::LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;
        Ok(())
    }

    /// Save current input to history (if not empty and not duplicate of last entry)
    pub fn save_history(&mut self) {
        if !self.current_input.is_empty() {
            // Check if this is a duplicate of the last entry
            if self.history.last().map_or(false, |last| last == &self.current_input) {
                return;
            }
            self.history.push(self.current_input.clone());
        }
    }
}
