//! A `match` of a `Result` of what may throw as JS's `try` and `catch`: a
//! JS call's (ADRs 0035, 0637) or `catch_unwind`'s (ADR 0660), and what
//! the `catch` reads of what was thrown.

use std::collections::HashSet;

use crate::js::{self, Expr, ExprKind, Pattern, Stmt, StmtKind};

/// What a `catch` of an unwind names what it caught, where it reads the
/// payload `$unwound` makes of it: `thrown`, or a name given apart,
/// `thrown$1`, where the `catch`'s `payload` is `thrown` or its `handler`
/// reads or declares one, React's renderNode's `let thrown;` it sets
/// (rd551): a name of the program's own never keeps it a `$catchUnwind`.
fn thrown_name(payload: &str, handler: &[Stmt]) -> String {
    let mut taken = HashSet::from([payload.to_string()]);
    js::names_in(handler, &mut taken);
    crate::names::fresh_apart(&mut taken, "thrown")
}

/// Of `body`, which `ends` a function or doesn't: where it ends, the
/// function returns.
pub(super) fn try_catches(body: &mut Vec<Stmt>, ends: bool) {
    unwound_lets(body);
    let mut i = 0;
    while i < body.len() {
        // One tested after what the `if` asks first, `if (c &&
        // $catchUnwind(f).TAG === "Err") { .. }`, is the `try` inside the
        // `if` of what's first (`asked_first`).
        if let Some(asked) = asked_first(&body[i], &body[..i]) {
            body[i] = asked;
        }
        // A `Result` tested where it's made, `if ($catchUnwind(f).TAG ===
        // "Err" && c)`, is one made first, then tested (ADR 0706, amended).
        if let Some((made, tested)) = made_where_tested(&body[i])
            && let Some((caught, taken)) = chained_try_catch(&made, &tested, &body[..i], &body[i + 1..], ends)
        {
            body.splice(i..i + 1 + taken, caught);
        }
        if i + 1 >= body.len() {
            break;
        }
        // A flag out of `catch_unwind` to return from the function, the
        // `try` the closure's body is (`flagged_try`), where nothing after
        // reads it.
        if let Some(flagged) = flagged_try(&body[i], &body[i + 1])
            && let StmtKind::Const(result, _) = &body[i].kind
            && js::mentions_in(&body[i + 2..], result) == 0
        {
            body.splice(i..i + 2, [flagged]);
            i += 1;
            continue;
        }
        // Past what's declared for it to set, `let text;`, as MIR's lowering
        // declares a variable where the program does.
        let declared = (body[i + 1..].iter())
            .take_while(|s| matches!(s.kind, StmtKind::Let(_, None)))
            .count();
        let at = i + 1 + declared;
        if at < body.len()
            && let Some((caught, taken)) = chained_try_catch(&body[i], &body[at], &body[..i], &body[at + 1..], ends)
        {
            body.splice(at..at + 1 + taken, caught);
            body.remove(i);
        }
        i += 1;
    }
    ignored_tries(body);
}

/// Of `body`, after the `match`es whose arms it takes in, each `$try`
/// whose result is ignored, or whose throwing is returned.
fn ignored_tries(body: &mut [Stmt]) {
    // `$try(() => f())` whose result is ignored, `let _ = f();`, is `try {
    // f(); } catch {}`: the call, what it threw dropped.
    for stmt in body.iter_mut() {
        if let StmtKind::Expr(e) = &stmt.kind
            && let Some((call, _)) = e.caught()
        {
            let tried = vec![StmtKind::Expr(call).at(stmt.span)];
            stmt.kind = StmtKind::TryCatch(js::applied_tried(tried), None, Vec::new());
        }
        // Whether it threw, returned, `match f() { Ok(_) => false, Err(_)
        // => true }` or `f().is_err()`, is `try { f(); return false; }
        // catch { return true; }` (ADR 0639).
        if let StmtKind::Return(Some(e)) = &stmt.kind
            && let Some((call, of_throw)) = threw(e)
        {
            let span = stmt.span;
            let tried = vec![
                StmtKind::Expr(call).at(span),
                StmtKind::Return(Some(Expr::bool(!of_throw))).at(span),
            ];
            let caught = vec![StmtKind::Return(Some(Expr::bool(of_throw))).at(span)];
            stmt.kind = StmtKind::TryCatch(js::applied_tried(tried), None, caught);
        }
    }
}

/// `const r = $catchUnwind(() => { ..; if (c) { ..; return true; } return
/// false; }); if (r.TAG === "Ok" && r._0) { return v; }`, a flag a closure
/// gives out of `catch_unwind` to return from its function, which a closure
/// can't: the `try` its body is, each `return` of the flag the `if`'s
/// `return v;`, its last, of the other, gone, `try { ..; if (c) { ..; return
/// v; } } catch {}`, as JS returns from inside a `try`, React's
/// dispatchSetStateInternal (ADR 0660, amended): the `if`'s arm where the
/// closure returns its value (`arms_where_returned`), so only where its
/// `return` can't throw, which the `catch` would catch; where each other
/// `return` of the body gives the flag; and of `!r._0`, the other way.
fn flagged_try(made: &Stmt, tested: &Stmt) -> Option<Stmt> {
    let StmtKind::Const(result, value) = &made.kind else {
        return None;
    };
    let ExprKind::Call(callee, args) = &value.kind else {
        return None;
    };
    let (Some("$catchUnwind"), [f]) = (callee.helper_name(), args.as_slice()) else {
        return None;
    };
    let ExprKind::Arrow(params, body) = &f.kind else {
        return None;
    };
    let StmtKind::If(test, then, None) = &tested.kind else {
        return None;
    };
    let (head, Some(flag)) = split_chain(test) else {
        return None;
    };
    let (of, ok, eq) = js::result::tested(&head)?;
    if !params.is_empty() || ok != eq || !matches!(&of.kind, ExprKind::Var(v) if v == result) {
        return None;
    }
    let (read, when) = match &flag.kind {
        ExprKind::Unary(js::UnaryOp::Not, read) => (&**read, false),
        _ => (&flag, true),
    };
    if !js::result::reads_held(read, result)
        || !matches!(
            then.as_slice(),
            [Stmt {
                kind: StmtKind::Return(_),
                ..
            }]
        )
    {
        return None;
    }
    let flag = match when {
        true => Expr::var(result),
        false => Expr::unary(js::UnaryOp::Not, Expr::var(result)),
    };
    let ok = [StmtKind::If(flag, then.clone(), None).at(tested.span)];
    let (tried, _) = arms_where_returned(body, result, &ok)?;
    Some(StmtKind::TryCatch(tried, None, Vec::new()).at(tested.span))
}

