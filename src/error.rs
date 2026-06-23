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

    /// Scrollback capacity must be bounded.
    #[error("scrollback limit must be at most {maximum}, got {actual}")]
    InvalidScrollback {
        /// Inclusive upper bound.
        maximum: usize,
        /// Actual invalid value.
        actual: usize,
    },

    /// Font size must be useful for rendering.
    #[error("font size must be at least {minimum}, got {actual}")]
    InvalidFontSize {
        /// Inclusive lower bound.
        minimum: f32,
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

    /// A text selection must contain at least one cell.
    #[error("selection must contain at least one cell")]
    EmptySelection,

    /// Glyph atlas capacity must be non-zero.
    #[error("glyph atlas capacity must be at least {minimum}, got {actual}")]
    InvalidGlyphAtlasCapacity {
        /// Inclusive lower bound.
        minimum: usize,
        /// Actual invalid value.
        actual: usize,
    },
}
