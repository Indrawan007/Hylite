//! Deterministic property/differential hardening for multiline drafts.
//!
//! No external crates: a fixed LCG generates sources/values and every case is
//! checked against an independent reference transformation of the raw Source.
use super::*;
use crate::preview_paragraph::Target;

/// Deterministic xorshift so failures reproduce from the printed seed.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
    fn yes(&mut self) -> bool {
        self.next() & 1 == 0
    }
}
const FIRST: &str = "café🙂  \nfirst   ";
const SECOND: &str = "second é🙂\ntail &amp;  ";
const THIRD: &str = "third\nlast line\t";
fn corpus() -> Vec<String> {
    vec![
        FIRST.into(),
        SECOND.into(),
        "# heading\n".into(),
        "single line\n".into(),
        "> nested\n> paragraph\n".into(),
        "```text\nliteral\nparagraph\n```\n".into(),
        "- item\n- other\n".into(),
        "| a | b |\n| - | - |\n| 1 | 2 |\n".into(),
        "\n".into(),
        "\n\n".into(),
        "terminal\n\t".into(),
    ]
}
/// Sources always end with a terminal LF; document invariants are unchanged.
fn random_source(rng: &mut Rng, corpus: &[String]) -> String {
    let mut parts = Vec::new();
    for _ in 0..(1 + rng.below(4)) {
        parts.push(rng.pick(corpus).clone());
        if rng.yes() {
            parts.push("\n".into());
        }
    }
    if rng.yes() {
        parts.push(format!("{FIRST}\n"));
    }
    if rng.yes() {
        parts.push(format!("{SECOND}\n"));
    }
    if rng.yes() {
        parts.push(THIRD.to_owned());
    }
    let mut text = parts.concat();
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
}
fn roots(doc: &Document) -> Vec<Target> {
    doc.preview_blocks()
        .iter()
        .filter_map(|b| {
            Some(Target {
                snapshot: doc.snapshot(),
                block: b.source.clone(),
                payload: b.multiline_paragraph_payload.clone()?,
            })
        })
        .collect()
}
/// Independent reference: the only legitimate post-edit Source.
fn reference(source: &str, anchor: &Target, value: &str) -> String {
    let mut out = source.to_owned();
    out.replace_range(anchor.payload.clone(), value);
    out
}
fn mutated_value(rng: &mut Rng, original: &str) -> String {
    let mut lines: Vec<String> = original.split('\n').map(str::to_owned).collect();
    match rng.below(8) {
        0 => format!("{original}"),
        1 => format!("{original}\nedited line"),
        2 => {
            if lines.len() > 1 {
                lines.pop();
            }
            lines.join("\n")
        }
        3 => format!("é🙂 {}", original.replace('\n', "\n  tail ")),
        4 => "x\ny".into(),
        5 => vec!["line"; 1 + rng.below(6)].join("\n"),
        6 => format!("{}\n{}", original, "z".repeat(rng.below(40))),
        _ => format!("{}  ", original.replace("e", "é")),
    }
}
fn invalid_value(rng: &mut Rng) -> String {
    match rng.below(9) {
        0 => String::new(),
        1 => "one".into(),
        2 => " one\ntwo".into(),
        3 => "one\n\ntwo".into(),
        4 => "one\ntwo\t".into(),
        5 => "one\n# heading".into(),
        6 => "```\ncode".into(),
        7 => vec!["x"; 129].join("\n"),
        _ => format!(
            "{}\nx",
            "x".repeat(crate::preview_paragraph_multiline::MAX_BYTES)
        ),
    }
}
fn state(doc: &Document) -> (Snapshot, (u64, u64), usize, String, bool, bool, bool) {
    (
        doc.snapshot(),
        doc.render_key(),
        doc.caret_offset(),
        doc.buffer_text(),
        doc.is_dirty(),
        doc.can_undo(),
        doc.can_redo(),
    )
}
#[test]
fn property_commit_matches_reference_source_for_both_directions() {
    let corpus = corpus();
    let mut rng = Rng(0x5eed_1234_abcd_0001);
    let mut commits = 0;
    let mut refusals = 0;
    for case in 0..400 {
        let text = random_source(&mut rng, &corpus);
        let mut doc = Document::new(&text);
        let all = roots(&doc);
        if all.is_empty() {
            continue;
        }
        let index = rng.below(all.len());
        let anchor = all[index].clone();
        let original = doc.multiline_paragraph_payload(&anchor).unwrap();
        let value = mutated_value(&mut rng, &original);
        let direction = if rng.yes() {
            crate::document::ParagraphDirection::Next
        } else {
            crate::document::ParagraphDirection::Previous
        };
        let neighbour = doc.adjacent_multiline_paragraph(&anchor, direction);
        let plan = if original == value {
            if direction == crate::document::ParagraphDirection::Next {
                doc.prepare_multiline_paragraph_next(&anchor, &value)
            } else {
                doc.prepare_multiline_paragraph_previous(&anchor, &value)
            }
        } else if direction == crate::document::ParagraphDirection::Next {
            doc.prepare_multiline_paragraph_next(&anchor, &value)
        } else {
            doc.prepare_multiline_paragraph_previous(&anchor, &value)
        };
        let Some(plan) = plan else {
            // Missing neighbour or refused value: state must be byte-identical.
            assert_eq!(state(&doc).3, text, "case {case}");
            assert!(!doc.can_undo(), "case {case}");
            refusals += 1;
            continue;
        };
        // The controller always builds the next native draft first.
        let moved = plan.move_target();
        let moved_value = plan.move_value().to_owned();
        assert!(crate::ui::multiline_paragraph::Draft::new(moved.clone(), &moved_value).is_some());
        let expected = reference(&text, &anchor, &value);
        let (next, changed) = doc.commit_multiline_paragraph_move(plan).unwrap();
        assert_eq!(doc.buffer_text(), expected, "case {case}");
        assert_eq!(changed, original != value, "case {case}");
        assert_eq!(
            doc.caret_offset(),
            anchor.payload.start + value.len(),
            "case {case}"
        );
        let blocks = crate::document::preview_model::parse(&expected);
        let complete = |b: &Block| {
            b.kind == crate::document::preview_model::Kind::Paragraph
                && b.source == (anchor.block.start..anchor.block.start + value.len() + 1)
                && b.multiline_paragraph_payload.as_ref()
                    == Some(&(anchor.block.start..anchor.block.start + value.len()))
        };
        assert!(
            blocks.iter().any(complete),
            "case {case}: replacement must stay one complete multiline root"
        );
        if let Some(neighbour) = neighbour {
            assert_eq!(next.snapshot, doc.snapshot(), "case {case}");
            assert_eq!(
                doc.multiline_paragraph_payload(&next).as_deref(),
                Some(next_payload(&expected, &neighbour, &anchor, &value).as_str()),
                "case {case}: moved neighbour keeps its exact raw payload"
            );
        }
        if changed {
            assert!(doc.undo(), "case {case}");
            assert_eq!(doc.buffer_text(), text, "case {case}: one atomic edge");
            assert!(doc.redo(), "case {case}");
            assert_eq!(doc.buffer_text(), expected, "case {case}");
            commits += 1;
        } else {
            assert!(!doc.can_undo(), "case {case}: unchanged move adds no edge");
            assert_eq!(
                doc.snapshot(),
                all[index].snapshot,
                "case {case}: identity kept"
            );
        }
        let _ = doc.preview.take();
        assert!(roots(&Document::new(&doc.buffer_text())).len() >= 1);
    }
    assert!(commits > 20, "expected many commits, got {commits}");
    assert!(refusals > 0, "expected some refusals");
}
/// Neighbour payload text: unchanged for both directions because only the anchor
/// bytes change and a Previous neighbour starts before the anchor payload.
fn next_payload(source: &str, neighbour: &Target, _anchor: &Target, _value: &str) -> String {
    source[neighbour.payload.clone()].to_owned()
}
#[test]
fn property_refusal_and_stale_targets_never_mutate_source_or_history() {
    let corpus = corpus();
    let mut rng = Rng(0x5eed_1234_abcd_0002);
    let mut checked = 0;
    for case in 0..250 {
        let doc = Document::new(&random_source(&mut rng, &corpus));
        let all = roots(&doc);
        if all.is_empty() {
            continue;
        }
        let anchor = all[rng.below(all.len())].clone();
        let bad = invalid_value(&mut rng);
        let before = state(&doc);
        let ptr = doc.preview.get().map(|b| b.as_ptr());
        assert!(
            doc.prepare_multiline_paragraph_next(&anchor, &bad)
                .is_none(),
            "case {case}"
        );
        assert!(
            doc.prepare_multiline_paragraph_previous(&anchor, &bad)
                .is_none(),
            "case {case}"
        );
        assert_eq!(state(&doc), before, "case {case}");
        assert_eq!(doc.preview.get().map(|b| b.as_ptr()), ptr, "case {case}");
        checked += 1;
    }
    assert!(checked > 100, "insufficient coverage: {checked}");
}
#[test]
fn property_stale_and_cross_document_plans_are_rejected_every_time() {
    let corpus = corpus();
    let mut rng = Rng(0x5eed_1234_abcd_0003);
    let mut rejected = 0;
    for case in 0..200 {
        let text = random_source(&mut rng, &corpus);
        let mut doc = Document::new(&text);
        let all = roots(&doc);
        if all.is_empty() {
            continue;
        }
        let anchor = all[rng.below(all.len())].clone();
        let value = mutated_value(&mut rng, &doc.multiline_paragraph_payload(&anchor).unwrap());
        let Some(plan) = doc.prepare_multiline_paragraph_next(&anchor, &value) else {
            continue;
        };
        let _ = doc.preview.take();
        assert!(
            doc.commit_multiline_paragraph_move(plan).is_none(),
            "case {case}"
        );
        assert_eq!(doc.buffer_text(), text, "case {case}");
        assert!(
            !doc.can_undo(),
            "case {case}: refusal must not record history"
        );
        rejected += 1;
    }
    assert!(rejected > 50, "insufficient coverage: {rejected}");
}
#[test]
fn property_local_draft_history_stays_bounded_under_random_edits() {
    let mut rng = Rng(0x5eed_1234_abcd_0004);
    let value = "first line\nsecond line\nthird line";
    let mut draft = crate::ui::multiline_paragraph::Draft::new(
        Target {
            snapshot: Default::default(),
            block: 0..value.len() + 1,
            payload: 0..value.len(),
        },
        value,
    )
    .unwrap();
    let alphabet = ['a', 'é', '🙂', ' ', '1', 'Z'];
    for _ in 0..300 {
        match rng.below(6) {
            0 => draft.edit(ied(Edit::Insert(*rng.pick(&alphabet)))),
            1 => draft.edit(ied(Edit::Enter)),
            2 => draft.edit(ied(Edit::Backspace)),
            3 => draft.undo_redo(false),
            4 => draft.undo_redo(true),
            _ => {
                draft.edit(text_editor::Action::Move(Motion::DocumentEnd));
            }
        }
        // Local session invariants hold after every operation.
        assert!(draft.value().len() <= crate::preview_paragraph_multiline::MAX_BYTES);
        assert!(draft.value().len() <= crate::multiline_draft_history::MAX_TEXT_BYTES);
    }
}
fn ied(edit: Edit) -> text_editor::Action {
    text_editor::Action::Edit(edit)
}
#[test]
fn property_seeded_runs_are_deterministic() {
    fn run(seed: u64) -> Vec<(String, usize, bool)> {
        let corpus = corpus();
        let mut rng = Rng(seed);
        let mut out = Vec::new();
        for _ in 0..40 {
            let mut doc = Document::new(&random_source(&mut rng, &corpus));
            let all = roots(&doc);
            if all.is_empty() {
                continue;
            }
            let anchor = all[rng.below(all.len())].clone();
            let original = doc.multiline_paragraph_payload(&anchor).unwrap();
            let value = mutated_value(&mut rng, &original);
            let prepared = doc.prepare_multiline_paragraph_previous(&anchor, &value);
            let changed = prepared
                .and_then(|plan| doc.commit_multiline_paragraph_move(plan))
                .map(|(_, changed)| changed);
            out.push((doc.buffer_text(), all.len(), changed.unwrap_or(false)));
        }
        out
    }
    assert_eq!(run(7), run(7));
    assert_ne!(run(7), run(8));
}
