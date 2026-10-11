//! A body lowered from its MIR, as a person writes it (ADR 0364): each
//! rewrite changes nothing that runs. MIR's one return block is a `return`
//! where each path ends, and what the structure needed to reach it, a
//! labeled block or a `break`, goes. A closure's body is tidied as its own.

use std::collections::{HashMap, HashSet};

use crate::js::{self, Expr, ExprKind, Op, Stmt, StmtKind, UnaryOp};

use super::negate;
use crate::lower::iterators::{eager_chains, iterated};

/// What MIR's lowering, and THIR, say of a body for tidying it.
pub(super) struct Hints<'h> {
    /// The name of the function's return place.
    pub(super) result: &'h str,
    /// The temporaries a function that skips what's falsy is given.
    pub(super) falsy: &'h HashSet<String>,
    /// And the temporaries such a `test && value` tests.
    pub(super) tested: &'h HashSet<String>,
    /// Where the program writes each `if`'s condition, and each `return`.
    pub(super) conditions: &'h [js::Span],
    pub(super) returns: &'h [js::Span],
    /// The `const`s MIR's lowering made of what an element it made ahead
    /// of a branch reads (`capture_jsx`): as its temporaries, read where
    /// they're used where nothing between tells.
    pub(super) captured: &'h HashSet<String>,
    /// The variables of Rust's `bool`: JS's `true` and `false` alone.
    pub(super) booleans: &'h HashSet<String>,
}

/// `body`, tidied, by what `hints` say of it.
pub(super) fn tidy(body: &mut Vec<Stmt>, hints: &Hints) {
    let Hints {
        result,
        falsy,
        tested,
        conditions,
        returns,
        captured,
        booleans,
    } = *hints;
    // Twice: what a block's statements, in its place, are followed by, and
    // a loop's label nothing names then.
    for _ in 0..2 {
        js::statement_lists(body, &mut |stmts| {
            returned(stmts, result);
            unreachable_after(stmts);
        });
        js::statement_lists(body, &mut |stmts| branch_ends(stmts));
        js::statement_lists(body, &mut |stmts| tail_jumps(stmts));
        bottom_up(body, &mut |stmts| branch_values(stmts, returns, booleans));
        js::statement_lists(body, &mut unlabeled);
    }
    bottom_up(body, &mut |stmts| branch_values(stmts, returns, booleans));
    let mut read = HashSet::new();
    js::visit_stmts(body, &mut |name| {
        read.insert(name.to_string());
    });
    if !read.contains(result) {
        body.retain(|s| !matches!(&s.kind, StmtKind::Let(name, None) if name == result));
    }
    js::statement_lists(body, &mut guarded);
    declarations(body);
    aliases(body);
    // Of `const`s now, the iterator's too; and what a loop made so ends with,
    // a `continue` and its label, goes.
    // An iterator a loop inside another makes, a `let` assigned each time,
    // read by nothing else.
    let mut mentions: HashMap<String, usize> = HashMap::new();
    js::visit_stmts(body, &mut |name| *mentions.entry(name.to_string()).or_default() += 1);
    // The innermost first: what a loop ends with may be another, recovered.
    bottom_up(body, &mut |stmts| loops(stmts, &mentions));
    js::statement_lists(body, &mut |stmts| early_breaks(stmts));
    js::statement_lists(body, &mut |stmts| tail_jumps(stmts));
    js::statement_lists(body, &mut unlabeled);
    // A `let` declared first, its one write now beside it, what a loop
    // held: a `const` there.
    declarations(body);
    js::statement_lists(body, &mut |stmts| continued(stmts));
    js::statement_lists(body, &mut |stmts| tail_jumps(stmts));
    js::statement_lists(body, &mut |stmts| innermost_jumps(stmts));
    bottom_up(body, &mut |stmts| and_tests(stmts, conditions));
    // A chain of tests a loop ended, its body now ending where the chain's does.
    bottom_up(body, &mut |stmts| branch_values(stmts, returns, booleans));
    js::statement_lists(body, &mut unlabeled);
    // What's given where `false` is skipped as `undefined` is, `test &&
    // value`, as THIR gives it there: one read there alone, which nothing
    // else tells from `undefined`.
    let falsy: HashSet<&String> = falsy.iter().filter(|name| js::mentions_in(body, name) == 1).collect();
    // A test of one read there alone is read for its truth: `!!a || b`'s,
    // `a || b`.
    let tested: HashSet<&String> = tested.iter().filter(|name| js::mentions_in(body, name) == 1).collect();
    js::statement_lists(body, &mut |stmts| {
        for stmt in stmts.iter_mut() {
            if let StmtKind::Const(name, value) = &mut stmt.kind
                && falsy.contains(name)
                && let ExprKind::Cond(test, then, none) = &value.kind
                && matches!(none.kind, ExprKind::Undefined)
            {
                *value = Expr::bin(Op::And, test.tested(), (**then).clone());
            }
            if let StmtKind::Const(name, value) = &mut stmt.kind
                && tested.contains(name)
            {
                *value = value.tested();
            }
        }
    });
    let changing = changing(body);
    js::statement_lists(body, &mut |stmts| filtered_guards(stmts, &changing));
    alias_variables(body, &changing);
    js::statement_lists(body, &mut destructured);
    // What an array made for its parts left, `const _5 = 6;`, too.
    alias_variables(body, &changing);
    js::statement_lists(body, &mut |stmts| inlined(stmts, &changing, captured));
    named_temporaries(body);
    eager_chains(body);
    tail_returns(body, returns);
    // A `let` declared first that nothing's left to assign or read.
    let mut mentions: HashMap<String, usize> = HashMap::new();
    js::visit_stmts(body, &mut |name| *mentions.entry(name.to_string()).or_default() += 1);
    body.retain(|stmt| !matches!(&stmt.kind, StmtKind::Let(name, None) if !mentions.contains_key(name)));
}

