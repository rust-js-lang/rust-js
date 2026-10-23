//! Ranges (ADR 0129): `a..b` is `{ start: a, end: b }`, as any struct is,
//! and so is `a..=b`; `a..` is `{ start: a }`. Iterated, a range is its
//! items: an array, or for `a..`, which never ends, a JS iterator.

use crate::js::{self, Expr, Op, Prop, Stmt, StmtKind, UnaryOp};
use crate::lower::display::{Pretty, join};
use crate::lower::recognition::IterMethod;
use crate::lower::representation::Num;
use crate::lower::{FnCx, R};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

/// A range made as stepped, lazily, its runtime's generator (ADR 0472).
pub(crate) const LAZY_RANGE: &str = "$lazyRange";
/// A range's numbers from its end, `$lazyRangeBack(a, b, step)`.
pub(crate) const LAZY_RANGE_BACK: &str = "$lazyRangeBack";

/// Which of std's ranges a type is.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::lower) enum RangeKind {
    /// `a..b`
    Exclusive,
    /// `a..=b`
    Inclusive,
    /// `a..`
    From,
    /// `..b`
    To,
    /// `..=b`
    ToInclusive,
    /// `..`
    Full,
}

impl RangeKind {
    /// Its bounds, by the names of its fields.
    pub(in crate::lower) fn bounds(self) -> &'static [&'static str] {
        match self {
            RangeKind::Exclusive | RangeKind::Inclusive => &["start", "end"],
            RangeKind::From => &["start"],
            RangeKind::To | RangeKind::ToInclusive => &["end"],
            RangeKind::Full => &[],
        }
    }

    /// The runtime's helper that steps a range of this kind from its
    /// start, or, `back`, its end: `$rangeNext`, `$rangeInclusiveNextBack`.
    /// `None` of one JS doesn't step, or a `RangeFrom` from its end, which
    /// it hasn't. The one table, which `stepped_by` and `next()` ask, and
    /// `stepped_range` reads back.
    pub(in crate::lower) fn step(self, back: bool) -> Option<&'static str> {
        Some(match (self, back) {
            (RangeKind::Exclusive, false) => "$rangeNext",
            (RangeKind::Exclusive, true) => "$rangeNextBack",
            (RangeKind::Inclusive, false) => "$rangeInclusiveNext",
            (RangeKind::Inclusive, true) => "$rangeInclusiveNextBack",
            (RangeKind::From, false) => "$rangeFromNext",
            _ => return None,
        })
    }

    /// What `{:?}` shows between its bounds.
    fn dots(self) -> &'static str {
        match self {
            RangeKind::Inclusive | RangeKind::ToInclusive => "..=",
            _ => "..",
        }
    }
}

