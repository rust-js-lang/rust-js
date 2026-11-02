//! What a function's return alone owns of what it's given (ADR 0317,
//! amended): a parameter, an `Rc` or an `Option` of one, moved, as it is or
//! as its payload, into a field of the struct a new `Rc` points at, the
//! `Rc` the function returns, on each path it returns by,
//!
//! ```ignore
//! fn create_request(on_all_ready: Option<Rc<dyn Fn()>>) -> Rc<Request> {
//!     let request = Rc::new(Request {
//!         on_all_ready: on_all_ready.unwrap_or_else(|| Rc::new(noop)),
//!         ..
//!     });
//!     start(&request);
//!     request
//! }
//! ```
//!
//! and a field no code reads but to call through it, private to its crate,
//! of a struct that isn't `Clone`: nothing clones the `Rc` it holds, nor
//! moves it out. So what it holds lives only while that struct's value
//! does, which is while its `Rc`'s strong count is one at least, or while
//! it's dropped, which runs code only where its drop runs a destructor,
//! whose `Rc` is counted then (ADR 0320). What a caller gives it, a closure,
//! runs only while the `Rc` returned is alive: a `Weak` of that `Rc` it
//! upgrades gives it (`self_handles`'s owner handles).
//!
//! Read of each body's MIR, as borrowck reads it, a function of the
//! crate's that's no generic's; a library's of its contract, which says
//! the same of its own (`crate::library::Calling::owned_by_return`).

use super::self_handles::{Uses, assign_at, drops, whole};
use crate::lower::recognition::{HandleStep, handle_step, rc_pointee, trait_holds_in};
use crate::lower::{Body, fn_def};
use rustc_abi::FieldIdx;
use rustc_middle::mir::{self, AggregateKind, Local, Location, Operand, Place, PlaceElem, Rvalue, TerminatorKind};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

/// The parameters of each of the crate's functions that what it returns
/// alone owns, by index.
#[derive(Default)]
pub(crate) struct OwnedByReturn {
    owned: HashMap<DefId, Vec<u32>>,
}

