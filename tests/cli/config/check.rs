use super::*;

#[test]
fn config_check_cli_validates_toml_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let path = test_cli_config_path("valid-config.toml");
    fs::write(
        &path,
        r##"
        [terminal]
        cols = 96
        rows = 32
        scrollback_lines = 2048

        [font]
        family = "Gromaq Mono"
        fallback_families = ["JetBrains Mono Nerd Font", "/tmp/fallback.ttf"]
        size_px = 16.5
        cell_width_px = 11
        line_height_px = 21

        [theme]
        preset = "gromaq-dark"
        background = "#1f2028"
        foreground = "#e8e2d6"
        cursor = "#f4c06a"
        selection = "#26364f"
        background_opacity = 0.42
        cursor_opacity = 0.5
        selection_opacity = 0.25
        cursor_style = "underline"
        cursor_blinking = false
        surface_padding_px = 18
        cell_spacing_px = 2
        dim_opacity = 0.42

        [performance]
        target_fps = 120
        dirty_region_rendering = true

        [shell]
        program = "/bin/zsh"
        args = ["-l"]
        cwd = "/tmp"

        [welcome]
        enabled = false
        "##,
    )
    .unwrap();

    let path_arg = path.to_string_lossy().into_owned();
    let exit = run_with_backend(["gromaq", "--config-check", path_arg.as_str()], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("config check: ok"));
    assert!(exit.stdout.contains("terminal: 96x32"));
    assert!(exit.stdout.contains("scrollback lines: 2048"));
    assert!(exit.stdout.contains("shell: /bin/zsh"));
    assert!(exit.stdout.contains("shell args: -l"));
    assert!(exit.stdout.contains("shell cwd: /tmp"));
    assert!(exit.stdout.contains("welcome enabled: false"));
    assert!(exit.stdout.contains("font: Gromaq Mono 16.5px"));
    assert!(
        exit.stdout
            .contains("font configured fallbacks: JetBrains Mono Nerd Font /tmp/fallback.ttf")
    );
    assert!(
        exit.stdout
            .contains("font source: <unresolved: native runtime failed: configured font family is not installed or supported by name: Gromaq Mono; use an explicit font file path>")
    );
    assert!(exit.stdout.contains("font fallbacks: <unknown>"));
    assert!(exit.stdout.contains("cell width: 11px"));
    assert!(exit.stdout.contains("line height: 21px"));
    assert!(exit.stdout.contains("theme preset: gromaq-dark"));
    assert!(exit.stdout.contains("theme background: #1f2028"));
    assert!(exit.stdout.contains("theme foreground: #e8e2d6"));
    assert!(exit.stdout.contains("theme cursor: #f4c06a"));
    assert!(exit.stdout.contains("theme selection: #26364f"));
    assert!(exit.stdout.contains("theme background opacity: 0.42"));
    assert!(exit.stdout.contains("theme cursor opacity: 0.5"));
    assert!(exit.stdout.contains("theme selection opacity: 0.25"));
    assert!(exit.stdout.contains("theme cursor style: underline"));
    assert!(exit.stdout.contains("theme cursor blinking: false"));
    assert!(exit.stdout.contains("theme surface padding px: 18"));
    assert!(exit.stdout.contains("theme cell spacing px: 2"));
    assert!(exit.stdout.contains("theme dim opacity: 0.42"));
    assert!(exit.stdout.contains("target fps: 120"));
    assert!(exit.stdout.contains("dirty-region rendering: true"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
    let _ = fs::remove_file(path);
}

#[test]
fn config_check_cli_reports_resolved_configured_font_fallbacks() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let path = test_cli_config_path("font-fallback-config.toml");
    let font_path = system_mono_font_path();
    fs::write(
        &path,
        format!(
            r#"
        [font]
        family = "{}"
        fallback_families = ["{}"]
        "#,
            font_path.display(),
            font_path.display()
        ),
    )
    .unwrap();

    let path_arg = path.to_string_lossy().into_owned();
    let exit = run_with_backend(["gromaq", "--config-check", path_arg.as_str()], &backend);

    assert_eq!(exit.code, 0);
    assert!(
        exit.stdout
            .contains(&format!("font source: {}", font_path.display()))
    );
    assert!(exit.stdout.contains(&format!(
        "font configured fallbacks: {}",
        font_path.display()
    )));
    let font_fallbacks = exit
        .stdout
        .lines()
        .find(|line| line.starts_with("font fallbacks:"))
        .unwrap();
    assert_ne!(font_fallbacks, "font fallbacks: <unknown>");
    assert!(!font_fallbacks.contains(&font_path.display().to_string()));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
    let _ = fs::remove_file(path);
}

#[test]
fn config_check_cli_reports_invalid_config_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let path = test_cli_config_path("invalid-config.toml");
    fs::write(&path, "[performance]\ntarget_fps = 0\n").unwrap();

    let path_arg = path.to_string_lossy().into_owned();
    let exit = run_with_backend(["gromaq", "--config-check", path_arg.as_str()], &backend);

    assert_eq!(exit.code, 1);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.contains("config check failed:"));
    assert!(exit.stderr.contains("target fps"));
    assert!(exit.stderr.contains(&format!(
        "run `gromaq --config-check {}` after editing",
        path.display()
    )));
    assert!(backend.requests.borrow().is_empty());
    let _ = fs::remove_file(path);
}

#[test]
fn config_check_cli_requires_path() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--config-check"], &backend);

    assert_eq!(exit.code, 2);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.contains("missing config path"));
    assert!(backend.requests.borrow().is_empty());
}
