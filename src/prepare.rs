//! Readability preparation on the completed JS tree. No rustc types or APIs.
//! Nothing moves: what JSX holds stays where it's written, a list's callback
//! of several statements and a handler's too, as a person writes them, and
//! oxfmt lays them out where they are (ADR 0218). The printer only lays out
//! the result.

use std::collections::HashSet;

use crate::js::{self, Expr, ExprKind, JsxTag, Prop, Stmt, StmtKind};

pub fn module(module: &mut js::Module) {
    let methods = module.namespaces.iter_mut().flat_map(|n| n.methods.iter_mut());
    for function in module.functions.iter_mut().chain(methods) {
        block(&mut function.body);
        shadows(&mut function.body, None);
        // A closure's own parameter, shadowed in it: nothing outside it sets one.
        js::each_expr_mut(&mut function.body, &mut |e| {
            if let ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) = &mut e.kind {
                let own: HashSet<String> = params.iter().flat_map(|p| p.names()).map(str::to_string).collect();
                shadows(body, Some(&own));
            }
        });
        constants(&mut function.body);
        tested_constants(&mut function.body);
        js::each_block_mut(&mut function.body, &mut |stmts| imported(stmts));
    }
    for constant in &mut module.consts {
        expr(&mut constant.value);
    }
    block(&mut module.statements);
}

/// `const run = await import(spec).then((m) => m.run)` is `const { run } =
/// await import(spec)`, and another name `{ run: next }` (ADR 0304).
fn imported(stmts: &mut [Stmt]) {
    for stmt in stmts {
        let StmtKind::Const(name, value) = &stmt.kind else {
            continue;
        };
        let ExprKind::Await(awaited) = &value.kind else {
            continue;
        };
        let ExprKind::Call(then, args) = &awaited.kind else {
            continue;
        };
        let (ExprKind::Member(import, method), [read]) = (&then.kind, args.as_slice()) else {
            continue;
        };
        let (ExprKind::Import(_), "then", ExprKind::Arrow(params, body)) = (&import.kind, method.as_str(), &read.kind)
        else {
            continue;
        };
        let (
            [js::Pattern::Name(m)],
            [
                Stmt {
                    kind: StmtKind::Return(Some(got)),
                    ..
                },
            ],
        ) = (params.as_slice(), body.as_slice())
        else {
            continue;
        };
        let ExprKind::Member(of, export) = &got.kind else {
            continue;
        };
        if !matches!(&of.kind, ExprKind::Var(v) if v == m) {
            continue;
        }
        stmt.kind = StmtKind::Destructure {
            pattern: js::Pattern::Object(vec![(export.clone(), name.clone(), None)], None),
            value: Expr {
                kind: ExprKind::Await(import.clone()),
                span: value.span,
            },
            mutable: false,
        };
    }
}

fn block(body: &mut Vec<Stmt>) {
    try_catches(body);
    for stmt in body.iter_mut() {
        if let Some(coalesced) = coalescing(stmt) {
            stmt.kind = coalesced;
        }
        // `else {}`, of nothing but Rust's comments, is no `else`.
        if let StmtKind::If(_, _, els) = &mut stmt.kind
            && els.as_ref().is_some_and(Vec::is_empty)
        {
            *els = None;
        }
        // `let timeout = undefined;` is `let timeout;`, as a person writes it:
        // a `let` starts as `undefined`, each time it's run.
        if let StmtKind::Let(_, value) = &mut stmt.kind
            && value.as_ref().is_some_and(|v| matches!(v.kind, ExprKind::Undefined))
        {
            *value = None;
        }
    }
    for stmt in body {
        match &mut stmt.kind {
            StmtKind::Const(_, e)
            | StmtKind::Expr(e)
            | StmtKind::Throw(e)
            | StmtKind::Return(Some(e))
            | StmtKind::Let(_, Some(e))
            | StmtKind::Destructure { value: e, .. } => expr(e),
            StmtKind::Assign(a, b) => {
                expr(a);
                expr(b);
            }
            StmtKind::If(e, a, b) => {
                expr(e);
                block(a);
                if let Some(b) = b {
                    block(b);
                }
            }
            StmtKind::While { cond, body, .. } => {
                expr(cond);
                block(body);
            }
            StmtKind::Labeled(_, body) => block(body),
            StmtKind::Try(body, finally) | StmtKind::TryCatch(body, _, finally) => {
                block(body);
                block(finally);
            }
            StmtKind::ForOf { iterable, body, .. } => {
                expr(iterable);
                block(body);
            }
            StmtKind::For { start, test, body, .. } => {
                expr(start);
                expr(test);
                block(body);
            }
            _ => {}
        }
    }
}

