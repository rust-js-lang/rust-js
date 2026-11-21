//! `const n$1 = n;`, Rust's `let n = n;`, a variable shadowed by its own
//! value, is `n` itself (ADR 0277): `<Type />` for `let Type =
//! from_unknown(Type)`, `content` for `let content = content.clone()`,
//! where `n` holds that value wherever `n$1` is read.

use crate::collections::IndexSet;

use crate::js::{self, Expr, ExprKind, Function, Name, Pattern, Stmt, StmtKind};

/// `function`'s shadows of their own value, and each of its closures'.
pub(super) fn function(function: &mut Function) {
    let own = own_names(&function.params, &mut function.body);
    shadows(&mut function.body, &own, &IndexSet::default());
    written_back(&mut function.body, &own, &IndexSet::default());
    // A closure's own parameter, shadowed in it, or a variable it declares,
    // `for (const arg of args)`'s: nothing outside it sets one (ADR 0853).
    js::each_expr_mut(&mut function.body, &mut |e| {
        if let ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) = &mut e.kind {
            let own = own_names(params, body);
            let given: IndexSet<Name> = params.iter().flat_map(|p| p.bound()).cloned().collect();
            shadows(body, &own, &given);
        }
    });
}

/// The names a function or a closure of `params` and `body` declares: its
/// parameters and each variable its blocks declare, a loop's, a `catch`'s
/// and a function's too, which only it, and its closures, set.
fn own_names(params: &[Pattern], body: &mut Vec<Stmt>) -> IndexSet<Name> {
    let mut own: IndexSet<Name> = params.iter().flat_map(|p| p.bound()).cloned().collect();
    js::each_block_mut(body, &mut |stmts| {
        for stmt in stmts.iter() {
            match &stmt.kind {
                StmtKind::Const(name, _) | StmtKind::Let(name, _) | StmtKind::For { name, .. } => {
                    _ = own.insert(name.clone());
                }
                // Each name a pattern declares, `const [id, chunks] =
                // metadata;`'s, an or-pattern's alike arms' (rd712).
                StmtKind::Destructure { pattern, .. } => {
                    own.extend(pattern.bound().into_iter().cloned());
                }
                StmtKind::TryCatch(_, Some(name), _) => _ = own.insert(name.clone()),
                StmtKind::Function(function) => _ = own.insert(function.name.clone()),
                _ => {}
            }
        }
    });
    own
}

/// Each `const n$1 = n;` of `body` that's `n` (`shadow`), `n` in its place:
/// each read of the copy, by its id, a read of `n`, never a closure's own
/// of its word, its `catch_unwind`'s result (#457), a function's declared
/// beside it (#1766), nor a closure's parameter (ADR 1150); and only where
/// each of those reads is of `n` still, as JS's scopes say, none where
/// another of its spelling takes it, a function's own `const n = n$1`
/// beside it, React's Flight server's `busboyStream`
/// (`js::scopes::not_read_as_themselves`). Of no variable made of its word
/// alone, which no scope tells from another of its spelling.
fn shadows(body: &mut Vec<Stmt>, own: &IndexSet<Name>, given: &IndexSet<Name>) {
    let mut kept_apart: IndexSet<Name> = IndexSet::default();
    while let Some((alias, of)) = shadow(body, own, given, &kept_apart) {
        // Of a global, `const performance$1 = performance;`, each read the
        // global's still, as `Expr::global` made it (`js::validate`).
        let mut folded = body.clone();
        js::each_expr_mut(&mut folded, &mut |e| {
            if matches!(&e.kind, ExprKind::Var(name) if *name == alias) {
                e.kind = ExprKind::Var(of.clone());
            }
        });
        js::each_block_mut(&mut folded, &mut |stmts| {
            stmts.retain(|s| {
                !matches!(&s.kind, StmtKind::Const(name, value)
                    if *name == alias && matches!(&value.kind, ExprKind::Var(v) if *v == of))
            });
        });
        match js::scopes::not_read_as_themselves(&folded, &[&of]).is_empty() {
            true => *body = folded,
            false => _ = kept_apart.insert(alias),
        }
    }
}

