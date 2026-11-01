//! [`util`](https://nodejs.org/api/util.html): utilities.

use std::marker::PhantomData;

use js::Promise;

/// [`util.TextEncoder`](https://nodejs.org/api/util.html#class-utiltextencoder)
/// and [`util.TextDecoder`](https://nodejs.org/api/util.html#class-utiltextdecoder)
/// are the WHATWG classes, webapi's types, made by `util`'s own constructors,
/// imported from `util`: where a host has no global `TextEncoder`, a test
/// environment's, `util`'s is still there.
pub use webapi::{TextDecodeOptions, TextDecoder, TextDecoderOptions, TextEncoder};

pub mod text_encoder {
    use super::TextEncoder;

    /// [`new util.TextEncoder()`](https://nodejs.org/api/util.html#class-utiltextencoder):
    /// an encoder of text as UTF-8.
    #[cfg_attr(rust_js, rust_js::link_name = "new util#TextEncoder")]
    pub fn new() -> &'static TextEncoder {
        unreachable!()
    }
}

pub mod text_decoder {
    use super::{TextDecoder, TextDecoderOptions};

    /// [`new util.TextDecoder(encoding, options)`](https://nodejs.org/api/util.html#new-textdecoderencoding-options):
    /// a decoder of text in `encoding`, UTF-8 where it's `None`, as
    /// @types/node's `constructor(encoding?: string, options?: { fatal?:
    /// boolean; ignoreBOM?: boolean })` has it, webapi's
    /// `text_decoder::new`'s: React's Flight client's `new
    /// TextDecoder('utf-8', {ignoreBOM: preserveByteOrderMark})`.
    #[cfg_attr(rust_js, rust_js::link_name = "new util#TextDecoder")]
    pub fn new(encoding: Option<&str>, options: Option<TextDecoderOptions>) -> &'static TextDecoder {
        unreachable!()
    }
}

/// What [`promisify`] makes of a function that takes its callback last,
/// `(err, value) => void` or `(err) => void`, as @types/node's overloads of
/// it type each: a function of the same arguments, borrowed or not, giving
/// a `Promise`, rejected with what it failed with. `Args` are the arguments
/// before the callback, `(A1, A2)`, or [`Borrowed`]: each form is its own
/// trait, which rustc tells apart by its arguments alone.
pub trait Promisify<Args> {
    type Promised;
}

/// The arguments of a function whose first is borrowed, a path,
/// `readdir(&dir)`: [`Promisify`]'s, which no value is.
pub struct Borrowed<P: ?Sized>(PhantomData<fn(&P)>);

macro_rules! promisify {
    ($($a:ident),*) => {
        impl<$($a,)* E, T> Promisify<($($a,)*)> for fn($($a,)* Box<dyn FnOnce(Option<E>, T)>) {
            type Promised = Box<dyn Fn($($a),*) -> Promise<T>>;
        }

        impl<$($a,)* E> Promisify<($($a,)*)> for fn($($a,)* Box<dyn FnOnce(Option<E>)>) {
            type Promised = Box<dyn Fn($($a),*) -> Promise<()>>;
        }
    };
}

promisify!();
promisify!(A1);
promisify!(A1, A2);
promisify!(A1, A2, A3);
promisify!(A1, A2, A3, A4);
promisify!(A1, A2, A3, A4, A5);

// One whose first argument is borrowed: a function of any borrow of it.
impl<P: ?Sized, E, T> Promisify<Borrowed<P>> for for<'a> fn(&'a P, Box<dyn FnOnce(Option<E>, T)>) {
    type Promised = Box<dyn for<'a> Fn(&'a P) -> Promise<T>>;
}

impl<P: ?Sized, E> Promisify<Borrowed<P>> for for<'a> fn(&'a P, Box<dyn FnOnce(Option<E>)>) {
    type Promised = Box<dyn for<'a> Fn(&'a P) -> Promise<()>>;
}

/// [`util.promisify(original)`](https://nodejs.org/api/util.html#utilpromisifyoriginal):
/// `original`, which takes its callback last, as a function that gives a
/// promise: `promisify(fs::readdir)(&dir).await`.
#[cfg_attr(rust_js, rust_js::link_name = "util#promisify")]
pub fn promisify<Args, F: Promisify<Args>>(original: F) -> F::Promised {
    unreachable!()
}
