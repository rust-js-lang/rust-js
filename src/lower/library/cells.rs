//! What a library's contract says of its structs' fields and its
//! `RefCell`s (ADRs 0863, 0906): each field's facts, the quiet ones a
//! consumer may lend (#1032), and each library's cells.

use super::{Foreign, key};
use crate::collections::IndexMap;
use crate::library::{FieldFact, RefCellFacts};
use rustc_span::def_id::DefId;

impl Foreign<'_, '_> {
    /// Whether `field`, a library's struct's, is one its contract says
    /// `fact` of (ADR 0906).
    pub(in crate::lower) fn field_has(&self, field: DefId, fact: FieldFact) -> bool {
        !field.is_local()
            && self
                .library(field.krate)
                .and_then(|library| library.fields.get(&key(self.tcx, field)))
                .is_some_and(|facts| facts.contains(&fact))
    }

    /// Each field of a library's struct its contract says is `fact`, by its
    /// key, and the library that says so (ADR 0906).
    pub(in crate::lower) fn fields_saying(&self, fact: FieldFact) -> IndexMap<String, String> {
        (self.dependencies.libraries.iter())
            .flat_map(|(name, library)| {
                (library.fields.iter())
                    .filter(move |(_, facts)| facts.contains(&fact))
                    .map(move |(key, _)| (key.clone(), name.clone()))
            })
            .collect()
    }

    /// What each library does with its `RefCell`s (ADR 0863), by its name.
    pub(in crate::lower) fn ref_cell_facts(&self) -> Vec<(String, RefCellFacts)> {
        let mut facts: Vec<_> = (self.dependencies.libraries.iter())
            .filter(|(_, library)| !library.ref_cells.is_empty())
            .map(|(name, library)| (name.clone(), library.ref_cells.clone()))
            .collect();
        facts.sort_by(|a, b| a.0.cmp(&b.0));
        facts
    }
}
