//! A body lowered from its MIR, as a person writes it (ADR 0364): each
//! rewrite changes nothing that runs. MIR's one return block is a `return`
//! where each path ends, and what the structure needed to reach it, a
//! labeled block or a `break`, goes. A closure's body is tidied as its own.

use std::collections::{HashMap, HashSet};

use crate::js::{self, Expr, ExprKind, Op, Prop, Stmt, StmtKind, UnaryOp};

use super::negate;
use crate::lower::iterators::{self, eager_chains, eager_spreads, iterated};

/// What MIR's lowering, and THIR, say of a body for tidying it.
pub(super) struct Hints<'h> {
    /// The name of the function's return place.
    pub(super) result: &'h str,
    /// The temporaries a function that skips what's falsy is given.
    pub(super) falsy: &'h HashSet<String>,
    /// And the temporaries such a `test && value` tests.
    pub(super) tested: &'h HashSet<String>,
    /// Where the program writes each `if`'s condition, twice, and each
    /// `match` of a guarded arm and its guard.
    pub(super) conditions: &'h [(js::Span, js::Span)],
    /// Where each loop is written, whether as a `while`, and its label.
    pub(super) loops: &'h [(js::Span, bool, Option<String>)],
    /// Where each `if` the program wrote of a `continue` tests: one alone,
    /// or one that ends what it does.
    pub(super) continues: &'h [js::Span],
    /// Where each `&&` or `||` the program wrote is.
    pub(super) logicals: &'h [js::Span],
    /// The closures a `let` names that a hook's function alone is (ADR 0297).
    pub(super) callbacks: &'h HashSet<String>,
    /// Where each `match` that reads a table by its key is (ADR 0233).
    pub(super) tables: &'h [js::Span],
    /// What a `let`'s struct pattern binds, by name.
    pub(super) patterns: &'h HashSet<String>,
    /// Whether the body is a closure's, tidied again in its function's.
    pub(super) closure: bool,
    pub(super) returns: &'h [js::Span],
    /// The `const`s MIR's lowering made of what an element it made ahead
    /// of a branch reads (`capture_jsx`): as its temporaries, read where
    /// they're used where nothing between tells.
    pub(super) captured: &'h HashSet<String>,
    /// The variables of Rust's `bool`: JS's `true` and `false` alone.
    pub(super) booleans: &'h HashSet<String>,
    /// The variables of plain values, which nothing changes, and `...items`
    /// of arrays whose items nothing changes (`Locals::plain`).
    pub(super) plain: &'h HashSet<String>,
    /// Where each closure that passes on what it's given to Rust's function
    /// is written (ADR 0279).
    pub(super) forwarding: &'h [js::Span],
    /// Where each closure of one call whose JS gives `undefined` is written:
    /// a component's handler of one, `() => f()`, as THIR's (ADR 0040).
    pub(super) concise: &'h [js::Span],
}

/// `body`, tidied, by what `hints` say of it.
pub(super) fn tidy(body: &mut Vec<Stmt>, hints: &Hints) {
    let Hints {
        result,
        falsy,
        tested,
        conditions,
        loops: written,
        continues,
        logicals,
        callbacks,
        tables,
        patterns,
        closure,
        returns,
        captured,
        booleans,
        plain,
        forwarding,
        concise,
    } = *hints;
    // A guard of a handle made for it alone: the place it's a handle on.
    unguarded(body);
    // Twice: what a block's statements, in its place, are followed by, and
    // a loop's label nothing names then.
    for _ in 0..2 {
        js::statement_lists(body, &mut |stmts| {
            returned(stmts, result);
            unreachable_after(stmts);
        });
        // An arm MIR made the block's fall-through, written first: first.
        js::statement_lists(body, &mut |stmts| written_first(stmts));
        js::statement_lists(body, &mut |stmts| branch_ends(stmts));
        js::statement_lists(body, &mut |stmts| tail_jumps(stmts));
        bottom_up(body, &mut |stmts| branch_values(stmts, returns, booleans));
        js::statement_lists(body, &mut unlabeled);
    }
    bottom_up(body, &mut pattern_tests);
    bottom_up(body, &mut or_tests);
    bottom_up(body, &mut |stmts| branch_values(stmts, returns, booleans));
    let mut read = HashSet::new();
    js::visit_stmts(body, &mut |name| {
        read.insert(name.to_string());
    });
    if !read.contains(result) {
        body.retain(|s| !matches!(&s.kind, StmtKind::Let(name, None) if name == result));
    }
    js::statement_lists(body, &mut guarded);
    declarations(body);
    aliases(body, captured);
    // Of `const`s now, the iterator's too; and what a loop made so ends with,
    // a `continue` and its label, goes.
    // An iterator a loop inside another makes, a `let` assigned each time,
    // read by nothing else.
    let mut mentions: HashMap<String, usize> = HashMap::new();
    js::visit_stmts(body, &mut |name| *mentions.entry(name.to_string()).or_default() += 1);
    // The innermost first: what a loop ends with may be another, recovered.
    bottom_up(body, &mut |stmts| loops(stmts, &mentions));
    js::statement_lists(body, &mut |stmts| while_tests(stmts, written));
    js::statement_lists(body, &mut |stmts| early_breaks(stmts));
    js::statement_lists(body, &mut |stmts| tail_jumps(stmts));
    js::statement_lists(body, &mut unlabeled);
    // A `let` declared first, its one write now beside it, what a loop
    // held: a `const` there.
    declarations(body);
    js::statement_lists(body, &mut |stmts| continued(stmts, continues));
    js::statement_lists(body, &mut |stmts| tail_jumps(stmts));
    js::statement_lists(body, &mut |stmts| innermost_jumps(stmts));
    // A chain of tests a loop ended, its body now ending where the chain's does.
    bottom_up(body, &mut |stmts| branch_values(stmts, returns, booleans));
    js::statement_lists(body, &mut unlabeled);
    js::statement_lists(body, &mut |stmts| logical_returns(stmts, logicals, returns, booleans));
    // What's given where `false` is skipped as `undefined` is, `test &&
    // value`, as THIR gives it there: one read there alone, which nothing
    // else tells from `undefined`.
    let falsy: HashSet<&String> = falsy.iter().filter(|name| js::mentions_in(body, name) == 1).collect();
    // A test of one read there alone is read for its truth: `!!a || b`'s,
    // `a || b`.
    let tested: HashSet<&String> = tested.iter().filter(|name| js::mentions_in(body, name) == 1).collect();
    js::statement_lists(body, &mut |stmts| {
        for stmt in stmts.iter_mut() {
            if let StmtKind::Const(name, value) = &mut stmt.kind
                && falsy.contains(name)
                && let ExprKind::Cond(test, then, none) = &value.kind
                && matches!(none.kind, ExprKind::Undefined)
            {
                *value = Expr::bin(Op::And, test.tested(), (**then).clone());
            }
            if let StmtKind::Const(name, value) = &mut stmt.kind
                && tested.contains(name)
            {
                *value = value.tested();
            }
        }
    });
    let changing = changing(body);
    js::statement_lists(body, &mut |stmts| filtered_guards(stmts, &changing));
    alias_variables(body, &changing);
    read_handles(body);
    js::statement_lists(body, &mut |stmts| plain_reads(stmts, plain, captured));
    js::statement_lists(body, &mut |stmts| destructured(stmts, patterns));
    js::statement_lists(body, &mut rest_parts);
    // What an array made for its parts left, `const _5 = 6;`, too.
    alias_variables(body, &changing);
    named_temporaries(body);
    js::each_expr_mut(body, &mut range_items);
    eager_chains(body);
    js::statement_lists(body, &mut |stmts| eager_spreads(stmts));
    returned_in_branches(body, result);
    // Each `let` where it's first named, a `const` where it's written once.
    js::statement_lists(body, &mut sunk_lets);
    declarations(body);
    // What those `const`s now read once, where it's read, and the values
    // of branches that leaves.
    // A pattern's copy of a variable, `const error = raw;`: the variable.
    js::statement_lists(body, &mut |stmts| copied_bindings(stmts));
    // A tuple read item by item, `children[0]`: its items.
    let changing = self::changing(body);
    js::statement_lists(body, &mut |stmts| tuple_reads(stmts, &changing, captured));
    // `dyn` values given to JS, `[{ value, impl }].map((item) => item.value)`: the values.
    js::statement_lists(body, &mut |stmts| unwrapped_values(stmts));
    let changed = self::changing(body);
    let set = closures_set(body);
    // What an element MIR made only where a test holds reads, as one value,
    // read in place below, where the test holds.
    // A match's table read, `TITLES[kind]`, where it's written, before
    // what it's given to reads it in place.
    js::statement_lists(body, &mut |stmts| table_reads(stmts, tables));
    js::statement_lists(body, &mut |stmts| guarded_lets(stmts, captured));
    js::statement_lists(body, &mut |stmts| inlined(stmts, &changed, &set, plain, captured));
    bottom_up(body, &mut |stmts| branch_values(stmts, returns, booleans));
    js::statement_lists(body, &mut |stmts| table_reads(stmts, tables));
    declarations(body);
    js::statement_lists(body, &mut |stmts| inlined(stmts, &changed, &set, plain, captured));
    js::each_expr_mut(body, &mut |e| implied_tests(e, &changed));
    // A chain read in place now, `$lent($iter(xs)).some(f)` of `const _7 =
    // $iter(xs);`: the array's, as THIR's; and a range made there, its
    // numbers made as they're stepped.
    js::each_expr_mut(body, &mut range_items);
    eager_chains(body);
    // Of each, a callback that passes what it's given on to Rust's function,
    // `(x, i) => f(x, i)`: `f` itself, as THIR's.
    js::each_expr_mut(body, &mut |e| passed_on(e, forwarding));
    // An element's place checked, then set: the place the check gives.
    js::statement_lists(body, &mut checked_sets);
    // A copy made of what's spread, read in place, `{ ...a, ...{ ...b } }`:
    // what it's a copy of.
    js::each_expr_mut(body, &mut spread_spreads);
    // A chain's tests, where each is read in place now: `if (a) { if (f(a))
    // { .. } }`, `if (a && f(a))`.
    bottom_up(body, &mut |stmts| and_tests(stmts, conditions));
    // What a block ends after, its tests read in place now: its `else`.
    js::statement_lists(body, &mut |stmts| tail_elses(stmts));
    // A check MIR wrote first in a chain's last `else`: the chain's arm.
    js::statement_lists(body, &mut |stmts| chained_checks(stmts));
    js::statement_lists(body, &mut unlabeled);
    // `x != null && typeof x === "string"`: the `typeof` test, which says it.
    js::each_expr_mut(body, &mut typeof_tests);
    // A loop's test those read in place now, `while (true) { if (!(m.length
    // > n)) { .. } }`'s of `const _7 = m.length;`.
    js::statement_lists(body, &mut |stmts| while_tests(stmts, written));
    // What those leave at the end, a `return;` the program didn't write.
    tail_returns(body, returns);
    // A chain's arms, as statements, in the order written.
    js::statement_lists(body, &mut |stmts| written_chains(stmts));
    // `const result = e; return result;` of what the function gives: `return e;`.
    js::statement_lists(body, &mut |stmts| returned_consts(stmts, result));
    // And their arms that do nothing.
    // A cell only its `value` is read and set of, by name: its variable.
    plain_cells(body);
    js::statement_lists(body, &mut |stmts| named_callbacks(stmts, callbacks));
    // A child shown only where a test holds, `c ? <b /> : undefined`: `c &&
    // <b />`, as JSX writes it (ADR 0235), of a boolean test, whose `false`
    // renders nothing as `undefined` does; of another, its truth, `!!href
    // && <a />`, as `""` or `0` would render.
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Jsx(jsx) = &mut e.kind {
            for child in jsx.children.iter_mut() {
                if let ExprKind::Cond(test, shown, none) = &child.kind
                    && matches!(none.kind, ExprKind::Undefined)
                {
                    let test = match is_boolean(test, booleans) {
                        true => (**test).clone(),
                        false => Expr::unary(UnaryOp::Not, Expr::unary(UnaryOp::Not, (**test).clone())),
                    };
                    *child = Expr {
                        reads: false,
                        span: child.span,
                        kind: ExprKind::Binary(Op::And, Box::new(test), shown.clone()),
                    };
                }
            }
        }
    });
    // A handler of one call a component's given, `() => { f(); }`, held then
    // read in place: `() => f()`, as THIR gives one where it's made.
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Jsx(jsx) = &mut e.kind {
            for prop in &mut jsx.props {
                if let js::Prop::Field(name, value) = prop
                    && js::is_handler_name(name)
                    && concise.contains(&value.span)
                {
                    js::returning_its_call(value);
                }
            }
        }
    });
    js::statement_lists(body, &mut |stmts| named_labels(stmts, written));
    loop_exits(body, returns, written);
    js::statement_lists(body, &mut map_counts);
    // `!` of what's read in place now, `!(t === "pre")` of `!_5`: as
    // `Expr::unary` makes it, `t !== "pre"`.
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Unary(op, arg) = &e.kind {
            *e = Expr {
                span: e.span,
                ..Expr::unary(*op, (**arg).clone())
            };
        }
    });
    // What `filter` keeps, then `or` or `unwrap_or`: `x || y`.
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Binary(Op::Coalesce, kept, _) = &e.kind
            && truthy_or_undefined(kept)
        {
            let ExprKind::Binary(_, kept, otherwise) = std::mem::replace(&mut e.kind, ExprKind::Undefined) else {
                unreachable!("matched")
            };
            e.kind = ExprKind::Binary(Op::Or, Box::new(truth_of(*kept)), otherwise);
        }
    });
    js::statement_lists(body, &mut forwarded_values);
    unread_maps(body);
    // An array's `$iter` an adapter alone steps, left lazy, as what its
    // closures do is in Rust's order: the array's own iterator.
    // A closure's, by the function it's in, which sees what takes what it
    // gives, `xs.flatMap((x) => $iter(x).map(f))`'s, first.
    if !closure {
        js::each_expr_mut(body, &mut array_values);
    }
    handles_in_place(body);
    // A `let` declared first that nothing's left to assign or read.
    let mut mentions: HashMap<String, usize> = HashMap::new();
    js::visit_stmts(body, &mut |name| *mentions.entry(name.to_string()).or_default() += 1);
    body.retain(|stmt| !matches!(&stmt.kind, StmtKind::Let(name, None) if !mentions.contains_key(name)));
}

/// A `for` loop as MIR has it, `const iter = $iter(xs); while (true) {
/// const t = $next(iter); if (t == null) { exit } const x = t; body }`, is
/// `for (const x of xs) { body } exit`; of a range, `$rangeNext` of
/// `{ start, end }`, `for (let i = start; i < end; i++)`: where nothing else
/// reads the iterator, and only its end leaves the loop. MIR's other shape,
/// `if (t != null) { const x = t; body } exit`, is `for (const x of xs) {
/// body; break; } exit`, its body going on to the exit where it ends.
fn loops(stmts: &mut Vec<Stmt>, mentions: &HashMap<String, usize>) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        if let Some(made) = for_of(&stmts[i], &stmts[i + 1], &stmts[i + 2..], mentions) {
            stmts.splice(i..i + 2, made);
        }
        i += 1;
    }
}

fn for_of(made: &Stmt, looped: &Stmt, after: &[Stmt], mentions: &HashMap<String, usize>) -> Option<Vec<Stmt>> {
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
    let step = match (&callee.kind, args.as_slice()) {
        (
            ExprKind::Var(f),
            [
                Expr {
                    kind: ExprKind::Var(it),
                    ..
                },
            ],
        ) if it == iter => f.as_str(),
        _ => return None,
    };
    if !matches!(step, "$next" | "$nextSome" | "$rangeNext")
        || js::mentions_in(std::slice::from_ref(looped), iter) != 1
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
    // Going on to the exit where it ends, unless it leaves already, a
    // `continue` say: a `break`.
    if falls_out && !inner.last().is_some_and(ends) {
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
    // An item that may itself be `None` was boxed by `$nextSome` for the
    // test, and `$someValue` takes it out: JS's `for` binds it as it is.
    if step == "$nextSome" {
        let unboxed = |value: &Expr| match &value.kind {
            ExprKind::Call(f, args) if matches!(&f.kind, ExprKind::Var(f) if f == "$someValue") => {
                matches!(args.as_slice(), [Expr { kind: ExprKind::Var(v), .. }] if v == item)
            }
            _ => false,
        };
        let first = inner.first_mut()?;
        match &mut first.kind {
            StmtKind::Const(_, value) | StmtKind::Let(_, Some(value)) if unboxed(value) => {
                *value = Expr::var(item);
            }
            _ => return None,
        }
    }
    let (pattern, mutable, taken) = binding(&inner, item)?;
    let body = inner[taken..].to_vec();
    if js::mentions_in(&body, item) > 0 || js::mentions_in(&exit, item) > 0 {
        return None;
    }
    let looped = match step {
        "$next" | "$nextSome" => StmtKind::ForOf {
            label: label.clone(),
            pattern,
            mutable,
            iterable: unarrayed(iterated(source.clone())),
            body,
        },
        // A range's end, read once by Rust, is read each step by JS's `for`:
        // one that reads the same then, a constant or a variable the body
        // doesn't assign; else a `const end` of it first, as THIR writes it.
        _ => {
            let ExprKind::Object(props) = &source.kind else {
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
            if start_key != "start" || end_key != "end" || mutable {
                return None;
            }
            let held = (!same).then(|| {
                let unused = |n: &String| !mentions.contains_key(n);
                let name = std::iter::once("end".to_string())
                    .chain((1..).map(|k| format!("end${k}")))
                    .find(unused)
                    .expect("a name");
                (name, end.clone())
            });
            if held.is_some() && !(start.is_constant() || matches!(start.kind, ExprKind::Var(_))) {
                return None;
            }
            let test = match &held {
                Some((held, _)) => Expr::var(held),
                None => end.clone(),
            };
            let looped = StmtKind::For {
                label: label.clone(),
                test: Expr::bin(Op::Lt, Expr::var(&name), test),
                name,
                start: start.clone(),
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

/// What a `for…of` iterates, of `Array.from(xs)`, a string's `chars()`:
/// `xs` itself, the same items, which nothing changes as it's iterated.
fn unarrayed(e: Expr) -> Expr {
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
            reads: false,
            kind,
            span: e.span,
        },
    }
}

/// `while (true) { if (t) { exit } rest }`, `exit` leaving, and nothing in
/// `rest` leaving the loop: `while (!t) { rest } exit`, as the program
/// wrote it; of an `exit` that ends with the loop's `break`, without it.
fn while_tests(stmts: &mut Vec<Stmt>, loops: &[(js::Span, bool, Option<String>)]) {
    // The loop the program wrote that starts where it does, innermost, a
    // `while`: not a `loop` it wrote, which stays `while (true)`.
    let written_while = |span: js::Span| written_loop(loops, span).is_some_and(|&(_, w, _)| w);
    let mut i = 0;
    while i < stmts.len() {
        if !written_while(stmts[i].span) {
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
            stmts[i] = StmtKind::While {
                label,
                cond: not(test),
                body: body.collect(),
            }
            .at(span);
            let n = exit.len();
            stmts.splice(i + 1..i + 1, exit);
            i += n;
        } else if let StmtKind::While { label, cond, body } = &stmts[i].kind
            && matches!(cond.kind, ExprKind::Bool(true))
            && let [first, exit @ ..] = body.as_slice()
            && let StmtKind::If(_, looped, None) = &first.kind
            && exit
                .last()
                .is_some_and(|s| matches!(s.kind, StmtKind::Return(_) | StmtKind::Throw(_)))
            && !breaks(looped, label.as_deref())
            && !breaks(exit, label.as_deref())
            && !goes_on(exit, label.as_deref())
        {
            // `while (true) { if (c) { a } exit }`, `a` going on only by
            // `continue`: `while (c) { a; break; } exit`, a path out of `a`
            // the loop's end.
            let span = stmts[i].span;
            let StmtKind::While { label, body, .. } = std::mem::replace(&mut stmts[i].kind, StmtKind::Break(None))
            else {
                unreachable!("matched")
            };
            let mut body = body.into_iter();
            let Some(Stmt {
                kind: StmtKind::If(test, mut looped, None),
                ..
            }) = body.next()
            else {
                unreachable!("matched")
            };
            if !looped.last().is_some_and(ends) {
                looped.push(StmtKind::Break(None).at(span));
            }
            stmts[i] = StmtKind::While {
                label,
                cond: test,
                body: looped,
            }
            .at(span);
            let exit: Vec<Stmt> = body.collect();
            let n = exit.len();
            stmts.splice(i + 1..i + 1, exit);
            i += n;
        }
        i += 1;
    }
}

/// Whether a `continue` in `stmts` goes on with the loop they're in, labeled
/// `label`: one of no label not inside a loop of their own, or of its label.
fn goes_on(stmts: &[Stmt], label: Option<&str>) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Continue(None) => true,
        StmtKind::Continue(Some(l)) => Some(l.as_str()) == label,
        StmtKind::If(_, a, b) => goes_on(a, label) || b.as_deref().is_some_and(|b| goes_on(b, label)),
        StmtKind::Labeled(_, body) => goes_on(body, label),
        StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
            label.is_some_and(|l| names_continue(body, l))
        }
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => goes_on(a, label) || goes_on(b, label),
        _ => false,
    })
}

/// Whether a `continue` in `stmts`, however deep, names `label`.
fn names_continue(stmts: &[Stmt], label: &str) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Continue(Some(l)) => l == label,
        StmtKind::If(_, a, b) => names_continue(a, label) || b.as_deref().is_some_and(|b| names_continue(b, label)),
        StmtKind::While { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Labeled(_, body) => names_continue(body, label),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => names_continue(a, label) || names_continue(b, label),
        _ => false,
    })
}

/// A loop's body that ends `if (c) { a; continue l; } break;`, `l` the loop
/// or none: `if (!c) { break; } a`, as a person writes an early exit.
fn early_breaks(stmts: &mut [Stmt]) {
    for stmt in stmts.iter_mut() {
        let (label, body) = match &mut stmt.kind {
            StmtKind::While { label, body, .. }
            | StmtKind::ForOf { label, body, .. }
            | StmtKind::For { label, body, .. } => (label.clone(), body),
            _ => continue,
        };
        let [.., last_if, last] = body.as_slice() else { continue };
        let continues = |s: &Stmt| match &s.kind {
            StmtKind::Continue(None) => true,
            StmtKind::Continue(Some(l)) => Some(l) == label.as_ref(),
            _ => false,
        };
        let shape = matches!(&last.kind, StmtKind::Break(None))
            && matches!(&last_if.kind, StmtKind::If(_, then, None) if then.last().is_some_and(continues));
        if !shape {
            continue;
        }
        body.pop();
        let Some(Stmt {
            kind: StmtKind::If(test, mut then, None),
            span,
        }) = body.pop()
        else {
            unreachable!("matched")
        };
        then.pop();
        body.push(StmtKind::If(not(test), vec![StmtKind::Break(None).at(span)], None).at(span));
        body.extend(then);
    }
}

