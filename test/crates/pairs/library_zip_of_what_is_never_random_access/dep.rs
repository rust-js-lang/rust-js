//! A library's zip of a `Vec` of what's never `Copy`, so never
//! `TrustedRandomAccessNoCoerce` whatever its parameter is, with a slice's
//! iterator, which is for any item, as itertools' `MultiProduct::last` zips
//! them: the zip's facts are known here, none a caller's to give (#1385).
#[derive(Clone)]
pub struct Held<I> {
    pub iter: I,
}

pub fn lasts<I: Iterator + Clone>(iters: Vec<Held<I>>, values: &[I::Item]) -> Vec<I::Item>
where
    I::Item: Clone,
{
    iters
        .into_iter()
        .zip(values.iter())
        .map(|(held, value)| held.iter.last().unwrap_or_else(|| value.clone()))
        .collect()
}
