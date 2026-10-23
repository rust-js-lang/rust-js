//! Tidy's values: arrays, handles, callbacks and cells, as JS writes them.

use super::*;

/// `$iter(xs).filter(f)`, an array's `$iter`, which knows where it is, made
/// for an iterator helper alone, which nothing else steps: `xs.values()
/// .filter(f)`, the array's own iterator.
pub(super) fn array_values(e: &mut Expr) {
    // Each array of a lazy `flatMap`, `(item) => $iter(item)`, left lazy:
    // the array, which an iterator's `flatMap` steps the same.
    if let ExprKind::Call(callee, args) = &mut e.kind
        && matches!(&callee.kind, ExprKind::Member(_, m) if m == "flatMap")
        && let [f] = args.as_mut_slice()
        && let ExprKind::Arrow(params, body) = &mut f.kind
        && let [js::Pattern::Name(param)] = params.as_slice()
        && let [
            Stmt {
                kind: StmtKind::Return(Some(each)),
                ..
            },
        ] = body.as_mut_slice()
        && crate::lower::iterators::is_stepped_items(each)
        && matches!(&iterated(each.clone()).kind, ExprKind::Var(v) if v == param)
    {
        *each = Expr::var(param);
    }
    // A set or a map made of an array's `$iter`, `new Set($iter(xs))`: of
    // the array, whose items it steps the same.
    if let ExprKind::New(class, args) = &mut e.kind
        && matches!(&class.kind, ExprKind::Var(c) if c == "Set" || c == "Map")
        && let [items] = args.as_mut_slice()
        && crate::lower::iterators::is_stepped_items(items)
    {
        *items = iterated(items.clone());
        return;
    }
    // The rest of a string's `chars()` not stepped, `$restStr($iter(cs))`:
    // all of them, `cs.join("")`, which may be a `char`'s case
    // mapping's, below.
    if let ExprKind::Call(callee, args) = &e.kind
        && callee.helper_name() == Some("$restStr")
        && let [it] = &args[..]
        && crate::lower::iterators::is_stepped_items(it)
        && let ExprKind::Call(_, items) = &it.kind
        && let [items] = &items[..]
    {
        *e = Expr {
            span: e.span,
            ..Expr::call(Expr::member(items.clone(), "join"), vec![Expr::str("")])
        };
    }
    // A string's characters joined, `Array.from(c.toUpperCase()).join("")`,
    // a `char`'s capital collected: the string.
    if let ExprKind::Call(callee, args) = &e.kind
        && let ExprKind::Member(chars, join) = &callee.kind
        && join == "join"
        && matches!(&args[..], [Expr { kind: ExprKind::Str(sep), .. }] if sep.is_empty())
        && let ExprKind::Call(from, of) = &chars.kind
        && matches!(&from.kind, ExprKind::Member(array, f) if f == "from" && matches!(&array.kind, ExprKind::Var(a) if a == "Array"))
        && let [text] = &of[..]
        && matches!(&text.kind, ExprKind::Call(case, _)
            if matches!(&case.kind, ExprKind::Member(_, m) if m == "toUpperCase" || m == "toLowerCase"))
    {
        *e = Expr {
            span: e.span,
            ..text.clone()
        };
        return;
    }
    // Each item with its index, `enumerate()`'s `.map((x, i) => [i, x])`:
    // the array's `entries()`.
    if let ExprKind::Call(callee, args) = &e.kind
        && let ExprKind::Member(it, name) = &callee.kind
        && name == "map"
        && let [pair] = &args[..]
        && pairs_index(pair)
        && crate::lower::iterators::is_stepped_items(it)
        && let ExprKind::Call(_, items) = &it.kind
        && let [items] = &items[..]
    {
        *e = Expr {
            span: e.span,
            ..Expr::call(Expr::member(items.clone(), "entries"), Vec::new())
        };
        return;
    }
    if let ExprKind::Call(callee, _) = &mut e.kind
        && let ExprKind::Member(it, name) = &mut callee.kind
        && matches!(
            name.as_str(),
            "map" | "filter" | "flatMap" | "forEach" | "some" | "every" | "find" | "take" | "drop" | "reduce"
        )
        && crate::lower::iterators::is_stepped_items(it)
        && let ExprKind::Call(_, args) = &mut it.kind
        && args.len() == 1
    {
        let items = args.pop().expect("one");
        **it = Expr::call(Expr::member(items, "values"), Vec::new());
    }
}

