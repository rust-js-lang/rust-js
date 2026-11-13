//! The class names a CSS Module exports, the one thing rust-js reads of a
//! stylesheet (#1512): its text in, its export keys out, sorted, or why it
//! can't say them. Lightning CSS lists them, with its CSS Modules config
//! and nothing else; this function is all of rust-js that knows of it, so
//! another parser can stand behind it.

use lightningcss::css_modules::Config;
use lightningcss::declaration::DeclarationBlock;
use lightningcss::error::{ErrorLocation, ParserError};
use lightningcss::properties::Property;
use lightningcss::properties::list::{CounterStyle, ListStyleType, PredefinedCounterStyle};
use lightningcss::rules::{CssRule, CssRuleList};
use lightningcss::selector::{Component, PseudoClass};
use lightningcss::stylesheet::{ParserOptions, PrinterOptions, StyleSheet};

use crate::program::Exported;

/// Lightning CSS's config of what's exported: Turbopack's, Next's, with
/// `custom_idents`, or what every bundler exports, without. A container's
/// name is a custom identifier, which Next has unhashed (vercel/next.js#71233).
fn config(custom_idents: bool) -> Config {
    Config {
        animation: true,
        grid: false,
        container: false,
        custom_idents,
        dashed_idents: false,
        ..Config::default()
    }
}

/// The keys a CSS Module of `source`, the file `filename`, exports, as its
/// bundler names them as they are (`asIs`), sorted; or why they can't be
/// said: a file Lightning CSS doesn't read, or one of ICSS's `:export` or
/// `:import`, whose keys it doesn't list where css-loader does (#1617).
pub fn exported_keys(source: &str, filename: &str) -> Result<Exported, String> {
    let keys = keys_of(source, filename, false)?;
    let turbopack = keys_of(source, filename, true)?;
    let turbopack_only = turbopack.into_iter().filter(|key| !keys.contains(key)).collect();
    Ok(Exported { keys, turbopack_only })
}

/// The keys of `exported_keys`, Turbopack's, of `custom_idents`, or every
/// bundler's.
fn keys_of(source: &str, filename: &str, custom_idents: bool) -> Result<Vec<String>, String> {
    let options = ParserOptions {
        filename: filename.to_string(),
        css_modules: Some(config(custom_idents)),
        ..ParserOptions::default()
    };
    let mut sheet = StyleSheet::parse(source, options).map_err(|error| match error.kind {
        ParserError::DeprecatedCssModulesValueRule => format!(
            "its `@value`{}: Lightning CSS, Next's CSS parser, refuses it where webpack's css-loader exports its name (#1617)",
            at(error.loc)
        ),
        kind => format!("Lightning CSS, Next's CSS parser, can't read it{}: {kind}", at(error.loc)),
    })?;
    if let Some(name) = sheet.rules.0.iter().find_map(icss_rule) {
        return Err(format!(
            "its `:{name}` is ICSS, whose names Lightning CSS, Next's CSS parser, doesn't export where webpack's css-loader does (#1617)"
        ));
    }
    // A counter style's name, which Lightning CSS exports where a list
    // names it whatever `custom_idents` says, and css-loader and Vite don't.
    if !custom_idents {
        unname_counter_styles(&mut sheet.rules);
    }
    let printed = sheet
        .to_css(PrinterOptions::default())
        .map_err(|error| error.to_string())?;
    let mut keys: Vec<String> = printed.exports.unwrap_or_default().into_keys().collect();
    keys.sort();
    Ok(keys)
}

/// Each list's named counter style, `list-style: thumbs`, in `rules` and
/// each rule they hold, made a predefined one, which nothing exports: what's
/// read of the sheet is its keys alone, never its CSS.
fn unname_counter_styles(rules: &mut CssRuleList<'_>) {
    for rule in &mut rules.0 {
        match rule {
            CssRule::Style(style) => {
                unname_in(&mut style.declarations);
                unname_counter_styles(&mut style.rules);
            }
            CssRule::Nesting(nesting) => {
                unname_in(&mut nesting.style.declarations);
                unname_counter_styles(&mut nesting.style.rules);
            }
            CssRule::NestedDeclarations(nested) => unname_in(&mut nested.declarations),
            CssRule::Media(media) => unname_counter_styles(&mut media.rules),
            CssRule::Supports(supports) => unname_counter_styles(&mut supports.rules),
            CssRule::LayerBlock(layer) => unname_counter_styles(&mut layer.rules),
            CssRule::Container(container) => unname_counter_styles(&mut container.rules),
            CssRule::Scope(scope) => unname_counter_styles(&mut scope.rules),
            CssRule::StartingStyle(starting) => unname_counter_styles(&mut starting.rules),
            CssRule::MozDocument(document) => unname_counter_styles(&mut document.rules),
            _ => {}
        }
    }
}

