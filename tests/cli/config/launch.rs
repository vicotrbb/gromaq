use super::*;

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
        fallback_families = ["{}"]
        size_px = 16.5

        [shell]
        program = "/bin/zsh"
        args = ["-l", "-i"]
        cwd = "/tmp"

        [welcome]
        enabled = false
        "#,
            font_path.display(),
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
            line_height_px: 44,
            ..RendererConfig::default()
        }
    );
    assert_eq!(launches[0].font_family, font_path.to_string_lossy());
    assert_eq!(
        launches[0].font_fallback_families,
        vec![font_path.to_string_lossy().into_owned()]
    );
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
    assert!(exit.stderr.contains(&format!(
        "run `gromaq --config-check {}` before launch",
        path.display()
    )));
    assert!(backend.requests.borrow().is_empty());
    assert!(app.launches.borrow().is_empty());
    let _ = fs::remove_file(path);
}
