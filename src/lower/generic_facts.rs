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
//! | a raw pointer whose pointee is a type parameter, a handle (ADR 1114), and a `&mut` of one, generic code's box (ADRs 0454, 1086), and of one given a `&mut`, a box of its box (#1300, #1303), and a `Result` of its type parameters given a write's result, tagged (ADR 0461, amended, #1408), `Form`'s | Form | `src/lower/generic_facts.rs form_given` |
//! | a `Some` of it, boxed or not, by its value, `$some(x)`'s (ADR 0051) | Given | `src/lower/representation/options.rs boxed_payload` |
//! | an iterator or a function given as it is, not boxed (#992) | Form | `src/lower/iterators.rs given_as_it_is` |
//! | an iterator given where generic code steps a JS iterator, adapted at every route to it, a call, a `fn` pointer, a function value (ADR 0061) | Form | `src/lower/iterators/stepping.rs given_iterators` |
//! | whether generic code asks the hint of what it's given, each caller's instance made knowing it, and what a generic caller makes of its own parameters made so in it (ADR 0170, amended) | Instances | `src/lower/analysis/hint_asked.rs mark_hint_knowing` |
//! | whether an `Rc` of it is counted (#1009) | Instances | `src/lower/analysis/rc_counts.rs counted_rcs` |
//! | whether a clone of it is a copy apart (#1119) | Instances | `src/lower/analysis/clones.rs instances_of` |
//! | where an object of it is stored (#1114) | Instances | `src/lower/analysis/stored_places.rs instances` |
//! | whether every caller's `Some` of it is unboxed (ADR 0051, amended) | Instances | `src/lower/analysis/mutation.rs unboxed_params` |
//! | what JS gives a function JS calls (ADR 0051, amended) | Instances | `src/lower/instances.rs given_calls_of` |
//! | what std decides by its type, `eq` of `TrustedLen`s, a zip's fold (ADR 1014) | Instances | `src/lower/analysis/callers_decide.rs refuse_what_callers_decide` |
//! | whether it's shown with a `Formatter`'s options, pretty or in hex, so each caller's type it's given has a writer that takes them (#1331) | Instances | `src/lower/analysis/shown_params.rs shown_params` |
//! | an `Error` impl's `type_id`, which a `dyn Error`'s downcasts compare: none of a generic impl's, a downcast to its type refused (ADR 0331, amended) | Instances | `src/lower/generic_facts.rs error_impl_has_type_id` |
//! | whether a trait of a type built of it can hold for any `T`, a fact no impl answers yes to, `vec::IntoIter<Held<T>>`'s `TrustedRandomAccessNoCoerce`, known here, no caller's (#1385) | Instances | `src/lower/generic_facts.rs may_hold_for_some` |

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
    /// A `&mut` of its type parameter whose caller's is a `&mut` to a
    /// `&mut` to an object, `&mut W` of `&mut by`, `by: &mut Vec<u8>`: the
    /// box of the type parameter's value, which its caller's `Form::Box`
    /// is, the object's in-place cell. Given, a box whose `value` is that
    /// cell, `$heldCellOf(cell)` (#1300); given back, a box whose `value`
    /// is the object, `$heldObjectOf(box)` (#1303), a direct call's as a
    /// function value's, a `fn` pointer's (#1322).
    HeldBox,
    /// A `Result` of generic code's type parameters, `io::Result<T>` or
    /// `Result<(), E>`, whose caller's is a write's result, `fmt::Result` or
    /// `io::Result<()>`: generic code's `Result`, `{ TAG }`, where its
    /// caller's is `undefined` or the error (ADR 0187, ADR 0462). Given,
    /// `$writeTagged(result)`; given back, `$writeOf(result)` (ADR 0461,
    /// amended, #1408).
    Tagged,
    /// A type parameter, `B` of `map<B, F: FnMut(Self::Item) -> B>` or `T`
    /// of `same<T>(t: T) -> T`, whose caller's type is a write's result, as
    /// what a function gives back: generic code holds it as a value,
    /// `undefined` or its error, where a call of the caller's function
    /// throws its error. A function given there gives it back
    /// `$writeTry(() => f(..))`; given back to its caller, its error is
    /// thrown, `$writeCheck(r)`, as a call of one throws (#1410). As an
    /// argument, the same form on both sides.
    Valued,
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
            // Of what each `&mut` is to, the box of a `Form::Box`.
            Form::HeldBox => match (*declared.kind(), *instantiated.kind()) {
                (ty::Ref(_, declared, Mutability::Mut), ty::Ref(_, instantiated, Mutability::Mut)) => {
                    self.form_differs(Form::Box, (def_id, declared, instantiated))
                }
                _ => false,
            },
            // Through shared references, which are the value, a `Result` not
            // a write's whose caller's is one.
            Form::Tagged => {
                let (declared, instantiated) = past_shared_refs(declared, instantiated);
                self.tagged_where(declared, instantiated)
            }
            Form::Valued => valued_where(self.tcx, declared, instantiated),
        }
    }

    /// The form generic code has of what a function gives back, generic
    /// code's `declared` result, its caller's `instantiated`: a value's
    /// (`form_of`), or a write's result held as a value where generic code
    /// holds a type parameter, `Form::Valued` (#1410).
    pub(in crate::lower) fn form_of_result(&self, crossing: Crossing<'tcx>) -> Option<Form> {
        self.form_of(crossing)
            .or_else(|| self.form_differs(Form::Valued, crossing).then_some(Form::Valued))
    }

    /// Whether `declared`, a type of generic code's, is a `Result` it has as
    /// `{ TAG }` where its caller's, `instantiated`, is a write's result:
    /// `Form::Tagged`'s place.
    fn tagged_where(&self, declared: Ty<'tcx>, instantiated: Ty<'tcx>) -> bool {
        let generic = !self.is_write_result(declared) && self.is_write_result(instantiated);
        generic && self.recognition().is_result(declared)
    }

    /// Where std's code given an item of type `item` reads it as a `Result`
    /// of its type parameters, a collect into a `Result`'s: the crossing,
    /// of std's `Result`, its `Result<T, E>` and `item`, where it's a write's
    /// result, `Form::Tagged`'s, as a generic function's is (#1410).
    pub(in crate::lower) fn std_reads_tagged(&self, item: Ty<'tcx>) -> Option<Crossing<'tcx>> {
        let result = crate::lower::recognition::std_item(self.tcx, crate::lower::recognition::StdItem::Result);
        let declared = self.tcx.type_of(result).instantiate_identity().skip_normalization();
        let crossing = (result, declared, item);
        self.form_differs(Form::Tagged, crossing).then_some(crossing)
    }

    /// Whether a write's result, of the caller's `instantiated` type, is
    /// held inside generic code's `declared` type where generic code has a
    /// `Result` of its type parameters, past where a value's form is
    /// adapted (`Form::Tagged`'s, at the top, past shared references): in a
    /// `Vec<Result<T, E>>`, an `Option` of one or a `&mut` to one, whose
    /// items generic code reads as `{ TAG }` and its caller holds as
    /// `undefined` or the error, refused (#1416). A type parameter holds
    /// what it's given as it is, whatever it is.
    pub(in crate::lower) fn write_result_inside(
        &self,
        (_, declared, instantiated): Crossing<'tcx>,
    ) -> Option<Ty<'tcx>> {
        let (declared, instantiated) = past_shared_refs(declared, instantiated);
        if self.tagged_where(declared, instantiated) {
            return None;
        }
        self.tagged_below(declared, instantiated)
    }

    /// Whether a write's result, the caller's `instantiated`, is given for a
    /// type parameter, `declared` past shared references, that a bound of
    /// `def_id`'s reads as a `Result` of its type parameters, `r: &impl
    /// AsRef<Result<T, E>>`: generic code reads it as `{ TAG }` through its
    /// bound, where it has the value as it is, refused (#1416).
    pub(in crate::lower) fn write_result_through_bound(
        &self,
        (def_id, declared, instantiated): Crossing<'tcx>,
    ) -> bool {
        let (declared, instantiated) = past_shared_refs(declared, instantiated);
        if !matches!(declared.kind(), ty::Param(_)) || !self.is_write_result(instantiated) {
            return false;
        }
        let generic_result = |ty: Ty<'tcx>| {
            ty.walk()
                .filter_map(|part| part.as_type())
                .any(|part| self.recognition().is_result(part) && part.has_param() && !self.is_write_result(part))
        };
        (self.tcx.clauses_of(def_id).instantiate_identity(self.tcx).into_iter())
            .map(|(clause, _)| clause.skip_normalization())
            .filter_map(|clause| clause.as_trait_clause())
            .map(|bound| bound.skip_binder().trait_ref)
            .any(|tr| tr.self_ty() == declared && tr.args.types().skip(1).any(generic_result))
    }

    /// The write's result of `instantiated` held where `declared` has a
    /// `Result` of its type parameters, at or below the top.
    fn tagged_below(&self, declared: Ty<'tcx>, instantiated: Ty<'tcx>) -> Option<Ty<'tcx>> {
        if self.tagged_where(declared, instantiated) {
            return Some(instantiated);
        }
        let parts = |ty: Ty<'tcx>| -> Vec<Ty<'tcx>> {
            match *ty.kind() {
                ty::Adt(_, args) => args.types().collect(),
                ty::Tuple(items) => items.to_vec(),
                ty::Ref(_, inner, _) | ty::Array(inner, _) | ty::Slice(inner) => vec![inner],
                _ => Vec::new(),
            }
        };
        let same_shape = match (*declared.kind(), *instantiated.kind()) {
            (ty::Adt(d, _), ty::Adt(i, _)) => d.did() == i.did(),
            (ty::Tuple(_), ty::Tuple(_))
            | (ty::Ref(..), ty::Ref(..))
            | (ty::Array(..), ty::Array(..))
            | (ty::Slice(_), ty::Slice(_)) => true,
            _ => false,
        };
        if !same_shape {
            return None;
        }
        (parts(declared).into_iter().zip(parts(instantiated))).find_map(|(d, i)| self.tagged_below(d, i))
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

    /// How a dictionary's entry reads what a type parameter given a `ty`
    /// holds: of a `&mut`, generic code's box, an object's in-place cell
    /// (`Form::Box`) or its caller's cell of a number, whose `value` it is
    /// to; else the value itself, a `&mut` to a function or an iterator's
    /// too, given as it is (`given_as_the_value`, #1414).
    pub(in crate::lower) fn held_in_generic_box(&self, ty: Ty<'tcx>) -> fn(Expr) -> Expr {
        match self.in_generic_box(ty) {
            true => Expr::held,
            false => |value| value,
        }
    }

    /// Whether a type parameter given a `ty` holds it as generic code's box,
    /// whose `value` it's to: a `&mut`, but to a function or an iterator,
    /// which may be given as it is (`given_as_the_value`, ADR 0061), or to
    /// a `dyn`, whose pair it is (ADR 0099).
    pub(in crate::lower) fn in_generic_box(&self, ty: Ty<'tcx>) -> bool {
        matches!(*ty.kind(), ty::Ref(_, pointee, Mutability::Mut)
            if !self.is_callable(pointee) && !self.implements_iterator(pointee) && !pointee.is_trait())
    }

    /// The form generic code has of `declared` where its caller's type is
    /// `instantiated`, if it's another than the caller's.
    pub(in crate::lower) fn form_of(&self, crossing: Crossing<'tcx>) -> Option<Form> {
        let form = [Form::Handle, Form::Box, Form::HeldBox, Form::Tagged]
            .into_iter()
            .find(|&form| self.form_differs(form, crossing));
        let answer = match form {
            None => "as-it-is",
            Some(Form::Handle) => "handle",
            Some(Form::Box) => "box",
            Some(Form::HeldBox) => "held-box",
            Some(Form::Tagged) => "tagged",
            Some(Form::Valued) => "valued",
        };
        crate::decisions::record("generic-form", answer);
        form
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
        self.reified_const((def_id, args), fn_def_ty, made)
    }

    /// A generic function's instance `def_id` of `args` as a function value
    /// in its callers' forms, `made`: its module's `const`, so two pointers
    /// to it are one, as a generic function's value is (ADR 0395); of an
    /// instance of the caller's own type parameters, the arrow made there.
    pub(in crate::lower) fn reified_const(
        &mut self,
        (def_id, args): (DefId, ty::GenericArgsRef<'tcx>),
        fn_def_ty: Ty<'tcx>,
        made: impl FnOnce(&mut Self) -> super::R<Expr>,
    ) -> super::R<Expr> {
        if args.has_non_region_param() || args.has_aliases() {
            return made(self);
        }
        let name = super::camel_case(self.tcx.item_name(def_id).as_str());
        let base = format!("{}Fn", super::traits::instance_name(self.tcx, &name, args));
        self.module_fn_pointer((fn_def_ty, &base), made)
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
            Form::HeldBox => Expr::call_helper("$heldCellOf", vec![value]),
            Form::Tagged => Expr::call_helper("$writeTagged", vec![value]),
            Form::Valued => value,
        }
    }

    /// `value`, generic code's of its `declared` type, given back where
    /// its caller's type is `instantiated`: in the caller's form, a
    /// handle's `value` (ADR 1114), a held box's box of the object
    /// (#1303).
    pub(in crate::lower) fn form_back(&self, form: Form, crossing: Crossing<'tcx>, value: Expr) -> Expr {
        if !self.form_differs(form, crossing) {
            return value;
        }
        match form {
            Form::Handle | Form::Box => Expr::held(value),
            Form::HeldBox => Expr::call_helper("$heldObjectOf", vec![value]),
            Form::Tagged => Expr::call_helper("$writeOf", vec![value]),
            Form::Valued => Expr::call_helper("$writeCheck", vec![value]),
        }
    }
}

