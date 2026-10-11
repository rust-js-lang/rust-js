//! JavaScript identifier policy shared by lowering and linking.

use std::collections::HashSet;

/// Pick an unused name: `x`, then `x$1`, `x$2`, ... Rust identifiers can't
/// contain `$`, so these never clash with a user's name.
pub(crate) fn fresh_in(taken: &mut HashSet<String>, base: &str) -> String {
    let base = js_ident(base);
    if taken.insert(base.clone()) {
        return base;
    }
    // One given apart already, `caption$1`: the name it was given apart
    // from's next, `caption$2`, which reclaiming can make `caption` again
    // (ADR 0352).
    let root = match base.rsplit_once('$') {
        Some((root, k)) if !root.is_empty() && !k.is_empty() && k.bytes().all(|b| b.is_ascii_digit()) => root,
        _ => &base,
    };
    (1..)
        .map(|k| format!("{root}${k}"))
        .find(|name| taken.insert(name.clone()))
        .unwrap()
}

/// Rust names that mean something else in JS get a `$` suffix.
pub(crate) fn js_ident(name: &str) -> String {
    const RESERVED: &[&str] = &[
        "arguments",
        "await",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "debugger",
        "default",
        "delete",
        "do",
        "else",
        "enum",
        "eval",
        "export",
        "extends",
        "false",
        "finally",
        "for",
        "function",
        "if",
        "implements",
        "import",
        "in",
        "instanceof",
        "interface",
        "let",
        "new",
        "null",
        "package",
        "private",
        "protected",
        "public",
        "return",
        "static",
        "super",
        "switch",
        "this",
        "throw",
        "true",
        "try",
        "typeof",
        "var",
        "void",
        "while",
        "with",
        "yield",
        "undefined",
        "NaN",
        "Infinity",
        "Math",
        "Error",
        "String",
        "WeakMap",
        "DataView",
        "ArrayBuffer",
        "Number",
        "BigInt",
        "Object",
    ];
    if RESERVED.contains(&name) {
        format!("{name}$")
    } else {
        name.to_string()
    }
}
