//! Tidy's jumps: `break`s, `continue`s, `return`s and labels, as a person
//! writes a loop's or a block's exits.

use super::*;

/// Whether a `continue` in `stmts` goes on with the loop they're in, labeled
/// `label`: one of no label not inside a loop of their own, or of its label.
pub(super) fn goes_on(stmts: &[Stmt], label: Option<&str>) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Continue(None) => true,
        StmtKind::Continue(Some(l)) => Some(l.as_str()) == label,
        StmtKind::If(_, a, b) => goes_on(a, label) || b.as_deref().is_some_and(|b| goes_on(b, label)),
        StmtKind::Labeled(_, body) => goes_on(body, label),
        StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
            label.is_some_and(|l| names_continue(body, l))
        }
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => goes_on(a, label) || goes_on(b, label),
        _ => false,
    })
}

/// Whether a `continue` in `stmts`, however deep, names `label`.
pub(super) fn names_continue(stmts: &[Stmt], label: &str) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Continue(Some(l)) => l == label,
        StmtKind::If(_, a, b) => names_continue(a, label) || b.as_deref().is_some_and(|b| names_continue(b, label)),
        StmtKind::While { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Labeled(_, body) => names_continue(body, label),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => names_continue(a, label) || names_continue(b, label),
        _ => false,
    })
}

/// A loop's body that ends `if (c) { a; continue l; } break;`, `l` the loop
/// or none: `if (!c) { break; } a`, as a person writes an early exit.
pub(super) fn early_breaks(stmts: &mut [Stmt]) {
    for stmt in stmts.iter_mut() {
        let (label, body) = match &mut stmt.kind {
            StmtKind::While { label, body, .. }
            | StmtKind::ForOf { label, body, .. }
            | StmtKind::For { label, body, .. } => (label.clone(), body),
            _ => continue,
        };
        let [.., last_if, last] = body.as_slice() else { continue };
        let continues = |s: &Stmt| match &s.kind {
            StmtKind::Continue(None) => true,
            StmtKind::Continue(Some(l)) => Some(l) == label.as_ref(),
            _ => false,
        };
        let shape = matches!(&last.kind, StmtKind::Break(None))
            && matches!(&last_if.kind, StmtKind::If(_, then, None) if then.last().is_some_and(continues));
        if !shape {
            continue;
        }
        body.pop();
        let Some(Stmt {
            kind: StmtKind::If(test, mut then, None),
            span,
        }) = body.pop()
        else {
            unreachable!("matched")
        };
        then.pop();
        body.push(StmtKind::If(not(test), vec![StmtKind::Break(None).at(span)], None).at(span));
        body.extend(then);
    }
}

/// A `while (true)` whose body ends `if (c) { a } return x;`, MIR's loop
/// that returns where it ends, `while let Some(p) = it.next()` of a
/// function that gives `None` after it, as a person writes an early exit:
/// - `a` going on, `if (c) { a; continue; }`: `if (!c) { break; } a` and
///   the `return x;` after the loop, as `early_breaks` makes of one that
///   breaks; not where something else leaves the loop, nor of what it
///   declares, which isn't seen after it.
/// - `a` never going on past its own end: `if (!c) { return x; } a` (ADR 0370).
pub(super) fn early_returns(stmts: &mut Vec<Stmt>) {
    let mut at = 0;
    while at < stmts.len() {
        while let Some(after) = early_return(&mut stmts[at]) {
            if let Some(after) = after {
                stmts.insert(at + 1, after);
                break;
            }
        }
        at += 1;
    }
}

/// One of `early_returns`'s steps of `stmt`, if it takes one: the `return`
/// it leaves after the loop, if it does. Its body's end, `if (c) { a }
/// return x;`, or `if (c) { a } else { e }` of a lone exit `e`, `return;` as
/// MIR's `?` is, or `break;`: `if (!c) { e } a`, as `a` goes on to the next
/// time round where the body ends.
pub(super) fn early_return(stmt: &mut Stmt) -> Option<Option<Stmt>> {
    let StmtKind::While { label, cond, body } = &mut stmt.kind else {
        return None;
    };
    if !matches!(cond.kind, ExprKind::Bool(true)) {
        return None;
    }
    let continues = |s: &Stmt| match &s.kind {
        StmtKind::Continue(None) => true,
        StmtKind::Continue(Some(l)) => Some(l) == label.as_ref(),
        _ => false,
    };
    let is_return = |s: &Stmt| matches!(s.kind, StmtKind::Return(_));
    let exits = |s: &Stmt| matches!(s.kind, StmtKind::Return(_) | StmtKind::Break(_) | StmtKind::Continue(_));
    // Where the `if` is, and whether the exit is its `else`.
    let (at, in_else) = match body.as_slice() {
        [
            ..,
            Stmt {
                kind: StmtKind::If(_, _, Some(otherwise)),
                ..
            },
        ] if matches!(otherwise.as_slice(), [e] if exits(e)) => (body.len() - 1, true),
        [
            ..,
            Stmt {
                kind: StmtKind::If(_, _, None),
                ..
            },
            last,
        ] if is_return(last) => (body.len() - 2, false),
        _ => return None,
    };
    let rest = &body[at..];
    let StmtKind::If(_, then, otherwise) = &rest[0].kind else {
        unreachable!("matched")
    };
    let exit = match otherwise {
        Some(otherwise) => &otherwise[0],
        None => &rest[1],
    };
    let mut inside = false;
    if let StmtKind::Return(Some(value)) = &exit.kind {
        value.visit_vars(&mut |v| inside |= declared(body).any(|d| d == v));
    }
    let goes_on = then.last().is_some_and(continues);
    // The `return` after the loop: of no other way out of it, so nothing
    // after it ran, nor of what it declares.
    let after = goes_on && !in_else;
    if after && (breaks(body, label.as_deref()) || inside) {
        return None;
    }
    if !in_else && !goes_on && !then.last().is_some_and(ends) {
        return None;
    }
    let mut tail = body.split_off(at).into_iter();
    let Some(Stmt {
        kind: StmtKind::If(test, mut then, otherwise),
        span,
    }) = tail.next()
    else {
        unreachable!("matched")
    };
    let exit = match otherwise {
        Some(mut otherwise) => otherwise.pop(),
        None => tail.next(),
    }
    .expect("matched");
    if goes_on {
        then.pop();
    }
    if after {
        body.push(StmtKind::If(not(test), vec![StmtKind::Break(None).at(span)], None).at(span));
        body.extend(then);
        return Some(Some(exit));
    }
    body.push(StmtKind::If(not(test), vec![exit], None).at(span));
    body.extend(then);
    Some(None)
}

