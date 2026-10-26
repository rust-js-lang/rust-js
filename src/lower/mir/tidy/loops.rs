//! Tidy's loops: MIR's `loop`s, made `for` and `while` loops.

use super::*;

/// A `for` loop as MIR has it, `const iter = $iter(xs); while (true) {
/// const t = $next(iter); if (t == null) { exit } const x = t; body }`, is
/// `for (const x of xs) { body } exit`; of a range, `$rangeNext` of
/// `{ start, end }`, `for (let i = start; i < end; i++)`, and an inclusive
/// one's, `$rangeInclusiveNext`, `i <= end` (ADR 0036): where nothing else
/// reads the iterator, and only its end leaves the loop. MIR's other shape,
/// `if (t != null) { const x = t; body } exit`, is `for (const x of xs) {
/// body; break; } exit`, its body going on to the exit where it ends.
pub(super) fn loops(stmts: &mut Vec<Stmt>, mentions: &mut HashMap<String, usize>, numbers: &HashSet<String>) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        if let Some(made) = for_of(&stmts[i], &stmts[i + 1], &stmts[i + 2..], mentions, numbers) {
            // What it names, its loop's `end` held say, taken: another
            // loop's is another name.
            for stmt in &made {
                if let StmtKind::Const(name, _) | StmtKind::For { name, .. } = &stmt.kind {
                    *mentions.entry(name.clone()).or_default() += 1;
                }
            }
            let bound = made.iter().find_map(|s| match &s.kind {
                StmtKind::ForOf {
                    pattern: js::Pattern::Name(name),
                    ..
                } => Some(name.clone()),
                _ => None,
            });
            stmts.splice(i..i + 2, made);
            // A `mut` binding's `let`, declared before the loop for MIR's
            // assignment each turn: the loop's own now.
            if let Some(bound) = bound
                && let Some(at) = stmts[..i]
                    .iter()
                    .position(|s| matches!(&s.kind, StmtKind::Let(name, None) if *name == bound))
                && js::mentions_in(&stmts[..i], &bound) == 0
                && js::mentions_in(&stmts[i + 1..], &bound) == 0
            {
                stmts.remove(at);
                i -= 1;
            }
        }
        i += 1;
    }
}

/// Of `body`, a function's, each `let` of no value its own statements
/// declare that a loop binds now, `for (let propName in value)` of `for mut
/// prop_name in js::keys_in(value)`, declared first for MIR's assignment
/// each turn, where nothing but such loops names it: gone, as `loops` takes
/// one beside its loop, wherever in the function the loop is.
pub(super) fn loop_bound_lets(body: &mut Vec<Stmt>) {
    let bound_by = |stmt: &Stmt| match &stmt.kind {
        StmtKind::ForOf {
            pattern: js::Pattern::Name(name),
            ..
        } => Some(name.clone()),
        _ => None,
    };
    let mut bound = HashSet::new();
    js::each_list(body, &mut |stmts| bound.extend(stmts.iter().filter_map(bound_by)));
    if bound.is_empty() {
        return;
    }
    let mut without = body.clone();
    js::statement_lists(&mut without, &mut |stmts| {
        stmts.retain(|s| bound_by(s).is_none_or(|name| !bound.contains(&name)))
    });
    body.retain(|s| {
        !matches!(&s.kind, StmtKind::Let(name, None)
            if bound.contains(name) && js::mentions_in(&without, name) == 0)
    });
}

/// The step of an `Iterator` dictionary, `IIterator.next(it)` (ADR 0411).
const DICTIONARY: &str = "dictionary";

