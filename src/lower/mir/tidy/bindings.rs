//! Tidy's bindings: declarations, aliases, what's read where it's made,
//! and temporaries and their names.

use super::*;

/// `const x = [loud(1), 2]` nothing reads: each part that does something, in
/// turn, `loud(1);`, as the array, made here, is never seen; and `const x =
/// f()`, what a call gave, its `fmt::Result` say: `f();`.
pub(super) fn unread_made(stmts: &mut Vec<Stmt>, mentions: &HashMap<String, usize>) {
    let mut i = 0;
    while i < stmts.len() {
        if let StmtKind::Const(name, value) = &stmts[i].kind
            && !mentions.contains_key(name)
            && let ExprKind::Array(parts) = &value.kind
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
pub(super) fn is_temporary(name: &str) -> bool {
    name.strip_prefix('_')
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit() || c == '$'))
}

/// `const _5 = children;`, a temporary of a variable nothing changes, nor it:
/// `children` wherever it's read.
pub(super) fn alias_variables(body: &mut Vec<Stmt>, changing: &HashSet<String>) {
    let mut aliases: Vec<(String, Expr)> = Vec::new();
    js::statement_lists(body, &mut |stmts| {
        stmts.retain(|stmt| match &stmt.kind {
            StmtKind::Const(name, value)
                if is_temporary(name)
                    && !changing.contains(name)
                    && matches!(&value.kind, ExprKind::Var(of) if !changing.contains(of)) =>
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
/// time.
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

/// `if (raw) { const error = raw; .. }`, a pattern's binding of a variable,
/// `if let Some(error) = raw`, which MIR copies where `raw` is borrowed:
/// `raw` where `error` is read, where the block sets
/// neither again, declares neither, nor reads `error` in a closure, which
/// may run after; a `let`-`else`'s too, after the test that leaves.
pub(super) fn copied_bindings(stmts: &mut Vec<Stmt>) {
    for stmt in stmts.iter_mut() {
        let StmtKind::If(test, then, _) = &mut stmt.kind else {
            continue;
        };
        let Some(Stmt {
            kind: StmtKind::Const(b, value),
            ..
        }) = then.first()
        else {
            continue;
        };
        let ExprKind::Var(a) = &value.kind else {
            continue;
        };
        // What the test says is there, `raw` of `raw && ..`, `!!raw` or `raw
        // != null`.
        let mut first = &*test;
        while let ExprKind::Binary(Op::And, x, _) = &first.kind {
            first = x;
        }
        if let ExprKind::Unary(UnaryOp::Not, x) = &first.kind
            && let ExprKind::Unary(UnaryOp::Not, x) = &x.kind
        {
            first = x;
        }
        let present = match &first.kind {
            ExprKind::Var(v) => v == a,
            _ => js::null_test(first, true).is_some_and(|v| matches!(&v.kind, ExprKind::Var(v) if v == a)),
        };
        let (a, b) = (a.clone(), b.clone());
        if !present || is_temporary(&a) || is_temporary(&b) || a == b {
            continue;
        }
        let rest = &then[1..];
        if rebinds(rest, &a) || rebinds(rest, &b) || in_closure(rest, &b) {
            continue;
        }
        then.remove(0);
        js::each_expr_mut(then, &mut |e| {
            if let ExprKind::Var(v) = &mut e.kind
                && *v == b
            {
                *v = a.clone();
            }
        });
    }
    let mut i = 0;
    while i + 1 < stmts.len() {
        if let StmtKind::If(test, then, None) = &stmts[i].kind
            && then.last().is_some_and(ends)
            && let StmtKind::Const(b, value) = &stmts[i + 1].kind
            && let ExprKind::Var(a) = &value.kind
            && absent(test, a)
            && !is_temporary(a)
            && !is_temporary(b)
            && a != b
        {
            let (a, b) = (a.clone(), b.clone());
            let rest = &stmts[i + 2..];
            if !rebinds(rest, &a) && !rebinds(rest, &b) && !in_closure(rest, &b) {
                stmts.remove(i + 1);
                js::each_expr_mut(&mut stmts[i + 1..], &mut |e| {
                    if let ExprKind::Var(v) = &mut e.kind
                        && *v == b
                    {
                        *v = a.clone();
                    }
                });
            }
        }
        i += 1;
    }
}

/// Whether `stmts` set `name` again, or declare it, a closure's in them too.
pub(super) fn rebinds(stmts: &[Stmt], name: &str) -> bool {
    let mut found = false;
    js::statement_lists(&mut stmts.to_vec(), &mut |list| {
        for stmt in list.iter() {
            found |= match &stmt.kind {
                StmtKind::Const(n, _) | StmtKind::Let(n, _) => n == name,
                StmtKind::Destructure { pattern, .. } | StmtKind::ForOf { pattern, .. } => {
                    pattern.names().contains(&name)
                }
                StmtKind::For { name: n, .. } => n == name,
                StmtKind::Assign(target, _) => matches!(&target.kind, ExprKind::Var(v) if v == name),
                _ => false,
            };
        }
    });
    js::each_expr_mut(&mut stmts.to_vec(), &mut |e| {
        if let ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) = &e.kind {
            found |= params.iter().any(|p| p.names().contains(&name)) || rebinds(body, name);
        }
    });
    found
}

/// Whether `stmts` set something of `name`'s, `name[0] = ..`, a closure's
/// in them too.
pub(super) fn sets_through(stmts: &[Stmt], name: &str) -> bool {
    let mut set = false;
    js::statement_lists(&mut stmts.to_vec(), &mut |list| {
        for stmt in list.iter() {
            if let StmtKind::Assign(target, _) = &stmt.kind {
                target.visit_vars(&mut |v| set |= v == name);
            }
        }
    });
    js::each_expr_mut(&mut stmts.to_vec(), &mut |e| {
        if let ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) = &e.kind {
            set |= sets_through(body, name);
        }
    });
    set
}

/// Whether a closure in `stmts` reads `name`.
pub(super) fn in_closure(stmts: &[Stmt], name: &str) -> bool {
    let mut found = false;
    js::each_expr_mut(&mut stmts.to_vec(), &mut |e| {
        if let ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) = &e.kind {
            found |= js::mentions_in(body, name) > 0;
        }
    });
    found
}

/// `_4`, a temporary MIR holds still, read more than once: named after the
/// field it reads, `routes` of `tree.routes`, a trait object's whose method
/// is called, `receiver`, else `value`, or `tmp` of a `let` its branches
/// set, a name no other in the function has, its parameters' and closures'
/// too.
pub(in crate::lower::mir) fn readable_temporaries(params: &[js::Pattern], body: &mut Vec<Stmt>) {
    let mut taken: HashSet<String> = params.iter().flat_map(|p| p.names()).map(str::to_string).collect();
    js::names_in(body, &mut taken);
    // What an element is given each as, `href={_5}` or a child, `{_6}`: the
    // prop's name, `href`, or `children`.
    let mut given: HashMap<String, String> = HashMap::new();
    js::each_expr_mut(&mut body.clone(), &mut |e| {
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
                    let base = match (given.get(name), &value.kind) {
                        (Some(prop), _) => prop.clone(),
                        (None, _) if is_held_arg(name) => "arg".to_string(),
                        (None, ExprKind::Member(_, field)) => field.clone(),
                        (None, _) => "value".to_string(),
                    };
                    renames.push((name.clone(), base));
                }
                StmtKind::Let(name, _) if is_temporary(name) => renames.push((name.clone(), "tmp".to_string())),
                _ => {}
            }
        }
    });
    for (from, base) in renames {
        let to = crate::names::fresh_in(&mut taken, &base);
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
    }
}

