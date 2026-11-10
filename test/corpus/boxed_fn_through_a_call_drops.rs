// A boxed function that reaches std through a call, `at_0(Box::new(f)) as
// Box<dyn ..>`, or as a local: left unrun by `or_insert_with` of a key
// that's there, it's dropped there, as std's drops it; run by
// `catch_unwind`, a `dyn Fn`'s dropped as its call returns; and held by a `scan`, whose function generic code takes
// through an arrow that gives it the state's box, dropped with the scan,
// before its state, as `Scan { iter, f, state }` drops its fields, in both
// builds (#1220).
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

struct Loud(u32);
impl Loud {
    fn get(&self) -> u32 {
        self.0
    }
}
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn at_0<T>(it: T) -> T {
    it
}
fn entries_and_catch() {
    let mut counts = HashMap::new();
    counts.insert("x", 0);
    let a = Loud(22);
    *counts
        .entry("x")
        .or_insert_with(at_0(Box::new(move || a.get())) as Box<dyn FnOnce() -> u32 + '_>) += 0;
    println!("after entry");
    let b = Loud(30);
    let k: Box<dyn FnOnce() -> u32> = Box::new(move || b.get());
    *counts.entry("x").or_insert_with(k) += 0;
    println!("after local entry");
    let c = Loud(40);
    let r = catch_unwind(AssertUnwindSafe(
        at_0(Box::new(move || c.get())) as Box<dyn FnOnce() -> u32>
    ));
    println!("caught {:?}", r.ok());
    // A `dyn Fn` box, whose call doesn't consume it: dropped as it returns.
    let e = Loud(60);
    let r = catch_unwind(AssertUnwindSafe(at_0(Box::new(move || e.get())) as Box<dyn Fn() -> u32>));
    println!("caught {:?}", r.ok());
    let d = Loud(50);
    let sums: Vec<u32> = [1u32, 2]
        .iter()
        .scan(
            0,
            at_0(Box::new(move |acc: &mut u32, &x: &u32| {
                *acc += x + d.get() * 0;
                Some(*acc)
            })) as Box<dyn FnMut(&mut u32, &u32) -> Option<u32>>,
        )
        .collect();
    println!("scan {sums:?}");
    println!("end");
}

struct D(&'static str);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct It {
    left: u32,
    _held: D,
}

impl Iterator for It {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        (self.left > 0).then(|| {
            self.left -= 1;
            self.left
        })
    }
}

fn it(left: u32) -> It {
    It { left, _held: D("src") }
}

fn zero(_g: &D) -> u32 {
    0
}

fn scans() {
    let g = D("f");
    let n: u32 = (0..3u32)
        .scan(
            0u32,
            at_0(Box::new(move |s: &mut u32, x: u32| {
                *s += x + zero(&g);
                Some(*s)
            })) as Box<dyn FnMut(&mut u32, u32) -> Option<u32>>,
        )
        .sum();
    println!("function sum {n}");
    let g = D("f");
    let n = it(3)
        .scan(
            D("st"),
            at_0(Box::new(move |_: &mut D, x: u32| Some(x + zero(&g)))) as Box<dyn FnMut(&mut D, u32) -> Option<u32>>,
        )
        .fold(0, |a, x| a + x);
    println!("both fold {n}");
    let g = D("f");
    let mut held = it(3).scan(
        D("st"),
        at_0(Box::new(move |_: &mut D, x: u32| Some(x + zero(&g)))) as Box<dyn FnMut(&mut D, u32) -> Option<u32>>,
    );
    println!("next {:?}", held.next());
    drop(held);
    println!("dropped");
    let g = D("f");
    let v: Vec<u32> = (0..3u32)
        .scan(
            D("st"),
            at_0(Box::new(move |_: &mut D, x: u32| Some(x + zero(&g)))) as Box<dyn FnMut(&mut D, u32) -> Option<u32>>,
        )
        .map(|x| x * 2)
        .collect();
    println!("then map {v:?}");
}

fn main() {
    entries_and_catch();
    scans();
}
