//! An `Option`'s payload: what its `T` is in JS, and whether a `Some` of it
//! is boxed, as one of a value that may look like `None` is (ADR 0051,
//! amended).

use super::super::FnCx;
use super::super::bindings;
use rustc_middle::ty;
use rustc_middle::ty::Ty;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What an `Option<T>`'s `T` is in JS: through references, `Box` and `Rc`,
    /// which are the value itself (ADR 0023), but for a counted `Rc`, an
    /// object of its own (ADR 0320), and a `&mut` that's a cell, a handle
    /// on its place (ADR 0099), even of a generic `T`.
    fn payload(&self, mut ty: Ty<'tcx>) -> Ty<'tcx> {
        loop {
            ty = match ty.kind() {
                _ if self.is_cell(ty) => return ty,
                ty::Ref(_, inner, _) => *inner,
                ty::Adt(_, args) if ty.is_box() || self.is_rc(ty) && self.counted_rc(ty).is_none() => args.type_at(0),
                ty::Adt(_, args) if self.recognition().pinned(ty).is_some() => args.type_at(0),
                _ => return ty,
            };
        }
    }

    /// An `Option<T>` whose `T` might look like `None`: a type parameter, or
    /// a `()` or an `Option` itself. `Some` of it is boxed when it does
    /// (ADR 0051).
    pub(in crate::lower) fn boxed_payload(&self, ty: Ty<'tcx>) -> bool {
        let payload = self.payload(ty);
        if self.js_given(payload) {
            return false;
        }
        (self.is_unknown(payload) && !self.defined_by_its_bounds(payload)) || self.can_be_nullish(ty)
    }

    /// Of an `Option<Option<T>>`, or an `Option<&Option<T>>`, whose `Some`
    /// is boxed (`boxed_payload`) and whose `T` never looks like `None`, so
    /// a `Some` of `T` is `T` itself: its `T`. A local MIR makes and reads
    /// only where it's held is its payload then (`OptionWrites::nested`):
    /// its `None` `undefined`, its `Some(None)` `null` and its
    /// `Some(Some(v))` `v` (ADR 1017).
    pub(in crate::lower) fn nested_payload(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let some = self.option_of(ty)?;
        if self.is_cell(some) || !self.boxed_payload(some) {
            return None;
        }
        let held = match some.kind() {
            ty::Ref(_, held, ty::Mutability::Not) => *held,
            _ => some,
        };
        let payload = self.option_of(held)?;
        (!self.boxed_payload(payload)).then_some(payload)
    }

    /// Is `ty` a type parameter JS gives, of a function JS calls, or of a
    /// copy of one its values reach: any JS value, `undefined` too, whose
    /// `Some` is the value, as TypeScript's `T | undefined` says (ADR 0051,
    /// amended)? Not one a function trait bounds, a JS function whatever it
    /// is, nor one a binding of its bound reads.
    fn js_given(&self, ty: Ty<'tcx>) -> bool {
        let ty::Param(param) = ty.kind() else {
            return false;
        };
        (self.lowered_params()).is_some_and(|params| params.js.contains(&param.index))
            && !self.read_by_its_bounds(ty)
            && !self.is_callable(ty)
    }

    /// What the lowering of the body being lowered takes its type
    /// parameters as, its own JS or a copy of it (`analysis::JsGiven`): of
    /// a nested body of its own, a closure's, too, but not of a trait's
    /// default copied into an impl, whose are its own.
    pub(in crate::lower) fn lowered_params(&self) -> Option<super::super::analysis::Params> {
        let owner = self.tcx.typeck_root_def_id(self.frame.owner);
        (owner == self.tcx.typeck_root_def_id(self.item))
            .then(|| self.krate.given.js_given.params(self.tcx, owner, self.copy))
    }

    /// Is `ty` a type parameter never `undefined` or `null`, so `Some` of
    /// one is the value itself? One a binding of its bound reads,
    /// `node.sortIndex` of `T: Node`, as the binding takes on trust (ADR
    /// 0307); one bound by `FnOnce`, `FnMut` or `Fn`, a JS function whatever
    /// it is; or one no call gives what may look like `None`, whose JS
    /// caller is given the value it gives (ADR 0051, amended).
    pub(in crate::lower) fn defined_by_its_bounds(&self, ty: Ty<'tcx>) -> bool {
        let ty::Param(param) = ty.kind() else {
            return false;
        };
        let unboxed = match self.lowered_params() {
            Some(params) => params.unboxed.contains(&param.index),
            None => {
                let owner = self.tcx.typeck_root_def_id(self.frame.owner);
                self.krate.given.unboxed.contains(&(owner, param.index))
            }
        };
        self.read_by_its_bounds(ty) || self.is_callable(ty) || unboxed
    }

    fn read_by_its_bounds(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Param(_))
            && self.typing_env.param_env.caller_bounds().iter().any(|clause| {
                clause.as_trait_clause().is_some_and(|bound| {
                    bound.self_ty().skip_binder() == ty
                        && (self.tcx.associated_items(bound.def_id()).in_definition_order())
                            .any(|item| item.is_method() && bindings::is_binding(self.tcx, item.def_id))
                })
            })
    }

    /// Can a `T` be `undefined` or `null` in JS? Then `Option<T>` can't be
    /// `T` itself: `Some(())` and `None` would be the same value.
    pub(in crate::lower) fn can_be_nullish(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.payload(ty);
        // One JS gives may be `undefined` or `null` as it's given.
        self.js_given(ty)
            || (super::super::recognition::known_by_bounds(ty) && !self.defined_by_its_bounds(ty))
            // An uncounted `Weak::new()`, `null` (ADR 0843).
            || self.uncounted_weak(ty).is_some()
            || nullish_by_its_shape(&self.recognition(), ty)
    }
}

