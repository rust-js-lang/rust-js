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

mod locals;

pub use locals::SteppedLocal;
use locals::{borrows, stepped_locals};

use super::iterator_facts::typed_key::{TypedKey, typed_key};
use crate::analysis::mir_reads::user_variable::user_variable;
use crate::generic_facts::{GivenADrop, given_a_drop_of};
use crate::recognition::{
    Recognition, back_stepped_source, chained_sources, cloned_source, copied_source, counted_source, enumerated_source,
    enumerates_a_compiled_struct, filter_mapped_source, filtered_source, flattened_source, inspected_source,
    is_double_ended, is_exact_size, is_iter_adapter_def, mapped_source, reversed_source, stepped_by_source,
    zipped_sources,
};
use crate::{Body, fn_def};
use rust_js_program::collections::{IndexMap, IndexSet};
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
    (library, drops, keeps_capacity): (bool, impl Fn(Ty<'tcx>) -> bool + Copy, bool),
    given: (bool, Option<&GivenADrop<'_>>),
) -> BackStepped<'tcx> {
    let mut typed = back_stepped(tcx, recognition, all_bodies, (library, drops));
    let (locals, typed_sources, lengths) = stepped_locals(tcx, recognition, all_bodies, (drops, keeps_capacity), given);
    typed.extend(typed_sources);
    BackStepped {
        typed,
        borrows: borrows(all_bodies),
        locals,
        lengths,
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
) -> IndexSet<TypedKey<'tcx>> {
    let mut typed: IndexSet<TypedKey<'tcx>> = IndexSet::default();
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
                let backs = library_backs(tcx, recognition, callee, args, &drops);
                typed.extend(backs.into_iter().map(|ty| typed_key(tcx, body.def_id, ty)));
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
                // Not std's compiled struct a function gives, nor what holds
                // one, whose value is its maker's (#1226).
                let compiled = crate::recognition::compiled_behind_an_impl(tcx, recognition.typing_env, hidden);
                // Not through a std adapter, whose own call steps what it
                // holds: `filter(p).map(f).rev()`'s is `rev()`'s.
                for held in asked.walk().filter_map(|part| part.as_type()) {
                    if matches!(held.kind(), ty::Adt(adapter, _) if is_iter_adapter_def(tcx, adapter.did()))
                        // The type asked itself too where it's of an array's
                        // items with a destructor, which `rev()` can't step
                        // from its back once it's held in a variable.
                        && (held != asked || given || dropping_items(recognition, held, &drops))
                    {
                        let made = back_stepped_adapter(tcx, recognition, held, given, &drops)
                            .filter(|&made| !holds_compiled(tcx, made, &compiled));
                        if given {
                            typed.extend(made.map(|ty| typed_key(tcx, body.def_id, ty)));
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
                    true => typed.extend(
                        (given_behind_an_impl(tcx, recognition, hidden, &drops).into_iter())
                            .map(|ty| typed_key(tcx, body.def_id, ty)),
                    ),
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
                || !crate::library::escapes(tcx, library, local.to_def_id())
            {
                continue;
            }
            // What its consumers give it, a std adapter of a double-ended
            // source, which it may step from its back: each made knowing it,
            // by their makers as by its body (`library_backs`).
            let identity = ty::GenericArgs::identity_for_item(tcx, body.def_id);
            let backs = library_backs(tcx, recognition, body.def_id, identity, &drops);
            typed.extend(backs.into_iter().map(|ty| typed_key(tcx, body.def_id, ty)));
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
                    let given = given_behind_an_impl(tcx, recognition, hidden, &drops);
                    typed.extend(given.into_iter().map(|ty| typed_key(tcx, body.def_id, ty)));
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
            typed.insert(typed_key(tcx, owner, made));
            of_body.entry(owner).or_default().insert(made);
        }
    }
    with_stepped_sources(tcx, typed)
}

/// `typed`, and the source of each of them that steps from its back where
/// its source does, `back_stepped_source`'s, a `Filter`'s of a `Copied`
/// given to generic code that steps it from its back: a type's question
/// too, made knowing its back where it's made (#388, #851).
fn with_stepped_sources<'tcx>(tcx: TyCtxt<'tcx>, typed: IndexSet<TypedKey<'tcx>>) -> IndexSet<TypedKey<'tcx>> {
    let mut all = typed.clone();
    for (owner, ty) in typed {
        let sources = back_stepped_source(tcx, ty).map(|held| tcx.erase_and_anonymize_regions(held.peel_refs()));
        // Its source's, keyed where the source's parameters are declared.
        all.extend(sources.map(|held| owner.map_or((None, held), |owner| typed_key(tcx, owner, held))));
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
    hidden: Ty<'tcx>,
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> Vec<Ty<'tcx>> {
    let compiled = crate::recognition::compiled_behind_an_impl(tcx, recognition.typing_env, hidden);
    (recognition.reveal(hidden).walk().filter_map(|part| part.as_type()))
        .filter_map(|held| {
            matches!(held.kind(), ty::Adt(a, _) if is_iter_adapter_def(tcx, a.did()))
                .then(|| back_stepped_adapter(tcx, recognition, held, true, &drops))
                .flatten()
        })
        .filter(|&made| !holds_compiled(tcx, made, &compiled))
        .collect()
}

/// Whether `made`, an adapter the crate may make knowing its back, is or
/// holds one of `compiled`, std's struct an `impl` hides
/// (`compiled_behind_an_impl`), which no such form of it steps (#1226).
fn holds_compiled<'tcx>(tcx: TyCtxt<'tcx>, made: Ty<'tcx>, compiled: &IndexSet<Ty<'tcx>>) -> bool {
    (made.walk().filter_map(|part| part.as_type()))
        .any(|part| compiled.contains(&tcx.erase_and_anonymize_regions(part)))
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
    // A chain made here, whose own form steps only forward: made knowing
    // its back where it's made, `$lazyChain` of its sides made knowing
    // theirs, as one an adapter asks beneath it is (#739), where MIR lends
    // it to its own `nth_back`, `next_back` or `rfind` (#1118). Not one a
    // function gives behind an `impl`, std's `Chain`, above.
    if !given && chained_sources(tcx, adapter).is_some() {
        return Some(tcx.erase_and_anonymize_regions(adapter));
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
        while let Some(reversed) = crate::recognition::reversed_source(tcx, source) {
            source = reversed.peel_refs();
        }
        source
    };
    // Of generic code's owning one, whose item the crate's typing
    // environment can't normalize, `vec::IntoIter<T>`'s: the `T` it owns,
    // its first type, which `drops` asks as its owner's (#1406).
    let array_of_drops = |source: Ty<'tcx>| {
        let owned = || match (recognition.array_source(source), source.kind()) {
            (Some(true), ty::Adt(_, args)) => args.types().next(),
            _ => None,
        };
        recognition.array_source(source).is_some()
            && (recognition.iterator_item(source).or_else(owned)).is_some_and(&drops)
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
pub struct BackStepped<'tcx> {
    /// The types whose question is a type's (`back_stepped`'s second):
    /// asked of every value of one, never of a value alone. The other types
    /// `back_stepped` gives seed `stepped_locals` alone, what a value made
    /// of what's stepped is made of (#851).
    pub(crate) typed: IndexSet<TypedKey<'tcx>>,
    /// Each `&mut` of a whole local, the local it lends (`borrows`).
    pub(crate) borrows: IndexMap<(rustc_span::def_id::DefId, rustc_middle::mir::Local), rustc_middle::mir::Local>,
    pub(crate) locals: IndexMap<(rustc_span::def_id::DefId, rustc_middle::mir::Local), SteppedLocal>,
    /// Each local whose length is asked, made knowing it where it's made,
    /// and how (`LengthAsked`).
    pub(crate) lengths: IndexMap<(rustc_span::def_id::DefId, rustc_middle::mir::Local), LengthAsked>,
}

/// How a local whose length the crate asks is made knowing it
/// (`BackStepped::lengths`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LengthAsked {
    /// Counted by its maker, which makes it knowing its length
    /// (`made_counted_from_its_back`): what an enumeration stepped from its
    /// back is made of, which counts it there (#1354), and a counted
    /// adapter's temporary a `Vec` is grown by (#1317).
    Counted,
    /// A variable, or a temporary, that holds what's made in place, whose
    /// length, or whose parts', the growth of a `Vec` by what it's made
    /// into asks (`lengths_asked`): made knowing it as it's written
    /// (`written_as_asked`, #1795, #1744, #2041).
    Held,
}

/// What the crate's later uses ask of a local's value, by its maker's MIR
/// identity (`BackStepped::asked_of`): its back, and how it's held where
/// it's stepped so, and its length. Its maker makes it knowing each.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Asked {
    pub(crate) back: Option<SteppedLocal>,
    pub(crate) len: Option<LengthAsked>,
}

