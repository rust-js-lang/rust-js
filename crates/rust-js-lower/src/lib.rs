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
//!
//! The lowering's base, a crate of its own (#2299, ADR 1181): `FnCx` and
//! the analyses, recognition and representation every lowering asks;
//! MIR's lowering, `rust-js-mir`, depends on it, and the driver on both.

#![feature(rustc_private)]

// A unit test's binary links rustc as the compiler's does.
extern crate rustc_abi;
extern crate rustc_arena;
extern crate rustc_ast;
extern crate rustc_const_eval;
extern crate rustc_data_structures;
#[cfg(test)]
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_expand;
extern crate rustc_feature;
extern crate rustc_hir;
extern crate rustc_hir_analysis;
extern crate rustc_index;
extern crate rustc_lexer;
extern crate rustc_lint_defs;
extern crate rustc_middle;
extern crate rustc_mir_dataflow;
extern crate rustc_mir_transform;
extern crate rustc_parse;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_structures;
extern crate rustc_target;
extern crate rustc_trait_selection;

use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_middle::ty::consts::ConstExt as _;
use std::cell::{Cell, RefCell};

use std::rc::Rc;

use rustc_ast::LitKind;
use rustc_hir::def::{CtorKind, DefKind};
use rustc_middle::thir::{ExprId, LocalVarId, Pat, PatKind, Thir};
use rustc_middle::ty::{self, GenericArgsRef, Ty, TyCtxt, TypeVisitableExt};
use rustc_span::Span;
use rustc_span::def_id::{DefId, LocalModId};

use rust_js_program::js::{self, Expr, Prop, Stmt, StmtKind};

pub mod aggregates;
pub mod analysis;
pub mod binding_calls;
pub mod bindings;
mod bodies;
pub mod body_queries;
pub mod boundary;
pub mod call_graph;
pub mod calls;
pub mod classes;
pub mod combinators;
pub mod copies;
pub mod crate_facts;
pub mod declarations;
pub mod default_exports;
pub mod discriminants;
pub mod display;
pub mod drops;
mod field_keys;
pub mod format_args;
mod format_spec;
pub mod generic_facts;
pub mod held;
pub mod instances;
pub mod items;
pub mod iterators;
pub mod jsx;
mod jsx_api;
pub mod lent;
pub mod library;
pub mod literals;
pub mod liveness;
pub mod load_order;
mod loops;
mod module_files;
pub(crate) use module_files::module_path;
pub use module_files::{located, module_file};
pub mod mir_hooks;
pub mod mut_refs;
pub mod named_constants;
pub mod noted;
pub mod offered;
mod ordering;
mod parameter_names;
pub mod passed_on;
pub mod patterns;
mod places;
mod plain;
pub mod pointers;
pub mod predicates;
pub mod recognition;
mod recursive_fns;
pub mod reexports;
pub mod refusal;
pub mod representation;
pub mod serde;
pub mod session;
mod shipped;
mod shortcuts;
pub mod signatures;
pub mod sources;
pub mod std_coverage;
pub mod std_impls;
mod std_rows;
pub mod std_types;
mod support;
pub mod temporary_names;
pub use temporary_names::{Base, Role};
pub mod traits;
mod unions;
pub mod untagged;
mod uses;

pub use analysis::{
    Copied, bodies_of_build, collect_bodies, collect_initializers, collect_mir, collect_std_bodies, copied_bodies,
};
/// Whether each `#[rust_js::name]` gives a name its item hasn't anyway, of
/// a crate `--rustc` compiles for its metadata too (ADR 0573).
pub use bindings::given_names_differ;
use bindings::{Export, is_binding};
pub use declarations::std_types::module as std_declarations;
/// Whether each `#[rust_js::types]` is a TypeScript type rust-js reads, of
/// a crate `--rustc` compiles for its metadata too (ADR 0811).
pub use declarations::validate_types;
pub use instances::{Instances, instance_bodies};
pub use recognition::models::json as std_models;
pub use recognition::models::rows::StdRow;
use rust_js_program::casing::{camel_case, unhyphenated};
use rust_js_program::names::{fresh_in, js_ident};
/// Whether each `From` into an untagged enum is the variant of its
/// argument, of a crate `--rustc` compiles, whose JS rust-js doesn't write
/// (ADR 0214, amended).
pub use untagged::binding_froms_give_their_variants;

pub use recognition::{Std, TypeFact};
pub use representation::{MutRefForm, Num, const_js, eval_const, f32_literal, static_value};
pub use serde::{SerdeAttributes, attributes as serde_attributes};

use body_queries::strip;
pub use crate_facts::{CrateFacts, Settled};
pub use parameter_names::{default_parameter_name, parameter_name, written_parameter_name};

pub type R<T> = Result<T, refusal::Refusal>;

/// A function's THIR, copied out of rustc before borrowck steals it: read
/// for the shape and facts its MIR is lowered with. Cloned for each build
/// of a case one rustc session compiles (#2301).
#[derive(Clone)]
pub struct Body<'tcx> {
    pub def_id: DefId,
    pub thir: Thir<'tcx>,
    pub expr: ExprId,
    pub facts: body_queries::BodyFacts<'tcx>,
    /// Its MIR, which the body is lowered from where it's read (ADR 0364).
    pub mir: Option<analysis::Mir<'tcx>>,
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
pub struct FnInfo {
    pub module: LocalModId,
    pub name: String,
    /// A method's type's object of methods (ADR 0047): `Counter` for `Counter.tick`.
    pub owner: Option<String>,
    /// What its module exports it by where its module names it apart: its
    /// own name, `delete` of `delete$`, a word JS keeps, or `fetch` of
    /// `fetch$1`, a global its module may read, `export { delete$ as
    /// delete }` (#338, ADR 0922, amended).
    export: Option<String>,
}

impl FnInfo {
    /// The name its module exports it by, where it exports it by its own
    /// (`default_exports::exported_as` asks it of an alias first).
    pub fn exported(&self) -> &str {
        self.export.as_deref().unwrap_or(&self.name)
    }
}