/// `if (a) { if (b) { s } }`, the tests of one condition the program wrote,
/// `a && b`'s: `if (a && b) { s }`. What's bound between them, a read that
/// does nothing, comes first in `s`, and `b` reads its value, as
/// `if let`s chained with `&&` read.
fn and_tests(stmts: &mut [Stmt], conditions: &[(js::Span, js::Span)]) {
    let within = |c: &js::Span, s: js::Span| !s.is_none() && c.lo <= s.lo && s.hi <= c.hi;
    for stmt in stmts.iter_mut() {
        let span = stmt.span;
        let StmtKind::If(test, then, None) = &mut stmt.kind else {
            continue;
        };
        let [bound @ .., inner] = then.as_slice() else { continue };
        let StmtKind::If(second, _, None) = &inner.kind else {
            continue;
        };
        let one = (conditions.iter()).any(|(first, then)| within(first, span) && within(then, inner.span));
        let pure = bound
            .iter()
            .all(|s| matches!(&s.kind, StmtKind::Const(_, value) if value.reads_same()));
        if !one || !pure {
            continue;
        }
        let second = bound
            .iter()
            .rev()
            .try_fold(second.clone(), |second, stmt| match &stmt.kind {
                StmtKind::Const(name, value) => second.substitute(&|v| (v == name).then(|| value.clone())),
                _ => None,
            });
        let Some(second) = second else { continue };
        let Some(Stmt {
            kind: StmtKind::If(_, body, None),
            ..
        }) = then.pop()
        else {
            unreachable!("matched")
        };
        let first = std::mem::replace(test, Expr::undefined());
        *test = Expr::bin(Op::And, first, second);
        then.extend(body);
    }
}

/// Where a function ends, a `return;` the program didn't write, `written`
/// where those it did are: nothing, as it falls off the end there; and
/// `if (c) { s; return; } rest`, `if (c) { s } else { rest }`, as the
/// program wrote it, `if (!c) { rest }` of no `s`.
fn tail_returns(stmts: &mut Vec<Stmt>, returns: &[js::Span]) {
    let unwritten = |s: &Stmt| matches!(s.kind, StmtKind::Return(None)) && !written(returns, s.span);
    while stmts.last().is_some_and(unwritten) {
        stmts.pop();
    }
    if let Some(at) = (stmts.iter())
        .position(|s| matches!(&s.kind, StmtKind::If(_, then, None) if then.last().is_some_and(unwritten)))
    {
        let rest = stmts.split_off(at + 1);
        let Some(Stmt {
            kind: StmtKind::If(test, mut then, None),
            span,
        }) = stmts.pop()
        else {
            unreachable!("found")
        };
        then.pop();
        match (then.is_empty(), rest.is_empty()) {
            (true, true) => {}
            (true, false) => stmts.push(StmtKind::If(not(test), rest, None).at(span)),
            (false, true) => stmts.push(StmtKind::If(test, then, None).at(span)),
            (false, false) => stmts.push(StmtKind::If(test, then, Some(rest)).at(span)),
        }
    }
    // `if (c) { s } else { return; }`, a `let … else`'s `return` the program
    // wrote: `if (!c) { return; } s`, its binding first, `const b = x; if (b
    // == null) { return; }`, as THIR guards it.
    if let Some(Stmt {
        kind: StmtKind::If(_, then, Some(otherwise)),
        ..
    }) = stmts.last()
        && matches!(otherwise.as_slice(), [r] if matches!(r.kind, StmtKind::Return(None)) && written(returns, r.span))
        && !then.last().is_some_and(ends)
    {
        let Some(Stmt {
            kind: StmtKind::If(test, then, Some(otherwise)),
            span,
        }) = stmts.pop()
        else {
            unreachable!("matched")
        };
        let mut then = then.into_iter().peekable();
        // The binding of what it tests is set, first, tested itself.
        let bound = match (&test.kind, then.peek().map(|s| &s.kind)) {
            (ExprKind::Binary(Op::LooseNe, x, null), Some(StmtKind::Const(b, value)))
                if matches!(null.kind, ExprKind::Null | ExprKind::Undefined) && same(x, value) =>
            {
                Some(b.clone())
            }
            _ => None,
        };
        let test = match &bound {
            Some(b) => {
                stmts.push(then.next().expect("the binding"));
                Expr::bin(Op::LooseEq, Expr::var(b), Expr::null())
            }
            None => not(test),
        };
        stmts.push(StmtKind::If(test, otherwise, None).at(span));
        stmts.extend(then);
    }
    match stmts.last_mut().map(|s| &mut s.kind) {
        Some(StmtKind::If(_, then, otherwise)) => {
            tail_returns(then, returns);
            if let Some(branch) = otherwise {
                tail_returns(branch, returns);
            }
        }
        Some(StmtKind::Labeled(_, body)) => tail_returns(body, returns),
        _ => {}
    }
}

/// Whether `stmts` hold a `return` the program wrote, of `returns`.
fn writes_return(stmts: &[Stmt], returns: &[js::Span]) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Return(_) => written(returns, stmt.span),
        StmtKind::If(_, a, b) => writes_return(a, returns) || b.as_deref().is_some_and(|b| writes_return(b, returns)),
        _ => false,
    })
}

/// Whether a statement at `span` is a `return` the program wrote, of
/// `returns`, where those are.
fn written(returns: &[js::Span], span: js::Span) -> bool {
    !span.is_none() && returns.iter().any(|w| w.lo <= span.lo && span.hi <= w.hi)
}

/// A `match` reading a table by its key (ADR 0233), as MIR tests each
/// variant: `if (k === "a") { v = M.a; } else { v = k === "b" ? M.b : M.c; }`,
/// `v = M[k]`; of each arm its own name, `"a"`, `v = k`. And of returns,
/// `if (k === "a") { return M.a; } .. return M.c;`, `return M[k];`.
fn table_reads(stmts: &mut Vec<Stmt>, tables: &[js::Span]) {
    let within = |s: js::Span| !s.is_none() && tables.iter().any(|t| t.lo <= s.lo && s.hi <= t.hi);
    let mut i = 0;
    while i < stmts.len() {
        let returned = (stmts[i..].iter())
            .take_while(|s| {
                within(s.span)
                    && matches!(&s.kind, StmtKind::If(_, then, None)
                        if matches!(then.as_slice(), [Stmt { kind: StmtKind::Return(Some(_)), .. }]))
            })
            .count();
        if returned > 0
            && let Some(Stmt {
                kind: StmtKind::Return(Some(last)),
                ..
            }) = stmts.get(i + returned)
        {
            let mut arms: Vec<(Option<Expr>, Expr)> = (stmts[i..i + returned].iter())
                .map(|s| match &s.kind {
                    StmtKind::If(test, then, None) => match then.as_slice() {
                        [
                            Stmt {
                                kind: StmtKind::Return(Some(value)),
                                ..
                            },
                        ] => (Some(test.clone()), value.clone()),
                        _ => unreachable!("taken"),
                    },
                    _ => unreachable!("taken"),
                })
                .collect();
            value_arms(last, &mut arms);
            if let Some(read) = table_read(&arms) {
                let span = stmts[i].span;
                stmts.splice(i..=i + returned, [StmtKind::Return(Some(read)).at(span)]);
            }
        }
        i += 1;
    }
    for stmt in stmts.iter_mut() {
        if !within(stmt.span) {
            continue;
        }
        let mut arms = Vec::new();
        let target = match &stmt.kind {
            StmtKind::If(..) => assigned_arms(stmt, &mut arms),
            // Taken apart too, `const { name, classes } = ..`, as react.dev's
            // Tag takes its variant.
            StmtKind::Const(_, e)
            | StmtKind::Let(_, Some(e))
            | StmtKind::Return(Some(e))
            | StmtKind::Destructure { value: e, .. } => {
                value_arms(e, &mut arms);
                None
            }
            StmtKind::Assign(target, e) => {
                value_arms(e, &mut arms);
                Some(target.clone())
            }
            _ => continue,
        };
        let Some(read) = table_read(&arms) else { continue };
        match (&mut stmt.kind, target) {
            (StmtKind::If(..), Some(target)) => stmt.kind = StmtKind::Assign(target, read),
            (
                StmtKind::Const(_, e)
                | StmtKind::Let(_, Some(e))
                | StmtKind::Return(Some(e))
                | StmtKind::Assign(_, e)
                | StmtKind::Destructure { value: e, .. },
                _,
            ) => *e = read,
            _ => {}
        }
    }
}

/// The arms of `if (k === "a") { v = .. } else ..`, each its test and value,
/// and what each assigns, the same.
fn assigned_arms(stmt: &Stmt, arms: &mut Vec<(Option<Expr>, Expr)>) -> Option<Expr> {
    match &stmt.kind {
        StmtKind::If(test, then, Some(otherwise)) => {
            let [
                Stmt {
                    kind: StmtKind::Assign(target, value),
                    ..
                },
            ] = then.as_slice()
            else {
                return None;
            };
            arms.push((Some(test.clone()), value.clone()));
            let [rest] = otherwise.as_slice() else { return None };
            let rest = assigned_arms(rest, arms)?;
            same(&rest, target).then_some(rest)
        }
        StmtKind::Assign(target, value) => {
            value_arms(value, arms);
            Some(target.clone())
        }
        _ => None,
    }
}

/// The arms of `k === "a" ? M.a : ..`, each its test and value.
fn value_arms(e: &Expr, arms: &mut Vec<(Option<Expr>, Expr)>) {
    match &e.kind {
        ExprKind::Cond(test, value, rest) => {
            arms.push((Some((**test).clone()), (**value).clone()));
            value_arms(rest, arms);
        }
        _ => arms.push((None, e.clone())),
    }
}

/// `M[k]`, of arms each `k === "a"` giving `M.a`, the last whichever's
/// left; or `k` itself, of each giving its own name.
fn table_read(arms: &[(Option<Expr>, Expr)]) -> Option<Expr> {
    let [first, .., (None, _)] = arms else { return None };
    let (Some(first), _) = first else { return None };
    let ExprKind::Binary(Op::Eq, key, _) = &first.kind else {
        return None;
    };
    let mut table: Option<&Expr> = None;
    let mut fields = Vec::new();
    let mut names = 0;
    for (test, value) in arms {
        let name = match test {
            Some(test) => {
                let ExprKind::Binary(Op::Eq, k, name) = &test.kind else {
                    return None;
                };
                let ExprKind::Str(name) = &name.kind else { return None };
                if !same(k, key) {
                    return None;
                }
                Some(name.as_str())
            }
            None => None,
        };
        match &value.kind {
            ExprKind::Member(of, field) if name.is_none_or(|n| n == field) && !fields.contains(&field.as_str()) => {
                if table.is_some_and(|t| !same(t, of)) {
                    return None;
                }
                table = Some(of);
                fields.push(field.as_str());
            }
            ExprKind::Str(s) if name.is_none_or(|n| n == s) => names += 1,
            _ => return None,
        }
    }
    match table {
        None if names == arms.len() => Some((**key).clone()),
        Some(table) if fields.len() == arms.len() => Some(Expr::index(table.clone(), (**key).clone())),
        _ => None,
    }
}

/// `l: { if (a) { break l; } if (b) { break l; } rest } t`, of `let … else`
/// and checks that go on to one end, `t`, a `throw` or a `return`, and
/// `rest` never going on past its own: `if (a || b) { t } rest`, as a
/// person writes a guard; each guard one that tests on too, an `||` of
/// `&&`s the program tests (`break_test`).
fn guarded(stmts: &mut Vec<Stmt>) {
    for i in 0..stmts.len() {
        let StmtKind::Labeled(label, body) = &stmts[i].kind else {
            continue;
        };
        let tail = &stmts[i + 1..];
        let guards = (body.iter()).take_while(|s| break_test(s, label).is_some()).count();
        let rest = &body[guards..];
        if !tail.last().is_some_and(ends) {
            continue;
        }
        if guards == 0 || targets(rest, label) || !rest.last().is_some_and(ends) {
            continue;
        }
        // What `rest` declares, now beside what's before it.
        if declared(rest).any(|name| declared(&stmts[..i]).any(|n| n == name)) {
            continue;
        }
        let test = (body[..guards].iter())
            .map(|s| break_test(s, label).expect("a guard"))
            .reduce(|a, b| Expr::bin(Op::Or, a, b))
            .expect("one");
        let span = stmts[i].span;
        let rest = rest.to_vec();
        let tail = stmts.split_off(i + 1);
        stmts.pop();
        stmts.push(StmtKind::If(test, tail, None).at(span));
        stmts.extend(rest);
        return;
    }
}

/// When `stmt` breaks out to `label`, of `if`s that do or test on:
/// `if (k) { if (!m) { if (c) { break l; } } else { break l; } }`'s `k && (m
/// || c)`.
fn break_test(stmt: &Stmt, label: &str) -> Option<Expr> {
    let StmtKind::If(test, then, otherwise) = &stmt.kind else {
        return None;
    };
    let breaks = |s: &[Stmt]| matches!(s, [Stmt { kind: StmtKind::Break(Some(l)), .. }] if l == label);
    let inner = |s: &[Stmt]| match s {
        [one] => break_test(one, label),
        _ => None,
    };
    match otherwise.as_deref() {
        None if breaks(then) => Some(test.clone()),
        None => Some(Expr::bin(Op::And, test.clone(), inner(then)?)),
        Some(otherwise) if breaks(otherwise) => Some(Expr::bin(Op::Or, not(test.clone()), inner(then)?)),
        Some(_) => None,
    }
}

/// A loop's body that ends in a block, `l: { … break l; … }`, its `break l`
/// the loop's `continue`: the block's statements in its place. And one that
/// ends `if (!c) { a; b }`, as MIR has a `continue` the program wrote,
/// `if (c) { continue; } a; b`.
fn continued(stmts: &mut [Stmt], guards: &[js::Span]) {
    for stmt in stmts.iter_mut() {
        let (looped, body) = match &mut stmt.kind {
            StmtKind::While { label, body, .. }
            | StmtKind::ForOf { label, body, .. }
            | StmtKind::For { label, body, .. } => (label.clone(), body),
            _ => continue,
        };
        loop {
            match body.last().map(|s| &s.kind) {
                Some(StmtKind::Labeled(..)) => {
                    let Some(Stmt {
                        kind: StmtKind::Labeled(block, mut inner),
                        span,
                    }) = body.pop()
                    else {
                        unreachable!("matched")
                    };
                    // Its branches first, `if (c) { s; break l; } t` an `else`,
                    // as `branch_ends` has them: a `continue` the program
                    // wrote, which ends one, comes back (below).
                    ending(&mut inner, &block);
                    let clashes = declared(&inner).any(|name| declared(body).any(|n| n == name));
                    if clashes || continues(&mut inner, &block, looped.as_deref(), false).is_none() {
                        body.push(StmtKind::Labeled(block, inner).at(span));
                        break;
                    }
                    continues(&mut inner, &block, looped.as_deref(), true);
                    body.extend(inner);
                }
                // One the program ended with a `continue`, `if (t) { a;
                // continue; }`, of what MIR has as its `else`.
                Some(StmtKind::If(_, _, Some(otherwise)))
                    if written(guards, body.last().expect("matched").span)
                        && !declared(otherwise).any(|name| declared(&body[..body.len() - 1]).any(|n| n == name)) =>
                {
                    let Some(Stmt {
                        kind: StmtKind::If(test, mut then, Some(otherwise)),
                        span,
                    }) = body.pop()
                    else {
                        unreachable!("matched")
                    };
                    if !then.last().is_some_and(ends) {
                        then.push(StmtKind::Continue(None).at(span));
                    }
                    body.push(StmtKind::If(test, then, None).at(span));
                    body.extend(otherwise);
                }
                // Of one statement, where the program wrote the `continue`,
                // whatever its test, `i % 2 !== 0` of `i % 2 == 0`.
                Some(StmtKind::If(test, then, None))
                    if written(guards, body.last().expect("matched").span) || (negated(test) && then.len() > 1) =>
                {
                    let Some(Stmt {
                        kind: StmtKind::If(test, then, None),
                        span,
                    }) = body.pop()
                    else {
                        unreachable!("matched")
                    };
                    body.push(StmtKind::If(not(test), vec![StmtKind::Continue(None).at(span)], None).at(span));
                    body.extend(then);
                }
                _ => break,
            }
        }
        // A `continue` where the body ends anyway, in the branches that end
        // it: none.
        fn ends_going_on(stmts: &mut Vec<Stmt>, looped: &Option<String>) {
            let more = stmts.len() > 1;
            match stmts.last_mut().map(|s| &mut s.kind) {
                Some(StmtKind::Continue(l)) if (l.is_none() || l == looped) && more => {
                    stmts.pop();
                }
                Some(StmtKind::If(_, then, otherwise)) => {
                    ends_going_on(then, looped);
                    if let Some(otherwise) = otherwise {
                        ends_going_on(otherwise, looped);
                    }
                }
                _ => {}
            }
        }
        ends_going_on(body, &looped);
    }
}

/// A `continue l` or a `break l` of loop `l` where it's the innermost
/// loop: `continue` or `break`, as JS's unlabeled ones go there.
fn innermost_jumps(stmts: &mut [Stmt]) {
    fn unlabel(stmts: &mut [Stmt], looped: &str) {
        for stmt in stmts {
            match &mut stmt.kind {
                StmtKind::Continue(label) | StmtKind::Break(label) if label.as_deref() == Some(looped) => *label = None,
                StmtKind::If(_, a, b) => {
                    unlabel(a, looped);
                    if let Some(b) = b {
                        unlabel(b, looped);
                    }
                }
                StmtKind::Labeled(_, body) => unlabel(body, looped),
                StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                    unlabel(a, looped);
                    unlabel(b, looped);
                }
                _ => {}
            }
        }
    }
    for stmt in stmts {
        if let StmtKind::While {
            label: Some(label),
            body,
            ..
        }
        | StmtKind::ForOf {
            label: Some(label),
            body,
            ..
        }
        | StmtKind::For {
            label: Some(label),
            body,
            ..
        } = &mut stmt.kind
        {
            unlabel(body, label);
        }
    }
}

/// Each `break block` in `stmts` a `continue` of the loop around, labeled
/// `looped`, written if `write`: `None` where one inside a loop of its own
/// would need the loop's label, and it has none.
fn continues(stmts: &mut [Stmt], block: &str, looped: Option<&str>, write: bool) -> Option<()> {
    fn each(stmts: &mut [Stmt], block: &str, looped: Option<&str>, inner: bool, write: bool) -> Option<()> {
        for stmt in stmts {
            match &mut stmt.kind {
                StmtKind::Break(Some(l)) if l == block => {
                    let label = match inner {
                        true => Some(looped?.to_string()),
                        false => None,
                    };
                    if write {
                        stmt.kind = StmtKind::Continue(label);
                    }
                }
                StmtKind::If(_, a, b) => {
                    each(a, block, looped, inner, write)?;
                    if let Some(b) = b {
                        each(b, block, looped, inner, write)?;
                    }
                }
                StmtKind::Labeled(_, body) => each(body, block, looped, inner, write)?,
                StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                    each(a, block, looped, inner, write)?;
                    each(b, block, looped, inner, write)?;
                }
                StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
                    each(body, block, looped, true, write)?
                }
                _ => {}
            }
        }
        Some(())
    }
    // Not `looped` itself, a label the block's statements can't name twice.
    if looped == Some(block) {
        return None;
    }
    each(stmts, block, looped, false, write)
}

/// Whether `test` says `item`, what `$next` gave, is none: `item == null`,
/// or `!item` of an array's.
fn absent(test: &Expr, item: &str) -> bool {
    match &test.kind {
        ExprKind::Binary(Op::LooseEq, a, b) => {
            matches!(&a.kind, ExprKind::Var(v) if v == item) && matches!(b.kind, ExprKind::Null | ExprKind::Undefined)
        }
        ExprKind::Unary(UnaryOp::Not, a) => matches!(&a.kind, ExprKind::Var(v) if v == item),
        _ => false,
    }
}

/// Whether `test` says `item` is there: `item != null`, or `item`.
fn present(test: &Expr, item: &str) -> bool {
    match &test.kind {
        ExprKind::Binary(Op::LooseNe, a, b) => {
            matches!(&a.kind, ExprKind::Var(v) if v == item) && matches!(b.kind, ExprKind::Null | ExprKind::Undefined)
        }
        ExprKind::Var(v) => v == item,
        _ => false,
    }
}

/// What a `for` binds of `item`, read by the first statements of `body`:
/// `const x = item;`, or `const a = item[0]; const b = item[1];`, its
/// pattern, whether it's a `let`, and how many statements; `_` where nothing
/// reads it.
fn binding(body: &[Stmt], item: &str) -> Option<(js::Pattern, bool, usize)> {
    let reads = |value: &Expr| -> Option<Result<Option<usize>, String>> {
        match &value.kind {
            ExprKind::Var(v) if v == item => Some(Ok(None)),
            ExprKind::Index(of, index)
                if matches!(&of.kind, ExprKind::Var(v) if v == item)
                    && let ExprKind::Num(k) = index.kind
                    && k >= 0.0
                    && k.fract() == 0.0 =>
            {
                Some(Ok(Some(k as usize)))
            }
            ExprKind::Member(of, key) if matches!(&of.kind, ExprKind::Var(v) if v == item) => Some(Err(key.clone())),
            _ => None,
        }
    };
    let mut parts = Vec::new();
    for stmt in body {
        let (bound, value, mutable) = match &stmt.kind {
            StmtKind::Const(bound, value) => (bound, value, false),
            StmtKind::Let(bound, Some(value)) => (bound, value, true),
            _ => break,
        };
        match reads(value) {
            Some(part) => parts.push((part, bound.clone(), mutable)),
            None => break,
        }
    }
    let mutable = parts.iter().any(|&(_, _, m)| m);
    match parts.as_slice() {
        [] if js::mentions_in(body, item) == 0 => Some((js::Pattern::Name("_".to_string()), false, 0)),
        [(Ok(None), bound, mutable)] => Some((js::Pattern::Name(bound.clone()), *mutable, 1)),
        _ if !parts.is_empty() && parts.iter().all(|(p, ..)| matches!(p, Ok(Some(_)))) => {
            let last = parts.iter().filter_map(|(p, ..)| p.clone().ok().flatten()).max()?;
            let mut items = vec![None; last + 1];
            for (part, bound, _) in &parts {
                items[part.clone().ok().flatten()?] = Some(bound.clone());
            }
            Some((js::Pattern::names_array(items), mutable, parts.len()))
        }
        _ if !parts.is_empty() && parts.iter().all(|(p, ..)| p.is_err()) => {
            let fields = (parts.iter())
                .map(|(p, bound, _)| (p.clone().expect_err("keyed"), js::Pattern::Name(bound.clone()), None))
                .collect();
            Some((js::Pattern::Object(fields, None), mutable, parts.len()))
        }
        _ => None,
    }
}

