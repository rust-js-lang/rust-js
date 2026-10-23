//! A number's operators: `+`, `-`, a bitwise or a shift, a unary, a cast,
//! and what a big integer's shift and mask are.

use super::*;

/// The largest constant a 32-bit integer is multiplied by exactly in a
/// double: any 32-bit value, under 2^32, times one of at most 2^21 is under
/// 2^53, where every integer a double holds is exact.
const EXACT_FACTOR: u128 = 1 << 21;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    // ── Operators ───────────────────────────────────────────────────────

    /// `known` is `r`'s value, when rustc knows it and the JS doesn't show
    /// it: a named `const` (ADR 0031).
    pub(in crate::lower) fn binary(
        &mut self,
        op: BinOp,
        l: Expr,
        r: Expr,
        known: Option<i128>,
        ty: Ty<'tcx>,
        span: Span,
    ) -> R<Expr> {
        let comparison = match op {
            BinOp::Eq => Some(Op::Eq),
            BinOp::Ne => Some(Op::Ne),
            BinOp::Lt => Some(Op::Lt),
            BinOp::Le => Some(Op::Le),
            BinOp::Gt => Some(Op::Gt),
            BinOp::Ge => Some(Op::Ge),
            _ => None,
        };
        if comparison.is_some() && bounds(ty).is_some_and(|(min, max)| at_bound(op, &l, &r, min, max)) {
            return Ok(Expr::bool(true));
        }
        // `char`s by code point, as Rust orders them (ADR 0183).
        if let Some(js_op) = comparison
            && ty.is_char()
            && !matches!(op, BinOp::Eq | BinOp::Ne)
        {
            return Ok(self.text_compare(js_op, l, r));
        }
        if let Some(js_op) = comparison {
            return Ok(Expr::bin(js_op, without_zero_sign(l), without_zero_sign(r)));
        }

        if ty.is_bool() {
            // `&`, `|`, `^` on bools evaluate both sides and give a bool.
            return match op {
                BinOp::BitXor => Ok(Expr::bin(Op::Ne, l, r)),
                BinOp::BitAnd => Ok(Expr::unary(
                    UnaryOp::Not,
                    Expr::unary(UnaryOp::Not, Expr::bin(Op::BitAnd, l, r)),
                )),
                BinOp::BitOr => Ok(Expr::unary(
                    UnaryOp::Not,
                    Expr::unary(UnaryOp::Not, Expr::bin(Op::BitOr, l, r)),
                )),
                _ => Err(self.unsupported(span, "this operator on `bool`")),
            };
        }

        let num = self.num(ty, span)?;
        if num.float() {
            let js_op = match op {
                BinOp::Add => Op::Add,
                BinOp::Sub => Op::Sub,
                BinOp::Mul => Op::Mul,
                BinOp::Div => Op::Div,
                BinOp::Rem => Op::Rem,
                _ => return Err(self.unsupported(span, &format!("this operator on `{ty}`"))),
            };
            // An `f32`'s is the exact one rounded to an `f32`, as Rust's is:
            // a double's 53 bits hold it close enough to round once (ADR
            // 0122). `%` of two is exact, and one already.
            let e = Expr::bin(js_op, l, r);
            return Ok(if num == Num::F32 && op != BinOp::Rem {
                num.wrap(e)
            } else {
                e
            });
        }
        if num.big() {
            return self.big_binary(op, l, r, known, num, span);
        }

        // What the body's ranges show stays in range is JS's own (ADR 0576).
        if let BinOp::Add | BinOp::Sub | BinOp::Mul = op
            && self.body_facts.exact.contains(&span)
        {
            let js_op = match op {
                BinOp::Add => Op::Add,
                BinOp::Sub => Op::Sub,
                _ => Op::Mul,
            };
            return Ok(Expr::bin(js_op, l, r));
        }
        // Integers: compute exactly in JS, then wrap back into range.
        Ok(match op {
            BinOp::Add => num.wrap(Expr::bin(Op::Add, l, r)),
            BinOp::Sub => num.wrap(Expr::bin(Op::Sub, l, r)),
            // A 32-bit product can exceed 2^53 and lose bits; `Math.imul` can't.
            // Of a constant of at most 2^21 either side, `n * 2`, it can't
            // either, so it's JS's own `*`, wrapped as `+`'s is (ADR 0011).
            BinOp::Mul if num.bits() == 32 => {
                let small = |c: Option<i128>| c.is_some_and(|c| c.unsigned_abs() <= EXACT_FACTOR);
                if small(known.or_else(|| r.as_int())) || small(l.as_int()) {
                    num.wrap(Expr::bin(Op::Mul, l, r))
                } else {
                    let product = Expr::call(Expr::member(Expr::var("Math"), "imul"), vec![l, r]);
                    if num.signed() { product } else { num.wrap(product) }
                }
            }
            BinOp::Mul => num.wrap(Expr::bin(Op::Mul, l, r)),
            BinOp::Div | BinOp::Rem => {
                let (js_op, name) = match op {
                    BinOp::Div => (Op::Div, "$div"),
                    _ => (Op::Rem, "$rem"),
                };
                // A literal divisor that can't panic stays inline: `a / 3 | 0`.
                let safe = known
                    .or_else(|| r.as_int())
                    .is_some_and(|d| d != 0 && !(num.signed() && d == -1));
                let quotient = if safe {
                    Expr::bin(js_op, l, r)
                } else {
                    let mut args = vec![l, r];
                    if num.signed() {
                        args.push(Expr::int(num.range().0));
                    }
                    Expr::call(Expr::helper(name), args)
                };
                // The remainder of in-range integers is already in range, but a
                // signed one can be JS's `-0`, `-2 % 2`, which `| 0` makes
                // `0`: an integer is never `-0` (ADR 0064).
                if op == BinOp::Rem && safe && num.signed() {
                    Expr::bin(Op::BitOr, quotient, Expr::int(0))
                } else if op == BinOp::Rem && safe {
                    quotient
                } else {
                    num.wrap(quotient)
                }
            }
            BinOp::BitAnd => self.bitwise(Op::BitAnd, l, r, num),
            BinOp::BitOr => self.bitwise(Op::BitOr, l, r, num),
            BinOp::BitXor => self.bitwise(Op::BitXor, l, r, num),
            // Rust (without overflow checks) masks the shift amount to the
            // type's width. JS masks to 32, which is only right for 32 bits.
            BinOp::Shl if num.bits() == 32 => num.wrap(Expr::bin(Op::Shl, l, r)),
            BinOp::Shl => num.wrap(Expr::bin(Op::Shl, l, mask_shift(r, num))),
            BinOp::Shr => {
                let js_op = if num.signed() { Op::Shr } else { Op::UShr };
                let r = if num.bits() == 32 { r } else { mask_shift(r, num) };
                Expr::bin(js_op, l, r)
            }
            _ => return Err(self.unsupported(span, "this operator")),
        })
    }

    /// An `i64`'s or a `u64`'s operator (ADR 0086): exact on BigInts, then
    /// wrapped. A quotient needs no wrap, but can panic as Rust's does.
    pub(in crate::lower) fn big_binary(
        &mut self,
        op: BinOp,
        l: Expr,
        r: Expr,
        known: Option<i128>,
        num: Num,
        span: Span,
    ) -> R<Expr> {
        Ok(match op {
            BinOp::Add => num.wrap(Expr::bin(Op::Add, unwrapped(l), unwrapped(r))),
            BinOp::Sub => num.wrap(Expr::bin(Op::Sub, unwrapped(l), unwrapped(r))),
            BinOp::Mul => num.wrap(Expr::bin(Op::Mul, unwrapped(l), unwrapped(r))),
            BinOp::Div | BinOp::Rem => {
                let (js_op, name) = match op {
                    BinOp::Div => (Op::Div, "$bigDiv"),
                    _ => (Op::Rem, "$bigRem"),
                };
                let safe = known
                    .or_else(|| r.as_bigint())
                    .is_some_and(|d| d != 0 && !(num.signed() && d == -1));
                if safe {
                    Expr::bin(js_op, l, r)
                } else {
                    let mut args = vec![l, r];
                    if num.signed() {
                        args.push(Expr::bigint(num.range().0));
                    }
                    Expr::call(Expr::helper(name), args)
                }
            }
            // Of two in range, in range.
            BinOp::BitAnd => Expr::bin(Op::BitAnd, l, r),
            BinOp::BitOr => Expr::bin(Op::BitOr, l, r),
            BinOp::BitXor => Expr::bin(Op::BitXor, l, r),
            // The amount masked to 63, as release Rust masks it, and a BigInt,
            // whatever its own type.
            BinOp::Shl => num.wrap(Expr::bin(Op::Shl, unwrapped(l), big_shift(r, num))),
            BinOp::Shr => Expr::bin(Op::Shr, l, big_shift(r, num)),
            _ => return Err(self.unsupported(span, "this operator")),
        })
    }

    /// `NonZero::new(n)`: `n`, or `None` of zero.
    pub(in crate::lower) fn non_zero_new(&mut self, n: Expr, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        let num = self.num(ty, span)?;
        // Of a constant, `NonZero::new(7)`: its answer.
        if let Some(value) = n.as_int().or_else(|| n.as_bigint()) {
            return Ok(if value == 0 { Expr::undefined() } else { n });
        }
        let n = if n.reads_same() { n } else { self.spill("n", n, out) };
        let zero = Expr::bin(Op::Eq, n.clone(), num.literal(0));
        Ok(Expr::cond(zero, Expr::undefined(), n))
    }

    pub(in crate::lower) fn bitwise(&self, op: Op, l: Expr, r: Expr, num: Num) -> Expr {
        // JS bitwise ops return signed 32-bit results; only u32 needs fixing.
        let e = Expr::bin(op, l, r);
        if num == Num::U32 { num.wrap(e) } else { e }
    }

    pub(in crate::lower) fn unary(&mut self, op: UnOp, a: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        match op {
            UnOp::Not if ty.is_bool() => Ok(Expr::unary(UnaryOp::Not, a)),
            UnOp::Not => {
                let num = self.num(ty, span)?;
                if num.float() {
                    return Err(self.unsupported(span, &format!("`!` on `{ty}`")));
                }
                let e = Expr::unary(UnaryOp::BitNot, a);
                Ok(if num.signed() { e } else { num.wrap(e) })
            }
            UnOp::Neg => {
                let num = self.num(ty, span)?;
                // `-x` of a literal is just a negative literal, in range:
                // `-129i8` is 127, as `-i8::MIN` is itself.
                // A float's is exact, an `f32`'s one already, and `-0` isn't 0.
                if num.float() {
                    return Ok(Expr::unary(UnaryOp::Neg, a));
                }
                if let Some(n) = a.as_int().or_else(|| a.as_bigint()) {
                    return Ok(num.wrap(num.literal(-n)));
                }
                Ok(num.wrap(Expr::unary(UnaryOp::Neg, a)))
            }
            UnOp::PtrMetadata => Err(self.unsupported(span, "pointer metadata")),
        }
    }

    /// A number `From` a `bool`, `flag ? 1 : 0`, of a float too, which `as`
    /// can't make, or a `char`, as `as` makes it (ADR 0358).
    pub(in crate::lower) fn number_from(&mut self, v: Expr, from: Ty<'tcx>, to: Ty<'tcx>, span: Span) -> R<Expr> {
        if from.is_bool() {
            let num = self.num(to, span)?;
            return Ok(Expr::cond(v, num.literal(1), num.literal(0)));
        }
        self.cast(v, from, to, span)
    }

    pub(in crate::lower) fn cast(&mut self, v: Expr, from: Ty<'tcx>, to: Ty<'tcx>, span: Span) -> R<Expr> {
        let target = self.num(to, span)?;
        if from.is_bool() && !target.float() {
            return Ok(Expr::cond(v, target.literal(1), target.literal(0)));
        }
        // A `char` is its code point (ADR 0063), and a `u8` as a `char` its
        // character.
        if from.is_char() {
            let code = Expr::call(Expr::member(v, "codePointAt"), vec![Expr::int(0)]);
            let code = if target.big() { to_bigint(code) } else { code };
            let (lo, hi) = target.range();
            return Ok(if target.float() || (lo <= 0 && hi >= 0x10ffff) {
                code
            } else {
                target.wrap(code)
            });
        }
        // An `Ordering` is -1, 0 or 1 already (ADR 0036).
        let (v, source) = if self.is_lang_adt(from, LangItem::OrderingEnum) {
            (v, Num::I8)
        } else if let ty::Adt(adt, _) = from.kind()
            && is_fieldless_enum(*adt)
        {
            // A fieldless enum's discriminant, by its variant's name (ADR 0013).
            let discriminants = discriminants(self.tcx, *adt);
            // Each one fits the target type, so there's nothing to wrap.
            let (lo, hi) = target.range();
            let fits = target.float()
                || discriminants
                    .iter()
                    .all(|&(_, d)| lo <= d && (d < 0 || d as u128 <= hi));
            let repr = rustc_middle::ty::util::IntTypeExt::to_ty(&adt.repr().discr_type(), self.tcx);
            let repr = Num::of(repr).unwrap_or(Num::I32);
            // The discriminants as what they're read as: the target, if each
            // fits it, else the enum's own type, cast to it after. A 64-bit
            // one is a BigInt from the start, exact past 2^53.
            let read = if fits { target } else { repr };
            let value = self
                .named_discriminant(v, *adt, read.big())
                .map_err(|_| self.unsupported(span, "casting an enum whose value doesn't name its variant"))?;
            if fits {
                return Ok(value);
            }
            (value, repr)
        } else {
            (v, self.num(from, span)?)
        };
        match (source, target) {
            // An `f32` is an `f64` exactly, and an `f64` is its nearest `f32`
            // (ADR 0122).
            (Num::F64, Num::F64) | (Num::F32, Num::F32) | (Num::F32, Num::F64) => Ok(v),
            (Num::F64, Num::F32) => Ok(target.wrap(v)),
            // `as` from float to int saturates (ADR 0086): `NaN` is 0, and the
            // rest is truncated into range. An `f32` is an `f64` exactly.
            (Num::F32 | Num::F64, _) => {
                let (lo, hi) = target.range();
                let name = if target.big() { "$f64ToBig" } else { "$f64ToInt" };
                Ok(Expr::call(
                    Expr::helper(name),
                    vec![v, target.literal(lo), target.literal(hi as i128)],
                ))
            }
            // An `i64` or `u64` is its nearest `f64`, as `as` rounds it.
            (source, Num::F64) if source.big() => Ok(Expr::call(Expr::var("Number"), vec![v])),
            // Every other integer fits exactly in an f64.
            (_, Num::F64) => Ok(v),
            // Its nearest `f32`, rounded once: through an `f64` it would be
            // rounded twice, which can miss it (ADR 0122).
            (source, Num::F32) if source.big() => Ok(Expr::call(Expr::helper("$bigToF32"), vec![v])),
            // Up to 16 bits, an `f32` holds it exactly; a 32-bit one, an `f64`
            // does, rounded once.
            (source, Num::F32) if source.bits() <= 16 => Ok(v),
            (_, Num::F32) => Ok(target.wrap(v)),
            _ => {
                let (lo, hi) = source.range();
                let (tlo, thi) = target.range();
                let fits = tlo <= lo && hi <= thi;
                Ok(match (source.big(), target.big()) {
                    // Into a BigInt, then into range.
                    (false, true) if fits => to_bigint(v),
                    (false, true) => target.wrap(to_bigint(v)),
                    // A constant, or what a mask keeps in range: `x & 1023n`.
                    (true, false) if let Some(n) = v.as_bigint() => target.wrap(Expr::int(n)),
                    (true, false) if masked(&v).is_some_and(|mask| mask >= 0 && mask as u128 <= thi) => {
                        Expr::call(Expr::var("Number"), vec![v])
                    }
                    // Into range as a BigInt, then a number.
                    (true, false) => {
                        let method = if target.signed() { "asIntN" } else { "asUintN" };
                        let wrapped = Expr::call(
                            Expr::member(Expr::var("BigInt"), method),
                            vec![Expr::int(target.bits().into()), v],
                        );
                        Expr::call(Expr::var("Number"), vec![wrapped])
                    }
                    _ if fits => v,
                    _ => target.wrap(v),
                })
            }
        }
    }
}

