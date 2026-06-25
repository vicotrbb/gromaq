use std::collections::VecDeque;
use std::hint::black_box;
use std::path::Path;

use criterion::Criterion;
use gromaq::Style;
use gromaq::app::{NativePtyResize, NativePtySessionIo, NativePtySpawner};
use gromaq::pty::{PtyConfig, PtyError};
use gromaq::renderer::GlyphKey;

pub(crate) const LARGE_OUTPUT: &str = "\
\x1b[31;1merror\x1b[0m line one\r\n\
normal log line with unicode 界 and attributes\r\n\
\x1b[32mok\x1b[0m line three\r\n\
";

pub(crate) const ASCII_RENDER_OUTPUT: &str = "\
error status 0123456789 ABC xyz\r\n\
normal log line with attributes\r\n\
prompt $ cargo test --all\r\n\
";

pub(crate) const BENCH_MONOSPACE_FONT_CANDIDATES: &[&str] = &[
    "/System/Library/Fonts/SFNSMono.ttf",
    "/System/Library/Fonts/Menlo.ttc",
    "/System/Library/Fonts/Supplemental/Courier New.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/dejavu-sans-fonts/DejaVuSansMono.ttf",
    "/usr/share/fonts/truetype/liberation2/LiberationMono-Regular.ttf",
    "/usr/share/fonts/liberation/LiberationMono-Regular.ttf",
    "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf",
];

pub(crate) const BOUNDED_STATE_BATCHES: usize = 4;
pub(crate) const BOUNDED_STATE_LINES_PER_BATCH: usize = 512;
pub(crate) const BOUNDED_STATE_SCROLLBACK_LINES: usize = 128;
pub(crate) const CONTINUOUS_OUTPUT_BATCHES: usize = 32;
pub(crate) const CONTINUOUS_OUTPUT_LINES_PER_BATCH: usize = 8;
pub(crate) const CONTINUOUS_OUTPUT_SCROLLBACK_LINES: usize = 64;
pub(crate) const SCROLLBACK_NAVIGATION_LINES: usize = 4_096;
pub(crate) const SCROLLBACK_NAVIGATION_STEPS: usize = 512;
pub(crate) const ALTERNATE_SCREEN_STAGES: usize = 3;
pub(crate) const FRAME_SCHEDULER_TIMELINE_STEPS: usize = 512;
pub(crate) const REAL_PTY_BENCH_LINES: usize = 512;
pub(crate) const UNICODE_CLUSTER_BENCH_LINES: usize = 512;
pub(crate) const GLYPH_ATLAS_HOT_KEYS: usize = 64;
pub(crate) const GLYPH_ATLAS_CHURN_KEYS: usize = 512;
pub(crate) const GLYPH_ATLAS_LOOKUPS: usize = 4_096;
pub(crate) const RUNTIME_PROTOCOL_INPUT_PAYLOAD: &[u8] =
    b"\x1b[?1004h\x1b[?1000h\x1b[?1006h\x1b[3;5H\x1b[6n\x1b[5n\x1b[c\x1b[>c";

#[derive(Debug)]
pub(crate) struct BenchPtySession {
    pub(crate) output: VecDeque<Vec<u8>>,
    pub(crate) echo_input: bool,
}

#[derive(Debug)]
pub(crate) struct BenchPayloadPtySession {
    pub(crate) output: VecDeque<Vec<u8>>,
}

impl NativePtySessionIo for BenchPayloadPtySession {
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError> {
        Ok(self.output.pop_front().unwrap_or_default())
    }

    fn write_input(&mut self, _bytes: &[u8]) -> Result<(), PtyError> {
        Ok(())
    }

    fn resize(&mut self, _size: NativePtyResize) -> Result<(), PtyError> {
        Ok(())
    }
}

impl NativePtySessionIo for BenchPtySession {
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError> {
        Ok(self.output.pop_front().unwrap_or_default())
    }

    fn write_input(&mut self, bytes: &[u8]) -> Result<(), PtyError> {
        if self.echo_input {
            self.output.push_back(bytes.to_vec());
        }
        Ok(())
    }

    fn resize(&mut self, _size: NativePtyResize) -> Result<(), PtyError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BenchPtySpawner {
    pub(crate) chunks: usize,
    pub(crate) echo_input: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct BenchPayloadPtySpawner {
    pub(crate) payloads: Vec<Vec<u8>>,
}

impl NativePtySpawner for BenchPayloadPtySpawner {
    type Session = BenchPayloadPtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(BenchPayloadPtySession {
            output: VecDeque::from(self.payloads.clone()),
        })
    }
}

impl NativePtySpawner for BenchPtySpawner {
    type Session = BenchPtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        let mut output = VecDeque::with_capacity(self.chunks);
        for _ in 0..self.chunks {
            output.push_back(LARGE_OUTPUT.as_bytes().to_vec());
        }
        Ok(BenchPtySession {
            output,
            echo_input: self.echo_input,
        })
    }
}

