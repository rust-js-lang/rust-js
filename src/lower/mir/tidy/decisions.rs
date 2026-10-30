//! A match MIR tests as a tree, an arm more than one of its leaves reaches,
//! `'0'..='9' | '.'`, through a labeled block each leaf breaks out of: an
//! `if` chain of the arms (ADR 0364). The tree's tests read what nothing
//! changes, so an arm's test is what holds on the paths to it; the paths
//! are apart, so the arms are too, in any order, and the one whose test is
//! longest is the chain's `else`. A guard that calls, `[n, ..] if
//! below(*n)`, is a test too, and the chain of a tree with one does what
//! `calls::Model` says the tree does, each way (#221).

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::js::{self, Expr, ExprKind, Op, Stmt, StmtKind, UnaryOp};
use crate::lower::mir::hints::{Constant, Guard, Switch};

use super::{ends, first_written, made_where, names_break};

mod calls;
mod defaults;

use calls::Model;
pub(super) use defaults::default_values;

/// More leaves than a match a person writes has: a tree of tests that
/// aren't one.
const LEAVES: usize = 64;

/// Statements an arm may share with another, copied into each: the jumps
/// that end them, `return w;`, as a person writes each arm's.
const SHARED: usize = 3;

/// What `decisions` is told of the body: each guard the program wrote
/// (`Written::guards`), those of a `match` that's a `switch` (ADR 0629),
/// whose tree `switches` makes, and the variables only a guard reads,
/// each given just before it (`bound`).
pub(super) struct Told<'h> {
    pub guards: &'h [Guard],
    pub switches: &'h [Switch],
    pub bound: &'h HashSet<String>,
    /// Each loop's label (`loop_labels`), a `break` of which leaves it.
    pub loops: &'h HashSet<String>,
}

/// The label of each loop in `body`.
pub(super) fn loop_labels(body: &[Stmt]) -> HashSet<String> {
    let mut labels = HashSet::new();
    js::each_list(body, &mut |list| {
        for stmt in list {
            if let StmtKind::While { label: Some(l), .. }
            | StmtKind::DoWhile { label: Some(l), .. }
            | StmtKind::ForOf { label: Some(l), .. }
            | StmtKind::For { label: Some(l), .. } = &stmt.kind
            {
                labels.insert(l.clone());
            }
        }
    });
    labels
}

/// The variables of `body` each assigned what reads nothing that changes
/// just before a test that calls reads it, and named nowhere else: MIR's
/// binding of a pattern's value a guard reads, `x = b; if (odd(x))`,
/// `(1, x) | (x, 1) if odd(x)`'s (#221).
pub(super) fn bound(body: &[Stmt]) -> HashSet<String> {
    let mut named: HashMap<&str, usize> = HashMap::new();
    js::visit_stmts(body, &mut |name| *named.entry(name).or_default() += 1);
    let mut given: HashMap<String, usize> = HashMap::new();
    js::each_list(body, &mut |list| {
        for pair in list.windows(2) {
            if let StmtKind::Assign(target, value) = &pair[0].kind
                && let ExprKind::Var(name) = &target.kind
                && let StmtKind::If(test, then, otherwise) = &pair[1].kind
                && binds(name, value, test, then, otherwise.as_deref())
            {
                *given.entry(name.clone()).or_default() +=
                    1 + js::mentions_in(&[StmtKind::Expr(test.clone()).at(pair[1].span)], name);
            }
        }
    });
    (given.into_iter())
        .filter(|(name, n)| named.get(name.as_str()) == Some(n))
        .map(|(name, _)| name)
        .collect()
}

/// Whether `name`, given `value` just before `if (test) then else
/// otherwise`, is read there only by its test, one that calls, and what
/// it's given reads nothing that changes: that test, of `value` in its
/// place, is the same.
fn binds(name: &str, value: &Expr, test: &Expr, then: &[Stmt], otherwise: Option<&[Stmt]>) -> bool {
    pure(value)
        && !value.mentions_var(name)
        && !pure(test)
        && test.mentions_var(name)
        && js::mentions_in(then, name) == 0
        && otherwise.is_none_or(|o| js::mentions_in(o, name) == 0)
}

/// Each block MIR made in `stmts` that a match's tree breaks out of,
/// `bb9: { .. }`: the `if` chain of its arms, and what follows the block,
/// where one arm alone goes on to it, that arm's. Of a body `loops` has
/// made what it can of MIR's loops in, whose labeled `break`s it reads.
pub(super) fn decisions(stmts: &mut Vec<Stmt>, told: &Told) {
    let mut i = stmts.len();
    while i > 0 {
        i -= 1;
        let StmtKind::Labeled(label, body) = &stmts[i].kind else {
            continue;
        };
        if !crate::lower::mir::is_block_label(label) {
            continue;
        }
        let Some((chain, takes_rest)) = chain((label, stmts[i].span), body, &stmts[i + 1..], told) else {
            continue;
        };
        if takes_rest {
            stmts.truncate(i);
            stmts.extend(chain);
        } else {
            stmts.splice(i..i + 1, chain);
        }
    }
}

/// A leaf of the tree: the tests that hold on the way to it, in the order
/// they're made, what it does then, and whether that goes on past the
/// block.
struct Leaf<'s> {
    tests: Vec<Expr>,
    body: Vec<&'s Stmt>,
    out: bool,
    /// Whether it ends with a loop whose own `break`s leave the block.
    leaves_loop: bool,
}

