//! What follows a block a `switch` is in that one case alone reaches: that
//! case's, as a case's body is written after a `// Fallthrough` to it
//! (ADR 0905, amended).

use crate::js::{self, Stmt, StmtKind};
use crate::lower::mir::hints::Switch;

/// Of a block `outer: { ..; inner: { ..; switch (..) { .. } break outer; }
/// rest }`, whose `rest` only `break inner` reaches, from one case alone,
/// its last statement: `rest` that case's, then its `break outer`, and the
/// block `inner`, which nothing names then, gone. MIR placed the
/// statements of the arm a guarded case falls to past the `match`, the
/// `break inner` of each arm that reached them (ADR 0905); one case is
/// all that reaches them where the others fall to it.
///
/// The case's `break inner` may end a branch of an `if` in it, a guard
/// that otherwise falls to the next case, as a guarded arm that a guarded
/// arm falls to has, `case HostSingleton: if (supportsSingletons) { break
/// bb17; }`: `rest` that branch's. And a block that is the last of the
/// blocks it's in, `bb29: { bb6: { bb17: { switch .. } .. } .. }`, of a
/// chain of guarded arms each falling to the next, is the inner one's
/// first, so the `switch` is the next one's (ADR 0905, amended).
pub(super) fn blocks_one_case_reaches(stmts: &mut [Stmt]) {
    for stmt in stmts.iter_mut() {
        let StmtKind::Labeled(outer, list) = &mut stmt.kind else {
            continue;
        };
        let outer = outer.clone();
        blocks_one_case_reaches(list);
        let Some(i) = (list.iter()).rposition(|s| matches!(s.kind, StmtKind::Labeled(..))) else {
            continue;
        };
        // What a `break` of no label in it leaves, the loop's, it would
        // leave the `switch` in a case.
        if i + 1 == list.len() || js::breaks(&list[i + 1..], None) {
            continue;
        }
        let rest = list[i + 1..].to_vec();
        let StmtKind::Labeled(inner, body) = &mut list[i].kind else {
            unreachable!("matched")
        };
        let inner = inner.clone();
        // `.. switch (..) { .. } break outer;`, or a `switch` that leaves
        // itself, the `switch` alone naming `inner`, by one case's last
        // statement; or one followed by what leaves, the rest of a loop's
        // turn, `parent = parent.return; continue;`, which the other cases
        // go on to (ADR 0702, amended).
        let broken = matches!(body.last().map(|s| &s.kind), Some(StmtKind::Break(Some(l))) if *l == outer);
        let Some(at) = (body.iter()).rposition(|s| matches!(s.kind, StmtKind::Switch { .. })) else {
            continue;
        };
        let (head, tail) = body.split_at_mut(at);
        let (switch, tail) = tail.split_first_mut().expect("the switch");
        let left = match tail {
            [] => js::ends(switch),
            [_] if broken => true,
            _ => tail.last().is_some_and(js::ends) && !js::names_jump(tail, &inner),
        };
        if !left || js::names_jump(head, &inner) {
            continue;
        }
        let broken = broken && tail.len() == 1;
        let StmtKind::Switch { cases, default, .. } = &mut switch.kind else {
            continue;
        };
        let reaching: Vec<usize> = (js::case_lists(cases, default).enumerate())
            .filter(|(_, case)| js::names_jump(case, &inner))
            .map(|(k, _)| k)
            .collect();
        let [only] = reaching[..] else {
            continue;
        };
        let Some(case) = js::case_lists_mut(cases, default).nth(only) else {
            continue;
        };
        let mut named = 0;
        js::each_list(case, &mut |list| {
            named += (list.iter())
                .filter(|s| matches!(&s.kind, StmtKind::Break(Some(l)) | StmtKind::Continue(Some(l)) if *l == inner))
                .count();
        });
        let mut rest = Some(rest);
        if named != 1 || !placed(case, &inner, &outer, &mut rest) {
            continue;
        }
        // The block, then its `break outer`, which its end does.
        list.truncate(i + 1);
        let StmtKind::Labeled(_, body) = list.pop().expect("the block").kind else {
            unreachable!("matched")
        };
        list.extend(body);
        if broken {
            list.pop();
        }
    }
}

/// `rest` in place of the `break inner` that ends `case` or a branch of an
/// `if` in it, not in a loop, where a `continue` of no label in `rest`
/// would be the loop's; then the `break outer` that ends the block it
/// leaves where `rest` goes on. Whether it was.
fn placed(case: &mut Vec<Stmt>, inner: &str, outer: &str, rest: &mut Option<Vec<Stmt>>) -> bool {
    if matches!(case.last().map(|s| &s.kind), Some(StmtKind::Break(Some(l))) if l == inner)
        && let Some(rest) = rest.take()
    {
        case.pop();
        let ends = rest.last().is_some_and(js::ends);
        case.extend(rest);
        if !ends {
            case.push(StmtKind::Break(Some(outer.to_string())).at(js::Span::NONE));
        }
        return true;
    }
    case.iter_mut().any(|stmt| match &mut stmt.kind {
        StmtKind::If(_, then, otherwise) => {
            placed(then, inner, outer, rest) || otherwise.as_mut().is_some_and(|b| placed(b, inner, outer, rest))
        }
        _ => false,
    })
}

