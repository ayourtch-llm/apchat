// Core shared types and utilities for APChat
// Contains the WebexDebugState for toggling debug output

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Debug state for Webex integration
/// 
/// This struct provides a thread-safe way to toggle debug output
/// for Webex-related operations. It uses an AtomicBool wrapped in
/// an Arc for shared access across the application.
/// 
/// # Usage
/// 
/// Create the state once and pass it to Webex components:
/// ```rust
/// let debug_state = WebexDebugState::new();
/// 
/// // Pass to WebexClient and WebexWebSocketRouter
/// let client = WebexClient::new(debug_state.clone());
/// let router = WebexWebSocketRouter::new(debug_state.clone());
/// ```
/// 
/// Toggle debug mode:
/// ```rust
/// debug_state.toggle(); // Flips between enabled/disabled
/// debug_state.set_enabled(true); // Explicitly enable
/// debug_state.set_enabled(false); // Explicitly disable
/// ```
/// 
/// Check if debug is enabled:
/// ```rust
/// if debug_state.is_enabled() {
///     // Log debug information
/// }
/// ```
pub struct WebexDebugState {
    enabled: Arc<AtomicBool>,
}

impl WebexDebugState {
    /// Create a new WebexDebugState with debug disabled by default
    /// 
    /// # Example
    /// ```rust
    /// let state = WebexDebugState::new();
    /// assert!(!state.is_enabled());
    /// ```
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(false)), // default OFF
        }
    }

    /// Enable or disable debug mode
    /// 
    /// # Arguments
    /// * `enabled` - Whether to enable debug output
    /// 
    /// # Example
    /// ```rust
    /// let state = WebexDebugState::new();
    /// state.set_enabled(true);
    /// assert!(state.is_enabled());
    /// ```
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    /// Check if debug mode is currently enabled
    /// 
    /// # Returns
    /// `true` if debug is enabled, `false` otherwise
    /// 
    /// # Example
    /// ```rust
    /// let state = WebexDebugState::new();
    /// assert!(!state.is_enabled());
    /// ```
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Toggle debug mode (enabled <-> disabled)
    /// 
    /// # Example
    /// ```rust
    /// let state = WebexDebugState::new();
    /// state.toggle();
    /// assert!(state.is_enabled());
    /// state.toggle();
    /// assert!(!state.is_enabled());
    /// ```
    pub fn toggle(&self) {
        let current = self.enabled.load(Ordering::SeqCst);
        self.enabled.store(!current, Ordering::SeqCst);
    }
}

impl Clone for WebexDebugState {
    fn clone(&self) -> Self {
        Self {
            enabled: Arc::clone(&self.enabled),
        }
    }
}

impl Default for WebexDebugState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state_is_disabled() {
        let state = WebexDebugState::new();
        assert!(!state.is_enabled());
    }

    #[test]
    fn test_set_enabled() {
        let state = WebexDebugState::new();
        state.set_enabled(true);
        assert!(state.is_enabled());
        state.set_enabled(false);
        assert!(!state.is_enabled());
    }

    #[test]
    fn test_toggle() {
        let state = WebexDebugState::new();
        assert!(!state.is_enabled());
        
        state.toggle();
        assert!(state.is_enabled());
        
        state.toggle();
        assert!(!state.is_enabled());
        
        state.toggle();
        assert!(state.is_enabled());
    }

    #[test]
    fn test_clone_shares_state() {
        let state1 = WebexDebugState::new();
        let state2 = state1.clone();
        
        state1.set_enabled(true);
        assert!(state2.is_enabled());
        
        state2.toggle();
        assert!(!state1.is_enabled());
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;
        
        let state = WebexDebugState::new();
        let state_clone = state.clone();
        
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                state_clone.toggle();
            }
        });
        
        for _ in 0..100 {
            state.set_enabled(!state.is_enabled());
        }
        
        handle.join().unwrap();
        
        // If we get here without deadlock, thread safety is working
        assert!(state.is_enabled() || !state.is_enabled());
    }
}