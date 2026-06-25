use std::cell::RefCell;
use std::fs;

use gromaq::GromaqConfig;
use gromaq::app::{NativeAppConfig, NativeTerminalRuntimeConfig};
use gromaq::cli::{CliExit, NativeAppLaunchConfig};
use gromaq::renderer::RendererConfig;

use super::{
    MockAppLauncher, MockBackend, run_with_backend, run_with_backend_and_app,
    system_mono_font_path, test_cli_config_path,
};

#[test]
fn config_template_cli_prints_parseable_default_toml_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--config-template"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stderr.is_empty());
    assert!(exit.stdout.contains("[terminal]"));
    assert!(exit.stdout.contains("[shell]"));
    assert!(exit.stdout.contains("# program = \"/bin/zsh\""));
    assert!(exit.stdout.contains("[welcome]"));
    assert!(exit.stdout.contains("enabled = true"));
    assert!(exit.stdout.contains("[font]"));
    assert!(exit.stdout.contains("size_px = 34"));
    assert!(exit.stdout.contains("line_height_px = 47"));
    assert!(exit.stdout.contains("# cell_width_px = 19"));
    assert!(exit.stdout.contains("[theme]"));
    assert!(
        exit.stdout
            .contains("# presets: gromaq-dark, gromaq-graphite, gromaq-ghostty")
    );
    assert!(exit.stdout.contains("preset = \"gromaq-ghostty\""));
    assert!(exit.stdout.contains("selection = \"#2f3b52\""));
    assert!(exit.stdout.contains("background_opacity = 1"));
    assert!(exit.stdout.contains("cursor_style = \"block\""));
    assert!(exit.stdout.contains("cursor_blinking = true"));
    assert!(exit.stdout.contains("ansi = [\"#242933\", \"#ff6b7a\""));
    assert!(exit.stdout.contains("surface_padding_px = 14"));
    assert!(exit.stdout.contains("cell_spacing_px = 0"));
    assert!(exit.stdout.contains("dim_opacity = 0.68"));
    assert!(exit.stdout.contains("[performance]"));
    let parsed = GromaqConfig::from_toml_str(&exit.stdout).unwrap();
    assert_eq!(parsed, GromaqConfig::default());
    assert!(backend.requests.borrow().is_empty());
}

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

#[test]
fn no_arguments_launches_native_terminal_app() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = MockAppLauncher {
        launches: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend_and_app(["gromaq"], &backend, &app);

    assert_eq!(
        exit,
        CliExit {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        }
    );
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(app.launches.borrow().len(), 1);
    assert_eq!(app.launches.borrow()[0], NativeAppLaunchConfig::default());
}

#[test]
fn config_launch_cli_loads_config_and_launches_native_app_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = MockAppLauncher {
        launches: RefCell::new(Vec::new()),
    };
    let path = test_cli_config_path("launch-config.toml");
    let font_path = system_mono_font_path();
    fs::write(
        &path,
        format!(
            r#"
        [terminal]
        cols = 132
        rows = 40
        scrollback_lines = 4096

        [performance]
        target_fps = 120
        dirty_region_rendering = false

        [font]
        family = "{}"
        size_px = 16.5

        [shell]
        program = "/bin/zsh"
        args = ["-l", "-i"]
        cwd = "/tmp"

        [welcome]
        enabled = false
        "#,
            font_path.display()
        ),
    )
    .unwrap();

    let path_arg = path.to_string_lossy().into_owned();
    let exit = run_with_backend_and_app(["gromaq", "--config", path_arg.as_str()], &backend, &app);

    assert_eq!(
        exit,
        CliExit {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        }
    );
    assert!(backend.requests.borrow().is_empty());
    let launches = app.launches.borrow();
    assert_eq!(launches.len(), 1);
    assert_eq!(
        launches[0].app,
        NativeAppConfig {
            target_fps: 120,
            welcome_screen: false,
            ..NativeAppConfig::default()
        }
    );
    assert_eq!(
        launches[0].runtime,
        NativeTerminalRuntimeConfig {
            terminal_cols: 132,
            terminal_rows: 40,
            scrollback_lines: 4096,
            shell: gromaq::pty::ShellCommand {
                program: "/bin/zsh".into(),
                args: vec!["-l".into(), "-i".into()],
                cwd: Some("/tmp".into()),
            },
            ..NativeTerminalRuntimeConfig::default()
        }
    );
    assert_eq!(
        launches[0].renderer,
        RendererConfig {
            target_fps: 120,
            dirty_regions: false,
            font_size_px: 17,
            cell_width_px: 9,
            line_height_px: 47,
            ..RendererConfig::default()
        }
    );
    assert_eq!(launches[0].font_family, font_path.to_string_lossy());
    assert_eq!(launches[0].config_path.as_deref(), Some(path.as_path()));
    let _ = fs::remove_file(path);
}

#[test]
fn config_launch_cli_reports_invalid_config_without_launch_or_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = MockAppLauncher {
        launches: RefCell::new(Vec::new()),
    };
    let path = test_cli_config_path("invalid-launch-config.toml");
    fs::write(&path, "[terminal]\ncols = 0\n").unwrap();

    let path_arg = path.to_string_lossy().into_owned();
    let exit = run_with_backend_and_app(["gromaq", "--config", path_arg.as_str()], &backend, &app);

    assert_eq!(exit.code, 1);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.contains("config launch failed:"));
    assert!(exit.stderr.contains("columns"));
    assert!(backend.requests.borrow().is_empty());
    assert!(app.launches.borrow().is_empty());
    let _ = fs::remove_file(path);
}
