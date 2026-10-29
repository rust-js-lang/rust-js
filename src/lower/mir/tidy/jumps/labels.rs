//! Labels: a jump's exit after a block, the labels written and named,
//! and those never taken.

use super::*;
use crate::lower::mir::tidy::bindings::declared;
use std::collections::{HashMap, HashSet};

/// Of `exits_after`, the loop at `i` of `stmts`: what it does after it, its
/// way out made a `break`.
pub(super) fn exit_after(stmts: &mut [Stmt], i: usize) -> Option<Vec<Stmt>> {
    let Stmt {
        kind: StmtKind::While { label, cond, body },
        span,
    } = &stmts[i]
    else {
        return None;
    };
    if !matches!(cond.kind, ExprKind::Bool(true)) || span.is_none() {
        return None;
    }
    // Not of a loop that `break`s: what it does after its own `break` is
    // put after it as written by the stages before (ADR 0702, amended).
    if breaks(body, label.as_deref()) {
        return None;
    }
    // The labels inside it, whose jumps don't leave it.
    let mut inner: HashSet<String> = label.iter().cloned().collect();
    js::each_list(body, &mut |list| {
        for stmt in list.iter() {
            match &stmt.kind {
                StmtKind::Labeled(l, _)
                | StmtKind::While { label: Some(l), .. }
                | StmtKind::ForOf { label: Some(l), .. }
                | StmtKind::For { label: Some(l), .. } => {
                    inner.insert(l.clone());
                }
                _ => {}
            }
        }
    });
    let end = span.hi;
    // The spans of what a block MIR made holds, which has none of its own:
    // where it's written is where they are.
    let held = |s: &Stmt| {
        let mut spans = Vec::new();
        if s.span.is_none() && matches!(s.kind, StmtKind::Labeled(..)) {
            js::each_list(std::slice::from_ref(s), &mut |list| {
                spans.extend(list.iter().map(|t| t.span).filter(|t| !t.is_none()))
            });
        }
        spans
    };
    let after = |s: &Stmt| match s.span.is_none() {
        true => held(s).iter().all(|t| t.lo >= end),
        false => s.span.lo >= end,
    };
    // What goes on where it's put: no jump to the loop, nor to a label
    // inside it, but one of its own.
    let stays = |s: &Stmt| {
        let mut own: HashSet<String> = HashSet::new();
        let mut jumps: Vec<String> = Vec::new();
        js::each_list(std::slice::from_ref(s), &mut |list| {
            for stmt in list.iter() {
                match &stmt.kind {
                    StmtKind::Labeled(l, _)
                    | StmtKind::While { label: Some(l), .. }
                    | StmtKind::ForOf { label: Some(l), .. }
                    | StmtKind::For { label: Some(l), .. } => {
                        own.insert(l.clone());
                    }
                    StmtKind::Break(Some(l)) | StmtKind::Continue(Some(l)) => jumps.push(l.clone()),
                    _ => {}
                }
            }
        });
        // One of no label it can't hold: the loop has no `break`, and a
        // `continue` of it isn't written after it.
        !jumps.iter().any(|l| inner.contains(l) && !own.contains(l))
    };
    // A loop nothing `break`s, the program's next one, never goes on
    // either: it leaves as its `return` does, where none of its jumps is to
    // a label in this one, `break bb22` of a block it's in.
    let leaves = |s: &Stmt| match &s.kind {
        StmtKind::Return(_) | StmtKind::Throw(_) => true,
        StmtKind::Break(Some(l)) | StmtKind::Continue(Some(l)) => !inner.contains(l),
        StmtKind::While { label, cond, body } => {
            matches!(cond.kind, ExprKind::Bool(true)) && !breaks(body, label.as_deref()) && stays(s)
        }
        _ => false,
    };
    // Each way out, where what it does from `k` on was written after the
    // loop: its list's path, and `k`.
    fn sites(
        stmts: &[Stmt],
        path: &mut Vec<(usize, bool)>,
        found: &mut Vec<(Vec<(usize, bool)>, usize)>,
        at: &dyn Fn(&[Stmt]) -> Option<usize>,
    ) {
        // What's moved, its own ways out too, moves with it.
        let mut end = stmts.len();
        if let Some(k) = at(stmts) {
            found.push((path.clone(), k));
            end = k;
        }
        for (i, stmt) in stmts[..end].iter().enumerate() {
            match &stmt.kind {
                StmtKind::If(_, then, otherwise) => {
                    path.push((i, false));
                    sites(then, path, found, at);
                    path.pop();
                    if let Some(otherwise) = otherwise {
                        path.push((i, true));
                        sites(otherwise, path, found, at);
                        path.pop();
                    }
                }
                StmtKind::Labeled(_, body) => {
                    path.push((i, false));
                    sites(body, path, found, at);
                    path.pop();
                }
                _ => {}
            }
        }
    }
    // An `if` each of whose branches leaves so, `if (total > 10) { break
    // bb18; } else { continue bb1; }`, leaves too.
    fn leaving(s: &Stmt, leaves: &dyn Fn(&Stmt) -> bool) -> bool {
        leaves(s)
            || matches!(&s.kind, StmtKind::If(_, then, Some(otherwise))
                if [then, otherwise].iter().all(|b| b.last().is_some_and(|s| leaving(s, leaves))))
    }
    let at = |list: &[Stmt]| -> Option<usize> {
        let last = list.last().filter(|s| leaving(s, &leaves))?;
        let mut k = list.len() - 1;
        while k > 0 && stays(&list[k - 1]) && after(&list[k - 1]) {
            k -= 1;
        }
        // Of what the program wrote after the loop, MIR's own beside it: a
        // `return` MIR made where its value is written, a tuple's, `(i,
        // n)`, which MIR made of its parts, where they are.
        let at = |s: &Stmt| match &s.kind {
            StmtKind::Return(Some(value)) if s.span.is_none() => {
                let mut at = value.span;
                value.each(&mut |e: &Expr| {
                    if at.is_none() {
                        at = e.span;
                    }
                });
                at
            }
            _ => s.span,
        };
        let written =
            (list[k..].iter()).any(|s| (!at(s).is_none() && at(s).lo >= end) || held(s).iter().any(|t| t.lo >= end));
        (written && after(last)).then_some(k)
    };
    let mut found = Vec::new();
    sites(body, &mut Vec::new(), &mut found, &at);
    let [(path, k)] = found.as_slice() else {
        return None;
    };
    let (path, k) = (path.clone(), *k);
    // What the loop declares, but what's moved: none of it read there.
    let mut kept = body.clone();
    let moved = at_path(&mut kept, &path).split_off(k);
    // Its `break`, where what's moved ends, its last character's, however
    // deep: what's written before it, a comment, is the moved code's, after
    // the loop, and the `break` is no part of the loop's body as written
    // (ADR 0702, amended).
    let mut moved_end = 0;
    js::each_list(&moved, &mut |list| {
        moved_end = list.iter().map(|s| s.span.hi).fold(moved_end, u32::max)
    });
    let at = js::Span {
        lo: moved_end.saturating_sub(1),
        hi: moved_end,
    };
    let jump = StmtKind::Break(None).at(at);
    let made: HashSet<&str> = declared(&moved).collect();
    let mut own: HashSet<String> = HashSet::new();
    js::statement_lists(&mut kept, &mut |list| own.extend(declared(list).map(str::to_string)));
    if (own.iter()).any(|name| !made.contains(name.as_str()) && js::mentions_in(&moved, name) > 0) {
        return None;
    }
    let StmtKind::While { body, .. } = &mut stmts[i].kind else {
        unreachable!("matched")
    };
    let list = at_path(body, &path);
    list.truncate(k);
    list.push(jump);
    Some(moved)
}

