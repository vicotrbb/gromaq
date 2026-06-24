use gromaq::app::{
    NativeTerminalRuntime, NativeTerminalRuntimeConfig, load_default_native_glyph_cache,
    load_native_glyph_cache,
};
use gromaq::renderer::{GlyphAtlas, GlyphAtlasConfig, RenderPlanner};
use gromaq::{GromaqError, Terminal, TerminalConfig};

use crate::support::{MockFrameRenderer, MockPtySession, system_mono_font_path};

#[test]
fn native_terminal_runtime_invalidates_clean_frame_for_redraw() {
    let mut runtime =
        NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig::default())
            .unwrap();
    let mut renderer = MockFrameRenderer::default();

    assert!(!runtime.render_terminal_frame(&mut renderer).unwrap());
    runtime.invalidate_terminal_frame();

    assert!(runtime.render_terminal_frame(&mut renderer).unwrap());
    let metrics = runtime.dump_runtime_perf_metrics();
    assert_eq!(metrics.render_attempts, 2);
    assert_eq!(metrics.clean_frame_skips, 1);
    assert_eq!(metrics.rendered_frames, 1);
}

#[test]
fn native_terminal_runtime_keeps_frame_dirty_after_renderer_error() {
    let mut runtime =
        NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig::default())
            .unwrap();
    runtime.invalidate_terminal_frame();
    let mut renderer = MockFrameRenderer {
        render_error: Some(GromaqError::GlyphAtlasInvariant {
            reason: "forced renderer failure",
        }),
        ..MockFrameRenderer::default()
    };

    let error = runtime.render_terminal_frame(&mut renderer).unwrap_err();

    assert_eq!(
        error.to_string(),
        "glyph atlas invariant violation: forced renderer failure"
    );
    let metrics = runtime.dump_runtime_perf_metrics();
    assert_eq!(metrics.render_attempts, 1);
    assert_eq!(metrics.rendered_frames, 0);
    assert_eq!(metrics.render_time_samples, 0);

    assert!(runtime.render_terminal_frame(&mut renderer).unwrap());
    let metrics = runtime.dump_runtime_perf_metrics();
    assert_eq!(metrics.render_attempts, 2);
    assert_eq!(metrics.rendered_frames, 1);
    assert_eq!(metrics.render_time_samples, 1);
    assert_eq!(renderer.frames.len(), 1);
}

#[test]
fn default_native_glyph_cache_loads_system_monospace_font() {
    let cache = load_default_native_glyph_cache().unwrap();

    assert!(cache.is_empty());
}

#[test]
fn configured_native_glyph_cache_loads_explicit_font_file_path() {
    let font_path = system_mono_font_path();

    let cache = load_native_glyph_cache(&font_path.to_string_lossy()).unwrap();

    assert!(cache.is_empty());
}

#[test]
fn configured_native_glyph_cache_rejects_missing_explicit_font_file_path() {
    let error = match load_native_glyph_cache("/definitely/missing/gromaq-font.ttf") {
        Ok(_) => panic!("missing explicit font path should be rejected"),
        Err(error) => error,
    };

    assert!(
        error
            .to_string()
            .contains("configured font file does not exist")
    );
}

#[test]
fn configured_native_glyph_cache_rejects_unsupported_font_family_name() {
    let error = match load_native_glyph_cache("Definitely Missing Mono") {
        Ok(_) => panic!("unsupported named font should be rejected"),
        Err(error) => error,
    };

    assert!(
        error
            .to_string()
            .contains("configured font family is not installed or supported by name")
    );
}

#[test]
fn default_native_glyph_cache_rasterizes_emoji_with_fallback_font() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());
    terminal.write_str("😀").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(8).unwrap());
    let mut planner = RenderPlanner::new(24);
    let plan = planner
        .plan_frame(
            &terminal.dump_grid(),
            terminal.dump_cursor(),
            &dirty,
            &mut atlas,
        )
        .unwrap();
    let mut cache = load_default_native_glyph_cache().unwrap();

    let batch = cache.rasterize_plan(&plan).unwrap();

    assert_eq!(batch.rasterized, 1);
    assert_eq!(batch.bitmaps.len(), 1);
    assert!(
        batch.bitmaps[0]
            .rgba
            .chunks_exact(4)
            .any(|pixel| pixel[3] > 0)
    );
}
