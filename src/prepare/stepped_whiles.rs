//! A `while` that counts its variable last, as a `for` that steps it (ADR
//! 0974); and a counter read and then stepped, `x++` (ADR 0948).

use crate::js::{self, Expr, ExprKind, Name, Op, Stmt, StmtKind};

/// `let i = 0; while (i < n) { ..; i++; }`: `for (let i = 0; i < n; i++)
/// { .. }`, as React counts. Each runs the same: the start, then the test,
/// the body and the step, in turn. A walk, `child = child.sibling`, stays
/// the `while` React writes it as (ADR 0974, amended). Only where:
///
/// - the `let` is right before the loop, and nothing after the loop in its
///   block names the variable, which the `for` keeps to itself;
/// - the test reads it, and the body's last statement alone counts it,
///   `i++`, `at -= 2`, of no label's `continue` of the loop before it,
///   which in a `for` would run the step it skips in the `while`; and by
///   what the body doesn't declare, which the step, after the body's
///   block, can't see;
/// - no function in the loop names it: a `for`'s `let` is one variable for
///   each turn, which a closure made in a turn keeps, where the `while`'s
///   is one for all.
pub(super) fn stepped_whiles(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        if let Some(looped) = stepped(&stmts[i], &stmts[i + 1], &stmts[i + 2..]) {
            stmts[i].kind = looped;
            stmts.remove(i + 1);
        }
        i += 1;
    }
}

/// The `for` of `declared` and `looped`, a `let` and a `while`, followed in
/// their block by `after`.
fn stepped(declared: &Stmt, looped: &Stmt, after: &[Stmt]) -> Option<StmtKind> {
    let StmtKind::Let(name, Some(start)) = &declared.kind else {
        return None;
    };
    let StmtKind::While { label, cond, body } = &looped.kind else {
        return None;
    };
    let (last, rest) = body.split_last()?;
    // Its count: `name += value` and `name -= value`, or `name++` and
    // `name--`, as the printer writes a statement of a step of one; not a
    // walk's, `child = child.sibling`, nor a bit's, `rest &= rest - 1`,
    // which React writes as `while`s.
    let one = |e: &Expr| matches!(e.kind, ExprKind::Num(n) if n == 1.0);
    let (target, step) = match &last.kind {
        StmtKind::Compound(Op::Add, target, by) if one(by) => (target, js::Step::Up),
        StmtKind::Compound(Op::Sub, target, by) if one(by) => (target, js::Step::Down),
        StmtKind::Compound(op @ (Op::Add | Op::Sub), target, by) => (target, js::Step::By(*op, by.clone())),
        _ => return None,
    };
    if !matches!(&target.kind, ExprKind::Var(v) if v == name)
        || !cond.mentions_name(name)
        || js::mentions_of(after, name) > 0
        || js::continues(rest, label.as_ref())
        || js::Writes::sets(rest, name)
        || in_function(body, name)
        || function_names(cond, name)
        || step
            .value()
            .is_some_and(|value| function_names(value, name) || reads_declared(value, rest))
    {
        return None;
    }
    Some(StmtKind::For {
        label: label.clone(),
        name: name.clone(),
        start: start.clone(),
        test: cond.clone(),
        step,
        body: rest.to_vec(),
    })
}

/// Whether `value` reads a name `stmts` declare: the body's, which a
/// `for`'s step, after the body's block, can't see, `i += stride` of a
/// `const stride = ..;` in it.
fn reads_declared(value: &Expr, stmts: &[Stmt]) -> bool {
    let mut declared = false;
    value.visit_names(&mut |v| declared |= js::declares(stmts, v));
    declared
}

/// Whether a function in `stmts`, declared or a value, names the variable
/// `name`, by its id (ADR 1150).
fn in_function(stmts: &[Stmt], name: &Name) -> bool {
    let mut named = false;
    js::each_list(stmts, &mut |list| {
        named |= (list.iter()).any(|s| matches!(&s.kind, StmtKind::Function(f) if js::mentions_of(&f.body, name) > 0));
    });
    js::each_expr(stmts, &mut |e| named |= function_names(e, name));
    named
}

/// Whether `e` is, or holds, a function that names the variable `name`.
fn function_names(e: &Expr, name: &Name) -> bool {
    let mut named = false;
    e.each(&mut |e| {
        named |= match &e.kind {
            ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => js::mentions_of(body, name) > 0,
            ExprKind::Function(function) => js::mentions_of(&function.body, name) > 0,
            _ => false,
        };
    });
    named
}

