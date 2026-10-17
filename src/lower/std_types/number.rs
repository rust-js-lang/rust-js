//! Arithmetic, casts, and methods of integers and `f64` (ADR 0064).
//! A small integer or `f64` is a JS number; `i64`/`u64` use BigInt (ADR 0086).
//! Many number methods map to `Math`: `x.sqrt()` is `Math.sqrt(x)`. Where
//! Rust's answer differs from JS's (`round` of a half, `pow` past 2^53),
//! a helper gives Rust's.

use crate::js::StmtKind;
use crate::js::{self, Expr, Op, Stmt, UnaryOp};
use crate::lower::calls::Call;
use crate::lower::discriminants;
use crate::lower::recognition::{Std, TypeFact, any_trait};
use crate::lower::representation::{Num, is_fieldless_enum};
use crate::lower::std_types::result;
use crate::lower::{FnCx, R};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::mir::{BinOp, UnOp};
use rustc_middle::ty::{self, Ty, TypeFoldable, TypeVisitableExt};
use rustc_span::Span;

/// A `Duration`'s own, of its nanoseconds, a BigInt (ADR 0188): each unit
/// by how many nanoseconds it has.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum DurationOp {
    /// `Duration::new(secs, nanos)`, which panics past `MAX`.
    New,
    /// `from_secs` and the like.
    From(i64),
    /// `as_secs` and the like: the whole ones.
    As(i64),
    /// `subsec_nanos` and the like: of what's under a second.
    Subsec(i64),
    IsZero,
    /// `+` and `-`, which panic as std's do.
    Add,
    Sub,
    /// `checked_add` and `checked_sub`: `None` where `+` or `-` panics.
    Checked {
        add: bool,
    },
}