/// Whether `f` is `(x, i) => [i, x]`, an item paired with its index.
pub(super) fn pairs_index(f: &Expr) -> bool {
    let var = |e: &Expr| match &e.kind {
        ExprKind::Var(v) => Some(v.clone()),
        _ => None,
    };
    if let ExprKind::Arrow(params, body) = &f.kind
        && let [x, i] = &params[..]
        && let [
            Stmt {
                kind: StmtKind::Return(Some(pair)),
                ..
            },
        ] = &body[..]
        && let ExprKind::Array(items) = &pair.kind
        && let [first, second] = &items[..]
    {
        return x.name().is_some_and(|x| var(second).as_deref() == Some(x))
            && i.name().is_some_and(|i| var(first).as_deref() == Some(i));
    }
    false
}

/// `isItem()` of `isItem(item)`, `globalThis.isItem()`, what a call calls, as a key.
pub(in crate::lower::mir) fn call_path(e: &Expr) -> Option<String> {
    let ExprKind::Call(callee, _) = &e.kind else {
        return None;
    };
    let callee = match &callee.kind {
        ExprKind::Var(name) => name.clone(),
        _ => field_path(callee)?,
    };
    Some(format!("{callee}()"))
}

/// `element.editable`, a field's path of a variable, as a key.
pub(in crate::lower::mir) fn field_path(e: &Expr) -> Option<String> {
    match &e.kind {
        ExprKind::Member(of, field) => {
            let of = match &of.kind {
                ExprKind::Var(name) => name.clone(),
                _ => field_path(of)?,
            };
            Some(format!("{of}.{field}"))
        }
        _ => None,
    }
}

/// `((e) => e.data)(r._0)`, an arrow called where it's made: what
/// `calls::apply` makes of it, its value, where it can.
pub(super) fn applied_arrows(e: &mut Expr) {
    if let ExprKind::Call(callee, args) = &e.kind
        && matches!(callee.kind, ExprKind::Arrow(..))
    {
        let applied = crate::lower::calls::apply((**callee).clone(), args.clone());
        if !matches!(&applied.kind, ExprKind::Call(f, _) if matches!(f.kind, ExprKind::Arrow(..))) {
            *e = Expr {
                span: e.span,
                ..applied
            };
        }
    }
}

/// `xs.map(f)` of `xs.map((x, i) => f(x, i))`, a closure MIR says passes on
/// what it's given to a function of Rust's, which takes no more: the index
/// and the array the method gives it too change nothing (ADR 0279).
pub(super) fn passed_on(e: &mut Expr, forwarding: &[js::Span]) {
    let ExprKind::Call(callee, args) = &mut e.kind else {
        return;
    };
    let ExprKind::Member(_, name) = &callee.kind else {
        return;
    };
    let [f] = &mut args[..] else { return };
    if ["map", "filter", "some", "every", "find", "forEach"].contains(&name.as_str())
        && forwarding.contains(&f.span)
        && let Some(g) = iterators::passed_on(f, false)
    {
        *f = g;
    }
}

/// `$index(xs, i); xs[i] = v;`, an element's place MIR checks, then sets:
/// `xs[$at(xs, i)] = v;`, the place `$at` checks, with the message `$index`
/// was given. `v` is made before
/// either, as Rust makes it.
pub(super) fn checked_sets(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        if let StmtKind::Expr(check) = &stmts[i].kind
            && let ExprKind::Call(f, args) = &check.kind
            && f.helper_name() == Some("$index")
            && let [items, index, failure @ ..] = &args[..]
            && let StmtKind::Assign(target, _) = &stmts[i + 1].kind
            && let ExprKind::Index(of, at) = &target.kind
            && js::same_path(of, items)
            && (js::same_path(at, index) || at.as_int().is_some() && at.as_int() == index.as_int())
        {
            // A `VecDeque`'s message, `failure`, with it.
            let at = [vec![items.clone(), index.clone()], failure.to_vec()].concat();
            let checked = Expr::index(items.clone(), Expr::call_helper("$at", at));
            if let StmtKind::Assign(target, _) = &mut stmts[i + 1].kind {
                *target = Expr {
                    span: target.span,
                    ..checked
                };
            }
            stmts.remove(i);
            continue;
        }
        i += 1;
    }
}

