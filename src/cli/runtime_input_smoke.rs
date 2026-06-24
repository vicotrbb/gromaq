//! Runtime input and performance CLI smoke commands.

pub(super) use perf_smoke::{
    runtime_idle_smoke_exit, runtime_perf_budget_smoke_exit, runtime_perf_smoke_exit,
};
pub(super) use protocol_smoke::{
    runtime_focus_smoke_exit, runtime_mouse_smoke_exit, runtime_response_smoke_exit,
};

mod perf_smoke;
mod protocol_smoke;
mod pty_smoke;
