//! A body lowered from its MIR, as a person writes it (ADR 0364): each
//! rewrite changes nothing that runs. MIR's one return block is a `return`
//! where each path ends, and what the structure needed to reach it, a
//! labeled block or a `break`, goes. A closure's body is tidied as its own.

use std::collections::{HashMap, HashSet};

use crate::js::{self, Expr, ExprKind, Op, Stmt, StmtKind, UnaryOp, breaks, ends, names_break};

use crate::lower::iterators::{self, eager_chains, eager_spreads, iterated};

mod bindings;
mod branches;
mod decisions;
mod destructuring;
mod flags;
mod jumps;
mod loops;
mod values;

pub(super) use self::bindings::{HELD_ARG, readable_temporaries, temporary, unsuffixed_params, user_name};
pub(super) use self::destructuring::{copied_params, destructured_params};
pub(super) use self::flags::named_flags;
pub(super) use self::values::{call_path, field_path, writer_body};
use self::{bindings::*, branches::*, destructuring::*, jumps::*, loops::*, values::*};

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
    /// Where each `loop` a `break` gives a value is written, `break a`.
    pub(super) valued: &'h [js::Span],
    /// Where each `if` the program wrote of a `continue` tests: one alone,
    /// or one that ends what it does.
    pub(super) continues: &'h [js::Span],
    /// Where each `&&` or `||` the program wrote is.
    pub(super) logicals: &'h [js::Span],
    /// The closures a `let` names that a hook's function alone is (ADR 0297).
    pub(super) callbacks: &'h HashSet<String>,
    /// Where each `match` that reads a table by its key is (ADR 0233).
    pub(super) tables: &'h [(js::Span, Vec<String>)],
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
    /// The variables of Rust's integers: JS's numbers, which `++` steps.
    pub(super) numbers: &'h HashSet<String>,
    /// The variables of plain values, which nothing changes, and `...items`
    /// of arrays whose items nothing changes (`Locals::plain`).
    pub(super) plain: &'h HashSet<String>,
    /// Where each closure that passes on what it's given to Rust's function
    /// is written (ADR 0279).
    pub(super) forwarding: &'h [js::Span],
    /// Where each closure of one call whose JS gives `undefined` is written:
    /// a component's handler of one, `() => f()` (ADR 0040).
    pub(super) concise: &'h [js::Span],
    /// Each drop flag, `noisy$live`: whether a value is there to drop.
    pub(super) flags: &'h HashSet<String>,
}

