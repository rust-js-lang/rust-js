//! What the crate steps from its back (ADR 0545), by the MIR identity of
//! what holds a value, not its type, which another value of it need not
//! share (#851): each local, by its body, and each field a back step is
//! given (`stepped_locals`), its maker then making it knowing its back where
//! it's made, so one held in a variable before it's stepped, as a library's
//! drop flags make one, still steps from its back; what an adapter stepped
//! or asked so holds, a chain's sides, a zip's, a map's whose closure has a
//! destructor, made so too (#718, #790, #385). And the types whose question
//! is a type's (`back_stepped`): one a function gives behind an `impl`,
//! which its callers can't see to make knowing its back, one a library's
//! function is given or gives (#1024), and a generic function's own of its
//! type parameters, each of its callers' (#388). No type's set besides: the
//! lowering asks the value (`stepped_from_its_back`).

use crate::collections::{IndexMap, IndexSet};
use crate::lower::recognition::{
    Recognition, back_stepped_source, chained_sources, cloned_source, copied_source, counted_source, enumerated_source,
    enumerates_a_compiled_struct, filter_mapped_source, filtered_source, flattened_source, inspected_source,
    is_double_ended, is_exact_size, is_iter_adapter_def, mapped_source, reversed_source, stepped_by_source,
    zipped_sources,
};
use crate::lower::{Body, fn_def};
use rustc_hir::def::DefKind;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, Ty, TyCtxt};

