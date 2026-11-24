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
//! | `size_of::<T>()` and the rest, `TypeFact`'s (ADR 0145) | Given | `crates/rust-js-lower/src/traits/given.rs given_type_fact` |
//! | the size std grows a `Vec` of it by and checks its room by, `TypeFact::Room`: `undefined` of #934's case alone, an impl's associated type no dictionary carries (#1260) | Given | `crates/rust-js-lower/src/iterators/hint_asks.rs room_value` |
//! | which collects of it check their room upfront, so ask its `Room`: one counted by the program (#1436) | Given | `crates/rust-js-lower/src/recognition/lengths.rs counted_by_program` |
//! | a bound's methods, its dictionary (ADR 0049) | Given | `crates/rust-js-lower/src/traits/routes.rs evidence_for` |
//! | a std function's `Iterator` dictionary of what it's given as it is (#1147, #1163) | Given | `crates/rust-js-lower/src/analysis/iterator_steps.rs stepped_params` |
//! | what dropping one runs, its drop slot (ADR 0421) | Given | `crates/rust-js-lower/src/traits/drop_params.rs drop_slot` |
//! | whether a derived `Clone` or `Default` of a type makes a field of it before another, which it drops as a later one panics, so the derive is given drops (#1517) | Given | `crates/rust-js-lower/src/generic_facts/caller_facts.rs makes_a_param_field_first` |
//! | whether a type holds a function's type parameter, as `repeat_n`'s `RepeatN<T>` holds its `T`, so a std maker of a compiled iterator is given its drop (#1458) | Given | `crates/rust-js-lower/src/generic_facts.rs holds_type_parameter` |
//! | whether a `partition`'s or an `unzip`'s side is a type parameter, its caller's, which std fills by its `Default` and `Extend`, so it's the helper's array as before #2377; filling it by its dictionaries is #2387's | Given | `crates/rust-js-lower/src/generic_facts.rs is_type_parameter` |
//! | whether a type names a type parameter, so what it is isn't known: of a walk for a kept raw pointer, it may be the `&mut`'s pointee or a reference to it, so it may keep it (#1781); of a type a `dyn` in a cell may be, its trait's impls may hold of it, so it may be (#1006); of what a property read reads, a getter of the program's may answer it, each caller's instance settling it (#2109); of a value `CellForm`'s proof finds dropped while a guard's held, each instance's destructor runs, and a generic body's loud cell type is loud of each instance (#2490) | Given | `crates/rust-js-lower/src/generic_facts.rs may_be_any_type` |
//! | whether each parameter a type names, of the body that made it, is another body's own, by index and name, so what its drop runs is asked in that body's env, where a caller's `SIZE` isn't and rustc crashes (#1608) | Instances | `crates/rust-js-lower/src/generic_facts.rs params_owned_by` |
//! | whether what a `Box<I>` holds is stepped by a dictionary its caller gives, which may carry the crate's own `fold` or `count` std's `Box<I>` never calls, so no default passes on to it (#1611) | Given | `crates/rust-js-lower/src/generic_facts/caller_facts.rs boxed_steps_given` |
//! | whether a clone of it is its caller's, which may run any code, so a `cycle()` of an array's items of it clones as std's `Cycle` does (#1630) | Given | `crates/rust-js-lower/src/generic_facts/caller_facts.rs clone_given_by_caller` |
//! | whether a bound's dictionary is read by its function, a closure's or an inline constant's it makes, or a callee it gives it to, so it's given one; of a trait Copy's, Clone's and the iterators' own analyses don't answer for (#1915) | Given | `crates/rust-js-lower/src/analysis/read_bounds.rs unread_bounds` |
//! | a raw pointer whose pointee is a type parameter, a handle (ADR 1114), and a `&mut` of one, generic code's box (ADRs 0454, 1086), and of one given a `&mut`, a box of its box (#1300, #1303), and a `Result` of its type parameters given a write's result, tagged (ADR 0461, amended, #1408), `Form`'s | Form | `crates/rust-js-lower/src/generic_facts.rs form_given` |
//! | a `Some` of it, boxed or not, by its value, `$some(x)`'s (ADR 0051) | Given | `crates/rust-js-lower/src/representation/options.rs boxed_payload` |
//! | an iterator or a function given as it is, not boxed (#992) | Form | `crates/rust-js-lower/src/iterators.rs given_as_it_is` |
//! | whether an iterator may be a `Repeat`, whose own `count` and `last` name their caller: a type parameter's may, so generic code's `count` and `last` are given where they're called (#2324) | Form | `crates/rust-js-lower/src/recognition/own_steps.rs may_be_repeat` |
//! | an iterator given where generic code steps a JS iterator, adapted at every route to it, a call, a `fn` pointer, a function value (ADR 0061) | Form | `crates/rust-js-lower/src/iterators/stepping.rs given_iterators` |
//! | whether generic code asks the hint of what it's given, each caller's instance made knowing it, and what a generic caller makes of its own parameters made so in it (ADR 0170, amended) | Instances | `crates/rust-js-lower/src/analysis/hint_asked.rs mark_hint_knowing` |
//! | whether an `Rc` of it is counted (#1009) | Instances | `crates/rust-js-lower/src/analysis/rc_counts.rs counted_rcs` |
//! | whether a clone of it is a copy apart (#1119) | Instances | `crates/rust-js-lower/src/analysis/clones.rs instances_of` |
//! | whether its `<`, `<=`, `>`, `>=`, `==` and `!=` are JS's own, every caller of a function the crate sees whole giving a JS number, so they read no dictionary (#1874) | Instances | `crates/rust-js-lower/src/analysis/mutation.rs compared_params` |
//! | where an object of it is stored (#1114) | Instances | `crates/rust-js-lower/src/analysis/stored_places.rs instances` |
//! | whether every caller's `Some` of it is unboxed (ADR 0051, amended) | Instances | `crates/rust-js-lower/src/analysis/mutation.rs unboxed_params` |
//! | what JS gives a function JS calls (ADR 0051, amended) | Instances | `crates/rust-js-lower/src/instances.rs given_calls_of` |
//! | whether a closure, or a boxed `Fn`, given for it is called by value, `FnOnce`'s `call_once`, or by `&`, through every generic caller, an impl's own parameter matched in its type (#2031, #2050) | Instances | `crates/rust-js-lower/src/instances.rs on_the_way` |
//! | which `Fn` trait a function value given for it is called through, the most its bounds name of `Fn`, `FnMut` and `FnOnce`, whose shim's location a `#[track_caller]` function made a value is given (#2335) | Given | `crates/rust-js-lower/src/generic_facts.rs called_through` |
//! | what std decides by its type, `eq` of `TrustedLen`s, a zip's fold (ADR 1014) | Instances | `crates/rust-js-lower/src/analysis/callers_decide.rs refuse_what_callers_decide` |
//! | whether it's shown with a `Formatter`'s options, pretty or in hex, so each caller's type it's given has a writer that takes them (#1331) | Instances | `crates/rust-js-lower/src/analysis/shown_params.rs shown_params` |
//! | whether an iterator built of type parameters a `Vec` is collected of is `TrustedLen`, which std's room and its fold of it ask, its parts' facts and its items' `ConstSize` (#1614) | Given | `crates/rust-js-lower/src/generic_facts/caller_facts.rs trusted_len_asked` |
//! | whether a caller's `&mut` to a value is one generic code may write whole, through its box, as a function declares a `&mut` of a type parameter there, `Self` too, which the copy policy asks (#1665) | Instances | `crates/rust-js-lower/src/generic_facts.rs writes_through_its_box` |
//! | an `Error` impl's `type_id`, which a `dyn Error`'s downcasts compare: none of a generic impl's, a downcast to its type refused (ADR 0331, amended) | Instances | `crates/rust-js-lower/src/generic_facts/caller_facts.rs error_impl_has_type_id` |
//! | whether a caller gives it a value with a destructor, its function given a drop for it, `drop_params`' (ADR 0098), what steps its items from their back made closed where it's made (#1406) | Instances | `crates/rust-js-lower/src/generic_facts/caller_facts.rs given_a_drop_of` |
//! | whether a trait of a type built of it can hold for any `T`, a fact no impl answers yes to, `vec::IntoIter<Held<T>>`'s `TrustedRandomAccessNoCoerce`, known here, no caller's (#1385) | Instances | `crates/rust-js-lower/src/generic_facts.rs may_hold_for_some` |
//! | whether a std impl's type, its first argument or a raw pointer's pointee, is the impl's type parameter, `impl<T> NonNull<T>`'s, as a std path names its block apart from `NonNull::<[T]>`'s and `*const [T; N]`'s, so a method is known by whose it is and its name (#1681) | Instances | `crates/rust-js-lower/src/generic_facts.rs is_impl_parameter` |

