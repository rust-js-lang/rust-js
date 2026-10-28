//! Statements and blocks walked, and what they say: each block and each
//! expression in them, the bindings they declare, and how they end.

use super::*;

impl Module {
    /// Each expression in it, outermost first, item by item.
    pub fn each_expr_mut(&mut self, f: &mut dyn FnMut(&mut Expr)) {
        each_item_expr_mut(&mut self.items, f);
    }

    /// What it holds that only an earlier phase makes and resolves, which
    /// the printer is never given (`to_oxc.rs`): a symbol linking names, an
    /// item's hole or a drop's argument the pipeline fills, a handle
    /// preparation writes, an `import()` of a module the output plan gives
    /// its specifier. The first, by what it is, if any.
    pub fn unprintable(&mut self) -> Option<&'static str> {
        let mut found = None;
        self.each_expr_mut(&mut |e| {
            let kind = match &e.kind {
                ExprKind::Symbol(_) => "a module's symbol",
                ExprKind::ItemHole(_) => "an item's hole",
                ExprKind::DropArgument(..) => "a drop's argument",
                ExprKind::Handle(_, _) | ExprKind::Pair(..) => "a `&mut`'s handle",
                ExprKind::Import(ImportFrom::Module(_)) => "an `import()` of a module without its specifier",
                _ => return,
            };
            found.get_or_insert(kind);
        });
        found
    }

    /// Each variable its code reads, a helper's `$cmp` or its own: what it
    /// imports of the package is the helpers among them (ADR 0103), found
    /// in its tree, not in its text, where a string can spell one.
    pub fn read_vars(&self) -> BTreeSet<&str> {
        let mut vars = BTreeSet::new();
        let mut read = |name| {
            vars.insert(name);
        };
        for item in &self.items {
            item.visit_vars(&mut read);
        }
        vars
    }
}

/// `x` of `x != null`, whether `x` is set, or of `x == null` where `set`
/// isn't: `undefined` as `null`, which `==` holds alike (`Expr::presence`,
/// ADR 0298). The one answer to what a test of being set is.
pub(crate) fn null_test(test: &Expr, set: bool) -> Option<&Expr> {
    let op = if set { Op::LooseNe } else { Op::LooseEq };
    match &test.kind {
        ExprKind::Binary(o, x, null) if *o == op && matches!(null.kind, ExprKind::Null | ExprKind::Undefined) => {
            Some(x)
        }
        _ => None,
    }
}

/// How a test of being set is written, of an `Option` whose `None` is only
/// one of `undefined` and `null` (`Known::NoneUndefined`,
/// `Known::NoneNull`, `Expr::presence`'s): `x !== undefined` or `x !==
/// null`, and `===` of one not set, as upstream's JS tests `window.event`
/// and `node.textContent` (ADR 0298, amended). `None` of any other test,
/// written `x != null` as it is.
pub(crate) fn strict_presence(test: &Expr) -> Option<(Op, Expr)> {
    let set = null_test(test, true).is_some();
    if !set && null_test(test, false).is_none() {
        return None;
    }
    let none = match test.facts.of_presence() {
        f if f == Facts::of(Known::NoneUndefined) => Expr::undefined(),
        f if f == Facts::of(Known::NoneNull) => Expr::null(),
        _ => return None,
    };
    Some((if set { Op::Ne } else { Op::Eq }, none))
}

/// Whether `place`, what a `take()` writes, is one rust-js made, read and
/// written as a path: a variable, or a field of one, `root.cancel`, a
/// plain data property none of whose reads a getter answers (ADR 0996),
/// so reading it twice reads the same value, and writing it runs nothing.
pub(crate) fn taken_place(place: &Expr) -> bool {
    if place.facts.has(Known::Getter) || place.facts.has(Known::Setter) {
        return false;
    }
    match &place.kind {
        ExprKind::Var(_) => true,
        ExprKind::Member(object, _) => taken_place(object),
        _ => false,
    }
}

/// Are `a` and `b` the same variable or path of one: `s`, `a.b`, `a?.b`, an
/// item by a key that reads the same, `counts[name]`, `files["/a"]`,
/// `v[0]`, or the runtime's checked item, `$index(v, i).hits`, which names
/// the same place each time (ADR 0056)? The one answer, for each pass that
/// asks whether two reads are of one place.
pub(crate) fn same_path(a: &Expr, b: &Expr) -> bool {
    let same_key = |i: &Expr, j: &Expr| {
        same_path(i, j)
            || matches!((&i.kind, &j.kind), (ExprKind::Str(i), ExprKind::Str(j)) if i == j)
            || matches!((&i.kind, &j.kind), (ExprKind::Num(i), ExprKind::Num(j)) if i == j)
    };
    match (&a.kind, &b.kind) {
        (ExprKind::Var(x), ExprKind::Var(y)) => x == y,
        (ExprKind::Member(x, f), ExprKind::Member(y, g)) => f == g && same_path(x, y),
        (ExprKind::OptionalMember(x, f), ExprKind::OptionalMember(y, g)) => f == g && same_path(x, y),
        (ExprKind::Index(x, i), ExprKind::Index(y, j)) => same_path(x, y) && same_key(i, j),
        (ExprKind::Call(f, xs), ExprKind::Call(g, ys)) => {
            matches!((f.helper_name(), g.helper_name()), (Some(f), Some(g)) if f == g && crate::runtime::names_a_place(f))
                && xs.len() == ys.len()
                && xs.iter().zip(ys).all(|(x, y)| same_key(x, y))
        }
        _ => false,
    }
}

