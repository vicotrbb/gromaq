use std::{fs, path::Path};

#[test]
fn current_host_compatibility_proof_records_replayable_host_metadata() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let compatibility_proof_script =
        fs::read_to_string(root.join("scripts/prove-current-host-compatibility.sh")).unwrap();

    assert!(compatibility_proof_script.contains("target/compatibility-proof"));
    assert!(compatibility_proof_script.contains("cargo test --test pty -- --nocapture"));
    assert!(compatibility_proof_script.contains("cargo run -- --runtime-tool-workflow-smoke"));
    assert!(compatibility_proof_script.contains("command -v"));
    assert!(compatibility_proof_script.contains("GROMAQ_REQUIRED_COMPAT_TOOLS"));
    assert!(compatibility_proof_script.contains("required compatibility tool missing"));
    assert!(compatibility_proof_script.contains("summary.txt"));
    assert!(compatibility_proof_script.contains("host_uname="));
    assert!(compatibility_proof_script.contains("host_os="));
    assert!(compatibility_proof_script.contains("host_arch="));
    assert!(compatibility_proof_script.contains("rustc_version="));
    assert!(compatibility_proof_script.contains("cargo_version="));
    assert!(compatibility_proof_script.contains("git_commit="));
    assert!(compatibility_proof_script.contains("git_dirty="));
    assert!(compatibility_proof_script.contains("tools_present="));
    assert!(compatibility_proof_script.contains("tools_missing="));
    assert!(compatibility_proof_script.contains("pty_tests_passed="));
    assert!(compatibility_proof_script.contains("runtime_tool_workflow_checked="));
    assert!(compatibility_proof_script.contains("runtime_tool_workflow_passed="));
    assert!(compatibility_proof_script.contains("runtime_tool_workflow_skipped="));
    assert!(compatibility_proof_script.contains("runtime_tool_workflow_failed="));
    assert!(compatibility_proof_script.contains("runtime_tool_workflow_passed_names="));
    assert!(compatibility_proof_script.contains("runtime_tool_workflow_skipped_names="));
    assert!(compatibility_proof_script.contains("Current-host compatibility proof: ok"));
}
