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
mod borrow_checks;
mod expr_walk;
mod imports;
mod known;
mod make;
mod optional_call;
pub mod result;
mod stmt_walk;
mod storage;
mod template;
pub mod undefined_arguments;
pub mod undefined_keys;
pub mod validate;
mod walk;
mod writes;

pub use self::{
    applied::{
        applied_blocks, applied_call, applied_closures, applied_tried, called_body, read_where_made, returns,
        returns_replaced, tried_extent, tried_set_where_made,
    },
    borrow_checks::borrow_checks_first,
    expr_walk::*,
    imports::{Import, ImportFrom, Package, Reexport},
    known::{Known, Role, forget as forget_known, found_names as known_found, remembering},
    stmt_walk::*,
    walk::*,
    writes::Writes,
};
use crate::collections::{IndexMap, IndexSet};

use std::collections::BTreeSet;

/// Byte offsets `lo..hi` into the Rust source file. `Span::NONE` (empty)
/// means "no mapping": oxc skips empty spans when building the source map.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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

/// `it.next().value`: what a step of a JS iterator gives, `undefined` at
/// its end, its result's own `value`, not a box's (rd270).
pub fn next_value(it: Expr) -> Expr {
    Expr::member(Expr::call(Expr::member(it, "next"), Vec::new()), "value")
}
/// How a place stores what's written to it, which a handle on it keeps,
/// wherever it's written through (`ExprKind::Handle`, ADR 0791).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Storage {
    /// What's written itself.
    AsGiven,
    /// A `#[rust_js::nullable]` field's `Option`, or `Cell` of one: its
    /// `None`, `null` (ADRs 0275, 0772).
    NullForNone,
    /// A flattened state's cell, the object it's flattened in (rd380, ADR
    /// 1044): what's written is each of its keys, these, comma-separated,
    /// set on that object, never the object itself.
    InPlace(&'static str),
    /// An object a `&mut` is, written whole in place, `$assign(place,
    /// value)`, so each name for it sees what's written (ADRs 0147, 0454):
    /// a `&mut dyn`'s pair of one whose impl writes its `value` (#968).
    Assigned,
}

/// What a `Symbol` of a module's namespace exports, the module itself,
/// `import * as Plugin` (rd114, ADR 1018), as a `Reexport`'s `*` is of an
/// `export * as ns from`: one of its items is a member of it,
/// `Plugin.extractEvents`.
pub const NAMESPACE: &str = "*";

/// The field a `dyn` value's pair holds its dictionary in, `d.impl` (ADR
/// 0049), beside its `value`.
pub const DICTIONARY: &str = "impl";

/// The field an unwind's payload holds what was thrown in, `payload.error`:
/// a panic's `Error`, beside its message, `value` (ADR 0692); what JS threw
/// of any other; or the `$Unwinding` a typed payload was thrown in. A
/// downcast to a `&JsError` finds it (ADR 0710).
pub const THROWN_VALUE: &str = "error";

/// The function of a `dyn`'s dictionary that drops what it holds, `$drop`,
/// if that has a destructor.
pub const DICTIONARY_DROP: &str = "$drop";

/// The property an enum's value says its variant in, `e.TAG` (ADR 0033),
/// where the enum doesn't name its own, `#[rust_js::tag = ".."]`.
pub const TAG: &str = "TAG";

/// The property a tuple variant holds its field `i` in, `_0`, `_1`, as in
/// ReScript (ADR 0033).
pub fn positional(i: usize) -> String {
    format!("_{i}")
}