use super::copies::InPlaceWriter;
use super::{FnCx, R};
use crate::recognition::StdItem;
use rust_js_program::js::Expr;
use rustc_middle::ty::{self, Mutability, Ty, TypeVisitableExt};
use rustc_span::Span;
use rustc_span::def_id::{CRATE_DEF_ID, DefId};

mod caller_facts;
mod refusals;
pub(crate) use caller_facts::{
    GivenADrop, boxed_steps_given, clone_given_by_caller, error_impl_has_type_id, given_a_drop_of,
    makes_a_param_field_first, trusted_len_asked,
};

/// Whether `def` takes a type parameter of its own that its bounds make an
/// `Iterator` or an `IntoIterator`, `from_iter`'s `I`: what a caller gives
/// it to step, not `Hash::hash`'s `H` nor `fold`'s `B` (#2376).
pub(crate) fn takes_an_iterator(tcx: ty::TyCtxt<'_>, def: DefId) -> bool {
    let parents = tcx.generics_of(def).parent_count as u32;
    let iterators = [StdItem::Iterator, StdItem::IntoIterator].map(|item| crate::recognition::std_item(tcx, item));
    (tcx.clauses_of(def).clauses.iter())
        .filter_map(|(clause, _)| clause.as_trait_clause())
        .any(|bound| {
            let bound = bound.skip_binder();
            iterators.contains(&bound.def_id())
                && matches!(*bound.self_ty().kind(), ty::Param(param) if param.index >= parents)
        })
}

/// Which `Fn` trait the argument `at` of a call of `callee` is called
/// through: the most its parameter's bounds name of `Fn`, `FnMut` and
/// `FnOnce`, `Iterator::map`'s `F: FnMut` `FnMut`'s. Its shim's location
/// is the one a `#[track_caller]` function made a value is given (#2335).
/// None where the parameter isn't a type parameter, or names none of them.
pub fn called_through(tcx: ty::TyCtxt<'_>, callee: DefId, at: usize) -> Option<ty::ClosureKind> {
    let sig = tcx.instantiate_bound_regions_with_erased(tcx.fn_sig(callee).instantiate_identity().skip_normalization());
    let param = *sig.inputs().get(at)?;
    if !matches!(param.kind(), ty::Param(_)) {
        return None;
    }
    let kinds: Vec<ty::ClosureKind> = (tcx.param_env(callee).caller_bounds())
        .filter_map(|clause| clause.as_trait_clause())
        .filter(|clause| tcx.instantiate_bound_regions_with_erased(clause.self_ty()) == param)
        .filter_map(|clause| tcx.fn_trait_kind_from_def_id(clause.def_id()))
        .collect();
    [ty::ClosureKind::Fn, ty::ClosureKind::FnMut, ty::ClosureKind::FnOnce]
        .into_iter()
        .find(|kind| kinds.contains(kind))
}

/// Where a value crosses into generic code or back: the generic function,
/// whose bounds say what its type parameters are, a type of its signature
/// as it's written, and its caller's type for it.
pub type Crossing<'tcx> = (DefId, Ty<'tcx>, Ty<'tcx>);

/// A value's form that generic code has one of, whatever its instance, and
/// its caller adapts at the boundary (ADR 1115).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Form {
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
    /// `$writeTry(() => f(..))`; given back to its caller, a call's value,
    /// its error is thrown, `$writeCheck(r)`, as a call of one throws, and
    /// caught again, `fmt_result_value`'s, its one owner (#1410, #1634). As
    /// an argument, the same form on both sides.
    Valued,
    /// A type parameter generic code holds as an iterator, whose `&mut` is
    /// the value itself (ADR 0061), given its caller's iterator that JS
    /// holds as no object, a fieldless enum's string, a unit struct's
    /// `undefined` or a number, whose `&mut` is a cell: generic code holds
    /// it in a place of its own, `{ value: x }`, so its `&mut` is that
    /// place, which the type's own `&mut self` steps take (#2255). Given,
    /// `{ value: x }`; given back, its `value`. A `&mut` of it is its
    /// caller's cell already, generic code's place. Its dictionaries read
    /// the place's `value` where they take it by value or `&` and make one
    /// of what they give back of it (`placed_dictionary`). A tuple of one
    /// taken or given back by value is a tuple of each part so (`Placed`).
    Placed,
}

/// What of a value is in a place of its own where it crosses into generic
/// code or back (`placed_shape`, `Form::Placed`): the value, or the parts
/// of a tuple taken or given by value, each so or as it is.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Placed {
    Itself,
    Parts(Vec<Option<Placed>>),
}

impl Placed {
    /// Its runtime code, as `$inPlace`, `$fromPlace` and the dictionaries
    /// that read and make places take it: `1` of the value, a list of a
    /// tuple's parts, each `0` as it is.
    pub(crate) fn code(&self) -> Expr {
        match self {
            Placed::Itself => Expr::int(1),
            Placed::Parts(parts) => Expr::array(
                (parts.iter())
                    .map(|part| part.as_ref().map_or_else(|| Expr::int(0), Placed::code))
                    .collect(),
            ),
        }
    }

