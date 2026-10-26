//! What JS reads, which rustc's dead-code lint can't see (ADR 0440): the
//! fields of a struct of the crate's a program gives a binding, a JS
//! function, which reads them. `StateConfig { doc, extensions }`, given to
//! CodeMirror's `EditorState.create`, has each field read there, as a struct
//! Rust serializes has; rustc would say none is ever read. And a class a
//! program hands JS, `js::class::<T>()` (ADR 0761): JS makes it, `new T()`,
//! and reads and calls what its prototype has, so its constructor, its
//! impl's items and its fields are used, and what they use, as rustc's own
//! walk from them would find, which never starts there.

use std::sync::OnceLock;

use rustc_hir as hir;
use rustc_hir::def::Res;
use rustc_hir::intravisit::{self, Visitor};
use rustc_middle::hir::nested_filter;
use rustc_middle::middle::dead_code::DeadCodeLivenessSummary;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_middle::util::Providers;
use rustc_span::ErrorGuaranteed;
use rustc_span::def_id::LocalDefId;

type Liveness = fn(TyCtxt<'_>, ()) -> Result<DeadCodeLivenessSummary, ErrorGuaranteed>;

/// rustc's own liveness, which rust-js's adds to.
static RUSTC_LIVENESS: OnceLock<Liveness> = OnceLock::new();

/// rustc's providers, with liveness that counts what JS reads.
pub(crate) fn provide(providers: &mut Providers) {
    RUSTC_LIVENESS.get_or_init(|| providers.queries.live_symbols_and_ignored_derived_traits);
    providers.queries.live_symbols_and_ignored_derived_traits = live_symbols;
}

fn live_symbols(tcx: TyCtxt<'_>, (): ()) -> Result<DeadCodeLivenessSummary, ErrorGuaranteed> {
    let mut summary = RUSTC_LIVENESS.get().expect("rustc's liveness, kept first")(tcx, ())?;
    let read = read_by_js(tcx);
    summary.pre_deferred_seeding.live_symbols.extend(read.iter().copied());
    summary.final_result.live_symbols.extend(read);
    Ok(summary)
}

/// Each field of a struct of the crate's in what a body gives a binding, as
/// its value's type is before it's coerced, `&FileOptions` given as a `&dyn
/// Any`, and of a struct in one of those fields.
fn read_by_js(tcx: TyCtxt<'_>) -> Vec<LocalDefId> {
    let mut given = Given {
        tcx,
        results: None,
        structs: Vec::new(),
        classes: Vec::new(),
    };
    for owner in tcx.hir_body_owners() {
        given.results = Some(tcx.typeck(owner));
        intravisit::walk_body(&mut given, tcx.hir_body_owned_by(owner));
    }
    let mut fields = used_by_classes(tcx, &given.classes);
    let mut seen = Vec::new();
    let mut structs = given.structs;
    structs.extend(given.classes);
    while let Some(adt) = structs.pop() {
        if seen.contains(&adt) {
            continue;
        }
        seen.push(adt);
        for field in adt.all_fields() {
            let Some(local) = field.did.as_local() else { continue };
            fields.push(local);
            let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
            structs.extend(local_structs(ty));
        }
    }
    fields
}

/// The crate's structs in `ty`, through references, `Option`s and the like.
fn local_structs<'tcx>(ty: Ty<'tcx>) -> impl Iterator<Item = ty::AdtDef<'tcx>> {
    ty.walk().filter_map(|part| match part.as_type()?.kind() {
        ty::Adt(adt, _) if adt.is_struct() && adt.did().is_local() => Some(*adt),
        _ => None,
    })
}

/// The structs in what each call of a binding is given, and the classes
/// `js::class::<T>()` hands JS.
struct Given<'tcx> {
    tcx: TyCtxt<'tcx>,
    results: Option<&'tcx ty::TypeckResults<'tcx>>,
    structs: Vec<ty::AdtDef<'tcx>>,
    classes: Vec<ty::AdtDef<'tcx>>,
}

// Each body on its own, with its own results: a closure's, or a `const`'s
// in a type, is a body owner of its own, not walked into from another's.
impl<'tcx> Visitor<'tcx> for Given<'tcx> {
    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        let results = self.results.expect("a body's results");
        let (callee, args): (Option<_>, Vec<&hir::Expr<'_>>) = match expr.kind {
            hir::ExprKind::Call(callee, args) => match results.expr_ty(callee).kind() {
                ty::FnDef(def_id, _) => (Some(*def_id), args.iter().collect()),
                _ => (None, Vec::new()),
            },
            hir::ExprKind::MethodCall(_, receiver, args, _) => (
                results.type_dependent_def_id(expr.hir_id),
                std::iter::once(receiver).chain(args).collect(),
            ),
            _ => (None, Vec::new()),
        };
        if callee.is_some_and(|callee| super::bindings::is_binding(self.tcx, callee)) {
            for arg in args {
                self.gather(results, arg);
            }
        }
        // `js::class::<T>()` of the crate's class: JS's `T`.
        if let hir::ExprKind::Call(callee, _) = expr.kind
            && let Some((def_id, generic_args)) = super::fn_def(results.expr_ty(callee))
            && super::bindings::is_binding(self.tcx, def_id)
            && matches!(
                super::bindings::js_form(self.tcx, def_id),
                super::bindings::JsForm::Class
            )
            && let Some(ty::Adt(adt, _)) = generic_args.types().next().map(|t| t.kind())
            && adt.did().is_local()
            && super::classes::is_class(self.tcx, adt.did())
        {
            self.classes.push(*adt);
        }
        intravisit::walk_expr(self, expr);
    }
}

