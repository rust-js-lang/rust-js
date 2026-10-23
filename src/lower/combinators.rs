//! `Option`'s and `Result`'s combinators, more iterator adapters, and more
//! of `Vec`'s methods (ADR 0062). A closure whose body is one value is
//! written in place, as `Option::map`'s is: `o ?? f()` is `o ?? 0` for
//! `unwrap_or_else(|| 0)`.

mod adapters;
mod callbacks;
mod values;

use self::adapters::*;

use super::calls::apply;
use super::drops::Drops;
use super::recognition::trait_method;
use super::std_types::result;
use super::{FnCx, R, representation::Num};
use crate::js::{self, Expr, Op, Prop, Stmt, StmtKind};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

/// The variant of its subject on which a std call leaves an argument it's
/// given unused: a function unrun, `map`'s of a `None`, `unwrap_or_else`'s
/// of a `Some`; a value unkept, `unwrap_or`'s of a `Some`.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Unused {
    None,
    Some,
    Ok,
    Err,
    /// `bool::then`'s and `then_some`'s, of `false`.
    False,
}

impl Unused {
    /// What its subject's called, held to be tested.
    pub(super) fn subject(self) -> &'static str {
        match self {
            Unused::None | Unused::Some => "option",
            Unused::Ok | Unused::Err => "result",
            Unused::False => "condition",
        }
    }
}

/// The arguments `known` is given, each with the variant of its subject it
/// leaves it unused on, where std drops it, with what it holds: a function
/// unrun (ADR 0469), a value unkept, `unwrap_or`'s default (ADR 0490). A
/// function it runs drops what it holds as it ends. Of an `Option`, a
/// `Result` or a `bool` it takes by value, tested once it's done; what
/// writes a place, `get_or_insert_with`, or a map's entry, `or_insert_with`,
/// drops it where it branches.
pub(super) fn unused_arguments(known: super::recognition::Std) -> &'static [(usize, Unused)] {
    use super::recognition::Std;
    match known {
        Std::OptionMap | Std::Comb(Comb::AndThen | Comb::Filter | Comb::IsSomeAnd | Comb::IsNoneOr) => {
            &[(1, Unused::None)]
        }
        Std::Comb(Comb::MapOr) => &[(2, Unused::None)],
        Std::UnwrapOr | Std::Comb(Comb::UnwrapOrElse | Comb::OkOr | Comb::OkOrElse | Comb::Or | Comb::OrElse) => {
            &[(1, Unused::Some)]
        }
        Std::Comb(Comb::Then | Comb::ThenSome) => &[(1, Unused::False)],
        Std::Comb(Comb::MapOrElse) => &[(1, Unused::Some), (2, Unused::None)],
        Std::Comb(Comb::ResultMap | Comb::ResultAndThen | Comb::IsOkAnd) => &[(1, Unused::Err)],
        Std::Comb(Comb::ResultMapOr) => &[(2, Unused::Err)],
        Std::ResultOr | Std::Comb(Comb::ResultOr) => &[(1, Unused::Ok)],
        Std::Comb(Comb::And) => &[(1, Unused::None)],
        Std::Comb(Comb::ResultAnd) => &[(1, Unused::Err)],
        Std::Comb(Comb::MapErr | Comb::ResultUnwrapOrElse | Comb::IsErrAnd | Comb::ResultOrElse) => &[(1, Unused::Ok)],
        Std::Comb(Comb::ResultMapOrElse) => &[(1, Unused::Ok), (2, Unused::Err)],
        _ => &[],
    }
}

/// An `Option`, `Result` or `Vec` method (ADR 0062).
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Comb {
    UnwrapOrElse,
    UnwrapOrDefault,
    MapOr,
    MapOrElse,
    AndThen,
    Filter,
    OkOr,
    OkOrElse,
    Or,
    OrElse,
    IsSomeAnd,
    IsNoneOr,
    ResultMap,
    MapErr,
    /// A `Result`'s `map_or(d, f)` and `map_or_else(g, f)`: `f` of an `Ok`'s
    /// value, else `d`, or `g` of the `Err`'s.
    ResultMapOr,
    ResultMapOrElse,
    ResultAndThen,
    ResultUnwrapOrElse,
    ResultUnwrapOrDefault,
    Err,
    IsOkAnd,
    IsErrAnd,
    /// An `Option`'s `and`, `xor`, `zip`, `unzip`, `transpose`, `inspect`,
    /// `map_or_default` and `as_slice` (ADR 0326).
    And,
    Xor,
    Zip,
    Unzip,
    OptionTranspose,
    Inspect,
    MapOrDefault,
    AsSlice,
    /// An `Option`'s or a `Result`'s `as_deref()` of an `Rc` or of a type
    /// with its own `Deref`: what it points at, or its `deref()`.
    AsDeref,
    /// A `Result`'s `and`, `or`, `or_else`, `flatten`, `inspect` and
    /// `inspect_err`, `iter`, `map_or_default`, `transpose`, and `cloned`
    /// and `copied` (ADR 0326).
    ResultAnd,
    ResultOr,
    ResultOrElse,
    ResultFlatten,
    ResultInspect {
        err: bool,
    },
    ResultIter,
    /// `iter_mut()` and `as_mut()` of a `Result`: a `&mut` to what's in it,
    /// a handle on `r._0` of a number or text (ADR 0343).
    ResultIterMut,
    ResultAsMut,
    ResultMapOrDefault,
    ResultTranspose,
    ResultCloned,
    Contains,
    BinarySearch,
    /// `binary_search_by(f)`, and `_by_key(&b, f)`: by a comparison.
    BinarySearchBy,
    BinarySearchByKey,
    /// `rotate_left(n)`, or `rotate_right(n)`, and what std's assertion
    /// calls `n`: a slice's `mid` or `k`, a `VecDeque`'s `n`.
    Rotate {
        left: bool,
        count: &'static str,
    },
    SplitOff,
    /// `b.then(|| x)` and `b.then_some(x)`: `b ? x : undefined`.
    Then,
    ThenSome,
    Extend,
    /// `v.extend_from_slice(&s)`: a clone of each of `s`'s items.
    ExtendFromSlice,
    Insert,
    Remove,
    Swap,
    Truncate,
    Dedup,
    /// `dedup_by`, `dedup_by_key`, `swap_remove`, `resize`, `resize_with`
    /// and `pop_if` of a `Vec` (ADR 0315).
    DedupBy,
    DedupByKey,
    SwapRemove,
    Resize,
    ResizeWith,
    PopIf,
    Windows,
    Chunks,
    Concat,
}

