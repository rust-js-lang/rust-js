//! `const n$1 = n;`, Rust's `let n = n;`, a variable shadowed by its own
//! value, is `n` itself (ADR 0277): `<Type />` for `let Type =
//! from_unknown(Type)`, `content` for `let content = content.clone()`,
//! where `n` holds that value wherever `n$1` is read.

use std::collections::HashSet;

use crate::js::{self, Expr, ExprKind, Function, Pattern, Stmt, StmtKind};

/// `function`'s shadows of their own value, and each of its closures'.
pub(super) fn function(function: &mut Function) {
    let own = own_names(&function.params, &mut function.body);
    shadows(&mut function.body, &own);
    // A closure's own parameter, shadowed in it, or a variable it declares,
    // `for (const arg of args)`'s: nothing outside it sets one (ADR 0853).
    js::each_expr_mut(&mut function.body, &mut |e| {
        if let ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) = &mut e.kind {
            let own = own_names(params, body);
            shadows(body, &own);
        }
    });
}

/// The names a function or a closure of `params` and `body` declares: its
/// parameters and each variable its blocks declare, a loop's, a `catch`'s
/// and a function's too, which only it, and its closures, set.
fn own_names(params: &[Pattern], body: &mut Vec<Stmt>) -> HashSet<String> {
    let mut own: HashSet<String> = params.iter().flat_map(|p| p.names()).map(str::to_string).collect();
    js::each_block_mut(body, &mut |stmts| {
        for stmt in stmts.iter() {
            match &stmt.kind {
                StmtKind::Const(name, _) | StmtKind::Let(name, _) | StmtKind::For { name, .. } => {
                    _ = own.insert(name.clone());
                }
                StmtKind::TryCatch(_, Some(name), _) => _ = own.insert(name.clone()),
                StmtKind::Function(function) => _ = own.insert(function.name.clone()),
                // Not a pattern's names, a `for…of`'s or a destructuring's:
                // a binding of one of them as it is, `let arg: &JsObject =
                // cast(arg)`, is that name already, never `const arg$1 =
                // arg;` (ADR 0853, amended).
                _ => {}
            }
        }
    });
    own
}

