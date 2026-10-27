//! A `Drop` impl's body where it's dropped (ADRs 0589, 0608).

use crate::js::{self, Expr, ExprKind, Item, Pattern, Prop, Stmt, StmtKind};
use std::collections::{HashMap, HashSet};

/// Of `items`, a module's, each function `drops` names, a `Drop` impl's
/// `drop`, which isn't exported and that no other module calls: where each
/// of its calls is a statement whose place its body fits, a closure's too,
/// its body there, as a closure called where it's made is
/// (`js::applied_call`), and the function gone. The guard it's given, made
/// there of its fields and only read by them, is its fields:
///
/// ```js
/// const previousPriorityLevel = currentPriorityLevel;
/// try {
///   return workLoop(initialTime);
/// } finally {
///   currentPriorityLevel = previousPriorityLevel;
/// }
/// ```
pub(in crate::lower) fn inlined_drops(items: &mut Vec<Item>, drops: &HashSet<String>) {
    // Each in turn, one no other drop left calls first, so a `Drop` impl's
    // body is where its value's dropped before the drops it calls are, each
    // guard taken apart where it's made; among those, in the module's order,
    // never a set's, which would change the JS from one build to the next.
    let mut left: Vec<String> = (items.iter())
        .filter_map(|item| match item {
            Item::Function(f) if drops.contains(&f.name) => Some(f.name.clone()),
            _ => None,
        })
        .collect();
    while !left.is_empty() {
        let called = |name: &String| {
            (left.iter()).any(|other| {
                other != name
                    && (items.iter()).any(
                        |item| matches!(item, Item::Function(f) if f.name == *other && js::mentions_in(&f.body, name) > 0),
                    )
            })
        };
        let next = left.iter().position(|name| !called(name)).unwrap_or(0);
        let name = left.remove(next);
        inline_drop(items, &name);
    }
}

/// `inlined_drops`' of the drop `name`.
fn inline_drop(items: &mut Vec<Item>, name: &str) {
    let Some(at) = items.iter().position(
        |item| matches!(item, Item::Function(f) if f.name == name && matches!(f.params.as_slice(), [Pattern::Name(_)])),
    ) else {
        return;
    };
    let Item::Function(drop) = &items[at] else {
        unreachable!("found")
    };
    let (params, body) = (drop.params.clone(), drop.body.clone());
    // One whose body drops a value of its own type calls itself: written
    // at each call, it would make another call to write, never done (#229).
    if js::mentions_in(&body, name) > 0 {
        return;
    }
    let mentions: usize = (items.iter().enumerate())
        .map(|(i, item)| match i == at {
            true => js::mentions_in(&body, name),
            false => mentions_in_item(item, name),
        })
        .sum();
    // Each drop its body, where each is a call as a statement (ADR
    // 0589): each function's body made so, then, where every mention is
    // one, set.
    let mut bodies = Vec::new();
    let mut inlined = 0;
    let mut each = true;
    for (i, item) in items.iter_mut().enumerate() {
        if i == at {
            continue;
        }
        each_function(item, &mut |own_params, own_body| {
            if js::mentions_in(own_body, name) == 0 {
                return;
            }
            let mut made = own_body.clone();
            let mut given = Vec::new();
            loop {
                let whole = made.clone();
                let mut done = false;
                js::each_block_mut(&mut made, &mut |stmts| {
                    if done {
                        return;
                    }
                    let Some(k) = stmts.iter().position(|s| calls(s, name)) else {
                        return;
                    };
                    match js::applied_call(&stmts[k], &whole, &params, &body) {
                        Some(block) => {
                            given.extend(argument(&stmts[k]));
                            stmts.splice(k..=k, block);
                            inlined += 1;
                        }
                        None => each = false,
                    }
                    done = true;
                });
                if !done || !each {
                    break;
                }
            }
            // A drop of nothing to do leaves its `finally` empty: the `try`
            // is its block, where what that declares is no other name here.
            js::each_block_mut(&mut made, &mut |stmts| unwrapped_tries(stmts));
            // Where its name is the guard's alone: a closure's parameter of
            // that name, or a `const` another closure makes, is another
            // value, whose `d[0]` is no guard's field (#239); so, of one
            // named twice, in each closure where it's its own.
            for guard in given {
                if alone(own_params, &made, &guard, true) {
                    unread_writes(&mut made, &guard, false);
                    fields_of(&mut made, own_params, &guard);
                    one_field_of(&mut made, own_params, &guard);
                    continue;
                }
                js::each_expr_mut(&mut made, &mut |e| {
                    if let ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) = &mut e.kind
                        && alone(params, body, &guard, false)
                    {
                        unread_writes(body, &guard, false);
                        fields_of(body, params, &guard);
                        one_field_of(body, params, &guard);
                    }
                });
            }
            bodies.push(made);
        });
    }
    if !each || inlined == 0 || inlined != mentions {
        return;
    }
    let mut bodies = bodies.into_iter();
    for (i, item) in items.iter_mut().enumerate() {
        if i == at {
            continue;
        }
        each_function(item, &mut |_, body| {
            if js::mentions_in(body, name) > 0 {
                *body = bodies.next().expect("one made of each");
            }
        });
    }
    items.remove(at);
}