pub(in crate::lower) fn mask_shift(r: Expr, num: Num) -> Expr {
    Expr::bin(Op::BitAnd, r, Expr::num(num.bits() - 1))
}

/// `shift_amount`, by the two types: a trait's, `Shl<i64>` of a `u32`, where
/// there are no expressions, as in a dictionary (ADR 0108).
pub(in crate::lower) fn shift_amount_of<'tcx>(op: BinOp, r: Expr, lhs: Ty<'tcx>, rhs: Ty<'tcx>) -> Expr {
    let big = |ty: Ty<'tcx>| Num::of(ty.peel_refs()).is_some_and(Num::big);
    if !matches!(op, BinOp::Shl | BinOp::Shr) || big(lhs) || !big(rhs) {
        return r;
    }
    match r.as_bigint() {
        Some(n) => Expr::int(n & 63),
        None => Expr::call(Expr::var("Number"), vec![Expr::bin(Op::BitAnd, r, Expr::bigint(63))]),
    }
}

/// `BigInt(x)`, or of a literal, the BigInt literal.
pub(in crate::lower) fn to_bigint(e: Expr) -> Expr {
    match e.as_int() {
        Some(n) => Expr::bigint(n),
        None => Expr::call(Expr::var("BigInt"), vec![e]),
    }
}

