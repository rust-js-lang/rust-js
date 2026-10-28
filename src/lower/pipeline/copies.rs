//! The copies of a function its callers need besides its own JS, once
//! lowered (ADR 0051, amended): one its lowering made alike another, its
//! own JS or an earlier copy, is that one, and what calls it calls that.

use std::collections::{HashMap, HashSet};

use rustc_span::def_id::{DefId, LocalModId};

use super::super::analysis::JsGiven;
use super::super::{FnInfo, LoweredFn};
use crate::js::{self, Expr, ExprKind};

/// A function by where it's named: its module, its type's object of
/// methods if it's a method, and its name.
type Named = (LocalModId, Option<String>, String);

/// Of `copies`, each lowered, those made alike another left out, and what
/// calls them, in `items` and the rest, calling that one: the copies kept,
/// each function's in the order they're numbered.
pub(super) fn settle(
    js_given: &JsGiven,
    fns: &HashMap<DefId, FnInfo>,
    items: &mut [(DefId, LoweredFn)],
    copies: Vec<((DefId, usize), LoweredFn)>,
) -> HashMap<DefId, Vec<LoweredFn>> {
    let named = |id: DefId, copy: Option<usize>| -> Named {
        let info = &fns[&id];
        let name = match copy {
            Some(at) => js_given.name(id, at).to_owned(),
            None => info.name.clone(),
        };
        (info.module, info.owner.clone(), name)
    };
    // What each copy made alike another is: that one, its own JS (`None`) or
    // an earlier copy.
    let mut same: HashMap<(DefId, usize), Option<usize>> = HashMap::new();
    let settled = |same: &HashMap<(DefId, usize), Option<usize>>, id: DefId, mut copy: Option<usize>| {
        while let Some(at) = copy {
            match same.get(&(id, at)) {
                Some(&to) => copy = to,
                None => break,
            }
        }
        copy
    };
    let renames = |same: &HashMap<(DefId, usize), Option<usize>>| -> HashMap<Named, String> {
        (same.keys())
            .map(|&(id, at)| (named(id, Some(at)), named(id, settled(same, id, Some(at))).2))
            .collect()
    };
    let main = |id: DefId| items.iter().find(|(of, _)| *of == id).map(|(_, item)| &item.function);
    let mut changed = true;
    while changed {
        changed = false;
        let renamed = renames(&same);
        for ((id, at), copy) in &copies {
            if same.contains_key(&(*id, *at)) {
                continue;
            }
            let module = fns[id].module;
            let earlier = (copies.iter())
                .filter(|((of, other), _)| of == id && other < at && !same.contains_key(&(*of, *other)))
                .map(|((_, other), copy)| (Some(*other), &copy.function));
            // Each read as if it were the one it's asked to be alike, so a
            // copy that calls itself, a walk of a tree, is alike one that
            // calls itself.
            let found = (main(*id).map(|function| (None, function)).into_iter().chain(earlier)).find(|(to, other)| {
                let mut as_if = renamed.clone();
                as_if.insert(named(*id, Some(*at)), named(*id, settled(&same, *id, *to)).2);
                alike(other, module, &as_if) == alike(&copy.function, module, &as_if)
            });
            if let Some((to, _)) = found {
                same.insert((*id, *at), to);
                changed = true;
            }
        }
    }
    let renamed = renames(&same);
    let mut kept: HashMap<DefId, Vec<LoweredFn>> = HashMap::new();
    let mut made = Vec::new();
    for ((id, at), copy) in copies {
        if !same.contains_key(&(id, at)) {
            made.push((id, copy));
        }
    }
    let lowered = (items.iter_mut()).chain(made.iter_mut()).map(|(id, item)| (*id, item));
    for (id, item) in lowered {
        if !renamed.is_empty() {
            rename(&mut item.function, fns[&id].module, &renamed);
        }
        // A call of one made alike another, a call of that one, imported by
        // its name.
        let references = std::mem::take(&mut item.dependencies.copy_references);
        for (from, of, at) in references {
            match settled(&same, of, Some(at)) {
                Some(at) => _ = item.dependencies.copy_references.insert((from, of, at)),
                None => _ = item.dependencies.references.insert((from, of)),
            }
        }
    }
    for (id, copy) in made {
        kept.entry(id).or_default().push(copy);
    }
    kept
}