/// `unname_counter_styles` of one block's declarations.
fn unname_in(block: &mut DeclarationBlock<'_>) {
    let unnamed = || ListStyleType::CounterStyle(CounterStyle::Predefined(PredefinedCounterStyle::Decimal));
    for property in block.declarations.iter_mut().chain(&mut block.important_declarations) {
        match property {
            Property::ListStyleType(kind @ ListStyleType::CounterStyle(CounterStyle::Name(_))) => *kind = unnamed(),
            Property::ListStyle(list)
                if matches!(list.list_style_type, ListStyleType::CounterStyle(CounterStyle::Name(_))) =>
            {
                list.list_style_type = unnamed();
            }
            _ => {}
        }
    }
}

/// Where in the file an error is, `, at line 3, column 7`, its line counted
/// from 1, as an editor does, which Lightning CSS counts from 0.
fn at(location: Option<ErrorLocation>) -> String {
    location.map_or_else(String::new, |at| {
        format!(", at line {}, column {}", at.line + 1, at.column)
    })
}

/// `export` or `import` of a rule of ICSS's, `:export { primary: red }`,
/// which Lightning CSS takes for a style rule of a pseudo-class it doesn't
/// know.
fn icss_rule(rule: &CssRule) -> Option<&'static str> {
    let CssRule::Style(style) = rule else {
        return None;
    };
    let components = style
        .selectors
        .0
        .iter()
        .flat_map(|selector| selector.iter_raw_match_order());
    components.into_iter().find_map(|component| match component {
        Component::NonTSPseudoClass(PseudoClass::Custom { name } | PseudoClass::CustomFunction { name, .. }) => {
            ["export", "import"].into_iter().find(|icss| name.as_ref() == *icss)
        }
        _ => None,
    })
}

/// The binding of the CSS Module `specifier` names, `./site.module.css`,
/// read where it's run, as a binding's `link_name` names it: from the crate
/// root's JS, where `rust-js bindings` is run (ADR 1133).
pub fn binding_of_file(specifier: &str) -> Result<String, String> {
    if !crate::program::is_relative_specifier(specifier) || !specifier.ends_with(".module.css") {
        return Err(format!(
            "`{specifier}` isn't a CSS Module of the app's: name it as its binding does, from the crate root's JS, `./site.module.css`"
        ));
    }
    let source = std::fs::read_to_string(specifier).map_err(|error| format!("reading {specifier}: {error}"))?;
    let name = std::path::Path::new(specifier)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let exported =
        exported_keys(&source, &name).map_err(|why| format!("the classes of `{specifier}` can't be said: {why}"))?;
    Ok(binding(specifier, &exported.keys))
}

/// The binding of a CSS Module of `keys`, imported as `specifier`, `./site.module.css`, as
/// `rust-js bindings` prints it, once (ADR 1133): the struct of its classes,
/// a getter of each, and the module's default import, the form the check
/// reads. A class no Rust name says, or two that would have one name, is
/// left to a note, read by `get []` or a getter of the program's own name.
pub fn binding(specifier: &str, keys: &[String]) -> String {
    let mut named: Vec<(String, &str)> = Vec::new();
    let mut unnamed: Vec<&str> = Vec::new();
    for key in keys {
        match rust_name(key) {
            Some(name) => named.push((name, key)),
            None => unnamed.push(key),
        }
    }
    // Two classes one name would say: neither is the getter's.
    let shared: Vec<String> = named
        .iter()
        .filter(|(name, _)| named.iter().filter(|(other, _)| other == name).count() > 1)
        .map(|(name, _)| name.clone())
        .collect();
    named.retain(|(name, key)| {
        let kept = !shared.contains(name);
        if !kept {
            unnamed.push(key);
        }
        kept
    });
    let mut out = String::from("use std::marker::PhantomData;\n\nuse js::JsObject;\n\n");
    out.push_str(&format!("/// The classes of `{specifier}`.\n"));
    out.push_str(
        "#[cfg_attr(rust_js, rust_js::css_module)]\npub struct Styles(PhantomData<JsObject>);\n\nimpl Styles {\n",
    );
    for (at, (name, key)) in named.iter().enumerate() {
        if at > 0 {
            out.push('\n');
        }
        out.push_str(&format!(
            "    #[cfg_attr(rust_js, rust_js::link_name = \"get {key}\")]\n    pub fn {name}(&self) -> String {{\n        unreachable!()\n    }}\n"
        ));
    }
    out.push_str("}\n");
    if !unnamed.is_empty() {
        unnamed.sort();
        let listed: Vec<String> = unnamed.iter().map(|key| format!("`{key}`")).collect();
        out.push_str(&format!(
            "\n// No getter is written of {}: name each yourself, or read it by\n// `#[cfg_attr(rust_js, rust_js::link_name = \"get []\")]`.\n",
            listed.join(", ")
        ));
    }
    out.push_str(&format!(
        "\nunsafe extern \"Rust\" {{\n    #[link_name = \"{specifier}#default\"]\n    pub safe static STYLES: &'static Styles;\n}}\n"
    ));
    out
}

