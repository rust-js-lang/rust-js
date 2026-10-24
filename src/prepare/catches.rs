//! A `match` of a `Result` of what may throw as JS's `try` and `catch`: a
//! JS call's (ADRs 0035, 0637) or `catch_unwind`'s (ADR 0660), and what
//! the `catch` reads of what was thrown.

use std::collections::HashSet;

use crate::js::{self, Expr, ExprKind, Pattern, Stmt, StmtKind};

/// What a `catch` of an unwind names what it caught, where it reads the
/// payload `$unwound` makes of it.
const THROWN: &str = "thrown";

pub(super) fn try_catches(body: &mut Vec<Stmt>) {
    unwound_lets(body);
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
    let mut i = 0;
    while i < body.len() {
        // A `Result` tested where it's made, `if ($catchUnwind(f).TAG ===
        // "Err" && c)`, is one made first, then tested (ADR 0706, amended).
        if let Some((made, tested)) = made_where_tested(&body[i])
            && let Some((caught, taken)) = chained_try_catch(&made, &tested, &body[..i], &body[i + 1..])
        {
            body.splice(i..i + 1 + taken, caught);
        }
        if i + 1 >= body.len() {
            break;
        }
        // Past what's declared for it to set, `let text;`, as MIR's lowering
        // declares a variable where the program does.
        let declared = (body[i + 1..].iter())
            .take_while(|s| matches!(s.kind, StmtKind::Let(_, None)))
            .count();
        let at = i + 1 + declared;
        if at < body.len()
            && let Some((caught, taken)) = chained_try_catch(&body[i], &body[at], &body[..i], &body[at + 1..])
        {
            body.splice(at..at + 1 + taken, caught);
            body.remove(i);
        }
        i += 1;
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
fn chained_try_catch(made: &Stmt, tested: &Stmt, before: &[Stmt], rest: &[Stmt]) -> Option<(Vec<Stmt>, usize)> {
    if let Some(caught) = try_catch(made, tested, before, rest) {
        return Some(caught);
    }
    let StmtKind::Const(result, _) = &made.kind else {
        return None;
    };
    let (tested, otherwise) = unchained(result, tested)?;
    let rest: Vec<Stmt> = otherwise.iter().chain(rest).cloned().collect();
    let (mut caught, taken) = try_catch(made, &tested, before, &rest)?;
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
/// `before`, what's before it, which that `let` mustn't clash with.
fn try_catch(made: &Stmt, tested: &Stmt, before: &[Stmt], rest: &[Stmt]) -> Option<(Vec<Stmt>, usize)> {
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
    let held = match ok {
        [stmt] => kept(stmt).map(|kind| kind.at(stmt.span)),
        _ => None,
    };
    // Nothing else reads the `Result`.
    let reads = js::mentions_in(ok, result) + js::mentions_in(err, result) + js::mentions_in(after, result);
    if reads != usize::from(held.is_some() && value_name.is_none()) {
        return None;
    }
    let span = made.span;
    // What unwound is the payload `$unwound` makes of what was thrown: the
    // `Err`'s binding, of the `catch`'s, `thrown` (ADR 0660).
    let (binding, in_catch) = match (caught, &error) {
        (js::Caught::Unwound, Some(payload)) => {
            if payload == THROWN || js::mentions_in(err, THROWN) > 0 {
                return None;
            }
            let unwound = Expr::call_helper("$unwound", vec![Expr::var(THROWN)]);
            let made = match set_again {
                true => StmtKind::Let(payload.clone(), Some(unwound)),
                false => StmtKind::Const(payload.clone(), unwound),
            }
            .at(span);
            (
                Some(THROWN.to_string()),
                std::iter::once(made).chain(err.iter().cloned()).collect(),
            )
        }
        _ => (error.clone(), err.to_vec()),
    };
    // A closure's body, of `catch_unwind(|| ..)`, its value returned: in
    // the `try`, its `return`s the function's (ADR 0660); and what the
    // `catch` reads of the payload as it was thrown (ADR 0706).
    let caught = |tried: Vec<Stmt>| {
        let tried = js::applied_tried(tried);
        let (binding, handler) = read_as_thrown(binding.clone(), in_catch.clone());
        StmtKind::TryCatch(tried, binding, handler).at(span)
    };
    let alone = StmtKind::Expr(call.clone()).at(span);
    // What can't throw: what makes values, and what leaves with one.
    let cannot_throw = |stmt: &Stmt| match &stmt.kind {
        StmtKind::Return(value) => value.as_ref().is_none_or(|v| !v.may_throw()),
        StmtKind::Break(_) | StmtKind::Continue(_) => true,
        _ => stmt.only_makes_values(),
    };
    if let Some(held) = held {
        return Some((vec![caught(vec![held])], taken));
    }
    // What the `Ok` arm does can't throw: in the `try`.
    if ok.iter().all(cannot_throw) {
        let first = match &value_name {
            Some(name) => StmtKind::Const(name.clone(), call.clone()).at(span),
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
            StmtKind::Assign(Expr::var(name), call.clone()).at(span)
        }
        None => alone,
    };
    out.push(caught(vec![tried]));
    out.extend(ok.iter().cloned());
    Some((out, taken))
}

/// `const { _0: x } = $catchUnwind(f);` of an `f` that never returns, `let
/// Err(x) = catch_unwind(|| panic!(""))`'s, whose `Ok` is none: `try {
/// ..f's body } catch (thrown) { const x = $unwound(thrown); .. }`, the
/// `catch` what follows up to its last read of `x`, as what follows runs
/// only once `f` threw (ADR 0706).
fn unwound_lets(body: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < body.len() {
        if let Some((made, taken)) = unwound_let(&body[i], &body[i + 1..]) {
            let made_len = made.len();
            body.splice(i..=i + taken, made);
            i += made_len - 1;
        }
        i += 1;
    }
}

/// `unwound_lets`' of `stmt`, then `rest`: the `try`, after what it
/// declares, and how many of `rest` its `catch` takes.
fn unwound_let(stmt: &Stmt, rest: &[Stmt]) -> Option<(Vec<Stmt>, usize)> {
    // `const { _0: x } = r;`, or `const r = ..; const x = r._0;`.
    let (key, payload, value, rest, skipped) = match (&stmt.kind, rest.first().map(|s| &s.kind)) {
        (
            StmtKind::Destructure {
                pattern: Pattern::Object(fields, None),
                value,
                mutable: false,
            },
            _,
        ) => match fields.as_slice() {
            [(key, Pattern::Name(payload), None)] => (key.as_str(), payload, value, rest, 0),
            _ => return None,
        },
        (StmtKind::Const(result, value), Some(StmtKind::Const(payload, read))) => match &read.kind {
            ExprKind::Member(of, key)
                if matches!(&of.kind, ExprKind::Var(v) if v == result) && js::mentions_in(rest, result) == 1 =>
            {
                (key.as_str(), payload, value, &rest[1..], 1)
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
    if key != "_0" || !never_returns || payload == THROWN || js::mentions_in(rest, THROWN) > 0 {
        return None;
    }
    let last = rest
        .iter()
        .rposition(|s| js::mentions_in(std::slice::from_ref(s), payload) > 0)?;
    let (caught, after) = rest.split_at(last + 1);
    // What the `catch` declares that what follows reads, `let control;`
    // before the `try`, set in the `catch`; anything else it declares, in
    // a block of its own, read nowhere else.
    let read_after = |name: &str| js::mentions_in(after, name) > 0;
    let mut declared = Vec::new();
    let mut handler =
        vec![StmtKind::Const(payload.clone(), Expr::call_helper("$unwound", vec![Expr::var(THROWN)])).at(stmt.span)];
    for s in caught {
        match &s.kind {
            StmtKind::Const(name, value) | StmtKind::Let(name, Some(value)) if read_after(name) => {
                declared.push(StmtKind::Let(name.clone(), None).at(s.span));
                handler.push(StmtKind::Assign(Expr::var(name), value.clone()).at(s.span));
            }
            _ => handler.push(s.clone()),
        }
    }
    let mut inner = HashSet::new();
    js::each_declared(&handler, &mut |name| {
        inner.insert(name.to_string());
    });
    if inner.iter().any(|name| read_after(name)) {
        return None;
    }
    let tried = js::applied_tried(vec![StmtKind::Expr(call).at(stmt.span)]);
    let (binding, handler) = read_as_thrown(Some(THROWN.to_string()), handler);
    declared.push(StmtKind::TryCatch(tried, binding, handler).at(stmt.span));
    Some((declared, skipped + last + 1))
}

/// A `catch (thrown) { const payload = $unwound(thrown); .. }`,
/// what it reads of `payload` as `thrown` is (ADR 0706): its
/// `resume_unwind`, `$unwinding(payload)`, of a payload only read, as
/// `$unwinding($unwound(e))` is `e`; and what was thrown, its `&JsError`,
/// `payload.error`, `thrown` (ADR 0710). Where nothing else reads the
/// payload, the `catch` is of it, `catch (payload)`; and of a payload its
/// shadow sets to what was thrown first, `let x = $unwound(thrown); x =
/// x.error;`, `catch (x)`.
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
    let resumed = |e: &Expr| {
        matches!(&e.kind, ExprKind::Call(callee, args)
            if callee.helper_name() == Some("$unwinding") && matches!(args.as_slice(), [arg] if of_payload(arg)))
    };
    // Only read: each mention a field's, `payload.impl`, or its resume,
    // and none written through it.
    let mut read = 0;
    js::each_expr(&rest, &mut |e| match &e.kind {
        ExprKind::Member(of, _) if of_payload(of) => read += 1,
        _ if resumed(e) => read += 1,
        _ => {}
    });
    let only_read = read == js::mentions_in(&rest, &payload) && !js::Writes::of(&rest).changed.contains(&payload);
    js::each_expr_mut(&mut rest, &mut |e| {
        if (only_read && resumed(e)) || e.thrown_error_of().is_some_and(of_payload) {
            *e = Expr::var(&thrown).or_at(e.span);
        }
    });
    let mut declared = 0;
    js::each_declared(&rest, &mut |name| declared += usize::from(name == payload));
    if js::mentions_in(&rest, &payload) > 0 || declared > 0 {
        let mut kept = vec![handler[0].clone()];
        kept.extend(rest);
        return (Some(thrown), kept);
    }
    js::each_expr_mut(&mut rest, &mut |e| {
        if matches!(&e.kind, ExprKind::Var(v) if *v == thrown) {
            *e = Expr::var(&payload).or_at(e.span);
        }
    });
    (Some(payload), rest)
}

/// `$try(() => call).TAG === "Err"`, or `!== "Ok"`: whether a JS call
/// threw, a `match` of its `Result` that gives `true` of the one and
/// `false` of the other, or its `is_err()`, `is_ok()`. The call, and the
/// test's value where it threw (ADR 0639).
fn threw(e: &Expr) -> Option<(Expr, bool)> {
    let (of, ok, eq) = js::result::tested(e)?;
    Some((of.caught()?.0, eq != ok))
}
