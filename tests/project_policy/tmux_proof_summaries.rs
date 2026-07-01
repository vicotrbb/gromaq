use std::{fs, path::Path};

#[test]
fn configured_manual_tmux_proof_summary_surfaces_manager_reference_render_proof() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let proof_script =
        fs::read_to_string(root.join("scripts/prove-macos-native-tmux-manual.sh")).unwrap();
    let summary_marker = "tmux-manager-reference.stderr: ${manager_reference_stderr_path}";
    let summary_sections = proof_script.match_indices(summary_marker).count();
    assert_eq!(
        summary_sections, 2,
        "expected preflight and completed summaries"
    );
    for (offset, _) in proof_script.match_indices(summary_marker) {
        let summary = &proof_script[offset..];
        for marker in [
            "grep -F \"tmux status strip rendered: true\" \"${manager_reference_stdout_path}\"",
            "grep -F \"tmux status pane command rendered: true\" \"${manager_reference_stdout_path}\"",
            "grep -F \"tmux manager panel rendered: true\" \"${manager_reference_stdout_path}\"",
        ] {
            assert!(summary.contains(marker), "{marker}");
        }
    }
}

#[test]
fn configured_manual_tmux_proof_summary_surfaces_runtime_action_proof() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let proof_script =
        fs::read_to_string(root.join("scripts/prove-macos-native-tmux-manual.sh")).unwrap();
    let summary_marker = "tmux-runtime-tmux-ui-smoke.stderr: ${runtime_tmux_ui_smoke_stderr_path}";
    assert_eq!(proof_script.match_indices(summary_marker).count(), 2);
    for (offset, _) in proof_script.match_indices(summary_marker) {
        let summary = &proof_script[offset..];
        for marker in [
            "grep -F \"confirmation path checked: true\" \"${runtime_tmux_ui_smoke_stdout_path}\"",
            "grep -F \"safe action dispatched: true\" \"${runtime_tmux_ui_smoke_stdout_path}\"",
            "grep -F \"workspace duplicate prevented: true\" \"${runtime_tmux_ui_smoke_stdout_path}\"",
            "grep -F \"cleanup killed session: true\" \"${runtime_tmux_ui_smoke_stdout_path}\"",
        ] {
            assert!(summary.contains(marker), "{marker}");
        }
    }
}

#[test]
fn default_cargo_run_tmux_proof_summary_surfaces_runtime_action_proof() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let proof_script =
        fs::read_to_string(root.join("scripts/prove-macos-native-tmux-default-cargo-run.sh"))
            .unwrap();
    let summary_marker =
        "tmux-default-cargo-run-runtime-tmux-ui-smoke.stderr: ${runtime_tmux_ui_smoke_stderr_path}";
    assert_eq!(proof_script.match_indices(summary_marker).count(), 2);
    for (offset, _) in proof_script.match_indices(summary_marker) {
        let summary = &proof_script[offset..];
        for marker in [
            "grep -F \"confirmation path checked: true\" \"${runtime_tmux_ui_smoke_stdout_path}\"",
            "grep -F \"safe action dispatched: true\" \"${runtime_tmux_ui_smoke_stdout_path}\"",
            "grep -F \"cleanup killed session: true\" \"${runtime_tmux_ui_smoke_stdout_path}\"",
        ] {
            assert!(summary.contains(marker), "{marker}");
        }
    }
}