/// Is `name` a handler's, `on` and a capital, `onClick`, `onSubmit`?
pub fn is_handler_name(name: &str) -> bool {
    name.strip_prefix("on")
        .is_some_and(|s| s.starts_with(|c: char| c.is_ascii_uppercase()))
}

/// What a function of `params` and `body` passes on what it's given to, in
/// order, rest arguments spread on, `(x, ...rest) => g(x, ...rest)`: its
/// call's callee, which names none of its parameters; and, `statement`, of
/// `() => { g(); }`. Where that's the function it may be is its taker's,
/// `lower/passed_on.rs`'s (ADR 0605, amended), and a declaration's,
/// `prepare.rs`'s (ADR 0610).
pub fn passes_on<'e>(params: &[Pattern], body: &'e [Stmt], statement: bool) -> Option<&'e Expr> {
    let call = match body {
        [
            Stmt {
                kind: StmtKind::Return(Some(call)),
                ..
            },
        ] => call,
        [
            Stmt {
                kind: StmtKind::Expr(call),
                ..
            },
        ] if statement => call,
        _ => return None,
    };
    let ExprKind::Call(callee, args) = &call.kind else {
        return None;
    };
    let names: Option<Vec<&str>> = (params.iter())
        .zip(args)
        .map(|(param, arg)| match (param, &arg.kind) {
            (Pattern::Name(name), ExprKind::Var(v)) if v == name => Some(name.as_str()),
            (Pattern::Rest(name), ExprKind::Spread(spread)) if matches!(&spread.kind, ExprKind::Var(v) if v == name) => {
                Some(name.as_str())
            }
            _ => None,
        })
        .collect();
    let names = names.filter(|_| params.len() == args.len())?;
    (!names.iter().any(|name| callee.mentions_var(name))).then_some(&**callee)
}

/// An arrow of one call, `() => { f(); }`, as `() => f()`, which gives
/// what the call gives: for a call that gives what nothing reads, or
/// `undefined` whatever it is (ADR 0040).
pub fn returning_its_call(value: &mut Expr) {
    if let ExprKind::Arrow(_, body) = &mut value.kind
        && let [
            Stmt {
                kind: StmtKind::Expr(e),
                span,
            },
        ] = body.as_slice()
    {
        *body = vec![StmtKind::Return(Some(e.clone())).at(*span)];
    }
}

/// Each list of statements in `stmts`, itself first, then each nested one,
/// a closure's body too, and a function's declared in them, `function
/// inner() { .. }` (ADR 0308): for a pass that changes them. Each once.
pub fn each_block_mut(stmts: &mut Vec<Stmt>, f: &mut dyn FnMut(&mut Vec<Stmt>)) {
    each_slotted_block_mut(stmts, &mut |list, _| f(list));
}

/// `each_block_mut`, each list given where it is (`Slot`).
fn each_slotted_block_mut(stmts: &mut Vec<Stmt>, f: &mut dyn FnMut(&mut Vec<Stmt>, Slot)) {
    own_blocks(stmts, f);
    // Each closure, however deep, a declared function's too.
    each_expr_mut(stmts, &mut |e| closure_blocks(e, f));
}

/// `each_block_mut` of an expression's closures, a constant's value's.
fn each_block_in_mut(value: &mut Expr, f: &mut dyn FnMut(&mut Vec<Stmt>, Slot)) {
    value.each_mut(&mut |e| closure_blocks(e, f));
}

/// `stmts`' own lists, and those of the functions declared in them.
fn own_blocks(stmts: &mut Vec<Stmt>, f: &mut dyn FnMut(&mut Vec<Stmt>, Slot)) {
    statement_lists_in(stmts, Slot::Block, &mut |list, slot| {
        f(list, slot);
        for stmt in list.iter_mut() {
            if let StmtKind::Function(function) = &mut stmt.kind {
                own_blocks(&mut function.body, f);
            }
        }
    });
}

