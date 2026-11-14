//! A guarded `match` arm's bindings (#221, #523): MIR's binding of what a
//! guard reads, given just before its test, read as its value
//! (`bound`), and a guarded `|`'s arm bindings set in its guard's test,
//! A1's form, so its alternatives are one arm of the chain
//! (`bound_where_read`, C1).

use super::*;

/// The variables of `body` each assigned what reads nothing that changes
/// just before a test reads it, past others so given, and named nowhere
/// else: MIR's binding of a pattern's value a guard reads, `x = b; if
/// (odd(x))`, `(1, x) | (x, 1) if odd(x)`'s (#221), or `r$1 = o._0; if
/// (!(r$1 > 1))` of a guard of `guards` that calls nothing, `Some(Circle(r)
/// | Sphere(r)) if r > 1.0` (#523); read in the `if`s MIR nests of the
/// guard too, or by nothing, `x$1 = s[0]; y$1 = s[1]; if (g(c, y$1))`
/// (#771).
pub(in crate::lower::mir::tidy) fn bound(body: &[Stmt], guards: &[Guard]) -> IndexSet<String> {
    let mut named: IndexMap<&str, usize> = IndexMap::default();
    js::visit_stmts(body, &mut |name| *named.entry(name).or_default() += 1);
    let mut given: IndexMap<String, usize> = IndexMap::default();
    let binding = |s: &Stmt| match &s.kind {
        StmtKind::Assign(target, value) => matches!(target.kind, ExprKind::Var(_)) && pure(value),
        StmtKind::Const(_, value) => pure(value),
        _ => false,
    };
    js::each_list(body, &mut |list| {
        for (i, stmt) in list.iter().enumerate() {
            // A `let` right before a block MIR made, read only by its
            // tree's tests (`given_before`), past the declarations of the
            // arms' bindings, `let x;`, which read nothing (#1620).
            let block_at = (i + 1..list.len()).find(|&j| !matches!(list[j].kind, StmtKind::Let(_, None)));
            if let StmtKind::Let(name, Some(value)) = &stmt.kind
                && let Some(StmtKind::Labeled(label, block)) = block_at.and_then(|j| list.get(j)).map(|s| &s.kind)
                && crate::lower::mir::is_block_label(label)
                && pure(value)
                && let Some(reads) = tested_reads(block, name).filter(|&n| n > 0)
            {
                *given.entry(name.clone()).or_default() += reads;
            }
            // An assignment, or a `const` of another arm's binding of the
            // name, `const x$3 = _6[1];`, whose reads are counted too, as
            // they're the name's (#771).
            let (name, value, set) = match &stmt.kind {
                StmtKind::Assign(target, value) => match &target.kind {
                    ExprKind::Var(name) => (&**name, value, 1),
                    _ => continue,
                },
                StmtKind::Const(name, value) | StmtKind::Let(name, Some(value)) => (name, value, 0),
                _ => continue,
            };
            let Some(at) = (i + 1..list.len()).find(|&j| !binding(&list[j])) else {
                continue;
            };
            let StmtKind::If(test, then, otherwise) = &list[at].kind else {
                continue;
            };
            let otherwise = otherwise.as_deref();
            let unread = !test.mentions_var(name)
                && pure(value)
                && !value.mentions_var(name)
                && only_tested(then, name)
                && otherwise.is_none_or(|o| only_tested(o, name));
            if unread || binds(name, value, test, then, otherwise, guards) {
                let reads = js::mentions_in(&[StmtKind::Expr(test.clone()).at(list[at].span)], name)
                    + tested_reads(then, name).unwrap_or(0)
                    + otherwise.and_then(|o| tested_reads(o, name)).unwrap_or(0);
                *given.entry(name.clone()).or_default() += set + reads;
            }
        }
    });
    (given.into_iter())
        .filter(|(name, n)| named.get(name.as_str()) == Some(n))
        .map(|(name, _)| name)
        .collect()
}

