//! [`next/og`](https://nextjs.org/docs/app/api-reference/functions/image-response):
//! an image made of JSX and CSS, a page's Open Graph image.

use core::marker::PhantomData;
use core::ops::Deref;

use js::{ArrayBuffer, JsObject};
use react::JSX;
use react::webapi::Response;

/// [`ImageResponse`](https://nextjs.org/docs/app/api-reference/functions/image-response):
/// a `Response` of a PNG, drawn from its element.
pub struct ImageResponse(PhantomData<JsObject>);

impl Deref for ImageResponse {
    type Target = Response;

    fn deref(&self) -> &Response {
        // Never runs: rust-js compiles this `Deref` to the object itself.
        unsafe { &*(self as *const Self as *const Response) }
    }
}

/// [`ImageResponse`]'s constructors.
pub mod image_response {
    use super::*;

    /// `new ImageResponse(element)`: 1200 by 630 pixels.
    #[cfg_attr(rust_js, rust_js::link_name = "new next/og#ImageResponse")]
    pub fn new(element: JSX::Element) -> &'static ImageResponse {
        unreachable!()
    }

    /// `new ImageResponse(element, options)`.
    #[cfg_attr(rust_js, rust_js::link_name = "new next/og#ImageResponse")]
    pub fn new_with_options(element: JSX::Element, options: ImageResponseOptions<'_>) -> &'static ImageResponse {
        unreachable!()
    }

    unsafe extern "Rust" {
        /// `ImageResponse.displayName`: its name in React's tools.
        #[link_name = "next/og#ImageResponse.displayName"]
        pub safe static DISPLAY_NAME: &'static str;
    }
}

/// How an [`ImageResponse`] is drawn, and its response's status and
/// headers, as `ImageResponseOptions` types them.
#[derive(Default)]
pub struct ImageResponseOptions<'a> {
    /// Its width in pixels: 1200.
    pub width: Option<u32>,
    /// Its height in pixels: 630.
    pub height: Option<u32>,
    /// Draw the boxes' outlines.
    pub debug: Option<bool>,
    /// The fonts it's drawn in: Noto Sans.
    pub fonts: Option<&'a [ImageFont<'a>]>,
    /// The emoji's style: `"twemoji"`, its default, `"blobmoji"`, `"noto"`,
    /// `"openmoji"`, `"fluent"` or `"fluentFlat"`.
    pub emoji: Option<&'a str>,
    pub status: Option<u16>,
    #[cfg_attr(rust_js, rust_js::name = "statusText")]
    pub status_text: Option<&'a str>,
    pub headers: Option<&'a [&'a [&'a str]]>,
}

/// A font an [`ImageResponse`] is drawn in: its file's data, and the
/// `font-family`, `font-weight` and `font-style` it's for.
pub struct ImageFont<'a> {
    pub data: &'a ArrayBuffer,
    pub name: &'a str,
    /// 100 to 900.
    pub weight: Option<u16>,
    /// `"normal"` or `"italic"`.
    pub style: Option<&'a str>,
}
