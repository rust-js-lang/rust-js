//! A parameter's name as JS writes it (ADRs 0678, 0855): the one place
//! it's made, of a function's parameter, its declaration's too, and of a
//! fn pointer's type (ADR 1075).

use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;

use super::{bindings, camel_case};

/// A parameter's name as JS writes it: its `#[rust_js::name = ".."]`,
/// `Context` of React's `useContext` (ADR 0855); else camelCase, without
/// the leading `_` that tells Rust it's unused (ADR 0678). The one place a
/// parameter's JS name is made, the name its declaration says too.
pub(super) fn parameter_name(tcx: TyCtxt<'_>, function: DefId, name: &str) -> String {
    let given = function
        .as_local()
        .and_then(|local| bindings::given_parameter_name(tcx, local, name));
    given.unwrap_or_else(|| default_parameter_name(tcx, function, name))
}

/// A parameter's name a fn pointer's type writes, `fn(inner_write: bool)`'s
/// `innerWrite` (#543): camelCase, without the leading `_` that tells Rust
/// it's unused, as a function's is, where what's left isn't a word JS
/// keeps; no body reads it.
pub(super) fn written_parameter_name(name: &str) -> String {
    let named = camel_case(name);
    match named.strip_prefix('_').filter(|n| !n.is_empty() && !n.starts_with('_')) {
        Some(unmarked) if crate::names::js_ident(unmarked) == unmarked => unmarked.to_string(),
        _ => crate::names::js_ident(&named),
    }
}

/// A parameter's name where it says none (ADR 0678): camelCase, without
/// the leading `_` that tells Rust it's unused, `_ref`'s `ref`; but as Rust
/// writes it where what's left is a word JS keeps, `_new`, another
/// binding's of its function, of a closure in it or around it too, `_x`
/// beside an `x`, or where the body reads it, upstream's `_current`, whose
/// `_` is then the program's name and no mark of one unused (amended).
pub(super) fn default_parameter_name(tcx: TyCtxt<'_>, function: DefId, name: &str) -> String {
    let named = camel_case(name);
    let Some(unmarked) = named.strip_prefix('_').filter(|n| !n.is_empty() && !n.starts_with('_')) else {
        return named;
    };
    if reads_parameter(tcx, function, name) {
        return named;
    }
    // Another crate's body, read from its MIR alone, a library's the crate
    // compiles (ADR 0914) or std's: its bindings, its MIR's.
    let names = match tcx.typeck_root_def_id(function).as_local() {
        Some(root) => binding_names(tcx, root, function),
        None if tcx.is_mir_available(function) => (tcx.optimized_mir(function).var_debug_info.iter())
            .map(|info| info.name)
            .collect(),
        None => return named,
    };
    let taken = || names.iter().any(|other| camel_case(other.as_str()) == unmarked);
    match crate::names::js_ident(unmarked) == unmarked && !taken() {
        true => unmarked.to_string(),
        false => named,
    }
}