/// The list at `path` in `stmts`: each step an `if`'s branch, its `else` of
/// `true`, or a block's statements.
fn at_path<'a>(stmts: &'a mut Vec<Stmt>, path: &[(usize, bool)]) -> &'a mut Vec<Stmt> {
    let mut list = stmts;
    for &(at, otherwise) in path {
        list = match &mut list[at].kind {
            StmtKind::If(_, _, Some(branch)) if otherwise => branch,
            StmtKind::If(_, then, _) | StmtKind::Labeled(_, then) => then,
            _ => unreachable!("a path"),
        };
    }
    list
}

/// The loop the program wrote that starts where `span` does, innermost.
pub(in crate::lower::mir::tidy) fn written_loop(
    loops: &[(js::Span, bool, Option<String>)],
    span: js::Span,
) -> Option<&(js::Span, bool, Option<String>)> {
    (loops.iter())
        .filter(|(l, ..)| !span.is_none() && l.lo <= span.lo && span.hi <= l.hi)
        .min_by_key(|(l, ..)| l.hi - l.lo)
}

/// A label MIR gives by its block, `bb2`, of a loop the program labeled:
/// its label, where nothing around it has that one, `rows: for (…)`.
pub(in crate::lower::mir::tidy) fn named_labels(stmts: &mut [Stmt], loops: &[(js::Span, bool, Option<String>)]) {
    fn walk(stmts: &mut [Stmt], loops: &[(js::Span, bool, Option<String>)], around: &mut Vec<String>) {
        for stmt in stmts.iter_mut() {
            let span = stmt.span;
            let (label, body, written) = match &mut stmt.kind {
                StmtKind::While { label, body, .. }
                | StmtKind::ForOf { label, body, .. }
                | StmtKind::For { label, body, .. } => {
                    let Some(label) = label else {
                        walk(body, loops, around);
                        continue;
                    };
                    (label, body, written_loop(loops, span).and_then(|(_, _, l)| l.as_ref()))
                }
                StmtKind::Labeled(label, body) => (label, body, None),
                _ => continue,
            };
            if crate::lower::mir::is_block_label(label)
                && let Some(to) = written
                && !around.contains(to)
            {
                relabeled(body, label, to);
                *label = to.clone();
            }
            around.push(label.clone());
            walk(body, loops, around);
            around.pop();
        }
    }
    walk(stmts, loops, &mut Vec::new());
}

