//! Whether what a map's lookup reads of a key can't change while nothing
//! writes the key (#1844): its `Hash`, its `PartialEq` and `Eq`, its
//! `PartialOrd` and `Ord`, and the `Borrow` a lookup reads it through, are
//! each std's or derived, all the way down, over types with no interior
//! mutability reachable through any pointer. A call between a
//! `contains_key(k)` and an `m[k]` can't then change what either reads,
//! so the map still holds `k`. A hand-written impl anywhere is unknown, as
//! it may read a `Cell`, a thread-local or a static; so is a type
//! parameter's, a `dyn`'s, a function's or a raw pointer's.

use super::*;
use rust_js_program::collections::IndexSet;
use rustc_middle::ty::fast_reject::SimplifiedType;

/// Whether a lookup of `ty`, a map's key or what it's asked by, reads the
/// same until the key is written.
pub(crate) fn key_stays<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>) -> bool {
    let mut seen = IndexSet::default();
    stays(tcx, typing_env, ty, &mut seen)
}

fn stays<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    ty: Ty<'tcx>,
    seen: &mut IndexSet<Ty<'tcx>>,
) -> bool {
    // A type met again, of a recursive one: what its other parts say.
    if !seen.insert(ty) {
        return true;
    }
    let parts: Vec<Ty<'tcx>> = match *ty.kind() {
        ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Str | ty::Never => Vec::new(),
        ty::Ref(_, to, _) => vec![to],
        ty::Array(item, _) | ty::Slice(item) => vec![item],
        ty::Tuple(items) => items.to_vec(),
        // std's own impls read what its type arguments hold, of a type that
        // holds no cell itself: `String`, `Vec<T>`, `Box<T>`, `Rc<T>`. Not a
        // `Cell`, a `RefCell`, an atomic or a `Mutex`.
        ty::Adt(adt, args) if is_std_crate(tcx, adt.did()) => {
            if !ty.is_freeze(tcx, typing_env) {
                return false;
            }
            args.types().collect()
        }
        // Another crate's: each impl a lookup reads derived, none of
        // `Borrow`, and each field so.
        ty::Adt(adt, args) => {
            let simplified = SimplifiedType::Adt(adt.did());
            let impls =
                |t: DefId| (tcx.trait_impls_of(t).non_blanket_impls().get(&simplified).cloned()).unwrap_or_default();
            let read = [
                Some(std_item(tcx, StdItem::Hash)),
                tcx.lang_items().eq_trait(),
                Some(std_item(tcx, StdItem::Eq)),
                tcx.lang_items().partial_ord_trait(),
                Some(std_item(tcx, StdItem::Ord)),
            ];
            let derived =
                (read.into_iter().flatten()).all(|t| impls(t).iter().all(|&imp| tcx.is_automatically_derived(imp)));
            if !derived || !impls(std_item(tcx, StdItem::Borrow)).is_empty() {
                return false;
            }
            adt.all_fields().map(|f| f.ty(tcx, args).skip_normalization()).collect()
        }
        _ => return false,
    };
    parts.into_iter().all(|t| stays(tcx, typing_env, t, seen))
}