/// The lists of `e`, if it's a closure, its own, not its closures'.
fn closure_blocks(e: &mut Expr, f: &mut dyn FnMut(&mut Vec<Stmt>, Slot)) {
    match &mut e.kind {
        ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => own_blocks(body, f),
        ExprKind::Function(function) => own_blocks(&mut function.body, f),
        _ => {}
    }
}

/// `a == b`, or `a != b`, as `a === b` and `a !== b`: of what either
/// answers the same, one side never `null` or `undefined`. So too
/// `nullish_equal`'s `(a ?? null) === (b ?? null)`, whose `?? null` only
/// makes a `null` of an `undefined`.
pub(crate) fn strict_equality(e: Expr) -> Expr {
    match e.kind {
        ExprKind::Binary(Op::LooseEq, a, b) => Expr::bin(Op::Eq, *a, *b),
        ExprKind::Binary(Op::LooseNe, a, b) => Expr::bin(Op::Ne, *a, *b),
        ExprKind::Binary(op @ (Op::Eq | Op::Ne), ref a, ref b)
            if let (Some(a), Some(b)) = (nullish_operand(a), nullish_operand(b)) =>
        {
            Expr::bin(op, a.clone(), b.clone())
        }
        kind => Expr { kind, ..e },
    }
}

/// `(a ?? null) === (b ?? null)`: `a === b`, where `null` and `undefined`
/// are one, as two `None`s of an `Option` of a value of any shape are,
/// whose `Some`s `===` compares, as `==` can't, which makes `"1"` of `1`
/// (ADR 0612, amended). Each is read once, in order.
pub(crate) fn nullish_equal(a: Expr, b: Expr) -> Expr {
    let either = |x: Expr| Expr::bin(Op::Coalesce, x, Expr::null());
    Expr::bin(Op::Eq, either(a), either(b))
}

/// What `nullish_equal` wrote `?? null` of.
fn nullish_operand(e: &Expr) -> Option<&Expr> {
    match &e.kind {
        ExprKind::Binary(Op::Coalesce, x, fallback) if matches!(fallback.kind, ExprKind::Null) => Some(x),
        _ => None,
    }
}

/// `let list = [];` that nothing sets again, only changes in place, is
/// `const list = [];`, as JS writes it (ADR 0278). A variable a `&mut`'s
/// handle sets is set.
pub(crate) fn constants(body: &mut Vec<Stmt>) {
    let writes = Writes::of(body);
    each_block_mut(body, &mut |stmts| {
        for stmt in stmts.iter_mut() {
            if let StmtKind::Let(name, Some(value)) = &stmt.kind
                && !writes.may_replace(name)
            {
                stmt.kind = StmtKind::Const(name.clone(), value.clone());
            }
        }
    });
}

/// How many times `stmts` name the variable `name`, read or written.
pub fn mentions_in(stmts: &[Stmt], name: &str) -> usize {
    let mut count = 0;
    visit_stmts(stmts, &mut |var| {
        if var == name {
            count += 1;
        }
    });
    count
}

/// Each variable `stmts` read, in every statement and expression.
pub fn visit_stmts<'a>(stmts: &'a [Stmt], read: &mut dyn FnMut(&'a str)) {
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::Const(_, value)
            | StmtKind::Destructure { value, .. }
            | StmtKind::Expr(value)
            | StmtKind::Throw(value) => value.visit_vars(read),
            StmtKind::Let(_, value) | StmtKind::Return(value) => {
                if let Some(value) = value {
                    value.visit_vars(read);
                }
            }
            StmtKind::Assign(target, value) | StmtKind::Compound(_, target, value) => {
                target.visit_vars(read);
                value.visit_vars(read);
            }
            StmtKind::If(test, then, els) => {
                test.visit_vars(read);
                visit_stmts(then, read);
                if let Some(els) = els {
                    visit_stmts(els, read);
                }
            }
            StmtKind::While { cond, body, .. } | StmtKind::DoWhile { cond, body, .. } => {
                cond.visit_vars(read);
                visit_stmts(body, read);
            }
            StmtKind::ForOf { iterable, body, .. } => {
                iterable.visit_vars(read);
                visit_stmts(body, read);
            }
            StmtKind::For {
                start,
                test,
                step,
                body,
                ..
            } => {
                start.visit_vars(read);
                test.visit_vars(read);
                visit_stmts(body, read);
                if let Some(value) = step.value() {
                    value.visit_vars(read);
                }
            }
            StmtKind::Labeled(_, body) => visit_stmts(body, read),
            StmtKind::Switch {
                discriminant,
                cases,
                default,
            } => {
                discriminant.visit_vars(read);
                for Case { values, body, .. } in cases {
                    values.iter().for_each(|value| value.visit_vars(read));
                    visit_stmts(body, read);
                }
                if let Some(default) = default {
                    visit_stmts(default, read);
                }
            }
            StmtKind::Try(body, finally) | StmtKind::TryCatch(body, _, finally) => {
                visit_stmts(body, read);
                visit_stmts(finally, read);
            }
            StmtKind::Function(function) => visit_stmts(&function.body, read),
            StmtKind::Break(_) | StmtKind::Continue(_) | StmtKind::Directive(_) | StmtKind::Comments => {}
        }
    }
}

