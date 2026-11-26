//! The catalog of std's compiled bodies resolved to `DefId`s, once a
//! session (#1681's change 2, step 3): each of `COMPILED_STD`'s,
//! `COMPILED_STD_IO`'s and `COMPILED_STD_NET`'s rows to the methods it
//! names, each provided method a catalog names, `HELD_COMPILED`'s makers'
//! too, and each free function. A row that resolves to nothing, of a std
//! crate the session loads, is an internal error naming the row: a toolchain that renames or drops what
//! a row names fails here, every compile, not by leaving the body out.

use super::{
    COMPILED_STD, COMPILED_STD_FNS, COMPILED_STD_IO, COMPILED_STD_IO_PROVIDED, COMPILED_STD_NET, COMPILED_STD_PROVIDED,
    COMPILED_STD_TUPLES, HELD_COMPILED, PROVED_LINKS, std_iter_trait,
};
use super::{Of, StdCrate, StdPath, registry, std_path_id};
use rust_js_program::collections::IndexMap;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;

/// A row of a type's methods: what they're of, its trait's name, its
/// method's.
pub(crate) type MethodRow = (Of, Option<&'static str>, &'static str);

/// The catalog's rows, each resolved.
pub struct Rows {
    /// Each type's row of `COMPILED_STD`, of `core::iter`, and the methods
    /// it names, of each impl of its trait for the type.
    pub(crate) iter: Vec<(MethodRow, Vec<DefId>)>,
    /// `COMPILED_STD_IO`'s and `COMPILED_STD_NET`'s rows.
    pub(crate) io: Vec<(MethodRow, Vec<DefId>)>,
    /// Each provided method a catalog names, by its trait's name and its
    /// own: `COMPILED_STD_PROVIDED`'s and `HELD_COMPILED`'s makers'.
    pub(crate) provided: IndexMap<(&'static str, &'static str), DefId>,
    /// `io`'s traits' provided methods.
    pub(crate) io_provided: Vec<DefId>,
    /// Each free function, `repeat_n`, and the type it makes, if a
    /// `MEANING_ONLY` one.
    pub(crate) fns: Vec<(DefId, Option<StdPath>)>,
    /// `PROVED_LINKS`' methods, each an iterator trait's.
    pub(crate) proved: rust_js_program::collections::IndexSet<DefId>,
    /// `COMPILED_STD_TUPLES`' methods, of each impl of its trait for a tuple
    /// of its arity.
    pub(crate) tuples: Vec<DefId>,
}

impl Rows {
    /// The provided method of std's `trait_name` named `method`, which a
    /// catalog names; asked of one none names, an internal error.
    pub(crate) fn provided(&self, trait_name: &'static str, method: &'static str) -> DefId {
        *self.provided.get(&(trait_name, method)).unwrap_or_else(|| {
            panic!(
                "rust-js internal error: `{trait_name}::{method}` is asked of std's catalog, which names no such row"
            )
        })
    }
}

/// A row's method of each of `impls`: one at least, or the row names
/// nothing on this toolchain.
fn of_impls(tcx: TyCtxt<'_>, row: MethodRow, impls: &[DefId]) -> Vec<DefId> {
    let found: Vec<DefId> = (impls.iter())
        .flat_map(|&imp| tcx.associated_items(imp).in_definition_order())
        .filter(|item| matches!(item.kind, ty::AssocKind::Fn { .. }) && item.name().as_str() == row.2)
        .map(|item| item.def_id)
        .collect();
    if found.is_empty() {
        unresolved(row);
    }
    found
}

/// A row that names nothing on this toolchain.
fn unresolved(row: impl std::fmt::Debug) -> ! {
    panic!(
        "rust-js internal error: std's catalog row {row:?} (crates/rust-js-lower/src/recognition/compiled_std.rs) resolves to nothing on this toolchain"
    )
}

/// The impls of `trait_def` for `adt`, in `all_impls`' order: an impl for
/// an ADT is in the one bucket rustc keeps under its simplified type, so
/// only that bucket is asked, where every impl of the trait was, `Iterator`'s
/// hundreds for each row, 3.5% of the corpus's compiler time (#2312).
fn impls_for(tcx: TyCtxt<'_>, trait_def: DefId, adt: DefId) -> Vec<DefId> {
    use rustc_middle::ty::fast_reject::SimplifiedType;
    let impls = tcx.trait_impls_of(trait_def);
    let bucket = impls.non_blanket_impls().get(&SimplifiedType::Adt(adt));
    (bucket.into_iter().flatten().copied())
        .filter(|&imp| matches!(tcx.type_of(imp).skip_binder().kind(), ty::Adt(of, _) if of.did() == adt))
        .collect()
}

