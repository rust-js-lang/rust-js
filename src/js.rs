//! A tiny JavaScript AST: exactly the constructs rust-js emits.
//!
//! `lower.rs` builds it; `prepare.rs` prepares readable JSX; `to_oxc.rs`
//! converts it to oxc's AST, which prints
//! it (with correct parentheses) and builds the source map. Keeping our own
//! small tree means the lowering never touches oxc's large, fast-changing API.
//!
//! Every node carries a `Span`: byte offsets into the Rust source file. That
//! is what lets the source map point from JS back to Rust.

use std::collections::{BTreeSet, HashMap, HashSet};

/// Byte offsets `lo..hi` into the Rust source file. `Span::NONE` (empty)
/// means "no mapping": oxc skips empty spans when building the source map.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Span {
    pub lo: u32,
    pub hi: u32,
}

impl Span {
    pub const NONE: Span = Span { lo: 0, hi: 0 };

    pub fn is_none(self) -> bool {
        self.lo == self.hi
    }
}

/// The field a box, a cell or a handle holds the place it stands for in,
/// `b.value` (ADR 0099): a `&mut` to what JS can't change in place, a
/// `Cell`'s, an `Rc`'s, a guard's.
pub const HELD: &str = "value";

/// The field a `dyn` value's pair holds its dictionary in, `d.impl` (ADR
/// 0049), beside its `value`.
pub const DICTIONARY: &str = "impl";

/// The function of a `dyn`'s dictionary that drops what it holds, `$drop`,
/// if that has a destructor.
pub const DICTIONARY_DROP: &str = "$drop";

pub struct Module {
    pub header: String,
    /// `"use client"`, before its imports (ADR 0192).
    pub directives: Vec<String>,
    /// Imports from JS modules, from `#[link_name = "module#path"]` (ADR 0028).
    pub packages: Vec<Package>,
    /// Named imports, one declaration per Rust module this one uses.
    pub imports: Vec<Import>,
    /// What it exports of another module of the crate's, `export { a } from
    /// "./b.js"`, its `pub use` (ADR 0240).
    pub reexports: Vec<Import>,
    /// What it imports of `@rust-js/runtime`, sorted: the helpers its
    /// prepared tree reads (ADR 0103), chosen before it's printed.
    pub helpers: Vec<&'static str>,
    /// Its items, where its Rust has them, what's made when it's loaded
    /// after what that reads (ADR 0306).
    pub items: Vec<Item>,
    /// Lazy trait dictionary caches. `var` without an initializer is cycle-safe.
    pub caches: Vec<String>,
    /// `export default page;`, after its functions (ADR 0192).
    pub default_export: Option<String>,
}

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
        StmtKind::While { label, cond, body } => {
            matches!(cond.kind, ExprKind::Bool(true)) && !breaks(body, label.as_deref())
        }
        _ => false,
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
        // In a loop of its own, one that names it.
        StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
            label.is_some_and(|l| names_break(body, l))
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
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => names_break(a, label) || names_break(b, label),
        _ => false,
    })
}

/// A module's item.
pub enum Item {
    /// A type's methods, before what's made when it's loaded calls them.
    Namespace(Namespace),
    /// A `const`, with the value rustc computed (ADR 0031), or a
    /// thread-local's (ADR 0037).
    Const(Const),
    /// What it runs when it's loaded, one `js::on_load!`'s (ADR 0267).
    Statements(Vec<Stmt>),
    Function(Function),
}

impl Item {
    /// Its functions: itself, or a type's methods.
    pub fn functions_mut(&mut self) -> impl Iterator<Item = &mut Function> {
        let (function, methods) = match self {
            Item::Function(function) => (Some(function), None),
            Item::Namespace(namespace) => (None, Some(namespace.methods.iter_mut())),
            Item::Const(_) | Item::Statements(_) => (None, None),
        };
        function.into_iter().chain(methods.into_iter().flatten())
    }

    /// Each variable it reads, its functions' bodies' too.
    pub fn visit_vars<'a>(&'a self, read: &mut dyn FnMut(&'a str)) {
        match self {
            Item::Namespace(namespace) => namespace.methods.iter().for_each(|m| visit_stmts(&m.body, read)),
            Item::Const(constant) => constant.value.visit_vars(read),
            Item::Statements(stmts) => visit_stmts(stmts, read),
            Item::Function(function) => visit_stmts(&function.body, read),
        }
    }

    /// Where its Rust is, of what's in the crate's sources.
    fn spans(&self) -> Vec<Span> {
        let spans: Vec<Span> = match self {
            Item::Namespace(namespace) => namespace.methods.iter().map(|m| m.span).collect(),
            Item::Const(constant) => vec![constant.span],
            Item::Statements(stmts) => stmts.iter().map(|s| s.span).collect(),
            Item::Function(function) => vec![function.span],
        };
        spans.into_iter().filter(|span| !span.is_none()).collect()
    }

    /// The variable it declares at its module's top that another item can
    /// read: a `const`'s, a type's.
    pub fn declared(&self) -> Option<&str> {
        match self {
            Item::Namespace(Namespace { name, .. }) | Item::Const(Const { name, .. }) => Some(name),
            Item::Statements(_) | Item::Function(_) => None,
        }
    }
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

/// A type's methods (ADR 0047), an object named after the type:
/// `export const Counter = { new(step) { .. }, tick(counter) { .. } };`.
pub struct Namespace {
    pub name: String,
    /// Its methods, as functions: the object's properties.
    pub methods: Vec<Function>,
    pub export: bool,
}

/// `const SIZE = 4096;`, maybe exported.
pub struct Const {
    pub name: String,
    pub value: Expr,
    pub export: bool,
    /// `let`: a thread-local its module sets (ADR 0270).
    pub mutable: bool,
    /// The whole `const` item.
    pub span: Span,
}

pub struct Import {
    /// Exports as `(export, local)`, aliased only for name collisions.
    pub named: Vec<(String, String)>,
    /// A relative specifier, like `./math.js` or `../lib.js`.
    pub from: String,
}

/// Where a dynamic `import()` loads from (ADR 0304): a specifier, or a
/// module of the crate's by its path, until its file's specifier is known.
#[derive(Clone)]
pub enum ImportFrom {
    Specifier(String),
    Module(Vec<String>),
}

/// What one file imports from one JS module: its default export, named
/// exports as `(export, local)`, and the module itself.
pub struct Package {
    pub from: String,
    pub default: Option<String>,
    pub named: Vec<(String, String)>,
    pub namespace: Option<String>,
}

#[derive(Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Pattern>,
    pub body: Vec<Stmt>,
    pub export: bool,
    /// `async function`: an `async fn` (ADR 0029).
    pub is_async: bool,
    /// The whole `fn` item, and just its name.
    pub span: Span,
    pub name_span: Span,
}

/// What a parameter or declaration binds: a variable, or the parts of an
/// array or object, `[count, setCount]` or `{ initial, label }`.
#[derive(Clone)]
pub enum Pattern {
    Name(String),
    /// `None` skips an element: `[, b]`. An element may be taken apart in
    /// turn, `[i, { name }]`.
    Array(Vec<Option<Pattern>>),
    /// Each field, what it goes in, a variable or a pattern of its own, and
    /// its default where it's missing, `{ size = "md" }` (ADR 0212), a
    /// literal; and the variable the rest go in, `...rest` (ADR 0195).
    Object(Vec<(String, Pattern, Option<Expr>)>, Option<String>),
}

impl Pattern {
    /// An array of variables, `[count, setCount]`; `None` skips one.
    pub fn names_array(names: impl IntoIterator<Item = Option<String>>) -> Pattern {
        Pattern::Array(names.into_iter().map(|name| name.map(Pattern::Name)).collect())
    }

