// A closure an adapter holds, dropped where std drops it (#710): a fold by
// value moves a `map`'s, a `filter`'s, a `filter_map`'s, an `inspect`'s
// or a `flat_map`'s function into the fold it asks of its source, which
// drops it before what it has left, a `take`'s rest; stepped by `next`, by
// `&mut`, or held, the adapter drops it after its source, as its fields
// drop, and a `take_while`'s, a `map_while`'s, a `skip_while`'s, a
// `step_by`'s and a `zip`'s fold keep it so.
use std::collections::HashSet;
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        print!("d{} ", self.0);
        if self.0 == 40 {
            panic!("drop");
        }
    }
}

// What reads the guard a closure holds, so it's moved in whole.
fn zero(_guard: &D) -> u32 {
    0
}

fn ds() -> Vec<D> {
    (1..6).map(D).collect()
}

struct Counting(u32);

impl Iterator for Counting {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        (self.0 < 2).then(|| {
            self.0 += 1;
            D(self.0)
        })
    }
}

impl Drop for Counting {
    fn drop(&mut self) {
        print!("counting ");
    }
}

fn case(name: &str, run: impl FnOnce(D)) {
    print!("{name}: ");
    let r = catch_unwind(AssertUnwindSafe(|| run(D(9))));
    println!("{}", if r.is_err() { "panicked" } else { "" });
}

