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
pub mod result;
mod stmt_walk;
mod walk;
mod writes;

pub use self::{
    applied::{applied_blocks, applied_call, applied_tried, returns},
    expr_walk::*,
    stmt_walk::*,
    walk::*,
    writes::Writes,
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
/// How a place stores what's written to it, which a handle on it keeps,
/// wherever it's written through (`ExprKind::Handle`, ADR 0791).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Storage {
    /// What's written itself.
    AsGiven,
    /// A `#[rust_js::nullable]` field's `Option`, or `Cell` of one: its
    /// `None`, `null` (ADRs 0275, 0772).
    NullForNone,
}

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

pub struct Module {
    pub header: String,
    /// `"use client"`, before its imports (ADR 0192).
    pub directives: Vec<String>,
    /// Imports from JS modules, from `#[link_name = "module#path"]` (ADR 0028).
    pub packages: Vec<Package>,
    /// Named imports, one declaration per Rust module this one uses.
    pub imports: Vec<Import>,
    /// What it exports of another module, `export { a } from "./b.js"`, its
    /// `pub use`s, in the order they're written (ADR 0240, amended).
    pub reexports: Vec<Reexport>,
    /// What it imports of `@rust-js/runtime`, sorted: the helpers its
    /// prepared tree reads (ADR 0103), chosen before it's printed.
    pub helpers: Vec<&'static str>,
    /// Its items, where its Rust has them, what's made when it's loaded
    /// after what that reads (ADR 0306).
    pub items: Vec<Item>,
    /// `export default page;`, after its functions (ADR 0192).
    pub default_export: Option<String>,
    /// Whether that's its function's declaration, `export default function
    /// noop() {}`, not `export default noop;` after it (ADR 0611).
    pub default_declared: bool,
    /// `export { getCurrentTime as unstable_now };`, a `pub use` of its own
    /// item by another name, as `(local, exported)`, after its functions
    /// (ADR 0596).
    pub aliases: Vec<(String, String)>,
    /// Where its code is in the Rust, what a comment after its last code
    /// ends (ADR 0621).
    pub code: Span,
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
    /// A JS class's (ADR 0761), not an object's: the constructor function
    /// and its prototype.
    pub class: Option<Class>,
}

/// A JS class's methods (ADR 0761): which is its constructor, `function
/// Component(props) { this.props = props; }`, and which its prototype's,
/// `Component.prototype.setState = function (..) { .. }`; the rest are
/// the constructor's own, its statics. Its constants are its prototype's.
pub struct Class {
    pub constructor: String,
    pub prototype: Vec<String>,
    /// Where its struct is declared: the comments before it, its `///`
    /// doc, are the constructor function's (ADR 0761).
    pub declared: Span,
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

/// One `pub use` statement's `export { a, b as c } from "./m.js";` (ADR
/// 0240), where it's written, so its comments are before it: each `(export,
/// alias)`, `*` the module's namespace, `export * as ns from`.
pub struct Reexport {
    pub named: Vec<(String, String)>,
    pub from: String,
    pub span: Span,
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
    /// `#[inline(never)]`: written `/** @noinline */` before it, and kept
    /// out of its callers' place (ADR 0649).
    pub noinline: bool,
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

/// A case of a `switch` (`StmtKind::Switch`): its values, stacked, `case
/// 1: case 2:`, and its statements; and whether where they go on, they go
/// on to the next case's, as a guarded arm's case does where its guard
/// fails and the next arm names its constants (ADR 0899), not past the
/// `switch`. Where one falls, each way its statements go on to the next
/// has only tested: its case is `if (guard) { .. }` that leaves.
#[derive(Clone, PartialEq)]
pub struct Case {
    pub values: Vec<Expr>,
    pub body: Vec<Stmt>,
    pub falls: bool,
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
    /// or, `down`, `name--`, over a range or an array's indices from its
    /// end (ADR 0900).
    For {
        label: Option<String>,
        name: String,
        start: Expr,
        test: Expr,
        down: bool,
        body: Vec<Stmt>,
    },
    /// `label: { .. }`, which a `break label` leaves: a let chain's (ADR 0048).
    Labeled(String, Vec<Stmt>),
    /// `switch (discriminant) { case 1: case 2: .. default: .. }`: a `match`
    /// of constants the program wrote (ADR 0629). Each case is its values,
    /// stacked, and its statements, which end it as an `if`'s branch's do:
    /// the printer writes `break;` where they'd go on, and a block where
    /// they declare, and leaves out a `default` of none; but of a case
    /// that falls through to the next, `// Fallthrough` (ADR 0899). A
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
}

/// What the lowering knows of an expression, each of `Known`'s: a set, so
/// one fact never hides another (the review's P2, ADR 0889).
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct Facts(u16);

impl Facts {
    /// Nothing known.
    pub const NONE: Facts = Facts(0);

    /// `known` alone.
    pub const fn of(known: Known) -> Facts {
        Facts(1 << known as u16)
    }

