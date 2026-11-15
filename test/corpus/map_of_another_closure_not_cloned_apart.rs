// A generic function's `Map<I, F>` cloned, cycled, of `F: FnMut(i32) ->
// i32`, beside another generic function's own `Map` of its own closure,
// `|held: U| Held { held }`, collected in place: no `U` makes that
// closure an `FnMut(i32) -> i32`, so it's never the `Map` that's cloned,
// and isn't made cloned apart, whose items, which drop, it couldn't be
// (#1895).
use std::fmt::Display;

pub fn cycled<I, F, L>(items: I, f: F, label: L) -> Vec<i32>
where
    I: Iterator<Item = i32> + Clone,
    F: FnMut(i32) -> i32 + Clone,
    L: Display,
{
    println!("{label}");
    items.map(f).cycle().take(5).collect()
}

struct D(u8);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Held<U> {
    held: U,
}

trait Collecting {
    // A trait's default, generic over its own `U` too.
    fn collected_with<U>(self, extra: Vec<U>) -> usize
    where
        Self: Sized,
    {
        drop(self);
        let all: Vec<Held<U>> = extra.into_iter().map(|held| Held { held }).collect();
        all.len()
    }
}

struct Paired<I, U>(I, U);

impl<I: Iterator, U> Collecting for Paired<I, U> {}

fn main() {
    println!("{:?}", cycled(1..3, |x| x * x, "squares"));
    println!("{}", Paired(vec![D(1)].into_iter(), 5u8).collected_with(vec![D(6), D(7)]));
    println!("{}", Paired(vec![2].into_iter(), D(3)).collected_with(vec![4u8]));
}
