//! A library's zip of a `Vec` of what's never `Copy` with a `Vec` of a type
//! parameter's items, as itertools' `MultiProduct::last` zips them: whether
//! the second side is `TrustedRandomAccessNoCoerce`, of a `Copy` item, is
//! its caller's to say where the consumer compiles it (#1389).
#[derive(Clone)]
pub struct Held<I> {
    pub iter: I,
}

pub fn lasts<I: Iterator + Clone>(iters: Vec<Held<I>>, values: Vec<I::Item>) -> Vec<I::Item> {
    iters
        .into_iter()
        .zip(values)
        .map(|(held, value)| held.iter.last().unwrap_or(value))
        .collect()
}
