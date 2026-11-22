//! An optional call, `f?.(args)`: what JS evaluates of it, and its one
//! maker (#1369).

use super::*;

impl Expr {
    /// `f?.(args)`, a call of what may be `undefined`, a drop function a
    /// caller with nothing to drop leaves out (ADR 0098): the one maker of
    /// one. JS evaluates no argument of it where `f` is `undefined`, so each
    /// is one that does nothing (`has_effects`), a read or a pure helper's
    /// unboxing, `dropT?.($someValue(x))`; of another, a call or a getter's
    /// read, which Rust evaluates whatever it's given to,
    /// `f === undefined ? arg : f(arg)`, which evaluates it either way.
    /// `drop?.($entryInsert(..))` once wrote nothing where `drop` was none
    /// (#1369; test/architecture-optional-calls.test.ts holds the runtime's).
    pub fn optional_call(callee: Expr, args: Vec<Expr>) -> Expr {
        if !args.iter().any(Expr::has_effects) {
            return Expr::new(ExprKind::OptionalCall(Box::new(callee), args));
        }
        assert!(
            !callee.has_effects(),
            "an optional call's callee is evaluated twice: a read"
        );
        let evaluated = match args.len() {
            1 => args[0].clone(),
            _ => Expr::new(ExprKind::Sequence(args.clone())),
        };
        let absent = Expr::bin(Op::Eq, callee.clone(), Expr::js_undefined());
        Expr::cond(absent, evaluated, Expr::call(callee, args))
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    // An argument that reads, `drop?.(old)`, or a pure helper's unboxing,
    // is an optional call's; one that does something is evaluated either
    // way, `drop === undefined ? write(v) : drop(write(v))` (#1369).
    #[test]
    fn optional_call_evaluates_what_does_something() {
        let read = Expr::optional_call(Expr::var("drop"), vec![Expr::var("old")]);
        assert!(matches!(read.kind, ExprKind::OptionalCall(..)));
        let write = Expr::call(Expr::var("write"), vec![Expr::var("v")]);
        let called = Expr::optional_call(Expr::var("drop"), vec![write]);
        let ExprKind::Cond(_, absent, present) = &called.kind else {
            panic!("not evaluated either way");
        };
        assert!(matches!(absent.kind, ExprKind::Call(..)));
        assert!(matches!(present.kind, ExprKind::Call(..)));
    }
}
