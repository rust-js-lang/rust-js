//! Tidy's bindings: declarations, aliases, what's read where it's made,
//! and temporaries and their names.

use super::*;

/// `const x = [loud(1), 2]` nothing reads: each part that does something, in
/// turn, `loud(1);`, as the array, made here, is never seen; `const x =
/// f()`, what a call gave, its `fmt::Result` say: `f();`; and `const old =
/// task.callback; task.callback = next;`, the old value a `replace` read of
/// the place it writes: `task.callback = next;`. Not another read, which
/// may be a binding's getter, `el.offsetWidth`, which lays the page out.
pub(super) fn unread_made(stmts: &mut Vec<Stmt>, mentions: &HashMap<String, usize>) {
    let mut i = 0;
    while i < stmts.len() {
        if let StmtKind::Const(name, value) = &stmts[i].kind
            && !mentions.contains_key(name)
            && !value.has_effects()
            && matches!(stmts.get(i + 1).map(|next| &next.kind),
                Some(StmtKind::Assign(target, _)) if target.kind == value.kind)
        {
            stmts.remove(i);
            continue;
        }
        if let StmtKind::Const(name, value) = &stmts[i].kind
            && !mentions.contains_key(name)
            && let ExprKind::Array(parts) = &value.kind
            // A spread's effect is the stepping it does, which only an
            // array or a call's arguments write: the array whole (#256).
            && !parts.iter().any(|part| matches!(part.kind, ExprKind::Spread(_)))
        {
            let span = stmts[i].span;
            let done: Vec<Stmt> = (parts.iter())
                .filter(|part| part.has_effects())
                .map(|part| StmtKind::Expr(part.clone()).at(span))
                .collect();
            let count = done.len();
            stmts.splice(i..=i, done);
            i += count;
            continue;
        }
        if let StmtKind::Const(name, value) = &stmts[i].kind
            && !mentions.contains_key(name)
            && matches!(value.kind, ExprKind::Call(..))
        {
            stmts[i].kind = StmtKind::Expr(value.clone());
        }
        i += 1;
    }
}

/// Whether `stmts` read `name` alone, `name[i]` or `name.length`: not
/// assign it, call a method of it, or give it to what could change it.
pub(super) fn only_read(stmts: &[Stmt], name: &str) -> bool {
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
pub(super) fn assigns(stmts: &[Stmt], name: &str) -> bool {
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

/// An arrow's parameter its function's names made unique, `(value$1) =>
/// f(value$1)`, is the name it was made of where the arrow names nothing
/// else so, `(value) => f(value)`, as JS's arrows shadow: once nothing's
/// left to name. Not a name of the module's, which a closure's parameters
/// stay apart from.
pub(in crate::lower::mir) fn unsuffixed_params(body: &mut [Stmt], module: &HashSet<String>) {
    js::each_expr_mut(body, &mut |e| {
        let (ExprKind::Arrow(params, inner) | ExprKind::AsyncArrow(params, inner)) = &mut e.kind else {
            return;
        };
        for i in 0..params.len() {
            for k in 0..params[i].names().len() {
                let name = params[i].names()[k].to_string();
                let Some((base, _)) = crate::names::apart(&name) else {
                    continue;
                };
                let mut named = HashSet::new();
                js::names_in(inner, &mut named);
                named.extend(params.iter().flat_map(|p| p.names()).map(str::to_string));
                if named.contains(base) || module.contains(base) {
                    continue;
                }
                js::each_expr_mut(inner, &mut |e| {
                    if let ExprKind::Var(v) = &mut e.kind
                        && *v == name
                    {
                        *v = base.to_string();
                    }
                });
                *params[i].names_mut().swap_remove(k) = base.to_string();
            }
        }
    });
}

/// A temporary MIR made, by its local's index, `_5`: the one spelling of
/// one, which `is_temporary` knows, and no name the program gives has
/// (`user_name`).
pub(in crate::lower::mir) fn temporary(local: usize) -> String {
    format!("_{local}")
}

/// What MIR's lowering holds an argument in, `_arg`, named once tidied.
pub(in crate::lower::mir) const HELD_ARG: &str = "_arg";

/// A name the program gives a local, as JS's: one shaped as a temporary's,
/// `let _5`, is `_5_`, so tidy never takes it for one and names it apart.
pub(in crate::lower::mir) fn user_name(name: &str) -> String {
    match is_temporary(name) {
        true => format!("{name}_"),
        false => name.to_string(),
    }
}

/// Whether `name` is MIR's lowering's of an argument it holds, `_arg`.
pub(super) fn is_held_arg(name: &str) -> bool {
    name.split('$').next() == Some(HELD_ARG)
}

/// Whether `name` is a temporary MIR made, `_5`, or one given apart from
/// one, `_5$1`, not one the program names.
pub(in crate::lower::mir) fn is_temporary(name: &str) -> bool {
    name.strip_prefix('_')
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit() || c == '$'))
}

/// `const _5 = children;`, a temporary of a variable: `children` where it's
/// read, before `children` is set again, as in `match name.as_str()` of a
/// `let mut name`; one read nowhere, gone. A closure neither reads a
/// temporary, its captures its own `const`s, nor sets what one's of between,
/// which Rust's borrows forbid.
pub(super) fn alias_variables(body: &mut Vec<Stmt>) {
    js::statement_lists(body, &mut |stmts| {
        let mut i = 0;
        while i < stmts.len() {
            if let StmtKind::Const(alias, value) = &stmts[i].kind
                && is_temporary(alias)
                && let ExprKind::Var(of) = &value.kind
            {
                let last = (i + 1..stmts.len())
                    .rev()
                    .find(|&j| js::mentions_in(&stmts[j..=j], alias) > 0);
                // What it's of set nowhere from it to its last read, or
                // set only past each read on every way, `const _15 =
                // newLast; if (_15 == null) { .. } else { _15.next = clone;
                // newLast = clone; }`.
                let kept = last.is_some_and(|last| {
                    let reads = &stmts[i + 1..=last];
                    js::Writes::of(reads).may_replace(of)
                        && (declares(reads, of) || !read_before_set(reads, alias, of, &mut false))
                });
                if !kept {
                    // Each read where it's written, else where the
                    // temporary was, which says which `if` the program
                    // wrote a test is of (`jumps::written_at`).
                    let (alias, of, at) = (alias.clone(), Expr::var(of), value.span);
                    stmts.remove(i);
                    if let Some(last) = last {
                        js::each_expr_mut(&mut stmts[i..last], &mut |e| {
                            if matches!(&e.kind, ExprKind::Var(v) if *v == alias) {
                                *e = of.clone().or_at(e.span).or_at(at);
                            }
                        });
                    }
                    continue;
                }
            }
            i += 1;
        }
    });
}