/// `function` as it reads apart from its name and its export, of a module's
/// `module`, each copy `renamed` named as what it's alike.
fn alike(function: &js::Function, module: LocalModId, renamed: &HashMap<Named, String>) -> js::Function {
    let mut function = function.clone();
    rename(&mut function, module, renamed);
    function.name = String::new();
    function.export = false;
    function
}

/// Each call of a copy in `function`, of a module's `module`, named as the
/// copy's `renamed` says.
fn rename(function: &mut js::Function, module: LocalModId, renamed: &HashMap<Named, String>) {
    let mut each = |e: &mut Expr| match &mut e.kind {
        ExprKind::Var(name) => {
            if let Some(to) = renamed.get(&(module, None, name.clone())) {
                *name = to.clone();
            }
        }
        ExprKind::Symbol(symbol) => {
            if let Some(to) = (renamed.iter())
                .find(|((of, owner, name), _)| {
                    owner.is_none() && of.to_def_id().index.as_u32() == symbol.module && *name == symbol.export
                })
                .map(|(_, to)| to)
            {
                symbol.export = to.clone();
            }
        }
        // A method's, of its type's object, by its own module's name or
        // another's symbol.
        ExprKind::Member(object, name) => {
            let owner = match &object.kind {
                ExprKind::Var(owner) => Some((module, owner.clone())),
                ExprKind::Symbol(symbol) => (renamed.keys())
                    .find(|(of, _, _)| of.to_def_id().index.as_u32() == symbol.module)
                    .map(|(of, _, _)| (*of, symbol.export.clone())),
                _ => None,
            };
            if let Some((of, owner)) = owner
                && let Some(to) = renamed.get(&(of, Some(owner), name.clone()))
            {
                *name = to.clone();
            }
        }
        _ => {}
    };
    js::function_exprs_mut(function, &mut each);
}

/// Of each copy of a function another module calls, `references`'s, the
/// function itself in `functions`, exported.
pub(super) fn export(
    js_given: &JsGiven,
    fns: &HashMap<DefId, FnInfo>,
    references: &HashSet<(LocalModId, DefId, usize)>,
    functions: &mut HashMap<LocalModId, Vec<js::Function>>,
) {
    for &(from, id, at) in references {
        let info = &fns[&id];
        let name = js_given.name(id, at).to_owned();
        if info.module != from
            && let Some(f) = (functions.get_mut(&info.module)).and_then(|fs| fs.iter_mut().find(|f| f.name == name))
        {
            f.export = true;
        }
    }
}

/// What each module imports of each copy of another's function it calls:
/// from that module, by the copy's name.
pub(super) fn imported(
    js_given: &JsGiven,
    fns: &HashMap<DefId, FnInfo>,
    references: &HashSet<(LocalModId, DefId, usize)>,
) -> Vec<(LocalModId, (LocalModId, String, String, String))> {
    (references.iter())
        .map(|&(from, id, at)| {
            let name = js_given.name(id, at).to_owned();
            (from, (fns[&id].module, name.clone(), name.clone(), name))
        })
        .collect()
}

/// What `lowered`, a function of `module`'s, and its `copies` use, into
/// `pass`, and whether one writes JSX.
pub(super) fn gather(
    pass: &mut super::Pass,
    (module, lowered): (LocalModId, &mut LoweredFn),
    copies: &mut [LoweredFn],
) {
    let all = std::iter::once(&mut lowered.dependencies).chain(copies.iter_mut().map(|c| &mut c.dependencies));
    for dependencies in all {
        pass.references.extend(dependencies.references.drain());
        pass.package_uses.extend(dependencies.package_uses.drain());
        pass.held.extend(dependencies.held.drain());
        pass.copy_references.extend(dependencies.copy_references.drain());
    }
    if copies.iter().any(|copy| copy.jsx) {
        pass.jsx.insert(module);
    }
}
