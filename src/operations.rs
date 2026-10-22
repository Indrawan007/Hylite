//! Serialization and stale-result validation for asynchronous file operations.

use crate::document::Snapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationToken {
    id: u64,
    pub snapshot: Snapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Open,
    Save,
    Export,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Operation {
    pub token: OperationToken,
    pub kind: OperationKind,
}

#[derive(Default)]
pub struct Operations {
    next_id: u64,
    active: Option<Operation>,
}

impl Operations {
    pub fn active(&self) -> Option<Operation> {
        self.active
    }

    pub fn start(&mut self, kind: OperationKind, snapshot: Snapshot) -> Option<OperationToken> {
        if self.active.is_some() {
            return None;
        }
        self.next_id += 1;
        let token = OperationToken {
            id: self.next_id,
            snapshot,
        };
        self.active = Some(Operation { token, kind });
        Some(token)
    }

    pub fn finish(
        &mut self,
        token: OperationToken,
        kind: OperationKind,
        current: Snapshot,
    ) -> bool {
        if self.active != Some(Operation { token, kind }) {
            return false;
        }
        self.active = None;
        token.snapshot.document_id == current.document_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operations_are_serialized_and_tokens_are_not_reused() {
        let mut operations = Operations::default();
        let snapshot = Snapshot::default();
        let first = operations.start(OperationKind::Save, snapshot).unwrap();
        assert!(operations.start(OperationKind::Open, snapshot).is_none());
        assert!(operations.finish(first, OperationKind::Save, snapshot));
        let next = operations.start(OperationKind::Open, snapshot).unwrap();
        assert_ne!(first, next);
        assert!(!operations.finish(first, OperationKind::Save, snapshot));
        assert_eq!(operations.active().unwrap().token, next);
    }

    #[test]
    fn wrong_kind_does_not_consume_the_active_operation() {
        let mut operations = Operations::default();
        let snapshot = Snapshot::default();
        let token = operations.start(OperationKind::Save, snapshot).unwrap();
        assert!(!operations.finish(token, OperationKind::Export, snapshot));
        assert!(operations.active().is_some());
    }

    #[test]
    fn replacement_document_refuses_results_but_new_revisions_are_allowed() {
        let mut operations = Operations::default();
        let snapshot = Snapshot::default();
        let token = operations.start(OperationKind::Save, snapshot).unwrap();
        assert!(!operations.finish(
            token,
            OperationKind::Save,
            Snapshot {
                document_id: 1,
                ..snapshot
            }
        ));
        assert!(operations.active().is_none());
        let token = operations.start(OperationKind::Save, snapshot).unwrap();
        assert!(operations.finish(
            token,
            OperationKind::Save,
            Snapshot {
                revision: 1,
                ..snapshot
            }
        ));
    }
}