/// A `const n$1 = n;` that `shadows` makes `n`, and `n`: where nothing sets
/// `n` after it, nor in a closure, nor where a loop runs it again. Of `n`
/// not of `own`, the function's or the closure's own, a module's variable
/// say, which a call may set out of its sight: only where each read of
/// `n$1` is before anything that may run code (`read_before_code`, ADR
/// 0277, amended).
fn shadow(
    body: &[Stmt],
    own: &IndexSet<Name>,
    given: &IndexSet<Name>,
    kept_apart: &IndexSet<Name>,
) -> Option<(Name, Name)> {
    // What closures set: what a read where the copy was may not be of.
    let in_closures = js::Writes::of(body).replaced_in_closures;
    let mut walk = Walk {
        given: given.clone(),
        ..Walk::default()
    };
    walk.stmts(body, false);
    walk.shadows.into_iter().find_map(|(at, looped, alias, of)| {
        // Of a variable with an id of its own, or a global or a module's
        // name: one made of its word alone neither `js::scopes` nor
        // `js::validate` tells from another of its spelling (`shadows`).
        if kept_apart.contains(&alias) || !(of.has_id() || of.is_global() || of.is_module()) {
            return None;
        }
        let sets: Vec<usize> = walk
            .sets
            .iter()
            .filter(|(_, name)| *name == of)
            .map(|&(set, _)| set)
            .collect();
        let rest = after(body, &alias);
        // Set after it only where nothing reads it after: in a loop too,
        // whose next turn makes it anew, React's popToNextHostParent's
        // `hydrationParentFiber = hydrationParentFiber.return` (ADR 0277,
        // amended).
        let unread_after = || {
            rest.is_some_and(|rest| !read_later(rest, &alias) && read_before_set(rest, &alias, &of, &mut false, true))
        };
        let kept = !in_closures.contains(&of)
            && (sets.is_empty() || !looped && sets.iter().all(|&set| set < at) || unread_after());
        let seen = own.contains(&of) || rest.is_some_and(|rest| read_before_code(rest, &alias, &mut false, true));
        (kept && seen).then_some((alias, of))
    })
}

/// Whether `stmts` read `alias` where they don't run: in a closure, or a
/// function, or a handle's accessors, `{ get value() { return list$1.next;
/// } }`, each of which reads it when it's called.
fn read_later(stmts: &[Stmt], alias: &Name) -> bool {
    let mut later = false;
    js::each_expr(stmts, &mut |e| {
        later |= match &e.kind {
            ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => js::mentions_of(body, alias) > 0,
            ExprKind::Function(function) => js::mentions_of(&function.body, alias) > 0,
            ExprKind::Handle(place, _) => place.mentions_name(alias),
            ExprKind::Pair(a, b, _) => a.mentions_name(alias) || b.mentions_name(alias),
            _ => false,
        };
    });
    js::each_list(stmts, &mut |list| {
        later |= (list.iter()).any(|s| matches!(&s.kind, StmtKind::Function(f) if js::mentions_of(&f.body, alias) > 0));
    });
    later
}

