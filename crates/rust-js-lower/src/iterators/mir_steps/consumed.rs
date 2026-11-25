//! Who drops what std's code takes by value, standing in for its callee's
//! drop: the one answer, `consumed_drop`, which the consumer's lowering
//! and `dropped_after` each read and neither derives (#2572). Its cases in
//! the order they're asked: lent or nothing to drop, nobody's; a consumer
//! that closes it, `$closing`, which drops it at its end and as a panic
//! unwinds; a fallible `collect()`'s `GenericShunt`'s; the crate's own
//! `fold`'s; else the caller's after the call, or nobody's.

use super::*;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;

/// Who drops an iterator std's code takes by value, standing in for its
/// callee's drop (`consumed_drop`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConsumedDrop {
    /// Nothing to drop, or lent: its lender's.
    Nobody,
    /// The consumer closes it, `$closing`: at its end, and as a panic
    /// unwinds inside the callee's frame, before what the callee holds.
    Closes,
    /// The caller drops it after the call, what's left in it then too.
    After,
    /// A `from_iter` that runs code takes it in std's `GenericShunt`, its
    /// own to drop (#1588): nothing after, no `$closing`.
    Shunted,
    /// The crate's own `fold` or `rfold` takes it and drops it (#162).
    OwnFold,
}

/// A call of std's code that takes an iterator by value: its `callee`,
/// what `method` of an iterator's it is, the iterator's type and the
/// call's arguments' (the iterator's first), what it gives, whether it's
/// lent, `by_ref()`'s, and the JS of what it's given, where it's asked
/// with it.
#[derive(Clone, Copy)]
pub struct Consumed<'c, 'tcx> {
    pub callee: DefId,
    pub method: Option<IterMethod>,
    pub iter_ty: Ty<'tcx>,
    pub arg_tys: &'c [Ty<'tcx>],
    pub output: Ty<'tcx>,
    pub lent: bool,
    pub given: Option<&'c Expr>,
}

