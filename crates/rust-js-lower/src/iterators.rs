//! Iterators: a chain of std's adapters on an array or a JS iterator, and
//! what ends it (ADRs 0036, 0055, 0128), lazy where Rust's order can be seen
//! (ADR 0139), and what a chain that owns its items drops (ADR 0098).

mod back_facts;
pub(crate) mod boundary;
mod clones;
mod compiled_steps;
mod dictionaries;
mod eager;
mod ends;
mod held_shadow;
mod hint_asks;
mod hint_knowing;
mod in_place;
pub mod in_place_forms;
mod iterator_form;
mod knowing;
pub mod mir_steps;
mod source_iter;
mod stepping;
mod transparent;
mod value;

pub use self::eager::*;
pub(crate) use self::hint_asks::refuse_unknown_capacity_extends;
pub(crate) use self::hint_asks::{Room, Shunted};
pub(crate) use self::in_place::*;
pub(crate) use self::iterator_form::{OwnMethods, fewest_iterator_args, own_provided_key, stepped_ends};
pub use self::iterator_form::{stepped, stepped_range_by};
pub(crate) use self::stepping::{cloned_apart, stepped_by_parts};
pub(crate) use self::transparent::{Transparency, chains, count_chains};
pub use self::value::{
    EnumeratedForm, Form, IterValue, back_source, bounds_of, bounds_read, chain_parts, chars_made_of, closed,
    closed_around, closed_given_source, closed_parts, count_bound, enumerated_in, enumerated_knowing, enumerated_of,
    form_lacks_a_back, form_lacks_a_length, held_by_a_box, held_by_its_owner, held_of, holding, is_knowing, is_lent,
    is_range_stepped, is_reversed, iterator_parts, its_facts, knows_its_back_by_form, knows_its_given_length,
    lazily_made, lent_of, lent_string_iterator, lent_variable, made_again_runs_nothing, made_knowing_its_length,
    made_string_iterator, mapped_as_it_is, mapped_parts, passes_its_length_on, repeating_n, repeating_n_parts,
    reversed_by, reversed_of, source_mapped_as_it_is, stepped_by_ends, stepped_by_ends_parts, stepped_every,
    stepped_every_parts, stepped_from, stepped_range, stepped_value, steps, unlent, was_made_knowing_its_back,
    was_made_knowing_its_length, with_source, zip_parts,
};
pub(crate) use self::value::{enumerated_passing, made_knowing_its_back};

use super::calls::Call;
use super::combinators::IterSource;
use super::passed_on::{Taker, passed_on};
use super::recognition::{StdItem, std_item, trait_method};
use super::representation::Num;
use super::std_types::range::RangeKind;
use super::{FnCx, R, Std};
use rust_js_program::js;
use rust_js_program::js::{Expr, Op, Stmt, StmtKind};

use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;

/// Whether a bound of `tr` reads its value as its own type, `Clone`'s or
/// `Debug`'s, or the crate's trait that extends `Iterator` (ADR 0511): not
/// a marker's, nor `Iterator`'s or std's extension of it, whose methods
/// its JS iterator answers (ADRs 0164, 0456). The one rule of what's
/// given as it is, of an iterator and of what an `IntoIterator` makes.
fn reads_its_value(tcx: ty::TyCtxt<'_>, tr: DefId) -> bool {
    // `Copy`'s too, which has no method but whose copy function reads it
    // (ADR 0289): a copy of the JS iterator made of a fieldless enum's place
    // read no `copy` (#2540).
    tcx.is_lang_item(tr, rustc_hir::attrs::lang_items::LangItem::Copy)
        || (tr != std_item(tcx, StdItem::Iterator)
            && !super::recognition::is_iterator_extension(tcx, tr)
            && (tcx.associated_items(tr).in_definition_order()).any(|item| item.is_fn()))
}

