//! [`next/error`](https://nextjs.org/docs/app/api-reference/functions/catchError):
//! Next.js's error page, and a boundary of a component's errors.

use core::marker::PhantomData;

use js::{JsObject, Unknown};
use react::{ComponentValue, JSX, ReactNode};

/// `<Error statusCode={404} />`: Next.js's error page, the Pages Router's.
#[cfg_attr(rust_js, rust_js::link_name = "next/error#default")]
pub fn Error(props: ErrorProps<'_>) -> JSX::Element {
    unreachable!()
}

/// What an [`Error`] is given.
#[derive(Default)]
pub struct ErrorProps<'a> {
    /// The HTTP status it shows, `404`.
    #[cfg_attr(rust_js, rust_js::name = "statusCode")]
    pub status_code: u16,
    pub hostname: Option<&'a str>,
    /// Its message, in place of the status's own.
    pub title: Option<&'a str>,
    /// Shown dark where the system is.
    #[cfg_attr(rust_js, rust_js::name = "withDarkMode")]
    pub with_dark_mode: Option<bool>,
}

/// [`catchError(fallback)`](https://nextjs.org/docs/app/api-reference/functions/catchError),
/// in a `thread_local!`: a component given `P`, which shows its children,
/// `P`'s `children`, or `fallback` of its props and the error where one is
/// thrown.
#[cfg_attr(rust_js, rust_js::link_name = "next/error#catchError")]
pub fn catch_error<P, R: ReactNode>(fallback: impl Fn(P, &'static ErrorInfo) -> R + 'static) -> ComponentValue<P> {
    unreachable!()
}

/// What a [`catch_error`] fallback is given beside its props.
pub struct ErrorInfo(PhantomData<JsObject>);

impl ErrorInfo {
    /// What was thrown.
    #[cfg_attr(rust_js, rust_js::link_name = "get error")]
    pub fn error(&self) -> Option<&'static Unknown> {
        unreachable!()
    }

    /// Show the children again, rendered on the client.
    #[cfg_attr(rust_js, rust_js::link_name = "reset")]
    pub fn reset(&self) {
        unreachable!()
    }

    /// Show the children again, their Server Components rendered again too.
    #[cfg_attr(rust_js, rust_js::link_name = "retry")]
    pub fn retry(&self) {
        unreachable!()
    }
}
