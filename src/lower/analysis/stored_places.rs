//! The types whose objects say where they're stored (#1113): each object a
//! raw pointer's place passes through or points at, whose address is then
//! in the allocation that holds it, and each type that holds one of those
//! by value, whose own object's address that is. An object of one of them
//! is recorded, `$stored`, where a reference or a pointer to its place is
//! made, so `$addr` follows it to the outermost allocation whichever
//! pointer asks first. A crate that makes no raw pointer records nothing.

use crate::collections::IndexSet;
use crate::lower::Body;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;

/// The types whose objects are recorded where they're stored: by their
/// definitions, a struct's, an enum's or a union's, and whether any is, so
/// a type parameter's value, which may be one, is too.
#[derive(Default)]
pub(in crate::lower) struct StoredPlaces {
    recorded: IndexSet<DefId>,
}

impl StoredPlaces {
    /// Whether an object of `ty` says where it's stored: one of the
    /// recorded types', a tuple or an array that holds one; or a type
    /// parameter's, which may be one, where any is recorded. Not a `Vec`,
    /// whose items are an allocation of their own.
    pub(in crate::lower) fn records(&self, ty: Ty<'_>) -> bool {
        // A tuple or an array, an object of no definition a record could
        // name, is recorded in any crate that records any: its fields'
        // addresses are its container's as a struct's are.
        !self.recorded.is_empty()
            && (matches!(ty.kind(), ty::Tuple(parts) if !parts.is_empty())
                || matches!(ty.kind(), ty::Array(..))
                || holds(ty, &mut |id| self.recorded.contains(&id), true))
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

/// The crate's recorded types: of each raw pointer made of a place, `&raw
/// const o.inner.b`, `&x as *const T`, the type of each object its place
/// passes through and of what it points at; then each struct, enum or
/// union of the crate's or its libraries' that holds one of those by value,
/// in a field of any variant, again till none is added.
pub(in crate::lower) fn stored_places<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[&Body<'tcx>]) -> StoredPlaces {
    // A crate that asks no address, whose pointers are only compared, read
    // and written, `ptr::eq`'s, records nothing: where an object is stored
    // says nothing it asks.
    if !bodies.iter().any(|body| asks_addresses(tcx, body)) {
        return StoredPlaces::default();
    }
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
        return StoredPlaces::default();
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
    StoredPlaces { recorded }
}