/// Whether `id` holds what `into` makes as an `IntoIterator`, its
/// `IntoIter`, as it is: bounds it, or `into` itself, by a trait that reads
/// its value, `<T as IntoIterator>::IntoIter: Clone`, `T: Clone` (#2540),
/// or its signature holds it where
/// its caller reads it as the type it is, `-> T::IntoIter`, `it:
/// T::IntoIter`, `Option<T::IntoIter>`, `Pair<I::IntoIter>` given back
/// (`holds_readably`, ADRs 0411, 0561, #2331); not in std's adapter,
/// `Zip<I::IntoIter, J>`, a JS iterator of it; or a bound equates it to a
/// type, `J: IntoIterator<IntoIter = I>` (#2520); or its dictionary gives it, a
/// reference's (ADR 0457), whose `IntoIter` is its own value. Then `into`, a type
/// parameter or an associated type, is given as it is, with its
/// `IntoIterator` dictionary, which makes its `IntoIter` as it is (ADR 0552).
/// Of `id` alone: what an impl's sibling method holds, `Pair<A::IntoIter>`
/// of `pair()`, `first()` is given the same either way (#382).
pub(crate) fn into_iter_held_as_it_is<'tcx>(tcx: ty::TyCtxt<'tcx>, id: DefId, into: Ty<'tcx>) -> bool {
    // An associated type's, `I::Item: IntoIterator`'s, whose values no call
    // makes a JS iterator, an iterator's items, an `Option`'s `T |
    // undefined` among them, given as they are (#278); not what an
    // `into_iter()` makes, `I::IntoIter`, the JS iterator it gives.
    let into_iterator = std_item(tcx, StdItem::IntoIterator);
    let into_iter = super::recognition::trait_method(tcx, into_iterator, "IntoIter");
    if matches!(*into.kind(), ty::Alias(_, ty::AliasTy { kind: ty::Projection { def_id }, .. }) if def_id != into_iter)
    {
        return true;
    }
    // One a dictionary gives, a reference's, `for<'a> &'a mut C:
    // IntoIterator` (ADR 0457): what its `into_iter` makes is its own value,
    // as every dictionary's is (`assoc_iterator_as_it_is`), stepped by its
    // `IntoIterIterator` entry (#2331).
    if super::recognition::into_iter_by_dictionary(tcx, into_iterator, into) {
        return true;
    }
    // A type parameter another bound names, `impl Is<T>`'s: its caller's
    // own value, as it's given it, never the JS iterator a caller makes of
    // what it gives for it (#966).
    if let ty::Param(param) = *into.kind()
        && bound_as_iterator(tcx, id, param)
        && named_by_another_bound(tcx, id, param)
    {
        return true;
    }
    let is_its_iter = |ty: Ty<'tcx>| {
        matches!(*ty.kind(), ty::Alias(_, ty::AliasTy {
            kind: ty::Projection { def_id }, args, ..
        }) if def_id == into_iter && args.type_at(0) == into)
    };
    // One whose `IntoIter` a bound equates to a type, `J: IntoIterator<IntoIter
    // = I>`, `IntoIter = vec::IntoIter<u8>`: what its `into_iter` makes is
    // that type's value, which the body holds as it holds any of it, a type
    // parameter another bound names as it is (#966), a type known whole in
    // its own form, never the JS iterator `$intoIter` makes (#2520).
    let equated = (tcx.clauses_of(id).instantiate_identity(tcx).into_iter())
        .filter_map(|(clause, _)| clause.skip_normalization().as_projection_clause())
        .map(|p| p.skip_binder())
        .any(|p| p.def_id() == into_iter && p.self_ty() == into);
    if equated {
        return true;
    }
    let clauses: Vec<_> = (tcx.clauses_of(id).instantiate_identity(tcx).into_iter())
        .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
        .map(|p| p.skip_binder())
        .collect();
    // Or bounds `into` itself so, `T: IntoIterator + Clone`'s `t.clone()`,
    // `Debug`'s `{t:?}`: what they read is its caller's own value, as an
    // iterator's is (ADR 0474), never the JS iterator a caller made of it,
    // whose clone read `.value` of a `$Steps` (#2540).
    let an_into_iterator = (clauses.iter()).any(|p| p.self_ty() == into && p.def_id() == into_iterator);
    let read = (clauses.iter()).any(|p| {
        (is_its_iter(p.self_ty()) || (an_into_iterator && p.self_ty() == into && p.def_id() != into_iterator))
            && reads_its_value(tcx, p.def_id())
    });
    // Or another bound names it, `R: IteratorIndex<I::IntoIter>` of
    // itertools' `get`: what that one's impl makes of it, `Take<Skip<I>>`,
    // holds the value as it is, which a caller steps as the type it names
    // (#1412), as one another bound names is a caller's own value (#966).
    let named = (clauses.iter()).any(|p| {
        !is_its_iter(p.self_ty())
            && (p.trait_ref.args.iter().skip(1))
                .any(|arg| arg.walk().any(|part| part.as_type().is_some_and(is_its_iter)))
    });
    // Where its caller reads it, `-> T::IntoIter`, `Option<T::IntoIter>`,
    // `Pair<I::IntoIter>` (`holds_readably`, #2331).
    let holds = |ty: Ty<'tcx>| holds_readably(tcx, id, ty, &is_its_iter, &mut Vec::new());
    // A function's signature; an impl's type and its trait's arguments,
    // `One<A::IntoIter>` of `impl<A: IntoIterator> FromT<A> for
    // One<A::IntoIter>`, whose dictionary then takes `A`'s, made for each
    // caller's `A`, as its method that gives it back does: one module
    // const of it gave its method a JS iterable's whatever `A` was (#2263).
    let held = match tcx.def_kind(id) {
        rustc_hir::def::DefKind::Fn | rustc_hir::def::DefKind::AssocFn => {
            let sig = tcx.fn_sig(id).instantiate_identity().skip_binder();
            sig.inputs_and_output.iter().any(holds)
        }
        rustc_hir::def::DefKind::Impl { .. } => {
            let own = tcx.type_of(id).instantiate_identity().skip_normalization();
            let of_trait = (tcx.impl_opt_trait_ref(id))
                .map(|tr| {
                    tr.instantiate_identity()
                        .skip_normalization()
                        .args
                        .types()
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            std::iter::once(own).chain(of_trait).any(holds)
        }
        _ => false,
    };
    read || named || held
}

/// Whether a bound of `id` names `param` the `IntoIter` of an
/// `IntoIterator` it holds as it is, `U` of `I::Item: IntoIterator<IntoIter
/// = U>`: the same value, given as it is (ADR 0552, #278).
fn is_held_into_iter(tcx: ty::TyCtxt<'_>, id: DefId, param: ty::ParamTy) -> bool {
    let into_iterator = std_item(tcx, StdItem::IntoIterator);
    let into_iter = super::recognition::trait_method(tcx, into_iterator, "IntoIter");
    (tcx.clauses_of(id).instantiate_identity(tcx).into_iter())
        .filter_map(|(clause, _)| clause.skip_normalization().as_projection_clause())
        .map(|p| p.skip_binder())
        .any(|p| {
            p.def_id() == into_iter
                && p.term
                    .as_type()
                    .is_some_and(|t| matches!(t.kind(), ty::Param(q) if q.index == param.index))
                && into_iter_held_as_it_is(tcx, id, p.self_ty())
        })
}

/// Whether `ty`, a projection, an associated type's iterator, is its type's
/// own value in `id`, stepped by the `Iterator` its trait's dictionary
/// carries of it, `TMaker.OutIterator()`, never a JS iterator: the one
/// answer of what generic code holds of one (ADR 0061, amended, #2330).
/// std's `IntoIterator`'s `IntoIter`, where `id` holds it as it is (ADR
/// 0552): its caller gives an `IntoIterator` not held so as a JS iterator,
/// `$intoIter`'s, whose `IntoIter` is that JS iterator. Any other trait's
/// that it bounds by `Iterator`, always (`assoc_iterator_as_it_is`).
pub(crate) fn projection_as_it_is<'tcx>(tcx: ty::TyCtxt<'tcx>, id: DefId, ty: Ty<'tcx>) -> bool {
    let as_it_is = projection_as_it_is_of(tcx, id, ty);
    held_shadow::shadow_held(tcx, "projection", id, ty, as_it_is);
    as_it_is
}

fn projection_as_it_is_of<'tcx>(tcx: ty::TyCtxt<'tcx>, id: DefId, ty: Ty<'tcx>) -> bool {
    let ty::Alias(
        _,
        ty::AliasTy {
            kind: ty::Projection { def_id },
            args,
            ..
        },
    ) = *ty.kind()
    else {
        return false;
    };
    let into_iterator = std_item(tcx, StdItem::IntoIterator);
    if def_id == super::recognition::trait_method(tcx, into_iterator, "IntoIter") {
        return into_iter_held_as_it_is(tcx, id, args.type_at(0));
    }
    // An iterator's items `id` bounds by `Iterator` itself, `<S as
    // Iterator>::Item: Iterator`, or an `IntoIterator`'s: values no call
    // makes a JS iterator, which come as their caller holds them, the
    // crate's own its struct, a range its own, stepped by that bound's
    // dictionary, as what an `I::Item: IntoIterator` makes is (#278,
    // #2541). Not items another bound takes, `I::Item: IntoIterator` of
    // std's `Flatten`, whose `into_iter` makes what it steps.
    let iterator = std_item(tcx, StdItem::Iterator);
    if [iterator, into_iterator]
        .map(|tr| super::recognition::trait_method(tcx, tr, "Item"))
        .contains(&def_id)
        && (tcx.clauses_of(id).instantiate_identity(tcx).into_iter())
            .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
            .any(|p| p.def_id() == iterator && p.skip_binder().self_ty() == ty)
    {
        return true;
    }
    assoc_iterator_as_it_is(tcx, def_id)
}