/// Whether each read of `alias` in `stmts`, in JS's order, is before
/// anything sets `of`: `set` whether something did already. A branch's way
/// each its own; a `switch`'s cases one after another, as one falls into
/// the next; a loop's, which goes round, only where it sets nothing or
/// reads nothing. `of = .. alias ..` reads it first. `top`, whether it's
/// the copy's list or an `if`'s of it (`after_ways`).
fn read_before_set(stmts: &[Stmt], alias: &Name, of: &Name, set: &mut bool, top: bool) -> bool {
    let reads = |stmts: &[Stmt]| js::mentions_of(stmts, alias) > 0;
    let sets = |stmts: &[Stmt]| may_replace(stmts, of);
    for stmt in stmts {
        let one = std::slice::from_ref(stmt);
        let ok = match &stmt.kind {
            StmtKind::If(test, a, b) => {
                let first = !*set || !test.mentions_name(alias);
                let mut other = *set;
                let b = b.as_deref().unwrap_or(&[]);
                let ok = read_before_set(a, alias, of, set, top) && read_before_set(b, alias, of, &mut other, top);
                *set = after_ways((a, b), (*set, other), top);
                first && ok
            }
            StmtKind::Switch {
                discriminant,
                cases,
                default,
            } => {
                let first = !*set || !discriminant.mentions_name(alias);
                first && js::case_lists(cases, default).all(|body| read_before_set(body, alias, of, set, false))
            }
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                read_before_set(a, alias, of, set, false) && read_before_set(b, alias, of, set, false)
            }
            StmtKind::Labeled(_, body) => read_before_set(body, alias, of, set, false),
            StmtKind::Assign(target, value)
                if matches!(&target.kind, ExprKind::Var(v) if v == of)
                    && !sets(&[StmtKind::Expr(value.clone()).at(js::Span::NONE)]) =>
            {
                let ok = !*set || !value.mentions_name(alias);
                *set = true;
                ok
            }
            _ => {
                let (read, sets) = (reads(one), sets(one));
                let ok = !read || !*set && !sets;
                *set |= sets;
                ok
            }
        };
        if !ok {
            return false;
        }
    }
    true
}

/// What an `if` whose ways are `a` and `b` leaves for what follows it of
/// what each way did, `in_a` and `in_b`, whether it ran code or set the
/// variable: where the `if` is `top`, of the copy's list or of its `if`s,
/// only the way that goes on reaches what follows, as one that `leaves`,
/// `if (callServer$1 == null) { throw new Error(..); }`, reaches none of
/// it, whatever it did (rd646, ADR 0277, amended). Not inside a `try`,
/// whose `catch` and what follows it a `throw` reaches, nor a labeled
/// block or a `switch`, which a way leaves for what follows them in the
/// list (#515): each way's then.
fn after_ways((a, b): (&[Stmt], &[Stmt]), (in_a, in_b): (bool, bool), top: bool) -> bool {
    let ends = |way: &[Stmt]| top && way.last().is_some_and(leaves);
    match (ends(a), ends(b)) {
        (true, false) => in_b,
        (false, true) => in_a,
        _ => in_a || in_b,
    }
}

/// Whether `stmt` reaches none of the statements after the copy: it
/// returns, throws, never returns, or goes round the loop that holds the
/// copy again, as a loop after the copy is looked at whole, never way by
/// way. Not a `break`, which leaves a labeled block or a `switch` those
/// statements may hold for what follows it: `done: { if (swap) {
/// callServer = triple; break done; } } return callServer$1(id);` reads
/// the copy, where `callServer` was set (ADR 0277, amended).
fn leaves(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Return(_) | StmtKind::Throw(_) | StmtKind::Continue(_) => true,
        StmtKind::Expr(_) => js::ends(stmt),
        StmtKind::If(_, a, Some(b)) => a.last().is_some_and(leaves) && b.last().is_some_and(leaves),
        _ => false,
    }
}

/// The statements after `const alias = ..` in the list that has it.
fn after<'s>(stmts: &'s [Stmt], alias: &Name) -> Option<&'s [Stmt]> {
    for (i, stmt) in stmts.iter().enumerate() {
        if matches!(&stmt.kind, StmtKind::Const(name, _) if name == alias) {
            return Some(&stmts[i + 1..]);
        }
        let found = match &stmt.kind {
            StmtKind::If(_, a, b) => after(a, alias).or_else(|| b.as_deref().and_then(|b| after(b, alias))),
            StmtKind::While { body, .. }
            | StmtKind::DoWhile { body, .. }
            | StmtKind::ForOf { body, .. }
            | StmtKind::For { body, .. }
            | StmtKind::Labeled(_, body) => after(body, alias),
            StmtKind::Switch { cases, default, .. } => {
                js::case_lists(cases, default).find_map(|body| after(body, alias))
            }
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => after(a, alias).or_else(|| after(b, alias)),
            _ => None,
        };
        if found.is_some() {
            return found;
        }
    }
    None
}