/// A 64-bit or 128-bit shift's amount: masked to 63 or 127, as release
/// Rust masks it, as a BigInt, `BigInt(n) & 63n`.
pub(in crate::lower) fn big_shift(r: Expr, num: Num) -> Expr {
    let mask = i128::from(num.bits()) - 1;
    match r.as_int().or_else(|| r.as_bigint()) {
        Some(n) => Expr::bigint(n & mask),
        None => Expr::bin(Op::BitAnd, to_bigint(r), Expr::bigint(mask)),
    }
}

/// `x + y` of `BigInt.asUintN(64, x + y)`: what's added, subtracted,
/// multiplied or shifted left needn't be wrapped itself, as the result is,
/// modulo the same 2^64, so `a + b + c` is wrapped once.
pub(in crate::lower) fn unwrapped(e: Expr) -> Expr {
    if let js::ExprKind::Call(callee, args) = &e.kind
        && let js::ExprKind::Member(object, name) = &callee.kind
        && matches!(&object.kind, js::ExprKind::Var(v) if v == "BigInt")
        && (name == "asUintN" || name == "asIntN")
        && let [bits, inner] = args.as_slice()
        && bits.as_int() == Some(64)
    {
        return inner.clone();
    }
    e
}

/// The mask of `x & 1023n`, which keeps it from 0 to 1023.
pub(in crate::lower) fn masked(e: &Expr) -> Option<i128> {
    match &e.kind {
        js::ExprKind::Binary(Op::BitAnd, a, b) => b.as_bigint().or_else(|| a.as_bigint()),
        _ => None,
    }
}