    /// `value` given in this shape: `{ value: x }`, a tuple written here
    /// part by part, `[{ value: a }, b]`, another `$inPlace(t, [1, 0])`.
    fn given(&self, value: Expr) -> Expr {
        let parts = match self {
            Placed::Itself => {
                return Expr::object(vec![rust_js_program::js::Prop::Field(
                    rust_js_program::js::HELD.into(),
                    value,
                )]);
            }
            Placed::Parts(parts) => parts,
        };
        match value.kind {
            rust_js_program::js::ExprKind::Array(items, ..)
                if items.len() == parts.len()
                    && !(items.iter()).any(|item| matches!(item.kind, rust_js_program::js::ExprKind::Spread(_))) =>
            {
                Expr::array(
                    (items.into_iter().zip(parts))
                        .map(|(item, part)| match part {
                            Some(part) => part.given(item),
                            None => item,
                        })
                        .collect(),
                )
            }
            _ => Expr::call_helper("$inPlace", vec![value, self.code()]),
        }
    }

    /// `value`, generic code's in this shape, given back as its caller
    /// holds it: its place's `value`, a tuple's `$fromPlace(t, [1, 0])`.
    fn back(&self, value: Expr) -> Expr {
        match self {
            Placed::Itself => Expr::held(value),
            Placed::Parts(_) => Expr::call_helper("$fromPlace", vec![value, self.code()]),
        }
    }
}

/// How a route's arguments come to `crossing_into`: as their caller has
/// them, `Raw`, each given generic code's form there where its view differs
/// (`form_given`), a dictionary entry's parameter or what a recogniser
/// made; or so already, `Formed`, a call MIR lowered, whose `boxed_args`
/// read each from its place, a trait method's by its impl or dictionary;
/// `FormedOf` the direct call's, of the caller's types `given` (#2264).
#[derive(Clone, Copy)]
pub enum Arguments<'g, 'tcx> {
    Raw,
    Formed,
    FormedOf { given: &'g [Ty<'tcx>] },
}

