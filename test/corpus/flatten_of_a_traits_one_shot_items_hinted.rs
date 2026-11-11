//@ library-differs
// A flatten of a trait's associated iterator hints as std's does by
// whether it's `OneShot`, what the trait's dictionaries carry of it: an
// `option::IntoIter`'s, one item at most, bounds the hint, another's
// doesn't. Carried as every fact but a layout's is (#1341, #1348).
trait Source {
    type Iter: Iterator<Item = u32>;
    fn items(n: u32) -> Self::Iter;
}

struct Once;
impl Source for Once {
    type Iter = std::option::IntoIter<u32>;
    fn items(n: u32) -> Self::Iter {
        Some(n).into_iter()
    }
}

struct Many;
impl Source for Many {
    type Iter = std::vec::IntoIter<u32>;
    fn items(n: u32) -> Self::Iter {
        vec![n, n].into_iter()
    }
}

fn hint<S: Source>() -> (usize, Option<usize>) {
    (1..4).flat_map(S::items).size_hint()
}

fn main() {
    println!("{:?} {:?}", hint::<Once>(), hint::<Many>());
}