#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum NumOp {
    /// The same in JS: `Math.floor(x)`, `Math.atan2(y, x)`.
    Math(&'static str),
    Abs,
    UnsignedAbs,
    Pow,
    CheckedPow,
    Powi,
    /// `a.mul_add(b, c)`: `a * b + c`, rounded once (ADR 0430).
    MulAdd,
    Powf,
    /// `x.exp2()`: `2 ** x`, as `powf` is (ADR 0136).
    Exp2,
    Round,
    /// `a.total_cmp(&b)`: IEEE 754's total order, as Rust has it.
    TotalCmp,
    IsNan,
    IsFinite,
    IsInfinite,
    /// `is_normal()`, or `is_subnormal()` (`subnormal`).
    IsNormal {
        subnormal: bool,
    },
    /// `classify()`: its `FpCategory`.
    Classify,
    /// `leading_ones()`, or `trailing_ones()` (`trailing`).
    EdgeOnes {
        trailing: bool,
    },
    SwapBytes,
    ReverseBits,
    /// `to_le()` and `from_le()`, itself on wasm32, or `to_be()` and
    /// `from_be()` (`swap`), its bytes swapped.
    Endian {
        swap: bool,
    },
    /// `checked_div_euclid`, or `checked_rem_euclid` (`rem`).
    CheckedEuclid {
        rem: bool,
    },
    /// A float's `recip()`.
    Recip,
    /// A float's `to_be_bytes()` or `to_le_bytes()` (`little`).
    FloatToBytes {
        little: bool,
    },
    /// `f64::from_be_bytes(b)` or `from_le_bytes` (`little`).
    FloatFromBytes {
        little: bool,
    },
    Checked(BinOp),
    Saturating(BinOp),
    Wrapping(BinOp),
    RemEuclid,
    DivEuclid,
    Signum,
    LeadingZeros,
    TrailingZeros,
    CountOnes,
    IsPowerOfTwo,
    /// `next_power_of_two()`, wrapping to `0` past its width as release
    /// Rust's does, and `checked_next_power_of_two()`, `None` there.
    NextPowerOfTwo {
        checked: bool,
    },
    AbsDiff,
    /// A byte's ASCII test, `is_ascii_digit()`: the inclusive ranges it holds.
    AsciiIs(&'static [(u8, u8)]),
    /// A byte's `to_ascii_uppercase()`, or `to_ascii_lowercase()`.
    AsciiCase {
        upper: bool,
    },
    /// `x.clamp(min, max)`, an integer's `Ord::clamp` or a float's own.
    Clamp,
    /// `T::from_str_radix(s, radix)`, as `s.parse()` reads, in a radix.
    FromStrRadix,
    DivCeil,
    Fract,
    /// `to_radians()`, or `to_degrees()`.
    Angle {
        radians: bool,
    },
    /// `is_sign_negative()`, or `is_sign_positive()`, its negation.
    SignNegative {
        negated: bool,
    },
    /// `ilog2()` or `ilog10()`: how often `base` divides into it.
    Ilog {
        base: u32,
    },
    Isqrt,
    Midpoint,
    CountZeros,
    /// `is_positive()`, or `is_negative()`.
    IsPositive {
        negative: bool,
    },
    CheckedNeg,
    CheckedAbs,
    /// `checked_shl(n)` or `checked_shr(n)`: `None` from the width on.
    CheckedShift(BinOp),
    WrappingNeg,
    WrappingDiv,
    WrappingRem,
    /// `overflowing_add`, `_sub` and `_mul`, and `overflowing_neg()`.
    Overflowing(BinOp),
    OverflowingNeg,
    SaturatingPow,
    /// `rotate_left(n)`, or `rotate_right(n)`, of an integer's bits.
    RotateBits {
        left: bool,
    },
    /// `to_be_bytes()`, or `to_le_bytes()` and `to_ne_bytes()`, little-endian
    /// as wasm32 is.
    ToBytes {
        little: bool,
    },
    /// `T::from_be_bytes(bytes)`, or `from_le_bytes` and `from_ne_bytes`.
    FromBytes {
        little: bool,
    },
    /// A float's `to_bits()`, and `f64::from_bits(bits)`.
    ToBits,
    FromBits,
    /// A float's `to_int_unchecked::<T>()`: its `as T`, which is it where
    /// Rust defines it, in range.
    ToIntUnchecked,
}

/// std's `f32::to_degrees` factor, its own literal, as std writes it, which
/// is the `f32` nearest it.
#[allow(clippy::excessive_precision)]
const F32_DEGREES_PER_RADIAN: f32 = 57.2957795130823208767981548141051703;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// An integer's bits and a float's bytes, the same for every width: of a
    /// number or a BigInt, as num-traits' `PrimInt` and `Float` ask. `None`
    /// if `op` is another.
    fn bits_call(&mut self, op: NumOp, values: Vec<Expr>, num: Num) -> R<Result<Expr, Vec<Expr>>> {
        let width = Expr::int(num.bits().into());
        let signed = Expr::bool(num.signed());
        let bytes = Expr::int((num.bits() / 8).into());
        let call = |name: &'static str, list: Vec<Expr>| Expr::call(Expr::helper(name), list);
        let ops = matches!(
            op,
            NumOp::EdgeOnes { .. }
                | NumOp::SwapBytes
                | NumOp::ReverseBits
                | NumOp::Endian { .. }
                | NumOp::CheckedEuclid { .. }
                | NumOp::Recip
                | NumOp::FloatToBytes { .. }
                | NumOp::FloatFromBytes { .. }
        );
        if !ops {
            return Ok(Err(values));
        }
        let mut values = values.into_iter();
        let mut arg = || values.next().expect("rustc checked the arguments");
        Ok(Ok(match op {
            NumOp::EdgeOnes { trailing } => {
                let mut list = vec![arg(), width];
                if trailing {
                    list.push(Expr::bool(true));
                }
                call("$edgeOnes", list)
            }
            NumOp::SwapBytes | NumOp::Endian { swap: true } => call("$swapBytes", vec![arg(), width, signed]),
            NumOp::Endian { swap: false } => arg(),
            NumOp::ReverseBits => call("$reverseBits", vec![arg(), width, signed]),
            NumOp::CheckedEuclid { rem } => {
                let min = match num.signed() {
                    true => num.literal(num.range().0),
                    false => Expr::undefined(),
                };
                call("$checkedEuclid", vec![arg(), arg(), min, Expr::bool(rem)])
            }
            NumOp::Recip => num.wrap(Expr::bin(Op::Div, Expr::num(1.0), arg())),
            NumOp::FloatToBytes { little } => {
                let bits = call("$floatToBits", vec![arg(), bytes.clone()]);
                call("$toBytes", vec![bits, bytes, Expr::bool(little)])
            }
            NumOp::FloatFromBytes { little } => {
                let list = vec![arg(), bytes.clone(), Expr::bool(little), Expr::bool(false)];
                let bits = call("$fromBytes", list);
                call("$floatFromBits", vec![bits, bytes])
            }
            _ => unreachable!("one of the ops above"),
        }))
    }

    /// A `Wrapping`'s operator, of its type, its value, or its place where
    /// it's assigned, `assign`, read the same each time, and the operand.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::lower) fn wrapping_values(
        &mut self,
        op: Result<BinOp, UnOp>,
        assign: bool,
        wrapping: Ty<'tcx>,
        target: Expr,
        rhs: Option<Expr>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let inner = self.recognition().wrapping_of(wrapping).expect("a `Wrapping`");
        self.num(inner, span)?;
        let op = match op {
            Err(op) => {
                let result = self.unary(op, Expr::index(target, Expr::int(0)), inner, span)?;
                return Ok(Expr::array(vec![result]));
            }
            Ok(op) => op,
        };
        let rhs = rhs.expect("a binary operator's operand");
        let result = self.wrapping_result(op, Expr::index(target.clone(), Expr::int(0)), rhs, inner, span)?;
        if !assign {
            return Ok(Expr::array(vec![result]));
        }
        out.push(StmtKind::Assign(target, Expr::array(vec![result])).at(self.js_span(span)));
        Ok(Expr::undefined())
    }

    /// `a op rhs` of a `Wrapping`'s number `a`, its `inner`: the result's
    /// number. `rhs` is another `Wrapping`, or a shift's `usize` amount,
    /// which the number's shift masks to its width, as `wrapping_shl` does.
    pub(in crate::lower) fn wrapping_result(
        &mut self,
        op: BinOp,
        a: Expr,
        rhs: Expr,
        inner: Ty<'tcx>,
        span: Span,
    ) -> R<Expr> {
        let rhs = match op {
            BinOp::Shl | BinOp::Shr => rhs,
            _ => Expr::index(rhs, Expr::int(0)),
        };
        self.binary(op, a, rhs, None, inner, span)
    }

    /// A number's method, of its arguments' values.
    pub(in crate::lower) fn number_call(
        &mut self,
        op: NumOp,
        values: Vec<Expr>,
        ty: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let num = Num::of(ty).expect("a number's method");
        let values = match self.bits_call(op, values, num)? {
            Ok(value) => return Ok(value),
            Err(values) => values,
        };
        if num.big() {
            return self.big_number_call(op, values, ty, num, span, out);
        }
        // A number's range, of 32 bits at most.
        let (lo, hi) = num.range();
        let hi = hi as i128;
        let mut values = values.into_iter();
        let mut arg = || values.next().expect("rustc checked the arguments");
        let math = |name: &str, list: Vec<Expr>| Expr::call(Expr::member(Expr::var("Math"), name), list);
        let number = |name: &str, list: Vec<Expr>| Expr::call(Expr::member(Expr::var("Number"), name), list);
        // A narrow signed integer's bits, for the questions about them: -1i8 is 0xff.
        let bits = |x: Expr| match num {
            Num::I8 => Expr::bin(Op::BitAnd, x, Expr::int(0xff)),
            Num::I16 => Expr::bin(Op::BitAnd, x, Expr::int(0xffff)),
            _ => x,
        };
        let helper = |name: &'static str, list: Vec<Expr>| Expr::call(Expr::helper(name), list);
        // An `f32`'s result is rounded to one, but for those that are exact
        // already (ADR 0122).
        let rounded = |e: Expr| if num == Num::F32 { num.wrap(e) } else { e };
        Ok(match op {
            NumOp::Math(name @ ("floor" | "ceil" | "trunc" | "abs")) => {
                let list = values.collect();
                math(name, list)
            }
            NumOp::Math(name) => {
                let list = values.collect();
                rounded(math(name, list))
            }
            NumOp::Abs => num.wrap(math("abs", vec![arg()])),
            // An `i32::MIN`'s is 2^31, which a `u32` holds.
            NumOp::UnsignedAbs => math("abs", vec![arg()]),
            NumOp::CheckedPow => helper("$checkedPow", vec![arg(), arg(), Expr::int(lo), Expr::int(hi)]),
            // Exact below 2^53, and `$pow` multiplies as `Math.imul` does, so
            // what's past it wraps as Rust's does.
            NumOp::Pow => {
                let power = helper("$pow", vec![arg(), arg()]);
                if num == Num::I32 { power } else { num.wrap(power) }
            }
            NumOp::Powi if num == Num::F32 => helper("$powiF32", vec![arg(), arg()]),
            NumOp::Powi => helper("$powi", vec![arg(), arg()]),
            NumOp::MulAdd if num == Num::F32 => helper("$mulAddF32", vec![arg(), arg(), arg()]),
            NumOp::MulAdd => helper("$mulAdd", vec![arg(), arg(), arg()]),
            // `a ** b`, but where JS's `**` gives NaN, of a base of 1 or -1 to an
            // infinite or NaN power, Rust's is 1: `$powf`, unless the power is a
            // finite constant, `x.powf(2.0)`.
            NumOp::Powf => {
                let (a, b) = (arg(), arg());
                match &b.kind {
                    js::ExprKind::Num(n) if n.is_finite() => rounded(Expr::bin(Op::Pow, a, b)),
                    _ => rounded(helper("$powf", vec![a, b])),
                }
            }
            NumOp::Exp2 => rounded(Expr::bin(Op::Pow, Expr::num(2.0), arg())),
            NumOp::Round => helper("$round", vec![arg()]),
            NumOp::TotalCmp => helper("$totalCmp", vec![arg(), arg()]),
            NumOp::IsNan => number("isNaN", vec![arg()]),
            NumOp::IsFinite => number("isFinite", vec![arg()]),
            NumOp::IsInfinite => Expr::bin(Op::Eq, math("abs", vec![arg()]), Expr::var("Infinity")),
            // By the type's smallest normal value, `MIN_POSITIVE`.
            NumOp::IsNormal { .. } | NumOp::Classify => {
                let min = Expr::num(match num {
                    Num::F32 => f64::from(f32::MIN_POSITIVE),
                    _ => f64::MIN_POSITIVE,
                });
                match op {
                    NumOp::IsNormal { subnormal: true } => helper("$isNormal", vec![arg(), min, Expr::bool(true)]),
                    NumOp::IsNormal { .. } => helper("$isNormal", vec![arg(), min]),
                    _ => helper("$classify", vec![arg(), min]),
                }
            }
            // The exact result, if it's in range. A product past 2^53 is
            // rounded, but it's far out of range either way.
            NumOp::Checked(BinOp::Div) => {
                let min = Expr::int(lo);
                helper("$checkedDiv", vec![arg(), arg(), min])
            }
            NumOp::Checked(BinOp::Rem) => {
                let list = vec![arg(), arg(), Expr::int(lo)];
                helper("$checkedRem", list)
            }
            NumOp::Checked(op) => {
                let exact = Expr::bin(js_op(op), arg(), arg());
                helper("$checked", vec![exact, Expr::int(lo), Expr::int(hi)])
            }
            NumOp::CheckedNeg => {
                let exact = Expr::unary(UnaryOp::Neg, arg());
                helper("$checked", vec![exact, Expr::int(lo), Expr::int(hi)])
            }
            NumOp::CheckedAbs => {
                let exact = math("abs", vec![arg()]);
                helper("$checked", vec![exact, Expr::int(lo), Expr::int(hi)])
            }
            NumOp::Saturating(op) => {
                let exact = Expr::bin(js_op(op), arg(), arg());
                match (num.signed(), op) {
                    (false, BinOp::Sub) => math("max", vec![exact, Expr::int(0)]),
                    (false, _) => math("min", vec![exact, Expr::int(hi)]),
                    // `| 0`, exact in range, makes the -0 of `0 * -5` the integer 0.
                    (true, BinOp::Mul) => Expr::bin(
                        Op::BitOr,
                        math("min", vec![math("max", vec![exact, Expr::int(lo)]), Expr::int(hi)]),
                        Expr::int(0),
                    ),
                    _ => math("min", vec![math("max", vec![exact, Expr::int(lo)]), Expr::int(hi)]),
                }
            }
            NumOp::Wrapping(op) => {
                let (a, b) = (arg(), arg());
                self.binary(op, a, b, None, ty, span)?
            }
            NumOp::RemEuclid if !num.signed() => {
                let (a, b) = (arg(), arg());
                self.binary(BinOp::Rem, a, b, None, ty, span)?
            }
            NumOp::DivEuclid if !num.signed() => {
                let (a, b) = (arg(), arg());
                self.binary(BinOp::Div, a, b, None, ty, span)?
            }
            NumOp::RemEuclid => helper("$remEuclid", vec![arg(), arg(), Expr::int(lo)]),
            NumOp::DivEuclid => helper("$divEuclid", vec![arg(), arg(), Expr::int(lo)]),
            NumOp::IsPositive { negative } => Expr::bin(if negative { Op::Lt } else { Op::Gt }, arg(), Expr::int(0)),
            // Shifted, below the width, as `<<` shifts.
            NumOp::CheckedShift(op) => {
                let (a, by) = (arg(), arg());
                let a = if a.reads_same() { a } else { self.spill("n", a, out) };
                let by = if by.reads_same() { by } else { self.spill("by", by, out) };
                let fits = Expr::bin(Op::Lt, by.clone(), Expr::int(num.bits().into()));
                Expr::cond(fits, self.binary(op, a, by, None, ty, span)?, Expr::undefined())
            }
            NumOp::WrappingNeg => num.wrap(Expr::unary(UnaryOp::Neg, arg())),
            NumOp::WrappingDiv => helper("$wrappingDiv", vec![arg(), arg(), Expr::int(lo)]),
            NumOp::WrappingRem => helper("$wrappingRem", vec![arg(), arg(), Expr::int(lo)]),
            // What wrapping gives, and the exact result, which a product past
            // 2^53 rounds, far out of range either way.
            NumOp::Overflowing(op) => {
                let (a, b) = (arg(), arg());
                let a = if a.reads_same() { a } else { self.spill("a", a, out) };
                let b = if b.reads_same() { b } else { self.spill("b", b, out) };
                let exact = Expr::bin(js_op(op), a.clone(), b.clone());
                let wrapped = self.binary(op, a, b, None, ty, span)?;
                helper("$overflowing", vec![wrapped, exact, Expr::int(lo), Expr::int(hi)])
            }
            NumOp::OverflowingNeg => {
                let x = arg();
                let x = if x.reads_same() { x } else { self.spill("n", x, out) };
                let overflowed = if num.signed() {
                    Expr::bin(Op::Eq, x.clone(), Expr::int(lo))
                } else {
                    Expr::bin(Op::Ne, x.clone(), Expr::int(0))
                };
                Expr::array(vec![num.wrap(Expr::unary(UnaryOp::Neg, x)), overflowed])
            }
            // Of its unsigned bits, the sign bit among them.
            NumOp::RotateBits { left } => {
                let x = match num {
                    Num::I32 => Expr::bin(Op::UShr, arg(), Expr::int(0)),
                    _ => bits(arg()),
                };
                let list = vec![x, arg(), Expr::int(num.bits().into()), Expr::bool(left)];
                let rotated = helper("$rotateBits", list);
                if num.signed() { num.wrap(rotated) } else { rotated }
            }
            NumOp::ToBytes { little } => {
                let list = vec![arg(), Expr::int((num.bits() / 8).into()), Expr::bool(little)];
                helper("$toBytes", list)
            }
            NumOp::FromBytes { little } => {
                let list = vec![
                    arg(),
                    Expr::int((num.bits() / 8).into()),
                    Expr::bool(little),
                    Expr::bool(num.signed()),
                ];
                helper("$fromBytes", list)
            }
            NumOp::ToBits => {
                let list = vec![arg(), Expr::int((num.bits() / 8).into())];
                helper("$floatToBits", list)
            }
            NumOp::FromBits => {
                let list = vec![arg(), Expr::int((num.bits() / 8).into())];
                helper("$floatFromBits", list)
            }
            NumOp::ToIntUnchecked => unreachable!("a cast, in `mir_std_call`"),
            NumOp::EdgeOnes { .. }
            | NumOp::SwapBytes
            | NumOp::ReverseBits
            | NumOp::Endian { .. }
            | NumOp::CheckedEuclid { .. }
            | NumOp::Recip
            | NumOp::FloatToBytes { .. }
            | NumOp::FloatFromBytes { .. } => unreachable!("`bits_call`'s"),
            NumOp::SaturatingPow => {
                let list = vec![arg(), arg(), Expr::int(lo), Expr::int(hi)];
                helper("$saturatingPow", list)
            }
            NumOp::Signum if num.float() => helper("$signum", vec![arg()]),
            NumOp::Signum => math("sign", vec![arg()]),
            NumOp::Clamp if num.float() => {
                let debug = if num == Num::F32 { "$debugF32" } else { "$debugF64" };
                let list = vec![arg(), arg(), arg(), Expr::var(debug)];
                helper("$clampFloat", list)
            }
            NumOp::Clamp => helper("$clamp", vec![arg(), arg(), arg()]),
            NumOp::FromStrRadix => {
                let list = vec![arg(), Expr::int(lo), Expr::int(hi), arg()];
                helper("$parseInt", list)
            }
            NumOp::DivCeil => helper("$divCeil", vec![arg(), arg()]),
            // Exact: what's after the point, in an `f32` as in an `f64`.
            NumOp::Fract => {
                let x = arg();
                let x = if x.reads_same() { x } else { self.spill("x", x, out) };
                Expr::bin(Op::Sub, x.clone(), math("trunc", vec![x]))
            }
            // `x * (PI / 180)` or `x * (180 / PI)`, as std's constants are: an
            // `f32`'s of `f32`s, its degrees per radian a literal.
            NumOp::Angle { radians } => {
                let factor = match (num, radians) {
                    (Num::F32, true) => Expr::num(f64::from(std::f32::consts::PI / 180.0)),
                    (Num::F32, false) => Expr::num(f64::from(F32_DEGREES_PER_RADIAN)),
                    (_, true) => Expr::bin(Op::Div, Expr::member(Expr::var("Math"), "PI"), Expr::num(180.0)),
                    (_, false) => Expr::bin(Op::Div, Expr::num(180.0), Expr::member(Expr::var("Math"), "PI")),
                };
                rounded(Expr::bin(Op::Mul, arg(), factor))
            }
            NumOp::SignNegative { negated } => {
                let negative = helper("$signNegative", vec![arg()]);
                if negated {
                    Expr::unary(UnaryOp::Not, negative)
                } else {
                    negative
                }
            }
            NumOp::Ilog { base } => helper("$ilog", vec![arg(), Expr::int(base.into())]),
            NumOp::Isqrt => helper("$isqrt", vec![arg()]),
            // Rounded toward zero, as Rust's integer division is: exact, as the
            // sum of two 32-bit integers is.
            NumOp::Midpoint => math(
                "trunc",
                vec![Expr::bin(Op::Div, Expr::bin(Op::Add, arg(), arg()), Expr::num(2.0))],
            ),
            NumOp::CountZeros => {
                let ones = helper("$countOnes", vec![bits(arg())]);
                Expr::bin(Op::Sub, Expr::int(num.bits().into()), ones)
            }
            NumOp::LeadingZeros => {
                let zeros = math("clz32", vec![bits(arg())]);
                match num.bits() {
                    32 => zeros,
                    n => Expr::bin(Op::Sub, zeros, Expr::int((32 - n).into())),
                }
            }
            NumOp::TrailingZeros => {
                let x = bits(arg());
                helper("$trailingZeros", vec![x, Expr::int(num.bits().into())])
            }
            NumOp::CountOnes => {
                let x = bits(arg());
                helper("$countOnes", vec![x])
            }
            NumOp::NextPowerOfTwo { checked } => {
                let next = helper("$nextPowerOfTwo", vec![arg(), Expr::int(num.bits().into())]);
                match checked {
                    true => next,
                    false => Expr::bin(Op::Coalesce, next, Expr::int(0)),
                }
            }
            NumOp::IsPowerOfTwo => {
                let x = arg();
                let x = if x.reads_same() { x } else { self.spill("n", x, out) };
                let lower = Expr::bin(Op::BitAnd, x.clone(), Expr::bin(Op::Sub, x.clone(), Expr::int(1)));
                Expr::bin(
                    Op::And,
                    Expr::bin(Op::Ne, x, Expr::int(0)),
                    Expr::bin(Op::Eq, lower, Expr::int(0)),
                )
            }
            NumOp::AbsDiff => math("abs", vec![Expr::bin(Op::Sub, arg(), arg())]),
            // `b >= 48 && b <= 57`, as `(b'0'..=b'9').contains(&b)` is.
            NumOp::AsciiIs(ranges) => {
                let b = arg();
                let b = if b.reads_same() { b } else { self.spill("b", b, out) };
                let int = |n: u8| Expr::int(i128::from(n));
                ranges
                    .iter()
                    .map(|&(lo, hi)| match (lo, hi) {
                        _ if lo == hi => Expr::bin(Op::Eq, b.clone(), int(lo)),
                        (0, _) => Expr::bin(Op::Le, b.clone(), int(hi)),
                        _ => Expr::bin(
                            Op::And,
                            Expr::bin(Op::Ge, b.clone(), int(lo)),
                            Expr::bin(Op::Le, b.clone(), int(hi)),
                        ),
                    })
                    .reduce(|a, b| Expr::bin(Op::Or, a, b))
                    .expect("a range")
            }
            // `b >= 97 && b <= 122 ? b - 32 : b`: a letter's other case, 32 apart.
            NumOp::AsciiCase { upper } => {
                let b = arg();
                let b = if b.reads_same() { b } else { self.spill("b", b, out) };
                let (lo, hi, op) = match upper {
                    true => (b'a', b'z', Op::Sub),
                    false => (b'A', b'Z', Op::Add),
                };
                let letter = Expr::bin(
                    Op::And,
                    Expr::bin(Op::Ge, b.clone(), Expr::int(i128::from(lo))),
                    Expr::bin(Op::Le, b.clone(), Expr::int(i128::from(hi))),
                );
                Expr::cond(letter, Expr::bin(op, b.clone(), Expr::int(32)), b)
            }
        })
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// An `i64`'s or a `u64`'s methods (ADR 0086): a BigInt's, where JS's
    /// `Math` takes only numbers. What they count, `count_ones()` and the
    /// like, is a `u32`, a number.
    fn big_number_call(
        &mut self,
        op: NumOp,
        values: Vec<Expr>,
        ty: Ty<'tcx>,
        num: Num,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let (lo, hi) = num.range();
        let (lo, hi) = (num.literal(lo), num.literal(hi as i128));
        let mut values = values.into_iter();
        let mut arg = || values.next().expect("rustc checked the arguments");
        let helper = |name: &'static str, list: Vec<Expr>| Expr::call(Expr::helper(name), list);
        // A 128-bit one's width, where a helper takes it: 64 is the default.
        let width = |mut list: Vec<Expr>| {
            if num.bits() == 128 {
                list.push(Expr::int(128));
            }
            list
        };
        Ok(match op {
            NumOp::Abs => num.wrap(helper("$bigAbs", vec![arg()])),
            // An `i64::MIN`'s is 2^63, which a `u64` holds.
            NumOp::UnsignedAbs => helper("$bigAbs", vec![arg()]),
            NumOp::Pow => num.wrap(helper("$bigPow", width(vec![arg(), arg()]))),
            NumOp::CheckedPow => helper("$checkedPow", vec![arg(), arg(), lo, hi]),
            NumOp::Checked(BinOp::Rem) => {
                let min = if num.signed() { lo.clone() } else { Expr::undefined() };
                helper("$checkedRem", vec![arg(), arg(), min])
            }
            NumOp::Checked(BinOp::Div) => {
                let min = if num.signed() { lo } else { Expr::undefined() };
                helper("$bigCheckedDiv", vec![arg(), arg(), min])
            }
            NumOp::Checked(op) => {
                let exact = Expr::bin(js_op(op), arg(), arg());
                helper("$bigChecked", vec![exact, lo, hi])
            }
            NumOp::Saturating(op) => {
                let exact = Expr::bin(js_op(op), arg(), arg());
                helper("$bigClamp", vec![exact, lo, hi])
            }
            NumOp::Wrapping(op) => {
                let (a, b) = (arg(), arg());
                self.binary(op, a, b, None, ty, span)?
            }
            NumOp::RemEuclid if !num.signed() => {
                let (a, b) = (arg(), arg());
                self.binary(BinOp::Rem, a, b, None, ty, span)?
            }
            NumOp::DivEuclid if !num.signed() => {
                let (a, b) = (arg(), arg());
                self.binary(BinOp::Div, a, b, None, ty, span)?
            }
            NumOp::RemEuclid => helper("$bigRemEuclid", vec![arg(), arg(), lo]),
            NumOp::DivEuclid => helper("$bigDivEuclid", vec![arg(), arg(), lo]),
            NumOp::Signum => helper("$bigSignum", vec![arg()]),
            NumOp::LeadingZeros => helper("$bigLeadingZeros", width(vec![arg()])),
            NumOp::TrailingZeros => helper("$bigTrailingZeros", width(vec![arg()])),
            NumOp::CountOnes => helper("$bigCountOnes", width(vec![arg()])),
            NumOp::NextPowerOfTwo { checked } => {
                let next = helper("$nextPowerOfTwo", vec![arg(), Expr::int(num.bits().into())]);
                match checked {
                    true => next,
                    false => Expr::bin(Op::Coalesce, next, Expr::bigint(0)),
                }
            }
            NumOp::IsPowerOfTwo => {
                let x = arg();
                let x = if x.reads_same() { x } else { self.spill("n", x, out) };
                let lower = Expr::bin(Op::BitAnd, x.clone(), Expr::bin(Op::Sub, x.clone(), Expr::bigint(1)));
                Expr::bin(
                    Op::And,
                    Expr::bin(Op::Ne, x, Expr::bigint(0)),
                    Expr::bin(Op::Eq, lower, Expr::bigint(0)),
                )
            }
            NumOp::AbsDiff => helper("$bigAbsDiff", vec![arg(), arg()]),
            NumOp::Clamp => helper("$clamp", vec![arg(), arg(), arg()]),
            NumOp::IsPositive { negative } => Expr::bin(if negative { Op::Lt } else { Op::Gt }, arg(), Expr::bigint(0)),
            NumOp::CheckedNeg => {
                let exact = Expr::unary(UnaryOp::Neg, arg());
                helper("$bigChecked", vec![exact, lo, hi])
            }
            NumOp::CheckedAbs => {
                let exact = helper("$bigAbs", vec![arg()]);
                helper("$bigChecked", vec![exact, lo, hi])
            }
            NumOp::CheckedShift(op) => {
                let (a, by) = (arg(), arg());
                let a = if a.reads_same() { a } else { self.spill("n", a, out) };
                let by = if by.reads_same() { by } else { self.spill("by", by, out) };
                let fits = Expr::bin(Op::Lt, by.clone(), Expr::int(i128::from(num.bits())));
                Expr::cond(fits, self.binary(op, a, by, None, ty, span)?, Expr::undefined())
            }
            NumOp::WrappingNeg => num.wrap(Expr::unary(UnaryOp::Neg, arg())),
            NumOp::WrappingDiv => {
                let min = if num.signed() { lo.clone() } else { Expr::undefined() };
                helper("$wrappingDiv", vec![arg(), arg(), min])
            }
            NumOp::WrappingRem => {
                let min = if num.signed() { lo.clone() } else { Expr::undefined() };
                helper("$wrappingRem", vec![arg(), arg(), min])
            }
            // The exact result, and it wrapped.
            NumOp::Overflowing(op) => {
                let exact = Expr::bin(js_op(op), arg(), arg());
                let exact = self.spill("exact", exact, out);
                let wrapped = num.wrap(exact.clone());
                helper("$overflowing", vec![wrapped, exact, lo, hi])
            }
            NumOp::OverflowingNeg => {
                let x = arg();
                let x = if x.reads_same() { x } else { self.spill("n", x, out) };
                let overflowed = if num.signed() {
                    Expr::bin(Op::Eq, x.clone(), lo)
                } else {
                    Expr::bin(Op::Ne, x.clone(), Expr::bigint(0))
                };
                Expr::array(vec![num.wrap(Expr::unary(UnaryOp::Neg, x)), overflowed])
            }
            NumOp::SaturatingPow => helper("$saturatingPow", vec![arg(), arg(), lo, hi]),
            NumOp::RotateBits { left } => {
                let bits = Expr::int(num.bits().into());
                let x = Expr::call(Expr::member(Expr::var("BigInt"), "asUintN"), vec![bits.clone(), arg()]);
                let list = vec![x, arg(), bits, Expr::bool(left)];
                let rotated = helper("$rotateBits", list);
                if num.signed() { num.wrap(rotated) } else { rotated }
            }
            NumOp::ToBytes { little } => helper(
                "$toBytes",
                vec![arg(), Expr::int((num.bits() / 8).into()), Expr::bool(little)],
            ),
            NumOp::FromBytes { little } => {
                let list = vec![
                    arg(),
                    Expr::int((num.bits() / 8).into()),
                    Expr::bool(little),
                    Expr::bool(num.signed()),
                ];
                helper("$fromBytes", list)
            }
            NumOp::FromStrRadix => helper("$parseBig", vec![arg(), lo, hi, arg()]),
            NumOp::DivCeil => helper("$divCeil", vec![arg(), arg()]),
            NumOp::Ilog { base } => helper("$ilog", vec![arg(), Expr::int(base.into())]),
            NumOp::Isqrt => helper("$isqrt", vec![arg()]),
            // Rounded toward zero, as a BigInt's division is.
            NumOp::Midpoint => Expr::bin(Op::Div, Expr::bin(Op::Add, arg(), arg()), Expr::bigint(2)),
            NumOp::CountZeros => {
                let ones = helper("$bigCountOnes", width(vec![arg()]));
                Expr::bin(Op::Sub, Expr::int(num.bits().into()), ones)
            }
            _ => return Err(self.unsupported(span, &format!("this method of a {}-bit integer", num.bits()))),
        })
    }
}

