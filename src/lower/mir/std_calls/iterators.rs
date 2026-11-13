//! An iterator's call `mir_iter_call` leaves, a combinator's, a step
//! through a `Peekable`, a range's or a stream's (ADRs 0062, 0071):
//! `mir_iter_std_call`'s, apart.

use super::*;
use crate::lower::recognition::Lend;
use crate::lower::temporary_names::Role;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// An iterator's call `mir_iter_call` leaves: a combinator, a step
    /// through a `Peekable`, a range's, a stream's (ADRs 0062, 0071).
    pub(in crate::lower::mir) fn mir_iter_std_call<'c>(
        &self,
        state: &State<'_, 'tcx>,
        call: StdCall<'c, 'tcx>,
    ) -> Option<StdLowering<'c, 'a, 'tcx>> {
        let this = self;
        let StdCall {
            known,
            def_id,
            generic_args,
            arg_tys,
            output,
            span,
            ..
        } = call;
        Some(match known {
            // An `Option` of a place found given to a closure that takes it as
            // one, before any other combinator's: `at !== -1 ? f(at) : -1` of `and_then`'s, `: undefined`
            // of `map`'s (ADR 0642).
            Std::Comb(crate::lower::combinators::Comb::AndThen) | Std::OptionMap
                if state.facts.offsets().call_in_units(state.at) =>
            {
                lower(move |this, state, values, out| {
                    Ok(Value::Expr({
                        let at = this.value_expr(values.next().expect("the place found"), span)?;
                        let f = this.value_expr(values.next().expect("the closure"), span)?;
                        let at = match at.reads_same() {
                            true => at,
                            false => {
                                this.flush(state, out)?;
                                this.spill(Role::At, at, out)
                            }
                        };
                        let then = this.call_with(f, vec![at.clone()], Role::Then, out);
                        let mapped = known == Std::OptionMap;
                        let then = match mapped && this.option_of(output).is_some_and(|item| this.boxed_payload(item)) {
                            true => this.some(then),
                            false => then,
                        };
                        let none = if mapped { Expr::undefined() } else { Expr::int(-1) };
                        Expr::cond(crate::lower::std_types::text::found(at), then, none)
                    }))
                })
            }
            // An iterator's combinator, of a JS iterator, lazy as Rust's; one a
            // `&mut` lends, stepped through what can't close it.
            Std::IterComb(comb) => lower(move |this, state, values, out| {
                Ok(Value::Expr({
                    let iter_ty = arg_tys[0].peel_refs();
                    // Items with a destructor a combinator may leave undropped; not
                    // one that leaves none, `partition`'s and `max_by`'s, nor an
                    // adapter that holds what it steps, closed to drop it, which
                    // drops each it discards (ADR 0465).
                    let dropping = this
                        .iterator_item(iter_ty)
                        .filter(|&item| this.drops(item) != crate::lower::drops::Drops::Nothing);
                    let keeps_each = crate::lower::recognition::keeps_each(this.tcx.item_name(def_id).as_str());
                    if dropping.is_some() && !keeps_each && !comb.holds_its_sources() {
                        return Err(this.unsupported(
                            span,
                            "an iterator's combinator of items with a destructor, from its MIR",
                        ));
                    }
                    let lent = matches!(arg_tys[0].kind(), ty::Ref(_, _, ty::Mutability::Mut));
                    let exprs = values
                        .map(|v| match v {
                            Value::Ref(place) => Ok(place),
                            v => this.value_expr(v, span),
                        })
                        .collect::<R<Vec<_>>>()?;
                    let mut exprs = exprs;
                    let receiver = exprs.remove(0);
                    let it = this.as_js_iterator(receiver, iter_ty, span)?;
                    // One that owns items with a destructor, given whole to a
                    // consumer that leaves none, `partition`'s or `max_by`'s,
                    // closed too: a panic in its callback stops it part-way,
                    // and what's left is dropped as it unwinds, by its closing,
                    // as `dropping_source` closes `max()`'s (#423).
                    // Of a consumer, where it closes it (#1120).
                    let method = this.recognition().iterator_method(def_id);
                    let it = match lent {
                        true => this.lent_iterator(it),
                        false if comb.holds_its_sources() => this.closing(it, iter_ty, span)?,
                        false if keeps_each => {
                            this.consumer_closing(it, (method, iter_ty, false), dropping.is_some(), span)?
                        }
                        false => it,
                    };
                    // Another iterator it's given, `chain(Counter(0))`'s, the
                    // crate's: stepped by its own `next`, not its fields, as a
                    // type parameter's given as it is is by its dictionary (ADR
                    // 0487); one that owns items with a destructor, `zip(v)`'s,
                    // `$closing`.
                    let mut exprs = (exprs.into_iter().zip(&arg_tys[1..]))
                        .map(|(e, &ty)| {
                            // What `zip` and `chain` take, an `IntoIterator`'s
                            // items (ADR 0528); another's argument, a function or
                            // a value, as it is.
                            if !matches!(comb, IterComb::Zip | IterComb::Chain) {
                                return Ok(e);
                            }
                            // An iterator a zip by index asks its length of, as
                            // it is, `zipped` makes it knowing it (ADR 0973).
                            if comb == IterComb::Zip
                                && this.implements_iterator(ty)
                                && crate::lower::recognition::zips_by_index(this.tcx, this.typing_env, iter_ty, ty)
                            {
                                return Ok(e);
                            }
                            let e = this.iterable_items(e, ty, span, out)?;
                            // Closed to drop what it holds, a JS iterator of it.
                            Ok(match this.closing_drop(ty, span)? {
                                // An array, a `$iter` of it, as std's `IntoIter` steps
                                // it, its `nth` from either end too (#250).
                                Some(drop)
                                    if this.slice_item(ty.peel_refs()).is_some()
                                        && !crate::lower::iterators::is_js_iterator(&e) =>
                                {
                                    let it = this.stepped_items(e);
                                    this.closed_by(it, drop, ty)
                                }
                                Some(drop) if crate::lower::iterators::is_js_iterator(&e) => {
                                    this.closed_by(e, drop, ty)
                                }
                                Some(drop) => {
                                    let it = this.js_iterator(e);
                                    this.closed_by(it, drop, ty)
                                }
                                None => e,
                            })
                        })
                        .collect::<R<Vec<_>>>()?;
                    // A function it holds with a destructor, `inspect(f)`'s, in a
                    // variable, dropped after its source as it's closed (ADR
                    // 0991).
                    let held = (this.recognition().held_function(output))
                        .filter(|&function| this.drops(function) != crate::lower::drops::Drops::Nothing);
                    let held = match (held, exprs.first().cloned()) {
                        (Some(function), Some(f)) => {
                            let f = match f.kind {
                                js::ExprKind::Var(_) => f,
                                _ => this.spill(Role::F, f, out),
                            };
                            exprs[0] = f.clone();
                            Some((f, function))
                        }
                        _ => None,
                    };
                    if let Some(item) = dropping
                        && matches!(comb, IterComb::SkipWhile | IterComb::TakeWhile)
                    {
                        this.flush(state, out)?;
                        this.dropping_discarded(item, &mut exprs, span)?;
                    }
                    let made =
                        this.iter_comb(comb, it, exprs.into_iter(), generic_args, (iter_ty, output), span, out)?;
                    // A flatten the crate steps from its back, made knowing it
                    // (ADR 0545): `$flattenEnds`, of what the recognizers made
                    // in place, so marked, not std's struct (ADR 0899). A
                    // chain a function gives behind an `impl` is std's `Chain`,
                    // held, stepped from its back by its own methods (ADR 0919).
                    let made = match crate::lower::recognition::is_held_compiled(this.tcx, output) {
                        true => made.in_place(),
                        false => made,
                    };
                    // And an `inspect(f)`'s, made as a map, and a `filter_map(f)`'s (#473);
                    // a chain's, `$lazyChain` of sides made knowing theirs (#718, #739).
                    let back_made = [
                        IterComb::Flatten,
                        IterComb::FlatMap,
                        IterComb::Inspect,
                        IterComb::FilterMap,
                        IterComb::Chain,
                        IterComb::Zip,
                    ];
                    let made = match back_made.contains(&comb) && this.stepped_from_its_back(output) {
                        true => this.as_double_ended(made, output, span)?,
                        false => made,
                    };
                    match held {
                        Some(held) => this.closing_function(made, output, held, span)?,
                        None => made,
                    }
                }))
            }),
            // `cycle()` of another iterator than std's over an array, cloned
            // (ADR 0491); and one a struct holds, of an array's too, which
            // keeps both it cycles (ADR 0544); and of an array's items it
            // owns whose clone or drop shows (`clone_shows`), `Cycle::new`'s clone of all
            // of them, its first round theirs, moved, and each round after a
            // clone of all again, the one it ends dropped, as std's (#1583).
            Std::Step(StepOp::Cycle)
                if this.held_cycle(output).is_some()
                    || arg_tys.first().is_none_or(|t| match this.array_source(t.peel_refs()) {
                        None => true,
                        Some(owns) => {
                            owns && this.iterator_item(t.peel_refs()).is_some_and(|item| {
                                this.clone_shows(item) || this.drops(item) != crate::lower::drops::Drops::Nothing
                            })
                        }
                    }) =>
            {
                lower(move |this, _, values, _| {
                    Ok(Value::Expr({
                        let it = values.map(|v| this.value_expr(v, span)).next().expect("the iterator")?;
                        this.cycled(it, arg_tys[0].peel_refs(), span)?
                    }))
                })
            }
            // A `Peekable`'s own methods, rewritten over std's struct, the
            // JS iterator it is (ADR 0561): what it peeked at, or what it
            // steps next, kept; `next_if`, its next item where the test
            // takes it, else kept.
            Std::Step(op @ (StepOp::Peek | StepOp::PeekMut | StepOp::NextIf | StepOp::NextIfEq))
                if arg_tys.first().is_some_and(|t| this.is_peekable(t.peel_refs())) =>
            {
                lower(move |this, _, values, out| {
                    Ok(Value::Expr({
                        let mut exprs = values
                            .map(|v| this.value_expr(v, span))
                            .collect::<R<Vec<_>>>()?
                            .into_iter();
                        let it = exprs.next().expect("the iterator");
                        let boxed = this.option_of(output).is_some_and(|item| this.boxed_payload(item));
                        match op {
                            StepOp::Peek if boxed => Expr::call_helper("$peekSome", vec![it]),
                            // The item, in a crate that asks an address recorded as
                            // stored in the `Peekable`'s `peeked` (#1475).
                            StepOp::Peek => {
                                let mut given = [it];
                                let record = this.lent_record((def_id, generic_args), &mut given, out)?;
                                let [it] = given;
                                let peeked = Expr::call_helper("$peek", vec![it]);
                                match record {
                                    Some(record) => record(peeked),
                                    None => peeked,
                                }
                            }
                            // A handle on a number or text, else the item itself.
                            StepOp::PeekMut => match this.iterator_item(arg_tys[0].peel_refs()) {
                                // A cell of a number's or generic code's; in a crate
                                // that asks an address, a handle on its place in the
                                // `Peekable`, past its two `Option`s' tags, where no
                                // `$some` box is between (#1474).
                                Some(item) if this.mut_ref_form(item).is_cell() => {
                                    let peekable = arg_tys[0].peel_refs();
                                    let placed = !this.boxed_lent(peekable, Lend::Peekable)
                                        && this.lent_place((it.clone(), peekable), Lend::Peekable, None).is_some();
                                    match placed {
                                        true => {
                                            let it = match it.reads_same() {
                                                true => it,
                                                false => this.spill(Role::Peekable, it, out),
                                            };
                                            let place = (this.lent_place((it.clone(), peekable), Lend::Peekable, None))
                                                .expect("placed");
                                            let peeked = Expr::call_helper("$peek", vec![it]);
                                            Expr::cond(
                                                Expr::bin(Op::Eq, peeked, Expr::undefined()),
                                                Expr::undefined(),
                                                place,
                                            )
                                        }
                                        false => Expr::call_helper("$peekCell", vec![it]),
                                    }
                                }
                                // The item itself, an object, in a crate that asks
                                // an address recorded as stored in `peeked` (#1475).
                                _ => {
                                    let mut given = [it];
                                    let record = this.lent_record((def_id, generic_args), &mut given, out)?;
                                    let [it] = given;
                                    let item = Expr::call_helper("$peek", vec![it]);
                                    match record {
                                        Some(record) => record(item),
                                        None => item,
                                    }
                                }
                            },
                            // Of items that may look like `None`, each boxed, the
                            // test given it unboxed.
                            // Where the test may panic, the item dropped as it
                            // unwinds, as std's `next_if` holds it (#1648).
                            StepOp::NextIf => {
                                let f = exprs.next().expect("a test");
                                let item = this.iterator_item(arg_tys[0].peel_refs());
                                let drop = match item {
                                    Some(item) if this.call_may_unwind(arg_tys[1]) => {
                                        this.std_drop_function(item, span)?
                                    }
                                    _ => None,
                                };
                                let mut args = vec![it, f];
                                args.extend(drop);
                                Expr::call_helper(if boxed { "$nextIfSome" } else { "$nextIf" }, args)
                            }
                            _ if boxed => {
                                return Err(this.unsupported(span, "`next_if_eq` of items that may be `None`"));
                            }
                            _ => {
                                let item = arg_tys[1];
                                if !this.eq_is_identity(item) {
                                    return Err(
                                        this.unsupported(span, &format!("`next_if_eq` of `{}`s", item.peel_refs()))
                                    );
                                }
                                let x = exprs.next().expect("a value");
                                let x = if x.reads_same() {
                                    x
                                } else {
                                    this.spill(Role::Expected, x, out)
                                };
                                // Its parameter named apart from what it compares
                                // with, a variable the program calls `item` (#1667).
                                let item = Expr::param_apart("item", &[&x]);
                                let same = Expr::arrow(
                                    vec![item.clone().into()],
                                    vec![
                                        StmtKind::Return(Some(Expr::bin(Op::Eq, Expr::var(&item), x)))
                                            .at(js::Span::NONE),
                                    ],
                                );
                                Expr::call_helper("$nextIf", vec![it, same])
                            }
                        }
                    }))
                })
            }
            // A slice's or string's iterator's own: of a `$iter`, which knows
            // where it is (ADR 0071).
            Std::Step(op @ (StepOp::AsSlice | StepOp::AsStr | StepOp::Offset | StepOp::Cycle))
                if matches!(op, StepOp::AsStr | StepOp::Offset)
                    || arg_tys
                        .first()
                        .is_some_and(|t| this.array_source(t.peel_refs()).is_some()) =>
            {
                lower(move |this, _, values, out| {
                    Ok(Value::Expr({
                        let mut exprs = values
                            .map(|v| this.value_expr(v, span))
                            .collect::<R<Vec<_>>>()?
                            .into_iter();
                        let it = exprs.next().expect("the iterator");
                        match op {
                            // A string's `chars()`' rest, as a string, and its
                            // `char_indices()`', both `$chars`' own `rest()`
                            // (#118, #1186).
                            StepOp::AsStr => Expr::call_helper("$restStr", vec![it]),
                            StepOp::Offset => Expr::call_helper("$charIndicesOffset", vec![it]),
                            // Of items it borrows, or whose clone and drop
                            // don't show: each round of what it has left, a
                            // clone of each it owns that's told apart from it
                            // (`$cycle`).
                            StepOp::Cycle => {
                                let iter_ty = arg_tys[0].peel_refs();
                                let owns = this.array_source(iter_ty).expect("matched");
                                let it = match it.reads_same() || is_stepped_items(&it) {
                                    true => it,
                                    false => this.spill(Role::It, it, out),
                                };
                                let mut args = vec![this.stepped_left(it, iter_ty, false, span)?];
                                let item = this.iterator_item(iter_ty).expect("an iterator's item");
                                if owns && this.needs_clone(item) {
                                    args.push(this.clone_fn("item", item, span)?);
                                }
                                Expr::call_helper("$cycle", args)
                            }
                            _ => {
                                let it = if it.reads_same() {
                                    it
                                } else {
                                    this.spill(Role::It, it, out)
                                };
                                this.stepped_left(it, arg_tys[0].peel_refs(), false, span)?
                            }
                        }
                    }))
                })
            }
            // A standard stream's, `writeln!(io::stderr(), ..)`.
            Std::Stream(op) => lower(move |this, state, values, out| {
                Ok(Value::Expr({
                    let exprs = values.map(|v| this.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                    this.flush(state, out)?;
                    this.stream_values(op, exprs, span, out)
                }))
            }),
            // std's `size_hint()`, of an iterator of the crate's that keeps it:
            // `(0, None)` (ADR 0170).
            Std::SizeHint(false) => lower(move |this, state, values, out| {
                Ok(Value::Expr({
                    let it = this.value_expr(values.next().expect("the iterator"), span)?;
                    if it.has_effects() {
                        this.flush(state, out)?;
                        out.push(StmtKind::Expr(it).at(this.js_span(span)));
                    }
                    Expr::array(vec![Expr::int(0), Expr::undefined()])
                }))
            }),
            _ => return None,
        })
    }
}
