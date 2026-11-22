//! Expressions of an operator, as JS writes them: a test's negation, a
//! unary operator's and a binary one's, each folding what's known as it's
//! made. Split from make.rs by concept, as a file is at most 1,500 lines.

use super::*;

impl Expr {
    /// The test that holds where this one doesn't, read only for its truth:
    /// `!a` of `a`, `a` of `!a`, `a !== b` of `a === b`, and of a chain, its
    /// parts', `!a || b !== c` of `a && b === c`, where the chain's written.
    /// The one negation of a test.
    pub fn negated_test(self) -> Expr {
        match self.kind {
            ExprKind::Binary(op @ (Op::And | Op::Or), a, b) => {
                let flipped = if op == Op::And { Op::Or } else { Op::And };
                Expr::bin(flipped, a.negated_test(), b.negated_test()).or_at(self.span)
            }
            // Where the `!` was written, what it tests is, a test made of
            // a method's call, `!word` of `word.is_empty()` (#301).
            ExprKind::Unary(UnaryOp::Not, inner) => inner.or_at(self.span),
            // What it does first, then its value's negation, `(s, false)` of
            // `(s, true)` (#523).
            ExprKind::Sequence(mut items) => {
                let last = items.pop().map(Expr::negated_test).unwrap_or_else(|| Expr::bool(true));
                items.push(last);
                Expr {
                    kind: ExprKind::Sequence(items),
                    span: self.span,
                    facts: self.facts,
                    held_as: Default::default(),
                }
            }
            // Of integers, the converse: `!(n < 0)` is `n >= 0`.
            ExprKind::Binary(op @ (Op::Lt | Op::Le | Op::Gt | Op::Ge), a, b) if self.facts.has(Known::NeverNaN) => {
                let converse = match op {
                    Op::Lt => Op::Ge,
                    Op::Le => Op::Gt,
                    Op::Gt => Op::Le,
                    _ => Op::Lt,
                };
                Expr {
                    kind: ExprKind::Binary(converse, a, b),
                    span: self.span,
                    facts: self.facts,
                    held_as: Default::default(),
                }
            }
            kind => Expr::unary(
                UnaryOp::Not,
                Expr {
                    facts: self.facts,
                    kind,
                    span: self.span,
                    held_as: Default::default(),
                },
            ),
        }
    }

    pub fn unary(op: UnaryOp, arg: Expr) -> Expr {
        // `!false` is `true` (ADR 0298). An `Option`'s `!o` of `None` is
        // tested as `o == null` now, so no `!null` is made (#382).
        if let (UnaryOp::Not, ExprKind::Bool(b)) = (op, &arg.kind) {
            return Expr::bool(!b);
        }
        // `!!!a` is `!a`, for every `a`: the inner two make a `bool` of it.
        if let (UnaryOp::Not, ExprKind::Unary(UnaryOp::Not, inner)) = (op, &arg.kind)
            && let ExprKind::Unary(UnaryOp::Not, value) = &inner.kind
        {
            return Expr::unary(UnaryOp::Not, (**value).clone());
        }
        // `!(a === b)` is `a !== b`, for every `a` and `b`.
        if let (UnaryOp::Not, ExprKind::Binary(eq, a, b)) = (op, &arg.kind)
            && let Some(ne) = match eq {
                Op::Eq => Some(Op::Ne),
                Op::Ne => Some(Op::Eq),
                // A test of being set, of either form (`Op::negated_test`).
                _ => eq.negated_test(),
            }
        {
            // JS's `==` of any value may still run a `valueOf`
            // (`Known::Getter`); an `Option`'s presence negated is of the
            // `None` its operator says (`Op::negated_test`).
            return Expr {
                facts: arg.facts.of_evaluation(),
                kind: ExprKind::Binary(ne, a.clone(), b.clone()),
                span: arg.span,
                held_as: Default::default(),
            };
        }
        Expr::new(ExprKind::Unary(op, Box::new(arg)))
    }

