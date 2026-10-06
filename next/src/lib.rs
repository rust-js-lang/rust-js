//! [Next.js](https://nextjs.org) for rust-js (ADR 0192): what an app's
//! routes, `app/page.rs` beside `app/layout.js`, use of it. Its components,
//! `next/image` and `next/link`, and its navigation, `next/navigation`.
//!
//! ```rust,ignore
//! use next::image::{Image, ImageProps};
//! use react::{Element, jsx};
//!
//! pub fn Home() -> Element {
//!     jsx! { <Image src="/next.svg" alt="Next.js logo" width={Some(100)} {..Default::default()} /> }
//! }
//! js::export_default!(Home);
//! ```
//!
//! A component's optional props are `None` unless they're given, which
//! Next.js takes as its default: the rest from `{..Default::default()}`.

// A binding's parameters are its JS function's: its body never runs.
#![allow(non_snake_case, unused_variables)]

pub mod image;
pub mod link;
pub mod navigation;
pub mod router;
