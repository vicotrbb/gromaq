//! Help catalog proof helper for the native tmux UI smoke.

use winit::keyboard::{Key, ModifiersState};

use crate::app::TmuxManagerKeyOutcome;

pub(super) fn drive_help_catalog(runtime: &mut super::SmokeRuntime) -> bool {
    matches!(
        runtime.handle_tmux_manager_key(&Key::Character("?".into()), ModifiersState::empty()),
        TmuxManagerKeyOutcome::Consumed
    )
}
