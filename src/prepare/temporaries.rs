//! Each temporary's name spelled, of the words its body suggests of it
//! (`js::Name::suggested`, ADR 0569): tidy makes a temporary a name not
//! spelled yet that carries them, `routes` of `tree.routes`, its call's
//! name, what an element is given it as, and its role's word, `value`;
//! here, before the names lowering made are spelled and the program's
//! reclaimed, each takes the first of those words JS's scopes let it be,
//! shadowing nothing around it, an import of its module among them, and
//! shadowed by nothing inside (`Tree::of_in`), else its role's word given
//! apart, `value$1`. Not a word a variable spelled already in its scope
//! reclaims to, `next` beside `next$7`: the program's names, and those
//! lowering spelled, outrank a made one, as reclaiming prefers them (ADR
//! 0357). One speller of the module's names, preparation's, not a word
//! table of tidy's own (#1681 change 3).

use super::reclaim::names_mut;
use crate::collections::{IndexMap, IndexSet};
use crate::js::{self, Name, scopes};

/// `module`'s temporaries spelled, each item's in the order the program
/// first reads them, as tidy named them: one made in a `try` before a
/// `let` the function declares first and sets later (ADR 0705); one never
/// read last.
pub(super) fn spelled(module: &mut js::Module) {
    let top = scopes::Tree::module_top(module);
    // What an import given apart at the top reclaims to, `createContainer`
    // of `createContainer$1`, once no local takes it (`reclaim::imports`).
    let reclaimed: IndexSet<String> = (top.iter())
        .map(|word| Name::module(word).root().to_string())
        .filter(|root| !top.contains(root))
        .collect();
    for item in &mut module.items {
        let mut suggested: Vec<Name> = Vec::new();
        names_mut(item, true, &mut |name| {
            // By its word, as it's spelled below: a made name's word is its own
            // in its item, each copy's of a body applied again too (`rebound`).
            if name.is_suggested() && name.is_unspelled() && !suggested.iter().any(|s| s.is(name.word())) {
                suggested.push(name.clone());
            }
        });
        if suggested.is_empty() {
            continue;
        }
        // None of an item whose scopes declare a name twice, which has no
        // tree to ask: each apart from every word it has, below.
        let mut tree = scopes::Tree::of_in(item, &top);
        let mut first: IndexMap<Name, usize> = IndexMap::default();
        item.visit_names(&mut |name| {
            let at = first.len();
            first.entry(name.clone()).or_insert(at);
        });
        suggested.sort_by_key(|name| first.get(name).copied().unwrap_or(usize::MAX));
        // Each word of the item's and its module's top: of a temporary its
        // tree doesn't see declared, an object's method's, which no scope
        // asks, or of an item with no tree, apart from all of them, as
        // `reclaim::spelled` spells one of an item with no tree.
        let mut words: IndexSet<String> = top.iter().chain(&reclaimed).cloned().collect();
        names_mut(item, true, &mut |name| _ = words.insert(name.word().to_string()));
        let mut spelled: IndexMap<String, String> = IndexMap::default();
        for name in suggested {
            let from = name.word();
            let base = name.made_base().expect("suggested is made");
            let to = (name.preferred().iter().cloned())
                .chain(std::iter::once(base.to_string()))
                .chain((1..).map(|k| crate::names::given_apart(base, k)))
                .filter(|to| !crate::names::kept_word(to))
                .take(64 + name.preferred().len())
                .find(|to| {
                    tree.as_ref().is_some_and(|tree| {
                        !tree.shadows(from, to)
                            && tree.renames(from, to)
                            && !tree.reclaims_to(from, &|name| name.apart_of().is_some_and(|(of, _)| of == to))
                    }) && !reclaimed.contains(to)
                });
            let to = match (to, &mut tree) {
                (Some(to), Some(tree)) => {
                    tree.rename(from, &to);
                    to
                }
                _ => {
                    let base = (name.preferred().iter().map(String::as_str))
                        .find(|word| !words.contains(*word))
                        .unwrap_or(base);
                    crate::names::fresh_apart(&mut words, base)
                }
            };
            words.insert(to.clone());
            spelled.insert(name.into_word(), to);
        }
        let mut respelled: Vec<Name> = Vec::new();
        names_mut(item, true, &mut |name| {
            if let Some(to) = spelled.get(name.word()) {
                name.respell(to.clone());
                if !respelled.contains(name) {
                    respelled.push(name.clone());
                }
            }
        });
        // Each flag of one, `value$0$live` of the `value#4` it was made of,
        // made of its word now, `tryLock$0$live`, as tidy named a flag once
        // its temporary was named (`Name::suffixed_again`, #841).
        let mut flags: IndexMap<Name, Name> = IndexMap::default();
        names_mut(item, true, &mut |name| {
            if name.suffix_tail().is_some()
                && !flags.contains_key(name)
                && let Some(source) = respelled.iter().find(|source| source.is_origin_of(name))
            {
                let to = name.suffixed_again(source, &words);
                words.insert(to.word().to_string());
                flags.insert(name.clone(), to);
            }
        });
        names_mut(item, true, &mut |name| {
            if let Some(to) = flags.get(name) {
                *name = to.clone();
            }
        });
    }
}