/// Whether `name`, given `value` just before `if (test) then else
/// otherwise`, is read there only by its test, one that calls or a guard
/// of `guards`, and what it's given reads nothing that changes: that test,
/// of `value` in its place, is the same. Not of a test the program wrote
/// that calls nothing, `const _31 = heap[i]; if (_31 < heap[j])`, which
/// reads as written.
pub(super) fn binds(
    name: &str,
    value: &Expr,
    test: &Expr,
    then: &[Stmt],
    otherwise: Option<&[Stmt]>,
    guards: &[Guard],
) -> bool {
    let guard = || {
        let mut parts = Vec::new();
        leaf_tests(test, js::Span::NONE, &mut parts);
        (parts.iter()).all(|(_, at)| !at.is_none() && guards.iter().any(|g| within(*at, g.span)))
    };
    pure(value)
        && !value.mentions_var(name)
        && (!pure(test) || guard())
        && test.mentions_var(name)
        && only_tested(then, name)
        && otherwise.is_none_or(|o| only_tested(o, name))
}

/// What the `let`s right before a block MIR made give, `before`'s last,
/// each a pure read that the block's tree alone reads, in its tests, and
/// nothing after it: the binding of a guard's first alternative, `let x$1 =
/// _4[0]; bb18: { if (h(c, x$1) == null) { .. x$1 = _4[0]; ..`, which the
/// walk reads as its value, as it does the others' (#771). Past the
/// declarations of the arms' bindings between, `let x;`, which read
/// nothing (#1620).
pub(super) fn given_before(before: &[Stmt], body: &[Stmt], after: &[Stmt]) -> Vec<(String, Expr)> {
    let mut given = Vec::new();
    let declared = (before.iter().rev())
        .take_while(|s| matches!(s.kind, StmtKind::Let(_, None)))
        .count();
    for stmt in before[..before.len() - declared].iter().rev() {
        let (StmtKind::Let(name, Some(value)) | StmtKind::Const(name, value)) = &stmt.kind else {
            break;
        };
        if !pure(value) || value.mentions_var(name) {
            break;
        }
        // Given again in the tree, another alternative's binding: a `const
        // x = v[i];` the tree only tests, `switch (x)`'s, reads as named.
        if js::Writes::sets(body, name)
            && tested_reads(body, name).is_some_and(|n| n > 0)
            && js::mentions_in(after, name) == 0
        {
            given.push((name.clone(), value.clone()));
        }
    }
    given.reverse();
    given
}

/// Of `given`, what `given_before` gave the walk, each `let` of `stmts` no
/// statement reads now the chain reads its value in its place: gone, as
/// what it's given reads nothing that changes, `let x$1 = a;` of `(x, _) |
/// (x, 1) if h(c, x).is_some()` (#1620).
pub(super) fn given_unread(stmts: &mut Vec<Stmt>, given: &[(String, Expr)]) {
    for (name, _) in given {
        let at = stmts
            .iter()
            .position(|s| matches!(&s.kind, StmtKind::Let(n, Some(_)) if n == name));
        if let Some(at) = at
            && js::mentions_in(&stmts[..at], name) + js::mentions_in(&stmts[at + 1..], name) == 0
        {
            stmts.remove(at);
        }
    }
}

/// Whether `stmts` read `name` only in their `if`s' tests (`tested_reads`).
pub(super) fn only_tested(stmts: &[Stmt], name: &str) -> bool {
    tested_reads(stmts, name).is_some()
}

/// How many times `stmts` read `name`, each in an `if`'s test, the `if`s
/// MIR nests of a guard's `&&`, `if (g(c, x)) { if (x > 1) { .. } }`, up to
/// where one of them declares it again, the arm's own binding, `const x =
/// b;`, or gives it again, another alternative's, `x$1 = s[0];`: `None`
/// where another statement reads it (#771).
fn tested_reads(stmts: &[Stmt], name: &str) -> Option<usize> {
    let mut reads = 0;
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::If(test, then, otherwise) => {
                let mut read = 0;
                test.visit_vars(&mut |v| read += usize::from(v == name));
                reads += read
                    + tested_reads(then, name)?
                    + otherwise.as_deref().map_or(Some(0), |o| tested_reads(o, name))?;
            }
            StmtKind::Const(declared, value) | StmtKind::Let(declared, Some(value)) if declared == name => {
                return (!value.mentions_var(name)).then_some(reads);
            }
            StmtKind::Let(declared, None) if declared == name => return Some(reads),
            // Given again, another alternative's binding, `x$1 = s[0];`.
            StmtKind::Assign(target, value)
                if matches!(&target.kind, ExprKind::Var(v) if v == name) && !value.mentions_var(name) =>
            {
                return Some(reads);
            }
            _ if js::mentions_in(std::slice::from_ref(stmt), name) > 0 => return None,
            _ => {}
        }
    }
    Some(reads)
}

