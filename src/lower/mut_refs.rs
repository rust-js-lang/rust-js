//! A `&mut` to a value JS can't change in place, given to a call or given
//! back by one: a box the caller copies back, the place itself, or the
//! items a std call holds (ADRs 0072, 0099).

use super::std_types::cow::CowOp;
use super::std_types::map::{MapOp, Part};
use super::{FnCx, R, Std};
use crate::js::{self, Expr, Prop, Stmt, StmtKind};
use crate::lower::MutRefForm;
use rustc_ast::Mutability;
use rustc_middle::thir::{ExprId, ExprKind};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;
use std::collections::HashSet;

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

/// An in-place cell of `object`, `$cellOf(object)`: a `&mut` to an object
/// as generic code has it, a box, whose `value` is the object and whose
/// write of the whole changes it in place, `$assign`, as `*r = v` of the
/// object's own `&mut` does (ADRs 0153, 0454). `Cell::from_mut`'s, a
/// dictionary's `&mut Self::Out`, and a `&mut` given for longer than the
/// call (#528). What's moved out of one is apart (`moved_out`).
pub(in crate::lower) fn in_place_cell(object: Expr) -> Expr {
    Expr::call_helper("$cellOf", vec![object])
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What a box, `boxed`, the call it was given to returned, writes back to
    /// its `place`: an object's changed in place, `$assign(place, box.value)`,
    /// another's assigned (ADR 0074).
    pub(super) fn copied_back(&mut self, place: Expr, boxed: &str, object: bool, at: js::Span) -> Stmt {
        let back = Expr::held(Expr::var(boxed));
        match object {
            true => StmtKind::Expr(Expr::call(Expr::helper("$assign"), vec![place, back])).at(at),
            false => StmtKind::Assign(place, back).at(at),
        }
    }

    /// What's moved out of a cell or a box of an `item`, `old`, its `value`
    /// read where a write of the whole follows, `replace`'s, `take`'s and
    /// `swap`'s: a copy apart, `$apart(old)`, of an object or a type
    /// parameter's. The cell may be an in-place one, `Cell::from_mut`'s or a
    /// dictionary's `&mut Self::Out`, `$cellOf(object)`, whose write changes
    /// the object its `value` gave in place, so what was moved out would
    /// become what's written (#527, ADR 0454, amended).
    pub(in crate::lower) fn moved_out(&self, old: Expr, item: Ty<'tcx>) -> Expr {
        match self.is_unknown(item) || self.mut_ref_form(item).written_in_place() {
            true => Expr::call_helper("$apart", vec![old]),
            false => old,
        }
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
                let m = if m.reads_same() { m } else { self.spill("map", m, out) };
                let entries = self.in_order_of(m.clone(), receiver, span)?;
                values = vec![m, entries];
            }
            _ => {}
        }
        Ok(Some(Expr::call_helper(name, values)))
    }

    /// Of `declared`, a type in `def_id`'s signature as it's written, in a
    /// call of it, `generic_args`: the `&mut` to an object generic code has
    /// as a box where its caller has the object (ADRs 0099, 0454). Through
    /// shared references, a `&mut` to its type parameter, `&mut T` of `&mut
    /// v`, or its type parameter given one, `D` of `&mut v`, an impl's for
    /// `&mut T` too. `None` of any other, and of a `&mut` to a number or
    /// text, which its caller has as a cell already.
    pub(super) fn boxed_in_call(
        &self,
        def_id: DefId,
        declared: Ty<'tcx>,
        generic_args: ty::GenericArgsRef<'tcx>,
    ) -> Option<Ty<'tcx>> {
        let param_env = self.tcx.param_env(def_id);
        let mut ty = declared;
        while let ty::Ref(_, inner, Mutability::Not) = *ty.kind() {
            ty = inner;
        }
        let pointee = match *ty.kind() {
            ty::Ref(_, pointee, Mutability::Mut)
                if self.mut_ref_form_in(pointee, param_env) == MutRefForm::GenericBox =>
            {
                self.instantiated(pointee, generic_args)
            }
            ty::Param(_) if self.mut_ref_form_in(ty, param_env) == MutRefForm::GenericBox => {
                match *self.instantiated(ty, generic_args).kind() {
                    ty::Ref(_, pointee, Mutability::Mut) => pointee,
                    _ => return None,
                }
            }
            _ => return None,
        };
        (!self.mut_ref_form(pointee).is_cell())
            .then(|| Ty::new_mut_ref(self.tcx, self.tcx.lifetimes.re_erased, pointee))
    }

    /// Of a function value given for `def_id`'s parameter `i`, an `F:
    /// FnOnce(&mut T)` of `and_then_or_clear`'s, in a call of it,
    /// `generic_args`: which of the function's parameters generic code gives
    /// a box of what the caller has as an object, `&mut T` of a `T` that's
    /// one here (ADR 0454). `None` where none is.
    pub(super) fn unboxed_by_function(
        &self,
        def_id: DefId,
        i: usize,
        generic_args: ty::GenericArgsRef<'tcx>,
    ) -> Option<Vec<bool>> {
        let sig = self
            .tcx
            .fn_sig(def_id)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder();
        let ty::Param(_) = sig.inputs().get(i)?.kind() else {
            return None;
        };
        let param_env = self.tcx.param_env(def_id);
        let unboxed: Vec<bool> = (self.tcx.clauses_of(def_id).instantiate_identity(self.tcx).into_iter())
            .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
            .map(|bound| bound.skip_binder().trait_ref)
            .find(|tr| tr.self_ty() == sig.inputs()[i] && self.tcx.fn_trait_kind_from_def_id(tr.def_id).is_some())?
            .args
            .type_at(1)
            .tuple_fields()
            .iter()
            .map(|given| {
                matches!(*given.kind(), ty::Ref(_, pointee, Mutability::Mut)
                    if self.mut_ref_form_in(pointee, param_env) == MutRefForm::GenericBox
                        && !self.mut_ref_form(self.instantiated(pointee, generic_args)).is_cell())
            })
            .collect();
        unboxed.iter().any(|&unboxed| unboxed).then_some(unboxed)
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
        // A parameter or the result a box is made of or taken out of, its
        // `&mut` itself: what it's made of past that.
        let past_own = |ty: Ty<'tcx>| -> Option<Ty<'tcx>> {
            let mut at = ty;
            while let ty::Ref(_, inner, Mutability::Not) = *at.kind() {
                at = inner;
            }
            match *at.kind() {
                ty::Ref(_, pointee, Mutability::Mut)
                    if self.mut_ref_form_in(pointee, param_env) == MutRefForm::GenericBox =>
                {
                    None
                }
                ty::Param(_) if self.mut_ref_form_in(at, param_env) == MutRefForm::GenericBox => None,
                _ => Some(ty),
            }
        };
        let mut todo: Vec<Ty<'tcx>> = sig.inputs_and_output.iter().filter_map(past_own).collect();
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
                // What generic code gives a method of a trait of the crate's
                // its type parameter is bound by, `S: Pick`, through its
                // dictionary: an impl's entry makes its own of a
                // parameter's or the result's `&mut`, not of one within,
                // `Option<&mut Self>`'s (#682).
                if bound.def_id().is_local()
                    && matches!(args.type_at(0).kind(), ty::Param(_))
                    && self.tcx.trait_of_assoc(def_id) != Some(bound.def_id())
                {
                    for method in self.dispatched_methods(bound.def_id()) {
                        let method_args = ty::GenericArgs::for_item(self.tcx, method, |param, _| {
                            match (param.index as usize) < args.len() {
                                true => args[param.index as usize],
                                false => self.tcx.lifetimes.re_erased.into(),
                            }
                        });
                        let method_sig = (self.tcx.fn_sig(method).instantiate(self.tcx, method_args))
                            .skip_normalization()
                            .skip_binder();
                        todo.extend(method_sig.inputs_and_output.iter().filter_map(past_own));
                    }
                }
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
        // Each field's type in `def_id`'s own terms, as its signature is: in
        // the caller's, `I::Item` could be the caller's `A`.
        let own_env = ty::TypingEnv::non_body_analysis(self.tcx, def_id);
        let field_ty = |field: &ty::FieldDef, args| {
            let ty = field.ty(self.tcx, args).skip_normalization();
            (self.tcx)
                .try_normalize_erasing_regions(own_env, ty::Unnormalized::new_wip(ty))
                .unwrap_or(ty)
        };
        let mut seen = HashSet::new();
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
                if let ty::Param(_) = part.kind()
                    && let Some(here) = self.boxed_in_call(def_id, part, generic_args)
                {
                    return Some(here);
                }
                if let ty::Ref(_, pointee, Mutability::Mut) = *part.kind()
                    && self.mut_ref_form_in(pointee, param_env) == MutRefForm::GenericBox
                    && !self.mut_ref_form(self.instantiated(pointee, generic_args)).is_cell()
                {
                    return Some(Ty::new_mut_ref(
                        self.tcx,
                        self.tcx.lifetimes.re_erased,
                        self.instantiated(pointee, generic_args),
                    ));
                }
                if let ty::Adt(adt, args) = *part.kind()
                    && !self.is_std(adt.did())
                {
                    todo.extend(adt.all_fields().map(|field| field_ty(field, args)));
                }
            }
        }
        None
    }

    /// Can the borrow `fn_id`'s parameter `i` is given outlive the call: does
    /// its result's type, or another parameter's, which may keep it, name
    /// the borrow's lifetime (ADR 0099)? `keep<'a>(v: &mut
    /// Vec<&'a mut u32>, x: &'a mut u32)` keeps `x` in `v`.
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
        let static_to_it = sig.output().walk().any(|part| {
            matches!(part.as_type().map(|t| *t.kind()), Some(ty::Ref(region, pointee, _))
                if region.is_static() && Some(erased(pointee)) == lent_to)
        });
        static_to_it
            || others
                .chain([sig.output()])
                .any(|ty| regions(ty).iter().any(|region| lent.contains(region)))
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
            let what = format!("a `{here}` inside a generic function's parameters or result");
            return Err(self.unsupported(span, &what));
        }
        let declared = self
            .tcx
            .fn_sig(def_id)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder()
            .output();
        // A generic `Result` of a `fmt::Error`: a `fmt::Result` here.
        if self.generic_fmt_result(declared, generic_args) {
            return Ok(Expr::call_helper("$writeOf", vec![value]));
        }
        Ok(match self.boxed_in_call(def_id, declared, generic_args) {
            Some(_) => Expr::held(value),
            None => value,
        })
    }
}