/// `{ ...a, ...{ ...b } }` or `<X {...{ ...b }} />`, a spread of an object of
/// spreads alone: those spreads, `{ ...a, ...b }`, which give what it gives,
/// each read where it was; and an array's spread of an array, its items.
pub(super) fn spread_spreads(e: &mut Expr) {
    if let ExprKind::Array(items) = &mut e.kind {
        *items = (std::mem::take(items).into_iter())
            .flat_map(|item| match item {
                Expr {
                    kind: ExprKind::Spread(inner),
                    ..
                } if matches!(inner.kind, ExprKind::Array(_)) => {
                    let ExprKind::Array(inner) = inner.kind else {
                        unreachable!("matched")
                    };
                    inner
                }
                item => vec![item],
            })
            .collect();
        return;
    }
    let props = match &mut e.kind {
        ExprKind::Object(props) => props,
        ExprKind::Jsx(jsx) => &mut jsx.props,
        _ => return,
    };
    // Down to what's spread, `{...{ ...{ ...b } }}`'s `b`, a copy made of a
    // copy, as an element captured twice spills its props.
    let flat = |prop: &js::Prop| {
        matches!(prop, js::Prop::Spread(Expr { kind: ExprKind::Object(inner), .. })
            if inner.iter().all(|p| matches!(p, js::Prop::Spread(_))))
    };
    while props.iter().any(flat) {
        *props = (std::mem::take(props).into_iter())
            .flat_map(|prop| match prop {
                js::Prop::Spread(Expr {
                    kind: ExprKind::Object(inner),
                    ..
                }) if inner.iter().all(|p| matches!(p, js::Prop::Spread(_))) => inner,
                prop => vec![prop],
            })
            .collect();
    }
}

/// A map of an array's items nothing reads, `next_back` having stepped its
/// source instead: no statement, as making it does nothing.
pub(super) fn unread_maps(body: &mut Vec<Stmt>) {
    let mut read = HashSet::new();
    js::visit_stmts(body, &mut |name| {
        read.insert(name.to_string());
    });
    js::statement_lists(body, &mut |stmts| {
        stmts.retain(
            |s| !matches!(&s.kind, StmtKind::Const(name, value) if !read.contains(name) && iterators::is_items_map(value)),
        );
    });
}

/// `t = [undefined, f()]`, a list MIR held while it took its other items
/// out, `format_args!`'s of a value it read first: of one item left, read
/// only as `t[1]`, the item, `t = f()`, read as `t` (ADR 0512).
pub(super) fn holed_lists(body: &mut Vec<Stmt>) {
    // Each list with holes, by its name, its one item's index, and whether
    // it's assigned, not declared with it.
    let mut lists: Vec<(String, i128, bool)> = Vec::new();
    js::statement_lists(&mut body.clone(), &mut |stmts| {
        for stmt in stmts.iter() {
            let (name, value, assigned) = match &stmt.kind {
                StmtKind::Const(name, value) | StmtKind::Let(name, Some(value)) => (name.clone(), value, false),
                StmtKind::Assign(
                    Expr {
                        kind: ExprKind::Var(name),
                        ..
                    },
                    value,
                ) => (name.clone(), value, true),
                _ => continue,
            };
            let ExprKind::Array(items) = &value.kind else { continue };
            let left: Vec<usize> = (0..items.len())
                .filter(|&i| !matches!(items[i].kind, ExprKind::Undefined))
                .collect();
            if let [at] = left.as_slice()
                && items.len() > 1
            {
                lists.push((name, *at as i128, assigned));
            }
        }
    });
    for (name, at, assigned) in lists {
        let read = |e: &Expr| {
            matches!(&e.kind, ExprKind::Index(of, index)
                if matches!(&of.kind, ExprKind::Var(v) if *v == name) && index.as_int() == Some(at))
        };
        // Each read of it one of its item's, and it made once.
        let mut rest = body.clone();
        js::each_expr_mut(&mut rest, &mut |e| {
            if read(e) {
                *e = Expr::undefined();
            }
        });
        if js::mentions_in(&rest, &name) != usize::from(assigned)
            || js::mentions_in(body, &name) == usize::from(assigned)
        {
            continue;
        }
        js::statement_lists(body, &mut |stmts| {
            for stmt in stmts.iter_mut() {
                let value = match &mut stmt.kind {
                    StmtKind::Const(n, value) | StmtKind::Let(n, Some(value)) if *n == name => value,
                    StmtKind::Assign(
                        Expr {
                            kind: ExprKind::Var(n), ..
                        },
                        value,
                    ) if *n == name => value,
                    _ => continue,
                };
                if let ExprKind::Array(items) = &mut value.kind {
                    *value = std::mem::replace(&mut items[at as usize], Expr::undefined());
                }
            }
        });
        js::each_expr_mut(body, &mut |e| {
            if read(e) {
                *e = Expr {
                    span: e.span,
                    ..Expr::var(&name)
                };
            }
        });
    }
}

