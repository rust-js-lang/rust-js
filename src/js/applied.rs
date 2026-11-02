//! A closure called where it's made, `((q) => { .. })(q);`: its body in
//! its place, as a statement whose value nothing reads (ADR 0581), and as
//! what a statement evaluates first, its value where the call was (ADR
//! 0840).

use super::*;
use std::collections::HashSet;

/// `((taskQueue) => { .. })(taskQueue);`, a closure called where it's made,
/// as a statement whose value nothing reads: its body in its place, its
/// parameter the variable it's given, as a plain thread-local's
/// `with_borrow_mut(|task_queue| ..)` reads its variable (ADR 0581). A
/// `return;` in it leaves the closure, so it's in place only where the
/// call ends the function's body, a function's, never a module's, which
/// `ends_function` says; and what its body declares, where no other name
/// of the function is the same. And one that's what a statement evaluates
/// first, `if (((d) => { .. return v; })(warned))` of a thread-local's
/// `with(|d| ..)`, that returns only at its end: its body before the
/// statement, and `v` in the call's place (`value_of`, ADR 0840). In each
/// closure `body` makes too, whose body a `return;` ends, as `REPORT.with(|&f|
/// ..)` in a `catch_unwind`'s closure is (ADR 0840, amended).
pub fn applied_blocks(body: &mut Vec<Stmt>, ends_function: bool) {
    applied_in(body, ends_function);
    each_expr_mut(body, &mut applied_in_closure);
}

/// `applied_blocks`' of each closure `value` makes, a `const`'s value's,
/// a thread-local's that's its module's variable: each its own body, which
/// a `return;` ends, as React's DEV dispatchers' `useA() {
/// NESTED.with(|d| set_h(d.as_ref())) }` is `setH(nested ?? null)` (ADR
/// 0581, amended). The value itself is no body.
pub fn applied_closures(value: &mut Expr) {
    value.each_mut(&mut applied_in_closure);
}

/// `applied_in` of `e`'s body, where it's a closure or a function, whose
/// body a `return;` ends.
fn applied_in_closure(e: &mut Expr) {
    match &mut e.kind {
        ExprKind::Arrow(_, inner) | ExprKind::AsyncArrow(_, inner) => applied_in(inner, true),
        ExprKind::Function(function) => applied_in(&mut function.body, true),
        _ => {}
    }
}

/// `applied_blocks`' of `body` itself, not the closures it makes.
fn applied_in(body: &mut Vec<Stmt>, ends_function: bool) {
    loop {
        let whole = body.clone();
        if ends_function && tail_applied(body, &whole) {
            continue;
        }
        let mut done = false;
        statement_lists(body, &mut |stmts| {
            if done {
                return;
            }
            let inlined = |stmt: &Stmt| block_of(stmt, &whole, false, true, false).or_else(|| value_of(stmt, &whole));
            if let Some((i, inlined)) = (0..stmts.len()).find_map(|i| Some((i, inlined(&stmts[i])?))) {
                stmts.splice(i..=i, inlined);
                done = true;
                return;
            }
            // The last of a case whose end leaves its `switch`, where its
            // `return;` is the `break;` there (rd379).
            done = (stmts.iter_mut()).any(|stmt| {
                let StmtKind::Switch { cases, default, .. } = &mut stmt.kind else {
                    return false;
                };
                leaving_cases(cases, default).any(|list| {
                    let Some(inlined) = list.last().and_then(|last| block_of(last, &whole, false, true, true)) else {
                        return false;
                    };
                    list.pop();
                    list.extend(inlined);
                    true
                })
            });
        });
        if !done {
            break;
        }
    }
}

/// The call that ends the function's body `whole`, the last of `stmts`,
/// or the last of a branch of an `if` that's last, which ends it too, an
/// `else` the program wrote's (rd106): its body in its place, a `return;`
/// in it the function's. Whether one was.
fn tail_applied(stmts: &mut Vec<Stmt>, whole: &[Stmt]) -> bool {
    let Some(last) = stmts.len().checked_sub(1) else {
        return false;
    };
    if let Some(inlined) =
        block_of(&stmts[last], whole, true, true, false).or_else(|| returned_block(&stmts[last], whole))
    {
        stmts.splice(last..=last, inlined);
        return true;
    }
    match &mut stmts[last].kind {
        StmtKind::If(_, then, otherwise) => {
            let branch = |b: &mut Option<Vec<Stmt>>| b.as_mut().is_some_and(|b| tail_applied(b, whole));
            tail_applied(then, whole) || branch(otherwise)
        }
        // A `switch` that's last, a `match` of constants' (ADR 0629): each
        // case's whose end leaves it, and so ends the function (rd378).
        StmtKind::Switch { cases, default, .. } => leaving_cases(cases, default).any(|list| tail_applied(list, whole)),
        _ => false,
    }
}

/// The lists of a `switch`'s cases whose end leaves it: each case's that
/// doesn't fall through to the next, which is written its `break;`, and
/// the last's.
fn leaving_cases<'s>(
    cases: &'s mut [Case],
    default: &'s mut Option<Vec<Stmt>>,
) -> impl Iterator<Item = &'s mut Vec<Stmt>> {
    let last = (cases.len() + usize::from(default.is_some())).saturating_sub(1);
    let falls: Vec<bool> = cases.iter().map(|case| case.falls).collect();
    (case_lists_mut(cases, default).enumerate())
        .filter(move |(i, _)| *i == last || !falls[*i])
        .map(|(_, list)| list)
}