/// What `all_bodies` step from their back: the types (`back_stepped`), of
/// them those whose question is a type's, each local by its body
/// (`stepped_locals`), and each `&mut` of a local (`borrows`).
pub(super) fn of_crate<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    all_bodies: &[&Body<'tcx>],
    (library, drops): (bool, impl Fn(Ty<'tcx>) -> bool + Copy),
) -> BackStepped<'tcx> {
    let mut typed = back_stepped(tcx, recognition, all_bodies, (library, drops));
    let (locals, fields, typed_sources) = stepped_locals(tcx, recognition, all_bodies, drops);
    typed.extend(typed_sources);
    BackStepped {
        typed,
        borrows: borrows(all_bodies),
        locals,
        fields,
    }
}

/// The types whose question is a type's, not a value's (#851), regions
/// erased: one a function gives behind an `impl`, which its callers can't
/// see to make knowing its back, so each value of it its maker makes is;
/// one a library's function is given or gives (#1024); and each type a
/// generic function's that steps it from its back, of its callers' (#388).
pub(super) fn back_stepped<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    all_bodies: &[&Body<'tcx>],
    (library, drops): (bool, impl Fn(Ty<'tcx>) -> bool),
) -> IndexSet<Ty<'tcx>> {
    let mut typed: IndexSet<Ty<'tcx>> = IndexSet::default();
    // Each body's own, by its function: what a caller's call of it makes too.
    let mut of_body: IndexMap<rustc_span::def_id::DefId, IndexSet<Ty<'tcx>>> = IndexMap::default();
    for body in all_bodies {
        let stepped = of_body.entry(body.def_id).or_default();
        for expr in body.thir.exprs.iter() {
            let (ExprKind::ZstLiteral { .. }, Some((callee, args))) = (&expr.kind, fn_def(expr.ty)) else {
                continue;
            };
            // A library's function given a std adapter of a type parameter
            // its clauses make double-ended, `back<I, P>(it: Filter<I, P>)`
            // of an `I: DoubleEndedIterator`: its body, which may step it
            // from its back, is the library's, which the crate can't see, so
            // what it's given is made knowing its back where it's made
            // (#1024).
            if recognition.foreign.is_library(callee.krate) {
                typed.extend(library_backs(tcx, recognition, callee, args, &drops));
            }
            // `DoubleEndedIterator`'s own method, `next_back()`, which asks
            // it of what it's called on: below, of each adapter.
            let own_method = tcx.trait_of_assoc(callee).is_some_and(|tr| is_double_ended(tcx, tr));
            // What it asks to be double-ended holds such a type, as an
            // `Either` of filters does, stepped from its back by the impl
            // that asks it of what it holds; the type asked itself is its
            // call's own, below or at its `rev()`.
            for (clause, _) in tcx.clauses_of(callee).instantiate(tcx, args) {
                let Some(asked) = clause.skip_normalization().as_trait_clause() else {
                    continue;
                };
                if !is_double_ended(tcx, asked.def_id()) {
                    continue;
                }
                let hidden = asked.self_ty().skip_binder().peel_refs();
                let asked = recognition.reveal(hidden);
                // One a function gives behind an `impl`, which its caller
                // can't make knowing its back: made so where it's made.
                let given = hidden != asked;
                // Not through a std adapter, whose own call steps what it
                // holds: `filter(p).map(f).rev()`'s is `rev()`'s.
                for held in asked.walk().filter_map(|part| part.as_type()) {
                    if matches!(held.kind(), ty::Adt(adapter, _) if is_iter_adapter_def(tcx, adapter.did()))
                        // The type asked itself too where it's of an array's
                        // items with a destructor, which `rev()` can't step
                        // from its back once it's held in a variable.
                        && (held != asked || given || dropping_items(recognition, held, &drops))
                    {
                        let made = back_stepped_adapter(tcx, recognition, held, given, &drops);
                        if given {
                            typed.extend(made);
                        }
                        stepped.extend(made);
                    }
                }
            }
            // What its own method is called on: given behind an `impl`, each
            // adapter in it; else its own, which a generic function's, of
            // its type parameters, is each of its callers' (#388).
            if own_method && let Some(hidden) = args.types().next().map(|ty| ty.peel_refs()) {
                let adapter = recognition.reveal(hidden);
                match hidden != adapter {
                    true => typed.extend(given_behind_an_impl(tcx, recognition, adapter, &drops)),
                    false => stepped.extend(back_stepped_adapter(tcx, recognition, adapter, false, &drops)),
                }
            }
        }
    }
    // A library's function its consumers call, which gives what's behind
    // an `impl DoubleEndedIterator`: they step it from its back where the
    // library can't see, so each adapter in it is made knowing its back
    // where it's made, as one its own callers step is (#1024).
    if library {
        for body in all_bodies {
            let Some(local) = body.def_id.as_local() else { continue };
            if !matches!(tcx.def_kind(body.def_id), DefKind::Fn | DefKind::AssocFn)
                || !tcx.effective_visibilities(()).is_reachable(local)
            {
                continue;
            }
            // What its consumers give it, a std adapter of a double-ended
            // source, which it may step from its back: each made knowing it,
            // by their makers as by its body (`library_backs`).
            let identity = ty::GenericArgs::identity_for_item(tcx, body.def_id);
            typed.extend(library_backs(tcx, recognition, body.def_id, identity, &drops));
            // Its regions erased, those it binds too.
            let output = tcx
                .instantiate_bound_regions_with_erased(
                    tcx.fn_sig(body.def_id).instantiate_identity().skip_normalization(),
                )
                .output();
            for hidden in output.walk().filter_map(|part| part.as_type()) {
                let ty::Alias(
                    _,
                    ty::AliasTy {
                        kind: ty::Opaque { def_id },
                        ..
                    },
                ) = *hidden.kind()
                else {
                    continue;
                };
                let asked = (tcx.item_bounds(def_id).skip_binder().iter()).any(|clause| {
                    clause
                        .as_trait_clause()
                        .is_some_and(|bound| is_double_ended(tcx, bound.def_id()))
                });
                if asked {
                    typed.extend(given_behind_an_impl(
                        tcx,
                        recognition,
                        recognition.reveal(hidden),
                        &drops,
                    ));
                }
            }
        }
    }
    // And of a generic function's type parameters, `Filter<I, P>` a function
    // steps from its back that it's given: each of its callers' types,
    // `Filter<Copied<Iter<u32>>, {closure}>`, which its caller makes, through
    // what calls it in turn (#388). At most 32 calls deep, as a function
    // that calls itself with a type of its own types would go on.
    for _ in 0..32 {
        let mut more = Vec::new();
        for body in all_bodies {
            for expr in body.thir.exprs.iter() {
                let (ExprKind::ZstLiteral { .. }, Some((callee, args))) = (&expr.kind, fn_def(expr.ty)) else {
                    continue;
                };
                let Some(own) = of_body.get(&callee) else { continue };
                for &ty in own
                    .iter()
                    .filter(|ty| rustc_middle::ty::TypeVisitableExt::has_param(*ty))
                {
                    let made = ty::EarlyBinder::bind(tcx, ty)
                        .instantiate(tcx, args)
                        .skip_normalization();
                    let made = tcx.erase_and_anonymize_regions(made);
                    // Of the caller's own, which a type of its parameters may
                    // look like the callee's, `Filter<I, P>` of both.
                    if !of_body.get(&body.def_id).is_some_and(|known| known.contains(&made)) {
                        more.push((body.def_id, made));
                    }
                }
            }
        }
        if more.is_empty() {
            break;
        }
        for (owner, made) in more {
            typed.insert(made);
            of_body.entry(owner).or_default().insert(made);
        }
    }
    with_stepped_sources(tcx, typed)
}

/// `typed`, and the source of each of them that steps from its back where
/// its source does, `back_stepped_source`'s, a `Filter`'s of a `Copied`
/// given to generic code that steps it from its back: a type's question
/// too, made knowing its back where it's made (#388, #851).
fn with_stepped_sources<'tcx>(tcx: TyCtxt<'tcx>, typed: IndexSet<Ty<'tcx>>) -> IndexSet<Ty<'tcx>> {
    let mut all = typed.clone();
    for ty in typed {
        all.extend((back_stepped_source(tcx, ty)).map(|held| tcx.erase_and_anonymize_regions(held.peel_refs())));
    }
    all
}

/// Of a library's function `callee`, called with `args`, each std adapter
/// a parameter of it is, of a source a type parameter its clauses bound by
/// `DoubleEndedIterator`, which its body may step from its back: as it's
/// given, made knowing its back where it's made, regions erased.
fn library_backs<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    callee: rustc_span::def_id::DefId,
    args: ty::GenericArgsRef<'tcx>,
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> Vec<Ty<'tcx>> {
    let double_ended: Vec<Ty<'tcx>> = (tcx.clauses_of(callee).instantiate_identity(tcx).into_iter())
        .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
        .filter(|bound| is_double_ended(tcx, bound.def_id()))
        .map(|bound| bound.self_ty().skip_binder())
        .collect();
    let inputs =
        tcx.instantiate_bound_regions_with_erased(tcx.fn_sig(callee).instantiate_identity().skip_normalization());
    (inputs
        .inputs()
        .iter()
        .flat_map(|input| input.walk().filter_map(|part| part.as_type())))
    .filter(|held| {
        matches!(held.kind(), ty::Adt(a, _) if is_iter_adapter_def(tcx, a.did()))
            && back_stepped_source(tcx, *held).is_some_and(|source| double_ended.contains(&source))
    })
    .filter_map(|held| {
        let given = ty::EarlyBinder::bind(tcx, held)
            .instantiate(tcx, args)
            .skip_normalization();
        back_stepped_adapter(tcx, recognition, recognition.reveal(given), true, &drops)
    })
    .collect()
}

/// Each adapter in `adapter`, given behind an `impl` its callers step from
/// its back, that's made knowing its back where it's made, regions erased
/// (`back_stepped_adapter`'s, of one given).
fn given_behind_an_impl<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    adapter: Ty<'tcx>,
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> Vec<Ty<'tcx>> {
    (adapter.walk().filter_map(|part| part.as_type()))
        .filter_map(|held| {
            matches!(held.kind(), ty::Adt(a, _) if is_iter_adapter_def(tcx, a.did()))
                .then(|| back_stepped_adapter(tcx, recognition, held, true, &drops))
                .flatten()
        })
        .collect()
}

