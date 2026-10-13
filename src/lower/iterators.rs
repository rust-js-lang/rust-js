//! Iterators: a chain of std's adapters on an array or a JS iterator, and
//! what ends it (ADRs 0036, 0055, 0128), lazy where Rust's order can be seen
//! (ADR 0139), and what a chain that owns its items drops (ADR 0098).

use super::calls::Call;
use super::combinators::IterComb;
use super::combinators::IterSource;
use super::fn_def;
use super::recognition::{StdItem, std_item, trait_method};
use super::representation::Num;
use super::{FnCx, R, Std};
use crate::js;
use crate::js::{Expr, Op, Stmt, StmtKind};
use crate::runtime::Helper;

use rustc_middle::thir::{ExprId, ExprKind, LocalVarId};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;
use std::collections::HashSet;

/// What a function's iterator chains are, beyond their types, each stage by
/// its body's THIR and its expression.
#[derive(Default)]
pub(super) struct Chains {
    /// The stages that are JS iterators, though their types aren't lazy
    /// (ADR 0139), and the receivers that start one.
    pub(super) lazy: HashSet<(usize, ExprId)>,
    /// The variables that hold a lazy chain, which every use iterates.
    pub(super) locals: HashSet<LocalVarId>,
}

/// Whether `known` is an adapter, which makes an iterator of an iterator,
/// rather than a consumer, which ends one.
fn is_adapter(known: Std) -> bool {
    matches!(
        known,
        Std::ArrayMethod("map" | "filter")
            | Std::Enumerate
            | Std::Rev
            | Std::Skip
            | Std::Take
            | Std::Cloned
            | Std::Fuse
            | Std::IterComb(
                IterComb::FilterMap
                    | IterComb::Scan
                    | IterComb::FlatMap
                    | IterComb::Flatten
                    | IterComb::Zip
                    | IterComb::Chain
                    | IterComb::TakeWhile
                    | IterComb::SkipWhile
                    | IterComb::StepBy
                    | IterComb::Inspect
            )
    )
}

/// The value `(p) => p === value` or `(p) => value === p` looks for, if
/// it doesn't depend on `p` and reads the same however often it's read.
fn sought(test: &Expr) -> Option<Expr> {
    let js::ExprKind::Arrow(params, body) = &test.kind else {
        return None;
    };
    let (
        [js::Pattern::Name(param)],
        [
            Stmt {
                kind: StmtKind::Return(Some(e)),
                ..
            },
        ],
    ) = (params.as_slice(), body.as_slice())
    else {
        return None;
    };
    let js::ExprKind::Binary(Op::Eq, a, b) = &e.kind else {
        return None;
    };
    let is_param = |e: &Expr| matches!(&e.kind, js::ExprKind::Var(name) if name == param);
    let value = match (is_param(a), is_param(b)) {
        (true, false) => b,
        (false, true) => a,
        _ => return None,
    };
    (value.reads_same() && !value.mentions_var(param)).then(|| (**value).clone())
}

/// The locals stepped through, each a `$iter` that knows where it is (ADR
/// 0071): those `next()` is called on, or that are lent as a `&mut dyn
/// Iterator`, in the bodies being lowered, and those bound as one.
#[derive(Default)]
pub(super) struct Stepping {
    stepped: HashSet<LocalVarId>,
    bound: HashSet<LocalVarId>,
}

impl Stepping {
    pub(super) fn of(stepped: &HashSet<LocalVarId>) -> Stepping {
        Stepping {
            stepped: stepped.clone(),
            bound: HashSet::new(),
        }
    }
}

/// What a body inside another takes from the enclosing one's `Stepping`
/// while it's lowered, given back as it ends.
pub(super) struct EnclosingStepping {
    stepped: HashSet<LocalVarId>,
    bound: Option<HashSet<LocalVarId>>,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Start lowering a body inside the one being lowered, whose own stepped
    /// locals are `own`: a closure's, as well as the enclosing body's, which
    /// it sees; a copied default's, an item of its own, instead of them.
    pub(super) fn enter_body_stepping(&mut self, own: &HashSet<LocalVarId>, item: bool) -> EnclosingStepping {
        let stepped = self.stepping.stepped.clone();
        let bound = match item {
            true => {
                self.stepping.stepped = own.clone();
                Some(std::mem::take(&mut self.stepping.bound))
            }
            false => {
                self.stepping.stepped.extend(own);
                None
            }
        };
        EnclosingStepping { stepped, bound }
    }

    pub(super) fn leave_body_stepping(&mut self, enclosing: EnclosingStepping) {
        self.stepping.stepped = enclosing.stepped;
        if let Some(bound) = enclosing.bound {
            self.stepping.bound = bound;
        }
    }

    /// `Iterator.from(value)`: a JS iterator of an array or of one, which
    /// steps either (ADR 0061).
    pub(super) fn js_iterator(&self, value: Expr) -> Expr {
        Expr::call(Expr::member(Expr::var("Iterator"), "from"), vec![value])
    }

    /// `items`, each a `Result` or an `Option` (of `item`), collected into
    /// `target`, a `Result` or an `Option` of an array: the first `Err` or
    /// `None`, which ends it, or `Ok` of all the values, or the values.
    pub(in crate::lower) fn collect_fallible(
        &mut self,
        items: Expr,
        target: Ty<'tcx>,
        item: Option<Ty<'tcx>>,
        span: Span,
    ) -> R<Expr> {
        let ty::Adt(_, target_args) = target.kind() else {
            unreachable!("a `Result` or an `Option`")
        };
        let collection = target_args.type_at(0);
        if !self.is_vec_like(collection) && !collection.boxed_ty().is_some_and(|t| t.is_slice()) {
            return Err(self.unsupported(span, &format!("collecting into a `{target}`")));
        }
        Ok(if self.option_of(target).is_some() {
            let mut list = vec![items];
            if item.is_some_and(|item| self.option_of(item).is_some_and(|inner| self.boxed_payload(inner))) {
                self.runtime.insert(Helper::SomeValue);
                list.push(Expr::bool(true));
            }
            self.runtime.insert(Helper::CollectOptions);
            Expr::call(Expr::var("$collectOptions"), list)
        } else {
            self.runtime.insert(Helper::CollectResults);
            Expr::call(Expr::var("$collectResults"), vec![items])
        })
    }

