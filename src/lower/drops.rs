//! Destructors (ADR 0098): the scopes, flags and temporaries that drop a
//! value where rustc does, and the JS that drops it. What dropping a type
//! runs is `types`'s, and what a body owns and moves, found before it's
//! lowered, `facts`'.
//!
//! Where a body drops what it owns is MIR's drop elaboration's, which
//! `mir/drops.rs` follows (ADR 0364); this is what each drop runs, the drop
//! functions it's written in, and the drops a std function lowered in place
//! makes of its own, `map_or`'s fallback a `try` whose `finally` drops it.
//! What this can't do yet is an error, found before any JS is written: a
//! value with a destructor that nothing drops would be a wrong answer.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::rc::Rc;

use rustc_hir::def::DefKind;

use rustc_middle::thir::{ExprKind, Pat, StmtKind as ThirStmt};
use rustc_middle::ty::{self, GenericArgsRef, Ty, TyCtxt, TypeVisitableExt};
use rustc_span::Span;
use rustc_span::def_id::DefId;

use super::recognition::{ChannelEnd, StdItem};
use super::representation::variant_field;
use super::traits::item_drop_key;
use super::{FnCx, R, lower_first};
use crate::js::{self, Expr, Op, Stmt, StmtKind};

mod facts;
mod functions;
mod inlined;
mod parts;
mod scopes;
mod types;
mod uses;

pub(super) use self::uses::*;

use facts::{Facts, find_facts};
pub(super) use inlined::inlined_drops;
pub(super) use parts::is_plain_ref_cell;
pub(super) use types::Drops;
use types::{DropQuery, TypeDrops, describe};

/// Whether dropping `ty`, a type the crate knows, runs a destructor, of the
/// `Rc`s `counted` and no dictionaries (ADR 0320): an `Rc` of one is
/// counted, as its last drops it.
pub(super) fn drop_runs<'tcx>(
    recognition: super::recognition::Recognition<'_, 'tcx>,
    (library, closure_facts): (bool, &super::analysis::ClosureFacts),
    counted: &super::analysis::Counted<'tcx>,
    ty: Ty<'tcx>,
) -> bool {
    let state = TypeDrops::default();
    // Its type parameters, of no drop given here, drop nothing: their drops
    // are their callers', passed on as they are.
    without_dictionaries(recognition, (library, closure_facts), counted, &state).drops(ty) == Drops::Runs
}

/// What drops, asked in `recognition`'s typing environment with no
/// dictionaries given: a type parameter's drop is a maybe.
fn without_dictionaries<'a, 'tcx>(
    recognition: super::recognition::Recognition<'a, 'tcx>,
    (library, closure_facts): (bool, &'a super::analysis::ClosureFacts),
    counted: &'a super::analysis::Counted<'tcx>,
    state: &'a TypeDrops<'tcx>,
) -> DropQuery<'a, 'tcx> {
    DropQuery {
        recognition,
        evidence: super::traits::EvidenceQuery::without_evidence(recognition),
        library,
        counted,
        closure_facts,
        state,
    }
}

/// The drop functions one drop makes, which come before its code: a call
/// to one may be in a branch, as an `Option`'s, that another isn't in.
#[derive(Default)]
struct Made<'tcx> {
    functions: Vec<(Ty<'tcx>, String)>,
    defs: Vec<Stmt>,
}

/// A part of a value, field by field: each step the variant it's in, by
/// index, for an enum's, and the field.
pub(super) type Path = Vec<(Option<u32>, usize)>;

/// Each `RefCell` a pattern binds of a field, not as `_` (ADR 0362).
struct BoundRefCells<'a, 'b, 'tcx> {
    recognition: super::recognition::Recognition<'a, 'tcx>,
    thir: &'b rustc_middle::thir::Thir<'tcx>,
    found: &'b mut HashSet<Ty<'tcx>>,
}

impl<'b, 'tcx> rustc_middle::thir::visit::Visitor<'b, 'tcx> for BoundRefCells<'_, 'b, 'tcx> {
    fn thir(&self) -> &'b rustc_middle::thir::Thir<'tcx> {
        self.thir
    }

    fn visit_pat(&mut self, pat: &'b Pat<'tcx>) {
        if let rustc_middle::thir::PatKind::Variant { ref subpatterns, .. }
        | rustc_middle::thir::PatKind::Leaf { ref subpatterns } = pat.kind
        {
            for field in subpatterns {
                if !matches!(field.pattern.kind, rustc_middle::thir::PatKind::Wild)
                    && self.recognition.is_std_type(field.pattern.ty, StdItem::RefCell)
                {
                    self.found.insert(field.pattern.ty);
                }
            }
        }
        rustc_middle::thir::visit::walk_pat(self, pat);
    }
}

