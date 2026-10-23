//! Lowering: rustc's MIR  ──►  our JS AST.
//!
//! Each body is lowered from its MIR (`mir`, ADR 0364): what runs, in the
//! order it runs, made structured JS. Its THIR, copied before borrowck
//! steals it, is read beside it for shape and facts: names, spans, JSX,
//! iterator chains, drops (`body_queries`, `drops`).
//!
//! Semantics follow Rust with `overflow-checks = off` (the release profile):
//! integer arithmetic wraps. Division by zero and `MIN / -1` still panic,
//! because Rust panics on those in every profile.
//!
//! This module owns the state lowering shares: `Body`, `FnCx` and its
//! `Locals`, and nested bodies. `bodies` owns function/nested-body
//! lifecycle, `patterns` destructuring, and `places` reads and checked
//! indexes. `body_queries` has no emission context; its cached facts belong
//! to `Body`.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use rustc_ast::{LitKind, StrStyle};
use rustc_hir::def::{CtorKind, DefKind};

use rustc_middle::thir::{ExprId, LocalVarId, Pat, PatKind, Thir};

use rustc_middle::ty::{self, GenericArgsRef, Ty, TyCtxt, TypeVisitableExt};
use rustc_span::def_id::{CRATE_MOD_ID, DefId, LocalDefId, LocalModId};
use rustc_span::{ErrorGuaranteed, SourceFile, Span};

use crate::js::{self, Expr, Prop, Stmt, StmtKind};

mod aggregates;
mod analysis;
mod binding_calls;
mod bindings;
mod bodies;
mod body_queries;
mod calls;
mod combinators;
mod copies;
mod declarations;
mod display;
mod drops;
mod format_args;
mod format_spec;
mod items;
mod iterators;
mod jsx;
mod jsx_api;
mod library;
pub(crate) mod liveness;
mod loops;
mod mir;
mod mut_refs;
mod ordering;
mod patterns;
mod pipeline;
mod places;
mod plain;
mod pointers;
mod recognition;
mod refusal;
mod representation;
mod serde;
mod shipped;
mod shortcuts;
mod sources;
pub mod std_coverage;
mod std_impls;
mod std_types;
mod support;
mod traits;
mod unions;
mod untagged;

use crate::names::{fresh_in, js_ident};
use crate::program::TestFn;
pub use analysis::{collect_bodies, collect_initializers, collect_mir, collect_ranges, collect_std_bodies};
/// Whether each `#[rust_js::name]` gives a name its item hasn't anyway, of
/// a crate `--rustc` compiles for its metadata too (ADR 0573).
pub use bindings::given_names_differ;
use bindings::{Export, is_binding};
pub use pipeline::lower_crate;
use recognition::{Std, TypeFact};
use representation::{Num, const_js, eval_const, f32_literal, static_value};
pub use serde::{SerdeAttributes, attributes as serde_attributes};

use body_queries::strip;

type R<T> = Result<T, ErrorGuaranteed>;

/// A function's THIR, copied out of rustc before borrowck steals it: read
/// for the shape and facts its MIR is lowered with.
pub struct Body<'tcx> {
    def_id: DefId,
    thir: Thir<'tcx>,
    expr: ExprId,
    facts: body_queries::BodyFacts,
    /// Its MIR, which the body is lowered from where it's read (ADR 0364).
    mir: Option<mir::Mir<'tcx>>,
}

impl Body<'_> {
    /// Its root expression, of its THIR; none of a body read from its MIR
    /// alone, a std one's (ADR 0561), whose THIR is empty.
    pub(crate) fn root(&self) -> Option<ExprId> {
        (!self.thir.exprs.is_empty()).then_some(self.expr)
    }
}

/// Where a function or a `const` ends up in the JS: its module's file,
/// under this name.
struct FnInfo {
    module: LocalModId,
    name: String,
    /// A method's type's object of methods (ADR 0047): `Counter` for `Counter.tick`.
    owner: Option<String>,
}

/// The module `def_id`'s JS is written into: the crate's own item's, the
/// module it's in; a std body's (ADR 0561), the crate's
/// module named as std's is, `iter` of `core::iter::Peekable`'s. The one
/// answer to where a body goes.
pub(crate) fn body_module(tcx: TyCtxt<'_>, def_id: DefId) -> LocalModId {
    if let Some(local) = def_id.as_local() {
        return tcx.parent_module_from_def_id(local);
    }
    recognition::std_home_module(tcx, def_id).unwrap_or_else(|| {
        panic!(
            "a crate given std's bodies has no module for {}",
            tcx.def_path_str(def_id)
        )
    })
}

/// A module's path below the crate root, e.g. `["math", "stats"]`.
fn module_path(tcx: TyCtxt<'_>, module: LocalModId) -> Vec<String> {
    if module == CRATE_MOD_ID {
        return Vec::new();
    }
    let parent = tcx.parent_module_from_def_id(module.to_local_def_id());
    let mut path = module_path(tcx, parent);
    let name = tcx.item_name(module.to_def_id());
    // A module in a block, as bitflags' macro writes in a function, is one of
    // its own, though another of its parent's has its name: numbered in the
    // order they're written, after the one the parent declares, `names$1`.
    let in_block = |m: LocalModId| tcx.def_kind(tcx.local_parent(m.to_local_def_id())) != DefKind::Mod;
    let mut namesakes: Vec<LocalModId> = tcx
        .hir_crate_items(())
        .definitions()
        .filter(|&id| tcx.def_kind(id) == DefKind::Mod)
        .map(LocalModId::new_unchecked)
        .filter(|&m| {
            m != CRATE_MOD_ID
                && tcx.item_name(m.to_def_id()) == name
                && tcx.parent_module_from_def_id(m.to_local_def_id()) == parent
        })
        .collect();
    namesakes.sort_by_key(|&m| (in_block(m), m.to_local_def_id().local_def_index));
    match namesakes.iter().position(|&m| m == module) {
        Some(at) if at > 0 => path.push(crate::names::given_apart(&name.to_string(), at)),
        _ => path.push(name.to_string()),
    }
    path
}

