//! A struct, a variant or a tuple struct made, by its fields or a struct
//! update, `..base` (ADRs 0013, 0033, 0098), and a constructor taken as a
//! value (ADR 0125).

use super::bindings;
use super::recognition::{StdItem, is_std_def, std_item};
use super::representation::ordering_value;
use super::{Dest, FnCx, R, Shape, assembled};
use crate::js::{self, Expr, Prop, Stmt, StmtKind};
use crate::runtime::Helper;
use rustc_hir::def::CtorKind;
use rustc_middle::thir::{self as thir, AdtExprBase, ExprId};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;
use std::collections::HashMap;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A struct literal: `{ x: 1, y: 2 }`, or `[1, 2]` for a tuple struct.
    pub(super) fn adt(&mut self, adt: &thir::AdtExpr<'tcx>, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        let variant = adt.adt_def.variant(adt.variant_index);
        // A `fmt::Result`'s `Ok` is nothing (ADR 0054), and so is an
        // `io::Result<()>`'s (ADR 0132). A `fmt::Error` is thrown, for the
        // writers it passes out of and what they wrote (ADR 0187).
        if self.is_fmt_result(ty) || self.recognition().is_io_unit_result(ty) {
            return match variant.name.as_str() {
                "Ok" => Ok(Expr::undefined()),
                // A writer of the crate's is given each `write!`'s text whole (ADR
                // 0166), where Rust gives it a piece at a time: one that fails
                // would fail where Rust's doesn't.
                _ if self.is_fmt_result(ty)
                    && self
                        .tcx
                        .trait_impl_of_assoc(self.item)
                        .and_then(|imp| self.tcx.impl_opt_trait_id(imp))
                        .is_some_and(|tr| is_std_def(self.tcx, tr, StdItem::FmtWrite)) =>
                {
                    Err(self.unsupported(span, "a `fmt::Error` of a writer of the crate's"))
                }
                _ if self.is_fmt_result(ty) => {
                    for field in &adt.fields {
                        self.stmt(field.expr, &Dest::Discard, out)?;
                    }
                    self.runtime.insert(Helper::FmtError);
                    Ok(Expr::call(Expr::var("$fmtError"), Vec::new()))
                }
                _ => Err(self.unsupported(span, "an `io::Error`")),
            };
        }
        // `Some(x)` is `x`, and `None` is `undefined` (ADR 0030).
        if let Some(inner) = self.option_of(ty) {
            return match adt.fields.first() {
                // Of what might look like `None`, a generic `T` or an `Option` (ADR 0051).
                Some(field) if self.boxed_payload(inner) => {
                    let value = self.expr(field.expr, out)?;
                    Ok(self.some(value))
                }
                Some(field) => self.expr(field.expr, out),
                None => Ok(Expr::undefined()),
            };
        }
        // A variant without fields is its name (ADR 0013). One with fields is an
        // object tagged with it, `{ TAG: "Circle", _0: r }` (ADR 0033), built
        // below like a struct.
        if let Some(n) = ordering_value(self.tcx, adt.adt_def.did(), variant.name) {
            return Ok(Expr::int(n));
        }
        if adt.adt_def.is_enum() && variant.fields.is_empty() {
            return Ok(Expr::str(bindings::variant_name(self.tcx, variant)));
        }
        if adt.adt_def.is_union() {
            return Err(self.unsupported(span, "unions"));
        }
        // `struct Marker;` holds nothing, like `()`.
        if variant.ctor_kind() == Some(CtorKind::Const) {
            return Ok(Expr::undefined());
        }
        // `P { x, ..base }`: the fields not written come from `base`.
        // Keep the saved fields local: lowering the base can itself lower
        // another struct literal or update.
        let mut spilled_fields = None;
        let mut defaults = None;
        let base = match &adt.base {
            AdtExprBase::None => None,
            AdtExprBase::Base(fru) => match self.place(fru.base) {
                Some((place, _)) => Some(place),
                // `..Default::default()`: worked out once, after the fields,
                // as Rust does, so any field with effects runs first.
                None => {
                    let exprs: Vec<ExprId> = adt.fields.iter().map(|f| f.expr).collect();
                    let values = self.operands(&exprs, out)?;
                    let mut spilled = Vec::new();
                    for (field, value) in adt.fields.iter().zip(values) {
                        let value = if value.has_effects() {
                            self.spill(variant.fields[field.name].name.as_str(), value, out)
                        } else {
                            value
                        };
                        spilled.push(value);
                    }
                    spilled_fields = Some(spilled);
                    let given: Vec<usize> = adt.fields.iter().map(|f| f.name.as_usize()).collect();
                    let base = match self.derived_default_fields(fru.base, ty) {
                        Some(fields) => {
                            let (props, kept) = self.update_default(fields, &given, span)?;
                            // Each default the update reads is made in its place,
                            // in the order the base would make them, unless one
                            // it doesn't read is made for its effects, a
                            // hand-written one's: then the base is made whole.
                            if !kept {
                                defaults = Some(props.into_iter().map(|(_, value)| value).collect::<Vec<_>>());
                                None
                            } else {
                                Some(Expr::object(
                                    props
                                        .into_iter()
                                        .map(|(name, value)| Prop::Field(name, value))
                                        .collect(),
                                ))
                            }
                        }
                        None => Some(self.expr(fru.base, out)?),
                    };
                    base.map(|base| {
                        // An object of constants is read in place: its fields
                        // are those constants (`Expr::member`).
                        let constants = matches!(&base.kind, js::ExprKind::Object(props)
                            if props.iter().all(|p| matches!(p, Prop::Field(_, v) if v.is_constant())));
                        if base.reads_same() || constants {
                            base
                        } else {
                            self.spill("base", base, out)
                        }
                    })
                }
            },
            AdtExprBase::DefaultFields(_) => return Err(self.unsupported(span, "default field values")),
        };

        // Rust evaluates the fields in the order they're written. JS lists
        // them in declaration order, so every object of a type has the same
        // shape. If that reorders two calls, they go into `const`s first.
        let exprs: Vec<ExprId> = adt.fields.iter().map(|f| f.expr).collect();
        let mut values = match spilled_fields {
            Some(values) => values,
            None => self.operands(&exprs, out)?,
        };
        let reordered = !adt.fields.is_sorted_by_key(|f| f.name);
        if reordered && values.iter().filter(|v| v.has_effects()).count() > 1 {
            for (field, value) in adt.fields.iter().zip(&mut values) {
                self.made_first(variant.fields[field.name].name.as_str(), value, out);
            }
        }
        // `..p` moves the fields it doesn't name out of `p` once every
        // value is made, as Rust makes the struct (ADR 0098).
        if let AdtExprBase::Base(fru) = &adt.base
            && self.update_moves(fru.base)?
        {
            for (field, value) in adt.fields.iter().zip(&mut values) {
                if value.has_effects() {
                    let v = std::mem::replace(value, Expr::undefined());
                    *value = self.spill(variant.fields[field.name].name.as_str(), v, out);
                }
            }
            self.update_moved(fru.base, out)?;
        }
        let mut given: HashMap<usize, Expr> = adt.fields.iter().map(|f| f.name.as_usize()).zip(values).collect();

        let tag = adt.adt_def.is_enum().then(|| bindings::variant_name(self.tcx, variant));
        let shape = match tag {
            Some(_) => Shape::Object(self.variant_fields(variant, adt.args)),
            None => self.shape(ty),
        };
        let field_tys = match &shape {
            Shape::Object(fields) => fields.iter().map(|&(_, t)| t).collect(),
            Shape::Array(tys) => tys.clone(),
            Shape::Other => unreachable!("a struct with fields"),
        };
        let mut items = Vec::new();
        for (i, field_ty) in field_tys.into_iter().enumerate() {
            items.push(match (given.remove(&i), &base) {
                (Some(value), _) => value,
                (None, None) if let Some(defaults) = &mut defaults => {
                    std::mem::replace(&mut defaults[i], Expr::undefined())
                }
                (None, Some(base)) => self.copy_if_needed(self.project(base.clone(), ty, i), field_ty),
                (None, None) => unreachable!("rustc checked that every field is given"),
            });
        }
        Ok(assembled(shape, tag, items))
    }

    /// `value`, if it does something, made first, in a `const` of its own;
    /// an object made here each of its values, so it stays one, as a
    /// flattened struct's is taken apart where it's given (ADR 0213).
    fn made_first(&mut self, name: &str, value: &mut Expr, out: &mut Vec<Stmt>) {
        if let js::ExprKind::Object(props) = &mut value.kind {
            for prop in props {
                let (Prop::Field(key, value) | Prop::Getter(key, value)) = prop else {
                    continue;
                };
                let key = key.clone();
                self.made_first(&key, value, out);
            }
        } else if value.has_effects() {
            let fresh = self.fresh(name);
            let made = std::mem::replace(value, Expr::var(&fresh));
            let span = made.span;
            out.push(StmtKind::Const(fresh, made).at(span));
        }
    }

    /// The fields of `ty`, a struct whose `Default` is derived, when `base`
    /// is its `Default::default()`: what an update's base is made of.
    fn derived_default_fields(&self, base: ExprId, ty: Ty<'tcx>) -> Option<Vec<(String, Ty<'tcx>)>> {
        let thir::ExprKind::Call { fun, args, .. } = &self.thir[super::body_queries::strip(self.thir, base)].kind
        else {
            return None;
        };
        let default = std_item(self.tcx, StdItem::Default);
        let ty::FnDef(id, _) = *self.thir[*fun].ty.kind() else {
            return None;
        };
        let ty::Adt(adt, _) = ty.kind() else { return None };
        if !args.is_empty()
            || self.tcx.trait_of_assoc(id) != Some(default)
            || !adt.is_struct()
            || self.is_std(adt.did())
            || self.has_user_impl(default, ty)
        {
            return None;
        }
        match self.shape(ty) {
            Shape::Object(fields) => Some(fields),
            _ => None,
        }
    }

    /// An update's derived `Default` base, `..Default::default()`, of
    /// `fields`: each one's default, but none for a field the update gives
    /// where making it does nothing, as a `Node`'s does (ADR 0201). And
    /// whether one the update gives is made still, for its effects.
    fn update_default(
        &mut self,
        fields: Vec<(String, Ty<'tcx>)>,
        given: &[usize],
        span: Span,
    ) -> R<(Vec<(String, Expr)>, bool)> {
        let mut props = Vec::new();
        let mut kept = false;
        for (i, (name, field_ty)) in fields.into_iter().enumerate() {
            let replaced = given.contains(&i);
            let value = if replaced && self.is_node_param(field_ty) {
                Expr::undefined()
            } else {
                let value = self.default_value(field_ty, span)?;
                if replaced && !value.has_effects() {
                    Expr::undefined()
                } else {
                    kept |= replaced;
                    value
                }
            };
            props.push((name, value));
        }
        Ok((props, kept))
    }

    /// Is `ty` a type parameter this function bounds by react's `Node`? Each
    /// type that is one is std's or React's, whose default does nothing.
    fn is_node_param(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Param(_))
            && self.typing_env.param_env.caller_bounds().iter().any(|clause| {
                clause.as_trait_clause().is_some_and(|tr| {
                    let tr = tr.skip_binder();
                    tr.self_ty() == ty && bindings::is_jsx_node(self.tcx, tr.def_id())
                })
            })
    }

    /// A constructor as a value, `.map(Some)` or `.map(Shape::Circle)`: an
    /// arrow of its fields, making what a call of it makes (ADR 0125).
    pub(super) fn constructor_value(&mut self, def_id: DefId, args: ty::GenericArgsRef<'tcx>, span: Span) -> R<Expr> {
        let sig = self
            .tcx
            .fn_sig(def_id)
            .instantiate(self.tcx, args)
            .skip_normalization()
            .skip_binder();
        let ty = sig.output();
        let ty::Adt(adt_def, adt_args) = *ty.kind() else {
            return Err(self.unsupported(span, "this constructor as a value"));
        };
        let variant = adt_def.variant_with_ctor_id(def_id);
        // Its arrow reads only its own parameters, so their names can't take
        // another's: `value` of one field, and `_0`, `_1` of more, a variant's
        // own names for them.
        let params: Vec<String> = match sig.inputs().len() {
            1 => vec!["value".into()],
            count => (0..count).map(|i| format!("_{i}")).collect(),
        };
        let items: Vec<Expr> = params.iter().map(|name| Expr::var(name)).collect();
        let value = if let Some(inner) = self.option_of(ty) {
            // `Some`: the value, as `Some(x)` is `x` (ADR 0030), or boxed where
            // it could look like `None` (ADR 0051).
            let [item] = <[Expr; 1]>::try_from(items).map_err(|_| self.unsupported(span, "this constructor"))?;
            if self.boxed_payload(inner) {
                self.some(item)
            } else {
                item
            }
        } else if adt_def.is_union() || self.is_fmt_result(ty) {
            return Err(self.unsupported(span, "this constructor as a value"));
        } else {
            let tag = adt_def.is_enum().then(|| bindings::variant_name(self.tcx, variant));
            let shape = match tag {
                Some(_) => Shape::Object(self.variant_fields(variant, adt_args)),
                None => self.shape(ty),
            };
            assembled(shape, tag, items)
        };
        Ok(Expr::arrow(
            params.into_iter().map(Into::into).collect(),
            vec![StmtKind::Return(Some(value)).at(self.js_span(span))],
        ))
    }
}
