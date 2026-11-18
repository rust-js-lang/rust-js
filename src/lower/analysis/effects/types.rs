//! What a type says of the pointers a value of it holds and what they may
//! do (#2112 B, ADR 1174): whether it's a pointer and to what, a cell, a
//! scalar, a length that never changes; and what a shared reference to it
//! may change.

use super::*;

/// Whether `ty` is a cell, or a pointer to one however many down, a
/// reference, a `Box`, an `Rc`: what's not `Freeze` past its pointers.
pub(super) fn is_a_cell<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, mut ty: Ty<'tcx>) -> bool {
    for _ in 0..8 {
        match ty.builtin_deref(true).or_else(|| rc_pointee(tcx, ty)) {
            Some(to) => ty = to,
            None => return !ty.is_freeze(tcx, typing_env),
        }
    }
    true
}

/// Whether a value of `ty` is a scalar, or made of scalars alone, which
/// no length is.
pub(super) fn scalar(ty: Ty<'_>) -> bool {
    match ty.kind() {
        ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) => true,
        ty::Tuple(items) => items.iter().all(scalar),
        ty::Array(item, _) => scalar(*item),
        _ => false,
    }
}

/// Whether a value of `ty` has a length no write changes: a slice, an
/// array, a `str`.
pub(super) fn fixed_length(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Slice(_) | ty::Array(..) | ty::Str)
}

/// Whether `ty` is a pointer itself, whose facts are of what it points to:
/// a reference, a raw pointer, a `Box`, an `Rc` or `Arc`, `NonNull`, or
/// std's type that holds a borrow by a lifetime, a guard.
pub(super) fn is_a_pointer<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    ty.is_ref()
        || ty.is_box()
        || raw_pointee(tcx, ty).is_some()
        || rc_pointee(tcx, ty).is_some()
        || matches!(ty.kind(), ty::Adt(adt, args) if is_std_crate(tcx, adt.did()) && args.regions().next().is_some())
}

/// What a pointer of type `ty` points to: a reference's, a raw pointer's,
/// a `Box`'s, an `Rc`'s, or of std's type that holds a borrow by a
/// lifetime, its first type argument, a guard's; none where it can't be
/// said.
pub(super) fn pointee<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    ty.builtin_deref(true)
        .or_else(|| raw_pointee(tcx, ty).map(|(to, _)| to))
        .or_else(|| rc_pointee(tcx, ty))
        .or_else(|| match ty.kind() {
            ty::Adt(adt, args) if is_std_crate(tcx, adt.did()) && args.regions().next().is_some() => {
                args.types().next()
            }
            _ => None,
        })
}

/// Of the pointers a value holds: whether any, whether any may write what
/// it points to, and whether any may replace it.
// Flags, not a state: three facts of the pointers a type holds, each its own.
#[derive(Default)]
pub(super) struct Held {
    pub(super) holds: bool,
    pub(super) writes: bool,
    pub(super) replaces: bool,
}