    /// `$lent(it)`: a generic iterator lent as a `&mut`, which steps `it` and
    /// can't close it, as its lender goes on stepping it (ADR 0071).
    pub(super) fn lent_iterator(&mut self, it: Expr) -> Expr {
        self.runtime.insert(Helper::Lent);
        Expr::call(Expr::var("$lent"), vec![it])
    }

    /// `$iter(items)`: a JS iterator of an array that knows where it is
    /// (ADR 0071).
    pub(super) fn stepped_items(&mut self, items: Expr) -> Expr {
        self.runtime.insert(Helper::Iter);
        Expr::call(Expr::var("$iter"), vec![items])
    }

    /// What's left of `it`, a `$iter` of std's iterator over an array, of
    /// type `ty`: `it.items.slice(it.at, it.end)`, copies of the items it
    /// `owns` that need one (ADR 0181).
    pub(super) fn stepped_left(&mut self, it: Expr, ty: ty::Ty<'tcx>, owns: bool, span: Span) -> R<Expr> {
        let left = Expr::call(
            Expr::member(Expr::member(it.clone(), "items"), "slice"),
            vec![Expr::member(it.clone(), "at"), Expr::member(it, "end")],
        );
        let item = self.iterator_item(ty).expect("an iterator's item");
        Ok(match owns && self.needs_clone(item) {
            true => {
                let clone = self.clone_fn("item", item, span)?;
                Expr::call(Expr::member(left, "map"), vec![clone])
            }
            false => left,
        })
    }

    /// An iterator that's a JS iterator, not an array (ADR 0055): one of the
    /// crate's own, or std's adapters on one.
    fn is_lazy_iter(&self, ty: ty::Ty<'tcx>) -> bool {
        self.recognition().is_lazy_iter(ty)
    }

    /// Is a `ty` given for `callee`'s type parameter `input` one whose JS
    /// value isn't an iterator, where one goes: one of the crate's own, or a
    /// range, an object (ADR 0129), where `input` is an `Iterator` or an
    /// `IntoIterator`. A range is itself anywhere else.
    pub(super) fn given_as_iterator(&self, callee: DefId, input: ty::Ty<'tcx>, ty: ty::Ty<'tcx>) -> bool {
        let ty = self.reveal(ty);
        if self.is_user_iterator(ty) {
            return true;
        }
        if self.range_kind(ty.peel_refs()).is_none() {
            return false;
        }
        let env = ty::TypingEnv::post_analysis(self.tcx, callee);
        [StdItem::Iterator, StdItem::IntoIterator].into_iter().any(|item| {
            let tr = ty::TraitRef::new(self.tcx, std_item(self.tcx, item), [input]);
            matches!(
                self.tcx.codegen_select_candidate(env.as_query_input(tr)),
                Ok(rustc_middle::traits::ImplSource::Param(_))
            )
        })
    }

    /// An iterator of the crate's own as a JS one, `$iterator(it,
    /// countdownIterator_next)`, and a range as its items (ADR 0129).
    /// Anything else is `value` itself.
    pub(super) fn iter_source(&mut self, value: Expr, ty: ty::Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        let ty = self.reveal(ty);
        if self.range_kind(ty).is_some() {
            return self.range_items(value, ty, span, out);
        }
        // A `&mut` one is stepped through as it's taken from.
        if let ty::Ref(_, inner, _) = ty.kind()
            && self.range_kind(*inner).is_some()
        {
            return Err(self.unsupported(span, &format!("iterating over a `{ty}`, which steps it")));
        }
        // A generic one is an array or a JS iterator: `Iterator.from` takes
        // either (ADR 0061).
        if self.is_generic_iter(ty) {
            return Ok(self.js_iterator(value));
        }
        if !self.is_user_iterator(ty) {
            return Ok(value);
        }
        let iterator = std_item(self.tcx, StdItem::Iterator);
        self.user_iterator(value, ty, iterator, "next", span)
    }

    /// `DoubleEndedIterator`, if `ty` is an iterator of the crate's whose
    /// `next_back` is its own: `rev()` of it is a JS iterator of that,
    /// `$iterator(it, spanDoubleEndedIterator_next_back)`, stepped from the
    /// back as Rust's is (ADR 0164).
    pub(super) fn own_next_back(&self, ty: ty::Ty<'tcx>) -> R<Option<DefId>> {
        if !self.is_user_iterator(ty) {
            return Ok(None);
        }
        let Some(double_ended) = self.recognition().double_ended_iterator() else {
            return Ok(None);
        };
        let next_back = trait_method(self.tcx, double_ended, "next_back");
        let args = self.args_of(double_ended, ty.peel_refs());
        let own = self
            .resolve_instance(next_back, args)?
            .is_some_and(|i| self.is_rust_fn(i.def_id()));
        Ok(own.then_some(double_ended))
    }

    /// `exact_len`'s, of the iterator's value and its type `ty`.
    pub(in crate::lower) fn exact_len_of(
        &mut self,
        value: Expr,
        ty: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let iterator = std_item(self.tcx, StdItem::Iterator);
        let size_hint = trait_method(self.tcx, iterator, "size_hint");
        let args = self.args_of(iterator, ty);
        let hint = match self.trait_call(size_hint, args, vec![value.clone()], span, out)? {
            Some(hint) => hint,
            None => {
                if value.has_effects() {
                    out.push(StmtKind::Expr(value).at(self.js_span(span)));
                }
                Expr::array(vec![Expr::int(0), Expr::undefined()])
            }
        };
        self.runtime.insert(Helper::ExactLen);
        Ok(Expr::call(Expr::var("$exactLen"), vec![hint]))
    }

