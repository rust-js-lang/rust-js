//! Destructors (ADR 0098): the scopes, flags and temporaries that drop a
//! value where rustc does, and the JS that drops it. What dropping a type
//! runs is `types`'s, and what a body owns and moves, found before it's
//! lowered, `facts`'.
//!
//! A scope that owns such a value is a `try`, and its drops the `finally`.
//! What this can't do yet is an error, found before any JS is written: a
//! value with a destructor that nothing drops would be a wrong answer.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::rc::Rc;

use rustc_hir::def::DefKind;
use rustc_hir::{self as hir, HirId, Node};
use rustc_middle::middle::region;
use rustc_middle::thir::{BlockId, ExprId, ExprKind, LocalVarId, Pat, StmtKind as ThirStmt};
use rustc_middle::ty::{self, GenericArgsRef, Ty, TyCtxt, TypeVisitableExt};
use rustc_span::Span;
use rustc_span::def_id::DefId;

use super::recognition::{ChannelEnd, StdItem};
use super::representation::variant_field;
use super::traits::item_drop_key;
use super::{FnCx, R, lower_first};
use crate::js::{self, Expr, Op, Stmt, StmtKind};
use crate::runtime::Helper;

mod facts;
mod types;

pub(super) use facts::is_place;
use facts::{Facts, TempKind, binds_any, find_facts};
pub(super) use types::Drops;
use types::{DropQuery, TypeDrops, describe};

/// The drop functions one drop makes, which come before its code: a call
/// to one may be in a branch, as an `Option`'s, that another isn't in.
#[derive(Default)]
struct Made<'tcx> {
    functions: Vec<(Ty<'tcx>, String)>,
    defs: Vec<Stmt>,
}

/// A variable that owns a value with a destructor, dropped when its scope
/// ends: its JS value, and the flag that says it's still owned, if it moves.
struct Owned<'tcx> {
    value: Expr,
    ty: Ty<'tcx>,
    flag: Option<String>,
    /// Each part moved somewhere, and its flag.
    parts: Vec<(Path, Option<String>)>,
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

/// A temporary that ends with the statement being lowered.
pub(super) struct Temp<'tcx> {
    name: String,
    ty: Ty<'tcx>,
    flag: Option<String>,
    /// The operand it is, which must have been moved by the statement's end.
    operand: Option<ExprId>,
    /// The parts a pattern moves out of it: on every path, or with a flag.
    parts: Vec<(Path, Option<String>)>,
}

/// The statement being lowered, which a statement inside it saves and puts
/// back: the scopes its temporaries end in, its own and, for a `let`, the
/// rest of the block's, and the temporaries that end with it.
#[derive(Default)]
pub(super) struct Statement<'tcx> {
    scopes: Option<(region::Scope, Option<region::Scope>)>,
    temps: Vec<Temp<'tcx>>,
}

/// A copied default's drops: the drop functions of its trait's type
/// parameters, by index, and those it can't make, with why (ADR 0098).
pub(super) type DefaultDrops<'tcx> = (HashMap<u32, String>, HashMap<u32, (Ty<'tcx>, &'static str)>);

/// What a body inside another, a closure's or a copied default's, takes
/// from the drops of the one it's in while it's lowered, given back as it
/// ends: its statement and scopes, and a default's, the item's drops.
pub(super) struct EnclosingDrops<'tcx> {
    scopes: BodyScopes<'tcx>,
    swapped: Option<SwappedDrops<'tcx>>,
    /// The enclosing body's drop functions, where this body declares its own.
    hoisted: Option<Option<Hoisted<'tcx>>>,
}

/// The statement and the expressions being lowered, whose temporaries a
/// body inside them, a closure's, has none of: it has its own.
struct BodyScopes<'tcx> {
    statement: Statement<'tcx>,
    open: Vec<(region::Scope, Vec<Temp<'tcx>>)>,
}

/// A function's drops while a copied default body has its own.
struct SwappedDrops<'tcx> {
    params: HashMap<u32, String>,
    given: HashSet<u32>,
    used: HashSet<u32>,
    unsupported: HashMap<u32, (Ty<'tcx>, &'static str)>,
    cache: HashMap<Ty<'tcx>, Drops<'tcx>>,
    sizes: HashMap<Ty<'tcx>, (usize, bool)>,
}

