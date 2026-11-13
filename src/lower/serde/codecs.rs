//! Where a type's codec comes from: the impl of serde's `Serialize` or
//! `Deserialize` rustc selects for it, its function as it's called, and the
//! writers and readers generic code is given for its type parameters (ADRs
//! 0080, 0081). Split from serde.rs by concept (#1334).

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The function that writes a `ty` value as JSON, the derived
    /// `Serialize::serialize` of the crate's own type, if it has one.
    pub(super) fn serialize_fn(&self, ty: Ty<'tcx>) -> Option<DefId> {
        self.codec_fn(ty, true)
    }

    /// A type's `serialize`, or `deserialize`: of the crate's own impl, or of
    /// one a library of it exports (ADR 0100), the impl rustc selects for
    /// `ty` (`codec_impl`).
    pub(in crate::lower) fn codec_fn(&self, ty: Ty<'tcx>, serialize: bool) -> Option<DefId> {
        let (imp, _) = self.codec_impl(ty, serialize)?;
        let method = self.tcx.associated_item_def_ids(imp)[0];
        let foreign = imp.krate != rustc_span::def_id::LOCAL_CRATE;
        (!foreign || self.krate.foreign.item(method).is_some()).then_some(method)
    }

    /// The impl of serde's `Serialize`, or `Deserialize`, that rustc selects
    /// for `ty`, an instance of the crate's own type or a library's, and its
    /// arguments: `Wrap<String>`'s own where `Wrap<u32>` has another, never
    /// the first impl of its type (#1550). serde's trait is named by the
    /// type's impls of it, as rust-js has no other name of it.
    pub(super) fn codec_impl(&self, ty: Ty<'tcx>, serialize: bool) -> Option<(DefId, ty::GenericArgsRef<'tcx>)> {
        let ty::Adt(adt, _) = ty.kind() else {
            return None;
        };
        let impls = match self.krate.foreign.in_library(adt.did()) {
            true => self.tcx.trait_impls_in_crate(adt.did().krate),
            false => self.krate.trait_impls,
        };
        let trait_id = impls.iter().find_map(|&imp| {
            let tr = self.tcx.impl_trait_ref(imp).instantiate_identity().skip_normalization();
            let same = matches!(tr.self_ty().kind(), ty::Adt(a, _) if a.did() == adt.did());
            (same && serde_trait(self.tcx, tr.def_id) == Some(serialize)).then_some(tr.def_id)
        })?;
        // `Self`, and `Deserialize`'s `'de`, which selects no impl.
        let args = ty::GenericArgs::for_item(self.tcx, trait_id, |param, _| match param.kind {
            ty::GenericParamDefKind::Lifetime => self.tcx.lifetimes.re_erased.into(),
            _ => ty.into(),
        });
        let tr = ty::TraitRef::new_from_args(self.tcx, trait_id, args);
        let (imp, args) = crate::lower::traits::evidence_impl(self.tcx, self.typing_env, tr)?;
        impls.contains(&imp).then_some((imp, args))
    }

    /// The types a codec of `ty`, the impl selected for it, takes a writer or
    /// reader of: what that impl's type parameters are at `ty`, in the order
    /// its codec takes them (`codec_type_params`): `u8` of `Pair<String,
    /// u8>`'s `impl<T> Serialize for Pair<String, T>`, not each of `ty`'s
    /// arguments (#1550).
    pub(in crate::lower) fn codec_type_args(&self, ty: Ty<'tcx>, serialize: bool) -> Vec<Ty<'tcx>> {
        let Some((imp, args)) = self.codec_impl(ty, serialize) else {
            return Vec::new();
        };
        let self_ty = self.tcx.type_of(imp).instantiate_identity().skip_normalization();
        let ty::Adt(_, own) = self_ty.kind() else {
            return Vec::new();
        };
        codec_type_params(own).map(|p| args.type_at(p.index as usize)).collect()
    }

    /// A type's codec, `method`, as it's called: a derived one's function, or
    /// a hand-written one's, which gives a `Result`, made one that throws its
    /// error, `$jsonCodec(f)` (ADR 0459).
    pub(in crate::lower) fn codec_callee(&mut self, method: DefId) -> Expr {
        let callee = self.fn_ref(method);
        match crate::lower::recognition::serde_derived(self.tcx, self.tcx.parent(method)) {
            true => callee,
            false => Expr::call(Expr::helper("$jsonCodec"), vec![callee]),
        }
    }

    /// The writer (`serialize`) or reader of a type parameter: a generic
    /// codec's parameter, or a generic function's evidence for its bound,
    /// `T: Serialize` or `T: DeserializeOwned` (ADR 0081).
    pub(super) fn serde_evidence(&self, ty: Ty<'tcx>, serialize: bool) -> Option<Expr> {
        if let Some((_, name)) = self.codecs.iter().find(|(t, _)| *t == ty) {
            return Some(Expr::var(name));
        }
        self.given_evidence(|tr| tr.self_ty() == ty && serde_trait(self.tcx, tr.def_id) == Some(serialize))
    }

    /// A generic type's codec takes a function for each of its type
    /// parameters, `writeT` or `readT` (ADR 0080): their names, which the
    /// codec's type parameters now stand for.
    pub(super) fn codec_params(&mut self, args: ty::GenericArgsRef<'tcx>, prefix: &str) -> Vec<js::Pattern> {
        self.codecs = Vec::new();
        let mut params = Vec::new();
        for p in codec_type_params(args) {
            let t = Ty::new_param(self.tcx, p.index, p.name);
            let name = self.fresh(Base::Named(&format!("{prefix}{}", p.name)));
            self.codecs.push((t, name.clone()));
            params.push(name.into());
        }
        params
    }
}

/// The type parameters a codec takes a writer or reader of, of its impl's
/// self type's arguments, `own`: each that's one, in their order, `T` of
/// `Pair<String, T>`. The codec's parameters (`codec_params`) and what its
/// callers give it (`codec_type_args`) are both this (#1550).
pub(super) fn codec_type_params<'tcx>(own: ty::GenericArgsRef<'tcx>) -> impl Iterator<Item = ty::ParamTy> + 'tcx {
    own.types().filter_map(|t| match t.kind() {
        ty::Param(p) => Some(*p),
        _ => None,
    })
}
