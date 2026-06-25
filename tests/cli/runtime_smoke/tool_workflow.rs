use std::cell::RefCell;

use crate::{MockBackend, run_with_backend};

#[test]
fn runtime_tool_workflow_smoke_cli_reports_external_tool_pass_or_skip() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-tool-workflow-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime tool workflow smoke: ok"));
    assert!(exit.stdout.contains("tools checked: 2"));
    assert!(exit.stdout.contains("ssh:"));
    assert!(exit.stdout.contains("kubectl:"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}
