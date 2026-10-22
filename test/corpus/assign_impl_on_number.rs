// The crate's own `*Assign` of a number or a `String`, `impl
// AddAssign<Cents> for i32`, is its impl's, not the number's operator.
use std::ops::{AddAssign, ShlAssign, SubAssign};

#[derive(Clone, Copy)]
struct Cents(i32);

impl AddAssign<Cents> for i32 {
    fn add_assign(&mut self, c: Cents) {
        *self += c.0 * 100;
    }
}

impl SubAssign<Cents> for i64 {
    fn sub_assign(&mut self, c: Cents) {
        *self -= i64::from(c.0) * 100;
    }
}

impl ShlAssign<Cents> for u32 {
    fn shl_assign(&mut self, c: Cents) {
        *self <<= c.0 as u32 + 1;
    }
}

struct Tag(&'static str);

impl AddAssign<Tag> for String {
    fn add_assign(&mut self, t: Tag) {
        self.push('<');
        self.push_str(t.0);
        self.push('>');
    }
}

fn main() {
    let mut total = 0i32;
    total += Cents(5);
    total += 1;
    let mut big = 1_000i64;
    big -= Cents(3);
    let mut bits = 1u32;
    bits <<= Cents(2);
    let mut s = String::from("x");
    s += Tag("b");
    s += "!";
    println!("{total} {big} {bits} {s}");
}
