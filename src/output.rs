//! Resolve and validate all artifacts before touching output files.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::manifest::{self, Manifest, fingerprint};
use crate::paths::{absolute, parent_dir, relative, relative_resolved, resolve, specifier_file};

use crate::publish::Artifact;
use crate::{js, program, to_oxc};

pub struct OutputPlan {
    pub input: PathBuf,
    pub output: PathBuf,
    pub test: bool,
    pub manifest: Option<PathBuf>,
    /// A library's metadata (ADR 0100): where rustc wrote it, and where it's
    /// published, with the JS, as an artifact of this plan.
    pub metadata: Option<(PathBuf, PathBuf)>,
    /// Files of the build tool's published with the JS, as artifacts of this
    /// plan, and no part of the manifest: what Cargo is told (ADR 0101).
    pub extra: Vec<(PathBuf, Vec<u8>)>,
    /// Cargo's record of the sources (ADR 0101), the driver's to write:
    /// where it's published, and what writes it, given what else the build
    /// read (`read`).
    pub dep_info: Option<(PathBuf, DepInfo)>,
    /// The crate's settings for the JS (ADR 0117).
    pub settings: crate::settings::Settings,
    /// `@rust-js/std`'s build, whose manifest is committed beside its JS:
    /// its paths relative to it (ADR 0557).
    pub std_bodies: bool,
    /// The JS files the plan has, once planned, which a crate's checks read.
    pub written: Vec<PathBuf>,
    /// What the build read besides the crate, once planned: the app's
    /// Prettier configs (ADR 0381), which Cargo is told of.
    pub read: Vec<PathBuf>,
    jsx: HashSet<Vec<String>>,
    /// Each `#[path]` module's file, where its JS goes beside (ADR 0273).
    located: HashMap<Vec<String>, PathBuf>,
}

/// What writes Cargo's record of a crate's sources, given what else the
/// build read.
pub type DepInfo = Box<dyn FnOnce(&[PathBuf]) -> Result<Vec<u8>, String>>;

/// A module's JS, as rust-js lays it out, and where it's written.
struct Laid {
    js_file_name: String,
    js_path: PathBuf,
    file: PathBuf,
    printed: to_oxc::Output,
}

