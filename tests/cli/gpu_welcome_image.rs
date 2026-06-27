use std::cell::RefCell;
use std::fs;

use super::{MockBackend, run_with_backend};

#[test]
fn welcome_image_snapshot_cli_writes_ppm_artifact() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let path = std::env::temp_dir().join(format!(
        "gromaq-cli-{}-welcome-image.ppm",
        std::process::id()
    ));
    let _ = fs::remove_file(&path);

    let exit = run_with_backend(
        ["gromaq", "--welcome-image-snapshot", path.to_str().unwrap()],
        &backend,
    );

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("welcome image snapshot: ok"));
    assert!(exit.stdout.contains("size: 2x2"));
    assert!(exit.stdout.contains("bytes written: 23"));
    assert!(exit.stdout.contains("background pixel: [16, 18, 22, 255]"));
    assert!(exit.stdout.contains("image pixel: [234, 214, 255, 255]"));
    assert!(exit.stdout.contains("drawn pixels: 2"));
    assert!(exit.stderr.is_empty());
    assert_eq!(backend.requests.borrow().len(), 1);
    assert_eq!(
        fs::read(&path).unwrap(),
        b"P6\n2 2\n255\n\x10\x12\x16\xea\xd6\xff\xea\xd6\xff\x10\x12\x16"
    );

    fs::remove_file(path).unwrap();
}

#[test]
fn welcome_image_snapshot_cli_requires_output_path() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--welcome-image-snapshot"], &backend);

    assert_eq!(exit.code, 2);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.starts_with("usage: gromaq ["));
    assert!(
        exit.stderr
            .contains("missing snapshot path for --welcome-image-snapshot")
    );
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn welcome_image_snapshot_cli_rejects_extra_arguments() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let path = std::env::temp_dir().join(format!(
        "gromaq-cli-{}-welcome-image-extra.ppm",
        std::process::id()
    ));
    let _ = fs::remove_file(&path);

    let exit = run_with_backend(
        [
            "gromaq",
            "--welcome-image-snapshot",
            path.to_str().unwrap(),
            "extra",
        ],
        &backend,
    );

    assert_eq!(exit.code, 2);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.starts_with("usage: gromaq ["));
    assert!(exit.stderr.contains("unexpected extra argument: extra"));
    assert!(backend.requests.borrow().is_empty());
    assert!(!path.exists());
}
