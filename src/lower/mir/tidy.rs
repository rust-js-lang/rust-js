//! A body lowered from its MIR, as a person writes it (ADR 0364): each
//! rewrite changes nothing that runs. MIR's one return block is a `return`
//! where each path ends, and what the structure needed to reach it, a
//! labeled block or a `break`, goes. A closure's body is tidied as its own.

use std::collections::HashSet;

use crate::js::{self, Expr, ExprKind, Op, Stmt, StmtKind, UnaryOp};

use super::negate;

/// `body`, tidied: `result` is the name of the function's return place.
pub(super) fn tidy(body: &mut Vec<Stmt>, result: &str) {
    // Twice: what a block's statements, in its place, are followed by, and
    // a loop's label nothing names then.
    for _ in 0..2 {
        js::statement_lists(body, &mut |stmts| {
            returned(stmts, result);
            unreachable_after(stmts);
        });
        js::statement_lists(body, &mut |stmts| tail_jumps(stmts));
        bottom_up(body, &mut branch_values);
        js::statement_lists(body, &mut unlabeled);
    }
    bottom_up(body, &mut branch_values);
    let mut read = HashSet::new();
    js::visit_stmts(body, &mut |name| {
        read.insert(name.to_string());
    });
    if !read.contains(result) {
        body.retain(|s| !matches!(&s.kind, StmtKind::Let(name, None) if name == result));
    }
    declarations(body);
    aliases(body);
    let changing = changing(body);
    js::statement_lists(body, &mut |stmts| inlined(stmts, &changing));
}

/// Each list of statements in `stmts`, the nested ones first, not in its
/// closures: for a pass that a list's statements, done, make apply.
fn bottom_up(stmts: &mut Vec<Stmt>, f: &mut dyn FnMut(&mut Vec<Stmt>)) {
    for stmt in stmts.iter_mut() {
        match &mut stmt.kind {
            StmtKind::If(_, a, b) => {
                bottom_up(a, f);
                if let Some(b) = b {
                    bottom_up(b, f);
                }
            }
            StmtKind::While { body, .. }
            | StmtKind::ForOf { body, .. }
            | StmtKind::For { body, .. }
            | StmtKind::Labeled(_, body) => bottom_up(body, f),
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                bottom_up(a, f);
                bottom_up(b, f);
            }
            _ => {}
        }
    }
    f(stmts);
}

