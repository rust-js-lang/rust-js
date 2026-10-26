//! Tidy's destructuring: parts read apart, a tuple's and a struct's, as
//! one pattern.

use super::*;

/// `const _5 = e; const a = _5[0]; const b = _5[1];`, the temporary read
/// nowhere else, is `const [a, b] = e;`; and of its properties, `const { a,
/// b: c } = e;`; `let` if one of them is. Of a variable the program names,
/// two or more parts in a row, `let (a, b) = t;`'s: `const [a, b] = t;`;
/// one too of a `let`'s pattern, `patterns`, `const { message } = error;`.
pub(super) fn destructured(
    stmts: &mut Vec<Stmt>,
    (patterns, lets, user): (&HashSet<String>, &HashSet<String>, &HashSet<String>),
) {
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
        let moved = constants_first(stmts, i, first, &name);
        i += moved;
        let first = first + moved;
        let made = first > i;
        let mut parts = parts(&stmts[first..], &name);
        // Of a variable the program names, the names its patterns give: a
        // `let` of a name alone, `let init = lazy.init;`, a statement the
        // program wrote as one, ends them, as MIR's temporary of one did,
        // `let port = channel.port2(); channel.port1()..` (ADR 0598,
        // amended).
        if !made && let Some(k) = parts.iter().position(|(_, bound, ..)| lets.contains(bound)) {
            parts.truncate(k);
        }
        // Nor a name rust-js gave, a temporary's, `_2`, or of what an object
        // made later is made of, `value` of `Cell::new(u.a.get())`'s `{ value
        // }`: no name of the program's, read where it's used, `{ lane:
        // update.lane }`, once it's moved there (ADR 0598, amended).
        if !made && let Some(k) = parts.iter().position(|(_, bound, ..)| !user.contains(bound)) {
            parts.truncate(k);
        }
        let count = parts.len();
        let indexed = parts.iter().all(|(p, ..)| p.is_ok());
        let bound = parts.iter().all(|(_, name, ..)| patterns.contains(name));
        let enough = count >= if made || bound { 1 } else { 2 };
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

/// Each `let` of a constant among the parts of `name` read from `first`,
/// a drop flag's, `let a$live = true;`, moved before `at`, so the parts are
/// read in a row, `const a = _1[0]; const b = _1[1];`, one pattern, a
/// `let`'s or a parameter's. What it declares nothing before it can read.
/// How many moved.
fn constants_first(stmts: &mut Vec<Stmt>, at: usize, first: usize, name: &str) -> usize {
    let mut end = first;
    let mut constants = Vec::new();
    while let Some(stmt) = stmts.get(end) {
        match &stmt.kind {
            StmtKind::Let(bound, Some(value)) if value.is_constant() && bound != name => constants.push(end),
            StmtKind::Const(_, value) | StmtKind::Let(_, Some(value)) if part_read(value, name).is_some() => {}
            _ => break,
        }
        end += 1;
    }
    // Those after the last part stay where they are.
    while constants.last().map(|&k| k + 1) == Some(end) {
        constants.pop();
        end -= 1;
    }
    let moved: Vec<Stmt> = constants.iter().rev().map(|&k| stmts.remove(k)).collect();
    let count = moved.len();
    stmts.splice(at..at, moved.into_iter().rev());
    count
}

/// `const [_18, _19] = e; a = _18; b = _19;`, a tuple assigned to the
/// variables the program has, `(a, b) = e`, its temporaries read nowhere
/// else: `[a, b] = e;`, a destructuring assignment, as JS writes one, which
/// makes `e` whole before it sets any of them, as the temporaries did.
pub(super) fn assigned_parts(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        let StmtKind::Destructure {
            pattern: js::Pattern::Array(items),
            mutable: false,
            ..
        } = &stmts[i].kind
        else {
            i += 1;
            continue;
        };
        let parts: Option<Vec<String>> = (items.iter())
            .map(|item| match item {
                Some(js::Pattern::Name(name)) if is_temporary(name) => Some(name.clone()),
                _ => None,
            })
            .collect();
        // Each part assigned in the statements after it, in its order.
        let targets: Option<Vec<String>> = parts.as_ref().and_then(|parts| {
            (parts.iter().enumerate())
                .map(|(k, part)| match stmts.get(i + 1 + k).map(|s| &s.kind) {
                    Some(StmtKind::Assign(
                        Expr {
                            kind: ExprKind::Var(target),
                            ..
                        },
                        Expr {
                            kind: ExprKind::Var(read),
                            ..
                        },
                    )) if read == part => Some(target.clone()),
                    _ => None,
                })
                .collect()
        });
        let (Some(parts), Some(targets)) = (parts, targets) else {
            i += 1;
            continue;
        };
        let distinct = (targets.iter()).all(|t| targets.iter().filter(|u| *u == t).count() == 1);
        let after = i + 1 + parts.len();
        if !distinct || parts.iter().any(|part| js::mentions_in(&stmts[after..], part) > 0) {
            i += 1;
            continue;
        }
        let StmtKind::Destructure { value, .. } = std::mem::replace(&mut stmts[i].kind, StmtKind::Break(None)) else {
            unreachable!("matched")
        };
        stmts[i].kind = StmtKind::Assign(Expr::array(targets.iter().map(|t| Expr::var(t)).collect()), value);
        stmts.drain(i + 1..after);
        i += 1;
    }
}