/// Whether a value of `item`, an associated type, is its type's own value
/// in generic code wherever it's held, which its trait's dictionary steps:
/// one of a trait not std's that bounds it by `Iterator`, `type Out:
/// Iterator`, whose values only its impls' methods make, as they make them,
/// a range a range, the crate's own iterator itself, so one value has one
/// representation in generic code and concrete alike (ADR 0061, amended,
/// #2328, #2330). And std's `IntoIterator`'s `IntoIter`, which an impl's
/// dictionary makes as its own value too, a range a range, stepped by its
/// `IntoIterIterator` entry: one dictionary, which generic code that holds
/// it as it is and generic code given it by a reference's bound both take
/// (ADRs 0457, 0552, #2331). `held_by_generic_code` asks it of an impl's.
pub(crate) fn assoc_iterator_as_it_is(tcx: ty::TyCtxt<'_>, item: DefId) -> bool {
    if item == trait_method(tcx, std_item(tcx, StdItem::IntoIterator), "IntoIter") {
        return true;
    }
    !super::recognition::is_std_crate(tcx, item)
        && matches!(tcx.def_kind(item), rustc_hir::def::DefKind::AssocTy)
        // Not a method's `-> impl Iterator`, rustc's associated type of it,
        // whose type no caller names to read: a JS iterator, as this ADR's
        // #2005 amendment has it.
        && !tcx.is_impl_trait_in_trait(item)
        && (tcx.item_bounds(item).skip_binder().iter())
            .filter_map(|bound| bound.as_trait_clause())
            .any(|p| super::recognition::is_std_def(tcx, p.def_id(), StdItem::Iterator) || super::recognition::is_iterator_extension(tcx, p.def_id()))
}

/// Whether `callee`'s type parameter `index` is its trait's, `Self` or a
/// parameter of the trait a method of it is called through: given as the
/// trait's impl takes it, never made a JS iterator for it, which only a
/// method's own parameters are (ADR 0551).
pub(crate) fn given_by_its_trait(tcx: ty::TyCtxt<'_>, callee: DefId, index: u32) -> bool {
    tcx.trait_of_assoc(callee).is_some() && (index as usize) < tcx.generics_of(callee).parent_count
}

/// Whether `id`'s type parameter `param`, bound by `Iterator`, is given as it
/// is (ADR 0411): one an impl's type holds, `I` of `Doubled<I>`, or the
/// function's signature has in anything but itself, `Wrap<I>` or a result,
/// or another bound reads as its own type, `I: Clone`'s `clone(&it)` (ADR
/// 0474), whose value its caller may read as its own type, and so stepped
/// by its `Iterator` dictionary, `{ next }`, where it's stepped. One given
/// only itself, `i: I`, is a JS iterator (ADR 0061). The one answer, of a
/// callee's evidence and its caller's argument alike: of a call, `args`
/// its generic arguments, where what std's catalog compiles is asked of
/// the call (`compiled_instance`), `partition` into `Vec` sides its
/// helper's, never std's body; of the definition, `None` (#2398).
pub(crate) fn given_as_it_is<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    (id, args): (DefId, Option<ty::GenericArgsRef<'tcx>>),
    param: ty::ParamTy,
) -> bool {
    let as_it_is = given_as_it_is_of(tcx, (id, args), param);
    rust_js_program::decisions::record("iterator-given", if as_it_is { "as-it-is" } else { "js-iterator" });
    held_shadow::shadow_held(tcx, "given", id, Ty::new_param(tcx, param.index, param.name), as_it_is);
    as_it_is
}

fn given_as_it_is_of<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    (id, args): (DefId, Option<ty::GenericArgsRef<'tcx>>),
    param: ty::ParamTy,
) -> bool {
    // Bound by `Iterator` itself, or a trait that extends it, std's or the
    // crate's, whose `Iterator` `bounds` gives it (ADRs 0488, 0511), as
    // `bounds` reads its clauses, which make its evidence.
    // An `IntoIterator` whose `IntoIter` it holds as it is (ADR 0552).
    if into_iter_held_as_it_is(tcx, id, Ty::new_param(tcx, param.index, param.name)) {
        return true;
    }
    // What another's `IntoIterator` makes as it's held, `U` of `I::Item:
    // IntoIterator<IntoIter = U>`, `Flatten`'s (#278).
    if is_held_into_iter(tcx, id, param) {
        return true;
    }
    let iterator = std_item(tcx, StdItem::Iterator);
    let of_param: Vec<DefId> = tcx
        .clauses_of(id)
        .instantiate_identity(tcx)
        .into_iter()
        .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
        .filter(|p| p.self_ty().skip_binder() == Ty::new_param(tcx, param.index, param.name))
        .map(|p| p.def_id())
        .collect();
    if !of_param
        .iter()
        .any(|&tr| tr == iterator || super::recognition::implies_iterator(tcx, tr))
    {
        // One whose own bound names an iterator given as it is, `impl
        // Is<I>`'s, `A: Into<I>`'s: what its methods give of that one is
        // its own value, which goes as it is, as that one's does (#966).
        return names_given_as_it_is(tcx, (id, args), param);
    }
    // A bound whose methods read its value as its own type, `Clone`'s or
    // `Debug`'s, or the crate's trait that extends `Iterator` (ADR 0511):
    // not a marker's, nor std's extension of `Iterator`,
    // `DoubleEndedIterator`, whose methods its JS iterator answers (ADRs
    // 0164, 0456).
    let read = of_param.iter().any(|&tr| reads_its_value(tcx, tr));
    // A value of it its caller reads (`holds_readably`).
    let itself = |ty: Ty<'tcx>| matches!(ty.kind(), ty::Param(p) if p.index == param.index);
    let has = |ty: Ty<'tcx>| holds_readably(tcx, id, ty, &itself, &mut Vec::new());
    // An impl's type, `Doubled<I>`, or an associated type's value it gives,
    // `type Output = Take<Skip<I>>` of itertools' `IteratorIndex`, which
    // its callers step as the type it names, std's struct holding their own
    // value, as a function's result is (#1412).
    let held = match tcx.def_kind(id) {
        rustc_hir::def::DefKind::Impl { .. } => Some(id),
        _ => tcx.impl_of_assoc(id),
    }
    .is_some_and(|imp| {
        has(tcx.type_of(imp).instantiate_identity().skip_normalization())
            || (tcx.associated_items(imp).in_definition_order())
                .filter(|item| item.is_type())
                .any(|item| has(tcx.type_of(item.def_id).instantiate_identity().skip_normalization()))
    });
    // A trait's own method takes `Self` as its dictionary gives it, but one
    // compiled into `@rust-js/std` as a function over `Self`, `peekable`'s
    // (ADR 0185, 0561), as a function's signature says.
    let signature = matches!(
        tcx.def_kind(id),
        rustc_hir::def::DefKind::Fn | rustc_hir::def::DefKind::AssocFn
    ) && (tcx.trait_of_assoc(id).is_none()
        || match args {
            Some(args) => super::recognition::compiled_instance(tcx, id, args),
            None => super::recognition::compiled_std_body(tcx, id),
        })
        && tcx.trait_impl_of_assoc(id).is_none()
        && {
            let sig = tcx.fn_sig(id).instantiate_identity().skip_binder();
            // A `&mut I` and another value of it, an `I` or a `&mut I`,
            // which its body may write there, `*it = next`, `mem::swap(a,
            // b)`: its caller's own, one representation, written in place
            // (#537).
            let lent = |ty: Ty<'tcx>| matches!(*ty.kind(), ty::Ref(_, inner, ty::Mutability::Mut) if itself(inner));
            let inputs = sig.inputs();
            let given = inputs.iter().filter(|&&input| itself(input) || lent(input));
            let meet = inputs.iter().any(|&input| lent(input)) && given.count() > 1;
            // Not a `Box<I>`, which is `I`'s value, stepped as `I` is
            // (`iterator_held`, #1601).
            let inside = (inputs.iter()).any(|&input| !itself(crate::recognition::iterator_held(input)) && has(input));
            has(sig.output()) || meet || inside
        };
    // Another bound that names it, `impl Is<I>`'s (#966, #969), is given as
    // it is by the first test, `into_iter_held_as_it_is`'s, of a parameter
    // bound as an iterator, as one here is.
    held || signature || read
}

