//! Iterators: a chain of std's adapters on an array or a JS iterator, and
//! what ends it (ADRs 0036, 0055, 0128), lazy where Rust's order can be seen
//! (ADR 0139), and what a chain that owns its items drops (ADR 0098).

use super::calls::Call;
use super::combinators::IterSource;
use super::fn_def;
use super::recognition::{StdItem, std_item, trait_method};
use super::representation::Num;
use super::std_types::range::RangeKind;
use super::{FnCx, R, Std};
use crate::js;
use crate::js::{Expr, Op, Stmt, StmtKind};

use rustc_middle::thir::{ExprId, ExprKind};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;

/// `$iterator(value, next)`: a JS iterator of `value`, a Rust iterator,
/// stepped by `next`, which gives its `Option`; of one whose `Some` may be
/// boxed (ADR 0051), `boxed`, unboxed; given `back`, its `next_back`, one
/// that knows its back, which `$nextBack` steps (ADR 0429). The one JS form
/// of it.
pub(in crate::lower) fn stepped(value: Expr, next: Expr, boxed: bool, back: Option<Expr>) -> Expr {
    stepped_ends(value, next, boxed, back, None)
}

/// `stepped`'s, given `len` too, its length, one that knows it, an
/// `ExactSizeIterator`'s (ADR 0456).
pub(in crate::lower) fn stepped_ends(
    value: Expr,
    next: Expr,
    boxed: bool,
    back: Option<Expr>,
    len: Option<Expr>,
) -> Expr {
    let mut list = vec![value, next];
    if boxed || back.is_some() || len.is_some() {
        list.push(Expr::bool(boxed));
    }
    if back.is_some() || len.is_some() {
        list.push(back.unwrap_or_else(Expr::undefined));
    }
    list.extend(len);
    Expr::call(Expr::helper("$iterator"), list)
}