/// A handle made only to read or write its value there, `Cell::set` of
/// `&self.n`'s `({ get value() { .. }, set value(value) { .. } }).value =
/// v`: its place, `t.n = v`. Of a place that reads the same
/// alone: of one that computes, `a[f()]`, the setter would compute it after
/// `v`, where the place itself is computed before.
pub(super) fn handles_in_place(body: &mut [Stmt]) {
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Member(of, key) = &e.kind
            && key == js::HELD
            && let ExprKind::Handle(place) = &of.kind
            && place.reads_same()
        {
            *e = Expr {
                span: e.span,
                ..(**place).clone()
            };
        }
    });
}

/// `const h = { get value() { return o.f; }, .. }`, a `&Cell`'s handle on a
/// field, whose `value` alone is used, the handle never given: read before
/// anything could set `o.f`, `const h = o.f`, `h` where its `value` is
/// read (ADR 0287); else `o.f` where its `value` is, the `o` it was made
/// of, as MIR holds a reference written again before it's made.
pub(super) fn read_handles(body: &mut Vec<Stmt>) {
    let mut handles: Vec<(String, Expr, bool)> = Vec::new();
    js::statement_lists(&mut body.clone(), &mut |stmts| {
        for (k, stmt) in stmts.iter().enumerate() {
            if let StmtKind::Const(name, value) = &stmt.kind
                && let ExprKind::Handle(place) = &value.kind
                && matches!(place.kind, ExprKind::Member(..))
            {
                handles.push((name.clone(), (**place).clone(), read_at_once(stmts, k, name)));
            }
        }
    });
    handles.retain(|(name, place, at_once)| {
        // Each use of it one of its `value`: none left once those are.
        let mut rest = body.clone();
        js::each_expr_mut(&mut rest, &mut |e| {
            if let ExprKind::Member(of, key) = &e.kind
                && key == js::HELD
                && matches!(&of.kind, ExprKind::Var(v) if v == name)
            {
                *e = Expr::undefined();
            }
        });
        let mut root = place;
        while let ExprKind::Member(of, _) = &root.kind {
            root = of;
        }
        js::mentions_in(&rest, name) == 0 && (*at_once || matches!(root.kind, ExprKind::Var(_)))
    });
    if handles.is_empty() {
        return;
    }
    let read = |name: &str| (handles.iter()).find(|(h, ..)| h == name);
    js::statement_lists(body, &mut |stmts| {
        stmts.retain(
            |stmt| !matches!(&stmt.kind, StmtKind::Const(name, _) if read(name).is_some_and(|(.., at_once)| !at_once)),
        );
        for stmt in stmts.iter_mut() {
            if let StmtKind::Const(name, value) = &mut stmt.kind
                && read(name).is_some()
                && let ExprKind::Handle(place) = &value.kind
            {
                *value = (**place).clone();
            }
        }
    });
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Member(of, key) = &e.kind
            && key == js::HELD
            && let ExprKind::Var(v) = &of.kind
            && let Some((_, place, at_once)) = read(v)
        {
            let read = match at_once {
                true => Expr::var(v),
                false => place.clone(),
            };
            *e = Expr { span: e.span, ..read };
        }
    });
}

