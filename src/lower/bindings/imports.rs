//! What a binding's path imports from a JS module (ADR 0028): the export
//! it names, the rest of its path, and what a default or namespace import
//! is called.

/// What a JS module exports under a name, `("node:path", "join")`, or
/// `"default"` or `"*"` for its default export or the module itself.
pub(in crate::lower) type Export = (String, String);

/// An import from a JS module (ADR 0028): `"@codemirror/state#EditorState.create"`
/// is the export `("@codemirror/state", "EditorState")`, then the rest of
/// the path, `".create"`. The last `#` splits them, since a module's name
/// can start with one (Node's `#internal`).
/// The module itself may be named as JS's `import * as Scheduler` writes
/// it, `"scheduler#* as Scheduler.unstable_now"`: the export `"*"`, then
/// `".unstable_now"`, the name `namespace_name`'s (ADR 0256, amended).
pub(in crate::lower) fn js_import(path: &str) -> Option<(Export, &str)> {
    let (from, path) = path.rsplit_once('#')?;
    if let Some(named) = path.strip_prefix("* as ") {
        let name = named.split('.').next().unwrap_or_default();
        return (!from.is_empty() && !name.is_empty())
            .then(|| ((from.to_string(), "*".to_string()), &named[name.len()..]));
    }
    let export = path.split('.').next().unwrap_or_default();
    (!from.is_empty() && !export.is_empty()).then(|| ((from.to_string(), export.to_string()), &path[export.len()..]))
}

/// What a namespace import `js_import` reads is named, `Scheduler` of
/// `"scheduler#* as Scheduler.unstable_now"`, where its path names it.
pub(in crate::lower) fn namespace_name(path: &str) -> Option<&str> {
    let (_, path) = path.rsplit_once('#')?;
    let named = path.strip_prefix("* as ")?;
    named.split('.').next().filter(|name| !name.is_empty())
}

/// What a default or namespace import is called: after its module, as in
/// ReScript. `./greet.js` is `greet`, and `@codemirror/lang-rust` is `langRust`.
pub(in crate::lower) fn module_binding(from: &str) -> String {
    stem_binding(file_stem(from))
}

/// A JS module's file's name without its directory or extension,
/// `ChangeEventPlugin` of `./src/events/plugins/ChangeEventPlugin.js`, a
/// package's of its name, `scheduler`, `react-context-menu` of
/// `@radix-ui/react-context-menu`.
pub(in crate::lower) fn file_stem(from: &str) -> &str {
    let file = from.rsplit(['/', ':']).find(|s| !s.is_empty()).unwrap_or_default();
    file.split('.').next().unwrap_or_default()
}

/// A JS name of a file's stem, `module_binding`'s: `langRust` of
/// `lang-rust`, `ChangeEventPlugin` of itself.
pub(in crate::lower) fn stem_binding(stem: &str) -> String {
    // Its words, `lang` and `rust`, joined as a Rust name's are in JS.
    let words: Vec<&str> = stem
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '$')
        .filter(|w| !w.is_empty())
        .collect();
    let name = crate::lower::camel_case(&words.join("_"));
    if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{name}")
    } else {
        name
    }
}
