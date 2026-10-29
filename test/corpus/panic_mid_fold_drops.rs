// A stage that panics part-way through a fold of an owning iterator, its
// accumulator one with a destructor: the stage's item dropped, then the
// accumulator its adapter's fold holds, then what's left, as std's frames
// unwind, through `map`, `filter`, `take_while`, `skip_while`, `rev`,
// `chain`, `flat_map`, `enumerate`, `take`, `skip`, `map_while`, `step_by`
// and `scan`, and of `reduce`, `last` and `max_by_key` (#423).
use std::panic::{AssertUnwindSafe, catch_unwind};
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn ds(r: std::ops::Range<u32>) -> Vec<D> {
    r.map(D).collect()
}
fn boom(d: &D, at: u32) -> bool {
    if d.0 == at {
        panic!("boom");
    }
    true
}
fn run(name: &str, f: impl FnOnce()) {
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {}", r.is_err());
}
fn push(mut acc: Vec<D>, d: D) -> Vec<D> {
    acc.push(d);
    acc
}
fn main() {
    run("fold map", || {
        let x = ds(0..4)
            .into_iter()
            .inspect(|d| {
                boom(d, 2);
            })
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("fold filter", || {
        let x = ds(10..14).into_iter().filter(|d| boom(d, 12)).fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("fold take_while", || {
        let x = ds(20..24)
            .into_iter()
            .take_while(|d| boom(d, 22))
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("fold skip_while first", || {
        let x = ds(30..34)
            .into_iter()
            .skip_while(|d| !boom(d, 30))
            .fold(vec![D(39)], push);
        println!("{}", x.len());
    });
    run("fold rev map", || {
        let x = ds(40..44)
            .into_iter()
            .rev()
            .inspect(|d| {
                boom(d, 41);
            })
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("fold chain", || {
        let x = ds(50..52)
            .into_iter()
            .chain(ds(52..54))
            .inspect(|d| {
                boom(d, 52);
            })
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("fold flat_map", || {
        let x = ds(60..62)
            .into_iter()
            .flat_map(|d| {
                boom(&d, 61);
                vec![D(d.0 + 100)]
            })
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("fold enumerate", || {
        let x = ds(70..74)
            .into_iter()
            .enumerate()
            .map(|(i, d)| {
                println!("at {i}");
                boom(&d, 72);
                d
            })
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("fold take", || {
        let x = ds(80..84)
            .into_iter()
            .inspect(|d| {
                boom(d, 81);
            })
            .take(3)
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("fold skip", || {
        let x = ds(90..94)
            .into_iter()
            .skip(1)
            .inspect(|d| {
                boom(d, 92);
            })
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("reduce", || {
        let x = ds(110..114)
            .into_iter()
            .inspect(|d| {
                boom(d, 112);
            })
            .reduce(|a, b| if a.0 > b.0 { a } else { b });
        println!("{}", x.is_some());
    });
    run("fold map_while", || {
        let x = ds(120..124)
            .into_iter()
            .map_while(|d| {
                boom(&d, 122);
                Some(d)
            })
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("last", || {
        let x = ds(140..144).into_iter().take_while(|d| boom(d, 142)).last();
        println!("{}", x.is_some());
    });
    run("max_by_key", || {
        let x = ds(150..154).into_iter().max_by_key(|d| {
            boom(d, 152);
            d.0
        });
        println!("{}", x.is_some());
    });
    run("for_each", || {
        let mut v = Vec::new();
        ds(130..134).into_iter().for_each(|d| {
            boom(&d, 132);
            v.push(d);
        });
        println!("{}", v.len());
    });
    run("step_by", || {
        let x = ds(0..8)
            .into_iter()
            .inspect(|d| {
                boom(d, 4);
            })
            .step_by(2)
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("rev filter", || {
        let x = ds(10..14)
            .into_iter()
            .filter(|d| boom(d, 12))
            .rev()
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("rev enumerate", || {
        let x = ds(20..24)
            .into_iter()
            .enumerate()
            .rev()
            .map(|(i, d)| {
                println!("at {i}");
                boom(&d, 22);
                d
            })
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("skip inspect", || {
        let x = ds(30..35)
            .into_iter()
            .skip(1)
            .inspect(|d| {
                boom(d, 33);
            })
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("map then rev", || {
        let x = ds(50..54)
            .into_iter()
            .map(|d| {
                boom(&d, 52);
                D(d.0 + 100)
            })
            .rev()
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("rev take_while", || {
        let x = ds(60..64)
            .into_iter()
            .rev()
            .take_while(|d| boom(d, 61))
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("scan acc", || {
        let x = (90..94)
            .scan(0, |_, n| {
                if n == 92 {
                    panic!("92")
                }
                Some(n)
            })
            .fold(Vec::new(), |mut acc, n| {
                acc.push(D(n));
                acc
            });
        println!("{}", x.len());
    });
    run("skip_while later", || {
        let x = ds(80..84)
            .into_iter()
            .skip_while(|d| d.0 < 81)
            .inspect(|d| {
                boom(d, 82);
            })
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("chain of a stage", || {
        let x = ds(0..3)
            .into_iter()
            .inspect(|d| {
                boom(d, 1);
            })
            .chain(ds(3..5))
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("skip of a stage", || {
        let x = ds(10..14)
            .into_iter()
            .inspect(|d| {
                boom(d, 12);
            })
            .skip(1)
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("flat_map of numbers", || {
        let x = (40..43)
            .flat_map(|n| {
                if n == 41 {
                    panic!("41")
                }
                vec![n]
            })
            .fold(Vec::new(), |mut acc, n| {
                acc.push(D(n));
                acc
            });
        println!("{}", x.len());
    });
    run("by_ref stage", || {
        let mut it = ds(50..54).into_iter();
        let x = it
            .by_ref()
            .inspect(|d| {
                boom(d, 52);
            })
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("skip of numbers", || {
        let x = (60..64)
            .inspect(|&n| {
                if n == 62 {
                    panic!("62")
                }
            })
            .skip(1)
            .fold(Vec::new(), |mut acc, n| {
                acc.push(D(n));
                acc
            });
        println!("{}", x.len());
    });
    run("take of numbers", || {
        let x = (70..74)
            .inspect(|&n| {
                if n == 72 {
                    panic!("72")
                }
            })
            .take(3)
            .fold(Vec::new(), |mut acc, n| {
                acc.push(D(n));
                acc
            });
        println!("{}", x.len());
    });
    run("filter of numbers", || {
        let x = (80..84)
            .filter(|&n| {
                if n == 82 {
                    panic!("82")
                }
                true
            })
            .fold(Vec::new(), |mut acc, n| {
                acc.push(D(n));
                acc
            });
        println!("{}", x.len());
    });
    run("stage then skip_while", || {
        let x = ds(0..4)
            .into_iter()
            .inspect(|d| {
                boom(d, 2);
            })
            .skip_while(|d| d.0 < 1)
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("numbers map then rev", || {
        let x = (10..14)
            .inspect(|&n| {
                if n == 12 {
                    panic!("12")
                }
            })
            .rev()
            .fold(Vec::new(), |mut acc, n| {
                acc.push(D(n));
                acc
            });
        println!("{}", x.len());
    });
    run("filter_map then rev", || {
        let x = ds(30..34)
            .into_iter()
            .filter_map(|d| {
                boom(&d, 32);
                if d.0 == 31 { None } else { Some(d) }
            })
            .rev()
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("map knows its back", || {
        let x = ds(40..44)
            .into_iter()
            .map(|d| {
                boom(&d, 42);
                D(d.0 + 100)
            })
            .rev()
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("inspect then rev", || {
        let x = ds(20..24)
            .into_iter()
            .inspect(|d| {
                boom(d, 22);
            })
            .rev()
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
    run("rev step_by then a stage", || {
        let x = ds(160..168)
            .into_iter()
            .rev()
            .step_by(2)
            .inspect(|d| {
                boom(d, 163);
            })
            .fold(Vec::new(), push);
        println!("{}", x.len());
    });
}
