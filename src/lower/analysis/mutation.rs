//! The types something changes in place, which a copy is made of where
//! it's read (ADR 0052).

use crate::collections::{IndexMap, IndexSet};
use crate::lower::recognition::{assign_operator, flattens_mut, replaces_whole};
use crate::lower::{Body, FnInfo, fn_def, strip};
use rustc_ast::Mutability;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_middle::mir::BorrowKind;
use rustc_middle::thir::{ExprId, ExprKind};
use rustc_middle::ty;
use rustc_middle::ty::{Ty, TyCtxt};
use rustc_span::def_id::DefId;

/// The types whose JS objects get changed in place somewhere in the crate:
/// `a.b.c = ..` changes the object `a.b`, so it's `a.b`'s type. Only these
/// ever need copying (ADR 0020).
pub(super) fn mutated_types<'tcx>(tcx: TyCtxt<'tcx>, all_bodies: &[&Body<'tcx>]) -> IndexSet<Ty<'tcx>> {
    let mut mutated = IndexSet::default();
    for body in all_bodies {
        custom_mir_writes(tcx, body, &mut mutated);
        // A place a raw pointer is taken to or into, written whole again,
        // which is written over what it holds, `$assign(p, v)`, and each
        // object inside it a pointer is into (#1484): changed in place, so a
        // copy of it is made where one is read. A pointer's pointee is
        // already, below; an object it's inside isn't.
        if let Some(mir) = &body.mir {
            for (_, written, kept) in crate::lower::pointers::rewritten_places(tcx, &mir.body) {
                // And each object inside it kept, written over first.
                for place in std::iter::once(written).chain(kept) {
                    let ty = place.ty(&mir.body.local_decls, tcx).ty;
                    if object_like(tcx, ty) {
                        mutated.insert(ty);
                    }
                }
            }
        }
        // A `let`'s pattern's too, `let (a, _) = &mut t;`, and a closure's
        // or a function's parameter's, `|(a, b)|` of `iter_mut()`'s items,
        // `for_each(|(a, b)| *a += 1)` as a `for` loop's pattern (#1076).
        for stmt in body.thir.stmts.iter() {
            if let rustc_middle::thir::StmtKind::Let { ref pattern, .. } = stmt.kind {
                ref_mut_holders(tcx, pattern, &mut mutated);
            }
        }
        for param in body.thir.params.iter() {
            if let Some(pattern) = &param.pat {
                ref_mut_holders(tcx, pattern, &mut mutated);
            }
        }
        for expr in body.thir.exprs.iter() {
            // An enum something takes `&mut` of may have a variant's field
            // changed through it (and one matched with a `ref mut` binding,
            // `ref_mut_holders`, below). Not an `Option`, whose `Some(x)` is
            // `x` (ADR 0033): a `&mut` of one is a handle on its place, and
            // what its `Some` holds is changed as itself.
            let enum_ty = |ty: Ty<'tcx>| {
                let ty = ty.peel_refs();
                matches!(ty.kind(), ty::Adt(adt, _) if adt.is_enum()) && object_like(tcx, ty)
            };
            match expr.kind {
                ExprKind::Borrow {
                    borrow_kind: BorrowKind::Mut { .. },
                    arg,
                } if enum_ty(body.thir[arg].ty) => {
                    mutated.insert(body.thir[arg].ty.peel_refs());
                }
                // An array something takes `&mut` of may have its items changed
                // through it, as a `&mut [T]` std's slice methods, `iter_mut()`
                // or a function of a slice changes them by.
                ExprKind::Borrow {
                    borrow_kind: BorrowKind::Mut { .. },
                    arg,
                } if matches!(body.thir[arg].ty.kind(), ty::Array(..)) => {
                    mutated.insert(body.thir[arg].ty);
                }
                // And `as_flattened_mut()`, the items of each array of a slice
                // of arrays, which it changes through: each array's.
                ExprKind::Call { fun, ref args, .. }
                    if fn_def(body.thir[fun].ty).is_some_and(|(id, _)| flattens_mut(tcx, id))
                        && let [rows] = &args[..]
                        && let ty::Slice(row) | ty::Array(row, _) = body.thir[*rows].ty.peel_refs().kind() =>
                {
                    mutated.insert(*row);
                }
                _ => {}
            }
            // A `ref mut` binding of a pattern, through a `&mut` subject too,
            // `let (a, _) = &mut t`, may change the field it binds, of the
            // tuple or struct it's in, at any depth.
            match expr.kind {
                ExprKind::Match { ref arms, .. } => {
                    for &arm in arms.iter() {
                        ref_mut_holders(tcx, &body.thir[arm].pattern, &mut mutated);
                    }
                }
                ExprKind::Let { ref pat, .. } => {
                    ref_mut_holders(tcx, pat, &mut mutated);
                }
                _ => {}
            }
            // `*r = v` through a `&mut` changes an object in place (ADR 0147),
            // as `mem::swap`, `mem::replace` and `mem::take` of one do. So
            // does a closure's write of what it took by reference, `|x| p =
            // Point { x }`, THIR's of the variable, MIR's `*(_1.0) = ..`
            // through the capture's `&mut` (#469).
            if let ExprKind::Assign { lhs, .. } = expr.kind
                && (match body.thir[strip(&body.thir, lhs)].kind {
                    ExprKind::Deref { arg } => matches!(body.thir[arg].ty.kind(), ty::Ref(_, _, Mutability::Mut)),
                    _ => written_through_capture(tcx, &body.thir, strip(&body.thir, lhs)),
                })
                && object_like(tcx, body.thir[lhs].ty)
            {
                mutated.insert(body.thir[lhs].ty);
            }
            // And a `&mut *b` of what a `Box` holds, `*c += v`'s of a
            // `Wrapping`, which may write it whole: the box's value is the
            // object itself, so that write is in place, where a variable's
            // `&mut` is a box copied back into it (#967).
            if let ExprKind::Borrow {
                borrow_kind: BorrowKind::Mut { .. },
                arg,
            } = expr.kind
                && let ExprKind::Deref { arg: boxed } = body.thir[strip(&body.thir, arg)].kind
                && body.thir[boxed].ty.is_box()
                && object_like(tcx, body.thir[arg].ty)
            {
                mutated.insert(body.thir[arg].ty);
            }
            // And std's `op=` of a value it writes whole, a `Wrapping`'s, of
            // what a reference refers to, `*c.as_mut().unwrap() += d`'s `&mut
            // *r`: written in place where it is, as a box's content is, where
            // a variable's own `&mut` is the variable rebound (#1020).
            if let ExprKind::Call { fun, ref args, .. } = expr.kind
                && let ty::FnDef(def_id, _) = *body.thir[fun].ty.kind()
                && tcx
                    .trait_of_assoc(def_id)
                    .is_some_and(|t| assign_operator(tcx, t).is_some())
                && let Some(&target) = args.first()
                && let ExprKind::Borrow {
                    borrow_kind: BorrowKind::Mut { .. },
                    arg,
                } = body.thir[strip(&body.thir, target)].kind
                && let ExprKind::Deref { arg: through } = body.thir[strip(&body.thir, arg)].kind
                && body.thir[through].ty.is_ref()
                && matches!(body.thir[arg].ty.kind(), ty::Adt(adt, _) if !adt.did().is_local())
                && object_like(tcx, body.thir[arg].ty)
            {
                mutated.insert(body.thir[arg].ty);
            }
            if let ExprKind::Call { fun, ref args, .. } = expr.kind
                && let ty::FnDef(def_id, _) = *body.thir[fun].ty.kind()
                && replaces_whole(tcx, def_id)
            {
                for &arg in args.iter() {
                    if let ty::Ref(_, inner, Mutability::Mut) = *body.thir[arg].ty.kind()
                        && object_like(tcx, inner)
                    {
                        mutated.insert(inner);
                    }
                }
            }
            // What a raw pointer points to has a place of its own, which a copy
            // shared with another wouldn't be (ADR 0407): copied as if changed.
            // Not std's own, inside a macro of its, `thread_local!`'s.
            if let Some((pointee, _)) = crate::lower::pointers::raw_pointee(tcx, expr.ty)
                && !expr.span.in_external_macro(tcx.sess.source_map())
                && object_like(tcx, pointee)
            {
                mutated.insert(pointee);
            }
            // `a[i] = ..` changes the array `a` the same way, and so does a
            // `&mut` of a part that's its place, text or a number, not an
            // object changed in place: `a.0.push('!')`'s, `a[0] += "!"`. So
            // does a raw pointer to any part, `&raw mut p.x`, whose writes
            // are the part's place's, `$ptr(p, "x", 0)`'s, an object's too:
            // a write of one through it replaces the field (#1386).
            let place = match expr.kind {
                ExprKind::Assign { lhs, .. } | ExprKind::AssignOp { lhs, .. } => Some(lhs),
                ExprKind::Borrow {
                    borrow_kind: BorrowKind::Mut { .. },
                    arg,
                } if !object_like(tcx, body.thir[arg].ty) => Some(arg),
                ExprKind::RawBorrow {
                    mutability: Mutability::Mut,
                    arg,
                } => Some(arg),
                _ => None,
            };
            if let Some(place) = place
                && let ExprKind::Field { lhs: object, .. } | ExprKind::Index { lhs: object, .. } =
                    body.thir[strip(&body.thir, place)].kind
            {
                mutated.insert(body.thir[object].ty);
            }
        }
    }
    mutated
}