/// The module `def_id`'s JS is written into: the crate's own item's, the
/// module it's in, a class's member's its class's; a std body's (ADR 0561), the crate's
/// module named as std's is, `iter` of `core::iter::Peekable`'s. The one
/// answer to where a body goes.
pub(crate) fn body_module(tcx: TyCtxt<'_>, instances: &Instances, def_id: DefId) -> LocalModId {
    if let Some(local) = def_id.as_local() {
        // A class's member, wherever its impl is: the class's own module,
        // where it's declared, one class of all its members (#2497).
        if let Some(module) = classes::class_of(tcx, def_id)
            .and_then(|class| classes::implicit_constructor_home(tcx, class).or_else(|| classes::home(tcx, class)))
        {
            return module;
        }
        return tcx.parent_module_from_def_id(local);
    }
    // A library's function the crate compiles itself is beside its first
    // caller (ADR 0914).
    if let Some(module) = instances.module(tcx, def_id) {
        return module;
    }
    recognition::std_home_module(tcx, def_id).unwrap_or_else(|| {
        panic!(
            "a crate given std's bodies has no module for {}",
            tcx.def_path_str(def_id)
        )
    })
}

/// A function item's type: the function, and its generic args. rustc 1.99
/// binds the args; a body's never have bound vars, as rustc's own MIR
/// building reads them.
pub fn fn_def<'tcx>(ty: Ty<'tcx>) -> Option<(DefId, GenericArgsRef<'tcx>)> {
    match *ty.kind() {
        ty::FnDef(def_id, args) => Some((def_id, args.no_bound_vars().expect("a body's function item, unbound"))),
        _ => None,
    }
}

/// The constant a constant alias names: an associated const of a trait or
/// of an impl, a free one or an anonymous one. Each of 1.100's kinds, none
/// by a wildcard, so a kind a release adds is asked here first.
pub fn alias_const_def_id(kind: ty::AliasConstKind<'_>) -> DefId {
    match kind {
        ty::AliasConstKind::Projection { def_id }
        | ty::AliasConstKind::InherentSelf { def_id }
        | ty::AliasConstKind::InherentImpl { def_id }
        | ty::AliasConstKind::Free { def_id }
        | ty::AliasConstKind::Anon { def_id } => def_id,
    }
}

pub struct LoweredFn {
    pub function: js::Function,
    pub jsx: bool,
    pub dependencies: Dependencies,
    /// Of a binary's `main`, what its module runs last as it loads,
    /// `$main(main)`: std's runtime's start of it (#2292).
    pub entry: Option<Vec<js::Stmt>>,
}

pub struct Var {
    /// Usually the variable's JS name. An immutable binding into a pattern
    /// can instead just name the place it matched: `let (q, r) = t` makes
    /// `q` mean `t[0]`, with no JS variable at all.
    pub place: Expr,
    mutable: bool,
}

/// What a body knows of its variables: what each is in JS, and what its
/// lowering found of some. A closure's body and a coroutine's share their
/// enclosing body's, since they name its variables; a trait's default body,
/// copied into an impl, has its own (`enter_body`). Something known of a
/// variable, by its `LocalVarId`, goes here, and so goes with it.
#[derive(Default)]
pub struct Locals {
    pub vars: IndexMap<LocalVarId, Var>,
    /// A props pattern's rest, JS's `...props`, by its name: the keys the
    /// pattern names, which it never holds (ADRs 0195, 0205).
    rests: IndexMap<js::Name, Vec<String>>,
    /// What lowering a body from its MIR found of its variables (ADR 0364),
    /// mir.rs's, which the rest ask of it.
    pub mir: noted::Found,
}

/// How a Rust struct or tuple type is represented in JS (ADR 0020).
pub enum Shape<'tcx> {
    /// A struct with named fields: `{ x: 1, y: 2 }`.
    /// A struct's fields, by their keys: a name, or a symbol (ADR 1051).
    Object(Vec<(bindings::FieldKey, Ty<'tcx>)>),
    /// A tuple or tuple struct: `[1, 2]`.
    Array(Vec<Ty<'tcx>>),
    /// Anything else: numbers, `bool`, unit and unit structs (`undefined`), enums.
    Other,
}

/// A body lowered inside the one being lowered, and what it starts from
/// (`enter_body`). Each has its own THIR and owner; what
/// else isn't said here, it shares with the enclosing body.
pub enum Nested<'tcx> {
    /// A closure's, an arrow: the enclosing `Locals` and captures, and
    /// these names to start from.
    Closure { names: IndexSet<String> },
    /// A trait's default body, copied into an impl (ADR 0049): `Locals` of
    /// its own, a copy of the names, and the impl's evidence, arguments,
    /// typing environment and drops.
    Default {
        given: Box<traits::DefaultGiven<'tcx>>,
        typing_env: ty::TypingEnv<'tcx>,
        /// The drops it's given, by the type each drops in the impl's
        /// terms: the impl's (ADRs 0098, 0421) and its own (ADR 0163).
        drops: IndexMap<Ty<'tcx>, drops::GivenDrop>,
        /// Each associated type a caller could give a drop for (ADR 0489).
        slots: IndexSet<ty::AliasTy<'tcx>>,
    },
}

/// What a nested body took of the enclosing one's state, given back when
/// it's left (`leave_body`): what every body has of its own, and what its
/// kind has too, as `Nested` says.
struct Enclosing<'a, 'tcx> {
    frame: BodyFrame<'a, 'tcx>,
    drops: drops::EnclosingDrops<'tcx>,
    kind: EnclosingKind<'tcx>,
}

/// What each kind of nested body took, by `Nested`'s kinds.
enum EnclosingKind<'tcx> {
    Closure { names: IndexSet<String> },
    Default(Box<ItemScope<'tcx>>),
}

/// What a trait's default body, copied into an impl, has of its own that
/// another body lowers with: the item's names, `Locals`, evidence, arguments,
/// typing environment and drops.
struct ItemScope<'tcx> {
    names: IndexSet<String>,
    locals: Locals,
    given: traits::GivenScope<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
}