/// Of `stmts`, a block `l: { ..; switch (..) { .. case B: break l; .. } ..
/// }` whose end nothing falls to, its statements each leaving, followed by
/// `rest` that only that case's `break l` reaches, its last statement: `rest`
/// that case's, as a case's body is written after a `// Fallthrough` to it,
/// and the block, which nothing names then, gone. MIR placed the body of the
/// arm a guarded arm falls to past the `match` (ADR 0905, amended). `rest`
/// leaves, or, at the body's end, `at_end`, it's given the `return;` the
/// body's end is.
pub(super) fn rest_one_case_reaches(stmts: &mut Vec<Stmt>, at_end: bool) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        let rest_ends = stmts.last().is_some_and(js::ends);
        let StmtKind::Labeled(label, body) = &mut stmts[i].kind else {
            i += 1;
            continue;
        };
        let label = label.clone();
        if !(rest_ends || at_end) || !body.last().is_some_and(js::ends) || js::breaks(&stmts[i + 1..], None) {
            i += 1;
            continue;
        }
        let StmtKind::Labeled(_, body) = &mut stmts[i].kind else {
            unreachable!("matched")
        };
        // The one case whose last statement is `break l`, of a `switch` the
        // block holds, which nothing else of it names.
        let mut named = 0;
        js::each_list(body, &mut |list| {
            named += (list.iter())
                .filter(|s| matches!(&s.kind, StmtKind::Break(Some(l)) | StmtKind::Continue(Some(l)) if *l == label))
                .count();
        });
        let mut found = None;
        js::statement_lists(body, &mut |list| {
            for stmt in list.iter_mut() {
                if let StmtKind::Switch { cases, default, .. } = &mut stmt.kind {
                    for case in js::case_lists_mut(cases, default) {
                        if matches!(case.last().map(|s| &s.kind), Some(StmtKind::Break(Some(l))) if *l == label)
                            && !js::names_jump(&case[..case.len() - 1], &label)
                        {
                            found = Some(found.map_or(1, |n: usize| n + 1));
                        }
                    }
                }
            }
        });
        if found != Some(1) || named != 1 {
            i += 1;
            continue;
        }
        let rest = stmts.split_off(i + 1);
        let StmtKind::Labeled(_, mut body) = stmts.pop().expect("the block").kind else {
            unreachable!("matched")
        };
        let mut rest = Some(rest);
        js::statement_lists(&mut body, &mut |list| {
            for stmt in list.iter_mut() {
                if let StmtKind::Switch { cases, default, .. } = &mut stmt.kind {
                    for case in js::case_lists_mut(cases, default) {
                        if matches!(case.last().map(|s| &s.kind), Some(StmtKind::Break(Some(l))) if *l == label)
                            && let Some(rest) = rest.take()
                        {
                            case.truncate(case.len() - 1);
                            let ends = rest.last().is_some_and(js::ends);
                            case.extend(rest);
                            if !ends {
                                case.push(StmtKind::Return(None).at(js::Span::NONE));
                            }
                        }
                    }
                }
            }
        });
        stmts.extend(body);
        return;
    }
}

/// `switch (t) { case A: ..; break; .. } return;`, the `return` MIR joined
/// each arm's into after the `switch`: each case of an arm the program wrote
/// a `return` in (`returns`, `Written::switches`) ending with that `return;`,
/// not a `break;`, as React's popToNextHostParent's `case HostRoot:
/// rootOrSingletonContext = true; return;`, and the `return;` after the
/// `switch` gone where nothing reaches it then (ADR 0905, amended). The
/// same jump either way: the `switch`'s `break` goes on to that `return;`.
pub(super) fn returns_of_cases(stmts: &mut Vec<Stmt>, switches: &[Switch], returns: &[js::Span]) {
    let within = |outer: js::Span, inner: js::Span| {
        !outer.is_none() && !inner.is_none() && outer.lo <= inner.lo && inner.hi <= outer.hi
    };
    let mut i = 0;
    while i + 1 < stmts.len() {
        if !matches!(stmts[i + 1].kind, StmtKind::Return(None)) {
            i += 1;
            continue;
        }
        let returned = stmts[i + 1].clone();
        let StmtKind::Switch { cases, .. } = &mut stmts[i].kind else {
            i += 1;
            continue;
        };
        let mut changed = false;
        for case in cases.iter_mut() {
            // The arm a case is of, where its first constant is written.
            let arm = (case.values.first())
                .and_then(|value| (switches.iter().flat_map(|s| &s.arms)).find(|arm| within(arm.pattern, value.span)));
            if !arm.is_some_and(|arm| returns.iter().any(|r| within(arm.span, *r))) {
                continue;
            }
            // Its end, a `break;` the printer writes where it neither leaves
            // nor falls through, or one tidy wrote.
            if case
                .body
                .last()
                .is_some_and(|last| matches!(last.kind, StmtKind::Break(None)))
            {
                case.body.pop();
            } else if case.falls || case.body.last().is_some_and(js::ends) {
                continue;
            }
            case.body.push(returned.clone());
            changed = true;
        }
        if changed && js::ends(&stmts[i]) {
            stmts.remove(i + 1);
        }
        i += 1;
    }
}

