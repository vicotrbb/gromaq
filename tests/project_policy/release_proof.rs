use std::{fs, path::Path};

const REQUIRED_GITHUB_RELEASE_ASSETS: &[&str] = &[
    "gromaq-${version_without_prefix}-linux-${release_arch}.tar.gz",
    "gromaq_${version_without_prefix}_${deb_arch}.deb",
    "PKGBUILD",
    "default.SRCINFO",
    "gromaq.install",
    "Gromaq-macos-app.zip",
    "SHA256SUMS-linux-${release_arch}",
    "SHA256SUMS-macos-app",
];

#[test]
fn github_release_install_proof_checks_complete_published_asset_set() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let proof_script =
        fs::read_to_string(root.join("scripts/prove-github-release-install.sh")).unwrap();

    assert!(proof_script.contains("gh release view"));
    assert!(proof_script.contains("--json assets"));
    assert!(proof_script.contains("verify_release_asset"));
    assert!(proof_script.contains("published release asset missing"));

    for asset in REQUIRED_GITHUB_RELEASE_ASSETS {
        assert!(
            proof_script.contains(asset),
            "GitHub release install proof must verify published asset `{asset}`"
        );
    }
}

#[test]
fn github_release_publication_proof_checks_tag_and_assets() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let proof_script =
        fs::read_to_string(root.join("scripts/prove-github-release-publication.sh")).unwrap();

    for marker in [
        "gh release view",
        "--json tagName,isDraft,isPrerelease,url,assets",
        "tagName",
        "isDraft",
        "verify_release_asset",
        "published release asset missing",
        "GitHub Release ${version} was not found",
        "gromaq-${version_without_prefix}-linux-${release_arch}.tar.gz",
        "gromaq_${version_without_prefix}_${deb_arch}.deb",
        "PKGBUILD",
        "default.SRCINFO",
        "gromaq.install",
        "Gromaq-macos-app.zip",
        "SHA256SUMS-linux-${release_arch}",
        "SHA256SUMS-macos-app",
        "GitHub release publication proof: ok",
        "summary.txt",
    ] {
        assert!(
            proof_script.contains(marker),
            "GitHub release publication proof must include marker `{marker}`"
        );
    }
}

#[test]
fn github_release_macos_install_proof_checks_downloaded_app_bundle() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let proof_script =
        fs::read_to_string(root.join("scripts/prove-github-release-macos-install.sh")).unwrap();

    for marker in [
        "Darwin",
        "gh release view",
        "Gromaq-macos-app.zip",
        "SHA256SUMS-macos-app",
        "checksum mismatch",
        "extract_release_zip",
        "Contents/MacOS/${package}",
        "CFBundleShortVersionString",
        "lipo -info",
        "codesign --verify --deep --strict --verbose=4",
        "spctl -a -vvv -t execute",
        "GROMAQ_EXPECT_NOTARIZED",
        "GROMAQ_INSTALL_METHOD=release",
        "GROMAQ_MACOS_APP_DIR",
        "GitHub release macOS install proof: ok",
        "summary.txt",
    ] {
        assert!(
            proof_script.contains(marker),
            "GitHub release macOS install proof must include marker `{marker}`"
        );
    }
}

#[test]
fn macos_live_input_manual_proof_guides_installed_app_typing() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let proof_script =
        fs::read_to_string(root.join("scripts/prove-macos-live-input-manual.sh")).unwrap();

    for marker in [
        "Darwin",
        "GROMAQ_MANUAL_INPUT_APP",
        "open -W -n",
        "typed-ls.txt",
        "typed-pwd.txt",
        "typed-unicode.txt",
        "Type exactly: ls",
        "Type exactly: pwd",
        "Type exactly: unicode:界́🙂",
        "macOS live input manual proof: ok",
        "summary.txt",
    ] {
        assert!(
            proof_script.contains(marker),
            "macOS live input manual proof must include marker `{marker}`"
        );
    }
}

#[test]
fn macos_native_tmux_manual_proof_guides_live_manager_workflow() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let proof_script =
        fs::read_to_string(root.join("scripts/prove-macos-native-tmux-manual.sh")).unwrap();

    for marker in [
        "Darwin",
        "tmux -V",
        "git -C \"${root}\" status --short --branch",
        "git-status.txt",
        "git branch:",
        "git dirty:",
        "GROMAQ_MANUAL_TMUX_APP",
        "GROMAQ_MANUAL_TMUX_BINARY",
        "GROMAQ_MANUAL_TMUX_OPEN_ON_START",
        "cargo build",
        "target/debug/gromaq",
        "open -W -n",
        "gromaq-native-tmux-manual.toml",
        "manual-tmux-shell.sh",
        "tmux-session.txt",
        "tmux-status-strip-visible.txt",
        "tmux-manager-visible.txt",
        "tmux-navigation-checked.txt",
        "tmux-right-prompt-legible.txt",
        "tmux-start-session.txt",
        "tmux-start-session-exists.txt",
        "started-session exists: true",
        "tmux-attach-session.txt",
        "tmux-safe-action.txt",
        "tmux-new-window.txt",
        "tmux-post-windows.txt",
        "tmux-post-panes.txt",
        "tmux-kill-session-absent.txt",
        "post tmux windows:",
        "post tmux panes:",
        "kill-session absent: true",
        "tmux-refresh-checked.txt",
        "tmux-destructive-confirmation.txt",
        "tmux-workspace-visible.txt",
        "tmux-normal-shell-input.txt",
        "launch-mode.txt",
        "open-manager-on-start.txt",
        "Native tmux manager manual proof",
        "macOS native tmux manual proof: ok",
        "summary.txt",
    ] {
        assert!(
            proof_script.contains(marker),
            "macOS native tmux manual proof must include marker `{marker}`"
        );
    }
}