/// Whether a `break` in `stmts` leaves the loop they're the body of, labeled
/// `label`: one of no label not inside a loop of their own, or of its label.
fn breaks(stmts: &[Stmt], label: Option<&str>) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Break(None) => true,
        StmtKind::Break(Some(l)) => Some(l.as_str()) == label,
        StmtKind::If(_, a, b) => breaks(a, label) || b.as_deref().is_some_and(|b| breaks(b, label)),
        StmtKind::Labeled(_, body) => breaks(body, label),
        // In a loop of its own, one that names it.
        StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
            label.is_some_and(|l| names_break(body, l))
        }
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => breaks(a, label) || breaks(b, label),
        _ => false,
    })
}

/// Whether a `break` in `stmts`, however deep, names `label`.
fn names_break(stmts: &[Stmt], label: &str) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Break(Some(l)) => l == label,
        StmtKind::If(_, a, b) => names_break(a, label) || b.as_deref().is_some_and(|b| names_break(b, label)),
        StmtKind::While { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Labeled(_, body) => names_break(body, label),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => names_break(a, label) || names_break(b, label),
        _ => false,
    })
}

/// Whether `stmts` read `name` alone, `name[i]` or `name.length`: not
/// assign it, call a method of it, or give it to what could change it.
fn only_read(stmts: &[Stmt], name: &str) -> bool {
    let mut stmts = stmts.to_vec();
    let (mut all, mut read) = (0, 0);
    js::each_expr_mut(&mut stmts, &mut |e| match &e.kind {
        ExprKind::Var(v) if v == name => all += 1,
        ExprKind::Index(of, _) | ExprKind::Member(of, _)
            if matches!(&of.kind, ExprKind::Var(v) if v == name)
                && !matches!(&e.kind, ExprKind::Member(_, m) if m != "length") =>
        {
            read += 1
        }
        _ => {}
    });
    all == read && !assigns(&stmts, name)
}

/// Whether `stmts` assign `name`.
fn assigns(stmts: &[Stmt], name: &str) -> bool {
    let mut stmts = stmts.to_vec();
    let mut assigned = false;
    js::statement_lists(&mut stmts, &mut |list| {
        assigned |= list.iter().any(|s| {
            matches!(&s.kind, StmtKind::Assign(target, _)
            if matches!(&target.kind, ExprKind::Var(v) if v == name))
        });
    });
    assigned
}

/// Whether `name` is a temporary MIR made, `_5`, not one the program names.
/// Whether `name` is MIR's lowering's of an argument it holds, `_arg`,
/// named once tidied.
fn is_held_arg(name: &str) -> bool {
    name.split('$').next() == Some("_arg")
}

fn is_temporary(name: &str) -> bool {
    name.strip_prefix('_')
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit() || c == '$'))
}

/// `const _5 = children;`, a temporary of a variable nothing changes, nor it:
/// `children` wherever it's read.
fn alias_variables(body: &mut Vec<Stmt>, changing: &HashSet<String>) {
    let mut aliases: Vec<(String, Expr)> = Vec::new();
    js::statement_lists(body, &mut |stmts| {
        stmts.retain(|stmt| match &stmt.kind {
            StmtKind::Const(name, value)
                if is_temporary(name)
                    && !changing.contains(name)
                    && matches!(&value.kind, ExprKind::Var(of) if !changing.contains(of)) =>
            {
                aliases.push((name.clone(), value.clone()));
                false
            }
            _ => true,
        });
    });
    if aliases.is_empty() {
        return;
    }
    // An alias of an alias, its variable or constant.
    let of = |name: &str| {
        let mut value = Expr::var(name);
        while let ExprKind::Var(name) = &value.kind
            && let Some((_, to)) = aliases.iter().find(|(from, _)| from == name)
        {
            value = to.clone();
        }
        value
    };
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Var(name) = &e.kind
            && aliases.iter().any(|(from, _)| from == name)
        {
            *e = Expr {
                span: e.span,
                ..of(name)
            };
        }
    });
}

/// `const _5 = e; const a = _5[0]; const b = _5[1];`, the temporary read
/// nowhere else, is `const [a, b] = e;`; and of its properties, `const { a,
/// b: c } = e;`; `let` if one of them is. Of a variable the program names,
/// two or more parts in a row, `let (a, b) = t;`'s: `const [a, b] = t;`;
/// one too of a `let`'s pattern, `patterns`, `const { message } = error;`.
fn destructured(stmts: &mut Vec<Stmt>, patterns: &HashSet<String>) {
    let mut i = 0;
    while i < stmts.len() {
        // What the parts are read of, and where they start: a temporary this
        // statement makes, the statements after it; or a variable this one
        // reads a part of, here.
        let (name, first) = match &stmts[i].kind {
            StmtKind::Const(name, _) if is_temporary(name) => (name.clone(), i + 1),
            StmtKind::Const(_, value) | StmtKind::Let(_, Some(value))
                if let Some(v) = part_of(value)
                    && !is_temporary(v) =>
            {
                (v.to_string(), i)
            }
            _ => {
                i += 1;
                continue;
            }
        };
        let made = first > i;
        let parts = parts(&stmts[first..], &name);
        let count = parts.len();
        let indexed = parts.iter().all(|(p, ..)| p.is_ok());
        let bound = parts.iter().all(|(_, name, ..)| patterns.contains(name));
        let enough = count >= if made || bound { 1 } else { 2 };
        let pattern = pattern_of(&parts);
        if !enough || pattern.is_none() || (made && js::mentions_in(&stmts[first + count..], &name) > 0) {
            i += 1;
            continue;
        }
        // Of an array made here, `const [, b] = [x, 6]`: each part itself,
        // where what no part reads does nothing.
        if made
            && indexed
            && let StmtKind::Const(_, value) = &stmts[i].kind
            && let ExprKind::Array(items) = &value.kind
            && parts.windows(2).all(|w| w[0].0 < w[1].0)
            && parts.iter().all(|(p, ..)| matches!(p, Ok(k) if *k < items.len()))
            && (items.iter().enumerate()).all(|(k, item)| {
                parts.iter().any(|(p, ..)| *p == Ok(k)) || item.is_constant() || matches!(item.kind, ExprKind::Var(_))
            })
        {
            let span = stmts[i].span;
            let each: Vec<Stmt> = (parts.iter())
                .map(|(part, bound, mutable, _)| {
                    let item = items[*part.as_ref().expect("indexed")].clone();
                    match mutable {
                        true => StmtKind::Let(bound.clone(), Some(item)),
                        false => StmtKind::Const(bound.clone(), item),
                    }
                    .at(span)
                })
                .collect();
            stmts.splice(i..first + count, each);
            continue;
        }
        let mutable = parts.iter().any(|(_, _, m, _)| *m);
        let pattern = pattern.expect("checked");
        let value = match made {
            true => {
                stmts.drain(first..first + count);
                let StmtKind::Const(_, value) = std::mem::replace(&mut stmts[i].kind, StmtKind::Break(None)) else {
                    unreachable!("matched")
                };
                value
            }
            false => {
                stmts.drain(i + 1..i + count);
                Expr::var(&name)
            }
        };
        stmts[i].kind = StmtKind::Destructure {
            pattern,
            value,
            mutable,
        };
        i += 1;
    }
}

/// A `let`'s pattern's rest, which MIR's lowering takes apart with each
/// other field named apart, `const { href: _href, ...rest } = p;`, beside
/// what the pattern binds of the same, `const { href } = p;`: one pattern,
/// `const { href, ...rest } = p;`, as it's written.
fn rest_parts(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        let StmtKind::Destructure {
            pattern: js::Pattern::Object(fields, Some(_)),
            value,
            ..
        } = &stmts[i].kind
        else {
            i += 1;
            continue;
        };
        let ExprKind::Var(of) = &value.kind else {
            i += 1;
            continue;
        };
        let parts_at = |at: usize| match stmts.get(at).map(|s| &s.kind) {
            Some(StmtKind::Destructure {
                pattern: js::Pattern::Object(parts, None),
                value,
                mutable,
            }) if matches!(&value.kind, ExprKind::Var(v) if v == of)
                && parts.iter().all(|(key, _, _)| fields.iter().any(|(k, _, _)| k == key)) =>
            {
                Some((parts.clone(), *mutable))
            }
            _ => None,
        };
        let (at, (parts, mutable)) = match (i.checked_sub(1).and_then(parts_at), parts_at(i + 1)) {
            (Some(before), _) => (i - 1, before),
            (None, Some(after)) => (i + 1, after),
            (None, None) => {
                i += 1;
                continue;
            }
        };
        stmts.remove(at);
        let i = if at < i { i - 1 } else { i };
        if let StmtKind::Destructure {
            pattern: js::Pattern::Object(fields, _),
            mutable: m,
            ..
        } = &mut stmts[i].kind
        {
            for (key, part, default) in parts {
                if let Some(field) = fields.iter_mut().find(|(k, _, _)| *k == key) {
                    *field = (key, part, default);
                }
            }
            *m |= mutable;
        }
    }
}

/// A part of a value read into a variable: its index or key, the variable,
/// and whether it's a `let`.
/// And a key's default, `{ size = "md" }`'s.
type Part = (Result<usize, String>, String, bool, Option<Expr>);

/// The parts of `name` the first of `stmts` read, each a `const` or a
/// `let` of one, `const a = name[0];` or `const b = name.b;`.
fn parts(stmts: &[Stmt], name: &str) -> Vec<Part> {
    let mut parts = Vec::new();
    for stmt in stmts {
        let (bound, value, mutable) = match &stmt.kind {
            StmtKind::Const(bound, value) => (bound, value, false),
            StmtKind::Let(bound, Some(value)) => (bound, value, true),
            _ => break,
        };
        if bound == name {
            break;
        }
        let key = |e: &Expr| match &e.kind {
            ExprKind::Member(of, key) if matches!(&of.kind, ExprKind::Var(v) if v == name) => Some(key.clone()),
            _ => None,
        };
        let (part, default) = match &value.kind {
            ExprKind::Index(of, index)
                if matches!(&of.kind, ExprKind::Var(v) if v == name)
                    && let ExprKind::Num(k) = index.kind
                    && k >= 0.0
                    && k.fract() == 0.0 =>
            {
                (Ok(k as usize), None)
            }
            ExprKind::Member(..) if let Some(key) = key(value) => (Err(key), None),
            // A field with a default, `o.k === undefined ? d : o.k`.
            ExprKind::Cond(test, default, read)
                if let ExprKind::Binary(Op::Eq, tested, undefined) = &test.kind
                    && matches!(undefined.kind, ExprKind::Undefined)
                    && let Some(k) = key(read)
                    && key(tested).is_some_and(|t| t == k) =>
            {
                (Err(k), Some((**default).clone()))
            }
            _ => break,
        };
        parts.push((part, bound.clone(), mutable, default));
    }
    parts
}

/// The variable `value` reads a part of, as `parts` reads one: `o[0]`,
/// `o.k`, or `o.k === undefined ? d : o.k`.
fn part_of(value: &Expr) -> Option<&str> {
    let of = match &value.kind {
        ExprKind::Index(of, _) | ExprKind::Member(of, _) => of,
        ExprKind::Cond(_, _, read) => match &read.kind {
            ExprKind::Member(of, _) => of,
            _ => return None,
        },
        _ => return None,
    };
    match &of.kind {
        ExprKind::Var(v) => Some(v),
        _ => None,
    }
}

/// What takes `parts` apart, `[a, , c]` of indices, `{ a, b: c }` of keys:
/// none of none, or of both.
fn pattern_of(parts: &[Part]) -> Option<js::Pattern> {
    if parts.is_empty() {
        return None;
    }
    if parts.iter().all(|(p, ..)| p.is_ok()) {
        let last = parts.iter().filter_map(|(p, ..)| p.as_ref().ok()).max().copied()?;
        let mut items = vec![None; last + 1];
        for (part, bound, ..) in parts {
            items[*part.as_ref().ok()?] = Some(bound.clone());
        }
        return Some(js::Pattern::names_array(items));
    }
    let keyed: Option<Vec<_>> = (parts.iter())
        .map(|(part, bound, _, default)| Some((part.clone().err()?, js::Pattern::Name(bound.clone()), default.clone())))
        .collect();
    Some(js::Pattern::Object(keyed?, None))
}

/// A parameter MIR names, `_2`, whose parts the body reads where it starts
/// and reads nowhere else, `(_2) => { const i = _2[0]; const c = _2[1]; … }`:
/// taken apart where it's given, `([i, c]) => …`, as THIR writes a pattern.
pub(super) fn destructured_params(params: &mut [js::Pattern], body: &mut Vec<Stmt>) {
    // An `async fn`'s too, `__arg0`, which its body's `let` takes apart,
    // of the copy its body holds, `__arg0$1`.
    let unnamed = |name: &str| {
        let arg = name.split('$').next().and_then(|n| n.strip_prefix("__arg"));
        is_temporary(name) || arg.is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    };
    for param in params.iter_mut() {
        let js::Pattern::Name(name) = param else { continue };
        if !unnamed(name) {
            continue;
        }
        let mut name = name.clone();
        // `(_2) => { const n = _2; … }`, `|&n|`'s: `(n) => …`.
        while let Some(Stmt {
            kind: StmtKind::Const(bound, value),
            ..
        }) = body.first()
            && matches!(&value.kind, ExprKind::Var(v) if *v == name)
            && js::mentions_in(&body[1..], &name) == 0
        {
            name = bound.clone();
            *param = js::Pattern::Name(name.clone());
            body.remove(0);
        }
        if !unnamed(&name) {
            continue;
        }
        // Taken apart already, `const { params } = __arg0;`: that pattern.
        if let Some(Stmt {
            kind:
                StmtKind::Destructure {
                    pattern,
                    value,
                    mutable: false,
                },
            ..
        }) = body.first()
            && matches!(&value.kind, ExprKind::Var(v) if *v == name)
            && js::mentions_in(&body[1..], &name) == 0
        {
            *param = pattern.clone();
            body.remove(0);
            continue;
        }
        let read = nested_parts(body, &name);
        let count = read.len();
        if js::mentions_in(&body[count..], &name) > 0 {
            continue;
        }
        let read: Vec<_> = (read.iter())
            .map(|(path, bound, default)| (path.as_slice(), bound.as_str(), default.as_ref()))
            .collect();
        let Some(pattern) = nested_pattern(&read) else { continue };
        body.drain(..count);
        *param = pattern;
    }
}

/// `if (t) { return a; } return b;`, MIR's of a value the program wrote
/// with `&&` or `||` where `t` is tested: one value returned, `t ? a : b`,
/// as the program wrote it, each test before it folded in, `s !== "en" &&
/// (t ? a : b)`; not where it wrote a `return`.
fn logical_returns(stmts: &mut Vec<Stmt>, logicals: &[js::Span], returns: &[js::Span], booleans: &HashSet<String>) {
    let within = |s: js::Span| !s.is_none() && logicals.iter().any(|l| l.lo <= s.lo && s.hi <= l.hi);
    while let [.., last_if, last] = stmts.as_slice()
        && let StmtKind::Return(Some(no)) = &last.kind
        && let StmtKind::If(test, then, None) = &last_if.kind
        && let [
            Stmt {
                kind: StmtKind::Return(Some(yes)),
                span: yes_span,
            },
        ] = then.as_slice()
        && within(last_if.span)
        && !written(returns, last.span)
        && !written(returns, *yes_span)
    {
        let returned = StmtKind::Return(Some(conditional(test.clone(), yes.clone(), no.clone(), booleans)));
        let span = last_if.span;
        stmts.splice(stmts.len() - 2.., [returned.at(span)]);
    }
}

/// `$iter(xs).filter(f)`, an array's `$iter`, which knows where it is, made
/// for an iterator helper alone, which nothing else steps: `xs.values()
/// .filter(f)`, the array's own iterator.
fn array_values(e: &mut Expr) {
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
    // The rest of a string's `chars()` not stepped, `$restStr($iter(cs))`:
    // all of them, `cs.join("")`, as THIR's.
    if let ExprKind::Call(callee, args) = &e.kind
        && matches!(&callee.kind, ExprKind::Var(f) if f == "$restStr")
        && let [it] = &args[..]
        && crate::lower::iterators::is_stepped_items(it)
        && let ExprKind::Call(_, items) = &it.kind
        && let [items] = &items[..]
    {
        *e = Expr {
            span: e.span,
            ..Expr::call(Expr::member(items.clone(), "join"), vec![Expr::str("")])
        };
        return;
    }
    // Each item with its index, `enumerate()`'s `.map((x, i) => [i, x])`:
    // the array's `entries()`, as THIR's.
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
fn pairs_index(f: &Expr) -> bool {
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

/// `let t; try { t = a; } catch { t = b; } x = t;`, a temporary each way
/// through a statement gives its value last, then given to a variable: the
/// variable given it there, as THIR gives a match's value, where the
/// statement reads the variable nowhere.
fn forwarded_values(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i + 2 < stmts.len() {
        if let StmtKind::Let(t, None) = &stmts[i].kind
            && is_temporary(t)
            && let StmtKind::Assign(target, value) = &stmts[i + 2].kind
            && let ExprKind::Var(x) = &target.kind
            && matches!(&value.kind, ExprKind::Var(v) if v == t)
            && js::mentions_in(&stmts[i + 1..=i + 1], x) == 0
            && js::mentions_in(&stmts[i + 3..], t) == 0
            && last_assigns(&stmts[i + 1..=i + 1], t)
        {
            let (t, x) = (t.clone(), x.clone());
            let mut given = vec![stmts.remove(i + 1)];
            js::statement_lists(&mut given, &mut |stmts| {
                for stmt in stmts.iter_mut() {
                    if let StmtKind::Assign(target, _) = &mut stmt.kind
                        && matches!(&target.kind, ExprKind::Var(v) if *v == t)
                    {
                        *target = Expr::var(&x);
                    }
                }
            });
            stmts.splice(i..i + 2, given);
            continue;
        }
        i += 1;
    }
}

/// Whether each way through `stmts` that goes on past them gives `t` its
/// value last, `t = e` of what doesn't read it, in a branch of an `if` or
/// a `try` that ends them, and nothing else names `t`.
fn last_assigns(stmts: &[Stmt], t: &str) -> bool {
    let names = |s: &Stmt| js::mentions_in(std::slice::from_ref(s), t) > 0;
    let Some((last, before)) = stmts.split_last() else {
        return false;
    };
    if before.iter().any(names) {
        return false;
    }
    if !names(last) {
        return ends(last);
    }
    match &last.kind {
        StmtKind::Assign(target, value) => matches!(&target.kind, ExprKind::Var(v) if v == t) && !value.mentions_var(t),
        StmtKind::If(test, then, otherwise) => {
            !test.mentions_var(t) && last_assigns(then, t) && otherwise.as_deref().is_some_and(|o| last_assigns(o, t))
        }
        StmtKind::TryCatch(body, _, caught) => last_assigns(body, t) && last_assigns(caught, t),
        _ => false,
    }
}

/// `const t = q.title;` of plain data, which nothing changes, held by MIR,
/// a temporary or what an element reads: `q.title` where `t` is read, each
/// time, as THIR reads it.
fn plain_reads(stmts: &mut Vec<Stmt>, plain: &HashSet<String>, captured: &HashSet<String>) {
    let mut i = 0;
    while i < stmts.len() {
        let read = match &stmts[i].kind {
            StmtKind::Const(name, value) if is_temporary(name) || captured.contains(name) => {
                let mut root = value;
                while let ExprKind::Member(of, _) = &root.kind {
                    root = of;
                }
                (matches!(value.kind, ExprKind::Member(..))
                    && (matches!(&root.kind, ExprKind::Var(v) if plain.contains(v))
                        || field_path(value).is_some_and(|path| plain.contains(&path))))
                .then(|| (name.clone(), value.clone()))
            }
            _ => None,
        };
        let Some((name, value)) = read else {
            i += 1;
            continue;
        };
        stmts.remove(i);
        js::each_expr_mut(&mut stmts[i..], &mut |e| {
            if matches!(&e.kind, ExprKind::Var(v) if *v == name) {
                *e = Expr {
                    span: e.span,
                    ..value.clone()
                };
            }
        });
    }
}

/// A step into a value, `[1]` or `.name`.
#[derive(PartialEq)]
enum Step {
    At(usize),
    Key(String),
}

/// The steps from `name` to what `e` reads, `[At(1), Key("name")]` of
/// `_2[1].name`.
fn path(e: &Expr, name: &str) -> Option<Vec<Step>> {
    let (of, step) = match &e.kind {
        ExprKind::Var(v) if v == name => return Some(Vec::new()),
        ExprKind::Index(of, index)
            if let ExprKind::Num(k) = index.kind
                && k >= 0.0
                && k.fract() == 0.0 =>
        {
            (of, Step::At(k as usize))
        }
        ExprKind::Member(of, key) => (of, Step::Key(key.clone())),
        _ => return None,
    };
    let mut steps = path(of, name)?;
    steps.push(step);
    Some(steps)
}

/// The parts of `name` the first of `stmts` read, as `parts` finds them,
/// however deep, `const name = _2[1].name;`, each a path, its variable,
/// and a field's default.
fn nested_parts(stmts: &[Stmt], name: &str) -> Vec<(Vec<Step>, String, Option<Expr>)> {
    let mut parts = Vec::new();
    for stmt in stmts {
        let (StmtKind::Const(bound, value) | StmtKind::Let(bound, Some(value))) = &stmt.kind else {
            break;
        };
        if bound == name {
            break;
        }
        let (steps, default) = match &value.kind {
            // A field with a default, `o.k === undefined ? d : o.k`.
            ExprKind::Cond(test, default, read)
                if let ExprKind::Binary(Op::Eq, tested, undefined) = &test.kind
                    && matches!(undefined.kind, ExprKind::Undefined)
                    && let Some(steps) = path(read, name)
                    && matches!(steps.last(), Some(Step::Key(_)))
                    && path(tested, name).as_ref() == Some(&steps) =>
            {
                (steps, Some((**default).clone()))
            }
            _ => match path(value, name) {
                Some(steps) if !steps.is_empty() => (steps, None),
                _ => break,
            },
        };
        parts.push((steps, bound.clone(), default));
    }
    parts
}

/// The pattern of `parts`, each a path, its variable and its default: of
/// the first steps all indices, an array, all keys, an object, each part
/// a variable or, where the paths go on, a pattern in turn, `[i, { name
/// }]`.
fn nested_pattern(parts: &[(&[Step], &str, Option<&Expr>)]) -> Option<js::Pattern> {
    type Group<'p> = Vec<(&'p [Step], &'p str, Option<&'p Expr>)>;
    let mut groups: Vec<(&Step, Group)> = Vec::new();
    for &(steps, bound, default) in parts {
        let (first, rest) = steps.split_first()?;
        match groups.iter_mut().find(|(step, _)| *step == first) {
            Some((_, group)) => group.push((rest, bound, default)),
            None => groups.push((first, vec![(rest, bound, default)])),
        }
    }
    let part = |group: &Group| match group.as_slice() {
        [([], bound, default)] => Some((js::Pattern::Name(bound.to_string()), default.cloned())),
        _ if group.iter().all(|(rest, ..)| !rest.is_empty()) => Some((nested_pattern(group)?, None)),
        _ => None,
    };
    if groups.iter().all(|(step, _)| matches!(step, Step::At(_))) {
        let last = (groups.iter()).filter_map(|(step, _)| match step {
            Step::At(k) => Some(*k),
            Step::Key(_) => None,
        });
        let mut items = vec![None; last.max()? + 1];
        for (step, group) in &groups {
            let (Step::At(k), (pattern, None)) = (step, part(group)?) else {
                return None;
            };
            items[*k] = Some(pattern);
        }
        return Some(js::Pattern::Array(items));
    }
    let fields = (groups.iter())
        .map(|(step, group)| match step {
            Step::Key(key) => {
                let (pattern, default) = part(group)?;
                Some((key.clone(), pattern, default))
            }
            Step::At(_) => None,
        })
        .collect::<Option<_>>()?;
    Some(js::Pattern::Object(fields, None))
}

/// `const _3 = e; … if (_3) { const current = _3; … }`: a temporary that a
/// variable the program names takes, in a block inside where it's read,
/// `if let Some(current) = r.current()`'s, where nothing else is named so:
/// `const current = e;`, read by that name, as THIR binds it.
fn named_temporaries(body: &mut Vec<Stmt>) {
    // How often each name is declared, and read, in the whole body.
    let mut declared: HashMap<String, usize> = HashMap::new();
    js::statement_lists(&mut body.clone(), &mut |stmts| {
        for stmt in stmts.iter() {
            let names = match &stmt.kind {
                StmtKind::Const(name, _) | StmtKind::Let(name, _) => vec![name.as_str()],
                StmtKind::Destructure { pattern, .. } => pattern.names(),
                _ => Vec::new(),
            };
            for name in names {
                *declared.entry(name.to_string()).or_default() += 1;
            }
        }
    });
    let mut reads: HashMap<String, usize> = HashMap::new();
    js::visit_stmts(body, &mut |name| *reads.entry(name.to_string()).or_default() += 1);
    js::statement_lists(body, &mut |stmts| {
        // What it takes goes from after it: the list shortens.
        let mut next = 0;
        while next < stmts.len() {
            let i = next;
            next += 1;
            let StmtKind::Const(a, _) = &stmts[i].kind else {
                continue;
            };
            if !is_temporary(a) {
                continue;
            }
            let a = a.clone();
            let mut found = Vec::new();
            taking(&stmts[i + 1..], &a, &mut found);
            let [(b, after)] = found.as_slice() else { continue };
            if declared.get(b) != Some(&1) || reads.get(b).copied().unwrap_or(0) != *after {
                continue;
            }
            let b = b.clone();
            dropping(stmts, &a, &b);
            let StmtKind::Const(name, _) = &mut stmts[i].kind else {
                unreachable!("matched")
            };
            *name = b.clone();
            js::each_expr_mut(&mut stmts[i + 1..], &mut |e| {
                if let ExprKind::Var(v) = &mut e.kind
                    && *v == a
                {
                    *v = b.clone();
                }
            });
        }
    });
}

/// Each `const b = a;` of a name the program gives in the blocks of
/// `stmts`, not in their closures, and how often `b` is read after it.
fn taking(stmts: &[Stmt], a: &str, found: &mut Vec<(String, usize)>) {
    for (j, stmt) in stmts.iter().enumerate() {
        match &stmt.kind {
            StmtKind::Const(b, value) if !is_temporary(b) && matches!(&value.kind, ExprKind::Var(v) if v == a) => {
                found.push((b.clone(), js::mentions_in(&stmts[j + 1..], b)));
            }
            StmtKind::If(_, then, otherwise) => {
                taking(then, a, found);
                if let Some(otherwise) = otherwise {
                    taking(otherwise, a, found);
                }
            }
            StmtKind::While { body, .. }
            | StmtKind::ForOf { body, .. }
            | StmtKind::For { body, .. }
            | StmtKind::Labeled(_, body) => taking(body, a, found),
            StmtKind::Try(x, y) | StmtKind::TryCatch(x, _, y) => {
                taking(x, a, found);
                taking(y, a, found);
            }
            _ => {}
        }
    }
}

/// `stmts` without `taking`'s `const b = a;`, theirs or their blocks'.
fn dropping(stmts: &mut Vec<Stmt>, a: &str, b: &str) {
    stmts.retain(|s| {
        !matches!(&s.kind, StmtKind::Const(n, value)
            if n == b && matches!(&value.kind, ExprKind::Var(v) if v == a))
    });
    for stmt in stmts.iter_mut() {
        let lists: Vec<&mut Vec<Stmt>> = match &mut stmt.kind {
            StmtKind::If(_, then, otherwise) => std::iter::once(then).chain(otherwise.as_mut()).collect(),
            StmtKind::While { body, .. }
            | StmtKind::ForOf { body, .. }
            | StmtKind::For { body, .. }
            | StmtKind::Labeled(_, body) => vec![body],
            StmtKind::Try(x, y) | StmtKind::TryCatch(x, _, y) => vec![x, y],
            _ => Vec::new(),
        };
        for list in lists {
            dropping(list, a, b);
        }
    }
}

/// Each list of statements in `stmts`, the nested ones first, not in its
/// closures: for a pass that a list's statements, done, make apply.
fn bottom_up(stmts: &mut Vec<Stmt>, f: &mut dyn FnMut(&mut Vec<Stmt>)) {
    for stmt in stmts.iter_mut() {
        match &mut stmt.kind {
            StmtKind::If(_, a, b) => {
                bottom_up(a, f);
                if let Some(b) = b {
                    bottom_up(b, f);
                }
            }
            StmtKind::While { body, .. }
            | StmtKind::ForOf { body, .. }
            | StmtKind::For { body, .. }
            | StmtKind::Labeled(_, body) => bottom_up(body, f),
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                bottom_up(a, f);
                bottom_up(b, f);
            }
            _ => {}
        }
    }
    f(stmts);
}

