//! An arm's paths, the tests on the way to each of its leaves: each
//! path's tests without what those before it say, and paths alike but
//! for one value's test one path (ADR 0364, #221, #617, #631).

use super::*;

/// An arm's `paths` alike but for what each tests of one value, `a ===
/// 0 && b === 1` and `a !== 0 && a !== 1 && b === 1`: one, that value's
/// test of either's values, `a !== 1 && b === 1`, in its place, or none of
/// any value, so each test is still read where it was (#631). The tests
/// are pure: `(x && s && y) || (x && t && y)` is `x && (s || t) && y`.
/// Of paths that are each a test of one value, as `factored` leaves those
/// alike but for it, only where they're of any value, `a !== 2 || a ===
/// 2`, which is no test, an empty path (#673).
pub(super) fn unions(paths: &mut Vec<Vec<Expr>>, known: &[(Expr, Set)]) {
    let mut i = 0;
    while i < paths.len() {
        let union = (i + 1..paths.len()).find_map(|j| {
            let (p, q) = (&paths[i], &paths[j]);
            if p.len() != q.len() || p.is_empty() {
                return None;
            }
            let differ: Vec<usize> = (0..p.len()).filter(|&k| !p[k].written_alike(&q[k])).collect();
            let [k] = differ.as_slice() else { return None };
            let ((s, x), (t, y)) = (atom(&p[*k])?, atom(&q[*k])?);
            let set = (same_subject(&s, &t) && x.kind == y.kind).then(|| x.union(&y))?;
            // Of paths that test more, alike: a value's tests alone are
            // its alternatives as written, `c === "." || ..` (#631).
            (p.len() > 1 || any_here(known, &s, &set)).then_some((j, *k, s, set))
        });
        let Some((j, k, subject, set)) = union else {
            i += 1;
            continue;
        };
        paths.remove(j);
        match any_here(known, &subject, &set) {
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

/// Whether `set` holds of each value `subject` can have where `known`,
/// what the chain's arms before say failing, holds: of any value, or of
/// those left, `a !== 1` of `a === 0 || (a !== 0 && a !== 1)` past `a ===
/// 1`'s arm (#673).
fn any_here(known: &[(Expr, Set)], subject: &Expr, set: &Set) -> bool {
    known_of(known, subject, set.kind).within(set)
}

/// What `known` says of `subject`'s values of `kind`: each it can be.
fn known_of(known: &[(Expr, Set)], subject: &Expr, kind: Kind) -> Set {
    (known.iter())
        .filter(|(s, set)| same_subject(s, subject) && set.kind == kind)
        .fold(Set::all(kind), |all, (_, set)| all.intersect(set))
}

/// What `test` failing says of each value it tests, as one test of one
/// value does, and an `||` each of whose alternatives does: `a !== 1`,
/// `b !== 1` of `a === 1 || b === 1` (#673).
pub(super) fn failing(test: &Expr) -> Vec<(Expr, Set)> {
    match (&test.kind, atom(test)) {
        (_, Some((subject, set))) => vec![(subject, set.complement())],
        (ExprKind::Binary(Op::Or, a, b), None) => [failing(a), failing(b)].concat(),
        _ => Vec::new(),
    }
}

/// Whether `known` says `test`: true of each value its value can be there.
pub(super) fn said(known: &[(Expr, Set)], test: &Expr) -> bool {
    atom(test).is_some_and(|(subject, set)| {
        (known.iter()).any(|(s, of)| same_subject(s, &subject) && of.kind == set.kind)
            && any_here(known, &subject, &set)
    })
}

/// `tests`, each read where it's made, without what those before it in a
/// run of tests of one value already say: `None` where no value passes
/// them all.
pub(super) fn simplified(tests: &[Expr], patterns: bool) -> Option<Vec<Expr>> {
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
        // A run those before it say, `b !== 1` after `b === 0`: none, its
        // tests true wherever they're read (#673).
        if patterns && at.is_some() && now == before {
            i = j;
            continue;
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
    // === 3 && a === 2) || (b === 2 && a === 3)` (#617). One the
    // alternatives before it each say, `b !== 1` after them, goes as the
    // arm before them failing says it (`said`, #631, #673).
    let mut i = 0;
    while i < out.len() {
        let stronger =
            atom(&out[i]).and_then(|(subject, set)| (i + 1..out.len()).find(|&j| says_within(&out[j], &subject, &set)));
        // And a later test of it that says less, `c !== 1` after `c !== 0 &&
        // b === 0`: what both say, where the first is, its value read there
        // already (#673).
        let merged = atom(&out[i]).filter(|_| patterns).and_then(|(subject, set)| {
            (i + 1..out.len()).find_map(|j| {
                let (s, later) = atom(&out[j])?;
                (same_subject(&s, &subject) && later.kind == set.kind)
                    .then(|| (j, render(&subject, &set.intersect(&later))))
            })
        });
        match (stronger, merged) {
            (Some(j), _) => out[i] = out.remove(j),
            (None, Some((j, both))) => {
                out.remove(j);
                out[i] = Expr {
                    span: out[i].span,
                    ..both
                };
            }
            (None, None) => i += 1,
        }
    }
    Some(out)
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

/// `path`'s tests in the order they're written, `a === 2 && b === 3` of
/// `(2, 3)`, where MIR tests `b` first, after `(_, 1)`'s `b === 1` (#632):
/// of tests each of a part of one tuple MIR made, `_5[1]`, whose reads
/// neither call nor throw, so none guards what another reads. A tuple's
/// patterns are written in the order of its parts, each alternative's
/// tests placed where it is: by that, then by the part. Otherwise as MIR
/// reads them, a slice's `length` before the index it guards, `x != null`
/// before `x.v`, a getter where it ran.
pub(super) fn written_order(path: &mut [Expr]) {
    let part = |t: &Expr| {
        let (tuple, n) = part(t)?;
        Some((written_at(t)?, tuple, n))
    };
    let Some(parts) = path.iter().map(part).collect::<Option<Vec<_>>>() else {
        return;
    };
    if parts.iter().any(|(_, tuple, _)| *tuple != parts[0].1) {
        return;
    }
    let mut keyed: Vec<((u32, f64), Expr)> = (parts.into_iter().map(|(at, _, n)| (at, n)))
        .zip(path.iter().cloned())
        .collect();
    keyed.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    for (slot, (_, test)) in path.iter_mut().zip(keyed) {
        *slot = test;
    }
}

/// Of an arm's `j`th path, each test whose failing, with the path's other
/// tests, says another of its paths, `a !== 2` of `a !== 2 && b === 2 &&
/// c === 2` beside `a === 2 && b === 2`: gone, `(a === 2 && b === 2) ||
/// (b === 2 && c === 2)`, as `p || (!t && q)` is `p || q` where `t && q`
/// says `p`, the tests pure (#673). What it then holds of more is what
/// that other path holds of, the same arm. Not of a test that guards one
/// after it (`guards`), which reads where it held. Of the paths before it
/// first, `b !== 2` of `a === 2 && b !== 2 && c === 2 && d === 2` after
/// `a === 2 && b === 2`, then of those after it.
pub(super) fn absorbed(paths: &mut [Vec<Expr>], j: usize) {
    while paths[j].len() > 1 {
        let tests = &paths[j];
        let goes = |k: usize, others: &[Vec<Expr>]| {
            !(k + 1..tests.len()).any(|u| guards(&tests[k], &tests[u]))
                && (others.iter()).any(|path| !path.is_empty() && failing_says(tests, k, path))
        };
        let gone = (0..tests.len())
            .find(|&k| goes(k, &paths[..j]))
            .or_else(|| (0..tests.len()).find(|&k| goes(k, &paths[j + 1..])));
        match gone {
            Some(k) => drop(paths[j].remove(k)),
            None => break,
        }
    }
}

/// Whether `tests`, their `k`th failing, hold only where each of `path`'s
/// tests does: a test of a value, of what they say of it, or another test
/// alike.
fn failing_says(tests: &[Expr], k: usize, path: &[Expr]) -> bool {
    let failed = tests[k].clone().negated_test();
    let holding: Vec<&Expr> = (tests.iter().enumerate())
        .filter(|&(i, _)| i != k)
        .map(|(_, t)| t)
        .chain([&failed])
        .collect();
    path.iter().all(|test| match atom(test) {
        Some((subject, set)) => {
            let said = (holding.iter().filter_map(|t| atom(t)))
                .filter(|(s, of)| same_subject(s, &subject) && of.kind == set.kind)
                .map(|(_, of)| of)
                .reduce(|a, b| a.intersect(&b));
            said.is_some_and(|said| said.within(&set))
        }
        None => holding.iter().any(|t| t.written_alike(test)),
    })
}

/// Whether `test` may say whether `later` can be read: of an object
/// `later` reads a part of, `x != null` of `x[0]`'s, `x` or what holds
/// it; or a part of what holds one, `shape.TAG` of `shape._0[1]`'s, whose
/// variant says what its parts are. Not one part of another of what
/// holds both, `a[0]` of `a[1]`'s, `shape.TAG` of `shape._0`'s, a read of
/// which, of an object, neither throws nor calls. Of what it can't tell,
/// a call's, it may.
pub(super) fn guards(test: &Expr, later: &Expr) -> bool {
    let (Some(read), Some(after)) = (places(test), places(later)) else {
        return true;
    };
    let objects: Vec<&Expr> = after.iter().flat_map(|place| holders(place)).collect();
    (read.iter()).any(|place| match holder(place) {
        Some(of) => objects.iter().any(|o| holder(o).is_some_and(|o| within_place(of, o))),
        None => objects.iter().any(|o| within_place(place, o)),
    })
}

/// The places `e` reads, `x[0]` and `n` of `x[0] === n`: `None` where it
/// reads what isn't one, a call.
fn places(e: &Expr) -> Option<Vec<&Expr>> {
    let mut out = Vec::new();
    fn walk<'e>(e: &'e Expr, out: &mut Vec<&'e Expr>) -> Option<()> {
        match &e.kind {
            ExprKind::Var(_) | ExprKind::Member(..) | ExprKind::Index(..) => out.push(e),
            ExprKind::Unary(_, a) => walk(a, out)?,
            ExprKind::Binary(_, a, b) => {
                walk(a, out)?;
                walk(b, out)?;
            }
            _ if Value::of(e).is_some() => {}
            ExprKind::Null | ExprKind::Undefined | ExprKind::Bool(_) => {}
            _ => return None,
        }
        Some(())
    }
    walk(e, &mut out)?;
    Some(out)
}

/// What `place` is a part of, `x` of `x[0]`.
fn holder(place: &Expr) -> Option<&Expr> {
    match &place.kind {
        ExprKind::Member(of, _) | ExprKind::Index(of, _) => Some(of),
        _ => None,
    }
}

/// What holds `place`, each, `x[0]` and `x` of `x[0].v`.
fn holders(place: &Expr) -> impl Iterator<Item = &Expr> {
    std::iter::successors(holder(place), |p| holder(p))
}

/// Whether `inner` is `outer` or a part of it.
fn within_place(outer: &Expr, inner: &Expr) -> bool {
    inner.written_alike(outer) || holders(inner).any(|h| h.written_alike(outer))
}

/// The tuple MIR made and which of its parts `test` tests, `_5` and `1` of
/// `_5[1] === 3`.
pub(super) fn part(test: &Expr) -> Option<(String, f64)> {
    let (subject, _) = atom(test)?;
    let ExprKind::Index(of, at) = &subject.kind else {
        return None;
    };
    let (ExprKind::Var(tuple), ExprKind::Num(n)) = (&of.kind, &at.kind) else {
        return None;
    };
    Some((tuple.clone(), *n))
}
