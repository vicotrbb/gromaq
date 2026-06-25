use crate::cli::CliExit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ThemePreviewSnapshotReport {
    pub(super) preset: &'static str,
    pub(super) bytes_written: usize,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) preview_pixels: usize,
    pub(super) font_size_px: u16,
    pub(super) cell_width_px: u16,
    pub(super) line_height_px: u16,
    pub(super) background_opacity_percent: u32,
    pub(super) cursor_opacity_percent: u32,
    pub(super) selection_opacity_percent: u32,
    pub(super) surface_padding_px: u16,
    pub(super) cell_spacing_px: u16,
    pub(super) high_contrast_text_pixels: usize,
    pub(super) selection_pixels: usize,
    pub(super) cursor_pixels: usize,
    pub(super) prepared_quads: usize,
    pub(super) background_quads: usize,
    pub(super) cursor_quads: usize,
    pub(super) atlas_bytes: usize,
}

pub(super) fn theme_preview_snapshot_success(
    path: &str,
    report: &ThemePreviewSnapshotReport,
) -> CliExit {
    CliExit {
        code: 0,
        stdout: format!(
            "theme preview snapshot: ok\npath: {path}\npreset: {}\nbytes written: {}\nframe size: {}x{}\npreview pixels: {}\nfont size px: {}\ncell width px: {}\nline height px: {}\nbackground opacity percent: {}\ncursor opacity percent: {}\nselection opacity percent: {}\nsurface padding px: {}\ncell spacing px: {}\nhigh contrast text pixels: {}\nselection pixels: {}\ncursor pixels: {}\nprepared quads: {}\nbackground quads: {}\ncursor quads: {}\natlas bytes: {}\n",
            report.preset,
            report.bytes_written,
            report.width,
            report.height,
            report.preview_pixels,
            report.font_size_px,
            report.cell_width_px,
            report.line_height_px,
            report.background_opacity_percent,
            report.cursor_opacity_percent,
            report.selection_opacity_percent,
            report.surface_padding_px,
            report.cell_spacing_px,
            report.high_contrast_text_pixels,
            report.selection_pixels,
            report.cursor_pixels,
            report.prepared_quads,
            report.background_quads,
            report.cursor_quads,
            report.atlas_bytes
        ),
        stderr: String::new(),
    }
}

pub(super) fn theme_preview_snapshot_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("theme preview snapshot failed: {error}\n"),
    }
}