/// The least and the greatest value of an integer or a `char`, its code
/// point, of one `at_bound` can read: not a float's, nor a 128-bit one's.
fn bounds(ty: Ty<'_>) -> Option<(i128, i128)> {
    if ty.is_char() {
        return Some((0, char::MAX as i128));
    }
    let num = Num::of(ty).filter(|num| !num.float() && num.bits() < 128)?;
    let bits = num.bits();
    Some(match num.signed() {
        true => (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1),
        false => (0, (1i128 << bits) - 1),
    })
}

/// Whether `l op r` is true of every value of its type, between `min` and
/// `max`, its other side read for nothing else: a range pattern's test of
/// the bound the type has anyway, `i32::MIN..=-1`'s `i32::MIN <= n`, or
/// `'\u{80}'..=char::MAX`'s `c <= char::MAX`, which isn't written (ADR
/// 0262: it can't be observed).
fn at_bound(op: BinOp, l: &Expr, r: &Expr, min: i128, max: i128) -> bool {
    // A number, or a `char`, a string of one.
    let value = |e: &Expr| match &e.kind {
        js::ExprKind::Str(s) => match s.chars().collect::<Vec<_>>()[..] {
            [c] => Some(c as i128),
            _ => None,
        },
        _ => e.as_int().or_else(|| e.as_bigint()),
    };
    let (bound, other) = match op {
        BinOp::Le => match (value(l), value(r)) {
            (Some(lo), _) if lo == min => (true, r),
            (_, Some(hi)) if hi == max => (true, l),
            _ => (false, l),
        },
        BinOp::Ge => match (value(l), value(r)) {
            (_, Some(lo)) if lo == min => (true, l),
            (Some(hi), _) if hi == max => (true, r),
            _ => (false, l),
        },
        _ => (false, l),
    };
    bound && !other.has_effects()
}