/// Where a function ends, a `return;` the program didn't write, `written`
/// where those it did are: nothing, as it falls off the end there; and
/// `if (c) { s; return; } rest`, `if (c) { s } else { rest }`, as the
/// program wrote it, `if (!c) { rest }` of no `s`.
pub(super) fn tail_returns(stmts: &mut Vec<Stmt>, returns: &[js::Span]) {
    let unwritten = |s: &Stmt| matches!(s.kind, StmtKind::Return(None)) && !written(returns, s.span);
    while stmts.last().is_some_and(unwritten) {
        stmts.pop();
    }
    if let Some(at) = (stmts.iter())
        .position(|s| matches!(&s.kind, StmtKind::If(_, then, None) if then.last().is_some_and(unwritten)))
    {
        let rest = stmts.split_off(at + 1);
        let Some(Stmt {
            kind: StmtKind::If(test, mut then, None),
            span,
        }) = stmts.pop()
        else {
            unreachable!("found")
        };
        then.pop();
        match (then.is_empty(), rest.is_empty()) {
            (true, true) => {}
            (true, false) => stmts.push(StmtKind::If(not(test), rest, None).at(span)),
            (false, true) => stmts.push(StmtKind::If(test, then, None).at(span)),
            (false, false) => stmts.push(StmtKind::If(test, then, Some(rest)).at(span)),
        }
    }
    // `if (c) { s } else { return; }`, a `let … else`'s `return` the program
    // wrote: `if (!c) { return; } s`, its binding first, `const b = x; if (b
    // == null) { return; }`.
    if let Some(Stmt {
        kind: StmtKind::If(_, then, Some(otherwise)),
        ..
    }) = stmts.last()
        && matches!(otherwise.as_slice(), [r] if matches!(r.kind, StmtKind::Return(None)) && written(returns, r.span))
        && !then.last().is_some_and(ends)
    {
        let Some(Stmt {
            kind: StmtKind::If(test, then, Some(otherwise)),
            span,
        }) = stmts.pop()
        else {
            unreachable!("matched")
        };
        let mut then = then.into_iter().peekable();
        // The binding of what it tests is set, first, tested itself.
        let bound = match (&test.kind, then.peek().map(|s| &s.kind)) {
            (ExprKind::Binary(Op::LooseNe, x, null), Some(StmtKind::Const(b, value)))
                if matches!(null.kind, ExprKind::Null | ExprKind::Undefined) && js::same_path(x, value) =>
            {
                Some(b.clone())
            }
            _ => None,
        };
        let test = match &bound {
            Some(b) => {
                stmts.push(then.next().expect("the binding"));
                Expr::bin(Op::LooseEq, Expr::var(b), Expr::null())
            }
            None => not(test),
        };
        stmts.push(StmtKind::If(test, otherwise, None).at(span));
        stmts.extend(then);
    }
    match stmts.last_mut().map(|s| &mut s.kind) {
        Some(StmtKind::If(_, then, otherwise)) => {
            tail_returns(then, returns);
            if let Some(branch) = otherwise {
                tail_returns(branch, returns);
            }
        }
        Some(StmtKind::Labeled(_, body)) => tail_returns(body, returns),
        _ => {}
    }
}

/// Whether `stmts` hold a `return` the program wrote, of `returns`.
pub(super) fn writes_return(stmts: &[Stmt], returns: &[js::Span]) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Return(_) => written(returns, stmt.span),
        StmtKind::If(_, a, b) => writes_return(a, returns) || b.as_deref().is_some_and(|b| writes_return(b, returns)),
        _ => false,
    })
}

/// Whether a statement at `span` is a `return` the program wrote, of
/// `returns`, where those are.
pub(super) fn written(returns: &[js::Span], span: js::Span) -> bool {
    !span.is_none() && returns.iter().any(|w| w.lo <= span.lo && span.hi <= w.hi)
}

/// `l: { if (a) { break l; } if (b) { break l; } rest } t`, of `let … else`
/// and checks that go on to one end, `t`, a `throw` or a `return`, and
/// `rest` never going on past its own: `if (a || b) { t } rest`, as a
/// person writes a guard; each guard one that tests on too, an `||` of
/// `&&`s the program tests (`break_test`).
pub(super) fn guarded(stmts: &mut Vec<Stmt>) {
    for i in 0..stmts.len() {
        let StmtKind::Labeled(label, body) = &stmts[i].kind else {
            continue;
        };
        let tail = &stmts[i + 1..];
        let guards = (body.iter()).take_while(|s| break_test(s, label).is_some()).count();
        let rest = &body[guards..];
        if !tail.last().is_some_and(ends) {
            continue;
        }
        if guards == 0 || targets(rest, label) || !rest.last().is_some_and(ends) {
            continue;
        }
        // What `rest` declares, now beside what's before it.
        if declared(rest).any(|name| declared(&stmts[..i]).any(|n| n == name)) {
            continue;
        }
        let test = (body[..guards].iter())
            .map(|s| break_test(s, label).expect("a guard"))
            .reduce(|a, b| Expr::bin(Op::Or, a, b))
            .expect("one");
        let span = stmts[i].span;
        let rest = rest.to_vec();
        let tail = stmts.split_off(i + 1);
        stmts.pop();
        stmts.push(StmtKind::If(test, tail, None).at(span));
        stmts.extend(rest);
        return;
    }
}