/// Whether running `stmt` never goes on to what follows it.
pub fn ends(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Return(_) | StmtKind::Throw(_) | StmtKind::Break(_) | StmtKind::Continue(_) => true,
        // A call that never returns (ADR 0954).
        StmtKind::Expr(e) => e.facts.has(Known::Diverges),
        StmtKind::If(_, then, Some(otherwise)) => then.last().is_some_and(ends) && otherwise.last().is_some_and(ends),
        // A `switch` with a `default`, each of whose cases leaves (ADR
        // 0660), or falls to the next case (ADR 0905); not by a `break` of no
        // label, which leaves the `switch` for what follows it, `if
        // (legacyOff) { break; }` (ADR 0905, amended).
        StmtKind::Switch {
            cases,
            default: Some(default),
            ..
        } => {
            cases
                .iter()
                .all(|case| case.falls || case.body.last().is_some_and(ends))
                && default.last().is_some_and(ends)
                && !(cases.iter().map(|case| &case.body).chain([default])).any(|body| breaks(body, None))
        }
        StmtKind::While { label, cond, body } => {
            matches!(cond.kind, ExprKind::Bool(true)) && !breaks(body, label.as_deref())
        }
        // A `try` each of whose arms leaves, `try { return f(); } catch {
        // return ""; }` (ADR 0629, amended).
        StmtKind::TryCatch(body, _, caught) => body.last().is_some_and(ends) && caught.last().is_some_and(ends),
        _ => false,
    }
}

impl Stmt {
    /// Whether running this only makes values and assigns them, none of
    /// which may throw (`Expr::may_throw`): it neither throws nor leaves.
    /// The one answer of a statement's: what a panic's drops need no
    /// `catch` of, what a scope's `finally` runs past, what a `try` of a
    /// throwing call holds.
    pub fn only_makes_values(&self) -> bool {
        match &self.kind {
            StmtKind::Const(_, value) | StmtKind::Let(_, Some(value)) | StmtKind::Expr(value) => !value.may_throw(),
            // Not of a setter's property, which may throw (`Known::Setter`).
            StmtKind::Assign(target, value) => {
                !target.facts.has(Known::Setter) && !target.may_throw() && !value.may_throw()
            }
            StmtKind::Let(_, None) => true,
            _ => false,
        }
    }

    /// Whether running this can't throw, though it may run the program's
    /// code: what only makes values, and what catches whatever its code
    /// throws, `const result = $catchUnwind(() => ..)` (ADR 0873). What a
    /// scope's `finally` runs past, as it runs past what makes values.
    pub fn cannot_throw(&self) -> bool {
        let caught = |value: &Expr| {
            matches!(&value.kind, ExprKind::Call(callee, args)
                if callee.helper_name() == Some("$catchUnwind") && !args.iter().any(Expr::may_throw))
        };
        self.only_makes_values()
            || match &self.kind {
                StmtKind::Const(_, value) | StmtKind::Let(_, Some(value)) | StmtKind::Expr(value) => caught(value),
                StmtKind::Assign(target, value) => matches!(target.kind, ExprKind::Var(_)) && caught(value),
                _ => false,
            }
    }
}

/// A `dyn`'s pair, `{ value, impl }`, made of what does nothing: a shared
/// reference's, which nothing assigns through, so its parts read alike.
pub fn is_read_pair(props: &[Prop]) -> bool {
    matches!(props, [Prop::Field(value, held), Prop::Field(dictionary, given)]
        if value == HELD && dictionary == DICTIONARY && !held.has_effects() && !given.has_effects())
}

/// Whether `a` and `b` are the same JS, part by part, wherever each was
/// written, whatever it names what it declares and whatever's known of its
/// parts, the same JS doing the same: the drops a panic runs and the
/// scope's end runs, `for (const item$1 of v)` and `for (const item of
/// v)`, say, one of a function the analysis found never unwinds, so marked,
/// the other not (ADR 0902).
pub fn stmts_alike(a: &[Stmt], b: &[Stmt]) -> bool {
    let canonical = |stmts: &[Stmt]| {
        let mut stmts = stmts.to_vec();
        unspan(&mut stmts);
        let mut declared = Vec::new();
        binders(&mut stmts, &mut declared);
        let names: HashMap<String, String> = (declared.into_iter().enumerate())
            .map(|(i, name)| (name, format!("${i}")))
            .collect();
        let rename = |name: &mut String| {
            if let Some(to) = names.get(name.as_str()) {
                *name = to.clone();
            }
        };
        rename_binders(&mut stmts, &mut |n| rename(n));
        each_expr_mut(&mut stmts, &mut |e| {
            e.span = Span::NONE;
            e.facts = Facts::NONE;
            match &mut e.kind {
                ExprKind::Var(name) => rename(name),
                ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) => {
                    params.iter_mut().for_each(|p| p.each_name_mut(&mut |n| rename(n)));
                    unspan(body);
                    rename_binders(body, &mut |n| rename(n));
                }
                ExprKind::Function(function) => {
                    unspan(&mut function.body);
                    rename_binders(&mut function.body, &mut |n| rename(n));
                }
                _ => {}
            }
        });
        stmts
    };
    a.len() == b.len() && canonical(a) == canonical(b)
}

