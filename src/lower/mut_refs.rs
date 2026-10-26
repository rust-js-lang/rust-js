//! A `&mut` to a value JS can't change in place, given to a call or given
//! back by one: a box the caller copies back, the place itself, or the
//! items a std call holds (ADRs 0072, 0099).

use super::std_types::cow::CowOp;
use super::std_types::map::{MapOp, Part};
use super::{FnCx, R, Std};
use crate::js::{self, Expr, Prop, Stmt, StmtKind};
use rustc_ast::Mutability;
use rustc_middle::thir::{ExprId, ExprKind};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;
use std::collections::HashSet;

/// Whether `fn_id` may move the value its `&mut` parameter `i` refers to out
/// and keep it, `mem::replace`'s old value: where the parameter is a `&mut
/// T`, not a type parameter given the `&mut` itself, and a `T` is in what it
/// gives back or in another parameter, which it could be put in. Not in a
/// projection of it, `Self::Item`, which isn't the `T`.
pub(super) fn may_move_out<'tcx>(tcx: ty::TyCtxt<'tcx>, fn_id: DefId, i: usize) -> bool {
    let sig = tcx
        .fn_sig(fn_id)
        .instantiate_identity()
        .skip_normalization()
        .skip_binder();
    let Some(&ty::Ref(_, pointee, Mutability::Mut)) = sig.inputs().get(i).map(|input| input.kind()) else {
        return false;
    };
    let holds = |ty: Ty<'tcx>| {
        let mut walk = ty.walk();
        while let Some(part) = walk.next() {
            match part.as_type() {
                Some(part) if part == pointee => return true,
                Some(part) if matches!(part.kind(), ty::Alias(..)) => walk.skip_current_subtree(),
                _ => {}
            }
        }
        false
    };
    let others = sig
        .inputs()
        .iter()
        .enumerate()
        .filter(|&(j, _)| j != i)
        .map(|(_, &input)| input);
    holds(sig.output()) || others.into_iter().any(holds)
}

/// A box of `place`, `{ value: place }`: a `&mut` to a value JS can't change
/// in place, given to a call, which reads and writes its `value` (ADR 0074).
pub(super) fn boxed(place: Expr) -> Expr {
    Expr::object(vec![Prop::Field(js::HELD.into(), place)])
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
            ty::Ref(_, pointee, Mutability::Mut) if self.is_generic_boxed(pointee, param_env) => {
                self.instantiated(pointee, generic_args)
            }
            ty::Param(_) if self.is_generic_boxed(ty, param_env) => match *self.instantiated(ty, generic_args).kind() {
                ty::Ref(_, pointee, Mutability::Mut) => pointee,
                _ => return None,
            },
            _ => return None,
        };
        (!self.is_cell_pointee(pointee)).then(|| Ty::new_mut_ref(self.tcx, self.tcx.lifetimes.re_erased, pointee))
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
                    if self.is_generic_boxed(pointee, param_env)
                        && !self.is_cell_pointee(self.instantiated(pointee, generic_args)))
            })
            .collect();
        unboxed.iter().any(|&unboxed| unboxed).then_some(unboxed)
    }

    /// A `&mut` to one of `def_id`'s type parameters that's an object in a
    /// call of it, `generic_args`, anywhere but a parameter or its result: in
    /// a field of the crate's own type, an `Option` or a `Vec`; not a
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
        let mut todo: Vec<Ty<'tcx>> = Vec::new();
        for ty in sig.inputs_and_output.iter() {
            let mut at = ty;
            while let ty::Ref(_, inner, Mutability::Not) = *at.kind() {
                at = inner;
            }
            match *at.kind() {
                ty::Ref(_, pointee, Mutability::Mut) if self.is_generic_boxed(pointee, param_env) => {}
                ty::Param(_) if self.is_generic_boxed(at, param_env) => {}
                _ => todo.push(ty),
            }
        }
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
            for part in ty.walk().filter_map(|part| part.as_type()) {
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
                    && self.is_generic_boxed(pointee, param_env)
                    && !self.is_cell_pointee(self.instantiated(pointee, generic_args))
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
