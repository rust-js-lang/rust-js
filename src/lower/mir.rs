//! A function's body from its MIR (ADR 0364): what runs, in the order it
//! runs, made structured JS. MIR says everything that happens: each value in
//! a temporary of its own, in order, each borrow, move and check. So the JS
//! is correct as it's made, and made readable only where that's proved to
//! change nothing:
//!
//! - **Structure**: blocks become labeled blocks and loops by Ramsey's
//!   "Beyond Relooper", which turns any reducible graph, as Rust's are, into
//!   `break` and `continue`.
//! - **Folding**: a temporary used once is written where it's used, where
//!   nothing it could see or that could see it runs between (`fold`).
//!
//! The function's THIR is beside it, for its shape: names and spans.

mod assign;
mod awaits;
mod bounds;
mod calls;
mod cfg;
mod closures;
mod dispatch;
mod drops;
mod entries;
mod hints;
mod iter;
mod jsx_props;
mod local_facts;
mod locals;
mod offsets;
mod operands;
mod patterns;
mod places;
mod queries;
mod rvalues;
mod std_calls;
mod structure;
mod temporaries;
mod tidy;
pub(super) use tidy::pass_names as tidy_pass_names;
mod writer;

use self::queries::*;

use std::collections::{HashMap, HashSet};

use rustc_abi::{FieldIdx, VariantIdx};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_index::IndexVec;
use rustc_middle::mir::visit::{MutatingUseContext, NonMutatingUseContext, PlaceContext, Visitor};
use rustc_middle::mir::{
    self, AggregateKind, AssertKind, BasicBlock, BinOp, BorrowKind, Const, ConstOperand, Local, LocalKind, Operand,
    Place, PlaceElem, Promoted, Rvalue, StatementKind, TerminatorKind, UnOp, UnwindAction,
};
use rustc_middle::thir;
use rustc_middle::ty::{self, Ty, TypeVisitableExt};
use rustc_mir_dataflow::move_paths::MovePathIndex;
use rustc_span::Span;

use super::combinators::{IterComb, StepOp};
use super::iterators::is_stepped_items;
use super::jsx::JsxArg;
use super::recognition::{
    BuilderStep, DebugBuilder, PureStd, Std, StdItem, TraitCall, is_std_def, known_by_bounds, pure_std, std_item,
    trait_method,
};
use super::refusal::{Refused, refuse};
use super::representation::variant_field;
use super::std_types::cow::CowOp;
use super::std_types::map::MapOp;
use super::std_types::number::NumOp;
use super::std_types::rc::RcOp;
use super::std_types::slice::SliceOp;
use super::std_types::text::{StringEdit, TextOp, range_read};
use super::{Body, FnCx, LoweredFn, R, Shape, bindings, fn_def};
use crate::js::{self, Expr, Op, Prop, Stmt, StmtKind};

/// A body's MIR, as borrowck reads it (ADR 0364), and its promoted constants.
pub struct Mir<'tcx> {
    pub(super) body: mir::Body<'tcx>,
    pub(super) promoted: IndexVec<Promoted, mir::Body<'tcx>>,
    /// Of a coroutine's, an `async` body's: the blocks whose call is an
    /// `.await` (`awaits.rs`).
    pub(super) awaits: std::collections::HashSet<BasicBlock>,
}

impl<'tcx> Mir<'tcx> {
    /// It, of a function's generics, in another's terms, `args` of them: a
    /// closure's whose function rustc's MIR inlined (ADR 0899).
    pub(super) fn instantiated(&self, tcx: ty::TyCtxt<'tcx>, args: ty::GenericArgsRef<'tcx>) -> Self {
        let of = |body: &mir::Body<'tcx>| {
            ty::EarlyBinder::bind(tcx, body.clone())
                .instantiate(tcx, args)
                .skip_normalization()
        };
        Mir {
            body: of(&self.body),
            promoted: self.promoted.iter().map(of).collect(),
            awaits: self.awaits.clone(),
        }
    }

    /// `body`, its `.await`s each one call, and what can't throw never
    /// unwinding.
    pub(super) fn new(
        tcx: ty::TyCtxt<'tcx>,
        mut body: mir::Body<'tcx>,
        promoted: IndexVec<Promoted, mir::Body<'tcx>>,
    ) -> Mir<'tcx> {
        // A call that can't throw, a binding marked so or `unwrap_unchecked`,
        // has nothing to clean up (ADR 0301): rustc's dataflow of what's there,
        // which says what's dropped, follows no unwinding of it.
        for data in body.basic_blocks.as_mut() {
            if let TerminatorKind::Call { func, unwind, .. } = &mut data.terminator_mut().kind
                && let Some((id, _)) = func.const_fn_def()
                && match pure_std(tcx, id) {
                    Some(PureStd::Question | PureStd::BoxNew | PureStd::Unchecked) => true,
                    _ => bindings::is_binding(tcx, id) && bindings::cannot_throw(tcx, id),
                }
            {
                *unwind = UnwindAction::Unreachable;
            }
        }
        // An optimized body's, std's, switch that goes one way whatever it
        // tests, which drop elaboration leaves of a flag it settled, is the
        // way it goes; the discriminant it read and nothing else does, nothing.
        if !body.source.def_id().is_local() {
            // A block that assumes `UbChecks`, which rust-js reads as off,
            // never runs (ADR 0917).
            for data in body.basic_blocks.as_mut() {
                if crate::lower::std_coverage::assumes_ub_checks(data) {
                    data.statements.clear();
                    data.terminator_mut().kind = TerminatorKind::Unreachable;
                }
            }
            let unreachable: Vec<BasicBlock> = (body.basic_blocks.iter_enumerated())
                .filter(|(_, data)| matches!(data.terminator().kind, TerminatorKind::Unreachable))
                .map(|(block, _)| block)
                .collect();
            for data in body.basic_blocks.as_mut() {
                let terminator = data.terminator_mut();
                if let TerminatorKind::SwitchInt { targets, .. } = &terminator.kind {
                    let mut ways: Vec<BasicBlock> = (targets.all_targets().iter().copied())
                        .filter(|b| !unreachable.contains(b))
                        .collect();
                    ways.dedup();
                    if let [way] = ways[..] {
                        terminator.kind = TerminatorKind::Goto { target: way };
                    }
                }
            }
            // An `assume` of what an inlined `?` knows of its residual, which
            // runs nothing, goes; and a discriminant read for a switch that
            // went, which the block's terminator no longer tests and nothing
            // else reads: dropped with it.
            for data in body.basic_blocks.as_mut() {
                data.statements.retain(|statement| {
                    !matches!(&statement.kind, StatementKind::Intrinsic(intrinsic)
                        if matches!(**intrinsic, mir::NonDivergingIntrinsic::Assume(_)))
                });
                let tested = match &data.terminator().kind {
                    TerminatorKind::SwitchInt { discr, .. } => discr.place().and_then(|place| place.as_local()),
                    _ => None,
                };
                let mut read = ReadLocals(Vec::new());
                for statement in &data.statements {
                    Visitor::visit_statement(&mut read, statement, mir::Location::START);
                }
                data.statements.retain(|statement| {
                    !matches!(&statement.kind, StatementKind::Assign(assign)
                        if matches!(assign.1, Rvalue::Discriminant(_))
                            && assign.0.as_local().is_some_and(|local| tested != Some(local) && !read.0.contains(&local)))
                });
            }
        }
        let awaits = match body.coroutine.is_some() {
            true => awaits::awaits(tcx, &mut body),
            false => Default::default(),
        };
        Mir { body, promoted, awaits }
    }
}

