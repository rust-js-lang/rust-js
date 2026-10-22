//! Types and traits as words of JS names: an impl's dictionary's name, and
//! an evidence's.

use super::*;

/// The type, then the trait, then the trait's arguments other than their
/// defaults: `circleShape`, `metersFromF64` for `impl From<f64> for
/// Meters`, and `versionPartialEq`, whose `Rhs` is `Self` (ADR 0052).
pub(in crate::lower) fn impl_name(tcx: TyCtxt<'_>, id: DefId) -> String {
    let named = |id: DefId, full: bool, module: &str| {
        let tr = tcx.impl_trait_ref(id).instantiate_identity().skip_normalization();
        if !full {
            return format!(
                "{}{module}{}{}",
                lower_first(&js_word(&type_word(tcx, tr.self_ty()))),
                trait_word(tcx, tr),
                specializing_words(tcx, id)
            );
        }
        let mut name = lower_first(&js_word(&full_type_word(tcx, tr.self_ty())));
        name.push_str(module);
        name.push_str(tcx.item_name(tr.def_id).as_str());
        name.push_str(&specializing_words(tcx, id));
        for arg in tr.args.iter().skip(1) {
            // A const argument by its value, `Scaled10` of `Scaled<10>` (ADR 0135).
            let word = match (arg.as_type(), arg.as_const()) {
                (Some(ty), _) => js_word(&full_type_word(tcx, ty)),
                (None, Some(c)) => js_word(&c.to_string()),
                (None, None) => continue,
            };
            name.push_str(&crate::lower::upper_first(&word));
        }
        name
    };
    let tr = tcx.impl_trait_ref(id).instantiate_identity().skip_normalization();
    // Two traits of one name for one type, serde's `de::Error` and
    // `ser::Error`: each named with its trait's module too, `errorDeError`.
    let short = named(id, false, "");
    let clash = tcx
        .trait_impls_in_crate(id.krate)
        .iter()
        .any(|&other| tcx.impl_trait_id(other) != tr.def_id && named(other, false, "") == short);
    let module = match clash {
        true => {
            // A crate root's is its path, the crate's name, or `crate`.
            let parent = tcx.parent(tr.def_id);
            let module = (tcx.opt_item_name(parent)).map_or_else(|| tcx.def_path_str(parent), |name| name.to_string());
            crate::lower::upper_first(&crate::lower::camel_case(&module))
        }
        false => String::new(),
    };
    let name = named(id, false, &module);
    // Two impls of a trait for one type with other arguments, `Vec<i32>`'s and
    // `Vec<String>`'s: each named with the arguments, its type's and its
    // trait's, `vecI32Describe`; and, two types of one name, local to two
    // functions, numbered in the order they're declared.
    let mut alike: Vec<DefId> = tcx
        .all_impls(tr.def_id)
        .filter(|&other| other.krate == id.krate && named(other, false, &module) == name)
        .collect();
    if alike.len() < 2 {
        return name;
    }
    let full = named(id, true, &module);
    alike.retain(|&other| named(other, true, &module) == full);
    alike.sort_by_key(|other| other.index);
    match alike.iter().position(|&other| other == id) {
        Some(at) if at > 0 => format!("{full}{}", at + 1),
        _ => full,
    }
}

/// Of a std impl that specializes another of its trait for the same type,
/// `impl<I: FusedIterator> FuseImpl<I> for Fuse<I>` the blanket `impl<I:
/// Iterator>`'s (ADR 0561): the traits its bounds name that the other's
/// don't, `FusedIterator`, which tell the two apart; nothing of another.
fn specializing_words(tcx: TyCtxt<'_>, id: DefId) -> String {
    let trait_id = tcx.impl_trait_id(id);
    if !crate::lower::recognition::std_private(tcx, trait_id) {
        return String::new();
    }
    let Some(other) = tcx
        .all_impls(trait_id)
        .find(|&other| other != id && tcx.specializes((id, other)))
    else {
        return String::new();
    };
    let bounds = |imp: DefId| -> Vec<DefId> {
        (tcx.clauses_of(imp).instantiate_identity(tcx).into_iter())
            .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
            .map(|bound| bound.def_id())
            .collect()
    };
    let others = bounds(other);
    (bounds(id).into_iter())
        .filter(|bound| !others.contains(bound))
        .map(|bound| tcx.item_name(bound).to_string())
        .collect()
}

/// `type_word`, with the type's arguments other than their defaults:
/// `VecI32` of `Vec<i32>`, whose allocator is the default.
fn full_type_word<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> String {
    match ty.kind() {
        ty::Adt(adt, args) => {
            let mut name = tcx.item_name(adt.did()).to_string();
            let generics = tcx.generics_of(adt.did());
            for (param, arg) in generics.own_params.iter().zip(args.iter()) {
                let Some(arg) = arg.as_type() else { continue };
                if param
                    .default_value(tcx)
                    .map(|d| d.instantiate(tcx, args).skip_normalization())
                    == Some(arg.into())
                {
                    continue;
                }
                name.push_str(&crate::lower::upper_first(&js_word(&full_type_word(tcx, arg))));
            }
            name
        }
        _ => type_word(tcx, ty),
    }
}

/// The key a dictionary has a fact of its associated type `item` under,
/// `ItemSize` (ADR 0145).
pub(in crate::lower) fn item_fact_key(tcx: TyCtxt<'_>, item: DefId, fact: TypeFact) -> String {
    format!("{}{}", tcx.item_name(item), fact.word())
}