/// `const match = $try(() => f()); if (match.TAG === "Ok") { x = match._0; }
/// else { .. }`, a `match` of what a JS call threw whose `Ok` only keeps
/// the value and whose `Err` reads no error, is `try { x = f(); } catch {
/// .. }`, as JS writes it (ADR 0035): only the call is in the `try`, as only
/// it is in `$try`.
/// A `const` only tests read, `const hide = !!error || !ready`, is a test
/// itself: what it's made of is read by its truth, `error || !ready`, as
/// a person writes it (ADR 0298). One read as a value keeps its boolean.
fn tested_constants(body: &mut Vec<Stmt>) {
    let mut candidates = Vec::new();
    js::each_block_mut(body, &mut |stmts| {
        for stmt in stmts.iter() {
            if let StmtKind::Const(name, value) = &stmt.kind {
                let mut read = value.clone();
                if untest(&mut read) {
                    candidates.push(name.clone());
                }
            }
        }
    });
    for name in candidates {
        // Each read a test makes, marked: none is left if they all are.
        let mut marked = body.clone();
        js::each_block_mut(&mut marked, &mut |stmts| {
            for stmt in stmts.iter_mut() {
                match &mut stmt.kind {
                    StmtKind::If(cond, ..) | StmtKind::While { cond, .. } => mark_tested(cond, &name),
                    StmtKind::For { test, .. } => mark_tested(test, &name),
                    _ => {}
                }
            }
        });
        js::each_expr_mut(&mut marked, &mut |e| {
            if let ExprKind::Cond(test, ..) = &mut e.kind {
                mark_tested(test, &name);
            }
        });
        if js::mentions_in(&marked, &name) != 0 {
            continue;
        }
        js::each_block_mut(body, &mut |stmts| {
            for stmt in stmts.iter_mut() {
                if let StmtKind::Const(declared, value) = &mut stmt.kind
                    && *declared == name
                {
                    untest(value);
                }
            }
        });
    }
}

/// `name`'s reads in `test`, a test, and in the parts of it a test reads by
/// their truth, `!a`, `a && b`, `a || b`, renamed apart.
fn mark_tested(test: &mut Expr, name: &str) {
    match &mut test.kind {
        ExprKind::Var(var) if var == name => *var = String::from("$tested"),
        ExprKind::Unary(js::UnaryOp::Not, a) => mark_tested(a, name),
        ExprKind::Binary(js::Op::And | js::Op::Or, a, b) => {
            mark_tested(a, name);
            mark_tested(b, name);
        }
        _ => {}
    }
}

/// `e` read by its truth: `!!a` is `a`, in the parts of `&&` and `||`.
/// Whether it changed.
fn untest(e: &mut Expr) -> bool {
    if let ExprKind::Unary(js::UnaryOp::Not, not) = &e.kind
        && let ExprKind::Unary(js::UnaryOp::Not, value) = &not.kind
    {
        *e = (**value).clone();
        return true;
    }
    match &mut e.kind {
        ExprKind::Binary(js::Op::And | js::Op::Or, a, b) => {
            let left = untest(a);
            untest(b) || left
        }
        _ => false,
    }
}

fn try_catches(body: &mut Vec<Stmt>) {
    let mut i = 0;
    while i + 1 < body.len() {
        if let Some(caught) = try_catch(&body[i], &body[i + 1], &body[i + 2..]) {
            let span = body[i].span;
            body.splice(i..i + 2, [caught.at(span)]);
        }
        i += 1;
    }
}