/// `body`, a `catch_unwind` closure's, each `return v;` of a literal what
/// `ok`, its `Ok` arms, do where its value, `value`, is `v`, as a person
/// writes what the closure gives out to do there (ADR 0706, amended,
/// rd60): `if (didFatal) { panicOnRootError(..); return true; } return
/// false;` and `if (value) { return 0; }` are `if (didFatal) {
/// panicOnRootError(..); return 0; }`, and the `false`, which does
/// nothing, what falls out of the `try`; and what's after, the rest of
/// `ok`, which never reads the value. Only where each `if` of `ok` that
/// reads the value tests it against a literal; what each `return` is made
/// can't throw, which the `try`'s `catch` would catch, where the `Ok` arm's
/// throw is the function's; and of a value returned in more places than
/// one, is one jump, `return 0;`, not statements written twice.
fn arms_where_returned<'o>(body: &[Stmt], value: &str, ok: &'o [Stmt]) -> Option<(Vec<Stmt>, &'o [Stmt])> {
    let reads = ok
        .iter()
        .rposition(|s| js::mentions_in(std::slice::from_ref(s), value) > 0)?;
    let (tested, after) = ok.split_at(reads + 1);
    let mut returned = Vec::new();
    returned_values(body, &mut returned);
    // Where the body's last statement is written.
    let mut last = 0;
    js::each_list(body, &mut |list| {
        last = (list.iter().filter(|s| !s.span.is_none())).fold(last, |last, s| last.max(s.span.lo));
    });
    let arm = |v: &Expr, at: js::Span| {
        let mut arm = arm_of(tested, value, v)?;
        let places = returned.iter().filter(|r| same_literal(r, v) == Some(true)).count();
        let one_jump = matches!(
            arm.as_slice(),
            [Stmt {
                kind: StmtKind::Return(_) | StmtKind::Break(_) | StmtKind::Continue(_),
                ..
            }]
        );
        if !arm.iter().all(cannot_throw) || (places > 1 && !arm.is_empty() && !one_jump) {
            return None;
        }
        // What's said before the `return` is where it was, before what the
        // arm does there; and what's said in the arm, after the closure, is
        // where it's placed only at the body's last statement, as a comment
        // of the closure's written after this `return` is before what
        // follows it, not in an arm placed before that.
        if at.is_none() || at.lo < last {
            js::statement_lists(&mut arm, &mut |list| {
                list.iter_mut().for_each(|s| s.span = js::Span::NONE)
            });
            js::each_expr_mut(&mut arm, &mut |e| e.span = js::Span::NONE);
        }
        let said = (!at.is_none()).then(|| StmtKind::Comments.at(at));
        Some(said.into_iter().chain(arm).collect())
    };
    Some((js::returns_replaced(body, &arm)?, after))
}

/// Each value `stmts` return, but in a closure of their own.
fn returned_values(stmts: &[Stmt], out: &mut Vec<Expr>) {
    js::each_list(stmts, &mut |list| {
        out.extend(list.iter().filter_map(|s| match &s.kind {
            StmtKind::Return(Some(v)) => Some(v.clone()),
            _ => None,
        }));
    });
}

/// What `ok`, statements that test `value`, do where it's `v`, a literal:
/// each `if` of it its branch that runs, up to what ends; `None` where
/// `ok` reads `value` otherwise.
fn arm_of(ok: &[Stmt], value: &str, v: &Expr) -> Option<Vec<Stmt>> {
    let mut out = Vec::new();
    for stmt in ok {
        if js::mentions_in(std::slice::from_ref(stmt), value) == 0 {
            out.push(stmt.clone());
        } else {
            let StmtKind::If(test, then, otherwise) = &stmt.kind else {
                return None;
            };
            let branch = match holds(test, value, v)? {
                true => then.as_slice(),
                false => otherwise.as_deref().unwrap_or_default(),
            };
            let mut arm = arm_of(branch, value, v)?;
            // What's said before the arm, `// When it's true.` above
            // `Ok(true) =>`, is before what it does.
            if let Some(first) = arm.first_mut()
                && !stmt.span.is_none()
                && !first.span.is_none()
                && stmt.span.lo < first.span.lo
            {
                first.span.lo = stmt.span.lo;
            }
            out.extend(arm);
        }
        if out.last().is_some_and(js::ends) {
            break;
        }
    }
    Some(out)
}

/// Whether `test` holds where `value` is `v`, a literal: of `value`, `!`
/// of it, and `===` or `!==` of it and a literal; `None` otherwise.
fn holds(test: &Expr, value: &str, v: &Expr) -> Option<bool> {
    let is_value = |e: &Expr| matches!(&e.kind, ExprKind::Var(name) if name == value);
    match &test.kind {
        _ if is_value(test) => match v.kind {
            ExprKind::Bool(b) => Some(b),
            _ => None,
        },
        ExprKind::Unary(js::UnaryOp::Not, inner) => holds(inner, value, v).map(|h| !h),
        ExprKind::Binary(op @ (js::Op::Eq | js::Op::Ne), a, b) => {
            let other = match (is_value(a), is_value(b)) {
                (true, false) => b,
                (false, true) => a,
                _ => return None,
            };
            Some(same_literal(other, v)? == (*op == js::Op::Eq))
        }
        _ => None,
    }
}

