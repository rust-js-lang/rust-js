//! The types whose objects say where they're stored (#1113): each object a
//! raw pointer's place passes through or points at, whose address is then
//! in the allocation that holds it, and each type that holds one of those
//! by value, whose own object's address that is. An object of one of them
//! is recorded, `$stored`, where a reference or a pointer to its place is
//! made, so `$addr` follows it to the outermost allocation whichever
//! pointer asks first. A crate that makes no raw pointer records nothing.

use crate::collections::IndexSet;
use crate::lower::Body;
use crate::lower::instances::{Callers, callers_of, instances_of};
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;

/// The types whose objects are recorded where they're stored: by their
/// definitions, a struct's, an enum's or a union's, and whether any is, so
/// a type parameter's value, which may be one, is too.
#[derive(Default)]
pub(in crate::lower) struct StoredPlaces<'tcx> {
    recorded: IndexSet<DefId>,
    /// Whether the crate asks an address (`asks_addresses`).
    asks: bool,
    /// Whether the crate takes a slice's address in any form
    /// (`takes_slice_addresses`), or asks an address.
    slices: bool,
    /// What each raw pointer the crate has points at, whose address it may
    /// ask: a number's, an object's.
    pointees: IndexSet<Ty<'tcx>>,
    /// Each function the crate names and its namings, of a crate that asks
    /// an address: what generic code's instances are (`instances_of`).
    callers: Callers<'tcx>,
}

impl<'tcx> StoredPlaces<'tcx> {
    /// Whether the crate asks an address, `p as usize`, `{:p}`: where
    /// std keeps what it lends inline is then recorded where it lends it
    /// (#1114).
    pub(in crate::lower) fn asks(&self) -> bool {
        self.asks
    }

    /// Whether the crate takes a slice's address in any form, or asks an
    /// address: its own parts of a slice, `&v[a..b]` that JS's
    /// `v.slice(a, b)` would make where it's in bounds, are made by the
    /// runtime's `$part`, which records where each is, as every other
    /// part is recorded (#1185). Of a crate that doesn't, they're JS's own.
    pub(in crate::lower) fn takes_slice_addresses(&self) -> bool {
        self.slices
    }

    /// Whether an address of a `ty` may be asked: one of a recorded type,
    /// or of what a raw pointer the crate has points at (#1114).
    pub(in crate::lower) fn addressable(&self, tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
        self.records(ty) || self.pointees.contains(&tcx.erase_and_anonymize_regions(ty))
    }

    /// What `ty`, which holds `owner`'s type parameters, is in each of the
    /// crate's instances of it, `instances_of`'s: `None` where they can't
    /// all be seen.
    pub(in crate::lower) fn instances(
        &self,
        tcx: TyCtxt<'tcx>,
        ty: Ty<'tcx>,
        owner: DefId,
    ) -> Option<IndexSet<Ty<'tcx>>> {
        instances_of(tcx, &self.callers, ty, owner)
    }

    /// Whether an object of `ty` says where it's stored: one of the
    /// recorded types', a tuple or an array that holds one; or a type
    /// parameter's, which may be one, where any is recorded. Not a `Vec`,
    /// whose items are an allocation of their own.
    pub(in crate::lower) fn records(&self, ty: Ty<'tcx>) -> bool {
        // A tuple or an array, an object of no definition a record could
        // name, is recorded in any crate that records any: its fields'
        // addresses are its container's as a struct's are.
        let records = !self.recorded.is_empty()
            && (matches!(ty.kind(), ty::Tuple(parts) if !parts.is_empty())
                || matches!(ty.kind(), ty::Array(..))
                || holds(ty, &mut |id| self.recorded.contains(&id), true));
        crate::decisions::record("stored-place", if records { "recorded" } else { "unrecorded" });
        records
    }
}

/// Whether a value of `ty` holds, by value, an object of a type `recorded`
/// says, `ty` itself included; a type parameter's or an opaque type's, if
/// `unknown`, as it may.
fn holds<'tcx>(ty: Ty<'tcx>, recorded: &mut impl FnMut(DefId) -> bool, unknown: bool) -> bool {
    match *ty.kind() {
        ty::Adt(adt, _) => recorded(adt.did()),
        ty::Tuple(parts) => parts.iter().any(|part| holds(part, recorded, unknown)),
        ty::Array(item, _) | ty::Slice(item) => holds(item, recorded, unknown),
        ty::Param(_) | ty::Alias(..) => unknown,
        _ => false,
    }
}