/// When `stmt` breaks out to `label`, of `if`s that do or test on:
/// `if (k) { if (!m) { if (c) { break l; } } else { break l; } }`'s `k && (m
/// || c)`.
pub(super) fn break_test(stmt: &Stmt, label: &str) -> Option<Expr> {
    let StmtKind::If(test, then, otherwise) = &stmt.kind else {
        return None;
    };
    let breaks = |s: &[Stmt]| matches!(s, [Stmt { kind: StmtKind::Break(Some(l)), .. }] if l == label);
    let inner = |s: &[Stmt]| match s {
        [one] => break_test(one, label),
        _ => None,
    };
    match otherwise.as_deref() {
        None if breaks(then) => Some(test.clone()),
        None => Some(Expr::bin(Op::And, test.clone(), inner(then)?)),
        Some(otherwise) if breaks(otherwise) => Some(Expr::bin(Op::Or, not(test.clone()), inner(then)?)),
        Some(_) => None,
    }
}

/// A loop's body that ends in a block, `l: { … break l; … }`, its `break l`
/// the loop's `continue`: the block's statements in its place. And one that
/// ends `if (!c) { a; b }`, as MIR has a `continue` the program wrote,
/// `if (c) { continue; } a; b`.
pub(super) fn continued(stmts: &mut [Stmt], guards: &[js::Span]) {
    for stmt in stmts.iter_mut() {
        let (looped, body) = match &mut stmt.kind {
            StmtKind::While { label, body, .. }
            | StmtKind::ForOf { label, body, .. }
            | StmtKind::For { label, body, .. } => (label.clone(), body),
            _ => continue,
        };
        loop {
            match body.last().map(|s| &s.kind) {
                Some(StmtKind::Labeled(..)) => {
                    let Some(Stmt {
                        kind: StmtKind::Labeled(block, mut inner),
                        span,
                    }) = body.pop()
                    else {
                        unreachable!("matched")
                    };
                    // Its branches first, `if (c) { s; break l; } t` an `else`,
                    // as `branch_ends` has them: a `continue` the program
                    // wrote, which ends one, comes back (below).
                    ending(&mut inner, &block);
                    let clashes = declared(&inner).any(|name| declared(body).any(|n| n == name));
                    if clashes || continues(&mut inner, &block, looped.as_deref(), false).is_none() {
                        body.push(StmtKind::Labeled(block, inner).at(span));
                        break;
                    }
                    continues(&mut inner, &block, looped.as_deref(), true);
                    body.extend(inner);
                }
                // One the program ended with a `continue`, `if (t) { a;
                // continue; }`, of what MIR has as its `else`.
                Some(StmtKind::If(_, _, Some(otherwise)))
                    if written(guards, body.last().expect("matched").span)
                        && !declared(otherwise).any(|name| declared(&body[..body.len() - 1]).any(|n| n == name)) =>
                {
                    let Some(Stmt {
                        kind: StmtKind::If(test, mut then, Some(otherwise)),
                        span,
                    }) = body.pop()
                    else {
                        unreachable!("matched")
                    };
                    if !then.last().is_some_and(ends) {
                        then.push(StmtKind::Continue(None).at(span));
                    }
                    body.push(StmtKind::If(test, then, None).at(span));
                    body.extend(otherwise);
                }
                // Of one statement, where the program wrote the `continue`,
                // whatever its test, `i % 2 !== 0` of `i % 2 == 0`.
                Some(StmtKind::If(test, then, None))
                    if written(guards, body.last().expect("matched").span) || (negated(test) && then.len() > 1) =>
                {
                    let Some(Stmt {
                        kind: StmtKind::If(test, then, None),
                        span,
                    }) = body.pop()
                    else {
                        unreachable!("matched")
                    };
                    body.push(StmtKind::If(not(test), vec![StmtKind::Continue(None).at(span)], None).at(span));
                    body.extend(then);
                }
                _ => break,
            }
        }
        // A `continue` where the body ends anyway, in the branches that end
        // it: none.
        fn ends_going_on(stmts: &mut Vec<Stmt>, looped: &Option<String>) {
            let more = stmts.len() > 1;
            match stmts.last_mut().map(|s| &mut s.kind) {
                Some(StmtKind::Continue(l)) if (l.is_none() || l == looped) && more => {
                    stmts.pop();
                }
                Some(StmtKind::If(_, then, otherwise)) => {
                    ends_going_on(then, looped);
                    if let Some(otherwise) = otherwise {
                        ends_going_on(otherwise, looped);
                    }
                }
                _ => {}
            }
        }
        ends_going_on(body, &looped);
    }
}

/// A `continue l` or a `break l` of loop `l` where it's the innermost
/// loop: `continue` or `break`, as JS's unlabeled ones go there.
pub(super) fn innermost_jumps(stmts: &mut [Stmt]) {
    fn unlabel(stmts: &mut [Stmt], looped: &str) {
        for stmt in stmts {
            match &mut stmt.kind {
                StmtKind::Continue(label) | StmtKind::Break(label) if label.as_deref() == Some(looped) => *label = None,
                StmtKind::If(_, a, b) => {
                    unlabel(a, looped);
                    if let Some(b) = b {
                        unlabel(b, looped);
                    }
                }
                StmtKind::Labeled(_, body) => unlabel(body, looped),
                StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                    unlabel(a, looped);
                    unlabel(b, looped);
                }
                _ => {}
            }
        }
    }
    for stmt in stmts {
        if let StmtKind::While {
            label: Some(label),
            body,
            ..
        }
        | StmtKind::ForOf {
            label: Some(label),
            body,
            ..
        }
        | StmtKind::For {
            label: Some(label),
            body,
            ..
        } = &mut stmt.kind
        {
            unlabel(body, label);
        }
    }
}

/// Each `break block` in `stmts` a `continue` of the loop around, labeled
/// `looped`, written if `write`: `None` where one inside a loop of its own
/// would need the loop's label, and it has none.
pub(super) fn continues(stmts: &mut [Stmt], block: &str, looped: Option<&str>, write: bool) -> Option<()> {
    fn each(stmts: &mut [Stmt], block: &str, looped: Option<&str>, inner: bool, write: bool) -> Option<()> {
        for stmt in stmts {
            match &mut stmt.kind {
                StmtKind::Break(Some(l)) if l == block => {
                    let label = match inner {
                        true => Some(looped?.to_string()),
                        false => None,
                    };
                    if write {
                        stmt.kind = StmtKind::Continue(label);
                    }
                }
                StmtKind::If(_, a, b) => {
                    each(a, block, looped, inner, write)?;
                    if let Some(b) = b {
                        each(b, block, looped, inner, write)?;
                    }
                }
                StmtKind::Labeled(_, body) => each(body, block, looped, inner, write)?,
                StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                    each(a, block, looped, inner, write)?;
                    each(b, block, looped, inner, write)?;
                }
                StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
                    each(body, block, looped, true, write)?
                }
                _ => {}
            }
        }
        Some(())
    }
    // Not `looped` itself, a label the block's statements can't name twice.
    if looped == Some(block) {
        return None;
    }
    each(stmts, block, looped, false, write)
}

