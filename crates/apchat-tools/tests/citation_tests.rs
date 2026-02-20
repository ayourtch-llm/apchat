// Unit tests for AddCitationTool

use apchat_toolcore::{Tool, ToolParameters, ToolContext};
use apchat_policy::PolicyManager;
use apchat_tools::citation::{AddCitationTool, verify_citation_in_path};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn create_test_context(work_dir: PathBuf) -> ToolContext {
    ToolContext::new(
        work_dir,
        "test-session".to_string(),
        PolicyManager::default(),
    )
}

// ---- verify_citation_in_path unit tests ----

#[test]
fn test_verify_citation_found() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("source.txt");
    fs::write(&file, "Hello, world! This is a test.").unwrap();
    assert!(verify_citation_in_path(&file, "world").unwrap());
}

#[test]
fn test_verify_citation_not_found() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("source.txt");
    fs::write(&file, "Hello, world!").unwrap();
    assert!(!verify_citation_in_path(&file, "missing text").unwrap());
}

#[test]
fn test_verify_citation_file_not_readable() {
    let nonexistent = PathBuf::from("/nonexistent/path/file.txt");
    assert!(verify_citation_in_path(&nonexistent, "anything").is_err());
}

// ---- AddCitationTool integration tests ----

#[tokio::test]
async fn test_add_citation_success() {
    let dir = TempDir::new().unwrap();
    let context = create_test_context(dir.path().to_path_buf());

    // Create the cited file with the citation text in it
    fs::write(dir.path().join("notes.txt"), "Important fact: the sky is blue.").unwrap();

    let mut params = ToolParameters::new();
    params.set("citation_label", "sky-color");
    params.set("file_path", "notes.txt");
    params.set("citation_text", "sky is blue");

    let result = AddCitationTool.execute(params, &context).await;
    assert!(result.success, "Expected success: {:?}", result.error);

    let citations = fs::read_to_string(dir.path().join("citations.txt")).unwrap();
    assert!(citations.contains("sky-color:notes.txt:sky is blue"));
}

#[tokio::test]
async fn test_add_citation_file_not_found() {
    let dir = TempDir::new().unwrap();
    let context = create_test_context(dir.path().to_path_buf());

    let mut params = ToolParameters::new();
    params.set("citation_label", "lbl");
    params.set("file_path", "nonexistent.txt");
    params.set("citation_text", "some text");

    let result = AddCitationTool.execute(params, &context).await;
    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("File not found"));
}

#[tokio::test]
async fn test_add_citation_text_not_in_file() {
    let dir = TempDir::new().unwrap();
    let context = create_test_context(dir.path().to_path_buf());

    fs::write(dir.path().join("doc.txt"), "Hello world").unwrap();

    let mut params = ToolParameters::new();
    params.set("citation_label", "lbl");
    params.set("file_path", "doc.txt");
    params.set("citation_text", "this text is absent");

    let result = AddCitationTool.execute(params, &context).await;
    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("Citation text not found"));
}

#[tokio::test]
async fn test_add_citation_invalid_label_characters() {
    let dir = TempDir::new().unwrap();
    let context = create_test_context(dir.path().to_path_buf());

    fs::write(dir.path().join("doc.txt"), "Some content").unwrap();

    let mut params = ToolParameters::new();
    params.set("citation_label", "bad label!");
    params.set("file_path", "doc.txt");
    params.set("citation_text", "Some content");

    let result = AddCitationTool.execute(params, &context).await;
    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("Invalid citation_label"));
}

#[tokio::test]
async fn test_add_citation_duplicate_label() {
    let dir = TempDir::new().unwrap();
    let context = create_test_context(dir.path().to_path_buf());

    fs::write(dir.path().join("doc.txt"), "Some content here.").unwrap();
    // Pre-populate citations.txt with the same label
    fs::write(
        dir.path().join("citations.txt"),
        "my-label:doc.txt:Some content here.\n",
    )
    .unwrap();

    let mut params = ToolParameters::new();
    params.set("citation_label", "my-label");
    params.set("file_path", "doc.txt");
    params.set("citation_text", "Some content here.");

    let result = AddCitationTool.execute(params, &context).await;
    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("already exists"));
}

#[tokio::test]
async fn test_add_citation_creates_citations_file_if_absent() {
    let dir = TempDir::new().unwrap();
    let context = create_test_context(dir.path().to_path_buf());

    fs::write(dir.path().join("src.rs"), "fn main() {}").unwrap();

    let citations_path = dir.path().join("citations.txt");
    assert!(!citations_path.exists());

    let mut params = ToolParameters::new();
    params.set("citation_label", "rust/main");
    params.set("file_path", "src.rs");
    params.set("citation_text", "fn main()");

    let result = AddCitationTool.execute(params, &context).await;
    assert!(result.success, "Expected success: {:?}", result.error);
    assert!(citations_path.exists());
    let contents = fs::read_to_string(&citations_path).unwrap();
    assert!(contents.contains("rust/main:src.rs:fn main()"));
}

#[tokio::test]
async fn test_add_citation_label_all_allowed_chars() {
    let dir = TempDir::new().unwrap();
    let context = create_test_context(dir.path().to_path_buf());

    fs::write(dir.path().join("f.txt"), "abc").unwrap();

    let label = "aZ0-_+/";
    let mut params = ToolParameters::new();
    params.set("citation_label", label);
    params.set("file_path", "f.txt");
    params.set("citation_text", "abc");

    let result = AddCitationTool.execute(params, &context).await;
    assert!(result.success, "Expected success: {:?}", result.error);
}

#[tokio::test]
async fn test_add_citation_newline_in_file_path_rejected() {
    let dir = TempDir::new().unwrap();
    let context = create_test_context(dir.path().to_path_buf());

    let mut params = ToolParameters::new();
    params.set("citation_label", "lbl");
    params.set("file_path", "bad\npath.txt");
    params.set("citation_text", "text");

    let result = AddCitationTool.execute(params, &context).await;
    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("newline"));
}

#[tokio::test]
async fn test_add_citation_newline_in_citation_text_rejected() {
    let dir = TempDir::new().unwrap();
    let context = create_test_context(dir.path().to_path_buf());

    fs::write(dir.path().join("f.txt"), "line1\nline2").unwrap();

    let mut params = ToolParameters::new();
    params.set("citation_label", "lbl");
    params.set("file_path", "f.txt");
    params.set("citation_text", "line1\nline2");

    let result = AddCitationTool.execute(params, &context).await;
    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("newline"));
}

#[tokio::test]
async fn test_add_citation_multiple_citations_accumulate() {
    let dir = TempDir::new().unwrap();
    let context = create_test_context(dir.path().to_path_buf());

    fs::write(dir.path().join("doc.txt"), "alpha beta gamma").unwrap();

    for (label, text) in [("lbl-a", "alpha"), ("lbl-b", "beta"), ("lbl-c", "gamma")] {
        let mut params = ToolParameters::new();
        params.set("citation_label", label);
        params.set("file_path", "doc.txt");
        params.set("citation_text", text);
        let result = AddCitationTool.execute(params, &context).await;
        assert!(result.success, "label={} error={:?}", label, result.error);
    }

    let contents = fs::read_to_string(dir.path().join("citations.txt")).unwrap();
    assert!(contents.contains("lbl-a:doc.txt:alpha"));
    assert!(contents.contains("lbl-b:doc.txt:beta"));
    assert!(contents.contains("lbl-c:doc.txt:gamma"));
}
