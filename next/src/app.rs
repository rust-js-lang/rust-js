//! [`next/app`](https://nextjs.org/docs/pages/building-your-application/routing/custom-app):
//! the Pages Router's app, `pages/_app`, which renders each page in it.

use js::Unknown;
use react::ElementType;

use crate::router::NextRouter;

/// [`AppProps`](https://nextjs.org/docs/pages/building-your-application/routing/custom-app):
/// what the app is given, the page's component and the props its data
/// fetching gave, `<Component {...pageProps} />`.
pub struct AppProps<P = &'static Unknown> {
    #[cfg_attr(rust_js, rust_js::name = "Component")]
    pub component: ElementType,
    #[cfg_attr(rust_js, rust_js::name = "pageProps")]
    pub page_props: P,
    pub router: &'static NextRouter,
}
