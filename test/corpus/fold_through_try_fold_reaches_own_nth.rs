// What std's `fold` of an adapter reaches through a `try_fold` its own
// steps ask reaches the crate's own `nth` (#2195): `Take::fold` is its
// source's `try_fold`, and `Skip::try_fold` asks its source's `nth(n - 1)`,
// through a `fuse` or a `peekable`, which pass `nth` on, so `last`, `fold`,
// `sum` and a generic `last` of a `take` of a `skip` ask `nth(0)`, not the
// `nth(1)` its `next` asks; a `step_by`'s `try_fold` asks `nth` too. And a
// `take` of a `rev()` of a `Vec`'s `take`, folded, reaches the `nth_back`
// its source asks through the `try_fold`s, which drops what it passes as
// one slice; and a fold by value of a closure that drops, of one that
// holds what drops, drops the closure first.
struct Count(u32);

impl Iterator for Count {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 6 {
            return None;
        }
        self.0 += 1;
        Some(self.0)
    }

    fn nth(&mut self, n: usize) -> Option<u32> {
        println!("own nth {n}");
        for _ in 0..n {
            self.next()?;
        }
        self.next()
    }
}

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// Counts to 4, with its own `nth`, holding what drops.
struct Held(u32, Loud);

impl Iterator for Held {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 4 {
            return None;
        }
        self.0 += 1;
        Some(self.0)
    }

    fn nth(&mut self, n: usize) -> Option<u32> {
        println!("held nth {n}");
        for _ in 0..n {
            self.next()?;
        }
        self.next()
    }
}

fn last_of<I: Iterator>(it: I) -> Option<I::Item> {
    it.last()
}

fn main() {
    println!("last {:?}", Count(0).fuse().skip(1).take(3).last());
    println!("peekable {:?}", Count(0).peekable().skip(1).take(3).last());
    let folded = Count(0).fuse().skip(1).take(3).fold(0, |a, x| a * 10 + x);
    println!("fold {folded}");
    println!("sum {}", Count(0).fuse().skip(2).take(2).sum::<u32>());
    println!("count {}", Count(0).fuse().skip(1).take(3).count());
    println!("step_by {:?}", Count(0).fuse().step_by(2).take(2).last());
    println!("generic {:?}", last_of(Count(0).fuse().skip(1).take(3)));
    println!("max {:?}", Count(0).fuse().skip(1).take(3).max());
    let g = Loud(10);
    let held = Held(0, Loud(20)).fuse().skip(1).take(2);
    let n = held.fold(0, move |a, x| {
        let _g = &g;
        a + x
    });
    println!("held {n}");
    let v: Vec<Loud> = (1..=6).map(Loud).collect();
    let n = v.into_iter().take(3).rev().take(2).fold(0, |a, x| a + x.0);
    println!("folded {n}");
    let w: Vec<Loud> = (1..=6).map(Loud).collect();
    let m = w.into_iter().rev().take(4).rev().take(2).fold(0, |a, x| a + x.0);
    println!("folded {m}");
}
