//! The libraries a crate is compiled against (ADR 0100): each one's
//! contract, read and checked before compilation starts, and `@rust-js/std`'s,
//! built into the compiler. Split from library.rs, at its cap, before #1799
//! adds to what's checked.

use super::*;

#[derive(Default)]
pub struct Dependencies {
    /// Each library, by its key, its crate's name and identity
    /// (`Library::key`); `@rust-js/std`'s, of std's crates, by its package's
    /// name, `runtime::PACKAGE`, which no crate is.
    pub libraries: IndexMap<String, Exports>,
    pub inputs: Vec<std::path::PathBuf>,
}

impl Dependencies {
    /// Read and validate complete dependency artifacts before compilation
    /// starts, for a crate whose JS is in `dir` and `output`'s.
    pub fn load(paths: &[std::path::PathBuf], output: &Path, dir: &Path) -> Result<Self, String> {
        let mut result = Self::default();
        let mut needed: Vec<(String, String)> = Vec::new();
        let output = crate::paths::absolute(output)?;
        let parent = output.parent().ok_or("output has no parent")?;
        for path in paths {
            result.inputs.push(crate::paths::absolute(path)?);
            let manifest = Manifest::read_from(path)?;
            result.inputs.extend(manifest.sources.iter().cloned());
            if manifest.compiler.as_ref() != Some(&Compiler::current()) {
                return Err(format!("incompatible dependency compiler identity: {}", path.display()));
            }
            let library = manifest.library.ok_or("dependency manifest has no library contract")?;
            if library.version != VERSION {
                return Err(format!(
                    "unsupported library ABI {}; expected {VERSION}",
                    library.version
                ));
            }
            let key = library.key();
            if result.libraries.contains_key(&key) {
                return Err(format!("duplicate dependency crate: {key}"));
            }
            needed.extend(
                library
                    .libraries
                    .iter()
                    .map(|used| (library.name.clone(), used.clone())),
            );
            for artifact in manifest.artifacts.iter().chain(&library.inputs) {
                result.inputs.push(artifact.file.clone());
                let bytes = std::fs::read(&artifact.file).map_err(|e| format!("{}: {e}", artifact.file.display()))?;
                if crate::manifest::fingerprint(&bytes) != artifact.hash {
                    return Err(format!("dependency artifact changed: {}", artifact.file.display()));
                }
            }
            let (
                mut exports,
                Unread {
                    items,
                    types,
                    held,
                    reexported,
                    modules,
                    package,
                },
            ) = Exports::read(library, None);
            // Where a module's JS is, from the consumer's: by its package's
            // subpath, where Node finds the package from the crate's JS and
            // its `exports` resolve one to that file (ADR 0770, amended),
            // else its file's path.
            let package = (package.as_ref())
                .filter(|package| crate::packages::installed(dir, &package.name).as_ref() == Some(&package.dir));
            let specifier = |path: &Vec<String>| -> Result<String, String> {
                let module = (manifest.modules.iter())
                    .find(|m| &m.module == path)
                    .ok_or("dependency export references a missing module")?;
                // Of types alone, its `.d.ts` is, and its types are imported
                // from where its JS would be, as TypeScript finds them (ADR
                // 1005).
                let written = match module.types_only {
                    true => module.types.as_ref(),
                    false => Some(&module.file),
                };
                if !written.is_some_and(|written| manifest.artifacts.iter().any(|a| &a.file == written)) {
                    return Err("dependency module is not a fingerprinted artifact".into());
                }
                let subpath = package.and_then(|package| package.subpath_specifier(path));
                Ok(subpath.unwrap_or_else(|| crate::paths::specifier(parent, &module.file)))
            };
            // What its package's root re-exports, by the package's name,
            // where Node finds that package from the crate's JS (ADR 1008):
            // an item of its root module, whichever path names it; another,
            // where the crate's Rust names it through the library's root, as
            // `Foreign::import_key` says of each module (ADR 1008, amended).
            let rooted_of = |module: &Vec<String>, export: &str| {
                package.and_then(|package| {
                    (package.exports.iter())
                        .find(|reexport| &reexport.module == module && reexport.export == export)
                        .map(|reexport| (package.name.clone(), reexport.entry.clone()))
                })
            };
            // Of its root module, by the package's name, whatever names it.
            let imported_from = |module: &Vec<String>, export: String| -> Result<(String, String), String> {
                match rooted_of(module, &export) {
                    Some(rooted) if module.is_empty() => Ok(rooted),
                    _ => Ok((specifier(module)?, export)),
                }
            };
            for module in manifest.modules.iter().filter(|module| !module.types_only) {
                let names = module
                    .exports
                    .iter()
                    .filter(|name| *name != "default")
                    .cloned()
                    .collect();
                if let Ok(from) = specifier(&module.module) {
                    exports.files.insert(from, names);
                }
            }
            for item in items {
                let export = item.exported_as();
                let rooted = rooted_of(&item.module, &export);
                // An internal one by its internal name, `$$LIMIT`, which no name
                // of a consumer's own is, as it reserves its imports' (#2017).
                let name = if item.internal { export.clone() } else { item.export };
                let mut imported = Imported {
                    from: specifier(&item.module)?,
                    export,
                    name,
                    member: item.member,
                    calling: item.calling,
                    rooted: None,
                };
                match rooted {
                    Some((name, entry)) if item.module.is_empty() => (imported.from, imported.export) = (name, entry),
                    rooted => imported.rooted = rooted,
                }
                if exports.items.insert(item.key, imported).is_some() {
                    return Err("duplicate dependency export".into());
                }
            }
            // A type, where its module's declarations are written.
            for declared in types {
                let declared_here = (manifest.modules.iter()).any(|m| m.module == declared.module && m.types.is_some());
                if declared_here {
                    let export = if declared.default {
                        "default".to_string()
                    } else {
                        declared.name.clone()
                    };
                    let (from, _) = imported_from(&declared.module, export)?;
                    let imported = (from, declared.name, declared.default);
                    if exports.types.insert(declared.key, imported).is_some() {
                        return Err("duplicate dependency export".into());
                    }
                }
            }
            for held in held {
                let (from, export) = imported_from(&held.module, held.export)?;
                let imported = HeldImport {
                    binding: held.binding,
                    from,
                    export,
                    name: held.name,
                };
                if exports.held.insert((held.holder, held.rust_name), imported).is_some() {
                    return Err("duplicate dependency export".into());
                }
            }
            for reexported in reexported {
                let imported = (reexported.item, imported_from(&reexported.module, reexported.export)?);
                if exports
                    .reexported
                    .insert((reexported.holder, reexported.rust_name), imported)
                    .is_some()
                {
                    return Err("duplicate dependency export".into());
                }
            }
            // A module's namespace, where it has JS: its file, as a glob of
            // it is `export * from` of (#1420).
            for named in modules {
                let has_js = (manifest.modules.iter()).any(|m| m.module == named.module && !m.types_only);
                if has_js && exports.modules.insert(named.key, specifier(&named.module)?).is_some() {
                    return Err("duplicate dependency module".into());
                }
            }
            result.libraries.insert(key, exports);
        }
        if let Some((library, used)) = needed.iter().find(|(_, used)| !result.libraries.contains_key(used)) {
            // A key's crate name, before its identity: no crate's name has a `-`.
            let used = used.rsplit_once('-').map_or(used.as_str(), |(name, _)| name);
            return Err(format!(
                "`{library}` was compiled against `{used}`, a rust-js library: give its manifest too, with --dependency"
            ));
        }
        result.inputs.sort();
        result.inputs.dedup();
        Ok(result)
    }
}