/// Whether `place`, a closure's, is a variable or a field of one it took
/// by reference, that place itself: `p` of `|x| p = ..`, `pair.a` of
/// `|x| pair.a = ..`, which takes `pair.a` alone. Its write is through the
/// capture's `&mut`. Not a field of what it took, `pair.a` where it took
/// `pair`, whose write is of the field. Through a box too, `(*frame).size`
/// of a `Box<Frame>`, which a closure takes as that place, the box being
/// the value it holds (#997).
fn written_through_capture(tcx: TyCtxt<'_>, thir: &rustc_middle::thir::Thir<'_>, mut place: ExprId) -> bool {
    use rustc_middle::hir::place::ProjectionKind;
    // Each step to the place, a field's, or `None` of a box's deref.
    let mut fields = Vec::new();
    let (closure, var) = loop {
        match thir[place].kind {
            ExprKind::Field { lhs, name, .. } => {
                fields.push(Some(name));
                place = lhs;
            }
            ExprKind::Deref { arg } if thir[arg].ty.is_box() => {
                fields.push(None);
                place = arg;
            }
            ExprKind::Scope { value, .. } => place = value,
            ExprKind::UpvarRef {
                closure_def_id,
                var_hir_id,
            } => break (closure_def_id, var_hir_id.0),
            _ => return false,
        }
    };
    fields.reverse();
    let Some(closure) = closure.as_local() else {
        return false;
    };
    tcx.closure_captures(closure).iter().any(|captured| {
        captured.get_root_variable() == var
            && matches!(captured.info.capture_kind, ty::UpvarCapture::ByRef(_))
            && captured.place.projections.len() == fields.len()
            && (captured.place.projections.iter().zip(&fields)).all(|(projection, field)| {
                match (projection.kind, field) {
                    (ProjectionKind::Field(at, _), Some(field)) => at == *field,
                    (ProjectionKind::Deref, None) => true,
                    _ => false,
                }
            })
    })
}