/// What a branch makes a value of, as a person writes it: `if (c) {} else
/// { s }` is `if (!c) { s }`; `if (c) { x = a; } else { x = b; }` is `x =
/// c ? a : b;`, of two branches; and `l: { if (a) { if (b) { x = t; break l; } } x = f; }`,
/// `matches!`'s, is `x = a && b ? t : f;`.
fn branch_values(stmts: &mut Vec<Stmt>, returns: &[js::Span], booleans: &HashSet<String>) {
    for stmt in stmts.iter_mut() {
        let span = stmt.span;
        let kind = std::mem::replace(&mut stmt.kind, StmtKind::Break(None));
        stmt.kind = match kind {
            StmtKind::If(test, then, Some(otherwise)) if then.is_empty() => StmtKind::If(not(test), otherwise, None),
            // `if (a) { break l; } else if (b) { break l; }`, an or-pattern's
            // tests: `if (a || b) { break l; }`.
            StmtKind::If(a, then, Some(mut otherwise))
                if let [
                    Stmt {
                        kind: StmtKind::Break(Some(l)),
                        ..
                    },
                ] = then.as_slice()
                    && let [
                        Stmt {
                            kind: StmtKind::If(_, inner, None),
                            ..
                        },
                    ] = otherwise.as_slice()
                    && matches!(inner.as_slice(), [Stmt { kind: StmtKind::Break(Some(m)), .. }] if m == l) =>
            {
                let Some(Stmt {
                    kind: StmtKind::If(b, _, None),
                    ..
                }) = otherwise.pop()
                else {
                    unreachable!("matched")
                };
                StmtKind::If(or_all(a, b), then, None)
            }
            StmtKind::If(test, then, Some(otherwise)) if otherwise.is_empty() => StmtKind::If(test, then, None),
            // A chain of more arms, its last two folded first, `if (a) { x = p; }
            // else { x = b ? q : r; }`: the chain, as it's written; of a
            // temporary, where its first arm does more than give it, so the
            // whole is no conditional either.
            StmtKind::If(ref test, ref then, Some(ref otherwise))
                if let Some(Stmt {
                    kind: StmtKind::Assign(target, _),
                    ..
                }) = then.last()
                    && let ExprKind::Var(x) = &target.kind
                    && (!is_temporary(x) && then.len() == 1 || is_temporary(x) && then.len() > 1)
                    && let Some((y, folded)) = assigned_only(otherwise)
                    && x == y
                    && let ExprKind::Cond(t, q, r) = &folded.kind
                    && small(q)
                    && small(r) =>
            {
                let at = otherwise[0].span;
                let arm = |value: &Expr| vec![StmtKind::Assign(Expr::var(y), value.clone()).at(at)];
                let chained = StmtKind::If((**t).clone(), arm(q), Some(arm(r))).at(at);
                StmtKind::If(test.clone(), then.clone(), Some(vec![chained]))
            }
            StmtKind::If(test, ref then, Some(ref otherwise))
                if let Some((x, yes)) = assigned_only(then)
                    && let Some((y, no)) = assigned_only(otherwise)
                    && x == y
                    // Not one of more branches, which reads best as they are.
                    // A temporary's, an `if` the program wrote as a value, is one.
                    && ((small(yes) && small(no)) || is_temporary(x)) =>
            {
                // The value the program wrote first, first: `a || b ? x : y` of
                // `if a || b { x } else { y }` MIR tests `!a && !b` of.
                // A chain's next test in the `else` whichever comes first, as one
                // is written: `a === "x" ? y : a === "z" ? .. : ..`, a match's arms.
                let no_first = match (
                    matches!(yes.kind, ExprKind::Cond(..)),
                    matches!(no.kind, ExprKind::Cond(..)),
                ) {
                    (true, false) => true,
                    (false, true) => false,
                    _ => matches!((first_written(then), first_written(otherwise)), (Some(t), Some(o)) if o < t),
                };
                let value = match no_first {
                    true => conditional(not(test.clone()), no.clone(), yes.clone(), booleans),
                    false => conditional(test.clone(), yes.clone(), no.clone(), booleans),
                };
                StmtKind::Assign(Expr::var(x), value)
            }
            // `l: { if (!a) { if (!b) { s; break l; } } t }`, `||`'s: `if (a ||
            // b) { t } else { s }`, where nothing else breaks to `l`; and
            // `if (a && b) { s } else { t }` of tests not all negated.
            StmtKind::Labeled(label, body) if let Some((tests, otherwise, then)) = or_chain(&body, &label) => {
                let negated = tests.iter().all(|t| matches!(t.kind, ExprKind::Unary(UnaryOp::Not, _)));
                let test = (tests.into_iter())
                    .rev()
                    .reduce(|b, a| Expr::bin(Op::And, a, b))
                    .expect("one");
                match (then.is_empty(), negated) {
                    (true, _) => StmtKind::If(test, otherwise, None),
                    (false, true) => StmtKind::If(not(test), then, Some(otherwise)),
                    (false, false) => StmtKind::If(test, otherwise, Some(then)),
                }
            }
            StmtKind::Labeled(label, body) => match chain(&body, &label) {
                Some((tests, x, yes, no)) => {
                    let test = (tests.into_iter())
                        .rev()
                        .reduce(|b, a| Expr::bin(Op::And, a, b))
                        .expect("one");
                    StmtKind::Assign(Expr::var(&x), conditional(test, yes, no, booleans))
                }
                None => StmtKind::Labeled(label, body),
            },
            kind => kind,
        };
        stmt.span = span;
    }
    stmts.retain(|stmt| !matches!(&stmt.kind, StmtKind::Expr(e) if matches!(e.kind, ExprKind::Undefined)));
    // `if (x != null) { if (x === "a") { s } else if (x === "b") { t } }`:
    // the chains alone, which say it.
    let mut i = 0;
    while i < stmts.len() {
        match &stmts[i].kind {
            StmtKind::If(test, then, None) if !then.is_empty() && then.iter().all(|s| guarded_chain(s, test)) => {
                let StmtKind::If(_, then, _) = stmts.remove(i).kind else {
                    unreachable!("matched")
                };
                let n = then.len();
                stmts.splice(i..i, then);
                i += n;
            }
            _ => i += 1,
        }
    }
    // `if (a) { if (b) { return t; } } return f;`, last: `return a && b ? t : f;`.
    if let [.., last_if, last] = stmts.as_slice()
        && let StmtKind::Return(Some(no)) = &last.kind
        && let StmtKind::If(..) = &last_if.kind
        && let Some((tests, yes)) = returned_chain(std::slice::from_ref(last_if))
        && small(&yes)
        && small(no)
        && boolean_branch(&yes, no)
        // Not where the program wrote `return`, an early one.
        && !written(returns, last.span)
        && !writes_return(std::slice::from_ref(last_if), returns)
    {
        let test = (tests.into_iter())
            .rev()
            .reduce(|b, a| Expr::bin(Op::And, a, b))
            .expect("one");
        let value = conditional(test, yes, no.clone(), booleans);
        let span = last_if.span;
        stmts.truncate(stmts.len() - 2);
        stmts.push(StmtKind::Return(Some(value)).at(span));
    } else if let [before @ .., last_if, last] = stmts.as_slice()
        && let StmtKind::Return(Some(_)) = &last.kind
        && written(returns, last.span)
        && let Some((tests, body)) = returning_chain(std::slice::from_ref(last_if))
        // Not `if c { return a; } else { return b; }`'s: a `let … else`'s
        // rest ends where it goes on, not at a `return` the program wrote.
        && !body.last().is_some_and(|s| matches!(s.kind, StmtKind::Return(_)) && written(returns, s.span))
        && !declared(&body).any(|name| declared(before).any(|n| n == name))
    {
        // `f` the program wrote, a `let … else`'s else: `if (!(a && b)) {
        // return f; } t`, as THIR guards it.
        let test = (tests.into_iter())
            .rev()
            .reduce(|b, a| Expr::bin(Op::And, a, b))
            .expect("one");
        let (span, guard) = (last_if.span, stmts.pop().expect("the return"));
        stmts.pop();
        stmts.push(StmtKind::If(not(test), vec![guard], None).at(span));
        stmts.extend(body);
    }
}

/// Of `if (a) { const x = e; if (b) { s } }`, `s` ending where it leaves:
/// `[a, b]` and `const x = e; s`, what a level binds before its test, a
/// read that does nothing, written into later tests, as `broken_chain`'s.
fn returning_chain(stmts: &[Stmt]) -> Option<(Vec<Expr>, Vec<Stmt>)> {
    let [
        bound @ ..,
        Stmt {
            kind: StmtKind::If(test, inner, None),
            ..
        },
    ] = stmts
    else {
        return None;
    };
    if !bound
        .iter()
        .all(|s| matches!(&s.kind, StmtKind::Const(_, value) if value.reads_same()))
    {
        return None;
    }
    let (mut tests, body) = match returning_chain(inner) {
        Some(chain) => chain,
        None if inner.last().is_some_and(ends) => (Vec::new(), inner.clone()),
        None => return None,
    };
    tests.insert(0, test.clone());
    for stmt in bound.iter().rev() {
        let StmtKind::Const(name, value) = &stmt.kind else {
            unreachable!("matched")
        };
        for test in &mut tests {
            *test = test.substitute(&|v| (v == name).then(|| value.clone()))?;
        }
    }
    let mut all = bound.to_vec();
    all.extend(body);
    Some((tests, all))
}

/// Whether of `c ? yes : no` one is `true` or `false`: `c || no` or `c &&
/// yes`, not `?:`.
fn boolean_branch(yes: &Expr, no: &Expr) -> bool {
    matches!(yes.kind, ExprKind::Bool(_)) || matches!(no.kind, ExprKind::Bool(_))
}

/// Of `if (a) { if (b) { return t; } }`: its tests, `[a, b]`, and `t`.
fn returned_chain(stmts: &[Stmt]) -> Option<(Vec<Expr>, Expr)> {
    match stmts {
        [
            Stmt {
                kind: StmtKind::Return(Some(yes)),
                ..
            },
        ] => Some((Vec::new(), yes.clone())),
        [
            Stmt {
                kind: StmtKind::If(test, inner, None),
                ..
            },
        ] => {
            let (mut tests, yes) = returned_chain(inner)?;
            tests.insert(0, test.clone());
            Some((tests, yes))
        }
        _ => None,
    }
}

/// Whether `e` reads well as a branch of `c ? a : b`: not a conditional
/// itself, of more branches, nor an element, which reads best returned or
/// assigned as it is, nor a function.
fn small(e: &Expr) -> bool {
    // An element of two levels at most, `<b>Save</b>`, a value as THIR
    // writes one; one of more reads best as a branch of its own.
    fn levels(e: &Expr) -> usize {
        match &e.kind {
            ExprKind::Jsx(jsx) => 1 + jsx.children.iter().map(levels).max().unwrap_or(0),
            _ => 0,
        }
    }
    match &e.kind {
        ExprKind::Jsx(_) => levels(e) <= 2,
        _ => !matches!(
            e.kind,
            ExprKind::Cond(..) | ExprKind::Arrow(..) | ExprKind::AsyncArrow(..) | ExprKind::Function(_)
        ),
    }
}

/// The variable `stmts` only assign, and what to: `x = a;`.
fn assigned_only(stmts: &[Stmt]) -> Option<(&str, &Expr)> {
    match stmts {
        [
            Stmt {
                kind: StmtKind::Assign(target, value),
                ..
            },
        ] => match &target.kind {
            ExprKind::Var(x) => Some((x.as_str(), value)),
            _ => None,
        },
        _ => None,
    }
}

/// Of `l: { if (a) { if (b) { x = t; break l; } } x = f; }`: its tests,
/// `[a, b]`, `x`, `t` and `f`.
fn chain(body: &[Stmt], label: &str) -> Option<(Vec<Expr>, String, Expr, Expr)> {
    let [
        Stmt {
            kind: StmtKind::If(test, inner, None),
            ..
        },
        last,
    ] = body
    else {
        return None;
    };
    let (x, no) = assigned_only(std::slice::from_ref(last))?;
    let (mut tests, y, yes) = inner_chain(inner, label)?;
    if x != y {
        return None;
    }
    tests.insert(0, test.clone());
    Some((tests, x.to_string(), yes, no.clone()))
}

/// Of `l: { if (a) { if (b) { s; break l; } } t }`: its tests, `[a, b]`,
/// `s` and `t`, where `l` is named nowhere else.
fn or_chain(body: &[Stmt], label: &str) -> Option<(Vec<Expr>, Vec<Stmt>, Vec<Stmt>)> {
    let [first, then @ ..] = body else { return None };
    let (tests, otherwise) = broken_chain(std::slice::from_ref(first), label)?;
    if tests.is_empty() || targets(then, label) || targets(&otherwise, label) {
        return None;
    }
    Some((tests, otherwise, then.to_vec()))
}

/// Of `if (a) { if (b) { s; break l; } }`: `[a, b]` and `s`. What a level
/// binds before its test, `if (a) { const x = o.x; if (x) { … } }`, a read
/// that does nothing, comes first in `s`, and a test reads its value,
/// `a && o.x`, as `if let`s chained with `&&` read.
fn broken_chain(stmts: &[Stmt], label: &str) -> Option<(Vec<Expr>, Vec<Stmt>)> {
    match stmts {
        [
            bound @ ..,
            Stmt {
                kind: StmtKind::If(test, inner, None),
                ..
            },
        ] if bound
            .iter()
            .all(|s| matches!(&s.kind, StmtKind::Const(_, value) if value.reads_same())) =>
        {
            let (mut tests, otherwise) = broken_chain(inner, label)?;
            tests.insert(0, test.clone());
            for stmt in bound.iter().rev() {
                let StmtKind::Const(name, value) = &stmt.kind else {
                    unreachable!("matched")
                };
                for test in &mut tests {
                    *test = test.substitute(&|v| (v == name).then(|| value.clone()))?;
                }
            }
            let mut body = bound.to_vec();
            body.extend(otherwise);
            Some((tests, body))
        }
        [
            rest @ ..,
            Stmt {
                kind: StmtKind::Break(Some(l)),
                ..
            },
        ] if l == label => Some((Vec::new(), rest.to_vec())),
        _ => None,
    }
}

/// Whether `test` is `!c`, or `!(a && b)` as `not` writes it, `!a || !b`.
/// Not `!!c`, `c`'s truth, an `Option` never falsy's presence.
fn negated(test: &Expr) -> bool {
    match &test.kind {
        ExprKind::Unary(UnaryOp::Not, c) => !matches!(c.kind, ExprKind::Unary(UnaryOp::Not, _)),
        ExprKind::Binary(Op::Or, a, b) => negated(a) && negated(b),
        _ => false,
    }
}

/// `l: { if (t) { a; throw e; } b; }`, of an arm `b` MIR made the block's
/// fall-through though it's written before `a`, which leaves the block,
/// `if let Text(..) = k { b } else { a }`: `l: { if (!t) { b } else { a } }`,
/// as written. `b` runs where `t` doesn't hold either way.
fn written_first(stmts: &mut [Stmt]) {
    for stmt in stmts {
        let StmtKind::Labeled(label, body) = &mut stmt.kind else {
            continue;
        };
        let [first, rest @ ..] = body.as_slice() else {
            continue;
        };
        let StmtKind::If(_, then, None) = &first.kind else {
            continue;
        };
        let leaves = |s: &Stmt| match &s.kind {
            StmtKind::Throw(_) | StmtKind::Return(_) => true,
            StmtKind::Break(Some(l)) => l == label,
            _ => false,
        };
        let earlier = matches!((first_written(rest), first_written(then)), (Some(b), Some(a)) if b < a);
        if rest.is_empty() || !then.last().is_some_and(leaves) || !earlier {
            continue;
        }
        let mut rest = std::mem::take(body);
        let first = rest.remove(0);
        let StmtKind::If(test, then, None) = first.kind else {
            unreachable!("matched")
        };
        if rest
            .last()
            .is_some_and(|s| matches!(&s.kind, StmtKind::Break(Some(l)) if l == label))
        {
            rest.pop();
        }
        *body = vec![StmtKind::If(not(test), rest, Some(then)).at(first.span)];
    }
}

/// Of each block `l`, the statements it ends after, `tail_else`'s.
fn tail_elses(stmts: &mut [Stmt]) {
    fn tails(list: &mut Vec<Stmt>, label: &str) {
        *list = tail_else(std::mem::take(list), label);
        if let Some(Stmt {
            kind: StmtKind::If(_, then, otherwise),
            ..
        }) = list.last_mut()
        {
            tails(then, label);
            if let Some(otherwise) = otherwise {
                tails(otherwise, label);
            }
        }
    }
    for stmt in stmts {
        if let StmtKind::Labeled(label, body) = &mut stmt.kind {
            let label = label.clone();
            tails(body, &label);
        }
    }
}

