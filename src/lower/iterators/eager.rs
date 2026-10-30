//! A chain made eager, an array's own methods, where Rust's order can't be
//! seen (ADR 0139): its callbacks, its spreads and what it's collected into.

use super::*;

/// `enumerate()` then a method whose callback takes each pair apart,
/// `.map((x, i) => [i, x]).map(([i, w]) => ..)`: the method of the items
/// themselves, whose callback JS gives each one's index too, `.map((w, i)
/// => ..)` (ADR 0217); `reduce`'s, its pair the callback's second
/// parameter, `.reduce((acc, w, i) => .., start)`. Not `filter` or `find`,
/// which give back the pairs. `Err` gives back the callback, of any other.
pub(super) fn indexed_callback(items: &Expr, name: &str, f: Expr) -> Result<(Expr, Expr), Expr> {
    // What the callback is given before each item, `reduce`'s accumulator.
    let before = match name {
        "map" | "forEach" | "some" | "every" | "flatMap" => 0,
        "reduce" => 1,
        _ => return Err(f),
    };
    let js::ExprKind::Call(callee, args) = &items.kind else {
        return Err(f);
    };
    let js::ExprKind::Member(source, method) = &callee.kind else {
        return Err(f);
    };
    if method != "map" || !matches!(&args[..], [pairing] if is_enumerate_pairing(pairing)) {
        return Err(f);
    }
    let js::ExprKind::Arrow(params, body) = &f.kind else {
        return Err(f);
    };
    let (leading, [js::Pattern::Array(parts)]) = params.split_at(before.min(params.len())) else {
        return Err(f);
    };
    // `|(i, _)|`'s, its item left out: `[i]`.
    let (index, item) = match &parts[..] {
        [index, item] => (index, item),
        [index] => (index, &None),
        _ => return Err(f),
    };
    let index = match index {
        Some(js::Pattern::Name(index)) => Some(index.clone()),
        Some(_) => return Err(f),
        None => None,
    };
    // The item, taken apart as it was, `({ name }, i)`; `_` where only the
    // index is read, or `__` where the accumulator is `_` already, JS
    // binding a name once in a parameter list.
    let unread = || {
        let named = |n: &str| leading.iter().any(|p| matches!(p, js::Pattern::Name(l) if l == n));
        (1..)
            .map(|k| "_".repeat(k))
            .find(|n| !named(n) && js::mentions_in(body, n) == 0)
            .expect("a name")
    };
    let mut params = leading.to_vec();
    params.push(item.clone().unwrap_or_else(|| unread().into()));
    params.extend(index.map(js::Pattern::Name));
    Ok((
        (**source).clone(),
        Expr {
            span: f.span,
            ..Expr::arrow(params, body.clone())
        },
    ))
}

/// `(x, i) => [i, x]`, which `enumerate()` maps each item with.
fn is_enumerate_pairing(e: &Expr) -> bool {
    let var = |e: &Expr, name: &str| matches!(&e.kind, js::ExprKind::Var(v) if v == name);
    let js::ExprKind::Arrow(params, body) = &e.kind else {
        return false;
    };
    let [js::Pattern::Name(x), js::Pattern::Name(i)] = &params[..] else {
        return false;
    };
    let [stmt] = &body[..] else { return false };
    let StmtKind::Return(Some(pair)) = &stmt.kind else {
        return false;
    };
    let js::ExprKind::Array(parts) = &pair.kind else {
        return false;
    };
    (x.as_str(), i.as_str()) == ("x", "i")
        && matches!(&parts[..], [first, second] if var(first, "i") && var(second, "x"))
}

/// Whether `items` is an array made here, which nothing else holds: an
/// adapter's, a copy's, a `split`'s, or a helper's.
pub(super) fn fresh(items: &Expr) -> bool {
    match &items.kind {
        js::ExprKind::Array(_) => true,
        // Of two made here, either, an `Option`'s items, `o == null ? [] : [o]`.
        js::ExprKind::Cond(_, then, otherwise) => fresh(then) && fresh(otherwise),
        js::ExprKind::Call(callee, _) => match &callee.kind {
            js::ExprKind::Member(_, name) => [
                "map",
                "filter",
                "slice",
                "toReversed",
                "split",
                "from",
                "flatMap",
                "flat",
                "concat",
                // What it takes out, a heap's `drain()`'s.
                "splice",
            ]
            .contains(&name.as_str()),
            js::ExprKind::Var(name) => [
                "$charIndices",
                "$range",
                "$zip",
                "$windows",
                "$chunks",
                "$chunksExact",
                "$rchunks",
                "$rchunksMut",
                "$splitBy",
                "$lines",
                "$slice",
                "$rest",
                "$drain",
                "$splitOff",
                "$setAlgebra",
                "$sortedEntries",
                "$sortedKeys",
                "$args",
                "$vars",
                "$pathParts",
                "$pathAncestors",
                "$pathComponents",
            ]
            .contains(&name.as_str()),
            _ => false,
        },
        _ => false,
    }
}