/// A range method rust-js knows.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum RangeOp {
    /// `RangeInclusive::new(a, b)`, which `a..=b` is: `{ start: a, end: b }`.
    New,
    /// `r.contains(&x)`: `r.start <= x && x < r.end`.
    Contains,
    /// A `RangeInclusive`'s `start()` and `end()`: its field.
    Bound(&'static str),
    /// A `RangeInclusive`'s `into_inner()`: `[r.start, r.end]`.
    IntoInner,
    /// `is_empty()`: `!(r.start < r.end)`.
    IsEmpty,
    /// `len()`: `Math.max(0, r.end - r.start)`.
    Len,
    /// `size_hint()`: how many are left, exactly, as both bounds, `[n, n]`.
    SizeHint,
    /// A `Range`'s `next()` and `next_back()`, and an `a..`'s `next()`,
    /// which move its `start` or its `end`, as Rust's do.
    Next,
    NextBack,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A range's own `nth(n)`, `last()` and `count()`, of `a..b` or
    /// `a..=b`, or of one reversed, `(a..b).rev()`, `values` it and `n`: of
    /// its bounds, without stepping each number, as std's are (#163),
    /// `$rangeNth`, `$rangeInclusiveNthBack`, of a reversed one's
    /// `$iterator`'s bounds, `.of`. `None` of another
    /// method, or of a range of `char`s, which steps by code point, or a
    /// count of BigInts, which isn't a `usize`.
    pub(in crate::lower) fn range_own_step(
        &self,
        method: Option<IterMethod>,
        range: Ty<'tcx>,
        values: &[Expr],
    ) -> Option<Expr> {
        let reversed = self.recognition().reversed(range);
        let kind = self.range_kind(reversed.unwrap_or(range))?;
        let num = self.range_index(reversed.unwrap_or(range)).and_then(Num::of)?;
        let name = match (method?, kind, reversed.is_some()) {
            (IterMethod::Nth, RangeKind::Exclusive, false) => "$rangeNth",
            (IterMethod::Nth, RangeKind::Exclusive, true) => "$rangeNthBack",
            (IterMethod::Nth, RangeKind::Inclusive, false) => "$rangeInclusiveNth",
            (IterMethod::Nth, RangeKind::Inclusive, true) => "$rangeInclusiveNthBack",
            (IterMethod::Last, RangeKind::Exclusive, false) => "$rangeLast",
            (IterMethod::Last, RangeKind::Inclusive, false) => "$rangeInclusiveLast",
            (IterMethod::Last, RangeKind::Exclusive, true) => "$rangeFirst",
            (IterMethod::Last, RangeKind::Inclusive, true) => "$rangeInclusiveFirst",
            (IterMethod::Count, RangeKind::Exclusive, _) if !num.big() => "$rangeLen",
            (IterMethod::Count, RangeKind::Inclusive, _) if !num.big() => "$rangeInclusiveLen",
            _ => return None,
        };
        let mut values = values.to_vec();
        // A reversed one's bounds, its `$iterator`'s: those it was made of,
        // where it's made here, once `reversed_step` reads them.
        if reversed.is_some() {
            let first = values.first_mut()?;
            *first = Expr::member(first.clone(), "of");
        }
        Some(Expr::call_helper(name, values))
    }

    /// A range's `len()`, `range_count` of its bounds, where it's a range
    /// of numbers that aren't BigInts, `a..b` or `a..=b`, that `it` reads.
    pub(in crate::lower) fn range_len(&self, it: &Expr, range: Ty<'tcx>, kind: RangeKind) -> Option<Expr> {
        let numbers = self.range_index(range).and_then(Num::of).is_some_and(|num| !num.big());
        let (start, end) = range_bounds(it)?;
        (numbers && matches!(kind, RangeKind::Exclusive | RangeKind::Inclusive)).then(|| range_count(start, end, kind))
    }

    /// What a range's bounds are: the `u32` of a `Range<u32>`.
    pub(in crate::lower) fn range_index(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        match ty.kind() {
            ty::Adt(_, args) => args.types().next(),
            _ => None,
        }
    }

    /// A range's start and end as its `RangeBounds` gives them, `Bound`s of
    /// type `bound`, of its value `range` of type `ty`, a range of std's or a
    /// pair of `Bound`s: `{ TAG: "Included", _0: r.start }`, `"Unbounded"`,
    /// or the pair's own items. `range` is read once a bound, from a literal
    /// itself; `None` of a type that's neither. The one answer, of a
    /// `RangeBounds` dictionary's and of a B-tree's range alike.
    pub(in crate::lower) fn range_bound_values(
        &mut self,
        range: Expr,
        ty: Ty<'tcx>,
        bound: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<[Expr; 2]>> {
        use crate::lower::recognition::RangeBound;
        let Some((start, end)) = crate::lower::recognition::range_bounds(self.tcx, ty) else {
            return Ok(None);
        };
        let literal = matches!(range.kind, js::ExprKind::Object(_) | js::ExprKind::Array(_))
            && (range.reads_same() || !range.has_effects());
        let range = match literal || range.reads_same() {
            true => range,
            false => self.spill("range", range, out),
        };
        let variant = |name: &str| match bound.kind() {
            ty::Adt(adt, _) => (adt.variants().iter_enumerated())
                .find(|(_, v)| v.name.as_str() == name)
                .map(|(i, _)| i),
            _ => None,
        };
        let value = |this: &mut Self, part: RangeBound| -> R<Expr> {
            Ok(match part {
                RangeBound::Item(i) => Expr::index(range.clone(), Expr::int(i as i128)),
                RangeBound::Field(kind, field) => {
                    let index = variant(kind).ok_or_else(|| this.unsupported(span, "this range's bounds"))?;
                    let held = match &range.kind {
                        js::ExprKind::Object(props) => props.iter().find_map(|prop| match prop {
                            Prop::Field(name, value) if name == field => Some(value.clone()),
                            _ => None,
                        }),
                        _ => None,
                    };
                    let held = held.unwrap_or_else(|| Expr::member(range.clone(), field));
                    this.adt_value(bound, index, vec![held], &[], None, span)?
                }
                RangeBound::Unbounded => {
                    let index = variant("Unbounded").ok_or_else(|| this.unsupported(span, "this range's bounds"))?;
                    this.adt_value(bound, index, Vec::new(), &[], None, span)?
                }
            })
        };
        let start = value(self, start)?;
        let end = value(self, end)?;
        Ok(Some([start, end]))
    }

    /// The bounds of `{ start: a, end: b }` itself, if each reads the same
    /// wherever it's read, or does nothing, `i + 1`: what it reads, nothing
    /// changes between.
    pub(in crate::lower) fn range_literal_parts(&self, range: &Expr, kind: RangeKind) -> Option<Vec<Expr>> {
        let js::ExprKind::Object(props) = &range.kind else {
            return None;
        };
        if props.len() != kind.bounds().len() {
            return None;
        }
        kind.bounds()
            .iter()
            .map(|name| {
                props.iter().find_map(|prop| match prop {
                    Prop::Field(field, value) if field == name && (value.reads_same() || !value.has_effects()) => {
                        Some(value.clone())
                    }
                    _ => None,
                })
            })
            .collect()
    }

    /// A range's bounds, each to be read once or more: from `{ start: a,
    /// end: b }` itself, or from the range, a `const` first if reading it
    /// again would differ.
    pub(in crate::lower) fn range_parts(&mut self, range: Expr, kind: RangeKind, out: &mut Vec<Stmt>) -> Vec<Expr> {
        if let Some(parts) = self.range_literal_parts(&range, kind) {
            return parts;
        }
        let range = if range.reads_same() {
            range
        } else {
            self.spill("range", range, out)
        };
        kind.bounds()
            .iter()
            .map(|name| Expr::member(range.clone(), *name))
            .collect()
    }

    /// A range's items: `$range(r.start, r.end)`, `$range(a, b + 1)` of
    /// `a..=b`, and `$lazyRange(a)` of `a..`, a JS iterator that never ends.
    pub(in crate::lower) fn range_items(
        &mut self,
        range: Expr,
        ty: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let kind = self.range_kind(ty).expect("a range");
        let index = self.range_index(ty);
        if index.is_some_and(|index| index.is_char()) && matches!(kind, RangeKind::Exclusive | RangeKind::Inclusive) {
            let mut list = self.range_parts(range, kind, out);
            if kind == RangeKind::Inclusive {
                list.push(Expr::bool(true));
            }
            return Ok(Expr::call(Expr::helper("$charRange"), list));
        }
        let Some(num) = index.and_then(Num::of) else {
            return Err(self.unsupported(span, &format!("iterating over `{ty}`")));
        };
        let parts = self.range_parts(range, kind, out);
        Ok(match (kind, parts.as_slice()) {
            (RangeKind::Exclusive | RangeKind::Inclusive, [start, end]) => {
                // `..=6` is `..7`.
                let end = match end.as_int() {
                    Some(n) if kind == RangeKind::Inclusive && !num.big() => Expr::int(n + 1),
                    _ if kind == RangeKind::Inclusive => Expr::bin(Op::Add, end.clone(), num.literal(1)),
                    _ => end.clone(),
                };
                let name = if num.big() { "$bigRange" } else { "$range" };
                Expr::call(Expr::helper(name), vec![start.clone(), end])
            }
            (RangeKind::From, [start]) => Expr::call(Expr::helper(LAZY_RANGE), vec![start.clone()]),
            _ => return Err(self.unsupported(span, &format!("iterating over `{ty}`"))),
        })
    }

    /// `{:?}` of a range: its bounds', around its dots, `1..4` or `..=4`.
    pub(in crate::lower) fn range_debug(&mut self, range: Expr, ty: Ty<'tcx>, span: Span, pretty: &Pretty) -> R<Expr> {
        let kind = self.range_kind(ty).expect("a range");
        let index = self.range_index(ty);
        let shown = |this: &mut Self, parts: Vec<Expr>| -> R<Expr> {
            let mut parts = parts.into_iter();
            let mut pieces = Vec::new();
            if matches!(kind, RangeKind::Exclusive | RangeKind::Inclusive | RangeKind::From) {
                let start = parts.next().expect("a start");
                pieces.push(this.debug_string_with(start, index.expect("a bound's type"), span, pretty)?);
            }
            pieces.push(Expr::str(kind.dots()));
            if let Some(end) = parts.next() {
                pieces.push(this.debug_string_with(end, index.expect("a bound's type"), span, pretty)?);
            }
            Ok(join(pieces))
        };
        if let Some(parts) = self.range_literal_parts(&range, kind) {
            return shown(self, parts);
        }
        let parts = kind
            .bounds()
            .iter()
            .map(|name| Expr::member(Expr::var("range"), *name))
            .collect();
        let body = shown(self, parts)?;
        let f = Expr::arrow(
            vec!["range".into()],
            vec![StmtKind::Return(Some(body)).at(js::Span::NONE)],
        );
        Ok(self.applied(f, range))
    }

    /// A range's function, of its arguments' values and types.
    pub(in crate::lower) fn range_call(
        &mut self,
        op: RangeOp,
        values: Vec<Expr>,
        tys: &[Ty<'tcx>],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        if op == RangeOp::New {
            let props = ["start", "end"]
                .into_iter()
                .zip(values)
                .map(|(name, value)| Prop::Field(name.into(), value))
                .collect();
            return Ok(Expr::object(props));
        }
        let range_ty = tys[0].peel_refs();
        let kind = self.range_kind(range_ty).expect("a range's method");
        let index = self.range_index(range_ty);
        if op == RangeOp::Contains {
            let item_ty = tys[1].peel_refs();
            if index.is_none_or(|index| {
                index.peel_refs() != item_ty || !(self.is_primitive_ord(index) || self.is_text_ord(index))
            }) {
                return Err(self.unsupported(span, &format!("`contains` of a `{range_ty}`")));
            }
            let [range, item]: [Expr; 2] = values.try_into().ok().unwrap();
            let parts = self.range_parts(range, kind, out);
            let item = if item.reads_same() {
                item
            } else {
                self.spill("item", item, out)
            };
            // As Rust's: the start's test, then the end's; of `char`s, by code
            // point (ADR 0183).
            let text = index.is_some_and(|index| self.is_text_ord(index));
            let mut compare = |op: Op, a: Expr, b: Expr| match text {
                true => self.text_compare(op, a, b),
                false => Expr::bin(op, a, b),
            };
            let tests: Vec<Expr> = match (kind, parts.as_slice()) {
                (RangeKind::Exclusive, [start, end]) => vec![
                    compare(Op::Le, start.clone(), item.clone()),
                    compare(Op::Lt, item, end.clone()),
                ],
                (RangeKind::Inclusive, [start, end]) => vec![
                    compare(Op::Le, start.clone(), item.clone()),
                    compare(Op::Le, item, end.clone()),
                ],
                (RangeKind::From, [start]) => vec![compare(Op::Le, start.clone(), item)],
                (RangeKind::To, [end]) => vec![compare(Op::Lt, item, end.clone())],
                (RangeKind::ToInclusive, [end]) => vec![compare(Op::Le, item, end.clone())],
                _ => vec![Expr::bool(true)],
            };
            return Ok(tests
                .into_iter()
                .reduce(|all, test| Expr::bin(Op::And, all, test))
                .expect("a test"));
        }
        if matches!(op, RangeOp::Next | RangeOp::NextBack) {
            if index.and_then(Num::of).is_none() {
                return Err(self.unsupported(span, &format!("stepping through a `{range_ty}`")));
            }
            let range = values.into_iter().next().expect("the range");
            let Some(name) = kind.step(op == RangeOp::NextBack) else {
                return Err(self.unsupported(span, &format!("stepping through a `{range_ty}`")));
            };
            return Ok(Expr::call(Expr::helper(name), vec![range]));
        }
        let range = values.into_iter().next().expect("the range");
        let parts = self.range_parts(range, kind, out);
        Ok(match (op, parts.as_slice()) {
            (RangeOp::Bound(name), [start, end]) => {
                if name == "start" {
                    start.clone()
                } else {
                    end.clone()
                }
            }
            (RangeOp::IntoInner, [start, end]) => Expr::array(vec![start.clone(), end.clone()]),
            (RangeOp::IsEmpty, [start, end]) => {
                let op = if kind == RangeKind::Inclusive { Op::Le } else { Op::Lt };
                Expr::unary(UnaryOp::Not, Expr::bin(op, start.clone(), end.clone()))
            }
            // What's left of the numbers, both bounds; of BigInts, an error.
            (RangeOp::SizeHint, [start, end]) if index.and_then(Num::of).is_some_and(|n| !n.big()) => {
                let left = self.spill("left", range_count(start.clone(), end.clone(), kind), out);
                Expr::array(vec![left.clone(), left])
            }
            _ => return Err(self.unsupported(span, &format!("this method of a `{range_ty}`"))),
        })
    }
}

/// How many numbers a range of `start` and `end` has left, of `kind`:
/// `Math.max(end - start, 0)`, one more of `a..=b`, none once stepping has
/// passed its end. A range's `len()` and `size_hint()` alike.
pub(in crate::lower) fn range_count(start: Expr, end: Expr, kind: RangeKind) -> Expr {
    let gap = Expr::bin(Op::Sub, end, start);
    let gap = match kind {
        RangeKind::Inclusive => Expr::bin(Op::Add, gap, Expr::int(1)),
        _ => gap,
    };
    Expr::call(Expr::member(Expr::var("Math"), "max"), vec![gap, Expr::int(0)])
}

/// A range made where it's stepped, `$iterator({ start: 0, end: n },
/// $rangeNext)`, which nothing else steps, nor lends: its numbers, made as
/// they're stepped, `$lazyRange(0, n)`, so `.find(f)` stops where Rust's
/// does; of `a..=b`, `$lazyRange(a, b + 1)`. Of a number it starts at, as a
/// `char`'s range starts at text.
pub(in crate::lower) fn range_items(e: &mut Expr) {
    if let Some(made) = lazy_range(crate::lower::iterators::unlent(e)) {
        *e = made;
    }
}

/// A lazy range's own steps, of numbers it knows without stepping them, as
/// std's range does (#163): `step_by(n)`, `.filter((_, i) => i % n ===
/// 0)`, is `$lazyRange(a, b, n)`, and `skip(n)`, `.drop(n)`, starts it `n`
/// steps on; a range's own `count()` and `last()` are `range_own_step`'s.
/// Run after `range_items`, each of its source first.
pub(in crate::lower) fn range_steps(e: &mut Expr) {
    while let Some(folded) = range_step(e).or_else(|| reversed_step(e)) {
        *e = folded;
    }
}

/// Of a range made where it's stepped from its end, `$iterator({ start,
/// end }, $rangeNextBack)`: its bounds, where `.of` reads them, as
/// `range_own_step` does of one a variable holds; and `.drop(n)` of it, its
/// end `n` before, as `Rev`'s `skip` is the range's `nth_back`, where its
/// end is a number, not a BigInt `n` isn't; and `.step_by(n)` of it, its
/// numbers from its end, `$lazyRangeBack(a, b, n)` (#163).
fn reversed_step(e: &Expr) -> Option<Expr> {
    match &e.kind {
        js::ExprKind::Member(made, field) if field == "of" => {
            crate::lower::iterators::stepped_range(made)?;
            let js::ExprKind::Call(_, args) = &made.kind else {
                unreachable!("a stepped range is a call")
            };
            Some(args[0].clone())
        }
        js::ExprKind::Call(callee, args) => {
            let (js::ExprKind::Member(made, method), [n]) = (&callee.kind, &args[..]) else {
                return None;
            };
            let range = crate::lower::iterators::stepped_range(made)?;
            // `step_by(n)`, which tidy wrote as a filter of every `n`th:
            // the numbers from its end, `n` apart.
            if method == "filter" && range.back {
                let step = every_nth(n)?;
                let mut args = vec![range.start.clone(), range.end.clone(), step];
                if range.inclusive {
                    args.push(Expr::bool(true));
                }
                return Some(Expr::call_helper(LAZY_RANGE_BACK, args));
            }
            if method != "drop" || !range.back || !n.reads_same() || range.end.as_int().is_none() {
                return None;
            }
            let end = match (range.end.as_int(), n.as_int()) {
                (Some(end), Some(n)) => Expr::int(end - n),
                _ => Expr::bin(Op::Sub, range.end.clone(), n.clone()),
            };
            let js::ExprKind::Call(stepper, made_args) = &made.kind else {
                unreachable!("a stepped range is a call")
            };
            let bounds = Expr::object(vec![
                Prop::Field("start".to_string(), range.start.clone()),
                Prop::Field("end".to_string(), end),
            ]);
            Some(Expr::call((**stepper).clone(), vec![bounds, made_args[1].clone()]))
        }
        _ => None,
    }
}

/// `$lazyRange(a, b, s)`'s bounds and step, each read once where it's used.
fn lazy_range_parts(e: &Expr) -> Option<(Expr, Option<Expr>, Option<Expr>)> {
    let js::ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    if callee.helper_name() != Some(LAZY_RANGE) || !args.iter().all(Expr::reads_same) {
        return None;
    }
    match &args[..] {
        [start] => Some((start.clone(), None, None)),
        [start, end] => Some((start.clone(), Some(end.clone()), None)),
        [start, end, step] => Some((start.clone(), Some(end.clone()), Some(step.clone()))),
        _ => None,
    }
}

fn range_step(e: &Expr) -> Option<Expr> {
    let js::ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    let js::ExprKind::Member(source, method) = &callee.kind else {
        return None;
    };
    let (start, end, step) = lazy_range_parts(source)?;
    let made = |start: Expr, step: Option<Expr>| {
        let mut list = vec![start];
        list.extend(end.clone().or_else(|| step.is_some().then(|| Expr::var("Infinity"))));
        list.extend(step);
        Expr::call_helper(LAZY_RANGE, list)
    };
    match (method.as_str(), &args[..]) {
        ("filter", [f]) if step.is_none() => every_nth(f).map(|n| made(start, Some(n))),
        ("drop", [n]) if n.reads_same() => {
            let skipped = match &step {
                Some(step) => Expr::bin(Op::Mul, n.clone(), step.clone()),
                None => n.clone(),
            };
            let start = match (start.as_int(), skipped.as_int()) {
                (Some(a), Some(k)) => Expr::int(a + k),
                _ => Expr::bin(Op::Add, start, skipped),
            };
            Some(made(start, step))
        }
        _ => None,
    }
}

/// `n` of `(_, i) => i % n === 0`, `step_by(n)`'s filter of what it keeps.
pub(in crate::lower) fn every_nth(f: &Expr) -> Option<Expr> {
    let js::ExprKind::Arrow(params, body) = &f.kind else {
        return None;
    };
    let [js::Pattern::Name(_), js::Pattern::Name(i)] = params.as_slice() else {
        return None;
    };
    let [
        js::Stmt {
            kind: js::StmtKind::Return(Some(test)),
            ..
        },
    ] = body.as_slice()
    else {
        return None;
    };
    let js::ExprKind::Binary(Op::Eq, rem, zero) = &test.kind else {
        return None;
    };
    let js::ExprKind::Binary(Op::Rem, index, n) = &rem.kind else {
        return None;
    };
    (zero.as_int() == Some(0) && matches!(&index.kind, js::ExprKind::Var(v) if v == i) && n.reads_same())
        .then(|| (**n).clone())
}

/// `$lazyRange(a, b)` of `$iterator({ start: a, end: b }, $rangeNext)`, `a` a
/// number (`range_items`).
fn lazy_range(e: &Expr) -> Option<Expr> {
    let range = crate::lower::iterators::stepped_range(e)?;
    if range.back || !matches!(range.start.kind, js::ExprKind::Num(_)) {
        return None;
    }
    let end = match range.inclusive {
        true => match range.end.as_int() {
            Some(n) => Expr::int(n + 1),
            None => Expr::bin(Op::Add, range.end.clone(), Expr::int(1)),
        },
        false => range.end.clone(),
    };
    Some(Expr::call_helper(LAZY_RANGE, vec![range.start.clone(), end]))
}

/// A range's bounds, each read once: of `{ start: a, end: b }` made there,
/// `a` and `b`, or of one that reads the same, `r.start` and `r.end`.
pub(in crate::lower) fn range_bounds(range: &Expr) -> Option<(Expr, Expr)> {
    match &range.kind {
        js::ExprKind::Object(props) => match &props[..] {
            [js::Prop::Field(s, start), js::Prop::Field(e, end)] if s == "start" && e == "end" => {
                Some((start.clone(), end.clone()))
            }
            _ => None,
        },
        _ if range.reads_same() => Some((Expr::member(range.clone(), "start"), Expr::member(range.clone(), "end"))),
        _ => None,
    }
}