    /// An iterator of the crate's as a JS one, stepped by its trait's
    /// `method`: `Iterator`'s `next`, or `DoubleEndedIterator`'s `next_back`.
    pub(super) fn user_iterator(
        &mut self,
        value: Expr,
        ty: ty::Ty<'tcx>,
        trait_id: DefId,
        method: &str,
        span: Span,
    ) -> R<Expr> {
        let next = trait_method(self.tcx, trait_id, method);
        let args = self.args_of(trait_id, ty.peel_refs());
        // A generic `next` boxes a `Some` that looks like `None` (ADR 0051).
        let boxed = self.resolve_instance(next, args)?.is_some_and(|instance| {
            let id = instance.def_id();
            let output = self
                .tcx
                .fn_sig(id)
                .instantiate_identity()
                .skip_normalization()
                .skip_binder()
                .output();
            let output = self
                .tcx
                .try_normalize_erasing_regions(
                    ty::TypingEnv::post_analysis(self.tcx, id),
                    ty::Unnormalized::new_wip(output),
                )
                .unwrap_or(output);
            self.option_of(output).is_some_and(|item| self.boxed_payload(item))
        });
        let call = self.impl_call(next, args, vec![Expr::var("iterator")], span)?;
        // `(iterator) => f(iterator)` is `f`.
        let next = match &call.kind {
            js::ExprKind::Call(callee, list) if matches!(list.as_slice(), [only] if matches!(&only.kind, js::ExprKind::Var(n) if n == "iterator")) => {
                (**callee).clone()
            }
            _ => Expr::arrow(
                vec!["iterator".into()],
                vec![StmtKind::Return(Some(call)).at(js::Span::NONE)],
            ),
        };
        self.runtime.insert(Helper::Iterator);
        let mut list = vec![value, next];
        if boxed {
            self.runtime.insert(Helper::SomeValue);
            list.push(Expr::bool(true));
        }
        Ok(Expr::call(Expr::var("$iterator"), list))
    }

    /// What an iterator of type `iterator` yields: its `Item`.
    pub(super) fn iterator_item(&self, iterator: ty::Ty<'tcx>) -> Option<ty::Ty<'tcx>> {
        let trait_id = std_item(self.tcx, StdItem::Iterator);
        let item = trait_method(self.tcx, trait_id, "Item");
        let projection = ty::Ty::new_projection(self.tcx, ty::IsRigid::No, item, [iterator]);
        self.tcx
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(projection))
            .ok()
    }

    /// A chain's stage, by its body and its expression: what a consumer
    /// that takes it by `&mut`, as `find` does, borrows.
    pub(super) fn chain_key(&self, e: ExprId) -> (usize, ExprId) {
        (std::ptr::from_ref(self.thir) as usize, self.chain_stage(e))
    }

    fn chain_stage(&self, e: ExprId) -> ExprId {
        match self.thir[self.strip(e)].kind {
            ExprKind::Borrow { arg, .. } => self.strip(arg),
            _ => self.strip(e),
        }
    }

    /// Whether `e`'s value is a JS iterator: a lazy type's, or a stage a
    /// chain's consumer made lazy.
    pub(super) fn is_lazy_value(&self, e: ExprId) -> bool {
        self.is_lazy_iter(self.thir[e].ty)
            || self.chains.lazy.contains(&self.chain_key(e))
            || matches!(self.thir[self.chain_stage(e)].kind, ExprKind::VarRef { id } if self.chains.locals.contains(&id))
            || self.through_by_ref(e)
    }