/// Whether generic code's `declared` type, a type parameter's or an
/// associated type's, holds as a value what its caller has as a write's
/// result, `instantiated`, which a call of the caller's function throws:
/// `Form::Valued`'s place, which the analysis of what fails asks too
/// (#1410).
pub(in crate::lower) fn valued_where<'tcx>(
    tcx: rustc_middle::ty::TyCtxt<'tcx>,
    declared: Ty<'tcx>,
    instantiated: Ty<'tcx>,
) -> bool {
    crate::lower::recognition::known_by_bounds(declared)
        && crate::lower::recognition::is_write_result_type(tcx, instantiated)
}

/// What's refused of a write's result held inside what generic code has as
/// a `Result` of its type parameters (#1416).
pub(in crate::lower) fn held_write_result(held: Ty<'_>) -> String {
    format!(
        "a write's result, `{held}`, held inside what generic code has as a `Result` of its type parameters, `{{ TAG }}` there (#1416)"
    )
}

/// `declared` and `instantiated` past the shared references both have.
fn past_shared_refs<'tcx>(mut declared: Ty<'tcx>, mut instantiated: Ty<'tcx>) -> (Ty<'tcx>, Ty<'tcx>) {
    while let (ty::Ref(_, d, Mutability::Not), ty::Ref(_, i, Mutability::Not)) =
        (*declared.kind(), *instantiated.kind())
    {
        (declared, instantiated) = (d, i);
    }
    (declared, instantiated)
}