/// The `.rs` file a module's code lives in: its own file for `mod foo;`,
/// the parent's file for an inline `mod foo { .. }`.
fn module_file(tcx: TyCtxt<'_>, module: LocalModId) -> Arc<SourceFile> {
    let inner = tcx.hir_get_module(module).0.spans.inner_span;
    tcx.sess.source_map().lookup_source_file(inner.lo())
}

/// A function item's type: the function, and its generic args. rustc 1.99
/// binds the args; a body's never have bound vars, as rustc's own MIR
/// building reads them.
pub(crate) fn fn_def<'tcx>(ty: Ty<'tcx>) -> Option<(DefId, GenericArgsRef<'tcx>)> {
    match *ty.kind() {
        ty::FnDef(def_id, args) => Some((def_id, args.no_bound_vars().expect("a body's function item, unbound"))),
        _ => None,
    }
}

pub struct LoweredFn {
    pub function: js::Function,
    pub jsx: bool,
    dependencies: Dependencies,
}

struct Var {
    /// Usually the variable's JS name. An immutable binding into a pattern
    /// can instead just name the place it matched: `let (q, r) = t` makes
    /// `q` mean `t[0]`, with no JS variable at all.
    place: Expr,
    mutable: bool,
}

/// What a body knows of its variables: what each is in JS, and what its
/// lowering found of some. A closure's body and a coroutine's share their
/// enclosing body's, since they name its variables; a trait's default body,
/// copied into an impl, has its own (`enter_body`). Something known of a
/// variable, by its `LocalVarId`, goes here, and so goes with it.
#[derive(Default)]
struct Locals {
    vars: HashMap<LocalVarId, Var>,
    /// A props pattern's rest, JS's `...props`, by its name: the keys the
    /// pattern names, which it never holds (ADRs 0195, 0205).
    rests: HashMap<String, Vec<String>>,
    /// What lowering a body from its MIR found of its variables (ADR 0364),
    /// mir.rs's, which the rest ask of it.
    mir: mir::Found,
}

