//! [`next/font`](https://nextjs.org/docs/app/api-reference/components/font):
//! a font Next.js loads at build time and serves with the app, a Google
//! Font's or a file's, called in a `thread_local!` at the module's top, as
//! Next.js's compiler needs it: its class, its style and its CSS variable.

pub mod google;
pub mod local;

/// A font loaded, as `NextFont` and `NextFontWithVariable` type it.
pub struct NextFont {
    /// The class that sets the font, `className={inter.className}`.
    #[cfg_attr(rust_js, rust_js::name = "className")]
    pub class_name: String,
    pub style: NextFontStyle,
    /// The class that sets its CSS variable, of a font given one.
    pub variable: Option<String>,
}

/// A font's style, as an element's `style` sets it.
pub struct NextFontStyle {
    #[cfg_attr(rust_js, rust_js::name = "fontFamily")]
    pub font_family: String,
    #[cfg_attr(rust_js, rust_js::name = "fontWeight")]
    pub font_weight: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "fontStyle")]
    pub font_style: Option<String>,
}

/// How a font shows while it loads, CSS's `font-display`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Display {
    #[cfg_attr(rust_js, rust_js::name = "auto")]
    Auto,
    #[cfg_attr(rust_js, rust_js::name = "block")]
    Block,
    #[cfg_attr(rust_js, rust_js::name = "swap")]
    Swap,
    #[cfg_attr(rust_js, rust_js::name = "fallback")]
    Fallback,
    #[cfg_attr(rust_js, rust_js::name = "optional")]
    Optional,
}
