//! A library's generic functions that clone or copy what they're given,
//! never called in the library, so its own analysis gives their `Clone`
//! and `Copy` bounds no dictionary, each clone the value itself (ADR 0914,
//! amended); and ones that take an `IntoIterator` another bound reads, and
//! step an iterator's items through their own bound (#2540, #2541).

pub fn cloned<T: Iterator + Clone>(t: T) -> Vec<Option<T::Item>> {
    let mut a = t.clone();
    let mut b = t;
    vec![a.next(), b.next(), a.next()]
}

pub fn copied<T: Iterator + Copy>(t: T) -> Vec<Option<T::Item>> {
    let mut a = t;
    let mut b = t;
    vec![a.next(), b.next(), a.next()]
}

pub fn firsts<T: IntoIterator + Clone>(t: T) -> Vec<Option<T::Item>> {
    let mut a = t.clone().into_iter();
    let mut b = t.into_iter();
    vec![a.next(), b.next(), a.next()]
}

pub fn heads<S>(s: S) -> Vec<Option<<S::Item as Iterator>::Item>>
where
    S: Iterator,
    S::Item: Iterator,
{
    s.map(|mut i| i.next()).collect()
}
