//! Iterators from MIR (ADR 0364): each a JS iterator, whose helpers are
//! lazy as Rust's adapters are, so what each closure does happens in the
//! order Rust's does, item by item. A range is its `{ start, end }` until
//! it's iterated, and an iterator of the crate's its impl's `next`, made a
//! JS iterator where one's wanted. What's taken by a `&mut`, `any` of
//! `&mut it`, steps it through `$lent`, which a helper that stops early
//! closes in its place, as Rust leaves the iterator to go on.

use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, Mutability, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;

use crate::js::{self, Expr, Op, Stmt, StmtKind};
use crate::lower::iterators::{collected, from_iterator, is_js_iterator, made_array, stepped_range};
use crate::lower::recognition::{IterMethod, StdItem, TypeFact, is_std_def, std_item};
use crate::lower::representation::Num;
use crate::lower::std_types::heap::HeapOp;
use crate::lower::std_types::range::RangeKind;
use crate::lower::{FnCx, R, bindings};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A call of an `Iterator`'s or an `IntoIterator`'s method, of a std
    /// iterator, from MIR: `None` if it's another.
    pub(in crate::lower) fn mir_iter_call(
        &mut self,
        def_id: DefId,
        arg_tys: &[Ty<'tcx>],
        mut values: Vec<Expr>,
        output: Ty<'tcx>,
        span: Span,
        made: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let tcx = self.tcx;
        let Some(trait_id) = tcx.trait_of_assoc(def_id) else {
            return Ok(None);
        };
        let receiver_ty = arg_tys.first().copied().unwrap_or(tcx.types.unit);
        if !receiver_ty.is_ref()
            && is_std_def(tcx, trait_id, StdItem::Iterator)
            && self.dropped_after(def_id, receiver_ty, output, !values.is_empty(), span)?
        {
            let held = self.spill("iter", values.remove(0), made);
            values.insert(0, held.clone());
            let Some(result) = self.mir_iter_method(def_id, (trait_id, arg_tys), values, output, span, made)? else {
                return Ok(None);
            };
            let result = self.spill("result", result, made);
            self.drop_value(held, receiver_ty, span, made)?;
            return Ok(Some(result));
        }
        self.mir_iter_method(def_id, (trait_id, arg_tys), values, output, span, made)
    }

    /// Whether an iterator taken by value, `items.fold(..)`'s, of a
    /// `receiver` with something to drop, is dropped after its method, as
    /// the method's end drops it and JS doesn't: consumed, `collect()`, then
    /// dropped, by its own drop, the crate's own, arrayvec's `Drain`, or its
    /// caller's, a generic iterator's it's given (ADRs 0421, 0464). One whose
    /// result holds it, `map(f)`, is the result's to drop, closed to drop it
    /// (ADR 0465), or refused; of a generic one given no drop, its callers
    /// give none with a destructor (ADR 0190).
    fn dropped_after(
        &mut self,
        def_id: DefId,
        receiver: Ty<'tcx>,
        output: Ty<'tcx>,
        given: bool,
        span: Span,
    ) -> R<bool> {
        if self.drops(receiver) == crate::lower::drops::Drops::Nothing {
            return Ok(false);
        }
        let params_only = self.drop_query().drops_but_params(receiver) == crate::lower::drops::Drops::Nothing;
        let dropped = match params_only {
            true => self.given_drop(receiver).is_some(),
            false => self.is_user_iterator(receiver),
        };
        let holds = output.walk().any(|part| part.as_type() == Some(receiver));
        if dropped && given && !holds {
            return Ok(true);
        }
        if params_only {
            self.require_consumed_drops_none(receiver, output);
        } else if self.is_user_iterator(receiver) && given && self.recognition().closed_on_drop(output).is_none() {
            let name = self.tcx.item_name(def_id);
            return Err(self.unsupported(
                span,
                &format!("`{name}` of an iterator with a destructor, from its MIR"),
            ));
        }
        Ok(false)
    }

    /// `mir_iter_call`'s, once what it drops is seen to.
    fn mir_iter_method(
        &mut self,
        def_id: DefId,
        (trait_id, arg_tys): (DefId, &[Ty<'tcx>]),
        values: Vec<Expr>,
        output: Ty<'tcx>,
        span: Span,
        made: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let tcx = self.tcx;
        // Its name, for what's refused of it, and to ask of the crate's impl.
        let name = tcx.item_name(def_id);
        let name = name.as_str();
        let std_method = self.recognition().iterator_method(def_id);
        let receiver_ty = arg_tys.first().copied().unwrap_or(tcx.types.unit);
        let iter_ty = receiver_ty.peel_refs();
        if let Some(stepped) = self.given_iterator_call(std_method, iter_ty, &values) {
            return Ok(Some(stepped));
        }
        // `x.into_iter()`: a range stays one, an iterator is itself, and a
        // collection's items are a JS iterator of them.
        if std_method == Some(IterMethod::IntoIter) {
            let value = values.into_iter().next().expect("the receiver");
            if self.recognition().is_given_iterable(receiver_ty) {
                return Ok(Some(crate::lower::iterators::given_into_iter(value)));
            }
            if self.range_kind(iter_ty).is_some() || self.implements_iterator(receiver_ty) {
                return Ok(Some(value));
            }
            // `for x in &mut v` of numbers: handles on its items, as
            // `iter_mut()`'s are, `mir_std_call`'s (ADR 0099).
            let handles = matches!(receiver_ty.kind(), ty::Ref(_, _, Mutability::Mut))
                && self.iterator_item(output).is_some_and(|item| self.is_cell(item));
            if handles {
                return Ok(None);
            }
            if iter_ty.is_array() || iter_ty.is_slice() || self.is_vec_like(iter_ty) {
                return Ok(Some(self.std_iterator(value, output)));
            }
            return Ok(None);
        }
        // `B::from_iter(items)`, which `items.collect()` is (ADR 0160).
        if std_method == Some(IterMethod::FromIter) {
            return self.collected_from(values, receiver_ty, output, span);
        }
        // `nth_back(n)`, `rfold(init, f)` and `rfind(p)`: of what knows its
        // back, stepped from it, a generic one's too (ADRs 0429, 0455).
        if let Some(method @ (IterMethod::NthBack | IterMethod::RFold | IterMethod::RFind)) = std_method {
            return self.back_step(method, values, (iter_ty, output), span).map(Some);
        }
        // `next_back(&mut it)` and `len(&it)`: of what knows its back, a
        // range or a `$iter` (ADR 0071); not a lazy or a generic one.
        if matches!(std_method, Some(IterMethod::NextBack | IterMethod::Len)) {
            // The crate's own `len()`, std's: its `size_hint()`, exact.
            let own_len = || -> R<bool> {
                let args = self.args_of(trait_id, iter_ty);
                Ok(self
                    .resolve_instance(def_id, args)?
                    .is_some_and(|i| self.is_rust_fn(i.def_id())))
            };
            if self.is_user_iterator(iter_ty) && std_method == Some(IterMethod::Len) && !own_len()? {
                let it = values.into_iter().next().expect("the receiver");
                let mut made = Vec::new();
                let len = self.exact_len_of(it, iter_ty, span, &mut made)?;
                if !made.is_empty() {
                    return Err(self.unsupported(span, "this `len()`, from its MIR"));
                }
                return Ok(Some(len));
            }
            if self.is_user_iterator(iter_ty) {
                return Ok(None);
            }
            let it = values.into_iter().next().expect("the receiver");
            let boxed = self.option_of(output).is_some_and(|item| self.boxed_payload(item));
            let array = self.has_a_back(iter_ty);
            return Ok(Some(match (std_method, self.range_kind(iter_ty)) {
                (Some(IterMethod::NextBack), Some(kind)) if let Some(step) = kind.step(true) => {
                    Expr::call_helper(step, vec![it])
                }
                // A map of an array's items, `v.iter().map(f)`: its source's
                // last item, mapped, as Rust's `Map::next_back` maps that one
                // alone.
                (Some(IterMethod::NextBack), None)
                    if !boxed
                        && crate::lower::iterators::is_items_map(&it)
                        && let js::ExprKind::Call(callee, args) = &it.kind
                        && let js::ExprKind::Member(source, _) = &callee.kind =>
                {
                    let last = Expr::call_helper("$nextBack", vec![(**source).clone()]);
                    let last = self.spill("last", last, made);
                    let none = Expr::bin(Op::Eq, last.clone(), Expr::undefined());
                    Expr::cond(
                        none,
                        Expr::undefined(),
                        crate::lower::calls::apply(args[0].clone(), vec![last]),
                    )
                }
                (Some(IterMethod::NextBack), None) if array && boxed => Expr::call_helper("$nextBackSome", vec![it]),
                (Some(IterMethod::NextBack), None) if array => Expr::call_helper("$nextBack", vec![it]),
                // A generic one's, a JS iterator that knows it, which may be a
                // range's (ADR 0456).
                (Some(IterMethod::Len), None) if self.knows_its_length(iter_ty) => {
                    Expr::call(Expr::member(it, "len"), Vec::new())
                }
                (Some(IterMethod::Len), None) if array => self.items_left(it, made),
                // A chain made here of an array's items, which nothing else
                // steps, skipped or taken from, no closure of its to run: its
                // items' count, `$iter(xs).drop(1).toArray().length`, which is
                // `xs.slice(1).length`.
                (Some(IterMethod::Len), None) if counted_chain(&it) => {
                    Expr::member(Expr::call(Expr::member(it, "toArray"), Vec::new()), "length")
                }
                // A range of numbers' count (`range_len`).
                (Some(IterMethod::Len), Some(kind)) if let Some(count) = self.range_len(&it, iter_ty, kind) => count,
                _ => {
                    // Its type's name, not its closures', which name where they're written.
                    let shown = match iter_ty.kind() {
                        ty::Adt(adt, _) => self.tcx.def_path_str(adt.did()),
                        _ => iter_ty.to_string(),
                    };
                    return Err(self.unsupported(
                        span,
                        &format!("`{name}()` of a lazy or generic `{shown}`, from its MIR"),
                    ));
                }
            }));
        }
        // `rev()` and `peekable()`: of the crate's iterator, its own
        // `next_back`; of a range, stepped from its end; of items made
        // already, an array's, a `$iter` of them; not of a lazy one, whose
        // closures would run in another order (ADR 0164).
        if matches!(std_method, Some(IterMethod::Rev | IterMethod::Peekable)) {
            let receiver = values.into_iter().next().expect("the receiver");
            if self.is_user_iterator(iter_ty) {
                return match self.own_next_back(iter_ty)? {
                    Some(double_ended) if std_method == Some(IterMethod::Rev) => self
                        .user_iterator(receiver, iter_ty, double_ended, "next_back", span)
                        .map(Some),
                    _ => Err(self.unsupported(span, &format!("`{name}` of a `{iter_ty}`, from its MIR"))),
                };
            }
            if std_method == Some(IterMethod::Rev) && self.knows_its_back(iter_ty) {
                return Ok(Some(self.reversed(receiver, iter_ty)));
            }
            let range = self.range_kind(iter_ty);
            return Ok(Some(match (std_method, range) {
                (Some(IterMethod::Rev), Some(kind)) if let Some(step) = kind.step(true) => {
                    self.stepped_by(receiver, Expr::helper(step))
                }
                (_, Some(_)) => {
                    return Err(self.unsupported(span, &format!("`{name}` of a `{iter_ty}`, from its MIR")));
                }
                _ => self.rev_or_peekable_items(receiver, iter_ty, std_method, name, span)?,
            }));
        }
        // `size_hint()` of a `$iter`, exact, or of a generic one, as `$sizeHint`
        // finds it (ADR 0170); a range's is its bounds' (`RangeOp::SizeHint`).
        if std_method == Some(IterMethod::SizeHint)
            && !self.is_user_iterator(iter_ty)
            && self.range_kind(iter_ty.peel_refs()).is_none()
        {
            let mut it = values.into_iter().next().expect("the receiver");
            // A `map`'s is its source's, as Rust's `Map` passes it on; its
            // closure, unrun, is made for nothing.
            let mut mapped = false;
            while let js::ExprKind::Call(callee, args) = &it.kind
                && let js::ExprKind::Member(source, method) = &callee.kind
                && method == "map"
                && args.iter().all(|f| !f.has_effects())
            {
                it = (**source).clone();
                mapped = true;
            }
            let known = match mapped {
                true => crate::lower::iterators::is_stepped_items(&it),
                false => self.hints_its_size(iter_ty),
            };
            if !known {
                return Err(self.unsupported(span, &format!("`size_hint()` of a `{iter_ty}`, from its MIR")));
            }
            return Ok(Some(Expr::call_helper("$sizeHint", vec![it])));
        }
        // `next` of a `&mut` to the crate's iterator, `by_ref()`'s: std's of a
        // `&mut I`, which is `I`'s own, of what the `&mut` is to.
        if std_method == Some(IterMethod::Next)
            && self.is_user_iterator(iter_ty)
            && let ty::Ref(_, through, _) = receiver_ty.kind()
            && through.is_ref()
        {
            let next = def_id;
            let args = self.args_of(trait_id, iter_ty);
            let mut made = Vec::new();
            let value = values.into_iter().next().expect("the receiver");
            let call = self.trait_call(next, args, vec![value], span, &mut made)?;
            return match call {
                Some(call) if made.is_empty() => Ok(Some(call)),
                _ => Err(self.unsupported(span, "`next` of this `&mut`, from its MIR")),
            };
        }
        // Of the crate's iterator, its own `next` and what its impl writes
        // itself are its trait's; the rest, `map`, `fuse`, `collect`, std's
        // of a JS iterator its `next` steps (`as_js_iterator`).
        let own = self.is_user_iterator(iter_ty)
            && (std_method == Some(IterMethod::Next) || self.recognition().user_impl_defines(trait_id, iter_ty, name));
        if !is_std_def(tcx, trait_id, StdItem::Iterator) || own {
            return Ok(None);
        }
        let mut values = values.into_iter();
        let receiver = values.next().expect("the receiver");
        // `next(&mut it)`: a range's start moved, or the JS iterator stepped.
        if std_method == Some(IterMethod::Next) {
            let item = self.option_of(output).expect("`next` gives an `Option`");
            return Ok(Some(match self.range_kind(iter_ty) {
                Some(kind) => match kind.step(false) {
                    Some(step) => Expr::call_helper(step, vec![receiver]),
                    None => return Ok(None),
                },
                None if self.boxed_payload(item) => Expr::call_helper("$nextSome", vec![receiver]),
                None => Expr::call_helper("$next", vec![receiver]),
            }));
        }
        // Taken by a `&mut`, it's stepped through what can't close it, but by
        // `try_fold`'s helper, which steps it as it is and closes nothing.
        let lent =
            matches!(receiver_ty.kind(), ty::Ref(_, _, Mutability::Mut)) && std_method != Some(IterMethod::TryFold);
        if let Some(kept) = self.back_kept(std_method, &receiver, (iter_ty, output), (&mut values, arg_tys), span)? {
            return Ok(Some(kept));
        }
        let it = self.as_js_iterator(receiver, iter_ty, span)?;
        let it = match lent {
            true => self.lent_iterator(it),
            false => it,
        };
        let item = self.iterator_item(iter_ty);
        let dropping = item.filter(|&item| self.drops(item) != crate::lower::drops::Drops::Nothing);
        let it = self.dropping_source(it, (std_method, iter_ty, lent), dropping, name, span)?;
        let mut arg = || values.next().expect("rustc checked the arguments");
        // A function of the crate given as the callback, `filter(is_rule)`:
        // itself, which takes no more than it's given (ADR 0279).
        let own = arg_tys.get(1).is_some_and(|ty| self.is_crate_fn_item(*ty));
        let method = |name: &str, it: Expr, args: Vec<Expr>| Expr::call(Expr::member(it, name), args);
        Ok(Some(match std_method {
            // By what gives each as it is, an untagged variant's constructor,
            // `map(StaticPath::Params)`: no step.
            Some(IterMethod::Map) => match self.unary_callback(arg(), own) {
                f if f.is_identity() => it,
                f => method("map", it, vec![f]),
            },
            // Of items with a destructor: each it doesn't keep dropped as it
            // discards it (ADR 0098).
            Some(IterMethod::Filter) => {
                let test = self.unary_callback(arg(), own);
                let test = match dropping {
                    Some(item) => self.dropping_discarded(test, item, false, span, made)?,
                    None => test,
                };
                method("filter", it, vec![test])
            }
            Some(IterMethod::FlatMap) => method("flatMap", it, vec![self.unary_callback(arg(), own)]),
            Some(IterMethod::Take) => method("take", it, vec![arg()]),
            // Of items with a destructor, each it skips dropped as it's
            // skipped, as Rust's `nth` drops it (ADRs 0422, 0465).
            Some(IterMethod::Skip) => self.skipped(it, arg(), dropping, span)?,
            Some(IterMethod::ForEach) => method("forEach", it, vec![self.unary_callback(arg(), own)]),
            Some(IterMethod::Any) => method("some", it, vec![self.unary_callback(arg(), own)]),
            Some(IterMethod::All) => method("every", it, vec![self.unary_callback(arg(), own)]),
            // Of what looks like `None`: the first kept, boxed.
            Some(IterMethod::Find) => {
                let test = self.unary_callback(arg(), own);
                match (dropping, item.is_some_and(|item| self.boxed_payload(item))) {
                    (Some(item), _) => self.dropping_pick("$findDropping", it, test, item, span)?,
                    (None, true) => Expr::call_helper("$nextSome", vec![method("filter", it, vec![test])]),
                    (None, false) => method("find", it, vec![test]),
                }
            }
            Some(method @ (IterMethod::Position | IterMethod::RPosition)) => {
                self.positioned(it, arg(), (own, method), iter_ty, span)?
            }
            // Each item of both in turn, till one differs or either ends, as
            // Rust's `eq` steps them; `ne` its negation.
            // Of items with a destructor, each pair dropped as it's compared,
            // and each closed as it ends, as std's are dropped (ADR 0465).
            Some(method @ (IterMethod::Eq | IterMethod::Ne)) => {
                let Some(item) = item else { return Ok(None) };
                let ne = method == IterMethod::Ne;
                self.iterators_eq((it, arg()), (item, iter_ty, arg_tys[1]), (lent, ne), span)?
            }
            // The pairing, `(x, i) => [i, x]`, in place of which a
            // callback that takes the pair apart is given each item and its
            // index, of an array (`eager_chains`).
            Some(IterMethod::Enumerate) => {
                let pair = Expr::array(vec![Expr::var("i"), Expr::var("x")]);
                let f = Expr::arrow(
                    vec!["x".into(), "i".into()],
                    vec![StmtKind::Return(Some(pair)).at(js::Span::NONE)],
                );
                method("map", it, vec![f])
            }
            Some(IterMethod::Fold) => {
                let (init, f) = (arg(), arg());
                self.folded(it, init, f, arg_tys)
            }
            Some(IterMethod::TryFold) => {
                let (init, f) = (arg(), arg());
                self.try_folded(it, init, f, (arg_tys, output), span)?
            }
            // Each item counted, and dropped as `count` drops it.
            Some(IterMethod::Count) => {
                let n = self.fresh("n");
                let x = self.fresh("item");
                let mut body = Vec::new();
                if let Some(item) = item {
                    self.drop_value(Expr::var(&x), item, span, &mut body)?;
                }
                let mut params = vec![n.clone().into()];
                if !body.is_empty() {
                    params.push(x.into());
                }
                body.push(StmtKind::Return(Some(Expr::bin(Op::Add, Expr::var(&n), Expr::int(1)))).at(js::Span::NONE));
                method("reduce", it, vec![Expr::arrow(params, body), Expr::int(0)])
            }
            Some(IterMethod::Sum | IterMethod::Product) => {
                let num = Num::of(output)
                    .ok_or_else(|| self.unsupported(span, &format!("`{name}` of a `{output}`, from its MIR")))?;
                let (op, start) = match std_method {
                    Some(IterMethod::Sum) => (Op::Add, 0),
                    _ => (Op::Mul, 1),
                };
                let (a, b) = (self.fresh("a"), self.fresh("b"));
                let total = num.wrap(Expr::bin(op, Expr::var(&a), Expr::var(&b)));
                let f = Expr::arrow(
                    vec![a.into(), b.into()],
                    vec![StmtKind::Return(Some(total)).at(js::Span::NONE)],
                );
                // Rust's float `Sum` starts at -0.0, an empty sum's and one of
                // negative zeros' sign kept.
                let start = match (std_method, num.float()) {
                    (Some(IterMethod::Sum), true) => Expr::num(-0.0),
                    _ => num.literal(start),
                };
                method("reduce", it, vec![f, start])
            }
            Some(IterMethod::Copied) => it,
            Some(IterMethod::Cloned) => match item {
                Some(item) if self.is_copy(item.peel_refs()) => it,
                Some(item) => {
                    let x = self.fresh("item");
                    let mut body = Vec::new();
                    let copy = self.clone_value(Expr::var(&x), item.peel_refs(), span, &mut body)?;
                    body.push(StmtKind::Return(Some(copy)).at(js::Span::NONE));
                    method("map", it, vec![Expr::arrow(vec![x.into()], body)])
                }
                None => return Ok(None),
            },
            // Of all the items, in order, as of an array.
            Some(IterMethod::Max | IterMethod::Min) => {
                let items = method("toArray", it, Vec::new());
                self.extreme_of(std_method == Some(IterMethod::Max), items, item, span)?
            }
            Some(IterMethod::Last) => self.last_value(it, item, span, made)?,
            Some(IterMethod::Nth) => match dropping {
                Some(item) => self.dropping_pick("$nthDropping", it, arg(), item, span)?,
                None => return Ok(None),
            },
            Some(IterMethod::ByRef) => it,
            Some(IterMethod::Fuse) => self.fused(it, iter_ty, span)?,
            Some(IterMethod::Collect) => self.collected_into(it, output, item, Some(iter_ty), span)?,
            _ => return Ok(None),
        }))
    }

    /// Whether `$sizeHint` finds an `iter_ty`'s size: a `$iter`'s, exact, a
    /// generic one's, as it finds it (ADR 0170), and a `Fuse`'s of one, what
    /// it steps, or none once it's ended (ADR 0473).
    fn hints_its_size(&self, iter_ty: Ty<'tcx>) -> bool {
        let stepped = self.recognition().fuse_of(iter_ty).unwrap_or(iter_ty);
        self.array_source(stepped).is_some() || self.is_unknown(stepped)
    }

    /// `it.fuse()`, of an `iter_ty`: `$fuse(it)`, which steps it till it
    /// ends, then never again, and of a `FusedIterator`, `$fuse(it, true)`,
    /// keeps it (ADR 0473).
    fn fused(&mut self, it: Expr, iter_ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let mut list = vec![it];
        let fused = self.type_fact_value(iter_ty, TypeFact::Fused, span)?;
        if !matches!(fused.kind, js::ExprKind::Bool(false)) {
            list.push(fused);
        }
        Ok(Expr::call_helper("$fuse", list))
    }

    /// `it.collect()` into `output`: a heap of its items in heap order,
    /// `$heapFrom`; an array, a range's mapped where it's made; text, its
    /// pieces joined; a `Result` or an `Option` of one of these, the first
    /// `Err` or `None` or all; a map or a set, as a `From` of items makes one
    /// (ADR 0121). Of the `Ok` values of a fallible one, an array already,
    /// `iter_ty` is none, and an array is itself.
    /// `B::from_iter(items)`, `output` of `items` of type `given`: an
    /// array's, a `Vec`'s or a slice's items as they are, another's iterator
    /// as `collect` steps it, collected into `B` as `collect` collects.
    fn collected_from(&mut self, values: Vec<Expr>, given: Ty<'tcx>, output: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        let items = values.into_iter().next().expect("the items");
        let iter_ty = given.peel_refs();
        if iter_ty.is_array() || iter_ty.is_slice() || self.is_vec_like(iter_ty) {
            let item = match *iter_ty.kind() {
                ty::Array(item, _) | ty::Slice(item) => Some(item),
                ty::Adt(_, args) => args.types().next(),
                _ => None,
            };
            return Ok(Some(self.collected_into(items, output, item, None, span)?));
        }
        if !matches!(given.kind(), ty::Param(_)) && !self.implements_iterator(given) {
            return Ok(None);
        }
        let it = self.as_js_iterator(items, given, span)?;
        let item = self.iterator_item(given);
        Ok(Some(self.collected_into(it, output, item, Some(given), span)?))
    }

    pub(in crate::lower) fn collected_into(
        &mut self,
        it: Expr,
        output: Ty<'tcx>,
        item: Option<Ty<'tcx>>,
        iter_ty: Option<Ty<'tcx>>,
        span: Span,
    ) -> R<Expr> {
        let method = |name: &str, it: Expr, args: Vec<Expr>| Expr::call(Expr::member(it, name), args);
        let items = |it: Expr| match iter_ty {
            Some(_) => method("toArray", it, Vec::new()),
            None => it,
        };
        Ok(if self.is_std_type(output, StdItem::BinaryHeap) {
            self.heap_values(HeapOp::From, vec![items(it)], output, span)?
        } else if self.is_vec_like(output) {
            let Some(iter_ty) = iter_ty else { return Ok(it) };
            match self.mapped_range(&it, iter_ty) {
                Some(items) => items,
                None => match self.iterable_items(&it, iter_ty) {
                    Some(items) => items,
                    None => method("toArray", it, Vec::new()),
                },
            }
        } else if self.is_lang_adt(output, LangItem::String) {
            method("join", items(it), vec![Expr::str("")])
        } else if self.option_of(output).is_some() || self.recognition().is_result(output) {
            self.collect_fallible(it, output, item, span)?
        } else if self.is_map(output) {
            let set = self.is_set(output);
            self.made_of(set, self.tcx.mk_args(&[output.into()]), it, span)?
        } else if let Some(made) = self.made_by_own_from_iter(it.clone(), output, item, span)? {
            made
        } else if let ty::Param(_) = output.kind()
            && let Some(item) = item
            && let Some(dictionary) = self.collected_evidence(output, item)
        {
            // A type parameter's, its dictionary's `from_iter` (ADR 0445).
            Expr::call(Expr::member(dictionary, "from_iter"), vec![it])
        } else {
            return Err(self.unsupported(span, &format!("`collect` into a `{output}`, from its MIR")));
        })
    }

    /// `items` made a collection of the crate's own, `output`, by its
    /// `FromIterator<item>`'s `from_iter`, the function rust-js compiled.
    /// The `FromIterator<item>` evidence of a type parameter `output` that
    /// generic code collects into (ADR 0445).
    fn collected_evidence(&self, output: Ty<'tcx>, item: Ty<'tcx>) -> Option<Expr> {
        let from_iterator = std_item(self.tcx, StdItem::FromIterator);
        self.evidence_for(ty::TraitRef::new(self.tcx, from_iterator, [output, item]))
    }

    fn made_by_own_from_iter(
        &mut self,
        items: Expr,
        output: Ty<'tcx>,
        item: Option<Ty<'tcx>>,
        span: Span,
    ) -> R<Option<Expr>> {
        let Some((from_iter, args)) = item.and_then(|item| self.recognition().values_from_iter(output, item)) else {
            return Ok(None);
        };
        let Some(instance) = self
            .resolve_instance(from_iter, args)?
            .filter(|i| self.is_rust_fn(i.def_id()))
        else {
            return Ok(None);
        };
        let mut values = vec![items];
        values.extend(self.evidence_args(instance.def_id(), instance.args, span)?);
        Ok(Some(Expr::call(self.fn_ref(instance.def_id()), values)))
    }

    /// `position(f)`: where the first that `f` finds is, `$position`; and
    /// `rposition(f)`, of a slice's items, the last, `$rposition`, `f`
    /// called from the back as Rust's is, of a `$iter` lent as it is, which
    /// it steps from its back. Of a lazy chain, whose closures would run in
    /// another order, refused.
    fn positioned(
        &mut self,
        it: Expr,
        f: Expr,
        (own, method): (bool, IterMethod),
        iter_ty: Ty<'tcx>,
        span: Span,
    ) -> R<Expr> {
        let found = self.unary_callback(f, own);
        if method == IterMethod::Position {
            return Ok(Expr::call_helper("$position", vec![it, found]));
        }
        if self.array_source(iter_ty).is_none() {
            return Err(self.unsupported(span, "`rposition` of a lazy iterator, from its MIR"));
        }
        let items = crate::lower::iterators::unlent(&it).clone();
        Ok(Expr::call_helper("$rposition", vec![items, found]))
    }

    /// `it.fold(init, f)`: `it.reduce(f, init)`, `f` given after what it
    /// starts from. JS's `reduce` counts its arguments: a start of
    /// `undefined`, a `fold(None, ..)`'s, is given, or it starts from the
    /// first item (ADR 0330).
    fn folded(&mut self, it: Expr, init: Expr, f: Expr, arg_tys: &[Ty<'tcx>]) -> Expr {
        let own = arg_tys.get(2).is_some_and(|ty| self.is_crate_fn_item(*ty));
        let mut args = vec![self.binary_callback(f, own), init];
        crate::lower::calls::given_to_js(&mut args);
        Expr::call(Expr::member(it, "reduce"), args)
    }

    /// `it.try_fold(init, f)` into `output`, an `Option` or a `Result`: each
    /// item folded in till `f` gives a `None` or an `Err`, the rest left to
    /// step, `$tryFoldSome` or `$tryFoldOk` (ADR 0458).
    fn try_folded(
        &mut self,
        it: Expr,
        init: Expr,
        f: Expr,
        (arg_tys, output): (&[Ty<'tcx>], Ty<'tcx>),
        span: Span,
    ) -> R<Expr> {
        let helper = match self.option_of(output) {
            Some(made) if !self.boxed_payload(made) => "$tryFoldSome",
            None if self.recognition().is_result(output) => "$tryFoldOk",
            _ => return Err(self.unsupported(span, &format!("`try_fold` into a `{output}`, from its MIR"))),
        };
        let own = arg_tys.get(2).is_some_and(|ty| self.is_crate_fn_item(*ty));
        let mut args = vec![it, init, self.binary_callback(f, own)];
        crate::lower::calls::given_to_js(&mut args);
        Ok(Expr::call_helper(helper, args))
    }

    /// `rev()` or `peekable()` of a std iterator neither a range nor the
    /// crate's own: of items made already, an array's, a `$iter` of them,
    /// reversed for `rev`; a lazy chain made of its items reversed where its
    /// closures run in the same order; else refused (ADR 0164).
    fn rev_or_peekable_items(
        &mut self,
        receiver: Expr,
        iter_ty: Ty<'tcx>,
        std_method: Option<IterMethod>,
        name: &str,
        span: Span,
    ) -> R<Expr> {
        // A `Fuse` made here, not yet stepped, `$fuse(it)`: the same of what
        // it steps reversed (ADR 0473).
        if std_method == Some(IterMethod::Rev)
            && let Some(inner) = self.recognition().fuse_of(iter_ty)
            && let js::ExprKind::Call(callee, given) = &receiver.kind
            && callee.helper_name() == Some("$fuse")
        {
            let mut given = given.clone();
            given[0] = self.rev_or_peekable_items(given[0].clone(), inner, std_method, name, span)?;
            return Ok(Expr::call_helper("$fuse", given));
        }
        let items = match self.array_source(iter_ty) {
            // One a variable or a field holds, its `$iter`, stepped or
            // not: what's left of it.
            Some(_) if matches!(receiver.kind, js::ExprKind::Var(_) | js::ExprKind::Member(..)) => match std_method {
                Some(IterMethod::Peekable) => return Ok(receiver),
                _ => Expr::call_helper("$rest", vec![receiver]),
            },
            Some(_) => match self.std_iterator(receiver, iter_ty) {
                it if std_method == Some(IterMethod::Peekable) => return Ok(it),
                // Of one just made, the array, which `toReversed`
                // copies anyway.
                it if std_method == Some(IterMethod::Rev) && crate::lower::iterators::is_stepped_items(&it) => {
                    crate::lower::iterators::iterated(it)
                }
                it => Expr::call_helper("$rest", vec![it]),
            },
            // A `$iter` of an array, just made: the array.
            None if crate::lower::iterators::is_stepped_items(&receiver) => match std_method {
                Some(IterMethod::Peekable) => return Ok(receiver),
                _ => crate::lower::iterators::iterated(receiver),
            },
            // Of such an array's skipped and taken from, the array
            // sliced: no closure runs over it.
            None if std_method == Some(IterMethod::Rev) && counted_chain(&receiver) => sliced(&receiver),
            // A chain over an array's items of callbacks that read
            // no index: the chain over them reversed, each called
            // from the back, as Rust's `next_back` calls them.
            None if std_method == Some(IterMethod::Rev)
                && let Some(reversed) = reversed_inward(&receiver) =>
            {
                return Ok(reversed);
            }
            None => match from_iterator(receiver) {
                Ok(items) => Expr::call(Expr::member(Expr::var("Array"), "from"), vec![items]),
                // Of a lazy one, `peekable()`: what steps it when asked, as
                // Rust's `Peekable` does, its closures run in the same order.
                Err(lazy) if std_method == Some(IterMethod::Peekable) => {
                    return Ok(Expr::call_helper("$peekable", vec![lazy]));
                }
                Err(_) => {
                    return Err(self.unsupported(span, &format!("`{name}` of a lazy iterator, from its MIR")));
                }
            },
        };
        let items = match std_method {
            Some(IterMethod::Rev) => Expr::call(Expr::member(items, "toReversed"), Vec::new()),
            _ => items,
        };
        Ok(self.stepped_items(items))
    }

    /// Whether `next_back()` of an iterator of type `iter_ty` is `$nextBack`'s:
    /// a `$iter`, of an array's iterator or a `Peekable` of one, or a generic
    /// or a `dyn` one that knows its back (ADR 0429).
    /// What keeps `receiver`'s back, of type `iter_ty`, where it knows its
    /// back (ADR 0455): `by_ref()`'s, itself, a `$iter` or a `$iterator`,
    /// which closing doesn't end, not lent; a map's, `$mapEnds`, which steps
    /// it from either end and doesn't close it. `None` of another.
    fn back_kept(
        &mut self,
        method: Option<IterMethod>,
        receiver: &Expr,
        (iter_ty, output): (Ty<'tcx>, Ty<'tcx>),
        (values, arg_tys): (&mut std::vec::IntoIter<Expr>, &[Ty<'tcx>]),
        span: Span,
    ) -> R<Option<Expr>> {
        Ok(Some(match method {
            Some(IterMethod::ByRef) if self.knows_its_back(iter_ty) => {
                self.as_double_ended(receiver.clone(), iter_ty, span)?
            }
            // A map of one that knows its length, too (ADR 0456).
            Some(IterMethod::Map) if self.knows_its_back(output) || self.knows_its_length(output) => {
                let it = receiver.clone();
                let own = arg_tys.get(1).is_some_and(|ty| self.is_crate_fn_item(*ty));
                let f = self.unary_callback(values.next().expect("the function"), own);
                Expr::call_helper("$mapEnds", vec![it, f])
            }
            _ => return Ok(None),
        }))
    }

    /// `it.nth_back(n)`, `it.rfold(init, f)` or `it.rfind(p)`, of its
    /// values, the iterator first, of type `iter_ty`, giving `output`: its
    /// helper, stepping `it` from its back (ADR 0455). Of items with a
    /// destructor, what `nth_back` and `rfind` pass over is dropped, as
    /// std's are; `rfold` gives each to its function. Of items that could
    /// look like `None`, what's found is their `Some` (ADR 0051).
    fn back_step(
        &mut self,
        method: IterMethod,
        values: Vec<Expr>,
        (iter_ty, output): (Ty<'tcx>, Ty<'tcx>),
        span: Span,
    ) -> R<Expr> {
        let drop = match self.iterator_item(iter_ty) {
            Some(item) if method != IterMethod::RFold => self.std_drop_function(item, span)?,
            _ => None,
        };
        let boxed = self.option_of(output).is_some_and(|item| self.boxed_payload(item));
        let mut values = values.into_iter();
        let it = values.next().expect("the receiver");
        // Of what has a back, the iterator it steps, as `next_back`'s;
        // another made one, a range's of its bounds, stepped in place.
        let it = match self.range_kind(iter_ty).is_none() && self.has_a_back(iter_ty) {
            true => it,
            false => self.as_double_ended(it, iter_ty, span)?,
        };
        let mut given = vec![it];
        given.extend(values);
        if boxed || drop.is_some() {
            given.push(match boxed {
                true => Expr::helper("$some"),
                false => Expr::undefined(),
            });
        }
        given.extend(drop);
        let helper = match method {
            IterMethod::NthBack => "$nthBack",
            IterMethod::RFold => "$foldBack",
            _ => "$findBack",
        };
        Ok(Expr::call_helper(helper, given))
    }

    fn has_a_back(&self, iter_ty: Ty<'tcx>) -> bool {
        self.steps_an_array(iter_ty)
            || self.knows_its_back(iter_ty)
            || (self.recognition().is_peekable(iter_ty)
                && matches!(iter_ty.kind(), ty::Adt(_, args)
                    if args.types().next().is_some_and(|inner| self.array_source(inner).is_some())))
    }

    /// `rev()` of `it`, a generic or a `dyn` iterator of type `iter_ty` that
    /// knows its back (ADR 0429): a JS iterator stepped by `$nextBack`, its
    /// `Some` boxed where it may look like `None` (ADR 0051).
    fn reversed(&self, it: Expr, iter_ty: Ty<'tcx>) -> Expr {
        let boxed = self.iterator_item(iter_ty).is_none_or(|item| self.boxed_payload(item));
        let back = Expr::helper(if boxed { "$nextBackSome" } else { "$nextBack" });
        crate::lower::iterators::stepped(it, back, boxed, None)
    }

    /// What's stepped as it is: `by_ref()` of the crate's iterator, a `&mut`
    /// to it, it itself, which its own `next` steps; and of a type parameter
    /// given as it is, with its `Iterator` dictionary (ADR 0411), `by_ref()`
    /// and `into_iter()`, itself, and `next(&mut it)`, the dictionary's.
    fn given_iterator_call(&self, method: Option<IterMethod>, iter_ty: Ty<'tcx>, values: &[Expr]) -> Option<Expr> {
        let receiver = values.first()?.clone();
        if method == Some(IterMethod::ByRef) && self.is_user_iterator(iter_ty) {
            return Some(receiver);
        }
        let dictionary = self.iterator_evidence(iter_ty)?;
        match method? {
            IterMethod::Next => Some(Expr::call(Expr::member(dictionary, "next"), vec![receiver])),
            IterMethod::ByRef | IterMethod::IntoIter => Some(receiver),
            _ => None,
        }
    }

    /// The iterator `it`, of type `iter_ty`, a method is called on, of items
    /// with a destructor, `dropping`, or not, `lent` or not: of an adapter,
    /// `take`'s or `map`'s, of one that owns what has one, its items or
    /// itself, a crate's iterator with a `Drop`, moved into it, `$closing`,
    /// whose closing drops it, as the adapter closes it when it ends or is
    /// dropped (ADRs 0422, 0465, 0473), and of one lent, `by_ref().take(n)`,
    /// itself, its lender's (ADR 0468); else one that may leave some items
    /// with a destructor is refused.
    fn dropping_source(
        &mut self,
        it: Expr,
        (method, iter_ty, lent): (Option<IterMethod>, Ty<'tcx>, bool),
        dropping: Option<Ty<'tcx>>,
        name: &str,
        span: Span,
    ) -> R<Expr> {
        if method.is_some_and(IterMethod::holds_its_source) {
            return match lent {
                true => Ok(it),
                false => self.closing(it, iter_ty, span),
            };
        }
        if dropping.is_none() {
            return Ok(it);
        }
        self.check_leaves_none(method, name, span)?;
        Ok(it)
    }

    /// `it`, a JS iterator of a `ty` an adapter is given to hold, closed to
    /// drop what it hasn't given, `$closing(it, drop)`, where `closing_drop`
    /// says (ADRs 0422, 0465); else itself.
    pub(in crate::lower) fn closing(&mut self, it: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        Ok(match self.closing_drop(ty, span)? {
            Some(drop) => Expr::call_helper("$closing", vec![it, drop]),
            None => it,
        })
    }

    /// What closes a `ty` an adapter holds, its own drop: of one that owns
    /// items with a destructor, a generic one whose drop its caller gives,
    /// or the crate's own with one, stepped as `$iterator(value, next)`.
    /// `None` of one with nothing to drop.
    pub(in crate::lower) fn closing_drop(&mut self, ty: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        let owns = match self.recognition().owned_into_items(ty) {
            Some(item) => self.drops(item) != crate::lower::drops::Drops::Nothing,
            None => {
                self.given_drop(ty).is_some()
                    || self.is_user_iterator(ty) && self.drops(ty) != crate::lower::drops::Drops::Nothing
            }
        };
        match owns {
            // The crate's own is stepped as `$iterator(value, next)`, which
            // drops its `value` (ADR 0421).
            true => self.held_drop_function(ty, self.is_user_iterator(ty), span),
            false => Ok(None),
        }
    }

    /// `a.eq(b)`, or `a.ne(b)`, of `a`, of `item`s, an `iter_ty`, `lent` or
    /// not, and `b`, an `IntoIterator` of type `other`: `$iterEq`, each pair compared as
    /// Rust steps them; of items with a destructor, each pair dropped as it's
    /// compared, and each closed as it ends, as std's are dropped (ADR 0465).
    fn iterators_eq(
        &mut self,
        (it, other): (Expr, Expr),
        (item, iter_ty, other_ty): (Ty<'tcx>, Ty<'tcx>, Ty<'tcx>),
        (lent, ne): (bool, bool),
        span: Span,
    ) -> R<Expr> {
        let other = match self.closing_drop(other_ty, span)? {
            Some(drop) => Expr::call_helper("$closing", vec![self.js_iterator(other), drop]),
            None => other,
        };
        let it = match lent {
            true => it,
            false => self.closing(it, iter_ty, span)?,
        };
        let other_item = self.iterable_item(other_ty);
        let compare = self.item_eq((item, other_item), span)?;
        let drop_a = self.std_drop_function(item, span)?;
        let drop_b = self.std_drop_function(other_item, span)?;
        let mut list = vec![it, other];
        if compare.is_some() || drop_a.is_some() || drop_b.is_some() {
            list.push(compare.unwrap_or_else(Expr::undefined));
        }
        if drop_a.is_some() || drop_b.is_some() {
            list.push(drop_a.unwrap_or_else(Expr::undefined));
        }
        list.extend(drop_b);
        let same = Expr::call_helper("$iterEq", list);
        Ok(match ne {
            true => Expr::unary(js::UnaryOp::Not, same),
            false => same,
        })
    }

    /// `skip(n)` of `it`: JS's `drop(n)`; of items with a destructor,
    /// `$skipDropping`, each it skips dropped as it's skipped, as Rust's `nth`
    /// drops it (ADRs 0422, 0465).
    fn skipped(&mut self, it: Expr, n: Expr, dropping: Option<Ty<'tcx>>, span: Span) -> R<Expr> {
        Ok(match dropping {
            Some(item) => {
                let drop = self.std_drop_function(item, span)?.expect("a destructor");
                Expr::call_helper("$skipDropping", vec![it, n, drop])
            }
            None => Expr::call(Expr::member(it, "drop"), vec![n]),
        })
    }

    /// The one item of `it` a method picks, of items of type `item`, which
    /// has a destructor: `nth(n)`'s, `$nthDropping`, and `find(test)`'s,
    /// `$findDropping`, each it passes dropped, as std's are (ADR 0098), its
    /// `Some` boxed where it may look like `None` (ADR 0051).
    fn dropping_pick(&mut self, helper: &'static str, it: Expr, which: Expr, item: Ty<'tcx>, span: Span) -> R<Expr> {
        let drop = self.std_drop_function(item, span)?.expect("a destructor");
        let boxed = Expr::bool(self.boxed_payload(item));
        Ok(Expr::call_helper(helper, vec![it, which, drop, boxed]))
    }

    /// `last()` of `it`, of items of type `item`: of ones with a destructor,
    /// `$lastDropping`, each before the last dropped as the next comes, as
    /// std's are (ADR 0098); else the last of its array.
    fn last_value(&mut self, it: Expr, item: Option<Ty<'tcx>>, span: Span, made: &mut Vec<Stmt>) -> R<Expr> {
        match item
            .map(|item| self.std_drop_function(item, span))
            .transpose()?
            .flatten()
        {
            Some(drop) => Ok(Expr::call_helper("$lastDropping", vec![it, drop])),
            None => {
                let items = Expr::call(Expr::member(it, "toArray"), Vec::new());
                Ok(self.last_of(items, item, made))
            }
        }
    }

    /// An item with a destructor a method may leave undropped, `take`'s or
    /// `count`'s: what std drops, refused till it's dropped here. Not of one
    /// that drops or keeps each, or moves each on, `filter_map`'s and
    /// `find_map`'s to its function, nor `filter`'s and `skip_while`'s, which drop
    /// each they discard; an adapter, which holds its source closed to drop
    /// it, is `dropping_source`'s (ADRs 0422, 0465).
    fn check_leaves_none(&self, method: Option<IterMethod>, name: &str, span: Span) -> R<()> {
        let keeps_each = matches!(
            method,
            Some(
                IterMethod::Map
                    | IterMethod::ForEach
                    | IterMethod::Fold
                    | IterMethod::TryFold
                    | IterMethod::Collect
                    | IterMethod::Any
                    | IterMethod::All
                    | IterMethod::Position
                    | IterMethod::RPosition
                    | IterMethod::Enumerate
                    | IterMethod::ByRef
                    | IterMethod::Fuse
                    | IterMethod::Sum
                    | IterMethod::Product
                    | IterMethod::Count
                    | IterMethod::Max
                    | IterMethod::Min
                    | IterMethod::Eq
                    | IterMethod::Ne
                    | IterMethod::Last
                    | IterMethod::Nth
                    | IterMethod::Find
                    | IterMethod::Partition
                    | IterMethod::Filter
                    | IterMethod::SkipWhile
            )
        );
        if keeps_each || crate::lower::recognition::leaves_none(name) {
            return Ok(());
        }
        Err(self.unsupported(span, &format!("`{name}` of items with a destructor, from its MIR")))
    }

    /// `value`, an iterator of type `ty`, as a JS iterator: a range stepped
    /// by its `next`, the crate's by its impl's, a JS one as it is.
    pub(in crate::lower) fn as_js_iterator(&mut self, value: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        // The crate's own behind an `impl Iterator` too.
        let ty = self.reveal(ty);
        match self.range_kind(ty) {
            Some(kind) => match kind.step(false) {
                Some(step) => Ok(self.stepped_by(value, Expr::helper(step))),
                None => Err(self.unsupported(span, "iterating this range, from its MIR")),
            },
            None if self.is_user_iterator(ty) => {
                let trait_id = std_item(self.tcx, StdItem::Iterator);
                self.user_iterator(value, ty, trait_id, "next", span)
            }
            // One given its dictionary, stepped by it, its `Some` unboxed
            // (ADR 0411).
            None if let Some(dictionary) = self.iterator_evidence(ty) => {
                let boxed = self.iterator_item(ty).is_none_or(|item| self.boxed_payload(item));
                Ok(crate::lower::iterators::stepped(
                    value,
                    Expr::member(dictionary, "next"),
                    boxed,
                    None,
                ))
            }
            // A type parameter's, which may be given an array (ADR 0061): JS's
            // iterator of it. An associated type's is what a dictionary
            // gives, a JS iterator already.
            None if matches!(ty.kind(), ty::Param(_)) && !is_js_iterator(&value) => Ok(self.js_iterator(value)),
            None => Ok(value),
        }
    }

    /// How many items a `$iter` of an array has left, its `len()`: of one
    /// made here, `$iter(v)`, which nothing has stepped, its array's length,
    /// `v.length`; else `it.end - it.at`, `it` held if it's made here.
    fn items_left(&mut self, it: Expr, made: &mut Vec<Stmt>) -> Expr {
        if crate::lower::iterators::is_stepped_items(&it)
            && let js::ExprKind::Call(_, items) = &it.kind
        {
            return Expr::member(items[0].clone(), "length");
        }
        let it = match it.reads_same() {
            true => it,
            false => self.spill("it", it, made),
        };
        Expr::bin(Op::Sub, Expr::member(it.clone(), "end"), Expr::member(it, "at"))
    }

    /// Whether std's iterator `ty` is a `$iter` of an array, as
    /// `std_iterator` makes it: an array's own, a string's `chars()`,
    /// `bytes()` or `split`, a `VecDeque`'s, a slice's `iter_mut()`, a
    /// `BTreeMap`'s, its `range()`'s too: one that steps from its back and
    /// knows its length. The one list.
    pub(in crate::lower) fn steps_an_array(&self, ty: Ty<'tcx>) -> bool {
        self.array_source(ty).is_some()
            || self.recognition().is_str_chars(ty)
            || self.is_str_split(ty)
            || self.recognition().is_deque_iter(ty)
            || self.recognition().is_slice_iter_mut(ty)
            || self.recognition().is_tree_iter(ty)
            || self.recognition().is_process_items(ty)
    }

    /// A std iterator of type `ty` made of `value`, an array or a text, as
    /// MIR steps it: over an array, a `$iter`, which knows where it is, so a
    /// clone of it does (ADR 0181); another, the JS iterator of `value`.
    pub(in crate::lower) fn std_iterator(&mut self, value: Expr, ty: Ty<'tcx>) -> Expr {
        if is_js_iterator(&value) {
            value
        } else if made_array(&value) || self.steps_an_array(ty) {
            // An array made of it, `s.split(",")`'s, or one `steps_an_array`
            // lists, a `VecDeque`'s say: a `$iter` of it, as of an array's.
            self.stepped_items(value)
        } else {
            self.js_iterator(value)
        }
    }

    /// `$iterator({ start: a, end: b }, $rangeNext).map(f)`, a range of
    /// numbers `iter_ty` steps, each item mapped, all collected: its array's,
    /// `$range(a, b).map(f)`, made as big as what's collected
    /// anyway; what more than one map does in Rust's order, one item through
    /// them all at a time, only lazily. Not filtered, which may keep few of
    /// many, `$lazyRange(a, b).filter(f)` (`range_items`). Of `a..=b`,
    /// `$range(a, b + 1)`; stepped from its end, `.rev()`, `.toReversed()`
    /// first.
    fn mapped_range(&mut self, it: &Expr, iter_ty: Ty<'tcx>) -> Option<Expr> {
        let mut maps = Vec::new();
        let mut at = it;
        while let js::ExprKind::Call(callee, args) = &at.kind
            && let js::ExprKind::Member(source, name) = &callee.kind
            && name == "map"
            && let [f] = &args[..]
        {
            maps.push((name.clone(), f.clone()));
            at = source;
        }
        let range = stepped_range(at)?;
        let numbers = (iter_ty.walk())
            .filter_map(|arg| arg.as_type())
            .find(|&t| matches!(self.range_kind(t), Some(RangeKind::Exclusive | RangeKind::Inclusive)))
            .and_then(|t| self.range_index(t))
            .and_then(Num::of)
            .is_some_and(|num| !num.big());
        if !numbers || maps.iter().filter(|(_, f)| crate::lower::iterators::does(f)).count() > 1 && maps.len() > 1 {
            return None;
        }
        let end = match range.inclusive {
            true => match range.end.as_int() {
                Some(n) => Expr::int(n + 1),
                None => Expr::bin(js::Op::Add, range.end.clone(), Expr::int(1)),
            },
            false => range.end.clone(),
        };
        let mut items = Expr::call_helper("$range", vec![range.start.clone(), end]);
        if range.back {
            items = Expr::call(Expr::member(items, "toReversed"), Vec::new());
        }
        Some(maps.into_iter().rev().fold(items, |items, (name, f)| {
            Expr::call(Expr::member(items, &name), vec![f])
        }))
    }

    /// Of `Iterator.from(list)` collected, `it`, each item mapped or
    /// filtered on the way: the array it makes, `Array.from(nodes).map(f)`
    /// of a binding's list. Not of a generic
    /// value's, which may be an iterator with only a `next`, of which
    /// `Array.from` makes an empty array (ADR 0140).
    fn iterable_items(&self, it: &Expr, iter_ty: Ty<'tcx>) -> Option<Expr> {
        let mut source = self.reveal(iter_ty);
        while let ty::Adt(_, args) = source.kind()
            && let Some(inner) = args.types().next()
            && self.implements_iterator(inner)
        {
            source = inner;
        }
        if matches!(source.kind(), ty::Param(_) | ty::Alias(..)) {
            return None;
        }
        collected(it)
    }

    /// Whether `ty` is an iterator: one whose `Item` rustc says, a type or
    /// another trait's associated type, smallvec's `A::Item`, not one of
    /// std's iterator traits' still.
    pub(in crate::lower) fn implements_iterator(&self, ty: Ty<'tcx>) -> bool {
        let iterator_traits = [StdItem::Iterator, StdItem::IntoIterator].map(|item| std_item(self.tcx, item));
        let said = |item: Ty<'tcx>| match item.kind() {
            ty::Alias(
                _,
                ty::AliasTy {
                    kind: ty::Projection { def_id },
                    ..
                },
            ) => self
                .tcx
                .trait_of_assoc(*def_id)
                .is_some_and(|tr| !iterator_traits.contains(&tr)),
            ty::Alias(..) => false,
            _ => true,
        };
        self.iterator_item(ty).is_some_and(said) || matches!(ty.kind(), ty::Param(_))
    }

    /// A function JS's helpers give an item, and its index too, as one
    /// that takes the item alone: an arrow of one parameter as it is,
    /// anything else called with the item alone.
    fn unary_callback(&mut self, f: Expr, own: bool) -> Expr {
        match &f.kind {
            js::ExprKind::Arrow(params, _) if params.len() <= 1 => f,
            js::ExprKind::Var(_) if own => f,
            _ => {
                let x = self.fresh("item");
                let call = Expr::call(f, vec![Expr::var(&x)]);
                Expr::arrow(vec![x.into()], vec![StmtKind::Return(Some(call)).at(js::Span::NONE)])
            }
        }
    }

    /// Is `ty` a function item of the crate's own, not a JS binding's or a
    /// trait's method, whose JS is called with what Rust gives it alone?
    fn is_crate_fn_item(&self, ty: Ty<'tcx>) -> bool {
        match *ty.kind() {
            ty::FnDef(def_id, _) => {
                def_id.is_local()
                    && self.tcx.trait_of_assoc(def_id).is_none()
                    && !bindings::is_binding(self.tcx, def_id)
            }
            _ => false,
        }
    }

    /// As `unary_callback`, of a function of two: `reduce`'s, given an index
    /// too. `f64::max`'s and `f64::min`'s helpers take two alone.
    fn binary_callback(&mut self, f: Expr, own: bool) -> Expr {
        match &f.kind {
            js::ExprKind::Arrow(params, _) if params.len() <= 2 => f,
            js::ExprKind::Var(_) if own => f,
            _ if matches!(f.helper_name(), Some("$f64Max" | "$f64Min")) => f,
            _ => {
                let (a, b) = (self.fresh("a"), self.fresh("b"));
                let call = Expr::call(f, vec![Expr::var(&a), Expr::var(&b)]);
                Expr::arrow(
                    vec![a.into(), b.into()],
                    vec![StmtKind::Return(Some(call)).at(js::Span::NONE)],
                )
            }
        }
    }
}

/// `counted_chain`'s `$iter(xs).drop(1).take(2)`: the array it steps,
/// `xs.slice(1).slice(0, 2)`.
fn sliced(e: &Expr) -> Expr {
    match &e.kind {
        js::ExprKind::Call(callee, args)
            if let js::ExprKind::Member(source, name) = &callee.kind
                && let [n] = &args[..] =>
        {
            let items = sliced(source);
            match name.as_str() {
                "drop" => Expr::call(Expr::member(items, "slice"), vec![n.clone()]),
                _ => Expr::call(Expr::member(items, "slice"), vec![Expr::int(0), n.clone()]),
            }
        }
        _ => crate::lower::iterators::iterated(e.clone()),
    }
}

/// Whether `e` is an array's items made there, `$iter(xs)`, skipped or taken
/// from, `.drop(1)`, by a count that does nothing (`len()`'s).
fn counted_chain(e: &Expr) -> bool {
    let mut at = e;
    let mut stages = 0;
    while let js::ExprKind::Call(callee, args) = &at.kind
        && let js::ExprKind::Member(source, name) = &callee.kind
        && matches!(name.as_str(), "drop" | "take")
        && let [n] = &args[..]
        && !n.has_effects()
    {
        stages += 1;
        at = source;
    }
    stages > 0 && crate::lower::iterators::is_stepped_items(at)
}

/// `$iter(v).map(f).filter(p)`, a lazy chain over an array's items whose
/// callbacks read no index, made of the items reversed:
/// `$iter(v.toReversed()).map(f).filter(p)`. Rust's `Map` and `Filter`
/// call theirs from the back as they're stepped from it, so each is called
/// on the same items, in the same order (ADR 0164). An index, an
/// `enumerate`'s `(x, i) => [i, x]`, would count from the other end, so
/// it's made of the array's items first, where nothing's called before.
fn reversed_inward(e: &Expr) -> Option<Expr> {
    let js::ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    // `$iter(v)` of an array just made iterable: `$iter(v.toReversed())`.
    if crate::lower::iterators::is_stepped_items(e) {
        let items = crate::lower::iterators::iterated(e.clone());
        return Some(Expr::call(
            (**callee).clone(),
            vec![Expr::call(Expr::member(items, "toReversed"), Vec::new())],
        ));
    }
    // A range made here, stepped from its start, `$iterator({ start, end },
    // $rangeNext)`: stepped from its end, `$rangeNextBack`, as Rust's range
    // is by `next_back`.
    if let Some(range) = crate::lower::iterators::stepped_range(e)
        && !range.back
    {
        let kind = match range.inclusive {
            true => crate::lower::std_types::range::RangeKind::Inclusive,
            false => crate::lower::std_types::range::RangeKind::Exclusive,
        };
        let back = Expr::helper(kind.step(true)?);
        return Some(Expr::call((**callee).clone(), vec![args[0].clone(), back]));
    }
    // `Iterator.from(x)` of what JS steps, a string's `$charIndices(s)`: its
    // items, listed and reversed, as `Array.from(x)` lists a whole chain's,
    // stepped as an array's are, `$iter(Array.from(x).toReversed())`.
    if let Ok(source) = crate::lower::iterators::from_iterator(e.clone()) {
        let items = Expr::call(Expr::member(Expr::var("Array"), "from"), vec![source]);
        return Some(crate::lower::iterators::array_stepped(Expr::call(
            Expr::member(items, "toReversed"),
            Vec::new(),
        )));
    }
    let js::ExprKind::Member(base, method) = &callee.kind else {
        return None;
    };
    // A function given as it is, a Rust one of the item alone, reads no index.
    let no_index = |f: &Expr| match &f.kind {
        js::ExprKind::Arrow(params, _) => params.len() == 1,
        js::ExprKind::Var(_) | js::ExprKind::Member(..) => true,
        _ => false,
    };
    match (method.as_str(), args.as_slice()) {
        // An index read of an array's items, nothing before it to call: the
        // array mapped, then reversed, `$iter(v.map((x, i) => [i, x])
        // .toReversed())`, each index its item's.
        ("map", [f]) if !no_index(f) && crate::lower::iterators::is_stepped_items(base) => {
            let js::ExprKind::Call(stepper, _) = &base.kind else {
                return None;
            };
            let items = Expr::call(
                Expr::member(crate::lower::iterators::iterated((**base).clone()), "map"),
                args.clone(),
            );
            Some(Expr::call(
                (**stepper).clone(),
                vec![Expr::call(Expr::member(items, "toReversed"), Vec::new())],
            ))
        }
        ("map" | "filter", [f]) if no_index(f) => {
            Some(Expr::call(Expr::member(reversed_inward(base)?, method), args.clone()))
        }
        _ => None,
    }
}