/// Whether what `stmts[at]` names is last read before anything that could
/// set a field: no field set nor call made from it to there.
pub(super) fn read_at_once(stmts: &[Stmt], at: usize, name: &str) -> bool {
    let Some(last) = (at + 1..stmts.len())
        .rev()
        .find(|&k| js::mentions_in(&stmts[k..=k], name) > 0)
    else {
        return true;
    };
    let mut between = stmts[at + 1..=last].to_vec();
    let mut still = true;
    js::statement_lists(&mut between, &mut |stmts| {
        for stmt in stmts.iter() {
            if let StmtKind::Assign(target, _) = &stmt.kind {
                still &= matches!(target.kind, ExprKind::Var(_));
            }
        }
    });
    js::each_expr_mut(&mut between, &mut |e| still &= !e.has_effects());
    still
}

/// `const x = { value: undefined };`, a cell a pointer is taken to, declared
/// before its value (ADR 0407), and its first mention in the same block its
/// value set, `x.value = 1;`, of what doesn't read `x`: `const x = { value:
/// 1 };` there, nothing before it having it.
pub(super) fn first_cell_values(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        let StmtKind::Const(name, value) = &stmts[i].kind else {
            i += 1;
            continue;
        };
        let empty = matches!(&value.kind, ExprKind::Object(props)
            if matches!(props.as_slice(), [js::Prop::Field(key, held)] if key == js::HELD && matches!(held.kind, ExprKind::Undefined)));
        let name = name.clone();
        let first = (i + 1..stmts.len()).find(|&j| js::mentions_in(&stmts[j..=j], &name) > 0);
        let set = first.and_then(|j| match &stmts[j].kind {
            StmtKind::Assign(target, value)
                if matches!(&target.kind, ExprKind::Member(cell, key)
                    if key == js::HELD && matches!(&cell.kind, ExprKind::Var(v) if *v == name))
                    && !value.mentions_var(&name) =>
            {
                Some((j, value.clone()))
            }
            _ => None,
        });
        match (empty, set) {
            (true, Some((j, value))) => {
                let span = stmts[j].span;
                stmts[j] = StmtKind::Const(name, Expr::object(vec![js::Prop::Field(js::HELD.into(), value)])).at(span);
                stmts.remove(i);
            }
            _ => i += 1,
        }
    }
}

