//! What `#[rust_js::name = ".."]` names of a body's bindings, its
//! parameters' (ADR 0855) and its `let`s' (ADR 0855, amended): read from
//! the body's HIR, as rustc keeps their attributes nowhere else.

use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::{DefId, LocalDefId};
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

/// What a `use .. as` renames `def_id` to, `alias`, as JS writes it: as
/// the item's `#[rust_js::name]` is cased, where it has one, `use
/// scheduler::IDLE_PRIORITY as IDLE_SCHEDULER_PRIORITY` of `IdlePriority`
/// imports `IdleSchedulerPriority`, upstream's (React port gap rd79); else
/// as its own name is (`cased_as`, ADR 0202, amended).
pub(in crate::lower) fn alias_cased(tcx: TyCtxt<'_>, def_id: DefId, alias: &str) -> String {
    (super::given_name(tcx, def_id).and_then(|given| cased_like(&given, alias)))
        .unwrap_or_else(|| super::cased_as(tcx, def_id, alias))
}

/// `name` cased as `given`, an item's own JS name, is: camelCase of a
/// camelCase one, `getCurrentPriorityLevel`, PascalCase of a PascalCase
/// one, `IdlePriority`, and as it is of a SCREAMING one, `REACT_ELEMENT`.
/// None of one of no case of these, `$$typeof`.
fn cased_like(given: &str, name: &str) -> Option<String> {
    let first = given.chars().next()?;
    let word = given.chars().all(|c| c.is_ascii_alphanumeric());
    if given
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    {
        return Some(name.to_string());
    }
    // Its words, of a `SCREAMING` alias or a `snake_case` one.
    let camel = crate::lower::variable_case(name);
    match (word, first.is_ascii_lowercase(), first.is_ascii_uppercase()) {
        (true, true, _) => Some(camel),
        (true, _, true) => Some(crate::lower::upper_first(&camel)),
        _ => None,
    }
}