/// The locals a body reads, as an operand or in a place: what its
/// optimized form's dead switches leave unread is dropped with them.
struct ReadLocals(Vec<Local>);

impl<'tcx> Visitor<'tcx> for ReadLocals {
    fn visit_local(&mut self, local: Local, context: PlaceContext, _: mir::Location) {
        if !matches!(context, PlaceContext::MutatingUse(MutatingUseContext::Store)) && !self.0.contains(&local) {
            self.0.push(local);
        }
    }
}

/// What a temporary holds until it's used: a value, or a part of a
/// `format_args!` that only its `Arguments` shows.
#[derive(Clone)]
enum Value<'tcx> {
    Expr(Expr),
    /// An argument of a `format_args!`: how it's shown, its type, and it.
    Fmt(Std, Ty<'tcx>, Expr),
    /// An array of values, `format_args!`'s arguments among them.
    List(Vec<Value<'tcx>>),
    /// The variant of an enum, read to choose a branch: the enum, its type,
    /// and whether it's an untagged one given of what's there, never
    /// `undefined` nor `null`, `classify(Some(value))`'s (ADR 0676).
    Discriminant(Expr, Ty<'tcx>, bool),
    /// A shared reference to a place, which nothing changes while it's held
    /// (borrowck says so): the place, read where it's used.
    Place(Expr),
    /// A `&mut` to a place JS can't share, a number's: the place, read and
    /// written through where it's used, or by a closure that captured it.
    Ref(Expr),
    /// What `?`'s `Try::branch` makes of an `Option` or a `Result`: the
    /// value tried, and its type, whose `None` or `Err` is its `Break`.
    Branch(Expr, Ty<'tcx>),
    /// An `Option`'s variant, read to choose a branch, where the test that
    /// it's `Some` is known: a place found, `at !== -1`, or a strip's, `s.startsWith(p)` (ADR 0642).
    Tested(Expr),
    /// What a strip whose `Option` is only tested and taken apart makes: its
    /// test and what it gives where it holds (ADR 0642).
    Cut(Expr, Expr),
}

/// A parameter taken apart: its index, THIR's pattern, and the local MIR
/// binds each name the pattern binds to.
type ParamPattern = (usize, js::Pattern, Vec<(Local, String)>);

/// What's known of a body's locals.
/// What lowering a body from its MIR found of its variables, which goes
/// with them where a body inside it is lowered (`Locals`): each read and
/// written here alone, and asked of by the rest, `alike_local`.
#[derive(Default)]
pub(super) struct Found {
    /// Of a body lowered from its MIR (ADR 0364): the names of its locals
    /// that read the same wherever they're read, assigned once and never
    /// borrowed, and of those of plain Rust data, whose fields don't change
    /// either, which `reads_alike` and `plain_value` ask of.
    /// And of the others' arrays, their items, `...items`.
    pub(super) alike: HashSet<String>,
    pub(super) plain: HashSet<String>,
    /// And the places that hold its cells, `&mut`s to what JS can't change
    /// in place: a box or a handle, whose `value` is what it points at,
    /// unlike a struct's field of that name. Its locals', by name, and a
    /// place a reborrow found one at, `o.value` of `&mut **u`.
    pub(super) cells: Vec<Expr>,
    /// Of a body lowered from its MIR, the temporaries a function that skips
    /// what's falsy is given, whose `test ? value : undefined` is given as
    /// `test && value` (`#[rust_js::skips_falsy]`) where it's moved to.
    pub(super) falsy: HashSet<String>,
    /// And the temporaries such a `test && value` tests: read for their
    /// truth alone.
    pub(super) tested: HashSet<String>,
    /// Of a body lowered from its MIR, the `const`s it made of what an
    /// element made ahead of a branch reads, which tidy reads in place
    /// where it can.
    pub(super) captured: HashSet<String>,
    /// Of a body lowered from its MIR, where each closure it makes that
    /// passes on what it's given to Rust's function is written: an array
    /// method's, the function itself (ADR 0279).
    pub(super) forwarding: Vec<js::Span>,
    /// And where each is written of one call whose JS gives `undefined`
    /// anyway, and how: a handler's, or one of its value, `() => f()` (ADRs
    /// 0040, 0883).
    pub(super) concise: Vec<(js::Span, crate::lower::jsx::OneCall)>,
    /// Of a body lowered from its MIR, each field path it reads,
    /// `element.editable`, and whether it's a `bool` each time.
    pub(super) paths: HashMap<String, bool>,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `name`, a local of the body lowered from its MIR, reads the
    /// same wherever it's read: assigned once and never borrowed.
    pub(super) fn alike_local(&self, name: &str) -> bool {
        self.locals.mir.alike.contains(name)
    }

    /// Whether `name`, such a local, is plain Rust data, whose fields don't
    /// change either.
    pub(super) fn plain_local(&self, name: &str) -> bool {
        self.locals.mir.plain.contains(name)
    }

    /// `name`, a temporary a `test && value` given what skips what's falsy
    /// tests: read for its truth alone.
    pub(super) fn note_tested(&mut self, name: &str) {
        self.locals.mir.tested.insert(name.to_string());
    }

    /// `name`, a temporary given to what skips what's falsy, whose `test ?
    /// value : undefined` is given as `test && value` where it's moved to.
    pub(super) fn note_falsy(&mut self, name: &str) {
        self.locals.mir.falsy.insert(name.to_string());
    }
}

struct Locals {
    /// Each one's JS name, empty for one of `()`, which holds nothing.
    names: IndexVec<Local, String>,
    /// How many times each is read, as an operand or a place's base.
    reads: IndexVec<Local, usize>,
    /// How many times each is assigned.
    writes: IndexVec<Local, usize>,
    /// Each one whose place is borrowed: it stays a variable.
    borrowed: IndexVec<Local, bool>,
    /// The fields each is read by, as an operand, `copy _5.0`.
    fields: IndexVec<Local, Vec<usize>>,
    /// Each one a `&mut` is the place of, a number's: a call made while
    /// it lives, a closure's say, may assign it.
    changed: IndexVec<Local, bool>,
    /// Each one declared already.
    declared: IndexVec<Local, bool>,
    /// Each one the program names, not a macro of std's.
    user: IndexVec<Local, bool>,
    /// Each one a `String`, which `push_str` appends to by `+`.
    text: IndexVec<Local, bool>,
}

impl Locals {
    /// What tidy's last passes ask of them, by name: those the program
    /// names (`user`), and its `String`s (`text`).
    fn named(&self) -> tidy::Named {
        let names = |of: &IndexVec<Local, bool>| -> HashSet<String> {
            (self.names.iter_enumerated())
                .filter(|&(local, name)| of[local] && !name.is_empty())
                .map(|(_, name)| name.clone())
                .collect()
        };
        tidy::Named {
            user: names(&self.user),
            text: names(&self.text),
        }
    }
}

/// What a body's writes say of its `Option`s.
#[derive(Default)]
struct OptionWrites {
    /// The `Option`s made only `Some(..)` or `None` and read only by
    /// `#[rust_js::nullable]` fields: their `None` is `null` where it's made,
    /// `c ? v : null` (ADR 0275).
    null_nones: std::collections::HashSet<Local>,
    /// The locals only ever `Some(..)`, and shared references to one: never
    /// `None`, so `==` to one is `===` (`strictly_equal`).
    made_some: std::collections::HashSet<Local>,
    /// The locals a `js::SymbolFor::from_value` gives whose value is only
    /// tested against the registry's symbols (`registered_reads`): the
    /// value read itself, a `Some` wherever it's tested, as each case
    /// compares it by `===` (ADR 0634).
    registered: std::collections::HashSet<Local>,
    /// The locals a downcast of an unwind's payload to a `&JsError` gives,
    /// written once, and `?` of one: what was thrown, a `Some` wherever it's
    /// tested, a falsy one's too (`FnCx::finds_thrown`, ADR 0710); each its
    /// discriminant, `Some`'s, or `?`'s `Continue`'s.
    thrown: HashMap<Local, u128>,
    /// The untagged enums a call gives of a `Some(..)`, `js::classify(Some(value))`'s
    /// (`body_queries::present_calls`): never `undefined` nor `null` (ADR 0676).
    present: std::collections::HashSet<Local>,
    /// Where each guard `if let Some(v) = value` is written that holds where
    /// its arm of `match js::classify(value)` is reached
    /// (`body_queries::held_guards`): its test of `value` a `Some`'s.
    held: std::collections::HashSet<Span>,
}

/// A body's lowering state.
struct State<'m, 'tcx> {
    body: &'m mir::Body<'tcx>,
    /// What it knows of the body before it writes any of it (`local_facts.rs`).
    facts: local_facts::LocalFacts<'m, 'tcx>,
    /// What naming its locals settles, its boxes, borrowed names and drop
    /// flags (`local_facts::NamingFacts`), once it's named them.
    naming: local_facts::NamingFacts,
    /// As its structure is made, the labeled block, of `facts`' jumps, each
    /// MIR block a `break` of one goes to ends (ADR 0742).
    left: HashMap<BasicBlock, usize>,
    locals: Locals,
    /// Temporaries made and not yet used, in the order they were made.
    pending: Vec<(Local, Value<'tcx>)>,
    /// Each list read by its fields, once for each taken.
    taken: Vec<Local>,
    /// The blocks being lowered whose label a `break` or a `continue` names.
    labels: Vec<BasicBlock>,
    /// The names of the body's own locals: another variable, a static's, may
    /// change by any call.
    own_names: std::collections::HashSet<String>,
    /// The cleanup the temporaries made and not yet used unwind to, if one
    /// that drops something: the statement that runs them is its `try`.
    unwind: Option<BasicBlock>,
    /// Each local that's a `&mut` to a number's place that's always the
    /// same, `COUNT.value`: the place, named where it's used.
    refs: std::collections::HashMap<Local, Expr>,
    /// The boxes a call is given for its `&mut`s to numbers, each copied
    /// back to its place once the call returns (ADR 0074); an object's, in
    /// place, which each name for it sees (ADR 0147).
    copy_backs: Vec<(Expr, String, bool)>,
    /// Each local holding a read whose `Option` only `.flatten()` reads,
    /// the JS read itself (ADR 0311).
    flattened: std::collections::HashSet<Local>,
    /// Each local holding what `?`'s `Try::branch` made (`Value::Branch`).
    branches: std::collections::HashMap<Local, (Expr, Ty<'tcx>)>,
    /// Each local holding an update's derived `Default` base, its fields'
    /// defaults, each read where its field is (`default_base_at`).
    defaults: std::collections::HashMap<Local, Vec<Expr>>,
    binding_var: bool,
    /// Values made, each written where it's read: a constant, or what the
    /// module names, which reads the same anywhere, and an element, inside another or given its props,
    /// as JSX writes it, what it's made of read where it was made (ADR 0040).
    in_place: std::collections::HashMap<Local, Expr>,
    /// Each value read in place assigned before a drop instead, whose `let`
    /// the function's body declares first (ADR 0451).
    settled: Vec<Local>,
    /// Each temporary a map of an array's items is made in, `$iter(v).map(f)`:
    /// that, which `next_back` steps from its source's end.
    maps: std::collections::HashMap<Local, Expr>,
    /// Each local read by its name already, a subject a branch tests: made
    /// as a variable, never written where it's read.
    named: std::collections::HashSet<Local>,
    /// Each local a parameter's pattern binds, and the parameter it's moved
    /// out of, which the pattern takes apart already.
    parts: std::collections::HashMap<Local, Local>,
    /// Each shared reference to a place written where it's read, and each
    /// tuple of them: the place, read as one, which a closure captures as it
    /// is, or the tuple's list of them.
    places: std::collections::HashMap<Local, Value<'tcx>>,
    /// The indexes whose bounds check is where the element is read, `false`,
    /// `$index(xs, i)`, or written, `true`, `xs[$at(xs, i)]`
    /// (`deferred_check`).
    deferred: std::collections::HashMap<Local, bool>,
    /// The block whose statements are being lowered, and the statement of
    /// it, its terminator's past the last.
    at: BasicBlock,
    statement: usize,
    /// Each strip whose `Option` is read as a test and a cut (ADR 0642):
    /// its test, `s.startsWith(p)`, and what it gives, `s.slice(1)`.
    cuts: HashMap<Local, (Expr, Expr)>,
    /// A writer's `Formatter` (ADR 0054): the locals that are it, and the
    /// string it writes to, which the writer gives back.
    writer: Option<(Vec<Local>, String)>,
    /// Of a closure that writes to its maker's `Formatter`, which it
    /// captured: the string it writes to is its maker's, which it neither
    /// makes nor gives back (ADR 0539).
    writes_captured: bool,
    /// In a function that gives a value's `Result`, what `?` of a write's
    /// result throws on (ADR 0462): its error's type, and how it's made the
    /// function's error, which its end catches and gives as its `Err`.
    thrown_writes: Option<(Ty<'tcx>, Option<super::std_types::result::Conversion>)>,
    /// A writer's `Debug` builders (ADR 0136): each local that's one, to
    /// the one its `Formatter` made, and what that's been given.
    builders: HashMap<Local, Local>,
    built: HashMap<Local, Built<'tcx>>,
}

impl<'tcx> State<'_, 'tcx> {
    /// Where the statement being lowered is, `at`'s `statement`.
    fn location(&self) -> mir::Location {
        mir::Location {
            block: self.at,
            statement_index: self.statement,
        }
    }

    /// The `let` of each value settled before a drop (`settle_reads`),
    /// first in the function's body.
    fn settled_lets(&self) -> impl Iterator<Item = Stmt> + '_ {
        (self.settled.iter()).map(|&local| StmtKind::Let(self.locals.names[local].clone(), None).at(js::Span::NONE))
    }
}

/// A `Debug` builder's kind, a `DebugStruct` say, its type's name, and
/// each step, `field` and the like, of its arguments and their types; or of
/// one held in a variable, the array its steps add their parts to as
/// they're taken (ADR 0532).
type Built<'tcx> = (
    DebugBuilder,
    Option<Expr>,
    Vec<(BuilderStep, Vec<(Expr, Ty<'tcx>)>, ty::GenericArgsRef<'tcx>)>,
    Option<String>,
);

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    pub(super) fn lower_fn_mir(&mut self, body: &Body<'tcx>, mir: &Mir<'tcx>) -> R<LoweredFn> {
        let def_id = body.def_id;
        let span = self.tcx.def_span(def_id);
        if self.krate.fns.get(&def_id).is_none() {
            return Err(self.unsupported(span, "this item from its MIR"));
        }
        // A component's props with a default, given where JS takes them apart
        // (ADR 0212).
        for param in &self.frame.thir.params {
            self.check_props_taken_apart(param.ty, param.pat.as_deref(), param.ty_span.unwrap_or(span))?;
        }
        // Its dictionaries, for its bounds, after its parameters (ADR 0049).
        let evidence = self.evidence_params(def_id);
        let mut out = Vec::new();
        let (mut params, is_async) = self.mir_signature(body, mir, &mut out)?;
        // A C-variadic one's last parameter gathers the rest, which nothing
        // can come after (ADR 0428).
        if super::recognition::is_c_variadic(self.tcx, def_id) && !evidence.is_empty() {
            return Err(self.unsupported(span, "a C-variadic function given a dictionary, a fact or a drop"));
        }
        // So does one's marked `#[rust_js::rest]` (ADR 0664).
        if self.takes_rest(def_id) && !evidence.is_empty() {
            return Err(self.tcx.dcx().span_err(
                span,
                "rust-js: a function whose last parameter is `#[rust_js::rest]` given a dictionary, a fact, \
                 a drop or its caller's location, which JS's rest parameter can't come before",
            ));
        }
        // Where each is written, of a body whose JS takes its parameters as
        // it writes them, one each.
        let param_spans: Vec<js::Span> = match body.def_id.as_local() {
            Some(local)
                if self
                    .tcx
                    .hir_maybe_body_owned_by(local)
                    .is_some_and(|b| b.params.len() == params.len()) =>
            {
                let written = self.tcx.hir_body_owned_by(local).params;
                written.iter().map(|p| self.js_span(p.span)).collect()
            }
            _ => Vec::new(),
        };
        params.extend(evidence);
        // The drops it used, which its callers give it (ADR 0300).
        self.note_drop_uses(def_id);
        Ok(LoweredFn {
            function: js::Function {
                name: self.krate.fns[&def_id].name.clone(),
                params,
                body: out,
                // A constant's initializer, its module's `const`'s value, isn't
                // a function anyone calls; a library's function the crate
                // compiles itself is its own, which no other crate or JS
                // calls (ADR 0914).
                export: matches!(self.tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn)
                    && self.tcx.visibility(def_id).is_public()
                    && !self.krate.foreign.is_instance(def_id)
                    && (self.tcx.def_kind(def_id) != rustc_hir::def::DefKind::AssocFn
                        || self.tcx.inherent_impl_of_assoc(def_id).is_some()),
                is_async,
                noinline: super::recognition::is_noinline(self.tcx, def_id),
                // A body of the crate's maps to where it's written; a std one,
                // read from its MIR, nowhere (ADR 0364, 0561).
                span: match body.def_id.as_local() {
                    Some(local) => {
                        let whole = self.tcx.hir_span_with_body(self.tcx.local_def_id_to_hir_id(local));
                        self.js_span(function_start(self.tcx, whole, self.tcx.def_ident_span(def_id)))
                    }
                    None => js::Span::NONE,
                },
                name_span: self
                    .tcx
                    .def_ident_span(def_id)
                    .map_or(js::Span::NONE, |s| self.js_span(s)),
                param_spans,
            },
            jsx: self.jsx,
            dependencies: self.dependencies.take(),
        })
    }

    /// A body's parameters and, in `out`, its statements, from its MIR, and
    /// whether it's `async`: a function's, or a trait's default copied into
    /// an impl (ADR 0176), whose dictionaries its caller gives.
    pub(super) fn mir_signature(
        &mut self,
        body: &Body<'tcx>,
        mir: &Mir<'tcx>,
        into: &mut Vec<Stmt>,
    ) -> R<(Vec<js::Pattern>, bool)> {
        let def_id = body.def_id;
        let span = self.tcx.def_span(def_id);
        let (mut params, mut out) = self.mir_function(mir, None, None)?;
        // An `async fn`'s body is its coroutine's, an async arrow called
        // at once: the async function's own (ADR 0029).
        // An `on_load!` body is its module's top, where JS has no `return`:
        // each a `break` out of a labeled block of it (ADR 0267).
        if bindings::is_on_load(self.tcx, def_id) {
            let label = self.fresh("load");
            let mut returns = false;
            js::statement_lists(&mut out, &mut |stmts| {
                for stmt in stmts.iter_mut() {
                    if let StmtKind::Return(None) = stmt.kind {
                        stmt.kind = StmtKind::Break(Some(label.clone()));
                        returns = true;
                    }
                }
            });
            if returns {
                out = vec![StmtKind::Labeled(label, out).at(js::Span::NONE)];
            }
        }
        let is_async = self.tcx.asyncness(def_id).is_async();
        if is_async {
            let Some(body) = called_async_body(&mut out) else {
                return Err(self.unsupported(span, "this `async fn`, from its MIR"));
            };
            out = body;
            // Its parameters as its body takes them apart, `{ params }`.
            tidy::destructured_params(&mut params, &mut out);
            tidy::copied_params(&params, &mut out);
        }
        // The functions written in its body, each declared before what's
        // written after it (ADR 0308), and its `const`s (ADR 0031,
        // amended); a closure's in its own, where `mir_closure` put them,
        // unless it's lowered otherwise.
        let mut placed = std::collections::HashSet::new();
        js::each_expr_mut(&mut out, &mut |e| {
            if let js::ExprKind::ItemHole(index) = e.kind {
                placed.insert(index);
            }
        });
        let written: Vec<rustc_span::def_id::DefId> = (self.krate.local_functions.keys())
            .chain(self.krate.local_consts.keys())
            .filter(|&&id| {
                let mut parent = self.tcx.parent(id);
                while self.tcx.is_closure_like(parent) {
                    parent = self.tcx.parent(parent);
                }
                parent == def_id && !placed.contains(&id.index.as_u32())
            })
            .copied()
            .collect();
        self.place_written(written, &mut out);
        tidy::unsuffixed_params(&mut out, self.module_names);
        out.splice(0..0, self.directives(body.def_id));
        into.extend(out);
        Ok((params, is_async))
    }

    /// A body's parameters and statements. A closure's, given what it
    /// captured, reads them in place of its environment's fields.
    /// A parameter or a result of a type JS can't hold, a `&mut` to a
    /// `&mut [i32]` whose place it can't share: refused, of a body whose
    /// parameters are past its first `skip` and aren't its `formatter`.
    fn check_signature_types(
        &self,
        mir_body: &mir::Body<'tcx>,
        (skip, formatter): (usize, Option<Local>),
        def_span: Span,
    ) -> R<()> {
        let closure = self.tcx.is_closure_like(mir_body.source.def_id());
        let last = mir_body.args_iter().last();
        for local in mir_body.args_iter().skip(skip).filter(|&l| Some(l) != formatter) {
            let ty = mir_body.local_decls[local].ty;
            // Rest arguments as a value are a closure's last parameter, JS's
            // rest parameter, which each call through its type spreads (ADR
            // 0851); a function's is its last slice marked `#[rust_js::rest]`
            // (ADR 0664), which a call by its name spreads.
            if super::bindings::is_rest_value(self.tcx, ty) && !(closure && Some(local) == last) {
                let refused = match closure {
                    true => super::refusal::Refused::Because(
                        "a closure's `&dyn js::Rest<T>` parameter, rest arguments as a value, before its last",
                        "JS's rest parameter is a function's last",
                    ),
                    false => super::refusal::Refused::Because(
                        "a `&dyn js::Rest<T>` parameter, rest arguments as a value, of a Rust function",
                        "a function's rest parameter is its last slice marked `#[rust_js::rest]`",
                    ),
                };
                return Err(super::refusal::refuse(self.tcx, def_span, refused));
            }
            self.check_value_ty(ty, def_span)?;
        }
        if mir_body.coroutine.is_none() && formatter.is_none() {
            self.check_value_ty(mir_body.return_ty(), def_span)?;
        }
        Ok(())
    }

    /// A C-variadic function's last parameter, its `VaList`, gathers what it's
    /// given past the rest, `...args`: the array of them (ADR 0428); and so
    /// does a function's slice marked `#[rust_js::rest]`, the array of what
    /// JS gives it (ADR 0664); and a closure's last parameter of rest
    /// arguments as a value, `&dyn js::Rest<T>`, the array of them (ADR 0851).
    fn gather_rest(&self, mir_body: &mir::Body<'tcx>, params: &mut Vec<js::Pattern>) {
        let def_id = mir_body.source.def_id();
        let rest_value = (mir_body.args_iter().last())
            .is_some_and(|last| super::bindings::is_rest_value(self.tcx, mir_body.local_decls[last].ty));
        if (super::recognition::is_c_variadic(self.tcx, def_id) || self.takes_rest(def_id) || rest_value)
            && let Some(js::Pattern::Name(name)) = params.pop()
        {
            params.push(js::Pattern::Rest(name));
        }
    }

    fn mir_function(
        &mut self,
        mir: &Mir<'tcx>,
        captures: Option<Vec<(Expr, bool)>>,
        seed: Option<offsets::Seed>,
    ) -> R<(Vec<js::Pattern>, Vec<Stmt>)> {
        let mir_body = &mir.body;
        // What THIR's drop facts refuse of its closures holding what has a
        // destructor.
        self.check_closure_drops()?;
        let outer_falsy = std::mem::take(&mut self.locals.mir.falsy);
        let outer_tested = std::mem::take(&mut self.locals.mir.tested);
        let outer_captured = std::mem::take(&mut self.locals.mir.captured);
        let drops = self.mir_elaboration(mir_body);
        let patterns = match captures {
            None => self.mir_param_patterns(mir_body),
            Some(_) => Vec::new(),
        };
        let named: Vec<(Local, String)> = patterns.iter().flat_map(|(_, _, named)| named.clone()).collect();
        let mut locals = self.mir_locals(mir_body, drops.as_ref(), &named);
        // A drop reads what it drops more than once, so it's a variable.
        let dropped: Vec<Local> = drops.iter().flat_map(|drops| drops.read(mir_body)).collect();
        for &local in &dropped {
            locals.reads[local] += 2;
        }
        let given = captures.clone().unwrap_or_default();
        let (facts, held_bindings) = self.local_facts(mir, (&locals, drops, given), seed.as_ref());
        let mut state = State {
            body: mir_body,
            facts,
            left: Default::default(),
            locals,
            pending: Vec::new(),
            taken: Vec::new(),
            labels: Vec::new(),
            naming: Default::default(),
            own_names: Default::default(),
            unwind: None,
            refs: Default::default(),
            copy_backs: Vec::new(),
            branches: Default::default(),
            defaults: Default::default(),
            binding_var: false,
            flattened: Default::default(),
            in_place: Default::default(),
            settled: Vec::new(),
            maps: Default::default(),
            named: Default::default(),
            parts: Default::default(),
            places: Default::default(),
            deferred: Default::default(),
            at: BasicBlock::from_usize(0),
            statement: 0,
            cuts: Default::default(),
            writer: None,
            writes_captured: false,
            thrown_writes: None,
            builders: Default::default(),
            built: Default::default(),
        };
        self.mir_held_guards(&mut state, &held_bindings);
        // A writer's `Formatter`: no parameter, but the string it writes to,
        // `let f = ""`, which it gives back, its `fmt::Result` nothing. A
        // closure's, after its environment.
        let formatter = (mir_body.coroutine.is_none()
            && matches!(
                self.tcx.def_kind(mir_body.source.def_id()),
                DefKind::Fn | DefKind::AssocFn | DefKind::Closure
            ))
        .then(|| self.formatter_param(mir_body.source.def_id()))
        .flatten()
        .map(|i| Local::from_usize(i + 1 + usize::from(captures.is_some())));
        let named = formatter.map(|param| (param, state.locals.names[param].clone()));
        let named = named.as_ref().map(|(param, name)| (*param, name.as_str()));
        if let Some((aliases, name)) = self.body_writer(&mut state, mir_body, named, captures.as_deref())? {
            state.builders = self.writer_builders(mir_body, &aliases);
            // Given where a writer goes, the crate's generic `fmt::Write`
            // code: a `&mut` to the string, boxed and taken back as a
            // `String`'s is (ADR 0180).
            for &alias in &aliases {
                state.refs.insert(alias, Expr::var(&name));
            }
            for &alias in aliases.iter().chain(state.builders.keys()) {
                state.locals.names[alias] = String::new();
                state.locals.declared[alias] = true;
            }
            state.locals.names[mir::RETURN_PLACE] = String::new();
            state.writer = Some((aliases, name));
        }
        // Its `Formatter`'s options, where the crate gives any (ADRs 0058,
        // 0137): a parameter after the others.
        let options = (formatter.is_some() && self.writers_take_options()).then(|| self.fresh("options"));
        // A closure's first argument is its environment, read through its fields.
        // A coroutine's are that and what it's resumed with, which an async
        // function has neither of.
        let skip = match (captures.is_some(), mir_body.coroutine.is_some()) {
            (_, true) => mir_body.arg_count,
            (true, false) => 1,
            (false, false) => 0,
        };
        let def_span = self.tcx.def_span(mir_body.source.def_id());
        self.check_signature_types(mir_body, (skip, formatter), def_span)?;
        let mut boxes = HashSet::new();
        let mut params: Vec<js::Pattern> = mir_body
            .args_iter()
            .skip(skip)
            .filter(|&local| Some(local) != formatter)
            .map(|local| {
                state.locals.declared[local] = true;
                // One the program names nothing and reads nowhere, `_: Ref`,
                // is `_`.
                if !state.locals.user[local] && state.locals.reads[local] == 0 && !state.locals.borrowed[local] {
                    state.locals.names[local] = String::new();
                }
                let name = match state.locals.names[local].as_str() {
                    "" => self.fresh("_"),
                    name => name.to_string(),
                };
                state.locals.names[local].clone_from(&name);
                // A `&mut` to a value JS can't change in place: the box its
                // caller gives, its place the box's `value` (ADR 0074).
                let boxed = match captures {
                    // A closure's, by what it points at.
                    Some(_) => matches!(mir_body.local_decls[local].ty.kind(),
                        ty::Ref(_, pointee, ty::Mutability::Mut) if self.is_cell_pointee(*pointee)),
                    None => self.param_is_box(mir_body.source.def_id(), local.as_usize() - 1),
                };
                if boxed {
                    boxes.insert(name.clone());
                }
                js::Pattern::Name(name)
            })
            .collect();
        // Where each part a pattern binds is a local MIR moves out of the
        // parameter, once, and that's all it reads of it: the pattern.
        for (i, pattern, named) in patterns {
            let arg = Local::from_usize(i + 1);
            let moved_out = named.iter().all(|&(local, _)| {
                mir_body.basic_blocks.iter().flat_map(|data| &data.statements).any(|statement| {
                    matches!(&statement.kind, StatementKind::Assign(assign)
                        if assign.0 == Place::from(local)
                            && matches!(&assign.1, Rvalue::Use(Operand::Move(p) | Operand::Copy(p), _) if p.local == arg))
                })
            });
            // Its drops, of what's moved out, as its parts', read it too.
            let drops = 2 * dropped.iter().filter(|&&local| local == arg).count();
            if !moved_out || state.locals.reads[arg] - drops != named.len() {
                continue;
            }
            for (local, _) in named {
                state.locals.declared[local] = true;
                state.parts.insert(local, arg);
            }
            params[i] = pattern;
        }
        let borrowed_names = (state.locals.borrowed.iter_enumerated())
            .filter(|&(_, &borrowed)| borrowed)
            .map(|(local, _)| state.locals.names[local].clone())
            .collect();
        state.own_names = state.locals.names.iter().filter(|n| !n.is_empty()).cloned().collect();
        for (local, name) in state.locals.names.iter_enumerated() {
            if !name.is_empty() && self.is_cell(mir_body.local_decls[local].ty) {
                self.locals.mir.cells.push(Expr::var(name));
            }
        }
        // What reads the same wherever it's read.
        for (local, name) in state.locals.names.iter_enumerated() {
            if name.is_empty()
                || written_apart(&state, local)
                || state.locals.borrowed[local]
                || state.locals.changed[local]
            {
                continue;
            }
            self.locals.mir.alike.insert(name.clone());
            let is_plain = |ty: Ty<'tcx>| {
                ty.walk().all(|arg| match arg.kind() {
                    ty::GenericArgKind::Type(t) => {
                        !self.refers_mutably(t)
                            // Inside a binder, a function pointer's parameter, unknown.
                            && !t.has_escaping_bound_vars()
                            && t.is_freeze(self.tcx, self.typing_env)
                            && !self.is_js_object(t)
                            // A counted `Rc`'s or `Weak`'s counts change through
                            // what shares it (ADR 0320).
                            && self.counted_rc(t).is_none()
                            && self.counted_weak(t).is_none()
                    }
                    _ => true,
                })
            };
            let plain = is_plain(mir_body.local_decls[local].ty);
            // What's to through a `&mut` changes by what it's given to.
            let through_mut = matches!(
                mir_body.local_decls[local].ty.kind(),
                ty::Ref(_, _, ty::Mutability::Mut)
            );
            // Of a struct not all plain, a `Cell` in it, each field that is,
            // `q.title`, which nothing changes either.
            let fields: Vec<String> = match mir_body.local_decls[local].ty.peel_refs().kind() {
                ty::Adt(adt, _) if !plain && !through_mut && adt.is_struct() => {
                    match self.shape(mir_body.local_decls[local].ty.peel_refs()) {
                        Shape::Object(fields) => (fields.into_iter())
                            .filter(|(_, ty)| is_plain(*ty))
                            .map(|(field, _)| format!("{name}.{field}"))
                            .collect(),
                        _ => Vec::new(),
                    }
                }
                _ => Vec::new(),
            };
            if plain {
                self.locals.mir.plain.insert(name.clone());
            }
            self.locals.mir.plain.extend(fields);
            // Of an array, of what may be JS's, its items, which only a
            // `&mut` changes: `...items` (`read_past_calls`).
            let ty = mir_body.local_decls[local].ty.peel_refs();
            if !plain && !through_mut && (ty.is_slice() || ty.is_array() || self.is_vec_like(ty)) {
                self.locals.mir.plain.insert(format!("...{name}"));
            }
        }
        let mut out = Vec::new();
        // Each flag that says whether a value is there to drop, and the
        // values: a drop may be where their `const` isn't seen.
        let mut flagged = vec![false; mir_body.local_decls.len()];
        let mut flags = IndexVec::new();
        if let Some(drops) = state.facts.drops() {
            flags = IndexVec::from_elem(None, &drops.move_data.move_paths);
            for (path, &is) in drops.flagged.iter_enumerated() {
                if !is {
                    continue;
                }
                let place = drops.move_data.move_paths[path].place;
                flagged[place.local.as_usize()] = true;
                let flag = self.flag_name(&state, place);
                out.push(StmtKind::Let(flag.clone(), Some(Expr::bool(drops.initial[path]))).at(js::Span::NONE));
                flags[path] = Some(flag);
            }
        }
        state.naming = local_facts::NamingFacts::new(boxes, borrowed_names, flags);
        if let Some(name) = state.made_writing() {
            out.push(StmtKind::Let(name.clone(), Some(Expr::str(""))).at(js::Span::NONE));
        }
        // Locals assigned more than once are `let`s, declared first.
        for local in mir_body.local_decls.indices() {
            if mir_body.local_kind(local) != LocalKind::Arg
                && !state.locals.declared[local]
                && !state.locals.names[local].is_empty()
                && (queries::written_again(&state, local) || flagged[local.as_usize()])
            {
                self.check_variable(&state, local, def_span)?;
                let name = state.locals.names[local].clone();
                // One a pointer is taken to, its cell (ADR 0407).
                out.push(
                    match state.facts.pointed().locals.contains(&local) {
                        true => StmtKind::Const(
                            name,
                            Expr::object(vec![js::Prop::Field(js::HELD.into(), Expr::undefined())]),
                        ),
                        false => StmtKind::Let(name, None),
                    }
                    .at(js::Span::NONE),
                );
                state.locals.declared[local] = true;
            }
        }
        self.gather_rest(mir_body, &mut params);
        let start = BasicBlock::from_usize(0);
        let given = self.writer_options(&state, options.as_deref());
        let lowered = self.with_given(given, |cx| cx.mir_tree(&mut state, start))?;
        out.extend(state.settled_lets().chain(lowered));
        if let Some(options) = options {
            params.push(js::Pattern::Name(options));
        }
        // Each bounds check moved to where its element is read, made there.
        if !state.deferred.is_empty() {
            return Err(self.unsupported(mir_body.span, "a bounds check this read doesn't make, from its MIR"));
        }
        // A closure's own, which it named, apart from its body's.
        let falsy = std::mem::replace(&mut self.locals.mir.falsy, outer_falsy);
        let tested = std::mem::replace(&mut self.locals.mir.tested, outer_tested);
        // Where the program writes what tidy reads the body's JS by.
        let written = self.written();
        // The closures a `let` names that a hook's function alone is, by
        // their names (ADR 0297).
        let named: Vec<rustc_span::Span> = (self.frame.thir.stmts.iter())
            .filter_map(|stmt| match &stmt.kind {
                thir::StmtKind::Let { pattern, .. }
                    if let thir::PatKind::Binding { var, .. } = pattern.kind
                        && self.frame.facts.named_callbacks.contains(&var) =>
                {
                    Some(pattern.span)
                }
                _ => None,
            })
            .collect();
        let callbacks: std::collections::HashSet<String> = (mir_body.var_debug_info.iter())
            .filter_map(|info| match info.value {
                mir::VarDebugInfoContents::Place(place)
                    if place.projection.is_empty() && named.contains(&info.source_info.span) =>
                {
                    Some(state.locals.names[place.local].clone())
                }
                _ => None,
            })
            .collect();
        let captured = std::mem::replace(&mut self.locals.mir.captured, outer_captured);
        // The names of `bool`s, and of integers.
        let (mut booleans, numbers) = self.typed_names(mir_body, &state);
        // And each field path that's a `bool` each time it's read.
        booleans.extend(
            (self.locals.mir.paths.iter())
                .filter(|&(_, &b)| b)
                .map(|(path, _)| path.clone()),
        );
        // A class's method reads its `self` as `this` (ADR 0761).
        let def_id = mir_body.source.def_id();
        crate::lower::classes::receiver_as_this(self.tcx, def_id, &mut params, &mut out, &mut booleans)
            .map_err(|what| self.unsupported(mir_body.span, what))?;
        let params = std::cell::RefCell::new(params);
        tidy::tidy(
            &mut out,
            &tidy::Hints {
                result: &state.locals.names[mir::RETURN_PLACE],
                falsy: &falsy,
                tested: &tested,
                written: &written,
                callbacks: &callbacks,
                closure: (mir_body.args_iter().next())
                    .is_some_and(|local| matches!(mir_body.local_decls[local].ty.peel_refs().kind(), ty::Closure(..))),
                lets: &self.let_names(mir_body, &state),
                returns: &state.facts.jumps().returns,
                captured: &captured,
                clones: &self.clone_names(mir_body, &state),
                kept: &hints::kept_captures(&state),
                booleans: &booleans,
                numbers: &numbers,
                plain: &self.plain_names(),
                forwarding: &self.locals.mir.forwarding.clone(),
                concise: &self.locals.mir.concise.clone(),
                flags: &state.naming.flag_names(),
                fuse: self.fuse_import().as_deref(),
                until: self.krate.tidy_until,
                params: &params,
                suggested: &self.temporaries_names(mir_body, &state),
                named: &state.locals.named(),
                again: &std::cell::Cell::new(false),
            },
        );
        let params = params.into_inner();
        self.body_epilogue(&mut state, &mut out);
        Ok((params, out))
    }

    /// The names of plain values, which nothing changes (`Locals::plain`),
    /// and the module's imports, whose fields JS's modules never change
    /// either, `HighlightStyle.define`, as `reads_alike` has them.
    fn plain_names(&self) -> HashSet<String> {
        let mut plain = self.locals.mir.plain.clone();
        plain.extend(
            self.krate
                .imports
                .get(&self.module)
                .into_iter()
                .flat_map(|imports| imports.values().cloned()),
        );
        plain
    }
}