pub(super) fn for_of(
    made: &Stmt,
    looped: &Stmt,
    after: &[Stmt],
    mentions: &HashMap<String, usize>,
    numbers: &HashSet<String>,
) -> Option<Vec<Stmt>> {
    let (iter, source) = match &made.kind {
        StmtKind::Const(iter, source) => (iter, source),
        StmtKind::Assign(target, source)
            if let ExprKind::Var(iter) = &target.kind
                && mentions.get(iter) == Some(&2) =>
        {
            (iter, source)
        }
        _ => return None,
    };
    let StmtKind::While { label, cond, body } = &looped.kind else {
        return None;
    };
    // Where the program wrote it, which names its label (`named_labels`).
    let span = looped.span;
    if !matches!(cond.kind, ExprKind::Bool(true)) {
        return None;
    }
    let [first, second, rest @ ..] = body.as_slice() else {
        return None;
    };
    let StmtKind::Const(item, next) = &first.kind else {
        return None;
    };
    let ExprKind::Call(callee, args) = &next.kind else {
        return None;
    };
    // A helper's step, or an `Iterator` dictionary's, `IIterator.next(it)`,
    // of one given as it is (ADR 0411).
    let given = |d: &Expr| matches!(&d.kind, ExprKind::Member(of, name) if name == "next" && matches!(of.kind, ExprKind::Var(_)));
    let step = match (callee.helper_name(), args.as_slice()) {
        (
            Some(f),
            [
                Expr {
                    kind: ExprKind::Var(it),
                    ..
                },
            ],
        ) if it == iter => f,
        (
            None,
            [
                Expr {
                    kind: ExprKind::Var(it),
                    ..
                },
            ],
        ) if it == iter && given(callee) => DICTIONARY,
        _ => return None,
    };
    if !matches!(
        step,
        "$next" | "$nextSome" | "$rangeNext" | "$rangeInclusiveNext" | DICTIONARY
    ) || js::mentions_in(std::slice::from_ref(looped), iter) != 1
        || js::mentions_in(after, iter) != 0
    {
        return None;
    }
    // The other shape inside a block, `l: { if (t != null) { body } } exit`,
    // whose `break l` in the body goes to the exit: the loop's own `break`,
    // which the exit follows, labeled `l`, as a `break 'outer` is written.
    let (second, block) = match &second.kind {
        StmtKind::Labeled(l, within) if let [only] = within.as_slice() => (only, Some(l.clone())),
        _ => (second, None),
    };
    // When it's done, the exit; else, the body, which in the other shape goes
    // on to the exit, out of the loop.
    let (exit, mut inner, falls_out) = match &second.kind {
        StmtKind::If(test, then, None) if absent(test, item) && block.is_none() => (then.clone(), rest.to_vec(), false),
        StmtKind::If(test, then, Some(other)) if absent(test, item) && rest.is_empty() && block.is_none() => {
            (then.clone(), other.clone(), false)
        }
        StmtKind::If(test, then, None) if present(test, item) => (rest.to_vec(), then.clone(), true),
        _ => return None,
    };
    if !exit.last().is_some_and(ends) || breaks(&inner, label.as_deref()) {
        return None;
    }
    // Going on to the exit where it ends: a `break`, which `unreachable`
    // drops after what leaves already, a `continue` say.
    if falls_out {
        inner.push(StmtKind::Break(None).at(js::Span::NONE));
    }
    let label = match (label, block) {
        (Some(own), Some(block)) => {
            relabeled(&mut inner, &block, own);
            Some(own.clone())
        }
        (label, block) => label.clone().or(block),
    };
    let label = &label;
    // An item that may itself be `None` was boxed by `$nextSome`, or a
    // dictionary's `next`, for the test, and `$someValue` takes it out: JS's
    // `for` binds it as it is, a dictionary's `$iterator` unboxing it.
    let unboxed = |value: &Expr| {
        crate::lower::std_types::option::some_value_of(value)
            .is_some_and(|of| matches!(&of.kind, ExprKind::Var(v) if v == item))
    };
    let mut boxed = false;
    if matches!(step, "$nextSome" | DICTIONARY) {
        let first = inner.first_mut()?;
        match &mut first.kind {
            StmtKind::Const(_, value) | StmtKind::Let(_, Some(value)) if unboxed(value) => {
                *value = Expr::var(item);
                boxed = true;
            }
            _ if step == DICTIONARY => {}
            _ => return None,
        }
    }
    let (pattern, mutable, taken, aliases) = binding(&inner, item)?;
    let mut body = inner[taken..].to_vec();
    for (alias, of) in &aliases {
        js::each_expr_mut(&mut body, &mut |e| {
            if matches!(&e.kind, ExprKind::Var(v) if v == alias) {
                e.kind = ExprKind::Var(of.clone());
            }
        });
    }
    if js::mentions_in(&body, item) > 0 || js::mentions_in(&exit, item) > 0 {
        return None;
    }
    let looped = match step {
        // Stepped by its dictionary, a JS iterator of it.
        DICTIONARY => StmtKind::ForOf {
            label: label.clone(),
            pattern,
            mutable,
            iterable: crate::lower::iterators::stepped(source.clone(), (**callee).clone(), boxed, None),
            body,
            keys: false,
        },
        "$next" | "$nextSome" => {
            let iterable = taken_part(unarrayed(iterated(source.clone())));
            // The keys `for…in` visits, `js::keys_in(o)`'s: `for (const key
            // in o)` (ADR 0719).
            if let Some(object) = crate::lower::iterators::keys_in_source(&iterable) {
                StmtKind::ForOf {
                    label: label.clone(),
                    pattern,
                    mutable,
                    iterable: object.clone(),
                    body,
                    keys: true,
                }
            } else if let Some((counter, key, object)) = counted_keys(&iterable, &pattern, &body) {
                // Of their `enumerate()`, `[n, key]`: `let n = 0;` before the
                // `for…in`, each turn of which ends `n++`, as a person counts.
                let mut body = body;
                counted(&mut body, &counter, label.as_deref());
                let mut out = vec![StmtKind::Let(counter.clone(), Some(Expr::int(0))).at(js::Span::NONE)];
                out.push(
                    StmtKind::ForOf {
                        label: label.clone(),
                        pattern: js::Pattern::Name(key),
                        mutable,
                        iterable: object,
                        body,
                        keys: true,
                    }
                    .at(span),
                );
                out.extend(exit);
                return Some(out);
            } else if let Some(down) = from_the_end(&iterable, &pattern, mutable, &body, (numbers, mentions)) {
                let (name, start, test, body) = down;
                StmtKind::For {
                    label: label.clone(),
                    name,
                    start,
                    test,
                    down: true,
                    body,
                }
            } else {
                StmtKind::ForOf {
                    label: label.clone(),
                    pattern,
                    mutable,
                    iterable,
                    body,
                    keys: false,
                }
            }
        }
        // A range's end, read once by Rust, is read each step by JS's `for`:
        // one that reads the same then, a constant or a variable the body
        // doesn't assign; else a `const end` of it first. An inclusive one's
        // count passes its end only at the test that ends the loop, a JS
        // number not wrapping, as `$rangeInclusiveNext`'s doesn't.
        _ => {
            let ExprKind::Object(props, ..) = &source.kind else {
                return None;
            };
            let [js::Prop::Field(start_key, start), js::Prop::Field(end_key, end)] = props.as_slice() else {
                return None;
            };
            let js::Pattern::Name(name) = pattern else { return None };
            // Or `v.length` of what the body only reads, by index or its length.
            let same = end.is_constant()
                || matches!(&end.kind, ExprKind::Var(v) if !assigns(&body, v))
                || matches!(&end.kind, ExprKind::Member(of, length)
                    if length == "length" && matches!(&of.kind, ExprKind::Var(v) if only_read(&body, v)));
            // JS's `++` steps a number: a `char`'s range, text, steps by
            // its code point, past the surrogates, as `$rangeNext` does.
            // Of arithmetic, `i + 1`, an integer's: a `char` has none.
            let counted = start.as_int().is_some()
                || matches!(&start.kind, ExprKind::Var(v) if numbers.contains(v))
                || matches!(&start.kind, ExprKind::Unary(UnaryOp::Neg, n) if n.as_int().is_some())
                || matches!(&start.kind, ExprKind::Binary(op, ..) if matches!(op, Op::Add | Op::Sub | Op::Mul
                    | Op::Div | Op::Rem | Op::BitOr | Op::BitAnd | Op::BitXor | Op::Shl | Op::Shr | Op::UShr));
            if start_key != "start" || end_key != "end" || mutable || !counted {
                return None;
            }
            // A count the body doesn't read, Rust's `_`: `i`.
            let name = match name.as_str() {
                "_" => std::iter::once("i".to_string())
                    .chain((1..).map(|k| format!("i${k}")))
                    .find(|n| !mentions.contains_key(n))
                    .expect("a name"),
                _ => name,
            };
            let held = (!same).then(|| {
                let unused = |n: &String| !mentions.contains_key(n);
                let name = std::iter::once("end".to_string())
                    .chain((1..).map(|k| format!("end${k}")))
                    .find(unused)
                    .expect("a name");
                (name, end.clone())
            });
            // Its end held first, read before its start where Rust reads it
            // after: of a start that reads only variables, which an end of
            // no effects leaves as they are.
            if held.is_some() && !(start.is_constant() || (start.reads_only_vars() && !end.has_effects())) {
                return None;
            }
            let test = match &held {
                Some((held, _)) => Expr::var(held),
                None => end.clone(),
            };
            let looped = StmtKind::For {
                label: label.clone(),
                test: Expr::bin(
                    if step == "$rangeInclusiveNext" { Op::Le } else { Op::Lt },
                    Expr::var(&name),
                    test,
                ),
                name,
                start: start.clone(),
                down: false,
                body,
            };
            let mut out: Vec<Stmt> = held
                .into_iter()
                .map(|(n, e)| StmtKind::Const(n, e).at(js::Span::NONE))
                .collect();
            out.push(looped.at(span));
            out.extend(exit);
            return Some(out);
        }
    };
    let mut out = vec![looped.at(span)];
    out.extend(exit);
    Some(out)
}