fn try_catch(made: &Stmt, tested: &Stmt, rest: &[Stmt]) -> Option<StmtKind> {
    let StmtKind::Const(result, value) = &made.kind else {
        return None;
    };
    let ExprKind::Call(callee, args) = &value.kind else {
        return None;
    };
    let (ExprKind::Var(helper), [thunk]) = (&callee.kind, args.as_slice()) else {
        return None;
    };
    let ExprKind::Arrow(params, thunk) = &thunk.kind else {
        return None;
    };
    let [
        Stmt {
            kind: StmtKind::Return(Some(call)),
            ..
        },
    ] = thunk.as_slice()
    else {
        return None;
    };
    let StmtKind::If(test, kept, Some(failed)) = &tested.kind else {
        return None;
    };
    let ExprKind::Binary(js::Op::Eq, tag, ok) = &test.kind else {
        return None;
    };
    let of_result = |e: &Expr, name: &str| {
        matches!(&e.kind, ExprKind::Member(of, field)
        if field == name && matches!(&of.kind, ExprKind::Var(v) if v == result))
    };
    if helper != "$try"
        || !params.is_empty()
        || !of_result(tag, "TAG")
        || !matches!(&ok.kind, ExprKind::Str(s) if s == "Ok")
        || js::mentions_in(failed, result) > 0
        || js::mentions_in(rest, result) > 0
    {
        return None;
    }
    // The `Ok`'s value kept, as it is: nothing else, which could throw.
    let [Stmt { kind: kept, span }] = kept.as_slice() else {
        return None;
    };
    let body = match kept {
        StmtKind::Return(Some(e)) if of_result(e, "_0") => StmtKind::Return(Some(call.clone())),
        StmtKind::Assign(to, e) if of_result(e, "_0") && matches!(to.kind, ExprKind::Var(_)) => {
            StmtKind::Assign(to.clone(), call.clone())
        }
        _ => return None,
    };
    Some(StmtKind::TryCatch(vec![body.at(*span)], None, failed.clone()))
}

/// `if (x == null) { x = e; }` is `x = x ?? e`, printed `x ??= e`: the
/// same test, and `e` made only where it's none.
fn coalescing(stmt: &Stmt) -> Option<StmtKind> {
    let StmtKind::If(test, then, None) = &stmt.kind else {
        return None;
    };
    let ExprKind::Binary(js::Op::LooseEq, tested, null) = &test.kind else {
        return None;
    };
    let [
        Stmt {
            kind: StmtKind::Assign(target, value),
            ..
        },
    ] = then.as_slice()
    else {
        return None;
    };
    if !matches!(null.kind, ExprKind::Null) || !js::same_path(tested, target) {
        return None;
    }
    Some(StmtKind::Assign(
        target.clone(),
        Expr::bin(js::Op::Coalesce, target.clone(), value.clone()),
    ))
}

/// `let list = [];` that nothing sets again, only changes in place, is
/// `const list = [];`, as JS writes it (ADR 0278). A variable a `&mut`'s
/// handle sets is set.
fn constants(body: &mut Vec<Stmt>) {
    let mut set = HashSet::new();
    js::each_block_mut(body, &mut |stmts| {
        for stmt in stmts.iter() {
            if let StmtKind::Assign(target, _) = &stmt.kind {
                set.extend(root(target));
            }
        }
    });
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Handle(place) | ExprKind::Pair(place, _) = &e.kind {
            set.extend(root(place));
        }
    });
    js::each_block_mut(body, &mut |stmts| {
        for stmt in stmts.iter_mut() {
            if let StmtKind::Let(name, Some(value)) = &stmt.kind
                && !set.contains(name)
            {
                stmt.kind = StmtKind::Const(name.clone(), value.clone());
            }
        }
    });
}

/// The variable a place is, `x`: `x.a = 1` sets none.
fn root(place: &Expr) -> Option<String> {
    match &place.kind {
        ExprKind::Var(name) => Some(name.clone()),
        _ => None,
    }
}

/// `const n$1 = n;`, Rust's `let n = n;`, a variable shadowed by its own
/// value, is `n` itself: `<Type />` for `let Type = from_unknown(Type)`,
/// `content` for `let content = content.clone()`. Only where `n` holds
/// that value wherever `n$1` is read: nothing sets `n` after it, nor in a
/// closure, nor where a loop runs it again. In a closure, `own` are its
/// parameters, the only ones aliased: what's outside it, it doesn't see set.
fn shadows(body: &mut Vec<Stmt>, own: Option<&HashSet<String>>) {
    while let Some((alias, of)) = shadow(body, own) {
        js::each_expr_mut(body, &mut |e| {
            if matches!(&e.kind, ExprKind::Var(name) if *name == alias) {
                e.kind = ExprKind::Var(of.clone());
            }
        });
        js::each_block_mut(body, &mut |stmts| {
            stmts.retain(|s| {
                !matches!(&s.kind, StmtKind::Const(name, value)
                if *name == alias && matches!(&value.kind, ExprKind::Var(v) if *v == of))
            });
        });
    }
}

