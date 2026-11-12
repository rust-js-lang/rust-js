//! A `&mut` to a value JS can't change in place, given to a call or given
//! back by one: a box the caller copies back, the place itself, or the
//! items a std call holds (ADRs 0072, 0099).

use super::std_types::cow::CowOp;
use super::std_types::map::{MapOp, Part};
use super::{FnCx, R, Std};
use crate::collections::IndexSet;
use crate::js::{self, Expr, Prop, Stmt, StmtKind};
use crate::lower::MutRefForm;
use crate::lower::generic_facts::{Crossing, Form};
use crate::lower::temporary_names::Role;

/// The forms generic code gives a function it calls as an `F`, of each
/// parameter where they differ from its callers', and gives back
/// (`unboxed_by_function`).
pub(super) type FormsGiven<'tcx> = (Vec<Option<(Form, Crossing<'tcx>)>>, Option<Form>);
use rustc_ast::Mutability;
use rustc_middle::thir::{ExprId, ExprKind};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;

/// Whether a value of `ty` may hold a `part`: `part` is in it, not only in
/// a projection, `<I as Iterator>::Item`, which names `I` but holds an
/// item, not an `I`.
pub(in crate::lower) fn holds_outside_projections<'tcx>(ty: Ty<'tcx>, part: Ty<'tcx>) -> bool {
    let mut walk = ty.walk();
    while let Some(arg) = walk.next() {
        match arg.as_type() {
            Some(arg) if arg == part => return true,
            Some(arg) if matches!(arg.kind(), ty::Alias(..)) => walk.skip_current_subtree(),
            _ => {}
        }
    }
    false
}

/// A box of `place`, `{ value: place }`: a `&mut` to a value JS can't change
/// in place, given to a call, which reads and writes its `value` (ADR 0074).
pub(super) fn boxed(place: Expr) -> Expr {
    Expr::object(vec![Prop::Field(js::HELD.into(), place)])
}

/// What a value generic code gives a dictionary's entry, or takes back from
/// it, holds of a `&mut` generic code has as its box, `GenericBox`, past a
/// parameter's or the result's own `&mut`, which the entry makes its impl's
/// own too (#686, ADRs 0099, 0454, 1086): each such `&mut`, by the form a
/// `&mut` to the impl's type is, through `Option`s and tuples, `Option<&mut
/// Self>` and `(&mut Self, u32)`; anything else as it is, which
/// `nested_mut_object` walks, a `Vec<&mut Self>`.
#[derive(Clone, Debug)]
pub(in crate::lower) enum Within<'tcx> {
    /// A `&mut` generic code has as its box, of what the impl's `&mut` is.
    Mut(MutRefForm),
    /// A tuple, of what each of its fields holds.
    Tuple(Vec<Within<'tcx>>),
    /// An `Option` whose `Some` boxes its value on neither side, of what
    /// its value holds.
    Some(Box<Within<'tcx>>),
    /// Anything else, given as it is.
    AsItIs(Ty<'tcx>),
}

impl<'tcx> Within<'tcx> {
    /// The types given as they are, which may hold a `&mut` the entry
    /// doesn't make its impl's own.
    pub(in crate::lower) fn given_as_it_is(self, out: &mut Vec<Ty<'tcx>>) {
        match self {
            Within::Mut(_) => {}
            Within::Tuple(parts) => parts.into_iter().for_each(|part| part.given_as_it_is(out)),
            Within::Some(inner) => inner.given_as_it_is(out),
            Within::AsItIs(ty) => out.push(ty),
        }
    }

    /// Whether a value of what this says is another on the other side: a
    /// `&mut` it holds is of a form that isn't generic code's box.
    pub(in crate::lower) fn changes(&self) -> bool {
        match self {
            Within::Mut(form) => !form.is_cell(),
            Within::Tuple(parts) => parts.iter().any(Within::changes),
            Within::Some(inner) => inner.changes(),
            Within::AsItIs(_) => false,
        }
    }

    /// `value`, of what this says, as the other side takes it: toward the
    /// impl, `to_impl`, each `&mut` that's generic code's box made the
    /// impl's form, as a parameter's own is, `arg1?.value`; back, each the
    /// impl's made generic code's box, an in-place cell of an object, as the
    /// result's own is (ADR 0454). `None` where it's the same value.
    pub(in crate::lower) fn adapted(&self, value: &Expr, to_impl: bool) -> Option<Expr> {
        match self {
            Within::Mut(form) => match form.is_cell() {
                true => None,
                false if to_impl => Some(Expr::held(value.clone())),
                false => Some(in_place_cell(value.clone())),
            },
            Within::Tuple(parts) => {
                let field = |i: usize| Expr::index(value.clone(), Expr::int(i as i128));
                let made: Vec<Option<Expr>> = (parts.iter().enumerate())
                    .map(|(i, part)| part.adapted(&field(i), to_impl))
                    .collect();
                if made.iter().all(Option::is_none) {
                    return None;
                }
                let items = (made.into_iter().enumerate()).map(|(i, made)| made.unwrap_or_else(|| field(i)));
                Some(Expr::array(items.collect()))
            }
            Within::Some(inner) => {
                let made = inner.adapted(&value.clone().present(), to_impl)?;
                Some(match (inner.as_ref(), to_impl) {
                    (Within::Mut(_), true) => Expr::optional_member(value.clone(), js::HELD),
                    _ => Expr::cond(Expr::presence(value.clone(), true), made, Expr::undefined()),
                })
            }
            Within::AsItIs(_) => None,
        }
    }
}

/// What a dictionary's entry adapts within of `ty`, a parameter's or the
/// result's type (`Within`): what it is past its shared references, which
/// are the value; `None` of a `&mut`, which the entry makes its impl's own
/// apart, one level (ADR 0454).
pub(in crate::lower) fn adapted_part<'tcx>(ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let mut at = ty;
    while let ty::Ref(_, inner, Mutability::Not) = *at.kind() {
        at = inner;
    }
    (!matches!(at.kind(), ty::Ref(_, _, Mutability::Mut))).then_some(at)
}

/// `Within` of `declared` as `actual` gives it, each `&mut` `leaf` says is
/// generic code's box its `Mut`, of the `form` of what `actual`'s is to:
/// through tuples and `Option`s, of whose value, as `actual` has it,
/// `unboxed` says a `Some` doesn't box it: generic code's of a `&mut` is the
/// box, which never looks like `None` (ADR 0051). The one shape a dictionary's entry adapts (#686), which `FnCx::within`
/// asks of its forms and `traits::method_dictionary` of where a `&mut` of
/// `Self` is, which a declaration says.
pub(in crate::lower) fn within_shape<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    (declared, actual): (Ty<'tcx>, Ty<'tcx>),
    (leaf, form): (&dyn Fn(Ty<'tcx>) -> bool, &dyn Fn(Ty<'tcx>) -> MutRefForm),
    unboxed: &dyn Fn(Ty<'tcx>) -> bool,
) -> Within<'tcx> {
    if leaf(declared)
        && let ty::Ref(_, given, Mutability::Mut) = *actual.kind()
    {
        return Within::Mut(form(given));
    }
    if !declared.walk().filter_map(|part| part.as_type()).any(leaf) {
        return Within::AsItIs(declared);
    }
    let option_of = |ty: Ty<'tcx>| match *ty.kind() {
        ty::Adt(adt, args) if tcx.is_lang_item(adt.did(), rustc_hir::attrs::lang_items::LangItem::Option) => {
            Some(args.type_at(0))
        }
        _ => None,
    };
    match (*declared.kind(), *actual.kind()) {
        (ty::Tuple(fields), ty::Tuple(given)) if fields.len() == given.len() => Within::Tuple(
            (fields.iter().zip(given.iter()))
                .map(|(field, given)| within_shape(tcx, (field, given), (leaf, form), unboxed))
                .collect(),
        ),
        _ => match (option_of(declared), option_of(actual)) {
            (Some(some), Some(given)) if option_of(some).is_none() && unboxed(given) => {
                Within::Some(Box::new(within_shape(tcx, (some, given), (leaf, form), unboxed)))
            }
            _ => Within::AsItIs(declared),
        },
    }
}