/// A `for…of` of `iterable`, binding `pattern`, `mutable`, as a `for` that
/// counts down (ADR 0904): its counter's name, start and test, and its
/// body. Of an array's items reversed, `v.toReversed()`, which Rust's
/// `v.iter().rev()` lends it, so nothing changes it meanwhile: its indices
/// from the end, `for (let i = v.length - 1; i >= 0; i--) { const x = v[i];
/// .. }`, of a `v` the body doesn't assign, no copy made. Of a range of
/// numbers stepped from its end, `(a..b).rev()`: `for (let i = b - 1; i >=
/// a; i--)`, of an `a` that reads the same each step, a constant or a
/// variable the body doesn't assign; `b` read once, as Rust reads it.
fn from_the_end(
    iterable: &Expr,
    pattern: &js::Pattern,
    mutable: bool,
    body: &[Stmt],
    (numbers, mentions): (&HashSet<String>, &HashMap<String, usize>),
) -> Option<(String, Expr, Expr, Vec<Stmt>)> {
    let fresh = |base: &str| {
        std::iter::once(base.to_string())
            .chain((1..).map(|k| format!("{base}${k}")))
            .find(|n| !mentions.contains_key(n))
            .expect("a name")
    };
    if let Some(items) = iterators::reversed_source(iterable) {
        let ExprKind::Var(array) = &items.kind else {
            return None;
        };
        if assigns(body, array) {
            return None;
        }
        let index = fresh("i");
        let item = Expr::index(items.clone(), Expr::var(&index));
        let bound = match (pattern, mutable) {
            (js::Pattern::Name(name), false) => StmtKind::Const(name.clone(), item),
            (js::Pattern::Name(name), true) => StmtKind::Let(name.clone(), Some(item)),
            (pattern, mutable) => StmtKind::Destructure {
                pattern: pattern.clone(),
                value: item,
                mutable,
            },
        };
        let last = Expr::bin(Op::Sub, Expr::member(items.clone(), "length"), Expr::int(1));
        let test = Expr::bin(Op::Ge, Expr::var(&index), Expr::int(0));
        let mut stepped = vec![bound.at(js::Span::NONE)];
        stepped.extend(body.iter().cloned());
        return Some((index, last, test, stepped));
    }
    let range = iterators::stepped_range(iterable).filter(|range| range.back)?;
    let js::Pattern::Name(name) = pattern else {
        return None;
    };
    // JS's `--` steps a number: of a `char`'s range, text, none.
    let number = |e: &Expr| {
        e.as_int().is_some()
            || matches!(&e.kind, ExprKind::Var(v) if numbers.contains(v))
            || matches!(&e.kind, ExprKind::Unary(UnaryOp::Neg, n) if n.as_int().is_some())
            || matches!(&e.kind, ExprKind::Member(_, length) if length == "length")
            || matches!(&e.kind, ExprKind::Binary(op, ..) if matches!(op, Op::Sub | Op::Mul | Op::Div | Op::Rem
                | Op::BitOr | Op::BitAnd | Op::BitXor | Op::Shl | Op::Shr | Op::UShr))
            // `Math.max(to, 0)`, which JS makes a number of anything.
            || matches!(&e.kind, ExprKind::Call(callee, _) if matches!(&callee.kind,
                ExprKind::Member(math, name) if matches!(&math.kind, ExprKind::Var(m) if m == "Math")
                    && matches!(name.as_str(), "max" | "min")))
    };
    let same = range.start.is_constant() || matches!(&range.start.kind, ExprKind::Var(v) if !assigns(body, v));
    if mutable || !number(range.start) || !number(range.end) || !same {
        return None;
    }
    let name = match name.as_str() {
        "_" => fresh("i"),
        _ => name.clone(),
    };
    let start = match (range.inclusive, range.end.as_int()) {
        (true, _) => range.end.clone(),
        (false, Some(end)) => Expr::int(end - 1),
        (false, None) => Expr::bin(Op::Sub, range.end.clone(), Expr::int(1)),
    };
    let test = Expr::bin(Op::Ge, Expr::var(&name), range.start.clone());
    Some((name, start, test, body.to_vec()))
}

