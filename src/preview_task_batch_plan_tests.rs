use super::*;

fn batch(source: &str, checked: bool) -> BatchTarget {
    let tasks = source
        .match_indices('[')
        .map(|(start, _)| Target {
            snapshot: Snapshot::default(),
            item: 0..source.len(),
            marker: start..start + 3,
            checked: source.as_bytes()[start + 1] != b' ',
        })
        .collect();
    BatchTarget {
        snapshot: Snapshot::default(),
        tasks,
        checked,
    }
}
fn changed(source: &str, target: &BatchTarget) -> String {
    let change = plan_batch(source, Snapshot::default(), target).unwrap();
    assert_eq!(change.range.len(), change.replacement.len());
    assert_eq!(change.caret, change.replacement.len());
    let mut result = source.to_owned();
    result.replace_range(change.range, &change.replacement);
    result
}
#[test]
fn checking_changes_only_unchecked_bytes_preserving_uppercase_checked_markers_and_raw_body() {
    let source = "- [ ] café🙂 **raw**\n- [X] keep\n> - [ ] last\n\t";
    assert_eq!(
        changed(source, &batch(source, true)),
        "- [x] café🙂 **raw**\n- [X] keep\n> - [x] last\n\t"
    );
}
#[test]
fn clearing_changes_both_checked_cases_but_keeps_existing_spaces_and_whitespace() {
    let source = "- [x] one  \n- [ ] keep\n- [X] café🙂\n\t";
    assert_eq!(
        changed(source, &batch(source, false)),
        "- [ ] one  \n- [ ] keep\n- [ ] café🙂\n\t"
    );
}
#[test]
fn no_op_and_empty_batches_are_refused_without_inventing_a_change() {
    for (source, checked) in [
        ("- [X] upper\n- [x] lower\n", true),
        ("- [ ] empty\n", false),
        ("plain", true),
    ] {
        assert!(plan_batch(source, Snapshot::default(), &batch(source, checked)).is_none());
    }
}
#[test]
fn snapshots_outer_and_each_member_must_match_before_any_change_is_returned() {
    let source = "- [ ] first\n- [ ] last\n";
    for field in 0..6 {
        let mut target = batch(source, true);
        match field {
            0 => target.snapshot.document_id += 1,
            1 => target.snapshot.revision += 1,
            2 => target.snapshot.state_id += 1,
            3 => target.tasks[1].snapshot.document_id += 1,
            4 => target.tasks[1].snapshot.revision += 1,
            _ => target.tasks[1].snapshot.state_id += 1,
        }
        assert!(plan_batch(source, Snapshot::default(), &target).is_none());
    }
}
#[test]
fn duplicate_reversed_invalid_and_later_state_mismatch_targets_are_all_or_nothing() {
    let source = "- [ ] first\n- [ ] last\n";
    for field in 0..6 {
        let mut target = batch(source, true);
        match field {
            0 => target.tasks.push(target.tasks[0].clone()),
            1 => target.tasks.reverse(),
            2 => target.tasks[1].marker.end = usize::MAX,
            3 => target.tasks[1].checked = true,
            4 => target.tasks[1].item.end = 1,
            _ => target.tasks[1].marker = 1..4,
        }
        assert!(plan_batch(source, Snapshot::default(), &target).is_none());
    }
}
#[test]
fn cap_rejects_the_entire_oversized_batch_not_a_partial_prefix() {
    let source = "- [ ] task\n".repeat(MAX_BATCH_TASKS);
    assert!(plan_batch(&source, Snapshot::default(), &batch(&source, true)).is_some());
    let source = source + "- [ ] extra\n";
    assert!(plan_batch(&source, Snapshot::default(), &batch(&source, true)).is_none());
}
#[test]
fn planner_does_not_claim_complete_model_membership_or_task_provenance() {
    let source = "```md\n- [ ] literal\n```\n\n- [ ] genuine\n";
    let mut target = batch(source, true);
    assert!(plan_batch(source, Snapshot::default(), &target).is_some());
    target.tasks.pop();
    // A lexical plan cannot authenticate omission or a fenced marker. Document must.
    assert!(plan_batch(source, Snapshot::default(), &target).is_some());
}
