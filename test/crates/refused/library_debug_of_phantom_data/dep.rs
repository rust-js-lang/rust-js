//! A library's derived `Debug` of a generic struct with a `PhantomData<T>`
//! field, whose `Debug` shows `PhantomData<{type_name::<T>()}>`: generic
//! code compiled in the library, whose callers give it no fact of `T`'s
//! name yet (#1657).
#[derive(Debug)]
pub struct P<T> {
    pub v: i32,
    pub m: std::marker::PhantomData<T>,
}