/// `const n = { value: x }`, a cell, and `const m = n`, its clones, whose
/// `value` alone is read and set, by those names: `let n = x`, `n` where
/// each reads its `value` (ADR 0287). Nothing else has it, so nothing else
/// sees it change; its closures, which share the `let`, too.
pub(super) fn plain_cells(body: &mut Vec<Stmt>) {
    let mut cells: Vec<String> = Vec::new();
    let mut clones: HashMap<String, String> = HashMap::new();
    js::each_block_mut(&mut body.clone(), &mut |stmts| {
        for stmt in stmts.iter() {
            match &stmt.kind {
                StmtKind::Const(name, value)
                    if matches!(&value.kind, ExprKind::Object(props)
                        if matches!(props.as_slice(), [js::Prop::Field(key, _)] if key == js::HELD)) =>
                {
                    cells.push(name.clone());
                }
                StmtKind::Const(name, value) if let ExprKind::Var(of) = &value.kind => {
                    clones.insert(name.clone(), of.clone());
                }
                _ => {}
            }
        }
    });
    // Not one a closure's parameter or another declaration shadows, whose
    // `value` isn't the cell's.
    let mut bound: HashMap<String, usize> = HashMap::new();
    js::each_block_mut(&mut body.clone(), &mut |stmts| {
        for stmt in stmts.iter() {
            if let StmtKind::Const(name, _) | StmtKind::Let(name, _) = &stmt.kind {
                *bound.entry(name.clone()).or_default() += 1;
            }
        }
    });
    js::each_expr_mut(&mut body.clone(), &mut |e| {
        if let ExprKind::Arrow(params, _) | ExprKind::AsyncArrow(params, _) = &e.kind {
            for name in params.iter().flat_map(|p| p.names()) {
                *bound.entry(name.to_string()).or_default() += 1;
            }
        }
    });
    cells.retain(|c| bound.get(c).copied().unwrap_or(0) <= 1);
    // Each clone, to its cell.
    let cell_of = |name: &str| {
        let mut name = name;
        for _ in 0..clones.len() + 1 {
            if cells.iter().any(|c| c == name) {
                return Some(name.to_string());
            }
            name = clones.get(name)?;
        }
        None
    };
    let named: HashMap<String, String> = (cells.iter().chain(clones.keys()))
        .filter_map(|n| cell_of(n).map(|c| (n.clone(), c)))
        .collect();
    if named.is_empty() {
        return;
    }
    // Each read of the names, and each read of a `value` of one, or one a
    // clone is made of: the same, or one is read otherwise.
    let mut reads: HashMap<String, usize> = HashMap::new();
    js::visit_stmts(body, &mut |v| {
        if let Some(cell) = named.get(v) {
            *reads.entry(cell.clone()).or_default() += 1;
        }
    });
    let mut kept: HashMap<String, usize> = HashMap::new();
    let mut each = body.clone();
    js::each_expr_mut(&mut each, &mut |e| {
        if let ExprKind::Member(of, key) = &e.kind
            && key == js::HELD
            && let ExprKind::Var(v) = &of.kind
            && let Some(cell) = named.get(v)
        {
            *kept.entry(cell.clone()).or_default() += 1;
        }
    });
    js::each_block_mut(&mut each, &mut |stmts| {
        for stmt in stmts.iter() {
            if let StmtKind::Const(name, value) = &stmt.kind
                && let ExprKind::Var(of) = &value.kind
                && named.contains_key(name)
                && let Some(cell) = named.get(of)
            {
                *kept.entry(cell.clone()).or_default() += 1;
            }
        }
    });
    let plain: HashSet<&String> = (cells.iter()).filter(|c| reads.get(*c) == kept.get(*c)).collect();
    if plain.is_empty() {
        return;
    }
    let of_plain = |v: &str| named.get(v).filter(|c| plain.contains(c)).cloned();
    js::each_block_mut(body, &mut |stmts| {
        stmts.retain(|stmt| {
            !matches!(&stmt.kind, StmtKind::Const(name, value)
            if matches!(value.kind, ExprKind::Var(_)) && !plain.contains(name) && of_plain(name).is_some())
        });
        for stmt in stmts.iter_mut() {
            if let StmtKind::Const(name, value) = &mut stmt.kind
                && plain.contains(name)
                && let ExprKind::Object(props) = &mut value.kind
                && let Some(js::Prop::Field(_, init)) = props.pop()
            {
                stmt.kind = StmtKind::Let(name.clone(), Some(init));
            }
        }
    });
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Member(of, key) = &e.kind
            && key == js::HELD
            && let ExprKind::Var(v) = &of.kind
            && let Some(cell) = of_plain(v)
        {
            *e = Expr {
                span: e.span,
                ..Expr::var(&cell)
            };
        }
    });
}