/// Whether `ty`, a type in `id`'s signature, holds a value `sought` takes
/// where `id`'s caller reads it as the type it is: itself, or in what holds a
/// value as it is, a tuple, an `Option`, a `Vec`, a type of the crate's or a
/// compiled std struct (ADRs 0411, 0552, #2331). What an `impl Trait` of
/// `id`'s crate hides is read as the type it is, `I` of `fn f<I: Iterator>(it:
/// I) -> impl Iterator { it }`, whose caller reveals it as the value it gave
/// (#1142, #1230, ADR 0920); `seen`, the opaque types read, each once. Not in
/// a projection, `Vec<I::Item>`, whose value isn't given back, nor in std's
/// adapter made a JS iterator, `Zip<I, J>`, which its caller steps and never
/// reads as what it holds, but in std's own compile of it. The one walk, of
/// a type parameter given as it is and of an `IntoIter` held as it is.
fn holds_readably<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    id: DefId,
    ty: Ty<'tcx>,
    sought: &dyn Fn(Ty<'tcx>) -> bool,
    seen: &mut Vec<DefId>,
) -> bool {
    let mut walk = ty.walk();
    while let Some(arg) = walk.next() {
        let Some(part) = arg.as_type() else { continue };
        if sought(part) {
            return true;
        }
        match *part.kind() {
            ty::Alias(
                _,
                ty::AliasTy {
                    kind: ty::Opaque { def_id },
                    args,
                    ..
                },
            ) if def_id.krate == id.krate && !seen.contains(&def_id) => {
                walk.skip_current_subtree();
                seen.push(def_id);
                let hidden = tcx.type_of(def_id).instantiate(tcx, args).skip_normalization();
                if holds_readably(tcx, id, hidden, sought, seen) {
                    return true;
                }
            }
            ty::Alias(..) => walk.skip_current_subtree(),
            // Not of `id`'s own crate, std's function's own adapter, `Map<I,
            // F>` of core's `map`, which its struct holds as it is (ADR
            // 0561): asked of `id`, as the opaque arm above asks, so its
            // own compile, `@rust-js/std`'s, and each consumer's answer
            // alike (#1025). `is_compiled_std_type` answers of the type in
            // the build compiling, `compiled_for` this one, which the
            // answer of a std function's signature mustn't hang on.
            ty::Adt(adt, _)
                if super::recognition::is_iter_adapter_def(tcx, adt.did())
                    && !super::recognition::is_compiled_std_type(tcx, part)
                    && adt.did().krate != id.krate =>
            {
                walk.skip_current_subtree()
            }
            _ => {}
        }
    }
    false
}

/// Whether another bound of `id` names its type parameter `param`, of
/// another type's, `A: Into<I>`, an `impl Is<I>` parameter's, `F:
/// FnMut(I)`: what that one's methods give of it, or are given, is its
/// caller's own value, which knows it as the type it is, never a JS
/// iterator made for it (#966, #969). Not in a projection, `Vec<I::Item>`.
pub(crate) fn named_by_another_bound<'tcx>(tcx: ty::TyCtxt<'tcx>, id: DefId, param: ty::ParamTy) -> bool {
    let itself = |ty: Ty<'tcx>| matches!(ty.kind(), ty::Param(p) if p.index == param.index);
    let has = |ty: Ty<'tcx>| {
        let mut walk = ty.walk();
        while let Some(arg) = walk.next() {
            match arg.as_type().map(|t| *t.kind()) {
                Some(ty::Alias(..)) => walk.skip_current_subtree(),
                Some(ty::Param(p)) if p.index == param.index => return true,
                _ => {}
            }
        }
        false
    };
    (tcx.clauses_of(id).instantiate_identity(tcx).into_iter()).any(|(clause, _)| {
        let clause = clause.skip_normalization();
        if let Some(bound) = clause.as_trait_clause() {
            let bound = bound.skip_binder();
            return !itself(bound.self_ty())
                && bound
                    .trait_ref
                    .args
                    .iter()
                    .skip(1)
                    .any(|arg| arg.as_type().is_some_and(has));
        }
        clause.as_projection_clause().is_some_and(|p| {
            let p = p.skip_binder();
            !itself(p.self_ty())
                && (p
                    .projection_term
                    .args
                    .iter()
                    .skip(1)
                    .any(|arg| arg.as_type().is_some_and(has))
                    || p.term.as_type().is_some_and(has))
        })
    })
}

/// Whether a bound of `id`'s type parameter `param`, itself no iterator,
/// names another, bound by `Iterator` and given as it is, or an
/// `IntoIterator` held as it is (`into_iter_held_as_it_is`), `I` of `P:
/// Is<I>`, `A: Into<I>` or `F: FnOnce() -> I`: the one value passes between
/// them, so one representation (#966, #969).
fn names_given_as_it_is<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    (id, args): (DefId, Option<ty::GenericArgsRef<'tcx>>),
    param: ty::ParamTy,
) -> bool {
    let itself = |ty: Ty<'tcx>| matches!(ty.kind(), ty::Param(p) if p.index == param.index);
    let iterator = std_item(tcx, StdItem::Iterator);
    // Bound by `Iterator`, or a trait that extends it, as `given_as_it_is`
    // reads it past its first test, so never back here.
    let stepped = |q: ty::ParamTy| {
        (tcx.clauses_of(id).instantiate_identity(tcx).into_iter())
            .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
            .filter(|b| matches!(b.self_ty().skip_binder().kind(), ty::Param(s) if s.index == q.index))
            .any(|b| b.def_id() == iterator || super::recognition::implies_iterator(tcx, b.def_id()))
    };
    let names = |ty: Ty<'tcx>| {
        let mut walk = ty.walk();
        while let Some(arg) = walk.next() {
            match arg.as_type().map(|t| *t.kind()) {
                Some(ty::Alias(..)) => walk.skip_current_subtree(),
                Some(ty::Param(p)) if p.index != param.index && stepped(p) && given_as_it_is(tcx, (id, args), p) => {
                    return true;
                }
                // An `IntoIterator` held as it is, `impl Is<T>`'s `T`.
                Some(ty::Param(p))
                    if p.index != param.index
                        && bound_as_iterator(tcx, id, p)
                        && into_iter_held_as_it_is(tcx, id, Ty::new_param(tcx, p.index, p.name)) =>
                {
                    return true;
                }
                _ => {}
            }
        }
        false
    };
    (tcx.clauses_of(id).instantiate_identity(tcx).into_iter()).any(|(clause, _)| {
        let clause = clause.skip_normalization();
        if let Some(bound) = clause.as_trait_clause() {
            let bound = bound.skip_binder();
            return itself(bound.self_ty())
                && bound
                    .trait_ref
                    .args
                    .iter()
                    .skip(1)
                    .any(|arg| arg.as_type().is_some_and(names));
        }
        clause.as_projection_clause().is_some_and(|p| {
            let p = p.skip_binder();
            itself(p.self_ty()) && p.term.as_type().is_some_and(names)
        })
    })
}