    pub fn bin(op: Op, lhs: Expr, rhs: Expr) -> Expr {
        // `typeof p === "function" && p != null` is the `typeof` test: a
        // value of any type but `"object"` and `"undefined"` is there, and a
        // place read twice with nothing run between is the same value (ADR
        // 0298, amended).
        // A strict test of being set folds as a loose one (`null_test`, #2112 C S5).
        if op == Op::And
            && let Some(tested) = super::stmt_walk::null_test(&rhs, true)
            && super::stmt_walk::taken_place(tested)
            && typed_present(&lhs, tested)
        {
            return lhs;
        }
        // `x !== undefined && x == null` is `x === null`, and `x === undefined
        // || x != null` is `x !== null`: `== null` holds of both, a place read
        // twice the same value, an `Option` of an `Option`'s `Some(None)`
        // (ADR 1017).
        if let Some(strict) = null_apart(op, &lhs, &rhs) {
            return strict;
        }
        // `typeof x === "boolean" && x` is `x === true`, and `&& !x` is `x
        // === false`: each the value of the other, of any `x`, a place read
        // twice with nothing run between the same value, as an untagged
        // `Kind::Bool(true)`'s match tests it (rd444).
        if op == Op::And
            && let ExprKind::Binary(Op::Eq, kind, name) = &lhs.kind
            && let ExprKind::Unary(UnaryOp::Typeof, x) = &kind.kind
            && matches!(&name.kind, ExprKind::Str(t) if t == "boolean")
            && super::stmt_walk::taken_place(x)
        {
            let (y, truth) = match &rhs.kind {
                ExprKind::Unary(UnaryOp::Not, y) => (&**y, false),
                _ => (&rhs, true),
            };
            if super::same_path(x, y) {
                return Expr::bin(Op::Eq, (**x).clone(), Expr::bool(truth));
            }
        }
        // `0 < 0` is `false`: a comparison of two numbers written out is its
        // answer, as a person writes it.
        if let (ExprKind::Num(a), ExprKind::Num(b)) = (&lhs.kind, &rhs.kind) {
            let answer = match op {
                Op::Eq | Op::LooseEq | Op::NoneEq(_) => Some(a == b),
                Op::Ne | Op::LooseNe | Op::NoneNe(_) => Some(a != b),
                Op::Lt => Some(a < b),
                Op::Le => Some(a <= b),
                Op::Gt => Some(a > b),
                Op::Ge => Some(a >= b),
                _ => None,
            };
            if let Some(answer) = answer {
                return Expr::bool(answer);
            }
        }
        // And of two strings, a tag's test of a variant made here, `"Ok" ===
        // "Ok"`.
        if let (ExprKind::Str(a), ExprKind::Str(b)) = (&lhs.kind, &rhs.kind) {
            match op {
                Op::Eq | Op::LooseEq | Op::NoneEq(_) => return Expr::bool(a == b),
                Op::Ne | Op::LooseNe | Op::NoneNe(_) => return Expr::bool(a != b),
                _ => {}
            }
        }
        // And of `undefined` or `null` both, `None == null`, a test of an
        // `Option` written out: `==` holds of them, `!=` doesn't.
        if lhs.is_nullish() && rhs.is_nullish() {
            match op {
                // A test of being set of either form: `None` isn't set.
                Op::LooseEq | Op::NoneEq(_) => return Expr::bool(true),
                Op::LooseNe | Op::NoneNe(_) => return Expr::bool(false),
                _ => {}
            }
        }
        // `0 || x` is `x`: `||` gives what's after a falsy number, an
        // ordering's `Equal` before the next part's. And of `true` or
        // `false` first, `&&` and `||` are what JS gives, never running
        // what's after where it wouldn't.
        if op == Op::Or && matches!(lhs.kind, ExprKind::Num(n) if n == 0.0) {
            return rhs;
        }
        if let (Op::And | Op::Or, ExprKind::Bool(first)) = (op, &lhs.kind) {
            return match (op, *first) {
                (Op::And, true) | (Op::Or, false) => rhs,
                _ => lhs,
            };
        }
        // And of `true` or `false` after a comparison, which is a `bool`:
        // `a != null && true` is the comparison; `a != null && false` is
        // `false`, where the comparison does nothing.
        let compared = matches!(
            lhs.kind,
            ExprKind::Binary(
                Op::Eq
                    | Op::Ne
                    | Op::LooseEq
                    | Op::LooseNe
                    | Op::NoneEq(_)
                    | Op::NoneNe(_)
                    | Op::Lt
                    | Op::Le
                    | Op::Gt
                    | Op::Ge,
                ..
            ) | ExprKind::Unary(UnaryOp::Not, _)
        );
        if let (Op::And | Op::Or, ExprKind::Bool(second)) = (op, &rhs.kind)
            && compared
        {
            match (op, *second) {
                (Op::And, true) | (Op::Or, false) => return lhs,
                _ if !lhs.has_effects() => return rhs,
                _ => {}
            }
        }
        // `a != null && !!a` is `!!a`: neither is truthy.
        // A strict test of being set folds as a loose one (`null_test`, #2112 C S5).
        if op == Op::And
            && let Some(tested) = null_test(&lhs, true)
            && let ExprKind::Unary(UnaryOp::Not, inner) = &rhs.kind
            && let ExprKind::Unary(UnaryOp::Not, of) = &inner.kind
            && same_path(tested, of)
        {
            return rhs;
        }
        // `a != null && Array.isArray(a)` is `Array.isArray(a)`: no array is
        // `null` or `undefined`. A chain after it, `&& ..`, is made from the
        // left, this first.
        // A strict test of being set folds as a loose one (`null_test`, #2112 C S5).
        if op == Op::And
            && let Some(tested) = null_test(&lhs, true)
            && let ExprKind::Call(callee, args) = &rhs.kind
            && let (ExprKind::Member(array, is_array), [of]) = (&callee.kind, args.as_slice())
            && matches!(&array.kind, ExprKind::Var(name) if name.is("Array"))
            && is_array == "isArray"
            && same_path(tested, of)
        {
            return rhs;
        }
        // `"" + x + "px"` is `x + "px"`: either makes text of `x` as `+`
        // does, then joins.
        if op == Op::Add
            && matches!(rhs.kind, ExprKind::Str(_) | ExprKind::Template(..))
            && let ExprKind::Binary(Op::Add, empty, x) = &lhs.kind
            && matches!(&empty.kind, ExprKind::Str(s) if s.is_empty())
        {
            return Expr::bin(Op::Add, (**x).clone(), rhs);
        }
        // `"#" + ("" + x)` is `"#" + x`: text joined to `x` makes text of it
        // as `"" +` does. Of text joined already too, `";" + name + ":" +
        // ("" + value)`, React DOM's `";" + name + ":" + value + "px"` (React
        // port gap rd74).
        if op == Op::Add
            && is_text(&lhs)
            && let ExprKind::Binary(Op::Add, empty, x) = &rhs.kind
            && matches!(&empty.kind, ExprKind::Str(s) if s.is_empty())
        {
            return Expr::bin(Op::Add, lhs, (**x).clone());
        }
        Expr::new(ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)))
    }
}