#[test]
fn macos_native_tmux_default_cargo_run_proof_guides_no_arg_workflow() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let proof_script =
        fs::read_to_string(root.join("scripts/prove-macos-native-tmux-default-cargo-run.sh"))
            .unwrap();

    for marker in [
        "Darwin",
        "tmux -V",
        "cargo run",
        "git -C \"${root}\" status --short --branch",
        "git-status.txt",
        "git branch:",
        "git dirty:",
        "tmux Cmd/Ctrl+Shift+T",
        "keyboard, mouse, paste",
        "target/debug/gromaq",
        "strings \"${binary_path}\"",
        "tmux-default-cargo-run-binary-markers.txt",
        "cargo run -- --window-smoke",
        "tmux-default-cargo-run-window-smoke.stdout",
        "default startup content checked: true",
        "tmux status strip rendered: true",
        "tmux status pane command rendered:",
        "tmux manager panel rendered: true",
        "cargo run -- --runtime-tmux-ui-smoke",
        "tmux-default-cargo-run-runtime-tmux-ui-smoke.stdout",
        "skipped pty handoffs checked: attach=true start=true workspace=true",
        "workspace duplicate prevented: true",
        "cargo run -- --window-tmux-manager-snapshot",
        "tmux-default-cargo-run-manager-reference.ppm",
        "tmux-default-cargo-run-manager-reference.png",
        "tmux-default-cargo-run-manager-reference.stdout",
        "tmux-default-cargo-run-manager-reference.stderr",
        "tmux status pane command rendered: true",
        "tmux manager sessions:",
        "tmux manager windows:",
        "tmux manager panes:",
        "Expected manager reference snapshot:",
        "unexpected old startup marker",
        "tmux-default-cargo-run-current-startup.txt",
        "tmux-default-cargo-run-status-strip.txt",
        "If the manager is already visible on startup, close it with Esc",
        "Confirm Control/Super Shift+T opened or reopened a real manager panel.",
        "tmux-default-cargo-run-manager-visible.txt",
        "tmux-default-cargo-run-not-hint.txt",
        "tmux-default-cargo-run-state-visible.txt",
        "tmux-default-cargo-run-navigation.txt",
        "tmux-default-cargo-run-start-session.txt",
        "tmux-default-cargo-run-start-session-exists.txt",
        "started-session exists: true",
        "tmux-default-cargo-run-attach-session.txt",
        "tmux-default-cargo-run-safe-action.txt",
        "tmux-default-cargo-run-new-window.txt",
        "tmux-default-cargo-run-post-windows.txt",
        "tmux-default-cargo-run-post-panes.txt",
        "tmux-default-cargo-run-kill-session-absent.txt",
        "post tmux windows:",
        "post tmux panes:",
        "kill-session absent: true",
        "tmux-default-cargo-run-destructive-confirmation.txt",
        "tmux-default-cargo-run-isolated-kill.txt",
        "tmux-default-cargo-run-shell-input.txt",
        "tmux-default-cargo-run-right-prompt.txt",
        "macOS native tmux default cargo run proof: ok",
        "summary.txt",
    ] {
        assert!(
            proof_script.contains(marker),
            "macOS native tmux default cargo run proof must include marker `{marker}`"
        );
    }
}

#[test]
fn macos_native_tmux_default_cargo_run_proof_checks_binary_before_window() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let proof_script =
        fs::read_to_string(root.join("scripts/prove-macos-native-tmux-default-cargo-run.sh"))
            .unwrap();

    let build = proof_script.find("cargo build").unwrap();
    let marker_check = proof_script
        .find("does not contain current startup marker")
        .unwrap();
    let stale_check = proof_script.find("unexpected old startup marker").unwrap();
    let window_smoke = proof_script.find("cargo run -- --window-smoke").unwrap();
    let runtime_tmux_ui_smoke = proof_script
        .find("cargo run -- --runtime-tmux-ui-smoke")
        .unwrap();
    let interactive_launch = proof_script
        .find("A default Gromaq window will open through plain cargo run.")
        .unwrap();

    assert!(build < interactive_launch);
    assert!(marker_check < interactive_launch);
    assert!(stale_check < interactive_launch);
    assert!(window_smoke < interactive_launch);
    assert!(stale_check < window_smoke);
    assert!(runtime_tmux_ui_smoke < interactive_launch);
    assert!(window_smoke < runtime_tmux_ui_smoke);
}

#[test]
fn native_tmux_default_snapshot_proof_exports_inspectable_artifact() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let proof_script =
        fs::read_to_string(root.join("scripts/prove-native-tmux-default-snapshot.sh")).unwrap();

    for marker in [
        "cargo run -- --window-tmux-manager-snapshot",
        "tmux new-session -d -s",
        "snapshot_session=",
        "snapshot-session:",
        "default startup content checked: true",
        "tmux status strip rendered: true",
        "tmux status pane command rendered: true",
        "tmux manager panel rendered: true",
        "tmux manager sessions:",
        "tmux manager windows:",
        "tmux manager panes:",
        "require_positive_tmux_count",
        "tmux manager sessions",
        "tmux manager windows",
        "tmux manager panes",
        "gromaq-native-tmux-default-snapshot.ppm",
        "gromaq-native-tmux-default-snapshot.png",
        "native tmux default snapshot proof: ok",
        "summary.txt",
    ] {
        assert!(
            proof_script.contains(marker),
            "native tmux default snapshot proof must include marker `{marker}`"
        );
    }
}
