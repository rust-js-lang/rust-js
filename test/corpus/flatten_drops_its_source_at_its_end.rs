// A flatten and a flat_map hold their source in std's `Fuse`, which drops
// it at its first `None`, and at the end of a `try_fold` that runs it out,
// held or not: of the crate's iterator, stepped by hand, by a `while let`,
// by `sum`, and inside `any` and `take_while`'s collect. The plain build kept
// it till the flatten was dropped (#1507). One a `by_ref()` lends drops with
// its owner.
struct Flaky(u32);
impl Iterator for Flaky {
    type Item = Vec<u32>;
    fn next(&mut self) -> Option<Vec<u32>> {
        self.0 += 1;
        match self.0 {
            1 => Some(vec![1, 2]),
            3 => Some(vec![3]),
            _ => None,
        }
    }
}
impl Drop for Flaky {
    fn drop(&mut self) {
        println!("drop Flaky {}", self.0);
    }
}
struct Count(u32, &'static str);
impl Iterator for Count {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 < 4 {
            self.0 += 1;
            Some(self.0)
        } else {
            None
        }
    }
}
impl Drop for Count {
    fn drop(&mut self) {
        println!("drop {} {}", self.1, self.0);
    }
}
fn c(name: &'static str) -> Count {
    Count(0, name)
}
fn flaky() {
    let mut h = Flaky(0).flatten();
    println!("{:?}", h.next());
    println!("{:?}", h.next());
    println!("{:?}", h.next());
    println!("{:?}", h.next());
    println!("end h");
    let mut g = Flaky(0).flat_map(|v| v.into_iter().map(|x| x * 10));
    println!("{:?} {:?} {:?}", g.next(), g.next(), g.next());
    println!("end g");
    let s: u32 = Flaky(0).flatten().sum();
    println!("sum {s}");
}

fn counted() {
    let found = c("outer")
        .take(2)
        .flat_map(|_| c("inner").skip(2))
        .skip_while(|x| *x < 4)
        .any(|x| x == 4);
    println!("found {found}");
    let mut by_ref = c("held");
    let total: u32 = by_ref.by_ref().take(2).flat_map(|x| vec![x, x]).sum();
    println!("total {total}");
    println!("after by_ref");
    let mut f = c("fused").map(|x| vec![x]).flatten();
    while let Some(x) = f.next() {
        println!("x {x}");
    }
    println!("after while");
    let k: Vec<u32> = c("tf").flat_map(|x| Some(x)).take_while(|x| *x < 9).collect();
    println!("{k:?}");
}

/// A `try_fold` that runs the source out, its source dropped inside it.
fn folded() {
    let mut f = c("tf2").flat_map(|x| vec![x]);
    let n = f.try_fold(0u32, |a, x| a.checked_add(x));
    println!("{n:?}");
    let mut g = c("any").flat_map(|x| vec![x, x]);
    println!("{}", g.any(|x| x > 100));
    println!("before drop");
    drop(f);
    drop(g);
}

fn main() {
    flaky();
    counted();
    folded();
}