/// `if (c) { s; break l; } t`, statements `l` ends after: `if (c) { s }
/// else { t }`, an `else if` chain's, `t` so too.
fn tail_else(mut stmts: Vec<Stmt>, label: &str) -> Vec<Stmt> {
    let breaks = |list: &[Stmt]| matches!(list.last(), Some(Stmt { kind: StmtKind::Break(Some(l)), .. }) if l == label);
    if stmts.len() < 2 || !matches!(&stmts[0].kind, StmtKind::If(_, then, None) if breaks(then)) {
        return stmts;
    }
    let rest = stmts.split_off(1);
    let first = stmts.remove(0);
    let StmtKind::If(test, mut then, None) = first.kind else {
        unreachable!("matched")
    };
    then.pop();
    vec![StmtKind::If(test, then, Some(tail_else(rest, label))).at(first.span)]
}

/// `a && x != null && typeof x === "string"`, an `Option`'s test before a
/// classification's: `a && typeof x === "string"`, of a type no `null` is.
fn typeof_tests(e: &mut Expr) {
    let ExprKind::Binary(Op::And, left, right) = &e.kind else {
        return;
    };
    let ExprKind::Binary(Op::Eq, kind, name) = &right.kind else {
        return;
    };
    let (ExprKind::Unary(UnaryOp::Typeof, x), ExprKind::Str(name)) = (&kind.kind, &name.kind) else {
        return;
    };
    if matches!(name.as_str(), "object" | "undefined") {
        return;
    }
    let set = |e: &Expr| {
        matches!(&e.kind, ExprKind::Binary(Op::LooseNe, y, null)
            if matches!(null.kind, ExprKind::Null | ExprKind::Undefined) && js::same_path(x, y))
    };
    let tested = (**right).clone();
    *e = match &left.kind {
        _ if set(left) => tested,
        ExprKind::Binary(Op::And, first, last) if set(last) => Expr::bin(Op::And, (**first).clone(), tested),
        _ => return,
    };
}

/// `!e` of a chain of tests, part by part, `!!a || b !== c` of `!a && b ===
/// c`: a `bool`, as `!e` is, each `!x` of what's not one `!!x`.
fn negated_bools(e: Expr, booleans: &HashSet<String>) -> Expr {
    match e.kind {
        ExprKind::Binary(op @ (Op::And | Op::Or), left, right) => {
            let flipped = if op == Op::And { Op::Or } else { Op::And };
            Expr::bin(flipped, negated_bools(*left, booleans), negated_bools(*right, booleans))
        }
        ExprKind::Unary(UnaryOp::Not, x) if is_boolean(&x, booleans) => *x,
        ExprKind::Unary(UnaryOp::Not, x) => Expr::unary(UnaryOp::Not, Expr::unary(UnaryOp::Not, *x)),
        kind => negate(Expr {
            reads: false,
            kind,
            span: e.span,
        }),
    }
}

/// `else { if (t) { throw e; } s }`, a check MIR wrote first in an `else if`
/// chain's last `else`: `else if (!t) { s } else { throw e; }`, the chain as
/// written.
fn chained_checks(stmts: &mut [Stmt]) {
    for stmt in stmts {
        let StmtKind::If(_, _, Some(otherwise)) = &mut stmt.kind else {
            continue;
        };
        let [check, _, ..] = otherwise.as_slice() else {
            continue;
        };
        if !matches!(&check.kind, StmtKind::If(_, then, None)
            if matches!(then.as_slice(), [Stmt { kind: StmtKind::Throw(_), .. }]))
        {
            continue;
        }
        let rest = otherwise.split_off(1);
        let check = otherwise.remove(0);
        let StmtKind::If(test, throw, None) = check.kind else {
            unreachable!("matched")
        };
        *otherwise = vec![StmtKind::If(not(test), rest, Some(throw)).at(check.span)];
    }
}

/// Whether reading `e` after a call reads what it did before: a constant, a
/// variable no closure sets, `set`'s, which a call can't then; an import,
/// `<Seo>`'s tag, a module's export being never set again, a `static mut`
/// its `{ value }`; a field of a plain value, `meta.tag`, which no call can
/// change, as MIR's lowering reads it where it's read, or a builtin's
/// function, `Object.fromEntries`; an element of such, a closure made, and a
/// test or a choice of such, `selected ? "page" : undefined`.
fn read_past_calls(e: &Expr, set: &HashSet<String>, plain: &HashSet<String>) -> bool {
    let past = |e: &Expr| read_past_calls(e, set, plain);
    let mut root = e;
    while let ExprKind::Member(of, _) = &root.kind {
        root = of;
    }
    match &e.kind {
        ExprKind::Var(v) => !set.contains(v),
        ExprKind::Symbol(_) => true,
        ExprKind::Arrow(..) | ExprKind::AsyncArrow(..) => true,
        ExprKind::Member(of, _) => {
            matches!(&root.kind, ExprKind::Var(v) if plain.contains(v) && !set.contains(v))
                || matches!(&of.kind, ExprKind::Var(v)
                    if !set.contains(v) && matches!(v.as_str(), "Object" | "Math" | "JSON" | "Array" | "Number" | "String"))
        }
        ExprKind::Jsx(_) => inert_element(e, set),
        // The items of an array nothing changes, `...first`.
        ExprKind::Spread(a) => matches!(&a.kind, ExprKind::Var(v)
            if !set.contains(v) && (plain.contains(v) || plain.contains(&format!("...{v}")))),
        ExprKind::Unary(UnaryOp::Not, a) => past(a),
        ExprKind::Binary(
            Op::Eq
            | Op::Ne
            | Op::LooseEq
            | Op::LooseNe
            | Op::Lt
            | Op::Le
            | Op::Gt
            | Op::Ge
            | Op::And
            | Op::Or
            | Op::Coalesce,
            a,
            b,
        ) => past(a) && past(b),
        ExprKind::Cond(a, b, c) => past(a) && past(b) && past(c),
        _ => e.is_made_of_constants(),
    }
}

/// The variables a closure in `body` sets, `x = ..` in an arrow's body.
fn closures_set(body: &[Stmt]) -> HashSet<String> {
    let mut set = HashSet::new();
    js::each_expr_mut(&mut body.to_vec(), &mut |e| {
        if let ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) = &e.kind {
            js::statement_lists(&mut body.clone(), &mut |list| {
                for stmt in list.iter() {
                    if let StmtKind::Assign(target, _) = &stmt.kind {
                        let mut root = target;
                        while let ExprKind::Member(of, _) | ExprKind::Index(of, _) = &root.kind {
                            root = of;
                        }
                        if let ExprKind::Var(v) = &root.kind {
                            set.insert(v.clone());
                        }
                    }
                }
            });
        }
    });
    set
}

/// Whether making `e` reads only what no call changes: an element, array or
/// object of constants, of variables nothing sets again, of imports and of
/// arrows, which capture what they read, and of such elements.
fn inert_element(e: &Expr, changing: &HashSet<String>) -> bool {
    let props = |props: &[js::Prop]| {
        props.iter().all(|p| {
            let (js::Prop::Field(_, v) | js::Prop::Getter(_, v) | js::Prop::Spread(v)) = p;
            !matches!(p, js::Prop::Spread(_)) && inert_element(v, changing)
        })
    };
    match &e.kind {
        ExprKind::Var(v) => !changing.contains(v),
        ExprKind::Symbol(_) | ExprKind::Arrow(..) | ExprKind::AsyncArrow(..) => true,
        ExprKind::Array(items) => items.iter().all(|i| inert_element(i, changing)),
        ExprKind::Object(fields) => props(fields),
        // A choice of them, `open ? <IconClose /> : <IconHamburger />`.
        ExprKind::Cond(a, b, c) => [a, b, c].iter().all(|e| inert_element(e, changing)),
        ExprKind::Jsx(jsx) => {
            let tag = match &jsx.tag {
                js::JsxTag::Component(tag) => inert_element(tag, changing),
                _ => true,
            };
            tag && props(&jsx.props) && jsx.children.iter().all(|c| inert_element(c, changing))
        }
        _ => e.is_made_of_constants(),
    }
}

/// `if (t) { if (..) .. } else { y }`, `y` written before the chain in
/// the `then`: `if (!t) { y } else if (..) ..`, a match's arms as written,
/// which MIR tests `!== "a"` of with the rest first.
fn written_chains(stmts: &mut [Stmt]) {
    for stmt in stmts {
        let StmtKind::If(_, then, Some(otherwise)) = &stmt.kind else {
            continue;
        };
        let chain = |list: &[Stmt]| {
            matches!(
                list,
                [Stmt {
                    kind: StmtKind::If(..),
                    ..
                }]
            )
        };
        // Where the chain's first arm is written: its `if` is where the match is.
        let arm = match then.as_slice() {
            [
                Stmt {
                    kind: StmtKind::If(_, arm, _),
                    ..
                },
            ] => first_written(arm),
            _ => None,
        };
        let earlier = matches!((first_written(otherwise), arm), (Some(o), Some(t)) if o < t);
        if !chain(then) || chain(otherwise) || !earlier {
            continue;
        }
        let StmtKind::If(test, then, Some(otherwise)) = std::mem::replace(&mut stmt.kind, StmtKind::Break(None)) else {
            unreachable!("matched")
        };
        stmt.kind = StmtKind::If(not(test), otherwise, Some(then));
    }
}

/// `let c; if (t) { c = e; }`, what MIR made of an element only where its
/// test holds, ahead of a later child, read once: `const c = t ? e :
/// undefined;`, which reads as one, and is read in place as one.
fn guarded_lets(stmts: &mut Vec<Stmt>, captured: &HashSet<String>) {
    let mut i = 0;
    while i + 2 < stmts.len() {
        let made = match (&stmts[i].kind, &stmts[i + 1].kind) {
            (StmtKind::Let(x, None), StmtKind::If(test, then, None))
                if let [
                    Stmt {
                        kind: StmtKind::Assign(target, value),
                        ..
                    },
                ] = then.as_slice()
                    && matches!(&target.kind, ExprKind::Var(v) if v == x)
                    && (is_temporary(x) || captured.contains(x))
                    && js::mentions_in(&stmts[i + 2..], x) == 1 =>
            {
                Some((x.clone(), Expr::cond(test.clone(), value.clone(), Expr::undefined())))
            }
            _ => None,
        };
        if let Some((x, value)) = made {
            let span = stmts[i + 1].span;
            stmts.splice(i..i + 2, [StmtKind::Const(x, value).at(span)]);
        }
        i += 1;
    }
}

/// `t && <div className={t ? e : undefined} />`: `e`, where `t` holds, of a
/// test that does nothing and reads what nothing changes.
fn implied_tests(e: &mut Expr, changing: &HashSet<String>) {
    let (ExprKind::Binary(Op::And, test, child) | ExprKind::Cond(test, child, _)) = &mut e.kind else {
        return;
    };
    let mut unchanged = !test.has_effects();
    test.visit_vars(&mut |v| unchanged &= !changing.contains(v));
    if !unchanged {
        return;
    }
    let test = (**test).clone();
    child.each_mut(&mut |e| {
        if let ExprKind::Cond(t, yes, no) = &e.kind
            && matches!(no.kind, ExprKind::Undefined)
            && same_test(t, &test)
        {
            *e = (**yes).clone();
        }
    });
}

/// Whether `a` and `b` are one test of what nothing changes: one path,
/// `!` of one, or one compared to one constant, `error != null`.
fn same_test(a: &Expr, b: &Expr) -> bool {
    // `!!x`, a test of `x`'s truth, as `x` is.
    fn truth(e: &Expr) -> &Expr {
        match &e.kind {
            ExprKind::Unary(UnaryOp::Not, x) if let ExprKind::Unary(UnaryOp::Not, x) = &x.kind => x,
            _ => e,
        }
    }
    let (a, b) = (truth(a), truth(b));
    match (&a.kind, &b.kind) {
        (ExprKind::Unary(UnaryOp::Not, x), ExprKind::Unary(UnaryOp::Not, y)) => same_test(x, y),
        (ExprKind::Binary(p, x, k), ExprKind::Binary(q, y, l)) if p == q => {
            same(x, y)
                && match (&k.kind, &l.kind) {
                    (ExprKind::Null | ExprKind::Undefined, ExprKind::Null | ExprKind::Undefined) => true,
                    (ExprKind::Str(k), ExprKind::Str(l)) => k == l,
                    _ => k.as_int().is_some() && k.as_int() == l.as_int(),
                }
        }
        _ => same(a, b),
    }
}

/// `if (raw) { const error = raw; .. }`, a pattern's binding of a variable,
/// `if let Some(error) = raw`, which MIR copies where `raw` is borrowed:
/// `raw` where `error` is read, as THIR binds it, where the block sets
/// neither again, declares neither, nor reads `error` in a closure, which
/// may run after; a `let`-`else`'s too, after the test that leaves.
fn copied_bindings(stmts: &mut Vec<Stmt>) {
    for stmt in stmts.iter_mut() {
        let StmtKind::If(test, then, _) = &mut stmt.kind else {
            continue;
        };
        let Some(Stmt {
            kind: StmtKind::Const(b, value),
            ..
        }) = then.first()
        else {
            continue;
        };
        let ExprKind::Var(a) = &value.kind else {
            continue;
        };
        // What the test says is there, `raw` of `raw && ..`, `!!raw` or `raw
        // != null`.
        let mut first = &*test;
        while let ExprKind::Binary(Op::And, x, _) = &first.kind {
            first = x;
        }
        if let ExprKind::Unary(UnaryOp::Not, x) = &first.kind
            && let ExprKind::Unary(UnaryOp::Not, x) = &x.kind
        {
            first = x;
        }
        let present = match &first.kind {
            ExprKind::Var(v) => v == a,
            ExprKind::Binary(Op::LooseNe, v, null) => {
                matches!(&v.kind, ExprKind::Var(v) if v == a)
                    && matches!(null.kind, ExprKind::Null | ExprKind::Undefined)
            }
            _ => false,
        };
        let (a, b) = (a.clone(), b.clone());
        if !present || is_temporary(&a) || is_temporary(&b) || a == b {
            continue;
        }
        let rest = &then[1..];
        if rebinds(rest, &a) || rebinds(rest, &b) || in_closure(rest, &b) {
            continue;
        }
        then.remove(0);
        js::each_expr_mut(then, &mut |e| {
            if let ExprKind::Var(v) = &mut e.kind
                && *v == b
            {
                *v = a.clone();
            }
        });
    }
    let mut i = 0;
    while i + 1 < stmts.len() {
        if let StmtKind::If(test, then, None) = &stmts[i].kind
            && then.last().is_some_and(ends)
            && let StmtKind::Const(b, value) = &stmts[i + 1].kind
            && let ExprKind::Var(a) = &value.kind
            && absent(test, a)
            && !is_temporary(a)
            && !is_temporary(b)
            && a != b
        {
            let (a, b) = (a.clone(), b.clone());
            let rest = &stmts[i + 2..];
            if !rebinds(rest, &a) && !rebinds(rest, &b) && !in_closure(rest, &b) {
                stmts.remove(i + 1);
                js::each_expr_mut(&mut stmts[i + 1..], &mut |e| {
                    if let ExprKind::Var(v) = &mut e.kind
                        && *v == b
                    {
                        *v = a.clone();
                    }
                });
            }
        }
        i += 1;
    }
}

/// Whether `stmts` set `name` again, or declare it, a closure's in them too.
fn rebinds(stmts: &[Stmt], name: &str) -> bool {
    let mut found = false;
    js::statement_lists(&mut stmts.to_vec(), &mut |list| {
        for stmt in list.iter() {
            found |= match &stmt.kind {
                StmtKind::Const(n, _) | StmtKind::Let(n, _) => n == name,
                StmtKind::Destructure { pattern, .. } | StmtKind::ForOf { pattern, .. } => {
                    pattern.names().contains(&name)
                }
                StmtKind::For { name: n, .. } => n == name,
                StmtKind::Assign(target, _) => matches!(&target.kind, ExprKind::Var(v) if v == name),
                _ => false,
            };
        }
    });
    js::each_expr_mut(&mut stmts.to_vec(), &mut |e| {
        if let ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) = &e.kind {
            found |= params.iter().any(|p| p.names().contains(&name)) || rebinds(body, name);
        }
    });
    found
}

/// Whether `stmts` set something of `name`'s, `name[0] = ..`, a closure's
/// in them too.
fn sets_through(stmts: &[Stmt], name: &str) -> bool {
    let mut set = false;
    js::statement_lists(&mut stmts.to_vec(), &mut |list| {
        for stmt in list.iter() {
            if let StmtKind::Assign(target, _) = &stmt.kind {
                target.visit_vars(&mut |v| set |= v == name);
            }
        }
    });
    js::each_expr_mut(&mut stmts.to_vec(), &mut |e| {
        if let ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) = &e.kind {
            set |= sets_through(body, name);
        }
    });
    set
}

/// Whether a closure in `stmts` reads `name`.
fn in_closure(stmts: &[Stmt], name: &str) -> bool {
    let mut found = false;
    js::each_expr_mut(&mut stmts.to_vec(), &mut |e| {
        if let ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) = &e.kind {
            found |= js::mentions_in(body, name) > 0;
        }
    });
    found
}

/// `const children = [" ", c ? a : b];`, a tuple MIR holds, read only item
/// by item, `children[0]`: each item where it's read, of items that do
/// nothing and read what nothing changes, which read the same anywhere.
fn tuple_reads(stmts: &mut Vec<Stmt>, changing: &HashSet<String>, captured: &HashSet<String>) {
    let mut i = 0;
    while i < stmts.len() {
        let found = match &stmts[i].kind {
            StmtKind::Const(t, value)
                if (is_temporary(t) || captured.contains(t))
                    && let ExprKind::Array(items) = &value.kind
                    && items.iter().all(|item| {
                        let mut inert = !item.has_effects();
                        item.visit_vars(&mut |v| inert &= !changing.contains(v));
                        inert
                    }) =>
            {
                Some((t.clone(), items.clone()))
            }
            _ => None,
        };
        let Some((t, items)) = found else {
            i += 1;
            continue;
        };
        let item = |e: &Expr| match &e.kind {
            ExprKind::Index(of, k) if matches!(&of.kind, ExprKind::Var(v) if *v == t) => k
                .as_int()
                .and_then(|k| usize::try_from(k).ok())
                .filter(|&k| k < items.len()),
            _ => None,
        };
        let mut indexed = 0;
        js::each_expr_mut(&mut stmts[i + 1..], &mut |e| indexed += usize::from(item(e).is_some()));
        // Read only, not set through, `children[0] = ..`.
        if sets_through(&stmts[i + 1..], &t) || indexed == 0 || indexed != js::mentions_in(&stmts[i + 1..], &t) {
            i += 1;
            continue;
        }
        js::each_expr_mut(&mut stmts[i + 1..], &mut |e| {
            if let Some(k) = item(e) {
                *e = items[k].clone();
            }
        });
        stmts.remove(i);
    }
}

/// `[{ value: a, impl }, ..].map((item) => item.value)`, `dyn` values MIR
/// unwraps for JS, of what else does nothing: `[a, ..]`.
fn projected(e: &Expr) -> Option<Expr> {
    let ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    let (ExprKind::Member(items, map), [f]) = (&callee.kind, args.as_slice()) else {
        return None;
    };
    let (ExprKind::Array(items), "map") = (&items.kind, map.as_str()) else {
        return None;
    };
    let field = field_of(f)?;
    let values = items.iter().map(|item| match &item.kind {
        ExprKind::Object(props) => {
            let mut value = None;
            for prop in props {
                match prop {
                    js::Prop::Field(name, v) if *name == field => value = Some(v.clone()),
                    js::Prop::Field(_, v) if !v.has_effects() => {}
                    _ => return None,
                }
            }
            value
        }
        _ => None,
    });
    Some(Expr {
        span: e.span,
        ..Expr::array(values.collect::<Option<Vec<_>>>()?)
    })
}

/// `f` of `(item) => item.f`.
fn field_of(f: &Expr) -> Option<String> {
    let ExprKind::Arrow(params, body) = &f.kind else {
        return None;
    };
    let (
        [js::Pattern::Name(x)],
        [
            Stmt {
                kind: StmtKind::Return(Some(read)),
                ..
            },
        ],
    ) = (params.as_slice(), body.as_slice())
    else {
        return None;
    };
    match &read.kind {
        ExprKind::Member(of, field) if matches!(&of.kind, ExprKind::Var(v) if v == x) => Some(field.clone()),
        _ => None,
    }
}

/// `const t = [{ value: a, impl }, ..]; .. t.map((item) => item.value)`, read
/// so alone: `const t = [a, ..]; .. t`, `projected`'s, held.
fn unwrapped_values(stmts: &mut [Stmt]) {
    for i in 0..stmts.len() {
        let StmtKind::Const(t, value) = &stmts[i].kind else {
            continue;
        };
        if !is_temporary(t) || !matches!(value.kind, ExprKind::Array(_)) || js::mentions_in(&stmts[i + 1..], t) != 1 {
            continue;
        }
        let t = t.clone();
        let mut field = None;
        js::each_expr_mut(&mut stmts[i + 1..], &mut |e| {
            if let ExprKind::Call(callee, args) = &e.kind
                && let (ExprKind::Member(of, map), [f]) = (&callee.kind, args.as_slice())
                && matches!(&of.kind, ExprKind::Var(v) if *v == t)
                && map == "map"
            {
                field = field_of(f).map(|_| f.clone());
            }
        });
        let Some(f) = field else { continue };
        let StmtKind::Const(_, value) = &stmts[i].kind else {
            unreachable!("matched")
        };
        let call = Expr::call(Expr::member(value.clone(), "map"), vec![f]);
        let Some(values) = projected(&call) else {
            continue;
        };
        stmts[i].kind = StmtKind::Const(t.clone(), values);
        js::each_expr_mut(&mut stmts[i + 1..], &mut |e| {
            if let ExprKind::Call(callee, _) = &e.kind
                && let ExprKind::Member(of, _) = &callee.kind
                && matches!(&of.kind, ExprKind::Var(v) if *v == t)
            {
                *e = Expr::var(&t);
            }
        });
    }
}

