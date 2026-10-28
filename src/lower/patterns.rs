//! Bindings, destructuring and match/let-chain evaluation regions.

use super::recognition::StdItem;
use super::{FnCx, Num, Shape, bindings, without_refs};
use crate::js::{self, Expr};
use rustc_ast::{LitKind, Mutability};

use rustc_hir::def::DefKind;
use rustc_hir::{BindingMode, ByRef};

use rustc_middle::thir::{ExprKind, Pat, PatKind};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A tuple or struct pattern of plain variables and `_`s, as JS
    /// destructuring: `[count, setCount]`, `{ initial, label }`. Binds the
    /// variables, and says whether one is `mut`. `None`, binding nothing, if a
    /// part is anything else, or needs a copy of its own (ADR 0020).
    pub(super) fn js_pattern(&mut self, pat: &Pat<'tcx>) -> Option<(js::Pattern, bool)> {
        // A struct's or a tuple's fields, or an array's first items, `[a, b,
        // ..]`, which are `const [a, b] = xs` (ADR 0123): each by where it is.
        let fields: Vec<(usize, &Pat<'tcx>)> = match &pat.kind {
            PatKind::Leaf { subpatterns } => subpatterns.iter().map(|f| (f.field.as_usize(), &f.pattern)).collect(),
            PatKind::Array { prefix, slice, suffix }
                if suffix.is_empty() && slice.as_ref().is_none_or(|rest| matches!(rest.kind, PatKind::Wild)) =>
            {
                prefix.iter().enumerate().collect()
            }
            _ => return None,
        };
        // `(i, &x)`: a reference is the value (ADR 0023), so that part is `x`.
        let parts: Vec<_> = fields
            .into_iter()
            .map(|(i, field)| match without_refs(field).kind {
                // A cell, a `&mut` to a number, is taken apart as a cell (ADR 0099).
                _ if self.is_cell(field.ty) => None,
                // A `Cell` in an object's field is the property itself (ADR
                // 0288): what's bound of it is a handle on it, or a cell of its
                // own, which JS's destructuring wouldn't give.
                _ if matches!(self.shape(pat.ty), Shape::Object(_)) && self.is_std_type(field.ty, StdItem::Cell) => {
                    None
                }
                PatKind::Wild => Some((i, None)),
                PatKind::Binding {
                    name,
                    var,
                    mode: BindingMode(ByRef::No, mutability),
                    subpattern: None,
                    ty,
                    ..
                } if self.unsupported_part(ty).is_none() && !(self.contains_mutated(ty) && self.is_copy(ty)) => {
                    Some((i, Some((name, var, mutability == Mutability::Mut))))
                }
                _ => None,
            })
            .collect::<Option<_>>()?;
        let mutable = parts.iter().any(|(_, part)| part.is_some_and(|(_, _, m)| m));
        if matches!(pat.kind, PatKind::Array { .. }) {
            let mut items = Vec::new();
            for (i, part) in parts {
                items.resize(i + 1, None);
                items[i] = part.map(|(name, var, m)| self.bind(var, name.as_str(), m));
            }
            // `[a, , ]` is `[a]`.
            while items.last().is_some_and(Option::is_none) {
                items.pop();
            }
            return Some((js::Pattern::names_array(items), mutable));
        }
        let pattern = match self.shape(pat.ty) {
            Shape::Array(tys) => {
                let mut items = vec![None; tys.len()];
                for (i, part) in parts {
                    items[i] = part.map(|(name, var, m)| self.bind(var, name.as_str(), m));
                }
                // `[a, b, , ]` is `[a, b]`.
                while items.last().is_some_and(Option::is_none) {
                    items.pop();
                }
                js::Pattern::names_array(items)
            }
            Shape::Object(fields) => {
                let mut named = Vec::new();
                let mut rest = None;
                for (i, part) in parts {
                    let Some((name, var, m)) = part else { continue };
                    let bound = self.bind(var, name.as_str(), m);
                    // The props a component's struct doesn't name, `...rest` (ADR
                    // 0195), or a flattened struct's, typed (ADR 0204).
                    match super::bindings::is_rest_field(self.tcx, pat.ty, i) {
                        true => rest = Some(bound),
                        false => {
                            let default = self.prop_default(pat.ty, i, pat.span);
                            named.push((fields[i].0.clone(), js::Pattern::Name(bound), default))
                        }
                    }
                }
                // What the rest holds is what isn't named, so a field the
                // pattern leaves, `..` or `_`, is named still, `className:
                // _className`, or the rest would hold it (ADR 0205).
                if rest.is_some() {
                    let keys: Vec<&str> = (fields.iter().enumerate())
                        .filter(|&(i, (key, _))| {
                            !super::bindings::is_rest_field(self.tcx, pat.ty, i)
                                && !named.iter().any(|(k, _, _)| k == key)
                        })
                        .map(|(_, (key, _))| key.as_str())
                        .collect();
                    for (key, unused) in keys.iter().zip(self.left_out(&keys)) {
                        named.push((key.to_string(), js::Pattern::Name(unused), None));
                    }
                }
                if let Some(rest) = &rest {
                    let taken = named.iter().map(|(key, _, _)| key.clone()).collect();
                    self.locals.rests.insert(rest.clone(), taken);
                }
                js::Pattern::Object(named, rest)
            }
            Shape::Other => return None,
        };
        Some((pattern, mutable))
    }

    /// Field `i`'s default, of `ty`, a props struct, where JS takes it
    /// apart, `{ size = "md" }` (ADR 0212): its type's `Default`, which is
    /// a literal, or the literal its `#[rust_js::default]` says, of its type.
    pub(super) fn prop_default(&mut self, ty: Ty<'tcx>, i: usize, span: Span) -> Option<Expr> {
        let ty::Adt(adt, args) = ty.kind() else { return None };
        let field = adt.non_enum_variant().fields.iter().nth(i)?;
        let field_ty = field.ty(self.tcx, args).skip_normalization();
        // `#[rust_js::default(NAME)]`: a `const` beside the struct, its value.
        if let Some(name) = bindings::field_default_const(self.tcx, field) {
            let module = self.tcx.parent_module_from_def_id(adt.did().expect_local());
            let named = (self.tcx.hir_module_free_items(module))
                .map(|id| id.owner_id.to_def_id())
                .find(|&id| {
                    matches!(self.tcx.def_kind(id), DefKind::Const { .. }) && self.tcx.item_ident(id).name == name
                });
            let Some(def_id) = named else {
                self.unsupported(span, "a props field's default that names no `const` beside its struct");
                return None;
            };
            let value = super::eval_const(self.tcx, self.typing_env, def_id, ty::List::empty(), span)
                .and_then(|value| super::const_js(self.tcx, value));
            let Some(value) = value.filter(is_literal) else {
                self.unsupported(span, "a props field's default `const` that isn't a literal");
                return None;
            };
            return Some(value);
        }
        let default = match bindings::field_default(self.tcx, field)? {
            Some(lit) => {
                let of_type = match lit {
                    LitKind::Str(..) => self.is_string_like(field_ty),
                    LitKind::Bool(_) => field_ty.is_bool(),
                    LitKind::Int(..) => Num::of(field_ty).is_some(),
                    LitKind::Float(..) => Num::of(field_ty).is_some_and(Num::float),
                    _ => false,
                };
                if !of_type {
                    self.unsupported(span, "a props field's default that isn't of the field's type");
                    return None;
                }
                self.literal(&lit, false, field_ty, span).ok()?
            }
            None => self.default_value(field_ty, span).ok()?,
        };
        if !is_literal(&default) {
            self.unsupported(
                span,
                "a props field's default that isn't a literal: say it, `#[rust_js::default = \"..\"]`",
            );
            return None;
        }
        Some(default)
    }
}