/// Whether each read of `alias` in `stmts` is before `of` is set, on every
/// way through them, `set` whether it is where they start: an `if`'s
/// branches each their own way, its test before them; any other statement
/// that reads `alias` and sets `of` too, in an order this doesn't follow, is
/// not, but `of = e`, which reads `e` first. Not of a closure's read,
/// which may run once `of` is set: no closure reads a temporary, and
/// `copied_bindings` takes none whose binding one reads.
pub(super) fn read_before_set(stmts: &[Stmt], alias: &str, of: &str, set: &mut bool) -> bool {
    let reads = |stmts: &[Stmt]| js::mentions_in(stmts, alias) > 0;
    let sets = |stmts: &[Stmt]| js::Writes::of(stmts).may_replace(of);
    for stmt in stmts {
        let one = std::slice::from_ref(stmt);
        match &stmt.kind {
            StmtKind::If(test, then, otherwise) => {
                let test = [StmtKind::Expr(test.clone()).at(stmt.span)];
                if (*set && reads(&test)) || (reads(&test) && sets(&test)) {
                    return false;
                }
                *set |= sets(&test);
                let otherwise = otherwise.as_deref().unwrap_or_default();
                let mut other = *set;
                if !read_before_set(then, alias, of, set) || !read_before_set(otherwise, alias, of, &mut other) {
                    return false;
                }
                // A branch that leaves, by `continue`, `break` or `return`,
                // is no way to what's after the `if`: what's set there is
                // set on none, `if (child === skip) { child = child.sibling;
                // continue; }` (ADR 0434, amended). Its jump leaves `stmts`
                // too, as this follows no loop's or label's block, which its
                // jump might stay in.
                let goes_on = |stmts: &[Stmt]| !stmts.last().is_some_and(js::ends);
                *set = (goes_on(then) && *set) || (goes_on(otherwise) && other);
            }
            StmtKind::Assign(target, value) if matches!(&target.kind, ExprKind::Var(v) if v == of) => {
                let value = [StmtKind::Expr(value.clone()).at(stmt.span)];
                if *set && reads(&value) || reads(&value) && sets(&value) {
                    return false;
                }
                *set = true;
            }
            _ if reads(one) && (*set || sets(one)) => return false,
            _ => *set |= sets(one),
        }
    }
    true
}

/// `let t; try { t = a; } catch { t = b; } x = t;`, a temporary each way
/// through a statement gives its value last, then given to a variable: the
/// variable given it there, where the statement reads the variable
/// nowhere.
pub(super) fn forwarded_values(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i + 2 < stmts.len() {
        if let StmtKind::Let(t, None) = &stmts[i].kind
            && is_temporary(t)
            && let StmtKind::Assign(target, value) = &stmts[i + 2].kind
            && let ExprKind::Var(x) = &target.kind
            && matches!(&value.kind, ExprKind::Var(v) if v == t)
            && js::mentions_in(&stmts[i + 1..=i + 1], x) == 0
            && js::mentions_in(&stmts[i + 3..], t) == 0
            && last_assigns(&stmts[i + 1..=i + 1], t)
        {
            let (t, x) = (t.clone(), x.clone());
            let mut given = vec![stmts.remove(i + 1)];
            js::statement_lists(&mut given, &mut |stmts| {
                for stmt in stmts.iter_mut() {
                    if let StmtKind::Assign(target, _) = &mut stmt.kind
                        && matches!(&target.kind, ExprKind::Var(v) if *v == t)
                    {
                        *target = Expr::var(&x);
                    }
                }
            });
            stmts.splice(i..i + 2, given);
            continue;
        }
        i += 1;
    }
}

/// Whether each way through `stmts` that goes on past them gives `t` its
/// value last, `t = e` of what doesn't read it, in a branch of an `if` or
/// a `try` that ends them, and nothing else names `t`.
pub(super) fn last_assigns(stmts: &[Stmt], t: &str) -> bool {
    let names = |s: &Stmt| js::mentions_in(std::slice::from_ref(s), t) > 0;
    let Some((last, before)) = stmts.split_last() else {
        return false;
    };
    if before.iter().any(names) {
        return false;
    }
    if !names(last) {
        return ends(last);
    }
    match &last.kind {
        StmtKind::Assign(target, value) => matches!(&target.kind, ExprKind::Var(v) if v == t) && !value.mentions_var(t),
        StmtKind::If(test, then, otherwise) => {
            !test.mentions_var(t) && last_assigns(then, t) && otherwise.as_deref().is_some_and(|o| last_assigns(o, t))
        }
        StmtKind::TryCatch(body, _, caught) => last_assigns(body, t) && last_assigns(caught, t),
        _ => false,
    }
}

/// `const t = q.title;` of plain data, which nothing changes, held by MIR,
/// a temporary or what an element reads: `q.title` where `t` is read, each
/// time. Not a read a getter may answer, which may give another value each
/// time it's read (`Expr::reads_same`, #189).
pub(super) fn plain_reads(stmts: &mut Vec<Stmt>, plain: &HashSet<String>, captured: &HashSet<String>) {
    let mut i = 0;
    while i < stmts.len() {
        let read = match &stmts[i].kind {
            StmtKind::Const(name, value) if is_temporary(name) || captured.contains(name) => {
                let mut root = value;
                while let ExprKind::Member(of, _) = &root.kind {
                    root = of;
                }
                (matches!(value.kind, ExprKind::Member(..))
                    && value.reads_same()
                    && (matches!(&root.kind, ExprKind::Var(v) if plain.contains(v))
                        || field_path(value).is_some_and(|path| plain.contains(&path))))
                .then(|| (name.clone(), value.clone()))
            }
            _ => None,
        };
        let Some((name, value)) = read else {
            i += 1;
            continue;
        };
        stmts.remove(i);
        js::each_expr_mut(&mut stmts[i..], &mut |e| {
            if matches!(&e.kind, ExprKind::Var(v) if *v == name) {
                *e = Expr {
                    span: e.span,
                    ..value.clone()
                };
            }
        });
    }
}

