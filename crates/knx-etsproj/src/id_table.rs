//! Pass-1 id allocations of one entity kind, keyed by ETS id.
//!
//! A malformed project can repeat an ETS `Id` (validation reports it as
//! `DuplicateId`). Every element still gets its **own** internal id, so two
//! group addresses stay two group addresses instead of collapsing onto one id
//! and losing one of them on save. A cross-reference that names a repeated
//! ETS id cannot say which element it means; it resolves to
//! [`Reference::Ambiguous`] and the mapper reports it instead of guessing.

use std::cell::RefCell;
use std::collections::BTreeMap;

/// What a cross-reference to an ETS id resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reference<Id> {
    Unique(Id),
    Missing,
    Ambiguous,
}

pub(crate) struct IdTable<Id> {
    by_ets_id: BTreeMap<String, Vec<Id>>,
    /// How many occurrences of each ETS id pass 2 has already claimed.
    claimed: RefCell<BTreeMap<String, usize>>,
}

impl<Id> Default for IdTable<Id> {
    fn default() -> Self {
        Self {
            by_ets_id: BTreeMap::new(),
            claimed: RefCell::new(BTreeMap::new()),
        }
    }
}

impl<Id: Copy> IdTable<Id> {
    /// Records the id pass 1 allocated for one element, in document order.
    pub(crate) fn insert(&mut self, ets_id: &str, id: Id) {
        self.by_ets_id
            .entry(ets_id.to_string())
            .or_default()
            .push(id);
    }

    /// The element's own id. Occurrences of a repeated ETS id claim the
    /// allocations one after another, so each element keeps a distinct id.
    ///
    /// # Panics
    /// If pass 2 visits more elements with this ETS id than pass 1
    /// allocated — a mapper bug, not a property of the input.
    pub(crate) fn own(&self, ets_id: &str) -> Id {
        let ids = self
            .by_ets_id
            .get(ets_id)
            .expect("every element is allocated in pass 1");
        let mut claimed = self.claimed.borrow_mut();
        let next = claimed.entry(ets_id.to_string()).or_default();
        let id = *ids
            .get(*next)
            .expect("pass 2 visits each element pass 1 allocated exactly once");
        *next += 1;
        id
    }

    pub(crate) fn reference(&self, ets_id: &str) -> Reference<Id> {
        match self.by_ets_id.get(ets_id).map(Vec::as_slice) {
            None | Some([]) => Reference::Missing,
            Some([id]) => Reference::Unique(*id),
            Some(_) => Reference::Ambiguous,
        }
    }

    /// Every id allocated for `ets_id`, in document order (empty if none).
    pub(crate) fn all(&self, ets_id: &str) -> &[Id] {
        self.by_ets_id.get(ets_id).map_or(&[], Vec::as_slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_ets_ids_keep_distinct_own_ids_and_ambiguous_references() {
        let mut table = IdTable::default();
        table.insert("GA-1", 1u32);
        table.insert("GA-2", 2);
        table.insert("GA-1", 3);
        assert_eq!(table.own("GA-1"), 1);
        assert_eq!(table.own("GA-2"), 2);
        assert_eq!(table.own("GA-1"), 3);
        assert_eq!(table.reference("GA-1"), Reference::Ambiguous);
        assert_eq!(table.reference("GA-2"), Reference::Unique(2));
        assert_eq!(table.reference("GA-9"), Reference::Missing);
        assert_eq!(table.all("GA-1"), [1, 3]);
        assert_eq!(table.all("GA-9"), [] as [u32; 0]);
    }
}
