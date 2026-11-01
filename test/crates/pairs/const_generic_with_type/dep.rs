//! Const generics over an array's length beside a type parameter: the
//! library's own calls, and its consumer's, of values that drop and that
//! copy.
use std::fmt::Display;

pub fn joined<T: Display, const N: usize>(items: [T; N], sep: &str) -> String {
    let mut out = String::new();
    for (i, item) in items.into_iter().enumerate() {
        if i > 0 {
            out.push_str(sep);
        }
        out.push_str(&item.to_string());
    }
    format!("{N}:{out}")
}

pub fn rotated<T: Clone, const N: usize>(items: &[T; N]) -> [T; N] {
    let mut out = items.clone();
    out.rotate_left(1);
    out
}

pub struct Ring<T, const N: usize> {
    pub items: [T; N],
}

impl<T: Clone + Display, const N: usize> Ring<T, N> {
    pub fn turned(&self) -> Ring<T, N> {
        Ring {
            items: rotated(&self.items),
        }
    }

    pub fn shown(&self) -> String {
        joined(self.items.clone(), ",")
    }
}

pub fn report() -> String {
    let ring = Ring { items: [1u8, 2, 3] };
    format!(
        "{} {} {}",
        joined(["a", "b"], "-"),
        ring.turned().shown(),
        joined(rotated(&[1.5f64, 2.5]), "/")
    )
}