/// May `ty` be a JS object, which a `&mut` replaces in place (ADR 0147)? A
/// number, a string, an `Option` and a `Box` aren't: a `&mut` to one is its
/// place's, and a string can't change. Nor an `Rc` or a `Weak`, which its
/// owners share: a `&mut` to one is a cell, whose `*r = v` gives the place
/// another (ADR 0147, amended).
/// Of a body written in custom MIR (`injection_phase`), whose THIR is the
/// `mir!` macro's calls, not its writes: each object its MIR changes in
/// place, read from the MIR itself. A write through a projection,
/// `(RET as Variant0).1 = v` or `_5.0 = v`, changes each object along it,
/// the enum, the tuple; a `&mut` or `*mut` of a place may change it. Its
/// copy, `_5 = RET` of a `Copy` enum, is then made apart, where JS shared
/// one object and a write through either changed both (#1396).
fn custom_mir_writes<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>, mutated: &mut IndexSet<Ty<'tcx>>) {
    use rustc_middle::mir::{Place, ProjectionElem, Rvalue, StatementKind, TerminatorKind};
    let Some(mir) = &body.mir else {
        return;
    };
    let mir = &mir.body;
    if mir.injection_phase.is_none() {
        return;
    }
    let mut changed = |place: &Place<'tcx>, whole: bool| {
        let mut ty = mir.local_decls[place.local].ty;
        let mut on = vec![ty];
        for elem in place.projection {
            if matches!(elem, ProjectionElem::Deref) {
                on.clear();
            }
            ty = rustc_middle::mir::PlaceTy::from_ty(ty).projection_ty(tcx, elem).ty;
            on.push(ty);
        }
        // What's written whole isn't changed in place; what holds it is.
        if !whole {
            on.pop();
        }
        for ty in on {
            if object_like(tcx, ty) {
                mutated.insert(ty);
            }
        }
    };
    for block in mir.basic_blocks.iter() {
        for statement in &block.statements {
            match &statement.kind {
                StatementKind::Assign(assign) => {
                    if !assign.0.projection.is_empty() {
                        changed(&assign.0, false);
                    }
                    match &assign.1 {
                        Rvalue::Ref(_, rustc_middle::mir::BorrowKind::Mut { .. }, place)
                        | Rvalue::RawPtr(rustc_middle::mir::RawPtrKind::Mut, place) => changed(place, true),
                        _ => {}
                    }
                }
                StatementKind::SetDiscriminant { place, .. } => changed(place, true),
                _ => {}
            }
        }
        if let TerminatorKind::Call { destination, .. } = &block.terminator().kind
            && !destination.projection.is_empty()
        {
            changed(destination, false);
        }
    }
}