/// Whether `callee`'s type parameter `param` is bound by `Iterator`, a
/// trait that extends it, or `IntoIterator`, as its bounds elaborate them:
/// what's given for it may be stepped, a JS iterator where it's not given
/// as it is (ADR 0061). One bound by neither holds what it's given, as
/// `mem::replace`'s `T` and `Vec::push`'s do (#537).
pub(crate) fn bound_as_iterator(tcx: ty::TyCtxt<'_>, callee: DefId, param: ty::ParamTy) -> bool {
    bound_by(
        tcx,
        callee,
        param.index,
        &[std_item(tcx, StdItem::Iterator), std_item(tcx, StdItem::IntoIterator)],
    )
}

/// Whether `callee`'s type parameter of `index` is bound by `Iterator`
/// itself, or a trait that extends it, not `IntoIterator` alone
/// (`bound_as_iterator`): what's given for it is an iterator, stepped as
/// it is, not what makes one.
pub(crate) fn bound_by_iterator(tcx: ty::TyCtxt<'_>, callee: DefId, index: u32) -> bool {
    bound_by(tcx, callee, index, &[std_item(tcx, StdItem::Iterator)])
}

fn bound_by(tcx: ty::TyCtxt<'_>, callee: DefId, index: u32, traits: &[DefId]) -> bool {
    (tcx.param_env(callee).caller_bounds())
        .filter_map(|clause| clause.as_trait_clause())
        .any(|bound| {
            matches!(bound.self_ty().skip_binder().kind(), ty::Param(p) if p.index == index)
                && traits.contains(&bound.def_id())
        })
}

/// The value `(p) => p === value` or `(p) => value === p` looks for, if
/// it doesn't depend on `p` and reads the same however often it's read.
pub(crate) fn sought(test: &Expr) -> Option<Expr> {
    let js::ExprKind::Arrow(params, body) = &test.kind else {
        return None;
    };
    let (
        [js::Pattern::Name(param)],
        [
            Stmt {
                kind: StmtKind::Return(Some(e)),
                ..
            },
        ],
    ) = (params.as_slice(), body.as_slice())
    else {
        return None;
    };
    let js::ExprKind::Binary(Op::Eq, a, b) = &e.kind else {
        return None;
    };
    let is_param = |e: &Expr| matches!(&e.kind, js::ExprKind::Var(name) if name == param);
    let value = match (is_param(a), is_param(b)) {
        (true, false) => b,
        (false, true) => a,
        _ => return None,
    };
    (value.reads_same() && !value.mentions_name(param)).then(|| (**value).clone())
}

/// Whether `test`, one `sought` reads, compares what's never a NaN, as
/// Rust's types tell (`Known::NeverNaN`): what an array's `includes` finds
/// as its `===` does (rd353).
pub(crate) fn compares_never_nan(test: &Expr) -> bool {
    matches!(&test.kind, js::ExprKind::Arrow(_, body)
        if matches!(body.as_slice(), [Stmt { kind: StmtKind::Return(Some(e)), .. }]
            if matches!(e.kind, js::ExprKind::Binary(Op::Eq, ..)) && e.facts.has(js::Known::NeverNaN)))
}

/// `items` owned, an array of its own: itself, where it's one made here
/// (`made_array`); a range of a slice, `$slice(v, a, b)` or `$part(v, a,
/// b)`, a part that says where it is in `v`, copied once, `$copyOf(v, a,
/// b)`, which no record places there, so a pointer into it is its own
/// (#1185); a deque's range, `$sliceRange(v, a, b)`, copied once as
/// `slice::range` checks it, `$copyRange(v, a, b)` (#1937); else a copy,
/// `items.slice()`.
pub(crate) fn owned_items(items: Expr) -> Expr {
    if made_array(&items) {
        return items;
    }
    match &items.kind {
        js::ExprKind::Call(callee, args) if matches!(callee.helper_name(), Some("$slice" | "$part")) => {
            Expr::call(Expr::helper("$copyOf"), args.clone())
        }
        js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$sliceRange") => {
            Expr::call(Expr::helper("$copyRange"), args.clone())
        }
        _ => Expr::builtin(items, "slice", vec![]),
    }
}

/// Whether `e` is an array as it's made: `[a, b]`, `Array.from(x)`, a
/// string's `split` or a helper's, and through an array's methods that make one,
/// `.filter(f)`, not an iterator's of the same names, `$iter(xs).filter(f)`.
pub(super) fn made_array(e: &Expr) -> bool {
    match &e.kind {
        js::ExprKind::Array(..) => true,
        // A slice's items in a range in bounds, `items.slice(a, b)` (ADR 0806).
        _ if e.facts.has(js::Known::Sliced) => true,
        // An `Option`'s items, `o == null ? [] : [o]`.
        js::ExprKind::Cond(..) => fresh(e),
        // A helper's that makes one, `$windows(v, 2)`.
        js::ExprKind::Call(callee, _) if matches!(callee.kind, js::ExprKind::Var(_)) => fresh(e),
        js::ExprKind::Call(callee, _) => match &callee.kind {
            js::ExprKind::Member(object, name) => match name.as_str() {
                "split" => true,
                "from" => matches!(&object.kind, js::ExprKind::Var(v) if v.is("Array")),
                "map" | "filter" | "slice" | "toReversed" | "toSorted" | "concat" | "flat" | "flatMap" => {
                    made_array(object)
                }
                _ => false,
            },
            _ => false,
        },
        _ => false,
    }
}

/// Whether `e` is `$iter(items)`, `stepped_items`'s.
/// `$iter(v).map(f)`, a map of an array's items, made and not yet stepped:
/// nothing runs till it is, nor made again (`made_again_runs_nothing`).
pub fn is_items_map(e: &Expr) -> bool {
    if let js::ExprKind::Call(callee, args) = &e.kind
        && let [f] = args.as_slice()
        && let js::ExprKind::Member(source, method) = &callee.kind
        && source.helper_call_of("$iter").is_some()
    {
        method == "map" && f.reads_only_vars() && made_again_runs_nothing(source)
    } else {
        false
    }
}

/// `it.map(f)` its maker said is a JS iterator's (`Known::Iterator`, MIR's
/// map of what `method_source` made one), of an `f` that reads only
/// variables: lazy, nothing run till it's stepped, never an array's eager
/// `map` (#805).
pub fn lazily_mapped(e: &Expr) -> bool {
    if let js::ExprKind::Call(callee, args) = &e.kind
        && let [f] = args.as_slice()
        && let js::ExprKind::Member(_, method) = &callee.kind
    {
        e.facts.has(js::Known::Iterator) && method == "map" && f.reads_only_vars()
    } else {
        false
    }
}

/// `it.map(f)`, a map of what a variable holds, made and not yet stepped:
/// the variable.
pub fn mapped_var(e: &Expr) -> Option<&js::Name> {
    match &e.kind {
        js::ExprKind::Call(callee, args)
            if let [f] = args.as_slice()
                && let js::ExprKind::Member(source, method) = &callee.kind
                && let js::ExprKind::Var(name) = &source.kind
                && method == "map"
                && f.reads_only_vars() =>
        {
            Some(name)
        }
        _ => None,
    }
}

