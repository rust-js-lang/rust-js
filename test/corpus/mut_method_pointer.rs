// A trait's method that takes `&mut self`, as a function pointer: the arrow
// a call lowered as MIR's makes, its box given and copied back in it once
// the call returns (ADR 0099), a default's and an impl's alike, of a value
// changed in place and of a number, which its box is all of.
trait Counter {
    fn get(&self) -> i32;
    fn set(&mut self, n: i32);
    fn bump(&mut self, by: i32) -> i32 {
        let next = self.get() + by;
        self.set(next);
        next
    }
    fn replace_with(&mut self, other: Self)
    where
        Self: Sized,
    {
        *self = other;
    }
}

struct Tally(i32);

impl Counter for Tally {
    fn get(&self) -> i32 {
        self.0
    }
    fn set(&mut self, n: i32) {
        self.0 = n;
    }
}

impl Counter for i32 {
    fn get(&self) -> i32 {
        *self
    }
    fn set(&mut self, n: i32) {
        *self = n;
    }
}

fn main() {
    let bump: fn(&mut Tally, i32) -> i32 = Tally::bump;
    let set: fn(&mut Tally, i32) = Tally::set;
    let mut tally = Tally(100);
    println!("{}", bump(&mut tally, 1));
    set(&mut tally, 5);
    println!("{} {}", bump(&mut tally, 2), tally.0);
    let replace: fn(&mut Tally, Tally) = Tally::replace_with;
    replace(&mut tally, Tally(9));
    println!("{}", tally.0);
    let bump_number: fn(&mut i32, i32) -> i32 = i32::bump;
    let mut n = 40;
    println!("{} {}", bump_number(&mut n, 2), n);
}
