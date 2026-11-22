//! What a function JS gives again what it kept copies of it (#1723, ADR
//! 0020, amended): React renders a component again with the props it
//! kept, twice in `<StrictMode>`, and calls a reducer again with the action
//! it kept (`analysis/given_again.rs`). Rust's function owns what it's
//! given by value, and its next call gets its own; its JS shares JS's
//! object. So where one changes in place what it was given, or a part of it
//! moved out, it copies first: `const at = { ...props.at };`, as a clone
//! would, and a `Vec`'s `slice()`.

use super::super::recursive_fns::Kind;
use super::super::refusal::{Refused, refuse};
use super::super::temporary_names::Base;
use super::super::{FnCx, R, Shape, lower_first};
use rust_js_program::collections::{IndexMap, IndexSet};
use rust_js_program::js;
use rust_js_program::js::{Expr, Prop};
use rustc_middle::mir::{self, Operand, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

/// Where a body's parameters JS gives again are copied (`kept_copies`):
/// at its start, those it changes in place itself; where a part of it is
/// moved out, those a part moved out of is changed in place, or given to
/// the crate's own code, which may.
#[derive(Default)]
pub struct KeptCopies {
    pub(in crate::lower) at_start: IndexSet<mir::Local>,
    pub at_moves: IndexSet<mir::Local>,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `local` of `body` is a parameter JS gives again what it
    /// kept: of a function `given_again` names, given by value, not a
    /// reference, which Rust changes nothing through but a `&mut`, which
    /// React gives none of. A closure's environment, its first, isn't.
    pub fn kept_param(&self, body: &mir::Body<'tcx>, local: mir::Local) -> bool {
        let id = body.source.def_id();
        let first = 1 + usize::from(self.tcx.is_closure_like(id));
        (first..=body.arg_count).contains(&local.as_usize())
            && self.krate.given_again.contains(&id)
            && !(body.local_decls[local].ty.is_ref()
                || crate::lower::pointers::raw_pointee(self.tcx, body.local_decls[local].ty).is_some())
    }

    /// Of `body`'s parameters JS gives again (`kept_param`), those it copies
    /// and where: one written into, `props.at.x = 2`, or lent as a `&mut`,
    /// at its start; one a part moved out of is, followed through each move
    /// and each value made of what's moved, or is given by value to a
    /// function of the crate's own, not JS's, which owns it and may change
    /// it, where it's moved out. A `Copy` value is copied where it's read
    /// (`copy_if_needed`).
    pub fn kept_copies(&self, body: &mir::Body<'tcx>) -> KeptCopies {
        let mut copies = KeptCopies::default();
        let kept: Vec<mir::Local> = body.args_iter().filter(|&l| self.kept_param(body, l)).collect();
        if kept.is_empty() {
            return copies;
        }
        let direct = |p: &mir::Place<'tcx>| !p.projection.iter().any(|e| matches!(e, mir::ProjectionElem::Deref));
        // Whether a place of `of`'s is written into or lent as a `&mut`,
        // or, `given`, moved into a call of the crate's own code.
        let changed = |of: &IndexSet<mir::Local>, given: bool| -> bool {
            body.basic_blocks.iter().any(|data| {
                let written = data.statements.iter().any(|st| match &st.kind {
                    StatementKind::Assign(assign) => {
                        let (dest, rvalue) = &**assign;
                        (of.contains(&dest.local) && !dest.projection.is_empty() && direct(dest))
                            || matches!(rvalue, Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, p) | Rvalue::RawPtr(mir::RawPtrKind::Mut, p)
                                if of.contains(&p.local) && direct(p))
                    }
                    _ => false,
                });
                written
                    || (given
                        && matches!(&data.terminator().kind, TerminatorKind::Call { func, args, .. }
                            if !self.callee_is_js(body, func)
                                && args.iter().any(|a| matches!(&a.node, Operand::Move(p) if of.contains(&p.local) && direct(p)))))
            })
        };
        for local in kept {
            let itself: IndexSet<mir::Local> = [local].into_iter().collect();
            if changed(&itself, false) {
                copies.at_start.insert(local);
                continue;
            }
            // It, and what's moved out of it, or made of what is, in turn.
            let mut held = itself;
            loop {
                let before = held.len();
                for st in body.basic_blocks.iter().flat_map(|data| &data.statements) {
                    let StatementKind::Assign(assign) = &st.kind else {
                        continue;
                    };
                    let (dest, rvalue) = &**assign;
                    let moved =
                        |op: &Operand<'tcx>| matches!(op, Operand::Move(p) if held.contains(&p.local) && direct(p));
                    let from = match rvalue {
                        Rvalue::Use(op, _) => moved(op),
                        Rvalue::Aggregate(_, ops) => ops.iter().any(moved),
                        _ => false,
                    };
                    if from {
                        held.insert(dest.local);
                    }
                }
                if held.len() == before {
                    break;
                }
            }
            if changed(&held, true) {
                copies.at_moves.insert(local);
            }
        }
        copies
    }

    /// The copy of each of `body`'s parameters JS gives again that it
    /// changes in place itself (`kept_copies`' `at_start`), first, `props =
    /// { ...props, at: { ...props.at } }`, by the name `name_of` gives; of
    /// one a pattern takes apart, `({ at })`, each part's, `at = { ...at }`,
    /// `parts` its parts, as the pattern moves each out where it's given.
    pub fn kept_starts(
        &mut self,
        body: &mir::Body<'tcx>,
        parts: &IndexMap<mir::Local, mir::Local>,
        name_of: &dyn Fn(mir::Local) -> js::Name,
    ) -> R<Vec<js::Stmt>> {
        let copies = self.kept_copies(body);
        let copied = |param: &mir::Local| copies.at_start.contains(param) || copies.at_moves.contains(param);
        let parted = (parts.iter()).filter(|(_, param)| copied(param)).map(|(&part, _)| part);
        let mut out = Vec::new();
        for local in copies.at_start.iter().copied().chain(parted) {
            let name = name_of(local);
            let decl = &body.local_decls[local];
            let (ty, span) = (decl.ty, decl.source_info.span);
            let copy = self.kept_copy(Expr::var(&name), ty, span, &mut out)?;
            if !name.is_unnamed() && copy.kind != Expr::var(&name).kind {
                out.push(js::StmtKind::Assign(Expr::var(&name), copy).at(js::Span::NONE));
            }
        }
        Ok(out)
    }

    /// Whether `func` is JS's own function, a binding's, which is given the
    /// program's value as JS gives it (ADR 0020).
    fn callee_is_js(&self, body: &mir::Body<'tcx>, func: &Operand<'tcx>) -> bool {
        matches!(*func.ty(&body.local_decls, self.tcx).kind(), ty::FnDef(id, _) if super::super::bindings::is_binding(self.tcx, id))
    }

    /// Whether what a value of `ty` holds may be changed in place: a type
    /// something changes in place (`contains_mutated`), a `Vec` something
    /// does (`vec_changed`), or one holding either, so a copy JS keeps
    /// apart from it is its own. What a reference or an `Rc` points to
    /// Rust shares too.
    pub(in crate::lower) fn kept_may_change(&self, ty: Ty<'tcx>) -> bool {
        self.kept_may_change_in(ty, &mut IndexSet::default())
    }

    fn kept_may_change_in(&self, ty: Ty<'tcx>, seen: &mut IndexSet<Ty<'tcx>>) -> bool {
        if !seen.insert(ty)
            || ty.is_ref()
            || crate::lower::pointers::raw_pointee(self.tcx, ty).is_some()
            || self.is_rc(ty)
        {
            return false;
        }
        if self.contains_mutated(ty) {
            return true;
        }

        match self.shape(ty) {
            Shape::Object(fields) => fields.iter().any(|&(_, t)| self.kept_may_change_in(t, seen)),
            Shape::Array(tys) => tys.iter().any(|&t| self.kept_may_change_in(t, seen)),
            Shape::Other => match *ty.kind() {
                ty::Array(item, _) | ty::Slice(item) => self.kept_may_change_in(item, seen),
                _ if self.is_vec_like(ty) => {
                    self.vec_changed(ty)
                        || matches!(ty.kind(), ty::Adt(_, args) if self.kept_may_change_in(args.type_at(0), seen))
                }
                _ if let Some(inner) = self.option_of(ty) => self.kept_may_change_in(inner, seen),
                ty::Adt(adt, args) if adt.is_box() => self.kept_may_change_in(args.type_at(0), seen),
                ty::Adt(adt, args) if adt.is_enum() => adt
                    .variants()
                    .iter()
                    .any(|v| (self.variant_fields(v, args).iter()).any(|&(_, t)| self.kept_may_change_in(t, seen))),
                _ => false,
            },
        }
    }

    /// A copy of the `ty` at `place` that shares nothing JS may change in
    /// place with it (`kept_may_change`): a struct's or a tuple's each part
    /// that may, `{ ...props.at, path: props.at.path.slice() }`; a `Vec`'s
    /// `slice()`, or `map` of its items' copies; an `Option`'s `Some`'s; a
    /// `Box`'s value's, which it is; an enum's variant's parts. What it
    /// can't copy, a map JS holds in a form of its own, refused. A type
    /// inside itself, `Row` in its `children: Vec<Row>`, a function of its
    /// module that calls itself for the ones inside, `copyRow(row)`, as its
    /// clone is (`recursive_fns`): expanded in place, it never ended
    /// (#2096). Never its clone, which may run the program's `Clone`.
    pub fn kept_copy(&mut self, place: Expr, ty: Ty<'tcx>, span: Span, out: &mut Vec<js::Stmt>) -> R<Expr> {
        if !self.kept_may_change(ty) {
            return Ok(place);
        }
        // Read once where it's read more than once.
        if !place.reads_same() {
            let param = self.unspelled("value");
            let body = self.kept_copy(Expr::var(&param), ty, span, out)?;
            return Ok(Expr::call(
                Expr::arrow(
                    vec![param.clone().into()],
                    vec![js::StmtKind::Return(Some(body)).at(js::Span::NONE)],
                ),
                vec![place],
            ));
        }
        if self.is_copy(ty) {
            return Ok(self.copy(place, ty));
        }
        if self.is_recursive(ty) {
            let ty::Adt(adt, _) = ty.kind() else {
                unreachable!("a recursive type is an ADT");
            };
            let type_name = self.tcx.item_name(adt.did()).to_string();
            let (base, param) = (format!("copy{type_name}"), lower_first(&type_name));
            let name = self.recursive_fn(
                (Kind::KeptCopy, ty, &base),
                &[Base::Named(&param)],
                |cx, params, body| cx.kept_copy_parts(params[0].clone(), ty, span, body),
                out,
            )?;
            return Ok(Expr::call(Expr::var(&name), vec![place]));
        }
        self.kept_copy_parts(place, ty, span, out)
    }

    /// A `Vec`'s or an array's copy, of items `item` (`kept_copy`): its
    /// `slice()`, or `map` of its items' copies.
    fn kept_copy_items(&mut self, place: Expr, item: Ty<'tcx>, span: Span, out: &mut Vec<js::Stmt>) -> R<Expr> {
        if !self.kept_may_change(item) {
            return Ok(Expr::builtin(place, "slice", Vec::new()));
        }
        let name = self.fresh(Base::Named("item"));
        let body = self.kept_copy(Expr::var(&name), item, span, out)?;
        let copy = Expr::arrow(
            vec![name.clone().into()],
            vec![js::StmtKind::Return(Some(body)).at(js::Span::NONE)],
        );
        Ok(Expr::builtin(place, "map", vec![copy]))
    }

    /// `kept_copy` of `place`, a value read once, made of its parts' copies.
    fn kept_copy_parts(&mut self, place: Expr, ty: Ty<'tcx>, span: Span, out: &mut Vec<js::Stmt>) -> R<Expr> {
        match self.shape(ty) {
            Shape::Object(fields) => {
                let mut props = vec![Prop::Spread(place.clone())];
                let fields = self.object_fields(ty, fields);
                for (name, t, def) in fields {
                    // One held in a place of its own, which changes in
                    // place: a place of its own again (#2255).
                    if def.is_some_and(|def| self.placed_field(def, t)) {
                        let read = self.held(Expr::member(place.clone(), name.clone()), def, t);
                        let part = self.kept_copy(read, t, span, out)?;
                        props.push(Prop::Field(name, self.holding(part, def, t)));
                    } else if self.kept_may_change(t) {
                        props.push(Prop::Field(
                            name.clone(),
                            self.kept_copy(Expr::member(place.clone(), name), t, span, out)?,
                        ));
                    }
                }
                Ok(Expr::object(props))
            }
            Shape::Array(tys) => {
                let mut parts = Vec::new();
                for (i, t) in tys.into_iter().enumerate() {
                    let part = Expr::index(place.clone(), Expr::int(i as i128));
                    let part = self.kept_copy(self.held_part(part, ty, i, t), t, span, out)?;
                    parts.push(self.holding_part(part, ty, i, t));
                }
                Ok(Expr::array(parts))
            }
            Shape::Other => match *ty.kind() {
                ty::Array(item, _) => self.kept_copy_items(place, item, span, out),
                ty::Adt(_, args) if self.is_vec_like(ty) => self.kept_copy_items(place, args.type_at(0), span, out),
                ty::Adt(adt, args) if adt.is_box() => self.kept_copy(place, args.type_at(0), span, out),
                // `Some(x)` is `x` (ADR 0030); of a box, `Some(None)`, the
                // value inside, which this doesn't take apart.
                _ if let Some(inner) = self.option_of(ty)
                    && self.option_of(inner).is_none() =>
                {
                    let none = Expr::bin(js::Op::LooseEq, place.clone(), Expr::null());
                    Ok(Expr::cond(
                        none,
                        place.clone(),
                        self.kept_copy(place, inner, span, out)?,
                    ))
                }
                // `{ TAG: "Line", _0: .. }`: each variant with a part that
                // may change, its copy, every other value itself.
                ty::Adt(adt, args) if adt.is_enum() && self.untagged(ty).is_none() && !self.is_union(ty) => {
                    let copied = |v: &ty::VariantDef, fields: &[(String, Ty<'tcx>)]| {
                        fields.iter().any(|&(_, t)| self.kept_may_change(t)) || self.holds_placed(v, fields)
                    };
                    let groups = self.variant_groups(&adt, args, &copied, &|_, _| false);
                    let mut copies = Vec::new();
                    for crate::lower::copies::VariantGroup { indices, fields, first } in groups.copied {
                        let mut props = vec![Prop::Spread(place.clone())];
                        for ((name, t), def) in fields.into_iter().zip(&first.fields) {
                            if self.placed_field(def, t) {
                                let read = self.held(Expr::member(place.clone(), name.clone()), Some(def), t);
                                let part = self.kept_copy(read, t, span, out)?;
                                props.push(Prop::Field(name, self.holding(part, Some(def), t)));
                            } else if self.kept_may_change(t) {
                                let part = self.kept_copy(Expr::member(place.clone(), name.clone()), t, span, out)?;
                                props.push(Prop::Field(name, part));
                            }
                        }
                        copies.push((indices, Expr::object(props)));
                    }
                    Ok(self.variants_tested((place.clone(), place), (ty, adt), copies, &groups.others))
                }
                _ => Err(refuse(
                    self.tcx,
                    span,
                    Refused::Type(
                        "a value JS gives again, which it keeps, changed in place, a component's props or a reducer's action (#1723; #1727), of type",
                        ty,
                    ),
                )),
            },
        }
    }
}