/// The arms of the tree in `body`, the block `label` where it is, `after`
/// it in its list: the statements in its place, and whether they take the
/// place of `after` too.
fn chain((label, block): (&str, js::Span), body: &[Stmt], after: &[Stmt], told: &Told) -> Option<(Vec<Stmt>, bool)> {
    // A block a `return` of what it makes follows, `?`'s: its arms each
    // return, as `returned_in_branches` writes them.
    if let Some(StmtKind::Return(Some(value))) = after.first().map(|s| &s.kind)
        && matches!(value.kind, ExprKind::Var(_))
    {
        return None;
    }
    // The block's own variables, declared first: declared before the chain;
    // the body's names are its own while it's tidied.
    let declared = body
        .iter()
        .take_while(|s| matches!(s.kind, StmtKind::Let(_, None)))
        .count();
    let (decls, tree) = body.split_at(declared);
    let mut leaves = Vec::new();
    let mut tested = Vec::new();
    let mut calls = Vec::new();
    let mut tree_walk = Walk {
        label,
        told,
        leaves: &mut leaves,
        tested: &mut tested,
        calls: &mut calls,
    };
    tree_walk.walk(vec![tree], Vec::new())?;
    // A path no value takes, `c < "0" && c >= "A"`, goes.
    let valued = |e: &Expr| {
        Value::of(e).is_some()
            || (told.switches.iter()).any(|switch| {
                switch.valued
                    && (switch.arms.iter())
                        .flat_map(|arm| &arm.constants)
                        .any(|c| matches!(c, Constant::Value(at) if !e.span.is_none() && *at == e.span))
            })
    };
    // Only of a tree no `switch` takes, a `break` of the loop around it in
    // an arm or what follows, which a case would take for the `switch`'s
    // (#412): of the others, the `switch` of the value reads as written.
    let mut breaking = false;
    let arms_and_after = (leaves.iter()).flat_map(|leaf| leaf.body.iter().copied());
    for s in arms_and_after.chain(after) {
        js::each_list(std::slice::from_ref(s), &mut |list| {
            breaking |= (list.iter()).any(|s| match &s.kind {
                StmtKind::Break(l) => l.as_ref().is_none_or(|l| told.loops.contains(l)),
                _ => false,
            });
        });
    }
    let valued = |e: &Expr| breaking && valued(e);
    let leaves: Vec<(Vec<Expr>, Leaf)> = (leaves.into_iter())
        .filter_map(|leaf| Some((present_said(simplified(&leaf.tests)?, &valued), leaf)))
        .collect();
    // Leaves that do one thing, one arm.
    let mut arms: Vec<Arm> = Vec::new();
    for (tests, leaf) in leaves {
        let alike = |arm: &&mut Arm| {
            arm.out == leaf.out && arm.leaves_loop == leaf.leaves_loop && same_body(&arm.body, &leaf.body)
        };
        match arms.iter_mut().find(alike) {
            Some(arm) => {
                arm.tests.push(tests);
                arm.made.push(leaf.tests);
            }
            None => arms.push(Arm {
                tests: vec![tests],
                made: vec![leaf.tests],
                body: leaf.body,
                out: leaf.out,
                leaves_loop: leaf.leaves_loop,
            }),
        }
    }
    // What more than one arm does, copied into each: a jump, or none.
    let shared: Vec<&Stmt> = (arms.iter().enumerate())
        .flat_map(|(i, arm)| arm.body.iter().map(move |s| (i, *s)))
        .filter(|(i, s)| {
            arms[..*i]
                .iter()
                .any(|other| other.body.iter().any(|o| std::ptr::eq(*o, *s)))
        })
        .map(|(_, s)| s)
        .collect();
    if shared.len() > SHARED || !shared.iter().all(|s| ends(s) && size(s) == 1) {
        return None;
    }
    // Nor what one arm holds whole and another in part, an `if` the
    // program wrote after the `match`, `if (x > 5) { .. }`, one arm's
    // tail and the tests of the others' leaves: that arm's copy of it
    // (kj-switch).
    let held: Vec<Vec<*const Stmt>> = (arms.iter())
        .map(|arm| {
            let mut held = Vec::new();
            for s in &arm.body {
                js::each_list(std::slice::from_ref(*s), &mut |list| {
                    held.extend(list.iter().map(|s| s as *const Stmt))
                });
            }
            held
        })
        .collect();
    let copied = (0..arms.len()).any(|i| {
        (held[i].iter()).any(|s| {
            (0..arms.len()).any(|j| j != i && held[j].contains(s)) && !shared.iter().any(|t| std::ptr::eq(*t, *s))
        })
    });
    if copied {
        return None;
    }
    let mut bodies: Vec<Vec<Stmt>> = arms
        .iter()
        .map(|arm| arm.body.iter().map(|s| (*s).clone()).collect())
        .collect();
    for (arm, body) in arms.iter().zip(&mut bodies) {
        if arm.leaves_loop
            && let Some(StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. }) =
                body.last_mut().map(|s| &mut s.kind)
        {
            unlabel_breaks(body, label);
        }
    }
    // One arm alone going on past the block, the others ending: what follows
    // the block is that arm's.
    let going_on: Vec<usize> = (0..arms.len()).filter(|&i| arms[i].out).collect();
    let takes_rest = going_on.len() == 1 && !after.is_empty();
    if let [only] = going_on.as_slice()
        && takes_rest
    {
        bodies[*only].extend(after.iter().cloned());
    }
    // The `else`: an arm that does nothing, or the one whose test is longest;
    // and that one where the others' tests, with the arm that does nothing's
    // the `else`, are longer than the tree's, of an arm that does nothing
    // going on to what follows the block, `"C" => {}` between guarded arms
    // (not one a turn of a loop ends, which a `switch` of `continue`s may
    // be, ADR 0636): that arm's test negated, what follows it in the chain
    // inside, `if (name !== "C") { if (name === "D" && keep) { .. } else {
    // .. } }`.
    let empty: Vec<usize> = (0..arms.len()).filter(|&i| bodies[i].is_empty()).collect();
    let longest = (0..arms.len()).max_by_key(|&i| arms[i].tests.iter().flatten().map(atoms).sum::<usize>())?;
    let arranged = |otherwise: usize, mut arms: Vec<Arm>, mut bodies: Vec<Vec<Stmt>>| -> Option<Vec<Stmt>> {
        // The others in the order they're written: where the test that decides
        // each path to them, its last, or what they do first is.
        let written = |i: usize| {
            let tests = arms[i].tests.iter().filter_map(|tests| written_at(tests.last()?));
            tests.chain(first_written(&bodies[i])).min().unwrap_or(u32::MAX)
        };
        let mut order: Vec<usize> = (0..arms.len()).filter(|&i| i != otherwise).collect();
        order.sort_by_key(|&i| (written(i), i));
        // What an arm before it in the chain says of each path to it, where
        // it failed: a path through that arm's own tests up to its last, then
        // that last failing, `n >= 1 && !(a || b) && n === 2`, needs no more of
        // that last than that the chain got past that arm, `n === 2` once its
        // run of tests of `n` reads as one (#221). Of the tests as the tree
        // makes them, each path's then `simplified`.
        for k in 0..order.len() {
            let (before, at) = order.split_at(k);
            let i = at[0];
            let mut made = arms[i].made.clone();
            let mut changed = false;
            for path in &mut made {
                // Each earlier path's own failing, as long as one says more:
                // `(a, b)`'s paths `[n >= 1, x !== 1, y === 1]` and `[n >= 1, x
                // === 1]` leave `x !== 1` and `y !== 1` to say nothing more.
                let mut more = true;
                while more {
                    more = false;
                    for earlier in before.iter().flat_map(|&j| &arms[j].made) {
                        let Some((last, prefix)) = earlier.split_last() else {
                            continue;
                        };
                        let failed = last.clone().negated_test();
                        if !prefix.is_empty()
                            && path.len() > prefix.len() + 1
                            && (path[..prefix.len()].iter().zip(prefix)).all(|(a, b)| a.written_alike(b))
                            && path[prefix.len()].written_alike(&failed)
                        {
                            path.remove(prefix.len());
                            (changed, more) = (true, true);
                        }
                    }
                }
            }
            if changed
                && let Some(tests) = (made.iter())
                    .map(|path| Some(present_said(simplified(path)?, &valued)))
                    .collect::<Option<Vec<_>>>()
            {
                arms[i].tests = tests;
            }
        }
        // A tree whose guard calls: what it does each way its tests come out,
        // which the chain must do too, as `Model` tells (#221).
        let model = match calls.is_empty() {
            true => None,
            false => Some(Model::of(&arms, &calls, told.guards)?),
        };
        let mut steps: Vec<(usize, Expr)> = Vec::new();
        for &i in &order {
            // Its patterns in the order they're written, `'0'..='9' | '.'`; one
            // whose test is where the whole `|` is, after those it holds.
            // Of a tree that calls, each path as the tree tests it, which
            // `Model::fewest` reads as few tests as tell.
            let mut paths = match &model {
                Some(_) => arms[i].made.iter().filter_map(|path| simplified(path)).collect(),
                None => arms[i].tests.clone(),
            };
            let spans: Vec<js::Span> = paths.iter().filter_map(|t| t.last()).map(|t| t.span).collect();
            paths.sort_by_key(|tests| {
                let span = tests.last().map(|t| t.span).unwrap_or_default();
                let holds = spans.iter().any(|s| *s != span && span.lo <= s.lo && s.hi <= span.hi);
                (holds, tests.last().and_then(written_at).unwrap_or(u32::MAX))
            });
            let test = match &model {
                Some(model) => model.fewest(i, &steps, paths)?,
                None => factored(paths),
            };
            steps.push((i, test));
        }
        if let Some(model) = &model
            && !model.kept(&steps, otherwise)
        {
            return None;
        }
        let mut chain: Option<Vec<Stmt>> = match std::mem::take(&mut bodies[otherwise]) {
            body if body.is_empty() => None,
            body => Some(body),
        };
        // Of a tree that calls, each test read alike once: `Model::fewest`
        // reads no more than tells, and a test it makes again, `n >= 1` of
        // `[1, ..]`'s arm and `[n, ..] if f(*n)`'s, is the pattern's.
        let count = |tests: &mut dyn Iterator<Item = &Expr>| match &model {
            Some(_) => distinct(tests),
            None => tests.map(atoms).sum(),
        };
        let tests_made = count(&mut steps.iter().map(|(_, test)| test));
        for (k, (i, test)) in steps.into_iter().enumerate().rev() {
            let then = std::mem::take(&mut bodies[i]);
            // The chain's first `if` where the block is, of one MIR made where
            // its tree's first `if` is (`made_where`), so what's written before
            // the tree, `// c \n if a || b {`, is before it, not past its `{`;
            // an `else if` where its arm's code is, a comment before the arm
            // that code's (ADR 0444, amended).
            let span = match k {
                0 => made_where(block, tree),
                _ => then.first().map(|s| s.span).unwrap_or_default(),
            };
            chain = Some(vec![StmtKind::If(test, then, chain).at(span)]);
        }
        // Tests no longer than the tree's, or the tree reads better.
        let written = count(&mut tested.iter().map(|(_, test)| test));
        if tests_made > written {
            return None;
        }
        Some(chain.unwrap_or_default())
    };
    let chain = match empty.as_slice() {
        [only] => arranged(*only, arms.clone(), bodies.clone()).or_else(|| {
            let goes_on = *only != longest && !after.is_empty();
            goes_on.then(|| arranged(longest, arms, bodies)).flatten()
        }),
        _ => arranged(longest, arms, bodies),
    }?;
    let mut stmts: Vec<Stmt> = decls.to_vec();
    stmts.extend(chain);
    Some((stmts, takes_rest))
}

