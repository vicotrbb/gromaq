use std::hint::black_box;
use std::time::{Duration, Instant};

use criterion::Criterion;
use gromaq::renderer::{FrameScheduler, GlyphAtlas, GlyphAtlasConfig};

use crate::support::{
    FRAME_SCHEDULER_TIMELINE_STEPS, GLYPH_ATLAS_CHURN_KEYS, GLYPH_ATLAS_HOT_KEYS,
    GLYPH_ATLAS_LOOKUPS, glyph_atlas_bench_keys,
};

pub(crate) fn glyph_atlas_cache_churn(c: &mut Criterion) {
    let hot_keys = glyph_atlas_bench_keys(GLYPH_ATLAS_HOT_KEYS);
    let churn_keys = glyph_atlas_bench_keys(GLYPH_ATLAS_CHURN_KEYS);

    c.bench_function("glyph_atlas_cache_churn", |b| {
        b.iter(|| {
            let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(128).unwrap());
            for key in &hot_keys {
                atlas.lookup_or_insert(key.clone()).unwrap();
            }

            for index in 0..GLYPH_ATLAS_LOOKUPS {
                let hot = &hot_keys[index % hot_keys.len()];
                black_box(atlas.lookup_or_insert(black_box(hot.clone())).unwrap());

                if index % 4 == 0 {
                    let churn = &churn_keys[index % churn_keys.len()];
                    black_box(atlas.lookup_or_insert(black_box(churn.clone())).unwrap());
                }
            }

            let metrics = atlas.metrics();
            black_box(metrics.hits);
            black_box(metrics.misses);
            black_box(metrics.evictions);
            black_box(metrics.entries);
        });
    });
}

pub(crate) fn frame_scheduler_144hz_timeline(c: &mut Criterion) {
    c.bench_function("frame_scheduler_144hz_timeline", |b| {
        b.iter(|| {
            let mut scheduler = FrameScheduler::new(144).unwrap();
            let target_interval = scheduler.target_interval();
            let start = Instant::now();
            let first = scheduler.decide(start, true);
            scheduler.record_presented(start);
            let mut now = start;
            let mut render_decisions = usize::from(first.should_render);
            let mut paced_decisions = 0_usize;

            for step in 1..FRAME_SCHEDULER_TIMELINE_STEPS {
                let paced = scheduler.decide(now + Duration::from_millis(2), true);
                if paced.wait_for.is_some() {
                    paced_decisions += 1;
                }

                now = if step % 32 == 0 {
                    now + target_interval + target_interval + target_interval
                } else {
                    now + target_interval
                };
                let decision = scheduler.decide(now, true);
                if decision.should_render {
                    render_decisions += 1;
                    scheduler.record_presented(now);
                }
            }

            let idle = scheduler.decide(now + Duration::from_nanos(1), false);
            let metrics = scheduler.metrics();
            black_box(render_decisions);
            black_box(paced_decisions);
            black_box(idle);
            black_box(metrics.frames_presented);
            black_box(metrics.dropped_frames);
        });
    });
}