/// `f(arg);`, a call as a statement in `whole`, the function's body, of a
/// function of `params` and `body`: its body in its place, where a closure
/// of them called there would be; a `Drop` impl's, at its one drop (ADR
/// 0588).
pub fn applied_call(stmt: &Stmt, whole: &[Stmt], params: &[Pattern], body: &[Stmt]) -> Option<Vec<Stmt>> {
    let StmtKind::Expr(e) = &stmt.kind else { return None };
    let ExprKind::Call(_, args) = &e.kind else {
        return None;
    };
    let call = StmtKind::Expr(Expr::call(Expr::arrow(params.to_vec(), body.to_vec()), args.clone())).at(stmt.span);
    block_of(&call, whole, false, false, false)
}

/// The statements `stmt`, a call of a closure made there, is, in `whole`,
/// the function's body; `tail`, the body's last, which a `return;` ends too;
/// `hold`, of a closure the program made, which may be given a lazy
/// thread-local's call (`in_place`); `leaves`, the last of a case whose
/// end leaves its `switch`, as a `break;` there would.
fn block_of(stmt: &Stmt, whole: &[Stmt], tail: bool, hold: bool, leaves: bool) -> Option<Vec<Stmt>> {
    let StmtKind::Expr(e) = &stmt.kind else { return None };
    let ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    let ExprKind::Arrow(params, body) = &callee.kind else {
        return None;
    };
    // The closure's `return`s are its own, the function's only at its end,
    // and of no value, which nothing read; elsewhere, each in the loop its
    // body ends with, that loop's `break`; or, the last of a case whose
    // end leaves its `switch`, `SEEN.with(|seen| { if .. { return; } .. })`
    // of a `match`'s arm, that `switch`'s (rd379).
    let (any, valued) = returns(body);
    let mut body = body.clone();
    if valued {
        return None;
    }
    if any && !tail && !returns_broken(&mut body) {
        let mut broken = body.clone();
        if !(leaves && returns_as_breaks(&mut broken)) {
            return None;
        }
        body = broken;
    }
    in_place(stmt, whole, &[], (params, args), body, hold)
}

/// `return ((d) => { .. return v; .. })(x);`, a closure called where it's
/// made whose value the function's body ends returning: its body in place,
/// each `return` of it the function's, as the closure's value is what the
/// function returns, and its end the function's, which gives `undefined`
/// as the closure's does. React's warnAboutUpdateOnNotYetMountedFiberInDEV,
/// a thread-local's `with_borrow_mut(|did_warn| { .. return true; .. })`
/// (bx1, ADR 0581, amended).
fn returned_block(stmt: &Stmt, whole: &[Stmt]) -> Option<Vec<Stmt>> {
    let StmtKind::Return(Some(e)) = &stmt.kind else {
        return None;
    };
    let ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    let ExprKind::Arrow(params, body) = &callee.kind else {
        return None;
    };
    in_place(stmt, whole, &[], (params, args), body.clone(), true)
}

/// The statements `stmt` is, in `whole`, the function's body, where what
/// it evaluates first is a call of a closure made there whose body
/// returns only at its end, `return v;`: the body before it, and `stmt`
/// with `v` in the call's place, as running the body there and reading
/// `v` is what the call does, before anything else of `stmt` runs. Of a
/// statement whose value it is, its test, a `const`'s or a variable's
/// value, what it returns or throws, or a `switch`'s discriminant; `v;`
/// left out where it does nothing.
fn value_of(stmt: &Stmt, whole: &[Stmt]) -> Option<Vec<Stmt>> {
    let mut with_value = stmt.clone();
    let call = first_call(&mut with_value)?;
    let ExprKind::Call(callee, args) = &call.kind else {
        return None;
    };
    let ExprKind::Arrow(params, body) = &callee.kind else {
        return None;
    };
    let (last, before) = body.split_last()?;
    if !matches!(last.kind, StmtKind::Return(Some(_))) || returns(before).0 {
        return None;
    }
    // What else the statement reads or declares, which what the body
    // declares beside it mustn't be either: `const x` of `const x = v`.
    let mut beside = stmt.clone();
    if let Some(call) = first_call(&mut beside) {
        *call = Expr::undefined();
    }
    let mut inlined = in_place(stmt, whole, &[beside], (params, args), body.clone(), true)?;
    let Some(Stmt {
        kind: StmtKind::Return(Some(value)),
        ..
    }) = inlined.pop()
    else {
        unreachable!("the body's last `return`");
    };
    *call = Expr {
        span: call.span,
        ..value
    };
    if !matches!(&with_value.kind, StmtKind::Expr(e) if !e.has_effects()) {
        inlined.push(with_value);
    }
    Some(inlined)
}

/// Whether `e` is the call of a closure made where it's called.
fn closure_called(e: &Expr) -> bool {
    matches!(&e.kind, ExprKind::Call(callee, _) if matches!(callee.kind, ExprKind::Arrow(..)))
}