/// `const tick = () => { … }; useEffect(tick, [count]);`, a closure only a
/// hook's function is: written there, a function its `let` names, `function
/// tick() { … }`, unless it reads that name (ADR 0297).
pub(super) fn named_callbacks(stmts: &mut Vec<Stmt>, callbacks: &HashSet<String>) {
    let mut i = 0;
    while i < stmts.len() {
        if let StmtKind::Const(name, value) = &stmts[i].kind
            && callbacks.contains(name)
            && matches!(value.kind, ExprKind::Arrow(..) | ExprKind::AsyncArrow(..))
            && !value.mentions_var(name)
            // Nor one reading what it would be named, `step` of `step$1`
            // (ADR 0297).
            && name.split('$').next().is_none_or(|base| {
                let mut reads = false;
                value.visit_vars(&mut |v| reads |= v == base);
                !reads
            })
            && js::mentions_in(&stmts[i + 1..], name) == 1
        {
            let span = stmts[i].span;
            let StmtKind::Const(name, value) = stmts.remove(i).kind else {
                unreachable!("matched")
            };
            let (params, body, is_async) = match value.kind {
                ExprKind::Arrow(params, body) => (params, body, false),
                ExprKind::AsyncArrow(params, body) => (params, body, true),
                _ => unreachable!("matched"),
            };
            let function = js::Function {
                name: name.clone(),
                params,
                body,
                export: false,
                is_async,
                noinline: false,
                span: value.span,
                name_span: span,
            };
            let mut function = Some(function);
            js::each_expr_mut(&mut stmts[i..], &mut |e| {
                if matches!(&e.kind, ExprKind::Var(v) if *v == name)
                    && let Some(function) = function.take()
                {
                    *e = Expr {
                        reads: false,
                        kind: ExprKind::Function(Box::new(function)),
                        span: e.span,
                    };
                }
            });
            continue;
        }
        i += 1;
    }
}

/// A writer's body (ADR 0054), `let f = ""` first: each way through that
/// writes once and gives it back, `return s`; a first write the string's
/// start, `let f = s`; no `f` at all where nothing's left to write it.
pub(in crate::lower::mir) fn writer_body(out: &mut Vec<Stmt>, name: &str) {
    let Some(at) = out.iter().position(|s| {
        matches!(&s.kind, StmtKind::Let(n, Some(e)) if n == name && matches!(&e.kind, ExprKind::Str(t) if t.is_empty()))
    }) else {
        return;
    };
    let made = out.remove(at);
    let mut rest = out.split_off(at);
    returned_unwritten(&mut rest, name);
    if js::mentions_in(&rest, name) > 0 {
        match rest.first().map(|s| &s.kind) {
            Some(StmtKind::Assign(target, value))
                if matches!(&target.kind, ExprKind::Var(v) if v == name)
                    && let ExprKind::Binary(Op::Add, lhs, first) = &value.kind
                    && matches!(&lhs.kind, ExprKind::Var(v) if v == name) =>
            {
                let first = (**first).clone();
                let span = rest[0].span;
                rest[0] = StmtKind::Let(name.to_string(), Some(first)).at(span);
            }
            _ => rest.insert(0, made),
        }
    }
    out.extend(rest);
}

/// `((result) => body)(e)`, a function written to read a value more than
/// once, `{:?}` of a `Result` (`display::applied`), given what isn't a
/// variable: `e` held first, `const _$1 = e;`, and `body` in place of the
/// call, reading it, as a value read twice is held everywhere else, which
/// `readable_temporaries` then names. Of each statement, in the order it
/// evaluates what it's made of, while what it evaluates first may be read
/// after it, `read_past_calls`; not where a test decides whether it runs.
pub(super) fn applied_values(
    stmts: &mut Vec<Stmt>,
    set: &HashSet<String>,
    plain: &HashSet<String>,
    taken: &mut HashSet<String>,
) {
    let mut i = 0;
    while i < stmts.len() {
        let name = (1..)
            .map(|k| format!("_${k}"))
            .find(|n| !taken.contains(n))
            .expect("a free name");
        // What's evaluated first: an assignment's target, which `out += ..`
        // reads, then the value.
        let (root, mut before) = match &mut stmts[i].kind {
            StmtKind::Expr(e)
            | StmtKind::Const(_, e)
            | StmtKind::Let(_, Some(e))
            | StmtKind::Return(Some(e))
            | StmtKind::Throw(e) => (
                e,
                Before {
                    past: true,
                    reads: true,
                },
            ),
            StmtKind::Assign(target, e) | StmtKind::Compound(_, target, e) => {
                let before = Before {
                    past: read_past_calls(target, set, plain),
                    reads: target.only_reads(),
                };
                (e, before)
            }
            _ => {
                i += 1;
                continue;
            }
        };
        // Taken out, and the statement looked at again, for the next.
        if let Some(value) = first_applied(root, &mut before, &name, set, plain) {
            root.each_mut(&mut spliced_templates);
            let span = stmts[i].span;
            stmts.insert(i, StmtKind::Const(name.clone(), value).at(span));
            taken.insert(name);
        }
        i += 1;
    }
}

