//! A rewrite of many places, each one a call of its own, `<module>::at_<k>(..)`,
//! narrowed to those rustc accepts (#1204): rustc's errors of the program
//! (`--error-format=json`), each mapped by where it is to the places it's
//! of, which the rewrite then takes off, or changes, and tries again.

use std::collections::BTreeMap;
use std::ops::Range;
use std::process::Command;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

/// One of rustc's errors: its code, `E0277`, its message, and where it and
/// its notes are in the program, by bytes.
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub spans: Vec<Range<usize>>,
}

/// rustc's errors of `program`, a binary crate checked as `cargo check`
/// does, at `METAMORPH_EDITION`'s edition: `None` where it accepts it.
pub fn diagnostics(program: &str) -> Option<Vec<Diagnostic>> {
    let edition = std::env::var("METAMORPH_EDITION").unwrap_or_else(|_| String::from("2024"));
    let dir = std::env::temp_dir().join(format!("metamorph-narrow-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a temporary folder");
    let file = dir.join("narrowed.rs");
    std::fs::write(&file, program).expect("the program written");
    let out = Command::new("rustc")
        .args([
            &format!("--edition={edition}"),
            "--crate-type=bin",
            "--emit=metadata",
            "-Awarnings",
            "--error-format=json",
            "--crate-name=narrowed",
        ])
        .arg("-o")
        .arg(dir.join("narrowed.rmeta"))
        .arg(&file)
        .output();
    let _ = std::fs::remove_dir_all(&dir);
    let out = out.ok()?;
    if out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stderr);
    Some(
        text.lines()
            .filter(|line| line.contains("\"level\":\"error\""))
            .filter_map(parse)
            .collect(),
    )
}

/// A JSON string's text from `at`, past its opening quote, to its closing
/// one, its escapes read.
fn string_at(text: &str, at: usize) -> String {
    let mut out = String::new();
    let mut chars = text[at..].chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => break,
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(c) => out.push(c),
                None => break,
            },
            c => out.push(c),
        }
    }
    out
}

/// A diagnostic's line of rustc's JSON: its message, its code, and each
/// span of the program's own file, `narrowed.rs`, its notes' too.
fn parse(line: &str) -> Option<Diagnostic> {
    let message = string_at(line, line.find("\"message\":\"")? + "\"message\":\"".len());
    let code = line
        .find("\"code\":{\"code\":\"")
        .map(|at| string_at(line, at + "\"code\":{\"code\":\"".len()))
        .unwrap_or_default();
    let mut spans = Vec::new();
    let key = "\"file_name\":\"";
    let mut rest = line;
    while let Some(at) = rest.find(key) {
        rest = &rest[at + key.len()..];
        let file = string_at(rest, 0);
        let number = |name: &str, from: &str| -> Option<usize> {
            let at = from.find(name)? + name.len();
            from[at..].split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()
        };
        if file.ends_with("narrowed.rs")
            && let (Some(start), Some(end)) = (number("\"byte_start\":", rest), number("\"byte_end\":", rest))
        {
            spans.push(start..end);
        }
    }
    Some(Diagnostic { code, message, spans })
}

/// Where each place `<module>::at_<k>` is in `program`: the call, or the
/// cast of it, `at_k(..) as T`, and the function `at_k` itself.
pub fn sites(program: &str, module: &str) -> BTreeMap<usize, Vec<Range<usize>>> {
    let Ok(ast) = syn::parse_file(program) else {
        return BTreeMap::new();
    };
    let mut found = Sites {
        module,
        found: BTreeMap::new(),
    };
    found.visit_file(&ast);
    found.found
}

struct Sites<'m> {
    module: &'m str,
    found: BTreeMap<usize, Vec<Range<usize>>>,
}

impl Sites<'_> {
    /// The place `e` calls, `module::at_k(..)`, if it's one.
    fn place(&self, e: &syn::Expr) -> Option<usize> {
        let syn::Expr::Call(call) = e else { return None };
        let syn::Expr::Path(path) = &*call.func else {
            return None;
        };
        let segments: Vec<String> = path.path.segments.iter().map(|s| s.ident.to_string()).collect();
        match segments.as_slice() {
            [m, at] if m == self.module => at.strip_prefix("at_")?.parse().ok(),
            _ => None,
        }
    }
}

impl<'a> Visit<'a> for Sites<'_> {
    fn visit_expr(&mut self, e: &'a syn::Expr) {
        let at = match e {
            syn::Expr::Cast(cast) => self.place(&cast.expr),
            e => self.place(e),
        };
        if let Some(k) = at {
            self.found.entry(k).or_default().push(e.span().byte_range());
        }
        visit::visit_expr(self, e);
    }

    fn visit_item_mod(&mut self, m: &'a syn::ItemMod) {
        if m.ident == self.module
            && let Some((_, items)) = &m.content
        {
            for item in items {
                if let syn::Item::Fn(f) = item
                    && let Some(k) = f.sig.ident.to_string().strip_prefix("at_").and_then(|k| k.parse().ok())
                {
                    self.found.entry(k).or_default().push(f.span().byte_range());
                }
            }
        }
        visit::visit_item_mod(self, m);
    }
}

/// The places an error is of: for each of its spans, the innermost place
/// that holds it, or whose call it follows, `at_k(..).peek()`; or else the
/// place made last before it, of a value held and used later, `let p =
/// at_k(..); p.peek()`, whose error is where it's used.
pub fn places_of(program: &str, sites: &BTreeMap<usize, Vec<Range<usize>>>, d: &Diagnostic) -> Vec<usize> {
    let ranges = || sites.iter().flat_map(|(&k, ranges)| ranges.iter().map(move |r| (k, r)));
    let mut places = Vec::new();
    for span in &d.spans {
        let holding = ranges()
            .filter(|(_, r)| r.start <= span.start && span.start < r.end.max(r.start + 1))
            .min_by_key(|(_, r)| r.end - r.start);
        let following = || {
            ranges()
                .filter(|(_, r)| {
                    r.end <= span.start
                        && program
                            .get(r.end..span.start)
                            .is_some_and(|between| between.chars().all(|c| c.is_whitespace() || c == '.'))
                })
                .max_by_key(|(_, r)| r.end)
        };
        let before = || ranges().filter(|(_, r)| r.end <= span.start).max_by_key(|(_, r)| r.end);
        if let Some((k, _)) = holding.or_else(following).or_else(before)
            && !places.contains(&k)
        {
            places.push(k);
        }
    }
    places
}