/// A type as a word of an evidence name: a type parameter's, `T`, or an
/// associated type's of one, `SItem` of `<S as Source>::Item`.
pub(super) fn evidence_word<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> String {
    match ty.kind() {
        ty::Alias(
            _,
            alias @ ty::AliasTy {
                kind: ty::Projection { def_id },
                ..
            },
        ) => {
            format!("{}{}", evidence_word(tcx, alias.self_ty()), tcx.item_name(*def_id))
        }
        // `name: impl Into<String>`'s, which rustc names as it's written: its
        // trait's word alone, `IntoString`.
        ty::Param(p) if p.name.as_str().starts_with("impl ") => String::new(),
        // An array's and a slice's, of their item's: `TArray` of `[T; N]`,
        // as a where clause's bound of one is `TArrayDefault`.
        ty::Array(item, _) => format!("{}Array", evidence_word(tcx, *item)),
        ty::Slice(item) => format!("{}Slice", evidence_word(tcx, *item)),
        // A reference's, of what it's to: `CRefMut` of `&'a mut C`, as a
        // bound on one, `CRefMutIntoIterator` (ADR 0457).
        ty::Ref(_, inner, mutability) => {
            let kind = if mutability.is_mut() { "RefMut" } else { "Ref" };
            format!("{}{kind}", evidence_word(tcx, *inner))
        }
        // A struct's or an enum's own name, `String`, not its path.
        ty::Adt(..) => type_word(tcx, ty),
        // A derive's own, `__H` of `#[derive(Hash)]`'s `hash<__H: Hasher>`:
        // `H`, as it's written by hand (ADR 0463).
        ty::Param(p) if p.name.as_str().starts_with('_') && !p.name.as_str().trim_start_matches('_').is_empty() => {
            p.name.as_str().trim_start_matches('_').to_string()
        }
        _ => ty.to_string(),
    }
}

/// A type as a word of a JS name: an ADT's own name, `Meters` of
/// `Meters<T>`; a reference's, `RefFlags` of `&'a Flags` and `RefMutI32`
/// of `&mut i32`, with no lifetime, which isn't in the JS.
fn type_word<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> String {
    match ty.kind() {
        ty::Adt(adt, _) => tcx.item_name(adt.did()).to_string(),
        ty::Ref(_, inner, mutability) => {
            let inner = js_word(&type_word(tcx, *inner));
            let kind = if mutability.is_mut() { "RefMut" } else { "Ref" };
            format!("{kind}{}", crate::lower::upper_first(&inner))
        }
        // An associated type's, `BItem` of `<B as Array>::Item`.
        ty::Alias(..) => evidence_word(tcx, ty),
        // A slice's, its item's, `U32Slice` of `[u32]`.
        ty::Slice(item) => {
            let item = js_word(&type_word(tcx, *item));
            format!("{}Slice", crate::lower::upper_first(&item))
        }
        _ => ty.to_string(),
    }
}

/// The trait and its arguments other than their defaults, as a word of a JS
/// name: `ConvertF64` of `Convert<f64>`, `PartialEq` of `PartialEq<Self>`.
pub(super) fn trait_word<'tcx>(tcx: TyCtxt<'tcx>, tr: ty::TraitRef<'tcx>) -> String {
    trait_word_with_refs(tcx, tr, false)
}

/// `trait_word`, each reference among its arguments a `Ref` if `refs`.
pub(super) fn trait_word_with_refs<'tcx>(tcx: TyCtxt<'tcx>, tr: ty::TraitRef<'tcx>, refs: bool) -> String {
    let mut name = tcx.item_name(tr.def_id).to_string();
    let generics = tcx.generics_of(tr.def_id);
    for (param, arg) in generics.own_params.iter().zip(tr.args).skip(1) {
        let Some(arg) = arg.as_type() else { continue };
        if param
            .default_value(tcx)
            .map(|d| d.instantiate(tcx, tr.args).skip_normalization())
            == Some(arg.into())
        {
            continue;
        }
        // `&'r Base` is `RefBase`, a reference's word before its pointee's.
        let mut word = String::new();
        let mut pointee = arg;
        while let ty::Ref(_, inner, _) = *pointee.kind() {
            if refs {
                word.push_str("Ref");
            }
            pointee = inner;
        }
        let pointee = js_word(&type_word(tcx, pointee));
        match word.is_empty() {
            true => word.push_str(&pointee),
            false => word.push_str(&crate::lower::upper_first(&pointee)),
        }
        name.push_str(&crate::lower::upper_first(&word));
    }
    name
}

/// A type's name as part of a JS name: what isn't a letter or a digit is
/// `_`, so `Vec<T>` is `Vec_T_`.
pub(super) fn js_word(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

/// A type parameter's name as part of a JS name: `T`, or of `x: impl
/// fmt::Display`, which rustc names as it's written, its trait's, `Display`.
pub(super) fn param_word(name: Symbol) -> String {
    let Some(bound) = name.as_str().strip_prefix("impl ") else {
        return name.to_string();
    };
    let path: String = bound
        .chars()
        .take_while(|&c| c.is_alphanumeric() || c == '_' || c == ':')
        .collect();
    path.rsplit("::").next().unwrap_or_default().to_string()
}