/// Whether `a` and `b` are the same, where each is a literal of one kind,
/// as `===` says; `None` otherwise.
fn same_literal(a: &Expr, b: &Expr) -> Option<bool> {
    match (&a.kind, &b.kind) {
        (ExprKind::Num(x), ExprKind::Num(y)) => Some(x == y),
        (ExprKind::BigInt(x), ExprKind::BigInt(y)) => Some(x == y),
        (ExprKind::Bool(x), ExprKind::Bool(y)) => Some(x == y),
        (ExprKind::Str(x), ExprKind::Str(y)) => Some(x == y),
        _ => None,
    }
}

/// The name a `Result` tested where it's made is given,
/// `made_where_tested`'s: no JS identifier, so none of the program's, and
/// never written, as the `try` made of it reads it nowhere, or none is.
const TESTED: &str = "<tested>";

/// `if ($catchUnwind(f).TAG === "Err" && c) { .. }`, or a `?:` of it
/// returned: the `Result`, `const <tested> = $catchUnwind(f);`, and the
/// `if` of it, `if (<tested>.TAG === "Err" && c)` (ADR 0706, amended).
fn made_where_tested(stmt: &Stmt) -> Option<(Stmt, Stmt)> {
    let test = match &stmt.kind {
        StmtKind::If(test, ..) => test,
        StmtKind::Return(Some(Expr {
            kind: ExprKind::Cond(test, ..),
            ..
        })) => &**test,
        _ => return None,
    };
    let (head, rest) = split_chain(test);
    let (call, ok, eq) = js::result::tested(&head)?;
    let (call, _) = (call.clone(), call.caught()?);
    let head = js::result::is(Expr::var(TESTED), ok == eq).or_at(head.span);
    let test = match rest {
        Some(rest) => Expr::bin(js::Op::And, head, rest).or_at(test.span),
        None => head,
    };
    let mut tested = stmt.clone();
    match &mut tested.kind {
        StmtKind::If(at, ..) => *at = test,
        StmtKind::Return(Some(Expr {
            kind: ExprKind::Cond(at, ..),
            ..
        })) => **at = test,
        _ => unreachable!("matched above"),
    }
    Some((StmtKind::Const(TESTED.to_string(), call).at(stmt.span), tested))
}

/// `if (c && $catchUnwind(f).TAG === "Err" && d) { .. }`, with no `else`,
/// as `if (c) { try { .. } catch { if (d) { .. } } }`, React's
/// checkPassiveEvents (ADR 0706, amended): `c` first, the closure only where
/// it's true, as `&&` runs them; where what's first is all before the
/// `Result`'s test, and the `if` of the rest is a `try` (`chained_try_catch`).
/// `before`, what's before it in its body, which a `let` it makes mustn't
/// clash with.
fn asked_first(stmt: &Stmt, before: &[Stmt]) -> Option<Stmt> {
    let StmtKind::If(test, then, None) = &stmt.kind else {
        return None;
    };
    let mut parts = Vec::new();
    conjuncts(test, &mut parts);
    let at = (parts.iter().skip(1))
        .position(|part| js::result::tested(part).is_some_and(|(call, ..)| call.caught().is_some()))?
        + 1;
    let joined = |parts: &[&Expr]| {
        (parts.iter().skip(1)).fold((*parts[0]).clone(), |a, b| Expr::bin(js::Op::And, a, (*b).clone()))
    };
    let rest = StmtKind::If(joined(&parts[at..]), then.clone(), None).at(stmt.span);
    let (made, tested) = made_where_tested(&rest)?;
    let (caught, _) = chained_try_catch(&made, &tested, before, &[], false)?;
    Some(StmtKind::If(joined(&parts[..at]), caught, None).at(stmt.span))
}

/// Each of `test`'s `&&`s, in order.
fn conjuncts<'e>(test: &'e Expr, out: &mut Vec<&'e Expr>) {
    match &test.kind {
        ExprKind::Binary(js::Op::And, a, b) => {
            conjuncts(a, out);
            conjuncts(b, out);
        }
        _ => out.push(test),
    }
}

/// A test's first part and the rest of its `&&`s, in order: `a && b &&
/// c`'s `a`, and `b && c`.
fn split_chain(test: &Expr) -> (Expr, Option<Expr>) {
    match &test.kind {
        ExprKind::Binary(js::Op::And, a, b) => {
            let (head, rest) = split_chain(a);
            let rest = match rest {
                Some(rest) => Expr::bin(js::Op::And, rest, (**b).clone()),
                None => (**b).clone(),
            };
            (head, Some(rest))
        }
        _ => (test.clone(), None),
    }
}

/// `try_catch`, of `tested` as it is, or of a chain an `if let Err(e) =
/// catch_unwind(f)` heads (ADR 0706, amended): `if (r.TAG === "Err" && c) { .. }`
/// is the `if` of its `Err` alone, `c`'s `if` inside it, so the `catch`'s,
/// `try { .. } catch (e) { if (c) { .. } }`; its `else`, where its arm
/// leaves, after it, as what runs where the arm doesn't.
fn chained_try_catch(
    made: &Stmt,
    tested: &Stmt,
    before: &[Stmt],
    rest: &[Stmt],
    ends: bool,
) -> Option<(Vec<Stmt>, usize)> {
    if let Some(caught) = try_catch(made, tested, before, rest, ends) {
        return Some(caught);
    }
    let StmtKind::Const(result, _) = &made.kind else {
        return None;
    };
    let (tested, otherwise) = unchained(result, tested)?;
    let rest: Vec<Stmt> = otherwise.iter().chain(rest).cloned().collect();
    let (mut caught, taken) = try_catch(made, &tested, before, &rest, ends)?;
    // What it took of the `else` first, then of what followed the `if`.
    let kept = otherwise.len().saturating_sub(taken);
    caught.extend(otherwise[otherwise.len() - kept..].iter().cloned());
    Some((caught, taken.saturating_sub(otherwise.len())))
}

