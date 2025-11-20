# Sub‑Agent Specification (Summary‑Returning Wrapper)

## 1. Purpose
The **sub‑agent** is a thin extension of the existing `kimichat` CLI that enables a *summary‑mode* operation. When the user supplies an optional task description (e.g., `--task "refactor utils"`), the application:
1. Shows a 2‑word preview of the task in the prompt line.
2. Executes the full conversation pipeline exactly as the main agent does.
3. Returns **only a concise JSON summary** (with optional metadata) instead of the full transcript.

This provides a fast, program‑friendly way to obtain high‑level results while preserving all existing functionality.

---

## 2. High‑Level Flow
```
User Input (optional --task) ──► CLI parses flags ──► Build full prompt
                                 │
                                 ▼
                     run_conversation (unchanged core)
                                 │
                                 ▼
                ┌───────────────────────────────────────┐
                │   If summary‑mode:                     │
                │   • Generate 2‑word preview for UI    │
                │   • Summarise final assistant output │
                │   • Pack summary + metadata → JSON    │
                │   • Print JSON (pretty optional)      │
                └───────────────────────────────────────┘
                                 │
                                 ▼
                Normal display (full transcript) – unchanged
```

---

## 3. CLI Changes (`src/main.rs`)
- Add new arguments via `clap`:
  ```rust
  #[derive(Parser, Debug)]
  struct CliArgs {
      #[arg()]                     // regular interactive prompt (optional)
      prompt: Option<String>,

      /// Run in summary mode – give a short description of the task.
      #[arg(long, value_name = "TEXT")]
      task: Option<String>,

      /// Force a specific model (kimi|gpt-oss)
      #[arg(long)]
      model: Option<String>,

      /// Pretty‑print the JSON output (only useful with --task)
      #[arg(long)]
      pretty: bool,
  }
  ```
- When `task` is present, generate a **2‑word preview** using a helper (see §5) and prepend it to the prompt line shown to the user:
  ```rust
  let preview = two_word_preview(&args.task);
  let full_prompt = format!("[{}] {}", preview, args.task);
  ```
- Propagate a new flag `summary_mode: bool` to the conversation runner via a configuration struct (`RunCfg`).

---

## 4. Configuration Struct (`src/lib.rs` or wherever `run_conversation` lives)
Add a field to the existing run‑configuration:
```rust
pub struct RunCfg {
    pub model: ModelColor,
    // … other existing fields …
    pub summary_mode: bool,   // true when --task is used
}
```
No changes to the internal algorithm – the flag is only consulted **after** the conversation finishes.

---

## 5. Helper Functions (`src/preview.rs` – new module)
```rust
/// Returns a two‑word “title‑case” preview from a free‑form task description.
pub fn two_word_preview(task: &str) -> String {
    let stop = ["the","a","an","to","for","and","of","in"];
    let words: Vec<&str> = task
        .split_whitespace()
        .filter(|w| !stop.contains(&w.to_ascii_lowercase().as_str()))
        .take(2)
        .collect();
    words.join(" ").to_ascii_titlecase()
}
```
Used only when `--task` is supplied; the preview is displayed in the interactive line, e.g. `>>> [Refactor Utils] …`.

---

## 6. Summary Generation (`src/summary.rs` – new module)
### 6.1 Public API
```rust
pub enum Verbosity { Terse, Normal, Verbose }

/// Simple heuristic – picks the first N sentences based on verbosity.
pub fn heuristic_summary(output: &str, verbosity: Verbosity) -> String;

/// Optional model‑based summarisation (re‑uses the existing API call).
pub async fn model_summarize(output: &str, model: ModelColor) -> anyhow::Result<String>;

#[derive(Serialize)]
pub struct SummaryMeta {
    pub model_used: String,
    pub tokens_used: usize,
    pub elapsed_ms: u128,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_modified: Option<Vec<String>>, // populated from ConversationResult
}

/// Serialises the summary + meta to JSON (pretty optional).
pub fn to_json(summary: &str, meta: SummaryMeta, pretty: bool) -> String;
```
### 6.2 Implementation Sketch
```rust
pub fn heuristic_summary(output: &str, verbosity: Verbosity) -> String {
    let sentences: Vec<&str> = output.split('.').collect();
    match verbosity {
        Verbosity::Terse => sentences.get(0).map(|s| s.trim()).unwrap_or("").to_string(),
        Verbosity::Normal => sentences.iter().take(2).map(|s| s.trim()).collect::<Vec<_>>().join(". ") + ".",
        Verbosity::Verbose => sentences.iter().take(3).map(|s| s.trim()).collect::<Vec<_>>().join(". ") + ".",
    }
}

pub async fn model_summarize(output: &str, model: ModelColor) -> anyhow::Result<String> {
    let system = "You are a concise summariser. Summarise the following assistant response in three short sentences, preserving any file paths or command results.";
    let resp = crate::api::call_api(system, output, model).await?; // reuse existing helper
    Ok(resp.assistant_message)
}

pub fn to_json(summary: &str, meta: SummaryMeta, pretty: bool) -> String {
    let payload = serde_json::json!({"summary": summary, "metadata": meta});
    if pretty {
        serde_json::to_string_pretty(&payload).unwrap()
    } else {
        serde_json::to_string(&payload).unwrap()
    }
}
```
---