/// A block the program labeled, `'desc: { .. }`, by its name before
/// anything else reads the body: the JS block MIR's breaks leave whose end
/// a `break` the program wrote of it goes to, `desc: { .. }` (ADR 0742),
/// where no label around it has that name. A `break` written of it that MIR
/// sends to a block inside it first, one past the drops at its end, which
/// JS has none of, goes to it: a block followed by `break desc`, or that
/// ends it, leaves it as `break desc` does. Named so, it's the program's
/// from the first, and no pass that reads a block MIR made
/// (`is_block_label`) makes its `break`s something else: each is where it's
/// written (ADR 0790).
pub(in crate::lower::mir::tidy) fn written_blocks(body: &mut Vec<Stmt>, blocks: &[LabeledBlock]) {
    let written = |span: js::Span| (blocks.iter()).find(|b| !span.is_none() && b.span == span);
    // Where each JS block is written, by its label.
    let mut spans: HashMap<String, js::Span> = HashMap::new();
    js::each_list(body, &mut |list| {
        for stmt in list {
            if let StmtKind::Labeled(l, _) = &stmt.kind {
                spans.insert(l.clone(), stmt.span);
            }
        }
    });
    // The blocks of a labeled block's, past its end's drops: each one's
    // `break`s, its end's.
    js::statement_lists(body, &mut |stmts| {
        for i in 0..stmts.len() {
            let to = match (stmts.get(i + 1).map(|s| &s.kind), i + 1 == stmts.len()) {
                (Some(StmtKind::Break(Some(to))), _) => Some(to.clone()),
                _ => None,
            };
            if let Some(to) = to
                && written(stmts[i].span).is_some()
                && spans.get(&to) == Some(&stmts[i].span)
                && let StmtKind::Labeled(label, inner) = &mut stmts[i].kind
                && crate::lower::mir::is_block_label(label)
            {
                let from = label.clone();
                relabeled(inner, &from, &to);
            }
        }
        // A block of one that ends it, where it ends too.
        for stmt in stmts.iter_mut() {
            if written(stmt.span).is_some()
                && let StmtKind::Labeled(outer, inner) = &mut stmt.kind
                && let Some(Stmt {
                    kind: StmtKind::Labeled(last, last_body),
                    span,
                }) = inner.last_mut()
                && written(*span).is_some()
                && crate::lower::mir::is_block_label(last)
            {
                let from = last.clone();
                relabeled(last_body, &from, outer);
            }
        }
    });
    // Its name, where no other label has it: no loop's, nor another block's
    // the program labeled so.
    let mut taken: HashMap<String, usize> = HashMap::new();
    js::each_list(body, &mut |list| {
        for stmt in list {
            if let StmtKind::Labeled(l, _)
            | StmtKind::While { label: Some(l), .. }
            | StmtKind::ForOf { label: Some(l), .. }
            | StmtKind::For { label: Some(l), .. } = &stmt.kind
            {
                *taken.entry(l.clone()).or_default() += 1;
            }
        }
    });
    for block in blocks {
        *taken.entry(block.label.clone()).or_default() += 1;
    }
    js::statement_lists(body, &mut |stmts| {
        for stmt in stmts.iter_mut() {
            let name = written(stmt.span).map(|b| b.label.clone());
            if let StmtKind::Labeled(label, inner) = &mut stmt.kind
                && let Some(to) = name
                && crate::lower::mir::is_block_label(label)
                && taken.get(&to) == Some(&1)
                && js::names_jump(inner, label)
            {
                relabeled(inner, label, &to);
                taken.insert(to.clone(), 2);
                *label = to;
            }
        }
    });
}