/// A `const n$1 = n;` that `shadows` makes `n`, and `n`.
fn shadow(body: &[Stmt], own: Option<&HashSet<String>>) -> Option<(String, String)> {
    // What closures set, and what their parameters are named, which a
    // name read in them would mean instead.
    let mut in_closures = HashSet::new();
    let mut copy = body.to_vec();
    js::each_expr_mut(&mut copy, &mut |e| {
        if let ExprKind::Arrow(params, stmts) | ExprKind::AsyncArrow(params, stmts) = &mut e.kind {
            in_closures.extend(params.iter().flat_map(|p| p.names()).map(str::to_string));
            js::statement_lists(stmts, &mut |list| {
                in_closures.extend(list.iter().filter_map(|s| match &s.kind {
                    StmtKind::Assign(target, _) => match &target.kind {
                        ExprKind::Var(name) => Some(name.clone()),
                        _ => None,
                    },
                    _ => None,
                }));
            });
        }
    });
    let mut walk = Walk::default();
    walk.stmts(body, false);
    walk.shadows.into_iter().find_map(|(at, looped, alias, of)| {
        let sets: Vec<usize> = walk
            .sets
            .iter()
            .filter(|(_, name)| *name == of)
            .map(|&(set, _)| set)
            .collect();
        let kept = !in_closures.contains(&of) && (sets.is_empty() || !looped && sets.iter().all(|&set| set < at));
        let owned = own.is_none_or(|own| own.contains(&of));
        (kept && owned).then_some((alias, of))
    })
}

/// The function's statements in order, outside its closures: each
/// `const n$1 = n;`, where, and whether in a loop, and each variable set.
#[derive(Default)]
struct Walk {
    at: usize,
    shadows: Vec<(usize, bool, String, String)>,
    sets: Vec<(usize, String)>,
}

impl Walk {
    fn stmts(&mut self, stmts: &[Stmt], looped: bool) {
        for stmt in stmts {
            self.at += 1;
            match &stmt.kind {
                StmtKind::Const(alias, value) => {
                    if let ExprKind::Var(of) = &value.kind
                        && alias
                            .strip_prefix(of.as_str())
                            .and_then(|rest| rest.strip_prefix('$'))
                            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
                    {
                        self.shadows.push((self.at, looped, alias.clone(), of.clone()));
                    }
                }
                StmtKind::Assign(target, _) => {
                    if let ExprKind::Var(name) = &target.kind {
                        self.sets.push((self.at, name.clone()));
                    }
                }
                StmtKind::If(_, a, b) => {
                    self.stmts(a, looped);
                    if let Some(b) = b {
                        self.stmts(b, looped);
                    }
                }
                StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } => self.stmts(body, true),
                StmtKind::For { name, body, .. } => {
                    self.sets.push((self.at, name.clone()));
                    self.stmts(body, true);
                }
                StmtKind::Labeled(_, body) => self.stmts(body, looped),
                StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                    self.stmts(a, looped);
                    self.stmts(b, looped);
                }
                StmtKind::Let(..)
                | StmtKind::Destructure { .. }
                | StmtKind::Expr(_)
                | StmtKind::Break(_)
                | StmtKind::Continue(_)
                | StmtKind::Return(_)
                | StmtKind::Throw(_) => {}
            }
        }
    }
}

