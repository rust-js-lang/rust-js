//! `PartialOrd` and `Ord` (ADR 0057): a comparison is an `Ordering`, -1, 0
//! or 1 (ADR 0036), and `partial_cmp`'s `None` is `undefined`. JS's own `<`
//! for what it orders as Rust does, `$cmp`, a hand-written `cmp`, or the
//! parts compared in turn: `$cmp(a.x, b.x) || $cmp(a.y, b.y)`, since `Equal`
//! is the one that's falsy.

use super::bindings;
use super::recognition::{OrderingCall, StdItem, std_item, trait_method};
use super::representation::{Num, variant_field};
use super::{FnCx, R, Shape};
use crate::collections::IndexSet;
use crate::js::{self, Expr, Op, Stmt, StmtKind};
use rustc_abi::VariantIdx;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::traits::ImplSource;
use rustc_middle::ty::{self, Ty};
use rustc_span::def_id::DefId;
use rustc_span::{DUMMY_SP, Span};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    pub(super) fn ord_trait(&self) -> DefId {
        std_item(self.tcx, StdItem::Ord)
    }

    pub(super) fn partial_ord_trait(&self) -> DefId {
        self.tcx.require_lang_item(LangItem::PartialOrd, DUMMY_SP)
    }

    /// Whether comparing two `ty`s, by their `Ord` or `PartialOrd`, may
    /// panic: `order_may`'s answer of an impl any of whose methods may
    /// unwind (ADR 0902). The one answer of whether what std holds as it
    /// compares, a heap's, a B-tree's, is dropped as a comparison unwinds
    /// (#901, #911).
    pub(in crate::lower) fn order_may_panic(&self, ty: Ty<'tcx>) -> bool {
        order_may(&self.recognition(), ty, &|imp| self.impl_may_unwind(imp))
    }

    /// Whether a comparison of two `ty`s, by their `Ord` or `PartialOrd`,
    /// can be seen: `order_may`'s answer of an impl any of whose methods
    /// isn't quiet (#925), one that prints, counts, panics or writes. The
    /// one answer of whether a stable sort of `ty`s is std's own, step for
    /// step, where JS's `sort` would ask another sequence (ADR 0036).
    pub(in crate::lower) fn order_seen(&self, ty: Ty<'tcx>) -> bool {
        order_may(&self.recognition(), ty, &|imp| {
            !impl_quiet(self.tcx, self.krate.quiet_fns, imp)
        })
    }

    /// Whether a comparison of two `ty`s by their `PartialEq`, `==` or
    /// `!=`, can be seen: `eq_may`'s answer of an impl any of whose methods
    /// isn't quiet (#962), as `order_seen` is of their order.
    pub(in crate::lower) fn eq_seen(&self, ty: Ty<'tcx>) -> bool {
        eq_may(&self.recognition(), ty, &|imp| {
            !impl_quiet(self.tcx, self.krate.quiet_fns, imp)
        })
    }

    /// Whether comparing `ty`s, by their `Ord`, is a total order (#936), so
    /// that any sort of them leaves them in one order: std's own of
    /// numbers, text, `char`s and `bool`s, and of what holds them; a
    /// derived one's, as its fields' are; a hand-written one's whose `cmp`
    /// and `partial_cmp` `orders_totally` say is one. Not a type
    /// parameter's, a `dyn`'s or an opaque type's.
    pub(in crate::lower) fn order_total(&self, ty: Ty<'tcx>) -> bool {
        self.order_total_in(ty, &Asking::default())
    }

    /// `order_total`, of the types `seen` asks it of already, each of which
    /// is taken to be total where it's reached again, its own order's
    /// `Some(self.cmp(other))`.
    fn order_total_in(&self, ty: Ty<'tcx>, seen: &Asking<'tcx>) -> bool {
        let ty = ty.peel_refs();
        if seen.borrow().contains(&ty) {
            return true;
        }
        if let ty::Param(_) | ty::Dynamic(..) | ty::Alias(..) = ty.kind() {
            return false;
        }
        seen.borrow_mut().push(ty);
        let recognition = self.recognition();
        let impls: Vec<DefId> = [self.ord_trait(), self.partial_ord_trait()]
            .into_iter()
            .filter_map(|tr| {
                recognition.user_impl(ty::TraitRef::new_from_args(self.tcx, tr, recognition.args_of(tr, ty)))
            })
            .collect();
        let rust_adt = |id: DefId| self.krate.foreign.compiled_by_rust_js(id);
        let total = match ty.kind() {
            // A hand-written one, each of whose methods orders totally; a
            // derived one, as its fields do.
            _ if impls.iter().any(|&imp| !self.tcx.is_automatically_derived(imp)) => {
                impls
                    .iter()
                    .all(|&imp| self.tcx.is_automatically_derived(imp) || self.impl_orders_totally(imp, seen))
                    && self.fields_total(ty, seen)
            }
            ty::Adt(adt, _) if rust_adt(adt.did()) => self.fields_total(ty, seen),
            ty::Adt(_, args) => args.types().all(|arg| self.order_total_in(arg, seen)),
            ty::Tuple(parts) => parts.iter().all(|part| self.order_total_in(part, seen)),
            ty::Array(item, _) | ty::Slice(item) => self.order_total_in(*item, seen),
            _ => true,
        };
        seen.borrow_mut().pop();
        total
    }

    /// Whether each field of the crate's `ty` orders totally, a derived
    /// order's parts; true of a type whose order is all hand-written.
    fn fields_total(&self, ty: Ty<'tcx>, seen: &Asking<'tcx>) -> bool {
        let derived = [self.ord_trait(), self.partial_ord_trait()].into_iter().any(|tr| {
            let tr = ty::TraitRef::new_from_args(self.tcx, tr, self.recognition().args_of(tr, ty));
            self.recognition()
                .user_impl(tr)
                .is_some_and(|imp| self.tcx.is_automatically_derived(imp))
        });
        match ty.kind() {
            ty::Adt(adt, args) if derived || !self.has_user_impl(self.ord_trait(), ty) => {
                let recognition = self.recognition();
                (adt.all_fields()).all(|f| self.order_total_in(recognition.field_ty(f, args), seen))
            }
            _ => true,
        }
    }

    /// Whether each method the impl `imp` writes, its `cmp` or its
    /// `partial_cmp`, orders totally, as `orders_totally` reads its body.
    fn impl_orders_totally(&self, imp: DefId, seen: &Asking<'tcx>) -> bool {
        self.tcx.associated_item_def_ids(imp).iter().all(|&method| {
            !matches!(self.tcx.def_kind(method), rustc_hir::def::DefKind::AssocFn)
                || self.body_orders_totally(method, seen)
        })
    }

    /// Whether the crate's function or closure `id` gives a total order, as
    /// `orders_totally` reads its body.
    fn body_orders_totally(&self, id: DefId, seen: &Asking<'tcx>) -> bool {
        let Some(body) = self.krate.bodies.get(&id).or_else(|| self.krate.closures.get(&id)) else {
            return false;
        };
        let Some(root) = body.root() else {
            return false;
        };
        let total_ty = |ty: Ty<'tcx>| self.order_total_in(ty, seen);
        let total_fn = |f: Ty<'tcx>| self.call_total_in(f, seen);
        super::body_queries::orders_totally(self.tcx, &body.thir, root, (&total_ty, &total_fn))
    }

    /// Whether a comparator, a closure's or a function's type, gives a total
    /// order (#936): one of the crate's whose body `orders_totally` says
    /// does, or std's `Ord::cmp` of a total order, a float's `total_cmp`, or
    /// JS's `localeCompare`.
    pub(in crate::lower) fn call_total(&self, f: Ty<'tcx>) -> bool {
        self.call_total_in(f, &Asking::default())
    }

    fn call_total_in(&self, f: Ty<'tcx>, seen: &Asking<'tcx>) -> bool {
        match *self.reveal(f).kind() {
            ty::Closure(id, _) => self.body_orders_totally(id, seen),
            ty::FnDef(id, args) if id.is_local() => args.skip_binder().is_empty() && self.body_orders_totally(id, seen),
            ty::FnDef(id, args) => {
                use super::recognition::{OrderingFn, ordering_fn};
                let args = args.skip_binder();
                match ordering_fn(self.tcx, id, args) {
                    Some(OrderingFn::Cmp) => args.types().next().is_some_and(|ty| self.order_total_in(ty, seen)),
                    Some(OrderingFn::TotalCmp | OrderingFn::LocaleCompare) => true,
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// Whether a call of `f`, a closure's or a function's type, can be seen
    /// (#925): any but one of the crate's that's quiet, or std's a quiet
    /// one would call, `Ord::cmp` of what `order_seen` says isn't; of
    /// another, a `dyn Fn`'s or a type parameter's, anyone's.
    pub(in crate::lower) fn call_seen(&self, f: Ty<'tcx>) -> bool {
        match *self.reveal(f).kind() {
            ty::Closure(def_id, _) => !self.krate.quiet_fns.contains(&def_id),
            ty::FnDef(def_id, args) => {
                !self.krate.quiet_fns.contains(&def_id)
                    && !super::analysis::std_quiet(
                        (self.tcx, &self.recognition()),
                        (def_id, args.skip_binder(), false),
                        &|imp| !impl_quiet(self.tcx, self.krate.quiet_fns, imp),
                        &|id| self.krate.quiet_fns.contains(&id),
                        &|ty| self.has_drops(ty),
                    )
            }
            _ => true,
        }
    }

    /// Does JS's `<` order `ty` as Rust does? Numbers, `bool`s (`false <
    /// true`), and `Ordering`s, which are numbers. Not strings and `char`s,
    /// which `<` orders by UTF-16 units (ADR 0183).
    pub(super) fn is_primitive_ord(&self, ty: Ty<'tcx>) -> bool {
        // A `&mut` to a number is a cell, an object (ADR 0099).
        if self.has_cell_layer(ty) {
            return false;
        }
        let ty = ty.peel_refs();
        // A numbered enum whose order is derived, by its numbers, as the
        // derive orders its variants (ADR 0722).
        let numbered = matches!(ty.kind(), ty::Adt(adt, _) if bindings::numbered(self.tcx, *adt))
            && !self.has_user_impl(self.partial_ord_trait(), ty)
            && !self.has_user_impl(self.ord_trait(), ty);
        Num::of(ty).is_some() || ty.is_bool() || self.is_lang_adt(ty, LangItem::OrderingEnum) || numbered
    }

    /// What `ty` orders as: of a `#[repr(transparent)]` struct whose order
    /// is derived, its field's, as `cmp_value` orders it, the struct being
    /// its field in JS (ADR 0584); else `ty`.
    fn ordered_as(&self, ty: Ty<'tcx>) -> Ty<'tcx> {
        let derived = !self.has_user_impl(self.partial_ord_trait(), ty) && !self.has_user_impl(self.ord_trait(), ty);
        match self.transparent(ty) {
            Some(field) if derived => self.ordered_as(field),
            _ => ty,
        }
    }

    /// Whether `ty` is ordered by std as `$cmp` orders it, asking nothing a
    /// program sees: numbers but floats, `bool`s, text and `char`s, whose
    /// dictionaries are the shared `$ordered` and `$partiallyOrdered` (#944),
    /// and whose `==` is std's `===`, `$strictlyEqual` (#962): not one whose
    /// `PartialEq` is hand-written.
    pub(in crate::lower) fn quietly_ordered(&self, ty: Ty<'tcx>) -> bool {
        let float = Num::of(ty.peel_refs()).is_some_and(Num::float);
        let own_eq = self.has_user_impl(self.partial_eq_trait(), ty.peel_refs());
        ((self.is_primitive_ord(ty) && !float) || self.is_text_ord(ty)) && !own_eq
    }

    /// Whether std's `PartialEq` of `ty`s is JS's `===`, the shared
    /// `$strictlyEqual` (#962): of what `quietly_ordered` says, and of a
    /// float, whose `NaN` `===` finds unequal to itself, as `==` does.
    pub(in crate::lower) fn eq_is_strict(&self, ty: Ty<'tcx>) -> bool {
        let float = !self.has_cell_layer(ty) && Num::of(ty.peel_refs()).is_some_and(Num::float);
        self.quietly_ordered(ty) || float
    }

    /// The runtime's shared dictionary of `ty`s, of std's `Ord` if `ord`,
    /// else its `PartialOrd`, of what `quietly_ordered` says std orders as
    /// `$cmp` does: `$partiallyOrdered`; of an `Ord`, `$intsOrdered` of an
    /// integer's and `$charsOrdered` of a `char`'s, whose own `clamp` shows
    /// its bounds as it panics, and `$ordered` of the rest, which keep std's
    /// (#944, #962).
    pub(in crate::lower) fn quiet_dictionary(&self, ty: Ty<'tcx>, ord: bool) -> Option<&'static str> {
        if !self.quietly_ordered(ty) {
            return None;
        }
        let ty = ty.peel_refs();
        Some(match () {
            _ if !ord => "$partiallyOrdered",
            _ if Num::of(ty).is_some() => "$intsOrdered",
            _ if ty.is_char() => "$charsOrdered",
            _ => "$ordered",
        })
    }

    /// Strings and `char`s, which `$cmp` orders by code point (ADR 0183).
    pub(super) fn is_text_ord(&self, ty: Ty<'tcx>) -> bool {
        !self.has_cell_layer(ty) && self.is_string_like(ty.peel_refs())
    }

    /// `a < b` and the rest, of strings or `char`s, by code point, as Rust
    /// orders them (ADR 0183): JS's own operator where one is a literal whose
    /// characters are all below U+D800, which it orders the same, `c >=
    /// "a"`; else `$cmp(a, b) < 0`.
    pub(super) fn text_compare(&mut self, op: Op, a: Expr, b: Expr) -> Expr {
        let exact = |e: &Expr| matches!(&e.kind, js::ExprKind::Str(s) if s.chars().all(|c| (c as u32) < 0xd800));
        if exact(&a) || exact(&b) {
            return Expr::bin(op, a, b);
        }
        Expr::bin(op, Expr::call(Expr::helper("$cmp"), vec![a, b]), Expr::int(0))
    }

    /// What `partial_cmp` of a `ty` runs, as rustc's impls say (#937): its
    /// hand-written impl's; its `cmp`, of a derive that wrote
    /// `Some(Ord::cmp(self, other))`; else its parts' in turn.
    pub(in crate::lower) fn partial_runs(&self, ty: Ty<'tcx>) -> PartialRuns {
        let (ty, partial_ord) = (ty.peel_refs(), self.partial_ord_trait());
        let tr = ty::TraitRef::new_from_args(self.tcx, partial_ord, self.args_of(partial_ord, ty));
        // A hand-written one that's `Some(self.cmp(other))`, as clippy asks
        // it to be, does nothing but call `cmp`, which is called in its
        // place: nothing tells them apart (ADR 0262's "none").
        match self.recognition().user_impl(tr) {
            Some(imp) if self.partial_cmp_runs_cmp(imp) => PartialRuns::Cmp,
            Some(_) => PartialRuns::Own,
            None => match self.recognition().derived_impl(partial_ord, ty) {
                Some(imp) if self.partial_cmp_runs_cmp(imp) => PartialRuns::Cmp,
                _ => PartialRuns::Parts,
            },
        }
    }

    /// Whether the impl `imp`'s `partial_cmp` is `Some(self.cmp(other))`:
    /// of the crate's, as its HIR reads (`partial_cmp_is_cmp`); of a
    /// library's, as its contract says, which its own compile read so, a
    /// consumer having no HIR of it (#964).
    fn partial_cmp_runs_cmp(&self, imp: DefId) -> bool {
        match imp.is_local() {
            true => partial_cmp_is_cmp(self.tcx, imp),
            false => self.krate.foreign.partial_cmp_is_cmp(imp),
        }
    }

    /// The crate's or a library's impl of `PartialOrd` for `ty` that writes
    /// its own `<`, `<=`, `>` or `>=`, which std's comparisons call where
    /// they ask one (#937).
    pub(in crate::lower) fn own_operators(&self, ty: Ty<'tcx>) -> Option<DefId> {
        let partial_ord = self.partial_ord_trait();
        let tr = ty::TraitRef::new_from_args(self.tcx, partial_ord, self.args_of(partial_ord, ty.peel_refs()));
        (self.recognition().user_impl(tr)).filter(|&imp| writes_own_operators(self.tcx, imp))
    }

    /// `a op b`, `<`, `<=`, `>` or `>=`, of two `ty`s, as std's
    /// `PartialOrd::lt` and the rest are (#937): JS's own operator of what
    /// it orders as Rust does; a hand-written impl's own `lt`, where it
    /// writes one; else its `partial_cmp` against 0, which `None`'s
    /// `undefined` makes false. The one answer of what a comparison, a
    /// sort's `is_less`, and `max`'s and `min`'s `other < self` run.
    pub(in crate::lower) fn operator_value(
        &mut self,
        op: Op,
        a: Expr,
        b: Expr,
        ty: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let (_, inner) = self.through_refs(Expr::undefined(), ty);
        // Of a type parameter, its `PartialOrd` dictionary's, which holds
        // its impl's own or std's (#944).
        if self.is_unknown(inner) {
            let (a, _) = self.through_refs(a, ty);
            let (b, _) = self.through_refs(b, ty);
            let partial_ord = self.partial_ord_trait();
            let tr = ty::TraitRef::new_from_args(self.tcx, partial_ord, self.args_of(partial_ord, inner));
            let dictionary = self.evidence_for(tr).ok_or_else(|| self.no_evidence(span, tr))?;
            return Ok(Expr::call(Expr::member(dictionary, operator_name(op)), vec![a, b]));
        }
        // JS's own `<` orders numbers and `bool`s as Rust does, and strings
        // where one is a literal below U+D800 (ADR 0183); and a derived
        // order of a `#[repr(transparent)]` struct of one, which is its
        // field (ADR 0584). A value of any shape's `<` and `>` are JS's own;
        // its `<=` isn't, which coerces, so it's its `partial_cmp`'s, below
        // (ADR 0931).
        let field = self.ordered_as(inner);
        let shape = matches!(op, Op::Lt | Op::Gt) && any_shape(self.tcx, field);
        if self.is_primitive_ord(field) || shape || self.is_text_ord(field) {
            let (a, _) = self.through_refs(a, ty);
            let (b, _) = self.through_refs(b, ty);
            return Ok(match self.is_text_ord(field) {
                true => self.text_compare(op, a, b),
                false => Expr::bin(op, a, b),
            });
        }
        if let Some(imp) = self.own_operators(inner) {
            let name = operator_name(op);
            if writes_operator(self.tcx, imp, name) {
                let (a, _) = self.through_refs(a, ty);
                let (b, _) = self.through_refs(b, ty);
                let method = trait_method(self.tcx, self.partial_ord_trait(), name);
                return self.impl_call(method, self.args_of(self.partial_ord_trait(), inner), vec![a, b], span);
            }
        }
        let order = self.cmp_value(a, b, ty, true, span, out)?;
        Ok(Expr::bin(op, order, Expr::int(0)))
    }

    /// The `PartialOrd` dictionary of `ty`s, whose `lt`, `le`, `gt` and
    /// `ge` std's helpers that ask them are given, a heap's, `clamp`'s,
    /// `max`'s: each its impl's own or std's, as `operator_value` runs them
    /// (#937, #944).
    pub(in crate::lower) fn operators(&mut self, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let partial_ord = self.partial_ord_trait();
        let tr = ty::TraitRef::new_from_args(self.tcx, partial_ord, self.args_of(partial_ord, ty));
        self.dictionary(tr, span)
    }

    /// Is `ty` `Ord`, so that its `partial_cmp` is never `None`?
    fn is_total(&self, ty: Ty<'tcx>) -> bool {
        let tr = ty::TraitRef::new(self.tcx, self.ord_trait(), [self.tcx.erase_and_anonymize_regions(ty)]);
        matches!(
            crate::lower::traits::select(self.tcx, self.typing_env, tr),
            Ok(ImplSource::UserDefined(_) | ImplSource::Param(_) | ImplSource::Builtin(..))
        )
    }

    /// `a.cmp(&b)`, or `a.partial_cmp(&b)` if `partial`, of `ty` values: an
    /// `Ordering`, and for `partial_cmp`, `undefined` when there's none.
    pub(super) fn cmp_value(
        &mut self,
        a: Expr,
        b: Expr,
        ty: Ty<'tcx>,
        partial: bool,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let (a, _) = self.through_refs(a, ty);
        let (b, ty) = self.through_refs(b, ty);
        // `f64`'s `NaN` isn't ordered at all; nor is a value of any shape
        // JS's `<` and `>` don't order, nor finds `===` (ADR 0931).
        if partial && (Num::of(ty).is_some_and(Num::float) || any_shape(self.tcx, ty)) {
            return Ok(Expr::call(Expr::helper("$partialCmp"), vec![a, b]));
        }
        // A path's components in turn (ADR 0401).
        if self.recognition().is_path(ty) {
            return Ok(Expr::call(Expr::helper("$pathCmp"), vec![a, b]));
        }
        if self.is_primitive_ord(ty) || self.is_text_ord(ty) || self.recognition().is_os_text(ty) {
            return Ok(Expr::call(Expr::helper("$cmp"), vec![a, b]));
        }
        // A raw pointer, by its address (ADR 0562).
        if let Some((pointee, _)) = super::pointers::raw_pointee(self.tcx, ty)
            && self.is_handle_pointer(ty)
        {
            let size = self.pointee_size(pointee, span)?;
            return Ok(Expr::call(Expr::helper("$ptrCmp"), vec![a, b, size]));
        }
        let (ord, partial_ord) = (self.ord_trait(), self.partial_ord_trait());
        // Of a type parameter, its caller's impl's: `cmp` its `Ord`'s,
        // `partial_cmp` its `PartialOrd`'s, `TOrd.PartialOrd()` of an `Ord`
        // bound, as rustc's supertrait has it (#944).
        if self.is_unknown(ty) {
            if !partial {
                let total = ty::TraitRef::new(self.tcx, ord, [ty]);
                let dictionary = self.evidence_for(total).ok_or_else(|| self.no_evidence(span, total))?;
                return Ok(Expr::call(Expr::member(dictionary, "cmp"), vec![a, b]));
            }
            let tr = ty::TraitRef::new_from_args(self.tcx, partial_ord, self.args_of(partial_ord, ty));
            let dictionary = self.evidence_for(tr).ok_or_else(|| self.no_evidence(span, tr))?;
            let partial_cmp = super::bindings::trait_method_name(self.tcx, partial_ord, "partial_cmp");
            return Ok(Expr::call(Expr::member(dictionary, partial_cmp), vec![a, b]));
        }
        // What rustc's impls say each runs, never one for the other (#937):
        // a hand-written `cmp` is `Ord`'s alone, a hand-written
        // `partial_cmp` `PartialOrd`'s alone; a derived `partial_cmp` its
        // `cmp` where rustc's derive wrote it so, else its parts'.
        if !partial && self.has_user_impl(ord, ty) {
            let cmp = trait_method(self.tcx, ord, "cmp");
            return self.impl_call(cmp, self.args_of(ord, ty), vec![a, b], span);
        }
        if partial {
            match self.partial_runs(ty) {
                PartialRuns::Own => {
                    let cmp = trait_method(self.tcx, partial_ord, "partial_cmp");
                    return self.impl_call(cmp, self.args_of(partial_ord, ty), vec![a, b], span);
                }
                PartialRuns::Cmp => return self.cmp_value(a, b, ty, false, span, out),
                PartialRuns::Parts => {}
            }
        }
        // Derived, of a `#[repr(transparent)]` struct, which is its field
        // (ADR 0584): the field's order.
        if let Some(field) = self.transparent(ty) {
            return self.cmp_value(a, b, field, partial, span, out);
        }
        // A type inside itself, `C(Box<V>)`'s: its order is a function, which
        // calls itself for the ones inside, as its clone is.
        // One for each type and module (`recursive_fns`, #517).
        if self.is_recursive(ty) {
            let ty::Adt(adt, _) = ty.kind() else {
                unreachable!("a recursive type is an ADT");
            };
            let order = if partial { "partialCmp" } else { "cmp" };
            let base = format!("{order}{}", self.tcx.item_name(adt.did()));
            let name = self.recursive_fn(
                (super::recursive_fns::Kind::Cmp { partial }, ty, &base),
                &["a", "b"],
                |cx, params, body| cx.cmp_derived(params[0].clone(), params[1].clone(), ty, partial, span, body),
                out,
            )?;
            return Ok(Expr::call(Expr::var(&name), vec![a, b]));
        }
        self.cmp_derived(a, b, ty, partial, span, out)
    }

    /// `cmp_value`'s of a derived order, of what it's made of in turn.
    fn cmp_derived(
        &mut self,
        a: Expr,
        b: Expr,
        ty: Ty<'tcx>,
        partial: bool,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let (ord, partial_ord) = (self.ord_trait(), self.partial_ord_trait());
        // Each is read more than once below.
        let a = if a.reads_same() { a } else { self.spill("left", a, out) };
        let b = if b.reads_same() { b } else { self.spill("right", b, out) };
        match ty.kind() {
            _ if let Some(inner) = self.option_of(ty) => {
                // `None` is less than any `Some`.
                let (x, y) = if self.boxed_payload(inner) {
                    (self.some_value(a.clone()), self.some_value(b.clone()))
                } else {
                    (a.clone(), b.clone())
                };
                let some = self.cmp_value(x, y, inner, partial, span, out)?;
                let none = |x: &Expr| Expr::bin(Op::LooseEq, x.clone(), Expr::null());
                Ok(Expr::cond(
                    none(&a),
                    Expr::cond(none(&b), Expr::int(0), Expr::int(-1)),
                    Expr::cond(none(&b), Expr::int(1), some),
                ))
            }
            // A counted `Rc`'s `value` (ADR 0320).
            ty::Adt(_, args) if let Some(pointee) = self.counted_rc(ty) => {
                let (a, b) = (self.counted_pointee(a, pointee), self.counted_pointee(b, pointee));
                self.cmp_value(a, b, args.type_at(0), partial, span, out)
            }
            ty::Adt(_, args) if ty.is_box() || self.is_rc(ty) || self.recognition().pinned(ty).is_some() => {
                self.cmp_value(a, b, args.type_at(0), partial, span, out)
            }
            // A `Cow`: what it borrows (ADR 0400).
            ty::Adt(..) if let Some((x, borrowed)) = self.cow_deref(&a, ty) => {
                let y = self.cow_deref(&b, ty).expect("a `Cow`").0;
                self.cmp_value(x, y, borrowed, partial, span, out)
            }
            // `Reverse(x)`: `x`s the other way round, as its impl has it.
            ty::Adt(_, args) if self.is_reverse(ty) => {
                let inside = |x: Expr| Expr::index(x, Expr::int(0));
                self.cmp_value(inside(b), inside(a), args.type_at(0), partial, span, out)
            }
            // `Wrapping(x)`: its number's, as its derive has it (ADR 0175).
            ty::Adt(_, args) if self.recognition().wrapping_of(ty).is_some() => {
                let inside = |x: Expr| Expr::index(x, Expr::int(0));
                self.cmp_value(inside(a), inside(b), args.type_at(0), partial, span, out)
            }
            ty::Adt(_, args) if self.is_vec_like(ty) => self.cmp_items(a, b, args.type_at(0), partial, span),
            // A B-tree: its items in its order, as a sequence's, a map's its
            // entries, key then value.
            ty::Adt(_, args) if self.is_sorted(ty) => {
                let item = match self.is_set(ty) {
                    true => args.type_at(0),
                    false => Ty::new_tup(self.tcx, &[args.type_at(0), args.type_at(1)]),
                };
                let (a, b) = (self.in_order_of(a, ty, span)?, self.in_order_of(b, ty, span)?);
                self.cmp_items(a, b, item, partial, span)
            }
            ty::Array(item, _) | ty::Slice(item) => self.cmp_items(a, b, *item, partial, span),
            // A fieldless enum: by its discriminants, as the derive compares
            // them, `Low = 1` before `High = 2` however they're declared.
            ty::Adt(adt, _) if super::representation::is_string_enum(self.tcx, *adt) => {
                let mut variants = super::discriminants(self.tcx, *adt);
                variants.sort_by_key(|&(_, value)| value);
                let names = variants.into_iter().map(|(name, _)| Expr::str(name)).collect();
                Ok(Expr::call(Expr::helper("$cmpIn"), vec![Expr::array(names), a, b]))
            }
            // One with fields, the crate's, tagged, or std's whose order is
            // derived, `Result`'s or a path's `Component`'s: by its variant, as
            // the derive orders them, `Ok` before `Err`, then, of the same, its
            // fields in turn.
            ty::Adt(adt, args)
                if adt.is_enum()
                    && (adt.did().is_local() && !bindings::is_untagged(self.tcx, adt.did())
                        || !adt.did().is_local()
                            && !self.krate.foreign.in_library(adt.did())
                            && self.recognition().derives(if partial { partial_ord } else { ord }, ty)) =>
            {
                // Of those a value can be, in their discriminants' order, a
                // signed one's `-5` before `3` (#1131).
                let mut variants: Vec<(VariantIdx, crate::lower::Discr)> = (adt.discriminants(self.tcx))
                    .filter(|&(v, _)| !crate::lower::recognition::never_made(self.tcx, adt.variant(v)))
                    .map(|(v, d)| (v, crate::lower::Discr::of(self.tcx, d)))
                    .collect();
                variants.sort_by_key(|&(_, value)| value);
                // Where each is among them, `a.TAG === "Rect" ? 1 : ..`.
                let place = |cx: &mut Self, x: &Expr| {
                    let rest = &variants[..variants.len() - 1];
                    (rest.iter().enumerate().rev()).fold(
                        Expr::int((variants.len() - 1) as i128),
                        |others, (i, &(v, _))| {
                            Expr::cond(
                                cx.variant_test(x.clone(), ty, *adt, v, false),
                                Expr::int(i as i128),
                                others,
                            )
                        },
                    )
                };
                let (at_a, at_b) = (place(self, &a), place(self, &b));
                let order = Expr::call(Expr::helper("$cmp"), vec![at_a, at_b]);
                // Of the same variant, its fields, each in turn.
                let mut same = Expr::int(0);
                for &(v, _) in variants.iter().rev() {
                    let def = adt.variant(v);
                    if def.fields.is_empty() {
                        continue;
                    }
                    let mut orders = Vec::new();
                    let mut total = true;
                    for (i, field) in def.fields.iter().enumerate() {
                        let t = self.field_ty(field, args);
                        total &= !partial || self.is_total(t);
                        let name = variant_field(self.tcx, def, i);
                        orders.push(self.cmp_value(
                            Expr::member(a.clone(), &name),
                            Expr::member(b.clone(), &name),
                            t,
                            partial,
                            span,
                            out,
                        )?);
                    }
                    let fields = match total {
                        true => orders
                            .into_iter()
                            .reduce(|all, next| Expr::bin(Op::Or, all, next))
                            .expect("a field"),
                        false => Expr::call(Expr::helper("$thenCmp"), orders),
                    };
                    same = Expr::cond(self.variant_test(a.clone(), ty, *adt, v, false), fields, same);
                }
                Ok(Expr::bin(Op::Or, order, same))
            }
            ty::Adt(adt, _) if adt.is_enum() => Err(self.unsupported(span, &format!("comparing `{ty}`s"))),
            // Another crate's struct orders as its impl says, which may not be
            // field by field; but a derive's is, as the crate's own (ADR 0555),
            // and a `PhantomData`'s is `Equal`, as a type of one value's (#514).
            ty::Adt(adt, _)
                if !adt.did().is_local()
                    && !self.recognition().derives(if partial { partial_ord } else { ord }, ty)
                    && !self.is_lang_adt(ty, LangItem::PhantomData) =>
            {
                Err(self.unsupported(span, &format!("comparing `{ty}`s")))
            }
            // A type of one value, `()` or a struct of no fields, `struct
            // Zst;`: `Equal` to itself, as its derive has it.
            _ if self.one_value(ty) && !a.has_effects() && !b.has_effects() => Ok(Expr::int(0)),
            _ => {
                let parts: Vec<(Expr, Expr, Ty<'tcx>)> = match self.shape(ty) {
                    Shape::Object(fields) => Self::named_fields(fields)
                        .into_iter()
                        .map(|(name, t)| (Expr::member(a.clone(), name.clone()), Expr::member(b.clone(), name), t))
                        .collect(),
                    Shape::Array(tys) => tys
                        .into_iter()
                        .enumerate()
                        .map(|(i, t)| {
                            let at = |x: &Expr| Expr::index(x.clone(), Expr::int(i as i128));
                            (at(&a), at(&b), t)
                        })
                        .collect(),
                    Shape::Other => return Err(self.unsupported(span, &format!("comparing `{ty}`s"))),
                };
                // Each part in turn, until one isn't `Equal`. An unordered
                // one (`undefined`) is falsy too, so where one can be, the
                // parts go through `$thenCmp`, which stops at it.
                let total = !partial || parts.iter().all(|&(_, _, t)| self.is_total(t));
                let mut orders = Vec::new();
                for (x, y, t) in parts {
                    // A part of one value, a `PhantomData`, is `Equal`: no order.
                    if self.one_value(t) {
                        continue;
                    }
                    orders.push(self.cmp_value(x, y, t, partial, span, out)?);
                }
                if orders.is_empty() {
                    return Ok(Expr::int(0));
                }
                if total {
                    return Ok(orders
                        .into_iter()
                        .reduce(|all, next| Expr::bin(Op::Or, all, next))
                        .expect("some parts"));
                }
                Ok(Expr::call(Expr::helper("$thenCmp"), orders))
            }
        }
    }

    /// Whether `ty` has one value, `()` or a struct of no fields, which is
    /// `Equal` to itself.
    fn one_value(&self, ty: Ty<'tcx>) -> bool {
        ty.is_unit()
            || matches!(ty.kind(), ty::Adt(adt, _) if adt.is_struct() && adt.non_enum_variant().fields.is_empty())
    }

    /// Two sequences, item by item, then by length: `$cmpItems(a, b, $cmp)`.
    fn cmp_items(&mut self, a: Expr, b: Expr, item: Ty<'tcx>, partial: bool, span: Span) -> R<Expr> {
        let compare = self.cmp_fn(item, partial, span)?;
        Ok(Expr::call(Expr::helper("$cmpItems"), vec![a, b, compare]))
    }

    /// `binary_search(&x)` of a slice `items` of `item`s: `$binarySearch`
    /// where `<` orders them as `Ord` does, `is_primitive_ord`'s; any other
    /// by its `Ord`, as `sort()` orders it, strings and `char`s by code
    /// point (ADR 0183), a derived one by its variants and fields.
    pub(in crate::lower) fn binary_searched(
        &mut self,
        items: Expr,
        x: Expr,
        item: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        if self.is_primitive_ord(item) {
            return Ok(Expr::call_helper("$binarySearch", vec![items, x]));
        }
        let x = match x.reads_same() {
            true => x,
            false => self.spill("item", x, out),
        };
        let mut body = Vec::new();
        let order = self.cmp_value(Expr::var("each"), x, item, false, span, &mut body)?;
        body.push(StmtKind::Return(Some(order)).at(js::Span::NONE));
        Ok(Expr::call_helper(
            "$binarySearchBy",
            vec![items, Expr::arrow(vec!["each".into()], body)],
        ))
    }

    /// `is_sorted()` of `items`, a slice or an iterator of `item`s, each two
    /// in a row by their `PartialOrd`: `$isSortedBy(items, (a, b) => a <=
    /// b)` of numbers, JS's `<=` being `false` of a NaN as `partial_cmp`'s
    /// is; else their `partial_cmp` at most `Equal`, which `None` isn't.
    /// Given `drop`, of items with a destructor, `$isSortedBy` drops them
    /// where std's `is_sorted_by` does (#776).
    pub(in crate::lower) fn is_sorted_call(
        &mut self,
        items: Expr,
        item: Ty<'tcx>,
        drop: Option<Expr>,
        span: Span,
    ) -> R<Expr> {
        let item = item.peel_refs();
        let (a, b) = (Expr::var("a"), Expr::var("b"));
        let le = match super::representation::Num::of(item) {
            Some(_) => Expr::bin(Op::Le, a, b),
            None => {
                let cmp = self.cmp_fn(item, true, span)?;
                Expr::bin(Op::Le, Expr::call(cmp, vec![a, b]), Expr::int(0))
            }
        };
        let in_order = Expr::arrow(
            vec!["a".into(), "b".into()],
            vec![StmtKind::Return(Some(le)).at(js::Span::NONE)],
        );
        let mut args = vec![items, in_order];
        if let Some(drop) = drop {
            args.extend([Expr::undefined(), drop]);
        }
        Ok(Expr::call_helper("$isSortedBy", args))
    }

    /// `(a, b) => <their Ordering>`, or the function itself: `$cmp`,
    /// `wordOrd_cmp`, or a dictionary's, `TOrd.cmp`, which needs no `this`.
    pub(super) fn cmp_fn(&mut self, ty: Ty<'tcx>, partial: bool, span: Span) -> R<Expr> {
        let mut body = Vec::new();
        let order = self.cmp_value(Expr::var("a"), Expr::var("b"), ty, partial, span, &mut body)?;
        if body.is_empty()
            && let js::ExprKind::Call(callee, args) = &order.kind
            && callee.reads_same()
            && matches!(args.as_slice(), [x, y] if is_var(x, "a") && is_var(y, "b"))
        {
            return Ok((**callee).clone());
        }
        body.push(StmtKind::Return(Some(order)).at(js::Span::NONE));
        Ok(Expr::arrow(vec!["a".into(), "b".into()], body))
    }

    /// `a < b` and the rest, `a.cmp(&b)`, `a.partial_cmp(&b)`, `a.max(b)`
    /// and `a.min(b)`. `None` for numbers, which std's operators already are.
    pub(super) fn ordering_call(
        &mut self,
        id: DefId,
        tr: ty::TraitRef<'tcx>,
        values: Vec<Expr>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let ty = tr.self_ty();
        // Of a type and another, the crate's own `PartialOrd<u32>`: not an
        // order of the type with itself.
        let other = tr
            .args
            .types()
            .nth(1)
            .is_some_and(|rhs| rhs.peel_refs() != ty.peel_refs());
        let Some((call, partial)) = self.recognition().ordering_call(id, tr) else {
            return Ok(None);
        };
        let operator = match call {
            OrderingCall::Lt => Some(Op::Lt),
            OrderingCall::Le => Some(Op::Le),
            OrderingCall::Gt => Some(Op::Gt),
            OrderingCall::Ge => Some(Op::Ge),
            _ => None,
        };
        // Of a type parameter, its `Ord` dictionary's `max`, `min` or `clamp`,
        // its impl's own or std's, `$ord`'s, given its drop where it has a
        // destructor, by which std's drops what it doesn't give back (#962).
        if matches!(call, OrderingCall::Max | OrderingCall::Min | OrderingCall::Clamp) && self.is_unknown(ty) {
            let dictionary = self.dictionary(tr, span)?;
            let name = match call {
                OrderingCall::Max => "max",
                OrderingCall::Min => "min",
                _ => "clamp",
            };
            let mut args = values;
            args.extend(self.std_drop_function(ty, span)?);
            return Ok(Some(Expr::call(Expr::member(dictionary, name), args)));
        }
        // `clamp`, by its `Ord`, which panics of bounds the wrong way round
        // as `Ord`'s default does, a `char`'s showing them; of a type with a
        // destructor, its drop, which drops the two it doesn't give back,
        // and all three as a comparison panics, as std's `clamp` drops its
        // parameters (#912).
        if matches!(call, OrderingCall::Clamp) {
            // std's `min <= max`, `self < min` and `self > max`, its
            // `PartialOrd` dictionary's, an impl's own included (#937).
            let compare = self.operators(ty, span)?;
            let mut args = values;
            args.push(compare);
            let show = ty.peel_refs().is_char().then(|| {
                let c = Expr::var("c");
                let shown = Expr::call_helper("$debugStr", vec![c, Expr::str("'")]);
                Expr::arrow(vec!["c".into()], vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)])
            });
            let drop = self.std_drop_function(ty, span)?;
            if show.is_some() || drop.is_some() {
                args.push(show.unwrap_or_else(Expr::undefined));
            }
            args.extend(drop);
            return Ok(Some(Expr::call_helper("$clampBy", args)));
        }
        // `max` and `min` of a type with a destructor, std's `Ord::max`'s
        // `other < self`, the one it doesn't give back dropped, and both as
        // the comparison panics, as std's drops its parameters (#912).
        if matches!(call, OrderingCall::Max | OrderingCall::Min)
            && let Some(drop) = self.std_drop_function(ty, span)?
        {
            let compare = self.operators(ty, span)?;
            let name = if matches!(call, OrderingCall::Max) {
                "$maxOf"
            } else {
                "$minOf"
            };
            return Ok(Some(Expr::call_helper(name, [values, vec![compare, drop]].concat())));
        }
        let [a, b]: [Expr; 2] = values
            .try_into()
            .map_err(|_| self.unsupported(span, "this comparison"))?;
        // Its `<` is std's default, its impl's `partial_cmp` against 0, which
        // `None`'s `undefined` makes false; its `partial_cmp`, its impl's.
        if other {
            // Of references, std's `impl PartialOrd<&B> for &A`: its
            // referents' order, `A: PartialOrd<B>`'s, as chrono's `dt >=
            // &DateTime::<Utc>::MIN_UTC` of a `&DateTime<Tz>` (#467).
            let tr = match (ty.kind(), tr.args.types().nth(1).map(|rhs| *rhs.kind())) {
                (ty::Ref(_, a, _), Some(ty::Ref(_, b, _))) => ty::TraitRef::new(self.tcx, tr.def_id, [*a, b]),
                _ => tr,
            };
            let generic = matches!(tr.self_ty().kind(), ty::Param(_));
            let Some(op) = operator.filter(|_| generic || self.is_user_impl(tr)) else {
                return Ok(None);
            };
            // Of a type parameter, its caller's impl's operator, its
            // dictionary's (#944); of the crate's, its own where it writes
            // one, else its `partial_cmp` against 0.
            if generic {
                let dictionary = self.dictionary(tr, span)?;
                return Ok(Some(Expr::call(
                    Expr::member(dictionary, operator_name(op)),
                    vec![a, b],
                )));
            }
            let imp = self.recognition().user_impl(tr);
            if let Some(imp) = imp
                && writes_operator(self.tcx, imp, operator_name(op))
            {
                let method = trait_method(self.tcx, tr.def_id, operator_name(op));
                return Ok(Some(self.impl_call(method, tr.args, vec![a, b], span)?));
            }
            let partial_cmp = trait_method(self.tcx, tr.def_id, "partial_cmp");
            let order = self.impl_call(partial_cmp, tr.args, vec![a, b], span)?;
            return Ok(Some(Expr::bin(op, order, Expr::int(0))));
        }
        // `max` and `min` return one of them, so each is read twice.
        let (a, b) = if matches!(call, OrderingCall::Max | OrderingCall::Min) {
            let a = if a.reads_same() { a } else { self.spill("left", a, out) };
            let b = if b.reads_same() { b } else { self.spill("right", b, out) };
            (a, b)
        } else {
            (a, b)
        };
        // `partial_cmp`'s `undefined` makes every one of these false, as `None` does.
        // `max` and `min` ask std's `other < self`, `PartialOrd::lt` of the
        // second against the first, as a comparison that prints or counts
        // sees (#912, #937).
        Ok(Some(match call {
            _ if let Some(op) = operator => self.operator_value(op, a, b, ty, span, out)?,
            // The second when they're equal, as Rust's `max` does.
            OrderingCall::Max => {
                let less = self.operator_value(Op::Lt, b.clone(), a.clone(), ty, span, out)?;
                Expr::cond(less, a, b)
            }
            OrderingCall::Min => {
                let less = self.operator_value(Op::Lt, b.clone(), a.clone(), ty, span, out)?;
                Expr::cond(less, b, a)
            }
            _ => self.cmp_value(a, b, ty, partial, span, out)?,
        }))
    }
}

/// `PartialOrd`'s method of the operator `op`, `<`'s `lt`.
fn operator_name(op: Op) -> &'static str {
    match op {
        Op::Lt => "lt",
        Op::Le => "le",
        Op::Gt => "gt",
        _ => "ge",
    }
}

/// Whether `ty` is a value of any shape, `js::Unknown` (ADR 0612).
fn any_shape(tcx: rustc_middle::ty::TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.peel_refs().kind(), rustc_middle::ty::Adt(adt, _) if super::declarations::any_shape(tcx, adt.did()))
}

fn is_var(e: &Expr, name: &str) -> bool {
    matches!(&e.kind, js::ExprKind::Var(n) if n == name)
}

/// The types `order_total` is asking of, outermost first.
type Asking<'tcx> = std::cell::RefCell<Vec<Ty<'tcx>>>;

/// Whether comparing two `ty`s, by their `Ord` or `PartialOrd`, may do what
/// `may` says an impl's methods may: an impl of the crate's or a library's,
/// hand-written or derived, `may`'s; a derived one's as its fields' orders;
/// std's types' as what they hold's; a type parameter's, a `dyn`'s or an
/// opaque type's, anyone's. Std's order of numbers, text and `bool`s does
/// nothing. Of what may panic (`order_may_panic`) and of what can be seen
/// (`order_seen`, #925), and of the quiet functions as they're found.
pub(in crate::lower) fn order_may<'tcx>(
    recognition: &super::recognition::Recognition<'_, 'tcx>,
    ty: Ty<'tcx>,
    may: &dyn Fn(DefId) -> bool,
) -> bool {
    let tcx = recognition.tcx;
    let traits = [
        std_item(tcx, StdItem::Ord),
        tcx.require_lang_item(LangItem::PartialOrd, DUMMY_SP),
    ];
    order_may_in(recognition, ty, (&traits, may), &mut Vec::new())
}