/// An arm's test of its `paths`, the tests of each: what each tests first
/// alike, once, before the rest of each, `n >= 1 && (a === 1 || b === 1)`,
/// not `n >= 1 && a === 1 || n >= 1 && b === 1`, as the tests are pure
/// (#221); and each tried as `alternatives` says.
fn factored(mut paths: Vec<Vec<Expr>>) -> Expr {
    let shared = match paths.split_first() {
        Some((first, others)) if !others.is_empty() => (0..first.len())
            .take_while(|&t| {
                (others.iter()).all(|path| path.len() > t + 1 && path[t].written_alike(&first[t]))
                    && first.len() > t + 1
            })
            .count(),
        _ => 0,
    };
    let before: Vec<Expr> = paths.first().map(|path| path[..shared].to_vec()).unwrap_or_default();
    for path in &mut paths {
        path.drain(..shared);
    }
    alternatives(&mut paths);
    match before.is_empty() {
        true => any(paths.into_iter().map(all).collect()),
        false => all(before
            .into_iter()
            .chain([any(paths.into_iter().map(all).collect())])
            .collect()),
    }
}

/// An arm's `paths`, the tests of each, tried one after another: of one
/// whose single test those before it fail, what that failing says left
/// out of each after it, `(1, _) | (_, 1)`'s `a === 1 || b === 1`, not `a
/// === 1 || (a !== 1 && b === 1)`. Tried first, where those before it each
/// first test its value, and one of them says it fails: the value is read
/// first then as before, and each test after only where it was.
fn alternatives(paths: &mut Vec<Vec<Expr>>) {
    for j in 0..paths.len() {
        let [only] = paths[j].as_slice() else { continue };
        let Some((subject, set)) = atom(only) else { continue };
        let fails = set.complement();
        let tests_first = |tests: &[Expr]| {
            tests
                .first()
                .and_then(atom)
                .is_some_and(|(s, _)| same_subject(&s, &subject))
        };
        let says = |tests: &[Expr]| {
            (tests.iter()).any(|t| atom(t).is_some_and(|(s, set)| same_subject(&s, &subject) && fails.within(&set)))
        };
        let mut i = j;
        while i > 0 && tests_first(&paths[i - 1]) {
            i -= 1;
        }
        if paths[i..j].iter().any(|p| says(p)) {
            let path = paths.remove(j);
            paths.insert(i, path);
        }
    }
    // And of a value's `===` no set says, a symbol's, `s === A || (s !== A
    // && s === B)`'s `s !== A`, as it's written (#412).
    let mut failed: Vec<(Expr, Set)> = Vec::new();
    let mut negated: Vec<Expr> = Vec::new();
    for tests in paths {
        let said = |t: &Expr| {
            atom(t).is_some_and(|(s, set)| {
                failed
                    .iter()
                    .any(|(f, fails)| same_subject(f, &s) && fails.within(&set))
            }) || negated.iter().any(|n| n.written_alike(t))
        };
        if tests.iter().any(|t| !said(t)) {
            tests.retain(|t| !said(t));
        }
        if let [only] = tests.as_slice() {
            match atom(only) {
                Some((s, set)) => failed.push((s, set.complement())),
                None if matches!(&only.kind, ExprKind::Binary(Op::Eq, of, _) if is_subject(of)) => {
                    negated.push(only.clone().negated_test())
                }
                None => {}
            }
        }
    }
}

