//! What std lends a `&mut` to from inside its receiver's storage, and where
//! in the receiver it is, rustc's layout of it (#1474, #1478): one answer,
//! which the lowering's handles on those places read and the crate's
//! analysis of which type facts generic code asks reads too.

use super::*;
use rustc_abi::{FieldIdx, VariantIdx};
use rustc_middle::ty::TypeVisitableExt;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// What std's `def_id` lends a `&mut` to from inside a `receiver`, where
    /// the lowering makes its place a handle the pointer machinery reads
    /// (`lent_place`): a `OnceCell`'s `get_mut`, a `LazyCell`'s `force_mut`,
    /// a `Peekable`'s `peek_mut`, a `Result`'s `as_mut`, of either variant,
    /// and its `iter_mut`, of its `Ok`. `None` of another call.
    pub(in crate::lower) fn lends_mut_of(&self, def_id: DefId, receiver: Ty<'tcx>) -> Option<&'static [Lend]> {
        let name = self.tcx.item_name(def_id);
        let result = self.is_std_type(receiver, StdItem::Result);
        match name.as_str() {
            "get_mut" if self.is_std_type(receiver, StdItem::OnceCell) && !self.is_lock_cell(receiver) => {
                Some(&[Lend::OnceCell])
            }
            "force_mut" if self.is_std_type(receiver, StdItem::LazyCell) && !self.is_lock_cell(receiver) => {
                Some(&[Lend::LazyCell])
            }
            "peek_mut" if self.is_peekable(receiver) => Some(&[Lend::Peekable]),
            "as_mut" if result => Some(&[Lend::Ok, Lend::Err]),
            "iter_mut" if result => Some(&[Lend::Ok]),
            _ => None,
        }
    }

    /// The byte offset in a `receiver`, of no type parameter, of what std
    /// lends of it, `lend`, rustc's layout for wasm32: a `OnceCell`'s value
    /// past its `Option`'s tag, a `LazyCell`'s past its state's, a
    /// `Peekable`'s past both its `Option`s', a `Result`'s variant's field.
    pub(in crate::lower) fn lend_offset(&self, receiver: Ty<'tcx>, lend: Lend) -> Option<u64> {
        let some = |option: Ty<'tcx>| self.variant_field_offset(option, VariantIdx::from_u32(1), FieldIdx::ZERO);
        match lend {
            Lend::OnceCell => {
                let (option, at) = self.field_path(receiver, &["inner", "value"])?;
                Some(at + some(option)?)
            }
            Lend::LazyCell => {
                let (state, at) = self.field_path(receiver, &["state", "value"])?;
                let ty::Adt(adt, _) = state.kind() else {
                    return None;
                };
                let init = (adt.variants().iter_enumerated())
                    .find(|(_, v)| v.name.as_str() == "Init")?
                    .0;
                Some(at + self.variant_field_offset(state, init, FieldIdx::ZERO)?)
            }
            Lend::Peekable => {
                let (outer, at) = self.field_path(receiver, &["peeked"])?;
                let inner = self.option_of(outer)?;
                Some(at + some(outer)? + some(inner)?)
            }
            Lend::Ok => self.variant_field_offset(receiver, VariantIdx::from_u32(0), FieldIdx::ZERO),
            Lend::Err => self.variant_field_offset(receiver, VariantIdx::from_u32(1), FieldIdx::ZERO),
        }
    }

    /// The type at the end of `path`, struct fields by name from `ty`, and
    /// its byte offset in `ty`.
    pub(in crate::lower) fn field_path(&self, mut ty: Ty<'tcx>, path: &[&str]) -> Option<(Ty<'tcx>, u64)> {
        let mut at = 0;
        for name in path {
            let ty::Adt(adt, args) = *ty.kind() else {
                return None;
            };
            let (i, field) =
                (adt.non_enum_variant().fields.iter_enumerated()).find(|(_, f)| f.name.as_str() == *name)?;
            at += self.laid_out(ty)?.fields.offset(i.as_usize()).bytes();
            ty = self
                .tcx
                .erase_and_anonymize_regions(field.ty(self.tcx, args).skip_normalization());
        }
        Some((ty, at))
    }

    /// The byte offset of field `field` of variant `variant` of `ty`, an
    /// enum, rustc's layout of that variant; `None` of a type its caller
    /// lays out.
    pub(in crate::lower) fn variant_field_offset(
        &self,
        ty: Ty<'tcx>,
        variant: VariantIdx,
        field: FieldIdx,
    ) -> Option<u64> {
        let layout = self.laid_out(ty)?;
        let cx = rustc_middle::ty::layout::LayoutCx::new(self.tcx, ty::TypingEnv::fully_monomorphized());
        Some(layout.for_variant(&cx, variant).fields.offset(field.as_usize()).bytes())
    }

    /// `ty`'s layout for wasm32, the target rust-js checks programs for (ADR
    /// 0090): what `size_of`, `align_of` and plain data's offsets are. Of a
    /// type without parameters, as codegen asks: 1.98 finds an `async fn`'s
    /// future too generic to lay out otherwise (ADR 0109).
    /// None of a type with a type parameter, which its caller lays out.
    pub(in crate::lower) fn laid_out(&self, ty: Ty<'tcx>) -> Option<rustc_middle::ty::layout::TyAndLayout<'tcx>> {
        if ty.has_param() {
            return None;
        }
        self.tcx
            .layout_of(ty::TypingEnv::fully_monomorphized().as_query_input(ty))
            .ok()
    }
}
