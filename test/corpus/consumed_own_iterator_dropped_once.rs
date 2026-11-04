// The crate's iterator with a `Drop`, over items with a `Drop`, consumed
// by each of std's consumers, whole, held, opaque and generic, and with a
// panic part-way under `catch_unwind`: dropped once, by the consumer's
// closing, at its end or as the panic unwinds, never again after it
// (#637); and what takes it by `&mut`, `any`, `find`, `position` and the
// `try_` forms, leaves it to its owner.
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct It {
    items: Vec<D>,
}

impl Drop for It {
    fn drop(&mut self) {
        println!("drop it");
    }
}

impl Iterator for It {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        self.items.pop()
    }
}

fn it(n: u32) -> It {
    It {
        items: (n..n + 3).map(D).collect(),
    }
}

fn run(name: &str, f: impl FnOnce()) {
    println!("-- {name}");
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {}", r.is_err());
}

fn main() {
    run("fold", || println!("{}", it(0).fold(0, |n, d| n + d.0)));
    run("fold panics", || {
        println!("{}", it(0).fold(0, |n, d| if d.0 == 1 { panic!("1") } else { n + d.0 }))
    });
    run("for_each", || it(10).for_each(|d| println!("each {}", d.0)));
    run("count", || println!("{}", it(20).count()));
    run("last", || println!("{:?}", it(30).last().map(|d| d.0)));
    run("sum", || println!("{}", it(40).map(|d| d.0).sum::<u32>()));
    run("collect", || println!("{}", it(50).collect::<Vec<_>>().len()));
    run("any", || println!("{}", it(60).any(|d| d.0 == 61)));
    run("all", || println!("{}", it(70).all(|d| d.0 > 71)));
    run("find", || println!("{:?}", it(80).find(|d| d.0 == 81).map(|d| d.0)));
    run("position", || println!("{:?}", it(90).position(|d| d.0 == 91)));
    run("try_fold", || {
        println!("{:?}", it(100).try_fold(0, |n, d| (d.0 != 101).then_some(n + d.0)))
    });
    run("try_for_each", || {
        println!(
            "{:?}",
            it(110).try_for_each(|d| if d.0 == 111 { Err(d.0) } else { Ok(()) })
        )
    });
    more();
    run("held fold", || {
        let i = it(120);
        println!("{}", i.fold(0, |n, d| n + d.0));
    });
}

fn opaque(n: u32) -> impl Iterator<Item = D> {
    it(n)
}

fn generic<I: Iterator<Item = D>>(i: I) -> u32 {
    i.fold(0, |n, d| n + d.0)
}

fn more() {
    run("opaque fold", || println!("{}", opaque(130).fold(0, |n, d| n + d.0)));
    run("generic fold", || println!("{}", generic(it(140))));
    run("by_ref fold", || {
        let mut i = it(150);
        println!("{}", i.by_ref().take(1).fold(0, |n, d| n + d.0));
        println!("{}", i.fold(0, |n, d| n + d.0));
    });
    run("for_each panics", || {
        it(160).for_each(|d| {
            if d.0 == 161 {
                panic!("161")
            }
        })
    });
    run("count of filter", || {
        println!("{}", it(170).filter(|d| d.0 > 170).count())
    });
}