/// `$iter($iter(items))`, an array's iterator made one again where a name
/// that held one was read in place: the one, as the runtime's `$iter` gives
/// one back as it is, an outer `lowerless` unread (#1152).
pub fn stepped_once(e: &mut Expr) {
    while (e.helper_call_of("$iter").and_then(|made| made.arg("items"))).is_some_and(is_stepped_items)
        && let js::ExprKind::Call(_, args) = &mut e.kind
    {
        *e = args.swap_remove(0);
    }
}

pub fn is_stepped_items(e: &Expr) -> bool {
    fresh_iter_source(e).is_some()
}

/// What a binding's call made, `made`, of type `output`: of a `dyn
/// Iterator`, boxed or lent, marked JS's own (`Known::BindingIterator`,
/// #1359).
pub fn made_by_a_binding(made: Expr, output: Ty<'_>) -> Expr {
    let held = output.peel_refs();
    match held.boxed_ty().unwrap_or(held).kind() {
        ty::Dynamic(..) => Expr {
            facts: made.facts.with(js::Known::BindingIterator),
            ..made
        },
        _ => made,
    }
}

/// Whether what `e` steps is a `dyn Iterator` a binding made, JS's own
/// (`made_by_a_binding`): `e` itself, or a JS adapter of one source alone
/// made of it, `.map(f)`, `.filter(f)`, `.take(n)`, `.drop(n)`, whose hint
/// is of that source's. Not a `flatMap`'s, whose inner iterators may be
/// what Rust made.
pub fn binding_made(e: &Expr) -> bool {
    if e.facts.has(js::Known::BindingIterator) {
        return true;
    }
    match &e.kind {
        js::ExprKind::Call(callee, _) => match &callee.kind {
            js::ExprKind::Member(object, name) if matches!(name.as_str(), "map" | "filter" | "take" | "drop") => {
                binding_made(object)
            }
            _ => false,
        },
        _ => false,
    }
}

/// Whether `e` is a JS iterator: one its type says is (`Known::Iterator`),
/// or as it's made, `Iterator.from(..)`, a
/// `$iter`, a runtime helper's that gives one (`runtime::gives_iterator`,
/// as its own source says, #2229), a range's `$iterator` or a
/// `$lazyChain`'s generator, a collection's `values()`, `keys()` or
/// `entries()`, or an iterator helper's of one, `.map(f)` of a range's.
/// Whether `e` is a call of JS's own adapter, `it.map(f)`, `filter`,
/// `flatMap`, `take` or `drop`, of a JS iterator: what the recognizers make
/// in place, never std's compiled struct, whatever marks it carries (#1491).
fn is_js_adapter(e: &Expr) -> bool {
    matches!(&e.kind, js::ExprKind::Call(callee, _)
        if matches!(&callee.kind, js::ExprKind::Member(object, name)
            if matches!(name.as_str(), "map" | "filter" | "flatMap" | "take" | "drop") && is_js_iterator(object)))
}

/// Which of JS's own iterators the recognizers made in place `e` is, the
/// one answer of what its JS form knows (#1792, #1803):
/// - `Hintless`: an `Iterator.prototype`'s, an array's `$iter(v)`, a
///   collection's `values()`, `keys()` or `entries()`, JS's own adapters,
///   `map`, `filter`, `flatMap`, `take` or `drop`, of one, or a variable
///   that holds one (`Known::IterHintless`, `marks_kept`): it knows no hint
///   and has no fold, whatever std's struct of its type knows;
/// - `Range`: a range's `$lazyRange`, or the runtime's own adapters of one,
///   whose hint is std's and whose fold steps it by its `next`.
///
/// `None` of another: std's compiled struct, a runtime `$Steps` of what
/// isn't one of these, whose fold is its own.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnForm {
    Hintless,
    Range,
}

pub(crate) fn own_form(e: &Expr) -> Option<OwnForm> {
    let js::ExprKind::Call(callee, args) = &e.kind else {
        return e.facts.has(js::Known::IterHintless).then_some(OwnForm::Hintless);
    };
    if e.facts.has(js::Known::IterHintless) || fresh_iter_source(e).is_some() {
        return Some(OwnForm::Hintless);
    }
    // A range stepped by the runtime, `$iterator(range, $rangeNext)` as
    // lowering makes it (`Known::IterRange`), printed `$lazyRange(a, b)`.
    if e.facts.has(js::Known::IterRange) || callee.helper_name() == Some(super::std_types::range::LAZY_RANGE) {
        return Some(OwnForm::Range);
    }
    match &callee.kind {
        js::ExprKind::Member(_, name) if args.is_empty() && matches!(name.as_str(), "values" | "keys" | "entries") => {
            Some(OwnForm::Hintless)
        }
        js::ExprKind::Member(object, name)
            if matches!(name.as_str(), "map" | "filter" | "flatMap" | "take" | "drop") =>
        {
            own_form(object)
        }
        _ => None,
    }
}

/// `it`, JS's own adapters of a JS iterator, `$iter(v).map(f).filter(g)` or
/// `opaque($iter(v)).map(f)`, made again on a source the runtime's adapters
/// link to, `$stepsAround($iter(v)).map(f).filter(g)`: `$Steps`' own `map`,
/// `filter`, `flatMap`, `take` and `drop`, each of which keeps what it
/// steps (`from`), so what follows the links from it finds the source
/// `Vec`, as std's `SourceIter` does, where JS's own keep none (#1941).
/// `None` of another form, an array's own `map` among them, or of a source
/// alone, which is found as it is.
pub(crate) fn with_linked_source(it: &Expr) -> Option<Expr> {
    let js::ExprKind::Call(callee, args) = &it.kind else {
        return None;
    };
    // The `Some`s, `$somes`'s, a filter of what it steps (#2199), and a
    // flatten's items of them, `$someItems`'s (#2205).
    if let Some(helper) = (["$somes", "$someItems"].into_iter()).find(|&h| callee.helper_name() == Some(h))
        && let [kept] = args.as_slice()
    {
        return with_linked_source(kept).map(|linked| Expr::call_helper(helper, vec![linked]));
    }
    let js::ExprKind::Member(object, name) = &callee.kind else {
        return None;
    };
    if !matches!(name.as_str(), "map" | "filter" | "flatMap" | "take" | "drop") {
        return None;
    }
    let object = match with_linked_source(object) {
        Some(linked) => linked,
        None if is_js_iterator(object) || own_form(object) == Some(OwnForm::Hintless) => {
            Expr::call_helper("$stepsAround", vec![(**object).clone()])
        }
        None => return None,
    };
    Some(Expr::call(Expr::member(object, name.clone()), args.clone()))
}

/// Whether `it` is JS's own adapters of JS's own iterator (`own_form`'s
/// `Hintless`): what knows no hint, whatever std's struct of its type
/// knows. `v.iter().inspect(f)` made in place, `$iter(v).map(..)`, has
/// none, where std's `Inspect` of a slice's knows its length (#1792).
pub(crate) fn form_knows_no_hint(it: &Expr) -> bool {
    matches!(&it.kind, js::ExprKind::Call(callee, _)
        if matches!(&callee.kind, js::ExprKind::Member(_, name)
            if matches!(name.as_str(), "map" | "filter" | "flatMap" | "take" | "drop")))
        && own_form(it) == Some(OwnForm::Hintless)
}

