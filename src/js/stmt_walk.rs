//! Statements and blocks walked, and what they say: each block and each
//! expression in them, the bindings they declare, and how they end.

use super::*;

impl Module {
    /// Each expression in it, outermost first, item by item.
    pub fn each_expr_mut(&mut self, f: &mut dyn FnMut(&mut Expr)) {
        each_item_expr_mut(&mut self.items, f);
    }

    /// What it holds that only an earlier phase makes and resolves, which
    /// the printer is never given (`to_oxc.rs`): a symbol linking names, a
    /// function's hole or a drop's argument the pipeline fills, a handle
    /// preparation writes, an `import()` of a module the output plan gives
    /// its specifier. The first, by what it is, if any.
    pub fn unprintable(&mut self) -> Option<&'static str> {
        let mut found = None;
        self.each_expr_mut(&mut |e| {
            let kind = match &e.kind {
                ExprKind::Symbol(_) => "a module's symbol",
                ExprKind::FunctionHole(_) => "a function's hole",
                ExprKind::DropArgument(..) => "a drop's argument",
                ExprKind::Handle(_) | ExprKind::Pair(..) => "a `&mut`'s handle",
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

/// What a presence test tests: `x` of `x != null`, or of `!!x`, an
/// `Option` of a value never falsy (ADR 0298).
pub(crate) fn presence_of(test: &Expr) -> Option<&Expr> {
    match &test.kind {
        _ if let Some(x) = null_test(test, true) => Some(x),
        ExprKind::Unary(UnaryOp::Not, not) => match &not.kind {
            ExprKind::Unary(UnaryOp::Not, x) => Some(x),
            _ => None,
        },
        _ => None,
    }
}

/// `x` of `x != null`, whether `x` is set, or of `x == null` where `set`
/// isn't: `undefined` as `null`, which `==` holds alike. The one answer to
/// what a test of being set is.
pub(crate) fn null_test(test: &Expr, set: bool) -> Option<&Expr> {
    let op = if set { Op::LooseNe } else { Op::LooseEq };
    match &test.kind {
        ExprKind::Binary(o, x, null) if *o == op && matches!(null.kind, ExprKind::Null | ExprKind::Undefined) => {
            Some(x)
        }
        _ => None,
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
    own_blocks(stmts, f);
    // Each closure, however deep, a declared function's too.
    each_expr_mut(stmts, &mut |e| closure_blocks(e, f));
}

/// `each_block_mut` of an expression's closures, a constant's value's.
fn each_block_in_mut(value: &mut Expr, f: &mut dyn FnMut(&mut Vec<Stmt>)) {
    value.each_mut(&mut |e| closure_blocks(e, f));
}

/// `stmts`' own lists, and those of the functions declared in them.
fn own_blocks(stmts: &mut Vec<Stmt>, f: &mut dyn FnMut(&mut Vec<Stmt>)) {
    statement_lists(stmts, &mut |list| {
        f(list);
        for stmt in list.iter_mut() {
            if let StmtKind::Function(function) = &mut stmt.kind {
                own_blocks(&mut function.body, f);
            }
        }
    });
}

/// The lists of `e`, if it's a closure, its own, not its closures'.
fn closure_blocks(e: &mut Expr, f: &mut dyn FnMut(&mut Vec<Stmt>)) {
    match &mut e.kind {
        ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => own_blocks(body, f),
        ExprKind::Function(function) => own_blocks(&mut function.body, f),
        _ => {}
    }
}

/// `stmts`, and each list nested in its statements, not in its closures.
pub fn statement_lists(stmts: &mut Vec<Stmt>, f: &mut dyn FnMut(&mut Vec<Stmt>)) {
    f(stmts);
    for stmt in stmts.iter_mut() {
        match &mut stmt.kind {
            StmtKind::If(_, a, b) => {
                statement_lists(a, f);
                if let Some(b) = b {
                    statement_lists(b, f);
                }
            }
            StmtKind::While { body, .. }
            | StmtKind::ForOf { body, .. }
            | StmtKind::For { body, .. }
            | StmtKind::Labeled(_, body) => statement_lists(body, f),
            StmtKind::Switch { cases, default, .. } => {
                case_lists_mut(cases, default).for_each(|body| statement_lists(body, f));
            }
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                statement_lists(a, f);
                statement_lists(b, f);
            }
            // A function's, as a closure's, are its own.
            StmtKind::Const(..)
            | StmtKind::Let(..)
            | StmtKind::Destructure { .. }
            | StmtKind::Assign(..)
            | StmtKind::Compound(..)
            | StmtKind::Expr(_)
            | StmtKind::Break(_)
            | StmtKind::Continue(_)
            | StmtKind::Directive(_)
            | StmtKind::Return(_)
            | StmtKind::Throw(_)
            | StmtKind::Function(_) => {}
        }
    }
}

/// Each expression in `stmts`, outermost first, nested ones too, in
/// closures' bodies, JSX and templates: for a pass that changes them.
pub fn each_expr_mut(stmts: &mut [Stmt], f: &mut dyn FnMut(&mut Expr)) {
    for stmt in stmts {
        match &mut stmt.kind {
            StmtKind::Const(_, e)
            | StmtKind::Let(_, Some(e))
            | StmtKind::Destructure { value: e, .. }
            | StmtKind::Expr(e)
            | StmtKind::Return(Some(e))
            | StmtKind::Throw(e) => e.each_mut(f),
            StmtKind::Assign(a, b) | StmtKind::Compound(_, a, b) => {
                a.each_mut(f);
                b.each_mut(f);
            }
            StmtKind::If(test, a, b) => {
                test.each_mut(f);
                each_expr_mut(a, f);
                if let Some(b) = b {
                    each_expr_mut(b, f);
                }
            }
            StmtKind::While { cond, body, .. } => {
                cond.each_mut(f);
                each_expr_mut(body, f);
            }
            StmtKind::ForOf { iterable, body, .. } => {
                iterable.each_mut(f);
                each_expr_mut(body, f);
            }
            StmtKind::For { start, test, body, .. } => {
                start.each_mut(f);
                test.each_mut(f);
                each_expr_mut(body, f);
            }
            StmtKind::Labeled(_, body) => each_expr_mut(body, f),
            StmtKind::Switch {
                discriminant,
                cases,
                default,
            } => {
                discriminant.each_mut(f);
                for (values, body) in cases.iter_mut() {
                    values.iter_mut().for_each(|value| value.each_mut(f));
                    each_expr_mut(body, f);
                }
                if let Some(default) = default {
                    each_expr_mut(default, f);
                }
            }
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                each_expr_mut(a, f);
                each_expr_mut(b, f);
            }
            StmtKind::Function(function) => each_expr_mut(&mut function.body, f),
            StmtKind::Let(_, None)
            | StmtKind::Break(_)
            | StmtKind::Continue(_)
            | StmtKind::Return(None)
            | StmtKind::Directive(_) => {}
        }
    }
}

/// `a == b`, or `a != b`, as `a === b` and `a !== b`: of what either
/// answers the same, one side never `null` or `undefined`.
pub(crate) fn strict_equality(e: Expr) -> Expr {
    match e.kind {
        ExprKind::Binary(Op::LooseEq, a, b) => Expr::bin(Op::Eq, *a, *b),
        ExprKind::Binary(Op::LooseNe, a, b) => Expr::bin(Op::Ne, *a, *b),
        kind => Expr { kind, ..e },
    }
}

/// `let list = [];` that nothing sets again, only changes in place, is
/// `const list = [];`, as JS writes it (ADR 0278). A variable a `&mut`'s
/// handle sets is set.
pub(crate) fn constants(body: &mut Vec<Stmt>) {
    let mut set = HashSet::new();
    each_block_mut(body, &mut |stmts| {
        for stmt in stmts.iter() {
            if let StmtKind::Assign(target, _) = &stmt.kind {
                set.extend(var_of(target));
            }
        }
    });
    each_expr_mut(body, &mut |e| {
        if let ExprKind::Handle(place) | ExprKind::Pair(place, _) = &e.kind {
            set.extend(var_of(place));
        }
    });
    each_block_mut(body, &mut |stmts| {
        for stmt in stmts.iter_mut() {
            if let StmtKind::Let(name, Some(value)) = &stmt.kind
                && !set.contains(name)
            {
                stmt.kind = StmtKind::Const(name.clone(), value.clone());
            }
        }
    });
}

/// The variable `place` is, if it's one.
fn var_of(place: &Expr) -> Option<String> {
    match &place.kind {
        ExprKind::Var(name) => Some(name.clone()),
        _ => None,
    }
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
            StmtKind::While { cond, body, .. } => {
                cond.visit_vars(read);
                visit_stmts(body, read);
            }
            StmtKind::ForOf { iterable, body, .. } => {
                iterable.visit_vars(read);
                visit_stmts(body, read);
            }
            StmtKind::For { start, test, body, .. } => {
                start.visit_vars(read);
                test.visit_vars(read);
                visit_stmts(body, read);
            }
            StmtKind::Labeled(_, body) => visit_stmts(body, read),
            StmtKind::Switch {
                discriminant,
                cases,
                default,
            } => {
                discriminant.visit_vars(read);
                for (values, body) in cases {
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
            StmtKind::Break(_) | StmtKind::Continue(_) | StmtKind::Directive(_) => {}
        }
    }
}

/// Whether running `stmt` never goes on to what follows it.
pub fn ends(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Return(_) | StmtKind::Throw(_) | StmtKind::Break(_) | StmtKind::Continue(_) => true,
        StmtKind::If(_, then, Some(otherwise)) => then.last().is_some_and(ends) && otherwise.last().is_some_and(ends),
        // A `switch` with a `default`, each of whose cases leaves (ADR 0660).
        StmtKind::Switch {
            cases,
            default: default @ Some(_),
            ..
        } => case_lists(cases, default).all(|body| body.last().is_some_and(ends)),
        StmtKind::While { label, cond, body } => {
            matches!(cond.kind, ExprKind::Bool(true)) && !breaks(body, label.as_deref())
        }
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
            StmtKind::Assign(target, value) => !target.may_throw() && !value.may_throw(),
            StmtKind::Let(_, None) => true,
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
/// written and whatever it names what it declares: the drops a panic runs
/// and the scope's end runs, `for (const item$1 of v)` and `for (const item
/// of v)`, say.
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
            StmtKind::While { body, .. } | StmtKind::Labeled(_, body) => rename_binders(body, rename),
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
        StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
            label.is_some_and(|l| names_break(body, l))
        }
        StmtKind::Switch { cases, default, .. } => {
            label.is_some_and(|l| case_lists(cases, default).any(|body| names_break(body, l)))
        }
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => breaks(a, label) || breaks(b, label),
        _ => false,
    })
}