/// Whether `test` says `item`, what `$next` gave, is none: `item == null`,
/// or `!item` of an array's.
pub(super) fn absent(test: &Expr, item: &str) -> bool {
    match &test.kind {
        ExprKind::Binary(Op::LooseEq, a, b) => {
            matches!(&a.kind, ExprKind::Var(v) if v == item) && matches!(b.kind, ExprKind::Null | ExprKind::Undefined)
        }
        ExprKind::Unary(UnaryOp::Not, a) => matches!(&a.kind, ExprKind::Var(v) if v == item),
        _ => false,
    }
}

/// Whether `test` says `item` is there: `item != null`, or `item`.
pub(super) fn present(test: &Expr, item: &str) -> bool {
    match &test.kind {
        ExprKind::Binary(Op::LooseNe, a, b) => {
            matches!(&a.kind, ExprKind::Var(v) if v == item) && matches!(b.kind, ExprKind::Null | ExprKind::Undefined)
        }
        ExprKind::Var(v) => v == item,
        _ => false,
    }
}

/// What a `for` binds of `item`, read by the first statements of `body`:
/// `const x = item;`, or `const a = item[0]; const b = item[1];`, its
/// pattern, whether it's a `let`, and how many statements; `_` where nothing
/// reads it.
pub(super) fn binding(body: &[Stmt], item: &str) -> Option<(js::Pattern, bool, usize)> {
    let reads = |value: &Expr| -> Option<Result<Option<usize>, String>> {
        match &value.kind {
            ExprKind::Var(v) if v == item => Some(Ok(None)),
            ExprKind::Index(of, index)
                if matches!(&of.kind, ExprKind::Var(v) if v == item)
                    && let ExprKind::Num(k) = index.kind
                    && k >= 0.0
                    && k.fract() == 0.0 =>
            {
                Some(Ok(Some(k as usize)))
            }
            ExprKind::Member(of, key) if matches!(&of.kind, ExprKind::Var(v) if v == item) => Some(Err(key.clone())),
            _ => None,
        }
    };
    let mut parts = Vec::new();
    for stmt in body {
        let (bound, value, mutable) = match &stmt.kind {
            StmtKind::Const(bound, value) => (bound, value, false),
            StmtKind::Let(bound, Some(value)) => (bound, value, true),
            // A `mut` binding MIR declares before the loop, `row = item`: the
            // loop's own, its outer `let` then read by nothing.
            StmtKind::Assign(target, value)
                if let ExprKind::Var(bound) = &target.kind
                    && parts.is_empty() =>
            {
                (bound, value, assigns(&body[1..], bound))
            }
            _ => break,
        };
        match reads(value) {
            Some(part) => parts.push((part, bound.clone(), mutable)),
            None => break,
        }
    }
    let mutable = parts.iter().any(|&(_, _, m)| m);
    match parts.as_slice() {
        [] if js::mentions_in(body, item) == 0 => Some((js::Pattern::Name("_".to_string()), false, 0)),
        [(Ok(None), bound, mutable)] => Some((js::Pattern::Name(bound.clone()), *mutable, 1)),
        _ if !parts.is_empty() && parts.iter().all(|(p, ..)| matches!(p, Ok(Some(_)))) => {
            let last = parts.iter().filter_map(|(p, ..)| p.clone().ok().flatten()).max()?;
            let mut items = vec![None; last + 1];
            for (part, bound, _) in &parts {
                items[part.clone().ok().flatten()?] = Some(bound.clone());
            }
            Some((js::Pattern::names_array(items), mutable, parts.len()))
        }
        _ if !parts.is_empty() && parts.iter().all(|(p, ..)| p.is_err()) => {
            let fields = (parts.iter())
                .map(|(p, bound, _)| (p.clone().expect_err("keyed"), js::Pattern::Name(bound.clone()), None))
                .collect();
            Some((js::Pattern::Object(fields, None), mutable, parts.len()))
        }
        _ => None,
    }
}

/// Whether a `break` in `stmts` leaves the loop they're the body of, labeled
/// `label`: one of no label not inside a loop of their own, or of its label.
pub(super) fn breaks(stmts: &[Stmt], label: Option<&str>) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Break(None) => true,
        StmtKind::Break(Some(l)) => Some(l.as_str()) == label,
        StmtKind::If(_, a, b) => breaks(a, label) || b.as_deref().is_some_and(|b| breaks(b, label)),
        StmtKind::Labeled(_, body) => breaks(body, label),
        // In a loop of its own, one that names it.
        StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
            label.is_some_and(|l| names_break(body, l))
        }
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => breaks(a, label) || breaks(b, label),
        _ => false,
    })
}

/// `?` of a `Result`, `if (r.TAG !== "Err") { s } else { return r; }`: its
/// `return` first, `if (r.TAG === "Err") { return r; } s`.
pub(super) fn tried_first(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        if let StmtKind::If(test, _, Some(otherwise)) = &stmts[i].kind
            && let [
                Stmt {
                    kind: StmtKind::Return(Some(returned)),
                    ..
                },
            ] = otherwise.as_slice()
            && let ExprKind::Var(r) = &returned.kind
            && let ExprKind::Binary(Op::Ne, tag, err) = &test.kind
            && matches!(&tag.kind, ExprKind::Member(of, key) if key == "TAG" && matches!(&of.kind, ExprKind::Var(v) if v == r))
            && matches!(&err.kind, ExprKind::Str(s) if s == "Err")
        {
            let span = stmts[i].span;
            let StmtKind::If(test, then, Some(otherwise)) =
                std::mem::replace(&mut stmts[i].kind, StmtKind::Break(None))
            else {
                unreachable!("matched")
            };
            let count = then.len();
            stmts.splice(
                i..=i,
                std::iter::once(StmtKind::If(not(test), otherwise, None).at(span)).chain(then),
            );
            i += count;
        }
        i += 1;
    }
}