impl OutputPlan {
    /// Write one `.js` and `.js.map` per module (ADR 0019), and the
    /// configs the app's Prettier read, laying them out (ADR 0381).
    fn generate(
        &self,
        modules: Vec<program::LoweredModule>,
        sources: &program::Sources,
    ) -> Result<(Vec<Artifact>, Vec<PathBuf>), String> {
        let mut artifacts = Vec::new();
        let mut typed = Vec::new();
        let mut laid = Vec::new();
        // Each source's path is resolved once, and each module's directory:
        // a module's map asks for a path only to the sources it points into
        // (found in review).
        let resolved: Vec<PathBuf> = sources
            .files
            .iter()
            .map(|source| resolve(source.path.as_deref().unwrap_or(&self.input)))
            .collect();
        for module in modules {
            let js_path = self.js_path(&module.path);
            let rust_path = module.file.clone().unwrap_or_else(|| self.input.clone());
            let specified = |imports: &[program::LoweredImport]| {
                imports
                    .iter()
                    .map(|import| js::Import {
                        named: import.named.clone(),
                        from: self.specifier(&module.path, &import.path),
                    })
                    .collect::<Vec<_>>()
            };
            let imports = specified(&module.imports);
            let located = |from: &str| self.linked_specifier(&module.path, from);
            // Where a module of the crate's, or a JS module, is from this
            // one's file, as its JS and its `.d.ts` name it.
            let from_here = |from: &js::ImportFrom| match from {
                js::ImportFrom::Module(path) => self.specifier(&module.path, path),
                js::ImportFrom::Specifier(specifier) => located(specifier),
            };
            let reexports = (module.reexports.iter())
                .map(|reexport| js::Import {
                    named: reexport.named.clone(),
                    from: from_here(&reexport.from),
                })
                .collect::<Vec<_>>();
            let packages = module
                .packages
                .into_iter()
                .map(|package| js::Package {
                    from: located(&package.from),
                    ..package
                })
                .collect();
            // rust-js's line, then the file's own leading comments, a
            // license's, as the file it replaced had them (ADR 0383).
            // The file from its crate's directory, of a crate Cargo builds, so
            // a crate's JS is the same whichever build wrote it, its author's
            // or an app's of it from npm (ADR 0414); a file compiled alone, as
            // it's given.
            let shown = absolute(&rust_path)
                .ok()
                .zip(absolute(&self.settings.dir).ok().filter(|_| self.settings.cargo))
                .and_then(|(file, dir)| file.strip_prefix(dir).ok().map(std::path::Path::to_path_buf))
                .unwrap_or_else(|| rust_path.clone());
            let mut header = format!("// Generated by rust-js from {}. Do not edit.", shown.display());
            let own = module.file.as_ref().and_then(|file| {
                let file = resolve(file);
                sources
                    .files
                    .iter()
                    .find(|source| source.path.as_deref().map(resolve).as_ref() == Some(&file))
            });
            if let Some(comments) = own
                .map(|source| crate::comments::leading_comments(&source.text))
                .filter(|c| !c.is_empty())
            {
                header.push_str("\n\n");
                header.push_str(comments);
            }
            let mut js_module = js::Module {
                header,
                directives: module.directives,
                packages,
                imports,
                reexports,
                helpers: Vec::new(),
                items: module.items,
                caches: module.caches,
                default_export: module.default_export,
            };

            // What it imports when it's asked for, from where its file is, a
            // module of the crate's by its file's specifier (ADR 0304).
            js_module.each_expr_mut(&mut |e| {
                if let js::ExprKind::Import(from) = &mut e.kind {
                    *from = js::ImportFrom::Specifier(from_here(from));
                }
            });
            crate::prepare::module(&mut js_module);
            // What only an earlier phase resolves, left: a compiler's
            // mistake, refused, not printed.
            if let Some(what) = js_module.unprintable() {
                return Err(format!(
                    "rust-js left {what} in {} for printing: a bug of rust-js's",
                    js_path.display()
                ));
            }
            // Of the helpers it asked for, and what they use, the names its
            // prepared tree reads (ADR 0103).
            let helper_sources: Vec<&'static str> = module.runtime.iter().map(|helper| helper.source()).collect();
            js_module.helpers = crate::runtime::imported_helpers(&helper_sources, &js_module.read_vars());
            // A helper the module reads and didn't ask for would be a JS
            // error where it runs, its import left out: refused, as a
            // compiler's mistake, not written.
            if let Some(name) = (js_module.read_vars().into_iter())
                .find(|name| crate::runtime::exports(name) && !js_module.helpers.contains(name))
            {
                return Err(format!(
                    "rust-js wrote `{name}` in {} without asking for the helper that declares it: a bug of rust-js's",
                    js_path.display()
                ));
            }
            let dir = resolve(parent_dir(&js_path));
            let own = resolve(&rust_path);
            let path_of = |i: usize| relative_resolved(&dir, &resolved[i]);
            let js_file_name = js_path.file_name().unwrap_or_default().to_string_lossy();
            let file = absolute(&js_path)?;
            let printed = to_oxc::emit(
                &js_module,
                sources,
                &path_of,
                resolved.iter().position(|source| *source == own),
                &relative_resolved(&dir, &own),
                &js_file_name,
            );
            laid.push(Laid {
                js_file_name: js_file_name.into_owned(),
                js_path: js_path.clone(),
                file,
                printed,
            });
            // What it exports, typed, for the TypeScript that imports it (ADR 0196).
            if self.settings.declarations
                && let Some(mut declarations) = module.declarations
            {
                declarations["header"] = js_module.header.clone().into();
                // Another module's types, from its file, as its JS imports it (ADR 0210).
                // A JS module's, a binding's re-export, as its JS has it (ADR 0240).
                for import in declarations["declarations"].as_array_mut().into_iter().flatten() {
                    let object = import.as_object_mut().expect("a declaration");
                    let from = match (object.remove("module"), object.remove("specifier")) {
                        (Some(path), _) => {
                            js::ImportFrom::Module(serde_json::from_value(path).map_err(|e| e.to_string())?)
                        }
                        (None, Some(serde_json::Value::String(specifier))) => js::ImportFrom::Specifier(specifier),
                        _ => continue,
                    };
                    object.insert("from".into(), from_here(&from).into());
                }
                typed.push((types_path(&js_path), declarations));
            }
        }
        // Printed by TypeScript, all in one session of its (ADR 0207).
        let mut declared = Vec::new();
        if !typed.is_empty() {
            let (paths, models): (Vec<PathBuf>, Vec<serde_json::Value>) = typed.into_iter().unzip();
            declared = paths
                .into_iter()
                .zip(crate::typescript::print(&self.settings, &models)?)
                .collect();
        }
        // Laid out by the app's Prettier, each file it has a config for, all
        // in one session of it (ADR 0381).
        let mut files = Vec::new();
        for laid in &laid {
            files.push((laid.file.clone(), laid.printed.code.as_str()));
        }
        for (path, text) in &declared {
            files.push((absolute(path)?, text.as_str()));
        }
        let prettier = crate::prettier::laid_out(&self.settings, &files)?;
        let (js_texts, types_texts) = prettier.texts.split_at(laid.len());
        for (laid, text) in laid.into_iter().zip(js_texts) {
            let Laid {
                js_file_name,
                js_path,
                file,
                printed: to_oxc::Output { code, map },
            } = laid;
            // The same program, its map moved to Prettier's layout, as a
            // transform's is; or rust-js's, and a warning says why.
            let (code, map) = match text {
                Some(text) => match crate::format::transformed(&code, text.clone(), &map, &js_file_name) {
                    Ok(moved) => moved,
                    Err(why) => {
                        eprintln!(
                            "warning: rust-js: the app's Prettier {why}: `{}` is written as rust-js lays it out",
                            js_path.display()
                        );
                        (code, map)
                    }
                },
                None => (code, map),
            };
            // What the crate's transforms make of it (ADR 0117), then where
            // its map is.
            let (mut code, map) = crate::hooks::transform(&self.settings, &file, &js_file_name, code, map);
            code.push_str(&format!("//# sourceMappingURL={js_file_name}.map\n"));
            let map_path = PathBuf::from(format!("{}.map", js_path.display()));
            artifacts.push(Artifact {
                path: js_path,
                bytes: code.into_bytes(),
            });
            artifacts.push(Artifact {
                path: map_path,
                bytes: map.into_bytes(),
            });
        }
        for ((path, text), laid_out) in declared.into_iter().zip(types_texts) {
            artifacts.push(Artifact {
                path,
                bytes: laid_out.clone().unwrap_or(text).into_bytes(),
            });
        }
        Ok((artifacts, prettier.configs))
    }

