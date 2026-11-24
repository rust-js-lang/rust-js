//! A library's fieldless enum that is its own iterator, and its generic
//! code that shows what it's given (#2545), and clones or copies it, never
//! called in the library, so its export takes no `Clone` dictionary or copy
//! function (ADR 0914, amended; #2540).

#[derive(Debug, Clone, Copy)]
pub enum Light {
    Red,
    Green,
}

impl Iterator for Light {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        let was = *self as u8;
        *self = match self {
            Light::Red => Light::Green,
            Light::Green => Light::Red,
        };
        Some(was)
    }
}

pub fn shown<T: std::fmt::Debug>(t: T) -> String {
    format!("{t:?}")
}

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
