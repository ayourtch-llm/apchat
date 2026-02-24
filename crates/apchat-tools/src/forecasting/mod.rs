//! Reverso Time Series Forecasting — Candle/Rust Port
//!
//! Zero-shot univariate time series forecasting using the Reverso foundation
//! model family (arXiv:2602.17634), implemented in Rust with the candle
//! tensor framework and rustfft for FFT convolutions.
//!
//! # Attribution
//!
//! Original Python/NumPy/Numba implementation by **@oaustegard**:
//! <https://github.com/oaustegard/claude-skills/tree/main/forecasting-reverso>
//!
//! Reverso model paper: *arXiv:2602.17634*
//! Model weights: <https://huggingface.co/shinfxh/reverso>

pub mod config;
pub mod model;
pub mod ops;
pub mod tool;

pub use tool::ForecastTool;
