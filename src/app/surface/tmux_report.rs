use crate::app::NativeTerminalRuntime;
use crate::renderer::RenderPlan;

pub(super) fn plan_contains_tmux_status_pane_command<S>(
    runtime: &NativeTerminalRuntime<S>,
    plan: &RenderPlan,
) -> bool {
    let Some(command) = runtime.last_rendered_tmux_status_pane_command() else {
        return false;
    };
    plan.glyphs.iter().any(|glyph| glyph.text == command)
        || plan
            .glyphs
            .iter()
            .map(|glyph| glyph.text.as_str())
            .collect::<String>()
            .contains(command)
}
