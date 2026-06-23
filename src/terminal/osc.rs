use base64::{Engine as _, engine::general_purpose};

use super::{MAX_OSC_TITLE_BYTES, MAX_OSC8_HYPERLINK_BYTES, MAX_OSC52_CLIPBOARD_BYTES};

pub(super) fn decode_bounded_osc_text(bytes: &[u8]) -> Option<&str> {
    if bytes.len() > MAX_OSC_TITLE_BYTES {
        return None;
    }
    std::str::from_utf8(bytes).ok()
}

pub(super) fn decode_osc52_clipboard(params: &[&[u8]]) -> Option<String> {
    let selector = params
        .get(1)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())?;
    if !selector.is_empty() && !selector.chars().any(|ch| ch == 'c') {
        return None;
    }
    let payload = params
        .get(2)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())?;
    if payload == "?" {
        return None;
    }
    let max_encoded_len = MAX_OSC52_CLIPBOARD_BYTES.div_ceil(3) * 4;
    if payload.len() > max_encoded_len {
        return None;
    }
    let decoded = general_purpose::STANDARD
        .decode(payload)
        .or_else(|_| general_purpose::STANDARD_NO_PAD.decode(payload))
        .ok()?;
    if decoded.len() > MAX_OSC52_CLIPBOARD_BYTES {
        return None;
    }
    String::from_utf8(decoded).ok()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Osc8HyperlinkAction {
    Open(String),
    Close,
    Ignore,
}

pub(super) fn decode_osc8_hyperlink(params: &[&[u8]]) -> Osc8HyperlinkAction {
    let Some(uri) = params.get(2) else {
        return Osc8HyperlinkAction::Ignore;
    };
    if uri.is_empty() {
        return Osc8HyperlinkAction::Close;
    }
    if uri.len() > MAX_OSC8_HYPERLINK_BYTES {
        return Osc8HyperlinkAction::Ignore;
    }
    let Ok(uri) = std::str::from_utf8(uri) else {
        return Osc8HyperlinkAction::Ignore;
    };
    Osc8HyperlinkAction::Open(uri.to_owned())
}