/// `const _3 = e; … if (_3) { const current = _3; … }`: a temporary that a
/// variable the program names takes, in a block inside where it's read,
/// `if let Some(current) = r.current()`'s, or several arms' of one name,
/// where nothing else is named so: `const current = e;`, read by that name.
pub(super) fn named_temporaries(body: &mut Vec<Stmt>) {
    // How often each name is declared, and read, in the whole body.
    let mut declared: HashMap<String, usize> = HashMap::new();
    js::each_list(body, &mut |stmts| {
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
            // One name, each arm's that binds it, `n` of `(n, Some(x))` and
            // `(n, None)`, given apart until they're named in their scopes,
            // `n` and `n$1` (ADR 0357): each declared nowhere else, and read
            // only after it.
            let Some((b, _)) = found.first() else { continue };
            let base = |name: &str| name.split('$').next().unwrap_or(name).to_string();
            if found.iter().any(|(other, after)| {
                base(other) != base(b)
                    || declared.get(other) != Some(&1)
                    || reads.get(other).copied().unwrap_or(0) != *after
            }) {
                continue;
            }
            let b = b.clone();
            let taken: Vec<String> = found.iter().map(|(name, _)| name.clone()).collect();
            for name in &taken {
                dropping(stmts, &a, name);
            }
            let StmtKind::Const(name, _) = &mut stmts[i].kind else {
                unreachable!("matched")
            };
            *name = b.clone();
            js::each_expr_mut(&mut stmts[i + 1..], &mut |e| {
                if let ExprKind::Var(v) = &mut e.kind
                    && (*v == a || taken.contains(v))
                {
                    *v = b.clone();
                }
            });
        }
    });
}

/// Each `const b = a;` of a name the program gives in the blocks of
/// `stmts`, not in their closures, and how often `b` is read after it.
pub(super) fn taking(stmts: &[Stmt], a: &str, found: &mut Vec<(String, usize)>) {
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
            StmtKind::Switch { cases, default, .. } => {
                js::case_lists(cases, default).for_each(|body| taking(body, a, found));
            }
            StmtKind::Try(x, y) | StmtKind::TryCatch(x, _, y) => {
                taking(x, a, found);
                taking(y, a, found);
            }
            _ => {}
        }
    }
}

/// `stmts` without `taking`'s `const b = a;`, theirs or their blocks'.
pub(super) fn dropping(stmts: &mut Vec<Stmt>, a: &str, b: &str) {
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
            StmtKind::Switch { cases, default, .. } => js::case_lists_mut(cases, default).collect(),
            StmtKind::Try(x, y) | StmtKind::TryCatch(x, _, y) => vec![x, y],
            _ => Vec::new(),
        };
        for list in lists {
            dropping(list, a, b);
        }
    }
}

/// `let c; if (t) { c = e; }`, what MIR made of an element only where its
/// test holds, ahead of a later child, read once: `const c = t ? e :
/// undefined;`, which reads as one, and is read in place as one.
pub(super) fn guarded_lets(stmts: &mut Vec<Stmt>, captured: &HashSet<String>) {
    let mut i = 0;
    while i + 2 < stmts.len() {
        let made = match (&stmts[i].kind, &stmts[i + 1].kind) {
            (StmtKind::Let(x, None), StmtKind::If(test, then, None))
                if let [
                    Stmt {
                        kind: StmtKind::Assign(target, value),
                        ..
                    },
                ] = then.as_slice()
                    && matches!(&target.kind, ExprKind::Var(v) if v == x)
                    && (is_temporary(x) || captured.contains(x))
                    && js::mentions_in(&stmts[i + 2..], x) == 1 =>
            {
                Some((x.clone(), Expr::cond(test.clone(), value.clone(), Expr::undefined())))
            }
            _ => None,
        };
        if let Some((x, value)) = made {
            let span = stmts[i + 1].span;
            stmts.splice(i..i + 2, [StmtKind::Const(x, value).at(span)]);
        }
        i += 1;
    }
}

/// Whether `stmts` set `name` again, or declare it, a closure's in them too:
/// their own lists, then each closure's, each found once by `each_expr`,
/// however deep.
pub(super) fn rebinds(stmts: &[Stmt], name: &str) -> bool {
    binds(stmts, name, true)
}

/// Whether `stmts` declare `name` again, a closure's parameter too, as
/// `rebinds` says, but for what assigns it.
pub(super) fn declares(stmts: &[Stmt], name: &str) -> bool {
    binds(stmts, name, false)
}

fn binds(stmts: &[Stmt], name: &str, assigned: bool) -> bool {
    let own = |stmts: &[Stmt]| {
        let mut found = false;
        js::each_list(stmts, &mut |list| {
            for stmt in list.iter() {
                found |= match &stmt.kind {
                    StmtKind::Const(n, _) | StmtKind::Let(n, _) => n == name,
                    StmtKind::Destructure { pattern, .. } | StmtKind::ForOf { pattern, .. } => {
                        pattern.names().contains(&name)
                    }
                    StmtKind::For { name: n, .. } => n == name,
                    StmtKind::Assign(target, _) => assigned && matches!(&target.kind, ExprKind::Var(v) if v == name),
                    _ => false,
                };
            }
        });
        found
    };
    let mut found = own(stmts);
    js::each_expr(stmts, &mut |e| {
        if let ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) = &e.kind {
            found |= params.iter().any(|p| p.names().contains(&name)) || own(body);
        }
    });
    found
}

/// Whether a closure in `stmts` reads `name`.
pub(super) fn in_closure(stmts: &[Stmt], name: &str) -> bool {
    let mut found = false;
    js::each_expr(stmts, &mut |e| {
        if let ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) = &e.kind {
            found |= js::mentions_in(body, name) > 0;
        }
    });
    found
}

