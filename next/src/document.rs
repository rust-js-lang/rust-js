//! [`next/document`](https://nextjs.org/docs/pages/building-your-application/routing/custom-document):
//! the Pages Router's document, the `<html>` and `<body>` every page is
//! rendered in, a `pages/_document`'s default export.

use core::marker::PhantomData;
use core::ops::Deref;

use js::{JsObject, Promise, Unknown};
use react::attributes::{HTMLAttributes, HtmlHTMLAttributes};
use react::webapi::HTMLHeadElement;
use react::{ComponentValue, JSX, ReactNode};

use crate::NextPageContext;

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
pub fn Head<C: ReactNode>(props: HeadProps<'_, C>) -> JSX::Element {
    unreachable!()
}

/// What a [`Head`] is given, as Next.js types it: an [`OriginProps`]', the
/// `<head>`'s attributes, and what it holds beside what Next.js puts in it.
#[derive(Default)]
pub struct HeadProps<'a, C> {
    pub nonce: Option<&'a str>,
    /// `"anonymous"`, `"use-credentials"` or `""`.
    #[cfg_attr(rust_js, rust_js::name = "crossOrigin")]
    pub cross_origin: Option<&'a str>,
    /// The `<head>`'s other attributes, as React's `HTMLAttributes`.
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub head: HTMLAttributes<'a, HTMLHeadElement>,
    pub children: C,
}

/// What [`NextScript`] is given, and [`Head`] too, as `OriginProps` types
/// it: the `nonce` and `crossOrigin` of the scripts and links Next.js puts
/// in.
#[derive(Default)]
pub struct OriginProps<'a, C> {
    pub nonce: Option<&'a str>,
    /// `"anonymous"`, `"use-credentials"` or `""`.
    #[cfg_attr(rust_js, rust_js::name = "crossOrigin")]
    pub cross_origin: Option<&'a str>,
    pub children: C,
}

/// `<Main />`: where the page is rendered.
#[cfg_attr(rust_js, rust_js::link_name = "next/document#Main")]
pub fn Main() -> JSX::Element {
    unreachable!()
}

/// `<NextScript />`: the scripts Next.js runs the page with.
#[cfg_attr(rust_js, rust_js::link_name = "next/document#NextScript")]
pub fn NextScript<C: ReactNode>(props: OriginProps<'_, C>) -> JSX::Element {
    unreachable!()
}

/// [`Document`](https://nextjs.org/docs/pages/building-your-application/routing/custom-document),
/// next/document's default export: Next.js's own document, its statics.
pub mod document {
    use super::*;

    /// `Document.getInitialProps(ctx)`: what Next.js's own document gives,
    /// the page rendered, for a custom one's to add to.
    #[cfg_attr(rust_js, rust_js::link_name = "next/document#default.getInitialProps")]
    pub fn get_initial_props(ctx: &DocumentContext) -> Promise<DocumentInitialProps> {
        unreachable!()
    }
}

/// What a document's `getInitialProps` is given, as `DocumentContext` types
/// it: a page's context, and rendering the page.
pub struct DocumentContext(PhantomData<JsObject>);

impl Deref for DocumentContext {
    type Target = NextPageContext;

    fn deref(&self) -> &NextPageContext {
        // Never runs: rust-js compiles this `Deref` to the object itself.
        unsafe { &*(self as *const Self as *const NextPageContext) }
    }
}

impl DocumentContext {
    /// Render the page, `await ctx.renderPage()`.
    #[cfg_attr(rust_js, rust_js::link_name = "renderPage")]
    pub fn render_page(&self) -> Promise<DocumentInitialProps> {
        unreachable!()
    }

    /// [`render_page`](Self::render_page), the app and the page made
    /// another of first, as a CSS-in-JS library collects their styles.
    #[cfg_attr(rust_js, rust_js::link_name = "renderPage")]
    pub fn render_page_with(&self, options: ComponentsEnhancer) -> Promise<DocumentInitialProps> {
        unreachable!()
    }

    /// What Next.js's own document gives of `ctx`.
    #[cfg_attr(rust_js, rust_js::link_name = "defaultGetInitialProps")]
    pub fn default_get_initial_props(&self, ctx: &DocumentContext) -> Promise<DocumentInitialProps> {
        unreachable!()
    }

    /// [`default_get_initial_props`](Self::default_get_initial_props), its
    /// scripts of `options`' nonce.
    #[cfg_attr(rust_js, rust_js::link_name = "defaultGetInitialProps")]
    pub fn default_get_initial_props_with_options(&self, ctx: &DocumentContext, options: NonceOptions<'_>) -> Promise<DocumentInitialProps> {
        unreachable!()
    }
}

/// [`DocumentContext::default_get_initial_props_with_options`]'s.
#[derive(Default)]
pub struct NonceOptions<'a> {
    pub nonce: Option<&'a str>,
}

/// What [`DocumentContext::render_page_with`] makes another of, the app and
/// the page, as `ComponentsEnhancer` types it.
#[derive(Default)]
pub struct ComponentsEnhancer {
    #[cfg_attr(rust_js, rust_js::name = "enhanceApp")]
    pub enhance_app: Option<Box<dyn Fn(&'static ComponentValue<Unknown>) -> &'static ComponentValue<Unknown>>>,
    #[cfg_attr(rust_js, rust_js::name = "enhanceComponent")]
    pub enhance_component: Option<Box<dyn Fn(&'static ComponentValue<Unknown>) -> &'static ComponentValue<Unknown>>>,
}

/// What a document's `getInitialProps` gives, as `DocumentInitialProps`
/// types it: the page's HTML, its head's elements, and its styles.
pub struct DocumentInitialProps {
    pub html: String,
    pub head: Option<Vec<Option<JSX::Element>>>,
    pub styles: Option<Vec<JSX::Element>>,
}

/// What a document is given, as `DocumentProps` types it: its
/// [`DocumentInitialProps`], and Next.js's own.
pub type DocumentProps = DocumentInitialProps;