/// What a branch makes a value of, as a person writes it: `if (c) {} else
/// { s }` is `if (!c) { s }`; `if (c) { x = a; } else { x = b; }` is `x =
/// c ? a : b;`, of two branches; and `l: { if (a) { if (b) { x = t; break l; } } x = f; }`,
/// `matches!`'s, is `x = a && b ? t : f;`.
fn branch_values(stmts: &mut Vec<Stmt>) {
    for stmt in stmts.iter_mut() {
        let span = stmt.span;
        let kind = std::mem::replace(&mut stmt.kind, StmtKind::Break(None));
        stmt.kind = match kind {
            StmtKind::If(test, then, Some(otherwise)) if then.is_empty() => StmtKind::If(not(test), otherwise, None),
            StmtKind::If(test, then, Some(otherwise)) if otherwise.is_empty() => StmtKind::If(test, then, None),
            // A test of two numbers that fails, a check of a constant
            // divisor's, `if (2 === 0)`: nothing.
            StmtKind::If(test, _, None) if never(&test) => StmtKind::Expr(Expr::undefined()),
            StmtKind::If(test, ref then, Some(ref otherwise))
                if let Some((x, yes)) = assigned_only(then)
                    && let Some((y, no)) = assigned_only(otherwise)
                    && x == y
                    // Not one of more branches, which reads best as they are.
                    && small(yes)
                    && small(no) =>
            {
                StmtKind::Assign(Expr::var(x), conditional(test, yes.clone(), no.clone()))
            }
            // `if (c) { return a; } else { return b; }`: `return c ? a : b;`.
            StmtKind::If(test, ref then, Some(ref otherwise))
                if let [
                    Stmt {
                        kind: StmtKind::Return(Some(yes)),
                        ..
                    },
                ] = then.as_slice()
                    && let [
                        Stmt {
                            kind: StmtKind::Return(Some(no)),
                            ..
                        },
                    ] = otherwise.as_slice()
                    && small(yes)
                    && small(no) =>
            {
                StmtKind::Return(Some(conditional(test, yes.clone(), no.clone())))
            }
            // `l: { if (!a) { if (!b) { s; break l; } } t }`, `||`'s: `if (a ||
            // b) { t } else { s }`, where nothing else breaks to `l`.
            StmtKind::Labeled(label, body) if let Some((tests, otherwise, then)) = or_chain(&body, &label) => {
                let test = (tests.into_iter()).rev().reduce(|b, a| both(a, b)).expect("one");
                match then.is_empty() {
                    true => StmtKind::If(test, otherwise, None),
                    false => StmtKind::If(not(test), then, Some(otherwise)),
                }
            }
            StmtKind::Labeled(label, body) => match chain(&body, &label) {
                Some((tests, x, yes, no)) => {
                    let test = (tests.into_iter()).rev().reduce(|b, a| both(a, b)).expect("one");
                    StmtKind::Assign(Expr::var(&x), conditional(test, yes, no))
                }
                None => StmtKind::Labeled(label, body),
            },
            kind => kind,
        };
        stmt.span = span;
    }
    stmts.retain(|stmt| !matches!(&stmt.kind, StmtKind::Expr(e) if matches!(e.kind, ExprKind::Undefined)));
    // `if (a) { if (b) { return t; } } return f;`, last: `return a && b ? t : f;`.
    if let [.., last_if, last] = stmts.as_slice()
        && let StmtKind::Return(Some(no)) = &last.kind
        && let StmtKind::If(..) = &last_if.kind
        && let Some((tests, yes)) = returned_chain(std::slice::from_ref(last_if))
        && small(&yes)
        && small(no)
    {
        let test = (tests.into_iter()).rev().reduce(|b, a| both(a, b)).expect("one");
        let value = conditional(test, yes, no.clone());
        let span = last_if.span;
        stmts.truncate(stmts.len() - 2);
        stmts.push(StmtKind::Return(Some(value)).at(span));
    }
}

/// Of `if (a) { if (b) { return t; } }`: its tests, `[a, b]`, and `t`.
fn returned_chain(stmts: &[Stmt]) -> Option<(Vec<Expr>, Expr)> {
    match stmts {
        [
            Stmt {
                kind: StmtKind::Return(Some(yes)),
                ..
            },
        ] => Some((Vec::new(), yes.clone())),
        [
            Stmt {
                kind: StmtKind::If(test, inner, None),
                ..
            },
        ] => {
            let (mut tests, yes) = returned_chain(inner)?;
            tests.insert(0, test.clone());
            Some((tests, yes))
        }
        _ => None,
    }
}

/// Whether `test` compares two numbers it holds, and fails.
fn never(test: &Expr) -> bool {
    let ExprKind::Binary(op, a, b) = &test.kind else {
        return false;
    };
    let (ExprKind::Num(a), ExprKind::Num(b)) = (&a.kind, &b.kind) else {
        return false;
    };
    !match op {
        Op::Eq | Op::LooseEq => a == b,
        Op::Ne | Op::LooseNe => a != b,
        Op::Lt => a < b,
        Op::Le => a <= b,
        Op::Gt => a > b,
        Op::Ge => a >= b,
        _ => return false,
    }
}

/// Whether `e` reads well as a branch of `c ? a : b`: not a conditional
/// itself, of more branches, nor an element, which reads best returned or
/// assigned as it is, nor a function.
fn small(e: &Expr) -> bool {
    !matches!(
        e.kind,
        ExprKind::Cond(..) | ExprKind::Jsx(_) | ExprKind::Arrow(..) | ExprKind::AsyncArrow(..) | ExprKind::Function(_)
    )
}

/// The variable `stmts` only assign, and what to: `x = a;`.
fn assigned_only(stmts: &[Stmt]) -> Option<(&str, &Expr)> {
    match stmts {
        [
            Stmt {
                kind: StmtKind::Assign(target, value),
                ..
            },
        ] => match &target.kind {
            ExprKind::Var(x) => Some((x.as_str(), value)),
            _ => None,
        },
        _ => None,
    }
}

