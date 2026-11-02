//! What a class's constructor gives (rd554, ADR 0761 amended): `Self`, or
//! its instance shared, `Rc<Self>` or `Arc<Self>`, as React's Flight
//! server's `RequestInstance` gives the `Rc` its constructor hands its root
//! task, `createTask(this, ..)`. An `Rc` the crate counts no count of is
//! the value it points at (ADR 0844), so `new RequestInstance(..)` is the
//! `Rc`; one it counts is `{ value, strong, weak }`, which `new` can't
//! give, so it's refused.

use super::{constructor_of, is_class};
use crate::lower::Body;
use crate::lower::body_queries::strip;
use crate::lower::recognition::{StdItem, is_class_of, is_with_this, js_root_fn, rc_pointee, std_item};
use rustc_hir::def::DefKind;
use rustc_middle::thir::visit::{Visitor, walk_expr};
use rustc_middle::thir::{self, ExprId, ExprKind, LocalVarId, PatKind, StmtKind, Thir};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

/// Whether `output`, what a class's constructor gives, is the class's
/// instance, `own`: `Self`, or an `Rc` or an `Arc` of it.
pub(super) fn gives_instance<'tcx>(tcx: TyCtxt<'tcx>, output: Ty<'tcx>, own: Ty<'tcx>) -> bool {
    output == own || rc_pointee(tcx, output) == Some(own)
}

/// Report each of the crate's classes whose constructor gives an `Rc` of
/// it that `counts` says the crate counts: JS's `new` gives the instance,
/// not the box that counts it. False if there was one.
pub(crate) fn refuse_counted_instances<'tcx>(tcx: TyCtxt<'tcx>, counts: impl Fn(Ty<'tcx>) -> bool) -> bool {
    let mut valid = true;
    for def_id in tcx.hir_crate_items(()).definitions() {
        let id = def_id.to_def_id();
        if tcx.def_kind(id) != DefKind::Struct || !is_class(tcx, id) {
            continue;
        }
        let Some(new) = constructor_of(tcx, id) else { continue };
        let output = tcx
            .fn_sig(new)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder()
            .output();
        if let Some(pointee) = rc_pointee(tcx, output)
            && counts(pointee)
        {
            tcx.dcx().span_err(
                tcx.def_span(new),
                format!(
                    "rust-js: a `#[rust_js::class]` constructor that gives `{output}` gives the instance `new` makes, an `Rc` the crate counts nothing of; this crate counts it, `{{ value, strong, weak }}`, as it reads its counts or its last owner runs a destructor: give `Self`, `fn new(..) -> Self`, and `Rc::new(..)` it where it's made (rd554, ADR 0761)"
                ),
            );
            valid = false;
        }
    }
    valid
}

/// The nullable fields the crate's constructors of its structs each set as
/// they begin, before anything else can read their instance: present,
/// `T | null`, never absent, so a read of one is `null` or a value, given
/// on as it is (rd628). React's Flight client's `ResponseInstance` is
/// `Class::of(function::with_this(..))`, which sets `this._moduleLoading
/// = moduleLoading ?? null;` first; its `prepareDestinationForModule(
/// response._moduleLoading, ..)` reads it as upstream does. A struct the
/// crate makes with a literal writes each such field's `None` `null` (ADR
/// 0772); one the crate's constructor makes is its constructor's; one JS
/// makes, its `.d.ts`, which says so. A field one of a struct's
/// constructors doesn't set first, or that's `#[rust_js::optional]`,
/// absent until it's set (ADR 0810), isn't one.
pub(crate) fn set_where_constructed<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[&Body<'tcx>]) -> HashSet<DefId> {
    let of_id: HashMap<DefId, &Body<'tcx>> = bodies.iter().map(|body| (body.def_id, *body)).collect();
    // Of each struct, the fields every constructor of it sets first.
    let mut set: HashMap<DefId, HashSet<DefId>> = HashMap::new();
    for body in bodies {
        let thir = &body.thir;
        for e in thir.exprs.iter() {
            let ExprKind::Call { fun, ref args, .. } = e.kind else {
                continue;
            };
            let ty::FnDef(did, generic_args) = *thir[strip(thir, fun)].ty.kind() else {
                continue;
            };
            if !is_class_of(tcx, did) {
                continue;
            }
            // `Class<fn(..) -> S>::of`'s `S`, the instance its `new` makes.
            let signature = generic_args.skip_binder().type_at(0);
            if !matches!(signature.kind(), ty::FnPtr(..)) {
                continue;
            }
            let ty::Adt(adt, _) = signature.fn_sig(tcx).output().skip_binder().kind() else {
                continue;
            };
            if !adt.is_struct() || !adt.did().is_local() {
                continue;
            }
            // A constructor that isn't a `with_this` closure sets nothing
            // of its `this`.
            let first = (args.first())
                .and_then(|&f| with_this_closure(tcx, thir, f))
                .and_then(|closure| of_id.get(&closure))
                .map(|closure| set_first(tcx, &closure.thir, closure.expr))
                .unwrap_or_default();
            (set.entry(adt.did()))
                .and_modify(|fields| fields.retain(|field| first.contains(field)))
                .or_insert(first);
        }
    }
    (set.into_values().flatten())
        .filter(|&field| field_def(tcx, field).is_some_and(|def| crate::lower::bindings::null_where_made(tcx, def)))
        .collect()
}

