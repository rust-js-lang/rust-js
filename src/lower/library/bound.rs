//! The npm packages a crate's JS imports through bindings (#1799, ADR 0100,
//! amended): an app installs one copy of a package, which a bare specifier,
//! `"@floating-ui/dom"`, resolves to wherever its importer's JS is, so two
//! releases of one binding crate over one package would each read that
//! copy, one of them another release's JS than it binds. A crate refuses
//! that, of its own imports and its libraries', naming the crates whose JS
//! imports each. Two binding crates of a package by distinct npm names, an
//! alias `"dom-v0": "npm:@floating-ui/dom@0.5"`, are two packages; and two
//! binding crates of other names over one package are each their author's.

use super::*;
use rust_js_program::library::Bound;

/// The packages the crate's JS imports through bindings, `bound_to`'s, each
/// of its binding's crate, with those its libraries' imports do; the pairs
/// of releases of one binding crate over one package refused.
pub(in crate::lower) fn bound(
    tcx: TyCtxt<'_>,
    bound_to: &IndexMap<super::super::bindings::Export, IndexSet<DefId>>,
    foreign: &Foreign<'_, '_>,
) -> Vec<Bound> {
    let by = crate_name(tcx, LOCAL_CRATE);
    let own = bound_to.iter().flat_map(|((specifier, _), bindings)| {
        let package = package(specifier).map(str::to_string);
        let by = by.clone();
        bindings.iter().filter_map(move |binding| {
            Some(Bound {
                package: package.clone()?,
                binding: crate_name(tcx, binding.krate),
                crate_id: crate_id(tcx, binding.krate),
                by: by.clone(),
            })
        })
    });
    let theirs = (foreign.dependencies.libraries.values()).flat_map(|library| library.bound.iter().cloned());
    let all: BTreeSet<Bound> = own.chain(theirs).collect();
    // Each package's binding crates, by their names, each release's
    // importers.
    let mut releases: IndexMap<(&str, &str), IndexMap<&str, BTreeSet<&str>>> = IndexMap::default();
    for bound in &all {
        (releases.entry((&bound.package, &bound.binding)).or_default())
            .entry(&bound.crate_id)
            .or_default()
            .insert(&bound.by);
    }
    for ((package, binding), ids) in releases {
        if ids.len() < 2 {
            continue;
        }
        // In their names' order, which a release's identity, of its path,
        // isn't.
        let mut importers: Vec<String> = (ids.values())
            .map(|by| by.iter().map(|by| format!("`{by}`")).collect::<Vec<_>>().join(" and "))
            .collect();
        importers.sort();
        tcx.dcx().err(format!(
            "rust-js: two releases of `{binding}` bind the npm package `{package}`, one used by {}, the other by {}: an app installs `{package}` once, which both would read, so one release's bindings would call another's JS; depend on one release of `{binding}`, or bind the other through an npm alias of its own, `\"{package}-v0\": \"npm:{package}@..\"` (#1799)",
            importers[0], importers[1]
        ));
    }
    all.into_iter().collect()
}

/// The npm package a bare specifier names, `@floating-ui/dom` of
/// `@floating-ui/dom/utils`, `react` of `react/jsx-runtime`; none of a file's,
/// `./x.js`, a URL's or a scheme's, `node:path`, or Node's own `#internal`.
fn package(specifier: &str) -> Option<&str> {
    if specifier.starts_with(['.', '/', '#']) || specifier.contains(':') {
        return None;
    }
    let mut parts = specifier.splitn(3, '/');
    let first = parts.next().filter(|first| !first.is_empty())?;
    match first.starts_with('@') {
        true => parts.next().map(|second| &specifier[..first.len() + 1 + second.len()]),
        false => Some(first),
    }
}

#[cfg(test)]
mod tests {
    use super::package;

    #[test]
    fn a_bare_specifiers_package() {
        assert_eq!(package("@floating-ui/dom/utils"), Some("@floating-ui/dom"));
        assert_eq!(package("react/jsx-runtime"), Some("react"));
        assert_eq!(package("react"), Some("react"));
        assert_eq!(package("./widget.js"), None);
        assert_eq!(package("node:path"), None);
        assert_eq!(package("#internal"), None);
        assert_eq!(package("@scope"), None);
    }
}