/// Each chain in `stmts` that MIR made as Rust runs it (ADR 0364), item by
/// item, `Array.from($iter(xs).map(f))`, written as an array's, `xs.map(f)`
/// (ADR 0055), where the order its closures run in can't be told from
/// Rust's by what takes it. One that's lent, `$lent(…).some(f)`, made for
/// this call alone, no one else steps.
pub(in crate::lower) fn eager_chains(stmts: &mut [Stmt]) {
    js::each_expr_mut(stmts, &mut |e| {
        // `nth(n)`'s, `Array.from(chain.drop(n).take(1))[0]`: of an array
        // whose stages do nothing, `xs[n]`.
        if let js::ExprKind::Index(first, zero) = &e.kind
            && zero.as_int() == Some(0)
            && let Some((chain, n)) = nth_of(first)
            && let Some((items, _)) = eager_items(chain, false, None)
        {
            *e = Expr {
                span: e.span,
                ..Expr::index(items, n.clone())
            };
            return;
        }
        // `collect()`'s, `Array.from(chain)`: of a chain of chains, their
        // items spread, `[...a, ...b]`; of an array whose stages do nothing,
        // a new one, a stage's already, the source copied unless it's one
        // made here.
        if let Some(chain) = e.collected_of() {
            // Of a runtime helper's, an array or an iterable, `Iterator.from`
            // makes nothing `Array.from` doesn't: `Array.from($rsplit(s,
            // "="))`. Not of what may be a binding's iterator, which may
            // have only a `next` (ADR 0140).
            if is_iterator_from(chain)
                && let js::ExprKind::Call(_, given) = &chain.kind
                && let [items] = &given[..]
                && let js::ExprKind::Call(helper, _) = &items.kind
                && helper.helper_name().is_some()
            {
                *e = Expr {
                    span: e.span,
                    ..Expr::collected(items.clone())
                };
                return;
            }
            let eager = match chained(chain) {
                Some(parts) => Some(Expr::array(parts.into_iter().map(Expr::spread).collect())),
                None => match eager_items(chain, true, None) {
                    Some((items, 0)) if !fresh(&items) => Some(Expr::call(Expr::member(items, "slice"), vec![])),
                    Some((items, _)) => Some(items),
                    None => None,
                },
            };
            if let Some(eager) = eager {
                *e = Expr { span: e.span, ..eager };
            }
            return;
        }
        let js::ExprKind::Call(callee, args) = &e.kind else {
            return;
        };
        // `next()`'s, `$next(chain)`, of an iterator made here of an array
        // whose stages do nothing: its first item, `xs[0]`.
        if callee.helper_name() == Some("$next")
            && let [chain] = &args[..]
            && let Some((items, _)) = eager_items(chain, false, None)
        {
            *e = Expr {
                span: e.span,
                ..Expr::index(items, Expr::int(0))
            };
            return;
        }
        // What reads every item, as each comes, `$max`'s: of the array
        // itself, `$max(xs)`; `$maxBy`'s compares as each comes. Given its
        // items' drop, an array is the helper's own, which it closes to drop
        // what's left where a panic stops it (#423): of an owning `$iter`
        // closed by that drop, the array, closed as the helper closes it;
        // of a chain of one closed where nothing may throw, its items; of
        // one not closed, which it doesn't own, none.
        if let Some(name) = callee.helper_name()
            && steps_all(name)
            && let Some(chain) = args.first()
            && let Some(items) = match item_drop_given(name, args) {
                Some(drop) => match closed_by_its_drop(chain, drop) {
                    Some(items) => Some(items.clone()),
                    None => eager_items_of(chain, true, args.get(1), true).map(|(items, _)| items),
                },
                None => eager_items(chain, true, args.get(1)).map(|(items, _)| items),
            }
        {
            let mut args = args.clone();
            args[0] = items;
            *e = Expr {
                span: e.span,
                ..Expr::call((**callee).clone(), args)
            };
            return;
        }
        // `$position`'s, of the array, lent or not: what it steps has no
        // `return` to close.
        if callee.helper_name() == Some("$position")
            && let [chain, f] = &args[..]
            && let Some((items, _)) = eager_items(chain, false, None)
        {
            *e = Expr {
                span: e.span,
                ..Expr::call((**callee).clone(), vec![items, f.clone()])
            };
            return;
        }
        let js::ExprKind::Member(chain, name) = &callee.kind else {
            return;
        };
        let eager = match (name.as_str(), &args[..]) {
            // A `map(g)` whose items `forEach` does nothing with,
            // `collect::<()>()`'s: `forEach(g)`, each `g`'s value dropped as
            // the items were.
            ("forEach", [f])
                if matches!(&f.kind, js::ExprKind::Arrow(params, body) if params.is_empty() && body.is_empty())
                    && let js::ExprKind::Call(stage, given) = &chain.kind
                    && let js::ExprKind::Member(source, map) = &stage.kind
                    && map == "map"
                    && let [g] = &given[..] =>
            {
                Some(Expr::call(Expr::member((**source).clone(), "forEach"), vec![g.clone()]))
            }
            ("forEach", [f]) => eager_items(chain, true, Some(f)).map(|(items, _)| match set_of(&items, f) {
                // A set's items each given alone: the set's own `forEach`,
                // `set.forEach((x) => ..)`, as JS writes it (ADR 0877).
                Some(set) => Expr::call(Expr::member(set, "forEach"), vec![f.clone()]),
                None => array_method(items, "forEach", f.clone()),
            }),
            // `count()`'s, `reduce((n) => n + 1, 0)`: the array's `length`.
            ("reduce", [f, init]) if init.as_int() == Some(0) && counts(f) => {
                eager_items(chain, true, None).map(|(items, _)| Expr::member(items, "length"))
            }
            // Its start is made before any item is stepped, as Rust's `fold`
            // is given it: one that does something, before no stage that
            // does, which an array's would run first.
            ("reduce", [f, init]) => match init.has_effects() {
                true => eager_items(chain, false, None),
                false => eager_items(chain, true, Some(f)),
            }
            .map(|(items, _)| {
                let (items, f) = indexed_callback(&items, "reduce", f.clone()).unwrap_or_else(|f| (items, f));
                Expr::call(Expr::member(items, "reduce"), vec![f, init.clone()])
            }),
            // Whether any item is a literal, which `===` and `includes` agree
            // of whatever the items are: `includes`.
            ("some", [f])
                if let Some(value) = sought(f).filter(|v| {
                    matches!(&v.kind, js::ExprKind::Str(_) | js::ExprKind::Bool(_)) || v.as_int().is_some()
                }) =>
            {
                eager_items(chain, false, None)
                    .map(|(items, _)| Expr::call(Expr::member(items, "includes"), vec![value]))
            }
            ("some" | "every", [f]) => {
                eager_items(chain, false, None).map(|(items, _)| array_method(items, name, f.clone()))
            }
            ("find", [f]) => eager_items(chain, false, None)
                .map(|(items, _)| Expr::call(Expr::member(items, "find"), vec![f.clone()])),
            _ => None,
        };
        if let Some(eager) = eager {
            *e = Expr { span: e.span, ..eager };
        }
    });
}