#[derive(Clone, Default, PartialEq)]
pub struct Module {
    pub header: String,
    /// Whether the file's leading comments, the header's, are right above
    /// its code, no blank line between: then, of a module that imports
    /// nothing, none in its JS either, `// This is a generated file.` right
    /// above its first `export` (#419).
    pub header_on_code: bool,
    /// `"use client"`, before its imports (ADR 0192).
    pub directives: Vec<String>,
    /// Imports from JS modules, from `#[link_name = "module#path"]` (ADR 0028).
    pub packages: Vec<Package>,
    /// Named imports, one declaration per Rust module this one uses.
    pub imports: Vec<Import>,
    /// What it exports of another module, `export { a } from "./b.js"`, its
    /// `pub use`s, in the order they're written (ADR 0240, amended).
    pub reexports: Vec<Reexport>,
    /// What it imports of `@rust-js/std`'s helpers, sorted: those its
    /// prepared tree reads (ADR 0103), chosen before it's printed.
    pub helpers: Vec<&'static str>,
    /// Where they're imported from, of a module of `@rust-js/std` itself,
    /// std's bodies compiled: its `runtime.js`, `./runtime.js`; else the
    /// package, `@rust-js/std` (#1095).
    pub helpers_from: Option<String>,
    /// Its items, where its Rust has them, what's made when it's loaded
    /// after what that reads (ADR 0306).
    pub items: Vec<Item>,
    /// `export default page;`, after its functions (ADR 0192).
    pub default_export: Option<String>,
    /// Whether that's its function's declaration, `export default function
    /// noop() {}`, not `export default noop;` after it (ADR 0611).
    pub default_declared: bool,
    /// `export { getCurrentTime as unstable_now };`, a `pub use` of its own
    /// item by another name, as `(local, exported)`, after its functions (ADR 0596).
    pub aliases: Vec<(String, String)>,
    /// Where its code is in the Rust, what a comment after its last code ends (ADR 0621).
    pub code: Span,
    /// Each function that's now a declaration elsewhere, the variable or
    /// the field it only called (ADR 0610, #535), or a namespace's method
    /// that's a function or its re-export (ADR 1037, rd758), as
    /// `(function, declaration)`, where each is written: the comments
    /// before the function, its doc too, and in it are before the
    /// declaration, a re-export's statement where its `use`'s are.
    pub taken_into: Vec<(Span, Span)>,
    /// Each binding's (`true`) and type's item, of no JS where written (#643).
    pub unprinted: Vec<(Span, bool)>,
}

/// A module's item.
#[derive(Clone, PartialEq)]
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
                namespace.methods.iter().for_each(|m| m.visit_vars(read));
            }
            Item::Const(constant) => constant.value.visit_vars(read),
            Item::Statements(stmts) => visit_stmts(stmts, read),
            Item::Function(function) => function.visit_vars(read),
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

    /// Where it's first written in the crate's sources, of one that is.
    pub fn written_at(&self) -> Option<u32> {
        self.spans().iter().map(|span| span.lo).min()
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
#[derive(Clone, PartialEq)]
pub struct Namespace {
    pub name: String,
    /// Its constants, `ID: (s) => s`, properties before its methods (ADR 0410).
    pub constants: Vec<Const>,
    /// Its methods, as functions: the object's properties.
    pub methods: Vec<Function>,
    pub export: bool,
    /// A JS class's (ADR 0761), not an object's: the constructor function
    /// and its prototype.
    pub class: Option<Class>,
}

impl Namespace {
    /// The namespace `name` of a module's `namespaces`, made of nothing
    /// where there's none yet.
    pub fn of<'n>(namespaces: &'n mut Vec<Namespace>, name: &str) -> &'n mut Namespace {
        match namespaces.iter().position(|n| n.name == name) {
            Some(at) => &mut namespaces[at],
            None => {
                namespaces.push(Namespace {
                    name: name.to_owned(),
                    constants: Vec::new(),
                    methods: Vec::new(),
                    export: false,
                    class: None,
                });
                namespaces.last_mut().expect("just pushed")
            }
        }
    }
}

/// A JS class's methods (ADR 0761): which is its constructor, `function
/// Component(props) { this.props = props; }`, and which its prototype's,
/// `Component.prototype.setState = function (..) { .. }`; the rest are
/// the constructor's own, its statics. Its constants are its prototype's.
#[derive(Clone, Default, PartialEq)]
pub struct Class {
    pub constructor: String,
    pub prototype: Vec<String>,
    /// Its prototype's getters, `Object.defineProperty(QueenPosition.prototype,
    /// "leftDiagonal", { get() { .. } })` (#357).
    pub getters: Vec<String>,
    /// Each prototype method other classes of its module share, by its
    /// name, and theirs: one function assigned to each,
    /// `ReactDOMHydrationRoot.prototype.render =
    /// ReactDOMRoot.prototype.render = function (children) { .. };` (rd283).
    pub shared: Vec<(String, Vec<String>)>,
    /// Where its struct is declared: the comments before it, its `///`
    /// doc, are the constructor function's (ADR 0761), or an ES class's.
    pub declared: Span,
    /// The class it extends, a binding's, by the name its module imports
    /// it as, `Component` of react's: an ES class, `class Surface extends
    /// Component { .. }`, as `extends` and `super(props)` are written
    /// (rd331); `None` of a constructor function.
    pub extends: Option<String>,
    /// Whether it's an ES class, one that extends a binding's (rd331) or says
    /// `#[rust_js::class(es)]` (rd682), not a constructor function (ADR 0761).
    pub es: bool,
    /// Each prototype method, by its name, that's the function of its
    /// module it passes its parameters on to, which the prototype's
    /// property is where its JS is that call alone (`forwarded`),
    /// `ReactDOMHydrationRoot.prototype.unstable_scheduleHydration =
    /// scheduleHydration;` (rd715, ADR 0605 amended).
    pub forwarding: Vec<String>,
    /// The fields an ES class declares, `chunkName;`, each where its Rust field is (#568).
    pub fields: Vec<(Span, String)>,
    /// Whether the item after it, a `js::on_load!`'s one statement,
    /// replaces its prototype, `ReactPromise.prototype =
    /// Object.create(Promise.prototype)`: printed right after its
    /// constructor, before its methods are assigned to the prototype it
    /// makes, as upstream orders it (#448, ADR 0761 amended).
    pub prototype_set: bool,
}