/// Whether `id`'s type parameter `param`, bound by `Iterator`, is given as it
/// is (ADR 0411): one an impl's type holds, `I` of `Doubled<I>`, or the
/// function's signature has in anything but itself, `Wrap<I>` or a result,
/// whose value its caller may read as its own type, and so stepped by its
/// `Iterator` dictionary, `{ next }`, where it's stepped. One given only
/// itself, `i: I`, is a JS iterator (ADR 0061). The one answer, of a
/// callee's evidence and its caller's argument alike.
pub(in crate::lower) fn given_as_it_is<'tcx>(tcx: ty::TyCtxt<'tcx>, id: DefId, param: ty::ParamTy) -> bool {
    // Bound by `Iterator` itself, as `bounds` reads its clauses, which make
    // its evidence: not by a trait it's a supertrait of.
    let iterator = std_item(tcx, StdItem::Iterator);
    let bound = tcx
        .clauses_of(id)
        .instantiate_identity(tcx)
        .into_iter()
        .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
        .any(|p| p.def_id() == iterator && p.self_ty().skip_binder() == Ty::new_param(tcx, param.index, param.name));
    if !bound {
        return false;
    }
    // A value of it, not its `Item`'s, `Vec<I::Item>`: what's in a
    // projection isn't given back.
    let has = |ty: Ty<'tcx>| {
        let mut walk = ty.walk();
        while let Some(arg) = walk.next() {
            match arg.as_type().map(|t| *t.kind()) {
                Some(ty::Alias(..)) => walk.skip_current_subtree(),
                Some(ty::Param(p)) if p.index == param.index => return true,
                _ => {}
            }
        }
        false
    };
    let held = match tcx.def_kind(id) {
        rustc_hir::def::DefKind::Impl { .. } => Some(id),
        _ => tcx.impl_of_assoc(id),
    }
    .is_some_and(|imp| has(tcx.type_of(imp).instantiate_identity().skip_normalization()));
    let signature = matches!(
        tcx.def_kind(id),
        rustc_hir::def::DefKind::Fn | rustc_hir::def::DefKind::AssocFn
    ) && tcx.trait_of_assoc(id).is_none()
        && tcx.trait_impl_of_assoc(id).is_none()
        && {
            let sig = tcx.fn_sig(id).instantiate_identity().skip_binder();
            has(sig.output())
                || sig.inputs().iter().any(|&input| {
                    !matches!(input.peel_refs().kind(), ty::Param(p) if p.index == param.index) && has(input)
                })
        };
    held || signature
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

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `Iterator.from(value)`: a JS iterator of an array or of one, which
    /// steps either (ADR 0061).
    pub(super) fn js_iterator(&self, value: Expr) -> Expr {
        if is_lent(&value) {
            return value;
        }
        Expr::call(Expr::member(Expr::var("Iterator"), "from"), vec![value])
    }

    /// `items`, each a `Result` or an `Option` (of `item`), collected into
    /// `target`, a `Result` or an `Option` of a collection: the first `Err`
    /// or `None`, which ends it, or `Ok` of all the values, or the values,
    /// an array, made the collection where it's another, `into` it.
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
        let into = match self.is_vec_like(collection) || collection.boxed_ty().is_some_and(|t| t.is_slice()) {
            true => None,
            false => {
                let values = self.fresh("values");
                // What each `Ok` or `Some` holds.
                let value = item.and_then(|item| match item.kind() {
                    ty::Adt(_, args) => args.types().next(),
                    _ => None,
                });
                let made = self.collected_into(Expr::var(&values), collection, value, None, span)?;
                Some(match made.kind {
                    // A function that only takes them, a `from_iter`: the function.
                    js::ExprKind::Call(f, args) if matches!(args.as_slice(), [arg] if matches!(&arg.kind, js::ExprKind::Var(v) if *v == values)) => {
                        *f
                    }
                    kind => Expr::arrow(
                        vec![values.into()],
                        vec![StmtKind::Return(Some(Expr { kind, ..made })).at(js::Span::NONE)],
                    ),
                })
            }
        };
        Ok(if self.option_of(target).is_some() {
            let mut list = vec![items];
            let boxed = item.is_some_and(|item| self.option_of(item).is_some_and(|inner| self.boxed_payload(inner)));
            if boxed || into.is_some() {
                list.push(Expr::bool(boxed));
            }
            list.extend(into);
            Expr::call(Expr::helper("$collectOptions"), list)
        } else {
            let mut list = vec![items];
            list.extend(into);
            Expr::call(Expr::helper("$collectResults"), list)
        })
    }

    /// `$lent(it)`: a generic iterator lent as a `&mut`, which steps `it` and
    /// can't close it, as its lender goes on stepping it (ADR 0071); of one
    /// already lent, `by_ref()`'s, itself, a JS iterator nothing closes.
    pub(super) fn lent_iterator(&mut self, it: Expr) -> Expr {
        if is_lent(&it) {
            return it;
        }
        Expr::call(Expr::helper("$lent"), vec![it])
    }

    /// `$iter(items)`: a JS iterator of an array that knows where it is
    /// (ADR 0071).
    pub(super) fn stepped_items(&mut self, items: Expr) -> Expr {
        array_stepped(items)
    }

    /// `$iterator(value, next)`: a JS iterator of `value`, stepped by `next`,
    /// a range's by its `$rangeNext`.
    pub(super) fn stepped_by(&mut self, value: Expr, next: Expr) -> Expr {
        stepped(value, next, false, None)
    }

    /// What's left of `it`, a `$iter` of std's iterator over an array, of
    /// type `ty`: `it.items.slice(it.at, it.end)`, copies of the items it
    /// `owns` that need one (ADR 0181). Of one just made, `$iter(items)`,
    /// nothing's stepped: `items`, which Rust lends it or it owns.
    pub(super) fn stepped_left(&mut self, it: Expr, ty: ty::Ty<'tcx>, owns: bool, span: Span) -> R<Expr> {
        let left = match fresh_iter_source(&it) {
            Some(items) => items.clone(),
            None => Expr::call(
                Expr::member(Expr::member(it.clone(), "items"), "slice"),
                vec![Expr::member(it.clone(), "at"), Expr::member(it, "end")],
            ),
        };
        let item = self.iterator_item(ty).expect("an iterator's item");
        Ok(match owns && self.needs_clone(item) {
            true => {
                let clone = self.clone_fn("item", item, span)?;
                Expr::call(Expr::member(left, "map"), vec![clone])
            }
            false => left,
        })
    }

    /// Is a `ty` given for `callee`'s type parameter `input` one whose JS
    /// value isn't an iterator, where one goes: one of the crate's own, or a
    /// range, an object (ADR 0129), where `input` is an `Iterator` or an
    /// `IntoIterator`. A range is itself anywhere else.
    pub(super) fn given_as_iterator(&self, callee: DefId, input: ty::Ty<'tcx>, ty: ty::Ty<'tcx>) -> bool {
        // As it is, with its dictionary (ADR 0411).
        if let ty::Param(param) = input.kind()
            && given_as_it_is(self.tcx, callee, *param)
        {
            return false;
        }
        let ty = self.reveal(ty);
        if self.is_user_iterator(ty) {
            return true;
        }
        if self.range_kind(ty.peel_refs()).is_none() {
            return false;
        }
        // Bound so where the callee says, as `declared_bounds` reads it.
        let env = ty::TypingEnv::post_analysis(self.tcx, callee);
        (self.declared_bounds(input, env.param_env).into_iter())
            .any(|id| std_item(self.tcx, StdItem::Iterator) == id || std_item(self.tcx, StdItem::IntoIterator) == id)
    }

    /// The arguments of a call of the crate's `callee`, of types `given`:
    /// each iterator of the crate's, or range, where a generic iterator goes
    /// a JS iterator (ADR 0061), a function's and a trait impl's method's
    /// alike.
    pub(in crate::lower) fn given_iterators(
        &mut self,
        callee: DefId,
        values: &mut [Expr],
        given: &[ty::Ty<'tcx>],
        span: Span,
    ) -> R<()> {
        let inputs = self.tcx.fn_sig(callee).instantiate_identity().skip_binder().inputs();
        for ((value, &input), &given) in values.iter_mut().zip(inputs).zip(given) {
            let (input, given) = match *input.kind() {
                ty::Ref(_, inner, _) => (inner, given.peel_refs()),
                _ => (input, given),
            };
            // One that asks its length, a JS iterator that knows it, its back
            // too where it steps it from there (ADR 0456).
            if self.exact_size_param(callee, input) {
                let held = std::mem::replace(value, Expr::undefined());
                let back = self.double_ended_param(callee, input);
                *value = self.as_exact_size(held, given, back, span)?;
            // One that steps it from its back, a JS iterator that knows it
            // (ADR 0429).
            } else if self.double_ended_param(callee, input) {
                let held = std::mem::replace(value, Expr::undefined());
                *value = self.as_double_ended(held, given, span)?;
            } else if matches!(input.kind(), ty::Param(_)) && self.given_as_iterator(callee, input, given) {
                let held = std::mem::replace(value, Expr::undefined());
                *value = self.as_js_iterator(held, given, span)?;
            }
        }
        Ok(())
    }

    /// Whether what `callee` holds for its drop `index`, of the type `ty`
    /// given for it, is a JS iterator its caller made of an iterator of the
    /// crate's, `$iterator(value, next)`, whose drop drops `value`, its `of`
    /// (ADR 0421): its type parameter's, or that one's `IntoIter`, the
    /// iterator itself.
    pub(in crate::lower) fn iterated_slot(
        &self,
        callee: DefId,
        index: u32,
        args: ty::GenericArgsRef<'tcx>,
        ty: ty::Ty<'tcx>,
    ) -> bool {
        let slot = super::traits::drop_slot(self.tcx, self.krate.foreign, callee, index);
        let param = match *slot.kind() {
            ty::Param(param) => param,
            ty::Alias(_, alias) => match *alias.self_ty().kind() {
                ty::Param(param) => param,
                _ => return false,
            },
            _ => return false,
        };
        let given = args.type_at(param.index as usize);
        self.is_user_iterator(self.reveal(given))
            && self.reveal(ty) == self.reveal(given)
            && self.given_as_iterator(callee, ty::Ty::new_param(self.tcx, param.index, param.name), given)
    }

    /// An iterator of the crate's own as a JS one, `$iterator(it,
    /// countdownIterator_next)`. Anything else is `value` itself.
    pub(super) fn iter_source(&mut self, value: Expr, ty: ty::Ty<'tcx>, span: Span) -> R<Expr> {
        let ty = self.reveal(ty);
        // A `&mut` range is stepped through as it's taken from.
        if let ty::Ref(_, inner, _) = ty.kind()
            && self.range_kind(*inner).is_some()
        {
            return Err(self.unsupported(span, &format!("iterating over a `{ty}`, which steps it")));
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

    /// Whether `ty`'s JS iterator knows its back, which `$nextBack` steps
    /// (ADR 0429): a `dyn DoubleEndedIterator`'s, or a type parameter's
    /// bound by one, given a JS iterator (ADR 0061), not as it is (ADR 0411).
    /// The one answer, of what generic code steps from its back and of what
    /// its callers give it.
    pub(in crate::lower) fn knows_its_back(&self, ty: ty::Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        // A map of one, made one that knows its back too, `$mapEnds` (ADR 0455).
        if let Some(source) = super::recognition::mapped_source(self.tcx, ty) {
            return self.knows_its_back(source);
        }
        if let Some(id) = super::recognition::dyn_iter_trait(self.tcx, ty) {
            return super::recognition::is_double_ended(self.tcx, id);
        }
        self.double_ended_param(self.tcx.typeck_root_def_id(self.item), ty)
    }

    /// Whether `ty` is `id`'s type parameter bound by `DoubleEndedIterator`,
    /// given a JS iterator that knows its back (`knows_its_back`).
    fn double_ended_param(&self, id: DefId, ty: ty::Ty<'tcx>) -> bool {
        self.extension_param(id, ty, super::recognition::is_double_ended)
    }

    /// Whether `ty` is `id`'s type parameter bound by `ExactSizeIterator`,
    /// given a JS iterator that knows its length (`knows_its_length`).
    fn exact_size_param(&self, id: DefId, ty: ty::Ty<'tcx>) -> bool {
        self.extension_param(id, ty, super::recognition::is_exact_size)
    }

    /// Whether `ty` is `id`'s type parameter bound by an iterator's
    /// extension trait, `is`'s, given a JS iterator (ADR 0061), not as it is
    /// (ADR 0411).
    fn extension_param(&self, id: DefId, ty: ty::Ty<'tcx>, is: fn(ty::TyCtxt<'tcx>, DefId) -> bool) -> bool {
        let ty::Param(param) = ty.kind() else {
            return false;
        };
        let env = ty::TypingEnv::post_analysis(self.tcx, id);
        !given_as_it_is(self.tcx, id, *param)
            && (self.declared_bounds(ty, env.param_env).into_iter()).any(|bound| is(self.tcx, bound))
    }

    /// Whether a value of `ty` is a JS iterator that knows its length, its
    /// `len()` (ADR 0456): a type parameter's bound by `ExactSizeIterator`,
    /// a `dyn`'s, or a map of one. The one answer, of what generic code asks
    /// the length of and what its callers give it.
    pub(in crate::lower) fn knows_its_length(&self, ty: ty::Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        if let Some(source) = super::recognition::mapped_source(self.tcx, ty) {
            return self.knows_its_length(source);
        }
        if let Some(id) = super::recognition::dyn_iter_trait(self.tcx, ty) {
            return super::recognition::is_exact_size(self.tcx, id);
        }
        self.exact_size_param(self.tcx.typeck_root_def_id(self.item), ty)
    }

    /// `value`, an iterator of type `ty`, given where generic code asks its
    /// length, an `ExactSizeIterator`'s, and its back, `back` (ADR 0456): a
    /// JS iterator that knows them, an array's, its `$iter`; a range's of
    /// numbers, `$iterator(range, $rangeNext, false, $rangeNextBack,
    /// $rangeLen)`; one already, itself.
    pub(in crate::lower) fn as_exact_size(&mut self, value: Expr, ty: ty::Ty<'tcx>, back: bool, span: Span) -> R<Expr> {
        let ty = self.reveal(ty);
        if self.knows_its_length(ty) && (!back || self.knows_its_back(ty)) {
            return Ok(value);
        }
        if let Some(kind) = self.range_kind(ty)
            && kind == super::std_types::range::RangeKind::Exclusive
            && let (Some(next), Some(from_back)) = (kind.step(false), kind.step(true))
        {
            let from_back = back.then(|| Expr::helper(from_back));
            return Ok(stepped_ends(
                value,
                Expr::helper(next),
                false,
                from_back,
                Some(Expr::helper("$rangeLen")),
            ));
        }
        if self.steps_an_array(ty) {
            return Ok(self.std_iterator(value, ty));
        }
        Err(self.unsupported(
            span,
            &format!("a `{ty}` as an `ExactSizeIterator`, which rust-js can't count"),
        ))
    }

    /// `value`, a double-ended iterator of type `ty`, as a JS iterator that
    /// knows its back (ADR 0429): an array's, its `$iter`; a range's,
    /// `$iterator(range, $rangeNext, false, $rangeNextBack)`; the crate's
    /// own, stepped by its impls' `next` and `next_back`; one already, a
    /// `dyn`'s or a type parameter's, itself.
    pub(in crate::lower) fn as_double_ended(&mut self, value: Expr, ty: ty::Ty<'tcx>, span: Span) -> R<Expr> {
        let ty = self.reveal(ty);
        if self.knows_its_back(ty) {
            return Ok(value);
        }
        if let Some(kind) = self.range_kind(ty)
            && let (Some(next), Some(back)) = (kind.step(false), kind.step(true))
        {
            return Ok(stepped(value, Expr::helper(next), false, Some(Expr::helper(back))));
        }
        if self.steps_an_array(ty) {
            return Ok(self.std_iterator(value, ty));
        }
        // A map made here of one that knows its back, `src.map(f)`: its
        // source's items, each `f`'s, from either end, as Rust's `Map` steps
        // them (ADR 0455).
        if let Some(source) = super::recognition::mapped_source(self.tcx, ty)
            && let js::ExprKind::Call(callee, args) = &value.kind
            && let js::ExprKind::Member(source_value, method) = &callee.kind
            && method == "map"
            && let [f] = args.as_slice()
        {
            let source_value = self.as_double_ended((**source_value).clone(), source.peel_refs(), span)?;
            return Ok(Expr::call_helper("$mapEnds", vec![source_value, f.clone()]));
        }
        if let Some(double_ended) = self.own_next_back(ty)? {
            let iterator = std_item(self.tcx, StdItem::Iterator);
            let (next, boxed) = self.user_next(ty, iterator, "next", span)?;
            let (back, _) = self.user_next(ty, double_ended, "next_back", span)?;
            return Ok(stepped(value, next, boxed, Some(back)));
        }
        Err(self.unsupported(
            span,
            &format!("a `{ty}` as a `DoubleEndedIterator`, which rust-js can't step from its back"),
        ))
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
        Ok(Expr::call(Expr::helper("$exactLen"), vec![hint]))
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
        let (next, boxed) = self.user_next(ty, trait_id, method, span)?;
        Ok(stepped(value, next, boxed, None))
    }

    /// The function that steps the crate's iterator of type `ty`, its impl's
    /// `method` of `trait_id`, `countdownIterator_next`, and whether its
    /// `Some` is boxed, as a generic `next`'s is (ADR 0051).
    fn user_next(&mut self, ty: ty::Ty<'tcx>, trait_id: DefId, method: &str, span: Span) -> R<(Expr, bool)> {
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
        Ok((next, boxed))
    }

    /// The `Iterator` dictionary this function was given for `ty`, a type
    /// parameter given as it is (ADR 0411).
    pub(super) fn iterator_evidence(&self, ty: ty::Ty<'tcx>) -> Option<Expr> {
        let ty = ty.peel_refs();
        // Only of one given as it is: another's is a JS iterator (ADR 0061),
        // whatever bound names `Iterator`, a supertrait's too.
        let ty::Param(param) = ty.kind() else {
            return None;
        };
        if !given_as_it_is(self.tcx, self.tcx.typeck_root_def_id(self.item), *param) {
            return None;
        }
        let iterator = std_item(self.tcx, StdItem::Iterator);
        self.evidence_for(ty::TraitRef::new(self.tcx, iterator, [ty]))
    }

    /// `ty`'s `Iterator` dictionary, `{ next }` (ADR 0411): its `next`
    /// steps a `ty` as it is, a range's `$rangeNext`, the crate's own its
    /// impl's, a JS iterator's `$next`, each giving the `Option` generic code
    /// reads, a `Some` that looks like `None` boxed (ADR 0051).
    pub(super) fn iterator_dictionary(&mut self, ty: ty::Ty<'tcx>, span: Span) -> R<Expr> {
        let ty = self.reveal(ty);
        let next = match self.range_kind(ty) {
            Some(kind) => match kind.step(false) {
                Some(step) => Expr::helper(step),
                None => return Err(self.unsupported(span, &format!("the `Iterator` of a `{ty}`"))),
            },
            None if self.is_user_iterator(ty) => {
                let trait_id = std_item(self.tcx, StdItem::Iterator);
                self.user_next(ty, trait_id, "next", span)?.0
            }
            None => match self.iterator_item(ty) {
                Some(item) if !self.boxed_payload(item) => Expr::helper("$next"),
                _ => Expr::helper("$nextSome"),
            },
        };
        Ok(Expr::object(vec![js::Prop::Field("next".into(), next)]))
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

    /// What an `IntoIterator` of type `ty` gives, its `Item`.
    pub(in crate::lower) fn iterable_item(&self, ty: ty::Ty<'tcx>) -> ty::Ty<'tcx> {
        let into_iterator = std_item(self.tcx, StdItem::IntoIterator);
        let item = trait_method(self.tcx, into_iterator, "Item");
        let projection = ty::Ty::new_projection(self.tcx, ty::IsRigid::No, item, [ty]);
        (self.tcx)
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(projection))
            .unwrap_or(projection)
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
    /// `max()` or `min()` of `items`, an array of `item`s: of
    /// what JS's `<` doesn't order, by its `cmp` (ADR 0057), each passed over
    /// dropped as it's passed, of items with a destructor (ADR 0098).
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
                let helper = if max { "$maxBy" } else { "$minBy" };
                self.extreme_by(helper, vec![items, compare], item, span)?
            }
            _ => Expr::call(Expr::helper(if max { "$max" } else { "$min" }), vec![items]),
        })
    }

    /// `helper`, `$maxBy` or one of its kind, given `args`, then whether an
    /// `item` is boxed where it may look like `None` (ADR 0051), and what
    /// drops each passed over, of one with a destructor (ADR 0098).
    pub(in crate::lower) fn extreme_by(
        &mut self,
        helper: &'static str,
        mut args: Vec<Expr>,
        item: ty::Ty<'tcx>,
        span: Span,
    ) -> R<Expr> {
        let boxed = self.boxed_payload(item);
        let drop = self.std_drop_function(item, span)?;
        if boxed || drop.is_some() {
            args.push(Expr::bool(boxed));
        }
        args.extend(drop);
        Ok(Expr::call(Expr::helper(helper), args))
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
            // Its value's drop too, which it runs when it's closed (ADR 0422).
            Std::IterSource(IterSource::Repeat) => {
                let value = arg();
                let item = generic_args.type_at(0);
                let mut list = vec![value];
                let drop = self.drop_function(item, span)?;
                if self.needs_clone(item) {
                    list.push(self.clone_fn("value", item, span)?);
                } else if drop.is_some() {
                    list.push(Expr::undefined());
                }
                list.extend(drop);
                Expr::call(Expr::helper("$repeating"), list)
            }
            Std::IterSource(IterSource::RepeatWith) => Expr::call(Expr::helper("$repeatingWith"), vec![arg()]),
            // Their closures' `Option`s: a `Some` that looks like `None` is boxed (ADR 0051).
            Std::IterSource(source @ (IterSource::Successors | IterSource::FromFn)) => {
                let item = generic_args.type_at(0);
                let name = match source {
                    IterSource::Successors => "$successors",
                    _ => "$fromFn",
                };
                let boxed = self.boxed_payload(item);
                let mut list: Vec<Expr> = (0..tys.len()).map(|_| arg()).collect();
                if boxed {
                    list.push(Expr::bool(true));
                }
                Expr::call(Expr::helper(name), list)
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
                // What it takes out, a heap's `drain()`'s.
                "splice",
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
                "$args",
                "$vars",
                "$pathParts",
                "$pathAncestors",
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
        if callee.helper_name() == Some("$next")
            && let [chain] = &args[..]
            && let Some((items, _)) = eager_items(chain, false, None)
        {
            *e = Expr {
                span: e.span,
                ..Expr::index(items, Expr::int(0))
            };
            return;
        }
        // What reads every item alone, `$max`'s: of the array itself, not a
        // copy, `$max(xs)`; `$maxBy`'s compares as each comes.
        if matches!(&callee.kind, js::ExprKind::Var(v)
            if ["$max", "$min", "$maxBy", "$minBy", "$maxByKey", "$minByKey"].contains(&v.as_str()))
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
        if callee.helper_name() == Some("$position")
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
            // Its start is made before any item is stepped, as Rust's `fold`
            // is given it: one that does something, before no stage that
            // does, which an array's would run first.
            ("reduce", [f, init]) => match init.has_effects() {
                true => eager_items(chain, false, None),
                false => eager_items(chain, true, Some(f)),
            }
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
    // An owning source closed to drop what it hasn't given, `$closing($iter(xs),
    // drop)`, which a chain of `map`s and `filter`s run over every item never
    // closes (ADR 0465): its items.
    let at = match &at.kind {
        js::ExprKind::Call(callee, args)
            if callee.helper_name() == Some("$closing")
                && whole
                && stages.iter().all(|(name, _)| matches!(*name, "map" | "filter")) =>
        {
            &args[0]
        }
        _ => at,
    };
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
                && zip.helper_name() == Some("$lazyZip")
                && let [a, b] = &sides[..]
                && is_stepped_items(a)
                && is_stepped_items(b) =>
        {
            Expr::call(Expr::helper("$zip"), vec![iterated(a.clone()), iterated(b.clone())])
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
        // Each item an array, as it is: `flat()`, which takes arrays apart.
        "flatMap" => match flattened.next().expect("made") {
            f if f.is_identity() => Expr::call(Expr::member(items, "flat"), Vec::new()),
            f => array_method(items, name, f),
        },
        name => array_method(items, name, f.clone()),
    });
    Some((items, count))
}

