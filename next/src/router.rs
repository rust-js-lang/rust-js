//! [`next/router`](https://nextjs.org/docs/pages/api-reference/functions/use-router):
//! the Pages Router's route a component is on, and going to another, as
//! react.dev's pages use it. The App Router's is `next/navigation`'s.

use core::marker::PhantomData;

use js::JsObject;

#[cfg_attr(rust_js, rust_js::link_name = "next/router#useRouter")]
pub fn use_router() -> &'static NextRouter {
    unreachable!()
}

/// [`NextRouter`](https://nextjs.org/docs/pages/api-reference/functions/use-router#router-object).
pub struct NextRouter(PhantomData<JsObject>);

impl NextRouter {
    /// The path as the browser shows it, its query and hash too.
    #[cfg_attr(rust_js, rust_js::link_name = "get asPath")]
    pub fn as_path(&self) -> &'static str {
        unreachable!()
    }

    /// The page's route, `/blog/[slug]`.
    #[cfg_attr(rust_js, rust_js::link_name = "get pathname")]
    pub fn pathname(&self) -> &'static str {
        unreachable!()
    }

    /// Where the app is served from, under its host.
    #[cfg_attr(rust_js, rust_js::link_name = "get basePath")]
    pub fn base_path(&self) -> &'static str {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "push")]
    pub fn push(&self, href: &str) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "replace")]
    pub fn replace(&self, href: &str) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "back")]
    pub fn back(&self) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "reload")]
    pub fn reload(&self) {
        unreachable!()
    }
}
