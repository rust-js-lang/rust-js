//! [`next/legacy/image`](https://nextjs.org/docs/pages/api-reference/components/image-legacy):
//! the `<img>` Next.js sized by a `layout`, and fit to it by `objectFit`,
//! before `next/image`; deprecated, but still Next.js's.

use react::JSX;

/// `<Image src="/photo.jpg" layout={Some("fill")} objectFit={Some("cover")} alt={Some("..")} {..Default::default()} />`.
#[cfg_attr(rust_js, rust_js::link_name = "next/legacy/image#default")]
pub fn Image(props: ImageProps<'_>) -> JSX::Element {
    unreachable!()
}

/// What an [`Image`] is given: `src`, and each of the rest `None`, which
/// Next.js takes as its default. Its text is borrowed, as
/// [`next::image`](crate::image::ImageProps)'s is.
#[derive(Default)]
pub struct ImageProps<'a> {
    /// A path, `/photo.jpg`, or an absolute URL its config allows.
    pub src: &'a str,
    /// Its text for who can't see it, `""` for one that's only decoration.
    pub alt: Option<&'a str>,
    /// Its width in pixels, needed unless it's laid out to `"fill"`.
    pub width: Option<u32>,
    /// Its height in pixels, needed unless it's laid out to `"fill"`.
    pub height: Option<u32>,
    /// `"intrinsic"`, its default, `"fixed"`, `"responsive"`, or `"fill"`,
    /// its parent, which is positioned.
    pub layout: Option<&'a str>,
    /// How a `"fill"` one fits its parent, CSS's `object-fit`: `"cover"`.
    #[cfg_attr(rust_js, rust_js::name = "objectFit")]
    pub object_fit: Option<&'a str>,
    /// Where it's placed in its parent, CSS's `object-position`.
    #[cfg_attr(rust_js, rust_js::name = "objectPosition")]
    pub object_position: Option<&'a str>,
    /// The widths it's shown at, as `sizes` media queries.
    pub sizes: Option<&'a str>,
    /// Its quality, 1 to 100: 75.
    pub quality: Option<u32>,
    /// Loaded first, as it's the page's largest: preloaded in the `<head>`.
    pub priority: Option<bool>,
    /// `"lazy"`, its default, or `"eager"`.
    pub loading: Option<&'a str>,
    /// How far from the viewport it starts loading, a CSS margin: `"200px"`.
    #[cfg_attr(rust_js, rust_js::name = "lazyBoundary")]
    pub lazy_boundary: Option<&'a str>,
    /// `"empty"`, its default, or `"blur"`.
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
}