/// Dependencies recorded by one function (including copied trait bodies and
/// closures), returned with its JS. They never mutate the crate's inputs.
#[derive(Default)]
pub struct Dependencies {
    pub references: IndexSet<(LocalModId, DefId)>,
    /// Each copy of a function another module's calls name, by its index
    /// (ADR 0051, amended).
    pub copy_references: IndexSet<(LocalModId, DefId, usize)>,
    /// Each split function's body another module's calls name, `f$` (#2403
    /// step 2b(a), ADR 1193).
    pub unchecked_references: IndexSet<(LocalModId, DefId)>,
    /// The checkers of the crate's structs a function's checks of what JS
    /// gives it call, each its module's, by its name: `None` while it's
    /// being made, as a struct that holds itself checks by its own (#2403
    /// step 2b(b), ADR 1194).
    pub checkers: IndexMap<DefId, Option<Checker>>,
    /// While a generic type's checker is made, its type parameters' checkers
    /// it's given, by each parameter's index, innermost last (ADR 1194).
    pub checker_params: Vec<Vec<(u32, rust_js_program::js::Name)>>,
    /// Each call of a namespace's method through a value whose call needs a
    /// copy of it, which calls the value's function, the module's own JS:
    /// refused where that copy isn't alike it (rd311, ADRs 0051, 1037).
    pub through_copies: Vec<(DefId, usize, rustc_span::Span)>,
    pub package_uses: IndexSet<(LocalModId, Export)>,
    /// Each import read whose fields change in place, a library's static
    /// of a type that does, an atomic's `{ value }`, which a read of moves
    /// past no write (#1028).
    pub changing_imports: IndexSet<String>,
    pub uses: Vec<(DefId, DefId)>,
    /// Each binding a module reads through another of the crate's that
    /// holds it, by its `const`'s name there and whether it's that
    /// module's default (ADR 0633).
    pub held: IndexSet<(LocalModId, LocalModId, String, bool, DefId)>,
    /// Each of the crate's modules a module reads a function through the
    /// namespace of, which re-exports it, `Plugin.registerEvents` (rd137):
    /// an import of that module, where the function's own is another's.
    pub reexporters: IndexSet<(LocalModId, LocalModId)>,
    /// The class a `js::on_load!` body replaces the prototype of, where:
    /// set right after its constructor, before its methods (#448, ADR
    /// 0761 amended).
    pub prototype_set: Option<(DefId, rustc_span::Span)>,
    /// Each ES class whose prototype it reads as a value,
    /// `js::class::<T>().prototype()`, where an accessor may be defined of
    /// a field's name: its fields undeclared (#568).
    pub prototypes_read: IndexSet<DefId>,
    /// Whether it left a dictionary of a value a library makes a `dyn` of
    /// without `fmtInto`, its writes not followed: what lowering found, which
    /// the library's contract says (`makes_fmt_dyns`, #1797, ADR 1148).
    pub fmt_into_left: bool,
    /// Whether it made a closure the function it passes on to, or an `Rc`'s
    /// closure the function, only as the crate reads no function's
    /// identity: one where Rust has two, which a library's consumer that
    /// compares them compiles its own of (ADR 0605, amended; #1052).
    pub merges_identities: bool,
    /// Each `#[track_caller]` function it made a value of whose calls it
    /// doesn't say, in a library, which a binary that links it is refused
    /// for (#2335).
    pub untold_locations: Vec<rust_js_program::library::UntoldLocation>,
}