/// Of `l: { if (a) { if (b) { x = t; break l; } } x = f; }`: its tests,
/// `[a, b]`, `x`, `t` and `f`.
fn chain(body: &[Stmt], label: &str) -> Option<(Vec<Expr>, String, Expr, Expr)> {
    let [
        Stmt {
            kind: StmtKind::If(test, inner, None),
            ..
        },
        last,
    ] = body
    else {
        return None;
    };
    let (x, no) = assigned_only(std::slice::from_ref(last))?;
    let (mut tests, y, yes) = inner_chain(inner, label)?;
    if x != y {
        return None;
    }
    tests.insert(0, test.clone());
    Some((tests, x.to_string(), yes, no.clone()))
}

/// Of `l: { if (a) { if (b) { s; break l; } } t }`: its tests, `[a, b]`,
/// `s` and `t`, where `l` is named nowhere else.
fn or_chain(body: &[Stmt], label: &str) -> Option<(Vec<Expr>, Vec<Stmt>, Vec<Stmt>)> {
    let [first, then @ ..] = body else { return None };
    let (tests, otherwise) = broken_chain(std::slice::from_ref(first), label)?;
    if tests.is_empty() || targets(then, label) || targets(&otherwise, label) {
        return None;
    }
    Some((tests, otherwise, then.to_vec()))
}

/// Of `if (a) { if (b) { s; break l; } }`: `[a, b]` and `s`.
fn broken_chain(stmts: &[Stmt], label: &str) -> Option<(Vec<Expr>, Vec<Stmt>)> {
    match stmts {
        [
            Stmt {
                kind: StmtKind::If(test, inner, None),
                ..
            },
        ] => {
            let (mut tests, otherwise) = broken_chain(inner, label)?;
            tests.insert(0, test.clone());
            Some((tests, otherwise))
        }
        [
            rest @ ..,
            Stmt {
                kind: StmtKind::Break(Some(l)),
                ..
            },
        ] if l == label => Some((Vec::new(), rest.to_vec())),
        _ => None,
    }
}

/// `!e`, as a person writes it: `!!a` is `a`, `!(a === b)` `a !== b`, and
/// `!(a && b)` of `bool`s `!a || !b`.
fn not(e: Expr) -> Expr {
    match e.kind {
        ExprKind::Binary(op @ (Op::And | Op::Or), a, b) if is_boolean(&a) && is_boolean(&b) => {
            let flipped = if op == Op::And { Op::Or } else { Op::And };
            Expr::bin(flipped, not(*a), not(*b))
        }
        ExprKind::Unary(UnaryOp::Not, inner) => *inner,
        kind => Expr::unary(UnaryOp::Not, Expr { kind, span: e.span }),
    }
}

/// Of `if (b) { x = t; break l; }`, nested: its tests, `x` and `t`.
fn inner_chain(stmts: &[Stmt], label: &str) -> Option<(Vec<Expr>, String, Expr)> {
    match stmts {
        [
            assign,
            Stmt {
                kind: StmtKind::Break(Some(l)),
                ..
            },
        ] if l == label => {
            let (x, yes) = assigned_only(std::slice::from_ref(assign))?;
            Some((Vec::new(), x.to_string(), yes.clone()))
        }
        [
            Stmt {
                kind: StmtKind::If(test, inner, None),
                ..
            },
        ] => {
            let (mut tests, x, yes) = inner_chain(inner, label)?;
            tests.insert(0, test.clone());
            Some((tests, x, yes))
        }
        _ => None,
    }
}

/// `test ? yes : no`, of a `bool` test `test && yes` where `no` is
/// `false`, and `test || no` where `yes` is `true`.
fn conditional(test: Expr, yes: Expr, no: Expr) -> Expr {
    // `!c ? a : b` is `c ? b : a`.
    let (test, yes, no) = match test.kind {
        ExprKind::Unary(UnaryOp::Not, inner)
            if !matches!(yes.kind, ExprKind::Bool(_)) && !matches!(no.kind, ExprKind::Bool(_)) =>
        {
            (*inner, no, yes)
        }
        kind => (Expr { kind, span: test.span }, yes, no),
    };
    if !is_boolean(&test) {
        return Expr::cond(test, yes, no);
    }
    match (&yes.kind, &no.kind) {
        (ExprKind::Bool(true), ExprKind::Bool(false)) => test,
        (ExprKind::Bool(false), ExprKind::Bool(true)) => negate(test),
        (ExprKind::Bool(true), _) => Expr::bin(Op::Or, test, no),
        (ExprKind::Bool(false), _) => Expr::bin(Op::And, negate(test), no),
        (_, ExprKind::Bool(true)) => Expr::bin(Op::Or, negate(test), yes),
        _ => Expr::cond(test, yes, no),
    }
}