/// `const a = x; x = a + 1;`, a counter's value read and then stepped, as
/// JS writes it: `const a = x++;`, whose value is the one before the step
/// (ADR 0948). Of a path `x` that does nothing to read (`same_path`), and a
/// step of the number literal `1`, which only a number's is, up or down.
pub(super) fn counters(module: &mut js::Module) {
    let mut each = |stmts: &mut Vec<Stmt>| {
        let mut i = 0;
        while i + 1 < stmts.len() {
            let step = counter_step(&stmts[i], &stmts[i + 1], false);
            if let Some(op) = step
                && let StmtKind::Const(_, read) | StmtKind::Let(_, Some(read)) = &mut stmts[i].kind
            {
                let span = read.span;
                let target = std::mem::replace(read, Expr::taken());
                *read = Expr::stepped(op, target).or_at(span);
                stmts.remove(i + 1);
            }
            i += 1;
        }
    };
    js::each_item_block_mut(&mut module.items, &mut each);
    for item in &mut module.items {
        let js::Item::Namespace(namespace) = item else { continue };
        let Some(class) = namespace.class.as_ref().filter(|class| class.es) else {
            continue;
        };
        if let Some(constructor) = namespace
            .methods
            .iter_mut()
            .find(|method| method.name.is(&class.constructor))
        {
            inline_constructor_steps(&mut constructor.body);
        }
    }
}

/// Of `read`, `const a = x;`, and `write`, `x = a + 1;`, the step, up or
/// down; with `compound`, `x++` or `x += 1` too, which only an ES
/// constructor's field assignment takes in (`inline_constructor_steps`):
/// elsewhere the program's `const a = x; x++;` stays as it wrote it, as
/// React's `act` does (#2079, ADR 0948, amended).
fn counter_step(read: &Stmt, write: &Stmt, compound: bool) -> Option<Op> {
    let (name, read) = match &read.kind {
        StmtKind::Const(name, read) | StmtKind::Let(name, Some(read)) => (name, read),
        _ => return None,
    };
    let (op, target, amount) = match &write.kind {
        StmtKind::Assign(target, value) => match &value.kind {
            ExprKind::Binary(op, from, amount) if matches!(&from.kind, ExprKind::Var(v) if v == name) => {
                (*op, target, &**amount)
            }
            _ => return None,
        },
        // A plain variable has no getter. Its second read sees the old
        // value too, so a previously formed compound can give that value.
        StmtKind::Compound(op, target, amount) if compound && matches!(read.kind, ExprKind::Var(_)) => {
            (*op, target, amount)
        }
        _ => return None,
    };
    (matches!(op, Op::Add | Op::Sub)
        && matches!(amount.kind, ExprKind::Num(n) if n == 1.0)
        && js::same_path(read, target)
        && !read.has_effects())
    .then_some(op)
}

/// Binding an ES constructor's `this` has completed before it assigns
/// a field. Reading that receiver has no observable effect, so the sole
/// use of an adjacent step takes the step in its assignment.
fn inline_constructor_steps(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        // Its compound step, `const a = x; x++; this.id = a;`, is the
        // field's: `this.id = x++;`, here alone (#2079).
        if i + 2 < stmts.len()
            && let Some(op) = counter_step(&stmts[i], &stmts[i + 1], true)
            && let StmtKind::Const(name, _) = &stmts[i].kind
            && let StmtKind::Assign(target, value) = &stmts[i + 2].kind
            && matches!(&target.kind, ExprKind::Member(receiver, _) if matches!(receiver.kind, ExprKind::This))
            && matches!(&value.kind, ExprKind::Var(read) if read == name)
            && js::mentions_of(&stmts[i + 3..], name) == 0
            && let StmtKind::Const(_, read) = &stmts[i].kind
        {
            let stepped = Expr::stepped(op, read.clone()).or_at(read.span);
            if let StmtKind::Assign(_, value) = &mut stmts[i + 2].kind {
                *value = stepped;
            }
            stmts.drain(i..i + 2);
            continue;
        }
        let step = match (&stmts[i].kind, &stmts[i + 1].kind) {
            (StmtKind::Const(name, step), StmtKind::Assign(target, value))
                if matches!(step.kind, ExprKind::Stepped(..))
                    && matches!(&target.kind, ExprKind::Member(receiver, _) if matches!(receiver.kind, ExprKind::This))
                    && matches!(&value.kind, ExprKind::Var(read) if read == name)
                    && js::mentions_of(&stmts[i + 2..], name) == 0 =>
            {
                Some(step.clone())
            }
            _ => None,
        };
        if let Some(step) = step
            && let StmtKind::Assign(_, value) = &mut stmts[i + 1].kind
        {
            *value = step;
            stmts.remove(i);
        } else {
            i += 1;
        }
    }
}
