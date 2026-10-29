// A closure that panics part-way through an owning iterator: each item it
// held, what the consumer kept or collected and what's left dropped once,
// in std's order: the closure's own first, then, of a `Vec` std collects
// in place, what it collected before what's left, else what's left first
// (#423).
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    d.0.is_multiple_of(2)
}

fn small(d: &D) -> bool {
    d.0 < 3
}

fn run(name: &str, f: impl FnOnce()) {
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {}", r.is_err());
}

fn main() {
    run("find", || {
        let x = ds(0..4).into_iter().find(|d| boom(d, 2));
        println!("{}", x.is_some());
    });
    run("position", || {
        let x = ds(10..14).into_iter().position(|d| boom(&d, 12));
        println!("{x:?}");
    });
    run("any", || {
        let x = ds(20..24).into_iter().any(|d| boom(&d, 22));
        println!("{x}");
    });
    run("all", || {
        let x = ds(30..34).into_iter().all(|d| !boom(&d, 32) || true);
        println!("{x}");
    });
    run("take_while", || {
        let x: Vec<D> = ds(40..44).into_iter().take_while(|d| !boom(d, 42) || true).collect();
        println!("{}", x.len());
    });
    run("skip_while", || {
        let x: Vec<D> = ds(50..54).into_iter().skip_while(|d| boom(d, 51) || true).collect();
        println!("{}", x.len());
    });
    run("partition", || {
        let (a, b): (Vec<D>, Vec<D>) = ds(60..64).into_iter().partition(|d| boom(d, 62));
        println!("{} {}", a.len(), b.len());
    });
    run("inspect", || {
        let x: Vec<D> = ds(80..84)
            .into_iter()
            .inspect(|d| {
                boom(d, 82);
            })
            .collect();
        println!("{}", x.len());
    });
    run("max_by_key first", || {
        let x = ds(70..73).into_iter().max_by_key(|d| {
            boom(d, 70);
            d.0
        });
        println!("{}", x.is_some());
    });
    run("last at its end", || {
        let x = ds(160..163).into_iter().take_while(|d| boom(d, 162) || true).last();
        println!("{}", x.is_some());
    });
    run("filter by a fn", || {
        let x: Vec<D> = ds(0..5).into_iter().filter(small).collect();
        println!("{}", x.len());
    });
    run("filter", || {
        let x: Vec<D> = ds(90..94).into_iter().filter(|d| boom(d, 92) || true).collect();
        println!("{}", x.len());
    });
    run("deque", || {
        let x: VecDeque<D> = ds(100..104)
            .into_iter()
            .inspect(|d| {
                boom(d, 102);
            })
            .collect();
        println!("{}", x.len());
    });
    run("btree", || {
        let x: BTreeMap<u32, D> = ds(110..114)
            .into_iter()
            .map(|d| {
                boom(&d, 112);
                (d.0, d)
            })
            .collect();
        println!("{}", x.len());
    });
    run("hashmap", || {
        let x: HashMap<u32, D> = ds(120..123)
            .into_iter()
            .map(|d| {
                boom(&d, 121);
                (d.0, d)
            })
            .collect();
        println!("{}", x.len());
    });
    run("result", || {
        let x: Result<Vec<D>, ()> = ds(130..134)
            .into_iter()
            .map(|d| {
                boom(&d, 132);
                Ok(d)
            })
            .collect();
        println!("{}", x.is_ok());
    });
    run("option", || {
        let x: Option<Vec<D>> = ds(140..144)
            .into_iter()
            .map(|d| {
                boom(&d, 142);
                Some(d)
            })
            .collect();
        println!("{}", x.is_some());
    });
    run("count", || {
        let x = ds(150..154).into_iter().filter(|d| boom(d, 152) || true).count();
        println!("{x}");
    });
    run("sum", || {
        let x: u32 = ds(170..174)
            .into_iter()
            .map(|d| {
                boom(&d, 172);
                d.0
            })
            .sum();
        println!("{x}");
    });
    run("for", || {
        for d in ds(180..184) {
            boom(&d, 182);
        }
    });
    run("chain", || {
        let x: Vec<D> = ds(190..192)
            .into_iter()
            .chain(ds(193..195))
            .inspect(|d| {
                boom(d, 193);
            })
            .collect();
        println!("{}", x.len());
    });
    run("rev", || {
        let x: Vec<D> = ds(200..204)
            .into_iter()
            .rev()
            .inspect(|d| {
                boom(d, 202);
            })
            .collect();
        println!("{}", x.len());
    });
    run("enumerate", || {
        let x: Vec<(usize, D)> = ds(210..214)
            .into_iter()
            .enumerate()
            .map(|(i, d)| {
                boom(&d, 212);
                (i, d)
            })
            .collect();
        println!("{}", x.len());
    });
    run("scan", || {
        let x: Vec<u32> = ds(220..224)
            .into_iter()
            .scan(0, |s, d| {
                boom(&d, 222);
                *s += d.0;
                Some(*s)
            })
            .collect();
        println!("{}", x.len());
    });
    run("map_while", || {
        let x: Vec<D> = ds(230..234)
            .into_iter()
            .map_while(|d| {
                boom(&d, 232);
                Some(d)
            })
            .collect();
        println!("{}", x.len());
    });
    run("min_by", || {
        let x = ds(240..244).into_iter().min_by(|a, b| {
            boom(a, 242);
            boom(b, 242);
            a.0.cmp(&b.0)
        });
        println!("{}", x.is_some());
    });
    run("zip pairs", || {
        let x: Vec<(D, D)> = ds(0..4)
            .into_iter()
            .map(|d| {
                boom(&d, 2);
                D(d.0 + 100)
            })
            .zip(ds(10..14))
            .collect();
        println!("{}", x.len());
    });
    run("btreeset", || {
        let x: BTreeSet<D> = ds(20..24)
            .into_iter()
            .inspect(|d| {
                boom(d, 22);
            })
            .collect();
        println!("{}", x.len());
    });
    run("heap", || {
        let x: BinaryHeap<D> = ds(30..34)
            .into_iter()
            .inspect(|d| {
                boom(d, 32);
            })
            .collect();
        println!("{}", x.len());
    });
    run("list", || {
        let x: LinkedList<D> = ds(40..44)
            .into_iter()
            .inspect(|d| {
                boom(d, 42);
            })
            .collect();
        println!("{}", x.len());
    });
    run("hashset one", || {
        let x: HashSet<D> = ds(50..53)
            .into_iter()
            .inspect(|d| {
                boom(d, 51);
            })
            .collect();
        println!("{}", x.len());
    });
    run("result bigger", || {
        let x: Result<Vec<(u8, D)>, ()> = ds(60..64)
            .into_iter()
            .map(|d| {
                boom(&d, 62);
                Ok((1, d))
            })
            .collect();
        println!("{}", x.is_ok());
    });
    run("filter_map bigger", || {
        let x: Vec<(D, u64)> = ds(70..74)
            .into_iter()
            .filter_map(|d| {
                boom(&d, 72);
                if d.0 == 71 { None } else { Some((d, 1)) }
            })
            .collect();
        println!("{}", x.len());
    });
    run("extend", || {
        let mut v = vec![D(80)];
        v.extend(ds(81..85).into_iter().inspect(|d| {
            boom(d, 83);
        }));
        println!("{}", v.len());
    });
    run("string", || {
        let s: String = ds(90..94)
            .into_iter()
            .map(|d| {
                boom(&d, 92);
                'x'
            })
            .collect();
        println!("{s}");
    });
    run("sum", || {
        let x: u32 = ds(110..114)
            .into_iter()
            .map(|d| {
                boom(&d, 112);
                d.0
            })
            .sum();
        println!("{x}");
    });
    run("for_each", || {
        ds(130..134).into_iter().for_each(|d| {
            boom(&d, 132);
            println!("saw {}", d.0);
        });
    });
    run("rev collect", || {
        let x: Vec<D> = ds(140..144)
            .into_iter()
            .rev()
            .inspect(|d| {
                boom(d, 142);
            })
            .collect();
        println!("{}", x.len());
    });
    run("flat_map panics", || {
        let out: Vec<u32> = ds(0..3)
            .into_iter()
            .flat_map(|d| if d.0 == 1 { panic!("1") } else { vec![d.0, d.0] })
            .collect();
        println!("{out:?}");
    });
    run("zip of maps", || {
        let out: Vec<(u32, u32)> = ds(10..13)
            .into_iter()
            .map(|d| if d.0 == 11 { panic!("11") } else { d.0 })
            .zip(ds(20..23).into_iter().map(|d| d.0))
            .collect();
        println!("{out:?}");
    });
    run("string of chars", || {
        let out: String = ds(30..33)
            .into_iter()
            .map(|d| {
                if d.0 == 32 {
                    panic!("32")
                } else {
                    char::from(b'a' + (d.0 % 26) as u8)
                }
            })
            .collect();
        println!("{out}");
    });
    run("set via filter", || {
        let out: HashSet<u32> = ds(40..43)
            .into_iter()
            .filter(|d| if d.0 == 41 { panic!("41") } else { true })
            .map(|d| d.0)
            .collect();
        println!("{}", out.len());
    });
    run("result of owned", || {
        let out: Result<Vec<D>, String> = ds(50..53)
            .into_iter()
            .map(|d| if d.0 == 52 { panic!("52") } else { Ok(d) })
            .collect();
        println!("{}", out.is_ok());
    });
}
