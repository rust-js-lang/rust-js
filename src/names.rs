//! JavaScript identifier policy shared by lowering and linking.

use crate::collections::IndexSet;

/// Pick an unused name: `x`, then `x$1`, `x$2`, ... Rust identifiers can't
/// contain `$`, so these never clash with a user's name.
pub(crate) fn fresh_in(taken: &mut IndexSet<String>, base: &str) -> String {
    fresh_apart(taken, &js_ident(base))
}

/// `base`, or given apart from what `taken` has, `base$1`, as `fresh_in`
/// names, of a name that's fine as it is: a property's, `new` too.
pub(crate) fn fresh_apart(taken: &mut IndexSet<String>, base: &str) -> String {
    if taken.insert(base.to_string()) {
        return base.to_string();
    }
    // One given apart already, `caption$1`: the name it was given apart
    // from's next, `caption$2`, which reclaiming can make `caption` again
    // (ADR 0352).
    let root = apart(base).map_or(base, |(root, _)| root);
    (1..)
        .map(|k| given_apart(root, k))
        .find(|name| taken.insert(name.clone()))
        .unwrap()
}

/// `base` given apart the `k`th time, `caption$2`: the one spelling of a
/// name given apart, which `apart` takes apart.
pub(crate) fn given_apart(base: &str, k: usize) -> String {
    format!("{base}${k}")
}

/// A name given apart, `caption$2`: what it was given apart from, and how
/// many times, `("caption", 2)`. Not a word JS keeps, `Error$`, nor a
/// name never given apart.
pub(crate) fn apart(name: &str) -> Option<(&str, usize)> {
    let (base, k) = name.rsplit_once('$')?;
    if base.is_empty() || k.is_empty() || !k.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((base, k.parse().ok()?))
}

/// A name lowering makes that isn't spelled yet, the `k`th of `base`'s,
/// `value#1` (#1681 change 3, M2): what M3's names table will hold for a
/// name the compiler makes, a variable of its own and not the program's.
/// No JS reads it, nor does `apart` take it for a name given apart, so tidy
/// takes it for no kin of the program's `value`; preparation spells each
/// once, apart from every name its module has (`prepare::reclaim::spelled`),
/// and the validator refuses one that reaches printing.
pub(crate) fn unspelled(base: &str, k: usize) -> String {
    format!("{base}#{k}")
}

/// Of a name not spelled yet, `value#1` (`unspelled`), what it's of,
/// `value`; none of any other.
pub(crate) fn unspelled_base(name: &str) -> Option<&str> {
    let (base, k) = name.rsplit_once('#')?;
    (!base.is_empty() && !k.is_empty() && k.bytes().all(|b| b.is_ascii_digit())).then_some(base)
}

/// The cache of a trait's dictionary, `$circleShape` of `circleShape()`,
/// apart from its module's other caches, `taken`: `$`, which no Rust name
/// has, as the runtime's helpers are named. One a helper has, `$durationAdd`
/// of a `Duration` of the crate's own, is given apart, `$durationAdd$1`: its
/// module may import that helper too.
pub(crate) fn dictionary_cache(taken: &mut IndexSet<String>, dictionary: &str) -> String {
    loop {
        let cache = fresh_in(taken, &format!("${dictionary}"));
        if !crate::runtime::exports(&cache) {
            return cache;
        }
    }
}

/// The words JS keeps for itself, which no variable may be: a Rust name
/// that's one is named apart for good, `class$`.
const KEPT: &[&str] = &[
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
];

/// JS's globals, ECMAScript's and those of the hosts' rust-js reads,
/// `console` and `TextEncoder`: a Rust name that is one is named apart,
/// `Error$`, as the output may read the global; it may be its name again
/// where nothing in its module reads it (ADR 0352).
const GLOBALS: &[&str] = &[
    "Array",
    "ArrayBuffer",
    "Atomics",
    "BigInt",
    "BigInt64Array",
    "BigUint64Array",
    "Boolean",
    "DataView",
    "Date",
    "Error",
    "EvalError",
    "FinalizationRegistry",
    "Float32Array",
    "Float64Array",
    "Function",
    "Int8Array",
    "Int16Array",
    "Int32Array",
    "Iterator",
    "JSON",
    "Map",
    "Math",
    "Number",
    "Object",
    "Promise",
    "Proxy",
    "RangeError",
    "ReferenceError",
    "Reflect",
    "RegExp",
    "Set",
    "SharedArrayBuffer",
    "String",
    "Symbol",
    "SyntaxError",
    "TypeError",
    "URIError",
    "Uint8Array",
    "Uint8ClampedArray",
    "Uint16Array",
    "Uint32Array",
    "WeakMap",
    "WeakRef",
    "WeakSet",
    "TextDecoder",
    "TextEncoder",
    "console",
    "globalThis",
    "queueMicrotask",
    "structuredClone",
];

/// Whether JS keeps `name` for itself, so a variable named so is named apart
/// for good.
/// Whether `name` is one of JS's globals, `GLOBALS`', which the output's
/// own names are named apart from wherever its module reads it (ADR 0352).
pub(crate) fn is_js_global(name: &str) -> bool {
    GLOBALS.contains(&name)
}

pub(crate) fn kept_word(name: &str) -> bool {
    KEPT.contains(&name)
}

/// Whether `name` is a JS name, which a key or a property may be as it is.
pub(crate) fn is_identifier(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// Rust names that mean something else in JS get a `$` suffix.
pub(crate) fn js_ident(name: &str) -> String {
    if KEPT.contains(&name) || GLOBALS.contains(&name) {
        format!("{name}$")
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A cache is apart from the runtime's helpers and from its module's
    // other caches, one named apart already, `durationAdd$1`'s, too.
    #[test]
    fn dictionary_caches_are_apart() {
        let mut taken = IndexSet::default();
        assert_eq!(dictionary_cache(&mut taken, "circleShape"), "$circleShape");
        assert_eq!(dictionary_cache(&mut taken, "durationAdd"), "$durationAdd$1");
        assert_eq!(dictionary_cache(&mut taken, "durationAdd$1"), "$durationAdd$2");
        assert_eq!(dictionary_cache(&mut taken, "circleShape"), "$circleShape$1");
    }
}