/// A `for` loop as MIR has it, `const iter = $iter(xs); while (true) {
/// const t = $next(iter); if (t == null) { exit } const x = t; body }`, is
/// `for (const x of xs) { body } exit`; of a range, `$rangeNext` of
/// `{ start, end }`, `for (let i = start; i < end; i++)`: where nothing else
/// reads the iterator, and only its end leaves the loop. MIR's other shape,
/// `if (t != null) { const x = t; body } exit`, is `for (const x of xs) {
/// body; break; } exit`, its body going on to the exit where it ends.
fn loops(stmts: &mut Vec<Stmt>, mentions: &HashMap<String, usize>) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        if let Some(made) = for_of(&stmts[i], &stmts[i + 1], &stmts[i + 2..], mentions) {
            stmts.splice(i..i + 2, made);
        }
        i += 1;
    }
}

fn for_of(made: &Stmt, looped: &Stmt, after: &[Stmt], mentions: &HashMap<String, usize>) -> Option<Vec<Stmt>> {
    let (iter, source) = match &made.kind {
        StmtKind::Const(iter, source) => (iter, source),
        StmtKind::Assign(target, source)
            if let ExprKind::Var(iter) = &target.kind
                && mentions.get(iter) == Some(&2) =>
        {
            (iter, source)
        }
        _ => return None,
    };
    let StmtKind::While { label, cond, body } = &looped.kind else {
        return None;
    };
    if !matches!(cond.kind, ExprKind::Bool(true)) {
        return None;
    }
    let [first, second, rest @ ..] = body.as_slice() else {
        return None;
    };
    let StmtKind::Const(item, next) = &first.kind else {
        return None;
    };
    let ExprKind::Call(callee, args) = &next.kind else {
        return None;
    };
    let step = match (&callee.kind, args.as_slice()) {
        (
            ExprKind::Var(f),
            [
                Expr {
                    kind: ExprKind::Var(it),
                    ..
                },
            ],
        ) if it == iter => f.as_str(),
        _ => return None,
    };
    if !matches!(step, "$next" | "$rangeNext")
        || js::mentions_in(std::slice::from_ref(looped), iter) != 1
        || js::mentions_in(after, iter) != 0
    {
        return None;
    }
    // When it's done, the exit; else, the body, which in the other shape goes
    // on to the exit, out of the loop.
    let (exit, mut inner, falls_out) = match &second.kind {
        StmtKind::If(test, then, None) if absent(test, item) => (then.clone(), rest.to_vec(), false),
        StmtKind::If(test, then, Some(other)) if absent(test, item) && rest.is_empty() => {
            (then.clone(), other.clone(), false)
        }
        StmtKind::If(test, then, None) if present(test, item) => (rest.to_vec(), then.clone(), true),
        _ => return None,
    };
    if !exit.last().is_some_and(ends) || breaks(&inner, label.as_deref()) {
        return None;
    }
    if falls_out {
        inner.push(StmtKind::Break(None).at(js::Span::NONE));
    }
    let (pattern, mutable, taken) = binding(&inner, item)?;
    let body = inner[taken..].to_vec();
    if js::mentions_in(&body, item) > 0 || js::mentions_in(&exit, item) > 0 {
        return None;
    }
    let looped = match step {
        "$next" => StmtKind::ForOf {
            label: label.clone(),
            pattern,
            mutable,
            iterable: iterated(source.clone()),
            body,
        },
        // A range's end, read once by Rust, is read each step by JS's `for`:
        // only one that reads the same then, a constant or a variable the
        // body doesn't assign.
        _ => {
            let ExprKind::Object(props) = &source.kind else {
                return None;
            };
            let [js::Prop::Field(start_key, start), js::Prop::Field(end_key, end)] = props.as_slice() else {
                return None;
            };
            let js::Pattern::Name(name) = pattern else { return None };
            // Or `v.length` of what the body only reads, by index or its length.
            let same = end.is_constant()
                || matches!(&end.kind, ExprKind::Var(v) if !assigns(&body, v))
                || matches!(&end.kind, ExprKind::Member(of, length)
                    if length == "length" && matches!(&of.kind, ExprKind::Var(v) if only_read(&body, v)));
            if start_key != "start" || end_key != "end" || !same || mutable {
                return None;
            }
            StmtKind::For {
                label: label.clone(),
                test: Expr::bin(Op::Lt, Expr::var(&name), end.clone()),
                name,
                start: start.clone(),
                body,
            }
        }
    };
    let mut out = vec![looped.at(js::Span::NONE)];
    out.extend(exit);
    Some(out)
}

/// A loop's body that ends `if (c) { a; continue l; } break;`, `l` the loop
/// or none: `if (!c) { break; } a`, as a person writes an early exit.
fn early_breaks(stmts: &mut [Stmt]) {
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

/// `if (a) { if (b) { s } }`, the tests of one condition the program wrote,
/// `a && b`'s: `if (a && b) { s }`. What's bound between them, a read that
/// does nothing, comes first in `s`, and `b` reads its value, as
/// `if let`s chained with `&&` read.
fn and_tests(stmts: &mut [Stmt], conditions: &[js::Span]) {
    let within = |c: &js::Span, s: js::Span| !s.is_none() && c.lo <= s.lo && s.hi <= c.hi;
    for stmt in stmts.iter_mut() {
        let span = stmt.span;
        let StmtKind::If(test, then, None) = &mut stmt.kind else {
            continue;
        };
        let [bound @ .., inner] = then.as_slice() else { continue };
        let StmtKind::If(second, _, None) = &inner.kind else {
            continue;
        };
        let one = conditions.iter().any(|c| within(c, span) && within(c, inner.span));
        let pure = bound
            .iter()
            .all(|s| matches!(&s.kind, StmtKind::Const(_, value) if value.reads_same()));
        if !one || !pure {
            continue;
        }
        let second = bound
            .iter()
            .rev()
            .try_fold(second.clone(), |second, stmt| match &stmt.kind {
                StmtKind::Const(name, value) => second.substitute(&|v| (v == name).then(|| value.clone())),
                _ => None,
            });
        let Some(second) = second else { continue };
        let Some(Stmt {
            kind: StmtKind::If(_, body, None),
            ..
        }) = then.pop()
        else {
            unreachable!("matched")
        };
        let first = std::mem::replace(test, Expr::undefined());
        *test = both(first, second);
        then.extend(body);
    }
}

/// Where a function ends, a `return;` the program didn't write, `written`
/// where those it did are: nothing, as it falls off the end there; and
/// `if (c) { s; return; } rest`, `if (c) { s } else { rest }`, as the
/// program wrote it, `if (!c) { rest }` of no `s`.
fn tail_returns(stmts: &mut Vec<Stmt>, returns: &[js::Span]) {
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
    match stmts.last_mut().map(|s| &mut s.kind) {
        Some(StmtKind::If(test, then, otherwise)) => {
            tail_returns(then, returns);
            if let Some(branch) = otherwise {
                tail_returns(branch, returns);
            }
            // A branch it emptied: the other one's, of its test negated.
            if then.is_empty()
                && let Some(branch) = otherwise.take()
            {
                *test = not(std::mem::replace(test, Expr::undefined()));
                *then = branch;
            }
        }
        Some(StmtKind::Labeled(_, body)) => tail_returns(body, returns),
        _ => {}
    }
}

/// Whether `stmts` hold a `return` the program wrote, of `returns`.
fn writes_return(stmts: &[Stmt], returns: &[js::Span]) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Return(_) => written(returns, stmt.span),
        StmtKind::If(_, a, b) => writes_return(a, returns) || b.as_deref().is_some_and(|b| writes_return(b, returns)),
        _ => false,
    })
}