/// Whether `name` is one variable's in a function of `params` and `body`:
/// declared there once, and, of a closure in the function a drop's written
/// in, not `whole`, by a parameter or its own statements' `const`, so no
/// read of it in the closure is of a variable outside.
fn alone(params: &[Pattern], body: &[Stmt], name: &str, whole: bool) -> bool {
    let given = params.iter().any(|p| p.names().contains(&name));
    let mut declared = usize::from(given);
    js::each_declared(body, &mut |n| declared += usize::from(n == name));
    declared == 1
        && (whole
            || given
            || body
                .iter()
                .any(|s| matches!(&s.kind, StmtKind::Const(n, _) if n == name)))
}

/// Each `try { .. } finally {}` of `stmts`, a drop's of nothing to do
/// (ADR 0608, amended): its block in its place, where what the block
/// declares at its top `stmts` names nowhere else.
fn unwrapped_tries(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        let StmtKind::Try(block, finally) = &stmts[i].kind else {
            i += 1;
            continue;
        };
        let mut declared = Vec::new();
        for s in block {
            match &s.kind {
                StmtKind::Const(name, _) | StmtKind::Let(name, _) => declared.push(name.clone()),
                StmtKind::Destructure { pattern, .. } => {
                    declared.extend(pattern.names().into_iter().map(str::to_string))
                }
                StmtKind::Function(function) => declared.push(function.name.clone()),
                _ => {}
            }
        }
        let others: Vec<Stmt> = (stmts.iter().enumerate())
            .filter(|&(k, _)| k != i)
            .map(|(_, s)| s.clone())
            .collect();
        let mut named = HashSet::new();
        js::each_declared(&others, &mut |name| _ = named.insert(name.to_string()));
        let free = declared
            .iter()
            .all(|name| !named.contains(name) && js::mentions_in(&others, name) == 0);
        if !finally.is_empty() || !free {
            i += 1;
            continue;
        }
        let block = block.clone();
        let n = block.len();
        stmts.splice(i..=i, block);
        i += n;
    }
}

