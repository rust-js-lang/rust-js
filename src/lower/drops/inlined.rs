//! A `Drop` impl's body where it's dropped (ADRs 0589, 0608).

use crate::collections::{IndexMap, IndexSet};
use crate::js::{self, Expr, ExprKind, Item, Pattern, Prop, Stmt, StmtKind};

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
pub(in crate::lower) fn inlined_drops(items: &mut Vec<Item>, drops: &IndexSet<String>) {
    // Each in turn, one no other drop left calls first, so a `Drop` impl's
    // body is where its value's dropped before the drops it calls are, each
    // guard taken apart where it's made; among those, in the module's order,
    // never a set's, which would change the JS from one build to the next.
    let mut left: Vec<js::Name> = (items.iter())
        .filter_map(|item| match item {
            Item::Function(f) if drops.contains(f.name.word()) => Some(f.name.clone()), // word: a module's drop function, by its item's name
            _ => None,
        })
        .collect();
    while !left.is_empty() {
        let called = |name: &js::Name| {
            (left.iter()).any(|other| {
                other != name
                    && (items.iter()).any(
                        |item| matches!(item, Item::Function(f) if f.name == *other && js::mentions_of(&f.body, name) > 0),
                    )
            })
        };
        let next = left.iter().position(|name| !called(name)).unwrap_or(0);
        let name = left.remove(next);
        inline_drop(items, &name);
    }
}

