use std::hint::black_box;

use criterion::{BatchSize, Criterion};
use gromaq::{DirtyRegion, DirtyTracker, Terminal, TerminalConfig};

use crate::support::{
    LARGE_OUTPUT, SCROLLBACK_NAVIGATION_LINES, SCROLLBACK_NAVIGATION_STEPS,
    scrollback_navigation_payload, unicode_cluster_output_payload,
};

pub(crate) fn parser_large_output(c: &mut Criterion) {
    c.bench_function("parser_large_output", |b| {
        b.iter(|| {
            let mut terminal = Terminal::new(TerminalConfig::new(120, 36).unwrap());
            for _ in 0..256 {
                terminal.write_str(black_box(LARGE_OUTPUT)).unwrap();
            }
            black_box(terminal.dump_perf_metrics());
        });
    });
}

pub(crate) fn unicode_emoji_cluster_output(c: &mut Criterion) {
    let payload = unicode_cluster_output_payload();

    c.bench_function("unicode_emoji_cluster_output", |b| {
        b.iter(|| {
            let mut terminal = Terminal::new(
                TerminalConfig::new(120, 36)
                    .unwrap()
                    .with_scrollback_limit(4_096)
                    .unwrap(),
            );
            terminal.write_bytes(black_box(&payload)).unwrap();
            black_box(terminal.dump_grid());
            black_box(terminal.dump_scrollback());
            black_box(terminal.dump_perf_metrics());
        });
    });
}

pub(crate) fn scrollback_large_output(c: &mut Criterion) {
    let mut output = String::with_capacity(20_000);
    for line in 0..2_000 {
        use std::fmt::Write as _;
        write!(&mut output, "line {line:04}\r\n").expect("writing to a String is infallible");
    }

    c.bench_function("scrollback_large_output", |b| {
        b.iter(|| {
            let mut terminal = Terminal::new(
                TerminalConfig::new(80, 12)
                    .unwrap()
                    .with_scrollback_limit(20_000)
                    .unwrap(),
            );
            terminal.write_bytes(black_box(output.as_bytes())).unwrap();
            black_box(terminal.dump_scrollback());
        });
    });
}

pub(crate) fn scrollback_view_navigation(c: &mut Criterion) {
    let payload = scrollback_navigation_payload();

    c.bench_function("scrollback_view_navigation", |b| {
        b.iter_batched(
            || {
                let mut terminal = Terminal::new(
                    TerminalConfig::new(80, 24)
                        .unwrap()
                        .with_scrollback_limit(SCROLLBACK_NAVIGATION_LINES)
                        .unwrap(),
                );
                terminal.write_bytes(&payload).unwrap();
                terminal.take_dirty_regions();
                terminal
            },
            |mut terminal| {
                let mut moved_rows = 0_usize;

                for _ in 0..SCROLLBACK_NAVIGATION_STEPS {
                    moved_rows += usize::from(terminal.scroll_display_up(1));
                    let grid = terminal.dump_grid();
                    black_box(grid.line_text(0));
                    black_box(terminal.take_dirty_regions());
                }

                for _ in 0..SCROLLBACK_NAVIGATION_STEPS {
                    moved_rows += usize::from(terminal.scroll_display_down(1));
                    let grid = terminal.dump_grid();
                    black_box(grid.line_text(0));
                    black_box(terminal.take_dirty_regions());
                }

                black_box(moved_rows);
                black_box(terminal.dump_perf_metrics());
            },
            BatchSize::SmallInput,
        );
    });
}

pub(crate) fn dirty_region_coalescing(c: &mut Criterion) {
    c.bench_function("dirty_region_coalescing", |b| {
        b.iter(|| {
            let mut dirty = DirtyTracker::default();
            for row in 0..36 {
                dirty.mark_span(row, 0, 80);
                dirty.mark_cell(row, row % 120);
                dirty.mark_region(DirtyRegion {
                    row,
                    col: 40,
                    rows: 1,
                    cols: 40,
                });
            }
            black_box(dirty.contains_region(DirtyRegion {
                row: 0,
                col: 0,
                rows: 36,
                cols: 80,
            }));
            black_box(dirty.take());
        });
    });
}