/// Whether a statement at `span` is a `return` the program wrote, of
/// `returns`, where those are.
fn written(returns: &[js::Span], span: js::Span) -> bool {
    !span.is_none() && returns.iter().any(|w| w.lo <= span.lo && span.hi <= w.hi)
}

/// `l: { if (a) { break l; } if (b) { break l; } rest } t`, of `let … else`
/// and checks that go on to one end, `t`, a `throw` or a `return`, and
/// `rest` never going on past its own: `if (a || b) { t } rest`, as a
/// person writes a guard.
fn guarded(stmts: &mut Vec<Stmt>) {
    for i in 0..stmts.len() {
        let StmtKind::Labeled(label, body) = &stmts[i].kind else {
            continue;
        };
        let tail = &stmts[i + 1..];
        if !tail.last().is_some_and(ends) {
            continue;
        }
        let guards = (body.iter())
            .take_while(|s| {
                matches!(&s.kind, StmtKind::If(_, then, None)
                    if matches!(then.as_slice(), [Stmt { kind: StmtKind::Break(Some(l)), .. }] if l == label))
            })
            .count();
        let rest = &body[guards..];
        if guards == 0 || targets(rest, label) || !rest.last().is_some_and(ends) {
            continue;
        }
        // What `rest` declares, now beside what's before it.
        if declared(rest).any(|name| declared(&stmts[..i]).any(|n| n == name)) {
            continue;
        }
        let test = (body[..guards].iter())
            .map(|s| match &s.kind {
                StmtKind::If(test, ..) => test.clone(),
                _ => unreachable!("a guard"),
            })
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

/// A loop's body that ends in a block, `l: { … break l; … }`, its `break l`
/// the loop's `continue`: the block's statements in its place. And one that
/// ends `if (!c) { a; b }`, as MIR has a `continue` the program wrote,
/// `if (c) { continue; } a; b`.
fn continued(stmts: &mut [Stmt]) {
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
                    let clashes = declared(&inner).any(|name| declared(body).any(|n| n == name));
                    if clashes || continues(&mut inner, &block, looped.as_deref(), false).is_none() {
                        body.push(StmtKind::Labeled(block, inner).at(span));
                        break;
                    }
                    continues(&mut inner, &block, looped.as_deref(), true);
                    body.extend(inner);
                }
                Some(StmtKind::If(test, then, None))
                    if matches!(test.kind, ExprKind::Unary(UnaryOp::Not, _)) && then.len() > 1 =>
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
    }
}