/// `order_may`'s, of comparing two `ty`s by their `PartialEq`, `==`.
pub(in crate::lower) fn eq_may<'tcx>(
    recognition: &super::recognition::Recognition<'_, 'tcx>,
    ty: Ty<'tcx>,
    may: &dyn Fn(DefId) -> bool,
) -> bool {
    let traits = [recognition.tcx.require_lang_item(LangItem::PartialEq, DUMMY_SP)];
    order_may_in(recognition, ty, (&traits, may), &mut Vec::new())
}

fn order_may_in<'tcx>(
    recognition: &super::recognition::Recognition<'_, 'tcx>,
    ty: Ty<'tcx>,
    (traits, may): (&[DefId], &dyn Fn(DefId) -> bool),
    seen: &mut Vec<Ty<'tcx>>,
) -> bool {
    let tcx = recognition.tcx;
    let ty = ty.peel_refs();
    if seen.contains(&ty) {
        return false;
    }
    if let ty::Param(_) | ty::Dynamic(..) | ty::Alias(..) = ty.kind() {
        return true;
    }
    seen.push(ty);
    let impls: Vec<DefId> = (traits.iter().copied())
        .filter_map(|tr| recognition.user_impl(ty::TraitRef::new_from_args(tcx, tr, recognition.args_of(tr, ty))))
        .collect();
    let rust_adt = |id: DefId| recognition.foreign.compiled_by_rust_js(id);
    let result = match ty.kind() {
        _ if !impls.is_empty() => impls.iter().any(|&imp| may(imp)),
        // A derived order of the crate's type, as its fields' orders.
        ty::Adt(adt, args) if rust_adt(adt.did()) => {
            (adt.all_fields()).any(|f| order_may_in(recognition, recognition.field_ty(f, args), (traits, may), seen))
        }
        ty::Adt(_, args) => args
            .types()
            .any(|arg| order_may_in(recognition, arg, (traits, may), seen)),
        ty::Tuple(parts) => parts
            .iter()
            .any(|part| order_may_in(recognition, part, (traits, may), seen)),
        ty::Array(item, _) | ty::Slice(item) => order_may_in(recognition, *item, (traits, may), seen),
        _ => false,
    };
    seen.pop();
    result
}