/// `_4`, a temporary MIR holds still, read more than once: named as THIR
/// names one, after the field it reads, `routes` of `tree.routes`, else
/// `value`, or `tmp` of a `let` its branches set, a name no other in the
/// function has, its parameters' and closures' too.
pub(super) fn readable_temporaries(params: &[js::Pattern], body: &mut Vec<Stmt>) {
    let mut taken: HashSet<String> = params.iter().flat_map(|p| p.names()).map(str::to_string).collect();
    names_in(body, &mut taken);
    // What an element is given each as, `href={_5}` or a child, `{_6}`: the
    // prop's name, `href`, or `children`, as THIR names one it holds.
    let mut given: HashMap<String, String> = HashMap::new();
    js::each_expr_mut(&mut body.clone(), &mut |e| {
        if let ExprKind::Jsx(jsx) = &e.kind {
            for prop in &jsx.props {
                if let js::Prop::Field(key, value) = prop
                    && let ExprKind::Var(v) = &value.kind
                    && (is_temporary(v) || is_held_arg(v))
                    && key.chars().all(|c| c.is_ascii_alphanumeric())
                {
                    given.entry(v.clone()).or_insert_with(|| key.clone());
                }
            }
            for child in &jsx.children {
                if let ExprKind::Var(v) = &child.kind
                    && (is_temporary(v) || is_held_arg(v))
                {
                    given.entry(v.clone()).or_insert_with(|| "children".to_string());
                }
            }
        }
    });
    let mut renames = Vec::new();
    js::statement_lists(body, &mut |list| {
        for stmt in list.iter() {
            match &stmt.kind {
                StmtKind::Const(name, value) if is_temporary(name) || is_held_arg(name) => {
                    let base = match (given.get(name), &value.kind) {
                        (Some(prop), _) => prop.clone(),
                        (None, _) if is_held_arg(name) => "arg".to_string(),
                        (None, ExprKind::Member(_, field)) => field.clone(),
                        (None, _) => "value".to_string(),
                    };
                    renames.push((name.clone(), base));
                }
                StmtKind::Let(name, _) if is_temporary(name) => renames.push((name.clone(), "tmp".to_string())),
                _ => {}
            }
        }
    });
    for (from, base) in renames {
        let to = crate::names::fresh_in(&mut taken, &base);
        js::statement_lists(body, &mut |list| {
            for stmt in list.iter_mut() {
                if let StmtKind::Const(name, _) | StmtKind::Let(name, _) = &mut stmt.kind
                    && *name == from
                {
                    *name = to.clone();
                }
            }
        });
        js::each_expr_mut(body, &mut |e| {
            if let ExprKind::Var(v) = &mut e.kind
                && *v == from
            {
                *v = to.clone();
            }
        });
    }
}

/// Each name `stmts` read or declare, their closures' and parameters' too.
fn names_in(stmts: &[Stmt], names: &mut HashSet<String>) {
    js::visit_stmts(stmts, &mut |name| {
        names.insert(name.to_string());
    });
    js::statement_lists(&mut stmts.to_vec(), &mut |list| {
        for stmt in list.iter() {
            match &stmt.kind {
                StmtKind::Const(name, _) | StmtKind::Let(name, _) | StmtKind::For { name, .. } => {
                    names.insert(name.clone());
                }
                StmtKind::Destructure { pattern, .. } | StmtKind::ForOf { pattern, .. } => {
                    names.extend(pattern.names().into_iter().map(str::to_string));
                }
                StmtKind::TryCatch(_, Some(name), _) => {
                    names.insert(name.clone());
                }
                _ => {}
            }
        }
    });
    js::each_expr_mut(&mut stmts.to_vec(), &mut |e| {
        if let ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) = &e.kind {
            names.extend(params.iter().flat_map(|p| p.names()).map(str::to_string));
            names_in(body, names);
        }
    });
}

/// `const result = e; return result;`, the place MIR makes what a function
/// gives in, `returned`'s once it's a `const`: `return e;`.
fn returned_consts(stmts: &mut Vec<Stmt>, result: &str) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        let returns = matches!((&stmts[i].kind, &stmts[i + 1].kind), (StmtKind::Const(name, _), StmtKind::Return(Some(value)))
            if name == result && matches!(&value.kind, ExprKind::Var(v) if v == name));
        if returns {
            let StmtKind::Const(_, value) = stmts.remove(i).kind else {
                unreachable!("matched")
            };
            stmts[i].kind = StmtKind::Return(Some(value));
        }
        i += 1;
    }
}

/// `a || b`, of `b` a chain itself too, `a || b1 || b2`, as one is written.
fn or_all(a: Expr, b: Expr) -> Expr {
    match b.kind {
        ExprKind::Binary(Op::Or, x, y) => Expr::bin(Op::Or, or_all(a, *x), *y),
        kind => Expr::bin(
            Op::Or,
            a,
            Expr {
                reads: false,
                kind,
                span: b.span,
            },
        ),
    }
}

/// `!e`, an `if`'s test, as a person writes it: `!!a` is `a`, `!(a === b)`
/// `a !== b`, and `!(a && b)` `!a || !b`, whose truth is the same.
fn not(e: Expr) -> Expr {
    match e.kind {
        ExprKind::Binary(op @ (Op::And | Op::Or), a, b) => {
            let flipped = if op == Op::And { Op::Or } else { Op::And };
            Expr::bin(flipped, not(*a), not(*b))
        }
        ExprKind::Unary(UnaryOp::Not, inner) => *inner,
        kind => Expr::unary(
            UnaryOp::Not,
            Expr {
                reads: false,
                kind,
                span: e.span,
            },
        ),
    }
}

/// Of `if (b) { x = t; break l; }`, nested: its tests, `x` and `t`.
fn inner_chain(stmts: &[Stmt], label: &str) -> Option<(Vec<Expr>, String, Expr)> {
    match stmts {
        [
            assign,
            Stmt {
                kind: StmtKind::Break(Some(l)),
                ..
            },
        ] if l == label => {
            let (x, yes) = assigned_only(std::slice::from_ref(assign))?;
            Some((Vec::new(), x.to_string(), yes.clone()))
        }
        [
            Stmt {
                kind: StmtKind::If(test, inner, None),
                ..
            },
        ] => {
            let (mut tests, x, yes) = inner_chain(inner, label)?;
            tests.insert(0, test.clone());
            Some((tests, x, yes))
        }
        _ => None,
    }
}

/// `test ? yes : no`: `test && yes` where `no` is `false`, and `test || no`
/// where `yes` is `true`, the test `!!test` where it's not a `bool`.
fn conditional(test: Expr, yes: Expr, no: Expr, booleans: &HashSet<String>) -> Expr {
    // Where the test itself is the value, of what's not a `bool`, `!!c`, as
    // THIR writes it; its negation too, `!!o` of `!o`'s, `is_some()`'s.
    let truth = |test: Expr| match is_boolean(&test, booleans) {
        true => test,
        false => Expr::unary(UnaryOp::Not, Expr::unary(UnaryOp::Not, test)),
    };
    // A chain's negation, `a || b` of `!a && !b`, each part a `bool` still,
    // `!!a` of a value's `!a`.
    let negate = |test: Expr| match test.kind {
        ExprKind::Binary(Op::And | Op::Or, ..) => negated_bools(test, booleans),
        _ => negate(test),
    };
    match (&yes.kind, &no.kind) {
        (ExprKind::Bool(true), ExprKind::Bool(false)) => truth(test),
        (ExprKind::Bool(false), ExprKind::Bool(true)) => truth(negate(test)),
        (ExprKind::Bool(true), _) => Expr::bin(Op::Or, truth(test), no),
        (ExprKind::Bool(false), _) => Expr::bin(Op::And, truth(negate(test)), no),
        (_, ExprKind::Bool(true)) => Expr::bin(Op::Or, truth(negate(test)), yes),
        (_, ExprKind::Bool(false)) => Expr::bin(Op::And, truth(test), yes),
        _ => Expr::cond(test, yes, no),
    }
}

/// Whether `test` holds where `by` does: `x != null` where `x === "lit"`,
/// `typeof x === "string"` there too, and where each side of `a || b` says
/// it.
fn implies(by: &Expr, test: &Expr) -> bool {
    match (&test.kind, &by.kind) {
        (_, ExprKind::Binary(Op::Or, a, b)) => implies(a, test) && implies(b, test),
        (ExprKind::Binary(Op::LooseNe, x, null), ExprKind::Binary(Op::Eq, y, literal))
            if matches!(null.kind, ExprKind::Null | ExprKind::Undefined) =>
        {
            same(x, y) && literal.is_constant() && !matches!(literal.kind, ExprKind::Null | ExprKind::Undefined)
        }
        (ExprKind::Binary(Op::Eq, kind, name), ExprKind::Binary(Op::Eq, y, literal))
            if let ExprKind::Unary(UnaryOp::Typeof, x) = &kind.kind =>
        {
            same(x, y)
                && match (&name.kind, &literal.kind) {
                    (ExprKind::Str(t), ExprKind::Str(_)) => t == "string",
                    (ExprKind::Str(t), ExprKind::Num(_)) => t == "number",
                    (ExprKind::Str(t), ExprKind::Bool(_)) => t == "boolean",
                    _ => false,
                }
        }
        _ => false,
    }
}

/// Whether each test of the `else if` chain `stmt` says `guard`, and it has
/// no last `else`: where `guard` fails, it does nothing.
fn guarded_chain(stmt: &Stmt, guard: &Expr) -> bool {
    match &stmt.kind {
        StmtKind::If(test, _, otherwise) => {
            implies(test, guard)
                && (otherwise.as_deref()).is_none_or(|rest| matches!(rest, [next] if guarded_chain(next, guard)))
        }
        _ => false,
    }
}

/// Whether `e` is truthy or `undefined`, no other falsy value: `x ? x :
/// undefined`, what `filter` keeps of `x`, and `a || b` and `a ?? b` of
/// such.
fn truthy_or_undefined(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Cond(test, yes, no) => matches!(no.kind, ExprKind::Undefined) && same(&test.tested(), yes),
        ExprKind::Binary(Op::Or, _, b) => truthy_or_undefined(b),
        ExprKind::Binary(Op::Coalesce, a, b) => truthy_or_undefined(a) && truthy_or_undefined(b),
        _ => false,
    }
}

/// `e`, `truthy_or_undefined`, where only its truthy values are read, as
/// `||`'s first operand: `x ? x : undefined` is `x`, and `a ?? b` `a || b`.
fn truth_of(e: Expr) -> Expr {
    let span = e.span;
    match e.kind {
        ExprKind::Cond(_, yes, _) => *yes,
        ExprKind::Binary(Op::Or, a, b) => Expr::bin(Op::Or, *a, truth_of(*b)),
        ExprKind::Binary(Op::Coalesce, a, b) => Expr::bin(Op::Or, truth_of(*a), truth_of(*b)),
        kind => Expr {
            reads: false,
            kind,
            span,
        },
    }
}

/// `$orInsert(m, k, 0); const h = $mutGet(m, k); h.value = f(h.value);`,
/// MIR's `*m.entry(k).or_insert(0) += 1`, the handle read nowhere else: the
/// value there or put in, then set, `const current = $orInsert(m, k, 0);
/// m.set(k, f(current));`, as THIR writes it.
fn map_counts(stmts: &mut Vec<Stmt>) {
    let call_of = |e: &Expr, helper: &str| match &e.kind {
        ExprKind::Call(callee, args) if matches!(&callee.kind, ExprKind::Var(f) if f == helper) => Some(args.clone()),
        _ => None,
    };
    let mut i = 0;
    while i + 2 < stmts.len() {
        if let StmtKind::Expr(inserted) = &stmts[i].kind
            && let Some(args) = call_of(inserted, "$orInsert")
            && let [map, key, _] = &args[..]
            && let StmtKind::Const(handle, got) = &stmts[i + 1].kind
            && let Some(got) = call_of(got, "$mutGet")
            && let [m, k] = &got[..]
            && same(map, m)
            && (same(key, k) || matches!((&key.kind, &k.kind), (ExprKind::Str(a), ExprKind::Str(b)) if a == b))
            && let StmtKind::Assign(target, value) = &stmts[i + 2].kind
            && matches!(&target.kind, ExprKind::Member(of, v)
                if v == "value" && matches!(&of.kind, ExprKind::Var(h) if h == handle))
            && js::mentions_in(&stmts[i + 3..], handle) == 0
        {
            let name = (std::iter::once("current".to_string()))
                .chain((1..).map(|n| format!("current${n}")))
                .find(|n| js::mentions_in(stmts, n) == 0)
                .expect("a name");
            let mut value = vec![StmtKind::Expr(value.clone()).at(js::Span::NONE)];
            js::each_expr_mut(&mut value, &mut |e| {
                if let ExprKind::Member(of, v) = &e.kind
                    && v == "value"
                    && matches!(&of.kind, ExprKind::Var(h) if h == handle)
                {
                    *e = Expr::var(&name);
                }
            });
            if js::mentions_in(&value, handle) > 0 {
                i += 1;
                continue;
            }
            let StmtKind::Expr(value) = value.pop().expect("one").kind else {
                unreachable!("made")
            };
            let set = Expr::call(Expr::member(map.clone(), "set"), vec![key.clone(), value]);
            let span = stmts[i + 2].span;
            stmts[i].kind = StmtKind::Const(name, inserted.clone());
            stmts.splice(i + 1..i + 3, [StmtKind::Expr(set).at(span)]);
        }
        i += 1;
    }
}

/// Whether `a` and `b` are the same variable, or the same property or item
/// of one.
fn same(a: &Expr, b: &Expr) -> bool {
    match (&a.kind, &b.kind) {
        (ExprKind::Var(x), ExprKind::Var(y)) => x == y,
        (ExprKind::Member(x, p), ExprKind::Member(y, q))
        | (ExprKind::OptionalMember(x, p), ExprKind::OptionalMember(y, q)) => p == q && same(x, y),
        (ExprKind::Index(x, i), ExprKind::Index(y, j)) => {
            same(x, y) && (same(i, j) || i.as_int().is_some() && i.as_int() == j.as_int())
        }
        _ => false,
    }
}

/// Whether `e` is a `bool` whatever it reads: a comparison, a `!`, one of
/// `booleans`, or `&&` and `||` of them.
fn is_boolean(e: &Expr, booleans: &HashSet<String>) -> bool {
    match &e.kind {
        ExprKind::Bool(_) | ExprKind::Unary(UnaryOp::Not, _) => true,
        ExprKind::Var(name) => booleans.contains(name),
        ExprKind::Member(..) => field_path(e).is_some_and(|path| booleans.contains(&path)),
        ExprKind::Call(..) if call_path(e).is_some_and(|path| booleans.contains(&path)) => true,
        ExprKind::Binary(
            Op::Eq | Op::Ne | Op::LooseEq | Op::LooseNe | Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::InstanceOf | Op::In,
            ..,
        ) => true,
        ExprKind::Binary(Op::And | Op::Or, a, b) => is_boolean(a, booleans) && is_boolean(b, booleans),
        _ => false,
    }
}

/// `isItem()` of `isItem(item)`, `globalThis.isItem()`, what a call calls, as a key.
pub(super) fn call_path(e: &Expr) -> Option<String> {
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
pub(super) fn field_path(e: &Expr) -> Option<String> {
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

/// The variables something may change: each `let`'s, and each assigned.
fn changing(body: &mut Vec<Stmt>) -> HashSet<String> {
    let mut changing = assigned(body);
    js::each_block_mut(body, &mut |stmts| {
        for stmt in stmts.iter() {
            if let StmtKind::Let(name, _) = &stmt.kind {
                changing.insert(name.clone());
            }
        }
    });
    changing
}

/// A temporary MIR made, `const _5 = e;`, read once, by the statement
/// after it, or after those that are moved there too: `e` where it's read, where nothing that runs before it there
/// does anything, or anywhere outside a closure if `e` does nothing and
/// reads nothing that changes.
/// `set`, the variables a closure sets: what a call can change of them.
fn inlined(
    stmts: &mut Vec<Stmt>,
    changing: &HashSet<String>,
    set: &HashSet<String>,
    plain: &HashSet<String>,
    captured: &HashSet<String>,
) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        let (head, rest) = stmts.split_at_mut(i + 1);
        let StmtKind::Const(name, value) = &head[i].kind else {
            i += 1;
            continue;
        };
        let temporary = is_temporary(name) || captured.contains(name);
        // What's read before it: what does nothing, where it does nothing
        // either; else only constants and variables nothing changes, which
        // what it does can't change. Of what an element made ahead of a
        // branch reads, what does nothing and reads no such variable: the
        // element's tree is the order the program makes them in.
        // What does nothing and reads what no call changes, a `let` no
        // closure sets, passes what does something too.
        let acts = value.has_effects();
        let element = captured.contains(name);
        let unchanged_by_calls = !acts && read_past_calls(value, set, plain);
        let passes = |e: &Expr| match acts {
            false => unchanged_by_calls || !e.has_effects(),
            true if element => {
                let mut unchanged = !e.has_effects();
                e.visit_vars(&mut |v| unchanged &= !set.contains(v));
                unchanged
            }
            true => read_past_calls(e, set, plain),
        };
        // The statement that reads it, past `const`s of what it may be read
        // past, `const list$1 = list;` a callback's borrow.
        let at = (rest.iter()).position(|s| js::mentions_in(std::slice::from_ref(s), name) > 0);
        // And what does nothing and reads what no call changes, past any
        // statement that assigns nothing it reads, a loop's too.
        let mut reads = Vec::new();
        value.visit_vars(&mut |v| reads.push(v.to_string()));
        let past = |at: usize| {
            (rest[..at].iter()).all(|s| matches!(&s.kind, StmtKind::Const(_, v) if passes(v)))
                || (unchanged_by_calls && reads.iter().all(|v| !assigns(&rest[..at], v)))
        };
        let Some(at) = at.filter(|&at| past(at)) else {
            i += 1;
            continue;
        };
        if !temporary || js::mentions_in(rest, name) != 1 || js::mentions_in(&rest[at..=at], name) != 1 {
            i += 1;
            continue;
        }
        let mut stable = !value.has_effects();
        value.visit_vars(&mut |v| stable &= !changing.contains(v));
        let read = match &mut rest[at].kind {
            StmtKind::Const(_, e)
            | StmtKind::Let(_, Some(e))
            | StmtKind::Expr(e)
            | StmtKind::Return(Some(e))
            | StmtKind::Throw(e)
            | StmtKind::Destructure { value: e, .. } => Some(e),
            StmtKind::Assign(target, e) if !target.has_effects() => {
                let mut in_target = false;
                target.visit_vars(&mut |v| in_target |= v == name);
                // What's set of it, `_25[k] = v`, `_25` what does nothing:
                // JS reads a target's object first, as it was read.
                let mut root = &*target;
                while let ExprKind::Index(of, _) | ExprKind::Member(of, _) = &root.kind {
                    root = of;
                }
                let set_of = matches!(&root.kind, ExprKind::Var(v) if v == name) && !value.has_effects();
                match (in_target, set_of) {
                    (false, _) => Some(e),
                    (true, true) => Some(target),
                    (true, false) => None,
                }
            }
            StmtKind::If(test, ..) => {
                let mut in_test = false;
                test.visit_vars(&mut |v| in_test |= v == name);
                in_test.then_some(test)
            }
            _ => None,
        };
        let Some(read) = read else {
            i += 1;
            continue;
        };
        // One made where a test holds, `t ? e : undefined`, may be read where
        // that test holds.
        let guard = match &value.kind {
            ExprKind::Cond(test, _, no) if matches!(no.kind, ExprKind::Undefined) => Some(&**test),
            _ => None,
        };
        if !stable && leads(read, name, guard, &passes) != Some(true) {
            i += 1;
            continue;
        }
        let value = value.clone();
        let Some(replaced) = read.substitute(&|v| (v == name).then(|| value.clone())) else {
            i += 1;
            continue;
        };
        *read = replaced;
        stmts.remove(i);
        // The one before it may now be read by the statement after it too.
        i = i.saturating_sub(1);
    }
}

/// Whether `e` reads `name` where all that JS evaluates before it in `e`
/// `passes`: `Some(true)`; `Some(false)` if something doesn't, or it's read
/// where it may not be; `None` if it isn't read.
fn leads(e: &Expr, name: &str, guard: Option<&Expr>, passes: &dyn Fn(&Expr) -> bool) -> Option<bool> {
    let mut reads = false;
    e.visit_vars(&mut |v| reads |= v == name);
    if !reads {
        return None;
    }
    let in_order = |items: &mut dyn Iterator<Item = &Expr>| {
        for item in items {
            match leads(item, name, guard, passes) {
                Some(found) => return Some(found),
                None if !passes(item) => return Some(false),
                None => {}
            }
        }
        Some(false)
    };
    match &e.kind {
        ExprKind::Var(v) => Some(v == name),
        ExprKind::Member(a, _)
        | ExprKind::OptionalMember(a, _)
        | ExprKind::Unary(_, a)
        | ExprKind::Spread(a)
        | ExprKind::Stringed(a)
        | ExprKind::Await(a) => leads(a, name, guard, passes),
        ExprKind::Index(a, b) => in_order(&mut [&**a, &**b].into_iter()),
        // Of what runs where its own test holds, `t ? e : undefined`, `b` of
        // `t && b` too: `e` runs there just where it ran.
        ExprKind::Binary(Op::And, a, b) | ExprKind::Cond(a, b, _)
            if guard.is_some_and(|g| same_test(a, g)) && leads(a, name, guard, passes).is_none() && passes(a) =>
        {
            leads(b, name, guard, passes)
        }
        // What runs only as a test says is `b`, `c` of `a ? b : c`.
        ExprKind::Binary(Op::And | Op::Or | Op::Coalesce, a, _) | ExprKind::Cond(a, ..) => {
            leads(a, name, guard, passes).or(Some(false))
        }
        ExprKind::Binary(_, a, b) => in_order(&mut [&**a, &**b].into_iter()),
        ExprKind::Array(items) | ExprKind::Template(_, items, _) => in_order(&mut items.iter()),
        ExprKind::Call(f, args) | ExprKind::New(f, args) => in_order(&mut std::iter::once(&**f).chain(args)),
        ExprKind::Object(props) => in_order(&mut props.iter().map(|p| {
            let (js::Prop::Field(_, v) | js::Prop::Getter(_, v) | js::Prop::Spread(v)) = p;
            v
        })),
        ExprKind::Jsx(jsx) => {
            let tag = match &jsx.tag {
                js::JsxTag::Component(tag) => Some(tag),
                _ => None,
            };
            let props = jsx.props.iter().map(|p| {
                let (js::Prop::Field(_, v) | js::Prop::Getter(_, v) | js::Prop::Spread(v)) = p;
                v
            });
            // JS's JSX runtime makes `key` after the children, and
            // `createElement` before them: read there, after both.
            let key = (jsx.props.iter()).find_map(|p| match p {
                js::Prop::Field(k, v) if k == "key" => Some(v),
                _ => None,
            });
            if let Some(key) = key
                && let Some(found) = leads(key, name, guard, passes)
            {
                let others = props.filter(|v| !std::ptr::eq(*v, key));
                let mut before = tag.into_iter().chain(others).chain(&jsx.children);
                return Some(found && before.all(|e| leads(e, name, guard, passes).is_none() && passes(e)));
            }
            in_order(&mut tag.into_iter().chain(props).chain(&jsx.children))
        }
        _ => Some(false),
    }
}