/// A loop's body `if (t) { a; continue; } rest`, `a` written after `rest`,
/// each of which always leaves: `if (!t) { rest } a`, in the order the
/// program wrote them; `a`'s last `continue` goes on anyway. Not of a lone
/// exit, `if (!value) { return; }` of a `?`, written where the block it
/// shares is: a guard first, as a person writes it.
pub(super) fn written_order_steps(stmts: &mut [Stmt]) {
    for stmt in stmts {
        let (StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. }) = &mut stmt.kind
        else {
            continue;
        };
        let found = (0..body.len()).find(|&i| {
            let StmtKind::If(_, then, None) = &body[i].kind else {
                return false;
            };
            let rest = &body[i + 1..];
            let lone = match then.as_slice() {
                [
                    Stmt {
                        kind: StmtKind::Return(value),
                        ..
                    },
                ] => value.as_ref().is_none_or(Expr::is_constant),
                [
                    Stmt {
                        kind: StmtKind::Break(_) | StmtKind::Continue(_),
                        ..
                    },
                ] => true,
                _ => false,
            };
            !lone
                && then.last().is_some_and(ends)
                && rest.last().is_some_and(ends)
                && matches!((first_written(rest), first_written(then)), (Some(r), Some(a)) if r < a)
        });
        let Some(i) = found else { continue };
        let rest = body.split_off(i + 1);
        let Some(Stmt {
            kind: StmtKind::If(test, mut then, None),
            span,
        }) = body.pop()
        else {
            unreachable!("found")
        };
        if matches!(then.last().map(|l| &l.kind), Some(StmtKind::Continue(None))) {
            then.pop();
        }
        body.push(StmtKind::If(not(test), rest, None).at(span));
        body.extend(then);
    }
}

/// `while (true) { if (c) { body; continue; } rest }`, a loop MIR tests at
/// its head whose `rest`, what's after it, always leaves elsewhere: `while
/// (c) { body } rest`, where nothing else jumps in it
/// to the loop, as `rest`'s jumps would go to another once it's outside.
pub(super) fn while_heads(stmts: &mut Vec<Stmt>) -> bool {
    let mut headed = false;
    let mut i = 0;
    while i < stmts.len() {
        if let StmtKind::While { label, cond, body } = &stmts[i].kind
            && matches!(cond.kind, ExprKind::Bool(true))
            && let [first, rest @ ..] = body.as_slice()
            && let StmtKind::If(test, then, None) = &first.kind
            && let [then @ .., last] = then.as_slice()
            && matches!(&last.kind, StmtKind::Continue(l) if l.is_none() || l == label)
            && !own_jumps(then, label.as_deref())
            && !own_jumps(rest, label.as_deref())
            && rest.last().is_some_and(ends)
        {
            let span = stmts[i].span;
            let looped = StmtKind::While {
                label: label.clone(),
                cond: test.clone(),
                body: then.to_vec(),
            };
            let rest = rest.to_vec();
            stmts.splice(i..=i, std::iter::once(looped.at(span)).chain(rest));
            headed = true;
        }
        i += 1;
    }
    headed
}

/// Whether `stmts` jump for loop `label`: an unnamed `break` or `continue`
/// of its own, or one that names it, however deep.
pub(super) fn own_jumps(stmts: &[Stmt], label: Option<&str>) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Break(None) | StmtKind::Continue(None) => true,
        StmtKind::Break(Some(l)) | StmtKind::Continue(Some(l)) => Some(l.as_str()) == label,
        StmtKind::If(_, a, b) => own_jumps(a, label) || b.as_deref().is_some_and(|b| own_jumps(b, label)),
        StmtKind::Labeled(_, body) => own_jumps(body, label),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => own_jumps(a, label) || own_jumps(b, label),
        // In a loop of its own, one that names it.
        StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
            label.is_some_and(|l| names_jump(body, l))
        }
        _ => false,
    })
}

/// Whether a `break` or a `continue` in `stmts`, however deep, names `label`.
pub(super) fn names_jump(stmts: &[Stmt], label: &str) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Break(Some(l)) | StmtKind::Continue(Some(l)) => l == label,
        StmtKind::If(_, a, b) => names_jump(a, label) || b.as_deref().is_some_and(|b| names_jump(b, label)),
        StmtKind::While { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Labeled(_, body) => names_jump(body, label),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => names_jump(a, label) || names_jump(b, label),
        _ => false,
    })
}

/// Whether a `break` in `stmts`, however deep, names `label`.
pub(super) fn names_break(stmts: &[Stmt], label: &str) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Break(Some(l)) => l == label,
        StmtKind::If(_, a, b) => names_break(a, label) || b.as_deref().is_some_and(|b| names_break(b, label)),
        StmtKind::While { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Labeled(_, body) => names_break(body, label),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => names_break(a, label) || names_break(b, label),
        _ => false,
    })
}

/// Of each block `l`, the statements it ends after, `tail_else`'s.
pub(super) fn tail_elses(stmts: &mut [Stmt]) {
    fn tails(list: &mut Vec<Stmt>, label: &str) {
        *list = tail_else(std::mem::take(list), label);
        if let Some(Stmt {
            kind: StmtKind::If(_, then, otherwise),
            ..
        }) = list.last_mut()
        {
            tails(then, label);
            if let Some(otherwise) = otherwise {
                tails(otherwise, label);
            }
        }
    }
    for stmt in stmts {
        if let StmtKind::Labeled(label, body) = &mut stmt.kind {
            let label = label.clone();
            tails(body, &label);
        }
    }
}

