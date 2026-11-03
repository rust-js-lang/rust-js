//@ run-fail: slice index starts at 3 but ends at 2
// A part of a slice reversed, `v[a..b].reverse()`, is those items of the
// slice reversed where they are, checked as `&mut v[a..b]` is: past the
// end, or ending before it starts, it panics before any item moves. A part
// kept, `let part = &mut v[a..b]`, is a view, which `reverse` reverses too.

use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};

#[derive(Debug)]
struct Point {
    x: i32,
}

impl Drop for Point {
    fn drop(&mut self) {
        if self.x == 9 {
            println!("dropped");
        }
    }
}

struct Queue {
    items: Vec<Option<Point>>,
    length: usize,
}

impl Queue {
    fn flip_from(&mut self, first: usize) {
        self.items[first..self.length].reverse();
    }
}

fn tail(v: &mut Vec<i32>) -> &mut [i32] {
    &mut v[1..]
}

fn bump(part: &mut [i32], by: i32) {
    for x in part {
        *x += by;
    }
}

fn counted(calls: &Cell<u32>) -> i32 {
    calls.set(calls.get() + 1);
    1
}

fn bound(seen: &mut Vec<&'static str>, name: &'static str, at: usize) -> usize {
    seen.push(name);
    at
}

fn flip(v: &mut [String], a: usize, b: usize) {
    v[a..b].reverse();
}

fn main() {
    let mut v = vec![1, 2, 3, 4, 5, 6];
    v[1..5].reverse();
    println!("{v:?}");
    v[3..].reverse();
    println!("{v:?}");
    v[..2].reverse();
    println!("{v:?}");
    v[2..2].reverse();
    v[4..5].reverse();
    v[..].reverse();
    println!("{v:?}");
    v[1..=3].reverse();
    println!("{v:?}");
    v[1..5][1..3].reverse();
    println!("{v:?}");
    tail(&mut v)[2..].reverse();
    println!("{v:?}");

    let mut seen = Vec::new();
    let mut a = [10, 20, 30, 40, 50];
    a[bound(&mut seen, "start", 0)..bound(&mut seen, "end", 3)].reverse();
    println!("{a:?} {seen:?}");

    let mut words: Vec<String> = ["a", "b", "c", "d"].map(String::from).to_vec();
    flip(&mut words, 0, 3);
    println!("{words:?}");

    let mut queue = Queue {
        items: vec![Some(Point { x: 1 }), None, Some(Point { x: 3 }), Some(Point { x: 4 })],
        length: 3,
    };
    queue.flip_from(0);
    queue.items[0].as_mut().unwrap().x = 30;
    println!("{:?}", queue.items);

    let part = &mut v[2..];
    part.reverse();
    part[0] = 0;
    println!("{v:?}");

    if let Some(part) = v.get_mut(..3) {
        part.reverse();
    }
    println!("{v:?}");

    // A view's range is checked where it's made, before what's given after it.
    let early = panic::catch_unwind(|| {
        let mut v = vec![1, 2, 3];
        bump(&mut v[5..], {
            println!("by");
            1
        });
        v
    });
    println!("{}", early.is_err());
    let calls = Cell::new(0);
    let early = panic::catch_unwind(AssertUnwindSafe(|| {
        let _loud = Point { x: 9 };
        let mut v = vec![1, 2, 3];
        bump(&mut v[5..], {
            let by = counted(&calls);
            by + counted(&calls)
        });
        v
    }));
    println!("{} {}", early.is_err(), calls.get());
    let early = panic::catch_unwind(|| {
        let mut v = vec![1, 2, 3];
        bump(&mut v[5..], Point { x: 9 }.x);
        v
    });
    println!("{}", early.is_err());
    let past = panic::catch_unwind(|| {
        let mut v = vec![1, 2, 3];
        v[1..4].reverse();
        v
    });
    println!("{}", past.is_err());
    let mut v = vec![1, 2, 3];
    let start = v.len();
    v[start..2].reverse();
    println!("{v:?}");
}