/// Whether `body` asks a pointer's address (ADR 0562): a pointer made an
/// integer, `p as usize`, `p.addr()`, `p.expose_provenance()`; the distance
/// between two, `offset_from`; their order, `p < q`, `p.cmp(&q)`; or one
/// shown, `{:p}` and `{:?}` of a pointer, its address.
fn asks_addresses<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> bool {
    let thir = &body.thir;
    thir.exprs.iter().any(|expr| match expr.kind {
        ExprKind::Cast { source } => thir[source].ty.is_raw_ptr() && expr.ty.is_integral(),
        ExprKind::Binary { op, lhs, .. } => {
            matches!(
                op,
                rustc_middle::mir::BinOp::Lt
                    | rustc_middle::mir::BinOp::Le
                    | rustc_middle::mir::BinOp::Gt
                    | rustc_middle::mir::BinOp::Ge
            ) && thir[lhs].ty.is_raw_ptr()
        }
        ExprKind::Call { fun, ref args, .. } => {
            let Some((id, _)) = crate::lower::fn_def(thir[fun].ty) else {
                return false;
            };
            let name = tcx.item_name(id);
            let pointer_arg = args.first().is_some_and(|&arg| thir[arg].ty.peel_refs().is_raw_ptr());
            match name.as_str() {
                "addr"
                | "expose_provenance"
                | "offset_from"
                | "byte_offset_from"
                | "offset_from_unsigned"
                | "cmp"
                | "partial_cmp"
                | "lt"
                | "le"
                | "gt"
                | "ge" => pointer_arg,
                // `{:p}` of anything, `{:?}` of a pointer.
                "new_pointer" => true,
                "new_debug" => pointer_arg,
                _ => false,
            }
        }
        _ => false,
    })
}

/// Whether `body` takes a slice's address in any form (#1185): a raw
/// pointer to a slice or an array, `s as *const [T]`, `ptr::eq(a, b)`'s of
/// two slices; a slice's, an array's or a `Vec`'s `as_ptr()`,
/// `as_mut_ptr()`, or their ranges; or a pointer to an item of one, `&v[1]
/// as *const T`. Not a raw pointer to a `Vec`, `ptr::eq` of two or the one
/// a thread-local's initializer is read through: it's to the vector, not
/// into its items, which only those forms point into.
fn takes_slice_addresses<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> bool {
    let thir = &body.thir;
    let items = |ty: Ty<'tcx>| ty.is_slice() || ty.is_array();
    let of_items = |ty: Ty<'tcx>| {
        let ty = ty.peel_refs();
        items(ty)
            || matches!(ty.kind(), ty::Adt(adt, _) if crate::lower::recognition::is_std_def(tcx, adt.did(), crate::lower::recognition::StdItem::Vec))
    };
    thir.exprs.iter().any(|expr| match expr.kind {
        _ if crate::lower::pointers::raw_pointee(tcx, expr.ty).is_some_and(|(pointee, _)| items(pointee)) => true,
        ExprKind::Call { fun, ref args, .. } => {
            crate::lower::fn_def(thir[fun].ty).is_some_and(|(id, _)| {
                matches!(tcx.item_name(id).as_str(), "as_ptr" | "as_mut_ptr" | "as_ptr_range" | "as_mut_ptr_range")
                    && args.first().is_some_and(|&arg| of_items(thir[arg].ty))
            })
        }
        ExprKind::RawBorrow { arg, .. } | ExprKind::Borrow { arg, .. } if expr.ty.is_raw_ptr() || matches!(expr.kind, ExprKind::RawBorrow { .. }) => {
            matches!(thir[arg].kind, ExprKind::Index { lhs, .. } if of_items(thir[lhs].ty))
        }
        ExprKind::PointerCoercion { source, .. } | ExprKind::Cast { source } if expr.ty.is_raw_ptr() => {
            matches!(thir[source].kind, ExprKind::Borrow { arg, .. } if matches!(thir[arg].kind, ExprKind::Index { lhs, .. } if of_items(thir[lhs].ty)))
        }
        _ => false,
    })
}

