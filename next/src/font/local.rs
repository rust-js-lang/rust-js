//! [`next/font/local`](https://nextjs.org/docs/app/api-reference/components/font#local-fonts):
//! a font of the app's own files.

use super::{Display, NextFont};

/// `localFont(options)`, in a `thread_local!` at the module's top: the font
/// of `options.src`'s files.
#[cfg_attr(rust_js, rust_js::link_name = "next/font/local#default")]
pub fn localFont(options: LocalFont<'_>) -> NextFont {
    unreachable!()
}

/// A local font's options, as `LocalFont` types them: its files, required,
/// among them, so it has no `Default`.
pub struct LocalFont<'a> {
    pub src: LocalFontSrc<'a>,
    pub display: Option<Display>,
    pub weight: Option<&'a str>,
    pub style: Option<&'a str>,
    /// The font its fallback's metrics are adjusted to, or `false`, none.
    #[cfg_attr(rust_js, rust_js::name = "adjustFontFallback")]
    pub adjust_font_fallback: Option<AdjustFontFallback>,
    pub fallback: Option<&'a [&'a str]>,
    pub preload: Option<bool>,
    /// Its CSS variable, `"--font-mono"`, which [`NextFont::variable`] names.
    pub variable: Option<&'a str>,
    /// The `@font-face`'s other declarations.
    pub declarations: Option<&'a [FontDeclaration<'a>]>,
}

/// Its file, a path relative to the module, or its files, each of a weight
/// and a style: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum LocalFontSrc<'a> {
    Path(&'a str),
    Files(&'a [LocalFontFile<'a>]),
}

/// A file of the font, its path required.
pub struct LocalFontFile<'a> {
    pub path: &'a str,
    pub weight: Option<&'a str>,
    pub style: Option<&'a str>,
}

/// `"Arial" | "Times New Roman" | false`: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum AdjustFontFallback {
    Bool(bool),
    #[cfg_attr(rust_js, rust_js::name = "Arial")]
    Arial,
    #[cfg_attr(rust_js, rust_js::name = "Times New Roman")]
    TimesNewRoman,
}

/// An `@font-face` declaration, `font-feature-settings: "liga" 0`.
pub struct FontDeclaration<'a> {
    pub prop: &'a str,
    pub value: &'a str,
}