/// Leaves that do one thing, and the tests of each path to them.
#[derive(Clone)]
struct Arm<'s> {
    tests: Vec<Vec<Expr>>,
    /// Each path's tests as the tree makes them, before `simplified`.
    made: Vec<Vec<Expr>>,
    body: Vec<&'s Stmt>,
    out: bool,
    leaves_loop: bool,
}

/// A walk of a tree's tests to its leaves: the block it's in, and what it
/// finds.
struct Walk<'s, 'w> {
    label: &'s str,
    told: &'w Told<'w>,
    leaves: &'w mut Vec<Leaf<'s>>,
    /// Each test of the tree, once: the `if`'s, and what it tests, a
    /// guard's of the value a binding gives in its place.
    tested: &'w mut Vec<(&'s Expr, Expr)>,
    /// Each part of a test that calls, a guard's (`Model`).
    calls: &'w mut Vec<Expr>,
}

impl<'s> Walk<'s, '_> {
    /// The leaves of `frames`, the rest of each list the walk is in,
    /// innermost last, given that `tests` hold: `None` where one isn't a
    /// tree's leaf.
    fn walk(&mut self, mut frames: Vec<&'s [Stmt]>, tests: Vec<Expr>) -> Option<()> {
        if self.leaves.len() > LEAVES {
            return None;
        }
        while frames.last().is_some_and(|f| f.is_empty()) {
            frames.pop();
        }
        let Some(frame) = frames.last_mut() else {
            // Past the block's end, as its `break` goes.
            self.leaves.push(Leaf {
                tests,
                body: Vec::new(),
                out: true,
                leaves_loop: false,
            });
            return Some(());
        };
        let current: &'s [Stmt] = frame;
        let (stmt, rest) = current.split_first()?;
        if let Some(Test {
            test,
            written,
            then,
            otherwise,
            rest,
        }) = self.test_at(stmt, rest)
        {
            *frame = rest;
            if !self.tested.iter().any(|(t, _)| std::ptr::eq(*t, written)) {
                self.tested.push((written, test.clone()));
                let mut parts = Vec::new();
                leaf_tests(&test, js::Span::NONE, &mut parts);
                self.calls
                    .extend(parts.into_iter().filter(|(t, _)| !pure(t)).map(|(t, _)| t.clone()));
            }
            let mut yes = frames.clone();
            yes.push(then);
            self.walk(yes, [tests.clone(), vec![test.clone()]].concat())?;
            let mut no = frames;
            if let Some(otherwise) = otherwise {
                no.push(otherwise);
            }
            return self.walk(no, [tests, vec![test.negated_test()]].concat());
        }
        let label = self.label;
        match &stmt.kind {
            StmtKind::Break(Some(l)) if l == label => {
                self.leaves.push(Leaf {
                    tests,
                    body: Vec::new(),
                    out: true,
                    leaves_loop: false,
                });
                Some(())
            }
            _ => {
                // What's done from here on, to what ends it.
                let mut body: Vec<&Stmt> = frames.iter().rev().flat_map(|f| f.iter()).collect();
                if let Some(end) = body.iter().position(|s| ends(s)) {
                    body.truncate(end + 1);
                }
                let out = match body.last().map(|s| &s.kind) {
                    Some(StmtKind::Break(Some(l))) if l == label => {
                        body.pop();
                        true
                    }
                    Some(_) => !ends(body[body.len() - 1]),
                    None => true,
                };
                // A `break` of the block can't be the chain's, but in a loop it
                // ends with, the loop's own, which leaves to where the block does.
                let breaks = |s: &Stmt| names_break(std::slice::from_ref(s), label);
                let (out, leaves_loop) = match body.split_last() {
                    Some((last, before)) if !before.iter().any(|s| breaks(s)) && breaks(last) => {
                        let (StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. }) =
                            &last.kind
                        else {
                            return None;
                        };
                        if !loop_breaks(body, label) {
                            return None;
                        }
                        (true, true)
                    }
                    _ if body.iter().any(|s| breaks(s)) => return None,
                    _ => (out, false),
                };
                self.leaves.push(Leaf {
                    tests,
                    body,
                    out,
                    leaves_loop,
                });
                Some(())
            }
        }
    }

    /// The test of the tree `stmt` makes, `rest` after it in its list: an
    /// `if` of what reads nothing that changes, or of a guard the program
    /// wrote, which calls; and a guard's, `x = b; if (odd(x))`, a value
    /// given to what it alone reads (`binds`), with that value in its
    /// place, `odd(b)`.
    fn test_at(&self, stmt: &'s Stmt, rest: &'s [Stmt]) -> Option<Test<'s>> {
        let guard = |test: &Expr| {
            let mut parts = Vec::new();
            leaf_tests(test, js::Span::NONE, &mut parts);
            let switched = |g: &Guard| {
                (self.told.switches.iter())
                    .flat_map(|s| &s.arms)
                    .any(|arm| arm.guard == Some(g.span))
            };
            (parts.iter()).all(|(part, span)| {
                pure(part)
                    || !span.is_none() && (self.told.guards.iter()).any(|g| within(*span, g.span) && !switched(g))
            })
        };
        if let StmtKind::If(test, then, otherwise) = &stmt.kind {
            return guard(test).then(|| Test {
                test: test.clone(),
                written: test,
                then,
                otherwise: otherwise.as_deref(),
                rest,
            });
        }
        let (name, value, local) = match &stmt.kind {
            StmtKind::Const(name, value) | StmtKind::Let(name, Some(value)) => (name, value, true),
            StmtKind::Assign(target, value) => match &target.kind {
                ExprKind::Var(name) if self.told.bound.contains(name) => (name, value, false),
                _ => return None,
            },
            _ => return None,
        };
        let (next, rest) = rest.split_first()?;
        let StmtKind::If(test, then, otherwise) = &next.kind else {
            return None;
        };
        if !binds(name, value, test, then, otherwise.as_deref())
            || local && js::mentions_in(rest, name) > 0
            || !guard(test)
        {
            return None;
        }
        Some(Test {
            test: test.substitute(&|v| (v == name).then(|| value.clone()))?,
            written: test,
            then,
            otherwise: otherwise.as_deref(),
            rest,
        })
    }
}