impl<'c, 'tcx> Consumed<'c, 'tcx> {
    /// A call of `callee`, `method`'s, of an `iter_ty` given `arg_tys`,
    /// giving `output`, asked without its JS.
    pub fn of(
        callee: DefId,
        method: Option<IterMethod>,
        (iter_ty, arg_tys, output): (Ty<'tcx>, &'c [Ty<'tcx>], Ty<'tcx>),
        lent: bool,
    ) -> Self {
        Consumed {
            callee,
            method,
            iter_ty,
            arg_tys,
            output,
            lent,
            given: None,
        }
    }

    /// std's `collect()` of an `iter_ty` into an `output`, as a collect
    /// lowered apart from its call asks it, a `from_iter`'s.
    pub fn collecting(tcx: TyCtxt<'tcx>, iter_ty: &'c Ty<'tcx>, output: Ty<'tcx>) -> Self {
        let iterator = crate::recognition::std_item(tcx, crate::recognition::StdItem::Iterator);
        let collect = crate::recognition::trait_method(tcx, iterator, "collect");
        Consumed::of(
            collect,
            Some(IterMethod::Collect),
            (*iter_ty, std::slice::from_ref(iter_ty), output),
            false,
        )
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Who drops what `call` takes, the one answer (#2572). Of an iterator
    /// with something to drop, `items.fold(..)`'s: closed by its consumer
    /// where it closes it, which drops it there, nothing after
    /// (#637); its `GenericShunt`'s of a fallible `collect()` whose
    /// `from_iter` runs code (#1588); the crate's own `fold`'s (#162); else
    /// dropped after the call, as the method's end drops it and JS doesn't,
    /// by its own drop, the crate's own, arrayvec's `Drain`, or its
    /// caller's, a generic iterator's it's given (ADRs 0421, 0464). One
    /// whose result holds it, `map(f)`, is the result's to drop, closed to
    /// drop it, as each std adapter is (ADR 0465).
    pub fn consumed_drop(&mut self, call: Consumed<'_, 'tcx>, span: Span) -> R<ConsumedDrop> {
        let Consumed {
            callee: def_id,
            method,
            iter_ty: receiver,
            arg_tys,
            output,
            lent,
            given,
        } = call;
        if self.drops(receiver) == crate::drops::Drops::Nothing {
            return Ok(ConsumedDrop::Nobody);
        }
        // What its consumer closes, which drops it there, at its end or as a
        // panic unwinds: nothing after (#637), as `mir_iter_method` closes it.
        let dropping =
            (self.iterator_item(receiver)).is_some_and(|item| self.drops(item) != crate::drops::Drops::Nothing);
        // Not `extend`'s, which never closes what it's given: dropped after
        // it, what's left in it then too, as it unwinds as well (#1120).
        let extends = (self.tcx.trait_of_assoc(def_id)).is_some_and(|tr| crate::recognition::is_extend(self.tcx, tr));
        // A consumer that takes each item, `for_each`'s, `fold`'s, `count`'s
        // or `last`'s, of items with a destructor, closes what it's given,
        // which drops it at its end and as a panic part-way unwinds (#422),
        // of what closing drops (`closes_to_drop`). Not of one lent, its
        // lender's; nor of a type parameter's, its own drop after, which
        // MIR writes, unwinding too; nor a `collect()` of an adapter, which
        // runs nothing of its own as it steps, its adapters closing their
        // source already. A `partition` or an `unzip` closes a type
        // parameter's too, as std's fold drops it before what they collected
        // (#1150, #1161). A `collect()` of the crate's iterator itself, or
        // of a type parameter's, which may be one, closes it, so a panic in
        // its `next` drops it before what it collected, as std's collect
        // frame holds it inside the one that holds its `Vec` (#655, #640).
        // A sized box's of a type parameter's is the type parameter's (#2219).
        let by_bounds = crate::recognition::known_by_bounds(crate::recognition::held_unboxed(receiver));
        let consumer_closes = dropping
            && !method.is_some_and(IterMethod::holds_its_source)
            && !lent
            && match method {
                Some(IterMethod::Collect) => self.is_user_iterator(receiver) || by_bounds,
                // A `partition` or an `unzip`, of a type parameter's too,
                // whose source std's fold drops as a panic unwinds, before
                // what it collected (#1150, #1161).
                Some(IterMethod::Partition | IterMethod::Unzip) => true,
                _ => !by_bounds,
            }
            && self.closes_to_drop(receiver);
        // A keyed extreme, `max_by_key`'s or `min_by_key`'s, of the crate's
        // iterator with a destructor, or a type parameter's, closes it: of a
        // key function moved into its fold with a destructor
        // (`consumed_function`), std's `Map` the key's `reduce` folds drops
        // the iterator, then the function, where its first step ends, after
        // the fold its function drops before what's left (#789); of keys
        // with a destructor, the fold drops the iterator before `let (_, x)`
        // drops the kept key (#1649).
        let keyed_closes = || {
            let (Some(IterMethod::MaxByKey(_)), [iter, f, ..]) = (method, arg_tys) else {
                return false;
            };
            let keys_drop = || {
                self.iterator_item(*iter).is_some_and(|item| {
                    let key = self.key_of(*f, Ty::new_imm_ref(self.tcx, self.tcx.lifetimes.re_erased, item));
                    self.drops(key) != crate::drops::Drops::Nothing
                })
            };
            (self.is_user_iterator(*iter) || crate::recognition::known_by_bounds(*iter))
                && self.closes_to_drop(*iter)
                && (self.consumed_function(method, arg_tys) || keys_drop())
        };
        if !extends && (consumer_closes || keyed_closes()) {
            return Ok(ConsumedDrop::Closes);
        }
        // A `collect()` into an `Option` or a `Result` of a collection whose
        // `from_iter` runs code gives it the iterator, in std's
        // `GenericShunt`, its to drop or keep, nothing after (#1588); one
        // that closes it drops it there, nothing after (#2561).
        if method == Some(IterMethod::Collect) && self.try_processed_into(output, receiver)?.is_some() {
            return Ok(ConsumedDrop::Shunted);
        }
        // A fallible `collect()`, `sum` or `product` into an `Option` or a
        // `Result`, std's `try_process` each, closed, so the source drops
        // inside its frame, as the kept `Err` is held (#2561, #2580): of what
        // closing drops (`closes_to_drop`), a type parameter's or one that
        // owns items with a destructor. Not the crate's own iterator, closed
        // where its consumer closes it or dropped after the call; nor a
        // compiled std adapter's struct, which closing leaves as it is,
        // dropped after.
        if matches!(
            method,
            Some(IterMethod::Collect | IterMethod::Sum | IterMethod::Product)
        ) && (self.option_of(output).is_some() || self.recognition().is_result(output))
            && !self.is_user_iterator(receiver)
            && self.closes_to_drop(receiver)
        {
            return Ok(ConsumedDrop::Closes);
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
        // Or an `extend` by `for_each`, of a hash set's, a B-tree's or a
        // list's, which reaches it too (#1509).
        let folds = match extends {
            true => arg_tys.first().is_some_and(|&into| self.extends_by_fold(into)),
            false => self.consumes_by_fold(def_id, output),
        };
        if folds && own {
            if (self.recognition().held_function(receiver))
                .is_some_and(|function| self.drops(function) != crate::drops::Drops::Nothing)
            {
                return Err(self.unsupported(
                    span,
                    "an adapter holding a closure with a destructor, over an iterator whose impl writes its own `fold`",
                ));
            }
            return Ok(ConsumedDrop::OwnFold);
        }
        let params_only = self.drop_query().drops_but_params(receiver) == crate::drops::Drops::Nothing;
        // And an adapter that holds a function with a destructor, `map(f)`'s,
        // dropped with it (ADR 0485).
        let holds_function = (self.recognition().held_function(receiver))
            .is_some_and(|function| self.drops(function) != crate::drops::Drops::Nothing);
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
        let holds = crate::mut_refs::holds_outside_projections(output, receiver);
        Ok(match dropped && given.is_some() && !holds {
            true => ConsumedDrop::After,
            false => ConsumedDrop::Nobody,
        })
    }

    /// `it`, what `call` takes, closed where its consumer closes it
    /// (`consumed_drop`), else as it is, dropped after the call or nobody's,
    /// never both (#1120, #1123).
    pub fn consumer_closing(&mut self, it: Expr, call: Consumed<'_, 'tcx>, span: Span) -> R<Expr> {
        match self.consumed_drop(call, span)? {
            ConsumedDrop::Closes => self.closing(it, call.iter_ty, span),
            _ => Ok(it),
        }
    }
}