/// `tested`, `if (result.TAG === "Err" && c) { .. }`, as the `if` of the
/// `Err` alone, `c`'s inside it, and what its `else` runs, after it; what
/// the arm sets the payload's binding to first, `let e = result._0; e =
/// e.error;`, before `c`, which reads neither. `None` where it's no such
/// chain, or its `else` runs where its arm doesn't leave.
fn unchained(result: &str, tested: &Stmt) -> Option<(Stmt, Vec<Stmt>)> {
    let span = tested.span;
    let (test, then, otherwise) = match &tested.kind {
        StmtKind::If(test, then, otherwise) => (test, then.clone(), otherwise.clone()),
        StmtKind::Return(Some(Expr {
            kind: ExprKind::Cond(test, then, otherwise),
            ..
        })) => (
            &**test,
            vec![StmtKind::Return(Some((**then).clone())).at(span)],
            Some(vec![StmtKind::Return(Some((**otherwise).clone())).at(span)]),
        ),
        _ => return None,
    };
    let (head, Some(chained)) = split_chain(test) else {
        return None;
    };
    let (of, ok, eq) = js::result::tested(&head)?;
    let leaves = then.last().is_some_and(js::ends);
    if ok == eq || !matches!(&of.kind, ExprKind::Var(v) if v == result) || (otherwise.is_some() && !leaves) {
        return None;
    }
    let bound = match then.first().map(|s| &s.kind) {
        Some(StmtKind::Const(name, value) | StmtKind::Let(name, Some(value)))
            if js::result::reads_held(value, result) =>
        {
            Some(name.clone())
        }
        _ => None,
    };
    // What `c` reads is as it was where it's read: what's moved before it
    // sets none of it.
    let hoisted = match &bound {
        Some(name) if js::mentions_in(&[StmtKind::Expr(chained.clone()).at(span)], name) == 0 => {
            1 + (then[1..].iter())
                .take_while(|s| {
                    matches!(&s.kind, StmtKind::Assign(to, _) if matches!(&to.kind, ExprKind::Var(v) if v == name))
                        && s.only_makes_values()
                })
                .count()
        }
        Some(_) => return None,
        None => 0,
    };
    // `c`'s `if` is where `c` is written, after what's said before it.
    let at = if chained.span.is_none() { span } else { chained.span };
    let mut arm = then[..hoisted].to_vec();
    arm.push(StmtKind::If(chained, then[hoisted..].to_vec(), None).at(at));
    Some((StmtKind::If(head, arm, None).at(span), otherwise.unwrap_or_default()))
}

