// A `skip(1)` whose first `nth(0)` gives back an item whose drop panics:
// as that unwinds, what the consumer's frame holds is dropped, its
// accumulator, its closure, then what's left of the source, as std's
// `Skip` drops that item in the frame of each step (#844).
use std::panic::catch_unwind;

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 1 {
            panic!("drop");
        }
    }
}

struct Acc(u32);

impl Drop for Acc {
    fn drop(&mut self) {
        println!("drop acc {}", self.0);
    }
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3)]
}

fn main() {
    let r = catch_unwind(|| {
        let g = D(90);
        let acc = ds().into_iter().skip(1).fold(Acc(0), move |mut a, x| {
            let _g = &g;
            a.0 += x.0;
            a
        });
        println!("fold {}", acc.0);
    });
    println!("{}", r.is_err());
    let r = catch_unwind(|| {
        let mut it = ds().into_iter().skip(1);
        let t = it.try_fold(Acc(0), |mut a, x| {
            a.0 += x.0;
            if a.0 > 9 { Err(a.0) } else { Ok(a) }
        });
        println!("try_fold {}", t.is_ok());
    });
    println!("{}", r.is_err());
    let r = catch_unwind(|| {
        let n = ds().into_iter().skip(1).count();
        println!("count {n}");
    });
    println!("{}", r.is_err());
    let r = catch_unwind(|| {
        let l = ds().into_iter().skip(1).last();
        println!("last {}", l.map_or(0, |d| d.0));
    });
    println!("{}", r.is_err());
    let r = catch_unwind(|| {
        ds().into_iter().skip(1).for_each(|x| println!("item {}", x.0));
    });
    println!("{}", r.is_err());
    let r = catch_unwind(|| {
        let s: String = ds().into_iter().skip(1).map(|x| char::from(b'a' + x.0 as u8)).collect();
        println!("string {s}");
    });
    println!("{}", r.is_err());
}
