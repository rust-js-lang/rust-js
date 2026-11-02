//! A library's private tuple struct whose constructor its generic code
//! gives as a function, `.map(Self)`, as smallvec's `ConstNonNull::new`
//! does (#476), and a public one.
struct Held<T>(T);

impl<T> Held<T> {
    fn new(value: Option<T>) -> Option<Self> {
        value.map(Self)
    }
}

#[derive(Debug)]
pub struct Wrap(pub u32);

/// Keeps `value` where it's given one; a consumer giving a value with a
/// destructor compiles this, and `Held::new`, itself.
pub fn keep<T>(value: Option<T>, other: T) -> Option<T> {
    drop(other);
    Held::new(value).map(|held| held.0)
}
