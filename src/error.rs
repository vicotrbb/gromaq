//! Error types for terminal core operations.

use thiserror::Error;

/// Result alias used by the Gromaq foundation library.
pub type Result<T> = std::result::Result<T, GromaqError>;

/// Errors produced by deterministic terminal-core operations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum GromaqError {
    /// Terminal dimensions must be non-zero and fit the supported model.
    #[error("{field} must be at least {minimum}, got {actual}")]
    InvalidDimension {
        /// Name of the invalid field.
        field: &'static str,
        /// Inclusive lower bound.
        minimum: u32,
        /// Actual invalid value.
        actual: u32,
    },

    /// Terminal grid area must stay within a bounded allocation size.
    #[error("terminal grid must contain at most {maximum} cells, got {actual}")]
    InvalidGridArea {
        /// Inclusive upper bound.
        maximum: u64,
        /// Actual requested cell count.
        actual: u64,
    },

    /// Scrollback capacity must be bounded.
    #[error("scrollback limit must be at most {maximum}, got {actual}")]
    InvalidScrollback {
        /// Inclusive upper bound.
        maximum: usize,
        /// Actual invalid value.
        actual: usize,
    },

    /// Font size must be useful for rendering.
    #[error("font size must be finite and between {minimum} and {maximum}, got {actual}")]
    InvalidFontSize {
        /// Inclusive lower bound.
        minimum: f32,
        /// Inclusive upper bound.
        maximum: f32,
        /// Actual invalid value.
        actual: f32,
    },

    /// Frame target must be non-zero.
    #[error("target fps must be at least {minimum}, got {actual}")]
    InvalidTargetFps {
        /// Inclusive lower bound.
        minimum: u32,
        /// Actual invalid value.
        actual: u32,
    },

    /// Loading a configuration file from disk failed.
    #[error("failed to read config file {path}: {message}")]
    ConfigRead {
        /// Config file path.
        path: String,
        /// Underlying read error message.
        message: String,
    },

    /// Parsing TOML configuration text failed.
    #[error("failed to parse config TOML: {message}")]
    ConfigParse {
        /// Parser error message.
        message: String,
    },

    /// Serializing configuration to TOML failed.
    #[error("failed to serialize config TOML: {message}")]
    ConfigSerialize {
        /// Serializer error message.
        message: String,
    },

    /// Glyph atlas capacity must be bounded and non-zero.
    #[error("glyph atlas capacity must be between {minimum} and {maximum}, got {actual}")]
    InvalidGlyphAtlasCapacity {
        /// Inclusive lower bound.
        minimum: usize,
        /// Inclusive upper bound.
        maximum: usize,
        /// Actual invalid value.
        actual: usize,
    },

    /// Glyph atlas internal structures diverged.
    #[error("glyph atlas invariant violation: {reason}")]
    GlyphAtlasInvariant {
        /// Invariant that was violated.
        reason: &'static str,
    },
}
