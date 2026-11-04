//! What a `&mut` to a value is in JS, `MutRefForm`: the one answer every
//! operation on one asks, a whole write, a move out, a copy back, a
//! reborrow, a std call's handle and a declaration's type (ADR 1086). The
//! predicates it's made of are this module's alone: two of them answered
//! one question differently, `Cell::from_mut`'s and a parameter's (#614).

use super::super::FnCx;
use super::super::recognition::{StdItem, is_std_def};
use crate::js::{self, Expr};
use rustc_ast::Mutability;
use rustc_middle::ty::{self, Ty};
use rustc_span::def_id::DefId;
use serde_json::{Value, json};

/// What a `&mut` to a value of some type is in JS (ADR 1086). Each
/// operation on a `&mut` matches it, so a form an operation doesn't say
/// what it does of is a compile error, not a silent choice.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum MutRefForm {
    /// A box, `{ value }`, of a value JS can't change in place: a number,
    /// a `String`, an enum with a fieldless variant, a counted `Rc`, a
    /// flattened state, a `&mut` to a plain object (ADRs 0072, 0074, 0147,
    /// #478). Its `value` is the place.
    Box,
    /// A box, `{ value }`, of a type parameter, which may be a number, as
    /// generic code is compiled once (ADR 0099). Where its caller has an
    /// object, it's given an in-place cell of it, `$cellOf` (ADR 0454).
    GenericBox,
    /// The object itself, which a write of the whole changes in place,
    /// `$assign` (ADRs 0025, 0147).
    Object,
    /// A guard, the object of the cell it guards (ADR 0328): the object,
    /// whose write of the whole in place would write what the cell holds.
    Guard,
    /// A type parameter of the function being lowered given as it is (ADR
    /// 0411): its caller's own value, written in place, `$assign` (ADR 0481).
    GivenAsItIs,
    /// The value itself, which is changed only by calling or stepping it: a
    /// JS function (ADR 0099), an iterator, or a type parameter bound by one
    /// (ADR 0061).
    Itself,
}

impl MutRefForm {
    /// A cell, a box or a handle, `{ value }`, whose `value` is the place
    /// the `&mut` points at (ADR 0099).
    pub(in crate::lower) fn is_cell(self) -> bool {
        match self {
            MutRefForm::Box | MutRefForm::GenericBox => true,
            MutRefForm::Object | MutRefForm::Guard | MutRefForm::GivenAsItIs | MutRefForm::Itself => false,
        }
    }

    /// The `&mut` is the object it points at, a JS object: changes through
    /// it change the one object (ADR 0025).
    pub(in crate::lower) fn is_the_object(self) -> bool {
        match self {
            MutRefForm::Object | MutRefForm::Guard => true,
            MutRefForm::Box | MutRefForm::GenericBox | MutRefForm::GivenAsItIs | MutRefForm::Itself => false,
        }
    }

    /// A write of the whole through the `&mut` changes what it points at in
    /// place, `$assign`: the one answer of `*r = v` and of `mem::replace`
    /// and the like, `$take` (ADRs 0147, 0411, 0553).
    pub(in crate::lower) fn written_in_place(self) -> bool {
        match self {
            MutRefForm::Object | MutRefForm::GivenAsItIs => true,
            MutRefForm::Box | MutRefForm::GenericBox | MutRefForm::Guard | MutRefForm::Itself => false,
        }
    }

    /// The TypeScript type a declaration gives a `&mut` of this form to
    /// what's declared `pointee`: a cell's `{ value: T }`, or the pointee's
    /// own (#619).
    pub(in crate::lower) fn ts_type(self, pointee: Value) -> Value {
        match self.is_cell() {
            true => held_ts(pointee),
            false => pointee,
        }
    }
}

/// `{ value: T }`: what holds a `value` of `ty`, a cell or a box (ADRs
/// 0023, 0074), as its JS reads it, `js::HELD`.
pub(in crate::lower) fn held_ts(ty: Value) -> Value {
    let value = json!({ "kind": "property", "name": js::HELD, "optional": false, "readonly": false, "type": ty });
    json!({ "kind": "object", "members": [value] })
}

