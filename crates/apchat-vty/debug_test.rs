#[cfg(test)]
mod tests {
    use super::super::readline::readline::Readline;
    use std::io;

    #[test]
    fn test_handle_delete_debug() {
        let mut readline = Readline::new().expect("Failed to create Readline");
        println!("Initial lines: {:?}", readline.lines);
        println!("Initial cursor_line: {}", readline.cursor_line);
        println!("Initial cursor_col: {}", readline.cursor_col);
        println!("Initial line: '{}'", readline.line);
        println!("Initial cursor: {}", readline.cursor);
        
        readline.line = "hello".to_string();
        readline.cursor = 1;
        
        println!("After setting line='hello', cursor=1");
        println!("lines: {:?}", readline.lines);
        println!("cursor_line: {}", readline.cursor_line);
        println!("cursor_col: {}", readline.cursor_col);
        println!("line: '{}'", readline.line);
        println!("cursor: {}", readline.cursor);
        
        let result = readline.handle_delete();
        println!("After handle_delete, result: {}", result);
        println!("lines: {:?}", readline.lines);
        println!("cursor_line: {}", readline.cursor_line);
        println!("cursor_col: {}", readline.cursor_col);
        println!("line: '{}'", readline.line);
    }
}