/// Whether `imp`, an `Error` impl, gives its dictionary its type's
/// `TypeId`: one of a type of no type parameter. A generic one's, whose key
/// its caller's types would make, gives none, and a downcast to a type it's
/// of is refused (ADR 0331, amended).
pub(in crate::lower) fn error_impl_has_type_id(tcx: rustc_middle::ty::TyCtxt<'_>, imp: DefId) -> bool {
    !tcx.type_of(imp).instantiate_identity().skip_normalization().has_param()
}

/// Whether any types given for `ty`'s type parameters, and values for its
/// const parameters, may make `trait_id` hold of it, each an unknown: of `vec::IntoIter<T>`'s
/// `TrustedRandomAccessNoCoerce`, whose impl asks `T: Copy`, yes; of
/// `vec::IntoIter<MultiProductIter<I>>`'s, no, as no impl makes
/// `MultiProductIter<_>` `Copy`, whatever `I` is (#1385).
pub(in crate::lower) fn may_hold_for_some<'tcx>(
    tcx: rustc_middle::ty::TyCtxt<'tcx>,
    ty: Ty<'tcx>,
    trait_id: DefId,
) -> bool {
    use rustc_middle::ty::TypeFoldable;
    use rustc_trait_selection::infer::TyCtxtInferExt;
    use rustc_trait_selection::traits::query::evaluate_obligation::InferCtxtExt;
    let infcx = tcx.infer_ctxt().build(ty::TypingMode::non_body_analysis());
    let mut unknown = ty::BottomUpFolder {
        tcx,
        ty_op: |t: Ty<'tcx>| match t.kind() {
            ty::Param(_) => infcx.next_ty_var(rustc_span::DUMMY_SP),
            _ => t,
        },
        lt_op: |r| r,
        ct_op: |c: ty::Const<'tcx>| match c.kind() {
            ty::ConstKind::Param(_) => infcx.next_const_var(rustc_span::DUMMY_SP),
            _ => c,
        },
    };
    let ty = tcx.erase_and_anonymize_regions(ty).fold_with(&mut unknown);
    let obligation = rustc_trait_selection::traits::Obligation::new(
        tcx,
        rustc_trait_selection::traits::ObligationCause::dummy(),
        ty::ParamEnv::empty(),
        ty::TraitRef::new(tcx, trait_id, [ty]),
    );
    infcx.predicate_may_hold(&obligation)
}

/// Whether `ty` is a type parameter itself, as decision coverage counts a
/// typed answer of one apart (`recognition::kind_of`, ADR 1124).
pub(in crate::lower) fn is_type_parameter(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Param(_))
}