impl Dependencies {
    /// `@rust-js/std` added to what a crate is compiled against: each of
    /// its items, keyed as std's are, imported from its package's one entry,
    /// `@rust-js/std`, as its helpers are (#1095).
    pub fn add_std(&mut self) -> Result<(), String> {
        let manifest = Manifest::read(STD_MANIFEST.as_bytes())?;
        let library = manifest.library.ok_or("the std manifest has no library contract")?;
        if library.version != VERSION {
            return Err(format!(
                "the std manifest's library ABI is {}; expected {VERSION}",
                library.version
            ));
        }
        let (mut exports, Unread { items, .. }) = Exports::read(library, Some(crate::runtime::PACKAGE));
        for item in items {
            let imported = Imported {
                from: crate::runtime::PACKAGE.to_string(),
                export: item.exported_as(),
                name: item.export,
                member: item.member,
                calling: item.calling,
                rooted: None,
            };
            if exports.items.insert(item.key, imported).is_some() {
                return Err("duplicate std export".into());
            }
        }
        // Its items are std's, of `core`, `alloc` or `std`: by its package's
        // name, which no crate has, not theirs, which a program's may
        // (ADR 1054).
        self.libraries.insert(crate::runtime::PACKAGE.to_string(), exports);
        Ok(())
    }
}