    /// In test mode, `<output>.test.js` beside the output: one `test(..)`
    /// per `#[test]` function, for `bun test` (ADR 0026). `test` is the
    /// runner's global, so nothing here is specific to one runner.
    fn tests(&self, tests: &[program::TestFn]) -> Option<Artifact> {
        if !self.test {
            return None;
        }
        let mut modules: Vec<&Vec<String>> = Vec::new();
        for test in tests {
            if !modules.contains(&&test.module) {
                modules.push(&test.module);
            }
        }
        // One namespace import per module, named after it, like ADR 0019.
        let crate_name = self
            .output
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        // `test` is the runner's global; a module's alias, a Rust name, is
        // never a helper's `$` one.
        let mut taken: std::collections::HashSet<String> = ["test".to_string()].into();
        let mut aliases = Vec::new();
        let mut code = format!(
            "// Generated by rust-js from {}: its `#[test]` functions, for `bun test`. Do not edit.\n\n",
            self.input.display()
        );
        for module in &modules {
            let base = module
                .last()
                .cloned()
                .unwrap_or_else(|| crate_name.clone())
                .replace('-', "_");
            let alias = crate::names::fresh_in(&mut taken, &base);
            code.push_str(&to_oxc::import_line(
                &format!("* as {alias}"),
                &self.specifier(&[], module),
            ));
            aliases.push(alias);
        }
        // What a test's outcome is, the runtime's (ADR 0026).
        let helpers: Vec<&str> = [
            ("$shouldPanic", tests.iter().any(|t| t.should_panic.is_some())),
            ("$testResult", tests.iter().any(|t| t.returns_result)),
        ]
        .into_iter()
        .filter_map(|(helper, used)| used.then_some(helper))
        .collect();
        if !helpers.is_empty() {
            code.push_str(&to_oxc::import_line(
                &format!("{{ {} }}", helpers.join(", ")),
                crate::runtime::PACKAGE,
            ));
        }
        code.push('\n');
        for test in tests {
            let alias = &aliases[modules.iter().position(|m| **m == test.module).unwrap()];
            let f = format!("{alias}.{}", test.name);
            let body = match &test.should_panic {
                None if test.returns_result => format!("() => $testResult({f}())"),
                None => f,
                Some(None) => format!("() => $shouldPanic({f})"),
                Some(Some(expected)) => format!("() => $shouldPanic({f}, {expected:?})"),
            };
            let register = if test.ignore { "test.skip" } else { "test" };
            code.push_str(&format!("{register}({:?}, {body});\n", test.label));
        }
        let stem = self
            .output
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let path = self.output.with_file_name(format!("{stem}.test.js"));
        Some(Artifact {
            path,
            bytes: code.into_bytes(),
        })
    }

