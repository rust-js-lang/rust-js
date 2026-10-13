//! `Option`'s and `Result`'s combinators, more iterator adapters, and more
//! of `Vec`'s methods (ADR 0062). A closure whose body is one value is
//! written in place, as `Option::map`'s is: `o ?? f()` is `o ?? 0` for
//! `unwrap_or_else(|| 0)`.

use super::calls::apply;
use super::drops::Drops;
use super::recognition::trait_method;
use super::{FnCx, R, representation::Num};
use crate::js::{self, Expr, Op, Prop, Stmt, StmtKind};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

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
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What a `value` of `ty` points at, as `*value` reads it: a counted
    /// `Rc`'s value, or the `deref()` of a type with its own `Deref`.
    fn deref_value(&mut self, value: Expr, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        // A counted `Rc`'s, its `value` (ADR 0320).
        if self.counted_rc(ty).is_some() {
            return Ok(Expr::member(value, "value"));
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

    pub(super) fn ok(value: Expr) -> Expr {
        Expr::object(vec![
            Prop::Field("TAG".into(), Expr::str("Ok")),
            Prop::Field("_0".into(), value),
        ])
    }

    pub(super) fn err(value: Expr) -> Expr {
        Expr::object(vec![
            Prop::Field("TAG".into(), Expr::str("Err")),
            Prop::Field("_0".into(), value),
        ])
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
        let tag = |t: &str| Expr::bin(Op::Eq, Expr::member(subject.clone(), "TAG"), Expr::str(t));
        let inside = || Expr::member(subject.clone(), "_0");
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
                let fallback = next();
                let fallback = eager(self, fallback, out);
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
                self.or_dropped(tag("Ok"), name, then, error, fallback, span, out)?
            }
            Comb::ResultMapOr => {
                let fallback = next();
                let fallback = eager(self, fallback, out);
                let f = next();
                let mapped = self.call_with(f, vec![inside()], "map", out);
                Expr::cond(tag("Ok"), mapped, fallback)
            }
            Comb::ResultMapOrElse => {
                let (g, f) = (next(), next());
                let mapped = self.call_with(f, vec![inside()], "map", out);
                let fallback = self.call_with(g, vec![inside()], "fallback", out);
                Expr::cond(tag("Ok"), mapped, fallback)
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
                let e = next();
                let e = eager(self, e, out);
                Expr::cond(some, Self::ok(value), Self::err(e))
            }
            Comb::OkOrElse => {
                let f = next();
                let e = self.call_with(f, Vec::new(), "error", out);
                Expr::cond(some, Self::ok(value), Self::err(e))
            }
            Comb::Or => {
                let other = next();
                let other = eager(self, other, out);
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
                Expr::cond(tag("Ok"), Self::ok(mapped), subject)
            }
            Comb::MapErr => {
                let f = next();
                let mapped = self.call_with(f, vec![inside()], "map", out);
                Expr::cond(tag("Err"), Self::err(mapped), subject)
            }
            Comb::ResultAndThen => {
                let f = next();
                let then = self.call_with(f, vec![inside()], "then", out);
                Expr::cond(tag("Ok"), then, subject)
            }
            Comb::ResultUnwrapOrElse => {
                let f = next();
                let fallback = self.call_with(f, vec![inside()], "fallback", out);
                Expr::cond(tag("Ok"), inside(), fallback)
            }
            Comb::ResultUnwrapOrDefault => {
                let ok_ty = generic_args.type_at(0);
                let fallback = self.default_value(ok_ty, span)?;
                Expr::cond(tag("Ok"), inside(), fallback)
            }
            // An `Ok` with a destructor is dropped (ADR 0179).
            Comb::Err if self.drops(generic_args.type_at(0)) != Drops::Nothing => {
                let name = self.fresh("error");
                let then = vec![StmtKind::Assign(Expr::var(&name), inside()).at(js::Span::NONE)];
                let value = (inside(), generic_args.type_at(0));
                self.or_dropped(tag("Err"), name, then, value, Expr::undefined(), span, out)?
            }
            Comb::Err => Expr::cond(tag("Err"), inside(), Expr::undefined()),
            // Rust works out `b` either way: in a `const` first if it has effects.
            Comb::And => {
                let other = next();
                let other = eager(self, other, out);
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
                let ok_value = Expr::member(value.clone(), "_0");
                let ok_value = match self.boxed_payload(ok_ty) {
                    true => self.some(ok_value),
                    false => ok_value,
                };
                let is_ok = Expr::bin(Op::Eq, Expr::member(value, "TAG"), Expr::str("Ok"));
                let absent = none();
                let inner = Expr::cond(is_ok, Self::ok(ok_value), subject);
                Expr::cond(absent, Self::ok(Expr::undefined()), inner)
            }
            Comb::Inspect | Comb::ResultInspect { .. } => {
                let f = next();
                let (test, given) = match comb {
                    Comb::Inspect => (some, value),
                    Comb::ResultInspect { err } => (tag(if err { "Err" } else { "Ok" }), inside()),
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
                    _ => (tag("Ok"), inside(), generic_args.types().nth(2)),
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
                    None => Expr::cond(tag("Ok"), Self::ok(pointee), subject),
                }
            }
            Comb::ResultAnd | Comb::ResultOr => {
                let other = next();
                let other = eager(self, other, out);
                match comb {
                    Comb::ResultAnd => Expr::cond(tag("Ok"), other, subject),
                    _ => Expr::cond(tag("Ok"), subject, other),
                }
            }
            Comb::ResultOrElse => {
                let f = next();
                let other = self.call_with(f, vec![inside()], "fallback", out);
                Expr::cond(tag("Ok"), subject, other)
            }
            Comb::ResultFlatten => Expr::cond(tag("Ok"), inside(), subject),
            Comb::ResultIter => Expr::cond(tag("Ok"), Expr::array(vec![inside()]), Expr::array(Vec::new())),
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
                    Comb::ResultIterMut => Expr::cond(tag("Ok"), Expr::array(vec![ok]), Expr::array(Vec::new())),
                    // Of objects, the `Result` itself, as `as_ref()`'s is.
                    _ if !self.is_boxable(ok_ty) && !self.is_boxable(err_ty) => subject,
                    _ => {
                        let side = |name: Expr, value: Expr| {
                            Expr::object(vec![Prop::Field("TAG".into(), name), Prop::Field("_0".into(), value)])
                        };
                        // Of both, a handle on `r._0` whichever it is.
                        if self.is_boxable(ok_ty) && self.is_boxable(err_ty) {
                            side(Expr::member(subject.clone(), "TAG"), ok)
                        } else {
                            let err = part(self, err_ty);
                            Expr::cond(tag("Ok"), side(Expr::str("Ok"), ok), side(Expr::str("Err"), err))
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
                Expr::cond(tag("Ok"), Expr::cond(present, Expr::undefined(), Self::ok(x)), subject)
            }
            // A clone of an `Ok`'s value where a clone is more than the value.
            Comb::ResultCloned => {
                let item = generic_args.types().next().expect("its value's type");
                match self.needs_clone(item) {
                    true => {
                        let clone = self.clone_value(inside(), item, span, out)?;
                        Expr::cond(tag("Ok"), Self::ok(clone), subject)
                    }
                    false => subject,
                }
            }
            // The other variant's value, with a destructor, is dropped (ADR 0179).
            Comb::IsOkAnd | Comb::IsErrAnd
                if self.drops(generic_args.type_at(usize::from(comb == Comb::IsOkAnd))) != Drops::Nothing =>
            {
                let p = next();
                let which = if comb == Comb::IsOkAnd { "Ok" } else { "Err" };
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
                let which = if comb == Comb::IsOkAnd { "Ok" } else { "Err" };
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
                // What `<` orders as `Ord` does: integers, `char`s, strings. Not
                // a `&mut` to one, a cell (ADR 0099).
                let ordered =
                    !self.has_cell_layer(item) && (Num::of(item).is_some_and(|n| !n.float())) || item.is_bool();
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
                if !ordered {
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
                let list = vec![subject, next(), Expr::str(count)];
                if left {
                    helper("$rotateLeft", list)
                } else {
                    helper("$rotateRight", list)
                }
            }
            Comb::Insert => helper("$insertAt", vec![subject, next(), next()]),
            Comb::Remove => helper("$removeAt", vec![subject, next()]),
            Comb::Swap => helper("$swap", vec![subject, next(), next()]),
            Comb::Truncate => helper("$truncate", vec![subject, next()]),
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
                helper(name, vec![subject, next()])
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
                helper(name, vec![subject, same])
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
            Comb::ResizeWith => helper("$resizeWith", vec![subject, next(), next()]),
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

    /// What a slice, an array or a `Vec` holds.
    pub(super) fn slice_item(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let ty = ty.peel_refs();
        match ty.kind() {
            ty::Slice(item) | ty::Array(item, _) => Some(*item),
            ty::Adt(_, args) if self.is_vec_like(ty) => Some(args.type_at(0)),
            _ => None,
        }
    }

    /// Is `==` on `ty` JS's `===`?
    pub(super) fn eq_is_identity(&self, ty: Ty<'tcx>) -> bool {
        // A `&mut` to one is a cell, an object (ADR 0099): not by identity.
        if self.has_cell_layer(ty) {
            return false;
        }
        let ty = ty.peel_refs();
        self.is_string_like(ty)
            || Num::of(ty).is_some_and(|n| !n.float())
            || ty.is_bool()
            || matches!(ty.kind(), ty::Adt(adt, _) if super::is_fieldless_enum(*adt))
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
            IterComb::Zip if lazy => {
                let other = next();
                Expr::call(Expr::helper("$lazyZip"), vec![items, other])
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
            IterComb::StepBy => {
                let n = next();
                let n = if n.reads_same() { n } else { self.spill("step", n, out) };
                let keep = Expr::bin(Op::Eq, Expr::bin(Op::Rem, Expr::var("i"), n), Expr::int(0));
                let f = Expr::arrow(
                    vec!["_".into(), "i".into()],
                    vec![StmtKind::Return(Some(keep)).at(js::Span::NONE)],
                );
                method(items, "filter", vec![f])
            }
            IterComb::MaxByKey(max) | IterComb::MaxBy(max) => {
                let items = if lazy {
                    method(items, "toArray", Vec::new())
                } else {
                    items
                };
                let f = next();
                let compare = match comb {
                    IterComb::MaxBy(_) => f,
                    _ => {
                        let key_ty = generic_args
                            .types()
                            .nth(1)
                            .ok_or_else(|| self.unsupported(span, "this key"))?;
                        let key = if matches!(f.kind, js::ExprKind::Var(_)) {
                            f
                        } else {
                            self.spill("key", f, out)
                        };
                        let mut body = Vec::new();
                        let order = self.cmp_value(
                            Expr::call(key.clone(), vec![Expr::var("a")]),
                            Expr::call(key, vec![Expr::var("b")]),
                            key_ty,
                            false,
                            span,
                            &mut body,
                        )?;
                        body.push(StmtKind::Return(Some(order)).at(js::Span::NONE));
                        Expr::arrow(vec!["a".into(), "b".into()], body)
                    }
                };
                Expr::call(Expr::var(if max { "$maxBy" } else { "$minBy" }), vec![items, compare])
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
