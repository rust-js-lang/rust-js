//! A struct, a variant or a tuple struct made, by its fields or a struct
//! update, `..base` (ADRs 0013, 0033, 0098), and a constructor taken as a
//! value (ADR 0125).

use super::bindings;
use super::fn_def;
use super::recognition::{StdItem, std_item};
use super::representation::ordering_value;
use super::{FnCx, R, Shape, assembled};
use crate::js::{self, Expr, Prop, StmtKind};
use rustc_abi::VariantIdx;
use rustc_hir::def::CtorKind;
use rustc_middle::thir::{self as thir, ExprId};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;

/// What MIR tells of the value a struct literal's field is given, which a
/// nullable field stores by (ADRs 0275, 0810).
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(super) enum Given {
    /// What may be `undefined`.
    #[default]
    Unknown,
    /// `null` or a value already (`nulled`).
    Nulled,
    /// A `None` the literal writes, `S { label: None }`'s, or a
    /// `Cell::new(None)`'s: an optional field's left out.
    WrittenNone,
}

/// How the struct literal written at `span`, `P {\n x: 1, .. }`, is
/// written, which its object is written as (ADR 0835): across lines where
/// its `{` ends a line, as rustfmt writes one too long for a line, and as
/// Prettier tells an object a person wrote across lines. On one line
/// otherwise, and of a macro's or a desugaring's, a range's, `a..b`.
fn literal_layout(tcx: ty::TyCtxt<'_>, span: Span) -> js::ObjectLayout {
    let code = match span.from_expansion() {
        true => String::new(),
        false => tcx.sess.source_map().span_to_snippet(span).unwrap_or_default(),
    };
    let after = code.find('{').map_or("", |brace| &code[brace + 1..]);
    js::ObjectLayout {
        lines: after[..after.len() - after.trim_start().len()].contains('\n'),
        commented: Vec::new(),
    }
}

