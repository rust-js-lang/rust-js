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
mod types;

use facts::{Facts, find_facts};
pub(super) use types::Drops;
use types::{DropQuery, TypeDrops, describe};

/// Whether dropping `ty`, a type the crate knows, runs a destructor, of the
/// `Rc`s `counted` and no dictionaries (ADR 0320): an `Rc` of one is
/// counted, as its last drops it.
pub(super) fn drop_runs<'tcx>(
    recognition: super::recognition::Recognition<'_, 'tcx>,
    library: bool,
    counted: &super::analysis::Counted<'tcx>,
    ty: Ty<'tcx>,
) -> bool {
    let state = TypeDrops::default();
    // Its type parameters, of no drop given here, drop nothing: their drops
    // are their callers', passed on as they are.
    without_dictionaries(recognition, library, counted, &state).drops(ty) == Drops::Runs
}

/// What drops, asked in `recognition`'s typing environment with no
/// dictionaries given: a type parameter's drop is a maybe.
fn without_dictionaries<'a, 'tcx>(
    recognition: super::recognition::Recognition<'a, 'tcx>,
    library: bool,
    counted: &'a super::analysis::Counted<'tcx>,
    state: &'a TypeDrops<'tcx>,
) -> DropQuery<'a, 'tcx> {
    DropQuery {
        recognition,
        evidence: super::traits::EvidenceQuery::without_evidence(recognition),
        library,
        counted,
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
    /// The enclosing body's drop functions, where this body declares its own.
    hoisted: Option<Option<Hoisted<'tcx>>>,
}

/// A function's drops while a copied default body has its own.
struct SwappedDrops<'tcx> {
    params: HashMap<u32, String>,
    given: HashSet<u32>,
    items: HashMap<ty::AliasTy<'tcx>, u32>,
    slots: HashSet<ty::AliasTy<'tcx>>,
    self_args: Option<(ty::GenericArgsRef<'tcx>, ty::TypingEnv<'tcx>)>,
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
        let owner = self.tcx.typeck_root_def_id(self.body_owner);
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
                self.krate
                    .no_drops
                    .borrow_mut()
                    .entry(owner)
                    .or_default()
                    .extend(params);
            }
            return Ok(());
        }
        if callee.is_local() {
            self.krate.drop_checks.borrow_mut().push(DropCheck {
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
        let owner = self.tcx.typeck_root_def_id(self.body_owner);
        let params = owned_params(self.tcx, ty);
        self.krate
            .no_drops
            .borrow_mut()
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
        self.drop_state.closures.insert(closure, (self.body_owner, places));
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
            // count (ADR 0320); a `Weak` drops only a count.
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::RefCell) => self.drop_size(args.type_at(0), stack),
            ty::Adt(..) if let Some(pointee) = self.counted_rc(ty) => {
                let (size, recursive) = self.drop_size(pointee, stack);
                (size + 1, recursive)
            }
            ty::Adt(..) if self.weak_of(ty).is_some() => (1, false),
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

    /// A `ty`'s drop, written in place: its own `drop`, then its parts'.
    fn drop_parts(
        &mut self,
        value: Expr,
        ty: Ty<'tcx>,
        span: Span,
        made: &mut Made<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let js_span = self.js_span(span);
        match ty.kind() {
            // A `Pin` is its pointer (ADR 0329).
            ty::Adt(_, args) if ty.is_box() || self.recognition().pinned(ty).is_some() => {
                self.drop_in(value, args.type_at(0), span, made, out)?
            }
            ty::Adt(_, args) if self.is_vec_like(ty) => self.drop_items(value, args.type_at(0), span, made, out)?,
            // An iterator owning an array's items: each it hasn't given yet.
            ty::Adt(..) if let Some(item) = self.recognition().owned_items(ty) => {
                self.drop_items(value, item, span, made, out)?
            }
            // Closed, `it.return()`, which drops what it holds (ADR 0422), and
            // an adapter that holds a function with a destructor, made closing
            // to drop it, `map(f)`'s (ADR 0485).
            ty::Adt(..)
                if self.recognition().closed_on_drop(ty).is_some()
                    || (self.recognition().held_function(ty))
                        .is_some_and(|function| self.drops(function) != Drops::Nothing) =>
            {
                out.push(StmtKind::Expr(Expr::call(Expr::member(value, "return"), Vec::new())).at(js_span));
            }
            // A channel's end (ADR 0142): a receiver's drops what's still in
            // its queue, as Rust's does.
            ty::Adt(_, args) if let Some(end) = self.recognition().channel_end(ty) => {
                let call = match end {
                    ChannelEnd::Sender => Expr::call_helper("$dropSender", vec![value]),
                    ChannelEnd::Receiver => {
                        let mut values = vec![value];
                        values.extend(self.drop_function(args.type_at(0), span)?);
                        Expr::call_helper("$dropReceiver", values)
                    }
                };
                out.push(StmtKind::Expr(call).at(js_span));
            }
            // A map's entries, each its key, then its value, as Rust drops a
            // pair, and a set's items, a B-tree's in its keys' order (ADRs
            // 0321, 0467).
            ty::Adt(_, args) if self.is_map(ty) => {
                let set = self.is_set(ty);
                let key_ty = args.type_at(0);
                let value_ty = (!set).then(|| args.type_at(1));
                let drops = |this: &Self, ty: Option<Ty<'tcx>>| ty.is_some_and(|ty| this.drops(ty) != Drops::Nothing);
                let (keys, values) = (drops(self, Some(key_ty)), drops(self, value_ty));
                if !self.is_sorted(ty) && !(keys && values) {
                    let (items, item) = match (set, keys) {
                        (true, _) => (value, key_ty),
                        (false, true) => (Expr::call(Expr::member(value, "keys"), Vec::new()), key_ty),
                        (false, false) => (
                            Expr::call(Expr::member(value, "values"), Vec::new()),
                            value_ty.expect("a map's values"),
                        ),
                    };
                    self.drop_items(items, item, span, made, out)?;
                } else {
                    let entries = match self.is_sorted(ty) {
                        true => self.in_order_of(value, ty, span)?,
                        false => value,
                    };
                    let (key, item) = (self.fresh("key"), self.fresh("value"));
                    let mut body = Vec::new();
                    let pattern = match value_ty {
                        None => {
                            self.drop_in(Expr::var(&key), key_ty, span, made, &mut body)?;
                            js::Pattern::Name(key)
                        }
                        Some(value_ty) => {
                            self.drop_in(Expr::var(&key), key_ty, span, made, &mut body)?;
                            self.drop_in(Expr::var(&item), value_ty, span, made, &mut body)?;
                            js::Pattern::names_array([keys.then_some(key), values.then_some(item)])
                        }
                    };
                    out.push(
                        StmtKind::ForOf {
                            label: None,
                            pattern,
                            mutable: false,
                            iterable: entries,
                            body,
                        }
                        .at(js_span),
                    );
                }
            }
            // A `RefCell`'s value (ADR 0320), and a `Mutex`'s or an
            // `RwLock`'s, each `{ value }` as it is (ADR 0144), and a `Cell`'s
            // (ADR 0519); of a `RefCell<dyn T>`, the cell is the pair, `{
            // value, impl }` (ADR 0397).
            ty::Adt(_, args)
                if [StdItem::RefCell, StdItem::Mutex, StdItem::RwLock, StdItem::Cell]
                    .into_iter()
                    .any(|cell| self.is_std_type(ty, cell)) =>
            {
                let inner = args.type_at(0);
                let value = match self.cell_dyn_trait(inner) {
                    Some(_) => value,
                    None => Expr::held(value),
                };
                self.drop_in(value, inner, span, made, out)?
            }
            // A `OnceCell`'s `Option`, `{ value }`, `undefined` where it's
            // not set (ADR 0519).
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::OnceCell) => {
                let held = Ty::new_option(self.tcx, args.type_at(0));
                self.drop_in(Expr::held(value), held, span, made, out)?
            }
            // A counted `Rc`: `$rcDrop(rc, drop)`, `drop` what the last drops of
            // what it points at, if that has a destructor, of a `dyn` its
            // pair, which it is (ADR 0520); a `Weak`: `$weakDrop` (ADR 0320).
            ty::Adt(..) if let Some(pointee) = self.counted_rc(ty) => {
                let mut args = vec![value];
                if self.drops(pointee) == Drops::Runs {
                    let name = self.fresh("value");
                    let mut body = Vec::new();
                    self.drop_in(Expr::var(&name), pointee, span, made, &mut body)?;
                    // `(value) => { dropNode(value); }` is `dropNode`.
                    let drop = match body.as_slice() {
                        [
                            Stmt {
                                kind:
                                    StmtKind::Expr(Expr {
                                        kind: js::ExprKind::Call(callee, given),
                                        ..
                                    }),
                                ..
                            },
                        ] if matches!(callee.kind, js::ExprKind::Var(_))
                            && matches!(given.as_slice(), [Expr { kind: js::ExprKind::Var(n), .. }] if *n == name) =>
                        {
                            (**callee).clone()
                        }
                        _ => Expr::arrow(vec![name.into()], body),
                    };
                    args.push(drop);
                }
                out.push(StmtKind::Expr(Expr::call(Expr::helper("$rcDrop"), args)).at(js_span));
            }
            // A heap's `PeekMut` (ADR 0333).
            ty::Adt(..) if self.recognition().peek_mut_of(ty).is_some() => {
                out.push(StmtKind::Expr(Expr::call(Expr::helper("$peekMutDrop"), vec![value])).at(js_span));
            }
            // A guard: `$unborrow(cell)` (ADR 0328).
            ty::Adt(..) if self.is_guard(ty) => {
                out.push(StmtKind::Expr(Expr::call(Expr::helper("$unborrow"), vec![value])).at(js_span));
            }
            ty::Adt(..) if self.weak_of(ty).is_some() => {
                out.push(StmtKind::Expr(Expr::call(Expr::helper("$weakDrop"), vec![value])).at(js_span));
            }
            // What a closure holds: the variables it took, where it was made.
            // Only there, or in a closure made inside it, can JS see them.
            ty::Closure(def_id, _) => {
                let made_here = self.drop_state.closures.get(def_id).cloned().filter(|(made_in, _)| {
                    std::iter::successors(Some(self.body_owner), |&d| self.tcx.opt_parent(d)).any(|d| d == *made_in)
                });
                let Some((_, places)) = made_here else {
                    // One its maker gave back carries its drop (ADR 0484).
                    if super::recognition::carries_its_drop(self.tcx, *def_id) {
                        let js_span = self.js_span(span);
                        out.push(StmtKind::Expr(Expr::call_helper("$dispose", vec![value])).at(js_span));
                        return Ok(());
                    }
                    return Err(self.unsupported(span, "dropping a closure that holds a value with a destructor here"));
                };
                for (place, ty) in places {
                    self.drop_in(place, ty, span, made, out)?;
                }
            }
            // A `dyn`'s: its dictionary's `$drop`, if what it holds has one.
            ty::Dynamic(..) => {
                let dictionary = Expr::member(value.clone(), js::DICTIONARY);
                out.push(dyn_dropped(dictionary, Expr::held(value), js_span));
            }
            // `dropT?.(value)`: the caller's drop, if its `T` has one; of a
            // std trait's associated type of one, `dropIIntoIter` (ADR 0421).
            _ if let Some(index) = self.given_drop(ty) => {
                self.drop_state.used_drops.insert(index);
                let drop = Expr::var(&self.drop_state.param_drops[&index]);
                let js_span = self.js_span(span);
                out.push(
                    StmtKind::Expr(Expr {
                        reads: false,
                        kind: js::ExprKind::OptionalCall(Box::new(drop), vec![value]),
                        span: js_span,
                    })
                    .at(js_span),
                );
            }
            // `ZZone.$dropOffset?.(value)`: its impl's, if its type has one (ADR 0178).
            ty::Alias(..) if let Some(drop) = self.item_drop(ty) => {
                out.push(
                    StmtKind::Expr(Expr {
                        reads: false,
                        kind: js::ExprKind::OptionalCall(Box::new(drop), vec![value]),
                        span: js_span,
                    })
                    .at(js_span),
                );
            }
            ty::Array(item, _) | ty::Slice(item) => self.drop_items(value, *item, span, made, out)?,
            ty::Tuple(items) => {
                for (i, item) in items.iter().enumerate() {
                    let part = self.project(value.clone(), ty, i);
                    self.drop_in(part, item, span, made, out)?;
                }
            }
            ty::Adt(adt, args) => {
                if let Some(d) = self.tcx.adt_destructor(adt.did())
                    && self.runs_drop(d.did)
                {
                    // `drop(&mut self)`: a `&mut` to what isn't an object is a
                    // box of it (ADR 0072).
                    let this = match self.is_boxable(ty) {
                        true => Expr::object(vec![js::Prop::Field(js::HELD.into(), value.clone())]),
                        false => value.clone(),
                    };
                    let drop = self
                        .tcx
                        .lang_items()
                        .drop_trait()
                        .map(|t| self.tcx.associated_item_def_ids(t)[0])
                        .expect("`Drop` has `drop`");
                    let call = self
                        .trait_call(drop, self.tcx.mk_args(&[ty.into()]), vec![this], span, out)?
                        .ok_or_else(|| self.unsupported(span, &format!("calling `{ty}`'s `drop`")))?;
                    out.push(StmtKind::Expr(call).at(js_span));
                }
                if let Some(inner) = self.option_of(ty) {
                    // A `Some` of what may look like `None`, a generic `T`
                    // say, is boxed (ADR 0051): its value, unboxed.
                    let payload = match self.boxed_payload(inner) {
                        true => self.some_value(value.clone()),
                        false => value.clone(),
                    };
                    let mut some = Vec::new();
                    self.drop_in(payload, inner, span, made, &mut some)?;
                    out.push(StmtKind::If(Expr::bin(Op::LooseNe, value, Expr::null()), some, None).at(js_span));
                } else if adt.is_struct() {
                    for (i, field) in adt.non_enum_variant().fields.iter().enumerate() {
                        let part = self.project(value.clone(), ty, i);
                        let field = self.field_ty(field, args);
                        // A `dyn` last field: through the `impl` its struct
                        // was given (ADRs 0409, 0436).
                        if let ty::Dynamic(..) = field.kind() {
                            let dictionary = Expr::member(value.clone(), js::DICTIONARY);
                            out.push(dyn_dropped(dictionary, part, js_span));
                            continue;
                        }
                        self.drop_in(part, field, span, made, out)?;
                    }
                } else if adt.is_enum() {
                    for variant in adt.variants() {
                        let mut fields = Vec::new();
                        // An untagged enum's variant is its payload (ADR 0214).
                        let untagged = self.untagged(ty).is_some();
                        for (i, field) in variant.fields.iter().enumerate() {
                            let part = match untagged {
                                true => value.clone(),
                                false => Expr::member(value.clone(), variant_field(self.tcx, variant, i)),
                            };
                            self.drop_in(part, self.field_ty(field, args), span, made, &mut fields)?;
                        }
                        if fields.is_empty() {
                            continue;
                        }
                        let test = match adt.variants().len() {
                            1 => Expr::bool(true),
                            _ => self.variant_test(value.clone(), ty, *adt, adt.variant_index_with_id(variant.def_id)),
                        };
                        out.push(StmtKind::If(test, fields, None).at(js_span));
                    }
                } else if !adt.is_union() {
                    // A union's fields are never dropped, which Rust holds
                    // to what has no destructor, or a `ManuallyDrop` (ADR 0406).
                    return Err(self.unsupported(span, &format!("dropping `{ty}`")));
                }
            }
            _ => return Err(self.unsupported(span, &format!("dropping `{ty}`"))),
        }
        Ok(())
    }

    /// Each item of an array, in order: `for (const item of v) { .. }`.
    fn drop_items(
        &mut self,
        items: Expr,
        item: Ty<'tcx>,
        span: Span,
        made: &mut Made<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let name = self.fresh("item");
        let mut body = Vec::new();
        self.drop_in(Expr::var(&name), item, span, made, &mut body)?;
        let js_span = self.js_span(span);
        out.push(
            StmtKind::ForOf {
                label: None,
                pattern: js::Pattern::Name(name),
                mutable: false,
                iterable: items,
                body,
            }
            .at(js_span),
        );
        Ok(())
    }

    /// This body's facts, found the first time they're asked for; what it
    /// does that isn't supported is an error then.
    /// A `RefCell` type a field holds as its value (ADR 0362): each field of
    /// one in the crate is only ever borrowed for a moment, where nothing can
    /// ask, so no borrow of it is counted, its count stays 0, and its checks
    /// can't fail. A cell lent, bound, moved, replaced or cloned, of a derive,
    /// or of a type with a destructor, or a library's, is a cell.
    pub(super) fn plain_ref_cell(&self, ty: Ty<'tcx>) -> bool {
        self.is_std_type(ty, StdItem::RefCell)
            && (self.krate.plain_ref_cells)
                .get_or_init(|| self.find_plain_ref_cells())
                .contains(&ty)
    }

    fn find_plain_ref_cells(&self) -> HashSet<Ty<'tcx>> {
        let tcx = self.tcx;
        // Of THIR's borrows, as MIR's are, whose guards of a field's handle
        // tidy reads as the field (ADR 0364).
        if self.krate.library {
            return HashSet::new();
        }
        let mut plain = HashSet::new();
        let mut cells = HashSet::new();
        let bodies = (self.krate.bodies.values().copied()).chain(self.krate.closures.values().copied());
        for body in bodies {
            // Each body's own facts, asked where it is, of its own type
            // parameters, with no dictionaries: a type parameter's drop is
            // then a maybe, and its borrow counted.
            let owner = tcx.typeck_root_def_id(body.def_id.to_def_id());
            let recognition = super::recognition::Recognition {
                tcx,
                typing_env: ty::TypingEnv::post_analysis(tcx, owner),
                trait_impls: self.krate.trait_impls,
                foreign: self.krate.foreign,
            };
            let state = TypeDrops::default();
            let query = without_dictionaries(recognition, false, self.krate.counted, &state);
            let thir = &body.thir;
            let facts = find_facts(&query, thir, body.def_id.to_def_id());
            // `x.f` lent to a borrow held for a moment: `x.f.borrow_mut().push(..)`.
            let mut for_a_moment = HashSet::new();
            for expr in thir.exprs.iter() {
                if let ExprKind::Call { fun, ref args, .. } = expr.kind
                    && facts.momentary.contains(&fun)
                    && let Some(&lent) = args.first()
                    && let ExprKind::Borrow { arg, .. } = thir[super::body_queries::strip(thir, lent)].kind
                {
                    for_a_moment.insert(super::body_queries::strip(thir, arg));
                }
            }
            for (id, expr) in thir.exprs.iter_enumerated() {
                if let ExprKind::Field { .. } = expr.kind
                    && self.is_std_type(expr.ty, StdItem::RefCell)
                {
                    match for_a_moment.contains(&id) {
                        true => plain.insert(expr.ty),
                        false => cells.insert(expr.ty),
                    };
                }
            }
            // One bound by a pattern is a value of its own, a cell.
            let mut bound = BoundRefCells {
                recognition: self.recognition(),
                thir,
                found: &mut cells,
            };
            for expr in thir.exprs.iter() {
                if let ExprKind::Let { ref pat, .. } = expr.kind {
                    rustc_middle::thir::visit::Visitor::visit_pat(&mut bound, pat);
                }
            }
            for stmt in thir.stmts.iter() {
                if let ThirStmt::Let { ref pattern, .. } = stmt.kind {
                    rustc_middle::thir::visit::Visitor::visit_pat(&mut bound, pattern);
                }
            }
            for arm in thir.arms.iter() {
                rustc_middle::thir::visit::Visitor::visit_pat(&mut bound, &arm.pattern);
            }
            for param in thir.params.iter() {
                if let Some(pat) = &param.pat {
                    rustc_middle::thir::visit::Visitor::visit_pat(&mut bound, pat);
                }
            }
        }
        // A derive's, which reads its fields itself (ADR 0049).
        for imp in tcx.hir_crate_items(()).definitions() {
            if matches!(tcx.def_kind(imp), DefKind::Impl { of_trait: true })
                && super::recognition::known_derive(tcx, imp.to_def_id())
                && let ty::Adt(adt, args) = tcx.type_of(imp).instantiate_identity().skip_normalization().kind()
            {
                for field in adt.all_fields() {
                    let field_ty = field.ty(tcx, args).skip_normalization();
                    if self.is_std_type(field_ty, StdItem::RefCell) {
                        cells.insert(field_ty);
                    }
                }
            }
        }
        // A generic one may be any of them.
        if cells.iter().any(|ty| ty.has_param()) {
            return HashSet::new();
        }
        // Of no type parameter: asked anywhere.
        let recognition = super::recognition::Recognition {
            tcx,
            typing_env: ty::TypingEnv::fully_monomorphized(),
            trait_impls: self.krate.trait_impls,
            foreign: self.krate.foreign,
        };
        let state = TypeDrops::default();
        let query = without_dictionaries(recognition, false, self.krate.counted, &state);
        plain.retain(|ty| !cells.contains(ty) && !ty.has_param() && !query.has_drops(*ty));
        plain
    }

    pub(super) fn drop_facts(&mut self) -> R<Rc<Facts>> {
        let key = std::ptr::from_ref(self.thir) as usize;
        if let Some(facts) = self.drop_state.facts.get(&key) {
            return Ok(facts.clone());
        }
        let facts = Rc::new(find_facts(&self.drop_query(), self.thir, self.body_owner));
        self.drop_state.facts.insert(key, facts.clone());
        let mut failed = None;
        for (span, what, _) in &facts.problems {
            failed = Some(self.unsupported(*span, what));
        }
        match failed {
            Some(guar) => Err(guar),
            None => Ok(facts),
        }
    }

    /// What THIR's facts refuse of the closures of the body being lowered
    /// that hold a value with a destructor: one made where JS can't see what
    /// it took, given away uncalled, or assigned.
    pub(in crate::lower) fn check_closure_drops(&self) -> R<()> {
        let facts = find_facts(&self.drop_query(), self.thir, self.body_owner);
        let mut failed = None;
        for (span, what, _) in facts.problems.iter().filter(|(.., closure)| *closure) {
            failed = Some(self.unsupported(*span, what));
        }
        match failed {
            Some(guar) => Err(guar),
            None => Ok(()),
        }
    }

    /// Start lowering a body inside the one being lowered: its own facts, and
    /// none of the enclosing one's statement and scopes. A copied default's,
    /// `default`, is given its trait's type parameters' drops and those it
    /// can't make (ADR 0098).
    /// `frame`: a closure's or a copied default's, a JS function of its own,
    /// which declares its own drop functions.
    pub(super) fn enter_body_drops(
        &mut self,
        default: Option<DefaultDrops<'tcx>>,
        frame: bool,
    ) -> R<EnclosingDrops<'tcx>> {
        let swapped = default.is_some();
        if let Some(default) = default {
            let outer = self.swap_drops(default);
            self.drop_state.outer.push(outer);
        }
        self.drop_facts()?;
        Ok(EnclosingDrops {
            swapped,
            hoisted: frame.then(|| self.begin_hoisting()),
        })
    }

    /// Declare drop functions once, at the top of the body that's starting:
    /// what the enclosing one had, to give back to `end_hoisting`.
    pub(super) fn begin_hoisting(&mut self) -> Option<Hoisted<'tcx>> {
        self.drop_state.hoisted.replace(Hoisted::default())
    }

    /// The drop functions the body that's ending declared, for its top, and
    /// the enclosing body's back.
    pub(super) fn end_hoisting(&mut self, enclosing: Option<Hoisted<'tcx>>) -> Vec<Stmt> {
        std::mem::replace(&mut self.drop_state.hoisted, enclosing).map_or_else(Vec::new, |hoisted| hoisted.defs)
    }

    /// Finish the body `enter_body_drops` started: every owner it has had a
    /// scope, and every move its flag cleared, and the enclosing body's back.
    pub(super) fn leave_body_drops(&mut self, enclosing: EnclosingDrops<'tcx>) -> Vec<Stmt> {
        if enclosing.swapped {
            let outer = self.drop_state.outer.pop().expect("the function's drops");
            self.restore_drops(outer);
        }
        match enclosing.hoisted {
            Some(outer) => self.end_hoisting(outer),
            None => Vec::new(),
        }
    }

    /// The std associated types of the function being lowered's type
    /// parameters a caller would give a drop for, `drop_items` (ADR 0489).
    pub(super) fn give_drop_slots(&mut self, slots: impl IntoIterator<Item = Ty<'tcx>>) {
        self.drop_state.types.slots = (slots.into_iter())
            .filter_map(|ty| match *ty.kind() {
                ty::Alias(_, alias) => Some(alias),
                _ => None,
            })
            .collect();
    }

    /// The function being lowered is given a drop `index`, named `name`, of
    /// `ty`: its type parameter's (ADR 0098), or a std trait's associated
    /// type of one (ADR 0421).
    pub(super) fn give_drop_param(&mut self, index: u32, name: String, ty: Ty<'tcx>) {
        match ty.kind() {
            ty::Alias(_, alias) => {
                self.drop_state.types.items.insert(*alias, index);
            }
            _ => {
                self.drop_state.types.given.insert(index);
            }
        }
        self.drop_state.param_drops.insert(index, name);
    }

    /// Each drop this function is given of a std associated type of a type
    /// parameter, `dropTItem` of `T::Item`, by its type, which a copied
    /// default of its impl is given too (ADR 0421).
    pub(super) fn given_item_drops(&self) -> Vec<(ty::AliasTy<'tcx>, String)> {
        let mut given: Vec<(ty::AliasTy<'tcx>, u32)> = (self.drop_state.types.items.iter())
            .filter(|(_, index)| self.drop_state.param_drops.contains_key(index))
            .map(|(&alias, &index)| (alias, index))
            .collect();
        given.sort_by_key(|&(_, index)| index);
        (given.into_iter())
            .map(|(alias, index)| (alias, self.drop_state.param_drops[&index].clone()))
            .collect()
    }

    /// Whether a drop of `ty`, which holds nothing with a destructor but
    /// type parameters' values, is written with drops this function is
    /// given: each type parameter it holds is given one, a closure's of what
    /// it took (ADR 0485).
    pub(super) fn holds_given_drops(&self, ty: Ty<'tcx>) -> bool {
        let held = match ty.kind() {
            ty::Closure(_, args) => args.as_closure().tupled_upvars_ty(),
            _ => ty,
        };
        owned_params(self.tcx, held)
            .into_iter()
            .all(|index| self.drop_state.param_drops.contains_key(&index))
    }

    /// The index of the drop this function is given for a `ty`, a type
    /// parameter or an associated type of one, if it's given one.
    pub(super) fn given_drop(&self, ty: Ty<'tcx>) -> Option<u32> {
        let index = match ty.kind() {
            ty::Param(param) => param.index,
            ty::Alias(_, alias) => self.drop_query().slot_of(*alias)?,
            _ => return None,
        };
        self.drop_state.param_drops.contains_key(&index).then_some(index)
    }

    /// A drop given for a while, a dictionary entry's for its method's own
    /// type parameter (ADR 0163), `index`, of `ty`: what was given before,
    /// to put back. An associated type's, found by its type (ADR 0421), is
    /// lent under an index no type parameter has, so it hides none's.
    pub(super) fn lend_drop_param(&mut self, index: u32, name: String, ty: Ty<'tcx>) -> LentDrop<'tcx> {
        let alias = match *ty.kind() {
            ty::Alias(_, alias) => Some(alias),
            _ => None,
        };
        let index = match alias {
            Some(alias) => {
                let index = u32::MAX - self.drop_state.types.items.len() as u32;
                self.drop_state.types.items.insert(alias, index);
                index
            }
            None => {
                self.drop_state.types.given.insert(index);
                index
            }
        };
        let before = self.drop_state.param_drops.insert(index, name);
        LentDrop { index, alias, before }
    }

    /// Put back what `lend_drop_param` replaced.
    pub(super) fn return_drop_param(&mut self, lent: LentDrop<'tcx>) {
        if let Some(alias) = lent.alias {
            self.drop_state.types.items.remove(&alias);
        } else if lent.before.is_none() {
            self.drop_state.types.given.remove(&lent.index);
        }
        match lent.before {
            Some(name) => self.drop_state.param_drops.insert(lent.index, name),
            None => self.drop_state.param_drops.remove(&lent.index),
        };
    }

    /// A copied default body's drops, in place of this function's, for its
    /// trait's type parameters, `Self` among them (ADR 0049): each is what the
    /// impl's argument for it drops, with the impl's own drops. What was
    /// there, and what was found with it, is given back by `restore_drops`.
    fn swap_drops(&mut self, (drops, unsupported, items, slots, self_args): DefaultDrops<'tcx>) -> SwappedDrops<'tcx> {
        let cache = std::mem::take(&mut *self.drop_state.types.cache.borrow_mut());
        let sizes = std::mem::take(&mut *self.drop_state.sizes.borrow_mut());
        let given = (drops.keys().copied())
            .filter(|index| !items.values().any(|at| at == index))
            .collect();
        SwappedDrops {
            params: std::mem::replace(&mut self.drop_state.param_drops, drops),
            given: std::mem::replace(&mut self.drop_state.types.given, given),
            items: std::mem::replace(&mut self.drop_state.types.items, items),
            slots: std::mem::replace(&mut self.drop_state.types.slots, slots),
            self_args: (self.drop_state.types.self_args)
                .replace((self_args, self.impl_typing_env().unwrap_or(self.typing_env))),
            used: std::mem::take(&mut self.drop_state.used_drops),
            unsupported: std::mem::replace(&mut self.drop_state.types.unsupported, unsupported),
            cache,
            sizes,
        }
    }

    /// The type parameters whose drops the body being lowered has used.
    pub(super) fn used_drops(&self) -> HashSet<u32> {
        self.drop_state.used_drops.clone()
    }

    /// What `f` makes of what's in the impl's terms, in a copied default:
    /// the arguments of the impl's own method its body calls, or a
    /// dictionary of its impl's, whose type parameters are the impl's, not
    /// its trait's, which share their indexes: asked of the impl's drops
    /// (ADR 0495). Anywhere else, `f`'s.
    pub(super) fn with_impl_drops<T>(&mut self, f: impl FnOnce(&mut Self) -> R<T>) -> R<T> {
        let Some(mut outer) = self.drop_state.outer.pop() else {
            return f(self);
        };
        self.exchange_drops(&mut outer);
        let made = f(self);
        self.exchange_drops(&mut outer);
        self.drop_state.outer.push(outer);
        made
    }

    /// The function's drops and `other`'s, each in the other's place.
    fn exchange_drops(&mut self, other: &mut SwappedDrops<'tcx>) {
        let state = &mut self.drop_state;
        std::mem::swap(&mut state.param_drops, &mut other.params);
        std::mem::swap(&mut state.types.given, &mut other.given);
        std::mem::swap(&mut state.types.items, &mut other.items);
        std::mem::swap(&mut state.types.slots, &mut other.slots);
        std::mem::swap(&mut state.types.self_args, &mut other.self_args);
        std::mem::swap(&mut state.used_drops, &mut other.used);
        std::mem::swap(&mut state.types.unsupported, &mut other.unsupported);
        std::mem::swap(&mut *state.types.cache.borrow_mut(), &mut other.cache);
        std::mem::swap(&mut *state.sizes.borrow_mut(), &mut other.sizes);
    }

    fn restore_drops(&mut self, swapped: SwappedDrops<'tcx>) {
        self.drop_state.param_drops = swapped.params;
        self.drop_state.types.given = swapped.given;
        self.drop_state.types.items = swapped.items;
        self.drop_state.types.slots = swapped.slots;
        self.drop_state.types.self_args = swapped.self_args;
        self.drop_state.used_drops = swapped.used;
        self.drop_state.types.unsupported = swapped.unsupported;
        *self.drop_state.types.cache.borrow_mut() = swapped.cache;
        *self.drop_state.sizes.borrow_mut() = swapped.sizes;
    }

    /// The drops this function is given, as it takes them: by their type
    /// parameters' order.
    pub(super) fn given_drops(&self) -> Vec<String> {
        let mut given: Vec<(&u32, &String)> = self.drop_state.param_drops.iter().collect();
        given.sort();
        given.into_iter().map(|(_, name)| name.clone()).collect()
    }

    /// `ZZone.$dropOffset`, an associated type's drop, of the dictionary of
    /// the impl it's of, which has one where its type does (ADR 0178).
    pub(super) fn item_drop(&self, ty: Ty<'tcx>) -> Option<Expr> {
        let (owner, item) = self.drop_query().item_drop_of(ty)?;
        let dictionary = self.evidence_for(owner)?;
        Some(Expr::member(dictionary, item_drop_key(self.tcx, item)))
    }

    /// `drop_function` of a `ty` a std function drops itself, `truncate`'s
    /// items or `insert`'s old value: of a type parameter whose drop this
    /// function isn't given, none, one its callers give none with a
    /// destructor (ADR 0190).
    pub(super) fn std_drop_function(&mut self, ty: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        let drop = self.drop_function(ty, span)?;
        if drop.is_none() {
            self.require_no_drops(ty);
        }
        Ok(drop)
    }

    /// The function that drops a `ty`, which a generic function is given for
    /// its type parameter: its `drop` itself, when that's all its drop is,
    /// `noisyDrop_drop`, or an arrow; a type parameter's is the one this
    /// function was given. None for a type with nothing to drop.
    pub(super) fn drop_function(&mut self, ty: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        if let ty::Param(_) = ty.kind() {
            let index = self.given_drop(ty);
            if let Some(index) = index {
                self.drop_state.used_drops.insert(index);
            }
            return Ok(index.map(|index| Expr::var(&self.drop_state.param_drops[&index])));
        }
        if let Some(index) = self.given_drop(ty) {
            self.drop_state.used_drops.insert(index);
            return Ok(Some(Expr::var(&self.drop_state.param_drops[&index])));
        }
        match self.drops(ty) {
            Drops::Nothing => return Ok(None),
            Drops::Unsupported(t, what) => return Err(self.unsupported(span, &describe(t, what))),
            Drops::Runs => {}
        }
        if let Some(drop) = self.item_drop(ty) {
            return Ok(Some(drop));
        }
        let base = match ty.kind() {
            ty::Adt(adt, _) => lower_first(self.tcx.item_name(adt.did()).as_str()),
            _ => "value".to_string(),
        };
        let param = self.fresh(&base);
        let mut body = Vec::new();
        self.drop_value(Expr::var(&param), ty, span, &mut body)?;
        // `(noisy) => noisyDrop_drop(noisy)` is `noisyDrop_drop`.
        if let [
            Stmt {
                kind: StmtKind::Expr(call),
                ..
            },
        ] = body.as_slice()
            && let js::ExprKind::Call(callee, args) = &call.kind
            && let [arg] = args.as_slice()
            && matches!(&arg.kind, js::ExprKind::Var(name) if *name == param)
            && matches!(&callee.kind, js::ExprKind::Var(_) | js::ExprKind::Symbol(_))
        {
            return Ok(Some((**callee).clone()));
        }
        // `(intoIter) => { for (const item of intoIter) nDrop_drop(item); }`,
        // what drops each an iterator hasn't given, is `$dropEach(nDrop_drop)`.
        if let [
            Stmt {
                kind:
                    StmtKind::ForOf {
                        label: None,
                        pattern: js::Pattern::Name(item),
                        iterable,
                        body: each,
                        ..
                    },
                ..
            },
        ] = body.as_slice()
            && matches!(&iterable.kind, js::ExprKind::Var(name) if *name == param)
            && let [
                Stmt {
                    kind: StmtKind::Expr(call),
                    ..
                },
            ] = each.as_slice()
            && let js::ExprKind::Call(callee, args) = &call.kind
            && let [arg] = args.as_slice()
            && matches!(&arg.kind, js::ExprKind::Var(name) if name == item)
            && matches!(&callee.kind, js::ExprKind::Var(_) | js::ExprKind::Symbol(_))
        {
            return Ok(Some(Expr::call_helper("$dropEach", vec![(**callee).clone()])));
        }
        Ok(Some(Expr::arrow(vec![param.into()], body)))
    }

    /// The function that drops what generic code holds of a `ty`: `ty`'s
    /// drop function, or where its caller made it a JS iterator, `iterated`,
    /// `$iterator(value, next)`, one that drops `value`, its `of` (ADR 0421).
    pub(super) fn held_drop_function(&mut self, ty: Ty<'tcx>, iterated: bool, span: Span) -> R<Option<Expr>> {
        if !iterated || !self.has_drops(ty) {
            return self.drop_function(ty, span);
        }
        let param = self.fresh("it");
        let mut body = Vec::new();
        self.drop_value(Expr::member(Expr::var(&param), "of"), ty, span, &mut body)?;
        Ok(Some(Expr::arrow(vec![param.into()], body)))
    }

    /// The drop function for a `ty` that `callee`, which takes only the
    /// drops it uses, is given for its type parameter `index`: an argument
    /// the pipeline keeps where `callee` uses it (ADR 0300). The drops of
    /// this function's that it's made of are passed on, not used, where
    /// this function takes only those it uses too.
    pub(super) fn drop_argument(
        &mut self,
        callee: DefId,
        index: u32,
        (ty, iterated): (Ty<'tcx>, bool),
        span: Span,
    ) -> R<Expr> {
        let before = std::mem::take(&mut self.drop_state.used_drops);
        let drop = self.held_drop_function(ty, iterated, span);
        let through = std::mem::replace(&mut self.drop_state.used_drops, before);
        if !self.in_copied_default() && self.krate.drop_uses.borrow().takes_used(self.item) {
            let mut uses = self.krate.drop_uses.borrow_mut();
            uses.passed
                .extend(through.into_iter().map(|param| ((self.item, param), (callee, index))));
        } else {
            self.drop_state.used_drops.extend(through);
        }
        let drop = drop?.unwrap_or_else(Expr::undefined);
        Ok(Expr::drop_argument(callee.index.as_u32(), index, drop))
    }

    /// Note which drops `id`, which takes only those it uses, used, and the
    /// names it took them by (ADR 0300).
    pub(super) fn note_drop_uses(&self, id: DefId) {
        let mut uses = self.krate.drop_uses.borrow_mut();
        if !uses.takes_used(id) {
            return;
        }
        uses.used
            .extend(self.drop_state.used_drops.iter().map(|&param| (id, param)));
        let names = self
            .drop_state
            .param_drops
            .iter()
            .map(|(&index, name)| (index, name.clone()));
        uses.names.insert(id, names.collect());
    }

    /// Whether dropping a `ty` reads it once, so a value that isn't a
    /// variable can be dropped where it's made: `noisyDrop_drop(["x"])`.
    pub(super) fn drops_once(&self, ty: Ty<'tcx>) -> bool {
        match ty.kind() {
            // Its drop drops what it holds, not the function.
            ty::Closure(..) => true,
            ty::Adt(_, args) if ty.is_box() || self.recognition().pinned(ty).is_some() => {
                self.drops_once(args.type_at(0))
            }
            ty::Adt(adt, args) => {
                self.tcx
                    .adt_destructor(adt.did())
                    .is_some_and(|d| self.runs_drop(d.did))
                    && adt.is_struct()
                    && adt.all_fields().all(|f| !self.has_drops(self.field_ty(f, args)))
            }
            _ => false,
        }
    }

    /// `value`, which `drop_value` may read more than once, in a `const`
    /// first unless it reads the same each time, or is read once.
    pub(super) fn droppable(&mut self, value: Expr, ty: Ty<'tcx>, out: &mut Vec<Stmt>) -> Expr {
        if value.reads_same() || self.drops_once(ty) {
            value
        } else {
            self.spill("value", value, out)
        }
    }
}

/// The type parameters a value of `ty` holds, not behind a reference: what
/// it drops of a caller's (ADR 0190).
fn owned_params<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Vec<u32> {
    let mut params = Vec::new();
    let mut walker = ty.walk();
    while let Some(part) = walker.next() {
        let Some(part) = part.as_type() else { continue };
        match part.kind() {
            // Behind a reference, or a pointer, a `NonNull`'s too (ADR 0407).
            ty::Ref(..) => walker.skip_current_subtree(),
            _ if super::pointers::raw_pointee(tcx, part).is_some() => walker.skip_current_subtree(),
            ty::Param(param) => params.push(param.index),
            _ => {}
        }
    }
    params
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
    no_drops: &RefCell<HashMap<DefId, BTreeSet<u32>>>,
    checks: &[DropCheck],
) -> bool {
    let mut reported = HashSet::new();
    let mut changed = true;
    while changed {
        changed = false;
        for check in checks {
            let required = no_drops.borrow().get(&check.callee).cloned().unwrap_or_default();
            for (index, params) in check.given.iter().filter(|(index, _)| required.contains(index)) {
                match params {
                    None if reported.insert(check.span) => {
                        let (path, name) = (tcx.def_path_str(check.callee), param_name(tcx, check.callee, *index));
                        let what = format!("giving `{path}`'s `{name}` a type with a destructor, where it takes none");
                        super::refusal::refuse(tcx, check.span, super::refusal::Refused::Feature(&what));
                    }
                    None => {}
                    Some(params) => {
                        let mut no_drops = no_drops.borrow_mut();
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

/// What the crate's functions that take only the drops they use did with
/// them, as each was lowered (ADR 0300): those they used, those they passed
/// on as another's, and the names they took them by.
#[derive(Default)]
pub(super) struct DropUses {
    takes_used: HashSet<DefId>,
    used: HashSet<(DefId, u32)>,
    passed: Vec<((DefId, u32), (DefId, u32))>,
    names: HashMap<DefId, Vec<(u32, String)>>,
}

impl DropUses {
    /// Each function given drops that's called where it's named, whose
    /// callers are all seen or told by its manifest: not a trait's method,
    /// which a dictionary's callers call, nor an impl's.
    pub(super) fn new(tcx: TyCtxt<'_>, drop_params: &HashMap<DefId, Vec<u32>>) -> DropUses {
        let takes_used = drop_params
            .keys()
            .copied()
            .filter(|&id| {
                id.is_local()
                    && tcx.trait_of_assoc(id).is_none()
                    && tcx.trait_impl_of_assoc(id).is_none()
                    && !matches!(tcx.def_kind(id), DefKind::Impl { .. })
            })
            .collect();
        DropUses {
            takes_used,
            ..DropUses::default()
        }
    }

    pub(super) fn takes_used(&self, id: DefId) -> bool {
        self.takes_used.contains(&id)
    }

    /// The drops each function keeps, those it uses or passes on to one
    /// that keeps it, in `drop_params`; and the lowered functions with only
    /// those, as parameters and arguments.
    pub(super) fn keep(self, drop_params: &mut HashMap<DefId, Vec<u32>>, items: &mut [(DefId, super::LoweredFn)]) {
        let mut kept = self.used;
        let mut changed = true;
        while changed {
            changed = false;
            for &(from, to) in &self.passed {
                if kept.contains(&to) && kept.insert(from) {
                    changed = true;
                }
            }
        }
        for id in &self.takes_used {
            if let Some(indices) = drop_params.get_mut(id) {
                indices.retain(|&index| kept.contains(&(*id, index)));
            }
        }
        for (id, item) in items {
            if let Some(names) = self.names.get(id) {
                let unused: HashSet<&str> = names
                    .iter()
                    .filter(|&&(index, _)| !kept.contains(&(*id, index)))
                    .map(|(_, name)| name.as_str())
                    .collect();
                item.function
                    .params
                    .retain(|param| !matches!(param, js::Pattern::Name(name) if unused.contains(name.as_str())));
            }
            js::each_expr_mut(&mut item.function.body, &mut |e| keep_arguments(e, &kept));
        }
    }
}

/// `e` with only the drop arguments kept: a call's, and the call an arrow
/// that only passes its parameters on is, which is then its callee,
/// `(arg0) => keep(arg0)` being `keep`.
fn keep_arguments(e: &mut Expr, kept: &HashSet<(DefId, u32)>) {
    let local = |item: u32| DefId {
        krate: rustc_span::def_id::LOCAL_CRATE,
        index: rustc_span::def_id::DefIndex::from_u32(item),
    };
    // One left last, `undefined`, is no argument: prepare leaves it out.
    let keep = |args: &mut Vec<Expr>| {
        *args = std::mem::take(args)
            .into_iter()
            .filter_map(|arg| match arg.kind {
                js::ExprKind::DropArgument(item, index, drop) => kept.contains(&(local(item), index)).then_some(*drop),
                _ => Some(arg),
            })
            .collect();
    };
    let has_drops = |args: &[Expr]| args.iter().any(|a| matches!(a.kind, js::ExprKind::DropArgument(..)));
    let callee = match &mut e.kind {
        js::ExprKind::Arrow(params, body) => {
            let [
                Stmt {
                    kind: StmtKind::Return(Some(call)),
                    ..
                },
            ] = body.as_mut_slice()
            else {
                return;
            };
            let js::ExprKind::Call(callee, args) = &mut call.kind else {
                return;
            };
            if !has_drops(args) {
                return;
            }
            keep(args);
            let names: Vec<&str> = params
                .iter()
                .filter_map(|param| match param {
                    js::Pattern::Name(name) => Some(name.as_str()),
                    _ => None,
                })
                .collect();
            let passed = names.len() == params.len()
                && names.len() == args.len()
                && names
                    .iter()
                    .zip(args.iter())
                    .all(|(name, arg)| matches!(&arg.kind, js::ExprKind::Var(a) if a == name))
                && match &callee.kind {
                    js::ExprKind::Var(name) => !names.contains(&name.as_str()),
                    js::ExprKind::Symbol(_) => true,
                    _ => false,
                };
            passed.then(|| (**callee).clone())
        }
        js::ExprKind::Call(_, args) | js::ExprKind::OptionalCall(_, args) | js::ExprKind::New(_, args)
            if has_drops(args) =>
        {
            keep(args);
            None
        }
        _ => None,
    };
    if let Some(callee) = callee {
        *e = callee;
    }
}

/// The name of `callee`'s type parameter `index`, `I`.
fn param_name(tcx: TyCtxt<'_>, callee: DefId, index: u32) -> String {
    tcx.generics_of(callee).param_at(index as usize, tcx).name.to_string()
}

/// `impl.$drop?.(value)`: a `dyn`'s value dropped through its dictionary,
/// if what it holds has a drop.
fn dyn_dropped(dictionary: Expr, value: Expr, js_span: js::Span) -> Stmt {
    let drop = Expr::member(dictionary, js::DICTIONARY_DROP);
    StmtKind::Expr(Expr {
        reads: false,
        kind: js::ExprKind::OptionalCall(Box::new(drop), vec![value]),
        span: js_span,
    })
    .at(js_span)
}