/// A temporary MIR made, `const _5 = e;`, read once, by the statement
/// after it, or after those that are moved there too: `e` where it's read, where nothing that runs before it there
/// does anything, or anywhere outside a closure if `e` does nothing and
/// reads nothing that changes.
/// `set`, the variables a closure sets: what a call can change of them.
pub(super) fn inlined(
    stmts: &mut Vec<Stmt>,
    changing: &HashSet<String>,
    set: &HashSet<String>,
    plain: &HashSet<String>,
    captured: &HashSet<String>,
) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        let (head, rest) = stmts.split_at_mut(i + 1);
        let StmtKind::Const(name, value) = &head[i].kind else {
            i += 1;
            continue;
        };
        let temporary = is_temporary(name) || captured.contains(name);
        // What's read before it: what does nothing, where it does nothing
        // either; else only constants and variables nothing changes, which
        // what it does can't change. Of what an element made ahead of a
        // branch reads, what does nothing and reads no such variable: the
        // element's tree is the order the program makes them in.
        // What does nothing and reads what no call changes, a `let` no
        // closure sets, passes what does something too.
        let acts = value.has_effects();
        let element = captured.contains(name);
        let unchanged_by_calls = !acts && read_past_calls(value, set, plain);
        let passes = |e: &Expr| match acts {
            false => unchanged_by_calls || !e.has_effects(),
            true if element => {
                let mut unchanged = !e.has_effects();
                e.visit_vars(&mut |v| unchanged &= !set.contains(v));
                unchanged
            }
            true => read_past_calls(e, set, plain),
        };
        // The statement that reads it, past `const`s of what it may be read
        // past, `const list$1 = list;` a callback's borrow.
        let at = (rest.iter()).position(|s| js::mentions_in(std::slice::from_ref(s), name) > 0);
        // And what does nothing and reads what no call changes, past any
        // statement that assigns nothing it reads, a loop's too.
        let mut reads = Vec::new();
        value.visit_vars(&mut |v| reads.push(v.to_string()));
        let past = |at: usize| {
            (rest[..at].iter()).all(|s| matches!(&s.kind, StmtKind::Const(_, v) if passes(v)))
                || (unchanged_by_calls && reads.iter().all(|v| !assigns(&rest[..at], v)))
        };
        let Some(at) = at.filter(|&at| past(at)) else {
            i += 1;
            continue;
        };
        if !temporary || js::mentions_in(rest, name) != 1 || js::mentions_in(&rest[at..=at], name) != 1 {
            i += 1;
            continue;
        }
        let mut stable = !value.has_effects();
        value.visit_vars(&mut |v| stable &= !changing.contains(v));
        let read = match &mut rest[at].kind {
            StmtKind::Const(_, e)
            | StmtKind::Let(_, Some(e))
            | StmtKind::Expr(e)
            | StmtKind::Return(Some(e))
            | StmtKind::Throw(e)
            | StmtKind::Destructure { value: e, .. } => Some(e),
            StmtKind::Assign(target, e) if !target.has_effects() => {
                let mut in_target = false;
                target.visit_vars(&mut |v| in_target |= v == name);
                // What's set of it, `_25[k] = v`, `_25` what does nothing:
                // JS reads a target's object first, as it was read.
                let mut root = &*target;
                while let ExprKind::Index(of, _) | ExprKind::Member(of, _) = &root.kind {
                    root = of;
                }
                let set_of = matches!(&root.kind, ExprKind::Var(v) if v == name) && !value.has_effects();
                match (in_target, set_of) {
                    (false, _) => Some(e),
                    (true, true) => Some(target),
                    (true, false) => None,
                }
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
        // One made where a test holds, `t ? e : undefined`, may be read where
        // that test holds.
        let guard = match &value.kind {
            ExprKind::Cond(test, _, no) if matches!(no.kind, ExprKind::Undefined) => Some(&**test),
            _ => None,
        };
        if !stable && leads(read, name, guard, &passes) != Some(true) {
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
pub(super) fn leads(e: &Expr, name: &str, guard: Option<&Expr>, passes: &dyn Fn(&Expr) -> bool) -> Option<bool> {
    let mut reads = false;
    e.visit_vars(&mut |v| reads |= v == name);
    if !reads {
        return None;
    }
    let in_order = |items: &mut dyn Iterator<Item = &Expr>| {
        for item in items {
            match leads(item, name, guard, passes) {
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
        | ExprKind::Await(a) => leads(a, name, guard, passes),
        ExprKind::Index(a, b) => in_order(&mut [&**a, &**b].into_iter()),
        // Of what runs where its own test holds, `t ? e : undefined`, `b` of
        // `t && b` too: `e` runs there just where it ran.
        ExprKind::Binary(Op::And, a, b) | ExprKind::Cond(a, b, _)
            if guard.is_some_and(|g| same_test(a, g)) && leads(a, name, guard, passes).is_none() && passes(a) =>
        {
            leads(b, name, guard, passes)
        }
        // What runs only as a test says is `b`, `c` of `a ? b : c`.
        ExprKind::Binary(Op::And | Op::Or | Op::Coalesce, a, _) | ExprKind::Cond(a, ..) => {
            leads(a, name, guard, passes).or(Some(false))
        }
        ExprKind::Binary(_, a, b) => in_order(&mut [&**a, &**b].into_iter()),
        ExprKind::Array(items) | ExprKind::Template(_, items, _) => in_order(&mut items.iter()),
        ExprKind::Call(f, args) | ExprKind::New(f, args) => in_order(&mut std::iter::once(&**f).chain(args)),
        ExprKind::Object(props) => in_order(&mut props.iter().map(|p| {
            let (js::Prop::Field(_, v) | js::Prop::Getter(_, v) | js::Prop::Setter(_, v) | js::Prop::Spread(v)) = p;
            v
        })),
        ExprKind::Jsx(jsx) => {
            let tag = match &jsx.tag {
                js::JsxTag::Component(tag) => Some(tag),
                _ => None,
            };
            let props = jsx.props.iter().map(|p| {
                let (js::Prop::Field(_, v) | js::Prop::Getter(_, v) | js::Prop::Setter(_, v) | js::Prop::Spread(v)) = p;
                v
            });
            // JS's JSX runtime makes `key` after the children, and
            // `createElement` before them: read there, after both.
            let key = (jsx.props.iter()).find_map(|p| match p {
                js::Prop::Field(k, v) if k == "key" => Some(v),
                _ => None,
            });
            if let Some(key) = key
                && let Some(found) = leads(key, name, guard, passes)
            {
                let others = props.filter(|v| !std::ptr::eq(*v, key));
                let mut before = tag.into_iter().chain(others).chain(&jsx.children);
                return Some(found && before.all(|e| leads(e, name, guard, passes).is_none() && passes(e)));
            }
            in_order(&mut tag.into_iter().chain(props).chain(&jsx.children))
        }
        _ => Some(false),
    }
}

/// `const a = e; .. const b = a;`, where nothing else reads `a`, is `const
/// b = e;` where `a` was: `b` is read nowhere before, which JS's `const`
/// says of its block.
pub(super) fn aliases(body: &mut Vec<Stmt>, captured: &HashSet<String>) {
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
                if !is_temporary(a) && (captured.contains(&b) || is_temporary(&b)) {
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

/// A `let x;` MIR declares first, where it's first named: before the first
/// statement that names it, and in the one branch of an `if` that does; in
/// a loop's body that assigns it before it's read.
pub(super) fn sunk_lets(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        let StmtKind::Let(name, None) = &stmts[i].kind else {
            i += 1;
            continue;
        };
        let name = name.clone();
        let names = |stmt: &Stmt| js::mentions_in(std::slice::from_ref(stmt), &name) > 0;
        let Some(first) = (i + 1..stmts.len()).find(|&j| names(&stmts[j])) else {
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
            && assigned_first(body, &name)
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
/// so, which is all that names it; or an `if` each of whose branches does
/// so or leaves the turn first, `let y = match x { 0 => continue, .. }`'s.
pub(super) fn assigned_first(stmts: &[Stmt], name: &str) -> bool {
    let names = |s: &Stmt| js::mentions_in(std::slice::from_ref(s), name) > 0;
    let Some(at) = stmts.iter().position(names) else {
        return false;
    };
    let alone = !stmts[at + 1..].iter().any(names);
    let assigned = |stmts: &[Stmt]| !stmts.iter().any(names) || assigned_first(stmts, name);
    match &stmts[at].kind {
        StmtKind::Assign(target, value) => {
            matches!(&target.kind, ExprKind::Var(v) if v == name) && !value.mentions_var(name)
        }
        StmtKind::If(test, then, otherwise) => {
            !test.mentions_var(name)
                && (alone && assigned(then) && otherwise.as_deref().is_none_or(assigned)
                    || assigned_on(then, name) && otherwise.as_deref().is_some_and(|o| assigned_on(o, name)))
        }
        _ => false,
    }
}

/// Whether each way through `stmts` that goes on past them assigns `name`
/// before anything reads it: one that leaves the turn first, by `continue`,
/// `return`, `throw` or the loop's own `break`, doesn't go on. A labeled
/// `break` may go on, to the end of a block in the turn.
pub(super) fn assigned_on(stmts: &[Stmt], name: &str) -> bool {
    let names = |s: &Stmt| js::mentions_in(std::slice::from_ref(s), name) > 0;
    for stmt in stmts {
        if names(stmt) {
            return assigned_first(std::slice::from_ref(stmt), name);
        }
        if matches!(
            stmt.kind,
            StmtKind::Continue(_) | StmtKind::Return(_) | StmtKind::Throw(_) | StmtKind::Break(None)
        ) {
            return true;
        }
    }
    false
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

/// The variables `body` assigns, and each a handle is on, which its
/// setter assigns (ADR 0099).
pub(super) fn assigned(body: &mut Vec<Stmt>) -> HashSet<String> {
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
    assigned.extend(handled(body));
    assigned
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
