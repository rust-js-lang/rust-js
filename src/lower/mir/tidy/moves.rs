//! Tidy's moves: a temporary's value read where it's read, not where it's
//! made, where `moved` says that keeps what the program does, and why
//! (the fourth review: the decision was interleaved with the rewrite).

use super::*;

/// What `inlined` asks of the body it moves values in, each fact its own,
/// asked of the body as it is (ADR 0670).
pub(super) struct MoveFacts<'a> {
    /// The variables something in the body changes.
    pub changing: &'a HashSet<String>,
    /// The variables a closure sets: what a call can change of them.
    pub set: &'a HashSet<String>,
    /// The variables whose value is plain data, whose fields no call
    /// changes.
    pub plain: &'a HashSet<String>,
    /// The elements made ahead of a branch, which may be read where it is.
    pub captured: &'a HashSet<String>,
    /// The temporaries each of whose values is read once, by the statement
    /// after it (`paired_temporaries`).
    pub paired: &'a HashSet<String>,
}

/// A temporary MIR made, `const _5 = e;`, read once, by the statement
/// after it, or after those that are moved there too: `e` where it's read,
/// where `moved` says what the program does stays the same.
pub(super) fn inlined(stmts: &mut Vec<Stmt>, facts: &MoveFacts) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        let Some((Moved { at, read, replaced }, _)) = moved(&stmts[i], &stmts[i + 1..], facts) else {
            i += 1;
            continue;
        };
        *read.of_mut(&mut stmts[i + 1 + at]) = replaced;
        stmts.remove(i);
        // The one before it may now be read by the statement after it too.
        i = i.saturating_sub(1);
    }
}

/// Why a temporary's value may be read where it's read instead of where
/// it's made: what keeps what the program does the same.
#[derive(Debug, PartialEq)]
enum Why {
    /// It does nothing, and nothing it reads changes, so it gives the same
    /// wherever it's read.
    Stable,
    /// Where it's read, all JS evaluates before it there passes it
    /// (`leads`): what moving it past can't tell from where it was.
    ReadFirst,
}

/// Which of a statement's expressions reads the temporary.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Read {
    /// Its value: a `const`'s, a `return`'s, an assignment's right side.
    Value,
    /// An assignment's target whose object it is, `_25[k] = v`.
    Target,
    /// An `if`'s test.
    Test,
}

impl Read {
    /// Of `stmt`, where `read_of` found it: the expression.
    fn of(self, stmt: &Stmt) -> &Expr {
        match (self, &stmt.kind) {
            (
                Read::Value,
                StmtKind::Const(_, e)
                | StmtKind::Let(_, Some(e))
                | StmtKind::Expr(e)
                | StmtKind::Return(Some(e))
                | StmtKind::Throw(e)
                | StmtKind::Destructure { value: e, .. }
                | StmtKind::Assign(_, e),
            )
            | (Read::Target, StmtKind::Assign(e, _))
            | (Read::Test, StmtKind::If(e, ..)) => e,
            _ => unreachable!("a statement `read_of` found it read in"),
        }
    }

    /// `of`, to replace.
    fn of_mut(self, stmt: &mut Stmt) -> &mut Expr {
        match (self, &mut stmt.kind) {
            (
                Read::Value,
                StmtKind::Const(_, e)
                | StmtKind::Let(_, Some(e))
                | StmtKind::Expr(e)
                | StmtKind::Return(Some(e))
                | StmtKind::Throw(e)
                | StmtKind::Destructure { value: e, .. }
                | StmtKind::Assign(_, e),
            )
            | (Read::Target, StmtKind::Assign(e, _))
            | (Read::Test, StmtKind::If(e, ..)) => e,
            _ => unreachable!("a statement `read_of` found it read in"),
        }
    }
}

/// A temporary's value moved to where it's read: `at` statements past it,
/// in its expression `read`, which becomes `replaced`.
struct Moved {
    at: usize,
    read: Read,
    replaced: Expr,
}