/// Whether `inner`, in place of the statement at `i` of `stmts`, declares
/// nothing declared beside it.
pub(in crate::lower::mir::tidy) fn fits_in_place(stmts: &[Stmt], i: usize, inner: &[Stmt]) -> bool {
    let inner: HashSet<&str> = declared(inner).collect();
    !(stmts.iter().enumerate())
        .any(|(j, s)| j != i && declared(std::slice::from_ref(s)).any(|name| inner.contains(name)))
}

/// A label nothing breaks to or continues: a block's statements in its
/// place, where they declare nothing its neighbors do; a loop, unlabeled,
/// where MIR made the label: one the program wrote stays through tidy,
/// which preparation drops where nothing names it then (wl4).
pub(in crate::lower::mir::tidy) fn unlabeled(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        let splice = matches!(&stmts[i].kind, StmtKind::Labeled(label, body)
            if !js::names_jump(body, label) && fits_in_place(stmts, i, body));
        if splice {
            let StmtKind::Labeled(_, body) = stmts.remove(i).kind else {
                unreachable!("matched")
            };
            stmts.splice(i..i, body);
            // What it held is looked at again: a label inside it, now here.
            continue;
        }
        if let StmtKind::While { label, body, .. }
        | StmtKind::ForOf { label, body, .. }
        | StmtKind::For { label, body, .. } = &mut stmts[i].kind
            && label
                .as_deref()
                .is_some_and(|l| crate::lower::mir::is_block_label(l) && !js::names_jump(body, l))
        {
            *label = None;
        }
        i += 1;
    }
}

/// Of `stmts`, run while `name` is still `""`: `f += s; return f` is
/// `return s`, and `return f` is `return ""`, into each branch that
/// starts so.
pub(in crate::lower::mir::tidy) fn returned_unwritten(stmts: &mut Vec<Stmt>, name: &str) {
    let is_name = |e: &Expr| matches!(&e.kind, ExprKind::Var(v) if v == name);
    let mut i = 0;
    while i < stmts.len() {
        let returned = matches!(stmts.get(i + 1).map(|s| &s.kind), Some(StmtKind::Return(Some(r))) if is_name(r));
        let mentioned = js::mentions_in(std::slice::from_ref(&stmts[i]), name) > 0;
        let next = stmts.get(i + 1).cloned();
        match &mut stmts[i].kind {
            StmtKind::Assign(target, value)
                if is_name(target)
                    && let ExprKind::Binary(Op::Add, lhs, written) = &value.kind
                    && is_name(lhs)
                    && returned =>
            {
                let written = (**written).clone();
                let span = stmts[i].span;
                stmts.splice(i..i + 2, [StmtKind::Return(Some(written)).at(span)]);
                return;
            }
            StmtKind::Return(Some(r)) if is_name(r) => {
                *r = Expr::str("");
                return;
            }
            // An `if` and `else` the `return f` after ends: each its own.
            StmtKind::If(_, then, Some(els)) if returned && let Some(back) = next => {
                then.push(back.clone());
                els.push(back);
                stmts.remove(i + 1);
                continue;
            }
            StmtKind::If(_, then, els) => {
                returned_unwritten(then, name);
                if let Some(els) = els {
                    returned_unwritten(els, name);
                }
                if js::mentions_in(std::slice::from_ref(&stmts[i]), name) > 0 {
                    return;
                }
            }
            _ if mentioned => return,
            _ => {}
        }
        i += 1;
    }
}