/// Of `stmts`, in which `guard` is made and dropped, each write of one of
/// its fields that nothing reads after, `_restore[0] = undefined;` of a
/// `Drop`'s `self.0.take()`: gone, as a guard is dropped once, and is
/// nothing after (ADR 0705, amended). `read_after` says whether what runs
/// after `stmts` may read it. Into a `try`, an `if` and a labeled block,
/// and a loop's body where it's made, fresh each time round: what else
/// runs again, or after a `catch`, is left as it is.
fn unread_writes(stmts: &mut Vec<Stmt>, guard: &str, mut read_after: bool) {
    let field = |e: &Expr| {
        matches!(&e.kind, ExprKind::Index(of, _) | ExprKind::Member(of, _)
            if matches!(&of.kind, ExprKind::Var(v) if v == guard))
    };
    let mut i = stmts.len();
    while i > 0 {
        i -= 1;
        if !read_after
            && let StmtKind::Assign(target, value) = &stmts[i].kind
            && field(target)
            && !value.has_effects()
            && js::mentions_in(&[StmtKind::Expr(value.clone()).at(value.span)], guard) == 0
        {
            stmts.remove(i);
            continue;
        }
        let read = js::mentions_in(&stmts[i..=i], guard) > 0;
        let made_in = |body: &[Stmt]| {
            body.iter()
                .any(|s| matches!(&s.kind, StmtKind::Const(n, _) | StmtKind::Let(n, _) if n == guard))
        };
        match &mut stmts[i].kind {
            StmtKind::Try(block, finally) => {
                unread_writes(finally, guard, read_after);
                let read_finally = js::mentions_in(finally, guard) > 0;
                unread_writes(block, guard, read_after || read_finally);
            }
            StmtKind::If(_, a, b) => {
                if let Some(b) = b {
                    unread_writes(b, guard, read_after);
                }
                unread_writes(a, guard, read_after);
            }
            StmtKind::Labeled(_, body) => unread_writes(body, guard, read_after),
            StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. }
                if made_in(body) =>
            {
                unread_writes(body, guard, false)
            }
            _ => {}
        }
        read_after |= read;
    }
}

/// Each `const n = e;` of `body` of what `field` says is a read of a
/// guard's field nothing writes, which reads as `v`, the variable it was
/// made of or a `const` of its own, `const old = _restore[0];` of a
/// `Drop`'s `self.0.take()`: `n` read as `v`, where each of `body`'s
/// declarations of `n` is such a copy, as nothing sets either once it's
/// made (ADR 0705, amended).
fn copies_read_as(body: &mut Vec<Stmt>, field: &dyn Fn(&Expr) -> bool, v: &str) {
    let mut copies = Vec::new();
    js::each_block_mut(body, &mut |stmts| {
        for s in stmts.iter() {
            if let StmtKind::Const(n, value) = &s.kind
                && n != v
                && field(value)
            {
                copies.push(n.clone());
            }
        }
    });
    copies.sort();
    copies.dedup();
    for n in copies {
        // Each of its declarations a copy, in each place the drop is.
        let mut declared = 0;
        js::each_declared(body, &mut |name| declared += usize::from(name == n));
        let mut copied = 0;
        js::each_block_mut(body, &mut |stmts| {
            copied += (stmts.iter())
                .filter(|s| matches!(&s.kind, StmtKind::Const(name, value) if *name == n && field(value)))
                .count();
        });
        if declared != copied {
            continue;
        }
        js::each_block_mut(body, &mut |stmts| {
            stmts.retain(|s| !matches!(&s.kind, StmtKind::Const(name, value) if *name == n && field(value)));
        });
        js::each_expr_mut(body, &mut |e| {
            if let ExprKind::Var(name) = &mut e.kind
                && *name == n
            {
                *name = v.to_string();
            }
        });
    }
}

/// How often `item` names `name`.
fn mentions_in_item(item: &Item, name: &str) -> usize {
    let value = |e: &Expr| js::mentions_in(&[StmtKind::Expr(e.clone()).at(e.span)], name);
    match item {
        Item::Namespace(namespace) => {
            (namespace.constants.iter()).map(|c| value(&c.value)).sum::<usize>()
                + (namespace.methods.iter())
                    .map(|f| js::mentions_in(&f.body, name))
                    .sum::<usize>()
        }
        Item::Const(constant) => value(&constant.value),
        Item::Statements(stmts) => js::mentions_in(stmts, name),
        Item::Function(function) => js::mentions_in(&function.body, name),
    }
}

/// What's asked of each function: its parameters and its body.
type EachFunction<'f> = dyn FnMut(&[Pattern], &mut Vec<Stmt>) + 'f;

