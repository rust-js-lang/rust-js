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

use js::{IntoUnknown, JsError, JsObject, Uint8Array, Unknown};

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

/// [`stream.Readable`](https://nodejs.org/api/stream.html#class-streamreadable):
/// what's read from, as its `read` pushes its chunks, React's prelude of a
/// prerender.
#[cfg_attr(rust_js, rust_js::types = "stream#Readable")]
pub struct Readable(PhantomData<JsObject>);

impl Readable {
    /// [`readable.push(chunk)`](https://nodejs.org/api/stream.html#readablepushchunk-encoding):
    /// `chunk` given to what reads it, any value, as @types/node's `any`, `None`
    /// `null`, its end; whether it would take more now.
    #[cfg_attr(rust_js, rust_js::link_name = "push")]
    #[cfg_attr(rust_js, rust_js::nullable(chunk))]
    pub fn push(&self, chunk: Option<&dyn IntoUnknown>) -> bool {
        unreachable!()
    }

    /// [`readable.destroy(error)`](https://nodejs.org/api/stream.html#readabledestroyerror):
    /// the stream closed at once, `"error"` emitted with `error` where there
    /// is one; the stream itself.
    #[cfg_attr(rust_js, rust_js::link_name = "destroy")]
    pub fn destroy(&self, error: Option<&dyn IntoUnknown>) -> &Readable {
        unreachable!()
    }
}

/// What [`readable::new`] is given, @types/node's `ReadableOptions`: its
/// `read`, called with how much is asked for when it may push more.
#[derive(Clone, Copy, Default)]
pub struct ReadableOptions<'a> {
    pub read: Option<&'a dyn Fn(f64)>,
    pub high_water_mark: Option<f64>,
}

pub mod readable {
    use super::{Readable, ReadableOptions};

    /// [`new stream.Readable(options)`](https://nodejs.org/api/stream.html#new-streamreadableoptions):
    /// a stream its `read` pushes to.
    #[cfg_attr(rust_js, rust_js::link_name = "new stream#Readable")]
    pub fn new(options: Option<&ReadableOptions<'_>>) -> &'static Readable {
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

impl EventEmitterExt for Readable {}

/// `"close"`: it and what it read from closed.
impl Emits<event::Close> for Readable {
    type Listener = dyn Fn();
    type Args = ();
}

/// `"data"`: a chunk, any value, as @types/node's `(chunk: any) => void`:
/// a `Buffer` or text of a stream of bytes, what was pushed of an object
/// stream's.
impl Emits<event::Data> for Readable {
    type Listener = dyn Fn(Option<&Unknown>);
    type Args = (Option<&'static Unknown>,);
}

/// `"end"`: all of it read.
impl Emits<event::End> for Readable {
    type Listener = dyn Fn();
    type Args = ();
}

/// `"error"`: what failed reading, the stream destroyed of it.
impl Emits<event::Error> for Readable {
    type Listener = dyn Fn(&JsError);
    type Args = (&'static JsError,);
}

/// `"pause"`: it stopped giving its chunks.
impl Emits<event::Pause> for Readable {
    type Listener = dyn Fn();
    type Args = ();
}

/// `"readable"`: a chunk to `read()`, or its end.
impl Emits<event::Readable> for Readable {
    type Listener = dyn Fn();
    type Args = ();
}

/// `"resume"`: it gives its chunks again.
impl Emits<event::Resume> for Readable {
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