/// What an `if` always runs, in its place: of `if (false) { .. } else {
/// rest }`, `rest`, and of `if (true) { rest } else { .. }`, an
/// or-pattern's first alternative taken (#247), where what `rest` declares
/// isn't declared beside it; of an `if` that runs nothing either way, a
/// drop flag's whose branch dropped nothing, `if (_2$live) {}`, its test
/// where it does anything, else nothing.
pub(in crate::lower::mir::tidy) fn never_taken(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        let span = stmts[i].span;
        // Whether what a branch declares is declared nowhere beside the
        // `if`, so it can take the `if`'s place.
        let constant = matches!(&stmts[i].kind, StmtKind::If(test, ..) if matches!(test.kind, ExprKind::Bool(_)));
        let beside: HashSet<&str> = match constant {
            true => (stmts.iter().enumerate())
                .filter(|(k, _)| *k != i)
                .flat_map(|(_, s)| declared(std::slice::from_ref(s)))
                .collect(),
            false => HashSet::new(),
        };
        let beside: HashSet<String> = beside.into_iter().map(str::to_string).collect();
        let fits = |rest: &[Stmt]| declared(rest).all(|n| !beside.contains(n));
        if let StmtKind::If(test, then, els) = &mut stmts[i].kind {
            let rest = if matches!(test.kind, ExprKind::Bool(false)) && fits(els.as_deref().unwrap_or_default()) {
                Some(els.take().unwrap_or_default())
            } else if matches!(test.kind, ExprKind::Bool(true)) && fits(then) {
                Some(std::mem::take(then))
            } else if then.is_empty() && els.as_ref().is_none_or(|els| els.is_empty()) {
                let test = std::mem::replace(test, Expr::bool(false));
                Some(if test.has_effects() {
                    vec![StmtKind::Expr(test).at(span)]
                } else {
                    Vec::new()
                })
            } else {
                None
            };
            // What's put in its place is read again: an `if` of nothing
            // may be in an `else` taken.
            if let Some(rest) = rest {
                stmts.splice(i..=i, rest);
                continue;
            }
        }
        i += 1;
    }
}

/// The `switch` a block the program labeled leaves, labeled by it, where a
/// `break` of it is in a `switch` or a loop inside a case, which a plain
/// `break` would leave instead: React's `getTag: switch (type) { ..
/// default: switch (type.$$typeof) { case X: ..; break getTag; } }` (ADR
/// 0944). MIR leaves the block around the `switch`, `L: { switch (..) { ..
/// } } return tag;`, each case's end a copy of the `return`; or around the
/// inner `switch`, in a case, followed by a jump that leaves the outer one,
/// `L: { switch (..) { .. break L; } } break;`: then the block's statements
/// in its place, the jump gone where they never go on to it, and the outer
/// `switch` labeled `L`. Each `break L` of a case's own, outside a `switch`
/// or a loop of its own, is the case's `break;`, and each copy of the
/// `return` after it too, an `if` written with no `else` then none (ADR
/// 0805); a label nothing names then, gone. A block MIR made
/// (`is_block_label`) labels nothing.
pub(in crate::lower::mir::tidy) fn labeled_switches(body: &mut [Stmt], elses: &[(js::Span, js::Span, js::Span)]) {
    /// `stmts`, whose end is that of the blocks of `ending` and goes on to
    /// the `return` of `then`.
    fn walk(
        stmts: &mut [Stmt],
        ending: &[String],
        then: Option<&Option<Expr>>,
        elses: &[(js::Span, js::Span, js::Span)],
    ) {
        let n = stmts.len();
        // What each statement goes on to, a `return` after it.
        let next = |stmts: &[Stmt], i: usize| match stmts.get(i + 1).map(|s| &s.kind) {
            Some(StmtKind::Return(value)) => Some(value.clone()),
            Some(_) => None,
            None => then.cloned(),
        };
        for i in 0..n {
            let leaving = if i + 1 == n { ending } else { &[] };
            let returned = next(stmts, i);
            if let Some(label) = left_by(&mut stmts[i], leaving, returned.as_ref()) {
                let switch = std::mem::replace(&mut stmts[i], StmtKind::Comments.at(js::Span::NONE));
                let span = switch.span;
                stmts[i] = StmtKind::Labeled(label, vec![switch]).at(span);
            }
            labeled_cases(&mut stmts[i], returned.as_ref(), elses);
        }
        for i in 0..n {
            let ending: &[String] = if i + 1 == n { ending } else { &[] };
            let returned = next(stmts, i);
            match &mut stmts[i].kind {
                StmtKind::Labeled(label, body) => {
                    let ending: Vec<String> = ending.iter().cloned().chain([label.clone()]).collect();
                    walk(body, &ending, returned.as_ref(), elses);
                }
                StmtKind::If(_, a, b) => {
                    walk(a, ending, returned.as_ref(), elses);
                    if let Some(b) = b {
                        walk(b, ending, returned.as_ref(), elses);
                    }
                }
                StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
                    walk(body, &[], None, elses)
                }
                StmtKind::Switch { cases, default, .. } => {
                    js::case_lists_mut(cases, default).for_each(|case| walk(case, &[], None, elses));
                }
                StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                    walk(a, &[], None, elses);
                    walk(b, &[], None, elses);
                }
                _ => {}
            }
        }
    }
    walk(body, &[], None, elses);
}

