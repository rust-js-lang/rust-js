//! `Option`'s and `Result`'s combinators, more iterator adapters, and more
//! of `Vec`'s methods (ADR 0062). A closure whose body is one value is
//! written in place, as `Option::map`'s is: `o ?? f()` is `o ?? 0` for
//! `unwrap_or_else(|| 0)`.

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
    /// and it drops each it discards as it discards it (ADR 0465).
    pub(in crate::lower) fn holds_its_sources(self) -> bool {
        matches!(
            self,
            IterComb::Zip
                | IterComb::Chain
                | IterComb::TakeWhile
                | IterComb::SkipWhile
                | IterComb::StepBy
                | IterComb::Inspect
        )
    }

    /// Whether it leaves no item with a destructor undropped (ADR 0098): it
    /// moves each it takes on, `partition`'s into one of its two (ADR 0445),
    /// `filter_map`'s and `find_map`'s to its function; or it drops each it
    /// passes over, `max_by`'s and `min_by`'s, by a key or not.
    pub(in crate::lower) fn leaves_none(self) -> bool {
        matches!(
            self,
            IterComb::Partition | IterComb::FilterMap | IterComb::FindMap | IterComb::MaxBy(_) | IterComb::MaxByKey(_)
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
    RepeatWith,
    Successors,
    FromFn,
}

/// Stepping through an iterator (ADR 0071): a `Peekable`, and a local that
/// `next()` is called on, are a `$iter` object, `{ items, at, end }`.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum StepOp {
    Next,
    Peekable,
    Peek,
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
    /// What a `value` of `ty` points at, as `*value` reads it: a counted
    /// `Rc`'s value, or the `deref()` of a type with its own `Deref`.
    fn deref_value(&mut self, value: Expr, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        // A counted `Rc`'s, its `value` (ADR 0320).
        if self.counted_rc(ty).is_some() {
            return Ok(Expr::held(value));
        }
        let deref = self.tcx.require_lang_item(LangItem::Deref, span);
        if self.has_user_impl(deref, ty) {
            let method = trait_method(self.tcx, deref, "deref");
            let args = self.args_of(deref, ty);
            if let Some(call) = self.trait_call(method, args, vec![value], span, out)? {
                return Ok(call);
            }
        }
        Err(self.unsupported(span, &format!("`as_deref` of a `{ty}`")))
    }

    /// `vec![item; n]`, of their values (ADR 0364).
    pub(in crate::lower) fn copies_values(
        &mut self,
        (item, item_ty): (Expr, Ty<'tcx>),
        n: Expr,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        if !self.needs_clone(item_ty) {
            let array = Expr::new_(Expr::var("Array"), vec![n]);
            return Ok(Expr::call(Expr::member(array, "fill"), vec![item]));
        }
        // A hand-written Clone (including one in a field) can change values
        // and run effects. Clone n - 1 times, then move the original item.
        if !self.structural_clone(item_ty) {
            let name = self.fresh("item");
            let mut body = Vec::new();
            let copy = self.clone_value(Expr::var(&name), item_ty, span, &mut body)?;
            body.push(StmtKind::Return(Some(copy)).at(js::Span::NONE));
            return Ok(Expr::call(
                Expr::helper("$repeat"),
                vec![item, n, Expr::arrow(vec![name.into()], body)],
            ));
        }
        let item = if item.reads_same() {
            item
        } else {
            self.spill("item", item, out)
        };
        let mut body = Vec::new();
        let copy = self.clone_value(item, item_ty, span, &mut body)?;
        body.push(StmtKind::Return(Some(copy)).at(js::Span::NONE));
        let length = Expr::object(vec![Prop::Field("length".into(), n)]);
        let from = Expr::member(Expr::var("Array"), "from");
        Ok(Expr::call(from, vec![length, Expr::arrow(Vec::new(), body)]))
    }

    /// `f(args)`, with a closure that only returns written in place. One of
    /// statements gets a name first: `const f = (x) => { .. }; f(o)`.
    fn call_with(&mut self, f: Expr, args: Vec<Expr>, name: &str, out: &mut Vec<Stmt>) -> Expr {
        let args: Vec<Expr> = args
            .into_iter()
            .map(|a| if a.reads_same() { a } else { self.spill("value", a, out) })
            .collect();
        let applied = super::calls::apply(f.clone(), args.clone());
        match &applied.kind {
            js::ExprKind::Call(callee, _) if matches!(callee.kind, js::ExprKind::Arrow(..)) => {
                let f = self.spill(name, f, out);
                Expr::call(f, args)
            }
            _ => applied,
        }
    }

    /// `name`, set by `then` where `test` holds, and otherwise to `otherwise`
    /// once `given_up`, the other variant's value std drops, is dropped
    /// (ADR 0179): `let holds; if (r.TAG === "Ok") { holds = f(r._0); } else
    /// { eDrop_drop(r._0); holds = false; }`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn or_dropped(
        &mut self,
        test: Expr,
        name: String,
        then: Vec<Stmt>,
        given_up: (Expr, Ty<'tcx>),
        otherwise: Expr,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let js_span = self.js_span(span);
        let mut dropped = Vec::new();
        self.drop_value(given_up.0, given_up.1, span, &mut dropped)?;
        dropped.push(StmtKind::Assign(Expr::var(&name), otherwise).at(js_span));
        out.push(StmtKind::Let(name.clone(), None).at(js_span));
        out.push(StmtKind::If(test, then, Some(dropped)).at(js_span));
        Ok(Expr::var(&name))
    }

    /// `test ? kept : otherwise`, where `given_up`, the other variant's
    /// value std drops, has nothing to drop; else `or_dropped`'s (ADRs 0179,
    /// 0490).
    fn kept_or(
        &mut self,
        test: Expr,
        kept: Expr,
        given_up: (Expr, Ty<'tcx>),
        otherwise: Expr,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        if self.drops(given_up.1) == Drops::Nothing {
            return Ok(Expr::cond(test, kept, otherwise));
        }
        let name = self.fresh("kept");
        let then = vec![StmtKind::Assign(Expr::var(&name), kept).at(js::Span::NONE)];
        self.or_dropped(test, name, then, given_up, otherwise, span, out)
    }

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
        if let Comb::ResultUnwrapOrElse | Comb::ResultUnwrapOrDefault = comb
            && let Some(value) = result::unwrapped_made(&subject, &values)
        {
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
            return Ok(Expr::array(items));
        }
        // A `Some` not boxed is read once by `??`, and the default made only
        // where it's `None`, as Rust makes it: `get(n) ?? ""`.
        if comb == Comb::UnwrapOrDefault && !boxed {
            let inner = self.option_of(subject_ty).expect("an `Option`");
            let fallback = self.default_value(inner, span)?;
            return Ok(Expr::bin(Op::Coalesce, subject, fallback));
        }
        // And `f` called only where it's `None`: `get(n) ?? root`. A closure
        // of statements is its arrow, made first, which does nothing.
        if comb == Comb::UnwrapOrElse && !boxed {
            let fallback = self.call_with(values.remove(0), Vec::new(), "fallback", out);
            return Ok(Expr::bin(Op::Coalesce, subject, fallback));
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
        let helper = |name: &'static str, list: Vec<Expr>| Expr::call(Expr::helper(name), list);
        Ok(match comb {
            Comb::Then | Comb::ThenSome => unreachable!("handled above"),
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
            Comb::OrElse => {
                let f = next();
                let other = self.call_with(f, Vec::new(), "fallback", out);
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
                let ok_ty = generic_args.type_at(0);
                let fallback = self.default_value(ok_ty, span)?;
                Expr::cond(tag(true), inside(), fallback)
            }
            // An `Ok` with a destructor is dropped (ADR 0179).
            Comb::Err if self.drops(generic_args.type_at(0)) != Drops::Nothing => {
                let name = self.fresh("error");
                let then = vec![StmtKind::Assign(Expr::var(&name), inside()).at(js::Span::NONE)];
                let value = (inside(), generic_args.type_at(0));
                self.or_dropped(tag(false), name, then, value, Expr::undefined(), span, out)?
            }
            Comb::Err => Expr::cond(tag(false), inside(), Expr::undefined()),
            // Rust works out `b` either way: in a `const` first if it has effects.
            Comb::And => {
                let other = eager(self, next(), out);
                Expr::cond(some, other, Expr::undefined())
            }
            // The one that's `Some`, where only one is.
            Comb::Xor | Comb::Zip => {
                let other = next();
                let other = if other.reads_same() {
                    other
                } else {
                    self.spill("other", other, out)
                };
                let other_some = Expr::bin(Op::LooseNe, other.clone(), Expr::null());
                if comb == Comb::Xor {
                    let other_none = Expr::bin(Op::LooseEq, other.clone(), Expr::null());
                    Expr::cond(some, Expr::cond(other_none, subject, Expr::undefined()), other)
                } else {
                    let other_inner = self.option_of(tys[1]).expect("an `Option`");
                    let other_value = match self.boxed_payload(other_inner) {
                        true => self.some_value(other),
                        false => other,
                    };
                    let both = Expr::bin(Op::And, some, other_some);
                    Expr::cond(both, Expr::array(vec![value, other_value]), Expr::undefined())
                }
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
                    halves.push(match self.boxed_payload(part) {
                        true => self.some(half),
                        false => half,
                    });
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
                let ok_value = match self.boxed_payload(ok_ty) {
                    true => self.some(ok_value),
                    false => ok_value,
                };
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
            // An `Err` `or` gives up is dropped (ADRs 0179, 0490).
            Comb::ResultAnd | Comb::ResultOr => {
                let other = eager(self, next(), out);
                let error = (inside(), generic_args.type_at(1));
                match comb {
                    Comb::ResultAnd => Expr::cond(tag(true), other, subject),
                    _ => self.kept_or(tag(true), subject.clone(), error, other, span, out)?,
                }
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
                // Strings and `char`s by code point, `$cmp` (ADR 0183).
                if self.is_text_ord(item) {
                    let x = if x.reads_same() { x } else { self.spill("item", x, out) };
                    let order = self.text_order(Expr::var("each"), x);
                    let f = Expr::arrow(
                        vec!["each".into()],
                        vec![StmtKind::Return(Some(order)).at(js::Span::NONE)],
                    );
                    return Ok(helper("$binarySearchBy", vec![subject, f]));
                }
                // What `<` orders as `Ord` does, `is_primitive_ord`'s.
                if !self.is_primitive_ord(item) {
                    return Err(self.unsupported(span, &format!("`binary_search` of `{item}`s")));
                }
                helper("$binarySearch", vec![subject, x])
            }
            Comb::Contains => {
                let x = next();
                let item = self
                    .slice_item(subject_ty)
                    .ok_or_else(|| self.unsupported(span, "`contains` of this"))?;
                if self.eq_is_identity(item) {
                    Expr::call(Expr::member(subject, "includes"), vec![x])
                } else {
                    let x = if x.reads_same() { x } else { self.spill("item", x, out) };
                    let mut body = Vec::new();
                    let same = self.eq_value(Expr::var("each"), x, item, span, &mut body)?;
                    body.push(StmtKind::Return(Some(same)).at(js::Span::NONE));
                    let f = Expr::arrow(vec!["each".into()], body);
                    Expr::call(Expr::member(subject, "some"), vec![f])
                }
            }
            Comb::Extend => {
                let items = next();
                // A range is an object: its items as its `next` gives them,
                // a `char`'s by code point.
                let items = match self.range_kind(tys[1].peel_refs()) {
                    Some(_) => self.as_js_iterator(items, tys[1].peel_refs(), span)?,
                    None => items,
                };
                let spread = Expr::call(Expr::member(Expr::var("Array"), "from"), vec![items]);
                helper("$extend", vec![subject, spread])
            }
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
                let key = if key.reads_same() {
                    key
                } else {
                    self.spill("key", key, out)
                };
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
            Comb::Swap => helper("$swap", vec![subject, next(), next()]),
            Comb::Truncate => helper("$truncate", self.dropping(vec![subject, next()], subject_ty, span)?),
            Comb::Dedup => {
                let item = self
                    .slice_item(subject_ty)
                    .ok_or_else(|| self.unsupported(span, "`dedup` of this"))?;
                if !self.eq_is_identity(item) {
                    return Err(self.unsupported(span, "`dedup` of what `===` doesn't compare"));
                }
                helper("$dedup", vec![subject])
            }
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
            // Its key's `==`, of what `===` compares, each item's key made once a
            // comparison, the later item's first, as std's are.
            Comb::DedupByKey => {
                let key_ty = generic_args.type_at(generic_args.len() - 1);
                if !self.eq_is_identity(key_ty) {
                    return Err(self.unsupported(span, "`dedup_by_key` of keys `===` doesn't compare"));
                }
                let key = next();
                let key = if key.reads_same() {
                    key
                } else {
                    self.spill("key", key, out)
                };
                let (a, b) = (self.fresh("a"), self.fresh("b"));
                let same = Expr::bin(
                    Op::Eq,
                    Expr::call(key.clone(), vec![Expr::var(&a)]),
                    Expr::call(key, vec![Expr::var(&b)]),
                );
                let same = Expr::arrow(
                    vec![a.into(), b.into()],
                    vec![StmtKind::Return(Some(same)).at(js::Span::NONE)],
                );
                let name = self.by_cells(subject_ty, "$dedupBy", "$dedupByCells");
                helper(name, self.dropping(vec![subject, same], subject_ty, span)?)
            }
            Comb::SwapRemove => helper("$swapRemove", vec![subject, next()]),
            // Each new item its own, a clone, but the last, which is the value
            // given, as std moves it; one whose copies can't be told apart, it.
            Comb::Resize => {
                let (length, item) = (next(), next());
                let item_ty = self
                    .slice_item(subject_ty)
                    .ok_or_else(|| self.unsupported(span, "`resize` of this"))?;
                let mut list = vec![subject, length, item];
                if self.needs_clone(item_ty) {
                    let name = self.fresh("item");
                    let mut body = Vec::new();
                    let copy = self.clone_value(Expr::var(&name), item_ty, span, &mut body)?;
                    body.push(StmtKind::Return(Some(copy)).at(js::Span::NONE));
                    list.push(Expr::arrow(vec![name.into()], body));
                }
                helper("$resize", list)
            }
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
            Comb::Concat => Expr::call(Expr::member(subject, "flat"), Vec::new()),
        })
    }

    /// The helper of items a closure takes `&mut`: of objects, `objects`;
    /// of what a `&mut` to is a cell, a number or text, `cells` (ADR 0315).
    fn by_cells(&self, items: Ty<'tcx>, objects: &'static str, cells: &'static str) -> &'static str {
        match self.slice_item(items).is_some_and(|item| self.is_object(item)) {
            true => objects,
            false => cells,
        }
    }

    /// The function that drops each item a slice, an array or a `Vec` of
    /// type `ty` holds, where they have a destructor: what `truncate`,
    /// `retain`, `dedup_by` and `resize_with` drop what they remove with,
    /// last (ADR 0098).
    pub(super) fn items_drop(&mut self, ty: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        let item = self.slice_item(ty).expect("a slice's, an array's or a `Vec`'s items");
        self.std_drop_function(item, span)
    }

    /// `list`, a call's arguments, and last, `items_drop` of `ty`, where it
    /// has one.
    pub(super) fn dropping(&mut self, mut list: Vec<Expr>, ty: Ty<'tcx>, span: Span) -> R<Vec<Expr>> {
        list.extend(self.items_drop(ty, span)?);
        Ok(list)
    }

    /// What a slice, an array or a `Vec` holds.
    pub(super) fn slice_item(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let ty = ty.peel_refs();
        match ty.kind() {
            ty::Slice(item) | ty::Array(item, _) => Some(*item),
            ty::Adt(_, args) if self.is_vec_like(ty) => Some(args.type_at(0)),
            _ => None,
        }
    }

    /// Does `includes` find a `ty` as `==` does, `js_eq`'s SameValueZero?
    pub(super) fn eq_is_identity(&self, ty: Ty<'tcx>) -> bool {
        self.js_eq(ty) == Some(super::std_impls::JsEq::SameValue)
    }

    /// The drop of a `receiver`'s items, an iterator's, where they have a
    /// destructor.
    fn item_drop_of(&mut self, receiver: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        match self.iterator_item(receiver.peel_refs()) {
            Some(item) => self.std_drop_function(item, span),
            None => Ok(None),
        }
    }

    /// One of `IterComb`'s, on `items`: an array, or a JS iterator if `lazy`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn iter_comb(
        &mut self,
        comb: IterComb,
        items: Expr,
        mut rest: std::vec::IntoIter<Expr>,
        generic_args: ty::GenericArgsRef<'tcx>,
        receiver_ty: Ty<'tcx>,
        lazy: bool,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let mut next = || rest.next().expect("rustc checked the arguments");
        let method = |items: Expr, name: &str, list: Vec<Expr>| Expr::call(Expr::member(items, name), list);
        // Whether the item is `Some`: by its truth where it's never falsy,
        // `.filter(Boolean)` once prepared (ADR 0298).
        let kept = generic_args.types().nth(1);
        let present = || {
            let test = match kept {
                Some(kept) => self.present(Expr::var("item"), kept),
                None => Expr::bin(Op::LooseNe, Expr::var("item"), Expr::null()),
            };
            Expr::arrow(
                vec!["item".into()],
                vec![StmtKind::Return(Some(test)).at(js::Span::NONE)],
            )
        };
        let item_ty = || self.iterator_item(receiver_ty);
        Ok(match comb {
            // `Some`s only: `.map(f).filter((item) => item != null)`.
            IterComb::FilterMap => method(method(items, "map", vec![next()]), "filter", vec![present()]),
            IterComb::FindMap => method(method(items, "map", vec![next()]), "find", vec![present()]),
            IterComb::FlatMap => {
                // A closure that returns an `Option` is a `filter_map`.
                let returned = generic_args.types().nth(1);
                if returned.is_some_and(|t| self.option_of(t).is_some()) {
                    method(method(items, "map", vec![next()]), "filter", vec![present()])
                } else {
                    method(items, "flatMap", vec![next()])
                }
            }
            // Of `Option`s, through a reference too, `iter()`'s `&Some(x)`,
            // which is `x`: the `Some`s.
            IterComb::Flatten => match item_ty() {
                Some(item) if self.option_of(item.peel_refs()).is_some() => method(items, "filter", vec![present()]),
                // Of arrays, each stepped as an array is, `$iter(item)`: what
                // `flat()` takes apart where the chain is made eager.
                Some(item)
                    if lazy && {
                        let item = item.peel_refs();
                        item.is_array() || item.is_slice() || self.is_vec_like(item)
                    } =>
                {
                    let stepped = self.stepped_items(Expr::var("item"));
                    let each = Expr::arrow(
                        vec!["item".into()],
                        vec![StmtKind::Return(Some(stepped)).at(js::Span::NONE)],
                    );
                    method(items, "flatMap", vec![each])
                }
                _ if lazy => {
                    let each = Expr::arrow(
                        vec!["item".into()],
                        vec![StmtKind::Return(Some(Expr::var("item"))).at(js::Span::NONE)],
                    );
                    method(items, "flatMap", vec![each])
                }
                _ => method(items, "flat", Vec::new()),
            },
            // `chain` and `zip` are lazy when either side is (ADR 0128):
            // `iterator_call` says so in `lazy`.
            // Of items with a destructor, each of its own it steps where the
            // other's run out dropped (ADR 0465).
            IterComb::Zip if lazy => {
                let mut args = vec![items, next()];
                args.extend(self.item_drop_of(receiver_ty, span)?);
                Expr::call(Expr::helper("$lazyZip"), args)
            }
            IterComb::Zip => Expr::call(Expr::helper("$zip"), vec![items, next()]),
            IterComb::Chain if lazy => Expr::call(Expr::helper("$lazyChain"), vec![items, next()]),
            IterComb::Chain => method(items, "concat", vec![next()]),
            IterComb::TakeWhile if lazy => Expr::call(Expr::helper("$lazyTakeWhile"), vec![items, next()]),
            IterComb::TakeWhile => Expr::call(Expr::helper("$takeWhile"), vec![items, next()]),
            IterComb::SkipWhile if lazy => Expr::call(Expr::helper("$lazySkipWhile"), vec![items, next()]),
            IterComb::SkipWhile => Expr::call(Expr::helper("$skipWhile"), vec![items, next()]),
            IterComb::Scan => {
                let items = if lazy {
                    method(items, "toArray", Vec::new())
                } else {
                    items
                };
                let (init, f) = (next(), next());
                Expr::call(Expr::helper("$scan"), vec![items, init, f])
            }
            // Of items with a destructor, each it steps over dropped as it's
            // passed, as std's `nth` drops it (ADR 0465).
            IterComb::StepBy => {
                let n = next();
                let n = if n.reads_same() { n } else { self.spill("step", n, out) };
                let keep = Expr::bin(Op::Eq, Expr::bin(Op::Rem, Expr::var("i"), n), Expr::int(0));
                let body = match self.item_drop_of(receiver_ty, span)? {
                    Some(drop) => vec![
                        StmtKind::If(
                            keep,
                            vec![StmtKind::Return(Some(Expr::bool(true))).at(js::Span::NONE)],
                            None,
                        )
                        .at(js::Span::NONE),
                        StmtKind::Expr(Expr::call(drop, vec![Expr::var("item")])).at(js::Span::NONE),
                        StmtKind::Return(Some(Expr::bool(false))).at(js::Span::NONE),
                    ],
                    None => vec![StmtKind::Return(Some(keep)).at(js::Span::NONE)],
                };
                let param = if body.len() > 1 { "item" } else { "_" };
                method(items, "filter", vec![Expr::arrow(vec![param.into(), "i".into()], body)])
            }
            // Each passed over dropped as it's passed, of items with a
            // destructor (ADR 0098); a key made once for each, as std's.
            IterComb::MaxByKey(max) | IterComb::MaxBy(max) => {
                let item = self
                    .iterator_item(receiver_ty.peel_refs())
                    .ok_or_else(|| self.unsupported(span, "this iterator's items"))?;
                // Of an array's items, the array, as `$max`'s are.
                let items = match lazy {
                    true => method(items, "toArray", Vec::new()),
                    false => items,
                };
                let f = next();
                match comb {
                    IterComb::MaxBy(_) => {
                        let helper = if max { "$maxBy" } else { "$minBy" };
                        self.extreme_by(helper, vec![items, f], item, span)?
                    }
                    _ => {
                        let key_ty = generic_args
                            .types()
                            .nth(1)
                            .ok_or_else(|| self.unsupported(span, "this key"))?;
                        if self.drops(key_ty) != crate::lower::drops::Drops::Nothing {
                            return Err(self.unsupported(span, "a key with a destructor"));
                        }
                        let compare = self.cmp_fn(key_ty, false, span)?;
                        let helper = if max { "$maxByKey" } else { "$minByKey" };
                        self.extreme_by(helper, vec![items, f, compare], item, span)?
                    }
                }
            }
            IterComb::Product => {
                let ty = generic_args
                    .types()
                    .nth(1)
                    .ok_or_else(|| self.unsupported(span, "this product"))?;
                let num = self.num(ty, span)?;
                let times = match num {
                    Num::F64 => Expr::bin(Op::Mul, Expr::var("a"), Expr::var("b")),
                    Num::I32 | Num::U32 => num.wrap(Expr::call(
                        Expr::member(Expr::var("Math"), "imul"),
                        vec![Expr::var("a"), Expr::var("b")],
                    )),
                    _ => num.wrap(Expr::bin(Op::Mul, Expr::var("a"), Expr::var("b"))),
                };
                let f = Expr::arrow(
                    vec!["a".into(), "b".into()],
                    vec![StmtKind::Return(Some(times)).at(js::Span::NONE)],
                );
                method(items, "reduce", vec![f, num.literal(1)])
            }
            IterComb::Nth => {
                let n = next();
                if lazy {
                    let first = method(method(items, "drop", vec![n]), "take", vec![Expr::int(1)]);
                    Expr::index(method(first, "toArray", Vec::new()), Expr::int(0))
                } else {
                    Expr::index(items, n)
                }
            }
            IterComb::Inspect => {
                let f = next();
                let f = if matches!(f.kind, js::ExprKind::Var(_)) {
                    f
                } else {
                    self.spill("inspect", f, out)
                };
                let item = Expr::var("item");
                let each = Expr::arrow(
                    vec!["item".into()],
                    vec![
                        StmtKind::Expr(Expr::call(f, vec![item.clone()])).at(js::Span::NONE),
                        StmtKind::Return(Some(item)).at(js::Span::NONE),
                    ],
                );
                method(items, "map", vec![each])
            }
            IterComb::Partition => {
                let items = if lazy {
                    method(items, "toArray", Vec::new())
                } else {
                    items
                };
                Expr::call(Expr::helper("$partition"), vec![items, next()])
            }
            IterComb::Unzip => {
                let into = generic_args.types().skip(3).all(|into| self.is_vec_like(into));
                if !into {
                    return Err(self.unsupported(span, "`unzip` into what isn't a `Vec`"));
                }
                let items = if lazy {
                    method(items, "toArray", Vec::new())
                } else {
                    items
                };
                Expr::call(Expr::helper("$unzip"), vec![items])
            }
        })
    }
}

/// `v.insert(at, value)`: `$insertAt`, which refuses an index past the end;
/// at the front, which no length refuses, `v.unshift(value)`.
fn inserted(subject: Expr, at: Expr, value: Expr) -> Expr {
    match at.as_int() {
        Some(0) => Expr::call(Expr::member(subject, "unshift"), vec![value]),
        _ => Expr::call(Expr::helper("$insertAt"), vec![subject, at, value]),
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Each argument a std call `known` is given by value with a destructor,
    /// a function or a value it may leave unused, of arguments of types
    /// `tys`, by its argument, with where it's dropped, `handled_argument`'s.
    /// A function what it gives holds, `map(f)`'s, is refused.
    pub(super) fn given_unused(
        &self,
        known: Option<super::recognition::Std>,
        (tys, output): (&[Ty<'tcx>], Ty<'tcx>),
        span: Span,
    ) -> R<Vec<(usize, Option<Unused>)>> {
        let mut given = Vec::new();
        for (at, &ty) in tys.iter().enumerate() {
            let Some(when) = self.handled_argument(known, at, ty) else {
                continue;
            };
            // Kept by a lazy adapter it's given to, `map(f)`'s, it drops with it
            // (ADR 0485); by anything else, refused.
            if when.is_none() && self.recognition().held_function(output) == Some(ty) {
                continue;
            }
            if when.is_none() && output.walk().any(|part| part.as_type() == Some(ty)) {
                return Err(self.unsupported(
                    span,
                    "a function that holds a value with a destructor, kept by what it's given to",
                ));
            }
            given.push((at, when));
        }
        Ok(given)
    }

    /// Where an argument std is given by value, `at` of `known`, of type
    /// `ty`, that holds a value with a destructor, is dropped, as std drops
    /// it: on the variant `known` leaves it unused on, a function called by
    /// `FnOnce` alone, whose body drops it where it's run (ADRs 0469, 0470),
    /// or a value, which it keeps where it's used (ADR 0490); a function
    /// called by `&` or `&mut`, as the call returns, `Some(None)`. `None` of
    /// one with nothing to drop, or that's left as it was, run once or kept
    /// by its call.
    pub(super) fn handled_argument(
        &self,
        known: Option<super::recognition::Std>,
        at: usize,
        ty: Ty<'tcx>,
    ) -> Option<Option<Unused>> {
        let unused = || {
            (unused_arguments(known?).iter())
                .find(|&&(arg, _)| arg == at)
                .map(|&(_, when)| Some(when))
        };
        let Some(kind) = super::recognition::function_kind(self.tcx, self.body_owner, ty) else {
            return unused().filter(|_| self.drops_function(ty));
        };
        if !self.drops_function(ty) {
            return None;
        }
        match kind {
            ty::ClosureKind::FnOnce => unused(),
            ty::ClosureKind::Fn | ty::ClosureKind::FnMut => Some(None),
        }
    }

    /// A function called by `&` or `&mut` that holds a value with a
    /// destructor, given where `callee` calls it by `FnOnce` alone, as its
    /// argument: refused, as that call drops none of it, where Rust's drops
    /// it once it's run (ADR 0470).
    pub(super) fn check_called_once(&self, callee: rustc_span::def_id::DefId, tys: &[Ty<'tcx>], span: Span) -> R<()> {
        let inputs = self.tcx.fn_sig(callee).instantiate_identity().skip_binder().inputs();
        for (&ty, &input) in tys.iter().zip(inputs) {
            let kind = super::recognition::function_kind(self.tcx, self.body_owner, ty);
            if kind.is_some_and(|kind| kind != ty::ClosureKind::FnOnce)
                && super::recognition::function_kind(self.tcx, callee, input) == Some(ty::ClosureKind::FnOnce)
                && self.drops_function(ty)
            {
                let name = self.tcx.item_name(callee);
                return Err(self.unsupported(
                    span,
                    &format!("giving `{name}`, which calls it once, a function called by `&` that holds a value with a destructor"),
                ));
            }
        }
        Ok(())
    }

    /// Whether a function of type `ty`, called by `FnOnce` alone, which a
    /// std call that writes a place or a map's entry may leave unrun, is
    /// dropped there, as `get_or_insert_with`'s is (ADR 0469).
    pub(super) fn drops_unrun(&self, ty: Ty<'tcx>) -> bool {
        super::recognition::function_kind(self.tcx, self.body_owner, ty) == Some(ty::ClosureKind::FnOnce)
            && self.drops_function(ty)
    }

    /// Whether a function, or a value, of type `ty` std is given is dropped
    /// where std drops it: one that holds a value with a destructor, or a
    /// type parameter's whose drop this function is given. Of one it isn't
    /// given, its callers give none with a destructor (ADR 0190).
    fn drops_function(&self, ty: Ty<'tcx>) -> bool {
        if self.drops(ty) == Drops::Nothing {
            return false;
        }
        // Of what holds only type parameters' values: its own drop given, or,
        // a closure that took them, theirs (ADR 0485).
        if self.drop_query().drops_but_params(ty) == Drops::Nothing
            && self.given_drop(ty).is_none()
            && !(matches!(ty.kind(), ty::Closure(..)) && self.holds_given_drops(ty))
        {
            self.require_no_drops(ty);
            return false;
        }
        true
    }

    /// Each argument `f`, of type `ty`, a std call was given, dropped where
    /// std drops it: as it returns, or where it's left unused, by the
    /// variant of its `subject` it's left on: `if (option == null) {
    /// loudDrop_drop(a); }`.
    pub(super) fn dropped_given(
        &mut self,
        subject: Option<Expr>,
        given: Vec<(Option<Unused>, Expr, Ty<'tcx>)>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let js_span = self.js_span(span);
        for (when, f, ty) in given {
            let mut dropped = Vec::new();
            self.drop_value(f, ty, span, &mut dropped)?;
            let Some(when) = when else {
                out.extend(dropped);
                continue;
            };
            let subject = subject.clone().expect("the subject it's left unrun by");
            let test = match when {
                Unused::None => Expr::bin(Op::LooseEq, subject, Expr::null()),
                Unused::Some => Expr::bin(Op::LooseNe, subject, Expr::null()),
                Unused::Ok => result::is(subject, true),
                Unused::Err => result::is(subject, false),
                Unused::False => Expr::unary(js::UnaryOp::Not, subject),
            };
            out.push(StmtKind::If(test, dropped, None).at(js_span));
        }
        Ok(())
    }
}