/// Whether a value of `ty`, a payload seen through its references, is
/// `undefined` or `null` by its type alone: a `()`, an `Option`, a
/// `MaybeUninit` before it's written (ADR 0332), a unit struct, or an
/// untagged enum of `undefined` or `null`, `js::Kind` (ADR 0676).
fn nullish_by_its_shape<'tcx>(recognition: &super::super::recognition::Recognition<'_, 'tcx>, ty: Ty<'tcx>) -> bool {
    ty.is_unit()
        || recognition.option_of(ty).is_some()
        || recognition.uninit_of(ty).is_some()
        || matches!(ty.kind(), ty::Adt(adt, _) if adt.is_struct() && adt.non_enum_variant().fields.is_empty())
        || matches!(ty.kind(), ty::Adt(adt, _) if super::super::untagged::holds_nullish(recognition.tcx, *adt))
}

/// Whether a type a caller gives generic code may make a value of it there
/// look like `None`, so that a `Some` of it is boxed (ADR 0051, amended):
/// what `can_be_nullish` says of it, seen through every reference, `Box`,
/// `Rc` and `Pin`, and of any `Weak`, which may be `Weak::new()`'s `null`,
/// and a type parameter or an associated type of one, whatever its bounds.
/// Never less than `can_be_nullish`, which knows which `Rc`s are counted.
pub(in crate::lower) fn may_look_like_none<'tcx>(
    recognition: &super::super::recognition::Recognition<'_, 'tcx>,
    ty: Ty<'tcx>,
) -> bool {
    let ty = payload_through(recognition, ty);
    super::super::recognition::known_by_bounds(ty)
        || super::super::recognition::weak_pointee(recognition.tcx, ty).is_some()
        || nullish_by_its_shape(recognition, ty)
}

/// What a value of `ty` is, seen through every reference, `Box`, `Rc` and
/// `Pin`, which are the value itself as an `Option`'s payload, whatever a
/// crate counts (`may_look_like_none`'s).
pub(in crate::lower) fn payload_through<'tcx>(
    recognition: &super::super::recognition::Recognition<'_, 'tcx>,
    mut ty: Ty<'tcx>,
) -> Ty<'tcx> {
    loop {
        ty = match ty.kind() {
            ty::Ref(_, inner, _) => *inner,
            ty::Adt(_, args) if ty.is_box() || recognition.is_rc(ty) || recognition.pinned(ty).is_some() => {
                args.type_at(0)
            }
            _ => return ty,
        };
    }
}
