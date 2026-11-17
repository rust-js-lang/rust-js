//! What a loop's counter tidy's loops made is spelled, and what holds a
//! range's end (`Name::counter`, ADR 1042, #2032): preparation's, which
//! spells every other name too (ADR 1150), not a table of the words lowering
//! saw. A counter is the first of `i`, `j`, `k`, `i$1`, .. and an end the
//! first of `end`, `end$1`, .., that shadows no variable around it and that
//! JS's scopes let it be: loops side by side each count with `i`, one inside
//! a loop of `i` with `j`.

use super::reclaim::{mentions, names_mut};
use crate::collections::IndexMap;
use crate::js::scopes;
use crate::js::{self, Name};

/// Each item's counters spelled, outer before inner as its tree has them,
/// once the program's names are reclaimed (`reclaim::module`), so a
/// counter is spelled apart from a variable around it as it's spelled at
/// the end: of a loop inside one whose `i$1` reclaiming makes `i`, `j`.
pub(super) fn spelled(module: &mut js::Module) {
    let mut taken = mentions(module);
    for item in &mut module.items {
        let mut counters: Vec<Name> = Vec::new();
        names_mut(item, true, &mut |name| {
            if name.is_counter() && !counters.iter().any(|c| c.is(name.word())) {
                counters.push(name.clone());
            }
        });
        if counters.is_empty() {
            continue;
        }
        let mut tree = scopes::Tree::of(item);
        let mut spelled: IndexMap<String, String> = IndexMap::default();
        for name in counters {
            let in_scope = tree.as_mut().and_then(|tree| {
                let to = counter_word(&name, tree)?;
                tree.rename(name.word(), &to);
                Some(to)
            });
            let to = in_scope.unwrap_or_else(|| counter_word_apart(&name, &taken));
            taken.insert(to.clone());
            spelled.insert(name.into_word(), to);
        }
        names_mut(item, true, &mut |name| {
            if let Some(to) = spelled.get(name.word()) {
                name.respell(to);
            }
        });
    }
}

/// `name`'s word, a counter's or a held end's: the first of its candidates
/// that shadows nothing around it (`Tree::shadows`) and that `tree` lets
/// `name` be, each read staying its own (`Tree::renames`). `None` where
/// none of the first 64 is.
fn counter_word(name: &Name, tree: &scopes::Tree) -> Option<String> {
    let base = name.made_base().expect("a counter is made");
    candidates(base)
        .filter(|to| !crate::names::kept_word(to))
        .take(64)
        .find(|to| !tree.shadows(name.word(), to) && tree.renames(name.word(), to))
}

/// `name`'s word where its item has no tree, a scope of it declaring a
/// word twice: the first of its candidates no name of its module has
/// (`taken`), which no scope can mistake for another's.
fn counter_word_apart(name: &Name, taken: &crate::collections::IndexSet<String>) -> String {
    let base = name.made_base().expect("a counter is made");
    candidates(base)
        .find(|to| !crate::names::kept_word(to) && !taken.contains(to))
        .expect("a word nothing has")
}

/// What a counter of `base` may be spelled, in order: of `i`, `i`, `j`, `k`,
/// then `i$1`, `i$2`, ..; of another, `end`, `end$1`, .. (ADR 1042).
fn candidates(base: &str) -> Box<dyn Iterator<Item = String> + '_> {
    let apart = (1..).map(move |k| crate::names::given_apart(base, k));
    match base {
        "i" => Box::new(["i", "j", "k"].into_iter().map(String::from).chain(apart)),
        _ => Box::new(std::iter::once(base.to_string()).chain(apart)),
    }
}
