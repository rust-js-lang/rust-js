//! Arithmetic, casts, and methods of integers and `f64` (ADR 0064).
//! A small integer or `f64` is a JS number; `i64`/`u64` use BigInt (ADR 0086).
//! Many number methods map to `Math`: `x.sqrt()` is `Math.sqrt(x)`. Where
//! Rust's answer differs from JS's (`round` of a half, `pow` past 2^53),
//! a helper gives Rust's.

mod generic;
pub(in crate::lower) mod math;
mod operators;

use self::generic::*;
pub(in crate::lower) use self::generic::{int_as, int_out_of};
pub(in crate::lower) use self::operators::*;

use crate::js::StmtKind;
use crate::js::result;
use crate::js::{self, Expr, Op, Stmt, UnaryOp};
use crate::lower::calls::Call;
use crate::lower::discriminants;
use crate::lower::recognition::{Std, TypeFact, any_trait};
use crate::lower::representation::{Num, is_fieldless_enum};
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

/// How an operation past its type's range answers: none, wrapped, kept to
/// the range, or wrapped and whether it was past.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum SignMode {
    Checked,
    Wrapping,
    Saturating,
    Overflowing,
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
    /// `x.round_ties_even()`: a tie to the even integer.
    RoundTiesEven,
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
    /// `min_value()` or `max_value()` (`max`) of an integer, deprecated for
    /// `MIN` and `MAX`, which their bodies give.
    Bound {
        max: bool,
    },
    /// `bit_width()` of an unsigned integer: its bits but its leading
    /// zeros, as std's body has it.
    BitWidth,
    TrailingZeros,
    CountOnes,
    /// `isolate_lowest_one()`, or `isolate_highest_one()` (`highest`): its
    /// bits but the one, as std's body has it, of the operators.
    IsolateOne {
        highest: bool,
    },
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
    /// A float's `log(base)`: its `ln()` over the base's, as std's is.
    Log,
    /// `sin_cos()`: the pair.
    SinCos,
    /// A float's `rem_euclid(rhs)`, or `div_euclid(rhs)`, as std computes it.
    FloatEuclid {
        rem: bool,
    },
    /// `abs_sub(other)`, deprecated: the difference where it's positive, else 0.
    AbsSub,
    /// `copysign(sign)`: its magnitude, given `sign`'s sign.
    CopySign,
    /// A float's `midpoint(other)`: halfway, which an `f64`'s finds without
    /// going past its largest; an `f32`'s is `f64`s' sum halved, as std's is.
    FloatMidpoint,
    /// `to_radians()`, or `to_degrees()`.
    Angle {
        radians: bool,
    },
    /// `is_sign_negative()`, or `is_sign_positive()`, its negation.
    SignNegative {
        negated: bool,
    },
    /// `ilog2()` or `ilog10()`: how often `base` divides into it; `None`
    /// of what isn't positive, `checked`, `checked_ilog10()`'s.
    Ilog {
        base: u32,
        checked: bool,
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
    /// `x.saturating_neg()` and `x.saturating_abs()`: past the range, `MAX`,
    /// as only `-MIN` is.
    SaturatingNeg {
        abs: bool,
    },
    WrappingDiv,
    /// `a.overflowing_div(b)`, or `overflowing_rem`, `rem`: what
    /// `wrapping_div` gives and whether it wrapped.
    OverflowingDiv {
        rem: bool,
    },
    /// `a.saturating_div(b)`: `MIN / -1` is `MAX`.
    SaturatingDiv,
    /// `a.is_multiple_of(b)` of an unsigned integer.
    IsMultipleOf,
    /// `x.cast_unsigned()` or `x.cast_signed()`: `as` the other type of its
    /// width.
    CastSign,
    /// `a.wrapping_rem_euclid(b)`, or `wrapping_div_euclid`, `div`.
    WrappingEuclid {
        div: bool,
    },
    /// `a.checked_next_multiple_of(b)` of an unsigned integer, or
    /// `next_multiple_of`, which wraps past its range, `checked: false`.
    NextMultipleOf {
        checked: bool,
    },
    /// `x.ilog(base)`, or `checked_ilog`: how many times `base` divides it.
    IlogBase {
        checked: bool,
    },
    /// `x.checked_isqrt()` of a signed integer: none of a negative.
    CheckedIsqrt,
    /// `a.overflowing_shl(n)` or `overflowing_shr`, `right`: the wrapping
    /// shift, and whether `n` is past the width.
    OverflowingShift {
        right: bool,
    },
    /// `x.overflowing_abs()`: the wrapping one, and whether it's `MIN`.
    OverflowingAbs,
    /// `x.overflowing_pow(n)`: the wrapping power, and whether it wrapped.
    OverflowingPow,
    /// `a.checked_add_signed(b)` of an unsigned integer and a signed one, or
    /// `a.checked_add_unsigned(b)` of a signed and an unsigned one, and
    /// their wrapping, saturating and overflowing forms.
    AddOtherSign(SignMode),
    /// `a.checked_signed_diff(b)` of unsigned integers: `a - b` if a signed
    /// one of their width holds it.
    CheckedSignedDiff,
    /// `a.eq_ignore_ascii_case(&b)` of two bytes.
    BytesEqIgnoreAsciiCase,
    /// `a.unbounded_shl(n)` or `unbounded_shr`, `right`: as `<<` or `>>`
    /// below the width, past it 0, or a negative's `-1` shifted right.
    UnboundedShift {
        right: bool,
    },
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

    /// A `Wrapping`'s operator's value, `[a[0] op rhs]`, of its type, read
    /// the same each time, and the operand.
    pub(in crate::lower) fn wrapping_value(
        &mut self,
        op: Result<BinOp, UnOp>,
        wrapping: Ty<'tcx>,
        target: Expr,
        rhs: Option<Expr>,
        span: Span,
    ) -> R<Expr> {
        let inner = self.recognition().wrapping_of(wrapping).expect("a `Wrapping`");
        self.num(inner, span)?;
        let result = match op {
            Err(op) => self.unary(op, Expr::index(target, Expr::int(0)), inner, span)?,
            Ok(op) => {
                let rhs = rhs.expect("a binary operator's operand");
                self.wrapping_result(op, Expr::index(target, Expr::int(0)), rhs, inner, span)?
            }
        };
        Ok(Expr::array(vec![result]))
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

    /// A number's method written the same of a number and of a BigInt: a
    /// runtime helper of both, or a cast. `Err` of the values of another.
    fn either_width_call(
        &mut self,
        op: NumOp,
        values: Vec<Expr>,
        ty: Ty<'tcx>,
        num: Num,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Result<Expr, Vec<Expr>>> {
        let (lo, hi) = num.range();
        let min = match num.signed() {
            true => num.literal(lo),
            false => Expr::undefined(),
        };
        let helper = |name: &'static str, list: Vec<Expr>| Expr::call(Expr::helper(name), list);
        let [a, rest @ ..] = values.as_slice() else {
            return Ok(Err(values));
        };
        let (a, b) = (a.clone(), rest.first().cloned());
        Ok(Ok(match (op, b) {
            (NumOp::OverflowingDiv { rem }, Some(b)) => {
                let name = if rem { "$overflowingRem" } else { "$overflowingDiv" };
                helper(name, vec![a, b, min])
            }
            (NumOp::SaturatingDiv, Some(b)) => helper("$saturatingDiv", vec![a, b, min, num.literal(hi as i128)]),
            (NumOp::IsMultipleOf, Some(b)) => helper("$isMultipleOf", vec![a, b]),
            (NumOp::WrappingEuclid { div }, Some(b)) => {
                let name = if div {
                    "$wrappingDivEuclid"
                } else {
                    "$wrappingRemEuclid"
                };
                helper(name, vec![a, b, min])
            }
            (NumOp::NextMultipleOf { checked }, Some(b)) => {
                let name = if checked {
                    "$checkedNextMultipleOf"
                } else {
                    "$nextMultipleOf"
                };
                helper(name, vec![a, b, num.literal(hi as i128)])
            }
            (NumOp::IlogBase { checked }, Some(base)) => helper("$ilog", vec![a, base, Expr::bool(checked)]),
            (NumOp::CheckedIsqrt, None) => helper("$checkedIsqrt", vec![a]),
            // std's bodies, of the operators: `self & self.wrapping_neg()`,
            // `lanes & -lanes`; and `self & (MIN_BIT.wrapping_shr(
            // self.leading_zeros()))`, its highest bit shifted down to the
            // highest it has, `0`'s shift masked to none (ADR 0156, amended).
            (NumOp::IsolateOne { highest }, None) => {
                let a = if a.reads_same() { a } else { self.spill("value", a, out) };
                let other = match highest {
                    false => self.unary(UnOp::Neg, a.clone(), ty, span)?,
                    true => {
                        let zeros = self.number_call(NumOp::LeadingZeros, vec![a.clone()], ty, span, out)?;
                        let top = num.literal(if num.signed() { lo } else { 1 << (num.bits() - 1) });
                        self.binary(BinOp::Shr, top, zeros, None, ty, span)?
                    }
                };
                self.binary(BinOp::BitAnd, a, other, None, ty, span)?
            }
            (NumOp::OverflowingShift { right }, Some(n)) => {
                let n = if n.reads_same() { n } else { self.spill("by", n, out) };
                let past = Expr::bin(Op::Ge, n.clone(), Expr::int(i128::from(num.bits())));
                let op = if right { BinOp::Shr } else { BinOp::Shl };
                Expr::array(vec![self.binary(op, a, n, None, ty, span)?, past])
            }
            (NumOp::OverflowingAbs, None) => {
                let a = if a.reads_same() { a } else { self.spill("value", a, out) };
                let at_min = Expr::bin(Op::Eq, a.clone(), num.literal(lo));
                let negated = Expr::unary(UnaryOp::Neg, a.clone());
                let abs = Expr::cond(Expr::bin(Op::Lt, a.clone(), num.literal(0)), negated, a.clone());
                let wrapped = Expr::cond(at_min.clone(), num.literal(lo), abs);
                Expr::array(vec![wrapped, at_min])
            }
            (NumOp::OverflowingPow, Some(n)) => {
                let a = if a.reads_same() { a } else { self.spill("base", a, out) };
                let n = if n.reads_same() { n } else { self.spill("exp", n, out) };
                let exact = helper("$powExact", vec![a.clone(), n.clone()]);
                let wrapped = self.number_call(NumOp::Pow, vec![a, n], ty, span, out)?;
                helper(
                    "$overflowing",
                    vec![wrapped, exact, num.literal(lo), num.literal(hi as i128)],
                )
            }
            (NumOp::AddOtherSign(mode), Some(b)) => {
                let exact = Expr::bin(Op::Add, a, b);
                let (lo, hi) = (num.literal(lo), num.literal(hi as i128));
                match mode {
                    SignMode::Checked if num.big() => helper("$bigChecked", vec![exact, lo, hi]),
                    SignMode::Checked => helper("$checked", vec![exact, lo, hi]),
                    SignMode::Wrapping => num.wrap(exact),
                    SignMode::Saturating => helper("$saturate", vec![exact, lo, hi]),
                    SignMode::Overflowing => {
                        let exact = self.spill("exact", exact, out);
                        helper("$overflowing", vec![num.wrap(exact.clone()), exact, lo, hi])
                    }
                }
            }
            (NumOp::CheckedSignedDiff, Some(b)) => {
                let half = 1i128 << (num.bits() - 1);
                let checked = if num.big() { "$bigChecked" } else { "$checked" };
                helper(
                    checked,
                    vec![Expr::bin(Op::Sub, a, b), num.literal(-half), num.literal(half - 1)],
                )
            }
            (NumOp::BytesEqIgnoreAsciiCase, Some(b)) => helper("$bytesEqIgnoreAsciiCase", vec![a, b]),
            (NumOp::UnboundedShift { right }, Some(n)) => {
                let a = if a.reads_same() { a } else { self.spill("value", a, out) };
                let n = if n.reads_same() { n } else { self.spill("by", n, out) };
                let past = Expr::bin(Op::Ge, n.clone(), Expr::int(i128::from(num.bits())));
                let op = if right { BinOp::Shr } else { BinOp::Shl };
                let shifted = self.binary(op, a.clone(), n, None, ty, span)?;
                let filled = match right && num.signed() {
                    true => Expr::cond(Expr::bin(Op::Lt, a, num.literal(0)), num.literal(-1), num.literal(0)),
                    false => num.literal(0),
                };
                Expr::cond(past, filled, shifted)
            }
            (NumOp::CastSign, None) => {
                let other = match ty.kind() {
                    ty::Int(int) => Ty::new_uint(self.tcx, int.to_unsigned()),
                    ty::Uint(uint) => Ty::new_int(self.tcx, uint.to_signed()),
                    _ => return Ok(Err(values)),
                };
                self.cast(a, ty, other, span)?
            }
            _ => return Ok(Err(values)),
        }))
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
        if let NumOp::Bound { max } = op {
            let (lo, hi) = num.range();
            return Ok(num.literal(if max { hi as i128 } else { lo }));
        }
        let values = match self.bits_call(op, values, num)? {
            Ok(value) => return Ok(value),
            Err(values) => values,
        };
        let values = match self.either_width_call(op, values, ty, num, span, out)? {
            Ok(value) => return Ok(value),
            Err(values) => values,
        };
        // `Math`'s function, its arguments as they are (ADR 0887).
        if let Some(name) = math::function(op, num) {
            // `Math.clz32` takes its argument modulo 2^32 itself, `lanes as
            // u32`'s wrap too (ADR 0895).
            let values = match name {
                "clz32" => values.into_iter().map(operators::bits_operand).collect(),
                _ => values,
            };
            return Ok(Expr::call(Expr::member(Expr::var("Math"), name), values));
        }
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
            NumOp::OverflowingDiv { .. }
            | NumOp::SaturatingDiv
            | NumOp::IsMultipleOf
            | NumOp::CastSign
            | NumOp::WrappingEuclid { .. }
            | NumOp::NextMultipleOf { .. }
            | NumOp::UnboundedShift { .. }
            | NumOp::IlogBase { .. }
            | NumOp::CheckedIsqrt
            | NumOp::OverflowingShift { .. }
            | NumOp::OverflowingAbs
            | NumOp::OverflowingPow
            | NumOp::AddOtherSign(_)
            | NumOp::CheckedSignedDiff
            | NumOp::IsolateOne { .. }
            | NumOp::BytesEqIgnoreAsciiCase => {
                unreachable!("lowered by either_width_call")
            }
            NumOp::Bound { .. } => unreachable!("lowered first"),
            // An `f32`'s, rounded to one: `Math`'s are `math::function`'s.
            NumOp::Math(name) => {
                let list = values.collect();
                rounded(math(name, list))
            }
            NumOp::Abs => num.wrap(math("abs", vec![arg()])),
            NumOp::UnsignedAbs => unreachable!("`math::function`'s"),
            NumOp::CheckedPow => helper("$checkedPow", vec![arg(), arg(), Expr::int(lo), Expr::int(hi)]),
            // Exact below 2^53, and `$pow` multiplies as `Math.imul` does, so
            // what's past it wraps as Rust's does.
            NumOp::Pow => {
                let power = helper("$pow", vec![arg(), arg()]);
                if num == Num::I32 { power } else { num.wrap(power) }
            }
            NumOp::Powi if num == Num::F32 => helper("$powiF32", vec![arg(), arg()]),
            // A variable's square, which `$powi` multiplies once, `x * x`,
            // as Rust's does: the same value, read twice with nothing run
            // between; or a field's of one, or of `this`, whose read does
            // nothing, a getter's not. Not `x ** 2`, whose precision is the
            // engine's.
            NumOp::Powi => match (arg(), arg()) {
                (x, n) if n.as_int() == Some(2) && read_twice(&x) => Expr::bin(Op::Mul, x.clone(), x),
                (x, n) => helper("$powi", vec![x, n]),
            },
            NumOp::MulAdd if num == Num::F32 => helper("$mulAddF32", vec![arg(), arg(), arg()]),
            NumOp::MulAdd => helper("$mulAdd", vec![arg(), arg(), arg()]),
            // `a ** b`, but where JS's `**` gives NaN, of a base of 1 or -1 to an
            // infinite or NaN power, Rust's is 1: `$powf`, unless the power is a
            // finite constant, `x.powf(2.0)`, or the base a constant but ±1,
            // `2f64.powf(n)`.
            NumOp::Powf => {
                let (a, b) = (arg(), arg());
                match (&a.kind, &b.kind) {
                    (_, js::ExprKind::Num(n)) if n.is_finite() => rounded(Expr::bin(Op::Pow, a, b)),
                    (js::ExprKind::Num(n), _) if n.abs() != 1.0 => rounded(Expr::bin(Op::Pow, a, b)),
                    _ => rounded(helper("$powf", vec![a, b])),
                }
            }
            NumOp::Exp2 => rounded(Expr::bin(Op::Pow, Expr::num(2.0), arg())),
            NumOp::Round => helper("$round", vec![arg()]),
            NumOp::RoundTiesEven => helper("$roundTiesEven", vec![arg()]),
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
            NumOp::SaturatingNeg { abs } => {
                let operand = arg();
                // Of 0, `-0` is JS's `-0`, which `| 0` makes the integer 0, as
                // `saturating_mul`'s does; of a constant that isn't, it's needless.
                let zero = !abs && operand.as_int().is_none_or(|n| n == 0);
                let exact = match abs {
                    true => math("abs", vec![operand]),
                    false => Expr::unary(UnaryOp::Neg, operand),
                };
                let saturated = math("min", vec![exact, Expr::int(hi)]);
                if zero {
                    Expr::bin(Op::BitOr, saturated, Expr::int(0))
                } else {
                    saturated
                }
            }
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
            NumOp::Signum => unreachable!("`math::function`'s"),
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
            NumOp::Log => rounded(Expr::bin(Op::Div, math("log", vec![arg()]), math("log", vec![arg()]))),
            NumOp::SinCos => {
                let x = arg();
                let x = if x.reads_same() { x } else { self.spill("x", x, out) };
                Expr::array(vec![
                    rounded(math("sin", vec![x.clone()])),
                    rounded(math("cos", vec![x])),
                ])
            }
            NumOp::FloatEuclid { rem } => rounded(helper("$floatEuclid", vec![arg(), arg(), Expr::bool(rem)])),
            NumOp::AbsSub => rounded(helper("$absSub", vec![arg(), arg()])),
            NumOp::FloatMidpoint if num == Num::F32 => {
                rounded(Expr::bin(Op::Div, Expr::bin(Op::Add, arg(), arg()), Expr::int(2)))
            }
            NumOp::FloatMidpoint => helper("$floatMidpoint", vec![arg(), arg()]),
            // The sign of `sign`, `-0`'s too, by `$signNegative`.
            NumOp::CopySign => {
                let x = math("abs", vec![arg()]);
                let x = if x.reads_same() { x } else { self.spill("x", x, out) };
                let negative = helper("$signNegative", vec![arg()]);
                Expr::cond(negative, Expr::unary(UnaryOp::Neg, x.clone()), x)
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
            NumOp::Ilog { base, checked } => ilog(arg(), base, checked),
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
            // A narrower integer's, of its bits as a `u32`'s, less the bits
            // it hasn't: a 32-bit one's is `math::function`'s.
            NumOp::LeadingZeros => {
                let clz32 = math::function(op, Num::U32).expect("a `u32`'s is `Math`'s");
                let zeros = math(clz32, vec![bits(arg())]);
                Expr::bin(Op::Sub, zeros, Expr::int((32 - num.bits()).into()))
            }
            // Its 32 bits but their leading zeros, whatever its own width.
            NumOp::BitWidth => {
                let clz32 = math::function(NumOp::LeadingZeros, Num::U32).expect("a `u32`'s is `Math`'s");
                Expr::bin(Op::Sub, Expr::int(32), math(clz32, vec![bits(arg())]))
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
            NumOp::BitWidth => Expr::bin(
                Op::Sub,
                Expr::int(num.bits().into()),
                helper("$bigLeadingZeros", width(vec![arg()])),
            ),
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
            NumOp::SaturatingNeg { abs } => {
                let exact = match abs {
                    true => helper("$bigAbs", vec![arg()]),
                    false => Expr::unary(UnaryOp::Neg, arg()),
                };
                helper("$clamp", vec![exact, lo, hi])
            }
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
            NumOp::Ilog { base, checked } => ilog(arg(), base, checked),
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

/// Whether `e` is a variable, `this`, or a field of one, whose read does
/// nothing: read twice, `x * x`, it's the same value.
fn read_twice(e: &Expr) -> bool {
    fn place(e: &Expr) -> bool {
        match &e.kind {
            js::ExprKind::Var(_) | js::ExprKind::This => true,
            js::ExprKind::Member(of, _) => place(of),
            _ => false,
        }
    }
    place(e) && !e.has_effects()
}