/// Of a loop of `[n, key]` of the keys `for…in` visits, enumerated,
/// `$keysIn(o).map((x, i) => [i, x])`: `n`, `key` and `o`, where a counter
/// is the same `n` each turn: no closure in the body reads it, which would
/// see it change, where each turn's `const` is its own (ADR 0719). A name
/// is the function's one variable of it here, as the `i` a range's loop
/// takes is; preparation names the counter apart from another's.
fn counted_keys(iterable: &Expr, pattern: &js::Pattern, body: &[Stmt]) -> Option<(String, String, Expr)> {
    let object = crate::lower::iterators::keys_in_source(crate::lower::iterators::enumeration_source(iterable)?)?;
    let js::Pattern::Array(parts) = pattern else {
        return None;
    };
    let [Some(js::Pattern::Name(counter)), Some(js::Pattern::Name(key))] = parts.as_slice() else {
        return None;
    };
    let mut closed_over = false;
    js::each_expr(body, &mut |e| {
        if let ExprKind::Arrow(_, inner) | ExprKind::AsyncArrow(_, inner) = &e.kind {
            closed_over |= js::mentions_in(inner, counter) > 0;
        }
        if let ExprKind::Function(function) = &e.kind {
            closed_over |= js::mentions_in(&function.body, counter) > 0;
        }
    });
    let declared = |stmts: &[Stmt]| {
        let mut found = false;
        js::each_list(stmts, &mut |list| {
            found |= list
                .iter()
                .any(|s| matches!(&s.kind, StmtKind::Function(f) if js::mentions_in(&f.body, counter) > 0));
        });
        found
    };
    (!closed_over && !declared(body)).then(|| (counter.clone(), key.clone(), object.clone()))
}

