//! What generic code needs of its type parameters (ADR 1115): one JS body
//! for every instance of a generic function, where Rust writes one each,
//! so wherever JS's form of a value, or what std does with it, depends on
//! `T`, the one body learns it by one of three routes:
//!
//! - **Given**: at run time, a parameter or a dictionary its caller passes,
//!   or the value itself, which tells generic code and its caller alike:
//!   `$some(x)` boxes only what looks like `None`.
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
//! | a raw pointer whose pointee is a type parameter, a handle (ADR 1114), and a `&mut` of one, generic code's box (ADRs 0454, 1086), `Form`'s | Form | `src/lower/generic_facts.rs form_given` |
//! | a `Some` of it, boxed or not, by its value, `$some(x)`'s (ADR 0051) | Given | `src/lower/representation/options.rs boxed_payload` |
//! | an iterator or a function given as it is, not boxed (#992) | Form | `src/lower/iterators.rs given_as_it_is` |
//! | whether an `Rc` of it is counted (#1009) | Instances | `src/lower/analysis/rc_counts.rs counted_rcs` |
//! | whether a clone of it is a copy apart (#1119) | Instances | `src/lower/analysis/clones.rs instances_of` |
//! | where an object of it is stored (#1114) | Instances | `src/lower/analysis/stored_places.rs instances` |
//! | whether every caller's `Some` of it is unboxed (ADR 0051, amended) | Instances | `src/lower/analysis/mutation.rs unboxed_params` |
//! | what JS gives a function JS calls (ADR 0051, amended) | Instances | `src/lower/instances.rs given_calls_of` |
//! | what std decides by its type, `eq` of `TrustedLen`s, a zip's fold (ADR 1014) | Instances | `src/lower/analysis/callers_decide.rs refuse_what_callers_decide` |

use super::FnCx;
use crate::js::Expr;
use rustc_middle::ty::{self, Mutability, Ty, TypeVisitableExt};
use rustc_span::def_id::DefId;

/// Where a value crosses into generic code or back: the generic function,
/// whose bounds say what its type parameters are, a type of its signature
/// as it's written, and its caller's type for it.
pub(in crate::lower) type Crossing<'tcx> = (DefId, Ty<'tcx>, Ty<'tcx>);