/// Each function `item` is or holds, its parameters and its body: a type's
/// methods, and each arrow a `const` is or is made of, a closure coerced
/// to a `fn`, `const CALL: fn(..) = |f| { .. }`, or a field of a struct
/// or a tuple of them, React's `callComponent = { react_stack_bottom_frame:
/// |..| { .. } }`, or of either value a conditional chooses, as a
/// function is; and what the module runs as it loads.
fn each_function(item: &mut Item, f: &mut EachFunction<'_>) {
    fn arrow(value: &mut Expr, f: &mut EachFunction<'_>) {
        match &mut value.kind {
            ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) => f(params, body),
            ExprKind::Object(props, _) => {
                for prop in props {
                    if let Prop::Field(_, value) | Prop::Method(_, value) = prop {
                        arrow(value, f);
                    }
                }
            }
            ExprKind::Array(items) => items.iter_mut().for_each(|item| arrow(item, f)),
            // Either value a test chooses, React's DEV dispatchers', `__DEV__
            // ? { useMemo(..) { .. } } : null`.
            ExprKind::Cond(_, then, otherwise) => {
                arrow(then, f);
                arrow(otherwise, f);
            }
            _ => {}
        }
    }
    match item {
        Item::Function(function) => f(&function.params, &mut function.body),
        Item::Namespace(namespace) => {
            for method in &mut namespace.methods {
                f(&method.params, &mut method.body);
            }
            for constant in &mut namespace.constants {
                arrow(&mut constant.value, f);
            }
        }
        Item::Const(constant) => arrow(&mut constant.value, f),
        // What the module runs as it loads, `js::on_load!`'s, with its
        // closures, React's DEV dispatchers made there: no function's body,
        // which no `return` ends, but a drop's call there is a statement
        // as in one.
        Item::Statements(stmts) => f(&[], stmts),
    }
}

/// Whether `stmt` is `name(arg);`.
fn calls(stmt: &Stmt, name: &str) -> bool {
    matches!(&stmt.kind, StmtKind::Expr(e)
        if matches!(&e.kind, ExprKind::Call(callee, args)
            if args.len() == 1 && matches!(&callee.kind, ExprKind::Var(v) if v == name)))
}

/// The variable `f(guard);` gives.
fn argument(stmt: &Stmt) -> Option<String> {
    let StmtKind::Expr(e) = &stmt.kind else { return None };
    let ExprKind::Call(_, args) = &e.kind else { return None };
    match &args.first()?.kind {
        ExprKind::Var(v) => Some(v.clone()),
        _ => None,
    }
}

