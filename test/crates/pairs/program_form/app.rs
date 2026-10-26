//! A crate giving a library's generic functions a `Point` it changes in
//! place, which they must copy, and a value with a destructor, which a
//! panic must drop; and numbers, which they needn't do either for.
use dep::{Doubled, First};
use std::collections::HashSet;
use std::panic::{self, AssertUnwindSafe};

#[derive(Clone, Copy, Debug)]
struct Point {
    x: i32,
}

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

pub fn main() {
    let context = dep::create_context(Point { x: 1 });
    let mut taken = context.current_value.replace(Point { x: 2 });
    taken.x += 10;
    println!("{:?} {:?} {:?}", taken, context.current_value.get(), context.current_value2.get());
    let (state, dispatcher) = dep::use_state(Loud(1), Some("client"));
    println!("{} {dispatcher}", state.0);
    let outside = panic::catch_unwind(AssertUnwindSafe(|| dep::use_state(Loud(2), None).1));
    println!("outside a render: {}", outside.is_err());
    let asked = panic::catch_unwind(AssertUnwindSafe(|| dep::dispatched(Loud(5), None)));
    println!("asked outside a render: {}", asked.is_err());
    println!("{}", dep::dispatched(Loud(6), Some("worker")));
    // What it compiles asks what the library's own would: a size, and a
    // `Formatter`'s width where the crate's writers take one.
    println!("{} [{:>9?}]", dep::footprint(Loud(10)), dep::Label("x"));
    // Its writer, which it compiles, and its `Display`, which it calls.
    let (early, late) = (dep::written(Loud(8), &dep::Hour(7)), dep::written(Loud(9), &dep::Hour(30)));
    println!("{early:?} {late:?} {}", dep::Hour(9));
    let (a, b) = dep::pair_of(7, Loud(4));
    println!("{a} {b}");
    let (mut c, d) = dep::twice(Point { x: 3 });
    c.x = 4;
    println!("{c:?} {d:?} {}", dep::marks());
    let numbers = dep::create_context(7);
    println!("{} {}", numbers.current_value2.get(), dep::use_state(3, Some("server")).0);
    // The library's impls it never uses itself, compiled here where they're
    // given its generic code: sets keyed by a type equal only to itself and
    // by a derived one, and a hand-written `PartialEq` and `Hash`.
    let keys = [dep::Key { label: "one" }, dep::Key { label: "two" }];
    let mut set = HashSet::new();
    for key in &keys {
        set.insert(key);
    }
    set.insert(&keys[0]);
    let tags: HashSet<dep::Tag> = [dep::Tag { id: 1 }, dep::Tag { id: 1 }, dep::Tag { id: 2 }]
        .into_iter()
        .collect();
    println!("{} {} {} {}", set.len(), tags.len(), keys[0] == keys[1], keys[1].label);
    let same_hash = dep::hash_of(&keys[0]) == dep::hash_of(&keys[0]);
    println!("{} {same_hash}", dep::same(&keys[0], &keys[1]));
    let digits = dep::hash_of(&dep::Digit(1)) == dep::hash_of(&dep::Digit(21));
    println!("{} {digits} {}", dep::same(&dep::Digit(1), &dep::Digit(11)), dep::Digit(4).doubled());
    println!("{} {}", dep::hash_of(&dep::Tag { id: 3 }), dep::shown(&dep::Tag { id: 4 }));
    // A `Copy` struct and a pair the library copies, changed in place here.
    let spots = [dep::Spot { x: 1 }, dep::Spot { x: 2 }];
    let mut spot = dep::first_spot(&spots);
    spot.x += 10;
    let mut down = dep::outer(&spots, 1);
    down.x += 20;
    let mut up = dep::inner(&spots, 2);
    up.x += 30;
    println!("{down:?} {up:?} {:?}", spots[0]);
    let pairs = [(1, 2), (3, 4)];
    let mut pair = dep::first_pair(&pairs);
    pair.0 = 5;
    println!("{spot:?} {:?} {pair:?} {:?} {}", spots[0], pairs[0], dep::shown(&spots[1]));
    // And an impl's, called through its dictionary or its library's code.
    let held = dep::Spots(vec![dep::Spot { x: 5 }]);
    let mut first = held.first();
    first.x += 1;
    let mut again = dep::first_of(&held);
    again.x += 2;
    println!("{first:?} {again:?} {:?}", held.0[0]);
}