/// `a && b`, but `b` where it says `a`: `x === "lit"` that `x != null`, or
/// that `typeof x === "string"`.
fn both(a: Expr, b: Expr) -> Expr {
    // The tests `b` says, of its `&&`'s first.
    let first = match &b.kind {
        ExprKind::Binary(Op::And, first, _) => first,
        _ => &b,
    };
    let says = |test: &Expr| match (&test.kind, &first.kind) {
        (ExprKind::Binary(Op::LooseNe, x, null), ExprKind::Binary(Op::Eq, y, literal))
            if matches!(null.kind, ExprKind::Null | ExprKind::Undefined) =>
        {
            same(x, y) && literal.is_constant() && !matches!(literal.kind, ExprKind::Null | ExprKind::Undefined)
        }
        (ExprKind::Binary(Op::Eq, kind, name), ExprKind::Binary(Op::Eq, y, literal))
            if let ExprKind::Unary(UnaryOp::Typeof, x) = &kind.kind =>
        {
            same(x, y)
                && match (&name.kind, &literal.kind) {
                    (ExprKind::Str(t), ExprKind::Str(_)) => t == "string",
                    (ExprKind::Str(t), ExprKind::Num(_)) => t == "number",
                    (ExprKind::Str(t), ExprKind::Bool(_)) => t == "boolean",
                    _ => false,
                }
        }
        _ => false,
    };
    // `a && b` of `a` an `&&` itself: each of its tests `b` says goes.
    let mut tests: Vec<Expr> = Vec::new();
    let mut flat = |e: Expr| tests.push(e);
    fn spread(e: Expr, push: &mut dyn FnMut(Expr)) {
        match e.kind {
            ExprKind::Binary(Op::And, a, b) => {
                spread(*a, push);
                spread(*b, push);
            }
            _ => push(e),
        }
    }
    spread(a, &mut flat);
    tests.retain(|t| !says(t));
    tests
        .into_iter()
        .rev()
        .fold(b, |rest, test| Expr::bin(Op::And, test, rest))
}

/// Whether `a` and `b` are the same variable, or the same property of one.
fn same(a: &Expr, b: &Expr) -> bool {
    match (&a.kind, &b.kind) {
        (ExprKind::Var(x), ExprKind::Var(y)) => x == y,
        (ExprKind::Member(x, p), ExprKind::Member(y, q)) => p == q && same(x, y),
        _ => false,
    }
}

/// Whether `e` is a `bool` whatever it reads: a comparison, a `!`, or
/// `&&` and `||` of them.
fn is_boolean(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Bool(_) | ExprKind::Unary(UnaryOp::Not, _) => true,
        ExprKind::Binary(
            Op::Eq | Op::Ne | Op::LooseEq | Op::LooseNe | Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::InstanceOf | Op::In,
            ..,
        ) => true,
        ExprKind::Binary(Op::And | Op::Or, a, b) => is_boolean(a) && is_boolean(b),
        _ => false,
    }
}

/// The variables something may change: each `let`'s, and each assigned.
fn changing(body: &mut Vec<Stmt>) -> HashSet<String> {
    let mut changing = assigned(body);
    js::each_block_mut(body, &mut |stmts| {
        for stmt in stmts.iter() {
            if let StmtKind::Let(name, _) = &stmt.kind {
                changing.insert(name.clone());
            }
        }
    });
    changing
}

