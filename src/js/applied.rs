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
    each_expr_mut(body, &mut |e| match &mut e.kind {
        ExprKind::Arrow(_, inner) | ExprKind::AsyncArrow(_, inner) => applied_in(inner, true),
        ExprKind::Function(function) => applied_in(&mut function.body, true),
        _ => {}
    });
}

/// `applied_blocks`' of `body` itself, not the closures it makes.
fn applied_in(body: &mut Vec<Stmt>, ends_function: bool) {
    loop {
        let whole = body.clone();
        if ends_function
            && let Some(last) = body.len().checked_sub(1)
            && let Some(inlined) = block_of(&body[last], &whole, true, true)
        {
            body.splice(last..=last, inlined);
            continue;
        }
        let mut done = false;
        statement_lists(body, &mut |stmts| {
            if done {
                return;
            }
            let inlined = |stmt: &Stmt| block_of(stmt, &whole, false, true).or_else(|| value_of(stmt, &whole));
            if let Some((i, inlined)) = (0..stmts.len()).find_map(|i| Some((i, inlined(&stmts[i])?))) {
                stmts.splice(i..=i, inlined);
                done = true;
            }
        });
        if !done {
            break;
        }
    }
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
    block_of(&call, whole, false, false)
}

/// The statements `stmt`, a call of a closure made there, is, in `whole`,
/// the function's body; `tail`, the body's last, which a `return;` ends too;
/// `hold`, of a closure the program made, which may be given a lazy
/// thread-local's call (`in_place`).
fn block_of(stmt: &Stmt, whole: &[Stmt], tail: bool, hold: bool) -> Option<Vec<Stmt>> {
    let StmtKind::Expr(e) = &stmt.kind else { return None };
    let ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    let ExprKind::Arrow(params, body) = &callee.kind else {
        return None;
    };
    // The closure's `return`s are its own, the function's only at its end,
    // and of no value, which nothing read; elsewhere, each in the loop its
    // body ends with, that loop's `break`.
    let (any, valued) = returns(body);
    let mut body = body.clone();
    if valued || any && !tail && !returns_broken(&mut body) {
        return None;
    }
    in_place(stmt, whole, &[], (params, args), body, hold)
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
    // One that only throws at its end, returning nowhere, `throw exception;`
    // of a thread-local's `with(|e| js::throw(e))`: its body is the
    // statement, as nothing of the statement runs after the call (ADR 0840,
    // amended).
    let throws = matches!(last.kind, StmtKind::Throw(_)) && !returns(body).0;
    if !throws && (!matches!(last.kind, StmtKind::Return(Some(_))) || returns(before).0) {
        return None;
    }
    // What else the statement reads or declares, which what the body
    // declares beside it mustn't be either: `const x` of `const x = v`.
    let mut beside = stmt.clone();
    if let Some(call) = first_call(&mut beside) {
        *call = Expr::undefined();
    }
    let mut inlined = in_place(stmt, whole, &[beside], (params, args), body.clone(), true)?;
    if throws {
        return Some(inlined);
    }
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

/// The call of a closure made where it's called that `e` evaluates before
/// anything else of it: itself, or what an operator, a test, or a
/// property's read evaluates first.
fn evaluated_first(e: &mut Expr) -> Option<&mut Expr> {
    if matches!(&e.kind, ExprKind::Call(callee, _) if matches!(callee.kind, ExprKind::Arrow(..))) {
        return Some(e);
    }
    match &mut e.kind {
        ExprKind::Unary(op, a) if !op.writes() => evaluated_first(a),
        ExprKind::Binary(_, a, _) | ExprKind::Cond(a, ..) | ExprKind::Member(a, _) | ExprKind::Index(a, _) => {
            evaluated_first(a)
        }
        _ => None,
    }
}

/// The call `value_of` takes of `stmt`, where it's one.
fn first_call(stmt: &mut Stmt) -> Option<&mut Expr> {
    match &mut stmt.kind {
        StmtKind::Expr(e)
        | StmtKind::Const(_, e)
        | StmtKind::Let(_, Some(e))
        | StmtKind::Return(Some(e))
        | StmtKind::Throw(e)
        | StmtKind::If(e, ..)
        | StmtKind::Switch { discriminant: e, .. } => evaluated_first(e),
        // A variable's, which nothing's read of before its value.
        StmtKind::Assign(target, e) if matches!(target.kind, ExprKind::Var(_)) => evaluated_first(e),
        _ => None,
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
        ExprKind::Call(callee, called)
            if hold && called.is_empty() && matches!(callee.kind, ExprKind::Var(_) | ExprKind::Symbol(_)) =>
        {
            (None, true)
        }
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
    if !held
        && !matches!(&arg.kind, ExprKind::Var(v) if v == param)
        && !substituted(&mut body, &|v| (v == param).then(|| arg.clone()))
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
    let body_of = |value: &Expr| match &value.kind {
        ExprKind::Call(callee, args) if args.is_empty() => match &callee.kind {
            ExprKind::Arrow(params, body) if params.is_empty() => Some(body.clone()),
            _ => None,
        },
        _ => None,
    };
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
        Some(StmtKind::Const(name, value)) => match body_of(value) {
            Some(mut body)
                if body.split_last().is_some_and(|(last, before)| {
                    !returns(before).0 && matches!(last.kind, StmtKind::Return(Some(_)))
                }) =>
            {
                let last = body.pop().expect("a last statement");
                let StmtKind::Return(Some(value)) = last.kind else {
                    unreachable!("matched above")
                };
                body.push(StmtKind::Const(name.clone(), value).at(tried[0].span));
                tried.splice(0..1, body);
                tried
            }
            _ => tried,
        },
        _ => tried,
    }
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
        StmtKind::While { cond, body, .. } => one(cond) && substituted(body, with),
        StmtKind::ForOf { iterable, body, .. } => one(iterable) && substituted(body, with),
        StmtKind::For { start, test, body, .. } => one(start) && one(test) && substituted(body, with),
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
    // Each `return;` outside any inner loop, whose `break` would be its own.
    fn broken(stmts: &mut [Stmt]) -> bool {
        stmts.iter_mut().all(|stmt| match &mut stmt.kind {
            StmtKind::Return(None) => {
                stmt.kind = StmtKind::Break(None);
                true
            }
            StmtKind::If(_, a, b) => broken(a) && b.as_deref_mut().is_none_or(broken),
            StmtKind::Labeled(_, a) => broken(a),
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => broken(a) && broken(b),
            StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
                !returns(body).0
            }
            // Whose `break` would leave it.
            StmtKind::Switch { cases, default, .. } => !case_lists(cases, default).any(|body| returns(body).0),
            _ => true,
        })
    }
    broken(inner)
}