/// What `e` evaluates before anything else of it that `is` says it is:
/// itself, or what an operator, a test, or a property's read evaluates
/// first.
fn evaluated_first<'e>(e: &'e mut Expr, is: &dyn Fn(&Expr) -> bool) -> Option<&'e mut Expr> {
    if is(e) {
        return Some(e);
    }
    match &mut e.kind {
        ExprKind::Unary(op, a) if !op.writes() => evaluated_first(a, is),
        ExprKind::Binary(_, a, _) | ExprKind::Cond(a, ..) | ExprKind::Member(a, _) | ExprKind::Index(a, _) => {
            evaluated_first(a, is)
        }
        _ => None,
    }
}

/// The call `value_of` takes of `stmt`, where it's one.
fn first_call(stmt: &mut Stmt) -> Option<&mut Expr> {
    first_of(stmt, &closure_called)
}

/// What `stmt` evaluates before anything else of it that `is` says it is:
/// of a statement whose value it is, its test, a `const`'s or a variable's
/// value, what it returns or throws, or a `switch`'s discriminant.
fn first_of<'s>(stmt: &'s mut Stmt, is: &dyn Fn(&Expr) -> bool) -> Option<&'s mut Expr> {
    match &mut stmt.kind {
        StmtKind::Expr(e)
        | StmtKind::Const(_, e)
        | StmtKind::Let(_, Some(e))
        | StmtKind::Return(Some(e))
        | StmtKind::Throw(e)
        | StmtKind::If(e, ..)
        | StmtKind::Switch { discriminant: e, .. } => evaluated_first(e, is),
        // A variable's, which nothing's read of before its value.
        StmtKind::Assign(target, e) if matches!(target.kind, ExprKind::Var(_)) => evaluated_first(e, is),
        _ => None,
    }
}

/// Of `stmts`, the `const name = v;` of a name the program never gave the
/// value, `catchUnwind`, the `Result`'s a `try`'s value was read from, read
/// once, by the statement after it before anything else that statement
/// evaluates: `v` there, `if (!objectIs(getSnapshot(), renderedValue))`,
/// as React writes it. The same program: nothing is evaluated between
/// (ADR 0706, amended).
pub fn read_where_made(stmts: &mut Vec<Stmt>, name: &str) {
    let Some(at) = (stmts.iter()).position(|s| matches!(&s.kind, StmtKind::Const(n, _) if n == name)) else {
        return;
    };
    let rest = &stmts[at + 1..];
    if rest.is_empty() || mentions_in(rest, name) != 1 || declares(rest, name) {
        return;
    }
    let StmtKind::Const(_, value) = &stmts[at].kind else {
        unreachable!("found above")
    };
    let value = value.clone();
    let is_read = |e: &Expr| matches!(&e.kind, ExprKind::Var(v) if v == name);
    let Some(read) = first_of(&mut stmts[at + 1], &is_read) else {
        return;
    };
    *read = Expr {
        span: read.span,
        ..value
    };
    // What's said before the value, `// Whether it's three.`, is said
    // before what reads it now.
    let made = stmts.remove(at);
    if !made.span.is_none() && (stmts[at].span.is_none() || made.span.lo < stmts[at].span.lo) {
        stmts[at].span.lo = made.span.lo;
        stmts[at].span.hi = stmts[at].span.hi.max(made.span.hi);
    }
}