    /// Whether `e` is `it.by_ref()`, or a chain on it: each stage takes from
    /// `it` only what it's asked for, as Rust's does, so `take(2)` leaves the
    /// rest in `it` (ADR 0071).
    fn through_by_ref(&self, e: ExprId) -> bool {
        let mut at = self.strip(e);
        loop {
            // `&mut *it.by_ref()`, as `take` takes it.
            while let ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } = self.thir[at].kind {
                at = self.strip(arg);
            }
            let ExprKind::Call { fun, ref args, .. } = self.thir[at].kind else {
                return false;
            };
            match (self.std_fn(fun), args.first()) {
                (Some(Std::IterByRef), _) => return true,
                (Some(known), Some(&inner)) if is_adapter(known) => at = self.strip(inner),
                _ => return false,
            }
        }
    }

    /// `test`, a stage's predicate, as one that drops the `item` it
    /// discards: one `filter` doesn't keep, or one `skip_while` skips.
    pub(in crate::lower) fn dropping_discarded(
        &mut self,
        test: Expr,
        item: ty::Ty<'tcx>,
        discard_when: bool,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let test = if matches!(test.kind, js::ExprKind::Var(_)) {
            test
        } else {
            self.spill(if discard_when { "skip" } else { "keep" }, test, out)
        };
        let js_span = self.js_span(span);
        let mut drop = Vec::new();
        self.drop_value(Expr::var("item"), item, span, &mut drop)?;
        let asked = Expr::call(test, vec![Expr::var("item")]);
        let answer = |b: bool| StmtKind::Return(Some(Expr::bool(b))).at(js_span);
        let body = if discard_when {
            drop.push(answer(true));
            vec![StmtKind::If(asked, drop, None).at(js_span), answer(false)]
        } else {
            let mut rest = vec![StmtKind::If(asked, vec![answer(true)], None).at(js_span)];
            rest.extend(drop);
            rest.push(answer(false));
            rest
        };
        Ok(Expr::arrow(vec!["item".into()], body))
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `max()` or `min()` of `items`, an array of `item`s: of what JS's `<`
    /// doesn't order, by its `cmp` (ADR 0057).
    pub(in crate::lower) fn extreme_of(
        &mut self,
        max: bool,
        items: Expr,
        item: Option<ty::Ty<'tcx>>,
        span: Span,
    ) -> R<Expr> {
        Ok(match item {
            Some(item) if !self.is_primitive_ord(item) => {
                let compare = self.cmp_fn(item, false, span)?;
                self.runtime.insert(if max { Helper::MaxBy } else { Helper::MinBy });
                let mut list = vec![items, compare];
                if self.boxed_payload(item) {
                    self.runtime.insert(Helper::Some);
                    list.push(Expr::bool(true));
                }
                Expr::call(Expr::var(if max { "$maxBy" } else { "$minBy" }), list)
            }
            _ => {
                self.runtime.insert(if max { Helper::Max } else { Helper::Min });
                Expr::call(Expr::var(if max { "$max" } else { "$min" }), vec![items])
            }
        })
    }

    /// `last()` of `items`, an array of `item`s: one that looks like `None`
    /// boxed (ADR 0051).
    pub(in crate::lower) fn last_of(&mut self, items: Expr, item: Option<ty::Ty<'tcx>>, out: &mut Vec<Stmt>) -> Expr {
        match item {
            Some(item) if self.boxed_payload(item) => {
                let items = if items.reads_same() {
                    items
                } else {
                    self.spill("items", items, out)
                };
                let last = Expr::bin(Op::Sub, Expr::member(items.clone(), "length"), Expr::int(1));
                self.some_at(items, last)
            }
            _ => Expr::call(Expr::member(items, "at"), vec![Expr::int(-1)]),
        }
    }

    /// `v.sort()`, or `v.sort_by_key(key)`: in place (ADR 0036), of the items'
    /// value. JS's `sort()` compares as strings: right for `bool`s, and
    /// numbers need `a - b`, strings `$cmp`, by code point (ADR 0183).
    pub(in crate::lower) fn sort_values(
        &mut self,
        key: Option<Expr>,
        items: Expr,
        receiver_ty: ty::Ty<'tcx>,
        generic_args: ty::GenericArgsRef<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let method = |items: Expr, name: &str, list: Vec<Expr>| Expr::call(Expr::member(items, name), list);
        let (a, b) = (Expr::var("a"), Expr::var("b"));
        Ok(match key {
            // Sorting, in place (ADR 0036). JS's `sort()` compares as strings:
            // right for `bool`s, and numbers need `a - b`, strings `$cmp`, by
            // code point (ADR 0183).
            None => {
                let elem = match receiver_ty.peel_refs().kind() {
                    ty::Slice(t) | ty::Array(t, _) => *t,
                    _ => return Err(self.unsupported(span, "sorting this")),
                };
                // A comparator's answer is a number, so a BigInt's is its `cmp`.
                if Num::of(elem).is_some_and(|n| !n.big()) {
                    let js_span = self.js_span(span);
                    let f = Expr::arrow(
                        vec!["a".into(), "b".into()],
                        vec![StmtKind::Return(Some(Expr::bin(Op::Sub, a, b))).at(js_span)],
                    );
                    method(items, "sort", vec![f])
                } else if elem.is_bool() {
                    method(items, "sort", vec![])
                } else {
                    // By its `cmp`: JS's `sort` is stable too (ADR 0057).
                    method(items, "sort", vec![self.cmp_fn(elem, false, span)?])
                }
            }
            Some(key) => {
                let key = if matches!(key.kind, js::ExprKind::Var(_)) {
                    key
                } else {
                    self.spill("key", key, out)
                };
                let js_span = self.js_span(span);
                // The keys' `cmp`, which is `$cmp` for what JS orders.
                let key_ty = generic_args.types().nth(1).expect("`sort_by_key` names its key");
                let mut body = Vec::new();
                let compare = self.cmp_value(
                    Expr::call(key.clone(), vec![a]),
                    Expr::call(key, vec![b]),
                    key_ty,
                    false,
                    span,
                    &mut body,
                )?;
                // A key read more than once, like a tuple's, is a `const` first.
                body.push(StmtKind::Return(Some(compare)).at(js_span));
                let f = Expr::arrow(vec!["a".into(), "b".into()], body);
                method(items, "sort", vec![f])
            }
        })
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// One of std's iterator sources (ADR 0128): `None` if `known` is another.
    pub(super) fn iter_source_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
    ) -> R<Option<Expr>> {
        let Call {
            generic_args,
            tys,
            span,
            ..
        } = call;
        let mut arg = || values.next().expect("rustc checked the arguments");
        Ok(Some(match known {
            Std::IterSource(IterSource::Once) => Expr::array(vec![arg()]),
            Std::IterSource(IterSource::Empty) => Expr::array(vec![]),
            Std::IterSource(IterSource::Repeat) => {
                let value = arg();
                let item = generic_args.type_at(0);
                let mut list = vec![value];
                if self.needs_clone(item) {
                    list.push(self.clone_fn("value", item, span)?);
                }
                self.runtime.insert(Helper::Repeating);
                Expr::call(Expr::var("$repeating"), list)
            }
            Std::IterSource(IterSource::RepeatWith) => {
                self.runtime.insert(Helper::RepeatingWith);
                Expr::call(Expr::var("$repeatingWith"), vec![arg()])
            }
            // Their closures' `Option`s: a `Some` that looks like `None` is boxed (ADR 0051).
            Std::IterSource(source @ (IterSource::Successors | IterSource::FromFn)) => {
                let item = generic_args.type_at(0);
                let (helper, name) = match source {
                    IterSource::Successors => (Helper::Successors, "$successors"),
                    _ => (Helper::FromFn, "$fromFn"),
                };
                let boxed = self.boxed_payload(item);
                let mut list: Vec<Expr> = (0..tys.len()).map(|_| arg()).collect();
                if boxed {
                    list.push(Expr::bool(true));
                }
                self.runtime.insert(helper);
                Expr::call(Expr::var(name), list)
            }
            _ => return Ok(None),
        }))
    }

    /// Does closure `e` call a function of the crate, or a closure, a generic
    /// `impl Fn`'s too, whose JS takes the arguments Rust gives it alone? And,
    /// `dyn_fn`, a `dyn Fn`, a prop's, a JS caller's function as its type says.
    pub(super) fn calls_rust(&self, e: ExprId, dyn_fn: bool) -> bool {
        let ExprKind::Closure(ref closure) = self.thir[self.strip(e)].kind else {
            return false;
        };
        self.closure_calls_rust(self.krate.closures[&closure.closure_id], dyn_fn)
    }

    /// `calls_rust`, of a closure's body.
    pub(super) fn closure_calls_rust(&self, body: &super::Body<'tcx>, dyn_fn: bool) -> bool {
        let thir = &body.thir;
        let mut call = super::body_queries::strip(thir, body.expr);
        if let ExprKind::Block { block } = thir[call].kind
            && thir[block].stmts.is_empty()
            && let Some(value) = thir[block].expr
        {
            call = super::body_queries::strip(thir, value);
        }
        let ExprKind::Call { fun, ref args, .. } = thir[call].kind else {
            return false;
        };
        let Some((def_id, _)) = fn_def(thir[fun].ty) else {
            return false;
        };
        match self.tcx.trait_of_assoc(def_id) {
            Some(fn_trait) if self.tcx.fn_trait_kind_from_def_id(fn_trait).is_some() => {
                match thir[args[0]].ty.peel_refs().kind() {
                    ty::Closure(..) | ty::Param(_) => true,
                    ty::Dynamic(..) => dyn_fn,
                    ty::Adt(adt, args) if adt.is_box() => dyn_fn && matches!(args.type_at(0).kind(), ty::Dynamic(..)),
                    _ => false,
                }
            }
            _ => def_id.is_local() && !super::bindings::is_binding(self.tcx, def_id),
        }
    }

    /// Does `e` name one that knows where it is: a `Peekable`, or a local
    /// `next()` steps through?
    pub(super) fn is_stepping(&self, e: ExprId) -> bool {
        // `it`, `&mut it`, or `&mut *it` of a `&mut` to one.
        let mut e = self.strip(e);
        while let ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } = self.thir[e].kind {
            e = self.strip(arg);
        }
        let ty = self.thir[e].ty;
        self.is_peekable(ty)
            || matches!(self.thir[e].kind, ExprKind::VarRef { id } if self.stepping.bound.contains(&id))
    }
}