/// The set `items` are of, `set` of `Array.from(set)`, a `HashSet`'s
/// items (`Known::SetItems`), where `f`, what each is given to, reads the
/// item alone: an arrow of one parameter, or none, which a set's `forEach`
/// gives the item first, as an array's does, then the item again and the
/// set, where an array's gives its index and itself (ADR 0877).
fn set_of(items: &Expr, f: &Expr) -> Option<Expr> {
    let js::ExprKind::Call(_, args) = &items.kind else {
        return None;
    };
    match (&args[..], &f.kind) {
        ([set], js::ExprKind::Arrow(params, _)) if items.facts.has(js::Known::SetItems) && params.len() <= 1 => {
            Some(set.clone())
        }
        _ => None,
    }
}

/// A chain MIR makes lazily for the next statement's array alone to
/// spread, `const newLogs = $iter(xs).filter(f).map(g); const all =
/// [...a, ...newLogs];`, of stages that do nothing else: an array,
/// `xs.filter(f).map(g)`, which spreads alike.
pub(in crate::lower) fn eager_spreads(stmts: &mut [Stmt]) {
    for i in 0..stmts.len().saturating_sub(1) {
        let (head, rest) = stmts.split_at_mut(i + 1);
        let StmtKind::Const(name, value) = &mut head[i].kind else {
            continue;
        };
        let spread = |e: &Expr| {
            matches!(&e.kind, js::ExprKind::Array(items)
                if items.iter().any(|item| matches!(&item.kind, js::ExprKind::Spread(of)
                    if matches!(&of.kind, js::ExprKind::Var(v) if v == name))))
        };
        let next = match &rest[0].kind {
            StmtKind::Const(_, e) | StmtKind::Let(_, Some(e)) | StmtKind::Return(Some(e)) => spread(e),
            _ => false,
        };
        if !next || js::mentions_in(rest, name) != 1 {
            continue;
        }
        if let Some((items, stages)) = eager_items(value, false, None)
            && stages > 0
        {
            *value = items;
        }
    }
}