/// A `continue l` or a `break l` of loop `l` where it's the innermost
/// loop: `continue` or `break`, as JS's unlabeled ones go there.
fn innermost_jumps(stmts: &mut [Stmt]) {
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
fn continues(stmts: &mut [Stmt], block: &str, looped: Option<&str>, write: bool) -> Option<()> {
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
fn absent(test: &Expr, item: &str) -> bool {
    match &test.kind {
        ExprKind::Binary(Op::LooseEq, a, b) => {
            matches!(&a.kind, ExprKind::Var(v) if v == item) && matches!(b.kind, ExprKind::Null | ExprKind::Undefined)
        }
        ExprKind::Unary(UnaryOp::Not, a) => matches!(&a.kind, ExprKind::Var(v) if v == item),
        _ => false,
    }
}

/// Whether `test` says `item` is there: `item != null`, or `item`.
fn present(test: &Expr, item: &str) -> bool {
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
fn binding(body: &[Stmt], item: &str) -> Option<(js::Pattern, bool, usize)> {
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
            Some((js::Pattern::Array(items), mutable, parts.len()))
        }
        _ if !parts.is_empty() && parts.iter().all(|(p, ..)| p.is_err()) => {
            let fields = (parts.iter())
                .map(|(p, bound, _)| (p.clone().expect_err("keyed"), bound.clone(), None))
                .collect();
            Some((js::Pattern::Object(fields, None), mutable, parts.len()))
        }
        _ => None,
    }
}

/// Whether a `break` in `stmts` leaves the loop they're the body of, labeled
/// `label`: one of no label not inside a loop of their own, or of its label.
fn breaks(stmts: &[Stmt], label: Option<&str>) -> bool {
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

/// Whether a `break` in `stmts`, however deep, names `label`.
fn names_break(stmts: &[Stmt], label: &str) -> bool {
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

/// Whether `stmts` read `name` alone, `name[i]` or `name.length`: not
/// assign it, call a method of it, or give it to what could change it.
fn only_read(stmts: &[Stmt], name: &str) -> bool {
    let mut stmts = stmts.to_vec();
    let (mut all, mut read) = (0, 0);
    js::each_expr_mut(&mut stmts, &mut |e| match &e.kind {
        ExprKind::Var(v) if v == name => all += 1,
        ExprKind::Index(of, _) | ExprKind::Member(of, _)
            if matches!(&of.kind, ExprKind::Var(v) if v == name)
                && !matches!(&e.kind, ExprKind::Member(_, m) if m != "length") =>
        {
            read += 1
        }
        _ => {}
    });
    all == read && !assigns(&stmts, name)
}

/// Whether `stmts` assign `name`.
fn assigns(stmts: &[Stmt], name: &str) -> bool {
    let mut stmts = stmts.to_vec();
    let mut assigned = false;
    js::statement_lists(&mut stmts, &mut |list| {
        assigned |= list.iter().any(|s| {
            matches!(&s.kind, StmtKind::Assign(target, _)
            if matches!(&target.kind, ExprKind::Var(v) if v == name))
        });
    });
    assigned
}

/// Whether `name` is a temporary MIR made, `_5`, not one the program names.
fn is_temporary(name: &str) -> bool {
    name.strip_prefix('_')
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit() || c == '$'))
}

/// `const _5 = children;`, a temporary of a variable nothing changes, nor it,
/// or of a constant: `children`, or the constant, wherever it's read.
fn alias_variables(body: &mut Vec<Stmt>, changing: &HashSet<String>) {
    let mut aliases: Vec<(String, Expr)> = Vec::new();
    js::statement_lists(body, &mut |stmts| {
        stmts.retain(|stmt| match &stmt.kind {
            StmtKind::Const(name, value)
                if is_temporary(name)
                    && !changing.contains(name)
                    && (value.is_constant() || matches!(&value.kind, ExprKind::Var(of) if !changing.contains(of))) =>
            {
                aliases.push((name.clone(), value.clone()));
                false
            }
            _ => true,
        });
    });
    if aliases.is_empty() {
        return;
    }
    // An alias of an alias, its variable or constant.
    let of = |name: &str| {
        let mut value = Expr::var(name);
        while let ExprKind::Var(name) = &value.kind
            && let Some((_, to)) = aliases.iter().find(|(from, _)| from == name)
        {
            value = to.clone();
        }
        value
    };
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Var(name) = &e.kind
            && aliases.iter().any(|(from, _)| from == name)
        {
            *e = Expr {
                span: e.span,
                ..of(name)
            };
        }
    });
}

/// `const _5 = e; const a = _5[0]; const b = _5[1];`, the temporary read
/// nowhere else, is `const [a, b] = e;`; and of its properties, `const { a,
/// b: c } = e;`; `let` if one of them is. Of a variable the program names,
/// two or more parts in a row, `let (a, b) = t;`'s: `const [a, b] = t;`.
fn destructured(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        // What the parts are read of, and where they start: a temporary this
        // statement makes, the statements after it; or a variable this one
        // reads a part of, here.
        let (name, first) = match &stmts[i].kind {
            StmtKind::Const(name, _) if is_temporary(name) => (name.clone(), i + 1),
            StmtKind::Const(_, value) | StmtKind::Let(_, Some(value))
                if let Some(v) = part_of(value)
                    && !is_temporary(v) =>
            {
                (v.to_string(), i)
            }
            _ => {
                i += 1;
                continue;
            }
        };
        let made = first > i;
        let parts = parts(&stmts[first..], &name);
        let count = parts.len();
        let indexed = parts.iter().all(|(p, ..)| p.is_ok());
        let enough = count >= if made { 1 } else { 2 };
        let pattern = pattern_of(&parts);
        if !enough || pattern.is_none() || (made && js::mentions_in(&stmts[first + count..], &name) > 0) {
            i += 1;
            continue;
        }
        // Of an array made here, `const [, b] = [x, 6]`: each part itself,
        // where what no part reads does nothing.
        if made
            && indexed
            && let StmtKind::Const(_, value) = &stmts[i].kind
            && let ExprKind::Array(items) = &value.kind
            && parts.windows(2).all(|w| w[0].0 < w[1].0)
            && parts.iter().all(|(p, ..)| matches!(p, Ok(k) if *k < items.len()))
            && (items.iter().enumerate()).all(|(k, item)| {
                parts.iter().any(|(p, ..)| *p == Ok(k)) || item.is_constant() || matches!(item.kind, ExprKind::Var(_))
            })
        {
            let span = stmts[i].span;
            let each: Vec<Stmt> = (parts.iter())
                .map(|(part, bound, mutable, _)| {
                    let item = items[*part.as_ref().expect("indexed")].clone();
                    match mutable {
                        true => StmtKind::Let(bound.clone(), Some(item)),
                        false => StmtKind::Const(bound.clone(), item),
                    }
                    .at(span)
                })
                .collect();
            stmts.splice(i..first + count, each);
            continue;
        }
        let mutable = parts.iter().any(|(_, _, m, _)| *m);
        let pattern = pattern.expect("checked");
        let value = match made {
            true => {
                stmts.drain(first..first + count);
                let StmtKind::Const(_, value) = std::mem::replace(&mut stmts[i].kind, StmtKind::Break(None)) else {
                    unreachable!("matched")
                };
                value
            }
            false => {
                stmts.drain(i + 1..i + count);
                Expr::var(&name)
            }
        };
        stmts[i].kind = StmtKind::Destructure {
            pattern,
            value,
            mutable,
        };
        i += 1;
    }
}

/// A part of a value read into a variable: its index or key, the variable,
/// and whether it's a `let`.
/// And a key's default, `{ size = "md" }`'s.
type Part = (Result<usize, String>, String, bool, Option<Expr>);