/// `_4`, a temporary MIR holds still, read more than once: named after
/// what it holds, the field it reads, `routes` of `tree.routes` or
/// `length` of `slice?.length`, else what rust-analyzer suggests of it,
/// `trailingZeros` or `point` (`suggested`, ADR 0569); a trait object's
/// whose method is called, `receiver`; else `value`, an argument held
/// `arg`, or `tmp` of a `let` its branches set. The first of those names no
/// other in the function has, its parameters' and closures' too, else
/// `value`, `arg` or `tmp`, given apart where it must be.
pub(super) fn readable_temporaries(
    params: &[js::Pattern],
    body: &mut Vec<Stmt>,
    suggested: &HashMap<String, Vec<String>>,
) {
    let mut taken: HashSet<String> = params.iter().flat_map(|p| p.names()).map(str::to_string).collect();
    js::names_in(body, &mut taken);
    // And the name each function another module gives this one is imported
    // by, which the link step keeps where no local takes it: a suggested
    // `prepare` of `prepare(3)` would push the import apart, `prepare$1`.
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Symbol(symbol) = &e.kind {
            taken.insert(symbol.export.clone());
        }
    });
    // What an element is given each as, `href={_5}` or a child, `{_6}`: the
    // prop's name, `href`, or `children`.
    let mut given: HashMap<String, String> = HashMap::new();
    js::each_expr(body, &mut |e| {
        // A trait object's, `_5.impl.add(_5.value)`: its `receiver`.
        if let ExprKind::Call(callee, _) = &e.kind
            && let ExprKind::Member(of, _) = &callee.kind
            && let ExprKind::Member(object, key) = &of.kind
            && key == js::DICTIONARY
            && let ExprKind::Var(v) = &object.kind
            && is_temporary(v)
        {
            given.entry(v.clone()).or_insert_with(|| "receiver".to_string());
        }
        if let ExprKind::Jsx(jsx) = &e.kind {
            for prop in &jsx.props {
                if let js::Prop::Field(key, value) = prop
                    && let ExprKind::Var(v) = &value.kind
                    && (is_temporary(v) || is_held_arg(v))
                    && key.chars().all(|c| c.is_ascii_alphanumeric())
                {
                    given.entry(v.clone()).or_insert_with(|| key.clone());
                }
            }
            for child in &jsx.children {
                if let ExprKind::Var(v) = &child.kind
                    && (is_temporary(v) || is_held_arg(v))
                {
                    given.entry(v.clone()).or_insert_with(|| "children".to_string());
                }
            }
        }
    });
    let mut renames = Vec::new();
    js::statement_lists(body, &mut |list| {
        for stmt in list.iter() {
            match &stmt.kind {
                StmtKind::Const(name, value) if is_temporary(name) || is_held_arg(name) => {
                    let any = match is_held_arg(name) {
                        true => "arg",
                        false => "value",
                    };
                    let names = match (given.get(name), &value.kind) {
                        (Some(prop), _) => vec![prop.clone()],
                        (None, _) if is_held_arg(name) => Vec::new(),
                        (None, ExprKind::Member(_, field) | ExprKind::OptionalMember(_, field)) => vec![field.clone()],
                        (None, _) => suggested.get(name).cloned().unwrap_or_default(),
                    };
                    renames.push((name.clone(), names, any));
                }
                // Held in a `try` that assigns it, `let _arg; try { _arg = .. }`,
                // as what it's given as is: `arg`.
                StmtKind::Let(name, _) if is_held_arg(name) => renames.push((name.clone(), Vec::new(), "arg")),
                StmtKind::Let(name, _) if is_temporary(name) => {
                    renames.push((name.clone(), suggested.get(name).cloned().unwrap_or_default(), "tmp"));
                }
                _ => {}
            }
        }
    });
    // In the order they're first read, as the program reads: one made in a
    // `try` is named before a `let` the function declares first and sets
    // later, `let next; .. next = $next(z);` (ADR 0705).
    let mut first = HashMap::new();
    let mut at = 0;
    js::visit_stmts(body, &mut |name| {
        first.entry(name.to_string()).or_insert(at);
        at += 1;
    });
    renames.sort_by_key(|(from, ..)| first.get(from).copied().unwrap_or(usize::MAX));
    for (from, names, any) in renames {
        // The first name of what it holds no other has, else what any is
        // named, `value`, `arg` or `tmp`, given apart where it must be.
        let base = (names.into_iter())
            .find(|name| !taken.contains(name))
            .unwrap_or_else(|| any.to_string());
        let to = crate::names::fresh_in(&mut taken, &base);
        // Not a closure's own of its name, `(_2) => _2[0]` (ADR 0902).
        js::outside_shadows(body, &from, |body| {
            js::statement_lists(body, &mut |list| {
                for stmt in list.iter_mut() {
                    if let StmtKind::Const(name, _) | StmtKind::Let(name, _) = &mut stmt.kind
                        && *name == from
                    {
                        *name = to.clone();
                    }
                }
            });
            js::each_expr_mut(body, &mut |e| {
                if let ExprKind::Var(v) = &mut e.kind
                    && *v == from
                {
                    *v = to.clone();
                }
            });
        });
    }
}

