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
    // The module's own items' words, a split function's entry among them.
    let declared: IndexSet<String> = (module.items.iter())
        .filter_map(|item| match item {
            js::Item::Function(function) => Some(&function.name),
            _ => item.declared_name(),
        })
        .map(|name| name.word().to_string())
        .collect();
    for item in &mut module.items {
        apart_where_declared_again(item);
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
        // A split function's body the item reads, `checkedDiv$`, reads its
        // entry's word too, `checkedDiv`, the module's own: no local takes it,
        // as none would where the item called the entry (#2403, ADR 1193).
        let entries_read: IndexSet<String> = (words.iter())
            .filter_map(|word| word.strip_suffix('$'))
            .filter(|entry| declared.contains(*entry))
            .map(str::to_string)
            .collect();
        let mut spelled: IndexMap<String, String> = IndexMap::default();
        for name in suggested {
            let from = name.word();
            let base = name.made_base().expect("suggested is made");
            let to = (name.preferred().iter().cloned())
                .chain(std::iter::once(base.to_string()))
                .chain((1..).map(|k| crate::names::given_apart(base, k)))
                .filter(|to| !crate::names::kept_word(to) && !entries_read.contains(to))
                .take(64 + name.preferred().len())
                .find(|to| {
                    tree.as_ref().is_some_and(|tree| {
                        !tree.shadows(from, to)
                            && tree.renames(from, to)
                            && !tree.reclaims_to(from, &|name| {
                                name.apart_of().is_some_and(|(of, _)| of == to)
                                    // And one of the program's not spelled
                                    // yet, a scope's function, which it's
                                    // spelled (#2048), or lowering's, a
                                    // flattened struct's field's `parts`,
                                    // which outranks a made one as the
                                    // program's does (#1715).
                                    || (name.made_base() == Some(to)
                                        && (name.is_the_programs() || !name.is_suggested()))
                            })
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
                flags.insert(name.clone(), name.suffixed_again(source));
            }
        });
        names_mut(item, true, &mut |name| {
            if let Some(to) = flags.get(name) {
                *name = to.clone();
            }
        });
    }
}

/// Each name lowering made, not spelled yet, declared again in a block that
/// declares it already, a word of its own from there on: two variables,
/// as JS can't declare one twice in a block. Two copies of one body
/// applied in it, a `Drop`'s at each value it drops (`js::applied`), each
/// keep the made names of the body, which a copy in a scope of its own, a
/// cleanup's arrow, is spelled with alike; beside each other, each is its
/// own, spelled apart, `value$1` beside `value`, of the words suggested of
/// it still (#1681 change 3, step 2's regression).
fn apart_where_declared_again(item: &mut js::Item) {
    let mut names: Vec<Name> = Vec::new();
    names_mut(item, true, &mut |name| names.push(name.clone()));
    let mut apart = |list: &mut Vec<js::Stmt>| {
        let mut declared: Vec<Name> = Vec::new();
        for at in 0..list.len() {
            let here: Vec<Name> = js::declared_here(&list[at]).into_iter().cloned().collect();
            for name in here {
                if !name.is_unspelled() || !declared.iter().any(|d| d.is(name.word())) {
                    declared.push(name);
                    continue;
                }
                let to = name.spelled_later_apart(&names);
                names.push(to.clone());
                js::replaced(&mut list[at..], &name, &to);
                declared.push(to);
            }
        }
    };
    let mut each = |stmts: &mut Vec<js::Stmt>| {
        js::statement_lists(stmts, &mut apart);
        js::each_expr_mut(stmts, &mut |e| match &mut e.kind {
            js::ExprKind::Arrow(_, body) | js::ExprKind::AsyncArrow(_, body) => js::statement_lists(body, &mut apart),
            js::ExprKind::Function(function) => js::statement_lists(&mut function.body, &mut apart),
            _ => {}
        });
    };
    match item {
        js::Item::Function(function) => each(&mut function.body),
        js::Item::Namespace(namespace) => namespace.methods.iter_mut().for_each(|method| each(&mut method.body)),
        js::Item::Statements(stmts) => each(stmts),
        js::Item::Const(_) => {}
    }
}