/// Whether a `break` in `stmts`, however deep, names `label`.
pub fn names_break(stmts: &[Stmt], label: &str) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Break(Some(l)) => l == label,
        StmtKind::If(_, a, b) => names_break(a, label) || b.as_deref().is_some_and(|b| names_break(b, label)),
        StmtKind::While { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Labeled(_, body) => names_break(body, label),
        StmtKind::Switch { cases, default, .. } => case_lists(cases, default).any(|body| names_break(body, label)),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => names_break(a, label) || names_break(b, label),
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
        // The items each reads when it's loaded.
        let reads: Vec<HashSet<usize>> = (items.iter().enumerate())
            .map(|(i, item)| {
                let mut seen = HashSet::new();
                let mut work = Vec::new();
                match item {
                    Item::Const(constant) => constant.value.visit_vars(&mut |v| work.push(v)),
                    Item::Statements(stmts) => visit_stmts(stmts, &mut |v| work.push(v)),
                    Item::Namespace(_) | Item::Function(_) => {}
                }
                while let Some(name) = work.pop() {
                    if seen.insert(name)
                        && let Some(item) = called.get(name)
                    {
                        item.visit_vars(&mut |v| work.push(v));
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

/// Each expression in `items`, outermost first, as `each_expr_mut` goes.
pub fn each_item_expr_mut(items: &mut [Item], f: &mut dyn FnMut(&mut Expr)) {
    for item in items {
        match item {
            Item::Const(constant) => constant.value.each_mut(f),
            Item::Statements(stmts) => each_expr_mut(stmts, f),
            Item::Namespace(_) | Item::Function(_) => item
                .functions_mut()
                .for_each(|function| each_expr_mut(&mut function.body, f)),
        }
    }
}

/// Each list of statements in `items`, as `each_block_mut` goes: what a
/// module runs when it's loaded, its functions' and methods' bodies, and
/// each closure's, a constant's too.
pub fn each_item_block_mut(items: &mut [Item], f: &mut dyn FnMut(&mut Vec<Stmt>)) {
    for item in items {
        match item {
            Item::Const(constant) => each_block_in_mut(&mut constant.value, f),
            Item::Statements(stmts) => each_block_mut(stmts, f),
            Item::Namespace(_) | Item::Function(_) => item
                .functions_mut()
                .for_each(|function| each_block_mut(&mut function.body, f)),
        }
    }
}

/// A `switch`'s lists of statements: each case's, then its default's.
pub fn case_lists<'s>(
    cases: &'s [(Vec<Expr>, Vec<Stmt>)],
    default: &'s Option<Vec<Stmt>>,
) -> impl Iterator<Item = &'s Vec<Stmt>> {
    cases.iter().map(|(_, body)| body).chain(default)
}

/// A `switch`'s lists of statements, to change: each case's, then its
/// default's.
pub fn case_lists_mut<'s>(
    cases: &'s mut [(Vec<Expr>, Vec<Stmt>)],
    default: &'s mut Option<Vec<Stmt>>,
) -> impl Iterator<Item = &'s mut Vec<Stmt>> {
    cases.iter_mut().map(|(_, body)| body).chain(default.as_mut())
}
