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
        ExprKind::Binary(Op::LooseNe, x, null) if matches!(null.kind, ExprKind::Null) => Some(x),
        ExprKind::Unary(UnaryOp::Not, not) => match &not.kind {
            ExprKind::Unary(UnaryOp::Not, x) => Some(x),
            _ => None,
        },
        _ => None,
    }
}

/// Are `a` and `b` one path: a variable, or `a.b.c` or `a?.b` of the same names?
pub(crate) fn same_path(a: &Expr, b: &Expr) -> bool {
    match (&a.kind, &b.kind) {
        (ExprKind::Var(x), ExprKind::Var(y)) => x == y,
        (ExprKind::Member(x, f), ExprKind::Member(y, g)) => f == g && same_path(x, y),
        (ExprKind::OptionalMember(x, f), ExprKind::OptionalMember(y, g)) => f == g && same_path(x, y),
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
/// a closure's body too: for a pass that changes them.
pub fn each_block_mut(stmts: &mut Vec<Stmt>, f: &mut dyn FnMut(&mut Vec<Stmt>)) {
    statement_lists(stmts, f);
    each_expr_mut(stmts, &mut |e| match &mut e.kind {
        ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => statement_lists(body, f),
        ExprKind::Function(function) => statement_lists(&mut function.body, f),
        _ => {}
    });
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
            | StmtKind::Expr(_)
            | StmtKind::Break(_)
            | StmtKind::Continue(_)
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
            StmtKind::Assign(a, b) => {
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
            StmtKind::Let(_, None) | StmtKind::Break(_) | StmtKind::Continue(_) | StmtKind::Return(None) => {}
        }
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
            StmtKind::Assign(target, value) => {
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
            StmtKind::Break(_) | StmtKind::Continue(_) => {}
        }
    }
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
    /// `None` skips an element: `[, b]`.
    Array(Vec<Option<String>>),
    /// Each field, the variable it goes in, and its default where it's
    /// missing, `{ size = "md" }` (ADR 0212), a literal; and the variable the
    /// rest go in, `...rest` (ADR 0195).
    Object(Vec<(String, String, Option<Expr>)>, Option<String>),
}

impl Pattern {
    /// The variables it binds.
    pub fn names(&self) -> Vec<&str> {
        match self {
            Pattern::Name(name) => vec![name.as_str()],
            Pattern::Array(items) => items.iter().flatten().map(String::as_str).collect(),
            Pattern::Object(fields, rest) => fields
                .iter()
                .map(|(_, name, _)| name.as_str())
                .chain(rest.as_deref())
                .collect(),
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
        Expr { kind, span: Span::NONE }
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
        if property == "value" {
            if let ExprKind::Handle(place) = object.kind {
                return *place;
            }
            if let ExprKind::Object(props) = &object.kind
                && let [Prop::Field(name, value)] = props.as_slice()
                && name == "value"
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
        // `[x][0]` is `x`, where `x` is an item, not `...items`.
        if let (ExprKind::Array(items), Some(0)) = (&object.kind, index.as_int())
            && let [item] = &items[..]
            && !matches!(item.kind, ExprKind::Spread(_))
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
                kind: ExprKind::Binary(ne, a.clone(), b.clone()),
                span: arg.span,
            };
        }
        Expr::new(ExprKind::Unary(op, Box::new(arg)))
    }

    pub fn bin(op: Op, lhs: Expr, rhs: Expr) -> Expr {
        // `a != null && !!a` is `!!a`: neither is truthy.
        if op == Op::And
            && let ExprKind::Binary(Op::LooseNe, tested, null) = &lhs.kind
            && matches!(null.kind, ExprKind::Null)
            && let ExprKind::Unary(UnaryOp::Not, inner) = &rhs.kind
            && let ExprKind::Unary(UnaryOp::Not, of) = &inner.kind
            && same_path(tested, of)
        {
            return rhs;
        }
        // `a != null && Array.isArray(a)` is `Array.isArray(a)`: no array is
        // `null` or `undefined`.
        if op == Op::And
            && let Some(tested) = presence_of(&lhs)
            && let ExprKind::Call(callee, args) = &rhs.kind
            && let (ExprKind::Member(array, is_array), [of]) = (&callee.kind, args.as_slice())
            && matches!(&array.kind, ExprKind::Var(name) if name == "Array")
            && is_array == "isArray"
            && same_path(tested, of)
        {
            return rhs;
        }
        Expr::new(ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)))
    }

    pub fn cond(test: Expr, then: Expr, els: Expr) -> Expr {
        // `a != null ? a.b : undefined` is `a?.b`, as a person writes it. One
        // property only: `a?.b.c` would end the chain at `.c` too.
        if let Some(tested) = presence_of(&test)
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
        if let Some(tested) = presence_of(&test)
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
                let (Prop::Field(_, value) | Prop::Spread(value) | Prop::Getter(_, value)) = prop;
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
                    Prop::Field(_, value) | Prop::Spread(value) | Prop::Getter(_, value) => value.visit_vars(read),
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

    /// Is there JSX in this, like `items.map((t) => <li>..</li>)`?
    pub fn contains_jsx(&self) -> bool {
        match &self.kind {
            ExprKind::Jsx(_) => true,
            ExprKind::Array(items) => items.iter().any(Expr::contains_jsx),
            ExprKind::Object(props) => props.iter().any(|p| match p {
                Prop::Field(_, value) | Prop::Getter(_, value) | Prop::Spread(value) => value.contains_jsx(),
            }),
            ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => body.iter().any(|s| match &s.kind {
                StmtKind::Return(Some(value)) | StmtKind::Expr(value) => value.contains_jsx(),
                _ => false,
            }),
            ExprKind::Member(a, _)
            | ExprKind::OptionalMember(a, _)
            | ExprKind::Unary(_, a)
            | ExprKind::Await(a)
            | ExprKind::Spread(a)
            | ExprKind::Stringed(a)
            | ExprKind::DropArgument(_, _, a) => a.contains_jsx(),
            ExprKind::Handle(_) | ExprKind::Pair(..) => false,
            ExprKind::Index(a, b) | ExprKind::OptionalIndex(a, b) | ExprKind::Binary(_, a, b) => {
                a.contains_jsx() || b.contains_jsx()
            }
            ExprKind::Cond(a, b, c) => a.contains_jsx() || b.contains_jsx() || c.contains_jsx(),
            ExprKind::Call(f, args) | ExprKind::OptionalCall(f, args) | ExprKind::New(f, args) => {
                f.contains_jsx() || args.iter().any(Expr::contains_jsx)
            }
            ExprKind::Template(_, values, _) => values.iter().any(Expr::contains_jsx),
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
            | ExprKind::FunctionHole(_)
            | ExprKind::Import(_)
            | ExprKind::Function(_)
            | ExprKind::Regex(_) => false,
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
                let names: Vec<&str> = params.iter().flat_map(Pattern::names).collect();
                if names.iter().any(|n| with(n).is_some()) {
                    return None;
                }
                let clash = std::cell::Cell::new(false);
                let inner = |n: &str| {
                    let replaced = with(n)?;
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
        Some(Expr { kind, span: self.span })
    }

    /// Does this name the variable `name` anywhere in it?
    pub fn mentions_var(&self, name: &str) -> bool {
        match &self.kind {
            ExprKind::Var(n) => n == name,
            ExprKind::Member(a, _)
            | ExprKind::OptionalMember(a, _)
            | ExprKind::Unary(_, a)
            | ExprKind::Await(a)
            | ExprKind::Spread(a)
            | ExprKind::Handle(a)
            | ExprKind::DropArgument(_, _, a) => a.mentions_var(name),
            ExprKind::Index(a, b) | ExprKind::Binary(_, a, b) | ExprKind::Pair(a, b) => {
                a.mentions_var(name) || b.mentions_var(name)
            }
            ExprKind::Cond(a, b, c) => a.mentions_var(name) || b.mentions_var(name) || c.mentions_var(name),
            ExprKind::Call(f, args) | ExprKind::OptionalCall(f, args) | ExprKind::New(f, args) => {
                f.mentions_var(name) || args.iter().any(|a| a.mentions_var(name))
            }
            ExprKind::Array(items) => items.iter().any(|a| a.mentions_var(name)),
            _ => false,
        }
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
    /// whose value the test only asks the truth of.
    pub fn tested(&self) -> Expr {
        match &self.kind {
            ExprKind::Unary(UnaryOp::Not, inner) if let ExprKind::Unary(UnaryOp::Not, value) = &inner.kind => {
                value.tested()
            }
            ExprKind::Binary(op @ (Op::And | Op::Or), a, b) => Expr {
                kind: ExprKind::Binary(*op, Box::new(a.tested()), Box::new(b.tested())),
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
            ExprKind::Index(object, index) => object.reads_same() && index.is_constant(),
            _ => self.is_constant(),
        }
    }

    /// Whether this reads nothing but variables, and does nothing: no
    /// call, and no property, which something done meanwhile could change
    /// through a `&mut`. Elements, conditions and literals of variables.
    pub fn reads_only_vars(&self) -> bool {
        let props = |props: &[Prop]| {
            props.iter().all(|p| match p {
                Prop::Field(_, value) | Prop::Getter(_, value) | Prop::Spread(value) => value.reads_only_vars(),
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
                Prop::Field(_, value) | Prop::Getter(_, value) | Prop::Spread(value) => value.has_effects(),
            }),
            ExprKind::Unary(_, a) => a.has_effects(),
            ExprKind::Binary(_, a, b) => a.has_effects() || b.has_effects(),
            ExprKind::Cond(a, b, c) => a.has_effects() || b.has_effects() || c.has_effects(),
            ExprKind::Call(..) | ExprKind::OptionalCall(..) | ExprKind::New(..) => true,
            // Making an element runs nothing: a component runs when React renders it.
            ExprKind::Jsx(jsx) => {
                matches!(&jsx.tag, JsxTag::Component(c) if c.has_effects())
                    || jsx.props.iter().any(|p| match p {
                        Prop::Field(_, value) | Prop::Getter(_, value) | Prop::Spread(value) => value.has_effects(),
                    })
                    || jsx.children.iter().any(Expr::has_effects)
            }
        }
    }
}