/// A `match` of a JS call's `Result` (ADR 0035), `made`, its `$try`, then
/// `tested`, the `if` of its `Ok` or its `Err`, and of `rest`, what follows
/// it: `try { .. } catch { .. }` of the call (ADR 0637), and how many of
/// `rest` it takes. The `Err` arm reads no error but by its binding, the
/// `catch`'s. Only the call is in the `try`, as only it is in Rust: what
/// the `Ok` arm does is too where it can't throw, `try { return f(); }`;
/// else it's after it, where the `Err` arm leaves, the `Ok`'s value a `let`
/// the `try` sets, `let v; try { v = f(); } catch { return; } g(v);`.
/// `before`, what's before it, which that `let` mustn't clash with; an
/// `Err` arm that ends what `ends` a function, `rest` empty, leaves, as the
/// function returns there, its `catch` ending in `return;`.
fn try_catch(made: &Stmt, tested: &Stmt, before: &[Stmt], rest: &[Stmt], ends: bool) -> Option<(Vec<Stmt>, usize)> {
    let StmtKind::Const(result, value) = &made.kind else {
        return None;
    };
    let (call, caught) = value.caught()?;
    // A `?:` of it returned, `unwrap_or(d)`'s, is the `if` of its arms,
    // each returned (ADR 0660).
    let arms;
    let tested = match &tested.kind {
        StmtKind::Return(Some(Expr {
            kind: ExprKind::Cond(test, then, otherwise),
            ..
        })) => {
            let returned = |e: &Expr| vec![StmtKind::Return(Some(e.clone())).at(tested.span)];
            arms = StmtKind::If((**test).clone(), returned(then), Some(returned(otherwise))).at(tested.span);
            &arms
        }
        _ => tested,
    };
    let StmtKind::If(test, then, otherwise) = &tested.kind else {
        return None;
    };
    // Whether the `if` is of the `Ok`: `TAG === "Ok"`, or `!== "Err"`.
    let (tested, ok, eq) = js::result::tested(test)?;
    if !matches!(&tested.kind, ExprKind::Var(v) if v == result) {
        return None;
    }
    let of_ok = ok == eq;
    // Each arm's statements, and how many of `rest` are one's: an `if`'s
    // and its `else`'s; or, after an arm that leaves, the other's, `rest`,
    // which leaves too, where it's the `Err`'s, the `catch`'s; or, after an
    // `Ok` arm that leaves where `rest` goes on, none, the `catch`'s, what
    // follows the `try` run where the call threw, as it ran after the `if`
    // (ADR 0637, amended).
    let leaves = |stmts: &[Stmt]| stmts.last().is_some_and(js::ends);
    let (ok, err, taken): (&[Stmt], &[Stmt], usize) = match (of_ok, otherwise) {
        (true, Some(otherwise)) => (then, otherwise, 0),
        (false, Some(otherwise)) => (otherwise, then, 0),
        (true, None) if !leaves(then) => (then, &[], 0),
        (true, None) if leaves(rest) => (then, rest, rest.len()),
        (false, None) if leaves(then) => (rest, then, rest.len()),
        (false, None) => (&[], then, 0),
        // An `Ok` arm that leaves, `Ok(()) => return`, where what follows
        // reads the `Err`: what does, and what reads what that declares,
        // the `Err` arm's, the `catch`'s, as of a `let Err(x)` (ADR 0830),
        // and what's after it after the `try`, which runs where the call
        // threw as it ran after the `match` (ADR 0637, amended).
        (true, None) if let Some(n) = caught_run(result, rest) => (then, &rest[..n], n),
        // An `Err` arm of only comments, `Err(_) => { // .. }`, whose block
        // MIR's `else` held, tidy put after the `if` (ADR 0753): the
        // `catch`'s, as commitMount's `} catch (x) { // If
        // URL.createObjectURL() errors .. }` (rd156).
        (true, None) if leaves(then) && rest.first().is_some_and(|s| matches!(s.kind, StmtKind::Comments)) => {
            (then, &rest[..1], 1)
        }
        (true, None) => (then, &[], 0),
    };
    let after = &rest[taken..];
    // Each arm's binding, `const v = result._0;`: the `Ok`'s value, and the
    // `Err`'s error, the `catch`'s.
    // `let` where the arm sets it again, a shadow that's the binding it
    // shadows (ADR 0650), `let x = result._0; x = ..`.
    let bound = |stmts: &[Stmt]| match stmts.first().map(|s| &s.kind) {
        Some(StmtKind::Const(name, value)) if js::result::reads_held(value, result) => Some((name.clone(), false)),
        Some(StmtKind::Let(name, Some(value))) if js::result::reads_held(value, result) => Some((name.clone(), true)),
        _ => None,
    };
    let (value_name, error) = (bound(ok).map(|(name, _)| name), bound(err));
    let (error, set_again) = match error {
        Some((name, set_again)) => (Some(name), set_again),
        None => (None, false),
    };
    let ok = &ok[usize::from(value_name.is_some())..];
    let err = &err[usize::from(error.is_some())..];
    // An `Ok` arm that reads its value only as `result._0`, where a pattern
    // inside it tests and binds it, `Ok(Some(f)) => return f`'s `if
    // (result._0) { const f = result._0; return f; }`: the value is the
    // `try`'s, which nothing else reads, by the pattern's name where it
    // binds the whole value, `if (f) { return f; }`, else by the result's.
    let read_in_place;
    let (value_name, ok) = match value_name {
        None if held_reads(ok, result) > 0 && held_reads(ok, result) == js::mentions_in(ok, result) => {
            let mut ok = ok.to_vec();
            js::each_expr_mut(&mut ok, &mut |e| {
                if js::result::reads_held(e, result) {
                    *e = Expr::var(result).or_at(e.span);
                }
            });
            let name = match whole_binding(&ok, result, &call) {
                Some(binding) => {
                    js::statement_lists(&mut ok, &mut |stmts| {
                        stmts.retain(|s| !matches!(&s.kind, StmtKind::Const(name, _) if *name == binding));
                    });
                    js::each_expr_mut(&mut ok, &mut |e| {
                        if matches!(&e.kind, ExprKind::Var(v) if v == result) {
                            *e = Expr::var(&binding).or_at(e.span);
                        }
                    });
                    binding
                }
                None => result.clone(),
            };
            read_in_place = ok;
            (Some(name), read_in_place.as_slice())
        }
        value_name => (value_name, ok),
    };
    // The `Ok`'s value kept as it is, `return result._0;` or `x =
    // result._0;`, or its binding's: the call's.
    let kept = |stmt: &Stmt| -> Option<StmtKind> {
        let read = |e: &Expr| match &value_name {
            Some(name) => matches!(&e.kind, ExprKind::Var(v) if v == name),
            None => js::result::reads_held(e, result),
        };
        match &stmt.kind {
            StmtKind::Return(Some(e)) if read(e) => Some(StmtKind::Return(Some(call.clone()))),
            StmtKind::Assign(to, e) if read(e) && matches!(to.kind, ExprKind::Var(_)) => {
                Some(StmtKind::Assign(to.clone(), call.clone()))
            }
            _ => None,
        }
    };
    // Where the program wrote it, of a `return` written after the call's
    // `$try`, `if let Ok(function) = js::eval(code) { //
    // eslint-disable-next-line no-eval\n return function; }`'s, so a
    // comment above it is above the call it's made of, `return (0,
    // eval)(code);` (rd525); not an assignment of the `Ok`'s value, `let
    // href = match catch_unwind(|| { // c\n parse(s) }) { Ok(h) => h, .. }`'s
    // `href = parse(s)`, which is where that value is, the call's, after
    // the comment in the closure (rd545).
    let held = match ok {
        [stmt] => kept(stmt).map(|kind| {
            let written = matches!(stmt.kind, StmtKind::Return(_))
                && !stmt.span.is_none()
                && !made.span.is_none()
                && stmt.span.lo >= made.span.hi;
            (kind, written.then_some(stmt.span))
        }),
        _ => None,
    };
    // Nothing else reads the `Result`: the `Ok` arm reads its value, where
    // that's by the result's name.
    let ok_reads = match value_name.as_deref() == Some(result.as_str()) {
        true => 0,
        false => js::mentions_in(ok, result),
    };
    let reads = ok_reads + js::mentions_in(err, result) + js::mentions_in(after, result);
    if reads != usize::from(held.is_some() && value_name.is_none()) {
        return None;
    }
    // An `Err` arm that ends the function returns, where the `Ok` arm, after
    // the `try`, would run next (ADR 0637, amended).
    let returned;
    let err = match ends && after.is_empty() && !leaves(err) && held.is_none() && !ok.iter().all(cannot_throw) {
        true => {
            returned = ending_in_return(err.to_vec());
            returned.as_slice()
        }
        false => err,
    };
    // Where the `match` is, of what's written first of it, the call it
    // tests, `catch_unwind(..)`, not where its `Ok` arm is, which a
    // temporary MIR made has: the comments above the `match` are above the
    // `try` (rd156).
    let span = match value.span.is_none() || (!made.span.is_none() && made.span.lo <= value.span.lo) {
        true => made.span,
        false => js::Span {
            lo: value.span.lo,
            hi: made.span.hi.max(value.span.hi),
        },
    };
    // What unwound is the payload `$unwound` makes of what was thrown: the
    // `Err`'s binding, of the `catch`'s, `thrown` (ADR 0660).
    let (binding, in_catch) = match (caught, &error) {
        (js::Caught::Unwound, Some(payload)) => {
            let thrown = thrown_name(payload, err);
            let unwound = Expr::call_helper("$unwound", vec![Expr::var(&thrown)]);
            let made = match set_again {
                true => StmtKind::Let(payload.clone(), Some(unwound)),
                false => StmtKind::Const(payload.clone(), unwound),
            }
            .at(span);
            (Some(thrown), std::iter::once(made).chain(err.iter().cloned()).collect())
        }
        _ => (error.clone(), err.to_vec()),
    };
    // A closure's body, of `catch_unwind(|| ..)`, its value returned: in
    // the `try`, its `return`s the function's (ADR 0660); and what the
    // `catch` reads of the payload as it was thrown (ADR 0706).
    let caught = |tried: Vec<Stmt>| {
        let mut tried = js::applied_tried(tried);
        // A value the program never named, read by the `Result`'s name,
        // `catchUnwind`, is the call where it's read first (ADR 0706,
        // amended).
        if let Some(name) = value_name.as_deref().filter(|name| *name == result) {
            js::read_where_made(&mut tried, name);
        }
        let (binding, handler) = read_as_thrown(binding.clone(), in_catch.clone());
        StmtKind::TryCatch(tried, binding, handler).at(span)
    };
    // What's of the call is where it is, `f(x)` of `catch_unwind(|| { ..
    // f(x) })`, so a comment before it in the closure is before it in the
    // `try` (ADR 0444, amended), as one before the `match` is before that.
    let of_call = match call.span.is_none() {
        true => span,
        false => call.span,
    };
    let alone = StmtKind::Expr(call.clone()).at(of_call);
    let cannot_throw = |stmt: &Stmt| cannot_throw(stmt);
    // The `Ok`'s value kept is the call's too, `return f(x)`.
    if let Some((held, written)) = held {
        return Some((vec![caught(vec![held.at(written.unwrap_or(of_call))])], taken));
    }
    // A value the program never named, a literal the closure returns that
    // the `Ok` arms test, `Ok(true) => return 0`: each arm where the closure
    // returns its value, the rest of the arm after the `try` where the
    // `catch` leaves, else in it where it can't throw (ADR 0706, amended).
    if value_name.as_deref() == Some(result.as_str())
        && let Some(body) = js::called_body(&call)
        && let Some((mut tried, rest)) = arms_where_returned(body, result, ok)
        && (leaves(err) || rest.iter().all(cannot_throw))
    {
        let after = match leaves(err) {
            true => rest.to_vec(),
            false => {
                tried.extend(rest.iter().cloned());
                Vec::new()
            }
        };
        return Some((std::iter::once(caught(tried)).chain(after).collect(), taken));
    }
    // What the `Ok` arm does can't throw: in the `try`.
    if ok.iter().all(cannot_throw) {
        let first = match &value_name {
            Some(name) => StmtKind::Const(name.clone(), call.clone()).at(of_call),
            None => alone,
        };
        let tried = std::iter::once(first).chain(ok.iter().cloned()).collect();
        return Some((vec![caught(tried)], taken));
    }
    // Else after it, where the `Err` arm leaves, its value set by the `try`:
    // a `let` of a name nothing else here reads or declares.
    let unread =
        |name: &String| js::mentions_in(before, name) + js::mentions_in(err, name) + js::mentions_in(after, name) == 0;
    if !leaves(err) || !value_name.as_ref().is_none_or(unread) {
        return None;
    }
    let mut out = Vec::new();
    let tried = match &value_name {
        Some(name) => {
            out.push(StmtKind::Let(name.clone(), None).at(span));
            StmtKind::Assign(Expr::var(name), call.clone()).at(of_call)
        }
        None => alone,
    };
    out.push(caught(vec![tried]));
    out.extend(ok.iter().cloned());
    Some((out, taken))
}

