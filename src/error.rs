//! Error types for terminal core operations.

mod kind;

pub use kind::GromaqError;

/// Result alias used by the Gromaq foundation library.
pub type Result<T> = std::result::Result<T, GromaqError>;