/// Whether each read of `alias` in `stmts`, in JS's order, is before
/// anything that may run code, a call, a getter or a setter, a module's
/// variable then what it was: `ran` whether something did already. A
/// branch's way each its own; a `try`'s block in its order, its `catch`
/// or `finally` after all of it; what a loop or a `switch` reads of it,
/// only where it runs nothing. `top`, whether it's the copy's list or an
/// `if`'s of it (`after_ways`).
fn read_before_code(stmts: &[Stmt], alias: &Name, ran: &mut bool, top: bool) -> bool {
    for stmt in stmts {
        let ok = match &stmt.kind {
            StmtKind::Const(_, e)
            | StmtKind::Let(_, Some(e))
            | StmtKind::Expr(e)
            | StmtKind::Return(Some(e))
            | StmtKind::Throw(e) => read_first(e, alias, ran),
            StmtKind::Assign(target, value) | StmtKind::Compound(_, target, value) => {
                read_first(target, alias, ran) && read_first(value, alias, ran)
            }
            StmtKind::If(test, a, b) => {
                let first = read_first(test, alias, ran);
                let mut other = *ran;
                let b = b.as_deref().unwrap_or(&[]);
                let ok = read_before_code(a, alias, ran, top) && read_before_code(b, alias, &mut other, top);
                *ran = after_ways((a, b), (*ran, other), top);
                first && ok
            }
            // A `try`'s `catch` or `finally` runs after any of its block,
            // which may have run code before it threw: `ran` is what the
            // block leaves it, whether anything in it ran (ADR 0277,
            // amended).
            StmtKind::Try(block, after) | StmtKind::TryCatch(block, _, after) => {
                read_before_code(block, alias, ran, false) && read_before_code(after, alias, ran, false)
            }
            StmtKind::Let(_, None)
            | StmtKind::Break(_)
            | StmtKind::Continue(_)
            | StmtKind::Return(None)
            | StmtKind::Directive(_)
            | StmtKind::Comments => true,
            _ => {
                let one = std::slice::from_ref(stmt);
                let runs = runs_code(one);
                let read = js::mentions_of(one, alias) > 0;
                *ran |= runs;
                !read || !runs && !*ran
            }
        };
        if !ok {
            return false;
        }
    }
    true
}

/// Whether each read of `alias` in `e`, in JS's order, is before anything
/// in it that may run code (`read_before_code`): a call's callee and
/// arguments before it runs, a property's object before its getter.
fn read_first(e: &Expr, alias: &Name, ran: &mut bool) -> bool {
    let one = [StmtKind::Expr(e.clone()).at(js::Span::NONE)];
    let read = js::mentions_of(&one, alias) > 0;
    if !runs_code(&one) {
        return !read || !*ran;
    }
    match &e.kind {
        ExprKind::Call(callee, args) | ExprKind::New(callee, args) | ExprKind::OptionalCall(callee, args) => {
            let ok = read_first(callee, alias, ran) && args.iter().all(|arg| read_first(arg, alias, ran));
            *ran = true;
            ok
        }
        // A property's object is read before its getter may run:
        // `activeElement.detachEvent(..)`'s receiver (rd146).
        ExprKind::Member(object, _) => {
            let ok = read_first(object, alias, ran);
            *ran = true;
            ok
        }
        ExprKind::Binary(_, a, b) => read_first(a, alias, ran) && read_first(b, alias, ran),
        ExprKind::Unary(_, a) => read_first(a, alias, ran),
        ExprKind::SomeWhere(test, yes) => read_first(test, alias, ran) && read_first(yes, alias, &mut ran.clone()),
        ExprKind::Cond(test, yes, no) => {
            let first = read_first(test, alias, ran);
            let mut other = *ran;
            let ok = read_first(yes, alias, ran) && read_first(no, alias, &mut other);
            *ran |= other;
            first && ok
        }
        _ => {
            *ran = true;
            !read
        }
    }
}