/// A `let`'s pattern's rest, which MIR's lowering takes apart with each
/// other field named apart, `const { href: _href, ...rest } = p;`, beside
/// what the pattern binds of the same, `const { href } = p;`: one pattern,
/// `const { href, ...rest } = p;`, as it's written.
pub(super) fn rest_parts(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        let StmtKind::Destructure {
            pattern: js::Pattern::Object(fields, Some(_)),
            value,
            ..
        } = &stmts[i].kind
        else {
            i += 1;
            continue;
        };
        let ExprKind::Var(of) = &value.kind else {
            i += 1;
            continue;
        };
        let parts_at = |at: usize| match stmts.get(at).map(|s| &s.kind) {
            Some(StmtKind::Destructure {
                pattern: js::Pattern::Object(parts, None),
                value,
                mutable,
            }) if matches!(&value.kind, ExprKind::Var(v) if v == of)
                && parts.iter().all(|(key, _, _)| fields.iter().any(|(k, _, _)| k == key)) =>
            {
                Some((parts.clone(), *mutable))
            }
            _ => None,
        };
        let (at, (parts, mutable)) = match (i.checked_sub(1).and_then(parts_at), parts_at(i + 1)) {
            (Some(before), _) => (i - 1, before),
            (None, Some(after)) => (i + 1, after),
            (None, None) => {
                i += 1;
                continue;
            }
        };
        stmts.remove(at);
        let i = if at < i { i - 1 } else { i };
        if let StmtKind::Destructure {
            pattern: js::Pattern::Object(fields, _),
            mutable: m,
            ..
        } = &mut stmts[i].kind
        {
            for (key, part, default) in parts {
                if let Some(field) = fields.iter_mut().find(|(k, _, _)| *k == key) {
                    *field = (key, part, default);
                }
            }
            *m |= mutable;
        }
    }
}

/// A part of a value read into a variable: its index or key, the variable,
/// and whether it's a `let`.
/// And a key's default, `{ size = "md" }`'s.
pub(super) type Part = (Result<usize, String>, String, bool, Option<Expr>);

/// The parts of `name` the first of `stmts` read, each a `const` or a
/// `let` of one, `const a = name[0];` or `const b = name.b;`.
pub(super) fn parts(stmts: &[Stmt], name: &str) -> Vec<Part> {
    let mut parts: Vec<Part> = Vec::new();
    for stmt in stmts {
        let (bound, value, mutable) = match &stmt.kind {
            StmtKind::Const(bound, value) => (bound, value, false),
            StmtKind::Let(bound, Some(value)) => (bound, value, true),
            _ => break,
        };
        if bound == name {
            break;
        }
        // One step in, `name[0]` or `name.b`, as `part_read` reads one.
        let Some((mut steps, default)) = part_read(value, name).filter(|(steps, _)| steps.len() == 1) else {
            break;
        };
        let part = match steps.pop().expect("one step") {
            Step::At(k) => Ok(k),
            Step::Key(k) => Err(k),
        };
        // One part read again, `const a = x[0]; const b = x[0];`: two names
        // a pattern can't give it, so it's where they end.
        if parts.iter().any(|(p, ..)| *p == part) {
            break;
        }
        parts.push((part, bound.clone(), mutable, default));
    }
    parts
}

