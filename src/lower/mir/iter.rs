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
use crate::lower::iterators::was_made_knowing_its_back as knows_back;
use crate::lower::recognition::{OwnStep, Reach};

use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, Mutability, Ty, TypeVisitableExt};
use rustc_span::Span;
use rustc_span::def_id::DefId;

use crate::js::{self, Expr, Op, Stmt, StmtKind};
use crate::lower::iterators::{collected, is_js_iterator, made_array, stepped_range};
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
        let Some(trait_id) = self.tcx.trait_of_assoc(def_id) else {
            return Ok(None);
        };
        // `C::from_iter(items)`, what `items.collect::<C>()` calls, std's
        // `items.into_iter()` collected: those calls (ADR 0160, #838).
        if self.recognition().iterator_method(def_id) == Some(IterMethod::FromIter) {
            let given = arg_tys.first().copied().unwrap_or(self.tcx.types.unit);
            return self.collected_from_iter(given, values, output, span, made);
        }
        self.consumed_or_held(def_id, (trait_id, arg_tys), values, output, span, made)
    }

    /// `mir_iter_call`'s, of a method of `trait_id`'s: one that takes its
    /// iterator by value and drops it as it ends, which JS doesn't, called
    /// on what's held, then dropped, `consumed_call`'s, the one place what
    /// std's code consumes is dropped (#1116); another as it is.
    fn consumed_or_held(
        &mut self,
        def_id: DefId,
        (trait_id, arg_tys): (DefId, &[Ty<'tcx>]),
        mut values: Vec<Expr>,
        output: Ty<'tcx>,
        span: Span,
        made: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let receiver_ty = arg_tys.first().copied().unwrap_or(self.tcx.types.unit);
        if !receiver_ty.is_ref()
            && (is_std_def(self.tcx, trait_id, StdItem::Iterator) || self.recognition().rfolds_what_it_consumes(def_id))
            && self.dropped_after(def_id, (receiver_ty, arg_tys), output, values.first(), span)?
        {
            let held = self.spill("iter", values.remove(0), made);
            values.insert(0, held.clone());
            return self.consumed_call(held, (def_id, trait_id, arg_tys), values, output, span, made);
        }
        self.cloned_apart_call(def_id, (trait_id, arg_tys), values, output, span, made)
    }

    /// `C::from_iter(items)`, `output` of `items` of type `given`: a
    /// sequence's or a map's items as they are; another's as std's
    /// `from_iter` steps them, `items.into_iter()`, of an iterator itself,
    /// then `collect()` of that, each the call it is, so it's dropped where
    /// `collect` drops it (ADR 0160, #838).
    fn collected_from_iter(
        &mut self,
        given: Ty<'tcx>,
        values: Vec<Expr>,
        output: Ty<'tcx>,
        span: Span,
        made: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        if let Some(collected) = self.collected_as_they_are(&values[0], given, output, span)? {
            return Ok(Some(collected));
        }
        let tcx = self.tcx;
        let trait_method = crate::lower::recognition::trait_method;
        let collect = trait_method(tcx, std_item(tcx, StdItem::Iterator), "collect");
        let into_iter = trait_method(tcx, std_item(tcx, StdItem::IntoIterator), "into_iter");
        // An iterator's `into_iter()` is itself.
        if self.iterator_item(given).is_some() {
            return self.mir_iter_call(collect, &[given], values, output, span, made);
        }
        let stepped = crate::lower::recognition::iterator_type_of(tcx, self.typing_env, given);
        let Some(it) = self.mir_iter_call(into_iter, &[given], values, stepped, span, made)? else {
            return Ok(None);
        };
        self.mir_iter_call(collect, &[stepped], vec![it], output, span, made)
    }

    /// `size_hint()` of `receiver`, an iterator of type `iter_ty` neither the
    /// crate's own nor a range: of a `$iter`, exact, or of a generic one, as
    /// `$sizeHint` finds it (ADR 0170), by its dictionary where it's given
    /// one, which has its impl's own (ADR 0561).
    /// `next_back()` of what's made here, by its form: of a map of an
    /// array's items, `v.iter().map(f)`, its source's last item, mapped, as
    /// Rust's `Map::next_back` maps that one alone; of an enumeration,
    /// `src.map((x, i) => [i, x])`, what steps it from its back, `$enumerate`
    /// of its source, its index from the back where it is (ADR 0528, #395).
    /// `None` of another.
    fn next_back_made_here(
        &mut self,
        it: Expr,
        iter_ty: Ty<'tcx>,
        boxed: bool,
        span: Span,
        made: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        if crate::lower::iterators::enumerated_of(&it).is_some() {
            let back = self.as_double_ended(it, iter_ty, span)?;
            let next_back = if boxed { "$nextBackSome" } else { "$nextBack" };
            return Ok(Some(Expr::call_helper(next_back, vec![back])));
        }
        if !boxed
            && crate::lower::iterators::is_items_map(&it)
            && let js::ExprKind::Call(callee, args) = &it.kind
            && let js::ExprKind::Member(source, _) = &callee.kind
        {
            let last = Expr::call_helper("$nextBack", vec![(**source).clone()]);
            let last = self.spill("last", last, made);
            let none = Expr::bin(Op::Eq, last.clone(), Expr::undefined());
            let mapped = crate::lower::calls::apply(args[0].clone(), vec![last]);
            return Ok(Some(Expr::cond(none, Expr::undefined(), mapped)));
        }
        Ok(None)
    }

    fn size_hint_of(&mut self, receiver: Expr, iter_ty: Ty<'tcx>, span: Span, made: &mut Vec<Stmt>) -> R<Expr> {
        // Of a string's `char`s, whose hint is of the bytes it has left,
        // `(bytes.div_ceil(4), Some(bytes))` (str/iter.rs:100), not of how
        // many it has, which its JS counted (#733): std's struct's,
        // compiled, or refused.
        let of_text = (iter_ty.walk().filter_map(|held| held.as_type())).any(|held| {
            let text = self.recognition();
            text.is_str_chars(held) || text.is_char_indices(held) || text.is_string_drain(held)
        });
        if of_text && !self.struct_hint_known(iter_ty) {
            return Err(self.unsupported(span, &format!("`size_hint()` of a `{iter_ty}`, of a string's `char`s")));
        }
        // Of one made here that `$sizeHint` can't ask, a JS helper's, a
        // slice's `skip(1)` say, counted where it's made, `len_in_place`'s:
        // that both ways, `(n, Some(n))`, as Rust's `ExactSizeIterator`
        // hints it (#156). Not an array's items, or a map of them, whose
        // `$iter` knows it.
        let mut source = &receiver;
        while let js::ExprKind::Call(callee, _) = &source.kind
            && let js::ExprKind::Member(of, method) = &callee.kind
            && method == "map"
        {
            source = of;
        }
        if !crate::lower::iterators::is_stepped_items(source)
            && let Some(len) = self.len_in_place(&receiver, iter_ty, made)
        {
            let n = self.spill("n", len, made);
            return Ok(Expr::array(vec![n.clone(), n]));
        }
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
                // Or of a type parameter's, its JS iterator's, `$given`'s, which
                // has its dictionary's `size_hint` (#385), or of std's adapter
                // of one whose hint is std's (`adapted_hint_known`). Or where a
                // `map` made of another adapter, an `enumerate()`'s, passes on
                // its source's, as that adapter does (#733).
                crate::lower::iterators::is_stepped_items(&it)
                    || source_ty.is_none() && self.adapted_hint_known(iter_ty)
                    || source_ty.is_some_and(|ty| self.adapted_hint_known(ty))
            }
            // Or of std's adapters of those, each a `$Steps` with std's
            // hint (#733).
            false => self.array_source(iter_ty).is_some() || self.adapted_hint_known(iter_ty),
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

    /// `next` and `size_hint` of a `&mut` to the crate's iterator,
    /// `by_ref()`'s: std's of a `&mut I`, which is `I`'s own, of what the
    /// `&mut` is to (#745). `None` of another, or of a hint its impl doesn't
    /// write, std's, `$sizeHint`'s.
    fn own_step_through_mut(
        &mut self,
        (def_id, trait_id): (DefId, DefId),
        (iter_ty, receiver_ty): (Ty<'tcx>, Ty<'tcx>),
        values: &[Expr],
        span: Span,
    ) -> R<Option<Expr>> {
        let std_method = self.recognition().iterator_method(def_id);
        let stepped = matches!(std_method, Some(IterMethod::Next | IterMethod::SizeHint));
        let through_mut = matches!(receiver_ty.kind(), ty::Ref(_, through, _) if through.is_ref());
        if !(stepped && through_mut && self.is_user_iterator(iter_ty)) {
            return Ok(None);
        }
        let name = self.tcx.item_name(def_id);
        let args = self.args_of(trait_id, iter_ty);
        let mut made = Vec::new();
        let value = values.first().expect("the receiver").clone();
        match self.trait_call(def_id, args, vec![value], span, &mut made)? {
            Some(call) if made.is_empty() => Ok(Some(call)),
            None if std_method == Some(IterMethod::SizeHint) => Ok(None),
            _ => Err(self.unsupported(span, &format!("`{name}` of this `&mut`, from its MIR"))),
        }
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
        let (Some((parted, _, _)), [receiver_ty, other_ty, ..]) = (parted, arg_tys) else {
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
        // A chain's other side given as an iterator is as the receiver is,
        // its JS value where it is; one given as what makes an iterator, a
        // `Vec`, refused: a chain of one is std's struct where it's cloned,
        // which its compiled `Clone` clones (#382).
        let iterates = matches!(parted, Parted::Map | Parted::Filter) || self.implements_iterator(other_ty);
        // A chain's other side given as what makes an iterator, `a.iter()
        // .chain(b)` of a `&[i32]`: its iterator, the chain's second part,
        // made where the chain is, so a clone clones how far it's stepped
        // (#1002).
        let (other, other_ty) = match iterates {
            true => (other, other_ty),
            false
                if let ty::Adt(_, chained) = self.reveal(output).kind()
                    && let Some(into) = chained.types().nth(1)
                    && self.implements_iterator(into) =>
            {
                (self.as_js_iterator(other, other_ty, span)?, into)
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
            Parted::Chain => ["a", "b"],
        };
        let own = self.is_called_as_written(other_ty);
        let back = self.stepped_from_its_back(output);
        let adapter = self.cloned_apart_adapter(
            (name.as_str(), parts, names),
            ([receiver_ty, other_ty], output),
            lent,
            span,
            made,
            |cx, a, b| {
                // One stepped from its back, its source made knowing it, a map
                // `$mapEnds` and a filter `$filterEnds`, as one not cloned
                // apart is (ADRs 0455, 0526, 0984).
                if matches!(parted, Parted::Map | Parted::Filter) && back {
                    let source = cx.as_double_ended(a, receiver_ty, span)?;
                    let callback = cx.unary_callback(b, own);
                    let ends = if parted == Parted::Map {
                        "$mapEnds"
                    } else {
                        "$filterEnds"
                    };
                    return Ok(Expr::call_helper(ends, vec![source, callback]));
                }
                let a_value = a.clone();
                let a = cx.as_js_iterator(a, receiver_ty, span)?;
                Ok(match parted {
                    // Its source given its function, `it.map(f)`.
                    Parted::Map | Parted::Filter => {
                        let callback = cx.unary_callback(b, own);
                        Expr::call(Expr::member(a, name.as_str()), vec![callback])
                    }
                    // Its sides made knowing theirs, as a chain not cloned apart
                    // is (#718, #739).
                    Parted::Chain if back => {
                        let b = cx.as_double_ended(b, other_ty, span)?;
                        Expr::call_helper("$lazyChain", vec![cx.as_double_ended(a_value, receiver_ty, span)?, b])
                    }
                    Parted::Chain => Expr::call_helper("$lazyChain", vec![a, cx.as_js_iterator(b, other_ty, span)?]),
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
        self.closing_function(adapter, output, (f, function), span).map(Some)
    }

    /// What `$holding(adapter, drop)`, `closing_function`'s, holds: its
    /// adapter, of its own type (#851).
    pub(in crate::lower) fn held_by_holding(e: &Expr) -> Option<&Expr> {
        match &e.kind {
            js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$holding") => args.first(),
            _ => None,
        }
    }

    /// `adapter`, of type `ty`, which holds `f`, a function of type
    /// `function` with a destructor, closed to drop it: `$holding(adapter,
    /// () => drop(f))`, its source closed, then its function dropped, as
    /// Rust drops an adapter's fields (ADRs 0485, 0991); folded, where std's
    /// fold of it drops it, `held_function_folded`'s `moved` (#710).
    pub(in crate::lower::mir) fn closing_function(
        &mut self,
        adapter: Expr,
        ty: Ty<'tcx>,
        (f, function): (Expr, Ty<'tcx>),
        span: Span,
    ) -> R<Expr> {
        let mut body = Vec::new();
        self.drop_value(f, function, span, &mut body)?;
        let knows_length = crate::lower::iterators::was_made_knowing_its_length(&adapter);
        let mut args = vec![adapter, Expr::arrow(Vec::new(), body)];
        let moved =
            (self.recognition().held_function_folded(ty)).and_then(crate::lower::recognition::HeldFolded::moved);
        args.extend(moved.map(Expr::str));
        let holding = Expr::call_helper("$holding", args);
        // Made knowing its length, as its adapter is, which a variable that
        // holds it keeps (#851).
        Ok(match knows_length {
            true => crate::lower::iterators::made_knowing_its_length(holding),
            false => holding,
        })
    }

    /// `mir_iter_method`'s call of `values`, `held` first, an iterator a
    /// std method took by value and rust-js drops after it: dropped though
    /// the call throws, a panic, as the method that took it drops it as it
    /// unwinds, of one whose steps may, a type parameter's, or one running
    /// what may panic (#424).
    fn consumed_call(
        &mut self,
        held: Expr,
        (def_id, trait_id, arg_tys): (DefId, DefId, &[Ty<'tcx>]),
        values: Vec<Expr>,
        output: Ty<'tcx>,
        span: Span,
        made: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let receiver_ty = arg_tys.first().copied().unwrap_or(self.tcx.types.unit);
        let mut call = Vec::new();
        let Some(result) = self.mir_iter_method(def_id, (trait_id, arg_tys), values, output, span, &mut call)? else {
            made.extend(call);
            return Ok(None);
        };
        // A collect that closed it, at its end or as a panic unwinds, of one
        // dropped by closing it: dropped, nothing after (#660).
        if result.facts.has(js::Known::ClosesWhatItSteps) && self.dropped_by_closing(receiver_ty) {
            made.extend(call);
            return Ok(Some(result));
        }
        // Its result before what's dropped after it, held where it's a value;
        // a `()`, `for_each`'s, the call's statement (#1079).
        let result = match output.is_unit() {
            true => {
                call.push(StmtKind::Expr(result).at(self.js_span(span)));
                Expr::undefined()
            }
            false => self.spill("result", result, &mut call),
        };
        let mut dropped = Vec::new();
        self.dropped_consumed(held, (receiver_ty, def_id, output), span, &mut dropped)?;
        // Or a function it's given that may panic, `for_each(f)`'s, which
        // unwinds through the method that took it (#1079).
        let given_may_panic = (arg_tys.iter().skip(1)).any(|&ty| {
            let ty = ty.peel_refs();
            self.is_callable(ty) && self.call_may_unwind(ty)
        });
        match receiver_ty.has_param() || self.steps_may_panic(receiver_ty) || given_may_panic {
            true => self.unwinding(call, dropped.clone(), self.js_span(span), made),
            false => made.extend(call),
        }
        made.extend(dropped);
        Ok(Some(result))
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
        // Its dictionary's `fold`, or of `rfold`, `DoubleEndedIterator`'s
        // `rfold`, which drops it where it's there.
        let (by, fold) = match self.recognition().rfolds_what_it_consumes(method) {
            true => (self.recognition().double_ended_iterator(), "rfold"),
            false => (Some(std_item(self.tcx, StdItem::Iterator)), "fold"),
        };
        // Of `count` and `last`, its own `count` or `last` too, the crate's,
        // which took it and dropped it (#505, #522).
        let own = match self.recognition().iterator_method(method) {
            Some(IterMethod::Count) => Some("count"),
            Some(IterMethod::Last) => Some("last"),
            _ => None,
        };
        let taken = match self.consumes_by_fold(method, output) {
            false => None,
            true => match by.and_then(|by| Some((by, self.iterator_evidence(receiver, by)?))) {
                // Whether its step took it, the value's own answer where the
                // dictionary's is std's helper that asks it, `$foldOf`, which
                // a type parameter's may be (`$took`, #621, #649).
                Some((by, dictionary)) => {
                    let unowned = |name: &str| {
                        let name = crate::lower::bindings::trait_method_name(self.tcx, by, name);
                        let step = Expr::member(dictionary.clone(), name);
                        Expr::unary(js::UnaryOp::Not, Expr::call_helper("$took", vec![step, held.clone()]))
                    };
                    Some(match own {
                        Some(own) => Expr::bin(Op::And, unowned(fold), unowned(own)),
                        None => unowned(fold),
                    })
                }
                // Of a type parameter given no dictionary, generic code's,
                // what its JS iterator says the crate's own steps took,
                // `$iterator`'s `ownFold`, `ownLast` and `ownCount` (#522).
                None if matches!(receiver.kind(), ty::Param(_)) => {
                    let unowned = |name: &str| {
                        let flag = format!("own{}{}", name[..1].to_uppercase(), &name[1..]);
                        Expr::unary(js::UnaryOp::Not, Expr::member(held.clone(), &flag))
                    };
                    Some(match own {
                        Some(own) => Expr::bin(Op::And, unowned(fold), unowned(own)),
                        None => unowned(fold),
                    })
                }
                None => None,
            },
        };
        let Some(unfolded) = taken else {
            return self.drop_value(held, receiver, span, made);
        };
        let mut dropped = Vec::new();
        self.drop_value(held, receiver, span, &mut dropped)?;
        if !dropped.is_empty() {
            made.push(StmtKind::If(unfolded, dropped, None).at(self.js_span(span)));
        }
        Ok(())
    }

    /// Whether a call of `def_id` at `span` is a `rev()`, an `enumerate()`
    /// or a `map()` whose adapter a std iterator's method steps where it's
    /// made, `body_queries::in_place_adapters` says: what the recognizers
    /// make of it, not std's compiled one (ADR 0871).
    pub(in crate::lower::mir) fn adapter_in_place(&self, def_id: rustc_span::def_id::DefId, span: Span) -> bool {
        crate::lower::recognition::makes_held_compiled(self.tcx, def_id)
            && self.frame.facts.in_place_adapters.contains(&span)
    }

    /// `made`, what a call of `def_id` at `span` gives: of a `rev()`, an
    /// `enumerate()` or a `map()` made in place (`adapter_in_place`), what
    /// the recognizers make of it (ADR 0871), marked so, which each of its
    /// uses asks (`made_in_place`), its local's reads too (#730). The one
    /// maker of the mark.
    pub(in crate::lower::mir) fn made_by(&self, def_id: DefId, span: Span, made: Expr) -> Expr {
        match self.adapter_in_place(def_id, span) {
            true => made.in_place(),
            false => made,
        }
    }

    /// Whether a call of an iterator trait's method `def_id`, of
    /// `generic_args`, steps `receiver`, a `Rev`, an `Enumerate` or a `Map`
    /// made in place, `v.iter().map(f).rfold(..)`'s `rfold`: the
    /// recognizers', as what it steps is theirs, not its compiled impl's
    /// (ADR 0871), as its maker marked it (`made_in_place`, #730).
    pub(in crate::lower::mir) fn steps_adapter_in_place(
        &self,
        def_id: rustc_span::def_id::DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
        receiver: Option<&Expr>,
    ) -> bool {
        self.tcx
            .trait_of_assoc(def_id)
            .is_some_and(|tr| crate::lower::recognition::is_iterator_extension(self.tcx, tr))
            && (generic_args.types().next()).is_some_and(|ty| crate::lower::recognition::is_held_compiled(self.tcx, ty))
            && receiver.is_some_and(crate::lower::iterators::made_in_place)
    }

    /// Whether `value`, of `ty`, is std's compiled struct, a compiled std
    /// iterator's (ADR 0561): not a `Rev`, an `Enumerate` or a `Map` made in
    /// place, as its maker marked it (`made_in_place`, #730), which is what
    /// the recognizers make of it, a JS iterator that closes as it ends (ADR
    /// 0871).
    pub(in crate::lower) fn compiled_struct(&self, ty: Ty<'tcx>, value: &Expr) -> bool {
        crate::lower::recognition::is_compiled_std_iterator(self.tcx, ty)
            && !(crate::lower::recognition::is_held_compiled(self.tcx, ty)
                && crate::lower::iterators::made_in_place(value))
    }
    /// Whether an iterator taken by value, `items.fold(..)`'s, of a
    /// `receiver` with something to drop, is dropped after its method, as
    /// the method's end drops it and JS doesn't: consumed, `collect()`, then
    /// dropped, by its own drop, the crate's own, arrayvec's `Drain`, or its
    /// caller's, a generic iterator's it's given (ADRs 0421, 0464). One whose
    /// result holds it, `map(f)`, is the result's to drop, closed to drop it,
    /// as each std adapter is (ADR 0465); of a generic one given no drop, its
    /// callers give none with a destructor (ADR 0190).
    fn dropped_after(
        &mut self,
        def_id: DefId,
        (receiver, arg_tys): (Ty<'tcx>, &[Ty<'tcx>]),
        output: Ty<'tcx>,
        given: Option<&Expr>,
        span: Span,
    ) -> R<bool> {
        if self.drops(receiver) == crate::lower::drops::Drops::Nothing {
            return Ok(false);
        }
        // What its consumer closes, which drops it there, at its end or as a
        // panic unwinds: nothing after (#637). Asked of its revealed type, as
        // `mir_iter_method` closes it.
        let iter_ty = self.reveal(receiver);
        let dropping =
            (self.iterator_item(iter_ty)).is_some_and(|item| self.drops(item) != crate::lower::drops::Drops::Nothing);
        let method = self.recognition().iterator_method(def_id);
        if self.consumer_closes(method, iter_ty, false, dropping) || self.keyed_consumer_closes(method, arg_tys) {
            return Ok(false);
        }
        // What the crate's own `fold` is called by: it takes the iterator
        // and drops it, so nothing's dropped after (#162). Not of an adapter
        // that holds a function with a destructor, which it doesn't drop.
        let own = match self.recognition().rfolds_what_it_consumes(def_id) {
            true => self
                .recognition()
                .reaches_its_own(receiver, OwnStep::Rfold, Reach::Sure),
            false => self.recognition().reaches_its_own(receiver, OwnStep::Fold, Reach::Sure),
        };
        if self.consumes_by_fold(def_id, output) && own {
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
        let compiled = given.is_none_or(|given| self.compiled_struct(receiver, given));
        let dropped = match params_only {
            true => {
                self.has_generic_drop(receiver)
                    || (compiled && self.holds_given_drops(receiver))
                    || holds_function
                    || closed
            }
            // Not a `copied()` of one closed to drop what it holds, which is
            // it (ADR 0547): consumed to its end, it's closed as it's done
            // (ADR 0570).
            false => self.is_user_iterator(receiver) || compiled || holds_function,
        };
        // What it gives that holds it: not an item of it, `Option<I::Item>`,
        // `last()`'s, whose projection names `I` (#522).
        let holds = crate::lower::mut_refs::holds_outside_projections(output, receiver);
        if dropped && given.is_some() && !holds {
            return Ok(true);
        }
        if params_only {
            self.require_consumed_drops_none(receiver, output);
        }
        Ok(false)
    }

    /// Whether an adapter of type `ty` is its source itself in JS: a
    /// `copied()`, or a `cloned()` of what needs no clone, each item as it
    /// is (ADR 0547). The one answer, of what's made and what's dropped.
    pub(in crate::lower) fn is_its_source(&self, ty: Ty<'tcx>) -> bool {
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
        // What it is, of an `impl Iterator` a function gives too, as each of
        // its methods asks it (ADR 0985).
        let iter_ty = self.reveal(receiver_ty.peel_refs());
        if let Some(stepped) = self.given_iterator_call(std_method, iter_ty, &values) {
            return Ok(Some(stepped));
        }
        if std_method == Some(IterMethod::IntoIter) && !self.holds_into_iter(receiver_ty) {
            let value = values.into_iter().next().expect("the receiver");
            return Ok(self.value_into_iter(value, receiver_ty, output));
        }
        // A range's own `nth`, `last` and `count`, of its bounds (#163).
        if let Some(stepped) = self.range_own_step(std_method, (iter_ty, receiver_ty.is_mutable_ptr()), &values) {
            return Ok(Some(stepped));
        }
        // `nth_back(n)`, `rfold(init, f)` and `rfind(p)`: of what knows its
        // back, stepped from it, a generic one's too (ADRs 0429, 0455).
        if let Some(method) = std_method.filter(|method| method.steps_from_its_back()) {
            return self.back_step(method, values, (iter_ty, output, arg_tys), span);
        }
        // `next_back(&mut it)` and `len(&it)`: of what knows its back, a range, a `$iter`,
        // one held or made knowing it where it's held (ADRs 0071, 0549, #718); not a lazy one.
        if matches!(std_method, Some(IterMethod::NextBack | IterMethod::Len)) {
            let it = values.into_iter().next().expect("the receiver");
            // The crate's iterator's `len()`, std's: its `size_hint()`, exact.
            // One its impl writes itself is its own, never asked here (ADR
            // 0492). A compiled std iterator's whose `len` isn't compiled,
            // `Peekable`'s, is its struct's hint (ADR 0561), where it's
            // std's struct, not one made in place (ADR 0871).
            if (self.is_user_iterator(iter_ty) || self.compiled_struct(iter_ty, &it))
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
            // `next_back` steps it where it is, closing nothing: of one lent,
            // `by_ref()`'s, what it lends.
            let it = match std_method {
                Some(IterMethod::NextBack) => crate::lower::iterators::unlent(&it).clone(),
                _ => it,
            };
            let boxed = self.option_of(output).is_some_and(|item| self.boxed_payload(item));
            if std_method == Some(IterMethod::NextBack)
                && self.range_kind(iter_ty).is_none()
                && let Some(back) = self.next_back_made_here(it.clone(), iter_ty, boxed, span, made)?
            {
                return Ok(Some(back));
            }
            let back = self.has_a_back(iter_ty, &it) || std_method == Some(IterMethod::NextBack) && knows_back(&it);
            // `len(&self)` of a `Self` that's a `&mut`.
            let lends_self = matches!(receiver_ty.kind(), ty::Ref(_, through, _) if through.is_mutable_ptr());
            return Ok(Some(match (std_method, self.range_kind(iter_ty)) {
                (Some(IterMethod::NextBack), Some(kind)) if let Some(step) = kind.step(true) => {
                    Expr::call_helper(step, vec![it])
                }
                (Some(IterMethod::NextBack), None) if back && boxed => Expr::call_helper("$nextBackSome", vec![it]),
                (Some(IterMethod::NextBack), None) if back => Expr::call_helper("$nextBack", vec![it]),
                // A generic one's, a JS iterator that knows it, which may be a
                // range's (ADR 0456).
                (Some(IterMethod::Len), None) if self.knows_its_length(iter_ty) => {
                    Expr::call(Expr::member(it, "len"), Vec::new())
                }
                // Of a `&mut`, `by_ref()`'s, `$lent`'s, which passes on the
                // `len` of what it's to, as std's `&mut I` does (#745).
                (Some(IterMethod::Len), None) if lends_self => Expr::call(Expr::member(it, "len"), Vec::new()),
                (Some(IterMethod::Len), None) if self.steps_an_array(iter_ty) => self.items_left(it, made),
                // One made in place, by what each of its adapters counts of
                // its source: a range of numbers' count, an array's items
                // skipped, `Math.max(xs.length - n, 0)` (ADR 0986).
                (Some(IterMethod::Len), _) if let Some(len) = self.len_in_place(&it, iter_ty, made) => len,
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
        // `rev()`, `rev_called`'s.
        if std_method == Some(IterMethod::Rev) {
            let receiver = values.into_iter().next().expect("the receiver");
            return self
                .rev_called(receiver, (iter_ty, receiver_ty, output), def_id, span)
                .map(Some);
        }
        // `size_hint()` of a `$iter`, exact, or of a generic one, as `$sizeHint`
        // finds it (ADR 0170); a range's is its bounds' (`RangeOp::SizeHint`).
        if std_method == Some(IterMethod::SizeHint)
            && !self.is_user_iterator(iter_ty)
            && self.range_kind(iter_ty.peel_refs()).is_none()
        {
            let receiver = values.into_iter().next().expect("the receiver");
            return self.size_hint_of(receiver, iter_ty, span, made).map(Some);
        }
        if let Some(call) = self.own_step_through_mut((def_id, trait_id), (iter_ty, receiver_ty), &values, span)? {
            return Ok(Some(call));
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
                Some(_) => match self.range_next(iter_ty, false, receiver) {
                    Some(next) => next,
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
        let it = self.method_source(receiver, iter_ty, (std_method, output), span)?;
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
            // By what gives each as it is, `map(StaticPath::Params)`: no step, but where a `Map`
            // hides the crate's own steps (#235), or its source's `nth` of what it drops (#493).
            Some(IterMethod::Map) => match self.unary_callback(arg(), own) {
                f if f.is_identity() && dropping.is_none() && !self.recognition().reaches_own_steps(iter_ty) => it,
                f => crate::lower::iterators::lazily_made(method("map", it, vec![f])),
            },
            // Of items with a destructor: each it doesn't keep dropped as it
            // discards it (ADR 0098), `filtered`'s.
            Some(IterMethod::Filter) => {
                let test = self.unary_callback(arg(), own);
                self.filtered(it, test, dropping, span)?
            }
            Some(IterMethod::Take) => method("take", it, vec![arg()]),
            // Of items with a destructor, each it skips dropped as it's
            // skipped, as Rust's `nth` drops it (ADRs 0422, 0465).
            Some(IterMethod::Skip) => self.skipped(it, arg(), dropping, span)?,
            Some(IterMethod::ForEach) => self.for_each_called(it, (arg(), own), arg_tys, span)?,
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
            Some(IterMethod::Fold) => self.folded((it, def_id), (Some(arg()), arg()), (arg_tys, item), span)?,
            Some(IterMethod::Reduce) => self.folded((it, def_id), (None, arg()), (arg_tys, item), span)?,
            Some(m) if m.tries() => self.try_folded(it, m, &mut arg, (own, output), arg_tys, span)?,
            Some(IterMethod::Count) => self.counted(it, item, (def_id, iter_ty), span)?,
            Some(IterMethod::Sum | IterMethod::Product) => {
                let sum = std_method == Some(IterMethod::Sum);
                self.accumulated((it, def_id), (output, item), sum, span)?
            }
            Some(IterMethod::Copied | IterMethod::Cloned) if self.is_its_source(output) => it,
            Some(IterMethod::Copied | IterMethod::Cloned) => match item {
                Some(item) => self.copied_items(it, item, output, span)?,
                None => return Ok(None),
            },
            // Of its items as they come, which the helper steps (#179).
            Some(m) if m.extreme() => self.extreme_called(it, (m, &mut arg), item, arg_tys, span)?,
            Some(IterMethod::Last) => self.last_called(it, iter_ty, item, span, made)?,
            Some(m @ (IterMethod::IsSorted | IterMethod::IsSortedBy | IterMethod::IsSortedByKey)) => match item {
                Some(item) => self.sorted_called(it, (m, &mut arg, own), item, arg_tys, span)?,
                None => return Ok(None),
            },
            Some(IterMethod::Nth) => match dropping {
                Some(item) => self.dropping_pick("$nthDropping", it, arg(), item, span)?,
                None => return Ok(None),
            },
            Some(IterMethod::AdvanceBy) => self.advanced_by(it, arg(), dropping, span)?,
            Some(IterMethod::ByRef) => it,
            Some(IterMethod::Collect) => self.collected_counted(it, (output, item), receiver_ty, span, made)?,
            _ => return Ok(None),
        }))
    }

    /// `it.advance_by(n)`, of items of type `dropping` with a destructor:
    /// its own, which drops what it passes itself, else std's, each dropped
    /// as it's passed, as `$advanceByOf` gives it (ADR 0913).
    fn advanced_by(&mut self, it: Expr, n: Expr, dropping: Option<Ty<'tcx>>, span: Span) -> R<Expr> {
        let mut args = vec![it, n];
        if let Some(item) = dropping {
            args.extend(self.std_drop_function(item, span)?);
        }
        Ok(Expr::call_helper("$advanceBy", args))
    }

    /// `it.count()`, of `def_id` of an iterator of type `iter_ty`: each item
    /// counted, and dropped as `count` drops it; of a string's UTF-16 units,
    /// its `length` (ADR 0632).
    /// `copied()` or `cloned()` of an iterator of `item`s, references: a
    /// copy of what each points at, once. Of an item that's a reference
    /// to a reference, `&&Fiber`, the reference, which a copy is itself,
    /// not what it points at, so the items as they are. Of one that knows
    /// its back or its length, `output`, a map that knows them too,
    /// `$mapEnds`, as a map is (#387): a chain stepped from its back knows
    /// no length, and `.map` has no back to step (#850).
    /// Its source made knowing its back where it's to know its own, as a
    /// map's is: a `step_by(n)`'s, `$stepByEnds` (#851).
    fn copied_items(&mut self, it: Expr, item: Ty<'tcx>, output: Ty<'tcx>, span: Span) -> R<Expr> {
        let back = self.knows_its_back(output);
        let ends = back || self.knows_its_length(output);
        let source = crate::lower::recognition::copied_source(self.tcx, output)
            .or_else(|| crate::lower::recognition::cloned_source(self.tcx, output));
        let it = match source {
            Some(source) if back => self.as_double_ended(it, source.peel_refs(), span)?,
            _ => it,
        };
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
        let copy = Expr::arrow(vec![x.into()], body);
        Ok(match ends {
            true => Expr::call_helper("$mapEnds", vec![it, copy]),
            false => Expr::call(Expr::member(it, "map"), vec![copy]),
        })
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
        // Its own `count` where it may have one, `$count`'s: std's compiled
        // one, a `Take`'s, which advances its source and asks no `nth`
        // (#213, ADR 0913), a `Skip`'s, whose `nth` drops a reversed `Vec`'s
        // as one slice (#261), and a type parameter's, which its caller's
        // iterator may have, `Take`'s in std's `Skip::count`. A flatten made
        // in place has one where its inner iterators' own count is reached,
        // `$flatMap`'s (#736); JS's `flatMap` has none.
        let iterator = std_item(self.tcx, StdItem::Iterator);
        let revealed = self.reveal(iter_ty);
        let in_place_flatten = crate::lower::recognition::flattened_source(self.tcx, revealed).is_some()
            && !self.compiled_struct(revealed, &it);
        let inners_own = self
            .recognition()
            .reaches_its_own(revealed, OwnStep::Count, Reach::Sure);
        // And a filter's, std's `Filter::count`, which asks its source's
        // hint first, where that may reach what the program sees (#733).
        let filter_asks = crate::lower::recognition::filtered_source(self.tcx, revealed)
            .is_some_and(|source| !self.compiled_struct(revealed, &it) && self.hint_reaches_own(source));
        // And a `Vec`'s, an array's or a `VecDeque`'s `IntoIter`'s, `len()`,
        // then what it has left dropped as one slice, where its items have
        // a drop, which the default would drop each as it's folded (#862).
        let slice_count = !body.is_empty() && self.recognition().drops_passed_as_slice(revealed);
        let own_count = matches!(revealed.kind(), ty::Param(_))
            || filter_asks
            || slice_count
            || self.compiled_step(revealed, iterator, "count") && (!in_place_flatten || inners_own);
        if own_count {
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
    /// an iterator is itself, as rustc selects std's blanket `impl<I:
    /// Iterator> IntoIterator for I` of it (`iterates_as_itself`), a generic
    /// one of the crate's, `Wrapper<I>`, too (#806); and a collection's
    /// items a JS iterator; `None` of what `mir_std_call` makes, `for x in
    /// &mut v`'s handles on numbers (ADR 0099).
    fn value_into_iter(&mut self, value: Expr, receiver_ty: Ty<'tcx>, output: Ty<'tcx>) -> Option<Expr> {
        let iter_ty = receiver_ty.peel_refs();
        if self.recognition().is_given_iterable(receiver_ty) {
            return Some(crate::lower::iterators::given_into_iter(value));
        }
        if self.range_kind(iter_ty).is_some() || self.recognition().iterates_as_itself(receiver_ty) {
            return Some(value);
        }
        let handles = matches!(receiver_ty.kind(), ty::Ref(_, _, Mutability::Mut))
            && self.iterator_item(output).is_some_and(|item| self.is_cell(item));
        let items = iter_ty.is_array() || iter_ty.is_slice() || self.is_vec_like(iter_ty);
        (!handles && items).then(|| self.std_iterator(value, output))
    }
}