/// Where the temporary `stmt` makes, `const _5 = e;`, or a paired one
/// assigns, may be read in its place, of `rest`, the statements after it,
/// and why: of the tree as it is, which this doesn't change.
fn moved(stmt: &Stmt, rest: &[Stmt], facts: &MoveFacts) -> Option<(Moved, Why)> {
    let (name, value, assigned) = match &stmt.kind {
        StmtKind::Const(name, value) => (name, value, false),
        // Or an assignment of a temporary each of whose is read once, by
        // the statement after it (`paired_temporaries`).
        StmtKind::Assign(target, value)
            if let ExprKind::Var(name) = &target.kind
                && facts.paired.contains(name) =>
        {
            (name, value, true)
        }
        _ => return None,
    };
    // A temporary MIR made, or an element made ahead of a branch: what the
    // program didn't name.
    let element = facts.captured.contains(name);
    if !is_temporary(name) && !element {
        return None;
    }
    // What may run before it's read: what does nothing, where it does
    // nothing either; else only constants and variables nothing changes,
    // which what it does can't change. Of what an element made ahead of a
    // branch reads, what does nothing and reads no such variable: the
    // element's tree is the order the program makes them in. What does
    // nothing and reads what no call changes, a `let` no closure sets,
    // passes what does something too.
    let acts = value.has_effects();
    let unchanged_by_calls = !acts && read_past_calls(value, facts.set, facts.plain);
    let passes = |e: &Expr| match acts {
        false => unchanged_by_calls || !e.has_effects(),
        true if element => {
            let mut unchanged = !e.has_effects();
            e.visit_vars(&mut |v| unchanged &= !facts.set.contains(v));
            unchanged
        }
        true => read_past_calls(e, facts.set, facts.plain),
    };
    // The statement that reads it, past `const`s of what it may be read
    // past, `const list$1 = list;` a callback's borrow; and what does
    // nothing and reads what no call changes, past any statement that
    // assigns nothing it reads, a loop's too.
    let at = (rest.iter()).position(|s| js::mentions_in(std::slice::from_ref(s), name) > 0)?;
    let mut reads = Vec::new();
    value.visit_vars(&mut |v| reads.push(v.to_string()));
    let past = (rest[..at].iter()).all(|s| matches!(&s.kind, StmtKind::Const(_, v) if passes(v)))
        || (unchanged_by_calls && reads.iter().all(|v| !assigns(&rest[..at], v)));
    if !past {
        return None;
    }
    // Read once in what follows, and once there; of a paired temporary,
    // assigned again after, once by the statement after it.
    let once = match assigned {
        true => js::mentions_in(&rest[..=at], name) == 1,
        false => js::mentions_in(rest, name) == 1,
    };
    if !once || js::mentions_in(&rest[at..=at], name) != 1 {
        return None;
    }
    let read = read_of(&rest[at], name, value)?;
    let expr = read.of(&rest[at]);
    // Not an arrow into a call of it it'd be left called where it's made
    // in, `((l) => { .. })(take)`: it's held, `const map = (l) => ..;`, as
    // where no guard's `try` is (ADR 0705).
    if let ExprKind::Arrow(..) = value.kind {
        let mut left_called = false;
        expr.each(&mut |e| {
            if let ExprKind::Call(callee, args) = &e.kind
                && matches!(&callee.kind, ExprKind::Var(v) if v == name)
            {
                left_called |= applied_in_place(value, args).is_none();
            }
        });
        if left_called {
            return None;
        }
    }
    let mut stable = !value.has_effects();
    value.visit_vars(&mut |v| stable &= !facts.changing.contains(v));
    // One made where a test holds, `t ? e : undefined`, may be read where
    // that test holds.
    let guard = match &value.kind {
        ExprKind::Cond(test, _, no) if matches!(no.kind, ExprKind::Undefined) => Some(&**test),
        _ => None,
    };
    let why = match stable {
        true => Why::Stable,
        false if leads(expr, name, guard, &passes) == Some(true) => Why::ReadFirst,
        false => return None,
    };
    let replaced = expr.substitute(&|v| (v == name).then(|| value.clone()))?;
    Some((Moved { at, read, replaced }, why))
}

