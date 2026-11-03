//! How an untagged enum's JS object type's class is told apart (ADR
//! 1062, amended, #555): a DOM node by its `nodeType`, a member its
//! `#[rust_js::test(nodeType = 9)]` names, the same in any realm and of no
//! global; any other class `instanceof` would tell refused where it's
//! tested, as `instanceof Blob` is `false` of another realm's `Blob` and
//! throws where the host has none.

use super::Kind;
use crate::lower::FnCx;
use crate::lower::bindings;
use crate::lower::recognition::Recognition;
use rustc_middle::ty::{AdtDef, GenericArgsRef, Ty, TyCtxt, VariantDef};

/// What tells a variant of a JS object type's class apart from the
/// others: a member its `#[rust_js::test(nodeType = 9)]` names, a DOM
/// node's, `node.nodeType === 9`, which is the same in any realm and needs
/// no global, where `instanceof Document` is `false` of a frame's
/// document and throws where the host has no `Document` (#555); else its
/// class. Its class is still what `js::class::<Document>()` is
/// (`class_value`).
pub(super) fn member_of_class(tcx: TyCtxt<'_>, kind: Kind) -> Kind {
    match kind {
        Kind::Class(name, Some(class)) => match bindings::test_member(tcx, class) {
            Some(Some((key, values))) => Kind::Member(key, Box::new(Kind::Constants(values))),
            _ => Kind::Class(name, Some(class)),
        },
        kind => kind,
    }
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Whether `variant`'s payload is a JS object type's class, told by
    /// `instanceof` or by the member it names (`member_of_class`).
    pub(super) fn holds_a_class(&self, variant: &VariantDef, args: GenericArgsRef<'tcx>) -> bool {
        (variant.fields.iter().next()).is_some_and(|field| {
            matches!(
                self.untagged_kind(field.ty(self.tcx, args).skip_normalization()),
                Some(Kind::Class(..))
            )
        })
    }

    /// The variants of `adt` a test of `variants` tells by what it is, or
    /// what it leaves out: of an `otherwise` none of one kind, each other.
    fn told_by_test<'v>(
        &self,
        adt: AdtDef<'tcx>,
        args: GenericArgsRef<'tcx>,
        variants: &[&'v VariantDef],
    ) -> Vec<&'v VariantDef>
    where
        'tcx: 'v,
    {
        let otherwise = |v: &VariantDef| bindings::is_otherwise(self.tcx, v.def_id);
        match variants.iter().any(|v| otherwise(v)) && self.otherwise_kind(adt, args).is_none() {
            true => adt.variants().iter().filter(|v| !otherwise(v)).collect(),
            false => variants.to_vec(),
        }
    }

    /// A variant a test of `variants` of `adt` would tell by `instanceof`
    /// of a JS object type's class, its own or one it leaves out, an
    /// object's `!(value instanceof Blob)`, and the class: refused, as
    /// `instanceof Blob` is `false` of a `Blob` of another realm, a frame's
    /// or a JSDOM window's, where Rust knows its variant, and throws a
    /// `ReferenceError` where the host has no `Blob` global (#555). Not
    /// where the class is what the others leave (`lone_class_variant`,
    /// ADR 1062), nor of one told by its member, a DOM node's `nodeType`
    /// (`member_of_class`), nor of std's `Map` and `Set`, which only the
    /// program makes, of ECMAScript's own globals.
    pub(super) fn untold_class(
        &self,
        adt: AdtDef<'tcx>,
        args: GenericArgsRef<'tcx>,
        variants: &[&VariantDef],
    ) -> Option<(String, String)> {
        if self.lone_class_variant(adt, args).is_some() {
            return None;
        }
        (self.told_by_test(adt, args, variants).into_iter()).find_map(|v| {
            let parts = self.untagged_parts(adt, args, v, false);
            let mut kinds = parts
                .iter()
                .flat_map(|(own, narrower)| std::iter::once(own).chain(narrower));
            kinds.find_map(|kind| match kind {
                Kind::Class(class, Some(_)) => Some((v.name.to_string(), class.clone())),
                _ => None,
            })
        })
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Refuses a test of `variants` of the untagged enum `ty` that would
    /// tell a class by `instanceof` (`untold_class`), where the function
    /// is: a clone's, an `==`'s or a drop's. Not a `match`'s arms', which
    /// it refuses itself once it knows they differ (`telling_arms`).
    pub(in crate::lower) fn refuse_untold(&self, ty: Ty<'tcx>, variants: &[&VariantDef]) {
        if !self.telling_arms.get()
            && let Some(refused) = self.untold_class(ty, variants)
        {
            self.tcx.dcx().span_err(self.tcx.def_span(self.item), refused);
        }
    }

    /// Why a test of the untagged enum `ty` is refused, where JS would tell
    /// a variant by `instanceof` of a JS object type's class
    /// (`untold_class`): `false` of another realm's object, and a
    /// `ReferenceError` where the host lacks the global (#555).
    pub(in crate::lower) fn untold_class(&self, ty: Ty<'tcx>, variants: &[&VariantDef]) -> Option<String> {
        let (adt, args) = self.untagged(ty)?;
        let recognition = self.recognition();
        // Two classes of one member, an `HTMLElement` and an `SVGElement`,
        // each `nodeType` 1, which `instanceof` alone told apart.
        if let Some((variant, (other, Some(both)))) = (recognition.told_by_test(adt, args, variants).into_iter())
            .find_map(|v| Some((v.name, self.untagged_alike(ty, v)?)))
        {
            return Some(format!(
                "`{variant}` and `{other}` are each {both}, which JS can't tell apart in an untagged enum in every \
                 realm, as `instanceof` is `false` of another realm's object and throws where the host lacks the \
                 class (ADR 1062, amended). Tell them apart in Rust: a tagged enum, whose tag says which it is"
            ));
        }
        let (variant, class) = recognition.untold_class(adt, args, variants)?;
        let name = self.tcx.item_name(adt.did());
        Some(format!(
            "JS can't tell `{name}::{variant}`, a `{class}`, from `{name}`'s other variants in every realm: \
             `instanceof {class}` is `false` of a `{class}` of another realm, a frame's or a JSDOM window's, \
             and throws a `ReferenceError` where the host has no `{class}` global; a DOM node is told by its \
             `nodeType`, but only where each node class in the enum has its own (ADR 1062, amended). \
             Tell them apart in Rust: a tagged enum, whose tag says which it is"
        ))
    }
}