/// `body`, a loop's, each turn of which ends `counter++`: where it goes on
/// to the next turn, at its end and before each `continue` of the loop's,
/// one of no label not in a loop of its own, or of `label`.
fn counted(body: &mut Vec<Stmt>, counter: &str, label: Option<&str>) {
    let step = || StmtKind::Compound(Op::Add, Expr::var(counter), Expr::int(1)).at(js::Span::NONE);
    fn before_continues(stmts: &mut Vec<Stmt>, step: &dyn Fn() -> Stmt, label: Option<&str>, nested: bool) {
        let mut i = 0;
        while i < stmts.len() {
            let ours = match &stmts[i].kind {
                StmtKind::Continue(None) => !nested,
                StmtKind::Continue(Some(l)) => Some(l.as_str()) == label,
                _ => false,
            };
            if ours {
                stmts.insert(i, step());
                i += 2;
                continue;
            }
            match &mut stmts[i].kind {
                StmtKind::If(_, a, b) => {
                    before_continues(a, step, label, nested);
                    if let Some(b) = b {
                        before_continues(b, step, label, nested);
                    }
                }
                StmtKind::Labeled(_, body) => before_continues(body, step, label, nested),
                StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
                    before_continues(body, step, label, true)
                }
                StmtKind::Switch { cases, default, .. } => {
                    for body in js::case_lists_mut(cases, default) {
                        before_continues(body, step, label, nested);
                    }
                }
                StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                    before_continues(a, step, label, nested);
                    before_continues(b, step, label, nested);
                }
                _ => {}
            }
            i += 1;
        }
    }
    before_continues(body, &step, label, false);
    if !body.last().is_some_and(ends) {
        body.push(step());
    }
}