/// `if (c) { s; break l; } t`, statements `l` ends after: `if (c) { s }
/// else { t }`, an `else if` chain's, `t` so too.
pub(super) fn tail_else(mut stmts: Vec<Stmt>, label: &str) -> Vec<Stmt> {
    let breaks = |list: &[Stmt]| matches!(list.last(), Some(Stmt { kind: StmtKind::Break(Some(l)), .. }) if l == label);
    if stmts.len() < 2 || !matches!(&stmts[0].kind, StmtKind::If(_, then, None) if breaks(then)) {
        return stmts;
    }
    let rest = stmts.split_off(1);
    let first = stmts.remove(0);
    let StmtKind::If(test, mut then, None) = first.kind else {
        unreachable!("matched")
    };
    then.pop();
    vec![StmtKind::If(test, then, Some(tail_else(rest, label))).at(first.span)]
}

/// `result = e; return result;` is `return e;`: nothing reads the return
/// place once it's returned, nor could a closure.
pub(super) fn returned(stmts: &mut Vec<Stmt>, result: &str) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        let assigns = matches!(&stmts[i].kind, StmtKind::Assign(target, _)
            if matches!(&target.kind, ExprKind::Var(name) if name == result));
        let returns = matches!((&stmts[i].kind, &stmts[i + 1].kind), (StmtKind::Assign(target, _), StmtKind::Return(Some(value)))
            if matches!((&target.kind, &value.kind), (ExprKind::Var(a), ExprKind::Var(b)) if a == b));
        if assigns && returns {
            let StmtKind::Assign(_, value) = stmts.remove(i).kind else {
                unreachable!("matched")
            };
            stmts[i].kind = StmtKind::Return(Some(value));
        }
        i += 1;
    }
}

/// `l: { .. result = a; break l; .. result = b; } return result;`, a
/// function's returns MIR jumps out of a block for, a loop's too: each a
/// `return` where it's written, `return a;`, and the block's statements.
pub(super) fn returned_from_block(stmts: &mut Vec<Stmt>, result: &str) {
    let [.., block, last] = stmts.as_slice() else {
        return;
    };
    let StmtKind::Labeled(label, body) = &block.kind else {
        return;
    };
    if !matches!(&last.kind, StmtKind::Return(Some(v)) if matches!(&v.kind, ExprKind::Var(r) if r == result)) {
        return;
    }
    let label = label.clone();
    let mut body = body.clone();
    js::statement_lists(&mut body, &mut |list| {
        let mut i = 0;
        while i + 1 < list.len() {
            if let StmtKind::Assign(target, value) = &list[i].kind
                && matches!(&target.kind, ExprKind::Var(r) if r == result)
                && matches!(&list[i + 1].kind, StmtKind::Break(Some(l)) if *l == label)
            {
                let value = value.clone();
                let span = list[i].span;
                list.splice(i..=i + 1, [StmtKind::Return(Some(value)).at(span)]);
            }
            i += 1;
        }
    });
    if !made_at_ends(&body, result) {
        return;
    }
    returned_at_ends(&mut body);
    let mut jumps = false;
    js::statement_lists(&mut body, &mut |list| jumps |= targets(list, &label));
    if jumps || js::mentions_in(&body, result) > 0 || js::mentions_in(&stmts[..stmts.len() - 2], result) > 0 {
        return;
    }
    stmts.truncate(stmts.len() - 2);
    stmts.extend(body);
    // `if (c) { s; return a; } else { return b; }` of a `return b` written
    // before `s`, a `let .. else`'s: `if (!c) { return b; } s; return a;`,
    // as the program wrote it.
    js::statement_lists(stmts, &mut |list| {
        for stmt in list.iter_mut() {
            if let StmtKind::If(_, then, Some(otherwise)) = &stmt.kind
                && let [
                    Stmt {
                        kind: StmtKind::Return(_),
                        span,
                    },
                ] = otherwise.as_slice()
                && then.last().is_some_and(ends)
                && !span.is_none()
                && first_written(then).is_some_and(|first| span.lo < first)
            {
                let StmtKind::If(test, then, Some(otherwise)) =
                    std::mem::replace(&mut stmt.kind, StmtKind::Break(None))
                else {
                    unreachable!("matched")
                };
                stmt.kind = StmtKind::If(not(test), otherwise, Some(then));
            }
        }
    });
}

/// Where the first of `stmts` that says is written, inside a loop's too.
pub(super) fn first_written(stmts: &[Stmt]) -> Option<u32> {
    stmts.iter().find_map(|stmt| match &stmt.kind {
        _ if !stmt.span.is_none() => Some(stmt.span.lo),
        StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::While { body, .. }
        | StmtKind::Labeled(_, body) => first_written(body),
        StmtKind::If(_, then, _) => first_written(then),
        _ => None,
    })
}

/// Whether `stmts` end assigning `result`, or leaving, each branch of an
/// `if` that ends them too.
pub(super) fn made_at_ends(stmts: &[Stmt], result: &str) -> bool {
    match stmts.last().map(|s| &s.kind) {
        Some(StmtKind::Assign(target, _)) => matches!(&target.kind, ExprKind::Var(r) if r == result),
        Some(StmtKind::Return(_) | StmtKind::Throw(_)) => true,
        // Or that never goes on: a loop it never leaves.
        Some(StmtKind::While { cond, body, label }) => {
            matches!(cond.kind, ExprKind::Bool(true)) && !breaks(body, label.as_deref())
        }
        Some(StmtKind::If(_, then, Some(otherwise))) => made_at_ends(then, result) && made_at_ends(otherwise, result),
        _ => false,
    }
}

/// `stmts`, of `made_at_ends`, returning what they assign the result.
pub(super) fn returned_at_ends(stmts: &mut [Stmt]) {
    let Some(last) = stmts.last_mut() else { return };
    match &mut last.kind {
        StmtKind::Assign(_, value) => {
            let value = std::mem::replace(value, Expr::undefined());
            last.kind = StmtKind::Return(Some(value));
        }
        StmtKind::If(_, then, Some(otherwise)) => {
            returned_at_ends(then);
            returned_at_ends(otherwise);
        }
        _ => {}
    }
}

/// What follows a `return`, a `throw`, a `break` or a `continue` in the
/// same list never runs, nor what follows an `if` each of whose branches
/// ends so, nor a `while (true)` no `break` leaves.
pub(super) fn unreachable_after(stmts: &mut Vec<Stmt>) {
    if let Some(at) = stmts.iter().position(ends) {
        stmts.truncate(at + 1);
    }
}