/// Whether `stmts` declare the name `name`, however deep, a closure's
/// parameter or declaration in them too: where a read of it there may be
/// of another than the one before them.
pub fn declares(stmts: &[Stmt], name: &str) -> bool {
    let mut copy = stmts.to_vec();
    let mut found = false;
    rename_binders(&mut copy, &mut |n| found |= n == name);
    each_expr_mut(&mut copy, &mut |e| match &mut e.kind {
        ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) => {
            params
                .iter_mut()
                .for_each(|p| p.each_name_mut(&mut |n| found |= n == name));
            rename_binders(body, &mut |n| found |= n == name);
        }
        ExprKind::Function(function) => {
            (function.params.iter_mut()).for_each(|p| p.each_name_mut(&mut |n| found |= n == name));
            rename_binders(&mut function.body, &mut |n| found |= n == name);
        }
        _ => {}
    });
    found
}

/// `stmts` with each declaration and read of the name `from`, a closure's
/// in them too, of `to`, which none of them mentions: the same program,
/// where `from` is one `stmts` declare at their top.
pub fn renamed(stmts: &mut [Stmt], from: &str, to: &str) {
    let rename = |name: &mut String| {
        if name == from {
            *name = to.to_string();
        }
    };
    rename_binders(stmts, &mut |n| rename(n));
    each_expr_mut(stmts, &mut |e| match &mut e.kind {
        ExprKind::Var(name) => rename(name),
        ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) => {
            params.iter_mut().for_each(|p| p.each_name_mut(&mut |n| rename(n)));
            rename_binders(body, &mut |n| rename(n));
        }
        ExprKind::Function(function) => {
            function
                .params
                .iter_mut()
                .for_each(|p| p.each_name_mut(&mut |n| rename(n)));
            rename_binders(&mut function.body, &mut |n| rename(n));
        }
        _ => {}
    });
}

/// `change` of `stmts`, a renaming of `name`, made where `name` is the one
/// `stmts` mean by it: each function in them that takes a parameter of the
/// name, a closure's `(_2) => _2[0]`, reads its own there, so it's set
/// apart while `change` runs, then given back.
pub fn outside_shadows(stmts: &mut Vec<Stmt>, name: &str, change: impl FnOnce(&mut Vec<Stmt>)) {
    const APART: &str = "\u{0}apart";
    each_expr_mut(stmts, &mut |e| {
        let (params, body) = match &mut e.kind {
            ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) => (params, body),
            ExprKind::Function(function) => (&mut function.params, &mut function.body),
            _ => return,
        };
        if params.iter().any(|p| p.names().contains(&name)) {
            for param in params.iter_mut() {
                param.each_name_mut(&mut |n| {
                    if n == name {
                        *n = APART.to_string();
                    }
                });
            }
            renamed(body, name, APART);
        }
    });
    change(stmts);
    renamed(stmts, APART, name);
}

/// Each read of the variable `name` in `stmts` JS's `this`: in them, and
/// in each arrow in them, which sees the `this` of the function it's
/// made in; not in a function made in them, `js::function::with_this`'s,
/// which JS calls with a `this` of its own, where `name` stays. Whether
/// one reads it there, so `stmts` must declare it, `const name = this;`
/// (ADRs 0761, 0972). Of a body lowered, before preparation makes
/// methods of arrows (ADR 0882), and where a function declared in it is
/// a nested `fn`'s, which captures nothing.
pub fn read_as_this(stmts: &mut [Stmt], name: &str) -> bool {
    const APART: &str = "\u{0}this";
    let mut nested = false;
    each_expr_mut(stmts, &mut |e| {
        if let ExprKind::Function(function) = &mut e.kind
            && mentions_in(&function.body, name) > 0
        {
            nested = true;
            renamed(&mut function.body, name, APART);
        }
    });
    each_expr_mut(stmts, &mut |e| {
        if matches!(&e.kind, ExprKind::Var(v) if v == name) {
            e.kind = ExprKind::This;
        }
    });
    renamed(stmts, APART, name);
    nested
}