/// What a `for…of` iterates of an array's items taken from,
/// `$iter(xs).take(n)`, `.drop(n)` or both: the array's part,
/// `xs.slice(0, n)`, as a chain's `take` is (`sliced`); of their
/// `enumerate().take(n)`, the part's `entries()`, each index as it was. An
/// `enumerate().skip(n)` keeps its iterator's `drop`, as the part's indices
/// would start at 0. The part is read as the loop starts, where its
/// iterator would read each item as it's stepped: nothing changes the
/// array meanwhile, as Rust lends it to the iterator (ADR 0748).
fn taken_part(iterable: Expr) -> Expr {
    use crate::lower::mir::iter::sources::{counted_chain, sliced};
    let span = iterable.span;
    let part = if counted_chain(&iterable) {
        sliced(&iterable)
    } else if let ExprKind::Call(callee, args) = &iterable.kind
        && let ExprKind::Member(it, name) = &callee.kind
        && name == "take"
        && let [n] = &args[..]
        && let Some(items) = iterators::enumeration_source(it)
        && counted_chain(&Expr::call(Expr::member(items.clone(), "take"), vec![n.clone()]))
    {
        let part = sliced(&Expr::call(Expr::member(items.clone(), "take"), vec![n.clone()]));
        Expr::call(Expr::member(part, "entries"), Vec::new())
    } else {
        return iterable;
    };
    Expr { span, ..part }
}

/// What a `for…of` iterates, of `Array.from(xs)`, a string's `chars()`:
/// `xs` itself, the same items, which nothing changes as it's iterated.
pub(super) fn unarrayed(e: Expr) -> Expr {
    match e.kind {
        ExprKind::Call(callee, mut args)
            if args.len() == 1
                && matches!(&callee.kind, ExprKind::Member(object, from)
                    if from == "from" && matches!(&object.kind, ExprKind::Var(v) if v == "Array"))
                && matches!(args[0].kind, ExprKind::Var(_)) =>
        {
            args.pop().expect("one")
        }
        kind => Expr {
            facts: js::Facts::NONE,
            kind,
            span: e.span,
        },
    }
}

/// `body; break;`, a loop's body that leaves where it ends, each `break`
/// where the program wrote it: of `if (t) { b; continue; } else { a }`,
/// `if (t) { b } else { a; break; }`; of `if (t) { a } else { b; continue;
/// }`, the `break` after it, which the early breaks put in `a`, where it
/// was written (ADR 0730). A
/// `break` after what leaves anyway is dropped by `unreachable`, the one
/// pass that drops what's after a leave. A `continue` of no label, in the
/// loop's body and no loop in it, is the loop's own.
fn broken_after(body: &mut Vec<Stmt>) {
    let goes_on = |s: &Stmt| matches!(s.kind, StmtKind::Continue(None));
    // What follows a branch that leaves anyway is `unreachable`'s to drop.
    let leaves = |stmts: &mut Vec<Stmt>| stmts.push(StmtKind::Break(None).at(js::Span::NONE));
    if let Some(Stmt {
        kind: StmtKind::If(_, then, Some(otherwise)),
        ..
    }) = body.last_mut()
        && then.last().is_some_and(goes_on)
        && !otherwise.last().is_some_and(goes_on)
    {
        then.pop();
        leaves(otherwise);
        return;
    }
    leaves(body);
}

