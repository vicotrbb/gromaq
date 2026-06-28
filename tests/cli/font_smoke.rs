use std::cell::RefCell;

use super::{MockBackend, run_with_backend};

#[test]
fn font_symbol_fallback_smoke_rasterizes_braille_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--font-symbol-fallback-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("font symbol fallback smoke: ok"));
    assert!(exit.stdout.contains("sample:"));
    assert!(exit.stdout.contains("glyphs rasterized: 1"));
    assert!(exit.stdout.contains("bitmap:"));
    assert!(exit.stdout.contains("alpha pixels:"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}