/// The crate's recorded types: of each raw pointer made of a place, `&raw
/// const o.inner.b`, `&x as *const T`, the type of each object its place
/// passes through and of what it points at; then each struct, enum or
/// union of the crate's or its libraries' that holds one of those by value,
/// in a field of any variant, again till none is added.
pub(in crate::lower) fn stored_places<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[&Body<'tcx>]) -> StoredPlaces<'tcx> {
    // A crate that asks no address, whose pointers are only compared, read
    // and written, `ptr::eq`'s, records nothing: where an object is stored
    // says nothing it asks.
    let slices = bodies.iter().any(|body| takes_slice_addresses(tcx, body));
    if !bodies.iter().any(|body| asks_addresses(tcx, body)) {
        return StoredPlaces {
            slices,
            ..StoredPlaces::default()
        };
    }
    let callers = callers_of(tcx, bodies);
    let mut pointees: IndexSet<Ty<'tcx>> = IndexSet::default();
    let mut recorded: IndexSet<DefId> = IndexSet::default();
    // Each struct, enum or union type the crate's code has, with its
    // arguments, whose fields say what an object of it holds.
    let mut seen: IndexSet<Ty<'tcx>> = IndexSet::default();
    let note = |ty: Ty<'tcx>, recorded: &mut IndexSet<DefId>| {
        holds(
            ty,
            &mut |id| {
                recorded.insert(id);
                false
            },
            false,
        );
    };
    for body in bodies {
        let thir = &body.thir;
        for expr in thir.exprs.iter() {
            if let Some((pointee, _)) = crate::lower::pointers::raw_pointee(tcx, expr.ty) {
                pointees.insert(tcx.erase_and_anonymize_regions(pointee));
            }
            for part in expr.ty.walk() {
                if let Some(ty) = part.as_type()
                    && let ty::Adt(..) = ty.kind()
                {
                    seen.insert(tcx.erase_and_anonymize_regions(ty));
                }
            }
            let place = match expr.kind {
                ExprKind::RawBorrow { arg, .. } => arg,
                // `&x as *const T`: a reference made a raw pointer.
                ExprKind::PointerCoercion { source, .. } | ExprKind::Cast { source }
                    if expr.ty.is_raw_ptr()
                        && let ExprKind::Borrow { arg, .. } = thir[source].kind =>
                {
                    arg
                }
                _ => continue,
            };
            // What it points at, and each object its place is in, to the
            // first deref, whose referent says where it is where that
            // reference was made.
            let mut at = place;
            loop {
                note(thir[at].ty, &mut recorded);
                at = match thir[at].kind {
                    ExprKind::Field { lhs, .. } | ExprKind::Index { lhs, .. } => lhs,
                    ExprKind::Scope { value, .. } => value,
                    _ => break,
                };
            }
        }
    }
    if recorded.is_empty() {
        return StoredPlaces {
            recorded,
            asks: true,
            slices: true,
            pointees,
            callers,
        };
    }
    // Each type that holds a recorded one by value, of the types its code
    // has, by their fields' types with their arguments: an `Rc<T>`'s
    // and a `Box<T>`'s hold a pointer to theirs, not it.
    let fields = |ty: Ty<'tcx>| -> Vec<Ty<'tcx>> {
        let ty::Adt(adt, args) = *ty.kind() else {
            return Vec::new();
        };
        adt.all_fields()
            .map(|field| tcx.erase_and_anonymize_regions(field.ty(tcx, args).skip_normalization()))
            .collect()
    };
    let mut adts: Vec<Ty<'tcx>> = seen.into_iter().collect();
    let mut at = 0;
    while at < adts.len() && adts.len() < 10_000 {
        for field in fields(adts[at]) {
            for part in field.walk() {
                if let Some(ty) = part.as_type()
                    && let ty::Adt(..) = ty.kind()
                    && !adts.contains(&ty)
                {
                    adts.push(ty);
                }
            }
        }
        at += 1;
    }
    loop {
        let added: IndexSet<DefId> = (adts.iter().copied())
            .filter_map(|ty| match *ty.kind() {
                ty::Adt(adt, _) if !recorded.contains(&adt.did()) => Some((adt.did(), ty)),
                _ => None,
            })
            .filter(|&(_, ty)| {
                fields(ty)
                    .into_iter()
                    .any(|field| holds(field, &mut |inner| recorded.contains(&inner), true))
            })
            .map(|(id, _)| id)
            .collect();
        if added.is_empty() {
            break;
        }
        recorded.extend(added);
    }
    StoredPlaces {
        recorded,
        asks: true,
        slices: true,
        pointees,
        callers,
    }
}