/// A loop the program labeled, `'find_parent: while let ..`, whose turn ends
/// in a `switch` whose cases leave the loop, MIR's `switch (..) { case A:
/// ..; break; .. default: rest; continue; } break;`: each case's way past
/// the `switch` the loop's `break` of the program's label, and the
/// `default`'s `rest` after the `switch`, `findParent: while (..) { switch
/// (..) { case A: ..; break findParent; } rest }`, as React's
/// commitDeletionEffects writes it (ADR 0702, amended). Not a loop the
/// program didn't label, whose label upstream has none of. And of a block
/// MIR made of the turn, `b: { switch (..) { .. break b; } rest; continue;
/// } break;`, each `break b` the loop's, the block its statements.
pub(super) fn loops_left_by_cases(stmts: &mut [Stmt], loops: &[(js::Span, bool, Option<String>)]) {
    for stmt in stmts.iter_mut() {
        let span = stmt.span;
        let (StmtKind::While { label, body, .. }
        | StmtKind::ForOf { label, body, .. }
        | StmtKind::For { label, body, .. }) = &mut stmt.kind
        else {
            continue;
        };
        let Some((.., Some(written))) = super::written_loop(loops, span) else {
            continue;
        };
        let written = written.clone();
        let own = |l: &Option<String>| l.is_none() || *l == *label;
        // `b: { ..; continue; } break;`, a block MIR made of the turn whose
        // `break`s leave the loop, as its end does: its statements, each
        // `break b` the loop's `break` of the program's label.
        if let [.., block, last] = &body[..]
            && let StmtKind::Labeled(b, inner) = &block.kind
            && crate::lower::mir::is_block_label(b)
            && matches!(&last.kind, StmtKind::Break(l) if own(l))
            && matches!(inner.last().map(|s| &s.kind), Some(StmtKind::Continue(l)) if own(l))
            && !super::declared(inner).any(|name| super::declared(&body[..body.len() - 2]).any(|n| n == name))
        {
            let b = b.clone();
            body.pop();
            let StmtKind::Labeled(_, mut inner) = body.pop().expect("the block").kind else {
                unreachable!("matched")
            };
            inner.pop();
            relabel(&mut inner, &b, &written);
            body.extend(inner);
            *label = Some(written);
            continue;
        }
        let [.., switch, last] = &body[..] else {
            continue;
        };
        let StmtKind::Switch {
            cases,
            default: Some(default),
            ..
        } = &switch.kind
        else {
            continue;
        };
        let continued = matches!(default.last().map(|s| &s.kind), Some(StmtKind::Continue(l)) if own(l));
        let rest = &default[..default.len().saturating_sub(1)];
        if !matches!(&last.kind, StmtKind::Break(l) if own(l))
            || !continued
            || cases.is_empty()
            || js::continues(rest, label.as_deref())
            || super::declared(rest).any(|name| super::declared(&body[..body.len() - 2]).any(|n| n == name))
        {
            continue;
        }
        body.pop();
        let at = body.len() - 1;
        let StmtKind::Switch { cases, default, .. } = &mut body[at].kind else {
            unreachable!("matched")
        };
        let mut rest = default.take().expect("matched");
        rest.pop();
        for case in cases.iter_mut() {
            leaving(&mut case.body, &written);
            if !case.falls && !case.body.last().is_some_and(js::ends) {
                case.body
                    .push(StmtKind::Break(Some(written.clone())).at(js::Span::NONE));
            }
        }
        body.extend(rest);
        *label = Some(written);
    }
}

/// Each `break` of no label in a case's `stmts`, not in a loop or a
/// `switch` of their own, which leaves the `switch`, then the loop: the
/// loop's, `written`'s.
fn leaving(stmts: &mut [Stmt], written: &str) {
    for stmt in stmts {
        match &mut stmt.kind {
            StmtKind::Break(l @ None) => *l = Some(written.to_string()),
            StmtKind::If(_, a, b) => {
                leaving(a, written);
                if let Some(b) = b {
                    leaving(b, written);
                }
            }
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                leaving(a, written);
                leaving(b, written);
            }
            _ => {}
        }
    }
}

/// Each `break from` in `stmts`, however deep, `break to`.
fn relabel(stmts: &mut Vec<Stmt>, from: &str, to: &str) {
    js::statement_lists(stmts, &mut |list| {
        for stmt in list.iter_mut() {
            if matches!(&stmt.kind, StmtKind::Break(Some(l)) if l == from) {
                stmt.kind = StmtKind::Break(Some(to.to_string()));
            }
        }
    });
}