/// `enumerate()` then a method whose callback takes each pair apart,
/// `.map((x, i) => [i, x]).map(([i, w]) => ..)`: the method of the items
/// themselves, whose callback JS gives each one's index too, `.map((w, i)
/// => ..)` (ADR 0217). Not `filter` or `find`, which give back the pairs.
/// `Err` gives back the callback, of any other.
fn indexed_callback(items: &Expr, name: &str, f: Expr) -> Result<(Expr, Expr), Expr> {
    if !["map", "forEach", "some", "every", "flatMap"].contains(&name) {
        return Err(f);
    }
    let js::ExprKind::Call(callee, args) = &items.kind else {
        return Err(f);
    };
    let js::ExprKind::Member(source, method) = &callee.kind else {
        return Err(f);
    };
    if method != "map" || !matches!(&args[..], [pairing] if is_enumerate_pairing(pairing)) {
        return Err(f);
    }
    let js::ExprKind::Arrow(params, body) = &f.kind else {
        return Err(f);
    };
    let [js::Pattern::Array(parts)] = &params[..] else {
        return Err(f);
    };
    let [index, item] = &parts[..] else { return Err(f) };
    let index = match index {
        Some(js::Pattern::Name(index)) => Some(index.clone()),
        Some(_) => return Err(f),
        None => None,
    };
    // The item, taken apart as it was, `({ name }, i)`; `_` where only the
    // index is read: no Rust variable is.
    let mut params = vec![item.clone().unwrap_or_else(|| "_".into())];
    params.extend(index.map(js::Pattern::Name));
    Ok((
        (**source).clone(),
        Expr {
            span: f.span,
            ..Expr::arrow(params, body.clone())
        },
    ))
}

/// `g` of `(x, i) => g(x, i)`, an arrow that passes on what it's given, in
/// order, to a function it doesn't name a parameter.
pub(super) fn passed_on(f: &Expr) -> Option<Expr> {
    let js::ExprKind::Arrow(params, body) = &f.kind else {
        return None;
    };
    let [
        Stmt {
            kind: StmtKind::Return(Some(call)),
            ..
        },
    ] = &body[..]
    else {
        return None;
    };
    let js::ExprKind::Call(callee, args) = &call.kind else {
        return None;
    };
    let js::ExprKind::Var(g) = &callee.kind else {
        return None;
    };
    let names: Vec<&str> = params
        .iter()
        .filter_map(|p| match p {
            js::Pattern::Name(name) => Some(name.as_str()),
            _ => None,
        })
        .collect();
    let passed = names.len() == params.len()
        && args.len() == names.len()
        && !names.contains(&g.as_str())
        && args
            .iter()
            .zip(&names)
            .all(|(a, n)| matches!(&a.kind, js::ExprKind::Var(v) if v == n));
    passed.then(|| Expr {
        reads: false,
        kind: js::ExprKind::Var(g.clone()),
        span: f.span,
    })
}

/// `(x, i) => [i, x]`, which `enumerate()` maps each item with.
fn is_enumerate_pairing(e: &Expr) -> bool {
    let var = |e: &Expr, name: &str| matches!(&e.kind, js::ExprKind::Var(v) if v == name);
    let js::ExprKind::Arrow(params, body) = &e.kind else {
        return false;
    };
    let [js::Pattern::Name(x), js::Pattern::Name(i)] = &params[..] else {
        return false;
    };
    let [stmt] = &body[..] else { return false };
    let StmtKind::Return(Some(pair)) = &stmt.kind else {
        return false;
    };
    let js::ExprKind::Array(parts) = &pair.kind else {
        return false;
    };
    (x.as_str(), i.as_str()) == ("x", "i")
        && matches!(&parts[..], [first, second] if var(first, "i") && var(second, "x"))
}

/// Whether `items` is an array made here, which nothing else holds: an
/// adapter's, a copy's, a `split`'s, or a helper's.
fn fresh(items: &Expr) -> bool {
    match &items.kind {
        js::ExprKind::Array(_) => true,
        js::ExprKind::Call(callee, _) => match &callee.kind {
            js::ExprKind::Member(_, name) => [
                "map",
                "filter",
                "slice",
                "toReversed",
                "split",
                "from",
                "toArray",
                "flatMap",
                "flat",
                "concat",
            ]
            .contains(&name.as_str()),
            js::ExprKind::Var(name) => [
                "$range",
                "$bigRange",
                "$charRange",
                "$zip",
                "$takeWhile",
                "$skipWhile",
                "$windows",
                "$chunks",
                "$chunksExact",
                "$chunksExactMut",
                "$splitBy",
                "$lines",
                "$slice",
                "$rest",
                "$scan",
                "$drain",
                "$splitOff",
                "$setAlgebra",
                "$sortedEntries",
                "$sortedKeys",
            ]
            .contains(&name.as_str()),
            _ => false,
        },
        _ => false,
    }
}

