//! A body lowered from its MIR, as a person writes it (ADR 0364): each
//! rewrite changes nothing that runs. MIR's one return block is a `return`
//! where each path ends, and what the structure needed to reach it, a
//! labeled block or a `break`, goes. A closure's body is tidied as its own.

use std::collections::{HashMap, HashSet};

use crate::js::{self, Expr, ExprKind, Op, Stmt, StmtKind, UnaryOp, breaks, ends, names_break};

use crate::lower::iterators::{self, eager_chains, eager_spreads, iterated};

mod bindings;
mod branches;
mod contracts;
mod decisions;
mod destructuring;
mod flags;
mod jumps;
mod logicals;
mod loops;
mod stages;
mod switches;
mod unwinds;
mod values;

pub(super) use self::bindings::{HELD_ARG, is_temporary, temporary, unsuffixed_params, user_name};
pub(super) use self::destructuring::{copied_params, destructured_params};
pub(super) use self::values::{call_path, field_path, writer_body};
use self::{
    bindings::*, branches::*, destructuring::*, jumps::*, logicals::*, loops::*, switches::switched, values::*,
};
use super::hints::Switch;

/// What a body's last passes ask of its locals, by name: those the program
/// names, and its `String`s.
pub(super) struct Named {
    pub(super) user: HashSet<String>,
    pub(super) text: HashSet<String>,
}

/// A body tidied, once its parameters are taken apart: its temporaries
/// named, its drop flags too, and then, its temporaries named alike, each
/// run of statements a value's drop ends one `try { .. } finally { .. }`
/// (ADR 0437), what a `try` declared before it, `let x;`, where it's made;
/// a shadow of a binding the program names read no more, that binding
/// (ADR 0650); and a `String` appended to twice, once (ADR 0652).
pub(super) fn named(
    params: &[js::Pattern],
    body: &mut Vec<Stmt>,
    flags: &HashSet<String>,
    suggested: &HashMap<String, Vec<String>>,
    locals: &Named,
) {
    bindings::readable_temporaries(params, body, suggested);
    flags::named_flags(body, flags);
    js::statement_lists(body, &mut unwinds::scoped_drops);
    bindings::declarations(body);
    js::statement_lists(body, &mut unwinds::returned_through_finally);
    bindings::shadowed(params, body, &locals.user);
    crate::lower::std_types::text::appended(body, &locals.text);
}

/// What MIR's lowering, and THIR, say of a body for tidying it.
pub(super) struct Hints<'h> {
    /// The name of the function's return place.
    pub(super) result: &'h str,
    /// The temporaries a function that skips what's falsy is given.
    pub(super) falsy: &'h HashSet<String>,
    /// And the temporaries such a `test && value` tests.
    pub(super) tested: &'h HashSet<String>,
    /// Where the program writes its `if`s, loops, `&&`s, tables and
    /// comments (`hints::Written`).
    pub(super) written: &'h super::hints::Written,
    /// The closures a `let` names that a hook's function alone is (ADR 0297).
    pub(super) callbacks: &'h HashSet<String>,
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
    /// What `Iterator::fuse`'s compiled body is called here, `iterator_fuse`,
    /// where the module imports it: a `Fuse` stepped till it ends is its
    /// source (ADR 0561).
    pub(super) fuse: Option<&'h str>,
    /// The pass to stop after, `--tidy-until`'s, `stage/pass`.
    pub(super) until: Option<&'h str>,
}

/// `body`, tidied, by what `hints` say of it: each stage's passes in turn
/// (`stages::STAGES`), up to `hints.until`'s where it names one, and in a
/// debug build, what each stage leaves checked (ADR 0659).
pub(super) fn tidy(body: &mut Vec<Stmt>, hints: &Hints) {
    for stage in stages::STAGES {
        for pass in stage.passes {
            (pass.run)(body, hints);
            if hints.until == Some(&*pass_name(stage, pass)) {
                return;
            }
        }
        if cfg!(debug_assertions) {
            for contract in contracts::ALWAYS.iter().chain(stage.ensures) {
                if let Some(why) = (contract.broken)(body) {
                    panic!("tidy's `{}` stage broke \"{}\": {why}", stage.name, contract.name);
                }
            }
        }
    }
}

