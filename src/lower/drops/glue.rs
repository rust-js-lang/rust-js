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
/// asked in and the type it drops: one function's types, dropped with it.
pub(super) type Shims<'tcx> = RefCell<IndexMap<(ty::TypingEnv<'tcx>, Ty<'tcx>), Rc<mir::Body<'tcx>>>>;

/// Where a part a shim drops is, in the value it drops.
#[derive(Clone, Copy)]
pub(super) enum Part {
    /// A struct's or a tuple's field, by its index.
    Field(usize),
    /// An `Option`'s value.
    OptionValue,
    /// A variant's field, by its index.
    Variant(rustc_abi::VariantIdx, usize),
}

impl<'a, 'tcx> DropQuery<'a, 'tcx> {
    /// rustc's drop glue of `ty`, built once for each type.
    fn shim(&self, ty: Ty<'tcx>) -> Rc<mir::Body<'tcx>> {
        let typing_env = self.recognition.typing_env;
        let key = (typing_env, ty);
        if let Some(body) = self.state.shims.borrow().get(&key) {
            return body.clone();
        }
        let body = Rc::new(drop_shim(self.recognition.tcx, typing_env, ty));
        self.state.shims.borrow_mut().insert(key, body.clone());
        body
    }

    /// Which part of a `ty` a `place` of its shim is, and its type: a
    /// field's, as its type's fields are given their arguments
    /// (`field_ty`), an `Option`'s value, or a variant's field. `None` of
    /// a place of no other shape.
    pub(super) fn glue_place(&self, ty: Ty<'tcx>, place: Place<'tcx>) -> Option<(Part, Ty<'tcx>)> {
        let recognition = self.recognition;
        match (place.projection.as_slice(), ty.kind()) {
            ([ProjectionElem::Deref, ProjectionElem::Field(field, _)], ty::Adt(adt, args)) => Some((
                Part::Field(field.as_usize()),
                recognition.field_ty(&adt.non_enum_variant().fields[*field], args),
            )),
            ([ProjectionElem::Deref, ProjectionElem::Field(field, _)], ty::Tuple(items)) => {
                Some((Part::Field(field.as_usize()), items[field.as_usize()]))
            }
            (
                [
                    ProjectionElem::Deref,
                    ProjectionElem::Downcast(..),
                    ProjectionElem::Field(..),
                ],
                _,
            ) if let Some(inner) = recognition.option_of(ty) => Some((Part::OptionValue, inner)),
            (
                [
                    ProjectionElem::Deref,
                    ProjectionElem::Downcast(_, variant),
                    ProjectionElem::Field(field, _),
                ],
                ty::Adt(adt, args),
            ) => Some((
                Part::Variant(*variant, field.as_usize()),
                recognition.field_ty(&adt.variant(*variant).fields[*field], args),
            )),
            _ => None,
        }
    }

    /// The types of the parts rustc's drop glue of `ty`, a struct, an enum,
    /// a union or a tuple, drops, each once, in order, every variant's:
    /// what dropping it drops beside its own `drop`, the one list of them
    /// each question of its drop asks. `None` of a part of a place of no
    /// other shape.
    pub(super) fn glue_parts(&self, ty: Ty<'tcx>) -> Option<Vec<Ty<'tcx>>> {
        let body = self.shim(ty);
        let mut places = IndexSet::default();
        for data in body.basic_blocks.iter().filter(|data| !data.is_cleanup) {
            if let TerminatorKind::Drop { place, .. } = data.terminator().kind {
                places.insert(place);
            }
        }
        (places.into_iter())
            .map(|place| self.glue_place(ty, place).map(|(_, part)| part))
            .collect()
    }
}

/// rustc's drop glue of `ty`, `core::ptr::drop_glue::<T>`'s shim, built in
/// `typing_env` so a type parameter's part is a `Drop` of it.
///
/// `rustc_mir_transform::build_drop_shim` is internal to rustc. It is
/// called here and nowhere else, so a toolchain bump that changes it
/// (pinned to 1.99.0 in `rust-toolchain.toml`) breaks this one call.
fn drop_shim<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>) -> mir::Body<'tcx> {
    let glue = tcx.require_lang_item(rustc_hir::attrs::lang_items::LangItem::DropGlue, rustc_span::DUMMY_SP);
    // Its elaboration asks for an environment after analysis, as codegen's
    // is: what's asked before lowering has an analysis's.
    let typing_env = typing_env.with_post_analysis_normalized(tcx);
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
    /// The drop of `value`, a struct, an enum, a union or a tuple, lowered
    /// from rustc's drop glue of its type `ty`: its own `drop`, then each
    /// field's, of an enum its variant's, and what each leaves to run if it
    /// panics.
    pub(super) fn drop_glue(
        &mut self,
        value: Expr,
        ty: Ty<'tcx>,
        span: Span,
        made: &mut Made<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let body = self.drop_query().shim(ty);
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
            let js_span = self.js_span(glue.span);
            return Ok(parts::dropped_after_unwinding(
                part,
                (unwinding, rest),
                js_span,
                &mut self.catch_names(),
            ));
        }
    }

    /// An enum's drop of its variant's fields: an `if` of the test of each
    /// variant that drops any, in their order, only one of which runs, the
    /// test a match writes (`variant_test`), an `Option`'s by what its
    /// `None` can be (`Expr::presence`). Of one variant, `if (true)`.
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
            let test = if adt.variants().len() == 1 {
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
                let Some((at, field_ty)) = self.drop_query().glue_place(ty, place) else {
                    return Err(self.unsupported(span, &format!("dropping `{ty}`")));
                };
                let part = match at {
                    Part::Field(field) => self.project(value.clone(), ty, field),
                    // A `Some` of what may look like `None`, a generic `T`
                    // say, is boxed (ADR 0051).
                    Part::OptionValue => match self.boxed_payload(field_ty) {
                        true => self.some_value(value.clone()),
                        false => value.clone(),
                    },
                    // An untagged enum's variant is its payload (ADR 0214).
                    Part::Variant(_, _) if self.untagged(ty).is_some() => value.clone(),
                    Part::Variant(variant, field) => {
                        let ty::Adt(adt, _) = ty.kind() else {
                            unreachable!("a variant's field")
                        };
                        Expr::member(value.clone(), variant_field(self.tcx, adt.variant(variant), field))
                    }
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