pub(crate) fn skip_benchmark(c: &mut Criterion, name: &'static str, reason: &str) {
    eprintln!("skipping {name}: {reason}");
    c.bench_function(name, |b| b.iter(|| black_box(())));
}

pub(crate) fn bench_monospace_font_bytes() -> Result<Vec<u8>, String> {
    let Some(path) = BENCH_MONOSPACE_FONT_CANDIDATES
        .iter()
        .map(Path::new)
        .find(|path| path.exists())
    else {
        return Err("no local monospace font candidate found".to_owned());
    };
    std::fs::read(path).map_err(|error| {
        format!(
            "failed to read monospace font candidate {}: {error}",
            path.display()
        )
    })
}

pub(crate) fn real_pty_large_output_script() -> String {
    format!(
        "i=0; while [ \"$i\" -lt {REAL_PTY_BENCH_LINES} ]; do printf 'gromaq-real-pty-%04d\\n' \"$i\"; i=$((i + 1)); done"
    )
}

pub(crate) fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

pub(crate) fn unicode_cluster_output_payload() -> Vec<u8> {
    let clusters = [
        "👩\u{200d}❤\u{fe0f}\u{200d}💋\u{200d}👨",
        "🧑🏾\u{200d}⚕\u{fe0f}",
        "🏳️\u{200d}🌈",
        "🇺🇸",
        "🏴\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}",
        "A\u{0301}\u{0302}",
    ];
    let mut payload = Vec::with_capacity(UNICODE_CLUSTER_BENCH_LINES * 96);
    for line in 0..UNICODE_CLUSTER_BENCH_LINES {
        let cluster = clusters[line % clusters.len()];
        payload.extend_from_slice(
            format!(
                "\x1b[3{}mcluster-{line:04} {cluster} {cluster}\x1b[0m\r\n",
                line % 8
            )
            .as_bytes(),
        );
    }
    payload
}

pub(crate) fn glyph_atlas_bench_keys(count: usize) -> Vec<GlyphKey> {
    (0..count)
        .map(|index| {
            let style = if index % 3 == 0 {
                Style {
                    bold: true,
                    ..Style::default()
                }
            } else if index % 5 == 0 {
                Style {
                    italic: true,
                    ..Style::default()
                }
            } else {
                Style::default()
            };
            let text = format!("g{index:03}");
            let first = text.chars().next().unwrap();
            GlyphKey::for_text(&text, first, style, 14 + (index % 4) as u16)
        })
        .collect()
}

pub(crate) fn bounded_state_payloads() -> Vec<Vec<u8>> {
    (0..BOUNDED_STATE_BATCHES)
        .map(|batch| {
            let start = batch * BOUNDED_STATE_LINES_PER_BATCH;
            let end = start + BOUNDED_STATE_LINES_PER_BATCH;
            let mut payload = Vec::new();
            for line in start..end {
                payload.extend_from_slice(format!("gromaq-bounded-line-{line:04}\n").as_bytes());
            }
            payload
        })
        .collect()
}

pub(crate) fn continuous_output_payloads() -> Vec<Vec<u8>> {
    (0..CONTINUOUS_OUTPUT_BATCHES)
        .map(|batch| {
            let start = batch * CONTINUOUS_OUTPUT_LINES_PER_BATCH;
            let end = start + CONTINUOUS_OUTPUT_LINES_PER_BATCH;
            let mut payload = Vec::new();
            for line in start..end {
                payload.extend_from_slice(format!("gromaq-continuous-line-{line:03}\n").as_bytes());
            }
            payload
        })
        .collect()
}

pub(crate) fn scrollback_navigation_payload() -> Vec<u8> {
    let mut payload = Vec::new();
    for line in 0..SCROLLBACK_NAVIGATION_LINES {
        payload.extend_from_slice(format!("gromaq-scrollback-nav-line-{line:04}\n").as_bytes());
    }
    payload
}

pub(crate) fn alternate_screen_payloads() -> Vec<Vec<u8>> {
    vec![
        b"primary\n".to_vec(),
        b"\x1b[?1049halt-view\n".to_vec(),
        b"\x1b[?1049lrestored\n".to_vec(),
    ]
}
