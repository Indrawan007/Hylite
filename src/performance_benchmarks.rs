//! Opt-in, headless Stage 10b measurements. This module is compiled only for tests.

use crate::document::Document;
use iced::widget::text_editor::{Action, Edit};
use iced::Theme;
use std::fs;
use std::hint::black_box;
use std::time::{Duration, Instant};

const DATASETS: [(&str, usize); 3] = [("100KB", 100_000), ("1MB", 1_000_000), ("5MB", 5_000_000)];
const SAMPLE_COUNT: usize = 3;
const WALL_BUDGET_SECONDS: u64 = 180;

#[test]
#[ignore = "explicit opt-in Stage 10b benchmark; run with --ignored"]
fn stage10b_baseline() {
    let suite_started = Instant::now();
    let directory = tempfile::tempdir().expect("create temporary benchmark directory");

    for (label, target_bytes) in DATASETS {
        let source = synthetic_markdown(target_bytes);
        assert_eq!(source.len(), target_bytes, "fixture size for {label}");
        let path = directory.path().join(format!("stage10b-{label}.md"));
        fs::write(&path, &source).expect("write synthetic benchmark document");

        let open = measure(&suite_started, || {
            let started = Instant::now();
            let file = fs::read_to_string(&path).expect("read synthetic benchmark document");
            let document = Document::new(&file);
            let elapsed = started.elapsed();
            let _ = black_box((file, document));
            elapsed
        });

        let typing = measure(&suite_started, || {
            let mut document = Document::new(&source);
            let started = Instant::now();
            document.apply(Action::Edit(Edit::Insert('x')));
            let elapsed = started.elapsed();
            let _ = black_box(document);
            elapsed
        });

        let parsing = measure(&suite_started, || {
            let document = Document::new(&source);
            let started = Instant::now();
            let block_count = document.preview_blocks().len();
            let elapsed = started.elapsed();
            let _ = black_box((block_count, document));
            elapsed
        });

        let rendering = measure(&suite_started, || {
            let document = Document::new(&source);
            let blocks = document.preview_blocks();
            let theme = Theme::Light;
            let started = Instant::now();
            let view = crate::document::render_preview(blocks, &theme, document.snapshot(), None);
            let elapsed = started.elapsed();
            let _ = black_box(view);
            let _ = black_box(document);
            elapsed
        });

        println!(
            "STAGE10B dataset={label} bytes={target_bytes} samples={SAMPLE_COUNT} warmups=1 open_ms[min/median/max]={} typing_ms[min/median/max]={} parse_ms[min/median/max]={} render_tree_ms[min/median/max]={}",
            describe(&open),
            describe(&typing),
            describe(&parsing),
            describe(&rendering),
        );
    }

    ensure_within_budget(&suite_started);
}

fn measure(started: &Instant, mut operation: impl FnMut() -> Duration) -> Vec<Duration> {
    let _ = black_box(operation());
    ensure_within_budget(started);

    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        samples.push(operation());
        ensure_within_budget(started);
    }
    samples
}

fn ensure_within_budget(started: &Instant) {
    assert!(
        started.elapsed() < Duration::from_secs(WALL_BUDGET_SECONDS),
        "Stage 10b benchmark exceeded its {WALL_BUDGET_SECONDS}s wall-clock budget"
    );
}

fn describe(samples: &[Duration]) -> String {
    let mut ordered = samples.to_vec();
    ordered.sort_unstable();
    let median = ordered[ordered.len() / 2];
    let maximum = *ordered.last().expect("at least one measured sample");
    format!(
        "{:.3}/{:.3}/{:.3}",
        milliseconds(ordered[0]),
        milliseconds(median),
        milliseconds(maximum),
    )
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn synthetic_markdown(target_bytes: usize) -> String {
    let mut paragraph = String::from(
        "Synthetic paragraph with **bold**, *emphasis*, `inline code`, and a [link](https://example.invalid), followed by ordinary prose: ",
    );
    paragraph.push_str(&"ordinary benchmark prose without markup; ".repeat(48));
    paragraph.push_str("\n\n");

    let mut source = String::with_capacity(target_bytes);
    let mut paragraphs_until_section = 0;
    while source.len() < target_bytes {
        if paragraphs_until_section == 0 {
            source.push_str("## Synthetic section\n\n");
            paragraphs_until_section = 16;
        }
        source.push_str(&paragraph);
        paragraphs_until_section -= 1;
    }

    assert!(source.is_ascii(), "fixture truncation requires ASCII input");
    source.truncate(target_bytes);
    source
}