    /// The variables it binds.
    pub fn names(&self) -> Vec<&str> {
        match self {
            Pattern::Name(name) => vec![name.as_str()],
            Pattern::Array(items) => items.iter().flatten().flat_map(Pattern::names).collect(),
            Pattern::Object(fields, rest) => fields
                .iter()
                .flat_map(|(_, p, _)| p.names())
                .chain(rest.as_deref())
                .collect(),
        }
    }

    /// The variables it binds, to rename.
    pub fn names_mut(&mut self) -> Vec<&mut String> {
        match self {
            Pattern::Name(name) => vec![name],
            Pattern::Array(items) => items.iter_mut().flatten().flat_map(Pattern::names_mut).collect(),
            Pattern::Object(fields, rest) => fields
                .iter_mut()
                .flat_map(|(_, p, _)| p.names_mut())
                .chain(rest.as_mut())
                .collect(),
        }
    }

    /// Its defaults, `"md"` of `{ size = "md" }`, in order.
    pub fn defaults(&self) -> Vec<&Expr> {
        match self {
            Pattern::Name(_) => Vec::new(),
            Pattern::Array(items) => items.iter().flatten().flat_map(Pattern::defaults).collect(),
            Pattern::Object(fields, _) => fields
                .iter()
                .flat_map(|(_, p, default)| default.iter().chain(p.defaults()))
                .collect(),
        }
    }

    /// The variable it is, if it's one.
    pub fn name(&self) -> Option<&str> {
        match self {
            Pattern::Name(name) => Some(name),
            _ => None,
        }
    }
}

impl From<String> for Pattern {
    fn from(name: String) -> Pattern {
        Pattern::Name(name)
    }
}

impl From<&str> for Pattern {
    fn from(name: &str) -> Pattern {
        Pattern::Name(name.to_string())
    }
}

#[derive(Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone)]
pub enum StmtKind {
    Const(String, Expr),
    Let(String, Option<Expr>),
    /// `const [a, b] = value;`, or `let` if one of them is reassigned.
    Destructure {
        pattern: Pattern,
        value: Expr,
        mutable: bool,
    },
    /// `target = value`, where `target` is a variable, `a.b` or `a[0]`.
    Assign(Expr, Expr),
    /// `target op= value`, `s += t`: what preparation makes of
    /// `target = target op value`, as JS writes a string built up.
    Compound(Op, Expr, Expr),
    Expr(Expr),
    If(Expr, Vec<Stmt>, Option<Vec<Stmt>>),
    While {
        label: Option<String>,
        cond: Expr,
        body: Vec<Stmt>,
    },
    /// `for (const name of iterable) { .. }`: a `for` over a sequence (ADR 0025),
    /// and `for (const [i, x] of ..)` for a tuple's parts. `let` if one's `mut`.
    ForOf {
        label: Option<String>,
        pattern: Pattern,
        mutable: bool,
        iterable: Expr,
        body: Vec<Stmt>,
    },
    /// `for (let name = start; test; name++) { .. }`: a `for` over a range.
    For {
        label: Option<String>,
        name: String,
        start: Expr,
        test: Expr,
        body: Vec<Stmt>,
    },
    /// `label: { .. }`, which a `break label` leaves: a let chain's (ADR 0048).
    Labeled(String, Vec<Stmt>),
    /// `try { .. } finally { .. }`: a scope, and the drops that end it,
    /// however it's left (ADR 0098).
    Try(Vec<Stmt>, Vec<Stmt>),
    /// `try { .. } catch (error) { .. }`: a writer's `fmt::Error`, given
    /// what it wrote, or taken as a `fmt::Result` (ADR 0187); or `catch {
    /// .. }`, what a JS call threw, unread (ADR 0035).
    TryCatch(Vec<Stmt>, Option<String>, Vec<Stmt>),
    Break(Option<String>),
    Continue(Option<String>),
    /// `function name(..) { .. }`: a function written in a body (ADR 0308).
    Function(Box<Function>),
    Return(Option<Expr>),
    /// `throw new Error(..)`: a panic (ADR 0012).
    Throw(Expr),
    /// `"use cache";`: a function's or an arrow's directive, first in its body
    /// (ADR 0349).
    Directive(String),
}

impl StmtKind {
    pub fn at(self, span: Span) -> Stmt {
        Stmt { kind: self, span }
    }
}

#[derive(Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
    /// A call of a method of a `str` or a slice, as Rust's types tell,
    /// `s.startsWith(t)`: never a crate's or a binding's of the same
    /// name, `P.at(p)`, which may do anything.
    pub reads: bool,
}

/// A cross-module reference, resolved before preparation or printing.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Symbol {
    pub module: u32,
    pub export: String,
}

#[derive(Clone)]
pub enum ExprKind {
    Num(f64),
    /// A `u64` or an `i64` (ADR 0086): `5n`.
    BigInt(i128),
    /// A `u128` past `i128::MAX`, which `BigInt` can't hold (ADR 0171).
    BigUint(u128),
    Bool(bool),
    Str(String),
    /// A string written across lines in its source, `r#"<div>` and on: a
    /// template literal with its line breaks, as a person writes a page.
    Lines(String),
    Undefined,
    /// `undefined` given to JS, a binding or a JS function, which no pass
    /// leaves out of a call: what it calls may count its arguments,
    /// `Math.max(1, undefined)` (ADR 0330).
    GivenUndefined,
    /// Only to test against: `o != null` (ADR 0030).
    Null,
    Var(String),
    Symbol(Symbol),
    /// `object.property`, e.g. `Math.imul` or `math.add`.
    Member(Box<Expr>, String),
    /// `a?.b`: `undefined` where `a` is `undefined` or `null`.
    OptionalMember(Box<Expr>, String),
    /// `object[index]`, e.g. `pair[0]`.
    Index(Box<Expr>, Box<Expr>),
    /// `String(x)` of a Rust number, which a template writes as `x`: it
    /// makes it text as `String` does. Not of any value, a symbol say,
    /// which a template throws on (ADR 0066).
    Stringed(Box<Expr>),
    /// `a?.[i]`: `undefined` where `a` is `undefined` or `null`.
    OptionalIndex(Box<Expr>, Box<Expr>),
    /// `[a, b]`: a tuple or tuple struct (ADR 0020).
    Array(Vec<Expr>),
    /// `...a`, an array's item only: `[...path, last]`, a slice's `concat`.
    Spread(Box<Expr>),
    /// `{ x: a, y: b }`: a struct (ADR 0020).
    Object(Vec<Prop>),
    Unary(UnaryOp, Box<Expr>),
    Binary(Op, Box<Expr>, Box<Expr>),
    Cond(Box<Expr>, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    /// `f?.(args)`: a call of what may be `undefined`, as a drop function
    /// a caller with nothing to drop leaves out (ADR 0098).
    OptionalCall(Box<Expr>, Vec<Expr>),
    /// `new Event(t)`: a JS constructor (ADR 0024).
    New(Box<Expr>, Vec<Expr>),
    /// `(a, b) => { .. }`: a closure (ADR 0022).
    Arrow(Vec<Pattern>, Vec<Stmt>),
    /// `async (a) => { .. }`: an async closure or block (ADR 0029).
    AsyncArrow(Vec<Pattern>, Vec<Stmt>),
    /// `function Label(props) { .. }`: a function a block makes and gives,
    /// named, as an expression (ADR 0296).
    Function(Box<Function>),
    /// Where such a function goes until the pipeline puts it there: its
    /// item's index.
    FunctionHole(u32),
    /// `import("./lint.js")`: the module, loaded when it's asked for (ADR 0304).
    Import(ImportFrom),
    /// A drop function given to one of the crate's functions for its type
    /// parameter, `(item, index, drop)`: until the pipeline keeps it, where
    /// that function uses it, or leaves it out (ADR 0300).
    DropArgument(u32, u32, Box<Expr>),
    /// `await p`: `.await` (ADR 0029).
    Await(Box<Expr>),
    /// `<div className="hero">..</div>`, `<Counter initial={1} />` or `<>..</>` (ADR 0040).
    Jsx(Box<Jsx>),
    /// A regular expression literal, as written: `/^\p{White_Space}$/u` (ADR 0063).
    Regex(String),
    /// `\`Some(${x})\``: the texts around the values, as they read (one more
    /// than the values), and the values (ADR 0066); and whether a line break
    /// in its texts is one, where its format string was written across lines.
    Template(Vec<String>, Vec<Expr>, bool),
    /// `{ get value() { return x; }, set value(value) { x = value; } }`: a
    /// `&mut` kept, reading and writing its place each time (ADR 0099). The
    /// place is what's read then, not before: it's never taken out.
    Handle(Box<Expr>),
    /// `{ impl: d, get value() { return x; }, set value(value) { x = value; } }`:
    /// a `&mut dyn Trait` of a value JS can't change in place, its impl's
    /// dictionary and a handle on its place (ADR 0099).
    Pair(Box<Expr>, Box<Expr>),
}

/// A JSX element (ADR 0040).
#[derive(Clone)]
pub struct Jsx {
    pub tag: JsxTag,
    /// Its attributes, `className="hero"`, or `{...props}`.
    pub props: Vec<Prop>,
    pub children: Vec<Expr>,
}

#[derive(Clone)]
pub enum JsxTag {
    /// `<>`.
    Fragment,
    /// A DOM element: `<div>`.
    Intrinsic(String),
    /// A component: `<Counter>`, `<StrictMode>`, `<stats.Chart>`.
    Component(Expr),
}

#[derive(Clone)]
pub enum Prop {
    /// `name: value`, printed as `name` when `value` is a variable of that name.
    Field(String, Expr),
    /// `...value`: copy every field of `value`.
    Spread(Expr),
    /// `get name() { .. }`, of an arrow that takes nothing: its body runs
    /// on each read, as a Rust constant's value is a new one at each use.
    Getter(String, Expr),
    /// `set name(v) { .. }`, of an arrow that takes one: a handle's on a
    /// place, which preparation writes of `Handle` (ADR 0099).
    Setter(String, Expr),
}

#[derive(Clone, Copy)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
    /// `typeof x`: what an untagged enum's variant is told by (ADR 0214).
    Typeof,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Op {
    Or,
    And,
    /// `a ?? b`: `unwrap_or` (ADR 0030).
    Coalesce,
    BitOr,
    BitXor,
    BitAnd,
    Eq,
    Ne,
    /// `==` and `!=`: only against `null`, for `None` (ADR 0030), and on
    /// options, where `null` and `undefined` are both `None`.
    LooseEq,
    LooseNe,
    /// `x instanceof C`: a `#[link_name = "instanceof C"]` binding.
    InstanceOf,
    /// `key in x`: a `#[link_name = "in []"]` binding.
    In,
    Lt,
    Le,
    Gt,
    Ge,
    Shl,
    Shr,
    UShr,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    /// `a ** b`: `f64`'s `powf` (ADR 0064).
    Pow,
}

impl Expr {
    fn new(kind: ExprKind) -> Expr {
        Expr {
            kind,
            span: Span::NONE,
            reads: false,
        }
    }

