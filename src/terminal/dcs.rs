use vte::Params;

use super::state::DcsHandler;
use super::{MAX_DCS_PAYLOAD_BYTES, Terminal};

impl Terminal {
    pub(super) fn start_dcs_handler(
        &mut self,
        _params: &Params,
        intermediates: &[u8],
        ignore: bool,
        action: char,
    ) {
        self.dcs_payload.clear();
        self.dcs_payload_overflowed = false;
        self.dcs_handler = if !ignore && intermediates == b"$" && action == 'q' {
            Some(DcsHandler::Decrqss)
        } else {
            None
        };
    }

    pub(super) fn push_dcs_byte(&mut self, byte: u8) {
        if self.dcs_handler.is_none() {
            self.dcs_payload.clear();
            return;
        }
        if self.dcs_payload_overflowed {
            return;
        }
        if self.dcs_payload.len() == MAX_DCS_PAYLOAD_BYTES {
            self.dcs_payload_overflowed = true;
            self.dcs_payload.clear();
            return;
        }
        self.dcs_payload.push(byte);
    }

    pub(super) fn finish_dcs_handler(&mut self) {
        let Some(DcsHandler::Decrqss) = self.dcs_handler.take() else {
            self.dcs_payload.clear();
            self.dcs_payload_overflowed = false;
            return;
        };
        if self.dcs_payload_overflowed {
            self.dcs_payload.clear();
            self.dcs_payload_overflowed = false;
            self.report_decrqss(&[]);
        } else {
            let request = std::mem::take(&mut self.dcs_payload);
            self.report_decrqss(&request);
        }
    }
}
