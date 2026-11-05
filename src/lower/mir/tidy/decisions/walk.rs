//! The walk of a match's tree to its leaves (`decisions`, ADR 0364): the
//! tests that hold on each way, what each leaf does, and the variables the
//! tree declares between its tests (#771).

use super::*;

/// A leaf of the tree: the tests that hold on the way to it, in the order
/// they're made, what it does then, and whether that goes on past the
/// block.
pub(super) struct Leaf<'s> {
    pub(super) tests: Vec<Expr>,
    pub(super) body: Vec<&'s Stmt>,
    pub(super) out: bool,
    /// Whether it ends with a loop whose own `break`s leave the block.
    pub(super) leaves_loop: bool,
}

/// A walk of a tree's tests to its leaves: the block it's in, and what it
/// finds.
pub(super) struct Walk<'s, 'w> {
    pub(super) label: &'s str,
    pub(super) told: &'w Told<'w>,
    pub(super) leaves: &'w mut Vec<Leaf<'s>>,
    /// Each test of the tree, once: the `if`'s, and what it tests, a
    /// guard's of the value a binding gives in its place.
    pub(super) tested: &'w mut Vec<(&'s Expr, Expr)>,
    /// Each part of a test that calls, a guard's (`Model`).
    pub(super) calls: &'w mut Vec<Expr>,
    /// Each variable the tree declares between its tests, `let x;` of an
    /// arm's binding MIR declares where the tree reaches the arm's
    /// pattern: declared before the chain, once (#771).
    pub(super) declared: &'w mut Vec<&'s Stmt>,
    /// Each variable a guard's binding declares that the walk read as its
    /// value, `let x = b;`.
    pub(super) locals: &'w mut Vec<String>,
    /// The block's body and what follows it in its list.
    pub(super) around: (&'s [Stmt], &'s [Stmt]),
}

impl<'s> Walk<'s, '_> {
    /// The leaves of `frames`, the rest of each list the walk is in,
    /// innermost last, given that `tests` hold, and what each guard's
    /// binding read in a test there is (`given`, `Walk::test_at`): `None`
    /// where one isn't a tree's leaf.
    pub(super) fn walk(
        &mut self,
        mut frames: Vec<&'s [Stmt]>,
        tests: Vec<Expr>,
        given: Vec<(String, Expr)>,
    ) -> Option<()> {
        if self.leaves.len() > LEAVES {
            return None;
        }
        while frames.last().is_some_and(|f| f.is_empty()) {
            frames.pop();
        }
        let declares = matches!(
            frames.last().and_then(|f| f.first()).map(|s| &s.kind),
            Some(StmtKind::Let(_, None))
        );
        let leaving = declares && self.leaf(&frames).is_none();
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
        // A variable declared between the tree's tests, `let x;` of an arm's
        // binding, where what's done from here is no leaf, a test past it
        // leaving the block: declared before the chain (#771); one named
        // outside what follows it, another variable of its name, stays.
        let (body, after) = self.around;
        if let StmtKind::Let(name, None) = &stmt.kind
            && !rest.is_empty()
            && leaving
            && js::mentions_in(body, name) == js::mentions_in(current, name)
            && js::mentions_in(after, name) == 0
        {
            self.declared.push(stmt);
            *frame = rest;
            return self.walk(frames, tests, given);
        }
        if let Some(Test {
            test,
            written,
            then,
            otherwise,
            rest,
            given: bound,
            locals,
        }) = self.test_at(stmt, rest, &given)
        {
            self.locals.extend(locals);
            let given = [given, bound].concat();
            *frame = rest;
            if !self.tested.iter().any(|(t, _)| std::ptr::eq(*t, written)) {
                self.tested.push((written, test.clone()));
                let mut parts = Vec::new();
                leaf_tests(&test, js::Span::NONE, &mut parts);
                self.calls
                    .extend(parts.into_iter().filter(|(t, _)| !pure(t)).map(|(t, _)| t.clone()));
            }
            // A way no value takes, `b === 1` once `b >= 1 && b <= 2` failed,
            // has no leaf: not walked, so a run of `if`s each going on to
            // the next is as many ways as its values take (#771).
            let taken = |tests: &[Expr]| simplified(tests, false).is_some();
            let mut yes = frames.clone();
            yes.push(then);
            let passing = [tests.clone(), vec![test.clone()]].concat();
            if taken(&passing) {
                self.walk(yes, passing, given.clone())?;
            }
            let mut no = frames;
            if let Some(otherwise) = otherwise {
                no.push(otherwise);
            }
            let failing = [tests, vec![test.negated_test()]].concat();
            if !taken(&failing) {
                return Some(());
            }
            return self.walk(no, failing, given);
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
                let (body, out, leaves_loop) = self.leaf(&frames)?;
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

    /// What's done from `frames` on, to what ends it, a leaf's: its
    /// statements, whether it goes on past the block, and whether it ends
    /// with a loop whose own `break`s leave the block. `None` where a
    /// `break` of the block is in it but where it ends.
    fn leaf(&self, frames: &[&'s [Stmt]]) -> Option<(Vec<&'s Stmt>, bool, bool)> {
        let label = self.label;
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
        Some((body, out, leaves_loop))
    }
}

/// A test `Walk::test_at` finds: what it tests, the `if`'s own, its
/// branches, and what follows it.
pub(super) struct Test<'s> {
    pub(super) test: Expr,
    pub(super) written: &'s Expr,
    pub(super) then: &'s [Stmt],
    pub(super) otherwise: Option<&'s [Stmt]>,
    pub(super) rest: &'s [Stmt],
    /// What the bindings before it give, read in its branches' tests too.
    pub(super) given: Vec<(String, Expr)>,
    /// Those of them that declare their variable, `let x = b;`.
    pub(super) locals: Vec<String>,
}
