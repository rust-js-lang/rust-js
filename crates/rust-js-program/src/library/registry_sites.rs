//! Where a crate of crates.io's that rust-js reads as rustc builds it raises
//! the panics its runtime model raises in its place (#2325): serde_json's
//! `IndexMut` of a `Value`, `json["k"] = v` of a number, and hashbrown's
//! own map's `get_disjoint_mut` of two keys of one entry. Native Rust names
//! the crate's own line, of the version Cargo built, under where its
//! registry keeps it: found in that version's source by each `panic!`'s
//! message, and named under `/cargo/registry/src/<index>/`, whatever this
//! machine's `$CARGO_HOME` (ADR 0262's exception, the user's decision).
//! Found once, as the crate's check is made (`cargo.rs`'s translate), and
//! kept beside its metadata (`library::sites`), which a consumer's
//! lowering reads (`recognition::shipped`).

use crate::collections::IndexMap;
use std::path::Path;

/// A panic of a registry crate's whose place rust-js's model raises.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum RegistrySite {
    /// serde_json's `IndexMut` of a key of what isn't an object.
    JsonIndexKey,
    /// serde_json's `IndexMut` of an index of what isn't an array.
    JsonIndexIndex,
    /// serde_json's `IndexMut` of an index past an array's end.
    JsonIndexBound,
    /// hashbrown's own map's `get_disjoint_mut` of two keys of one entry.
    MapGetDisjointMut,
}

/// A crate's sites: its file and each `panic!`'s message.
struct CrateSites {
    name: &'static str,
    file: &'static str,
    sites: &'static [(RegistrySite, &'static str)],
}

/// Each crate's sites, the one catalog.
const CATALOG: &[CrateSites] = &[
    CrateSites {
        name: "serde_json",
        file: "src/value/index.rs",
        sites: &[
            (RegistrySite::JsonIndexKey, "cannot access key {:?} in JSON {}"),
            (RegistrySite::JsonIndexIndex, "cannot access index {} of JSON {}"),
            (
                RegistrySite::JsonIndexBound,
                "cannot access index {} of JSON array of length {}",
            ),
        ],
    },
    CrateSites {
        name: "hashbrown",
        file: "src/raw.rs",
        sites: &[(RegistrySite::MapGetDisjointMut, "duplicate keys found")],
    },
];

impl RegistrySite {
    /// Its key in the fact a crate's sites are kept in.
    pub fn key(self) -> &'static str {
        match self {
            RegistrySite::JsonIndexKey => "jsonIndexKey",
            RegistrySite::JsonIndexIndex => "jsonIndexIndex",
            RegistrySite::JsonIndexBound => "jsonIndexBound",
            RegistrySite::MapGetDisjointMut => "mapGetDisjointMut",
        }
    }
}

/// Whether the crate `name` has sites the catalog finds.
pub fn has_sites(name: &str) -> bool {
    CATALOG.iter().any(|each| each.name == name)
}

/// The sites of the crate `name` Cargo builds at `dir`, its package's
/// directory, each `file:line:column` under `/cargo/registry/src/`, by its
/// key; or why they can't be found: not a registry's crate, or a message
/// its source doesn't raise.
pub fn find_sites(name: &str, dir: &Path) -> Result<IndexMap<&'static str, String>, String> {
    let Some(&CrateSites { file, .. }) = CATALOG.iter().find(|each| each.name == name) else {
        return Ok(IndexMap::default());
    };
    let source = std::fs::read_to_string(dir.join(file)).map_err(|error| format!("{name}'s {file}: {error}"))?;
    sites_of(name, dir, &source)
}

/// Where `path`, a file of a dependency Cargo fetched, is named the same on
/// every machine, whatever its `$CARGO_HOME` (ADR 0262's difference 63, the
/// user's decision): a registry's crate's, `<home>/registry/src/<index>/..`,
/// as `/cargo/registry/src/<index>/..`, and a git dependency's,
/// `<home>/git/checkouts/..`, as `/cargo/git/checkouts/..`, so no JS
/// rust-js writes carries the machine's home (#2435). The one rule, of a
/// panic's location the crate's own body raises (`location_at`) and of each
/// site found here; `None` of any other file, which is named as rustc
/// names it.
pub fn cargo_named(path: &Path) -> Option<String> {
    let parts: Vec<String> = path
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect();
    let at = parts.windows(3).position(|three| {
        let pair = [&three[0][..], &three[1][..]];
        pair == ["registry", "src"] || pair == ["git", "checkouts"]
    })?;
    Some(format!("/cargo/{}", parts[at..].join("/")))
}