/// `xs.map(f)` of `xs.map((x, i) => f(x, i))`, a closure MIR says passes on
/// what it's given to a function of Rust's, which takes no more: the index
/// and the array the method gives it too change nothing (ADR 0279).
fn passed_on(e: &mut Expr, forwarding: &[js::Span]) {
    let ExprKind::Call(callee, args) = &mut e.kind else {
        return;
    };
    let ExprKind::Member(_, name) = &callee.kind else {
        return;
    };
    let [f] = &mut args[..] else { return };
    if ["map", "filter", "some", "every", "find", "forEach"].contains(&name.as_str())
        && forwarding.contains(&f.span)
        && let Some(g) = iterators::passed_on(f)
    {
        *f = g;
    }
}

/// `$index(xs, i); xs[i] = v;`, an element's place MIR checks, then sets:
/// `xs[$at(xs, i)] = v;`, the place `$at` checks, as THIR writes it. `v`
/// is made before either, as Rust makes it.
fn checked_sets(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        if let StmtKind::Expr(check) = &stmts[i].kind
            && let ExprKind::Call(f, args) = &check.kind
            && matches!(&f.kind, ExprKind::Var(v) if v == "$index")
            && let [items, index] = &args[..]
            && let StmtKind::Assign(target, _) = &stmts[i + 1].kind
            && let ExprKind::Index(of, at) = &target.kind
            && same(of, items)
            && (same(at, index) || at.as_int().is_some() && at.as_int() == index.as_int())
        {
            let checked = Expr::index(
                items.clone(),
                Expr::call(Expr::var("$at"), vec![items.clone(), index.clone()]),
            );
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
fn spread_spreads(e: &mut Expr) {
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

/// `const a = e; .. const b = a;`, where nothing else reads `a`, is `const
/// b = e;` where `a` was: `b` is read nowhere before, which JS's `const`
/// says of its block.
fn aliases(body: &mut Vec<Stmt>, captured: &HashSet<String>) {
    js::statement_lists(body, &mut |stmts| {
        let mut i = 0;
        while i < stmts.len() {
            let alias = match &stmts[i].kind {
                StmtKind::Const(a, _) => (i + 1..stmts.len())
                    .find(|&j| js::mentions_in(&stmts[j..=j], a) > 0)
                    .filter(|&j| {
                        matches!(&stmts[j].kind, StmtKind::Const(_, value)
                            if matches!(&value.kind, ExprKind::Var(read) if read == a))
                            && js::mentions_in(&stmts[j..], a) == 1
                    }),
                _ => None,
            };
            if let Some(j) = alias {
                let StmtKind::Const(b, _) = stmts.remove(j).kind else {
                    unreachable!("matched")
                };
                let StmtKind::Const(a, _) = &mut stmts[i].kind else {
                    unreachable!("matched")
                };
                // A variable the program names, of one MIR's lowering made
                // for a prop, `const code = activeCode`, or a temporary,
                // `const _5 = props`: the variable, read where that was.
                if !is_temporary(a) && (captured.contains(&b) || is_temporary(&b)) {
                    let a = a.clone();
                    js::each_expr_mut(&mut stmts[j..], &mut |e| {
                        if let ExprKind::Var(v) = &mut e.kind
                            && *v == b
                        {
                            v.clone_from(&a);
                        }
                    });
                } else {
                    *a = b;
                }
            }
            i += 1;
        }
    });
}

/// A `let x;` MIR declares first, where it's first named, as THIR declares
/// it: before the first statement that names it, and in the one branch of
/// an `if` that does; in a loop's body that assigns it before it's read.
fn sunk_lets(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        let StmtKind::Let(name, None) = &stmts[i].kind else {
            i += 1;
            continue;
        };
        let name = name.clone();
        let names = |stmt: &Stmt| js::mentions_in(std::slice::from_ref(stmt), &name) > 0;
        let Some(first) = (i + 1..stmts.len()).find(|&j| names(&stmts[j])) else {
            i += 1;
            continue;
        };
        let alone = !stmts[first + 1..].iter().any(names);
        let declared = stmts.remove(i);
        let first = first - 1;
        if alone
            && let StmtKind::If(test, then, otherwise) = &mut stmts[first].kind
            && !test.mentions_var(&name)
        {
            let branch = match (js::mentions_in(then, &name) > 0, otherwise) {
                (true, Some(otherwise)) if js::mentions_in(otherwise, &name) > 0 => None,
                (true, _) => Some(then),
                (false, Some(otherwise)) => Some(otherwise),
                (false, None) => None,
            };
            if let Some(branch) = branch {
                branch.insert(0, declared);
                continue;
            }
        }
        // Into a labeled block, a block.
        if alone && let StmtKind::Labeled(_, body) = &mut stmts[first].kind {
            body.insert(0, declared);
            continue;
        }
        // Into a loop whose body assigns it before anything reads it, each
        // turn: the variable Rust declares in the body, each turn its own.
        if alone
            && let StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } | StmtKind::While { body, .. } =
                &mut stmts[first].kind
            && assigned_first(body, &name)
            && !looped_reads(&stmts[first], &name)
        {
            let (StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } | StmtKind::While { body, .. }) =
                &mut stmts[first].kind
            else {
                unreachable!("matched")
            };
            body.insert(0, declared);
            continue;
        }
        // Only `let`s between, where it is; otherwise, what followed it is
        // next.
        if stmts[i..first].iter().all(|s| matches!(s.kind, StmtKind::Let(_, None))) {
            stmts.insert(i, declared);
            i += 1;
        } else {
            stmts.insert(first, declared);
        }
    }
}

/// Whether `stmts` assign `name` before anything reads it, each way through
/// them that names it: first `name = e`, or a branch of an `if` that does
/// so, which is all that names it; or an `if` each of whose branches does
/// so or leaves the turn first, `let y = match x { 0 => continue, .. }`'s.
fn assigned_first(stmts: &[Stmt], name: &str) -> bool {
    let names = |s: &Stmt| js::mentions_in(std::slice::from_ref(s), name) > 0;
    let Some(at) = stmts.iter().position(names) else {
        return false;
    };
    let alone = !stmts[at + 1..].iter().any(names);
    let assigned = |stmts: &[Stmt]| !stmts.iter().any(names) || assigned_first(stmts, name);
    match &stmts[at].kind {
        StmtKind::Assign(target, value) => {
            matches!(&target.kind, ExprKind::Var(v) if v == name) && !value.mentions_var(name)
        }
        StmtKind::If(test, then, otherwise) => {
            !test.mentions_var(name)
                && (alone && assigned(then) && otherwise.as_deref().is_none_or(assigned)
                    || assigned_on(then, name) && otherwise.as_deref().is_some_and(|o| assigned_on(o, name)))
        }
        _ => false,
    }
}

/// Whether each way through `stmts` that goes on past them assigns `name`
/// before anything reads it: one that leaves the turn first, by `continue`,
/// `return`, `throw` or the loop's own `break`, doesn't go on. A labeled
/// `break` may go on, to the end of a block in the turn.
fn assigned_on(stmts: &[Stmt], name: &str) -> bool {
    let names = |s: &Stmt| js::mentions_in(std::slice::from_ref(s), name) > 0;
    for stmt in stmts {
        if names(stmt) {
            return assigned_first(std::slice::from_ref(stmt), name);
        }
        if matches!(
            stmt.kind,
            StmtKind::Continue(_) | StmtKind::Return(_) | StmtKind::Throw(_) | StmtKind::Break(None)
        ) {
            return true;
        }
    }
    false
}

/// Whether loop `stmt`'s own test, start or what it steps names `name`.
fn looped_reads(stmt: &Stmt, name: &str) -> bool {
    match &stmt.kind {
        StmtKind::While { cond, .. } => cond.mentions_var(name),
        StmtKind::For { start, test, .. } => start.mentions_var(name) || test.mentions_var(name),
        StmtKind::ForOf { iterable, .. } => iterable.mentions_var(name),
        _ => true,
    }
}

/// `let x; .. x = e;` is `.. let x = e;`, where nothing between names `x`,
/// nor does `e`; and a `let` nothing assigns after, a closure neither, is
/// a `const`.
fn declarations(body: &mut Vec<Stmt>) {
    js::statement_lists(body, &mut |stmts| {
        let mut i = 0;
        while i < stmts.len() {
            let first = match &stmts[i].kind {
                // What it's declared with does nothing, and is never read.
                StmtKind::Let(name, init) if init.as_ref().is_none_or(|e| !e.has_effects()) => (i + 1..stmts.len())
                    .find(|&j| js::mentions_in(&stmts[j..=j], name) > 0)
                    .filter(|&j| {
                        matches!(&stmts[j].kind, StmtKind::Assign(target, _)
                            if matches!(&target.kind, ExprKind::Var(assigned) if assigned == name))
                            && js::mentions_in(&stmts[j..=j], name) == 1
                    }),
                _ => None,
            };
            let Some(j) = first else {
                i += 1;
                continue;
            };
            let StmtKind::Let(name, _) = stmts.remove(i).kind else {
                unreachable!("matched")
            };
            let assign = &mut stmts[j - 1];
            let StmtKind::Assign(_, value) = std::mem::replace(&mut assign.kind, StmtKind::Break(None)) else {
                unreachable!("matched")
            };
            assign.kind = StmtKind::Let(name, Some(value));
        }
    });
    let assigned = assigned(body);
    js::statement_lists(body, &mut |stmts| {
        for stmt in stmts.iter_mut() {
            if let StmtKind::Let(name, Some(value)) = &mut stmt.kind
                && !assigned.contains(name)
            {
                let value = std::mem::replace(value, js::Expr::undefined());
                stmt.kind = StmtKind::Const(std::mem::take(name), value);
            }
        }
    });
}

/// The variables `body` assigns, and each a handle is on, which its
/// setter assigns (ADR 0099).
fn assigned(body: &mut Vec<Stmt>) -> HashSet<String> {
    let mut assigned = HashSet::new();
    js::each_block_mut(body, &mut |stmts| {
        for stmt in stmts.iter() {
            if let StmtKind::Assign(target, _) = &stmt.kind
                && let ExprKind::Var(name) = &target.kind
            {
                assigned.insert(name.clone());
            }
        }
    });
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Handle(place) | ExprKind::Pair(place, _) = &e.kind
            && let ExprKind::Var(name) = &place.kind
        {
            assigned.insert(name.clone());
        }
    });
    assigned
}

/// `result = e; return result;` is `return e;`: nothing reads the return
/// place once it's returned, nor could a closure.
fn returned(stmts: &mut Vec<Stmt>, result: &str) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        let assigns = matches!(&stmts[i].kind, StmtKind::Assign(target, _)
            if matches!(&target.kind, ExprKind::Var(name) if name == result));
        let returns = matches!((&stmts[i].kind, &stmts[i + 1].kind), (StmtKind::Assign(target, _), StmtKind::Return(Some(value)))
            if matches!((&target.kind, &value.kind), (ExprKind::Var(a), ExprKind::Var(b)) if a == b));
        if assigns && returns {
            let StmtKind::Assign(_, value) = stmts.remove(i).kind else {
                unreachable!("matched")
            };
            stmts[i].kind = StmtKind::Return(Some(value));
        }
        i += 1;
    }
}

/// A range made where it's stepped, `$iterator({ start: 0, end: n },
/// $rangeNext)`, which nothing else steps, nor lends: its numbers, made as
/// they're stepped, `$lazyRange(0, n)`, so `.find(f)` stops where Rust's
/// does; of `a..=b`, `$lazyRange(a, b + 1)`. Of a number it starts at, as a
/// `char`'s range starts at text.
fn range_items(e: &mut Expr) {
    if let Some(made) = lazy_range(iterators::unlent(e)) {
        *e = made;
    }
}

/// `$lazyRange(a, b)` of `$iterator({ start: a, end: b }, $rangeNext)`, `a` a
/// number (`range_items`).
fn lazy_range(e: &Expr) -> Option<Expr> {
    let ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    let (ExprKind::Var(iterator), [range, next]) = (&callee.kind, &args[..]) else {
        return None;
    };
    let (ExprKind::Object(bounds), ExprKind::Var(next)) = (&range.kind, &next.kind) else {
        return None;
    };
    let [Prop::Field(s, start), Prop::Field(f, end)] = &bounds[..] else {
        return None;
    };
    let inclusive = match next.as_str() {
        "$rangeNext" => false,
        "$rangeInclusiveNext" => true,
        _ => return None,
    };
    if iterator != "$iterator"
        || (s.as_str(), f.as_str()) != ("start", "end")
        || !matches!(start.kind, ExprKind::Num(_))
    {
        return None;
    }
    let end = match inclusive {
        true => match end.as_int() {
            Some(n) => Expr::int(n + 1),
            None => Expr::bin(Op::Add, end.clone(), Expr::int(1)),
        },
        false => end.clone(),
    };
    Some(Expr::call(Expr::var("$lazyRange"), vec![start.clone(), end]))
}

/// A map of an array's items nothing reads, `next_back` having stepped its
/// source instead: no statement, as making it does nothing.
fn unread_maps(body: &mut Vec<Stmt>) {
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

/// A handle made only to read or write its value there, `Cell::set` of
/// `&self.n`'s `({ get value() { .. }, set value(value) { .. } }).value =
/// v`: its place, `t.n = v`, as THIR's. Of a place that reads the same
/// alone: of one that computes, `a[f()]`, the setter would compute it after
/// `v`, where the place itself is computed before.
fn handles_in_place(body: &mut [Stmt]) {
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Member(of, key) = &e.kind
            && key == "value"
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

/// `const g = $borrow(h, true)` of a handle `h` made there, a plain
/// `RefCell` field's (ADR 0362): what counts its borrows is that new
/// object, which nothing else sees, so none can fail. `g.value` is the
/// place `h` is a handle on, `$unborrow(g)` nothing, and a `try` left
/// only rethrowing what it catches, its body.
fn unguarded(body: &mut Vec<Stmt>) {
    let mut places: HashMap<String, Expr> = HashMap::new();
    let mut mixed: HashSet<String> = HashSet::new();
    let guard_of = |stmt: &Stmt| -> Option<(String, Expr)> {
        let (name, value) = match &stmt.kind {
            StmtKind::Const(name, value) => (name, value),
            StmtKind::Assign(target, value) if let ExprKind::Var(name) = &target.kind => (name, value),
            _ => return None,
        };
        let ExprKind::Call(callee, args) = &value.kind else {
            return None;
        };
        let ExprKind::Handle(place) = &args.first()?.kind else {
            return None;
        };
        (matches!(&callee.kind, ExprKind::Var(f) if f == "$borrow" || f == "$borrowMut") && place.reads_same())
            .then(|| (name.clone(), (**place).clone()))
    };
    js::each_block_mut(&mut body.clone(), &mut |stmts| {
        for stmt in stmts.iter() {
            if let Some((name, place)) = guard_of(stmt) {
                match places.get(&name) {
                    Some(seen) if !same(seen, &place) => {
                        mixed.insert(name);
                    }
                    _ => {
                        places.insert(name, place);
                    }
                }
            }
        }
    });
    places.retain(|name, _| !mixed.contains(name));
    // Each read of it its `value`, or what `$unborrow` is given.
    let mut reads: HashMap<String, usize> = HashMap::new();
    js::visit_stmts(body, &mut |v| {
        if places.contains_key(v) {
            *reads.entry(v.to_string()).or_default() += 1;
        }
    });
    let mut kept: HashMap<String, usize> = HashMap::new();
    js::each_expr_mut(&mut body.clone(), &mut |e| match &e.kind {
        ExprKind::Member(of, key)
            if key == "value"
                && let ExprKind::Var(g) = &of.kind
                && places.contains_key(g) =>
        {
            *kept.entry(g.clone()).or_default() += 1;
        }
        ExprKind::Call(callee, args)
            if matches!(&callee.kind, ExprKind::Var(f) if f == "$unborrow")
                && let [a] = args.as_slice()
                && let ExprKind::Var(g) = &a.kind
                && places.contains_key(g) =>
        {
            *kept.entry(g.clone()).or_default() += 1;
        }
        _ => {}
    });
    // Its own assignments, `_27 = $borrowMut(..)`, name it too.
    js::each_block_mut(&mut body.clone(), &mut |stmts| {
        for stmt in stmts.iter() {
            if let StmtKind::Assign(target, _) = &stmt.kind
                && let ExprKind::Var(g) = &target.kind
                && places.contains_key(g)
            {
                *kept.entry(g.clone()).or_default() += 1;
            }
        }
    });
    places.retain(|name, _| reads.get(name) == kept.get(name));
    js::each_block_mut(body, &mut |stmts| {
        stmts.retain(|stmt| !guard_of(stmt).is_some_and(|(name, _)| places.contains_key(&name)));
    });
    if places.is_empty() {
        return;
    }
    js::each_block_mut(body, &mut |stmts| {
        stmts.retain(|stmt| {
            !matches!(&stmt.kind, StmtKind::Expr(e)
            if matches!(&e.kind, ExprKind::Call(callee, args)
                if matches!(&callee.kind, ExprKind::Var(f) if f == "$unborrow")
                    && matches!(args.as_slice(), [a] if matches!(&a.kind, ExprKind::Var(g) if places.contains_key(g)))))
        });
    });
    js::each_block_mut(body, &mut |stmts| {
        let mut i = 0;
        while i < stmts.len() {
            if let StmtKind::TryCatch(_, Some(error), handler) = &stmts[i].kind
                && let [rethrow] = handler.as_slice()
                && matches!(&rethrow.kind, StmtKind::Throw(e) if matches!(&e.kind, ExprKind::Var(v) if v == error))
            {
                let StmtKind::TryCatch(inner, ..) = stmts.remove(i).kind else {
                    unreachable!("matched")
                };
                let n = inner.len();
                stmts.splice(i..i, inner);
                i += n;
                continue;
            }
            i += 1;
        }
    });
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Member(of, key) = &e.kind
            && key == "value"
            && let ExprKind::Var(g) = &of.kind
            && let Some(place) = places.get(g)
        {
            *e = Expr {
                span: e.span,
                ..place.clone()
            };
        }
    });
}

/// `const h = { get value() { return o.f; }, .. }`, a `&Cell`'s handle on a
/// field, whose `value` alone is read, never set nor the handle given:
/// `const h = o.f`, `h` where its `value` is read, as THIR reads a cell
/// field a pattern binds where it's bound (ADR 0287).
fn read_handles(body: &mut Vec<Stmt>) {
    let mut handles: Vec<String> = Vec::new();
    js::statement_lists(&mut body.clone(), &mut |stmts| {
        for stmt in stmts.iter() {
            if let StmtKind::Const(name, value) = &stmt.kind
                && let ExprKind::Handle(place) = &value.kind
                && matches!(place.kind, ExprKind::Member(..))
            {
                handles.push(name.clone());
            }
        }
    });
    handles.retain(|name| {
        // Each read of it one of its `value`, none set: none left once
        // those are read otherwise.
        let mut set = false;
        js::statement_lists(&mut body.clone(), &mut |stmts| {
            for stmt in stmts.iter() {
                if let StmtKind::Assign(target, _) = &stmt.kind {
                    target.visit_vars(&mut |v| set |= v == name);
                }
            }
        });
        let mut rest = body.clone();
        js::each_expr_mut(&mut rest, &mut |e| {
            if let ExprKind::Member(of, key) = &e.kind
                && key == "value"
                && matches!(&of.kind, ExprKind::Var(v) if v == name)
            {
                *e = Expr::undefined();
            }
        });
        !set && js::mentions_in(&rest, name) == 0
    });
    if handles.is_empty() {
        return;
    }
    js::statement_lists(body, &mut |stmts| {
        for stmt in stmts.iter_mut() {
            if let StmtKind::Const(name, value) = &mut stmt.kind
                && handles.contains(name)
                && let ExprKind::Handle(place) = &value.kind
            {
                *value = (**place).clone();
            }
        }
    });
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Member(of, key) = &e.kind
            && key == "value"
            && let ExprKind::Var(v) = &of.kind
            && handles.contains(v)
        {
            *e = Expr {
                span: e.span,
                ..Expr::var(v)
            };
        }
    });
}

