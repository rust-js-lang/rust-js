// A step that panics as `last` or a fold takes what it's given, the crate's
// own iterator's `next`, a `flat_map`'s closure or a JS iterator helper's: the step's own dropped,
// then the item `last` kept or the accumulator a fold holds, then what's
// left, as Rust's frames unwind (#426). Of one that holds nothing to drop,
// making each item, the item `last` kept is dropped as its `next` panics.
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Panicky {
    items: std::vec::IntoIter<D>,
    at: u32,
}

impl Iterator for Panicky {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        let next = self.items.next();
        if next.as_ref().is_some_and(|d| d.0 == self.at) {
            panic!("boom");
        }
        next
    }
}

// Its item bound by `?`, a variable with a drop flag (#425).
struct Questioned {
    items: std::vec::IntoIter<D>,
    at: u32,
}

impl Iterator for Questioned {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        let d = self.items.next()?;
        if d.0 == self.at {
            panic!("boom");
        }
        Some(d)
    }
}

// It holds nothing with a destructor: each item it gives is made.
struct Made {
    at: u32,
    upto: u32,
}

impl Iterator for Made {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        if self.at == self.upto {
            panic!("boom");
        }
        self.at += 1;
        Some(D(self.at))
    }
}

fn ds(r: std::ops::Range<u32>) -> Vec<D> {
    r.map(D).collect()
}

fn run(name: &str, f: impl FnOnce()) {
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {}", r.is_err());
}

fn main() {
    run("own next last", || {
        let x = Panicky {
            items: ds(170..174).into_iter(),
            at: 172,
        }
        .last();
        println!("{}", x.is_some());
    });
    run("own next by ? last", || {
        let x = Questioned {
            items: ds(190..194).into_iter(),
            at: 192,
        }
        .last();
        println!("{}", x.is_some());
    });
    run("made next last", || {
        let x = Made { at: 210, upto: 213 }.last();
        println!("{}", x.is_some());
    });
    run("own next fold", || {
        let x = Panicky {
            items: ds(180..184).into_iter(),
            at: 182,
        }
        .fold(Vec::new(), |mut acc, d| {
            acc.push(d);
            acc
        });
        println!("{}", x.len());
    });
    run("flat_map last", || {
        let x = ds(60..64)
            .into_iter()
            .flat_map(|d| {
                if d.0 == 62 {
                    panic!("62");
                }
                vec![D(d.0 + 100)]
            })
            .last();
        println!("{}", x.is_some());
    });
    run("flatten fold", || {
        let x = vec![ds(70..72), ds(72..74)]
            .into_iter()
            .flatten()
            .inspect(|d| {
                if d.0 == 72 {
                    panic!("72");
                }
            })
            .fold(Vec::new(), |mut acc, d| {
                acc.push(d);
                acc
            });
        println!("{}", x.len());
    });
    run("chars fold", || {
        let x = "abcd"
            .chars()
            .map(|c| {
                if c == 'c' {
                    panic!("c");
                }
                c as u32
            })
            .fold(Vec::new(), |mut acc, n| {
                acc.push(D(n));
                acc
            });
        println!("{}", x.len());
    });
}
