//! Which of a call's last arguments are none (ADRs 0313, 0443, 0441): the
//! one rule, which lowering asks of a call it makes of what it knows the
//! types of and prepare of every call, as `undefined_keys` is an object's.

use super::{Expr, ExprKind, Known};

/// `args` with its last `undefined`s that are no argument left out: one
/// after another argument, a `None` say, `f(x)` of `f(x, undefined)`, as a
/// person leaves an optional one out, only `arguments.length` telling (ADR
/// 0313); one alone too where lowering marked it none, `Known::Absent`, a
/// `None` of a parameter that may be left out or a `()`, `cb()` (ADRs 0443,
/// 0441); a lone one otherwise a value, `push(undefined)`, which a JS
/// method that counts its arguments is given. What JS is given,
/// `GivenUndefined`, stays (ADR 0330).
pub fn left_out(args: &mut Vec<Expr>) {
    while let Some(last) = args.last()
        && matches!(last.kind, ExprKind::Undefined)
        && (args.len() > 1 || last.facts.has(Known::Absent))
    {
        args.pop();
    }
}