fn object_like<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    match ty.kind() {
        ty::Adt(..)
            if crate::lower::recognition::rc_pointee(tcx, ty).is_some()
                || crate::lower::recognition::weak_pointee(tcx, ty).is_some() =>
        {
            false
        }
        ty::Adt(adt, _) => {
            !adt.is_box()
                && ![LangItem::String, LangItem::Option]
                    .into_iter()
                    .any(|item| tcx.is_lang_item(adt.did(), item))
        }
        ty::Tuple(items) => !items.is_empty(),
        ty::Array(..) | ty::Slice(_) => true,
        _ => false,
    }
}

/// Each object-like type a `ref mut` binding of `pat` is a part of, which
/// a write through it changes in place, into `mutated`, an enum's variant's
/// too; and whether `pat` holds one.
fn ref_mut_holders<'tcx>(
    tcx: TyCtxt<'tcx>,
    pat: &rustc_middle::thir::Pat<'tcx>,
    mutated: &mut IndexSet<Ty<'tcx>>,
) -> bool {
    use rustc_middle::thir::PatKind;
    let mut holds = false;
    let mut subpatterns: Vec<&rustc_middle::thir::Pat<'tcx>> = Vec::new();
    match &pat.kind {
        PatKind::Binding { mode, subpattern, .. } => {
            holds = matches!(mode.0, rustc_hir::ByRef::Yes(_, Mutability::Mut));
            subpatterns.extend(subpattern.as_deref());
        }
        PatKind::Leaf { subpatterns: fields }
        | PatKind::Variant {
            subpatterns: fields, ..
        } => {
            subpatterns.extend(fields.iter().map(|field| &field.pattern));
        }
        PatKind::Deref { subpattern, .. } => subpatterns.push(subpattern),
        PatKind::Slice { prefix, slice, suffix } | PatKind::Array { prefix, slice, suffix } => {
            subpatterns.extend(prefix.iter().chain(slice.as_deref()).chain(suffix.iter()));
        }
        PatKind::Or { pats } => subpatterns.extend(pats.iter()),
        _ => {}
    }
    let parts = subpatterns
        .into_iter()
        .fold(false, |any, sub| ref_mut_holders(tcx, sub, mutated) | any);
    if parts
        && !matches!(
            pat.kind,
            PatKind::Binding { .. } | PatKind::Deref { .. } | PatKind::Or { .. }
        )
        && object_like(tcx, pat.ty.peel_refs())
    {
        mutated.insert(pat.ty.peel_refs());
    }
    holds || parts
}

/// The type parameters whose `Copy` bound takes a copy function, `TCopy`
/// (ADR 0289): every one of a function a dictionary may call, a trait's,
/// and of the crate's own others those a caller of the crate's gives a
/// value whose copy may not be the value itself, or a type parameter of its
/// own that is one: a library's public one too, whose consumer that gives
/// one more compiles it itself (ADR 0914). A reference, a number, text, a
/// function, and an `Option` of one is its own copy.
pub(in crate::lower) fn copied_params<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    fns: &IndexMap<DefId, FnInfo>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
) -> IndexSet<(DefId, u32)> {
    let bounded = |id| copy_bounded(tcx, id);
    given_params(
        tcx,
        all_bodies,
        (fns.keys(), &|id| fns.contains_key(&id)),
        foreign,
        &bounded,
        &|ty| copies_itself(tcx, ty),
    )
}

