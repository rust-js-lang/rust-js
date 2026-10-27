//! A body lowered from its MIR, as a person writes it (ADR 0364): each
//! rewrite changes nothing that runs. MIR's one return block is a `return`
//! where each path ends, and what the structure needed to reach it, a
//! labeled block or a `break`, goes. A closure's body is tidied as its own.

use std::collections::{HashMap, HashSet};

use crate::js::{self, Expr, ExprKind, Op, Stmt, StmtKind, UnaryOp, breaks, ends, names_break};

use crate::lower::iterators::{self, eager_chains, eager_spreads, iterated};

mod bindings;
mod branches;
mod comparisons;
mod contracts;
mod copied;
mod decisions;
mod destructuring;
mod flags;
mod jumps;
mod logicals;
mod loops;
mod moves;
mod options;
mod shadows;
mod stages;
mod strict_switches;
mod switches;
mod unwinds;
mod values;

pub(super) use self::bindings::{HELD_ARG, is_temporary, temporary, unsuffixed_params, user_name};
pub(super) use self::destructuring::{copied_params, destructured_params};
pub(super) use self::values::{call_path, field_path, writer_body};
use self::{
    bindings::*, branches::*, comparisons::*, copied::*, destructuring::*, jumps::*, logicals::*, loops::*, moves::*,
    options::*, switches::switched, values::*,
};
use super::hints::{LabeledBlock, Switch};

/// What a body's last passes ask of its locals, by name: those the program
/// names, and its `String`s.
pub(super) struct Named {
    pub(super) user: HashSet<String>,
    pub(super) text: HashSet<String>,
}

/// What a body's `let`s bind, each by its name.
pub(super) struct LetNames {
    /// What a struct or tuple pattern binds, `let Point { x, y } = p;`.
    pub(super) patterns: HashSet<String>,
    /// What a name alone binds, `let x = p.x;`.
    pub(super) named: HashSet<String>,
    /// Of those, a variable given another type, `let lazy: &LazyComponent
    /// = unsafe { js::cast(children) };`.
    pub(super) casts: HashSet<String>,
    /// Of those, one an `if let` takes apart, the variable alone its
    /// scrutinee, `let types = t.types.borrow().clone(); if let Some(types)
    /// = types`: a `let` that ends a borrow, which reads as its value
    /// written there where only that test reads it (ADR 0866).
    pub(super) taken_apart: HashSet<String>,
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
    /// What the body's `let`s bind, by name.
    pub(super) lets: &'h LetNames,
    /// Whether the body is a closure's, tidied again in its function's.
    pub(super) closure: bool,
    pub(super) returns: &'h [js::Span],
    /// The `const`s MIR's lowering made of what an element it made ahead
    /// of a branch reads (`capture_jsx`): as its temporaries, read where
    /// they're used where nothing between tells.
    pub(super) captured: &'h HashSet<String>,
    /// The variables a `clone()` made (`hints.rs::clone_names`).
    pub(super) clones: &'h HashSet<String>,
    /// Of a closure, the variables it captured by value, which its maker
    /// never assigns again from where it makes it (ADR 0879).
    pub(super) kept: &'h HashSet<String>,
    /// The variables of Rust's `bool`: JS's `true` and `false` alone.
    pub(super) booleans: &'h HashSet<String>,
    /// The variables of Rust's integers: JS's numbers, which `++` steps.
    pub(super) numbers: &'h HashSet<String>,
    /// The variables of Rust's floats: JS's numbers, never an object whose
    /// `valueOf` a comparison runs (ADR 0966).
    pub(super) floats: &'h HashSet<String>,
    /// The variables of plain values, which nothing changes, and `...items`
    /// of arrays whose items nothing changes (`Locals::plain`).
    pub(super) plain: &'h HashSet<String>,
    /// Each closure of one call, where it's written and what it passes its
    /// call on to (ADR 0605, amended).
    pub(super) forwarding: &'h [crate::lower::passed_on::Forwarding],
    /// Where each closure of one call whose JS gives `undefined` is written:
    /// a component's handler of one, `() => f()` (ADR 0040).
    pub(super) concise: &'h [(js::Span, crate::lower::jsx::OneCall)],
    /// Each drop flag, `noisy$live`: whether a value is there to drop.
    pub(super) flags: &'h HashSet<String>,
    /// What `Iterator::fuse`'s compiled body is called here, `iterator_fuse`,
    /// where the module imports it: a `Fuse` stepped till it ends is its
    /// source (ADR 0561).
    pub(super) fuse: Option<&'h str>,
    /// The pass to stop after, `--tidy-until`'s, `stage/pass`.
    pub(super) until: Option<&'h str>,
    /// The function's parameters, which the `finish` stage takes apart.
    pub(super) params: &'h std::cell::RefCell<Vec<js::Pattern>>,
    /// The names MIR suggests of its temporaries, for `finish` to give.
    pub(super) suggested: &'h HashMap<String, Vec<String>>,
    /// The locals the program names, and its `String`s, by name.
    pub(super) named: &'h Named,
    /// Whether the stage running found its `again` stages must run once
    /// it's done (`stages::Stage::again`): set by its passes, taken by
    /// `tidy`.
    pub(super) again: &'h std::cell::Cell<bool>,
}