impl Class {
    /// What `method`, one of its prototype's, is in the prototype's place
    /// where it's the function it passes its parameters on to
    /// (`forwarding`): that function, named, where its JS, as it's linked,
    /// is that call alone, of what it's given, in order, and nothing more,
    /// `scheduleHydration` of `function (target) { scheduleHydration(target);
    /// }`. Its result is the function's, as their Rust types are one, so a
    /// call that's a statement gives what the function gives. Any
    /// conversion its call needs, a box, a dictionary, is JS of its own,
    /// and it stays a function (rd715). Never an ES class's, whose
    /// methods are written in its body (`passed_on::forwarded_function`).
    pub fn forwarded(&self, method: &Function) -> Option<Expr> {
        if !self.forwarding.contains(&method.name) {
            return None;
        }
        let callee = passes_on(&method.params, &method.body, true).filter(|callee| callee.is_named())?;
        Some(Expr {
            span: method.span,
            ..callee.clone()
        })
    }
}

/// `const SIZE = 4096;`, maybe exported.
#[derive(Clone, PartialEq)]
pub struct Const {
    pub name: String,
    pub value: Expr,
    pub export: bool,
    /// `let`: a thread-local its module sets (ADR 0270).
    pub mutable: bool,
    /// Of a namespace thread-local's, its value's fields, each one of its
    /// module's exports, which it declares: `export const { render, act } =
    /// createReactNoop(..);`, its own `name` no JS's (ADR 1037, amended).
    pub fields: Fields,
    /// The whole `const` item.
    pub span: Span,
}

/// A namespace thread-local's exports, `Const::fields`: each its value's
/// field's name, and where its struct declares it, which the map gives
/// each name in the list; and where that struct is, the list's (rd610,
/// ADR 1037, amended).
#[derive(Clone, Default, PartialEq)]
pub struct Fields {
    pub names: Vec<(String, Span)>,
    pub declared: Span,
}

impl Fields {
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

#[derive(Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Pattern>,
    pub body: Vec<Stmt>,
    pub export: bool,
    /// `async function`: an `async fn` (ADR 0029).
    pub is_async: bool,
    /// `#[inline(never)]`: written `/** @noinline */` before it, and kept
    /// out of its callers' place (ADR 0649).
    pub noinline: bool,
    /// The whole `fn` item, and just its name.
    pub span: Span,
    pub name_span: Span,
    /// Where each parameter is written, one each, a comment before which
    /// is before it (ADR 0708, amended); none where that isn't known.
    pub param_spans: Vec<Span>,
    /// Declared in a body away from where it's written, a scope struct's
    /// method's (ADR 1043): set apart by a blank line before it and after
    /// it, as a person sets apart a function declared among statements.
    pub set_apart: bool,
}

impl Function {
    /// Each variable it reads: its parameters' defaults', `IIterator = {
    /// next: $next }`'s (ADR 0995), and its body's.
    pub fn visit_vars<'a>(&'a self, read: &mut dyn FnMut(&'a str)) {
        self.params
            .iter()
            .flat_map(Pattern::defaults)
            .for_each(|d| d.visit_vars(read));
        visit_stmts(&self.body, read);
    }
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

