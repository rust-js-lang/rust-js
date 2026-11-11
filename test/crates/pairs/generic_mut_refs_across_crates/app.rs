//! The app gives the library's generic `&mut T` code its own places: a
//! local number, a struct's field, a `Vec`'s item, an array's, a tuple's
//! parts, an `Option<String>` taken and replaced, and a slot held over a
//! number, each read after as Rust leaves it. A slot held over a struct's
//! field, a `&mut Point` in what a generic function gives, is refused, in
//! one crate as in two.
use dep::{both, bump, hold, renew, swap_ends};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug)]
struct Shape {
    at: Point,
    scale: u8,
    names: Vec<String>,
}

#[derive(Debug, Default, PartialEq, PartialOrd)]
struct Loud(u32);

impl Clone for Loud {
    fn clone(&self) -> Loud {
        println!("clone {}", self.0);
        Loud(self.0 + 100)
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub fn main() {
    let mut n = 7u32;
    bump(&mut n, 5);
    let mut shape = Shape { at: Point { x: 1, y: 2 }, scale: 3, names: vec!["a".into(), "b".into(), "c".into()] };
    bump(&mut shape.scale, 4);
    bump(&mut shape.at.x, 10);
    let mut v = vec![1.5f64, 2.5, 3.5];
    bump(&mut v[1], 0.25);
    let mut arr = [10i64, 20, 30];
    bump(&mut arr[2], -5);
    println!("{n} {} {:?} {v:?} {arr:?}", shape.scale, shape.at);

    swap_ends(&mut shape.names);
    swap_ends(&mut arr);
    let mut points = [Point { x: 1, y: 1 }, Point::default(), Point { x: 9, y: 9 }];
    swap_ends(&mut points);
    println!("{:?} {arr:?} {points:?}", shape.names);

    {
        let mut slot = hold(&mut n);
        slot.set(slot.get() * 2);
    }
    println!("{:?} {n}", shape.at);

    let mut label: Option<String> = Some("x".into());
    renew(&mut label, |old| old.map(|s| s + "y"));
    let mut count = 3usize;
    renew(&mut count, |c| c + 1);
    let mut pair = (1u8, 2.5f32);
    both(&mut pair, 2, 0.5);
    println!("{label:?} {count} {pair:?}");

    // Of a type that drops: the consumer's own copy of each (ADR 0914).
    let mut louds = vec![Loud(1), Loud(2), Loud(3)];
    swap_ends(&mut louds);
    let mut held = Some(Loud(4));
    renew(&mut held, |old| old.map(|l| Loud(l.0 * 10)));
    let mut one = Loud(5);
    renew(&mut one, |l| Loud(l.0 + 1));
    println!("{louds:?} {held:?} {one:?}");
}