/// `Object.fromEntries([["/index.html", code], ..])`, each key a string
/// written out, is the object literal of them, `{ "/index.html": code }`:
/// the same properties, made in the same order. Not of a key given twice,
/// which a literal would have twice, nor of `__proto__`, which a literal
/// makes the prototype, nor of an `undefined` value, which an object's
/// field leaves out (ADR 0280) and an entry keeps.
fn entries_written_out(e: &Expr) -> Option<Expr> {
    let ExprKind::Call(f, args) = &e.kind else {
        return None;
    };
    let (ExprKind::Member(object, name), [entries]) = (&f.kind, args.as_slice()) else {
        return None;
    };
    let (ExprKind::Var(object), ExprKind::Array(entries)) = (&object.kind, &entries.kind) else {
        return None;
    };
    if object != "Object" || name != "fromEntries" {
        return None;
    }
    let mut fields = Vec::new();
    for entry in entries {
        let ExprKind::Array(pair) = &entry.kind else {
            return None;
        };
        let [key, value] = pair.as_slice() else {
            return None;
        };
        let ExprKind::Str(key) = &key.kind else {
            return None;
        };
        let given = fields
            .iter()
            .any(|field| matches!(field, Prop::Field(k, _) if k == key));
        if given || key == "__proto__" || matches!(value.kind, ExprKind::Undefined) {
            return None;
        }
        fields.push(Prop::Field(key.clone(), value.clone()));
    }
    Some(Expr {
        kind: ExprKind::Object(fields),
        span: e.span,
    })
}

fn expr(e: &mut Expr) {
    match &mut e.kind {
        ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => block(body),
        ExprKind::Function(function) => block(&mut function.body),
        ExprKind::Jsx(jsx) => {
            if let JsxTag::Component(e) = &mut jsx.tag {
                expr(e);
            }
            let element = matches!(jsx.tag, JsxTag::Intrinsic(_));
            for prop in &mut jsx.props {
                match prop {
                    Prop::Field(name, value) => {
                        // React ignores what a DOM element's handler returns: keep
                        // single-call ones as concise arrow expressions (ADR 0040).
                        // A component may read what its callback returns, which
                        // lowering says of one whose call gives `undefined`.
                        if element && js::is_handler_name(name) {
                            js::returning_its_call(value);
                        }
                        expr(value);
                    }
                    Prop::Getter(_, value) | Prop::Spread(value) => expr(value),
                }
            }
            for child in &mut jsx.children {
                expr(child);
            }
        }
        ExprKind::Member(a, _)
        | ExprKind::OptionalMember(a, _)
        | ExprKind::Unary(_, a)
        | ExprKind::Await(a)
        | ExprKind::Spread(a)
        | ExprKind::Handle(a)
        | ExprKind::DropArgument(_, _, a) => expr(a),
        ExprKind::Index(a, b) | ExprKind::Binary(_, a, b) | ExprKind::Pair(a, b) => {
            expr(a);
            expr(b);
        }
        ExprKind::Template(_, values, _) => values.iter_mut().for_each(expr),
        ExprKind::OptionalCall(f, args) => {
            expr(f);
            args.iter_mut().for_each(expr);
        }
        ExprKind::Cond(a, b, c) => {
            expr(a);
            expr(b);
            expr(c);
        }
        ExprKind::Call(f, args) | ExprKind::New(f, args) => {
            expr(f);
            args.iter_mut().for_each(expr);
            if let Some(object) = entries_written_out(e) {
                *e = object;
            }
        }
        ExprKind::Array(items) => items.iter_mut().for_each(expr),
        ExprKind::Object(props) => {
            // A field that's `undefined`, a `None` say, is no key, `{ code }`,
            // as JS leaves it out: what reads it reads `undefined` either way
            // (ADR 0280). Not after a spread, whose field it would then be.
            if !props.iter().any(|p| matches!(p, Prop::Spread(_))) {
                props.retain(|p| !matches!(p, Prop::Field(_, value) if matches!(value.kind, ExprKind::Undefined)));
            }
            for prop in props {
                let (Prop::Field(_, value) | Prop::Getter(_, value) | Prop::Spread(value)) = prop;
                expr(value);
            }
        }
        // Every variant with an expression in it is above, so a rule of
        // what an expression is, ADR 0280's say, holds wherever it's written.
        ExprKind::Num(_)
        | ExprKind::BigInt(_)
        | ExprKind::BigUint(_)
        | ExprKind::Bool(_)
        | ExprKind::Str(_)
        | ExprKind::Lines(_)
        | ExprKind::Undefined
        | ExprKind::Null
        | ExprKind::Var(_)
        | ExprKind::Symbol(_)
        | ExprKind::FunctionHole(_)
        | ExprKind::Import(_)
        | ExprKind::Regex(_) => {}
    }
}