/// The variable `value` reads a part of, as `parts` reads one: `o[0]`,
/// `o.k`, or `o.k === undefined ? d : o.k`.
pub(super) fn part_of(value: &Expr) -> Option<&str> {
    let read = match &value.kind {
        ExprKind::Cond(_, _, read) => read,
        _ => value,
    };
    let (ExprKind::Index(of, _) | ExprKind::Member(of, _)) = &read.kind else {
        return None;
    };
    let ExprKind::Var(v) = &of.kind else {
        return None;
    };
    part_read(value, v)
        .is_some_and(|(steps, _)| steps.len() == 1)
        .then_some(v.as_str())
}

/// What takes `parts` apart, `[a, , c]` of indices, `{ a, b: c }` of keys:
/// none of none, or of both.
pub(super) fn pattern_of(parts: &[Part]) -> Option<js::Pattern> {
    if parts.is_empty() {
        return None;
    }
    if parts.iter().all(|(p, ..)| p.is_ok()) {
        let last = parts.iter().filter_map(|(p, ..)| p.as_ref().ok()).max().copied()?;
        let mut items = vec![None; last + 1];
        for (part, bound, ..) in parts {
            items[*part.as_ref().ok()?] = Some(bound.clone());
        }
        return Some(js::Pattern::names_array(items));
    }
    let keyed: Option<Vec<_>> = (parts.iter())
        .map(|(part, bound, _, default)| Some((part.clone().err()?, js::Pattern::Name(bound.clone()), default.clone())))
        .collect();
    Some(js::Pattern::Object(keyed?, None))
}

/// A parameter MIR names, `_2`, whose parts the body reads where it starts
/// and reads nowhere else, `(_2) => { const i = _2[0]; const c = _2[1]; … }`:
/// taken apart where it's given, `([i, c]) => …`.
pub(in crate::lower::mir) fn destructured_params(params: &mut [js::Pattern], body: &mut Vec<Stmt>) {
    // An `async fn`'s too, `__arg0`, which its body's `let` takes apart,
    // of the copy its body holds, `__arg0$1`.
    let unnamed = |name: &str| {
        let arg = name.split('$').next().and_then(|n| n.strip_prefix("__arg"));
        is_temporary(name) || arg.is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    };
    for param in params.iter_mut() {
        let js::Pattern::Name(name) = param else { continue };
        if !unnamed(name) {
            continue;
        }
        let mut name = name.clone();
        // `(_2) => { const n = _2; … }`, `|&n|`'s: `(n) => …`.
        while let Some(Stmt {
            kind: StmtKind::Const(bound, value),
            ..
        }) = body.first()
            && matches!(&value.kind, ExprKind::Var(v) if *v == name)
            && js::mentions_in(&body[1..], &name) == 0
        {
            name = bound.clone();
            *param = js::Pattern::Name(name.clone());
            body.remove(0);
        }
        if !unnamed(&name) {
            continue;
        }
        // Taken apart already, `const { params } = __arg0;`: that pattern.
        if let Some(Stmt {
            kind:
                StmtKind::Destructure {
                    pattern,
                    value,
                    mutable: false,
                },
            ..
        }) = body.first()
            && matches!(&value.kind, ExprKind::Var(v) if *v == name)
            && js::mentions_in(&body[1..], &name) == 0
        {
            *param = pattern.clone();
            body.remove(0);
            continue;
        }
        // Read after the constants the body declares first, its drop flags.
        constants_first(body, 0, 0, &name);
        let start = (body.iter())
            .take_while(
                |stmt| matches!(&stmt.kind, StmtKind::Let(bound, Some(value)) if value.is_constant() && *bound != name),
            )
            .count();
        let read = nested_parts(&body[start..], &name);
        let count = read.len();
        if js::mentions_in(&body[start + count..], &name) > 0 {
            continue;
        }
        let read: Vec<_> = (read.iter())
            .map(|(path, bound, default)| (path.as_slice(), bound.as_str(), default.as_ref()))
            .collect();
        let Some(pattern) = nested_pattern(&read) else { continue };
        body.drain(start..start + count);
        *param = pattern;
    }
}

/// A step into a value, `[1]` or `.name`.
#[derive(PartialEq)]
pub(super) enum Step {
    At(usize),
    Key(String),
}

