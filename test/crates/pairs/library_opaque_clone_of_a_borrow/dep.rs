//! A library's `impl Iterator + Clone + '_`, which holds the borrow its
//! function is given, a consumer may clone (#1201): what it hides is read
//! with the function's lifetimes erased, which crashed the compiler left
//! bound.
pub fn words(s: &str) -> impl Iterator<Item = &str> + Clone + '_ {
    s.split(' ')
}

pub fn pairs<'a>(v: &'a [u32]) -> impl Iterator<Item = &'a [u32]> + Clone + 'a {
    v.windows(2)
}
