//! GPU-specific CLI command dispatch and output formatting.

use super::CliExit;
use crate::native_gpu::{
    GpuAdapterSnapshot, GpuBootstrap, GpuBootstrapBackend, GpuBootstrapConfig, GpuBootstrapError,
    GpuGlyphAtlasUploadRunner, GpuSmokeRunner, GpuTerminalTextRunner, GpuTextAtlasUploadRunner,
    GpuTextureUploadRunner, GpuTexturedQuadRunner,
};

/// Adapter metadata reporting abstraction.
pub trait AdapterReport {
    /// Return stable adapter metadata.
    fn adapter_report(&self) -> &GpuAdapterSnapshot;
}

/// Full GPU context capability set required by CLI GPU commands.
pub trait GpuCommandContext:
    AdapterReport
    + GpuSmokeRunner
    + GpuTextureUploadRunner
    + GpuGlyphAtlasUploadRunner
    + GpuTextAtlasUploadRunner
    + GpuTexturedQuadRunner
    + GpuTerminalTextRunner
{
}

impl<T> GpuCommandContext for T where
    T: AdapterReport
        + GpuSmokeRunner
        + GpuTextureUploadRunner
        + GpuGlyphAtlasUploadRunner
        + GpuTextAtlasUploadRunner
        + GpuTexturedQuadRunner
        + GpuTerminalTextRunner
{
}

impl AdapterReport for GpuAdapterSnapshot {
    fn adapter_report(&self) -> &GpuAdapterSnapshot {
        self
    }
}

impl GpuSmokeRunner for GpuAdapterSnapshot {
    fn run_smoke(&self) -> Result<crate::native_gpu::GpuSmokeReport, GpuBootstrapError> {
        Err(metadata_without_live_context_error())
    }
}

impl GpuTextureUploadRunner for GpuAdapterSnapshot {
    fn run_texture_upload_smoke(
        &self,
    ) -> Result<crate::native_gpu::GpuTextureUploadReport, GpuBootstrapError> {
        Err(metadata_without_live_context_error())
    }
}

impl GpuGlyphAtlasUploadRunner for GpuAdapterSnapshot {
    fn run_glyph_atlas_upload_smoke(
        &self,
    ) -> Result<crate::native_gpu::GpuGlyphAtlasUploadReport, GpuBootstrapError> {
        Err(metadata_without_live_context_error())
    }
}

impl GpuTextAtlasUploadRunner for GpuAdapterSnapshot {
    fn run_text_atlas_upload_smoke(
        &self,
    ) -> Result<crate::native_gpu::GpuTextAtlasUploadReport, GpuBootstrapError> {
        Err(metadata_without_live_context_error())
    }
}

impl GpuTexturedQuadRunner for GpuAdapterSnapshot {
    fn run_textured_quad_smoke(
        &self,
    ) -> Result<crate::native_gpu::GpuTexturedQuadReport, GpuBootstrapError> {
        Err(metadata_without_live_context_error())
    }
}

impl GpuTerminalTextRunner for GpuAdapterSnapshot {
    fn run_terminal_text_smoke(
        &self,
    ) -> Result<crate::native_gpu::GpuTerminalTextReport, GpuBootstrapError> {
        Err(metadata_without_live_context_error())
    }
}

impl AdapterReport for crate::native_gpu::NativeGpuContext {
    fn adapter_report(&self) -> &GpuAdapterSnapshot {
        self.adapter()
    }
}

