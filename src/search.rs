//! Literal, case-sensitive, non-overlapping search over a document snapshot.

use crate::document::Snapshot;
use crate::text_edit::{valid_range, TextChange};
use std::ops::Range;

#[derive(Default)]
pub struct Search {
    pub open: bool,
    pub replace_open: bool,
    pub query: String,
    pub replacement: String,
    matches: Vec<Range<usize>>,
    current: Option<usize>,
    snapshot: Option<Snapshot>,
    searched_query: String,
}

impl Search {
    pub fn refresh(&mut self, text: &str, snapshot: Snapshot) {
        if self.snapshot == Some(snapshot) && self.searched_query == self.query {
            return;
        }
        self.matches = if self.query.is_empty() {
            Vec::new()
        } else {
            text.match_indices(self.query.as_str())
                .map(|(start, matched)| start..start + matched.len())
                .collect()
        };
        self.current = None;
        self.snapshot = Some(snapshot);
        self.searched_query.clone_from(&self.query);
    }

    pub fn reset_document(&mut self) {
        self.matches.clear();
        self.current = None;
        self.snapshot = None;
    }

    /// A different navigation command must not leave an old replacement target.
    pub fn clear_current(&mut self) {
        self.current = None;
    }

    pub fn count(&self) -> usize {
        self.matches.len()
    }
    pub fn index(&self) -> Option<usize> {
        self.current
    }
    pub fn current(&self) -> Option<Range<usize>> {
        self.matches.get(self.current?).cloned()
    }

    pub fn navigate(&mut self, backwards: bool) -> Option<Range<usize>> {
        let count = self.matches.len();
        if count == 0 {
            self.current = None;
            return None;
        }
        self.current = Some(match (self.current, backwards) {
            (None, false) => 0,
            (None, true) => count - 1,
            (Some(index), false) => (index + 1) % count,
            (Some(index), true) => (index + count - 1) % count,
        });
        self.current()
    }

    pub fn next_from(&mut self, offset: usize) -> Option<Range<usize>> {
        self.current = self
            .matches
            .iter()
            .position(|range| range.start >= offset)
            .or_else(|| (!self.matches.is_empty()).then_some(0));
        self.current()
    }

    /// Caller must refresh against the current document immediately before use.
    pub fn replacement(&self, text: &str) -> Option<TextChange> {
        let range = self.current()?;
        if !valid_range(text, &range) || text[range.clone()] != self.query {
            return None;
        }
        Some(TextChange {
            range,
            replacement: self.replacement.clone(),
            caret: self.replacement.len(),
        })
    }

    /// Matches are taken once from the original text; replacement text is never
    /// recursively searched. The whole replacement is a single undo transaction.
    pub fn replacement_all(&self, text: &str) -> Option<TextChange> {
        if self.matches.is_empty() {
            return None;
        }
        let mut replacement = String::new();
        let mut previous = 0;
        for range in &self.matches {
            if !valid_range(text, range)
                || range.start < previous
                || text[range.clone()] != self.query
            {
                return None;
            }
            replacement.push_str(&text[previous..range.start]);
            replacement.push_str(&self.replacement);
            previous = range.end;
        }
        replacement.push_str(&text[previous..]);
        Some(TextChange {
            range: 0..text.len(),
            caret: replacement.len(),
            replacement,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn search(text: &str, query: &str) -> Search {
        let mut search = Search {
            query: query.into(),
            ..Search::default()
        };
        search.refresh(text, Snapshot::default());
        search
    }

    #[test]
    fn unicode_matches_are_byte_ranges_and_navigation_wraps() {
        let mut search = search("é✨ é✨", "é✨");
        assert_eq!(search.count(), 2);
        assert_eq!(search.navigate(false), Some(0..5));
        assert_eq!(search.index(), Some(0));
        assert_eq!(search.navigate(false), Some(6..11));
        assert_eq!(search.navigate(false), Some(0..5));
        assert_eq!(search.navigate(true), Some(6..11));
    }

    #[test]
    fn empty_missing_and_overlapping_queries_are_well_defined() {
        assert_eq!(search("aaa", "").count(), 0);
        assert_eq!(search("aaa", "z").navigate(false), None);
        assert_eq!(search("aaa", "aa").count(), 1);
        assert_eq!(search("Hello hello", "hello").count(), 1);
    }

    #[test]
    fn replacement_all_is_not_recursive_and_accepts_empty_replacement() {
        let mut search = search("a a", "a");
        search.replacement = "aa".into();
        assert_eq!(search.replacement_all("a a").unwrap().replacement, "aa aa");
        search.replacement.clear();
        assert_eq!(search.replacement_all("a a").unwrap().replacement, " ");
    }

    #[test]
    fn changed_document_or_query_invalidates_the_active_match() {
        let mut search = search("aa", "a");
        search.navigate(false);
        search.refresh("aa", Snapshot::default());
        assert_eq!(search.current(), Some(0..1));
        search.refresh(
            "ba",
            Snapshot {
                revision: 1,
                ..Snapshot::default()
            },
        );
        assert!(search.current().is_none());
        search.query = "b".into();
        search.refresh(
            "ba",
            Snapshot {
                revision: 1,
                ..Snapshot::default()
            },
        );
        assert_eq!(search.navigate(false), Some(0..1));
        search.reset_document();
        assert_eq!(search.count(), 0);
        assert!(search.current().is_none());
    }

    #[test]
    fn replacement_requires_an_active_match_and_checks_the_source_slice() {
        let mut search = search("é a", "é");
        assert!(search.replacement("é a").is_none());
        search.navigate(false);
        search.replacement = "✨".into();
        assert_eq!(search.replacement("é a").unwrap().range, 0..2);
        assert!(search.replacement("xx a").is_none());
        assert!(search.replacement_all("xx a").is_none());
    }

    #[test]
    fn next_from_skips_the_replacement_and_wraps_at_eof() {
        let mut search = search("a a a", "a");
        assert_eq!(search.next_from(2), Some(2..3));
        assert_eq!(search.next_from(9), Some(0..1));
    }

    #[test]
    fn panel_state_is_independent_of_query_and_document_matches() {
        let mut state = Search::default();
        assert!(!state.open);
        assert!(!state.replace_open);
        state.open = true;
        state.replace_open = true;
        state.reset_document();
        assert!(state.open && state.replace_open);
    }
    #[test]
    fn clearing_active_match_keeps_cached_results_but_disables_single_replace() {
        let mut search = search("a a", "a");
        assert!(search.navigate(false).is_some());
        search.clear_current();
        assert_eq!(search.count(), 2);
        assert!(search.current().is_none());
        assert!(search.replacement("a a").is_none());
        // A refresh of the same snapshot must not resurrect the old target.
        search.refresh("a a", Snapshot::default());
        assert!(search.current().is_none());
        assert_eq!(search.navigate(false), Some(0..1));
    }
}