/// Each chain in `stmts` that MIR made as Rust runs it (ADR 0364), item by
/// item, `$iter(xs).map(f).toArray()`, written as an array's, `xs.map(f)`
/// (ADR 0055), where the order its closures run in can't be told from
/// Rust's by what takes it. One that's lent, `$lent(…).some(f)`, made for
/// this call alone, no one else steps.
pub(super) fn eager_chains(stmts: &mut [Stmt]) {
    js::each_expr_mut(stmts, &mut |e| {
        // `nth(n)`'s, `chain.drop(n).take(1).toArray()[0]`: of an array
        // whose stages do nothing, `xs[n]`.
        if let js::ExprKind::Index(first, zero) = &e.kind
            && zero.as_int() == Some(0)
            && let Some((chain, n)) = nth_of(first)
            && let Some((items, _)) = eager_items(chain, false, None)
        {
            *e = Expr {
                span: e.span,
                ..Expr::index(items, n.clone())
            };
            return;
        }
        let js::ExprKind::Call(callee, args) = &e.kind else {
            return;
        };
        // `next()`'s, `$next(chain)`, of an iterator made here of an array
        // whose stages do nothing: its first item, `xs[0]`.
        // A split's too, `Iterator.from(path.split(c))`, an array.
        let split = |chain: &Expr| match &chain.kind {
            js::ExprKind::Call(_, of) if is_iterator_from(chain) => match &of[0].kind {
                js::ExprKind::Call(f, _)
                    if matches!(&f.kind, js::ExprKind::Var(v) if v == "$splitBy")
                        || matches!(&f.kind, js::ExprKind::Member(_, m) if m == "split") =>
                {
                    Some((of[0].clone(), 0))
                }
                _ => None,
            },
            _ => None,
        };
        if matches!(&callee.kind, js::ExprKind::Var(v) if v == "$next")
            && let [chain] = &args[..]
            && let Some((items, _)) = eager_items(chain, false, None).or_else(|| split(chain))
        {
            *e = Expr {
                span: e.span,
                ..Expr::index(items, Expr::int(0))
            };
            return;
        }
        // What reads every item alone, `$max`'s: of the array itself, not a
        // copy, `$max(xs)`; `$maxBy`'s compares as each comes.
        if matches!(&callee.kind, js::ExprKind::Var(v) if ["$max", "$min", "$maxBy", "$minBy"].contains(&v.as_str()))
            && let Some(first) = args.first()
            && let js::ExprKind::Call(to_array, none) = &first.kind
            && none.is_empty()
            && let js::ExprKind::Member(chain, name) = &to_array.kind
            && name == "toArray"
            && let Some((items, _)) = eager_items(chain, true, args.get(1))
        {
            let mut args = args.clone();
            args[0] = items;
            *e = Expr {
                span: e.span,
                ..Expr::call((**callee).clone(), args)
            };
            return;
        }
        // `$position`'s, of the array, lent or not: what it steps has no
        // `return` to close.
        if matches!(&callee.kind, js::ExprKind::Var(v) if v == "$position")
            && let [chain, f] = &args[..]
            && let Some((items, _)) = eager_items(chain, false, None)
        {
            *e = Expr {
                span: e.span,
                ..Expr::call((**callee).clone(), vec![items, f.clone()])
            };
            return;
        }
        let js::ExprKind::Member(chain, name) = &callee.kind else {
            return;
        };
        let eager = match (name.as_str(), &args[..]) {
            ("toArray", []) if let Some(parts) = chained(chain) => {
                Some(Expr::array(parts.into_iter().map(Expr::spread).collect()))
            }
            ("toArray", []) => match eager_items(chain, true, None) {
                // A new array: a stage's is one already; the source, copied,
                // unless it's one made here.
                Some((items, 0)) if !fresh(&items) => Some(Expr::call(Expr::member(items, "slice"), vec![])),
                Some((items, _)) => Some(items),
                None => None,
            },
            ("forEach", [f]) => {
                eager_items(chain, true, Some(f)).map(|(items, _)| array_method(items, "forEach", f.clone()))
            }
            // `count()`'s, `reduce((n) => n + 1, 0)`: the array's `length`.
            ("reduce", [f, init]) if init.as_int() == Some(0) && counts(f) => {
                eager_items(chain, true, None).map(|(items, _)| Expr::member(items, "length"))
            }
            ("reduce", [f, init]) => eager_items(chain, true, Some(f))
                .map(|(items, _)| Expr::call(Expr::member(items, "reduce"), vec![f.clone(), init.clone()])),
            // Whether any item is a literal, which `===` and `includes` agree
            // of whatever the items are: `includes`.
            ("some", [f])
                if let Some(value) = sought(f).filter(|v| {
                    matches!(&v.kind, js::ExprKind::Str(_) | js::ExprKind::Bool(_)) || v.as_int().is_some()
                }) =>
            {
                eager_items(chain, false, None)
                    .map(|(items, _)| Expr::call(Expr::member(items, "includes"), vec![value]))
            }
            ("some" | "every", [f]) => {
                eager_items(chain, false, None).map(|(items, _)| array_method(items, name, f.clone()))
            }
            ("find", [f]) => eager_items(chain, false, None)
                .map(|(items, _)| Expr::call(Expr::member(items, "find"), vec![f.clone()])),
            _ => None,
        };
        if let Some(eager) = eager {
            *e = Expr { span: e.span, ..eager };
        }
    });
}

/// A chain MIR makes lazily for the next statement's array alone to
/// spread, `const newLogs = $iter(xs).filter(f).map(g); const all =
/// [...a, ...newLogs];`, of stages that do nothing else: an array,
/// `xs.filter(f).map(g)`, which spreads alike.
pub(super) fn eager_spreads(stmts: &mut [Stmt]) {
    for i in 0..stmts.len().saturating_sub(1) {
        let (head, rest) = stmts.split_at_mut(i + 1);
        let StmtKind::Const(name, value) = &mut head[i].kind else {
            continue;
        };
        let spread = |e: &Expr| {
            matches!(&e.kind, js::ExprKind::Array(items)
                if items.iter().any(|item| matches!(&item.kind, js::ExprKind::Spread(of)
                    if matches!(&of.kind, js::ExprKind::Var(v) if v == name))))
        };
        let next = match &rest[0].kind {
            StmtKind::Const(_, e) | StmtKind::Let(_, Some(e)) | StmtKind::Return(Some(e)) => spread(e),
            _ => false,
        };
        if !next || js::mentions_in(rest, name) != 1 {
            continue;
        }
        if let Some((items, stages)) = eager_items(value, false, None)
            && stages > 0
        {
            *value = items;
        }
    }
}

