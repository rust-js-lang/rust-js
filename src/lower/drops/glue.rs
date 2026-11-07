//! A type's drop glue lowered from rustc's own (#887): the MIR of its
//! `drop_glue::<T>` shim, which gives the parts it drops, their order and
//! what runs as one of them panics. Rustc builds it the way it builds the
//! drop glue a native build runs. The JS is still rust-js's: how each part
//! is reached and dropped.
//!
//! The shim is one level deep: the type's own `Drop::drop`, then a `Drop`
//! of each part rustc drops, in order. Each `Drop` unwinds to a cleanup
//! that drops the parts after it, one after another, and aborts if one of
//! them panics (`unwind terminate`). Each part's own glue is the drop of
//! its type (`drop_in`).

use rustc_middle::mir::{self, BasicBlock, Place, ProjectionElem, TerminatorKind, UnwindAction};

use super::*;

/// The shims built for this function, by the typing environment each is
/// asked in and the type it drops.
pub(super) type Shims<'tcx> = RefCell<IndexMap<(ty::TypingEnv<'tcx>, Ty<'tcx>), Rc<mir::Body<'tcx>>>>;

/// rustc's drop glue of `ty`, `core::ptr::drop_glue::<T>`'s shim, built in
/// `typing_env` so a type parameter's part is a `Drop` of it.
///
/// `rustc_mir_transform::build_drop_shim` is internal to rustc. It is
/// called here and nowhere else, so a toolchain bump that changes it
/// (pinned to 1.99.0 in `rust-toolchain.toml`) breaks this one call.
fn drop_shim<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>) -> mir::Body<'tcx> {
    let glue = tcx.require_lang_item(rustc_hir::attrs::lang_items::LangItem::DropGlue, rustc_span::DUMMY_SP);
    rustc_mir_transform::build_drop_shim(tcx, glue, Some(ty), typing_env)
}

/// What one lowering of a shim knows: the value it drops and its type,
/// and each part's JS, written once even though the shim drops the part
/// on more than one path. A part is a place, or `None` for the type's own
/// `drop`.
struct Glue<'b, 'tcx> {
    body: &'b mir::Body<'tcx>,
    value: Expr,
    ty: Ty<'tcx>,
    span: Span,
    parts: IndexMap<Option<Place<'tcx>>, Vec<Stmt>>,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The drop of `value`, a struct or a tuple, lowered from rustc's drop
    /// glue of its type `ty`: its own `drop`, then each field's, and what
    /// each leaves to run if it panics.
    pub(super) fn drop_glue(
        &mut self,
        value: Expr,
        ty: Ty<'tcx>,
        span: Span,
        made: &mut Made<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let key = (self.typing_env, ty);
        let cached = self.drop_state.shims.borrow().get(&key).cloned();
        let body = match cached {
            Some(body) => body,
            None => {
                let body = Rc::new(drop_shim(self.tcx, self.typing_env, ty));
                self.drop_state.shims.borrow_mut().insert(key, body.clone());
                body
            }
        };
        let mut glue = Glue {
            body: &body,
            value,
            ty,
            span,
            parts: IndexMap::default(),
        };
        let dropped = self.glue_from(&mut glue, mir::START_BLOCK, false, made, out)?;
        out.extend(dropped);
        Ok(())
    }