/// A `const`'s or a `static`'s object, of the value rustc computed, as its
/// struct literal is written, as one written in place is (ADR 0835); its
/// objects within, as rustc's value has no literal of theirs, on one line
/// where they fit.
pub(super) fn const_object(tcx: ty::TyCtxt<'_>, def_id: rustc_span::def_id::LocalDefId, mut value: Expr) -> Expr {
    if let js::ExprKind::Object(_, layout) = &mut value.kind
        && let Some(body) = tcx.hir_maybe_body_owned_by(def_id)
        && let rustc_hir::ExprKind::Struct(..) = body.value.kind
    {
        *layout = literal_layout(tcx, body.value.span);
    }
    value
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A struct literal: `{ x: 1, y: 2 }`, or `[1, 2]` for a tuple struct.
    /// `made`, a struct with a flattened field's object, as JS has it: the
    /// flattened field's own fields, of a struct made there, or its spread,
    /// `{ ...item, severity }`, where it's declared (ADR 0204).
    pub(super) fn flattened_object(&self, made: Expr, ty: Ty<'tcx>) -> Expr {
        let js::ExprKind::Object(props, layout) = made.kind else {
            return made;
        };
        let Shape::Object(names) = self.shape(ty) else {
            unreachable!("an object of a struct's fields")
        };
        // Each field by its name, as the object lists them as written.
        let flattened = |name: &str| {
            (names.iter().position(|(n, _)| n == name)).is_some_and(|i| bindings::is_flatten_field(self.tcx, ty, i))
        };
        let mut flat = Vec::new();
        for prop in props {
            match prop {
                Prop::Field(name, value) if flattened(&name) => match value.kind {
                    js::ExprKind::Object(inner, ..) => flat.extend(inner),
                    js::ExprKind::Undefined => {}
                    _ => flat.push(Prop::Spread(value)),
                },
                prop => flat.push(prop),
            }
        }
        Expr {
            facts: js::Facts::NONE,
            kind: js::ExprKind::Object(flat, layout),
            span: made.span,
        }
    }

    /// The order the fields of a struct or a variant made at `span` are
    /// given in, each an index of its declared fields: as written, then
    /// those its `..base` fills, as declared, the order Rust evaluates them
    /// in, which its object lists them in (ADR 0674). `P { c, a, ..base }`
    /// of `struct P { a, b, c }` is `[2, 0, 1]`. As declared, of a tuple
    /// struct's or variant's, an array of them in place, and where THIR
    /// says nothing of it.
    pub(super) fn written_order(&self, adt: &ty::AdtDef<'tcx>, variant_index: VariantIdx, span: Span) -> Vec<usize> {
        let variant = adt.variant(variant_index);
        let declared: Vec<usize> = (0..variant.fields.len()).collect();
        if variant.ctor_kind().is_some() || declared.len() < 2 {
            return declared;
        }
        let mut orders = (self.frame.thir.exprs.iter()).filter_map(|e| match &e.kind {
            thir::ExprKind::Adt(made)
                if e.span == span && made.adt_def == *adt && made.variant_index == variant_index =>
            {
                let written: Vec<usize> = made.fields.iter().map(|f| f.name.as_usize()).collect();
                let filled = declared.iter().filter(|i| !written.contains(i));
                Some(written.iter().chain(filled).copied().collect::<Vec<usize>>())
            }
            _ => None,
        });
        match (orders.next(), orders.next()) {
            (Some(order), None) if order.len() == declared.len() => order,
            // Two at one place, of a macro, may be written two ways.
            _ => declared,
        }
    }

    /// The fields of the struct literal made at `span`, by their declared
    /// index, a comment is written on: after its value on its line, `v:
    /// Cell::new(None), // why`, in it, or on lines of their own right
    /// before it. A `None`'s keeps its key there, the comment's place (ADR
    /// 0280, amended).
    fn commented_fields(&self, adt: &ty::AdtDef<'tcx>, variant_index: VariantIdx, span: Span) -> Vec<usize> {
        use rustc_lexer::TokenKind;
        let Some(made) = (self.frame.thir.exprs.iter()).find_map(|e| match &e.kind {
            thir::ExprKind::Adt(made)
                if e.span == span && made.adt_def == *adt && made.variant_index == variant_index =>
            {
                Some(made)
            }
            _ => None,
        }) else {
            return Vec::new();
        };
        if adt.variant(variant_index).ctor_kind().is_some() || span.from_expansion() {
            return Vec::new();
        }
        let Ok(text) = self.tcx.sess.source_map().span_to_snippet(span) else {
            return Vec::new();
        };
        let at = |s: Span| (s.lo().0 - span.lo().0) as usize..(s.hi().0 - span.lo().0) as usize;
        let mut fields: Vec<_> = (made.fields.iter())
            .map(|f| (f.name, self.frame.thir[f.expr].span))
            .filter(|&(_, s)| !s.from_expansion() && span.contains(s))
            .map(|(name, s)| (name, at(s)))
            .collect();
        fields.sort_by_key(|(_, range)| range.start);
        let commented = |piece: &str| {
            rustc_lexer::tokenize(piece, rustc_lexer::FrontmatterAllowed::No)
                .any(|t| matches!(t.kind, TokenKind::LineComment { .. } | TokenKind::BlockComment { .. }))
        };
        // Its `{`, before its first field's value, and its `}`.
        let open = (fields.first())
            .and_then(|(_, first)| text[..first.start].rfind('{'))
            .map_or(0, |brace| brace + 1);
        let close = text.rfind('}').unwrap_or(text.len());
        (0..fields.len())
            .filter(|&k| {
                let value = &fields[k].1;
                // From the line after what's before it, to its line's end.
                let before = if k == 0 { open } else { fields[k - 1].1.end };
                let next = fields.get(k + 1).map_or(close, |(_, next)| next.start);
                let (Some(lead), Some(tail)) = (text.get(before..value.start), text.get(value.end..next)) else {
                    return false;
                };
                let start = lead.find('\n').map_or(value.start, |n| before + n);
                let end = tail.find('\n').map_or(next, |n| value.end + n);
                text.get(start..end).is_some_and(commented)
            })
            .map(|k| fields[k].0.as_usize())
            .collect()
    }

    /// How the struct or variant literal made at `span` is written, which
    /// its object is written as (`literal_layout`); on one line where THIR
    /// has no literal of it there, of a tuple struct or variant.
    pub(super) fn object_layout(
        &self,
        adt: &ty::AdtDef<'tcx>,
        variant_index: VariantIdx,
        span: Span,
    ) -> js::ObjectLayout {
        let written = adt.variant(variant_index).ctor_kind().is_none()
            && (self.frame.thir.exprs.iter()).any(|e| {
                matches!(&e.kind, thir::ExprKind::Adt(made)
                    if e.span == span && made.adt_def == *adt && made.variant_index == variant_index)
            });
        match written {
            true => literal_layout(self.tcx, span),
            false => js::ObjectLayout::default(),
        }
    }

    /// A struct's or a variant's value, of its fields' values in the order
    /// they're declared: as `adt` makes a literal without a base, which is
    /// how MIR makes each (ADR 0364). An object of them lists them in
    /// `order`, each a declared field's index, as written
    /// (`written_order`), or as declared where it's `None`. Each field is
    /// `given` what MIR tells of its value, nothing where it's left out.
    pub(super) fn adt_value(
        &mut self,
        ty: Ty<'tcx>,
        variant_index: VariantIdx,
        fields: Vec<Expr>,
        given: &[Given],
        order: Option<&[usize]>,
        span: Span,
    ) -> R<Expr> {
        let ty::Adt(adt_def, args) = *ty.kind() else {
            unreachable!("an ADT's value")
        };
        let variant = adt_def.variant(variant_index);
        // A write's result's `Ok` is nothing (ADRs 0054, 0132); its error is
        // thrown, a `fmt::Error` (ADR 0187) or the `io::Error` (ADR 0462).
        if self.is_write_result(ty) {
            return Ok(
                match self
                    .tcx
                    .is_lang_item(variant.def_id, rustc_hir::attrs::lang_items::LangItem::ResultOk)
                {
                    true => Expr::undefined(),
                    _ if self.is_fmt_result(ty) => Expr::call(Expr::helper("$fmtError"), Vec::new()),
                    _ => Expr::call_helper("$ioThrow", fields),
                },
            );
        }
        // A `ManuallyDrop`, or what `pin!` pins, is the value it holds; a
        // `#[repr(transparent)]` struct its field, beside a marker's (ADR 0584).
        if self.transparent(ty).is_some() {
            let at = super::recognition::transparent_field(self.tcx, adt_def).unwrap_or(0);
            if let Some(value) = fields.get(at) {
                return Ok(value.clone());
            }
        }
        // The registry's symbol of its key (ADR 0634).
        if self.recognition().is_symbol_for(ty)
            && let Some(key) = fields.first()
        {
            return Ok(super::representation::symbol_for(key.clone()));
        }
        if let Some(inner) = self.option_of(ty) {
            return Ok(match fields.into_iter().next() {
                Some(value) if self.boxed_payload(inner) => self.some(value),
                Some(value) => value,
                None => Expr::undefined(),
            });
        }
        if self.untagged(ty).is_some()
            && let Some(value) = super::untagged::unit_value(self.tcx, variant)
        {
            return Ok(value);
        }
        if self.untagged(ty).is_some() || bindings::is_tagged_otherwise(self.tcx, adt_def.did(), variant) {
            let Ok([field]) = <[Expr; 1]>::try_from(fields) else {
                return Err(self.unsupported(span, "this untagged enum's variant"));
            };
            return Ok(field);
        }
        if let Some(n) = ordering_value(self.tcx, adt_def.did(), variant.name) {
            return Ok(Expr::int(n));
        }
        if adt_def.is_enum() && variant.fields.is_empty() {
            return Ok(self.unit_variant(adt_def.did(), variant));
        }
        if variant.ctor_kind() == Some(CtorKind::Const) {
            return Ok(bindings::unit_name(self.tcx, adt_def.did()).map_or_else(Expr::undefined, Expr::str));
        }
        let mut fields = fields;
        // A `#[rust_js::nullable]` field's `None` is `null` (ADR 0275);
        // what's `nulled`, `null` or a value already, is as it is.
        // A `Cell`'s, what it holds, which the field is (ADR 0772). An
        // optional one's `None` the literal writes is left out, and what may
        // be `None` otherwise is stored as any nullable field's is: never
        // `{ label: undefined }` (ADR 0810, amended).
        for (i, (field, value)) in variant.fields.iter().zip(&mut fields).enumerate() {
            if !bindings::is_nullable(self.tcx, field) {
                continue;
            }
            let given = given.get(i).copied().unwrap_or_default();
            let field_ty = field.ty(self.tcx, args).skip_normalization();
            // A `Vec` of them, whose items are, is made as it's given (ADR
            // 0586, amended).
            if bindings::nullable_option(self.tcx, field_ty).is_none() {
                continue;
            }
            // And a `RefCell`'s that's the property it holds (ADR 0863).
            let cell = self.is_std_type(field_ty, StdItem::Cell) || self.plain_ref_cell(field_ty);
            let made = std::mem::replace(value, Expr::undefined());
            let held = match cell {
                true => Expr::held(made.clone()),
                false => made.clone(),
            };
            if bindings::is_optional(self.tcx, field)
                && (given == Given::WrittenNone
                    || matches!(held.kind, js::ExprKind::Undefined | js::ExprKind::GivenUndefined))
            {
                // An `Option`'s the `None` it is, a read of what's only
                // `None`; a cell's as it's made, which a drop may read.
                *value = match self.option_of(field_ty) {
                    Some(_) => Expr::undefined(),
                    None => made,
                };
                continue;
            }
            let stored = js::Storage::NullForNone.written(held, given == Given::Nulled);
            *value = match cell {
                true => Expr::object(vec![Prop::Field(js::HELD.into(), stored)]),
                false => stored,
            };
        }
        let tag = (adt_def.is_enum()).then(|| {
            (
                bindings::tag_key(self.tcx, adt_def.did()),
                bindings::variant_tag(self.tcx, variant),
            )
        });
        let shape = match tag {
            Some(_) => Shape::Object(self.variant_fields(variant, args)),
            None => self.shape(ty),
        };
        // The keys a comment is written on, whose `undefined`, a `None`'s,
        // is kept: the comment's place (ADR 0280, amended).
        let mut commented = Vec::new();
        let (shape, items) = match shape {
            // A `Cell` in an object's field is what it holds (ADR 0288).
            Shape::Object(names) => {
                commented = (self.commented_fields(&adt_def, variant_index, span).into_iter())
                    .filter_map(|i| names.get(i).map(|(name, _)| name.clone()))
                    .collect();
                let mut items: Vec<Option<Expr>> = (names.iter().zip(fields))
                    .map(|(&(_, field_ty), value)| Some(self.holding(value, field_ty)))
                    .collect();
                let declared: Vec<usize> = (0..names.len()).collect();
                let order = order.unwrap_or(&declared);
                let names = order.iter().map(|&i| names[i].clone()).collect();
                let items = (order.iter())
                    .map(|&i| items[i].take().expect("each field once"))
                    .collect();
                (Shape::Object(names), items)
            }
            Shape::Array(_) => (shape, fields),
            Shape::Other => return Err(self.unsupported(span, &format!("a `{ty}`"))),
        };
        let mut made = assembled(shape, tag, items);
        if let js::ExprKind::Object(_, layout) = &mut made.kind {
            *layout = js::ObjectLayout {
                commented,
                ..self.object_layout(&adt_def, variant_index, span)
            };
        }
        Ok(made)
    }

    /// The fields of `ty`, a struct whose `Default` is derived, when `base`
    /// is its `Default::default()`: what an update's base is made of.
    pub(super) fn derived_default_fields(&self, base: ExprId, ty: Ty<'tcx>) -> Option<Vec<(String, Ty<'tcx>)>> {
        let thir::ExprKind::Call { fun, args, .. } =
            &self.frame.thir[super::body_queries::strip(self.frame.thir, base)].kind
        else {
            return None;
        };
        let default = std_item(self.tcx, StdItem::Default);
        let (id, _) = fn_def(self.frame.thir[*fun].ty)?;
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
    /// where making it does nothing, as a `ReactNode`'s does (ADR 0201). And
    /// whether one the update gives is made still, for its effects.
    pub(super) fn update_default(
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

    /// Is `ty` a type parameter this function bounds by react's `ReactNode`? Each
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
            count => (0..count).map(js::positional).collect(),
        };
        let items: Vec<Expr> = params.iter().map(|name| Expr::var(name)).collect();
        // A `#[repr(transparent)]` struct is its field, as `adt_value` makes
        // one (ADR 0584): `.map(Wrap)` gives what it's given.
        let transparent = self
            .transparent(ty)
            .is_some()
            .then(|| super::recognition::transparent_field(self.tcx, adt_def).unwrap_or(0));
        let value = if let Some(at) = transparent
            && at < items.len()
        {
            items[at].clone()
        } else if self.recognition().is_symbol_for(ty) {
            // The registry's symbol of its key, `Symbol.for(value)` (ADR 0634).
            let [key] = <[Expr; 1]>::try_from(items).map_err(|_| self.unsupported(span, "this constructor"))?;
            super::representation::symbol_for(key)
        } else if self.untagged(ty).is_some() || bindings::is_tagged_otherwise(self.tcx, adt_def.did(), variant) {
            // An untagged enum's variant is its payload (ADR 0214), and a
            // discriminated union's `otherwise` the object it holds (ADR 0284).
            let [item] = <[Expr; 1]>::try_from(items).map_err(|_| self.unsupported(span, "this constructor"))?;
            item
        } else if let Some(inner) = self.option_of(ty) {
            // `Some`: the value, as `Some(x)` is `x` (ADR 0030), or boxed where
            // it could look like `None` (ADR 0051).
            let [item] = <[Expr; 1]>::try_from(items).map_err(|_| self.unsupported(span, "this constructor"))?;
            if self.boxed_payload(inner) {
                self.some(item)
            } else {
                item
            }
        } else if self.is_fmt_result(ty) {
            return Err(self.unsupported(span, "this constructor as a value"));
        } else {
            let tag = (adt_def.is_enum()).then(|| {
                (
                    bindings::tag_key(self.tcx, adt_def.did()),
                    bindings::variant_tag(self.tcx, variant),
                )
            });
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
