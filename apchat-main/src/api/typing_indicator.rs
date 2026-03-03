//! Typing Indicator Module
//!
//! Provides animated typing indicators for when the bot is thinking/generating responses.

use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::Duration;
use colored::Colorize;
use apchat_vty::print_heart_red;

/// Animated typing indicator state
#[derive(Debug, Clone)]
pub struct TypingIndicator;

impl TypingIndicator {
    /// Create a new typing indicator
    pub fn new() -> Self {
        Self
    }

    /// Start the typing indicator animation
    /// Returns a handle that can be used to stop it
    pub fn start(&self) -> TypingIndicatorHandle {
        let running = std::sync::Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        
        // Start animation in background thread
        thread::spawn(move || {
            let frames = ["⌨️", "⌨️", "⌨️"];
            let dots = [".", "..", "..."];
            
            for i in 0..30 { // ~5 seconds max animation
                if !running_clone.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                
                let frame = frames[i % frames.len()];
                let dot = dots[i % dots.len()];
                let message = format!("{} Bot is thinking{}{}", 
                    frame.bright_cyan(), 
                    dot.bright_yellow(), 
                    " ".repeat(20)); // Clear space
                
                print_heart_red(&format!("\r{}", message), false);
                thread::sleep(Duration::from_millis(300));
            }
        });
        
        TypingIndicatorHandle {
            running,
        }
    }

    /// Stop the typing indicator
    pub fn stop(handle: &TypingIndicatorHandle) {
        handle.running.store(false, std::sync::atomic::Ordering::Relaxed);
        // Clear the line
        print_heart_red(&format!("\r{}", " ".repeat(50)), false);
        print_heart_red(&format!("\r"), false);
    }
}

/// Handle to control the typing indicator animation
#[derive(Debug, Clone)]
pub struct TypingIndicatorHandle {
    pub running: std::sync::Arc<AtomicBool>,
}

impl Default for TypingIndicatorHandle {
    fn default() -> Self {
        Self {
            running: std::sync::Arc::new(AtomicBool::new(true)),
        }
    }
}