/// `test` with what each of `given` names in its place, the latest of a
/// name's.
fn given_in(test: &Expr, given: &[(String, Expr)]) -> Option<Expr> {
    match given.iter().any(|(name, _)| test.mentions_var(name)) {
        true => test.substitute(&|v| (given.iter().rev()).find(|(name, _)| name == v).map(|(_, e)| e.clone())),
        false => Some(test.clone()),
    }
}

impl<'s> Walk<'s, '_> {
    /// The test of the tree `stmt` makes, `rest` after it in its list: an
    /// `if` of what reads nothing that changes, or of a guard the program
    /// wrote, which calls; and a guard's, `x = b; if (odd(x))`, values
    /// given to what they alone read (`binds`), MIR's bindings of a
    /// pattern for its guard, `const x = s[0]; const y = s[s.length - 1];
    /// if (y * 2 > 1)`, each with its value in its place, `odd(b)`, in the
    /// `if`s MIR nests of the guard too, `if (g(c, b)) { if (b > 1)`
    /// (`given`, #771). Of each test, what `given` names is its value.
    pub(super) fn test_at(&self, stmt: &'s Stmt, rest: &'s [Stmt], given: &[(String, Expr)]) -> Option<Test<'s>> {
        let guard = |test: &Expr| {
            let mut parts = Vec::new();
            leaf_tests(test, js::Span::NONE, &mut parts);
            // A `switch`'s guard is a test too, of a tree `chain` leaves to
            // the `switch` (`switched_guard`).
            (parts.iter()).all(|(part, span)| {
                pure(part) || !span.is_none() && (self.told.guards.iter()).any(|g| within(*span, g.span))
            })
        };
        if let StmtKind::If(test, then, otherwise) = &stmt.kind {
            return guard(test).then(|| {
                Some(Test {
                    test: given_in(test, given)?,
                    written: test,
                    then,
                    otherwise: otherwise.as_deref(),
                    rest,
                    given: Vec::new(),
                    locals: Vec::new(),
                })
            })?;
        }
        // The run of bindings, each given what reads nothing that changes,
        // then the `if`.
        let binding = |s: &'s Stmt| match &s.kind {
            StmtKind::Const(name, value) | StmtKind::Let(name, Some(value)) => Some((name, value, true)),
            StmtKind::Assign(target, value) => match &target.kind {
                ExprKind::Var(name) if self.told.bound.contains(name.as_str()) => Some((&**name, value, false)),
                _ => None,
            },
            _ => None,
        };
        let mut run = vec![binding(stmt)?];
        let mut rest = rest;
        let (test, then, otherwise, written) = loop {
            let (next, after) = rest.split_first()?;
            rest = after;
            match &next.kind {
                StmtKind::If(test, then, otherwise) => break (test, then, otherwise.as_deref(), test),
                _ => run.push(binding(next)?),
            }
        };
        // Each read by the test alone, or by nothing, a binding the guard
        // doesn't read, `const x = s[0];` of `&[x, .., y] if y * 2 > 1`; one
        // at least by the test.
        let read = |(name, value, local): &(&String, &Expr, bool)| {
            let unread = !test.mentions_var(name)
                && pure(value)
                && !value.mentions_var(name)
                && only_tested(then, name)
                && otherwise.is_none_or(|o| only_tested(o, name));
            (binds(name, value, test, then, otherwise, self.told.guards) || unread)
                && !(run.iter()).any(|(other, value, _)| other != name && value.mentions_var(name))
                && !(*local && js::mentions_in(rest, name) > 0)
        };
        // Or none read at all, a binding the arm doesn't read either, `(2,
        // x) | (x, 0) if g(c, a * 2, 1) => break` (#771).
        let dead = |(name, ..): &(&String, &Expr, bool)| {
            tested_reads(then, name) == Some(0) && otherwise.is_none_or(|o| tested_reads(o, name) == Some(0))
        };
        let tested = run.iter().any(|(name, ..)| test.mentions_var(name)) || run.iter().all(dead);
        if !run.iter().all(read) || !tested || !guard(test) {
            return None;
        }
        let bound: Vec<(String, Expr)> = (run.iter())
            .map(|(name, value, _)| Some(((*name).clone(), given_in(value, given)?)))
            .collect::<Option<_>>()?;
        let given = [given, &bound].concat();
        Some(Test {
            test: given_in(test, &given)?,
            written,
            then,
            otherwise,
            rest,
            given: bound,
            locals: (run.iter())
                .filter(|(.., local)| *local)
                .map(|(name, ..)| (*name).clone())
                .collect(),
        })
    }
}