/// What `--tidy-until` names a pass by: `stage/pass`.
fn pass_name(stage: &stages::Stage, pass: &stages::Pass) -> String {
    format!("{}/{}", stage.name, pass.name)
}

/// Each pass's name, in the order they run: what `--tidy-until` may name.
pub(crate) fn pass_names() -> Vec<String> {
    (stages::STAGES.iter())
        .flat_map(|stage| stage.passes.iter().map(move |pass| pass_name(stage, pass)))
        .collect()
}

/// The function's return place, `let` first, where nothing reads it now.
fn unread_result(body: &mut Vec<Stmt>, result: &str) {
    let mut read = HashSet::new();
    js::visit_stmts(body, &mut |name| {
        read.insert(name.to_string());
    });
    if !read.contains(result) {
        body.retain(|s| !matches!(&s.kind, StmtKind::Let(name, None) if name == result));
    }
}

/// What's given where `false` is skipped as `undefined` is, `test &&
/// value`: one read there alone, which nothing else tells from
/// `undefined`. A test of one read there alone is read for its truth:
/// `!!a || b`'s, `a || b`.
fn falsy_values(body: &mut Vec<Stmt>, falsy: &HashSet<String>, tested: &HashSet<String>) {
    let falsy: HashSet<&String> = falsy.iter().filter(|name| js::mentions_in(body, name) == 1).collect();
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
}

/// A child shown only where a test holds, `c ? <b /> : undefined`: `c &&
/// <b />`, as JSX writes it (ADR 0235), of a boolean test, whose `false`
/// renders nothing as `undefined` does; of another, its truth, `!!href &&
/// <a />`, as `""` or `0` would render.
fn shown_children(e: &mut Expr, booleans: &HashSet<String>) {
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
}

/// A handler of one call a component's given, `() => { f(); }`, held then
/// read in place: `() => f()`.
fn concise_handlers(e: &mut Expr, concise: &[js::Span]) {
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
}

/// `!` of what's read in place now, `!(t === "pre")` of `!_5`: as
/// `Expr::unary` makes it, `t !== "pre"`.
fn negations(e: &mut Expr) {
    if let ExprKind::Unary(op, arg) = &e.kind {
        *e = Expr {
            span: e.span,
            ..Expr::unary(*op, (**arg).clone())
        };
    }
}

/// A field of a `dyn` field's pair read in place now, `{ value: f.last,
/// impl: f.impl }.impl`: as `Expr::member` makes it.
fn pairs_fields(e: &mut Expr) {
    if let ExprKind::Member(object, field) = &e.kind
        && let ExprKind::Object(props) = &object.kind
        && js::is_read_pair(props)
    {
        *e = Expr {
            span: e.span,
            ..Expr::member((**object).clone(), field.clone())
        };
    }
}

