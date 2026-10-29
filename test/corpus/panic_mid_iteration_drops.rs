// A closure that panics part-way through an owning iterator: what it hadn't
// stepped, and what a `collect` had collected, dropped as Rust unwinds,
// the collected first, through `map`, `filter_map`, `for_each`, `fold` and
// `extend` (#422).

use std::panic::{catch_unwind, AssertUnwindSafe};

struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn run(name: &str, f: impl FnOnce()) {
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {}", r.is_err());
}

fn main() {
    run("map collect", || {
        let v = vec![D(1), D(2), D(3), D(4)];
        let out: Vec<D> = v.into_iter().map(|d| if d.0 == 3 { panic!("three") } else { d }).collect();
        println!("{}", out.len());
    });
    run("filter_map", || {
        let v = vec![D(5), D(6), D(7)];
        let out: Vec<u32> = v.into_iter().filter_map(|d| if d.0 == 6 { panic!("six") } else { Some(d.0) }).collect();
        println!("{out:?}");
    });
    run("for_each", || {
        let v = vec![D(8), D(9), D(10)];
        v.into_iter().for_each(|d| if d.0 == 9 { panic!("nine") });
    });
    run("fold", || {
        let v = vec![D(11), D(12), D(13)];
        let n = v.into_iter().fold(0, |acc, d| if d.0 == 12 { panic!("12") } else { acc + d.0 });
        println!("{n}");
    });
    run("extend", || {
        let mut a = vec![D(14)];
        a.extend(vec![D(15), D(16), D(17)].into_iter().map(|d| if d.0 == 16 { panic!("16") } else { d }));
    });
}