/// How a Rust struct or tuple type is represented in JS (ADR 0020).
enum Shape<'tcx> {
    /// A struct with named fields: `{ x: 1, y: 2 }`.
    Object(Vec<(String, Ty<'tcx>)>),
    /// A tuple or tuple struct: `[1, 2]`.
    Array(Vec<Ty<'tcx>>),
    /// Anything else: numbers, `bool`, unit and unit structs (`undefined`), enums.
    Other,
}

/// A body lowered inside the one being lowered, and what it starts from
/// (`enter_body`). Each has its own THIR and owner; what
/// else isn't said here, it shares with the enclosing body.
enum Nested<'tcx> {
    /// A closure's, an arrow: the enclosing `Locals` and captures, and
    /// these names to start from.
    Closure { names: HashSet<String> },
    /// A trait's default body, copied into an impl (ADR 0049): `Locals` of
    /// its own, a copy of the names, and the impl's evidence, arguments,
    /// typing environment and drops.
    Default {
        evidence: Vec<(ty::TraitRef<'tcx>, Expr)>,
        self_args: ty::GenericArgsRef<'tcx>,
        typing_env: ty::TypingEnv<'tcx>,
        /// The drop, by name, for each of its trait's type parameters the
        /// impl's argument for has one (ADR 0098), and those rust-js can't
        /// make one for, and why.
        drops: HashMap<u32, String>,
        unsupported: HashMap<u32, (Ty<'tcx>, &'static str)>,
        /// And of a std associated type of one, `Self::Item`, its slot's
        /// (`drop_slot`), the impl's (ADR 0421), and each it could be given
        /// (ADR 0489).
        items: HashMap<ty::AliasTy<'tcx>, u32>,
        slots: HashSet<ty::AliasTy<'tcx>>,
    },
}

/// What a nested body took of the enclosing one's state, given back when
/// it's left (`leave_body`): what every body has of its own, and what its
/// kind has too, as `Nested` says.
struct Enclosing<'a, 'tcx> {
    thir: &'a Thir<'tcx>,
    body_facts: &'a body_queries::BodyFacts,
    body_owner: DefId,
    drops: drops::EnclosingDrops<'tcx>,
    kind: EnclosingKind<'tcx>,
}

/// What each kind of nested body took, by `Nested`'s kinds.
enum EnclosingKind<'tcx> {
    Closure { names: HashSet<String> },
    Default(Box<ItemScope<'tcx>>),
}

/// What a trait's default body, copied into an impl, has of its own that
/// another body lowers with: the item's names, `Locals`, evidence, arguments,
/// typing environment and drops.
struct ItemScope<'tcx> {
    names: HashSet<String>,
    locals: Locals,
    given: traits::GivenScope<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
}

/// Immutable analysis inputs shared by function lowering.
struct CrateFacts<'a, 'tcx> {
    sources: &'a sources::CapturedSources,
    mutated: &'a HashSet<Ty<'tcx>>,
    /// The `Rc`s counted (ADR 0320).
    counted: &'a analysis::Counted<'tcx>,
    /// What each body may borrow, of its cells (ADR 0580).
    cell_borrows: &'a analysis::CellBorrows<'tcx>,
    /// Whether a function holding a value with a destructor is made a `dyn
    /// Fn` (ADR 0521).
    closure_facts: &'a analysis::ClosureFacts,
    /// The types the crate clones (ADR 0525).
    cloned: &'a HashSet<Ty<'tcx>>,
    /// The kinds of std's adapters explicit wherever the crate makes one
    /// (ADR 0542): those its types hold, and those with a count it clones.
    explicit_adapters: &'a HashSet<DefId>,
    /// The map and filter types the crate steps from their back (ADR 0545).
    back_stepped: &'a HashSet<Ty<'tcx>>,
    /// The trait constants generic code reads (ADR 0106).
    generic_consts: &'a HashSet<DefId>,
    changed_vecs: &'a HashSet<Ty<'tcx>>,
    /// Each generic function's type parameters it's given a drop for (ADR 0098).
    drop_params: &'a HashMap<DefId, Vec<u32>>,
    /// The type parameters whose `Copy` bound takes a copy function (ADR
    /// 0289), and whose `Iterator` its dictionary (ADR 0411).
    given: traits::Given<'a>,
    /// Each generic function's type parameters it's given a fact of (ADR 0145).
    type_facts: &'a HashMap<DefId, Vec<(u32, TypeFact)>>,
    /// The functions that take their caller's location (ADR 0427).
    caller_locations: &'a HashSet<DefId>,
    /// The functions that may return `Err(fmt::Error)`, and whether any of
    /// the crate's or its libraries' may, as a generic `T`'s then may (ADR 0187).
    failing: &'a HashSet<DefId>,
    any_failing: bool,
    /// Whether a `Vec`'s capacity is kept, as the crate or a library it uses
    /// asks it (ADR 0419).
    keeps_capacity: bool,
    /// The type parameters each of the crate's functions takes no destructor
    /// of, and the calls of them to check once all are lowered (ADR 0190).
    no_drops: &'a RefCell<HashMap<DefId, BTreeSet<u32>>>,
    drop_checks: &'a RefCell<Vec<drops::DropCheck>>,
    /// What the functions that take only the drops they use did with them (ADR 0300).
    drop_uses: &'a RefCell<drops::DropUses>,
    closures: &'a HashMap<DefId, &'a Body<'tcx>>,
    bodies: &'a HashMap<DefId, &'a Body<'tcx>>,
    fns: &'a HashMap<DefId, FnInfo>,
    imports: &'a HashMap<LocalModId, HashMap<Export, String>>,
    /// The exports read through `require(module)`, not imported (ADR 0305).
    required: &'a HashSet<Export>,
    /// Where a path reads a binding through the module that holds it, by
    /// the path's span (ADR 0633).
    held_reads: &'a HashMap<Span, bindings::HeldRead>,
    /// What the crate's libraries export (ADR 0100).
    foreign: &'a library::Foreign<'a, 'tcx>,
    /// Is this crate compiled as a library, for others to use (ADR 0100)?
    library: bool,
    trait_impls: &'a [DefId],
    /// `#[serde(..)]` attributes, from the expanded crate (ADR 0077).
    serde_attrs: &'a serde::SerdeAttributes,
    /// Do its `Debug` functions take whether to be pretty (ADR 0137)?
    pretty_debug: bool,
    /// Does it give a placeholder's options to its writers and dictionaries
    /// (ADR 0058)?
    format_options: bool,
    /// Whether the crate asks `{:x?}` of anything: then an integer a writer
    /// shows is in hex where its options say (ADR 0559).
    debug_hex: bool,
    /// The `RefCell`s a field holds as their value, never counted (ADR
    /// 0362), found once the first is asked of.
    plain_ref_cells: &'a std::cell::OnceCell<HashSet<Ty<'tcx>>>,
    /// The functions a block makes and gives, each a named function
    /// expression there (ADR 0296).
    named_expressions: &'a HashSet<DefId>,
    /// The functions written in a function's body, by the block they're
    /// in, each a function declaration there (ADR 0308).
    local_functions: &'a HashMap<DefId, rustc_span::Span>,
}

impl CrateFacts<'_, '_> {
    /// Do its writers take their `Formatter`'s options after their
    /// arguments, whether to be pretty or a placeholder's (ADRs 0058, 0137)?
    fn writers_take_options(&self) -> bool {
        self.pretty_debug || self.format_options
    }
}

/// Dependencies recorded by one function (including copied trait bodies and
/// closures), returned with its JS. They never mutate the crate's inputs.
#[derive(Default)]
struct Dependencies {
    references: HashSet<(LocalModId, DefId)>,
    package_uses: HashSet<(LocalModId, Export)>,
    uses: Vec<(DefId, DefId)>,
    /// Each binding a module reads through another of the crate's that
    /// holds it, by its `const`'s name there and whether it's that
    /// module's default (ADR 0633).
    held: HashSet<(LocalModId, LocalModId, String, bool, DefId)>,
}

struct FnCx<'a, 'tcx> {
    krate: &'a CrateFacts<'a, 'tcx>,
    dependencies: RefCell<Dependencies>,
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    /// What the generic item being lowered is given, besides its arguments.
    given: Given<'tcx>,
    /// In a generic type's derived `serialize` or `deserialize` (ADR
    /// 0080): each type parameter, and the parameter that writes or reads it.
    codecs: Vec<(Ty<'tcx>, String)>,
    thir: &'a Thir<'tcx>,
    body_facts: &'a body_queries::BodyFacts,
    /// The module receiving this function and its recorded dependencies.
    module: LocalModId,
    /// What this body knows of its variables.
    locals: Locals,
    /// JS names already taken in this function.
    names: HashSet<String>,
    /// Those taken by the module: its functions, imports and globals.
    module_names: &'a HashSet<String>,
    /// Whether this function makes JSX.
    jsx: bool,
    /// What writing to a `Formatter` knows (ADRs 0054, 0137).
    writing: display::Writing,
    /// The recursive types being cloned, and the function each one's clone
    /// is (`clone_value`), which a clone inside it calls.
    cloning: Vec<(Ty<'tcx>, String)>,
    /// The recursive types being ordered, partially or not, and the
    /// function each one's order is (`cmp_value`), which an order inside it
    /// calls.
    ordering: Vec<(Ty<'tcx>, bool, String)>,
    /// The item being lowered: what `fn_ref` records as using its target.
    item: DefId,
    /// What walks of types found, each walked once, under each typing
    /// environment: what a type holds depends on the bounds in force, as a
    /// copied default's `T::Item` is a number and its sibling's a `Vec`.
    walks: RefCell<HashMap<ty::TypingEnv<'tcx>, Rc<TypeWalks<'tcx>>>>,
    /// What's dropped, and where (ADR 0098).
    drop_state: drops::DropState<'tcx>,
    /// Whose body `thir` is: its scope tree says where temporaries end.
    body_owner: DefId,
}

/// What the generic item being lowered is given, besides its arguments.
#[derive(Default)]
struct Given<'tcx> {
    /// Its dictionaries, for its bounds (ADR 0049).
    evidence: Vec<(ty::TraitRef<'tcx>, Expr)>,
    /// Each const parameter it's given, by its index: `N` (ADR 0107).
    const_params: Vec<(u32, Expr)>,
    /// Each fact it's given, `TSize` (ADR 0145), of the type it's of: a type
    /// parameter, or a std associated type of one, `IItem` (ADR 0466).
    type_facts: Vec<(Ty<'tcx>, TypeFact, Expr)>,
    /// In a trait's default body copied into an impl (ADR 0049): the impl's
    /// arguments for the trait's parameters, `Self` among them.
    self_args: Option<ty::GenericArgsRef<'tcx>>,
    /// And the impl's typing environment, where what its arguments name
    /// resolves: the body's own is its trait's.
    self_env: Option<ty::TypingEnv<'tcx>>,
    /// Of a function that tracks its caller (ADR 0427): the location it's
    /// given, and the function, whose own body reads it.
    caller_location: Option<(DefId, Expr)>,
}

/// What walks of types found, kept so a type met again, along another path
/// through a type, isn't walked again: `Foo2(Foo1, Foo1)` of `Foo1(Foo0,
/// Foo0)` is walked once, not 2^n times.
struct TypeWalks<'tcx> {
    /// What `unsupported_in` found of each struct and enum it looked into.
    representable: RefCell<HashMap<Ty<'tcx>, Option<Ty<'tcx>>>>,
    /// While `unsupported_in` walks a type: how far out, among the types it's
    /// inside, is the one a walk took as fine, being inside itself.
    assumed: Cell<usize>,
    /// What `contains_mutated` and `needs_clone_in` found of each type, and
    /// how far out `needs_clone_in` assumed.
    mutated: RefCell<HashMap<Ty<'tcx>, bool>>,
    clones: RefCell<HashMap<Ty<'tcx>, bool>>,
    clone_assumed: Cell<usize>,
}

impl Default for TypeWalks<'_> {
    fn default() -> Self {
        TypeWalks {
            representable: RefCell::new(HashMap::new()),
            assumed: Cell::new(usize::MAX),
            mutated: RefCell::new(HashMap::new()),
            clones: RefCell::new(HashMap::new()),
            clone_assumed: Cell::new(usize::MAX),
        }
    }
}

/// What an arrow of `params` that returns `call` is, if it's a function of
/// the crate's by name given them as they are: `(arg0) => shapeArea_area(arg0)`
/// is `shapeArea_area`, not a dictionary's method, read off it. Not a
/// runtime helper, whose arrow names what it's given by its type, `(s) =>
/// $byteLen(s)`, where it's written in place.
/// A call of a dictionary made here, `({ fmt: (value, options) => .. })
/// .fmt(a, b)`: its function's, `((value, options) => ..)(a, b)`, an arrow,
/// which no `this` reaches, of a dictionary nothing else in has an effect
/// (ADR 0538).
fn in_place_method(call: Expr) -> Expr {
    if let js::ExprKind::Call(callee, list) = &call.kind
        && let js::ExprKind::Member(object, name) = &callee.kind
        && let js::ExprKind::Object(props) = &object.kind
        && !object.has_effects()
        && let Some(f) = props.iter().find_map(|p| match p {
            js::Prop::Field(field, f) if field == name && matches!(f.kind, js::ExprKind::Arrow(..)) => Some(f),
            _ => None,
        })
    {
        return Expr::call(f.clone(), list.clone());
    }
    call
}

fn called_by_name(params: &[String], call: &Expr) -> Option<Expr> {
    let js::ExprKind::Call(callee, list) = &call.kind else {
        return None;
    };
    // Or a function made here, which takes as many: itself (ADR 0538).
    let arrow = matches!(&callee.kind, js::ExprKind::Arrow(taken, _)
        if taken.len() == params.len() && taken.iter().all(|p| matches!(p, js::Pattern::Name(_))));
    let by_name = (matches!(callee.kind, js::ExprKind::Var(_)) || arrow)
        && callee.helper_name().is_none()
        && list.len() == params.len()
        && (list.iter().zip(params)).all(|(value, name)| matches!(&value.kind, js::ExprKind::Var(v) if v == name))
        && params.iter().all(|name| !callee.mentions_var(name));
    by_name.then(|| (**callee).clone())
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What walks of types found under the typing environment in force.
    fn walks(&self) -> Rc<TypeWalks<'tcx>> {
        Rc::clone(self.walks.borrow_mut().entry(self.typing_env).or_default())
    }

    // ── Statement mode ──────────────────────────────────────────────────

    // ── Expression mode ─────────────────────────────────────────────────

    // ── Leaves ──────────────────────────────────────────────────────────

    fn literal(&self, lit: &LitKind, neg: bool, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        match *lit {
            LitKind::Bool(b) => Ok(Expr::bool(b)),
            // One written across lines keeps them, a template literal.
            LitKind::Str(s, style) if s.as_str().contains('\n') && written_across_lines(self.tcx, style, span) => {
                Ok(Expr::lines(s.as_str()))
            }
            LitKind::Str(s, _) => Ok(Expr::str(s.as_str())),
            // A `char` is a string of one character (ADR 0034).
            LitKind::Char(c) => Ok(Expr::str(c.to_string())),
            LitKind::Int(n, _) => {
                let num = self.num(ty, span)?;
                let n = n.get() as i128;
                // One too big for its type, where `overflowing_literals` is
                // allowed, wraps to it as rustc's does: `256u8` is 0.
                Ok(num.wrap(num.literal(if neg { -n } else { n })))
            }
            LitKind::Byte(b) => Ok(Expr::int(b.into())),
            LitKind::Float(sym, _) if Num::of(ty) == Some(Num::F64) => {
                let x: f64 = sym
                    .as_str()
                    .replace('_', "")
                    .parse()
                    .expect("rustc validated the literal");
                Ok(Expr::num(if neg { -x } else { x }))
            }
            // Read as an `f32`, as rustc reads it: nearest to the digits,
            // rounded once (ADR 0122).
            LitKind::Float(sym, _) if Num::of(ty) == Some(Num::F32) => {
                let x: f32 = sym
                    .as_str()
                    .replace('_', "")
                    .parse()
                    .expect("rustc validated the literal");
                Ok(f32_literal(if neg { -x } else { x }))
            }
            _ => Err(self.unsupported(span, "this literal")),
        }
    }

    // ── Structs and tuples (ADR 0020) ───────────────────────────────────

    // ── Helpers ─────────────────────────────────────────────────────────

    /// Map the original callsite into the compiler-owned source arena.
    fn js_span(&self, span: Span) -> js::Span {
        self.krate.sources.span(span)
    }

    fn strip(&self, e: ExprId) -> ExprId {
        strip(self.thir, e)
    }

    fn fresh(&mut self, base: &str) -> String {
        fresh_in(&mut self.names, base)
    }

    /// The names a pattern gives the fields it leaves out, so the rest
    /// after them holds only the others (ADR 0205), each `_field`: one left
    /// out is `_` once it's known, as preparation names it.
    fn left_out(&mut self, keys: &[&str]) -> Vec<String> {
        (keys.iter())
            .map(|key| self.fresh(&format!("_{}", camel_case(&crate::names::js_ident(key)))))
            .collect()
    }

    fn bind(&mut self, var: LocalVarId, name: &str, mutable: bool) -> String {
        let name = self.fresh(&camel_case(name));
        self.locals.vars.insert(
            var,
            Var {
                place: Expr::var(&name),
                mutable,
            },
        );
        name
    }

    /// A `const` (ADR 0031). One of ours is its name, `SIZE` or `util.SIZE`,
    /// copied where a use might change it: each use is a value of its own.
    /// Anyone else's, like `u32::MAX`, is its value, written in place. A
    /// trait's in generic code, `S::SIDES`, is its impl's dictionary's (ADR 0106).
    fn named_const(&mut self, def_id: DefId, args: ty::GenericArgsRef<'tcx>, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        // A trait's, of a type: its impl's (ADR 0410), of one generic code
        // names too, `Self::CAPACITY` of an `impl<T, const CAP: usize>`.
        let (def_id, args) = match self.tcx.trait_of_assoc(def_id) {
            Some(_) => match ty::Instance::try_resolve(self.tcx, self.typing_env, def_id, args) {
                Ok(Some(instance)) => (instance.def_id(), instance.args),
                _ => (def_id, args),
            },
            _ => (def_id, args),
        };
        // One holding a `Cell`, which changes through a shared reference, is
        // its value, written in place: `{ value: 5 }` each time it's used.
        if self.krate.fns.contains_key(&def_id) && ty.is_freeze(self.tcx, self.typing_env) {
            // `fn_ref` also records the use, which is what imports its module.
            let place = self.fn_ref(def_id);
            let copied = self.const_use_copied(ty)
                // One whose value JS holds as itself, `undefined` of one with
                // no byte written, `MaybeUninit::uninit()`'s, which nothing
                // changes (ADR 0332).
                && !representation::const_value_js(self.tcx, self.typing_env, def_id, args, span)
                    .is_some_and(|value| value.is_constant());
            return match copied {
                true => self.const_copy(place, ty, span),
                false => Ok(place),
            };
        }
        if let Some(value) = representation::const_value_js(self.tcx, self.typing_env, def_id, args, span) {
            return Ok(value);
        }
        match self.tcx.trait_of_assoc(def_id) {
            Some(trait_id) if args.has_non_region_param() => {
                let dictionary = self.dictionary(ty::TraitRef::from_assoc(self.tcx, trait_id, args), span)?;
                Ok(Expr::member(dictionary, bindings::fn_name(self.tcx, def_id)))
            }
            // The crate's, of a type parameter's or a const generic's, which
            // rustc can't say as a value, arrayvec's `[const {
            // MaybeUninit::uninit() }; CAP]`, or a trait's default, of the
            // type it's given, or one holding a pointer, semver's `HEAD`
            // (ADR 0562): its MIR, given these, as code, where its module
            // has no constant of it (ADR 0127), above.
            _ if def_id.is_local() => self.crate_const_as_code(def_id, args),
            _ => Err(self.unsupported(span, "this constant")),
        }
    }

    /// A function as a value: a constructor's, `.map(Some)`, an arrow making
    /// what its call makes (ADR 0125); the crate's, `component(Card, props)`,
    /// its JS name, given its dictionaries; a std one's, `.map(str::trim)`,
    /// `(s) => s.trim()`; a binding's. `None` for any other, a library's
    /// among them, which the arrow of its call as MIR's is, its import by
    /// name where that's all it is (ADR 0100).
    fn fn_item_value(
        &mut self,
        def_id: DefId,
        args: ty::GenericArgsRef<'tcx>,
        ty: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let js_span = self.js_span(span);
        if let Some(why) = self.krate.foreign.unlisted(def_id) {
            return Err(self.tcx.dcx().span_err(span, why));
        }
        if matches!(self.tcx.def_kind(def_id), DefKind::Ctor(_, CtorKind::Fn)) {
            return self.constructor_value(def_id, args, span).map(Some);
        }
        if self.krate.fns.contains_key(&def_id) || self.is_rust_trait_fn(def_id, args) {
            if self.tcx.trait_of_assoc(def_id).is_some() {
                if self.takes_caller_location(def_id) {
                    return Err(self.unsupported(
                        span,
                        "a `#[track_caller]` trait method that reads its caller, as a value",
                    ));
                }
                let inputs = self
                    .tcx
                    .fn_sig(def_id)
                    .instantiate(self.tcx, args)
                    .skip_normalization()
                    .skip_binder()
                    .inputs()
                    .to_vec();
                let variadic = recognition::is_c_variadic(self.tcx, def_id);
                // A `&mut` to an object it's given, the object itself, is a box,
                // `{ value }`, to the method, as a call lowered as MIR's makes
                // it (ADR 0099): that call's arrow. One to anything else is a
                // cell, a box already, given as it is.
                if inputs.iter().any(
                    |t| matches!(*t.kind(), ty::Ref(_, inner, ty::Mutability::Mut) if !self.is_cell_pointee(inner)),
                ) {
                    return Ok(None);
                }
                let params: Vec<String> = (0..inputs.len()).map(|i| self.fresh(&format!("arg{i}"))).collect();
                let mut values: Vec<Expr> = params.iter().map(|name| Expr::var(name)).collect();
                // A C-variadic one's, given on what it's given past the rest,
                // `(...args) => f(...args)` (ADR 0428).
                let variadic = variadic.then(|| self.fresh("args"));
                if let Some(rest) = &variadic {
                    values.push(Expr::spread(Expr::var(rest)));
                }
                // A std trait's, `ToString::to_string` or `i32::max`: what
                // its call is.
                let Some(call) = self.trait_call(def_id, args, values, span, out)? else {
                    return Ok(None);
                };
                let call = in_place_method(call);
                if let Some(callee) = called_by_name(&params, &call) {
                    return Ok(Some(callee));
                }
                let mut patterns: Vec<js::Pattern> = params.into_iter().map(Into::into).collect();
                patterns.extend(variadic.map(js::Pattern::Rest));
                return Ok(Some(Expr::arrow(
                    patterns,
                    vec![StmtKind::Return(Some(call)).at(js_span)],
                )));
            }
            let callee = self.fn_ref(def_id);
            // One given its caller's location is called where it's defined,
            // as rustc's shim of it calls it (ADR 0427).
            let location = self.location_at(self.tcx.def_span(def_id));
            let evidence = self.evidence_args_with(def_id, args, location, span)?;
            return Ok(Some(if evidence.is_empty() {
                callee
            } else {
                let count = self
                    .tcx
                    .fn_sig(def_id)
                    .instantiate(self.tcx, args)
                    .skip_normalization()
                    .skip_binder()
                    .inputs()
                    .len();
                let params: Vec<String> = (0..count).map(|i| format!("arg{i}")).collect();
                let values = params.iter().map(|name| Expr::var(name)).chain(evidence).collect();
                Expr::arrow(
                    params.into_iter().map(Into::into).collect(),
                    vec![StmtKind::Return(Some(Expr::call(callee, values))).at(js_span)],
                )
            }));
        }
        let known = self.recognition().classify(def_id, args);
        if let Some(known) = known
            && let Some(f) = self.std_fn_value(known, ty, span)?
        {
            return Ok(Some(f));
        }
        if let Some(Std::MaxOf(max)) = known {
            return Ok(Some(Expr::helper(if max { "$f64Max" } else { "$f64Min" })));
        }
        if is_binding(self.tcx, def_id) {
            return self.binding_value(def_id, args, span, span).map(Some);
        }
        Ok(None)
    }

    /// `[item; count]`, the count a caller's `N` or a number (ADR 0107);
    /// `constant`, whether the item is a named constant's value.
    fn repeat(
        &mut self,
        (item, item_ty): (Expr, Ty<'tcx>),
        count: ty::Const<'tcx>,
        constant: bool,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let count = self
            .tcx
            .normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(count));
        // A caller's `N` (ADR 0107), or the number.
        let n = count.try_to_target_usize(self.tcx);
        let length = match n {
            Some(n) => Expr::int(n as i128),
            None => self.const_arg(count, span)?,
        };
        // Nothing yet, `MaybeUninit::uninit()`'s: so many slots, as an
        // array of uninit items is (ADR 0332).
        if matches!(item.kind, js::ExprKind::Undefined) {
            return Ok(crate::lower::plain::filled(length, None));
        }
        // A `Copy` value's copies are copies of its bits, which a
        // value that nothing changes needs none of; one that isn't
        // `Copy` is a constant's, made again for each, as each use of
        // a constant is.
        let copied = if constant {
            self.const_use_copied(item_ty)
        } else if self.is_copy(item_ty) {
            self.contains_mutated(item_ty)
        } else if n == Some(1) {
            // The one there is, which Rust allows of any value.
            return Ok(Expr::array(vec![item]));
        } else if self.needs_clone(item_ty) {
            return Err(self.unsupported(span, "`[x; N]` of a value that isn't `Copy`"));
        } else {
            false
        };
        if !copied {
            if let Some(n) = n
                && n <= 4
                && item.is_constant()
            {
                return Ok(Expr::array(vec![item; n as usize]));
            }
            let array = Expr::new_(Expr::var("Array"), vec![length]);
            return Ok(Expr::call(Expr::member(array, "fill"), vec![item]));
        }
        // A constant's use is its copy already, `EMPTY.slice()`: made again
        // for each, as each use of a constant is.
        let copy = match constant {
            true => item,
            false => {
                let item = match item.reads_same() {
                    true => item,
                    false => self.spill("item", item, out),
                };
                self.copy(item, item_ty)
            }
        };
        let length = Expr::object(vec![Prop::Field("length".into(), length)]);
        let from = Expr::member(Expr::var("Array"), "from");
        Ok(Expr::call(
            from,
            vec![
                length,
                Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(copy)).at(js::Span::NONE)]),
            ],
        ))
    }

    /// `const <base> = value;`, so it's evaluated here, then its name.
    fn spill(&mut self, base: &str, value: Expr, out: &mut Vec<Stmt>) -> Expr {
        let name = self.fresh(&unhyphenated(base));
        let span = value.span;
        out.push(StmtKind::Const(name.clone(), value).at(span));
        Expr::var(&name)
    }

    fn unsupported(&self, span: Span, what: &str) -> ErrorGuaranteed {
        refusal::refuse(self.tcx, span, refusal::Refused::Feature(what))
    }

    /// No dictionary for `tr` was given: a component's, which React never
    /// gives one (ADR 0201), or one rust-js doesn't make.
    fn no_evidence(&self, span: Span, tr: ty::TraitRef<'tcx>) -> ErrorGuaranteed {
        if bindings::is_component(self.tcx, self.item) {
            let what = format!("a component's `{tr}`");
            refusal::refuse(
                self.tcx,
                span,
                refusal::Refused::Because(&what, "React gives a component no dictionary"),
            )
        } else {
            self.unsupported(span, &format!("implementation evidence for `{tr}`"))
        }
    }
}