/// `body`, tidied, by what `hints` say of it: each stage's passes in turn
/// (`stages::STAGES`), up to `hints.until`'s where it names one, and in a
/// debug build, what each stage leaves checked (ADR 0659).
pub(super) fn tidy(body: &mut Vec<Stmt>, hints: &Hints) {
    for stage in stages::STAGES {
        if run_stage(body, hints, stage, stage.name) {
            return;
        }
        // Its stages run again where its passes found they must, each
        // through this runner, named `again/<stage>`.
        if !stage.again.is_empty() && hints.again.take() {
            for again in stages::STAGES.iter().filter(|s| stage.again.contains(&s.name)) {
                if run_stage(body, hints, again, &again_name(again)) {
                    return;
                }
            }
        }
    }
}

/// `stage`'s passes in turn, named under `name`, then in a debug build,
/// what it ensures, checked. Whether `hints.until` stopped it.
fn run_stage(body: &mut Vec<Stmt>, hints: &Hints, stage: &stages::Stage, name: &str) -> bool {
    for pass in stage.passes {
        (pass.run)(body, hints);
        if hints.until == Some(&*pass_name(name, pass)) {
            return true;
        }
    }
    if cfg!(debug_assertions) {
        for contract in contracts::ALWAYS.iter().chain(stage.ensures) {
            if let Some(why) = (contract.broken)(body) {
                panic!("tidy's `{name}` stage broke \"{}\": {why}", contract.name);
            }
        }
    }
    false
}

/// What a stage run again is named, `again/names`.
fn again_name(stage: &stages::Stage) -> String {
    format!("again/{}", stage.name)
}

/// What `--tidy-until` names a pass by: `stage/pass`.
fn pass_name(stage: &str, pass: &stages::Pass) -> String {
    format!("{stage}/{}", pass.name)
}

/// Each pass's name, in the order they run, a stage's run again after it
/// where it is: what `--tidy-until` may name.
pub(crate) fn pass_names() -> Vec<String> {
    let mut names = Vec::new();
    for stage in stages::STAGES {
        names.extend(stage.passes.iter().map(|pass| pass_name(stage.name, pass)));
        for again in stages::STAGES.iter().filter(|s| stage.again.contains(&s.name)) {
            names.extend(again.passes.iter().map(|pass| pass_name(&again_name(again), pass)));
        }
    }
    names
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
                    facts: js::Facts::NONE,
                    span: child.span,
                    kind: ExprKind::Binary(Op::And, Box::new(test), shown.clone()),
                };
            }
        }
    }
}