/// Of `Iterator.from(list)` collected, `chain`, each item mapped or filtered
/// on the way: the array it makes, as a page writes it,
/// `Array.from(nodes).map(f)`. Not of the runtime's, `$lazyChain(a, b)`'s,
/// read as their parts.
pub(in crate::lower) fn collected(chain: &Expr) -> Option<Expr> {
    fn sourced(e: &Expr) -> Option<Expr> {
        match &e.kind {
            js::ExprKind::Call(callee, args)
                if let js::ExprKind::Member(source, name) = &callee.kind
                    && matches!(name.as_str(), "map" | "filter" | "take" | "drop" | "flatMap") =>
            {
                Some(Expr::call(Expr::member(sourced(source)?, name), args.clone()))
            }
            _ if is_iterator_from(e) => {
                let list = iterated(e.clone());
                if matches!(&list.kind, js::ExprKind::Call(f, _) if matches!(&f.kind, js::ExprKind::Var(f) if f.starts_with('$')))
                {
                    return None;
                }
                let items = Expr::call(Expr::member(Expr::var("Array"), "from"), vec![list]);
                Some(array_stepped(items))
            }
            _ => None,
        }
    }
    eager_items(&sourced(chain)?, true, None).map(|(items, _)| items)
}

/// Of `Iterator.from($lazyChain(a, b))`, a `chain` MIR makes: what each part
/// steps, `a` and `b`, which spread in turn as it steps them, `[...a, ...b]`;
/// of a part `Iterator.from(xs)`, or `cloned()`'s `xs.map((x) => x)`, `xs`.
fn chained(e: &Expr) -> Option<Vec<Expr>> {
    fn part(e: &Expr) -> Vec<Expr> {
        match &e.kind {
            js::ExprKind::Call(callee, args) => match (&callee.kind, args.as_slice()) {
                (_, [a, b]) if callee.helper_name() == Some("$lazyChain") => [part(a), part(b)].concat(),
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
    (is_iterator_from(e) && callee.helper_name() == Some("$lazyChain")).then(|| part(chain))
}

/// Whether `e` is `Iterator.from(x)`.
fn is_iterator_from(e: &Expr) -> bool {
    matches!(&e.kind, js::ExprKind::Call(callee, args) if args.len() == 1
        && matches!(&callee.kind, js::ExprKind::Member(object, from)
            if from == "from" && matches!(&object.kind, js::ExprKind::Var(v) if v == "Iterator")))
}

/// Whether `e` is `$lent(x)`, a JS iterator nothing closes.
fn is_lent(e: &Expr) -> bool {
    !std::ptr::eq(unlent(e), e)
}

/// What `$lent(e)` lends, `e`, made for this call alone, which no one else
/// steps; or `e` itself.
pub(super) fn unlent(e: &Expr) -> &Expr {
    match &e.kind {
        js::ExprKind::Call(lent, of) if lent.helper_name() == Some("$lent") && of.len() == 1 => &of[0],
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
    // as Rust's types tell (`Expr::read`), not a crate's own `at`, `P.at(p)`:
    // their operands', as `has_effects` says of the runtime's pure helpers,
    // `$find(s, t)`; and such a helper given as it is.
    const READS: &[&str] = &["startsWith", "endsWith", "includes", "toUpperCase", "toLowerCase", "at"];
    fn acts(e: &Expr) -> bool {
        let mut e = e.clone();
        e.each_mut(&mut |part| {
            if let js::ExprKind::Call(callee, args) = &part.kind {
                let reads = match &callee.kind {
                    js::ExprKind::Member(_, name) => part.reads && READS.contains(&name.as_str()),
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
        js::ExprKind::Var(_) => !f.helper_name().is_some_and(crate::runtime::is_pure),
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
            && iter.helper_name() == Some("$iter")
            && items.iter().all(Expr::reads_only_vars)
    } else {
        false
    }
}

pub(super) fn is_stepped_items(e: &Expr) -> bool {
    matches!(&e.kind, js::ExprKind::Call(callee, _) if callee.helper_name() == Some("$iter"))
}

/// Whether `e` is a JS iterator as it's made: `Iterator.from(..)`, a
/// `$iter` or a range's `$iterator`, a `$cycle`'s generator, a
/// collection's `values()`, `keys()` or `entries()`, or an iterator
/// helper's of one, `.map(f)` of a range's.
pub(super) fn is_js_iterator(e: &Expr) -> bool {
    let js::ExprKind::Call(callee, _) = &e.kind else {
        return false;
    };
    if callee.helper_name().is_some() {
        return matches!(callee.helper_name(), Some("$iterator" | "$cycle")) || is_stepped_items(e);
    }
    match &callee.kind {
        // Not `Object.keys(o)`'s, an array.
        js::ExprKind::Member(object, name) => match &object.kind {
            js::ExprKind::Var(v) if v == "Object" => false,
            js::ExprKind::Var(v) if v == "Iterator" => name == "from",
            _ if matches!(name.as_str(), "map" | "filter" | "flatMap" | "take" | "drop") => is_js_iterator(object),
            _ => matches!(name.as_str(), "values" | "keys" | "entries"),
        },
        _ => false,
    }
}

/// What `Iterator.from(x)` makes a JS iterator of, `x`: `Ok` of it, or
/// `Err` of `e` itself, of anything else.
pub(super) fn from_iterator(e: Expr) -> Result<Expr, Expr> {
    match e.kind {
        js::ExprKind::Call(callee, mut args)
            if args.len() == 1
                && matches!(&callee.kind, js::ExprKind::Member(object, name)
                    if name == "from" && matches!(&object.kind, js::ExprKind::Var(v) if v == "Iterator")) =>
        {
            Ok(args.pop().expect("one"))
        }
        kind => Err(Expr {
            reads: false,
            kind,
            span: e.span,
        }),
    }
}

/// A range's numbers as `stepped_by` steps them, `$iterator({ start: a,
/// end: b }, $rangeNext)`: its bounds, whether its end is in it, `a..=b`'s,
/// and whether it's stepped from its end.
pub(super) struct SteppedRange<'e> {
    pub(super) start: &'e Expr,
    pub(super) end: &'e Expr,
    pub(super) inclusive: bool,
    pub(super) back: bool,
}

pub(super) fn stepped_range(e: &Expr) -> Option<SteppedRange<'_>> {
    let js::ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    let (Some(iterator), [range, next]) = (callee.helper_name(), &args[..]) else {
        return None;
    };
    let (js::ExprKind::Object(bounds), Some(next)) = (&range.kind, next.helper_name()) else {
        return None;
    };
    let [js::Prop::Field(s, start), js::Prop::Field(f, end)] = &bounds[..] else {
        return None;
    };
    let (inclusive, back) = [(false, false), (false, true), (true, false), (true, true)]
        .into_iter()
        .find(|&(inclusive, back)| {
            let kind = if inclusive {
                RangeKind::Inclusive
            } else {
                RangeKind::Exclusive
            };
            kind.step(back) == Some(next)
        })?;
    (iterator == "$iterator" && (s.as_str(), f.as_str()) == ("start", "end")).then_some(SteppedRange {
        start,
        end,
        inclusive,
        back,
    })
}

/// What `for (x of e)` steps through of `e`, `$iter(xs)`'s or
/// `Iterator.from(xs)`'s: `xs` itself.
pub(super) fn iterated(e: Expr) -> Expr {
    match e.kind {
        js::ExprKind::Call(callee, mut args)
            if args.len() == 1
                && match &callee.kind {
                    _ if callee.helper_name().is_some() => callee.helper_name() == Some("$iter"),
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

/// What `it`, an iterator made here of an array, `$iter(v)`, steps: `v`,
/// all of it left.
pub(super) fn fresh_iter_source(it: &Expr) -> Option<&Expr> {
    match &it.kind {
        js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$iter") && args.len() == 1 => args.first(),
        _ => None,
    }
}

/// `$iter(items)`, `stepped_items`'s, where no `FnCx` is.
pub(super) fn array_stepped(items: Expr) -> Expr {
    Expr::call(Expr::helper("$iter"), vec![items])
}
