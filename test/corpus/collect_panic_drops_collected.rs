// A collect into a `Vec` that a panic stops partway drops what it
// collected, as std's `from_iter` drops its `Vec` as it unwinds (#800): a
// range's `map`, which is no array's `map` where its items have a
// destructor, collected and held; a generic `Vec::from_iter(iter)` of a
// type parameter's `IntoIterator`, which steps its `IntoIter` and asks its
// caller whether std collects it in place; and a `Vec`'s own iterator,
// which std collects in place.

struct D(i32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn made(n: i32) -> D {
    if n == 3 {
        panic!("at {n}");
    }
    D(n)
}

fn concrete() -> Vec<D> {
    (1..=3).map(made).collect()
}

fn held() -> usize {
    let v: Vec<D> = (1..=4).map(made).collect();
    v.len()
}

fn gathered<I: IntoIterator<Item = D>>(iter: I) -> Vec<D> {
    Vec::from_iter(iter)
}

fn of_vec(v: Vec<i32>) -> Vec<D> {
    v.into_iter().map(made).collect()
}

fn panicked<T>(f: impl FnOnce() -> T + std::panic::UnwindSafe) -> bool {
    std::panic::catch_unwind(f).is_err()
}

fn main() {
    println!("{}", panicked(concrete));
    println!("{}", panicked(held));
    println!("{}", panicked(|| gathered((1..=5).map(made))));
    println!("{}", panicked(|| gathered(vec![1, 2, 3, 4].into_iter().map(made))));
    println!("{}", panicked(|| of_vec(vec![1, 2, 3, 4])));
    let fine = gathered((1..=2).map(made));
    println!("{}", fine.len());
}