/// The parts of `name` the first of `stmts` read, each a `const` or a
/// `let` of one, `const a = name[0];` or `const b = name.b;`.
fn parts(stmts: &[Stmt], name: &str) -> Vec<Part> {
    let mut parts = Vec::new();
    for stmt in stmts {
        let (bound, value, mutable) = match &stmt.kind {
            StmtKind::Const(bound, value) => (bound, value, false),
            StmtKind::Let(bound, Some(value)) => (bound, value, true),
            _ => break,
        };
        if bound == name {
            break;
        }
        let key = |e: &Expr| match &e.kind {
            ExprKind::Member(of, key) if matches!(&of.kind, ExprKind::Var(v) if v == name) => Some(key.clone()),
            _ => None,
        };
        let (part, default) = match &value.kind {
            ExprKind::Index(of, index)
                if matches!(&of.kind, ExprKind::Var(v) if v == name)
                    && let ExprKind::Num(k) = index.kind
                    && k >= 0.0
                    && k.fract() == 0.0 =>
            {
                (Ok(k as usize), None)
            }
            ExprKind::Member(..) if let Some(key) = key(value) => (Err(key), None),
            // A field with a default, `o.k === undefined ? d : o.k`.
            ExprKind::Cond(test, default, read)
                if let ExprKind::Binary(Op::Eq, tested, undefined) = &test.kind
                    && matches!(undefined.kind, ExprKind::Undefined)
                    && let Some(k) = key(read)
                    && key(tested).is_some_and(|t| t == k) =>
            {
                (Err(k), Some((**default).clone()))
            }
            _ => break,
        };
        parts.push((part, bound.clone(), mutable, default));
    }
    parts
}

/// The variable `value` reads a part of, as `parts` reads one: `o[0]`,
/// `o.k`, or `o.k === undefined ? d : o.k`.
fn part_of(value: &Expr) -> Option<&str> {
    let of = match &value.kind {
        ExprKind::Index(of, _) | ExprKind::Member(of, _) => of,
        ExprKind::Cond(_, _, read) => match &read.kind {
            ExprKind::Member(of, _) => of,
            _ => return None,
        },
        _ => return None,
    };
    match &of.kind {
        ExprKind::Var(v) => Some(v),
        _ => None,
    }
}

/// What takes `parts` apart, `[a, , c]` of indices, `{ a, b: c }` of keys:
/// none of none, or of both.
fn pattern_of(parts: &[Part]) -> Option<js::Pattern> {
    if parts.is_empty() {
        return None;
    }
    if parts.iter().all(|(p, ..)| p.is_ok()) {
        let last = parts.iter().filter_map(|(p, ..)| p.as_ref().ok()).max().copied()?;
        let mut items = vec![None; last + 1];
        for (part, bound, ..) in parts {
            items[*part.as_ref().ok()?] = Some(bound.clone());
        }
        return Some(js::Pattern::Array(items));
    }
    let keyed: Option<Vec<_>> = (parts.iter())
        .map(|(part, bound, _, default)| Some((part.clone().err()?, bound.clone(), default.clone())))
        .collect();
    Some(js::Pattern::Object(keyed?, None))
}

/// A parameter MIR names, `_2`, whose parts the body reads where it starts
/// and reads nowhere else, `(_2) => { const i = _2[0]; const c = _2[1]; … }`:
/// taken apart where it's given, `([i, c]) => …`, as THIR writes a pattern.
pub(super) fn destructured_params(params: &mut [js::Pattern], body: &mut Vec<Stmt>) {
    for param in params.iter_mut() {
        let js::Pattern::Name(name) = param else { continue };
        if !is_temporary(name) {
            continue;
        }
        let name = name.clone();
        let read = parts(body, &name);
        let count = read.len();
        if js::mentions_in(&body[count..], &name) > 0 {
            continue;
        }
        let Some(pattern) = pattern_of(&read) else { continue };
        body.drain(..count);
        *param = pattern;
    }
}

/// `const _3 = e; … if (_3) { const current = _3; … }`: a temporary that a
/// variable the program names takes, in a block inside where it's read,
/// `if let Some(current) = r.current()`'s, where nothing else is named so:
/// `const current = e;`, read by that name, as THIR binds it.
fn named_temporaries(body: &mut Vec<Stmt>) {
    // How often each name is declared, and read, in the whole body.
    let mut declared: HashMap<String, usize> = HashMap::new();
    js::statement_lists(&mut body.clone(), &mut |stmts| {
        for stmt in stmts.iter() {
            let names = match &stmt.kind {
                StmtKind::Const(name, _) | StmtKind::Let(name, _) => vec![name.as_str()],
                StmtKind::Destructure { pattern, .. } => pattern.names(),
                _ => Vec::new(),
            };
            for name in names {
                *declared.entry(name.to_string()).or_default() += 1;
            }
        }
    });
    let mut reads: HashMap<String, usize> = HashMap::new();
    js::visit_stmts(body, &mut |name| *reads.entry(name.to_string()).or_default() += 1);
    js::statement_lists(body, &mut |stmts| {
        // What it takes goes from after it: the list shortens.
        let mut next = 0;
        while next < stmts.len() {
            let i = next;
            next += 1;
            let StmtKind::Const(a, _) = &stmts[i].kind else {
                continue;
            };
            if !is_temporary(a) {
                continue;
            }
            let a = a.clone();
            let mut found = Vec::new();
            taking(&stmts[i + 1..], &a, &mut found);
            let [(b, after)] = found.as_slice() else { continue };
            if declared.get(b) != Some(&1) || reads.get(b).copied().unwrap_or(0) != *after {
                continue;
            }
            let b = b.clone();
            dropping(stmts, &a, &b);
            let StmtKind::Const(name, _) = &mut stmts[i].kind else {
                unreachable!("matched")
            };
            *name = b.clone();
            js::each_expr_mut(&mut stmts[i + 1..], &mut |e| {
                if let ExprKind::Var(v) = &mut e.kind
                    && *v == a
                {
                    *v = b.clone();
                }
            });
        }
    });
}

/// Each `const b = a;` of a name the program gives in the blocks of
/// `stmts`, not in their closures, and how often `b` is read after it.
fn taking(stmts: &[Stmt], a: &str, found: &mut Vec<(String, usize)>) {
    for (j, stmt) in stmts.iter().enumerate() {
        match &stmt.kind {
            StmtKind::Const(b, value) if !is_temporary(b) && matches!(&value.kind, ExprKind::Var(v) if v == a) => {
                found.push((b.clone(), js::mentions_in(&stmts[j + 1..], b)));
            }
            StmtKind::If(_, then, otherwise) => {
                taking(then, a, found);
                if let Some(otherwise) = otherwise {
                    taking(otherwise, a, found);
                }
            }
            StmtKind::While { body, .. }
            | StmtKind::ForOf { body, .. }
            | StmtKind::For { body, .. }
            | StmtKind::Labeled(_, body) => taking(body, a, found),
            StmtKind::Try(x, y) | StmtKind::TryCatch(x, _, y) => {
                taking(x, a, found);
                taking(y, a, found);
            }
            _ => {}
        }
    }
}