fn js_op(op: BinOp) -> Op {
    match op {
        BinOp::Add => Op::Add,
        BinOp::Sub => Op::Sub,
        _ => Op::Mul,
    }
}

/// A signed remainder compared, `x % 2 === 0`: without the `| 0` that makes
/// JS's `-0` a `0`, as a comparison can't tell them apart.
fn without_zero_sign(e: Expr) -> Expr {
    match e.kind {
        js::ExprKind::Binary(Op::BitOr, ref rem, ref zero)
            if matches!(rem.kind, js::ExprKind::Binary(Op::Rem, ..)) && zero.as_int() == Some(0) =>
        {
            (**rem).clone()
        }
        _ => e,
    }
}

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

        // Integers: compute exactly in JS, then wrap back into range.
        Ok(match op {
            BinOp::Add => num.wrap(Expr::bin(Op::Add, l, r)),
            BinOp::Sub => num.wrap(Expr::bin(Op::Sub, l, r)),
            // A 32-bit product can exceed 2^53 and lose bits; `Math.imul` can't.
            BinOp::Mul if num.bits() == 32 => {
                let product = Expr::call(Expr::member(Expr::var("Math"), "imul"), vec![l, r]);
                if num.signed() { product } else { num.wrap(product) }
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

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A number's conversion, comparison, operator or size (ADRs 0011, 0057): `None` if `known` is another.
    pub(in crate::lower) fn numeric_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Call {
            generic_args,
            tys,
            span,
            ..
        } = call;
        let mut arg = || values.next().expect("rustc checked the arguments");
        let js_span = self.js_span(span);
        Ok(Some(match known {
            Std::Duration(op) => {
                let nanos = |n: i64| Expr::bigint(i128::from(n));
                let helper = |name: &'static str, args: Vec<Expr>| Expr::call(Expr::helper(name), args);
                match op {
                    DurationOp::New => {
                        let (secs, subsec) = (arg(), arg());
                        helper("$durationNew", vec![secs, subsec])
                    }
                    DurationOp::From(1) | DurationOp::As(1) => arg(),
                    DurationOp::From(unit) => Expr::bin(Op::Mul, arg(), nanos(unit)),
                    DurationOp::As(unit) => Expr::bin(Op::Div, arg(), nanos(unit)),
                    DurationOp::Subsec(unit) => {
                        let part = Expr::bin(Op::Rem, arg(), nanos(1_000_000_000));
                        let part = match unit {
                            1 => part,
                            _ => Expr::bin(Op::Div, part, nanos(unit)),
                        };
                        Expr::call(Expr::var("Number"), vec![part])
                    }
                    DurationOp::IsZero => Expr::bin(Op::Eq, arg(), nanos(0)),
                    DurationOp::Add => {
                        let (a, b) = (arg(), arg());
                        helper("$durationAdd", vec![a, b])
                    }
                    DurationOp::Sub => {
                        let (a, b) = (arg(), arg());
                        helper("$durationSub", vec![a, b])
                    }
                    DurationOp::Checked { add: true } => {
                        let (a, b) = (arg(), arg());
                        helper("$durationChecked", vec![Expr::bin(Op::Add, a, b)])
                    }
                    DurationOp::Checked { add: false } => {
                        let (a, b) = (arg(), arg());
                        let a = if a.reads_same() { a } else { self.spill("a", a, out) };
                        let b = if b.reads_same() { b } else { self.spill("b", b, out) };
                        let less = Expr::bin(Op::Lt, a.clone(), b.clone());
                        Expr::cond(less, Expr::undefined(), Expr::bin(Op::Sub, a, b))
                    }
                }
            }
            Std::SliceToArray { into } => {
                let target = if into {
                    generic_args.type_at(1)
                } else {
                    generic_args.type_at(0)
                };
                let ty::Array(_, len) = target.peel_refs().kind() else {
                    unreachable!("recognized as an array")
                };
                let len = len
                    .try_to_target_usize(self.tcx)
                    .ok_or_else(|| self.unsupported(span, "an array of a generic length here"))?;
                let slice = arg();
                // An array of its own, `[T; N]`, is a copy, as a write to the
                // slice's after doesn't change it; of a part, `$slice`'s copy
                // already. A `&[T; N]` is the slice's.
                let fresh = matches!(&slice.kind, js::ExprKind::Array(_))
                    || matches!(&slice.kind, js::ExprKind::Call(callee, _)
                        if callee.helper_name() == Some("$slice"));
                let slice = if slice.reads_same() {
                    slice
                } else {
                    self.spill("slice", slice, out)
                };
                let fits = Expr::bin(Op::Eq, Expr::member(slice.clone(), "length"), Expr::int(len as i128));
                let value = match target.is_ref() || fresh {
                    true => slice.clone(),
                    false => Expr::call(Expr::member(slice.clone(), "slice"), vec![]),
                };
                Expr::cond(fits, result::ok(value), result::err(Expr::undefined()))
            }
            Std::ToBig => Expr::call(Expr::var("BigInt"), vec![arg()]),
            Std::Cast { into } => {
                let (from, to) = match into {
                    true => (generic_args.type_at(0), generic_args.type_at(1)),
                    false => (generic_args.type_at(1), generic_args.type_at(0)),
                };
                let value = arg();
                self.number_from(value, from, to, span)?
            }
            Std::TryFromInt { into } => {
                let target = if into {
                    generic_args.type_at(1)
                } else {
                    generic_args.type_at(0)
                };
                let num = self.num(target, span)?;
                let (lo, hi) = num.range();
                let tried = Expr::call(
                    Expr::helper("$tryFromInt"),
                    vec![arg(), num.literal(lo), num.literal(hi as i128)],
                );
                // A `NonZero`'s, an error of `0` (ADR 0177), its message as the
                // others' is.
                match crate::lower::recognition::is_non_zero_ty(target) {
                    true => Expr::call(
                        Expr::helper("$nonZeroOk"),
                        vec![tried, Expr::str("number would be zero for non-zero type")],
                    ),
                    false => tried,
                }
            }
            Std::FromDigit => Expr::call(Expr::helper("$fromDigit"), vec![arg(), arg()]),
            Std::FromU32 => Expr::call(Expr::helper("$fromU32"), vec![arg()]),
            Std::Cmp => Expr::call(Expr::helper("$cmp"), vec![arg(), arg()]),
            Std::MaxOf(max) => {
                let num = Num::of(tys[0].peel_refs());
                let callee = if num.is_some_and(Num::float) {
                    Expr::helper(if max { "$f64Max" } else { "$f64Min" })
                } else if num.is_some_and(Num::big) {
                    // `Math.max` takes numbers only.
                    Expr::helper(if max { "$bigMax" } else { "$bigMin" })
                } else {
                    Expr::member(Expr::var("Math"), if max { "max" } else { "min" })
                };
                Expr::call(callee, vec![arg(), arg()])
            }
            // An `Ordering` is -1, 0 or 1: `Equal` is the one that's falsy.
            Std::Operator(op) => {
                let ty = generic_args
                    .types()
                    .next()
                    .expect("an operator's trait has a type")
                    .peel_refs();
                let (l, r) = (arg(), arg());
                // `a << &n` of an `i64` `n`: its type, the trait's `Rhs`.
                let r = match generic_args.types().nth(1) {
                    Some(rhs) => crate::lower::std_types::number::shift_amount_of(op, r, ty, rhs),
                    None => r,
                };
                self.binary(op, l, r, None, ty, span)?
            }
            Std::UnaryOperator(op) => {
                let ty = generic_args
                    .types()
                    .next()
                    .expect("an operator's trait has a type")
                    .peel_refs();
                let a = arg();
                self.unary(op, a, ty, span)?
            }
            Std::Reverse => Expr::unary(UnaryOp::Neg, arg()),
            // The size rustc works out for the wasm32 target, which rust-js
            // checks programs for, as a `const` of it has (ADR 0090). A
            // generic function is one JS function for every type, so a type
            // parameter's is given by its caller (ADR 0145).
            Std::NeedsDrop => {
                let of = generic_args.types().next().expect("`needs_drop`'s type argument");
                self.type_fact_value(of, TypeFact::NeedsDrop, span)?
            }
            Std::Layout { fact, of_val } => {
                let of = generic_args.types().next().expect("a size's type argument");
                // A type's, its own fact; a value's of a type rustc knows, its
                // bytes, which may be the value's own.
                let bytes = match of_val && !matches!(of.kind(), ty::Param(_)) {
                    true => Expr::int(self.layout_bytes((fact, of_val), of, span)?),
                    false => self.type_fact_value(of, fact, span)?,
                };
                // What's measured still runs, if it does anything.
                if of_val {
                    let measured = arg();
                    if measured.has_effects() {
                        out.push(StmtKind::Expr(measured).at(js_span));
                    }
                }
                bytes
            }
            _ => return Ok(None),
        }))
    }

    /// A fact of `of` (ADR 0145): its size or alignment, as rustc works it out
    /// for the wasm32 target, or its name; of a type parameter, the one this
    /// function was given for it.
    pub(in crate::lower) fn type_fact_value(&self, of: Ty<'tcx>, fact: TypeFact, span: Span) -> R<Expr> {
        let what = match fact {
            TypeFact::Size => "size_of",
            TypeFact::Align => "align_of",
            TypeFact::Name => "type_name",
            TypeFact::Id => "the `TypeId`",
            TypeFact::NeedsDrop => "needs_drop",
        };
        // An associated type's, `<A as Array>::Item`'s: its trait's
        // dictionary's (ADR 0145).
        if let ty::Alias(
            _,
            alias @ ty::AliasTy {
                kind: ty::Projection { def_id },
                ..
            },
        ) = of.kind()
            && self
                .krate
                .type_facts
                .get(def_id)
                .is_some_and(|facts| facts.iter().any(|&(_, f)| f == fact))
            && let Some(dictionary) = self.evidence_for(alias.trait_ref(self.tcx))
        {
            return Ok(Expr::member(
                dictionary,
                super::super::traits::item_fact_key(self.tcx, *def_id, fact),
            ));
        }
        if let ty::Param(param) = of.kind() {
            // A `T: Any`'s `TypeId` is its dictionary's (ADR 0331).
            if fact == TypeFact::Id
                && self.given_type_fact(param.index, fact).is_none()
                && let Some(any) = any_trait(self.tcx)
                && let Some(dictionary) = self.evidence_for(ty::TraitRef::new(self.tcx, any, [of]))
            {
                return Ok(Expr::call(Expr::member(dictionary, "type_id"), Vec::new()));
            }
            return self
                .given_type_fact(param.index, fact)
                .ok_or_else(|| self.unsupported(span, &format!("`{what}` of a type parameter")));
        }
        if fact == TypeFact::NeedsDrop {
            return Ok(Expr::bool(of.needs_drop(self.tcx, self.typing_env)));
        }
        if !matches!(fact, TypeFact::Name | TypeFact::Id) {
            return Ok(Expr::int(self.layout_bytes((fact, false), of, span)?));
        }
        // The name of one that holds a type parameter, `Cow<'_, T>`: rustc's,
        // with what its caller gives of each parameter's in its place (ADR
        // 0145).
        if fact == TypeFact::Name && of.has_param() {
            return self.name_of_generic(of, span);
        }
        if of.has_param() {
            return Err(self.unsupported(span, &format!("{what} of a type with a type parameter")));
        }
        let of = self
            .tcx
            .normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(of));
        Ok(Expr::str(match fact {
            TypeFact::Id => self.type_id_key(self.tcx.erase_and_anonymize_regions(of)),
            _ => rustc_const_eval::util::type_name(self.tcx, of),
        }))
    }

    /// The name of `of`, a type that holds type parameters, `Cow<'_, T>`:
    /// rustc's, which names a parameter `_`, so each is put in as a type no
    /// program names, `[!; 4000000000]` and up, by its index, and the name
    /// given for it, `TName`, put where its text is.
    fn name_of_generic(&self, of: Ty<'tcx>, span: Span) -> R<Expr> {
        const BASE: u64 = 4_000_000_000;
        let tcx = self.tcx;
        let marked = of.fold_with(&mut ty::BottomUpFolder {
            tcx,
            ty_op: |t: Ty<'tcx>| match t.kind() {
                ty::Param(param) => Ty::new_array(tcx, tcx.types.never, BASE + u64::from(param.index)),
                _ => t,
            },
            lt_op: |r| r,
            ct_op: |c| c,
        });
        let marked = tcx.normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(marked));
        let name = rustc_const_eval::util::type_name(tcx, marked);
        let mut text = name.as_str();
        let mut parts = Vec::new();
        while let Some(at) = text.find("[!; ") {
            let rest = &text[at + 4..];
            let digits = rest.find(']').expect("an array's length");
            let index = rest[..digits].parse::<u64>().expect("a marked parameter") - BASE;
            parts.push(Expr::str(&text[..at]));
            let name = self.given_type_fact(index as u32, TypeFact::Name);
            parts.push(name.ok_or_else(|| self.unsupported(span, "`type_name` of a type parameter"))?);
            text = &rest[digits + 1..];
        }
        parts.push(Expr::str(text));
        Ok(crate::lower::display::join(parts))
    }

    /// The bytes `size_of`, `align_of`, `size_of_val` or `align_of_val`
    /// gives for `of`: its size or alignment, and whether of a value's.
    pub(in crate::lower) fn layout_bytes(&self, (fact, of_val): (TypeFact, bool), of: Ty<'tcx>, span: Span) -> R<i128> {
        let name = match (fact, of_val) {
            (TypeFact::Align, false) => "align_of",
            (TypeFact::Align, true) => "align_of_val",
            (_, false) => "size_of",
            (_, true) => "size_of_val",
        };
        if of.has_param() {
            return Err(self.unsupported(span, &format!("`{name}` of a type parameter")));
        }
        if !of.is_sized(self.tcx, self.typing_env) {
            return Err(self.unsupported(span, &format!("`{name}` of a value without one size")));
        }
        let layout = self
            .layout(of)
            .ok_or_else(|| self.unsupported(span, &format!("`{name}` of this type")))?;
        let bytes = if fact == TypeFact::Align {
            layout.align.abi.bytes()
        } else {
            layout.size.bytes()
        };
        Ok(bytes as i128)
    }
}
