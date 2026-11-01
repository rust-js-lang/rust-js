//! A class's prototype its module's load replaces, React's Flight
//! client's `ReactPromise.prototype = Object.create(Promise.prototype)`,
//! set right after its constructor, before its methods (#448, ADR 0761
//! amended).

use super::{Member, constructor_of, is_class, member};
use crate::js;
use rustc_hir::def::DefKind;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::{DefId, LocalModId};

/// A `js::on_load!` statement that replaces a class's prototype: its
/// module, the class, where, and its statement's place.
pub(crate) struct PrototypeSet {
    module: LocalModId,
    class: DefId,
    at: rustc_span::Span,
    place: js::Span,
}

/// Of a `js::on_load!` body lowered, `body`, which replaces `set`'s
/// class's prototype, where: the statement, set right after its
/// constructor; refused, saying so, where the body has other statements,
/// a `let` it reads say.
pub(crate) fn set_alone(
    tcx: TyCtxt<'_>,
    module: LocalModId,
    set: Option<(DefId, rustc_span::Span)>,
    body: &[js::Stmt],
) -> Option<PrototypeSet> {
    let (class, at) = set?;
    if let [set] = body {
        return Some(PrototypeSet {
            module,
            class,
            at,
            place: set.span,
        });
    }
    let what = format!(
        "replacing the prototype of `{}`, a `#[rust_js::class]` with prototype methods, beside other statements: replace it in a `js::on_load!` of its own (#448)",
        tcx.item_name(class)
    );
    crate::lower::refusal::refuse(tcx, at, crate::lower::refusal::Refused::Feature(&what));
    None
}

/// Each of the crate's classes that has a prototype method or getter,
/// which its module assigns to the prototype it makes (#448).
pub(crate) fn with_prototype_members(tcx: TyCtxt<'_>) -> Vec<DefId> {
    (tcx.hir_crate_items(()).definitions())
        .map(|id| id.to_def_id())
        .filter(|&id| tcx.def_kind(id) == DefKind::Struct && is_class(tcx, id))
        .filter(|&adt| {
            (tcx.inherent_impls(adt).iter())
                .flat_map(|&imp| tcx.associated_item_def_ids(imp).iter().copied())
                .any(|item| matches!(member(tcx, item), Some(Member::Method | Member::Getter)))
        })
        .collect()
}

/// Each class's prototype its module's load replaces, a `js::on_load!` of
/// `set_prototype` alone, `ReactPromise.prototype =
/// Object.create(Promise.prototype)`, given as `(class, its namespace's name,
/// where, its statement's place)`: its item moved to follow the class's,
/// which prints it right after the constructor, before the methods are
/// assigned to the prototype it makes, as upstream orders it
/// (`js::Class::prototype_set`). Refused, saying why, where it's made of
/// what the module makes, which isn't made there, where what the module
/// runs as it loads is between them, which would see the prototype
/// replaced first, and of another module's class or an ES class's, whose
/// `prototype` JS doesn't let be written (#448, ADR 0761 amended).
pub(crate) fn set_prototypes(
    tcx: TyCtxt<'_>,
    items: &mut Vec<js::Item>,
    module: LocalModId,
    sets: &[PrototypeSet],
    owner_of: impl Fn(DefId) -> Option<String>,
) {
    let sets = (sets.iter()).filter(|set| set.module == module);
    for &PrototypeSet {
        class: adt, at, place, ..
    } in sets
    {
        let Some(owner) = constructor_of(tcx, adt).and_then(&owner_of) else {
            continue;
        };
        let owner = owner.as_str();
        let refuse = |why: String| {
            let what = format!(
                "replacing the prototype of `{}`, a `#[rust_js::class]` with prototype methods, {why} (#448)",
                tcx.item_name(adt)
            );
            crate::lower::refusal::refuse(tcx, at, crate::lower::refusal::Refused::Feature(&what));
        };
        let set = (items.iter()).position(
            |item| matches!(item, js::Item::Statements(stmts) if stmts.first().is_some_and(|s| s.span == place)),
        );
        let class = (items.iter())
            .position(|item| matches!(item, js::Item::Namespace(n) if n.name == owner && n.class.is_some()));
        let (Some(set), Some(class)) = (set, class) else {
            refuse(
                "in another module than its class's: replace it in a `js::on_load!` of the class's module".to_owned(),
            );
            continue;
        };
        if let js::Item::Namespace(js::Namespace {
            class: Some(js::Class {
                extends: Some(extends), ..
            }),
            ..
        }) = &items[class]
        {
            refuse(format!(
                "an ES class extending `{extends}`, whose `prototype` JS doesn't let be written"
            ));
            continue;
        }
        // What the module makes, which isn't made yet right after the
        // constructor: a function too, which may read it.
        let made: std::collections::HashSet<&str> = (items.iter())
            .filter_map(|item| match item {
                js::Item::Function(function) => Some(function.name.as_str()),
                item => item.declared(),
            })
            .filter(|&name| name != owner)
            .collect();
        let mut read = None;
        if let js::Item::Statements(stmts) = &items[set] {
            js::visit_stmts(stmts, &mut |name| {
                if read.is_none() && made.contains(name) {
                    read = Some(name.to_owned());
                }
            });
        }
        if let Some(name) = read {
            refuse(format!(
                "of `{name}`, its module's: set right after its constructor, before its methods, it's made of globals and imports alone"
            ));
            continue;
        }
        // What runs as the module loads written between them would see it
        // replaced first: a literal's `const` runs nothing. One written
        // after the load, made before it as what the class's methods
        // read, is made after it, as it's written.
        let (from, to) = (items[class].written_at(), items[set].written_at());
        let written_between = |item: &js::Item| {
            item.written_at()
                .is_some_and(|at| from.is_some_and(|from| from < at) && to.is_some_and(|to| at < to))
        };
        let between = (items.iter())
            .filter(|item| written_between(item))
            .find_map(|item| match item {
                js::Item::Function(_) | js::Item::Namespace(_) => None,
                js::Item::Const(constant) if constant.value.is_constant() => None,
                js::Item::Const(constant) => Some(format!("`{}`", constant.name)),
                js::Item::Statements(_) => Some("another `js::on_load!`".to_owned()),
            });
        if let Some(between) = between {
            refuse(format!(
                "after {between}, which its module makes as it loads: write the `js::on_load!` right after the class's impl"
            ));
            continue;
        }
        let statements = items.remove(set);
        let class = class - usize::from(set < class);
        items.insert(class + 1, statements);
        if let js::Item::Namespace(js::Namespace { class: Some(class), .. }) = &mut items[class] {
            class.prototype_set = true;
        }
    }
}
