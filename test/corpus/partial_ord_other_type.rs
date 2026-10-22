// A type compared with another by its own `PartialOrd<u32>`: `<` and the
// rest are std's default, its `partial_cmp` against `Equal`, not an order
// of the type with itself; `==` its own `PartialEq<u32>`.
use std::cmp::Ordering;

struct Meters(u32);

impl PartialEq<u32> for Meters {
    fn eq(&self, other: &u32) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<u32> for Meters {
    fn partial_cmp(&self, other: &u32) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}

fn main() {
    let m = Meters(3);
    println!("{} {} {} {} {:?}", m < 4, m > 1, m <= 3, m >= 5, m.partial_cmp(&4));
    println!("{} {}", m == 3, m != 3);
}