/// What a binding's path reads, `console.log`, `react.__CLIENT_INTERNALS..`
/// of `react#*.__CLIENT_INTERNALS..` by `import`'s name of the export it's
/// in, or `require("eslint-plugin-react-hooks").rules` of one required
/// (ADRs 0028, 0305).
fn path_read(path: &str, required: &HashSet<Export>, import: impl FnOnce(&Export) -> String) -> Expr {
    match bindings::js_import(path) {
        Some((export, rest)) if required.contains(&export) => {
            let module = Expr::call(Expr::var("require"), vec![Expr::str(&export.0)]);
            format!("{}{rest}", export.1).split('.').fold(module, Expr::member)
        }
        Some((export, rest)) => global(&format!("{}{rest}", import(&export))),
        None => global(path),
    }
}

/// A JS global, like `document`, or a path from one, like `console.log`.
fn global(name: &str) -> Expr {
    let mut parts = name.split('.');
    let first = Expr::var(parts.next().unwrap_or_default());
    parts.fold(first, Expr::member)
}

/// A fieldless enum's variants, each its name and its discriminant, in the
/// order they're declared: `Red = 1` is `("Red", 1)`.
fn discriminants<'tcx>(tcx: TyCtxt<'tcx>, adt: ty::AdtDef<'tcx>) -> Vec<(String, i128)> {
    adt.discriminants(tcx)
        .map(|(index, d)| {
            let value = d.val as i128;
            let value = if d.ty.is_signed() {
                let bits = d.ty.primitive_size(tcx).bits();
                (value << (128 - bits)) >> (128 - bits)
            } else {
                value
            };
            (bindings::variant_name(tcx, adt.variant(index)), value)
        })
        .collect()
}