/// `const n = { value: x }`, a cell, and `const m = n`, its clones, whose
/// `value` alone is read and set, by those names: `let n = x`, `n` where
/// each reads its `value`, as THIR keeps a cell only its function uses
/// (ADR 0287). Nothing else has it, so nothing else sees it change.
fn plain_cells(body: &mut Vec<Stmt>) {
    let mut cells: Vec<String> = Vec::new();
    let mut clones: HashMap<String, String> = HashMap::new();
    js::statement_lists(&mut body.clone(), &mut |stmts| {
        for stmt in stmts.iter() {
            match &stmt.kind {
                StmtKind::Const(name, value)
                    if matches!(&value.kind, ExprKind::Object(props)
                        if matches!(props.as_slice(), [js::Prop::Field(key, _)] if key == "value")) =>
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
    js::statement_lists(&mut body.clone(), &mut |stmts| {
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
            && key == "value"
            && let ExprKind::Var(v) = &of.kind
            && let Some(cell) = named.get(v)
        {
            *kept.entry(cell.clone()).or_default() += 1;
        }
    });
    js::statement_lists(&mut each, &mut |stmts| {
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
    js::statement_lists(body, &mut |stmts| {
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
            && key == "value"
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
fn named_callbacks(stmts: &mut Vec<Stmt>, callbacks: &HashSet<String>) {
    let mut i = 0;
    while i < stmts.len() {
        if let StmtKind::Const(name, value) = &stmts[i].kind
            && callbacks.contains(name)
            && matches!(value.kind, ExprKind::Arrow(..) | ExprKind::AsyncArrow(..))
            && !value.mentions_var(name)
            // Nor one reading what it would be named, `step` of `step$1`,
            // as THIR leaves it (ADR 0297).
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

/// `if (c) { s; result = a; } else { result = b; } return result;`, the
/// function's value its branches make: returned where each is made, `if (c)
/// { s; return a; } return b;`, at the `return`s the program wrote.
fn returned_in_branches(stmts: &mut Vec<Stmt>, result: &str) {
    returned_from_block(stmts, result);
    let [.., last_if, last] = stmts.as_slice() else {
        return;
    };
    let returns_result =
        matches!(&last.kind, StmtKind::Return(Some(v)) if matches!(&v.kind, ExprKind::Var(r) if r == result));
    if !returns_result
        || !matches!(last_if.kind, StmtKind::If(_, _, Some(_)))
        || !made_at_ends(std::slice::from_ref(last_if), result)
    {
        return;
    }
    let mut made = stmts[stmts.len() - 2].clone();
    returned_at_ends(std::slice::from_mut(&mut made));
    // Read nowhere else.
    if js::mentions_in(std::slice::from_ref(&made), result) > 0
        || js::mentions_in(&stmts[..stmts.len() - 2], result) > 0
    {
        return;
    }
    stmts.truncate(stmts.len() - 2);
    // An `else` after a branch that returns: what follows it.
    match made.kind {
        StmtKind::If(test, then, Some(otherwise)) if then.last().is_some_and(ends) => {
            stmts.push(StmtKind::If(test, then, None).at(made.span));
            stmts.extend(otherwise);
        }
        kind => stmts.push(Stmt { kind, span: made.span }),
    }
}

/// `l: { .. result = a; break l; .. result = b; } return result;`, a
/// function's returns MIR jumps out of a block for, a loop's too: each a
/// `return` where it's written, `return a;`, and the block's statements.
fn returned_from_block(stmts: &mut Vec<Stmt>, result: &str) {
    let [.., block, last] = stmts.as_slice() else {
        return;
    };
    let StmtKind::Labeled(label, body) = &block.kind else {
        return;
    };
    if !matches!(&last.kind, StmtKind::Return(Some(v)) if matches!(&v.kind, ExprKind::Var(r) if r == result)) {
        return;
    }
    let label = label.clone();
    let mut body = body.clone();
    js::statement_lists(&mut body, &mut |list| {
        let mut i = 0;
        while i + 1 < list.len() {
            if let StmtKind::Assign(target, value) = &list[i].kind
                && matches!(&target.kind, ExprKind::Var(r) if r == result)
                && matches!(&list[i + 1].kind, StmtKind::Break(Some(l)) if *l == label)
            {
                let value = value.clone();
                let span = list[i].span;
                list.splice(i..=i + 1, [StmtKind::Return(Some(value)).at(span)]);
            }
            i += 1;
        }
    });
    if !made_at_ends(&body, result) {
        return;
    }
    returned_at_ends(&mut body);
    let mut jumps = false;
    js::statement_lists(&mut body, &mut |list| jumps |= targets(list, &label));
    if jumps || js::mentions_in(&body, result) > 0 || js::mentions_in(&stmts[..stmts.len() - 2], result) > 0 {
        return;
    }
    stmts.truncate(stmts.len() - 2);
    stmts.extend(body);
    // `if (c) { s; return a; } else { return b; }` of a `return b` written
    // before `s`, a `let .. else`'s: `if (!c) { return b; } s; return a;`,
    // as the program wrote it.
    js::statement_lists(stmts, &mut |list| {
        for stmt in list.iter_mut() {
            if let StmtKind::If(_, then, Some(otherwise)) = &stmt.kind
                && let [
                    Stmt {
                        kind: StmtKind::Return(_),
                        span,
                    },
                ] = otherwise.as_slice()
                && then.last().is_some_and(ends)
                && !span.is_none()
                && first_written(then).is_some_and(|first| span.lo < first)
            {
                let StmtKind::If(test, then, Some(otherwise)) =
                    std::mem::replace(&mut stmt.kind, StmtKind::Break(None))
                else {
                    unreachable!("matched")
                };
                stmt.kind = StmtKind::If(not(test), otherwise, Some(then));
            }
        }
    });
}

/// Where the first of `stmts` that says is written, inside a loop's too.
fn first_written(stmts: &[Stmt]) -> Option<u32> {
    stmts.iter().find_map(|stmt| match &stmt.kind {
        _ if !stmt.span.is_none() => Some(stmt.span.lo),
        StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::While { body, .. }
        | StmtKind::Labeled(_, body) => first_written(body),
        StmtKind::If(_, then, _) => first_written(then),
        _ => None,
    })
}

/// Whether `stmts` end assigning `result`, or leaving, each branch of an
/// `if` that ends them too.
fn made_at_ends(stmts: &[Stmt], result: &str) -> bool {
    match stmts.last().map(|s| &s.kind) {
        Some(StmtKind::Assign(target, _)) => matches!(&target.kind, ExprKind::Var(r) if r == result),
        Some(StmtKind::Return(_) | StmtKind::Throw(_)) => true,
        Some(StmtKind::If(_, then, Some(otherwise))) => made_at_ends(then, result) && made_at_ends(otherwise, result),
        _ => false,
    }
}

/// `stmts`, of `made_at_ends`, returning what they assign the result.
fn returned_at_ends(stmts: &mut [Stmt]) {
    let Some(last) = stmts.last_mut() else { return };
    match &mut last.kind {
        StmtKind::Assign(_, value) => {
            let value = std::mem::replace(value, Expr::undefined());
            last.kind = StmtKind::Return(Some(value));
        }
        StmtKind::If(_, then, Some(otherwise)) => {
            returned_at_ends(then);
            returned_at_ends(otherwise);
        }
        _ => {}
    }
}

/// What follows a `return`, a `throw`, a `break` or a `continue` in the
/// same list never runs, nor what follows an `if` each of whose branches
/// ends so, nor a `while (true)` no `break` leaves.
fn unreachable_after(stmts: &mut Vec<Stmt>) {
    if let Some(at) = stmts.iter().position(ends) {
        stmts.truncate(at + 1);
    }
}

/// Whether running `stmt` never goes on to what follows it.
fn ends(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Return(_) | StmtKind::Throw(_) | StmtKind::Break(_) | StmtKind::Continue(_) => true,
        StmtKind::If(_, then, Some(otherwise)) => then.last().is_some_and(ends) && otherwise.last().is_some_and(ends),
        StmtKind::While { label, cond, body } => {
            matches!(cond.kind, ExprKind::Bool(true)) && !breaks(body, label.as_deref())
        }
        _ => false,
    }
}

/// A `break l` that ends labeled block `l`, or a `continue l` that ends
/// loop `l`'s body, where it would go on anyway: in the tails of the `if`s
/// that end them too.
fn tail_jumps(stmts: &mut [Stmt]) {
    for stmt in stmts {
        match &mut stmt.kind {
            StmtKind::Labeled(label, body) => drop_tail(body, &StmtKind::Break(Some(label.clone()))),
            StmtKind::While {
                label: Some(label),
                body,
                ..
            }
            | StmtKind::ForOf {
                label: Some(label),
                body,
                ..
            }
            | StmtKind::For {
                label: Some(label),
                body,
                ..
            } => drop_tail(body, &StmtKind::Continue(Some(label.clone()))),
            _ => {}
        }
    }
}

/// `const h = x ? x : undefined; if (h == null) { exit }`, a `let … else`
/// of what `filter` keeps: `if (!x) { exit }`, as THIR tests it, `h` and
/// what binds it then `x`, which it is; and `if (h != null) { s }`, `h`
/// read there alone, `if (x) { s }`.
fn filtered_guards(stmts: &mut Vec<Stmt>, changing: &HashSet<String>) {
    for i in 1..stmts.len() {
        let [made, checked] = &stmts[i - 1..=i] else {
            unreachable!("two")
        };
        let StmtKind::Const(h, value) = &made.kind else {
            continue;
        };
        let ExprKind::Cond(test, yes, no) = &value.kind else {
            continue;
        };
        let test = test.tested();
        let ExprKind::Var(x) = &test.kind else {
            continue;
        };
        if changing.contains(x)
            || !matches!(&yes.kind, ExprKind::Var(y) if y == x)
            || !matches!(no.kind, ExprKind::Undefined)
        {
            continue;
        }
        let StmtKind::If(guard, then, otherwise) = &checked.kind else {
            continue;
        };
        let ExprKind::Binary(op @ (Op::LooseEq | Op::LooseNe), tested, null) = &guard.kind else {
            continue;
        };
        if !matches!(&tested.kind, ExprKind::Var(t) if t == h)
            || !matches!(null.kind, ExprKind::Null | ExprKind::Undefined)
        {
            continue;
        }
        let kept = match op {
            Op::LooseEq => otherwise.is_none() && then.last().is_some_and(ends),
            _ => {
                otherwise.as_deref().is_none_or(|o| js::mentions_in(o, h) == 0)
                    && js::mentions_in(&stmts[i + 1..], h) == 0
            }
        };
        if !kept {
            continue;
        }
        let truth = match op {
            Op::LooseEq => Expr::unary(UnaryOp::Not, Expr::var(x)),
            _ => Expr::var(x),
        };
        // `h`, read as `x`: it goes.
        let (h, x) = (h.clone(), x.clone());
        stmts.remove(i - 1);
        let mut rest = stmts.split_off(i - 1);
        let StmtKind::If(guard, ..) = &mut rest[0].kind else {
            unreachable!("the test, where the `const` was")
        };
        *guard = truth;
        js::each_expr_mut(&mut rest, &mut |e| {
            if matches!(&e.kind, ExprKind::Var(v) if *v == h) {
                *e = Expr {
                    span: e.span,
                    ..Expr::var(&x)
                };
            }
        });
        stmts.extend(rest);
        return;
    }
}

/// A labeled block's `if (c) { s; break l; } t`, `s` leaving for where
/// `t` goes on to, its end: `if (c) { s } else { t }`, a `match`'s arms
/// as a person writes them, the `break`s then ending the branches; and a
/// block that ends it, one with it. Of `if (!a) { if (!b) { s; break l; }
/// } t`, an or-pattern's, `if (a || b) { t } else { s }`.
fn branch_ends(stmts: &mut [Stmt]) {
    for stmt in stmts {
        if let StmtKind::Labeled(label, body) = &mut stmt.kind {
            ending(body, label);
        }
    }
}

/// `stmts`, which end where block `label` does, as `branch_ends` has them.
fn ending(stmts: &mut Vec<Stmt>, label: &str) {
    if let Some(Stmt {
        kind: StmtKind::Labeled(inner, body),
        ..
    }) = stmts.last_mut()
    {
        let (inner, mut body) = (inner.clone(), std::mem::take(body));
        js::statement_lists(&mut body, &mut |stmts| {
            for stmt in stmts {
                if matches!(&stmt.kind, StmtKind::Break(Some(l)) if *l == inner) {
                    stmt.kind = StmtKind::Break(Some(label.to_string()));
                }
            }
        });
        stmts.pop();
        stmts.extend(body);
    }
    // Not a `let … else`'s, `if (c) { s; break l; } throw e;`: a guard.
    let exits = matches!(
        stmts.last().map(|s| &s.kind),
        Some(StmtKind::Return(_) | StmtKind::Throw(_))
    );
    let Some(at) = (stmts.iter())
        .position(|s| matches!(&s.kind, StmtKind::If(_, then, None) if leaves(&ands(then).1, label)))
        .filter(|_| !exits)
    else {
        if let Some(StmtKind::If(_, then, otherwise)) = stmts.last_mut().map(|s| &mut s.kind) {
            ending(then, label);
            if let Some(otherwise) = otherwise {
                ending(otherwise, label);
            }
        }
        return;
    };
    let rest = stmts.split_off(at + 1);
    let Some(Stmt {
        kind: StmtKind::If(test, then, None),
        span,
    }) = stmts.pop()
    else {
        unreachable!("found")
    };
    let (tests, mut then) = ands(&then);
    let mut rest = rest;
    ending(&mut then, label);
    ending(&mut rest, label);
    let tests: Vec<Expr> = std::iter::once(test).chain(tests).collect();
    let negated = tests.len() > 1
        && tests
            .iter()
            .all(|t| matches!(t.kind, ExprKind::Unary(UnaryOp::Not, _) | ExprKind::Binary(Op::Ne, ..)));
    let test = (tests.into_iter())
        .rev()
        .reduce(|b, a| Expr::bin(Op::And, a, b))
        .expect("one");
    stmts.push(match (rest.is_empty(), negated) {
        (true, _) => StmtKind::If(test, then, None).at(span),
        (false, true) => StmtKind::If(not(test), rest, Some(then)).at(span),
        (false, false) => StmtKind::If(test, then, Some(rest)).at(span),
    });
}

/// Of `if (b) { if (c) { s } }`, the body of an `if`: its tests, `[b, c]`,
/// and `s`; of others, none and themselves.
fn ands(stmts: &[Stmt]) -> (Vec<Expr>, Vec<Stmt>) {
    match stmts {
        [
            Stmt {
                kind: StmtKind::If(test, inner, None),
                ..
            },
        ] => {
            let (mut tests, body) = ands(inner);
            tests.insert(0, test.clone());
            (tests, body)
        }
        _ => (Vec::new(), stmts.to_vec()),
    }
}

/// Whether `stmts` always end with a `break label`.
fn leaves(stmts: &[Stmt], label: &str) -> bool {
    match stmts.last().map(|s| &s.kind) {
        Some(StmtKind::Break(Some(l))) => l == label,
        Some(StmtKind::If(_, then, Some(otherwise))) => leaves(then, label) && leaves(otherwise, label),
        _ => false,
    }
}

/// `jump` where it ends `stmts`, or a branch of the `if` that ends them.
fn drop_tail(stmts: &mut Vec<Stmt>, jump: &StmtKind) {
    let same = |kind: &StmtKind| match (kind, jump) {
        (StmtKind::Break(Some(a)), StmtKind::Break(Some(b)))
        | (StmtKind::Continue(Some(a)), StmtKind::Continue(Some(b))) => a == b,
        _ => false,
    };
    match stmts.last_mut().map(|s| &mut s.kind) {
        Some(kind) if same(kind) => {
            stmts.pop();
        }
        Some(StmtKind::If(_, then, otherwise)) => {
            drop_tail(then, jump);
            if let Some(otherwise) = otherwise {
                drop_tail(otherwise, jump);
            }
        }
        _ => {}
    }
}

/// Each `break from` and `continue from` in `stmts`, however deep, `to`'s.
fn relabeled(stmts: &mut Vec<Stmt>, from: &str, to: &str) {
    js::statement_lists(stmts, &mut |list| {
        for stmt in list.iter_mut() {
            if let StmtKind::Break(Some(l)) | StmtKind::Continue(Some(l)) = &mut stmt.kind
                && l == from
            {
                *l = to.to_string();
            }
        }
    });
}

/// A function that ends in a loop it leaves by one `return` MIR moved into
/// it, the value after the loop, `while (true) { if (c) { return out; } … }`:
/// the loop's `break`, the `return` after it, as a `break` is written; not
/// a `return` the program wrote, in a loop it wrote one in.
fn loop_exits(body: &mut Vec<Stmt>, returns: &[js::Span], loops: &[(js::Span, bool, Option<String>)]) {
    fn take(stmts: &mut [Stmt]) -> Option<Stmt> {
        for stmt in stmts.iter_mut() {
            let taken = match &mut stmt.kind {
                StmtKind::Return(Some(_)) => {
                    let jump = StmtKind::Break(None).at(stmt.span);
                    Some(std::mem::replace(stmt, jump))
                }
                StmtKind::If(_, then, otherwise) => take(then).or_else(|| otherwise.as_deref_mut().and_then(take)),
                StmtKind::Labeled(_, inner) => take(inner),
                _ => None,
            };
            if taken.is_some() {
                return taken;
            }
        }
        None
    }
    let Some(Stmt { kind, span }) = body.last() else { return };
    // Nor of a loop the program wrote a `return` in, which MIR may not span.
    if let Some((written, ..)) = written_loop(loops, *span)
        && returns.iter().any(|r| written.lo <= r.lo && r.hi <= written.hi)
    {
        return;
    }
    let (StmtKind::While { body: inner, .. } | StmtKind::ForOf { body: inner, .. } | StmtKind::For { body: inner, .. }) =
        kind
    else {
        return;
    };
    // Its one `return`, anywhere in it.
    let mut all = 0;
    js::statement_lists(&mut inner.clone(), &mut |list| {
        all += list.iter().filter(|s| matches!(s.kind, StmtKind::Return(_))).count();
    });
    let mut inner = inner.clone();
    let Some(ret) = take(&mut inner).filter(|_| all == 1) else {
        return;
    };
    // Of what's there after it: nothing the loop declares.
    let mut own: Vec<String> = Vec::new();
    js::statement_lists(&mut inner.clone(), &mut |list| {
        own.extend(declared(list).map(str::to_string))
    });
    match &body.last().expect("a loop").kind {
        StmtKind::ForOf { pattern, .. } => own.extend(pattern.names().into_iter().map(str::to_string)),
        StmtKind::For { name, .. } => own.push(name.clone()),
        _ => {}
    }
    if own
        .iter()
        .any(|name| js::mentions_in(std::slice::from_ref(&ret), name) > 0)
    {
        return;
    }
    if let Some(Stmt {
        kind: StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. },
        ..
    }) = body.last_mut()
    {
        *body = inner;
    }
    body.push(ret);
}

/// An or-pattern of constants as MIR tests it, each alternative failing, one
/// inside another, `if (s !== "a") { if (s !== "e") { b } } t`, where `b`
/// and `t` leave: its arm first, `if (s === "a" || s === "e") { t } b`, as
/// THIR writes one.
fn or_tests(stmts: &mut Vec<Stmt>) {
    // Each test, `s !== "a"`, of one subject, and what the last guards.
    fn chain<'s>(stmt: &'s Stmt, subject: &mut Option<&'s Expr>, tests: &mut Vec<&'s Expr>) -> Option<&'s [Stmt]> {
        let StmtKind::If(test, then, None) = &stmt.kind else {
            return None;
        };
        let ExprKind::Binary(Op::Ne, of, constant) = &test.kind else {
            return None;
        };
        if !constant.is_constant() || !subject.is_none_or(|s| same(s, of)) {
            return None;
        }
        *subject = Some(of);
        tests.push(test);
        match then.as_slice() {
            [inner] if let Some(body) = chain(inner, subject, tests) => Some(body),
            body => Some(body),
        }
    }
    let mut i = 0;
    while i + 1 < stmts.len() {
        let mut tests = Vec::new();
        let made = chain(&stmts[i], &mut None, &mut tests).and_then(|body| {
            let rest = &stmts[i + 1..];
            (tests.len() > 1
                && body.last().is_some_and(ends)
                && rest.last().is_some_and(ends)
                && !declared(body).any(|name| declared(rest).any(|n| n == name)))
            .then(|| {
                let test = (tests.iter())
                    .map(|t| not((*t).clone()))
                    .reduce(|a, b| Expr::bin(Op::Or, a, b))
                    .expect("two");
                (test, body.to_vec())
            })
        });
        if let Some((test, body)) = made {
            let span = stmts[i].span;
            let rest: Vec<Stmt> = stmts.drain(i + 1..).collect();
            stmts[i] = StmtKind::If(test, rest, None).at(span);
            stmts.extend(body);
            return;
        }
        i += 1;
    }
}

/// A pattern's test as MIR makes it of a bound a type has anyway, a
/// constant, `if (true) { s }`: its branch, `s`, as THIR writes a range's.
fn pattern_tests(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        // A constant's branch, which declares nothing its neighbors do.
        let constant = match &stmts[i].kind {
            StmtKind::If(test, then, otherwise) if let ExprKind::Bool(holds) = test.kind => {
                let mut neighbors = declared(&stmts[..i]).chain(declared(&stmts[i + 1..]));
                let ours: Vec<&str> = declared(then)
                    .chain(otherwise.iter().flat_map(|o| declared(o)))
                    .collect();
                (!neighbors.any(|n| ours.contains(&n))).then_some(holds)
            }
            _ => None,
        };
        if let Some(holds) = constant {
            let StmtKind::If(_, then, otherwise) = stmts.remove(i).kind else {
                unreachable!("matched")
            };
            let kept = match holds {
                true => then,
                false => otherwise.unwrap_or_default(),
            };
            let n = kept.len();
            stmts.splice(i..i, kept);
            i += n;
            continue;
        }
        i += 1;
    }
}

/// The loop the program wrote that starts where `span` does, innermost.
fn written_loop(
    loops: &[(js::Span, bool, Option<String>)],
    span: js::Span,
) -> Option<&(js::Span, bool, Option<String>)> {
    (loops.iter())
        .filter(|(l, ..)| !span.is_none() && l.lo <= span.lo && span.hi <= l.hi)
        .min_by_key(|(l, ..)| l.hi - l.lo)
}

/// A loop MIR labels by its block, `bb2: for (…)`, the program labeled:
/// its label, `rows: for (…)`, where no loop around it has that one.
fn named_labels(stmts: &mut [Stmt], loops: &[(js::Span, bool, Option<String>)]) {
    fn walk(stmts: &mut [Stmt], loops: &[(js::Span, bool, Option<String>)], around: &mut Vec<String>) {
        for stmt in stmts.iter_mut() {
            let span = stmt.span;
            let (label, body) = match &mut stmt.kind {
                StmtKind::While { label, body, .. }
                | StmtKind::ForOf { label, body, .. }
                | StmtKind::For { label, body, .. } => (label, body),
                _ => continue,
            };
            if let Some(from) = label.clone()
                && from.starts_with("bb")
                && let Some((_, _, Some(to))) = written_loop(loops, span)
                && !around.contains(to)
            {
                relabeled(body, &from, to);
                *label = Some(to.clone());
            }
            around.push(label.clone().unwrap_or_default());
            walk(body, loops, around);
            around.pop();
        }
    }
    walk(stmts, loops, &mut Vec::new());
}

/// A label nothing breaks to or continues: a block's statements in its
/// place, where they declare nothing its neighbors do; a loop, unlabeled.
fn unlabeled(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        let splice = match &stmts[i].kind {
            StmtKind::Labeled(label, body) if !targets(body, label) => {
                let inner: HashSet<&str> = declared(body).collect();
                !stmts
                    .iter()
                    .enumerate()
                    .any(|(j, s)| j != i && declared(std::slice::from_ref(s)).any(|name| inner.contains(name)))
            }
            _ => false,
        };
        if splice {
            let StmtKind::Labeled(_, body) = stmts.remove(i).kind else {
                unreachable!("matched")
            };
            stmts.splice(i..i, body);
            // What it held is looked at again: a label inside it, now here.
            continue;
        }
        if let StmtKind::While { label, body, .. }
        | StmtKind::ForOf { label, body, .. }
        | StmtKind::For { label, body, .. } = &mut stmts[i].kind
            && label.as_deref().is_some_and(|l| !targets(body, l))
        {
            *label = None;
        }
        i += 1;
    }
}

/// Whether a `break` or a `continue` in `stmts` names `label`.
fn targets(stmts: &[Stmt], label: &str) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Break(Some(l)) | StmtKind::Continue(Some(l)) => l == label,
        StmtKind::If(_, a, b) => targets(a, label) || b.as_deref().is_some_and(|b| targets(b, label)),
        StmtKind::While { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Labeled(_, body) => targets(body, label),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => targets(a, label) || targets(b, label),
        _ => false,
    })
}

/// The names `stmts` declare, not in blocks inside them.
fn declared(stmts: &[Stmt]) -> impl Iterator<Item = &str> {
    stmts.iter().filter_map(|s| match &s.kind {
        StmtKind::Const(name, _) | StmtKind::Let(name, _) => Some(name.as_str()),
        _ => None,
    })
}