/// `stmts` without `taking`'s `const b = a;`, theirs or their blocks'.
fn dropping(stmts: &mut Vec<Stmt>, a: &str, b: &str) {
    stmts.retain(|s| {
        !matches!(&s.kind, StmtKind::Const(n, value)
            if n == b && matches!(&value.kind, ExprKind::Var(v) if v == a))
    });
    for stmt in stmts.iter_mut() {
        let lists: Vec<&mut Vec<Stmt>> = match &mut stmt.kind {
            StmtKind::If(_, then, otherwise) => std::iter::once(then).chain(otherwise.as_mut()).collect(),
            StmtKind::While { body, .. }
            | StmtKind::ForOf { body, .. }
            | StmtKind::For { body, .. }
            | StmtKind::Labeled(_, body) => vec![body],
            StmtKind::Try(x, y) | StmtKind::TryCatch(x, _, y) => vec![x, y],
            _ => Vec::new(),
        };
        for list in lists {
            dropping(list, a, b);
        }
    }
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
fn branch_values(stmts: &mut Vec<Stmt>, returns: &[js::Span], booleans: &HashSet<String>) {
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
                    // A temporary's, an `if` the program wrote as a value, is one.
                    && ((small(yes) && small(no)) || is_temporary(x)) =>
            {
                StmtKind::Assign(Expr::var(x), conditional(test, yes.clone(), no.clone(), booleans))
            }
            // `l: { if (!a) { if (!b) { s; break l; } } t }`, `||`'s: `if (a ||
            // b) { t } else { s }`, where nothing else breaks to `l`; and
            // `if (a && b) { s } else { t }` of tests not all negated.
            StmtKind::Labeled(label, body) if let Some((tests, otherwise, then)) = or_chain(&body, &label) => {
                let negated = tests.iter().all(|t| matches!(t.kind, ExprKind::Unary(UnaryOp::Not, _)));
                let test = (tests.into_iter()).rev().reduce(|b, a| both(a, b)).expect("one");
                match (then.is_empty(), negated) {
                    (true, _) => StmtKind::If(test, otherwise, None),
                    (false, true) => StmtKind::If(not(test), then, Some(otherwise)),
                    (false, false) => StmtKind::If(test, otherwise, Some(then)),
                }
            }
            StmtKind::Labeled(label, body) => match chain(&body, &label) {
                Some((tests, x, yes, no)) => {
                    let test = (tests.into_iter()).rev().reduce(|b, a| both(a, b)).expect("one");
                    StmtKind::Assign(Expr::var(&x), conditional(test, yes, no, booleans))
                }
                None => StmtKind::Labeled(label, body),
            },
            kind => kind,
        };
        stmt.span = span;
    }
    stmts.retain(|stmt| !matches!(&stmt.kind, StmtKind::Expr(e) if matches!(e.kind, ExprKind::Undefined)));
    // `if (x != null) { if (x === "a") { s } else if (x === "b") { t } }`:
    // the chains alone, which say it.
    let mut i = 0;
    while i < stmts.len() {
        match &stmts[i].kind {
            StmtKind::If(test, then, None) if !then.is_empty() && then.iter().all(|s| guarded_chain(s, test)) => {
                let StmtKind::If(_, then, _) = stmts.remove(i).kind else {
                    unreachable!("matched")
                };
                let n = then.len();
                stmts.splice(i..i, then);
                i += n;
            }
            _ => i += 1,
        }
    }
    // `if (a) { if (b) { return t; } } return f;`, last: `return a && b ? t : f;`.
    if let [.., last_if, last] = stmts.as_slice()
        && let StmtKind::Return(Some(no)) = &last.kind
        && let StmtKind::If(..) = &last_if.kind
        && let Some((tests, yes)) = returned_chain(std::slice::from_ref(last_if))
        && small(&yes)
        && small(no)
        && boolean_branch(&yes, no)
        // Not where the program wrote `return`, an early one.
        && !written(returns, last.span)
        && !writes_return(std::slice::from_ref(last_if), returns)
    {
        let test = (tests.into_iter()).rev().reduce(|b, a| both(a, b)).expect("one");
        let value = conditional(test, yes, no.clone(), booleans);
        let span = last_if.span;
        stmts.truncate(stmts.len() - 2);
        stmts.push(StmtKind::Return(Some(value)).at(span));
    } else if let [before @ .., last_if, last] = stmts.as_slice()
        && let StmtKind::Return(Some(_)) = &last.kind
        && written(returns, last.span)
        && let Some((tests, body)) = returning_chain(std::slice::from_ref(last_if))
        && !declared(&body).any(|name| declared(before).any(|n| n == name))
    {
        // `f` the program wrote, a `let … else`'s else: `if (!(a && b)) {
        // return f; } t`, as THIR guards it.
        let test = (tests.into_iter()).rev().reduce(|b, a| both(a, b)).expect("one");
        let (span, guard) = (last_if.span, stmts.pop().expect("the return"));
        stmts.pop();
        stmts.push(StmtKind::If(not(test), vec![guard], None).at(span));
        stmts.extend(body);
    }
}

