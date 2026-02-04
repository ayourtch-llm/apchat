# Issue 148: Implement Candle Embedding Backend

## Summary

The `CandleBackend` in `crates/apchat-skills/src/embeddings/candle_backend.rs` has been fully implemented. It now uses the Candle ML library for embedding generation as an optional feature behind the `candle` feature flag.

## Location
- File: `crates/apchat-skills/src/embeddings/candle_backend.rs`
- Function: `CandleBackend::new` and `CandleBackend::embed`

## Current Behavior

The backend is fully implemented and functional:
- Loads sentence-transformers models (all-MiniLM-L6-v2 by default) from Hugging Face Hub
- Tokenizes input text using the tokenizers library
- Runs model inference on the Candle ML backend
- Extracts embeddings from model output (pooled output from BERT)
- Returns embedding vectors as `Vec<f32>`

The backend is available as an optional feature and must be enabled with `--features candle`.

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

The implementation includes:
1. **Dependencies** in `Cargo.toml` with optional feature flag:
   ```toml
   candle = { version = "0.7", features = ["metal"] }
   candle-transformers = "0.7"
   tokenizers = "0.19"
   reqwest = { version = "0.12", features = ["blocking", "json"] }
   ```

2. **Model loading** with Hugging Face download:
   - Downloads tokenizer.json, config.json, and model.safetensors
   - Caches models in ~/.okaychat/candle

3. **Tokenization** using the tokenizers library
4. **Embedding extraction** from BERT pooled output
5. **Batch embedding** support with `embed_batch` method

## Resolution

✅ **FIXED** - The Candle embedding backend is fully implemented:
- Loads all-MiniLM-L6-v2 model by default (384-dimensional embeddings)
- Downloads models from Hugging Face Hub automatically
- Supports CUDA and CPU devices
- Includes optional `candle` feature flag
- Provides both single and batch embedding methods

---
*Created: 2026-02-04*
*Resolved: 2026-02-04*

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
