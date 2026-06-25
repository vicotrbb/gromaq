use std::{fs, path::Path};

use super::support::relative_path;

const REQUIRED_VISUAL_CONTRACT_DOC_MARKERS: &[(&str, &str)] = &[
    ("README.md", "size_px = 32.0"),
    ("README.md", "line_height_px = 44.0"),
    ("README.md", "background_opacity = 1.0"),
    ("README.md", "surface_padding_px = 14"),
    ("README.md", "cell_spacing_px = 0"),
    ("README.md", "preset = \"gromaq-ghostty\""),
    ("README.md", "cargo run -- --theme-list"),
    ("README.md", "cargo run -- --theme-export gromaq-ghostty"),
    ("README.md", "cargo run -- --runtime-text-zoom-smoke"),
    ("README.md", "cargo run -- --theme-legibility-smoke"),
    (
        "README.md",
        "cargo run -- --theme-preview-snapshot target/gromaq-theme-preview.ppm",
    ),
    ("README.md", "cargo run -- --theme-preview-config"),
    ("documentation/theme.md", "34 px font size"),
    ("documentation/theme.md", "47 px line height"),
    ("documentation/theme.md", "19 px automatic cell width"),
    ("documentation/theme.md", "background_opacity"),
    ("documentation/theme.md", "built-in default is `14`"),
    ("documentation/theme.md", "cell_spacing_px"),
    ("documentation/theme.md", "Control/Super `+`"),
    ("documentation/theme.md", "Control/Super `0`"),
    (
        "documentation/theme.md",
        "`cargo run -- --runtime-text-zoom-smoke`",
    ),
    (
        "documentation/theme.md",
        "`cargo run -- --theme-legibility-smoke`",
    ),
    (
        "documentation/theme.md",
        "`cargo run -- --theme-preview-snapshot",
    ),
    (
        "documentation/theme.md",
        "`cargo run -- --theme-preview-config",
    ),
    ("documentation/theme.md", "gromaq --theme-list"),
    ("documentation/theme.md", "gromaq --theme-export"),
    ("documentation/theme.md", "gromaq --theme-preview-config"),
    ("documentation/compatibility.md", "32/18/44 px"),
    ("documentation/compatibility.md", "37/21/51 px"),
    ("documentation/compatibility.md", "gromaq-ghostty"),
];

#[test]
fn public_docs_keep_default_visual_contract_and_proof_commands() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    for (relative, marker) in REQUIRED_VISUAL_CONTRACT_DOC_MARKERS {
        let path = root.join(relative);
        let source = fs::read_to_string(&path).unwrap();
        assert!(
            source.contains(marker),
            "{} must document `{marker}` for the default visual contract",
            relative_path(root, &path)
        );
    }
}