/// The type parameters whose `Clone` bound takes its dictionary, `TClone`,
/// as a `Copy` bound takes a copy function (`copied_params`): where a
/// caller of the crate's gives a value whose clone may not be the value
/// itself (`clones_itself`), or a function a dictionary may call is
/// cloned of; a library's consumer that gives one more compiles its own
/// (ADR 0914, amended). `items` are the crate's, `counted` its `Rc`s whose
/// clone is one more of the count.
pub(in crate::lower) fn cloned_params<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    items: &IndexSet<DefId>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    counted: &super::Counted<'tcx>,
) -> IndexSet<(DefId, u32)> {
    let bounded = |id| clone_bounded(tcx, id);
    let itself = |ty| clones_itself(tcx, counted, ty);
    given_params(
        tcx,
        all_bodies,
        (items.iter(), &|id| items.contains(&id)),
        foreign,
        &bounded,
        &itself,
    )
}

/// The type parameters of the crate's own functions a `Some` of which is
/// the value itself, never a box (ADR 0051, amended): of a function only
/// its crate's calls call (`seen_whole`, `Fact::CopiedApart`), those none of its calls
/// gives what may look like `None`, `()` or an `Option`
/// (`representation::may_look_like_none`), or a type parameter of its own
/// that may be one. Every other is given one, a JS caller's `R` too, which
/// is any JS value and so is given as it is, as TypeScript's `R | undefined`
/// says; a library's consumer that gives one compiles its own (ADR 0914).
pub(in crate::lower) fn unboxed_params<'tcx>(
    recognition: &crate::lower::recognition::Recognition<'_, 'tcx>,
    all_bodies: &[&Body<'tcx>],
    items: &IndexSet<DefId>,
) -> IndexSet<(DefId, u32)> {
    let tcx = recognition.tcx;
    let foreign = recognition.foreign;
    let nullish = given_params(
        tcx,
        all_bodies,
        (items.iter(), &|id| items.contains(&id)),
        foreign,
        &|id| type_params(tcx, id),
        &|ty| !crate::lower::representation::may_look_like_none(recognition.tcx, ty),
    );
    (items.iter().copied())
        .filter(|&id| {
            crate::lower::instances::seen_whole(tcx, id, crate::lower::instances::Fact::CopiedApart(foreign))
                && tcx.typeck_root_def_id(id) == id
        })
        .flat_map(|id| type_params(tcx, id).into_iter().map(move |index| (id, index)))
        .filter(|param| !nullish.contains(param))
        .collect()
}

/// Each type parameter of `id`'s, its parents' too, by index.
pub(super) fn type_params(tcx: TyCtxt<'_>, id: DefId) -> Vec<u32> {
    if !matches!(tcx.def_kind(id), DefKind::Fn | DefKind::AssocFn) {
        return Vec::new();
    }
    let generics = tcx.generics_of(id);
    (0..generics.count() as u32)
        .filter(|&index| {
            matches!(
                generics.param_at(index as usize, tcx).kind,
                ty::GenericParamDefKind::Type { .. }
            )
        })
        .collect()
}

