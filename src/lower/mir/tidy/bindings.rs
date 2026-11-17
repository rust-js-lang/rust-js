//! Tidy's bindings: declarations, aliases, what's read where it's made,
//! and temporaries and their names.

use super::*;

mod given;
mod temporaries;
pub(super) use given::given_last;
pub(super) use temporaries::readable_temporaries;

/// A `let` declared first, in the body or a block of it, that nothing's
/// left to assign or read; or only a block that declares it again, an
/// index loop's item (ADR 1042); and a function made that nothing reads;
/// `mentions`, how many times `b` reads each variable.
pub(super) fn unused_lets(b: &mut Vec<Stmt>, mentions: IndexMap<js::Name, usize>) {
    let mut declared: IndexMap<js::Name, usize> = IndexMap::default();
    js::each_list(b, &mut |s| {
        for stmt in s {
            if let StmtKind::Let(name, _) | StmtKind::Const(name, _) = &stmt.kind {
                *declared.entry(name.clone()).or_default() += 1;
            }
        }
    });
    js::statement_lists(b, &mut |s| {
        let unread: Vec<js::Name> = (s.iter())
            .filter_map(|stmt| match &stmt.kind {
                StmtKind::Let(name, None)
                    if !mentions.contains_key(name)
                        || declared.get(name).is_some_and(|&n| n > 1) && redeclared_only(s, name) =>
                {
                    Some(name.clone())
                }
                _ => None,
            })
            .collect();
        s.retain(|stmt| !matches!(&stmt.kind, StmtKind::Let(name, None) if unread.contains(name)));
        // A function made that nothing reads, which making does
        // nothing: a binding's value MIR held in a variable of
        // its function's type, whose `&` is read by the type,
        // `make(&text_of)` of a thread-local's initializer (rd542);
        // or read, the program's, an instance's `noopU32` (#974).
        s.retain(|stmt| {
            !matches!(&stmt.kind, StmtKind::Const(name, value)
                if !mentions.contains_key(name) && (matches!(value.kind, js::ExprKind::Arrow(..) | js::ExprKind::AsyncArrow(..))
                        || matches!(value.kind, js::ExprKind::Var(_)) && value.facts.has(js::Known::Item)))
        })
    })
}

/// Whether each read and write of `name` in `stmts`, past its own `let
/// name;`, is in a block that declares it again, a loop's item, `const row
/// = v[i];`, so none is of the `let`'s.
fn redeclared_only(stmts: &[Stmt], name: &js::Name) -> bool {
    let mut rest: Vec<Stmt> = (stmts.iter())
        .filter(|s| !matches!(&s.kind, StmtKind::Let(n, None) if n == name))
        .cloned()
        .collect();
    js::statement_lists(&mut rest, &mut |list| {
        if list
            .iter()
            .any(|s| matches!(&s.kind, StmtKind::Let(n, _) | StmtKind::Const(n, _) if n == name))
        {
            list.clear();
        }
    });
    js::mentions_of(&rest, name) == 0
}

/// `const x = [loud(1), 2]` nothing reads: each part that does something, in
/// turn, `loud(1);`, as the array, made here, is never seen; `const x =
/// f()`, what a call gave, its `fmt::Result` say: `f();`; and `const old =
/// task.callback; task.callback = next;`, the old value a `replace` read of
/// the place it writes, a copy apart of it too, `$apart(dest.value)` (#527):
/// `task.callback = next;`. Not another read, which may be a binding's
/// getter, `el.offsetWidth`, which lays the page out.
pub(super) fn unread_made(stmts: &mut Vec<Stmt>, mentions: &IndexMap<js::Name, usize>) {
    let mut i = 0;
    while i < stmts.len() {
        if let StmtKind::Const(name, value) = &stmts[i].kind
            && !mentions.contains_key(name)
            && !value.has_effects()
            && let read = match &value.kind {
                ExprKind::Call(callee, args) if callee.helper_name() == Some("$apart") && args.len() == 1 => &args[0],
                _ => value,
            }
            && matches!(stmts.get(i + 1).map(|next| &next.kind),
                Some(StmtKind::Assign(target, _)) if target.kind == read.kind)
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
        // An array's iterator nothing steps, a `Splice` dropped where it's
        // made, `$iter($splice(v, a, b, xs))`: what making the array does,
        // as `$iter` of an array reads only what it's given (rd341).
        if let StmtKind::Expr(value) = &stmts[i].kind
            && let Some(items) = crate::lower::iterators::fresh_iter_source(value)
        {
            match items.has_effects() {
                true => stmts[i].kind = StmtKind::Expr(items.clone()),
                false => {
                    stmts.remove(i);
                    continue;
                }
            }
        }
        i += 1;
    }
}

