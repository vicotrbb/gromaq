use std::hint::black_box;
use std::path::Path;
use std::time::{Duration, Instant};

use criterion::Criterion;
use gromaq::pty::{PtyConfig, PtySession, ShellCommand};

use crate::support::{
    REAL_PTY_BENCH_LINES, contains_bytes, real_pty_large_output_script, skip_benchmark,
};

pub(crate) fn real_pty_shell_large_output_burst(c: &mut Criterion) {
    if !Path::new("/bin/sh").exists() {
        skip_benchmark(c, "real_pty_shell_large_output_burst", "/bin/sh not found");
        return;
    }

    c.bench_function("real_pty_shell_large_output_burst", |b| {
        b.iter(|| {
            let mut session = PtySession::spawn(PtyConfig {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
                shell: ShellCommand {
                    program: "/bin/sh".into(),
                    args: vec!["-lc".into(), real_pty_large_output_script().into()],
                    cwd: None,
                },
            })
            .unwrap();
            session.start_output_reader().unwrap();

            let marker = format!("gromaq-real-pty-{:04}", REAL_PTY_BENCH_LINES - 1);
            let mut output = Vec::new();
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                output.extend(session.drain_available_output().unwrap());
                if contains_bytes(&output, marker.as_bytes()) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(1));
            }

            assert!(
                contains_bytes(&output, marker.as_bytes()),
                "real PTY benchmark did not observe {marker}"
            );
            assert!(
                session
                    .wait_timeout(Duration::from_secs(5))
                    .unwrap()
                    .is_some()
            );
            black_box(output.len());
        });
    });
}

pub(crate) fn real_pty_shell_input_echo_roundtrip(c: &mut Criterion) {
    if !Path::new("/bin/sh").exists() {
        skip_benchmark(
            c,
            "real_pty_shell_input_echo_roundtrip",
            "/bin/sh not found",
        );
        return;
    }

    c.bench_function("real_pty_shell_input_echo_roundtrip", |b| {
        b.iter(|| {
            let mut session = PtySession::spawn(PtyConfig {
                rows: 8,
                cols: 40,
                pixel_width: 0,
                pixel_height: 0,
                shell: ShellCommand {
                    program: "/bin/sh".into(),
                    args: Vec::new(),
                    cwd: None,
                },
            })
            .unwrap();
            session.start_output_reader().unwrap();
            session
                .write_all(b"printf 'gromaq-real-pty-input\\n'\nexit\n")
                .unwrap();

            let marker = b"gromaq-real-pty-input";
            let mut output = Vec::new();
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                output.extend(session.drain_available_output().unwrap());
                if contains_bytes(&output, marker) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(1));
            }

            assert!(
                contains_bytes(&output, marker),
                "real PTY benchmark did not observe input echo output"
            );
            assert!(
                session
                    .wait_timeout(Duration::from_secs(5))
                    .unwrap()
                    .is_some()
            );
            black_box(output.len());
        });
    });
}