/// A copied default's drops: the drop functions of its trait's type
/// parameters, by index, and those it can't make, with why (ADR 0098).
pub(super) type DefaultDrops<'tcx> = (
    HashMap<u32, String>,
    HashMap<u32, (Ty<'tcx>, &'static str)>,
    HashMap<ty::AliasTy<'tcx>, u32>,
    HashSet<ty::AliasTy<'tcx>>,
    ty::GenericArgsRef<'tcx>,
);

/// What a body inside another, a closure's or a copied default's, takes
/// from the drops of the one it's in while it's lowered, given back as it
/// ends: its statement and scopes, and a default's, the item's drops.
pub(super) struct EnclosingDrops<'tcx> {
    /// Whether a copied default's drops were put in place of the function's,
    /// which `outer` keeps.
    swapped: bool,
    /// The enclosing body's drop functions: a nested body, a closure's or
    /// a copied default's, is a JS function of its own, which declares its
    /// own.
    hoisted: Option<Hoisted<'tcx>>,
}

/// A function's drops while a copied default body has its own.
struct SwappedDrops<'tcx> {
    params: HashMap<u32, String>,
    given: HashSet<u32>,
    items: HashMap<ty::AliasTy<'tcx>, u32>,
    slots: HashSet<ty::AliasTy<'tcx>>,
    self_args: Option<(ty::GenericArgsRef<'tcx>, ty::TypingEnv<'tcx>, ty::GenericArgsRef<'tcx>)>,
    used: HashSet<u32>,
    unsupported: HashMap<u32, (Ty<'tcx>, &'static str)>,
    cache: HashMap<Ty<'tcx>, Drops<'tcx>>,
    sizes: HashMap<Ty<'tcx>, (usize, bool)>,
}

/// A drop `lend_drop_param` lent: under which index, of which associated
/// type, if it's one's, and what it hid.
pub(super) struct LentDrop<'tcx> {
    index: u32,
    alias: Option<ty::AliasTy<'tcx>>,
    before: Option<String>,
}