/// Whether `stmts` read their function's `this`: in them, or in an arrow
/// in them, not in a function made in them, whose `this` is its own.
pub fn reads_this(stmts: &[Stmt]) -> bool {
    let mut stmts = stmts.to_vec();
    each_expr_mut(&mut stmts, &mut |e| {
        if let ExprKind::Function(function) = &mut e.kind {
            function.body.clear();
        }
    });
    let mut read = false;
    each_expr(&stmts, &mut |e| read |= matches!(e.kind, ExprKind::This));
    read
}

/// Each name `stmts` declare, however deep: a statement's, an arrow's
/// parameters and statements, a function's too.
pub fn declared_names(stmts: &[Stmt]) -> Vec<String> {
    let mut stmts = stmts.to_vec();
    let mut declared = Vec::new();
    binders(&mut stmts, &mut declared);
    each_expr_mut(&mut stmts, &mut |e| {
        if let ExprKind::Function(function) = &mut e.kind {
            for param in function.params.iter_mut() {
                param.each_name_mut(&mut |n| declared.push(n.clone()));
            }
        }
    });
    declared
}

/// The names `stmts` declare, in order, an arrow's in them too.
fn binders(stmts: &mut [Stmt], declared: &mut Vec<String>) {
    rename_binders(stmts, &mut |name| declared.push(name.clone()));
    each_expr_mut(stmts, &mut |e| match &mut e.kind {
        ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) => {
            params
                .iter_mut()
                .for_each(|p| p.each_name_mut(&mut |n| declared.push(n.clone())));
            rename_binders(body, &mut |n: &mut String| declared.push(n.clone()));
        }
        ExprKind::Function(function) => {
            rename_binders(&mut function.body, &mut |n: &mut String| declared.push(n.clone()))
        }
        _ => {}
    });
}

/// `rename` given each name `stmts`' statements declare, however deep, not
/// an arrow's in their expressions.
fn rename_binders(stmts: &mut [Stmt], rename: &mut dyn FnMut(&mut String)) {
    for stmt in stmts {
        match &mut stmt.kind {
            StmtKind::Const(name, _) | StmtKind::Let(name, _) => rename(name),
            StmtKind::Destructure { pattern, .. } => pattern.each_name_mut(&mut |n| rename(n)),
            StmtKind::If(_, a, b) => {
                rename_binders(a, rename);
                if let Some(b) = b {
                    rename_binders(b, rename);
                }
            }
            StmtKind::While { body, .. } | StmtKind::DoWhile { body, .. } | StmtKind::Labeled(_, body) => {
                rename_binders(body, rename)
            }
            StmtKind::Switch { cases, default, .. } => {
                case_lists_mut(cases, default).for_each(|body| rename_binders(body, rename));
            }
            StmtKind::ForOf { pattern, body, .. } => {
                pattern.each_name_mut(&mut |n| rename(n));
                rename_binders(body, rename);
            }
            StmtKind::For { name, body, .. } => {
                rename(name);
                rename_binders(body, rename);
            }
            StmtKind::Try(a, b) => {
                rename_binders(a, rename);
                rename_binders(b, rename);
            }
            StmtKind::TryCatch(a, error, b) => {
                rename_binders(a, rename);
                if let Some(error) = error {
                    rename(error);
                }
                rename_binders(b, rename);
            }
            StmtKind::Function(function) => {
                rename(&mut function.name);
                function
                    .params
                    .iter_mut()
                    .for_each(|p| p.each_name_mut(&mut |n| rename(n)));
                rename_binders(&mut function.body, rename);
            }
            _ => {}
        }
    }
}

/// Each statement of `stmts`, however deep, of no span.
fn unspan(stmts: &mut [Stmt]) {
    for stmt in stmts {
        stmt.span = Span::NONE;
        match &mut stmt.kind {
            StmtKind::If(_, a, b) => {
                unspan(a);
                if let Some(b) = b {
                    unspan(b);
                }
            }
            StmtKind::While { body, .. }
            | StmtKind::DoWhile { body, .. }
            | StmtKind::ForOf { body, .. }
            | StmtKind::For { body, .. }
            | StmtKind::Labeled(_, body) => unspan(body),
            StmtKind::Switch { cases, default, .. } => case_lists_mut(cases, default).for_each(|body| unspan(body)),
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                unspan(a);
                unspan(b);
            }
            StmtKind::Function(function) => unspan(&mut function.body),
            _ => {}
        }
    }
}