/// Whether `stmts` may run code where they run: a call, `new`, an
/// `await`, a JSX element, a dynamic import, or a binding's getter or
/// setter (`Known::Getter`, `Known::Setter`); not a closure made, whose
/// body runs where it's called, nor a binding's call that runs none of
/// the program's code (`Known::RunsNothing`), which can set none of its
/// variables, whose callee and arguments are looked at as any part is.
fn runs_code(stmts: &[Stmt]) -> bool {
    let mut runs = false;
    let mut stmts = stmts.to_vec();
    js::each_expr_mut(&mut stmts, &mut |e| {
        runs |= matches!(
            e.kind,
            ExprKind::Call(..) | ExprKind::New(..) | ExprKind::OptionalCall(..) if !e.facts.has(js::Known::RunsNothing)
        ) || matches!(e.kind, ExprKind::Await(..) | ExprKind::Jsx(..) | ExprKind::Import(..))
            || e.facts.has(js::Known::Getter)
            || e.facts.has(js::Known::Setter);
    });
    runs
}

/// The function's statements in order, outside its closures: each
/// `const n$1 = n;`, where, and whether in a loop, and each variable set.
#[derive(Default)]
struct Walk {
    /// The closure's parameters, of what it's given (`folds_into`).
    given: IndexSet<Name>,
    at: usize,
    shadows: Vec<(usize, bool, Name, Name)>,
    sets: Vec<(usize, Name)>,
}

impl Walk {
    fn stmts(&mut self, stmts: &[Stmt], looped: bool) {
        for stmt in stmts {
            self.at += 1;
            match &stmt.kind {
                StmtKind::Const(alias, value) => {
                    if let ExprKind::Var(of) = &value.kind
                        && folds_into(alias, of, &self.given)
                    {
                        self.shadows.push((self.at, looped, alias.clone(), of.clone()));
                    }
                }
                // Each variable it sets, as `js::Writes` says: each of a
                // destructuring one's, `[n, m] = ..` (ADR 1088, amended).
                StmtKind::Assign(..) | StmtKind::Compound(..) => {
                    for place in js::Writes::places(std::slice::from_ref(stmt)) {
                        if let ExprKind::Var(name) = place.kind {
                            self.sets.push((self.at, name));
                        }
                    }
                }
                StmtKind::If(_, a, b) => {
                    self.stmts(a, looped);
                    if let Some(b) = b {
                        self.stmts(b, looped);
                    }
                }
                StmtKind::While { body, .. } | StmtKind::DoWhile { body, .. } | StmtKind::ForOf { body, .. } => {
                    self.stmts(body, true)
                }
                StmtKind::For { name, body, .. } => {
                    self.sets.push((self.at, name.clone()));
                    self.stmts(body, true);
                }
                StmtKind::Labeled(_, body) => self.stmts(body, looped),
                StmtKind::Switch { cases, default, .. } => {
                    js::case_lists(cases, default).for_each(|body| self.stmts(body, looped));
                }
                StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                    self.stmts(a, looped);
                    self.stmts(b, looped);
                }
                StmtKind::Let(..)
                | StmtKind::Destructure { .. }
                | StmtKind::Expr(_)
                | StmtKind::Break(_)
                | StmtKind::Continue(_)
                | StmtKind::Directive(_)
                | StmtKind::Comments
                | StmtKind::Return(_)
                | StmtKind::Throw(_)
                | StmtKind::Function(_) => {}
            }
        }
    }
}

