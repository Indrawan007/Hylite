//! Idempotent Source focus through public Iced widget operations.
//!
//! TextEditor 0.13 has no public widget ID. A named, zero-padding Container
//! scopes the operation to the one Source editor instead of treating Tab's
//! focus_next() as a focus setter (it unfocuses an already-focused last widget).
use crate::commands::Message;
use iced::advanced::widget::{self, operation, Operation};
use iced::widget::container;
use iced::{Rectangle, Task};

pub fn id() -> container::Id {
    container::Id::new("hylite-source-focus")
}

pub fn task() -> Task<Message> {
    widget::operate(focus_operation())
}

fn focus_operation() -> impl Operation<Message> {
    struct FocusSource;

    impl Operation<Message> for FocusSource {
        fn focusable(&mut self, state: &mut dyn operation::Focusable, _id: Option<&widget::Id>) {
            state.focus();
        }

        fn container(
            &mut self,
            _id: Option<&widget::Id>,
            _bounds: Rectangle,
            children: &mut dyn FnMut(&mut dyn Operation<Message>),
        ) {
            children(self);
        }
    }

    // The scope contains exactly the Source TextEditor. Find/Go to Line inputs
    // and other anonymous focusable widgets are outside it and are not touched.
    operation::scope(id().into(), FocusSource)
}

#[cfg(test)]
mod tests {
    use super::*;
    use operation::Focusable;

    #[derive(Default)]
    struct FocusState(bool);

    impl Focusable for FocusState {
        fn is_focused(&self) -> bool {
            self.0
        }
        fn focus(&mut self) {
            self.0 = true;
        }
        fn unfocus(&mut self) {
            self.0 = false;
        }
    }

    fn traverse(
        operation: &mut dyn Operation<Message>,
        scope: widget::Id,
        source: &mut FocusState,
        anonymous: &mut FocusState,
        query: &mut FocusState,
    ) {
        operation.container(None, Rectangle::default(), &mut |operation| {
            operation.focusable(anonymous, None);
            operation.focusable(query, Some(&widget::Id::new("query")));
            operation.container(Some(&scope), Rectangle::default(), &mut |operation| {
                // Traverse a nested layout container as well, not just a direct
                // call to FocusSource.focusable in isolation.
                operation.container(None, Rectangle::default(), &mut |operation| {
                    operation.focusable(source, None);
                });
            });
        });
    }

    #[test]
    fn focusing_source_is_idempotent_for_focused_and_unfocused_editors() {
        for initially_focused in [false, true] {
            let mut source = FocusState(initially_focused);
            let mut anonymous = FocusState::default();
            let mut query = FocusState::default();
            for _ in 0..3 {
                traverse(
                    &mut focus_operation(),
                    id().into(),
                    &mut source,
                    &mut anonymous,
                    &mut query,
                );
                assert!(source.is_focused());
            }
        }
    }

    #[test]
    fn source_scope_does_not_focus_or_unfocus_other_widgets() {
        for outside_focused in [false, true] {
            let mut source = FocusState::default();
            let mut anonymous = FocusState(outside_focused);
            let mut query = FocusState(outside_focused);
            traverse(
                &mut focus_operation(),
                id().into(),
                &mut source,
                &mut anonymous,
                &mut query,
            );
            assert!(source.is_focused());
            assert_eq!(anonymous.is_focused(), outside_focused);
            assert_eq!(query.is_focused(), outside_focused);
        }
    }

    #[test]
    fn missing_source_scope_is_a_no_op() {
        let mut source = FocusState::default();
        let mut anonymous = FocusState(true);
        let mut query = FocusState(true);
        traverse(
            &mut focus_operation(),
            widget::Id::new("preview-not-source"),
            &mut source,
            &mut anonymous,
            &mut query,
        );
        assert!(!source.is_focused());
        assert!(anonymous.is_focused());
        assert!(query.is_focused());
    }
}