    /// What the shim's `block`, and the blocks after it, run. In a cleanup,
    /// each part runs as it is, one after another. Otherwise a part that
    /// may throw is followed by the rest: `try { part } catch (error) {
    /// unwinding; throw error; } rest` (`dropped_after_unwinding`).
    fn glue_from(
        &mut self,
        glue: &mut Glue<'_, 'tcx>,
        mut block: BasicBlock,
        cleanup: bool,
        made: &mut Made<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> R<Vec<Stmt>> {
        loop {
            let terminator = glue.body.basic_blocks[block].terminator();
            let (part, target, unwind) = match &terminator.kind {
                TerminatorKind::Goto { target } => {
                    block = *target;
                    continue;
                }
                TerminatorKind::Return | TerminatorKind::UnwindResume | TerminatorKind::UnwindTerminate(_) => {
                    return Ok(Vec::new());
                }
                TerminatorKind::Drop {
                    place, target, unwind, ..
                } => (Some(*place), *target, *unwind),
                TerminatorKind::Call {
                    func,
                    target: Some(target),
                    unwind,
                    ..
                } if func
                    .const_fn_def()
                    .is_some_and(|(callee, _)| self.is_drop_method(callee)) =>
                {
                    (None, *target, *unwind)
                }
                // An enum's variants, by its discriminant, the last one the
                // switch's `otherwise`.
                TerminatorKind::SwitchInt { targets, .. } => {
                    let targets = targets.clone();
                    return self.glue_variants(glue, &targets, cleanup, made, out);
                }
                _ => return Err(self.unsupported(glue.span, &format!("dropping `{}`", glue.ty))),
            };
            let part = self.glue_part(glue, part, made, out)?;
            let rest = self.glue_from(glue, target, cleanup, made, out)?;
            if cleanup || rest.is_empty() || !may_throw(&part) {
                return Ok(part.into_iter().chain(rest).collect());
            }
            let unwinding = match unwind {
                UnwindAction::Cleanup(cleanup) => self.glue_from(glue, cleanup, true, made, out)?,
                _ => Vec::new(),
            };
            return Ok(parts::dropped_after_unwinding(
                part,
                (unwinding, rest),
                self.js_span(glue.span),
            ));
        }
    }

    /// An enum's drop of its variant's fields: an `if` of the test of each
    /// variant that drops any, in their order, only one of which runs, the
    /// test a match writes (`variant_test`). Of one variant, `if (true)`;
    /// of an `Option`, `value != null`.
    fn glue_variants(
        &mut self,
        glue: &mut Glue<'_, 'tcx>,
        targets: &mir::SwitchTargets,
        cleanup: bool,
        made: &mut Made<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> R<Vec<Stmt>> {
        let ty::Adt(adt, _) = *glue.ty.kind() else {
            return Err(self.unsupported(glue.span, &format!("dropping `{}`", glue.ty)));
        };
        let js_span = self.js_span(glue.span);
        let mut arms = Vec::new();
        for (variant, discriminant) in adt.discriminants(self.tcx) {
            let block = (targets.iter())
                .find(|&(value, _)| value == discriminant.val)
                .map_or(targets.otherwise(), |(_, block)| block);
            let dropped = self.glue_from(glue, block, cleanup, made, out)?;
            if dropped.is_empty() {
                continue;
            }
            let value = glue.value.clone();
            let test = if self.option_of(glue.ty).is_some() {
                Expr::bin(js::Op::LooseNe, value, Expr::null())
            } else if adt.variants().len() == 1 {
                Expr::bool(true)
            } else {
                self.variant_test(value, glue.ty, adt, variant, false)
            };
            arms.push(StmtKind::If(test, dropped, None).at(js_span));
        }
        Ok(arms)
    }

    /// Whether `callee` is `Drop::drop`.
    fn is_drop_method(&self, callee: DefId) -> bool {
        (self.tcx.lang_items().drop_trait()).is_some_and(|drop| self.tcx.associated_item_def_ids(drop)[0] == callee)
    }

    /// The JS of one of the shim's parts, `None` its type's own `drop`,
    /// written the first time it's asked for.
    fn glue_part(
        &mut self,
        glue: &mut Glue<'_, 'tcx>,
        part: Option<Place<'tcx>>,
        made: &mut Made<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> R<Vec<Stmt>> {
        if let Some(known) = glue.parts.get(&part) {
            return Ok(known.clone());
        }
        let (value, ty, span) = (glue.value.clone(), glue.ty, glue.span);
        let js_span = self.js_span(span);
        let mut dropped = Vec::new();
        match part {
            None => {
                if let Some(call) = self.own_drop(value, ty, span, out)? {
                    dropped.push(StmtKind::Expr(call).at(js_span));
                }
            }
            Some(place) => {
                let (part, field_ty) = match (place.projection.as_slice(), ty.kind()) {
                    ([ProjectionElem::Deref, ProjectionElem::Field(field, _)], ty::Adt(adt, args)) => (
                        self.project(value.clone(), ty, field.as_usize()),
                        self.field_ty(&adt.non_enum_variant().fields[*field], args),
                    ),
                    ([ProjectionElem::Deref, ProjectionElem::Field(field, _)], ty::Tuple(items)) => (
                        self.project(value.clone(), ty, field.as_usize()),
                        items[field.as_usize()],
                    ),
                    // An `Option`'s value: a `Some` of what may look like
                    // `None`, a generic `T` say, is boxed (ADR 0051).
                    (
                        [
                            ProjectionElem::Deref,
                            ProjectionElem::Downcast(..),
                            ProjectionElem::Field(..),
                        ],
                        _,
                    ) if let Some(inner) = self.option_of(ty) => {
                        let payload = match self.boxed_payload(inner) {
                            true => self.some_value(value.clone()),
                            false => value.clone(),
                        };
                        (payload, inner)
                    }
                    (
                        [
                            ProjectionElem::Deref,
                            ProjectionElem::Downcast(_, variant),
                            ProjectionElem::Field(field, _),
                        ],
                        ty::Adt(adt, args),
                    ) => {
                        let def = adt.variant(*variant);
                        // An untagged enum's variant is its payload (ADR 0214).
                        let part = match self.untagged(ty).is_some() {
                            true => value.clone(),
                            false => Expr::member(value.clone(), variant_field(self.tcx, def, field.as_usize())),
                        };
                        (part, self.field_ty(&def.fields[*field], args))
                    }
                    _ => return Err(self.unsupported(span, &format!("dropping `{ty}`"))),
                };
                match field_ty.kind() {
                    // A `dyn` last field: through the `impl` its struct
                    // was given (ADRs 0409, 0436).
                    ty::Dynamic(..) => {
                        let dictionary = Expr::member(value, js::DICTIONARY);
                        dropped.push(dyn_dropped(dictionary, part, js_span));
                    }
                    _ => self.drop_in(part, field_ty, span, made, &mut dropped)?,
                }
            }
        }
        glue.parts.insert(part, dropped.clone());
        Ok(dropped)
    }
}