/// Of `L: switch (..) { .. }`, a label the program wrote: each copy of
/// `returned`, the `return` after it, the case's `break;`, an `if` written
/// with no `else` then none (ADR 0805). Each `break L` of a case's own is
/// the case's `break;` later, as a block's is (`breaks_past_switches`).
fn labeled_cases(stmt: &mut Stmt, returned: Option<&Option<Expr>>, elses: &[(js::Span, js::Span, js::Span)]) {
    let StmtKind::Labeled(label, body) = &mut stmt.kind else {
        return;
    };
    let [
        Stmt {
            kind: StmtKind::Switch { cases, default, .. },
            span: written,
        },
    ] = &mut body[..]
    else {
        return;
    };
    let (Some(value), false) = (returned, crate::lower::mir::is_block_label(label)) else {
        return;
    };
    let ending = (cases.iter_mut())
        .filter(|case| !case.falls)
        .map(|case| &mut case.body)
        .chain(default.as_mut());
    for case in ending {
        super::super::switches::returns_broken(case, value.as_ref(), *written);
        // An `if` written with no `else`, whose `then` leaves now.
        super::super::switches::unwritten_else(case, elses);
    }
}

/// Of a `switch`, the label of each block the program labeled that a case
/// ends with, followed by a jump that leaves the `switch`, through the
/// `if`s and blocks it's in: its `break;`, a `break` of a block of
/// `leaving`, whose end the `switch` ends, or MIR's copy of the `return`
/// it goes on to, `returned`'s. The one label of them all: each such
/// block's statements in its place, its `break`s the `switch`'s now, and
/// the jump after it gone where they never go on to it.
fn left_by(switch: &mut Stmt, leaving: &[String], returned: Option<&Option<Expr>>) -> Option<String> {
    let leaves = |stmt: Option<&Stmt>| match (stmt.map(|s| &s.kind), returned) {
        (Some(StmtKind::Break(None)), _) => true,
        (Some(StmtKind::Break(Some(to))), _) => leaving.contains(to),
        (Some(StmtKind::Return(None)), Some(None)) => true,
        (Some(StmtKind::Return(Some(value))), Some(Some(after))) => super::super::switches::alike(value, after),
        _ => false,
    };
    fn blocks(stmts: &[Stmt], leaves: &dyn Fn(Option<&Stmt>) -> bool, found: &mut Vec<String>) {
        for (k, stmt) in stmts.iter().enumerate() {
            match &stmt.kind {
                StmtKind::Labeled(label, body)
                    if !crate::lower::mir::is_block_label(label)
                        && leaves(stmts.get(k + 1))
                        && js::names_jump(body, label)
                        && fits_in_place(stmts, k, body) =>
                {
                    found.push(label.clone());
                }
                StmtKind::Labeled(_, body) => blocks(body, leaves, found),
                StmtKind::If(_, a, b) => {
                    blocks(a, leaves, found);
                    blocks(b.as_deref().unwrap_or_default(), leaves, found);
                }
                _ => {}
            }
        }
    }
    fn unwrapped(stmts: &mut Vec<Stmt>, label: &str) {
        let mut k = 0;
        while k < stmts.len() {
            match &mut stmts[k].kind {
                StmtKind::Labeled(l, _) if l == label => {
                    let StmtKind::Labeled(_, body) = stmts.remove(k).kind else {
                        unreachable!("matched")
                    };
                    let n = body.len();
                    let goes_on = body.last().is_none_or(|last| !never_goes_on(last));
                    stmts.splice(k..k, body);
                    k += n;
                    if !goes_on
                        && matches!(
                            stmts.get(k).map(|s| &s.kind),
                            Some(StmtKind::Break(_) | StmtKind::Return(_))
                        )
                    {
                        stmts.remove(k);
                    }
                    continue;
                }
                StmtKind::Labeled(_, body) => unwrapped(body, label),
                StmtKind::If(_, a, b) => {
                    unwrapped(a, label);
                    if let Some(b) = b {
                        unwrapped(b, label);
                    }
                }
                _ => {}
            }
            k += 1;
        }
    }
    let StmtKind::Switch { cases, default, .. } = &mut switch.kind else {
        return None;
    };
    let mut found = Vec::new();
    js::case_lists(cases, default).for_each(|case| blocks(case, &leaves, &mut found));
    found.dedup();
    let [label] = &found[..] else {
        return None;
    };
    let label = label.clone();
    js::case_lists_mut(cases, default).for_each(|case| unwrapped(case, &label));
    Some(label)
}