    /// `defaults`, to change.
    pub fn defaults_mut(&mut self) -> Vec<&mut Expr> {
        match self {
            Pattern::Name(_) | Pattern::Rest(_) => Vec::new(),
            Pattern::Defaulted(_, default) => vec![default],
            Pattern::Array(items) => items.iter_mut().flatten().flat_map(Pattern::defaults_mut).collect(),
            Pattern::Object(fields, _) => fields
                .iter_mut()
                .flat_map(|(_, p, default)| default.iter_mut().chain(p.defaults_mut()))
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

/// A case of a `switch` (`StmtKind::Switch`): its values, stacked, `case
/// 1: case 2:`, and its statements; and whether where they go on, they go
/// on to the next case's, as a guarded arm's case does where its guard
/// fails and the next arm names its constants (ADR 0905), not past the
/// `switch`. Where one falls, each way its statements go on to the next
/// has only tested: its case is `if (guard) { .. }` that leaves.
#[derive(Clone, PartialEq)]
pub struct Case {
    pub values: Vec<Expr>,
    pub body: Vec<Stmt>,
    pub falls: bool,
}

/// What a `for`'s update does to its variable: `name++`, `name--`, or
/// `name += value` and `name -= value`, which only preparation makes (ADR
/// 0974).
#[derive(Clone, PartialEq)]
pub enum Step {
    Up,
    Down,
    By(Op, Expr),
}

impl Step {
    /// The value it counts the variable by, of `name += value`.
    pub fn value(&self) -> Option<&Expr> {
        match self {
            Step::By(_, value) => Some(value),
            Step::Up | Step::Down => None,
        }
    }

    /// The value it counts the variable by, of `name += value`, to change.
    pub fn value_mut(&mut self) -> Option<&mut Expr> {
        match self {
            Step::By(_, value) => Some(value),
            Step::Up | Step::Down => None,
        }
    }
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
    /// `target = target op value`, as JS writes a string built up. A step
    /// of one is printed `x++` and `x--` (ADR 0631).
    Compound(Op, Expr, Expr),
    Expr(Expr),
    If(Expr, Vec<Stmt>, Option<Vec<Stmt>>),
    While {
        label: Option<String>,
        cond: Expr,
        body: Vec<Stmt>,
    },
    /// `do { .. } while (cond);`: a `while (true)` each of whose turns ends
    /// giving its variable its next value and testing it, preparation's
    /// alone, last (ADR 0978).
    DoWhile {
        label: Option<String>,
        body: Vec<Stmt>,
        cond: Expr,
    },
    /// `for (const name of iterable) { .. }`: a `for` over a sequence (ADR 0025),
    /// and `for (const [i, x] of ..)` for a tuple's parts. `let` if one's `mut`.
    /// Of `keys`, `for (const key in iterable)`, the keys JS's `for…in`
    /// visits of it (ADR 0719): a loop as the other is, its object read once.
    ForOf {
        label: Option<String>,
        pattern: Pattern,
        mutable: bool,
        iterable: Expr,
        body: Vec<Stmt>,
        keys: bool,
    },
    /// `for (let name = start; test; name++) { .. }`: a `for` over a range;
    /// or, `Step::Down`, `name--`, over a range or an array's indices from
    /// its end (ADR 0904); or, `Step::By`, `name += value`, a `while` that
    /// counts its variable last, preparation's (ADR 0974).
    For {
        label: Option<String>,
        name: String,
        start: Expr,
        test: Expr,
        step: Step,
        body: Vec<Stmt>,
    },
    /// `label: { .. }`, which a `break label` leaves: a let chain's (ADR 0048).
    Labeled(String, Vec<Stmt>),
    /// `switch (discriminant) { case 1: case 2: .. default: .. }`: a `match`
    /// of constants the program wrote (ADR 0629). Each case is its values,
    /// stacked, and its statements, which end it as an `if`'s branch's do:
    /// the printer writes `break;` where they'd go on, and a block where
    /// they declare, and leaves out a `default` of none; but of a case
    /// that falls through to the next, `// falls through` (ADR 0905). A
    /// `break` of a loop around it names the loop.
    Switch {
        discriminant: Expr,
        cases: Vec<Case>,
        default: Option<Vec<Stmt>>,
    },
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
    /// Where a block of only comments ends, `if c { // why }`'s: nothing
    /// runs, and the comments before its span are printed there, as
    /// written (ADR 0753).
    Comments,
}

/// What a `try`'s `catch` of a `Result` is given (`Expr::caught`): what
/// was thrown as it is, a JS call's `Err` (ADR 0035), or what unwound, the
/// payload `$unwound(e)` makes of it, `catch_unwind`'s (ADR 0660).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Caught {
    Thrown,
    Unwound,
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
    /// What its owner knows of it that its kind doesn't say, each a
    /// `Known`, kept where it's rebuilt of new parts.
    pub facts: Facts,
    /// The name the MIR temporary whose value it is, read in place, is
    /// given, `next` of `r.next()` (`temporaries_names`, ADR 0569): what a
    /// `const` of it a subject's reader makes is named (#769). None of one
    /// made of new parts.
    pub held_as: HeldAs,
}

/// `Expr::held_as`: the name a temporary is given, never part of what an
/// expression is, so two alike but for it are equal.
#[derive(Clone, Default)]
pub struct HeldAs(pub Option<std::sync::Arc<Held>>);

/// A temporary's name; `read` of a binding read as a property (#784).
#[derive(Clone)]
pub struct Held {
    pub name: String,
    pub read: bool,
}

impl PartialEq for HeldAs {
    fn eq(&self, _: &HeldAs) -> bool {
        true
    }
}

/// What the lowering knows of an expression, each of `Known`'s: a set, so
/// one fact never hides another (the review's P2, ADR 0889). Every read of
/// it is of what the compile hasn't forgotten (`known::seen`, ADR 1127):
/// `has`, the parts below and `==`, so a fact forgotten is one never said.
#[derive(Clone, Copy, Default)]
pub struct Facts(u64);

impl PartialEq for Facts {
    fn eq(&self, other: &Facts) -> bool {
        known::seen(self.0) == known::seen(other.0)
    }
}

impl Eq for Facts {}

impl Facts {
    /// Nothing known.
    pub const NONE: Facts = Facts(0);

    /// `known` alone.
    pub const fn of(known: Known) -> Facts {
        Facts(known.bit())
    }

    pub fn has(self, known: Known) -> bool {
        let has = known::seen(self.0) & known.bit();
        known::found(has);
        has != 0
    }

    /// These and `known`.
    pub fn with(self, known: Known) -> Facts {
        Facts(self.0 | known.bit())
    }

    /// These but `known`.
    pub fn without(self, known: Known) -> Facts {
        Facts(self.0 & !known.bit())
    }

    /// These and `other`'s.
    pub fn and(self, other: Facts) -> Facts {
        Facts(self.0 | other.0)
    }

    /// Those of these that are among `kept`, as a reader finds them.
    fn part(self, kept: &[Known]) -> Facts {
        let part = known::seen(self.0) & kept.iter().fold(0, |bits, known| bits | known.bit());
        known::found(part);
        Facts(part)
    }

    /// What an `Option`'s presence test says of the `None` it tests for,
    /// `Known::NoneUndefined` or `Known::NoneNull`, which a test made
    /// again of the same `Option`, its negation, keeps.
    pub fn of_presence(self) -> Facts {
        self.part(&[Known::NoneUndefined, Known::NoneNull])
    }

    /// The one `None` an `Option`'s is only ever, `Known::NoneUndefined`
    /// or `Known::NoneNull`, where these say one of them alone.
    pub fn only_none(self) -> Option<Known> {
        match (self.has(Known::NoneUndefined), self.has(Known::NoneNull)) {
            (true, false) => Some(Known::NoneUndefined),
            (false, true) => Some(Known::NoneNull),
            _ => None,
        }
    }

    /// Whether two `Option`s' `None` is one and the same, each only `null`,
    /// a WebIDL getter's `T | null`, or each only `undefined`: where `===`
    /// answers as `==` does, `node.firstChild === node.lastChild` (rd117).
    pub fn same_none(self, other: Facts) -> bool {
        self.only_none().is_some() && self.only_none() == other.only_none()
    }

    /// What evaluating it may do that its kind doesn't say, a getter's or
    /// a JS operator's of any value (`Known::Getter`), which the same
    /// evaluation made again keeps: `a != b` of `!(a == b)`, `+m` of `+m`
    /// (#462).
    pub fn of_evaluation(self) -> Facts {
        self.part(&[Known::Getter])
    }

    /// What a test says of its `if`, not of its value: a written `else`
    /// (`Known::Else`) and a guard (`Known::Guard`), which a test made again
    /// of the same truth keeps, `flags & 4` of `(flags & 4) !== 0`.
    pub fn of_statement(self) -> Facts {
        self.part(&[Known::Else, Known::Guard])
    }
}

/// A cross-module reference, resolved before preparation or printing.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Symbol {
    pub module: u32,
    pub export: String,
}

/// A piece of a string as its source writes it: its text, whether it's
/// written across lines, a template literal of them, and where it's
/// written, which a comment before it is before (ADR 0641).
#[derive(Clone, PartialEq)]
pub struct StrPiece {
    pub text: String,
    pub lines: bool,
    pub span: Span,
}

/// How a template's format string is written in its source, which its JS
/// writes as it is: one template literal, its line breaks as they are
/// where it's written across lines (ADR 0066); or `concat!`'s pieces, each
/// its own literal joined by `+`, a template literal where it holds a
/// value and otherwise a string (ADR 0641).
#[derive(Clone, PartialEq, Default)]
pub struct TemplateLayout {
    /// Whether its first piece, the whole of one not broken, is written
    /// across lines.
    pub lines: bool,
    /// Where its source breaks it into its next piece, in order.
    pub breaks: Vec<TemplateBreak>,
    /// Which of its values, in turn, its source writes across lines, a
    /// format argument's, whose JS may break the line after its `${` where
    /// it's too long for one, as Prettier does of one written so (ADR 0641,
    /// amended).
    pub across: Across,
}

/// Which of a template's values, in turn, are written across lines
/// (`TemplateLayout::across`): none, as of most, held in no room, so an
/// expression is no bigger for it.
#[derive(Clone, PartialEq, Default)]
pub struct Across(Option<Box<[bool]>>);

impl Across {
    /// Of each value in turn, whether it's written across lines.
    pub fn of(across: Vec<bool>) -> Across {
        Across(across.contains(&true).then(|| across.into_boxed_slice()))
    }

