//! [`next/image`](https://nextjs.org/docs/app/api-reference/components/image):
//! an `<img>` Next.js sizes, optimizes and lazy-loads.

use react::JSX;

/// `<Image src="/next.svg" alt="Next.js logo" width={Some(100)} height={Some(20)} {..Default::default()} />`.
#[cfg_attr(rust_js, rust_js::link_name = "next/image#default")]
pub fn Image(props: ImageProps<'_>) -> JSX::Element {
    unreachable!()
}

/// What an [`Image`] is given: `src` and `alt`, and each of the rest
/// `None`, which Next.js takes as its default. Its text is borrowed, as
/// [`LinkProps`](crate::link::LinkProps)'s is.
#[derive(Default)]
pub struct ImageProps<'a> {
    /// A path, `/next.svg`, or an absolute URL its config allows.
    pub src: &'a str,
    /// Its text for who can't see it, `""` for one that's only decoration.
    pub alt: &'a str,
    /// Its width in pixels, needed unless it's `fill`.
    pub width: Option<u32>,
    /// Its height in pixels, needed unless it's `fill`.
    pub height: Option<u32>,
    /// Fill its parent, which is positioned, in place of a width and height.
    pub fill: Option<bool>,
    /// The widths it's shown at, as `sizes` media queries, for a `fill` one.
    pub sizes: Option<&'a str>,
    /// Its quality, 1 to 100: 75.
    pub quality: Option<u32>,
    /// Loaded first, as it's the page's largest: preloaded in the `<head>`.
    pub preload: Option<bool>,
    /// What `preload` was before Next.js 16, which it still takes.
    pub priority: Option<bool>,
    /// `"lazy"`, its default, or `"eager"`.
    pub loading: Option<&'a str>,
    /// `"empty"`, `"blur"`, or a `data:image/..` URL, shown while it loads.
    pub placeholder: Option<&'a str>,
    /// The `data:` URL a `"blur"` placeholder shows.
    #[cfg_attr(rust_js, rust_js::name = "blurDataURL")]
    pub blur_data_url: Option<&'a str>,
    /// Shown as it is, not optimized.
    pub unoptimized: Option<bool>,
    /// The `<img>`'s classes.
    #[cfg_attr(rust_js, rust_js::name = "className")]
    pub class_name: Option<&'a str>,
    /// The `<img>`'s `id`.
    pub id: Option<&'a str>,
    /// The `<img>`'s `title`, its tooltip.
    pub title: Option<&'a str>,
}
