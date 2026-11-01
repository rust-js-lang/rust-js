//! [`stream`](https://nodejs.org/api/stream.html): a [`Writable`], what
//! data is written to, as React's server renderers write their chunks.
//!
//! ```rust,ignore
//! use node::stream::Writable;
//!
//! fn send(destination: &Writable, text: &str) -> bool {
//!     destination.write(text)
//! }
//! ```

use core::marker::PhantomData;

use js::{JsError, JsObject, Uint8Array};

use crate::buffer::Buffer;
use crate::event;
use crate::events::{Emits, EventEmitterExt};

/// [`stream.Writable`](https://nodejs.org/api/stream.html#class-streamwritable):
/// what's written to, a file's, a socket's or a response's.
#[cfg_attr(rust_js, rust_js::types = "stream#Writable")]
pub struct Writable(PhantomData<JsObject>);

impl Writable {
    /// [`writable.write(chunk)`](https://nodejs.org/api/stream.html#writablewritechunk-encoding-callback):
    /// `chunk` written, text as UTF-8; whether the stream would take more
    /// now, `false` where it's buffered past its high water mark, until
    /// `"drain"`.
    #[cfg_attr(rust_js, rust_js::link_name = "write")]
    pub fn write(&self, chunk: impl IntoChunk) -> bool {
        unreachable!()
    }

    /// [`writable.end()`](https://nodejs.org/api/stream.html#writableendchunk-encoding-callback):
    /// no more written; the stream itself.
    #[cfg_attr(rust_js, rust_js::link_name = "end")]
    pub fn end(&self) -> &Writable {
        unreachable!()
    }

    /// [`writable.destroy(error)`](https://nodejs.org/api/stream.html#writabledestroyerror):
    /// the stream closed at once, `"error"` emitted with `error` where
    /// there is one; the stream itself.
    #[cfg_attr(rust_js, rust_js::link_name = "destroy")]
    pub fn destroy(&self, error: Option<&JsError>) -> &Writable {
        unreachable!()
    }
}

impl EventEmitterExt for Writable {}

/// `"drain"`: it takes more, after a `write` gave `false`.
impl Emits<event::Drain> for Writable {
    type Listener = dyn Fn();
    type Args = ();
}

/// `"error"`: what failed writing, the stream destroyed of it.
impl Emits<event::Error> for Writable {
    type Listener = dyn Fn(&JsError);
    type Args = (&'static JsError,);
}

/// `"close"`: it and what it wrote to closed.
impl Emits<event::Close> for Writable {
    type Listener = dyn Fn();
    type Args = ();
}

/// What a stream's chunk, `string | Uint8Array`, takes: each as it is (ADR
/// 0229), a [`Buffer`] among them.
#[diagnostic::on_unimplemented(message = "`{Self}` is not a text nor bytes")]
#[cfg_attr(rust_js, rust_js::types = "string | Uint8Array")]
pub trait IntoChunk: sealed::Sealed {}
impl IntoChunk for &str {}
impl IntoChunk for &Uint8Array {}
impl IntoChunk for &Buffer {}

mod sealed {
    pub trait Sealed {}
    impl Sealed for &str {}
    impl Sealed for &js::Uint8Array {}
    impl Sealed for &super::Buffer {}
}
