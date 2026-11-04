// The crate's own `fold` and `count` are called where std's adapters call
// them, and not where they don't: a `step_by()` counts by its source's
// `next` and `nth`, never its `fold`, a `fuse()`'s too; a `skip()` counts by
// its source's `count`, a `skip()`'s or an `enumerate()`'s in turn (#671).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Folds(u32);

impl Iterator for Folds {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
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

struct Counts(u32);

impl Iterator for Counts {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        if self.0 >= 4 {
            return None;
        }
        self.0 += 1;
        Some(D(self.0))
    }
    fn count(self) -> usize {
        println!("own count");
        (4 - self.0) as usize
    }
}

struct Nths(u32);

impl Iterator for Nths {
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
}

fn main() {
    println!("{}", Folds(3).fuse().step_by(2).count());
    println!("{}", Folds(5).step_by(2).count());
    println!("{}", Nths(5).fuse().step_by(2).count());
    println!("{}", Folds(4).fuse().step_by(3).fold(0, |a, x| a + x));
    println!("{}", Counts(0).skip(1).skip(1).count());
    println!("{}", Counts(0).enumerate().skip(1).count());
    println!("{}", Counts(0).skip(1).count());
    println!("{}", Counts(0).enumerate().count());
    let it = Counts(0).skip(1);
    println!("{}", it.skip(1).count());
}