/// Whether `stmt` never goes on to what's after it: a jump past it, a
/// `break` of a label, a `return`, a `throw` or a `continue`; or a `switch`
/// with a `default` each of whose cases ends with one, none of its own
/// `break;`.
fn never_goes_on(stmt: &Stmt) -> bool {
    let past = |s: &Stmt| {
        matches!(
            s.kind,
            StmtKind::Break(Some(_)) | StmtKind::Return(_) | StmtKind::Throw(_) | StmtKind::Continue(_)
        )
    };
    match &stmt.kind {
        StmtKind::Switch {
            cases,
            default: Some(default),
            ..
        } => cases.iter().all(|case| case.body.last().is_some_and(past)) && default.last().is_some_and(past),
        _ => past(stmt),
    }
}

/// A body's jumps settled once a pass has moved them: a `break` or a
/// `continue` that goes where the code goes on anyway, gone (`tail_jumps`),
/// then a label nothing names any more, gone (`unlabeled`). Run after any
/// pass that leaves a jump at a block's end.
pub(in crate::lower::mir::tidy) fn settle_jumps(body: &mut Vec<Stmt>) {
    js::statement_lists(body, &mut |stmts| tail_jumps(stmts));
    js::statement_lists(body, &mut unlabeled);
}

/// `l: while (true) { b: { if (x != null) { .. } break b; } return; }`, a
/// `while let` that ends a function's `body`, the `return;` MIR's of the
/// loop's way out, which is the function's end: the
/// loop's `break l;`, as a loop followed by code ends, so its test and its
/// `break`s are where the program wrote them, `while (parent != null) {
/// .. }`, as React's commitDeletionEffects walks its parents. Not a
/// `return;` the program wrote, which has its span.
pub(in crate::lower::mir::tidy) fn loop_ends_returned(body: &mut [Stmt]) {
    let Some(Stmt {
        kind: StmtKind::While {
            label: Some(label),
            cond,
            body: looped,
        },
        ..
    }) = body.last_mut()
    else {
        return;
    };
    if !matches!(cond.kind, ExprKind::Bool(true)) {
        return;
    }
    // A `while let`'s, its test's `if`, in a block MIR made or not, and the
    // way out after it, `l: while (true) { if (x != null) { .. } return; }`.
    let [first, last] = looped.as_mut_slice() else {
        return;
    };
    let tested = |s: &Stmt| matches!(s.kind, StmtKind::If(_, _, None));
    let tests = match &first.kind {
        StmtKind::Labeled(block, inner) => match &inner[..] {
            [test] => tested(test),
            [
                test,
                Stmt {
                    kind: StmtKind::Break(Some(b)),
                    ..
                },
            ] => tested(test) && b == block,
            _ => false,
        },
        _ => tested(first),
    };
    if tests && matches!(last.kind, StmtKind::Return(None)) && last.span.is_none() {
        last.kind = StmtKind::Break(Some(label.clone()));
    }
}
