use std::cell::RefCell;

use gromaq::cli::run_with_backend;

use super::MockBackend;

#[test]
fn gpu_text_atlas_smoke_cli_reports_font_backed_atlas_upload_result() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--gpu-text-atlas-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("GPU text atlas smoke: ok"));
    assert!(exit.stdout.contains("32x18"));
    assert!(exit.stdout.contains("occupied slots: 2"));
    assert!(exit.stdout.contains("rasterized glyphs: 2"));
    assert!(exit.stdout.contains("reused glyphs: 1"));
    assert!(exit.stdout.contains("covered pixels: 96"));
    assert!(exit.stdout.contains("matching bytes: 2304/2304"));
    assert!(exit.stderr.is_empty());
    assert_eq!(backend.requests.borrow().len(), 1);
}

#[test]
fn gpu_textured_quad_smoke_cli_reports_draw_result() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--gpu-textured-quad-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("GPU textured quad smoke: ok"));
    assert!(exit.stdout.contains("4x4"));
    assert!(exit.stdout.contains("first pixel: [255, 0, 0, 255]"));
    assert!(exit.stdout.contains("drawn pixels: 16"));
    assert!(exit.stderr.is_empty());
    assert_eq!(backend.requests.borrow().len(), 1);
}

#[test]
fn gpu_terminal_text_smoke_cli_reports_draw_result() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--gpu-terminal-text-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("GPU terminal text smoke: ok"));
    assert!(exit.stdout.contains("glyphs: 3"));
    assert!(exit.stdout.contains("background quads: 1"));
    assert!(exit.stdout.contains("quads: 3"));
    assert!(exit.stdout.contains("decoration quads: 1"));
    assert!(exit.stdout.contains("cursor quads: 1"));
    assert!(
        exit.stdout
            .contains("first drawn pixel: [13, 188, 121, 255]")
    );
    assert!(exit.stdout.contains("background pixel: [9, 13, 18, 255]"));
    assert!(exit.stdout.contains("glyph pixel: [244, 247, 251, 255]"));
    assert!(exit.stdout.contains("glyph/background contrast x100: 1842"));
    assert!(exit.stdout.contains("cursor pixel: [229, 229, 229, 255]"));
    assert!(exit.stdout.contains("drawn pixels: 160"));
    assert!(exit.stderr.is_empty());
    assert_eq!(backend.requests.borrow().len(), 1);
}

#[test]
fn gpu_terminal_text_perf_smoke_cli_reports_timing_result() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--gpu-terminal-text-perf-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("GPU terminal text perf smoke: ok"));
    assert!(exit.stdout.contains("frames: 16"));
    assert!(exit.stdout.contains("80x24"));
    assert!(exit.stdout.contains("drawn pixels: 160"));
    assert!(exit.stdout.contains("min ns: 1000"));
    assert!(exit.stdout.contains("avg ns: 2000"));
    assert!(exit.stdout.contains("max ns: 3000"));
    assert!(exit.stdout.contains("p95 ns: 3000"));
    assert!(exit.stderr.is_empty());
    assert_eq!(backend.requests.borrow().len(), 1);
}

#[test]
fn gpu_smoke_cli_reports_readback_result() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--gpu-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("GPU smoke: ok"));
    assert!(exit.stdout.contains("4x4"));
    assert!(exit.stdout.contains("first pixel: [26, 51, 76, 255]"));
    assert!(exit.stderr.is_empty());
    assert_eq!(backend.requests.borrow().len(), 1);
}

#[test]
fn gpu_glyph_atlas_smoke_cli_reports_atlas_upload_result() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--gpu-glyph-atlas-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("GPU glyph atlas smoke: ok"));
    assert!(exit.stdout.contains("4x2"));
    assert!(exit.stdout.contains("occupied slots: 2"));
    assert!(exit.stdout.contains("matching bytes: 32/32"));
    assert!(exit.stderr.is_empty());
    assert_eq!(backend.requests.borrow().len(), 1);
}

#[test]
fn gpu_upload_smoke_cli_reports_upload_readback_result() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--gpu-upload-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("GPU upload smoke: ok"));
    assert!(exit.stdout.contains("2x2"));
    assert!(exit.stdout.contains("first pixel: [255, 0, 0, 255]"));
    assert!(exit.stdout.contains("matching bytes: 16/16"));
    assert!(exit.stderr.is_empty());
    assert_eq!(backend.requests.borrow().len(), 1);
}
