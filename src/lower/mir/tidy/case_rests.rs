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
pub(super) fn blocks_one_case_reaches(stmts: &mut [Stmt]) {
    for stmt in stmts.iter_mut() {
        let StmtKind::Labeled(outer, list) = &mut stmt.kind else {
            continue;
        };
        let outer = outer.clone();
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
        // statement.
        let broken = matches!(body.last().map(|s| &s.kind), Some(StmtKind::Break(Some(l))) if *l == outer);
        let end = body.len() - usize::from(broken);
        let Some((switch, before)) = body[..end].split_last_mut() else {
            continue;
        };
        if !(broken || js::ends(switch)) || js::names_jump(before, &inner) {
            continue;
        }
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
        let (
            Some(Stmt {
                kind: StmtKind::Break(Some(to)),
                ..
            }),
            Some(rest_of_case),
        ) = (case.last(), case.split_last().map(|(_, r)| r))
        else {
            continue;
        };
        if *to != inner || js::names_jump(rest_of_case, &inner) {
            continue;
        }
        case.pop();
        let ends = rest.last().is_some_and(js::ends);
        case.extend(rest);
        if !ends {
            case.push(StmtKind::Break(Some(outer.clone())).at(js::Span::NONE));
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
