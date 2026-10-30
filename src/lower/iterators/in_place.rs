//! A chain stepped in place, and what a lent one, `nth` and an array's
//! methods do.

use super::*;

/// What a JS iterator's own methods step till its first `done`, and no more:
/// its helpers and its consumers; and `Array.from`, `collect()`'s.
pub(super) const STEPPED_TILL_DONE: [&str; 10] = [
    "map", "filter", "take", "drop", "flatMap", "reduce", "forEach", "some", "every", "find",
];

/// What the runtime's helpers step their first argument till its first
/// `done` with, each item as it comes (#179): the extremes', `partition`'s
/// and `unzip`'s.
pub(in crate::lower) const HELPERS_STEPPING_ALL: [&str; 8] = [
    "$max",
    "$min",
    "$maxBy",
    "$minBy",
    "$maxByKey",
    "$minByKey",
    "$partition",
    "$unzip",
];

/// `e`, stepped till it ends, by a `for` or not: of an `$intoIter(xs)`,
/// where nothing reads where it is, JS's iterator of `xs`, which a `for`
/// makes itself (ADR 0475); of a `fuse()` of a JS iterator, the iterator,
/// which ends at its first `done` as the `Fuse` would, and of `$knowing`
/// of an array's iterator, which knows its back and length already, the
/// iterator (ADR 0561); through `$rev` of one.
pub(super) fn stepped_in_place(e: &mut Expr, by_for: bool, fuse: Option<&str>) {
    let js::ExprKind::Call(callee, given) = &mut e.kind else {
        return;
    };
    if matches!(callee.helper_name(), Some("$rev" | "$knowing")) && !given.is_empty() {
        stepped_in_place(&mut given[0], false, fuse);
    }
    // An iterator of the crate's stepped till it ends, which nothing asks
    // its size: given no `size_hint`, nor the places before it it needs.
    if callee.helper_name() == Some("$iterator") && given.len() >= 6 {
        given[5] = Expr::undefined();
        fewest_iterator_args(given);
        return;
    }
    let fused = matches!(&callee.kind, js::ExprKind::Var(name) if Some(name.as_str()) == fuse);
    let replacement = match callee.helper_name() {
        Some("$intoIter") if by_for => given[0].clone(),
        Some("$intoIter") => Expr::call(Expr::member(Expr::var("Iterator"), "from"), given.clone()),
        Some("$knowing") if is_stepped_items(&given[0]) => given[0].clone(),
        // A `fuse()` given no drops, its dictionary and its fact alone, the
        // drops it doesn't use left off (ADR 0300): its source. One given
        // its source's drop stays, which drops what it didn't give as it's
        // closed (ADR 0983).
        _ if fused && given.len() <= 3 && given.first().is_some_and(is_js_iterator) => given[0].clone(),
        _ => return,
    };
    *e = replacement;
    if fused {
        stepped_in_place(e, by_for, fuse);
    }
}

/// Whether `e` is `$lent(x)`, a JS iterator nothing closes.
pub(super) fn is_lent(e: &Expr) -> bool {
    !std::ptr::eq(unlent(e), e)
}

/// What `$lent(e)` lends, `e`, made for this call alone, which no one else
/// steps; or `e` itself.
pub(in crate::lower) fn unlent(e: &Expr) -> &Expr {
    match &e.kind {
        js::ExprKind::Call(lent, of) if lent.helper_name() == Some("$lent") && of.len() == 1 => &of[0],
        _ => e,
    }
}

/// Of `Array.from(chain.drop(n).take(1))`, `nth(n)`'s first: `chain` and `n`.
pub(super) fn nth_of(e: &Expr) -> Option<(&Expr, &Expr)> {
    fn call<'e>(e: &'e Expr, name: &str) -> Option<(&'e Expr, &'e [Expr])> {
        match &e.kind {
            js::ExprKind::Call(callee, args) => match &callee.kind {
                js::ExprKind::Member(of, m) if m == name => Some((&**of, args.as_slice())),
                _ => None,
            },
            _ => None,
        }
    }
    let taken = e.collected_of()?;
    let (dropped, [one]) = call(taken, "take")? else {
        return None;
    };
    let (chain, [n]) = call(dropped, "drop")? else {
        return None;
    };
    (one.as_int() == Some(1)).then_some((chain, n))
}

/// `items.name(f)`, of an array, `f` taking each enumerated pair apart
/// given each item and its index, as `indexed_callback` has it.
pub(super) fn array_method(items: Expr, name: &str, f: Expr) -> Expr {
    let (items, f) = indexed_callback(&items, name, f).unwrap_or_else(|f| (items, f));
    Expr::call(Expr::member(items, name), vec![f])
}

/// Whether `f` is `count()`'s, `(n) => n + 1`.
pub(super) fn counts(f: &Expr) -> bool {
    let js::ExprKind::Arrow(params, body) = &f.kind else {
        return false;
    };
    let [js::Pattern::Name(n)] = params.as_slice() else {
        return false;
    };
    matches!(body.as_slice(), [Stmt { kind: StmtKind::Return(Some(e)), .. }]
        if matches!(&e.kind, js::ExprKind::Binary(Op::Add, a, one)
            if matches!(&a.kind, js::ExprKind::Var(v) if v == n) && one.as_int() == Some(1)))
}

