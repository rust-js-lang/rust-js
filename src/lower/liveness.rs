//! What JS reads, which rustc's dead-code lint can't see (ADR 0440): the
//! fields of a struct of the crate's a program gives a binding, a JS
//! function, which reads them. `StateConfig { doc, extensions }`, given to
//! CodeMirror's `EditorState.create`, has each field read there, as a struct
//! Rust serializes has; rustc would say none is ever read.

use std::sync::OnceLock;

use rustc_hir as hir;
use rustc_hir::intravisit::{self, Visitor};
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
    };
    for owner in tcx.hir_body_owners() {
        given.results = Some(tcx.typeck(owner));
        intravisit::walk_body(&mut given, tcx.hir_body_owned_by(owner));
    }
    let mut fields = Vec::new();
    let mut seen = Vec::new();
    let mut structs = given.structs;
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

/// The structs in what each call of a binding is given.
struct Given<'tcx> {
    tcx: TyCtxt<'tcx>,
    results: Option<&'tcx ty::TypeckResults<'tcx>>,
    structs: Vec<ty::AdtDef<'tcx>>,
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
                self.structs.extend(local_structs(results.expr_ty(arg)));
            }
        }
        intravisit::walk_expr(self, expr);
    }
}
