//! A chain that closes its source: what's dropped and when, a pick that
//! drops, and the last value.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What's stepped as it is: `by_ref()` of the crate's iterator, a `&mut`
    /// to it, it itself, which its own `next` steps; and of a type parameter
    /// given as it is, with its `Iterator` dictionary (ADR 0411), `by_ref()`
    /// and `into_iter()`, itself, and `next(&mut it)`, the dictionary's, as
    /// `next_back(&mut it)` and `len(&it)` are its extensions' (ADR 0487).
    pub(super) fn given_iterator_call(
        &self,
        method: Option<IterMethod>,
        iter_ty: Ty<'tcx>,
        values: &[Expr],
    ) -> Option<Expr> {
        let receiver = values.first()?.clone();
        if method == Some(IterMethod::ByRef) && self.is_user_iterator(iter_ty) {
            return Some(receiver);
        }
        let iterator = std_item(self.tcx, StdItem::Iterator);
        let (trait_id, entry) = match method? {
            IterMethod::ByRef | IterMethod::IntoIter => {
                self.iterator_evidence(iter_ty, iterator)?;
                return Some(receiver);
            }
            IterMethod::Next => (iterator, "next"),
            // Its item at `i`, by its dictionary's own reader, or the
            // array's or the range's it is (ADR 0975).
            IterMethod::GetUnchecked => {
                let dictionary = self.iterator_evidence(iter_ty, iterator)?;
                let at = values.get(1)?.clone();
                return Some(Expr::call_helper("$getUnchecked", vec![receiver, at, dictionary]));
            }
            IterMethod::NextBack => self.extension_entry_of(self.recognition().double_ended_iterator()?),
            IterMethod::Len => self.extension_entry_of(self.recognition().exact_size_iterator()?),
            _ => return None,
        };
        let dictionary = self.iterator_evidence(iter_ty, trait_id)?;
        Some(Expr::call(Expr::member(dictionary, entry), vec![receiver]))
    }

    /// An iterator's extension, `trait_id`, and the method its dictionary
    /// holds (ADR 0487).
    fn extension_entry_of(&self, trait_id: DefId) -> (DefId, &'static str) {
        (trait_id, crate::lower::recognition::extension_entry(self.tcx, trait_id))
    }

    /// The iterator `it`, of type `iter_ty`, a method is called on, of items
    /// with a destructor, `dropping`, or not, `lent` or not: of an adapter,
    /// `take`'s or `map`'s, of one that owns what has one, its items or
    /// itself, a crate's iterator with a `Drop`, moved into it, `$closing`,
    /// whose closing drops it, as the adapter closes it when it ends or is
    /// dropped (ADRs 0422, 0465, 0473), and of one lent, `by_ref().take(n)`,
    /// itself, its lender's (ADR 0468); else one that may leave some items
    /// with a destructor is refused.
    pub(super) fn dropping_source(
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
            Some(drop) => self.closed_by(it, drop, ty),
            None => it,
        })
    }

    /// `$closing(it, drop)`, `it` a `ty` closed by `drop`, which drops what
    /// an array's items it passes over from its back as one slice, as a
    /// `Vec`'s `IntoIter` does; of one that owns them and drops each as it's
    /// passed, a `Drain`'s or a B-tree's, `$closing(it, drop, false)` (#234).
    pub(in crate::lower) fn closed_by(&self, it: Expr, drop: Expr, ty: Ty<'tcx>) -> Expr {
        let mut args = vec![it, drop];
        if self.passes_each_owned(ty) {
            args.push(Expr::bool(false));
        }
        Expr::call_helper("$closing", args)
    }

    /// Whether `ty` owns an array's items and drops each it passes over
    /// from its back as it's passed, not as one slice (#234).
    pub(in crate::lower) fn passes_each_owned(&self, ty: Ty<'tcx>) -> bool {
        self.array_source(ty.peel_refs()) == Some(true) && !self.recognition().drops_passed_as_slice(ty)
    }

    /// What closes a `ty` an adapter holds, its own drop: of one that owns
    /// items with a destructor, a generic one whose drop its caller gives,
    /// or the crate's own with one, stepped as `$iterator(value, next)`.
    /// `None` of one with nothing to drop, and of a compiled std iterator's
    /// struct, a `Peekable`'s, which drops what it holds itself, closed or
    /// stepped to its end (ADRs 0561, 0925).
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
            // drops its `value` (ADR 0421), as one given as it is is (ADR
            // 0514).
            true => {
                let stepped = self.is_user_iterator(ty) || self.iterator_given_as_it_is(ty);
                self.held_drop_function(ty, stepped, span)
            }
            false => Ok(None),
        }
    }

    /// `a.eq(b)`, or `a.ne(b)`, of `a`, of `item`s, an `iter_ty`, `lent` or
    /// not, and `b`, an `IntoIterator` of type `other`: `$iterEq`, each pair compared as
    /// Rust steps them; of items with a destructor, each pair dropped as it's
    /// compared, and each closed as it ends, as std's are dropped (ADR 0465).
    pub(super) fn iterators_eq(
        &mut self,
        (it, other): (Expr, Expr),
        (item, iter_ty, other_ty): (Ty<'tcx>, Ty<'tcx>, Ty<'tcx>),
        (lent, ne): (bool, bool),
        span: Span,
    ) -> R<Expr> {
        // A range's, or the crate's, stepped as one, as `chain`'s and `zip`'s are.
        let other = self.as_js_iterator(other, other_ty, span)?;
        // Of two `TrustedLen` iterators, std's `SpecIterEq` asks their
        // lengths first, and gives `false` of unequal ones with no item
        // stepped or compared (ADR 0992): each counted where it's made,
        // before it's stepped, as `len_in_place` counts it.
        let trusted_len =
            crate::lower::recognition::fact_trait(self.tcx, crate::lower::recognition::TypeFact::TrustedLen);
        let other_iter = crate::lower::recognition::iterator_type_of(self.tcx, self.typing_env, other_ty);
        let trusted = trusted_len.is_some_and(|trusted| {
            [iter_ty, other_iter]
                .iter()
                .all(|&ty| self.recognition().implements_trait(ty, trusted))
        });
        let lengths = match trusted {
            true => {
                let mut spilled = Vec::new();
                let source = crate::lower::iterators::closed_parts(&it).map_or(&it, |(source, _)| source);
                let a = self.counted_before_stepping(source, iter_ty, &mut spilled);
                // What's given whole, a `Vec` or an array: its length.
                let b = match self.slice_item(other_ty.peel_refs()) {
                    Some(_) if !self.implements_iterator(other_ty) => Some(Expr::member(other.clone(), "length")),
                    _ => self.counted_before_stepping(&other, other_iter, &mut spilled),
                };
                (a.zip(b)).filter(|_| spilled.is_empty())
            }
            false => None,
        };
        // Of ones it can't count before stepping them, refused where what
        // comparing them runs can be seen: a function either holds, a map's,
        // or items with a destructor, which std's would leave unrun and drop
        // as they are (ADR 0996, #386).
        let other_item = self.iterable_item(other_ty);
        let runs_code = |ty: &Ty<'tcx>| {
            ty.walk()
                .any(|part| part.as_type().is_some_and(|t| t.is_closure() || t.is_fn()))
        };
        if lengths.is_none()
            && trusted
            && ([iter_ty, other_iter].iter().any(runs_code)
                || [item, other_item]
                    .iter()
                    .any(|&t| self.drops(t) != crate::lower::drops::Drops::Nothing))
        {
            return Err(self.unsupported(
                span,
                "`eq` of two iterators std asks the lengths of first, whose lengths rust-js can't count before stepping them",
            ));
        }
        let other = match self.closing_drop(other_ty, span)? {
            Some(drop) => {
                let it = self.js_iterator(other);
                self.closed_by(it, drop, other_ty)
            }
            None => other,
        };
        let it = match lent {
            true => it,
            false => self.closing(it, iter_ty, span)?,
        };
        let compare = self.item_eq((item, other_item), span)?;
        let drop_a = self.std_drop_function(item, span)?;
        let drop_b = self.std_drop_function(other_item, span)?;
        let mut list = vec![it, other];
        if compare.is_some() || drop_a.is_some() || drop_b.is_some() || lengths.is_some() {
            list.push(compare.unwrap_or_else(Expr::undefined));
        }
        if drop_a.is_some() || drop_b.is_some() || lengths.is_some() {
            list.push(drop_a.unwrap_or_else(Expr::undefined));
        }
        if drop_b.is_some() || lengths.is_some() {
            list.push(drop_b.unwrap_or_else(Expr::undefined));
        }
        list.extend(lengths.map(|(a, b)| Expr::array(vec![a, b])));
        let same = Expr::call_helper("$iterEq", list);
        Ok(match ne {
            true => Expr::unary(js::UnaryOp::Not, same),
            false => same,
        })
    }

    /// `a.cmp(b)` of two iterators, `partial_cmp`, and `lt`, `le`, `gt` and
    /// `ge`: `$iterCmp(a, b, cmp)`, which steps both in turn as std's
    /// `cmp_by` does, by their items' `Ord` or `PartialOrd`; `lt` and the rest
    /// it against 0, which `partial_cmp`'s `None`, `undefined`, fails, as
    /// std's are `false` of it. Of items with a destructor, which it doesn't
    /// drop as it compares them, refused.
    pub(super) fn iterators_cmp(
        &mut self,
        (it, other): (Expr, Expr),
        (item, other_ty): (Option<Ty<'tcx>>, Ty<'tcx>),
        method: IterMethod,
        span: Span,
    ) -> R<Expr> {
        let other_item = self.iterable_item(other_ty);
        // `it` is stepped already; what it's compared with, as it's given.
        let other = self.as_js_iterator(other, other_ty, span)?;
        let item = item.filter(|&item| item == other_item);
        let Some(item) = item.filter(|&item| self.drops(item) == crate::lower::drops::Drops::Nothing) else {
            return Err(self.unsupported(span, "comparing iterators of these items"));
        };
        let compare = self.cmp_fn(item, method != IterMethod::Cmp, span)?;
        let order = Expr::call_helper("$iterCmp", vec![it, other, compare]);
        let op = match method {
            IterMethod::Lt => Op::Lt,
            IterMethod::Le => Op::Le,
            IterMethod::Gt => Op::Gt,
            IterMethod::Ge => Op::Ge,
            _ => return Ok(order),
        };
        Ok(Expr::bin(op, order, Expr::int(0)))
    }

    /// `skip(n)` of `it`: JS's `drop(n)`; of items with a destructor,
    /// `$skipping`, each it skips dropped as it's skipped, as Rust's `nth`
    /// drops it (ADRs 0422, 0465).
    pub(super) fn skipped(&mut self, it: Expr, n: Expr, dropping: Option<Ty<'tcx>>, span: Span) -> R<Expr> {
        Ok(match dropping {
            Some(item) => {
                let drop = self.std_drop_function(item, span)?.expect("a destructor");
                Expr::call_helper("$skipping", vec![it, n, drop])
            }
            None => Expr::call(Expr::member(it, "drop"), vec![n]),
        })
    }

    /// The one item of `it` a method picks, of items of type `item`, which
    /// has a destructor: `nth(n)`'s, `$nthDropping`, and `find(test)`'s,
    /// `$findDropping`, each it passes dropped, as std's are (ADR 0098), its
    /// `Some` boxed where it may look like `None` (ADR 0051).
    pub(super) fn dropping_pick(
        &mut self,
        helper: &'static str,
        it: Expr,
        which: Expr,
        item: Ty<'tcx>,
        span: Span,
    ) -> R<Expr> {
        let drop = self.std_drop_function(item, span)?.expect("a destructor");
        let boxed = Expr::bool(self.boxed_payload(item));
        Ok(Expr::call_helper(helper, vec![it, which, drop, boxed]))
    }

    /// `last()` of `it`, of items of type `item`: of ones with a destructor,
    /// `$lastDropping`, each before the last dropped as the next comes, as
    /// std's are (ADR 0098); else the last of its array.
    /// `it.last()` of an iterator of type `iter_ty`: of one whose `last`
    /// reaches the crate's own, its `last`, as `nth` is `$nth` (#182), its
    /// `Some` boxed where the item looks like `None` (ADR 0051); else its
    /// last item, `last_value`, of what steps it by its `fold`.
    pub(super) fn last_called(
        &mut self,
        it: Expr,
        iter_ty: Ty<'tcx>,
        item: Option<Ty<'tcx>>,
        span: Span,
        made: &mut Vec<Stmt>,
    ) -> R<Expr> {
        if self.recognition().lasts_its_own(iter_ty) {
            let boxed = item.is_some_and(|item| self.boxed_payload(item));
            return Ok(Expr::call_helper(if boxed { "$lastSome" } else { "$last" }, vec![it]));
        }
        let it = self.by_its_fold(it, iter_ty);
        self.last_value(it, item, span, made)
    }

    pub(super) fn last_value(&mut self, it: Expr, item: Option<Ty<'tcx>>, span: Span, made: &mut Vec<Stmt>) -> R<Expr> {
        match item
            .map(|item| self.std_drop_function(item, span))
            .transpose()?
            .flatten()
        {
            // `Some` of an item that could look like `None` boxed (ADR 0051,
            // #204).
            Some(drop) => {
                let mut given = vec![it, drop];
                if item.is_some_and(|item| self.boxed_payload(item)) {
                    given.push(Expr::bool(true));
                }
                Ok(Expr::call_helper("$lastDropping", given))
            }
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
                    | IterMethod::Reduce
                    | IterMethod::TryFold
                    | IterMethod::TryForEach
                    | IterMethod::Collect
                    | IterMethod::Any
                    | IterMethod::All
                    | IterMethod::Position
                    | IterMethod::RPosition
                    | IterMethod::Enumerate
                    | IterMethod::ByRef
                    | IterMethod::Sum
                    | IterMethod::Product
                    | IterMethod::Count
                    | IterMethod::Max
                    | IterMethod::Min
                    | IterMethod::Eq
                    | IterMethod::Ne
                    | IterMethod::Last
                    | IterMethod::Nth
                    | IterMethod::AdvanceBy
                    | IterMethod::Find
                    | IterMethod::Partition
                    | IterMethod::Filter
                    | IterMethod::SkipWhile
            )
        );
        if keeps_each {
            return Ok(());
        }
        Err(self.unsupported(span, &format!("`{name}` of items with a destructor, from its MIR")))
    }
}
