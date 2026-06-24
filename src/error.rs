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

    /// Frame target must stay within supported deterministic pacing bounds.
    #[error("target fps must be between {minimum} and {maximum}, got {actual}")]
    InvalidTargetFps {
        /// Inclusive lower bound.
        minimum: u32,
        /// Inclusive upper bound.
        maximum: u32,
        /// Actual invalid value.
        actual: u32,
    },

    /// Theme colors must use a supported hex RGB format.
    #[error("theme color {field} must use #RRGGBB, got {actual}")]
    InvalidThemeColor {
        /// Name of the invalid theme color field.
        field: &'static str,
        /// Actual invalid value.
        actual: String,
    },

    /// Theme surface padding must stay bounded.
    #[error("theme surface padding must be at most {maximum}px, got {actual}px")]
    InvalidThemePadding {
        /// Inclusive upper bound in physical pixels.
        maximum: u16,
        /// Actual invalid value in physical pixels.
        actual: u16,
    },

    /// Configured shell program must contain a usable command.
    #[error("shell program must not be empty")]
    InvalidShellProgram,

    /// Configured shell arguments must contain usable values.
    #[error("shell argument at index {index} must not be empty")]
    InvalidShellArgument {
        /// Index of the invalid argument.
        index: usize,
    },

    /// Configured shell working directory must contain a usable path.
    #[error("shell working directory must not be empty")]
    InvalidShellCwd,

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