/// Whether `stmts` read `name` alone, `name[i]` or `name.length`: not
/// assign it, call a method of it, or give it to what could change it.
pub(super) fn only_read(stmts: &[Stmt], name: &js::Name) -> bool {
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
    all == read && !js::Writes::sets(&stmts, name)
}

/// A temporary MIR made, by its local's index, `_5`: the one spelling of
/// one, which `is_temporary` knows, and no name the program gives has
/// (`user_name`).
pub(in crate::lower::mir) use crate::lower::temporary_names::temporary;

use crate::js::Name;
use crate::lower::temporary_names::spelled_as_held_arg;
pub(in crate::lower::mir) use crate::lower::temporary_names::spelled_as_temporary;

/// A name the program gives a local, as JS's: one spelled as a temporary's,
/// `let _5`, is `_5_`, so its JS reads as no temporary's.
pub(in crate::lower::mir) fn user_name(name: &str) -> String {
    match spelled_as_temporary(name) {
        true => format!("{name}_"),
        false => name.to_string(),
    }
}

/// Whether `name` is MIR's lowering's of an argument it holds, `_arg`, or
/// one given apart from it (`Name::held_arg`).
pub(super) fn is_held_arg(name: &Name) -> bool {
    checked(name, name.is_held_arg(), spelled_as_held_arg(name.word()))
}

/// Whether `name` is a temporary MIR made, `_5`, or one given apart from
/// one, `_5$1`, not one the program names (`Name::temporary`).
pub(in crate::lower::mir) fn is_temporary(name: &Name) -> bool {
    checked(
        name,
        name.is_temporary() || name.is_unread(),
        spelled_as_temporary(name.word()) || name.is("_"),
    )
}

/// `typed`, a name's form's answer, which is tidy's; where
/// `RUST_JS_CHECK_NAMES` is set, in a debug build, checked against
/// `spelled`, what its word says, the answer before names had forms (#1681
/// change 3, S4): a name made of text, a `String`'s, where a name of its
/// form should go on, is found where they differ.
pub(in crate::lower::mir) fn checked(name: &Name, typed: bool, spelled: bool) -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    // A name not spelled yet, `value#1`, has no word to say it.
    if cfg!(debug_assertions)
        && *ON.get_or_init(|| std::env::var_os("RUST_JS_CHECK_NAMES").is_some())
        && !name.is_unspelled()
    {
        assert_eq!(typed, spelled, "`{name}`: its form and its word disagree");
    }
    typed
}