/// `const a = e; .. const b = a;`, where nothing else reads `a`, is `const
/// b = e;` where `a` was: `b` is read nowhere before, which JS's `const`
/// says of its block. And `const b = a;` that a `clone()` made, `clones`,
/// of a parameter `a` nothing assigns or declares again, is `a`, read where
/// `b` was: the same value, an uncounted `Rc`'s clone the same object,
/// `payload` of `const resolved = payload` (ADR 0846); and of what a
/// closure captured by value, `kept`, which its maker never assigns
/// again, as fixed in it (ADR 0879). Of a variable a `let`'s
/// `#[rust_js::name]` names, `given`, and one it doesn't, `let made =
/// js::unknown_of(synthetic_base_event)`: the name the program gave, for
/// both (ADR 1009).
pub(super) fn aliases(
    body: &mut Vec<Stmt>,
    captured: &HashSet<String>,
    (params, kept): (&[js::Pattern], &HashSet<String>),
    (clones, given): (&HashSet<String>, &HashSet<String>),
) {
    let fixed: HashSet<String> = (params.iter())
        .flat_map(|param| param.names())
        .chain(kept.iter().map(String::as_str))
        .filter(|name| !rebinds(body, name))
        .map(str::to_string)
        .collect();
    js::statement_lists(body, &mut |stmts| {
        let mut i = 0;
        while i < stmts.len() {
            if let StmtKind::Const(b, value) = &stmts[i].kind
                && let ExprKind::Var(a) = &value.kind
                && fixed.contains(a)
                && clones.contains(b)
                && !rebinds(&stmts[i + 1..], b)
            {
                let (a, b) = (a.clone(), b.clone());
                stmts.remove(i);
                js::each_expr_mut(&mut stmts[i..], &mut |e| {
                    if let ExprKind::Var(v) = &mut e.kind
                        && *v == b
                    {
                        v.clone_from(&a);
                    }
                });
                continue;
            }
            // And of a field of one, `const fragmentFiber =
            // this.fragmentFiber;`, an `Rc`'s clone out of a cell's borrow,
            // read once, as the next statement's call's argument before
            // anything there that may write it: read there, in its place,
            // `traverse(this.fragmentFiber, (..) => ..)`, as upstream (rd189).
            if let StmtKind::Const(b, value) = &stmts[i].kind
                && clones.contains(b)
                && field_of(value).is_some_and(|root| root.is_none_or(|root| fixed.contains(root)))
                && js::mentions_in(&stmts[i + 1..], b) == 1
                && let Some(next) = stmts.get(i + 1)
                && first_read_as_argument(next, b)
            {
                let (value, b) = (value.clone(), b.clone());
                stmts.remove(i);
                js::each_expr_mut(&mut stmts[i..=i], &mut |e| {
                    if matches!(&e.kind, ExprKind::Var(v) if *v == b) {
                        *e = value.clone();
                    }
                });
                continue;
            }
            i += 1;
        }
    });
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
                let StmtKind::Const(a, _) = &mut stmts[i].kind else {
                    unreachable!("matched")
                };
                // A variable the program names, of one MIR's lowering made
                // for a prop, `const code = activeCode`, or a temporary,
                // `const _5 = props`: the variable, read where that was.
                let named = given.contains(a.as_str()) && !given.contains(&b);
                if !is_temporary(a) && (captured.contains(&b) || is_temporary(&b) || named) {
                    let a = a.clone();
                    js::each_expr_mut(&mut stmts[j..], &mut |e| {
                        if let ExprKind::Var(v) = &mut e.kind
                            && *v == b
                        {
                            v.clone_from(&a);
                        }
                    });
                } else {
                    *a = b;
                }
            }
            i += 1;
        }
    });
}

/// Of a field read, `a.b.c`, its root's name; of one of `this`, which
/// nothing assigns, `this.fragmentFiber`, none.
fn field_of(e: &Expr) -> Option<Option<&str>> {
    match &e.kind {
        ExprKind::Member(of, _) => match &of.kind {
            ExprKind::Var(root) => Some(Some(root)),
            ExprKind::This => Some(None),
            _ => field_of(of),
        },
        _ => None,
    }
}

/// Whether `stmt` is a call, given or its value returned or held, whose
/// argument `name` is, read before anything that may do something: its
/// callee and the arguments before it do nothing.
fn first_read_as_argument(stmt: &Stmt, name: &str) -> bool {
    let (StmtKind::Expr(call) | StmtKind::Return(Some(call)) | StmtKind::Const(_, call)) = &stmt.kind else {
        return false;
    };
    let ExprKind::Call(callee, args) = &call.kind else {
        return false;
    };
    let Some(at) = args
        .iter()
        .position(|arg| matches!(&arg.kind, ExprKind::Var(v) if v == name))
    else {
        return false;
    };
    !callee.has_effects() && args[..at].iter().all(|arg| !arg.has_effects())
}

/// A `let x;` MIR declares first, where it's first named: before the first
/// statement that names it, and in the one branch of an `if` that does; in
/// a loop's body that assigns it before it's read.
/// Whether `stmt` reads `name` where it's the variable, not only in a
/// closure whose parameter of that name shadows it, `(a, b) => ..`'s `b`.
fn reads_past_shadows(stmt: &Stmt, name: &str) -> bool {
    let mut unshadowed = [stmt.clone()];
    js::each_expr_mut(&mut unshadowed, &mut |e| {
        if let ExprKind::Arrow(params, _) | ExprKind::AsyncArrow(params, _) = &e.kind
            && params.iter().any(|p| p.names().contains(&name))
        {
            *e = Expr::undefined();
        }
    });
    js::mentions_in(&unshadowed, name) > 0
}

pub(super) fn sunk_lets(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        let StmtKind::Let(name, None) = &stmts[i].kind else {
            i += 1;
            continue;
        };
        let name = name.clone();
        let names = |stmt: &Stmt| js::mentions_in(std::slice::from_ref(stmt), &name) > 0;
        let Some(first) = (i + 1..stmts.len()).find(|&j| names(&stmts[j]) && reads_past_shadows(&stmts[j], &name))
        else {
            i += 1;
            continue;
        };
        let alone = !stmts[first + 1..].iter().any(names);
        let declared = stmts.remove(i);
        let first = first - 1;
        if alone
            && let StmtKind::If(test, then, otherwise) = &mut stmts[first].kind
            && !test.mentions_var(&name)
        {
            let branch = match (js::mentions_in(then, &name) > 0, otherwise) {
                (true, Some(otherwise)) if js::mentions_in(otherwise, &name) > 0 => None,
                (true, _) => Some(then),
                (false, Some(otherwise)) => Some(otherwise),
                (false, None) => None,
            };
            if let Some(branch) = branch {
                branch.insert(0, declared);
                continue;
            }
        }
        // Into a labeled block, a block.
        if alone && let StmtKind::Labeled(_, body) = &mut stmts[first].kind {
            body.insert(0, declared);
            continue;
        }
        // Into a loop whose body assigns it before anything reads it, each
        // turn: the variable Rust declares in the body, each turn its own.
        if alone
            && let StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } | StmtKind::While { body, .. } =
                &mut stmts[first].kind
            && assigned_first(body, &name, &[])
            && !looped_reads(&stmts[first], &name)
        {
            let (StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } | StmtKind::While { body, .. }) =
                &mut stmts[first].kind
            else {
                unreachable!("matched")
            };
            body.insert(0, declared);
            continue;
        }
        // Only `let`s between, where it is; otherwise, what followed it is
        // next.
        if stmts[i..first].iter().all(|s| matches!(s.kind, StmtKind::Let(_, None))) {
            stmts.insert(i, declared);
            i += 1;
        } else {
            stmts.insert(first, declared);
        }
    }
}

