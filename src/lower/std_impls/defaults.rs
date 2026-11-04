//! `Default` where rust-js writes the implementation: a derived one, a
//! std type's, or a bound's dictionary's (ADR 0052). A hand-written one is
//! called instead.

use crate::js::{self, Expr, Prop, StmtKind};
use crate::lower::recognition::{self, StdItem, std_item};
use crate::lower::representation::Num;
use crate::lower::{FnCx, R, Shape, bindings, serde, std_types};
use rustc_hir as hir;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::{DefKind, Res};
use rustc_middle::traits::ImplSource;
use rustc_middle::ty::{self, Ty};
use rustc_span::def_id::DefId;
use rustc_span::{Span, Symbol};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A derived `Default` of an enum is its `#[default]` variant, which
    /// has no fields: the constructor the derived body names, `Mode::Off`.
    fn default_variant(&self, default: DefId, ty: Ty<'tcx>) -> Option<DefId> {
        let tr = ty::TraitRef::new(self.tcx, default, [self.tcx.erase_and_anonymize_regions(ty)]);
        let Ok(ImplSource::UserDefined(imp)) = self.tcx.codegen_select_candidate(self.typing_env.as_query_input(tr))
        else {
            return None;
        };
        let method = self.tcx.associated_item_def_ids(imp.impl_def_id)[0].as_local()?;
        let mut expr = self.tcx.hir_body_owned_by(method).value;
        while let hir::ExprKind::Block(hir::Block { expr: Some(inner), .. }, _) = expr.kind {
            expr = inner;
        }
        let hir::ExprKind::Path(ref path) = expr.kind else {
            return None;
        };
        match self.tcx.typeck(method).qpath_res(path, expr.hir_id) {
            Res::Def(DefKind::Ctor(..), ctor) => Some(ctor),
            _ => None,
        }
    }

    /// `Default::default()` of `ty`: `0`, `""`, `[]`, a struct of its fields'
    /// defaults, or a call of a hand-written `default`.
    pub(in crate::lower) fn default_value(&mut self, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let default = std_item(self.tcx, StdItem::Default);
        // What a bound gave this function for it, a type parameter's or
        // a where clause's of one made of them, `[Self::Item; N]: Default`:
        // its dictionary's.
        let tr = ty::TraitRef::new(self.tcx, default, [ty]);
        if let Some(dictionary) = self.evidence_for(tr) {
            return Ok(Expr::call(Expr::member(dictionary, "default"), Vec::new()));
        }
        if self.is_unknown(ty) {
            return Err(self.no_evidence(span, tr));
        }
        // A React element's, React's empty node (ADR 0192): the react
        // crate's `impl` has no JS of its own.
        if let ty::Adt(adt, _) = ty.kind()
            && self
                .tcx
                .get_attrs_by_path(adt.did(), &[Symbol::intern("rust_js"), Symbol::intern("jsx_element")])
                .next()
                .is_some()
        {
            return Ok(Expr::undefined());
        }
        // react's `Rest`: no props, `undefined`, which a spread spreads
        // nothing of, and a JSX attribute isn't written for (ADR 0195).
        if bindings::is_rest(self.tcx, ty) {
            return Ok(Expr::undefined());
        }
        if self.has_user_impl(default, ty) {
            let method = self.tcx.associated_item_def_ids(default)[0];
            return self.impl_call(method, self.args_of(default, ty), Vec::new(), span);
        }
        self.check_value_ty(ty, span)?;
        // serde_json's `Value` is `Null` (ADR 0083).
        if self.json_type(ty) == Some(serde::Json::Value) {
            return Ok(Expr::str("Null"));
        }
        let std = |item: StdItem| self.is_std_type(ty, item);
        Ok(match ty.kind() {
            _ if let Some(num) = Num::of(ty) => num.literal(0),
            ty::Bool => Expr::bool(false),
            ty::Char => Expr::str("\0"),
            // `&str`'s is `""`, a slice's `[]`, and an array's its items'.
            ty::Ref(_, inner, _) if inner.is_str() => Expr::str(""),
            ty::Ref(_, inner, _) if inner.is_slice() => Expr::array(Vec::new()),
            // A `Box<str>`'s and a `Box<[T]>`'s, and `PhantomData`, which holds nothing.
            ty::Str => Expr::str(""),
            ty::Slice(_) => Expr::array(Vec::new()),
            _ if self.is_lang_adt(ty, LangItem::PhantomData) => Expr::undefined(),
            ty::Array(item, len) => {
                let len = len
                    .try_to_target_usize(self.tcx)
                    .ok_or_else(|| self.unsupported(span, &format!("`Default` of `{ty}`")))?;
                Expr::array((0..len).map(|_| self.default_value(*item, span)).collect::<R<_>>()?)
            }
            _ if ty.is_unit() || self.option_of(ty).is_some() => Expr::undefined(),
            _ if self.is_lang_adt(ty, LangItem::String) => Expr::str(""),
            _ if self.is_vec_like(ty) => Expr::array(Vec::new()),
            ty::Adt(_, args) if self.is_map(ty) => {
                let class = self.map_class(self.is_set(ty), self.is_sorted(ty), args.types().next(), span)?;
                Expr::new_(class, Vec::new())
            }
            ty::Adt(_, args) if self.counted_rc(ty).is_some() => {
                Self::new_rc(self.default_value(args.type_at(0), span)?)
            }
            ty::Adt(_, args) if ty.is_box() || self.is_rc(ty) => self.default_value(args.type_at(0), span)?,
            ty::Adt(..) if recognition::weak_pointee(self.tcx, ty).is_some() => self.weak_to_nothing(ty),
            ty::Adt(..) if std(StdItem::OnceCell) => {
                Expr::object(vec![Prop::Field(js::HELD.into(), Expr::undefined())])
            }
            // `LazyCell::new(T::default)` (ADR 0318).
            ty::Adt(_, args) if std(StdItem::LazyCell) => {
                let made = self.default_value(args.type_at(0), span)?;
                let init = Expr::arrow(vec![], vec![StmtKind::Return(Some(made)).at(js::Span::NONE)]);
                Expr::object(vec![Prop::Field("init".into(), init)])
            }
            // A `RefCell` of an object, the object (ADR 0863).
            ty::Adt(_, args) if std(StdItem::RefCell) && self.is_value_cell(ty) => {
                self.default_value(args.type_at(0), span)?
            }
            ty::Adt(_, args) if std(StdItem::Cell) || std(StdItem::RefCell) || std(StdItem::Atomic) => Expr::object(
                vec![Prop::Field(js::HELD.into(), self.default_value(args.type_at(0), span)?)],
            ),
            ty::Adt(adt, _) if adt.is_enum() && !self.is_std(adt.did()) => {
                let variant = self
                    .default_variant(default, ty)
                    .ok_or_else(|| self.unsupported(span, &format!("`Default` of `{ty}`")))?;
                self.unit_variant(adt.did(), adt.variant_with_ctor_id(variant))
            }
            // A std hasher's, as its `new()` (ADR 0463).
            ty::Adt(..) if let Some(which) = self.recognition().std_hasher(ty) => {
                std_types::hash::new_hasher(which, Vec::new())
            }
            // A range is a struct as any is (ADR 0129), and std's `Default`
            // of one, `Range`'s and `RangeFull`'s, is derived: its fields'
            // defaults, `{ start: 0, end: 0 }`, and `..`'s `undefined`, as
            // below (#639).
            ty::Adt(adt, _) if self.is_std(adt.did()) && self.range_kind(ty).is_none() => {
                return Err(self.unsupported(span, &format!("`Default` of `{ty}`")));
            }
            ty::Adt(adt, _) if adt.is_struct() && adt.non_enum_variant().fields.is_empty() => Expr::undefined(),
            // A `#[repr(transparent)]` struct's is its field's (ADR 0584).
            ty::Adt(adt, args) if let Some(at) = recognition::transparent_field(self.tcx, *adt) => {
                let field = &adt.non_enum_variant().fields[rustc_abi::FieldIdx::from_usize(at)];
                self.default_value(field.ty(self.tcx, args).skip_normalization(), span)?
            }
            _ => match self.shape(ty) {
                // A flattened field's defaults are the struct's own (ADR 0204).
                Shape::Object(fields) => {
                    let made = Expr::object(
                        fields
                            .into_iter()
                            .enumerate()
                            .map(|(i, (key, t))| {
                                let value = self.field_default(ty, i, t, span)?;
                                let value = self.holding(value, self.struct_field_def(ty, i), t);
                                // One a symbol names, `[SYMBOL]: ..` (ADR 1051).
                                Ok(match key.symbol_item() {
                                    Some((item, at)) => Prop::Keyed(self.field_symbol(item, at), value),
                                    None => Prop::Field(key.text().to_owned(), value),
                                })
                            })
                            .collect::<R<_>>()?,
                    );
                    self.flattened_object(made, ty)
                }
                Shape::Array(tys) => {
                    Expr::array(tys.into_iter().map(|t| self.default_value(t, span)).collect::<R<_>>()?)
                }
                Shape::Other => return Err(self.unsupported(span, &format!("`Default` of `{ty}`"))),
            },
        })
    }
}