/// An iterator adapter or consumer (ADR 0062), over an array or a JS iterator.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum IterComb {
    FilterMap,
    /// `map_while(f)`: `map(f)`, then its `Some`s till the first `None`,
    /// as `take_while` takes them (ADR 0515).
    MapWhile,
    /// `scan(init, |acc, x| ..)`: the state in a box, which the closure's
    /// `&mut` parameter is (ADR 0074).
    Scan,
    FlatMap,
    Flatten,
    Zip,
    Chain,
    TakeWhile,
    SkipWhile,
    StepBy,
    MaxByKey(bool),
    MaxBy(bool),
    Product,
    Nth,
    FindMap,
    Partition,
    /// `unzip()` into two `Vec`s: `$unzip(pairs)`.
    Unzip,
    /// `inspect(f)`: `map((item) => { f(item); return item; })` (ADR 0136).
    Inspect,
}

impl IterComb {
    /// A lazy adapter that holds what it steps, and what it's given to step
    /// after, `zip`'s and `chain`'s: an owning one given it is `$closing`,
    /// and it drops each it discards as it discards it (ADR 0465); and
    /// `flat_map`'s and `flatten`'s, which hold the inner one they step too,
    /// closed as well (ADR 0510); and `filter_map`'s (ADR 0518).
    pub(in crate::lower) fn holds_its_sources(self) -> bool {
        matches!(
            self,
            IterComb::Zip
                | IterComb::Chain
                | IterComb::FlatMap
                | IterComb::Flatten
                | IterComb::TakeWhile
                | IterComb::MapWhile
                | IterComb::Scan
                | IterComb::FilterMap
                | IterComb::SkipWhile
                | IterComb::StepBy
                | IterComb::Inspect
        )
    }

    /// Whether it leaves no item with a destructor undropped (ADR 0098): it
    /// moves each it takes on, `partition`'s into one of its two (ADR 0445),
    /// `find_map`'s to its function; or it drops each it passes over,
    /// `max_by`'s and `min_by`'s, by a key or not. `filter_map`'s, an adapter
    /// that holds its source, is `holds_its_sources`' (ADR 0518).
    pub(in crate::lower) fn leaves_none(self) -> bool {
        matches!(
            self,
            IterComb::Partition | IterComb::FindMap | IterComb::MaxBy(_) | IterComb::MaxByKey(_)
        )
    }
}

/// One of std's iterator sources (ADR 0128): `once` and `empty` are
/// arrays, and the rest, which may never end, JS iterators.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum IterSource {
    Once,
    Empty,
    Repeat,
    RepeatN,
    RepeatWith,
    Successors,
    FromFn,
}

