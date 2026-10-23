//! A `Result`'s JS, `{ TAG: "Ok", _0: value }`, as an enum with fields is
//! (ADR 0019): made, tested and read here alone, by lowering, which makes
//! and reads it, and by preparation, which reads a `match` of a `$try`'s
//! back into a `try` (ADR 0637).

use super::*;

/// The property that holds what it holds, either variant's one field.
fn payload() -> String {
    positional(0)
}

/// `Ok`'s name as a `Result`'s `TAG` holds it, or `Err`'s where `ok` isn't.
fn name(ok: bool) -> &'static str {
    if ok { "Ok" } else { "Err" }
}

/// A `Result` whose `TAG` is `tag`, a variant's name or another's tag, of
/// `value`.
pub fn tagged(tag: Expr, value: Expr) -> Expr {
    Expr::object(vec![Prop::Field(TAG.into(), tag), Prop::Field(payload(), value)])
}

/// `Ok(value)`, or `Err(value)` where `ok` isn't.
pub fn made(ok: bool, value: Expr) -> Expr {
    tagged(Expr::str(name(ok)), value)
}

pub fn ok(value: Expr) -> Expr {
    made(true, value)
}

pub fn err(value: Expr) -> Expr {
    made(false, value)
}

/// Whether `result` is an `Ok`, or an `Err` where `ok` isn't:
/// `result.TAG === "Ok"`.
pub fn is(result: Expr, ok: bool) -> Expr {
    Expr::bin(Op::Eq, tag(result), Expr::str(name(ok)))
}

/// Its `TAG`, which variant it is.
pub fn tag(result: Expr) -> Expr {
    Expr::member(result, TAG)
}

/// What it holds, of either variant: `result._0`.
pub fn held(result: Expr) -> Expr {
    Expr::member(result, payload())
}

/// What unwrapping `result` gives where it's an `Ok` just made, a lock's or
/// a `to_value` that can't fail, and what else it's given, a fallback or a
/// message, does nothing seen: what it holds (ADRs 0144, 0449). Each way a
/// `Result` is unwrapped, `unwrap()`, `unwrap_or(d)`, `unwrap_or_else(f)`.
pub fn unwrapped_made(result: &Expr, given: &[Expr]) -> Option<Expr> {
    made_ok(result)
        .filter(|_| given.iter().all(|e| !e.has_effects()))
        .cloned()
}

/// What an `Ok` just made holds, `{ TAG: "Ok", _0: value }`'s `value`.
pub fn made_ok(e: &Expr) -> Option<&Expr> {
    made_ok_payload(e).flatten()
}

/// Whether `e` is an `Ok` just made, and what it holds: `{ TAG: "Ok", _0:
/// value }`'s `value`, or nothing of an `Ok(())`'s, `{ TAG: "Ok" }`.
pub fn made_ok_payload(e: &Expr) -> Option<Option<&Expr>> {
    let ExprKind::Object(props) = &e.kind else {
        return None;
    };
    let ok = |tag: &str, name: &Expr| tag == TAG && matches!(&name.kind, ExprKind::Str(s) if s == self::name(true));
    match props.as_slice() {
        [Prop::Field(tag, name)] if ok(tag, name) => Some(None),
        [Prop::Field(tag, name), Prop::Field(field, value)] if ok(tag, name) && *field == payload() => {
            Some(Some(value))
        }
        _ => None,
    }
}

/// Of a test `is` makes, or its negation, `r.TAG !== "Err"`: what it tests,
/// whether it's of `Ok`, and whether it's `===`.
pub fn tested(test: &Expr) -> Option<(&Expr, bool, bool)> {
    let ExprKind::Binary(op @ (Op::Eq | Op::Ne), tag, name) = &test.kind else {
        return None;
    };
    let ExprKind::Member(result, key) = &tag.kind else {
        return None;
    };
    let ExprKind::Str(name) = &name.kind else {
        return None;
    };
    let ok = match name.as_str() {
        n if n == self::name(true) => true,
        n if n == self::name(false) => false,
        _ => return None,
    };
    (key == TAG).then_some((&**result, ok, *op == Op::Eq))
}

/// Whether `e` reads what `result`, a variable, holds: `result._0`.
pub fn reads_held(e: &Expr, result: &str) -> bool {
    matches!(&e.kind, ExprKind::Member(of, field)
        if *field == payload() && matches!(&of.kind, ExprKind::Var(v) if v == result))
}