/// Whether `it`'s fold is the steps its `next` takes, so a consumer that
/// asks it by its fold steps it as by `next` (`own_form`'s either): JS's
/// own iterator, which has none, or a range's `$lazyRange` and its runtime
/// adapters, whose fold steps it by `next`. `$extend(v, it, true)` of one
/// would read as std's `extend_trusted`, and step it the same (#1803).
pub(crate) fn folds_as_it_steps(it: &Expr) -> bool {
    own_form(it).is_some()
}

fn hintless(e: &Expr) -> bool {
    own_form(e) == Some(OwnForm::Hintless)
}

/// What a variable that holds `value` keeps of it, a spill's or a MIR
/// local's: its marks (`held_marks`), and whether it's JS's own iterator,
/// which knows no hint (`hintless`, #1792).
pub fn marks_kept(value: &Expr) -> js::Facts {
    let marks = held_marks(value.facts);
    match hintless(value) {
        true => marks.with(js::Known::IterHintless),
        false => marks,
    }
}

pub fn is_js_iterator(e: &Expr) -> bool {
    if e.facts.has(js::Known::Iterator) {
        return true;
    }
    let js::ExprKind::Call(callee, _) = &e.kind else {
        return false;
    };
    // A runtime helper's call, as its own source says it gives (#2229).
    if let Some(helper) = callee.helper_name() {
        return rust_js_program::runtime::gives_iterator(helper) || is_stepped_items(e);
    }
    match &callee.kind {
        // Not `Object.keys(o)`'s, an array.
        js::ExprKind::Member(object, name) => match &object.kind {
            js::ExprKind::Var(v) if v.is("Object") => false,
            js::ExprKind::Var(v) if v.is("Iterator") => name == "from",
            _ if matches!(name.as_str(), "map" | "filter" | "flatMap" | "take" | "drop") => is_js_iterator(object),
            _ => matches!(name.as_str(), "values" | "keys" | "entries"),
        },
        _ => false,
    }
}

/// A string's `chars()` stepped only from its front, JS's own string
/// iterator, `s[Symbol.iterator]()`, which steps its `char`s as Rust's does
/// (#1186): a JS iterator, of what JS's own iteration of `s` steps.
pub(crate) fn string_iterator(s: Expr) -> Expr {
    let it = Expr::call(Expr::index(s, Expr::member(Expr::global("Symbol"), "iterator")), vec![]);
    Expr {
        facts: it.facts.with(js::Known::Iterator).with(js::Known::ReceiverIterated),
        ..it
    }
}

/// Of `string_iterator(s)`, as `e` is, `s`.
pub fn string_iterated(e: &Expr) -> Option<&Expr> {
    match &e.kind {
        js::ExprKind::Call(callee, args)
            if args.is_empty()
                && let js::ExprKind::Index(s, key) = &callee.kind
                && let js::ExprKind::Member(symbol, name) = &key.kind
                && name == "iterator"
                && matches!(&symbol.kind, js::ExprKind::Var(v) if v.is("Symbol")) =>
        {
            Some(s)
        }
        _ => None,
    }
}

/// What `Iterator.from(x)`, as `e` is, makes a JS iterator of: `x`.
pub(crate) fn iterator_from(e: &Expr) -> Option<&Expr> {
    match &e.kind {
        js::ExprKind::Call(callee, args)
            if let [x] = args.as_slice()
                && matches!(&callee.kind, js::ExprKind::Member(object, name)
                    if name == "from" && matches!(&object.kind, js::ExprKind::Var(v) if v.is("Iterator"))) =>
        {
            Some(x)
        }
        _ => None,
    }
}

/// What `Iterator.from(x)` makes a JS iterator of, `x`: `Ok` of it, or
/// `Err` of `e` itself, of anything else.
pub(super) fn from_iterator(e: Expr) -> Result<Expr, Box<Expr>> {
    let from = iterator_from(&e).is_some();
    match e.kind {
        js::ExprKind::Call(_, mut args) if from => Ok(args.pop().expect("one")),
        kind => Err(Box::new(Expr {
            facts: its_facts(e.facts),
            kind,
            span: e.span,
            held_as: Default::default(),
        })),
    }
}

/// What a `rev()` made in place is, as `rev_items` and `reversed_inward`
/// make it: the one reader of its form (ADRs 0986, 0987).
pub(crate) enum Reversal<'e> {
    /// `$rev(it)`, which steps `it` from its back: `it`.
    Stepped(&'e Expr),
    /// A range stepped from its end, `$iterator(range, $rangeNextBack)`:
    /// the range.
    Range(&'e Expr),
    /// An array's items reversed where they're made, `$iter(xs.toReversed())`,
    /// which knows its back and its length.
    Items,
    /// A map or a filter of them, `$iter(xs.toReversed()).map(f)`: what its
    /// source's is of the items reversed, read as its source's type is. Not
    /// a chain of a range stepped from its end, which as its source's would
    /// step forward.
    Chain,
}

/// `e`'s reversal, of a `rev()` made in place whose source is a range
/// stepped from its end by `range_back`, `$rangeNextBack`, or not a range.
pub(crate) fn reversal<'e>(e: &'e Expr, range_back: Option<&str>) -> Option<Reversal<'e>> {
    fn of_items(e: &Expr) -> bool {
        match &e.kind {
            _ if is_stepped_items(e) => true,
            js::ExprKind::Call(callee, args)
                if let js::ExprKind::Member(base, method) = &callee.kind
                    && matches!(method.as_str(), "map" | "filter")
                    && args.len() == 1 =>
            {
                of_items(base)
            }
            _ => false,
        }
    }
    let js::ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    if let Some(it) = reversed_of(e) {
        return Some(Reversal::Stepped(it));
    }
    if let Some([range, step]) = iterator_parts(e)
        && range_back.is_some()
        && step.helper_name() == range_back
    {
        return Some(Reversal::Range(range));
    }
    match (callee.helper_name(), args.as_slice()) {
        _ if is_stepped_items(e) => Some(Reversal::Items),
        _ => of_items(e).then_some(Reversal::Chain),
    }
}

/// Of `it.step_by(n)`, made where it's stepped, `$stepBy(it, n)`: `it`
/// and `n`, a whole number from 1 up to `2^31`, which every step adds.
pub fn stepped_by(e: &Expr) -> Option<(&Expr, i128)> {
    let [it, n] = stepped_every_parts(e)? else {
        return None;
    };
    let n = n.as_int().filter(|&n| (1..=1 << 31).contains(&n))?;
    Some((it, n))
}

/// What `for (x of e)` steps through of `e`, `$iter(xs)`'s or
/// `Iterator.from(xs)`'s: `xs` itself. Of another, `e`, every fact its
/// makers said of it kept, as a `Form` fact is never forgotten (#2602),
/// though not the temporary that held it.
pub fn iterated(e: Expr) -> Expr {
    let from = iterator_from(&e).is_some();
    match e.kind {
        js::ExprKind::Call(_, args) if from || is_stepped_items(&e) => args.into_iter().next().expect("one"),
        kind => Expr {
            facts: e.facts,
            kind,
            span: e.span,
            held_as: Default::default(),
        },
    }
}