/// `body`, of a closure of `params` called with `args` where `stmt` is in
/// `whole`, the function's body, in the call's place: its parameter read
/// as what it's given, a variable or a path of one that reads the same;
/// `None` where what it declares at its top is another name of the
/// function's, or of what's `beside` it in `stmt`, or the variable given,
/// or the parameter can't be read so.
fn in_place(
    stmt: &Stmt,
    whole: &[Stmt],
    beside: &[Stmt],
    (params, args): (&[Pattern], &[Expr]),
    mut body: Vec<Stmt>,
    hold: bool,
) -> Option<Vec<Stmt>> {
    let ([Pattern::Name(param)], [arg]) = (params, args) else {
        return None;
    };
    // What it's given, read the same wherever it's read: a variable, or a
    // field of one, a thread-local's `value`.
    // Or another module's, a thread-local imported, `REPORT_GLOBAL_ERROR`,
    // which the linker names apart from what's here (ADR 0840, amended).
    // Or, of a closure called where it's made, a thread-local made where
    // it's first used, `x()` (ADR 0037, amended): held first, `const cell =
    // x();`, as the call reads it before its body runs.
    let (given, held) = match &arg.kind {
        ExprKind::Var(name) => (Some(name), false),
        ExprKind::Member(of, _)
            if arg.reads_same()
                && let ExprKind::Var(name) = &of.kind =>
        {
            (Some(name), false)
        }
        ExprKind::Symbol(_) => (None, false),
        ExprKind::Member(of, _) if arg.reads_same() && matches!(of.kind, ExprKind::Symbol(_)) => (None, false),
        ExprKind::Call(callee, called) if hold && called.is_empty() && callee.is_named() => (None, true),
        // What a body that never reads its parameter is given, of no
        // effects: nothing, `finishDrop({ value: _finish })` of a unit
        // struct's `Drop`, which reads no `self` (ADR 0608, amended).
        _ if !arg.has_effects() && mentions_in(&body, param) == 0 => (None, false),
        _ => return None,
    };
    if held {
        body.insert(0, StmtKind::Const(param.clone(), arg.clone()).at(stmt.span));
    }
    // The names beside the body in its place, which what it declares
    // mustn't be: the function's others, what's beside it in `stmt`, the
    // variable it's given, and what a held argument reads, `const x$1 =
    // x();`, not `x`.
    let names = || {
        let mut others = whole.to_vec();
        statement_lists(&mut others, &mut |stmts| stmts.retain(|s| s != stmt));
        let mut names: HashSet<String> = given.into_iter().cloned().collect();
        names_in(&others, &mut names);
        names_in(beside, &mut names);
        if held {
            arg.visit_vars(&mut |name| _ = names.insert(name.to_string()));
        }
        names
    };
    // A parameter the body sets, where a shadow of it was, `let held =
    // held.as_ref().unwrap();`, which tidy's `shadows` made `held =
    // $unwrap(held);` (ADR 0352): from that set on, a name of its own, as
    // lowering names a shadow apart, `const held$1 = $unwrap(held);`, the
    // parameter before it read as what it's given (ADR 0840, amended). Of
    // a first set the body's own statement, which no closure made before
    // it could see, else the call stays.
    if sets(&body, param) {
        let first = body.iter().position(|s| sets(std::slice::from_ref(s), param))?;
        let StmtKind::Assign(target, value) = &body[first].kind else {
            return None;
        };
        let mut seen_before = false;
        each_expr(&body[..first], &mut |e| match &e.kind {
            ExprKind::Arrow(_, inner) | ExprKind::AsyncArrow(_, inner) => seen_before |= mentions_in(inner, param) > 0,
            ExprKind::Function(function) => seen_before |= mentions_in(&function.body, param) > 0,
            _ => {}
        });
        if !matches!(&target.kind, ExprKind::Var(v) if v == param) || seen_before {
            return None;
        }
        let value = value.clone();
        let mut mentioned = names();
        names_in(&body, &mut mentioned);
        // Apart from what the parameter was given apart from, `held` of
        // `held$1`, as lowering names each (ADR 0352).
        let root = crate::names::apart(param).map_or(param.as_str(), |(root, _)| root);
        let to = (1..)
            .map(|k| crate::names::given_apart(root, k))
            .find(|to| !mentioned.contains(to))
            .expect("a name nothing mentions");
        let mut after = body.split_off(first + 1);
        renamed(&mut after, param, &to);
        let again = sets(&after, &to);
        body[first].kind = match again {
            true => StmtKind::Let(to, Some(value)),
            false => StmtKind::Const(to, value),
        };
        body.extend(after);
    }
    // What the body declares at its top is declared beside `stmt`: one that
    // is another name of the function, or the variable it's given, is
    // named apart from them, `borrow$1`, as lowering names a body's own
    // (ADR 0352), and reclaimed where JS's scopes let it be (ADR 0357): a
    // `Drop`'s body, a function of its own, names its temporaries as the
    // function it's dropped in does (ADR 0589, amended).
    let mut declared = Vec::new();
    for s in &body {
        match &s.kind {
            StmtKind::Const(name, _) | StmtKind::Let(name, _) => declared.push(name.clone()),
            StmtKind::Destructure { pattern, .. } => declared.extend(pattern.names().into_iter().map(str::to_string)),
            StmtKind::Function(function) => declared.push(function.name.clone()),
            _ => {}
        }
    }
    if !declared.is_empty() {
        let names = names();
        let mut mentioned = names.clone();
        names_in(&body, &mut mentioned);
        for from in declared.into_iter().filter(|name| names.contains(name)) {
            let to = (1..)
                .map(|k| crate::names::given_apart(&from, k))
                .find(|to| !mentioned.contains(to))
                .expect("a name nothing mentions");
            renamed(&mut body, &from, &to);
            mentioned.insert(to);
        }
    }
    // Read as what it's given: a parameter never set, now, of a body with
    // no closure of its own whose names could shadow it.
    // Each read where it was in the body, not where the call is: a comment
    // above a thread-local's `with` is above the body's first statement,
    // not inside a loop's test that reads it (rd355).
    let mut read = arg.clone();
    read.each_mut(&mut |e| e.span = Span::NONE);
    if !held
        && !matches!(&arg.kind, ExprKind::Var(v) if v == param)
        && !substituted(&mut body, &|v| (v == param).then(|| read.clone()))
    {
        return None;
    }
    Some(body)
}

/// Whether `stmts` set `name`, `name = ..` or `name += ..`, in any block,
/// a closure's too.
fn sets(stmts: &[Stmt], name: &str) -> bool {
    let mut set = false;
    each_list(stmts, &mut |list| {
        set |= list.iter().any(|s| {
            matches!(&s.kind, StmtKind::Assign(t, _) | StmtKind::Compound(_, t, _)
                if matches!(&t.kind, ExprKind::Var(v) if v == name))
        });
    });
    each_expr(stmts, &mut |e| match &e.kind {
        ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => set |= sets(body, name),
        ExprKind::Function(function) => set |= sets(&function.body, name),
        _ => {}
    });
    set
}