pub struct FnCx<'a, 'tcx> {
    pub krate: &'a CrateFacts<'a, 'tcx>,
    /// What the crate's bodies lowered before it settled (`Settled`).
    pub settled: &'a Settled<'tcx>,
    pub dependencies: RefCell<Dependencies>,
    /// What lowering this function found of drops, which the pipeline
    /// gathers, of each function lowered or not, and resolves (ADRs 0190,
    /// 0300).
    pub drops_observed: RefCell<drops::DropsObserved>,
    /// The library items this function's lowering found the crate must
    /// compile itself, which their libraries' JS doesn't export, each with
    /// the module that asked, which the pipeline gathers: the crate is
    /// lowered again with them (ADR 0914, amended; ADR 1148).
    pub asked_of_libraries: RefCell<Vec<(DefId, LocalModId)>>,
    /// The module's functions this body made and called, beside those
    /// `settled` holds, which the pipeline settles after it (ADR 1148).
    pub module_fns_made: RefCell<recursive_fns::MadeHere<'tcx>>,
    /// What this body saw of the calls and returns of the crate's functions
    /// with own nullable parameters, which the pipeline settles after it
    /// (ADR 1148).
    pub nulled_seen: RefCell<analysis::nulled_params::NulledSeen>,
    /// The function the call being lowered is of, and what it's given,
    /// while it's lowered, which a clone made of what it's given reads its
    /// row of `clonedAt` by, of the std function rustc resolves it to,
    /// taken by the first clone made under it (`call_clone_site`, #2509,
    /// #2577): set by MIR's call (`rust-js-mir`'s `mir_call`).
    pub calling: std::cell::Cell<Option<(DefId, ty::GenericArgsRef<'tcx>)>>,
    pub tcx: TyCtxt<'tcx>,
    pub typing_env: ty::TypingEnv<'tcx>,
    /// What the generic item being lowered is given, besides its arguments.
    pub given: Given<'tcx>,
    /// In a generic type's derived `serialize` or `deserialize` (ADR
    /// 0080): each type parameter, and the parameter that writes or reads it.
    pub codecs: Vec<(Ty<'tcx>, js::Name)>,
    /// The body being lowered: its own, or a nested one's (`lowered_in`).
    pub frame: BodyFrame<'a, 'tcx>,
    /// The module receiving this function and its recorded dependencies.
    pub module: LocalModId,
    /// What this body knows of its variables.
    pub locals: Locals,
    /// JS names already taken in this function, its item's: given out of
    /// `&self` too, so each name a body declares is the item's own
    /// wherever it's made (#1681 change 3, M2).
    pub names: std::cell::RefCell<IndexSet<String>>,
    /// Of each word `fresh` gives out as a base, the first name made of it
    /// in the item: what a name given apart from that base is of, by its
    /// id (`js::Name::given_apart_from`, ADR 1150).
    pub first_of: std::cell::RefCell<IndexMap<String, js::Name>>,
    /// Those taken by the module: its functions, imports and globals.
    pub module_names: &'a IndexSet<String>,
    /// Whether this function makes JSX.
    pub jsx: bool,
    /// What writing to a `Formatter` knows (ADRs 0054, 0137).
    pub writing: display::Writing,
    /// The functions of recursive types only a caller knows being made
    /// here, which one inside calls (`recursive_fns`).
    pub recursive: recursive_fns::Making<'tcx>,
    /// The item being lowered: what `fn_ref` records as using its target.
    pub item: DefId,
    /// Which copy of it, where it's one its callers need besides its own JS
    /// (`analysis::JsGiven`, ADR 0051, amended).
    pub copy: Option<usize>,
    /// What walks of types found, each walked once, under each typing
    /// environment: what a type holds depends on the bounds in force, as a
    /// copied default's `T::Item` is a number and its sibling's a `Vec`.
    pub walks: RefCell<IndexMap<ty::TypingEnv<'tcx>, Rc<TypeWalks<'tcx>>>>,
    /// What's dropped, and where (ADR 0098).
    pub drop_state: drops::DropState<'tcx>,
    /// Of a function whose scope struct's methods are functions declared
    /// in it, each method's function's name, which calls of it call (ADR
    /// 1043).
    pub scope_functions: IndexMap<DefId, js::Name>,
    /// And the lowering of each this one declares, a copy or none (#500).
    pub scope_copies: IndexMap<DefId, Option<usize>>,
    /// While one of them is lowered, the name of the variable it closes
    /// over, its `self`'s.
    pub scope_self: Option<js::Name>,
    /// Once a function's body is lowered from its MIR, each parameter its JS
    /// takes, in order, by its Rust parameter's index, `None` of one Rust
    /// has none of, a `Formatter`'s options: where the checks of what JS
    /// gives it each place theirs (#2403, ADR 1192, amended).
    pub param_origins: Vec<Option<usize>>,
    /// Once a function's body is lowered, the variable of each of its
    /// scope structs' values, by the function, whose fields are then its
    /// variables (ADR 1043, amended).
    pub scope_values: Vec<(DefId, js::Name, rust_js_program::collections::IndexSet<js::Name>)>,
    /// While a `match`'s tests of its arms are made, which it refuses
    /// itself where they'd tell a variant by `instanceof` of a JS object
    /// type's class, once it knows its arms differ (#555): no test of them
    /// refuses it then.
    pub telling_arms: Cell<bool>,
    /// The local a std iterator method's call being lowered makes its value
    /// into, where it's a whole local, and its type, regions erased: its
    /// maker's MIR identity (#814, #851).
    pub made_into: Option<(rustc_middle::mir::Local, Ty<'tcx>)>,
    /// The whole locals that call is given, and their types, regions
    /// erased: what an adapter made in place where it's stepped is held in
    /// (#851).
    pub made_from: Vec<(rustc_middle::mir::Local, Ty<'tcx>)>,
    /// While what's asked is whether a value made before has a back, its
    /// form's or its mark's, not whether what's made is to be made knowing
    /// it (`has_a_back`'s, #851).
    pub asking_made: Cell<bool>,
    /// While the crate's iterator is made the JS iterator behind a `dyn`
    /// wrapper, `Box<dyn Iterator>` or `&mut dyn Iterator`, that wrapper:
    /// what `own_methods` gives what's made there is what the wrapper passes
    /// on as its own, std's catalog's (`seen_through`, #2217).
    pub seen_through: Cell<Option<crate::recognition::Wrapper>>,
}

/// What a body being lowered has of its own, which a nested one, a
/// closure's or a copied default's, takes whole and gives back
/// (`lowered_in`): its THIR, the facts asked of it, and whose it is. A
/// field every body has of its own goes here, and is swapped with it.
pub struct BodyFrame<'a, 'tcx> {
    pub thir: &'a Thir<'tcx>,
    pub facts: &'a body_queries::BodyFacts<'tcx>,
    /// Whose body `thir` is: its scope tree says where temporaries end.
    pub owner: DefId,
}

/// What the generic item being lowered is given, besides its arguments.
#[derive(Default)]
pub struct Given<'tcx> {
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
    /// Of a function that tracks its caller (ADR 0427): the location it's
    /// given, and the function, whose own body reads it.
    caller_location: Option<(DefId, Expr)>,
    /// Of a function made a value that tracks its caller: the location its
    /// call is given, that of the shim it's called through (#2335).
    value_location: Option<Expr>,
    /// Whether the call being lowered is made apart from MIR's, a function
    /// value's arrow's: no fact of the MIR call it's an argument of is its.
    called_apart: bool,
}

/// What walks of types found, kept so a type met again, along another path
/// through a type, isn't walked again: `Foo2(Foo1, Foo1)` of `Foo1(Foo0,
/// Foo0)` is walked once, not 2^n times.
/// What `contains_mutated`'s walk of a type found: whether it may be changed
/// in place, and the types it found unchanged on the way (#2204).
type Walked<'tcx> = (bool, Rc<[Ty<'tcx>]>);

pub struct TypeWalks<'tcx> {
    /// What `unsupported_in` found of each struct and enum it looked into.
    representable: RefCell<IndexMap<Ty<'tcx>, Option<Ty<'tcx>>>>,
    /// While `unsupported_in` walks a type: how far out, among the types it's
    /// inside, is the one a walk took as fine, being inside itself.
    assumed: Cell<usize>,
    /// What `contains_mutated` and `needs_clone_in` found of each type, and
    /// how far out `needs_clone_in` assumed: of `contains_mutated`'s, with
    /// the types its walk found unchanged, which a copy decision records
    /// in a library's contract (#2204).
    mutated: RefCell<IndexMap<Ty<'tcx>, Walked<'tcx>>>,
    /// While a walk of `contains_mutated` runs, a list each walk inside it
    /// gathers the types it finds unchanged into, innermost last, rather
    /// than the contract (`copies.rs`'s `elided`).
    unchanged: RefCell<Vec<Vec<Ty<'tcx>>>>,
    clones: RefCell<IndexMap<Ty<'tcx>, bool>>,
    clone_assumed: Cell<usize>,
}