/// A class's Rust name, `code_block` of `codeBlock` and of `code-block`, a
/// keyword raw, `r#type`; or none, of one no Rust name says, `10`, `self`.
fn rust_name(key: &str) -> Option<String> {
    let mut name = String::new();
    let mut previous: Option<char> = None;
    for c in key.chars() {
        if !c.is_alphanumeric() {
            if !name.is_empty() && !name.ends_with('_') {
                name.push('_');
            }
        } else {
            if c.is_uppercase() && previous.is_some_and(|p| p.is_lowercase() || p.is_numeric()) && !name.ends_with('_')
            {
                name.push('_');
            }
            name.extend(c.to_lowercase());
        }
        previous = Some(c);
    }
    let name = name.trim_end_matches('_').to_string();
    if name.is_empty() || name.starts_with(|c: char| c.is_numeric()) || UNRAW.contains(&name.as_str()) {
        return None;
    }
    Some(match KEYWORDS.contains(&name.as_str()) {
        true => format!("r#{name}"),
        false => name,
    })
}

/// Rust's keywords, strict and reserved, each a name only raw, `r#type`.
const KEYWORDS: [&str; 48] = [
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "do", "dyn", "else", "enum",
    "extern", "false", "final", "fn", "for", "gen", "if", "impl", "in", "let", "loop", "macro", "match", "mod", "move",
    "mut", "override", "priv", "pub", "ref", "return", "static", "struct", "trait", "true", "try", "type", "typeof",
    "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];
/// The keywords no raw name says.
const UNRAW: [&str; 4] = ["self", "super", "crate", "_"];

#[cfg(test)]
mod tests {
    use super::*;

    /// A class's name, a keyframes' and one inside `@media`, not one
    /// inside `:global(..)`.
    #[test]
    fn a_modules_keys_are_its_local_names() {
        let css = ".title { color: red } .code-block:hover { color: blue } :global(.app) .inner { color: green } \
                   @media (min-width: 1px) { .wide { color: red } } @keyframes spin { from { color: red } }";
        assert_eq!(
            exported_keys(css, "x.module.css").unwrap().keys,
            ["code-block", "inner", "spin", "title", "wide"]
        );
    }

    /// ICSS's `:export` and `:import`, and `@value`, are refused, never
    /// listed short (#1617).
    #[test]
    fn icss_and_values_are_refused() {
        for css in [
            ":export { primary: red } .a { color: red }",
            ":import(\"./other.css\") { imported: x } .a { color: red }",
            "@value primary: red; .a { color: primary }",
        ] {
            assert!(exported_keys(css, "x.module.css").is_err(), "{css}");
        }
    }

    /// Each class a getter, its Rust name snake_case, a keyword's raw;
    /// those no Rust name says, or two would share, left to a note.
    #[test]
    fn a_binding_names_each_class_once() {
        let keys: Vec<String> = ["codeBlock", "card-title", "type", "10", "self", "a-b", "aB"]
            .iter()
            .map(|key| key.to_string())
            .collect();
        let printed = binding("./site.module.css", &keys);
        assert!(printed.contains("rust_js::link_name = \"get codeBlock\")]\n    pub fn code_block(&self) -> String {"));
        assert!(
            printed.contains("rust_js::link_name = \"get card-title\")]\n    pub fn card_title(&self) -> String {")
        );
        assert!(printed.contains("rust_js::link_name = \"get type\")]\n    pub fn r#type(&self) -> String {"));
        assert!(printed.contains("// No getter is written of `10`, `a-b`, `aB`, `self`:"));
        assert!(
            printed
                .contains("#[link_name = \"./site.module.css#default\"]\n    pub safe static STYLES: &'static Styles;")
        );
    }
}