/// A temporary MIR made, `const _5 = e;`, read once, by the statement
/// after it: `e` where it's read, where nothing that runs before it there
/// does anything, or anywhere outside a closure if `e` does nothing and
/// reads nothing that changes.
fn inlined(stmts: &mut Vec<Stmt>, changing: &HashSet<String>) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        let (head, rest) = stmts.split_at_mut(i + 1);
        let StmtKind::Const(name, value) = &head[i].kind else {
            i += 1;
            continue;
        };
        let temporary = name
            .strip_prefix('_')
            .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit() || c == '$'));
        if !temporary || js::mentions_in(rest, name) != 1 || js::mentions_in(&rest[..1], name) != 1 {
            i += 1;
            continue;
        }
        let mut stable = !value.has_effects();
        value.visit_vars(&mut |v| stable &= !changing.contains(v));
        let read = match &mut rest[0].kind {
            StmtKind::Const(_, e)
            | StmtKind::Let(_, Some(e))
            | StmtKind::Expr(e)
            | StmtKind::Return(Some(e))
            | StmtKind::Throw(e)
            | StmtKind::Destructure { value: e, .. } => Some(e),
            StmtKind::Assign(target, e) if !target.has_effects() => {
                let mut in_target = false;
                target.visit_vars(&mut |v| in_target |= v == name);
                (!in_target).then_some(e)
            }
            StmtKind::If(test, ..) => {
                let mut in_test = false;
                test.visit_vars(&mut |v| in_test |= v == name);
                in_test.then_some(test)
            }
            _ => None,
        };
        let Some(read) = read else {
            i += 1;
            continue;
        };
        // What's read before it: what does nothing, where it does nothing
        // either; else only constants and variables nothing changes, which
        // what it does can't change.
        let acts = value.has_effects();
        let passes = |e: &Expr| match acts {
            false => !e.has_effects(),
            true => e.is_constant() || matches!(&e.kind, ExprKind::Var(v) if !changing.contains(v)),
        };
        if !stable && leads(read, name, &passes) != Some(true) {
            i += 1;
            continue;
        }
        let value = value.clone();
        let Some(replaced) = read.substitute(&|v| (v == name).then(|| value.clone())) else {
            i += 1;
            continue;
        };
        *read = replaced;
        stmts.remove(i);
    }
}

/// Whether `e` reads `name` where all that JS evaluates before it in `e`
/// `passes`: `Some(true)`; `Some(false)` if something doesn't, or it's read
/// where it may not be; `None` if it isn't read.
fn leads(e: &Expr, name: &str, passes: &dyn Fn(&Expr) -> bool) -> Option<bool> {
    let mut reads = false;
    e.visit_vars(&mut |v| reads |= v == name);
    if !reads {
        return None;
    }
    let in_order = |items: &mut dyn Iterator<Item = &Expr>| {
        for item in items {
            match leads(item, name, passes) {
                Some(found) => return Some(found),
                None if !passes(item) => return Some(false),
                None => {}
            }
        }
        Some(false)
    };
    match &e.kind {
        ExprKind::Var(v) => Some(v == name),
        ExprKind::Member(a, _)
        | ExprKind::OptionalMember(a, _)
        | ExprKind::Unary(_, a)
        | ExprKind::Spread(a)
        | ExprKind::Stringed(a)
        | ExprKind::Await(a) => leads(a, name, passes),
        ExprKind::Index(a, b) => in_order(&mut [&**a, &**b].into_iter()),
        // What runs only as a test says is `b`, `c` of `a ? b : c`.
        ExprKind::Binary(Op::And | Op::Or | Op::Coalesce, a, _) | ExprKind::Cond(a, ..) => {
            leads(a, name, passes).or(Some(false))
        }
        ExprKind::Binary(_, a, b) => in_order(&mut [&**a, &**b].into_iter()),
        ExprKind::Array(items) | ExprKind::Template(_, items, _) => in_order(&mut items.iter()),
        ExprKind::Call(f, args) | ExprKind::New(f, args) => in_order(&mut std::iter::once(&**f).chain(args)),
        ExprKind::Object(props) => in_order(&mut props.iter().map(|p| {
            let (js::Prop::Field(_, v) | js::Prop::Getter(_, v) | js::Prop::Spread(v)) = p;
            v
        })),
        ExprKind::Jsx(jsx) => {
            let tag = match &jsx.tag {
                js::JsxTag::Component(tag) => Some(tag),
                _ => None,
            };
            let props = jsx.props.iter().map(|p| {
                let (js::Prop::Field(_, v) | js::Prop::Getter(_, v) | js::Prop::Spread(v)) = p;
                v
            });
            in_order(&mut tag.into_iter().chain(props).chain(&jsx.children))
        }
        _ => Some(false),
    }
}

