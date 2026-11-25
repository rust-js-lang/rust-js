// One crate's own: each plain type JS gives and gets, a panic, a constant,
// a static, and a module of its own JS reads through its re-export.
pub mod shapes {
    pub fn area(w: u32, h: u32) -> u64 {
        w as u64 * h as u64
    }

    pub const SIDES: u8 = 4;
}

pub use shapes as geometry;

pub static GREETING: &str = "hi";

pub const LIMIT: i64 = -7;

pub fn add(a: u8, b: u8) -> u8 {
    a.wrapping_add(b)
}

pub fn halve(x: f64) -> f64 {
    x / 2.0
}

pub fn shout(text: &str) -> String {
    text.to_uppercase()
}

pub fn first(text: Option<&str>) -> Option<char> {
    text.and_then(|t| t.chars().next())
}

pub fn total(xs: &[u64]) -> u64 {
    xs.iter().fold(0u64, |sum, x| sum.wrapping_add(*x))
}

pub fn evens(n: u16) -> Vec<u16> {
    (0..n.min(6)).filter(|x| x % 2 == 0).collect()
}

pub fn pair(a: bool, b: i32) -> (bool, i32) {
    (!a, b.wrapping_neg())
}

pub fn checked(n: u32) -> u32 {
    if n > 1000 {
        panic!("too big: {n}");
    }
    n * 2
}