/// A `try` that ends a function, its last statement, or the last of an
/// `if`'s branch, a `try`'s block or a `catch` that does: what it holds, a
/// closure of nothing called where it's made, `catch_unwind(|| ..)`'s,
/// whose `return`s give no value, a `return;` in a loop, is its body, each
/// `return;` the function's, as nothing runs after the `try` but its
/// `finally` either way (ADR 0660, amended).
pub(super) fn tail_tries(body: &mut [Stmt]) {
    let Some(last) = body.last_mut() else { return };
    match &mut last.kind {
        StmtKind::If(_, then, otherwise) => {
            tail_tries(then);
            if let Some(otherwise) = otherwise {
                tail_tries(otherwise);
            }
        }
        StmtKind::Try(block, _) => tail_tries(block),
        StmtKind::TryCatch(tried, _, handler) => {
            tail_tries(handler);
            if let [only] = tried.as_slice()
                && let StmtKind::Expr(call) = &only.kind
                && let ExprKind::Call(callee, args) = &call.kind
                && args.is_empty()
                && let ExprKind::Arrow(params, inner) = &callee.kind
                && params.is_empty()
                && js::returns(inner) == (true, false)
            {
                *tried = inner.clone();
            }
            // Its last `return;`, `let Err(x) = catch_unwind(..) else {
            // return; }`'s, falls off the function's end as it does (rd551).
            if tried.last().is_some_and(|s| matches!(s.kind, StmtKind::Return(None))) {
                tried.pop();
            }
        }
        _ => {}
    }
}

/// `stmts`, each way they end with a `return;`: of a last `if` and its
/// `else`, each that goes on, `if (c) { throw e; } else { f(); return; }`.
fn ending_in_return(mut stmts: Vec<Stmt>) -> Vec<Stmt> {
    let leaves = |stmts: &[Stmt]| stmts.last().is_some_and(js::ends);
    match stmts.last_mut().map(|s| &mut s.kind) {
        Some(StmtKind::If(_, then, Some(otherwise))) => {
            for branch in [then, otherwise] {
                if !leaves(branch) {
                    *branch = ending_in_return(std::mem::take(branch));
                }
            }
        }
        _ => stmts.push(StmtKind::Return(None).at(js::Span::NONE)),
    }
    stmts
}