/// `const guard = { a: x, b: y };` of `body`, each of whose reads is of a
/// field, `guard.a`: `const a = x; const b = y;`, read as `a`, a `let` of
/// one written, `let hasMoreWork = true;`, where no other name of the
/// function is a field's; and a field of a variable that nothing sets
/// once the guard's made, `{ prototype, prevProps }`'s, read as that
/// variable, `prototype` (ADR 0705).
fn fields_of(body: &mut Vec<Stmt>, params: &[Pattern], guard: &str) {
    let mut made = None;
    js::each_block_mut(body, &mut |stmts| {
        if made.is_none()
            && let Some(k) = stmts
                .iter()
                .position(|s| matches!(&s.kind, StmtKind::Const(n, _) if n == guard))
            && let StmtKind::Const(_, value) = &stmts[k].kind
            && let ExprKind::Object(props, ..) = &value.kind
        {
            made = Some(props.clone());
        }
    });
    let Some(props) = made else { return };
    let mut fields = Vec::new();
    for prop in &props {
        match prop {
            Prop::Field(key, value) => fields.push((key.clone(), value.clone())),
            _ => return,
        }
    }
    // Every read of it a field's, and nothing written through it.
    let mut reads = 0;
    let mut stmts = body.clone();
    js::each_expr_mut(&mut stmts, &mut |e| {
        if let ExprKind::Member(of, key) = &e.kind
            && matches!(&of.kind, ExprKind::Var(v) if v == guard)
            && fields.iter().any(|(k, _)| k == key)
        {
            reads += 1;
        }
    });
    // Each field written through it, a `let` of its own.
    let mut written = HashSet::new();
    js::each_block_mut(&mut stmts, &mut |list| {
        for s in list.iter() {
            if let StmtKind::Assign(t, _) | StmtKind::Compound(_, t, _) = &s.kind
                && let ExprKind::Member(of, key) = &t.kind
                && matches!(&of.kind, ExprKind::Var(v) if v == guard)
            {
                written.insert(key.clone());
            }
        }
    });
    if reads != js::mentions_in(body, guard) {
        return;
    }
    let mut names = HashSet::new();
    js::names_in(body, &mut names);
    names.extend(params.iter().flat_map(|p| p.names()).map(str::to_string));
    // What each field reads as: the variable it was made of, where nothing
    // sets that once the guard's made and it's the one of its name; else a
    // `const` of its own, its key, where that names nothing else.
    let kept = kept_variables(body, params, guard, &fields);
    let read_as = |(key, value): &(String, Expr)| match &value.kind {
        ExprKind::Var(v) if !written.contains(key) && kept.contains(v) => Some(v.clone()),
        _ => None,
    };
    if fields
        .iter()
        .any(|field| read_as(field).is_none() && (names.contains(&field.0) || !crate::names::is_identifier(&field.0)))
    {
        return;
    }
    // A copy of a field nothing writes, `const old = guard.previous;`, is
    // what the field reads as.
    for field in fields.iter().filter(|(key, _)| !written.contains(key)) {
        let (key, v) = (&field.0, read_as(field).unwrap_or_else(|| field.0.clone()));
        let read = |e: &Expr| {
            matches!(&e.kind, ExprKind::Member(of, k)
                if k == key && matches!(&of.kind, ExprKind::Var(g) if g == guard))
        };
        copies_read_as(body, &read, &v);
    }
    // Each read of a field the value it was made of, never `undefined`
    // where that's what was known of it and nothing sets it again: what a
    // nullable place it's given stores as it is (ADR 0841, amended).
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Member(of, key) = &e.kind
            && matches!(&of.kind, ExprKind::Var(v) if v == guard)
            && let Some(field) = fields.iter().find(|(k, _)| k == key)
        {
            let read = Expr::var(&read_as(field).unwrap_or_else(|| key.clone())).or_at(e.span);
            *e = match field.1.facts.has(js::Known::Defined) && !written.contains(key) {
                true => read.defined(),
                false => read,
            };
        }
    });
    js::each_expr_mut(body, &mut Expr::store_again);
    js::each_block_mut(body, &mut |stmts| {
        // A copy of a field the variable it reads as, `const prevProps =
        // prevProps;`, is that variable.
        stmts.retain(
            |s| !matches!(&s.kind, StmtKind::Const(n, value) if matches!(&value.kind, ExprKind::Var(v) if v == n)),
        );
        if let Some(k) = stmts
            .iter()
            .position(|s| matches!(&s.kind, StmtKind::Const(n, _) if n == guard))
        {
            let span = stmts[k].span;
            let consts = (fields.iter())
                .filter(|field| read_as(field).is_none())
                .map(|(key, value)| match written.contains(key) {
                    true => StmtKind::Let(key.clone(), Some(value.clone())).at(span),
                    false => StmtKind::Const(key.clone(), value.clone()).at(span),
                });
            stmts.splice(k..=k, consts.collect::<Vec<_>>());
        }
    });
}

/// The variables `body` declares once, or `params` do, that nothing sets
/// once `const guard = ..;` is made: later where it's made, by a closure,
/// or through a handle; what the guard's field of one holds is that
/// variable, wherever the guard is read. A copy of the field into one of
/// its name, `const prevProps = guard.prevProps;`, a `match`'s binding in
/// its `Drop`, declares no other.
fn kept_variables(body: &[Stmt], params: &[Pattern], guard: &str, fields: &[(String, Expr)]) -> HashSet<String> {
    let mut declared: HashMap<String, usize> = HashMap::new();
    js::each_declared(body, &mut |name| *declared.entry(name.to_string()).or_default() += 1);
    let mut copies = body.to_vec();
    js::each_block_mut(&mut copies, &mut |list| {
        for s in list.iter() {
            if let StmtKind::Const(name, value) = &s.kind
                && let ExprKind::Member(of, key) = &value.kind
                && matches!(&of.kind, ExprKind::Var(g) if g == guard)
                && fields
                    .iter()
                    .any(|(k, v)| k == key && matches!(&v.kind, ExprKind::Var(v) if v == name))
                && let Some(count) = declared.get_mut(name)
            {
                *count -= 1;
            }
        }
    });
    for name in params.iter().flat_map(|p| p.names()) {
        *declared.entry(name.to_string()).or_default() += 1;
    }
    let whole = js::Writes::of(body);
    let mut later = None;
    let mut stmts = body.to_vec();
    js::each_block_mut(&mut stmts, &mut |list| {
        if later.is_none()
            && let Some(k) = list
                .iter()
                .position(|s| matches!(&s.kind, StmtKind::Const(n, _) if n == guard))
        {
            later = Some(js::Writes::of(&list[k + 1..]));
        }
    });
    let Some(later) = later else {
        return HashSet::new();
    };
    (declared.into_iter())
        .filter(|(name, count)| {
            *count == 1
                && !later.may_replace(name)
                && !whole.replaced_in_closures.contains(name)
                && !whole.handled.contains(name)
        })
        .map(|(name, _)| name)
        .collect()
}

