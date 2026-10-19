//! What dropping takes, crate-wide: the generic parameters given a drop,
//! and the derives that drop nothing (ADR 0098).

use crate::lower::recognition::{StdItem, is_std_def, known_derive, serde_impl};
use crate::lower::{Body, FnInfo, fn_def};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty;
use rustc_middle::ty::{Ty, TyCtxt, TypeVisitableExt};
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

/// The type parameters of the crate's own generic functions that a caller
/// gives a value with a destructor (ADR 0098), directly or through a
/// generic function of its own: those functions drop a `T` through a drop
/// function they're given. Only they are, so generic code nothing gives such
/// a value to is what it was.
pub(super) fn drop_params<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    fns: &HashMap<DefId, FnInfo>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    library: bool,
    runs: impl Fn(Ty<'tcx>, DefId) -> bool,
) -> HashMap<DefId, Vec<u32>> {
    let mut given: HashSet<(DefId, u32)> = HashSet::new();
    // A crate none of whose values drops anything, a destructor, a counted
    // `Rc`'s count or a guard's borrow, and none of a library's consumers,
    // gives generic code nothing to drop: what's below would give drops
    // nothing calls (ADR 0451).
    let mut asked: HashSet<Ty<'tcx>> = HashSet::new();
    let drops_any = all_bodies.iter().any(|body| {
        body.thir
            .exprs
            .iter()
            .any(|expr| !expr.ty.has_param() && asked.insert(expr.ty) && runs(expr.ty, body.def_id.to_def_id()))
    });
    if !library && !drops_any {
        return HashMap::new();
    }
    // A trait impl's methods are called through its dictionary, or resolved
    // where they're called, by callers the walk below can't see (ADRs 0098,
    // 0100), and what one drops needn't be in its own body: a helper it lends
    // a value to may, or a std method, or a default the trait wrote. So a
    // generic impl is given a drop for each type parameter that isn't `Copy`,
    // and so are its methods, unless it's a derive whose body drops none.
    let impls: HashSet<DefId> = fns
        .keys()
        .filter_map(|&id| {
            tcx.trait_impl_of_assoc(id)
                .or((tcx.def_kind(id) == DefKind::Impl { of_trait: true }).then_some(id))
        })
        .collect();
    for imp in impls {
        if drops_nothing_derived(tcx, imp) {
            continue;
        }
        let typing_env = ty::TypingEnv::non_body_analysis(tcx, imp);
        let owners: Vec<DefId> = std::iter::once(imp)
            .chain(tcx.associated_item_def_ids(imp).iter().copied())
            .collect();
        for param in &tcx.generics_of(imp).own_params {
            if crate::lower::traits::given_a_drop(tcx, typing_env, param) {
                for &id in &owners {
                    given.insert((id, param.index));
                }
            }
        }
    }
    // A trait's generic method, and each impl's of it, is given a drop for
    // each of its own type parameters the trait declares that isn't `Copy`:
    // a caller through a dictionary knows only the trait (ADR 0163).
    for &id in fns.keys() {
        for index in crate::lower::traits::own_drop_params(tcx, foreign, id) {
            given.insert((id, index));
        }
    }
    // A trait's default body is copied into each impl that keeps it (ADR 0049),
    // and dropped there as the impl's type drops: what it gives a generic
    // function of the crate's, as `discard(self)`, that function must be able
    // to drop. So its `Self`, and each type parameter of its trait that isn't
    // `Copy`, count as given a drop, here to be passed on.
    for body in all_bodies {
        let method = tcx.typeck_root_def_id(body.def_id.to_def_id());
        let Some(trait_id) = tcx.trait_of_assoc(method) else {
            continue;
        };
        let typing_env = ty::TypingEnv::non_body_analysis(tcx, method);
        for param in &tcx.generics_of(trait_id).own_params {
            if crate::lower::traits::given_a_drop(tcx, typing_env, param) {
                given.insert((method, param.index));
            }
        }
    }
    // A library's consumers are callers it never sees (ADR 0100): a function
    // of it they can reach is given a drop for each type parameter they could
    // give a value with a destructor, one that isn't `Copy`.
    if library {
        for &id in fns.keys() {
            // A trait impl's, and its methods, are decided above, and a
            // component is given none: React calls it (ADR 0199).
            if crate::lower::bindings::is_component(tcx, id)
                || tcx.trait_of_assoc(id).is_some()
                || tcx.trait_impl_of_assoc(id).is_some()
                || matches!(tcx.def_kind(id), DefKind::Impl { .. })
                || !crate::lower::library::reachable(tcx, id)
            {
                continue;
            }
            let typing_env = ty::TypingEnv::non_body_analysis(tcx, id);
            let generics = tcx.generics_of(id);
            for index in 0..generics.count() {
                let param = generics.param_at(index, tcx);
                if crate::lower::traits::given_a_drop(tcx, typing_env, param) {
                    given.insert((id, index as u32));
                }
            }
        }
    }
    // A caller's type parameter passed on as a callee's: `relay<U>` calling `consume::<U>`.
    let mut passed: Vec<((DefId, u32), (DefId, u32))> = Vec::new();
    for body in all_bodies {
        let caller = tcx.typeck_root_def_id(body.def_id.to_def_id());
        for expr in body.thir.exprs.iter() {
            let (ExprKind::ZstLiteral { .. }, Some((callee, args))) = (&expr.kind, fn_def(expr.ty)) else {
                continue;
            };
            if !fns.contains_key(&callee) || tcx.trait_of_assoc(callee).is_some() {
                continue;
            }
            // React calls a component, with no drop: one given a type with a
            // destructor is an error (ADR 0199).
            if crate::lower::bindings::is_component(tcx, callee) {
                for (index, arg) in args.iter().enumerate() {
                    if let Some(ty) = arg.as_type()
                        && runs(ty, caller)
                    {
                        let (name, param) = (tcx.item_name(callee), tcx.generics_of(callee).param_at(index, tcx).name);
                        let what = format!("giving component `{name}`'s `{param}` a type with a destructor");
                        let refused = crate::lower::refusal::Refused::Because(&what, "React gives a component no drop");
                        crate::lower::refusal::refuse(tcx, expr.span, refused);
                    }
                }
                continue;
            }
            for (index, arg) in args.iter().enumerate() {
                let Some(ty) = arg.as_type() else { continue };
                let index = index as u32;
                if runs(ty, caller) {
                    given.insert((callee, index));
                }
                for part in ty.walk() {
                    if let Some(part) = part.as_type()
                        && let ty::Param(param) = part.kind()
                    {
                        passed.push(((caller, param.index), (callee, index)));
                    }
                }
            }
        }
    }
    let mut changed = true;
    while changed {
        changed = false;
        for &(from, to) in &passed {
            if given.contains(&from) && given.insert(to) {
                changed = true;
            }
        }
    }
    // A std trait's associated type of a type parameter given a drop,
    // `<I as IntoIterator>::IntoIter`, holds what it does: given one too
    // (ADR 0421).
    let mut items: HashMap<DefId, Vec<Ty<'tcx>>> = HashMap::new();
    for (id, index) in given.clone() {
        let count = tcx.generics_of(id).count() as u32;
        let held = items
            .entry(id)
            .or_insert_with(|| crate::lower::traits::drop_items(tcx, foreign, id));
        for (at, &item) in held.iter().enumerate() {
            if crate::lower::traits::alias_of(item)
                .is_some_and(|alias| matches!(alias.self_ty().kind(), ty::Param(p) if p.index == index))
            {
                given.insert((id, count + at as u32));
            }
        }
    }
    let mut params: HashMap<DefId, Vec<u32>> = HashMap::new();
    for (def, index) in given {
        params.entry(def).or_default().push(index);
    }
    for indices in params.values_mut() {
        indices.sort();
    }
    params
}

/// Is `imp` a derive whose body drops nothing of what it's given or makes:
/// `Clone`'s, `Debug`'s, `Default`'s and the comparisons', or serde's, whose
/// codecs rust-js writes (ADR 0077)?
pub(super) fn drops_nothing_derived(tcx: TyCtxt<'_>, imp: DefId) -> bool {
    if !known_derive(tcx, imp) {
        return false;
    }
    let tr = tcx
        .impl_trait_ref(imp)
        .instantiate_identity()
        .skip_normalization()
        .def_id;
    [
        LangItem::Clone,
        LangItem::Copy,
        LangItem::PartialEq,
        LangItem::PartialOrd,
    ]
    .into_iter()
    .any(|item| tcx.is_lang_item(tr, item))
        || [
            StdItem::Eq,
            StdItem::Ord,
            StdItem::Hash,
            StdItem::Debug,
            StdItem::Default,
        ]
        .into_iter()
        .any(|item| is_std_def(tcx, tr, item))
        || serde_impl(tcx, imp).is_some()
}
