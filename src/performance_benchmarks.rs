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

/// Coarse per-operation ceilings: five times the worst median recorded in
/// `docs/performance.md`, in milliseconds for the 100 KB, 1 MB and 5 MB
/// datasets. They exist to fail a run that loses an order of magnitude or
/// stalls, not to state a latency target; they are not portable between
/// machines, and the harness stays opt-in.
const CEILINGS_MS: [(&str, [f64; 3]); 6] = [
    ("open", [0.4, 12.0, 175.0]),
    ("typing", [0.3, 4.0, 33.0]),
    ("parse", [26.0, 255.0, 1290.0]),
    ("render_tree", [1.05, 11.5, 56.0]),
    ("incremental_preview", [8.0, 80.0, 400.0]),
    ("fallback_preview", [35.0, 350.0, 1790.0]),
];

#[test]
#[ignore = "explicit opt-in Stage 10b benchmark; run with --ignored"]
fn stage10b_baseline() {
    let suite_started = Instant::now();
    let directory = tempfile::tempdir().expect("create temporary benchmark directory");

    for (dataset, (label, target_bytes)) in DATASETS.into_iter().enumerate() {
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

        let plain_source = synthetic_plain_markdown(target_bytes);
        assert_eq!(
            plain_source.len(),
            target_bytes,
            "plain fixture size for {label}"
        );
        let incremental = measure(&suite_started, || {
            let mut document = Document::new(&plain_source);
            document.preview_blocks(); // Warm the old full projection outside the timer.
            document.apply(Action::Edit(Edit::Insert('x')));
            let started = Instant::now();
            let block_count = document.preview_blocks().len();
            let elapsed = started.elapsed();
            let _ = black_box((block_count, document));
            elapsed
        });
        let fallback = measure(&suite_started, || {
            let mut document = Document::new(&plain_source);
            document.preview_blocks(); // Warm the old full projection outside the timer.
            document.apply(Action::Edit(Edit::Enter));
            let started = Instant::now();
            let block_count = document.preview_blocks().len();
            let elapsed = started.elapsed();
            let _ = black_box((block_count, document));
            elapsed
        });

        for (metric, samples) in [
            ("open", &open),
            ("typing", &typing),
            ("parse", &parsing),
            ("render_tree", &rendering),
            ("incremental_preview", &incremental),
            ("fallback_preview", &fallback),
        ] {
            assert_within_ceiling(metric, dataset, label, samples);
        }
        // Structural guarantees: the incremental cache must stay cheaper than
        // falling back to the full parser, and building the element tree must
        // stay cheaper than parsing the document. Ratios survive slow runners.
        assert!(
            median_of(&incremental) < median_of(&fallback),
            "Stage 10b {label}: incremental preview must stay cheaper than the full-parser fallback"
        );
        assert!(
            median_of(&rendering) < median_of(&parsing),
            "Stage 10b {label}: building the Preview tree must stay cheaper than parsing"
        );

        println!(
            "STAGE10B dataset={label} bytes={target_bytes} samples={SAMPLE_COUNT} warmups=1 open_ms[min/median/max]={} typing_ms[min/median/max]={} parse_ms[min/median/max]={} render_tree_ms[min/median/max]={} incremental_preview_ms[min/median/max]={} fallback_preview_ms[min/median/max]={}",
            describe(&open),
            describe(&typing),
            describe(&parsing),
            describe(&rendering),
            describe(&incremental),
            describe(&fallback),
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

fn median_of(samples: &[Duration]) -> Duration {
    let mut ordered = samples.to_vec();
    ordered.sort_unstable();
    ordered[ordered.len() / 2]
}

/// Fails the run when a metric's median crosses its coarse ceiling.
fn assert_within_ceiling(metric: &str, dataset: usize, label: &str, samples: &[Duration]) {
    let ceiling = CEILINGS_MS
        .iter()
        .find_map(|(name, ceilings)| (*name == metric).then_some(ceilings[dataset]))
        .expect("ceiling table covers every metric");
    let median = milliseconds(median_of(samples));
    assert!(
        median <= ceiling,
        "Stage 10b {label} {metric}: median {median:.3} ms exceeds its {ceiling:.3} ms ceiling"
    );
}

fn describe(samples: &[Duration]) -> String {
    let mut ordered = samples.to_vec();
    ordered.sort_unstable();
    let median = median_of(samples);
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

fn synthetic_plain_markdown(target_bytes: usize) -> String {
    const PARAGRAPH: &str = "Incremental benchmark prose remains plain and independent across paragraphs. The cache may reparse one eligible line while retaining exact Source offsets.\n\n";
    let mut source = String::with_capacity(target_bytes);
    while source.len() + PARAGRAPH.len() <= target_bytes {
        source.push_str(PARAGRAPH);
    }
    source.push_str(&"x".repeat(target_bytes - source.len()));
    assert!(source.is_ascii(), "plain benchmark fixture is ASCII");
    source
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