/// Whether `stmts` assign `name` before anything reads it, each way through
/// them that names it: first `name = e`, or a branch of an `if` that does
/// so, which is all that names it; or a loop or a labeled block that is all
/// that names it, whose body does so; or, of what's read after too, an
/// `if` or a labeled block each way on through which does so, or leaves the
/// turn first, `let y = match x { 0 => continue, .. }`'s (`assigned_on`).
/// `past`, of each block around, whether its `break` goes where nothing
/// reads it unassigned.
pub(super) fn assigned_first(stmts: &[Stmt], name: &str, past: &[(String, bool)]) -> bool {
    let names = |s: &Stmt| js::mentions_in(std::slice::from_ref(s), name) > 0;
    let Some(at) = stmts.iter().position(names) else {
        return false;
    };
    let alone = !stmts[at + 1..].iter().any(names);
    let assigned = |stmts: &[Stmt]| !stmts.iter().any(names) || assigned_first(stmts, name, past);
    match &stmts[at].kind {
        StmtKind::Assign(target, value) => {
            matches!(&target.kind, ExprKind::Var(v) if v == name) && !value.mentions_var(name)
        }
        StmtKind::If(test, then, otherwise) => {
            !test.mentions_var(name)
                && (alone && assigned(then) && otherwise.as_deref().is_none_or(assigned)
                    || assigned_on(&stmts[at..], name, false, past))
        }
        // A loop inside, all that names it, whose body assigns it first each
        // turn: nothing reads it but there.
        StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } | StmtKind::While { body, .. } => {
            alone && !looped_reads(&stmts[at], name) && assigned_first(body, name, past)
        }
        // A labeled block, all that names it, that does so, its `break`
        // going where nothing reads it; or one each of whose ways out does,
        // a `let`'s value MIR made in it, `l: { if (a) { x = 1; break l; } x
        // = 2; } f(x)`.
        StmtKind::Labeled(label, body) => {
            alone && assigned_first(body, name, &within(past, label, true))
                || assigned_on(&stmts[at..], name, false, past)
        }
        _ => false,
    }
}

/// `past` and the block `label`, whose `break` goes where `ok` says.
fn within(past: &[(String, bool)], label: &str, ok: bool) -> Vec<(String, bool)> {
    past.iter().cloned().chain([(label.to_string(), ok)]).collect()
}

/// Whether each way through `stmts` that goes on past them assigns `name`
/// before anything reads it, `after` whether a way that names nothing to
/// their end does so by what follows: one that leaves the turn first, by
/// `continue`, `return`, `throw` or the loop's own `break`, doesn't go on;
/// one that `break`s a block or a loop inside the turn goes where `past`
/// says, by its label, `""` the innermost loop's (`within`).
pub(super) fn assigned_on(stmts: &[Stmt], name: &str, after: bool, past: &[(String, bool)]) -> bool {
    let names = |s: &Stmt| js::mentions_in(std::slice::from_ref(s), name) > 0;
    // Where a `break` of `label` goes is fine: one of none here leaves the turn.
    let broken_ok = |label: &str| past.iter().rev().find(|(l, _)| l == label).is_none_or(|(_, ok)| *ok);
    for (k, stmt) in stmts.iter().enumerate() {
        if names(stmt) {
            let on = || assigned_on(&stmts[k + 1..], name, after, past);
            return match &stmt.kind {
                StmtKind::Assign(target, value) => {
                    matches!(&target.kind, ExprKind::Var(v) if v == name) && !value.mentions_var(name)
                }
                StmtKind::If(test, then, otherwise) => {
                    let on = on();
                    !test.mentions_var(name)
                        && assigned_on(then, name, on, past)
                        && assigned_on(otherwise.as_deref().unwrap_or_default(), name, on, past)
                }
                StmtKind::Labeled(label, body) => {
                    let on = on();
                    assigned_on(body, name, on, &within(past, label, on))
                }
                // A loop it leaves only by its `break`s, each of which goes on
                // set, or to what sets it: its end goes round again.
                StmtKind::While { cond, body, label } if matches!(cond.kind, ExprKind::Bool(true)) => {
                    let on = on();
                    let inner = within(past, "", on);
                    let inner = match label {
                        Some(label) => within(&inner, label, on),
                        None => inner,
                    };
                    assigned_on(body, name, true, &inner)
                }
                _ => false,
            };
        }
        match &stmt.kind {
            StmtKind::Continue(_) | StmtKind::Return(_) | StmtKind::Throw(_) => return true,
            StmtKind::Break(label) => return broken_ok(label.as_deref().unwrap_or_default()),
            _ => {}
        }
        // What it names nothing in may `break` a block or a loop it's in,
        // unset.
        let one = std::slice::from_ref(stmt);
        let innermost = past.iter().rposition(|(l, _)| l.is_empty());
        let unset = past.iter().enumerate().any(|(i, (label, ok))| {
            !ok && match label.as_str() {
                "" => Some(i) == innermost && js::breaks(one, None),
                label => js::names_break(one, label),
            }
        });
        if unset {
            return false;
        }
    }
    after
}

/// `let x; .. x = e;` is `.. let x = e;`, where nothing between names `x`,
/// nor does `e`; and a `let` nothing assigns after, a closure neither, is
/// a `const`.
pub(super) fn declarations(body: &mut Vec<Stmt>) {
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
    js::constants(body);
}

/// The names `stmts` declare, a destructuring's too, not in blocks inside
/// them.
pub(super) fn declared(stmts: &[Stmt]) -> impl Iterator<Item = &str> {
    stmts.iter().flat_map(|s| match &s.kind {
        StmtKind::Const(name, _) | StmtKind::Let(name, _) => vec![name.as_str()],
        StmtKind::Destructure { pattern, .. } => pattern.names(),
        _ => Vec::new(),
    })
}