impl Default for TypeWalks<'_> {
    fn default() -> Self {
        TypeWalks {
            representable: RefCell::new(IndexMap::default()),
            assumed: Cell::new(usize::MAX),
            mutated: RefCell::new(IndexMap::default()),
            unchanged: RefCell::new(Vec::new()),
            clones: RefCell::new(IndexMap::default()),
            clone_assumed: Cell::new(usize::MAX),
        }
    }
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
            // As it's written: across lines, a template literal of them, or
            // `concat!`'s pieces joined by `+` (ADR 0641).
            LitKind::Str(s, _) => Ok(literals::string_literal(self.tcx, s.as_str(), span, &|at| {
                self.js_span(at)
            })),
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
    pub fn js_span(&self, span: Span) -> js::Span {
        self.krate.sources.span(span)
    }

    pub fn strip(&self, e: ExprId) -> ExprId {
        strip(self.frame.thir, e)
    }

    /// A name of the item's, `base`'s word or given apart from those it has:
    /// its variables', its module's functions' and imports', and the
    /// globals it binds. So the parameter of an arrow lowering makes around
    /// a value the program gives never hides what that value reads,
    /// `next_if_eq(&item)`'s `(item$1) => item$1 === item` (#1667, #1681
    /// change 3, M2).
    /// Of the form `base` makes (`Base::name`): one named after a
    /// temporary, `_24$1` of `_24`, a temporary's still (#1681 change 3,
    /// S4); a variable of its own, whose reads are made of it ("names are
    /// ids", ADR 1150).
    pub fn fresh<'b>(&self, base: impl Into<Base<'b>>) -> js::Name {
        let base = base.into();
        match base {
            Base::Derived(source) => return self.derived(source),
            Base::Suffixed(source, suffix) => {
                return self.unspelled_as(|k| js::Name::suffixed(Ok(source), &[], suffix, k));
            }
            _ => {}
        }
        let word = base.text();
        let name = base.name(fresh_in(&mut self.names.borrow_mut(), &word));
        // The first of its base, or given apart from that one, of it.
        let mut first_of = self.first_of.borrow_mut();
        match first_of.get(word.as_ref()) {
            Some(first) => name.given_apart_from(first),
            None => {
                first_of.insert(word.into_owned(), name.clone());
                name
            }
        }
    }

    /// A holder of `source`'s value, of it by its id, not spelled yet
    /// (`js::Name::derived`): preparation spells it of what the source is
    /// spelled then, given apart from it (ADR 1150).
    fn derived(&self, source: &js::Name) -> js::Name {
        // Never of a loop's counter: a holder is spelled before counters are
        // (`reclaim::spelled`, then `counters::spelled`), so it would take
        // its counter's letter, `i`, and push the counter to `j`. None is
        // made; this says so where one would be.
        debug_assert!(
            !source.is_counter(),
            "a holder of a loop's counter, {source:?}, spelled before it (#2032)"
        );
        self.unspelled_as(|k| js::Name::derived(source, k))
    }

    /// A name lowering makes for a variable of its own, `base`'s, not
    /// spelled yet (`js::Name::made`): spelled by preparation once the
    /// program's names are all made, apart from them, so a name it makes
    /// early, of a closure's clone before the closure's own `key` is named,
    /// never costs the program its name (#1681 change 3, M2).
    fn unspelled(&self, base: &str) -> js::Name {
        self.unspelled_as(|k| js::Name::made(base, k))
    }

    /// The first of `made`'s names, its `k`th, that the item has none of
    /// yet: an unspelled name (`unspelled`), or a holder's (`derived`).
    pub fn unspelled_as(&self, made: impl Fn(usize) -> js::Name) -> js::Name {
        let mut names = self.names.borrow_mut();
        (1..).map(made).find(|name| name.reserve_in(&mut names)).unwrap()
    }

    /// A `catch`'s parameter each time it's asked, of the item's names,
    /// each its own (#1681 change 3, M2), as preparation spells it.
    pub fn catch_names(&self) -> impl FnMut() -> js::Name + '_ {
        move || self.fresh(Role::Error)
    }

    /// The names a pattern gives the fields it leaves out, so the rest
    /// after them holds only the others (ADR 0205), each `_field`: one left
    /// out is `_` once it's known, as preparation names it.
    pub fn left_out(&mut self, keys: &[&str]) -> Vec<js::Name> {
        (keys.iter())
            .map(|key| {
                self.fresh(Base::Named(&format!(
                    "_{}",
                    camel_case(&rust_js_program::names::js_ident(key))
                )))
            })
            .collect()
    }

    /// A variable a pattern binds, rustc's `LocalVarId`, the program's:
    /// its name the program's (`js::Name::program`, ADR 1150).
    fn bind(&mut self, var: LocalVarId, name: rustc_span::Symbol, mutable: bool) -> js::Name {
        let name = self.fresh(Base::Program(&js::Name::program(
            camel_case(name.as_str()),
            name.as_u32(),
        )));
        self.locals.vars.insert(
            var,
            Var {
                place: Expr::var(&name),
                mutable,
            },
        );
        name
    }

    /// A `const` (ADR 0031). One rust-js compiled, the crate's or a
    /// library's, is its name, `SIZE` or `util.SIZE`, imported from its
    /// module, copied where a use might change it: each use is a value of
    /// its own (ADR 0770). Anyone else's, like `u32::MAX`, is its value,
    /// written in place. A
    /// trait's in generic code, `S::SIDES`, is its impl's dictionary's (ADR 0106).
    /// A use a library hands out what it stores by, `stored`, is a copy of
    /// its own, its objects' too (`copies::stored`, #1907).
    pub fn named_const(
        &mut self,
        def_id: DefId,
        args: ty::GenericArgsRef<'tcx>,
        ty: Ty<'tcx>,
        span: Span,
        stored: bool,
    ) -> R<Expr> {
        // A trait's, of a type: its impl's (ADR 0410), of one generic code
        // names too, `Self::CAPACITY` of an `impl<T, const CAP: usize>`.
        let (def_id, args) = match self.tcx.trait_of_assoc(def_id) {
            Some(_) => match crate::traits::resolve(self.tcx, self.typing_env, def_id, args) {
                Ok(Some(instance)) => (instance.def_id(), instance.args),
                _ => (def_id, args),
            },
            _ => (def_id, args),
        };
        // One of `Math`'s, `std::f64::consts::LN_2`, is its, `Math.LN2`
        // (ADR 0887).
        if let Some(constant) = std_types::number::math::constant(self.tcx, def_id) {
            return Ok(constant);
        }
        // One JS names, a binding crate's, `node::ELEMENT_NODE`, is its JS
        // name, `Node.ELEMENT_NODE` (ADR 0031, amended, rd618).
        if let Some(name) = bindings::const_link_name(self.tcx, def_id) {
            return Ok(self.binding_ref(name.as_str(), span));
        }
        // One holding a `Cell`, which changes through a shared reference, is
        // its value, written in place: `{ value: 5 }` each time it's used.
        // One made where another module loads, read where this one does, is
        // its value, which rustc computed, where it has one (`reads_by_name`).
        let by_value = || {
            (!self.reads_by_name(def_id))
                .then(|| self.const_value_of(def_id, args, span))
                .flatten()
        };
        if let Some(value) = self.is_rust_fn(def_id).then(by_value).flatten() {
            return Ok(value);
        }
        if self.is_rust_fn(def_id) && ty.is_freeze(self.tcx, self.typing_env) {
            // `fn_ref` also records the use, which is what imports its module.
            let place = self.item_read(def_id, span);
            let copied = (self.const_use_copied(ty) || stored)
                // One whose value JS holds as itself, `undefined` of one with
                // no byte written, `MaybeUninit::uninit()`'s, which nothing
                // changes (ADR 0332).
                && !self.const_value_of(def_id, args, span).is_some_and(|value| value.is_constant());
            // Where it's written, wherever it's read, so a comment before it
            // is before it: an operand's, `/* */ SnapshotStatic |
            // MaySuspendCommit`, as a call's argument's (ADR 0708, amended).
            // A `bool`'s is one, which JS's name of it doesn't show (ADR
            // 0031, amended).
            let mut place = place.or_at(self.js_span(span));
            if ty.is_bool() {
                place.facts = place.facts.with(js::Known::Boolean);
            }
            return match copied {
                true if stored => Ok(self.copy_stored(place, ty)),
                true => self.const_copy(place, ty, span),
                false => Ok(place),
            };
        }
        if let Some(value) = self.const_value_of(def_id, args, span) {
            return Ok(value);
        }
        // A `TrustedRandomAccess`'s `MAY_HAVE_SIDE_EFFECT`, as rustc
        // evaluates it; where it can't, of a type only a caller knows, true:
        // what it lets std's zip leave unread does nothing when read, so
        // reading it gives the same (ADR 0975).
        if recognition::is_steps_code_const(self.tcx, def_id) {
            return Ok(Expr::bool(recognition::steps_code(
                self.tcx,
                self.typing_env,
                args.type_at(0),
            )));
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
            _ if def_id.is_local() => self.mir_hooks().crate_const_as_code(self, def_id, args),
            _ => Err(self.unsupported(span, "this constant")),
        }
    }

    /// Whether a call of the crate's `def_id`, given `args`, gives any
    /// argument other than as it is, as `given_iterators` says: an `Option`
    /// an `IntoIterator` bound takes, the array of its items (ADR 0061). Its
    /// value is then the arrow of its call, which does so too (#281).
    fn converts_arguments(&mut self, def_id: DefId, args: ty::GenericArgsRef<'tcx>, span: Span) -> R<bool> {
        let given = (self
            .tcx
            .fn_sig(def_id)
            .instantiate(self.tcx, args)
            .skip_normalization()
            .skip_binder())
        .inputs()
        .to_vec();
        let names: Vec<js::Name> = (0..given.len())
            .map(|i| js::Name::declared(format!("arg{i}")))
            .collect();
        let mut values: Vec<Expr> = names.iter().map(Expr::var).collect();
        self.given_iterators((def_id, args), &mut values, &given, span, &mut Vec::new())?;
        Ok((values.iter().zip(&names)).any(|(value, name)| !matches!(&value.kind, js::ExprKind::Var(v) if v == name)))
    }

    /// A function as a value: a constructor's, `.map(Some)`, an arrow making
    /// what its call makes (ADR 0125); the crate's, `component(Card, props)`,
    /// its JS name, given its dictionaries; a std one's, `.map(str::trim)`,
    /// `(s) => s.trim()`; a binding's. `None` for any other, a library's
    /// among them, which the arrow of its call as MIR's is, its import by
    /// name where that's all it is (ADR 0100), and one of the crate's that
    /// takes rest arguments, whose arrow spreads them (ADR 0664).
    pub fn fn_item_value(
        &mut self,
        def_id: DefId,
        args: ty::GenericArgsRef<'tcx>,
        ty: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let js_span = self.js_span(span);
        // A constructor, made where it's given, a library's too, `.map(Wrap)`
        // of its public struct or its compiled `.map(Self)` of its private
        // one, smallvec's `ConstNonNull` (#476): no item a manifest lists.
        if matches!(self.tcx.def_kind(def_id), DefKind::Ctor(_, CtorKind::Fn)) {
            return self.constructor_value(def_id, args, span).map(Some);
        }
        // A library's function its manifest doesn't list; not a trait's
        // method, `Zero::is_zero`, which is its impl's, as a call of it is.
        if self.tcx.trait_of_assoc(def_id).is_none()
            && let Some(why) = self.krate.foreign.unlisted(def_id)
        {
            return Err(crate::refusal::error(self.tcx, span, why));
        }
        // A std formatting trait's `fmt` where a writer is shown into a
        // writer: the function that writes into the sink it claims (#1652).
        if let Some(value) = self.std_fmt_fn_value(def_id, args, span)? {
            return Ok(Some(value));
        }
        // And a library's that takes its caller's location, given the one
        // where it's defined, as rustc's shim of it calls it (ADR 0427;
        // #1032), its call's arrow giving the place it's made a value.
        let located = self.krate.foreign.item(def_id).is_some() && self.takes_caller_location(def_id);
        if self.krate.fns.contains_key(&def_id) || self.is_rust_trait_fn(def_id, args) || located {
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
                    |t| matches!(*t.kind(), ty::Ref(_, inner, ty::Mutability::Mut) if !self.mut_ref_form(inner).is_cell()),
                ) {
                    return Ok(None);
                }
                let params: Vec<js::Name> = (0..inputs.len())
                    .map(|i| self.fresh(Base::Numbered(Role::Arg, i)))
                    .collect();
                let mut values: Vec<Expr> = params.iter().map(Expr::var).collect();
                // A C-variadic one's, given on what it's given past the rest,
                // `(...args) => f(...args)` (ADR 0428).
                let variadic = variadic.then(|| self.fresh(Role::Args));
                if let Some(rest) = &variadic {
                    values.push(Expr::spread(Expr::var(rest)));
                }
                // A std trait's, `ToString::to_string` or `i32::max`: what
                // its call is.
                let Some(call) = self.trait_call(def_id, args, values, false, span, out)? else {
                    return Ok(None);
                };
                let mut patterns: Vec<js::Pattern> = params.into_iter().map(Into::into).collect();
                patterns.extend(variadic.map(js::Pattern::Rest));
                let arrow = Expr::arrow(patterns, vec![StmtKind::Return(Some(call)).at(js_span)]);
                // Its call's function, where the arrow only passes on what it's
                // given (ADR 0538).
                let taker = passed_on::Taker::FnValue;
                return Ok(Some(passed_on::passed_on(&arrow, taker, &[]).unwrap_or(arrow)));
            }
            // One that takes rest arguments, given a slice by what calls it
            // as a value: the arrow of its call, which spreads it (ADR 0664),
            // but where JS alone reads it (`rest_function_itself`); a class's
            // constructor or method, `(props) => new Component(props)` (ADR
            // 0761).
            if self.takes_rest(def_id) || self.value_is_its_call(def_id, args, span)? {
                return Ok(None);
            }
            // Of the lowering that takes what it's given as it's given it, as
            // its call is (ADR 0051, amended).
            // As its path names it, through its module's namespace where it's
            // written so (rd114, ADR 1018).
            let copy = self.called_copy((def_id, args), span)?;
            let callee = self.named_fn_ref(def_id, copy, span);
            // One given its caller's location is called as the shim it's
            // called through gives it, a `Fn` trait's call (#2335), or, a
            // `fn` pointer's, where it's defined (ADR 0427).
            let defined = self.location_at(self.tcx.def_span(def_id));
            let location = self.value_location().unwrap_or_else(|| defined.clone());
            // Given where it's defined, it's the module's one function (ADR
            // 0395, below); given a shim's location elsewhere, a `Fn` trait's
            // call's (#2335), an arrow of its own, as a function item called
            // through a trait has no pointer whose identity is seen.
            let at_definition = location.kind == defined.kind;
            // Generic code's own forms, a `&mut T`'s box and a raw pointer's
            // handle, as generic code that calls it as an `F` gives them, not
            // its callers' (#975, ADR 1114): a `fn` pointer of an instance
            // whose forms differ is refused where it's made (#1187).
            if self
                .evidence_args_with(def_id, args, location.clone(), span)?
                .is_empty()
            {
                return Ok(Some(callee));
            }
            // Each named as the function names it, `(n) => bracketed(n,
            // IsI32)`; of a writer function, but its `Formatter`, which none
            // of its JS takes nor a call through its pointer gives (ADR 0538,
            // #1710).
            let formatter = self.recognition().formatter_param(def_id);
            let param_names: Vec<String> = (signatures::taken_parameter_names(self.tcx, def_id, formatter, false))
                .into_iter()
                .enumerate()
                .map(|(i, name)| name.unwrap_or_else(|| format!("arg{i}")))
                .collect();
            let name = match &callee.kind {
                js::ExprKind::Var(name) => name.word().to_string(), // word: a module function's, its instance's name made of it
                _ => camel_case(self.tcx.item_name(def_id).as_str()),
            };
            // And its options where it takes them, which a call through the
            // pointer gives after its arguments, before its dictionaries
            // (`mir_closure_call`, #1758).
            let options = formatter.is_some() && self.takes_options(def_id);
            let made = move |cx: &mut Self| {
                let evidence = cx.evidence_args_with(def_id, args, location, span)?;
                let mut params: Vec<rust_js_program::js::Name> =
                    (param_names.iter()).map(|name| cx.fresh(Base::Named(name))).collect();
                if options {
                    params.push(cx.fresh(Role::Options));
                }
                let values = params.iter().map(Expr::var).chain(evidence).collect();
                Ok(Expr::arrow(
                    params.into_iter().map(Into::into).collect(),
                    vec![StmtKind::Return(Some(Expr::call(callee, values))).at(js_span)],
                ))
            };
            // Of an instance its module knows whole, its module's `const`,
            // so each use is the one function, as Rust's pointer to it is
            // (ADR 0395, #974); of one given its caller's dictionaries, the
            // arrow of them, a function of the call that made it.
            let whole =
                at_definition && !args.has_non_region_param() && !args.has_aliases() && !self.in_copied_default();
            if whole {
                let base = traits::instance_name(self.tcx, &name, args);
                return self.module_fn_value((ty, &base), made).map(Some);
            }
            return made(self).map(Some);
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
            return self.binding_value(def_id, args, (span, false), span).map(Some);
        }
        Ok(None)
    }

    /// `[item; count]`, the count a caller's `N` or a number (ADR 0107);
    /// `constant`, whether the item is a named constant's value.
    pub fn repeat(
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
            return Ok(crate::plain::filled(length, None));
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
            let array = Expr::new_(Expr::global("Array"), vec![length]);
            return Ok(Expr::call(Expr::member(array, "fill"), vec![item]));
        }
        // A constant's use is its copy already, `EMPTY.slice()`: made again
        // for each, as each use of a constant is.
        let copy = match constant {
            true => item,
            false => {
                let item = match item.reads_same() {
                    true => item,
                    false => self.spill(Role::Item, item, out),
                };
                self.copy(item, item_ty)
            }
        };
        let length = Expr::object(vec![Prop::Field("length".into(), length)]);
        let from = Expr::member(Expr::global("Array"), "from");
        Ok(Expr::call(
            from,
            vec![
                length,
                Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(copy)).at(js::Span::NONE)]),
            ],
        ))
    }

    /// `const <base> = value;`, so it's evaluated here, then its name, which
    /// keeps what's known of an iterator's value (`iterators::marks_kept`).
    /// Of a role that says nothing of it, `value`, the name its call gives
    /// first, `pullTwo`, which preparation spells where its scope lets it be
    /// (`Name::suggested`, #890).
    pub fn spill<'b>(&mut self, base: impl Into<Base<'b>>, value: Expr, out: &mut Vec<Stmt>) -> Expr {
        let base = base.into();
        let name = match base {
            // A holder's, and a held argument's, of their form (ADR 1150).
            Base::Derived(_) | Base::HeldArg | Base::Suffixed(..) => self.fresh(base),
            _ => self.fresh(Base::Named(&unhyphenated(&base.text()))),
        };
        let name = match base {
            Base::Role(role) if role.says_nothing() => name.suggested(held::held_names(&value), role.text()),
            _ => name,
        };
        let (span, marks) = (value.span, iterators::marks_kept(&value));
        out.push(StmtKind::Const(name.clone(), value).at(span));
        let mut held = Expr::var(&name);
        held.facts = held.facts.and(marks);
        held
    }

    pub fn unsupported(&self, span: Span, what: &str) -> crate::refusal::Refusal {
        refusal::refuse(self.tcx, span, refusal::Refused::Feature(what))
    }

    /// `unsupported`, of a stopgap: what `crossing` doesn't convert yet
    /// (ADR 1201), said the same.
    pub fn unconverted(&self, span: Span, crossing: refusal::Crossing, what: &str) -> crate::refusal::Refusal {
        refusal::refuse(
            self.tcx,
            span,
            refusal::Refused::Unconverted(crossing, &refusal::Refused::Feature(what)),
        )
    }

    /// No dictionary for `tr` was given: a component's, which React never
    /// gives one (ADR 0201), or one rust-js doesn't make.
    fn no_evidence(&self, span: Span, tr: ty::TraitRef<'tcx>) -> crate::refusal::Refusal {
        if bindings::is_component(self.tcx, self.item) {
            let what = format!("a component's `{tr}`");
            refusal::refuse(
                self.tcx,
                span,
                refusal::Refused::Because(&what, "React gives a component no dictionary"),
            )
        } else {
            self.unsupported(span, &refusal::evidence(tr))
        }
    }
}