/// Of `$closing($iter(xs), $dropEach(drop))`, an owning `$iter` closed by
/// `drop`, as a helper given `drop` closes an array it's given (#423): `xs`.
fn closed_by_its_drop<'e>(chain: &'e Expr, drop: &Expr) -> Option<&'e Expr> {
    let js::ExprKind::Call(closing, args) = &chain.kind else {
        return None;
    };
    let [it, each] = &args[..] else { return None };
    let js::ExprKind::Call(drop_each, dropped) = &each.kind else {
        return None;
    };
    let by_drop = closing.helper_name() == Some("$closing")
        && drop_each.helper_name() == Some("$dropEach")
        && matches!(&dropped[..], [d] if d.kind == drop.kind);
    by_drop.then(|| fresh_iter_source(it)).flatten()
}

/// A chain MIR makes as Rust runs it (ADR 0364), item by item, `$iter(xs)
/// .map(f).filter(g)`: an array made of `xs`, `xs.map(f).filter(g)`
/// (ADR 0055), and how many stages it has, where the order its closures run
/// in can't be told from Rust's. `whole`: what takes it runs over every
/// item, as `collect` does, and one closure may do something, `taker`, its
/// callback, which runs as each item comes, among them; else it may stop
/// early, and no stage may.
fn eager_items(chain: &Expr, whole: bool, taker: Option<&Expr>) -> Option<(Expr, usize)> {
    eager_items_of(chain, whole, taker, false)
}

