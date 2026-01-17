#[cfg(test)]
mod file_curly_glance_tests {
    use apchat_tools::file_curly_glance::{find_matching_closing_bracket, is_empty_or_whitespace, find_starting_line};

    #[test]
    fn test_find_matching_closing_bracket_simple() {
        let content = "{}";
        let result = find_matching_closing_bracket(content, 0);
        assert_eq!(result, Some((1, 1)));
    }

    #[test]
    fn test_find_matching_closing_bracket_nested() {
        let content = "{\n    {\n    }\n}\n";
        let result = find_matching_closing_bracket(content, 0);
        assert_eq!(result, Some((4, 1)));
    }

    #[test]
    fn test_find_matching_closing_bracket_not_found() {
        let content = "{\n    {\n    }";
        let result = find_matching_closing_bracket(content, 0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_matching_closing_bracket_multiline() {
        let content = "{\n    let x = 5;\n    let y = 10;\n}\nnext line";
        let result = find_matching_closing_bracket(content, 0);
        assert_eq!(result, Some((4, 1)));
    }

    #[test]
    fn test_is_empty_or_whitespace_empty() {
        assert!(is_empty_or_whitespace(""));
    }

    #[test]
    fn test_is_empty_or_whitespace_spaces() {
        assert!(is_empty_or_whitespace("   "));
    }

    #[test]
    fn test_is_empty_or_whitespace_tabs() {
        assert!(is_empty_or_whitespace("\t\t"));
    }

    #[test]
    fn test_is_empty_or_whitespace_newlines() {
        assert!(is_empty_or_whitespace("\n\n"));
    }

    #[test]
    fn test_is_empty_or_whitespace_not_empty() {
        assert!(!is_empty_or_whitespace("x"));
    }

    #[test]
    fn test_is_empty_or_whitespace_mixed() {
        assert!(!is_empty_or_whitespace("  x  "));
    }

    #[test]
    fn test_find_starting_line_first_line() {
        let content = "{ content";
        let result = find_starting_line(content, 0);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_find_starting_line_second_line() {
        let content = "first line\n{ content";
        let result = find_starting_line(content, 12);
        assert_eq!(result, 2);
    }

    #[test]
    fn test_find_starting_line_third_line() {
        let content = "first line\nsecond line\n{ content";
        let result = find_starting_line(content, 24);
        assert_eq!(result, 3);
    }

    #[test]
    fn test_find_starting_line_with_newlines_before() {
        let content = "\n\n{ content";
        let result = find_starting_line(content, 2);
        assert_eq!(result, 3);
    }
}