/// What a `partial_cmp` of a type runs, as rustc's impls say (#937): one
/// rust-js never assumes from its `Ord`, which Rust doesn't tie to it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum PartialRuns {
    /// Its hand-written impl's `partial_cmp`.
    Own,
    /// `Some(Ord::cmp(self, other))`, a derive's beside a derived `Ord`:
    /// its `cmp`, whatever that is.
    Cmp,
    /// Its parts' `partial_cmp`s in turn: a derive's otherwise, or std's
    /// of what holds them.
    Parts,
}

/// Whether the impl `imp`'s `partial_cmp` is `Some(self.cmp(other))`,
/// `Some(Ord::cmp(self, other))` as rustc's derive writes it beside a
/// derived `Ord` (#937): its order its `cmp`'s, doing nothing else. Read
/// from its HIR, which MIR building leaves, of the crate's impls; a
/// library's own compile lists its own in its contract, which its
/// consumers read (`partial_cmp_runs_cmp`, #964).
pub(in crate::lower) fn partial_cmp_is_cmp(tcx: rustc_middle::ty::TyCtxt<'_>, imp: DefId) -> bool {
    use rustc_hir as hir;
    let partial_cmp = trait_method(
        tcx,
        tcx.require_lang_item(LangItem::PartialOrd, DUMMY_SP),
        "partial_cmp",
    );
    let Some(method) = (tcx.impl_item_implementor_ids(imp).get(&partial_cmp).copied()).and_then(DefId::as_local) else {
        return false;
    };
    let Some(body) = tcx.hir_maybe_body_owned_by(method) else {
        return false;
    };
    let params: Vec<hir::HirId> = (body.params.iter())
        .filter_map(|param| match param.pat.kind {
            hir::PatKind::Binding(_, id, ..) => Some(id),
            _ => None,
        })
        .collect();
    let [this, other] = params[..] else {
        return false;
    };
    let results = tcx.typeck(method);
    let ord_cmp = trait_method(tcx, std_item(tcx, StdItem::Ord), "cmp");
    // The expression a block ends with, through `&*x`.
    fn peel<'h>(mut e: &'h hir::Expr<'h>) -> &'h hir::Expr<'h> {
        loop {
            e = e.peel_drop_temps();
            match e.kind {
                hir::ExprKind::Block(block, _) if block.stmts.is_empty() && block.expr.is_some() => {
                    e = block.expr.expect("its value")
                }
                hir::ExprKind::AddrOf(_, _, of) | hir::ExprKind::Unary(hir::UnOp::Deref, of) => e = of,
                _ => return e,
            }
        }
    }
    let is_var = |e: &hir::Expr<'_>, var: hir::HirId| {
        matches!(peel(e).kind, hir::ExprKind::Path(ref path) if results.qpath_res(path, e.hir_id) == hir::def::Res::Local(var))
            || matches!(peel(e).kind, hir::ExprKind::Path(hir::QPath::Resolved(None, path)) if path.res == hir::def::Res::Local(var))
    };
    let hir::ExprKind::Call(some, [order]) = peel(body.value).kind else {
        return false;
    };
    let is_some = super::fn_def(results.expr_ty_adjusted(some))
        .is_some_and(|(ctor, _)| tcx.is_lang_item(tcx.parent(ctor), LangItem::OptionSome));
    let order = peel(order);
    let cmp_of_self = match order.kind {
        hir::ExprKind::Call(callee, [a, b]) => {
            super::fn_def(results.expr_ty_adjusted(callee)).is_some_and(|(f, _)| f == ord_cmp)
                && is_var(a, this)
                && is_var(b, other)
        }
        hir::ExprKind::MethodCall(_, receiver, [b], _) => {
            results.type_dependent_def_id(order.hir_id) == Some(ord_cmp) && is_var(receiver, this) && is_var(b, other)
        }
        _ => false,
    };
    is_some && cmp_of_self
}