/// `eager_items`', given to what `owns` the array it's made, a helper given
/// its items' drop, which closes it: only of a source that owns its items,
/// closed, never one it borrows (#423).
fn eager_items_of(chain: &Expr, whole: bool, taker: Option<&Expr>, owns: bool) -> Option<(Expr, usize)> {
    let mut stages = Vec::new();
    let mut at = unlent(chain);
    while let js::ExprKind::Call(callee, args) = &at.kind
        && let js::ExprKind::Member(source, name) = &callee.kind
        && matches!(name.as_str(), "map" | "filter" | "take" | "drop" | "flatMap")
        && let [f] = &args[..]
    {
        stages.push((name.as_str(), f));
        at = source;
    }
    // A count's, `take`'s or `drop`'s, the last first: what's before it,
    // which runs over the items it stops at or passes over only in Rust,
    // does nothing.
    let counted = |name: &str| matches!(name, "take" | "drop");
    if let Some(last) = stages.iter().position(|(name, _)| counted(name))
        && stages[last + 1..].iter().any(|(name, f)| !counted(name) && does(f))
    {
        return None;
    }
    let at = unlent(at);
    // An owning source closed to drop what it hasn't given, `$closing($iter(xs),
    // drop)`, which a chain of `map`s and `filter`s run over every item never
    // closes (ADR 0465): its items.
    let at = match &at.kind {
        js::ExprKind::Call(callee, args)
            if callee.helper_name() == Some("$closing")
                && whole
                && stages.iter().all(|(name, f)| {
                    // Not one a panic stops early, which drops what's left
                    // as it closes (#422), its stages' or what takes them.
                    matches!(*name, "map" | "filter") && !crate::lower::iterators::in_place::may_throw(f)
                })
                && taker.is_none_or(|f| !crate::lower::iterators::in_place::may_throw(f)) =>
        {
            &args[0]
        }
        _ if owns => return None,
        _ => at,
    };
    let js::ExprKind::Call(_, args) = &at.kind else {
        return None;
    };
    // Of an array, `$iter(xs)`; or two side by side, `$lazyZip` of each's,
    // `$zip(a, b)`, whose items pairing them make nothing else do.
    let source = match (&args[..], &at.kind) {
        ([source], _) if is_stepped_items(at) => source.clone(),
        (_, js::ExprKind::Call(zip, sides))
            if zip.helper_name() == Some("$lazyZip")
                && let [a, b] = &sides[..]
                && is_stepped_items(a)
                && is_stepped_items(b) =>
        {
            Expr::call(Expr::helper("$zip"), vec![iterated(a.clone()), iterated(b.clone())])
        }
        _ => return None,
    };
    // Each item's items an array too, all of them, in turn, as Rust's
    // `flat_map` steps each before the next: `xs.flatMap((x) => x.map(g))`.
    let mut flattened = Vec::new();
    for (name, f) in &stages {
        if *name == "flatMap" {
            let js::ExprKind::Arrow(params, body) = &f.kind else {
                return None;
            };
            let [
                Stmt {
                    kind: StmtKind::Return(Some(inner)),
                    span,
                },
            ] = body.as_slice()
            else {
                return None;
            };
            let items = match is_stepped_items(inner) {
                true => iterated(inner.clone()),
                false => eager_items(inner, true, None)?.0,
            };
            let body = vec![StmtKind::Return(Some(items)).at(*span)];
            flattened.push(Expr::arrow(params.clone(), body));
        }
    }
    let mut flattened = flattened.into_iter().rev();
    let doing = stages.iter().filter(|(name, f)| !counted(name) && does(f)).count();
    let quiet = match whole {
        true => doing + usize::from(taker.is_some_and(does)) <= 1,
        false => doing == 0,
    };
    if !quiet {
        return None;
    }
    let count = stages.len();
    let items = (stages.into_iter().rev()).fold(source, |items, (name, f)| match name {
        "take" => Expr::call(Expr::member(items, "slice"), vec![Expr::int(0), f.clone()]),
        "drop" => Expr::call(Expr::member(items, "slice"), vec![f.clone()]),
        // Each item an array, as it is: `flat()`, which takes arrays apart.
        "flatMap" => match flattened.next().expect("made") {
            f if f.is_identity() => Expr::call(Expr::member(items, "flat"), Vec::new()),
            f => array_method(items, name, f),
        },
        name => array_method(items, name, f.clone()),
    });
    Some((items, count))
}

/// Of `Iterator.from(list)` collected, `chain`, each item mapped or filtered
/// on the way: the array it makes, as a page writes it,
/// `Array.from(nodes).map(f)`. Not of the runtime's, `$lazyChain(a, b)`'s,
/// read as their parts.
pub(in crate::lower) fn collected(chain: &Expr) -> Option<Expr> {
    fn sourced(e: &Expr) -> Option<Expr> {
        match &e.kind {
            js::ExprKind::Call(callee, args)
                if let js::ExprKind::Member(source, name) = &callee.kind
                    && matches!(name.as_str(), "map" | "filter" | "take" | "drop" | "flatMap") =>
            {
                Some(Expr::call(Expr::member(sourced(source)?, name), args.clone()))
            }
            _ if is_iterator_from(e) => {
                let list = iterated(e.clone());
                // Not one a runtime helper makes: `$closing(..)`'s, listed,
                // would be stepped to its end and never closed (ADR 0510).
                if matches!(&list.kind, js::ExprKind::Call(f, _) if matches!(&f.kind, js::ExprKind::Var(f) if f.starts_with('$')))
                {
                    return None;
                }
                let items = Expr::call(Expr::member(Expr::var("Array"), "from"), vec![list]);
                Some(array_stepped(items))
            }
            _ => None,
        }
    }
    eager_items(&sourced(chain)?, true, None).map(|(items, _)| items)
}