/// A test `Walk::test_at` finds: what it tests, the `if`'s own, its
/// branches, and what follows it.
struct Test<'s> {
    test: Expr,
    written: &'s Expr,
    then: &'s [Stmt],
    otherwise: Option<&'s [Stmt]>,
    rest: &'s [Stmt],
}

/// Whether `inner` is where `outer` is.
fn within(inner: js::Span, outer: js::Span) -> bool {
    outer.lo <= inner.lo && inner.hi <= outer.hi
}

/// The parts of `test` that are each one test, `a`, `b` and `c` of `a &&
/// !(b || c)`, each with where it's written, or where what it's part of
/// is, `at` of none.
fn leaf_tests<'e>(test: &'e Expr, at: js::Span, out: &mut Vec<(&'e Expr, js::Span)>) {
    let at = if test.span.is_none() { at } else { test.span };
    match &test.kind {
        ExprKind::Binary(Op::And | Op::Or, a, b) => {
            leaf_tests(a, at, out);
            leaf_tests(b, at, out);
        }
        ExprKind::Unary(UnaryOp::Not, a) => leaf_tests(a, at, out),
        _ => out.push((test, at)),
    }
}

/// Whether each `break` of `label` in a loop's body `stmts` is in no loop
/// of its own: the loop's, unlabeled.
fn loop_breaks(stmts: &[Stmt], label: &str) -> bool {
    stmts.iter().all(|stmt| match &stmt.kind {
        StmtKind::If(_, a, b) => loop_breaks(a, label) && b.as_deref().is_none_or(|b| loop_breaks(b, label)),
        StmtKind::Labeled(_, body) => loop_breaks(body, label),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => loop_breaks(a, label) && loop_breaks(b, label),
        StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
            !names_break(body, label)
        }
        _ => true,
    })
}

/// Each `break` of `label` `loop_breaks` found: `break`.
fn unlabel_breaks(stmts: &mut [Stmt], label: &str) {
    for stmt in stmts {
        match &mut stmt.kind {
            StmtKind::Break(l) if l.as_deref() == Some(label) => *l = None,
            StmtKind::If(_, a, b) => {
                unlabel_breaks(a, label);
                if let Some(b) = b {
                    unlabel_breaks(b, label);
                }
            }
            StmtKind::Labeled(_, body) => unlabel_breaks(body, label),
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                unlabel_breaks(a, label);
                unlabel_breaks(b, label);
            }
            _ => {}
        }
    }
}

/// Whether reading `e` does nothing: variables, their properties, and
/// what's made of them without a call.
fn pure(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Num(_)
        | ExprKind::BigInt(_)
        | ExprKind::BigUint(_)
        | ExprKind::Bool(_)
        | ExprKind::Str(_)
        | ExprKind::Undefined
        | ExprKind::Null
        | ExprKind::Var(_) => true,
        ExprKind::Member(object, _) => pure(object),
        ExprKind::Index(object, index) => pure(object) && pure(index),
        ExprKind::Unary(_, a) => pure(a),
        ExprKind::Binary(_, a, b) => pure(a) && pure(b),
        ExprKind::Cond(a, b, c) => pure(a) && pure(b) && pure(c),
        _ => false,
    }
}

/// How many statements `stmt` is, with those inside it.
fn size(stmt: &Stmt) -> usize {
    let list = |stmts: &[Stmt]| stmts.iter().map(size).sum::<usize>();
    1 + match &stmt.kind {
        StmtKind::If(_, a, b) => list(a) + b.as_deref().map_or(0, list),
        StmtKind::While { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Labeled(_, body) => list(body),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => list(a) + list(b),
        _ => 0,
    }
}

/// Whether `a` and `b` do the same: the same statements, or ones written
/// alike.
fn same_body(a: &[&Stmt], b: &[&Stmt]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| std::ptr::eq(*x, *y) || same_stmt(x, y))
}

/// Whether `a` and `b` do the same: written alike, each part.
pub(super) fn same_stmt(a: &Stmt, b: &Stmt) -> bool {
    match (&a.kind, &b.kind) {
        (StmtKind::Assign(t, v), StmtKind::Assign(u, w)) => t.written_alike(u) && v.written_alike(w),
        (StmtKind::Expr(x), StmtKind::Expr(y)) | (StmtKind::Throw(x), StmtKind::Throw(y)) => x.written_alike(y),
        (StmtKind::Return(x), StmtKind::Return(y)) => match (x, y) {
            (Some(x), Some(y)) => x.written_alike(y),
            (None, None) => true,
            _ => false,
        },
        (StmtKind::Break(x), StmtKind::Break(y)) | (StmtKind::Continue(x), StmtKind::Continue(y)) => x == y,
        _ => false,
    }
}

