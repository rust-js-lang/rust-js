//! [Next.js](https://nextjs.org) for rust-js (ADR 0192): what an app's
//! routes, `app/page.rs` beside `app/layout.js`, use of it: each of its
//! modules, `next/link` the crate's `next::link`, as Next.js types it.
//!
//! ```rust,ignore
//! use next::image::{Image, ImageProps};
//! use react::{JSX, jsx};
//!
//! pub fn Home() -> JSX::Element {
//!     jsx! { <Image src="/next.svg" alt="Next.js logo" width={Some(100)} {..Default::default()} /> }
//! }
//! js::export_default!(Home);
//! ```
//!
//! A component's optional props are `None` unless they're given, which
//! Next.js takes as its default: the rest from `{..Default::default()}`.

// A binding's parameters are its JS function's: its body never runs.
#![allow(non_snake_case, unused_variables)]

pub mod app;
pub mod cache;
mod data_fetching;
pub mod document;
pub mod dynamic;
pub mod error;
pub mod form;
pub mod head;
pub mod headers;
pub mod image;
mod instrumentation;
pub mod legacy;
pub mod link;
pub mod metadata;
pub mod navigation;
pub mod offline;
pub mod og;
mod pages;
pub mod router;
pub mod script;
mod segment;
pub mod server;
pub mod web_vitals;

pub use metadata::{Metadata, MetadataRoute, ResolvedMetadata, ResolvedViewport, ResolvingMetadata, ResolvingViewport, Viewport};

/// One value, or a list of them, `T | T[]`: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum OneOrMany<'a, T> {
    One(T),
    Many(&'a [T]),
}

pub use data_fetching::{
    GetServerSideProps, GetServerSidePropsContext, GetServerSidePropsResult, GetStaticPaths, GetStaticPathsContext,
    GetStaticPathsFallback, GetStaticPathsResult, GetStaticProps, GetStaticPropsContext, GetStaticPropsResult, PermanentRedirect,
    Redirect, Revalidate, RevalidateReason, ServerNotFound, ServerProps, ServerRedirect, StaticNotFound, StaticPath, StaticPathParams,
    StaticProps, StaticRedirect, StatusRedirect,
};
pub use instrumentation::{
    ErrorRequest, Instrumentation, RequestErrorContext, RouterTransitionEvent, RouterTransitionPrefetchIntent, RouterTransitionStartEvent,
    RouterTransitionType,
};
pub use segment::{Instant, InstantConfig, InstantSample, SampleCookie};
pub use pages::{
    ApiConfig, BodyParser, BodyParserLimit, ClearPreviewDataOptions, DraftModeOptions, FileSizeSuffix, NextApiHandler, NextApiRequest,
    NextApiResponse, NextComponentType, NextPage, NextPageContext, PageConfig, PreviewData, PreviewDataOptions, QueryValue, ResponseLimit,
    RevalidateOptions, Route, ServerRuntime, SizeLimit, set_get_initial_props,
};