/// What a `try` holds, `tried`, a block of its own, with a closure of
/// nothing called where it's made, `catch_unwind(|| ..)`'s (ADR 0660),
/// as its body: `return (() => { .. })();` its body, each `return` of it
/// the function's, as returning what it returns is, where it never ends
/// without one, as a closure of a value doesn't; and `(() => { .. })();`
/// first, its body where it never returns, or returns only at its end,
/// whose value it's given to nothing: `return step(n);` is `step(n);`
/// (ADR 0706, amended). Not of an async closure, whose value is a promise.
pub fn applied_tried(mut tried: Vec<Stmt>) -> Vec<Stmt> {
    let body_of = |value: &Expr| called_body(value).map(<[Stmt]>::to_vec);
    match tried.first().map(|s| &s.kind) {
        Some(StmtKind::Return(Some(value))) if tried.len() == 1 => {
            match body_of(value).filter(|body| body.last().is_some_and(ends)) {
                Some(body) => body,
                None => tried,
            }
        }
        // What its body declares is named apart from the function's other
        // names, as lowering names each, until reclaimed (ADR 0352).
        Some(StmtKind::Expr(value)) => match body_of(value) {
            Some(body) if !returns(&body).0 => {
                tried.splice(0..1, body);
                tried
            }
            Some(mut body)
                if body
                    .split_last()
                    .is_some_and(|(last, before)| !returns(before).0 && matches!(last.kind, StmtKind::Return(_))) =>
            {
                let last = body.pop().expect("a last statement");
                if let StmtKind::Return(Some(value)) = last.kind
                    && value.has_effects()
                {
                    body.push(StmtKind::Expr(value).at(last.span));
                }
                tried.splice(0..1, body);
                tried
            }
            _ => tried,
        },
        // Its value the `try` keeps, `const value = (() => { ..; return v;
        // })();`, of a body that returns only at its end: its body, then
        // `const value = v;`, as what follows reads it, what the body
        // declares named apart from the rest as above (ADR 0660, amended).
        // So is a variable set, `result = (() => { ..; return v; })();`,
        // `result = v;`, React's createChildReconciler's (rd551).
        Some(
            StmtKind::Const(_, value)
            | StmtKind::Assign(
                Expr {
                    kind: ExprKind::Var(_), ..
                },
                value,
            ),
        ) => match body_of(value) {
            Some(mut body)
                if body.split_last().is_some_and(|(last, before)| {
                    !returns(before).0 && matches!(last.kind, StmtKind::Return(Some(_)))
                }) =>
            {
                let last = body.pop().expect("a last statement");
                let StmtKind::Return(Some(value)) = last.kind else {
                    unreachable!("matched above")
                };
                let kept = match &tried[0].kind {
                    StmtKind::Const(name, _) => StmtKind::Const(name.clone(), value),
                    StmtKind::Assign(to, _) => StmtKind::Assign(to.clone(), value),
                    _ => unreachable!("matched above"),
                };
                // Where the `return` was, after what's said before it.
                let span = if last.span.is_none() { tried[0].span } else { last.span };
                body.push(kept.at(span));
                tried.splice(0..1, body);
                tried
            }
            // A body that returns early, a guard's `if (n === 0) { return
            // 0; }`: each `return v;` sets the value, `found = 0;`, what
            // follows the guard its `else`, as a person writes it, where
            // each `return` ends what runs of it (rd60).
            Some(body) => {
                let (declared, to) = match &tried[0].kind {
                    StmtKind::Const(name, _) => (Some(name.clone()), Expr::var(name)),
                    StmtKind::Assign(to, _) => (None, to.clone()),
                    _ => unreachable!("matched above"),
                };
                // Not where the body declares the variable's name, which
                // its blocks would set in its place.
                let shadowed = matches!(&to.kind, ExprKind::Var(name) if declares(&body, name));
                let set = |value: &Expr, at: Span| Some(vec![StmtKind::Assign(to.clone(), value.clone()).at(at)]);
                let Some(set) = returns_replaced(&body, &set).filter(|_| !shadowed) else {
                    return tried;
                };
                let span = tried[0].span;
                let declared = declared.map(|name| StmtKind::Let(name, None).at(span));
                tried.splice(0..1, declared.into_iter().chain(set));
                tried
            }
            _ => tried,
        },
        _ => tried,
    }
}

/// The body of `e`, where it's a closure of nothing called where it's
/// made, `(() => { .. })()`.
pub fn called_body(e: &Expr) -> Option<&[Stmt]> {
    match &e.kind {
        ExprKind::Call(callee, args) if args.is_empty() => match &callee.kind {
            ExprKind::Arrow(params, body) if params.is_empty() => Some(body),
            _ => None,
        },
        _ => None,
    }
}