/// The steps from `name` to what `e` reads, `[At(1), Key("name")]` of
/// `_2[1].name`.
pub(super) fn path(e: &Expr, name: &str) -> Option<Vec<Step>> {
    let (of, step) = match &e.kind {
        ExprKind::Var(v) if v == name => return Some(Vec::new()),
        ExprKind::Index(of, index)
            if let ExprKind::Num(k) = index.kind
                && k >= 0.0
                && k.fract() == 0.0 =>
        {
            (of, Step::At(k as usize))
        }
        ExprKind::Member(of, key) => (of, Step::Key(key.clone())),
        _ => return None,
    };
    let mut steps = path(of, name)?;
    steps.push(step);
    Some(steps)
}

/// The part of `name` that `value` reads, however deep: the steps to it,
/// `_2[1].name`'s, and a field's default, `o.k === undefined ? d : o.k`'s.
/// The one reading of a part, `parts`', `part_of`'s and `nested_parts`'.
pub(super) fn part_read(value: &Expr, name: &str) -> Option<(Vec<Step>, Option<Expr>)> {
    match &value.kind {
        ExprKind::Cond(test, default, read)
            if let ExprKind::Binary(Op::Eq, tested, undefined) = &test.kind
                && matches!(undefined.kind, ExprKind::Undefined)
                && let Some(steps) = path(read, name)
                && matches!(steps.last(), Some(Step::Key(_)))
                && path(tested, name).as_ref() == Some(&steps) =>
        {
            Some((steps, Some((**default).clone())))
        }
        _ => path(value, name)
            .filter(|steps| !steps.is_empty())
            .map(|steps| (steps, None)),
    }
}

/// The parts of `name` the first of `stmts` read, as `parts` finds them,
/// however deep, `const name = _2[1].name;`, each a path, its variable,
/// and a field's default.
pub(super) fn nested_parts(stmts: &[Stmt], name: &str) -> Vec<(Vec<Step>, String, Option<Expr>)> {
    let mut parts = Vec::new();
    for stmt in stmts {
        let (StmtKind::Const(bound, value) | StmtKind::Let(bound, Some(value))) = &stmt.kind else {
            break;
        };
        if bound == name {
            break;
        }
        let Some((steps, default)) = part_read(value, name) else {
            break;
        };
        parts.push((steps, bound.clone(), default));
    }
    parts
}

/// The pattern of `parts`, each a path, its variable and its default: of
/// the first steps all indices, an array, all keys, an object, each part
/// a variable or, where the paths go on, a pattern in turn, `[i, { name
/// }]`.
pub(super) fn nested_pattern(parts: &[(&[Step], &str, Option<&Expr>)]) -> Option<js::Pattern> {
    type Group<'p> = Vec<(&'p [Step], &'p str, Option<&'p Expr>)>;
    let mut groups: Vec<(&Step, Group)> = Vec::new();
    for &(steps, bound, default) in parts {
        let (first, rest) = steps.split_first()?;
        match groups.iter_mut().find(|(step, _)| *step == first) {
            Some((_, group)) => group.push((rest, bound, default)),
            None => groups.push((first, vec![(rest, bound, default)])),
        }
    }
    let part = |group: &Group| match group.as_slice() {
        [([], bound, default)] => Some((js::Pattern::Name(bound.to_string()), default.cloned())),
        _ if group.iter().all(|(rest, ..)| !rest.is_empty()) => Some((nested_pattern(group)?, None)),
        _ => None,
    };
    if groups.iter().all(|(step, _)| matches!(step, Step::At(_))) {
        let last = (groups.iter()).filter_map(|(step, _)| match step {
            Step::At(k) => Some(*k),
            Step::Key(_) => None,
        });
        let mut items = vec![None; last.max()? + 1];
        for (step, group) in &groups {
            let (Step::At(k), (pattern, None)) = (step, part(group)?) else {
                return None;
            };
            items[*k] = Some(pattern);
        }
        return Some(js::Pattern::Array(items));
    }
    let fields = (groups.iter())
        .map(|(step, group)| match step {
            Step::Key(key) => {
                let (pattern, default) = part(group)?;
                Some((key.clone(), pattern, default))
            }
            Step::At(_) => None,
        })
        .collect::<Option<_>>()?;
    Some(js::Pattern::Object(fields, None))
}