/// `a && b && ..`, of none `true`.
fn all(tests: Vec<Expr>) -> Expr {
    tests
        .into_iter()
        .reduce(|a, b| Expr::bin(Op::And, a, b))
        .unwrap_or_else(|| Expr::bool(true))
}

/// `a || b || ..`.
fn any(tests: Vec<Expr>) -> Expr {
    tests
        .into_iter()
        .reduce(|a, b| Expr::bin(Op::Or, a, b))
        .unwrap_or_else(|| Expr::bool(false))
}

/// Where the first test `e` is made of is written.
fn written_at(e: &Expr) -> Option<u32> {
    let here = (!e.span.is_none()).then_some(e.span.lo);
    let within = match &e.kind {
        ExprKind::Binary(_, a, b) => written_at(a).into_iter().chain(written_at(b)).min(),
        ExprKind::Unary(_, a) => written_at(a),
        _ => None,
    };
    here.into_iter().chain(within).min()
}

/// How many tests `tests` are made of, each read alike once: a chain's
/// `n >= 1 && a === 1`, `n >= 1 && b(n)`, its `n >= 1` once.
fn distinct(tests: &mut dyn Iterator<Item = &Expr>) -> usize {
    let mut seen: Vec<&Expr> = Vec::new();
    for test in tests {
        let mut parts = Vec::new();
        leaf_tests(test, js::Span::NONE, &mut parts);
        for (part, _) in parts {
            let converse = part.clone().negated_test();
            if !seen.iter().any(|s| s.written_alike(part) || s.written_alike(&converse)) {
                seen.push(part);
            }
        }
    }
    seen.len()
}

/// How many tests `e` is made of.
fn atoms(e: &Expr) -> usize {
    match &e.kind {
        ExprKind::Binary(Op::And | Op::Or, a, b) => atoms(a) + atoms(b),
        ExprKind::Unary(UnaryOp::Not, a) => atoms(a),
        _ => 1,
    }
}

/// `tests`, each read where it's made, without what those before it in a
/// run of tests of one value already say: `None` where no value passes
/// them all.
fn simplified(tests: &[Expr]) -> Option<Vec<Expr>> {
    let mut flat = Vec::new();
    for test in tests {
        conjuncts(test.clone(), &mut flat);
    }
    let mut out = Vec::new();
    // What holds of each value tested, so far.
    let mut known: Vec<(Expr, Set)> = Vec::new();
    let mut i = 0;
    while i < flat.len() {
        let Some((subject, first)) = atom(&flat[i]) else {
            out.push(flat[i].clone());
            i += 1;
            continue;
        };
        // The run of tests of this value, read one after another.
        let mut run = first.clone();
        let mut sets = vec![first];
        let mut j = i + 1;
        while let Some((next, set)) = flat.get(j).and_then(atom)
            && same_subject(&next, &subject)
            && set.kind == run.kind
        {
            run = run.intersect(&set);
            sets.push(set);
            j += 1;
        }
        let at = known
            .iter()
            .position(|(s, set)| same_subject(s, &subject) && set.kind == run.kind);
        let before = at.map_or_else(|| Set::all(run.kind), |at| known[at].1.clone());
        let now = before.intersect(&run);
        if now.is_empty() {
            return None;
        }
        // As one of the run's own tests says it, or as it reads best.
        match (i..j).find(|&k| before.intersect(&sets[k - i]) == now) {
            Some(k) => out.push(flat[k].clone()),
            None => out.push(Expr {
                span: flat[j - 1].span,
                ..render(&subject, &now)
            }),
        }
        match at {
            Some(at) => known[at].1 = now,
            None => known.push((subject, now)),
        }
        i = j;
    }
    // A test of a value a later one of it says more than, `a !== 1` before
    // `a === 2`: that one, in its place, so each test is still read only
    // where it was, and the path's tests read `a === 2 && b === 2`.
    let mut i = 0;
    while i < out.len() {
        let stronger = atom(&out[i]).and_then(|(subject, set)| {
            (i + 1..out.len())
                .find(|&j| atom(&out[j]).is_some_and(|(s, later)| same_subject(&s, &subject) && later.within(&set)))
        });
        match stronger {
            Some(j) => out[i] = out.remove(j),
            None => i += 1,
        }
    }
    Some(out)
}

/// Of a path's `tests`, each `x != null` where another is `x === k` of a
/// `k` never `null` or `undefined` (`valued`), which says it: gone,
/// `symbolFor === C` alone, as a `switch` takes `None` in no case (#412).
fn present_said(mut tests: Vec<Expr>, valued: &dyn Fn(&Expr) -> bool) -> Vec<Expr> {
    fn says(subject: &Expr, e: &Expr, valued: &dyn Fn(&Expr) -> bool) -> bool {
        match &e.kind {
            ExprKind::Binary(Op::Eq, a, b) => {
                (same_subject(a, subject) && valued(b)) || (same_subject(b, subject) && valued(a))
            }
            ExprKind::Binary(Op::Or, a, b) => says(subject, a, valued) && says(subject, b, valued),
            _ => false,
        }
    }
    let said: Vec<bool> = (tests.iter())
        .map(|t| match &t.kind {
            ExprKind::Binary(Op::LooseNe, subject, none)
                if matches!(none.kind, ExprKind::Null | ExprKind::Undefined) =>
            {
                tests.iter().any(|other| says(subject, other, valued))
            }
            _ => false,
        })
        .collect();
    let mut said = said.into_iter();
    tests.retain(|_| !said.next().unwrap_or(false));
    tests
}

/// `test`'s parts that must all hold, `a && b`'s, into `out`.
fn conjuncts(test: Expr, out: &mut Vec<Expr>) {
    match test.kind {
        ExprKind::Binary(Op::And, a, b) => {
            conjuncts(*a, out);
            conjuncts(*b, out);
        }
        kind => out.push(Expr { kind, ..test }),
    }
}

