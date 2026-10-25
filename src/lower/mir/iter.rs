//! Iterators from MIR (ADR 0364): each a JS iterator, whose helpers are
//! lazy as Rust's adapters are, so what each closure does happens in the
//! order Rust's does, item by item. A range is its `{ start, end }` until
//! it's iterated, and an iterator of the crate's its impl's `next`, made a
//! JS iterator where one's wanted. What's taken by a `&mut`, `any` of
//! `&mut it`, steps it through `$lent`, which a helper that stops early
//! closes in its place, as Rust leaves the iterator to go on.

mod backs;
pub(in crate::lower::mir) use backs::reversed_in_place;
mod closing;
mod collect;
pub(in crate::lower::mir) mod sources;

use self::sources::*;

use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, Mutability, Ty, TypeVisitableExt};
use rustc_span::Span;
use rustc_span::def_id::DefId;

use crate::js::{self, Expr, Op, Stmt, StmtKind};
use crate::lower::iterators::{collected, from_iterator, is_js_iterator, made_array, stepped_range};
use crate::lower::recognition::{IterMethod, Parted, StdItem, is_std_def, std_item};
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
        values: Vec<Expr>,
        output: Ty<'tcx>,
        span: Span,
        made: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let mut values = self.marked_in_place(values, arg_tys, span);
        let Some(trait_id) = self.tcx.trait_of_assoc(def_id) else {
            return Ok(None);
        };
        let receiver_ty = arg_tys.first().copied().unwrap_or(self.tcx.types.unit);
        if !receiver_ty.is_ref()
            && is_std_def(self.tcx, trait_id, StdItem::Iterator)
            && self.dropped_after(def_id, receiver_ty, output, !values.is_empty(), span)?
        {
            let held = self.spill_marked(values.remove(0), made);
            values.insert(0, held.clone());
            let Some(result) = self.mir_iter_method(def_id, (trait_id, arg_tys), values, output, span, made)? else {
                return Ok(None);
            };
            let result = self.spill("result", result, made);
            self.dropped_consumed(held, (receiver_ty, def_id, output), span, made)?;
            return Ok(Some(result));
        }
        self.cloned_apart_call(def_id, (trait_id, arg_tys), values, output, span, made)
    }

    /// `size_hint()` of `receiver`, an iterator of type `iter_ty` neither the
    /// crate's own nor a range: of a `$iter`, exact, or of a generic one, as
    /// `$sizeHint` finds it (ADR 0170), by its dictionary where it's given
    /// one, which has its impl's own (ADR 0561).
    fn size_hint_of(&mut self, receiver: Expr, iter_ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let mut it = receiver;
        // A `map`'s is its source's, as Rust's `Map` passes it on; its
        // closure, unrun, is made for nothing. Its source's type, peeled
        // as each `map` is, where each is a `Map`.
        let mut mapped = false;
        let mut source_ty = Some(iter_ty.peel_refs());
        while let js::ExprKind::Call(callee, args) = &it.kind
            && let js::ExprKind::Member(source, method) = &callee.kind
            && method == "map"
            && args.iter().all(|f| !f.has_effects())
        {
            it = (**source).clone();
            mapped = true;
            source_ty = source_ty.and_then(|ty| crate::lower::recognition::mapped_source(self.tcx, ty));
        }
        // Of the crate's own, its `$iterator`'s, which has its impl's
        // `size_hint` where it writes one, else std's `(0, None)`.
        let known = match mapped {
            true => {
                crate::lower::iterators::is_stepped_items(&it)
                    || source_ty.is_some_and(|ty| self.is_user_iterator(ty) || self.maps_the_crates_own(ty))
            }
            false => self.hints_its_size(iter_ty),
        };
        if !known {
            return Err(self.unsupported(span, &format!("`size_hint()` of a `{iter_ty}`, from its MIR")));
        }
        // Of one given as it is, by its dictionary, which has its impl's
        // own where there's one (ADR 0411).
        let mut args = vec![it];
        if !mapped {
            let iterator = std_item(self.tcx, StdItem::Iterator);
            args.extend(self.iterator_evidence(iter_ty.peel_refs(), iterator));
        }
        Ok(Expr::call_helper("$sizeHint", args))
    }

    /// `mir_iter_call`'s, of a `map` or a `filter` cloned apart (ADR 0525):
    /// made by a function of its source and its function, which its clone
    /// calls again, of a source generic code is given too.
    fn cloned_apart_call(
        &mut self,
        def_id: DefId,
        (trait_id, arg_tys): (DefId, &[Ty<'tcx>]),
        values: Vec<Expr>,
        output: Ty<'tcx>,
        span: Span,
        made: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let parted = crate::lower::recognition::cloned_parts(self.tcx, self.reveal(output));
        let (Some((parted, _, stored)), [receiver_ty, other_ty, ..]) = (parted, arg_tys) else {
            return self.holding_function(def_id, (trait_id, arg_tys), values, output, span, made);
        };
        if values.len() != 2 || !self.iterator_cloned_apart(output) {
            return self.holding_function(def_id, (trait_id, arg_tys), values, output, span, made);
        }
        let (receiver_ty, other_ty) = (*receiver_ty, *other_ty);
        let name = self.tcx.item_name(def_id);
        let mut values = values.into_iter();
        let (receiver, other) = (
            values.next().expect("the receiver"),
            values.next().expect("its other part"),
        );
        // A chain's or a zip's other side given as what makes an iterator,
        // a `Vec`, an array or a slice: made the array's iterator here, and
        // cloned as the iterator the adapter keeps, where it is (#181). One
        // given as an iterator is as the receiver is, its JS value where it
        // is; any other, refused.
        let iterates = matches!(parted, Parted::Map | Parted::Filter) || self.implements_iterator(other_ty);
        let (other, other_ty) = match iterates {
            true => (other, other_ty),
            false if self.array_source(stored).is_some() => {
                let items = self.iterable_items(other, other_ty, span, made)?;
                (self.stepped_items(items), stored)
            }
            false => {
                return Err(self.unsupported(
                    span,
                    &format!("cloning a `{output}` of a `{other_ty}` it makes an iterator of"),
                ));
            }
        };
        let parts = [receiver, other];
        let lent = matches!(receiver_ty.kind(), ty::Ref(_, _, Mutability::Mut));
        let names = match parted {
            Parted::Map | Parted::Filter => ["it", "f"],
            Parted::Chain | Parted::Zip => ["a", "b"],
        };
        let own = self.is_called_as_written(other_ty);
        let adapter = self.cloned_apart_adapter(
            (name.as_str(), parts, names),
            ([receiver_ty, other_ty], output),
            lent,
            span,
            made,
            |cx, a, b| {
                let a = cx.as_js_iterator(a, receiver_ty, span)?;
                Ok(match parted {
                    // Its source given its function, `it.map(f)`.
                    Parted::Map | Parted::Filter => {
                        let callback = cx.unary_callback(b, own);
                        Expr::call(Expr::member(a, name.as_str()), vec![callback])
                    }
                    Parted::Chain => Expr::call_helper("$lazyChain", vec![a, cx.as_js_iterator(b, other_ty, span)?]),
                    Parted::Zip => Expr::call_helper("$lazyZip", vec![a, cx.as_js_iterator(b, other_ty, span)?]),
                })
            },
        )?;
        Ok(Some(adapter))
    }

    /// `mir_iter_method`'s, of an adapter that holds a function with a
    /// destructor, `map(f)`'s, made closing: `$closing(it, (it) => {
    /// it.return(); .. })`, which closes its source, then drops its function,
    /// as Rust drops a `Map`'s fields (ADR 0485).
    fn holding_function(
        &mut self,
        def_id: DefId,
        (trait_id, arg_tys): (DefId, &[Ty<'tcx>]),
        mut values: Vec<Expr>,
        output: Ty<'tcx>,
        span: Span,
        made: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let function = (self.recognition().held_function(output))
            .filter(|&function| self.drops(function) != crate::lower::drops::Drops::Nothing);
        let (Some(function), Some(f)) = (function, values.get(1).cloned()) else {
            return self.mir_iter_method(def_id, (trait_id, arg_tys), values, output, span, made);
        };
        let f = match f.kind {
            js::ExprKind::Var(_) => f,
            _ => self.spill("f", f, made),
        };
        values[1] = f.clone();
        let Some(adapter) = self.mir_iter_method(def_id, (trait_id, arg_tys), values, output, span, made)? else {
            return Ok(None);
        };
        let it = Expr::var("it");
        let mut body = vec![StmtKind::Expr(Expr::call(Expr::member(it, "return"), Vec::new())).at(js::Span::NONE)];
        self.drop_value(f, function, span, &mut body)?;
        Ok(Some(Expr::call_helper(
            "$closing",
            vec![adapter, Expr::arrow(vec!["it".into()], body)],
        )))
    }

    /// `held`, an iterator of type `receiver` its std method `method` took
    /// by value, dropped after it, as `dropped_after` says: of a type
    /// parameter given as it is, consumed by `fold`, where its dictionary
    /// has the crate's own, which took it and dropped it, not again (#162).
    fn dropped_consumed(
        &mut self,
        held: Expr,
        (receiver, method, output): (Ty<'tcx>, DefId, Ty<'tcx>),
        span: Span,
        made: &mut Vec<Stmt>,
    ) -> R<()> {
        let iterator = std_item(self.tcx, StdItem::Iterator);
        let Some(dictionary) =
            (self.iterator_evidence(receiver, iterator)).filter(|_| self.consumes_by_fold(method, output))
        else {
            return self.drop_value(held, receiver, span, made);
        };
        let mut dropped = Vec::new();
        self.drop_value(held, receiver, span, &mut dropped)?;
        if !dropped.is_empty() {
            let fold = crate::lower::bindings::trait_method_name(self.tcx, iterator, "fold");
            let unfolded = Expr::bin(Op::Eq, Expr::member(dictionary, fold), Expr::undefined());
            made.push(StmtKind::If(unfolded, dropped, None).at(self.js_span(span)));
        }
        Ok(())
    }

    /// `values`, of `arg_tys`, that the call at `span` takes, each a `Rev`,
    /// an `Enumerate` or a `Map` made in place, a variable's too, marked so
    /// (`made_in_place`): what takes them apart past this call asks it of
    /// them, never std's compiled back (#257). The one answer, where each
    /// std iterator method's values are made.
    pub(in crate::lower) fn marked_in_place(&self, values: Vec<Expr>, arg_tys: &[Ty<'tcx>], span: Span) -> Vec<Expr> {
        let in_place = |ty: Ty<'tcx>| {
            let ty = ty.peel_refs();
            crate::lower::recognition::is_held_compiled(self.tcx, ty) && !self.compiled_struct_at(ty, span)
        };
        (values.into_iter().enumerate())
            .map(|(i, value)| match arg_tys.get(i).is_some_and(|&ty| in_place(ty)) {
                true => value.in_place(),
                false => value,
            })
            .collect()
    }

    /// `value` spilled, `const iter = value;`, its mark kept on the name
    /// that reads it (`made_in_place`).
    fn spill_marked(&mut self, value: Expr, made: &mut Vec<Stmt>) -> Expr {
        let in_place = value.facts.has(js::Known::InPlace);
        let held = self.spill("iter", value, made);
        match in_place {
            true => held.in_place(),
            false => held,
        }
    }

    /// Whether a value of `ty` the call at `span` takes is std's compiled
    /// struct, a compiled std iterator's (ADR 0561): not a `Rev`, an
    /// `Enumerate` or a `Map` it steps where it's made, which is what the recognizers
    /// make of it, a JS iterator that closes as it ends (ADR 0871).
    pub(in crate::lower) fn compiled_struct_at(&self, ty: Ty<'tcx>, span: Span) -> bool {
        crate::lower::recognition::is_compiled_std_iterator(self.tcx, ty)
            && !(crate::lower::recognition::is_held_compiled(self.tcx, ty)
                && self.frame.facts.in_place_adapter_steps.contains(&span))
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
        // What the crate's own `fold` is called by: it takes the iterator
        // and drops it, so nothing's dropped after (#162). Not of an adapter
        // that holds a function with a destructor, which it doesn't drop.
        if self.consumes_by_fold(def_id, output) && self.recognition().folds_its_own(receiver) {
            if (self.recognition().held_function(receiver))
                .is_some_and(|function| self.drops(function) != crate::lower::drops::Drops::Nothing)
            {
                return Err(self.unsupported(
                    span,
                    "an adapter holding a closure with a destructor, over an iterator whose impl writes its own `fold`",
                ));
            }
            return Ok(false);
        }
        let params_only = self.drop_query().drops_but_params(receiver) == crate::lower::drops::Drops::Nothing;
        // And an adapter that holds a function with a destructor, `map(f)`'s,
        // dropped with it (ADR 0485).
        let holds_function = (self.recognition().held_function(receiver))
            .is_some_and(|function| self.drops(function) != crate::lower::drops::Drops::Nothing);
        // Or an adapter closed to drop what it holds, `take(n)`'s or
        // `cycle()`'s, made of one given its drop, `$closing` as
        // `closing_drop` makes it (ADR 0491).
        let closed = (self.recognition().closed_on_drop(receiver)).is_some_and(|held| self.given_drop(held).is_some());
        // A compiled std iterator's struct, `Peekable`'s, is dropped as a
        // struct is, by its fields' drops, given where it holds only type
        // parameters' values (ADR 0561).
        let compiled = self.compiled_struct_at(receiver, span);
        let dropped = match params_only {
            true => {
                self.given_drop(receiver).is_some()
                    || (compiled && self.holds_given_drops(receiver))
                    || holds_function
                    || closed
            }
            // Not a `copied()` of one closed to drop what it holds, which is
            // it (ADR 0547): consumed to its end, it's closed as it's done
            // (ADR 0570).
            false => self.is_user_iterator(receiver) || compiled || holds_function,
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

    /// Whether an adapter of type `ty` is its source itself in JS: a
    /// `copied()`, or a `cloned()` of what needs no clone, each item as it
    /// is (ADR 0547). The one answer, of what's made and what's dropped.
    fn is_its_source(&self, ty: Ty<'tcx>) -> bool {
        // Behind the `impl Iterator` it's given back as too.
        let ty = self.reveal(ty);
        let item = || {
            self.iterator_item(ty)
                .is_some_and(|item| !self.needs_clone(item.peel_refs()))
        };
        // A copy of a `Copy` item is a clone of it too, a struct's an object
        // of its own where something changes one in place.
        (crate::lower::recognition::copied_source(self.tcx, ty).is_some()
            || crate::lower::recognition::cloned_source(self.tcx, ty).is_some())
            && item()
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
        if std_method == Some(IterMethod::IntoIter) && !self.holds_into_iter(receiver_ty) {
            let value = values.into_iter().next().expect("the receiver");
            return Ok(self.value_into_iter(value, receiver_ty, output));
        }
        // A range's own `nth`, `last` and `count`, of its bounds (#163).
        if let Some(stepped) = self.range_own_step(std_method, iter_ty, &values, span) {
            return Ok(Some(stepped));
        }
        // `B::from_iter(items)`, which `items.collect()` is (ADR 0160).
        if std_method == Some(IterMethod::FromIter) {
            return self.collected_from(values, receiver_ty, output, span);
        }
        // `nth_back(n)`, `rfold(init, f)` and `rfind(p)`: of what knows its
        // back, stepped from it, a generic one's too (ADRs 0429, 0455).
        if let Some(method) = std_method.filter(|method| method.steps_from_its_back()) {
            return self.back_step(method, values, (iter_ty, output), span).map(Some);
        }
        // `next_back(&mut it)` and `len(&it)`: of what knows its back, a
        // range, a `$iter` or one held (ADRs 0071, 0549); not a lazy one.
        if matches!(std_method, Some(IterMethod::NextBack | IterMethod::Len)) {
            let it = values.into_iter().next().expect("the receiver");
            // The crate's iterator's `len()`, std's: its `size_hint()`, exact.
            // One its impl writes itself is its own, never asked here (ADR
            // 0492). A compiled std iterator's whose `len` isn't compiled,
            // `Peekable`'s, is its struct's hint (ADR 0561), where it's
            // std's struct, not one made in place (ADR 0871).
            if (self.is_user_iterator(iter_ty) || self.compiled_struct_at(iter_ty, span))
                && std_method == Some(IterMethod::Len)
            {
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
        // `rev()`: of the crate's iterator, `reversed_user`'s; of a range,
        // stepped from its end; of items made already, an array's, a `$iter`
        // of them; not of a lazy one, whose closures would run in another
        // order (ADR 0164). What it steps is closed to drop it (#225).
        if std_method == Some(IterMethod::Rev) {
            let receiver = values.into_iter().next().expect("the receiver");
            let unsupported = |cx: &Self| cx.unsupported(span, &format!("`{name}` of a `{iter_ty}`, from its MIR"));
            if self.is_user_iterator(iter_ty) {
                return self.reversed_user(receiver, receiver_ty, span).map(Some);
            }
            let receiver = self.closing(receiver, receiver_ty, span)?;
            return Ok(Some(match self.range_kind(iter_ty) {
                None if let Some(reversed) = self.reversed(&receiver, iter_ty) => reversed,
                Some(kind) if let Some(step) = kind.step(true) => self.stepped_by(receiver, Expr::helper(step)),
                Some(_) => return Err(unsupported(self)),
                None => self.rev_items(receiver, iter_ty, span)?,
            }));
        }
        // `size_hint()` of a `$iter`, exact, or of a generic one, as `$sizeHint`
        // finds it (ADR 0170); a range's is its bounds' (`RangeOp::SizeHint`).
        if std_method == Some(IterMethod::SizeHint)
            && !self.is_user_iterator(iter_ty)
            && self.range_kind(iter_ty.peel_refs()).is_none()
        {
            let receiver = values.into_iter().next().expect("the receiver");
            return self.size_hint_of(receiver, iter_ty, span).map(Some);
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
        // Only std's iterator methods, `iterator_method`'s; of the crate's
        // iterator, what its impl takes from `Iterator` is std's of a JS
        // iterator its `next` steps, and what it writes itself never comes
        // here (ADR 0492).
        if std_method.is_none() {
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
            matches!(receiver_ty.kind(), ty::Ref(_, _, Mutability::Mut)) && !std_method.is_some_and(IterMethod::tries);
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
        let own = arg_tys.get(1).is_some_and(|ty| self.is_called_as_written(*ty));
        let method = |name: &str, it: Expr, args: Vec<Expr>| Expr::call(Expr::member(it, name), args);
        Ok(Some(match std_method {
            // By what gives each as it is, `map(StaticPath::Params)`: no step,
            // but where a `Map` hides the crate's own steps (#235).
            Some(IterMethod::Map) => match self.unary_callback(arg(), own) {
                f if f.is_identity() && !self.recognition().reaches_own_steps(iter_ty) => it,
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
            Some(IterMethod::Enumerate) => crate::lower::iterators::enumerated(it),
            Some(m) if m.compares() => self.iterators_cmp((it, arg()), (item, arg_tys[1]), m, span)?,
            Some(IterMethod::Fold) => self.folded(it, (Some(arg()), arg()), (arg_tys, item)),
            Some(IterMethod::Reduce) => self.folded(it, (None, arg()), (arg_tys, item)),
            Some(m) if m.tries() => self.try_folded(it, m, &mut arg, (own, output), span)?,
            Some(IterMethod::Count) => self.counted(it, item, (def_id, iter_ty), span)?,
            Some(IterMethod::Sum | IterMethod::Product) => {
                let sum = std_method == Some(IterMethod::Sum);
                self.accumulated((it, def_id), (output, item), sum, span)?
            }
            Some(IterMethod::Copied | IterMethod::Cloned) if self.is_its_source(output) => it,
            Some(IterMethod::Copied | IterMethod::Cloned) => match item {
                Some(item) => self.copied_items(it, item, span)?,
                None => return Ok(None),
            },
            // Of its items as they come, which the helper steps (#179).
            Some(IterMethod::Max | IterMethod::Min) => {
                self.extreme_of(std_method == Some(IterMethod::Max), it, item, span)?
            }
            Some(IterMethod::Last) => self.last_called(it, iter_ty, item, span, made)?,
            Some(IterMethod::IsSorted) => match item {
                Some(item) => self.is_sorted_call(it, item, span)?,
                None => return Ok(None),
            },
            Some(IterMethod::Nth) => match dropping {
                Some(item) => self.dropping_pick("$nthDropping", it, arg(), item, span)?,
                None => return Ok(None),
            },
            Some(IterMethod::ByRef) => it,
            Some(IterMethod::Collect) => self.collected_into(it, output, item, Some(iter_ty), span)?,
            _ => return Ok(None),
        }))
    }

    /// `it.count()`, of `def_id` of an iterator of type `iter_ty`: each item
    /// counted, and dropped as `count` drops it; of a string's UTF-16 units,
    /// its `length` (ADR 0632).
    /// `copied()` or `cloned()` of an iterator of `item`s, references: a
    /// copy of what each points at, once. Of an item that's a reference
    /// to a reference, `&&Fiber`, the reference, which a copy is itself,
    /// not what it points at, so the items as they are.
    fn copied_items(&mut self, it: Expr, item: Ty<'tcx>, span: Span) -> R<Expr> {
        let x = self.fresh("item");
        let mut body = Vec::new();
        let pointee = match *item.kind() {
            ty::Ref(_, pointee, _) => pointee,
            _ => item,
        };
        let copy = self.clone_value(Expr::var(&x), pointee, span, &mut body)?;
        if body.is_empty() && matches!(&copy.kind, js::ExprKind::Var(v) if *v == x) {
            return Ok(it);
        }
        body.push(StmtKind::Return(Some(copy)).at(js::Span::NONE));
        Ok(Expr::call(
            Expr::member(it, "map"),
            vec![Expr::arrow(vec![x.into()], body)],
        ))
    }

    fn counted(
        &mut self,
        it: Expr,
        item: Option<Ty<'tcx>>,
        (def_id, iter_ty): (DefId, Ty<'tcx>),
        span: Span,
    ) -> R<Expr> {
        if crate::lower::recognition::counts_utf16_units(self.tcx, def_id, iter_ty)
            && let Some(length) = crate::lower::std_types::text::utf16_length(&it)
        {
            return Ok(length);
        }
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
        // A `Take`'s over what reaches the crate's own `nth`: std's `count`,
        // which advances its source and asks no `nth` (#213), `$count`'s; and
        // a `Take`'s or a `Skip`'s whose `nth` drops a reversed `Vec`'s as one
        // slice, as their `count`s do (#261).
        if (crate::lower::recognition::is_take(self.tcx, self.reveal(iter_ty))
            && self.recognition().nths_its_own(iter_ty))
            || (self.recognition().nths_a_slice(iter_ty)
                && self.recognition().reversed(iter_ty).is_none()
                && crate::lower::recognition::stepped_by_source(self.tcx, self.reveal(iter_ty)).is_none())
        {
            let mut given = vec![it];
            if let Some(item) = item
                && let Some(drop) = self.drop_function(item, span)?
            {
                given.push(drop);
            }
            return Ok(Expr::call_helper("$count", given));
        }
        body.push(StmtKind::Return(Some(Expr::bin(Op::Add, Expr::var(&n), Expr::int(1)))).at(js::Span::NONE));
        Ok(Expr::call(
            Expr::member(it, "reduce"),
            vec![Expr::arrow(params, body), Expr::int(0)],
        ))
    }

    /// `x.into_iter()`, of `value` of type `receiver_ty`: a range stays one,
    /// an iterator is itself, and a collection's items a JS iterator; `None`
    /// of what `mir_std_call` makes, `for x in &mut v`'s handles on numbers
    /// (ADR 0099).
    fn value_into_iter(&mut self, value: Expr, receiver_ty: Ty<'tcx>, output: Ty<'tcx>) -> Option<Expr> {
        let iter_ty = receiver_ty.peel_refs();
        if self.recognition().is_given_iterable(receiver_ty) {
            return Some(crate::lower::iterators::given_into_iter(value));
        }
        if self.range_kind(iter_ty).is_some() || self.implements_iterator(receiver_ty) {
            return Some(value);
        }
        let handles = matches!(receiver_ty.kind(), ty::Ref(_, _, Mutability::Mut))
            && self.iterator_item(output).is_some_and(|item| self.is_cell(item));
        let items = iter_ty.is_array() || iter_ty.is_slice() || self.is_vec_like(iter_ty);
        (!handles && items).then(|| self.std_iterator(value, output))
    }

    /// Whether `$sizeHint` finds an `iter_ty`'s size: a `$iter`'s, exact, a
    /// generic one's, as it finds it (ADR 0170), and a compiled std
    /// iterator's, its struct's own hint (ADR 0561).
    fn hints_its_size(&self, iter_ty: Ty<'tcx>) -> bool {
        // One a struct holds, its own of its source's (ADR 0542).
        self.array_source(iter_ty).is_some()
            || self.is_unknown(iter_ty)
            || self.explicit_adapter(iter_ty)
            || self.struct_hint_known(iter_ty)
            || self.maps_the_crates_own(iter_ty)
    }

    /// Whether `iter_ty` is a `map` of the crate's own iterator, or of one
    /// of those: a `$Steps`'s `map`, whose hint is its source's, as std's
    /// `Map` passes it on.
    fn maps_the_crates_own(&self, iter_ty: Ty<'tcx>) -> bool {
        crate::lower::recognition::mapped_source(self.tcx, iter_ty.peel_refs())
            .is_some_and(|source| self.is_user_iterator(source) || self.maps_the_crates_own(source))
    }
}