fn main() {
    case("the issue's", |g| {
        let v: Vec<D> = ds()
            .into_iter()
            .take(2)
            .map(move |d| {
                let _g = &g;
                d
            })
            .chain(ds())
            .collect();
        print!("| {} | ", v.len());
    });
    case("sum", |g| {
        let n: u32 = ds().into_iter().take(2).map(move |d| d.0 + zero(&g)).sum();
        print!("| {n} | ");
    });
    case("count", |g| {
        let n = ds().into_iter().take(2).filter(move |d| d.0 > zero(&g)).count();
        print!("| {n} | ");
    });
    case("last", |g| {
        let last = ds().into_iter().take(2).map(move |d| d.0 + zero(&g)).last();
        print!("| {last:?} | ");
    });
    case("for_each", |g| {
        ds().into_iter()
            .take(2)
            .inspect(move |d| assert!(d.0 > zero(&g)))
            .for_each(drop);
    });
    case("fold", |g| {
        let n = ds()
            .into_iter()
            .take(2)
            .filter_map(move |d| Some(d.0 + zero(&g)))
            .fold(0, |a, b| a + b);
        print!("| {n} | ");
    });
    case("max", |g| {
        let max = ds().into_iter().take(2).map(move |d| d.0 + zero(&g)).max();
        print!("| {max:?} | ");
    });
    case("min_by_key", |g| {
        let min = ds().into_iter().take(2).map(move |d| d.0 + zero(&g)).min_by_key(|&n| n);
        print!("| {min:?} | ");
    });
    case("reduce", |g| {
        let n = ds()
            .into_iter()
            .take(2)
            .map(move |d| d.0 + zero(&g))
            .reduce(|a, b| a + b);
        print!("| {n:?} | ");
    });
    case("partition", |g| {
        let (odd, even): (Vec<u32>, Vec<u32>) = ds()
            .into_iter()
            .take(2)
            .map(move |d| d.0 + zero(&g))
            .partition(|n| n % 2 == 1);
        print!("| {odd:?} {even:?} | ");
    });
    case("extend", |g| {
        let mut v: Vec<u64> = Vec::new();
        v.extend(ds().into_iter().take(2).map(move |d| u64::from(d.0 + zero(&g))));
        print!("| {v:?} | ");
    });
    case("collect", |g| {
        let v: Vec<u64> = ds()
            .into_iter()
            .take(2)
            .map(move |d| u64::from(d.0 + zero(&g)))
            .collect();
        print!("| {v:?} | ");
    });
    case("collect a String", |g| {
        let s: String = ds()
            .into_iter()
            .take(2)
            .map(move |d| (d.0 + zero(&g)).to_string())
            .collect();
        print!("| {s} | ");
    });
    case("collect a set", |g| {
        let set: HashSet<u32> = ds().into_iter().take(2).map(move |d| d.0 + zero(&g)).collect();
        print!("| {} | ", set.len());
    });
    case("chained first", |g| {
        let n: u32 = ds()
            .into_iter()
            .take(2)
            .map(move |d| d.0 + zero(&g))
            .chain(ds().into_iter().take(1).map(|d| d.0))
            .sum();
        print!("| {n} | ");
    });
    case("chained second", |g| {
        let n: u32 = (0..1)
            .chain(ds().into_iter().take(2).map(move |d| d.0 + zero(&g)))
            .sum();
        print!("| {n} | ");
    });
    case("skipped", |g| {
        let n: u32 = ds().into_iter().skip(1).take(2).map(move |d| d.0 + zero(&g)).sum();
        print!("| {n} | ");
    });
    case("skipped after a take", |g| {
        let n: u32 = ds().into_iter().take(3).skip(1).map(move |d| d.0 + zero(&g)).sum();
        print!("| {n} | ");
    });
    case("taken while, then mapped", |g| {
        let n: u32 = ds()
            .into_iter()
            .take_while(|d| d.0 < 3)
            .map(move |d| d.0 + zero(&g))
            .sum();
        print!("| {n} | ");
    });
    case("enumerated", |g| {
        let n = ds()
            .into_iter()
            .take(2)
            .map(move |d| d.0 + zero(&g))
            .enumerate()
            .count();
        print!("| {n} | ");
    });
    case("flat_map", |g| {
        let n = ds()
            .into_iter()
            .take(2)
            .flat_map(move |d| {
                let _g = &g;
                vec![d]
            })
            .count();
        print!("| {n} | ");
    });
    case("peekable", |g| {
        let n = ds().into_iter().take(2).map(move |d| d.0 + zero(&g)).peekable().count();
        print!("| {n} | ");
    });
    case("fused", |g| {
        let n: u32 = ds().into_iter().take(2).map(move |d| d.0 + zero(&g)).fuse().sum();
        print!("| {n} | ");
    });
    case("the crate's iterator", |g| {
        let n: u32 = Counting(0).map(move |d| d.0 + zero(&g)).sum();
        print!("| {n} | ");
    });
    case("two closures", |g| {
        let h = D(8);
        let n: u32 = ds()
            .into_iter()
            .take(2)
            .map(move |d| {
                let _g = &g;
                d
            })
            .map(move |d| d.0 + zero(&h))
            .sum();
        print!("| {n} | ");
    });
    case("a filter's, then a map's", |g| {
        let h = D(8);
        let n: u32 = ds()
            .into_iter()
            .take(2)
            .filter(move |d| d.0 > zero(&g))
            .map(move |d| d.0 + zero(&h))
            .sum();
        print!("| {n} | ");
    });
    case("stepped, then summed", |g| {
        let mut it = ds().into_iter().take(3).map(move |d| d.0 + zero(&g));
        let first = it.next();
        let n: u32 = it.sum();
        print!("| {first:?} {n} | ");
    });
    case("a panic in what's left", |g| {
        let n: u32 = [D(1), D(2), D(40), D(5)]
            .into_iter()
            .take(2)
            .map(move |d| d.0 + zero(&g))
            .sum();
        print!("| {n} | ");
    });
    // Kept: what's left first.
    case("collected in place", |g| {
        let v: Vec<D> = ds()
            .into_iter()
            .take(2)
            .map(move |d| {
                let _g = &g;
                d
            })
            .collect();
        print!("| {} | ", v.len());
    });
    case("collected in place, of numbers", |g| {
        let v: Vec<u32> = ds().into_iter().take(2).map(move |d| d.0 + zero(&g)).collect();
        print!("| {v:?} | ");
    });
    case("collected by next", |g| {
        let v: Vec<u64> = ds()
            .into_iter()
            .take(2)
            .map(move |d| u64::from(d.0 + zero(&g)))
            .filter(|_| true)
            .collect();
        print!("| {v:?} | ");
    });
    case("a for loop", |g| {
        for n in ds().into_iter().take(2).map(move |d| d.0 + zero(&g)) {
            print!("{n} ");
        }
    });
    case("find", |g| {
        let found = ds().into_iter().take(2).map(move |d| d.0 + zero(&g)).find(|&n| n == 9);
        print!("| {found:?} | ");
    });
    case("taken after", |g| {
        let n: u32 = ds().into_iter().map(move |d| d.0 + zero(&g)).take(2).sum();
        print!("| {n} | ");
    });
    case("take_while", |g| {
        let n = ds().into_iter().take_while(move |d| d.0 < 3 + zero(&g)).count();
        print!("| {n} | ");
    });
    case("map_while", |g| {
        let n: u32 = ds()
            .into_iter()
            .map_while(move |d| (d.0 < 3).then_some(d.0 + zero(&g)))
            .sum();
        print!("| {n} | ");
    });
    case("skip_while", |g| {
        let n = ds().into_iter().take(3).skip_while(move |d| d.0 < 2 + zero(&g)).count();
        print!("| {n} | ");
    });
    case("step_by", |g| {
        let n = ds().into_iter().take(3).map(move |d| d.0 + zero(&g)).step_by(2).count();
        print!("| {n} | ");
    });
    case("zip", |g| {
        let n = ds().into_iter().take(2).map(move |d| d.0 + zero(&g)).zip(0..5).count();
        print!("| {n} | ");
    });
    case("by_ref", |g| {
        let mut it = ds().into_iter().take(3).map(move |d| d.0 + zero(&g));
        let n: u32 = it.by_ref().sum();
        print!("| {n} | ");
    });
}