/// The impls of `trait_def` for a primitive type, an integer or `char`, in
/// `all_impls`' order: each in the bucket rustc keeps under its simplified
/// type.
fn primitive_impls(tcx: TyCtxt<'_>, trait_def: DefId) -> Vec<DefId> {
    use rustc_middle::ty::fast_reject::SimplifiedType;
    (tcx.trait_impls_of(trait_def).non_blanket_impls().iter())
        .filter(|(simplified, _)| {
            matches!(
                simplified,
                SimplifiedType::Int(_) | SimplifiedType::Uint(_) | SimplifiedType::Char
            )
        })
        .flat_map(|(_, impls)| impls.iter().copied())
        .collect()
}

/// Each row of the catalog, resolved.
pub(super) fn resolve(tcx: TyCtxt<'_>) -> Rows {
    let methods = |row: MethodRow| -> Vec<DefId> {
        let (of, trait_name, _) = row;
        let item = match of {
            Of::Struct(item) => item,
            Of::Primitives => {
                let trait_def =
                    (trait_name.and_then(|name| std_iter_trait(tcx, name))).unwrap_or_else(|| unresolved(row));
                return of_impls(tcx, row, &primitive_impls(tcx, trait_def));
            }
        };
        // Of a std crate this session doesn't load, `alloc` of a
        // `#![no_std]` crate's, the rows are none of its, not unresolved:
        // the table's (`std_path_id`), which refuses one of none while
        // `std` is loaded.
        let Some(adt) = std_path_id(tcx, item) else {
            return Vec::new();
        };
        // A trait std keeps private is beside the type, `FuseImpl` in
        // `Fuse`'s module.
        let private_trait = |name: &str| {
            (tcx.module_children(tcx.parent(adt)).iter())
                .find(|child| child.ident.as_str() == name)
                .and_then(|child| child.res.opt_def_id())
        };
        let impls: Vec<DefId> = match trait_name {
            None => tcx.inherent_impls(adt).to_vec(),
            Some(trait_name) => {
                let trait_def = std_iter_trait(tcx, trait_name)
                    .or_else(|| private_trait(trait_name))
                    .unwrap_or_else(|| unresolved(row));
                impls_for(tcx, trait_def, adt)
            }
        };
        of_impls(tcx, row, &impls)
    };
    let provided_of = |trait_def: DefId, method: &str| {
        (tcx.associated_items(trait_def).in_definition_order())
            .find(|item| matches!(item.kind, ty::AssocKind::Fn { .. }) && item.name().as_str() == method)
            .map(|item| item.def_id)
    };
    let iter = (COMPILED_STD.iter()).map(|&row| (row, methods(row))).collect();
    // `io`'s rows are `alloc::io`'s, whose impls a crate without `alloc`
    // doesn't load, though `core`'s module names the types too.
    let alloc = super::std_crate_num(tcx, StdCrate::Alloc).is_some();
    let io = (COMPILED_STD_IO.iter().filter(|_| alloc).map(|&row| (row, methods(row))))
        .chain(COMPILED_STD_NET.iter().map(|&row| (row, methods(row))))
        .collect();
    let named =
        (COMPILED_STD_PROVIDED.iter().copied()).chain(HELD_COMPILED.iter().map(|&(_, maker, _)| ("Iterator", maker)));
    let provided = named
        .map(|(trait_name, method)| {
            let id = std_iter_trait(tcx, trait_name)
                .and_then(|trait_def| provided_of(trait_def, method))
                .unwrap_or_else(|| unresolved((trait_name, method)));
            ((trait_name, method), id)
        })
        .collect();
    let io_provided = (COMPILED_STD_IO_PROVIDED.iter())
        .filter(|_| super::std_crate_num(tcx, StdCrate::Alloc).is_some())
        .map(|&(trait_name, method)| {
            registry::by_path(tcx, StdCrate::Alloc, &["io", trait_name])
                .or_else(|| registry::by_path(tcx, StdCrate::Core, &["io", trait_name]))
                .and_then(|trait_def| provided_of(trait_def, method))
                .unwrap_or_else(|| unresolved((trait_name, method)))
        })
        .collect();
    let fns = (COMPILED_STD_FNS.iter())
        .map(|&(item, makes)| (std_path_id(tcx, item).unwrap_or_else(|| unresolved(item)), makes))
        .collect();
    let proved = (PROVED_LINKS.iter())
        .map(|&(trait_name, method)| {
            std_iter_trait(tcx, trait_name)
                .and_then(|trait_def| provided_of(trait_def, method))
                .unwrap_or_else(|| unresolved((trait_name, method)))
        })
        .collect();
    // Of a tuple of its arity, the impls rustc keeps under that simplified
    // type, as an ADT's are under its own (`impls_for`).
    let tuples = (COMPILED_STD_TUPLES.iter())
        .flat_map(|&(trait_name, arity)| {
            let trait_def = std_iter_trait(tcx, trait_name).unwrap_or_else(|| unresolved((trait_name, "")));
            let impls = tcx.trait_impls_of(trait_def);
            let bucket = impls
                .non_blanket_impls()
                .get(&ty::fast_reject::SimplifiedType::Tuple(arity));
            let found: Vec<DefId> = (bucket.into_iter().flatten())
                .flat_map(|&imp| tcx.associated_items(imp).in_definition_order())
                .filter(|item| matches!(item.kind, ty::AssocKind::Fn { .. }))
                .map(|item| item.def_id)
                .collect();
            if found.is_empty() {
                unresolved((trait_name, ""));
            }
            found
        })
        .collect();
    Rows {
        iter,
        io,
        provided,
        io_provided,
        fns,
        proved,
        tuples,
    }
}