/// Whether `id` is a comparison trait's provided method that its
/// dictionary holds, an impl's own or std's, for generic code and std's
/// helpers to call: `PartialOrd`'s `lt`, `le`, `gt` and `ge` (#944),
/// `PartialEq`'s `ne` and `Ord`'s `max`, `min` and `clamp` (#962).
pub(in crate::lower) fn held_by_comparison_dictionary(tcx: rustc_middle::ty::TyCtxt<'_>, id: DefId) -> bool {
    let Some(tr) = tcx.trait_of_assoc(id) else {
        return false;
    };
    let names: &[&str] = if tcx.is_lang_item(tr, LangItem::PartialOrd) {
        &OPERATORS
    } else if tcx.is_lang_item(tr, LangItem::PartialEq) {
        &["ne"]
    } else if tr == std_item(tcx, StdItem::Ord) {
        &["max", "min", "clamp"]
    } else {
        &[]
    };
    names.contains(&tcx.item_name(id).as_str())
}

/// The runtime's maker of a comparison trait's dictionary, which fills in
/// std's provided methods where its impl writes none, the one place each
/// default is: `$partialEq`'s `ne`, `$partialOrd`'s `lt` and the rest, and
/// `$ord`'s `max`, `min` and `clamp` (#944, #962). None of another trait.
pub(in crate::lower) fn comparison_maker(tcx: rustc_middle::ty::TyCtxt<'_>, tr: DefId) -> Option<&'static str> {
    if tcx.is_lang_item(tr, LangItem::PartialEq) {
        Some("$partialEq")
    } else if tcx.is_lang_item(tr, LangItem::PartialOrd) {
        Some("$partialOrd")
    } else if tr == std_item(tcx, StdItem::Ord) {
        Some("$ord")
    } else {
        None
    }
}

/// `PartialOrd`'s operators' methods, by name.
const OPERATORS: [&str; 4] = ["lt", "le", "gt", "ge"];

/// Whether the impl `imp`, a `PartialOrd`'s, writes its own `<`, `<=`, `>`
/// or `>=`, which std's comparisons then call, not its `partial_cmp`.
fn writes_own_operators(tcx: rustc_middle::ty::TyCtxt<'_>, imp: DefId) -> bool {
    OPERATORS.into_iter().any(|name| writes_operator(tcx, imp, name))
}

/// Whether the impl `imp`, a `PartialOrd`'s, writes its own method `name`,
/// `lt`, rather than keeping std's.
fn writes_operator(tcx: rustc_middle::ty::TyCtxt<'_>, imp: DefId, name: &str) -> bool {
    let method = trait_method(tcx, tcx.require_lang_item(LangItem::PartialOrd, DUMMY_SP), name);
    tcx.impl_item_implementor_ids(imp).contains_key(&method)
}

/// Whether each method the impl `imp` writes is quiet (#925).
pub(in crate::lower) fn impl_quiet(tcx: rustc_middle::ty::TyCtxt<'_>, quiet: &IndexSet<DefId>, imp: DefId) -> bool {
    tcx.associated_item_def_ids(imp)
        .iter()
        .all(|&method| !matches!(tcx.def_kind(method), rustc_hir::def::DefKind::AssocFn) || quiet.contains(&method))
}
