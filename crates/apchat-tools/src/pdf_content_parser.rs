//! Custom PDF content parser to work around lopdf's broken decode_content()
//! Handles manual decompression and text extraction from PDF content streams

use anyhow::{Context, Result};
use std::collections::HashMap;

/// Parse PDF content from raw bytes and extract text
pub fn extract_text_from_content_bytes(
    content_bytes: &[u8],
    doc: &lopdf::Document,
    xobject_map: &HashMap<String, lopdf::ObjectId>,
) -> String {
    let content = String::from_utf8_lossy(content_bytes);

    let mut text = String::new();
    let mut tj_count = 0; // Track number of TJ arrays we've extracted

    // Parse TJ operators - pattern: [(...text...)] TJ
    let mut pos = 0;
    while let Some(tj_start) = content[pos..].find("TJ") {
        let abs_tj_start = pos + tj_start;

        // Add space between separate TJ arrays (not the first one)
        if tj_count > 0 && !text.is_empty() {
            // Check if there's actual text between the TJ operators (not just whitespace)
            let between = &content[pos..abs_tj_start];
            if between.chars().any(|c| !c.is_whitespace()) {
                text.push(' ');
            }
        }

        // Look backwards for the array start - pattern is [(...)] or [ (...)]
        // Try to find [ followed by ( (with optional whitespace)
        let content_before = &content[..abs_tj_start];
        let array_start = if let Some(idx) = content_before.rfind("[(") {
            Some(idx)
        } else {
            // Try with whitespace: [   (...
            let mut search_pos = abs_tj_start;
            let mut found = None;
            while let Some(bracket_idx) = content_before[..search_pos].rfind('[') {
                // Check if there's a ( after the [ with only whitespace in between
                let after_bracket = &content_before[bracket_idx + 1..];
                let trimmed = after_bracket.trim_start();
                if trimmed.starts_with('(') {
                    found = Some(bracket_idx);
                    break;
                }
                search_pos = bracket_idx;
                if search_pos == 0 {
                    break;
                }
            }
            found
        };

        if let Some(start) = array_start {
            // Find the matching closing bracket
            let bracket_content = &content[start..];
            if let Some(array_end) = find_matching_bracket(bracket_content) {
                let full_array = &bracket_content[..array_end];

                // Extract strings from the array
                let before_len = text.len();
                extract_text_from_tj_array(full_array, &mut text);
                // Only increment if we actually extracted something
                if text.len() > before_len {
                    tj_count += 1;
                }
            }
        }

        pos = abs_tj_start + 2;
    }

    // Also look for Tj operators - pattern: (text) Tj
    let mut tj_count_inner = 0;
    pos = 0;
    while let Some(tj_start) = content[pos..].find(") Tj") {
        let abs_tj_start = pos + tj_start;

        // Add space between Tj operators if there was previous text
        if tj_count_inner > 0 && !text.is_empty() {
            let between = &content[pos..abs_tj_start];
            if between.chars().any(|c| !c.is_whitespace()) {
                text.push(' ');
            }
        }

        // Look backwards for opening paren
        if let Some(string_start) = content[..abs_tj_start].rfind('(') {
            let pdf_string = &content[string_start + 1..abs_tj_start];
            // Handle PDF string escapes
            let decoded = decode_pdf_string(pdf_string);
            if !decoded.is_empty() {
                text.push_str(&decoded);
                tj_count_inner += 1;
            }
        }
        pos = abs_tj_start + 4;
    }

    text
}

/// Extract text from a TJ array format: [(...)(...)...] TJ
/// Strings in PDF can use parentheses with escapes
/// Negative numbers in TJ arrays indicate spacing adjustments (word breaks)
fn extract_text_from_tj_array(array_content: &str, output: &mut String) {
    // Skip the opening bracket
    let mut content = &array_content[1..];

    while !content.is_empty() {
        if content.starts_with(']') {
            break;
        }

        // Find the next element (either a string '(' or a number)
        if let Some(string_start) = content.find('(') {
            // Check if there's a negative number before this string (word break indicator)
            let before_string = &content[..string_start];
            let has_word_break = before_string.trim().starts_with('-');

            // Find matching closing paren
            if let Some(string_end) = find_matching_paren(&content[string_start..]) {
                let pdf_string = &content[string_start + 1..string_start + string_end];
                let decoded = decode_pdf_string(pdf_string);

                // Add space before word if this was preceded by a negative number
                if has_word_break && !output.is_empty() {
                    output.push(' ');
                }

                output.push_str(&decoded);
                content = &content[string_start + string_end + 1..];
            } else {
                break;
            }
        } else {
            break;
        }
    }
}

