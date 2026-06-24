use gromaq::app::NativeWindowSurface;
use gromaq::native_gpu::NativeGpuWindowSurface;
use gromaq::renderer::{
    BackgroundQuadBatch, GlyphAtlasImage, GlyphBitmap, GlyphEntry, GlyphQuad, GlyphQuadBatch,
    GlyphVertex, SurfaceGlyphFrame, SurfaceLifecycleAction,
};

use super::support::{MockSurfaceBackend, PresentedGlyphFrame, supported_surface_capabilities};

#[test]
fn native_window_surface_configures_and_resizes_surface_backend() {
    let backend = MockSurfaceBackend::default();
    let mut surface = NativeWindowSurface::new(backend, supported_surface_capabilities());

    assert_eq!(
        surface.configure_initial(1280, 800).unwrap(),
        SurfaceLifecycleAction::Configure
    );
    assert_eq!(
        surface.backend().configured_sizes.borrow().as_slice(),
        &[(1280, 800)]
    );
    assert_eq!(surface.configured_size(), Some((1280, 800)));

    assert_eq!(
        surface.resize(1280, 800).unwrap(),
        SurfaceLifecycleAction::None
    );
    assert_eq!(
        surface.backend().configured_sizes.borrow().as_slice(),
        &[(1280, 800)]
    );

    assert_eq!(
        surface.resize(0, 800).unwrap(),
        SurfaceLifecycleAction::DeferZeroSize
    );
    assert!(surface.is_suspended());
    assert_eq!(
        surface.backend().configured_sizes.borrow().as_slice(),
        &[(1280, 800)]
    );

    assert_eq!(
        surface.resize(1440, 900).unwrap(),
        SurfaceLifecycleAction::Reconfigure
    );
    assert_eq!(
        surface.backend().configured_sizes.borrow().as_slice(),
        &[(1280, 800), (1440, 900)]
    );
    assert_eq!(surface.configure_count(), 2);
}

#[test]
fn native_window_surface_presents_clear_frame_through_backend() {
    let backend = MockSurfaceBackend::default();
    let mut surface = NativeWindowSurface::new(backend, supported_surface_capabilities());

    surface.configure_initial(1280, 800).unwrap();
    surface.clear_and_present([0.02, 0.02, 0.025, 1.0]).unwrap();

    assert_eq!(
        surface.backend().presented_clear_colors.borrow().as_slice(),
        &[[0.02, 0.02, 0.025, 1.0]]
    );
}

#[test]
fn native_window_surface_presents_terminal_glyph_frame_through_backend() {
    let backend = MockSurfaceBackend::default();
    let mut surface = NativeWindowSurface::new(backend, supported_surface_capabilities());
    surface.configure_initial(1280, 800).unwrap();
    let atlas = GlyphAtlasImage::pack_rgba8(
        2,
        2,
        1,
        &[GlyphBitmap::try_solid_rgba8(
            GlyphEntry {
                slot: 0,
                generation: 0,
            },
            2,
            2,
            [255, 255, 255, 255],
        )
        .unwrap()],
    )
    .unwrap();
    let batch = GlyphQuadBatch {
        quads: vec![GlyphQuad {
            text: "A".to_owned(),
            ch: 'A',
            atlas_entry: GlyphEntry {
                slot: 0,
                generation: 0,
            },
            vertices: [
                GlyphVertex {
                    position: [0.0, 0.0],
                    uv: [0.0, 0.0],
                    foreground_rgba: [1.0, 1.0, 1.0, 1.0],
                },
                GlyphVertex {
                    position: [2.0, 0.0],
                    uv: [1.0, 0.0],
                    foreground_rgba: [1.0, 1.0, 1.0, 1.0],
                },
                GlyphVertex {
                    position: [2.0, 2.0],
                    uv: [1.0, 1.0],
                    foreground_rgba: [1.0, 1.0, 1.0, 1.0],
                },
                GlyphVertex {
                    position: [0.0, 2.0],
                    uv: [0.0, 1.0],
                    foreground_rgba: [1.0, 1.0, 1.0, 1.0],
                },
            ],
        }],
        indices: vec![0, 1, 2, 0, 2, 3],
    };

    surface
        .present_glyph_frame(SurfaceGlyphFrame {
            atlas: &atlas,
            background_batch: &BackgroundQuadBatch::default(),
            batch: &batch,
            decoration_batch: &BackgroundQuadBatch::default(),
            cursor_batch: &BackgroundQuadBatch::default(),
            width: 2,
            height: 2,
            clear_color: [0.02, 0.02, 0.025, 1.0],
        })
        .unwrap();

    assert_eq!(
        surface.backend().presented_glyph_frames.borrow().as_slice(),
        &[PresentedGlyphFrame {
            width: 2,
            height: 2,
            quads: 1,
            atlas_pixels: 4,
        }]
    );
}

#[test]
fn native_window_surface_configures_from_gpu_surface_handoff() {
    let gpu_surface = NativeGpuWindowSurface::new(
        MockSurfaceBackend::default(),
        supported_surface_capabilities(),
    );

    let surface = NativeWindowSurface::from_gpu_surface(gpu_surface, 1280, 800).unwrap();

    assert_eq!(surface.configured_size(), Some((1280, 800)));
    assert_eq!(surface.configure_count(), 1);
    assert_eq!(
        surface.backend().configured_sizes.borrow().as_slice(),
        &[(1280, 800)]
    );
}
