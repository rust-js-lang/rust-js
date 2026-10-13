//! [`util`](https://nodejs.org/api/util.html): utilities.

use js::Promise;

/// What [`promisify`] makes of a function that takes its callback last,
/// `(err, value) => void` or `(err) => void`, as @types/node's overloads of
/// it type each: a function of the same arguments, borrowed or not, giving
/// a `Promise`, rejected with what it failed with.
pub trait Promisify {
    type Promised;
}

macro_rules! promisify {
    ($($a:ident),*) => {
        impl<$($a,)* E, T> Promisify for fn($($a,)* Box<dyn FnOnce(Option<E>, T)>) {
            type Promised = Box<dyn Fn($($a),*) -> Promise<T>>;
        }

        impl<$($a,)* E> Promisify for fn($($a,)* Box<dyn FnOnce(Option<E>)>) {
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

// One whose first argument is borrowed, a path, `readdir(&dir)`: a function
// of any borrow of it.
impl<P: ?Sized, E, T> Promisify for for<'a> fn(&'a P, Box<dyn FnOnce(Option<E>, T)>) {
    type Promised = Box<dyn for<'a> Fn(&'a P) -> Promise<T>>;
}

impl<P: ?Sized, E> Promisify for for<'a> fn(&'a P, Box<dyn FnOnce(Option<E>)>) {
    type Promised = Box<dyn for<'a> Fn(&'a P) -> Promise<()>>;
}

/// [`util.promisify(original)`](https://nodejs.org/api/util.html#utilpromisifyoriginal):
/// `original`, which takes its callback last, as a function that gives a
/// promise: `promisify(fs::readdir)(&dir).await`.
#[cfg_attr(rust_js, rust_js::link_name = "util#promisify")]
pub fn promisify<F: Promisify>(original: F) -> F::Promised {
    unreachable!()
}