/// Whether running `stmt` never goes on to what follows it.
pub(super) fn ends(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Return(_) | StmtKind::Throw(_) | StmtKind::Break(_) | StmtKind::Continue(_) => true,
        StmtKind::If(_, then, Some(otherwise)) => then.last().is_some_and(ends) && otherwise.last().is_some_and(ends),
        StmtKind::While { label, cond, body } => {
            matches!(cond.kind, ExprKind::Bool(true)) && !breaks(body, label.as_deref())
        }
        _ => false,
    }
}

/// A `break l` that ends labeled block `l`, or a `continue l` that ends
/// loop `l`'s body, where it would go on anyway: in the tails of the `if`s
/// that end them too.
pub(super) fn tail_jumps(stmts: &mut [Stmt]) {
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

/// A labeled block's `if (c) { s; break l; } t`, `s` leaving for where
/// `t` goes on to, its end: `if (c) { s } else { t }`, a `match`'s arms
/// as a person writes them, the `break`s then ending the branches; and a
/// block that ends it, one with it. Of `if (!a) { if (!b) { s; break l; }
/// } t`, an or-pattern's, `if (a || b) { t } else { s }`.
pub(super) fn branch_ends(stmts: &mut [Stmt]) {
    for stmt in stmts {
        if let StmtKind::Labeled(label, body) = &mut stmt.kind {
            ending(body, label);
        }
    }
}

/// `stmts`, which end where block `label` does, as `branch_ends` has them.
pub(super) fn ending(stmts: &mut Vec<Stmt>, label: &str) {
    if let Some(Stmt {
        kind: StmtKind::Labeled(inner, body),
        ..
    }) = stmts.last_mut()
    {
        let (inner, mut body) = (inner.clone(), std::mem::take(body));
        js::statement_lists(&mut body, &mut |stmts| {
            for stmt in stmts {
                if matches!(&stmt.kind, StmtKind::Break(Some(l)) if *l == inner) {
                    stmt.kind = StmtKind::Break(Some(label.to_string()));
                }
            }
        });
        stmts.pop();
        stmts.extend(body);
    }
    // Not a `let … else`'s, `if (c) { s; break l; } throw e;`: a guard.
    let exits = matches!(
        stmts.last().map(|s| &s.kind),
        Some(StmtKind::Return(_) | StmtKind::Throw(_))
    );
    let Some(at) = (stmts.iter())
        .position(|s| matches!(&s.kind, StmtKind::If(_, then, None) if leaves(&ands(then).1, label)))
        .filter(|_| !exits)
    else {
        if let Some(StmtKind::If(_, then, otherwise)) = stmts.last_mut().map(|s| &mut s.kind) {
            ending(then, label);
            if let Some(otherwise) = otherwise {
                ending(otherwise, label);
            }
        }
        return;
    };
    let rest = stmts.split_off(at + 1);
    let Some(Stmt {
        kind: StmtKind::If(test, then, None),
        span,
    }) = stmts.pop()
    else {
        unreachable!("found")
    };
    let (tests, mut then) = ands(&then);
    let mut rest = rest;
    ending(&mut then, label);
    ending(&mut rest, label);
    let tests: Vec<Expr> = std::iter::once(test).chain(tests).collect();
    let negated = tests.len() > 1
        && tests
            .iter()
            .all(|t| matches!(t.kind, ExprKind::Unary(UnaryOp::Not, _) | ExprKind::Binary(Op::Ne, ..)));
    let test = (tests.into_iter())
        .rev()
        .reduce(|b, a| Expr::bin(Op::And, a, b))
        .expect("one");
    stmts.push(match (rest.is_empty(), negated) {
        (true, _) => StmtKind::If(test, then, None).at(span),
        (false, true) => StmtKind::If(not(test), rest, Some(then)).at(span),
        (false, false) => StmtKind::If(test, then, Some(rest)).at(span),
    });
}

/// Whether `stmts` always end with a `break label`.
pub(super) fn leaves(stmts: &[Stmt], label: &str) -> bool {
    match stmts.last().map(|s| &s.kind) {
        Some(StmtKind::Break(Some(l))) => l == label,
        Some(StmtKind::If(_, then, Some(otherwise))) => leaves(then, label) && leaves(otherwise, label),
        _ => false,
    }
}

/// `jump` where it ends `stmts`, or a branch of the `if` that ends them.
pub(super) fn drop_tail(stmts: &mut Vec<Stmt>, jump: &StmtKind) {
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

/// Each `break from` and `continue from` in `stmts`, however deep, `to`'s.
pub(super) fn relabeled(stmts: &mut Vec<Stmt>, from: &str, to: &str) {
    js::statement_lists(stmts, &mut |list| {
        for stmt in list.iter_mut() {
            if let StmtKind::Break(Some(l)) | StmtKind::Continue(Some(l)) = &mut stmt.kind
                && l == from
            {
                *l = to.to_string();
            }
        }
    });
}

/// A function that ends in a loop it leaves by one `return` MIR moved into
/// it, the value after the loop, `while (true) { if (c) { return out; } … }`:
/// the loop's `break`, the `return` after it, as a `break` is written; not
/// a `return` the program wrote, in a loop it wrote one in.
pub(super) fn loop_exits(
    body: &mut Vec<Stmt>,
    returns: &[js::Span],
    loops: &[(js::Span, bool, Option<String>)],
    valued: &[js::Span],
) {
    fn take(stmts: &mut [Stmt]) -> Option<Stmt> {
        for stmt in stmts.iter_mut() {
            let taken = match &mut stmt.kind {
                StmtKind::Return(Some(_)) => {
                    let jump = StmtKind::Break(None).at(stmt.span);
                    Some(std::mem::replace(stmt, jump))
                }
                StmtKind::If(_, then, otherwise) => take(then).or_else(|| otherwise.as_deref_mut().and_then(take)),
                StmtKind::Labeled(_, inner) => take(inner),
                _ => None,
            };
            if taken.is_some() {
                return taken;
            }
        }
        None
    }
    let Some(Stmt { kind, span }) = body.last() else { return };
    // Nor of a loop the program wrote a `return` in, which MIR may not span,
    // nor a `loop` whose `break a` gives the function's value, which is
    // returned where it breaks.
    if let Some((written, ..)) = written_loop(loops, *span)
        && (returns.iter().any(|r| written.lo <= r.lo && r.hi <= written.hi) || valued.contains(written))
    {
        return;
    }
    let (StmtKind::While { body: inner, .. } | StmtKind::ForOf { body: inner, .. } | StmtKind::For { body: inner, .. }) =
        kind
    else {
        return;
    };
    // Its one `return`, anywhere in it.
    let mut all = 0;
    js::statement_lists(&mut inner.clone(), &mut |list| {
        all += list.iter().filter(|s| matches!(s.kind, StmtKind::Return(_))).count();
    });
    let mut inner = inner.clone();
    let Some(ret) = take(&mut inner).filter(|_| all == 1) else {
        return;
    };
    // Of what's there after it: nothing the loop declares.
    let mut own: Vec<String> = Vec::new();
    js::statement_lists(&mut inner.clone(), &mut |list| {
        own.extend(declared(list).map(str::to_string))
    });
    match &body.last().expect("a loop").kind {
        StmtKind::ForOf { pattern, .. } => own.extend(pattern.names().into_iter().map(str::to_string)),
        StmtKind::For { name, .. } => own.push(name.clone()),
        _ => {}
    }
    if own
        .iter()
        .any(|name| js::mentions_in(std::slice::from_ref(&ret), name) > 0)
    {
        return;
    }
    if let Some(Stmt {
        kind: StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. },
        ..
    }) = body.last_mut()
    {
        *body = inner;
    }
    body.push(ret);
}

/// The loop the program wrote that starts where `span` does, innermost.
pub(super) fn written_loop(
    loops: &[(js::Span, bool, Option<String>)],
    span: js::Span,
) -> Option<&(js::Span, bool, Option<String>)> {
    (loops.iter())
        .filter(|(l, ..)| !span.is_none() && l.lo <= span.lo && span.hi <= l.hi)
        .min_by_key(|(l, ..)| l.hi - l.lo)
}

/// A loop MIR labels by its block, `bb2: for (…)`, the program labeled:
/// its label, `rows: for (…)`, where no loop around it has that one.
pub(super) fn named_labels(stmts: &mut [Stmt], loops: &[(js::Span, bool, Option<String>)]) {
    fn walk(stmts: &mut [Stmt], loops: &[(js::Span, bool, Option<String>)], around: &mut Vec<String>) {
        for stmt in stmts.iter_mut() {
            let span = stmt.span;
            let (label, body) = match &mut stmt.kind {
                StmtKind::While { label, body, .. }
                | StmtKind::ForOf { label, body, .. }
                | StmtKind::For { label, body, .. } => (label, body),
                _ => continue,
            };
            if let Some(from) = label.clone()
                && from.starts_with("bb")
                && let Some((_, _, Some(to))) = written_loop(loops, span)
                && !around.contains(to)
            {
                relabeled(body, &from, to);
                *label = Some(to.clone());
            }
            around.push(label.clone().unwrap_or_default());
            walk(body, loops, around);
            around.pop();
        }
    }
    walk(stmts, loops, &mut Vec::new());
}

/// A label nothing breaks to or continues: a block's statements in its
/// place, where they declare nothing its neighbors do; a loop, unlabeled.
pub(super) fn unlabeled(stmts: &mut Vec<Stmt>) {
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
        if let StmtKind::While { label, body, .. }
        | StmtKind::ForOf { label, body, .. }
        | StmtKind::For { label, body, .. } = &mut stmts[i].kind
            && label.as_deref().is_some_and(|l| !targets(body, l))
        {
            *label = None;
        }
        i += 1;
    }
}

