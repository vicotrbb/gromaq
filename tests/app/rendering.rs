use gromaq::app::{
    NativeTerminalRuntime, NativeTerminalRuntimeConfig, load_default_native_glyph_cache,
    load_native_glyph_cache,
};
use gromaq::renderer::{GlyphAtlas, GlyphAtlasConfig, RenderPlanner};
use gromaq::{GromaqError, Terminal, TerminalConfig};

use crate::support::{MockFrameRenderer, MockPtySession, system_mono_font_path};

fn host_has_cjk_fallback_font() -> bool {
    [
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Medium.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.otf",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    ]
    .into_iter()
    .any(|path| std::path::Path::new(path).exists())
}

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
fn native_terminal_runtime_renders_status_overlay_without_mutating_terminal_grid() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 24,
        terminal_rows: 4,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    let mut renderer = MockFrameRenderer::default();

    assert!(
        runtime
            .render_terminal_frame_with_status_overlay(&mut renderer, Some("144 fps"))
            .unwrap()
    );

    let frame = renderer.frames.last().unwrap();
    assert_eq!(frame.lines[0], "ready           144 fps");
    assert_eq!(frame.lines[1], ">");
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "ready");
    assert_eq!(runtime.terminal().dump_grid().line_text(1), ">");
    assert!(frame.dirty_regions.iter().any(|region| {
        region.row == 0 && region.col == 16 && region.rows == 1 && region.cols == 7
    }));
}

#[test]
fn native_terminal_runtime_renders_tmux_assist_overlay_once() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 48,
        terminal_rows: 4,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime.write_startup_text("ready\r\n> ").unwrap();
    runtime.show_tmux_assist_overlay();
    let mut renderer = MockFrameRenderer::default();

    assert!(
        runtime
            .render_terminal_frame_with_status_overlay(&mut renderer, Some("144 fps"))
            .unwrap()
    );

    let frame = renderer.frames.last().unwrap();
    assert!(frame.lines[0].contains("tmux split-window -h | Ctrl-b %"));
    assert!(!frame.lines[0].contains("144 fps"));
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "ready");
    assert_eq!(runtime.terminal().dump_grid().line_text(1), ">");

    runtime.invalidate_terminal_frame();
    assert!(
        runtime
            .render_terminal_frame_with_status_overlay(&mut renderer, Some("144 fps"))
            .unwrap()
    );
    assert!(renderer.frames.last().unwrap().lines[0].contains("144 fps"));
}

#[test]
fn native_terminal_runtime_renders_tmux_assist_overlay_below_right_prompt() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 64,
        terminal_rows: 5,
        ..NativeTerminalRuntimeConfig::default()
    })
    .unwrap();
    runtime
        .write_startup_text("ready\r\n................................ rb 2.7.5 15:42\r\n> ")
        .unwrap();
    runtime.show_tmux_assist_overlay();
    let mut renderer = MockFrameRenderer::default();

    assert!(
        runtime
            .render_terminal_frame_with_status_overlay(&mut renderer, Some("144 fps"))
            .unwrap()
    );

    let frame = renderer.frames.last().unwrap();
    assert!(frame.lines[2].contains("tmux split-window -h | Ctrl-b %"));
    assert!(frame.lines[1].contains("rb 2.7.5 15:42"));
    assert_eq!(
        runtime.terminal().dump_grid().line_text(1),
        "................................ rb 2.7.5 15:42"
    );
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

#[test]
fn default_native_glyph_cache_rasterizes_cjk_with_fallback_font_when_available() {
    if !host_has_cjk_fallback_font() {
        eprintln!("skipping CJK fallback proof because no known CJK fallback font is installed");
        return;
    }
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());
    terminal.write_str("界").unwrap();
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
