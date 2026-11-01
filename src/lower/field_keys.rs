//! A struct's fields' keys as JS reads and writes them: a name, `base.key`,
//! or a symbol `#[rust_js::name(SYMBOL)]` names it by, `base[SYMBOL]`
//! (ADR 1051). `bindings::field_key` says which; this reads it.

use super::{FnCx, bindings};
use crate::js::Expr;

use rustc_middle::ty::Ty;
use rustc_span::Span;
use rustc_span::def_id::DefId;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The symbol `item` holds, a field's key (ADR 1051): a binding's static
    /// as JS's global it names, `Symbol.asyncIterator`, the crate's or a
    /// library's as its item's read, a thread-local's made by then, as
    /// the path at `named` names it.
    pub(super) fn field_symbol(&self, item: DefId, named: Span) -> Expr {
        match self.tcx.is_foreign_item(item) {
            true => self.binding_ref(&bindings::js_name(self.tcx, item), named),
            false => self.item_read(item, named),
        }
    }

    /// `base.key`, a struct's field, or `base[SYMBOL]` of one a symbol
    /// names, that symbol as its item is read where it's used, a
    /// thread-local's made by then (ADR 1051).
    pub(super) fn field_member(&self, base: Expr, key: &bindings::FieldKey) -> Expr {
        match key.symbol_item() {
            Some((item, span)) => Expr::index(base, self.field_symbol(item, span)),
            None => Expr::member(base, key.text()),
        }
    }

    /// A variant's fields' keys, as a struct's are listed: each its text,
    /// as a symbol names no variant's field (`validate`, ADR 1051).
    pub(super) fn variant_keys(fields: Vec<(String, Ty<'tcx>)>) -> Vec<(bindings::FieldKey, Ty<'tcx>)> {
        (fields.into_iter())
            .map(|(name, ty)| (bindings::FieldKey::named(name), ty))
            .collect()
    }

    /// A struct's fields by their names, where `validate` lets a symbol
    /// name none: a props struct's, or one that derives or implements
    /// `Clone`, `PartialEq`, `PartialOrd`, `Debug`, `Hash` or serde's
    /// traits (ADR 1051).
    pub(super) fn named_fields(fields: Vec<(bindings::FieldKey, Ty<'tcx>)>) -> Vec<(String, Ty<'tcx>)> {
        (fields.into_iter())
            .map(|(key, ty)| (key.text().to_owned(), ty))
            .collect()
    }
}
