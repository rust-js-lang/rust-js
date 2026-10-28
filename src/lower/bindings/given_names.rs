//! What `#[rust_js::name = ".."]` names of a body's bindings, its
//! parameters' (ADR 0855) and its `let`s' (ADR 0855, amended): read from
//! the body's HIR, as rustc keeps their attributes nowhere else.

use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::LocalDefId;
use rustc_span::{Symbol, sym};

/// Each of `owner`'s parameters' `#[rust_js::name = ".."]`, its JS name, React's
/// `useContext(Context)` (ADR 0855), read from its body as `marked_rest` is.
pub(in crate::lower) fn named_parameters<'tcx>(
    tcx: TyCtxt<'tcx>,
    owner: LocalDefId,
) -> impl Iterator<Item = (&'tcx rustc_hir::Param<'tcx>, &'tcx rustc_hir::Attribute)> {
    let path = [Symbol::intern("rust_js"), sym::name];
    let params = tcx.hir_maybe_body_owned_by(owner).map_or(&[][..], |body| body.params);
    let attr =
        move |param: &'tcx rustc_hir::Param<'tcx>| tcx.hir_attrs(param.hir_id).iter().find(|a| a.path_matches(&path));
    params.iter().filter_map(move |param| Some((param, attr(param)?)))
}

/// The JS name `#[rust_js::name = ".."]` gives `owner`'s parameter that
/// binds `name`, if it gives one (ADR 0855).
pub(in crate::lower) fn given_parameter_name(tcx: TyCtxt<'_>, owner: LocalDefId, name: &str) -> Option<String> {
    named_parameters(tcx, owner).find_map(|(param, attr)| match param.pat.kind {
        rustc_hir::PatKind::Binding(_, _, ident, None) if ident.name.as_str() == name => {
            attr.value_str().map(|given| given.to_string())
        }
        _ => None,
    })
}

/// Each `let` of `owner`'s body, not of a closure in it, whose own body it
/// is, with its `#[rust_js::name = ".."]`: React's `$$typeof`, a name Rust
/// can't spell (ADR 0855, amended). Read from the body, as a parameter's is.
pub(in crate::lower) fn named_lets<'tcx>(
    tcx: TyCtxt<'tcx>,
    owner: LocalDefId,
) -> Vec<(&'tcx rustc_hir::LetStmt<'tcx>, &'tcx rustc_hir::Attribute)> {
    use rustc_hir::intravisit::{self, Visitor};
    struct Lets<'tcx> {
        tcx: TyCtxt<'tcx>,
        found: Vec<(&'tcx rustc_hir::LetStmt<'tcx>, &'tcx rustc_hir::Attribute)>,
    }
    impl<'tcx> Visitor<'tcx> for Lets<'tcx> {
        fn visit_local(&mut self, local: &'tcx rustc_hir::LetStmt<'tcx>) {
            let path = [Symbol::intern("rust_js"), sym::name];
            if let Some(attr) = self.tcx.hir_attrs(local.hir_id).iter().find(|a| a.path_matches(&path)) {
                self.found.push((local, attr));
            }
            intravisit::walk_local(self, local);
        }
    }
    let mut lets = Lets { tcx, found: Vec::new() };
    if let Some(body) = tcx.hir_maybe_body_owned_by(owner) {
        lets.visit_body(body);
    }
    lets.found
}

/// The JS name `#[rust_js::name = ".."]` gives each of `owner`'s `let`s that
/// binds a name, by its binding's span, which its MIR's debug info has.
pub(in crate::lower) fn given_let_names(tcx: TyCtxt<'_>, owner: LocalDefId) -> Vec<(rustc_span::Span, String)> {
    (named_lets(tcx, owner).into_iter())
        .filter_map(|(local, attr)| match local.pat.kind {
            rustc_hir::PatKind::Binding(_, _, _, None) => Some((local.pat.span, attr.value_str()?.to_string())),
            _ => None,
        })
        .collect()
}