impl OwnedByReturn {
    /// What `bodies` return alone own, given what a library's function's
    /// does, `foreign`'s, by its contract.
    pub(crate) fn new<'tcx>(
        tcx: TyCtxt<'tcx>,
        bodies: &[&Body<'tcx>],
        foreign: &dyn Fn(DefId, u32) -> bool,
    ) -> OwnedByReturn {
        let of: Vec<(DefId, &mir::Body<'tcx>)> = (bodies.iter())
            .filter_map(|body| Some((body.def_id, &body.mir.as_ref()?.body)))
            .collect();
        // Each function that may own one: of no generics, returning an `Rc`
        // of a struct of the crate's, given an `Rc` or an `Option` of one.
        let candidates: Vec<(DefId, &mir::Body<'tcx>, Vec<u32>)> = (of.iter())
            .filter(|(id, _)| {
                matches!(
                    tcx.def_kind(*id),
                    rustc_hir::def::DefKind::Fn | rustc_hir::def::DefKind::AssocFn
                ) && tcx.generics_of(*id).count() == 0
            })
            .filter_map(|&(id, body)| {
                returned_struct(tcx, body.local_decls[mir::RETURN_PLACE].ty)?;
                let params: Vec<u32> = (0..body.arg_count as u32)
                    .filter(|&i| held_rc(tcx, body.local_decls[Local::from_u32(i + 1)].ty))
                    .collect();
                (!params.is_empty()).then_some((id, body, params))
            })
            .collect();
        if candidates.is_empty() {
            return OwnedByReturn::default();
        }
        let structs: HashSet<DefId> = (candidates.iter())
            .filter_map(|(_, body, _)| returned_struct(tcx, body.local_decls[mir::RETURN_PLACE].ty))
            .collect();
        let read = read_otherwise(tcx, &of, &structs);
        let field_kept = |adt: DefId, field: FieldIdx| kept_field(tcx, adt, field) && !read.contains(&(adt, field));
        // Each only of what it calls already found, to a fixed point: a
        // function that gives it on to another, `create_request`'s to
        // `request_instance`.
        let mut owned: HashMap<DefId, Vec<u32>> = HashMap::new();
        loop {
            let mut more = false;
            for (id, body, params) in &candidates {
                for &index in params {
                    if owned.get(id).is_some_and(|o| o.contains(&index)) {
                        continue;
                    }
                    let by = |g: DefId, j: u32| match g.is_local() {
                        true => owned.get(&g).is_some_and(|o| o.contains(&j)),
                        false => foreign(g, j),
                    };
                    if owns(tcx, body, index, &field_kept, &by) {
                        owned.entry(*id).or_default().push(index);
                        more = true;
                    }
                }
            }
            if !more {
                break;
            }
        }
        for indices in owned.values_mut() {
            indices.sort();
        }
        OwnedByReturn { owned }
    }

    /// Does what `id`, the crate's function, returns alone own its
    /// parameter `index`?
    pub(crate) fn owns(&self, id: DefId, index: u32) -> bool {
        self.owned.get(&id).is_some_and(|indices| indices.contains(&index))
    }

    /// Its parameters what `id` returns alone owns, for its contract.
    pub(crate) fn of(&self, id: DefId) -> Vec<u32> {
        self.owned.get(&id).cloned().unwrap_or_default()
    }
}

/// The struct of the crate's an `Rc` of `ty` points at.
fn returned_struct<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<DefId> {
    match *rc_pointee(tcx, ty)?.kind() {
        ty::Adt(adt, _) if adt.is_struct() && adt.did().is_local() => Some(adt.did()),
        _ => None,
    }
}

/// Whether `ty` is an `Rc` or an `Option` of one.
fn held_rc<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    match *ty.kind() {
        ty::Adt(adt, args) if tcx.is_lang_item(adt.did(), rustc_hir::attrs::lang_items::LangItem::Option) => {
            rc_pointee(tcx, args.type_at(0)).is_some()
        }
        _ => rc_pointee(tcx, ty).is_some(),
    }
}

/// Whether `adt`'s `field` may keep what it's given alone: an `Rc`, which
/// no other crate can name, of a struct that isn't `Clone`, whose clone
/// would clone it.
fn kept_field(tcx: TyCtxt<'_>, adt: DefId, field: FieldIdx) -> bool {
    let def = tcx.adt_def(adt);
    let field = &def.non_enum_variant().fields[field];
    let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
    let clone = tcx.lang_items().clone_trait();
    let typing_env = ty::TypingEnv::non_body_analysis(tcx, adt);
    let of = tcx.type_of(adt).instantiate_identity().skip_normalization();
    rc_pointee(tcx, ty).is_some()
        && !tcx.visibility(field.did).is_public()
        && clone.is_some_and(|clone| trait_holds_in(tcx, typing_env, of, clone) == Some(false))
}

/// The fields of `structs` some body reads but to call through what they
/// hold: anything but a shared borrow of the field, `&request.on_all_ready`,
/// that's only dereferenced, or a drop of it.
fn read_otherwise<'tcx>(
    tcx: TyCtxt<'tcx>,
    bodies: &[(DefId, &mir::Body<'tcx>)],
    structs: &HashSet<DefId>,
) -> HashSet<(DefId, FieldIdx)> {
    let mut read = HashSet::new();
    for &(_, body) in bodies {
        let mut hits: Vec<(Location, Place<'tcx>, DefId, FieldIdx, bool)> = Vec::new();
        let mut visitor = Fields {
            tcx,
            body,
            structs,
            hits: &mut hits,
        };
        mir::visit::Visitor::visit_body(&mut visitor, body);
        if hits.is_empty() {
            continue;
        }
        let uses = Uses::of(body);
        for (at, place, adt, field, last) in hits {
            if !last || !only_called(tcx, body, &uses, at, place) {
                read.insert((adt, field));
            }
        }
    }
    read
}

/// Each place of a field of one of `structs`, and whether that field is
/// its last projection.
struct Fields<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    body: &'a mir::Body<'tcx>,
    structs: &'a HashSet<DefId>,
    hits: &'a mut Vec<(Location, Place<'tcx>, DefId, FieldIdx, bool)>,
}

impl<'tcx> mir::visit::Visitor<'tcx> for Fields<'_, 'tcx> {
    // What the debugger shows of a variable reads nothing.
    fn visit_var_debug_info(&mut self, _: &mir::VarDebugInfo<'tcx>) {}

    fn visit_place(&mut self, place: &Place<'tcx>, _: mir::visit::PlaceContext, at: Location) {
        for (index, (base, elem)) in place.iter_projections().enumerate() {
            if let PlaceElem::Field(field, _) = elem
                && let ty::Adt(adt, _) = base.ty(self.body, self.tcx).ty.kind()
                && self.structs.contains(&adt.did())
            {
                let last = index + 1 == place.projection.len();
                self.hits.push((at, *place, adt.did(), field, last));
            }
        }
    }
}

/// Whether what's at `at`, of `place`, a field, reads it only to call
/// through it: a shared borrow only dereferenced, a drop of it, or what
/// says nothing.
fn only_called<'tcx>(tcx: TyCtxt<'tcx>, body: &mir::Body<'tcx>, uses: &Uses, at: Location, place: Place<'tcx>) -> bool {
    let data = &body.basic_blocks[at.block];
    match data.statements.get(at.statement_index) {
        Some(statement) => match &statement.kind {
            mir::StatementKind::Assign(assign) => match &**assign {
                (to, Rvalue::Ref(_, mir::BorrowKind::Shared | mir::BorrowKind::Fake(_), borrowed))
                    if *borrowed == place && to.projection.is_empty() =>
                {
                    derefed_only(tcx, body, uses, to.local)
                }
                _ => false,
            },
            kind => super::self_handles::inert(kind),
        },
        None => matches!(&data.terminator().kind, TerminatorKind::Drop { place: dropped, .. } if *dropped == place),
    }
}

/// Whether `local`, a borrow of an `Rc`, is only dereferenced, as it is or
/// through copies and reborrows of it: what's reached, a reference to what
/// the `Rc` points at, is no `Rc` to keep.
fn derefed_only<'tcx>(tcx: TyCtxt<'tcx>, body: &mir::Body<'tcx>, uses: &Uses, local: Local) -> bool {
    let mut todo = vec![local];
    let mut seen = HashSet::new();
    while let Some(local) = todo.pop() {
        if !seen.insert(local) {
            continue;
        }
        for &at in uses.uses(local) {
            if drops(body, at, local) {
                continue;
            }
            if let Some((to, rvalue)) = assign_at(body, at) {
                let copied = match rvalue {
                    Rvalue::Use(Operand::Copy(p) | Operand::Move(p), ..) | Rvalue::CopyForDeref(p) => {
                        p.local == local && p.projection.is_empty()
                    }
                    Rvalue::Ref(_, mir::BorrowKind::Shared, p) => {
                        p.local == local && p.projection.as_slice() == [PlaceElem::Deref]
                    }
                    _ => false,
                };
                if !copied || !to.projection.is_empty() {
                    return false;
                }
                todo.push(to.local);
                continue;
            }
            let data = &body.basic_blocks[at.block];
            let TerminatorKind::Call { func, args, .. } = &data.terminator().kind else {
                return false;
            };
            let derefs = fn_def(func.ty(body, tcx))
                .is_some_and(|(id, generics)| handle_step(tcx, id, generics) == Some(HandleStep::RcDeref));
            if !derefs || args.len() != 1 || whole(&args[0].node) != Some(local) {
                return false;
            }
        }
    }
    true
}