/// Whether a `break` or a `continue` in `stmts` names `label`.
pub(super) fn targets(stmts: &[Stmt], label: &str) -> bool {
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

/// Of `stmts`, run while `name` is still `""`: `f += s; return f` is
/// `return s`, and `return f` is `return ""`, into each branch that
/// starts so.
pub(super) fn returned_unwritten(stmts: &mut Vec<Stmt>, name: &str) {
    let is_name = |e: &Expr| matches!(&e.kind, ExprKind::Var(v) if v == name);
    let mut i = 0;
    while i < stmts.len() {
        let returned = matches!(stmts.get(i + 1).map(|s| &s.kind), Some(StmtKind::Return(Some(r))) if is_name(r));
        let mentioned = js::mentions_in(std::slice::from_ref(&stmts[i]), name) > 0;
        let next = stmts.get(i + 1).cloned();
        match &mut stmts[i].kind {
            StmtKind::Assign(target, value)
                if is_name(target)
                    && let ExprKind::Binary(Op::Add, lhs, written) = &value.kind
                    && is_name(lhs)
                    && returned =>
            {
                let written = (**written).clone();
                let span = stmts[i].span;
                stmts.splice(i..i + 2, [StmtKind::Return(Some(written)).at(span)]);
                return;
            }
            StmtKind::Return(Some(r)) if is_name(r) => {
                *r = Expr::str("");
                return;
            }
            // An `if` and `else` the `return f` after ends: each its own.
            StmtKind::If(_, then, Some(els)) if returned && let Some(back) = next => {
                then.push(back.clone());
                els.push(back);
                stmts.remove(i + 1);
                continue;
            }
            StmtKind::If(_, then, els) => {
                returned_unwritten(then, name);
                if let Some(els) = els {
                    returned_unwritten(els, name);
                }
                if js::mentions_in(std::slice::from_ref(&stmts[i]), name) > 0 {
                    return;
                }
            }
            _ if mentioned => return,
            _ => {}
        }
        i += 1;
    }
}

/// `if (false) { .. } else { rest }`: `rest`, what's always run.
pub(super) fn never_taken(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        if let StmtKind::If(test, _, els) = &mut stmts[i].kind
            && matches!(test.kind, ExprKind::Bool(false))
        {
            let rest = els.take().unwrap_or_default();
            let count = rest.len();
            stmts.splice(i..=i, rest);
            i += count;
            continue;
        }
        i += 1;
    }
}
