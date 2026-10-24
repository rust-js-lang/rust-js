//! Labels: a jump's exit after a block, the labels written and named,
//! and those never taken.

use super::*;
use std::collections::HashMap;

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
    if !matches!(cond.kind, ExprKind::Bool(true)) || span.is_none() || breaks(body, label.as_deref()) {
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
    let after = |s: &Stmt| s.span.is_none() || s.span.lo >= end;
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
    let at = |list: &[Stmt]| -> Option<usize> {
        let last = list.last().filter(|s| leaves(s))?;
        let mut k = list.len() - 1;
        while k > 0 && stays(&list[k - 1]) && after(&list[k - 1]) {
            k -= 1;
        }
        // Of what the program wrote after the loop, MIR's own beside it: a
        // `return` MIR made where its value is written.
        let at = |s: &Stmt| match &s.kind {
            StmtKind::Return(Some(value)) if s.span.is_none() => value.span,
            _ => s.span,
        };
        let written = list[k..].iter().any(|s| !at(s).is_none() && at(s).lo >= end);
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
    let span = moved[0].span;
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
    list.push(StmtKind::Break(None).at(span));
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
/// place, where they declare nothing its neighbors do; a loop, unlabeled.
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
            && label.as_deref().is_some_and(|l| !js::names_jump(body, l))
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

/// `if (false) { .. } else { rest }`: `rest`, what's always run.
pub(in crate::lower::mir::tidy) fn never_taken(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        if let StmtKind::If(test, _, els) = &mut stmts[i].kind
            && matches!(test.kind, ExprKind::Bool(false))
        {
            let rest = els.take().unwrap_or_default();
            let count = rest.len();
            stmts.splice(i..=i, rest);
            i += count;
            continue;
        }
        i += 1;
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