/// Of `$lazyChain(a, b)`, a `chain` MIR makes: what each part
/// steps, `a` and `b`, which spread in turn as it steps them, `[...a, ...b]`;
/// of a part `Iterator.from(xs)`, or `cloned()`'s `xs.map((x) => x)`, `xs`.
fn chained(e: &Expr) -> Option<Vec<Expr>> {
    fn part(e: &Expr) -> Vec<Expr> {
        match &e.kind {
            js::ExprKind::Call(callee, args) => match (&callee.kind, args.as_slice()) {
                (_, [a, b]) if callee.helper_name() == Some("$lazyChain") => [part(a), part(b)].concat(),
                (js::ExprKind::Member(of, m), [f]) if m == "map" && f.is_identity() => part(of),
                _ if is_stepped_items(e) || is_iterator_from(e) => part(&args[0]),
                _ => vec![e.clone()],
            },
            _ => vec![e.clone()],
        }
    }
    let js::ExprKind::Call(callee, _) = &e.kind else {
        return None;
    };
    (callee.helper_name() == Some("$lazyChain")).then(|| part(e))
}

/// Whether `e` is `Iterator.from(x)`.
fn is_iterator_from(e: &Expr) -> bool {
    matches!(&e.kind, js::ExprKind::Call(callee, args) if args.len() == 1
        && matches!(&callee.kind, js::ExprKind::Member(object, from)
            if from == "from" && matches!(&object.kind, js::ExprKind::Var(v) if v == "Iterator")))
}

/// `items.into_iter()` of a generic `IntoIterator`, an array or a JS
/// iterator as its caller has it (ADR 0061): `$intoIter(items)`, of an
/// array the `$iter` concrete code has, which knows where it is; of one a
/// JS iterator already, itself (ADR 0475).
pub(in crate::lower) fn given_into_iter(items: Expr) -> Expr {
    match is_js_iterator(&items) {
        true => items,
        false => Expr::call_helper("$intoIter", vec![items]),
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What `Iterator::fuse`'s compiled body is called in this module, where
    /// the module imports it: `consumed_into_iters`'s `fuse`.
    pub(in crate::lower) fn fuse_import(&self) -> Option<String> {
        crate::lower::recognition::compiled_provided_method(self.tcx, "Iterator", "fuse")
            .and_then(|id| self.krate.foreign.import_key(id, self.module))
            .and_then(|export| self.krate.imports.get(&self.module)?.get(&export))
            .cloned()
    }
}

/// Of an `$intoIter(xs)` or a `fuse()` stepped in place by what steps it
/// till it ends, `$intoIter(xs).map(f)` or `for (const n of $intoIter(xs))`,
/// as each JS consumer and helper does: what steps the same,
/// `stepped_in_place`'s (ADRs 0475, 0561). Each statement's: a `for`'s
/// iterable, and each expression's. `fuse` is what `Iterator::fuse`'s
/// compiled body is called here, where it's imported.
pub(in crate::lower) fn consumed_into_iters(body: &mut Vec<Stmt>, fuse: Option<&str>) {
    js::statement_lists(body, &mut |stmts| {
        for stmt in stmts.iter_mut() {
            if let StmtKind::ForOf {
                iterable, keys: false, ..
            } = &mut stmt.kind
            {
                stepped_in_place(iterable, true, fuse);
            }
        }
    });
    js::each_expr_mut(body, &mut |e| {
        let js::ExprKind::Call(callee, args) = &mut e.kind else {
            return;
        };
        // `collect()`'s, `Array.from(it)`, steps it till its first `done`.
        if let js::ExprKind::Member(array, from) = &callee.kind
            && from == "from"
            && matches!(&array.kind, js::ExprKind::Var(v) if v == "Array")
            && let [it] = &mut args[..]
        {
            stepped_in_place(it, false, fuse);
        } else if let js::ExprKind::Member(object, method) = &mut callee.kind
            && STEPPED_TILL_DONE.contains(&method.as_str())
        {
            // The crate's iterator a `map` steps keeps its `size_hint`,
            // which the `map`'s is, as std's `Map` passes it on.
            if method == "map" && crate::lower::iterators::stepped_value(object).is_some() {
                return;
            }
            stepped_in_place(object, false, fuse);
        } else if callee.helper_name().is_some_and(steps_all)
            && let Some(first) = args.first_mut()
        {
            stepped_in_place(first, false, fuse);
        }
    });
}