    /// Where a module's JS goes in source, beside its Rust (ADRs 0041,
    /// 0273): the file of the module whose file its source is, `src/api.rs`'s
    /// `src/api.js` and `src/api/mod.rs`'s `src/api/mod.js`, or a `#[path]`
    /// module's beside its file; an inline module's, `mod inner { .. }`,
    /// where its file would be, `src/inner.js`.
    fn in_source(&self, module: &crate::program::LoweredModule) -> Result<PathBuf, String> {
        let extension = if self.jsx.contains(&module.path) { "jsx" } else { "js" };
        let input = absolute(&self.input)?;
        let root = parent_dir(&input);
        if let Some(source) = module.file.as_deref().map(absolute).transpose()? {
            // The module whose file `source` is, by where it is.
            let mut own: Vec<String> = match source == input {
                true => Vec::new(),
                false => relative_resolved(root, &source.with_extension(""))
                    .split(std::path::MAIN_SEPARATOR)
                    .map(str::to_owned)
                    .collect(),
            };
            if own.last().is_some_and(|name| name == "mod") {
                own.pop();
            }
            if own == module.path || module.located {
                return Ok(source.with_extension(extension));
            }
        }
        let mut path = root.join(module.path.iter().collect::<PathBuf>());
        path.set_extension(extension);
        Ok(path)
    }

    /// Where a module's JS goes: the root to the output file, `a::b` to
    /// `a/b.js` beside it, mirroring how Rust lays out `src/`. A module
    /// with JSX is a `.jsx` file, as JSX tools expect (ADR 0040).
    fn js_path(&self, module: &[String]) -> PathBuf {
        let jsx = self.jsx.contains(module);
        if let Some(file) = self.located.get(module) {
            return file.with_extension(if jsx { "jsx" } else { "js" });
        }
        if module.is_empty() {
            return if jsx {
                self.output.with_extension("jsx")
            } else {
                self.output.clone()
            };
        }
        let mut path = parent_dir(&self.output).to_path_buf();
        path.extend(module);
        path.set_extension(if jsx { "jsx" } else { "js" });
        path
    }

    /// The import specifier, in module `from`'s file, for module `to`'s file:
    /// `./util.js`, `../lib.js`, by where each is written, whether or not
    /// it has been yet.
    fn specifier(&self, from: &[String], to: &[String]) -> String {
        self.file_specifier(from, &self.js_path(to))
    }

    /// `to`, a file, as module `from`'s file imports it: `./a.js`,
    /// `../../b.js`. The one way a specifier of a file is made.
    fn file_specifier(&self, from: &[String], to: &std::path::Path) -> String {
        crate::paths::specifier(parent_dir(&self.js_path(from)), to)
    }

    /// A relative module `specifier` a binding names, `#[link_name]`'s,
    /// which is relative to the root's file (ADR 0028), as module `from`'s
    /// file imports it: `./greet.js` is `../greet.js` one down. A package's
    /// name stays as it is.
    fn linked_specifier(&self, from: &[String], specifier: &str) -> String {
        if !crate::paths::is_relative_specifier(specifier) {
            return specifier.to_string();
        }
        let dir = parent_dir(&self.output);
        let file = specifier_file(dir, specifier).unwrap_or_else(|_| dir.join(specifier));
        self.file_specifier(from, &file)
    }
}

impl OutputPlan {
    pub fn new(input: PathBuf, output: PathBuf, test: bool, manifest: Option<PathBuf>) -> Self {
        Self {
            input,
            output,
            test,
            manifest,
            metadata: None,
            extra: Vec::new(),
            dep_info: None,
            settings: crate::settings::Settings::default(),
            std_bodies: false,
            written: Vec::new(),
            read: Vec::new(),
            jsx: HashSet::new(),
            located: HashMap::new(),
        }
    }