/// A literal: a constant, or an array or object of them, which a default
/// in JS's destructuring is (ADR 0212).
fn is_literal(value: &Expr) -> bool {
    match &value.kind {
        js::ExprKind::Array(items) => items.iter().all(is_literal),
        js::ExprKind::Object(props, ..) => props
            .iter()
            .all(|p| matches!(p, js::Prop::Field(_, v) if is_literal(v))),
        _ => value.is_constant(),
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Is each call of `def_id` what it's given, in JS (`gives_its_argument`)?
    /// A library's, as its contract says.
    pub(super) fn gives_its_argument(&self, def_id: DefId) -> bool {
        gives_its_argument(self.tcx, self.krate.bodies, def_id) || self.krate.foreign.gives_its_argument(def_id)
    }
}

/// Is `def_id` the crate's function, of no type parameters, each call of
/// which is what it's given in JS, so one nothing reads as a value, of the
/// crate's alone, is written nowhere (ADR 0264, amended)? One whose body is
/// a `match` of its one parameter, a fieldless enum, giving each variant
/// its own name, `Section::Learn => "learn"`: a variant is its name in JS
/// (ADR 0013). Or a `From` between a `#[repr(transparent)]` struct and its
/// field, which is the struct in JS (ADR 0584), of the struct made of its
/// parameter, `EventPriority(lane)`, or its field read, `priority.0`: the
/// user's decision, `EventPriority::from(lane)` is `lane` (ADR 0584,
/// amended).
pub(super) fn gives_its_argument<'tcx>(
    tcx: rustc_middle::ty::TyCtxt<'tcx>,
    bodies: &std::collections::HashMap<DefId, &super::Body<'tcx>>,
    def_id: DefId,
) -> bool {
    {
        let Some(body) = bodies.get(&def_id) else {
            return false;
        };
        // Not one kept out of line, `#[inline(never)]` (ADR 0649): its call
        // is its call.
        if tcx.generics_of(def_id).count() != 0 || super::recognition::is_noinline(tcx, def_id) {
            return false;
        }
        let thir = &body.thir;
        let [param] = &thir.params.raw[..] else { return false };
        let Some(PatKind::Binding {
            var,
            subpattern: None,
            mode: BindingMode(ByRef::No, _),
            ..
        }) = param.pat.as_deref().map(|pat| &pat.kind)
        else {
            return false;
        };
        let strip = |e| super::body_queries::strip(thir, e);
        let Some(root) = body.root() else { return false };
        let mut value = strip(root);
        if let ExprKind::Block { block } = thir[value].kind {
            let block = &thir[block];
            let (true, Some(expr)) = (block.stmts.is_empty(), block.expr) else {
                return false;
            };
            value = strip(expr);
        }
        let given = |e| matches!(thir[strip(e)].kind, ExprKind::VarRef { id } if id == *var);
        let converts = (tcx.trait_impl_of_assoc(def_id))
            .and_then(|imp| tcx.impl_opt_trait_ref(imp))
            .is_some_and(|tr| super::recognition::is_std_def(tcx, tr.skip_binder().def_id, StdItem::From));
        if converts {
            let transparent = |ty: Ty<'tcx>| match ty.kind() {
                ty::Adt(adt, _) => super::recognition::transparent_field(tcx, *adt),
                _ => None,
            };
            return match thir[value].kind {
                ExprKind::Adt(ref made) => transparent(thir[value].ty).is_some_and(|at| {
                    matches!(made.base, rustc_middle::thir::AdtExprBase::None)
                        && matches!(&made.fields[..], [field] if field.name.as_usize() == at && given(field.expr))
                }),
                ExprKind::Field { lhs, name, .. } => {
                    given(lhs) && transparent(thir[strip(lhs)].ty).is_some_and(|at| name.as_usize() == at)
                }
                _ => false,
            };
        }
        let ty::Adt(adt, _) = param.ty.kind() else { return false };
        if !super::representation::is_string_enum(tcx, *adt) {
            return false;
        }
        let ExprKind::Match {
            scrutinee, ref arms, ..
        } = thir[value].kind
        else {
            return false;
        };
        if !matches!(thir[strip(scrutinee)].kind, ExprKind::VarRef { id } if id == *var)
            || arms.len() != adt.variants().len()
        {
            return false;
        }
        arms.iter().all(|&arm| {
            let arm = &thir[arm];
            arm.guard.is_none()
                && matches!(arm.pattern.kind, PatKind::Variant { variant_index, ref subpatterns, .. }
                    if subpatterns.is_empty()
                        && matches!(thir[strip(arm.body)].kind, ExprKind::Literal { lit, neg: false }
                            if matches!(lit.node, LitKind::Str(text, _)
                                if text.as_str() == super::bindings::variant_name(tcx, adt.variant(variant_index)))))
        })
    }
}
