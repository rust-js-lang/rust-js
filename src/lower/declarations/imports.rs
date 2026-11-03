//! What a module's declarations import, each type it names of another
//! module, and re-export, each as its JS has it (ADRs 0210, 0240).

use std::collections::BTreeSet;

use serde_json::{Value, json};

/// A statement's types of one module a module's declarations re-export,
/// `export type { Kind, Size as Measure } from "./shapes.js";`.
type TypeExport<'a> = (
    rustc_span::Span,
    &'a Option<crate::js::ImportFrom>,
    Vec<&'a (String, String)>,
);

/// The types of JS modules, of the crate's other modules or of a library's
/// it imports, each by its module, its path or its file's specifier, its
/// name and whether it's that module's default.
type ModuleImports<K> = BTreeSet<(K, String, bool)>;

/// The declarations that import and re-export, first in a `.d.ts`: the
/// types of JS modules', `import type { ReactNode } from "react";`; of
/// libraries' modules, by their file's specifier, `import type { Context }
/// from "../react/ReactContext.js";`, as a library's functions are
/// imported (ADR 0210, amended); of the crate's other modules, by their
/// path, which the output makes their file's (ADR 0210); then its `pub
/// use`s of another module's functions, `export { Challenges } from
/// "./Challenges.js";` (ADR 0240), a JS module's by its specifier, which
/// the output makes the file's too; and of types, `export type {
/// TemporaryReferenceSet } from ".."`, a statement's of one module one
/// (rd642).
pub(super) fn imports(
    (packages, libraries, modules): (
        &ModuleImports<String>,
        &ModuleImports<String>,
        &ModuleImports<Vec<String>>,
    ),
    (reexports, types): (&[crate::program::Reexport], &[crate::lower::reexports::TypeReexport]),
) -> Vec<Value> {
    let mut declarations = Vec::new();
    for (from, names, default) in of_modules(packages) {
        declarations.extend(type_imports(json!({ "from": from }), names, default));
    }
    for (from, names, default) in of_modules(libraries) {
        declarations.extend(type_imports(json!({ "specifier": from }), names, default));
    }
    for (path, names, default) in of_modules(modules) {
        declarations.extend(type_imports(json!({ "module": path }), names, default));
    }
    for reexport in reexports {
        let from = match &reexport.from {
            crate::js::ImportFrom::Module(path) => json!({ "module": path }),
            crate::js::ImportFrom::Specifier(specifier) => json!({ "specifier": specifier }),
        };
        let mut declaration = json!({ "kind": "export", "names": reexport.named });
        declaration
            .as_object_mut()
            .expect("an object")
            .extend(from.as_object().expect("an object").clone());
        declarations.push(declaration);
    }
    let mut grouped: Vec<TypeExport<'_>> = Vec::new();
    for (at, from, named) in types {
        match grouped.last_mut() {
            Some((was, was_from, names)) if was.lo() == at.lo() && *was_from == from => names.push(named),
            _ => grouped.push((*at, from, vec![named])),
        }
    }
    for (_, from, names) in grouped {
        let from = match from {
            Some(crate::js::ImportFrom::Module(path)) => json!({ "module": path }),
            Some(crate::js::ImportFrom::Specifier(specifier)) => json!({ "specifier": specifier }),
            None => json!({}),
        };
        let mut declaration = json!({ "kind": "export", "names": names, "typeOnly": true });
        (declaration.as_object_mut().expect("an object")).extend(from.as_object().expect("an object").clone());
        declarations.push(declaration);
    }
    declarations
}

impl super::Declarations<'_, '_> {
    /// Another module's item `did`, imported from `file`, as its default where it is (ADR 0761).
    pub(super) fn imported(&mut self, file: Vec<String>, did: rustc_span::def_id::DefId, name: String) {
        let default = crate::lower::default_exports::exported_as(self.tcx, did, &name) == "default";
        self.module_imports.insert((file, name, default));
    }
}

/// The modules whose namespaces the declarations name, each imported whole,
/// `import type * as ReactFiberReconciler from "./ReactFiberReconciler.js";`
/// (ADR 1037): of the crate's modules by their path, of libraries' by their
/// file's specifier, as their types are.
pub(super) fn namespaces(imported: &BTreeSet<(Result<Vec<String>, String>, String)>) -> Vec<Value> {
    (imported.iter())
        .map(|(from, name)| {
            let mut import = json!({ "kind": "import", "names": [], "namespace": name, "typeOnly": true });
            let from = match from {
                Ok(path) => json!({ "module": path }),
                Err(specifier) => json!({ "specifier": specifier }),
            };
            (import.as_object_mut().expect("an object")).extend(from.as_object().expect("an object").clone());
            import
        })
        .collect()
}

/// The type-only imports of a module, `from`, of its default and `names`:
/// one of each, `import type Complex from ..;` and `import type { Parts }
/// from ..;`, as TypeScript allows a type-only import a default or named
/// bindings, not both (#373).
fn type_imports(from: Value, names: Vec<&str>, default: Option<&str>) -> Vec<Value> {
    let import = |names: Vec<&str>, default: Option<&str>| {
        let mut import = json!({ "kind": "import", "names": names, "default": default, "typeOnly": true });
        (import.as_object_mut().expect("an object")).extend(from.as_object().expect("an object").clone());
        import
    };
    match default {
        Some(default) if !names.is_empty() => vec![import(Vec::new(), Some(default)), import(names, None)],
        _ => vec![import(names, default)],
    }
}

/// Each module of `imported`, in order, the names imported from it, and
/// its default, `Complex` and `Parts` of `Complex, { Parts }`.
fn of_modules<K: Ord>(imported: &ModuleImports<K>) -> Vec<(&K, Vec<&str>, Option<&str>)> {
    let modules: BTreeSet<&K> = imported.iter().map(|(module, ..)| module).collect();
    (modules.into_iter())
        .map(|module| {
            let of = |default: bool| (imported.iter()).filter(move |(m, _, d)| m == module && *d == default);
            let names: Vec<&str> = of(false).map(|(_, n, _)| n.as_str()).collect();
            (module, names, of(true).map(|(_, n, _)| n.as_str()).next())
        })
        .collect()
}