/// `const children = [" ", c ? a : b];`, a tuple MIR holds, read only item
/// by item, `children[0]`: each item where it's read, of items that do
/// nothing and read what nothing changes, which read the same anywhere.
/// One that does, `match (v.pop(), n > 3)`'s `v.pop()`, is held first, in
/// order, as a person holds it: `const value = v.pop();`, and what's read
/// before it with it (ADR 0369).
pub(super) fn tuple_reads(
    stmts: &mut Vec<Stmt>,
    changing: &HashSet<String>,
    set_elsewhere: &HashSet<String>,
    captured: &HashSet<String>,
) {
    let inert = |item: &Expr| {
        let mut inert = !item.has_effects();
        item.visit_vars(&mut |v| inert &= !changing.contains(v));
        inert
    };
    let mut i = 0;
    while i < stmts.len() {
        let found = match &stmts[i].kind {
            StmtKind::Const(t, value)
                if let ExprKind::Array(items) = &value.kind
                    && (is_temporary(t) || (captured.contains(t) && items.iter().all(inert))) =>
            {
                Some((t.clone(), items.clone()))
            }
            _ => None,
        };
        let Some((t, mut items)) = found else {
            i += 1;
            continue;
        };
        let len = items.len();
        let item = |e: &Expr| match &e.kind {
            ExprKind::Index(of, k) if matches!(&of.kind, ExprKind::Var(v) if *v == t) => {
                k.as_int().and_then(|k| usize::try_from(k).ok()).filter(|&k| k < len)
            }
            _ => None,
        };
        let mut indexed = 0;
        js::each_expr_mut(&mut stmts[i + 1..], &mut |e| indexed += usize::from(item(e).is_some()));
        // Read only, not set through, `children[0] = ..`.
        let writes = js::Writes::of(&stmts[i + 1..]);
        if writes.replaced.contains(&t)
            || writes.changed.contains(&t)
            || indexed == 0
            || indexed != js::mentions_in(&stmts[i + 1..], &t)
        {
            i += 1;
            continue;
        }
        // What's read before the last that acts is held too, as it was
        // read before it acted, `v.length` of `(v.len(), v.pop())`: all but
        // what no call changes, `(a + b) | 0` of variables nothing sets again.
        // A variable that changes, but not after it, `quotient`, nor by a
        // closure or a handle, read where the tuple's part is.
        let span = stmts[i].span;
        let after = &stmts[i + 1..];
        let acts = |item: &Expr| {
            !inert(item) && !matches!(&item.kind, ExprKind::Var(v) if !assigns(after, v) && !set_elsewhere.contains(v))
        };
        let last = items.iter().rposition(acts);
        let fixed = |item: &Expr| {
            let math = |e: &Expr| matches!(&e.kind, ExprKind::Var(m) if m == "Math");
            let mut fixed = inert(item);
            item.clone().each_mut(&mut |e| {
                fixed &= match &e.kind {
                    ExprKind::Call(callee, _) => matches!(&callee.kind, ExprKind::Member(of, _) if math(of)),
                    ExprKind::Member(of, _) => math(of),
                    _ => !matches!(
                        e.kind,
                        ExprKind::OptionalMember(..)
                            | ExprKind::Index(..)
                            | ExprKind::OptionalIndex(..)
                            | ExprKind::OptionalCall(..)
                            | ExprKind::Handle(_, _)
                            | ExprKind::New(..)
                            | ExprKind::Array(_)
                            | ExprKind::Object(..)
                            | ExprKind::Jsx(_)
                            | ExprKind::Arrow(..)
                            | ExprKind::AsyncArrow(..)
                            | ExprKind::Function(_)
                    ),
                }
            });
            fixed
        };
        let mut held = Vec::new();
        for (k, item) in items.iter_mut().enumerate() {
            if last.is_some_and(|last| k < last && !fixed(item) || k == last) {
                let name = crate::names::given_apart(&t, k);
                held.push(StmtKind::Const(name.clone(), std::mem::replace(item, Expr::var(&name))).at(span));
            }
        }
        js::each_expr_mut(&mut stmts[i + 1..], &mut |e| {
            if let Some(k) = item(e) {
                *e = items[k].clone();
            }
        });
        let count = held.len();
        stmts.splice(i..=i, held);
        i += count;
    }
}