/// Which of `stmt`'s expressions reads `name`, a temporary of `value`, where
/// it may be moved: its value, an `if`'s test, or an assignment's target
/// whose object it is, `_25[k] = v`, where `value` does nothing, as JS
/// reads a target's object first, as it was read.
fn read_of(stmt: &Stmt, name: &str, value: &Expr) -> Option<Read> {
    match &stmt.kind {
        StmtKind::Const(..)
        | StmtKind::Let(_, Some(_))
        | StmtKind::Expr(_)
        | StmtKind::Return(Some(_))
        | StmtKind::Throw(_)
        | StmtKind::Destructure { .. } => Some(Read::Value),
        StmtKind::Assign(target, _) if !target.has_effects() => {
            let mut in_target = false;
            target.visit_vars(&mut |v| in_target |= v == name);
            let mut root = target;
            while let ExprKind::Index(of, _) | ExprKind::Member(of, _) = &root.kind {
                root = of;
            }
            let set_of = matches!(&root.kind, ExprKind::Var(v) if v == name) && !value.has_effects();
            match (in_target, set_of) {
                (false, _) => Some(Read::Value),
                (true, true) => Some(Read::Target),
                (true, false) => None,
            }
        }
        StmtKind::If(test, ..) => {
            let mut in_test = false;
            test.visit_vars(&mut |v| in_test |= v == name);
            in_test.then_some(Read::Test)
        }
        _ => None,
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
            let (js::Prop::Field(_, v) | js::Prop::Getter(_, v) | js::Prop::Setter(_, v) | js::Prop::Spread(v)) = p;
            v
        })),
        ExprKind::Jsx(jsx) => {
            let tag = match &jsx.tag {
                js::JsxTag::Component(tag) => Some(tag),
                _ => None,
            };
            let props = jsx.props.iter().map(|p| {
                let (js::Prop::Field(_, v) | js::Prop::Getter(_, v) | js::Prop::Setter(_, v) | js::Prop::Spread(v)) = p;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn stmt(kind: StmtKind) -> Stmt {
        kind.at(js::Span::NONE)
    }

    fn call(f: &str, args: Vec<Expr>) -> Expr {
        Expr::call(Expr::var(f), args)
    }

    /// What `moved` says of `body`'s first statement, a temporary, given
    /// nothing in the body changed or set.
    fn decided(body: &[Stmt]) -> Option<Why> {
        let none = HashSet::new();
        let facts = MoveFacts {
            changing: &none,
            set: &none,
            plain: &none,
            captured: &none,
            paired: &none,
        };
        moved(&body[0], &body[1..], &facts).map(|(_, why)| why)
    }

    /// `const _1 = object.field; change(object); consume(_1);`: reading
    /// it after `change(object)` may read another value, though reading it
    /// does nothing; it stays.
    #[test]
    fn a_read_stays_before_a_call_that_may_change_it() {
        let read = Expr::member(Expr::var("object"), "field");
        let body = [
            stmt(StmtKind::Const("_1".into(), read)),
            stmt(StmtKind::Expr(call("change", vec![Expr::var("object")]))),
            stmt(StmtKind::Expr(call("consume", vec![Expr::var("_1")]))),
        ];
        assert_eq!(decided(&body), None);
    }

    /// `const _1 = a + 1; consume(_1);`: what does nothing, of what nothing
    /// changes, gives the same wherever it's read.
    #[test]
    fn what_does_nothing_of_what_nothing_changes_is_stable() {
        let sum = Expr::bin(Op::Add, Expr::var("a"), Expr::int(1));
        let body = [
            stmt(StmtKind::Const("_1".into(), sum)),
            stmt(StmtKind::Expr(call("consume", vec![Expr::var("_1")]))),
        ];
        assert_eq!(decided(&body), Some(Why::Stable));
    }

    /// `const _1 = f(); g(_1);`: `g`, all JS evaluates before it, passes
    /// it, so `g(f())` calls `f` where it was called.
    #[test]
    fn a_call_read_first_where_its_read_moves() {
        let body = [
            stmt(StmtKind::Const("_1".into(), call("f", Vec::new()))),
            stmt(StmtKind::Expr(call("g", vec![Expr::var("_1")]))),
        ];
        assert_eq!(decided(&body), Some(Why::ReadFirst));
    }

    /// `const _1 = f(); g(h(), _1);`: `g(h(), f())` would call `h` first;
    /// it stays.
    #[test]
    fn a_call_after_another_where_its_read_stays() {
        let body = [
            stmt(StmtKind::Const("_1".into(), call("f", Vec::new()))),
            stmt(StmtKind::Expr(call("g", vec![call("h", Vec::new()), Expr::var("_1")]))),
        ];
        assert_eq!(decided(&body), None);
    }

    /// `const _1 = f(); h(); g(_1);`: a statement between that does
    /// something; it stays.
    #[test]
    fn a_call_past_a_statement_that_acts_stays() {
        let body = [
            stmt(StmtKind::Const("_1".into(), call("f", Vec::new()))),
            stmt(StmtKind::Expr(call("h", Vec::new()))),
            stmt(StmtKind::Expr(call("g", vec![Expr::var("_1")]))),
        ];
        assert_eq!(decided(&body), None);
    }

    /// `const name = f(); g(name);`: a name the program wrote isn't moved.
    #[test]
    fn a_name_the_program_wrote_stays() {
        let body = [
            stmt(StmtKind::Const("name".into(), call("f", Vec::new()))),
            stmt(StmtKind::Expr(call("g", vec![Expr::var("name")]))),
        ];
        assert_eq!(decided(&body), None);
    }
}
