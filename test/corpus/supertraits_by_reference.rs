// A trait whose supertraits are one trait twice, differing only by a
// reference, as num-traits' `RefNum<Base>: NumOps<Base, Base> + for<'r>
// NumOps<&'r Base, Base>` is: each its own part of the dictionary, named
// apart, `AddBase` and `AddRefBase`.

use std::ops::Add;

trait RefAdd<Base>: Add<Base, Output = Base> + for<'r> Add<&'r Base, Output = Base> {}

impl<T, Base> RefAdd<Base> for T where T: Add<Base, Output = Base> + for<'r> Add<&'r Base, Output = Base> {}

fn twice_and_once<T: RefAdd<T> + Copy>(a: T, b: T) -> T {
    let by_value = a + b;
    by_value + &b
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Meters(i32);

impl Add for Meters {
    type Output = Meters;
    fn add(self, other: Meters) -> Meters {
        Meters(self.0 + other.0)
    }
}

impl<'r> Add<&'r Meters> for Meters {
    type Output = Meters;
    fn add(self, other: &'r Meters) -> Meters {
        Meters(self.0 + other.0 * 10)
    }
}

fn main() {
    println!("{} {}", twice_and_once(1, 2), twice_and_once(0.5, 0.25));
    println!("{:?}", twice_and_once(Meters(1), Meters(2)));
}