/// `find_sites`' of the crate's file's text, `source`.
fn sites_of(name: &str, dir: &Path, source: &str) -> Result<IndexMap<&'static str, String>, String> {
    let Some(&CrateSites { file, sites, .. }) = CATALOG.iter().find(|each| each.name == name) else {
        return Ok(IndexMap::default());
    };
    let Some(under) = cargo_named(&dir.join(file)).filter(|under| under.starts_with("/cargo/registry/")) else {
        return Err(format!(
            "{name} at {} isn't crates.io's, under a registry's index",
            dir.display()
        ));
    };
    (sites.iter())
        .map(|&(site, message)| {
            let (line, column) = panic_of(source, message)
                .ok_or_else(|| format!("{name}'s {file} raises no `panic!(\"{message}\", ..)`"))?;
            Ok((site.key(), format!("{under}:{line}:{column}")))
        })
        .collect()
}

/// Where `source` writes the `panic!` whose arguments' first is the text
/// `message`, its line and column from 1, as rustc's hook names a macro's
/// call; `None` where it writes none.
fn panic_of(source: &str, message: &str) -> Option<(usize, usize)> {
    let quoted = format!("\"{message}\"");
    source.match_indices("panic!(").find_map(|(at, call)| {
        let args = &source[at + call.len()..];
        // To its closing `)`, outside its strings, whose `{}` are a format's.
        let mut depth = 1usize;
        let mut quoted_text = false;
        let mut escaped = false;
        let end = args.char_indices().find_map(|(i, c)| {
            match (quoted_text, c) {
                (true, '\\') if !escaped => {
                    escaped = true;
                    return None;
                }
                (true, '"') if !escaped => quoted_text = false,
                (true, _) => {}
                (false, '"') => quoted_text = true,
                (false, '(' | '[' | '{') => depth += 1,
                (false, ')' | ']' | '}') => depth -= 1,
                (false, _) => {}
            }
            escaped = false;
            (depth == 0).then_some(i)
        })?;
        (args[..end].trim_start().starts_with(&quoted)).then(|| {
            let line_start = source[..at].rfind('\n').map_or(0, |n| n + 1);
            (
                source[..at].matches('\n').count() + 1,
                source[line_start..at].chars().count() + 1,
            )
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panic_is_found_by_its_message_across_lines() {
        let source = "fn f() {\n    match v {\n        _ => panic!(\"cannot access index {} of JSON {}\", i, t),\n    }\n    g(|| {\n        panic!(\n            \"cannot access index {} of JSON array of length {}\",\n            i, n\n        )\n    })\n}\n";
        assert_eq!(panic_of(source, "cannot access index {} of JSON {}"), Some((3, 14)));
        assert_eq!(
            panic_of(source, "cannot access index {} of JSON array of length {}"),
            Some((6, 9))
        );
        assert_eq!(panic_of(source, "cannot access key {:?} in JSON {}"), None);
        // A format's braces in its string, `{{` written once, aren't brackets.
        assert_eq!(
            panic_of("    panic!(\"a {{ brace {}\", x);\n", "a {{ brace {}"),
            Some((1, 5))
        );
    }

    #[test]
    fn a_fetched_dependency_s_file_is_named_under_cargo_whatever_the_home() {
        let named = |path: &str| cargo_named(Path::new(path));
        assert_eq!(
            named("/Users/u/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/itertools-0.14.0/src/lib.rs")
                .as_deref(),
            Some("/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/itertools-0.14.0/src/lib.rs")
        );
        assert_eq!(
            named("/opt/cargo-home/git/checkouts/ui-3a5c1e/9f8e7d6/src/lib.rs").as_deref(),
            Some("/cargo/git/checkouts/ui-3a5c1e/9f8e7d6/src/lib.rs")
        );
        assert_eq!(named("src/main.rs"), None);
        assert_eq!(named("/home/u/app/src/registry.rs"), None);
        assert_eq!(named("/home/u/app/registry/src"), None);
    }

    #[test]
    fn a_crate_s_sites_are_named_under_cargo_s_registry() {
        let dir = Path::new("/home/u/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.17.1");
        let source = "\n    fn d() {\n                    panic!(\"duplicate keys found\");\n    }\n";
        let sites = sites_of("hashbrown", dir, source).expect("its sites");
        assert_eq!(
            sites.get("mapGetDisjointMut").map(String::as_str),
            Some("/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashbrown-0.17.1/src/raw.rs:3:21")
        );
        assert!(sites_of("hashbrown", Path::new("/elsewhere/hashbrown-0.17.1"), source).is_err());
        let checkout = Path::new("/home/u/.cargo/git/checkouts/hashbrown-3a5c1e/9f8e7d6");
        assert!(sites_of("hashbrown", checkout, source).is_err());
        assert!(sites_of("hashbrown", dir, "fn d() {}\n").is_err());
    }
}
