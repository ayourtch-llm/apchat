/// Integration tests for plan_edits matching improvements
///
/// These tests demonstrate the three levels of matching:
/// 1. Exact match
/// 2. Normalized whitespace match
/// 3. Levenshtein distance matching (for error messages)

use std::fs;
use tempfile::TempDir;

const VPP_FILE_CONTENT: &str = r#"#
# Added support for building on macOS (Darwin)
else
OS_ID=darwin
OS_VERSION_ID=0
endif
OS_ID        = $(shell grep '^ID=' /etc/os-release | cut -f2- -d= | sed -e 's/"//g')
ifeq ($(OS_ID),rhel)
OS_VERSION_ID= $(shell grep '^VERSION_ID=' /etc/os-release | cut -f2- -d= | sed -e 's/"//g' | sed -e 's/\..*//')
else
OS_VERSION_ID= $(shell grep '^VERSION_ID=' /etc/os-release | cut -f2- -d= | sed -e 's/"//g')
endif
OS_CODENAME  = $(shell grep '^VERSION_CODENAME=' /etc/os-release | cut -f2- -d= | sed -e 's/"//g')
endif
"#;

#[test]
fn test_exact_match_fails_with_levenshtein_suggestions() {
    // This is the content the user tried to match (which doesn't exist exactly)
    let old_content_attempt = r#"#
# Added support for building on macOS (Darwin)
ifeq ($(OS_ID),darwin)
OS_ID=darwin
OS_VERSION_ID=0
endif
OS_ID        = $(shell grep '^ID=' /etc/os-release | cut -f2- -d= | sed -e 's/"//g')
ifeq ($(OS_ID),rhel)
OS_VERSION_ID= $(shell grep '^VERSION_ID=' /etc/os-release | cut -f2- -d= | sed -e 's/"//g' | sed -e 's/\..*//')
else
OS_VERSION_ID= $(shell grep '^VERSION_ID=' /etc/os-release | cut -f2- -d= | sed -e 's/"//g')
endif
OS_CODENAME  = $(shell grep '^VERSION_CODENAME=' /etc/os-release | cut -f2- -d= | sed -e 's/"//g')
endif"#;

    // The actual file content differs: starts with "else" not "ifeq"
    // This should trigger Levenshtein distance matching

    // Verify exact match fails
    assert!(!VPP_FILE_CONTENT.contains(old_content_attempt));

    // But the content is very similar (Levenshtein distance should be small)
    let distance = strsim::levenshtein(VPP_FILE_CONTENT, old_content_attempt);

    println!("Levenshtein distance between attempted match and file: {}", distance);
    println!("File length: {}, Search length: {}", VPP_FILE_CONTENT.len(), old_content_attempt.len());

    // The distance should be relatively small compared to the content size
    // (they differ by just a few characters - "else" vs "ifeq ($(OS_ID),darwin)")
    let max_len = VPP_FILE_CONTENT.len().max(old_content_attempt.len());
    let similarity_percent = ((max_len - distance) as f64 / max_len as f64) * 100.0;

    println!("Similarity: {:.1}%", similarity_percent);

    // Should be at least 80% similar
    assert!(similarity_percent > 80.0,
        "Expected high similarity for near-match, got {:.1}%", similarity_percent);
}

#[test]
fn test_whitespace_normalization() {
    let content_with_tabs = "line1\tvalue1\nline2\tvalue2";
    let content_with_spaces = "line1    value1\nline2    value2";

    // Exact match should fail
    assert!(!content_with_tabs.contains(content_with_spaces));
    assert!(!content_with_spaces.contains(content_with_tabs));

    // But after normalization they should match
    let normalize = |s: &str| -> String {
        let with_spaces = s.replace('\t', "    ");
        let normalized = with_spaces.replace("\r\n", "\n").replace('\r', "\n");
        normalized
            .lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    };

    let norm_tabs = normalize(content_with_tabs);
    let norm_spaces = normalize(content_with_spaces);

    assert_eq!(norm_tabs, norm_spaces, "Normalized versions should match");
}