/// Each `let n$1 = n;` of `body` written back, a module's variable Rust
/// reads into a local, sets and stores again, `let text = TEXT.with_borrow(
/// Clone::clone); .. text = slice(..); TEXT.set(Some(text.clone()));`:
/// `n` itself where `n$1` is `n` wherever it's read, `n = slice(..);`, as
/// upstream's FallbackCompositionState writes `fallbackText` (ADR 0277,
/// amended). Each write of `n$1` is right before its `n = n$1;`, nothing
/// else sets `n` nor `n$1` in a closure, and each read of `n$1` is before
/// anything that may run code since the `let` or the last write back, of
/// `n` not the function's own.
fn written_back(body: &mut Vec<Stmt>, own: &IndexSet<Name>, given: &IndexSet<Name>) {
    while let Some((alias, of)) = write_back(body, own, given) {
        js::each_block_mut(body, &mut |stmts| {
            stmts.retain(|s| match &s.kind {
                StmtKind::Let(name, Some(value)) => {
                    !(*name == alias && matches!(&value.kind, ExprKind::Var(v) if *v == of))
                }
                StmtKind::Assign(target, value) => {
                    !(matches!(&target.kind, ExprKind::Var(t) if *t == of)
                        && matches!(&value.kind, ExprKind::Var(v) if *v == alias))
                }
                _ => true,
            });
        });
        // Its reads the variable's it was written back to, as it's one now:
        // none in a closure (`read_later`), where another of its name could
        // be declared.
        js::each_expr_mut(body, &mut |e| {
            if matches!(&e.kind, ExprKind::Var(name) if *name == alias) {
                e.kind = ExprKind::Var(of.clone());
            }
        });
    }
}

/// A `let n$1 = n;` that `written_back` makes `n`, and `n`.
fn write_back(body: &[Stmt], own: &IndexSet<Name>, given: &IndexSet<Name>) -> Option<(Name, Name)> {
    let mut lets = Vec::new();
    js::each_list(body, &mut |list| {
        for stmt in list {
            if let StmtKind::Let(alias, Some(value)) = &stmt.kind
                && let ExprKind::Var(of) = &value.kind
                && folds_into(alias, of, given)
            {
                lets.push((alias.clone(), of.clone()));
            }
        }
    });
    let in_closures = js::Writes::of(body).replaced_in_closures;
    lets.into_iter().find(|(alias, of)| {
        let is = |e: &Expr, name: &Name| matches!(&e.kind, ExprKind::Var(v) if v == name);
        // Each write of either: of `n$1` right before `n = n$1;`, of `n`
        // only that.
        let mut paired = true;
        let mut written = false;
        js::each_list(body, &mut |list| {
            for (i, stmt) in list.iter().enumerate() {
                match &stmt.kind {
                    StmtKind::Assign(target, _) if is(target, alias) => {
                        written = true;
                        paired &= list.get(i + 1).is_some_and(
                            |next| matches!(&next.kind, StmtKind::Assign(t, v) if is(t, of) && is(v, alias)),
                        );
                    }
                    StmtKind::Assign(target, value) if is(target, of) => paired &= is(value, alias),
                    StmtKind::Compound(_, target, _) if is(target, alias) || is(target, of) => paired = false,
                    StmtKind::For { name, .. } if name == alias || name == of => paired = false,
                    _ => {}
                }
            }
        });
        written
            && paired
            && !in_closures.contains(alias)
            && !in_closures.contains(of)
            && !read_later(body, alias)
            && (own.contains(of) || synced_reads(body, alias, of, &mut false))
    })
}

/// `read_before_code` of a `let n$1 = n;` written back: each read of
/// `alias` before anything that may run code since the `let`, or since the
/// last `of = alias;`, which makes them one again.
fn synced_reads(stmts: &[Stmt], alias: &Name, of: &Name, ran: &mut bool) -> bool {
    for stmt in stmts {
        let ok = match &stmt.kind {
            StmtKind::Assign(target, value)
                if matches!(&target.kind, ExprKind::Var(t) if t == of)
                    && matches!(&value.kind, ExprKind::Var(v) if v == alias) =>
            {
                *ran = false;
                true
            }
            StmtKind::Let(name, Some(_)) if name == alias => true,
            StmtKind::Assign(target, value) if matches!(&target.kind, ExprKind::Var(t) if t == alias) => {
                read_first(value, alias, ran)
            }
            StmtKind::If(test, a, b) => {
                let first = read_first(test, alias, ran);
                let mut other = *ran;
                let ok =
                    synced_reads(a, alias, of, ran) && synced_reads(b.as_deref().unwrap_or(&[]), alias, of, &mut other);
                *ran |= other;
                first && ok
            }
            _ => read_before_code(std::slice::from_ref(stmt), alias, ran, true),
        };
        if !ok {
            return false;
        }
    }
    true
}