/// A closure of one call whose JS gives `undefined`, held then read in
/// place: a handler a component's given, `() => { f(); }` too (ADR 0040),
/// and any other the program wrote as its call's value, `|| f()`, as it
/// wrote it (ADR 0883): `() => f()`.
fn concise_arrows(e: &mut Expr, concise: &[(js::Span, crate::lower::jsx::OneCall)]) {
    let of = |value: &Expr| {
        concise
            .iter()
            .find(|(span, _)| *span == value.span)
            .map(|&(_, written)| written)
    };
    if matches!(e.kind, ExprKind::Arrow(..)) && of(e) == Some(crate::lower::jsx::OneCall::Value) {
        js::returning_its_call(e);
    }
    if let ExprKind::Jsx(jsx) = &mut e.kind {
        for prop in &mut jsx.props {
            if let js::Prop::Field(name, value) = prop
                && js::is_handler_name(name)
                && of(value).is_some()
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
        && let ExprKind::Object(props, ..) = &object.kind
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
/// as written. `b` runs where `t` doesn't hold either way. Not of an arm of
/// a `match` that's a `switch` (`switches`), whose cases are written as its
/// arms are, whatever order MIR tests them in: its chain stays as MIR tests
/// it, for the `switch` to read (ADR 0668).
fn written_first(stmts: &mut [Stmt], switches: &[Switch]) {
    for stmt in stmts {
        let StmtKind::Labeled(label, body) = &mut stmt.kind else {
            continue;
        };
        let [first, rest @ ..] = body.as_slice() else {
            continue;
        };
        let StmtKind::If(test, then, None) = &first.kind else {
            continue;
        };
        if switched(test, first.span, switches) {
            continue;
        }
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
            let (js::Prop::Field(_, v)
            | js::Prop::Getter(_, v)
            | js::Prop::Setter(_, v)
            | js::Prop::Method(_, v)
            | js::Prop::Spread(v)) = p;
            !matches!(p, js::Prop::Spread(_)) && inert_element(v, changing)
        })
    };
    match &e.kind {
        ExprKind::Var(v) => !changing.contains(v),
        ExprKind::Symbol(_) | ExprKind::Arrow(..) | ExprKind::AsyncArrow(..) => true,
        ExprKind::Array(items) => items.iter().all(|i| inert_element(i, changing)),
        ExprKind::Object(fields, ..) => props(fields),
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
        MoveFacts, decided_tests, inlined, is_temporary, paired_temporaries, settle_jumps, shared_tails,
        spread_spreads, temporary, turned, user_name,
    };
    use crate::js::{self, Expr, ExprKind, Op, Prop, Stmt, StmtKind, UnaryOp};

    fn call(f: &str) -> js::Stmt {
        StmtKind::Expr(Expr::call(Expr::var(f), Vec::new())).at(js::Span::NONE)
    }

    fn assign(x: &str, value: Expr) -> Stmt {
        StmtKind::Assign(Expr::var(x), value).at(js::Span::NONE)
    }

    /// Two arms that assign one variable are one assignment of a
    /// conditional; each reason they aren't, beside the case it refuses
    /// (`branches::one_assignment`).
    #[test]
    fn one_assignment_of_two_arms() {
        use super::branches::one_assignment;
        let none = js::Span::NONE;
        let one = |then: Vec<Stmt>, otherwise: Vec<Stmt>, test: &Expr| {
            one_assignment(test, (&then, &otherwise), none, ("_0", &[])).map(|one| one.target)
        };
        let c = Expr::var("c");
        // Accepted: one variable, each arm a value.
        assert_eq!(
            one(vec![assign("x", Expr::num(1))], vec![assign("x", Expr::num(2))], &c),
            Some("x".into())
        );
        // A test that does something is evaluated once either way.
        let called = Expr::call(Expr::var("ready"), Vec::new());
        assert!(
            one(
                vec![assign("x", Expr::num(1))],
                vec![assign("x", Expr::num(2))],
                &called
            )
            .is_some()
        );
        // Refused: two variables.
        assert_eq!(
            one(vec![assign("x", Expr::num(1))], vec![assign("y", Expr::num(2))], &c),
            None
        );
        // Refused: an arm that does more than assign.
        assert_eq!(
            one(
                vec![call("f"), assign("x", Expr::num(1))],
                vec![assign("x", Expr::num(2))],
                &c
            ),
            None
        );
        // Refused: each adding to it, as written, `x = x + 1`.
        let add = |n| Expr::bin(Op::Add, Expr::var("x"), Expr::num(n));
        assert_eq!(one(vec![assign("x", add(1))], vec![assign("x", add(2))], &c), None);
        // Refused: an arm that's a conditional, of more branches, but of a temporary.
        let cond = Expr::cond(Expr::var("d"), Expr::num(1), Expr::num(2));
        assert_eq!(
            one(vec![assign("x", cond.clone())], vec![assign("x", Expr::num(3))], &c),
            None
        );
        assert!(one(vec![assign("_4", cond)], vec![assign("_4", Expr::num(3))], &c).is_some());
        // Refused: the function's result where an arm gives nothing.
        assert_eq!(
            one(
                vec![assign("_0", Expr::num(1))],
                vec![assign("_0", Expr::undefined())],
                &c
            ),
            None
        );
        // Refused: a `switch`'s arms, where the test is one's pattern (ADR 0629).
        let at = js::Span { lo: 10, hi: 20 };
        let switches = [super::super::hints::Switch {
            span: js::Span { lo: 0, hi: 100 },
            arms: vec![super::super::hints::SwitchArm {
                span: at,
                pattern: at,
                guard: None,
                constants: Vec::new(),
                falls: false,
            }],
        }];
        let then = [assign("x", Expr::num(1))];
        let otherwise = [assign("x", Expr::num(2))];
        assert!(one_assignment(&c, (&then, &otherwise), at, ("_0", &switches)).is_none());
        assert!(one_assignment(&c, (&then, &otherwise), none, ("_0", &switches)).is_some());
    }

    /// `if (!c) { return 1; } a(); return 2;` of `if c { a(); return 2; }
    /// 1`, its rest written in the `then`, is the `if` written; not where
    /// the rest goes on past the list, which `exit` would then run after,
    /// nor where it isn't the `then`'s (`jumps::tails::written_thens`).
    #[test]
    fn written_thens_turn_only_a_rest_that_leaves() {
        use super::jumps::written_thens;
        let at = |lo, hi| js::Span { lo, hi };
        let spanned = |mut s: Stmt, lo| {
            s.span = at(lo, lo + 1);
            s
        };
        let guard = || {
            StmtKind::If(
                Expr::unary(UnaryOp::Not, Expr::var("c")),
                vec![StmtKind::Return(Some(Expr::num(1))).at(at(90, 91))],
                None,
            )
            .at(at(1, 2))
        };
        let thens = [(at(0, 3), at(4, 50))];
        let rest = |leaves: bool| {
            let mut rest = vec![spanned(call("a"), 10)];
            if leaves {
                rest.push(StmtKind::Return(Some(Expr::num(2))).at(at(20, 21)));
            }
            rest
        };
        let turned = |stmts: &[Stmt]| matches!(&stmts[0].kind, StmtKind::If(test, then, None) if matches!(test.kind, ExprKind::Var(_)) && then.len() == 2);
        let mut leaves = [vec![guard()], rest(true)].concat();
        written_thens(&mut leaves, &thens);
        assert!(turned(&leaves) && leaves.len() == 2);
        let mut goes_on = [vec![guard()], rest(false)].concat();
        written_thens(&mut goes_on, &thens);
        assert!(matches!(&goes_on[0].kind, StmtKind::If(test, ..) if matches!(test.kind, ExprKind::Unary(..))));
        let mut elsewhere = [vec![guard()], rest(true)].concat();
        written_thens(&mut elsewhere, &[(at(0, 3), at(60, 80))]);
        assert!(matches!(&elsewhere[0].kind, StmtKind::If(test, ..) if matches!(test.kind, ExprKind::Unary(..))));
    }

    /// What finishes a body, after the stages that tidy it, in the order it
    /// ran when it was a function of its own, `named`: the last passes
    /// `--tidy-until` may stop after, and the contracts checked after them
    /// (ADR 0659, amended).
    #[test]
    fn finish_is_the_last_stage_in_its_order() {
        let names = super::pass_names();
        let after: Vec<&str> = names
            .iter()
            .skip_while(|n| !n.starts_with("scopes/"))
            .map(|n| &**n)
            .collect();
        let stages: Vec<&str> = after.iter().map(|n| &n[..n.rfind('/').expect("stage/pass")]).collect();
        let mut stages_in_order = stages.clone();
        stages_in_order.dedup();
        // The body's scopes made, the stages that read temporaries in place
        // again, then its names given.
        assert_eq!(
            stages_in_order,
            ["scopes", "again/names", "again/in place", "again/chains", "finish"]
        );
        assert_eq!(
            after
                .iter()
                .filter_map(|n| n.strip_prefix("finish/"))
                .collect::<Vec<_>>(),
            [
                "temporaries named",
                "flags named",
                "shadows",
                "given last",
                "filled lazily",
                "appends",
                "nulled as they are"
            ]
        );
        assert!(names.last().is_some_and(|n| n == "finish/nulled as they are"));
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

    /// Closures nested 64 deep, the innermost setting `x`: each closure's
    /// body asked once, not once for each closure around it.
    #[test]
    fn rebinds_walks_nested_closures_once() {
        let mut body = vec![StmtKind::Assign(Expr::var("x"), Expr::int(1)).at(js::Span::NONE)];
        for _ in 0..64 {
            body = vec![StmtKind::Expr(Expr::arrow(Vec::new(), body)).at(js::Span::NONE)];
        }
        assert!(super::bindings::rebinds(&body, "x"));
        assert!(!super::bindings::rebinds(&body, "y"));
    }
    #[test]
    fn a_test_an_earlier_one_decided_goes() {
        let mut after = vec![
            StmtKind::Const("option".into(), Expr::call(Expr::var("f"), Vec::new())).at(js::Span::NONE),
            StmtKind::If(present("option"), vec![returns("option")], None).at(js::Span::NONE),
            StmtKind::Const("x".into(), Expr::member(Expr::var("v"), "item")).at(js::Span::NONE),
            StmtKind::If(present("option"), vec![call("drop")], None).at(js::Span::NONE),
            returns("x"),
        ];
        decided_tests(&mut after, &HashSet::new(), &[]);
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
        js::statement_lists(&mut within, &mut |stmts| decided_tests(stmts, &changing, &[]));
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
        let none = HashSet::new();
        let facts = MoveFacts {
            changing: &changing,
            set: &none,
            plain: &none,
            captured: &none,
            paired: &paired,
        };
        inlined(&mut body, &facts);
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
        shared_tails(&mut either, &[], &[]);
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
        shared_tails(&mut leaving, &[], &[]);
        assert!(
            matches!(leaving.as_slice(), [Stmt { kind: StmtKind::If(_, then, Some(otherwise)), .. }]
            if then.len() == 2 && otherwise.len() == 2)
        );
        let mut fallen = vec![branches(vec![call("a"), returns("v")], None), returns("v")];
        shared_tails(&mut fallen, &[], &[]);
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

    /// What an `if` always runs is put in its place (`jumps::never_taken`):
    /// of `if (false)`, its `else`; of an `if` of nothing either way, its
    /// test where the test does anything, else nothing; and with inner
    /// lists first, an `if` that what's taken inside it empties goes too.
    #[test]
    fn what_an_if_always_runs() {
        use super::jumps::never_taken;
        let none = js::Span::NONE;
        let if_ = |test: Expr, then: Vec<Stmt>, other: Option<Vec<Stmt>>| StmtKind::If(test, then, other).at(none);
        let mut body = vec![if_(Expr::bool(false), vec![call("a")], Some(vec![call("b")]))];
        never_taken(&mut body);
        assert!(body == vec![call("b")]);
        let mut body = vec![if_(Expr::var("live"), vec![], None), call("c")];
        never_taken(&mut body);
        assert!(body == vec![call("c")]);
        let ready = Expr::call(Expr::var("ready"), Vec::new());
        let mut body = vec![if_(ready.clone(), vec![], Some(vec![]))];
        never_taken(&mut body);
        assert!(body == vec![StmtKind::Expr(ready).at(none)]);
        let mut body = vec![if_(
            Expr::bool(false),
            vec![call("d")],
            Some(vec![if_(Expr::var("live"), vec![], None)]),
        )];
        never_taken(&mut body);
        assert!(body.is_empty());
        let mut body = vec![if_(
            Expr::var("live"),
            vec![if_(Expr::bool(false), vec![call("d")], None)],
            None,
        )];
        super::bottom_up(&mut body, &mut never_taken);
        assert!(body.is_empty());
    }
}