/// The keys `for (const key in object)` visits, `js::keys_in(object)`'s:
/// `$keysIn(object)`, a JS iterator of them, which a `for` loop of it is
/// that loop (ADR 0719).
pub(crate) fn keys_in(object: Expr) -> Expr {
    Expr::call(Expr::helper("$keysIn"), vec![object])
}

/// An array's items reversed, a copy of them: `items.toReversed()`, what a
/// `rev()` of them steps where it's made (ADR 0535).
pub(crate) fn reversed(items: Expr) -> Expr {
    Expr::call(Expr::member(items, "toReversed"), Vec::new())
}

/// What `e`, a `reversed` one, reverses: `items` of `items.toReversed()`.
/// The one reader of its form: a `for` that reads them from the end
/// instead, by index (ADR 0904).
pub fn reversed_source(e: &Expr) -> Option<&Expr> {
    let js::ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    match &callee.kind {
        js::ExprKind::Member(items, method) if method == "toReversed" && args.is_empty() => Some(items),
        _ => None,
    }
}

/// What `e`, a `keys_in` one, steps the keys of: `object` of
/// `$keysIn(object)`. The one reader of its form.
pub fn keys_in_source(e: &Expr) -> Option<&Expr> {
    match &e.kind {
        js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$keysIn") && args.len() == 1 => args.first(),
        _ => None,
    }
}

/// What `it`, an iterator made here of an array, `$iter(v)`, steps: `v`,
/// all of it left. Or `$iter(v, true)`, of what std hints no lower bound
/// of, whose hint is std's (#1215).
pub fn fresh_iter_source(it: &Expr) -> Option<&Expr> {
    match &it.kind {
        js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$iter") => match args.as_slice() {
            [items] => Some(items),
            [items, lowerless] if lowerless.kind == js::ExprKind::Bool(true) => Some(items),
            _ => None,
        },
        _ => None,
    }
}

/// Each `const it = $iter(items)` of a slice's `Iter` that's only stepped,
/// every read of it a `$next(it)` of an item that's never `undefined`,
/// the array's own iterator, `const it = items.values()`, each step
/// `it.next().value`, `undefined` at its end as `$next`'s is: React DOM's
/// Fizz runtime's stylesheet descriptor read part by part, with no helper
/// (rd270). Not of a name declared twice, which one read may be another's.
pub fn stepped_slices(body: &mut Vec<Stmt>) {
    let mut made = Vec::new();
    js::each_list(body, &mut |stmts| {
        for stmt in stmts.iter() {
            if let StmtKind::Const(name, e) = &stmt.kind
                && e.facts.has(js::Known::ArrayItems)
                && fresh_iter_source(e).is_some()
            {
                made.push(name.clone());
            }
        }
    });
    for name in made {
        let stepped = |e: &Expr| {
            matches!(&e.kind, js::ExprKind::Call(callee, args)
                if callee.helper_name() == Some("$next")
                    && matches!(args.as_slice(), [it] if matches!(&it.kind, js::ExprKind::Var(v) if *v == name)))
        };
        let (mut steps, mut declared) = (0, 0);
        js::each_expr(body, &mut |e| steps += usize::from(stepped(e)));
        js::each_list(body, &mut |stmts| {
            declared += stmts
                .iter()
                .filter(|s| matches!(&s.kind, StmtKind::Const(n, _) | StmtKind::Let(n, _) if *n == name))
                .count();
        });
        if declared != 1 || steps == 0 || js::mentions_of(body, &name) != steps {
            continue;
        }
        js::statement_lists(body, &mut |stmts| {
            for stmt in stmts.iter_mut() {
                if let StmtKind::Const(n, e) = &mut stmt.kind
                    && *n == name
                    && let Some(items) = fresh_iter_source(e).cloned()
                {
                    *e = Expr {
                        span: e.span,
                        ..Expr::builtin(items, "values", Vec::new())
                    };
                }
            }
        });
        js::each_expr_mut(body, &mut |e| {
            if stepped(e) {
                *e = Expr {
                    span: e.span,
                    ..js::next_value(Expr::var(&name))
                };
            }
        });
    }
}

/// `$iter(items)`, `stepped_items`'s, where no `FnCx` is.
pub(super) fn array_stepped(items: Expr) -> Expr {
    Expr::call(Expr::helper("$iter"), vec![items])
}

/// Whether `e`, of a `Rev`, an `Enumerate` or a `Map`, is what the
/// recognizers made of it in place, never std's struct, which one held is:
/// as its maker marked it (`Known::InPlace`, ADR 0871), or JS's own adapter
/// of a JS iterator, `it.map(f)`, which only the recognizers make, though
/// it carries no mark, a map's source asked on its own (`is_js_adapter`,
/// #1491). The one answer.
pub fn made_in_place(e: &Expr) -> bool {
    e.facts.has(js::Known::InPlace) || is_js_adapter(e)
}

/// Of `facts`, a value's, what's known of the value that what holds it
/// keeps on its name: its form's marks (`its_facts`), whether it's made in
/// place (`made_in_place`), and whether a binding made it, JS's own
/// iterator (`binding_made`), so `let modules = graph.modules(chunk);
/// modules.collect()` asks no hint, as upstream React's `Array.from(chunkModules)`.
pub(crate) fn held_marks(facts: js::Facts) -> js::Facts {
    [js::Known::InPlace, js::Known::BindingIterator]
        .into_iter()
        .filter(|&fact| facts.has(fact))
        .fold(its_facts(facts), js::Facts::with)
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `value`, of `ty`, is std's compiled struct, a compiled std
    /// iterator's (ADR 0561): not a `Rev`, an `Enumerate` or a `Map` made in
    /// place, as its maker marked it (`made_in_place`, #730), which is what
    /// the recognizers make of it, a JS iterator that closes as it ends (ADR
    /// 0871).
    pub fn compiled_struct(&self, ty: Ty<'tcx>, value: &Expr) -> bool {
        crate::recognition::is_compiled_std_iterator(self.tcx, ty)
            && !(crate::recognition::is_held_compiled(self.tcx, ty) && crate::iterators::made_in_place(value))
    }

    /// Which argument of a std call of `def_id`, given `arg_tys`, is an
    /// iterator it takes by value and drops as it's done: `Extend::extend`'s
    /// items, `v.extend(it)`'s, or the receiver of an `Iterator` method that
    /// consumes it, `unzip`'s or `partition`'s, where `mir_iter_method`
    /// lowers none of it and a std combinator does. `None` of another, or of
    /// one lent (#1120).
    // helper: asked by `mir` and the base alike, the base's since #2299 moved it from `mir::iter`
    pub fn consumed_at(&self, def_id: DefId, arg_tys: &[Ty<'tcx>]) -> Option<usize> {
        let tr = self.tcx.trait_of_assoc(def_id)?;
        let at = match crate::recognition::is_extend(self.tcx, tr) {
            true => 1,
            false if self.recognition().folds_what_it_consumes(def_id) => 0,
            false => return None,
        };
        arg_tys.get(at).filter(|ty| !ty.is_ref()).map(|_| at)
    }

    /// What `$holding(adapter, drop)`, `closing_function`'s, holds: its
    /// adapter, of its own type (#851).
    pub(crate) fn held_by_holding(e: &Expr) -> Option<&Expr> {
        match &e.kind {
            js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$holding") => args.first(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
