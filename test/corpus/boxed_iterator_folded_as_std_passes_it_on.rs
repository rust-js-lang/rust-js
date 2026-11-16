// A `Box` of std's compiled iterator, a `take()` of a `Vec`'s, folded: std's
// `Box<I>` passes on only `next`, `size_hint`, `nth`, `last`, `next_back`,
// `nth_back` and `len`, so its `rfold`, and a `rev()` of it folded, are the
// traits' defaults over `next_back`, which drop what `Take` skips 4 then 5,
// never `Take`'s own `rfold`, 5 then 4 (#1995). Its fold, count, last,
// nth_back and a `map()` of it summed, alike. Each row in a function of its
// own, its items printed as they drop.

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ds() -> Vec<D> {
    (1..=5).map(D).collect()
}

fn boxed_rfold() {
    let mut a = ds().into_iter().take(3);
    println!("a next {:?}", a.next().map(|x| x.0));
    let b = Box::new(a);
    println!("b rfold {}", b.rfold(0, |acc, x| acc * 10 + x.0));
}

fn boxed_rev_fold() {
    let mut a = ds().into_iter().take(3);
    println!("a next {:?}", a.next().map(|x| x.0));
    let b = Box::new(a);
    println!("b rev fold {}", b.rev().fold(0, |acc, x| acc * 10 + x.0));
}

fn boxed_fold_count_last() {
    let mut a = ds().into_iter().take(3);
    a.next();
    println!("fold {}", Box::new(a).fold(0, |acc, x| acc * 10 + x.0));
    let mut a = ds().into_iter().take(3);
    a.next_back();
    println!("count {}", Box::new(a).count());
    let mut a = ds().into_iter().take(3);
    a.next_back();
    println!("last {:?}", Box::new(a).last().map(|x| x.0));
    let mut a = ds().into_iter().skip(1);
    a.next_back();
    let mut b = Box::new(a);
    println!("nth_back {:?}", b.nth_back(1).map(|x| x.0));
    println!("next {:?}", b.next().map(|x| x.0));
}

fn boxed_map_sum() {
    let a = ds().into_iter().skip(1).take(3);
    let b = Box::new(a);
    let total: u32 = b.map(|x| x.0).sum();
    println!("sum {total}");
}

fn main() {
    println!("boxed rfold");
    boxed_rfold();
    println!("boxed rev fold");
    boxed_rev_fold();
    println!("boxed fold, count, last");
    boxed_fold_count_last();
    println!("boxed map sum");
    boxed_map_sum();
}