/// Whether `stmt` can't throw: what makes values, what leaves with one,
/// and an `if` of a test that can't, of each.
fn cannot_throw(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Return(value) => value.as_ref().is_none_or(|v| !v.may_throw()),
        StmtKind::Break(_) | StmtKind::Continue(_) => true,
        StmtKind::If(test, then, otherwise) => {
            !test.may_throw() && then.iter().all(cannot_throw) && otherwise.iter().flatten().all(cannot_throw)
        }
        _ => stmt.only_makes_values(),
    }
}

/// Of an `Ok` arm that reads its value as `result`, the pattern's binding
/// of all of it, `const f = result;` of `Ok(Some(f))`'s `if (result) {
/// const f = result; return f; }`, which may name the `try`'s value: its
/// one declaration there, every read of it after it, and a name `call`,
/// which the `try` runs first, neither reads nor declares, nor one given
/// apart from it, `picked$1`, which it would keep from being that name
/// (ADR 0352).
fn whole_binding(ok: &[Stmt], result: &str, call: &Expr) -> Option<String> {
    let mut found = None;
    let mut aliases = 0;
    js::each_list(ok, &mut |list| {
        for (at, stmt) in list.iter().enumerate() {
            if let StmtKind::Const(name, value) = &stmt.kind
                && matches!(&value.kind, ExprKind::Var(v) if v == result)
            {
                aliases += 1;
                found = Some((name.clone(), js::mentions_in(&list[at + 1..], name)));
            }
        }
    });
    let (name, read_after) = found.filter(|_| aliases == 1)?;
    let base = |n: &str| crate::names::apart(n).map_or(n, |(base, _)| base).to_string();
    let is_name = |n: &str| base(n) == base(&name);
    let mut declared = 0;
    js::each_declared(ok, &mut |d| declared += usize::from(is_name(d)));
    let mut of_call = HashSet::new();
    js::names_in(&[StmtKind::Expr(call.clone()).at(call.span)], &mut of_call);
    let unclashed = !of_call.iter().any(|n| is_name(n));
    (declared == 1 && read_after == js::mentions_in(ok, &name) && unclashed).then_some(name)
}

/// How often `stmts` read `result._0`, the value a `Result` holds.
fn held_reads(stmts: &[Stmt], result: &str) -> usize {
    let mut reads = 0;
    js::each_expr(stmts, &mut |e| reads += usize::from(js::result::reads_held(e, result)));
    reads
}

/// `const { _0: x } = $catchUnwind(f);` of an `f` that never returns, `let
/// Err(x) = catch_unwind(|| panic!(""))`'s, whose `Ok` is none: `try {
/// ..f's body } catch (thrown) { const x = $unwound(thrown); .. }`, as what
/// follows runs only once `f` threw (ADR 0706); the `catch` what follows
/// that reads `x`, or what that declares (ADR 0830).
fn unwound_lets(body: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < body.len() {
        let (before, rest) = body.split_at(i);
        if let Some((made, taken)) = unwound_let(before, &rest[0], &rest[1..]) {
            body.splice(i..=i + taken, [made]);
        }
        i += 1;
    }
}

/// `unwound_lets`' of `stmt`, after `before`, then `rest`: the `try`, and
/// how many of `rest` its `catch` takes.
fn unwound_let(before: &[Stmt], stmt: &Stmt, rest: &[Stmt]) -> Option<(Stmt, usize)> {
    // `const { _0: x } = r;`, `let` of an `x` a shadow sets again, or
    // `const r = ..; const x = r._0;`.
    let (key, payload, mutable, value, rest, skipped) = match (&stmt.kind, rest.first().map(|s| &s.kind)) {
        (
            StmtKind::Destructure {
                pattern: Pattern::Object(fields, None),
                value,
                mutable,
            },
            _,
        ) => match fields.as_slice() {
            [(key, Pattern::Name(payload), None)] => (key.as_str(), payload, *mutable, value, rest, 0),
            _ => return None,
        },
        (StmtKind::Const(result, value), Some(StmtKind::Const(payload, read))) => match &read.kind {
            ExprKind::Member(of, key)
                if matches!(&of.kind, ExprKind::Var(v) if v == result) && js::mentions_in(rest, result) == 1 =>
            {
                (key.as_str(), payload, false, value, &rest[1..], 1)
            }
            _ => return None,
        },
        _ => return None,
    };
    let (call, js::Caught::Unwound) = value.caught()? else {
        return None;
    };
    // `f` never returns: its body throws last, and returns nowhere.
    let ExprKind::Call(_, args) = &value.kind else {
        return None;
    };
    let never_returns = matches!(args.as_slice(), [f] if matches!(&f.kind, ExprKind::Arrow(params, body)
        if params.is_empty()
            && !js::returns(body).0
            && matches!(body.last().map(|s| &s.kind), Some(StmtKind::Throw(_)))));
    if key != "_0" || !never_returns {
        return None;
    }
    let caught = caught_run(payload, rest)?;
    // A function the `catch` would declare that's called before it, where
    // JS hoists it, stays where it is.
    let hoisted = rest[..caught]
        .iter()
        .any(|s| matches!(&s.kind, StmtKind::Function(function) if js::mentions_in(before, &function.name) > 0));
    if hoisted {
        return None;
    }
    let thrown = thrown_name(payload, &rest[..caught]);
    let unwound = Expr::call_helper("$unwound", vec![Expr::var(&thrown)]);
    let made = if mutable {
        StmtKind::Let(payload.clone(), Some(unwound))
    } else {
        StmtKind::Const(payload.clone(), unwound)
    };
    let mut handler = vec![made.at(stmt.span)];
    handler.extend(rest[..caught].iter().cloned());
    let tried = js::applied_tried(vec![StmtKind::Expr(call).at(stmt.span)]);
    let (binding, handler) = read_as_thrown(Some(thrown), handler);
    Some((
        StmtKind::TryCatch(tried, binding, handler).at(stmt.span),
        skipped + caught,
    ))
}