/// `body`, tidied, by what `hints` say of it.
pub(super) fn tidy(body: &mut Vec<Stmt>, hints: &Hints) {
    let Hints {
        result,
        falsy,
        tested,
        conditions,
        loops: written,
        valued,
        continues,
        logicals,
        callbacks,
        tables,
        patterns,
        closure,
        returns,
        captured,
        booleans,
        numbers,
        plain,
        forwarding,
        concise,
        flags,
    } = *hints;
    // What branches make a value of: the function's, and where an `if` is written.
    let made = (result, conditions);
    // A guard of a handle made for it alone: the place it's a handle on.
    crate::lower::std_types::cell::unguarded(body);
    // Where a drop's flag is known, no test of it, nor a `try` for it.
    flags::settled_flags(body, flags);
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
        bottom_up(body, &mut |stmts| branch_values(stmts, returns, booleans, made));
        js::statement_lists(body, &mut unlabeled);
    }
    bottom_up(body, &mut or_tests);
    bottom_up(body, &mut |stmts| branch_values(stmts, returns, booleans, made));
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
    let mut taken = mentions.clone();
    bottom_up(body, &mut |stmts| loops(stmts, &mut taken, numbers));
    // A match's tree of tests, its arms reached through a block MIR made:
    // the chain of its arms, now that the loops are made.
    bottom_up(body, &mut decisions::decisions);
    js::statement_lists(body, &mut early_returns);
    js::statement_lists(body, &mut |stmts| early_breaks(stmts));
    settle_jumps(body);
    // A `let` declared first, its one write now beside it, what a loop
    // held: a `const` there.
    declarations(body);
    js::statement_lists(body, &mut |stmts| continued(stmts, continues));
    js::statement_lists(body, &mut |stmts| tail_jumps(stmts));
    js::statement_lists(body, &mut |stmts| innermost_jumps(stmts));
    // A chain of tests a loop ended, its body now ending where the chain's does.
    bottom_up(body, &mut |stmts| branch_values(stmts, returns, booleans, made));
    js::statement_lists(body, &mut unlabeled);
    js::statement_lists(body, &mut |stmts| {
        logical_returns(stmts, logicals, conditions, returns, booleans)
    });
    // What's given where `false` is skipped as `undefined` is, `test &&
    // value`: one read there alone, which nothing else tells from
    // `undefined`.
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
    js::each_expr_mut(body, &mut crate::lower::std_types::range::range_items);
    eager_chains(body);
    js::statement_lists(body, &mut |stmts| eager_spreads(stmts));
    // A block, or branches, a function's body or a loop's ends in, the function's value made
    // in them, `?`'s.
    js::statement_lists(body, &mut |stmts| returned_in_branches(stmts, result));
    js::statement_lists(body, &mut |stmts| tail_continues(stmts, continues));
    // Each `let` where it's first named, a `const` where it's written once.
    js::statement_lists(body, &mut sunk_lets);
    declarations(body);
    // What those `const`s now read once, where it's read, and the values
    // of branches that leaves.
    // A pattern's copy of a variable, `const error = raw;`: the variable.
    js::statement_lists(body, &mut |stmts| copied_bindings(stmts));
    // A tuple read item by item, `children[0]`: its items.
    let changing = self::changing(body);
    let set_elsewhere = set_elsewhere(body);
    js::statement_lists(body, &mut |stmts| {
        tuple_reads(stmts, &changing, &set_elsewhere, captured)
    });
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
    bottom_up(body, &mut |stmts| branch_values(stmts, returns, booleans, made));
    js::statement_lists(body, &mut |stmts| table_reads(stmts, tables));
    declarations(body);
    js::statement_lists(body, &mut |stmts| inlined(stmts, &changed, &set, plain, captured));
    // A tuple's part held, `const _3$0 = v.pop();`, which an arm's binding
    // takes, by that name.
    named_temporaries(body);
    js::each_expr_mut(body, &mut |e| implied_tests(e, &changed));
    // A chain read in place now, `$lent($iter(xs)).some(f)` of `const _7 =
    // $iter(xs);`: the array's; and a range made there, its
    // numbers made as they're stepped.
    js::each_expr_mut(body, &mut crate::lower::std_types::range::range_items);
    eager_chains(body);
    // Of each, a callback that passes what it's given on to Rust's function,
    // `(x, i) => f(x, i)`: `f` itself.
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
    // A value a block MIR made gives where nothing else does: given first,
    // and the block's `break`s that go where it ends anyway, gone.
    js::statement_lists(body, &mut |stmts| decisions::default_values(stmts));
    settle_jumps(body);
    declarations(body);
    // `x != null && typeof x === "string"`: the `typeof` test, which says it.
    js::each_expr_mut(body, &mut typeof_tests);
    // A loop's test those read in place now, `while (true) { if (!(m.length
    // > n)) { .. } }`'s of `const _7 = m.length;`.
    js::statement_lists(body, &mut |stmts| while_tests(stmts, written));
    // What those leave at the end, a `return;` the program didn't write.
    tail_returns(body, returns);
    // A loop's early exits again, of what's taken its shape since.
    js::statement_lists(body, &mut early_returns);
    // A chain's arms, as statements, in the order written.
    js::statement_lists(body, &mut |stmts| written_chains(stmts));
    // `const result = e; return result;` of what the function gives: `return e;`.
    js::statement_lists(body, &mut |stmts| returned_consts(stmts, result));
    // And their arms that do nothing.
    // A cell a pointer's taken to, made where its value's first set.
    js::statement_lists(body, &mut first_cell_values);
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
    // read in place: `() => f()`.
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
    loop_exits(body, returns, written, valued);
    js::statement_lists(body, &mut crate::lower::std_types::map::counted_entries);
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
    // A value a function given it reads more than once, held where it's
    // made: `const _$1 = f(); .. _$1.TAG ..`, not `((r) => ..)(f())`.
    let mut taken = HashSet::new();
    js::names_in(body, &mut taken);
    js::statement_lists(body, &mut |stmts| applied_values(stmts, &set, plain, &mut taken));
    // A `let` declared first that nothing's left to assign or read.
    let mut mentions: HashMap<String, usize> = HashMap::new();
    js::visit_stmts(body, &mut |name| *mentions.entry(name.to_string()).or_default() += 1);
    body.retain(|stmt| !matches!(&stmt.kind, StmtKind::Let(name, None) if !mentions.contains_key(name)));
    // A `const` nothing reads, of a value made only to be dropped, `drop(vec![..])`'s:
    // what making it does.
    js::statement_lists(body, &mut |stmts| unread_made(stmts, &mentions));
    // Its jumps then, of the loop they're now in, unnamed.
    let mut headed = false;
    js::statement_lists(body, &mut |stmts| headed |= while_heads(stmts));
    js::statement_lists(body, &mut |stmts| written_order_steps(stmts));
    js::statement_lists(body, &mut tried_first);
    if headed {
        js::statement_lists(body, &mut |stmts| innermost_jumps(stmts));
        settle_jumps(body);
    }
    // `!(b >= 48)` of what's never `NaN`: `b < 48`.
    js::each_expr_mut(body, &mut |e| turned(e, numbers));
    // An `if` of `false`, a check that can't fail once what it reads is
    // in place: its `else`, if any.
    js::statement_lists(body, &mut never_taken);
    // For the source map: a `return` given the value it returns, at MIR's
    // end of the body, which the map has nothing of, where that value was
    // written; an `if`, where its test says, `if c` or its arm's pattern.
    js::statement_lists(body, &mut |stmts| {
        for stmt in stmts.iter_mut() {
            match &stmt.kind {
                StmtKind::Return(Some(value)) if stmt.span.is_none() => stmt.span = value.span,
                StmtKind::If(test, ..) if !test.span.is_none() => stmt.span = test.span,
                _ => {}
            }
        }
    });
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
        *body = vec![StmtKind::If(test.negated_test(), rest, Some(then)).at(first.span)];
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
    while let ExprKind::Member(of, _) | ExprKind::Index(of, _) = &root.kind {
        root = of;
    }
    let plain_root = matches!(&root.kind, ExprKind::Var(v) if plain.contains(v) && !set.contains(v));
    match &e.kind {
        ExprKind::Var(v) => !set.contains(v),
        // An item of a plain value, `pair[0]`, as a field of one is.
        ExprKind::Index(_, index) => plain_root && past(index),
        ExprKind::Symbol(_) => true,
        ExprKind::Arrow(..) | ExprKind::AsyncArrow(..) => true,
        ExprKind::Member(of, _) => {
            plain_root
                || matches!(&of.kind, ExprKind::Var(v)
                    if !set.contains(v) && matches!(v.as_str(), "Object" | "Math" | "JSON" | "Array" | "Number" | "String" | "console"))
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
        // Text or a number made of such, `${a}-${b}`, `a + 1`, `String(n)`,
        // and a call of the runtime's that only reads, `$debugStr(s)`.
        ExprKind::Template(_, values, _) => values.iter().all(past),
        ExprKind::Stringed(a) | ExprKind::Unary(_, a) => past(a),
        ExprKind::Binary(_, a, b) => past(a) && past(b),
        ExprKind::Call(callee, args) if callee.helper_name().is_some_and(crate::runtime::is_pure) => {
            args.iter().all(past)
        }
        // A function called at once, `((value) => ..)(value$1)`: its body,
        // its parameter what it's given.
        ExprKind::Call(callee, args)
            if let ExprKind::Arrow(params, body) = &callee.kind
                && let [js::Pattern::Name(param)] = params.as_slice()
                && let [
                    Stmt {
                        kind: StmtKind::Return(Some(result)),
                        ..
                    },
                ] = body.as_slice()
                && let [given] = args.as_slice() =>
        {
            let mut plain = plain.clone();
            plain.remove(param);
            past(given) && read_past_calls(result, set, &plain)
        }
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
            let (js::Prop::Field(_, v) | js::Prop::Getter(_, v) | js::Prop::Setter(_, v) | js::Prop::Spread(v)) = p;
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

/// The variables what's called may set: a closure's, and a getter's and
/// setter's handle's, `{ get value() { return n; }, set value(v) { n = v; } }`.
fn set_elsewhere(body: &[Stmt]) -> HashSet<String> {
    let mut set = closures_set(body);
    set.extend(handled(body));
    set
}

/// The variables a getter's and setter's handle is made of, which what it's
/// given may set.
fn handled(body: &[Stmt]) -> HashSet<String> {
    let mut handled = HashSet::new();
    js::each_expr_mut(&mut body.to_vec(), &mut |e| {
        if let ExprKind::Handle(place) | ExprKind::Pair(place, _) = &e.kind
            && let ExprKind::Var(name) = &place.kind
        {
            handled.insert(name.clone());
        }
    });
    handled
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{is_temporary, settle_jumps, spread_spreads, temporary, turned, user_name};
    use crate::js::{self, Expr, ExprKind, Op, Prop, StmtKind, UnaryOp};

    fn call(f: &str) -> js::Stmt {
        StmtKind::Expr(Expr::call(Expr::var(f), Vec::new())).at(js::Span::NONE)
    }

    // A labeled block MIR made, `l: { a(); break l; }`, its `break` going
    // where the block goes on anyway: `a();`.
    #[test]
    fn settled_jumps_leave_no_block_to_break() {
        let mut body = vec![
            StmtKind::Labeled(
                "l".into(),
                vec![call("a"), StmtKind::Break(Some("l".into())).at(js::Span::NONE)],
            )
            .at(js::Span::NONE),
        ];
        settle_jumps(&mut body);
        assert!(matches!(
            body.as_slice(),
            [js::Stmt {
                kind: StmtKind::Expr(_),
                ..
            }]
        ));
    }

    // `!(n < 5)` of an integer is `n >= 5`; of a float, which may be `NaN`,
    // it stays.
    #[test]
    fn a_test_of_an_integer_is_turned_not_a_floats() {
        let not_less = |x: &str| Expr::unary(UnaryOp::Not, Expr::bin(Op::Lt, Expr::var(x), Expr::int(5)));
        let numbers: HashSet<String> = ["n".to_string()].into();
        let mut of_int = not_less("n");
        turned(&mut of_int, &numbers);
        assert!(matches!(of_int.kind, ExprKind::Binary(Op::Ge, ..)));
        let mut of_float = not_less("x");
        turned(&mut of_float, &numbers);
        assert!(matches!(of_float.kind, ExprKind::Unary(UnaryOp::Not, _)));
    }

    // A temporary's name is one tidy knows; a program's of that shape isn't.
    #[test]
    fn a_temporary_is_known_by_its_name_and_a_users_never_is() {
        assert!(is_temporary(&temporary(5)));
        assert!(is_temporary(&crate::names::given_apart(&temporary(5), 1)));
        assert!(!is_temporary(&user_name("_5")));
        assert_eq!(user_name("total"), "total");
    }

    /// An element captured twice spills its props twice, `{...{ ...{ ...post } }}`,
    /// as react.dev's home page gives each BlogCard a post: `{...post}`.
    #[test]
    fn a_copy_of_a_copy_spreads_what_it_copies() {
        let copy = |of: Expr| Expr::object(vec![Prop::Spread(of)]);
        let mut card = Expr::jsx(js::Jsx {
            tag: js::JsxTag::Component(Expr::var("BlogCard")),
            props: vec![Prop::Spread(copy(copy(Expr::var("post"))))],
            children: Vec::new(),
        });
        spread_spreads(&mut card);
        let ExprKind::Jsx(jsx) = &card.kind else {
            unreachable!("an element")
        };
        assert!(matches!(jsx.props.as_slice(), [Prop::Spread(Expr { kind: ExprKind::Var(v), .. })] if v == "post"));
    }
}