/// `adapter`, regions erased, where it's a map or a filter of a source not
/// an array's, a filter of an array's items, which no form of its own
/// steps from its back (ADR 0990), a map of an array's items whose drop
/// runs, `drops`'s, or one that a function gives behind an `impl`,
/// `given`, which no form of its caller's steps.
fn back_stepped_adapter<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    adapter: Ty<'tcx>,
    given: bool,
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> Option<Ty<'tcx>> {
    // A flatten, of whatever source: its own forms never step from the back.
    // A chain a function gives behind an `impl` is std's `Chain`, which
    // steps from its back by its own methods (ADR 0919).
    if flattened_source(tcx, adapter).is_some() {
        return Some(tcx.erase_and_anonymize_regions(adapter));
    }
    // A `rev()`, whose forms, a range stepped from its end or an array's
    // items' chain reversed, step only forward: made knowing its back where
    // it's made, as MIR may hold it in a variable before its `rfold` or
    // `nth_back`, past a call, a cleanup that drops it, or a `&mut` (#702).
    if reversed_source(tcx, adapter).is_some() {
        return Some(tcx.erase_and_anonymize_regions(adapter));
    }
    // A zip, whose own forms, `$lazyZip` and `$trustedZip`, step only
    // forward: made knowing its back where it's made, `$zipEnds`, as MIR
    // holds it in a variable before its `next_back` (#740).
    if zipped_sources(tcx, adapter).is_some() {
        return Some(tcx.erase_and_anonymize_regions(adapter));
    }
    // An enumeration of items with a destructor, held in a variable to drop
    // them, whose own back the crate steps (#383): made knowing its back
    // where it's made, its source too, the crate's own iterator under a
    // `rev()`, a `skip(n)`, a `take(n)` or a `step_by(n)`, whose form there
    // steps only forward, `$iterator(it, nextBack)` with no back (#823). And
    // of a map or a filter of an array's items with a destructor, held to
    // drop them whatever it gives (#764). One of std's struct, `$enumerate`
    // of it, is its value's, `unread` (#712, #851).
    if let Some(source) = enumerated_source(tcx, adapter) {
        let item = recognition.iterator_item(adapter);
        return (item.is_some_and(&drops) || dropping_items(recognition, source.peel_refs(), &drops))
            .then(|| tcx.erase_and_anonymize_regions(adapter));
    }
    // An `inspect(f)` is made as a map, stepped from its back as one is.
    let mapped = mapped_source(tcx, adapter).or_else(|| inspected_source(tcx, adapter));
    // A `cloned()` too, made as a copy or as a map of each item's clone.
    let copied = copied_source(tcx, adapter).or_else(|| cloned_source(tcx, adapter));
    // A `filter_map`, made as a map filtered, steps from its back as a
    // filter does, by no form of its own (#473).
    let filtered = filtered_source(tcx, adapter).or_else(|| filter_mapped_source(tcx, adapter));
    let source = (mapped.or(filtered).or(copied))?.peel_refs();
    // A map of an array's items steps from its back by its own form, its
    // source's last item mapped, and a copy of them by its items'; a
    // filter's has none. A copy of another, a type parameter's, `Copied<I>`
    // generic code steps from its back, made knowing it (#385).
    let native = (mapped.is_some() || copied.is_some())
        && !given
        && recognition.array_source(source).is_some()
        && !dropping_items(recognition, adapter, &drops)
        && !closure_drops(tcx, adapter, &drops);
    (!native).then(|| tcx.erase_and_anonymize_regions(adapter))
}

/// Whether `adapter` is a map whose closure has a destructor, `drops`'s:
/// closed to drop it and held in a variable, so no form of its own steps
/// it from its back or counts it (#385). An `inspect(f)`'s too, made as a
/// map, `F` its second type as `Map`'s is (#780).
fn closure_drops<'tcx>(tcx: TyCtxt<'tcx>, adapter: Ty<'tcx>, drops: impl Fn(Ty<'tcx>) -> bool) -> bool {
    (mapped_source(tcx, adapter).is_some() || inspected_source(tcx, adapter).is_some())
        && matches!(adapter.kind(), ty::Adt(_, args) if drops(args.type_at(1)))
}

/// Whether `adapter`, a map, an inspect or a filter, is of an array's
/// items whose drop runs, `drops`'s: an owning source closed to drop them
/// (ADR 0465), or a `rev()` of one, which steps them from the other end
/// (#646). Or a zip with a side of such items, held in a variable as it's
/// made, std's `Zip` folding otherwise than it steps made in place where
/// it's stepped from its back (#812).
fn dropping_items<'tcx>(
    recognition: Recognition<'_, 'tcx>,
    adapter: Ty<'tcx>,
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> bool {
    let tcx = recognition.tcx;
    let unreversed = |mut source: Ty<'tcx>| {
        while let Some(reversed) = crate::lower::recognition::reversed_source(tcx, source) {
            source = reversed.peel_refs();
        }
        source
    };
    let array_of_drops = |source: Ty<'tcx>| {
        recognition.array_source(source).is_some() && recognition.iterator_item(source).is_some_and(&drops)
    };
    if let Some((a, b)) = zipped_sources(tcx, adapter) {
        return array_of_drops(a.peel_refs()) || array_of_drops(b.peel_refs());
    }
    // An `inspect(f)`'s too, made as a map (#751).
    mapped_source(tcx, adapter)
        .or_else(|| inspected_source(tcx, adapter))
        .or_else(|| filtered_source(tcx, adapter))
        .map(|source| unreversed(source.peel_refs()))
        .is_some_and(array_of_drops)
}

