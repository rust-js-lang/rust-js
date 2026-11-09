// An iterator with a destructor that a std method takes by value, given a
// function that panics: the method drops it as the panic unwinds through
// it, as a `for` loop's end does, so its drop runs before the panic is
// caught, of `for_each`, `fold`, `reduce`, `max_by_key` and `min_by` alike
// (#1079).
use std::panic::catch_unwind;

struct Loud(u8);

impl Iterator for Loud {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        self.0 += 1;
        if self.0 < 5 { Some(self.0) } else { None }
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn checked(x: u8) -> u8 {
    if x == 3 {
        panic!("three");
    }
    x
}

fn print(x: u8) {
    println!("item {}", checked(x));
}

fn main() {
    let mut sum = 0;
    let each = catch_unwind(move || Loud(0).for_each(|x| sum += checked(x)));
    println!("for_each {}", each.is_err());
    let named = catch_unwind(|| Loud(1).for_each(print));
    println!("named {}", named.is_err());
    let folded = catch_unwind(|| Loud(0).fold(0u8, |a, x| a + checked(x)));
    println!("fold {}", folded.is_err());
    let reduced = catch_unwind(|| Loud(0).reduce(|a, x| a + checked(x)));
    println!("reduce {}", reduced.is_err());
    let max = catch_unwind(|| Loud(1).max_by_key(|&x| checked(x)));
    println!("max_by_key {}", max.is_err());
    let min = catch_unwind(|| Loud(2).min_by(|a, b| checked(*a).cmp(b)));
    println!("min_by {}", min.is_err());
    // One that runs to its end, dropped after it, and one whose function
    // never panics.
    let mut total = 0;
    Loud(3).for_each(|x| total += checked(x));
    println!("total {total}");
    let quiet = Loud(0).fold(0u32, |a, x| a + u32::from(x));
    println!("quiet {quiet}");
}