/// A chain MIR makes as Rust runs it (ADR 0364), item by item, `$iter(xs)
/// .map(f).filter(g)`: an array made of `xs`, `xs.map(f).filter(g)`
/// (ADR 0055), and how many stages it has, where the order its closures run
/// in can't be told from Rust's. `whole`: what takes it runs over every
/// item, as `collect` does, and one closure may do something, `taker`, its
/// callback, which runs as each item comes, among them; else it may stop
/// early, and no stage may.
fn eager_items(chain: &Expr, whole: bool, taker: Option<&Expr>) -> Option<(Expr, usize)> {
    let mut stages = Vec::new();
    let mut at = unlent(chain);
    while let js::ExprKind::Call(callee, args) = &at.kind
        && let js::ExprKind::Member(source, name) = &callee.kind
        && matches!(name.as_str(), "map" | "filter" | "take" | "drop" | "flatMap")
        && let [f] = &args[..]
    {
        stages.push((name.as_str(), f));
        at = source;
    }
    // A count's, `take`'s or `drop`'s, the last first: what's before it,
    // which runs over the items it stops at or passes over only in Rust,
    // does nothing.
    let counted = |name: &str| matches!(name, "take" | "drop");
    if let Some(last) = stages.iter().position(|(name, _)| counted(name))
        && stages[last + 1..].iter().any(|(name, f)| !counted(name) && does(f))
    {
        return None;
    }
    let at = unlent(at);
    let js::ExprKind::Call(_, args) = &at.kind else {
        return None;
    };
    // Of an array, `$iter(xs)`; or two side by side, `$lazyZip` of each's,
    // `$zip(a, b)`, whose items pairing them make nothing else do.
    let source = match &args[..] {
        [source] if is_stepped_items(at) => source.clone(),
        [zipped]
            if is_iterator_from(at)
                && let js::ExprKind::Call(zip, sides) = &zipped.kind
                && matches!(&zip.kind, js::ExprKind::Var(f) if f == "$lazyZip")
                && let [a, b] = &sides[..]
                && is_stepped_items(a)
                && is_stepped_items(b) =>
        {
            Expr::call(Expr::var("$zip"), vec![iterated(a.clone()), iterated(b.clone())])
        }
        _ => return None,
    };
    // Each item's items an array too, all of them, in turn, as Rust's
    // `flat_map` steps each before the next: `xs.flatMap((x) => x.map(g))`.
    let mut flattened = Vec::new();
    for (name, f) in &stages {
        if *name == "flatMap" {
            let js::ExprKind::Arrow(params, body) = &f.kind else {
                return None;
            };
            let [
                Stmt {
                    kind: StmtKind::Return(Some(inner)),
                    span,
                },
            ] = body.as_slice()
            else {
                return None;
            };
            let items = match is_stepped_items(inner) {
                true => iterated(inner.clone()),
                false => eager_items(inner, true, None)?.0,
            };
            let body = vec![StmtKind::Return(Some(items)).at(*span)];
            flattened.push(Expr::arrow(params.clone(), body));
        }
    }
    let mut flattened = flattened.into_iter().rev();
    let doing = stages.iter().filter(|(name, f)| !counted(name) && does(f)).count();
    let quiet = match whole {
        true => doing + usize::from(taker.is_some_and(does)) <= 1,
        false => doing == 0,
    };
    if !quiet {
        return None;
    }
    let count = stages.len();
    let items = (stages.into_iter().rev()).fold(source, |items, (name, f)| match name {
        "take" => Expr::call(Expr::member(items, "slice"), vec![Expr::int(0), f.clone()]),
        "drop" => Expr::call(Expr::member(items, "slice"), vec![f.clone()]),
        "flatMap" => array_method(items, name, flattened.next().expect("made")),
        name => array_method(items, name, f.clone()),
    });
    Some((items, count))
}

/// Of `Iterator.from($lazyChain(a, b))`, a `chain` MIR makes: what each part
/// steps, `a` and `b`, which spread in turn as it steps them, `[...a, ...b]`;
/// of a part `Iterator.from(xs)`, or `cloned()`'s `xs.map((x) => x)`, `xs`.
fn chained(e: &Expr) -> Option<Vec<Expr>> {
    fn part(e: &Expr) -> Vec<Expr> {
        match &e.kind {
            js::ExprKind::Call(callee, args) => match (&callee.kind, args.as_slice()) {
                (js::ExprKind::Var(v), [a, b]) if v == "$lazyChain" => [part(a), part(b)].concat(),
                (js::ExprKind::Member(of, m), [f]) if m == "map" && f.is_identity() => part(of),
                _ if is_stepped_items(e) || is_iterator_from(e) => part(&args[0]),
                _ => vec![e.clone()],
            },
            _ => vec![e.clone()],
        }
    }
    let js::ExprKind::Call(_, args) = &e.kind else {
        return None;
    };
    let [chain] = args.as_slice() else { return None };
    let js::ExprKind::Call(callee, _) = &chain.kind else {
        return None;
    };
    (is_iterator_from(e) && matches!(&callee.kind, js::ExprKind::Var(v) if v == "$lazyChain")).then(|| part(chain))
}

/// Whether `e` is `Iterator.from(x)`.
fn is_iterator_from(e: &Expr) -> bool {
    matches!(&e.kind, js::ExprKind::Call(callee, args) if args.len() == 1
        && matches!(&callee.kind, js::ExprKind::Member(object, from)
            if from == "from" && matches!(&object.kind, js::ExprKind::Var(v) if v == "Iterator")))
}

/// What `$lent(e)` lends, `e`, made for this call alone, which no one else
/// steps; or `e` itself.
pub(super) fn unlent(e: &Expr) -> &Expr {
    match &e.kind {
        js::ExprKind::Call(lent, of) if matches!(&lent.kind, js::ExprKind::Var(v) if v == "$lent") && of.len() == 1 => {
            &of[0]
        }
        _ => e,
    }
}

/// Of `chain.drop(n).take(1).toArray()`, `nth(n)`'s first: `chain` and `n`.
fn nth_of(e: &Expr) -> Option<(&Expr, &Expr)> {
    fn call<'e>(e: &'e Expr, name: &str) -> Option<(&'e Expr, &'e [Expr])> {
        match &e.kind {
            js::ExprKind::Call(callee, args) => match &callee.kind {
                js::ExprKind::Member(of, m) if m == name => Some((&**of, args.as_slice())),
                _ => None,
            },
            _ => None,
        }
    }
    let (taken, []) = call(e, "toArray")? else { return None };
    let (dropped, [one]) = call(taken, "take")? else {
        return None;
    };
    let (chain, [n]) = call(dropped, "drop")? else {
        return None;
    };
    (one.as_int() == Some(1)).then_some((chain, n))
}

