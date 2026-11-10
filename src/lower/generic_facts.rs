//! What generic code needs of its type parameters (ADR 1115): one JS body
//! for every instance of a generic function, where Rust writes one each,
//! so wherever JS's form of a value, or what std does with it, depends on
//! `T`, the one body learns it by one of three routes:
//!
//! - **Given**: at run time, a parameter or a dictionary its caller passes.
//! - **Form**: one form in generic code, which its caller, knowing `T`,
//!   adapts at the call, `form_given` and `form_back`, here alone.
//! - **Instances**: at compile time, of every caller's instance, carried
//!   through generic callers; refused or compiled apart where they disagree.
//!
//! The catalogue, each fact, its route and the function that answers it,
//! which `test/architecture.test.ts` holds to the code:
//!
//! | Fact | Route | Owner |
//! |---|---|---|
//! | `size_of::<T>()` and the rest, `TypeFact`'s (ADR 0145) | Given | `src/lower/traits/given.rs given_type_fact` |
//! | a bound's methods, its dictionary (ADR 0049) | Given | `src/lower/traits/routes.rs evidence_for` |
//! | a std function's `Iterator` dictionary of what it's given as it is (#1147, #1163) | Given | `src/lower/analysis/iterator_steps.rs stepped_params` |
//! | what dropping one runs, its drop slot (ADR 0421) | Given | `src/lower/traits/drop_params.rs drop_slot` |
//! | a raw pointer whose pointee is a type parameter, a handle (ADR 1114) | Form | `src/lower/generic_facts.rs form_given` |
//! | a `Some` of it, boxed or not (ADR 0051) | Form | `src/lower/representation/options.rs boxed_payload` |
//! | a `&mut` of it, generic code's box (ADRs 0454, 1086), to move onto `form_given` | Form | `src/lower/mut_refs.rs boxed_in_call` |
//! | an iterator or a function given as it is, not boxed (#992) | Form | `src/lower/iterators.rs given_as_it_is` |
//! | whether an `Rc` of it is counted (#1009) | Instances | `src/lower/analysis/rc_counts.rs counted_rcs` |
//! | whether a clone of it is a copy apart (#1119) | Instances | `src/lower/analysis/clones.rs instances_of` |
//! | where an object of it is stored (#1114) | Instances | `src/lower/analysis/stored_places.rs instances` |
//! | whether every caller's `Some` of it is unboxed (ADR 0051, amended) | Instances | `src/lower/analysis/mutation.rs unboxed_params` |
//! | what JS gives a function JS calls (ADR 0051, amended) | Instances | `src/lower/instances.rs given_calls_of` |
//! | what std decides by its type, `eq` of `TrustedLen`s, a zip's fold (ADR 1014) | Instances | `src/lower/analysis/callers_decide.rs refuse_what_callers_decide` |

use super::FnCx;
use crate::js::Expr;
use rustc_middle::ty::{self, Ty};

/// A value's form that generic code has one of, whatever its instance, and
/// its caller adapts at the boundary (ADR 1115).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum Form {
    /// A raw pointer whose pointee is a type parameter: a handle on its
    /// place, whose `value` is the referent, as ADR 0454's box is (ADR
    /// 1114). A caller whose pointer is the value itself, a slice's, a pair
    /// `dyn`'s or a JS object's, adapts it.
    Handle,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether a value of generic code's `declared` type, its caller's type
    /// for it `instantiated`, has another form there than the caller's,
    /// adapted where it's given and given back.
    pub(in crate::lower) fn form_differs(&self, form: Form, declared: Ty<'tcx>, instantiated: Ty<'tcx>) -> bool {
        match form {
            Form::Handle => {
                let pointee = |ty| super::pointers::raw_pointee(self.tcx, ty).map(|(pointee, _)| pointee);
                matches!(pointee(declared).map(|p| *p.kind()), Some(ty::Param(_)))
                    && pointee(instantiated).is_some_and(|p| self.pointee_is_itself(p) || self.is_js_object(p))
            }
        }
    }

    /// Whether `ty`, a type of generic code's, is a raw pointer of its type
    /// parameter, `*mut T`: a handle on its place, whatever `T` is
    /// (`Form::Handle`).
    pub(in crate::lower) fn is_generic_handle(&self, ty: Ty<'tcx>) -> bool {
        super::pointers::raw_pointee(self.tcx, ty).is_some_and(|(pointee, _)| matches!(pointee.kind(), ty::Param(_)))
    }

    /// The referent of a shared reference given where generic code's
    /// `declared` `&T` goes, `instantiated` the caller's type for it, where
    /// JS holds it as itself, a number, so a pointer generic code made of it
    /// would be of a copy (ADR 1114): not an object's or a slice's, whose
    /// pointer is theirs, nor a type only generic callers know.
    pub(in crate::lower) fn copied_referent(&self, declared: Ty<'tcx>, instantiated: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let ty::Ref(_, pointee, ty::Mutability::Not) = *declared.kind() else {
            return None;
        };
        let ty::Param(_) = pointee.kind() else {
            return None;
        };
        let referent = instantiated.builtin_deref(true)?;
        (!self.is_unknown(referent) && !self.is_object(referent) && !self.pointee_is_itself(referent))
            .then_some(referent)
    }

    /// `value`, the caller's, given where generic code's `declared` type
    /// goes, `instantiated` the caller's type for it: in generic code's
    /// form. A slice's pointer, the array, given for a `*mut T` is a handle
    /// on it, `$ptrTo(array)`, whose `value` is it (ADR 1114).
    pub(in crate::lower) fn form_given(
        &self,
        form: Form,
        (declared, instantiated): (Ty<'tcx>, Ty<'tcx>),
        value: Expr,
    ) -> Expr {
        if !self.form_differs(form, declared, instantiated) {
            return value;
        }
        match form {
            Form::Handle => Expr::call_helper("$ptrTo", vec![value]),
        }
    }

    /// `value`, generic code's of its `declared` type, given back where
    /// its caller's type is `instantiated`: in the caller's form, a
    /// handle's `value` (ADR 1114).
    pub(in crate::lower) fn form_back(
        &self,
        form: Form,
        (declared, instantiated): (Ty<'tcx>, Ty<'tcx>),
        value: Expr,
    ) -> Expr {
        if !self.form_differs(form, declared, instantiated) {
            return value;
        }
        match form {
            Form::Handle => Expr::held(value),
        }
    }
}