/// The field `field` is, of its struct.
fn field_def(tcx: TyCtxt<'_>, field: DefId) -> Option<&ty::FieldDef> {
    let ty::Adt(adt, _) = tcx
        .type_of(tcx.parent(field))
        .instantiate_identity()
        .skip_normalization()
        .kind()
    else {
        return None;
    };
    adt.non_enum_variant().fields.iter().find(|f| f.did == field)
}

/// The closure `e` gives `function::with_this` of, `with_this(|this, (..)| ..)`.
fn with_this_closure<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, e: ExprId) -> Option<DefId> {
    let ExprKind::Call { fun, .. } = thir[strip(thir, e)].kind else {
        return None;
    };
    let ty::FnDef(did, generic_args) = *thir[strip(thir, fun)].ty.kind() else {
        return None;
    };
    let ty::Closure(closure, _) = *generic_args.skip_binder().types().next()?.kind() else {
        return None;
    };
    is_with_this(tcx, did).then_some(closure)
}

/// The fields of its `this` a `with_this` constructor sets first, each
/// with a cell's `set` among its body's leading statements, before
/// anything is given its `this` or reads the field: `let this: &S =
/// unsafe { js::cast(this) };`, then `this.f.set(value)`, where nothing
/// before reads `this` but a field of it another `set`s or reads. A
/// statement that gives its `this`, as it is or cast, or returns, ends them.
fn set_first<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, root: ExprId) -> HashSet<DefId> {
    let mut first = HashSet::new();
    // `this`, the closure's first parameter, its environment's past.
    let Some(this) = (thir.params.iter())
        .filter_map(|param| param.pat.as_deref())
        .next()
        .and_then(|pat| match pat.kind {
            PatKind::Binding {
                var, subpattern: None, ..
            } => Some(var),
            _ => None,
        })
    else {
        return first;
    };
    let ExprKind::Block { block } = thir[strip(thir, root)].kind else {
        return first;
    };
    let cell = std_item(tcx, StdItem::Cell);
    let mut reads = Reads {
        thir,
        this,
        cast: None,
        fields: HashSet::new(),
        given: false,
    };
    for &stmt in thir[block].stmts.iter() {
        match thir[stmt].kind {
            // `let this: &S = unsafe { js::cast(this) };`
            StmtKind::Let {
                ref pattern,
                initializer: Some(init),
                else_block: None,
                ..
            } if reads.cast.is_none()
                && let PatKind::Binding {
                    var, subpattern: None, ..
                } = pattern.kind
                && cast_of(tcx, thir, init) == Some(this) =>
            {
                reads.cast = Some(var);
            }
            // `this.f.set(value)`
            StmtKind::Expr { expr, .. }
                if let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, expr)].kind
                    && let ty::FnDef(set, _) = *thir[strip(thir, fun)].ty.kind()
                    && tcx.item_name(set).as_str() == "set"
                    && (tcx.opt_parent(set)).is_some_and(|imp| {
                        matches!(tcx.type_of(imp).instantiate_identity().skip_normalization().kind(),
                            ty::Adt(adt, _) if adt.did() == cell)
                    })
                    && let [receiver, value] = args[..]
                    && let Some(field) = reads.field_of(receiver) =>
            {
                reads.visit_expr(&thir[value]);
                if reads.given {
                    break;
                }
                if reads.fields.insert(field) {
                    first.insert(field);
                }
            }
            _ => {
                reads.visit_stmt(&thir[stmt]);
                if reads.given {
                    break;
                }
            }
        }
    }
    first
}