/// A guarded `|`'s bindings its arm reads, `(1, x) | (x, 1) if odd(x) =>
/// x * 10`: MIR gives the guard's `x` its value, then, the guard passing,
/// the arm's, once for each alternative, `x$1 = b; if (odd(x$1)) { x = b;
/// break bb19; }`, so the alternatives' arms differ by what `x` is given,
/// and the chain can't make them one arm. The arm's bindings are set in
/// the guard's test instead, A1's form for a guarded or-pattern's bindings
/// (C1, the user's decision, #523): where the guard reads once the one
/// binding the arm gives what the guard's is, where JS reads it each time
/// the test runs (`moves::leads`), there, `odd((x = b))`; otherwise first,
/// each guard's binding read as the arm's, `((x = b), (y = c), odd(x))`,
/// or as its value, `((x = b), odd(a + b))`. The tree with each site of a
/// binding two sites or more set so; `None` where there's none. Each is a
/// binding of the pattern of the guard's own arm (MIR writes it where the
/// pattern names it, `Guard::tested`), read only on the arm's way after an
/// alternative set it, so what a guard that fails sets is read nowhere;
/// each guard's binding is read only by its guard; and each value is a
/// pure read of what the `match` tests, which the guard can't change
/// (E0510), so it's the same read before the guard as after.
pub(super) fn bound_where_read(body: &[Stmt], told: &Told) -> Option<Vec<Stmt>> {
    // The block's own variables, declared first (`chain`'s), and its tree.
    let declared = (body.iter())
        .take_while(|s| matches!(s.kind, StmtKind::Let(_, None)))
        .count();
    let tree = &body[declared..];
    let mut found = Vec::new();
    sites(tree, told, &mut found);
    // Each guard's binding read only by its guards, and given only there.
    let given = |name: &String| {
        let at = |site: &Site| site.before.iter().filter(|(n, _)| n == name).count();
        let reads: usize = (found.iter())
            .filter(|site| at(site) > 0)
            .map(|site| {
                let mut reads = 0;
                site.guard.visit_vars(&mut |v| reads += usize::from(v == name));
                reads
            })
            .sum();
        js::mentions_in(tree, name) == found.iter().map(at).sum::<usize>() + reads
    };
    let read: IndexSet<String> = (found.iter().flat_map(|site| &site.before))
        .map(|(name, _)| name)
        .filter(|name| given(name))
        .cloned()
        .collect();
    found.retain(|site| site.before.iter().all(|(name, _)| read.contains(name)));
    // Each binding two sites or more set, named nowhere else in the tree.
    let set_by = |arm: &String| {
        found
            .iter()
            .filter(|site| site.arms.iter().any(|(a, _)| a == arm))
            .count()
    };
    let many: IndexSet<String> = (found.iter().flat_map(|site| &site.arms))
        .map(|(arm, _)| arm)
        .filter(|arm| set_by(arm) > 1 && js::mentions_in(tree, arm) == set_by(arm))
        .cloned()
        .collect();
    if many.is_empty() {
        return None;
    }
    let mut tree = tree.to_vec();
    rewrite(&mut tree, told, &many, &read);
    Some(body[..declared].iter().cloned().chain(tree).collect())
}

/// A guard's `if` in a tree whose branch first gives the guard's arm's
/// bindings their values (`bound_where_read`), and the guard's bindings
/// given before it.
struct Site {
    /// The arm's bindings, and what each is given.
    arms: Vec<(String, Expr)>,
    /// The guard's, each given just before the `if`.
    before: Vec<(String, Expr)>,
    guard: Expr,
    /// Where the guard is written.
    at: js::Span,
}

