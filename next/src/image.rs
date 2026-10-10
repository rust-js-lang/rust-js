//! [`next/image`](https://nextjs.org/docs/app/api-reference/components/image):
//! an `<img>` Next.js sizes, optimizes and lazy-loads.

use react::JSX;
use react::attributes::ImgHTMLAttributes;
use react::webapi::HTMLImageElement;

/// `<Image src="/next.svg" alt="Next.js logo" width={Some(100)} height={Some(20)} {..Default::default()} />`.
#[cfg_attr(rust_js, rust_js::link_name = "next/image#default")]
pub fn Image<S: IntoImageSrc>(props: ImageProps<'_, S>) -> JSX::Element {
    unreachable!()
}

/// The props of an `<img>` of `props`, as [`Image`] renders it, its
/// `srcSet` and `sizes` Next.js's, as `getImageProps(props)`: for a
/// `<picture>`'s `<source>`s, or a CSS `image-set()`.
#[cfg_attr(rust_js, rust_js::link_name = "next/image#getImageProps")]
pub fn get_image_props<S: IntoImageSrc>(img_props: ImageProps<'_, S>) -> ImageResult {
    unreachable!()
}

/// What [`get_image_props`] gives: an `<img>`'s props, `<img {...props} />`.
pub struct ImageResult {
    pub props: ImgHTMLAttributes<'static>,
}

/// What an [`Image`] is given, as Next.js types it: `src` and `alt`, and an
/// `<img>`'s attributes, each `None` but what's given, which Next.js takes
/// as its default. Its text is borrowed, so one a component computes is
/// given too.
#[derive(Default)]
pub struct ImageProps<'a, S = &'a str> {
    /// A path, `/next.svg`, an absolute URL its config allows, or a
    /// [`StaticImageData`], an image a module imports.
    pub src: S,
    /// Its text for who can't see it, `""` for one that's only decoration.
    pub alt: &'a str,
    /// Its width in pixels, needed unless it's `fill` or imported.
    pub width: Option<u32>,
    /// Its height in pixels, needed unless it's `fill` or imported.
    pub height: Option<u32>,
    /// Fill its parent, which is positioned, in place of a width and height.
    pub fill: Option<bool>,
    /// What makes its URL of a width, in place of Next.js's optimizer.
    pub loader: Option<ImageLoader>,
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
    /// The `src` its `<img>` has, where it's another than `src`'s.
    #[cfg_attr(rust_js, rust_js::name = "overrideSrc")]
    pub override_src: Option<&'a str>,
    /// Called with the `<img>` once it's loaded: deprecated, `onLoad` is.
    #[cfg_attr(rust_js, rust_js::name = "onLoadingComplete")]
    pub on_loading_complete: Option<OnLoadingComplete>,
    /// `next/legacy/image`'s, which Next.js maps to `style`: deprecated.
    pub layout: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "objectFit")]
    pub object_fit: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "objectPosition")]
    pub object_position: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "lazyBoundary")]
    pub lazy_boundary: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "lazyRoot")]
    pub lazy_root: Option<&'a str>,
    /// The widths it's shown at, as `sizes` media queries, for a `fill` one.
    pub sizes: Option<&'a str>,
    /// The `<img>`'s classes.
    #[cfg_attr(rust_js, rust_js::name = "className")]
    pub class_name: Option<&'a str>,
    /// The `<img>`'s `id`.
    pub id: Option<&'a str>,
    /// The `<img>`'s `title`, its tooltip.
    pub title: Option<&'a str>,
    /// The `<img>`'s other attributes, as React's `ImgHTMLAttributes`:
    /// `style`, `onLoad`, `decoding`. Those named here are these, which a
    /// `{..Default::default()}` gives too.
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub img: ImgHTMLAttributes<'a>,
}

/// An image a module imports, `import logo from "./logo.png"`, as Next.js's
/// bundler gives it: its URL and size.
pub struct StaticImageData {
    pub src: &'static str,
    pub height: f64,
    pub width: f64,
    #[cfg_attr(rust_js, rust_js::name = "blurDataURL")]
    pub blur_data_url: Option<&'static str>,
    #[cfg_attr(rust_js, rust_js::name = "blurWidth")]
    pub blur_width: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "blurHeight")]
    pub blur_height: Option<f64>,
}

/// What an `ImageProps`'s `src`, `string | StaticImport`, takes: each as it
/// is (ADR 0229).
#[diagnostic::on_unimplemented(message = "`{Self}` is not an image's `src`, a `&str` nor a `&StaticImageData`")]
#[cfg_attr(rust_js, rust_js::types = "string | import(\"next/image\").StaticImageData")]
pub trait IntoImageSrc: sealed::Sealed {}
impl IntoImageSrc for &str {}
impl IntoImageSrc for &String {}
impl IntoImageSrc for &StaticImageData {}

mod sealed {
    pub trait Sealed {}
    impl Sealed for &str {}
    impl Sealed for &String {}
    impl Sealed for &super::StaticImageData {}
}

/// What makes an image's URL of a width and quality, in place of
/// Next.js's optimizer, as `ImageLoader` types it.
pub type ImageLoader = Box<dyn Fn(ImageLoaderProps) -> String>;

/// What an [`ImageLoader`] is given.
pub struct ImageLoaderProps {
    pub src: String,
    pub width: f64,
    pub quality: Option<f64>,
}

/// What an [`ImageProps`]'s `on_loading_complete` is.
pub type OnLoadingComplete = Box<dyn Fn(&HTMLImageElement)>;
