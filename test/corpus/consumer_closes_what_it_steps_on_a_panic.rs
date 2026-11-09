// What a std consumer steps of an iterator whose step panics, of items with
// a destructor, is closed as the panic unwinds, what's left of it dropped,
// as Rust drops the iterator it took: a heap's `extend`, a `splice`'s
// replacement, generic code's `collect` into a std collection, a `Debug`
// builder's `entries` and `is_sorted_by`, each of which the runtime stepped
// by JS's own `for..of` or `Array.from`, which close nothing where the
// step itself throws (#1141).
use std::collections::{BTreeSet, BinaryHeap};
use std::panic::{catch_unwind, AssertUnwindSafe};
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3)]
}
fn key(d: D) -> u32 {
    if d.0 == 2 {
        panic!("at 2");
    }
    d.0
}

fn collected<C: FromIterator<u32>>(v: Vec<D>) -> C {
    v.into_iter().map(key).collect()
}

struct Listed;

impl std::fmt::Debug for Listed {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_list().entries(ds().into_iter().map(key)).finish()
    }
}

fn main() {
    let mut heap = BinaryHeap::from([1u32]);
    let r = catch_unwind(AssertUnwindSafe(|| heap.extend(vec![D(9), D(2), D(3)].into_iter().map(key))));
    println!("heap {} {:?}", r.is_err(), heap.into_sorted_vec());
    let mut v = vec![0u32, 9];
    let r = catch_unwind(AssertUnwindSafe(|| v.splice(0..1, ds().into_iter().map(key)).count()));
    println!("splice {}", r.is_err());
    let mut w = vec![0u32, 9];
    w.splice(1..1, BTreeSet::from([4u32, 3]));
    println!("{w:?}");
    let r = catch_unwind(|| collected::<BTreeSet<u32>>(ds()).len());
    println!("collect {}", r.is_err());
    let r = catch_unwind(|| format!("{:?}", Listed).len());
    println!("debug {}", r.is_err());
    let r = catch_unwind(|| ds().into_iter().map(key).is_sorted_by(|a, b| a <= b));
    println!("sorted {}", r.is_err());
    println!("{:?}", collected::<Vec<u32>>(vec![D(5), D(6)]));
    println!("{}", vec![D(7), D(8)].into_iter().map(|d| d.0).is_sorted_by(|a, b| a <= b));
}
