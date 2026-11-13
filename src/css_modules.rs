//! The class names a CSS Module exports, the one thing rust-js reads of a
//! stylesheet (#1512): its text in, its export keys out, sorted, or why it
//! can't say them. Lightning CSS lists them, with its CSS Modules config
//! and nothing else; this function is all of rust-js that knows of it, so
//! another parser can stand behind it.

use lightningcss::css_modules::Config;
use lightningcss::error::{ErrorLocation, ParserError};
use lightningcss::rules::CssRule;
use lightningcss::selector::{Component, PseudoClass};
use lightningcss::stylesheet::{ParserOptions, PrinterOptions, StyleSheet};

/// The keys a CSS Module of `source`, the file `filename`, exports, as its
/// bundler names them as they are (`asIs`), sorted; or why they can't be
/// said: a file Lightning CSS doesn't read, or one of ICSS's `:export` or
/// `:import`, whose keys it doesn't list where css-loader does (#1617).
pub fn exported_keys(source: &str, filename: &str) -> Result<Vec<String>, String> {
    let options = ParserOptions {
        filename: filename.to_string(),
        css_modules: Some(Config::default()),
        ..ParserOptions::default()
    };
    let sheet = StyleSheet::parse(source, options).map_err(|error| match error.kind {
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
    let printed = sheet
        .to_css(PrinterOptions::default())
        .map_err(|error| error.to_string())?;
    let mut keys: Vec<String> = printed.exports.unwrap_or_default().into_keys().collect();
    keys.sort();
    Ok(keys)
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
            exported_keys(css, "x.module.css").unwrap(),
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
}