impl<'tcx> BackStepped<'tcx> {
    /// Whether `ty`, regions erased, of `owner`'s body, is a type whose
    /// question is a type's (`typed`), where its parameters are declared.
    pub(crate) fn is_typed(&self, tcx: TyCtxt<'tcx>, owner: rustc_span::def_id::DefId, ty: Ty<'tcx>) -> bool {
        self.typed.contains(&typed_key(tcx, owner, ty))
    }

    /// What's asked of `local`'s value, of `owner`'s body: the one answer of
    /// its back and its length, which its maker reads.
    pub(crate) fn asked_of(&self, owner: rustc_span::def_id::DefId, local: rustc_middle::mir::Local) -> Asked {
        Asked {
            back: self.locals.get(&(owner, local)).copied(),
            len: self.lengths.get(&(owner, local)).copied(),
        }
    }

    /// The local `owner`'s `local` lends where it's a `&mut` of a whole
    /// one, `borrows`', else `local`: what's asked of a value by its MIR
    /// identity is asked of the local that holds it.
    pub(crate) fn lent(
        &self,
        owner: rustc_span::def_id::DefId,
        local: rustc_middle::mir::Local,
    ) -> rustc_middle::mir::Local {
        self.borrows.get(&(owner, local)).copied().unwrap_or(local)
    }