    /// `of.method(args)`, of a `str` or a slice `of` as Rust's types tell.
    pub fn read(of: Expr, method: &str, args: Vec<Expr>) -> Expr {
        Expr {
            reads: true,
            ..Expr::call(Expr::member(of, method), args)
        }
    }

    pub fn num(n: impl Into<f64>) -> Expr {
        Expr::new(ExprKind::Num(n.into()))
    }

    pub fn int(n: i128) -> Expr {
        // A number-represented integer fits in 32 bits, so this is exact.
        Expr::num(n as f64)
    }

    pub fn bigint(n: i128) -> Expr {
        Expr::new(ExprKind::BigInt(n))
    }

    /// A `u128`'s value, `BigInt` where it fits.
    pub fn biguint(n: u128) -> Expr {
        match i128::try_from(n) {
            Ok(n) => Expr::bigint(n),
            Err(_) => Expr::new(ExprKind::BigUint(n)),
        }
    }

    pub fn bool(b: bool) -> Expr {
        Expr::new(ExprKind::Bool(b))
    }

    pub fn str(s: impl Into<String>) -> Expr {
        Expr::new(ExprKind::Str(s.into()))
    }

    pub fn lines(s: impl Into<String>) -> Expr {
        Expr::new(ExprKind::Lines(s.into()))
    }

    /// A regular expression literal, as written: `/^[0-9]$/`.
    pub fn regex(literal: &str) -> Expr {
        Expr::new(ExprKind::Regex(literal.to_string()))
    }

    pub fn template(texts: Vec<String>, values: Vec<Expr>) -> Expr {
        debug_assert_eq!(texts.len(), values.len() + 1);
        Expr::new(ExprKind::Template(texts, values, false))
    }

    /// A template written across `lines`: its line breaks written as they are.
    pub fn with_lines(self, lines: bool) -> Expr {
        match self.kind {
            ExprKind::Template(texts, values, _) => Expr::new(ExprKind::Template(texts, values, lines)),
            _ => self,
        }
    }

    pub fn undefined() -> Expr {
        Expr::new(ExprKind::Undefined)
    }

    pub fn null() -> Expr {
        Expr::new(ExprKind::Null)
    }

    pub fn var(name: &str) -> Expr {
        Expr::new(ExprKind::Var(name.to_string()))
    }

    /// The runtime's helper `name` exports, `$cmp`: asked for by being
    /// written, as the linker asks for each its module's tree reads. A name
    /// the runtime doesn't export is a bug of rust-js's. No other variable
    /// is named so: Rust's names have no `$`, and the compiler's own are
    /// named apart from the runtime's (`names::dictionary_cache`).
    pub fn helper(name: &'static str) -> Expr {
        assert!(crate::runtime::exports(name), "the runtime exports no `{name}`");
        Expr::var(name)
    }

    /// The runtime's helper this is, by the name it exports (ADR 0103).
    pub fn helper_name(&self) -> Option<&str> {
        match &self.kind {
            ExprKind::Var(name) if name.starts_with('$') && crate::runtime::exports(name) => Some(name),
            _ => None,
        }
    }

    /// A call of the runtime's helper `name`, `$cmp(a, b)`.
    pub fn call_helper(name: &'static str, args: Vec<Expr>) -> Expr {
        Expr::call(Expr::helper(name), args)
    }

    pub fn drop_argument(item: u32, index: u32, drop: Expr) -> Expr {
        Expr::new(ExprKind::DropArgument(item, index, Box::new(drop)))
    }

    pub fn import(from: ImportFrom) -> Expr {
        Expr::new(ExprKind::Import(from))
    }

    pub fn handle(place: Expr) -> Expr {
        Expr::new(ExprKind::Handle(Box::new(place)))
    }

    pub fn pair(place: Expr, dictionary: Expr) -> Expr {
        Expr::new(ExprKind::Pair(Box::new(place), Box::new(dictionary)))
    }

    /// What a box, a cell or a handle holds: `b.value` (`HELD`).
    pub fn held(of: Expr) -> Expr {
        Expr::member(of, HELD)
    }

    pub fn member(object: Expr, property: impl Into<String>) -> Expr {
        let property = property.into();
        // `{ x: 0, y: 0 }.y`, a constant's field (`V::ZERO.y`), is `0`.
        if let ExprKind::Object(props) = &object.kind
            && props
                .iter()
                .all(|p| matches!(p, Prop::Field(_, value) if value.is_constant()))
            && let Some(Prop::Field(_, value)) = props
                .iter()
                .find(|p| matches!(p, Prop::Field(name, _) if *name == property))
        {
            return value.clone();
        }
        // A handle's `value` is the place it's on (ADR 0099), and a cell
        // made here, `{ value: x }`, holds `x`: a cell in a field is the
        // property itself (ADR 0288).
        if property == HELD {
            if let ExprKind::Handle(place) = object.kind {
                return *place;
            }
            if let ExprKind::Object(props) = &object.kind
                && let [Prop::Field(name, value)] = props.as_slice()
                && name == HELD
            {
                return value.clone();
            }
        }
        Expr::new(ExprKind::Member(Box::new(object), property))
    }

