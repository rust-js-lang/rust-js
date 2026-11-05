//! A guarded `match` arm's bindings (#221, #523): MIR's binding of what a
//! guard reads, given just before its test, read as its value
//! (`bound`), and a guarded `|`'s arm bindings set in its guard's test,
//! A1's form, so its alternatives are one arm of the chain
//! (`bound_where_read`, C1).

use super::*;

/// The variables of `body` each assigned what reads nothing that changes
/// just before a test reads it, and named nowhere else: MIR's binding of a
/// pattern's value a guard reads, `x = b; if (odd(x))`, `(1, x) | (x, 1) if
/// odd(x)`'s (#221), or `r$1 = o._0; if (!(r$1 > 1))` of a guard of
/// `guards` that calls nothing, `Some(Circle(r) | Sphere(r)) if r > 1.0`
/// (#523).
pub(in crate::lower::mir::tidy) fn bound(body: &[Stmt], guards: &[Guard]) -> HashSet<String> {
    let mut named: HashMap<&str, usize> = HashMap::new();
    js::visit_stmts(body, &mut |name| *named.entry(name).or_default() += 1);
    let mut given: HashMap<String, usize> = HashMap::new();
    js::each_list(body, &mut |list| {
        for pair in list.windows(2) {
            if let StmtKind::Assign(target, value) = &pair[0].kind
                && let ExprKind::Var(name) = &target.kind
                && let StmtKind::If(test, then, otherwise) = &pair[1].kind
                && binds(name, value, test, then, otherwise.as_deref(), guards)
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
        && js::mentions_in(then, name) == 0
        && otherwise.is_none_or(|o| js::mentions_in(o, name) == 0)
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
    let read: HashSet<String> = (found.iter().flat_map(|site| &site.before))
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
    let many: HashSet<String> = (found.iter().flat_map(|site| &site.arms))
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
                ExprKind::Var(name) => Some((name.clone(), value.clone())),
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
fn rewrite(list: &mut Vec<Stmt>, told: &Told, many: &HashSet<String>, read: &HashSet<String>) {
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