/// A function's drops, as its bodies are lowered.
#[derive(Default)]
pub(super) struct DropState<'tcx> {
    /// What a type drops depends on, beside its types (ADR 0098).
    types: TypeDrops<'tcx>,
    sizes: RefCell<HashMap<Ty<'tcx>, (usize, bool)>>,
    /// Each body's facts, by the address of its THIR: a closure's is its own.
    facts: HashMap<usize, Rc<Facts>>,
    /// The owners in scope, innermost last.
    owned: Vec<Owned<'tcx>>,
    /// Every owner given a scope, and each move lowered, which a body's
    /// facts must all be once it's lowered.
    registered: HashSet<LocalVarId>,
    lowered_moves: HashSet<(usize, ExprId)>,
    flags: HashMap<LocalVarId, String>,
    /// Moves that are a call's operands, and the statements that clear
    /// their flags, which wait until every operand is evaluated.
    deferred: HashMap<(usize, ExprId), Option<Stmt>>,
    statement: Statement<'tcx>,
    /// An operand temporary's flag, and whether its move was lowered.
    temp_flags: HashMap<(usize, ExprId), String>,
    temps_moved: HashSet<(usize, ExprId)>,
    /// The function being lowered's drop functions, by the index of the
    /// type parameter each drops.
    param_drops: HashMap<u32, String>,
    /// The type parameters whose drops the body has used.
    used_drops: HashSet<u32>,
    /// The variables declared by `let`s that can't leave (ADR 0301): what
    /// a scope moves after them it moves before anything can leave.
    quiet: HashSet<String>,
    part_flags: HashMap<(LocalVarId, Path), String>,
    /// Each closure made here that holds a value with a destructor: the body
    /// it's made in, and the variables it holds, which its drop drops.
    closures: HashMap<DefId, (DefId, Vec<(Expr, Ty<'tcx>)>)>,
    /// The flags of each temporary's parts that a pattern moves on some paths.
    temp_part_flags: HashMap<(usize, ExprId), Vec<(Path, String)>>,
    /// The scopes of the expressions being lowered, innermost last, and the
    /// temporaries that end with each: a condition's, a block's tail's.
    open: Vec<(region::Scope, Vec<Temp<'tcx>>)>,
    /// The drop functions of the function or closure being lowered, each
    /// declared once at its top; `None` where each drop declares its own.
    hoisted: Option<Hoisted<'tcx>>,
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
    pub(in crate::lower) fn drops(&self, ty: Ty<'tcx>) -> Drops<'tcx> {
        self.drop_query().drops(ty)
    }

    pub(in crate::lower) fn has_drops(&self, ty: Ty<'tcx>) -> bool {
        self.drop_query().has_drops(ty)
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
                let params = owned_params(ty);
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
        let params = owned_params(ty);
        self.krate
            .no_drops
            .borrow_mut()
            .entry(owner)
            .or_default()
            .extend(params);
        true
    }

    /// Is `drop`, a type's `Drop::drop`, one rust-js runs (ADR 0100)?
    pub(in crate::lower) fn runs_drop(&self, drop: DefId) -> bool {
        self.drop_query().runs_drop(drop)
    }

    /// The parts a pattern moves out of what it's matched against, by value.
    pub(super) fn pattern_paths(&self, pat: &Pat<'tcx>) -> Option<Vec<Path>> {
        self.drop_query().pattern_paths(pat)
    }

    /// A closure, at `closure`, made here: the places its drop drops.
    pub(super) fn closure_made(&mut self, closure: DefId, places: Vec<(Expr, Ty<'tcx>)>) {
        self.drop_state.closures.insert(closure, (self.body_owner, places));
    }

    /// The variables of the body being lowered that a closure flags as its
    /// own as it's lowered, given back once it is.
    pub(super) fn take_flags(&mut self, vars: &[LocalVarId]) -> Vec<(LocalVarId, Option<String>)> {
        vars.iter().map(|&v| (v, self.drop_state.flags.remove(&v))).collect()
    }

    pub(super) fn give_flags(&mut self, flags: Vec<(LocalVarId, Option<String>)>) {
        for (var, flag) in flags {
            match flag {
                Some(flag) => self.drop_state.flags.insert(var, flag),
                None => self.drop_state.flags.remove(&var),
            };
        }
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
            // A channel's end (ADR 0142).
            ty::Adt(..) if let Some(end) = self.recognition().channel_end(ty) => {
                self.runtime.insert(Helper::Channel);
                let drop = match end {
                    ChannelEnd::Sender => "$dropSender",
                    ChannelEnd::Receiver => "$dropReceiver",
                };
                out.push(StmtKind::Expr(Expr::call(Expr::var(drop), vec![value])).at(js_span));
            }
            // A map's values, a `BTreeMap`'s in its keys' order (ADR 0321).
            ty::Adt(_, args) if self.is_map(ty) => {
                let value_ty = args.type_at(1);
                if self.is_sorted(ty) {
                    let entries = self.in_order_of(value, ty, span)?;
                    let name = self.fresh("value");
                    let mut body = Vec::new();
                    self.drop_in(Expr::var(&name), value_ty, span, made, &mut body)?;
                    out.push(
                        StmtKind::ForOf {
                            label: None,
                            pattern: js::Pattern::Array(vec![None, Some(name)]),
                            mutable: false,
                            iterable: entries,
                            body,
                        }
                        .at(js_span),
                    );
                } else {
                    let values = Expr::call(Expr::member(value, "values"), Vec::new());
                    self.drop_items(values, value_ty, span, made, out)?;
                }
            }
            // A `RefCell`'s value (ADR 0320).
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::RefCell) => {
                self.drop_in(Expr::member(value, "value"), args.type_at(0), span, made, out)?
            }
            // A counted `Rc`: `$rcDrop(rc, drop)`, `drop` what the last drops of
            // what it points at, if that has a destructor; a `Weak`: `$weakDrop`
            // (ADR 0320).
            ty::Adt(..) if let Some(pointee) = self.counted_rc(ty) => {
                self.runtime.insert(Helper::Rc);
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
                out.push(StmtKind::Expr(Expr::call(Expr::var("$rcDrop"), args)).at(js_span));
            }
            // A heap's `PeekMut` (ADR 0333).
            ty::Adt(..) if self.recognition().peek_mut_of(ty).is_some() => {
                self.runtime.insert(Helper::HeapOps);
                out.push(StmtKind::Expr(Expr::call(Expr::var("$peekMutDrop"), vec![value])).at(js_span));
            }
            // A guard: `$unborrow(cell)` (ADR 0328).
            ty::Adt(..) if self.is_guard(ty) => {
                self.runtime.insert(Helper::Borrow);
                out.push(StmtKind::Expr(Expr::call(Expr::var("$unborrow"), vec![value])).at(js_span));
            }
            ty::Adt(..) if self.weak_of(ty).is_some() => {
                self.runtime.insert(Helper::Rc);
                out.push(StmtKind::Expr(Expr::call(Expr::var("$weakDrop"), vec![value])).at(js_span));
            }
            // What a closure holds: the variables it took, where it was made.
            // Only there, or in a closure made inside it, can JS see them.
            ty::Closure(def_id, _) => {
                let Some((made_in, places)) = self.drop_state.closures.get(def_id).cloned() else {
                    return Err(self.unsupported(span, "dropping a closure that holds a value with a destructor here"));
                };
                if !std::iter::successors(Some(self.body_owner), |&d| self.tcx.opt_parent(d)).any(|d| d == made_in) {
                    return Err(self.unsupported(span, "dropping a closure that holds a value with a destructor here"));
                }
                for (place, ty) in places {
                    self.drop_in(place, ty, span, made, out)?;
                }
            }
            // A `dyn`'s: its dictionary's `$drop`, if what it holds has one.
            ty::Dynamic(..) => {
                let drop = Expr::member(Expr::member(value.clone(), "impl"), "$drop");
                out.push(
                    StmtKind::Expr(Expr {
                        kind: js::ExprKind::OptionalCall(Box::new(drop), vec![Expr::member(value, "value")]),
                        span: js_span,
                    })
                    .at(js_span),
                );
            }
            // `dropT?.(value)`: the caller's drop, if its `T` has one.
            ty::Param(param) => {
                self.drop_state.used_drops.insert(param.index);
                let drop = Expr::var(&self.drop_state.param_drops[&param.index]);
                let js_span = self.js_span(span);
                out.push(
                    StmtKind::Expr(Expr {
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
                        true => Expr::object(vec![js::Prop::Field("value".into(), value.clone())]),
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
                        self.drop_in(part, self.field_ty(field, args), span, made, out)?;
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
                        let tag = super::bindings::variant_tag(self.tcx, variant);
                        let test = match adt.variants().len() {
                            1 => Expr::bool(true),
                            _ if untagged => self.untagged_variant_test(ty, variant, &value),
                            _ => Expr::bin(
                                Op::Eq,
                                Expr::member(value.clone(), super::bindings::tag_key(self.tcx, adt.did())),
                                tag,
                            ),
                        };
                        out.push(StmtKind::If(test, fields, None).at(js_span));
                    }
                } else {
                    return Err(self.unsupported(span, &format!("dropping `{ty}`")));
                }
            }
            _ => return Err(self.unsupported(span, &format!("dropping `{ty}`"))),
        }
        Ok(())
    }

    /// Drop `value`, a `ty` some of whose parts may have moved: a part with
    /// a flag only if it's still owned, the rest as `drop_value` would. Rust
    /// forbids moving out of a type with a `Drop` of its own (E0509), so
    /// what's around a moved part has no `drop` to call.
    fn drop_owned(
        &mut self,
        value: Expr,
        ty: Ty<'tcx>,
        parts: &[(Path, Option<String>)],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        if parts.is_empty() {
            return self.drop_value(value, ty, span, out);
        }
        let js_span = self.js_span(span);
        // This part itself moved on every path: nothing of it is left.
        if parts.iter().any(|(p, flag)| p.is_empty() && flag.is_none()) {
            return Ok(());
        }
        // This part itself moved, on some path: all of it only if it's owned.
        if let Some((_, Some(flag))) = parts.iter().find(|(p, _)| p.is_empty()) {
            let rest: Vec<(Path, Option<String>)> = parts.iter().filter(|(p, _)| !p.is_empty()).cloned().collect();
            let mut owned = Vec::new();
            self.drop_owned(value, ty, &rest, span, &mut owned)?;
            if !owned.is_empty() {
                out.push(StmtKind::If(Expr::var(flag), owned, None).at(js_span));
            }
            return Ok(());
        }
        let under = |step: (Option<u32>, usize)| -> Vec<(Path, Option<String>)> {
            parts
                .iter()
                .filter(|(p, _)| p[0] == step)
                .map(|(p, f)| (p[1..].to_vec(), f.clone()))
                .collect()
        };
        match ty.kind() {
            ty::Tuple(items) => {
                for (i, item) in items.iter().enumerate() {
                    let part = self.project(value.clone(), ty, i);
                    self.drop_owned(part, item, &under((None, i)), span, out)?;
                }
            }
            ty::Adt(adt, args) if adt.is_struct() => {
                for (i, field) in adt.non_enum_variant().fields.iter().enumerate() {
                    let part = self.project(value.clone(), ty, i);
                    self.drop_owned(part, self.field_ty(field, args), &under((None, i)), span, out)?;
                }
            }
            // `Some(x)` is `x` itself (ADR 0030).
            ty::Adt(adt, _) if let Some(inner) = self.option_of(ty) => {
                let some = adt.variants().iter().position(|v| !v.fields.is_empty()).unwrap_or(1) as u32;
                let mut body = Vec::new();
                self.drop_owned(value.clone(), inner, &under((Some(some), 0)), span, &mut body)?;
                if !body.is_empty() {
                    out.push(StmtKind::If(Expr::bin(Op::LooseNe, value, Expr::null()), body, None).at(js_span));
                }
            }
            ty::Adt(adt, args) if adt.is_enum() => {
                // An untagged enum's variant is its payload (ADR 0214).
                let untagged = self.untagged(ty).is_some();
                for (index, variant) in adt.variants().iter().enumerate() {
                    let mut fields = Vec::new();
                    for (i, field) in variant.fields.iter().enumerate() {
                        let part = match untagged {
                            true => value.clone(),
                            false => Expr::member(value.clone(), variant_field(self.tcx, variant, i)),
                        };
                        self.drop_owned(
                            part,
                            self.field_ty(field, args),
                            &under((Some(index as u32), i)),
                            span,
                            &mut fields,
                        )?;
                    }
                    if fields.is_empty() {
                        continue;
                    }
                    let test = match adt.variants().len() {
                        1 => Expr::bool(true),
                        _ if untagged => self.untagged_variant_test(ty, variant, &value),
                        _ => Expr::bin(
                            Op::Eq,
                            Expr::member(value.clone(), super::bindings::tag_key(self.tcx, adt.did())),
                            super::bindings::variant_tag(self.tcx, variant),
                        ),
                    };
                    out.push(StmtKind::If(test, fields, None).at(js_span));
                }
            }
            _ => return Err(self.unsupported(span, &format!("dropping what's left of a `{ty}`"))),
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
        if self.krate.library {
            return HashSet::new();
        }
        // Each body's own facts, asked with no dictionaries: a type
        // parameter's drop is then a maybe, and its borrow counted.
        let query = DropQuery {
            recognition: self.recognition(),
            evidence: self.no_evidence_query(),
            library: false,
            counted: self.krate.counted,
            state: &self.drop_state.types,
        };
        let mut plain = HashSet::new();
        let mut cells = HashSet::new();
        let bodies = (self.krate.bodies.values().copied()).chain(self.krate.closures.values().copied());
        for body in bodies {
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
        for (span, what) in &facts.problems {
            failed = Some(self.unsupported(*span, what));
        }
        match failed {
            Some(guar) => Err(guar),
            None => Ok(facts),
        }
    }

    /// `var`, just bound to `value` in `out`, owns a value with a
    /// destructor: its scope drops it. One that's moved gets a flag.
    pub(super) fn own(&mut self, var: LocalVarId, value: Expr, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> R<()> {
        let js_span = self.js_span(span);
        self.own_at(var, value, ty, js_span, out)
    }

    /// `own`, with the flags' statements at `js_span`. A part that's moved
    /// somewhere, `pair.a`, gets a flag of its own, `pair$a$live`.
    pub(super) fn own_at(
        &mut self,
        var: LocalVarId,
        value: Expr,
        ty: Ty<'tcx>,
        js_span: js::Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let facts = self.drop_facts()?;
        let base = match &value.kind {
            js::ExprKind::Var(name) => name.clone(),
            _ => "value".into(),
        };
        let flag = facts.moved.contains(&var).then(|| {
            let flag = self.fresh(&format!("{base}$live"));
            out.push(StmtKind::Let(flag.clone(), Some(Expr::bool(true))).at(js_span));
            self.drop_state.flags.insert(var, flag.clone());
            flag
        });
        let mut parts: Vec<(Path, Option<String>)> = Vec::new();
        for path in facts.parts.get(&var).into_iter().flatten() {
            if parts.iter().any(|(p, _)| p == path) {
                continue;
            }
            let named = self.path_name(ty, path);
            let flag = self.fresh(&format!("{base}${named}$live"));
            out.push(StmtKind::Let(flag.clone(), Some(Expr::bool(true))).at(js_span));
            self.drop_state.part_flags.insert((var, path.clone()), flag.clone());
            parts.push((path.clone(), Some(flag)));
        }
        self.drop_state.registered.insert(var);
        self.drop_state.owned.push(Owned { value, ty, flag, parts });
        Ok(())
    }

    /// A part's path as a name, its fields' and variants': `a`, `0$1`, `Some$0`.
    fn path_name(&self, ty: Ty<'tcx>, path: &Path) -> String {
        let mut names = Vec::new();
        let mut at = ty;
        for &(variant, field) in path {
            match at.kind() {
                ty::Adt(adt, args) => {
                    let def = match variant {
                        Some(v) => {
                            let def = adt.variants().iter().nth(v as usize).expect("the variant");
                            names.push(def.name.to_string());
                            def
                        }
                        None => adt.non_enum_variant(),
                    };
                    let f = def.fields.iter().nth(field).expect("the field");
                    names.push(f.name.to_string());
                    at = self.field_ty(f, args);
                }
                ty::Tuple(items) => {
                    names.push(field.to_string());
                    at = items[field];
                }
                _ => names.push(field.to_string()),
            }
        }
        names.join("$")
    }

    /// What `pat`, matched against `scrutinee`, a variable whose parts have
    /// flags, moves out of it: those parts' flags are cleared.
    pub(super) fn clear_parts(&mut self, scrutinee: ExprId, pat: &Pat<'tcx>, out: &mut Vec<Stmt>) {
        let js_span = self.js_span(pat.span);
        // A temporary's, taken apart (ADR 0131).
        let key = std::ptr::from_ref(self.thir) as usize;
        if let Some(flags) = self.drop_state.temp_part_flags.get(&(key, self.strip(scrutinee))) {
            let flags = flags.clone();
            for path in self.pattern_paths(pat).unwrap_or_default() {
                if let Some((_, flag)) = flags.iter().find(|(p, _)| *p == path) {
                    out.push(StmtKind::Assign(Expr::var(flag), Expr::bool(false)).at(js_span));
                }
            }
            return;
        }
        let ExprKind::VarRef { id } = self.thir[self.strip(scrutinee)].kind else {
            return;
        };
        for path in self.pattern_paths(pat).unwrap_or_default() {
            if let Some(flag) = self.drop_state.part_flags.get(&(id, path)) {
                out.push(StmtKind::Assign(Expr::var(flag), Expr::bool(false)).at(js_span));
            }
        }
    }

    /// Whether `var` owns a value with a destructor.
    pub(super) fn is_owner(&mut self, var: LocalVarId) -> R<bool> {
        Ok(self.drop_facts()?.owners.contains_key(&var))
    }

    /// Whether `e` is a temporary that a pattern takes apart (ADR 0131).
    pub(super) fn takes_apart_temporary(&mut self, e: ExprId) -> R<bool> {
        Ok(self.drop_facts()?.temp_parts.contains_key(&self.strip(e)))
    }

    /// Whether `e` is a temporary, and why.
    pub(super) fn temp_kind(&mut self, e: ExprId) -> R<Option<TempKind>> {
        Ok(self.drop_facts()?.temps.get(&e).copied())
    }

    /// `e`, a value with a destructor that's a temporary: in a `const` of its
    /// own, dropped where rustc's scope tree ends it. One that ends with the
    /// statement is dropped by its `finally`; one a `let` extends is owned
    /// by the rest of the block, as a variable is.
    pub(super) fn temporary(&mut self, e: ExprId, kind: TempKind, value: Expr, out: &mut Vec<Stmt>) -> R<Expr> {
        let span = self.thir[e].span;
        let ty = self.thir[e].ty;
        let js_span = self.js_span(span);
        let base = match ty.peel_refs().kind() {
            ty::Adt(adt, _) => lower_first(self.tcx.item_name(adt.did()).as_str()),
            _ => "temporary".to_string(),
        };
        let name = self.fresh(&base);
        out.push(StmtKind::Const(name.clone(), value).at(js_span));
        let (statement, rest) = match self.drop_state.statement.scopes {
            Some((statement, rest)) => (Some(statement), rest),
            None => (None, None),
        };
        let key = std::ptr::from_ref(self.thir) as usize;
        // The expression whose scope it ends with, if not the statement's.
        let mut open = None;
        let flag = match kind {
            TempKind::Operand => {
                // Outside any statement, a function body's tail's: the scope
                // of what moves it, the innermost that isn't its own.
                if statement.is_none() {
                    let own = self.thir[e].temp_scope_id;
                    match self.drop_state.open.iter().rposition(|(s, _)| s.local_id != own) {
                        Some(at) => open = Some(at),
                        None => return Err(self.unsupported(span, "a temporary with a destructor here")),
                    }
                }
                let flag = self.fresh(&format!("{name}$live"));
                out.push(StmtKind::Let(flag.clone(), Some(Expr::bool(true))).at(js_span));
                self.drop_state.temp_flags.insert((key, e), flag.clone());
                Some(flag)
            }
            TempKind::Place => {
                let tree = self.tcx.region_scope_tree(self.body_owner.expect_local());
                match tree.temporary_scope(self.thir[e].temp_scope_id).temp_lifetime {
                    // Never dropped, as a promoted constant isn't.
                    None => return Ok(Expr::var(&name)),
                    Some(scope) if statement.is_some_and(|st| scope == st || self.ends_with_statement(scope, st)) => {
                        None
                    }
                    // `let r = &f();`: it lives as long as `r` does, less what a
                    // `let` moves out of it, `let (a, ref b) = (f(), g());` (ADR 0131).
                    Some(scope) if Some(scope) == rest => {
                        let parts = match self.drop_facts()?.temp_parts.get(&e) {
                            Some((_, true)) => return Err(self.unsupported(span, "a temporary with a destructor here")),
                            Some((paths, false)) => paths.iter().map(|p| (p.clone(), None)).collect(),
                            None => Vec::new(),
                        };
                        self.drop_state.owned.push(Owned {
                            value: Expr::var(&name),
                            ty,
                            flag: None,
                            parts,
                        });
                        return Ok(Expr::var(&name));
                    }
                    // A condition's, a block's tail's: its expression's.
                    Some(scope) => match self.drop_state.open.iter().rposition(|(s, _)| *s == scope) {
                        Some(at) => {
                            open = Some(at);
                            None
                        }
                        None => return Err(self.unsupported(span, "a temporary with a destructor here")),
                    },
                }
            }
        };
        let operand = (kind == TempKind::Operand).then_some(e);
        // What a pattern moves out of it (ADR 0131): a part moved on only
        // some paths gets a flag, `tuple$0$live`, as a variable's does.
        let mut parts: Vec<(Path, Option<String>)> = Vec::new();
        if let Some((paths, flagged)) = self.drop_facts()?.temp_parts.get(&e) {
            let mut flags = Vec::new();
            for path in paths {
                if parts.iter().any(|(p, _)| p == path) {
                    continue;
                }
                let flag = flagged.then(|| {
                    let flag = match self.path_name(ty, path) {
                        named if named.is_empty() => self.fresh(&format!("{name}$live")),
                        named => self.fresh(&format!("{name}${named}$live")),
                    };
                    out.push(StmtKind::Let(flag.clone(), Some(Expr::bool(true))).at(js_span));
                    flags.push((path.clone(), flag.clone()));
                    flag
                });
                parts.push((path.clone(), flag));
            }
            self.drop_state.temp_part_flags.insert((key, e), flags);
        }
        let temp = Temp {
            name: name.clone(),
            ty,
            flag,
            operand,
            parts,
        };
        match open {
            Some(at) => self.drop_state.open[at].1.push(temp),
            None => self.drop_state.statement.temps.push(temp),
        }
        Ok(Expr::var(&name))
    }

    /// Whether `scope` ends as `statement` does: an `if let`'s, whose
    /// scrutinee's temporaries end with the `if` in Rust 2024, where the
    /// `if` is the whole statement and has no `else` to run after them.
    fn ends_with_statement(&self, scope: region::Scope, statement: region::Scope) -> bool {
        if scope.data != region::ScopeData::IfThenRescope {
            return false;
        }
        // The scope is the `then` block's, inside the `if`.
        let owner = self.tcx.local_def_id_to_hir_id(self.body_owner.expect_local()).owner;
        let then = HirId {
            owner,
            local_id: scope.local_id,
        };
        let id = self.tcx.parent_hir_id(then);
        matches!(self.tcx.hir_node(id), Node::Expr(e) if matches!(e.kind, hir::ExprKind::If(_, _, None)))
            && self.tcx.parent_hir_id(id).local_id == statement.local_id
    }

    /// Start lowering a statement whose temporaries end in `scopes`: its own,
    /// and for a `let`, the rest of the block's. What it replaces, a
    /// statement it's inside, `end_statement` puts back.
    pub(super) fn begin_statement(&mut self, scopes: (region::Scope, Option<region::Scope>)) -> Statement<'tcx> {
        std::mem::replace(
            &mut self.drop_state.statement,
            Statement {
                scopes: Some(scopes),
                temps: Vec::new(),
            },
        )
    }

    /// Finish the statement `begin_statement` started: `lowered`, its JS,
    /// goes in `out`, each of its temporaries dropped by a `finally` after
    /// the `const` that holds it, last first.
    pub(super) fn end_statement(
        &mut self,
        outer: Statement<'tcx>,
        lowered: Vec<Stmt>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let statement = std::mem::replace(&mut self.drop_state.statement, outer);
        self.place_temps(statement.temps, lowered, span, out)
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
        let swapped = default.map(|(drops, unsupported)| self.swap_drops(drops, unsupported));
        self.drop_facts()?;
        Ok(EnclosingDrops {
            scopes: self.take_scopes(),
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
    pub(super) fn leave_body_drops(&mut self, enclosing: EnclosingDrops<'tcx>) -> R<Vec<Stmt>> {
        self.check_drops()?;
        self.give_scopes(enclosing.scopes);
        if let Some(swapped) = enclosing.swapped {
            self.restore_drops(swapped);
        }
        Ok(match enclosing.hoisted {
            Some(outer) => self.end_hoisting(outer),
            None => Vec::new(),
        })
    }

    /// The enclosing body's statement and expressions, while a body inside
    /// it is lowered.
    fn take_scopes(&mut self) -> BodyScopes<'tcx> {
        BodyScopes {
            statement: std::mem::take(&mut self.drop_state.statement),
            open: std::mem::take(&mut self.drop_state.open),
        }
    }

    fn give_scopes(&mut self, scopes: BodyScopes<'tcx>) {
        self.drop_state.statement = scopes.statement;
        self.drop_state.open = scopes.open;
    }

    /// Start lowering the expression whose scope is `scope`, which a
    /// temporary may end with, as a condition's does.
    pub(super) fn begin_scope(&mut self, scope: region::Scope) {
        self.drop_state.open.push((scope, Vec::new()));
    }

    /// Finish the expression `begin_scope` started, whose JS is `out` from
    /// `mark` on: each of the temporaries that end with it is dropped by a
    /// `finally` after it.
    pub(super) fn end_scope(&mut self, mark: usize, span: Span, out: &mut Vec<Stmt>) -> R<()> {
        let (_, temps) = self.drop_state.open.pop().expect("a scope begun");
        if temps.is_empty() {
            return Ok(());
        }
        let lowered = out.split_off(mark);
        self.place_temps(temps, lowered, span, out)
    }

    /// Whether the innermost scope begun has temporaries that end with it.
    pub(super) fn scope_has_temps(&self) -> bool {
        self.drop_state.open.last().is_some_and(|(_, temps)| !temps.is_empty())
    }

    /// `lowered`, in `out`, with `temps` dropped after it, last first.
    fn place_temps(&mut self, temps: Vec<Temp<'tcx>>, lowered: Vec<Stmt>, span: Span, out: &mut Vec<Stmt>) -> R<()> {
        let key = std::ptr::from_ref(self.thir) as usize;
        // An operand temporary must have been moved where it's an operand.
        for t in &temps {
            if let Some(e) = t.operand
                && !self.drop_state.temps_moved.contains(&(key, e))
            {
                return Err(self.unsupported(
                    self.thir[e].span,
                    "a value with a destructor made before what may panic or leave early, here",
                ));
            }
        }
        if temps.is_empty() {
            out.extend(lowered);
            return Ok(());
        }
        // Each is declared in the scope's own JS, not in a branch of it. An
        // operand may be, `B::from_name(flag).ok_or_else(f)?` in an `else`: it's
        // moved where it's made, so its branch's `finally` is the scope's.
        let mut lowered = lowered;
        let mut placed = Vec::new();
        let mut in_branches = Vec::new();
        for t in temps {
            match lowered
                .iter()
                .position(|s| matches!(&s.kind, StmtKind::Const(n, _) if *n == t.name))
                .or_else(|| assigned_in_try(&lowered, &t.name))
            {
                Some(at) if placed.iter().any(|&(other, _)| other == at) => {
                    return Err(self.unsupported(span, "a temporary with a destructor in a branch"));
                }
                Some(at) => placed.push((at, t)),
                None if t.operand.is_some() && t.parts.is_empty() => in_branches.push(t),
                None => return Err(self.unsupported(span, "a temporary with a destructor in a branch")),
            }
        }
        // Last first: wrapping one makes the `const`s after it assignments,
        // where a later one's wouldn't be found.
        for t in in_branches.into_iter().rev() {
            if self.place_in_branch(&mut lowered, t, span)?.is_some() {
                return Err(self.unsupported(span, "a temporary with a destructor in a branch"));
            }
        }
        placed.sort_by_key(|(at, _)| *at);
        self.wrap_temps(lowered, placed, span, out)
    }

    /// `temp` dropped after the rest of the branch of `stmts` that declares
    /// it, or given back if none does.
    fn place_in_branch(&mut self, stmts: &mut Vec<Stmt>, temp: Temp<'tcx>, span: Span) -> R<Option<Temp<'tcx>>> {
        if let Some(at) = stmts
            .iter()
            .position(|s| matches!(&s.kind, StmtKind::Const(n, _) if *n == temp.name))
        {
            let mut wrapped = Vec::new();
            self.wrap_temps(std::mem::take(stmts), vec![(at, temp)], span, &mut wrapped)?;
            *stmts = wrapped;
            return Ok(None);
        }
        let mut temp = temp;
        for s in stmts.iter_mut() {
            let branches: Vec<&mut Vec<Stmt>> = match &mut s.kind {
                StmtKind::If(_, then, els) => std::iter::once(then).chain(els.as_mut()).collect(),
                StmtKind::Labeled(_, body) | StmtKind::Try(body, _) => vec![body],
                _ => Vec::new(),
            };
            for branch in branches {
                match self.place_in_branch(branch, temp, span)? {
                    Some(back) => temp = back,
                    None => return Ok(None),
                }
            }
        }
        Ok(Some(temp))
    }

    fn wrap_temps(
        &mut self,
        mut lowered: Vec<Stmt>,
        mut placed: Vec<(usize, Temp<'tcx>)>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        if placed.is_empty() {
            out.extend(lowered);
            return Ok(());
        }
        let (at, temp) = placed.remove(0);
        // Through its declaration, and its flags'.
        let mut end = at + 1;
        let flags: Vec<&String> = temp
            .flag
            .iter()
            .chain(temp.parts.iter().filter_map(|(_, f)| f.as_ref()))
            .collect();
        while matches!(lowered.get(end).map(|s| &s.kind), Some(StmtKind::Let(n, _)) if flags.contains(&n)) {
            end += 1;
        }
        let rest = lowered.split_off(end);
        out.extend(lowered);
        let placed = placed.into_iter().map(|(i, t)| (i - end, t)).collect();
        let mut inner = Vec::new();
        self.wrap_temps(rest, placed, span, &mut inner)?;
        // What the rest declares is used after it: declared before the `try`.
        let mut body = Vec::new();
        for s in inner {
            let js_span = s.span;
            match s.kind {
                StmtKind::Const(name, value) | StmtKind::Let(name, Some(value)) => {
                    out.push(StmtKind::Let(name.clone(), None).at(js_span));
                    body.push(StmtKind::Assign(Expr::var(&name), value).at(js_span));
                }
                StmtKind::Let(name, None) => out.push(StmtKind::Let(name, None).at(js_span)),
                StmtKind::Destructure { .. } => {
                    return Err(self.unsupported(
                        span,
                        "taking a value apart in a statement with a temporary that has a destructor",
                    ));
                }
                kind => body.push(kind.at(js_span)),
            }
        }
        let mut drop = Vec::new();
        self.drop_owned(Expr::var(&temp.name), temp.ty, &temp.parts, span, &mut drop)?;
        let js_span = self.js_span(span);
        let finally = match temp.flag {
            Some(flag) => vec![StmtKind::If(Expr::var(&flag), drop, None).at(js_span)],
            None => drop,
        };
        // Nothing to leave by, as `x = temporary[0];`: the drops come after.
        let cannot_leave = body
            .iter()
            .all(|s| matches!(&s.kind, StmtKind::Assign(Expr { kind: js::ExprKind::Var(_), .. }, value) if value.reads_same()));
        if body.is_empty() || cannot_leave {
            out.extend(body);
            out.extend(finally);
        } else {
            out.push(StmtKind::Try(body, finally).at(js_span));
        }
        Ok(())
    }

    /// The function being lowered is given a drop for its type parameter
    /// `index`, named `name` (ADR 0098).
    pub(super) fn give_drop_param(&mut self, index: u32, name: String) {
        self.drop_state.types.given.insert(index);
        self.drop_state.param_drops.insert(index, name);
    }

    /// A drop given for a while, a dictionary entry's for its method's own
    /// type parameter (ADR 0163): what was given before, to put back.
    pub(super) fn lend_drop_param(&mut self, index: u32, name: String) -> Option<String> {
        self.drop_state.types.given.insert(index);
        self.drop_state.param_drops.insert(index, name)
    }

    /// Put back what `lend_drop_param` replaced.
    pub(super) fn return_drop_param(&mut self, index: u32, before: Option<String>) {
        if before.is_none() {
            self.drop_state.types.given.remove(&index);
        }
        match before {
            Some(name) => self.drop_state.param_drops.insert(index, name),
            None => self.drop_state.param_drops.remove(&index),
        };
    }

    /// A copied default body's drops, in place of this function's, for its
    /// trait's type parameters, `Self` among them (ADR 0049): each is what the
    /// impl's argument for it drops, with the impl's own drops. What was
    /// there, and what was found with it, is given back by `restore_drops`.
    fn swap_drops(
        &mut self,
        drops: HashMap<u32, String>,
        unsupported: HashMap<u32, (Ty<'tcx>, &'static str)>,
    ) -> SwappedDrops<'tcx> {
        let cache = std::mem::take(&mut *self.drop_state.types.cache.borrow_mut());
        let sizes = std::mem::take(&mut *self.drop_state.sizes.borrow_mut());
        let given = drops.keys().copied().collect();
        SwappedDrops {
            params: std::mem::replace(&mut self.drop_state.param_drops, drops),
            given: std::mem::replace(&mut self.drop_state.types.given, given),
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

    fn restore_drops(&mut self, swapped: SwappedDrops<'tcx>) {
        self.drop_state.param_drops = swapped.params;
        self.drop_state.types.given = swapped.given;
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

    /// The function that drops a `ty`, which a generic function is given for
    /// its type parameter: its `drop` itself, when that's all its drop is,
    /// `noisyDrop_drop`, or an arrow; a type parameter's is the one this
    /// function was given. None for a type with nothing to drop.
    pub(super) fn drop_function(&mut self, ty: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        if let ty::Param(param) = ty.kind() {
            let drop = self
                .drop_state
                .param_drops
                .get(&param.index)
                .map(|name| Expr::var(name));
            if drop.is_some() {
                self.drop_state.used_drops.insert(param.index);
            }
            return Ok(drop);
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
        Ok(Some(Expr::arrow(vec![param.into()], body)))
    }

    /// Note the variables `stmts`, a `let` that can't leave, declare: none
    /// is where a scope can be left (ADR 0301).
    pub(super) fn note_quiet(&mut self, stmts: &[Stmt]) {
        for s in stmts {
            if let StmtKind::Const(name, _) | StmtKind::Let(name, _) = &s.kind {
                self.drop_state.quiet.insert(name.clone());
            }
        }
    }

    /// The drop function for a `ty` that `callee`, which takes only the
    /// drops it uses, is given for its type parameter `index`: an argument
    /// the pipeline keeps where `callee` uses it (ADR 0300). The drops of
    /// this function's that it's made of are passed on, not used, where
    /// this function takes only those it uses too.
    pub(super) fn drop_argument(&mut self, callee: DefId, index: u32, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let before = std::mem::take(&mut self.drop_state.used_drops);
        let drop = self.drop_function(ty, span);
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

    /// A value its scope drops that no variable names, as a `_` parameter.
    pub(super) fn own_value(&mut self, value: Expr, ty: Ty<'tcx>) {
        self.drop_state.owned.push(Owned {
            value,
            ty,
            flag: None,
            parts: Vec::new(),
        });
    }

    /// `value`, a parameter `pat` takes apart, owned by the scope, less
    /// what `pat` moves out of it, which is moved on every path (ADR 0131).
    pub(super) fn own_rest(&mut self, value: Expr, ty: Ty<'tcx>, pat: &Pat<'tcx>) -> R<()> {
        let paths = self
            .pattern_paths(pat)
            .ok_or_else(|| self.unsupported(pat.span, "moving part of a value with a destructor"))?;
        self.drop_state.owned.push(Owned {
            value,
            ty,
            flag: None,
            parts: paths.into_iter().map(|p| (p, None)).collect(),
        });
        Ok(())
    }

    /// How many owners are in scope: where a new scope's start.
    pub(super) fn owned_mark(&self) -> usize {
        self.drop_state.owned.len()
    }

    /// End the scope that began at `mark`: `body`, then the drops of what
    /// it owns, last first, in a `finally`.
    pub(super) fn close_scope(&mut self, mark: usize, body: Vec<Stmt>, span: Span, out: &mut Vec<Stmt>) -> R<()> {
        let mut owned: Vec<Owned<'tcx>> = self.drop_state.owned.drain(mark..).collect();
        let mut body = body;
        // What's moved before anything in the scope can leave, its flag
        // cleared first and never set again, `children$live = false;`, is
        // never the scope's to drop: no flag, and no `try` for it (ADR 0197).
        // A `let` that can't leave may come first (ADR 0301).
        let quiet = |s: &&Stmt| match &s.kind {
            StmtKind::Const(name, _) | StmtKind::Let(name, _) => self.drop_state.quiet.contains(name),
            _ => false,
        };
        let cleared: Vec<String> = body
            .iter()
            .filter(|s| !quiet(s))
            .map_while(|s| match &s.kind {
                StmtKind::Assign(target, value) if matches!(value.kind, js::ExprKind::Bool(false)) => {
                    match &target.kind {
                        js::ExprKind::Var(flag) => Some(flag.clone()),
                        _ => None,
                    }
                }
                _ => None,
            })
            .collect();
        let declared = |out: &[Stmt], flag: &str| {
            out.iter().any(|s| {
                matches!(&s.kind, StmtKind::Let(name, Some(value))
                    if name == flag && matches!(value.kind, js::ExprKind::Bool(true)))
            })
        };
        let never: Vec<String> = owned
            .iter()
            .filter_map(|o| o.flag.as_ref().filter(|_| o.parts.is_empty()))
            .filter(|flag| cleared.contains(flag) && js::mentions_in(&body, flag) == 1 && declared(out, flag))
            .cloned()
            .collect();
        if !never.is_empty() {
            owned.retain(|o| o.flag.as_ref().is_none_or(|flag| !never.contains(flag)));
            let named = |s: &Stmt| match &s.kind {
                StmtKind::Assign(target, _) => matches!(&target.kind, js::ExprKind::Var(flag) if never.contains(flag)),
                StmtKind::Let(flag, _) => never.contains(flag),
                _ => false,
            };
            body.retain(|s| !named(s));
            out.retain(|s| !named(s));
        }
        if owned.is_empty() {
            out.extend(body);
            return Ok(());
        }
        let js_span = self.js_span(span);
        let mut finally = Vec::new();
        for o in owned.into_iter().rev() {
            let mut drop = Vec::new();
            self.drop_owned(o.value, o.ty, &o.parts, span, &mut drop)?;
            match o.flag {
                Some(flag) => finally.push(StmtKind::If(Expr::var(&flag), drop, None).at(js_span)),
                None => finally.extend(drop),
            }
        }
        // Nothing between the declaration and the drops: nothing to leave by.
        if body.is_empty() {
            out.extend(finally);
        } else {
            out.push(StmtKind::Try(body, finally).at(js_span));
        }
        Ok(())
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

    /// Whether computing `e` can't panic, or leave early any other way: a
    /// literal, a variable, and what's built of them. A `let` of one needs
    /// no `try` of its own after one before it: nothing can leave between.
    pub(super) fn cannot_leave(&self, e: ExprId) -> bool {
        super::effects::cannot_leave_in(self.tcx, self.thir, e)
    }

    /// `e` moves a variable that owns a value with a destructor: it's not
    /// owned from here, so its flag is cleared first.
    pub(super) fn moved(&mut self, e: ExprId, out: &mut Vec<Stmt>) -> R<()> {
        let e = self.strip(e);
        let facts = self.drop_facts()?;
        // `*b` of a box: what it holds is moved out, which is all the box owns.
        let e = match self.thir[e].kind {
            ExprKind::Deref { arg } if facts.moves.contains(&self.strip(arg)) => self.strip(arg),
            _ => e,
        };
        let key = std::ptr::from_ref(self.thir) as usize;
        // A part moved out, `pair.a`: its flag.
        let flag = if let Some((var, path)) = facts.part_moves.get(&e) {
            self.drop_state.part_flags.get(&(*var, path.clone())).cloned()
        } else if let ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } = self.thir[e].kind
            && facts.moves.contains(&e)
        {
            self.drop_state.flags.get(&id).cloned()
        } else {
            return Ok(());
        };
        self.drop_state.lowered_moves.insert((key, e));
        if let Some(flag) = &flag {
            let js_span = self.js_span(self.thir[e].span);
            let clear = StmtKind::Assign(Expr::var(flag), Expr::bool(false)).at(js_span);
            match self.drop_state.deferred.get_mut(&(key, e)) {
                Some(slot) => *slot = Some(clear),
                None => out.push(clear),
            }
        }
        Ok(())
    }

    /// Whether `base`, a struct update's, is an owner it moves parts of.
    pub(super) fn update_moves(&mut self, base: ExprId) -> R<bool> {
        let base = self.strip(base);
        Ok(self.drop_facts()?.updates.contains_key(&base))
    }

    /// A struct update, `..base`, once every field's value is made: the
    /// parts it moves out of `base`, an owner, are no longer its own.
    pub(super) fn update_moved(&mut self, base: ExprId, out: &mut Vec<Stmt>) -> R<()> {
        let base = self.strip(base);
        let facts = self.drop_facts()?;
        let Some((var, paths)) = facts.updates.get(&base) else {
            return Ok(());
        };
        let key = std::ptr::from_ref(self.thir) as usize;
        self.drop_state.lowered_moves.insert((key, base));
        let js_span = self.js_span(self.thir[base].span);
        for path in paths {
            if let Some(flag) = self.drop_state.part_flags.get(&(*var, path.clone())) {
                out.push(StmtKind::Assign(Expr::var(flag), Expr::bool(false)).at(js_span));
            }
        }
        Ok(())
    }

    /// Before `list`, a call's operands, is evaluated: the variables it
    /// moves. Rust moves them as the call's made, after all of them are, so
    /// a later one that panics leaves them owned, and dropped.
    pub(super) fn defer_moves(&mut self, list: &[ExprId]) -> R<Vec<ExprId>> {
        let facts = self.drop_facts()?;
        let key = std::ptr::from_ref(self.thir) as usize;
        let moves: Vec<ExprId> = list
            .iter()
            .map(|&e| self.strip(e))
            .filter(|e| {
                facts.moves.contains(e)
                    || facts.part_moves.contains_key(e)
                    || facts.temps.get(e) == Some(&TempKind::Operand)
            })
            .collect();
        for &e in &moves {
            if facts.moves.contains(&e) || facts.part_moves.contains_key(&e) {
                self.drop_state.deferred.insert((key, e), None);
            }
        }
        Ok(moves)
    }

    /// Once `values`, the operands, are evaluated: what has effects goes in
    /// a `const`, in order, then the moves' flags are cleared.
    pub(super) fn end_moves(&mut self, moves: &[ExprId], values: &mut [Expr], out: &mut Vec<Stmt>) {
        let key = std::ptr::from_ref(self.thir) as usize;
        let mut clears: Vec<Stmt> = Vec::new();
        for &e in moves {
            if let Some(clear) = self.drop_state.deferred.remove(&(key, e)).flatten() {
                clears.push(clear);
            } else if let Some(flag) = self.drop_state.temp_flags.get(&(key, e)) {
                // A temporary operand, moved now.
                let js_span = self.js_span(self.thir[e].span);
                clears.push(StmtKind::Assign(Expr::var(flag), Expr::bool(false)).at(js_span));
                self.drop_state.temps_moved.insert((key, e));
            }
        }
        if clears.is_empty() {
            return;
        }
        for value in values.iter_mut() {
            if value.has_effects() {
                let original = std::mem::replace(value, Expr::undefined());
                *value = self.spill("arg", original, out);
            }
        }
        out.extend(clears);
    }

    /// `lhs = rhs` of a value with a destructor: the new value, then the
    /// old one's drop, if it's still owned, then the write, as Rust does it.
    pub(super) fn assign_dropping(&mut self, lhs: ExprId, rhs: ExprId, span: js::Span, out: &mut Vec<Stmt>) -> R<()> {
        let ty = self.thir[lhs].ty;
        let rust_span = self.thir[lhs].span;
        let value = self.expr(rhs, out)?;
        let value = if value.has_effects() {
            self.spill("next", value, out)
        } else {
            value
        };
        let target = match self.place(lhs) {
            Some((target, _)) => target,
            // Dropped, then written: a guard's cell is read twice.
            None => match self.deref_target(lhs, out)?.map(|t| t.kind) {
                Some(js::ExprKind::Member(guard, name)) => {
                    let guard = if guard.reads_same() {
                        *guard
                    } else {
                        self.spill("cell", *guard, out)
                    };
                    Expr::member(guard, &name)
                }
                _ => return Err(self.unsupported(rust_span, "assigning a value with a destructor here")),
            },
        };
        let flag = match self.thir[self.strip(lhs)].kind {
            ExprKind::VarRef { id } => self.drop_state.flags.get(&id).cloned(),
            // A box's value that may have moved, `*b = ..` after `consume(*b)`:
            // the box's own flag, as it was cleared by the move.
            ExprKind::Deref { arg } if self.thir[arg].ty.is_box() => match self.thir[self.strip(arg)].kind {
                ExprKind::VarRef { id } => self.drop_state.flags.get(&id).cloned(),
                _ => None,
            },
            // A part that may have moved, `pair.a = ..` after `consume(pair.a)`.
            _ => self.body_query().place_path(lhs).and_then(|(var, fields)| {
                let path: Path = fields.into_iter().map(|f| (None, f)).collect();
                self.drop_state.part_flags.get(&(var, path)).cloned()
            }),
        };
        let mut drop = Vec::new();
        self.drop_value(target.clone(), ty, rust_span, &mut drop)?;
        match &flag {
            Some(flag) => out.push(StmtKind::If(Expr::var(flag), drop, None).at(span)),
            None => out.extend(drop),
        }
        out.push(StmtKind::Assign(target, value).at(span));
        if let Some(flag) = flag {
            out.push(StmtKind::Assign(Expr::var(&flag), Expr::bool(true)).at(span));
        }
        Ok(())
    }

    /// Whether `block` binds a variable that owns a value with a destructor.
    pub(super) fn block_owns(&mut self, block: BlockId) -> R<bool> {
        let facts = self.drop_facts()?;
        let thir = self.thir;
        Ok(thir[block].stmts.iter().any(|&s| match &thir[s].kind {
            ThirStmt::Let { pattern, .. } => binds_any(pattern, &facts.owners),
            ThirStmt::Expr { .. } => false,
        }))
    }

    /// Once a body is lowered: every owner it has must have had a scope, and
    /// every move its flag cleared. One that didn't went a way this doesn't
    /// know, so it's an error, not JS that forgets a drop.
    pub(super) fn check_drops(&mut self) -> R<()> {
        let facts = self.drop_facts()?;
        let key = std::ptr::from_ref(self.thir) as usize;
        let mut failed = None;
        for (var, span) in &facts.owners {
            if !self.drop_state.registered.contains(var) {
                failed = Some(self.unsupported(*span, "binding a value with a destructor here"));
            }
        }
        for &e in facts
            .moves
            .iter()
            .chain(facts.part_moves.keys())
            .chain(facts.updates.keys())
        {
            if !self.drop_state.lowered_moves.contains(&(key, e)) {
                failed = Some(self.unsupported(self.thir[e].span, "moving a value with a destructor here"));
            }
        }
        match failed {
            Some(guar) => Err(guar),
            None => Ok(()),
        }
    }
}

/// Where a temporary declared before an earlier one's `try`, `let tuple;
/// try { arg = f(b); tuple = [option, arg]; } finally { .. }`, is owned
/// from: that `try`, if nothing after the assignment in it can leave early.
/// Should `f(b)` panic, the temporary was never made, so isn't dropped
/// (ADR 0191).
fn assigned_in_try(lowered: &[Stmt], name: &str) -> Option<usize> {
    let assigns = |s: &Stmt, name: Option<&str>| {
        matches!(&s.kind, StmtKind::Assign(Expr { kind: js::ExprKind::Var(n), .. }, value)
            if name.is_none_or(|name| n == name) && (name.is_some() || value.reads_same()))
    };
    lowered.iter().position(|s| match &s.kind {
        StmtKind::Try(body, _) => body
            .iter()
            .position(|s| assigns(s, Some(name)))
            .is_some_and(|at| body[at + 1..].iter().all(|s| assigns(s, None))),
        _ => false,
    })
}

/// The type parameters a value of `ty` holds, not behind a reference: what
/// it drops of a caller's (ADR 0190).
fn owned_params(ty: Ty<'_>) -> Vec<u32> {
    let mut params = Vec::new();
    let mut walker = ty.walk();
    while let Some(part) = walker.next() {
        let Some(part) = part.as_type() else { continue };
        match part.kind() {
            ty::Ref(..) | ty::RawPtr(..) => walker.skip_current_subtree(),
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
                        tcx.dcx().span_err(
                            check.span,
                            format!("rust-js does not support giving `{path}`'s `{name}` a type with a destructor, where it takes none yet"),
                        );
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