/// `items.name(f)`, of an array, `f` taking each enumerated pair apart
/// given each item and its index, as `indexed_callback` has it.
fn array_method(items: Expr, name: &str, f: Expr) -> Expr {
    let (items, f) = indexed_callback(&items, name, f).unwrap_or_else(|f| (items, f));
    Expr::call(Expr::member(items, name), vec![f])
}

/// Whether `f` is `count()`'s, `(n) => n + 1`.
fn counts(f: &Expr) -> bool {
    let js::ExprKind::Arrow(params, body) = &f.kind else {
        return false;
    };
    let [js::Pattern::Name(n)] = params.as_slice() else {
        return false;
    };
    matches!(body.as_slice(), [Stmt { kind: StmtKind::Return(Some(e)), .. }]
        if matches!(&e.kind, js::ExprKind::Binary(Op::Add, a, one)
            if matches!(&a.kind, js::ExprKind::Var(v) if v == n) && one.as_int() == Some(1)))
}

/// Whether calling `f` may do what can be seen, or throw: anything but an
/// arrow whose statements only make and give back what does nothing.
pub(super) fn does(f: &Expr) -> bool {
    // A `str`'s or a slice's methods that only read it, `s.startsWith(t)`,
    // as Rust's types tell (`Expr::read`), not a crate's own `at`, `P.at(p)`; and
    // the runtime's that only read, `$find(s, t)`, or compute, `$f64Max(a, b)`:
    // their operands'; and such a helper given as it is.
    const READS: &[&str] = &["startsWith", "endsWith", "includes", "toUpperCase", "toLowerCase", "at"];
    const HELPERS: &[&str] = &["$find", "$rfind", "$byteLen", "$f64Max", "$f64Min"];
    fn acts(e: &Expr) -> bool {
        let mut e = e.clone();
        e.each_mut(&mut |part| {
            if let js::ExprKind::Call(callee, args) = &part.kind {
                let reads = match &callee.kind {
                    js::ExprKind::Member(_, name) => part.reads && READS.contains(&name.as_str()),
                    js::ExprKind::Var(helper) => HELPERS.contains(&helper.as_str()),
                    _ => false,
                };
                if reads {
                    let mut operands = args.clone();
                    if let js::ExprKind::Member(of, _) = &callee.kind {
                        operands.push((**of).clone());
                    }
                    *part = Expr::array(operands);
                }
            }
        });
        e.has_effects()
    }
    fn body_does(stmts: &[Stmt]) -> bool {
        stmts.iter().any(|stmt| match &stmt.kind {
            StmtKind::Const(_, e) | StmtKind::Let(_, Some(e)) | StmtKind::Return(Some(e)) | StmtKind::Expr(e) => {
                acts(e)
            }
            StmtKind::Destructure { value, .. } => acts(value),
            StmtKind::Let(_, None) | StmtKind::Return(None) => false,
            StmtKind::If(test, then, otherwise) => {
                acts(test) || body_does(then) || otherwise.as_deref().is_some_and(body_does)
            }
            _ => true,
        })
    }
    match &f.kind {
        js::ExprKind::Arrow(_, body) => body_does(body),
        js::ExprKind::Var(helper) => !HELPERS.contains(&helper.as_str()),
        _ => true,
    }
}

/// Whether `e` is an array as it's made: `[a, b]`, `Array.from(x)`, a
/// string's `split` or a helper's, and through an array's methods that make one,
/// `.filter(f)`, not an iterator's of the same names, `$iter(xs).filter(f)`.
pub(super) fn made_array(e: &Expr) -> bool {
    match &e.kind {
        js::ExprKind::Array(_) => true,
        // A helper's that makes one, `$windows(v, 2)`.
        js::ExprKind::Call(callee, _) if matches!(callee.kind, js::ExprKind::Var(_)) => fresh(e),
        js::ExprKind::Call(callee, _) => match &callee.kind {
            js::ExprKind::Member(object, name) => match name.as_str() {
                "split" => true,
                "from" => matches!(&object.kind, js::ExprKind::Var(v) if v == "Array"),
                "map" | "filter" | "slice" | "toReversed" | "toSorted" | "concat" | "flat" | "flatMap" => {
                    made_array(object)
                }
                _ => false,
            },
            _ => false,
        },
        _ => false,
    }
}

/// Whether `e` is `$iter(items)`, `stepped_items`'s.
/// `$iter(v).map(f)`, a map of an array's items, made and not yet stepped:
/// nothing runs till it is.
pub(in crate::lower) fn is_items_map(e: &Expr) -> bool {
    if let js::ExprKind::Call(callee, args) = &e.kind
        && let [f] = args.as_slice()
        && let js::ExprKind::Member(source, method) = &callee.kind
        && let js::ExprKind::Call(iter, items) = &source.kind
    {
        method == "map"
            && f.reads_only_vars()
            && matches!(&iter.kind, js::ExprKind::Var(name) if name == "$iter")
            && items.iter().all(Expr::reads_only_vars)
    } else {
        false
    }
}

pub(super) fn is_stepped_items(e: &Expr) -> bool {
    matches!(&e.kind, js::ExprKind::Call(callee, _) if matches!(&callee.kind, js::ExprKind::Var(v) if v == "$iter"))
}

/// What `for (x of e)` steps through of `e`, `$iter(xs)`'s or
/// `Iterator.from(xs)`'s: `xs` itself.
pub(super) fn iterated(e: Expr) -> Expr {
    match e.kind {
        js::ExprKind::Call(callee, mut args)
            if args.len() == 1
                && match &callee.kind {
                    js::ExprKind::Var(f) => f == "$iter",
                    js::ExprKind::Member(object, from) => {
                        from == "from" && matches!(&object.kind, js::ExprKind::Var(v) if v == "Iterator")
                    }
                    _ => false,
                } =>
        {
            args.pop().expect("one")
        }
        kind => Expr {
            reads: false,
            kind,
            span: e.span,
        },
    }
}
