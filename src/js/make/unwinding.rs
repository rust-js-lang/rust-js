//! Unwinding's expressions made and read back: a JS call whose throw is an
//! `Err`, `$try(() => call)`, a panic, `new Error(message)`, what was
//! thrown, `catch_unwind`'s `$catchUnwind(f)`, and what a `try` of each
//! catches or leaves uncaught.

use crate::js::*;

impl Expr {
    /// `$try(() => call)`: a JS call whose throw is an `Err`, its `Result`
    /// (ADR 0035).
    pub fn tried(call: Expr) -> Expr {
        let span = call.span;
        let thunk = Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(call)).at(span)]);
        Expr::call_helper("$try", vec![thunk])
    }

    /// The call `$try(() => call)` tries, of one [`Expr::tried`] made.
    pub fn tried_call(&self) -> Option<&Expr> {
        let ExprKind::Call(callee, args) = &self.kind else {
            return None;
        };
        let (Some("$try"), [thunk]) = (callee.helper_name(), args.as_slice()) else {
            return None;
        };
        match &thunk.kind {
            ExprKind::Arrow(params, body) if params.is_empty() => match body.as_slice() {
                [
                    Stmt {
                        kind: StmtKind::Return(Some(call)),
                        ..
                    },
                ] => Some(call),
                _ => None,
            },
            _ => None,
        }
    }

    /// `new Error(message)`: a panic, of Rust's or a port's (ADR 0012), the
    /// one way rust-js makes one; `new Error()` of an empty message,
    /// `panic!("")`'s, as `message` is `""` either way (ADR 0706).
    pub fn panic(message: Expr) -> Expr {
        let args = match &message.kind {
            ExprKind::Str(text) if text.is_empty() => Vec::new(),
            _ => vec![message],
        };
        Expr::new_(Expr::global("Error"), args)
    }

    /// Of `new Error(..)`, a panic: what it's made of.
    pub fn panic_message(&self) -> Option<&[Expr]> {
        match &self.kind {
            ExprKind::New(class, args) if matches!(&class.kind, ExprKind::Var(name) if name.is("Error")) => Some(args),
            _ => None,
        }
    }

    /// What was thrown of an unwind's payload `pair`, `pair.error`, which
    /// its downcast to a `&JsError` finds (ADR 0710).
    pub fn thrown_error(pair: Expr) -> Expr {
        Expr::member(pair, THROWN_VALUE)
    }

    /// Of `Expr::thrown_error`'s, the payload.
    pub fn thrown_error_of(&self) -> Option<&Expr> {
        match &self.kind {
            ExprKind::Member(pair, field) if field == THROWN_VALUE => Some(pair),
            _ => None,
        }
    }

    /// `$catchUnwind(f)`: `catch_unwind(f)`'s `Result`, `f()`'s value, or
    /// what unwound, `$unwound(e)` of what was thrown (ADR 0660).
    pub fn caught_unwind(f: Expr) -> Expr {
        Expr::call_helper("$catchUnwind", vec![f])
    }

    /// What a `try` of this `Result` holds, and whether its `catch` is given
    /// what was thrown as it is, `$try(() => call)`'s (ADR 0637), or what
    /// unwound, `$unwound(e)`, `$catchUnwind(f)`'s, of `f()`, the closure
    /// called, whose body tidy's applied arrows make it, `() => g(x)`'s
    /// `g(x)`, mapped where the closure says it (ADR 0660); a value nothing
    /// reads there stays where it may throw, `"default" in m;` (#462).
    pub fn caught(&self) -> Option<(Expr, Caught)> {
        if let Some(call) = self.tried_call() {
            return Some((call.clone(), Caught::Thrown));
        }
        let ExprKind::Call(callee, args) = &self.kind else {
            return None;
        };
        let (Some("$catchUnwind"), [f]) = (callee.helper_name(), args.as_slice()) else {
            return None;
        };
        // What makes `f` runs before it's caught, as Rust makes it before
        // `catch_unwind` is called: never in the `try`.
        if f.may_throw() {
            return None;
        }
        Some((Expr::call(f.clone(), Vec::new()), Caught::Unwound))
    }

    /// What a throwing JS call's `Result` is of, where nothing catches its
    /// throw: the call, of `$try(() => call)`, and `await p`, of `await
    /// $settle(p)` (ADRs 0035, 0643). Its throw, or its rejection, goes on
    /// as JS made it.
    pub fn uncaught(&self) -> Option<Expr> {
        if let Some(call) = self.tried_call() {
            return Some(call.clone());
        }
        let ExprKind::Await(settled) = &self.kind else {
            return None;
        };
        let ExprKind::Call(callee, args) = &settled.kind else {
            return None;
        };
        match (callee.helper_name(), args.as_slice()) {
            (Some("$settle"), [promise]) => Some(Expr {
                span: self.span,
                ..Expr::await_(promise.clone())
            }),
            _ => None,
        }
    }
}
