// A `RefCell`'s guard mapped to part of what it guards: `Ref::map`,
// `RefMut::map`, `filter_map`, whose `Err` gives the guard back still held,
// and `map_split`, whose halves release the cell as the second drops; a
// guard mapped again, a half mapped, and two maps of one cell to one
// object, each released as native Rust's are; a panicking map releases
// what it was given (#1275).

use std::cell::{Ref, RefCell, RefMut};

#[derive(Debug)]
struct Point {
    x: u32,
    name: String,
}

#[derive(Debug)]
struct Outer {
    items: Vec<u32>,
    count: u32,
}

fn main() {
    // Plain: a field lent, written, and released.
    let c = RefCell::new(Point { x: 1, name: "p".into() });
    {
        let x = Ref::map(c.borrow(), |p| &p.x);
        let name = Ref::map(c.borrow(), |p| &p.name);
        println!("{} {} {:?}", *x, *name, c.try_borrow_mut().is_ok());
    }
    println!("{:?}", c.try_borrow_mut().is_ok());
    {
        let mut x = RefMut::map(c.borrow_mut(), |p| &mut p.x);
        *x += 41;
        println!("{:?}", c.try_borrow().is_ok());
    }
    println!("{:?}", c.borrow());
    // filter_map: its `Err` gives the guard back, still held.
    let r = Ref::filter_map(c.borrow(), |p| if p.x > 100 { Some(&p.x) } else { None });
    match r {
        Ok(x) => println!("ok {}", *x),
        Err(g) => println!("err {} {:?}", g.x, c.try_borrow_mut().is_ok()),
    }
    println!("{:?}", c.try_borrow_mut().is_ok());
    // map_split: both halves hold it, released when the second drops.
    {
        let (x, name) = Ref::map_split(c.borrow(), |p| (&p.x, &p.name));
        drop(x);
        println!("{} {:?}", *name, c.try_borrow_mut().is_ok());
    }
    println!("{:?}", c.try_borrow_mut().is_ok());
    // A panicking closure releases what it was given.
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _g = Ref::map(c.borrow(), |_p| -> &u32 { panic!("in map") });
    }));
    println!("{} {:?}", caught.is_err(), c.try_borrow_mut().is_ok());

    let outer = RefCell::new(Outer { items: vec![1, 2], count: 0 });
    outer.borrow_mut().count += 1;
    // (c) Nested maps, and a map of one of `map_split`'s halves.
    {
        let o = Ref::map(outer.borrow(), |o| o);
        let n = Ref::map(o, |o| &o.count);
        println!("{} {:?}", *n, outer.try_borrow_mut().is_ok());
    }
    println!("{:?}", outer.try_borrow_mut().is_ok());
    {
        let (x, name) = Ref::map_split(c.borrow(), |p| (&p.x, &p.name));
        let len = Ref::map(name, |n| n);
        drop(x);
        println!("{} {:?}", len.len(), c.try_borrow_mut().is_ok());
    }
    println!("{:?}", c.try_borrow_mut().is_ok());

    // (d) Two shared maps of one cell to one object.
    let v = RefCell::new((vec![1u32, 2], 0u32));
    {
        let a = Ref::map(v.borrow(), |t| &t.0);
        let b = Ref::map(v.borrow(), |t| &t.0);
        drop(a);
        println!("{:?} {:?}", *b, v.try_borrow_mut().is_ok());
    }
    println!("{:?}", v.try_borrow_mut().is_ok());
}