pub(super) fn gpu_command_exit<B>(arg: &str, backend: &B) -> CliExit
where
    B: GpuBootstrapBackend,
    B::Context: GpuCommandContext,
{
    let bootstrap = GpuBootstrap::new(GpuBootstrapConfig::native_default());
    match bootstrap.initialize_with(backend) {
        Ok(context) if arg == "--gpu-info" => gpu_info_exit(context.adapter_report()),
        Ok(context) if arg == "--gpu-smoke" => match context.run_smoke() {
            Ok(report) => CliExit {
                code: 0,
                stdout: format!(
                    "GPU smoke: ok\nsize: {}x{}\nfirst pixel: {:?}\nnon-zero bytes: {}\n",
                    report.width, report.height, report.first_pixel, report.nonzero_bytes
                ),
                stderr: String::new(),
            },
            Err(error) => CliExit::from(error),
        },
        Ok(context) if arg == "--gpu-upload-smoke" => match context.run_texture_upload_smoke() {
            Ok(report) => CliExit {
                code: 0,
                stdout: format!(
                    "GPU upload smoke: ok\nsize: {}x{}\nfirst pixel: {:?}\nlast pixel: {:?}\nmatching bytes: {}/{}\n",
                    report.width,
                    report.height,
                    report.first_pixel,
                    report.last_pixel,
                    report.matching_bytes,
                    report.total_bytes
                ),
                stderr: String::new(),
            },
            Err(error) => CliExit::from(error),
        },
        Ok(context) if arg == "--gpu-glyph-atlas-smoke" => {
            match context.run_glyph_atlas_upload_smoke() {
                Ok(report) => CliExit {
                    code: 0,
                    stdout: format!(
                        "GPU glyph atlas smoke: ok\nsize: {}x{}\noccupied slots: {}\nfirst pixel: {:?}\nsecond slot first pixel: {:?}\nmatching bytes: {}/{}\n",
                        report.width,
                        report.height,
                        report.occupied_slots,
                        report.first_pixel,
                        report.second_slot_first_pixel,
                        report.matching_bytes,
                        report.total_bytes
                    ),
                    stderr: String::new(),
                },
                Err(error) => CliExit::from(error),
            }
        }
        Ok(context) if arg == "--gpu-text-atlas-smoke" => {
            match context.run_text_atlas_upload_smoke() {
                Ok(report) => CliExit {
                    code: 0,
                    stdout: format!(
                        "GPU text atlas smoke: ok\nsize: {}x{}\noccupied slots: {}\nrasterized glyphs: {}\nreused glyphs: {}\ncovered pixels: {}\nmatching bytes: {}/{}\n",
                        report.width,
                        report.height,
                        report.occupied_slots,
                        report.rasterized_glyphs,
                        report.reused_glyphs,
                        report.covered_pixels,
                        report.matching_bytes,
                        report.total_bytes
                    ),
                    stderr: String::new(),
                },
                Err(error) => CliExit::from(error),
            }
        }
        Ok(context) if arg == "--gpu-textured-quad-smoke" => {
            match context.run_textured_quad_smoke() {
                Ok(report) => CliExit {
                    code: 0,
                    stdout: format!(
                        "GPU textured quad smoke: ok\nsize: {}x{}\nfirst pixel: {:?}\ndrawn pixels: {}\n",
                        report.width, report.height, report.first_pixel, report.drawn_pixels
                    ),
                    stderr: String::new(),
                },
                Err(error) => CliExit::from(error),
            }
        }
        Ok(context) => match context.run_terminal_text_smoke() {
            Ok(report) => CliExit {
                code: 0,
                stdout: format!(
                    "GPU terminal text smoke: ok\nsize: {}x{}\nglyphs: {}\nbackground quads: {}\nquads: {}\ndecoration quads: {}\ncursor quads: {}\nrasterized glyphs: {}\nreused glyphs: {}\nfirst drawn pixel: {:?}\ncursor pixel: {:?}\ndrawn pixels: {}\n",
                    report.width,
                    report.height,
                    report.glyphs,
                    report.background_quads,
                    report.quads,
                    report.decoration_quads,
                    report.cursor_quads,
                    report.rasterized_glyphs,
                    report.reused_glyphs,
                    report.first_drawn_pixel,
                    report.cursor_pixel,
                    report.drawn_pixels
                ),
                stderr: String::new(),
            },
            Err(error) => CliExit::from(error),
        },
        Err(error) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!("{error}\n"),
        },
    }
}

fn metadata_without_live_context_error() -> GpuBootstrapError {
    GpuBootstrapError::SmokeReadback("adapter metadata does not own a live GPU context".to_owned())
}

fn format_adapter(adapter: &GpuAdapterSnapshot) -> String {
    format!(
        "GPU adapter: {}\nbackend: {}\ndevice type: {}\nvendor: {}\ndevice: {}\n",
        adapter.name, adapter.backend, adapter.device_type, adapter.vendor, adapter.device
    )
}

fn gpu_info_exit(adapter: &GpuAdapterSnapshot) -> CliExit {
    CliExit {
        code: 0,
        stdout: format_adapter(adapter),
        stderr: String::new(),
    }
}

impl From<GpuBootstrapError> for CliExit {
    fn from(value: GpuBootstrapError) -> Self {
        Self {
            code: 1,
            stdout: String::new(),
            stderr: format!("{value}\n"),
        }
    }
}
