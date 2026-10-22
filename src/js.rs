//! A tiny JavaScript AST: exactly the constructs rust-js emits.
//!
//! `lower.rs` builds it; `prepare.rs` prepares readable JSX; `to_oxc.rs`
//! converts it to oxc's AST, which prints
//! it (with correct parentheses) and builds the source map. Keeping our own
//! small tree means the lowering never touches oxc's large, fast-changing API.
//!
//! Every node carries a `Span`: byte offsets into the Rust source file. That
//! is what lets the source map point from JS back to Rust.

mod applied;
mod expr_walk;
mod make;
mod stmt_walk;

pub use self::{
    applied::{applied_blocks, applied_call},
    expr_walk::*,
    stmt_walk::*,
};

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
    /// `export default page;`, after its functions (ADR 0192).
    pub default_export: Option<String>,
    /// `export { getCurrentTime as unstable_now };`, a `pub use` of its own
    /// item by another name, as `(local, exported)`, after its functions
    /// (ADR 0596).
    pub aliases: Vec<(String, String)>,
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
            Item::Namespace(namespace) => {
                namespace.constants.iter().for_each(|c| c.value.visit_vars(read));
                namespace.methods.iter().for_each(|m| visit_stmts(&m.body, read));
            }
            Item::Const(constant) => constant.value.visit_vars(read),
            Item::Statements(stmts) => visit_stmts(stmts, read),
            Item::Function(function) => visit_stmts(&function.body, read),
        }
    }

    /// Where its Rust is, of what's in the crate's sources.
    fn spans(&self) -> Vec<Span> {
        let spans: Vec<Span> = match self {
            Item::Namespace(namespace) => (namespace.constants.iter().map(|c| c.span))
                .chain(namespace.methods.iter().map(|m| m.span))
                .collect(),
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

/// A type's methods (ADR 0047), an object named after the type:
/// `export const Counter = { new(step) { .. }, tick(counter) { .. } };`.
pub struct Namespace {
    pub name: String,
    /// Its constants, `ID: (s) => s`, properties before its methods (ADR 0410).
    pub constants: Vec<Const>,
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
#[derive(Clone, PartialEq)]
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

#[derive(Clone, PartialEq)]
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
#[derive(Clone, PartialEq)]
pub enum Pattern {
    Name(String),
    /// `None` skips an element: `[, b]`. An element may be taken apart in
    /// turn, `[i, { name }]`.
    Array(Vec<Option<Pattern>>),
    /// Each field, what it goes in, a variable or a pattern of its own, and
    /// its default where it's missing, `{ size = "md" }` (ADR 0212), a
    /// literal; and the variable the rest go in, `...rest` (ADR 0195).
    Object(Vec<(String, Pattern, Option<Expr>)>, Option<String>),
    /// A function's last parameter that gathers what it's given past the
    /// rest, `...args`: a C-variadic function's (ADR 0428).
    Rest(String),
    /// A function's parameter and what it is where it's given none,
    /// `location = { file, line, column }`: a dictionary's entry's of an impl
    /// that tracks its caller, which a `dyn`'s call gives none (ADR 0427).
    Defaulted(String, Expr),
}

impl Pattern {
    /// Each name it declares.
    pub fn each_name_mut(&mut self, f: &mut dyn FnMut(&mut String)) {
        match self {
            Pattern::Name(name) | Pattern::Rest(name) | Pattern::Defaulted(name, _) => f(name),
            Pattern::Array(items) => items.iter_mut().flatten().for_each(|p| p.each_name_mut(f)),
            Pattern::Object(fields, rest) => {
                fields.iter_mut().for_each(|(_, p, _)| p.each_name_mut(f));
                if let Some(rest) = rest {
                    f(rest);
                }
            }
        }
    }

    /// An array of variables, `[count, setCount]`; `None` skips one.
    pub fn names_array(names: impl IntoIterator<Item = Option<String>>) -> Pattern {
        Pattern::Array(names.into_iter().map(|name| name.map(Pattern::Name)).collect())
    }

    /// The variables it binds.
    pub fn names(&self) -> Vec<&str> {
        match self {
            Pattern::Name(name) | Pattern::Rest(name) | Pattern::Defaulted(name, _) => vec![name.as_str()],
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
            Pattern::Name(name) | Pattern::Rest(name) | Pattern::Defaulted(name, _) => vec![name],
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
            Pattern::Name(_) | Pattern::Rest(_) => Vec::new(),
            Pattern::Defaulted(_, default) => vec![default],
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

#[derive(Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, PartialEq)]
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

#[derive(Clone, PartialEq)]
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

#[derive(Clone, PartialEq)]
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
#[derive(Clone, PartialEq)]
pub struct Jsx {
    pub tag: JsxTag,
    /// Its attributes, `className="hero"`, or `{...props}`.
    pub props: Vec<Prop>,
    pub children: Vec<Expr>,
}

#[derive(Clone, PartialEq)]
pub enum JsxTag {
    /// `<>`.
    Fragment,
    /// A DOM element: `<div>`.
    Intrinsic(String),
    /// A component: `<Counter>`, `<StrictMode>`, `<stats.Chart>`.
    Component(Expr),
}

#[derive(Clone, PartialEq)]
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

#[derive(Clone, Copy, PartialEq)]
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
            default_export: None,
            aliases: Vec::new(),
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