/// How many of `rest` a `catch` of `payload` takes: the fewest, from the
/// first, that hold each mention of `payload` and of every name they
/// declare, so nothing after them reads what the `catch` declares (ADR
/// 0830). `None` where none mentions `payload`.
fn caught_run(payload: &str, rest: &[Stmt]) -> Option<usize> {
    let mut names = HashSet::from([payload.to_string()]);
    let mut taken = 0;
    loop {
        let mentioned = |s: &Stmt| {
            names
                .iter()
                .any(|name| js::mentions_in(std::slice::from_ref(s), name) > 0)
        };
        let Some(last) = rest.iter().rposition(mentioned) else {
            return (taken > 0).then_some(taken);
        };
        if last < taken {
            return Some(taken);
        }
        taken = last + 1;
        js::each_declared(&rest[..taken], &mut |name| {
            names.insert(name.to_string());
        });
    }
}

/// A `catch (thrown) { const payload = $unwound(thrown); .. }`,
/// what it reads of `payload` as `thrown` is (ADR 0706): its
/// `resume_unwind`, `$unwinding(payload)`, of a payload as it was given,
/// nothing written through it (`Known::AsGiven`), as
/// `$unwinding($unwound(e))` is `e`; and what was thrown, its `&JsError`,
/// `payload.error`, `thrown` (ADR 0710). Where nothing else reads the
/// payload, the `catch` is of it, `catch (payload)`; and of a payload its
/// shadow sets to what was thrown first, `let x = $unwound(thrown); x =
/// x.error;`, `catch (x)`. Else the payload is made where it's first read
/// (`unwound_where_read`, rd550).
fn read_as_thrown(binding: Option<String>, handler: Vec<Stmt>) -> (Option<String>, Vec<Stmt>) {
    let Some(thrown) = binding else {
        return (binding, handler);
    };
    let unwound = |made: &Expr| {
        matches!(&made.kind, ExprKind::Call(callee, args)
            if callee.helper_name() == Some("$unwound")
                && matches!(args.as_slice(), [arg] if matches!(&arg.kind, ExprKind::Var(v) if *v == thrown)))
    };
    if let [
        Stmt {
            kind: StmtKind::Let(payload, Some(made)),
            ..
        },
        Stmt {
            kind: StmtKind::Assign(to, error),
            ..
        },
        rest @ ..,
    ] = handler.as_slice()
        && unwound(made)
        && matches!(&to.kind, ExprKind::Var(v) if v == payload)
        && error
            .thrown_error_of()
            .is_some_and(|of| matches!(&of.kind, ExprKind::Var(v) if v == payload))
        && js::mentions_in(rest, &thrown) == 0
    {
        return (Some(payload.clone()), rest.to_vec());
    }
    let Some(Stmt {
        kind: StmtKind::Const(payload, made),
        ..
    }) = handler.first()
    else {
        return (Some(thrown), handler);
    };
    if !unwound(made) {
        return (Some(thrown), handler);
    }
    let payload = payload.clone();
    let mut rest = handler[1..].to_vec();
    let of_payload = |e: &Expr| matches!(&e.kind, ExprKind::Var(v) if *v == payload);
    // A resume of the payload as it was given, which lowering knows
    // (`Known::AsGiven`): what was thrown, `$unwinding($unwound(e))` being
    // `e`.
    let resumed_as_given = |e: &Expr| {
        e.facts.has(js::Known::AsGiven)
            && matches!(&e.kind, ExprKind::Call(callee, args)
                if callee.helper_name() == Some("$unwinding") && matches!(args.as_slice(), [arg] if of_payload(arg)))
    };
    js::each_expr_mut(&mut rest, &mut |e| {
        if resumed_as_given(e) || e.thrown_error_of().is_some_and(of_payload) {
            *e = Expr::var(&thrown).or_at(e.span);
        }
    });
    let mut declared = 0;
    js::each_declared(&rest, &mut |name| declared += usize::from(name == payload));
    if js::mentions_in(&rest, &payload) > 0 || declared > 0 {
        return (Some(thrown), unwound_where_read(handler[0].clone(), &payload, rest));
    }
    js::each_expr_mut(&mut rest, &mut |e| {
        if matches!(&e.kind, ExprKind::Var(v) if *v == thrown) {
            *e = Expr::var(&payload).or_at(e.span);
        }
    });
    (Some(payload), rest)
}

/// A `catch`'s statements `rest`, and `made`, `const payload =
/// $unwound(thrown);`, where the payload is first read: before the first
/// of them that mentions it, so what comes before runs first, as it reads
/// nothing of the payload, nothing but its own writes, which can't fail
/// where the stack's spent as a call can (rd550). There, of that
/// statement's span, where a comment before it is before it. A function
/// declared before it reads none of the payload, as a nested `fn` captures
/// nothing, and a closure that reads it is declared after it.
fn unwound_where_read(made: Stmt, payload: &str, mut rest: Vec<Stmt>) -> Vec<Stmt> {
    let first = rest
        .iter()
        .position(|s| js::mentions_in(std::slice::from_ref(s), payload) > 0);
    match first {
        Some(at) if at > 0 => {
            let span = rest[at].span;
            rest.insert(at, Stmt { span, ..made });
        }
        _ => rest.insert(0, made),
    }
    rest
}

/// `$try(() => call).TAG === "Err"`, or `!== "Ok"`: whether a JS call
/// threw, a `match` of its `Result` that gives `true` of the one and
/// `false` of the other, or its `is_err()`, `is_ok()`. The call, and the
/// test's value where it threw (ADR 0639).
fn threw(e: &Expr) -> Option<(Expr, bool)> {
    let (of, ok, eq) = js::result::tested(e)?;
    Some((of.caught()?.0, eq != ok))
}
