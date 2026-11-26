//! The catalogue of std's bodies `@rust-js/std` compiles, made once, with
//! std (#2305): `walk`'s answer of each group of its roots, the base and
//! what each `MEANING_ONLY` adapter adds, kept in `std/compiled-std.json`
//! by `bun run std`, each id by its `DefPathHash`, under the hashes of the
//! crates they're of. A compile whose crates are those reads it, where it
//! walked std's MIR, about 6 ms, in every compile; one whose crates aren't,
//! std's own build, of a `core` built to unwind (ADR 0561), walks. The walk
//! is the one owner: test/compiled-std.test.ts holds the kept catalogue to
//! what it says.

use super::*;
use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_data_structures::fingerprint::Fingerprint;
use rustc_span::def_id::DefPathHash;

/// The catalogue kept: what `bun run std` made, which this compiler embeds.
const KEPT: &str = include_str!("../../../../../std/compiled-std.json");

/// The catalogue, as kept: each crate its ids are of by its name, with its
/// hash; the ids of the base group; and each `MEANING_ONLY` adapter's, by
/// its name, of those it adds to the base.
#[derive(::serde::Serialize, ::serde::Deserialize)]
struct Catalogue {
    crates: IndexMap<String, String>,
    base: Vec<String>,
    groups: IndexMap<String, Vec<String>>,
}

/// What a group's roots reach, by `walk`: the base, every adapter but
/// `MEANING_ONLY`'s, and with it `adapter`'s, where it's one.
fn group(tcx: TyCtxt<'_>, adapter: Option<StdPath>) -> IndexSet<DefId> {
    walk(tcx, &|item| !MEANING_ONLY.contains(&item) || Some(item) == adapter)
}

/// The catalogue `bun run std` keeps (`--std-catalogue`): each group walked.
/// What a union of groups' roots reaches is their walks' union, as the walk
/// takes each body on its own, never by what else it found; so each group
/// keeps what it adds to the base, and a compile its groups' union.
pub(crate) fn catalogue(tcx: TyCtxt<'_>) -> String {
    let unnamed = super::rows::derives_unnamed(tcx);
    assert!(
        unnamed.is_empty(),
        "rust-js internal error: std derives these of a struct the catalog compiles, which no row of crates/rust-js-lower/src/recognition/compiled_std.rs names, so its struct would be compared, hashed or made by a body that isn't std's: {unnamed:?}; add a row `(Of::Struct(..), Some(\"Trait\"), \"method\")` of each"
    );
    let base = group(tcx, None);
    let mut crates = IndexMap::default();
    let mut named = |ids: &mut dyn Iterator<Item = DefId>| -> Vec<String> {
        ids.map(|id| {
            crates.insert(
                tcx.crate_name(id.krate).to_string(),
                tcx.crate_hash(id.krate).to_string(),
            );
            hex(tcx.def_path_hash(id))
        })
        .collect()
    };
    let kept_base = named(&mut base.iter().copied());
    let groups = (MEANING_ONLY.iter())
        .map(|&adapter| {
            let added = group(tcx, Some(adapter));
            (
                format!("{adapter:?}"),
                named(&mut added.iter().copied().filter(|id| !base.contains(id))),
            )
        })
        .collect();
    let catalogue = Catalogue {
        crates,
        base: kept_base,
        groups,
    };
    format!(
        "{}\n",
        ::serde_json::to_string_pretty(&catalogue).expect("the catalogue serializes")
    )
}

/// The catalogue kept, of a build that calls the adapters `called` says:
/// the base and each called `MEANING_ONLY` adapter's group; `None` where a
/// crate its ids are of isn't this compile's, by its name and hash, or an
/// adapter has no group kept, so the compile walks.
pub(super) fn kept(tcx: TyCtxt<'_>, called: &dyn Fn(StdPath) -> bool) -> Option<IndexSet<DefId>> {
    let catalogue: Catalogue = ::serde_json::from_str(KEPT).ok()?;
    let loaded: IndexMap<String, String> = (tcx.crates(()).iter())
        .map(|&krate| (tcx.crate_name(krate).to_string(), tcx.crate_hash(krate).to_string()))
        .collect();
    if !(catalogue.crates.iter()).all(|(name, hash)| loaded.get(name) == Some(hash)) {
        return None;
    }
    let mut found = IndexSet::default();
    let mut add = |ids: &[String]| -> Option<()> {
        for id in ids {
            found.insert(tcx.def_path_hash_to_def_id(unhex(id)?)?);
        }
        Some(())
    };
    add(&catalogue.base)?;
    for &adapter in MEANING_ONLY.iter().filter(|&&adapter| called(adapter)) {
        add(catalogue.groups.get(&format!("{adapter:?}"))?)?;
    }
    Some(found)
}

fn hex(hash: DefPathHash) -> String {
    hash.0.to_le_bytes().iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(text: &str) -> Option<DefPathHash> {
    let mut bytes = [0u8; 16];
    for (at, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(text.get(at * 2..at * 2 + 2)?, 16).ok()?;
    }
    Some(DefPathHash(Fingerprint::from_le_bytes(bytes)))
}