/// `while (true) { if (t) { exit } rest }`, `exit` leaving, and nothing in
/// `rest` leaving the loop, nor in `exit` going on with it: `while (!t) {
/// rest } exit`, as the program wrote it; of an `exit` that ends with the
/// loop's `break`, without it.
pub(super) fn while_tests(
    stmts: &mut Vec<Stmt>,
    loops: &[(js::Span, bool, Option<String>)],
    written_breaks: &[(js::Span, js::Span)],
) {
    // The loop the program wrote that starts where it does, innermost, a
    // `while`: not a `loop` it wrote, which stays `while (true)`.
    let written_while = |span: js::Span| written_loop(loops, span).is_some_and(|&(_, w, _)| w);
    let mut i = 0;
    while i < stmts.len() {
        if !written_while(stmts[i].span) {
            i += 1;
            continue;
        }
        // `if (t) { exit } else { rest }` alone, its `else` not yet taken
        // apart (ADR 0237): `if (t) { exit } rest`.
        if let StmtKind::While { cond, body, .. } = &mut stmts[i].kind
            && matches!(cond.kind, ExprKind::Bool(true))
            && let [
                Stmt {
                    kind: StmtKind::If(_, exit, Some(_)),
                    ..
                },
            ] = body.as_slice()
            && exit
                .last()
                .is_some_and(|last| matches!(last.kind, StmtKind::Break(_) | StmtKind::Return(_) | StmtKind::Throw(_)))
            && let StmtKind::If(_, _, otherwise) = &mut body[0].kind
        {
            let rest = otherwise.take().unwrap_or_default();
            body.extend(rest);
        }
        // `while (true) { if (t) { rest } else { break; } }`, the `while`
        // itself as Rust writes it, its test as written (ADR 0683): `while
        // (t) { rest }` (ADR 0702); of a `while (a)`, `while (a && t) {
        // rest }`, as a guard's scope's loop is once what its first tidy
        // left apart is read in place (ADR 0705). A `break` of no label, in
        // the loop's body and no loop in it, is the loop's own.
        let own_break = |s: &Stmt| matches!(s.kind, StmtKind::Break(None));
        if let StmtKind::While { cond, body, .. } = &mut stmts[i].kind
            && let (
                Some(Stmt {
                    kind: StmtKind::If(_, _, otherwise),
                    ..
                }),
                after,
            ) = match body.as_slice() {
                [only] => (Some(only), None),
                [first, last] => (Some(first), Some(last)),
                _ => (None, None),
            }
            && match (otherwise, after) {
                (Some(exit), None) => matches!(exit.as_slice(), [b] if own_break(b)),
                (None, Some(last)) => own_break(last),
                _ => false,
            }
        {
            let ended = body.len() == 2;
            let Some(Stmt {
                kind: StmtKind::If(test, mut rest, _),
                ..
            }) = body.drain(..1).next()
            else {
                unreachable!("matched")
            };
            if ended {
                broken_after(&mut rest);
            }
            // The program's own `if`s of a `break` apart (`broken_apart`).
            let (test, broken) = broken_apart(test, Op::And, written_breaks);
            rest.splice(0..0, broken);
            *cond = match cond.kind {
                ExprKind::Bool(true) => test,
                _ => Expr::bin(Op::And, std::mem::replace(cond, Expr::undefined()), test),
            };
            *body = rest;
            i += 1;
            continue;
        }
        if let StmtKind::While { label, cond, body } = &stmts[i].kind
            && matches!(cond.kind, ExprKind::Bool(true))
            && let [first, rest @ ..] = body.as_slice()
            && let StmtKind::If(_, exit, None) = &first.kind
            && let Some(last) = exit.last()
            && let broken = matches!(&last.kind, StmtKind::Break(l) if l.is_none() || *l == *label)
            && (broken || matches!(last.kind, StmtKind::Return(_) | StmtKind::Throw(_)))
            && !breaks(&exit[..exit.len() - usize::from(broken)], label.as_deref())
            // A `continue` in the exit, after the loop, would go on with
            // another, the one around it (ADR 0683, amended).
            && !js::continues(exit, label.as_deref())
            // A `break` in it would run the exit, which it skips.
            && (exit.len() == usize::from(broken) || !breaks(rest, label.as_deref()))
        {
            let span = stmts[i].span;
            let StmtKind::While { label, body, .. } = std::mem::replace(&mut stmts[i].kind, StmtKind::Break(None))
            else {
                unreachable!("matched")
            };
            let mut body = body.into_iter();
            let Some(Stmt {
                kind: StmtKind::If(test, exit, None),
                ..
            }) = body.next()
            else {
                unreachable!("matched")
            };
            let mut exit = exit;
            if matches!(exit.last().map(|s| &s.kind), Some(StmtKind::Break(_))) {
                exit.pop();
            }
            // The program's own `if`s of a `break` apart (`broken_apart`):
            // each goes on to the exit after the loop as the test does.
            let (test, broken) = broken_apart(test, Op::Or, written_breaks);
            stmts[i] = StmtKind::While {
                label,
                cond: test.negated_test(),
                body: broken.into_iter().chain(body).collect(),
            }
            .at(span);
            let n = exit.len();
            stmts.splice(i + 1..i + 1, exit);
            i += n;
        }
        i += 1;
    }
}

