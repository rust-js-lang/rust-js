// A `scan()`'s function and state with a destructor, dropped once, as it's
// closed, after its source, as `Scan { iter, f, state }` drops its fields:
// consumed by value, collected, held and dropped, a state the closure
// replaces, as a panic in it unwinds, and of a scan an adapter holds (#760).
use std::panic::{AssertUnwindSafe, catch_unwind};

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

fn main() {
    let n = (0..3u32).scan(D("st"), |_, x| Some(x)).count();
    println!("state count {n}");
    let v: Vec<u32> = (0..3u32).scan(D("st"), |_, x| Some(x)).collect();
    println!("state collect {v:?}");
    let g = D("f");
    let n: u32 = (0..3u32)
        .scan(0u32, move |s, x| {
            *s += x + zero(&g);
            Some(*s)
        })
        .sum();
    println!("function sum {n}");
    let g = D("f");
    let n = it(3)
        .scan(D("st"), move |_, x| Some(x + zero(&g)))
        .fold(0, |a, x| a + x);
    println!("both fold {n}");
    let g = D("f");
    let mut held = it(3).scan(D("st"), move |_, x| Some(x + zero(&g)));
    println!("next {:?}", held.next());
    drop(held);
    println!("dropped");
    let n = (0..4u32)
        .scan(D("st0"), |s, x| {
            if x == 1 {
                *s = D("st1");
            }
            Some(x)
        })
        .count();
    println!("replaced {n}");
    let g = D("f");
    let r = catch_unwind(AssertUnwindSafe(|| {
        it(3)
            .scan(D("st"), move |_, x| {
                assert!(x != 1, "scan");
                Some(x + zero(&g))
            })
            .count()
    }));
    println!("panicked {}", r.is_err());
    let g = D("f");
    let v: Vec<u32> = (0..3u32)
        .scan(D("st"), move |_, x| Some(x + zero(&g)))
        .map(|x| x * 2)
        .collect();
    println!("then map {v:?}");
    let v: Vec<u32> = (0..5u32).scan(D("st"), |_, x| Some(x)).take(2).collect();
    println!("taken {v:?}");
    let n = (0..3u32)
        .scan(Some(D("opt")), |s, x| {
            if x == 2 {
                *s = None;
            }
            Some(x)
        })
        .count();
    println!("option {n}");
}
