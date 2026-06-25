use std::fs;

pub(crate) fn contrast_ratio(foreground: [u8; 3], background: [u8; 3]) -> f64 {
    let foreground = relative_luminance(foreground);
    let background = relative_luminance(background);
    let lighter = foreground.max(background);
    let darker = foreground.min(background);
    (lighter + 0.05) / (darker + 0.05)
}

pub(crate) fn assert_contrast_at_least(
    label: &str,
    foreground: [u8; 3],
    background: [u8; 3],
    minimum: f64,
) {
    let contrast = contrast_ratio(foreground, background);
    assert!(
        contrast >= minimum,
        "{label} contrast ratio {contrast:.2} should be at least {minimum:.2}"
    );
}

fn relative_luminance([red, green, blue]: [u8; 3]) -> f64 {
    let [red, green, blue] = [
        srgb_component(red),
        srgb_component(green),
        srgb_component(blue),
    ];
    (0.2126 * red) + (0.7152 * green) + (0.0722 * blue)
}

fn srgb_component(component: u8) -> f64 {
    let value = f64::from(component) / 255.0;
    if value <= 0.03928 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

pub(crate) fn test_config_path(name: &str) -> std::path::PathBuf {
    let directory = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("gromaq-config-tests");
    fs::create_dir_all(&directory).unwrap();
    directory.join(format!("{}-{name}", std::process::id()))
}