/// A struct's or a variant's value, of its fields' values in declared order:
/// an object, tagged `{ TAG: "Circle", _0: r }` for a variant (ADR 0033), or
/// an array for a tuple struct (ADR 0020).
fn assembled(shape: Shape<'_>, tag: Option<(String, Expr)>, items: Vec<Expr>) -> Expr {
    match shape {
        Shape::Object(fields) => {
            let tag = tag.map(|(key, name)| Prop::Field(key, name));
            let fields = fields.into_iter().zip(items).map(|((name, _), v)| Prop::Field(name, v));
            Expr::object(tag.into_iter().chain(fields).collect())
        }
        _ => Expr::array(items),
    }
}

/// `name` without its hyphens, each next letter capitalized: a hyphenated
/// key's variable, `data-step`'s `dataStep`.
fn unhyphenated(name: &str) -> String {
    let mut out = String::new();
    let mut capital = false;
    for c in name.chars() {
        match (c, capital) {
            ('-', _) => capital = true,
            (c, true) => {
                out.extend(c.to_uppercase());
                capital = false;
            }
            (c, false) => out.push(c),
        }
    }
    out
}

/// A Rust name as JS code writes it (ADRs 0038, 0573): `set_count` is
/// `setCount`. Leading and trailing underscores stay (`_unused`, `type_`),
/// and so does a name with no lowercase letter, a component's `H2` say.
/// The one place a `snake_case` name is made camelCase; a thread-local's
/// `SCREAMING_SNAKE` one is `variable_case`'s.
fn camel_case(name: &str) -> String {
    let core = name.trim_matches('_');
    if !core.contains('_') || !core.contains(|c: char| c.is_ascii_lowercase()) {
        return name.to_string();
    }
    words_joined(name, |word| word.to_string())
}