/// The first `((p) => body)(e)` in `e`, in the order it's evaluated, while
/// all evaluated before it, `before`, may be read after it: `e`, taken out
/// and held as `name`, and the call `body` of it.
fn first_applied(
    e: &mut Expr,
    before: &mut Before,
    name: &str,
    set: &HashSet<String>,
    plain: &HashSet<String>,
) -> Option<Expr> {
    if !before.past && !before.reads {
        return None;
    }
    if let ExprKind::Call(callee, args) = &e.kind
        && let ExprKind::Arrow(params, body) = &callee.kind
        && let [js::Pattern::Name(param)] = params.as_slice()
        && let [
            Stmt {
                kind: StmtKind::Return(Some(result)),
                ..
            },
        ] = body.as_slice()
        && let [value] = args.as_slice()
        && !value.reads_same()
        // Before what's evaluated before it: past what reads the same after
        // it, or, doing nothing itself, past what only reads.
        && (before.past || before.reads && !value.has_effects())
        && let Some(read) = result.substitute_in_callbacks(&|n: &str| (n == param).then(|| Expr::var(name)))
    {
        let value = value.clone();
        *e = Expr { span: e.span, ..read };
        return Some(value);
    }
    let found = match &mut e.kind {
        ExprKind::Template(_, values, _) | ExprKind::Array(values) => values
            .iter_mut()
            .find_map(|value| first_applied(value, before, name, set, plain)),
        ExprKind::Call(callee, args) | ExprKind::New(callee, args) => first_applied(callee, before, name, set, plain)
            .or_else(|| {
                args.iter_mut()
                    .find_map(|arg| first_applied(arg, before, name, set, plain))
            }),
        // A test decides whether what's after it runs: only it.
        ExprKind::Binary(Op::And | Op::Or | Op::Coalesce, a, _) | ExprKind::Cond(a, ..) => {
            first_applied(a, before, name, set, plain)
        }
        ExprKind::Binary(_, a, b) => {
            first_applied(a, before, name, set, plain).or_else(|| first_applied(b, before, name, set, plain))
        }
        ExprKind::Unary(_, a) | ExprKind::Stringed(a) => first_applied(a, before, name, set, plain),
        _ => None,
    };
    // What it is, evaluated whole before what's after it.
    if found.is_none() {
        before.past &= read_past_calls(e, set, plain);
        before.reads &= e.only_reads();
    }
    found
}

/// What a statement evaluates before a point in it: whether all of it reads
/// the same after a call, `read_past_calls`, and whether it only reads.
pub(super) struct Before {
    past: bool,
    reads: bool,
}

/// A template's value that's a template itself, a function's body put in
/// place of its call, `${`(${a}, ${b})`}`: its text and values in the
/// outer's, `(${a}, ${b})`.
fn spliced_templates(e: &mut Expr) {
    let ExprKind::Template(texts, values, false) = &mut e.kind else {
        return;
    };
    if !values.iter().any(|v| matches!(v.kind, ExprKind::Template(_, _, false))) {
        return;
    }
    let (mut spliced_texts, mut spliced_values) = (vec![texts[0].clone()], Vec::new());
    for (value, after) in std::mem::take(values).into_iter().zip(texts.iter().skip(1)) {
        match value.kind {
            ExprKind::Template(inner_texts, inner_values, false) => {
                let mut inner = inner_texts.into_iter();
                spliced_texts
                    .last_mut()
                    .expect("a text")
                    .push_str(&inner.next().unwrap_or_default());
                for (inner_value, inner_text) in inner_values.into_iter().zip(inner) {
                    spliced_values.push(inner_value);
                    spliced_texts.push(inner_text);
                }
            }
            _ => {
                spliced_values.push(value);
                spliced_texts.push(String::new());
            }
        }
        spliced_texts.last_mut().expect("a text").push_str(after);
    }
    *texts = spliced_texts;
    *values = spliced_values;
}