## 7. Post‑Conversation Hook (`src/main.rs`)
After `run_conversation` returns a `ConversationResult` (containing the final assistant message, token usage, elapsed time, and any file modifications):
```rust
if cfg.summary_mode {
    // 1️⃣ 2‑word preview already shown in the UI.
    // 2️⃣ Generate the summary (heuristic is default; switch to model if desired).
    let raw = result.assistant_message;
    let summary = summary::heuristic_summary(&raw, summary::Verbosity::Normal);

    // 3️⃣ Build metadata.
    let meta = summary::SummaryMeta {
        model_used: cfg.model.to_string(),
        tokens_used: result.tokens_used,
        elapsed_ms: result.elapsed_ms,
        status: "success".into(),
        files_modified: result.files_modified.clone(),
    };

    // 4️⃣ Print JSON (pretty if `--pretty`).
    let json = summary::to_json(&summary, meta, args.pretty);
    println!("{}", json);
} else {
    // Existing interactive rendering – unchanged.
    display_conversation(&result);
}
```
If the core run returns an error, construct a minimal JSON with `status: "error"` and a short error message in `summary`.

---

## 8. Tests (`tests/sub_agent.rs`)
- **unit**: `two_word_preview` returns expected titles.
- **unit**: `heuristic_summary` respects `Verbosity`.
- **integration**: Run the binary with `--task "create struct User"` and verify the output is valid JSON containing a `summary` field.
- **integration**: Run without `--task` and ensure normal interactive behavior.

Use `assert_cmd` or `std::process::Command` to invoke the compiled binary in a temporary workspace.

---

## 9. Documentation Updates
- **README.md** – add a new “Summary mode” section with examples:
  ```bash
  $ kimichat --task "refactor utils" "dummy prompt" --pretty
  {
    "summary": "Refactored utils.rs and updated tests; all cargo test pass.",
    "metadata": {
      "model_used": "Kimi",
      "tokens_used": 112,
      "elapsed_ms": 1845,
      "status": "success",
      "files_modified": ["src/utils.rs","src/utils_test.rs"]
    }
  }
  ```
- **kimi.md** – add a design note describing the *summary‑mode* flag and its interaction with the existing conversation pipeline.

---

## 10. Impact Analysis
| Area | Impact | Mitigation |
|------|--------|------------|
| **Core logic** | None – we only wrap the final result. | No changes to token handling, rate‑limiting, or tool validation. |
| **History** | Existing history file continues to store the full transcript (unchanged). | Summaries are **not** written back to history, preserving privacy. |
| **Performance** | Negligible (heuristic) or a single extra model call (optional). | Heuristic is default; model summarisation can be toggled off. |
| **Testing** | New module adds ~200 lines of test coverage. | CI updated to run `cargo test` and enforce >80 % coverage for `src/summary.rs`. |
| **User experience** | Adds a concise, machine‑readable output option. | Clear help text (`kimichat --help`) explains the new flags. |

---

## 11. Implementation Checklist
- [ ] Extend `CliArgs` with `--task` and `--pretty` flags.
- [ ] Add `two_word_preview` helper (`src/preview.rs`).
- [ ] Add `summary_mode` field to `RunCfg`.
- [ ] Create `src/summary.rs` with heuristic, optional model summariser, metadata struct, and JSON serializer.
- [ ] Update post‑conversation handling in `main.rs` to branch on `summary_mode`.
- [ ] Write unit & integration tests (`tests/sub_agent.rs`).
- [ ] Update README and `kimi.md` with usage examples.
- [ ] Run `cargo fmt`, `cargo clippy`, and ensure CI passes.

---

## 12. Future Extensions (Optional)
1. **Configurable verbosity** (`--verbosity terse|normal|verbose`).
2. **Custom summariser** – allow a user‑provided prompt via a config file.
3. **Batch mode** – accept a file with many `--task` lines and output a JSON array of summaries.
4. **Model‑based preview** – ask the model to generate the 2‑word tag instead of the simple heuristic.

These can be added later without touching the core pipeline.

---

**End of Specification**
