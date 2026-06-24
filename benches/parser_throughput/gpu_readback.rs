use std::hint::black_box;

use criterion::Criterion;
use gromaq::native_gpu::{
    GpuBootstrap, GpuBootstrapConfig, GpuGlyphAtlasUploadRunner, GpuTerminalTextRunner,
    GpuTextAtlasUploadRunner, GpuTextureUploadRunner, GpuTexturedQuadRunner, NativeGpuContext,
};

use super::skip_benchmark;

pub(super) fn gpu_textured_quad_draw_readback(c: &mut Criterion) {
    let Some(context) = native_gpu_context_for_benchmark(c, "gpu_textured_quad_draw_readback")
    else {
        return;
    };

    c.bench_function("gpu_textured_quad_draw_readback", |b| {
        b.iter(|| {
            let report = context.run_textured_quad_smoke().unwrap();
            black_box(report.width);
            black_box(report.height);
            black_box(report.drawn_pixels);
        });
    });
}

pub(super) fn gpu_terminal_text_draw_readback(c: &mut Criterion) {
    let Some(context) = native_gpu_context_for_benchmark(c, "gpu_terminal_text_draw_readback")
    else {
        return;
    };

    c.bench_function("gpu_terminal_text_draw_readback", |b| {
        b.iter(|| {
            let report = context.run_terminal_text_smoke().unwrap();
            black_box(report.width);
            black_box(report.height);
            black_box(report.glyphs);
            black_box(report.quads);
            black_box(report.rasterized_glyphs);
            black_box(report.reused_glyphs);
            black_box(report.drawn_pixels);
        });
    });
}

pub(super) fn gpu_text_atlas_upload_readback(c: &mut Criterion) {
    let Some(context) = native_gpu_context_for_benchmark(c, "gpu_text_atlas_upload_readback")
    else {
        return;
    };

    c.bench_function("gpu_text_atlas_upload_readback", |b| {
        b.iter(|| {
            let report = context.run_text_atlas_upload_smoke().unwrap();
            black_box(report.width);
            black_box(report.height);
            black_box(report.occupied_slots);
            black_box(report.rasterized_glyphs);
            black_box(report.reused_glyphs);
            black_box(report.covered_pixels);
            black_box(report.matching_bytes);
            black_box(report.total_bytes);
        });
    });
}

pub(super) fn gpu_texture_upload_readback(c: &mut Criterion) {
    let Some(context) = native_gpu_context_for_benchmark(c, "gpu_texture_upload_readback") else {
        return;
    };

    c.bench_function("gpu_texture_upload_readback", |b| {
        b.iter(|| {
            let report = context.run_texture_upload_smoke().unwrap();
            black_box(report.width);
            black_box(report.height);
            black_box(report.matching_bytes);
            black_box(report.total_bytes);
        });
    });
}

pub(super) fn gpu_glyph_atlas_upload_readback(c: &mut Criterion) {
    let Some(context) = native_gpu_context_for_benchmark(c, "gpu_glyph_atlas_upload_readback")
    else {
        return;
    };

    c.bench_function("gpu_glyph_atlas_upload_readback", |b| {
        b.iter(|| {
            let report = context.run_glyph_atlas_upload_smoke().unwrap();
            black_box(report.width);
            black_box(report.height);
            black_box(report.occupied_slots);
            black_box(report.matching_bytes);
            black_box(report.total_bytes);
        });
    });
}

fn native_gpu_context_for_benchmark(
    c: &mut Criterion,
    name: &'static str,
) -> Option<NativeGpuContext> {
    match GpuBootstrap::new(GpuBootstrapConfig::native_default()).initialize_native() {
        Ok(context) => Some(context),
        Err(error) => {
            skip_benchmark(c, name, &error.to_string());
            None
        }
    }
}