impl<'tcx> Given<'tcx> {
    /// The structs in `value`, given a binding, and in what it's made of,
    /// each part's type before it's coerced: `&imports` of `Some(&imports)`,
    /// an `Option<&dyn Any>`.
    fn gather(&mut self, results: &'tcx ty::TypeckResults<'tcx>, value: &'tcx hir::Expr<'tcx>) {
        self.structs.extend(local_structs(results.expr_ty(value)));
        let parts: Vec<&hir::Expr<'_>> = match value.kind {
            hir::ExprKind::Call(callee, args) if is_constructor(results, callee) => args.iter().collect(),
            hir::ExprKind::AddrOf(_, _, inner) => vec![inner],
            hir::ExprKind::Tup(parts) | hir::ExprKind::Array(parts) => parts.iter().collect(),
            hir::ExprKind::Struct(_, fields, _) => fields.iter().map(|field| field.expr).collect(),
            _ => Vec::new(),
        };
        for part in parts {
            self.gather(results, part);
        }
    }
}

/// Whether `callee` is a variant's or a tuple struct's constructor, `Some`.
fn is_constructor(results: &ty::TypeckResults<'_>, callee: &hir::Expr<'_>) -> bool {
    matches!(
        callee.kind,
        hir::ExprKind::Path(ref path) if matches!(results.qpath_res(path, callee.hir_id), hir::def::Res::Def(hir::def::DefKind::Ctor(..), _))
    )
}

/// What JS runs of each of `classes`, which it makes and whose prototype it
/// reads, and what that uses in turn: the class, constructed by its `new`,
/// each item of its impls, and each function, constant, static, struct
/// made, variant and field their bodies name, through the closures they
/// make, as rustc's walk marks them from a body it's given.
fn used_by_classes(tcx: TyCtxt<'_>, classes: &[ty::AdtDef<'_>]) -> Vec<LocalDefId> {
    let mut uses = Uses {
        tcx,
        results: None,
        used: Vec::new(),
        bodies: Vec::new(),
    };
    for adt in classes {
        let Some(local) = adt.did().as_local() else { continue };
        uses.mark(local);
        for &imp in tcx.inherent_impls(adt.did()) {
            for &item in tcx.associated_item_def_ids(imp) {
                if let Some(item) = item.as_local() {
                    uses.mark(item);
                }
            }
        }
    }
    while let Some(owner) = uses.bodies.pop() {
        uses.results = Some(tcx.typeck(owner));
        intravisit::walk_body(&mut uses, tcx.hir_body_owned_by(owner));
    }
    uses.used
}

/// What the bodies of what's used use, each body walked once.
struct Uses<'tcx> {
    tcx: TyCtxt<'tcx>,
    results: Option<&'tcx ty::TypeckResults<'tcx>>,
    used: Vec<LocalDefId>,
    /// Bodies of what's used, to walk.
    bodies: Vec<LocalDefId>,
}

impl Uses<'_> {
    fn mark(&mut self, id: LocalDefId) {
        if self.used.contains(&id) {
            return;
        }
        self.used.push(id);
        if self.tcx.hir_maybe_body_owned_by(id).is_some() {
            self.bodies.push(id);
        }
    }

    /// What a path names: a function, a constant, a static, a struct made,
    /// or a tuple struct's or a variant's constructor, which makes it.
    fn mark_res(&mut self, res: Res) {
        if let Res::Def(_, id) = res
            && let Some(local) = id.as_local()
        {
            self.mark(local);
        }
    }
}

// A closure's body is walked with the body it's in, whose results it shares.
impl<'tcx> Visitor<'tcx> for Uses<'tcx> {
    type NestedFilter = nested_filter::OnlyBodies;

    fn maybe_tcx(&mut self) -> TyCtxt<'tcx> {
        self.tcx
    }

    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        let results = self.results.expect("a body's results");
        match expr.kind {
            hir::ExprKind::Path(ref path) => self.mark_res(results.qpath_res(path, expr.hir_id)),
            hir::ExprKind::Struct(path, ..) => self.mark_res(results.qpath_res(path, expr.hir_id)),
            hir::ExprKind::MethodCall(..) => {
                if let Some(id) = results.type_dependent_def_id(expr.hir_id).and_then(|id| id.as_local()) {
                    self.mark(id);
                }
            }
            hir::ExprKind::Field(base, _) => {
                if let ty::Adt(adt, _) = results.expr_ty_adjusted(base).peel_refs().kind()
                    && let Some(index) = results.opt_field_index(expr.hir_id)
                    && let Some(field) = adt.non_enum_variant().fields.get(index)
                    && let Some(local) = field.did.as_local()
                {
                    self.mark(local);
                }
            }
            _ => {}
        }
        intravisit::walk_expr(self, expr);
    }
}
