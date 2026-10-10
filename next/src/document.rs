//! [`next/document`](https://nextjs.org/docs/pages/building-your-application/routing/custom-document):
//! the Pages Router's document, the `<html>` and `<body>` every page is
//! rendered in, a `pages/_document`'s default export.

use react::attributes::HtmlHTMLAttributes;
use react::{JSX, ReactNode};

/// `<Html lang="en">`: the document's `<html>`, of its attributes.
#[cfg_attr(rust_js, rust_js::link_name = "next/document#Html")]
pub fn Html<C: ReactNode>(props: HtmlProps<'_, C>) -> JSX::Element {
    unreachable!()
}

/// What an [`Html`] is given: its `<html>`'s attributes, and what's in it.
#[derive(Default)]
pub struct HtmlProps<'a, C> {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub props: HtmlHTMLAttributes<'a>,
    pub children: C,
}

/// `<Head />`: the document's `<head>`, what Next.js puts in it, and its
/// children.
#[cfg_attr(rust_js, rust_js::link_name = "next/document#Head")]
pub fn Head<C: ReactNode>(props: HeadProps<C>) -> JSX::Element {
    unreachable!()
}

/// What a [`Head`] holds, beside what Next.js puts in it.
#[derive(Default)]
pub struct HeadProps<C> {
    pub children: C,
}

/// `<Main />`: where the page is rendered.
#[cfg_attr(rust_js, rust_js::link_name = "next/document#Main")]
pub fn Main() -> JSX::Element {
    unreachable!()
}

/// `<NextScript />`: the scripts Next.js runs the page with.
#[cfg_attr(rust_js, rust_js::link_name = "next/document#NextScript")]
pub fn NextScript() -> JSX::Element {
    unreachable!()
}