/// `const guard = [x];`, a one-field tuple struct's guard, each of whose
/// reads is `guard[0]`, none written: `const guard = x;`, read as `guard`,
/// the field it's of (ADR 0608); and of a variable that nothing sets once
/// the guard's made, read as that variable, `previousFiber`, as a named
/// field of one is (`fields_of`, ADR 0705).
fn one_field_of(body: &mut Vec<Stmt>, params: &[Pattern], guard: &str) {
    let mut made = None;
    js::each_block_mut(body, &mut |stmts| {
        for s in stmts.iter() {
            if let StmtKind::Const(n, value) = &s.kind
                && n == guard
                && let ExprKind::Array(items) = &value.kind
                && let [item] = items.as_slice()
            {
                made.get_or_insert(item.clone());
            }
        }
    });
    let Some(item) = made else {
        return;
    };
    let known = item.facts;
    let field = |e: &Expr| {
        matches!(&e.kind, ExprKind::Index(of, at)
            if matches!(&of.kind, ExprKind::Var(v) if v == guard) && at.as_int() == Some(0))
    };
    let mut reads = 0;
    let mut written = false;
    let mut stmts = body.clone();
    js::each_expr_mut(&mut stmts, &mut |e| reads += usize::from(field(e)));
    js::each_block_mut(&mut stmts, &mut |list| {
        written |= list
            .iter()
            .any(|s| matches!(&s.kind, StmtKind::Assign(t, _) | StmtKind::Compound(_, t, _) if field(t)));
    });
    if written || reads != js::mentions_in(body, guard) {
        return;
    }
    // The variable it was made of; or read now, so not by Rust's name for
    // what's only dropped, `_restore`, where `restore` is free.
    let variable = match &item.kind {
        ExprKind::Var(v) if kept_variables(body, params, guard, &[]).contains(v) => Some(v.clone()),
        _ => None,
    };
    let mut names = HashSet::new();
    js::names_in(body, &mut names);
    // Not a word JS keeps, `_in`'s `in`.
    let read = match (&variable, guard.trim_start_matches('_')) {
        (Some(v), _) => v.clone(),
        (None, free) if !free.is_empty() && !names.contains(free) && !crate::names::kept_word(free) => free.to_string(),
        _ => guard.to_string(),
    };
    // A copy of it, `const old = _restore[0];`, is what it reads as too.
    copies_read_as(body, &field, &read);
    // Never `undefined` where that's what was known of it (ADR 0841,
    // amended), as `fields_of` reads one.
    js::each_expr_mut(body, &mut |e| {
        if field(e) {
            let read = Expr::var(&read).or_at(e.span);
            *e = match known.has(js::Known::Defined) {
                true => read.defined(),
                false => read,
            };
        }
    });
    js::each_expr_mut(body, &mut Expr::store_again);
    js::each_block_mut(body, &mut |stmts| {
        let made = |s: &Stmt| {
            matches!(&s.kind, StmtKind::Const(n, value)
                if n == guard && matches!(&value.kind, ExprKind::Array(items) if items.len() == 1))
        };
        if variable.is_some() {
            stmts.retain(|s| !made(s));
        }
        for s in stmts.iter_mut() {
            if made(s)
                && let StmtKind::Const(n, value) = &mut s.kind
                && let ExprKind::Array(items) = &mut value.kind
            {
                *n = read.clone();
                *value = items.remove(0);
            }
        }
    });
}
