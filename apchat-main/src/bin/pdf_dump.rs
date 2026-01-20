//! PDF dump utility - dumps PDF structure as JSON
//!
//! Usage:
//!   cargo run --bin pdf_dump -- <path/to/pdf>
//!   cargo run --bin pdf_dump -- <path/to/pdf> --max-objects 50

use anyhow::Result;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: pdf_dump <pdf_file> [--max-objects N]");
        eprintln!("  pdf_file: Path to PDF file");
        eprintln!("  --max-objects N: Maximum objects to dump (default: 100, 0 for unlimited)");
        std::process::exit(1);
    }

    let pdf_path = PathBuf::from(&args[1]);
    let mut max_objects = 100;

    // Parse optional arguments
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--max-objects" => {
                if i + 1 < args.len() {
                    max_objects = args[i + 1].parse().unwrap_or(100);
                    i += 2;
                } else {
                    eprintln!("Error: --max-objects requires a value");
                    std::process::exit(1);
                }
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                std::process::exit(1);
            }
        }
    }

    if !pdf_path.exists() {
        eprintln!("Error: File not found: {}", pdf_path.display());
        std::process::exit(1);
    }

    // Load and dump PDF
    match dump_pdf_structure(&pdf_path, max_objects) {
        Ok(json) => {
            println!("{}", json);
        }
        Err(e) => {
            eprintln!("Error dumping PDF: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn dump_pdf_structure(pdf_path: &std::path::Path, max_objects: usize) -> Result<String> {
    use serde_json::json;

    let doc = lopdf::Document::load(pdf_path)?;

    let mut result = json!({
        "file": pdf_path.display().to_string(),
        "version": doc.version,
        "page_count": doc.get_pages().len(),
        "extracted_text": extract_text_from_pdf(&doc),
        "objects": []
    });

    let objects = result["objects"].as_array_mut().unwrap();

    let mut count = 0;
    for (object_id, object) in doc.objects.iter() {
        if max_objects > 0 && count >= max_objects {
            break;
        }

        let obj_info = serialize_object(object_id, object, &doc);
        objects.push(obj_info);
        count += 1;
    }

    // Add page information
    let mut pages = vec![];
    for (page_num, page_id) in doc.get_pages().iter() {
        let page_info = serialize_page(page_num, *page_id, &doc);
        pages.push(page_info);
    }

    if let Some(pages_arr) = result.as_object_mut() {
        pages_arr.insert("pages".to_string(), json!(pages));
    }

    Ok(serde_json::to_string_pretty(&result)?)
}

/// Extract all text from PDF, including from XObjects
fn extract_text_from_pdf(doc: &lopdf::Document) -> String {
    let mut all_text = String::new();

    for (page_num, page_id) in doc.get_pages().iter() {
        if let Some(page_text) = extract_text_from_page(doc, *page_id) {
            if !page_text.is_empty() {
                all_text.push_str(&format!("\n=== Page {} ===\n{}", page_num, page_text));
            }
        }
    }

    if all_text.is_empty() {
        "[No text found - PDF may be image-based or use complex encodings]".to_string()
    } else {
        all_text
    }
}

/// Extract text from a single page, using custom parser to work around lopdf bugs
fn extract_text_from_page(doc: &lopdf::Document, page_id: (u32, u16)) -> Option<String> {
    use flate2::read::ZlibDecoder;
    use std::collections::HashMap;
    use std::io::Read;

    let page_obj = doc.get_object(lopdf::ObjectId::from(page_id)).ok()?;
    let page_dict = page_obj.as_dict().ok()?;

    // Build resource map for XObject lookups (optional)
    let mut xobject_map = HashMap::new();
    match page_dict.get(b"Resources") {
        Ok(resources) => {
            if let Ok(res_dict) = resources.as_dict() {
                if let Ok(xobjects) = res_dict.get(b"XObject") {
                    if let Ok(xobj_dict) = xobjects.as_dict() {
                        for (name, obj_ref) in xobj_dict.iter() {
                            let name_str = String::from_utf8_lossy(name).to_string();
                            if let Ok(ref_id) = obj_ref.as_reference() {
                                xobject_map.insert(name_str, ref_id);
                            }
                        }
                    }
                }
            }
        }
        Err(_) => {
        }
    }

    // Get and process content streams
    let content_ids = doc.get_page_contents(page_id);
    let mut text = String::new();

    for content_id in content_ids {
        let content_obj = doc.get_object(content_id).ok()?;
        let stream = content_obj.as_stream().ok()?;

        // Use manual decompression (lopdf's decode_content is broken for this PDF)
        let mut decoder = ZlibDecoder::new(&stream.content[..]);
        let mut decompressed = Vec::new();
        if decoder.read_to_end(&mut decompressed).is_ok() {
            let content_str = String::from_utf8_lossy(&decompressed);
            text.push_str(&extract_text_from_content_str(&content_str, &xobject_map));
        } else {
        }
    }

    if text.is_empty() {
        None
    } else {
        // Clean up text
        let cleaned: String = text.chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
            .collect();
        Some(cleaned.split_whitespace().collect::<Vec<_>>().join(" "))
    }
}

/// Extract text from content string (custom parser to work around lopdf bugs)
fn extract_text_from_content_str(content: &str, _xobject_map: &HashMap<String, lopdf::ObjectId>) -> String {
    let mut text = String::new();
    let mut tj_count = 0;

    // Parse TJ operators - pattern: [(...text...)] TJ
    let mut pos = 0;
    while let Some(tj_start) = content[pos..].find("TJ") {
        let abs_tj_start = pos + tj_start;

        // Add space between separate TJ arrays (not the first one)
        if tj_count > 0 && !text.is_empty() {
            let between = &content[pos..abs_tj_start];
            if between.chars().any(|c| !c.is_whitespace()) {
                text.push(' ');
            }
        }

        // Look backwards for the array start - pattern is [(...)] or [ (...)]
        let content_before = &content[..abs_tj_start];
        let array_start = if let Some(idx) = content_before.rfind("[(") {
            Some(idx)
        } else {
            // Try with whitespace: [   (...
            let mut search_pos = abs_tj_start;
            let mut found = None;
            while let Some(bracket_idx) = content_before[..search_pos].rfind('[') {
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
            let bracket_content = &content[start..];
            if let Some(array_end) = find_matching_bracket(bracket_content) {
                let full_array = &bracket_content[..array_end];
                let before_len = text.len();
                extract_text_from_tj_array(full_array, &mut text);
                if text.len() > before_len {
                    tj_count += 1;
                }
            } else {
            }
        } else {
        }

        pos = abs_tj_start + 2;
    }

    // Also look for Tj operators - pattern: (text) Tj
    let mut tj_count_inner = 0;
    pos = 0;
    while let Some(tj_start) = content[pos..].find(") Tj") {
        let abs_tj_start = pos + tj_start;

        // Add space between Tj operators
        if tj_count_inner > 0 && !text.is_empty() {
            let between = &content[pos..abs_tj_start];
            if between.chars().any(|c| !c.is_whitespace()) {
                text.push(' ');
            }
        }

        // Look backwards for opening paren
        if let Some(string_start) = content[..abs_tj_start].rfind('(') {
            let pdf_string = &content[string_start + 1..abs_tj_start];
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
/// Negative numbers in TJ arrays indicate word breaks
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

/// Find matching closing bracket
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

/// Find matching closing paren
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

/// Decode PDF string with UTF-16 BE support
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
    if bytes.len() >= 12 && bytes.len() % 2 == 0 {
        let num_pairs = bytes.len() / 2;

        // At least 90% of pairs must have high byte = 0 to be considered UTF-16 BE
        let utf16be_count = bytes.chunks(2).filter(|chunk| chunk[0] == 0).count();
        let utf16be_ratio = utf16be_count as f64 / num_pairs as f64;

        if utf16be_ratio >= 0.9 && num_pairs >= 6 {
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
            result.push(c);
        }
    }

    result.into_iter().collect()
}

fn serialize_object(id: &lopdf::ObjectId, obj: &lopdf::Object, doc: &lopdf::Document) -> serde_json::Value {
    use serde_json::json;

    let type_name = match obj {
        lopdf::Object::Null => "Null",
        lopdf::Object::Boolean(_) => "Boolean",
        lopdf::Object::Integer(_) => "Integer",
        lopdf::Object::Real(_) => "Real",
        lopdf::Object::Name(_) => "Name",
        lopdf::Object::String(_, _) => "String",
        lopdf::Object::Array(_) => "Array",
        lopdf::Object::Dictionary(_) => "Dictionary",
        lopdf::Object::Stream(_) => "Stream",
        lopdf::Object::Reference(_) => "Reference",
    };

    let value = match obj {
        lopdf::Object::Null => serde_json::Value::Null,
        lopdf::Object::Boolean(b) => json!(b),
        lopdf::Object::Integer(i) => json!(i),
        lopdf::Object::Real(f) => json!(f),
        lopdf::Object::Name(bytes) => json!(String::from_utf8_lossy(bytes).to_string()),
        lopdf::Object::String(bytes, fmt) => {
            json!({
                "value": String::from_utf8_lossy(bytes).to_string(),
                "format": format!("{:?}", fmt)
            })
        }
        lopdf::Object::Array(arr) => {
            let items: Vec<_> = arr.iter()
                .take(20) // Limit array items
                .map(|item| serialize_object(id, item, doc))
                .collect();
            json!(items)
        }
        lopdf::Object::Stream(stream) => {
            json!({
                "dict": serialize_dictionary(&stream.dict, doc),
                "allows_compression": stream.allows_compression,
                "content_length": stream.content.len()
            })
        }
        lopdf::Object::Dictionary(dict) => serialize_dictionary(dict, doc),
        lopdf::Object::Reference(ref_id) => {
            json!({
                "object_id": format!("{}_{}", ref_id.0, ref_id.1)
            })
        }
    };

    json!({
        "id": format!("{}_{}", id.0, id.1),
        "type": type_name,
        "value": value
    })
}

fn serialize_dictionary(dict: &lopdf::Dictionary, doc: &lopdf::Document) -> serde_json::Value {
    use serde_json::json;

    let mut map = serde_json::Map::new();
    for (key, val) in dict.iter() {
        let key_str = String::from_utf8_lossy(key).to_string();
        let val_json = serialize_object(&(0, 0), val, doc);
        map.insert(key_str, val_json);
    }
    json!(map)
}

fn serialize_page(page_num: &u32, page_id: (u32, u16), doc: &lopdf::Document) -> serde_json::Value {
    use serde_json::json;

    let mut page_info = json!({
        "number": page_num,
        "id": format!("{}_{}", page_id.0, page_id.1)
    });

    // Get page dictionary
    if let Ok(page_obj) = doc.get_object(lopdf::ObjectId::from(page_id)) {
        if let Ok(dict) = page_obj.as_dict() {
            // Extract common page properties
            if let Ok(media_box) = dict.get(b"MediaBox") {
                page_info["media_box"] = serialize_object(&(0, 0), media_box, doc);
            }

            // Get resources and XObjects
            if let Ok(resources) = dict.get(b"Resources") {
                if let Ok(res_dict) = resources.as_dict() {
                    if let Ok(xobjects) = res_dict.get(b"XObject") {
                        if let Ok(xobj_dict) = xobjects.as_dict() {
                            let mut xobj_list = serde_json::Map::new();
                            for (name, obj_ref) in xobj_dict.iter() {
                                let name_str = String::from_utf8_lossy(name).to_string();
                                xobj_list.insert(name_str, serialize_object(&(0, 0), obj_ref, doc));
                            }
                            page_info["xobjects"] = json!(xobj_list);
                        }
                    }
                }
            }

            // Get content streams with detailed operation info
            let content_ids = doc.get_page_contents(page_id);
            let mut contents = vec![];
            for content_id in content_ids {
                if let Ok(content_obj) = doc.get_object(content_id) {
                    if let Ok(stream) = content_obj.as_stream() {
                        let mut stream_info = json!({
                            "id": format!("{}_{}", content_id.0, content_id.1)
                        });

                        // Try to decode and show detailed operations
                        if let Ok(decoded) = stream.decode_content() {
                            let ops: Vec<_> = decoded.operations.iter()
                                .take(100) // Show more operations
                                .map(|op| {
                                    let op_name = String::from_utf8_lossy(op.operator.as_bytes()).to_string();
                                    let operands: Vec<_> = op.operands.iter()
                                        .take(5)
                                        .map(|arg| format_operand(arg))
                                        .collect();

                                    json!({
                                        "operator": op_name,
                                        "operands": operands,
                                        "operand_count": op.operands.len()
                                    })
                                })
                                .collect();

                            stream_info["operations"] = json!(ops);
                            stream_info["operation_count"] = json!(decoded.operations.len());
                        }

                        contents.push(stream_info);
                    }
                }
            }
            page_info["contents"] = json!(contents);
        }
    }

    page_info
}

/// Format an operand for display
fn format_operand(obj: &lopdf::Object) -> String {
    match obj {
        lopdf::Object::Null => "null".to_string(),
        lopdf::Object::Boolean(b) => b.to_string(),
        lopdf::Object::Integer(i) => i.to_string(),
        lopdf::Object::Real(f) => f.to_string(),
        lopdf::Object::Name(bytes) => format!("/{}", String::from_utf8_lossy(bytes)),
        lopdf::Object::String(bytes, _) => format!("'{}'", String::from_utf8_lossy(bytes)),
        lopdf::Object::Array(arr) => {
            let items: Vec<_> = arr.iter().take(3).map(format_operand).collect();
            format!("[{}]", items.join(" "))
        }
        lopdf::Object::Dictionary(_) => "<<dict>>".to_string(),
        lopdf::Object::Stream(_) => "<stream>".to_string(),
        lopdf::Object::Reference(id) => format!("{}_{}", id.0, id.1),
    }
}