impl Site {
    /// The test that sets the arm's bindings.
    fn test(&self) -> Option<Expr> {
        let sets: Vec<Expr> = (self.arms.iter())
            .map(|(arm, value)| Expr::assigned(Expr::var(arm), value.clone()))
            .collect();
        // The one guard's binding it reads, given what the arm's binding is.
        let read: Vec<&(String, Expr)> = (self.before.iter())
            .filter(|(name, _)| self.guard.mentions_var(name))
            .collect();
        if let ([(name, value)], [(_, given)]) = (read.as_slice(), self.arms.as_slice())
            && given.written_alike(value)
        {
            let mut reads = 0;
            self.guard.visit_vars(&mut |v| reads += usize::from(v == name));
            if reads == 1 && super::super::moves::leads(&self.guard, name, None, &|_| true) == Some(true) {
                return self.guard.substitute(&|v| (v == name).then(|| sets[0].clone()));
            }
        }
        let guard = self.guard.substitute(&|v| {
            let (_, value) = self.before.iter().find(|(name, _)| name == v)?;
            Some(match self.arms.iter().find(|(_, given)| given.written_alike(value)) {
                Some((arm, _)) => Expr::var(arm),
                None => value.clone(),
            })
        })?;
        Some(Expr {
            span: self.at,
            ..Expr::sequence(sets.into_iter().chain([guard]).collect())
        })
    }
}

/// The site whose `if` is `list[i]`, of a guard that calls, or that reads
/// a binding of its own given just before it.
fn site(list: &[Stmt], i: usize, told: &Told) -> Option<Site> {
    let StmtKind::If(test, then, _) = &list[i].kind else {
        return None;
    };
    // The guard it tests, and the pattern of its arm, the last before it.
    let mut parts = Vec::new();
    leaf_tests(test, js::Span::NONE, &mut parts);
    let guard = (told.guards.iter()).find(|g| {
        parts.iter().any(|(_, at)| !at.is_none()) && (parts.iter()).all(|(_, at)| at.is_none() || within(*at, g.span))
    })?;
    let pattern = (guard.tested.iter().skip(1))
        .filter(|t| t.hi <= guard.span.lo)
        .max_by_key(|t| t.hi)?;
    let binding = |s: &Stmt| match &s.kind {
        StmtKind::Assign(target, value) if within(s.span, *pattern) && !s.span.is_none() && pure(value) => {
            match &target.kind {
                ExprKind::Var(name) => Some((name.to_string(), value.clone())),
                _ => None,
            }
        }
        _ => None,
    };
    let given: Vec<(String, Expr)> = {
        let mut given: Vec<(String, Expr)> = list[..i].iter().rev().map_while(binding).collect();
        given.reverse();
        given
    };
    let arms: Vec<(String, Expr)> = (then.iter())
        .map_while(binding)
        .filter(|(arm, _)| !test.mentions_var(arm))
        .collect();
    let calls = parts.iter().any(|(part, _)| !pure(part));
    (!arms.is_empty() && (calls || given.iter().any(|(name, _)| test.mentions_var(name)))).then(|| Site {
        arms,
        before: given,
        guard: test.clone(),
        at: guard.span,
    })
}

/// Each site in `list` and the branches of its `if`s, into `out`.
fn sites(list: &[Stmt], told: &Told, out: &mut Vec<Site>) {
    for (i, stmt) in list.iter().enumerate() {
        out.extend(site(list, i, told));
        if let StmtKind::If(_, then, otherwise) = &stmt.kind {
            sites(then, told, out);
            sites(otherwise.as_deref().unwrap_or_default(), told, out);
        }
    }
}

/// Each site in `list` each of whose bindings is one of `many`, and each of
/// its guard's `read`'s, its test setting them, what its guard's are given
/// gone.
fn rewrite(list: &mut Vec<Stmt>, told: &Told, many: &IndexSet<String>, read: &IndexSet<String>) {
    let mut i = 0;
    while i < list.len() {
        if let Some(site) = site(list, i, told)
            && site.arms.iter().all(|(arm, _)| many.contains(arm))
            && site.before.iter().all(|(name, _)| read.contains(name))
            && let Some(test) = site.test()
        {
            let given = site.before.len();
            list.drain(i - given..i);
            i -= given;
            if let StmtKind::If(written, then, _) = &mut list[i].kind {
                *written = Expr {
                    span: match test.span.is_none() {
                        true => written.span,
                        false => test.span,
                    },
                    ..test
                };
                then.drain(..site.arms.len());
            }
        }
        if let StmtKind::If(_, then, otherwise) = &mut list[i].kind {
            rewrite(then, told, many, read);
            if let Some(otherwise) = otherwise {
                rewrite(otherwise, told, many, read);
            }
        }
        i += 1;
    }
}