/// What lowering's representation makes of the types in an item's
/// signature, asked of a context of that item, as its body is lowered in,
/// so a declaration says what its JS is (#619, #620, ADR 1077, amended):
/// one answer, lowering's own.
pub(in crate::lower) struct SignatureForms<'a, 'tcx> {
    /// What a `&mut` to the type is, in the item's signature: `mut_ref_form`'s
    /// answer, as `param_is_box`'s of a parameter (ADRs 0074, 0099).
    pub(in crate::lower) mut_ref: &'a dyn Fn(DefId, Ty<'tcx>) -> MutRefForm,
    /// The trait a `dyn` of the type is a pair of, `{ value, impl }`, its
    /// value and its impl's dictionary: `dynamic_trait`'s answer (ADR 0049).
    pub(in crate::lower) pair_trait: &'a dyn Fn(DefId, Ty<'tcx>) -> Option<DefId>,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What a `&mut` to a `ty` is, in the function being lowered (ADR 1086).
    pub(in crate::lower) fn mut_ref_form(&self, ty: Ty<'tcx>) -> MutRefForm {
        self.mut_ref_form_in(ty, self.typing_env.param_env)
    }

    /// `mut_ref_form`, of a type of another function's, whose type
    /// parameters `param_env` says what they're bound by: a callee's.
    pub(in crate::lower) fn mut_ref_form_in(&self, ty: Ty<'tcx>, param_env: ty::ParamEnv<'tcx>) -> MutRefForm {
        // Another function's type in its own terms: its projections
        // normalized there, as they can't be here, where its parameters
        // aren't (ADR 1086). And a type only it knows first.
        let ty = match param_env == self.typing_env.param_env {
            true => ty,
            false => {
                let there = ty::TypingEnv::new(param_env, ty::TypingMode::non_body_analysis());
                (self.tcx)
                    .try_normalize_erasing_regions(there, ty::Unnormalized::new_wip(ty))
                    .unwrap_or(ty)
            }
        };
        if self.is_generic_boxed(ty, param_env) {
            MutRefForm::GenericBox
        } else if self.is_boxable(ty, param_env) {
            MutRefForm::Box
        } else if self.is_guard(ty) {
            MutRefForm::Guard
        } else if self.is_object(ty) {
            MutRefForm::Object
        // Given as it is is of the function being lowered's own type
        // parameters, which another function's are not.
        } else if param_env == self.typing_env.param_env && self.iterator_given_as_it_is(ty) {
            MutRefForm::GivenAsItIs
        } else {
            MutRefForm::Itself
        }
    }

    /// Is a `&mut` of a `ty` a handle, `{ value }`, where std gives one to a
    /// closure, `retain_mut`'s, or a guard derefs to one: a cell's, or of a
    /// type only a caller knows, which generic code reads through its
    /// `value`, a box's (ADR 0454). Of a type parameter bound by `FnMut` or
    /// `Iterator`, or given as it is, a handle where `mut_ref_form` says the
    /// value itself (ADR 1086).
    pub(in crate::lower) fn handles_mut(&self, ty: Ty<'tcx>) -> bool {
        self.mut_ref_form(ty).is_cell() || self.is_unknown(ty)
    }

    /// A value JS can't change in place, a `&mut` to which is a box.
    fn is_boxable(&self, ty: Ty<'tcx>, param_env: ty::ParamEnv<'tcx>) -> bool {
        // What an `impl Trait` is, a returned closure's the closure.
        let ty = self.reveal(ty);
        // A `Pin` is its pointer (ADR 0329), and a `&mut` to a pinned
        // reference the pin, which only what it points at is changed through.
        if let Some(pointer) = self.recognition().pinned(ty) {
            return !self.mut_is_the_pin(ty) && self.is_boxable(pointer, param_env);
        }
        // A `&mut` to a value JS can't change in place is a cell, which can't
        // be changed in place either: a `&mut` to it, a cell of the cell. A
        // `&mut` to a plain object is the object, which a write of the whole
        // through a `&mut` to it, `*r = &mut other`, makes another: a cell
        // too, rust_decimal's `impl AddAssign for &mut Decimal`'s `&mut self`.
        // Not of an iterator or a function, each the JS one it steps or calls
        // (ADRs 0071, 0099), nor of a slice, which may be a view of part of
        // an array, split through a held `&mut` as it is.
        if let ty::Ref(_, inner, Mutability::Mut) = *ty.kind() {
            let plain_object = self.is_object(inner)
                && !inner.is_slice()
                && !self.is_callable(inner)
                && !self.implements_iterator(inner)
                && !self.is_generic_iter(inner);
            return self.mut_ref_form_in(inner, param_env).is_cell() || plain_object;
        }
        // A counted `Rc` or its `Weak`, an object its owners share (ADR 0523), or a
        // flattened state, its class's keys (#478): a `&mut` to one is a cell.
        let flattened = matches!(ty.kind(), ty::Adt(adt, _) if self.krate.flattened.states.contains(&adt.did()));
        let shared = self.counted_rc(ty).is_some() || self.counted_weak(ty).is_some() || flattened;
        // Its own function's types in its own terms, another's in that one's.
        let typing_env = match param_env == self.typing_env.param_env {
            true => self.typing_env,
            false => ty::TypingEnv::new(param_env, ty::TypingMode::non_body_analysis()),
        };
        !self.recognition().is_unknown_in(ty, typing_env)
            && (!self.is_object(ty) || shared)
            // A `&mut dyn FnMut()` is the function (ADR 0099).
            && !self.is_callable(ty)
            && self.unsupported_part(ty).is_none()
    }

    /// A type parameter a `&mut` is to, `&mut T` or `&mut Self`, where `param_env`
    /// says what it's bound by: a box, whatever it is (ADR 0099), since generic
    /// code is compiled once and `T` may be a number. Not one that's a JS
    /// function or a JS iterator already: a closure, whose `&mut` is the
    /// closure, or an iterator (ADR 0061).
    fn is_generic_boxed(&self, ty: Ty<'tcx>, param_env: ty::ParamEnv<'tcx>) -> bool {
        let typing_env = ty::TypingEnv::new(param_env, ty::TypingMode::non_body_analysis());
        self.recognition().is_unknown_in(ty, typing_env)
            && !self.declared_bounds(ty, param_env).into_iter().any(|id| {
                self.tcx.fn_trait_kind_from_def_id(id).is_some()
                    || is_std_def(self.tcx, id, StdItem::Iterator)
                    || is_std_def(self.tcx, id, StdItem::IntoIterator)
            })
    }

    /// A `&mut` to a value JS can't change in place, as a value: a cell, a box
    /// or a handle, whose `value` is the place it points at (ADR 0099).
    pub(in crate::lower) fn is_cell(&self, ty: Ty<'tcx>) -> bool {
        matches!(*ty.kind(), ty::Ref(_, inner, Mutability::Mut) if self.mut_ref_form(inner).is_cell())
    }

    /// A value of type `ty` seen through its references, as the code that
    /// shows, compares or orders one reads it: a `&` is the value (ADR 0023),
    /// a `&mut` to a value JS can't change in place a cell, whose `value` it
    /// is (ADR 0099), or the place of a `Handle` on one.
    pub(in crate::lower) fn through_refs(&self, mut value: Expr, mut ty: Ty<'tcx>) -> (Expr, Ty<'tcx>) {
        while let ty::Ref(_, inner, mutability) = *ty.kind() {
            if mutability == Mutability::Mut && self.mut_ref_form(inner).is_cell() {
                value = match value.kind {
                    js::ExprKind::Handle(place, _) => *place,
                    _ => Expr::held(value),
                };
            }
            ty = inner;
        }
        (value, ty)
    }

    /// Is one of the references `ty` is behind a `&mut` cell, so that its JS
    /// value is an object, not what it points at (ADR 0099)?
    pub(in crate::lower) fn has_cell_layer(&self, mut ty: Ty<'tcx>) -> bool {
        while let ty::Ref(_, inner, mutability) = *ty.kind() {
            if mutability == Mutability::Mut && self.mut_ref_form(inner).is_cell() {
                return true;
            }
            ty = inner;
        }
        false
    }

    /// What `fn_id`'s parameter `i` is, a `&mut`'s form in `fn_id`'s own
    /// terms; `None` of a parameter that's no `&mut`.
    pub(in crate::lower) fn param_form(&self, fn_id: DefId, i: usize) -> Option<MutRefForm> {
        let inputs = self
            .tcx
            .fn_sig(fn_id)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder()
            .inputs();
        match inputs.get(i).map(|input| *input.kind()) {
            Some(ty::Ref(_, inner, Mutability::Mut)) => Some(self.mut_ref_form_in(inner, self.tcx.param_env(fn_id))),
            _ => None,
        }
    }

    /// Is `fn_id`'s parameter `i` a box: a `&mut` to a value JS can't change in
    /// place (ADR 0074), or to a type parameter (ADR 0099)?
    pub(in crate::lower) fn param_is_box(&self, fn_id: DefId, i: usize) -> bool {
        self.param_form(fn_id, i).is_some_and(MutRefForm::is_cell)
    }
}