/// `inlined_drops`' of the drop `name`.
fn inline_drop(items: &mut Vec<Item>, name: &js::Name) {
    let Some(at) = items.iter().position(
        |item| matches!(item, Item::Function(f) if f.name == *name && matches!(f.params.as_slice(), [Pattern::Name(_), ..])),
    ) else {
        return;
    };
    let Item::Function(drop) = &items[at] else {
        unreachable!("found")
    };
    let (params, body) = (drop.params.clone(), drop.body.clone());
    // One whose body drops a value of its own type calls itself: written
    // at each call, it would make another call to write, never done (#229).
    if js::mentions_of(&body, name) > 0 {
        return;
    }
    let mentions: usize = (items.iter().enumerate())
        .map(|(i, item)| match i == at {
            true => js::mentions_of(&body, name),
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
            if js::mentions_of(own_body, name) == 0 {
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
            // Where its name is the guard's alone: a closure's parameter of
            // that name, or a `const` another closure makes, is another
            // value, whose `d[0]` is no guard's field (#239); so, of one
            // named twice, in each closure where it's its own.
            for guard in given {
                if alone(own_params, &made, &guard, true) {
                    taken_apart(&mut made, own_params, &guard);
                    continue;
                }
                js::each_expr_mut(&mut made, &mut |e| {
                    if let ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) = &mut e.kind
                        && alone(params, body, &guard, false)
                    {
                        taken_apart(body, params, &guard);
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
            if js::mentions_of(body, name) > 0 {
                *body = bodies.next().expect("one made of each");
                // A scope its body left nothing to drop in, none (#1594).
                js::each_block_mut(body, &mut crate::lower::drops::emptied_scopes);
            }
        });
    }
    items.remove(at);
}

/// `guard`, made in the function of `params` and `body` and dropped there,
/// taken apart: the writes of its fields nothing reads after gone, each
/// field a variable, and each field of a closure it calls once its body
/// there (#501).
fn taken_apart(body: &mut Vec<Stmt>, params: &[Pattern], guard: &js::Name) {
    // Of one used whole besides its fields, given to `$ptrTo` or stored,
    // what reaches it that way may read a field after: its writes stay
    // (#1704).
    if fields_alone(body, guard) {
        unread_writes(body, guard, GuardReads::default());
    }
    let mut fields = fields_of(body, params, guard);
    fields.extend(one_field_of(body, params, guard));
    for field in fields {
        called_closure(body, params, &field);
    }
}

/// `const restore = () => { .. };` of `body`, a function of `params`, a
/// guard's field taken apart, of the closure the guard was made of,
/// `Finally(|| ..)`, read once, the call `restore();` its `Drop`'s `(self.0)()`
/// is: its body there, as a closure called where it's made is
/// (`js::applied_call`), and the `const` gone, as a person writes the
/// `finally` (#501). The same code runs, when the call ran, in the same
/// order: making the closure does nothing, and an arrow's `this` is the
/// function's at either place. Only where each name its body reads is what
/// it is where it's made: declared in the function nowhere else, so no
/// declaration between the two shadows it (`reads_where_made`).
fn called_closure(body: &mut Vec<Stmt>, params: &[Pattern], name: &js::Name) {
    let mut closure = None;
    js::each_block_mut(&mut body.clone(), &mut |stmts| {
        for s in stmts.iter() {
            if let StmtKind::Const(n, value) = &s.kind
                && n == name
                && let ExprKind::Arrow(ps, b) = &value.kind
            {
                closure.get_or_insert((ps.clone(), b.clone()));
            }
        }
    });
    let Some((closure_params, closure_body)) = closure else {
        return;
    };
    let mut declared = 0;
    js::each_declared_name(body, &mut |n| declared += usize::from(n == name));
    // Called once, as `reach` asks (`reads_where_made`).
    if declared != 1 || !reads_where_made(body, params, name, (&closure_params, &closure_body)) {
        return;
    }
    // What's beside the body where it's called, the closure's `const` not:
    // it goes, so what the closure declares is no other name there.
    let mut whole = body.clone();
    js::each_block_mut(&mut whole, &mut |stmts| {
        stmts.retain(|s| !matches!(&s.kind, StmtKind::Const(n, _) if n == name));
    });
    let mut done = false;
    js::each_block_mut(body, &mut |stmts| {
        let called = |s: &Stmt| {
            matches!(&s.kind, StmtKind::Expr(e) if matches!(&e.kind, ExprKind::Call(callee, _)
                if matches!(&callee.kind, ExprKind::Var(v) if v == name)))
        };
        if done || !stmts.iter().any(called) {
            return;
        }
        done = true;
        let k = stmts.iter().position(called).expect("found");
        if let Some(block) = js::applied_call(&stmts[k], &whole, &closure_params, &closure_body) {
            stmts.splice(k..=k, block);
        }
    });
    if js::mentions_of(body, name) == 0 {
        js::each_block_mut(body, &mut |stmts| {
            stmts.retain(|s| !matches!(&s.kind, StmtKind::Const(n, _) if n == name));
        });
    }
}

/// Whether each variable `closure`, the closure `const name = ..` of `body`
/// holds, reads is, where `name` is called, the one it is where it's made:
/// one it declares itself, one of the module's, a parameter of the
/// function's, or one in reach of the call, as JS's scopes say by its id
/// (`js::scopes::reach`), each spelled apart from any other by
/// preparation (ADR 1150). One with no id of its own, its word alone, is
/// asked of no scope: none is.
fn reads_where_made(body: &[Stmt], params: &[Pattern], name: &js::Name, closure: (&[Pattern], &[Stmt])) -> bool {
    let (closure_params, closure_body) = closure;
    let mut inside: Vec<js::Name> = closure_params.iter().flat_map(|p| p.bound()).cloned().collect();
    js::each_declared_name(closure_body, &mut |n| inside.push(n.clone()));
    let given: Vec<&js::Name> = params.iter().flat_map(|p| p.bound()).collect();
    let mut outside: Vec<&js::Name> = Vec::new();
    let mut unknown = false;
    js::visit_stmt_names(closure_body, &mut |n| {
        if n.is_global() || n.is_module() || inside.contains(n) || given.contains(&n) || outside.contains(&n) {
            return;
        }
        unknown |= !n.has_id();
        outside.push(n);
    });
    !unknown && js::scopes::reach(body, name, &outside)
}

/// Whether `guard` is a variable of a function of `params` and `body`:
/// declared there, and, of a closure in the function a drop's written in,
/// not `whole`, by a parameter or its own statements' `const`, so no read
/// of it in the closure is of a variable outside. By its id, one
/// declaration: a tree copied whole, its declarations again, has fresh ids
/// (`js::rebound`), and one that doesn't `js::validate` refuses.
fn alone(params: &[Pattern], body: &[Stmt], guard: &js::Name, whole: bool) -> bool {
    let given = params.iter().any(|p| p.bound().contains(&guard));
    let mut declared = given;
    js::each_declared_name(body, &mut |n| declared |= n == guard);
    declared
        && (whole
            || given
            || body
                .iter()
                .any(|s| matches!(&s.kind, StmtKind::Const(n, _) if n == guard)))
}

/// What of a guard what runs after reads: the whole of it, given or read
/// otherwise, or the fields it reads, by key, `_restore.handler`'s
/// `handler`, `_restore[0]`'s `0` (rd429).
#[derive(Clone, Default)]
struct GuardReads {
    whole: bool,
    fields: IndexSet<String>,
}

impl GuardReads {
    /// Whether what runs after may read the field `key`.
    fn reads(&self, key: &str) -> bool {
        self.whole || self.fields.contains(key)
    }

    /// These and what `stmts` reads of `guard`: each field it reads or
    /// writes, or the whole, where it's named otherwise.
    fn add(&mut self, stmts: &[Stmt], guard: &js::Name) {
        let mentions = js::mentions_of(stmts, guard);
        if mentions == 0 {
            return;
        }
        let mut fields = 0;
        let mut copy = stmts.to_vec();
        js::each_expr_mut(&mut copy, &mut |e| {
            if let Some(key) = field_key(e, guard) {
                self.fields.insert(key);
                fields += 1;
            }
        });
        self.whole |= fields != mentions;
    }

    fn union(&mut self, other: GuardReads) {
        self.whole |= other.whole;
        self.fields.extend(other.fields);
    }
}

/// The key of `e`, a read of one of `guard`'s fields, `guard.key` or
/// `guard[0]`.
fn field_key(e: &Expr, guard: &js::Name) -> Option<String> {
    match &e.kind {
        ExprKind::Member(of, key) if matches!(&of.kind, ExprKind::Var(v) if v == guard) => Some(key.clone()),
        ExprKind::Index(of, at) if matches!(&of.kind, ExprKind::Var(v) if v == guard) => match at.kind {
            ExprKind::Num(n) => Some(n.to_string()),
            _ => None,
        },
        _ => None,
    }
}

/// Whether each mention of `guard` in `body` is of one of its fields,
/// `guard.a` or `guard[0]`: none of it whole, a pointer to it, `$ptrTo
/// (guard)`, or a value of it given or stored, through which a field may
/// be read where its name isn't.
fn fields_alone(body: &[Stmt], guard: &js::Name) -> bool {
    let mut fields = 0;
    let mut stmts = body.to_vec();
    js::each_expr_mut(&mut stmts, &mut |e| {
        if let ExprKind::Member(of, _) | ExprKind::Index(of, _) = &e.kind
            && matches!(&of.kind, ExprKind::Var(v) if v == guard)
        {
            fields += 1;
        }
    });
    fields == js::mentions_of(body, guard)
}

/// Of `stmts`, in which `guard` is made and dropped, each write of one of
/// its fields that nothing reads after, `_restore[0] = undefined;` of a
/// `Drop`'s `self.0.take()`: gone, as a guard is dropped once, and is
/// nothing after (ADR 0705, amended), though another field is read after,
/// `_restore.prevChunk` of the drop's next statement (rd429). `read_after`
/// says what of it what runs after `stmts` may read. Into a `try`, an `if`
/// and a labeled block, and a loop's body where it's made, fresh each time
/// round: what else runs again, or after a `catch`, is left as it is.
fn unread_writes(stmts: &mut Vec<Stmt>, guard: &js::Name, mut read_after: GuardReads) {
    let mut i = stmts.len();
    while i > 0 {
        i -= 1;
        if let StmtKind::Assign(target, value) = &stmts[i].kind
            && let Some(key) = field_key(target, guard)
            && !read_after.reads(&key)
            && !value.has_effects()
            && js::mentions_of(&[StmtKind::Expr(value.clone()).at(value.span)], guard) == 0
        {
            stmts.remove(i);
            continue;
        }
        let made_in = |body: &[Stmt]| {
            body.iter()
                .any(|s| matches!(&s.kind, StmtKind::Const(n, _) | StmtKind::Let(n, _) if n == guard))
        };
        match &mut stmts[i].kind {
            StmtKind::Try(block, finally) => {
                unread_writes(finally, guard, read_after.clone());
                let mut read_finally = read_after.clone();
                read_finally.add(finally, guard);
                unread_writes(block, guard, read_finally);
            }
            StmtKind::If(_, a, b) => {
                if let Some(b) = b {
                    unread_writes(b, guard, read_after.clone());
                }
                unread_writes(a, guard, read_after.clone());
            }
            StmtKind::Labeled(_, body) => unread_writes(body, guard, read_after.clone()),
            StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. }
                if made_in(body) =>
            {
                unread_writes(body, guard, GuardReads::default())
            }
            _ => {}
        }
        let mut read = GuardReads::default();
        read.add(&stmts[i..=i], guard);
        read_after.union(read);
    }
}

/// Each `const n = e;` of `body` of what `field` says is a read of a
/// guard's field nothing writes, which reads as `v`, the variable it was
/// made of or a `const` of its own, `const old = _restore[0];` of a
/// `Drop`'s `self.0.take()`: `n`, by its id, read as `v`, as nothing sets
/// either once it's made (ADR 0705, amended, #2026); never `undefined`
/// where `defined` says the
/// field was made of what isn't, as the field's own read is (ADR 0841,
/// amended): `initializingChunk = prevChunk` of the drop's `old ?? null`
/// (rd523).
fn copies_read_as(body: &mut Vec<Stmt>, field: &dyn Fn(&Expr) -> bool, v: &js::Name, defined: bool) {
    // Each copy by its id, one in each place the drop is its own: another
    // declaration of its word, a closure's `|old|` beside it, is another
    // variable, its reads its own (#2026); the `Drop`'s binding, `let Self {
    // previous, .. }`, one too, the variable the field was made of (ADR
    // 1150).
    let mut copies: Vec<js::Name> = Vec::new();
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
    if copies.is_empty() {
        return;
    }
    js::each_block_mut(body, &mut |stmts| {
        stmts.retain(|s| !matches!(&s.kind, StmtKind::Const(name, value) if copies.contains(name) && field(value)));
    });
    js::each_expr_mut(body, &mut |e| {
        if matches!(&e.kind, ExprKind::Var(name) if copies.contains(name)) {
            let read = Expr::var(v.clone()).or_at(e.span);
            *e = match defined {
                true => read.defined(),
                false => read,
            };
        }
    });
}

/// How often `item` names `name`.
fn mentions_in_item(item: &Item, name: &js::Name) -> usize {
    let value = |e: &Expr| js::mentions_of(&[StmtKind::Expr(e.clone()).at(e.span)], name);
    match item {
        Item::Namespace(namespace) => {
            (namespace.constants.iter()).map(|c| value(&c.value)).sum::<usize>()
                + (namespace.methods.iter())
                    .map(|f| js::mentions_of(&f.body, name))
                    .sum::<usize>()
        }
        Item::Const(constant) => value(&constant.value),
        Item::Statements(stmts) => js::mentions_of(stmts, name),
        Item::Function(function) => js::mentions_of(&function.body, name),
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

/// Whether `stmt` is `name(guard, ..);`, the guard and what else a generic
/// `Drop` is given, its type parameters' drops (ADR 0163).
fn calls(stmt: &Stmt, name: &js::Name) -> bool {
    matches!(&stmt.kind, StmtKind::Expr(e)
        if matches!(&e.kind, ExprKind::Call(callee, args)
            if !args.is_empty() && matches!(&callee.kind, ExprKind::Var(v) if v == name)))
}

/// The variable `f(guard);` gives.
fn argument(stmt: &Stmt) -> Option<js::Name> {
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
/// variable, `prototype` (ADR 0705). The `const`s it made of its fields.
fn fields_of(body: &mut Vec<Stmt>, params: &[Pattern], guard: &js::Name) -> Vec<js::Name> {
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
    let Some(props) = made else { return Vec::new() };
    let mut fields = Vec::new();
    for prop in &props {
        match prop {
            Prop::Field(key, value) => fields.push((key.clone(), value.clone())),
            _ => return Vec::new(),
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
    // Each field written through it, or lent `&mut`, a `let` of its own:
    // what `js::Writes` says is written, a swap's destructuring of it, `[name,
    // p.left] = ..`, `x++` and `(x = ..)` too (ADR 1088, amended, #1593).
    let written: IndexSet<String> = (js::Writes::places(body).into_iter())
        .filter_map(|place| match place.kind {
            ExprKind::Member(of, key) if matches!(&of.kind, ExprKind::Var(v) if v == guard) => Some(key),
            _ => None,
        })
        .collect();
    if reads != js::mentions_of(body, guard) {
        return Vec::new();
    }
    // What each field reads as: the variable it was made of, where nothing
    // sets that once the guard's made, and where each of its reads put
    // where the field's are reads it still, as JS's scopes say by its id,
    // in no nearer declaration's scope spelled as it is, which lowering may
    // leave (`js::scopes`, ADR 1150); else a `const` of its own, of its key,
    // a JS name.
    let mut kept = kept_variables(body, params, guard);
    loop {
        let mut made = body.clone();
        let read_as = |(key, value): &(String, Expr)| match &value.kind {
            ExprKind::Var(v) if !written.contains(key) && kept.contains(v) => Some(v.clone()),
            _ => None,
        };
        if (fields.iter()).any(|field| read_as(field).is_none() && !crate::names::is_identifier(&field.0)) {
            return Vec::new();
        }
        // Each field's own, of its key, a variable of its own, spelled by
        // preparation, its key where that's free, `previous$1` where it isn't
        // (ADR 1150), its `const` and its reads.
        let names = names_of(&made, params);
        let own: IndexMap<String, js::Name> = (fields.iter())
            .filter(|field| read_as(field).is_none())
            .map(|(key, _)| {
                (
                    key.clone(),
                    js::Name::declared(key.clone()).spelled_later(js::Name::free_made_k(&names, key)),
                )
            })
            .collect();
        let reads_as = |field: &(String, Expr)| read_as(field).unwrap_or_else(|| own[&field.0].clone());
        // A copy of a field nothing writes, `const old = guard.previous;`, is
        // what the field reads as.
        for field in fields.iter().filter(|(key, _)| !written.contains(key)) {
            let (key, v) = (&field.0, reads_as(field));
            let read = |e: &Expr| {
                matches!(&e.kind, ExprKind::Member(of, k)
                    if k == key && matches!(&of.kind, ExprKind::Var(g) if g == guard))
            };
            copies_read_as(&mut made, &read, &v, field.1.facts.has(js::Known::Defined));
        }
        // Each read of a field the value it was made of, never `undefined`
        // where that's what was known of it and nothing sets it again: what a
        // nullable place it's given stores as it is (ADR 0841, amended).
        js::each_expr_mut(&mut made, &mut |e| {
            if let ExprKind::Member(of, key) = &e.kind
                && matches!(&of.kind, ExprKind::Var(v) if v == guard)
                && let Some(field) = fields.iter().find(|(k, _)| k == key)
            {
                let read = Expr::var(reads_as(field)).or_at(e.span);
                *e = match field.1.facts.has(js::Known::Defined) && !written.contains(key) {
                    true => read.defined(),
                    false => read,
                };
            }
        });
        js::each_expr_mut(&mut made, &mut Expr::store_again);
        js::each_block_mut(&mut made, &mut |stmts| {
            if let Some(k) = stmts
                .iter()
                .position(|s| matches!(&s.kind, StmtKind::Const(n, _) if n == guard))
            {
                let span = stmts[k].span;
                let consts = (fields.iter())
                    .filter(|field| read_as(field).is_none())
                    .map(|(key, value)| match written.contains(key) {
                        true => StmtKind::Let(own[key].clone(), Some(value.clone())).at(span),
                        false => StmtKind::Const(own[key].clone(), value.clone()).at(span),
                    });
                stmts.splice(k..=k, consts.collect::<Vec<_>>());
            }
        });
        let read: Vec<js::Name> = fields.iter().filter_map(read_as).collect();
        let not = js::scopes::not_read_as_themselves(&made, &read.iter().collect::<Vec<_>>());
        if not.is_empty() {
            *body = made;
            return (fields.iter())
                .filter(|field| read_as(field).is_none())
                .map(|(key, _)| own[key].clone())
                .collect();
        }
        kept.retain(|v| !not.contains(v));
    }
}

/// Each variable `body` declares or reads, or `params` do: what a name
/// made of a base must be written apart from (`Name::free_made_k`).
fn names_of(body: &[Stmt], params: &[Pattern]) -> Vec<js::Name> {
    let mut names: Vec<js::Name> = params.iter().flat_map(|p| p.bound()).cloned().collect();
    js::each_declared_name(body, &mut |name| names.push(name.clone()));
    js::visit_stmt_names(body, &mut |name| names.push(name.clone()));
    names
}

/// The variables `body` declares, or `params` do, that nothing sets once
/// `const guard = ..;` is made: later where it's made, by a closure, or
/// through a handle; what the guard's field of one holds is that variable,
/// wherever the guard is read. Asked by their ids: a copy of the field, the
/// `Drop`'s binding, `let Self { previous, .. }`, is another variable, made
/// that one (`copies_read_as`); whether its reads put where the field's are
/// read it still, `js::scopes` says (`fields_of`, ADR 1150).
fn kept_variables(body: &[Stmt], params: &[Pattern], guard: &js::Name) -> IndexSet<js::Name> {
    let mut declared: IndexSet<js::Name> = IndexSet::default();
    js::each_declared_name(body, &mut |name| _ = declared.insert(name.clone()));
    declared.extend(params.iter().flat_map(|p| p.bound()).cloned());
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
        return IndexSet::default();
    };
    (declared.into_iter())
        .filter(|name| {
            !later.replaced.contains(name)
                && !later.handled.contains(name)
                && !whole.replaced_in_closures.contains(name)
                && !whole.handled.contains(name)
        })
        .collect()
}

/// `const guard = [x];`, a one-field tuple struct's guard, each of whose
/// reads is `guard[0]`, none written: `const guard = x;`, read as `guard`,
/// the field it's of (ADR 0608); and of a variable that nothing sets once
/// the guard's made, read as that variable, `previousFiber`, as a named
/// field of one is (`fields_of`, ADR 0705). The `const` it made, if it
/// made one.
fn one_field_of(body: &mut Vec<Stmt>, params: &[Pattern], guard: &js::Name) -> Option<js::Name> {
    let mut made = None;
    let mut guard_name = None;
    js::each_block_mut(body, &mut |stmts| {
        for s in stmts.iter() {
            if let StmtKind::Const(n, value) = &s.kind
                && n == guard
                && let ExprKind::Array(items) = &value.kind
                && let [item] = items.as_slice()
            {
                made.get_or_insert(item.clone());
                guard_name.get_or_insert(n.clone());
            }
        }
    });
    let (item, guard_name) = (made?, guard_name?);
    let known = item.facts;
    let field = |e: &Expr| {
        matches!(&e.kind, ExprKind::Index(of, at)
            if matches!(&of.kind, ExprKind::Var(v) if v == guard) && at.as_int() == Some(0))
    };
    let mut reads = 0;
    let mut stmts = body.clone();
    js::each_expr_mut(&mut stmts, &mut |e| reads += usize::from(field(e)));
    // Written, or lent `&mut`, as `js::Writes` says: a swap's `[guard[0],
    // pair.left] = ..` too (ADR 1088, amended, #1570).
    let written = js::Writes::places(body).iter().any(field);
    if written || reads != js::mentions_of(body, guard) {
        return None;
    }
    // The variable it was made of; or read now, so not by Rust's name for
    // what's only dropped, `_restore`, but `restore`, spelled by
    // preparation, `restore$1` where `restore` isn't free: its `_` taken
    // off as a parameter's is (`parameter_names::unmarked`), never where
    // what's left is a word JS keeps, `_in`'s `in`, or no name, `_9`'s `9`.
    // And read as it, where each read put where the field's are reads it
    // still, as `fields_of`'s are (`js::scopes`).
    let mut variable = match &item.kind {
        ExprKind::Var(v) if kept_variables(body, params, guard).contains(v) => Some(v.clone()),
        _ => None,
    };
    loop {
        let mut made = body.clone();
        let unmarked = crate::lower::parameter_names::unmarked(guard.word()); // word: the program's name, `_restore`, `_` its mark of one only dropped
        let read = match (&variable, unmarked) {
            (Some(v), _) => v.clone(),
            (None, Some(free)) => js::Name::declared(free.to_string())
                .spelled_later(js::Name::free_made_k(&names_of(&made, params), free)),
            _ => guard_name.clone(),
        };
        // A copy of it, `const old = _restore[0];`, is what it reads as too.
        copies_read_as(&mut made, &field, &read, known.has(js::Known::Defined));
        // Never `undefined` where that's what was known of it (ADR 0841,
        // amended), as `fields_of` reads one.
        js::each_expr_mut(&mut made, &mut |e| {
            if field(e) {
                let read = Expr::var(read.clone()).or_at(e.span);
                *e = match known.has(js::Known::Defined) {
                    true => read.defined(),
                    false => read,
                };
            }
        });
        js::each_expr_mut(&mut made, &mut Expr::store_again);
        js::each_block_mut(&mut made, &mut |stmts| {
            let made_here = |s: &Stmt| {
                matches!(&s.kind, StmtKind::Const(n, value)
                    if n == guard && matches!(&value.kind, ExprKind::Array(items) if items.len() == 1))
            };
            if variable.is_some() {
                stmts.retain(|s| !made_here(s));
            }
            for s in stmts.iter_mut() {
                if made_here(s)
                    && let StmtKind::Const(n, value) = &mut s.kind
                    && let ExprKind::Array(items) = &mut value.kind
                {
                    *n = read.clone();
                    *value = items.remove(0);
                }
            }
        });
        if let Some(v) = &variable
            && !js::scopes::not_read_as_themselves(&made, &[v]).is_empty()
        {
            variable = None;
            continue;
        }
        *body = made;
        return variable.is_none().then_some(read);
    }
}