    /// Whether a chain of type `ty` stays hand-written in `owner`'s body
    /// `mir`, the held flip's policy (`decide_held`, #1681): of a type whose
    /// question is a type's, or of the type of any local the body steps from
    /// its back or counts there (`LengthAsked::Counted`). Not whether this
    /// value is stepped from its back, which `stepped_from_its_back` alone
    /// answers, by its MIR identity: a policy of readability, as std's
    /// compiled structs read longer than the hand-written forms that are
    /// correct where they're kept (#2157, #2158, #2176). And of a type made
    /// knowing its hint wherever it's made, `hinted` (`Hinted::typed`), as
    /// its hand-written form is made so (#1236, #2209).
    pub(crate) fn keeps_hand_written(
        &self,
        recognition: Recognition<'_, 'tcx>,
        (owner, mir): (rustc_span::def_id::DefId, &rustc_middle::mir::Body<'tcx>),
        ty: Ty<'tcx>,
        hinted: &super::hint_asked::Hinted<'tcx>,
    ) -> bool {
        let tcx = recognition.tcx;
        let erased = |ty: Ty<'tcx>| tcx.erase_and_anonymize_regions(recognition.reveal(ty.peel_refs()));
        let ty = erased(ty);
        self.is_typed(tcx, owner, ty)
            || hinted.is_typed(tcx, owner, ty)
            || (self.locals.keys())
                .chain(
                    self.lengths
                        .iter()
                        .filter(|(_, how)| **how == LengthAsked::Counted)
                        .map(|(at, _)| at),
                )
                .any(|&(of, local)| of == owner && erased(mir.local_decls[local].ty) == ty)
    }
}

/// The body that makes what a call of `callee` with `args` gives: its own,
/// or of a closure called through its `Fn` trait, the closure's (#851).
pub(crate) fn made_by<'tcx>(
    tcx: TyCtxt<'tcx>,
    callee: rustc_span::def_id::DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> rustc_span::def_id::DefId {
    match args.types().next().map(|t| *t.kind()) {
        Some(ty::Closure(closure, _)) if tcx.fn_trait_kind_from_def_id(tcx.parent(callee)).is_some() => closure,
        _ => callee,
    }
}