/// The temporaries MIR declared once and assigned in more than one place,
/// `let _5; .. _5 = a; f(_5); .. _5 = b; g(_5);`, each assignment read
/// once, by the statement after it, and nowhere else: each is read where
/// it's made, as a `const` read once is (`inlined`, ADR 0564). Optimized
/// MIR's `bool`s are such, a test made and negated along each path to it.
pub(super) fn paired_temporaries(body: &[Stmt]) -> HashSet<String> {
    let mut mentions: HashMap<String, usize> = HashMap::new();
    js::visit_stmts(body, &mut |name| *mentions.entry(name.to_string()).or_default() += 1);
    let mut lets = HashSet::new();
    let mut pairs: HashMap<String, usize> = HashMap::new();
    let mut unpaired = HashSet::new();
    js::each_list(body, &mut |list| {
        for (k, stmt) in list.iter().enumerate() {
            match &stmt.kind {
                StmtKind::Let(name, None) if is_temporary(name) => {
                    lets.insert(name.clone());
                }
                StmtKind::Assign(target, value)
                    if let ExprKind::Var(name) = &target.kind
                        && is_temporary(name) =>
                {
                    let read_next =
                        (list.get(k + 1)).is_some_and(|s| js::mentions_in(std::slice::from_ref(s), name) == 1);
                    match !value.mentions_var(name) && read_next {
                        true => *pairs.entry(name.clone()).or_default() += 1,
                        false => {
                            unpaired.insert(name.clone());
                        }
                    }
                }
                _ => {}
            }
        }
    });
    (lets.into_iter())
        .filter(|name| !unpaired.contains(name))
        .filter(|name| pairs.get(name).is_some_and(|&n| mentions.get(name) == Some(&(2 * n))))
        .collect()
}

/// What `body`'s closures, the functions it declares and its `&mut`s'
/// handles read or set: not where they're written, but whenever they run.
pub(super) fn captured(body: &[Stmt]) -> HashSet<String> {
    let mut captured = HashSet::new();
    let mut insert = |name: &str| _ = captured.insert(name.to_string());
    js::each_expr(body, &mut |e| match &e.kind {
        ExprKind::Arrow(_, stmts) | ExprKind::AsyncArrow(_, stmts) => js::visit_stmts(stmts, &mut insert),
        ExprKind::Function(function) => js::visit_stmts(&function.body, &mut insert),
        ExprKind::Handle(place, _) | ExprKind::Pair(place, _) => place.visit_vars(&mut insert),
        _ => {}
    });
    js::each_list(body, &mut |stmts| {
        for stmt in stmts.iter() {
            if let StmtKind::Function(function) = &stmt.kind {
                js::visit_stmts(&function.body, &mut insert);
            }
        }
    });
    captured
}

/// `const node$1 = createNode(); cache.set(key, node$1); node = node$1;`, a
/// binding of a variable's own name, given apart, that the variable is
/// given after it, where nothing between names the variable: the variable
/// given it where it's made, `node = createNode(); cache.set(key, node);`,
/// as `node = match .. { None => { let node = make(); ..; node } .. }`
/// reads (ADR 0853). Only of a variable the body declares, or a parameter
/// of it: one a closure captures may be read after what's between throws,
/// as it was, by what called the closure. Not where a `break` or a
/// `continue` between may leave before the variable's given it, nor in a
/// `try`, whose `catch` or `finally` may read the variable as it was, nor
/// of what a closure or a `&mut`'s handle reads that's made before it's
/// given, which may run between: one made after can't, as it isn't yet,
/// `return () => listeners.delete(l)` after `listeners = new Set()`
/// (rd105), unless a loop around both makes it again first.
pub(super) fn given_last(params: &[js::Pattern], body: &mut Vec<Stmt>) {
    let mut declared: HashSet<String> = params.iter().flat_map(|p| p.names()).map(str::to_string).collect();
    declared_in(body, &mut declared);
    given_last_in(body, false, (&HashSet::new(), &declared));
}

/// What `stmts` declare, in their blocks too: not their closures', which
/// are their own bodies.
fn declared_in(stmts: &[Stmt], declared: &mut HashSet<String>) {
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::Const(name, _) | StmtKind::Let(name, _) | StmtKind::For { name, .. } => {
                declared.insert(name.clone());
            }
            StmtKind::Destructure { pattern, .. } => declared.extend(pattern.names().into_iter().map(str::to_string)),
            _ => {}
        }
        match &stmt.kind {
            StmtKind::If(_, a, b) => {
                declared_in(a, declared);
                if let Some(b) = b {
                    declared_in(b, declared);
                }
            }
            StmtKind::ForOf { pattern, body, .. } => {
                declared.extend(pattern.names().into_iter().map(str::to_string));
                declared_in(body, declared);
            }
            StmtKind::While { body, .. } | StmtKind::For { body, .. } | StmtKind::Labeled(_, body) => {
                declared_in(body, declared);
            }
            StmtKind::Switch { cases, default, .. } => {
                js::case_lists(cases, default).for_each(|body| declared_in(body, declared));
            }
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                declared_in(a, declared);
                declared_in(b, declared);
            }
            _ => {}
        }
    }
}

/// `given_last` of `stmts`, and of each block in them: in a `try`'s, none.
/// `made` is what's captured by what's made before `stmts` run, and of the
/// statements around them, a loop's again before them.
fn given_last_in(stmts: &mut Vec<Stmt>, in_try: bool, (made, declared): (&HashSet<String>, &HashSet<String>)) {
    if !in_try {
        given_first(stmts, (made, declared));
    }
    let mut made = made.clone();
    for stmt in stmts.iter_mut() {
        made.extend(captured(std::slice::from_ref(&*stmt)));
        let names = (&made, declared);
        match &mut stmt.kind {
            StmtKind::If(_, a, b) => {
                given_last_in(a, in_try, names);
                if let Some(b) = b {
                    given_last_in(b, in_try, names);
                }
            }
            StmtKind::While { body, .. }
            | StmtKind::For { body, .. }
            | StmtKind::ForOf { body, .. }
            | StmtKind::Labeled(_, body) => given_last_in(body, in_try, names),
            StmtKind::Switch { cases, default, .. } => {
                for body in js::case_lists_mut(cases, default) {
                    given_last_in(body, in_try, names);
                }
            }
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                given_last_in(a, true, names);
                given_last_in(b, true, names);
            }
            _ => {}
        }
    }
}

