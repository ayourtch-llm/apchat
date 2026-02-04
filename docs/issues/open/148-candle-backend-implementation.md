# Issue 148: Implement Candle Embedding Backend

## Summary

The `CandleBackend` in `crates/apchat-skills/src/embeddings/candle_backend.rs` is a stub that returns errors. It needs a full implementation using the Candle ML library for embedding generation.

## Location
- File: `crates/apchat-skills/src/embeddings/candle_backend.rs`
- Function: `CandleBackend::new` and `CandleBackend::embed`

## Current Behavior

The backend always returns errors:
```rust
pub fn new() -> Result<Self> {
    anyhow::bail!("Candle backend not yet implemented. Use fastembed instead.");
}

fn embed(&self, _text: &str) -> Result<Vec<f32>> {
    anyhow::bail!("Candle backend not yet implemented")
}
```

## Expected Behavior

The backend should:
1. Load a sentence-transformers model using Candle
2. Tokenize input text
3. Run model inference
4. Extract embeddings from the model output
5. Return the embedding vector

## Impact

- **Feature Parity**: Users who prefer Candle over fastembed can't use it
- **Flexibility**: Users are limited to one embedding backend
- **Dependencies**: The candle dependency is already in the codebase but unused

## Suggested Implementation

Based on the existing comments in the file:

1. **Add dependencies** to `Cargo.toml`:
   ```toml
   candle = { version = "0.7", features = ["nn"] }
   candle-transformers = "0.7"
   tokenizers = "0.19"
   ```

2. **Load model**:
   ```rust
   use candle::{Device, Tensor};
   use candle_transformers::models::bert;
   
   struct CandleBackend {
       model: BertModel,
       tokenizer: Tokenizer,
       device: Device,
   }
   ```

3. **Implement tokenization** using huggingface/tokenizers
4. **Implement embedding extraction** from model output
5. **Add configuration** for model path/source

## Alternative Approach

If Candle implementation is too complex:
- Mark backend as optional feature
- Provide clear error message suggesting fastembed as the default
- Create issue to document the path forward

## Resolution

(TO BE ADDED WHEN FIXED)

---
*Created: 2026-02-04*
*Resolved: (TO BE ADDED)*
