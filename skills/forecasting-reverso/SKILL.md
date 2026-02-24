---
name: forecasting-reverso
description: >
  Zero-shot univariate time series forecasting using the Reverso foundation
  model (arXiv:2602.17634), ported to Rust with candle framework.
  Activate when users provide time series data and request forecasts,
  predictions, or extrapolations. Supports Reverso Small (550K params).
  Triggers on "forecast", "predict", "time series", "Reverso", or when
  tabular data with a temporal dimension needs future-value estimation.
  Enable with `--forecasting` CLI flag and build with
  `cargo build --features forecasting`.
---

# Reverso Time Series Forecasting (Rust/Candle Port)

Produce zero-shot univariate time series forecasts using the Reverso foundation
model family (arXiv:2602.17634), ported to Rust using the **candle** tensor
framework and **rustfft** for FFT convolutions.

- **Reverso paper**: arXiv:2602.17634
- **Original code**: [shinfxh/reverso](https://github.com/shinfxh/reverso)
- **Model weights**: [shinfxh/reverso on HuggingFace](https://huggingface.co/shinfxh/reverso)

## Prerequisites

### Build with forecasting feature

```bash
cargo build --features forecasting
```

### Obtain model weights

The Reverso model weights must be converted to **safetensors** format for use
with the candle framework.

#### Step 1 — Download the PyTorch checkpoint

```bash
# From HuggingFace
wget https://huggingface.co/shinfxh/reverso/resolve/main/checkpoints/reverso_small/checkpoint.pth \
     -O /tmp/reverso/checkpoint.pth
```

#### Step 2 — Convert to safetensors

Use a small Python script (requires `torch` and `safetensors`):

```python
import torch
from safetensors.torch import save_file

state = torch.load("checkpoint.pth", map_location="cpu")
# Strip any wrapper keys
if "model_state_dict" in state:
    state = state["model_state_dict"]
elif "state_dict" in state:
    state = state["state_dict"]
# Strip DDP prefix
state = {k.removeprefix("module."): v.float() for k, v in state.items()
         if not k.startswith("shared_flashfftconv") and "flashfftconv" not in k}
save_file(state, "reverso_small.safetensors")
```

#### Alternative: user-uploaded weights

If HuggingFace is not accessible, ask the user to download and convert the
checkpoint, then provide the local safetensors path.

## Using the Forecast Tool

The `forecast` tool accepts these parameters:

| Parameter | Type | Required | Description |
|---|---|---|---|
| `series` | array of numbers | ✓ | Historical time series (1-D) |
| `prediction_length` | integer | ✓ | Number of future steps |
| `model_path` | string | ✓ | Path to `.safetensors` weights |
| `model_size` | string | | `"nano"`, `"small"` (default), `"full"` |
| `flip_equivariant` | boolean | | Flip-equivariant averaging (default: false) |

### Example tool call

```json
{
  "name": "forecast",
  "arguments": {
    "series": [1.0, 1.2, 1.1, 1.4, 1.3, 1.5, 1.6, 1.4, 1.7, 1.8],
    "prediction_length": 48,
    "model_path": "/tmp/reverso/reverso_small.safetensors",
    "model_size": "small"
  }
}
```

## Model Configuration

| Variant | d_model | Layers | Params |
|---|---|---|---|
| Nano | 32 | conv,attn | ~200K |
| **Small** | 64 | conv,attn,conv,attn | ~550K |
| Full | 128 | (conv,attn) × 4 | ~2.6M |

## Architecture

The model processes time series through:

1. **Embedding**: Linear projection from scalar to d_model dimensions
2. **Encoder layers** (interleaved):
   - **CNNBlock**: FFT-based circular long convolution with gated activation
   - **MLPBlock**: Two-layer feedforward with ReLU and skip connections
   - **AttentionBlock**: DeltaNet linear attention with short causal convolutions
3. **DecoderHead**: Cross-attention decoder producing 48 output positions per pass

For predictions longer than 48 steps, the model uses autoregressive rollout,
appending each chunk to the context window.

## Input Handling

- Accepts time series as a JSON array of numbers
- NaN values are linearly interpolated
- Series shorter than 2048 steps are left-padded with the first value
- Series longer than 2048 steps use only the most recent 2048 observations
- Min-max normalization to [0, 1] is applied automatically

**Provide at least a few hundred real data points** for meaningful results —
heavily padded context degrades forecast quality.

## Limitations

- The model is strongest with periodic or quasi-periodic signals and full
  2048-point context
- Short series (under ~200 points) produce degraded forecasts due to heavy padding
- Binary-valued input and series ending at exact min-max boundaries are
  out-of-distribution for the training data
- CPU-only inference (no GPU acceleration in this port)
- Requires pre-converted safetensors weights (see Prerequisites)

## Performance Notes

The Rust/candle port runs on CPU. Expected latencies for Reverso Small:

| Phase | Approximate Latency |
|---|---|
| Weight loading (safetensors) | < 1s |
| Forward pass (L=2048) | ~100–200ms |
| 96-step forecast (2 chunks) | ~200–400ms |
| 192-step forecast (4 chunks) | ~400–800ms |

Actual performance depends on CPU and context length. The DeltaNet recurrence
is O(L × d²) per head and is the main bottleneck.
