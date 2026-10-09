//! Bindings, destructuring and match/let-chain evaluation regions.

use super::fn_def;
use super::recognition::{Std, StdItem};
use super::{
    Binding, Dest, Evaluation, FnCx, Num, R, Shape, Var, bindings, camel_case, const_js, drops, fresh_in, js_ident,
    ordering_value, recognition::is_non_zero, std_impls, variant_field, without_refs,
};
use crate::js::{self, Expr, Op, Stmt, StmtKind, UnaryOp};
use rustc_ast::{LitKind, Mutability};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_hir::{BindingMode, ByRef, RangeEnd};
use rustc_middle::mir::BorrowKind;
use rustc_middle::thir::visit::{self, Visitor};
use rustc_middle::thir::{self, ArmId, ExprId, ExprKind, LogicalOp, Pat, PatKind, PatRangeBoundary};
use rustc_middle::ty::{self, Ty};
use rustc_span::def_id::DefId;
use rustc_span::{DesugaringKind, Span};

/// One level of a let chain: what runs before its test, its test's parts, and
/// what its body starts with, a `let`'s bindings.
pub(super) type LetLevel = (Vec<Stmt>, Vec<Expr>, Vec<Stmt>);

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A tuple or struct pattern of plain variables and `_`s, as JS
    /// destructuring: `[count, setCount]`, `{ initial, label }`. Binds the
    /// variables, and says whether one is `mut`. `None`, binding nothing, if a
    /// part is anything else, or needs a copy of its own (ADR 0020).
    pub(super) fn js_pattern(&mut self, pat: &Pat<'tcx>) -> Option<(js::Pattern, bool)> {
        // Of a reference, `let Params { message, code } = params();`: a
        // reference is the value (ADR 0023), taken apart as it is.
        let pat = without_refs(pat);
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
        // A flattened struct taken apart, `WithRouterProps { props: SeoProps {
        // title, .. }, router }`: its fields are its parent's in JS (ADR 0204),
        // so its pattern's are the parent's pattern's, `{ title, router }`.
        // Each where it's written, among the other fields.
        let mut flattened = Vec::new();
        let mut kept = Vec::new();
        let mut written = Vec::new();
        for (at, (i, field)) in fields.into_iter().enumerate() {
            if matches!(pat.kind, PatKind::Leaf { .. })
                && bindings::is_flatten_field(self.tcx, pat.ty, i)
                && matches!(without_refs(field).kind, PatKind::Leaf { .. })
            {
                let (nested, mutable) = self.js_pattern(field)?;
                flattened.push((at, nested, mutable));
            } else {
                kept.push((i, field));
                written.push(at);
            }
        }
        let fields = kept;
        // `(i, &x)`: a reference is the value (ADR 0023), so that part is `x`.
        let parts: Vec<_> = fields
            .into_iter()
            .map(|(i, field)| match without_refs(field).kind {
                // A cell, a `&mut` to a number, is taken apart as a cell (ADR 0099).
                _ if self.is_cell(field.ty) => None,
                // A `Cell` in an object's field is the property itself (ADR
                // 0288): what's bound of it is a handle on it, or a cell of its
                // own, which JS's destructuring wouldn't give. One read at
                // once is its value, then, which its `get()` reads (ADR 0293).
                _ if matches!(self.shape(pat.ty), Shape::Object(_)) && self.is_std_type(field.ty, StdItem::Cell) => {
                    match without_refs(field).kind {
                        PatKind::Binding {
                            name,
                            var,
                            subpattern: None,
                            ..
                        } if self.krate.read_at_once.contains(&var) => Some((i, Some((name, var, false)))),
                        _ => None,
                    }
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
                // Bound by a shared reference, each part is read once: what's
                // borrowed can't change while it is, and a `Cell`, which can,
                // is the one JS object either way (ADR 0244).
                PatKind::Binding {
                    name,
                    var,
                    mode: BindingMode(ByRef::Yes(_, Mutability::Not), _),
                    subpattern: None,
                    ty,
                    ..
                } if self.unsupported_part(ty).is_none() => Some((i, Some((name, var, false)))),
                _ => None,
            })
            .collect::<Option<_>>()?;
        let mutable =
            parts.iter().any(|(_, part)| part.is_some_and(|(_, _, m)| m)) || flattened.iter().any(|(_, _, m)| *m);
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
            return Some((js::Pattern::Array(items), mutable));
        }
        let pattern = match self.shape(pat.ty) {
            Shape::Array(_) if !flattened.is_empty() => return None,
            Shape::Array(tys) => {
                let mut items = vec![None; tys.len()];
                for (i, part) in parts {
                    items[i] = part.map(|(name, var, m)| self.bind(var, name.as_str(), m));
                }
                // `[a, b, , ]` is `[a, b]`.
                while items.last().is_some_and(Option::is_none) {
                    items.pop();
                }
                js::Pattern::Array(items)
            }
            Shape::Object(fields) => {
                let mut named = Vec::new();
                let mut rest = None;
                let some_flattened = !flattened.is_empty();
                let mut flattened = flattened.into_iter().peekable();
                let mut flattened_rest = None;
                let mut merge = |nested: js::Pattern, named: &mut Vec<_>| -> Option<()> {
                    let js::Pattern::Object(more, more_rest) = nested else {
                        return None;
                    };
                    named.extend(more);
                    if more_rest.is_some() {
                        if flattened_rest.is_some() {
                            return None;
                        }
                        flattened_rest = more_rest;
                    }
                    Some(())
                };
                for (k, (i, part)) in parts.into_iter().enumerate() {
                    while let Some((_, nested, _)) = flattened.next_if(|(at, _, _)| *at < written[k]) {
                        merge(nested, &mut named)?;
                    }
                    let Some((name, var, m)) = part else { continue };
                    let bound = self.bind(var, name.as_str(), m);
                    // A `&Cell` read at once, as its value (ADR 0293).
                    if self.krate.read_at_once.contains(&var)
                        && let Some(local) = self.locals.vars.get_mut(&var)
                    {
                        local.place = Expr::handle(local.place.clone());
                    }
                    // The props a component's struct doesn't name, `...rest` (ADR
                    // 0195), or a flattened struct's, typed (ADR 0204).
                    match super::bindings::is_rest_field(self.tcx, pat.ty, i) {
                        true => rest = Some(bound),
                        false => {
                            let default = self.prop_default(pat.ty, i, pat.span);
                            named.push((fields[i].0.clone(), bound, default))
                        }
                    }
                }
                for (_, nested, _) in flattened {
                    merge(nested, &mut named)?;
                }
                // A rest beside one would hold the flattened struct's fields
                // its pattern leaves.
                if some_flattened {
                    if rest.is_some() {
                        return None;
                    }
                    rest = flattened_rest;
                }
                // What the rest holds is what isn't named, so a field the
                // pattern leaves, `..` or `_`, is named still, `className:
                // _className`, or the rest would hold it (ADR 0205).
                if rest.is_some() {
                    for (i, (key, _)) in fields.iter().enumerate() {
                        if !super::bindings::is_rest_field(self.tcx, pat.ty, i)
                            && !named.iter().any(|(k, _, _)| k == key)
                        {
                            let unused = self.fresh(&format!("_{}", camel_case(&js_ident(key))));
                            named.push((key.clone(), unused, None));
                        }
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
    fn prop_default(&mut self, ty: Ty<'tcx>, i: usize, span: Span) -> Option<Expr> {
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

    pub(super) fn lower_let(
        &mut self,
        pat: &Pat<'tcx>,
        init: Option<ExprId>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        // A chain kept in a variable: who ends it can't be told here, so it's
        // lazy if a stage does what can be seen, when every use of it iterates
        // it, and nothing that wants an array sees it (ADR 0139).
        if let PatKind::Binding {
            var,
            mode: BindingMode(ByRef::No, _),
            subpattern: None,
            ..
        } = pat.kind
            && let Some(init) = init
        {
            self.keep_chain(var, init);
        }
        // A closure only a hook's function is: written there, named, unless
        // it reads what it would be named (ADR 0297).
        if let PatKind::Binding { name, var, .. } = pat.kind
            && self.body_facts.named_callbacks.contains(&var)
            && let Some(init) = init
        {
            let value = self.expr(init, out)?;
            let js_name = super::camel_case(name.as_str());
            let mut reads_own = false;
            value.visit_vars(&mut |read| reads_own |= read == js_name);
            let (params, body, is_async) = match value.kind {
                js::ExprKind::Arrow(ref params, ref body) if !reads_own => (params.clone(), body.clone(), false),
                js::ExprKind::AsyncArrow(ref params, ref body) if !reads_own => (params.clone(), body.clone(), true),
                _ => {
                    let bound = self.bind(var, name.as_str(), false);
                    out.push(StmtKind::Const(bound, value).at(self.js_span(span)));
                    return Ok(());
                }
            };
            let span = value.span;
            let function = js::Function {
                name: js_name,
                params,
                body,
                export: false,
                is_async,
                span,
                name_span: self.js_span(pat.span),
            };
            self.locals.vars.insert(
                var,
                Var {
                    place: Expr {
                        kind: js::ExprKind::Function(Box::new(function)),
                        span,
                    },
                    mutable: false,
                    depth: self.loops.len(),
                },
            );
            return Ok(());
        }
        // A cell that's its function's variable (ADR 0287): `let n = 0` of
        // what it starts as, and a clone of it the same variable.
        if let PatKind::Binding {
            name,
            var,
            mode: BindingMode(ByRef::No, _),
            subpattern: None,
            ..
        } = pat.kind
            && let Some(&first) = self.krate.plain_cells.get(&var)
            && let Some(init) = init
        {
            if first != var
                && let Some(place) = self.locals.vars.get(&first).map(|v| v.place.clone())
            {
                let depth = self.loops.len();
                self.locals.vars.insert(
                    var,
                    Var {
                        place,
                        mutable: true,
                        depth,
                    },
                );
                return Ok(());
            }
            if let Some(start) = self.cell_start(init) {
                let value = self.expr(start, out)?;
                let name = self.bind(var, name.as_str(), true);
                out.push(StmtKind::Let(name, Some(value)).at(self.js_span(span)));
                return Ok(());
            }
        }
        // A local that `it.next()` steps through: what knows where it is,
        // `$iter(v)` or `Iterator.from(it)` (ADR 0071).
        if let PatKind::Binding {
            name,
            var,
            mode: BindingMode(ByRef::No, mutability),
            subpattern: None,
            ty,
            ..
        } = pat.kind
            && let Some(init) = init
            && let Some(value) = self.stepped_value(var, (init, ty), span, out)?
        {
            let name = self.bind(var, name.as_str(), mutability == Mutability::Mut);
            let kind = if mutability == Mutability::Mut {
                StmtKind::Let(name, Some(value))
            } else {
                StmtKind::Const(name, value)
            };
            out.push(kind.at(self.js_span(span)));
            return Ok(());
        }
        // `let f = { let c = ..; move |n| .. };`: the block's statements
        // first, then `const f = ..` of its value. Every local has a JS name
        // of its own, so none of them can clash where they now are.
        if let Some(init) = init
            && let ExprKind::Block { block } = self.thir[self.strip(init)].kind
            && let thir::Block {
                targeted_by_break: false,
                expr: Some(value),
                safety_mode: thir::BlockSafety::Safe,
                span: block_span,
                ..
            } = self.thir[block]
            && !block_span.from_expansion()
            && !self.block_owns(block).unwrap_or(true)
        {
            self.block_stmts(block, out)?;
            return self.lower_let(pat, Some(value), span, out);
        }
        // `async fn f(x)` moves `x` into its body with `let x = x;` (ADR 0029).
        // In JS the body is the function's, so they're one variable.
        // `format_args!` holds its values in `super let args = (&a, &b);`, then
        // `super let args = [Argument::new_display(args.0), ..];` (ADR 0034).
        // Both are only read from, so they name their parts where they are:
        // `format!("{} ms", t)` is `t + " ms"`, with no arrays in between.
        if self.in_format_args(span)
            && let PatKind::Binding {
                var,
                mode: BindingMode(ByRef::No, Mutability::Not),
                subpattern: None,
                ..
            } = pat.kind
            && let Some(init) = init
        {
            let parts = match self.thir[self.strip(init)].kind {
                ExprKind::Tuple { ref fields } => self.tuple_parts(fields, "arg", true, out)?,
                _ => self.expr(init, out)?,
            };
            self.locals.vars.insert(
                var,
                Var {
                    place: parts,
                    mutable: false,
                    depth: self.loops.len(),
                },
            );
            return Ok(());
        }
        // `let a = f()?;` on an option: the value is `a` itself, so it's kept
        // under that name: `const a = f(); if (a == null) { return undefined; }`.
        if let PatKind::Binding {
            name,
            var,
            mode: BindingMode(ByRef::No, Mutability::Not),
            subpattern: None,
            ..
        } = pat.kind
            && let Some(init) = init
            && let Some(tried) = self.body_query().as_question(init)
            && self.option_of(self.thir[tried].ty).is_some()
            && !self.has_drops(self.thir[init].ty)
        {
            let value = self.question(init, tried, Some(name.as_str()), out)?;
            self.locals.vars.insert(
                var,
                Var {
                    place: value,
                    mutable: false,
                    depth: self.loops.len(),
                },
            );
            return Ok(());
        }
        if span.is_desugaring(DesugaringKind::Async)
            && let PatKind::Binding {
                var,
                mode: BindingMode(ByRef::No, mutability),
                subpattern: None,
                ..
            } = pat.kind
            && let Some(init) = init
            && let ExprKind::UpvarRef { var_hir_id, .. } = self.thir[self.strip(init)].kind
            && let Some(outer) = self.locals.vars.get(&var_hir_id)
        {
            let alias = Var {
                place: outer.place.clone(),
                mutable: mutability == Mutability::Mut,
                depth: outer.depth,
            };
            self.locals.vars.insert(var, alias);
            return Ok(());
        }
        // `let y = &mut x;`: `y` names `x`, so `*y = 5` is `x = 5` (ADR 0099).
        // While `y` lives, Rust lets nothing else use `x`.
        if let PatKind::Binding {
            name,
            var,
            mode: BindingMode(ByRef::No, Mutability::Not),
            subpattern: None,
            ty,
            ..
        } = pat.kind
            && let ty::Ref(_, inner, Mutability::Mut) = *ty.kind()
            && !self.is_object(inner)
            && let Some(init) = init
            && let moved = match self.thir[self.strip(init)].kind {
                ExprKind::VarRef { id } => self.is_alias(id).then_some(id),
                _ => None,
            }
            && let Some(borrowed) = self.mut_borrowed(init).or(moved.map(|_| init))
        {
            let place = match moved {
                Some(id) => self.locals.vars[&id].place.clone(),
                // `let x = &mut 1;`: `let x = 1;`, which `x` names.
                None if self.is_temporary(borrowed) => {
                    let value = self.expr(borrowed, out)?;
                    let home = self.fresh(&camel_case(name.as_str()));
                    out.push(StmtKind::Let(home.clone(), Some(value)).at(self.js_span(span)));
                    Expr::var(&home)
                }
                None => self.fixed_place(borrowed, pat.span, out)?,
            };
            self.bind_alias(var);
            self.locals.vars.insert(
                var,
                Var {
                    place,
                    mutable: true,
                    depth: self.loops.len(),
                },
            );
            return Ok(());
        }
        // `let mut n = m.lock().unwrap();` of a number: `n` names the cell's
        // `value`, as a `&mut` names its place, and the guard keeps anything
        // else from using it meanwhile (ADRs 0025, 0099, 0144).
        if let PatKind::Binding {
            var,
            mode: BindingMode(ByRef::No, _),
            subpattern: None,
            ty,
            ..
        } = pat.kind
            && self.is_guard(ty)
            && let ty::Adt(_, args) = ty.kind()
            && !args.types().next().is_some_and(|guarded| self.is_object(guarded))
            && let Some(init) = init
            && let Some(cell) = self.guarded_cell(init)
            && (self.in_element(cell) || self.place(cell).is_some())
        {
            // Fixed where it's locked, as a `&mut`'s place is.
            let place = Expr::member(self.fixed_place(cell, pat.span, out)?, "value");
            self.bind_alias(var);
            self.locals.vars.insert(
                var,
                Var {
                    place,
                    mutable: true,
                    depth: self.loops.len(),
                },
            );
            return Ok(());
        }
        let from_async = span.is_desugaring(DesugaringKind::Async);
        let span = self.js_span(span);
        match &pat.kind {
            PatKind::Binding {
                name,
                var,
                mode,
                subpattern: None,
                ty,
                ..
            } => {
                self.check_by_value(*mode, *ty, pat.span)?;
                self.check_value_ty(*ty, pat.span)?;
                let mutable = mode.1 == Mutability::Mut;
                // `let ref r = f();` owns what `f` made, as `let r = f();` does;
                // `let ref r = x;` borrows `x` (ADR 0098).
                let owned = match mode.0 {
                    ByRef::No => Some(*ty),
                    ByRef::Yes(..) => init
                        .filter(|&i| !drops::is_place(&self.thir[self.strip(i)].kind))
                        .map(|i| self.thir[i].ty),
                }
                .filter(|&t| self.has_drops(t));
                let owns = owned.is_some();
                if owns && init.is_none() {
                    return Err(self.unsupported(pat.span, "a `let` of a value with a destructor, without its value"));
                }
                let name = match init {
                    // Only control flow needs `let x;` and then assignments in
                    // its branches. Anything else (a closure, say) computes its
                    // statements first and then has a value.
                    Some(init) if self.is_simple(init) || !self.is_control_flow(init) => {
                        let value = self.expr(init, out)?;
                        let name = self.bind(*var, name.as_str(), mutable);
                        let kind = if mutable {
                            StmtKind::Let(name.clone(), Some(value))
                        } else {
                            StmtKind::Const(name.clone(), value)
                        };
                        out.push(kind.at(span));
                        name
                    }
                    Some(init) => {
                        // A table read by what's matched has a value (ADR 0233):
                        // `const variant = variantMap[type]`.
                        let indexed = match self.thir[self.strip(init)].kind {
                            ExprKind::Match {
                                scrutinee, ref arms, ..
                            } => {
                                let arms = arms.clone();
                                self.match_index(scrutinee, &arms, out)?
                            }
                            _ => None,
                        };
                        let name = self.bind(*var, name.as_str(), mutable);
                        match indexed {
                            Some(value) if mutable => out.push(StmtKind::Let(name.clone(), Some(value)).at(span)),
                            Some(value) => out.push(StmtKind::Const(name.clone(), value).at(span)),
                            None => {
                                out.push(StmtKind::Let(name.clone(), None).at(span));
                                self.stmt(init, &Dest::Assign(name.clone()), out)?;
                            }
                        }
                        name
                    }
                    None => {
                        let name = self.bind(*var, name.as_str(), mutable);
                        out.push(StmtKind::Let(name.clone(), None).at(span));
                        name
                    }
                };
                // `let mut r = &mut x;`: a cell, whose `value` is `x` (ADR 0099).
                if mode.0 == ByRef::No && self.is_cell(*ty) {
                    self.bind_boxed(*var);
                }
                if let Some(owned) = owned {
                    self.own(*var, Expr::var(&name), owned, pat.span, out)?;
                }
                Ok(())
            }
            // `let _ = f();` drops what `f` made at once; `let _ = x;` doesn't move `x`.
            PatKind::Wild => match init {
                Some(init)
                    if self.has_drops(self.thir[init].ty)
                        && !matches!(
                            self.thir[self.strip(init)].kind,
                            ExprKind::VarRef { .. }
                                | ExprKind::Field { .. }
                                | ExprKind::Index { .. }
                                | ExprKind::Deref { .. }
                                | ExprKind::UpvarRef { .. }
                                | ExprKind::StaticRef { .. }
                        ) =>
                {
                    let ty = self.thir[init].ty;
                    let value = self.expr(init, out)?;
                    let value = self.droppable(value, ty, out);
                    self.drop_value(value, ty, pat.span, out)
                }
                Some(init) => self.stmt(init, &Dest::Discard, out),
                None => Ok(()),
            },
            // `let (q, r) = divmod(a, b);`
            _ => {
                // `let (a, b);`, which a destructuring assignment gives values
                // later: each of its variables is a `let a;` of its own.
                let Some(init) = init else {
                    let mut bindings = Vec::new();
                    let mut other = false;
                    pat.walk_always(|p| match p.kind {
                        PatKind::Binding { subpattern: None, .. } => bindings.push(p.clone()),
                        PatKind::Binding { .. } => other = true,
                        _ => {}
                    });
                    if other {
                        return Err(self.unsupported(pat.span, "this `let` pattern without a value"));
                    }
                    for binding in bindings {
                        self.lower_let(&binding, None, binding.span, out)?;
                    }
                    return Ok(());
                };
                // `let (count, set_count) = use_state(0);` is
                // `const [count, setCount] = useState(0);`. A place is taken
                // apart where it is, below.
                if self.place(init).is_none() && !self.is_control_flow(init) {
                    let value = self.expr(init, out)?;
                    // A temporary taken apart binds each part on its own, so
                    // its `try` can hold them (ADR 0131).
                    if self.takes_apart_temporary(init)? {
                        return self.destructure(pat, value, true, false, out);
                    }
                    if let Some((pattern, mutable)) = self.js_pattern(pat) {
                        out.push(
                            StmtKind::Destructure {
                                pattern,
                                value,
                                mutable,
                            }
                            .at(span),
                        );
                        return Ok(());
                    }
                    let subject = self.spill("tmp", value, out);
                    return self.destructure(pat, subject, true, false, out);
                }
                let items = self.item_subject(init);
                // `let {error: rawError, registerBundler} = sandpack;`: a
                // place taken apart into variables is too, each read once
                // as Rust reads it (ADR 0291). Not what owns a destructor,
                // which is moved out of the place below, nor an `async fn`'s
                // parameter, its own pattern (ADR 0029).
                let mut binds = false;
                pat.walk_always(|p| binds |= matches!(p.kind, PatKind::Binding { .. }));
                if binds
                    && !from_async
                    && !items
                    && !self.has_drops(self.thir[init].ty)
                    && let Some((pattern, mutable)) = self.js_pattern(pat)
                {
                    let value = self.expr(init, out)?;
                    out.push(
                        StmtKind::Destructure {
                            pattern,
                            value,
                            mutable,
                        }
                        .at(span),
                    );
                    return Ok(());
                }
                let (subject, stable) = self.subject(init, "tmp", out)?;
                // What it binds by value is moved out of `init` (ADR 0098).
                self.clear_parts(init, pat, out);
                self.destructure(pat, subject, stable, items, out)
            }
        }
    }

    /// Bind the variables of an irrefutable pattern to the parts of `subject`.
    pub(super) fn destructure(
        &mut self,
        pat: &Pat<'tcx>,
        subject: Expr,
        stable: bool,
        items: bool,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let mut bindings = Vec::new();
        // Irrefutable as rustc checks it, a parameter's, a `for`'s or a
        // `let`'s: a test of one always holds, as `Ok(n) | Err(n)`'s does.
        self.pattern_test(pat, &subject, &mut bindings)?;
        self.bind_all(bindings, stable, items, self.js_span(pat.span), out)
    }

    /// Where a `match` or `let` finds the value it takes apart, and whether
    /// that stays unchanged while the pattern's variables live.
    ///
    /// A place is used where it is, and may be stable (see `stable_place`).
    /// A tuple of stable places (`match (a, b)`) is used without building
    /// the array. Anything else is computed once into a `const` named `base`,
    /// which is stable: no Rust variable can move or change it.
    /// The stable place a `match` of `e` tests where it is: `e`'s own, or
    /// that of what a binding gives back as it is, `kind_of(&children)` or
    /// `classify(value)`, the node itself, or `o.as_deref()` of an
    /// `Option`, the option itself (ADR 0211). What's bound of it names it.
    pub(super) fn matched_place(&self, e: ExprId) -> Option<Expr> {
        if let Some(place) = self.stable_place(e) {
            return Some(place);
        }
        let ExprKind::Call { fun, ref args, .. } = self.thir[self.strip(e)].kind else {
            return None;
        };
        let [arg] = args[..] else { return None };
        let as_is = fn_def(self.thir[fun].ty).is_some_and(|(def_id, _)| {
            bindings::is_binding(self.tcx, def_id)
                && matches!(bindings::js_form(self.tcx, def_id), bindings::JsForm::This)
        });
        if !as_is && !matches!(self.std_fn(fun), Some(Std::Pointee)) {
            return None;
        }
        let arg = match self.thir[self.strip(arg)].kind {
            ExprKind::Borrow { arg, .. } => arg,
            _ => arg,
        };
        self.stable_place(arg)
    }

    pub(super) fn subject(&mut self, e: ExprId, base: &str, out: &mut Vec<Stmt>) -> R<(Expr, bool)> {
        // A `&mut` in a variable, which names its place (ADR 0099): a handle on
        // it, as every `&mut` to a value JS can't change in place is matched.
        if self.is_cell(self.thir[e].ty) && !self.is_cell_value(e) && self.place(e).is_some() {
            return Ok((self.read(e, out)?, false));
        }
        if let Some(place) = self.matched_place(e) {
            return Ok((place, true));
        }
        // `&mut x`: the place, which its `ref mut` bindings name, as a `&x`
        // one is; of a temporary, `&mut Some(3)`, a `let` of it (ADR 0099).
        // Of a value JS can't change in place, the `&mut` is a handle on the
        // place, which a `&mut` pattern takes apart as the place itself and a
        // binding of the `&mut` binds.
        if let ExprKind::Borrow {
            borrow_kind: BorrowKind::Mut { .. },
            arg,
        } = self.thir[self.strip(e)].kind
        {
            let cell = |place: Expr, this: &Self| {
                if this.makes_cell(this.thir[arg].ty) {
                    Expr::handle(place)
                } else {
                    place
                }
            };
            if let Some((place, _)) = self.place(arg) {
                return Ok((cell(place, self), false));
            }
            if self.is_temporary(arg) {
                let value = self.expr(arg, out)?;
                let name = self.fresh(base);
                out.push(StmtKind::Let(name.clone(), Some(value)).at(self.js_span(self.thir[e].span)));
                self.home_temporary(name.clone());
                return Ok((cell(Expr::var(&name), self), false));
            }
        }
        // `&x`: a reference is the value (ADR 0023), and a borrowed `x` stays put.
        if let ExprKind::Borrow {
            borrow_kind: BorrowKind::Shared,
            arg,
        } = self.thir[self.strip(e)].kind
            && let Some(place) = self.stable_place(arg)
        {
            return Ok((place, true));
        }
        if let Some((place, _)) = self.place(e) {
            return Ok((place, false));
        }
        // A temporary taken apart is in a `const` of its own already, which
        // owns what's left of it (ADR 0131).
        if self.takes_apart_temporary(e)? {
            return Ok((self.expr(e, out)?, true));
        }
        // `match (a, b)` tests `a` and `b` where they are. A part that isn't a
        // place that stays put goes in a `const` of its own, in order.
        if let ExprKind::Tuple { ref fields } = self.thir[self.strip(e)].kind
            && !fields.is_empty()
        {
            return Ok((self.tuple_parts(fields, base, false, out)?, true));
        }
        let value = self.expr(e, out)?;
        let name = self.fresh(base);
        out.push(StmtKind::Const(name.clone(), value).at(self.js_span(self.thir[e].span)));
        Ok((Expr::var(&name), true))
    }

    /// `[a, b]` for a tuple `(a, b)` that's only taken apart: each part a
    /// place that stays put, a constant, or else a `const` of its own, in order.
    /// With `used_once`, a part without effects is written in place too.
    pub(super) fn tuple_parts(
        &mut self,
        fields: &[ExprId],
        base: &str,
        used_once: bool,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let mut parts = Vec::new();
        for &f in fields {
            // `format_args!`'s parts are references: `&a` is `a`. Not `&*o` of a
            // box, which is its `value` (ADR 0074).
            let referent = match self.thir[self.strip(f)].kind {
                ExprKind::Borrow { arg, .. } => arg,
                _ => f,
            };
            let part = match self.stable_place(referent) {
                Some(place) => place,
                None => {
                    let value = self.expr(f, out)?;
                    // One that reads only variables that never change, `Some(anchor)`
                    // of jsx!'s captured `ref`, reads the same where it's used (ADR 0252).
                    if value.is_constant() || (used_once && !value.has_effects()) || self.reads_unchanging(&value, out)
                    {
                        value
                    } else {
                        self.spill(base, value, out)
                    }
                }
            };
            parts.push(part);
        }
        Ok(Expr::array(parts))
    }

    /// Is `span` rustc's lowering of a `format_args!` (so `format!`, `panic!`, ..)?
    pub(super) fn in_format_args(&self, span: Span) -> bool {
        matches!(span.desugaring_kind(), Some(DesugaringKind::FormatLiteral { .. }))
    }

    /// Give a pattern's variables their JS meaning. Immutable ones bound into
    /// a stable subject just name the place they matched, as ReScript does:
    /// `P { x, y } => x + y` becomes `p.x + p.y`. The rest get a variable
    /// holding their own value. Of `items`, a std call's (`item_subject`),
    /// a `&mut` is the item, not a cell (ADR 0099).
    pub(super) fn bind_all(
        &mut self,
        bindings: Vec<Binding<'tcx>>,
        stable: bool,
        items: bool,
        span: js::Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        // `x @ B { b, .. }`: `x`, or where it's moved, is the same JS object
        // as the value, and a change to it would change what `b` reads in
        // place, so each binding has its own. `n @ 1..=9` binds nothing else.
        let stable = stable && !(bindings.len() > 1 && bindings.iter().any(|b| b.whole));
        for b in bindings {
            // `Some(r)` of an `Option<&mut i32>`: `r` is a cell (ADR 0099).
            if !b.by_ref_mut && self.is_cell(b.ty) {
                if items {
                    self.bind_item(b.var);
                } else {
                    self.bind_boxed(b.var);
                }
            }
            // A place that's computed, like `$someValue(o)`, goes in a `const`.
            // A `ref mut` one always does: `*r = x` writes the place it names.
            // One that owns what it binds, by value, drops it as its scope
            // ends (ADR 0098): a `const` of its own, named as in Rust.
            let owns = self.is_owner(b.var)?;
            if (stable || b.by_ref_mut) && !b.mutable && !b.place.has_effects() && !owns {
                // `ref mut` of a `let` variable writes it, as a `&mut` in a
                // variable does (ADR 0099): `if let Some(n) = p { *n += 1 }`.
                // Not an object's: its variable may be a `&mut` itself.
                if b.by_ref_mut
                    && matches!(*b.ty.kind(), ty::Ref(_, inner, _) if !self.is_object(inner))
                    && self.is_let(&b.place)
                {
                    self.bind_alias(b.var);
                }
                self.locals.vars.insert(
                    b.var,
                    Var {
                        place: b.place,
                        mutable: false,
                        depth: self.loops.len(),
                    },
                );
                continue;
            }
            let value = self.copy_if_needed(b.place, b.ty).or_at(span);
            let name = self.bind(b.var, &b.name, b.mutable);
            let kind = if b.mutable {
                StmtKind::Let(name.clone(), Some(value))
            } else {
                StmtKind::Const(name.clone(), value)
            };
            out.push(kind.at(span));
            if owns {
                self.own_at(b.var, Expr::var(&name), b.ty, span, out)?;
            }
        }
        Ok(())
    }

    pub(super) fn lower_match(&mut self, scrutinee: ExprId, arms: &[ArmId], dest: &Dest, out: &mut Vec<Stmt>) -> R<()> {
        // A `fmt::Result` is nothing in JS (ADR 0054): there's no `Err` to match.
        if self.is_fmt_result(self.thir[scrutinee].ty) {
            return Err(self.unsupported(self.thir[scrutinee].span, "matching a `fmt::Result`"));
        }
        if self.recognition().is_io_unit_result(self.thir[scrutinee].ty) {
            return Err(self.unsupported(self.thir[scrutinee].span, "matching an `io::Result`"));
        }
        // Evaluate the scrutinee once, unless it's a place that can be
        // tested where it is.
        let items = self.item_subject(scrutinee);
        let (subject, stable) = self.subject(scrutinee, "match", out)?;

        // Each arm: its test, a guard's statements and test when it needs
        // statements of its own, and its body.
        // A guard that needs statements: they and its test; or an `if let`
        // guard, a let chain, whose levels the arm's body goes inside.
        enum Guarded {
            Test(Vec<Stmt>, Expr),
            Chain(Vec<LetLevel>),
        }
        type Arm = (Option<Expr>, Option<Guarded>, Vec<Stmt>, js::Span);
        let mut chain: Vec<Arm> = Vec::new();
        for (i, &arm_id) in arms.iter().enumerate() {
            let arm = &self.thir[arm_id];
            let arm_span = self.js_span(arm.span);
            let pat_span = self.js_span(arm.pattern.span);
            let mut bindings = Vec::new();
            let mut test = self
                .pattern_test(&arm.pattern, &subject, &mut bindings)?
                .map(|t| t.or_at(pat_span));
            if let Some(guard) = arm.guard {
                self.check_guarded(guard, &bindings)?;
            }

            // A guard is tested before the arm's body, where a binding that
            // isn't the place it names gets its `const`. So the guard reads
            // each binding from its place, which nothing has changed yet: it
            // runs right after the pattern's test.
            let mut guarded = None;
            if let Some(guard) = arm.guard {
                for b in &bindings {
                    let place = Var {
                        place: b.place.clone(),
                        mutable: false,
                        depth: self.loops.len(),
                    };
                    self.locals.vars.insert(b.var, place);
                }
                // `Some(x) if let Ok(n) = x.parse() =>` (ADR 0127): a let chain,
                // whose `let`s bind for the arm's body.
                let parts = self.let_chain(guard).or_else(|| {
                    let part = self.strip(guard);
                    matches!(self.thir[part].kind, ExprKind::Let { .. }).then(|| vec![part])
                });
                if let Some(parts) = parts {
                    let levels = self.let_chain_levels(parts);
                    for b in &bindings {
                        self.locals.vars.remove(&b.var);
                    }
                    guarded = Some(Guarded::Chain(levels?));
                } else {
                    let mut before = Vec::new();
                    let guard = self.expr(guard, &mut before);
                    for b in &bindings {
                        self.locals.vars.remove(&b.var);
                    }
                    let guard = guard?;
                    if before.is_empty() {
                        test = Some(match test {
                            Some(t) => Expr::bin(Op::And, t, guard),
                            None => guard,
                        });
                    } else {
                        guarded = Some(Guarded::Test(before, guard));
                    }
                }
            }
            // The arm owns what its pattern moves out of the scrutinee, and
            // drops it as it ends (ADR 0098).
            let mut body = Vec::new();
            let mark = self.owned_mark();
            self.clear_parts(scrutinee, &arm.pattern, &mut body);
            self.bind_all(bindings, stable, items, pat_span, &mut body)?;
            // Rust checked the match is exhaustive, so if we reach the last
            // unguarded arm, it matches. No need to test it.
            if i == arms.len() - 1 && arm.guard.is_none() {
                test = None;
            }
            // What ends with the arm, `f(v)` of a body that's `match f(v) {
            // .. }`, is dropped as it ends, before what it binds (ADR 0191).
            let mut arm_body = Vec::new();
            self.begin_scope(arm.scope);
            self.stmt(arm.body, dest, &mut arm_body)?;
            self.end_scope(0, arm.span, &mut arm_body)?;
            self.close_scope(mark, arm_body, arm.span, &mut body)?;
            let done = test.is_none() && guarded.is_none();
            chain.push((test, guarded, body, arm_span));
            if done {
                break; // Later arms are unreachable.
            }
        }

        // Fold into `if (..) {..} else if (..) {..} else {..}`. A guard with
        // statements runs them after its arm's test, and one that fails goes
        // on to the later arms, so the chain is a labeled block that a
        // matched arm leaves.
        let mut label = None;
        let mut rest: Option<Vec<Stmt>> = None;
        for (test, guarded, mut body, span) in chain.into_iter().rev() {
            if let Some(guarded) = guarded {
                let leaves = matches!(
                    body.last().map(|s| &s.kind),
                    Some(StmtKind::Return(_) | StmtKind::Throw(_) | StmtKind::Break(_) | StmtKind::Continue(_))
                );
                if !leaves {
                    let label = label.get_or_insert_with(|| fresh_in(&mut self.labels, "arms")).clone();
                    body.push(StmtKind::Break(Some(label)).at(span));
                }
                let before = match guarded {
                    Guarded::Test(mut before, guard) => {
                        before.push(StmtKind::If(guard, body, None).at(span));
                        before
                    }
                    // The body inside the chain's levels, which a level that
                    // doesn't hold goes past, on to the later arms.
                    Guarded::Chain(levels) => {
                        let mut before = Vec::new();
                        self.assemble_let_chain(levels, body, None, span, &mut before);
                        before
                    }
                };
                let mut arm = match test {
                    Some(t) => vec![StmtKind::If(t, before, None).at(span)],
                    None => before,
                };
                arm.extend(rest.unwrap_or_default());
                rest = Some(arm);
                continue;
            }
            rest = match test {
                // An arm that does nothing, `Dot => {}`, before others:
                // `if (s !== "Dot") { .. }`, not `if (s === "Dot") {} else ..`.
                Some(t) if body.is_empty() && rest.is_some() => Some(vec![
                    StmtKind::If(std_impls::negate(t), rest.unwrap_or_default(), None).at(span),
                ]),
                Some(t) => Some(vec![StmtKind::If(t, body, rest).at(span)]),
                // A last arm that does nothing, `None => {}`: no `else {}`.
                None if body.is_empty() => None,
                None => Some(body),
            };
        }
        match label {
            Some(label) => {
                let span = self.js_span(self.thir[scrutinee].span);
                out.push(StmtKind::Labeled(label, rest.unwrap_or_default()).at(span));
            }
            None => out.extend(rest.unwrap_or_default()),
        }
        Ok(())
    }

    /// An `if`'s condition as the parts joined by `&&`, when there are
    /// several and one is a `let`: a let chain (ADR 0048).
    pub(super) fn let_chain(&self, cond: ExprId) -> Option<Vec<ExprId>> {
        fn parts(cx: &FnCx<'_, '_>, e: ExprId, found: &mut Vec<ExprId>) {
            let e = cx.strip(e);
            match cx.thir[e].kind {
                ExprKind::LogicalOp {
                    op: LogicalOp::And,
                    lhs,
                    rhs,
                } => {
                    parts(cx, lhs, found);
                    parts(cx, rhs, found);
                }
                _ => found.push(e),
            }
        }
        let mut found = Vec::new();
        parts(self, cond, &mut found);
        let has_let = found.iter().any(|&p| matches!(self.thir[p].kind, ExprKind::Let { .. }));
        (found.len() > 1 && has_let).then_some(found)
    }

    /// `if let Some(h) = half(n) && h > 2 && let Some(q) = f(h) { .. } else { .. }`.
    /// Each part runs only if the ones before it held, and may read what an
    /// earlier `let` bound. Parts that need no statements of their own join
    /// one test: `const h = half(n); if (h != null && h > 2) { .. }`. One that
    /// does, like a `let` of a call, opens an `if` inside. With more than one,
    /// the `else` follows them all in a labeled block, which the `then` leaves.
    pub(super) fn lower_let_chain(
        &mut self,
        parts: Vec<ExprId>,
        then: ExprId,
        else_opt: Option<ExprId>,
        dest: &Dest,
        span: js::Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let mark = self.owned_mark();
        let levels = self.let_chain_levels(parts)?;
        // One a later test fails after would have to be dropped before the
        // `else`, with what's left of what it came from.
        if self.owned_mark() > mark {
            let span = self.thir[then].span;
            return Err(self.unsupported(span, "a let chain that binds a value with a destructor"));
        }
        let mut then_out = Vec::new();
        self.stmt(then, dest, &mut then_out)?;
        let else_out = match else_opt {
            Some(els) => {
                let mut else_out = Vec::new();
                self.stmt(els, dest, &mut else_out)?;
                Some(else_out)
            }
            None => None,
        };
        self.assemble_let_chain(levels, then_out, else_out, span, out);
        Ok(())
    }

    /// A let chain's parts, each level what runs before its test, its test,
    /// and what its body starts with (a `let`'s bindings). What each `let`
    /// binds is in scope after, for what's lowered in its body.
    pub(super) fn let_chain_levels(&mut self, parts: Vec<ExprId>) -> R<Vec<LetLevel>> {
        let mut levels: Vec<LetLevel> = vec![(Vec::new(), Vec::new(), Vec::new())];
        for part in parts {
            let (mut before, mut bindings) = (Vec::new(), Vec::new());
            let test = match self.thir[part].kind {
                ExprKind::Let { expr, ref pat } => self.if_let(expr, pat, &mut bindings, &mut before)?,
                _ => self.expr(part, &mut before)?,
            };
            let level = levels.last_mut().expect("a level");
            if level.1.is_empty() || (before.is_empty() && level.2.is_empty()) {
                level.0.extend(before);
                level.1.push(test);
                level.2.extend(bindings);
            } else {
                levels.push((before, vec![test], bindings));
            }
        }
        Ok(levels)
    }

    /// A let chain's levels around its `then`, lowered already, and its
    /// `else`, if it has one.
    pub(super) fn assemble_let_chain(
        &mut self,
        levels: Vec<LetLevel>,
        mut then_out: Vec<Stmt>,
        mut else_out: Option<Vec<Stmt>>,
        span: js::Span,
        out: &mut Vec<Stmt>,
    ) {
        // An `else` of one statement, `throw ..`, is written at each level a
        // test fails at, as a person writes it; a longer one is reached
        // from them all by a label's `break`.
        let repeated = else_out.as_ref().is_some_and(|e| e.len() == 1);
        let label = (levels.len() > 1 && else_out.is_some() && !repeated).then(|| fresh_in(&mut self.labels, "chain"));
        let leaves = matches!(
            then_out.last().map(|s| &s.kind),
            Some(StmtKind::Return(_) | StmtKind::Throw(_) | StmtKind::Break(_) | StmtKind::Continue(_))
        );
        if let Some(label) = &label
            && !leaves
        {
            then_out.push(StmtKind::Break(Some(label.clone())).at(span));
        }
        // Built from the innermost level out; only a lone level has the `else`.
        let single = levels.len() == 1;
        let mut body = then_out;
        for (before, tests, bindings) in levels.into_iter().rev() {
            let test = tests.into_iter().reduce(and).unwrap_or_else(|| Expr::bool(true));
            let mut inner = bindings;
            inner.extend(body);
            let els = match (single, repeated) {
                (true, _) => else_out.take(),
                (false, true) => else_out.clone(),
                (false, false) => None,
            };
            body = before;
            body.push(StmtKind::If(test, inner, els).at(span));
        }
        match (label, else_out) {
            (Some(label), Some(else_out)) => {
                body.extend(else_out);
                out.push(StmtKind::Labeled(label, body).at(span));
            }
            _ => out.extend(body),
        }
    }

    /// `if let pat = scrutinee`: the test, with the pattern's variables
    /// bound at the start of the `then` branch. `if let Some(el) = find()`
    /// keeps the value in a `const` named like the variable, which is then
    /// just that `const`: `const el = find(); if (el != null) { .. }`.
    pub(super) fn if_let(
        &mut self,
        scrutinee: ExprId,
        pat: &Pat<'tcx>,
        then_out: &mut Vec<Stmt>,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        if let Some(test) = self.slot_binding(scrutinee, pat, out)? {
            return Ok(test);
        }
        let base = match &pat.kind {
            PatKind::Variant { subpatterns, .. } if subpatterns.len() == 1 => match &subpatterns[0].pattern.kind {
                PatKind::Binding { name, .. } => name.to_string(),
                _ => "value".to_string(),
            },
            _ => "value".to_string(),
        };
        let items = self.item_subject(scrutinee);
        let (subject, stable) = self.subject(scrutinee, &base, out)?;
        // `let Some(href) = href.filter(..)`: the filter's test, and `href`,
        // what it kept, in place of a `const` of the `Option` (ADR 0232).
        if let Some((test, kept, field)) = self.filter_kept(&subject, pat, out) {
            let mut bindings = Vec::new();
            let inner = self.pattern_test(field, &kept, &mut bindings)?;
            let stable = matches!(&kept.kind, js::ExprKind::Var(name) if self.plain_value(name));
            self.clear_parts(scrutinee, pat, then_out);
            self.bind_all(bindings, stable, items, self.js_span(pat.span), then_out)?;
            return Ok(match inner {
                Some(inner) => Expr::bin(Op::And, test, inner),
                None => test,
            });
        }
        let mut bindings = Vec::new();
        let test = self.pattern_test(pat, &subject, &mut bindings)?;
        // What it binds by value is moved out of the scrutinee (ADR 0098).
        self.clear_parts(scrutinee, pat, then_out);
        self.bind_all(bindings, stable, items, self.js_span(pat.span), then_out)?;
        Ok(test.unwrap_or_else(|| Expr::bool(true)))
    }

    /// `Some(p)` of a `filter`'s `Option`, which `subject` names, the last
    /// `const` in `out`: that `const` taken back, the filter's test, what it
    /// kept, and `p`. Of text, falsy only empty, `x != null && x.length !==
    /// 0` is `x`, as a test.
    fn filter_kept<'p>(
        &self,
        subject: &Expr,
        pat: &'p Pat<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> Option<(Expr, Expr, &'p Pat<'tcx>)> {
        let PatKind::Variant {
            adt_def,
            variant_index,
            subpatterns,
            ..
        } = &pat.kind
        else {
            return None;
        };
        let [field] = subpatterns.as_slice() else { return None };
        let some = self
            .tcx
            .is_lang_item(adt_def.variant(*variant_index).def_id, LangItem::OptionSome);
        if !some || self.option_of(pat.ty).is_none_or(|inner| self.boxed_payload(inner)) {
            return None;
        }
        let js::ExprKind::Var(name) = &subject.kind else {
            return None;
        };
        let Some(Stmt {
            kind: StmtKind::Const(held, value),
            ..
        }) = out.last()
        else {
            return None;
        };
        if held != name {
            return None;
        }
        let (test, kept) = super::options::filtered(value)?;
        out.pop();
        Some((test, kept, &field.pattern))
    }

    /// `if let Some(n) = m.get_mut(&k)` of a map whose values are primitives:
    /// `let n = m.get(k)`, which a write through `n` puts back (`PreparedPlace::Slot`).
    pub(super) fn slot_binding(&mut self, scrutinee: ExprId, pat: &Pat<'tcx>, out: &mut Vec<Stmt>) -> R<Option<Expr>> {
        let ExprKind::Call { fun, ref args, .. } = self.thir[self.strip(scrutinee)].kind else {
            return Ok(None);
        };
        let PatKind::Variant { subpatterns, .. } = &pat.kind else {
            return Ok(None);
        };
        let [field] = subpatterns.as_slice() else {
            return Ok(None);
        };
        let PatKind::Binding {
            name,
            var,
            mode: BindingMode(ByRef::No, Mutability::Not),
            subpattern: None,
            ty,
            ..
        } = &field.pattern.kind
        else {
            return Ok(None);
        };
        let Some((get, generic_args)) = fn_def(self.thir[self.strip(fun)].ty) else {
            return Ok(None);
        };
        let slot = self.recognition().is_mutable_map_get(get, generic_args)
            && matches!(ty.kind(), ty::Ref(_, value, Mutability::Mut) if self.is_primitive_key(*value));
        if !slot {
            return Ok(None);
        }
        let args = args.clone();
        let [map, key]: [Expr; 2] = self.operands(&args, out)?.try_into().ok().expect("a map and a key");
        let map = if map.reads_same() {
            map
        } else {
            self.spill("map", map, out)
        };
        let key = if key.reads_same() {
            key
        } else {
            self.spill("key", key, out)
        };
        let name = self.bind(*var, name.as_str(), true);
        let there = Expr::call(Expr::member(map.clone(), "get"), vec![key.clone()]);
        out.push(StmtKind::Let(name.clone(), Some(there)).at(self.js_span(pat.span)));
        self.bind_slot(*var, (map, key));
        Ok(Some(Expr::bin(Op::LooseNe, Expr::var(&name), Expr::null())))
    }

    /// The shape `as_matches` takes: `pat => true, _ => false`.
    /// A guard is tried with each alternative of a `|` pattern that matches,
    /// in turn, until it holds: `(a, _) | (_, a) if a > 10` of `(3, 42)`
    /// takes the arm, with `a` 42. A binding at a choice of places is only
    /// the first's, so a guard of it is an error (ADR 0124). One bound at the
    /// same place in each is the same value, whichever alternative it's of.
    fn check_guarded(&self, guard: ExprId, bindings: &[Binding<'tcx>]) -> R<()> {
        if bindings.iter().any(|b| b.chosen) {
            return Err(self.unsupported(
                self.thir[guard].span,
                "a guard of a `|` pattern binding a name at another place in each alternative",
            ));
        }
        Ok(())
    }

    pub(super) fn is_matches(&self, arms: &[ArmId]) -> bool {
        let is_bool = |arm: ArmId, want: bool| matches!(self.thir[self.strip(self.thir[arm].body)].kind, ExprKind::Literal { lit, .. } if lit.node == LitKind::Bool(want));
        matches!(arms, &[first, rest] if is_bool(first, true) && is_bool(rest, false)
            && matches!(self.thir[rest].pattern.kind, PatKind::Wild) && self.thir[rest].guard.is_none())
    }

    /// `matches!(x, pat)`, or `match x { pat if guard => true, _ => false }`:
    /// just the test, `x.TAG === "Circle"`, when the pattern binds nothing
    /// the guard can't read where it is.
    /// `(kind ?? "Primary") === "Primary" ? a : b`: a two-arm `match` as a
    /// value, as a person writes it, its arms plain (ADR 0209). Its subject
    /// is in place where the test reads it once, else in a `const` of its
    /// own; what its arms bind, of a subject in place, is named where it is,
    /// `item.path`. `None` where it's statements.
    /// `match kind { Kind::Note => &MAP.note, Kind::Pitfall => &MAP.pitfall }`,
    /// each arm a variant whose value is its name giving the field of that
    /// name of one table: `MAP[kind]`, as JS reads a table by its key. Rust
    /// says every variant has its arm (ADR 0233).
    pub(super) fn match_index(&mut self, scrutinee: ExprId, arms: &[ArmId], out: &mut Vec<Stmt>) -> R<Option<Expr>> {
        if arms.len() < 2 {
            return Ok(None);
        }
        const SUBJECT: &str = "$subject";
        let place = self.stable_place(scrutinee);
        let subject = place.clone().unwrap_or_else(|| Expr::var(SUBJECT));
        let mut table: Option<Expr> = None;
        let mut own_names = 0;
        for &arm in arms {
            let arm = &self.thir[arm];
            if arm.guard.is_some() {
                return Ok(None);
            }
            let mut bindings = Vec::new();
            let Some(test) = self.pattern_test(&arm.pattern, &subject, &mut bindings)? else {
                return Ok(None);
            };
            let js::ExprKind::Binary(Op::Eq, tested, name) = &test.kind else {
                return Ok(None);
            };
            let js::ExprKind::Str(name) = &name.kind else {
                return Ok(None);
            };
            if !bindings.is_empty() || !same_place(tested, &subject) {
                return Ok(None);
            }
            // `Kind::Note => "note"`: the variant's own name, which it is
            // (ADR 0264).
            if let ExprKind::Literal { lit, neg: false } = self.thir[self.strip(arm.body)].kind
                && let LitKind::Str(text, _) = lit.node
                && text.as_str() == name.as_str()
            {
                own_names += 1;
                continue;
            }
            let body = match self.thir[self.strip(arm.body)].kind {
                ExprKind::Borrow {
                    borrow_kind: BorrowKind::Shared,
                    arg,
                } => arg,
                _ => arm.body,
            };
            let Some(field) = self.stable_place(body) else {
                return Ok(None);
            };
            let js::ExprKind::Member(of, key) = field.kind else {
                return Ok(None);
            };
            if key != *name || table.as_ref().is_some_and(|table| !same_place(table, &of)) {
                return Ok(None);
            }
            table = Some(*of);
        }
        let key = |this: &mut Self, out: &mut Vec<Stmt>| match place {
            Some(place) => Ok(place),
            None => this.expr(scrutinee, out),
        };
        // Each variant's own name: what's matched.
        if own_names == arms.len() {
            return Ok(Some(key(self, out)?));
        }
        let Some(table) = table.filter(|_| own_names == 0) else {
            return Ok(None);
        };
        Ok(Some(Expr::index(table, key(self, out)?)))
    }

    pub(super) fn match_conditional(
        &mut self,
        scrutinee: ExprId,
        arms: &[ArmId],
        span: js::Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let &[first, second] = arms else { return Ok(None) };
        if !self.is_conditional_match(scrutinee, arms) {
            return Ok(None);
        }
        // Tested where it is, or, of what isn't a place, read once: a name
        // no Rust one has, which the test is read for.
        const SUBJECT: &str = "$subject";
        // Of what isn't a place, and an arm that binds of it, a `const` of
        // it, whose places they name (ADR 0309).
        let mut binds = false;
        for arm in [first, second] {
            self.thir[arm]
                .pattern
                .walk_always(|p| binds |= matches!(p.kind, PatKind::Binding { .. }));
        }
        let mut spilled = None;
        let (subject, value) = match self.matched_place(scrutinee) {
            Some(place) => (place, None),
            None if binds && self.is_value(scrutinee) => {
                let value = self.expr(scrutinee, out)?;
                let name = self.fresh("match");
                spilled = Some((name.clone(), value));
                (Expr::var(&name), None)
            }
            None => (Expr::var(SUBJECT), Some(self.expr(scrutinee, out)?)),
        };
        let (mut bindings, mut otherwise) = (Vec::new(), Vec::new());
        let first_test = self.pattern_test(&self.thir[first].pattern, &subject, &mut bindings)?;
        self.pattern_test(&self.thir[second].pattern, &subject, &mut otherwise)?;
        if !self.binds_in_place(&bindings, self.thir[first].guard.is_some())?
            || !self.binds_in_place(&otherwise, false)?
        {
            return Ok(None);
        }
        let items = self.item_subject(scrutinee);
        let at = out.len();
        if let Some((name, value)) = &spilled {
            out.push(StmtKind::Const(name.clone(), value.clone()).at(span));
        }
        self.bind_all(bindings, true, items, span, out)?;
        // A first arm that takes everything has no test: its guard is it,
        // or its body is the value, the second never reached.
        let Some(mut test) = first_test else {
            if let Some(value) = value.filter(Expr::has_effects) {
                out.push(StmtKind::Expr(value).at(span));
            }
            let test = match self.thir[first].guard {
                Some(guard) => self.expr(guard, out)?,
                None => return Ok(Some(self.expr(self.thir[first].body, out)?)),
            };
            let yes = self.evaluated(self.thir[first].body)?;
            self.bind_all(otherwise, true, items, span, out)?;
            let no = self.evaluated(self.thir[second].body)?;
            return Ok(Some(self.conditional(test, yes, no, span, out)));
        };
        if let Some(value) = value {
            let mut reads = 0;
            test.visit_vars(&mut |var| reads += usize::from(var == SUBJECT));
            let read = match reads {
                1 => value,
                _ => {
                    let name = self.fresh("match");
                    out.push(StmtKind::Const(name.clone(), value).at(span));
                    Expr::var(&name)
                }
            };
            test = test
                .substitute(&|var| (var == SUBJECT).then(|| read.clone()))
                .expect("a test has no closure inside");
        }
        // A guard is the arm's test too, `n === 0 && flag`, made only if it matches.
        if let Some(guard) = self.thir[first].guard {
            let guard = self.evaluated(guard)?;
            test = match guard.statements.is_empty() {
                true => Expr::bin(Op::And, test, guard.value),
                false => {
                    let otherwise = Evaluation {
                        statements: Vec::new(),
                        value: Expr::bool(false),
                    };
                    self.conditional(test, guard, otherwise, span, out)
                }
            };
        }
        let yes = self.evaluated(self.thir[first].body)?;
        self.bind_all(otherwise, true, items, span, out)?;
        let no = self.evaluated(self.thir[second].body)?;
        // The option's value, or a part of it, or a default: `e ?? d`,
        // `e?.[1] ?? d`, of a variable or a `const` of a value, which it
        // then needs no more, as `map_or` is (ADR 0309).
        let option = match (&spilled, &subject.kind) {
            (Some((name, value)), _) if out.len() == at + 1 => Some((name.clone(), value.clone())),
            (None, js::ExprKind::Var(name)) => Some((name.clone(), subject.clone())),
            _ => None,
        };
        if let Some((name, option)) = option
            && yes.statements.is_empty()
            && no.statements.is_empty()
            && let Some(nullish) = super::options::nullish_or(
                &name,
                &option,
                &test,
                &yes.value,
                &no.value,
                self.option_of(self.thir[scrutinee].ty.peel_refs())
                    .is_some_and(|inner| self.never_falsy(inner)),
                !self.can_be_nullish(self.thir[self.thir[first].body].ty),
            )
        {
            if spilled.is_some() {
                out.pop();
            }
            return Ok(Some(nullish));
        }
        Ok(Some(self.conditional(test, yes, no, span, out)))
    }

    /// Does `bind_all` name each of these where it is, with no statement:
    /// read only, owning nothing, no cell, nor a `|` pattern's choice a
    /// guard reads?
    fn binds_in_place(&mut self, bindings: &[Binding<'tcx>], guarded: bool) -> R<bool> {
        if bindings.len() > 1 && bindings.iter().any(|b| b.whole) {
            return Ok(false);
        }
        for b in bindings {
            if b.mutable
                || b.by_ref_mut
                || (guarded && b.chosen)
                || b.place.has_effects()
                || self.is_cell(b.ty)
                || self.is_owner(b.var)?
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Is this `match` one `match_conditional` writes: two arms, plain,
    /// the first's guard plain too? The second can't be guarded but where
    /// it's never reached.
    /// Is `e` a value, not a place: a call's, say, which a `match` of it
    /// reads from a `const` (ADR 0309). Not what a JS call threw, whose
    /// `match` is a `try` of its statements (ADR 0035).
    fn is_value(&self, e: ExprId) -> bool {
        let thrown = match self.thir[self.strip(e)].kind {
            ExprKind::Call { fun, .. } => super::fn_def(self.thir[fun].ty).is_some_and(|(def_id, args)| {
                super::bindings::is_binding(self.tcx, def_id)
                    && matches!(
                        self.recognition().catching(def_id, args),
                        super::recognition::Catching::Result
                    )
            }),
            _ => false,
        };
        !thrown
            && !matches!(
                self.thir[self.strip(e)].kind,
                ExprKind::VarRef { .. }
                    | ExprKind::UpvarRef { .. }
                    | ExprKind::Field { .. }
                    | ExprKind::Deref { .. }
                    | ExprKind::Index { .. }
                    | ExprKind::StaticRef { .. }
            )
    }

    pub(super) fn is_conditional_match(&self, scrutinee: ExprId, arms: &[ArmId]) -> bool {
        // What an arm binds is named where it is, a place's, or a `const`'s
        // of a value that isn't one, a call's (ADR 0309): read only, owning
        // nothing (`binds_in_place` checks the rest as it binds).
        let place = self.matched_place(scrutinee).is_some() || self.is_value(scrutinee);
        let in_place = |pat: &Pat<'tcx>| {
            let mut in_place = true;
            pat.walk_always(|p| {
                if let PatKind::Binding {
                    mode: BindingMode(by_ref, mutability),
                    ty,
                    ..
                } = p.kind
                {
                    in_place &= place
                        && mutability == Mutability::Not
                        && !matches!(by_ref, ByRef::Yes(_, Mutability::Mut))
                        && !(ty.is_ref() && ty.is_mutable_ptr())
                        && !(matches!(by_ref, ByRef::No) && self.has_drops(ty));
                }
            });
            in_place
        };
        let &[first, _] = arms else { return false };
        self.thir[first].guard.is_none_or(|guard| self.is_simple(guard))
            && arms.iter().all(|&arm| {
                let arm = &self.thir[arm];
                in_place(&arm.pattern) && self.is_simple(arm.body)
            })
    }

    pub(super) fn as_matches(&mut self, scrutinee: ExprId, arms: &[ArmId], out: &mut Vec<Stmt>) -> R<Option<Expr>> {
        let is_bool = |arm: ArmId, want: bool| matches!(self.thir[self.strip(self.thir[arm].body)].kind, ExprKind::Literal { lit, .. } if lit.node == LitKind::Bool(want));
        let &[first, rest] = arms else { return Ok(None) };
        if !is_bool(first, true)
            || !is_bool(rest, false)
            || !matches!(self.thir[rest].pattern.kind, PatKind::Wild)
            || self.thir[rest].guard.is_some()
        {
            return Ok(None);
        }
        let items = self.item_subject(scrutinee);
        let start = out.len();
        let (subject, stable) = self.subject(scrutinee, "match", out)?;
        let mut bindings = Vec::new();
        let test = self.pattern_test(&self.thir[first].pattern, &subject, &mut bindings)?;
        if let Some(guard) = self.thir[first].guard {
            self.check_guarded(guard, &bindings)?;
        }
        // Tested once, first, and bound nowhere, it's read where it's tested:
        // `child.type.mdxName === "inlineCode"`, as react.dev's Link has it.
        let test = match test {
            Some(test) if bindings.is_empty() && self.thir[first].guard.is_none() && !self.makes_drops(scrutinee) => {
                Some(read_in_place(test, out, start))
            }
            test => test,
        };
        if !bindings.is_empty() && (!stable || bindings.iter().any(|b| b.mutable)) {
            return Err(self.unsupported(self.thir[first].pattern.span, "this binding in `matches!`"));
        }
        let span = self.js_span(self.thir[first].span);
        self.bind_all(bindings, stable, items, span, out)?;
        let guard = match self.thir[first].guard {
            Some(guard) if self.is_simple(guard) => Some(self.evaluated(guard)?),
            Some(guard) => return Err(self.unsupported(self.thir[guard].span, "this guard")),
            None => None,
        };
        let test = match (test, guard) {
            (Some(t), Some(g)) if g.statements.is_empty() => Expr::bin(Op::And, t, g.value),
            (Some(t), Some(g)) => self.conditional(
                t,
                g,
                Evaluation {
                    statements: Vec::new(),
                    value: Expr::bool(false),
                },
                span,
                out,
            ),
            (None, Some(g)) => {
                out.extend(g.statements);
                g.value
            }
            (Some(t), None) => t,
            (None, None) => Expr::bool(true),
        };
        Ok(Some(test))
    }

    /// Does `e`, or anything in it, make a value with a destructor, whose
    /// `const` its drop names?
    fn makes_drops(&self, e: ExprId) -> bool {
        struct Types<'a, 'tcx> {
            thir: &'a thir::Thir<'tcx>,
            types: Vec<Ty<'tcx>>,
        }
        impl<'a, 'tcx> Visitor<'a, 'tcx> for Types<'a, 'tcx> {
            fn thir(&self) -> &'a thir::Thir<'tcx> {
                self.thir
            }
            fn visit_expr(&mut self, expr: &'a thir::Expr<'tcx>) {
                self.types.push(expr.ty);
                visit::walk_expr(self, expr);
            }
        }
        let mut types = Types {
            thir: self.thir,
            types: Vec::new(),
        };
        types.visit_expr(&self.thir[e]);
        types.types.into_iter().any(|ty| self.has_drops(ty))
    }

    /// A JS boolean test for "`subject` matches `pat`" (`None`: always matches).
    pub(super) fn pattern_test(
        &mut self,
        pat: &Pat<'tcx>,
        subject: &Expr,
        bindings: &mut Vec<Binding<'tcx>>,
    ) -> R<Option<Expr>> {
        match &pat.kind {
            PatKind::Wild => Ok(None),
            PatKind::Binding {
                name,
                var,
                mode,
                subpattern,
                ty,
                ..
            } => {
                // A `ref mut` binding names the place it matched (`bind_all`),
                // so even a number's can be written through.
                let by_ref_mut = matches!(mode.0, ByRef::Yes(_, Mutability::Mut));
                if !by_ref_mut {
                    self.check_by_value(*mode, *ty, pat.span)?;
                }
                bindings.push(Binding {
                    var: *var,
                    name: name.to_string(),
                    mutable: mode.1 == Mutability::Mut,
                    by_ref_mut,
                    whole: subpattern.is_some(),
                    place: subject.clone(),
                    chosen: false,
                    ty: *ty,
                });
                // `x @ 1..=9`: bound, and tested by what's after the `@`.
                match subpattern {
                    Some(inner) => self.pattern_test(inner, subject, bindings),
                    None => Ok(None),
                }
            }
            PatKind::Constant { value } => {
                let value = self.const_value(*value, pat.span)?;
                // A `bool` is JS's own: `true` is the subject, `false` its negation
                // (ADR 0209).
                Ok(Some(match value.kind {
                    js::ExprKind::Bool(true) if pat.ty.is_bool() => subject.clone(),
                    js::ExprKind::Bool(false) if pat.ty.is_bool() => Expr::unary(UnaryOp::Not, subject.clone()),
                    _ => Expr::bin(Op::Eq, subject.clone(), value),
                }))
            }
            // `1..=9`, `i32::MIN..0`, `'a'..='z'`: between its bounds, as `<`
            // compares numbers, and `char`s by code point (ADR 0183).
            PatKind::Range(range) => {
                let bound = |boundary: &PatRangeBoundary<'tcx>| match *boundary {
                    PatRangeBoundary::Finite(valtree) => {
                        let value = ty::Value { ty: range.ty, valtree };
                        const_js(self.tcx, value)
                            .map(Some)
                            .ok_or_else(|| self.unsupported(pat.span, "this range"))
                    }
                    PatRangeBoundary::NegInfinity | PatRangeBoundary::PosInfinity => Ok(None),
                };
                let (mut lo, mut hi) = (bound(&range.lo)?, bound(&range.hi)?);
                let below = if range.end == RangeEnd::Included {
                    Op::Le
                } else {
                    Op::Lt
                };
                // A bound at the type's own end always holds: `n >= 0` of a `u32`.
                if let Some(num) = Num::of(range.ty).filter(|&n| !n.float()) {
                    let (min, max) = num.range();
                    lo = lo.filter(|lo| lo.as_int() != Some(min));
                    hi = hi.filter(|hi| {
                        !(range.end == RangeEnd::Included
                            && i128::try_from(max).is_ok_and(|max| hi.as_int() == Some(max)))
                    });
                }
                let text = range.ty.is_char();
                let mut tests = Vec::new();
                for (op, bound) in [(Op::Ge, lo), (below, hi)] {
                    let Some(bound) = bound else { continue };
                    tests.push(match text {
                        true => self.text_compare(op, subject.clone(), bound),
                        false => Expr::bin(op, subject.clone(), bound),
                    });
                }
                Ok(tests.into_iter().reduce(|a, b| Expr::bin(Op::And, a, b)))
            }
            // `Some(p)`: not `null` or `undefined`, and the value itself matches `p`.
            // A constant needs no `!= null`: `o === 0` already says it. So does
            // one through a reference, like every string literal: `o === "a"`.
            PatKind::Variant {
                adt_def,
                variant_index,
                subpatterns,
                ..
            } if self.tcx.is_lang_item(adt_def.did(), LangItem::Option) => {
                // `None`, or `Some(..)`, whose `..` names no field but is `Some`.
                let some = self
                    .tcx
                    .is_lang_item(adt_def.variant(*variant_index).def_id, LangItem::OptionSome);
                let Some(field) = subpatterns.first() else {
                    let op = if some { Op::LooseNe } else { Op::LooseEq };
                    return Ok(Some(Expr::bin(op, subject.clone(), Expr::null())));
                };
                // A generic `T`'s value may be boxed (ADR 0051): the pattern is on what's inside.
                let value = match self.option_of(pat.ty) {
                    Some(inner) if self.boxed_payload(inner) => self.some_value(subject.clone()),
                    _ => subject.clone(),
                };
                let inner = self.pattern_test(&field.pattern, &value, bindings)?;
                let present = match self.option_of(pat.ty) {
                    Some(inner) => self.present(subject.clone(), inner),
                    None => Expr::bin(Op::LooseNe, subject.clone(), Expr::null()),
                };
                // `d === "Up"` of a unit variant, or `o === 0` of `Ordering::Equal`:
                // `undefined === "Up"` is false too (ADR 0193).
                let names_value = |test: &Expr| {
                    matches!(&test.kind, js::ExprKind::Binary(Op::Eq, left, right)
                        if same_place(left, subject)
                            && matches!(right.kind, js::ExprKind::Str(_) | js::ExprKind::Num(_) | js::ExprKind::Bool(_)))
                };
                let mut value = &field.pattern;
                while let PatKind::Deref { subpattern, .. } = &value.kind {
                    value = subpattern;
                }
                Ok(Some(match inner {
                    // `undefined >= 1` is false too.
                    Some(test) if matches!(value.kind, PatKind::Constant { .. } | PatKind::Range(_)) => test,
                    Some(test) if names_value(&test) => test,
                    Some(test) => and(present, test),
                    None => present,
                }))
            }
            // An untagged enum's variant (ADR 0214): its payload's kind, then
            // the payload itself, which is the value.
            PatKind::Variant {
                adt_def,
                variant_index,
                subpatterns,
                ..
            } if self.untagged(pat.ty).is_some() => {
                let variant = adt_def.variant(*variant_index);
                if let Some(other) = self.untagged_alike(pat.ty, variant) {
                    let message = format!(
                        "`{}` and `{other}` hold values of one kind, which JS can't tell apart in an untagged enum (ADR 0214)",
                        variant.name
                    );
                    return Err(self.tcx.dcx().span_err(pat.span, message));
                }
                let kind = self.untagged_variant_test(pat.ty, variant, subject);
                let mut tests = Vec::new();
                for field in subpatterns {
                    tests.extend(self.pattern_test(&field.pattern, subject, bindings)?);
                }
                // `x === "a"` holds of a string only: `Kind::String("a")`
                // needs no `typeof x === "string"`.
                if !tests.first().is_some_and(|test| says_kind(test, subject, &kind)) {
                    tests.insert(0, kind);
                }
                Ok(tests.into_iter().reduce(|a, b| Expr::bin(Op::And, a, b)))
            }
            // A variant (ADR 0013, 0033): its name, or its `TAG`, then its fields.
            // An enum with one variant needs no test.
            PatKind::Variant {
                adt_def,
                variant_index,
                subpatterns,
                ..
            } => {
                let variant = adt_def.variant(*variant_index);
                if let Some(n) = ordering_value(self.tcx, adt_def.did(), variant.name) {
                    return Ok(Some(Expr::bin(Op::Eq, subject.clone(), Expr::int(n))));
                }
                // A discriminated union's `otherwise`: an object whose tag is
                // none of the others', and its field the object (ADR 0284).
                if bindings::is_tagged_otherwise(self.tcx, adt_def.did(), variant) {
                    let key = bindings::tag_key(self.tcx, adt_def.did());
                    let mut tests: Vec<Expr> = (adt_def.variants().iter())
                        .filter(|other| other.def_id != variant.def_id)
                        .map(|other| {
                            let name = Expr::str(bindings::variant_name(self.tcx, other));
                            Expr::bin(Op::Ne, Expr::member(subject.clone(), &key), name)
                        })
                        .collect();
                    for field in subpatterns {
                        tests.extend(self.pattern_test(&field.pattern, subject, bindings)?);
                    }
                    return Ok(tests.into_iter().reduce(|a, b| Expr::bin(Op::And, a, b)));
                }
                let name = Expr::str(bindings::variant_name(self.tcx, variant));
                let mut tests = Vec::new();
                if adt_def.variants().len() > 1 {
                    let key = bindings::tag_key(self.tcx, adt_def.did());
                    let unit = variant.fields.is_empty() && bindings::declared_tag(self.tcx, adt_def.did()).is_none();
                    tests.push(match unit {
                        true => Expr::bin(Op::Eq, subject.clone(), name),
                        false => Expr::bin(Op::Eq, Expr::member(subject.clone(), &key), name),
                    });
                }
                for field in subpatterns {
                    let part = Expr::member(
                        subject.clone(),
                        variant_field(self.tcx, variant, field.field.as_usize()),
                    );
                    // A `Cell` in a variant's field is what it holds (ADR 0288).
                    let part = self.held(part, field.pattern.ty);
                    tests.extend(self.pattern_test(&field.pattern, &part, bindings)?);
                }
                Ok(tests.into_iter().reduce(|a, b| Expr::bin(Op::And, a, b)))
            }
            // Matching through a reference: the reference is the value (ADR 0023).
            // `&mut 3` of a cell, a box or a handle: what it points at (ADR 0099).
            PatKind::Deref { subpattern, .. } if self.is_cell(pat.ty) => {
                let pointee = match &subject.kind {
                    js::ExprKind::Handle(place) => (**place).clone(),
                    _ => Expr::member(subject.clone(), "value"),
                };
                self.pattern_test(subpattern, &pointee, bindings)
            }
            PatKind::Deref { subpattern, .. } => self.pattern_test(subpattern, subject, bindings),
            // A `NonZero` constant's pattern, `NonZero(NonZeroU64Inner(2))` as
            // rustc writes it, is on the number both are (ADR 0177).
            PatKind::Leaf { subpatterns }
                if let [field] = &subpatterns[..]
                    && let ty::Adt(adt, args) = pat.ty.kind()
                    && (is_non_zero(adt.did())
                        || adt.is_struct()
                            && matches!(adt.non_enum_variant().fields.raw.as_slice(),
                                [only] if matches!(only.ty(self.tcx, args).skip_normalization().kind(), ty::Pat(..)))) =>
            {
                self.pattern_test(&field.pattern, subject, bindings)
            }
            // A struct or tuple: every field must match.
            PatKind::Leaf { subpatterns } => {
                let mut tests = Vec::new();
                for field in subpatterns {
                    // JS's props have no `rest`: only taking them apart where
                    // they're given gives what's left of them (ADR 0195).
                    if bindings::is_rest(self.tcx, field.pattern.ty) {
                        return Err(self.unsupported(
                            field.pattern.span,
                            "a `Rest` of props taken apart here: take them apart where they're given, `fn f(Props { a, rest }: Props)`",
                        ));
                    }
                    if bindings::is_rest_field(self.tcx, pat.ty, field.field.as_usize()) {
                        return Err(self.unsupported(
                            field.pattern.span,
                            "flattened props taken apart here: take them apart where they're given, `fn f(Props { a, anchor }: Props)`",
                        ));
                    }
                    let part = self.project(subject.clone(), pat.ty, field.field.as_usize());
                    tests.extend(self.pattern_test(&field.pattern, &part, bindings)?);
                }
                Ok(tests.into_iter().reduce(|a, b| Expr::bin(Op::And, a, b)))
            }
            // `Circle(r) | Sphere(r)` (ADR 0124): each alternative's test, and
            // each binds the same names, as rustc checks. One each binds at
            // the same place, `s._0`, is that place; one bound elsewhere in
            // each, `(0, x) | (x, 0)`, is the place of the first that matched,
            // `p[0] === 0 ? p[1] : p[0]`, the last needing no test.
            PatKind::Or { pats } => {
                let mut alternatives = Vec::new();
                for p in pats {
                    let mut bound = Vec::new();
                    let test = self.pattern_test(p, subject, &mut bound)?;
                    alternatives.push((test, bound));
                }
                for binding in alternatives[0].1.clone() {
                    let places: Vec<(Option<Expr>, Expr)> = alternatives
                        .iter()
                        .map(|(test, bound)| {
                            let place = bound
                                .iter()
                                .find(|b| b.var == binding.var)
                                .map_or_else(|| binding.place.clone(), |b| b.place.clone());
                            (test.clone(), place)
                        })
                        .collect();
                    let same = places.iter().all(|(_, place)| same_place(place, &binding.place));
                    let chosen = binding.chosen || !same;
                    let place = if same {
                        binding.place.clone()
                    } else if binding.by_ref_mut {
                        // A choice of places can't be written through.
                        return Err(self.unsupported(
                            pat.span,
                            "a `ref mut` bound at another place in each alternative of a `|` pattern",
                        ));
                    } else {
                        let mut places = places.into_iter().rev();
                        let (_, last) = places.next().expect("a `|` pattern has alternatives");
                        places.fold(last, |rest, (test, place)| match test {
                            Some(test) => Expr::cond(test, place, rest),
                            None => place,
                        })
                    };
                    bindings.push(Binding {
                        place,
                        chosen,
                        ..binding
                    });
                }
                // `p | _` always matches.
                if alternatives.iter().any(|(test, _)| test.is_none()) {
                    return Ok(None);
                }
                Ok(alternatives
                    .into_iter()
                    .filter_map(|(test, _)| test)
                    .reduce(|a, b| Expr::bin(Op::Or, a, b)))
            }
            // `[first, .., last]` (ADR 0123): a slice's length, then each item,
            // `xs[0]` and `xs[xs.length - 1]`; an array's length is its type's,
            // so its last is `xs[2]`. What `..` binds is the items it stands
            // for, `xs.slice(1, xs.length - 1)`, a copy, as `&v[a..b]` is (ADR
            // 0063): one to write through can't be.
            PatKind::Array { prefix, slice, suffix } | PatKind::Slice { prefix, slice, suffix } => {
                let length = match pat.ty.peel_refs().kind() {
                    ty::Array(_, len) => len.try_to_target_usize(self.tcx).map(|n| n as i128),
                    _ => None,
                };
                let count = Expr::member(subject.clone(), "length");
                let (before, after) = (prefix.len() as i128, suffix.len() as i128);
                let mut tests = Vec::new();
                if matches!(pat.kind, PatKind::Slice { .. }) && !(slice.is_some() && before + after == 0) {
                    let op = if slice.is_some() { Op::Ge } else { Op::Eq };
                    tests.push(Expr::bin(op, count.clone(), Expr::int(before + after)));
                }
                // Where an item is, counted from the end: `xs[2]`, or `xs[xs.length - 1]`.
                let from_end = |k: i128| match length {
                    Some(n) => Expr::int(n - k),
                    None => Expr::bin(Op::Sub, count.clone(), Expr::int(k)),
                };
                for (i, item) in prefix.iter().enumerate() {
                    let place = Expr::index(subject.clone(), Expr::int(i as i128));
                    tests.extend(self.pattern_test(item, &place, bindings)?);
                }
                for (j, item) in suffix.iter().enumerate() {
                    let place = Expr::index(subject.clone(), from_end(after - j as i128));
                    tests.extend(self.pattern_test(item, &place, bindings)?);
                }
                if let Some(rest) = slice {
                    if let PatKind::Binding { mode, ty, .. } = &rest.kind
                        && (matches!(mode.0, ByRef::Yes(_, Mutability::Mut)) || ty.is_ref() && ty.is_mutable_ptr())
                    {
                        return Err(self.unsupported(rest.span, "a `&mut` to part of a slice"));
                    }
                    let mut range = vec![Expr::int(before)];
                    if after > 0 {
                        range.push(from_end(after));
                    }
                    let part = Expr::call(Expr::member(subject.clone(), "slice"), range);
                    tests.extend(self.pattern_test(rest, &part, bindings)?);
                }
                Ok(tests.into_iter().reduce(|a, b| Expr::bin(Op::And, a, b)))
            }
            _ => Err(self.unsupported(pat.span, "this pattern")),
        }
    }
}

/// Are two of a pattern's places the same one, as `s._0` of `Circle(r)` and
/// of `Sphere(r)` is? Only what such places are made of is compared, the
/// rest taken as different, which a choice of places stays right for.
pub(super) fn same_place(a: &Expr, b: &Expr) -> bool {
    use js::ExprKind as K;
    match (&a.kind, &b.kind) {
        (K::Var(x), K::Var(y)) | (K::Str(x), K::Str(y)) => x == y,
        (K::Num(x), K::Num(y)) => x == y,
        (K::BigInt(x), K::BigInt(y)) => x == y,
        (K::Member(x, m), K::Member(y, n)) | (K::OptionalMember(x, m), K::OptionalMember(y, n)) => {
            m == n && same_place(x, y)
        }
        (K::Index(x, i), K::Index(y, j)) => same_place(x, y) && same_place(i, j),
        (K::Binary(op, x, i), K::Binary(other, y, j)) => op == other && same_place(x, y) && same_place(i, j),
        (K::Call(f, xs), K::Call(g, ys)) => {
            same_place(f, g) && xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| same_place(x, y))
        }
        _ => false,
    }
}

/// A literal: a constant, or an array or object of them, which a default
/// in JS's destructuring is (ADR 0212).
fn is_literal(value: &Expr) -> bool {
    match &value.kind {
        js::ExprKind::Array(items) => items.iter().all(is_literal),
        js::ExprKind::Object(props) => props
            .iter()
            .all(|p| matches!(p, js::Prop::Field(_, v) if is_literal(v))),
        _ => value.is_constant(),
    }
}

/// Is `test` `x === "a"` of `subject`, a literal of the kind `kind`,
/// `typeof x === "string"`, tests?
fn says_kind(test: &Expr, subject: &Expr, kind: &Expr) -> bool {
    use js::ExprKind as K;
    let K::Binary(Op::Eq, left, literal) = &test.kind else {
        return false;
    };
    let of = match literal.kind {
        K::Str(_) => "string",
        K::Num(_) => "number",
        K::Bool(_) => "boolean",
        _ => return false,
    };
    same_place(left, subject)
        && matches!(&kind.kind, K::Binary(Op::Eq, typeof_, name)
            if matches!(&typeof_.kind, K::Unary(UnaryOp::Typeof, x) if same_place(x, subject))
                && matches!(&name.kind, K::Str(name) if name == of))
}

/// `test`, with the `const`s lowering its subject put last in `out`, from
/// `start` on, each read where `test` reads it, where that's once and
/// first, as JS read the `const`: `const match = value.a; match === "x"`
/// is `value.a === "x"`.
fn read_in_place(mut test: Expr, out: &mut Vec<Stmt>, start: usize) -> Expr {
    loop {
        let Some(Stmt {
            kind: StmtKind::Const(name, value),
            ..
        }) = out[start..].last()
        else {
            return test;
        };
        let mut reads = 0;
        test.visit_vars(&mut |var| reads += usize::from(var == name));
        if reads != 1 || !reads_first(&test, name) {
            return test;
        }
        let Some(read) = test.substitute(&|var| (var == name).then(|| value.clone())) else {
            return test;
        };
        out.pop();
        test = read;
    }
}

/// Is `name` what JS reads first of `e`, before anything it does?
fn reads_first(e: &Expr, name: &str) -> bool {
    use js::ExprKind as K;
    match &e.kind {
        K::Var(var) => var == name,
        K::Binary(_, first, _)
        | K::Member(first, _)
        | K::Index(first, _)
        | K::Unary(_, first)
        | K::Cond(first, _, _)
        | K::Call(first, _) => reads_first(first, name),
        _ => false,
    }
}

/// `a && b`, but `b` alone where it's `typeof x === "string"` after `x !=
/// null`, which a `typeof` of a string, a number or the like holds of no
/// `null`: `Some(s)` of an `Option<&Unknown>`, then `Kind::String(t)` of `s`.
fn and(a: Expr, b: Expr) -> Expr {
    use js::ExprKind as K;
    let not_null = |e: &Expr| js::presence_of(e).cloned();
    let typed = |e: &Expr| match &e.kind {
        K::Binary(Op::Eq, of, kind) => match (&of.kind, &kind.kind) {
            (K::Unary(UnaryOp::Typeof, x), K::Str(kind)) if kind != "object" && kind != "undefined" => {
                Some((**x).clone())
            }
            _ => None,
        },
        _ => None,
    };
    match (not_null(&a), typed(&b)) {
        (Some(x), Some(y)) if matches!((&x.kind, &y.kind), (K::Var(x), K::Var(y)) if x == y) => b,
        _ => Expr::bin(Op::And, a, b),
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Is `def_id` the crate's function, of no type parameters, whose body is
    /// a `match` of its one parameter, a fieldless enum, giving each variant
    /// its own name, `Section::Learn => "learn"`? A variant is its name in JS
    /// (ADR 0013), so it gives back what it's given (ADR 0264).
    pub(super) fn gives_own_name(&self, def_id: DefId) -> bool {
        let Some(body) = self.krate.bodies.get(&def_id) else {
            return false;
        };
        if self.tcx.generics_of(def_id).count() != 0 {
            return false;
        }
        let thir = &body.thir;
        let [param] = &thir.params.raw[..] else { return false };
        let Some(PatKind::Binding {
            var, subpattern: None, ..
        }) = param.pat.as_deref().map(|pat| &pat.kind)
        else {
            return false;
        };
        let ty::Adt(adt, _) = param.ty.kind() else { return false };
        if !adt.is_enum() || adt.variants().iter().any(|variant| !variant.fields.is_empty()) {
            return false;
        }
        let strip = |e| super::body_queries::strip(thir, e);
        let mut value = strip(body.expr);
        if let ExprKind::Block { block } = thir[value].kind {
            let block = &thir[block];
            let (true, Some(expr)) = (block.stmts.is_empty(), block.expr) else {
                return false;
            };
            value = strip(expr);
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
                                if text.as_str() == super::bindings::variant_name(self.tcx, adt.variant(variant_index)))))
        })
    }
}
