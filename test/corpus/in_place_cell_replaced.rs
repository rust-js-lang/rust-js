// `Cell::from_mut` of an object writes it in place: what its `replace`,
// `take` and `swap` move out is a copy apart, not the object the write
// changes (#527, ADR 0454), of a `Vec`, a `String` and a struct, a local's
// and a parameter's; and generic code's `mem::replace`, `take` and `swap`.
use std::cell::Cell;
use std::mem;

#[derive(Debug, Default, Clone, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

fn replace_in<T>(r: &mut T, v: T) -> T {
    mem::replace(r, v)
}

fn take_in<T: Default>(r: &mut T) -> T {
    mem::take(r)
}

fn swap_in<T>(a: &mut T, b: &mut T) {
    mem::swap(a, b)
}

fn cell_replace(v: &mut Vec<i32>) -> Vec<i32> {
    Cell::from_mut(v).replace(vec![9])
}

fn main() {
    let mut v = vec![1, 2];
    let old = Cell::from_mut(&mut v).replace(vec![3]);
    println!("{old:?} {v:?}");

    let mut v = vec![1, 2];
    let old = Cell::from_mut(&mut v).take();
    println!("{old:?} {v:?}");

    let mut a = vec![1, 2];
    let mut b = vec![3, 4];
    Cell::from_mut(&mut a).swap(Cell::from_mut(&mut b));
    println!("{a:?} {b:?}");

    let mut s = String::from("old");
    let old = Cell::from_mut(&mut s).replace(String::from("new"));
    println!("{old} {s}");

    let mut p = Point { x: 1, y: 2 };
    let old = Cell::from_mut(&mut p).replace(Point { x: 3, y: 4 });
    println!("{old:?} {p:?}");

    let mut p = Point { x: 1, y: 2 };
    let old = Cell::from_mut(&mut p).take();
    println!("{old:?} {p:?}");

    let mut v = vec![1, 2];
    let old = cell_replace(&mut v);
    println!("{old:?} {v:?}");

    let mut v = vec![1, 2];
    let old = replace_in(&mut v, vec![5]);
    println!("{old:?} {v:?}");

    let mut v = vec![1, 2];
    let old = take_in(&mut v);
    println!("{old:?} {v:?}");

    let mut a = vec![1, 2];
    let mut b = vec![3, 4];
    swap_in(&mut a, &mut b);
    println!("{a:?} {b:?}");

    let mut p = Point { x: 1, y: 2 };
    let old = replace_in(&mut p, Point { x: 7, y: 8 });
    println!("{old:?} {p:?}");

    let mut p = Point { x: 1, y: 2 };
    let mut q = Point { x: 5, y: 6 };
    swap_in(&mut p, &mut q);
    println!("{p:?} {q:?}");

    let mut v = vec![1, 2];
    let c = Cell::from_mut(&mut v);
    let first = c.replace(vec![3]);
    let second = c.replace(vec![4]);
    println!("{first:?} {second:?} {v:?}");
}