/// `given_last` of the block `stmts` itself: each variable given a binding
/// made before it in the block, the binding named nowhere after.
fn given_first(stmts: &mut Vec<Stmt>, (made, declared): (&HashSet<String>, &HashSet<String>)) {
    let mut given = 0;
    while given < stmts.len() {
        let Some((at, x, y)) = given_from(stmts, given, (made, declared)) else {
            given += 1;
            continue;
        };
        let StmtKind::Const(_, made) = &mut stmts[at].kind else {
            unreachable!("`given_from`'s `const`")
        };
        let made = std::mem::replace(made, Expr::undefined());
        stmts[at].kind = StmtKind::Assign(Expr::var(&x), made);
        stmts.remove(given);
        js::each_expr_mut(&mut stmts[at + 1..given], &mut |e| {
            if matches!(&e.kind, ExprKind::Var(v) if *v == y) {
                *e = Expr::var(&x).or_at(e.span);
            }
        });
    }
}

/// Of `stmts[given]`, `x = y;`, `given_first`'s: where `y` is made, `x`
/// and `y`. What's captured is `made`'s, and what the block makes before.
fn given_from(
    stmts: &[Stmt],
    given: usize,
    (made, declared): (&HashSet<String>, &HashSet<String>),
) -> Option<(usize, String, String)> {
    let StmtKind::Assign(target, value) = &stmts[given].kind else {
        return None;
    };
    let (ExprKind::Var(x), ExprKind::Var(y)) = (&target.kind, &value.kind) else {
        return None;
    };
    if !kin(x, y) || !declared.contains(x) {
        return None;
    }
    let captured = captured(&stmts[..given]);
    let early = |name: &str| made.contains(name) || captured.contains(name);
    if early(x) || early(y) {
        return None;
    }
    let at = (stmts[..given].iter()).position(|s| matches!(&s.kind, StmtKind::Const(n, _) if n == y))?;
    let StmtKind::Const(_, made) = &stmts[at].kind else {
        return None;
    };
    let between = &stmts[at + 1..given];
    let kept = made.mentions_var(x)
        || js::mentions_in(between, x) > 0
        || jumps(between)
        || js::mentions_in(&stmts[given + 1..], y) > 0;
    (!kept).then(|| (at, x.clone(), y.clone()))
}

/// Whether `apart` is `name` given apart, `node$1` of `node` or of `node$2`.
fn kin(name: &str, apart: &str) -> bool {
    let base = crate::names::apart(name).map_or(name, |(base, _)| base);
    crate::names::apart(apart).is_some_and(|(of, _)| of == base)
}

/// Whether a `break` or a `continue` is anywhere in `stmts`.
fn jumps(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Break(_) | StmtKind::Continue(_) => true,
        StmtKind::If(_, a, b) => jumps(a) || b.as_deref().is_some_and(jumps),
        StmtKind::While { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Labeled(_, body) => jumps(body),
        StmtKind::Switch { cases, default, .. } => js::case_lists(cases, default).any(|body| jumps(body)),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => jumps(a) || jumps(b),
        _ => false,
    })
}

/// `const cache$1 = node.o; let cache; if (!cache$1) { cache = new
/// WeakMap(); node.o = cache; } else { cache = cache$1; }`, a variable
/// given what's read before, the other way given its own: the variable
/// given what's read, `let cache = node.o; if (!cache) { cache = new
/// WeakMap(); node.o = cache; }`, as a cache filled the first time it's
/// read, `match node.o.get() { None => { ..; cache } Some(cache) => cache
/// }`, reads (ADR 0853). What's read is named as the variable is, given
/// apart, and nowhere else. The other way reads the variable only once
/// it's given its value, as Rust reads none before, so it's never read as
/// what's read.
pub(super) fn filled_lazily(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i + 2 < stmts.len() {
        if let Some((read, var)) = lazily_filled(stmts, i) {
            let at = match &stmts[i].kind {
                StmtKind::Const(..) => i,
                _ => i + 1,
            };
            let StmtKind::Const(_, made) = &stmts[at].kind else {
                unreachable!("a `const` of what's read")
            };
            let declared = StmtKind::Let(var.clone(), Some(made.clone())).at(stmts[at].span);
            let StmtKind::If(test, then, Some(otherwise)) =
                std::mem::replace(&mut stmts[i + 2].kind, StmtKind::Comments)
            else {
                unreachable!("the `if` `lazily_filled` matched")
            };
            let test = (test.substitute(&|v| (v == read).then(|| Expr::var(&var)))).unwrap_or(test);
            stmts[i + 2].kind = match gives_read(&otherwise, &var, &read) {
                true => StmtKind::If(test, then, None),
                false => StmtKind::If(test.negated_test(), otherwise, None),
            };
            stmts.splice(i..i + 2, [declared]);
        }
        i += 1;
    }
}

/// Of `stmts[i..]`, `filled_lazily`'s: what's read and the variable.
fn lazily_filled(stmts: &[Stmt], i: usize) -> Option<(String, String)> {
    let (read, var) = match (&stmts[i].kind, &stmts[i + 1].kind) {
        (StmtKind::Const(read, _), StmtKind::Let(var, None)) | (StmtKind::Let(var, None), StmtKind::Const(read, _)) => {
            (read, var)
        }
        _ => return None,
    };
    let StmtKind::If(test, then, Some(otherwise)) = &stmts[i + 2].kind else {
        return None;
    };
    let other = match (gives_read(then, var, read), gives_read(otherwise, var, read)) {
        (false, true) => then.as_slice(),
        (true, false) => otherwise.as_slice(),
        _ => return None,
    };
    // What's read named as the variable is, given apart: not a binding of
    // the program's own name, `fnNode`, which stays.
    let kept = !kin(var, read)
        || test.mentions_var(var)
        || js::mentions_in(other, read) > 0
        || js::mentions_in(&stmts[i + 3..], read) > 0;
    (!kept).then(|| (read.clone(), var.clone()))
}

/// Whether `branch` is `var = read;` alone.
fn gives_read(branch: &[Stmt], var: &str, read: &str) -> bool {
    matches!(branch, [s] if matches!(&s.kind, StmtKind::Assign(t, v)
        if matches!(&t.kind, ExprKind::Var(n) if n == var) && matches!(&v.kind, ExprKind::Var(n) if n == read)))
}