    /// Whether the `i`th value is written across lines.
    pub fn at(&self, i: usize) -> bool {
        self.all().get(i) == Some(&true)
    }

    /// Each value's, in turn; none, where none is.
    pub fn all(&self) -> &[bool] {
        self.0.as_deref().unwrap_or_default()
    }
}

impl TemplateLayout {
    /// One template literal on one line.
    pub fn is_plain(&self) -> bool {
        !self.lines && self.breaks.is_empty()
    }
}

/// How an object's source writes it: across lines, its first field on a
/// line after its `{`, as rustfmt lays out a struct literal too long for
/// one, which its JS keeps on lines of its own, as Prettier keeps an
/// object a person wrote so, where it has two fields or more, or its
/// literal a comment; or on one line, which its JS is on where it fits
/// (ADR 0835, amended). One no source writes, a representation's, is on
/// one.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct ObjectLayout {
    pub lines: bool,
    /// Whether its literal writes a comment between its braces, which
    /// keeps an object of one field across lines where the literal is
    /// (ADR 0835, amended, the user's decision).
    pub commented: bool,
    /// The keys whose `undefined` is kept: one a comment is written on in
    /// its literal, `v: None, // why`, `v: undefined, // why`, the comment's
    /// place, but an optional one's, absent until set (ADR 0280, amended);
    /// and one of a field that holds a cell, set
    /// later, `destroy: undefined`, the object's shape whole from its start
    /// (amended, the user's decision, 2026-10-27).
    pub kept: Vec<String>,
    /// What its literal writes of each field, where it does: boxed, as
    /// few objects are a literal's, so an expression is no larger.
    pub fields: Option<Box<WrittenFields>>,
}