    pub fn has(self, known: Known) -> bool {
        self.0 & Facts::of(known).0 != 0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// These and `known`.
    pub fn with(self, known: Known) -> Facts {
        Facts(self.0 | Facts::of(known).0)
    }
}

/// What the lowering knows of an expression that its kind doesn't say:
/// the one place a later pass reads it from, not from where it's written.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Known {
    /// A call of a method of a `str` or a slice, as Rust's types tell,
    /// `s.startsWith(t)`: never a crate's or a binding's of the same
    /// name, `P.at(p)`, which may do anything.
    Read,
    /// A binding's read of a property, `get x`, `get X.y` or `get []`,
    /// `js::get`'s, or of the method a binding calls on a JS value,
    /// `obj.send` of `obj.send(a)`: a getter may answer it, which may do
    /// anything, and may give another value read again (ADRs 0672, 0750).
    Getter,
    /// A binding's write of a property, `set x` or `set X.y`, the target
    /// of its assignment: a setter may answer it, which may do anything,
    /// throw too, so the write never only makes a value (ADR 0660,
    /// amended).
    Setter,
    /// A slice's items in a range the body's ranges show in bounds,
    /// `items.slice(a, b)`: an array made here, which nothing else holds,
    /// as `$slice(..)` is (ADR 0806).
    Sliced,
    /// A set's items in its order, `Array.from(set)` of a `HashSet`'s JS
    /// `Set`, or a class of it, `$KeySet`: an array made here, whose
    /// `forEach` of a callback of the item alone is the set's own,
    /// `set.forEach((x) => ..)`, which steps the same items in the same
    /// order, the set unchanged while Rust borrows it (ADR 0877).
    SetItems,
    /// Never `undefined`, as Rust's types tell, though maybe `null`: a
    /// `Some`'s value, `map`'s, or what's set through a variable holding a
    /// cell's handle that MIR shows is `null` or a value already,
    /// `make(context)` of `set(Some(make(context)))`. A nullable place
    /// stores it as it is (`Storage::written`, ADR 0841).
    Defined,
    /// A read of a thread-local made where it's first used, `key()`
    /// (ADR 0037, amended): making it runs its initializer, which Rust runs
    /// inside the call that uses it, after what that call is given, so it's
    /// read there, not before them; read again, it's what it made.
    /// `LocalKey::set` of one not made yet makes it of the value given,
    /// `key.initWith?.(cell)` (#252).
    LocalRead,
    /// A test of an `Option` of a value never falsy by its truth, `!!o` or
    /// `!o` (`Expr::presence`, ADR 0298, amended): true exactly where `o`
    /// is set, or isn't, as `o != null` is, which `null_test` reads it as.
    Presence,
    /// A `Rev`, an `Enumerate` or a `Map` made in place, the recognizers'
    /// JS iterator, not std's struct, as the call that takes it says
    /// (`mir_iter_call`, ADR 0871): a step past that call, which takes its
    /// form apart, reads it here, not from its form.
    InPlace,
    /// One of the program's functions, its own or a library's, by its
    /// name, `noop` (`items.rs::fn_ref`): a binding nothing assigns, the
    /// same function wherever and whenever it's read (`Expr::is_fixed`,
    /// ADR 0693, amended).
    Item,
    /// A read of another module's variable, an imported `export let x`
    /// that its module sets (ADR 0270, amended): what it is where it's
    /// read, as ES's live binding, never another name's that read it
    /// before.
    Live,
    /// A binding's method, `obj.send` of `obj.send(a)`: JS the program
    /// declared, which may do anything, whatever its name, `join` too (the
    /// review's P2, ADR 0890), never one of JS's own that only read.
    Binding,
    /// A dictionary the function was given, `TStore` of `TStore.join(t)`:
    /// its methods are the program's, whatever their names, never JS's own
    /// that only read (ADR 0900).
    Evidence,
    /// A call that throws only where what it's given does: std's print,
    /// `console.log(text)`, whose stream never fails (ADRs 0132, 0901); or
    /// a function each call of which does, one of the crate's that never
    /// unwinds, `loudDrop_drop` (ADR 0902).
    NeverThrows,
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
/// object a person wrote so; or on one line, which its JS is on where it
/// fits (ADR 0835). One no source writes, a representation's, is on one.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct ObjectLayout {
    pub lines: bool,
    /// The keys a comment is written on in its literal, `v: Cell::new(None),
    /// // why`, whose `undefined` is kept, `v: undefined, // why`: the
    /// comment's place (ADR 0280, amended).
    pub commented: Vec<String>,
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
    /// dictionary and a handle on its place (ADR 0099).
    Pair(Box<Expr>, Box<Expr>),
    /// `(name = value)`, an assignment's value: what preparation makes of a
    /// loop's first `const` that the test leaving it reads, `while (!(step =
    /// iterator.next()).done) { .. }` (ADR 0860). Nothing before
    /// preparation makes one, so lowering and tidy never see it.
    Assigned(String, Box<Expr>),
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
    /// `name(a, b) { .. }`, of an arrow that reads no `this`: a field that's
    /// a closure, which preparation writes of a `Field` (ADR 0882).
    Method(String, Expr),
}

#[derive(Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Neg,
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
            default_declared: false,
            aliases: Vec::new(),
            code: Span::NONE,
        }
    }

    // A binding's method is never JS's own that only reads, whatever its
    // name, and stays a getter's read beside it (ADR 0890).
    #[test]
    fn a_bindings_method_runs_what_it_may() {
        let own = Expr::call(Expr::member(Expr::var("parts"), "join"), vec![Expr::str(", ")]);
        assert!(own.only_reads() && !own.runs_program_code());
        let bound = Expr::call(Expr::member(Expr::var("store"), "join").binding(), Vec::new());
        assert!(!bound.only_reads() && bound.runs_program_code());
        let both = Expr::member(Expr::var("store"), "join")
            .binding()
            .answered_by_a_getter();
        assert!(both.facts.has(Known::Binding) && both.facts.has(Known::Getter));
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
            noinline: false,
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