/// A value's form that generic code has one of, whatever its instance, and
/// its caller adapts at the boundary (ADR 1115).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum Form {
    /// A raw pointer whose pointee is a type parameter: a handle on its
    /// place, whose `value` is the referent, as ADR 0454's box is (ADR
    /// 1114). A caller whose pointer is the value itself, a slice's, a pair
    /// `dyn`'s or a JS object's, adapts it.
    Handle,
    /// A `&mut` whose pointee is a type parameter, or a type parameter
    /// given one, past shared references: generic code's box, whose `value`
    /// is the referent (ADRs 0099, 0454). A caller whose `&mut` is the
    /// object, an object's or a slice's, gives an in-place cell of it,
    /// `$cellOf(object)`, whose write changes it in place, a direct call's
    /// as a value's.
    Box,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether a value of generic code's `declared` type, `def_id`'s, its
    /// caller's type for it `instantiated`, has another form there than the
    /// caller's, adapted where it's given and given back.
    pub(in crate::lower) fn form_differs(&self, form: Form, (def_id, declared, instantiated): Crossing<'tcx>) -> bool {
        match form {
            Form::Handle => {
                let pointee = |ty| super::pointers::raw_pointee(self.tcx, ty).map(|(pointee, _)| pointee);
                matches!(pointee(declared).map(|p| *p.kind()), Some(ty::Param(_)))
                    && pointee(instantiated).is_some_and(|p| self.pointee_is_itself(p) || self.is_js_object(p))
            }
            // Through shared references, a `&mut` to its type parameter,
            // `&mut T` of `&mut v`, or its type parameter given one, `D` of
            // `&mut v`, an impl's for `&mut T` too, not one that takes a
            // function or an iterator as it is (`given_as_the_value`); not
            // of a `&mut` to a number or text, which its caller has as a cell
            // already.
            Form::Box => {
                let (mut declared, mut instantiated) = (declared, instantiated);
                while let (ty::Ref(_, d, Mutability::Not), ty::Ref(_, i, Mutability::Not)) =
                    (*declared.kind(), *instantiated.kind())
                {
                    (declared, instantiated) = (d, i);
                }
                self.declares_box(def_id, declared)
                    && !self.given_as_the_value(def_id, declared, instantiated)
                    && matches!(*instantiated.kind(),
                        ty::Ref(_, pointee, Mutability::Mut) if !self.mut_ref_form(pointee).is_cell())
            }
        }
    }

    /// Whether `declared`, a type of generic code's, `def_id`'s, is, past
    /// shared references, a `&mut` of its type parameter or a type
    /// parameter, which it has as its box where its caller gives a `&mut`
    /// to an object (`Form::Box`): of its own types alone, so a caller asks
    /// it before its instance's, which a type of its own parameters has
    /// none of.
    pub(in crate::lower) fn declares_box(&self, def_id: DefId, mut declared: Ty<'tcx>) -> bool {
        while let ty::Ref(_, inner, Mutability::Not) = *declared.kind() {
            declared = inner;
        }
        let param_env = self.tcx.param_env(def_id);
        match *declared.kind() {
            ty::Ref(..) => self.is_generic_box_in(declared, param_env),
            ty::Param(_) => self.mut_ref_form_in(declared, param_env).is_generic_box(),
            _ => false,
        }
    }

    /// Whether `ty`, a type of generic code's whose type parameters
    /// `param_env` says what they're bound by, is a `&mut` of its type
    /// parameter, `&mut T`, that it has as its box, whatever `T` is
    /// (`Form::Box`): not one bound by a function trait or `Iterator`,
    /// which is the value itself (ADR 0061).
    pub(in crate::lower) fn is_generic_box_in(&self, ty: Ty<'tcx>, param_env: ty::ParamEnv<'tcx>) -> bool {
        matches!(*ty.kind(), ty::Ref(_, pointee, Mutability::Mut)
            if self.mut_ref_form_in(pointee, param_env).is_generic_box())
    }

    /// The form generic code has of `declared` where its caller's type is
    /// `instantiated`, if it's another than the caller's.
    pub(in crate::lower) fn form_of(&self, crossing: Crossing<'tcx>) -> Option<Form> {
        [Form::Handle, Form::Box]
            .into_iter()
            .find(|&form| self.form_differs(form, crossing))
    }

    /// A generic function's instance `def_id` of `args` as a `fn` pointer,
    /// `function` its value, of its own type `fn_def_ty`, which takes
    /// generic code's forms: a function of its own, which takes its type's, as its type's
    /// other functions do, and gives each in generic code's form, a slice's
    /// pointer `$ptrTo(arg0)`, an object's `&mut` `$cellOf(arg0)`, and what
    /// it gives back in its type's, `.value` (#1187). One for the instance,
    /// its module's `const`, so two pointers to it are one, as a generic
    /// function's value is (ADR 0395); of an instance of the caller's own
    /// type parameters, the arrow made there.
    pub(in crate::lower) fn reified_pointer(
        &mut self,
        (def_id, args): (DefId, ty::GenericArgsRef<'tcx>),
        (function, fn_def_ty): (Expr, Ty<'tcx>),
    ) -> super::R<Expr> {
        let declared = self
            .tcx
            .fn_sig(def_id)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder();
        let inputs: Vec<Crossing<'tcx>> = (declared.inputs().iter())
            .map(|&input| (def_id, input, self.instantiated(input, args)))
            .collect();
        let output = (def_id, declared.output(), self.instantiated(declared.output(), args));
        let made = move |cx: &mut Self| -> super::R<Expr> {
            let params: Vec<String> = (0..inputs.len()).map(|i| format!("arg{i}")).collect();
            let given = (params.iter().zip(&inputs))
                .map(|(name, &given)| match cx.form_of(given) {
                    Some(form) => cx.form_given(form, given, Expr::var(name)),
                    None => Expr::var(name),
                })
                .collect();
            let called = Expr::call(function, given);
            let called = match cx.form_of(output) {
                Some(form) => cx.form_back(form, output, called),
                None => called,
            };
            Ok(Expr::arrow(
                params.into_iter().map(Into::into).collect(),
                vec![crate::js::StmtKind::Return(Some(called)).at(crate::js::Span::NONE)],
            ))
        };
        if args.has_non_region_param() || args.has_aliases() {
            return made(self);
        }
        let name = super::camel_case(self.tcx.item_name(def_id).as_str());
        let base = format!("{}Fn", super::traits::instance_name(self.tcx, &name, args));
        self.module_fn_pointer((fn_def_ty, &base), made)
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
    pub(in crate::lower) fn form_given(&self, form: Form, crossing: Crossing<'tcx>, value: Expr) -> Expr {
        if !self.form_differs(form, crossing) {
            return value;
        }
        match form {
            Form::Handle => Expr::call_helper("$ptrTo", vec![value]),
            Form::Box => Expr::call_helper("$cellOf", vec![value]),
        }
    }

    /// `value`, generic code's of its `declared` type, given back where
    /// its caller's type is `instantiated`: in the caller's form, a
    /// handle's `value` (ADR 1114).
    pub(in crate::lower) fn form_back(&self, form: Form, crossing: Crossing<'tcx>, value: Expr) -> Expr {
        if !self.form_differs(form, crossing) {
            return value;
        }
        match form {
            Form::Handle | Form::Box => Expr::held(value),
        }
    }
}
