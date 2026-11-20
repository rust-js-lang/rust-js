//@ library-differs
// A `Box` of std's adapter steps as std's `Box<I>` does: it writes `next`,
// `size_hint`, `nth`, `last`, their back's and `len`, passing each on to
// what it holds, and leaves `fold`, `count`, `try_fold` and the rest to the
// traits' defaults over its `next`. So a `count()` or a `fold()` of a boxed
// `map` steps the crate's iterator by `next`, never asking its own `fold`,
// and a boxed `skip(1)` lent to a `fold` asks `Skip`'s `next`'s `nth(1)`,
// never its own `try_fold`'s `nth(0)`: whether the box is called on, bound
// first, lent or given to generic code, as a boxed `filter`'s is; a bare box
// of the crate's iterator, the same (#2179).
struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }

    fn nth(&mut self, n: usize) -> Option<u32> {
        println!("own nth {n}");
        for _ in 0..n {
            self.next()?;
        }
        self.next()
    }

    fn fold<B, F: FnMut(B, u32) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own fold");
        let mut acc = init;
        while let Some(x) = self.next() {
            acc = f(acc, x);
        }
        acc
    }
}

fn total<I: Iterator<Item = u32>>(it: I) -> u32 {
    it.fold(0, |a, x| a + x)
}

fn main() {
    println!("inline {}", Box::new(Countdown(3).map(|x| x * 2)).count());
    let boxed = Box::new(Countdown(3).map(|x| x * 2));
    println!("bound {}", boxed.fold(0, |a, x| a + x));
    println!("generic {}", total(Box::new(Countdown(3).map(|x| x * 2))));
    let mut boxed = Box::new(Countdown(3).skip(1));
    println!("lent {}", (&mut boxed).fold(0, |a, x| a + x));
    let boxed = Box::new(Countdown(3).filter(|x| x % 2 == 0));
    println!("filter {}", boxed.count());
    println!("bare {}", Box::new(Countdown(3)).count());
    println!("unboxed {}", Countdown(3).map(|x| x * 2).count());
}
