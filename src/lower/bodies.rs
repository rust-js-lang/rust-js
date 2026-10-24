//! Function setup, captures and nested-body entry/exit. Context changes live here.

use super::{Body, Enclosing, EnclosingKind, FnCx, ItemScope, Nested, R};
use crate::js::{Stmt, StmtKind};

use rustc_middle::thir::{ExprId, ExprKind, Pat, PatKind};
use rustc_middle::ty;
use rustc_span::Span;
use rustc_span::def_id::DefId;
use std::collections::HashSet;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The directives of a function's or a closure's body, `"use cache";`,
    /// its first statements (ADR 0349); a std body, read from its MIR, has none.
    pub(super) fn directives(&self, owner: DefId) -> Vec<Stmt> {
        let Some(owner) = owner.as_local() else {
            return Vec::new();
        };
        (super::bindings::directives_of(self.tcx, owner).into_iter())
            .filter_map(|(directive, span)| Some(StmtKind::Directive(directive?.to_string()).at(self.js_span(span))))
            .collect()
    }

    // ── Closures (ADR 0022) ─────────────────────────────────────────────

    /// `lower` of `body`, a body inside the one being lowered, entered as
    /// `nested` says and left whatever `lower` gives, an error too: what it
    /// gives, and the drop functions the body declares, for its top. The
    /// one way into a nested body and out of it.
    pub(super) fn lowered_in<T>(
        &mut self,
        body: &'a Body<'tcx>,
        owner: DefId,
        nested: Nested<'tcx>,
        lower: impl FnOnce(&mut Self) -> R<T>,
    ) -> R<(T, Vec<Stmt>)> {
        let enclosing = self.enter_body(body, owner, nested)?;
        let lowered = lower(self);
        let hoisted = self.leave_body(enclosing);
        Ok((lowered?, hoisted))
    }

    /// Start lowering `body`, as `nested` says it starts. Each body is
    /// checked for what it drops (ADR 0098). One that fails to start leaves
    /// the enclosing body's context as it was.
    fn enter_body(&mut self, body: &'a Body<'tcx>, owner: DefId, nested: Nested<'tcx>) -> R<Enclosing<'a, 'tcx>> {
        let frame = super::BodyFrame {
            thir: &body.thir,
            facts: &body.facts,
            owner,
        };
        let frame = std::mem::replace(&mut self.frame, frame);
        let mut default_drops = None;
        let kind = match nested {
            Nested::Closure { names } => EnclosingKind::Closure {
                names: std::mem::replace(&mut self.names, names),
            },
            Nested::Default {
                given,
                typing_env,
                drops,
                unsupported,
                items,
                slots,
            } => {
                default_drops = Some((drops, unsupported, items, slots, given.self_args));
                EnclosingKind::Default(Box::new(ItemScope {
                    names: self.names.clone(),
                    locals: std::mem::take(&mut self.locals),
                    given: self.enter_default(*given),
                    typing_env: std::mem::replace(&mut self.typing_env, typing_env),
                }))
            }
        };
        match self.enter_body_drops(default_drops) {
            Ok(drops) => Ok(Enclosing { frame, drops, kind }),
            Err(refused) => {
                self.restore_enclosing(frame, kind);
                Err(refused)
            }
        }
    }

    /// Finish the body `enter_body` started, and go back to the enclosing one:
    /// the drop functions it declares, for its top.
    fn leave_body(&mut self, enclosing: Enclosing<'a, 'tcx>) -> Vec<Stmt> {
        let hoisted = self.leave_body_drops(enclosing.drops);
        self.restore_enclosing(enclosing.frame, enclosing.kind);
        hoisted
    }

    /// The enclosing body's frame and context back, where `enter_body`
    /// changed them: on leaving the body, or failing to start it.
    fn restore_enclosing(&mut self, frame: super::BodyFrame<'a, 'tcx>, kind: EnclosingKind<'tcx>) {
        self.frame = frame;
        match kind {
            EnclosingKind::Closure { names } => {
                self.names = names;
            }
            EnclosingKind::Default(scope) => {
                let ItemScope {
                    names,
                    locals,
                    given,
                    typing_env,
                } = *scope;
                self.names = names;
                self.locals = locals;
                self.leave_default(given);
                self.typing_env = typing_env;
            }
        }
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether props of `adt` bound whole, `props`, are only passed on as
    /// given, `<Inner {..props} />`, whose component applies its defaults,
    /// or read of a field with no default (ADR 0212).
    /// A props field's default is where JS takes them apart (ADR 0212): a
    /// component's props of type `ty` taken whole, `pat` their pattern, would
    /// have none, but where they're passed on as given or read of a field
    /// with none.
    pub(super) fn check_props_taken_apart(&self, ty: ty::Ty<'tcx>, pat: Option<&Pat<'tcx>>, span: Span) -> R<()> {
        if super::bindings::is_component(self.tcx, self.frame.owner)
            && let ty::Adt(adt, _) = ty.kind()
            && adt.is_struct()
            && (adt.non_enum_variant().fields.iter()).any(|f| super::bindings::field_default(self.tcx, f).is_some())
            && !pat.is_some_and(|p| matches!(p.kind, PatKind::Leaf { .. }))
            && !pat.is_some_and(|p| self.passed_on_whole(p, *adt))
        {
            return Err(self.unsupported(
                span,
                "props with a default taken whole: they're taken apart where they're given, `fn f(Props { size, .. }: Props)`",
            ));
        }
        Ok(())
    }

    fn passed_on_whole(&self, pat: &Pat<'tcx>, adt: ty::AdtDef<'tcx>) -> bool {
        let PatKind::Binding { var, .. } = pat.kind else {
            return false;
        };
        let names = |e: ExprId| matches!(self.frame.thir[e].kind, ExprKind::VarRef { id } if id == var);
        let facts = self.frame.facts;
        let mut allowed: HashSet<ExprId> = facts.jsx_given.union(&facts.jsx_bases).copied().collect();
        for expr in self.frame.thir.exprs.iter() {
            if let ExprKind::Field { lhs, name, .. } = expr.kind
                && names(self.strip(lhs))
                && super::bindings::field_default(self.tcx, &adt.non_enum_variant().fields[name]).is_none()
            {
                allowed.insert(self.strip(lhs));
            }
        }
        self.frame
            .thir
            .exprs
            .iter_enumerated()
            .all(|(e, _)| !names(e) || allowed.contains(&e))
    }
}