/// `[{ value: a, impl }, ..].map((item) => item.value)`, `dyn` values MIR
/// unwraps for JS, of what else does nothing: `[a, ..]`.
pub(super) fn projected(e: &Expr) -> Option<Expr> {
    let ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    let (ExprKind::Member(items, map), [f]) = (&callee.kind, args.as_slice()) else {
        return None;
    };
    let (ExprKind::Array(items), "map") = (&items.kind, map.as_str()) else {
        return None;
    };
    let field = field_of(f)?;
    let values = items.iter().map(|item| match &item.kind {
        ExprKind::Object(props, ..) => {
            let mut value = None;
            for prop in props {
                match prop {
                    js::Prop::Field(name, v) if *name == field => value = Some(v.clone()),
                    js::Prop::Field(_, v) if !v.has_effects() => {}
                    _ => return None,
                }
            }
            value
        }
        _ => None,
    });
    Some(Expr {
        span: e.span,
        ..Expr::array(values.collect::<Option<Vec<_>>>()?)
    })
}

/// `f` of `(item) => item.f`.
pub(super) fn field_of(f: &Expr) -> Option<String> {
    let ExprKind::Arrow(params, body) = &f.kind else {
        return None;
    };
    let (
        [js::Pattern::Name(x)],
        [
            Stmt {
                kind: StmtKind::Return(Some(read)),
                ..
            },
        ],
    ) = (params.as_slice(), body.as_slice())
    else {
        return None;
    };
    match &read.kind {
        ExprKind::Member(of, field) if matches!(&of.kind, ExprKind::Var(v) if v == x) => Some(field.clone()),
        _ => None,
    }
}

/// `const t = [{ value: a, impl }, ..]; .. t.map((item) => item.value)`, read
/// so alone: `const t = [a, ..]; .. t`, `projected`'s, held.
pub(super) fn unwrapped_values(stmts: &mut [Stmt]) {
    for i in 0..stmts.len() {
        let StmtKind::Const(t, value) = &stmts[i].kind else {
            continue;
        };
        if !is_temporary(t) || !matches!(value.kind, ExprKind::Array(_)) || js::mentions_in(&stmts[i + 1..], t) != 1 {
            continue;
        }
        let t = t.clone();
        let mut field = None;
        js::each_expr_mut(&mut stmts[i + 1..], &mut |e| {
            if let ExprKind::Call(callee, args) = &e.kind
                && let (ExprKind::Member(of, map), [f]) = (&callee.kind, args.as_slice())
                && matches!(&of.kind, ExprKind::Var(v) if *v == t)
                && map == "map"
            {
                field = field_of(f).map(|_| f.clone());
            }
        });
        let Some(f) = field else { continue };
        let StmtKind::Const(_, value) = &stmts[i].kind else {
            unreachable!("matched")
        };
        let call = Expr::call(Expr::member(value.clone(), "map"), vec![f]);
        let Some(values) = projected(&call) else {
            continue;
        };
        stmts[i].kind = StmtKind::Const(t.clone(), values);
        js::each_expr_mut(&mut stmts[i + 1..], &mut |e| {
            if let ExprKind::Call(callee, _) = &e.kind
                && let ExprKind::Member(of, _) = &callee.kind
                && matches!(&of.kind, ExprKind::Var(v) if *v == t)
            {
                *e = Expr::var(&t);
            }
        });
    }
}

/// A parameter an `async fn`'s body copies where it starts and reads nowhere
/// else, `const forbidden$2 = forbidden$1`: the copy, of the parameter's own
/// name, is the parameter, as JS's parameters are variables the body may
/// set. Not a variable the program names otherwise, `let mut n = from`.
pub(in crate::lower::mir) fn copied_params(params: &[js::Pattern], body: &mut Vec<Stmt>) {
    let names: Vec<&str> = params.iter().flat_map(|p| p.names()).collect();
    while let Some(first) = body.first()
        && let StmtKind::Const(copy, value) | StmtKind::Let(copy, Some(value)) = &first.kind
        && let ExprKind::Var(param) = &value.kind
        && names.contains(&param.as_str())
        && !names.contains(&copy.as_str())
        && copy.split('$').next() == param.split('$').next()
        && js::mentions_in(body, param) == 1
    {
        let (copy, param) = (copy.clone(), param.clone());
        body.remove(0);
        js::each_expr_mut(body, &mut |e| {
            if let ExprKind::Var(v) = &mut e.kind
                && *v == copy
            {
                *v = param.clone();
            }
        });
    }
}