/// What the crate steps from its back: each local, by its body, and each
/// field, `stepped_locals`' (#814, #851), by the value its maker makes,
/// made knowing its back where it's made (ADR 0545); and the types whose
/// question is a type's.
pub(in crate::lower) struct BackStepped<'tcx> {
    /// The types whose question is a type's (`back_stepped`'s second):
    /// asked of every value of one, never of a value alone. The other types
    /// `back_stepped` gives seed `stepped_locals` alone, what a value made
    /// of what's stepped is made of (#851).
    pub(in crate::lower) typed: IndexSet<Ty<'tcx>>,
    /// Each `&mut` of a whole local, the local it lends (`borrows`).
    pub(in crate::lower) borrows:
        IndexMap<(rustc_span::def_id::DefId, rustc_middle::mir::Local), rustc_middle::mir::Local>,
    pub(in crate::lower) locals: IndexMap<(rustc_span::def_id::DefId, rustc_middle::mir::Local), SteppedLocal>,
    /// Each field of a struct or an enum's variant a back step is given,
    /// each value MIR moves into it marked `Given` (`stepped_locals`'s): a
    /// read of it is of a value made knowing its back (#851).
    pub(in crate::lower) fields: IndexSet<SteppedField>,
}

/// The body that makes what a call of `callee` with `args` gives: its own,
/// or of a closure called through its `Fn` trait, the closure's (#851).
pub(in crate::lower) fn made_by<'tcx>(
    tcx: TyCtxt<'tcx>,
    callee: rustc_span::def_id::DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> rustc_span::def_id::DefId {
    match args.types().next().map(|t| *t.kind()) {
        Some(ty::Closure(closure, _)) if tcx.fn_trait_kind_from_def_id(tcx.parent(callee)).is_some() => closure,
        _ => callee,
    }
}

/// What `stepped_locals` finds: each local stepped from its back, each
/// field, and the types a std struct holding them asks by type alone.
type SteppedValues<'tcx> = (
    IndexMap<(rustc_span::def_id::DefId, rustc_middle::mir::Local), SteppedLocal>,
    IndexSet<SteppedField>,
    IndexSet<Ty<'tcx>>,
);

/// A field of a struct or an enum's variant: its ADT, its variant and its
/// index.
pub(in crate::lower) type SteppedField = (rustc_span::def_id::DefId, rustc_abi::VariantIdx, rustc_abi::FieldIdx);

/// How a local the crate steps from its back holds its value
/// (`stepped_locals`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum SteppedLocal {
    /// A temporary given to what steps it, lent, `nth_back(&mut _11)`'s,
    /// or whole, `rfold(move _11, ..)`'s, which MIR holds in a variable to
    /// lend it or while an argument's statement runs: its value made in
    /// place, by its form, is made knowing its back where it's written to
    /// one (#718, #811).
    Lent,
    /// A value given to what asks its back, held in a variable or a field
    /// where it's given, a chain's side, what an adapter stepped from its
    /// back holds, or a function's return place: made knowing its back by
    /// its maker (#718, #790).
    Given,
}