    /// `object?.property`.
    pub fn optional_member(object: Expr, property: impl Into<String>) -> Expr {
        Expr::new(ExprKind::OptionalMember(Box::new(object), property.into()))
    }

    /// `String(x)` of a Rust number, `x` in a template (ADR 0066).
    pub fn stringed(value: Expr) -> Expr {
        Expr::new(ExprKind::Stringed(Box::new(value)))
    }

    pub fn optional_index(object: Expr, index: Expr) -> Expr {
        Expr::new(ExprKind::OptionalIndex(Box::new(object), Box::new(index)))
    }

    pub fn index(object: Expr, index: Expr) -> Expr {
        // `[a, b][1]` is `b`, where none is `...items`, and what's left
        // unread does nothing.
        if let (ExprKind::Array(items), Some(k)) = (&object.kind, index.as_int())
            && let Some(item) = usize::try_from(k).ok().and_then(|k| items.get(k))
            && !items.iter().any(|i| matches!(i.kind, ExprKind::Spread(_)))
            && items.iter().all(|i| std::ptr::eq(i, item) || !i.has_effects())
        {
            return item.clone();
        }
        Expr::new(ExprKind::Index(Box::new(object), Box::new(index)))
    }

    pub fn array(items: Vec<Expr>) -> Expr {
        Expr::new(ExprKind::Array(items))
    }

    /// `...items`, as an array's item or a call's argument.
    pub fn spread(items: Expr) -> Expr {
        Expr::new(ExprKind::Spread(Box::new(items)))
    }

    pub fn object(props: Vec<Prop>) -> Expr {
        Expr::new(ExprKind::Object(props))
    }

    /// The test that holds where this one doesn't, read only for its truth:
    /// `!a` of `a`, `a` of `!a`, `a !== b` of `a === b`, and of a chain, its
    /// parts', `!a || b !== c` of `a && b === c`. The one negation of a test.
    pub fn negated_test(self) -> Expr {
        match self.kind {
            ExprKind::Binary(op @ (Op::And | Op::Or), a, b) => {
                let flipped = if op == Op::And { Op::Or } else { Op::And };
                Expr::bin(flipped, a.negated_test(), b.negated_test())
            }
            ExprKind::Unary(UnaryOp::Not, inner) => *inner,
            kind => Expr::unary(
                UnaryOp::Not,
                Expr {
                    reads: false,
                    kind,
                    span: self.span,
                },
            ),
        }
    }

    pub fn unary(op: UnaryOp, arg: Expr) -> Expr {
        // `!!!a` is `!a`, for every `a`: the inner two make a `bool` of it.
        if let (UnaryOp::Not, ExprKind::Unary(UnaryOp::Not, inner)) = (op, &arg.kind)
            && let ExprKind::Unary(UnaryOp::Not, value) = &inner.kind
        {
            return Expr::unary(UnaryOp::Not, (**value).clone());
        }
        // `!(a === b)` is `a !== b`, for every `a` and `b`.
        if let (UnaryOp::Not, ExprKind::Binary(eq @ (Op::Eq | Op::Ne | Op::LooseEq | Op::LooseNe), a, b)) =
            (op, &arg.kind)
        {
            let ne = match eq {
                Op::Eq => Op::Ne,
                Op::Ne => Op::Eq,
                Op::LooseEq => Op::LooseNe,
                _ => Op::LooseEq,
            };
            return Expr {
                reads: false,
                kind: ExprKind::Binary(ne, a.clone(), b.clone()),
                span: arg.span,
            };
        }
        Expr::new(ExprKind::Unary(op, Box::new(arg)))
    }