impl ObjectLayout {
    /// Where `key`'s field is written, of those its literal says.
    pub fn start(&self, key: &str) -> Option<u32> {
        self.written(key).map(|written| written.lo)
    }

    /// Where `key`'s field's value is written, of those its literal says,
    /// from its start to its end.
    pub fn written(&self, key: &str) -> Option<Span> {
        let fields = self.fields.as_deref()?;
        fields.written.iter().find(|(k, _)| k == key).map(|&(_, at)| at)
    }

    /// `key`'s JSDoc, of those its literal's struct documents.
    pub fn doc(&self, key: &str) -> Option<&str> {
        let fields = self.fields.as_deref()?;
        fields.docs.iter().find(|(k, _)| k == key).map(|(_, doc)| doc.as_str())
    }
}

/// What a struct literal writes of its fields (`ObjectLayout::fields`).
#[derive(Clone, PartialEq, Eq, Default)]
pub struct WrittenFields {
    /// Each key's JSDoc, its struct's field's doc comment, where the crate
    /// writes one literal of the struct, the one object it describes (ADR
    /// 0444, amended): printed before the field.
    pub docs: Vec<(String, String)>,
    /// Where each key's field is written in its literal, its value,
    /// `create_instance: Box::new({ .. })`'s `Box::new(..)`, not where the JS
    /// value's own place is, inside it: what a comment and a blank line
    /// before the field are before (ADR 0708, amended), and what a comment
    /// past its end is after (rd321).
    pub written: Vec<(String, Span)>,
}

/// A piece of a template as its format string is written, `concat!`'s
/// (`Expr::template_pieces`): its texts and values, as `Expr::template`
/// takes them, which of its values are written across lines, and whether
/// it is.
pub type TemplatePiece = (Vec<String>, Vec<Expr>, Vec<bool>, bool);

