use super::*;

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
    assert!(exit.stdout.contains("[tmux]"));
    assert!(exit.stdout.contains("open_manager_on_start = true"));
    assert!(exit.stdout.contains("[font]"));
    assert!(
        exit.stdout
            .contains("family = \"JetBrains Mono Nerd Font\"")
    );
    assert!(
        exit.stdout
            .contains("# fallback_families = [\"Apple Color Emoji\"]")
    );
    assert!(exit.stdout.contains("size_px = 32"));
    assert!(exit.stdout.contains("line_height_px = 44"));
    assert!(exit.stdout.contains("# cell_width_px = 18"));
    assert!(exit.stdout.contains("[theme]"));
    assert!(
        exit.stdout
            .contains("# presets: gromaq-dark, gromaq-graphite, gromaq-ghostty")
    );
    assert!(exit.stdout.contains("preset = \"gromaq-ghostty\""));
    assert!(exit.stdout.contains("selection = \"#2f3b52\""));
    assert!(exit.stdout.contains("background_opacity = 1"));
    assert!(exit.stdout.contains("cursor_opacity = 1"));
    assert!(exit.stdout.contains("selection_opacity = 1"));
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