/// `value`, a call made after `made`, the statements its arguments'
/// crossing made (`crossing_into`): itself where there are none, else an
/// arrow that runs them, then gives it, of a route that writes no
/// statements of its own.
pub(crate) fn made_before(made: Vec<rust_js_program::js::Stmt>, value: Expr) -> Expr {
    if made.is_empty() {
        return value;
    }
    let mut body = made;
    body.push(rust_js_program::js::StmtKind::Return(Some(value)).at(rust_js_program::js::Span::NONE));
    Expr::call(Expr::arrow(Vec::new(), body), Vec::new())
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `values`, the arguments of a call of the crate's `callee` with
    /// `args`, crossing into its generic code by whatever route: each in
    /// generic code's form where its view differs from its caller's
    /// (`form_given`, of `Arguments::Raw` ones), then each iterator of the
    /// crate's, or range, given where generic code steps a JS iterator, made
    /// one (`given_iterators`, ADR 0061). The one owner each route asks: a
    /// direct call, a trait method's through its impl or a dictionary, a
    /// conversion's dictionary, `collect()`'s `from_iter`, an
    /// `IntoIterator`'s `into_iter` and a writer's `fmt`, so none gives the
    /// crate's iterator bare where generic code steps a JS iterator, as a
    /// conversion's dictionary did (#2264).
    pub fn crossing_into(
        &mut self,
        (callee, args): (DefId, ty::GenericArgsRef<'tcx>),
        values: &mut [Expr],
        arguments: Arguments<'_, 'tcx>,
        span: Span,
        out: &mut Vec<rust_js_program::js::Stmt>,
    ) -> R<()> {
        let sig = (self.tcx)
            .instantiate_bound_regions_with_erased(self.tcx.fn_sig(callee).instantiate_identity().skip_normalization());
        let instantiated: Vec<Ty<'tcx>> = sig
            .inputs()
            .iter()
            .map(|&declared| self.instantiated(declared, args))
            .collect();
        for (at, (&declared, &caller)) in sig.inputs().iter().zip(&instantiated).enumerate() {
            let others = (instantiated.iter().enumerate())
                .filter(|&(other, _)| other != at)
                .map(|(_, &ty)| ty);
            self.refuse_guard_written_whole((declared, caller), others, span)?;
        }
        let given = match arguments {
            Arguments::Raw => {
                for ((value, &declared), &caller) in values.iter_mut().zip(sig.inputs()).zip(&instantiated) {
                    let crossing = (callee, declared, caller);
                    if let Some(form) = self.form_of(crossing) {
                        *value = self.form_given(form, crossing, value.clone(), span)?;
                    }
                }
                &instantiated[..]
            }
            Arguments::Formed => &instantiated[..],
            Arguments::FormedOf { given } => given,
        };
        self.given_iterators((callee, args), values, given, span, out)
    }

    /// Whether a value of generic code's `declared` type, `def_id`'s, its
    /// caller's type for it `instantiated`, has another form there than the
    /// caller's, adapted where it's given and given back.
    pub fn form_differs(&self, form: Form, (def_id, declared, instantiated): Crossing<'tcx>) -> bool {
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
            // An iterator held in a place of its own, or a tuple of one.
            Form::Placed => self.placed_shape((def_id, declared, instantiated)).is_some(),
        }
    }

    /// Whether generic code `def_id` holds a value of its `declared` type,
    /// its caller's `instantiated`, in a place of its own (`Form::Placed`),
    /// the value itself, not a tuple of it: through shared references,
    /// which are the value, a type parameter whose `&mut` is the value
    /// itself, given an iterator whose `&mut` is a cell. What an
    /// `IntoIterator` of generic code's makes, `U::IntoIter`, is so too, as
    /// it's held as it is (ADR 0552); never an item, `I::Item`, which its
    /// iterator gives as its caller holds it. A field's (`placed_field`) and
    /// a place's, a `&mut` of it, ask it.
    pub(crate) fn holds_in_place(&self, (def_id, declared, instantiated): Crossing<'tcx>) -> bool {
        let (declared, instantiated) = past_shared_refs(declared, instantiated);
        let into_iter = || {
            crate::recognition::trait_method(
                self.tcx,
                crate::recognition::std_item(self.tcx, crate::recognition::StdItem::IntoIterator),
                "IntoIter",
            )
        };
        let made_by_into_iter = matches!(*declared.kind(),
            ty::Alias(_, ty::AliasTy { kind: ty::Projection { def_id }, .. }) if def_id == into_iter());
        (matches!(declared.kind(), ty::Param(_)) || made_by_into_iter)
            && self.placed_iterator(instantiated)
            && (self
                .mut_ref_form_in(declared, self.tcx.param_env(def_id))
                .is_the_value()
                || self.held_in_place_by_its_impl(def_id, declared))
    }

    /// What of a value crossing into generic code or back is in a place of
    /// its own (`Form::Placed`): the value (`holds_in_place`), or the parts
    /// of a tuple given or given back by value, `(A, B)` of itertools'
    /// `multizip`, each as it is held, which no one can tell from a tuple
    /// made anew, as nothing keeps a tuple moved (ADR 0262's no
    /// difference). Not a tuple behind a reference, which is its caller's.
    pub(crate) fn placed_shape(&self, crossing: Crossing<'tcx>) -> Option<Placed> {
        let (def_id, declared, instantiated) = crossing;
        if let (ty::Tuple(d), ty::Tuple(i)) = (*declared.kind(), *instantiated.kind()) {
            let parts: Vec<Option<Placed>> = (d.iter().zip(i.iter()))
                .map(|(d, i)| self.placed_shape((def_id, d, i)))
                .collect();
            return parts.iter().any(Option::is_some).then_some(Placed::Parts(parts));
        }
        self.holds_in_place(crossing).then_some(Placed::Itself)
    }

    /// A type generic code `def_id` holds in a place of its own
    /// (`Form::Placed`) inside what its `declared` type is, its caller's
    /// `instantiated`, where what holds it doesn't, or the other way about:
    /// a `Vec<I>`, an `Option<I>` or a struct of the crate's, which its
    /// caller holds as it is; an iterator not the crate's, std's `Take<I>`,
    /// which holds it as that iterator's own `Iterator` impl does, where
    /// `def_id` doesn't, a derived `Clone` of `Take<I>` (#2255). Refused.
    /// Not at the top, the form's own crossing, nor a part of a tuple there
    /// taken or given by value, which it crosses part by part
    /// (`placed_shape`); not a `&mut` to one, its caller's cell either way.
    /// Nor inside a library's struct whose fields only its own generic code
    /// reads, which holds it as generic code holds it wherever.
    pub(crate) fn placed_within(&self, def_id: DefId, declared: Ty<'tcx>, instantiated: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let placed = |d: Ty<'tcx>, i: Ty<'tcx>| self.holds_in_place((def_id, d, i));
        // Each part, and whether what holds it holds it in a place, `None`
        // as generic code holds it, whichever; a tuple's as what holds the
        // tuple, `held`, holds it.
        let parts = |declared: Ty<'tcx>,
                     instantiated: Ty<'tcx>,
                     held: Option<bool>|
         -> Vec<(Ty<'tcx>, Ty<'tcx>, Option<bool>)> {
            let not = Some(false);
            match (*declared.kind(), *instantiated.kind()) {
                (ty::Ref(_, d, Mutability::Not), ty::Ref(_, i, Mutability::Not))
                | (ty::Array(d, _), ty::Array(i, _))
                | (ty::Slice(d), ty::Slice(i)) => vec![(d, i, not)],
                (ty::Tuple(d), ty::Tuple(i)) => d.iter().zip(i.iter()).map(|(d, i)| (d, i, held)).collect(),
                // A `dyn`'s trait's types, which its caller's impl, its
                // pair's, takes as its caller holds them, `&mut dyn Get<I>`.
                (ty::Dynamic(d, ..), ty::Dynamic(i, ..)) => (d.iter().zip(i.iter()))
                    .flat_map(|(d, i)| {
                        let types = |p: ty::Binder<'tcx, ty::ExistentialPredicate<'tcx>>| -> Vec<Ty<'tcx>> {
                            match p.skip_binder() {
                                ty::ExistentialPredicate::Trait(tr) => tr.args.types().collect(),
                                ty::ExistentialPredicate::Projection(p) => {
                                    p.args.types().chain(p.term.as_type()).collect()
                                }
                                ty::ExistentialPredicate::AutoTrait(_) => Vec::new(),
                            }
                        };
                        types(d)
                            .into_iter()
                            .zip(types(i))
                            .map(|(d, i)| (d, i, not))
                            .collect::<Vec<_>>()
                    })
                    .collect(),
                // A `&mut` to what's held so is its caller's cell either way;
                // to another, what it's to, an object, `&mut Wrap<I>`.
                (ty::Ref(_, d, Mutability::Mut), ty::Ref(_, i, Mutability::Mut)) if !placed(d, i) => {
                    vec![(d, i, not)]
                }
                // Of the crate's own, its fields, each held as its own code
                // holds it, in a place of its own where its `Iterator` impl
                // holds it so (`placed_field`), as a `&mut I` field holds
                // its caller's cell either way, `Back { iter: &mut I }`'s.
                // One `def_id` knows the type of reads it so too, by the
                // same answer: only one of its type parameters' is held as
                // `def_id` holds that parameter.
                (ty::Adt(d, dargs), ty::Adt(i, iargs)) if d.did() == i.did() && built_by_the_crate(d) => (d
                    .all_fields())
                .map(|field| {
                    let of = |args| field.ty(self.tcx, args).skip_normalization();
                    let held = of(dargs).has_param() && self.placed_field(field, of(iargs));
                    (of(dargs), of(iargs), Some(held))
                })
                .collect(),
                // Of a library's struct the program can't read a field of,
                // itertools' `IntoChunks<I>`, what its own generic code
                // builds and reads alone, as std's iterators are, which
                // holds a type parameter as generic code holds it wherever:
                // its type arguments, each as it's held, walked on.
                (ty::Adt(d, dargs), ty::Adt(i, iargs))
                    if d.did() == i.did()
                        && d.is_struct()
                        && !self.recognition().in_sysroot(d.did())
                        && self.held_in_place_by(instantiated).is_empty()
                        && !(d.all_fields()).any(|field| field.vis.is_accessible_from(CRATE_DEF_ID, self.tcx)) =>
                {
                    (dargs.types().zip(iargs.types())).map(|(d, i)| (d, i, None)).collect()
                }
                // Of another crate's that iterates none in a place, std's,
                // which the program reads by shape, and a library's it may,
                // an enum's or one with a public field, its fields too, past
                // what only names a type, `PhantomData<T>` and a raw
                // pointer's pointee, a `Vec<T>`'s.
                (ty::Adt(d, dargs), ty::Adt(i, iargs))
                    if d.did() == i.did() && self.held_in_place_by(instantiated).is_empty() && !d.is_phantom_data() =>
                {
                    (d.all_fields())
                        .map(|field| {
                            let of = |args| field.ty(self.tcx, args).skip_normalization();
                            (of(dargs), of(iargs), not)
                        })
                        .collect()
                }
                _ if let (Some((d, _)), Some((i, _))) = (
                    super::pointers::raw_pointee(self.tcx, declared),
                    super::pointers::raw_pointee(self.tcx, instantiated),
                ) =>
                {
                    vec![(d, i, not)]
                }
                (ty::Adt(d, dargs), ty::Adt(i, iargs)) if d.did() == i.did() => {
                    let held = self.held_in_place_by(instantiated);
                    (dargs.types().zip(iargs.types()).enumerate())
                        .map(|(at, (d, i))| (d, i, Some(held.contains(&at))))
                        .collect()
                }
                _ => Vec::new(),
            }
        };
        // A tuple by value crosses part by part (`placed_shape`).
        let top = match declared.kind() {
            ty::Tuple(_) => None,
            _ => Some(false),
        };
        let (declared, instantiated) = past_shared_refs(declared, instantiated);
        if !instantiated
            .walk()
            .filter_map(|part| part.as_type())
            .any(|t| self.placed_iterator(t))
        {
            return None;
        }
        let mut todo = parts(declared, instantiated, top);
        // Each once, as a type of the crate's may hold itself.
        let mut seen = rust_js_program::collections::IndexSet::default();
        while let Some((d, i, held)) = todo.pop() {
            if !seen.insert((d, i, held)) {
                continue;
            }
            if held.is_some_and(|held| placed(d, i) != (held && self.placed_iterator(i))) {
                return Some(i);
            }
            todo.extend(parts(d, i, held));
        }
        None
    }

    /// Whether `param`, a type parameter of `def_id`'s impl, or of `def_id`
    /// itself, a type, is one its type, an iterator, holds in a place of its
    /// own, as its own `Iterator` impl holds it so: `I` of std's derived
    /// `Clone` of `Take<I>`, which clones what that holds as it's held
    /// (`held_in_place_by`), and of the crate's own `Both<I, J>`, whose field
    /// `a: I` holds it so wherever it's known (`field_crossing`).
    fn held_in_place_by_its_impl(&self, def_id: DefId, param: Ty<'tcx>) -> bool {
        use rustc_hir::def::DefKind;
        let tcx = self.tcx;
        let holder = match tcx.def_kind(def_id) {
            DefKind::Impl { .. } | DefKind::Struct | DefKind::Enum => def_id,
            _ => match tcx.impl_of_assoc(def_id) {
                Some(imp) => imp,
                None => return false,
            },
        };
        let ty::Adt(adt, args) = *tcx.type_of(holder).instantiate_identity().skip_normalization().kind() else {
            return false;
        };
        let Some(at) = args.types().position(|arg| arg == param) else {
            return false;
        };
        self.iterated_in_place(adt, at)
    }

    /// The type arguments, by position, that `ty`, an iterator not the
    /// crate's, std's `Take<I>`, holds in a place of its own: those its
    /// `Iterator` impl holds so (`Form::Placed`), as what makes one is
    /// generic code that holds it so. None of another type, which its caller
    /// makes and holds as it is, nor of one whose type arguments are a
    /// caller's.
    fn held_in_place_by(&self, ty: Ty<'tcx>) -> Vec<usize> {
        let ty::Adt(adt, args) = *ty.kind() else {
            return Vec::new();
        };
        match built_by_the_crate(adt) {
            true => Vec::new(),
            false => (0..args.types().count())
                .filter(|&at| self.iterated_in_place(adt, at))
                .collect(),
        }
    }

    /// Whether `adt`'s `Iterator` impl holds its type argument at `at` in a
    /// place of its own (`Form::Placed`): a type parameter of the impl whose
    /// `&mut` is the value itself, `I` of `impl<I: Iterator> Iterator for
    /// Take<I>`.
    fn iterated_in_place(&self, adt: ty::AdtDef<'tcx>, at: usize) -> bool {
        let tcx = self.tcx;
        let iterator = super::recognition::std_item(tcx, super::recognition::StdItem::Iterator);
        tcx.all_impls(iterator).any(|iterating| {
            matches!(*tcx.type_of(iterating).instantiate_identity().skip_normalization().kind(),
            ty::Adt(of, declared) if of.did() == adt.did()
                && declared.types().nth(at).is_some_and(|held| {
                    matches!(held.kind(), ty::Param(_))
                        && self.mut_ref_form_in(held, tcx.param_env(iterating)).is_the_value()
                }))
        })
    }

    /// Whether `ty`, a type known whole, is an iterator whose `&mut` is a
    /// cell, as JS holds it as no object, `Form::Placed`'s: a fieldless
    /// enum's string, a unit struct's `undefined` or a struct's number, the
    /// crate's or a library's, or a `Box` of one, which is what it holds.
    pub(crate) fn placed_iterator(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(..))
            && !ty.has_param()
            && self.mut_ref_form(ty).is_cell()
            && self.implements_iterator(ty)
    }

    /// The form generic code has of what a function gives back, generic
    /// code's `declared` result, its caller's `instantiated`: a value's
    /// (`form_of`), or a write's result held as a value where generic code
    /// holds a type parameter, `Form::Valued` (#1410).
    pub(crate) fn form_of_result(&self, crossing: Crossing<'tcx>) -> Option<Form> {
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
    pub(crate) fn std_reads_tagged(&self, item: Ty<'tcx>) -> Option<Crossing<'tcx>> {
        let result = crate::recognition::std_item(self.tcx, crate::recognition::StdItem::Result);
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
    pub fn write_result_inside(&self, (_, declared, instantiated): Crossing<'tcx>) -> Option<Ty<'tcx>> {
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
    pub fn write_result_through_bound(&self, (def_id, declared, instantiated): Crossing<'tcx>) -> bool {
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
    pub(crate) fn declares_box(&self, def_id: DefId, mut declared: Ty<'tcx>) -> bool {
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
    pub(crate) fn is_generic_box_in(&self, ty: Ty<'tcx>, param_env: ty::ParamEnv<'tcx>) -> bool {
        matches!(*ty.kind(), ty::Ref(_, pointee, Mutability::Mut)
            if self.mut_ref_form_in(pointee, param_env).is_generic_box())
    }

    /// How a dictionary's entry reads what a type parameter given a `ty`
    /// holds: of a `&mut`, generic code's box, an object's in-place cell
    /// (`Form::Box`) or its caller's cell of a number, whose `value` it is
    /// to; else the value itself, a `&mut` to a function or an iterator's
    /// too, given as it is (`given_as_the_value`, #1414).
    pub(crate) fn held_in_generic_box(&self, ty: Ty<'tcx>) -> fn(Expr) -> Expr {
        match self.in_generic_box(ty) {
            true => Expr::held,
            false => |value| value,
        }
    }

    /// Whether a type parameter given a `ty` holds it as generic code's box,
    /// whose `value` it's to: a `&mut`, but to a function or an iterator,
    /// which may be given as it is (`given_as_the_value`, ADR 0061), or to
    /// a `dyn`, whose pair it is (ADR 0099). Of a `&mut` to a type this code
    /// doesn't know, its own parameter's, the box it holds it as, unless
    /// that's a function or an iterator given as it is (`mut_ref_form`):
    /// `held.as_ref()` of a `held: &mut T` reads what it's to.
    pub(crate) fn in_generic_box(&self, ty: Ty<'tcx>) -> bool {
        matches!(*ty.kind(), ty::Ref(_, pointee, Mutability::Mut)
        if match self.is_unknown(pointee) {
            true => self.is_cell(ty),
            false => !self.is_callable(pointee) && !self.implements_iterator(pointee) && !pointee.is_trait(),
        })
    }

    /// The crossing of a field of the crate's own type `ty`, variant
    /// `variant`'s `field`, declared generic code's box, `&mut T` or a `T`
    /// given a `&mut` to an object, `Slot { at: &mut point }`: held as that
    /// box wherever the type is, the object's in-place cell, `$cellOf(point)`,
    /// so its generic code and the code that knows `T` hold one value, as
    /// a call's `Form::Box` is; read through it, written and made at its
    /// crossing (#1405). And a field whose type parameter the type's own
    /// `Iterator` impl holds in a place of its own (`Form::Placed`), given
    /// an iterator JS holds as no object, `a: I` of `Both<Tri, J>`: held in
    /// that place wherever the type is, `{ value: "A" }`, as its impls and
    /// what's generic over the type hold it (#2255). `None` where this code
    /// holds the field as it's declared, a number's cell or generic code's
    /// own box.
    pub fn field_crossing(
        &self,
        ty: Ty<'tcx>,
        variant: rustc_abi::VariantIdx,
        field: rustc_abi::FieldIdx,
    ) -> Option<Crossing<'tcx>> {
        let ty::Adt(adt, args) = *ty.kind() else {
            return None;
        };
        let def = &adt.variant(variant).fields[field];
        let instantiated = def.ty(self.tcx, args).skip_normalization();
        let instantiated = (self.tcx)
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(instantiated))
            .unwrap_or(instantiated);
        (self.field_crossing_of(adt, def, instantiated)).filter(|&crossing| self.form_differs(Form::Box, crossing))
    }

    /// Whether `field`, of the crate's own type, given `ty` (`Tri` for
    /// `a: I` of a `Both<Tri, J>`), is held in a place of its own wherever
    /// its type is, `{ value: "A" }` (`field_crossing`'s `Form::Placed`,
    /// #2255): what reads, copies, clones, compares or makes that type by
    /// its shape reads and makes the field so (`held`, `holding`).
    pub fn placed_field(&self, field: &ty::FieldDef, ty: Ty<'tcx>) -> bool {
        let tcx = self.tcx;
        let owner = tcx.parent(field.did);
        let owner = match tcx.def_kind(owner) {
            rustc_hir::def::DefKind::Variant => tcx.parent(owner),
            _ => owner,
        };
        matches!(
            tcx.def_kind(owner),
            rustc_hir::def::DefKind::Struct | rustc_hir::def::DefKind::Enum
        ) && (self.field_crossing_of(tcx.adt_def(owner), field, ty))
            .is_some_and(|crossing| self.holds_in_place(crossing))
    }

    /// Whether `ty`, a struct, holds a field in a place of its own
    /// (`placed_field`).
    pub(crate) fn struct_holds_placed(&self, ty: Ty<'tcx>) -> bool {
        matches!(*ty.kind(), ty::Adt(adt, args) if adt.is_struct()
            && (adt.non_enum_variant().fields.iter()).any(|field| self.placed_field(field, self.field_ty(field, args))))
    }

    /// Whether `variant`, of fields `fields` as its type has them, holds one
    /// in a place of its own (`placed_field`): its fields' definitions
    /// matter to what copies, clones or compares it by its shape.
    pub(crate) fn holds_placed(&self, variant: &ty::VariantDef, fields: &[(String, Ty<'tcx>)]) -> bool {
        (variant.fields.iter().zip(fields)).any(|(def, &(_, t))| self.placed_field(def, t))
    }

    /// The crossing of `def`, a field of `adt`, the crate's own type, given
    /// `instantiated`: its type's code's declared type against it.
    fn field_crossing_of(
        &self,
        adt: ty::AdtDef<'tcx>,
        def: &ty::FieldDef,
        instantiated: Ty<'tcx>,
    ) -> Option<Crossing<'tcx>> {
        if adt.is_union() || self.is_std(adt.did()) {
            return None;
        }
        let declared = def.ty(self.tcx, ty::GenericArgs::identity_for_item(self.tcx, adt.did()));
        Some((adt.did(), declared.skip_normalization(), instantiated))
    }

    /// The form generic code has of `declared` where its caller's type is
    /// `instantiated`, if it's another than the caller's.
    pub fn form_of(&self, crossing: Crossing<'tcx>) -> Option<Form> {
        let form = [Form::Handle, Form::Box, Form::HeldBox, Form::Tagged, Form::Placed]
            .into_iter()
            .find(|&form| self.form_differs(form, crossing));
        let answer = match form {
            None => "as-it-is",
            Some(Form::Handle) => "handle",
            Some(Form::Box) => "box",
            Some(Form::HeldBox) => "held-box",
            Some(Form::Tagged) => "tagged",
            Some(Form::Placed) => "placed",
            Some(Form::Valued) => unreachable!("`form_of` never asks `Form::Valued`"),
        };
        rust_js_program::decisions::record("generic-form", answer);
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
    /// type parameters, the arrow made there. Of a writer function, its
    /// `Formatter` none of its JS takes (ADR 0054), which a call through
    /// the pointer gives none of either (ADR 0538): not a parameter of the
    /// arrow (#1710).
    pub fn reified_pointer(
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
        let formatter = self.recognition().formatter_param(def_id);
        let inputs: Vec<Crossing<'tcx>> = (declared.inputs().iter().enumerate())
            .filter(|&(at, _)| Some(at) != formatter)
            .map(|(_, &input)| (def_id, input, self.instantiated(input, args)))
            .collect();
        // Each named as the function's JS names it, `(p) => ..` (#1328).
        let names: Vec<String> = (super::signatures::taken_parameter_names(self.tcx, def_id, formatter, false))
            .into_iter()
            .enumerate()
            .map(|(at, name)| name.unwrap_or_else(|| format!("arg{at}")))
            .collect();
        let output = (def_id, declared.output(), self.instantiated(declared.output(), args));
        let span = self.tcx.def_span(def_id);
        // And its `Formatter`'s options where it takes them, which a call
        // through the pointer gives after its arguments (ADR 0538,
        // `mir_closure_call`'s), before its dictionaries (#1758).
        let options = formatter.is_some() && self.takes_options(def_id);
        let made = move |cx: &mut Self| -> super::R<Expr> {
            let mut params: Vec<rust_js_program::js::Name> = (names.iter())
                .map(|name| cx.fresh(super::temporary_names::Base::Named(name)))
                .collect();
            let mut given = (params.iter().zip(&inputs))
                .map(|(name, &given)| match cx.form_of(given) {
                    Some(form) => cx.form_given(form, given, Expr::var(name), span),
                    None => Ok(Expr::var(name)),
                })
                .collect::<super::R<Vec<_>>>()?;
            if options {
                let name = cx.fresh(super::temporary_names::Role::Options);
                given.push(Expr::var(&name));
                params.push(name);
            }
            let called = Expr::call(function, given);
            let called = match cx.form_of(output) {
                Some(form) => cx.form_back(form, output, called),
                None => called,
            };
            Ok(Expr::arrow(
                params.into_iter().map(Into::into).collect(),
                vec![rust_js_program::js::StmtKind::Return(Some(called)).at(rust_js_program::js::Span::NONE)],
            ))
        };
        self.reified_const((def_id, args), fn_def_ty, made)
    }

    /// A generic function's instance `def_id` of `args` as a function value
    /// in its callers' forms, `made`: its module's `const`, so two pointers
    /// to it are one, as a generic function's value is (ADR 0395); of an
    /// instance of the caller's own type parameters, the arrow made there.
    pub fn reified_const(
        &mut self,
        (def_id, args): (DefId, ty::GenericArgsRef<'tcx>),
        fn_def_ty: Ty<'tcx>,
        made: impl FnOnce(&mut Self) -> super::R<Expr>,
    ) -> super::R<Expr> {
        if args.has_non_region_param() || args.has_aliases() {
            return made(self);
        }
        let name = rust_js_program::casing::camel_case(self.tcx.item_name(def_id).as_str());
        let base = format!("{}Fn", super::traits::instance_name(self.tcx, &name, args));
        self.module_fn_pointer((fn_def_ty, &base), made)
    }

    /// The referent of a shared reference given where generic code's
    /// `declared` `&T` goes, `instantiated` the caller's type for it, where
    /// JS holds it as itself, a number, so a pointer generic code made of it
    /// would be of a copy (ADR 1114): not an object's or a slice's, whose
    /// pointer is theirs, nor a type only generic callers know.
    pub fn copied_referent(&self, declared: Ty<'tcx>, instantiated: Ty<'tcx>) -> Option<Ty<'tcx>> {
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
    /// Generic code's box of an object is an in-place cell of it, a write in
    /// place of the caller's referent (`in_place`, #1665).
    pub fn form_given(&self, form: Form, crossing: Crossing<'tcx>, value: Expr, span: Span) -> R<Expr> {
        if !self.form_differs(form, crossing) {
            return Ok(value);
        }
        Ok(match form {
            Form::Handle => Expr::call_helper("$ptrTo", vec![value]),
            Form::Box => {
                let (_, _, instantiated) = crossing;
                let written = instantiated.builtin_deref(true).unwrap_or(instantiated);
                self.in_place(InPlaceWriter::CellOf, written, vec![value], span)?
            }
            Form::HeldBox => Expr::call_helper("$heldCellOf", vec![value]),
            Form::Tagged => Expr::call_helper("$writeTagged", vec![value]),
            Form::Valued => value,
            Form::Placed => self.placed_shape(crossing).expect("a placed crossing").given(value),
        })
    }

    /// How generic code `def_id` holds a reference its caller gives where
    /// its `declared` type goes, `instantiated` the caller's `&C` or `&mut
    /// C`, which a bound by std's `IntoIterator` on it reads (ADR 0457):
    /// the one answer of a collection's and of an iterator's `&mut I`
    /// (#2336). A type parameter another bound names, `impl Is<I>`'s, holds
    /// it as that one's value, as a type parameter given it holds it
    /// (`ref_held_by_a_parameter`, #966); one given as it is, or a `&mut`
    /// of one, as concrete code has it; another, of an iterator, as the JS
    /// iterator its caller made (ADR 0061), of a collection as it is (ADR
    /// 0976); a `&mut` of a type parameter in the box its caller makes of
    /// a `&mut` to an object (`Form::Box`, ADR 0454).
    pub(crate) fn ref_held(&self, crossing: Crossing<'tcx>) -> RefHeld {
        let (def_id, declared, instantiated) = crossing;
        let ty::Ref(_, pointee, _) = *instantiated.kind() else {
            return RefHeld::AsItIs;
        };
        let given_as_it_is = |p: ty::ParamTy| crate::iterators::given_as_it_is(self.tcx, (def_id, None), p);
        let unconverted = match self.implements_iterator(pointee) {
            true => RefHeld::Stepped,
            false => RefHeld::AsItIs,
        };
        match *declared.kind() {
            ty::Param(p) if crate::iterators::named_by_another_bound(self.tcx, def_id, p) => {
                self.ref_held_by_a_parameter(instantiated)
            }
            ty::Param(p) if given_as_it_is(p) => RefHeld::AsItIs,
            ty::Param(_) => unconverted,
            _ if self.form_differs(Form::Box, crossing) => RefHeld::InBox,
            ty::Ref(_, inner, _) if matches!(*inner.kind(), ty::Param(p) if given_as_it_is(p)) => RefHeld::AsItIs,
            _ => unconverted,
        }
    }

    /// How a type parameter given `ty`, a reference, holds it, where no
    /// crossing says more (`ref_held`): in generic code's box where it has
    /// one and the caller's `&mut` isn't that box already, a cell's, an
    /// object's (`in_generic_box`, ADR 0454); else as it is.
    pub(crate) fn ref_held_by_a_parameter(&self, ty: Ty<'tcx>) -> RefHeld {
        match *ty.kind() {
            ty::Ref(_, pointee, Mutability::Mut)
                if self.in_generic_box(ty) && !self.mut_ref_form(pointee).is_cell() =>
            {
                RefHeld::InBox
            }
            _ => RefHeld::AsItIs,
        }
    }

    /// `value`, generic code's of its `declared` type, given back where
    /// its caller's type is `instantiated`: in the caller's form, a
    /// handle's `value` (ADR 1114), a held box's box of the object
    /// (#1303).
    pub fn form_back(&self, form: Form, crossing: Crossing<'tcx>, value: Expr) -> Expr {
        if !self.form_differs(form, crossing) {
            return value;
        }
        match form {
            Form::Handle | Form::Box => box_back(value),
            Form::Placed => self.placed_shape(crossing).expect("a placed crossing").back(value),
            Form::HeldBox => Expr::call_helper("$heldObjectOf", vec![value]),
            Form::Tagged => Expr::call_helper("$writeOf", vec![value]),
            Form::Valued => unreachable!("a call's write result, `fmt_result_value`'s (#1634)"),
        }
    }
}

/// Whether generic code's `declared` type, a type parameter's or an
/// associated type's, holds as a value what its caller has as a write's
/// result, `instantiated`, which a call of the caller's function throws:
/// `Form::Valued`'s place, which the analysis of what fails asks too
/// (#1410).
pub(crate) fn valued_where<'tcx>(
    tcx: rustc_middle::ty::TyCtxt<'tcx>,
    declared: Ty<'tcx>,
    instantiated: Ty<'tcx>,
) -> bool {
    crate::recognition::known_by_bounds(declared) && crate::recognition::is_write_result_type(tcx, instantiated)
}

/// What's refused of a write's result held inside what generic code has as
/// a `Result` of its type parameters (#1416).
pub fn held_write_result(held: Ty<'_>) -> String {
    format!(
        "a write's result, `{held}`, held inside what generic code has as a `Result` of its type parameters, `{{ TAG }}` there (#1416)"
    )
}

/// Whether `adt` is the crate's own, whose values its own code may build
/// and read as it holds them, where generic code holds a type parameter of
/// it otherwise (`Form::Placed`): not another crate's, std's `Take<I>`,
/// which only that crate's generic code makes (#2255).
fn built_by_the_crate(adt: ty::AdtDef<'_>) -> bool {
    adt.did().is_local()
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

/// Whether any types given for `ty`'s type parameters, and values for its
/// const parameters, may make `trait_id` hold of it, each an unknown: of `vec::IntoIter<T>`'s
/// `TrustedRandomAccessNoCoerce`, whose impl asks `T: Copy`, yes; of
/// `vec::IntoIter<MultiProductIter<I>>`'s, no, as no impl makes
/// `MultiProductIter<_>` `Copy`, whatever `I` is (#1385).
pub(crate) fn may_hold_for_some<'tcx>(tcx: rustc_middle::ty::TyCtxt<'tcx>, ty: Ty<'tcx>, trait_id: DefId) -> bool {
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
/// typed answer of one apart (`recognition::kind_of`, ADR 1124); and as a
/// `partition`'s or an `unzip`'s side is its caller's, which std fills by
/// its `Default` and `Extend`, the helper's array as before #2377, where a
/// type that holds one, a crate's `Bag<T>`, is the crate's own `Extend`
/// (filling it by its dictionaries is #2387's).
pub(crate) fn is_type_parameter(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Param(_))
}

/// The index of the type parameter `ty` is, `Self` too, where it's one:
/// which of a function's generic arguments it's given there (#1661).
pub(crate) fn type_parameter_of(ty: Ty<'_>) -> Option<u32> {
    type_parameter(ty).map(|param| param.index)
}

/// The type parameter `ty` is, where it's one, `type_parameter_of`'s, as a
/// question of it by name asks it, `given_as_it_is` (#2104).
pub(crate) fn type_parameter(ty: Ty<'_>) -> Option<ty::ParamTy> {
    match ty.kind() {
        ty::Param(param) => Some(*param),
        _ => None,
    }
}

/// Whether a function's `declared` input is a `&mut` of a type parameter,
/// `Self` too, or a type parameter itself: what its caller gives a `&mut`
/// to there, generic code may write whole, through its box, an in-place cell
/// (ADR 0454), or through the `&mut` it holds, `Pin::new(&mut b)`'s `Ptr`
/// that `set` writes through (#1665).
pub(crate) fn writes_through_its_box(declared: Ty<'_>) -> bool {
    match *declared.kind() {
        ty::Ref(_, to, Mutability::Mut) => matches!(to.kind(), ty::Param(_)),
        ty::Param(_) => true,
        _ => false,
    }
}

/// Whether `ty`, a std impl's type's first argument or a raw pointer's
/// pointee, is a type parameter: `impl<T> NonNull<T>`'s `T`, which a std path
/// writes `NonNull::<T>`, not `NonNull::<[T]>` or `*const [T; N]` (#1681).
pub(crate) fn is_impl_parameter(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Param(_))
}

/// Whether `ty` holds the type parameter of index `index`, itself or in
/// any type it's made of, as `repeat_n`'s `RepeatN<T>` holds its `T`, which
/// a std maker of a compiled iterator is given the drop of (#1458).
pub(crate) fn holds_type_parameter(ty: Ty<'_>, index: u32) -> bool {
    type_parameter_count(ty, index) > 0
}

/// How many times `ty` names the type parameter of index `index`, itself
/// or in the types it's made of: whether a declaration shows its identity,
/// `selected<T>(value: &T) -> &T` twice (#1866).
pub(crate) fn type_parameter_count(ty: Ty<'_>, index: u32) -> usize {
    ty.walk()
        .filter_map(|part| part.as_type())
        .filter(|part| matches!(part.kind(), ty::Param(param) if param.index == index))
        .count()
}

/// Whether `ty` names a type parameter, so what it's instantiated as isn't
/// known: of generic code `pointer_may_keep` walks as its own, it may be
/// the `&mut`'s pointee, or a reference or raw pointer to it (#1781); of a
/// type `may_be_cell` asks whether a `dyn` may be, an impl of the `dyn`'s
/// trait may hold of it (#1006); of what a property read reads, a getter
/// of the program's may answer it, which each caller's instance settles,
/// a body's summary keeping it till then (`cell_borrows`' `reads`, #2109).
pub(crate) fn may_be_any_type(ty: Ty<'_>) -> bool {
    ty.has_param()
}

/// Whether each parameter `ty` names, a type's or a const's, is one of
/// `id`'s generics, by its index and name: of a type made in another
/// body's terms, a caller's, it may name one `id` hasn't, whose env then
/// can't give a const's type, where rustc crashes (#1608).
pub(crate) fn params_owned_by<'tcx>(tcx: ty::TyCtxt<'tcx>, id: DefId, ty: Ty<'tcx>) -> bool {
    let generics = tcx.generics_of(id);
    let owns = |index: u32, name: rustc_span::Symbol| {
        (index as usize) < generics.count() && generics.param_at(index as usize, tcx).name == name
    };
    ty.walk().all(|part| match part.kind() {
        ty::GenericArgKind::Type(ty) => match *ty.kind() {
            ty::Param(param) => owns(param.index, param.name),
            _ => true,
        },
        ty::GenericArgKind::Const(c) => match c.kind() {
            ty::ConstKind::Param(param) => owns(param.index, param.name),
            _ => true,
        },
        ty::GenericArgKind::Lifetime(_) => true,
    })
}

/// How generic code holds a reference its caller gave (`ref_held`): in the
/// box its caller made of a `&mut` to an object, as concrete code has it,
/// or as the JS iterator its caller made of an iterator's (#2336).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RefHeld {
    InBox,
    AsItIs,
    Stepped,
}

impl RefHeld {
    /// `value`, generic code's, as its caller has it: out of the box, as
    /// `form_back` reads `Form::Box`'s; else itself.
    pub(crate) fn read(self, value: Expr) -> Expr {
        match self {
            RefHeld::InBox => box_back(value),
            RefHeld::AsItIs | RefHeld::Stepped => value,
        }
    }
}

/// What generic code's box, `Form::Box`'s and a handle's, holds: its
/// `value`, the one read of it back (`form_back`, `RefHeld::read`).
fn box_back(value: Expr) -> Expr {
    Expr::held(value)
}