/// The traits whose derives a program calls of std's struct: `{:?}` is
/// shown by its own path (`shown_private`, `debug_shown`), and `Copy` and
/// `Eq` write no step.
const DERIVED: &[&str] = &["Clone", "PartialEq", "PartialOrd", "Ord", "Hash", "Default"];

/// Derived methods of a compiled struct no row names, and why: a program
/// can't call them, as rust-js refuses it.
const DERIVES_LEFT_OUT: &[(StdPath, &str, &str, &str)] = &[
    // Comparing `std::net` addresses is refused (`refusal.rs`'s of a
    // comparison): `Ipv4Addr`'s and `Ipv6Addr`'s own `Ord`s, which these
    // derives call, aren't compiled.
    (
        StdPath::NetSocketAddrV4,
        "PartialOrd",
        "partial_cmp",
        "comparing net addresses is refused",
    ),
    (
        StdPath::NetSocketAddrV4,
        "Ord",
        "cmp",
        "comparing net addresses is refused",
    ),
    (
        StdPath::NetSocketAddrV6,
        "PartialOrd",
        "partial_cmp",
        "comparing net addresses is refused",
    ),
    (
        StdPath::NetSocketAddrV6,
        "Ord",
        "cmp",
        "comparing net addresses is refused",
    ),
    (
        StdPath::NetIpAddr,
        "PartialOrd",
        "partial_cmp",
        "comparing net addresses is refused",
    ),
    (StdPath::NetIpAddr, "Ord", "cmp", "comparing net addresses is refused"),
    (
        StdPath::NetSocketAddr,
        "PartialOrd",
        "partial_cmp",
        "comparing net addresses is refused",
    ),
    (
        StdPath::NetSocketAddr,
        "Ord",
        "cmp",
        "comparing net addresses is refused",
    ),
];

/// Each derived method std writes of a struct the catalog compiles that no
/// row names, as `Range::eq` was before its row (ADR 1206): `bun run std`
/// refuses to keep a catalogue while one is missing (`stored.rs`), so a
/// struct held as std's struct is never compared, hashed, ordered or made
/// by a body that isn't std's, but where `DERIVES_LEFT_OUT` says why, each
/// of which std derives. Each is `Type::Trait::method`.
pub(super) fn derives_unnamed(tcx: TyCtxt<'_>) -> Vec<String> {
    let rows = (COMPILED_STD.iter()).chain(COMPILED_STD_IO).chain(COMPILED_STD_NET);
    let mut structs: Vec<StdPath> = Vec::new();
    for &(of, ..) in rows.clone() {
        if let Of::Struct(item) = of
            && !structs.contains(&item)
        {
            structs.push(item);
        }
    }
    let mut unnamed = Vec::new();
    let mut seen_left_out = Vec::new();
    for item in structs {
        let Some(adt) = std_path_id(tcx, item) else { continue };
        for &trait_name in DERIVED {
            let Some(trait_def) = std_iter_trait(tcx, trait_name) else {
                continue;
            };
            let derived = impls_for(tcx, trait_def, adt)
                .into_iter()
                .filter(|&imp| tcx.is_automatically_derived(imp));
            for imp in derived {
                for method in (tcx.associated_items(imp).in_definition_order())
                    .filter(|assoc| matches!(assoc.kind, ty::AssocKind::Fn { .. }))
                {
                    let name = method.name();
                    let named = (rows.clone()).any(|&(of, of_trait, row)| {
                        of == Of::Struct(item) && of_trait == Some(trait_name) && row == name.as_str()
                    });
                    let left_out = (DERIVES_LEFT_OUT.iter())
                        .any(|&(of, of_trait, left, _)| of == item && of_trait == trait_name && left == name.as_str());
                    let path = format!("{item:?}::{trait_name}::{name}");
                    if named == left_out {
                        // Named and left out both, the entry's stale.
                        unnamed.push(if named {
                            format!("{path} (left out, but named)")
                        } else {
                            path
                        });
                    } else if left_out {
                        seen_left_out.push(path);
                    }
                }
            }
        }
    }
    // And each left out is one std derives.
    for &(of, of_trait, left, _) in DERIVES_LEFT_OUT {
        let path = format!("{of:?}::{of_trait}::{left}");
        if !seen_left_out.contains(&path) {
            unnamed.push(format!("{path} (left out, but std derives no such method)"));
        }
    }
    unnamed
}