    pub fn bin(op: Op, lhs: Expr, rhs: Expr) -> Expr {
        // `a != null && !!a` is `!!a`: neither is truthy.
        if op == Op::And
            && let Some(tested) = null_test(&lhs, true)
            && let ExprKind::Unary(UnaryOp::Not, inner) = &rhs.kind
            && let ExprKind::Unary(UnaryOp::Not, of) = &inner.kind
            && same_path(tested, of)
        {
            return rhs;
        }
        // `a != null && a === "index"` is `a === "index"`: no literal is
        // `null` or `undefined`.
        if op == Op::And
            && let Some(tested) = null_test(&lhs, true)
            && let ExprKind::Binary(Op::Eq, a, b) = &rhs.kind
            && [(a, b), (b, a)].iter().any(|(x, literal)| {
                same_path(tested, x) && matches!(literal.kind, ExprKind::Str(_) | ExprKind::Bool(_) | ExprKind::Num(_))
            })
        {
            return rhs;
        }
        // `a != null && Array.isArray(a)` is `Array.isArray(a)`: no array is
        // `null` or `undefined`; and of `Array.isArray(a) && ..`, that.
        let mut first = &rhs;
        while let ExprKind::Binary(Op::And, a, _) = &first.kind {
            first = a;
        }
        if op == Op::And
            && let Some(tested) = presence_of(&lhs)
            && let ExprKind::Call(callee, args) = &first.kind
            && let (ExprKind::Member(array, is_array), [of]) = (&callee.kind, args.as_slice())
            && matches!(&array.kind, ExprKind::Var(name) if name == "Array")
            && is_array == "isArray"
            && same_path(tested, of)
        {
            return rhs;
        }
        // `x + "" + "px"` is `x + "px"`: either makes text of `x` as `+`
        // does, then joins.
        if op == Op::Add
            && matches!(rhs.kind, ExprKind::Str(_) | ExprKind::Template(..))
            && let ExprKind::Binary(Op::Add, x, empty) = &lhs.kind
            && matches!(&empty.kind, ExprKind::Str(s) if s.is_empty())
        {
            return Expr::bin(Op::Add, (**x).clone(), rhs);
        }
        Expr::new(ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)))
    }

    pub fn cond(test: Expr, then: Expr, els: Expr) -> Expr {
        // `a != null ? a.b : undefined` is `a?.b`, as a person writes it. One
        // property only: `a?.b.c` would end the chain at `.c` too. Not of
        // `!!a`, which an empty string fails, `!s.is_empty()`'s: an
        // `Option` never falsy's is lowered as one of `a != null` (ADR 0298).
        if let Some(tested) = null_test(&test, true)
            && matches!(els.kind, ExprKind::Undefined)
            && let ExprKind::Member(object, property) = &then.kind
            && same_path(tested, object)
        {
            let chain = Expr::optional_member((**object).clone(), property.clone());
            return Expr {
                span: then.span,
                ..chain
            };
        }
        // `a != null ? a.m(x) : undefined` is `a?.m(x)`: `?.` skips the call,
        // its arguments too, where `a` is, as the test does.
        if let Some(tested) = null_test(&test, true)
            && matches!(els.kind, ExprKind::Undefined)
            && let ExprKind::Call(callee, args) = &then.kind
            && let ExprKind::Member(object, property) = &callee.kind
            && same_path(tested, object)
        {
            let chain = Expr::call(
                Expr::optional_member((**object).clone(), property.clone()),
                args.clone(),
            );
            return Expr {
                span: then.span,
                ..chain
            };
        }
        Expr::new(ExprKind::Cond(Box::new(test), Box::new(then), Box::new(els)))
    }

    pub fn new_(callee: Expr, args: Vec<Expr>) -> Expr {
        Expr::new(ExprKind::New(Box::new(callee), args))
    }

    pub fn arrow(params: Vec<Pattern>, body: Vec<Stmt>) -> Expr {
        Expr::new(ExprKind::Arrow(params, body))
    }

    pub fn async_arrow(params: Vec<Pattern>, body: Vec<Stmt>) -> Expr {
        Expr::new(ExprKind::AsyncArrow(params, body))
    }

    pub fn await_(promise: Expr) -> Expr {
        Expr::new(ExprKind::Await(Box::new(promise)))
    }

    pub fn jsx(jsx: Jsx) -> Expr {
        Expr::new(ExprKind::Jsx(Box::new(jsx)))
    }

    pub fn call(callee: Expr, args: Vec<Expr>) -> Expr {
        Expr::new(ExprKind::Call(Box::new(callee), args))
    }

    /// Give this node a span, unless it already has one. Lowering calls
    /// this on every result, so the outermost node made for a Rust
    /// expression gets that expression's span.
    pub fn or_at(mut self, span: Span) -> Expr {
        if self.span.is_none() {
            self.span = span;
        }
        self
    }

    /// The integer value, if this is an integer literal.
    pub fn as_int(&self) -> Option<i128> {
        match self.kind {
            ExprKind::Num(n) if n.fract() == 0.0 => Some(n as i128),
            _ => None,
        }
    }

    pub fn as_bigint(&self) -> Option<i128> {
        match self.kind {
            ExprKind::BigInt(n) => Some(n),
            _ => None,
        }
    }

    /// Literals can be evaluated at any time, so they never need a temporary.
    pub fn is_constant(&self) -> bool {
        matches!(
            self.kind,
            ExprKind::Num(_)
                | ExprKind::BigInt(_)
                | ExprKind::BigUint(_)
                | ExprKind::Bool(_)
                | ExprKind::Str(_)
                | ExprKind::Lines(_)
                | ExprKind::Undefined
                | ExprKind::GivenUndefined
                | ExprKind::Null
        )
    }

    /// A function, an object, an array or a string made here: never `null`
    /// nor `undefined`, as an `Option`'s `Some` of it.
    pub fn is_made_here(&self) -> bool {
        matches!(
            self.kind,
            ExprKind::Arrow(..)
                | ExprKind::AsyncArrow(..)
                | ExprKind::Object(_)
                | ExprKind::Array(_)
                | ExprKind::Template(..)
        )
    }

    /// A constant, or an object made here of them, which nothing made
    /// before it or after can change.
    pub fn is_made_of_constants(&self) -> bool {
        match &self.kind {
            ExprKind::Object(props) => {
                (props.iter()).all(|prop| matches!(prop, Prop::Field(_, value) if value.is_made_of_constants()))
            }
            _ => self.is_constant(),
        }
    }

    /// This, then each expression in it, as `each_expr_mut` goes.
    pub fn each_mut(&mut self, f: &mut dyn FnMut(&mut Expr)) {
        f(self);
        let props = |props: &mut [Prop], f: &mut dyn FnMut(&mut Expr)| {
            for prop in props {
                let (Prop::Field(_, value) | Prop::Spread(value) | Prop::Getter(_, value) | Prop::Setter(_, value)) =
                    prop;
                value.each_mut(f);
            }
        };
        match &mut self.kind {
            ExprKind::Member(a, _)
            | ExprKind::OptionalMember(a, _)
            | ExprKind::Unary(_, a)
            | ExprKind::Await(a)
            | ExprKind::Spread(a)
            | ExprKind::Stringed(a)
            | ExprKind::Handle(a)
            | ExprKind::DropArgument(_, _, a) => a.each_mut(f),
            ExprKind::Index(a, b)
            | ExprKind::OptionalIndex(a, b)
            | ExprKind::Binary(_, a, b)
            | ExprKind::Pair(a, b) => {
                a.each_mut(f);
                b.each_mut(f);
            }
            ExprKind::Cond(a, b, c) => {
                a.each_mut(f);
                b.each_mut(f);
                c.each_mut(f);
            }
            ExprKind::Call(callee, args) | ExprKind::OptionalCall(callee, args) | ExprKind::New(callee, args) => {
                callee.each_mut(f);
                args.iter_mut().for_each(|a| a.each_mut(f));
            }
            ExprKind::Array(items) | ExprKind::Template(_, items, _) => items.iter_mut().for_each(|a| a.each_mut(f)),
            ExprKind::Object(fields) => props(fields, f),
            ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => each_expr_mut(body, f),
            ExprKind::Function(function) => each_expr_mut(&mut function.body, f),
            ExprKind::Jsx(jsx) => {
                if let JsxTag::Component(tag) = &mut jsx.tag {
                    tag.each_mut(f);
                }
                props(&mut jsx.props, f);
                jsx.children.iter_mut().for_each(|c| c.each_mut(f));
            }
            ExprKind::Var(_)
            | ExprKind::Num(_)
            | ExprKind::BigInt(_)
            | ExprKind::BigUint(_)
            | ExprKind::Bool(_)
            | ExprKind::Str(_)
            | ExprKind::Lines(_)
            | ExprKind::Undefined
            | ExprKind::GivenUndefined
            | ExprKind::Null
            | ExprKind::Symbol(_)
            | ExprKind::FunctionHole(_)
            | ExprKind::Import(_)
            | ExprKind::Regex(_) => {}
        }
    }

    /// Each variable this reads, closures' bodies too, but not a string's
    /// or a regular expression's text, nor a property's name.
    pub fn visit_vars<'a>(&'a self, read: &mut dyn FnMut(&'a str)) {
        let props = |props: &'a [Prop], read: &mut dyn FnMut(&'a str)| {
            for prop in props {
                match prop {
                    Prop::Field(_, value) | Prop::Spread(value) | Prop::Getter(_, value) | Prop::Setter(_, value) => {
                        value.visit_vars(read)
                    }
                }
            }
        };
        match &self.kind {
            ExprKind::Var(name) => read(name),
            ExprKind::Member(a, _)
            | ExprKind::OptionalMember(a, _)
            | ExprKind::Unary(_, a)
            | ExprKind::Await(a)
            | ExprKind::Spread(a)
            | ExprKind::Stringed(a)
            | ExprKind::Handle(a)
            | ExprKind::DropArgument(_, _, a) => a.visit_vars(read),
            ExprKind::Index(a, b)
            | ExprKind::OptionalIndex(a, b)
            | ExprKind::Binary(_, a, b)
            | ExprKind::Pair(a, b) => {
                a.visit_vars(read);
                b.visit_vars(read);
            }
            ExprKind::Cond(a, b, c) => {
                a.visit_vars(read);
                b.visit_vars(read);
                c.visit_vars(read);
            }
            ExprKind::Call(f, args) | ExprKind::OptionalCall(f, args) | ExprKind::New(f, args) => {
                f.visit_vars(read);
                args.iter().for_each(|a| a.visit_vars(read));
            }
            ExprKind::Array(items) | ExprKind::Template(_, items, _) => items.iter().for_each(|a| a.visit_vars(read)),
            ExprKind::Object(fields) => props(fields, read),
            ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => visit_stmts(body, read),
            ExprKind::Function(function) => visit_stmts(&function.body, read),
            ExprKind::Jsx(jsx) => {
                if let JsxTag::Component(tag) = &jsx.tag {
                    tag.visit_vars(read);
                }
                props(&jsx.props, read);
                jsx.children.iter().for_each(|c| c.visit_vars(read));
            }
            ExprKind::Num(_)
            | ExprKind::BigInt(_)
            | ExprKind::BigUint(_)
            | ExprKind::Bool(_)
            | ExprKind::Str(_)
            | ExprKind::Lines(_)
            | ExprKind::Undefined
            | ExprKind::GivenUndefined
            | ExprKind::Null
            | ExprKind::Symbol(_)
            | ExprKind::FunctionHole(_)
            | ExprKind::Import(_)
            | ExprKind::Regex(_) => {}
        }
    }

    /// This with each variable that `with` names replaced by its expression:
    /// a closure's parameter by its argument, to put its body in place.
    /// `None` if there's a closure inside, whose own names could shadow them.
    pub fn substitute(&self, with: &dyn Fn(&str) -> Option<Expr>) -> Option<Expr> {
        self.replace(with, false)
    }

    /// `substitute`, going into the closures inside that only return, as
    /// `map`'s callbacks do, if none of their parameters is a name replaced
    /// or one the replacements read. Only for callbacks that run at once: in
    /// one that runs later, a variable would be read later.
    pub fn substitute_in_callbacks(&self, with: &dyn Fn(&str) -> Option<Expr>) -> Option<Expr> {
        self.replace(with, true)
    }

    fn replace(&self, with: &dyn Fn(&str) -> Option<Expr>, callbacks: bool) -> Option<Expr> {
        let all = |items: &[Expr]| {
            items
                .iter()
                .map(|e| e.replace(with, callbacks))
                .collect::<Option<Vec<_>>>()
        };
        let one = |e: &Expr| e.replace(with, callbacks).map(Box::new);
        let props = |props: &[Prop]| {
            props
                .iter()
                .map(|p| match p {
                    Prop::Field(name, value) => Some(Prop::Field(name.clone(), value.replace(with, callbacks)?)),
                    Prop::Getter(name, value) => Some(Prop::Getter(name.clone(), value.replace(with, callbacks)?)),
                    Prop::Setter(name, value) => Some(Prop::Setter(name.clone(), value.replace(with, callbacks)?)),
                    Prop::Spread(value) => Some(Prop::Spread(value.replace(with, callbacks)?)),
                })
                .collect::<Option<Vec<_>>>()
        };
        let kind = match &self.kind {
            ExprKind::Var(name) => match with(name) {
                Some(e) => return Some(e.or_at(self.span)),
                None => ExprKind::Var(name.clone()),
            },
            // A closure reading none of the names replaced is the same
            // closure, whenever it runs.
            ExprKind::Arrow(..) | ExprKind::AsyncArrow(..) | ExprKind::Function(_)
                if {
                    let mut replaced = false;
                    self.visit_vars(&mut |name| replaced |= with(name).is_some());
                    !replaced
                } =>
            {
                self.kind.clone()
            }
            ExprKind::Arrow(params, body) if callbacks => {
                let [
                    Stmt {
                        kind: StmtKind::Return(Some(value)),
                        span,
                    },
                ] = body.as_slice()
                else {
                    return None;
                };
                // A name it binds is its own in its body, not the one replaced.
                let names: Vec<&str> = params.iter().flat_map(Pattern::names).collect();
                let clash = std::cell::Cell::new(false);
                let inner = |n: &str| {
                    let replaced = with(n).filter(|_| !names.contains(&n))?;
                    clash.set(clash.get() || replaced.mentions(&names));
                    Some(replaced)
                };
                let value = value.replace(&inner, true)?;
                if clash.get() {
                    return None;
                }
                ExprKind::Arrow(params.clone(), vec![StmtKind::Return(Some(value)).at(*span)])
            }
            ExprKind::Arrow(..) | ExprKind::AsyncArrow(..) | ExprKind::Function(_) => return None,
            ExprKind::Member(a, field) => ExprKind::Member(one(a)?, field.clone()),
            ExprKind::OptionalMember(a, field) => ExprKind::OptionalMember(one(a)?, field.clone()),
            ExprKind::Handle(place) => ExprKind::Handle(one(place)?),
            ExprKind::DropArgument(item, index, drop) => ExprKind::DropArgument(*item, *index, one(drop)?),
            ExprKind::Pair(place, dictionary) => ExprKind::Pair(one(place)?, one(dictionary)?),
            ExprKind::Index(a, b) => ExprKind::Index(one(a)?, one(b)?),
            ExprKind::OptionalIndex(a, b) => ExprKind::OptionalIndex(one(a)?, one(b)?),
            ExprKind::Array(items) => ExprKind::Array(all(items)?),
            ExprKind::Object(fields) => ExprKind::Object(props(fields)?),
            ExprKind::Unary(op, a) => ExprKind::Unary(*op, one(a)?),
            ExprKind::Binary(op, a, b) => ExprKind::Binary(*op, one(a)?, one(b)?),
            ExprKind::Cond(a, b, c) => ExprKind::Cond(one(a)?, one(b)?, one(c)?),
            ExprKind::Call(f, args) => ExprKind::Call(one(f)?, all(args)?),
            ExprKind::OptionalCall(f, args) => ExprKind::OptionalCall(one(f)?, all(args)?),
            ExprKind::New(f, args) => ExprKind::New(one(f)?, all(args)?),
            ExprKind::Await(a) => ExprKind::Await(one(a)?),
            ExprKind::Spread(a) => ExprKind::Spread(one(a)?),
            ExprKind::Stringed(a) => ExprKind::Stringed(one(a)?),
            ExprKind::Template(texts, values, lines) => ExprKind::Template(texts.clone(), all(values)?, *lines),
            ExprKind::Jsx(jsx) => ExprKind::Jsx(Box::new(Jsx {
                tag: match &jsx.tag {
                    JsxTag::Component(c) => JsxTag::Component(c.replace(with, callbacks)?),
                    tag => tag.clone(),
                },
                props: props(&jsx.props)?,
                children: all(&jsx.children)?,
            })),
            ExprKind::Num(_)
            | ExprKind::BigInt(_)
            | ExprKind::BigUint(_)
            | ExprKind::Bool(_)
            | ExprKind::Str(_)
            | ExprKind::Lines(_)
            | ExprKind::Undefined
            | ExprKind::GivenUndefined
            | ExprKind::Null
            | ExprKind::Symbol(_)
            | ExprKind::FunctionHole(_)
            | ExprKind::Import(_)
            | ExprKind::Regex(_) => self.kind.clone(),
        };
        Some(Expr {
            reads: false,
            kind,
            span: self.span,
        })
    }

    /// Does this name the variable `name` anywhere in it, a template's, an
    /// object's, JSX's or a closure's part too: `visit_vars`' answer, the one
    /// walk of what an expression reads.
    pub fn mentions_var(&self, name: &str) -> bool {
        let mut found = false;
        self.visit_vars(&mut |var| found |= var == name);
        found
    }

    /// Could this read one of `names`? Only a path of them, `a.b`, or a
    /// constant is known not to.
    fn mentions(&self, names: &[&str]) -> bool {
        match &self.kind {
            ExprKind::Var(n) => names.contains(&n.as_str()),
            ExprKind::Symbol(_) => false,
            ExprKind::Member(object, _) | ExprKind::OptionalMember(object, _) => object.mentions(names),
            _ => !self.is_constant(),
        }
    }

    /// This as a test: `!!a` is `a`, in the parts of `&&` and `||` too,
    /// whose value the test only asks the truth of. `x != null && x` is `x`,
    /// of a path, `props.meta`, too, wherever in an `&&` it is: what's
    /// truthy is set.
    pub fn tested(&self) -> Expr {
        self.as_test(&mut false)
    }

    /// `tested`, if it's not this as it is.
    pub fn retested(&self) -> Option<Expr> {
        let mut changed = false;
        let tested = self.as_test(&mut changed);
        changed.then_some(tested)
    }

    /// `tested`, telling `changed` if it's not this as it is.
    fn as_test(&self, changed: &mut bool) -> Expr {
        match &self.kind {
            ExprKind::Unary(UnaryOp::Not, inner) if let ExprKind::Unary(UnaryOp::Not, value) = &inner.kind => {
                *changed = true;
                value.as_test(changed)
            }
            ExprKind::Binary(Op::And, ..) => {
                fn parts(e: &Expr, out: &mut Vec<Expr>, changed: &mut bool) {
                    match &e.kind {
                        ExprKind::Binary(Op::And, a, b) => {
                            parts(a, out, changed);
                            parts(b, out, changed);
                        }
                        _ => out.push(e.as_test(changed)),
                    }
                }
                let mut all = Vec::new();
                parts(self, &mut all, changed);
                let set_and_true = |i: usize| {
                    null_test(&all[i], true).is_some_and(|x| all.get(i + 1).is_some_and(|next| same_path(next, x)))
                };
                let kept: Vec<Expr> = (0..all.len())
                    .filter(|&i| !set_and_true(i))
                    .map(|i| all[i].clone())
                    .collect();
                *changed |= kept.len() < all.len();
                let span = self.span;
                kept.into_iter()
                    .reduce(|a, b| Expr {
                        reads: false,
                        kind: ExprKind::Binary(Op::And, Box::new(a), Box::new(b)),
                        span,
                    })
                    .expect("a part")
            }
            ExprKind::Binary(op @ Op::Or, a, b) => Expr {
                reads: false,
                kind: ExprKind::Binary(*op, Box::new(a.as_test(changed)), Box::new(b.as_test(changed))),
                span: self.span,
            },
            _ => self.clone(),
        }
    }

    /// Is this `(x) => x`?
    pub fn is_identity(&self) -> bool {
        let ExprKind::Arrow(params, body) = &self.kind else {
            return false;
        };
        let [Pattern::Name(param)] = params.as_slice() else {
            return false;
        };
        matches!(body.as_slice(), [Stmt { kind: StmtKind::Return(Some(value)), .. }]
            if matches!(&value.kind, ExprKind::Var(name) if name == param))
    }

    /// Is this the same value, cheaply, however often it's read? A variable,
    /// a property of one, an item of one at a constant index (`t[0]`), or a
    /// constant: what can be written twice.
    pub fn reads_same(&self) -> bool {
        match &self.kind {
            ExprKind::Var(_) | ExprKind::Symbol(_) => true,
            ExprKind::Member(object, _) | ExprKind::OptionalMember(object, _) => object.reads_same(),
            ExprKind::Index(object, index) => object.reads_same() && (index.is_constant() || index.reads_same()),
            _ => self.is_constant(),
        }
    }

    /// Whether this reads nothing but variables, and does nothing: no
    /// call, and no property, which something done meanwhile could change
    /// through a `&mut`. Elements, conditions and literals of variables.
    pub fn reads_only_vars(&self) -> bool {
        let props = |props: &[Prop]| {
            props.iter().all(|p| match p {
                Prop::Field(_, value) | Prop::Getter(_, value) | Prop::Setter(_, value) | Prop::Spread(value) => {
                    value.reads_only_vars()
                }
            })
        };
        match &self.kind {
            ExprKind::Var(_)
            | ExprKind::Symbol(_)
            | ExprKind::FunctionHole(_)
            | ExprKind::Arrow(..)
            | ExprKind::AsyncArrow(..)
            | ExprKind::Function(_) => true,
            ExprKind::Unary(_, a) | ExprKind::Spread(a) => a.reads_only_vars(),
            ExprKind::Binary(_, a, b) => a.reads_only_vars() && b.reads_only_vars(),
            ExprKind::Cond(a, b, c) => a.reads_only_vars() && b.reads_only_vars() && c.reads_only_vars(),
            ExprKind::Array(items) | ExprKind::Template(_, items, _) => items.iter().all(Expr::reads_only_vars),
            ExprKind::Object(items) => props(items),
            ExprKind::Jsx(jsx) => {
                let tag = match &jsx.tag {
                    JsxTag::Component(c) => c.reads_only_vars(),
                    _ => true,
                };
                tag && props(&jsx.props) && jsx.children.iter().all(Expr::reads_only_vars)
            }
            _ => self.is_constant(),
        }
    }

    /// Could evaluating this do something observable (call a function, throw)?
    /// Whether evaluating it may throw: whatever does something may, but a
    /// call of a runtime helper that never throws, `$rangeNext(range)`,
    /// throws only where its arguments do.
    pub fn may_throw(&self) -> bool {
        match &self.kind {
            ExprKind::Call(callee, args) if callee.helper_name().is_some_and(crate::runtime::never_throws) => {
                args.iter().any(Expr::may_throw)
            }
            _ => self.has_effects(),
        }
    }

    /// Whether evaluating this only reads, however much it calls: each call
    /// a runtime helper's that only reads, `$debugStr(s)`, a JS global's
    /// conversion, `String(n)`, or a JS method of text or an array that
    /// changes nothing, `.map(f)`, `.join(", ")`, each callback's body only
    /// reading too; nothing assigned.
    pub fn only_reads(&self) -> bool {
        self.calls_only(&|helper| crate::runtime::is_pure(helper))
    }

    /// Whether evaluating this may run the program's own code: a function
    /// of the crate's, a `Debug` of its own, a method of what JS doesn't
    /// know, a dictionary's. Not a runtime helper's, which runs only what
    /// it's given, a JS global's, nor a JS method of text or an array,
    /// though each callback's body is looked at.
    pub fn runs_program_code(&self) -> bool {
        !self.calls_only(&|_| true)
    }

    /// Whether each call in this is a helper's `helper` allows, a JS
    /// global's, or a JS method of text or an array that changes nothing,
    /// with each callback's body the same, and nothing assigned: the one
    /// list of what JS's own calls are, `only_reads`' and
    /// `runs_program_code`'s.
    fn calls_only(&self, helper: &dyn Fn(&str) -> bool) -> bool {
        const METHODS: &[&str] = &[
            "map",
            "join",
            "filter",
            "slice",
            "concat",
            "toString",
            "toFixed",
            "padStart",
            "padEnd",
            "toUpperCase",
            "toLowerCase",
            "entries",
            "keys",
            "values",
            "toArray",
            "repeat",
            "trim",
            "at",
            "includes",
            "indexOf",
            "startsWith",
            "endsWith",
            "codePointAt",
            "charCodeAt",
        ];
        const GLOBALS: &[&str] = &["String", "Number", "BigInt", "Math", "JSON", "Array", "Object"];
        let mut only = true;
        let mut each = self.clone();
        each.each_mut(&mut |part| {
            only &= match &part.kind {
                ExprKind::Call(callee, _) | ExprKind::OptionalCall(callee, _) => match &callee.kind {
                    ExprKind::Var(_) if let Some(name) = callee.helper_name() => helper(name),
                    ExprKind::Var(name) => GLOBALS.contains(&name.as_str()),
                    ExprKind::Member(of, method) | ExprKind::OptionalMember(of, method) => {
                        METHODS.contains(&method.as_str())
                            || matches!(&of.kind, ExprKind::Var(global) if GLOBALS.contains(&global.as_str()))
                    }
                    // A callback called at once, its body looked at as each part is.
                    ExprKind::Arrow(..) => true,
                    _ => false,
                },
                ExprKind::New(..) | ExprKind::Await(_) | ExprKind::Import(_) | ExprKind::Handle(_) => false,
                ExprKind::Arrow(_, body) => body.iter().all(|stmt| {
                    matches!(
                        &stmt.kind,
                        StmtKind::Return(_) | StmtKind::Expr(_) | StmtKind::Const(..)
                    )
                }),
                ExprKind::AsyncArrow(..) | ExprKind::Function(_) => false,
                _ => true,
            };
        });
        only
    }

    pub fn has_effects(&self) -> bool {
        match &self.kind {
            ExprKind::Num(_)
            | ExprKind::BigInt(_)
            | ExprKind::BigUint(_)
            | ExprKind::Bool(_)
            | ExprKind::Str(_)
            | ExprKind::Lines(_)
            | ExprKind::Undefined
            | ExprKind::GivenUndefined
            | ExprKind::Null
            | ExprKind::Var(_)
            | ExprKind::Symbol(_)
            | ExprKind::Regex(_)
            | ExprKind::FunctionHole(_)
            | ExprKind::Arrow(..)
            | ExprKind::AsyncArrow(..)
            | ExprKind::Function(_) => false,
            // It lets other code run meanwhile, and loads a module.
            ExprKind::Await(_) | ExprKind::Import(_) => true,
            ExprKind::Spread(a) | ExprKind::Stringed(a) | ExprKind::DropArgument(_, _, a) => a.has_effects(),
            ExprKind::Member(object, _) | ExprKind::OptionalMember(object, _) => object.has_effects(),
            // Making one reads nothing: its getter does, later.
            ExprKind::Handle(_) => false,
            ExprKind::Pair(_, dictionary) => dictionary.has_effects(),
            ExprKind::Index(object, index) | ExprKind::OptionalIndex(object, index) => {
                object.has_effects() || index.has_effects()
            }
            ExprKind::Array(items) | ExprKind::Template(_, items, _) => items.iter().any(Expr::has_effects),
            ExprKind::Object(props) => props.iter().any(|p| match p {
                Prop::Field(_, value) | Prop::Getter(_, value) | Prop::Setter(_, value) | Prop::Spread(value) => {
                    value.has_effects()
                }
            }),
            ExprKind::Unary(_, a) => a.has_effects(),
            ExprKind::Binary(_, a, b) => a.has_effects() || b.has_effects(),
            ExprKind::Cond(a, b, c) => a.has_effects() || b.has_effects() || c.has_effects(),
            // A number wrapped to its width, `BigInt.asIntN(64, n)`, or
            // computed by `Math`, `Math.max(a, 0)`, but for `Math.random()`,
            // which steps its state: what its arguments do. Nothing the
            // program names shadows `Math`; that one's renamed, `Math$`.
            ExprKind::Call(callee, args)
                if let ExprKind::Member(of, name) = &callee.kind
                    && let ExprKind::Var(global) = &of.kind
                    && match global.as_str() {
                        "BigInt" => matches!(name.as_str(), "asIntN" | "asUintN"),
                        "Math" => name != "random",
                        _ => false,
                    } =>
            {
                args.iter().any(Expr::has_effects)
            }
            // The runtime's that only read what they're given, make a value
            // of it, or its text, `$jsonIndex(v, "a")`, `$debugStr(s)`: what
            // their arguments do.
            ExprKind::Call(callee, args) if callee.helper_name().is_some_and(crate::runtime::is_pure) => {
                args.iter().any(Expr::has_effects)
            }
            ExprKind::Call(..) | ExprKind::OptionalCall(..) | ExprKind::New(..) => true,
            // Making an element runs nothing: a component runs when React renders it.
            ExprKind::Jsx(jsx) => {
                matches!(&jsx.tag, JsxTag::Component(c) if c.has_effects())
                    || jsx.props.iter().any(|p| match p {
                        Prop::Field(_, value)
                        | Prop::Getter(_, value)
                        | Prop::Setter(_, value)
                        | Prop::Spread(value) => value.has_effects(),
                    })
                    || jsx.children.iter().any(Expr::has_effects)
            }
        }
    }
}