/// The call at `at`, a block's terminator: what it calls, its arguments,
/// and its destination, a local whole.
fn call_at<'a, 'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &'a mir::Body<'tcx>,
    at: Location,
) -> Option<(DefId, ty::GenericArgsRef<'tcx>, Vec<&'a Operand<'tcx>>, Local)> {
    let data = &body.basic_blocks[at.block];
    if at.statement_index != data.statements.len() {
        return None;
    }
    let TerminatorKind::Call {
        func,
        args,
        destination,
        ..
    } = &data.terminator().kind
    else {
        return None;
    };
    let (id, generics) = fn_def(func.ty(body, tcx))?;
    destination.projection.is_empty().then_some(())?;
    Some((id, generics, args.iter().map(|a| &a.node).collect(), destination.local))
}

/// Whether `operand` reads any of `locals`.
fn reads_any(operand: &Operand<'_>, locals: &[Local]) -> bool {
    operand.place().is_some_and(|p| locals.contains(&p.local))
}

/// Whether what `body` returns alone owns its parameter `index`: moved, as
/// it is or as its payload, into a field `field_kept` says keeps it, of a
/// struct an `Rc::new` is given, or given on to a function whose return
/// owns it, `by`; that `Rc` only moved, borrowed shared and dropped, and
/// what each path returns.
fn owns<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &mir::Body<'tcx>,
    index: u32,
    field_kept: &dyn Fn(DefId, FieldIdx) -> bool,
    by: &dyn Fn(DefId, u32) -> bool,
) -> bool {
    let uses = Uses::of(body);
    let param = Local::from_u32(index + 1);
    // What holds the parameter's value, as it is or as its payload.
    let mut held = vec![param];
    // The structs made of it, and the `Rc`s of them.
    let mut values: Vec<Local> = Vec::new();
    let mut rcs: Vec<Local> = Vec::new();
    // Where the return place is given one of those `Rc`s.
    let mut returned: HashSet<Location> = HashSet::new();
    let mut i = 0;
    while i < held.len() {
        let local = held[i];
        i += 1;
        if local == mir::RETURN_PLACE || (local != param && uses.def(local).is_none()) {
            return false;
        }
        for &at in uses.uses(local) {
            if drops(body, at, local) {
                continue;
            }
            if let Some((to, rvalue)) = assign_at(body, at) {
                if !to.projection.is_empty() {
                    return false;
                }
                match rvalue {
                    Rvalue::Use(operand, ..) | Rvalue::Cast(_, operand, _)
                        if matches!(operand, Operand::Move(_)) && whole(operand) == Some(local) =>
                    {
                        held.push(to.local);
                    }
                    Rvalue::Aggregate(kind, fields) => {
                        let AggregateKind::Adt(adt, variant, ..) = **kind else {
                            return false;
                        };
                        if fields.iter().filter(|f| reads_any(f, &[local])).count() != 1 {
                            return false;
                        }
                        let Some(field) = fields
                            .iter()
                            .position(|f| matches!(f, Operand::Move(_)) && whole(f) == Some(local))
                        else {
                            return false;
                        };
                        // `Some(on_all_ready)`, given on.
                        if tcx.is_lang_item(adt, rustc_hir::attrs::lang_items::LangItem::Option) {
                            if variant.as_u32() != 1 {
                                return false;
                            }
                            held.push(to.local);
                            continue;
                        }
                        if !tcx.adt_def(adt).is_struct() || !field_kept(adt, FieldIdx::from_usize(field)) {
                            return false;
                        }
                        values.push(to.local);
                    }
                    _ => return false,
                }
                continue;
            }
            let Some((id, generics, args, to)) = call_at(tcx, body, at) else {
                return false;
            };
            let Some(given) = args.iter().position(|a| reads_any(a, &[local])) else {
                return false;
            };
            if args.iter().filter(|a| reads_any(a, &[local])).count() != 1
                || !matches!(args[given], Operand::Move(_))
                || whole(args[given]) != Some(local)
            {
                return false;
            }
            match handle_step(tcx, id, generics) {
                // Its payload, or what's made in its place.
                Some(HandleStep::Unwrap | HandleStep::UnwrapOr) if given == 0 => held.push(to),
                _ if tcx.generics_of(id).count() == 0 && by(id, given as u32) => {
                    if to == mir::RETURN_PLACE {
                        returned.insert(at);
                    } else {
                        rcs.push(to);
                    }
                }
                _ => return false,
            }
        }
    }
    // Each struct made of it, given `Rc::new` alone.
    for &value in &values {
        if uses.def(value).is_none() {
            return false;
        }
        for &at in uses.uses(value) {
            if drops(body, at, value) {
                continue;
            }
            match call_at(tcx, body, at) {
                Some((id, generics, args, to))
                    if handle_step(tcx, id, generics) == Some(HandleStep::RcNew)
                        && args.len() == 1
                        && whole(args[0]) == Some(value) =>
                {
                    if to == mir::RETURN_PLACE {
                        returned.insert(at);
                    } else {
                        rcs.push(to);
                    }
                }
                _ => return false,
            }
        }
    }
    // Each `Rc` of it moved, borrowed shared and dropped, until returned.
    let mut i = 0;
    while i < rcs.len() {
        let rc = rcs[i];
        i += 1;
        if uses.def(rc).is_none() {
            return false;
        }
        for &at in uses.uses(rc) {
            if drops(body, at, rc) {
                continue;
            }
            let Some((to, rvalue)) = assign_at(body, at) else {
                return false;
            };
            if !to.projection.is_empty() {
                return false;
            }
            match rvalue {
                Rvalue::Use(operand @ Operand::Move(_), ..) if whole(operand) == Some(rc) => {
                    if to.local == mir::RETURN_PLACE {
                        returned.insert(at);
                    } else {
                        rcs.push(to.local);
                    }
                }
                Rvalue::Ref(_, mir::BorrowKind::Shared | mir::BorrowKind::Fake(_), p)
                    if p.local == rc && p.projection.is_empty() => {}
                _ => return false,
            }
        }
    }
    // What's returned, on each path, is one of them.
    let defs: Vec<Location> = (uses.defs_of(mir::RETURN_PLACE).iter())
        .copied()
        .filter(|at| !body.basic_blocks[at.block].is_cleanup)
        .collect();
    !defs.is_empty() && defs.iter().all(|at| returned.contains(at))
}
