//! A library's iterator that writes its own `fold`, consumed in the library
//! and by its user: std's consumers call that `fold`, which is given its
//! accumulator's and its closure's drops, so what it takes, what a panic in
//! it unwinds past and what a closure holds drop as natively (#2271, ADR
//! 0687's amendment of 2026-11-02).
pub struct D(pub u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop D{}", self.0);
    }
}

/// Counts to `end`, its own `fold` panicking at `boom`.
pub struct Ds {
    pub n: u32,
    pub end: u32,
    pub boom: u32,
}

impl Iterator for Ds {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        if self.n < self.end {
            self.n += 1;
            Some(D(self.n))
        } else {
            None
        }
    }

    fn fold<B, F: FnMut(B, D) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own fold");
        let mut acc = init;
        while let Some(d) = self.next() {
            if d.0 == self.boom {
                panic!("in fold");
            }
            acc = f(acc, d);
        }
        acc
    }
}

pub fn ds(boom: u32) -> Ds {
    Ds { n: 0, end: 4, boom }
}

/// A generic adapter whose own `fold` folds its source's, holding a value
/// with a destructor as it does, and panicking at its source's `2` where
/// that value is `77`.
pub struct Two<I>(pub I, pub u32);

impl<I: Iterator<Item = D>> Iterator for Two<I> {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        self.0.next()
    }

    fn fold<B, F: FnMut(B, D) -> B>(self, init: B, mut f: F) -> B {
        println!("two fold");
        let guard = D(self.1);
        self.0.fold(init, |acc, d| {
            if d.0 == 2 && guard.0 == 77 {
                panic!("two");
            }
            f(acc, d)
        })
    }
}

/// A fold of the caller's iterator, its accumulator with a destructor.
pub struct Total(pub u32);

impl Drop for Total {
    fn drop(&mut self) {
        println!("drop Total{}", self.0);
    }
}

pub fn total<I: Iterator<Item = u32>>(it: I) -> u32 {
    it.fold(Total(0), |a, x| Total(a.0 + x)).0
}

fn caught(name: &str, f: &dyn Fn()) {
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    println!("{name} {}", r.is_err());
}

/// std's consumers of `Ds`, in the library, its own `fold` panicking at
/// `boom`.
pub fn consumed(boom: u32) {
    caught("last", &|| println!("{:?}", ds(boom).last().map(|d| d.0)));
    caught("max", &|| println!("{:?}", ds(boom).max_by_key(|d| d.0).map(|d| d.0)));
    caught("min", &|| {
        println!("{:?}", ds(boom).min_by(|a, b| a.0.cmp(&b.0)).map(|d| d.0))
    });
    caught("reduce", &|| {
        let r = ds(boom).reduce(|a, b| if a.0 > b.0 { a } else { b });
        println!("{:?}", r.map(|d| d.0))
    });
    caught("collect", &|| println!("{}", ds(boom).collect::<Vec<D>>().len()));
    caught("partition", &|| {
        let (a, b): (Vec<D>, Vec<D>) = ds(boom).partition(|d| d.0 % 2 == 0);
        println!("{} {}", a.len(), b.len())
    });
    caught("count", &|| println!("{}", ds(boom).count()));
    caught("sum", &|| println!("{}", ds(boom).map(|d| d.0).sum::<u32>()));
    caught("for_each", &|| ds(boom).for_each(|d| println!("each {}", d.0)));
    caught("unzip", &|| {
        let (a, b): (Vec<D>, Vec<u32>) = ds(boom)
            .map(|d| {
                let n = d.0;
                (d, n)
            })
            .unzip();
        println!("{} {}", a.len(), b.len())
    });
    caught("fold-vec", &|| {
        let v = ds(boom).fold(Vec::new(), |mut v, d| {
            v.push(d);
            v
        });
        println!("{}", v.len())
    });
    caught("fold-acc", &|| {
        println!("{}", ds(boom).fold(D(400), |a, d| D(a.0 + d.0)).0)
    });
    caught("fold-captured", &|| {
        let keep = D(100);
        let sum = ds(boom).fold(0, move |a, d| {
            let k = &keep;
            a + d.0 + k.0 - 100
        });
        println!("{sum}");
        println!("after");
    });
    caught("each-captured", &|| {
        let keep = D(200);
        ds(boom).for_each(move |d| {
            let k = &keep;
            println!("each {} {}", d.0, k.0)
        });
        println!("after");
    });
    caught("max-captured", &|| {
        let keep = D(900);
        let max = ds(boom).max_by_key(move |d| {
            let k = &keep;
            d.0 + k.0
        });
        println!("{:?}", max.map(|d| d.0));
        println!("after");
    });
}

/// std's consumers of `Two`, in the library.
pub fn adapted(boom: u32, guard: u32) {
    caught("two last", &|| {
        println!("{:?}", Two(ds(boom), guard).last().map(|d| d.0))
    });
    caught("two collect", &|| {
        println!("{}", Two(ds(boom), guard).collect::<Vec<D>>().len())
    });
    caught("two fold", &|| {
        println!("{}", Two(ds(boom), guard).fold(D(50), |a, d| D(a.0 + d.0)).0)
    });
    caught("two count", &|| println!("{}", Two(ds(boom), guard).count()));
    caught("two for_each", &|| {
        Two(ds(boom), guard).for_each(|d| println!("each {}", d.0))
    });
    caught("two of a vec", &|| {
        let source = vec![D(31), D(32), D(33)].into_iter();
        println!("{}", Two(source, guard).fold(D(60), |a, d| D(a.0 + d.0)).0)
    });
}