/// Each `const n$1 = n;` of `body` that's `n` (`shadow`), `n` in its place.
fn shadows(body: &mut Vec<Stmt>, own: &HashSet<String>) {
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

/// A `const n$1 = n;` that `shadows` makes `n`, and `n`: where nothing sets
/// `n` after it, nor in a closure, nor where a loop runs it again. Of `n`
/// not of `own`, the function's or the closure's own, a module's variable
/// say, which a call may set out of its sight: only where each read of
/// `n$1` is before anything that may run code (`read_before_code`, ADR
/// 0277, amended).
fn shadow(body: &[Stmt], own: &HashSet<String>) -> Option<(String, String)> {
    // What closures set, and what their parameters are named, which a
    // name read in them would mean instead.
    let mut in_closures = js::Writes::of(body).replaced_in_closures;
    js::each_expr(body, &mut |e| {
        if let ExprKind::Arrow(params, _) | ExprKind::AsyncArrow(params, _) = &e.kind {
            in_closures.extend(params.iter().flat_map(|p| p.names()).map(str::to_string));
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
        let rest = after(body, &alias);
        // Set after it only where nothing reads it after: in a loop too,
        // whose next turn makes it anew, React's popToNextHostParent's
        // `hydrationParentFiber = hydrationParentFiber.return` (ADR 0277,
        // amended).
        let unread_after =
            || rest.is_some_and(|rest| !read_later(rest, &alias) && read_before_set(rest, &alias, &of, &mut false));
        let kept = !in_closures.contains(&of)
            && (sets.is_empty() || !looped && sets.iter().all(|&set| set < at) || unread_after());
        let seen = own.contains(&of) || rest.is_some_and(|rest| read_before_code(rest, &alias, &mut false));
        (kept && seen).then_some((alias, of))
    })
}

/// Whether `stmts` read `alias` where they don't run: in a closure, or a
/// function, or a handle's accessors, `{ get value() { return list$1.next;
/// } }`, each of which reads it when it's called.
fn read_later(stmts: &[Stmt], alias: &str) -> bool {
    let mut later = false;
    js::each_expr(stmts, &mut |e| {
        later |= match &e.kind {
            ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => js::mentions_in(body, alias) > 0,
            ExprKind::Function(function) => js::mentions_in(&function.body, alias) > 0,
            ExprKind::Handle(place, _) => place.mentions_var(alias),
            ExprKind::Pair(a, b) => a.mentions_var(alias) || b.mentions_var(alias),
            _ => false,
        };
    });
    js::each_list(stmts, &mut |list| {
        later |= (list.iter()).any(|s| matches!(&s.kind, StmtKind::Function(f) if js::mentions_in(&f.body, alias) > 0));
    });
    later
}

/// Whether each read of `alias` in `stmts`, in JS's order, is before
/// anything sets `of`: `set` whether something did already. A branch's way
/// each its own; a `switch`'s cases one after another, as one falls into
/// the next; a loop's, which goes round, only where it sets nothing or
/// reads nothing. `of = .. alias ..` reads it first.
fn read_before_set(stmts: &[Stmt], alias: &str, of: &str, set: &mut bool) -> bool {
    let reads = |stmts: &[Stmt]| js::mentions_in(stmts, alias) > 0;
    let sets = |stmts: &[Stmt]| js::Writes::of(stmts).may_replace(of);
    for stmt in stmts {
        let one = std::slice::from_ref(stmt);
        let ok = match &stmt.kind {
            StmtKind::If(test, a, b) => {
                let first = !*set || !test.mentions_var(alias);
                let mut other = *set;
                let ok = read_before_set(a, alias, of, set)
                    && read_before_set(b.as_deref().unwrap_or(&[]), alias, of, &mut other);
                *set |= other;
                first && ok
            }
            StmtKind::Switch {
                discriminant,
                cases,
                default,
            } => {
                let first = !*set || !discriminant.mentions_var(alias);
                first && js::case_lists(cases, default).all(|body| read_before_set(body, alias, of, set))
            }
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                read_before_set(a, alias, of, set) && read_before_set(b, alias, of, set)
            }
            StmtKind::Labeled(_, body) => read_before_set(body, alias, of, set),
            StmtKind::Assign(target, value)
                if matches!(&target.kind, ExprKind::Var(v) if v == of)
                    && !sets(&[StmtKind::Expr(value.clone()).at(js::Span::NONE)]) =>
            {
                let ok = !*set || !value.mentions_var(alias);
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

/// The statements after `const alias = ..` in the list that has it.
fn after<'s>(stmts: &'s [Stmt], alias: &str) -> Option<&'s [Stmt]> {
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
/// only where it runs nothing.
fn read_before_code(stmts: &[Stmt], alias: &str, ran: &mut bool) -> bool {
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
                let ok =
                    read_before_code(a, alias, ran) && read_before_code(b.as_deref().unwrap_or(&[]), alias, &mut other);
                *ran |= other;
                first && ok
            }
            // A `try`'s `catch` or `finally` runs after any of its block,
            // which may have run code before it threw: `ran` is what the
            // block leaves it, whether anything in it ran (ADR 0277,
            // amended).
            StmtKind::Try(block, after) | StmtKind::TryCatch(block, _, after) => {
                read_before_code(block, alias, ran) && read_before_code(after, alias, ran)
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
                let read = js::mentions_in(one, alias) > 0;
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
/// arguments before it runs.
fn read_first(e: &Expr, alias: &str, ran: &mut bool) -> bool {
    let one = [StmtKind::Expr(e.clone()).at(js::Span::NONE)];
    let read = js::mentions_in(&one, alias) > 0;
    if !runs_code(&one) {
        return !read || !*ran;
    }
    match &e.kind {
        ExprKind::Call(callee, args) | ExprKind::New(callee, args) | ExprKind::OptionalCall(callee, args) => {
            let ok = read_first(callee, alias, ran) && args.iter().all(|arg| read_first(arg, alias, ran));
            *ran = true;
            ok
        }
        ExprKind::Binary(_, a, b) => read_first(a, alias, ran) && read_first(b, alias, ran),
        ExprKind::Unary(_, a) => read_first(a, alias, ran),
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
                        && crate::names::apart(alias).is_some_and(|(base, _)| base == of)
                    {
                        self.shadows.push((self.at, looped, alias.clone(), of.clone()));
                    }
                }
                StmtKind::Assign(target, _) | StmtKind::Compound(_, target, _) => {
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