#[test]
fn test_levenshtein_sliding_window() {
    // Simulate finding the best match in a larger file
    let file_content = r#"Some header content
Line 1
Line 2
Target content here
More lines
Footer content"#;

    let search_for = "Target content here\nMore lines";

    // The search content appears in the middle of the file
    // Levenshtein should find it with a small window

    let file_lines: Vec<&str> = file_content.lines().collect();
    let search_lines: Vec<&str> = search_for.lines().collect();

    // Try each possible window of the same size
    let mut min_distance = usize::MAX;
    let mut best_match_idx = 0;

    for start_idx in 0..=file_lines.len().saturating_sub(search_lines.len()) {
        let end_idx = start_idx + search_lines.len();
        let window = file_lines[start_idx..end_idx].join("\n");
        let distance = strsim::levenshtein(&window, search_for);

        if distance < min_distance {
            min_distance = distance;
            best_match_idx = start_idx;
        }
    }

    println!("Best match at line {} with distance {}", best_match_idx + 1, min_distance);

    // Should find exact match (distance 0) at line 4 (index 3)
    assert_eq!(min_distance, 0, "Should find exact match");
    assert_eq!(best_match_idx, 3, "Should find at line 4");
}

#[test]
fn test_show_whitespace() {
    let show_whitespace = |s: &str| -> String {
        s.replace('\t', "⇥")
         .replace('\n', "↵\n")
         .replace('\r', "⏎")
         .replace(' ', "·")
    };

    let input = "Hello\tWorld\nNext line\r\nWith CRLF";
    let output = show_whitespace(input);

    assert!(output.contains("⇥"), "Should show tabs");
    assert!(output.contains("↵"), "Should show newlines");
    assert!(output.contains("⏎"), "Should show carriage returns");
    assert!(output.contains("·"), "Should show spaces");

    println!("Whitespace visualization:\n{}", output);
}

#[test]
fn test_realistic_vpp_file_scenario() {
    // Create a temp directory with the vpp-file content
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("vpp-file");
    fs::write(&file_path, VPP_FILE_CONTENT).unwrap();

    // The user's attempted old_content (which doesn't match exactly)
    let old_content_attempt = r#"#
# Added support for building on macOS (Darwin)
ifeq ($(OS_ID),darwin)
OS_ID=darwin
OS_VERSION_ID=0
endif"#;

    // Read the file
    let file_content = fs::read_to_string(&file_path).unwrap();

    // Exact match should fail
    assert!(!file_content.contains(old_content_attempt));

    // But we should be able to find very similar content
    let file_lines: Vec<&str> = file_content.lines().collect();
    let search_lines: Vec<&str> = old_content_attempt.lines().collect();

    let mut best_distance = usize::MAX;
    let mut best_start = 0;

    // Sliding window search
    for start_idx in 0..=file_lines.len().saturating_sub(search_lines.len()) {
        let end_idx = start_idx + search_lines.len();
        let window = file_lines[start_idx..end_idx].join("\n");
        let distance = strsim::levenshtein(&window, old_content_attempt);

        if distance < best_distance {
            best_distance = distance;
            best_start = start_idx;
        }
    }

    println!("Best match found at line {} with distance {}", best_start + 1, best_distance);

    let max_len = old_content_attempt.len();
    let similarity = ((max_len - best_distance) as f64 / max_len as f64) * 100.0;

    println!("Similarity: {:.1}%", similarity);

    // Should find content that's at least 70% similar
    // (differs by "else" vs "ifeq ($(OS_ID),darwin)")
    assert!(similarity > 70.0,
        "Should find similar content, got {:.1}% similar", similarity);

    // Print the best match for debugging
    println!("\nBest matching content:");
    let end_idx = (best_start + search_lines.len()).min(file_lines.len());
    for (i, line) in file_lines[best_start..end_idx].iter().enumerate() {
        println!("  {:3} | {}", best_start + i + 1, line);
    }
}
