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
    /// its first statements (ADR 0349).
    pub(super) fn directives(&self, owner: rustc_span::def_id::LocalDefId) -> Vec<Stmt> {
        (super::bindings::directives_of(self.tcx, owner).into_iter())
            .filter_map(|(directive, span)| Some(StmtKind::Directive(directive?.to_string()).at(self.js_span(span))))
            .collect()
    }

    // ── Closures (ADR 0022) ─────────────────────────────────────────────

    /// Start lowering `body`, a body inside the one being lowered, as
    /// `nested` says it starts. Each body finds its own stepped iterators
    /// (ADR 0071) and is checked for what it drops (ADR 0098). A body that
    /// fails leaves the state as it is: its whole item fails with it.
    pub(super) fn enter_body(
        &mut self,
        body: &'a Body<'tcx>,
        owner: DefId,
        nested: Nested<'tcx>,
    ) -> R<Enclosing<'a, 'tcx>> {
        let thir = std::mem::replace(&mut self.thir, &body.thir);
        let body_facts = std::mem::replace(&mut self.body_facts, &body.facts);
        let body_owner = std::mem::replace(&mut self.body_owner, owner);
        let stepping = self.enter_body_stepping(&body.facts.stepped, matches!(nested, Nested::Default { .. }));
        let mut default_drops = None;
        // A closure and a copied default are JS functions of their own, which
        // declare their own drop functions.
        let frame = true;
        let kind = match nested {
            Nested::Closure { names } => EnclosingKind::Closure {
                loops: std::mem::take(&mut self.loops),
                names: std::mem::replace(&mut self.names, names),
            },
            Nested::Default {
                evidence,
                self_args,
                typing_env,
                drops,
                unsupported,
            } => {
                default_drops = Some((drops, unsupported));
                EnclosingKind::Default(Box::new(ItemScope {
                    names: self.names.clone(),
                    locals: std::mem::take(&mut self.locals),
                    given: self.enter_default(evidence, self_args),
                    typing_env: std::mem::replace(&mut self.typing_env, typing_env),
                }))
            }
        };
        Ok(Enclosing {
            thir,
            body_facts,
            body_owner,
            stepping,
            drops: self.enter_body_drops(default_drops, frame)?,
            kind,
        })
    }

    /// Finish the body `enter_body` started, and go back to the enclosing one:
    /// the drop functions it declares, for its top.
    /// `from_mir`: a body lowered from its MIR, whose drops are its own
    /// (ADR 0364), which THIR's facts don't check (`check_drops`).
    pub(super) fn leave_body(&mut self, enclosing: Enclosing<'a, 'tcx>, from_mir: bool) -> R<Vec<Stmt>> {
        let hoisted = self.leave_body_drops(enclosing.drops, from_mir)?;
        self.thir = enclosing.thir;
        self.body_facts = enclosing.body_facts;
        self.body_owner = enclosing.body_owner;
        self.leave_body_stepping(enclosing.stepping);
        match enclosing.kind {
            EnclosingKind::Closure { loops, names } => {
                self.loops = loops;
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
        Ok(hoisted)
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
        if super::bindings::is_component(self.tcx, self.body_owner)
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
        let names = |e: ExprId| matches!(self.thir[e].kind, ExprKind::VarRef { id } if id == var);
        let (given, bases) = super::body_queries::jsx_given_props(self.tcx, self.thir);
        let mut allowed: HashSet<ExprId> = given.union(&bases).copied().collect();
        for expr in self.thir.exprs.iter() {
            if let ExprKind::Field { lhs, name, .. } = expr.kind
                && names(self.strip(lhs))
                && super::bindings::field_default(self.tcx, &adt.non_enum_variant().fields[name]).is_none()
            {
                allowed.insert(self.strip(lhs));
            }
        }
        self.thir
            .exprs
            .iter_enumerated()
            .all(|(e, _)| !names(e) || allowed.contains(&e))
    }
}
