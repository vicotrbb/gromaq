use criterion::{criterion_group, criterion_main};

#[path = "parser_throughput/gpu_readback.rs"]
mod gpu_readback;
#[path = "parser_throughput/pty_runtime.rs"]
mod pty_runtime;
#[path = "parser_throughput/rendering.rs"]
mod rendering;
#[path = "parser_throughput/support.rs"]
mod support;
#[path = "parser_throughput/terminal.rs"]
mod terminal;

criterion_group!(
    benches,
    terminal::parser_large_output,
    terminal::unicode_emoji_cluster_output,
    terminal::scrollback_large_output,
    terminal::scrollback_view_navigation,
    terminal::dirty_region_coalescing,
    rendering::glyph_atlas_cache_churn,
    rendering::frame_scheduler_144hz_timeline,
    rendering::render_plan_large_dirty_region,
    rendering::glyph_quad_generation_large_plan,
    rendering::rasterized_glyph_cache_hot_plan,
    rendering::prepared_surface_glyph_frame_large_plan,
    rendering::native_input_echo_render_cycle,
    rendering::font_rasterizer_combining_cell,
    pty_runtime::pty_runtime_pump_large_output,
    pty_runtime::real_pty_shell_large_output_burst,
    pty_runtime::real_pty_shell_input_echo_roundtrip,
    pty_runtime::runtime_bounded_state_batches,
    pty_runtime::runtime_state_snapshot_bounded_session,
    pty_runtime::runtime_continuous_output_batches,
    pty_runtime::runtime_alternate_screen_stages,
    pty_runtime::runtime_protocol_input_reports,
    gpu_readback::gpu_textured_quad_draw_readback,
    gpu_readback::gpu_terminal_text_draw_readback,
    gpu_readback::gpu_text_atlas_upload_readback,
    gpu_readback::gpu_texture_upload_readback,
    gpu_readback::gpu_glyph_atlas_upload_readback
);
criterion_main!(benches);