/// `const a = e; .. const b = a;`, where nothing else reads `a`, is `const
/// b = e;` where `a` was: `b` is read nowhere before, which JS's `const`
/// says of its block.
fn aliases(body: &mut Vec<Stmt>) {
    js::statement_lists(body, &mut |stmts| {
        let mut i = 0;
        while i < stmts.len() {
            let alias = match &stmts[i].kind {
                StmtKind::Const(a, _) => (i + 1..stmts.len())
                    .find(|&j| js::mentions_in(&stmts[j..=j], a) > 0)
                    .filter(|&j| {
                        matches!(&stmts[j].kind, StmtKind::Const(_, value)
                            if matches!(&value.kind, ExprKind::Var(read) if read == a))
                            && js::mentions_in(&stmts[j..], a) == 1
                    }),
                _ => None,
            };
            if let Some(j) = alias {
                let StmtKind::Const(b, _) = stmts.remove(j).kind else {
                    unreachable!("matched")
                };
                if let StmtKind::Const(a, _) = &mut stmts[i].kind {
                    *a = b;
                }
            }
            i += 1;
        }
    });
}

/// `let x; .. x = e;` is `.. let x = e;`, where nothing between names `x`,
/// nor does `e`; and a `let` nothing assigns after, a closure neither, is
/// a `const`.
fn declarations(body: &mut Vec<Stmt>) {
    js::statement_lists(body, &mut |stmts| {
        let mut i = 0;
        while i < stmts.len() {
            let first = match &stmts[i].kind {
                // What it's declared with does nothing, and is never read.
                StmtKind::Let(name, init) if init.as_ref().is_none_or(|e| !e.has_effects()) => (i + 1..stmts.len())
                    .find(|&j| js::mentions_in(&stmts[j..=j], name) > 0)
                    .filter(|&j| {
                        matches!(&stmts[j].kind, StmtKind::Assign(target, _)
                            if matches!(&target.kind, ExprKind::Var(assigned) if assigned == name))
                            && js::mentions_in(&stmts[j..=j], name) == 1
                    }),
                _ => None,
            };
            let Some(j) = first else {
                i += 1;
                continue;
            };
            let StmtKind::Let(name, _) = stmts.remove(i).kind else {
                unreachable!("matched")
            };
            let assign = &mut stmts[j - 1];
            let StmtKind::Assign(_, value) = std::mem::replace(&mut assign.kind, StmtKind::Break(None)) else {
                unreachable!("matched")
            };
            assign.kind = StmtKind::Let(name, Some(value));
        }
    });
    let assigned = assigned(body);
    js::statement_lists(body, &mut |stmts| {
        for stmt in stmts.iter_mut() {
            if let StmtKind::Let(name, Some(value)) = &mut stmt.kind
                && !assigned.contains(name)
            {
                let value = std::mem::replace(value, js::Expr::undefined());
                stmt.kind = StmtKind::Const(std::mem::take(name), value);
            }
        }
    });
}

/// The variables `body` assigns, and each a handle is on, which its
/// setter assigns (ADR 0099).
fn assigned(body: &mut Vec<Stmt>) -> HashSet<String> {
    let mut assigned = HashSet::new();
    js::each_block_mut(body, &mut |stmts| {
        for stmt in stmts.iter() {
            if let StmtKind::Assign(target, _) = &stmt.kind
                && let ExprKind::Var(name) = &target.kind
            {
                assigned.insert(name.clone());
            }
        }
    });
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Handle(place) | ExprKind::Pair(place, _) = &e.kind
            && let ExprKind::Var(name) = &place.kind
        {
            assigned.insert(name.clone());
        }
    });
    assigned
}

/// `result = e; return result;` is `return e;`: nothing reads the return
/// place once it's returned, nor could a closure.
fn returned(stmts: &mut Vec<Stmt>, result: &str) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        let assigns = matches!(&stmts[i].kind, StmtKind::Assign(target, _)
            if matches!(&target.kind, ExprKind::Var(name) if name == result));
        let returns = matches!(&stmts[i + 1].kind, StmtKind::Return(Some(value))
            if matches!(&value.kind, ExprKind::Var(name) if name == result));
        if assigns && returns {
            let StmtKind::Assign(_, value) = stmts.remove(i).kind else {
                unreachable!("matched")
            };
            stmts[i].kind = StmtKind::Return(Some(value));
        }
        i += 1;
    }
}