/// What a binding's path reads, `console.log`, `react.__CLIENT_INTERNALS..`
/// of `react#*.__CLIENT_INTERNALS..` by `import`'s name of the export it's
/// in, or `require("eslint-plugin-react-hooks").rules` of one required
/// (ADRs 0028, 0305).
pub fn path_read(path: &str, required: &IndexSet<Export>, import: impl FnOnce(&Export) -> String) -> Expr {
    match bindings::js_import(path) {
        Some((export, rest)) if required.contains(&export) => bindings::read_path(bindings::required(&export), rest),
        // The import's name, which its declaration declares.
        Some((export, rest)) => rooted(&format!("{}{rest}", import(&export)), Expr::module),
        None => rooted(path, Expr::global),
    }
}

/// A path from `root` of its first name, a JS global, like `document`, or
/// an import's name: the name itself, or a path from it, like `console.log`
/// or `window['$RT']` (ADR 1033), or one read unbound, `(0, eval)`.
fn rooted(name: &str, root: fn(&str) -> Expr) -> Expr {
    if let Some(inner) = bindings::unbound(name) {
        return Expr::unbound(rooted(inner, root));
    }
    let first = bindings::first_name(name);
    bindings::read_path(root(first), &name[first.len()..])
}

/// A struct's or a variant's value, of its fields' values in declared order:
/// an object, tagged `{ TAG: "Circle", _0: r }` for a variant (ADR 0033), or
/// an array for a tuple struct (ADR 0020).
/// A field a symbol names is `[SYMBOL]: value`, that symbol's item read
/// where it's made (ADR 1051).
impl<'tcx> FnCx<'_, 'tcx> {
    fn assembled(&self, shape: Shape<'tcx>, tag: Option<(String, Expr)>, items: Vec<Expr>) -> Expr {
        match shape {
            Shape::Object(fields) => {
                let tag = tag.map(|(key, name)| Prop::Field(key, name));
                let fields = fields
                    .into_iter()
                    .zip(items)
                    .map(|((key, _), v)| match key.symbol_item() {
                        Some((item, span)) => Prop::Keyed(self.field_symbol(item, span), v),
                        None => Prop::Field(key.text().to_owned(), v),
                    });
                Expr::object(tag.into_iter().chain(fields).collect())
            }
            _ => Expr::array(items),
        }
    }
}

