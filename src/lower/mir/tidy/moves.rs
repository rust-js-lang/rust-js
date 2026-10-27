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
        let (made, rest) = stmts.split_at_mut(i + 1);
        if !entered(&made[i], rest, facts) {
            i += 1;
            continue;
        }
        stmts.remove(i);
        // The one before it may now be read by the statement after it too.
        i = i.saturating_sub(1);
    }
}

/// `stmt`'s temporary read in its place in `rest`, where `moved` says it
/// may be; or in a block that `rest` starts with and reads it in alone,
/// `const _183 = f(); bb200: { if (_183) .. }`, which entering runs
/// nothing before (ADR 0750). Whether it was.
fn entered(stmt: &Stmt, rest: &mut [Stmt], facts: &MoveFacts) -> bool {
    if let Some((Moved { at, read, replaced }, _)) = moved(stmt, rest, facts) {
        *read.of_mut(&mut rest[at]) = replaced;
        return true;
    }
    let (StmtKind::Const(name, _), [first, after @ ..]) = (&stmt.kind, rest) else {
        return false;
    };
    match &mut first.kind {
        StmtKind::Labeled(_, body) if js::mentions_in(after, name) == 0 => entered(stmt, body, facts),
        _ => false,
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
    let read = read_of(&rest[at], name)?;
    let expr = read.of(&rest[at]);
    let single = once && js::mentions_in(&rest[at..=at], name) == 1;
    // Or tested more than once there and nowhere else, an or-pattern's
    // tests of a field (ADR 0780).
    if !single && (assigned || !tested_in_place(value, expr, name, js::mentions_in(rest, name))) {
        return None;
    }
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
    let mut replaced = expr.substitute(&|v| (v == name).then(|| value.clone()))?;
    // A cell made in its place and read, `{ value: x }.value`, a struct's
    // field made of one: what it holds, as `Expr::member` reads one where
    // it's made, and its key then left out where that's `undefined` (ADR
    // 0810, amended). Not an assignment's target, which is no read.
    if read != Read::Target {
        replaced.each_mut(&mut |e| {
            if let ExprKind::Member(object, field) = &e.kind
                && let ExprKind::Object(..) = object.kind
            {
                let read = Expr::member((**object).clone(), field.clone());
                if !matches!(read.kind, ExprKind::Member(..)) {
                    *e = read.or_at(e.span);
                }
            }
        });
    }
    Some((Moved { at, read, replaced }, why))
}

/// Whether `value`, a temporary's, `const _5 = payload._status;`, is read
/// in each of `expr`'s tests of it, all `reads` of it there are, as an
/// or-pattern's, `payload._status === 0 || payload._status === -1` (ADR
/// 0780): a place's read, a variable's property no getter may answer, which
/// reads the same where nothing between the tests does anything, and each
/// read a side of a `===` or `!==`.
fn tested_in_place(value: &Expr, expr: &Expr, name: &str, reads: usize) -> bool {
    let mut place = value;
    while let ExprKind::Member(of, _) = &place.kind {
        if place.facts.has(js::Known::Getter) {
            return false;
        }
        place = of;
    }
    let of_a_variable = matches!(&place.kind, ExprKind::Var(v) if v != name) || matches!(place.kind, ExprKind::This);
    if !matches!(value.kind, ExprKind::Member(..)) || !of_a_variable || expr.has_effects() {
        return false;
    }
    let mut compared = 0;
    expr.each(&mut |e| {
        if let ExprKind::Binary(Op::Eq | Op::Ne, a, b) = &e.kind {
            compared += [a, b]
                .into_iter()
                .filter(|side| matches!(&side.kind, ExprKind::Var(v) if v == name))
                .count();
        }
    });
    compared == reads
}

/// Which of `stmt`'s expressions reads `name`, a temporary, where
/// it may be moved: its value, an `if`'s test, or an assignment's target
/// whose object it is, `_25[k] = v`: JS reads a target's object first,
/// then its key, then the value, so the temporary runs where it ran, whatever it
/// does, `window.document.title = t` (#188).
fn read_of(stmt: &Stmt, name: &str) -> Option<Read> {
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
            let set_of = matches!(&root.kind, ExprKind::Var(v) if v == name);
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
        // A binding's method of a JS value, `obj.send(_5, ..)`: looked up
        // before its arguments run, by a getter it may be, where Rust runs
        // them first: nothing moves past its receiver (ADR 0750).
        ExprKind::Call(f, _)
            if let ExprKind::Member(receiver, _) = &f.kind
                && f.facts.has(js::Known::Getter) =>
        {
            leads(receiver, name, guard, passes).or(Some(false))
        }
        // Another method's call, `out.push(_20, ..)`: its receiver, then its
        // arguments. What JS looks up of the receiver is the method its
        // type has, which nothing the program does changes; a call MIR
        // makes is written so, its receiver read first (ADR 0750).
        ExprKind::Call(f, args) if let ExprKind::Member(receiver, _) = &f.kind => {
            in_order(&mut std::iter::once(&**receiver).chain(args))
        }
        ExprKind::Call(f, args) | ExprKind::New(f, args) => in_order(&mut std::iter::once(&**f).chain(args)),
        ExprKind::Object(props, ..) => in_order(&mut props.iter().map(|p| {
            let (js::Prop::Field(_, v)
            | js::Prop::Getter(_, v)
            | js::Prop::Setter(_, v)
            | js::Prop::Method(_, v)
            | js::Prop::Spread(v)) = p;
            v
        })),
        ExprKind::Jsx(jsx) => {
            let tag = match &jsx.tag {
                js::JsxTag::Component(tag) => Some(tag),
                _ => None,
            };
            let props = jsx.props.iter().map(|p| {
                let (js::Prop::Field(_, v)
                | js::Prop::Getter(_, v)
                | js::Prop::Setter(_, v)
                | js::Prop::Method(_, v)
                | js::Prop::Spread(v)) = p;
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

/// `const types = prev.types; if (dev && types != null && types !== other)
/// { .. }`: a `let` an `if let` takes apart, `taken_apart`'s, of a place's
/// read that only the test of the `if` right after it reads, `prev.types`
/// where the test reads `types`, each time (ADR 0866): as the program would
/// write the place there, `if let Some(types) = &prev.types`, but for a
/// `RefCell`'s borrow, which the `let` ends once its value's read,
/// `prev.types.borrow().clone()`, the object (ADR 0863), and which JS
/// doesn't have. What's read where `tested_read` says it may be.
pub(super) fn tested_reads(stmts: &mut Vec<Stmt>, taken_apart: &HashSet<String>) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        let Some((test, then)) = tested_read(&stmts[i], &stmts[i + 1..], taken_apart) else {
            i += 1;
            continue;
        };
        stmts.remove(i);
        if let StmtKind::If(tested, branch, _) = &mut stmts[i].kind {
            *tested = test;
            if let Some((read, first)) = then {
                *read.of_mut(&mut branch[0]) = first;
            }
        }
        // A `let` before it may now be read by the `if` too.
        i = i.saturating_sub(1);
    }
}

/// Of `stmt`, `const x = p.f;`, and `rest`, the statements after it, the
/// test of the `if` `rest` starts with, `x` read there as `p.f`: where `p.f`
/// reads a variable's property, which no getter may answer and nothing
/// that runs before each read changes, as the test does nothing; and
/// nothing else reads `x`, its branches nor what follows it. Or, where `x`
/// is the property's own name, `const autoName = instance.autoName`, its
/// `then`'s first statement too, where what it reads is `x` alone, `return
/// autoName`, nothing run before it: `if (instance.autoName != null) {
/// return instance.autoName; }`, as React's getViewTransitionName writes it
/// (ADR 0866, amended, the user's decision, 2026-10-27). That statement's
/// read too, where it reads `x` so.
fn tested_read(stmt: &Stmt, rest: &[Stmt], taken_apart: &HashSet<String>) -> Option<(Expr, Option<(Read, Expr)>)> {
    let (StmtKind::Const(name, value), Some(StmtKind::If(test, then, _))) = (&stmt.kind, rest.first().map(|s| &s.kind))
    else {
        return None;
    };
    // The property's own name, `const autoName = instance.autoName`.
    let own_name = matches!(&value.kind, ExprKind::Member(_, property) if property == name);
    if !taken_apart.contains(name) && !own_name {
        return None;
    }
    let place = place_read(value, name);
    let mut tested = 0;
    test.visit_vars(&mut |v| tested += usize::from(v == name));
    if !place || tested == 0 || test.has_effects() {
        return None;
    }
    let substituted = |e: &Expr| e.substitute(&|v| (v == name).then(|| value.clone()));
    // Of the property's own name, read once more by the `then`'s first
    // statement, nothing it evaluates first doing anything.
    let then = match js::mentions_in(rest, name) - tested {
        0 if taken_apart.contains(name) => None,
        1 if own_name
            && let Some(first) = then.first()
            && js::mentions_in(std::slice::from_ref(first), name) == 1
            && let Some(read) = read_of(first, name)
            && matches!(&read.of(first).kind, ExprKind::Var(v) if v == name) =>
        {
            Some((read, substituted(read.of(first))?))
        }
        _ => return None,
    };
    Some((substituted(test)?, then))
}

/// `const child = current.child; workInProgress.child = child;`: a `let`
/// that ends a borrow, `let child = current.child.borrow().clone();`
/// (`LetNames::ends_borrow`), of a place's read, which no getter may answer,
/// read once, by the statement right after it, where nothing JS evaluates
/// before it there does anything (`read_again`): the place read there,
/// `workInProgress.child = current.child;`, as the program would write it
/// but for the borrow, which JS doesn't have (ADR 0866, amended, the user's
/// decision, 2026-10-27). Nothing runs between the two reads, so the place
/// holds what the `let` read. Only of a name the field's own, `child` of
/// `current.child`, which says nothing the field doesn't: one named
/// otherwise, `const pendingFirst = pendingQueue.next;`, is the program's,
/// as upstream's is (ADR 0866, amended again).
pub(super) fn ended_borrows(stmts: &mut Vec<Stmt>, ends_borrow: &HashSet<String>) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        if let StmtKind::Const(name, value) = &stmts[i].kind
            && ends_borrow.contains(name)
            && matches!(&value.kind, ExprKind::Member(_, property) if property == name)
            && place_read(value, name)
            && stored_whole(&stmts[i + 1], name)
            && read_again(stmts, i)
        {
            continue;
        }
        i += 1;
    }
}

/// Whether `stmt` stores `name` whole, `target = name`: where the program
/// would write the place (ADR 0866, amended). Not an argument's, `push(..,
/// handlerOnStack, ..)`, a test's, `if (alternate == null && ..)`, nor a
/// property's object, `alternate?.sibling`, where upstream keeps the name.
fn stored_whole(stmt: &Stmt, name: &str) -> bool {
    let StmtKind::Assign(_, value) = &stmt.kind else {
        return false;
    };
    // Or what a nullable place stores of it, `name ?? null` (ADR 0586).
    let value = match &value.kind {
        ExprKind::Binary(Op::Coalesce, value, null) if matches!(null.kind, ExprKind::Null) => value,
        _ => value,
    };
    matches!(&value.kind, ExprKind::Var(v) if v == name)
}

/// Whether `value`, `const name = value`, is a variable's property, or a
/// chain of them, not `name`'s, which no getter may answer.
fn place_read(value: &Expr, name: &str) -> bool {
    let mut root = value;
    while let ExprKind::Member(of, _) = &root.kind {
        root = of;
    }
    matches!(value.kind, ExprKind::Member(..))
        && matches!(&root.kind, ExprKind::Var(v) if v != name)
        && !value.has_effects()
}

/// `if (fiber != null && fiber._debugTask) { const debugTask =
/// fiber._debugTask; return debugTask.run(..); }`: a let chain's binding,
/// `stmts[at]`, of a place the chain's test read in its stead, which
/// `bound_into` says reads the same again and no getter answers, read again
/// where `stmts`, the `if`'s branch, reads it: once, in the statement right
/// after it, where nothing JS evaluates before it there does anything
/// (`leads`), so nothing can have changed the place since the test read
/// it, which does nothing either (ADR 0866, amended). As upstream writes it,
/// `return fiber._debugTask.run(..)`. Where something runs first, or it's
/// read again, the binding stays: Rust's value is what the test read.
/// Whether it was.
pub(super) fn read_again(stmts: &mut Vec<Stmt>, at: usize) -> bool {
    let (StmtKind::Const(name, value), Some(next)) = (&stmts[at].kind, stmts.get(at + 1)) else {
        return false;
    };
    if js::mentions_in(&stmts[at + 1..], name) != 1 {
        return false;
    }
    let Some(read) = read_of(next, name) else {
        return false;
    };
    if leads(read.of(next), name, None, &|e| !e.has_effects()) != Some(true) {
        return false;
    }
    let Some(replaced) = read.of(next).substitute(&|v| (v == name).then(|| value.clone())) else {
        return false;
    };
    *read.of_mut(&mut stmts[at + 1]) = replaced;
    stmts.remove(at);
    true
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