/// Where a template's source breaks it into its next piece: in a text, by
/// its index, at a byte offset in it, as a value is never broken.
#[derive(Clone, PartialEq)]
pub struct TemplateBreak {
    pub text: usize,
    pub at: usize,
    /// Whether the piece it starts is written across lines.
    pub lines: bool,
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
    /// A string written in its source otherwise than as one literal on one
    /// line, as its pieces: `concat!("a ", "b")`'s, each its own literal
    /// joined by `+`, `"a " + "b"` (ADR 0641); and one written across
    /// lines, `r#"<div>` and on, a template literal with its line breaks,
    /// as a person writes a page. Never one piece on one line, a `Str`'s.
    Written(Vec<StrPiece>),
    Undefined,
    /// `undefined` given to JS, a binding or a JS function, which no pass
    /// leaves out of a call: what it calls may count its arguments,
    /// `Math.max(1, undefined)` (ADR 0330).
    GivenUndefined,
    /// Only to test against: `o != null` (ADR 0030).
    Null,
    Var(String),
    /// `this`: a class's own, its constructor's and its prototype's
    /// methods', Rust's `self` (ADR 0761).
    This,
    /// `super`, called first in the constructor of a class that extends one,
    /// `super(props)` (rd331).
    Super,
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
    /// `{ x: a, y: b }`: a struct (ADR 0020), and how its literal is
    /// written in its source, across lines or on one (ADR 0835).
    Object(Vec<Prop>, ObjectLayout),
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
    /// Where an item written in a body goes until the pipeline puts it
    /// there: such a function, one written in a function's body (ADR 0308),
    /// or a `const` (ADR 0031, amended); its item's index.
    ItemHole(u32),
    /// `import("./lint.js")`: the module, loaded when it's asked for (ADR 0304).
    Import(ImportFrom),
    /// `"../dep/element.js"`, what `require` loads, made from the importing file (ADR 0305).
    Specifier(ImportFrom),
    /// A drop function given to one of the crate's functions for its type
    /// parameter, `(item, index, drop)`: until the pipeline keeps it, where
    /// that function uses it, or leaves it out (ADR 0300). The item is its
    /// crate's number and its index there, one of a library's the crate
    /// compiles itself too (ADR 0914).
    DropArgument((u32, u32), u32, Box<Expr>),
    /// `(0, f)`: `f` read as a value, unbound, which a call gives no
    /// `this`, and which of `eval` is an indirect eval, run in the global
    /// scope: a binding's path `(0, eval)`, `js::eval`'s (ADR 1033, amended).
    Unbound(Box<Expr>),
    /// `await p`: `.await` (ADR 0029).
    Await(Box<Expr>),
    /// `<div className="hero">..</div>`, `<Counter initial={1} />` or `<>..</>` (ADR 0040).
    Jsx(Box<Jsx>),
    /// A regular expression literal, as written: `/^\p{White_Space}$/u` (ADR 0063).
    Regex(String),
    /// `\`Some(${x})\``: the texts around the values, as they read (one more
    /// than the values), and the values (ADR 0066); and how its format
    /// string is written, across lines or as `concat!`'s pieces.
    Template(Vec<String>, Vec<Expr>, TemplateLayout),
    /// `{ get value() { return x; }, set value(value) { x = value; } }`: a
    /// `&mut` kept, reading and writing its place each time (ADR 0099). The
    /// place is what's read then, not before: it's never taken out. And
    /// how its place stores what's written through it, `Storage`'s.
    Handle(Box<Expr>, Storage),
    /// `{ impl: d, get value() { return x; }, set value(value) { x = value; } }`:
    /// a `&mut dyn Trait` of a value JS can't change in place, its impl's
    /// dictionary and a handle on its place (ADR 0099), and how its place
    /// stores what's written through it, as a `Handle`'s (#968).
    Pair(Box<Expr>, Box<Expr>, Storage),
    /// `(target = value)`, an assignment's value, of a variable or a plain
    /// field: what preparation makes of a loop's first `const` that the test
    /// leaving it reads, `while (!(step = iterator.next()).done) { .. }` (ADR
    /// 0860).
    /// Nothing before preparation makes one, so lowering and tidy never see
    /// it.
    Assigned(Box<Expr>, Box<Expr>),
    /// `target++` or `target--` (`Op::Add`, `Op::Sub`), a step of one whose
    /// value, the one before the step, is read: what preparation makes of
    /// `const a = x; x = a + 1;`, `const a = x++;` (ADR 0948). Nothing
    /// before preparation makes one, so lowering and tidy never see it.
    Stepped(Op, Box<Expr>),
    /// `(a, b)`: each in turn, the last's value, as a condition holds a
    /// block's statements before its value, `check(f) || ((flags.value =
    /// f & ~32), false)` of `check(f) || { flags.set(f & !32); false }`
    /// (#523). Only tidy makes one, of what an `if`'s test does on its way
    /// (`logicals::skipped_tests`): of calls, and of `Assigned`s of a
    /// field, which set no binding.
    Sequence(Vec<Expr>),
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
    /// `[key]: value`: a field a symbol names, `key` that symbol's item
    /// as it's read, `[RESPONSE_SYMBOL]: response` (ADR 1051).
    Keyed(Expr, Expr),
    /// `[key](a) { .. }`: a `Keyed` field that's a closure reading no
    /// `this`, which preparation writes of it as of a `Field` (ADRs 0882,
    /// 1051).
    KeyedMethod(Expr, Expr),
    /// `...value`: copy every field of `value`.
    Spread(Expr),
    /// `get name() { .. }`, of an arrow that takes nothing: its body runs
    /// on each read, as a Rust constant's value is a new one at each use.
    Getter(String, Expr),
    /// `set name(v) { .. }`, of an arrow that takes one: a handle's on a
    /// place, which preparation writes of `Handle` (ADR 0099).
    Setter(String, Expr),
    /// `name(a, b) { .. }`, of an arrow that reads no `this`: a field that's
    /// a closure, which preparation writes of a `Field` (ADR 0882).
    Method(String, Expr),
}

