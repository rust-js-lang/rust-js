// A map made knowing its back, `$mapEnds`, held apart where no `let` makes
// it: a parameter's, a field's, a variable stepped from its back first.
// The held flip leaves each chain on one of them the hand-written form,
// as the value it's given is a JS iterator, not std's struct (#1681's
// step iii).
use std::iter::Map;
use std::ops::Range;

fn doubled(x: u32) -> u32 {
    x * 2
}

fn weighted(shifted: Map<Range<u32>, fn(u32) -> u32>) -> u32 {
    shifted.enumerate().rfold(0, |a, (i, x)| a + i as u32 * x)
}

struct Held {
    it: Map<Range<u32>, fn(u32) -> u32>,
}

fn main() {
    println!("{}", weighted((1..6).map(doubled as fn(u32) -> u32)));
    let held = Held {
        it: (0..4).map(doubled as fn(u32) -> u32),
    };
    println!("{:?}", held.it.enumerate().next_back());
    let mut it = (2..7u32).map(doubled as fn(u32) -> u32);
    it.next_back();
    println!("{:?}", it.enumerate().rev().take(2).collect::<Vec<_>>());
}