/// `body`, a closure's in a block of its own, each `return v;` of it what
/// `replace` makes of `v`, and of where the `return` is: the value set,
/// `to = v;` (rd60), or what a `try`'s `Ok` arm does of it, `return 0;`
/// (ADR 0706, amended). What it makes that goes on is where the `return`
/// ends what runs of the body: last, or last of an `if`'s branch, what
/// follows that `if` its `else`, of one branch at most, so nothing's
/// written twice; what ends, `return 0;`, is where the `return` is, what
/// follows the `if` after it; in a loop or a `switch`, only what returns
/// from the function, `return;`, as the `return` did the closure
/// (`returned_in`). `None` where a `return` is in a `try`, or in a loop or
/// a `switch` where what it's made doesn't return, `replace` makes nothing
/// of one, or a way through the body ends with no value.
pub fn returns_replaced(body: &[Stmt], replace: &dyn Fn(&Expr, Span) -> Option<Vec<Stmt>>) -> Option<Vec<Stmt>> {
    // Of no `return`, it ends otherwise, a `throw`.
    if !body.last().is_some_and(ends) {
        return None;
    }
    replaced(body, true, replace)
}

/// `stmts`, each `return v;` in them `replace`'s, where they're `last` of
/// the body, so what goes on after them is what follows the body, or they
/// go on to what follows them in it (`returns_replaced`).
fn replaced(stmts: &[Stmt], last: bool, replace: &dyn Fn(&Expr, Span) -> Option<Vec<Stmt>>) -> Option<Vec<Stmt>> {
    let mut out = Vec::with_capacity(stmts.len());
    for (at, stmt) in stmts.iter().enumerate() {
        let rest = &stmts[at + 1..];
        if !returns(std::slice::from_ref(stmt)).0 {
            out.push(stmt.clone());
            continue;
        }
        match &stmt.kind {
            StmtKind::Return(Some(value)) if rest.is_empty() => {
                let made = replace(value, stmt.span)?;
                if !last && !made.last().is_some_and(ends) {
                    return None;
                }
                out.extend(made);
                return Some(out);
            }
            StmtKind::If(test, then, otherwise) => {
                // Each branch in place, where what goes on of it goes on
                // to what follows the `if` as it did.
                let in_place = |branch: &[Stmt]| replaced(branch, last && rest.is_empty(), replace);
                if let Some(yes) = in_place(then)
                    && let Some(no) = otherwise.as_deref().map(in_place).unwrap_or(Some(Vec::new()))
                {
                    let no = otherwise.is_some().then_some(no);
                    out.push(StmtKind::If(test.clone(), yes, no).at(stmt.span));
                    continue;
                }
                // Else what follows is the `else` of the branch that goes
                // on, where they're last.
                let otherwise = otherwise.as_deref().unwrap_or_default();
                let goes_on = |branch: &[Stmt]| !branch.last().is_some_and(ends);
                if !last || (!rest.is_empty() && goes_on(then) && goes_on(otherwise)) {
                    return None;
                }
                let branch = |branch: &[Stmt]| match goes_on(branch) {
                    true => replaced(&[branch, rest].concat(), true, replace),
                    false => replaced(branch, true, replace),
                };
                let (yes, no) = (branch(then)?, branch(otherwise)?);
                let nothing = |stmts: &[Stmt]| stmts.iter().all(|s| matches!(s.kind, StmtKind::Comments));
                let made = match (nothing(&yes), nothing(&no)) {
                    (true, false) => StmtKind::If(test.clone().negated_test(), no, None),
                    (_, true) => StmtKind::If(test.clone(), yes, None),
                    (false, false) => StmtKind::If(test.clone(), yes, Some(no)),
                };
                out.push(made.at(stmt.span));
                return Some(out);
            }
            // In a loop or a `switch`, where what each `return` is made
            // returns from the function, as the closure's `return` did.
            _ => out.push(returned_in(stmt, replace)?),
        }
    }
    Some(out)
}

/// `stmt`, a loop or a `switch`, or what's in one, each `return v;` in it
/// what `replace` makes of `v`, where that returns from the function as
/// the `return` did the closure: it ends, and nothing it makes is a
/// `break` or a `continue`, which would be of the loop. `None` of a `try`,
/// whose `finally` would run after the arm, not before it.
fn returned_in(stmt: &Stmt, replace: &dyn Fn(&Expr, Span) -> Option<Vec<Stmt>>) -> Option<Stmt> {
    let list = |stmts: &[Stmt]| -> Option<Vec<Stmt>> {
        let mut out = Vec::with_capacity(stmts.len());
        for s in stmts {
            match &s.kind {
                StmtKind::Return(Some(value)) => {
                    let made = replace(value, s.span)?;
                    let mut jumps = false;
                    let jump = |s: &Stmt| matches!(s.kind, StmtKind::Break(_) | StmtKind::Continue(_));
                    each_list(&made, &mut |l| jumps |= l.iter().any(jump));
                    if jumps || !made.last().is_some_and(ends) {
                        return None;
                    }
                    out.extend(made);
                }
                _ if returns(std::slice::from_ref(s)).0 => out.push(returned_in(s, replace)?),
                _ => out.push(s.clone()),
            }
        }
        Some(out)
    };
    let mut out = stmt.clone();
    match &mut out.kind {
        StmtKind::If(_, then, otherwise) => {
            *then = list(then)?;
            if let Some(otherwise) = otherwise {
                *otherwise = list(otherwise)?;
            }
        }
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Labeled(_, body) => *body = list(body)?,
        StmtKind::Switch { cases, default, .. } => {
            for case in cases {
                case.body = list(&case.body)?;
            }
            if let Some(default) = default {
                *default = list(default)?;
            }
        }
        _ => return None,
    }
    Some(out)
}

