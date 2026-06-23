//! Host clipboard abstraction.

/// Minimal clipboard interface used by terminal actions.
pub trait HostClipboard {
    /// Read plain text from the clipboard.
    fn read_text(&self) -> Option<String>;
    /// Replace clipboard contents with plain text.
    fn write_text(&mut self, text: &str);
}

/// Deterministic in-memory clipboard adapter for tests.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryClipboard {
    text: Option<String>,
}

impl MemoryClipboard {
    /// Create an in-memory clipboard with initial text.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
        }
    }

    /// Read plain text from the clipboard.
    pub fn read_text(&self) -> Option<String> {
        self.text.clone()
    }

    /// Replace clipboard contents with plain text.
    pub fn write_text(&mut self, text: &str) {
        self.text = Some(text.to_owned());
    }
}

impl HostClipboard for MemoryClipboard {
    fn read_text(&self) -> Option<String> {
        MemoryClipboard::read_text(self)
    }

    fn write_text(&mut self, text: &str) {
        MemoryClipboard::write_text(self, text);
    }
}

/// Native OS clipboard adapter for plain text.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeClipboard;

impl NativeClipboard {
    /// Create a native clipboard adapter.
    pub fn new() -> Self {
        Self
    }

    /// Read plain text from the operating system clipboard.
    pub fn read_text(&self) -> Option<String> {
        arboard::Clipboard::new().ok()?.get_text().ok()
    }

    /// Replace operating system clipboard contents with plain text.
    pub fn write_text(&mut self, text: &str) {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text.to_owned());
        }
    }
}

impl HostClipboard for NativeClipboard {
    fn read_text(&self) -> Option<String> {
        NativeClipboard::read_text(self)
    }

    fn write_text(&mut self, text: &str) {
        NativeClipboard::write_text(self, text);
    }
}
