//! JS's `Math` as std's own (ADR 0887): the one table of which std
//! functions and constants are `Math`'s members, which a call, a function
//! taken as a `fn`, `js::is_defined` and a constant's read all ask.

use super::*;
use crate::lower::recognition::f64_const;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;

/// `std::f64::consts`' constants that are `Math`'s, by their names there
/// and in `Math`: each the same `f64`, as `test/corpus/math_values.rs`
/// holds against native Rust. `TAU`, `FRAC_PI_2` and the rest `Math` has
/// none of; an `f32`'s are `f32`s, not `Math`'s `f64`s.
const CONSTANTS: [(&str, &str); 8] = [
    ("E", "E"),
    ("LN_2", "LN2"),
    ("LN_10", "LN10"),
    ("LOG2_E", "LOG2E"),
    ("LOG10_E", "LOG10E"),
    ("PI", "PI"),
    ("SQRT_2", "SQRT2"),
    ("FRAC_1_SQRT_2", "SQRT1_2"),
];

/// The `Math` function a number's method is, of a receiver of `num`: the
/// one whose call, given the method's arguments as they are, in order, is
/// the method's for every value of `num`, so that the function itself is
/// the method. `Math.clz32` of a 32-bit integer's `leading_zeros`, and an
/// `f64`'s `sqrt`, `sin` or `atan2`, whose call is `Math`'s already (ADR
/// 0064). Not an `f32`'s that's rounded to one (ADR 0122), its `sqrt`, but
/// its exact `floor`, `ceil`, `trunc` and `abs`; not a narrower integer's
/// `leading_zeros`, less the bits it hasn't; and not one whose answer
/// differs for some values, `round`'s of a half, `signum`'s of a `0.0`, or
/// `max`'s of a `NaN` (ADR 0365).
pub(in crate::lower) fn function(op: NumOp, num: Num) -> Option<&'static str> {
    match op {
        _ if num.big() => None,
        NumOp::Math(name) if num == Num::F64 => Some(name),
        NumOp::Math(name @ ("floor" | "ceil" | "trunc" | "abs")) => Some(name),
        // `Math.clz32` reads its number's 32 bits, an `i32`'s sign among them.
        NumOp::LeadingZeros if !num.float() && num.bits() == 32 => Some("clz32"),
        // `i32::MIN`'s is 2^31, which a `u32` holds.
        NumOp::UnsignedAbs => Some("abs"),
        // An integer is never JS's `-0` (ADR 0064).
        NumOp::Signum if !num.float() => Some("sign"),
        _ => None,
    }
}

/// `Math.LN2` of `std::f64::consts::LN_2`, where the constant is one of
/// `Math`'s.
pub(in crate::lower) fn constant(tcx: TyCtxt<'_>, id: DefId) -> Option<Expr> {
    let name = f64_const(tcx, id)?;
    let (_, member) = CONSTANTS.iter().find(|(rust, _)| *rust == name)?;
    Some(Expr::member(Expr::var("Math"), *member))
}

/// What a constant's initializer is, where it's written as one of `Math`'s
/// constants, `const LN2: f64 = std::f64::consts::LN_2`: `Math.LN2`, where
/// a literal, or any other expression, is what rustc computed.
pub(in crate::lower) fn written_constant<'tcx>(
    tcx: TyCtxt<'tcx>,
    thir: &rustc_middle::thir::Thir<'tcx>,
    expr: rustc_middle::thir::ExprId,
) -> Option<Expr> {
    match thir[crate::lower::body_queries::strip(thir, expr)].kind {
        rustc_middle::thir::ExprKind::NamedConst { def_id, .. } => constant(tcx, def_id),
        _ => None,
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The `Math` function a std function is, by `function`: `Math.clz32`
    /// of `u32::leading_zeros`. `None` of one that isn't.
    pub(in crate::lower) fn math_function(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Expr> {
        let Some(Std::Number(op)) = self.recognition().classify(def_id, args) else {
            return None;
        };
        let sig = self
            .tcx
            .fn_sig(def_id)
            .instantiate(self.tcx, args)
            .skip_normalization()
            .skip_binder();
        let num = Num::of(sig.inputs().first()?.peel_refs())?;
        function(op, num).map(|name| Expr::member(Expr::var("Math"), name))
    }
}