/// Of `body`, each `try` whose closure's value `applied_tried` gave a
/// variable declared just before it, `let didFatal; try { const didFatal$1 =
/// throwException(..); ..; didFatal = didFatal$1; } catch ..`, that sets
/// the variable where the value is made, `didFatal = throwException(..);
/// ..`, as React's throwAndUnwindWorkLoop writes it (rd534, ADR 0706
/// amended). Only where that's the same program: what follows the value
/// in the `try` never names the variable, so reads none of it set early,
/// and reads the value in no closure, which may run once the variable
/// changes; and where what follows may throw, the variable set early is
/// never seen: the `catch` never names it and never goes on, leaving the
/// block the variable is of, or sets it before anything names it; and no
/// function the block declares, which runs where it's called, names it.
pub fn tried_set_where_made(body: &mut [Stmt]) {
    for i in 1..body.len() {
        let declared = body[..i]
            .iter()
            .rev()
            .take_while(|s| matches!(s.kind, StmtKind::Let(_, None)))
            .count();
        let lets = &body[i - declared.max(1)..i];
        let declared: Vec<String> = (lets.iter())
            .filter_map(|s| match &s.kind {
                StmtKind::Let(name, _) => Some(name.clone()),
                _ => None,
            })
            .collect();
        // A value its arm binds is a copy too, `const found = found$1;
        // done = found;`, set where it's made in turn.
        while let StmtKind::TryCatch(tried, binding, caught) = &body[i].kind {
            let Some((made, set, variable)) = set_where_made(tried, binding.as_deref(), caught, |name| {
                declared.iter().any(|n| n == name)
            }) else {
                break;
            };
            if body
                .iter()
                .any(|s| matches!(&s.kind, StmtKind::Function(f) if mentions_in(&f.body, &variable) > 0))
            {
                break;
            }
            set_at(&mut body[i], made, set, &variable);
        }
    }
}

/// `try`'s `v = value;` at `set` gone, `variable` set where the `const` at
/// `made` makes the value.
fn set_at(try_catch: &mut Stmt, made: usize, set: usize, variable: &str) {
    let StmtKind::TryCatch(tried, ..) = &mut try_catch.kind else {
        unreachable!("a try")
    };
    // A comment before the value, `// What it found.` above the closure's
    // `did_fatal`, is where it was, after what follows.
    let span = tried[set].span;
    let StmtKind::Assign(to, _) = std::mem::replace(&mut tried[set].kind, StmtKind::Comments) else {
        unreachable!("found by set_where_made")
    };
    if span.is_none() {
        tried.remove(set);
    }
    let StmtKind::Const(value_name, value) = &tried[made].kind else {
        unreachable!("found by set_where_made")
    };
    let value_name = value_name.clone();
    tried[made].kind = StmtKind::Assign(to, value.clone());
    renamed(&mut tried[made + 1..set], &value_name, variable);
}

/// Of a `try`'s `tried` and its `catch`, the `const` of the value made, the
/// variable's `v = value;` and the variable, a `declared` one, where setting
/// it where the value is made is the same program (`tried_set_where_made`).
fn set_where_made(
    tried: &[Stmt],
    binding: Option<&str>,
    caught: &[Stmt],
    declared: impl Fn(&str) -> bool,
) -> Option<(usize, usize, String)> {
    tried.iter().enumerate().find_map(|(set, stmt)| {
        let StmtKind::Assign(to, from) = &stmt.kind else {
            return None;
        };
        let (ExprKind::Var(variable), ExprKind::Var(value)) = (&to.kind, &from.kind) else {
            return None;
        };
        let made = tried[..set]
            .iter()
            .position(|s| matches!(&s.kind, StmtKind::Const(n, _) if n == value))?;
        let between = &tried[made + 1..set];
        // The `catch` leaves the variable's block, naming it nowhere, or
        // sets it before it names it.
        let unseen = |caught: &[Stmt]| {
            let leaves = caught.last().is_some_and(ends) && mentions_in(caught, variable) == 0;
            let first_set = caught.iter().position(|s| {
                matches!(&s.kind, StmtKind::Assign(t, v)
                    if matches!(&t.kind, ExprKind::Var(n) if n == variable) && !v.mentions_var(variable))
            });
            leaves || first_set.is_some_and(|first| mentions_in(&caught[..first], variable) == 0)
        };
        let same = declared(variable)
            && mentions_in(&tried[..set], variable) == 0
            && !declares(between, variable)
            && mentions_in(&tried[set + 1..], value) == 0
            && !read_in_a_closure(between, value)
            && (between.iter().all(Stmt::cannot_throw) || binding != Some(variable.as_str()) && unseen(caught));
        same.then(|| (made, set, variable.clone()))
    })
}

