// Generic code's `collect()` into each std collection, by its dictionary's
// `fromIter` entry, of an iterator whose step panics part-way or whose hint
// overflows the collection's capacity before any step: what's left of it is
// dropped as the panic unwinds, as std drops the iterator its collect holds,
// by the collect itself, each of the runtime's closing what it steps (ADR
// 0445, amended: no second close around the entry).

use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Clone)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn key(d: D) -> u32 {
    if d.0 == 2 {
        panic!("at 2");
    }
    d.0
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3), D(4)]
}

fn collected<C: FromIterator<u32>>(v: Vec<D>) -> C {
    v.into_iter().map(key).collect()
}

fn pairs<C: FromIterator<(u32, u32)>>(v: Vec<D>) -> C {
    v.into_iter().map(|d| (key(d), 0)).collect()
}

fn chars<C: FromIterator<char>>(v: Vec<D>) -> C {
    v.into_iter().map(|d| char::from_digit(key(d), 10).unwrap()).collect()
}

fn options<C: FromIterator<Option<u32>>>(v: Vec<D>) -> C {
    v.into_iter().map(|d| Some(key(d))).collect()
}

fn results<C: FromIterator<Result<u32, ()>>>(v: Vec<D>) -> C {
    v.into_iter().map(|d| Ok(key(d))).collect()
}

fn repeated<C: FromIterator<u32>>(n: usize) -> C {
    std::iter::repeat_n(D(7), n).map(|d| d.0).collect()
}

fn run(name: &str, f: impl FnOnce()) {
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {}", r.is_err());
}

fn main() {
    run("vec", || drop(collected::<Vec<u32>>(ds())));
    run("deque", || drop(collected::<VecDeque<u32>>(ds())));
    run("list", || drop(collected::<LinkedList<u32>>(ds())));
    run("heap", || drop(collected::<BinaryHeap<u32>>(ds())));
    run("btreeset", || drop(collected::<BTreeSet<u32>>(ds())));
    run("hashset", || drop(collected::<HashSet<u32>>(ds())));
    run("boxed", || drop(collected::<Box<[u32]>>(ds())));
    run("btreemap", || drop(pairs::<BTreeMap<u32, u32>>(ds())));
    run("hashmap", || drop(pairs::<HashMap<u32, u32>>(ds())));
    run("string", || drop(chars::<String>(ds())));
    run("option", || drop(options::<Option<Vec<u32>>>(ds())));
    run("vec huge", || drop(repeated::<Vec<u32>>(usize::MAX)));
    run("deque huge", || drop(repeated::<VecDeque<u32>>(usize::MAX)));
    run("heap huge", || drop(repeated::<BinaryHeap<u32>>(usize::MAX)));
    run("hashset huge", || drop(repeated::<HashSet<u32>>(usize::MAX)));
    run("boxed huge", || drop(repeated::<Box<[u32]>>(usize::MAX)));
    run("result", || drop(results::<Result<Vec<u32>, ()>>(ds())));
}