/// What follows a `return`, a `throw`, a `break` or a `continue` in the
/// same list never runs, nor what follows an `if` each of whose branches
/// ends so.
fn unreachable_after(stmts: &mut Vec<Stmt>) {
    if let Some(at) = stmts.iter().position(ends) {
        stmts.truncate(at + 1);
    }
}

/// Whether running `stmt` never goes on to what follows it.
fn ends(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Return(_) | StmtKind::Throw(_) | StmtKind::Break(_) | StmtKind::Continue(_) => true,
        StmtKind::If(_, then, Some(otherwise)) => then.last().is_some_and(ends) && otherwise.last().is_some_and(ends),
        _ => false,
    }
}

/// A `break l` that ends labeled block `l`, or a `continue l` that ends
/// loop `l`'s body, where it would go on anyway: in the tails of the `if`s
/// that end them too.
fn tail_jumps(stmts: &mut [Stmt]) {
    for stmt in stmts {
        match &mut stmt.kind {
            StmtKind::Labeled(label, body) => drop_tail(body, &StmtKind::Break(Some(label.clone()))),
            StmtKind::While {
                label: Some(label),
                body,
                ..
            } => drop_tail(body, &StmtKind::Continue(Some(label.clone()))),
            _ => {}
        }
    }
}

/// `jump` where it ends `stmts`, or a branch of the `if` that ends them.
fn drop_tail(stmts: &mut Vec<Stmt>, jump: &StmtKind) {
    let same = |kind: &StmtKind| match (kind, jump) {
        (StmtKind::Break(Some(a)), StmtKind::Break(Some(b)))
        | (StmtKind::Continue(Some(a)), StmtKind::Continue(Some(b))) => a == b,
        _ => false,
    };
    match stmts.last_mut().map(|s| &mut s.kind) {
        Some(kind) if same(kind) => {
            stmts.pop();
        }
        Some(StmtKind::If(_, then, otherwise)) => {
            drop_tail(then, jump);
            if let Some(otherwise) = otherwise {
                drop_tail(otherwise, jump);
            }
        }
        _ => {}
    }
}

/// A label nothing breaks to or continues: a block's statements in its
/// place, where they declare nothing its neighbors do; a loop, unlabeled.
fn unlabeled(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        let splice = match &stmts[i].kind {
            StmtKind::Labeled(label, body) if !targets(body, label) => {
                let inner: HashSet<&str> = declared(body).collect();
                !stmts
                    .iter()
                    .enumerate()
                    .any(|(j, s)| j != i && declared(std::slice::from_ref(s)).any(|name| inner.contains(name)))
            }
            _ => false,
        };
        if splice {
            let StmtKind::Labeled(_, body) = stmts.remove(i).kind else {
                unreachable!("matched")
            };
            stmts.splice(i..i, body);
            // What it held is looked at again: a label inside it, now here.
            continue;
        }
        if let StmtKind::While { label, body, .. } = &mut stmts[i].kind
            && label.as_deref().is_some_and(|l| !targets(body, l))
        {
            *label = None;
        }
        i += 1;
    }
}

/// Whether a `break` or a `continue` in `stmts` names `label`.
fn targets(stmts: &[Stmt], label: &str) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Break(Some(l)) | StmtKind::Continue(Some(l)) => l == label,
        StmtKind::If(_, a, b) => targets(a, label) || b.as_deref().is_some_and(|b| targets(b, label)),
        StmtKind::While { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Labeled(_, body) => targets(body, label),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => targets(a, label) || targets(b, label),
        _ => false,
    })
}

/// The names `stmts` declare, not in blocks inside them.
fn declared(stmts: &[Stmt]) -> impl Iterator<Item = &str> {
    stmts.iter().filter_map(|s| match &s.kind {
        StmtKind::Const(name, _) | StmtKind::Let(name, _) => Some(name.as_str()),
        _ => None,
    })
}