/// The value `e` tests against constants, and the values that pass: of
/// `c >= "a" && c <= "z"`, `c` and `"a"..="z"`.
fn atom(e: &Expr) -> Option<(Expr, Set)> {
    match &e.kind {
        ExprKind::Unary(UnaryOp::Not, a) => atom(a).map(|(s, set)| (s, set.complement())),
        ExprKind::Binary(op @ (Op::And | Op::Or), a, b) => {
            let ((s, x), (t, y)) = (atom(a)?, atom(b)?);
            (same_subject(&s, &t) && x.kind == y.kind).then(|| {
                let set = if *op == Op::And { x.intersect(&y) } else { x.union(&y) };
                (s, set)
            })
        }
        ExprKind::Binary(op @ (Op::Eq | Op::Ne | Op::Lt | Op::Le | Op::Gt | Op::Ge), a, b) => {
            let (subject, value, op) = match (Value::of(a), Value::of(b)) {
                (None, Some(value)) => (&**a, value, *op),
                (Some(value), None) => (&**b, value, flipped(*op)),
                _ => return None,
            };
            if !is_subject(subject) {
                return None;
            }
            let kind = value.kind();
            let point = Interval {
                lo: Some((value.clone(), true)),
                hi: Some((value.clone(), true)),
            };
            let pieces = match op {
                Op::Eq | Op::Ne => vec![point],
                Op::Lt | Op::Le => vec![Interval {
                    lo: None,
                    hi: Some((value, op == Op::Le)),
                }],
                _ => vec![Interval {
                    lo: Some((value, op == Op::Ge)),
                    hi: None,
                }],
            };
            let set = Set {
                kind,
                pieces,
                nan: false,
            };
            Some((subject.clone(), if op == Op::Ne { set.complement() } else { set }))
        }
        _ => None,
    }
}

/// `a < b` as `b > a`.
fn flipped(op: Op) -> Op {
    match op {
        Op::Lt => Op::Gt,
        Op::Le => Op::Ge,
        Op::Gt => Op::Lt,
        Op::Ge => Op::Le,
        op => op,
    }
}

/// A value a test reads the same each time: a variable, a property or an
/// item of one, or `typeof` one.
fn is_subject(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Var(_) => true,
        ExprKind::Member(object, _) => is_subject(object),
        ExprKind::Index(object, index) => is_subject(object) && Value::of(index).is_some(),
        ExprKind::Unary(UnaryOp::Typeof, of) => is_subject(of),
        _ => false,
    }
}

fn same_subject(a: &Expr, b: &Expr) -> bool {
    match (&a.kind, &b.kind) {
        (ExprKind::Unary(UnaryOp::Typeof, x), ExprKind::Unary(UnaryOp::Typeof, y)) => same_subject(x, y),
        _ => a.written_alike(b),
    }
}

/// `subject` in `set`, as a person writes it: the values that pass, or
/// those that don't, whichever is shorter and says the same of `NaN`.
fn render(subject: &Expr, set: &Set) -> Expr {
    let other = set.complement();
    // `x !== a && x !== b`: all but some, and `NaN` if a number.
    let points = other.pieces.iter().all(Interval::is_point) && (set.nan || set.kind != Kind::Num);
    if points {
        return all(other
            .pieces
            .iter()
            .map(|p| test(subject, Op::Ne, &p.lo.as_ref().expect("a point").0))
            .collect());
    }
    let passing = any(set.pieces.iter().map(|p| p.render(subject)).collect());
    // A number's comparisons are false of `NaN`; one where it passes is
    // `!` of those of the others.
    if set.nan {
        return any(other.pieces.iter().map(|p| p.render(subject)).collect()).negated_test();
    }
    let failing = || match set.kind {
        Kind::Num => None,
        _ => Some(all(other.pieces.iter().map(|p| p.render_outside(subject)).collect())),
    };
    match failing() {
        Some(failing) if atoms(&failing) < atoms(&passing) => failing,
        _ => passing,
    }
}

fn test(subject: &Expr, op: Op, value: &Value) -> Expr {
    Expr::bin(op, subject.clone(), value.expr.clone())
}

/// What a test compares to: a string, a number, or a `BigInt`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Str,
    Num,
    Big,
}

/// A constant tested against, and how it's written.
#[derive(Clone)]
struct Value {
    expr: Expr,
    of: Lit,
}

#[derive(Clone, PartialEq)]
enum Lit {
    /// As JS orders strings, by UTF-16 code unit.
    Str(Vec<u16>),
    Num(f64),
    Big(i128),
}

impl Value {
    fn of(e: &Expr) -> Option<Value> {
        let of = match &e.kind {
            ExprKind::Str(s) => Lit::Str(s.encode_utf16().collect()),
            ExprKind::Num(n) if !n.is_nan() => Lit::Num(*n),
            ExprKind::BigInt(n) => Lit::Big(*n),
            ExprKind::BigUint(n) => Lit::Big(i128::try_from(*n).ok()?),
            ExprKind::Unary(UnaryOp::Neg, inner) => match Value::of(inner)?.of {
                Lit::Num(n) => Lit::Num(-n),
                Lit::Big(n) => Lit::Big(n.checked_neg()?),
                Lit::Str(_) => return None,
            },
            _ => return None,
        };
        Some(Value { expr: e.clone(), of })
    }

    fn kind(&self) -> Kind {
        match self.of {
            Lit::Str(_) => Kind::Str,
            Lit::Num(_) => Kind::Num,
            Lit::Big(_) => Kind::Big,
        }
    }

    fn cmp(&self, other: &Value) -> Ordering {
        match (&self.of, &other.of) {
            (Lit::Str(a), Lit::Str(b)) => a.cmp(b),
            (Lit::Num(a), Lit::Num(b)) => a.partial_cmp(b).expect("no NaN is a constant here"),
            (Lit::Big(a), Lit::Big(b)) => a.cmp(b),
            _ => unreachable!("a set holds values of one kind"),
        }
    }
}

/// Values of one kind from `lo` to `hi`, each `None` where there's no
/// bound, and whether it's in.
#[derive(Clone)]
struct Interval {
    lo: Option<(Value, bool)>,
    hi: Option<(Value, bool)>,
}

impl Interval {
    fn is_empty(&self) -> bool {
        match (&self.lo, &self.hi) {
            (Some((lo, a)), Some((hi, b))) => match lo.cmp(hi) {
                Ordering::Greater => true,
                Ordering::Equal => !(*a && *b),
                Ordering::Less => false,
            },
            _ => false,
        }
    }

    fn is_point(&self) -> bool {
        matches!((&self.lo, &self.hi), (Some((lo, true)), Some((hi, true))) if lo.cmp(hi) == Ordering::Equal)
    }

