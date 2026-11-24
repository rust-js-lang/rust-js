//! Each local, by its body, whose value the crate steps from its back, and
//! how it holds it (`stepped_locals`, #814): by the MIR identity of what
//! holds a value, never its type, its maker then making it knowing its back
//! where it's made; each local whose length is asked; and each `&mut` of a
//! whole local (`borrows`). back_stepped.rs' types whose question is a
//! type's are the other half of the owner.

use super::*;

/// What `stepped_locals` finds: each local stepped from its back, the
/// types a std struct holding them asks by type alone, and each local whose
/// length is asked. The fields a back step is given it follows to their
/// locals alone: a read of one is of a compiled std adapter, which its
/// maker never marks (#2112's survey, `places.rs`' mark retired).
type SteppedValues<'tcx> = (
    IndexMap<(rustc_span::def_id::DefId, rustc_middle::mir::Local), SteppedLocal>,
    IndexSet<TypedKey<'tcx>>,
    IndexMap<(rustc_span::def_id::DefId, rustc_middle::mir::Local), LengthAsked>,
);

/// A field of a struct or an enum's variant: its ADT, its variant and its
/// index.
pub(crate) type SteppedField = (rustc_span::def_id::DefId, rustc_abi::VariantIdx, rustc_abi::FieldIdx);