/// The type parameters of `items`, the crate's, whose bound `bounded` says
/// takes what does to a value what its trait does, a copy or a clone: every
/// one of a function a dictionary may call, and of the others those a
/// caller of the crate's gives a value it isn't `itself` of, or a type
/// parameter of its own that is one.
/// Carried by `instances.rs`' `params_given`, `seen_whole` of `Fact::CopiedApart` its
/// policy of a function seen whole (ADR 1115).
fn given_params<'a, 'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    (items, is_item): (impl Iterator<Item = &'a DefId>, &dyn Fn(DefId) -> bool),
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    bounded: &dyn Fn(DefId) -> Vec<u32>,
    itself: &dyn Fn(Ty<'tcx>) -> bool,
) -> IndexSet<(DefId, u32)> {
    let callers = crate::lower::instances::callers_of(tcx, all_bodies);
    crate::lower::instances::params_given(
        tcx,
        &callers,
        (items, is_item),
        &|id| crate::lower::instances::seen_whole(tcx, id, crate::lower::instances::Fact::CopiedApart(foreign)),
        bounded,
        itself,
    )
}

/// Whether `id`'s type parameter `index`'s `Clone` bound takes no
/// dictionary, its clone the value itself: only the crate's callers give
/// it, none what isn't its own clone (`clones`, `cloned_params`'s; ADR
/// 0914, amended).
pub(in crate::lower) fn clones_by_itself(
    tcx: TyCtxt<'_>,
    foreign: &crate::lower::library::Foreign<'_, '_>,
    clones: &IndexSet<(DefId, u32)>,
    (id, index): (DefId, u32),
) -> bool {
    crate::lower::instances::seen_whole(tcx, id, crate::lower::instances::Fact::CopiedApart(foreign))
        && !clones.contains(&(tcx.typeck_root_def_id(id), index))
}

/// The type parameters of `id` bound by `Copy`, by their indices, an
/// impl's with its own.
pub(in crate::lower) fn copy_bounded(tcx: TyCtxt<'_>, id: DefId) -> Vec<u32> {
    match tcx.lang_items().copy_trait() {
        Some(copy) => bounded_by(tcx, id, copy),
        None => Vec::new(),
    }
}

/// The type parameters of `id` bound by `Clone`, by their indices, an
/// impl's with its own (ADR 0914, amended).
pub(in crate::lower) fn clone_bounded(tcx: TyCtxt<'_>, id: DefId) -> Vec<u32> {
    match tcx.lang_items().clone_trait() {
        Some(clone) => bounded_by(tcx, id, clone),
        None => Vec::new(),
    }
}

/// The type parameters of `id` bound by `trait_id`, by their indices.
fn bounded_by(tcx: TyCtxt<'_>, id: DefId, trait_id: DefId) -> Vec<u32> {
    let mut indices = Vec::new();
    for (clause, _) in tcx.clauses_of(id).instantiate_identity(tcx) {
        if let ty::ClauseKind::Trait(predicate) = clause.skip_normalization().kind().skip_binder()
            && predicate.def_id() == trait_id
            && let ty::Param(param) = predicate.self_ty().kind()
            && !indices.contains(&param.index)
        {
            indices.push(param.index);
        }
    }
    indices
}

/// Is a copy of `ty` the value itself, whatever changes: a shared
/// reference, a number, `bool`, text, a function, a closure that changes
/// nothing it captured, and an `Option` of one?
pub(in crate::lower) fn copies_itself<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    match ty.kind() {
        ty::Ref(_, _, Mutability::Not) => true,
        ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Str | ty::Never => true,
        ty::FnDef(..) | ty::FnPtr(..) => true,
        // A closure that changes what it captured has copies of its own,
        // which rust-js refuses where it's copied (ADR 0246).
        ty::Closure(_, args) => args.as_closure().kind() == ty::ClosureKind::Fn,
        ty::Tuple(items) => items.is_empty(),
        ty::Adt(adt, args) if tcx.is_lang_item(adt.did(), LangItem::Option) => copies_itself(tcx, args.type_at(0)),
        _ => false,
    }
}

/// Is a clone of `ty` the value itself, whatever changes: what's its own
/// copy (`copies_itself`) but a closure, which may be cloned apart (ADR
/// 0522); text, `String` too, which nothing changes in place in JS; an
/// `Rc` or an `Arc` that isn't counted (`counted`), the value it shares
/// (ADR 0023); and an `Option` of one (ADR 0914, amended).
pub(in crate::lower) fn clones_itself<'tcx>(tcx: TyCtxt<'tcx>, counted: &super::Counted<'tcx>, ty: Ty<'tcx>) -> bool {
    match ty.kind() {
        ty::Closure(..) => false,
        ty::Adt(adt, args) if tcx.is_lang_item(adt.did(), LangItem::Option) => {
            clones_itself(tcx, counted, args.type_at(0))
        }
        ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::String) => true,
        _ if let Some(pointee) = crate::lower::recognition::rc_pointee(tcx, ty) => !counted.counts(tcx, pointee),
        _ => copies_itself(tcx, ty),
    }
}