/// A thread-local's name as JS writes its module's variable (ADR 0573):
/// its `SCREAMING_SNAKE` words lowercase, then camelCase, `TASK_QUEUE` as
/// `taskQueue`; another, as `camel_case` writes it.
fn variable_case(name: &str) -> String {
    let core = name.trim_matches('_');
    if core.contains(|c: char| c.is_ascii_lowercase()) || !core.contains(|c: char| c.is_ascii_uppercase()) {
        return camel_case(name);
    }
    words_joined(name, |word| word.to_ascii_lowercase())
}

/// `name`'s words, each `cased`, joined as camelCase, between its leading
/// and trailing underscores.
fn words_joined(name: &str, cased: impl Fn(&str) -> String) -> String {
    let core = name.trim_matches('_');
    let lead = &name[..name.len() - name.trim_start_matches('_').len()];
    let trail = &name[name.trim_end_matches('_').len()..];
    let mut out = lead.to_string();
    for (i, word) in core.split('_').filter(|w| !w.is_empty()).enumerate() {
        match i {
            0 => out.push_str(&cased(word)),
            _ => out.push_str(&upper_first(&cased(word))),
        }
    }
    out.push_str(trail);
    out
}

/// `counter` → `Counter`: a word joined into a camelCase name, after the
/// first, `RefFlags` of `Ref` and `flags`.
pub(super) fn upper_first(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|c| std::iter::once(c.to_ascii_uppercase()).chain(chars).collect())
        .unwrap_or_default()
}

