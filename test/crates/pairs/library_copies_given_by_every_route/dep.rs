//! A library's generic functions whose `Copy` and `Clone` bounds its own
//! calls never give an object, which another crate does, by every route.
pub fn dup<T: Copy>(t: T) -> (T, T) {
    (t, t)
}

pub fn cloned<T: Clone>(t: &T) -> (T, T) {
    (t.clone(), t.clone())
}