/// A `while`'s test, `test`, of the tests `op` joins, `&&` of those it
/// goes on by, `||` of those it leaves by: the loop's own, and an `if` of
/// the `break` of each `if` the program wrote of one (`Written::breaks`)
/// those after it test, first to last, in the loop's body where they're
/// written (ADR 0702, amended). `while (i < n && !(c && i === 3))` of
/// `while i < n { if c && i == 3 { break; } .. }` is `while (i < n) { if (c
/// && i === 3) { break; } .. }`. The same tests, in the same order.
fn broken_apart(test: Expr, op: Op, written_breaks: &[(js::Span, js::Span)]) -> (Expr, Vec<Stmt>) {
    fn operands(e: &Expr, op: Op, out: &mut Vec<Expr>) {
        match &e.kind {
            ExprKind::Binary(o, a, b) if *o == op => {
                operands(a, op, out);
                operands(b, op, out);
            }
            _ => out.push(e.clone()),
        }
    }
    let mut all = Vec::new();
    operands(&test, op, &mut all);
    // Of which `if` the program wrote each is the test of, if one.
    let written = |e: &Expr| super::jumps::jump_tested(written_breaks, super::jumps::written_at(e));
    let own = all.iter().rposition(|e| written(e).is_none()).map_or(0, |k| k + 1);
    if own == 0 || own == all.len() {
        return (test, Vec::new());
    }
    let tested = all.split_off(own);
    let join = |es: Vec<Expr>| es.into_iter().reduce(|a, b| Expr::bin(op, a, b)).expect("one or more");
    let mut ifs = Vec::new();
    let mut tested = tested.into_iter().peekable();
    while let Some(first) = tested.next() {
        let of = written(&first);
        let mut group = vec![first];
        while let Some(next) = tested.next_if(|e| written(e) == of) {
            group.push(next);
        }
        let at = super::jumps::written_at(&group[0]);
        let exit = match op {
            Op::And => join(group).negated_test(),
            _ => join(group),
        };
        ifs.push(StmtKind::If(exit, vec![StmtKind::Break(None).at(at)], None).at(at));
    }
    (join(all), ifs)
}

/// Whether loop `stmt`'s own test, start or what it steps names `name`.
pub(super) fn looped_reads(stmt: &Stmt, name: &str) -> bool {
    match &stmt.kind {
        StmtKind::While { cond, .. } => cond.mentions_var(name),
        StmtKind::For { start, test, .. } => start.mentions_var(name) || test.mentions_var(name),
        StmtKind::ForOf { iterable, .. } => iterable.mentions_var(name),
        _ => true,
    }
}