/// Whether `stmts` may replace `of`, or lend it to be replaced
/// (`js::Writes`), by its id.
fn may_replace(stmts: &[Stmt], of: &Name) -> bool {
    let writes = js::Writes::of(stmts);
    writes.replaced.contains(of) || writes.handled.contains(of)
}

/// Whether `alias`, a `const` of `of`'s value, is `of` given apart, `p$1`
/// of `p`, or a holder made of it (`Name::derived`): a copy of a closure's
/// parameter, `p$13` of `p$12`, which reclaiming spells plainly once it's
/// folded, by its word's root (ADR 1150).
fn folds_into(alias: &Name, of: &Name, given: &IndexSet<Name>) -> bool {
    let base = |root: &str| match given.contains(of) {
        true => of.root() == root,
        false => of.is(root),
    };
    alias.apart_of().is_some_and(|(root, _)| base(root)) || of.is_origin_of(alias)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn function(name: Name, params: Vec<Name>, body: Vec<Stmt>) -> Function {
        Function {
            name,
            params: params.into_iter().map(Pattern::Name).collect(),
            body,
            export: false,
            is_async: false,
            noinline: false,
            span: js::Span::NONE,
            name_span: js::Span::NONE,
            param_spans: Vec::new(),
            set_apart: false,
        }
    }

    fn call(callee: &Name, of: &Name) -> Stmt {
        StmtKind::Expr(Expr::call(Expr::var(callee.clone()), vec![Expr::var(of.clone())])).at(js::Span::NONE)
    }

    // React's Flight server's `decodeReplyFromBusboy`: its copy of the
    // stream, `const busboyStream$1 = busboyStream`, read by a function
    // declared beside it into one of its own of the stream's name, `const
    // busboyStream = busboyStream$1`, stays: folded, that one would read
    // itself before it's made, which `js::validate` refuses
    // (`js::scopes::not_read_as_themselves`).
    #[test]
    fn a_copy_read_into_a_function_s_own_of_its_name_stays() {
        let stream = Name::program("busboyStream".into(), 1);
        let copy = Name::program("busboyStream$1".into(), 2);
        let own = Name::program("busboyStream".into(), 3);
        let destroy = Name::global("destroy");
        let flush = Name::declared("flush".into());
        let inner = function(
            flush.clone(),
            Vec::new(),
            vec![
                StmtKind::Const(own.clone(), Expr::var(copy.clone())).at(js::Span::NONE),
                call(&destroy, &own),
            ],
        );
        let mut outer = function(
            Name::declared("decode".into()),
            vec![stream.clone()],
            vec![
                StmtKind::Const(copy.clone(), Expr::var(stream.clone())).at(js::Span::NONE),
                StmtKind::Function(Box::new(inner)).at(js::Span::NONE),
                call(&flush, &stream),
            ],
        );
        super::function(&mut outer);
        assert!(matches!(&outer.body[0].kind, StmtKind::Const(name, _) if *name == copy));
        assert_eq!(js::validate::body(&outer.body), []);
    }

    // A copy of a variable made of its word alone, which no scope tells
    // from another of its spelling, a closure's own: never folded.
    #[test]
    fn a_copy_of_a_variable_of_its_word_alone_stays() {
        let value = Name::from("value");
        let copy = Name::program("value$1".into(), 1);
        let destroy = Name::global("destroy");
        let mut outer = function(
            Name::declared("decode".into()),
            vec![value.clone()],
            vec![
                StmtKind::Const(copy.clone(), Expr::var(value.clone())).at(js::Span::NONE),
                call(&destroy, &copy),
            ],
        );
        super::function(&mut outer);
        assert!(matches!(&outer.body[0].kind, StmtKind::Const(name, _) if *name == copy));
    }
}
