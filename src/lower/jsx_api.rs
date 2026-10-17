//! Element bindings are compiler plumbing. Check resolved uses, including
//! aliases, method references and unused functions, rather than source names.
use super::bindings::is_element_builder;
use rustc_hir::{
    self as hir,
    intravisit::{self, Visitor},
};
use rustc_middle::ty::{self, TyCtxt, TypeckResults};
use rustc_span::def_id::DefId;
use rustc_span::{Span, Symbol};

pub(super) fn validate(tcx: TyCtxt<'_>) -> bool {
    let mut valid = true;
    for owner in tcx.hir_body_owners() {
        let body = tcx.hir_body_owned_by(owner);
        let mut visitor = Uses {
            tcx,
            types: tcx.typeck_body(body.id()),
            permitted: None,
            valid: true,
        };
        visitor.visit_body(body);
        valid &= visitor.valid;
    }
    valid
}

struct Uses<'tcx> {
    tcx: TyCtxt<'tcx>,
    types: &'tcx TypeckResults<'tcx>,
    permitted: Option<hir::HirId>,
    valid: bool,
}

impl Uses<'_> {
    fn check(&mut self, def: DefId, span: Span, permitted: bool) {
        if permitted
            || !matches!(
                self.tcx.def_kind(def),
                hir::def::DefKind::Fn | hir::def::DefKind::AssocFn
            )
        {
            return;
        }
        if super::bindings::is_unexpanded_jsx(self.tcx, def) {
            self.tcx.dcx().span_err(span, "rust-js: `jsx!` is called by the name it's imported as, `use react::jsx;` and `jsx! { .. }`, not by a path to it");
            self.valid = false;
        } else if is_element_builder(self.tcx, def) {
            self.tcx.dcx().span_err(span, "rust-js: element builders are compiler-only; construct elements and set their props inside `jsx! { <Tag ... /> }`");
            self.valid = false;
        }
    }
}

impl Uses<'_> {
    /// A prop `jsx!` gives, written as at `written`: as JSX names it, `js`,
    /// and only so (ADR 0435). What isn't a name, a child or what a
    /// companion fills, isn't written.
    fn spelled(&mut self, written: Span, js: &str) {
        let Ok(text) = self.tcx.sess.source_map().span_to_snippet(written) else {
            return;
        };
        let name = !text.is_empty() && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        if name && text != js {
            self.tcx.dcx().span_err(
                written,
                format!("rust-js: `{text}` is `{js}` in JSX, as React names it"),
            );
            self.valid = false;
        }
    }
}

impl<'tcx> Uses<'tcx> {
    /// `{ #[rust_js::spelled("labelText")] let _ = (); component(C, props) }`:
    /// each name written of `props`' type, its field's JS name, the field
    /// it gives by JSX's rule, `jsx_syntax::snake`, or one flattened in it.
    fn component_props(&mut self, block: &'tcx hir::Block<'tcx>) {
        let [
            hir::Stmt {
                kind: hir::StmtKind::Let(local),
                ..
            },
            ..,
        ] = block.stmts
        else {
            return;
        };
        let path = [Symbol::intern("rust_js"), Symbol::intern("spelled")];
        let Some(attr) = self.tcx.hir_attrs(local.hir_id).iter().find(|a| a.path_matches(&path)) else {
            return;
        };
        // The props given the component, through a `ref` given it.
        let mut call = block.expr;
        while let Some(hir::Expr {
            kind: hir::ExprKind::MethodCall(_, receiver, _, _),
            ..
        }) = call
        {
            call = Some(receiver);
        }
        let Some(hir::Expr {
            kind: hir::ExprKind::Call(_, [_, props]),
            ..
        }) = call
        else {
            return;
        };
        let mut fields = Vec::new();
        let mut types = vec![self.types.expr_ty(props)];
        while let Some(ty) = types.pop() {
            if let ty::Adt(adt, args) = ty.kind()
                && adt.is_struct()
            {
                for field in adt.all_fields() {
                    if super::bindings::is_flatten(self.tcx, field) {
                        types.push(field.ty(self.tcx, args).skip_normalization());
                    }
                    fields.push(field);
                }
            }
        }
        for written in attr.meta_item_list().unwrap_or_default() {
            let Some(lit) = written.lit() else { continue };
            let text = lit.symbol.as_str();
            if fields.iter().any(|f| super::bindings::field_key(self.tcx, f) == text) {
                continue;
            }
            let rust = crate::jsx_syntax::snake(text);
            if let Some(field) = fields.iter().find(|f| f.name.as_str() == rust) {
                let js = super::bindings::field_key(self.tcx, field);
                self.tcx.dcx().span_err(
                    lit.span,
                    format!("rust-js: `{text}` is `{js}` in JSX, as React names it"),
                );
                self.valid = false;
            }
        }
    }
}

impl<'tcx> Visitor<'tcx> for Uses<'tcx> {
    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        let generated = self
            .tcx
            .hir_attrs(expr.hir_id)
            .iter()
            .any(|a| a.path_matches(&[Symbol::intern("rust_js"), Symbol::intern("jsx")]));
        let old = self.permitted;
        match expr.kind {
            hir::ExprKind::Call(callee, _) if generated => self.permitted = Some(callee.hir_id),
            hir::ExprKind::MethodCall(segment, _, _, _) => {
                if let Some(def) = self.types.type_dependent_def_id(expr.hir_id) {
                    self.check(def, expr.span, generated);
                    // An element's attribute, its builder's prop.
                    if generated
                        && let super::bindings::JsForm::Prop(Some(prop)) = super::bindings::js_form(self.tcx, def)
                        && prop != "..."
                    {
                        self.spelled(segment.ident.span, &prop);
                    }
                }
            }
            // A component's props as written, `jsx!`'s record of them.
            hir::ExprKind::Block(block, _) => self.component_props(block),
            hir::ExprKind::Path(ref path) => {
                if let hir::def::Res::Def(_, def) = self.types.qpath_res(path, expr.hir_id) {
                    self.check(def, expr.span, self.permitted == Some(expr.hir_id));
                }
            }
            _ => {}
        }
        intravisit::walk_expr(self, expr);
        self.permitted = old;
    }
}