    pub fn plan(&mut self, linked: crate::link::Linked, sources: Vec<PathBuf>) -> Result<ArtifactPlan, String> {
        let mut lowered = linked.into_program();
        self.jsx = lowered
            .modules
            .iter()
            .filter(|m| m.jsx)
            .map(|m| m.path.clone())
            .collect();
        // A `#[path]` module's file, below the root's directory as the output
        // is: `pages/errors/[errorCode].rs` (ADR 0273). One outside it,
        // `../shared/model.rs`, is where its module path says, in the output's.
        self.located = lowered
            .modules
            .iter()
            .filter(|m| m.located)
            .filter_map(|m| {
                let below = relative(parent_dir(&self.input), m.file.as_ref()?);
                (!below.starts_with("..")).then(|| (m.path.clone(), parent_dir(&self.output).join(below)))
            })
            .collect();
        let sources: Vec<PathBuf> = sources.into_iter().map(|p| absolute(&p)).collect::<Result<_, _>>()?;
        if let Some(library) = &mut lowered.library {
            library.inputs = sources
                .iter()
                .map(|file| {
                    let bytes = std::fs::read(file).map_err(|e| e.to_string())?;
                    Ok(manifest::Artifact {
                        file: file.clone(),
                        hash: fingerprint(&bytes),
                    })
                })
                .collect::<Result<_, String>>()?;
        }
        let mut planned = Vec::new();
        let mut modules = Vec::new();
        for module in &lowered.modules {
            let requested = self.js_path(&module.path);
            let file = absolute(&requested)?;
            let map = absolute(&PathBuf::from(format!("{}.map", requested.display())))?;
            planned.extend([file.clone(), map.clone()]);
            let types = match self.settings.declarations && module.declarations.is_some() {
                true => Some(absolute(&types_path(&requested))?),
                false => None,
            };
            planned.extend(types.clone());
            let imports = module
                .imports
                .iter()
                .map(|import| absolute(&self.js_path(&import.path)))
                .collect::<Result<Vec<_>, _>>()?;
            modules.push(manifest::Module {
                module: module.path.clone(),
                file,
                map,
                types,
                source: module.file.as_deref().map(absolute).transpose()?,
                in_source: Some(self.in_source(module)?),
                imports,
                exports: exports(module),
                links: Vec::new(),
                type_links: Vec::new(),
            });
        }
        let test_artifact = self.tests(&lowered.tests);
        let tests = test_artifact.as_ref().map(|test| absolute(&test.path)).transpose()?;
        if let Some(test) = &test_artifact {
            planned.push(absolute(&test.path)?);
        }
        // A library's metadata, as rustc wrote it: published where it's asked
        // for, with what's made from the same build, or not at all.
        let metadata = match &self.metadata {
            Some((written, path)) => {
                let path = absolute(path)?;
                planned.push(path.clone());
                let bytes = std::fs::read(written).map_err(|e| format!("cannot read the metadata rustc wrote: {e}"))?;
                Some(Artifact { path, bytes })
            }
            None => None,
        };
        let mut extra = Vec::new();
        for (path, bytes) in std::mem::take(&mut self.extra) {
            let path = absolute(&path)?;
            planned.push(path.clone());
            extra.push(Artifact { path, bytes });
        }
        if let Some((path, _)) = &self.dep_info {
            planned.push(absolute(path)?);
        }
        let input = absolute(&self.input)?;
        let output = absolute(&self.output)?;
        let manifest_path = self.manifest.as_deref().map(absolute).transpose()?;
        let mut paths = HashSet::new();
        for path in planned.iter().chain(manifest_path.iter()) {
            if sources.contains(path) || path == &input || !paths.insert(path.clone()) {
                return Err(format!(
                    "output collision at `{}`; pick another output name",
                    path.display()
                ));
            }
            if path.exists() && !path.is_file() {
                return Err(format!("output `{}` is not a file", path.display()));
            }
        }

        let (mut artifacts, configs) = self.generate(lowered.modules, &lowered.sources)?;
        self.read = configs;
        if let Some((path, write)) = self.dep_info.take() {
            extra.push(Artifact {
                path: absolute(&path)?,
                bytes: write(&self.read)?,
            });
        }
        artifacts.extend(test_artifact);
        artifacts.extend(metadata);
        for artifact in &mut artifacts {
            artifact.path = absolute(&artifact.path)?;
        }
        // Where each module's JS and declarations name another's file, by a
        // parse of what was printed: what a host that copies them elsewhere
        // rewrites (ADR 0101).
        for module in &mut modules {
            module.links = links(&artifacts, &module.file, false)?;
            if let Some(types) = &module.types {
                module.type_links = links(&artifacts, types, true)?;
            }
        }
        self.written = artifacts
            .iter()
            .filter(|a| a.path.extension().is_some_and(|ext| ext == "js" || ext == "jsx"))
            .map(|a| a.path.clone())
            .collect();

        let mut stale = Vec::new();
        if let Some(path) = manifest_path {
            // Only remove files owned by this input/output pair, still containing
            // exactly the bytes this compiler wrote. User edits are preserved.
            if path.exists() {
                let previous =
                    Manifest::read_from(&path).map_err(|e| format!("invalid manifest `{}`: {e}", path.display()))?;
                if previous.input != input || previous.output != output {
                    return Err(format!("manifest `{}` belongs to another compilation", path.display()));
                }
                for file in previous.artifacts {
                    let old = absolute(&file.file)?;
                    let generated = ["js", "jsx", "map"]
                        .iter()
                        .any(|ext| old.extension().is_some_and(|e| e == *ext))
                        || old.to_string_lossy().ends_with(".d.ts");
                    if generated
                        && old.starts_with(parent_dir(&output))
                        && !sources.contains(&old)
                        && !paths.contains(&old)
                        && old != input
                    {
                        stale.push(manifest::Artifact {
                            file: old,
                            hash: file.hash,
                        });
                    }
                }
            }
            let manifest = Manifest {
                library: lowered.library,
                version: manifest::VERSION,
                compiler: Some(manifest::Compiler::current()),
                input,
                output,
                sources,
                modules,
                artifacts: artifacts
                    .iter()
                    .map(|a| manifest::Artifact {
                        file: a.path.clone(),
                        hash: fingerprint(&a.bytes),
                    })
                    .collect(),
                // The publisher's, where it isn't the compiler: a WASI host's.
                stale: if cfg!(target_os = "wasi") {
                    stale.clone()
                } else {
                    Vec::new()
                },
                tests,
            };
            let manifest = match self.std_bodies {
                true => manifest.relative_to(parent_dir(&path)),
                false => manifest,
            };
            // What Cargo is told of the build, then the manifest, last: a
            // host that sees it sees all the build wrote.
            artifacts.extend(std::mem::take(&mut extra));
            artifacts.push(Artifact {
                path,
                bytes: format!(
                    "{}\n",
                    serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?
                )
                .into_bytes(),
            });
        }
        artifacts.extend(extra);
        Ok(ArtifactPlan { artifacts, stale })
    }
}