/// Stepping through an iterator (ADR 0071): a local that `next()` is called
/// on is a `$iter` object, `{ items, at, end }`; a `Peekable` std's struct,
/// `{ iter, peeked }`, a JS iterator, whose own methods are rewritten over
/// it (ADR 0561).
#[derive(Clone, Copy, PartialEq)]
pub(super) enum StepOp {
    Next,
    Peek,
    /// A `Peekable`'s `peek_mut`: the item, or a handle on it (ADR 0513).
    PeekMut,
    NextIf,
    NextIfEq,
    /// `Chars::as_str`: the rest, as a string.
    AsStr,
    /// A slice's or a `Vec`'s iterator's `as_slice`: the rest.
    AsSlice,
    /// `cycle()` of std's iterator over an array: the rest, gone over
    /// again and again, as a clone of it is the rest (ADR 0181).
    Cycle,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// An `Option`'s or a `Result`'s combinator, of its arguments' values,
    /// `tys` their types (ADR 0364).
    /// `made`: whether the subject was just made by a call, so no other
    /// variable shares it.
    pub(in crate::lower) fn comb_values(
        &mut self,
        comb: Comb,
        (mut values, tys): (Vec<Expr>, &[Ty<'tcx>]),
        made: bool,
        generic_args: ty::GenericArgsRef<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let subject_ty = tys[0].peel_refs();
        // An `Option` whose `Some` may be boxed (ADR 0051).
        let boxed = self
            .option_of(subject_ty)
            .is_some_and(|inner| self.boxed_payload(inner));
        if let Comb::Then | Comb::ThenSome = comb {
            let some = generic_args.type_at(0);
            let (test, value) = (values.remove(0), values.remove(0));
            let value = if comb == Comb::Then {
                self.call_with(value, Vec::new(), "value", out)
            } else if value.has_effects() {
                // Rust works it out either way.
                self.spill("value", value, out)
            } else {
                value
            };
            // Boxed where it looks like `None` (ADR 0051).
            let value = if self.boxed_payload(some) {
                self.some(value)
            } else {
                value
            };
            return Ok(Expr::cond(test, value, Expr::undefined()));
        }
        // `map_err(|e| e.to_string())` of a parse error, whose message is
        // already a string: the same `Result`. Only one just made, so no
        // other variable is left sharing it.
        if matches!(comb, Comb::ResultMap | Comb::MapErr) && values[1].is_identity() && made {
            return Ok(values.remove(0));
        }
        let subject = values.remove(0);
        if let Some(value) = self.made_in_place(comb, boxed, subject_ty, &subject, &values) {
            return Ok(value);
        }
        // A slice's `concat` of parts written out, `[path, &[last]].concat()`:
        // an array of them, spread, `[...path, ...[last]]`.
        if comb == Comb::Concat
            && let js::ExprKind::Array(parts) = &subject.kind
            && !self
                .slice_item(subject_ty)
                .is_some_and(|item| self.is_string_like(item))
        {
            let items = parts.iter().map(|part| Expr::spread(part.clone())).collect();
            return self.concat_copies(Expr::array(items), subject_ty, span);
        }
        if let Some(value) = self.read_once(comb, boxed, subject_ty, &subject, &mut values, span, out)? {
            return Ok(value);
        }
        // The subject is read more than once.
        let spilled_at = (!subject.reads_same()).then_some(out.len());
        let subject = if subject.reads_same() {
            subject
        } else {
            let base = if self.option_of(subject_ty).is_some() {
                "option"
            } else {
                "result"
            };
            self.spill(base, subject, out)
        };
        let mut rest = values.into_iter();
        let mut next = || rest.next().expect("rustc checked the arguments");
        let some = Expr::bin(Op::LooseNe, subject.clone(), Expr::null());
        let none = || Expr::bin(Op::LooseEq, subject.clone(), Expr::null());
        let tag = |ok: bool| result::is(subject.clone(), ok);
        let inside = || result::held(subject.clone());
        // What a `Some` holds: the value, or what's in its box.
        let value = if boxed {
            self.some_value(subject.clone())
        } else {
            subject.clone()
        };
        // A value Rust computes either way, that JS would only compute when
        // it's needed: in a `const` first if it has effects.
        let eager = |this: &mut Self, value: Expr, out: &mut Vec<Stmt>| {
            if value.has_effects() {
                this.spill("fallback", value, out)
            } else {
                value
            }
        };
        // One read more than once: in a `const` first unless it reads the same.
        let once = |this: &mut Self, value: Expr, name: &str, out: &mut Vec<Stmt>| match value.reads_same() {
            true => value,
            false => this.spill(name, value, out),
        };
        let helper = |name: &'static str, list: Vec<Expr>| Expr::call(Expr::helper(name), list);
        Ok(match comb {
            Comb::Then | Comb::ThenSome | Comb::OrElse => unreachable!("handled above"),
            // `??` would give a box for the value in it.
            Comb::UnwrapOrElse => {
                let f = next();
                let fallback = self.call_with(f, Vec::new(), "fallback", out);
                match boxed {
                    true => Expr::cond(some, value, fallback),
                    false => Expr::bin(Op::Coalesce, subject, fallback),
                }
            }
            // A boxed `Some`'s; one not boxed is `??`, above.
            Comb::UnwrapOrDefault => {
                let inner = self.option_of(subject_ty).expect("an `Option`");
                let fallback = self.default_value(inner, span)?;
                Expr::cond(some, value, fallback)
            }
            // A fallback with a destructor, unused, is dropped once `f` has
            // run, or thrown, as Rust drops an argument (ADR 0179).
            Comb::MapOr if self.drops(tys[1]) != Drops::Nothing => {
                let fallback = next();
                let fallback = match fallback.kind {
                    js::ExprKind::Var(_) => fallback,
                    _ => self.spill("fallback", fallback, out),
                };
                let f = next();
                let mut body = Vec::new();
                let mapped = self.call_with(f, vec![value], "map", &mut body);
                let result = self.fresh("mapped");
                body.push(StmtKind::Assign(Expr::var(&result), mapped).at(js::Span::NONE));
                let mut dropped = Vec::new();
                self.drop_value(fallback.clone(), tys[1], span, &mut dropped)?;
                let js_span = self.js_span(span);
                out.push(StmtKind::Let(result.clone(), None).at(js_span));
                out.push(
                    StmtKind::If(
                        some,
                        vec![StmtKind::Try(body, dropped).at(js_span)],
                        Some(vec![StmtKind::Assign(Expr::var(&result), fallback).at(js_span)]),
                    )
                    .at(js_span),
                );
                Expr::var(&result)
            }
            Comb::MapOr => {
                let fallback = eager(self, next(), out);
                let f = next();
                let mapped = self.call_with(f, vec![value], "map", out);
                // `e?.[1] ?? d`, as a `match` of it is (ADR 0309).
                let option = match spilled_at {
                    Some(at) if out.len() == at + 1 => match &out[at].kind {
                        StmtKind::Const(name, option) => Some((name.clone(), option.clone(), true)),
                        _ => None,
                    },
                    Some(_) => None,
                    None => match &subject.kind {
                        js::ExprKind::Var(name) => Some((name.clone(), subject.clone(), false)),
                        _ => None,
                    },
                };
                let never_nullish = !self.can_be_nullish(generic_args.type_at(1));
                match option.filter(|_| !boxed).and_then(|(name, option, spilled)| {
                    let nullish =
                        super::std_types::option::nullish_or(&name, &option, &some, &mapped, &fallback, never_nullish)?;
                    Some((nullish, spilled))
                }) {
                    Some((nullish, spilled)) => {
                        if spilled {
                            out.pop();
                        }
                        nullish
                    }
                    None => Expr::cond(some, mapped, fallback),
                }
            }
            Comb::MapOrElse => {
                let (g, f) = (next(), next());
                let mapped = self.call_with(f, vec![value], "map", out);
                let fallback = self.call_with(g, Vec::new(), "fallback", out);
                Expr::cond(some, mapped, fallback)
            }
            // An `Err` with a destructor is dropped, and a fallback with one
            // once `f` has run, or thrown (ADR 0179).
            Comb::ResultMapOr
                if self.drops(generic_args.type_at(1)) != Drops::Nothing || self.drops(tys[1]) != Drops::Nothing =>
            {
                let fallback_ty = tys[1];
                let fallback = next();
                let fallback = match fallback.kind {
                    js::ExprKind::Var(_) => fallback,
                    _ => self.spill("fallback", fallback, out),
                };
                let f = next();
                let name = self.fresh("mapped");
                let mut body = Vec::new();
                let mapped = self.call_with(f, vec![inside()], "map", &mut body);
                body.push(StmtKind::Assign(Expr::var(&name), mapped).at(js::Span::NONE));
                let mut unused = Vec::new();
                self.drop_value(fallback.clone(), fallback_ty, span, &mut unused)?;
                let then = match unused.is_empty() {
                    true => body,
                    false => vec![StmtKind::Try(body, unused).at(self.js_span(span))],
                };
                let error = (inside(), generic_args.type_at(1));
                self.or_dropped(tag(true), name, then, error, fallback, span, out)?
            }
            Comb::ResultMapOr => {
                let fallback = eager(self, next(), out);
                let f = next();
                let mapped = self.call_with(f, vec![inside()], "map", out);
                Expr::cond(tag(true), mapped, fallback)
            }
            Comb::ResultMapOrElse => {
                let (g, f) = (next(), next());
                let mapped = self.call_with(f, vec![inside()], "map", out);
                let fallback = self.call_with(g, vec![inside()], "fallback", out);
                Expr::cond(tag(true), mapped, fallback)
            }
            Comb::AndThen => {
                let f = next();
                let then = self.call_with(f, vec![value], "then", out);
                Expr::cond(some, then, Expr::undefined())
            }
            // A value with a destructor its test doesn't keep is dropped once
            // the test says so, or throws (ADR 0179).
            Comb::Filter if self.drops(self.option_of(subject_ty).expect("an `Option`")) != Drops::Nothing => {
                let inner = self.option_of(subject_ty).expect("an `Option`");
                let p = next();
                let mut body = Vec::new();
                let keep = self.call_with(p, vec![value.clone()], "keep", &mut body);
                let kept = self.fresh("kept");
                let js_span = self.js_span(span);
                body.push(
                    StmtKind::If(
                        Expr::bin(Op::And, some.clone(), keep),
                        vec![StmtKind::Assign(Expr::var(&kept), subject).at(js_span)],
                        None,
                    )
                    .at(js_span),
                );
                let mut drop = Vec::new();
                self.drop_value(value, inner, span, &mut drop)?;
                let unkept = Expr::bin(Op::Eq, Expr::var(&kept), Expr::undefined());
                let dropped = vec![StmtKind::If(Expr::bin(Op::And, some, unkept), drop, None).at(js_span)];
                out.push(StmtKind::Let(kept.clone(), None).at(js_span));
                out.push(StmtKind::Try(body, dropped).at(js_span));
                Expr::var(&kept)
            }
            Comb::Filter => {
                let p = next();
                let keep = self.call_with(p, vec![value], "keep", out);
                Expr::cond(Expr::bin(Op::And, some, keep), subject, Expr::undefined())
            }
            Comb::OkOr => {
                let e = eager(self, next(), out);
                Expr::cond(some, result::ok(value), result::err(e))
            }
            Comb::OkOrElse => {
                let f = next();
                let e = self.call_with(f, Vec::new(), "error", out);
                Expr::cond(some, result::ok(value), result::err(e))
            }
            Comb::Or => {
                let other = eager(self, next(), out);
                Expr::bin(Op::Coalesce, subject, other)
            }
            Comb::IsSomeAnd => {
                let p = next();
                let holds = self.call_with(p, vec![value], "holds", out);
                Expr::bin(Op::And, some, holds)
            }
            Comb::IsNoneOr => {
                let p = next();
                let holds = self.call_with(p, vec![value], "holds", out);
                Expr::bin(Op::Or, none(), holds)
            }
            Comb::ResultMap => {
                let f = next();
                let mapped = self.call_with(f, vec![inside()], "map", out);
                Expr::cond(tag(true), result::ok(mapped), subject)
            }
            Comb::MapErr => {
                let f = next();
                let mapped = self.call_with(f, vec![inside()], "map", out);
                Expr::cond(tag(false), result::err(mapped), subject)
            }
            Comb::ResultAndThen => {
                let f = next();
                let then = self.call_with(f, vec![inside()], "then", out);
                Expr::cond(tag(true), then, subject)
            }
            Comb::ResultUnwrapOrElse => {
                let f = next();
                let fallback = self.call_with(f, vec![inside()], "fallback", out);
                Expr::cond(tag(true), inside(), fallback)
            }
            Comb::ResultUnwrapOrDefault => {
                let fallback = self.default_value(generic_args.type_at(0), span)?;
                let given_up = (inside(), generic_args.type_at(1));
                self.kept_or(tag(true), inside(), given_up, fallback, span, out)?
            }
            // An `Ok` with a destructor is dropped (ADR 0179). Its `Some` is
            // boxed where the error looks like `None`, `()` say, as `ok()`'s
            // is (ADR 0051).
            Comb::Err if self.drops(generic_args.type_at(0)) != Drops::Nothing => {
                let name = self.fresh("error");
                let error = self.held_in_option(inside(), generic_args.type_at(1));
                let then = vec![StmtKind::Assign(Expr::var(&name), error).at(js::Span::NONE)];
                let value = (inside(), generic_args.type_at(0));
                self.or_dropped(tag(false), name, then, value, Expr::undefined(), span, out)?
            }
            Comb::Err => {
                let error = self.held_in_option(inside(), generic_args.type_at(1));
                Expr::cond(tag(false), error, Expr::undefined())
            }
            // Rust works out `b` either way: in a `const` first if it has effects.
            // A `Some`'s value `and` gives up is dropped (ADR 0496).
            Comb::And => {
                let other = eager(self, next(), out);
                let given_up = (value.clone(), generic_args.type_at(0));
                self.kept_or(none(), Expr::undefined(), given_up, other, span, out)?
            }
            // The one that's `Some`, where only one is.
            Comb::Xor | Comb::Zip => {
                let other = once(self, next(), "other", out);
                let own = (subject.clone(), value.clone(), generic_args.type_at(0));
                self.unpaired(comb == Comb::Xor, own, (other, tys[1]), span, out)?
            }
            // A pair of `Option`s, each boxed where it looks like `None`.
            Comb::Unzip => {
                let ty::Tuple(parts) = self.option_of(subject_ty).expect("an `Option`").kind() else {
                    return Err(self.unsupported(span, "`unzip` of this"));
                };
                let parts: Vec<Ty<'tcx>> = parts.to_vec();
                let mut halves = Vec::new();
                for (i, part) in parts.into_iter().enumerate() {
                    let half = Expr::index(value.clone(), Expr::int(i as i128));
                    halves.push(self.held_in_option(half, part));
                }
                Expr::cond(
                    some,
                    Expr::array(halves),
                    Expr::array(vec![Expr::undefined(), Expr::undefined()]),
                )
            }
            Comb::OptionTranspose => {
                let result = self.option_of(subject_ty).expect("an `Option`");
                let ok_ty = match result.kind() {
                    ty::Adt(_, result) => result.type_at(0),
                    _ => return Err(self.unsupported(span, "`transpose` of this")),
                };
                let ok_value = result::held(value.clone());
                let ok_value = self.held_in_option(ok_value, ok_ty);
                let is_ok = result::is(value, true);
                let absent = none();
                let inner = Expr::cond(is_ok, result::ok(ok_value), subject);
                Expr::cond(absent, result::ok(Expr::undefined()), inner)
            }
            Comb::Inspect | Comb::ResultInspect { .. } => {
                let f = next();
                let (test, given) = match comb {
                    Comb::Inspect => (some, value),
                    Comb::ResultInspect { err } => (tag(!err), inside()),
                    _ => unreachable!("an inspection"),
                };
                let mut call = Vec::new();
                let called = self.call_with(f, vec![given], "inspected", &mut call);
                call.push(StmtKind::Expr(called).at(self.js_span(span)));
                out.push(StmtKind::If(test, call, None).at(self.js_span(span)));
                subject
            }
            // `map_or_default<U, F>`: `U`'s default where there's nothing to map.
            Comb::MapOrDefault | Comb::ResultMapOrDefault => {
                let f = next();
                let (test, given, mapped_ty) = match comb {
                    Comb::MapOrDefault => (some, value, generic_args.types().nth(1)),
                    _ => (tag(true), inside(), generic_args.types().nth(2)),
                };
                let fallback = self.default_value(mapped_ty.expect("its result's type"), span)?;
                let mapped = self.call_with(f, vec![given], "map", out);
                Expr::cond(test, mapped, fallback)
            }
            Comb::AsSlice => Expr::cond(some, Expr::array(vec![value]), Expr::array(Vec::new())),
            Comb::AsDeref => {
                let option = self.option_of(subject_ty);
                let inner = match (option, subject_ty.kind()) {
                    (Some(inner), _) => inner,
                    (None, ty::Adt(_, sides)) => sides.type_at(0),
                    _ => return Err(self.unsupported(span, "`as_deref` of this")),
                };
                // An `Rc` that isn't counted is what it points at (ADR 0023).
                if self.is_rc(inner) && self.counted_rc(inner).is_none() {
                    return Ok(subject);
                }
                let given = if option.is_some() { value } else { inside() };
                let pointee = self.deref_value(given, inner, span, out)?;
                match option {
                    Some(_) => Expr::cond(some, pointee, Expr::undefined()),
                    None => Expr::cond(tag(true), result::ok(pointee), subject),
                }
            }
            // An `Ok`'s value `and` gives up, or an `Err`'s `or` does, is
            // dropped (ADRs 0179, 0490, 0496).
            Comb::ResultAnd | Comb::ResultOr => {
                let other = eager(self, next(), out);
                let ok = comb == Comb::ResultOr;
                let given_up = (inside(), generic_args.type_at(usize::from(ok)));
                self.kept_or(tag(ok), subject.clone(), given_up, other, span, out)?
            }
            Comb::ResultOrElse => {
                let f = next();
                let other = self.call_with(f, vec![inside()], "fallback", out);
                Expr::cond(tag(true), subject, other)
            }
            Comb::ResultFlatten => Expr::cond(tag(true), inside(), subject),
            Comb::ResultIter => Expr::cond(tag(true), Expr::array(vec![inside()]), Expr::array(Vec::new())),
            Comb::ResultIterMut | Comb::ResultAsMut => {
                let ty::Adt(_, sides) = subject_ty.peel_refs().kind() else {
                    return Err(self.unsupported(span, "this `Result`"));
                };
                let (ok_ty, err_ty) = (sides.type_at(0), sides.type_at(1));
                let part = |this: &Self, ty: Ty<'tcx>| match this.is_boxable(ty) {
                    true => Expr::handle(inside()),
                    false => inside(),
                };
                let ok = part(self, ok_ty);
                match comb {
                    Comb::ResultIterMut => Expr::cond(tag(true), Expr::array(vec![ok]), Expr::array(Vec::new())),
                    // Of objects, the `Result` itself, as `as_ref()`'s is.
                    _ if !self.is_boxable(ok_ty) && !self.is_boxable(err_ty) => subject,
                    _ => {
                        // Of both, a handle on `r._0` whichever it is.
                        if self.is_boxable(ok_ty) && self.is_boxable(err_ty) {
                            result::tagged(result::tag(subject.clone()), ok)
                        } else {
                            let err = part(self, err_ty);
                            Expr::cond(tag(true), result::ok(ok), result::err(err))
                        }
                    }
                }
            }
            // `Ok(None)` is `None`, `Ok(Some(x))` `Some(Ok(x))`, `Err(e)` `Some(Err(e))`.
            Comb::ResultTranspose => {
                let inner = match subject_ty.kind() {
                    ty::Adt(_, result) => self.option_of(result.type_at(0)).expect("an `Option`"),
                    _ => return Err(self.unsupported(span, "`transpose` of this")),
                };
                let x = match self.boxed_payload(inner) {
                    true => self.some_value(inside()),
                    false => inside(),
                };
                let present = Expr::bin(Op::LooseEq, inside(), Expr::null());
                Expr::cond(
                    tag(true),
                    Expr::cond(present, Expr::undefined(), result::ok(x)),
                    subject,
                )
            }
            // A clone of an `Ok`'s value where a clone is more than the value.
            Comb::ResultCloned => {
                let item = generic_args.types().next().expect("its value's type");
                match self.needs_clone(item) {
                    true => {
                        let clone = self.clone_value(inside(), item, span, out)?;
                        Expr::cond(tag(true), result::ok(clone), subject)
                    }
                    false => subject,
                }
            }
            // The other variant's value, with a destructor, is dropped (ADR 0179).
            Comb::IsOkAnd | Comb::IsErrAnd
                if self.drops(generic_args.type_at(usize::from(comb == Comb::IsOkAnd))) != Drops::Nothing =>
            {
                let p = next();
                let which = comb == Comb::IsOkAnd;
                let name = self.fresh("holds");
                let mut then = Vec::new();
                let holds = self.call_with(p, vec![inside()], "holds", &mut then);
                then.push(StmtKind::Assign(Expr::var(&name), holds).at(js::Span::NONE));
                let other = (inside(), generic_args.type_at(usize::from(comb == Comb::IsOkAnd)));
                self.or_dropped(tag(which), name, then, other, Expr::bool(false), span, out)?
            }
            Comb::IsOkAnd | Comb::IsErrAnd => {
                let p = next();
                let holds = self.call_with(p, vec![inside()], "holds", out);
                let which = comb == Comb::IsOkAnd;
                Expr::bin(Op::And, tag(which), holds)
            }
            // `v.contains(&x)`: JS's `includes` for what `===` compares, and
            // `==` item by item for the rest (ADR 0053).
            Comb::SplitOff => helper("$splitOff", vec![subject, next()]),
            Comb::BinarySearch => {
                let x = next();
                let item = self
                    .slice_item(subject_ty)
                    .ok_or_else(|| self.unsupported(span, "`binary_search` of this"))?;
                // What `<` orders as `Ord` does, `is_primitive_ord`'s.
                if self.is_primitive_ord(item) {
                    return Ok(helper("$binarySearch", vec![subject, x]));
                }
                // Any other by its `Ord`, as `sort()` orders it: strings and
                // `char`s by code point (ADR 0183), a derived one by its
                // variants and fields.
                let x = once(self, x, "item", out);
                let mut body = Vec::new();
                let order = self.cmp_value(Expr::var("each"), x, item, false, span, &mut body)?;
                body.push(StmtKind::Return(Some(order)).at(js::Span::NONE));
                helper("$binarySearchBy", vec![subject, Expr::arrow(vec!["each".into()], body)])
            }
            Comb::Contains => {
                let x = next();
                let item = self
                    .slice_item(subject_ty)
                    .ok_or_else(|| self.unsupported(span, "`contains` of this"))?;
                if self.eq_is_identity(item) {
                    Expr::call(Expr::member(subject, "includes"), vec![x])
                } else {
                    let x = once(self, x, "item", out);
                    let mut body = Vec::new();
                    let same = self.eq_value(Expr::var("each"), x, item, span, &mut body)?;
                    body.push(StmtKind::Return(Some(same)).at(js::Span::NONE));
                    let f = Expr::arrow(vec!["each".into()], body);
                    Expr::call(Expr::member(subject, "some"), vec![f])
                }
            }
            Comb::Extend => self.extended(subject, next(), (tys[1], subject_ty), span, out)?,
            // Each item a clone, as `to_vec()` makes them.
            Comb::ExtendFromSlice => {
                let items = next();
                let item = self
                    .slice_item(subject_ty)
                    .ok_or_else(|| self.unsupported(span, "`extend_from_slice` of this"))?;
                let items = if self.needs_clone(item) {
                    self.clone_items(items, item, span)?
                } else {
                    items
                };
                helper("$extend", vec![subject, items])
            }
            Comb::BinarySearchBy => helper("$binarySearchBy", vec![subject, next()]),
            // `f(item)` compared with `b`, by its type's `Ord`.
            Comb::BinarySearchByKey => {
                let (key, f) = (next(), next());
                let key = once(self, key, "key", out);
                let key_ty = generic_args
                    .types()
                    .nth(1)
                    .expect("`binary_search_by_key` has a key type");
                let compare = self.cmp_fn(key_ty, false, span)?;
                let item = Expr::var("item");
                let mapped = apply(f, vec![item]);
                let body = vec![StmtKind::Return(Some(apply(compare, vec![mapped, key]))).at(js::Span::NONE)];
                let by = Expr::arrow(vec!["item".into()], body);
                helper("$binarySearchBy", vec![subject, by])
            }
            Comb::Rotate { left, count } => {
                let name = if left { "$rotateLeft" } else { "$rotateRight" };
                helper(name, vec![subject, next(), Expr::str(count)])
            }
            Comb::Insert => inserted(subject, next(), next()),
            Comb::Remove => helper("$removeAt", vec![subject, next()]),
            Comb::Swap => helper("$swap", self.deque_marked(vec![subject, next(), next()], subject_ty)),
            Comb::Truncate => helper("$truncate", self.dropping(vec![subject, next()], subject_ty, span)?),
            Comb::Dedup => self.dedup_call(subject, subject_ty, span)?,
            // Its closure takes `&mut` items: an object itself, or a cell of a
            // number or text, whose change is written back, as std's is.
            Comb::DedupBy | Comb::DedupByKey | Comb::PopIf
                if self
                    .slice_item(subject_ty)
                    .is_none_or(|item| !self.is_object(item) && !self.is_boxable(item)) =>
            {
                return Err(self.unsupported(span, "this call, of items a `&mut` can't be given to its closure"));
            }
            Comb::DedupBy => {
                let name = self.by_cells(subject_ty, "$dedupBy", "$dedupByCells");
                helper(name, self.dropping(vec![subject, next()], subject_ty, span)?)
            }
            // Its key's `==`, each item's key made once a comparison, the later
            // item's first, as std's are: named first where `==` reads it more
            // than once, of what `===` doesn't compare.
            Comb::DedupByKey => {
                let key_ty = generic_args.type_at(generic_args.len() - 1);
                let key = next();
                let key = once(self, key, "key", out);
                let (a, b) = (self.fresh("a"), self.fresh("b"));
                let mut body = Vec::new();
                let mut keyed = |item: &str, named: &str, body: &mut Vec<Stmt>| {
                    let made = Expr::call(key.clone(), vec![Expr::var(item)]);
                    match self.eq_is_identity(key_ty) {
                        true => made,
                        false => self.spill(named, made, body),
                    }
                };
                let (ka, kb) = (keyed(&a, "aKey", &mut body), keyed(&b, "bKey", &mut body));
                let same = self.eq_value(ka, kb, key_ty, span, &mut body)?;
                body.push(StmtKind::Return(Some(same)).at(js::Span::NONE));
                let same = Expr::arrow(vec![a.into(), b.into()], body);
                let name = self.by_cells(subject_ty, "$dedupBy", "$dedupByCells");
                helper(name, self.dropping(vec![subject, same], subject_ty, span)?)
            }
            Comb::SwapRemove => helper("$swapRemove", vec![subject, next()]),
            Comb::Resize => self.resize_call(vec![subject, next(), next()], subject_ty, span)?,
            Comb::ResizeWith => helper(
                "$resizeWith",
                self.dropping(vec![subject, next(), next()], subject_ty, span)?,
            ),
            Comb::PopIf => {
                let name = self.by_cells(subject_ty, "$popIf", "$popIfCell");
                helper(name, vec![subject, next()])
            }
            Comb::Windows => helper("$windows", vec![subject, next()]),
            Comb::Chunks => helper("$chunks", vec![subject, next()]),
            // Of strings, one string; of `Vec`s or arrays, one array.
            Comb::Concat
                if self
                    .slice_item(subject_ty)
                    .is_some_and(|item| self.is_string_like(item)) =>
            {
                Expr::call(Expr::member(subject, "join"), vec![Expr::str("")])
            }
            Comb::Concat => self.concatenated(subject, subject_ty, span)?,
        })
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What a combinator is, made of its subject where it's read once: an
    /// `unwrap_or_else` of a `Result` just made, its value; and
    /// `is_some_and(|first| Rc::ptr_eq(first, &task))`, a comparison of the
    /// `Some` with a value of its type, which, its `Some` not boxed, can't
    /// be `undefined` nor `null`: `peek(queue) === task`, which a `None`
    /// fails (ADR 0591).
    fn made_in_place(
        &mut self,
        comb: Comb,
        boxed: bool,
        subject_ty: Ty<'tcx>,
        subject: &Expr,
        values: &[Expr],
    ) -> Option<Expr> {
        if let Comb::ResultUnwrapOrElse | Comb::ResultUnwrapOrDefault = comb {
            return result::unwrapped_made(subject, values);
        }
        if comb == Comb::IsSomeAnd && !boxed && self.option_of(subject_ty).is_some() {
            let compared = compared_with(&values[0])?;
            return Some((compared.substitute(&|v| (v == HELD_SOME).then(|| subject.clone()))).unwrap_or(compared));
        }
        None
    }

    /// What `comb` gives of `subject`, of type `subject_ty`, read once where
    /// it is, `None` where it's read more than once: a fallback's, `o ?? b`,
    /// and a `filter`'s by truth, `o || undefined`.
    #[allow(clippy::too_many_arguments)]
    fn read_once(
        &mut self,
        comb: Comb,
        boxed: bool,
        subject_ty: Ty<'tcx>,
        subject: &Expr,
        values: &mut Vec<Expr>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        // A `Some` not boxed is read once by `??`, and the default made only
        // where it's `None`, as Rust makes it: `get(n) ?? ""`; and `f` called
        // only there, `get(n) ?? root`, a closure of statements its arrow,
        // made first, which does nothing. `or_else`'s and `or`'s, of a box
        // too, which they give as it is: `or`'s `b`, which Rust works out
        // first, where it does nothing. Of what `filter` kept, its test
        // (`or_fallback`).
        let fallback = match comb {
            Comb::UnwrapOrDefault if !boxed => {
                let inner = self.option_of(subject_ty).expect("an `Option`");
                Some(self.default_value(inner, span)?)
            }
            Comb::UnwrapOrElse if !boxed => Some(self.call_with(values.remove(0), Vec::new(), "fallback", out)),
            Comb::OrElse => Some(self.call_with(values.remove(0), Vec::new(), "fallback", out)),
            Comb::Or if !values[0].has_effects() => Some(values.remove(0)),
            _ => None,
        };
        if let Some(fallback) = fallback {
            return Ok(Some(super::std_types::option::or_fallback(subject.clone(), fallback)));
        }
        // `filter` by the truth of what it holds, `|v| js::truthy(Some(*v))`:
        // what JS's `||` keeps of it, `o || undefined`, read once, which a
        // fallback after makes `o || b` (ADR 0635). A `Some` boxed is a box,
        // whose truth isn't its value's.
        if comb == Comb::Filter
            && !boxed
            && self.drops(self.option_of(subject_ty).expect("an `Option`")) == Drops::Nothing
            && tests_its_truth(&values[0])
        {
            return Ok(Some(Expr::bin(Op::Or, subject.clone(), Expr::undefined())));
        }
        Ok(None)
    }
}

/// What `p`, an `is_some_and`'s predicate, is of its argument where it's a
/// comparison of it with what's not it, `x === task`, the argument written
/// `HELD_SOME`: the `===`, `HELD_SOME === task`.
fn compared_with(p: &Expr) -> Option<Expr> {
    let applied = crate::lower::calls::apply(p.clone(), vec![Expr::var(HELD_SOME)]);
    let js::ExprKind::Binary(Op::Eq, a, b) = &applied.kind else {
        return None;
    };
    let held = |e: &Expr| matches!(&e.kind, js::ExprKind::Var(v) if v == HELD_SOME);
    let mentions = js::mentions_in(&[StmtKind::Expr(applied.clone()).at(applied.span)], HELD_SOME);
    ((held(a) || held(b)) && mentions == 1).then_some(applied)
}

/// Whether `p`, a `filter`'s predicate, is the truth of its argument itself,
/// `!!$some`, `js::truthy`'s of it, or a `bool`'s own: as JS's `||` tests
/// what it's given (ADR 0635).
fn tests_its_truth(p: &Expr) -> bool {
    let applied = crate::lower::calls::apply(p.clone(), vec![Expr::var(HELD_SOME)]);
    matches!(&applied.tested().kind, js::ExprKind::Var(v) if v == HELD_SOME)
}

/// The argument `compared_with` and `tests_its_truth` give a predicate: a
/// name no Rust variable has.
const HELD_SOME: &str = "$some";

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What `concat` of `parts`, of type `parts_ty`, joins, `joined`: each
    /// item a copy where it needs one (`copies_of`).
    fn concat_copies(&mut self, joined: Expr, parts_ty: Ty<'tcx>, span: Span) -> R<Expr> {
        match self.slice_item(parts_ty).and_then(|part| self.slice_item(part)) {
            Some(item) => self.copies_of(joined, item, span),
            None => Ok(joined),
        }
    }

    /// `parts.flat()`, of arrays, `concat_copies`.
    fn concatenated(&mut self, parts: Expr, parts_ty: Ty<'tcx>, span: Span) -> R<Expr> {
        self.concat_copies(Expr::call(Expr::member(parts, "flat"), Vec::new()), parts_ty, span)
    }

    /// `args`, a `VecDeque`'s, `subject_ty`, marked `true`, whose panic is
    /// its own assertion where a slice's is an index's.
    fn deque_marked(&self, mut args: Vec<Expr>, subject_ty: Ty<'tcx>) -> Vec<Expr> {
        if self.is_std_type(subject_ty.peel_refs(), crate::lower::recognition::StdItem::VecDeque) {
            args.push(Expr::bool(true));
        }
        args
    }

    /// `extend` of `items`, of type `iterable`, onto `collection`, a
    /// collection of type `into`: each pushed as it's stepped
    /// (`extended_items`); onto a heap, its order rebuilt after, as std's
    /// is (ADR 0333).
    fn extended(
        &mut self,
        collection: Expr,
        items: Expr,
        (iterable, into): (Ty<'tcx>, Ty<'tcx>),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let items = self.extended_items(items, (iterable, into), span, out)?;
        if self.is_std_type(into, crate::lower::recognition::StdItem::BinaryHeap) {
            let item = self
                .slice_item(into)
                .ok_or_else(|| self.unsupported(span, "this heap"))?;
            let cmp = self.cmp_fn(item, false, span)?;
            return Ok(Expr::call_helper("$heapExtend", vec![collection, items, cmp]));
        }
        Ok(Expr::call_helper("$extend", vec![collection, items]))
    }

    /// What `extend` of `items`, of type `iterable`, pushes onto a
    /// collection of type `into`: each as it's stepped; of an array's
    /// iterator made here, the array; of `&T`s, a copy where it needs one
    /// (`copied_into`).
    fn extended_items(
        &mut self,
        items: Expr,
        (iterable, into): (Ty<'tcx>, Ty<'tcx>),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let items = self.iterable_items(items, iterable, span, out)?;
        let items = super::iterators::fresh_iter_source(&items).cloned().unwrap_or(items);
        self.copied_into(items, self.iterable_item(iterable), self.slice_item(into), span)
    }
}
