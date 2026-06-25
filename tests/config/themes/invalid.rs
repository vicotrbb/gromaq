use gromaq::{GromaqConfig, GromaqError};

#[test]
fn invalid_theme_colors_are_rejected() {
    let invalid_cases = [
        (
            r##"
            [theme]
            background = "1f2028"
            "##,
            "background",
        ),
        (
            r##"
            [theme]
            foreground = "#zzzzzz"
            "##,
            "foreground",
        ),
        (
            r##"
            [theme]
            cursor = "#12345"
            "##,
            "cursor",
        ),
        (
            r##"
            [theme]
            selection = "#12345"
            "##,
            "selection",
        ),
    ];

    for (toml, field) in invalid_cases {
        let error = GromaqConfig::from_toml_str(toml).unwrap_err();
        assert!(matches!(
            error,
            GromaqError::InvalidThemeColor {
                field: actual_field,
                ..
            } if actual_field == field
        ));
    }
}

#[test]
fn invalid_theme_surface_padding_is_rejected() {
    let error = GromaqConfig::from_toml_str(
        r#"
        [theme]
        surface_padding_px = 513
        "#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GromaqError::InvalidThemePadding {
            maximum: 512,
            actual: 513,
        }
    ));
}

#[test]
fn invalid_theme_cell_spacing_is_rejected() {
    let error = GromaqConfig::from_toml_str(
        r#"
        [theme]
        cell_spacing_px = 33
        "#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GromaqError::InvalidThemeCellSpacing {
            maximum: 32,
            actual: 33,
        }
    ));
}

#[test]
fn invalid_theme_dim_opacity_is_rejected() {
    for dim_opacity in [0.09, f32::NAN, f32::INFINITY, 1.01] {
        let mut config = GromaqConfig::default();
        config.theme.dim_opacity = dim_opacity;

        let error = config.validate().unwrap_err();

        assert!(
            error.to_string().contains("dim opacity"),
            "{error} did not mention dim opacity"
        );
    }
}

#[test]
fn invalid_theme_background_opacity_is_rejected() {
    for background_opacity in [-0.01, f32::NAN, f32::INFINITY, 1.01] {
        let mut config = GromaqConfig::default();
        config.theme.background_opacity = background_opacity;

        let error = config.validate().unwrap_err();

        assert!(
            error.to_string().contains("background opacity"),
            "{error} did not mention background opacity"
        );
    }
}

#[test]
fn invalid_theme_cursor_and_selection_opacity_are_rejected() {
    for (field, invalid_opacity) in [
        ("cursor_opacity", 0.09),
        ("cursor_opacity", f32::NAN),
        ("cursor_opacity", f32::INFINITY),
        ("cursor_opacity", 1.01),
        ("selection_opacity", 0.09),
        ("selection_opacity", f32::NAN),
        ("selection_opacity", f32::INFINITY),
        ("selection_opacity", 1.01),
    ] {
        let mut config = GromaqConfig::default();
        if field == "cursor_opacity" {
            config.theme.cursor_opacity = invalid_opacity;
        } else {
            config.theme.selection_opacity = invalid_opacity;
        }

        let error = config.validate().unwrap_err();

        assert!(
            error.to_string().contains(&field.replace('_', " ")),
            "{error} did not mention {field}"
        );
    }
}

#[test]
fn invalid_theme_ansi_palette_length_is_rejected() {
    let error = GromaqConfig::from_toml_str(
        r##"
        [theme]
        ansi = ["#000000", "#111111"]
        "##,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GromaqError::InvalidThemeAnsiPaletteLength {
            expected: 16,
            actual: 2,
        }
    ));
}