/// Of `if (a) { const x = e; if (b) { s } }`, `s` ending where it leaves:
/// `[a, b]` and `const x = e; s`, what a level binds before its test, a
/// read that does nothing, written into later tests, as `broken_chain`'s.
fn returning_chain(stmts: &[Stmt]) -> Option<(Vec<Expr>, Vec<Stmt>)> {
    let [
        bound @ ..,
        Stmt {
            kind: StmtKind::If(test, inner, None),
            ..
        },
    ] = stmts
    else {
        return None;
    };
    if !bound
        .iter()
        .all(|s| matches!(&s.kind, StmtKind::Const(_, value) if value.reads_same()))
    {
        return None;
    }
    let (mut tests, body) = match returning_chain(inner) {
        Some(chain) => chain,
        None if inner.last().is_some_and(ends) => (Vec::new(), inner.clone()),
        None => return None,
    };
    tests.insert(0, test.clone());
    for stmt in bound.iter().rev() {
        let StmtKind::Const(name, value) = &stmt.kind else {
            unreachable!("matched")
        };
        for test in &mut tests {
            *test = test.substitute(&|v| (v == name).then(|| value.clone()))?;
        }
    }
    let mut all = bound.to_vec();
    all.extend(body);
    Some((tests, all))
}

/// Whether of `c ? yes : no` one is `true` or `false`: `c || no` or `c &&
/// yes`, not `?:`.
fn boolean_branch(yes: &Expr, no: &Expr) -> bool {
    matches!(yes.kind, ExprKind::Bool(_)) || matches!(no.kind, ExprKind::Bool(_))
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

/// Of `if (a) { if (b) { s; break l; } }`: `[a, b]` and `s`. What a level
/// binds before its test, `if (a) { const x = o.x; if (x) { … } }`, a read
/// that does nothing, comes first in `s`, and a test reads its value,
/// `a && o.x`, as `if let`s chained with `&&` read.
fn broken_chain(stmts: &[Stmt], label: &str) -> Option<(Vec<Expr>, Vec<Stmt>)> {
    match stmts {
        [
            bound @ ..,
            Stmt {
                kind: StmtKind::If(test, inner, None),
                ..
            },
        ] if bound
            .iter()
            .all(|s| matches!(&s.kind, StmtKind::Const(_, value) if value.reads_same())) =>
        {
            let (mut tests, otherwise) = broken_chain(inner, label)?;
            tests.insert(0, test.clone());
            for stmt in bound.iter().rev() {
                let StmtKind::Const(name, value) = &stmt.kind else {
                    unreachable!("matched")
                };
                for test in &mut tests {
                    *test = test.substitute(&|v| (v == name).then(|| value.clone()))?;
                }
            }
            let mut body = bound.to_vec();
            body.extend(otherwise);
            Some((tests, body))
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
        ExprKind::Binary(op @ (Op::And | Op::Or), a, b)
            if is_boolean(&a, &HashSet::new()) && is_boolean(&b, &HashSet::new()) =>
        {
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

/// `test ? yes : no`: `test && yes` where `no` is `false`, and `test || no`
/// where `yes` is `true`, the test `!!test` where it's not a `bool`.
fn conditional(mut test: Expr, yes: Expr, no: Expr, booleans: &HashSet<String>) -> Expr {
    // A test is read for its truth alone: `!!c`'s is `c`'s.
    while let ExprKind::Unary(UnaryOp::Not, inner) = &test.kind
        && let ExprKind::Unary(UnaryOp::Not, c) = &inner.kind
    {
        test = (**c).clone();
    }
    // Where the test itself is the value, of what's not a `bool`, `!!c`, as
    // THIR writes it.
    let truth = |test: Expr| match is_boolean(&test, booleans) {
        true => test,
        false => Expr::unary(UnaryOp::Not, Expr::unary(UnaryOp::Not, test)),
    };
    match (&yes.kind, &no.kind) {
        (ExprKind::Bool(true), ExprKind::Bool(false)) => truth(test),
        (ExprKind::Bool(false), ExprKind::Bool(true)) => negate(test),
        (ExprKind::Bool(true), _) => Expr::bin(Op::Or, truth(test), no),
        (ExprKind::Bool(false), _) => Expr::bin(Op::And, negate(test), no),
        (_, ExprKind::Bool(true)) => Expr::bin(Op::Or, negate(test), yes),
        (_, ExprKind::Bool(false)) => both(truth(test), yes),
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
    tests.retain(|t| !implies(first, t));
    tests
        .into_iter()
        .rev()
        .fold(b, |rest, test| Expr::bin(Op::And, test, rest))
}

/// Whether `test` holds where `by` does: `x != null` where `x === "lit"`,
/// `typeof x === "string"` there too, and where each side of `a || b` says
/// it.
fn implies(by: &Expr, test: &Expr) -> bool {
    match (&test.kind, &by.kind) {
        (_, ExprKind::Binary(Op::Or, a, b)) => implies(a, test) && implies(b, test),
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
    }
}

/// Whether each test of the `else if` chain `stmt` says `guard`, and it has
/// no last `else`: where `guard` fails, it does nothing.
fn guarded_chain(stmt: &Stmt, guard: &Expr) -> bool {
    match &stmt.kind {
        StmtKind::If(test, _, otherwise) => {
            implies(test, guard)
                && (otherwise.as_deref()).is_none_or(|rest| matches!(rest, [next] if guarded_chain(next, guard)))
        }
        _ => false,
    }
}

/// Whether `a` and `b` are the same variable, or the same property of one.
fn same(a: &Expr, b: &Expr) -> bool {
    match (&a.kind, &b.kind) {
        (ExprKind::Var(x), ExprKind::Var(y)) => x == y,
        (ExprKind::Member(x, p), ExprKind::Member(y, q)) => p == q && same(x, y),
        _ => false,
    }
}

/// Whether `e` is a `bool` whatever it reads: a comparison, a `!`, one of
/// `booleans`, or `&&` and `||` of them.
fn is_boolean(e: &Expr, booleans: &HashSet<String>) -> bool {
    match &e.kind {
        ExprKind::Bool(_) | ExprKind::Unary(UnaryOp::Not, _) => true,
        ExprKind::Var(name) => booleans.contains(name),
        ExprKind::Binary(
            Op::Eq | Op::Ne | Op::LooseEq | Op::LooseNe | Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::InstanceOf | Op::In,
            ..,
        ) => true,
        ExprKind::Binary(Op::And | Op::Or, a, b) => is_boolean(a, booleans) && is_boolean(b, booleans),
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
/// after it, or after those that are moved there too: `e` where it's read, where nothing that runs before it there
/// does anything, or anywhere outside a closure if `e` does nothing and
/// reads nothing that changes.
fn inlined(stmts: &mut Vec<Stmt>, changing: &HashSet<String>, captured: &HashSet<String>) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        let (head, rest) = stmts.split_at_mut(i + 1);
        let StmtKind::Const(name, value) = &head[i].kind else {
            i += 1;
            continue;
        };
        let temporary = is_temporary(name) || captured.contains(name);
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
        // The one before it may now be read by the statement after it too.
        i = i.saturating_sub(1);
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
        // `t = e; t = f;`, `e` doing nothing and `f` not reading `t`: `t = f;`,
        // a `vec!`'s box made, then filled.
        if let (StmtKind::Assign(first, made), StmtKind::Assign(second, filled)) = (&stmts[i].kind, &stmts[i + 1].kind)
            && let (ExprKind::Var(a), ExprKind::Var(b)) = (&first.kind, &second.kind)
            && a == b
            && !made.has_effects()
            && !filled.mentions_var(a)
        {
            stmts.remove(i);
            continue;
        }
        // `result = e; return result;`, or a temporary's.
        let returned_var = |name: &str| name == result || is_temporary(name);
        let assigns = matches!(&stmts[i].kind, StmtKind::Assign(target, _)
            if matches!(&target.kind, ExprKind::Var(name) if returned_var(name)));
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

/// What follows a `return`, a `throw`, a `break` or a `continue` in the
/// same list never runs, nor what follows an `if` each of whose branches
/// ends so, nor a `while (true)` no `break` leaves.
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
        StmtKind::While { label, cond, body } => {
            matches!(cond.kind, ExprKind::Bool(true)) && !breaks(body, label.as_deref())
        }
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
            } => drop_tail(body, &StmtKind::Continue(Some(label.clone()))),
            _ => {}
        }
    }
}

/// `const h = x ? x : undefined; if (h == null) { exit }`, a `let … else`
/// of what `filter` keeps: `if (!x) { exit }`, as THIR tests it, `h` and
/// what binds it then `x`, which it is.
fn filtered_guards(stmts: &mut Vec<Stmt>, changing: &HashSet<String>) {
    for i in 1..stmts.len() {
        let [made, checked] = &stmts[i - 1..=i] else {
            unreachable!("two")
        };
        let StmtKind::Const(h, value) = &made.kind else {
            continue;
        };
        let ExprKind::Cond(test, yes, no) = &value.kind else {
            continue;
        };
        let test = test.tested();
        if let ExprKind::Var(x) = &test.kind
            && !changing.contains(x)
            && matches!(&yes.kind, ExprKind::Var(y) if y == x)
            && matches!(no.kind, ExprKind::Undefined)
            && let StmtKind::If(guard, then, None) = &checked.kind
            && let ExprKind::Binary(Op::LooseEq, tested, null) = &guard.kind
            && matches!(&tested.kind, ExprKind::Var(t) if t == h)
            && matches!(null.kind, ExprKind::Null | ExprKind::Undefined)
            && then.last().is_some_and(ends)
        {
            // `h`, and a binding of it nothing changes, read as `x`: unread,
            // they go.
            let (h, x) = (h.clone(), x.clone());
            let mut read: Vec<String> = vec![h];
            stmts.remove(i - 1);
            for stmt in &stmts[i - 1..] {
                if let StmtKind::Const(name, value) = &stmt.kind
                    && matches!(&value.kind, ExprKind::Var(of) if read.contains(of))
                    && !changing.contains(name)
                {
                    read.push(name.clone());
                }
            }
            let StmtKind::If(guard, ..) = &mut stmts[i - 1].kind else {
                unreachable!("the guard, where the `const` was")
            };
            *guard = Expr::unary(UnaryOp::Not, Expr::var(&x));
            js::each_expr_mut(&mut stmts[i - 1..], &mut |e| {
                if matches!(&e.kind, ExprKind::Var(v) if read.contains(v)) {
                    *e = Expr {
                        span: e.span,
                        ..Expr::var(&x)
                    };
                }
            });
            return;
        }
    }
}

/// A labeled block's `if (c) { s; break l; } t`, `s` leaving for where
/// `t` goes on to, its end: `if (c) { s } else { t }`, a `match`'s arms
/// as a person writes them, the `break`s then ending the branches; and a
/// block that ends it, one with it. Of `if (!a) { if (!b) { s; break l; }
/// } t`, an or-pattern's, `if (a || b) { t } else { s }`.
fn branch_ends(stmts: &mut [Stmt]) {
    for stmt in stmts {
        if let StmtKind::Labeled(label, body) = &mut stmt.kind {
            ending(body, label);
        }
    }
}

/// `stmts`, which end where block `label` does, as `branch_ends` has them.
fn ending(stmts: &mut Vec<Stmt>, label: &str) {
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
    let test = (tests.into_iter()).rev().reduce(|b, a| both(a, b)).expect("one");
    stmts.push(match (rest.is_empty(), negated) {
        (true, _) => StmtKind::If(test, then, None).at(span),
        (false, true) => StmtKind::If(not(test), rest, Some(then)).at(span),
        (false, false) => StmtKind::If(test, then, Some(rest)).at(span),
    });
}

/// Of `if (b) { if (c) { s } }`, the body of an `if`: its tests, `[b, c]`,
/// and `s`; of others, none and themselves.
fn ands(stmts: &[Stmt]) -> (Vec<Expr>, Vec<Stmt>) {
    match stmts {
        [
            Stmt {
                kind: StmtKind::If(test, inner, None),
                ..
            },
        ] => {
            let (mut tests, body) = ands(inner);
            tests.insert(0, test.clone());
            (tests, body)
        }
        _ => (Vec::new(), stmts.to_vec()),
    }
}

/// Whether `stmts` always end with a `break label`.
fn leaves(stmts: &[Stmt], label: &str) -> bool {
    match stmts.last().map(|s| &s.kind) {
        Some(StmtKind::Break(Some(l))) => l == label,
        Some(StmtKind::If(_, then, Some(otherwise))) => leaves(then, label) && leaves(otherwise, label),
        _ => false,
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