/// Each name `stmts` read or declare, their closures', functions' and
/// parameters' too: what a new name must not be.
pub fn names_in(stmts: &[Stmt], names: &mut HashSet<String>) {
    visit_stmts(stmts, &mut |name| {
        names.insert(name.to_string());
    });
    let mut stmts = stmts.to_vec();
    let params = |params: &[Pattern], names: &mut HashSet<String>| {
        names.extend(params.iter().flat_map(|p| p.names()).map(str::to_string));
    };
    each_block_mut(&mut stmts, &mut |list| {
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
                StmtKind::Function(function) => {
                    names.insert(function.name.clone());
                    params(&function.params, names);
                }
                _ => {}
            }
        }
    });
    each_expr_mut(&mut stmts, &mut |e| match &e.kind {
        ExprKind::Arrow(ps, _) | ExprKind::AsyncArrow(ps, _) => params(ps, names),
        ExprKind::Function(function) => params(&function.params, names),
        _ => {}
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module_of(value: Expr) -> Module {
        Module {
            header: String::new(),
            directives: Vec::new(),
            packages: Vec::new(),
            imports: Vec::new(),
            reexports: Vec::new(),
            helpers: Vec::new(),
            items: vec![Item::Statements(vec![StmtKind::Expr(value).at(Span::NONE)])],
            caches: Vec::new(),
            default_export: None,
        }
    }

    // What only an earlier phase resolves is found however deep it is.
    #[test]
    fn unprintable_finds_what_an_earlier_phase_resolves() {
        let handle = Expr::call(Expr::var("f"), vec![Expr::handle(Expr::var("n"))]);
        assert_eq!(module_of(handle).unprintable(), Some("a `&mut`'s handle"));
        let import = Expr::new(ExprKind::Import(ImportFrom::Module(vec!["a".into()])));
        assert!(module_of(import).unprintable().is_some());
        assert_eq!(module_of(Expr::var("x")).unprintable(), None);
    }

    // What an expression reads is one walk's answer: a template's, an
    // object's part too, which `t = `${t}x`` reads `t` in.
    #[test]
    fn mentions_var_finds_every_read() {
        let template = Expr::template(vec![String::new(), "x".into()], vec![Expr::var("t")]);
        assert!(template.mentions_var("t"));
        let object = Expr::object(vec![Prop::Field("a".into(), Expr::var("t"))]);
        assert!(object.mentions_var("t"));
        assert!(!object.mentions_var("u"));
    }

    // A test of being set is one however `null` is spelled, and its `==`
    // is one of not being set.
    #[test]
    fn null_test_is_one_answer() {
        for null in [Expr::null(), Expr::new(ExprKind::Undefined)] {
            let set = Expr::new(ExprKind::Binary(
                Op::LooseNe,
                Box::new(Expr::var("x")),
                Box::new(null.clone()),
            ));
            assert!(null_test(&set, true).is_some() && null_test(&set, false).is_none());
            let unset = Expr::new(ExprKind::Binary(Op::LooseEq, Box::new(Expr::var("x")), Box::new(null)));
            assert!(null_test(&unset, false).is_some() && null_test(&unset, true).is_none());
        }
    }

    // Each list of a module's statements is visited once, a constant's
    // closures' however deep, and a `function` it holds.
    #[test]
    fn each_item_block_once() {
        let ret = |e: Expr| vec![StmtKind::Return(Some(e)).at(Span::NONE)];
        let inner = Expr::arrow(vec![], ret(Expr::var("x")));
        let outer = Expr::arrow(vec![], ret(inner));
        let function = Function {
            name: String::new(),
            params: Vec::new(),
            body: ret(Expr::var("y")),
            export: false,
            is_async: false,
            span: Span::NONE,
            name_span: Span::NONE,
        };
        let value = Expr::array(vec![outer, Expr::new(ExprKind::Function(Box::new(function)))]);
        let mut items = vec![Item::Const(Const {
            name: "c".into(),
            value,
            export: false,
            mutable: false,
            span: Span::NONE,
        })];
        let mut lists = 0;
        each_item_block_mut(&mut items, &mut |_| lists += 1);
        assert_eq!(lists, 3);
    }

    // A new name is none a function declared in a block has, or takes.
    #[test]
    fn names_in_finds_functions_and_their_params() {
        let inner = Function {
            name: "inner".into(),
            params: vec![Pattern::Name("x".into())],
            body: vec![StmtKind::Const("y".into(), Expr::var("x")).at(Span::NONE)],
            export: false,
            is_async: false,
            span: Span::NONE,
            name_span: Span::NONE,
        };
        let mut names = HashSet::new();
        names_in(&[StmtKind::Function(Box::new(inner)).at(Span::NONE)], &mut names);
        for name in ["inner", "x", "y"] {
            assert!(names.contains(name), "{name}");
        }
    }
}
