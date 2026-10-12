// A trait's default copied into each impl (ADR 0176) lending its `self`
// to the impl's method: an object `Self` changed in place, a number's
// through a box (ADR 0099), as Rust changes each.
trait Ch: Sized {
    fn change(mut self) -> Self {
        self.set(5);
        self
    }
    fn set(&mut self, a: i32);
}

struct X {
    a: i32,
}

impl Ch for X {
    fn set(&mut self, a: i32) {
        self.a = a;
    }
}

impl Ch for i32 {
    fn set(&mut self, a: i32) {
        *self += a;
    }
}

fn main() {
    println!("{} {}", X { a: 1 }.change().a, 1.change());
}
