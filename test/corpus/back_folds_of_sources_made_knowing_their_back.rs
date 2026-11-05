// An adapter folded from its back, `rfold`, is given a source made knowing
// its back (#778): an `enumerate()` of a map of a `Vec`'s items with a
// destructor, `$enumerate($mapEnds(..))`, not of JS's own `.map(f)`; and a
// `copied()` of std's compiled `fuse()` or `peekable()`, its copy its
// source, given the struct's compiled back where it's made; and an
// `enumerate()` of a slice's `cloned()` of items with a destructor, JS's
// `.map(clone)`, and of the crate's iterator's `skip()` and `rev()` of
// them, its own `rfold` and `nth_back` asked, `$skipping(.., drop)` made of
// it knowing its back. Beside them, the same of items with none.
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Debug)]
struct Acc(u32);
impl Drop for Acc {
    fn drop(&mut self) {
        println!("drop acc {}", self.0);
    }
}

fn ds() -> Vec<D> {
    (1..=5).map(D).collect()
}

/// The crate's own iterator, double-ended and exact-sized.
struct Every(u32, u32);
impl Iterator for Every {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.0 < self.1).then(|| {
            self.0 += 1;
            self.0
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.1 - self.0) as usize;
        (n, Some(n))
    }
}
impl DoubleEndedIterator for Every {
    fn next_back(&mut self) -> Option<u32> {
        (self.0 < self.1).then(|| {
            self.1 -= 1;
            self.1 + 1
        })
    }
}
impl ExactSizeIterator for Every {}

#[derive(Debug)]
struct C(u32);
impl Clone for C {
    fn clone(&self) -> C {
        println!("clone {}", self.0);
        C(self.0)
    }
}
impl Drop for C {
    fn drop(&mut self) {
        println!("drop c{}", self.0);
    }
}

/// The crate's iterator of what has a destructor, with its own back steps.
struct Gives(u32, u32);
impl Drop for Gives {
    fn drop(&mut self) {
        println!("drop gives");
    }
}
impl Iterator for Gives {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        (self.0 < self.1).then(|| {
            self.0 += 1;
            D(self.0)
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.1 - self.0) as usize;
        (n, Some(n))
    }
}
impl DoubleEndedIterator for Gives {
    fn next_back(&mut self) -> Option<D> {
        (self.0 < self.1).then(|| {
            self.1 -= 1;
            D(self.1 + 1)
        })
    }
    fn rfold<B, F: FnMut(B, D) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own rfold");
        let mut acc = init;
        while let Some(x) = self.next_back() {
            acc = f(acc, x);
        }
        acc
    }
}
impl ExactSizeIterator for Gives {}

fn main() {
    let cs = vec![C(1), C(2), C(3)];
    let acc = cs.iter().cloned().enumerate().rfold(Acc(0), |mut a, (i, c)| {
        a.0 = a.0 * 10 + i as u32 + c.0;
        a
    });
    println!("{acc:?}");
    let acc = Gives(0, 5).skip(1).enumerate().rfold(Acc(0), |mut a, (i, d)| {
        a.0 = a.0 * 10 + i as u32 + d.0;
        a
    });
    println!("{acc:?}");
    let acc = Gives(0, 4).rev().enumerate().rfold(Acc(0), |mut a, (i, d)| {
        a.0 = a.0 * 10 + i as u32 + d.0;
        a
    });
    println!("{acc:?}");
    let acc = ds()
        .into_iter()
        .map(|x| D(x.0 + 100))
        .enumerate()
        .rfold(Acc(0), |mut a, (i, x)| {
            a.0 += i as u32 * 1000 + x.0;
            a
        });
    println!("{acc:?}");
    let v = vec![1u32, 2, 3];
    let acc = v.iter().cloned().enumerate().rfold(Acc(0), |mut a, (i, x)| {
        a.0 += i as u32 * 10 + x;
        a
    });
    println!("{acc:?}");
    let acc = Every(0, 5).skip(1).enumerate().rfold(Acc(0), |mut a, (i, x)| {
        a.0 = a.0 * 10 + i as u32 + x;
        a
    });
    println!("{acc:?}");
    let acc = Every(0, 4).rev().enumerate().rfold(Acc(0), |mut a, (i, x)| {
        a.0 = a.0 * 10 + i as u32 + x;
        a
    });
    println!("{acc:?}");
    let nums = [1u32, 2, 3, 4, 5];
    let acc = nums.iter().fuse().copied().rfold(Acc(0), |mut a, x| {
        a.0 = a.0 * 10 + x;
        a
    });
    println!("{acc:?}");
    let acc = nums.iter().peekable().copied().rfold(Acc(0), |mut a, x| {
        a.0 = a.0 * 10 + x;
        a
    });
    println!("{acc:?}");
}