/// Whether a closure or function in `stmts` names `name`.
fn read_in_a_closure(stmts: &[Stmt], name: &str) -> bool {
    let mut read = false;
    each_expr(stmts, &mut |e| match &e.kind {
        ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => read |= mentions_in(body, name) > 0,
        ExprKind::Function(function) => read |= mentions_in(&function.body, name) > 0,
        _ => {}
    });
    each_list(stmts, &mut |list| {
        read |= (list.iter()).any(|s| matches!(&s.kind, StmtKind::Function(f) if mentions_in(&f.body, name) > 0));
    });
    read
}

/// Whether `stmts` return, but in a closure of their own, and whether a
/// `return` gives a value.
pub fn returns(stmts: &[Stmt]) -> (bool, bool) {
    let mut found = (false, false);
    for s in stmts {
        let (any, valued) = match &s.kind {
            StmtKind::Return(value) => (true, value.is_some()),
            StmtKind::If(_, a, b) => {
                let (x, y) = returns(a);
                let (z, w) = b.as_deref().map(returns).unwrap_or_default();
                (x || z, y || w)
            }
            StmtKind::While { body, .. }
            | StmtKind::DoWhile { body, .. }
            | StmtKind::ForOf { body, .. }
            | StmtKind::For { body, .. }
            | StmtKind::Labeled(_, body) => returns(body),
            StmtKind::Switch { cases, default, .. } => (case_lists(cases, default).map(|body| returns(body)))
                .fold((false, false), |(x, y), (z, w)| (x || z, y || w)),
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                let (x, y) = returns(a);
                let (z, w) = returns(b);
                (x || z, y || w)
            }
            _ => (false, false),
        };
        found = (found.0 || any, found.1 || valued);
    }
    found
}

/// `stmts` with each variable `with` names read as its expression, each of
/// their own expressions once; whether each could be, none with a closure
/// whose names could shadow one.
fn substituted(stmts: &mut [Stmt], with: &dyn Fn(&str) -> Option<Expr>) -> bool {
    let one = |e: &mut Expr| match e.substitute(with) {
        Some(read) => {
            *e = read;
            true
        }
        None => false,
    };
    stmts.iter_mut().all(|stmt| match &mut stmt.kind {
        StmtKind::Const(_, e)
        | StmtKind::Let(_, Some(e))
        | StmtKind::Destructure { value: e, .. }
        | StmtKind::Expr(e)
        | StmtKind::Return(Some(e))
        | StmtKind::Throw(e) => one(e),
        StmtKind::Assign(a, b) | StmtKind::Compound(_, a, b) => one(a) && one(b),
        StmtKind::If(test, a, b) => {
            one(test) && substituted(a, with) && b.as_deref_mut().is_none_or(|b| substituted(b, with))
        }
        StmtKind::While { cond, body, .. } | StmtKind::DoWhile { cond, body, .. } => {
            one(cond) && substituted(body, with)
        }
        StmtKind::ForOf { iterable, body, .. } => one(iterable) && substituted(body, with),
        StmtKind::For {
            start,
            test,
            step,
            body,
            ..
        } => one(start) && one(test) && substituted(body, with) && step.value_mut().is_none_or(one),
        StmtKind::Labeled(_, body) => substituted(body, with),
        StmtKind::Switch {
            discriminant,
            cases,
            default,
        } => {
            one(discriminant)
                && cases
                    .iter_mut()
                    .all(|Case { values, body, .. }| values.iter_mut().all(one) && substituted(body, with))
                && default.as_deref_mut().is_none_or(|default| substituted(default, with))
        }
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => substituted(a, with) && substituted(b, with),
        StmtKind::Let(_, None)
        | StmtKind::Break(_)
        | StmtKind::Continue(_)
        | StmtKind::Return(None)
        | StmtKind::Directive(_)
        | StmtKind::Comments => true,
        StmtKind::Function(_) => false,
    })
}

/// A closure's body that ends with a loop, each `return;` in it in that
/// loop and in no other: each that loop's `break;`, as leaving the loop
/// ends the body. Whether it is.
fn returns_broken(body: &mut [Stmt]) -> bool {
    let Some((last, before)) = body.split_last_mut() else {
        return false;
    };
    if returns(before).0 {
        return false;
    }
    let (StmtKind::While {
        label: None,
        body: inner,
        ..
    }
    | StmtKind::ForOf {
        label: None,
        body: inner,
        ..
    }
    | StmtKind::For {
        label: None,
        body: inner,
        ..
    }) = &mut last.kind
    else {
        return false;
    };
    returns_as_breaks(inner)
}

/// Each `return;` of `stmts` a `break;`, where none is in an inner loop or
/// `switch`, whose `break` would be its own. Whether each is.
fn returns_as_breaks(stmts: &mut [Stmt]) -> bool {
    stmts.iter_mut().all(|stmt| match &mut stmt.kind {
        StmtKind::Return(None) => {
            stmt.kind = StmtKind::Break(None);
            true
        }
        StmtKind::If(_, a, b) => returns_as_breaks(a) && b.as_deref_mut().is_none_or(returns_as_breaks),
        StmtKind::Labeled(_, a) => returns_as_breaks(a),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => returns_as_breaks(a) && returns_as_breaks(b),
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. } => !returns(body).0,
        // Whose `break` would leave it.
        StmtKind::Switch { cases, default, .. } => !case_lists(cases, default).any(|body| returns(body).0),
        _ => true,
    })
}