/// `const _5 = children;`, a temporary of a variable: `children` where it's
/// read, before `children` is set again, as in `match name.as_str()` of a
/// `let mut name`; one read nowhere, gone. A closure neither reads a
/// temporary, its captures its own `const`s, nor sets what one's of between,
/// which Rust's borrows forbid. A `&mut`'s handle, or a `dyn`'s pair, reads
/// its place whenever it's read, past its last mention: one whose place
/// reads a temporary keeps it where what it's of is set anywhere, `const
/// _10 = index` of `lanes[_10]` before `index++` (#968).
pub(super) fn alias_variables(body: &mut Vec<Stmt>, fixed: &IndexSet<String>) {
    let set = js::Writes::of(body).replaced;
    js::statement_lists(body, &mut |stmts| {
        let mut i = 0;
        while i < stmts.len() {
            if let StmtKind::Const(alias, value) = &stmts[i].kind
                && (is_temporary(alias) || fixed.contains(alias.word()))
                && let ExprKind::Var(of) = &value.kind
            {
                let last = (i + 1..stmts.len())
                    .rev()
                    .find(|&j| js::mentions_of(&stmts[j..=j], alias) > 0);
                // What it's of set nowhere from it to its last read, or
                // set only past each read on every way, `const _15 =
                // newLast; if (_15 == null) { .. } else { _15.next = clone;
                // newLast = clone; }`.
                let deferred = set.contains(of) && handles_read(&stmts[i + 1..]).contains(alias);
                let kept = deferred
                    || last.is_some_and(|last| {
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
                                // What MIR said of the read's `None`, `null`
                                // where it's stored (`nulled::stored_nulled`),
                                // is so of the variable it's of there.
                                let none = e.facts.only_none();
                                *e = of.clone().or_at(e.span).or_at(at);
                                if let Some(none) = none {
                                    e.facts = e.facts.with(none);
                                }
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
pub(super) fn read_before_set(stmts: &[Stmt], alias: &js::Name, of: &js::Name, set: &mut bool) -> bool {
    let reads = |stmts: &[Stmt]| js::mentions_of(stmts, alias) > 0;
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
            && js::mentions_of(&stmts[i + 1..=i + 1], x) == 0
            && js::mentions_of(&stmts[i + 3..], t) == 0
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
pub(super) fn last_assigns(stmts: &[Stmt], t: &js::Name) -> bool {
    let names = |s: &Stmt| js::mentions_of(std::slice::from_ref(s), t) > 0;
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
        StmtKind::Assign(target, value) => {
            matches!(&target.kind, ExprKind::Var(v) if v == t) && !value.mentions_name(t)
        }
        StmtKind::If(test, then, otherwise) => {
            !test.mentions_name(t) && last_assigns(then, t) && otherwise.as_deref().is_some_and(|o| last_assigns(o, t))
        }
        StmtKind::TryCatch(body, _, caught) => last_assigns(body, t) && last_assigns(caught, t),
        _ => false,
    }
}

/// `const t = q.title;` of plain data, which nothing changes, held by MIR,
/// a temporary or what an element reads: `q.title` where `t` is read, each
/// time. Not a read a getter may answer, which may give another value each
/// time it's read (`Expr::reads_same`, #189).
pub(super) fn plain_reads(stmts: &mut Vec<Stmt>, plain: &IndexSet<js::Path>, captured: &IndexSet<js::Name>) {
    let mut i = 0;
    while i < stmts.len() {
        let read = match &stmts[i].kind {
            StmtKind::Const(name, value) if is_temporary(name) || captured.contains(name) => {
                (value.reads_same() && super::plain_field(value, plain)).then(|| (name.clone(), value.clone()))
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
    let mut declared: IndexMap<String, usize> = IndexMap::default();
    js::each_list(body, &mut |stmts| {
        for stmt in stmts.iter() {
            let names = match &stmt.kind {
                StmtKind::Const(name, _) | StmtKind::Let(name, _) => vec![name.word()],
                StmtKind::Destructure { pattern, .. } => pattern.names(),
                _ => Vec::new(),
            };
            for name in names {
                *declared.entry(name.to_string()).or_default() += 1;
            }
        }
    });
    let mut reads: IndexMap<String, usize> = IndexMap::default();
    js::visit_stmts(body, &mut |name| *reads.entry(name.to_string()).or_default() += 1);
    js::statement_lists(body, &mut |stmts| {
        // What it takes goes from after it: the list shortens.
        let mut next = 0;
        while next < stmts.len() {
            let i = next;
            next += 1;
            // Or one set once in the `try` it's first written in, a drop
            // guard's, `let _8; try { _8 = descriptorOf(name); } catch ..`,
            // and never again: as a `const` from there on (#444).
            let a = match &stmts[i].kind {
                StmtKind::Const(a, _) => a.clone(),
                StmtKind::Let(a, None) if set_once_in_try(&stmts[i + 1..], a) => a.clone(),
                // Or the one name a `const` takes apart, a cell's value read
                // while its guard is, `const { value: _4 } = $borrow(cell)`,
                // `let Some(chunk) = cell.borrow().clone() else ..`'s (#452).
                StmtKind::Destructure {
                    pattern,
                    mutable: false,
                    ..
                } => match pattern.bound()[..] {
                    [only] => only.clone(),
                    _ => continue,
                },
                _ => continue,
            };
            if !is_temporary(&a) {
                continue;
            }
            let mut found = Vec::new();
            taking(&stmts[i + 1..], &a, &mut found);
            // One name, each arm's that binds it, `n` of `(n, Some(x))` and
            // `(n, None)`, given apart until they're named in their scopes,
            // `n` and `n$1` (ADR 0357): each declared nowhere else, and read
            // only after it.
            let Some((b, _)) = found.first() else { continue };
            let base = |name: &str| name.split('$').next().unwrap_or(name).to_string();
            if found.iter().any(|(other, after)| {
                base(other.word()) != base(b.word())
                    || declared.get(other.word()) != Some(&1)
                    || reads.get(other.word()).copied().unwrap_or(0) != *after
            }) {
                continue;
            }
            // A `let` set in a `try` takes the name in its own, wider scope:
            // not where another of that name is declared, `max` of a
            // destructuring beside it, which would be `max$1` then.
            let set_in_try = matches!(stmts[i].kind, StmtKind::Let(..));
            if set_in_try
                && (declared.keys())
                    .any(|name| base(name) == base(b.word()) && !found.iter().any(|(other, _)| other.is(name)))
            {
                continue;
            }
            let b = b.clone();
            let taken: Vec<js::Name> = found.iter().map(|(name, _)| name.clone()).collect();
            for name in &taken {
                dropping(stmts, &a, name);
            }
            match &mut stmts[i].kind {
                StmtKind::Const(name, _) | StmtKind::Let(name, _) => *name = b.clone(),
                StmtKind::Destructure { pattern, .. } => pattern.each_name_mut(&mut |name| *name = b.clone()),
                _ => unreachable!("matched"),
            }
            // Each read of them, by its id: a closure's own parameter of a
            // name, `(_2) => _2[0]` where `_2` is a temporary here too, is
            // another variable, never one of these (#1105; ADR 1150).
            for name in std::iter::once(&a).chain(&taken) {
                js::each_expr_mut(&mut stmts[i + 1..], &mut |e| {
                    if let ExprKind::Var(v) = &mut e.kind
                        && v == name
                    {
                        *v = b.clone();
                    }
                });
            }
        }
    });
}

/// Whether `a`, declared `let a;` before `stmts`, is first written in a
/// `try` of them, `try { a = e; } catch ..`, a drop guard's, set there
/// and mentioned nowhere else in it, and never set again: what's after it
/// reads one value, as of a `const`.
pub(super) fn set_once_in_try(stmts: &[Stmt], a: &js::Name) -> bool {
    let mentions = |s: &Stmt| js::mentions_of(std::slice::from_ref(s), a);
    let Some(j) = stmts.iter().position(|s| mentions(s) > 0) else {
        return false;
    };
    let set = match &stmts[j].kind {
        StmtKind::Try(body, _) | StmtKind::TryCatch(body, _, _) => body.iter().find(|s| mentions(s) > 0).is_some_and(
            |s| matches!(&s.kind, StmtKind::Assign(target, _) if matches!(&target.kind, ExprKind::Var(v) if v == a)),
        ),
        _ => false,
    };
    set && mentions(&stmts[j]) == 1 && !js::Writes::of(&stmts[j + 1..]).may_replace(a)
}

/// Each `const b = a;` of a name the program gives in the blocks of
/// `stmts`, not in their closures, and how often `b` is read after it.
pub(super) fn taking(stmts: &[Stmt], a: &js::Name, found: &mut Vec<(js::Name, usize)>) {
    for (j, stmt) in stmts.iter().enumerate() {
        match &stmt.kind {
            StmtKind::Const(b, value) if !is_temporary(b) && matches!(&value.kind, ExprKind::Var(v) if v == a) => {
                found.push((b.clone(), js::mentions_of(&stmts[j + 1..], b)));
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
pub(super) fn dropping(stmts: &mut Vec<Stmt>, a: &js::Name, b: &js::Name) {
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
pub(super) fn guarded_lets(stmts: &mut Vec<Stmt>, captured: &IndexSet<js::Name>) {
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
                    && js::mentions_of(&stmts[i + 2..], x) == 1 =>
            {
                Some((x.clone(), Expr::cond(test.clone(), value.clone(), Expr::js_undefined())))
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
pub(super) fn rebinds(stmts: &[Stmt], name: &js::Name) -> bool {
    binds(stmts, name, true)
}

/// Whether `stmts` declare `name` again, a closure's parameter too, as
/// `rebinds` says, but for what assigns it.
pub(super) fn declares(stmts: &[Stmt], name: &js::Name) -> bool {
    binds(stmts, name, false)
}

fn binds(stmts: &[Stmt], name: &js::Name, assigned: bool) -> bool {
    bindings(stmts, name, assigned) > 0
}

/// How many times `stmts` declare `name`, a closure's parameter too, and
/// set it, where `assigned`: as `binds` finds them.
fn bindings(stmts: &[Stmt], name: &js::Name, assigned: bool) -> usize {
    let own = |stmts: &[Stmt]| {
        let mut found = 0;
        js::each_list(stmts, &mut |list| {
            for stmt in list.iter() {
                found += usize::from(match &stmt.kind {
                    StmtKind::Const(n, _) | StmtKind::Let(n, _) => n == name,
                    StmtKind::Destructure { pattern, .. } | StmtKind::ForOf { pattern, .. } => {
                        pattern.bound().contains(&name)
                    }
                    StmtKind::For { name: n, .. } => n == name,
                    StmtKind::Assign(target, _) => assigned && matches!(&target.kind, ExprKind::Var(v) if v == name),
                    _ => false,
                });
            }
        });
        found
    };
    let mut found = own(stmts);
    js::each_expr(stmts, &mut |e| {
        if let ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) = &e.kind {
            found += (params.iter()).filter(|p| p.bound().contains(&name)).count() + own(body);
        }
    });
    found
}

/// Whether a closure in `stmts` reads `name`.
pub(super) fn in_closure(stmts: &[Stmt], name: &js::Name) -> bool {
    let mut found = false;
    js::each_expr(stmts, &mut |e| {
        if let ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) = &e.kind {
            found |= js::mentions_of(body, name) > 0;
        }
    });
    found
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
    captured: &IndexSet<js::Name>,
    (params, kept): (&[js::Pattern], &IndexSet<js::Name>),
    (clones, given): (&crate::lower::mir::hints::Clones, &IndexSet<js::Name>),
) {
    clone_aliases(body, (params, kept), clones);
    js::statement_lists(body, &mut |stmts| {
        let mut i = 0;
        while i < stmts.len() {
            let alias = match &stmts[i].kind {
                StmtKind::Const(a, _) => (i + 1..stmts.len())
                    .find(|&j| js::mentions_of(&stmts[j..=j], a) > 0)
                    .filter(|&j| {
                        matches!(&stmts[j].kind, StmtKind::Const(_, value)
                            if matches!(&value.kind, ExprKind::Var(read) if read == a))
                            && js::mentions_of(&stmts[j..], a) == 1
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
                let named = given.contains(a) && !given.contains(&b);
                if !is_temporary(a) && (captured.contains(&b) || is_temporary(&b) || named) {
                    let a = a.clone();
                    js::each_expr_mut(&mut stmts[j..], &mut |e| {
                        if let ExprKind::Var(v) = &mut e.kind
                            && *v == b
                        {
                            *v = a.clone();
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

/// `const b = a;` that a `clone()` made, `clones`, of a parameter `a`
/// nothing assigns or declares again, or of what's fixed in a closure,
/// `kept`, is `a`, read where `b` was (ADRs 0846, 0879, amended): `aliases`'
/// first rewrite, and again once a tuple of clones is read part by part,
/// `const chunk = this` of `let (chunk, resolved) = (Rc::clone(self), ..)`
/// (rd515).
pub(super) fn clone_aliases(
    body: &mut Vec<Stmt>,
    (params, kept): (&[js::Pattern], &IndexSet<js::Name>),
    clones: &crate::lower::mir::hints::Clones,
) {
    let fixed: IndexSet<js::Name> = (params.iter())
        .flat_map(|param| param.bound())
        .chain(kept)
        .filter(|name| !rebinds(body, name))
        .cloned()
        .collect();
    clones_read_as(body, &fixed, &clones.all);
}

/// `const b = a;` that a `clone()` of an uncounted `Rc` made,
/// `clones.rcs`, of a variable `a` the body declares once and never
/// replaces, its closures neither (`Writes`), `const response =
/// createResponse(options)`, is `a`, read where `b` was: the same object
/// (the user's decision, rd616; ADR 0846, amended). Once the passes before
/// have read in place what they read in place, a binding of a pattern's
/// guard among them, `const old = current;` whose `current` is set again.
pub(super) fn variable_clones(body: &mut Vec<Stmt>, clones: &crate::lower::mir::hints::Clones) {
    let writes = js::Writes::of(body);
    let fixed: IndexSet<js::Name> = (cloned_from(body, &clones.rcs).into_iter())
        .filter(|(_, a)| bindings(body, a, false) == 1 && !writes.may_replace(a))
        .map(|(_, a)| a)
        .collect();
    clones_read_as(body, &fixed, &clones.rcs);
}

/// Each `const b = a;` of `body`'s own lists, `b` one of `clones` and `a`
/// one of `fixed`, gone, `b` read as `a` after it.
fn clones_read_as(body: &mut Vec<Stmt>, fixed: &IndexSet<js::Name>, clones: &IndexSet<String>) {
    js::statement_lists(body, &mut |stmts| {
        let mut i = 0;
        while i < stmts.len() {
            if let StmtKind::Const(b, value) = &stmts[i].kind
                && let ExprKind::Var(a) = &value.kind
                && fixed.contains(a)
                && clones.contains(b.word())
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
            i += 1;
        }
    });
}

/// Each `const b = a;` of `body`'s own lists that a `clone()` made,
/// `clones`: `b` and `a`.
fn cloned_from(body: &mut Vec<Stmt>, clones: &IndexSet<String>) -> Vec<(js::Name, js::Name)> {
    let mut found = Vec::new();
    js::statement_lists(body, &mut |stmts| {
        for stmt in stmts.iter() {
            if let StmtKind::Const(b, value) = &stmt.kind
                && let ExprKind::Var(a) = &value.kind
                && clones.contains(b.word())
            {
                found.push((b.clone(), a.clone()));
            }
        }
    });
    found
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
        let names = |stmt: &Stmt| js::mentions_of(std::slice::from_ref(stmt), &name) > 0;
        let Some(first) = (i + 1..stmts.len()).find(|&j| names(&stmts[j])) else {
            i += 1;
            continue;
        };
        let alone = !stmts[first + 1..].iter().any(names);
        let declared = stmts.remove(i);
        let first = first - 1;
        if alone
            && let StmtKind::If(test, then, otherwise) = &mut stmts[first].kind
            && !test.mentions_name(&name)
        {
            let branch = match (js::mentions_of(then, &name) > 0, otherwise) {
                (true, Some(otherwise)) if js::mentions_of(otherwise, &name) > 0 => None,
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

/// `chain`, an `if` and its `else if`s that `let name;` is declared just
/// before, nothing after reading it: each branch that names it given its
/// own, `if (op === 0) { const x = l; .. } else if (l === 3) { const x = op;
/// .. }`, as Rust binds `x` in each alternative of `(0, x) | (x, 3)` (#771),
/// where each assigns it before anything reads it and no test names it.
/// Whether it is so: otherwise `chain` is as it was.
pub(super) fn declared_in_branches(name: &js::Name, chain: &mut Stmt) -> bool {
    fn branches<'s>(stmt: &'s mut Stmt, name: &js::Name, out: &mut Vec<&'s mut Vec<Stmt>>) -> bool {
        let StmtKind::If(test, then, otherwise) = &mut stmt.kind else {
            return false;
        };
        if test.mentions_name(name) {
            return false;
        }
        out.push(then);
        match otherwise {
            Some(list) => {
                let nested = matches!(list.as_slice(), [s] if matches!(s.kind, StmtKind::If(..)));
                if nested {
                    branches(&mut list[0], name, out)
                } else {
                    out.push(list);
                    true
                }
            }
            None => true,
        }
    }
    let mut all = Vec::new();
    let walked = branches(chain, name, &mut all);
    let named: Vec<&mut Vec<Stmt>> = all.into_iter().filter(|b| js::mentions_of(b, name) > 0).collect();
    let ok = walked && named.len() >= 2 && named.iter().all(|b| assigned_first(b, name, &[]));
    if !ok {
        return false;
    }
    for branch in named {
        branch.insert(0, StmtKind::Let(name.clone(), None).at(js::Span::NONE));
        declarations(branch);
    }
    true
}

/// Whether `stmts` assign `name` before anything reads it, each way through
/// them that names it: first `name = e`, or a branch of an `if` that does
/// so, which is all that names it; or a loop or a labeled block that is all
/// that names it, whose body does so; or, of what's read after too, an
/// `if` or a labeled block each way on through which does so, or leaves the
/// turn first, `let y = match x { 0 => continue, .. }`'s (`assigned_on`).
/// `past`, of each block around, whether its `break` goes where nothing
/// reads it unassigned.
pub(super) fn assigned_first(stmts: &[Stmt], name: &js::Name, past: &[(Option<js::Name>, bool)]) -> bool {
    let names = |s: &Stmt| js::mentions_of(std::slice::from_ref(s), name) > 0;
    let Some(at) = stmts.iter().position(names) else {
        return false;
    };
    let alone = !stmts[at + 1..].iter().any(names);
    let assigned = |stmts: &[Stmt]| !stmts.iter().any(names) || assigned_first(stmts, name, past);
    match &stmts[at].kind {
        StmtKind::Assign(target, value) => {
            matches!(&target.kind, ExprKind::Var(v) if v == name) && !value.mentions_name(name)
        }
        StmtKind::If(test, then, otherwise) => {
            !test.mentions_name(name)
                && (alone && assigned(then) && otherwise.as_deref().is_none_or(assigned)
                    || assigned_on(&stmts[at..], name, false, past))
        }
        // A loop inside, all that names it, whose body assigns it first each
        // turn: nothing reads it but there.
        StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } | StmtKind::While { body, .. } => {
            alone && !looped_reads(&stmts[at], name) && assigned_first(body, name, past)
        }
        // A labeled block, all that names it, that does so, its `break`
        // going where nothing reads it, as one of a label `past` doesn't
        // name does; or one each of whose ways out does, a `let`'s value
        // MIR made in it, `l: { if (a) { x = 1; break l; } x = 2; } f(x)`.
        StmtKind::Labeled(_, body) => {
            alone && assigned_first(body, name, past) || assigned_on(&stmts[at..], name, false, past)
        }
        _ => false,
    }
}

/// `past` and the block `label`, whose `break` goes where `ok` says.
fn within(past: &[(Option<js::Name>, bool)], label: Option<&js::Name>, ok: bool) -> Vec<(Option<js::Name>, bool)> {
    past.iter().cloned().chain([(label.cloned(), ok)]).collect()
}

/// Whether each way through `stmts` that goes on past them assigns `name`
/// before anything reads it, `after` whether a way that names nothing to
/// their end does so by what follows: one that leaves the turn first, by
/// `continue`, `return`, `throw` or the loop's own `break`, doesn't go on;
/// one that `break`s a block or a loop inside the turn goes where `past`
/// says, by its label, `""` the innermost loop's (`within`).
pub(super) fn assigned_on(stmts: &[Stmt], name: &js::Name, after: bool, past: &[(Option<js::Name>, bool)]) -> bool {
    let names = |s: &Stmt| js::mentions_of(std::slice::from_ref(s), name) > 0;
    // Where a `break` of `label` goes is fine: one of none here leaves the turn.
    let broken_ok = |label: Option<&js::Name>| {
        past.iter()
            .rev()
            .find(|(l, _)| l.as_ref() == label)
            .is_none_or(|(_, ok)| *ok)
    };
    for (k, stmt) in stmts.iter().enumerate() {
        if names(stmt) {
            let on = || assigned_on(&stmts[k + 1..], name, after, past);
            return match &stmt.kind {
                StmtKind::Assign(target, value) => {
                    matches!(&target.kind, ExprKind::Var(v) if v == name) && !value.mentions_name(name)
                }
                StmtKind::If(test, then, otherwise) => {
                    let on = on();
                    !test.mentions_name(name)
                        && assigned_on(then, name, on, past)
                        && assigned_on(otherwise.as_deref().unwrap_or_default(), name, on, past)
                }
                StmtKind::Labeled(label, body) => {
                    let on = on();
                    assigned_on(body, name, on, &within(past, Some(label), on))
                }
                // A loop it leaves only by its `break`s, each of which goes on
                // set, or to what sets it: its end goes round again.
                StmtKind::While { cond, body, label } if matches!(cond.kind, ExprKind::Bool(true)) => {
                    let on = on();
                    let inner = within(past, None, on);
                    let inner = match label {
                        Some(label) => within(&inner, Some(label), on),
                        None => inner,
                    };
                    assigned_on(body, name, true, &inner)
                }
                _ => false,
            };
        }
        match &stmt.kind {
            StmtKind::Continue(_) | StmtKind::Return(_) | StmtKind::Throw(_) => return true,
            StmtKind::Break(label) => return broken_ok(label.as_ref()),
            _ => {}
        }
        // What it names nothing in may `break` a block or a loop it's in,
        // unset.
        let one = std::slice::from_ref(stmt);
        let innermost = past.iter().rposition(|(l, _)| l.is_none());
        let unset = past.iter().enumerate().any(|(i, (label, ok))| {
            !ok && match label {
                None => Some(i) == innermost && js::breaks(one, None),
                Some(label) => js::names_break(one, label),
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
                    .find(|&j| js::mentions_of(&stmts[j..=j], name) > 0)
                    .filter(|&j| {
                        matches!(&stmts[j].kind, StmtKind::Assign(target, _)
                            if matches!(&target.kind, ExprKind::Var(assigned) if assigned == name))
                            && js::mentions_of(&stmts[j..=j], name) == 1
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
/// them, their words (`declared_names`).
pub(super) fn declared(stmts: &[Stmt]) -> impl Iterator<Item = &str> {
    declared_names(stmts).map(Name::word)
}

/// The names `stmts` declare, a destructuring's too, not in blocks inside
/// them.
pub(super) fn declared_names(stmts: &[Stmt]) -> impl Iterator<Item = &Name> {
    stmts.iter().flat_map(|s| match &s.kind {
        StmtKind::Const(name, _) | StmtKind::Let(name, _) => vec![name],
        StmtKind::Destructure { pattern, .. } => pattern.bound(),
        _ => Vec::new(),
    })
}

/// The temporaries MIR declared once and assigned in more than one place,
/// `let _5; .. _5 = a; f(_5); .. _5 = b; g(_5);`, each assignment read
/// once, by the statement after it, and nowhere else: each is read where
/// it's made, as a `const` read once is (`inlined`, ADR 0564). Optimized
/// MIR's `bool`s are such, a test made and negated along each path to it.
pub(super) fn paired_temporaries(body: &[Stmt]) -> IndexSet<String> {
    let mut mentions: IndexMap<String, usize> = IndexMap::default();
    js::visit_stmts(body, &mut |name| *mentions.entry(name.to_string()).or_default() += 1);
    let mut lets = IndexSet::default();
    let mut pairs: IndexMap<String, usize> = IndexMap::default();
    let mut unpaired = IndexSet::default();
    js::each_list(body, &mut |list| {
        for (k, stmt) in list.iter().enumerate() {
            match &stmt.kind {
                StmtKind::Let(name, None) if is_temporary(name) => {
                    lets.insert(name.to_string());
                }
                StmtKind::Assign(target, value)
                    if let ExprKind::Var(name) = &target.kind
                        && is_temporary(name) =>
                {
                    let read_next =
                        (list.get(k + 1)).is_some_and(|s| js::mentions_of(std::slice::from_ref(s), name) == 1);
                    match !value.mentions_name(name) && read_next {
                        true => *pairs.entry(name.to_string()).or_default() += 1,
                        false => {
                            unpaired.insert(name.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
    });
    (lets.into_iter())
        .filter(|name| !unpaired.contains(name.as_str()))
        .filter(|name| {
            pairs
                .get(name.as_str())
                .is_some_and(|&n| mentions.get(name.as_str()) == Some(&(2 * n)))
        })
        .collect()
}

/// What `body`'s handles and pairs read, whenever their `value` is (#968).
pub(super) fn handles_read(body: &[Stmt]) -> IndexSet<js::Name> {
    let mut read = IndexSet::default();
    js::each_expr(body, &mut |e| {
        if let ExprKind::Handle(place, _) | ExprKind::Pair(place, ..) = &e.kind {
            place.visit_names(&mut |name| _ = read.insert(name.clone()));
        }
    });
    read
}

/// What `body`'s closures, the functions it declares and its `&mut`s'
/// handles read or set: not where they're written, but whenever they run.
pub(super) fn captured(body: &[Stmt]) -> IndexSet<js::Name> {
    let mut captured = IndexSet::default();
    let mut insert = |name: &js::Name| _ = captured.insert(name.clone());
    js::each_expr(body, &mut |e| match &e.kind {
        // Not one its call runs alone while it runs, which reads what it
        // captures before its statement ends (#1168).
        ExprKind::Arrow(..) if e.facts.has(js::Known::RunsInItsCall) => {}
        ExprKind::Arrow(_, stmts) | ExprKind::AsyncArrow(_, stmts) => js::visit_stmt_names(stmts, &mut insert),
        ExprKind::Function(function) => js::visit_stmt_names(&function.body, &mut insert),
        _ => {}
    });
    js::each_list(body, &mut |stmts| {
        for stmt in stmts.iter() {
            if let StmtKind::Function(function) = &stmt.kind {
                js::visit_stmt_names(&function.body, &mut insert);
            }
        }
    });
    captured.extend(handles_read(body));
    captured
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
            let test = (test.substitute(&|v| (v == read).then(|| Expr::var(var.clone()))))
                .expect("`lazily_filled` renames the test");
            stmts[i + 2].kind = match gives_read(&otherwise, var.word(), &read) {
                true => StmtKind::If(test, then, None),
                false => StmtKind::If(test.negated_test(), otherwise, None),
            };
            stmts.splice(i..i + 2, [declared]);
        }
        i += 1;
    }
}

/// Of `stmts[i..]`, `filled_lazily`'s: what's read and the variable.
fn lazily_filled(stmts: &[Stmt], i: usize) -> Option<(String, js::Name)> {
    let (read, var) = match (&stmts[i].kind, &stmts[i + 1].kind) {
        (StmtKind::Const(read, _), StmtKind::Let(var, None)) | (StmtKind::Let(var, None), StmtKind::Const(read, _)) => {
            (read, var)
        }
        _ => return None,
    };
    let StmtKind::If(test, then, Some(otherwise)) = &stmts[i + 2].kind else {
        return None;
    };
    let other = match (
        gives_read(then, var.word(), read.word()),
        gives_read(otherwise, var.word(), read.word()),
    ) {
        (false, true) => then.as_slice(),
        (true, false) => otherwise.as_slice(),
        _ => return None,
    };
    // What's read named as the variable is, given apart: not a binding of
    // the program's own name, `fnNode`, which stays.
    // Nor where a closure in the test reads it, which `substitute` can't
    // rename, so the test would read a `const` no longer declared (#1525).
    let renamed = test.substitute(&|v| read.is(v).then(|| Expr::var(var))).is_some();
    let kept = !read.given_apart_of(var)
        || test.mentions_name(var)
        || js::mentions_of(other, read) > 0
        || js::mentions_of(&stmts[i + 3..], read) > 0
        || !renamed;
    (!kept).then(|| (read.to_string(), var.clone()))
}

/// Whether `branch` is `var = read;` alone.
fn gives_read(branch: &[Stmt], var: &str, read: &str) -> bool {
    matches!(branch, [s] if matches!(&s.kind, StmtKind::Assign(t, v)
        if matches!(&t.kind, ExprKind::Var(n) if n.is(var)) && matches!(&v.kind, ExprKind::Var(n) if n.is(read))))
}

/// Of `moved`, statements made a branch of an `if` that `after` follows:
/// the names of each late `let x;` among them that `after` reads, to be
/// declared before the `if` instead, as the program wrote them, in its
/// scope; `None` where another declaration `after` reads would be made the
/// branch's, `const v = ..`, which can't be (rd578, ADR 0237, amended).
pub(super) fn kept_declarations(moved: &[Stmt], after: &[Stmt]) -> Option<Vec<js::Name>> {
    let mut kept = Vec::new();
    for stmt in moved {
        for name in js::declared_here(stmt) {
            if js::mentions_in(after, &name) == 0 {
                continue;
            }
            match &stmt.kind {
                StmtKind::Let(declared, None) => kept.push(declared.clone()),
                _ => return None,
            }
        }
    }
    Some(kept)
}

/// `kept`'s late `let`s, `kept_declarations`', taken out of `moved` and
/// declared in `stmts` before its `at`th: how many.
pub(super) fn declared_before(stmts: &mut Vec<Stmt>, at: usize, moved: &mut Vec<Stmt>, kept: &[js::Name]) -> usize {
    moved.retain(|s| !matches!(&s.kind, StmtKind::Let(name, None) if kept.iter().any(|k| name == k)));
    let declared = kept
        .iter()
        .map(|name| StmtKind::Let(name.clone(), None).at(js::Span::NONE));
    stmts.splice(at..at, declared);
    kept.len()
}
