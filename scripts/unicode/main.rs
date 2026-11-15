// What Rust's std answers of every `char`, for scripts/unicode.ts (#1888,
// ADR 1163): each Unicode question rust-js's runtime asks, asked of the
// pinned toolchain's own std, so its tables are Rust's whatever Unicode
// version a JS engine has. Printed as JSON: each set as the boundaries of
// its ranges, `[start, end)` pairs in order; each case map as its changed
// `char`s and what each becomes.
#![feature(titlecase, case_ignorable)]

use std::fmt::Write;

/// The boundaries of the ranges of `char`s `has` holds.
fn boundaries(has: impl Fn(char) -> bool) -> Vec<u32> {
    let mut out = Vec::new();
    let mut inside = false;
    for code in 0..=0x11_0000u32 {
        let now = char::from_u32(code).is_some_and(&has);
        if now != inside {
            out.push(code);
            inside = now;
        }
    }
    out
}

/// Whether `{:?}` shows `escaped` as a `\u{..}` escape.
fn unicode_escape(escaped: &str) -> bool {
    escaped.starts_with("\\u{")
}

/// Each `char` `map` changes, and the `char`s it becomes.
fn changed(map: impl Fn(char) -> String) -> Vec<(u32, Vec<u32>)> {
    (0..=0x10_FFFFu32)
        .filter_map(char::from_u32)
        .filter_map(|c| {
            let to = map(c);
            (to != c.to_string()).then(|| (c as u32, to.chars().map(u32::from).collect()))
        })
        .collect()
}

fn list(out: &mut String, name: &str, values: &[u32]) {
    let joined: Vec<String> = values.iter().map(u32::to_string).collect();
    write!(out, "\"{name}\":[{}],", joined.join(",")).unwrap();
}

fn map(out: &mut String, name: &str, entries: &[(u32, Vec<u32>)]) {
    let joined: Vec<String> = (entries.iter())
        .map(|(from, to)| {
            let to: Vec<String> = to.iter().map(u32::to_string).collect();
            format!("[{from},[{}]]", to.join(","))
        })
        .collect();
    write!(out, "\"{name}\":[{}],", joined.join(",")).unwrap();
}

fn main() {
    let (major, minor, update) = char::UNICODE_VERSION;
    let mut out = format!("{{\"unicode\":\"{major}.{minor}.{update}\",");
    list(&mut out, "alphabetic", &boundaries(char::is_alphabetic));
    list(&mut out, "numeric", &boundaries(char::is_numeric));
    list(&mut out, "uppercase", &boundaries(char::is_uppercase));
    list(&mut out, "lowercase", &boundaries(char::is_lowercase));
    list(&mut out, "whitespace", &boundaries(char::is_whitespace));
    // `str::to_lowercase`'s Final_Sigma: what it skips, and what it asks.
    list(&mut out, "caseIgnorable", &boundaries(char::is_case_ignorable));
    list(&mut out, "cased", &boundaries(char::is_cased));
    // `{:?}`'s escapes: of a `char` after another, which isn't escaped
    // for being Grapheme_Extend; and of one alone, which is.
    let named = |c: char| matches!(c, '\\' | '\'' | '"' | '\n' | '\r' | '\t' | '\0');
    let after = |c: char| unicode_escape(&format!("a{c}").escape_debug().to_string()[1..]);
    list(&mut out, "debugEscaped", &boundaries(|c| !named(c) && after(c)));
    list(
        &mut out,
        "debugExtended",
        &boundaries(|c| !named(c) && !after(c) && unicode_escape(&c.escape_debug().to_string())),
    );
    map(&mut out, "upper", &changed(|c| c.to_uppercase().collect()));
    map(&mut out, "lower", &changed(|c| c.to_lowercase().collect()));
    out.pop();
    out.push('}');
    println!("{out}");
}