/// For the source map: a `return` given the value it returns, at MIR's end
/// of the body, which the map has nothing of, where that value was written;
/// an `if`, where its test says, `if c` or its arm's pattern.
fn source_spans(stmts: &mut [Stmt]) {
    for stmt in stmts.iter_mut() {
        match &stmt.kind {
            StmtKind::Return(Some(value)) if stmt.span.is_none() => stmt.span = value.span,
            StmtKind::If(test, ..) if !test.span.is_none() => stmt.span = test.span,
            _ => {}
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
            StmtKind::Switch { cases, default, .. } => {
                js::case_lists_mut(cases, default).for_each(|body| bottom_up(body, f))
            }
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
    let writes = js::Writes::of(body);
    let mut set = writes.by_closures();
    set.extend(writes.handled);
    set
}

/// The variables something may change: each `let`'s, each assigned, and
/// each a handle's made on.
fn changing(body: &mut Vec<Stmt>) -> HashSet<String> {
    let writes = js::Writes::of(body);
    let mut changing = writes.replaced;
    changing.extend(writes.handled);
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

    use super::{
        decided_tests, inlined, is_temporary, paired_temporaries, settle_jumps, shared_tails, spread_spreads,
        temporary, turned, user_name,
    };
    use crate::js::{self, Expr, ExprKind, Op, Prop, Stmt, StmtKind, UnaryOp};

    fn call(f: &str) -> js::Stmt {
        StmtKind::Expr(Expr::call(Expr::var(f), Vec::new())).at(js::Span::NONE)
    }

    fn present(x: &str) -> Expr {
        Expr::bin(Op::LooseNe, Expr::var(x), Expr::null())
    }

    fn returns(x: &str) -> Stmt {
        StmtKind::Return(Some(Expr::var(x))).at(js::Span::NONE)
    }

    // `if (option != null) { return option; } .. if (option != null) { s }`,
    // a std body's drop of what its early return gave away: the second test
    // fails where it's reached, so it goes. Within `if (p != null) { .. }`,
    // `p != null` holds, so an `if` of it there is its `then`, and one of
    // `p.item != null` within `if (p.item == null)` is gone.
    #[test]
    fn a_test_an_earlier_one_decided_goes() {
        let mut after = vec![
            StmtKind::Const("option".into(), Expr::call(Expr::var("f"), Vec::new())).at(js::Span::NONE),
            StmtKind::If(present("option"), vec![returns("option")], None).at(js::Span::NONE),
            StmtKind::Const("x".into(), Expr::member(Expr::var("v"), "item")).at(js::Span::NONE),
            StmtKind::If(present("option"), vec![call("drop")], None).at(js::Span::NONE),
            returns("x"),
        ];
        decided_tests(&mut after, &HashSet::new());
        assert!(matches!(
            after.as_slice(),
            [
                Stmt {
                    kind: StmtKind::Const(..),
                    ..
                },
                Stmt {
                    kind: StmtKind::If(..),
                    ..
                },
                Stmt {
                    kind: StmtKind::Const(..),
                    ..
                },
                Stmt {
                    kind: StmtKind::Return(_),
                    ..
                },
            ]
        ));
        let some_value = |x: &str| Expr::member(Expr::var(x), "item");
        let absent = |e: Expr| Expr::bin(Op::LooseEq, e, Expr::null());
        let innermost = StmtKind::If(
            present("p"),
            vec![
                StmtKind::If(
                    Expr::bin(Op::LooseNe, some_value("p"), Expr::null()),
                    vec![call("g")],
                    None,
                )
                .at(js::Span::NONE),
            ],
            None,
        );
        let mut within = vec![
            StmtKind::If(
                present("p"),
                vec![
                    StmtKind::If(
                        absent(some_value("p")),
                        vec![
                            StmtKind::Assign(Expr::var("r"), Expr::int(0)).at(js::Span::NONE),
                            innermost.at(js::Span::NONE),
                        ],
                        None,
                    )
                    .at(js::Span::NONE),
                ],
                None,
            )
            .at(js::Span::NONE),
        ];
        let changing: HashSet<String> = ["r".to_string()].into();
        js::statement_lists(&mut within, &mut |stmts| decided_tests(stmts, &changing));
        let [
            Stmt {
                kind: StmtKind::If(_, then, None),
                ..
            },
        ] = within.as_slice()
        else {
            panic!("the outer if")
        };
        let [
            Stmt {
                kind: StmtKind::If(_, then, None),
                ..
            },
        ] = then.as_slice()
        else {
            panic!("the inner if")
        };
        assert!(matches!(
            then.as_slice(),
            [Stmt {
                kind: StmtKind::Assign(..),
                ..
            }]
        ));
    }

    // `let _5; let _6; _5 = a != null; _6 = !_5; if (!_6) { f(); }`, twice,
    // of a `bool` optimized MIR made and tested once each time: each is read
    // where it's made, `if (!!(a != null)) { f(); }`, as a `const` read once is.
    #[test]
    fn a_temporary_assigned_then_read_once_each_time_is_read_where_its_made() {
        let (t5, t6) = (temporary(5), temporary(6));
        let pair = |x: &str, f: &str| {
            vec![
                StmtKind::Assign(Expr::var(&t5), present(x)).at(js::Span::NONE),
                StmtKind::Assign(Expr::var(&t6), Expr::unary(UnaryOp::Not, Expr::var(&t5))).at(js::Span::NONE),
                StmtKind::If(Expr::unary(UnaryOp::Not, Expr::var(&t6)), vec![call(f)], None).at(js::Span::NONE),
            ]
        };
        let mut body = vec![
            StmtKind::Let(t5.clone(), None).at(js::Span::NONE),
            StmtKind::Let(t6.clone(), None).at(js::Span::NONE),
        ];
        body.extend(pair("a", "f"));
        body.extend(pair("b", "g"));
        let paired = paired_temporaries(&body);
        assert_eq!(paired, [t5.clone(), t6.clone()].into());
        let changing: HashSet<String> = [t5.clone(), t6.clone()].into();
        inlined(
            &mut body,
            &changing,
            &HashSet::new(),
            &HashSet::new(),
            &HashSet::new(),
            &paired,
        );
        let [
            _,
            _,
            Stmt {
                kind: StmtKind::If(test, ..),
                ..
            },
            Stmt {
                kind: StmtKind::If(..), ..
            },
        ] = body.as_slice()
        else {
            panic!("two tests, {}", body.len())
        };
        assert!(matches!(&test.kind, ExprKind::Unary(UnaryOp::Not, inner)
            if matches!(&inner.kind, ExprKind::Unary(UnaryOp::Not, e) if matches!(&e.kind, ExprKind::Binary(Op::LooseNe, ..)))));
        assert_eq!(js::mentions_in(&body, &t5) + js::mentions_in(&body, &t6), 0);
    }

    // `if (c) { a(); x = v; } else { b(); x = v; }`, one assignment MIR
    // wrote into both branches: `if (c) { a(); } else { b(); } x = v;`; one
    // that leaves, `return v;`, stays in each, the branches' shape; and `if
    // (c) { a(); return v; } return v;` is `if (c) { a(); } return v;`.
    #[test]
    fn a_tail_both_branches_end_with_is_written_once() {
        let branches = |then: Vec<Stmt>, otherwise: Option<Vec<Stmt>>| {
            StmtKind::If(Expr::var("c"), then, otherwise).at(js::Span::NONE)
        };
        let assigns = || StmtKind::Assign(Expr::var("x"), Expr::var("v")).at(js::Span::NONE);
        let mut either = vec![branches(vec![call("a"), assigns()], Some(vec![call("b"), assigns()]))];
        shared_tails(&mut either);
        let [
            Stmt {
                kind: StmtKind::If(_, then, Some(otherwise)),
                ..
            },
            Stmt {
                kind: StmtKind::Assign(..),
                ..
            },
        ] = either.as_slice()
        else {
            panic!("the branches, then the assignment")
        };
        assert_eq!((then.len(), otherwise.len()), (1, 1));
        let mut leaving = vec![branches(
            vec![call("a"), returns("v")],
            Some(vec![call("b"), returns("v")]),
        )];
        shared_tails(&mut leaving);
        assert!(
            matches!(leaving.as_slice(), [Stmt { kind: StmtKind::If(_, then, Some(otherwise)), .. }]
            if then.len() == 2 && otherwise.len() == 2)
        );
        let mut fallen = vec![branches(vec![call("a"), returns("v")], None), returns("v")];
        shared_tails(&mut fallen);
        let [
            Stmt {
                kind: StmtKind::If(_, then, None),
                ..
            },
            Stmt {
                kind: StmtKind::Return(_),
                ..
            },
        ] = fallen.as_slice()
        else {
            panic!("the branch, then the return")
        };
        assert_eq!(then.len(), 1);
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