/// Whether calling `f` may throw, a panic: anything but an arrow none of
/// whose statements may, or one of the crate's functions that never
/// unwinds (`Known::NeverThrows`, ADR 0902), as an owning chain's stage it
/// runs over each item of then, which a panic stops early (#422, #423).
pub(in crate::lower) fn may_throw(f: &Expr) -> bool {
    fn stmts_may_throw(stmts: &[Stmt]) -> bool {
        stmts.iter().any(|stmt| match &stmt.kind {
            StmtKind::Const(_, e) | StmtKind::Let(_, Some(e)) | StmtKind::Return(Some(e)) | StmtKind::Expr(e) => {
                e.may_throw()
            }
            StmtKind::Let(_, None) | StmtKind::Return(None) => false,
            StmtKind::Assign(target, e) => target.may_throw() || e.may_throw(),
            StmtKind::If(test, then, otherwise) => {
                test.may_throw() || stmts_may_throw(then) || otherwise.as_deref().is_some_and(stmts_may_throw)
            }
            _ => !stmt.cannot_throw(),
        })
    }
    match &f.kind {
        js::ExprKind::Arrow(_, body) => stmts_may_throw(body),
        _ => !f.facts.has(js::Known::NeverThrows),
    }
}

/// Whether stepping a chain, `it`, may throw by what its stages are given:
/// an arrow that may, `may_throw`'s, or of a chain given a stage, its own
/// (#422).
pub(in crate::lower) fn chain_may_throw(it: &Expr) -> bool {
    let js::ExprKind::Call(callee, args) = &it.kind else {
        return false;
    };
    let own = matches!(&callee.kind, js::ExprKind::Member(object, _) if chain_may_throw(object));
    own || args.iter().any(|arg| match &arg.kind {
        js::ExprKind::Arrow(..) => may_throw(arg),
        js::ExprKind::Call(..) => chain_may_throw(arg),
        _ => false,
    })
}

/// How many callbacks a chain, `it`, gives its stages, as its JS shows them:
/// each function it's given, an arrow or a function's name (#423).
pub(in crate::lower) fn callbacks_in(it: &Expr) -> usize {
    let js::ExprKind::Call(callee, args) = &it.kind else {
        return 0;
    };
    let own = match &callee.kind {
        js::ExprKind::Member(object, _) => callbacks_in(object),
        _ => 0,
    };
    own + (args.iter())
        .map(|arg| match &arg.kind {
            js::ExprKind::Arrow(..) => 1,
            js::ExprKind::Var(_) if arg.facts.has(js::Known::Item) => 1,
            js::ExprKind::Call(..) => callbacks_in(arg),
            _ => 0,
        })
        .sum::<usize>()
}

/// Whether a chain, `it`, steps an owning source closed to drop what it
/// hasn't given, `$closing(..)`.
pub(in crate::lower) fn closes_a_source(it: &Expr) -> bool {
    let js::ExprKind::Call(callee, args) = &it.kind else {
        return false;
    };
    callee.helper_name() == Some("$closing")
        || matches!(&callee.kind, js::ExprKind::Member(object, _) if closes_a_source(object))
        || args.iter().any(closes_a_source)
}

/// Whether calling `f` may do what can be seen, or throw: anything but an
/// arrow whose statements only make and give back what does nothing.
pub(in crate::lower) fn does(f: &Expr) -> bool {
    // What `has_effects` says, the one answer: of a `str`'s or a slice's
    // methods that only read it, `s.startsWith(t)`, their operands'
    // (`Known::Read`), as of the runtime's pure helpers, `$find(s, t)`.
    fn acts(e: &Expr) -> bool {
        e.has_effects()
    }
    // Of what it declares, `declared`, an assignment sets nothing outside
    // it: `let sign; if (x < 0) { sign = "-"; } else { sign = "+"; }`, as
    // the program wrote it (rd187).
    fn body_does(stmts: &[Stmt], declared: &mut Vec<String>) -> bool {
        stmts.iter().any(|stmt| match &stmt.kind {
            StmtKind::Const(name, e) | StmtKind::Let(name, Some(e)) => {
                declared.push(name.clone());
                acts(e)
            }
            StmtKind::Return(Some(e)) | StmtKind::Expr(e) => acts(e),
            StmtKind::Destructure { value, .. } => acts(value),
            StmtKind::Let(name, None) => {
                declared.push(name.clone());
                false
            }
            StmtKind::Return(None) => false,
            StmtKind::Assign(target, e) => {
                !matches!(&target.kind, js::ExprKind::Var(name) if declared.contains(name)) || acts(e)
            }
            StmtKind::If(test, then, otherwise) => {
                acts(test)
                    || body_does(then, &mut declared.clone())
                    || otherwise
                        .as_deref()
                        .is_some_and(|o| body_does(o, &mut declared.clone()))
            }
            _ => true,
        })
    }
    match &f.kind {
        js::ExprKind::Arrow(_, body) => body_does(body, &mut Vec::new()),
        js::ExprKind::Var(_) => !f.helper_name().is_some_and(crate::runtime::is_pure),
        _ => true,
    }
}