/// What `module` exports, by name, as it's printed: its exported functions,
/// constants and namespaces, what it re-exports, by the name it gives, and
/// `default`, of its default export.
fn exports(module: &program::LoweredModule) -> Vec<String> {
    let own = module.items.iter().filter_map(|item| match item {
        js::Item::Function(function) if function.export => Some(function.name.clone()),
        js::Item::Const(constant) if constant.export => Some(constant.name.clone()),
        js::Item::Namespace(namespace) if namespace.export => Some(namespace.name.clone()),
        _ => None,
    });
    let reexported =
        (module.reexports.iter()).flat_map(|reexport| reexport.named.iter().map(|(_, alias)| alias.clone()));
    let default = module.default_export.as_ref().map(|_| "default".to_string());
    own.chain(reexported).chain(default).collect()
}

/// What a build writes and what it leaves stale, constructed by `plan` alone,
/// after every name and content has been validated: its fields are its own.
pub(crate) struct ArtifactPlan {
    artifacts: Vec<Artifact>,
    stale: Vec<manifest::Artifact>,
}

impl ArtifactPlan {
    /// Carry the plan out: the publisher writes what it says.
    pub(crate) fn publish(self) -> Result<(), String> {
        crate::publish::publish(&self.artifacts, &self.stale)
    }
}

/// Each relative specifier in `file`, as written, by its bytes, and the file
/// it names: none of a file not written, or of a package, `"react"`.
fn links(artifacts: &[Artifact], file: &std::path::Path, declarations: bool) -> Result<Vec<manifest::Link>, String> {
    let Some(artifact) = artifacts.iter().find(|artifact| artifact.path == file) else {
        return Ok(Vec::new());
    };
    let code = std::str::from_utf8(&artifact.bytes).map_err(|e| e.to_string())?;
    to_oxc::module_specifiers(code, declarations)?
        .into_iter()
        .filter(|(_, _, specifier)| crate::paths::is_relative_specifier(specifier))
        .map(|(start, end, specifier)| {
            let file = specifier_file(parent_dir(file), &specifier)?;
            Ok(manifest::Link { start, end, file })
        })
        .collect()
}

/// `Tag.d.ts` of `Tag.jsx` or `Tag.js`: its declarations' (ADR 0196).
fn types_path(js: &std::path::Path) -> PathBuf {
    js.with_extension("d.ts")
}