#[derive(Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Neg,
    /// `+x`: any value as a number, as JS's `ToNumber` makes it, which
    /// throws of a `BigInt` and a symbol, `js::to_number`'s (rd331c).
    Plus,
    Not,
    BitNot,
    /// `typeof x`: what an untagged enum's variant is told by (ADR 0214).
    Typeof,
    /// `delete x.key`: the property gone, `js::delete`'s (ADR 0697).
    Delete,
}

impl UnaryOp {
    /// Whether it changes what it's of, as `delete` does: an expression of
    /// it is never pure, whatever it's of.
    pub fn writes(self) -> bool {
        matches!(self, UnaryOp::Delete)
    }
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

    /// A constant's array of literals is known so, `Known::Literal`, and
    /// the walks don't enter it; an array holding a variable isn't (#1096).
    #[test]
    fn a_constants_array_of_literals_is_known_literal() {
        let literals = Expr::literal_array(vec![
            Expr::int(1),
            Expr::literal_array(vec![Expr::str("a")]),
            Expr::undefined(),
        ]);
        assert!(literals.facts.has(Known::Literal));
        assert!(!literals.has_effects() && literals.reads_only_vars());
        let mut read = Vec::new();
        literals.visit_vars(&mut |name| read.push(name));
        assert!(read.is_empty());
        let held = Expr::literal_array(vec![Expr::int(1), Expr::var("x")]);
        assert!(!held.facts.has(Known::Literal));
        let mut read = Vec::new();
        held.visit_vars(&mut |name| read.push(name));
        assert_eq!(read, ["x"]);
    }

    fn module_of(value: Expr) -> Module {
        Module {
            items: vec![Item::Statements(vec![StmtKind::Expr(value).at(Span::NONE)])],
            ..Module::default()
        }
    }

    // A call is JS's own only where lowering built it so (`Known::BuiltIn`):
    // a binding's method, the crate's `Grid.map`, or a global's name the
    // crate's `String` has, may do anything, whatever its name (ADR 0890,
    // #1663, #1674).
    #[test]
    fn only_a_built_in_call_is_js_own() {
        let own = Expr::builtin(Expr::var("parts"), "join", vec![Expr::str(", ")]);
        assert!(own.only_reads() && !own.runs_program_code());
        let global = Expr::call(Expr::global("String"), vec![Expr::var("n")]).built_in();
        assert!(global.only_reads() && !global.runs_program_code());
        for named in [
            Expr::call(Expr::member(Expr::var("store"), "join"), Vec::new()),
            Expr::call(Expr::member(Expr::var("Grid"), "map"), vec![Expr::var("g")]),
            Expr::call(Expr::var("String"), vec![Expr::var("n")]),
        ] {
            assert!(!named.only_reads() && named.runs_program_code());
        }
    }

    // A dictionary's method is the program's, whatever its name: `join` of
    // a crate's trait is no array's (ADR 0900).
    #[test]
    fn a_dictionarys_method_runs_program_code() {
        let given = Expr::call(
            Expr::member(Expr::var("TStore").evidence(), "join"),
            vec![Expr::var("t")],
        );
        assert!(!given.only_reads() && given.runs_program_code());
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
            noinline: false,
            span: Span::NONE,
            name_span: Span::NONE,
            param_spans: Vec::new(),
            set_apart: false,
        };
        let value = Expr::array(vec![outer, Expr::new(ExprKind::Function(Box::new(function)))]);
        let mut items = vec![Item::Const(Const {
            name: "c".into(),
            value,
            export: false,
            mutable: false,
            fields: Default::default(),
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
            noinline: false,
            span: Span::NONE,
            name_span: Span::NONE,
            param_spans: Vec::new(),
            set_apart: false,
        };
        let mut names = IndexSet::default();
        names_in(&[StmtKind::Function(Box::new(inner)).at(Span::NONE)], &mut names);
        for name in ["inner", "x", "y"] {
            assert!(names.contains(name), "{name}");
        }
    }
}
