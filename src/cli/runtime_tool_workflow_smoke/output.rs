//! Output formatting for external tool workflow smoke reports.

use super::{ToolWorkflowReport, ToolWorkflowResult};

pub(super) fn render_tool_workflow_report(report: &ToolWorkflowReport) -> String {
    let passed = report
        .results
        .iter()
        .filter(|result| matches!(result, ToolWorkflowResult::Passed { .. }))
        .count();
    let skipped = report
        .results
        .iter()
        .filter(|result| matches!(result, ToolWorkflowResult::Skipped { .. }))
        .count();
    let failed = report
        .results
        .iter()
        .filter(|result| matches!(result, ToolWorkflowResult::Failed { .. }))
        .count();
    let mut output = format!(
        "runtime tool workflow smoke: ok\ntools checked: {}\npassed: {passed}\nskipped: {skipped}\nfailed: {failed}\n",
        report.results.len()
    );
    for result in &report.results {
        output.push_str(&render_tool_workflow_result(result));
    }
    output
}

fn render_tool_workflow_result(result: &ToolWorkflowResult) -> String {
    match result {
        ToolWorkflowResult::Passed {
            name,
            expected,
            output_bytes,
        } => {
            format!("{name}: passed, expected {expected:?}, output bytes {output_bytes}\n")
        }
        ToolWorkflowResult::Skipped { name } => {
            format!("{name}: skipped, not found on PATH\n")
        }
        ToolWorkflowResult::Failed { name, reason } => {
            format!("{name}: failed, {reason}\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_renders_pass_skip_and_failure_counts() {
        let report = ToolWorkflowReport {
            results: vec![
                ToolWorkflowResult::Passed {
                    name: "ssh",
                    expected: "OpenSSH",
                    output_bytes: 42,
                },
                ToolWorkflowResult::Skipped { name: "kubectl" },
                ToolWorkflowResult::Failed {
                    name: "example",
                    reason: "missing marker".to_owned(),
                },
            ],
        };

        let output = render_tool_workflow_report(&report);

        assert!(output.contains("tools checked: 3"));
        assert!(output.contains("passed: 1"));
        assert!(output.contains("skipped: 1"));
        assert!(output.contains("failed: 1"));
        assert!(output.contains("ssh: passed, expected \"OpenSSH\", output bytes 42"));
        assert!(output.contains("kubectl: skipped, not found on PATH"));
        assert!(output.contains("example: failed, missing marker"));
    }
}