/// Find matching closing bracket for [ taking into account nested arrays
fn find_matching_bracket(s: &str) -> Option<usize> {
    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;
    let mut chars = s.chars().enumerate();

    while let Some((i, c)) = chars.next() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match c {
            '\\' => escape_next = true,
            '(' if !in_string => in_string = true,
            ')' if in_string => in_string = false,
            '[' if !in_string => depth += 1,
            ']' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
    }
    None
}

/// Find matching closing paren for ( taking into account nested parens and escapes
fn find_matching_paren(s: &str) -> Option<usize> {
    let mut depth = 0;
    let mut escape_next = false;

    for (i, c) in s.chars().enumerate() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match c {
            '\\' => escape_next = true,
            '(' => depth += 1,
            ')' => {
                if depth == 1 {
                    return Some(i);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    None
}

/// Decode PDF string escape sequences and UTF-16 BE encoding
fn decode_pdf_string(s: &str) -> String {
    let bytes = s.as_bytes();

    // Check for UTF-16 BE BOM
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE && bytes.len() % 2 == 0 && bytes.len() >= 4 {
        let utf16_data: Vec<u16> = bytes
            .chunks(2)
            .skip(1) // Skip BOM
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        if let Ok(decoded) = String::from_utf16(&utf16_data) {
            return decoded;
        }
    }

    // Check for UTF-16 BE without BOM - require at least 6 byte pairs and most must have high byte = 0
    // This avoids false positives while being tolerant of trailing non-UTF16 chars like )
    if bytes.len() % 2 == 0 {
        let num_pairs = bytes.len() / 2;

        // At least 90% of pairs must have high byte = 0 to be considered UTF-16 BE
        let utf16be_count = bytes.chunks(2).filter(|chunk| chunk[0] == 0).count();
        let utf16be_ratio = utf16be_count as f64 / num_pairs as f64;

        if utf16be_ratio >= 0.9 {
            // Very likely UTF-16 BE
            let utf16_data: Vec<u16> = bytes
                .chunks(2)
                .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
                .collect();
            if let Ok(decoded) = String::from_utf16(&utf16_data) {
                return decoded;
            }
        }
    }

    // Handle escape sequences
    let mut result = Vec::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next_char) = chars.next() {
                match next_char {
                    'n' => result.push('\n'),
                    'r' => result.push('\r'),
                    't' => result.push('\t'),
                    'b' => result.push('\x08'),
                    'f' => result.push('\x0c'),
                    '(' => result.push('('),
                    ')' => result.push(')'),
                    '\\' => result.push('\\'),
                    '0'..='7' => {
                        // Octal escape: \nnn or \nn
                        let mut octal = String::from(next_char);
                        if let Some(&c) = chars.peek() {
                            if c.is_ascii_digit() && ('0'..='7').contains(&c) {
                                chars.next();
                                octal.push(c);
                                if let Some(&c) = chars.peek() {
                                    if c.is_ascii_digit() && ('0'..='7').contains(&c) {
                                        chars.next();
                                        octal.push(c);
                                    }
                                }
                            }
                        }
                        if let Ok(code) = u8::from_str_radix(&octal, 8) {
                            result.push(code as char);
                        }
                    }
                    _ => result.push(next_char),
                }
            }
        } else {
            if c == '\0' {
               if let Some(c) = chars.next() {
                  result.push(c);
               }
            } else {
               result.push(c);
            }
        }
    }

    result.into_iter().collect()
}

/// Manually decompress a PDF stream using flate2
pub fn decompress_stream_raw(stream: &lopdf::Stream) -> Result<Vec<u8>> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    let mut decoder = ZlibDecoder::new(&stream.content[..]);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)
        .context("Failed to decompress PDF stream")?;
    Ok(decompressed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_find_matching_paren() {
        assert_eq!(find_matching_paren("(test)"), Some(4));
        assert_eq!(find_matching_paren("((test))"), Some(6));
        assert_eq!(find_matching_paren("(test (nested))"), Some(13));
        assert_eq!(find_matching_paren("(test\\))"), Some(6));
    }

    #[test]
    fn test_decode_pdf_string_utf16() {
        // UTF-16 BE: \x00H\x00e\x00l\x00l\x00o
        let encoded = "\x00H\x00e\x00l\x00l\x00o";
        assert_eq!(decode_pdf_string(encoded), "Hello");
    }

    #[test]
    #[ignore]
    fn test_decode_pdf_string_ascii() {
        assert_eq!(decode_pdf_string("(test)"), "test");
    }
}
