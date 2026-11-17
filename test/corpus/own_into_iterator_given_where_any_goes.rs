// The crate's own `IntoIterator`, of `&Board` and of `Board`, given where
// std's code or generic code takes any `IntoIterator` (#1940): its own
// `into_iter` runs where Rust calls it, as a chain is made, as an
// extend starts, as generic code calls `into_iter`, as a splice has taken
// out its range, never at the call nor at a first step. A `splice` whose
// range is out of bounds panics before its `into_iter` runs; an
// `into_iter` that panics in a `splice` drops what it took out and puts the
// tail back; one that panics in an `extend` unwinds out of it. A zip's are
// `own_into_iterator_zipped`'s.
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind};

struct Board {
    cells: Vec<u32>,
    bad: bool,
}

impl<'a> IntoIterator for &'a Board {
    type Item = u32;
    type IntoIter = std::iter::Copied<std::slice::Iter<'a, u32>>;
    fn into_iter(self) -> Self::IntoIter {
        println!("into_iter &Board");
        assert!(!self.bad, "a bad board");
        self.cells.iter().copied()
    }
}

impl IntoIterator for Board {
    type Item = u32;
    type IntoIter = std::vec::IntoIter<u32>;
    fn into_iter(self) -> Self::IntoIter {
        println!("into_iter Board");
        self.cells.into_iter()
    }
}

fn board() -> Board {
    Board {
        cells: vec![1, 2, 3],
        bad: false,
    }
}

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop Loud {}", self.0);
    }
}

struct Louds(Vec<u32>);

impl IntoIterator for Louds {
    type Item = Loud;
    type IntoIter = std::iter::Map<std::vec::IntoIter<u32>, fn(u32) -> Loud>;
    fn into_iter(self) -> Self::IntoIter {
        println!("into_iter Louds");
        self.0.into_iter().map(Loud as fn(u32) -> Loud)
    }
}

fn panics() {
    let bad = Board {
        cells: vec![5, 6],
        bad: true,
    };
    let good = board();
    let mut w = vec![1u32, 2, 3, 4];
    let r = catch_unwind(AssertUnwindSafe(|| {
        let gone: Vec<u32> = w.splice(1..9, &good).collect();
        println!("{gone:?}");
    }));
    println!("{} {w:?}", r.is_err());
    let mut w = vec![1u32, 2, 3, 4];
    let r = catch_unwind(AssertUnwindSafe(|| {
        let gone: Vec<u32> = w.splice(1..3, &bad).collect();
        println!("{gone:?}");
    }));
    println!("{} {w:?}", r.is_err());
    let mut l = vec![Loud(1), Loud(2), Loud(3), Loud(4)];
    let r = catch_unwind(AssertUnwindSafe(|| {
        let gone: Vec<Loud> = l.splice(1..3, Louds(vec![7])).collect();
        println!("{}", gone.len());
    }));
    println!("{} {}", r.is_err(), l.len());
    let r = catch_unwind(AssertUnwindSafe(|| {
        let mut v = vec![0u32];
        v.extend(&bad);
        println!("{v:?}");
    }));
    println!("{}", r.is_err());
}

fn total(items: impl IntoIterator<Item = u32>) -> u32 {
    println!("total starts");
    let it = items.into_iter();
    println!("total has it");
    it.sum()
}

fn main() {
    let b = board();
    let c = [9u32].iter().copied().chain(&b);
    println!("chain made");
    println!("{:?}", c.collect::<Vec<_>>());
    let mut v = vec![10];
    v.extend(&b);
    println!("{v:?}");
    println!("{}", total(&b));
    println!("{}", total(board()));
    let mut w = vec![1u32, 2, 3, 4];
    let gone: Vec<u32> = w.splice(1..3, &b).collect();
    println!("{w:?} {gone:?}");
    let mut s: HashSet<u32> = HashSet::new();
    s.extend(&b);
    println!("{}", s.len());
    let mut t: BTreeSet<u32> = BTreeSet::new();
    t.extend(board());
    println!("{t:?}");
    let mut d: VecDeque<u32> = VecDeque::new();
    d.extend(&b);
    println!("{d:?}");
    let mut m: HashMap<u32, u32> = HashMap::new();
    m.extend((&b).into_iter().map(|x| (x, x)));
    let bm: BTreeMap<u32, u32> = (&b).into_iter().map(|x| (x, x * 2)).collect();
    println!("{} {bm:?}", m.len());
    let mut x = vec![0u32];
    x.extend(board());
    println!("{x:?}");
    panics();
}