/// Whether a `break` in `stmts` leaves the loop they're the body of, labeled
/// `label`: one of no label not inside a loop of their own, or of its label.
pub fn breaks(stmts: &[Stmt], label: Option<&str>) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Break(None) => true,
        StmtKind::Break(Some(l)) => Some(l.as_str()) == label,
        StmtKind::If(_, a, b) => breaks(a, label) || b.as_deref().is_some_and(|b| breaks(b, label)),
        StmtKind::Labeled(_, body) => breaks(body, label),
        // In a loop of its own, one that names it; in a `switch`, which a
        // `break` of no label would leave, too (ADR 0629).
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. } => label.is_some_and(|l| names_break(body, l)),
        StmtKind::Switch { cases, default, .. } => {
            label.is_some_and(|l| case_lists(cases, default).any(|body| names_break(body, l)))
        }
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => breaks(a, label) || breaks(b, label),
        _ => false,
    })
}

/// Whether a `continue` in `stmts` goes on with the loop they're the body
/// of, labeled `label`: one of no label not inside a loop of their own, a
/// `switch`'s too, which takes none, or one of its label.
pub fn continues(stmts: &[Stmt], label: Option<&str>) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Continue(None) => true,
        StmtKind::Continue(Some(l)) => Some(l.as_str()) == label,
        StmtKind::If(_, a, b) => continues(a, label) || b.as_deref().is_some_and(|b| continues(b, label)),
        StmtKind::Labeled(_, body) => continues(body, label),
        StmtKind::Switch { cases, default, .. } => case_lists(cases, default).any(|body| continues(body, label)),
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. } => label.is_some_and(|l| names_jump(body, l)),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => continues(a, label) || continues(b, label),
        _ => false,
    })
}

/// Whether a `break` in `stmts`, however deep, names `label`.
pub fn names_break(stmts: &[Stmt], label: &str) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Break(Some(l)) => l == label,
        StmtKind::If(_, a, b) => names_break(a, label) || b.as_deref().is_some_and(|b| names_break(b, label)),
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Labeled(_, body) => names_break(body, label),
        StmtKind::Switch { cases, default, .. } => case_lists(cases, default).any(|body| names_break(body, label)),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => names_break(a, label) || names_break(b, label),
        _ => false,
    })
}

/// Whether a `break` or a `continue` in `stmts`, however deep, names `label`.
pub fn names_jump(stmts: &[Stmt], label: &str) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Break(Some(l)) | StmtKind::Continue(Some(l)) => l == label,
        StmtKind::If(_, a, b) => names_jump(a, label) || b.as_deref().is_some_and(|b| names_jump(b, label)),
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Labeled(_, body) => names_jump(body, label),
        StmtKind::Switch { cases, default, .. } => case_lists(cases, default).any(|body| names_jump(body, label)),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => names_jump(a, label) || names_jump(b, label),
        _ => false,
    })
}

/// `items` where their Rust has them, each made when its module loads
/// after the `const`s and types it reads, the functions' it calls read
/// too, closures' too: JS throws on a `const` read before it's made, where
/// Rust's thread-locals are made when first read. Of a cycle, the first
/// written comes first (ADR 0306).
pub fn in_load_order(items: Vec<Item>) -> Vec<Item> {
    // Where each is written: one in another, a `const` in a function's
    // body, just before it; one without a place, beside the one before it.
    let spans: Vec<Vec<Span>> = items.iter().map(Item::spans).collect();
    let around = |inner: Span| {
        (spans.iter().flatten())
            .filter(|outer| outer.lo <= inner.lo && inner.hi <= outer.hi && **outer != inner)
            .map(|outer| outer.lo)
            .min()
    };
    let mut at = (0, false, 0);
    let keys: Vec<(u32, bool, u32)> = (spans.iter())
        .map(|spans| {
            if let Some(&own) = spans.iter().min_by_key(|span| span.lo) {
                at = match around(own) {
                    Some(outer) => (outer, false, own.lo),
                    None => (own.lo, true, own.lo),
                };
            }
            at
        })
        .collect();
    let mut placed: Vec<((u32, bool, u32), Item)> = keys.into_iter().zip(items).collect();
    placed.sort_by_key(|(at, _)| *at);
    let mut items: Vec<Option<Item>> = placed.into_iter().map(|(_, item)| Some(item)).collect();

    let order = {
        let items: Vec<&Item> = items.iter().flatten().collect();
        let declaring: HashMap<&str, usize> = (items.iter().enumerate())
            .filter_map(|(i, item)| Some((item.declared()?, i)))
            .collect();
        let called: HashMap<&str, &Item> = (items.iter())
            .filter_map(|item| match item {
                Item::Function(function) => Some((function.name.as_str(), *item)),
                Item::Namespace(namespace) => Some((namespace.name.as_str(), *item)),
                Item::Const(_) | Item::Statements(_) => None,
            })
            .collect();
        // What each `const` holds, a function its value may be, which what
        // reads it may call (ADR 0306, amended).
        let holds: HashMap<&str, Vec<&str>> = (items.iter())
            .filter_map(|item| match item {
                Item::Const(constant) => {
                    let mut held = Vec::new();
                    (constant.value).visit_held_vars(&mut |v, only_held| {
                        if only_held {
                            held.push(v);
                        }
                    });
                    Some((constant.name.as_str(), held))
                }
                _ => None,
            })
            .collect();
        // The items each reads when it's loaded: what its value only holds,
        // `clz32Fallback`, made before it but not run; what it runs, and
        // what that reads, calls and holds, all of it.
        let reads: Vec<HashSet<usize>> = (items.iter().enumerate())
            .map(|(i, item)| {
                let mut seen = HashSet::new();
                let mut work = Vec::new();
                match item {
                    Item::Const(constant) => constant.value.visit_held_vars(&mut |v, only_held| match only_held {
                        true => _ = seen.insert(v),
                        false => work.push(v),
                    }),
                    Item::Statements(stmts) => visit_stmts(stmts, &mut |v| work.push(v)),
                    Item::Namespace(_) | Item::Function(_) => {}
                }
                let mut ran = HashSet::new();
                while let Some(name) = work.pop() {
                    seen.insert(name);
                    if ran.insert(name) {
                        if let Some(item) = called.get(name) {
                            item.visit_vars(&mut |v| work.push(v));
                        }
                        work.extend(holds.get(name).into_iter().flatten());
                    }
                }
                seen.iter()
                    .filter_map(|name| declaring.get(name))
                    .copied()
                    .filter(|&d| d != i)
                    .collect()
            })
            .collect();
        let mut done = vec![false; items.len()];
        let mut order = Vec::new();
        while let Some(next) = (0..items.len())
            .find(|&i| !done[i] && reads[i].iter().all(|&d| done[d]))
            .or_else(|| done.iter().position(|d| !d))
        {
            done[next] = true;
            order.push(next);
        }
        order
    };
    order.into_iter().filter_map(|i| items[i].take()).collect()
}