/// The stem of the JS file `module`'s JS is written to, which its
/// namespace is named after (`namespace_name`): a `#[path]` module's
/// file's, `DOMPluginEventSystem` of `#[path =
/// "src/events/DOMPluginEventSystem.rs"]`, as ADR 0273 places its JS
/// beside it; any other's its Rust name, its module path's last.
fn module_file_stem(tcx: TyCtxt<'_>, module: LocalModId) -> String {
    let name = || tcx.item_name(module.to_def_id()).to_string();
    if !located(tcx, module) {
        return name();
    }
    let file = module_file(tcx, module).name.clone().into_local_path();
    (file.as_deref())
        .and_then(std::path::Path::file_stem)
        .and_then(|stem| stem.to_str())
        .map_or_else(name, str::to_string)
}

/// `&x` is `x`: a reference is the value (ADR 0023).
fn without_refs<'p, 'tcx>(mut pat: &'p Pat<'tcx>) -> &'p Pat<'tcx> {
    while let PatKind::Deref { subpattern, .. } = &pat.kind {
        pat = subpattern;
    }
    pat
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    pub fn recognition(&self) -> recognition::Recognition<'_, 'tcx> {
        recognition::Recognition {
            tcx: self.tcx,
            typing_env: self.typing_env,
            trait_impls: self.krate.trait_impls,
            foreign: self.krate.foreign,
        }
    }
}

/// Translate a frontend module identity into an owned link symbol.
pub fn module_symbol(module: LocalModId, export: &str) -> rust_js_program::js::Symbol {
    rust_js_program::js::Symbol {
        module: module.to_def_id().index.as_u32(),
        export: export.to_owned(),
    }
}

/// The catalogue of std's bodies `@rust-js/std` compiles, walked, which
/// `bun run std` keeps (`--std-catalogue`, #2305).
pub fn compiled_std_catalogue(tcx: TyCtxt<'_>) -> String {
    recognition::catalogue(tcx)
}

/// Where each std function std's hook names its own line of raises its
/// panics, from its MIR, which `bun run std` keeps (`--std-raised`, #2292):
/// `Err` naming one whose sites aren't there.
pub fn std_raised(tcx: TyCtxt<'_>) -> Result<String, String> {
    recognition::raised_catalogue(tcx)
}

/// A checker of the crate's type a check calls (#2403, ADRs 1194, 1196): a
/// function of its module, or, of a struct's fields alone, its table's
/// constant, `const checkPoint = /* @__PURE__ */ $checkShape(..)`.
#[derive(Clone)]
pub enum Checker {
    Function(rust_js_program::js::Function),
    Shape(rust_js_program::js::Const),
}