/// An in-place cell of `object`, `$cellOf(object)`: a `&mut` to an object
/// as generic code has it, a box, whose `value` is the object and whose
/// write of the whole changes it in place, `$assign`, as `*r = v` of the
/// object's own `&mut` does (ADRs 0153, 0454). `Cell::from_mut`'s, a
/// dictionary's `&mut Self::Out`, and a `&mut` given for longer than the
/// call (#528). What's moved out of one is apart (`moved_out`).
pub(in crate::lower) fn in_place_cell(object: Expr) -> Expr {
    Expr::call_helper("$cellOf", vec![object])
}

/// A box a call is given for a `&mut` to a place, copied back once the call
/// returns (ADR 0074): the place, and what's copied back to it, the box's
/// `value`, or what that holds in the caller's form, the `&mut` an in-place
/// cell of generic code's holds (#1300).
pub(super) type CopyBack = (Expr, Expr);

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What a box of a number, `boxed`, the call it was given to returned,
    /// writes back to its `place`, `place = box.value` (ADR 0074). An
    /// object's `&mut` given where generic code has its box is an in-place
    /// cell, never copied back (ADR 1115).
    pub(super) fn copied_back(&mut self, place: Expr, back: Expr, at: js::Span) -> Stmt {
        StmtKind::Assign(place, back).at(at)
    }

    /// `p` of `&mut p`, a reborrow's `&mut *&mut v[0]` too: `v[0]`.
    pub(super) fn mut_borrowed(&self, arg: ExprId) -> Option<ExprId> {
        let ExprKind::Borrow {
            borrow_kind: rustc_middle::mir::BorrowKind::Mut { .. },
            arg: place,
        } = self.frame.thir[self.strip(arg)].kind
        else {
            return None;
        };
        let mut place = place;
        while let ExprKind::Deref { arg: inner } = self.frame.thir[self.strip(place)].kind
            && let ExprKind::Borrow {
                borrow_kind: rustc_middle::mir::BorrowKind::Mut { .. },
                arg: reborrowed,
            } = self.frame.thir[self.strip(inner)].kind
        {
            place = reborrowed;
        }
        Some(place)
    }

    /// `makes_items`, of the call of `callee` and its arguments' types (ADR
    /// 0364): not a `&mut` one gives, an iterator's item, or the items of
    /// one it takes as an `IntoIterator`, a `&mut [T]`'s `&mut T` that
    /// `zip(b)` steps, made where it's made into its iterator (ADR 0926);
    /// not `into_iter`'s own `self`, which makes them.
    pub(in crate::lower) fn makes_items_of(
        &self,
        callee: DefId,
        output: Ty<'tcx>,
        generic_args: ty::GenericArgsRef<'tcx>,
        tys: &[Ty<'tcx>],
    ) -> Option<Ty<'tcx>> {
        let cells = |ty: Ty<'tcx>| ty.walk().filter_map(|part| part.as_type()).filter(|&t| self.is_cell(t));
        let into_iterator = super::recognition::std_item(self.tcx, super::recognition::StdItem::IntoIterator);
        let inputs = self.tcx.fn_sig(callee).instantiate_identity().skip_binder().inputs();
        let bounds = self.tcx.param_env(callee).caller_bounds();
        let taken_into = |i: usize| {
            inputs.get(i).is_some_and(|&input| {
                matches!(input.kind(), ty::Param(param) if !super::recognition::is_trait_self(*param))
                    && bounds.iter().any(|clause| {
                        clause.as_trait_clause().is_some_and(|bound| {
                            bound.def_id() == into_iterator && bound.self_ty().skip_binder() == input
                        })
                    })
            })
        };
        let items = |(i, t): (usize, Ty<'tcx>)| {
            let into = taken_into(i).then(|| self.iterable_item(t));
            self.iterator_item(t).into_iter().chain(into)
        };
        let given: Vec<_> = tys
            .iter()
            .copied()
            .enumerate()
            .flat_map(|(i, t)| cells(t).chain(items((i, t)).flat_map(cells)))
            .chain(
                generic_args
                    .types()
                    .flat_map(|t| cells(t).chain(self.iterator_item(t).into_iter().flat_map(cells))),
            )
            .collect();
        let mut made = cells(output).chain(self.iterator_item(output).into_iter().flat_map(cells));
        made.find(|t| !given.contains(t))
    }

    /// `handle_helper`, of the receiver's type.
    fn handle_helper_of(&self, known: Std, receiver: Ty<'tcx>) -> Option<&'static str> {
        // A `Box<[T; N]>` is its array.
        let receiver = receiver.boxed_ty().unwrap_or(receiver);
        let sequence = receiver.is_array() || receiver.is_slice() || self.is_vec_like(receiver);
        let map = self.is_map(receiver) && !self.is_set(receiver);
        Some(match known {
            Std::Same | Std::Pointee if sequence => "$mutItems",
            Std::First | Std::SliceGet | Std::SliceLast if sequence => "$mutAt",
            Std::Map(MapOp::Get) if map => "$mutGet",
            Std::Map(MapOp::Iter(Part::Entries)) if map => "$mutEntries",
            Std::Map(MapOp::Iter(Part::Values)) if map => "$mutValues",
            // A `Cow`'s `to_mut()` of text or a number (ADR 0319).
            Std::Cow(CowOp::ToMut) => "$cowMut",
            _ => return None,
        })
    }

    /// A handle on each item `known` gives a `&mut` to, of its arguments'
    /// values and its receiver's type: `$mutAt(v, 0)` and the like (ADR
    /// 0152); `None` of what's given no handle.
    pub(in crate::lower) fn item_handles_of(
        &mut self,
        known: Std,
        mut values: Vec<Expr>,
        receiver: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Some(name) = self.handle_helper_of(known, receiver) else {
            return Ok(None);
        };
        match known {
            Std::First => values.push(Expr::int(0)),
            Std::SliceLast => values.push(Expr::int(-1)),
            // A B-tree's in its keys' order.
            Std::Map(MapOp::Iter(_)) if self.is_sorted(receiver) => {
                let m = values.remove(0);
                let m = if m.reads_same() {
                    m
                } else {
                    self.spill(Role::Map, m, out)
                };
                let entries = self.in_order_of(m.clone(), receiver, span)?;
                values = vec![m, entries];
            }
            _ => {}
        }
        Ok(Some(Expr::call_helper(name, values)))
    }

    /// Whether `declared`, a type parameter of `def_id`'s, given a `&mut` to
    /// a function or an iterator, `instantiated` its caller's type, takes it
    /// as it is, not in generic code's box: where its value is read as
    /// another type parameter its bound names that's called or stepped,
    /// `P: Is<F>` of `F: FnMut()`, `I: Iterator` or `I: IntoIterator`, as
    /// that one is given it, its form's (`mut_ref_form`), the function, the
    /// iterator or the range itself (#992). A collection given for an
    /// `IntoIterator` is boxed still, as its dictionary takes its cell.
    pub(in crate::lower) fn given_as_the_value(
        &self,
        def_id: DefId,
        declared: Ty<'tcx>,
        instantiated: Ty<'tcx>,
    ) -> bool {
        matches!(declared.kind(), ty::Param(_))
            && matches!(*instantiated.kind(),
                ty::Ref(_, pointee, Mutability::Mut) if self.is_callable(pointee) || self.implements_iterator(pointee))
            && self.read_as_stepped(self.tcx.param_env(def_id), declared)
    }

    /// Whether `param`'s value is read as another type parameter its bound
    /// names, `P: Is<F>`, that's called or stepped, bound by a function
    /// trait, `Iterator` or `IntoIterator`, which is given a function or an
    /// iterator as it is (#992).
    fn read_as_stepped(&self, param_env: ty::ParamEnv<'tcx>, param: Ty<'tcx>) -> bool {
        (param_env.caller_bounds().iter())
            .filter_map(|clause| clause.as_trait_clause())
            .filter(|bound| bound.self_ty().skip_binder() == param)
            .flat_map(|bound| bound.skip_binder().trait_ref.args.types().skip(1))
            .any(|named| {
                matches!(named.kind(), ty::Param(_))
                    && self.declared_bounds(named, param_env).into_iter().any(|id| {
                        self.tcx.fn_trait_kind_from_def_id(id).is_some()
                            || crate::lower::recognition::is_std_def(
                                self.tcx,
                                id,
                                crate::lower::recognition::StdItem::Iterator,
                            )
                            || crate::lower::recognition::is_std_def(
                                self.tcx,
                                id,
                                crate::lower::recognition::StdItem::IntoIterator,
                            )
                    })
            })
    }

    /// Of a function value given for `def_id`'s parameter `i`, an `F:
    /// FnOnce(&mut T)` of `and_then_or_clear`'s, in a call of it,
    /// `generic_args`: which of the function's parameters generic code gives
    /// in its own form where the caller has another, a box of what the
    /// caller has as an object, `&mut T` of a `T` that's one here (ADR 0454),
    /// or a handle of a raw pointer the caller has as the value, a slice's
    /// (ADR 1114), each given the function as `x.value`, or a box of a
    /// `&mut` to an object, given as `$heldObjectOf(x)` (#1322), each its
    /// form and crossing, given back by `form_back`; and the form of what
    /// it gives back, a raw pointer's given generic code as `$ptrTo(r)`.
    /// `None` where none is. A function value takes its callers' forms
    /// (#1324); where generic code's are its own, the function itself is
    /// given (`takes_forms_given`).
    pub(super) fn unboxed_by_function(
        &self,
        def_id: DefId,
        i: usize,
        generic_args: ty::GenericArgsRef<'tcx>,
    ) -> Option<FormsGiven<'tcx>> {
        let sig = self
            .tcx
            .fn_sig(def_id)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder();
        let ty::Param(_) = sig.inputs().get(i)?.kind() else {
            return None;
        };
        let given_as = sig.inputs()[i];
        let clauses: Vec<_> = (self.tcx.clauses_of(def_id).instantiate_identity(self.tcx).into_iter())
            .map(|(clause, _)| clause.skip_normalization())
            .collect();
        let fn_bound = |f: Ty<'tcx>| {
            (clauses.iter())
                .filter_map(|clause| clause.as_trait_clause())
                .map(|bound| bound.skip_binder().trait_ref)
                .find(|tr| tr.self_ty() == f && self.tcx.fn_trait_kind_from_def_id(tr.def_id).is_some())
        };
        // Its own bound, or one of a type parameter its bound names that its
        // value is read as, `P: Is<F>` of `F: Fn(*mut T)` (#992).
        let f = std::iter::once(given_as)
            .chain(
                (clauses.iter())
                    .filter_map(|clause| clause.as_trait_clause())
                    .map(|bound| bound.skip_binder().trait_ref)
                    .filter(|tr| tr.self_ty() == given_as)
                    .flat_map(|tr| tr.args.types().skip(1).collect::<Vec<_>>()),
            )
            .find(|&f| fn_bound(f).is_some())?;
        let unboxed: Vec<Option<(Form, Crossing<'tcx>)>> = fn_bound(f)?
            .args
            .type_at(1)
            .tuple_fields()
            .iter()
            .map(|passed| {
                let crossing = (def_id, passed, self.instantiated(passed, generic_args));
                self.form_of(crossing).map(|form| (form, crossing))
            })
            .collect();
        // What it gives back, its bound's `Output`.
        let output = (clauses.iter())
            .filter_map(|clause| clause.as_projection_clause())
            .map(|projection| projection.skip_binder())
            .find(|projection| {
                projection.projection_term.self_ty() == f
                    && self.tcx.is_lang_item(
                        projection.def_id(),
                        rustc_hir::attrs::lang_items::LangItem::FnOnceOutput,
                    )
            })
            .and_then(|projection| projection.term.as_type());
        let back = output.and_then(|out| self.form_of_result((def_id, out, self.instantiated(out, generic_args))));
        (unboxed.iter().any(Option::is_some) || matches!(back, Some(Form::Handle | Form::Tagged | Form::Valued)))
            .then_some((unboxed, back))
    }

    /// Whether a generic function's instance `def_id` given `args`, given
    /// generic code that calls it as an `F` in the forms `given`
    /// (`unboxed_by_function`'s), takes them itself: its value takes its
    /// callers' forms where they differ from generic code's (#1324), and its
    /// JS function, generic code's own, takes each form generic code gives
    /// and gives back the form generic code takes back, so it's given
    /// itself, `apply_mut(.., replace_with)` (#975).
    pub(in crate::lower) fn takes_forms_given(
        &self,
        (def_id, args): (DefId, ty::GenericArgsRef<'tcx>),
        (params, back): &FormsGiven<'tcx>,
    ) -> bool {
        if !self.reified_takes_generic_forms(def_id, args) {
            return false;
        }
        let sig = self
            .tcx
            .fn_sig(def_id)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder();
        let own = |declared: Ty<'tcx>| self.form_of((def_id, declared, self.instantiated(declared, args)));
        sig.inputs().len() == params.len()
            && (sig.inputs().iter().zip(params)).all(|(&declared, given)| own(declared) == given.map(|(form, _)| form))
            && own(sig.output()) == *back
    }
    /// Whether a generic function's instance `def_id` given `args`, as a
    /// `fn` pointer, would be given its callers' forms where its value takes
    /// generic code's, as generic code that calls it as an `F` gives them
    /// (#1187): a `&mut` of its own type parameter, generic code's box,
    /// which its instance makes an object's or a slice's, not a cell's; or a
    /// raw pointer of one, generic code's handle, which its instance makes a
    /// slice's, a pair `dyn`'s or a JS object's, the value itself (ADR 1114).
    /// Not a trait's method, whose value is its impl's or its copied
    /// default's, of the instance's own types; a function of the crate's or
    /// a library's, whose JS rust-js wrote.
    pub(in crate::lower) fn reified_takes_generic_forms(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
        if self.tcx.trait_of_assoc(def_id).is_some()
            || !(self.is_rust_fn(def_id) || self.krate.foreign.item(def_id).is_some())
        {
            return false;
        }
        let sig = self
            .tcx
            .fn_sig(def_id)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder();
        (sig.inputs().iter().chain([&sig.output()])).any(|&declared| {
            self.form_of((def_id, declared, self.instantiated(declared, args)))
                .is_some()
        })
    }

    /// The methods of `trait_id`, a trait of the crate's, that generic code
    /// calls through its dictionary: each of no type or const parameters of
    /// its own, whose entry takes their evidence as well, and not a binding.
    fn dispatched_methods(&self, trait_id: DefId) -> Vec<DefId> {
        (self.tcx.associated_items(trait_id).in_definition_order())
            .filter(|item| self.tcx.def_kind(item.def_id) == rustc_hir::def::DefKind::AssocFn)
            .map(|item| item.def_id)
            .filter(|&method| !crate::lower::bindings::is_binding(self.tcx, method))
            .filter(|&method| {
                (self.tcx.generics_of(method).own_params.iter())
                    .all(|param| matches!(param.kind, ty::GenericParamDefKind::Lifetime))
            })
            .collect()
    }

    /// Of a type in `def_id`'s signature, or a bound trait's method's, a
    /// parameter or the result a box is made of or taken out of, its `&mut`
    /// itself: what it's made of past that, `None` where that's all it is.
    fn past_own(&self, ty: Ty<'tcx>, param_env: ty::ParamEnv<'tcx>) -> Option<Ty<'tcx>> {
        let mut at = ty;
        while let ty::Ref(_, inner, Mutability::Not) = *at.kind() {
            at = inner;
        }
        match *at.kind() {
            ty::Ref(..) if self.is_generic_box_in(at, param_env) => None,
            ty::Param(_) if self.mut_ref_form_in(at, param_env).is_generic_box() => None,
            _ => Some(ty),
        }
    }

    /// What `declared`, a type of generic code's whose type parameters `env`
    /// says what they're bound by, holds of a `&mut` that's generic code's
    /// box, given as `actual`, the impl's type, is (`Within`): the one
    /// answer of what a dictionary's entry adapts and of what a call that
    /// gives one is refused.
    pub(in crate::lower) fn within(
        &self,
        declared: Ty<'tcx>,
        env: ty::ParamEnv<'tcx>,
        actual: Ty<'tcx>,
    ) -> Within<'tcx> {
        let leaf = |ty: Ty<'tcx>| self.is_generic_box_in(ty, env);
        let form = |given: Ty<'tcx>| self.mut_ref_form(given);
        within_shape(self.tcx, (declared, actual), (&leaf, &form), &|payload| {
            !self.boxed_payload(payload)
        })
    }

    /// What generic code, `def_id`'s, called with `generic_args`, gives a
    /// method of a trait of the crate's that `bound` is, its type
    /// parameter's, `S: Pick`, through its dictionary, past each parameter's
    /// and the result's own `&mut` and what `within` says the impl's entry
    /// adapts, `Option<&mut Self>`'s `&mut` (#686): what it gives as it is,
    /// `Vec<&mut Self>` (#682). None of a method of `def_id`'s own trait,
    /// which it is.
    fn dispatched_takes(
        &self,
        def_id: DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
        bound: ty::PolyTraitPredicate<'tcx>,
    ) -> Vec<Ty<'tcx>> {
        let args = bound.skip_binder().trait_ref.args;
        // Of a trait rust-js compiled, a library's too, whose impls' entries
        // its consumer's calls give as its own compile's do.
        if !self.krate.foreign.compiled_by_rust_js(bound.def_id())
            || !matches!(args.type_at(0).kind(), ty::Param(_))
            || self.tcx.trait_of_assoc(def_id) == Some(bound.def_id())
        {
            return Vec::new();
        }
        let param_env = self.tcx.param_env(def_id);
        let mut takes = Vec::new();
        for method in self.dispatched_methods(bound.def_id()) {
            let method_args =
                ty::GenericArgs::for_item(self.tcx, method, |param, _| match (param.index as usize) < args.len() {
                    true => args[param.index as usize],
                    false => self.tcx.lifetimes.re_erased.into(),
                });
            let method_sig = (self.tcx.fn_sig(method).instantiate(self.tcx, method_args))
                .skip_normalization()
                .skip_binder();
            for ty in (method_sig.inputs_and_output.iter()).filter_map(|ty| self.past_own(ty, param_env)) {
                match adapted_part(ty) {
                    Some(part) => {
                        (self.within(part, param_env, self.instantiated(part, generic_args))).given_as_it_is(&mut takes)
                    }
                    None => takes.push(ty),
                }
            }
        }
        takes
    }

    /// A `&mut` to an object that a generic impl of the crate's, `impl_id`,
    /// given `impl_args`, gives a method of a trait its type parameter is
    /// bound by within what it takes, `impl<T: Pick> Wrap for Holder<T>`'s
    /// `T::pick(.., vec![&mut ..])`, through `T`'s dictionary, whose entry
    /// gives its impl generic code's box where it doesn't adapt it, as it
    /// does `Some(&mut ..)` (#685, #686): where its dictionary is made, as
    /// `nested_mut_object` asks of a function's call.
    pub(in crate::lower) fn impl_nested_mut_object(
        &self,
        impl_id: DefId,
        impl_args: ty::GenericArgsRef<'tcx>,
    ) -> Option<Ty<'tcx>> {
        let todo = (self.tcx.clauses_of(impl_id).instantiate_identity(self.tcx).into_iter())
            .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
            .flat_map(|bound| self.dispatched_takes(impl_id, impl_args, bound))
            .collect();
        self.nested_mut_within(impl_id, impl_args, todo)
    }

    /// A `&mut` to one of `def_id`'s type parameters that's an object in a
    /// call of it, `generic_args`, anywhere but a parameter or its result: in
    /// a field of the crate's own type, an `Option` or a `Vec`, or within
    /// what a method of a trait it's bound by takes or gives (#682); not a
    /// function's parameter through `F: Fn(&mut T)`, which the function
    /// given takes out of generic code's box (`unboxed_by_function`). Generic
    /// code has a cell there (ADR 0099), and the caller the object; a
    /// parameter is given a box, and the result's taken out.
    pub(super) fn nested_mut_object(&self, def_id: DefId, generic_args: ty::GenericArgsRef<'tcx>) -> Option<Ty<'tcx>> {
        let sig = self
            .tcx
            .fn_sig(def_id)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder();
        let param_env = self.tcx.param_env(def_id);
        let mut todo: Vec<Ty<'tcx>> = (sig.inputs_and_output.iter())
            .filter_map(|ty| self.past_own(ty, param_env))
            .collect();
        // A bound's types, but its own, `D: Get`'s `D`, which isn't a value:
        // a closure's parameters, `F: Fn(D)`, are.
        let mut given = Vec::new();
        for (clause, _) in self.tcx.clauses_of(def_id).instantiate_identity(self.tcx) {
            let clause = clause.skip_normalization();
            // A function's parameters, `F: FnOnce(&mut T)`'s, past each
            // `&mut`, which the function given takes out of the box.
            let walked = |trait_id: DefId, args: ty::GenericArgsRef<'tcx>| -> Vec<Ty<'tcx>> {
                match self.tcx.fn_trait_kind_from_def_id(trait_id).is_some() {
                    true => (args.type_at(1).tuple_fields().iter())
                        .map(|given| match *given.kind() {
                            ty::Ref(_, pointee, Mutability::Mut) => pointee,
                            _ => given,
                        })
                        .collect(),
                    false => args.types().skip(1).collect(),
                }
            };
            if let Some(bound) = clause.as_trait_clause() {
                let args = bound.skip_binder().trait_ref.args;
                todo.extend(walked(bound.def_id(), args));
                given.push(args.type_at(0));
                todo.extend(self.dispatched_takes(def_id, generic_args, bound));
            }
            if let Some(projection) = clause.as_projection_clause() {
                let projection = projection.skip_binder();
                let args = projection.projection_term.args;
                todo.extend(walked(projection.projection_term.trait_def_id(self.tcx), args));
                given.extend(args.types().next());
                todo.extend(projection.term.as_type());
            }
        }
        // A bound's own type is walked past itself, `Vec<D>: Trait`'s `D`; a
        // `&mut` one's, `for<'a> &'a mut C: IntoIterator`'s, from what it's
        // to, its dictionary taking the box generic code has (ADR 0457).
        for ty in given {
            match *ty.kind() {
                ty::Param(_) => {}
                ty::Ref(_, inner, Mutability::Mut) => todo.push(inner),
                _ => todo.push(ty),
            }
        }
        self.nested_mut_within(def_id, generic_args, todo)
    }

    /// Of `todo`, types of `def_id`'s, a generic function's or impl's, past
    /// what a box is made of: a `&mut` to an object, in a call of it,
    /// `generic_args`, that generic code has as a box (`nested_mut_object`).
    fn nested_mut_within(
        &self,
        def_id: DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
        mut todo: Vec<Ty<'tcx>>,
    ) -> Option<Ty<'tcx>> {
        // Each field's type in `def_id`'s own terms, as its signature is: in
        // the caller's, `I::Item` could be the caller's `A`.
        let own_env = ty::TypingEnv::non_body_analysis(self.tcx, def_id);
        let field_ty = |field: &ty::FieldDef, args| {
            let ty = field.ty(self.tcx, args).skip_normalization();
            (self.tcx)
                .try_normalize_erasing_regions(own_env, ty::Unnormalized::new_wip(ty))
                .unwrap_or(ty)
        };
        let mut seen = IndexSet::default();
        while let Some(ty) = todo.pop() {
            let mut walk = ty.walk();
            while let Some(part) = walk.next() {
                let Some(part) = part.as_type() else {
                    continue;
                };
                // A `PhantomData` holds no value: anyhow's `Mut<'a, T>`'s
                // `PhantomData<&'a mut T>`, which only marks its lifetime, is
                // no `&mut` generic code has (#516).
                if let ty::Adt(marker, _) = part.kind()
                    && marker.is_phantom_data()
                {
                    walk.skip_current_subtree();
                    continue;
                }
                if !seen.insert(part) {
                    continue;
                }
                // A type parameter given a `&mut` to an object, past a
                // parameter or the result: generic code has a box, and the
                // caller the object (ADR 0454).
                if matches!(part.kind(), ty::Param(_) | ty::Ref(_, _, Mutability::Mut)) {
                    let here = self.instantiated(part, generic_args);
                    // Named as its caller has it, its lifetime erased:
                    // `&mut C`.
                    if self.form_differs(Form::Box, (def_id, part, here))
                        && let ty::Ref(_, pointee, Mutability::Mut) = *here.kind()
                    {
                        return Some(Ty::new_mut_ref(self.tcx, self.tcx.lifetimes.re_erased, pointee));
                    }
                }
                // A field held as generic code's box wherever its type is,
                // `Slot { at: &mut T }`'s (`field_crossing`, #1405): only
                // what it's to walked, which may hold another `&mut`. Its
                // type's arguments are its fields', walked there alone:
                // `ParserI<&mut Parser>`'s is its field's.
                if let ty::Adt(adt, args) = *part.kind()
                    && !self.is_std(adt.did())
                {
                    walk.skip_current_subtree();
                    let here = self.instantiated(part, generic_args);
                    for (variant, fields) in adt.variants().iter_enumerated() {
                        for (i, field) in fields.fields.iter_enumerated() {
                            let ty = field_ty(field, args);
                            match self.field_crossing(here, variant, i) {
                                None => todo.push(ty),
                                Some(_) => todo.extend(match *ty.kind() {
                                    ty::Ref(_, pointee, Mutability::Mut) => Some(pointee),
                                    _ => None,
                                }),
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Can the borrow `fn_id`'s parameter `i` is given outlive the call: does
    /// its result's type, or another parameter's, which may keep it, name
    /// the borrow's lifetime (ADR 0099)? `keep<'a>(v: &mut
    /// Vec<&'a mut u32>, x: &'a mut u32)` keeps `x` in `v`. And a bound
    /// that names it, of what another parameter is, `v: &mut impl
    /// GetMut<Vec<&'a mut u32>>` (#973); of a parameter that's a type
    /// parameter given the borrow, `into_iter(self)`'s `Self` of `&mut C`,
    /// another parameter or the result that names that type parameter,
    /// `Self::IntoIter`, which may hold it (#973).
    /// Whether a function of parameters `inputs` and result `output` may let
    /// the reference its parameter `i` is given leave as a raw pointer,
    /// which has no lifetime to say so: a raw pointer in its result, or in
    /// another parameter behind a reference or a pointer, where the callee
    /// may store one, `out: &mut *mut T`; not a raw pointer given by value,
    /// `ptr::hash(p, state)`'s `p`, which takes nothing out (ADR 1114). A
    /// raw pointer in a field of the crate's own type too, lock_api's
    /// `ExclusiveGuardData<T>(*mut T)` (#1283). Of its signature alone, so
    /// each call of it, direct, a dictionary's, a function pointer's, a
    /// closure's or a library's, agrees.
    fn lets_pointers_out_of(&self, (inputs, output): (&[Ty<'tcx>], Ty<'tcx>), i: usize) -> bool {
        let raw = |ty: Ty<'tcx>| self.holds_a_raw_pointer(ty, &mut Vec::new());
        let stores = |ty: Ty<'tcx>| {
            ty.walk().any(|part| {
                part.as_type()
                    .and_then(|t| (t.is_ref() || t.is_raw_ptr()).then(|| t.builtin_deref(true)).flatten())
                    .is_some_and(raw)
            })
        };
        inputs.get(i).is_some_and(|input| input.is_ref())
            && (raw(output) || (inputs.iter().enumerate()).any(|(j, &ty)| j != i && stores(ty)))
    }

    /// Whether `ty` holds a raw pointer: in it, or in a field of the
    /// crate's own type in it, `seen` those walked already.
    fn holds_a_raw_pointer(&self, ty: Ty<'tcx>, seen: &mut Vec<DefId>) -> bool {
        let mut walk = ty.walk();
        while let Some(part) = walk.next() {
            let Some(part) = part.as_type() else {
                continue;
            };
            match *part.kind() {
                _ if part.is_raw_ptr() => return true,
                // A `PhantomData` holds no value, a marker's `*const ()` none.
                ty::Adt(marker, _) if marker.is_phantom_data() => walk.skip_current_subtree(),
                ty::Adt(adt, args) if !self.is_std(adt.did()) && !seen.contains(&adt.did()) => {
                    seen.push(adt.did());
                    if (adt.all_fields())
                        .any(|field| self.holds_a_raw_pointer(field.ty(self.tcx, args).skip_normalization(), seen))
                    {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// `lets_pointers_out_of` of the function `fn_id`.
    pub(in crate::lower) fn lets_pointers_out(&self, fn_id: DefId, i: usize) -> bool {
        let (inputs, output) = self.signature_of(fn_id);
        self.lets_pointers_out_of((&inputs, output), i)
    }

    /// Whether a call of `fn_id` is given, for its parameter `i`, the
    /// pointer to the place a reference refers to, not a box of a copy:
    /// where it may let it out as a raw pointer (`lets_pointers_out`) and
    /// the parameter is a box, a number's or generic code's (ADR 1114). A
    /// parameter that's the object is given the object, whose pointer,
    /// `$ptrTo(object)`, is one for the object.
    pub(in crate::lower) fn given_its_pointer(&self, fn_id: DefId, i: usize) -> bool {
        self.lets_pointers_out(fn_id, i) && self.param_is_box(fn_id, i)
    }

    /// `given_its_pointer` of a function value of signature `sig`, a
    /// function pointer's or a closure's, whose parameters are its types'.
    pub(in crate::lower) fn sig_given_its_pointer(&self, (inputs, output): (&[Ty<'tcx>], Ty<'tcx>), i: usize) -> bool {
        self.lets_pointers_out_of((inputs, output), i)
            && matches!(*inputs[i].kind(), ty::Ref(_, pointee, Mutability::Mut) if self.mut_ref_form(pointee).is_cell())
    }

    /// The parameters and result of `fn_id`, a closure's its own, untupled.
    pub(in crate::lower) fn signature_of(&self, fn_id: DefId) -> (Vec<Ty<'tcx>>, Ty<'tcx>) {
        if self.tcx.is_closure_like(fn_id)
            && let ty::Closure(_, args) = *self
                .tcx
                .type_of(fn_id)
                .instantiate_identity()
                .skip_normalization()
                .kind()
        {
            let sig = args.as_closure().sig().skip_binder();
            let params = match sig.inputs().first().map(|t| *t.kind()) {
                Some(ty::Tuple(params)) => params.to_vec(),
                _ => sig.inputs().to_vec(),
            };
            return (params, sig.output());
        }
        // A coroutine's, an `async` block's, which takes nothing it can let out.
        if self.tcx.is_closure_like(fn_id) {
            return (Vec::new(), self.tcx.types.unit);
        }
        let sig = self
            .tcx
            .fn_sig(fn_id)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder();
        (sig.inputs().to_vec(), sig.output())
    }

    /// Whether `local` is a parameter of the function or closure `body`
    /// is, given the pointer to its place (`given_its_pointer`); a closure's
    /// parameters numbered after what it captures.
    pub(in crate::lower) fn given_its_pointer_here(
        &self,
        body: &rustc_middle::mir::Body<'tcx>,
        local: rustc_middle::mir::Local,
    ) -> bool {
        let fn_id = body.source.def_id();
        let closure = self.tcx.is_closure_like(fn_id);
        let first = if closure { 2 } else { 1 };
        let index = local.as_usize();
        let (inputs, output) = self.signature_of(fn_id);
        (first..first + inputs.len()).contains(&index)
            && match closure {
                true => self.sig_given_its_pointer((&inputs, output), index - first),
                // Or one whose borrow outlives the call, which its callers
                // give a handle on its place, the place's pointer, httparse's
                // `deinit_slice_mut` (#1283).
                false => {
                    self.given_its_pointer(fn_id, index - first)
                        || (self.param_is_box(fn_id, index - first) && self.borrow_outlives_call(fn_id, index - first))
                }
            }
    }

    pub(super) fn borrow_outlives_call(&self, fn_id: DefId, i: usize) -> bool {
        let sig = self
            .tcx
            .fn_sig(fn_id)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder();
        let Some(&input) = sig.inputs().get(i) else {
            return false;
        };
        let regions = |ty: Ty<'tcx>| ty.walk().filter_map(|part| part.as_region()).collect::<Vec<_>>();
        // The borrow's own lifetime, not what it refers to's: `pick(slot:
        // &mut &'a [i32], other: &'a [i32])` replaces the slice, keeping
        // nothing of the borrow.
        let lent = match *input.kind() {
            ty::Ref(region, ..) => vec![region],
            _ => regions(input),
        };
        let others = sig
            .inputs()
            .iter()
            .enumerate()
            .filter(|&(j, _)| j != i)
            .map(|(_, &ty)| ty);
        // And a `'static` reference to what the borrow refers to, which only
        // `unsafe` code makes of it, a `transmute` that erases its lifetime
        // as yoke's does (ADR 0562): the same place, given back.
        let erased = |ty: Ty<'tcx>| self.tcx.erase_and_anonymize_regions(ty);
        let lent_to = input.builtin_deref(true).map(erased);
        // Of a type parameter, `&'a mut impl GetMut<String>`'s, which may be
        // what the `'static` reference is to (#975).
        let unknown_to = lent_to.is_some_and(|to| matches!(to.kind(), ty::Param(_)));
        let static_to_it = sig.output().walk().any(|part| match part.as_type().map(|t| *t.kind()) {
            Some(ty::Ref(region, pointee, mutability)) if region.is_static() => {
                Some(erased(pointee)) == lent_to || (unknown_to && mutability.is_mut())
            }
            _ => false,
        });
        // What the function's bounds say of its type parameters, a trait's
        // arguments and an associated type's: not what outlives what.
        let bounds: Vec<Ty<'tcx>> = (self.tcx.clauses_of(fn_id).instantiate_identity(self.tcx).into_iter())
            .filter_map(|(clause, _)| {
                let clause = clause.skip_normalization();
                let kept = clause
                    .as_trait_clause()
                    .map(|p| p.skip_binder().trait_ref.args)
                    .or_else(|| {
                        clause
                            .as_projection_clause()
                            .map(|p| p.skip_binder().projection_term.args)
                    });
                let term = clause
                    .as_projection_clause()
                    .and_then(|p| p.skip_binder().term.as_type());
                kept.map(|args| (args, term))
            })
            .flat_map(|(args, term)| args.types().skip(1).chain(term))
            .collect();
        let named = |ty: Ty<'tcx>| regions(ty).iter().any(|region| lent.contains(region));
        // A type parameter given the borrow, named again where what's kept
        // is: in another parameter's type or the result's.
        let given_as = matches!(input.kind(), ty::Param(_)).then_some(input);
        let holds_it = |ty: Ty<'tcx>| given_as.is_some_and(|param| ty.walk().any(|part| part.as_type() == Some(param)));
        static_to_it
            || (others.chain([sig.output()])).any(|ty| named(ty) || holds_it(ty))
            || bounds.into_iter().any(named)
    }

    /// What the call of `def_id`, given `generic_args`, gives, `value`, as
    /// its caller has it: a `&mut` generic code boxes, its `value` (ADR 0099).
    /// A trait's method, called through a dictionary, by the trait's
    /// signature, which its dictionary's entry gives (ADR 0454).
    pub(in crate::lower) fn generic_result_values(
        &self,
        (def_id, generic_args): (DefId, ty::GenericArgsRef<'tcx>),
        value: Expr,
        span: Span,
    ) -> R<Expr> {
        if !self.is_rust_fn(def_id) && self.tcx.trait_of_assoc(def_id).is_none() {
            return Ok(value);
        }
        if self.is_rust_fn(def_id)
            && let Some(here) = self.nested_mut_object(def_id, generic_args)
        {
            let what = format!("a `{here}` inside a generic function's parameters or result (#1421)");
            return Err(self.unsupported(span, &what));
        }
        let declared = self
            .tcx
            .fn_sig(def_id)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder()
            .output();
        // In the caller's form: a raw pointer of a type parameter's, a
        // handle in generic code, a slice's the array (ADR 1114); a `&mut`
        // generic code boxes, an object's the object (ADR 0454); a `&mut`
        // to a type parameter given a `&mut` to an object, a box of its
        // box, a box of the object (#1303); a generic `Result` of a write's
        // result, `undefined` or its error, `$writeOf(r)` (ADR 0461,
        // amended, #1408), one held inside it refused.
        let crossing = (def_id, declared, self.instantiated(declared, generic_args));
        if let Some(held) = self.write_result_inside(crossing) {
            return Err(self.unsupported(span, &crate::lower::generic_facts::held_write_result(held)));
        }
        Ok(match self.form_of_result(crossing) {
            Some(form) => self.form_back(form, crossing, value),
            None => value,
        })
    }
}