/// The variable `e`, `unsafe { js::cast(this) }`, casts, as it is.
fn cast_of<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, mut e: ExprId) -> Option<LocalVarId> {
    e = strip(thir, e);
    if let ExprKind::Block { block } = thir[e].kind
        && thir[block].stmts.is_empty()
    {
        e = strip(thir, thir[block].expr?);
    }
    let ExprKind::Call { fun, ref args, .. } = thir[e].kind else {
        return None;
    };
    let ty::FnDef(cast, _) = *thir[strip(thir, fun)].ty.kind() else {
        return None;
    };
    if js_root_fn(tcx, cast).is_none_or(|name| name.as_str() != "cast") {
        return None;
    }
    match args[..] {
        [arg] => match thir[strip(thir, arg)].kind {
            ExprKind::VarRef { id } => Some(id),
            _ => None,
        },
        _ => None,
    }
}

/// What a constructor's leading statements read of its `this`: the fields
/// of it they read or set, and whether one gives it, as it is or cast, or
/// returns.
struct Reads<'a, 'tcx> {
    thir: &'a Thir<'tcx>,
    this: LocalVarId,
    cast: Option<LocalVarId>,
    fields: HashSet<DefId>,
    given: bool,
}

impl Reads<'_, '_> {
    /// The field of the cast `this` that `e` is, through a borrow,
    /// `&(*this).f`.
    fn field_of(&self, mut e: ExprId) -> Option<DefId> {
        let thir = self.thir;
        e = strip(thir, e);
        if let ExprKind::Borrow { arg, .. } = thir[e].kind {
            e = strip(thir, arg);
        }
        self.field_in(&thir[e].kind)
    }

    /// The field of the cast `this` that an expression of `kind` is,
    /// `(*this).f`.
    fn field_in(&self, kind: &ExprKind<'_>) -> Option<DefId> {
        let thir = self.thir;
        let ExprKind::Field {
            lhs,
            variant_index,
            name,
        } = *kind
        else {
            return None;
        };
        let ExprKind::Deref { arg } = thir[strip(thir, lhs)].kind else {
            return None;
        };
        let ExprKind::VarRef { id } = thir[strip(thir, arg)].kind else {
            return None;
        };
        let ty::Adt(adt, _) = thir[lhs].ty.kind() else {
            return None;
        };
        (Some(id) == self.cast).then(|| adt.variant(variant_index).fields[name].did)
    }
}

impl<'a, 'tcx> Visitor<'a, 'tcx> for Reads<'a, 'tcx> {
    fn thir(&self) -> &'a Thir<'tcx> {
        self.thir
    }

    fn visit_expr(&mut self, expr: &'a thir::Expr<'tcx>) {
        match expr.kind {
            ExprKind::VarRef { id } if id == self.this || Some(id) == self.cast => self.given = true,
            ExprKind::Return { .. } | ExprKind::Become { .. } | ExprKind::Yield { .. } => self.given = true,
            // What a closure captures is read where it's made.
            ExprKind::Closure(ref closure) => {
                for &upvar in closure.upvars.iter() {
                    self.visit_expr(&self.thir[upvar]);
                }
            }
            _ => {}
        }
        // A field of `this` read or lent, which reads nothing of another.
        if let Some(field) = self.field_in(&expr.kind) {
            self.fields.insert(field);
            return;
        }
        walk_expr(self, expr);
    }
}