/// `Counter` → `counter`: a value of the type, named after it.
pub(super) fn lower_first(name: &str) -> String {
    let mut chars = name.chars();
    chars
        .next()
        .map(|c| c.to_lowercase().chain(chars).collect())
        .unwrap_or_default()
}

/// `&x` is `x`: a reference is the value (ADR 0023).
fn without_refs<'p, 'tcx>(mut pat: &'p Pat<'tcx>) -> &'p Pat<'tcx> {
    while let PatKind::Deref { subpattern, .. } = &pat.kind {
        pat = subpattern;
    }
    pat
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    fn recognition(&self) -> recognition::Recognition<'_, 'tcx> {
        recognition::Recognition {
            tcx: self.tcx,
            typing_env: self.typing_env,
            trait_impls: self.krate.trait_impls,
            foreign: self.krate.foreign,
        }
    }
}

/// Translate a frontend module identity into an owned link symbol.
fn module_symbol(module: LocalModId, export: &str) -> crate::js::Symbol {
    crate::js::Symbol {
        module: module.to_def_id().index.as_u32(),
        export: export.to_owned(),
    }
}

/// Whether a string literal's line breaks are written as such in its
/// source, not as `\n`: a raw string's always are, and a cooked one's
/// where it spans lines, other than by a `\` ending one, which leaves
/// the break out.
fn written_across_lines(tcx: TyCtxt<'_>, style: StrStyle, span: Span) -> bool {
    !span.from_expansion()
        && match style {
            StrStyle::Raw(_) => true,
            StrStyle::Cooked => tcx
                .sess
                .source_map()
                .span_to_snippet(span)
                .is_ok_and(|code| code.match_indices('\n').any(|(i, _)| !code[..i].ends_with('\\'))),
        }
}

/// Whether the string literal at `span` is written across lines, as MIR's
/// constant of it says only by where it's written.
pub(super) fn literal_across_lines(tcx: TyCtxt<'_>, span: Span) -> bool {
    let Ok(code) = tcx.sess.source_map().span_to_snippet(span) else {
        return false;
    };
    let style = match code.chars().next() {
        Some('r') => StrStyle::Raw(0),
        Some('"') => StrStyle::Cooked,
        _ => return false,
    };
    written_across_lines(tcx, style, span)
}

/// A `const`'s text, `const PAGE: &str = r#"<main>..`, written across
/// lines: a template literal of them, as a literal written so in place is.
fn const_lines(tcx: TyCtxt<'_>, def_id: LocalDefId, value: Expr) -> Expr {
    let js::ExprKind::Str(text) = &value.kind else {
        return value;
    };
    let body = tcx.hir_body_owned_by(def_id);
    match body.value.kind {
        rustc_hir::ExprKind::Lit(lit)
            if let rustc_ast::LitKind::Str(_, style) = lit.node
                && text.contains('\n')
                && written_across_lines(tcx, style, lit.span) =>
        {
            Expr::lines(text.as_str())
        }
        _ => value,
    }
}
