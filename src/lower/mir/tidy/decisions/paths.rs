//! An arm's paths, the tests on the way to each of its leaves: each
//! path's tests without what those before it say, and paths alike but
//! for one value's test one path (ADR 0364, #221, #617, #631).

use super::*;

/// An arm's `paths` alike but for what each tests of one value, `a ===
/// 0 && b === 1` and `a !== 0 && a !== 1 && b === 1`: one, that value's
/// test of either's values, `a !== 1 && b === 1`, in its place, or none of
/// any value, so each test is still read where it was (#631). The tests
/// are pure: `(x && s && y) || (x && t && y)` is `x && (s || t) && y`.
pub(super) fn unions(paths: &mut Vec<Vec<Expr>>) {
    let mut i = 0;
    while i < paths.len() {
        let union = (i + 1..paths.len()).find_map(|j| {
            let (p, q) = (&paths[i], &paths[j]);
            // Of paths that test more, alike: a value's tests alone are
            // its alternatives as written, `c === "." || ..` (#631).
            if p.len() != q.len() || p.len() < 2 {
                return None;
            }
            let differ: Vec<usize> = (0..p.len()).filter(|&k| !p[k].written_alike(&q[k])).collect();
            let [k] = differ.as_slice() else { return None };
            let ((s, x), (t, y)) = (atom(&p[*k])?, atom(&q[*k])?);
            (same_subject(&s, &t) && x.kind == y.kind).then(|| (j, *k, s, x.union(&y)))
        });
        let Some((j, k, subject, set)) = union else {
            i += 1;
            continue;
        };
        paths.remove(j);
        match set == Set::all(set.kind) {
            true => drop(paths[i].remove(k)),
            false => {
                let span = paths[i][k].span;
                paths[i][k] = Expr {
                    span,
                    ..render(&subject, &set)
                };
            }
        }
    }
}

/// `tests`, each read where it's made, without what those before it in a
/// run of tests of one value already say: `None` where no value passes
/// them all.
pub(super) fn simplified(tests: &[Expr]) -> Option<Vec<Expr>> {
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
    // where it was, and the path's tests read `a === 2 && b === 2`; or an
    // or-pattern's alternatives each of which does, `a !== 1` before `(b
    // === 3 && a === 2) || (b === 2 && a === 3)` (#617).
    // And one an or-pattern's alternatives before it each say, `b !== 1`
    // after them, goes (#631).
    let mut i = 0;
    while i < out.len() {
        let stronger =
            atom(&out[i]).and_then(|(subject, set)| (i + 1..out.len()).find(|&j| says_within(&out[j], &subject, &set)));
        let said = atom(&out[i]).is_some_and(|(subject, set)| {
            (0..i).any(|j| atom(&out[j]).is_none() && says_within(&out[j], &subject, &set))
        });
        match stronger {
            Some(j) => out[i] = out.remove(j),
            None if said => drop(out.remove(i)),
            None => i += 1,
        }
    }
    // A path whose or-pattern no alternative of holds, of what it says
    // of a value, `a === 0 && ((b === 3 && a === 2) || ..)`: none (#631).
    let excluded =
        (out.iter()).any(|t| atom(t).is_none() && (known.iter()).any(|(subject, set)| excludes(t, subject, set)));
    (!excluded).then_some(out)
}

/// Whether `e` says of `subject` no more than `set` holds: a test of it
/// within `set`, `a === 2` of `a !== 1`'s, of an `&&` one of its tests, of
/// an `||` each of its alternatives, `(b === 3 && a === 2) || (b === 2 &&
/// a === 3)` (`simplified`, #617).
fn says_within(e: &Expr, subject: &Expr, set: &Set) -> bool {
    match &e.kind {
        ExprKind::Binary(Op::And, a, b) => says_within(a, subject, set) || says_within(b, subject, set),
        ExprKind::Binary(Op::Or, a, b) => says_within(a, subject, set) && says_within(b, subject, set),
        _ => atom(e).is_some_and(|(s, later)| same_subject(&s, subject) && later.within(set)),
    }
}

/// Whether `e` is false wherever `subject`'s value is in `set`: a test
/// of it outside `set`, of an `&&` one of its tests, of an `||` each of
/// its alternatives, `(b === 3 && a === 2) || (b === 2 && a === 3)` of `a
/// === 0`'s (`simplified`, #631).
fn excludes(e: &Expr, subject: &Expr, set: &Set) -> bool {
    match &e.kind {
        ExprKind::Binary(Op::And, a, b) => excludes(a, subject, set) || excludes(b, subject, set),
        ExprKind::Binary(Op::Or, a, b) => excludes(a, subject, set) && excludes(b, subject, set),
        _ => atom(e).is_some_and(|(s, other)| {
            same_subject(&s, subject) && other.kind == set.kind && other.intersect(set).is_empty()
        }),
    }
}