/// Each local, by its body, whose value the crate steps from its back, and
/// how it holds it (#814): a temporary lent to what asks its back, a
/// `DoubleEndedIterator` clause of the callee's of its type, `nth_back(&mut
/// _11)`'s; a variable's value given whole to it, `rev()`'s or generic
/// code's, a temporary moved out of one too; what an adapter that steps what
/// it's given from its back holds, where it's stepped from its own or asked
/// to be, a map's, a chain's sides, a zip's, a map's whose closure has a
/// destructor (#851); and a
/// function's return place, where a caller's destination is one. Not a
/// temporary given whole, made in place where it's stepped, whose form the
/// step reads, `v.slice(2).toReversed()`; nor a variable lent, std's struct,
/// whose compiled back the step asks. By the MIR identity of what holds a
/// value, not its type, which another value of it need not share.
pub(super) fn stepped_locals<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    all_bodies: &[&Body<'tcx>],
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> SteppedValues<'tcx> {
    use rustc_middle::mir;
    let erased = |ty: Ty<'tcx>| tcx.erase_and_anonymize_regions(recognition.reveal(ty.peel_refs()));
    let zip = crate::lower::recognition::compiled_provided_method(tcx, "Iterator", "zip");
    let rev = crate::lower::recognition::compiled_provided_method(tcx, "Iterator", "rev");
    let Some(double_ended) = recognition.double_ended_iterator() else {
        return Default::default();
    };
    let mut stepped: IndexMap<(rustc_span::def_id::DefId, mir::Local), SteppedLocal> = IndexMap::default();
    // Each temporary given whole to what asks its back, made in place,
    // whose form its step reads: not made knowing it itself, but what it
    // holds beneath is, a variable's value an adapter made in place holds
    // (#790).
    let mut asked: IndexSet<(rustc_span::def_id::DefId, mir::Local)> = IndexSet::default();
    let mut held_whole: IndexSet<(rustc_span::def_id::DefId, mir::Local)> = IndexSet::default();
    // Each zip a side of which MIR holds in a variable before it's made, no
    // form of the side left to count in place.
    let mut zips_of_held: IndexSet<(rustc_span::def_id::DefId, mir::Local)> = IndexSet::default();
    // Each field of a struct or an enum's variant a back step is given,
    // lent or whole, `self.it.next_back()` of `struct Evens { it: Filter<..> }`:
    // each value MIR moves into it as it makes one, made knowing its back by
    // its maker (#851).
    let mut stepped_fields: IndexSet<SteppedField> = IndexSet::default();
    let mut typed_sources: IndexSet<Ty<'tcx>> = IndexSet::default();
    let mark = |stepped: &mut IndexMap<_, _>, at, how| {
        // `Given` wins: its maker makes it, wherever it's lent besides.
        let held = stepped.entry(at).or_insert(how);
        if how == SteppedLocal::Given {
            *held = how;
        }
    };
    loop {
        let before = (
            stepped.len(),
            stepped.values().filter(|h| **h == SteppedLocal::Given).count(),
            asked.len(),
            stepped_fields.len(),
            zips_of_held.len(),
        );
        for body in all_bodies {
            let Some(mir) = body.mir.as_ref() else { continue };
            let mir = &mir.body;
            let owner = body.def_id;
            // Each whole `&mut` of a whole local, `_12 = &mut _11`, and each
            // move of one into another, `_5 = move _4`.
            let mut lent: IndexMap<mir::Local, mir::Local> = IndexMap::default();
            let mut moved: IndexMap<mir::Local, mir::Local> = IndexMap::default();
            // Where each `&mut` is taken, and where each call's value is
            // made, its call's return: what's held from there to its step.
            let mut lent_at: IndexMap<mir::Local, (mir::BasicBlock, usize)> = IndexMap::default();
            // Each `&mut` of a field, `_3 = &mut ((*_1).0)`, and each value
            // moved into a field as a struct is made.
            let mut lent_field: IndexMap<mir::Local, mir::Place<'tcx>> = IndexMap::default();
            let mut into_fields: Vec<(
                (rustc_span::def_id::DefId, rustc_abi::VariantIdx, rustc_abi::FieldIdx),
                mir::Local,
            )> = Vec::new();
            let mut made_at: IndexMap<mir::Local, (mir::BasicBlock, usize)> = IndexMap::default();
            for (block, data) in mir.basic_blocks.iter_enumerated() {
                if let mir::TerminatorKind::Call {
                    destination,
                    target: Some(target),
                    ..
                } = &data.terminator().kind
                    && destination.projection.is_empty()
                {
                    made_at.insert(destination.local, (*target, 0));
                }
                for (at, statement) in data.statements.iter().enumerate() {
                    let mir::StatementKind::Assign(assign) = &statement.kind else {
                        continue;
                    };
                    let (to, rvalue) = &**assign;
                    if !to.projection.is_empty() {
                        continue;
                    }
                    match rvalue {
                        mir::Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, of) if of.projection.is_empty() => {
                            lent.insert(to.local, of.local);
                            lent_at.insert(to.local, (block, at + 1));
                        }
                        // A shared one too, `len(&self)`'s, which reads it
                        // where it's lent as a `&mut` step does (#851).
                        mir::Rvalue::Ref(_, mir::BorrowKind::Shared, of) if of.projection.is_empty() => {
                            lent.insert(to.local, of.local);
                            lent_at.insert(to.local, (block, at + 1));
                        }
                        // A reborrow of one, `&mut (*_r)` of an `_r = &mut x`:
                        // what that lends.
                        mir::Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, of)
                            if matches!(of.projection.as_slice(), [mir::ProjectionElem::Deref])
                                && let Some(&lends) = lent.get(&of.local) =>
                        {
                            lent.insert(to.local, lends);
                            lent_at.insert(to.local, (block, at + 1));
                        }
                        // And of a field's, what that lends (#1013).
                        mir::Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, of)
                            if matches!(of.projection.as_slice(), [mir::ProjectionElem::Deref])
                                && let Some(&field) = lent_field.get(&of.local) =>
                        {
                            lent_field.insert(to.local, field);
                        }
                        mir::Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, of) => {
                            lent_field.insert(to.local, *of);
                        }
                        mir::Rvalue::Aggregate(kind, operands) => {
                            if let mir::AggregateKind::Adt(adt, variant, ..) = **kind {
                                for (field, operand) in operands.iter_enumerated() {
                                    if let mir::Operand::Move(of) | mir::Operand::Copy(of) = operand
                                        && let Some(local) = of.as_local()
                                    {
                                        into_fields.push(((adt, variant, field), local));
                                    }
                                }
                            }
                        }
                        mir::Rvalue::Use(mir::Operand::Move(of) | mir::Operand::Copy(of), ..)
                            if of.projection.is_empty() =>
                        {
                            moved.insert(to.local, of.local);
                        }
                        _ => {}
                    }
                }
            }
            // What a local was moved out of, through each move: a variable's
            // value a temporary was given to give on.
            let origin = |mut local: mir::Local| {
                for _ in 0..8 {
                    match moved.get(&local) {
                        Some(&of) => local = of,
                        None => break,
                    }
                }
                local
            };
            for &(field, local) in &into_fields {
                if stepped_fields.contains(&field) {
                    mark(&mut stepped, (owner, origin(local)), SteppedLocal::Given);
                }
            }
            for (&to, &of) in &moved {
                if let Some(&how) = stepped.get(&(owner, to)) {
                    mark(&mut stepped, (owner, of), how);
                }
                if asked.contains(&(owner, to)) {
                    asked.insert((owner, of));
                }
            }
            for (block, data) in mir.basic_blocks.iter_enumerated() {
                let mir::TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    fn_span,
                    ..
                } = &data.terminator().kind
                else {
                    continue;
                };
                let Some((callee, generic_args)) = func.const_fn_def() else {
                    continue;
                };
                // A `rev()` held, std's compiled `Rev`, which steps what it's
                // given by its `next_back`, not one a std method's form
                // reverses where it's made (ADR 0871): what it's given made
                // knowing its back (#851).
                if Some(callee) == rev
                    && !body.facts.in_place_adapters.contains(fn_span)
                    && let Some(arg) = args.first()
                    && let mir::Operand::Move(place) | mir::Operand::Copy(place) = &arg.node
                    && let Some(local) = place.as_local()
                {
                    mark(&mut stepped, (owner, origin(local)), SteppedLocal::Given);
                }
                // A zip that reads its sides by index (ADR 0973), which asks
                // the length of `b`, a map whose closure has a destructor:
                // made knowing its length where it's made, as `back_stepped`'s
                // rule of its type says (#775).
                if Some(callee) == zip
                    && let [a, b] = generic_args.types().collect::<Vec<_>>()[..]
                    && !rustc_middle::ty::TypeVisitableExt::has_param(&(a, b))
                    && let [_, arg] = &args[..]
                    && let mir::Operand::Move(place) | mir::Operand::Copy(place) = &arg.node
                    && let Some(local) = place.as_local()
                {
                    let typing_env = ty::TypingEnv::post_analysis(tcx, tcx.typeck_root_def_id(owner));
                    let b = recognition.reveal(crate::lower::recognition::iterator_type_of(tcx, typing_env, b));
                    if crate::lower::recognition::zips_by_index(tcx, typing_env, a, b) && closure_drops(tcx, b, &drops)
                    {
                        let local = origin(local);
                        let how = match mir.local_decls[local].is_user_variable() {
                            true => SteppedLocal::Given,
                            false => SteppedLocal::Lent,
                        };
                        mark(&mut stepped, (owner, local), how);
                    }
                }
                // Each local given, lent or whole, its type, and whether MIR
                // holds it across what acts before the call.
                let given: Vec<(mir::Local, bool, Ty<'tcx>, bool)> = (args.iter())
                    .filter_map(|arg| match &arg.node {
                        mir::Operand::Move(place) | mir::Operand::Copy(place) if place.projection.is_empty() => {
                            let (local, lent, from) = match lent.get(&place.local) {
                                Some(&of) => (of, true, lent_at.get(&place.local)),
                                None => (origin(place.local), false, made_at.get(&origin(place.local))),
                            };
                            let held = from.is_some_and(|&from| held_across(mir, from, block));
                            Some((local, lent, erased(mir.local_decls[local].ty), held))
                        }
                        _ => None,
                    })
                    .collect();
                // A value given beside a `&mut` of a local stepped from its
                // back, of its type, which the callee may write there,
                // `replace_in(&mut it, next)`, its caller's own as one
                // representation (#537): stepped so too (#851).
                // Beside a `&mut` of a field stepped from its back too,
                // `replaced(&mut held.0, next)` (#1013).
                let lent_stepped_fields = (args.iter()).filter_map(|arg| match &arg.node {
                    mir::Operand::Move(place) | mir::Operand::Copy(place) => {
                        let field = *lent_field.get(&place.as_local()?)?;
                        (field_of(tcx, mir, field).is_some_and(|of| stepped_fields.contains(&of)))
                            .then(|| (None, erased(field.ty(&mir.local_decls, tcx).ty)))
                    }
                    _ => None,
                });
                let lent_stepped_locals = (given.iter())
                    .filter(|&&(local, lent, ..)| lent && stepped.contains_key(&(owner, local)))
                    .map(|&(local, _, ty, _)| (Some(local), ty));
                let beside: Vec<(Option<mir::Local>, Ty<'tcx>)> =
                    lent_stepped_locals.chain(lent_stepped_fields).collect();
                for (local, ty) in beside {
                    for &(other, other_lent, other_ty, _) in &given {
                        if !other_lent && Some(other) != local && other_ty == ty {
                            mark(&mut stepped, (owner, other), SteppedLocal::Given);
                        }
                    }
                }
                // What asks its back: a clause of the callee's.
                for (clause, _) in tcx.clauses_of(callee).instantiate(tcx, generic_args) {
                    let Some(clause) = clause.skip_normalization().as_trait_clause() else {
                        continue;
                    };
                    let asked_ty = erased(clause.self_ty().skip_binder());
                    // Or a zip's length, of one a side of which MIR holds, no
                    // form left to count it by in place: made knowing its back
                    // and length, `$zipEnds`, the one form that knows it
                    // (#740, #851). Or a map's whose closure has a destructor,
                    // held to drop it, whose form no use reads: made knowing
                    // both, as the set's rule of its type says (#385, #851).
                    let length = is_exact_size(tcx, clause.def_id());
                    let zip_length = length && zipped_sources(tcx, asked_ty).is_some();
                    let map_length = length && closure_drops(tcx, asked_ty, &drops);
                    if clause.def_id() != double_ended && !zip_length && !map_length {
                        continue;
                    }
                    // A field it's given, lent or whole, of the type asked.
                    for arg in args.iter() {
                        let (mir::Operand::Move(place) | mir::Operand::Copy(place)) = &arg.node else {
                            continue;
                        };
                        // Through a `&mut` of it too, `next_back(&mut r)` of
                        // an `r = it.by_ref()`.
                        let place = match place.as_local().and_then(|local| {
                            (lent_field.get(&local)).or_else(|| lent.get(&local).and_then(|of| lent_field.get(of)))
                        }) {
                            Some(lent) => *lent,
                            None => *place,
                        };
                        if let Some(field) = field_of(tcx, mir, place)
                            && erased(place.ty(&mir.local_decls, tcx).ty) == asked_ty
                        {
                            stepped_fields.insert(field);
                        }
                    }
                    for &(local, lent, ty, held) in &given {
                        let user = mir.local_decls[local].is_user_variable();
                        // A counted adapter lent, whose form no step reads
                        // where MIR holds it (#718); another held across what
                        // acts before its step, an argument's statement,
                        // `nth_back({ k += 1; k - 2 })`, which makes it
                        // first, its form gone where the step reads it (#811).
                        let counted = counted_source(tcx, ty).is_some() || stepped_by_source(tcx, ty).is_some();
                        // Or, as back_stepped's set says of its type: the
                        // receiver of `DoubleEndedIterator`'s own method, of a
                        // kind no form of whose own a step reads from its back
                        // where it's made, a `rev()`'s, a zip's, a flatten's or
                        // a map of what's no array's (`back_stepped_adapter`'s);
                        // or of an array's items or a closure with a
                        // destructor, or an enumeration of std's struct,
                        // whatever asks it. Not a map of an array's items an
                        // `rposition` asks, whose step reads its form (#851).
                        let own_method = tcx.trait_of_assoc(callee).is_some_and(|tr| is_double_ended(tcx, tr));
                        let receiver = given.first().is_some_and(|&(first, ..)| first == local);
                        let unread = || {
                            own_method
                                && receiver
                                && back_stepped_adapter(tcx, recognition, ty, false, &drops).is_some()
                                || dropping_items(recognition, ty, &drops)
                                || closure_drops(tcx, ty, &drops)
                                || enumerates_a_compiled_struct(tcx, ty)
                                // A chain of items with a destructor, which
                                // MIR holds to drop them before it's asked
                                // (#718).
                                || chained_sources(tcx, ty).is_some()
                                    && recognition.iterator_item(ty).is_some_and(&drops)
                        };
                        let held = held || unread();
                        match (ty == asked_ty, lent, user) {
                            (true, true, false)
                                if counted || held || zip_length && zips_of_held.contains(&(owner, local)) =>
                            {
                                mark(&mut stepped, (owner, local), SteppedLocal::Lent)
                            }
                            (true, false, true) => mark(&mut stepped, (owner, local), SteppedLocal::Given),
                            // A variable lent to its step, `f.next_back()` of a
                            // `let mut f = v.iter().filter(p)`: a JS form its
                            // maker makes knowing its back; not a counted
                            // adapter, std's struct where a variable holds
                            // it (#718, #851).
                            (true, true, true) if !counted => mark(&mut stepped, (owner, local), SteppedLocal::Given),
                            (true, false, false) => {
                                asked.insert((owner, local));
                                if held {
                                    held_whole.insert((owner, local));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                if !destination.projection.is_empty() {
                    continue;
                }
                let made = erased(destination.ty(&mir.local_decls, tcx).ty);
                // Not an array's iterator, which is counted as it is.
                if zipped_sources(tcx, made).is_some()
                    && (given.iter()).any(|&(_, _, ty, held)| held && recognition.array_source(ty).is_none())
                {
                    zips_of_held.insert((owner, destination.local));
                }
                let made_stepped = stepped.contains_key(&(owner, destination.local));
                let made_asked = made_stepped || asked.contains(&(owner, destination.local));
                if !made_asked {
                    continue;
                }
                // An adapter of what it's given, stepped from its back where
                // it is: a chain's sides, which `$lazyChain` steps from their
                // back as MIR holds side `a` in a variable before the chain is
                // made, no form of it left to read (#718); a variable another
                // holds beneath, held where it's given (#790). Not one made in
                // place beneath a map, a filter or a copy, whose maker makes
                // it knowing its back by its form.
                // A chain's sides or a zip's, which `$lazyChain` and `$zipEnds`
                // step from their back, MIR holding a side in a variable
                // before the adapter is made (#718, #792, #851).
                let chain = chained_sources(tcx, made).is_some() || zipped_sources(tcx, made).is_some();
                let sources: Vec<Ty<'tcx>> = match chained_sources(tcx, made).or_else(|| zipped_sources(tcx, made)) {
                    Some((a, b)) => vec![a, b],
                    None => back_stepped_source(tcx, made)
                        .or_else(|| counted_source(tcx, made).map(|(source, _)| source))
                        .or_else(|| stepped_by_source(tcx, made))
                        .or_else(|| enumerated_source(tcx, made))
                        .into_iter()
                        .collect(),
                };
                // Held by std's struct, which asks the dictionary of what it
                // holds by its type alone, `mapDoubleEndedIterator_nextBack`'s:
                // a question of the type (#851).
                let held_by_struct = crate::lower::recognition::is_held_compiled(tcx, made)
                    && mir.local_decls[destination.local].is_user_variable();
                for source in sources.into_iter().map(erased) {
                    for &(local, _, ty, _) in &given {
                        let user = mir.local_decls[local].is_user_variable();
                        // Or of an adapter stepped from its back that holds a
                        // closure with a destructor, which MIR holds what it's
                        // made of before it's made, its form gone (#851).
                        let spilled = made_asked && closure_drops(tcx, made, &drops);
                        let copy = copied_source(tcx, made).or_else(|| cloned_source(tcx, made)).is_some();
                        if ty == source && (chain && made_asked || user || spilled || copy && made_asked) {
                            mark(&mut stepped, (owner, local), SteppedLocal::Given);
                            if held_by_struct {
                                typed_sources.insert(ty);
                            }
                        } else if ty == source && made_asked && !user {
                            // What an asked adapter made in place is made of,
                            // in place beneath it, asked too, down to what a
                            // variable holds (#790).
                            asked.insert((owner, local));
                        }
                    }
                }
            }
        }
        // What a call gives that's stepped from its back, lent or given whole,
        // `make().rfold(..)`'s, made by the function called, the crate's own
        // or a closure called through its `Fn` trait: its return place,
        // which its maker makes it into or moves it to (#851).
        for body in all_bodies {
            let Some(mir) = body.mir.as_ref() else { continue };
            for data in mir.body.basic_blocks.iter() {
                let mir::TerminatorKind::Call { func, destination, .. } = &data.terminator().kind else {
                    continue;
                };
                let at = (body.def_id, destination.local);
                if !destination.projection.is_empty() || !stepped.contains_key(&at) && !asked.contains(&at) {
                    continue;
                }
                let Some((callee, generic_args)) = func.const_fn_def() else {
                    continue;
                };
                let made_by = made_by(tcx, callee, generic_args);
                if all_bodies.iter().any(|b| b.def_id == made_by) {
                    mark(&mut stepped, (made_by, mir::RETURN_PLACE), SteppedLocal::Given);
                }
            }
        }
        let given = stepped.values().filter(|h| **h == SteppedLocal::Given).count();
        if (
            stepped.len(),
            given,
            asked.len(),
            stepped_fields.len(),
            zips_of_held.len(),
        ) == before
        {
            // And each temporary given whole held across what acts before
            // its step, `map(f).rfold({ k += 1; k }, g)`'s, as one lent: its
            // step reads the form of one made in place (#811).
            for at in held_whole {
                stepped.entry(at).or_insert(SteppedLocal::Lent);
            }
            return (stepped, stepped_fields, typed_sources);
        }
    }
}

/// Whether `mir` holds what's made or lent at `from`, a block and the first
/// statement after it, across what acts before the call that ends `to`: a
/// write to a variable or a place, or a call, a switch or a drop between,
/// along the blocks MIR goes through (`Goto`, an overflow's `Assert`). Not
/// moves, borrows or arithmetic into temporaries, which its step's lowering
/// reads in place (#811).
fn held_across(
    mir: &rustc_middle::mir::Body<'_>,
    from: (rustc_middle::mir::BasicBlock, usize),
    to: rustc_middle::mir::BasicBlock,
) -> bool {
    use rustc_middle::mir;
    let acts = |statement: &mir::Statement<'_>| match &statement.kind {
        mir::StatementKind::Assign(assign) => {
            let (place, _) = &**assign;
            !place.projection.is_empty() || mir.local_decls[place.local].is_user_variable()
        }
        mir::StatementKind::SetDiscriminant { .. } | mir::StatementKind::Intrinsic(..) => true,
        _ => false,
    };
    let (mut block, mut first) = from;
    for _ in 0..64 {
        let data = &mir.basic_blocks[block];
        if data.statements.get(first..).is_some_and(|rest| rest.iter().any(acts)) {
            return true;
        }
        if block == to {
            return false;
        }
        block = match &data.terminator().kind {
            mir::TerminatorKind::Goto { target } | mir::TerminatorKind::Assert { target, .. } => *target,
            _ => return true,
        };
        first = 0;
    }
    true
}

/// The field `place` is, of a struct or of an enum's variant, a field
/// projection last: `(*_1).0`, `((_2 as Some).0)`.
pub(in crate::lower) fn field_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    mir: &rustc_middle::mir::Body<'tcx>,
    place: rustc_middle::mir::Place<'tcx>,
) -> Option<SteppedField> {
    use rustc_middle::mir::ProjectionElem;
    let (last, base) = place.projection.split_last()?;
    let ProjectionElem::Field(field, _) = last else {
        return None;
    };
    let (variant, base) = match base.split_last() {
        Some((ProjectionElem::Downcast(_, variant), rest)) => (*variant, rest),
        _ => (rustc_abi::FIRST_VARIANT, base),
    };
    let base_ty = rustc_middle::mir::Place::ty_from(place.local, base, &mir.local_decls, tcx).ty;
    match base_ty.kind() {
        ty::Adt(adt, _) => Some((adt.did(), variant, *field)),
        _ => None,
    }
}

/// Each `&mut` or `&` MIR lends a whole local by, `_3 = &mut _4`, by its body: the
/// local it lends, which a call given `_3` is given the value of, an
/// adapter made in place where it's stepped (#851). Not a fact of its own.
pub(super) fn borrows<'tcx>(
    all_bodies: &[&Body<'tcx>],
) -> IndexMap<(rustc_span::def_id::DefId, rustc_middle::mir::Local), rustc_middle::mir::Local> {
    use rustc_middle::mir;
    let mut out = IndexMap::default();
    for body in all_bodies {
        let Some(mir) = body.mir.as_ref() else { continue };
        for data in mir.body.basic_blocks.iter() {
            for statement in &data.statements {
                if let mir::StatementKind::Assign(assign) = &statement.kind
                    && let (to, mir::Rvalue::Ref(_, mir::BorrowKind::Mut { .. } | mir::BorrowKind::Shared, of)) =
                        &**assign
                    && to.projection.is_empty()
                    && of.projection.is_empty()
                {
                    out.insert((body.def_id, to.local), of.local);
                }
            }
        }
    }
    out
}