/// How a local the crate steps from its back holds its value
/// (`stepped_locals`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SteppedLocal {
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
    (drops, keeps_capacity): (impl Fn(Ty<'tcx>) -> bool, bool),
    (params_may_drop, given_a_drop): (bool, Option<&GivenADrop<'_>>),
) -> SteppedValues<'tcx> {
    use rustc_middle::mir;
    let erased = |ty: Ty<'tcx>| tcx.erase_and_anonymize_regions(recognition.reveal(ty.peel_refs()));
    let zip = crate::recognition::compiled_provided_method(tcx, "Iterator", "zip");
    let rev = crate::recognition::compiled_provided_method(tcx, "Iterator", "rev");
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
    // lent or whole, `self.it.next_back()` of `struct Evens { it: Filter<..> }`,
    // and the trait whose step asks it, `DoubleEndedIterator` or
    // `ExactSizeIterator`: each value MIR moves into it as it makes one,
    // made knowing its back by its maker (#851), where that value's type
    // has the trait, as the trait solver says of the instance. A generic
    // impl's step, `impl<I: DoubleEndedIterator> DoubleEndedIterator for
    // Own<I>`'s `self.0.next_back()`, never runs on an `Own<Count>` whose
    // `Count` isn't one (#2153).
    let mut stepped_fields: IndexSet<(SteppedField, rustc_span::def_id::DefId)> = IndexSet::default();
    let field_stepped = |fields: &IndexSet<(SteppedField, rustc_span::def_id::DefId)>,
                         field: SteppedField,
                         (owner, value): (rustc_span::def_id::DefId, Ty<'tcx>)| {
        let typing_env = ty::TypingEnv::post_analysis(tcx, tcx.typeck_root_def_id(owner));
        (fields.iter()).any(|&(of, asked)| {
            of == field && crate::recognition::trait_holds_in(tcx, typing_env, erased(value), asked) != Some(false)
        })
    };
    let mut typed_sources: IndexSet<TypedKey<'tcx>> = IndexSet::default();
    let mut counted: IndexMap<(rustc_span::def_id::DefId, mir::Local), LengthAsked> = IndexMap::default();
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
            // What each call's value is made of: the locals it's given,
            // `_5 = inspect(move _4, ..)`'s `_4`.
            let mut made_of: IndexMap<mir::Local, Vec<mir::Local>> = IndexMap::default();
            for (block, data) in mir.basic_blocks.iter_enumerated() {
                if let mir::TerminatorKind::Call {
                    destination,
                    target: Some(target),
                    args,
                    ..
                } = &data.terminator().kind
                    && destination.projection.is_empty()
                {
                    made_at.insert(destination.local, (*target, 0));
                    let of = args.iter().filter_map(|arg| match &arg.node {
                        mir::Operand::Move(place) | mir::Operand::Copy(place) => place.as_local(),
                        _ => None,
                    });
                    made_of.entry(destination.local).or_default().extend(of);
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
                if field_stepped(&stepped_fields, field, (owner, mir.local_decls[origin(local)].ty)) {
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
                // knowing its back (#851). And its type's, as the struct
                // steps it by its type's dictionary, a `fuse()` given the
                // `rev()` of a `filter_map` a variable holds (as a struct's
                // part is, `held_by_struct`'s).
                if callee == rev
                    && !body.facts.in_place_adapters.ground_made_at(&body.thir, *fn_span)
                    && let Some(arg) = args.first()
                    && let mir::Operand::Move(place) | mir::Operand::Copy(place) = &arg.node
                    && let Some(local) = place.as_local()
                {
                    mark(&mut stepped, (owner, origin(local)), SteppedLocal::Given);
                    typed_sources.insert(typed_key(tcx, owner, erased(mir.local_decls[origin(local)].ty)));
                }
                // A zip that reads its sides by index (ADR 0973), which asks
                // the length of `b`, a map whose closure has a destructor:
                // made knowing its length where it's made, as `back_stepped`'s
                // rule of its type says (#775).
                if callee == zip
                    && let [a, b] = generic_args.types().collect::<Vec<_>>()[..]
                    && !rustc_middle::ty::TypeVisitableExt::has_param(&(a, b))
                    && let [_, arg] = &args[..]
                    && let mir::Operand::Move(place) | mir::Operand::Copy(place) = &arg.node
                    && let Some(local) = place.as_local()
                {
                    let typing_env = ty::TypingEnv::post_analysis(tcx, tcx.typeck_root_def_id(owner));
                    let b = recognition.reveal(crate::recognition::iterator_type_of(tcx, typing_env, b));
                    if crate::recognition::zips_by_index(tcx, typing_env, a, b) && closure_drops(tcx, b, &drops) {
                        let local = origin(local);
                        let how = match user_variable(&mir.local_decls[local]) {
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
                // A value that leaves for code that may give it back, a copy of
                // it or itself, to be stepped from its back where this walk
                // doesn't follow: `pass(a)` of `fn pass<I>(it: I) -> I`, `dup(&a)`,
                // `a.clone()`, `Box::new(a)`. By the callee's declared signature,
                // what its parameter is given comes back where its result names
                // a type parameter of that parameter's type, or another parameter
                // it may write does, `&mut Vec<I>`; a library's function may do
                // anything with it. Not the iterator traits' own methods, whose
                // steps the walk reads, nor generic code that only steps it,
                // which its type's question answers (#388). One of std's
                // adapters of a double-ended type, or one the walk can't say
                // isn't, is made knowing its back where it's made, as `rev()`'s
                // is: unsure is made knowing (#1981).
                let iterator_protocol = (tcx.trait_of_assoc(callee)).is_some_and(|tr| {
                    is_double_ended(tcx, tr)
                        || is_exact_size(tcx, tr)
                        || crate::recognition::is_iterator_trait(tcx, tr)
                        || crate::recognition::is_std_def(tcx, tr, crate::recognition::StdItem::IntoIterator)
                });
                if !iterator_protocol {
                    let library = recognition.foreign.is_library(callee.krate);
                    let declared = tcx.instantiate_bound_regions_with_erased(
                        tcx.fn_sig(callee).instantiate_identity().skip_normalization(),
                    );
                    let params_of = |ty: Ty<'tcx>| -> Vec<Ty<'tcx>> {
                        (ty.walk().filter_map(|part| part.as_type()))
                            .filter(|t| crate::generic_facts::is_type_parameter(*t))
                            .collect()
                    };
                    for (i, arg) in args.iter().enumerate() {
                        let (mir::Operand::Move(place) | mir::Operand::Copy(place)) = &arg.node else {
                            continue;
                        };
                        let Some(given) = place.as_local() else { continue };
                        let local = lent.get(&given).copied().unwrap_or_else(|| origin(given));
                        let ty = erased(mir.local_decls[local].ty);
                        let adapter = matches!(ty.kind(), ty::Adt(adt, _) if is_iter_adapter_def(tcx, adt.did()));
                        if !adapter || recognition.trait_holds(ty, double_ended) == Some(false) {
                            continue;
                        }
                        let comes_back = library
                            || declared.inputs().get(i).is_some_and(|&input| {
                                let named = params_of(input);
                                let names = |t: Ty<'tcx>| params_of(t).iter().any(|p| named.contains(p));
                                // A function of no type parameter, of the adapter's own
                                // type: its result or another `&mut` parameter of it.
                                let same = |t: Ty<'tcx>| {
                                    t.walk()
                                        .filter_map(|part| part.as_type())
                                        .any(|part| erased(part) == ty)
                                };
                                let written = |j: usize, t: Ty<'tcx>| {
                                    j != i && matches!(t.kind(), ty::Ref(_, _, ty::Mutability::Mut))
                                };
                                names(declared.output())
                                    || (named.is_empty() && same(declared.output()))
                                    || (declared.inputs().iter().enumerate())
                                        .any(|(j, &t)| written(j, t) && (names(t) || named.is_empty() && same(t)))
                            });
                        if comes_back {
                            mark(&mut stepped, (owner, local), SteppedLocal::Given);
                            typed_sources.insert(typed_key(tcx, owner, ty));
                        }
                    }
                }
                // A value given beside a `&mut` of a local stepped from its
                // back, of its type, which the callee may write there,
                // `replace_in(&mut it, next)`, its caller's own as one
                // representation (#537): stepped so too (#851).
                // Beside a `&mut` of a field stepped from its back too,
                // `replaced(&mut held.0, next)` (#1013).
                let lent_stepped_fields = (args.iter()).filter_map(|arg| match &arg.node {
                    mir::Operand::Move(place) | mir::Operand::Copy(place) => {
                        let field = *lent_field.get(&place.as_local()?)?;
                        let value = field.ty(&mir.local_decls, tcx).ty;
                        (field_of(tcx, mir, field).is_some_and(|of| field_stepped(&stepped_fields, of, (owner, value))))
                            .then(|| (None, erased(value)))
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
                // A `len()` of a variable, through what moves or lends it: its
                // length asked of it, made knowing it where it's written, as
                // a `Vec`'s growth asks it (`written_as_asked`), a copy of a
                // map made in place, whose type says nothing of its form
                // (#1978).
                if recognition.iterator_method(callee) == Some(crate::recognition::IterMethod::Len)
                    && let Some(mut local) = (args.first()).and_then(|arg| match &arg.node {
                        mir::Operand::Move(place) | mir::Operand::Copy(place) => place.as_local(),
                        _ => None,
                    })
                {
                    let mut seen: IndexSet<mir::Local> = IndexSet::default();
                    while seen.insert(local)
                        && let Some(&of) = moved.get(&local).or_else(|| lent.get(&local))
                    {
                        local = of;
                    }
                    if user_variable(&mir.local_decls[local]) {
                        counted.entry((owner, local)).or_insert(LengthAsked::Held);
                    }
                }
                // A zip a `Vec` is grown by, in a crate that keeps capacities,
                // exact-sized but not `TrustedLen`, whose length the collect
                // or the extend asks to grow it by (`grown_ask`'s `exact`),
                // MIR holding it in a temporary, no form of it left to count:
                // made knowing its back and length where it's made,
                // `$zipEnds`, the one form that knows it (#1394). And a
                // `take(n)` or a `skip(n)` it's grown by, or what passes its
                // length on of one, a `map` of it (`length_kept_source`): each
                // temporary of it counted, made knowing its length where it's
                // made, `$taking` or `$skipping` of its source made knowing
                // it, as one an enumeration counts from its back is (#1317).
                if keeps_capacity {
                    let typing_env = ty::TypingEnv::post_analysis(tcx, tcx.typeck_root_def_id(owner));
                    let grown = (recognition.vec_grown_by(callee, generic_args, typing_env))
                        .map(erased)
                        .filter(|&grown| !crate::recognition::is_trusted_len(tcx, typing_env, grown));
                    // And each variable whose value it's made of, `let d =
                    // v.iter().map(f)` grown by `d.inspect(g)`, or a chain a
                    // variable holds whose parts' lengths std's sums: the
                    // length that growth asks, of it or of its parts, asked
                    // of the variable, made knowing it where it's made, as
                    // what's made in place is where it's grown (#1795, #1744).
                    // A temporary too, which MIR holds while an argument is
                    // made, the `inspect(f)` of `.inspect(f).chain(vec![9])`
                    // while `vec!` allocates, written where it's made (#2041).
                    // By what the grown value is made of, back from what the
                    // call's given, through each value of an asked type: not
                    // another variable of the same type.
                    if let Some(grown) = grown {
                        let asked = lengths_asked(recognition, grown);
                        // Each step back: what a move gave it, what a borrow
                        // lends, or what the call that made it was given.
                        let mut at: Vec<mir::Local> = (args.iter())
                            .filter_map(|arg| match &arg.node {
                                mir::Operand::Move(place) | mir::Operand::Copy(place) => place.as_local(),
                                _ => None,
                            })
                            .collect();
                        let mut seen: IndexSet<mir::Local> = IndexSet::default();
                        while let Some(local) = at.pop() {
                            if !seen.insert(local) {
                                continue;
                            }
                            if let Some(&of) = moved.get(&local).or_else(|| lent.get(&local)) {
                                at.push(of);
                                continue;
                            }
                            if !asked.contains(&erased(mir.local_decls[local].ty)) {
                                continue;
                            }
                            counted.entry((owner, local)).or_insert(LengthAsked::Held);
                            at.extend(made_of.get(&local).into_iter().flatten());
                        }
                    }
                    if let Some(grown) = grown
                        && recognition.grows_by_exact_len(grown)
                    {
                        if zipped_sources(tcx, grown).is_some() {
                            for &(local, lent, ty, _) in &given {
                                if !lent && ty == grown {
                                    mark(&mut stepped, (owner, local), SteppedLocal::Given);
                                }
                            }
                        }
                        let mut at = Some(grown);
                        while let Some(ty) = at {
                            if counted_source(tcx, ty).is_some() {
                                for (local, decl) in mir.local_decls.iter_enumerated() {
                                    if !user_variable(decl) && erased(decl.ty) == ty {
                                        counted.insert((owner, local), LengthAsked::Counted);
                                    }
                                }
                            }
                            at = crate::recognition::length_kept_source(tcx, ty).map(erased);
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
                            stepped_fields.insert((field, clause.def_id()));
                        }
                    }
                    for &(local, lent, ty, held) in &given {
                        let user = user_variable(&mir.local_decls[local]);
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
                                // Of its own type parameter's, where a caller
                                // gives it a drop (`drop_params`', ADR 0098),
                                // closed where it's made, which no form of a
                                // map's own steps from its back (#1406).
                                || own_method
                                    && receiver
                                    && dropping_items(recognition, ty, |item| {
                                        given_a_drop_of(tcx, owner, item, (params_may_drop, given_a_drop))
                                    })
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
                // A `rev()` a variable holds, std's struct, which steps what it
                // holds from its back and asks it its length, by the
                // dictionary of its type alone, whatever the variable is asked:
                // of a source not std's compiled struct, made in place, a
                // copy's map, that source's type made knowing both, as one
                // a struct holds stepped from its back is (#851, #1978), and
                // each adapter beneath it that steps from its back where its
                // source does (`back_stepped_source`), not what it's made of,
                // a slice's iterator, whose type others share.
                if crate::recognition::is_held_compiled(tcx, made)
                    && user_variable(&mir.local_decls[destination.local])
                    && let Some(source) = reversed_source(tcx, made)
                    && !crate::recognition::is_compiled_std_iterator(tcx, source.peel_refs())
                {
                    let mut at = Some(source.peel_refs());
                    while let Some(adapter) = at
                        && let Some(beneath) = back_stepped_source(tcx, adapter)
                    {
                        typed_sources.insert(typed_key(tcx, owner, erased(adapter)));
                        at = Some(beneath.peel_refs());
                    }
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
                    // A flatten of `Option`s, made a filter of the `Some`s,
                    // steps its source from its back as a filter does (#1342).
                    None => back_stepped_source(tcx, made)
                        .or_else(|| recognition.flattened_options(made).map(|(source, ..)| source))
                        .or_else(|| counted_source(tcx, made).map(|(source, _)| source))
                        .or_else(|| stepped_by_source(tcx, made))
                        .or_else(|| enumerated_source(tcx, made))
                        .into_iter()
                        .collect(),
                };
                // Held by std's struct, which asks the dictionary of what it
                // holds by its type alone, `mapDoubleEndedIterator_nextBack`'s:
                // a question of the type (#851).
                let held_by_struct = crate::recognition::is_held_compiled(tcx, made)
                    && user_variable(&mir.local_decls[destination.local]);
                for source in sources.into_iter().map(erased) {
                    for &(local, _, ty, _) in &given {
                        let user = user_variable(&mir.local_decls[local]);
                        // Or of an adapter stepped from its back that holds a
                        // closure with a destructor, which MIR holds what it's
                        // made of before it's made, its form gone (#851).
                        let spilled = made_asked && closure_drops(tcx, made, &drops);
                        let copy = copied_source(tcx, made).or_else(|| cloned_source(tcx, made)).is_some();
                        if ty == source && (chain && made_asked || user || spilled || copy && made_asked) {
                            mark(&mut stepped, (owner, local), SteppedLocal::Given);
                            // An enumeration's source, counted from its back:
                            // its index there is its length less one (#1354).
                            if made_asked && enumerated_source(tcx, made).is_some() {
                                counted.insert((owner, local), LengthAsked::Counted);
                            }
                            if held_by_struct {
                                typed_sources.insert(typed_key(tcx, owner, ty));
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
            return (stepped, typed_sources, counted);
        }
    }
}

/// The types the growth of a `Vec` by an iterator of type `grown` asks
/// the length of, in a crate that keeps capacities (lowering's `grown_ask`):
/// of an `ExactSizeIterator`, its own, and its source's that it passes on,
/// a map's (`length_kept_source`); of a chain or a zip whose hint is its
/// parts', or a copy of one, each part's so (`lower_bound_kept_by_parts`,
/// #1216). None of another.
fn lengths_asked<'tcx>(recognition: Recognition<'_, 'tcx>, grown: Ty<'tcx>) -> IndexSet<Ty<'tcx>> {
    let tcx = recognition.tcx;
    let mut asked = IndexSet::default();
    if !recognition.grows_by_exact_len(grown) && !recognition.lower_bound_kept_by_parts(grown) {
        return asked;
    }
    let mut at = vec![grown];
    while let Some(ty) = at.pop() {
        let ty = tcx.erase_and_anonymize_regions(recognition.reveal(ty.peel_refs()));
        if !asked.insert(ty) {
            continue;
        }
        if recognition.grows_by_exact_len(ty) {
            at.extend(crate::recognition::length_kept_source(tcx, ty));
        } else if let Some(source) = copied_source(tcx, ty) {
            at.push(source);
        } else if let Some((a, b)) = chained_sources(tcx, ty).or_else(|| zipped_sources(tcx, ty)) {
            at.extend([a, b]);
        }
    }
    asked
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
            !place.projection.is_empty() || user_variable(&mir.local_decls[place.local])
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
fn field_of<'tcx>(
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