    fn intersect(&self, other: &Interval) -> Interval {
        // The higher low bound, the one that leaves its value out of two at one value.
        let lo = match (&self.lo, &other.lo) {
            (None, b) => b.clone(),
            (a, None) => a.clone(),
            (Some(a), Some(b)) => Some(match a.0.cmp(&b.0) {
                Ordering::Greater => a.clone(),
                Ordering::Less => b.clone(),
                Ordering::Equal => (a.0.clone(), a.1 && b.1),
            }),
        };
        let hi = match (&self.hi, &other.hi) {
            (None, b) => b.clone(),
            (a, None) => a.clone(),
            (Some(a), Some(b)) => Some(match a.0.cmp(&b.0) {
                Ordering::Less => a.clone(),
                Ordering::Greater => b.clone(),
                Ordering::Equal => (a.0.clone(), a.1 && b.1),
            }),
        };
        Interval { lo, hi }
    }

    /// `subject` within it.
    fn render(&self, subject: &Expr) -> Expr {
        if self.is_point() {
            return test(subject, Op::Eq, &self.lo.as_ref().expect("a point").0);
        }
        let lo = (self.lo.as_ref()).map(|(v, i)| test(subject, if *i { Op::Ge } else { Op::Gt }, v));
        let hi = (self.hi.as_ref()).map(|(v, i)| test(subject, if *i { Op::Le } else { Op::Lt }, v));
        all(lo.into_iter().chain(hi).collect())
    }

    /// `subject` outside it, of a kind that has no `NaN`.
    fn render_outside(&self, subject: &Expr) -> Expr {
        if self.is_point() {
            return test(subject, Op::Ne, &self.lo.as_ref().expect("a point").0);
        }
        let lo = (self.lo.as_ref()).map(|(v, i)| test(subject, if *i { Op::Lt } else { Op::Le }, v));
        let hi = (self.hi.as_ref()).map(|(v, i)| test(subject, if *i { Op::Gt } else { Op::Ge }, v));
        any(lo.into_iter().chain(hi).collect())
    }
}

/// The values of one kind a test passes: intervals apart and in order, and
/// for numbers whether `NaN` passes.
#[derive(Clone)]
struct Set {
    kind: Kind,
    pieces: Vec<Interval>,
    nan: bool,
}

impl PartialEq for Set {
    fn eq(&self, other: &Set) -> bool {
        let bound = |a: &Option<(Value, bool)>, b: &Option<(Value, bool)>| match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => a.1 == b.1 && a.0.cmp(&b.0) == Ordering::Equal,
            _ => false,
        };
        self.kind == other.kind
            && self.nan == other.nan
            && self.pieces.len() == other.pieces.len()
            && (self.pieces.iter().zip(&other.pieces)).all(|(a, b)| bound(&a.lo, &b.lo) && bound(&a.hi, &b.hi))
    }
}

impl Set {
    fn all(kind: Kind) -> Set {
        Set {
            kind,
            pieces: vec![Interval { lo: None, hi: None }],
            nan: kind == Kind::Num,
        }
    }

    fn is_empty(&self) -> bool {
        self.pieces.is_empty() && !self.nan
    }

    /// Whether each value it passes `other` passes too.
    fn within(&self, other: &Set) -> bool {
        self.kind == other.kind && self.intersect(other) == *self
    }

    fn intersect(&self, other: &Set) -> Set {
        let mut pieces: Vec<Interval> = (self.pieces.iter())
            .flat_map(|a| other.pieces.iter().map(move |b| a.intersect(b)))
            .filter(|p| !p.is_empty())
            .collect();
        pieces.sort_by(|a, b| match (&a.lo, &b.lo) {
            (None, None) => Ordering::Equal,
            (None, _) => Ordering::Less,
            (_, None) => Ordering::Greater,
            (Some(a), Some(b)) => a.0.cmp(&b.0).then(b.1.cmp(&a.1)),
        });
        Set {
            kind: self.kind,
            pieces,
            nan: self.nan && other.nan,
        }
        .merged()
    }

    /// The values it leaves out.
    fn complement(&self) -> Set {
        let mut pieces = Vec::new();
        // Where the gap after the last piece starts, `None` of none before.
        let mut from: Option<(Value, bool)> = None;
        let mut to_the_end = true;
        for piece in &self.pieces {
            if let Some((v, i)) = &piece.lo {
                let gap = Interval {
                    lo: from.clone(),
                    hi: Some((v.clone(), !i)),
                };
                if !gap.is_empty() {
                    pieces.push(gap);
                }
            }
            match &piece.hi {
                Some((v, i)) => from = Some((v.clone(), !i)),
                None => {
                    to_the_end = false;
                    break;
                }
            }
        }
        if to_the_end {
            pieces.push(Interval { lo: from, hi: None });
        }
        Set {
            kind: self.kind,
            pieces,
            nan: self.kind == Kind::Num && !self.nan,
        }
    }

    fn union(&self, other: &Set) -> Set {
        self.complement().intersect(&other.complement()).complement()
    }

    /// Pieces that overlap or meet, one.
    fn merged(mut self) -> Set {
        let mut pieces: Vec<Interval> = Vec::new();
        for piece in self.pieces {
            if let Some(last) = pieces.last_mut()
                && meets(&last.hi, &piece.lo)
            {
                let further = match (&last.hi, &piece.hi) {
                    (None, _) => false,
                    (_, None) => true,
                    (Some(a), Some(b)) => match b.0.cmp(&a.0) {
                        Ordering::Greater => true,
                        Ordering::Equal => b.1 && !a.1,
                        Ordering::Less => false,
                    },
                };
                if further {
                    last.hi = piece.hi;
                }
                continue;
            }
            pieces.push(piece);
        }
        self.pieces = pieces;
        self
    }
}

/// Whether a piece that ends at `hi` and one that starts at `lo` leave no
/// value between them.
fn meets(hi: &Option<(Value, bool)>, lo: &Option<(Value, bool)>) -> bool {
    match (hi, lo) {
        (None, _) | (_, None) => true,
        (Some((h, hi_in)), Some((l, lo_in))) => match l.cmp(h) {
            Ordering::Less => true,
            Ordering::Equal => *hi_in || *lo_in,
            Ordering::Greater => false,
        },
    }
}