/// A function's drops, as its bodies are lowered.
#[derive(Default)]
pub(super) struct DropState<'tcx> {
    /// What a type drops depends on, beside its types (ADR 0098).
    types: TypeDrops<'tcx>,
    sizes: RefCell<HashMap<Ty<'tcx>, (usize, bool)>>,
    /// Each body's facts, by the address of its THIR: a closure's is its own.
    facts: HashMap<usize, Rc<Facts>>,
    /// The function being lowered's drop functions, by the index of the
    /// type parameter each drops.
    param_drops: HashMap<u32, String>,
    /// The type parameters whose drops the body has used.
    used_drops: HashSet<u32>,
    /// Each closure made here that holds a value with a destructor: the body
    /// it's made in, and the variables it holds, which its drop drops.
    closures: HashMap<DefId, (DefId, Vec<(Expr, Ty<'tcx>)>)>,
    /// The drop functions of the function or closure being lowered, each
    /// declared once at its top; `None` where each drop declares its own.
    hoisted: Option<Hoisted<'tcx>>,
    /// The drops of each function a copied default is lowered in, innermost
    /// last, in its impl's terms (ADR 0495).
    outer: Vec<SwappedDrops<'tcx>>,
}

/// A body's drop functions, each of a type inside itself or with a long
/// drop: its name, and its `const`.
#[derive(Default)]
pub(super) struct Hoisted<'tcx> {
    functions: Vec<(Ty<'tcx>, String)>,
    defs: Vec<Stmt>,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What a type drops, asked of types and this function's facts alone.
    pub(super) fn drop_query(&self) -> DropQuery<'_, 'tcx> {
        DropQuery {
            recognition: self.recognition(),
            evidence: self.evidence_query(),
            library: self.krate.library,
            counted: self.krate.counted,
            closure_facts: self.krate.closure_facts,
            state: &self.drop_state.types,
        }
    }

    /// What dropping a `ty` runs.
    /// Of a copied default's type, the impl's (ADR 0176), asked where the
    /// impl is, as it may name the impl's own parameters, `SIZE` of a
    /// `WriteBuffer<SIZE>`, which the trait's method doesn't have.
    pub(in crate::lower) fn drops(&self, ty: Ty<'tcx>) -> Drops<'tcx> {
        self.impl_drop_query().drops(self.impl_type(ty))
    }

    pub(in crate::lower) fn has_drops(&self, ty: Ty<'tcx>) -> bool {
        self.impl_drop_query().has_drops(self.impl_type(ty))
    }

    /// `drop_query`, in a copied default asked where its types in the
    /// impl's terms are known (`impl_typing_env`).
    fn impl_drop_query(&self) -> DropQuery<'_, 'tcx> {
        let mut query = self.drop_query();
        if let Some(typing_env) = self.impl_typing_env() {
            query.recognition.typing_env = typing_env;
        }
        query
    }

    /// A call of `callee` of `args`: the type parameters it takes no
    /// destructor of, checked, a library's now, from its manifest, the
    /// crate's once each is lowered (ADR 0190).
    pub(in crate::lower) fn check_drops_given(&self, callee: DefId, args: GenericArgsRef<'tcx>, span: Span) -> R<()> {
        let owner = self.tcx.typeck_root_def_id(self.frame.owner);
        let given: Vec<(u32, Option<Vec<u32>>)> = args
            .iter()
            .enumerate()
            .filter_map(|(index, arg)| {
                let ty = arg.as_type()?;
                let params = owned_params(self.tcx, ty);
                let own = self.drop_query().drops_but_params(ty) != Drops::Nothing;
                (own || !params.is_empty()).then(|| (index as u32, (!own).then_some(params)))
            })
            .collect();
        if given.is_empty() {
            return Ok(());
        }
        if let Some(item) = self.krate.foreign.item(callee) {
            for (index, params) in given.iter().filter(|(index, _)| item.no_drops.contains(index)) {
                let Some(params) = params else {
                    let (path, name) = (self.tcx.def_path_str(callee), param_name(self.tcx, callee, *index));
                    return Err(self.unsupported(
                        span,
                        &format!("giving `{path}`'s `{name}` a type with a destructor, where it takes none"),
                    ));
                };
                self.drops_observed
                    .borrow_mut()
                    .no_drops
                    .entry(owner)
                    .or_default()
                    .extend(params);
            }
            return Ok(());
        }
        if callee.is_local() {
            self.drops_observed.borrow_mut().checks.push(DropCheck {
                caller: owner,
                callee,
                span,
                given,
            });
        }
        Ok(())
    }

    /// A value of `ty` that drops only what its type parameters do, taken as
    /// one that drops nothing, which the function's callers then give none of
    /// (ADR 0190): a generic iterator chrono steps, loops over or folds.
    /// `false` of one that drops something of its own, or in a copied
    /// default, whose parameters are its trait's.
    pub(in crate::lower) fn require_no_drops(&self, ty: Ty<'tcx>) -> bool {
        if self.in_copied_default() || self.drop_query().drops_but_params(ty) != Drops::Nothing {
            return false;
        }
        let owner = self.tcx.typeck_root_def_id(self.frame.owner);
        let params = owned_params(self.tcx, ty);
        self.drops_observed
            .borrow_mut()
            .no_drops
            .entry(owner)
            .or_default()
            .extend(params);
        true
    }

    /// A generic iterator taken by value, `items.fold(..)`, which drops it as
    /// it's done, which JS doesn't: one its callers give none with a
    /// destructor (ADR 0190). Not one what it makes, `output`, holds, and is
    /// closed to drop, `take(n)` of a `repeat`, whose drop it is (ADR 0422).
    pub(in crate::lower) fn require_consumed_drops_none(&self, receiver: Ty<'tcx>, output: Ty<'tcx>) {
        if self.recognition().closed_on_drop(output).is_none() {
            self.require_no_drops(receiver);
        }
    }

    /// Is `drop`, a type's `Drop::drop`, one rust-js runs (ADR 0100)?
    pub(in crate::lower) fn runs_drop(&self, drop: DefId) -> bool {
        self.drop_query().runs_drop(drop)
    }

    /// A closure, at `closure`, made here: the places its drop drops.
    pub(super) fn closure_made(&mut self, closure: DefId, places: Vec<(Expr, Ty<'tcx>)>) {
        self.drop_state.closures.insert(closure, (self.frame.owner, places));
    }

    /// Drop what the closure `closure` took: the variables it took, where it
    /// was made. Only there, or in a closure made inside it, can JS see them.
    fn drop_taken(&mut self, closure: DefId, span: Span, made: &mut Made<'tcx>, out: &mut Vec<Stmt>) -> R<()> {
        let made_here = self.drop_state.closures.get(&closure).cloned().filter(|(made_in, _)| {
            std::iter::successors(Some(self.frame.owner), |&d| self.tcx.opt_parent(d)).any(|d| d == *made_in)
        });
        let Some((_, places)) = made_here else {
            return Err(self.unsupported(span, "dropping a closure that holds a value with a destructor here"));
        };
        for (place, ty) in places {
            self.drop_in(place, ty, span, made, out)?;
        }
        Ok(())
    }

    /// The drop a closure that carries its drop carries, `$dropping`'s:
    /// what it took dropped, where it's made (ADR 0484).
    pub(super) fn carried_drop(&mut self, closure: DefId, span: Span) -> R<Vec<Stmt>> {
        let mut made = Made::default();
        let mut code = Vec::new();
        self.drop_taken(closure, span, &mut made, &mut code)?;
        Ok(made.defs.into_iter().chain(code).collect())
    }

    /// What the closure `closure` holds, `closure_made`'s, where it's made.
    pub(super) fn closure_held(&self, closure: DefId) -> Vec<(Expr, Ty<'tcx>)> {
        (self.drop_state.closures.get(&closure)).map_or_else(Vec::new, |(_, held)| held.clone())
    }

    /// Drop `value`, a `ty`: its own `drop`, then each part's, in Rust's
    /// order. `value` is read more than once, so it must read the same.
    pub(super) fn drop_value(&mut self, value: Expr, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> R<()> {
        let mut made = Made::default();
        let mut code = Vec::new();
        self.drop_in(value, ty, span, &mut made, &mut code)?;
        out.extend(made.defs);
        out.extend(code);
        Ok(())
    }

    /// How many drops writing a `ty`'s in place takes, and whether it's
    /// inside itself, found once for each type.
    fn drop_size(&self, ty: Ty<'tcx>, stack: &mut Vec<Ty<'tcx>>) -> (usize, bool) {
        if !self.has_drops(ty) {
            return (0, false);
        }
        // One inside a type that's inside it: both are inside themselves.
        if stack.contains(&ty) {
            return (0, true);
        }
        if let Some(&known) = self.drop_state.sizes.borrow().get(&ty) {
            return known;
        }
        stack.push(ty);
        let sum = |cx: &Self, tys: &mut dyn Iterator<Item = Ty<'tcx>>, stack: &mut Vec<Ty<'tcx>>| {
            tys.map(|t| cx.drop_size(t, stack))
                .fold((0, false), |(n, r), (m, q)| (n + m, r || q))
        };
        let (size, recursive) = match ty.kind() {
            ty::Adt(_, args) if ty.is_box() || self.recognition().pinned(ty).is_some() => {
                self.drop_size(args.type_at(0), stack)
            }
            ty::Adt(_, args) if self.is_vec_like(ty) => self.drop_size(args.type_at(0), stack),
            ty::Array(item, _) | ty::Slice(item) => self.drop_size(*item, stack),
            // A `RefCell`'s value, and what a counted `Rc` points at, after its
            // count (ADR 0320); a counted `Weak` drops only a count, an
            // uncounted one nothing (ADR 0843).
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::RefCell) => self.drop_size(args.type_at(0), stack),
            ty::Adt(..) if let Some(pointee) = self.counted_rc(ty) => {
                let (size, recursive) = self.drop_size(pointee, stack);
                (size + 1, recursive)
            }
            ty::Adt(..) if self.counted_weak(ty).is_some() => (1, false),
            ty::Adt(..) if self.uncounted_weak(ty).is_some() => (0, false),
            ty::Tuple(items) => sum(self, &mut items.iter(), stack),
            ty::Adt(adt, args) => {
                let own = usize::from(
                    self.tcx
                        .adt_destructor(adt.did())
                        .is_some_and(|d| self.runs_drop(d.did)),
                );
                let (parts, recursive) = sum(self, &mut adt.all_fields().map(|f| self.field_ty(f, args)), stack);
                (own + parts, recursive)
            }
            _ => (1, false),
        };
        stack.pop();
        self.drop_state.sizes.borrow_mut().insert(ty, (size, recursive));
        (size, recursive)
    }

    /// A variable's of `ty`, refused where what it holds drops as JS can't
    /// drop it (ADR 0321).
    pub(in crate::lower) fn check_held_drops(&self, ty: Ty<'tcx>, span: Span) -> R<()> {
        match self.drops(self.reveal(ty)) {
            Drops::Unsupported(t, what) => Err(self.unsupported(span, &describe(t, what))),
            Drops::Nothing | Drops::Runs => Ok(()),
        }
    }

    fn drop_in(&mut self, value: Expr, ty: Ty<'tcx>, span: Span, made: &mut Made<'tcx>, out: &mut Vec<Stmt>) -> R<()> {
        let ty = self.reveal(ty);
        match self.drops(ty) {
            Drops::Nothing => return Ok(()),
            Drops::Unsupported(t, what) => return Err(self.unsupported(span, &describe(t, what))),
            Drops::Runs => {}
        }
        // A type inside itself, as a list in a `Box` of itself, would need a
        // function of its own to drop.
        // One whose drop is a function already, in this drop: that.
        if let Some((_, name)) = made.functions.iter().find(|(t, _)| *t == ty) {
            let js_span = self.js_span(span);
            out.push(StmtKind::Expr(Expr::call(Expr::var(name), vec![value])).at(js_span));
            return Ok(());
        }
        // One declared at the body's top already.
        if let Some(name) = self
            .drop_state
            .hoisted
            .as_ref()
            .and_then(|hoisted| hoisted.functions.iter().find(|(t, _)| *t == ty))
            .map(|(_, name)| name.clone())
        {
            let js_span = self.js_span(span);
            out.push(StmtKind::Expr(Expr::call(Expr::var(&name), vec![value])).at(js_span));
            return Ok(());
        }
        // A type of the crate's own whose drop is long, or inside itself, as
        // a list is, gets a function of its own, which calls itself for the
        // ones inside: its drop is written once, not once for each path to it.
        if let ty::Adt(adt, _) = ty.kind()
            && (adt.did().is_local() || self.krate.foreign.in_library(adt.did()))
            && let (size, recursive) = self.drop_size(ty, &mut Vec::new())
            && (recursive || size > 8)
        {
            let type_name = self.tcx.item_name(adt.did()).to_string();
            let name = self.fresh(&format!("drop{type_name}"));
            let param = self.fresh(&lower_first(&type_name));
            let js_span = self.js_span(span);
            // Declared once, at the top of the body, where it hoists them.
            match self.drop_state.hoisted.as_mut() {
                Some(hoisted) => hoisted.functions.push((ty, name.clone())),
                None => made.functions.push((ty, name.clone())),
            }
            let mut body = Vec::new();
            self.drop_parts(Expr::var(&param), ty, span, made, &mut body)?;
            let def = StmtKind::Const(name.clone(), Expr::arrow(vec![param.into()], body)).at(js_span);
            match self.drop_state.hoisted.as_mut() {
                Some(hoisted) => hoisted.defs.push(def),
                None => made.defs.push(def),
            }
            out.push(StmtKind::Expr(Expr::call(Expr::var(&name), vec![value])).at(js_span));
            return Ok(());
        }
        self.drop_parts(value, ty, span, made, out)
    }
}

/// A call of one of the crate's functions, given what each of its type
/// parameters is given: `None` of a value with a destructor of its own,
/// else the caller's type parameters it holds (ADR 0190).
pub(super) struct DropCheck {
    caller: DefId,
    callee: DefId,
    span: Span,
    given: Vec<(u32, Option<Vec<u32>>)>,
}

/// Each call of the crate's functions given a value with a destructor where
/// the function takes none: an error at the call. One given a caller's type
/// parameter makes the caller take none of that either (ADR 0190). Whether
/// any was an error.
pub(super) fn check_no_drops(
    tcx: TyCtxt<'_>,
    no_drops: &mut HashMap<DefId, BTreeSet<u32>>,
    checks: &[DropCheck],
) -> bool {
    let mut reported = HashSet::new();
    let mut changed = true;
    while changed {
        changed = false;
        for check in checks {
            let required = no_drops.get(&check.callee).cloned().unwrap_or_default();
            for (index, params) in check.given.iter().filter(|(index, _)| required.contains(index)) {
                match params {
                    None if reported.insert(check.span) => {
                        let (path, name) = (tcx.def_path_str(check.callee), param_name(tcx, check.callee, *index));
                        let what = format!("giving `{path}`'s `{name}` a type with a destructor, where it takes none");
                        super::refusal::refuse(tcx, check.span, super::refusal::Refused::Feature(&what));
                    }
                    None => {}
                    Some(params) => {
                        let caller = no_drops.entry(check.caller).or_default();
                        for &param in params {
                            changed |= caller.insert(param);
                        }
                    }
                }
            }
        }
    }
    !reported.is_empty()
}