/// Each expression in `items`, outermost first, as `each_expr_mut` goes: a
/// namespace's constants' too, an impl's `const`s.
pub fn each_item_expr_mut(items: &mut [Item], f: &mut dyn FnMut(&mut Expr)) {
    for item in items {
        match item {
            Item::Const(constant) => constant.value.each_mut(f),
            Item::Statements(stmts) => each_expr_mut(stmts, f),
            Item::Namespace(namespace) => {
                (namespace.constants.iter_mut()).for_each(|constant| constant.value.each_mut(f));
                (namespace.methods.iter_mut()).for_each(|method| function_exprs_mut(method, f));
            }
            Item::Function(function) => function_exprs_mut(function, f),
        }
    }
}

/// Each expression of `function`: its parameters' defaults, `IIterator = {
/// next: $next }`'s (ADR 0995), and its body's.
pub fn function_exprs_mut(function: &mut Function, f: &mut dyn FnMut(&mut Expr)) {
    (function.params.iter_mut()).for_each(|param| param.defaults_mut().into_iter().for_each(|d| d.each_mut(f)));
    each_expr_mut(&mut function.body, f);
}

/// Each list of statements in `items`, as `each_block_mut` goes: what a
/// module runs when it's loaded, its functions' and methods' bodies, and
/// each closure's, a constant's too.
pub fn each_item_block_mut(items: &mut [Item], f: &mut dyn FnMut(&mut Vec<Stmt>)) {
    each_item_slotted_block_mut(items, &mut |list, _| f(list));
}

/// `each_item_block_mut`, each list given where it is (`Slot`): an `if`'s
/// `else`, or another block.
pub fn each_item_slotted_block_mut(items: &mut [Item], f: &mut dyn FnMut(&mut Vec<Stmt>, Slot)) {
    for item in items {
        match item {
            Item::Const(constant) => each_block_in_mut(&mut constant.value, f),
            Item::Statements(stmts) => each_slotted_block_mut(stmts, f),
            Item::Namespace(namespace) => {
                (namespace.constants.iter_mut()).for_each(|constant| each_block_in_mut(&mut constant.value, f));
                (namespace.methods.iter_mut()).for_each(|method| each_slotted_block_mut(&mut method.body, f));
            }
            Item::Function(function) => each_slotted_block_mut(&mut function.body, f),
        }
    }
}

/// A `switch`'s lists of statements: each case's, then its default's.
pub fn case_lists<'s>(cases: &'s [Case], default: &'s Option<Vec<Stmt>>) -> impl Iterator<Item = &'s Vec<Stmt>> {
    cases.iter().map(|case| &case.body).chain(default)
}

/// A `switch`'s lists of statements, to change: each case's, then its
/// default's.
pub fn case_lists_mut<'s>(
    cases: &'s mut [Case],
    default: &'s mut Option<Vec<Stmt>>,
) -> impl Iterator<Item = &'s mut Vec<Stmt>> {
    cases.iter_mut().map(|case| &mut case.body).chain(default.as_mut())
}
