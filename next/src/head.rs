//! [`next/head`](https://nextjs.org/docs/pages/api-reference/components/head):
//! what a page puts in the document's `<head>`, in the Pages Router.

use react::{JSX, ReactNode};

/// `<Head><link rel="preconnect" href=".." /></Head>`.
#[cfg_attr(rust_js, rust_js::link_name = "next/head#default")]
pub fn Head<C: ReactNode>(props: HeadProps<C>) -> JSX::Element {
    unreachable!()
}

/// What a [`Head`] holds: its elements, which go in the `<head>`.
pub struct HeadProps<C> {
    pub children: C,
}