/// The pointers `ty` holds: a `&mut`, a raw pointer, a `Box`, which may
/// replace what they point to; a shared reference, an `Rc` or `Arc`, which
/// may change only what's inside a cell of it (`changes_through_shared`);
/// std's type that holds a borrow by a lifetime, one that may replace
/// where what it holds is invariant, `RefMut<'_, T>`, as a `&mut T`; or
/// what can't be seen, a `dyn`, a type parameter, an opaque type. Of std's
/// types, what their type arguments hold, a `Vec<&mut u32>`'s; of the
/// crate's, what their fields do; a closure's, its captures'.
pub(super) fn pointers_in<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>) -> Held {
    fn walk<'tcx>(
        tcx: TyCtxt<'tcx>,
        typing_env: ty::TypingEnv<'tcx>,
        ty: Ty<'tcx>,
        seen: &mut Vec<Ty<'tcx>>,
        found: &mut Held,
    ) {
        if seen.contains(&ty) {
            return;
        }
        seen.push(ty);
        let all = |found: &mut Held| {
            *found = Held {
                holds: true,
                writes: true,
                replaces: true,
            }
        };
        match *ty.kind() {
            ty::Bool
            | ty::Char
            | ty::Int(_)
            | ty::Uint(_)
            | ty::Float(_)
            | ty::Str
            | ty::Never
            | ty::FnDef(..)
            | ty::FnPtr(..) => {}
            ty::Ref(_, to, mutability) => {
                found.holds = true;
                found.writes |= mutability.is_mut() || changes_through_shared(tcx, typing_env, to);
                found.replaces |= mutability.is_mut();
                walk(tcx, typing_env, to, seen, found);
            }
            ty::Slice(item) | ty::Array(item, _) => walk(tcx, typing_env, item, seen, found),
            ty::Tuple(items) => items.iter().for_each(|t| walk(tcx, typing_env, t, seen, found)),
            ty::Closure(_, args) => {
                (args.as_closure().upvar_tys().iter()).for_each(|t| walk(tcx, typing_env, t, seen, found))
            }
            ty::Adt(adt, args) if is_std_crate(tcx, adt.did()) => {
                // A raw pointer std's type is, `NonNull<T>`, and a `Box`,
                // whose deref is to what it points to, `&(*items)` a closure
                // captures: each may replace it.
                if raw_pointee(tcx, ty).is_some() || ty.is_box() {
                    all(found);
                }
                // An `Rc`, an `Arc`, which share what they point to.
                if let Some(to) = rc_pointee(tcx, ty) {
                    found.holds = true;
                    found.writes |= changes_through_shared(tcx, typing_env, to);
                }
                // A borrow it holds by a lifetime, a guard's or an iterator's:
                // one that may replace where what it holds is invariant,
                // `RefMut<'_, T>`, `IterMut<'_, T>`, as a `&mut T` is.
                if args.regions().next().is_some() {
                    found.holds = true;
                    let variances = tcx.variances_of(adt.did());
                    let invariant = (args.iter().zip(variances.iter()))
                        .any(|(arg, variance)| arg.as_type().is_some() && *variance == ty::Variance::Invariant);
                    found.writes |= invariant || args.types().any(|t| changes_through_shared(tcx, typing_env, t));
                    found.replaces |= invariant;
                }
                args.types().for_each(|t| walk(tcx, typing_env, t, seen, found))
            }
            ty::Adt(adt, args) => (adt.all_fields())
                .for_each(|field| walk(tcx, typing_env, field.ty(tcx, args).skip_normalization(), seen, found)),
            _ => all(found),
        }
    }
    let mut found = Held::default();
    walk(tcx, typing_env, ty, &mut Vec::new(), &mut found);
    found
}

/// Whether a shared reference to `ty` may change what it refers to:
/// what's not `Freeze`, a `Cell` or a `RefCell`, or holds one, through a
/// pointer too, `Rc<RefCell<_>>`; a raw pointer; or what can't be seen, a
/// type parameter, a `dyn`, an opaque type. Of std's types, what their
/// type arguments are; of the crate's, what their fields are.
pub(in crate::lower) fn changes_through_shared<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    ty: Ty<'tcx>,
) -> bool {
    fn walk<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> bool {
        if seen.contains(&ty) {
            return false;
        }
        seen.push(ty);
        match *ty.kind() {
            ty::Bool
            | ty::Char
            | ty::Int(_)
            | ty::Uint(_)
            | ty::Float(_)
            | ty::Str
            | ty::Never
            | ty::FnDef(..)
            | ty::FnPtr(..) => false,
            ty::Ref(_, pointee, _) | ty::Slice(pointee) | ty::Array(pointee, _) => walk(tcx, typing_env, pointee, seen),
            ty::Tuple(items) => items.iter().any(|t| walk(tcx, typing_env, t, seen)),
            ty::Closure(_, args) => (args.as_closure().upvar_tys().iter()).any(|t| walk(tcx, typing_env, t, seen)),
            ty::Adt(adt, args) => {
                if !ty.is_freeze(tcx, typing_env) || raw_pointee(tcx, ty).is_some() {
                    return true;
                }
                if is_std_crate(tcx, adt.did()) {
                    args.types().any(|t| walk(tcx, typing_env, t, seen))
                } else {
                    (adt.all_fields())
                        .any(|field| walk(tcx, typing_env, field.ty(tcx, args).skip_normalization(), seen))
                }
            }
            _ => true,
        }
    }
    walk(tcx, typing_env, ty, &mut Vec::new())
}