/// Whether `function`'s body reads its parameter `name`, a closure in it
/// too (ADR 0678, amended): a path to the parameter's binding, of a body
/// the crate has; of one read from its MIR alone, a use of its local.
fn reads_parameter(tcx: TyCtxt<'_>, function: DefId, name: &str) -> bool {
    use rustc_hir::intravisit::{self, Visitor};
    let Some(local) = function.as_local() else {
        use rustc_middle::mir::{
            self as rmir,
            visit::{MutatingUseContext, PlaceContext, Visitor as _},
        };
        if !tcx.is_mir_available(function) {
            return false;
        }
        let body = tcx.optimized_mir(function);
        let Some(param) = (body.var_debug_info.iter()).find_map(|info| match info.value {
            rmir::VarDebugInfoContents::Place(place)
                if info.name.as_str() == name
                    && place.projection.is_empty()
                    && body.local_kind(place.local) == rmir::LocalKind::Arg =>
            {
                Some(place.local)
            }
            _ => None,
        }) else {
            return false;
        };
        struct Uses(rmir::Local, bool);
        impl<'tcx> rmir::visit::Visitor<'tcx> for Uses {
            fn visit_local(&mut self, local: rmir::Local, context: PlaceContext, _: rmir::Location) {
                // Not its drop, MIR's, which the source never writes.
                let dropped = matches!(context, PlaceContext::MutatingUse(MutatingUseContext::Drop));
                self.1 |= local == self.0 && context.is_use() && !dropped;
            }
        }
        let mut uses = Uses(param, false);
        uses.visit_body(body);
        return uses.1;
    };
    let Some(body) = tcx.hir_maybe_body_owned_by(local) else {
        return false;
    };
    let mut params = Vec::new();
    for param in body.params {
        param.pat.walk_always(|pat| {
            if let rustc_hir::PatKind::Binding(_, id, ident, _) = pat.kind
                && ident.name.as_str() == name
            {
                params.push(id);
            }
        });
    }
    struct Reads<'tcx> {
        tcx: TyCtxt<'tcx>,
        params: Vec<rustc_hir::HirId>,
        read: bool,
    }
    impl<'tcx> Reads<'tcx> {
        /// Whether `expr` is a path to one of the parameters.
        fn names_one(&self, expr: &rustc_hir::Expr<'_>) -> bool {
            matches!(expr.kind, rustc_hir::ExprKind::Path(rustc_hir::QPath::Resolved(None, path))
                if matches!(path.res, rustc_hir::def::Res::Local(id) if self.params.contains(&id)))
        }
    }
    impl<'tcx> Visitor<'tcx> for Reads<'tcx> {
        fn visit_local(&mut self, local: &'tcx rustc_hir::LetStmt<'tcx>) {
            // An `async fn`'s move of its parameter into its future, `let
            // _arg = _arg;`, which no program writes: the binding it makes
            // is the parameter the body reads.
            if let Some(init) = local.init
                && init.span.is_desugaring(rustc_span::DesugaringKind::Async)
                && self.names_one(init)
            {
                local.pat.walk_always(|pat| {
                    if let rustc_hir::PatKind::Binding(_, id, _, _) = pat.kind {
                        self.params.push(id);
                    }
                });
                return;
            }
            intravisit::walk_local(self, local);
        }
        fn visit_expr(&mut self, expr: &'tcx rustc_hir::Expr<'tcx>) {
            if self.names_one(expr) {
                self.read = true;
            }
            if let rustc_hir::ExprKind::Closure(closure) = expr.kind {
                intravisit::walk_body(self, self.tcx.hir_body(closure.body));
            }
            intravisit::walk_expr(self, expr);
        }
    }
    let mut reads = Reads {
        tcx,
        params,
        read: false,
    };
    if !reads.params.is_empty() {
        reads.visit_expr(body.value);
    }
    reads.read
}

/// The name of each binding of `root`'s body that `function`'s parameters
/// could meet: `function`'s own, its closures' among them, and those of
/// each body around it, but not of a closure beside it, whose bindings it
/// never sees nor they its parameters (ADR 0678, amended): `|_instance|
/// false` beside `|instance, _callback| ..` in one static's value.
fn binding_names(tcx: TyCtxt<'_>, root: rustc_span::def_id::LocalDefId, function: DefId) -> Vec<rustc_span::Symbol> {
    use rustc_hir::intravisit::{self, Visitor};
    struct Names<'tcx> {
        tcx: TyCtxt<'tcx>,
        function: DefId,
        /// Whether what's walked is inside `function`'s body.
        inside: bool,
        names: Vec<rustc_span::Symbol>,
    }
    impl<'tcx> Visitor<'tcx> for Names<'tcx> {
        fn visit_pat(&mut self, pat: &'tcx rustc_hir::Pat<'tcx>) {
            if let rustc_hir::PatKind::Binding(_, _, ident, _) = pat.kind {
                self.names.push(ident.name);
            }
            intravisit::walk_pat(self, pat);
        }
        fn visit_expr(&mut self, expr: &'tcx rustc_hir::Expr<'tcx>) {
            if let rustc_hir::ExprKind::Closure(closure) = expr.kind {
                let id = closure.def_id.to_def_id();
                // A closure `function` is, or is in, or one inside it.
                if self.inside || self.tcx.is_descendant_of(self.function, id) {
                    let inside = self.inside || id == self.function;
                    let outer = std::mem::replace(&mut self.inside, inside);
                    intravisit::walk_body(self, self.tcx.hir_body(closure.body));
                    self.inside = outer;
                }
            }
            intravisit::walk_expr(self, expr);
        }
    }
    let mut names = Names {
        tcx,
        function,
        inside: function == root.to_def_id(),
        names: Vec::new(),
    };
    if let Some(body) = tcx.hir_maybe_body_owned_by(root) {
        intravisit::walk_body(&mut names, body);
    }
    names.names
}
