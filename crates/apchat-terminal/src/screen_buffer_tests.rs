// Test to verify the terminal layout with reserved lines

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_screen_buffer_reserved_lines() {
        // Create a screen buffer with 24 rows and 2 reserved lines
        let mut screen_buffer = ScreenBuffer::new(80, 24, 2);

        // Set status line
        screen_buffer.set_status_line("Status: Ready");

        // Set input line
        screen_buffer.set_input_line("Input: ");

        // Process some output
        screen_buffer.process_output("Hello World\n");
        screen_buffer.process_output("This is a test\n");

        // Get contents
        let contents = screen_buffer.get_contents(false, false);

        // Verify reserved lines are present
        assert!(contents.contains("Status: Ready"));
        assert!(contents.contains("Input: "));

        // Verify content area is available
        assert!(contents.contains("Hello World"));
        assert!(contents.contains("This is a test"));
    }

    #[test]
    fn test_terminal_layout() {
        let layout = TerminalLayout::new(80, 24, 2);

        // Test content rows calculation
        assert_eq!(layout.content_rows(), 22);

        // Test status line position
        assert_eq!(layout.status_line_row(), 22); // 0-indexed, row 22 is the 23rd row (line -2)

        // Test input line position
        assert_eq!(layout.input_line_row(), 23); // 0-indexed, row 23 is the 24th row (line -1)
    }

    #[test]
    fn test_screen_buffer_resize() {
        let mut screen_buffer = ScreenBuffer::new(80, 24, 2);
        screen_buffer.set_status_line("Initial status");

        // Resize to larger dimensions
        screen_buffer.resize(100, 30);

        // Verify new dimensions
        let (cols, rows) = screen_buffer.size();
        assert_eq!(cols, 100);
        assert_eq!(rows, 30);

        // Verify reserved lines still work
        screen_buffer.set_status_line("Resized status");
        let contents = screen_buffer.get_contents(false, false);
        assert!(contents.contains("Resized status"));
    }

    #[test]
    fn test_input_line_visibility() {
        let mut screen_buffer = ScreenBuffer::new(80, 24, 2);
        screen_buffer.set_input_line("Visible input");

        // Get contents with visible input line
        let contents_visible = screen_buffer.get_contents(false, false);
        assert!(contents_visible.contains("Visible input"));

        // Hide input line
        screen_buffer.set_input_line_visible(false);
        let contents_hidden = screen_buffer.get_contents(false, false);
        assert!(!contents_hidden.contains("Visible input"));

        // Show input line again
        screen_buffer.set_input_line_visible(true);
        let contents_visible_again = screen_buffer.get_contents(false, false);
        assert!(contents_visible_again.contains("Visible input"));
    }
}
